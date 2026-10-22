//! Real LAYOUT/VIEWPORT records and paper-space objects, separate from model geometry.
use super::{
    emit::{Ctx, Emitter, Library, Out},
    entity::Parsed,
    lexer::{Lexer, Pair},
    strings::Decoder,
};
use crate::{
    num::{parse_int, parse_real},
    report::Report,
    units,
    watch::{STOPPED, Watch},
};
use kentos_contracts::{CadLayout, CadViewport, DxfReadOptions, Vec2};
use std::collections::{BTreeMap, HashMap};

fn num(groups: &[Pair<'_>], code: i32, default: f64) -> f64 {
    groups
        .iter()
        .find(|p| p.code == code)
        .and_then(|p| parse_real(p.text()))
        .unwrap_or(default)
}
fn text(groups: &[Pair<'_>], code: i32, dec: Decoder) -> String {
    groups
        .iter()
        .find(|p| p.code == code)
        .map(|p| dec.string(p.value))
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn read(
    bytes: &[u8],
    lib: &Library,
    mut paper: Vec<Parsed>,
    dec: Decoder,
    options: &DxfReadOptions,
    model_scale: units::Scale,
    report: &mut Report,
    watch: &mut dyn Watch,
) -> Result<Vec<CadLayout>, String> {
    let mut lex = Lexer::new(bytes);
    let mut layouts: BTreeMap<String, (CadLayout, f64)> = BTreeMap::new();
    let mut owners = HashMap::new();
    let mut viewports = Vec::new();
    let mut seen = 0u32;
    while let Some(p) = lex.next()? {
        if p.code != 0 {
            continue;
        }
        seen += 1;
        if seen.is_multiple_of(2048) && !watch.step(950, 1000) {
            return Err(STOPPED.into());
        }
        let groups = lex.until_zero()?;
        if p.text() == "LAYOUT" {
            let at = groups
                .iter()
                .position(|p| p.code == 100 && p.text() == "AcDbLayout");
            let layout_groups = at.map_or(groups.as_slice(), |i| &groups[i + 1..]);
            let name = text(layout_groups, 1, dec);
            if name.is_empty() || name.eq_ignore_ascii_case("Model") {
                continue;
            }
            let width = num(&groups, 44, 420.);
            let height = num(&groups, 45, 297.);
            if !(width > 0. && height > 0. && width <= 1_000_000. && height <= 1_000_000.) {
                report.about(
                    "LAYOUT",
                    "pafta boyutu geçersiz; boyutsuz/boş pafta kaydı oluşturulmadı",
                );
                continue;
            }
            let paper_units = num(&groups, 72, 1.);
            if paper_units != 0. && paper_units != 1. {
                report.skip(
                    "LAYOUT",
                    "piksel/özel plot birimli pafta 2B kâğıt ölçüsüne çevrilemedi",
                    p.line,
                );
                continue;
            }
            let per = if paper_units == 0. { 25.4 } else { 1. };
            let owner = layout_groups
                .iter()
                .find(|p| p.code == 330)
                .and_then(|p| u64::from_str_radix(p.text().trim(), 16).ok());
            if let Some(block) = owner.and_then(|h| lib.block_records.get(&h)) {
                owners.insert(block.to_uppercase(), name.clone());
            }
            if layouts.len() >= 256 {
                return Err("DXF pafta sınırı (256) aşıldı; dosyayı bölün.".into());
            }
            layouts.insert(
                name.clone(),
                (
                    CadLayout {
                        name,
                        width_mm: width,
                        height_mm: height,
                        tab_order: num(layout_groups, 71, 1.).max(0.) as u32,
                        ..Default::default()
                    },
                    per,
                ),
            );
        } else if p.text() == "VIEWPORT" {
            // The viewport numbered 1 is the paperspace editor, not a model view frame.
            if num(&groups, 69, 1.) > 1. && num(&groups, 68, 1.) >= 0. {
                viewports.push((p.line, groups));
            }
        }
    }
    // Inactive paper spaces keep their objects in *Paper_SpaceN blocks.
    for (name, block) in &lib.blocks {
        if name.starts_with("*PAPER_SPACE") {
            for entity in &block.entities {
                let mut entity = entity.clone();
                entity.common.layout = owners.get(name).cloned().unwrap_or_default();
                paper.push(entity);
            }
        }
    }
    if layouts.is_empty() && (!paper.is_empty() || !viewports.is_empty()) {
        report.about(
            "LAYOUT",
            "kâğıt uzayı var, LAYOUT boyutu yok; A3 (420 × 297 mm) pafta oluşturuldu",
        );
        let mut names: std::collections::BTreeSet<String> = paper
            .iter()
            .map(|p| p.common.layout.clone())
            .chain(viewports.iter().map(|(_, groups)| text(groups, 410, dec)))
            .filter(|n| !n.is_empty())
            .collect();
        if names.is_empty() {
            names.insert("Layout1".into());
        }
        if names.len() > 256 {
            return Err("DXF pafta sınırı (256) aşıldı.".into());
        }
        for (i, name) in names.into_iter().enumerate() {
            layouts.insert(
                name.clone(),
                (
                    CadLayout {
                        name,
                        width_mm: 420.,
                        height_mm: 297.,
                        tab_order: i as u32 + 1,
                        ..Default::default()
                    },
                    1.,
                ),
            );
        }
    }
    let first = layouts
        .iter()
        .min_by_key(|(_, (l, _))| l.tab_order)
        .map(|(name, _)| name.clone())
        .unwrap_or_default();
    let mut objects: HashMap<String, Vec<Parsed>> = HashMap::new();
    for mut entity in paper {
        if entity.name == "VIEWPORT" {
            continue;
        }
        let name = if entity.common.layout.is_empty() {
            entity
                .common
                .owner
                .and_then(|h| lib.block_records.get(&h))
                .and_then(|b| owners.get(&b.to_uppercase()))
                .cloned()
                .unwrap_or_else(|| first.clone())
        } else {
            entity.common.layout.clone()
        };
        if !layouts.contains_key(&name) {
            report.skip(
                &entity.name,
                "kâğıt nesnesinin geçerli LAYOUT kaydı bulunamadı",
                entity.line,
            );
            continue;
        }
        entity.common.paper = false;
        objects.entry(name).or_default().push(entity);
    }
    for (line, groups) in viewports {
        let mut name = text(&groups, 410, dec);
        if name.is_empty() {
            name = groups
                .iter()
                .find(|p| p.code == 330)
                .and_then(|p| u64::from_str_radix(p.text().trim(), 16).ok())
                .and_then(|h| lib.block_records.get(&h))
                .and_then(|b| owners.get(&b.to_uppercase()))
                .cloned()
                .unwrap_or_else(|| first.clone());
        }
        let name = &name;
        let Some((layout, per)) = layouts.get_mut(name) else {
            report.skip("VIEWPORT", "görünümün paftası bulunamadı", line);
            continue;
        };
        let flags = groups
            .iter()
            .find(|p| p.code == 90)
            .and_then(|p| parse_int(p.text()))
            .unwrap_or(0);
        if flags & 1 != 0
            || num(&groups, 16, 0.) != 0.
            || num(&groups, 26, 0.) != 0.
            || num(&groups, 36, 1.) <= 0.
        {
            report.skip(
                "VIEWPORT",
                "perspektif/eğik 3B görünüm 2B pafta görünümüne aktarılmadı",
                line,
            );
            continue;
        }
        let width = num(&groups, 40, 0.) * *per;
        let height = num(&groups, 41, 0.) * *per;
        let view_height = model_scale.apply(num(&groups, 45, 0.));
        if !(width > 0. && height > 0. && view_height > 0.) {
            report.skip("VIEWPORT", "görünümün kâğıt/model boyutu geçersiz", line);
            continue;
        }
        let twist = num(&groups, 51, 0.).to_radians();
        let (sn, cs) = (crate::math::sin(twist), crate::math::cos(twist));
        let (cx, cy) = (num(&groups, 12, 0.), num(&groups, 22, 0.));
        layout.viewports.push(CadViewport {
            paper_center_mm: Vec2 {
                x: num(&groups, 10, 0.) * *per,
                y: num(&groups, 20, 0.) * *per,
            },
            width_mm: width,
            height_mm: height,
            view_center: Vec2 {
                x: model_scale.apply(num(&groups, 17, 0.) + cs * cx - sn * cy),
                y: model_scale.apply(num(&groups, 27, 0.) + sn * cx + cs * cy),
            },
            view_height,
            twist,
        });
        if flags & 65536 != 0 || groups.iter().any(|p| p.code == 331) {
            report.note("VIEWPORT", "özel kırpma veya görünümde dondurulmuş katmanlar aktarılmadı; dikdörtgen görünüm alındı", line);
        }
    }
    for (name, (layout, per)) in &mut layouts {
        let mut emitter = Emitter::new(lib, Out::new(1_000_000, 8_000_000), true);
        emitter.project_font = options
            .drawing_font
            .unwrap_or(kentos_contracts::DrawingFont::Barlow);
        for entity in objects.remove(name).unwrap_or_default() {
            emitter.emit(&entity, &Ctx::model());
        }
        emitter.notes_done();
        layout.entities = emitter.out.entities;
        for entity in &mut layout.entities {
            if entity.base().color.is_none() {
                entity.base_mut().color = lib
                    .layer_colors
                    .get(&entity.base().layer_id.to_uppercase())
                    .cloned();
            }
            units::entity(
                entity,
                units::Scale {
                    mul: *per,
                    div: 1000.,
                },
            );
        }
        report.absorb_losses(emitter.out.report.import());
    }
    let mut result: Vec<_> = layouts.into_values().map(|(layout, _)| layout).collect();
    // Many writers emit an empty default Layout1 even for model-only drawings.
    // It is not a user sheet until it has paper content or an actual model view.
    result.retain(|l| {
        !l.name.eq_ignore_ascii_case("Layout1") || !l.entities.is_empty() || !l.viewports.is_empty()
    });
    result.sort_by_key(|l| l.tab_order);
    if !result.is_empty() {
        report.fact(
            "Pafta",
            format!(
                "{} gerçek pafta, {} görünüm penceresi",
                result.len(),
                result.iter().map(|l| l.viewports.len()).sum::<usize>()
            ),
        );
    }
    Ok(result)
}
