//! Shared drawing utilities — color interpolation, rect insets, text wrapping.

use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};

use super::state::TRANSITION_FRAMES;
use crate::game;

/// Interpolate a base color toward the shadow background as `dim_level` rises
/// from 0 to TRANSITION_FRAMES.
pub(super) fn transition_color(base: Color, dim_level: u8) -> Color {
    let t = dim_level as f32 / TRANSITION_FRAMES as f32;
    let t = t.clamp(0.0, 1.0);
    let (br, bg, bb) = match base {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => return base,
    };
    let (sr, sg, sb) = (30.0, 25.0, 20.0);
    let r = (br * (1.0 - t) + sr * t) as u8;
    let g = (bg * (1.0 - t) + sg * t) as u8;
    let b = (bb * (1.0 - t) + sb * t) as u8;
    Color::Rgb(r, g, b)
}

/// Fade a color toward the shadow background as `progress` goes 0.0 → 1.0.
pub(super) fn fade_color(base: Color, progress: f32) -> Color {
    let t = progress.clamp(0.0, 1.0);
    let (br, bg, bb) = match base {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => return base,
    };
    let (sr, sg, sb) = (30.0, 25.0, 20.0);
    let r = (br * (1.0 - t) + sr * t) as u8;
    let g = (bg * (1.0 - t) + sg * t) as u8;
    let b = (bb * (1.0 - t) + sb * t) as u8;
    Color::Rgb(r, g, b)
}

/// Shrink a rect by `dx` columns and `dy` rows on each side.
pub(super) fn inset_rect(r: Rect, dx: u16, dy: u16) -> Rect {
    let dx = dx.min(r.width / 2);
    let dy = dy.min(r.height / 2);
    Rect {
        x: r.x + dx,
        y: r.y + dy,
        width: r.width.saturating_sub(dx * 2),
        height: r.height.saturating_sub(dy * 2),
    }
}

/// Apply `transition_color` to every span's fg/bg in a list of lines.
pub(super) fn dim_lines(lines: &[Line<'static>], dim_level: u8) -> Vec<Line<'static>> {
    if dim_level == 0 {
        return lines.to_vec();
    }
    lines
        .iter()
        .map(|l| {
            let spans: Vec<Span<'static>> = l
                .spans
                .iter()
                .map(|s| {
                    let mut style = s.style;
                    if let Some(fg) = style.fg {
                        style.fg = Some(transition_color(fg, dim_level));
                    }
                    if let Some(bg) = style.bg {
                        style.bg = Some(transition_color(bg, dim_level));
                    }
                    Span::styled(s.content.clone(), style)
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

/// Word-wrap text to a max line width. Returns a vec of lines.
pub(super) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            out.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Format a duration in milliseconds as `m:ss` or `0:ss`.
pub(super) fn format_duration(ms: game::DurationMs) -> String {
    let total_s = ms / 1000;
    let m = total_s / 60;
    let s = total_s % 60;
    if m > 0 {
        format!("{}:{:02}", m, s)
    } else {
        format!("0:{:02}", s)
    }
}

/// Format a quantity as a compact label: `999`, `12k`, `5M`.
pub(super) fn format_quantity(qty: u32) -> String {
    if qty >= 1_000_000 {
        format!("{}M", qty / 1_000_000)
    } else if qty >= 10_000 {
        format!("{}k", qty / 1_000)
    } else {
        format!("{}", qty)
    }
}
