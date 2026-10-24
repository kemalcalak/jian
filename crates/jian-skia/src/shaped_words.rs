//! Kerned, shaped advances for text that [`FontResolver`] measures and
//! paints.
//!
//! `Font::measure_str` and `Canvas::draw_str` map codepoints straight to
//! glyphs and sum their default advances, so pair kerning and contextual
//! substitutions never happen. Browsers, Figma and pen.dev shape with
//! HarfBuzz, and a paragraph that fits their width can wrap one line
//! earlier here. This module shapes with HarfBuzz too (Skia's `SkShaper`),
//! word by word in the manner of Blink's word-shape cache: text splits
//! into words, single spaces and single CJK characters, each token is
//! shaped once per (typeface, size, skew) and cached with its glyphs and
//! positions. A run's width is the sum of its tokens, and paint places the
//! tokens at the same running offsets, so measure and paint cannot drift
//! apart. Kerning across a space is not applied (Blink skips it as well).
//!
//! Callers keep complex scripts (bidi, joining) on the Paragraph path; the
//! tokens here are always laid out left to right.
//!
//! [`FontResolver`]: crate::font_resolve::FontResolver

use std::collections::HashMap;
use std::rc::Rc;

use skia_safe::shaper::run_handler::{Buffer, RunHandler, RunInfo};
use skia_safe::{Font, GlyphId, Point, Shaper, TextBlob, TextBlobBuilder};

/// Distinct tokens kept per face before that face's table is dropped. A
/// full refill costs one shaping pass over the visible text, far cheaper
/// than LRU bookkeeping on every lookup.
const MAX_WORDS_PER_FACE: usize = 16_384;

/// Distinct shaped runs (paint blobs) kept before the table is dropped.
const MAX_RUNS: usize = 8_192;

/// Wrap budget handed to the shaper. The shaper never wraps, but Skia
/// requires a finite width.
const SHAPE_WIDTH: f32 = 1.0e6;

/// One shaped token: advance plus the glyphs at their offsets from the
/// token's origin.
struct ShapedWord {
    width: f32,
    glyphs: Box<[GlyphId]>,
    positions: Box<[Point]>,
}

/// The face a token is shaped in. Synthetic bold widens the advance and is
/// applied by the caller; it does not change the shaped glyphs.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FaceKey {
    typeface: u32,
    size_bits: u32,
    skew: bool,
}

/// One typeface run of a shaped text run, ready to paint.
pub struct ShapedSegment {
    /// `None` for a segment made only of glyphs the face cannot draw.
    pub blob: Option<TextBlob>,
    /// The resolver's synthetic-bold branch for this segment's face.
    pub synthetic_bold: bool,
    /// Advance of the segment, synthetic-bold allowance included.
    pub width: f32,
}

/// A whole text run shaped for paint: one blob per typeface segment, with
/// glyph positions relative to the run's baseline origin.
pub struct ShapedRun {
    pub segments: Vec<ShapedSegment>,
    pub width: f32,
}

/// What a paint blob is cached under. Borrowed for lookups so the hot path
/// does not allocate.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RunStyle<'a> {
    pub family: &'a str,
    pub size_bits: u32,
    pub weight: u16,
    pub italic: bool,
}

struct RunEntry {
    family: String,
    size_bits: u32,
    weight: u16,
    italic: bool,
    text: String,
    run: Rc<ShapedRun>,
}

#[derive(Default)]
pub(crate) struct ShapedWords {
    shaper: Option<Shaper>,
    words: HashMap<FaceKey, HashMap<String, Rc<ShapedWord>>>,
    runs: HashMap<u64, Vec<RunEntry>>,
    run_count: usize,
}

impl ShapedWords {
    /// Forget every shaped token and run (the font set changed).
    pub(crate) fn clear(&mut self) {
        self.words.clear();
        self.runs.clear();
        self.run_count = 0;
    }

    /// Kerned advance of `text` in `font`, summed per token.
    pub(crate) fn width(&mut self, font: &Font, text: &str) -> f32 {
        let face = face_key(font);
        let mut width = 0.0;
        for token in tokens(text) {
            width += self.word(face, font, token).width;
        }
        width
    }

    /// Append the glyphs of `text` in `font`, starting `x` from the run
    /// origin, scaling advances by `stretch`. Returns the advance.
    pub(crate) fn push_glyphs(
        &mut self,
        font: &Font,
        text: &str,
        x: f32,
        stretch: f32,
        glyphs: &mut Vec<GlyphId>,
        positions: &mut Vec<Point>,
    ) -> f32 {
        let face = face_key(font);
        let mut advance = 0.0;
        for token in tokens(text) {
            let word = self.word(face, font, token);
            for (glyph, pos) in word.glyphs.iter().zip(word.positions.iter()) {
                glyphs.push(*glyph);
                positions.push(Point::new(x + advance + pos.x * stretch, pos.y));
            }
            advance += word.width * stretch;
        }
        advance
    }

    pub(crate) fn cached_run(&self, style: RunStyle<'_>, text: &str) -> Option<Rc<ShapedRun>> {
        let bucket = self.runs.get(&run_hash(style, text))?;
        bucket
            .iter()
            .find(|entry| {
                entry.size_bits == style.size_bits
                    && entry.weight == style.weight
                    && entry.italic == style.italic
                    && entry.family == style.family
                    && entry.text == text
            })
            .map(|entry| Rc::clone(&entry.run))
    }

    pub(crate) fn store_run(&mut self, style: RunStyle<'_>, text: &str, run: Rc<ShapedRun>) {
        if self.run_count >= MAX_RUNS {
            self.runs.clear();
            self.run_count = 0;
        }
        self.runs
            .entry(run_hash(style, text))
            .or_default()
            .push(RunEntry {
                family: style.family.to_string(),
                size_bits: style.size_bits,
                weight: style.weight,
                italic: style.italic,
                text: text.to_string(),
                run,
            });
        self.run_count += 1;
    }

    fn word(&mut self, face: FaceKey, font: &Font, token: &str) -> Rc<ShapedWord> {
        if let Some(word) = self.words.get(&face).and_then(|words| words.get(token)) {
            return Rc::clone(word);
        }
        let shaper = self.shaper.get_or_insert_with(new_shaper);
        let word = Rc::new(shape_token(shaper, font, token));
        let words = self.words.entry(face).or_default();
        if words.len() >= MAX_WORDS_PER_FACE {
            words.clear();
        }
        words.insert(token.to_string(), Rc::clone(&word));
        word
    }
}

/// Build the paint blob for one segment's glyphs.
pub(crate) fn segment_blob(
    font: &Font,
    glyphs: &[GlyphId],
    positions: &[Point],
) -> Option<TextBlob> {
    if glyphs.is_empty() {
        return None;
    }
    let mut builder = TextBlobBuilder::new();
    let (out_glyphs, out_positions) = builder.alloc_run_pos(font, glyphs.len(), None);
    out_glyphs.copy_from_slice(glyphs);
    out_positions.copy_from_slice(positions);
    builder.make()
}

fn new_shaper() -> Shaper {
    // No fallback manager: the resolver already split the run into
    // segments whose typeface covers every character.
    skia_safe::shapers::hb::shape_dont_wrap_or_reorder(None)
        .unwrap_or_else(skia_safe::shapers::primitive::primitive_text)
}

fn face_key(font: &Font) -> FaceKey {
    FaceKey {
        typeface: font.typeface().unique_id(),
        size_bits: font.size().to_bits(),
        skew: font.skew_x() != 0.0,
    }
}

fn run_hash(style: RunStyle<'_>, text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    style.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

fn shape_token(shaper: &Shaper, font: &Font, token: &str) -> ShapedWord {
    let mut collector = Collector::default();
    shaper.shape(token, font, true, SHAPE_WIDTH, &mut collector);
    if collector.glyphs.is_empty() && !token.is_empty() {
        // Shaper unavailable or refused the text: fall back to the plain
        // advances so text still measures and paints.
        return unshaped_token(font, token);
    }
    ShapedWord {
        width: collector.x,
        glyphs: collector.glyphs.into_boxed_slice(),
        positions: collector.positions.into_boxed_slice(),
    }
}

fn unshaped_token(font: &Font, token: &str) -> ShapedWord {
    let glyphs = font.str_to_glyphs_vec(token);
    let mut widths = vec![0.0; glyphs.len()];
    font.get_widths(&glyphs, &mut widths);
    let mut x = 0.0;
    let positions = widths
        .iter()
        .map(|w| {
            let p = Point::new(x, 0.0);
            x += w;
            p
        })
        .collect::<Vec<_>>();
    ShapedWord {
        width: x,
        glyphs: glyphs.into_boxed_slice(),
        positions: positions.into_boxed_slice(),
    }
}

/// Collects the shaper's glyphs and positions, one buffer per run, laid
/// end to end on the baseline.
#[derive(Default)]
struct Collector {
    glyphs: Vec<GlyphId>,
    positions: Vec<Point>,
    x: f32,
}

impl RunHandler for Collector {
    fn begin_line(&mut self) {}

    fn run_info(&mut self, _: &RunInfo) {}

    fn commit_run_info(&mut self) {}

    fn run_buffer(&mut self, info: &RunInfo) -> Buffer<'_> {
        let start = self.glyphs.len();
        self.glyphs.resize(start + info.glyph_count, 0);
        self.positions
            .resize(start + info.glyph_count, Point::default());
        Buffer::new(
            &mut self.glyphs[start..],
            &mut self.positions[start..],
            Point::new(self.x, 0.0),
        )
    }

    fn commit_run_buffer(&mut self, info: &RunInfo) {
        self.x += info.advance.x;
    }

    fn commit_line(&mut self) {}
}

/// Split `text` into shaping tokens: maximal runs of non-space, non-CJK
/// characters, and every space or CJK character on its own. CJK text has
/// no spaces, so shaping it per character keeps the cache bounded by the
/// character set rather than by every prefix the line wrap probes.
pub(crate) fn tokens(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        let mut chars = rest.char_indices();
        let (_, first) = chars.next()?;
        let end = if first == ' ' || is_cjk(first) {
            first.len_utf8()
        } else {
            chars
                .find(|(_, c)| *c == ' ' || is_cjk(*c))
                .map(|(i, _)| i)
                .unwrap_or(rest.len())
        };
        let (token, tail) = rest.split_at(end);
        rest = tail;
        Some(token)
    })
}

/// Same predicate as the line wrap's per-character break set.
fn is_cjk(ch: char) -> bool {
    let c = ch as u32;
    (0x4e00..=0x9fff).contains(&c)
        || (0x3400..=0x4dbf).contains(&c)
        || (0x3040..=0x30ff).contains(&c)
        || (0xac00..=0xd7af).contains(&c)
        || (0x3000..=0x303f).contains(&c)
        || (0xff00..=0xffef).contains(&c)
}

#[cfg(test)]
mod tests {
    use super::tokens;

    #[test]
    fn tokens_split_words_spaces_and_cjk() {
        let got: Vec<&str> = tokens("Hello  world, 设计a").collect();
        assert_eq!(got, vec!["Hello", " ", " ", "world,", " ", "设", "计", "a"]);
        assert_eq!(tokens("").count(), 0);
    }

    #[test]
    fn paint_runs_advance_exactly_as_measured() {
        let resolver = crate::FontResolver::new(skia_safe::FontMgr::default());
        for text in ["AVATAR WAVE", "Hello,  world", "设计 tools"] {
            let measured = resolver.measure_text(text, 18.0, None, 400, false);
            let run = resolver.shape_run(text, 18.0, None, 400, false);
            assert!(
                (run.width - measured).abs() < 1e-3,
                "{text}: {} vs {measured}",
                run.width
            );
            let segments: f32 = run.segments.iter().map(|s| s.width).sum();
            assert!((segments - measured).abs() < 1e-3, "{text}");
        }
    }

    #[test]
    fn repeated_runs_come_from_the_cache() {
        let resolver = crate::FontResolver::new(skia_safe::FontMgr::default());
        let a = resolver.shape_run("cached line", 14.0, None, 400, false);
        let b = resolver.shape_run("cached line", 14.0, None, 400, false);
        assert!(std::rc::Rc::ptr_eq(&a, &b));
    }
}
