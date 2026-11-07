//! KGM's earthwork rows under a profile (plan/road.md Y89, docs/adr/0995). KGM's 2019 technical specification, Brükner:
//! "Denge kesimlerinin başlangıç ve bitiş kilometrelerinin diyagramda gösterilmesi gereklidir. Bunlara ait kazı, dolgu,
//! faktörlü kazı, taşıma momenti gibi hesaplar veya sonuçlarının paftada yazılması zorunludur." The June 2024 sample
//! sheets write them under the profile: HAFİR, HAFİRLİ FAK., İMLA, ARİYET, DEPO, each balance section's totals in m³
//! ("602114", "-" for none), and "DN : 0+000.00" up the rows at its balance points.
//!
//! The sections are a mass haul diagram's (docs/adr/0907): from its volume's first station to its last, cut at the
//! diagram's balance points. Each section's HAFİR is the volume's raw cut, HAFİRLİ FAK. the cut by its factor, İMLA the
//! raw fill; its net N, the factored cut less the factored fill, is DEPO where positive and ARİYET where negative (the
//! borrow brought to it or the waste taken off it: between two balance points the ordinate, sites in, returns to the
//! line). A volume row a balance point falls in is shared by length, as the ordinate is straight along it.

use kentos_commands::HaulSiteKind;
use kentos_domain::Document;

use crate::checks::Stop;

/// The rows, top down, as KGM's 2024 sheets name them.
pub const ROWS: [&str; 5] = ["HAFİR", "HAFİRLİ FAK.", "İMLA", "ARİYET", "DEPO"];

/// A balance section: its stations and its five values (m³), in the rows' order.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub from: f64,
    pub to: f64,
    pub values: [f64; 5],
}

/// The sections, the stations of the balance points DN is written at, and the stretches of cut unsuitable for fill
/// (docs/adr/0996; KGM's DKY, "dolguda kullanılamayan yarma"): their stations and their unsuitable cut (m³).
#[derive(Clone, Debug, PartialEq)]
pub struct Earthwork {
    pub sections: Vec<Section>,
    pub balance: Vec<f64>,
    pub unsuitable: Vec<[f64; 3]>,
}

/// A volume as KGM writes it: whole cubic metres, "-" for less than half of one.
pub fn volume_text(v: f64) -> String {
    if v.abs() < 0.5 { "-".to_owned() } else { format!("{v:.0}") }
}

/// The sections of the mass haul diagram `def`.
pub(crate) fn earthwork(doc: &Document, def: &kentos_commands::HaulDef) -> Result<Earthwork, Stop> {
    let (haul, _) = crate::road_hauls::read(doc, def, false)?;
    let vdef = doc.road_volumes().iter().find(|v| v.id == def.volume).ok_or_else(|| {
        crate::road_terrains::fail(crate::codes::UNKNOWN_VOLUME, format!("“{}” kimlikli kübaj hesabı projede yok.", def.volume), "haul")
    })?;
    let (report, _, alignment) = crate::road_volumes::compute_on(doc, vdef)?;
    let rows = &report.rows;
    let (Some(first), Some(last)) = (rows.first().map(|r| r.from), rows.last().map(|r| r.to)) else {
        return Ok(Earthwork { sections: Vec::new(), balance: Vec::new(), unsuitable: Vec::new() });
    };
    // The cuts: the ends and the balance points between.
    let mut cuts = vec![first];
    cuts.extend(haul.balance_points.iter().copied().filter(|s| *s > first + 1e-9 && *s < last - 1e-9));
    cuts.push(last);
    let sections = cuts
        .windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let mut v = [0.0; 4];
            for r in rows {
                let (lo, hi) = (r.from.max(a), r.to.min(b));
                if hi <= lo || r.to <= r.from {
                    continue;
                }
                let k = (hi - lo) / (r.to - r.from);
                let (cut, fill) = (r.cut.unwrap_or(0.0), r.fill.unwrap_or(0.0));
                v[0] += k * cut;
                v[1] += k * cut * r.cut_factor.unwrap_or(1.0);
                v[2] += k * fill;
                v[3] += k * fill * r.fill_factor.unwrap_or(1.0);
            }
            let net = v[1] - v[3];
            Section { from: a, to: b, values: [v[0], v[1], v[2], (-net).max(0.0), net.max(0.0)] }
        })
        .collect();
    // DN: the balance points; the start on the line when the line is the ordinate's start (0); the end when the
    // ordinate, its sites in, comes back to the line. The 2024 sheets write "DN : 0+000.00" at the road's start.
    let mut balance: Vec<f64> = haul.balance_points.iter().copied().filter(|s| *s > first + 1e-9 && *s < last - 1e-9).collect();
    if def.balance.abs() <= 1e-9 {
        balance.insert(0, first);
    }
    let placed = crate::road_hauls_network::placed(doc, def, &alignment)?;
    let sites: f64 = def
        .sites
        .iter()
        .zip(&placed.unconnected)
        .filter(|(_, off)| !**off)
        .map(|(s, _)| if s.kind == HaulSiteKind::Borrow { s.volume } else { -s.volume })
        .sum();
    let end = rows.last().map_or(0.0, |r| r.mass) + sites;
    let scale = report.cut.abs().max(report.fill.abs()).max(1.0);
    if (end - def.balance).abs() <= 1e-9 * scale {
        balance.push(last);
    }
    // DKY (docs/adr/0996): each stretch's unsuitable cut, its share of the raw cut over it, a row shared by length.
    let unsuitable = def
        .unsuitable
        .iter()
        .filter_map(|u| {
            let (a, b) = (u.from.max(first), u.to.min(last));
            (b > a).then(|| {
                let v: f64 = rows
                    .iter()
                    .filter(|r| r.to > r.from)
                    .map(|r| {
                        let (lo, hi) = (r.from.max(a), r.to.min(b));
                        if hi > lo { (hi - lo) / (r.to - r.from) * r.cut.unwrap_or(0.0) } else { 0.0 }
                    })
                    .sum();
                [a, b, v * u.share]
            })
        })
        .collect();
    Ok(Earthwork { sections, balance, unsuitable })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each case's sections as the independent reference makes them (scripts/fixtures/road_earthwork_band_cases.py, on
    /// the mass haul cases' drawing): every value within a thousandth of a cubic metre, every station within a
    /// micrometre, the same balance points.
    #[test]
    fn sections_are_the_references() {
        let all: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/road/v1/earthwork-bands.json")).expect("json");
        for case in all["cases"].as_array().expect("cases") {
            let setup = if case["setup"].is_null() { &all["setup"] } else { &case["setup"] };
            let doc = Document::from_snapshot_v2(serde_json::from_value(setup.clone()).expect("setup")).expect("opens");
            let name = case["haul"].as_str().unwrap();
            let def = doc.road_hauls().iter().find(|h| h.id == name || h.name == name).expect("haul").clone();
            let got = earthwork(&doc, &def).unwrap_or_else(|_| panic!("sections of {name}"));
            let want = &case["earthwork"];
            let near = |a: f64, b: &serde_json::Value, tol: f64| (a - b.as_f64().unwrap()).abs() < tol;
            let wb = want["balance"].as_array().unwrap();
            assert_eq!(got.balance.len(), wb.len(), "{}: {:?}", case["name"], got.balance);
            assert!(got.balance.iter().zip(wb).all(|(a, b)| near(*a, b, 1e-6)), "{}: {:?}", case["name"], got.balance);
            let wu = want["unsuitable"].as_array().unwrap();
            assert_eq!(got.unsuitable.len(), wu.len(), "{}", case["name"]);
            for (g, w) in got.unsuitable.iter().zip(wu) {
                let w = w.as_array().unwrap();
                assert!(near(g[0], &w[0], 1e-6) && near(g[1], &w[1], 1e-6) && near(g[2], &w[2], 1e-3), "{}: {g:?}", case["name"]);
            }
            let ws = want["sections"].as_array().unwrap();
            assert_eq!(got.sections.len(), ws.len(), "{}", case["name"]);
            for (g, w) in got.sections.iter().zip(ws) {
                assert!(near(g.from, &w["from"], 1e-6) && near(g.to, &w["to"], 1e-6), "{}: {g:?}", case["name"]);
                for (a, b) in g.values.iter().zip(w["values"].as_array().unwrap()) {
                    assert!(near(*a, b, 1e-3), "{}: {g:?} vs {w}", case["name"]);
                }
            }
        }
    }

    #[test]
    fn volumes_as_kgm_writes_them() {
        assert_eq!(volume_text(602_114.3), "602114");
        assert_eq!(volume_text(0.49), "-");
        assert_eq!(volume_text(0.0), "-");
        assert_eq!(volume_text(2229.5), "2230");
    }
}
