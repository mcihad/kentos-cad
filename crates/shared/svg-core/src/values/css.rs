//! `<style>` sheets with the selectors icon files use (class, id, tag,
//! attribute, descendant and child), and their matching.

use kentos_geometry_core::api::json::{ToJson, write_str};
use kentos_style_core::js::text::{is_space, trim};

use super::number::split_ws;
use super::xml::XmlNode;

#[derive(Clone, Debug, PartialEq)]
pub struct Compound {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, Option<String>)>,
    /// The id was named before the tag (the order the TypeScript object held them in).
    pub id_first: bool,
}

impl ToJson for Compound {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"classes\":");
        self.classes.write_json(out);
        out.push_str(",\"attrs\":[");
        for (i, (name, value)) in self.attrs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            write_str(out, name);
            if let Some(v) = value {
                out.push_str(",\"value\":");
                write_str(out, v);
            }
            out.push('}');
        }
        out.push(']');
        let named = [("tag", &self.tag), ("id", &self.id)];
        let order: [usize; 2] = if self.id_first { [1, 0] } else { [0, 1] };
        for k in order {
            if let (name, Some(v)) = named[k] {
                out.push_str(&format!(",\"{name}\":"));
                write_str(out, v);
            }
        }
        out.push('}');
    }
}

impl ToJson for Selector {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"parts\":[");
        for (i, (c, child)) in self.parts.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"c\":");
            c.write_json(out);
            out.push_str(",\"child\":");
            child.write_json(out);
            out.push('}');
        }
        out.push_str("],\"spec\":");
        self.spec.write_json(out);
        out.push('}');
    }
}

/// Right to left: the element itself first, then its ancestors with the combinator between.
#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    pub parts: Vec<(Compound, bool)>,
    pub spec: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decl {
    pub prop: String,
    pub value: String,
    pub important: bool,
}

kentos_geometry_core::json_struct!(out Decl { prop, value, important });
kentos_geometry_core::json_struct!(out Rule { selectors, decls, order });

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub decls: Vec<Decl>,
    pub order: f64,
}

/// `/!\s*important\s*$/i`: where the leftmost such tail starts.
fn important_at(value: &str) -> Option<usize> {
    for (i, c) in value.char_indices() {
        if c != '!' {
            continue;
        }
        let r = value[1 + i..].trim_start_matches(is_space);
        if r.len() >= 9
            && r.is_char_boundary(9)
            && r[..9].eq_ignore_ascii_case("important")
            && r[9..].chars().all(is_space)
        {
            return Some(i);
        }
    }
    None
}

pub fn parse_decls(text: &str) -> Vec<Decl> {
    let mut out = Vec::new();
    for decl in text.split(';') {
        let Some(i) = decl.find(':') else { continue };
        if i == 0 {
            continue;
        }
        let mut value = trim(&decl[i + 1..]).to_string();
        let important = match important_at(&value) {
            Some(at) => {
                value = trim(&value[..at]).to_string();
                true
            }
            None => false,
        };
        out.push(Decl {
            prop: trim(&decl[..i]).to_lowercase(),
            value,
            important,
        });
    }
    out
}

fn word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// One compound selector's token (no white space inside): its parts, or None when it is not one this reads.
fn compound(t: &str, spec: &mut f64) -> Option<Compound> {
    let mut c = Compound {
        tag: None,
        id: None,
        classes: Vec::new(),
        attrs: Vec::new(),
        id_first: false,
    };
    let s: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < s.len() {
        // `([#.]?)(-?[_a-zA-Z][\w-]*)`
        let mut k = i;
        let prefix = if s[k] == '#' || s[k] == '.' {
            k += 1;
            Some(s[i])
        } else {
            None
        };
        let start = k;
        if k < s.len() && s[k] == '-' {
            k += 1;
        }
        if k < s.len() && (s[k] == '_' || s[k].is_ascii_alphabetic()) {
            k += 1;
            while k < s.len() && (word_char(s[k]) || s[k] == '-') {
                k += 1;
            }
            let name: String = s[start..k].iter().collect();
            match prefix {
                Some('#') => {
                    c.id_first |= c.tag.is_none() && c.id.is_none();
                    c.id = Some(name);
                    *spec += 10000.0;
                }
                Some(_) => {
                    c.classes.push(name);
                    *spec += 100.0;
                }
                None => {
                    c.tag = Some(name.to_lowercase());
                    *spec += 1.0;
                }
            }
            i = k;
            continue;
        }
        // `\[\s*([\w:-]+)\s*(?:=\s*["']?([^"'\]]*)["']?)?\s*\]`
        if s[i] == '[' {
            let mut k = i + 1;
            let n0 = k;
            while k < s.len() && (word_char(s[k]) || s[k] == ':' || s[k] == '-') {
                k += 1;
            }
            if k > n0 {
                let name: String = s[n0..k].iter().collect();
                let mut value = None;
                let mut ok = true;
                if k < s.len() && s[k] == '=' {
                    let mut j = k + 1;
                    if j < s.len() && (s[j] == '"' || s[j] == '\'') {
                        j += 1;
                    }
                    let v0 = j;
                    while j < s.len() && s[j] != '"' && s[j] != '\'' && s[j] != ']' {
                        j += 1;
                    }
                    let v: String = s[v0..j].iter().collect();
                    if j < s.len() && (s[j] == '"' || s[j] == '\'') {
                        j += 1;
                    }
                    if j < s.len() && s[j] == ']' {
                        value = Some(v);
                        k = j;
                    } else {
                        ok = false;
                    }
                }
                if ok && k < s.len() && s[k] == ']' {
                    c.attrs.push((name, value));
                    *spec += 100.0;
                    i = k + 1;
                    continue;
                }
            }
            return None;
        }
        if s[i] == '*' {
            i += 1;
            continue;
        }
        return None;
    }
    Some(c)
}

fn parse_selector(text: &str) -> Option<Selector> {
    // `.replace(/\s*>\s*/g, ' > ').split(/\s+/)`: '>' stands alone.
    let spaced: String = {
        let t = trim(text);
        let mut out = String::new();
        for c in t.chars() {
            if c == '>' {
                while out.ends_with(is_space) {
                    out.pop();
                }
                out.push_str(" > ");
            } else if is_space(c) && out.ends_with("> ") {
                continue;
            } else {
                out.push(c);
            }
        }
        out
    };
    let tokens: Vec<&str> = split_ws(&spaced)
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();
    let mut parts: Vec<(Compound, bool)> = Vec::new();
    let mut spec = 0.0;
    let mut child = false;
    for t in tokens {
        if t == ">" {
            child = true;
            continue;
        }
        if t.contains(['+', '~', ':']) {
            return None;
        }
        let c = compound(t, &mut spec)?;
        parts.insert(0, (c, false));
        if parts.len() > 1 {
            parts[1].1 = child;
        }
        child = false;
    }
    (!parts.is_empty()).then_some(Selector { parts, spec })
}

/// `text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/<!--|-->/g, '')`.
pub(super) fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("/*") {
        match rest[i + 2..].find("*/") {
            Some(j) => {
                out.push_str(&rest[..i]);
                rest = &rest[i + 2 + j + 2..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    // `/<!--|-->/g` in one pass: what a removal joins is not looked at again.
    let mut clean = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while !rest.is_empty() {
        if let Some(r) = rest
            .strip_prefix("<!--")
            .or_else(|| rest.strip_prefix("-->"))
        {
            rest = r;
        } else {
            let c = rest.chars().next().unwrap_or_default();
            clean.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    clean
}

/// Rules of a style sheet; at-rule blocks (@media, @font-face …) are left out.
pub fn parse_css(text: &str, order_from: f64) -> Vec<Rule> {
    let src: Vec<char> = strip_comments(text).chars().collect();
    let mut rules = Vec::new();
    let mut i = 0;
    let mut order = order_from;
    while i < src.len() {
        let Some(open) = src[i..].iter().position(|&c| c == '{').map(|k| k + i) else {
            break;
        };
        let head: String = src[i..open].iter().collect();
        let head = trim(&head).to_string();
        // Find the matching close brace (at-rules may nest).
        let mut depth = 1;
        let mut j = open + 1;
        while j < src.len() && depth > 0 {
            if src[j] == '{' {
                depth += 1;
            } else if src[j] == '}' {
                depth -= 1;
            }
            j += 1;
        }
        let body: String = if j > open + 1 {
            src[open + 1..j - 1].iter().collect()
        } else {
            String::new()
        };
        i = j;
        if head.starts_with('@') {
            continue;
        }
        let selectors: Vec<Selector> = head.split(',').filter_map(parse_selector).collect();
        if !selectors.is_empty() {
            rules.push(Rule {
                selectors,
                decls: parse_decls(&body),
                order,
            });
            order += 1.0;
        }
    }
    rules
}

fn match_compound(c: &Compound, n: &XmlNode) -> bool {
    if let Some(t) = c.tag.as_deref().filter(|t| !t.is_empty())
        && n.tag.to_lowercase() != t
    {
        return false;
    }
    if let Some(id) = c.id.as_deref().filter(|t| !t.is_empty())
        && n.attr("id") != Some(id)
    {
        return false;
    }
    if !c.classes.is_empty() {
        let cls = split_ws(n.attr("class").unwrap_or(""));
        if !c.classes.iter().all(|k| cls.contains(&k.as_str())) {
            return false;
        }
    }
    c.attrs.iter().all(|(name, value)| match value {
        None => n.has(name),
        Some(v) => n.attr(name) == Some(v.as_str()),
    })
}

/// Does the selector match node `n` whose ancestors are `path` (root first)?
pub fn matches(sel: &Selector, nodes: &[XmlNode], n: usize, path: &[usize]) -> bool {
    if !match_compound(&sel.parts[0].0, &nodes[n]) {
        return false;
    }
    let mut at = path.len();
    for k in 1..sel.parts.len() {
        let c = &sel.parts[k].0;
        let child = sel.parts[k - 1].1;
        if child {
            if at == 0 || !match_compound(c, &nodes[path[at - 1]]) {
                return false;
            }
            at -= 1;
        } else {
            while at > 0 && !match_compound(c, &nodes[path[at - 1]]) {
                at -= 1;
            }
            if at == 0 {
                return false;
            }
            at -= 1;
        }
    }
    true
}

/// Properties read from presentation attributes, rules and `style`.
pub const PROPS: [&str; 26] = [
    "fill",
    "stroke",
    "stroke-width",
    "stroke-opacity",
    "fill-opacity",
    "opacity",
    "stroke-dasharray",
    "stroke-linecap",
    "stroke-linejoin",
    "fill-rule",
    "font-size",
    "font-family",
    "font-weight",
    "text-anchor",
    "display",
    "visibility",
    "color",
    "clip-path",
    "mask",
    "filter",
    "stop-color",
    "stop-opacity",
    "marker-start",
    "marker-mid",
    "marker-end",
    "marker",
];
