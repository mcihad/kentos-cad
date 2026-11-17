//! A panel's tabs (plan/eklenti.md ADIM 22, the component inventory's layout; docs/adr/8057 §43): a component's `tab`
//! key puts it on a page. The components with the same tab share it; the tab bar lists the tabs in the order they first
//! appear and stands where the first component with a tab is. A component without one is on every page, in its place:
//! a button at the body's end stays under the pages, with no end mark to write.
//!
//! - **A view, not a rule:** an input on another page is not drawn but is shown: its value goes to the command, it is
//!   asked when required and the rules read it (a folded section's way, §38).
//! - **With sections:** a section gathers the components after it up to the next section or the first component on
//!   another tab, so a button without a tab after a tab's section is on every page, not folded with it.

use serde_json::Value as Json;

/// The key a component's tab is written under.
pub const TAB_KEY: &str = "tab";

/// The most tabs a panel has.
pub const MOST_TABS: usize = 8;

/// The longest a tab's words are.
const MOST_CHARS: usize = 40;

/// A component's tab as written, trimmed; none without one, or when it is not a tab's words (Kapı 1 says so).
pub fn tab_of(component: &Json) -> Option<String> {
    component
        .get(TAB_KEY)
        .and_then(Json::as_str)
        .map(str::trim)
        .filter(|t| fits(t))
        .map(str::to_owned)
}

fn fits(words: &str) -> bool {
    !words.is_empty() && words.chars().count() <= MOST_CHARS && !words.contains(['\n', '\r'])
}

/// Kapı 1's problems with a panel's tabs, each with its JSON pointer: a tab that is not a tab's words, more than
/// [`MOST_TABS`].
pub fn check(body: &[Json]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut tabs: Vec<String> = Vec::new();
    for (i, component) in body.iter().enumerate() {
        let at = format!("/body/{i}/{TAB_KEY}");
        let tab = match component.get(TAB_KEY) {
            None => None,
            Some(Json::String(words)) if fits(words.trim()) => Some(words.trim().to_owned()),
            Some(_) => {
                out.push((
                    at,
                    format!(
                        "tab, bileşenin sekmesinin adı olmalı: boş olmayan, tek satır, en çok {MOST_CHARS} karakterlik bir söz."
                    ),
                ));
                continue;
            }
        };
        if let Some(tab) = tab
            && !tabs.contains(&tab)
        {
            tabs.push(tab);
        }
    }
    if tabs.len() > MOST_TABS {
        out.push((
            "/body".to_owned(),
            format!(
                "Panelde en çok {MOST_TABS} sekme olur; {} sekme var.",
                tabs.len()
            ),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A tab is read trimmed; a tab that is not a tab's words and a ninth tab are said where they are written.
    #[test]
    fn a_tab_is_its_words_and_a_wrong_one_is_said_where_it_is() {
        assert_eq!(
            tab_of(&json!({ "type": "textField", "tab": "  Seçenekler " })),
            Some("Seçenekler".to_owned())
        );
        assert_eq!(tab_of(&json!({ "type": "textField", "tab": "  " })), None);
        let body = vec![
            json!({ "type": "textField", "id": "ad", "label": "Ad", "tab": "Genel" }),
            json!({ "type": "numberField", "id": "adet", "label": "Adet", "tab": 3 }),
            json!({ "type": "checkbox", "id": "tum", "label": "Tümü", "tab": "çok\nsatır" }),
            json!({ "type": "button", "id": "calistir", "label": "Çalıştır", "command": "x" }),
        ];
        let said = check(&body);
        assert_eq!(
            said.iter().map(|(at, _)| at.as_str()).collect::<Vec<_>>(),
            ["/body/1/tab", "/body/2/tab"],
            "{said:?}"
        );
        let many: Vec<Json> = (0..9)
            .map(|i| json!({ "type": "text", "text": "x", "tab": format!("Sekme {i}") }))
            .collect();
        assert_eq!(
            check(&many),
            [(
                "/body".to_owned(),
                "Panelde en çok 8 sekme olur; 9 sekme var.".to_owned()
            )]
        );
    }
}
