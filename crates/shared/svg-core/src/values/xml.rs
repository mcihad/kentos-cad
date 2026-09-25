//! The file's elements as the page sends them: a flat list, each element
//! with its parent's place.

use kentos_style_core::js::number;

/// An element as the importer reads it (the page parses the XML). The
/// nodes of a file sit in one list (`XmlTree`): a node is its index there,
/// as the TypeScript told nodes apart by identity.
#[derive(Clone, Debug, PartialEq)]
pub struct XmlNode {
    pub tag: String,
    /// Attributes in the file's order.
    pub attrs: Vec<(String, String)>,
    pub children: Vec<usize>,
    /// Text content: of a text element (without children), of a `#text` node, of a `<style>`.
    pub text: Option<String>,
}

impl XmlNode {
    pub fn attr(&self, k: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    }

    /// Whether the element has the attribute (its own: the TypeScript's
    /// `name in attrs` also found `constructor`, `toString` … on every
    /// element, docs/adr/0008 “SVG düzenleyicisi”).
    pub fn has(&self, k: &str) -> bool {
        self.attrs.iter().any(|(n, _)| n == k)
    }
}

/// Element `i` of a JSON array; null when absent.
fn item_at(
    v: &kentos_geometry_core::api::json::Json,
    i: usize,
) -> &kentos_geometry_core::api::json::Json {
    use kentos_geometry_core::api::json::Json;
    const NULL: Json = Json::Null;
    match v {
        Json::Arr(a) => a.get(i).unwrap_or(&NULL),
        _ => &NULL,
    }
}

/// A file's elements, root first (index 0). It crosses flat, each node as
/// `[tag, attributes, text | null, parent index]` in document order, so a
/// deep file is no deep JSON.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XmlTree {
    pub nodes: Vec<XmlNode>,
}

impl kentos_geometry_core::api::json::FromJson for XmlTree {
    fn from_json(v: &kentos_geometry_core::api::json::Json) -> Result<XmlTree, String> {
        use kentos_geometry_core::api::json::Json;
        let text_of = |j: &Json| match j {
            Json::Str(s) => s.clone(),
            Json::Num(x) => number::to_string(*x),
            Json::Bool(b) => b.to_string(),
            _ => String::new(),
        };
        let Json::Arr(items) = v else {
            return Err("düğüm listesi bekleniyordu".into());
        };
        let mut nodes: Vec<XmlNode> = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let parent = match item_at(item, 3) {
                Json::Num(p) if *p >= 0.0 && (*p as usize) < i && p.fract() == 0.0 => {
                    Some(*p as usize)
                }
                _ if i == 0 => None,
                _ => return Err(format!("{i}. düğümün üst düğümü yok")),
            };
            nodes.push(XmlNode {
                tag: text_of(item_at(item, 0)),
                attrs: match item_at(item, 1) {
                    Json::Obj(f) => f.iter().map(|(k, x)| (k.clone(), text_of(x))).collect(),
                    _ => Vec::new(),
                },
                children: Vec::new(),
                text: match item_at(item, 2) {
                    Json::Null => None,
                    t => Some(text_of(t)),
                },
            });
            if let Some(p) = parent {
                nodes[p].children.push(i);
            }
        }
        if nodes.is_empty() {
            return Err("boş düğüm listesi".into());
        }
        Ok(XmlTree { nodes })
    }
}
