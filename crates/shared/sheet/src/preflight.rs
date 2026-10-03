//! The preflight (design §9): what is wrong with a sheet before it is
//! exported, each finding with its severity, the item, a stable code, a
//! Turkish message and its fix — and, where the core can make it, the fix
//! as operations the interface offers as a button. An export with errors
//! asks the user to say “yine de aktar”.
//!
//! A value only the user can give (a template's unanswered question, a map
//! not placed yet, the data a legend or table waits for) is a placeholder:
//! still a finding, marked so the interface can list them together.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::display::{self, Note, RenderInputs};
use crate::error::Result;
use crate::kinds::*;
use crate::model::*;
use crate::ops::{ItemIds, MoveItems, Op, SetAtlas, SetItemProps};
use crate::paper;
use crate::scene::Scene;
use crate::style::{TextStyle, alpha};
use crate::text;
use crate::units::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A way to put a finding right: operations the core can apply, or an action of the host (`project.crs`, `map.placeFromView`, `sheet.variables`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Fix {
    pub label: String,
    #[serde(default)]
    pub ops: Vec<Op>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Finding {
    pub severity: Severity,
    /// Stable, `snake_case`.
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<ItemId>,
    pub message: String,
    /// How to put it right, in a sentence.
    pub fix: String,
    #[serde(default)]
    pub fixes: Vec<Fix>,
    /// A value only the user can give.
    #[serde(default)]
    pub placeholder: bool,
}

/// The codes that are placeholders: data the user gives, not a fault of the layout.
pub const PLACEHOLDERS: &[&str] = &[
    "value_missing",
    "map_unplaced",
    "atlas_no_layer",
    "table_data_missing",
    "coordinates_missing",
    "legend_empty",
    "picture_empty",
];

struct Out {
    list: Vec<Finding>,
}

impl Out {
    fn add(
        &mut self,
        severity: Severity,
        code: &str,
        item: Option<&Item>,
        message: String,
        fix: &str,
        fixes: Vec<Fix>,
    ) {
        let id = item.map(|i| i.id.clone());
        if self
            .list
            .iter()
            .any(|f| f.code == code && f.item == id && f.message == message)
        {
            return;
        }
        self.list.push(Finding {
            severity,
            code: code.to_owned(),
            item: id,
            message,
            fix: fix.to_owned(),
            fixes,
            placeholder: PLACEHOLDERS.contains(&code),
        });
    }
}

pub(crate) fn named(it: &Item) -> String {
    format!("“{}” ({})", it.name, it.kind.label())
}

fn op_fix(label: &str, op: Op) -> Fix {
    Fix {
        label: label.to_owned(),
        ops: vec![op],
        action: None,
    }
}

fn action(label: &str, action: &str) -> Fix {
    Fix {
        label: label.to_owned(),
        ops: Vec::new(),
        action: Some(action.to_owned()),
    }
}

/// “Sapmayı elle gir”: the north arrow's declination typed by hand from now on (the inspector
/// then asks for it), starting, when nothing was typed yet, from the model's value to the minute
/// (mdeg) where there is one: what the inspector's switch does. Otherwise the value is kept.
fn by_hand(i: &Item, from: Option<i32>) -> Fix {
    let mut kind = serde_json::json!({ "type": "northArrow", "declinationHand": true });
    let typed = match &i.kind {
        ItemKind::NorthArrow(n) => n.declination,
        _ => 0,
    };
    if let Some(v) = from.filter(|_| typed == 0) {
        kind["declination"] = serde_json::json!(v);
    }
    op_fix(
        "Sapmayı elle gir",
        Op::SetItemProps(SetItemProps {
            id: i.id.clone(),
            patch: serde_json::json!({ "kind": kind }),
        }),
    )
}

/// The text styles an item writes with.
fn styles(it: &Item) -> Vec<&TextStyle> {
    fn cells<'a>(rows: &'a [TitleRow], out: &mut Vec<&'a TextStyle>) {
        for r in rows {
            for c in &r.cells {
                if let Some(s) = &c.style {
                    out.push(s);
                }
                cells(&c.rows, out);
            }
        }
    }
    let mut out = Vec::new();
    match &it.kind {
        ItemKind::Map(m) => out.extend(m.grids.iter().map(|g| &g.labels.style)),
        ItemKind::Text(t) => out.push(&t.style),
        ItemKind::ScaleBar(s) => out.push(&s.text),
        ItemKind::NorthArrow(n) => out.push(&n.text),
        ItemKind::Legend(l) => out.extend([&l.title_style, &l.group_style, &l.label_style]),
        ItemKind::Table(t) => {
            out.extend([&t.title_style, &t.header_style.text, &t.cell_style.text])
        }
        ItemKind::CoordinateList(c) => {
            out.extend([&c.title_style, &c.header_style.text, &c.cell_style.text])
        }
        ItemKind::TitleBlock(tb) => {
            out.extend([&tb.label_style, &tb.value_style]);
            cells(&tb.rows, &mut out);
        }
        ItemKind::Border(b) => {
            if let Some(z) = &b.zones {
                out.push(&z.text);
            }
        }
        ItemKind::Picture(_) | ItemKind::Shape(_) | ItemKind::Line(_) | ItemKind::Group(_) => {}
    }
    out
}

/// The nearest standard scale (a tie goes to the larger scale).
fn nearest_standard(scale: u32) -> Option<u32> {
    paper::standard_scales().iter().copied().min_by(|a, b| {
        let da = libm::fabs(libm::log(f64::from(*a) / f64::from(scale.max(1))));
        let db = libm::fabs(libm::log(f64::from(*b) / f64::from(scale.max(1))));
        da.total_cmp(&db).then(a.cmp(b))
    })
}

/// Every finding of a sheet, errors first.
pub fn preflight(book: &SheetBook, sheet_id: &str, inputs: &RenderInputs) -> Result<Vec<Finding>> {
    let scene = Scene::new(book, sheet_id)?;
    let sheet = scene.sheet;
    let (_, notes) = display::build(book, sheet_id, inputs)?;
    let mut out = Out { list: Vec::new() };
    let page = sheet.page.rect();
    let margins = sheet.page.margin_rect();
    let caps = &inputs.capabilities;
    let maps: Vec<&Item> = scene
        .all()
        .map(|(i, _)| i)
        .filter(|i| matches!(i.kind, ItemKind::Map(_)))
        .collect();
    let is_map = |id: &str| maps.iter().any(|m| m.id == id);
    let mut has_text = false;
    for (it, master) in scene.all() {
        if !scene.shown(it) || it.is_group() {
            continue;
        }
        let b = rotated_bounds(&it.frame, it.rotation);
        // On the paper.
        if !page.contains_rect(&b) {
            let fully = !page.intersects(&b);
            let mut fixes = Vec::new();
            if !master && !it.locked {
                let dx = if i64::from(b.left) < i64::from(margins.left) {
                    i64::from(margins.left) - i64::from(b.left)
                } else if b.right() > margins.right() {
                    margins.right() - b.right()
                } else {
                    0
                };
                let dy = if i64::from(b.top) < i64::from(margins.top) {
                    i64::from(margins.top) - i64::from(b.top)
                } else if b.bottom() > margins.bottom() {
                    margins.bottom() - b.bottom()
                } else {
                    0
                };
                if i64::from(b.width) <= i64::from(margins.width)
                    && i64::from(b.height) <= i64::from(margins.height)
                {
                    fixes.push(op_fix(
                        "Basılabilir alana taşı",
                        Op::MoveItems(MoveItems {
                            ids: vec![it.id.clone()],
                            delta: [sat(dx), sat(dy)],
                        }),
                    ));
                }
            }
            out.add(
                Severity::Error,
                "off_page",
                Some(it),
                format!(
                    "{} {}.",
                    named(it),
                    if fully {
                        "kâğıdın tamamen dışında"
                    } else {
                        "kâğıttan taşıyor"
                    }
                ),
                "Öğeyi kâğıdın içine taşıyın ya da küçültün.",
                fixes,
            );
        } else if !margins.contains_rect(&b) && !matches!(it.kind, ItemKind::Border(_)) {
            out.add(
                Severity::Warning,
                "outside_margins",
                Some(it),
                format!(
                    "{} kenar boşluğuna taşıyor: yazıcı bu kısmı basmayabilir.",
                    named(it)
                ),
                "Öğeyi kenar boşluklarının içine alın ya da kenar boşluklarını küçültün.",
                Vec::new(),
            );
        }
        // Links to a map.
        let link_kinds = matches!(
            it.kind,
            ItemKind::ScaleBar(_) | ItemKind::NorthArrow(_) | ItemKind::Legend(_)
        );
        if link_kinds {
            match it.kind.map_link() {
                None => out.add(
                    Severity::Error,
                    "broken_link",
                    Some(it),
                    format!("{} bir haritaya bağlı değil.", named(it)),
                    "Denetçide “Harita” alanından bağlı olduğu haritayı seçin.",
                    Vec::new(),
                ),
                Some(m) if !is_map(m) => out.add(
                    Severity::Error,
                    "broken_link",
                    Some(it),
                    format!("{} silinmiş bir haritaya bağlı (bağı kopuk); başka bir haritaya kendiliğinden bağlanmaz.", named(it)),
                    "Denetçide “Harita” alanından bağlı olduğu haritayı yeniden seçin.",
                    Vec::new(),
                ),
                _ => {}
            }
        }
        if let ItemKind::Text(t) = &it.kind
            && let Some(m) = &t.map
            && !is_map(m)
        {
            out.add(
                Severity::Error,
                "broken_link",
                Some(it),
                format!(
                    "{} silinmiş bir haritanın ölçeğini okuyor (bağı kopuk).",
                    named(it)
                ),
                "Denetçide metnin haritasını yeniden seçin.",
                Vec::new(),
            );
        }
        if let ItemKind::Map(m) = &it.kind {
            if let Some(o) = &m.overview_of
                && !is_map(o)
            {
                out.add(
                    Severity::Error,
                    "broken_link",
                    Some(it),
                    format!(
                        "{} silinmiş bir haritanın genel bakışı (bağı kopuk).",
                        named(it)
                    ),
                    "Genel bakışın göstereceği haritayı yeniden seçin.",
                    Vec::new(),
                );
            }
            if let MapView::Fixed(f) = &m.view
                && m.overview_of.is_none()
                && !paper::is_standard_scale(f.scale)
            {
                let fixes = nearest_standard(f.scale)
                    .map(|s| {
                        vec![op_fix(
                            &format!("Ölçeği 1/{s} yap"),
                            Op::SetItemProps(SetItemProps {
                                id: it.id.clone(),
                                patch: serde_json::json!({ "kind": { "type": "map", "view": { "type": "fixed", "scale": s } } }),
                            }),
                        )]
                    })
                    .unwrap_or_default();
                out.add(
                    Severity::Warning,
                    "non_standard_scale",
                    Some(it),
                    format!("{} 1/{} ölçeğinde: standart bir ölçek değil.", named(it), f.scale),
                    "Kadastro ve imar paftaları standart ölçek ister (1/500, 1/1000, 1/2000, 1/5000 …).",
                    fixes,
                );
            }
            let grids = m.grids.iter().any(|g| g.enabled);
            if grids && !caps.georeferenced {
                out.add(
                    Severity::Error,
                    "needs_crs",
                    Some(it),
                    format!("{} karelajı koordinat sistemi ister; projenin koordinat sistemi yok.", named(it)),
                    "Proje ayarlarından bir koordinat sistemi seçin ya da karelajı kaldırın.",
                    vec![
                        action("Koordinat sistemi seç", "project.crs"),
                        op_fix(
                            "Karelajı kaldır",
                            Op::SetItemProps(SetItemProps {
                                id: it.id.clone(),
                                patch: serde_json::json!({ "kind": { "type": "map", "grids": [] } }),
                            }),
                        ),
                    ],
                );
            }
        }
        if let ItemKind::NorthArrow(_) = &it.kind
            && !caps.georeferenced
        {
            out.add(
                Severity::Error,
                "needs_crs",
                Some(it),
                format!(
                    "{} koordinat sistemi ister; projenin koordinat sistemi yok.",
                    named(it)
                ),
                "Proje ayarlarından bir koordinat sistemi seçin ya da kuzey okunu kaldırın.",
                vec![
                    action("Koordinat sistemi seç", "project.crs"),
                    op_fix(
                        "Kuzey okunu kaldır",
                        Op::RemoveItems(ItemIds {
                            ids: vec![it.id.clone()],
                        }),
                    ),
                ],
            );
        }
        if let ItemKind::Table(t) = &it.kind
            && let TableSource::Layer(l) = &t.source
        {
            if !caps.attribute_layers {
                out.add(
                    Severity::Error,
                    "needs_attribute_layers",
                    Some(it),
                    format!("{} öznitelikli bir katman ister; projede yok.", named(it)),
                    "Öznitelikli bir katman ekleyin ya da tabloyu sabit tabloya çevirin.",
                    vec![op_fix(
                        "Tabloyu kaldır",
                        Op::RemoveItems(ItemIds {
                            ids: vec![it.id.clone()],
                        }),
                    )],
                );
            }
            if let Some(m) = &l.only_in_map
                && !is_map(m)
            {
                out.add(
                    Severity::Error,
                    "broken_link",
                    Some(it),
                    format!(
                        "{} silinmiş bir haritanın içindekileri listeliyor (bağı kopuk).",
                        named(it)
                    ),
                    "Tablonun haritasını yeniden seçin.",
                    Vec::new(),
                );
            }
        }
        // Typefaces.
        for s in styles(it) {
            let known = text::has_font(&s.font);
            let host = inputs.fonts.as_ref().is_none_or(|f| f.contains(&s.font));
            if !known || !host {
                out.add(
                    Severity::Error,
                    "missing_font",
                    Some(it),
                    format!(
                        "{} “{}” yazı tipiyle yazılmış; bu yazı tipi yok.",
                        named(it),
                        s.font
                    ),
                    "Çizimin yazı tiplerinden birini seçin (Barlow, Arimo, Overpass …).",
                    Vec::new(),
                );
            }
        }
        if !styles(it).is_empty() {
            has_text = true;
        }
        // Pictures: their resolution at the size they print.
        if let ItemKind::Picture(p) = &it.kind
            && let Some(sha) = &p.asset
            && let Some(a) = book.asset(sha)
            && a.kind != AssetKind::Svg
        {
            let shown_mm =
                f64::from(it.content_rect().width.min(it.content_rect().height)) / 1000.0;
            let px = f64::from(if it.content_rect().width <= it.content_rect().height {
                a.width
            } else {
                a.height
            });
            let ppi = if shown_mm > 0.0 {
                px / (shown_mm / 25.4)
            } else {
                f64::MAX
            };
            if ppi < 150.0 {
                out.add(
                    Severity::Warning,
                    "low_resolution",
                    Some(it),
                    format!(
                        "{} basıldığı boyda {} ppi: 150 ppi'nin altında, bulanık çıkar.",
                        named(it),
                        round_i64(ppi)
                    ),
                    "Daha büyük bir resim koyun ya da çerçeveyi küçültün.",
                    Vec::new(),
                );
            }
        }
        // Covered wholly by an opaque item above it.
        if !master {
            let above = sheet.items.iter().skip_while(|x| x.id != it.id).skip(1);
            for up in above {
                if up.is_group() || !scene.shown(up) || up.rotation != 0 || up.opacity < 100 {
                    continue;
                }
                let opaque = up.fill.as_deref().is_some_and(|f| alpha(f) == 255)
                    || matches!(&up.kind, ItemKind::Shape(s) if s.fill.as_deref().is_some_and(|f| alpha(f) == 255) && matches!(s.shape, ShapeKind::Rect(_)));
                if opaque && up.frame.contains_rect(&b) {
                    out.add(
                        Severity::Warning,
                        "covered",
                        Some(it),
                        format!(
                            "{}, üstündeki “{}” öğesinin altında kalıyor: görünmez.",
                            named(it),
                            up.name
                        ),
                        "Öğeyi öne getirin ya da üstündekini kaydırın.",
                        Vec::new(),
                    );
                    break;
                }
            }
        }
    }
    // Items whose content runs into another's (a map, a frame or a shape under them is meant to be).
    let content = |it: &Item| {
        matches!(
            it.kind,
            ItemKind::Text(_)
                | ItemKind::Table(_)
                | ItemKind::CoordinateList(_)
                | ItemKind::TitleBlock(_)
                | ItemKind::Legend(_)
                | ItemKind::ScaleBar(_)
                | ItemKind::NorthArrow(_)
        )
    };
    let shown: Vec<&Item> = scene
        .all()
        .map(|(i, _)| i)
        .filter(|i| scene.shown(i) && content(i))
        .collect();
    for (k, a) in shown.iter().enumerate() {
        let ra = rotated_bounds(&a.content_rect(), a.rotation);
        for b in &shown[k + 1..] {
            let rb = rotated_bounds(&b.content_rect(), b.rotation);
            let w = ra.right().min(rb.right()) - i64::from(ra.left.max(rb.left));
            let h = ra.bottom().min(rb.bottom()) - i64::from(ra.top.max(rb.top));
            if w > 500 && h > 500 {
                out.add(
                    Severity::Warning,
                    "overlap",
                    Some(b),
                    format!("{} ile {} üst üste geliyor.", named(b), named(a)),
                    "Öğelerden birini kaydırın ya da küçültün; kâğıt küçüldüyse kısıtlarını denetleyin.",
                    Vec::new(),
                );
            }
        }
    }
    // The atlas.
    if let Some(a) = &sheet.atlas {
        if a.layer.trim().is_empty() {
            out.add(
                Severity::Error,
                "atlas_no_layer",
                None,
                "Atlasın kapsam katmanı seçilmedi: sayfa üretilmez.".to_owned(),
                "Pafta → Atlas'tan kapsam katmanını seçin.",
                vec![action("Kapsam katmanı seç", "sheet.atlas")],
            );
        }
        if !caps.georeferenced {
            out.add(
                Severity::Error,
                "needs_crs",
                None,
                "Atlas koordinat sistemi ister; projenin koordinat sistemi yok.".to_owned(),
                "Proje ayarlarından bir koordinat sistemi seçin ya da atlası kapatın.",
                vec![
                    action("Koordinat sistemi seç", "project.crs"),
                    op_fix(
                        "Atlası kapat",
                        Op::SetAtlas(SetAtlas {
                            sheet: sheet.id.clone(),
                            atlas: None,
                        }),
                    ),
                ],
            );
        }
    }
    // Assets the book lacks or the host has no bytes for.
    for (it, _) in scene.all() {
        for sha in crate::template::used_assets(std::slice::from_ref(it)) {
            let in_book = book.asset(&sha).is_some();
            let bytes = inputs.assets.as_ref().is_none_or(|a| a.contains(&sha));
            if !in_book || !bytes {
                out.add(
                    Severity::Error,
                    "missing_asset",
                    Some(it),
                    format!(
                        "{} kullandığı resim bulunamadı ({}…).",
                        named(it),
                        &sha[..sha.len().min(12)]
                    ),
                    "Resmi yeniden ekleyin ya da öğeden kaldırın.",
                    Vec::new(),
                );
            }
        }
    }
    let dpi = inputs.dpi.unwrap_or(sheet.export.dpi);
    if dpi < 150 {
        out.add(
            Severity::Warning,
            "dpi_low",
            None,
            format!("Dışa aktarma çözünürlüğü {dpi} dpi: baskı için düşük."),
            "Dışa aktarma ayarlarında en az 300 dpi seçin.",
            Vec::new(),
        );
    }
    // What the layout found.
    for n in &notes {
        // No coordinate system at all is `needs_crs` already (above).
        if n.code == "magnetic_no_place" && !caps.georeferenced {
            continue;
        }
        let it = scene.item(&n.item);
        note(&mut out, n, it);
    }
    if has_text {
        out.add(
            Severity::Info,
            "no_kerning",
            None,
            "Yazılar çekirdeğin ölçüleriyle yerleşir; harf çifti aralığı (kerning) bu sürümde uygulanmaz.".to_owned(),
            "Türkçe ve Latin yazıda fark göze görünmez; yazı tipi değişince satırlar yeniden kırılır.",
            Vec::new(),
        );
    }
    let order = |f: &Finding| {
        let pos = f
            .item
            .as_deref()
            .and_then(|id| scene.all().position(|(i, _)| i.id == id))
            .unwrap_or(usize::MAX);
        (f.severity, pos, f.code.clone(), f.message.clone())
    };
    out.list.sort_by_key(order);
    Ok(out.list)
}

fn note(out: &mut Out, n: &Note, it: Option<&Item>) {
    let who = it.map_or_else(|| n.item.clone(), named);
    match n.code {
        "value_missing" => out.add(
            Severity::Error,
            "value_missing",
            it,
            format!("{who}: “@{}” değerinin değeri yok; kâğıda ‹{}?› yazılıyor.", n.detail, n.detail),
            "Pafta → Değişkenler'den değeri girin.",
            vec![action("Değişkenleri aç", "sheet.variables")],
        ),
        "expression_error" => out.add(
            Severity::Error,
            "expression_error",
            it,
            format!("{who}: ifade çalışmıyor ({}).", n.detail),
            "İfadeyi İfade oluşturucuda düzeltin.",
            Vec::new(),
        ),
        "value_null" => out.add(
            Severity::Error,
            "value_null",
            it,
            format!("{who}: ifade boş değer verdi ({}).", n.detail),
            "İfadenin her durumda bir değer verdiğini denetleyin.",
            Vec::new(),
        ),
        "binding_error" => out.add(
            Severity::Error,
            "binding_error",
            it,
            format!("{who}: veriye bağlı özellik uygulanamadı ({}).", n.detail),
            "Bağın ifadesinin özelliğe uyan bir değer verdiğini denetleyin (sayı, metin ya da doğru/yanlış).",
            Vec::new(),
        ),
        // A text the core writes itself (a north diagram's angles, a scale bar's line, a
        // legend's label): it has no “Sığdır” of its own; the diagram's and the note's have
        // already been wrapped and made as small as they may be.
        "text_overflow" => match n.detail.split_once('\u{1f}') {
            Some(("written", what)) => out.add(
                Severity::Warning,
                "text_overflow",
                it,
                format!("{who}: {what} çerçeveye sığmıyor."),
                "Çerçeveyi büyütün ya da öğenin yazı boyunu küçültün.",
                Vec::new(),
            ),
            _ => out.add(
                Severity::Warning,
                "text_overflow",
                it,
                format!("{who}: yazı çerçevesine sığmıyor{}.", if n.detail.is_empty() { String::new() } else { format!(" ({})", n.detail) }),
                "Çerçeveyi büyütün, yazıyı kısaltın ya da “Sığdır”ı açın.",
                Vec::new(),
            ),
        },
        "table_overflow" => out.add(
            Severity::Warning,
            "table_overflow",
            it,
            format!("{who}: {} satır çerçeveye sığmadı, basılmayacak.", n.detail),
            "Tabloyu büyütün ya da taşan satırlar için bir devam çerçevesi ekleyin.",
            Vec::new(),
        ),
        "legend_overflow" => out.add(
            Severity::Warning,
            "legend_overflow",
            it,
            format!("{who}: lejantın bütün satırları çerçeveye sığmıyor."),
            "Lejantı büyütün ya da sütun sayısını artırın.",
            Vec::new(),
        ),
        "legend_empty" => out.add(
            Severity::Info,
            "legend_empty",
            it,
            format!("{who}: lejantın gösterecek satırı yok."),
            "Haritada görünen katmanlar lejantı doldurur.",
            Vec::new(),
        ),
        "grid_label_band" => out.add(
            Severity::Error,
            "grid_label_band",
            it,
            format!("{who}: karelaj yazıları yazı bandına sığmıyor ({} mm gerekiyor).", mm_text(n.detail.parse::<i64>().unwrap_or(0))),
            "Haritanın yazı bandını genişletin ya da yazıları içeri alın.",
            it.map(|i| {
                vec![op_fix(
                    "Yazı bandını genişlet",
                    Op::SetItemProps(SetItemProps {
                        id: i.id.clone(),
                        patch: serde_json::json!({ "kind": { "type": "map", "labelBand": n.detail.parse::<i64>().unwrap_or(0) + 500 } }),
                    }),
                )]
            })
            .unwrap_or_default(),
        ),
        "grid_too_dense" => out.add(
            Severity::Warning,
            "grid_too_dense",
            it,
            format!("{who}: karelaj aralığı ({}) bu ölçek için çok sık; çizilmedi.", n.detail),
            "Karelaj aralığını büyütün ya da 0 yapın (ölçekten seçilir).",
            Vec::new(),
        ),
        "grid_crs_unsupported" => out.add(
            Severity::Warning,
            "grid_crs_unsupported",
            it,
            format!("{who}: başka koordinat sisteminde ({}) karelaj bu sürümde çizilmiyor.", n.detail),
            "Karelajın koordinat sistemini haritanınkiyle aynı bırakın.",
            Vec::new(),
        ),
        "grid_format_unsupported" => out.add(
            Severity::Warning,
            "grid_format_unsupported",
            it,
            format!("{who}: derece-dakika-saniye yazıları yalnız coğrafi karelajda olur; metre yazıldı."),
            "Yazı biçimini “Metre” yapın.",
            Vec::new(),
        ),
        "map_unplaced" => out.add(
            Severity::Error,
            "map_unplaced",
            it,
            format!("{who}: haritanın yeri seçilmedi."),
            "Denetçide “Görünümden al” ile haritanın merkezini çizim alanından alın.",
            vec![action("Görünümden al", "map.placeFromView")],
        ),
        "picture_empty" => out.add(
            Severity::Info,
            "picture_empty",
            it,
            format!("{who}: resim seçilmedi."),
            "Denetçiden bir resim seçin.",
            Vec::new(),
        ),
        "convergence_unknown" => out.add(
            Severity::Warning,
            "convergence_unknown",
            it,
            format!("{who}: meridyen yakınsaması bilinmiyor; ok grid kuzeyini gösteriyor."),
            "Projenin koordinat sistemi bir TM ya da UTM dilimi olmalı.",
            Vec::new(),
        ),
        "table_data_missing" => out.add(
            Severity::Info,
            "table_data_missing",
            it,
            format!("{who}: tablonun katmanı{} henüz veri vermedi.", if n.detail.is_empty() { String::new() } else { format!(" ({})", n.detail) }),
            "Tablonun katmanını seçin; satırlar katmanın nesnelerinden gelir.",
            Vec::new(),
        ),
        "coordinates_missing" => out.add(
            Severity::Info,
            "coordinates_missing",
            it,
            format!("{who}: listelenecek nokta yok."),
            "Çizimden nesneleri seçip “Koordinatları al” deyin.",
            Vec::new(),
        ),
        "continuation_unused" => out.add(
            Severity::Warning,
            "continuation_unused",
            it,
            format!("{who}: hiçbir tablo bu devam çerçevesine akmıyor."),
            "Bir tablonun taşma ayarında bu çerçeveyi seçin ya da çerçeveyi silin.",
            vec![op_fix("Çerçeveyi sil", Op::RemoveItems(ItemIds { ids: vec![n.item.clone()] }))],
        ),
        "scale_bar_fit" => out.add(
            Severity::Warning,
            "scale_bar_fit",
            it,
            format!("{who}: ölçek çubuğu çerçeveye sığmıyor."),
            "Çerçeveyi genişletin ya da parça sayısını azaltın.",
            Vec::new(),
        ),
        "magnetic_no_place" => out.add(
            Severity::Error,
            "magnetic_no_place",
            it,
            format!("{who}: manyetik sapma hesaplanamıyor: haritanın yeri bilinmiyor (koordinat sistemi TM ya da UTM değil)."),
            "Projeye bir TM ya da UTM koordinat sistemi seçin ya da sapmayı elle girin.",
            {
                let mut fixes = vec![action("Koordinat sistemi seç", "project.crs")];
                fixes.extend(it.map(|i| by_hand(i, None)));
                fixes
            },
        ),
        "magnetic_no_date" => out.add(
            Severity::Error,
            "magnetic_no_date",
            it,
            format!("{who}: manyetik sapma için paftanın tarihi yok (@tarih)."),
            "Paftaya ya da projeye ISO biçiminde bir “tarih” değişkeni girin (2026-10-03).",
            vec![action("Değişkenleri aç", "sheet.variables")],
        ),
        "magnetic_out_of_model" => {
            let mut parts = n.detail.split('\u{1f}');
            let (model, year) = (
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
            );
            let value = parts.next().and_then(|v| v.parse::<i32>().ok());
            let validity = crate::wmm::info().map_or_else(String::new, |i| {
                format!(" ({}–{})", i.valid_from, i.valid_until)
            });
            out.add(
                Severity::Warning,
                "magnetic_out_of_model",
                it,
                format!("{who}: paftanın tarihi ({year}) {model} modelinin geçerlilik döneminin{validity} dışında; manyetik sapma yaklaşıktır."),
                "Paftanın tarihini denetleyin ya da sapmayı elle girin.",
                {
                    let mut fixes = vec![action("Değişkenleri aç", "sheet.variables")];
                    fixes.extend(it.map(|i| by_hand(i, value)));
                    fixes
                },
            )
        }
        "glyph_missing" => {
            let mut parts = n.detail.split('\u{1f}');
            let (chars, face, other) = (
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
            );
            let listed = chars
                .chars()
                .map(|c| format!("“{c}”"))
                .collect::<Vec<_>>()
                .join(", ");
            if other.is_empty() {
                out.add(
                    Severity::Warning,
                    "glyph_missing",
                    it,
                    format!("{who}: {listed} hiçbir çizim yazı tipinde yok ({face}); kâğıda “?” yazılıyor."),
                    "Karakteri yazı tiplerinin yazdığı bir karşılığıyla değiştirin (“≤” yerine “<=” gibi).",
                    Vec::new(),
                )
            } else {
                out.add(
                    Severity::Info,
                    "glyph_missing",
                    it,
                    format!("{who}: {listed} {face} yazı tipinde yok; {other} ile yazılıyor."),
                    "Böyle bırakabilir ya da bu karakteri içeren bir yazı tipi seçebilirsiniz.",
                    Vec::new(),
                )
            }
        }
        "zone_text_too_large" => out.add(
            Severity::Warning,
            "zone_text_too_large",
            it,
            format!("{who}: bölge yazıları bant için büyük."),
            "Bölge yazısını küçültün ya da çizgi arasını büyütün.",
            Vec::new(),
        ),
        // Already found from the model (links) or harmless.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_standard_scale() {
        assert_eq!(nearest_standard(855), Some(1000));
        assert_eq!(nearest_standard(1200), Some(1000));
        assert_eq!(nearest_standard(2300), Some(2500));
        assert_eq!(nearest_standard(7000), Some(5000));
    }
}
