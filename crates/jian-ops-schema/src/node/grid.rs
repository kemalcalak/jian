//! Grid auto layout: track lists, child placement and per-child alignment.
//!
//! A `layout: "grid"` container lays its children out on `columns` ×
//! `rows` tracks, CSS grid style. Tracks are written the way CSS writes
//! them, so a document reads like the stylesheet it came from:
//!
//! - a number is a fixed track in px (`120`);
//! - `"1fr"` shares the free space, `"auto"` sizes to the content;
//! - `"minmax(120px, 1fr)"` clamps a track between two sizes.
//!
//! `columns` / `rows` also take a plain count (`3` = three `1fr`
//! tracks) or one CSS string (`"repeat(3, 1fr)"`, `"200px 1fr auto"`),
//! and always serialize back to the count or the track array.
//!
//! Children pick a cell with `gridColumn` / `gridRow` (`2`, `"span 2"`,
//! `"1 / span 3"`, `"1 / 3"`); without them they flow into the next free
//! cell, row by row.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// One end of a `minmax()` track, or a plain track size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackSize {
    /// Fixed length in px.
    Px(f64),
    /// Share of the free space (`fr`).
    Fr(f64),
    /// Sized to the content.
    Auto,
}

impl TrackSize {
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        if s == "auto" || s == "min-content" || s == "max-content" {
            return Some(Self::Auto);
        }
        if let Some(n) = s.strip_suffix("fr") {
            return finite_non_negative(n).map(Self::Fr);
        }
        let n = s.strip_suffix("px").unwrap_or(&s);
        finite_non_negative(n).map(Self::Px)
    }

    fn css(self) -> String {
        match self {
            Self::Px(v) => format!("{}px", fmt_num(v)),
            Self::Fr(v) => format!("{}fr", fmt_num(v)),
            Self::Auto => "auto".to_owned(),
        }
    }
}

/// One grid track.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridTrack {
    /// A track of one size: fixed px, `fr` or `auto`.
    Size(TrackSize),
    /// `minmax(min, max)`. `min` is never `fr` (CSS forbids it).
    MinMax { min: TrackSize, max: TrackSize },
}

impl GridTrack {
    pub const FR1: Self = Self::Size(TrackSize::Fr(1.0));

    /// Parse one CSS track (`120`, `120px`, `1fr`, `auto`,
    /// `minmax(120px, 1fr)`). `None` for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim();
        let lower = t.to_ascii_lowercase();
        if let Some(inner) = lower
            .strip_prefix("minmax(")
            .and_then(|r| r.strip_suffix(')'))
        {
            let (a, b) = inner.split_once(',')?;
            let min = TrackSize::parse(a)?;
            let max = TrackSize::parse(b)?;
            if matches!(min, TrackSize::Fr(_)) {
                return None;
            }
            return Some(Self::MinMax { min, max });
        }
        TrackSize::parse(t).map(Self::Size)
    }

    /// The track as CSS text.
    pub fn css(&self) -> String {
        match self {
            Self::Size(s) => s.css(),
            Self::MinMax { min, max } => format!("minmax({}, {})", min.css(), max.css()),
        }
    }

    /// True when the track takes a share of the free space.
    pub fn is_flexible(&self) -> bool {
        matches!(
            self,
            Self::Size(TrackSize::Fr(_))
                | Self::MinMax {
                    max: TrackSize::Fr(_),
                    ..
                }
        )
    }
}

impl fmt::Display for GridTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size(TrackSize::Px(v)) => f.write_str(&fmt_num(*v)),
            other => f.write_str(&other.css()),
        }
    }
}

impl Serialize for GridTrack {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Size(TrackSize::Px(v)) if v.fract() == 0.0 && v.abs() < 1e15 => {
                s.serialize_i64(*v as i64)
            }
            Self::Size(TrackSize::Px(v)) => s.serialize_f64(*v),
            other => s.serialize_str(&other.css()),
        }
    }
}

impl<'de> Deserialize<'de> for GridTrack {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match NumberOrText::deserialize(d)? {
            NumberOrText::Number(n) if n.is_finite() && n >= 0.0 => {
                Ok(Self::Size(TrackSize::Px(n)))
            }
            NumberOrText::Number(n) => Err(serde::de::Error::custom(format!(
                "grid track must be a non-negative number, got {n}"
            ))),
            NumberOrText::Text(t) => Self::parse(&t).ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "unknown grid track `{t}` (use a px number, `1fr`, `auto` or `minmax(a, b)`)"
                ))
            }),
        }
    }
}

/// `columns` / `rows` of a grid container.
#[derive(Debug, Clone, PartialEq)]
pub enum GridTemplate {
    /// `n` equal `1fr` tracks.
    Count(u32),
    /// An explicit track list.
    Tracks(Vec<GridTrack>),
}

impl GridTemplate {
    /// Upper bound on tracks per axis; a larger count is clamped.
    pub const MAX_TRACKS: u32 = 256;

    /// The explicit tracks, a count expanded to `1fr` each.
    pub fn tracks(&self) -> Vec<GridTrack> {
        match self {
            Self::Count(n) => vec![GridTrack::FR1; (*n).min(Self::MAX_TRACKS) as usize],
            Self::Tracks(t) => t.clone(),
        }
    }

    /// Number of explicit tracks.
    pub fn len(&self) -> usize {
        match self {
            Self::Count(n) => (*n).min(Self::MAX_TRACKS) as usize,
            Self::Tracks(t) => t.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Parse a CSS track list: whitespace-separated tracks with
    /// `repeat(n, …)` groups, or a bare count (`"3"`).
    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim();
        if t.is_empty() {
            return None;
        }
        if let Ok(n) = t.parse::<u32>() {
            return Some(Self::Count(n.min(Self::MAX_TRACKS)));
        }
        let mut out = Vec::new();
        for token in split_top_level(t) {
            let lower = token.to_ascii_lowercase();
            if let Some(inner) = lower
                .strip_prefix("repeat(")
                .and_then(|r| r.strip_suffix(')'))
            {
                let (count, body) = inner.split_once(',')?;
                let count: u32 = count.trim().parse().ok()?;
                let group: Vec<GridTrack> = split_top_level(body)
                    .iter()
                    .map(|p| GridTrack::parse(p))
                    .collect::<Option<_>>()?;
                if group.is_empty() {
                    return None;
                }
                for _ in 0..count.min(Self::MAX_TRACKS) {
                    out.extend(group.iter().copied());
                }
            } else {
                out.push(GridTrack::parse(&token)?);
            }
            if out.len() > Self::MAX_TRACKS as usize {
                out.truncate(Self::MAX_TRACKS as usize);
                break;
            }
        }
        Some(Self::Tracks(out))
    }

    /// The template as CSS (`grid-template-columns`).
    pub fn css(&self) -> String {
        match self {
            Self::Count(n) => format!("repeat({}, 1fr)", n),
            Self::Tracks(t) => t.iter().map(GridTrack::css).collect::<Vec<_>>().join(" "),
        }
    }

    /// The template as the short text the property panel edits
    /// (`1fr 1fr 120`).
    pub fn short_text(&self) -> String {
        match self {
            Self::Count(n) => n.to_string(),
            Self::Tracks(t) => t
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

impl Serialize for GridTemplate {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Count(n) => s.serialize_u32(*n),
            Self::Tracks(t) => t.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for GridTemplate {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Number(f64),
            Tracks(Vec<GridTrack>),
            Text(String),
        }
        match Raw::deserialize(d)? {
            Raw::Number(n) if n.is_finite() && n >= 0.0 && n.fract() == 0.0 => {
                Ok(Self::Count((n as u32).min(Self::MAX_TRACKS)))
            }
            Raw::Number(n) => Err(serde::de::Error::custom(format!(
                "grid track count must be a whole number, got {n}"
            ))),
            Raw::Tracks(t) => Ok(Self::Tracks(t)),
            Raw::Text(t) => Self::parse(&t)
                .ok_or_else(|| serde::de::Error::custom(format!("unknown grid track list `{t}`"))),
        }
    }
}

/// Where a grid child sits on one axis: a 1-based start line (or auto
/// placement) and how many tracks it spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPlacement {
    /// 1-based start line; `None` = the next free cell.
    pub start: Option<u16>,
    /// Tracks covered, at least 1.
    pub span: u16,
}

impl Default for GridPlacement {
    fn default() -> Self {
        Self {
            start: None,
            span: 1,
        }
    }
}

impl GridPlacement {
    /// Upper bound on a span.
    pub const MAX_SPAN: u16 = 256;

    /// Parse `2`, `"span 2"`, `"1 / span 3"`, `"1 / 3"` or `"auto"`.
    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim().to_ascii_lowercase();
        let (a, b) = match t.split_once('/') {
            Some((a, b)) => (a.trim().to_owned(), Some(b.trim().to_owned())),
            None => (t.clone(), None),
        };
        let mut out = Self::default();
        if let Some(span) = a.strip_prefix("span") {
            out.span = parse_span(span)?;
            return b.is_none().then_some(out);
        }
        if a != "auto" {
            out.start = Some(parse_line(&a)?);
        }
        match b.as_deref() {
            None | Some("auto") => {}
            Some(end) => {
                if let Some(span) = end.strip_prefix("span") {
                    out.span = parse_span(span)?;
                } else {
                    let end = parse_line(end)?;
                    let start = out.start?;
                    if end <= start {
                        return None;
                    }
                    out.span = (end - start).min(Self::MAX_SPAN);
                }
            }
        }
        Some(out)
    }

    /// The placement as CSS (`grid-column`).
    pub fn css(&self) -> String {
        match (self.start, self.span) {
            (Some(s), 1) => s.to_string(),
            (Some(s), n) => format!("{s} / span {n}"),
            (None, 1) => "auto".to_owned(),
            (None, n) => format!("span {n}"),
        }
    }
}

fn parse_line(s: &str) -> Option<u16> {
    let n: u16 = s.trim().parse().ok()?;
    (n >= 1).then_some(n)
}

fn parse_span(s: &str) -> Option<u16> {
    let n: u16 = s.trim().parse().ok()?;
    (n >= 1).then_some(n.min(GridPlacement::MAX_SPAN))
}

impl Serialize for GridPlacement {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match (self.start, self.span) {
            (Some(start), 1) => s.serialize_u16(start),
            _ => s.serialize_str(&self.css()),
        }
    }
}

impl<'de> Deserialize<'de> for GridPlacement {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let parsed = match NumberOrText::deserialize(d)? {
            NumberOrText::Number(n) if n.fract() == 0.0 && (1.0..=65535.0).contains(&n) => {
                Some(Self {
                    start: Some(n as u16),
                    span: 1,
                })
            }
            NumberOrText::Number(_) => None,
            NumberOrText::Text(t) => Self::parse(&t),
        };
        parsed.ok_or_else(|| {
            serde::de::Error::custom(
                "grid placement must be a line number, `span n`, `a / span n` or `a / b`",
            )
        })
    }
}

/// A grid child's alignment inside its cell (`justifySelf` / `alignSelf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[cfg_attr(feature = "export-ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "export-ts", ts(export, export_to = "ops.ts"))]
#[serde(rename_all = "snake_case")]
pub enum SelfAlign {
    Start,
    Center,
    End,
    /// Fill the cell on that axis.
    Stretch,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NumberOrText {
    Number(f64),
    Text(String),
}

/// JSON Schema shape of the hand-serialized grid types.
#[derive(schemars::JsonSchema)]
#[allow(dead_code)]
#[serde(untagged)]
enum NumberOrString {
    Number(f64),
    Text(String),
}

#[derive(schemars::JsonSchema)]
#[allow(dead_code)]
#[serde(untagged)]
enum TemplateShape {
    Count(u32),
    Tracks(Vec<NumberOrString>),
    Text(String),
}

macro_rules! schema_as {
    ($ty:ty, $name:literal, $shape:ty) => {
        impl schemars::JsonSchema for $ty {
            fn schema_name() -> String {
                $name.to_owned()
            }
            fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                <$shape>::json_schema(gen)
            }
        }
    };
}

schema_as!(GridTrack, "GridTrack", NumberOrString);
schema_as!(GridTemplate, "GridTemplate", TemplateShape);
schema_as!(GridPlacement, "GridPlacement", NumberOrString);

fn finite_non_negative(s: &str) -> Option<f64> {
    let v: f64 = s.trim().parse().ok()?;
    (v.is_finite() && v >= 0.0).then_some(v)
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Split on whitespace outside parentheses (`minmax(0, 1fr) 2fr`).
fn split_top_level(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            c if c.is_whitespace() && depth <= 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
