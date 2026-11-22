//! A data layer read from a query (docs/adr/9113 §1): the table a driver is handed is the query's text, a single
//! `SELECT` or `WITH`, its first line saying which column is the rows' key and which their geometry:
//!
//! ```text
//! -- kentos: key=fid geometry=geom
//! SELECT fid, ada, geom FROM parseller WHERE alan > 500
//! ```
//!
//! The line is an SQL comment: the text is still the query as written, read and run by hand the same. A driver reads
//! it as `(<query>)`, a subquery under its box, condition and order, read-only; the key is an integer column the user
//! chose (no catalog says one for a query), the geometry a geometry column or none. A text without the line is a
//! table's name (ADR 0582's bound query layer kept its own form).

/// The line's start.
const HEAD: &str = "-- kentos:";

/// A column a query reads, with what it can be for a layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryColumn {
    pub name: String,
    /// An integer: it can be the rows' key.
    pub integer: bool,
    /// A geometry: it can be the layer's geometry.
    pub geometry: bool,
}

/// The column a key is chosen from first: `fid`, `id`, `gid`, `ogc_fid` (as a table's would be), else the first integer.
pub fn likely_key(columns: &[QueryColumn]) -> Option<&str> {
    let ints = || columns.iter().filter(|c| c.integer);
    ["fid", "id", "gid", "ogc_fid"]
        .iter()
        .find_map(|k| ints().find(|c| c.name == *k))
        .or_else(|| ints().next())
        .map(|c| c.name.as_str())
}

/// A query read as a table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryTable {
    /// The query as written, without the line.
    pub sql: String,
    /// The column that tells the rows apart: an integer, unique.
    pub key: String,
    /// The geometry column; none: a table without geometry.
    pub geometry: Option<String>,
}

/// Why a text is not a query a layer can read.
pub fn check(sql: &str) -> Result<(), String> {
    let body = emptied(&strip_comments(sql));
    let body = body.trim().trim_end_matches(';').trim();
    if body.is_empty() {
        return Err("Sorgu boş; bir SELECT yazın.".to_owned());
    }
    let first: String = body.chars().take_while(|c| c.is_ascii_alphabetic()).collect::<String>().to_ascii_uppercase();
    if first != "SELECT" && first != "WITH" {
        return Err("Sorgu SELECT ya da WITH ile başlamalı: katman yalnız okuyan bir sorgudan yapılır.".to_owned());
    }
    if body.contains(';') {
        return Err("Tek bir sorgu yazın: noktalı virgülle ayrılmış birden çok deyim olmaz.".to_owned());
    }
    Ok(())
}

/// The text with what its quotes hold left out (the quotes kept): a semicolon in a text is not a statement's end.
fn emptied(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut quote: Option<char> = None;
    for c in sql.chars() {
        match quote {
            Some(q) if c == q => {
                quote = None;
                out.push(c);
            }
            Some(_) => {}
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                }
                out.push(c);
            }
        }
    }
    out
}

/// The text with its `--` and `/* */` comments left out (texts in quotes kept as they are): what is checked.
fn strip_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            out.push(c);
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                out.push(c);
            }
            '-' if chars.peek() == Some(&'-') => {
                for d in chars.by_ref() {
                    if d == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = ' ';
                for d in chars.by_ref() {
                    if last == '*' && d == '/' {
                        break;
                    }
                    last = d;
                }
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    out
}

impl QueryTable {
    /// The query a table's text holds; none for a table's name (or a text without the line).
    pub fn parse(table: &str) -> Option<QueryTable> {
        let (head, sql) = table.split_once('\n')?;
        let words = head.trim().strip_prefix(HEAD)?;
        let mut key = None;
        let mut geometry = None;
        for word in words.split_whitespace() {
            match word.split_once('=') {
                Some(("key", k)) if !k.is_empty() => key = Some(k.to_owned()),
                Some(("geometry", g)) if !g.is_empty() => geometry = Some(g.to_owned()),
                _ => {}
            }
        }
        Some(QueryTable { sql: sql.trim().trim_end_matches(';').trim().to_owned(), key: key?, geometry })
    }

    /// The table's text: the line, then the query.
    pub fn text(&self) -> String {
        let geometry = self.geometry.as_deref().map(|g| format!(" geometry={g}")).unwrap_or_default();
        format!("{HEAD} key={}{geometry}\n{}", self.key, self.sql.trim())
    }

    /// Whether a column name can go on the line: no space and no `=`.
    pub fn plain(name: &str) -> bool {
        !name.is_empty() && !name.chars().any(|c| c.is_whitespace() || c == '=')
    }

    /// The layer's name a query makes when none is given: “Sorgu” and its first table's name when it names one.
    pub fn shown(&self) -> String {
        let body = strip_comments(&self.sql);
        let words: Vec<&str> = body.split_whitespace().collect();
        let from = words
            .iter()
            .position(|w| w.eq_ignore_ascii_case("FROM"))
            .and_then(|i| words.get(i + 1))
            .map(|t| t.trim_matches(|c: char| c == '"' || c == '(' || c == ')' || c == ','))
            .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("SELECT"));
        match from {
            Some(t) => format!("Sorgu: {}", t.rsplit('.').next().unwrap_or(t)),
            None => "Sorgu".to_owned(),
        }
    }
}

/// Whether a table's text is a query a layer reads.
pub fn is_query(table: &str) -> bool {
    QueryTable::parse(table).is_some()
}

/// A write to a query refused: a query layer is read-only (a transaction's insert, update and delete ask).
pub fn refuse_write(table: &str) -> Result<(), crate::DataError> {
    if is_query(table) {
        return Err(crate::DataError::Unsupported(
            "Sorgu katmanı salt okunurdur: satırları sorgunun okuduğu tabloda değiştirilir.".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_reads_back_from_its_text() {
        let q = QueryTable { sql: "SELECT fid, geom FROM kbs.parseller WHERE alan > 500".into(), key: "fid".into(), geometry: Some("geom".into()) };
        let text = q.text();
        assert_eq!(text, "-- kentos: key=fid geometry=geom\nSELECT fid, geom FROM kbs.parseller WHERE alan > 500");
        assert_eq!(QueryTable::parse(&text), Some(q.clone()));
        assert_eq!(q.shown(), "Sorgu: parseller");
        let none = QueryTable { geometry: None, ..q };
        assert_eq!(QueryTable::parse(&none.text()), Some(none));
        assert!(QueryTable::parse("kbs.parseller").is_none(), "a table's name");
        assert!(QueryTable::parse("SELECT 1\nFROM x").is_none(), "a query without the line");
        assert!(QueryTable::parse("-- kentos: geometry=geom\nSELECT 1").is_none(), "no key");
    }

    #[test]
    fn only_one_reading_statement_is_a_query() {
        assert!(check("SELECT * FROM t").is_ok());
        assert!(check("  with a as (select 1) select * from a;").is_ok(), "a trailing semicolon");
        assert!(check("-- yorum\nSELECT 'a;b' FROM t").is_ok(), "a semicolon in a text, a comment");
        assert!(check("SELECT 1; DROP TABLE t").is_err());
        assert!(check("DELETE FROM t").is_err());
        assert!(check("/* SELECT */ UPDATE t SET a = 1").is_err());
        assert!(check("   ").is_err());
    }
}
