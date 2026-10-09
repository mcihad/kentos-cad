//! Credits as the drawing's corner shows them (docs/adr/0208 §3): a
//! service's attribution (TileJSON's and a style's are HTML) as plain text
//! and its links, and the credits of the layers in view joined without
//! repeats.

use serde::Serialize;

/// A credit's text and the addresses it links to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Credit {
    pub text: String,
    pub links: Vec<(String, String)>,
}

fn entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" | "#39" => '\'',
        "nbsp" => ' ',
        "copy" => '©',
        "reg" => '®',
        "middot" => '·',
        "ndash" => '–',
        "mdash" => '—',
        n if n.starts_with("#x") || n.starts_with("#X") => {
            char::from_u32(u32::from_str_radix(&n[2..], 16).ok()?)?
        }
        n if n.starts_with('#') => char::from_u32(n[1..].parse().ok()?)?,
        _ => return None,
    })
}

/// An attribution as plain text, its tags left out and its entities read,
/// white space collapsed; the `<a href>`s as (text, address).
pub fn credit(html: &str) -> Credit {
    let mut text = String::with_capacity(html.len());
    let mut links = Vec::new();
    let mut open_link: Option<(String, usize)> = None;
    let mut rest = html;
    while let Some(c) = rest.chars().next() {
        if c == '<' {
            let Some(end) = rest.find('>') else {
                text.push_str(rest);
                break;
            };
            let tag = &rest[1..end];
            let lower = tag.to_ascii_lowercase();
            if lower.starts_with('a')
                && (lower.len() == 1 || lower[1..].starts_with(char::is_whitespace))
            {
                let href = lower.find("href=").and_then(|i| {
                    let v = &tag[i + 5..];
                    let q = v.chars().next()?;
                    if q == '"' || q == '\'' {
                        v[1..].find(q).map(|j| v[1..1 + j].to_owned())
                    } else {
                        Some(v.split_whitespace().next()?.to_owned())
                    }
                });
                open_link = href.map(|h| (h, text.len()));
            } else if lower.starts_with("/a") {
                if let Some((href, from)) = open_link.take() {
                    let shown = text[from..].trim().to_owned();
                    if !shown.is_empty() {
                        links.push((shown, href));
                    }
                }
            } else {
                // A block or a cell ends a word.
                let name = lower.trim_start_matches('/');
                let name = name
                    .split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or("");
                if matches!(
                    name,
                    "br" | "p" | "div" | "td" | "th" | "tr" | "li" | "table" | "h1" | "h2" | "h3"
                ) {
                    text.push(' ');
                }
            }
            rest = &rest[end + 1..];
        } else if c == '&' {
            match rest
                .find(';')
                .filter(|&i| i <= 10)
                .and_then(|i| entity(&rest[1..i]).map(|ch| (ch, i)))
            {
                Some((ch, i)) => {
                    text.push(ch);
                    rest = &rest[i + 1..];
                }
                None => {
                    text.push('&');
                    rest = &rest[1..];
                }
            }
        } else {
            text.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    Credit {
        text: collapsed,
        links,
    }
}

/// The credits of the layers in view, in order, each once (the same text
/// twice, or one that another holds whole, once).
pub fn joined<'a>(credits: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in credits {
        let c = c.trim();
        if c.is_empty() || out.iter().any(|o| o.contains(c)) {
            continue;
        }
        out.retain(|o| !c.contains(o.as_str()));
        out.push(c.to_owned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_credits_as_text_and_links() {
        let c = credit(
            r#"<a href="https://openfreemap.org" target="_blank">OpenFreeMap</a> <a href='https://www.openmaptiles.org/'>&copy; OpenMapTiles</a> Data from <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>"#,
        );
        assert_eq!(c.text, "OpenFreeMap © OpenMapTiles Data from OpenStreetMap");
        assert_eq!(c.links.len(), 3);
        assert_eq!(
            c.links[1],
            (
                "© OpenMapTiles".to_owned(),
                "https://www.openmaptiles.org/".to_owned()
            )
        );
        assert_eq!(
            credit("A &amp; B &#252; &unknown; C").text,
            "A & B ü &unknown; C"
        );
        assert_eq!(credit("a < b").text, "a < b");
        assert_eq!(
            joined([
                "© OpenStreetMap",
                "Esri",
                "© OpenStreetMap",
                "OpenTopoMap, © OpenStreetMap"
            ]),
            vec!["Esri", "OpenTopoMap, © OpenStreetMap"]
        );
    }
}
