//! Templates, `kentos.sheet.template/1` (design §12): a sheet with its master
//! page, its pictures' bytes and the questions it asks when used. A template's
//! maps keep a scale, never a place. System templates come with the program
//! (`templates/*.json`, the same files on every platform); a user's are made
//! from a sheet (`extract`) and kept by the host.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use kentos_contracts::{ProjectType, Workspace};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::error::{Result, SheetError};
use crate::kinds::*;
use crate::model::*;
use crate::paper;
use crate::units::SizeUm;
use crate::validate::{normalize, validate_book};

pub const TEMPLATE_SCHEMA: &str = "kentos.sheet.template/1";
/// The schema family; a later version's number follows the slash.
const TEMPLATE_FAMILY: &str = "kentos.sheet.template/";
/// The version this build writes and reads.
pub const TEMPLATE_VERSION: u32 = 1;

/// A paper a template suits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PaperChoice {
    pub paper: Paper,
    pub orientation: Orientation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TemplateMeta {
    /// `sys:…` for the program's own (read-only); anything else for a user's.
    pub id: String,
    pub revision: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The gallery's kind filter: “genel”, “kadastro”, “imar”, “teknik”, “mimari”, “tematik”, “atlas”, “rapor” …
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// The papers it suits, the recommended one first.
    #[serde(default)]
    pub papers: Vec<PaperChoice>,
    /// The work modes it is for (design §11a); both cad and gis: common.
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    /// The project types it is for; empty: any.
    #[serde(default)]
    pub project_types: Vec<ProjectType>,
    /// RFC 3339.
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub author: String,
}

/// A picture with its bytes (base64), so a template never prints an empty box on another machine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AssetWithBytes {
    pub meta: AssetMeta,
    /// The file's bytes, base64 (RFC 4648, with padding).
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Template {
    /// `kentos.sheet.template/1`.
    pub schema: String,
    pub meta: TemplateMeta,
    /// Its maps have a scale and no centre; its own `variables` are empty (the questions are below).
    pub sheet: Sheet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub master: Option<Master>,
    #[serde(default)]
    pub assets: Vec<AssetWithBytes>,
    /// What using it asks (ada, parsel, mahalle …), with default values.
    #[serde(default)]
    pub variables: Vec<Variable>,
}

/// The ids a new sheet from a template gets: the host makes them (the core never does).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct InstanceIds {
    pub sheet: SheetId,
    /// For the template's master page, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub master: Option<MasterId>,
    /// One per item of the template's sheet, in order.
    #[serde(default)]
    pub items: Vec<ItemId>,
    /// One per item of the template's master page, in order.
    #[serde(default)]
    pub master_items: Vec<ItemId>,
}

/// An answer to one of the template's questions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct VariableValue {
    pub name: String,
    pub value: VarValue,
}

/// How a template is used.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct InstanceOptions {
    /// Another paper: the items follow their constraints there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub paper: Option<PaperChoice>,
    /// The sheet's name; none: the template's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Where the maps look (the drawing area's centre, say).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub center: Option<GroundPoint>,
    /// The maps' scale (the project's plot scale); none: the template's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scale: Option<u32>,
    #[serde(default)]
    pub values: Vec<VariableValue>,
}

/// A sheet made from a template, and what the book gets with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Instance {
    pub sheet: Sheet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub master: Option<Master>,
    /// The pictures' metadata, for the book.
    pub assets: Vec<AssetMeta>,
    /// Their bytes, for the host's asset store.
    pub asset_bytes: Vec<AssetWithBytes>,
}

// ── base64 (RFC 4648) ────────────────────────────────────────────────────

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18u32, 12, 6, 0].iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(B64[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let t: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !t.len().is_multiple_of(4) {
        return None;
    }
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let mut out = Vec::with_capacity(t.len() / 4 * 3);
    let quads = t.len() / 4;
    for (q, chunk) in t.chunks(4).enumerate() {
        let pad = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && q + 1 != quads) {
            return None;
        }
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            let v = if i >= 4 - pad { 0 } else { val(c)? };
            n = (n << 6) | v;
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

/// SHA-256 of `bytes` as 64 lowercase hex digits: an asset's name.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let d = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in d {
        s.push(char::from(b"0123456789abcdef"[usize::from(b >> 4)]));
        s.push(char::from(b"0123456789abcdef"[usize::from(b & 15)]));
    }
    s
}

// ── Reading, migrating, validating ───────────────────────────────────────

/// A template's JSON brought to this build's version. Version 1 is the
/// first, so there is nothing to change yet; a later version is refused
/// rather than guessed at, and so is an unknown schema.
pub fn migrate(mut v: serde_json::Value) -> Result<serde_json::Value> {
    let schema = v
        .get("schema")
        .and_then(|s| s.as_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            SheetError::at(
                "unknown_schema",
                "schema",
                "Şablonun biçimi (schema) yazılı değil.",
            )
        })?;
    let version = schema
        .strip_prefix(TEMPLATE_FAMILY)
        .and_then(|n| n.parse::<u32>().ok())
        .ok_or_else(|| {
            SheetError::at(
                "unknown_schema",
                "schema",
                format!("“{schema}” bir pafta şablonu değil (beklenen: {TEMPLATE_SCHEMA})."),
            )
        })?;
    if version > TEMPLATE_VERSION {
        return Err(SheetError::at(
            "newer_schema",
            "schema",
            format!(
                "Şablon daha yeni bir sürümle yazılmış ({schema}); bu sürüm {TEMPLATE_SCHEMA} okur. Uygulamayı güncelleyin."
            ),
        ));
    }
    // Each step takes version n to n + 1; none exists before 1.
    let steps: &[fn(&mut serde_json::Value)] = &[];
    for step in steps.iter().skip(version.saturating_sub(1) as usize) {
        step(&mut v);
    }
    if let Some(o) = v.as_object_mut() {
        o.insert(
            "schema".to_owned(),
            serde_json::Value::String(TEMPLATE_SCHEMA.to_owned()),
        );
    }
    Ok(v)
}

/// The pictures a list of items uses (pictures and title block cells), in order, once each.
pub fn used_assets(items: &[Item]) -> Vec<String> {
    fn cells(rows: &[TitleRow], out: &mut Vec<String>) {
        for r in rows {
            for c in &r.cells {
                if let Some(p) = &c.picture {
                    out.push(p.clone());
                }
                cells(&c.rows, out);
            }
        }
    }
    let mut out = Vec::new();
    for it in items {
        match &it.kind {
            ItemKind::Picture(p) => {
                if let Some(a) = &p.asset {
                    out.push(a.clone());
                }
            }
            ItemKind::TitleBlock(tb) => cells(&tb.rows, &mut out),
            _ => {}
        }
    }
    let mut seen = BTreeSet::new();
    out.retain(|a| seen.insert(a.clone()));
    out
}

/// The book a template stands for (its sheet, master and pictures), for the book's rules.
fn as_book(t: &Template) -> SheetBook {
    SheetBook {
        schema: BOOK_SCHEMA.to_owned(),
        sheets: vec![t.sheet.clone()],
        masters: t.master.iter().cloned().collect(),
        assets: t.assets.iter().map(|a| a.meta.clone()).collect(),
        variables: Vec::new(),
    }
}

/// Every rule of a template. `system`: the program's own (`sys:` ids are reserved for them).
pub fn validate_template(t: &Template, system: bool) -> Result<()> {
    if t.schema != TEMPLATE_SCHEMA {
        return Err(SheetError::at(
            "unknown_schema",
            "schema",
            format!(
                "Şablonun biçimi “{}”; beklenen {TEMPLATE_SCHEMA}.",
                t.schema
            ),
        ));
    }
    let m = &t.meta;
    if !crate::validate::valid_id(&m.id) {
        return Err(SheetError::at(
            "bad_id",
            "meta.id",
            "Şablon kimliği 1 ile 128 karakter arasında olmalı.",
        ));
    }
    if m.id.starts_with("sys:") != system {
        return Err(SheetError::at(
            "reserved_id",
            "meta.id",
            if system {
                "Sistem şablonunun kimliği “sys:” ile başlamalı."
            } else {
                "“sys:” ile başlayan kimlikler uygulamanın kendi şablonlarınındır; başka bir kimlik verin."
            },
        ));
    }
    if m.name.trim().is_empty() || m.name.chars().count() > 120 {
        return Err(SheetError::at(
            "bad_name",
            "meta.name",
            "Şablonun adı boş olamaz ve en çok 120 karakter olabilir.",
        ));
    }
    if m.revision == 0 {
        return Err(SheetError::at(
            "out_of_range",
            "meta.revision",
            "Şablonun revizyonu 1'den başlar.",
        ));
    }
    if m.tags.len() > 20
        || m.tags
            .iter()
            .any(|t| t.trim().is_empty() || t.chars().count() > 32)
    {
        return Err(SheetError::at(
            "out_of_range",
            "meta.tags",
            "Şablonun en çok 20 etiketi olabilir; her biri 1 ile 32 karakter.",
        ));
    }
    if m.workspaces.is_empty() {
        return Err(SheetError::at(
            "out_of_range",
            "meta.workspaces",
            "Şablonun hangi kip için olduğu yazılmalı (ortak şablon için cad ve gis).",
        ));
    }
    if !t.sheet.variables.is_empty() {
        return Err(SheetError::at(
            "bad_template",
            "sheet.variables",
            "Şablonun soruları paftada değil, şablonun “variables” listesinde durur.",
        ));
    }
    if t.sheet.origin.is_some() {
        return Err(SheetError::at(
            "bad_template",
            "sheet.origin",
            "Şablonun kendi kaynağı olmaz.",
        ));
    }
    if let Some(master) = &t.master {
        if t.sheet.master.as_deref() != Some(master.id.as_str()) {
            return Err(SheetError::at(
                "bad_template",
                "sheet.master",
                "Şablonun ana sayfası paftasının ana sayfası olmalı.",
            ));
        }
    } else if t.sheet.master.is_some() {
        return Err(SheetError::at(
            "unknown_master",
            "sheet.master",
            "Şablon ana sayfasını taşımıyor.",
        ));
    }
    // A template keeps a scale, never a place.
    let items = t
        .sheet
        .items
        .iter()
        .chain(t.master.iter().flat_map(|m| m.items.iter()));
    for it in items {
        if let ItemKind::Map(map) = &it.kind
            && let MapView::Fixed(f) = &map.view
            && f.center.is_some()
        {
            return Err(SheetError::at(
                "template_has_place",
                format!("item {}", it.id),
                "Şablondaki harita bir yer taşımaz, yalnız ölçek taşır: merkezi kaldırın.",
            ));
        }
    }
    validate_book(&as_book(t))?;
    crate::validate::validate_variables("variables", &t.variables)?;
    // Pictures: every one used is carried, with bytes that are what its name says.
    let mut total: u64 = 0;
    let used: BTreeSet<String> = used_assets(&t.sheet.items)
        .into_iter()
        .chain(t.master.iter().flat_map(|m| used_assets(&m.items)))
        .collect();
    let mut carried = BTreeSet::new();
    for (i, a) in t.assets.iter().enumerate() {
        let bytes = base64_decode(&a.data).ok_or_else(|| {
            SheetError::at(
                "bad_asset",
                format!("assets[{i}].data"),
                "Resmin baytları base64 olarak okunamadı.",
            )
        })?;
        if sha256_hex(&bytes) != a.meta.sha256 || bytes.len() as u64 != u64::from(a.meta.bytes) {
            return Err(SheetError::at(
                "bad_asset",
                format!("assets[{i}]"),
                "Resmin baytları özetiyle (SHA-256) ya da boyuyla uyuşmuyor: şablon bozulmuş.",
            ));
        }
        total += bytes.len() as u64;
        carried.insert(a.meta.sha256.clone());
    }
    if total > TEMPLATE_ASSETS_MAX_BYTES {
        return Err(SheetError::at(
            "template_too_large",
            "assets",
            "Şablonun resimleri toplam 8 MB'ı geçemez.",
        ));
    }
    if let Some(missing) = used.iter().find(|u| !carried.contains(*u)) {
        return Err(SheetError::at(
            "missing_asset",
            "assets",
            format!(
                "Şablon kullandığı bir resmi taşımıyor ({}…).",
                &missing[..missing.len().min(12)]
            ),
        ));
    }
    Ok(())
}

/// Reads a user's template from JSON: migrated, parsed (an unknown field is an error), validated and normalised.
pub fn read_template(json: &str) -> Result<Template> {
    read_template_as(json, false)
}

fn read_template_as(json: &str, system: bool) -> Result<Template> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| SheetError::json("Şablon", &e))?;
    let v = migrate(v)?;
    let mut t: Template = serde_json::from_value(v).map_err(|e| SheetError::json("Şablon", &e))?;
    validate_template(&t, system)?;
    let mut b = as_book(&t);
    normalize(&mut b);
    if let Some(s) = b.sheets.pop() {
        t.sheet = s;
    }
    t.master = b.masters.pop();
    Ok(t)
}

// ── Using a template ─────────────────────────────────────────────────────

/// Ids replaced by `ids` everywhere they stand: the items' own, their groups and every link between them.
pub(crate) fn remap_items(items: &mut [Item], ids: &BTreeMap<String, String>) {
    let re = |s: &mut String| {
        if let Some(n) = ids.get(s.as_str()) {
            *s = n.clone();
        }
    };
    for it in items {
        if let Some(n) = ids.get(&it.id) {
            it.id = n.clone();
        }
        if let Some(g) = &mut it.group {
            re(g);
        }
        match &mut it.kind {
            ItemKind::Map(m) => {
                if let Some(o) = &mut m.overview_of {
                    re(o);
                }
            }
            ItemKind::ScaleBar(s) => {
                if let Some(m) = &mut s.map {
                    re(m);
                }
            }
            ItemKind::NorthArrow(n) => {
                if let Some(m) = &mut n.map {
                    re(m);
                }
            }
            ItemKind::Legend(l) => {
                if let Some(m) = &mut l.map {
                    re(m);
                }
            }
            ItemKind::Text(t) => {
                if let Some(m) = &mut t.map {
                    re(m);
                }
            }
            ItemKind::Table(t) => {
                if let TableSource::Layer(l) = &mut t.source
                    && let Some(m) = &mut l.only_in_map
                {
                    re(m);
                }
                if let Overflow::ContinueIn(c) = &mut t.overflow {
                    c.items.iter_mut().for_each(re);
                }
            }
            ItemKind::CoordinateList(c) => {
                if let Overflow::ContinueIn(ci) = &mut c.overflow {
                    ci.items.iter_mut().for_each(re);
                }
            }
            _ => {}
        }
    }
}

fn id_map(items: &[Item], new: &[ItemId], what: &str) -> Result<BTreeMap<String, String>> {
    if new.len() != items.len() {
        return Err(SheetError::new(
            "ids_missing",
            format!(
                "{what} için {} yeni kimlik gerekiyor, {} verildi.",
                items.len(),
                new.len()
            ),
        ));
    }
    Ok(items
        .iter()
        .zip(new)
        .map(|(i, n)| (i.id.clone(), n.clone()))
        .collect())
}

/// A new sheet from a template: new ids from the host, links kept, the paper chosen (items follow their constraints), the questions answered.
pub fn instantiate(t: &Template, ids: &InstanceIds, options: &InstanceOptions) -> Result<Instance> {
    let mut sheet = t.sheet.clone();
    let mut master = t.master.clone();
    let map = id_map(&sheet.items, &ids.items, "Paftanın öğeleri")?;
    remap_items(&mut sheet.items, &map);
    crate::variants::remap(&mut sheet.variants, &mut sheet.base_layout, &map);
    sheet.id = ids.sheet.clone();
    if let Some(m) = &mut master {
        let mid = ids.master.clone().ok_or_else(|| {
            SheetError::new(
                "ids_missing",
                "Şablonun ana sayfası için yeni bir kimlik gerekiyor.",
            )
        })?;
        let mmap = id_map(&m.items, &ids.master_items, "Ana sayfanın öğeleri")?;
        remap_items(&mut m.items, &mmap);
        crate::variants::remap(&mut m.variants, &mut m.base_layout, &mmap);
        m.id = mid.clone();
        sheet.master = Some(mid);
    }
    if let Some(name) = &options.name {
        sheet.name = name.clone();
    }
    // The paper chosen (or the template's own), with the layout it calls for (design §3.2a).
    let from = sheet.page.clone();
    let to = match &options.paper {
        Some(choice) => {
            let size = paper::paper_size(choice.paper, choice.orientation).ok_or_else(|| {
                SheetError::new("unknown_paper", "Özel kâğıt şablondan seçilemez; önce standart bir kâğıtla kullanın, sonra boyunu değiştirin.")
            })?;
            Page {
                paper: choice.paper,
                orientation: choice.orientation,
                size,
                ..from.clone()
            }
        }
        None => from.clone(),
    };
    crate::variants::settle(
        &mut crate::variants::LayoutMut::of_sheet(&mut sheet),
        &from,
        &to,
    );
    sheet.page = to;
    for it in &mut sheet.items {
        if let ItemKind::Map(m) = &mut it.kind
            && let MapView::Fixed(f) = &mut m.view
            && m.overview_of.is_none()
        {
            if let Some(c) = options.center {
                f.center = Some(c);
            }
            if let Some(s) = options.scale {
                f.scale = s;
            }
        }
    }
    sheet.variables = t
        .variables
        .iter()
        .map(|v| {
            let mut v = v.clone();
            if let Some(a) = options
                .values
                .iter()
                .find(|a| crate::expr::fold(&a.name) == crate::expr::fold(&v.name))
            {
                v.value = a.value.clone();
            }
            v
        })
        .collect();
    sheet.origin = Some(TemplateOrigin {
        template_id: t.meta.id.clone(),
        revision: t.meta.revision,
    });
    let book = SheetBook {
        sheets: vec![sheet.clone()],
        masters: master.iter().cloned().collect(),
        assets: t.assets.iter().map(|a| a.meta.clone()).collect(),
        ..SheetBook::default()
    };
    validate_book(&book)?;
    let mut b = book;
    normalize(&mut b);
    Ok(Instance {
        sheet: b.sheets.pop().unwrap_or(sheet),
        master: b.masters.pop(),
        assets: t.assets.iter().map(|a| a.meta.clone()).collect(),
        asset_bytes: t.assets.clone(),
    })
}

/// A template from a sheet (“Şablon olarak kaydet”): the maps' places
/// dropped (their scales kept), the pictures gathered with the bytes the
/// host gives, the sheet's variables turned into questions without answers.
pub fn extract(
    book: &SheetBook,
    sheet_id: &str,
    meta: &TemplateMeta,
    assets: &[AssetWithBytes],
) -> Result<Template> {
    let src = book.sheet(sheet_id).ok_or_else(|| {
        SheetError::new(
            "unknown_sheet",
            format!("“{sheet_id}” paftası kitapta yok."),
        )
    })?;
    let mut sheet = src.clone();
    let master = sheet
        .master
        .as_deref()
        .and_then(|m| book.master(m))
        .cloned();
    let strip = |items: &mut Vec<Item>| {
        for it in items {
            if let ItemKind::Map(m) = &mut it.kind
                && let MapView::Fixed(f) = &mut m.view
            {
                f.center = None;
            }
        }
    };
    strip(&mut sheet.items);
    let mut master = master;
    if let Some(m) = &mut master {
        strip(&mut m.items);
    }
    let variables = sheet
        .variables
        .drain(..)
        .map(|mut v| {
            v.value = VarValue::Null;
            v
        })
        .collect();
    sheet.origin = None;
    let used: Vec<String> = used_assets(&sheet.items)
        .into_iter()
        .chain(master.iter().flat_map(|m| used_assets(&m.items)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut carried = Vec::new();
    for sha in &used {
        let a = assets
            .iter()
            .find(|a| a.meta.sha256 == *sha)
            .ok_or_else(|| {
                SheetError::new(
                    "missing_asset",
                    format!(
                        "Şablona konacak bir resmin baytları verilmedi ({}…).",
                        &sha[..sha.len().min(12)]
                    ),
                )
            })?;
        carried.push(a.clone());
    }
    let t = Template {
        schema: TEMPLATE_SCHEMA.to_owned(),
        meta: meta.clone(),
        sheet,
        master,
        assets: carried,
        variables,
    };
    validate_template(&t, t.meta.id.starts_with("sys:"))?;
    Ok(t)
}

// ── System templates ─────────────────────────────────────────────────────

/// The program's own templates (design §12), in the gallery's order.
const SYSTEM: &[&str] = &[
    include_str!("../templates/genel-a4-dikey.json"),
    include_str!("../templates/genel-a3-yatay.json"),
    include_str!("../templates/rapor-sayfasi.json"),
    include_str!("../templates/cad-teknik.json"),
    include_str!("../templates/cad-mimari.json"),
    include_str!("../templates/aplikasyon-krokisi.json"),
    include_str!("../templates/ifraz-paftasi.json"),
    include_str!("../templates/imar-plani.json"),
    include_str!("../templates/gis-tematik.json"),
    include_str!("../templates/gis-atlas.json"),
];

/// The system templates, read once; one that does not read is left out (a test proves none is).
pub fn system_templates() -> &'static [Template] {
    static T: OnceLock<Vec<Template>> = OnceLock::new();
    T.get_or_init(|| {
        SYSTEM
            .iter()
            .filter_map(|j| read_template_as(j, true).ok())
            .collect()
    })
}

/// Every system template's reading, for the tests: the error of one that does not read.
pub fn system_template_errors() -> Vec<(usize, SheetError)> {
    SYSTEM
        .iter()
        .enumerate()
        .filter_map(|(i, j)| read_template_as(j, true).err().map(|e| (i, e)))
        .collect()
}

pub fn system_template(id: &str) -> Option<&'static Template> {
    system_templates().iter().find(|t| t.meta.id == id)
}

/// The page a paper choice gives, with a template's margins.
pub fn page_for(choice: &PaperChoice, like: &Page) -> Option<Page> {
    let size: SizeUm = paper::paper_size(choice.paper, choice.orientation)?;
    Some(Page {
        paper: choice.paper,
        orientation: choice.orientation,
        size,
        ..like.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for n in 0..10 {
            let bytes: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            let e = base64_encode(&bytes);
            assert_eq!(base64_decode(&e).unwrap(), bytes, "{e}");
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_decode("TQ==").unwrap(), b"M");
        assert!(base64_decode("TQ=").is_none());
        assert!(base64_decode("T=Q=").is_none());
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn migration_refuses_what_it_cannot_read() {
        let v = serde_json::json!({ "schema": "kentos.sheet.template/2" });
        assert_eq!(migrate(v).unwrap_err().code, "newer_schema");
        let v = serde_json::json!({ "schema": "qgis" });
        assert_eq!(migrate(v).unwrap_err().code, "unknown_schema");
        let v = serde_json::json!({ "schema": "kentos.sheet.template/1", "x": 1 });
        assert_eq!(migrate(v.clone()).unwrap(), v);
    }
}
