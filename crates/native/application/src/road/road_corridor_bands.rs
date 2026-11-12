//! KGM's slope and platform rows over a profile (plan/road.md Y83, docs/adr/0989): the June 2024 sample sheets draw
//! between the plan and the profile SOL DOL.ŞEVİ, SOL YAR.ŞEVİ, PLATFORM GEN., ÜST YAPI KAL., SAĞ DOL.ŞEVİ and SAĞ
//! YAR.ŞEVİ, each a run of cells along the km: a side's fill slope where it is in fill ("3/1") and "-" elsewhere, its
//! cut slope where it is in cut, the platform's width ("12.00 m.") and the pavement's thickness ("0.45 m.").
//!
//! The values are read off a corridor's crosses (its stations and the sections built there, docs/adr/0905): a cross's
//! side in cut or fill, its region's template (its transition counted) for the slopes and the widths, the region's
//! template's pavement for the thickness. A cell runs between two crosses whose values differ at their middle: the
//! corridor's own stations bound where a side turns from cut to fill (refined where it is asked to, docs/adr/0939).

use kentos_commands::CorridorDef;
use kentos_domain::Document;
use kentos_road::section::template::{Component, Kind, Side};

use crate::checks::Stop;

/// The rows, top down, as KGM's 2024 sheets name them.
pub const ROWS: [&str; 6] = ["SOL DOL.ŞEVİ", "SOL YAR.ŞEVİ", "PLATFORM GEN.", "ÜST YAPI KAL.", "SAĞ DOL.ŞEVİ", "SAĞ YAR.ŞEVİ"];

/// A run of one value along the stations.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub from: f64,
    pub to: f64,
    pub text: String,
}

/// A slope as KGM writes it, horizontal over vertical in small whole numbers (1.5 is "3/2", 3 is "3/1"); one that does
/// not fall on a denominator up to 10, to two decimals over one ("1.75/1").
pub fn slope_text(h_per_v: f64) -> String {
    for d in 1..=10u32 {
        let n = h_per_v * f64::from(d);
        if (n - n.round()).abs() < 1e-9 && n.round() >= 1.0 {
            return format!("{}/{d}", n.round() as i64);
        }
    }
    format!("{h_per_v:.2}/1")
}

fn width(side: &Side) -> f64 {
    side.components
        .iter()
        .map(|c| match *c {
            Component::Lane { width, .. } | Component::Shoulder { width, .. } => width,
        })
        .sum()
}

/// The six rows' cells over stations `s0`–`s1` of a corridor built now.
pub(crate) fn rows(doc: &Document, def: &CorridorDef, s0: f64, s1: f64) -> Result<[Vec<Cell>; 6], Stop> {
    let made = crate::road_corridors::make(doc, def, false)?;
    let thickness = |ri: usize| {
        let id = &def.regions.get(ri)?.template;
        let t = doc.road_templates().iter().find(|t| &t.id == id)?;
        t.pavement.as_ref().map(|p| p.layers.iter().map(|l| l.thickness).sum::<f64>())
    };
    // Each cross's six values, in station order.
    let mut at: Vec<(f64, [String; 6])> = Vec::new();
    for (ri, region) in made.regions.iter().enumerate() {
        for cross in &region.crosses {
            let s = cross.line.station;
            let Some(section) = &cross.section else { continue };
            let t = made.template_at(ri, s);
            let fill = |kind: Kind, side: &Side| if kind == Kind::Fill { slope_text(side.fill) } else { "-".to_owned() };
            let cut = |kind: Kind, side: &Side| if kind == Kind::Cut { slope_text(side.cut) } else { "-".to_owned() };
            let pavement = thickness(ri).map_or_else(|| "-".to_owned(), |v| format!("{v:.2} m."));
            at.push((
                s,
                [
                    fill(section.left.kind, &t.left),
                    cut(section.left.kind, &t.left),
                    format!("{:.2} m.", width(&t.left) + width(&t.right)),
                    pavement,
                    fill(section.right.kind, &t.right),
                    cut(section.right.kind, &t.right),
                ],
            ));
        }
    }
    at.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: [Vec<Cell>; 6] = Default::default();
    // The crosses in the view, and the one just before it (its values hold up to the middle toward the next).
    let first = at.iter().rposition(|(s, _)| *s <= s0 + 1e-9).unwrap_or(0);
    let shown: Vec<&(f64, [String; 6])> = at.iter().skip(first).take_while(|(s, _)| *s < s1 + 1e-9).collect();
    for (k, row) in out.iter_mut().enumerate() {
        for (i, (s, v)) in shown.iter().enumerate() {
            let from = if i == 0 { s0 } else { 0.5 * (shown[i - 1].0 + s) };
            let from = from.max(s0);
            match row.last_mut() {
                Some(c) if c.text == v[k] => {}
                // A change before the view's start: the first cell takes the later value.
                Some(c) if from <= c.from + 1e-9 => c.text = v[k].clone(),
                Some(c) => {
                    c.to = from;
                    row.push(Cell { from, to: s1, text: v[k].clone() });
                }
                None => row.push(Cell { from: s0, to: s1, text: v[k].clone() }),
            }
        }
        if let Some(c) = row.last_mut() {
            c.to = s1;
        }
    }
    // Off the corridor's crosses the rows stay empty: before its first, after its last.
    if let (Some(lo), Some(hi)) = (shown.first().map(|x| x.0), shown.last().map(|x| x.0)) {
        for row in &mut out {
            if lo > s0 + 1e-9 {
                if let Some(c) = row.first_mut() {
                    c.from = lo;
                }
                row.insert(0, Cell { from: s0, to: lo, text: String::new() });
            }
            if hi < s1 - 1e-9 {
                if let Some(c) = row.last_mut() {
                    c.to = hi;
                }
                row.push(Cell { from: hi, to: s1, text: String::new() });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each case's rows as the independent reference makes them (scripts/fixtures/road_corridor_band_cases.py, on the
    /// corridor cases' drawing): every cell's words the same, its ends within a micrometre.
    #[test]
    fn rows_are_the_references() {
        let all: serde_json::Value =
            serde_json::from_str(include_str!("../../../../../fixtures/road/v1/corridor-bands.json")).expect("json");
        for case in all["cases"].as_array().expect("cases") {
            let setup = if case["setup"].is_null() { &all["setup"] } else { &case["setup"] };
            let doc = Document::from_snapshot_v2(serde_json::from_value(setup.clone()).expect("setup")).expect("opens");
            let name = case["corridor"].as_str().unwrap();
            let def = doc.road_corridors().iter().find(|c| c.id == name || c.name == name).expect("corridor").clone();
            let profile = doc.road_profiles().iter().find(|p| p.id == case["profile"].as_str().unwrap()).expect("profile");
            let (s0, s1) = (profile.pvis[0].station, profile.pvis.last().unwrap().station);
            let got = rows(&doc, &def, s0, s1).unwrap_or_else(|_| panic!("rows of {name}"));
            for (k, (cells, want)) in got.iter().zip(case["rows"].as_array().unwrap()).enumerate() {
                let want = want.as_array().unwrap();
                assert_eq!(cells.len(), want.len(), "{} · {}: {cells:?}", case["name"], ROWS[k]);
                for (c, w) in cells.iter().zip(want) {
                    assert_eq!(c.text, w["text"].as_str().unwrap(), "{} · {}", case["name"], ROWS[k]);
                    let (f, t) = (w["from"].as_f64().unwrap(), w["to"].as_f64().unwrap());
                    assert!((c.from - f).abs() < 1e-6 && (c.to - t).abs() < 1e-6, "{} · {}: {c:?} vs {w}", case["name"], ROWS[k]);
                }
            }
        }
    }

    #[test]
    fn slopes_as_kgm_writes_them() {
        assert_eq!(slope_text(1.5), "3/2");
        assert_eq!(slope_text(3.0), "3/1");
        assert_eq!(slope_text(1.0), "1/1");
        assert_eq!(slope_text(0.5), "1/2");
        assert_eq!(slope_text(2.0 / 3.0), "2/3");
        assert_eq!(slope_text(1.75), "7/4");
        assert_eq!(slope_text(1.234), "1.23/1");
    }
}
