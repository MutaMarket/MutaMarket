//! The Rajdhani faces the legacy cards are drawn with, compiled into the
//! binary and loaded into resvg's font database explicitly. System fonts
//! are switched off in the `resvg` feature set, so these three faces are
//! the only ones a card can resolve and the output is identical on every
//! machine.
//!
//! The legacy passed a `.ttf` path straight to `ImagickDraw::setFont`, so
//! the face was picked by file. SVG picks a face by family plus weight, so
//! each file is registered and then looked up again in the database to
//! learn the family name and weight it was filed under.

use std::sync::{Arc, OnceLock};

use resvg::usvg::fontdb;

/// Legacy `fonts/Rajdhani-Medium.ttf`, the default of the legacy `Text`
/// component and therefore of nearly every label on a card.
const MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Rajdhani-Medium.ttf");

/// Legacy `fonts/Rajdhani-SemiBold.ttf`, the collection card's creator name.
const SEMI_BOLD: &[u8] = include_bytes!("../../assets/fonts/Rajdhani-SemiBold.ttf");

/// Legacy `fonts/Rajdhani-Bold.ttf`, the headline of every 600x315 card.
const BOLD: &[u8] = include_bytes!("../../assets/fonts/Rajdhani-Bold.ttf");

/// Which of the three faces a piece of text is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Medium,
    SemiBold,
    Bold,
}

/// A registered face: how SVG refers to it, and the metrics the legacy
/// asked Imagick for.
pub struct LoadedFace {
    family: String,
    weight: u16,
    parsed: ttf_parser::Face<'static>,
}

impl LoadedFace {
    pub fn family(&self) -> &str {
        &self.family
    }

    pub fn weight(&self) -> u16 {
        self.weight
    }

    /// Advance width of `text` at `font_size`, standing in for the legacy
    /// `queryFontMetrics(...)['textWidth']`. Kerning is ignored: it only
    /// shifts the truncation point and the value/difference gap by a
    /// fraction of a pixel.
    pub fn text_width(&self, text: &str, font_size: f64) -> f64 {
        let scale = font_size / f64::from(self.parsed.units_per_em());

        text.chars()
            .map(|character| {
                self.parsed
                    .glyph_index(character)
                    .and_then(|glyph| self.parsed.glyph_hor_advance(glyph))
                    .unwrap_or(0)
            })
            .map(|advance| f64::from(advance) * scale)
            .sum()
    }

    /// Distance from the top of the text box down to the baseline. The
    /// legacy drew with `Imagick::GRAVITY_NORTHWEST`, where the annotation
    /// coordinate is the top of the text rather than its baseline, so every
    /// ported y coordinate is shifted by this before it reaches SVG.
    pub fn ascent(&self, font_size: f64) -> f64 {
        f64::from(self.parsed.ascender()) * font_size / f64::from(self.parsed.units_per_em())
    }
}

pub struct FontSet {
    database: Arc<fontdb::Database>,
    medium: LoadedFace,
    semi_bold: LoadedFace,
    bold: LoadedFace,
}

impl FontSet {
    pub fn database(&self) -> Arc<fontdb::Database> {
        Arc::clone(&self.database)
    }

    pub fn face(&self, face: Face) -> &LoadedFace {
        match face {
            Face::Medium => &self.medium,
            Face::SemiBold => &self.semi_bold,
            Face::Bold => &self.bold,
        }
    }
}

/// The font set, built once per process.
pub fn fonts() -> &'static FontSet {
    static FONTS: OnceLock<FontSet> = OnceLock::new();

    FONTS.get_or_init(|| {
        let mut database = fontdb::Database::new();

        let medium = load(&mut database, MEDIUM);
        let semi_bold = load(&mut database, SEMI_BOLD);
        let bold = load(&mut database, BOLD);

        FontSet {
            database: Arc::new(database),
            medium,
            semi_bold,
            bold,
        }
    })
}

fn load(database: &mut fontdb::Database, data: &'static [u8]) -> LoadedFace {
    database.load_font_data(data.to_vec());

    let info = database
        .faces()
        .last()
        .expect("the face just loaded is in the database");

    LoadedFace {
        family: info
            .families
            .first()
            .map(|(family, _)| family.clone())
            .expect("a Rajdhani face names its family"),
        weight: info.weight.0,
        parsed: ttf_parser::Face::parse(data, 0).expect("a Rajdhani face parses"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Face, fonts};

    #[test]
    fn the_three_rajdhani_faces_are_registered_and_distinct() {
        let fonts = fonts();

        for face in [Face::Medium, Face::SemiBold, Face::Bold] {
            assert_eq!(fonts.face(face).family(), "Rajdhani");
        }

        // The SVG picks the file by weight, so the three must not collide.
        assert!(fonts.face(Face::Medium).weight() < fonts.face(Face::SemiBold).weight());
        assert!(fonts.face(Face::SemiBold).weight() < fonts.face(Face::Bold).weight());
    }

    #[test]
    fn text_measures_wider_the_longer_and_larger_it_gets() {
        let face = fonts().face(Face::Medium);

        assert_eq!(face.text_width("", 16.0), 0.0);
        assert!(face.text_width("Heat Sink", 16.0) > face.text_width("Heat", 16.0));
        assert!(face.text_width("Heat Sink", 30.0) > face.text_width("Heat Sink", 16.0));

        // Rajdhani is a condensed face: 16px text stays well under 16px per
        // character, which is what the legacy truncation loop relies on.
        assert!(face.text_width("Heat Sink", 16.0) < 9.0 * 16.0);
    }

    #[test]
    fn the_baseline_sits_below_the_top_of_the_text_box() {
        let face = fonts().face(Face::Medium);

        let ascent = face.ascent(16.0);
        assert!(ascent > 0.0 && ascent < 32.0);
        assert!((face.ascent(32.0) - ascent * 2.0).abs() < 1e-9);
    }
}
