//! Ruhsat takımı (docs/adr/3037, M37): a phase's architectural set of sheets made from the MO sheet
//! (`sys:mimari-ruhsat`) in the order the Mimarlar Odası's presentation standard gives: the project's information sheet
//! (Planlı Alanlar İmar Yönetmeliği 57(10)'s table and the set's live list), the site plan, then the storeys' plans from
//! the lowest up. Each sheet is numbered on its phase's code (`UP-01` …) and answers the digital project regulation's
//! file name fields (RG 5.8.2026/33331 EK-2: its type, kind, storey or place and sequence); the title block reads the
//! rest from the drawing (its variables and what it says of itself), never asked again sheet by sheet.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use crate::model::{Item, Orientation, Paper, SheetBook, VarValue};
use crate::ops::{AddAssets, AddMaster, AddSheet, Op};
use crate::template::{
    AssetWithBytes, InstanceIds, InstanceOptions, PaperChoice, Template, VariableValue,
};
use crate::{GroundPoint, ItemKind, MapLevel, Result, SheetError};

/// The template a set is made from.
pub const TEMPLATE: &str = "sys:mimari-ruhsat";

/// A project phase's set (MO, Mimarlık Hizmetleri Şartnamesi 7 and 9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Ön proje: ÖP, 1/100.
    Preliminary,
    /// Ruhsat: the application project the permit is given on (MO Yön. 18), UP at 1/100 (PAİY 57(27) lets a
    /// municipality ask 1/100 drawn with 1/50's technique).
    #[default]
    Permit,
    /// Uygulama: UP, 1/50.
    Application,
}

impl Phase {
    pub const ALL: [Phase; 3] = [Phase::Preliminary, Phase::Permit, Phase::Application];

    /// MO's sheet code.
    pub fn code(self) -> &'static str {
        match self {
            Phase::Preliminary => "ÖP",
            Phase::Permit | Phase::Application => "UP",
        }
    }

    /// The plans' scale.
    pub fn scale(self) -> u32 {
        match self {
            Phase::Application => 50,
            Phase::Preliminary | Phase::Permit => 100,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Phase::Preliminary => "Ön proje",
            Phase::Permit => "Ruhsat",
            Phase::Application => "Uygulama",
        }
    }

    /// Where the set's sheets stand in the drawing set.
    pub fn subset(self) -> Vec<String> {
        vec!["Mimari".into(), format!("{} takımı", self.name())]
    }
}

/// What a sheet of the set shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    /// The first sheet: PAİY 57(10)'s table and the set's list; no map.
    Info,
    /// Vaziyet planı: the ground storey and the parcel.
    Site,
    /// A storey's plan.
    Plan,
}

/// One sheet of the set.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub kind: PageKind,
    pub title: String,
    /// The storey its map draws (with the objects on none, docs/adr/0716).
    pub level: Option<String>,
    pub center: Option<GroundPoint>,
    pub scale: Option<u32>,
    /// EK-2's file type (Tablo 2.12), kind (2.13) and storey or place (2.14).
    pub file_type: &'static str,
    pub file_kind: &'static str,
    pub place: String,
}

impl Page {
    pub fn info() -> Page {
        Page {
            kind: PageKind::Info,
            title: "Proje bilgileri".into(),
            level: None,
            center: None,
            scale: None,
            file_type: "GN",
            file_kind: "MMC",
            place: "NNN".into(),
        }
    }

    pub fn site(level: Option<String>, center: Option<GroundPoint>, scale: u32) -> Page {
        Page {
            kind: PageKind::Site,
            title: "Vaziyet planı".into(),
            level,
            center,
            scale: Some(scale),
            file_type: "PL",
            file_kind: "VAZ",
            place: "NNN".into(),
        }
    }

    pub fn plan(
        level: &str,
        name: &str,
        place: &str,
        center: Option<GroundPoint>,
        scale: u32,
    ) -> Page {
        Page {
            kind: PageKind::Plan,
            title: format!("{name} planı"),
            level: Some(level.to_owned()),
            center,
            scale: Some(scale),
            file_type: "PL",
            file_kind: "MMC",
            place: place.to_owned(),
        }
    }
}

/// The set's publication's file names (EK-2 Tablo 2.1): the project's code and building, discipline MM, no item (XXXX),
/// the sheet's type, kind, storey or place and sequence, the revision; `file.publish.sheets`'s `{@ad}` reads them.
pub const FILE_NAMING: &str = "{@proje_kodu}_{@bina_kodu}_MM_XXXX_{@dosya_turu}_{@dosya_tipi}_{@kat_konum}_{@sira_no}_{@revizyon_no}";

/// The standard notes a new set's drawing gets.
pub const NOTES: &str = "1. Ölçüler santimetre, kotlar metre cinsindendir.\n2. Proje; statik, mekanik ve elektrik \
                         projeleriyle birlikte değerlendirilir.\n3. Uygulamada ölçüler yerinde doğrulanır; çizim \
                         üzerinden ölçü alınmaz.";

/// The drawing's variables a set's title block and information table read (docs/adr/3037): name, label and the value a
/// drawing that lacks it gets; none: it must be given (the sheet says it is missing).
pub const PARTICULARS: [(&str, &str, Option<&str>); 27] = [
    ("proje_adi", "İşin adı", None),
    ("yapi_sahibi", "Yapı sahibi", None),
    ("il", "İl", None),
    ("ilce", "İlçe", None),
    ("mahalle", "Mahalle", None),
    ("pafta", "Kadastro paftası", Some("—")),
    ("ada", "Ada", None),
    ("parsel", "Parsel", None),
    ("tapu_kaydi", "Tapu kaydı (cilt, sayfa)", Some("—")),
    ("mevcut_yapilar", "Arsadaki mevcut yapılar", Some("Yok")),
    ("tasiyici_sistem", "Taşıyıcı sistem", None),
    ("muteahhit", "Yapı müteahhidi", Some("—")),
    ("mimar", "Mimar (müellif) adı soyadı", None),
    ("mimar_unvan", "Mimarın unvanı", Some("Mimar")),
    ("sicil_no", "Mimarlar Odası sicil no", None),
    ("btb_no", "Büro tescil belgesi (BTB) no", None),
    ("buro_adres", "Büronun adresi", None),
    ("tasarlayan", "Tasarlayan", Some("—")),
    ("cizen", "Çizen", Some("—")),
    ("kontrol_eden", "Kontrol eden", Some("—")),
    ("denetciler", "Denetçi mimar ve mühendisler", Some("—")),
    ("yapi_denetim", "Yapı denetim kuruluşu", Some("—")),
    ("proje_no", "Proje no (büronun)", Some("—")),
    ("blok_no", "Blok no (tek blokta —)", Some("—")),
    (
        "proje_kodu",
        "Proje kodu (idareden; bilinmiyorsa 000000)",
        Some("000000"),
    ),
    ("bina_kodu", "Bina kodu (tek binada 00)", Some("00")),
    ("revizyon_no", "Revizyon no (revizyonsuz 000)", Some("000")),
];

/// EK-2 Tablo 2.14: each storey's code by its place: the one at ±0.00 (else the lowest above it) K00, those above K01,
/// K02 …, those below B01, B02 … from the nearest; `levels` are (id, elevation).
pub fn level_codes(levels: &[(String, f64)]) -> Vec<(String, String)> {
    let mut above: Vec<&(String, f64)> = levels.iter().filter(|(_, e)| *e > -1e-6).collect();
    let mut below: Vec<&(String, f64)> = levels.iter().filter(|(_, e)| *e <= -1e-6).collect();
    above.sort_by(|a, b| a.1.total_cmp(&b.1));
    below.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, (id, _)) in above.iter().enumerate() {
        out.push((id.clone(), format!("K{k:02}")));
    }
    for (k, (id, _)) in below.iter().enumerate() {
        out.push((id.clone(), format!("B{:02}", k + 1)));
    }
    out
}

/// The map's room on a landscape paper of the MO sheet, mm: its width less the margins and the 180 mm column, its height
/// less the margins (the template's base layout).
fn map_room(paper: Paper) -> Option<[f64; 2]> {
    let (w, h) = match paper {
        Paper::A3 => (420.0, 297.0),
        Paper::A2 => (594.0, 420.0),
        Paper::A1 => (841.0, 594.0),
        Paper::A0 => (1189.0, 841.0),
        _ => return None,
    };
    Some([w - 230.0, h - 40.0])
}

/// The papers a set is put on, the smallest first.
pub const PAPERS: [Paper; 4] = [Paper::A3, Paper::A2, Paper::A1, Paper::A0];

/// The smallest landscape A paper (A3 to A0) whose map holds every one of `sizes` (paper mm, a tenth to spare); none
/// when A0's does not.
pub fn paper_for(sizes: &[[f64; 2]]) -> Option<PaperChoice> {
    PAPERS
        .into_iter()
        .find(|p| {
            map_room(*p)
                .is_some_and(|[w, h]| sizes.iter().all(|s| s[0] * 1.1 <= w && s[1] * 1.1 <= h))
        })
        .map(|paper| PaperChoice {
            paper,
            orientation: Orientation::Landscape,
        })
}

/// One sheet made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Made {
    pub sheet: String,
    pub number: String,
    pub title: String,
}

/// The operations, in order, and what they make.
#[derive(Clone, Debug)]
pub struct PermitSheets {
    pub ops: Vec<Op>,
    pub pictures: Vec<AssetWithBytes>,
    pub made: Vec<Made>,
}

/// The first sheet's items (docs/adr/3037): PAİY 57(10)'s table (with 57(3)'s floor area ratio, footprint and units) and
/// the set's live list, in the map's place on `paper`.
fn info_items(paper: &PaperChoice, subset: &[String], sheets: usize) -> Result<Vec<Item>> {
    let [w, h] = map_room(paper.paper).unwrap_or([190.0, 257.0]);
    let width = w.min(200.0);
    let table_h = (h * 0.62).min(175.0);
    let list_top = 20.0 + table_h + 8.0;
    // The list's rows at 7 mm with its title and heading, within what is left.
    let list_h = ((sheets as f64 + 2.0) * 7.0).clamp(30.0, (h - table_h - 8.0).max(30.0));
    let um = |mm: f64| (mm * 1000.0).round() as i64;
    let row = |label: &str, value: &str| {
        json!({"height": 1, "cells": [
            {"width": 2, "value": label, "style": {"size": 2000, "weight": 600}},
            {"width": 3, "value": value}
        ]})
    };
    let rows = vec![
        json!({"height": 1, "cells": [{"value": "İLK PAFTA BİLGİ TABLOSU (PAİY 57/10)", "align": "center",
            "style": {"size": 2600, "weight": 700}, "fill": "#ededed"}]}),
        row(
            "Arsanın yeri (il / ilçe / mahalle)",
            "[% @il %] / [% @ilce %] / [% @mahalle %]",
        ),
        row("Tapu kaydı", "[% @tapu_kaydi %]"),
        row(
            "Pafta / ada / parsel",
            "[% @pafta %] / [% @ada %] / [% @parsel %]",
        ),
        row("Arsa alanı", "[% @arsa_alani %]"),
        row("Arsadaki mevcut yapılar", "[% @mevcut_yapilar %]"),
        row("Taşıyıcı sistem", "[% @tasiyici_sistem %]"),
        row(
            "Kat adedi",
            "[% @kat_adedi %] (bodrum [% @bodrum_kat_adedi %])",
        ),
        row(
            "Emsal / TAKS (plan)",
            "[% @emsal_orani %] / [% @taks_orani %]",
        ),
        row("Emsale konu alan", "[% @emsale_konu_alan %]"),
        row("Taban alanı", "[% @taban_alani %]"),
        row("Yapı inşaat alanı", "[% @yapi_insaat_alani %]"),
        row(
            "Toplam yapı inşaat alanı",
            "[% @toplam_yapi_insaat_alani %]",
        ),
        row("Bağımsız bölüm sayısı", "[% @bb_sayisi %]"),
        row("Kullanım amacı", "[% @kullanim_amaci %]"),
        row("Yapı sahibi", "[% @yapi_sahibi %]"),
        row("Yapı müteahhidi", "[% @muteahhit %]"),
        row(
            "Proje müellifi (mimar)",
            "[% @mimar %], [% @mimar_unvan %] · sicil [% @sicil_no %]",
        ),
        row("Denetçi mimar ve mühendisler", "[% @denetciler %]"),
        row("Yapı denetim kuruluşu", "[% @yapi_denetim %]"),
    ];
    let table: Item = serde_json::from_value(json!({
        "id": "bilgi-tablosu",
        "name": "İlk pafta bilgi tablosu",
        "frame": {"left": 30000, "top": 20000, "width": um(width), "height": um(table_h)},
        "constraints": {"h": "left", "v": "top", "relativeTo": "margins"},
        "fill": "#ffffff",
        "kind": {"type": "titleBlock", "outer": {"width": 500}, "inner": {"width": 180}, "cellPadding": 1000,
                 "valueStyle": {"size": 2200, "weight": 400}, "rows": rows}
    }))
    .map_err(|e| SheetError::new("bad_template", format!("Bilgi tablosu kurulamadı: {e}")))?;
    let list: Item = serde_json::from_value(json!({
        "id": "pafta-listesi",
        "name": "Pafta listesi",
        "frame": {"left": 30000, "top": um(list_top), "width": um(width), "height": um(list_h)},
        "constraints": {"h": "left", "v": "top", "relativeTo": "margins"},
        "kind": {
            "type": "table",
            "title": "PAFTA LİSTESİ",
            "titleStyle": {"size": 2600, "weight": 700},
            // A drawing index has its six standard columns; the view says nothing on a list of sheets.
            "columns": [
                {"heading": "Pafta no", "width": 24000, "align": "center"},
                {"heading": "Pafta adı"},
                {"heading": "Rev.", "width": 14000, "align": "center"},
                {"heading": "Durum", "width": 20000, "align": "center"},
                {"heading": "Takım", "width": 42000},
                {"heading": "", "width": 1000}
            ],
            "headerStyle": {"text": {"size": 2000, "weight": 600}, "fill": "#ededed", "padding": 800},
            "cellStyle": {"text": {"size": 2000, "weight": 400}, "padding": 800},
            "source": {"type": "drawingSet", "subset": subset}
        }
    }))
    .map_err(|e| SheetError::new("bad_template", format!("Pafta listesi kurulamadı: {e}")))?;
    Ok(vec![table, list])
}

/// `sheet` without the items `gone` names, in its other papers' layouts too.
fn drop_items(sheet: &mut crate::Sheet, gone: &dyn Fn(&String) -> bool) {
    sheet.items.retain(|i| !gone(&i.id));
    for v in sheet.variants.iter_mut() {
        v.frames.retain(|f| !gone(&f.item));
    }
    if let Some(base) = sheet.base_layout.as_mut() {
        base.frames.retain(|f| !gone(&f.item));
    }
}

/// The set's sheets of `pages` on `paper` made from `template` (the MO sheet), `ids` giving each sheet its ids from a
/// suggested sheet id; numbered on from `phase`'s code's first number no sheet has, in one change. Without `north` (a
/// project with no coordinate system) the north arrow is left out, as the CAD profile hides it (docs/adr/3037).
pub fn permit_sheets(
    book: &SheetBook,
    template: &Template,
    phase: Phase,
    paper: PaperChoice,
    pages: &[Page],
    north: bool,
    mut ids: impl FnMut(&str) -> InstanceIds,
) -> Result<PermitSheets> {
    if pages.is_empty() {
        return Err(SheetError::at(
            "permit_set_empty",
            "pages",
            "Takımda pafta yok: projede kat ya da çizim yok.",
        ));
    }
    let map_at = template
        .sheet
        .items
        .iter()
        .position(|i| matches!(i.kind, ItemKind::Map(_)))
        .ok_or_else(|| {
            SheetError::at(
                "not_fixed_map",
                "source",
                "Şablonun paftasında görünüm penceresi yok.",
            )
        })?;
    // The north arrow reads the map: the first sheet, which has none, has none either.
    let north_at = template
        .sheet
        .items
        .iter()
        .position(|i| matches!(i.kind, ItemKind::NorthArrow(_)));
    let mut used: BTreeSet<String> = book
        .sheets
        .iter()
        .filter_map(|s| s.drawing_info.as_ref())
        .map(|i| crate::expr::fold(&i.number))
        .collect();
    let mut taken: BTreeSet<String> = book.sheets.iter().map(|s| s.id.clone()).collect();
    let subset = phase.subset();
    let mut next = 1u32;
    // EK-2 2.10: the sequence counts on while every field before it is the same.
    let mut sequence: BTreeMap<(String, String, String), u32> = BTreeMap::new();
    let mut out = PermitSheets {
        ops: Vec::new(),
        pictures: Vec::new(),
        made: Vec::new(),
    };
    for page in pages {
        let number = loop {
            let n = format!("{}-{next:02}", phase.code());
            next += 1;
            if used.insert(crate::expr::fold(&n)) {
                break n;
            }
        };
        let mut sheet_id = format!("ruhsat-{}", crate::expr::fold(&number).to_lowercase());
        let mut k = 2;
        while taken.contains(&sheet_id) {
            sheet_id = format!("ruhsat-{}-{k}", crate::expr::fold(&number).to_lowercase());
            k += 1;
        }
        let ids = ids(&sheet_id);
        taken.insert(ids.sheet.clone());
        let n = sequence
            .entry((
                page.file_type.to_owned(),
                page.file_kind.to_owned(),
                page.place.clone(),
            ))
            .or_insert(0);
        *n += 1;
        let text = |v: &str| VarValue::Text(v.to_owned());
        let options = InstanceOptions {
            paper: Some(paper.clone()),
            name: Some(format!("{number} {}", page.title)),
            center: page.center,
            scale: page.scale,
            values: vec![
                VariableValue {
                    name: "dosya_turu".into(),
                    value: text(page.file_type),
                },
                VariableValue {
                    name: "dosya_tipi".into(),
                    value: text(page.file_kind),
                },
                VariableValue {
                    name: "kat_konum".into(),
                    value: text(&page.place),
                },
                VariableValue {
                    name: "sira_no".into(),
                    value: text(&format!("{:02}", *n)),
                },
                // The drawing's notes, not the template's (an unanswered question gives way, docs/adr/3037).
                VariableValue {
                    name: "notlar".into(),
                    value: VarValue::Null,
                },
            ],
        };
        let mut instance = crate::template::instantiate(template, &ids, &options)?;
        let sheet = &mut instance.sheet;
        sheet.drawing_info = Some(crate::drawing_set::DrawingInfo {
            number: number.clone(),
            title: page.title.clone(),
            revision: String::new(),
            status: String::new(),
            subset: subset.clone(),
        });
        // The template's items are the instance's by their place (`create::ids_for`).
        let map_item = ids.items.get(map_at).cloned().unwrap_or_default();
        let north_item = north_at.and_then(|k| ids.items.get(k).cloned());
        if !north || page.kind == PageKind::Info {
            drop_items(sheet, &|id: &String| Some(id) == north_item.as_ref());
        }
        match page.kind {
            PageKind::Info => {
                drop_items(sheet, &|id: &String| *id == map_item);
                // A sheet without a map has no scale: its title block says so.
                sheet.variables.push(crate::model::Variable {
                    name: "olcek".into(),
                    label: "Ölçek".into(),
                    kind: crate::model::VarKind::Text,
                    value: VarValue::Text("—".into()),
                });
                for mut item in info_items(&paper, &subset, pages.len())? {
                    item.id = format!("{}-{}", ids.sheet, item.id);
                    sheet.items.push(item);
                }
            }
            PageKind::Site | PageKind::Plan => {
                if let Some(item) = sheet.items.iter_mut().find(|i| i.id == map_item)
                    && let ItemKind::Map(m) = &mut item.kind
                {
                    if let Some(level) = &page.level {
                        m.level = Some(MapLevel {
                            id: level.clone(),
                            below: false,
                        });
                    }
                    // A set's scale is its phase's: locked, as a detail's is (docs/adr/1090 §2).
                    m.view_locked = true;
                }
            }
        }
        if !instance.assets.is_empty() {
            out.ops.push(Op::AddAssets(AddAssets {
                assets: instance.assets,
            }));
        }
        if let Some(master) = instance.master {
            out.ops.push(Op::AddMaster(AddMaster {
                master,
                index: None,
            }));
        }
        out.ops.push(Op::AddSheet(AddSheet {
            sheet: instance.sheet,
            index: None,
        }));
        for p in instance.asset_bytes {
            if !out.pictures.iter().any(|q| q.meta.sha256 == p.meta.sha256) {
                out.pictures.push(p);
            }
        }
        out.made.push(Made {
            sheet: ids.sheet,
            number,
            title: page.title.clone(),
        });
    }
    Ok(out)
}
