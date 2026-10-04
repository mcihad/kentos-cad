//! WKT's tree (docs/adr/0168 §5): a definition read into nodes, each a
//! keyword with its items in brackets or parentheses: quoted texts, numbers,
//! bare words and inner nodes. WKT 1 and WKT 2 share it; Shapefile's `.prj`
//! reader (`formats::shp::prj`) recognises the registry's systems on it, and
//! `crs::text` reads whole systems from it. Hostile text is refused quietly:
//! a length and a nesting bound, finite numbers only.

/// A definition longer than this is not a coordinate system (and is not parsed).
pub const MAX_LEN: usize = 64 * 1024;
/// Nesting of a definition (WKT 2's bound, derived systems are nine deep).
const MAX_DEPTH: u32 = 32;

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Node(Node),
    Str(String),
    Num(f64),
    /// A bare word: `EAST`, `Cartesian`.
    Word(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub items: Vec<Item>,
}

impl Node {
    /// The first inner node of one of `names` (ignoring case).
    pub fn child(&self, names: &[&str]) -> Option<&Node> {
        self.items.iter().find_map(|i| match i {
            Item::Node(n) if names.iter().any(|w| n.name.eq_ignore_ascii_case(w)) => Some(n),
            _ => None,
        })
    }

    /// The inner nodes of one of `names` (ignoring case), in order.
    pub fn children<'a>(&'a self, names: &'a [&'a str]) -> impl Iterator<Item = &'a Node> {
        self.items.iter().filter_map(move |i| match i {
            Item::Node(n) if names.iter().any(|w| n.name.eq_ignore_ascii_case(w)) => Some(n),
            _ => None,
        })
    }

    /// The first quoted text (a node's name).
    pub fn text(&self) -> Option<&str> {
        self.items.iter().find_map(|i| match i {
            Item::Str(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// The numbers, in order.
    pub fn numbers(&self) -> Vec<f64> {
        self.items
            .iter()
            .filter_map(|i| match i {
                Item::Num(v) => Some(*v),
                _ => None,
            })
            .collect()
    }

    /// The first number.
    pub fn number(&self) -> Option<f64> {
        self.numbers().first().copied()
    }

    /// This node and every node inside it, in the text's order.
    pub fn walk(&self) -> Vec<&Node> {
        let mut out = vec![self];
        for i in &self.items {
            if let Item::Node(n) = i {
                out.extend(n.walk());
            }
        }
        out
    }

    /// The first node in the text, this one included, of one of `names`.
    pub fn find(&self, names: &[&str]) -> Option<&Node> {
        self.walk()
            .into_iter()
            .find(|n| names.iter().any(|w| n.name.eq_ignore_ascii_case(w)))
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: u32,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.b.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn word(&mut self) -> Option<String> {
        self.ws();
        let start = self.i;
        while self
            .b
            .get(self.i)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            self.i += 1;
        }
        (self.i > start).then(|| String::from_utf8_lossy(&self.b[start..self.i]).into_owned())
    }

    fn node(&mut self) -> Option<Node> {
        let name = self.word()?;
        self.ws();
        let close = match self.b.get(self.i) {
            Some(b'[') => b']',
            Some(b'(') => b')',
            _ => return None,
        };
        self.i += 1;
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return None;
        }
        let mut items = Vec::new();
        loop {
            self.ws();
            match *self.b.get(self.i)? {
                c if c == close => {
                    self.i += 1;
                    break;
                }
                b',' => self.i += 1,
                b'"' => {
                    self.i += 1;
                    let mut s = Vec::new();
                    loop {
                        match *self.b.get(self.i)? {
                            b'"' if self.b.get(self.i + 1) == Some(&b'"') => {
                                s.push(b'"');
                                self.i += 2;
                            }
                            b'"' => {
                                self.i += 1;
                                break;
                            }
                            c => {
                                s.push(c);
                                self.i += 1;
                            }
                        }
                    }
                    items.push(Item::Str(String::from_utf8_lossy(&s).into_owned()));
                }
                b'0'..=b'9' | b'-' | b'+' | b'.' => {
                    let start = self.i;
                    while self.b.get(self.i).is_some_and(|c| {
                        c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E')
                    }) {
                        self.i += 1;
                    }
                    let t = std::str::from_utf8(&self.b[start..self.i]).ok()?;
                    let v: f64 = t.parse().ok()?;
                    if !v.is_finite() {
                        return None;
                    }
                    items.push(Item::Num(v));
                }
                c if c.is_ascii_alphabetic() => {
                    let back = self.i;
                    let w = self.word()?;
                    self.ws();
                    if matches!(self.b.get(self.i), Some(b'[' | b'(')) {
                        self.i = back;
                        items.push(Item::Node(self.node()?));
                    } else {
                        items.push(Item::Word(w));
                    }
                }
                _ => return None,
            }
        }
        self.depth -= 1;
        Some(Node { name, items })
    }
}

/// The whole text as one node; none for anything else (more after it, an
/// unclosed bracket, a text too long or too deep).
pub fn parse(text: &str) -> Option<Node> {
    if text.len() > MAX_LEN {
        return None;
    }
    let mut p = Parser {
        b: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let root = p.node()?;
    p.ws();
    (p.i == p.b.len()).then_some(root)
}

/// A name compared as the rules compare names: lower case, ASCII letters and digits only.
pub fn key(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodes_texts_numbers_and_words() {
        let n = parse(r#"AXIS["Easting",EAST,ORDER[1]]"#).expect("a node");
        assert_eq!(n.name, "AXIS");
        assert_eq!(n.text(), Some("Easting"));
        assert_eq!(n.items[1], Item::Word("EAST".into()));
        assert_eq!(n.child(&["order"]).and_then(Node::number), Some(1.0));
        assert_eq!(
            parse(r#"A["x""y"]"#).and_then(|n| n.text().map(str::to_owned)),
            Some("x\"y".into())
        );
    }

    #[test]
    fn hostile_text_is_refused_quietly() {
        assert_eq!(parse(&"PROJCS[".repeat(10_000)), None);
        assert_eq!(parse("PROJCS[\"a\",1e999999]"), None);
        assert_eq!(parse("PROJCS[\"unterminated"), None);
        assert_eq!(parse("A[1] B[2]"), None);
        let long = format!("A[{}1]", "1,".repeat(40_000));
        assert_eq!(parse(&long), None);
    }
}
