//! The four cards, ported from `App\Services\OpenGraph\Components`:
//! `ModuleHeader` + `ModuleAttribute` (the module card the controller
//! assembles inline), `TypeCard`, `CharacterCard` and `CollectionCard`.
//! Every geometry literal below is the legacy one.
//!
//! The legacy `DefaultCard` is not ported: no route, controller or view
//! ever constructed it.

use super::fonts::{Face, fonts};
use super::service::CardAttribute;
use super::svg::{Svg, Texture, icon};

/// Legacy `OpenGraphController::COLOR_BACKGROUND`, also the card background
/// of every 600x315 card.
const COLOR_BACKGROUND: &str = "hsl(230 16% 4%)";

/// Legacy `COLOR_BORDER`, and the meta-group accent of anything unmapped.
const COLOR_BORDER: &str = "hsl(225 12% 18%)";

/// Legacy `COLOR_POSITIVE`, which is also `COLOR_FACTION`.
const COLOR_POSITIVE: &str = "hsl(142 71% 45%)";

/// Legacy `COLOR_STORYLINE`.
const COLOR_STORYLINE: &str = "hsl(142 77% 73%)";

/// Legacy `COLOR_T2`.
const COLOR_T2: &str = "hsl(25 95% 53%)";

/// Legacy `COLOR_DEADSPACE`.
const COLOR_DEADSPACE: &str = "hsl(220 70% 50%)";

/// Legacy `COLOR_NEGATIVE_DERIVED`, which is also `COLOR_OFFICER`.
const COLOR_NEGATIVE_DERIVED: &str = "hsl(271 91% 65%)";

/// Legacy `COLOR_MUTED_FOREGROUND`, which is also `COLOR_T1`.
const COLOR_MUTED: &str = "hsl(220 10% 62%)";

/// Legacy `ModuleHeader::TEXT_COLOR`, shared by every card headline.
const COLOR_TEXT: &str = "hsl(210 20% 98%)";

/// Legacy `CharacterCard::COLOR_ACCENT`, the rule, the corner brackets and
/// the logo, in the theme lime instead of the legacy amber.
const COLOR_ACCENT: &str = "hsl(80 100% 45%)";

/// Legacy `ModuleAttribute::BACKGROUND_COLOR`, one shade above the card.
const COLOR_ROW: &str = "hsl(228 13% 7%)";

/// Legacy `ModuleAttribute::RAIL_BACKGROUND_COLOR`.
const COLOR_RAIL: &str = "hsl(226 11% 9.5%)";

/// Legacy `ModuleAttribute::NEGATIVE_COLOR`.
const COLOR_NEGATIVE: &str = "hsl(0 84% 60%)";

/// Legacy `ModuleAttribute::POSITIVE_DERIVED_COLOR`.
const COLOR_POSITIVE_DERIVED: &str = "hsl(330 81% 60%)";

/// Legacy `ModuleAttribute::BROWN_COLOR`, the worst-roll marker.
const COLOR_BROWN: &str = "#92400e";

/// Legacy `ModuleHeader::FONT_SIZE` and `ModuleAttribute::FONT_SIZE`.
const FONT_SIZE: f64 = 16.0;

/// The module card's canvas width.
const MODULE_WIDTH: f64 = 350.0;

/// The module card's canvas padding, applied by the legacy `Container`.
const MODULE_PADDING: f64 = 10.0;

/// The height of the header and of one attribute row.
const ROW_HEIGHT: f64 = 50.0;

/// The 600x315 cards' canvas size, the legacy `WIDTH`/`HEIGHT` of
/// `TypeCard`, `CharacterCard` and `CollectionCard`.
const CARD_WIDTH: f64 = 600.0;
const CARD_HEIGHT: f64 = 315.0;

/// The bar values of the legacy `AttributeBar` enum.
const BAR_BROWN: i16 = -1;
const BAR_GOLD: i16 = 1;
const BAR_DIAMOND: i16 = 2;

/// The default description of a character card, legacy `CharacterCard`.
const CHARACTER_FALLBACK_DESCRIPTION: &str = "Abyssal Module Portfolio";

/// The default description of a collection card, legacy `CollectionCard`.
const COLLECTION_FALLBACK_DESCRIPTION: &str = "A curated abyssal module collection";

/// Longest character description before the legacy ellipsis, and the length
/// it is cut back to.
const CHARACTER_DESCRIPTION_MAX: usize = 40;
const CHARACTER_DESCRIPTION_KEPT: usize = 37;

/// The same, for the collection name and description.
const COLLECTION_NAME_MAX: usize = 28;
const COLLECTION_NAME_KEPT: usize = 25;
const COLLECTION_DESCRIPTION_MAX: usize = 45;
const COLLECTION_DESCRIPTION_KEPT: usize = 42;

pub struct ModuleCard {
    /// The mutated type, whose icon the header shows.
    pub type_id: i64,
    pub source_type_name: String,
    pub mutaplasmid_name: String,
    pub meta_group_id: Option<i64>,
    pub attributes: Vec<CardAttribute>,
}

pub struct TypeCard {
    pub id: i64,
    pub name: String,
}

pub struct CharacterCard {
    pub name: String,
    pub description: Option<String>,
    /// The portrait bytes (JPEG from the image server), absent when it
    /// could not be reached.
    pub portrait: Option<Vec<u8>>,
}

pub struct CollectionCard {
    pub name: String,
    pub description: Option<String>,
    pub creator_name: String,
    pub module_count: i64,
    pub portrait: Option<Vec<u8>>,
}

/// The meta-group accent of the header rule, legacy
/// `OpenGraphController::getColorFromMetaGroup`.
fn meta_group_color(meta_group_id: Option<i64>) -> &'static str {
    match meta_group_id {
        Some(1) => COLOR_MUTED,
        Some(2) => COLOR_T2,
        Some(3) => COLOR_STORYLINE,
        Some(4) => COLOR_POSITIVE,
        Some(5) => COLOR_NEGATIVE_DERIVED,
        Some(6) => COLOR_DEADSPACE,
        _ => COLOR_BORDER,
    }
}

pub fn module(card: &ModuleCard) -> String {
    let content_width = MODULE_WIDTH - MODULE_PADDING * 2.0;
    let height =
        ROW_HEIGHT + card.attributes.len() as f64 * ROW_HEIGHT + 2.0 + MODULE_PADDING * 2.0;

    let mut svg = Svg::new();

    module_header(
        &mut svg,
        MODULE_PADDING,
        MODULE_PADDING,
        content_width,
        card,
    );

    for (index, attribute) in card.attributes.iter().enumerate() {
        let y = MODULE_PADDING + ROW_HEIGHT + index as f64 * ROW_HEIGHT;
        // The legacy passed an `isLast` flag that `ModuleAttribute` never
        // read, so there is nothing to port for the final row.
        module_attribute(&mut svg, MODULE_PADDING, y, content_width, attribute);
    }

    svg.stroked_rect(
        MODULE_PADDING,
        MODULE_PADDING,
        content_width,
        height - MODULE_PADDING * 2.0,
        "none",
        Some(COLOR_BORDER),
    );

    svg.finish(MODULE_WIDTH as u32, height as u32, COLOR_BACKGROUND)
}

fn module_header(svg: &mut Svg, x: f64, y: f64, width: f64, card: &ModuleCard) {
    let image_size = 34.0;
    let image_padding = (ROW_HEIGHT - image_size) / 2.0;
    let max_text_width = (width - image_size - 2.0 * image_padding - 5.0).trunc();

    svg.rect(x, y, width, ROW_HEIGHT, COLOR_BACKGROUND);
    svg.rect(
        x,
        y + ROW_HEIGHT - 2.0,
        width,
        2.0,
        meta_group_color(card.meta_group_id),
    );

    if let Some(png) = icon(card.type_id) {
        svg.image(
            x + image_padding,
            y + image_padding,
            image_size,
            image_size,
            &png,
        );
    }

    let text_x = x + image_size + 2.0 * image_padding;
    svg.text(
        text_x,
        y + 6.0,
        &truncate_to_width(&card.source_type_name, max_text_width),
        COLOR_TEXT,
        FONT_SIZE,
        Face::Medium,
    );
    svg.text(
        text_x,
        y + 23.0,
        &truncate_to_width(&card.mutaplasmid_name, max_text_width),
        COLOR_MUTED,
        FONT_SIZE,
        Face::Medium,
    );
}

/// Legacy `ModuleHeader::truncateText`: drop four characters, trim, and
/// re-append an ellipsis until the line fits. After the first pass that
/// shortens the text by one character each time, because the ellipsis it
/// just added is part of the four characters the next pass drops.
fn truncate_to_width(text: &str, max_width: f64) -> String {
    let font = fonts().face(Face::Medium);
    let mut text = text.to_owned();

    while font.text_width(&text, FONT_SIZE) > max_width && text.chars().count() > 4 {
        let kept: String = text
            .chars()
            .take(text.chars().count() - 4)
            .collect::<String>()
            .trim()
            .to_owned();
        text = format!("{kept}...");
    }

    text
}

fn module_attribute(svg: &mut Svg, x: f64, y: f64, width: f64, attribute: &CardAttribute) {
    let text_offset = x + 8.0 + 34.0 + 8.0;
    let text_y = y + 23.0;
    let rail_y = y + 44.0;
    let rail_start = x + 5.0;
    let rail_end = x + width - 5.0;
    let rail_width = rail_end - rail_start;
    let rail_center = (rail_start + rail_width / 2.0).trunc();
    let rail_half_width = (rail_width / 2.0).trunc();

    svg.rect(x, y, width, ROW_HEIGHT, COLOR_ROW);

    if let Some(png) = icon(attribute.id) {
        svg.image(x + 12.0, y + 10.0, 28.0, 28.0, &png);
    }

    svg.text(
        text_offset,
        y + 6.0,
        &attribute.name,
        COLOR_MUTED,
        FONT_SIZE - 2.0,
        Face::Medium,
    );
    svg.text(
        text_offset,
        text_y,
        &attribute.value,
        COLOR_TEXT,
        FONT_SIZE,
        Face::Medium,
    );

    let value_width = fonts()
        .face(Face::Medium)
        .text_width(&attribute.value, FONT_SIZE);
    let difference_x = (text_offset + value_width + 5.0).trunc();
    difference(svg, difference_x, text_y, attribute);

    svg.rect(rail_start, rail_y, rail_width, 2.0, COLOR_RAIL);

    if attribute.is_positive {
        positive_rail(svg, rail_center, rail_y, rail_half_width, attribute);
    } else {
        negative_rail(svg, rail_start, rail_y, rail_half_width, attribute);
    }

    svg.rect(x, y, width, 1.0, COLOR_BORDER);
}

/// Legacy `createDifferenceValueComponent`.
fn difference(svg: &mut Svg, x: f64, y: f64, attribute: &CardAttribute) {
    match attribute.bar {
        BAR_GOLD => svg.textured_text(
            x,
            y,
            &attribute.difference,
            Texture::Gold,
            FONT_SIZE,
            Face::Medium,
        ),
        BAR_DIAMOND => svg.textured_text(
            x,
            y,
            &attribute.difference,
            Texture::Diamond,
            FONT_SIZE,
            Face::Medium,
        ),
        BAR_BROWN => svg.text(
            x,
            y,
            &attribute.difference,
            COLOR_BROWN,
            FONT_SIZE,
            Face::Medium,
        ),
        _ => {
            let color = match (attribute.is_positive, attribute.derived) {
                (true, true) => COLOR_POSITIVE_DERIVED,
                (true, false) => COLOR_POSITIVE,
                (false, true) => COLOR_NEGATIVE_DERIVED,
                (false, false) => COLOR_NEGATIVE,
            };
            svg.text(x, y, &attribute.difference, color, FONT_SIZE, Face::Medium);
        }
    }
}

/// Legacy `createPositiveRail`: the bar grows right from the middle of the
/// rail. A gold or diamond bar is drawn at full half-width rather than at
/// the roll's width, because those bars only ever mark a perfect roll.
fn positive_rail(svg: &mut Svg, center: f64, y: f64, half_width: f64, attribute: &CardAttribute) {
    let bar_width = (attribute.fraction * half_width).trunc();

    match attribute.bar {
        BAR_GOLD => svg.gradient_bar(center, y, half_width, 2.0, Texture::Gold),
        BAR_DIAMOND => svg.gradient_bar(center, y, half_width, 2.0, Texture::Diamond),
        _ => {
            let color = if attribute.derived {
                COLOR_POSITIVE_DERIVED
            } else {
                COLOR_POSITIVE
            };
            svg.rect(center, y, bar_width, 2.0, color);
        }
    }

    svg.tiled(center, y, bar_width, 2.0, Texture::Arrow);
    svg.rect(center + bar_width, y, 1.0, 2.0, COLOR_TEXT);
}

/// Legacy `createNegativeRail`: the bar grows left towards the start of the
/// rail. A brown bar fills the whole left half; gold and diamond rolls fall
/// through to the plain negative colour, as they do in the legacy.
fn negative_rail(svg: &mut Svg, start: f64, y: f64, half_width: f64, attribute: &CardAttribute) {
    let bar_width = (attribute.fraction * half_width).trunc();
    let cap_x = start + (half_width - bar_width);

    if attribute.bar == BAR_BROWN {
        svg.rect(start, y, half_width, 2.0, COLOR_BROWN);
    } else {
        let color = if attribute.derived {
            COLOR_NEGATIVE_DERIVED
        } else {
            COLOR_NEGATIVE
        };
        svg.rect(cap_x, y, bar_width, 2.0, color);
    }

    svg.tiled(cap_x, y, bar_width, 2.0, Texture::ArrowLeft);
    svg.rect(cap_x, y, 1.0, 2.0, COLOR_TEXT);
}

pub fn type_card(card: &TypeCard) -> String {
    let mut svg = Svg::new();
    card_chrome(&mut svg);

    let left_x = 60.0;
    let logo_y = 36.0;
    let logo_height = 24.0;
    let icon_size = 80.0;
    let icon_y = logo_y + logo_height + 30.0;

    if let Some(png) = icon(card.id) {
        svg.image(left_x, icon_y, icon_size, icon_size, &png);
    }

    svg.rect(left_x, icon_y + icon_size + 16.0, 40.0, 2.0, COLOR_ACCENT);
    svg.text(
        left_x,
        icon_y + icon_size + 28.0,
        &strip_type_prefixes(&card.name),
        COLOR_TEXT,
        26.0,
        Face::Bold,
    );
    svg.text(
        left_x,
        icon_y + icon_size + 60.0,
        "Abyssal Modules",
        COLOR_MUTED,
        20.0,
        Face::Medium,
    );

    svg.finish(CARD_WIDTH as u32, CARD_HEIGHT as u32, "transparent")
}

pub fn character_card(card: &CharacterCard) -> String {
    let mut svg = Svg::new();
    card_chrome(&mut svg);

    let portrait_size = 130.0;
    let left_x = 60.0;
    let portrait_y = ((CARD_HEIGHT - portrait_size) / 2.0).trunc();

    svg.stroked_rect(
        left_x - 2.0,
        portrait_y - 2.0,
        portrait_size + 4.0,
        portrait_size + 4.0,
        "none",
        Some(COLOR_BORDER),
    );
    if let Some(portrait) = &card.portrait {
        svg.image(left_x, portrait_y, portrait_size, portrait_size, portrait);
    }

    let text_x = left_x + portrait_size + 32.0;

    svg.text(
        text_x,
        portrait_y + 14.0,
        &card.name,
        COLOR_TEXT,
        30.0,
        Face::Bold,
    );
    svg.rect(text_x, portrait_y + 52.0, 40.0, 2.0, COLOR_ACCENT);

    let description = ellipsize(
        or_fallback(card.description.as_deref(), CHARACTER_FALLBACK_DESCRIPTION),
        CHARACTER_DESCRIPTION_MAX,
        CHARACTER_DESCRIPTION_KEPT,
    );
    svg.text(
        text_x,
        portrait_y + 66.0,
        &description,
        COLOR_MUTED,
        18.0,
        Face::Medium,
    );

    svg.finish(CARD_WIDTH as u32, CARD_HEIGHT as u32, "transparent")
}

pub fn collection_card(card: &CollectionCard) -> String {
    let mut svg = Svg::new();
    card_chrome(&mut svg);

    let left_x = 60.0;

    let name = ellipsize(&card.name, COLLECTION_NAME_MAX, COLLECTION_NAME_KEPT);
    svg.text(left_x, 50.0, &name, COLOR_TEXT, 30.0, Face::Bold);
    svg.rect(left_x, 90.0, 40.0, 2.0, COLOR_ACCENT);

    let description = ellipsize(
        or_fallback(card.description.as_deref(), COLLECTION_FALLBACK_DESCRIPTION),
        COLLECTION_DESCRIPTION_MAX,
        COLLECTION_DESCRIPTION_KEPT,
    );
    svg.text(left_x, 104.0, &description, COLOR_MUTED, 18.0, Face::Medium);

    let creator_y = 150.0;
    let portrait_size = 44.0;

    svg.stroked_rect(
        left_x - 1.0,
        creator_y - 1.0,
        portrait_size + 2.0,
        portrait_size + 2.0,
        "none",
        Some(COLOR_BORDER),
    );
    if let Some(portrait) = &card.portrait {
        svg.image(left_x, creator_y, portrait_size, portrait_size, portrait);
    }

    let text_x = left_x + portrait_size + 12.0;
    svg.text(
        text_x,
        creator_y + 4.0,
        &card.creator_name,
        COLOR_TEXT,
        16.0,
        Face::SemiBold,
    );

    let modules = if card.module_count == 1 {
        "module"
    } else {
        "modules"
    };
    svg.text(
        text_x,
        creator_y + 24.0,
        &format!("{} {modules}", card.module_count),
        COLOR_MUTED,
        14.0,
        Face::Medium,
    );

    svg.finish(CARD_WIDTH as u32, CARD_HEIGHT as u32, "transparent")
}

/// The background, corner brackets and logo the three 600x315 cards open
/// with, identical in all of them. Deliberate divergence: the legacy drew
/// the brackets in `COLOR_BORDER` with only a 2px accent dot at the
/// corner, which reads as a stray dot next to a grey line; here the whole
/// bracket takes the accent.
fn card_chrome(svg: &mut Svg) {
    svg.rect(0.0, 0.0, CARD_WIDTH, CARD_HEIGHT, COLOR_BACKGROUND);

    svg.rect(24.0, 24.0, 36.0, 1.0, COLOR_ACCENT);
    svg.rect(24.0, 24.0, 1.0, 28.0, COLOR_ACCENT);
    svg.rect(24.0, 24.0, 2.0, 2.0, COLOR_ACCENT);

    svg.rect(
        CARD_WIDTH - 60.0,
        CARD_HEIGHT - 25.0,
        36.0,
        1.0,
        COLOR_ACCENT,
    );
    svg.rect(
        CARD_WIDTH - 25.0,
        CARD_HEIGHT - 52.0,
        1.0,
        28.0,
        COLOR_ACCENT,
    );
    svg.rect(
        CARD_WIDTH - 26.0,
        CARD_HEIGHT - 26.0,
        2.0,
        2.0,
        COLOR_ACCENT,
    );

    // The legacy logo box, 44x24, is the mark's own 394:217 proportion.
    let logo_width = 44.0;
    svg.logo(
        CARD_WIDTH - logo_width - 40.0,
        36.0,
        logo_width,
        COLOR_ACCENT,
    );
}

/// Legacy `TypeCard`'s `preg_replace('/\b(Abyssal|Mutated)\s+/i', '', ...)`
/// followed by a trim: every whole word `Abyssal` or `Mutated` that is
/// followed by whitespace disappears, along with that whitespace.
fn strip_type_prefixes(name: &str) -> String {
    const PREFIXES: [&str; 2] = ["abyssal", "mutated"];

    let characters: Vec<char> = name.chars().collect();
    let mut result = String::with_capacity(name.len());
    let mut index = 0;

    while index < characters.len() {
        let at_word_boundary = index == 0
            || !(characters[index - 1].is_alphanumeric() || characters[index - 1] == '_');
        let matched = at_word_boundary
            .then(|| {
                PREFIXES.into_iter().find(|prefix| {
                    let end = index + prefix.chars().count();
                    end < characters.len()
                        && characters[index..end]
                            .iter()
                            .zip(prefix.chars())
                            .all(|(character, expected)| character.to_ascii_lowercase() == expected)
                        && characters[end].is_whitespace()
                })
            })
            .flatten();

        match matched {
            Some(prefix) => {
                index += prefix.chars().count();
                while index < characters.len() && characters[index].is_whitespace() {
                    index += 1;
                }
            }
            None => {
                result.push(characters[index]);
                index += 1;
            }
        }
    }

    result.trim().to_owned()
}

/// The legacy `?:` fallback: PHP treats `null`, the empty string and the
/// string `"0"` as falsy, so all three fall back to the default text.
fn or_fallback<'a>(value: Option<&'a str>, fallback: &'a str) -> &'a str {
    match value {
        Some(value) if !value.is_empty() && value != "0" => value,
        _ => fallback,
    }
}

/// The legacy `mb_strlen(...) > max ? mb_substr(..., 0, kept).'...' : ...`.
fn ellipsize(text: &str, max: usize, kept: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }

    format!("{}...", text.chars().take(kept).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::{
        CARD_HEIGHT, CARD_WIDTH, COLOR_ACCENT, COLOR_BORDER, COLOR_DEADSPACE, COLOR_MUTED,
        COLOR_T2, Svg, card_chrome, ellipsize, meta_group_color, or_fallback, strip_type_prefixes,
        truncate_to_width,
    };
    use crate::og::svg::paint;

    #[test]
    fn card_chrome_draws_accent_brackets_and_the_vector_mark() {
        let mut svg = Svg::new();
        card_chrome(&mut svg);
        let document = svg.finish(CARD_WIDTH as u32, CARD_HEIGHT as u32, "transparent");

        let accent = format!(r#"fill="{}""#, paint(COLOR_ACCENT));
        // Three rectangles per bracket, plus the mark.
        assert_eq!(document.matches(&accent).count(), 7);
        assert!(!document.contains(&paint(COLOR_BORDER)));
        assert!(document.contains("<path fill=\""));
        assert!(!document.contains("data:image"));
    }

    #[test]
    fn meta_groups_map_to_the_legacy_accents() {
        assert_eq!(meta_group_color(Some(1)), COLOR_MUTED);
        assert_eq!(meta_group_color(Some(2)), COLOR_T2);
        assert_eq!(meta_group_color(Some(6)), COLOR_DEADSPACE);
        assert_eq!(meta_group_color(Some(99)), COLOR_BORDER);
        assert_eq!(meta_group_color(None), COLOR_BORDER);
    }

    #[test]
    fn type_names_lose_their_abyssal_and_mutated_prefixes() {
        assert_eq!(
            strip_type_prefixes("Abyssal Damage Control"),
            "Damage Control"
        );
        assert_eq!(strip_type_prefixes("Mutated Heat Sink"), "Heat Sink");
        assert_eq!(
            strip_type_prefixes("Abyssal Mutated Heat Sink"),
            "Heat Sink"
        );
        // Case-insensitive, and mid-name occurrences go too.
        assert_eq!(strip_type_prefixes("Large abyssal Shield"), "Large Shield");
        // A word that merely starts with the prefix is left alone, and so is
        // a trailing prefix with no whitespace after it.
        assert_eq!(
            strip_type_prefixes("Abyssalized Plate"),
            "Abyssalized Plate"
        );
        assert_eq!(strip_type_prefixes("Deep Abyssal"), "Deep Abyssal");
    }

    #[test]
    fn php_falsy_descriptions_fall_back() {
        assert_eq!(or_fallback(None, "fallback"), "fallback");
        assert_eq!(or_fallback(Some(""), "fallback"), "fallback");
        assert_eq!(or_fallback(Some("0"), "fallback"), "fallback");
        assert_eq!(or_fallback(Some("Rolls"), "fallback"), "Rolls");
    }

    #[test]
    fn long_text_is_cut_back_and_ellipsized() {
        assert_eq!(ellipsize("short", 10, 7), "short");
        assert_eq!(ellipsize("exactly-10", 10, 7), "exactly-10");
        assert_eq!(ellipsize("more than ten", 10, 7), "more th...");
        // Counted in characters, not bytes, like mb_strlen/mb_substr.
        assert_eq!(ellipsize("ααααααααααα", 10, 7), "ααααααα...");
    }

    #[test]
    fn header_text_shrinks_until_it_fits() {
        let long = "Ammatar Navy Large Micro Jump Drive Extender II";

        let wide = truncate_to_width(long, 1000.0);
        assert_eq!(wide, long, "text that already fits is untouched");

        let narrow = truncate_to_width(long, 120.0);
        assert!(narrow.ends_with("..."));
        assert!(narrow.chars().count() < long.chars().count());

        // The loop stops at four characters rather than looping forever on
        // text that can never fit.
        assert_eq!(truncate_to_width("abcd", 0.0), "abcd");
    }
}
