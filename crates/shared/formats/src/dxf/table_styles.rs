//! A DXF's table styles (docs/adr/1128 §4): the TABLESTYLE objects the ACAD_TABLESTYLE dictionary names. One with
//! KentOS's data is the project table style it carries, exactly. Another program's is taken when a table follows it
//! (an ACAD_TABLE's group 342): its groups as Autodesk's DXF reference gives them, the three cell styles in the order
//! data, title, heading (Open Design's specification, as LibreDWG's `dwg2.spec` reads it); sizes from drawing units
//! to paper mm at the project's plot scale; ACI colours as `#rrggbb`. What KentOS cannot keep (an upward table, a
//! suppressed title or heading, top or bottom alignment, borders that differ or are hidden) is a decision of the
//! report; a style no table follows is left out and said.

use std::collections::{HashMap, HashSet};

use kentos_contracts::ImportDecisionTopic::Style;
use kentos_contracts::{TableAlign, TableCellStyleDef, TableStyleDef};

use super::lexer::Lexer;
use super::strings::Decoder;
use super::{aci, entity, styles};
use crate::num::{parse_int, parse_real};
use crate::report::Report;

/// The largest size a cell style keeps, paper mm (`MAX_TABLE_STYLE_SIZE`).
const MOST: f64 = kentos_contracts::MAX_TABLE_STYLE_SIZE;

/// What the file's table styles came to.
#[derive(Debug, Default)]
pub struct Read {
    pub styles: Vec<TableStyleDef>,
    /// The text styles their cell styles name (the STYLE records' names, upper case): kept with them.
    pub text_styles: Vec<String>,
}

struct Object {
    groups: Vec<(i32, String)>,
    kentos: Option<TableStyleDef>,
}

/// An ACI number as a colour a cell style keeps: none for ByBlock, ByLayer and 7 (the drawing's ink).
fn colour(n: Option<i64>) -> Option<String> {
    let n = u8::try_from(n?).ok().filter(|n| (1..=255).contains(n) && *n != 7)?;
    let [r, g, b] = aci::rgb(n);
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

/// A fill's ACI number as a colour (7 its white paper).
fn fill(n: Option<i64>) -> Option<String> {
    let n = u8::try_from(n?).ok().filter(|n| (1..=255).contains(n))?;
    let [r, g, b] = aci::rgb(n);
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

/// Reads the file's table styles; `metres` turns a drawing length into metres, `plot` the project's plot scale.
pub fn read(
    bytes: &[u8],
    dec: Decoder,
    metres: &dyn Fn(f64) -> f64,
    plot: f64,
    report: &mut Report,
) -> Result<Read, String> {
    let mut lex = Lexer::new(bytes);
    let mut dictionaries: HashMap<u64, Vec<(String, u64)>> = HashMap::new();
    let mut root: Option<u64> = None;
    let mut objects: HashMap<u64, Object> = HashMap::new();
    let mut order: Vec<u64> = Vec::new();
    let mut followed: HashSet<u64> = HashSet::new();
    while let Some(p) = lex.next()? {
        if p.code != 0 {
            continue;
        }
        let kind = p.text().to_uppercase();
        if !matches!(kind.as_str(), "DICTIONARY" | "TABLESTYLE" | "ACAD_TABLE") {
            continue;
        }
        let groups = lex.until_zero()?;
        let hex = |code: i32| {
            groups.iter().find(|g| g.code == code).and_then(|g| u64::from_str_radix(g.text(), 16).ok())
        };
        match kind.as_str() {
            "ACAD_TABLE" => followed.extend(hex(342)),
            "DICTIONARY" => {
                let Some(h) = hex(5) else { continue };
                let mut entries = Vec::new();
                let mut name = None;
                for g in &groups {
                    match g.code {
                        3 => name = Some(dec.string(g.value)),
                        350 | 360 => {
                            if let (Some(n), Ok(t)) = (name.take(), u64::from_str_radix(g.text(), 16)) {
                                entries.push((n, t));
                            }
                        }
                        _ => {}
                    }
                }
                // The named object dictionary: the first, owned by no object.
                if root.is_none() && groups.iter().find(|g| g.code == 330).is_some_and(|g| g.text() == "0") {
                    root = Some(h);
                }
                dictionaries.insert(h, entries);
            }
            _ => {
                let Some(h) = hex(5) else { continue };
                let kentos = entity::xdata_of(&groups, dec)
                    .and_then(|m| m.table_style)
                    .and_then(|j| serde_json::from_str::<TableStyleDef>(&j).ok())
                    .filter(|s| s.problem().is_none());
                let groups = groups.iter().filter(|g| g.code < 1000).map(|g| (g.code, dec.string(g.value))).collect();
                objects.insert(h, Object { groups, kentos });
                order.push(h);
            }
        }
    }
    // The styles by the ACAD_TABLESTYLE dictionary's names, in its order; objects it does not name by their own.
    let named: Vec<(String, u64)> = root
        .and_then(|r| dictionaries.get(&r))
        .and_then(|e| e.iter().find(|(n, _)| n.eq_ignore_ascii_case("ACAD_TABLESTYLE")))
        .and_then(|(_, h)| dictionaries.get(h))
        .cloned()
        .unwrap_or_default();
    let mut listed: Vec<(String, u64)> = named.clone();
    for h in &order {
        if !named.iter().any(|(_, n)| n == h) {
            let description = objects[h].groups.iter().find(|g| g.0 == 3).map(|g| g.1.clone()).unwrap_or_default();
            listed.push((description, *h));
        }
    }
    let mut out = Read::default();
    let mut unused = Vec::new();
    for (name, h) in listed {
        let Some(o) = objects.get(&h) else { continue };
        let style = match &o.kentos {
            Some(s) => s.clone(),
            None if followed.contains(&h) => {
                let id = format!("dxf-table-{}", out.styles.len() + 1);
                let (s, lost) = from_groups(&o.groups, id, name.trim(), metres, plot);
                report.decide(
                    Style,
                    "table_style_from_acad_table",
                    &s.name,
                    "ACAD_TABLE nesneleri bloklarının çizgi ve yazılarıyla alındı, KentOS tablosu olmadı; izledikleri bu tablo stili projenin tablo stili olarak alındı",
                    0,
                );
                if !lost.is_empty() {
                    report.decide(Style, "table_style_lossy", &s.name, &format!("{}; KentOS bunları taşımaz", lost.join(", ")), 0);
                }
                s
            }
            None => {
                unused.push(name);
                continue;
            }
        };
        for cell in [&style.title, &style.header, &style.data].into_iter().flatten() {
            if let Some(t) = &cell.text_style
                && !out.text_styles.contains(&t.to_uppercase())
            {
                out.text_styles.push(t.to_uppercase());
            }
        }
        out.styles.push(style);
    }
    if !unused.is_empty() {
        report.decide_quiet(
            Style,
            "table_style_unused_left_out",
            "Stil",
            &format!("{} tablo stilini hiçbir tablo izlemiyor; alınmadı: {}", unused.len(), unused.join(", ")),
            0,
        );
    }
    Ok(out)
}

/// Another program's TABLESTYLE as a project table style, and what of it KentOS cannot keep.
fn from_groups(
    groups: &[(i32, String)],
    id: String,
    name: &str,
    metres: &dyn Fn(f64) -> f64,
    plot: f64,
) -> (TableStyleDef, Vec<String>) {
    let mut lost: Vec<String> = Vec::new();
    let mm = |v: Option<f64>| v.filter(|v| v.is_finite()).map(|v| metres(v) * 1000.0 / plot);
    let at = groups.iter().position(|g| g.0 == 100 && g.1 == "AcDbTableStyle").map_or(0, |i| i + 1);
    let body = &groups[at..];
    let first_cell = body.iter().position(|g| g.0 == 7).unwrap_or(body.len());
    let (head, cells) = body.split_at(first_cell);
    let real = |list: &[(i32, String)], code: i32| list.iter().find(|g| g.0 == code).and_then(|g| parse_real(&g.1));
    let int = |list: &[(i32, String)], code: i32| list.iter().find(|g| g.0 == code).and_then(|g| parse_int(&g.1));
    if int(head, 70) == Some(1) {
        lost.push("tablonun yukarı akışı (aşağı akar)".to_owned());
    }
    // 280 is the version first, then the title's suppressed flag.
    let flags: Vec<i64> = head.iter().filter(|g| g.0 == 280).filter_map(|g| parse_int(&g.1)).collect();
    if flags.get(1) == Some(&1) {
        lost.push("başlığın gizlenmesi".to_owned());
    }
    if int(head, 281) == Some(1) {
        lost.push("sütun başının gizlenmesi".to_owned());
    }
    let keep = |v: Option<f64>, positive: bool| {
        v.filter(|v| if positive { *v > 0.0 } else { *v >= 0.0 } && *v <= MOST)
    };
    let (h_margin, v_margin) = (keep(mm(real(head, 40)), false), keep(mm(real(head, 41)), false));
    let mut blocks: Vec<&[(i32, String)]> = Vec::new();
    let mut start = 0;
    for (i, g) in cells.iter().enumerate().skip(1) {
        if g.0 == 7 {
            blocks.push(&cells[start..i]);
            start = i;
        }
    }
    if !cells.is_empty() {
        blocks.push(&cells[start..]);
    }
    let mut kinds: [Option<TableCellStyleDef>; 3] = [None, None, None];
    for (k, b) in blocks.iter().take(3).enumerate() {
        let what = ["veri", "başlık", "sütun başı"][k];
        let text_style = b.iter().find(|g| g.0 == 7).map(|g| g.1.trim().to_owned()).filter(|n| !n.is_empty() && !styles::is_standard(n));
        let height = keep(mm(real(b, 140)), true);
        let align = int(b, 170).filter(|a| (1..=9).contains(a)).map(|a| {
            if !(4..=6).contains(&a) {
                lost.push(format!("{what} hücresinin üst ya da alt hizası (ortalı çizilir)"));
            }
            match (a - 1) % 3 {
                0 => TableAlign::Left,
                1 => TableAlign::Center,
                _ => TableAlign::Right,
            }
        });
        let fill_on = int(b, 283) == Some(1);
        // The borders: top, horizontal inside, bottom, left, vertical inside, right; KentOS keeps one look a cell
        // style, its bottom border's (the line under its row) when it is shown.
        let shown: Vec<usize> = (0..6).filter(|i| int(b, 284 + *i as i32) != Some(0)).collect();
        let weights: Vec<Option<i64>> = (0..6).map(|i| int(b, 274 + i)).collect();
        let colours: Vec<Option<i64>> = (0..6).map(|i| int(b, 64 + i)).collect();
        let pick = shown.iter().copied().find(|i| *i == 2).or(shown.first().copied());
        if shown.len() < 6 {
            lost.push(format!("{what} hücresinin gizli kenarları"));
        }
        if shown.iter().any(|i| weights[*i] != weights[shown[0]] || colours[*i] != colours[shown[0]]) {
            lost.push(format!("{what} hücresinin kenarlarının farklı kalınlık ve renkleri (alt kenarınki alındı)"));
        }
        let border_weight = pick.and_then(|i| weights[i]).filter(|w| *w >= 0).map(|w| w as f64 / 100.0);
        let border_color = pick.and_then(|i| colour(colours[i]));
        let cell = TableCellStyleDef {
            text_style,
            height,
            align,
            color: colour(int(b, 62)),
            fill: fill_on.then(|| fill(int(b, 63))).flatten(),
            horizontal_margin: h_margin,
            vertical_margin: v_margin,
            border_weight,
            border_color,
        };
        if cell != TableCellStyleDef::default() {
            kinds[k] = Some(cell);
        }
    }
    let [data, title, header] = kinds;
    lost.sort();
    lost.dedup();
    let name = if name.is_empty() { "Tablo stili".to_owned() } else { name.to_owned() };
    (TableStyleDef { id, name, title, header, data }, lost)
}
