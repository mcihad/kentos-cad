//! Annotation checks share the public dimension reader; no displayed number is treated as a measurement.
use kentos_commands::{
    DimensionBond, Entity, InspectionFinding, InspectionGroup, InspectionSeverity, Vec2,
};
use kentos_domain::{Document, Uuid};
use kentos_project::format::Format;

pub(super) fn finding(
    uid: Uuid,
    sources: &[String],
    code: &str,
    severity: InspectionSeverity,
    points: Vec<Vec2>,
    message: String,
    repair: &str,
) -> InspectionFinding {
    let primary = uid.to_string();
    let mut uids = vec![primary.clone()];
    uids.extend(
        sources
            .iter()
            .filter(|u| **u != primary)
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
    );
    InspectionFinding {
        group: InspectionGroup::Annotations,
        severity,
        code: code.into(),
        uids,
        points,
        message,
        repair: repair.into(),
        distance: None,
    }
}

/// A read-only convenience reader. Interactive queries use the cancellable context below.
pub fn findings(doc: &Document, uid: Uuid, e: &Entity) -> Vec<InspectionFinding> {
    let mut out = Vec::new();
    let _: Result<bool, std::convert::Infallible> =
        Context::default().visit(doc, uid, e, false, |f| {
            out.extend(f);
            Ok(true)
        });
    out
}

/// One immutable inspection reuses the definition library across all inserts.
#[derive(Default)]
pub(super) struct Context {
    blocks: Option<kentos_geometry_core::block::Blocks>,
}
impl Context {
    /// `None` lets the caller enforce its work/cancellation budget even on clean pieces.
    pub fn visit<E>(
        &mut self,
        doc: &Document,
        uid: Uuid,
        e: &Entity,
        include_hidden: bool,
        mut visit: impl FnMut(Option<InspectionFinding>) -> Result<bool, E>,
    ) -> Result<bool, E> {
        if !visit(None)? {
            return Ok(false);
        }
        for finding in own_findings(doc, uid, e) {
            if !visit(Some(finding))? {
                return Ok(false);
            }
        }
        if matches!(e, Entity::Insert(_)) {
            if !visit(None)? {
                return Ok(false);
            }
            let blocks = self
                .blocks
                .get_or_insert_with(|| crate::blocks::drawing_blocks(doc));
            return blocks.try_visit_annotations(&crate::geometry::shape(e), |piece| {
                let Some(piece) = piece else {
                    return visit(None);
                };
                let Some(origin) = &piece.annotation_origin else {
                    return Ok(true);
                };
                if !include_hidden
                    && !piece
                        .gates
                        .iter()
                        .all(|layer| doc.layers().is_visible(layer))
                {
                    return Ok(true);
                }
                for finding in piece_findings(doc, uid, origin, &piece.shape) {
                    if !visit(Some(finding))? {
                        return Ok(false);
                    }
                }
                Ok(true)
            });
        }
        Ok(true)
    }
}

/// The renderer already resolved transforms, dynamic states and clipping.
/// An insert owns the finding; its anonymous leaf never gets a fake UUID.
pub fn piece_findings(
    doc: &Document,
    owner: Uuid,
    origin: &kentos_geometry_core::block::AnnotationOrigin,
    shape: &kentos_geometry_core::entity::Shape,
) -> Vec<InspectionFinding> {
    let invalid = || {
        let b = kentos_geometry_core::entity::entity_bounds(shape);
        finding(owner,&[],"block_field_unverified",InspectionSeverity::Error,
            vec![Vec2{x:b.min_x+(b.max_x-b.min_x)*0.5,y:b.min_y+(b.max_y-b.min_y)*0.5}],
            "Blok parçasının yazı/alan kaynağı doğrulanamadı; saklanan değer güncel kabul edilemez.".into(),
            "Blok tanımını ve kaynağını inceleyin; doğrulanmadan çıktı teslim etmeyin.")
    };
    let Some(definition) =
        kentos_commands::BlockId::parse(&origin.definition).and_then(|id| doc.block(id))
    else {
        return vec![invalid()];
    };
    let Some(original) = definition.entities.get(origin.entity) else {
        return vec![invalid()];
    };
    let Some(placed) = crate::geometry::with_shape(original, shape.clone()) else {
        return vec![invalid()];
    };
    let mut out = match &placed {
        Entity::Text(t) => super::texts::placed_findings(doc, owner, t),
        Entity::Table(t) => super::texts::table_findings(doc, owner, t),
        _ => return vec![invalid()],
    };
    for f in &mut out {
        f.code = format!("block_{}", f.code);
        f.message = format!(
            "“{}” bloğu · {}. parça · {}",
            definition.name,
            origin.entity + 1,
            f.message
        );
        f.repair = "Blok tanımındaki yazıyı/hücreyi ve gerçek alan kaynaklarını inceleyin; tanımı açıkça düzenleyip yeniden denetleyin. Yerleştirme patlatılmaz veya sessizce değiştirilmez.".into();
    }
    out
}

fn own_findings(doc: &Document, uid: Uuid, e: &Entity) -> Vec<InspectionFinding> {
    let mut out = Vec::new();
    match e {
        Entity::Text(t) => out.extend(super::texts::findings(doc, uid, t)),
        Entity::Table(t) => out.extend(super::texts::table_findings(doc, uid, t)),
        Entity::Dimension(d) => {
            let read = crate::dimensions_query::read(&Format::of(doc.settings()), uid, d);
            let sources: Vec<_> = d
                .refs
                .iter()
                .flat_map(|r| r.each())
                .map(|(_, r)| Uuid::from_bytes(r.uid.0).to_string())
                .collect();
            let missing = sources.iter().any(|u| {
                Uuid::parse_str(u)
                    .ok()
                    .is_none_or(|u| doc.slot_of(u).is_none())
            });
            let broken_points: Vec<_> = d
                .refs
                .iter()
                .flat_map(|r| r.each())
                .filter(|(_, r)| !r.live() || doc.slot_of(Uuid::from_bytes(r.uid.0)).is_none())
                .map(|(name, _)| match name {
                    "b" => d.b,
                    "c" => d.c.unwrap_or(d.a),
                    _ => d.a,
                })
                .collect();
            let free_points: Vec<_> = [("a", Some(d.a)), ("b", Some(d.b)), ("c", d.c)]
                .into_iter()
                .filter(|(name, p)| {
                    p.is_some()
                        && !d
                            .refs
                            .iter()
                            .flat_map(|r| r.each())
                            .any(|(k, _)| k == *name)
                })
                .filter_map(|(_, p)| p)
                .collect();
            let point = if !broken_points.is_empty() {
                broken_points
            } else if read.bond == DimensionBond::Partial {
                free_points
            } else {
                vec![d.a]
            };
            let (code, severity, words) = if missing || read.bond == DimensionBond::Broken {
                (
                    "dimension_broken",
                    InspectionSeverity::Warning,
                    "Ölçünün kaynak bağı kopuk; saklanan geometri güncel kaynak ölçüsü sayılmaz.",
                )
            } else {
                match read.bond {
                    DimensionBond::Partial => (
                        "dimension_partial",
                        InspectionSeverity::Warning,
                        "Ölçü kısmen bağlı; serbest uç kaynak değişimini izlemez.",
                    ),
                    DimensionBond::Free => (
                        "dimension_free",
                        InspectionSeverity::Info,
                        "Ölçü serbest; kaynak değişimini izlemez.",
                    ),
                    _ => ("", InspectionSeverity::Info, ""),
                }
            };
            if !code.is_empty() {
                out.push(finding(uid, &sources, code, severity, point.clone(), words.into(), "Kaynağa git ile referansları inceleyin; Yeniden bağla ile ölçünün uçlarını doğru nesneye kenetleyin."));
            }
            // Its value in its style's SHX text style (docs/adr/1023), as a text is (ADR 1125): what the font
            // cannot draw is an error, a font the project does not embed a named substitute.
            if let Some(t) = crate::shx_text::dimension_value(
                doc.settings(),
                d.look.dim_style.as_deref(),
                d.a,
                0.0,
                d.height,
                &read.shown,
            ) && let Some(style) = crate::shx_text::style_of(doc.settings(), &t)
            {
                match crate::shx_text::Fonts::of(style, doc.styles()) {
                    Ok(fonts) => {
                        if let Err(why) = crate::shx_text::lay_out(&fonts, &t) {
                            out.push(finding(uid, &sources, "text_shx_unsupported", InspectionSeverity::Error, point.clone(),
                                format!("Ölçü değeri “{}” yazı stilinin gömülü SHX fontuyla çizilemiyor: {why} Ekranda yerine {} ile çizilir; çıktı teslim edilmez.",
                                    style.name, style.font.label()),
                                "Ölçü stilinin yazı stilini, eksik karakteri içeren bir SHX fontuna bağlayın ya da yazı stilini değiştirin; değer sessizce başka fonta çevrilmez."));
                        }
                    }
                    Err(why) => out.push(finding(uid, &sources, "text_font_substituted", InspectionSeverity::Warning, point.clone(),
                        format!("Özgün font: {} · Çizilen font: {}. Ölçü değerinin SHX fontu çizilemiyor: {why}",
                            style.font_file.as_deref().unwrap_or_default(), style.font.label()),
                        "SHX kaynağını Çizgi tipleri › SHX ekle… ya da cad.library.edit ile projeye ekleyin; ölçü değeri o zaman özgün fontla çizilir.")),
                }
            }
            if !read.measured.is_finite() {
                out.push(finding(
                    uid,
                    &sources,
                    "dimension_invalid",
                    InspectionSeverity::Error,
                    point.clone(),
                    "Ölçünün geometrisinden sonlu bir değer hesaplanamıyor.".into(),
                    "Ölçünün kaynak noktalarını ve türünü düzeltin.",
                ));
            } else if read.overridden {
                let actual = match read.unit.as_str() {
                    "length" | "coordinate" => Format::of(doc.settings()).length(read.measured),
                    "angle" => Format::of(doc.settings()).angle(read.measured),
                    _ => format!("{} %", read.measured),
                };
                out.push(finding(uid, &sources, "dimension_override", InspectionSeverity::Warning, point,
                    format!("Ölçü yazısı elle değiştirilmiş. Gerçek: {actual} · Gösterilen: {}", read.shown),
                    "Öznitelikler'de ölçü yazısını kontrol edin; ölçülen değeri göstermek için <> kullanın veya yazı değişikliğini kaldırın."));
            }
        }
        Entity::Leader(l) => {
            // Its note in its leader style's SHX text style (docs/adr/1025), as a text is (ADR 1125).
            if let Some(note) = l.text.as_deref()
                && let Some(t) = crate::shx_text::leader_note(
                    doc.settings(),
                    l.leader_style.as_deref(),
                    l.pts.last().copied().unwrap_or(kentos_commands::Vec2 { x: 0.0, y: 0.0 }),
                    l.rotation,
                    l.height,
                    note,
                )
                && let Some(style) = crate::shx_text::style_of(doc.settings(), &t)
            {
                let at = vec![t.p];
                match crate::shx_text::Fonts::of(style, doc.styles()) {
                    Ok(fonts) => {
                        if let Err(why) = crate::shx_text::lay_out(&fonts, &t) {
                            out.push(finding(uid, &[], "text_shx_unsupported", InspectionSeverity::Error, at,
                                format!("Kılavuz notu “{}” yazı stilinin gömülü SHX fontuyla çizilemiyor: {why} Ekranda yerine yazı tipiyle çizilir; çıktı teslim edilmez.",
                                    style.name),
                                "Kılavuz stilinin yazı stilini eksik karakteri içeren bir SHX fontuna bağlayın ya da yazı stilini değiştirin; not sessizce başka fonta çevrilmez."));
                        }
                    }
                    Err(why) => out.push(finding(uid, &[], "text_font_substituted", InspectionSeverity::Warning, at,
                        format!("Özgün font: {} · Çizilen font: {}. Kılavuz notunun SHX fontu çizilemiyor: {why}",
                            style.font_file.as_deref().unwrap_or_default(), style.font.label()),
                        "SHX kaynağını Çizgi tipleri › SHX ekle… ya da cad.library.edit ile projeye ekleyin; not o zaman özgün fontla çizilir.")),
                }
            }
            let arms = std::iter::once((l.tip.as_ref(), l.pts.first()))
                .chain(l.arms.iter().map(|a| (a.tip.as_ref(), a.pts.first())));
            let mut sources = Vec::new();
            let mut broken = Vec::new();
            let mut free = 0;
            let mut total = 0;
            for (tip, at) in arms {
                total += 1;
                match tip {
                    None => free += 1,
                    Some(r) => {
                        let source = Uuid::from_bytes(r.uid.0);
                        sources.push(source.to_string());
                        if !r.live() || doc.slot_of(source).is_none() {
                            broken.extend(at.copied());
                        }
                    }
                }
            }
            if !broken.is_empty() {
                out.push(finding(uid, &sources, "leader_broken", InspectionSeverity::Warning, broken,
                    "Kılavuzun en az bir kolunun kaynak bağı kopuk; ok ucu son yerinde kalıyor.".into(),
                    "Açıklama → Kılavuz ucunu bağla ile sorunlu kolun ucunu doğru nesneye kenetleyin."));
            }
            if free > 0 {
                out.push(finding(uid, &sources, "leader_free", if free == total { InspectionSeverity::Info } else { InspectionSeverity::Warning }, l.pts.first().copied().into_iter().collect(),
                    format!("Kılavuzun {total} kolundan {free} tanesi serbest; bu uçlar kaynak değişimini izlemez."),
                    "Serbest uçlar tasarım kararı olabilir; gerekiyorsa Açıklama → Kılavuz ucunu bağla kullanın."));
            }
        }
        _ => {}
    }
    out
}
