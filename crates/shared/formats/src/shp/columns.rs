//! A Shapefile's dBASE columns (docs/adr/0514): the attributes they read, their names in the file, and each one's kind
//! from what its values need. The kinds are worked out in passes over the rows, each pass a walk of every row's
//! attributes, so a writer that keeps its rows on disk (stream.rs, docs/adr/9112 §1) makes the columns the whole
//! writer makes from the same rows:
//!
//! 1. what each column's values need (the longest text, digits, fraction digits, a value its field refuses);
//! 2. a decimal column's width once its fraction digits are known.

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{LayerField, LayerFieldKind};

use super::write::{
    AsText, Column, DbfEncoding, Kind, Renamed, Written, canonical, cut_name, fixed, shown,
    significant,
};

/// A walk over the rows' attributes: it hands each row's to the function it is given.
pub(super) type Walk<'w> =
    dyn FnMut(&mut dyn FnMut(&BTreeMap<String, String>)) -> Result<(), String> + 'w;

/// A column before its kind: the attribute it reads, its field, its name in the file and as read back.
struct Named<'f> {
    key: String,
    field: Option<&'f LayerField>,
    name: Vec<u8>,
    shown: String,
}

/// The columns' attributes and names: the fields in their order, then the rows' other attributes (`keys`, every
/// attribute the rows have) by name; past dBASE's 255 left out and counted. A name that is not its attribute's is
/// noted.
fn named<'f>(
    set: &str,
    fields: &'f [LayerField],
    keys: &BTreeSet<String>,
    enc: DbfEncoding,
    out: &mut Written,
) -> Vec<Named<'f>> {
    let mut wanted: Vec<(String, Option<&LayerField>)> = Vec::new();
    for f in fields {
        if !wanted.iter().any(|(k, _)| *k == f.name) {
            wanted.push((f.name.clone(), Some(f)));
        }
    }
    for k in keys {
        if !wanted.iter().any(|(n, _)| n == k) {
            wanted.push((k.clone(), None));
        }
    }
    if wanted.len() > 255 {
        out.columns_left += (wanted.len() - 255) as u32;
        wanted.truncate(255);
    }
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut named = Vec::with_capacity(wanted.len());
    for (key, field) in wanted {
        let mut name = cut_name(&key, enc, 10);
        let mut n = 1;
        while !taken.insert(shown(&name, enc).to_lowercase()) {
            let tail = format!("_{n}");
            name = cut_name(&key, enc, 10 - tail.len());
            name.extend_from_slice(tail.as_bytes());
            n += 1;
        }
        let shown_name = shown(&name, enc);
        if shown_name != key {
            out.renamed.push(Renamed {
                set: set.to_owned(),
                attribute: key.clone(),
                column: shown_name.clone(),
            });
        }
        named.push(Named {
            key,
            field,
            name,
            shown: shown_name,
        });
    }
    named
}

/// What a column's values need, gathered a row at a time.
#[derive(Clone, Debug, Default)]
struct Gathered {
    /// A value its field's rule refuses.
    refused: bool,
    /// The longest value as it is, in the code page's bytes.
    raw: usize,
    /// The longest canonical value in the code page's bytes.
    bytes: usize,
    /// The longest canonical value's length.
    len: Option<usize>,
    /// The most fraction digits and significant digits of a canonical value.
    frac: Option<usize>,
    significant: Option<usize>,
    /// The longest canonical value with a decimal column's fraction digits (the second pass).
    fixed: Option<usize>,
}

fn most(a: Option<usize>, b: usize) -> Option<usize> {
    Some(a.map_or(b, |a| a.max(b)))
}

impl Gathered {
    fn take(&mut self, field: Option<&LayerField>, raw: Option<&String>, enc: DbfEncoding) {
        let Some(v) = raw else { return };
        self.raw = self.raw.max(enc.encode(v, &mut 0).len());
        match canonical(field, Some(v)) {
            Err(()) => self.refused = true,
            Ok(None) => {}
            Ok(Some(c)) => {
                self.bytes = self.bytes.max(enc.encode(&c, &mut 0).len());
                self.len = most(self.len, c.len());
                self.frac = most(self.frac, c.split_once('.').map_or(0, |(_, f)| f.len()));
                self.significant = most(self.significant, significant(&c));
            }
        }
    }

    fn take_fixed(&mut self, field: Option<&LayerField>, raw: Option<&String>, d: usize) {
        if let Ok(Some(c)) = canonical(field, raw)
            && let Some(f) = fixed(&c, d)
        {
            self.fixed = most(self.fixed, f.len());
        }
    }

    fn text(&self) -> Kind {
        Kind::Text(self.bytes.clamp(1, 254))
    }
}

/// A column's kind as its values decide it, or a decimal column's fraction digits whose width needs the second pass.
enum Decided {
    Kind(Kind, Option<&'static str>),
    Fixed(usize),
}

fn decide(field: Option<&LayerField>, g: &Gathered) -> Decided {
    if g.refused {
        return Decided::Kind(
            Kind::Text(g.raw.clamp(1, 254)),
            Some(
                "alanın kuralına uymayan değerleri var; değerler yazıldığı gibi metin sütununa yazıldı",
            ),
        );
    }
    let Some(f) = field else {
        return Decided::Kind(g.text(), None);
    };
    match f.kind {
        LayerFieldKind::Text => Decided::Kind(
            Kind::Text(g.bytes.max(f.length.unwrap_or(0) as usize).clamp(1, 254)),
            None,
        ),
        // A 64-bit whole number as a whole number too (docs/adr/0629 §4).
        LayerFieldKind::Integer | LayerFieldKind::BigInt => {
            let width = g.len.unwrap_or(1).max(1);
            if width > 18 {
                Decided::Kind(
                    g.text(),
                    Some(
                        "18 haneyi aşan tam sayılar dBASE okuyucularında kayan noktaya döner; tam basamaklarıyla metin sütununa yazıldı",
                    ),
                )
            } else {
                Decided::Kind(Kind::Number(width, 0), None)
            }
        }
        LayerFieldKind::Decimal => {
            // At least one fraction digit: a reader takes N without them for an integer.
            let d = g
                .frac
                .unwrap_or(0)
                .max(f.scale.unwrap_or(0) as usize)
                .max(1);
            if d > 15 || g.significant.unwrap_or(0) > 15 {
                Decided::Kind(
                    g.text(),
                    Some(
                        "15 anlamlı basamağı aşan ondalık sayılar dBASE okuyucularında yuvarlanır; tam basamaklarıyla metin sütununa yazıldı",
                    ),
                )
            } else {
                Decided::Fixed(d)
            }
        }
        LayerFieldKind::Date => Decided::Kind(Kind::Date, None),
        LayerFieldKind::Boolean => Decided::Kind(Kind::Logical, None),
        LayerFieldKind::DateTime => Decided::Kind(
            g.text(),
            Some("dBASE'te tarih-saat türü yok; YYYY-AA-GGTSS:DD:SS metniyle yazıldı"),
        ),
        LayerFieldKind::Time => Decided::Kind(
            g.text(),
            Some("dBASE'te saat türü yok; SS:DD:SS metniyle yazıldı"),
        ),
        // docs/adr/0629 §4: what dBASE has no type for goes as text, said.
        LayerFieldKind::Instant => Decided::Kind(
            g.text(),
            Some(
                "dBASE'te saat dilimli zaman türü yok; YYYY-AA-GGTSS:DD:SS ve dilimiyle metin olarak yazıldı",
            ),
        ),
        LayerFieldKind::Uuid => Decided::Kind(g.text(), None),
        LayerFieldKind::Json => Decided::Kind(
            g.text(),
            Some("dBASE'te JSON türü yok; metin olarak yazıldı"),
        ),
        LayerFieldKind::Binary => Decided::Kind(
            g.text(),
            Some("dBASE'te ikili tür yok; base64 metni olarak yazıldı"),
        ),
    }
}

/// The set's columns: named from the fields and the rows' attributes (`keys`), their kinds from the values `walk`
/// hands over, in one pass or two (a decimal column's width). A column whose field's type it cannot hold is noted
/// with why it is text.
pub(super) fn columns(
    set: &str,
    fields: &[LayerField],
    keys: &BTreeSet<String>,
    enc: DbfEncoding,
    out: &mut Written,
    walk: &mut Walk<'_>,
) -> Result<Vec<Column>, String> {
    let named = named(set, fields, keys, enc, out);
    let mut gathered = vec![Gathered::default(); named.len()];
    walk(&mut |attrs| {
        for (n, g) in named.iter().zip(gathered.iter_mut()) {
            g.take(n.field, attrs.get(&n.key), enc);
        }
    })?;
    let decided: Vec<Decided> = named
        .iter()
        .zip(&gathered)
        .map(|(n, g)| decide(n.field, g))
        .collect();
    if decided.iter().any(|d| matches!(d, Decided::Fixed(_))) {
        walk(&mut |attrs| {
            for ((n, g), d) in named.iter().zip(gathered.iter_mut()).zip(&decided) {
                if let Decided::Fixed(d) = d {
                    g.take_fixed(n.field, attrs.get(&n.key), *d);
                }
            }
        })?;
    }
    let mut cols = Vec::with_capacity(named.len());
    for ((n, g), d) in named.into_iter().zip(&gathered).zip(decided) {
        let (kind, reason) = match d {
            Decided::Kind(k, r) => (k, r),
            Decided::Fixed(d) => (
                Kind::Number(g.fixed.unwrap_or(if d > 0 { d + 2 } else { 1 }).max(1), d),
                None,
            ),
        };
        if let Some(reason) = reason {
            out.as_text.push(AsText {
                set: set.to_owned(),
                column: n.shown.clone(),
                reason,
            });
        }
        let typed = matches!(kind, Kind::Number(..) | Kind::Date | Kind::Logical);
        cols.push(Column {
            name: n.name,
            kind,
            key: n.key,
            field: if typed { n.field.cloned() } else { None },
        });
    }
    Ok(cols)
}
