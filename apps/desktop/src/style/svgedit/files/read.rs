//! SVG text into the editor's model (the web's `readSvg.ts`): the text is
//! cleaned as the library cleans it, parsed with roxmltree (after mending
//! what hand-made and exported files often break: HTML entities, prefixes
//! used without a declaration) and handed to the SVG core's importer as its
//! element list. The web's last resort, the browser's lenient HTML parser,
//! has no counterpart: such a file is refused with the parser's reason.
//! Texts keep their runs (`<tspan>`) and style sheets their text.

use kentos_svg_core::import::{ImportOptions, Imported, doc_from_svg_tree};
use kentos_svg_core::values::{XmlNode, XmlTree};

use super::super::doc::{Drawing, shape_id};

/// Elements whose text matters (and their descendants').
const TEXTUAL: [&str; 3] = ["text", "style", "title"];

/// A parse error with its line and column (1-based).
#[derive(Clone, Debug, PartialEq)]
pub struct XmlError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// The first XML error of a text, or none when it is well formed.
pub fn xml_error(text: &str) -> Option<XmlError> {
    match roxmltree::Document::parse(text) {
        Ok(_) => None,
        Err(e) => {
            let pos = e.pos();
            Some(XmlError {
                message: e.to_string(),
                line: pos.row as usize,
                column: pos.col as usize,
            })
        }
    }
}

const ENTITIES: [(&str, &str); 10] = [
    ("nbsp", "#160"),
    ("copy", "#169"),
    ("reg", "#174"),
    ("deg", "#176"),
    ("middot", "#183"),
    ("ndash", "#8211"),
    ("mdash", "#8212"),
    ("hellip", "#8230"),
    ("laquo", "#171"),
    ("raquo", "#187"),
];

/// Common breakages of hand-made and exported files, mended for strict XML (`mend`).
fn mend(text: &str) -> String {
    // Entities: the XML five stay, the known HTML ones become numbers, others text.
    let mut t = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        t.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let name_len = after
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(after.len());
        if name_len > 0 && after[name_len..].starts_with(';') {
            let name = &after[..name_len];
            let low = name.to_ascii_lowercase();
            if matches!(low.as_str(), "amp" | "lt" | "gt" | "quot" | "apos") {
                t.push('&');
                t.push_str(name);
                t.push(';');
            } else if let Some((_, num)) = ENTITIES.iter().find(|(n, _)| *n == low) {
                t.push('&');
                t.push_str(num);
                t.push(';');
            } else {
                t.push_str("&amp;");
                t.push_str(name);
                t.push(';');
            }
            rest = &after[name_len + 1..];
        } else {
            t.push('&');
            rest = after;
        }
    }
    t.push_str(rest);
    // Prefixes (xlink:, inkscape:, sodipodi: …) used without a declaration.
    let lower = t.to_ascii_lowercase();
    let Some(open) = lower.find("<svg") else {
        return t;
    };
    let Some(close) = t[open..].find('>') else {
        return t;
    };
    let root = &t[open..open + close];
    let declared: Vec<String> = root
        .match_indices("xmlns:")
        .filter_map(|(i, _)| {
            let name: String = root[i + 6..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            (!name.is_empty()).then_some(name)
        })
        .collect();
    let mut used: Vec<String> = Vec::new();
    let bytes = t.as_bytes();
    for (i, _) in t.match_indices(':') {
        // `␣prefix:name=`
        let start = t[..i]
            .rfind(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .map_or(0, |s| s + 1);
        if start == 0 || !bytes[start - 1].is_ascii_whitespace() || start == i {
            continue;
        }
        let prefix = &t[start..i];
        if !prefix.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        let name_end = t[i + 1..]
            .find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .map_or(t.len(), |e| i + 1 + e);
        if name_end == i + 1 || !t[name_end..].trim_start().starts_with('=') {
            continue;
        }
        if prefix != "xmlns" && prefix != "xml" && !used.iter().any(|u| u == prefix) {
            used.push(prefix.to_owned());
        }
    }
    let missing: Vec<&String> = used.iter().filter(|p| !declared.contains(p)).collect();
    if missing.is_empty() {
        return t;
    }
    let mut decl = String::new();
    if !root.contains("xmlns=") && !root.contains("xmlns =") {
        decl.push_str(" xmlns=\"http://www.w3.org/2000/svg\"");
    }
    for p in missing {
        let uri = if p == "xlink" {
            "http://www.w3.org/1999/xlink".to_owned()
        } else {
            format!("urn:kentos:{p}")
        };
        decl.push_str(&format!(" xmlns:{p}=\"{uri}\""));
    }
    let mut out = String::with_capacity(t.len() + decl.len());
    out.push_str(&t[..open + 4]);
    out.push_str(&decl);
    out.push_str(&t[open + 4..]);
    out
}

/// An attribute's name as the file wrote it (`xlink:href`, `inkscape:label`).
fn attr_name(node: roxmltree::Node<'_, '_>, a: &roxmltree::Attribute<'_, '_>) -> String {
    match a.namespace() {
        Some("http://www.w3.org/XML/1998/namespace") => format!("xml:{}", a.name()),
        Some(uri) => match node.lookup_prefix(uri) {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{}", a.name()),
            _ => a.name().to_owned(),
        },
        None => a.name().to_owned(),
    }
}

fn flatten(node: roxmltree::Node<'_, '_>, parent: Option<usize>, keep_text: bool, tree: &mut XmlTree) {
    let tag = node.tag_name().name().to_owned();
    let in_text = keep_text || TEXTUAL.contains(&tag.as_str());
    let at = tree.nodes.len();
    tree.nodes.push(XmlNode {
        tag,
        attrs: node.attributes().map(|a| (attr_name(node, &a), a.value().to_owned())).collect(),
        children: Vec::new(),
        text: None,
    });
    if let Some(p) = parent {
        tree.nodes[p].children.push(at);
    }
    for c in node.children() {
        if c.is_element() {
            flatten(c, Some(at), in_text, tree);
        } else if in_text && c.is_text() {
            let k = tree.nodes.len();
            tree.nodes.push(XmlNode {
                tag: "#text".to_owned(),
                attrs: Vec::new(),
                children: Vec::new(),
                text: Some(c.text().unwrap_or("").to_owned()),
            });
            tree.nodes[at].children.push(k);
        }
    }
}

/// The `<svg>` root's element list: strict XML, then mended XML; the reason when neither reads.
fn parse_tree(text: &str) -> Result<XmlTree, String> {
    let clean = kentos_native_style::file::sanitize_svg(text);
    let read = |t: &str| -> Result<XmlTree, String> {
        let options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        };
        let doc = roxmltree::Document::parse_with_options(t, options).map_err(|e| e.to_string())?;
        let root = doc.root_element();
        if root.tag_name().name() != "svg" {
            return Err("kök öğe <svg> değil".to_owned());
        }
        let mut tree = XmlTree::default();
        flatten(root, None, false, &mut tree);
        Ok(tree)
    };
    read(&clean).or_else(|_| read(&mend(&clean)))
}

/// Does this text look like SVG markup (clipboard, dropped text)?
pub fn looks_like_svg(text: &str) -> bool {
    let head: String = text.chars().take(4000).collect::<String>().to_ascii_lowercase();
    head.find("<svg")
        .is_some_and(|i| matches!(head[i + 4..].chars().next(), Some(c) if c.is_whitespace() || c == '>'))
}

/// A read drawing: the core's import with its new ids made here.
pub struct Read {
    pub doc: Drawing,
    pub imported: Imported,
}

/// SVG text as a drawing (`readSvg`), or why it cannot be read.
pub fn read_svg(text: &str, opts: &ImportOptions) -> Result<Read, String> {
    let tree = parse_tree(text).map_err(|why| format!("Dosya okunabilir bir SVG çizimi değil ({why})."))?;
    let imported = doc_from_svg_tree(&tree, opts)?;
    let mut doc = Drawing::from_obj(&imported.doc)?;
    let ids: Vec<String> = (0..imported.ids).map(|_| shape_id()).collect();
    let groups: Vec<String> = (0..imported.groups).map(|_| format!("g{}", &shape_id()[1..])).collect();
    let fresh = |v: &str| -> Option<String> {
        let mut chars = v.chars();
        let c = chars.next()?;
        let k: usize = chars.as_str().parse().ok()?;
        match c {
            '\u{1}' => ids.get(k).cloned(),
            '\u{2}' => groups.get(k).cloned(),
            _ => None,
        }
    };
    for s in &mut doc.shapes {
        if let Some(id) = s.text("id").and_then(&fresh) {
            s.set_text("id", &id);
        }
        if let Some(g) = s.text("group").and_then(&fresh) {
            s.set_text("group", &g);
        }
    }
    Ok(Read { doc, imported })
}

/// What an import did, in the status line's words (`importSummary`).
pub fn summary(r: &kentos_svg_core::import::Report) -> (String, Vec<String>) {
    let mut lost = Vec::new();
    let mut say = |n: usize, text: &str| {
        if n > 0 {
            lost.push(format!("{n} {text}"));
        }
    };
    say(r.gradients, "degrade düz renge çevrildi");
    say(r.patterns, "desen düz renge çevrildi");
    say(r.clips, "kırpma yolu atlandı");
    say(r.masks, "maske atlandı");
    say(r.filters, "süzgeç (filtre) atlandı");
    say(r.markers, "çizgi ucu işareti atlandı");
    say(r.images, "görüntü atlandı");
    say(r.broken, "kırık başvuru atlandı");
    say(r.approx_opacity, "şeklin saydamlığı yaklaşık alındı");
    let done = format!(
        "{} şekil alındı{}",
        r.shapes,
        if r.uses > 0 {
            format!(" ({} kopya açıldı)", r.uses)
        } else {
            String::new()
        }
    );
    (done, lost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mends_entities_and_undeclared_prefixes() {
        let t = mend(r#"<svg viewBox="0 0 10 10"><use xlink:href="\#a"/><text>a&nbsp;b &foo;</text></svg>"#);
        assert!(t.contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\""), "{t}");
        assert!(t.contains("xmlns=\"http://www.w3.org/2000/svg\""), "{t}");
        assert!(t.contains("a&#160;b &amp;foo;"), "{t}");
        assert!(roxmltree::Document::parse(&t).is_ok(), "{t}");
    }

    #[test]
    fn reads_a_drawing_with_fresh_ids() {
        let opts = ImportOptions {
            symbol_color: kentos_svg_core::import::SymbolColor::Auto,
            second_color: None,
            editor: false,
        };
        let r = read_svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10"><rect width="5" height="5"/><g><circle cx="10" cy="5" r="2"/><circle cx="14" cy="5" r="2"/></g></svg>"#,
            &opts,
        )
        .expect("reads");
        assert_eq!((r.doc.width, r.doc.height), (20.0, 10.0));
        assert_eq!(r.doc.shapes.len(), 3);
        assert!(r.doc.shapes.iter().all(|s| s.text("id").is_some_and(|id| id.starts_with('s'))));
        assert!(read_svg("<svg><rect", &opts).is_err());
        assert!(looks_like_svg("  <svg viewBox='0 0 1 1'/>"));
        assert!(!looks_like_svg("<svgx>"));
    }
}
