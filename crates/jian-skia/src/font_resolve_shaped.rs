//! Shaping half of [`FontResolver`]: kerned measure and paint runs over
//! the resolver's typeface segments (#197), and the variable-font weight
//! instancing those segments resolve through.

use std::rc::Rc;

use skia_safe::Font;

use super::{
    FontResolver, FontSegment, ResolvedTypeface, BOLD_WEIGHT_MIN, SYNTHETIC_BOLD_FACE_WEIGHT_MAX,
    SYNTHETIC_BOLD_WIDTH_FACTOR, SYNTHETIC_ITALIC_SKEW,
};
use crate::shaped_words::{RunStyle, ShapedRun, ShapedSegment};

impl FontResolver {
    /// Move a variable face onto the requested `wght`, so bold paints with
    /// the font's own bold design instead of a synthetic stroke.
    pub(super) fn at_variable_weight(
        &self,
        resolved: ResolvedTypeface,
        weight: u16,
    ) -> ResolvedTypeface {
        let key = (resolved.typeface.unique_id(), weight);
        let instance = self
            .cache
            .borrow_mut()
            .weight_instances
            .entry(key)
            .or_insert_with(|| {
                crate::variable_weight::instance_at_weight(&resolved.typeface, f32::from(weight))
            })
            .clone();
        let Some((typeface, face_weight)) = instance else {
            return resolved;
        };
        ResolvedTypeface {
            typeface,
            synthetic_bold: weight >= BOLD_WEIGHT_MIN
                && face_weight < f32::from(SYNTHETIC_BOLD_FACE_WEIGHT_MAX),
            synthetic_italic: resolved.synthetic_italic,
        }
    }

    /// Kerned advance of `text`: HarfBuzz-shaped per typeface segment and
    /// per word (see [`crate::shaped_words`]), with the synthetic-bold
    /// allowance on segments the face cannot carry. [`Self::shape_run`]
    /// paints the same advances.
    pub fn measure_text(
        &self,
        text: &str,
        font_size: f32,
        family: Option<&str>,
        weight: u16,
        italic: bool,
    ) -> f32 {
        // Serialize segmentation (resolves system typefaces via DirectWrite)
        // + shaping with all other font work. Reentrant, so this is a no-op
        // re-lock when called from within `SkiaMeasure`.
        crate::font_lock::with_font_lock(|| {
            // Segment first: it may refresh the resolver, which clears the
            // shaped-word cache borrowed below.
            let segments = self.segment_text(text, family, weight, italic);
            let mut shaped = self.shaped.borrow_mut();
            let mut width = 0.0_f32;
            for segment in segments {
                let font = segment_font(&segment, font_size);
                width += shaped.width(&font, &segment.text) * stretch(&segment);
            }
            width
        })
    }

    /// `text` shaped for paint: one positioned glyph blob per typeface
    /// segment, laid out with exactly the advances [`Self::measure_text`]
    /// reports. Cached per (text, family, size, weight, slant) so a
    /// repainted line costs one lookup.
    pub fn shape_run(
        &self,
        text: &str,
        font_size: f32,
        family: Option<&str>,
        weight: u16,
        italic: bool,
    ) -> Rc<ShapedRun> {
        crate::font_lock::with_font_lock(|| {
            self.refresh_if_stale();
            let style = RunStyle {
                family,
                size_bits: font_size.to_bits(),
                weight,
                italic,
            };
            if let Some(run) = self.shaped.borrow().cached_run(style, text) {
                return run;
            }
            // Segment before borrowing: segmentation may refresh the
            // resolver, which clears the shaped caches.
            let font_segments = self.segment_text(text, family, weight, italic);
            let mut shaped = self.shaped.borrow_mut();
            let mut segments = Vec::new();
            let mut x = 0.0_f32;
            let mut glyphs = Vec::new();
            let mut positions = Vec::new();
            for segment in font_segments {
                let font = segment_font(&segment, font_size);
                glyphs.clear();
                positions.clear();
                let width = shaped.push_glyphs(
                    &font,
                    &segment.text,
                    x,
                    stretch(&segment),
                    &mut glyphs,
                    &mut positions,
                );
                segments.push(ShapedSegment {
                    blob: crate::shaped_words::segment_blob(&font, &glyphs, &positions),
                    synthetic_bold: segment.synthetic_bold,
                    width,
                });
                x += width;
            }
            let run = Rc::new(ShapedRun { segments, width: x });
            shaped.store_run(style, text, Rc::clone(&run));
            run
        })
    }
}

/// The font a segment measures and paints with: unhinted, fractional
/// advances placed at subpixel offsets, as browsers lay text out. Hinted
/// advances round every glyph to whole pixels, which over a paragraph adds
/// up to more than the kerning takes away.
fn segment_font(segment: &FontSegment, font_size: f32) -> Font {
    let mut font = Font::new(&segment.typeface, font_size);
    font.set_subpixel(true);
    font.set_linear_metrics(true);
    if segment.synthetic_italic {
        font.set_skew_x(SYNTHETIC_ITALIC_SKEW);
    }
    font
}

/// Advance scale for a segment: synthetic bold strokes the outline, so
/// its glyphs take a little more room than the face's own advances.
fn stretch(segment: &FontSegment) -> f32 {
    if segment.synthetic_bold {
        SYNTHETIC_BOLD_WIDTH_FACTOR
    } else {
        1.0
    }
}
