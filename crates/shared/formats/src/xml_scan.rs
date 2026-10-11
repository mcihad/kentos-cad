//! A streaming reader of XML for the workbooks' big parts (docs/adr/0223
//! §1): a sheet of a quarter of a million rows is a hundred megabytes of
//! XML, which a tree (roxmltree) holds ten times over, in the browser's
//! formats worker too. This reads it tag by tag and keeps only the open
//! elements and the namespaces in scope; a reader keeps what it needs as it
//! goes by, and may stop early.
//!
//! What spreadsheets write is read by XML's rules: elements and attributes
//! (a value's literal tab and line breaks a space, its references kept),
//! text with XML's five entities and character references (a CR LF or a
//! lone CR a LF), CDATA as text, comments, processing instructions and the
//! declaration skipped, a byte order mark. Refused, as not XML: a document
//! type declaration (no entity but XML's own is read), an end tag that does
//! not close the open element, an element left open, a second root, text
//! outside the root, an undeclared prefix, a repeated attribute; an element
//! deeper than [`DEPTH`] as too deep. Names are matched by their namespace's
//! URI: the scan is given the URIs its reader looks for and says which one
//! a name is in.

use std::borrow::Cow;

use crate::xml::{DEPTH, Unparsed};

/// The URI of the `xml` prefix, bound in every document.
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// A name's namespace as the reader sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ns {
    /// No namespace (an unprefixed attribute, or an element with no default namespace).
    None,
    /// A namespace the reader does not look for.
    Other,
    /// The URI at this index of those the scan was given.
    Is(u8),
}

struct Attr<'a> {
    ns: Ns,
    local: &'a str,
    raw: &'a str,
}

/// A start tag: its name and attributes. An empty element's (`<a/>`) is
/// followed by its end at once.
pub struct Start<'s, 'a> {
    pub ns: Ns,
    pub local: &'a str,
    attrs: &'s [Attr<'a>],
}

impl<'a> Start<'_, 'a> {
    /// The value of the attribute `local` in `ns` (an unprefixed one: `Ns::None`).
    pub fn attr(&self, ns: Ns, local: &str) -> Result<Option<Cow<'a, str>>, Unparsed> {
        match self.attrs.iter().find(|a| a.ns == ns && a.local == local) {
            Some(a) => decode(a.raw, true).map(Some),
            None => Ok(None),
        }
    }
}

/// What the scan read next.
pub enum Event<'s, 'a> {
    Start(Start<'s, 'a>),
    End {
        ns: Ns,
        local: &'a str,
    },
    /// Text in an element (a run between two tags, or a CDATA section).
    Text(Cow<'a, str>),
}

/// The reader of one document.
pub struct Scan<'a, 'u> {
    text: &'a str,
    at: usize,
    uris: &'u [&'u str],
    /// The open elements: their qualified names (for the end tag) and names.
    open: Vec<(&'a str, Ns, &'a str)>,
    /// The prefixes bound ("" the default namespace), what they name, and the level that bound them.
    bindings: Vec<(&'a str, Ns, usize)>,
    /// The prefixes looked up since the bindings last changed, and what they name: a big part
    /// uses a few of the dozens its root binds, so a name is resolved without going through them.
    resolved: Vec<(&'a str, Ns)>,
    attrs: Vec<Attr<'a>>,
    /// An empty element's end, given next.
    pending: Option<(Ns, &'a str)>,
    rooted: bool,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// A character reference's or entity's text after `&` up to `;`.
fn entity(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let code = if let Some(hex) = name.strip_prefix("#x") {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                name.strip_prefix('#')?.parse::<u32>().ok()?
            };
            // XML's characters: no NUL, no surrogate, nothing past U+10FFFF.
            char::from_u32(code).filter(|&c| c != '\0')
        }
    }
}

/// A raw text's or attribute value's characters: references read, CR LF and
/// a lone CR a LF; in an attribute a literal tab or LF a space.
fn decode(raw: &str, attr: bool) -> Result<Cow<'_, str>, Unparsed> {
    let special = |b: u8| b == b'&' || b == b'\r' || (attr && (b == b'\t' || b == b'\n'));
    if !raw.bytes().any(special) {
        return Ok(Cow::Borrowed(raw));
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.bytes().position(special) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        match tail.as_bytes()[0] {
            b'&' => {
                let end = tail
                    .bytes()
                    .take(12)
                    .position(|b| b == b';')
                    .ok_or(Unparsed::NotXml)?;
                out.push(entity(&tail[1..end]).ok_or(Unparsed::NotXml)?);
                rest = &tail[end + 1..];
            }
            b'\r' => {
                out.push(if attr { ' ' } else { '\n' });
                rest = tail.strip_prefix("\r\n").unwrap_or(&tail[1..]);
            }
            _ => {
                out.push(' ');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    Ok(Cow::Owned(out))
}

impl<'a, 'u> Scan<'a, 'u> {
    /// A scan of `text` looking for the namespaces `uris` (at most 254).
    pub fn new(text: &'a str, uris: &'u [&'u str]) -> Self {
        Self {
            text: text.strip_prefix('\u{feff}').unwrap_or(text),
            at: 0,
            uris,
            open: Vec::new(),
            bindings: Vec::new(),
            resolved: Vec::new(),
            attrs: Vec::new(),
            pending: None,
            rooted: false,
        }
    }

    /// How many elements are open (the root's children are at 2 while open).
    pub fn depth(&self) -> usize {
        self.open.len()
    }

    /// What a prefix ("" the default) names here.
    fn bound(&mut self, prefix: &'a str) -> Result<Ns, Unparsed> {
        if let Some((_, ns)) = self.resolved.iter().find(|(p, _)| *p == prefix) {
            return Ok(*ns);
        }
        let ns = if let Some((_, ns, _)) = self.bindings.iter().rev().find(|(p, _, _)| *p == prefix)
        {
            *ns
        } else {
            match prefix {
                "" => Ns::None,
                "xml" => self.known(XML_NS),
                _ => return Err(Unparsed::NotXml),
            }
        };
        if self.resolved.len() < 8 {
            self.resolved.push((prefix, ns));
        }
        Ok(ns)
    }

    fn known(&self, uri: &str) -> Ns {
        match self.uris.iter().position(|u| *u == uri) {
            Some(i) => u8::try_from(i).map_or(Ns::Other, Ns::Is),
            None if uri.is_empty() => Ns::None,
            None => Ns::Other,
        }
    }

    /// Just past the next `pat` from `from`.
    fn after(&self, from: usize, pat: &str) -> Result<usize, Unparsed> {
        self.text
            .get(from..)
            .and_then(|rest| rest.find(pat))
            .map(|p| from + p + pat.len())
            .ok_or(Unparsed::NotXml)
    }

    /// The open element closed: its end, the bindings it made gone.
    fn close(&mut self) -> Result<(Ns, &'a str), Unparsed> {
        let level = self.open.len();
        let (_, ns, local) = self.open.pop().ok_or(Unparsed::NotXml)?;
        while self.bindings.last().is_some_and(|(_, _, at)| *at >= level) {
            self.bindings.pop();
            self.resolved.clear();
        }
        Ok((ns, local))
    }

    /// The next event; none at the document's end.
    pub fn event(&mut self) -> Result<Option<Event<'_, 'a>>, Unparsed> {
        if self.pending.take().is_some() {
            let (ns, local) = self.close()?;
            return Ok(Some(Event::End { ns, local }));
        }
        let text = self.text;
        let bytes = text.as_bytes();
        loop {
            if self.at >= bytes.len() {
                return if self.open.is_empty() && self.rooted {
                    Ok(None)
                } else {
                    Err(Unparsed::NotXml)
                };
            }
            if bytes[self.at] != b'<' {
                let rest = &text[self.at..];
                let end = rest.find('<').map_or(bytes.len(), |p| self.at + p);
                let raw = &text[self.at..end];
                self.at = end;
                if self.open.is_empty() {
                    if raw.bytes().all(is_space) {
                        continue;
                    }
                    return Err(Unparsed::NotXml);
                }
                return Ok(Some(Event::Text(decode(raw, false)?)));
            }
            // Most tags are elements' starts and ends: told by the byte after '<'.
            match bytes.get(self.at + 1) {
                Some(b'/') => return self.end_tag(),
                Some(b'!' | b'?') => {}
                _ => return self.start_tag(),
            }
            let rest = &text[self.at..];
            if rest.starts_with("<!--") {
                self.at = self.after(self.at + 4, "-->")?;
                continue;
            }
            if rest.starts_with("<![CDATA[") {
                let end = self.after(self.at + 9, "]]>")?;
                let raw = &text[self.at + 9..end - 3];
                self.at = end;
                if self.open.is_empty() {
                    return Err(Unparsed::NotXml);
                }
                let raw = if raw.contains('\r') {
                    Cow::Owned(raw.replace("\r\n", "\n").replace('\r', "\n"))
                } else {
                    Cow::Borrowed(raw)
                };
                return Ok(Some(Event::Text(raw)));
            }
            if rest.starts_with("<?") {
                self.at = self.after(self.at + 2, "?>")?;
                continue;
            }
            // A document type declaration and its entities are not read.
            return Err(Unparsed::NotXml);
        }
    }

    /// An end tag at `self.at`.
    fn end_tag(&mut self) -> Result<Option<Event<'_, 'a>>, Unparsed> {
        let text = self.text;
        let from = self.at + 2;
        let end = from
            + text.as_bytes()[from..]
                .iter()
                .position(|&b| b == b'>')
                .ok_or(Unparsed::NotXml)?;
        let name = text[from..end].trim_end_matches([' ', '\t', '\n', '\r']);
        self.at = end + 1;
        match self.open.last() {
            Some((qname, _, _)) if *qname == name => {}
            _ => return Err(Unparsed::NotXml),
        }
        let (ns, local) = self.close()?;
        Ok(Some(Event::End { ns, local }))
    }

    /// A start tag at `self.at`.
    fn start_tag(&mut self) -> Result<Option<Event<'_, 'a>>, Unparsed> {
        let text = self.text;
        let bytes = text.as_bytes();
        let mut i = self.at + 1;
        let name_end = i + bytes[i..]
            .iter()
            .position(|&b| is_space(b) || b == b'/' || b == b'>')
            .ok_or(Unparsed::NotXml)?;
        let qname = &text[i..name_end];
        if qname.is_empty() {
            return Err(Unparsed::NotXml);
        }
        if self.open.is_empty() && self.rooted {
            return Err(Unparsed::NotXml);
        }
        if self.open.len() >= DEPTH {
            return Err(Unparsed::TooDeep);
        }
        i = name_end;
        self.attrs.clear();
        let level = self.open.len() + 1;
        let mut declared: Vec<(&'a str, &'a str)> = Vec::new();
        let empty;
        loop {
            while i < bytes.len() && is_space(bytes[i]) {
                i += 1;
            }
            match bytes.get(i) {
                Some(b'/') => {
                    if bytes.get(i + 1) != Some(&b'>') {
                        return Err(Unparsed::NotXml);
                    }
                    empty = true;
                    i += 2;
                    break;
                }
                Some(b'>') => {
                    empty = false;
                    i += 1;
                    break;
                }
                Some(_) => {}
                None => return Err(Unparsed::NotXml),
            }
            let start = i;
            while i < bytes.len() && !is_space(bytes[i]) && !matches!(bytes[i], b'=' | b'/' | b'>')
            {
                i += 1;
            }
            let name = &text[start..i];
            while i < bytes.len() && is_space(bytes[i]) {
                i += 1;
            }
            if name.is_empty() || bytes.get(i) != Some(&b'=') {
                return Err(Unparsed::NotXml);
            }
            i += 1;
            while i < bytes.len() && is_space(bytes[i]) {
                i += 1;
            }
            let quote = *bytes.get(i).ok_or(Unparsed::NotXml)?;
            if quote != b'"' && quote != b'\'' {
                return Err(Unparsed::NotXml);
            }
            let close = i
                + 1
                + bytes[i + 1..]
                    .iter()
                    .position(|&b| b == quote)
                    .ok_or(Unparsed::NotXml)?;
            let raw = &text[i + 1..close];
            if raw.contains('<') {
                return Err(Unparsed::NotXml);
            }
            i = close + 1;
            // A name given twice (its prefix aside: checked once the prefixes are read).
            if self.attrs.iter().any(|a| a.local == name) {
                return Err(Unparsed::NotXml);
            }
            if let Some(prefix) = xmlns_prefix(name) {
                declared.push((prefix, raw));
            }
            // The qualified name for now; resolved once the tag's bindings are known.
            self.attrs.push(Attr {
                ns: Ns::Other,
                local: name,
                raw,
            });
        }
        for (prefix, raw) in declared {
            let uri = decode(raw, true)?;
            let ns = self.known(&uri);
            self.bindings.push((prefix, ns, level));
            self.resolved.clear();
        }
        let (prefix, local) = split(qname);
        let ns = self.bound(prefix)?;
        for k in 0..self.attrs.len() {
            let name = self.attrs[k].local;
            let (ans, alocal) = if xmlns_prefix(name).is_some() {
                (Ns::Other, name)
            } else {
                match split(name) {
                    ("", local) => (Ns::None, local),
                    (p, local) => (self.bound(p)?, local),
                }
            };
            self.attrs[k].ns = ans;
            self.attrs[k].local = alocal;
        }
        // Two names alike once their prefixes are read.
        for a in 0..self.attrs.len() {
            for b in a + 1..self.attrs.len() {
                if self.attrs[a].ns == self.attrs[b].ns
                    && self.attrs[a].local == self.attrs[b].local
                    && self.attrs[a].ns != Ns::Other
                {
                    return Err(Unparsed::NotXml);
                }
            }
        }
        self.at = i;
        self.rooted = true;
        self.open.push((qname, ns, local));
        if empty {
            self.pending = Some((ns, local));
        }
        Ok(Some(Event::Start(Start {
            ns,
            local,
            attrs: &self.attrs,
        })))
    }

    /// The text of the element just started, up to its end (its children's
    /// text in it, their tags left out), the scan then past its end.
    pub fn text_to_end(&mut self) -> Result<String, Unparsed> {
        let level = self.open.len();
        let mut out = String::new();
        loop {
            let ended = match self.event()? {
                Some(Event::Text(t)) => {
                    out.push_str(&t);
                    false
                }
                Some(Event::End { .. }) => true,
                Some(_) => false,
                None => return Err(Unparsed::NotXml),
            };
            if ended && self.open.len() < level {
                return Ok(out);
            }
        }
    }

    /// The element just started skipped to its end.
    pub fn skip_to_end(&mut self) -> Result<(), Unparsed> {
        let level = self.open.len();
        loop {
            let ended = match self.event()? {
                Some(Event::End { .. }) => true,
                Some(_) => false,
                None => return Err(Unparsed::NotXml),
            };
            if ended && self.open.len() < level {
                return Ok(());
            }
        }
    }
}

/// `xmlns` (the default, "") or `xmlns:p` (p): the prefix a declaration binds.
fn xmlns_prefix(name: &str) -> Option<&str> {
    if name == "xmlns" {
        Some("")
    } else {
        name.strip_prefix("xmlns:")
    }
}

/// A qualified name's prefix ("" none) and local part.
fn split(qname: &str) -> (&str, &str) {
    match qname.split_once(':') {
        Some((p, l)) => (p, l),
        None => ("", qname),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "urn:a";
    const B: &str = "urn:b";

    /// The events as text: `<ns:local attr=value>`, `</ns:local>`, `"text"`.
    fn events(text: &str) -> Result<Vec<String>, Unparsed> {
        let uris = [A, B];
        let mut scan = Scan::new(text, &uris);
        let mut out = Vec::new();
        let name = |ns: Ns, local: &str| match ns {
            Ns::None => local.to_owned(),
            Ns::Other => format!("?:{local}"),
            Ns::Is(i) => format!("{}:{local}", ["a", "b"][usize::from(i)]),
        };
        while let Some(e) = scan.event()? {
            out.push(match e {
                Event::Start(s) => {
                    let mut t = format!("<{}", name(s.ns, s.local));
                    for at in ["x", "y"] {
                        if let Some(v) = s.attr(Ns::None, at)? {
                            t.push_str(&format!(" {at}={v}"));
                        }
                        if let Some(v) = s.attr(Ns::Is(1), at)? {
                            t.push_str(&format!(" b:{at}={v}"));
                        }
                    }
                    t + ">"
                }
                Event::End { ns, local } => format!("</{}>", name(ns, local)),
                Event::Text(t) => format!("{t:?}"),
            });
        }
        Ok(out)
    }

    #[test]
    fn elements_attributes_and_text_come_in_order() {
        let got = events(
            "\u{feff}<?xml version=\"1.0\"?>\n<r xmlns=\"urn:a\" xmlns:q=\"urn:b\"><c x='1' q:y=\"2\">t&amp;u&#x41;&#66;</c><q:d/><!-- <no/> --><e><![CDATA[<x>&amp;]]></e></r>\n",
        )
        .expect("XML");
        assert_eq!(
            got,
            [
                "<a:r>",
                "<a:c x=1 b:y=2>",
                "\"t&uAB\"",
                "</a:c>",
                "<b:d>",
                "</b:d>",
                "<a:e>",
                "\"<x>&amp;\"",
                "</a:e>",
                "</a:r>",
            ]
        );
    }

    #[test]
    fn line_breaks_and_attribute_values_are_normalised() {
        let got = events("<r x=\"a\tb\r\nc&#10;d\">1\r\n2\r3</r>").expect("XML");
        assert_eq!(got, ["<r x=a b c\nd>", "\"1\\n2\\n3\"", "</r>"]);
    }

    #[test]
    fn namespaces_are_scoped_to_their_element() {
        let got = events("<r><s xmlns=\"urn:b\"><t/></s><t xmlns:p=\"urn:other\"><p:u/></t></r>")
            .expect("XML");
        assert_eq!(
            got,
            [
                "<r>", "<b:s>", "<b:t>", "</b:t>", "</b:s>", "<t>", "<?:u>", "</?:u>", "</t>",
                "</r>"
            ]
        );
    }

    #[test]
    fn what_is_not_xml_is_refused() {
        for bad in [
            "",
            "<r>",
            "<r></s>",
            "<r/><s/>",
            "text<r/>",
            "<p:r/>",
            "<r x='1' x='2'/>",
            "<r xmlns:p='urn:a' xmlns:q='urn:a' p:x='1' q:x='2'/>",
            "<r x=1/>",
            "<r x='<'/>",
            "<r>&nope;</r>",
            "<r>&#0;</r>",
            "<!DOCTYPE r [<!ENTITY e 'x'>]><r>&e;</r>",
            "<r><!-- open </r>",
        ] {
            assert_eq!(events(bad).err(), Some(Unparsed::NotXml), "{bad:?}");
        }
        let deep = format!("{}{}", "<a>".repeat(DEPTH + 1), "</a>".repeat(DEPTH + 1));
        assert_eq!(events(&deep).err(), Some(Unparsed::TooDeep));
        let fine = format!("{}{}", "<a>".repeat(DEPTH), "</a>".repeat(DEPTH));
        assert!(events(&fine).is_ok());
    }

    #[test]
    fn text_to_end_gathers_the_element_and_skip_leaves_it() {
        let uris: [&str; 0] = [];
        let mut scan = Scan::new("<r><p>a<s>b</s>c</p><q>x<y/>z</q><t/></r>", &uris);
        let mut seen = Vec::new();
        while let Some(e) = scan.event().expect("XML") {
            let local = match e {
                Event::Start(s) => s.local,
                _ => continue,
            };
            match local {
                "p" => seen.push(scan.text_to_end().expect("text")),
                "q" => scan.skip_to_end().expect("skipped"),
                other => seen.push(other.to_owned()),
            }
        }
        assert_eq!(seen, ["r", "abc", "t"]);
    }

    /// Every agreement with roxmltree on what the readers ask: the elements
    /// in order with their namespace and text.
    #[test]
    fn it_reads_what_roxmltree_reads() {
        let text = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<w:sheet xmlns:w=\"urn:a\" xmlns:r=\"urn:b\"><w:row r=\"1\"><w:c r=\"A1\" t=\"s\"><w:v>0</w:v></w:c><w:c r=\"B1\"><w:is><w:t xml:space=\"preserve\"> a &lt;b&gt; </w:t></w:is></w:c></w:row></w:sheet>";
        let doc = roxmltree::Document::parse(text).expect("XML");
        let tree: Vec<String> = doc
            .descendants()
            .filter_map(|n| {
                if n.is_element() {
                    Some(format!("<{}>", n.tag_name().name()))
                } else if n.is_text() {
                    Some(format!("{:?}", n.text().unwrap_or("")))
                } else {
                    None
                }
            })
            .collect();
        let uris = [A, B];
        let mut scan = Scan::new(text, &uris);
        let mut streamed = Vec::new();
        while let Some(e) = scan.event().expect("XML") {
            match e {
                Event::Start(s) => streamed.push(format!("<{}>", s.local)),
                Event::Text(t) => streamed.push(format!("{t:?}")),
                Event::End { .. } => {}
            }
        }
        assert_eq!(streamed, tree);
    }
}
