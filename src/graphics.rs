//! Raster graphics: the masthead, the cover and the front-page illustration.
//!
//! Everything that must look identical regardless of the reader's font
//! settings is rendered to an image. SVG templates come from the theme and are
//! rasterized with resvg using only the theme's own fonts, so output does not
//! depend on fonts installed on the host.

use std::io::Cursor;
use std::sync::Arc;

use base64::Engine;
use image::{GrayImage, ImageFormat, Luma};
use resvg::{tiny_skia, usvg};
use serde::Serialize;

use crate::theme::Theme;

#[derive(Debug, thiserror::Error)]
pub enum GraphicsError {
    #[error("front_image.data is not valid base64: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("front_image could not be decoded: {0}")]
    Decode(#[from] image::ImageError),
    #[error("front_image is larger than 12 MiB")]
    TooLarge,
    #[error("SVG template produced invalid SVG: {0}")]
    Svg(#[from] usvg::Error),
    #[error("template error: {0}")]
    Template(#[from] minijinja::Error),
    #[error("could not allocate a {0}x{1} canvas")]
    Canvas(u32, u32),
    #[error("PNG encoding failed: {0}")]
    Png(String),
}

/// Width of the masthead image in pixels. Kindle screens are 1072–1860 px
/// wide; 1400 px keeps the Fraktur crisp without bloating the file.
pub const MASTHEAD_WIDTH: u32 = 1400;
/// Cover size, 3:4 like current Kindle screens.
pub const COVER_SIZE: (u32, u32) = (1264, 1680);
/// Longest edge for the front-page illustration.
pub const FRONT_IMAGE_MAX_WIDTH: u32 = 1200;

// ---------------------------------------------------------------------------
// Text measurement

/// Advance-width based text measurement with the theme's own fonts. Kerning is
/// ignored, which is accurate enough for fitting and line breaking.
pub struct Metrics<'a> {
    face: ttf_parser::Face<'a>,
}

impl<'a> Metrics<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let face = ttf_parser::Face::parse(data, 0).expect("theme fonts are valid TrueType");
        Self { face }
    }

    pub fn width(&self, text: &str, size: f32) -> f32 {
        let upem = f32::from(self.face.units_per_em());
        let units: f32 = text
            .chars()
            .map(|c| {
                self.face
                    .glyph_index(c)
                    .and_then(|g| self.face.glyph_hor_advance(g))
                    .map_or(upem * 0.5, f32::from)
            })
            .sum();
        units * size / upem
    }

    /// Largest size ≤ `max_size` at which `text` fits into `max_width`.
    pub fn fit(&self, text: &str, max_width: f32, max_size: f32) -> f32 {
        let w = self.width(text, max_size);
        if w <= max_width {
            max_size
        } else {
            max_size * max_width / w
        }
    }

    /// Greedy line breaking at spaces. A single word longer than the line is
    /// kept on its own line.
    pub fn wrap(&self, text: &str, size: f32, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut cur = String::new();
        for word in text.split_whitespace() {
            let candidate = if cur.is_empty() {
                word.to_string()
            } else {
                format!("{cur} {word}")
            };
            if self.width(&candidate, size) <= max_width || cur.is_empty() {
                cur = candidate;
            } else {
                lines.push(std::mem::replace(&mut cur, word.to_string()));
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        lines
    }
}

// ---------------------------------------------------------------------------
// SVG rasterization

fn svg_options(theme: &Theme) -> usvg::Options<'static> {
    let mut db = usvg::fontdb::Database::new();
    for f in theme.fonts {
        db.load_font_data(f.data.to_vec());
    }
    usvg::Options {
        fontdb: Arc::new(db),
        ..usvg::Options::default()
    }
}

/// Renders SVG source to an opaque grayscale image.
fn rasterize(theme: &Theme, svg: &str) -> Result<GrayImage, GraphicsError> {
    let tree = usvg::Tree::from_str(svg, &svg_options(theme))?;
    let size = tree.size().to_int_size();
    let (w, h) = (size.width(), size.height());
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or(GraphicsError::Canvas(w, h))?;
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    // The background is opaque, so premultiplied RGBA equals straight RGBA.
    let gray = GrayImage::from_fn(w, h, |x, y| {
        let p = pixmap.pixel(x, y).expect("in bounds");
        let l =
            0.299 * f32::from(p.red()) + 0.587 * f32::from(p.green()) + 0.114 * f32::from(p.blue());
        Luma([l.round() as u8])
    });
    Ok(gray)
}

fn encode_png(img: &GrayImage) -> Result<Vec<u8>, GraphicsError> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)
        .map_err(|e| GraphicsError::Png(e.to_string()))?;
    Ok(out.into_inner())
}

fn encode_jpeg(img: &GrayImage, quality: u8) -> Result<Vec<u8>, GraphicsError> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality).encode_image(img)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Front image

/// A processed illustration: grayscale, contrast-stretched, resized JPEG.
pub struct Illustration {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Decodes the transported image and prepares it for e-ink.
pub fn prepare_illustration(data: &str) -> Result<Illustration, GraphicsError> {
    let b64 = match data.split_once(";base64,") {
        Some((prefix, rest)) if prefix.starts_with("data:") => rest,
        _ => data,
    };
    let b64: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
    if bytes.len() > crate::model::MAX_IMAGE_BYTES {
        return Err(GraphicsError::TooLarge);
    }
    let mut img = image::load_from_memory(&bytes)?.to_luma8();
    if img.width() > FRONT_IMAGE_MAX_WIDTH {
        let h = (u64::from(img.height()) * u64::from(FRONT_IMAGE_MAX_WIDTH)
            / u64::from(img.width())) as u32;
        img = image::imageops::resize(
            &img,
            FRONT_IMAGE_MAX_WIDTH,
            h.max(1),
            image::imageops::FilterType::Lanczos3,
        );
    }
    stretch_levels(&mut img);
    Ok(Illustration {
        width: img.width(),
        height: img.height(),
        jpeg: encode_jpeg(&img, 82)?,
    })
}

/// Auto-levels with 1 % clipping on both ends. Old scans are often yellowed
/// and low in contrast, which looks muddy on e-ink.
fn stretch_levels(img: &mut GrayImage) {
    let mut hist = [0u64; 256];
    for p in img.pixels() {
        hist[p[0] as usize] += 1;
    }
    let total: u64 = hist.iter().sum();
    let clip = total / 100;
    let find = |iter: &mut dyn Iterator<Item = usize>| {
        let mut acc = 0;
        for i in iter {
            acc += hist[i];
            if acc > clip {
                return i;
            }
        }
        0
    };
    let lo = find(&mut (0..256));
    let hi = find(&mut (0..256).rev());
    if hi <= lo + 10 {
        return; // flat image, leave it alone
    }
    let scale = 255.0 / (hi - lo) as f32;
    for p in img.pixels_mut() {
        let v = (f32::from(p[0]) - lo as f32) * scale;
        p[0] = v.clamp(0.0, 255.0).round() as u8;
    }
}

// ---------------------------------------------------------------------------
// Masthead

#[derive(Serialize)]
struct MastheadCtx<'a> {
    w: u32,
    h: u32,
    size: f32,
    baseline: f32,
    title: &'a str,
}

/// Renders the newspaper name as a PNG in the theme's display face.
pub fn masthead(
    theme: &Theme,
    env: &minijinja::Environment,
    title: &str,
) -> Result<Vec<u8>, GraphicsError> {
    let metrics = Metrics::new(theme.font_data(theme.masthead_font));
    let w = MASTHEAD_WIDTH;
    let size = metrics.fit(title, w as f32 * 0.94, 200.0).round();
    let h = (size * 1.18).round() as u32;
    let ctx = MastheadCtx {
        w,
        h,
        size,
        baseline: (size * 0.9).round(),
        title,
    };
    let svg = env
        .get_template("masthead.svg")?
        .render(minijinja::value::Serde(&ctx))?;
    encode_png(&rasterize(theme, &svg)?)
}

// ---------------------------------------------------------------------------
// Cover

#[derive(Serialize)]
struct TextRun {
    text: String,
    size: f32,
    baseline: f32,
}

#[derive(Serialize)]
struct Dateline {
    text: String,
    size: f32,
    baseline: f32,
    rule_top: f32,
    rule_bottom: f32,
}

#[derive(Serialize)]
struct ImageSlot {
    href: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Serialize)]
struct Line {
    text: String,
    y: f32,
}

#[derive(Serialize)]
struct HeadlineBlock {
    size: f32,
    rule_y: f32,
    lines: Vec<Line>,
}

#[derive(Serialize)]
struct CoverCtx {
    w: u32,
    h: u32,
    m: u32,
    name: TextRun,
    dateline: Dateline,
    image: Option<ImageSlot>,
    headlines: Vec<HeadlineBlock>,
}

pub struct CoverInput<'a> {
    pub title: &'a str,
    pub dateline: String,
    pub headlines: Vec<&'a str>,
    pub illustration: Option<&'a Illustration>,
}

/// Composes the library cover: name, date line, illustration and the top
/// headlines. Layout is computed here; the theme's SVG template draws it.
pub fn cover(
    theme: &Theme,
    env: &minijinja::Environment,
    input: &CoverInput,
) -> Result<Vec<u8>, GraphicsError> {
    let (w, h) = COVER_SIZE;
    let m = 64u32;
    let inner = (w - 2 * m) as f32;
    let mast = Metrics::new(theme.font_data(theme.masthead_font));
    let head = Metrics::new(theme.font_data(theme.headline_font));

    let name_size = mast.fit(input.title, inner, 168.0).round();
    let name_top = m as f32 + 24.0;
    let name = TextRun {
        text: input.title.to_string(),
        size: name_size,
        baseline: (name_top + name_size * 0.92).round(),
    };

    let rule_top = (name_top + name_size * 1.12).round();
    let dl_size = head.fit(&input.dateline, inner - 40.0, 30.0).round();
    let dateline = Dateline {
        text: input.dateline.clone(),
        size: dl_size,
        baseline: rule_top + 14.0 + dl_size,
        rule_top,
        rule_bottom: rule_top + dl_size + 30.0,
    };

    // Headlines from the bottom up; the illustration takes what is left.
    let with_image = input.illustration.is_some();
    let hl_size = if with_image { 46.0 } else { 64.0 };
    let line_h = hl_size * 1.18;
    let gap = hl_size * 0.9;
    let mut blocks: Vec<Vec<String>> = input
        .headlines
        .iter()
        .take(3)
        .map(|t| {
            let mut lines = head.wrap(t, hl_size, inner - 40.0);
            if lines.len() > 2 {
                lines.truncate(2);
                lines[1].push('…');
            }
            lines
        })
        .collect();
    blocks.retain(|b| !b.is_empty());
    let block_height: f32 = blocks.iter().map(|b| b.len() as f32 * line_h).sum::<f32>()
        + gap * blocks.len().saturating_sub(1) as f32;

    let bottom_limit = (h - m) as f32 - 50.0;
    let content_top = dateline.rule_bottom + 50.0;
    let (image, mut y) = match input.illustration {
        Some(ill) => {
            let max_w = inner;
            let max_h = (bottom_limit - block_height - 60.0 - content_top).max(200.0);
            let scale = (max_w / ill.width as f32).min(max_h / ill.height as f32);
            let (iw, ih) = (ill.width as f32 * scale, ill.height as f32 * scale);
            let slot = ImageSlot {
                href: format!(
                    "data:image/jpeg;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&ill.jpeg)
                ),
                x: ((w as f32 - iw) / 2.0).round(),
                y: content_top.round(),
                w: iw.round(),
                h: ih.round(),
            };
            let next = content_top + ih + 60.0;
            (Some(slot), next)
        }
        None => (
            None,
            content_top + ((bottom_limit - content_top - block_height) / 2.0).max(0.0),
        ),
    };

    let mut headlines = Vec::new();
    for lines in blocks {
        // Only drawn between blocks (the template skips it for the first).
        let rule_y = (y - gap / 2.0).round();
        let mut out = Vec::new();
        for text in lines {
            y += line_h;
            out.push(Line {
                text,
                y: (y - line_h * 0.22).round(),
            });
        }
        headlines.push(HeadlineBlock {
            size: hl_size,
            rule_y,
            lines: out,
        });
        y += gap;
    }

    let ctx = CoverCtx {
        w,
        h,
        m,
        name,
        dateline,
        image,
        headlines,
    };
    let svg = env
        .get_template("cover.svg")?
        .render(minijinja::value::Serde(&ctx))?;
    encode_jpeg(&rasterize(theme, &svg)?, 85)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> Metrics<'static> {
        let t = crate::theme::get(crate::theme::DEFAULT_THEME).unwrap();
        Metrics::new(t.font_data(t.headline_font))
    }

    #[test]
    fn wrap_respects_width() {
        let m = metrics();
        let lines = m.wrap(
            "Der Sanierungsstau wird zum Wahlkampfthema in vielen Städten",
            40.0,
            500.0,
        );
        assert!(lines.len() > 1);
        for l in &lines {
            assert!(m.width(l, 40.0) <= 500.0 || !l.contains(' '), "{l}");
        }
        assert_eq!(
            lines.join(" "),
            "Der Sanierungsstau wird zum Wahlkampfthema in vielen Städten"
        );
    }

    #[test]
    fn fit_shrinks_long_text() {
        let m = metrics();
        let s = m.fit("A very long newspaper name that cannot fit", 300.0, 100.0);
        assert!(s < 100.0);
        assert!(m.width("A very long newspaper name that cannot fit", s) <= 300.5);
    }

    #[test]
    fn illustration_is_downscaled_grayscale_jpeg() {
        let img =
            image::RgbImage::from_fn(2400, 1200, |x, _| image::Rgb([(x % 256) as u8, 120, 40]));
        let mut png = Cursor::new(Vec::new());
        img.write_to(&mut png, ImageFormat::Png).unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(png.into_inner());
        let ill = prepare_illustration(&format!("data:image/png;base64,{b64}")).unwrap();
        assert_eq!((ill.width, ill.height), (1200, 600));
        let decoded = image::load_from_memory(&ill.jpeg).unwrap();
        assert_eq!(decoded.color(), image::ColorType::L8);
    }

    #[test]
    fn invalid_image_is_an_error() {
        assert!(matches!(
            prepare_illustration("@@@"),
            Err(GraphicsError::Base64(_))
        ));
        let not_an_image = base64::engine::general_purpose::STANDARD.encode(b"hello");
        assert!(matches!(
            prepare_illustration(&not_an_image),
            Err(GraphicsError::Decode(_))
        ));
    }
}
