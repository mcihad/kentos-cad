//! The type cards' pictures: the web's own SVGs (apps/web/src/ui/settings/art),
//! drawn in the theme. On the web CSS colours them: the lines in the card's
//! secondary ink, the `hi` ones quieter until the card is chosen, then in the
//! accent; the `dim` ones at half strength (wizard.css). Here the same colours
//! are written into the markup before it is drawn.

use iced::widget::svg;
use iced::{Color, Element, Fill, Theme};
use kentos_contracts::Workspace;
use kentos_ui::theme::Tokens;

use crate::app::Message;

const CAD: &str = include_str!("../../../../web/src/ui/settings/art/project-cad.svg");
const GIS: &str = include_str!("../../../../web/src/ui/settings/art/project-gis.svg");

/// A type's picture, chosen or not, in the theme's colours: the handle is
/// the markup in those colours, one per theme and state (Iced keeps its
/// rasterized copy by the markup).
pub(super) fn picture<'a>(kind: Workspace, on: bool, theme: &Theme) -> Element<'a, Message> {
    let markup = match kind {
        Workspace::Cad => CAD,
        _ => GIS,
    };
    let (ink, hi) = colours(theme, on);
    svg(svg::Handle::from_memory(
        themed(markup, ink, hi).into_bytes(),
    ))
    .width(Fill)
    .height(Fill)
    .into()
}

/// The markup with the theme's colours in place of `currentColor`: the
/// lines in `ink`, the `hi` ones in `hi`, the `dim` ones at half strength.
pub(super) fn themed(markup: &str, ink: Color, hi: Color) -> String {
    let (ink, hi) = (hex(ink), hex(hi));
    markup
        .lines()
        .map(|line| {
            if line.contains("class=\"hi\"") {
                let line = line.replace("currentColor", &hi);
                // Its stroke is inherited from the picture's: given its own.
                if line.contains(" stroke=\"") {
                    line
                } else {
                    with_attribute(&line, &format!(" stroke=\"{hi}\""))
                }
            } else if line.contains("class=\"dim\"") {
                with_attribute(&line.replace("currentColor", &ink), " opacity=\".5\"")
            } else {
                line.replace("currentColor", &ink)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The element's line with `attribute` after its tag's name.
fn with_attribute(line: &str, attribute: &str) -> String {
    let Some(open) = line.find('<') else {
        return line.to_owned();
    };
    let name_end = line[open + 1..]
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .map_or(line.len(), |e| open + 1 + e);
    format!("{}{attribute}{}", &line[..name_end], &line[name_end..])
}

fn hex(c: Color) -> String {
    let [r, g, b, _] = c.into_rgba8();
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// The colours a card's picture is drawn in (wizard.css): the secondary ink
/// and a quiet `hi` at rest; the text's ink and the accent once chosen.
pub(super) fn colours(theme: &Theme, on: bool) -> (Color, Color) {
    let t = Tokens::of(theme);
    if on {
        (t.text, t.accent_hover)
    } else {
        (t.muted, t.faint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pictures_take_the_themes_colours_line_by_line() {
        let ink = Color::from_rgb8(0x9b, 0xa7, 0xb5);
        let hi = Color::from_rgb8(0x6c, 0x96, 0xe6);
        let cad = themed(CAD, ink, hi);
        assert!(!cad.contains("currentColor"));
        // The picture's own stroke is the ink; a `hi` line without its own stroke gets the accent's.
        assert!(cad.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 240 120\" fill=\"none\" stroke=\"#9ba7b5\""));
        assert!(cad.contains("<path stroke=\"#6c96e6\" class=\"hi\" d=\"M80 70h48M104 46v48\""));
        // One with its own (none) keeps it, its fill in the accent.
        assert!(cad.contains("<rect class=\"hi\" x=\"92\" y=\"15\" width=\"24\" height=\"4\" rx=\"1\" fill=\"#6c96e6\" stroke=\"none\"/>"));
        // `dim` lines at half strength.
        assert!(cad.contains("<path opacity=\".5\" class=\"dim\""));
        let gis = themed(GIS, ink, hi);
        assert!(!gis.contains("currentColor"));
        assert_eq!(gis.lines().count(), GIS.lines().count());
    }
}
