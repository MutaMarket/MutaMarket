//! In-game item links to specific modules, as found in chat messages, mails
//! and contract descriptions: `showinfo:{type_id}//{item_id}`. Port of the
//! legacy `App\DTO\ModuleLink`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleLink {
    pub type_id: i64,
    pub item_id: i64,
}

impl ModuleLink {
    /// The first module link contained in the text, if any.
    pub fn first_from(text: &str) -> Option<Self> {
        Self::all_from(text).into_iter().next()
    }

    /// Every module link contained in the text, in order.
    pub fn all_from(text: &str) -> Vec<Self> {
        const MARKER: &str = "showinfo:";

        let mut links = Vec::new();
        let mut rest = text;

        while let Some(position) = rest.find(MARKER) {
            rest = &rest[position + MARKER.len()..];

            let Some((type_id, after_type)) = leading_integer(rest) else {
                continue;
            };

            let Some(after_separator) = after_type.strip_prefix("//") else {
                continue;
            };

            let Some((item_id, _)) = leading_integer(after_separator) else {
                continue;
            };

            links.push(Self { type_id, item_id });
        }

        links
    }
}

fn leading_integer(text: &str) -> Option<(i64, &str)> {
    let end = text
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(text.len(), |(index, _)| index);

    if end == 0 {
        return None;
    }

    text[..end].parse().ok().map(|value| (value, &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::ModuleLink;

    #[test]
    fn parses_links_out_of_chat_messages() {
        let message = "check this out <url=showinfo:47408//1037153455177>50MN MWD</url> \
                       and also <url=showinfo:47702//1028141801559>web</url>";

        assert_eq!(
            ModuleLink::first_from(message),
            Some(ModuleLink {
                type_id: 47408,
                item_id: 1037153455177,
            }),
        );
        assert_eq!(ModuleLink::all_from(message).len(), 2);
    }

    #[test]
    fn ignores_texts_without_valid_links() {
        assert_eq!(ModuleLink::first_from("no link here"), None);
        assert_eq!(ModuleLink::first_from("showinfo:123"), None);
        assert_eq!(ModuleLink::first_from("showinfo:123//"), None);
        assert_eq!(ModuleLink::first_from("showinfo://456"), None);
    }
}
