/// Image-to-halfblock converter.
///
/// Converts PNG or SVG image data into ratatui Lines using Unicode halfblock
/// characters (▀ ▄ █) with 24-bit RGB colors. This is the same technique
/// used by the `lntrn` system info display.
///
/// Two vertical pixels are packed into one terminal character:
///   - Top pixel → background color
///   - Bottom pixel → foreground color + ▄ character
///
/// This gives us 2x vertical resolution compared to regular text.
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

/// A pixel with RGBA values
#[derive(Clone, Copy)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

/// Render a PNG image (from bytes) into halfblock Lines.
/// `width` is the desired width in terminal columns.
pub fn png_to_halfblock(png_bytes: &[u8], width: u16) -> Vec<Line<'static>> {
    let img = image::load_from_memory(png_bytes).expect("Failed to decode PNG");
    let rgba = img.to_rgba8();
    image_to_halfblock(&rgba, width)
}

/// Render an SVG (from bytes) into halfblock Lines.
/// `width` is the desired width in terminal columns.
pub fn svg_to_halfblock(svg_bytes: &[u8], width: u16) -> Vec<Line<'static>> {
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &Default::default())
        .expect("Failed to parse SVG");

    let svg_size = tree.size();

    // Height in terminal rows = width * (svg_height / svg_width) / 2
    // (divided by 2 because each terminal row = 2 pixel rows)
    let pixel_w = width as u32;
    let pixel_h = (pixel_w as f32 * svg_size.height() / svg_size.width()) as u32;
    // Round up to even number for halfblock pairing
    let pixel_h = if pixel_h % 2 != 0 { pixel_h + 1 } else { pixel_h };

    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixel_w, pixel_h)
        .expect("Failed to create pixmap");

    let scale_x = pixel_w as f32 / svg_size.width();
    let scale_y = pixel_h as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    // Center the image
    let offset_x = (pixel_w as f32 - svg_size.width() * scale) / 2.0;
    let offset_y = (pixel_h as f32 - svg_size.height() * scale) / 2.0;

    let transform = resvg::tiny_skia::Transform::from_translate(offset_x, offset_y)
        .post_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap_to_halfblock(&pixmap)
}

/// Convert an image::RgbaImage into halfblock Lines
fn image_to_halfblock(img: &image::RgbaImage, width: u16) -> Vec<Line<'static>> {
    let (orig_w, orig_h) = img.dimensions();
    let new_w = width as u32;
    let new_h = (new_w as f32 * orig_h as f32 / orig_w as f32) as u32;
    // Round up to even
    let new_h = if new_h % 2 != 0 { new_h + 1 } else { new_h };

    let resized = image::imageops::resize(
        img,
        new_w,
        new_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut lines = Vec::new();
    for y in (0..new_h).step_by(2) {
        let mut spans = Vec::new();
        for x in 0..new_w {
            let top = resized.get_pixel(x, y);
            let bot = if y + 1 < new_h {
                *resized.get_pixel(x, y + 1)
            } else {
                image::Rgba([0, 0, 0, 0])
            };

            let top_px = Pixel { r: top[0], g: top[1], b: top[2], a: top[3] };
            let bot_px = Pixel { r: bot[0], g: bot[1], b: bot[2], a: bot[3] };

            let span = pixels_to_span(top_px, bot_px);
            spans.push(span);
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Convert a tiny_skia Pixmap into halfblock Lines
fn pixmap_to_halfblock(pixmap: &resvg::tiny_skia::Pixmap) -> Vec<Line<'static>> {
    let w = pixmap.width();
    let h = pixmap.height();
    let data = pixmap.data();

    let px = |x: u32, y: u32| -> Pixel {
        let i = (y * w + x) as usize * 4;
        // tiny_skia uses premultiplied alpha, need to unpremultiply
        let a = data[i + 3];
        if a == 0 {
            return Pixel { r: 0, g: 0, b: 0, a: 0 };
        }
        let r = (data[i] as u16 * 255 / a as u16) as u8;
        let g = (data[i + 1] as u16 * 255 / a as u16) as u8;
        let b = (data[i + 2] as u16 * 255 / a as u16) as u8;
        Pixel { r, g, b, a }
    };

    let mut lines = Vec::new();
    for y in (0..h).step_by(2) {
        let mut spans = Vec::new();
        for x in 0..w {
            let top = px(x, y);
            let bot = if y + 1 < h { px(x, y + 1) } else { Pixel { r: 0, g: 0, b: 0, a: 0 } };
            spans.push(pixels_to_span(top, bot));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Convert a top/bottom pixel pair into a single styled Span.
/// Uses the same algorithm as the lntrn system info display.
fn pixels_to_span(top: Pixel, bot: Pixel) -> Span<'static> {
    let ta = top.a > 80;
    let ba = bot.a > 80;

    match (ta, ba) {
        (true, true) => {
            // Both pixels visible: bg = top color, fg = bottom color, char = ▄
            Span::styled(
                "▄",
                Style::default()
                    .bg(Color::Rgb(top.r, top.g, top.b))
                    .fg(Color::Rgb(bot.r, bot.g, bot.b)),
            )
        }
        (true, false) => {
            // Only top pixel: fg = top color, char = ▀
            Span::styled(
                "▀",
                Style::default().fg(Color::Rgb(top.r, top.g, top.b)),
            )
        }
        (false, true) => {
            // Only bottom pixel: fg = bottom color, char = ▄
            Span::styled(
                "▄",
                Style::default().fg(Color::Rgb(bot.r, bot.g, bot.b)),
            )
        }
        (false, false) => {
            // Both transparent
            Span::styled(" ", Style::default())
        }
    }
}
