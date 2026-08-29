//! The SVG document a card is built into, with one method per legacy
//! `App\Services\OpenGraph\Components` primitive.
//!
//! The legacy component tree resolved every child to an absolute position
//! before drawing (`Container::render` folded its own offset and padding
//! into the child), so the card builders here work in absolute coordinates
//! too and no nesting is needed: the document is a flat list of shapes in
//! paint order, exactly the order the legacy composited them in.

use std::fmt::Write as _;
use std::sync::OnceLock;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use super::fonts::{Face, fonts};

/// Legacy `public/img/gold.png`, the gold-bar texture.
const GOLD: &[u8] = include_bytes!("../../public/img/gold.png");

/// Legacy `public/img/diamond.png`, the diamond-bar texture.
const DIAMOND: &[u8] = include_bytes!("../../public/img/diamond.png");

/// Legacy `public/img/arrow.png`, tiled along a positive roll bar.
const ARROW: &[u8] = include_bytes!("../../public/img/arrow.png");

/// Legacy `public/img/arrow_left.png`, tiled along a negative roll bar.
const ARROW_LEFT: &[u8] = include_bytes!("../../public/img/arrow_left.png");

/// Legacy `public/img/logo-amber.png`, the mark in the corner of the
/// 600x315 cards.
const LOGO_AMBER: &[u8] = include_bytes!("../../public/img/logo-amber.png");

/// Where the per-type and per-attribute icons live, the legacy
/// `public_path('img/icons/{id}.png')`. Read from disk rather than compiled
/// in: there are 181 of them and a card uses at most a handful.
const ICON_DIR: &str = "public/img/icons";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Texture {
    Gold,
    Diamond,
    Arrow,
    ArrowLeft,
    LogoAmber,
}

impl Texture {
    /// The texture as a data URI, encoded once per process.
    fn data_uri(self) -> &'static str {
        static URIS: OnceLock<[String; 5]> = OnceLock::new();

        let uris = URIS.get_or_init(|| {
            [
                data_uri(GOLD),
                data_uri(DIAMOND),
                data_uri(ARROW),
                data_uri(ARROW_LEFT),
                data_uri(LOGO_AMBER),
            ]
        });

        match self {
            Self::Gold => &uris[0],
            Self::Diamond => &uris[1],
            Self::Arrow => &uris[2],
            Self::ArrowLeft => &uris[3],
            Self::LogoAmber => &uris[4],
        }
    }
}

/// The data URI of an embedded raster, typed by sniffing the header:
/// the EVE image server answers PNG for everything except character
/// portraits, which are JPEG, and resvg refuses a mislabelled image.
fn data_uri(bytes: &[u8]) -> String {
    const JPEG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];
    let mime = if bytes.starts_with(&JPEG_MAGIC) {
        "image/jpeg"
    } else {
        "image/png"
    };
    format!("data:{mime};base64,{}", BASE64.encode(bytes))
}

/// The icon of a type or attribute id, or `None` when it is not on disk.
/// The legacy `Image` component skipped an unreadable source so a missing
/// icon degrades the card instead of failing a request that a link unfurler
/// has no use for a 500 from; the same holds here.
pub fn icon(id: i64) -> Option<Vec<u8>> {
    std::fs::read(format!("{ICON_DIR}/{id}.png")).ok()
}

pub struct Svg {
    body: String,
    defs: String,
    next_id: u32,
}

impl Svg {
    pub fn new() -> Self {
        Self {
            body: String::new(),
            defs: String::new(),
            next_id: 0,
        }
    }

    /// Legacy `Rectangle`. Imagick's `rectangle($x1, $y1, $x2, $y2)` treats
    /// both corners as inclusive pixels, so a legacy rectangle of `width`
    /// covers `width + 1` columns; the extra pixel is kept here because the
    /// two-pixel roll bars and one-pixel borders are visibly thinner
    /// without it.
    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64, fill: &str) {
        self.stroked_rect(x, y, width, height, fill, None);
    }

    pub fn stroked_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: &str,
        stroke: Option<&str>,
    ) {
        let _ = write!(
            self.body,
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}""#,
            w = width + 1.0,
            h = height + 1.0,
            fill = paint(fill),
        );

        if let Some(stroke) = stroke {
            let _ = write!(self.body, r#" stroke="{}" stroke-width="1""#, paint(stroke));
        }

        self.body.push_str("/>");
    }

    /// Legacy `Text`. The legacy `fontWeight` argument is not ported: it was
    /// passed to `ImagickDraw::setFontWeight`, which has no effect once
    /// `setFont` has loaded a single-weight `.ttf`, so the face always came
    /// from `fontPath` alone.
    pub fn text(&mut self, x: f64, y: f64, text: &str, fill: &str, size: f64, face: Face) {
        self.text_with_paint(x, y, text, &paint(fill), size, face);
    }

    /// Legacy `TexturedText`: the glyphs are filled with a texture stretched
    /// into a `len * 10` by `20` box anchored at the text's top-left corner,
    /// the size the legacy resized the texture and its mask to.
    pub fn textured_text(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        texture: Texture,
        size: f64,
        face: Face,
    ) {
        let width = text.chars().count() as f64 * 10.0;
        let fill = self.pattern(x, y, width, 20.0, texture.data_uri());

        self.text_with_paint(x, y, text, &fill, size, face);
    }

    fn text_with_paint(&mut self, x: f64, y: f64, text: &str, paint: &str, size: f64, face: Face) {
        let font = fonts().face(face);

        let _ = write!(
            self.body,
            r#"<text x="{x}" y="{baseline}" fill="{paint}""#,
            baseline = y + font.ascent(size),
        );
        let _ = write!(
            self.body,
            r#" font-family="{family}" font-weight="{weight}" font-size="{size}""#,
            family = font.family(),
            weight = font.weight(),
        );
        let _ = write!(
            self.body,
            r#" xml:space="preserve">{}</text>"#,
            escape(text)
        );
    }

    /// Legacy `Image`: the source is stretched into the given box, never
    /// letterboxed, because Imagick's `resizeImage` ignores aspect ratio.
    pub fn image(&mut self, x: f64, y: f64, width: f64, height: f64, png: &[u8]) {
        self.image_uri(x, y, width, height, &data_uri(png));
    }

    /// A built-in texture drawn at exactly the given size, the legacy
    /// `Image` component pointed at one of the bundled PNGs.
    pub fn texture(&mut self, x: f64, y: f64, width: f64, height: f64, texture: Texture) {
        self.image_uri(x, y, width, height, texture.data_uri());
    }

    /// Legacy `GradientBar`: the texture stretched over the bar, one pixel
    /// wider and taller than the nominal box like the legacy resize.
    pub fn gradient_bar(&mut self, x: f64, y: f64, width: f64, height: f64, texture: Texture) {
        self.image_uri(x, y, width + 1.0, height + 1.0, texture.data_uri());
    }

    fn image_uri(&mut self, x: f64, y: f64, width: f64, height: f64, uri: &str) {
        let _ = write!(
            self.body,
            r#"<image x="{x}" y="{y}" width="{width}" height="{height}""#,
        );
        let _ = write!(self.body, r#" preserveAspectRatio="none" href="{uri}"/>"#);
    }

    /// Legacy `Pattern`: a square tile of `min(width, height) + 1` repeated
    /// across the box. The legacy loops step by the tile size and composite
    /// whole tiles, so the last row and column overflow the box; the filled
    /// rectangle here is rounded up to whole tiles to match.
    pub fn tiled(&mut self, x: f64, y: f64, width: f64, height: f64, texture: Texture) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let size = width.min(height) + 1.0;
        let fill = self.pattern(x, y, size, size, texture.data_uri());

        let _ = write!(
            self.body,
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}"/>"#,
            w = (width / size).ceil() * size,
            h = (height / size).ceil() * size,
        );
    }

    /// A one-tile pattern anchored at `x`/`y`, which is how both the tiled
    /// arrows and the textured text place their image in user space.
    fn pattern(&mut self, x: f64, y: f64, width: f64, height: f64, uri: &str) -> String {
        self.next_id += 1;
        let id = format!("p{}", self.next_id);

        let _ = write!(
            self.defs,
            r#"<pattern id="{id}" patternUnits="userSpaceOnUse""#,
        );
        let _ = write!(
            self.defs,
            r#" x="{x}" y="{y}" width="{width}" height="{height}">"#,
        );
        let _ = write!(
            self.defs,
            r#"<image width="{width}" height="{height}" preserveAspectRatio="none" href="{uri}"/>"#,
        );
        self.defs.push_str("</pattern>");

        format!("url(#{id})")
    }

    /// The finished document. `background` is the legacy `Canvas`
    /// background: `transparent` on the 600x315 cards (which paint their own
    /// full-bleed rectangle first) and the card colour on the module card.
    pub fn finish(self, width: u32, height: u32, background: &str) -> String {
        let mut document = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg""#);
        let _ = write!(
            document,
            r#" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );

        if !self.defs.is_empty() {
            let _ = write!(document, "<defs>{}</defs>", self.defs);
        }

        let background = paint(background);
        if background != "none" {
            let _ = write!(
                document,
                r#"<rect x="0" y="0" width="{width}" height="{height}" fill="{background}"/>"#,
            );
        }

        document.push_str(&self.body);
        document.push_str("</svg>");
        document
    }
}

/// A legacy colour string as SVG paint. The legacy palette is written in
/// the CSS `hsl(H S% L%)` form the app's stylesheet uses; it is converted
/// here rather than passed through so the output does not depend on how
/// much of CSS Color 4 the SVG parser happens to accept.
pub fn paint(color: &str) -> String {
    let color = color.trim();

    if color.eq_ignore_ascii_case("transparent") {
        return "none".to_owned();
    }

    match hsl_to_hex(color) {
        Some(hex) => hex,
        None => color.to_owned(),
    }
}

fn hsl_to_hex(color: &str) -> Option<String> {
    let inner = color
        .strip_prefix("hsl(")
        .or_else(|| color.strip_prefix("HSL("))?
        .strip_suffix(')')?;

    let mut parts = inner.split([' ', ',']).filter(|part| !part.is_empty());
    let hue: f64 = parts.next()?.parse().ok()?;
    let saturation: f64 = parts.next()?.trim_end_matches('%').parse().ok()?;
    let lightness: f64 = parts.next()?.trim_end_matches('%').parse().ok()?;
    if parts.next().is_some() {
        return None;
    }

    let saturation = saturation / 100.0;
    let lightness = lightness / 100.0;

    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue.rem_euclid(360.0) / 60.0;
    let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (red, green, blue) = match sector as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let offset = lightness - chroma / 2.0;

    let channel = |value: f64| ((value + offset) * 255.0).round().clamp(0.0, 255.0) as u8;

    Some(format!(
        "#{:02x}{:02x}{:02x}",
        channel(red),
        channel(green),
        channel(blue)
    ))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::{escape, paint};

    #[test]
    fn the_legacy_palette_converts_to_hex() {
        // The card background, border and pure primaries as a sanity check
        // of the conversion.
        assert_eq!(paint("hsl(230 16% 4%)"), "#09090c");
        assert_eq!(paint("hsl(0 0% 100%)"), "#ffffff");
        assert_eq!(paint("hsl(0 0% 0%)"), "#000000");
        assert_eq!(paint("hsl(0 100% 50%)"), "#ff0000");
        assert_eq!(paint("hsl(120 100% 50%)"), "#00ff00");
        assert_eq!(paint("hsl(240 100% 50%)"), "#0000ff");
        // Fractional lightness, as in the roll-rail background.
        assert_eq!(paint("hsl(226 11% 9.5%)").len(), 7);
    }

    #[test]
    fn non_hsl_paints_pass_through_and_transparent_becomes_none() {
        assert_eq!(paint("#92400e"), "#92400e");
        assert_eq!(paint("none"), "none");
        assert_eq!(paint("white"), "white");
        assert_eq!(paint("transparent"), "none");
    }

    #[test]
    fn text_is_xml_escaped() {
        assert_eq!(
            escape("Ammatar <Navy> & Co"),
            "Ammatar &lt;Navy&gt; &amp; Co"
        );
    }
}
