//! Candidate recognition over the shared analytic matcher. Read only, cancellable and revision guarded.
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
};
use kentos_commands::{
    BlockMatch, BlocksFind, BlocksFound, Bounds, CommandResult, CommandWarning, Entity,
    RECOGNITION_SCOPE_MAX, RECOGNITION_SOURCE_MAX, RECOGNITION_TOLERANCE, Vec2, codes,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::{Shape, entity_bounds_in};
mod definitions;
use kentos_geometry_core::ops::repeats::{self, Options};
use std::collections::{BTreeMap, HashMap, HashSet};

fn refuse(code: &str, message: &str, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn named(doc: &Document, list: &[String], path: &str) -> Result<Vec<Uuid>, Stop> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (i, text) in list.iter().enumerate() {
        let uid = Uuid::parse_str(text)
            .ok()
            .filter(|u| u.to_string() == *text)
            .ok_or_else(|| {
                refuse(
                    codes::INVALID_UID,
                    "Nesne kimliği küçük harfli, tireli UUID olmalı.",
                    &format!("{path}[{i}]"),
                )
            })?;
        if doc.slot_of(uid).and_then(|s| doc.get(s)).is_none() {
            return Err(refuse(
                codes::ENTITY_NOT_FOUND,
                "Aranacak nesne çizimde yok; kaynağı ve kapsamı yeniden seçin.",
                &format!("{path}[{i}]"),
            ));
        }
        if seen.insert(uid) {
            out.push(uid);
        }
    }
    Ok(out)
}

struct Checked {
    source: Vec<Uuid>,
    search: HashSet<Uuid>,
    options: Options,
}

// Shape deliberately omits route/elevation and architectural-role data. Such a path must
// not silently become an ordinary 2D source, even under geometry-only matching.
fn recognizable(e: &Entity) -> bool {
    match e {
        Entity::Line(_) | Entity::Circle(_) | Entity::Arc(_) => {
            repeats::supported(&crate::geometry::shape(e))
        }
        Entity::Text(t) if t.label_of.is_none() && t.label_scale.is_none() && t.annotative.is_none() && t.paragraph.fields.is_empty() => repeats::supported(&crate::geometry::shape(e)),
        Entity::Insert(i) if i.arch==Default::default() => repeats::supported(&crate::geometry::shape(e)),
        Entity::Polyline(p)
            if p.pts.len() <= repeats::PATH_VERTEX_LIMIT
                && p.zs.as_ref().is_none_or(|v| v.iter().all(Option::is_none))
                && p.ms.as_ref().is_none_or(|v| v.iter().all(Option::is_none))
                && p.space_separator.is_none()
                && p.parts.as_ref().is_none_or(|v| v.is_empty())
                && p.holes.as_ref().is_none_or(|v| v.is_empty()) =>
        {
            repeats::supported(&crate::geometry::shape(e))
        }
        _ => false,
    }
}

fn unsupported_reason(e: &Entity) -> &'static str {
    match e {
        Entity::Text(_) => "Yazı tek satırlı ve en çok 4096 bayt olmalı; etiket, alan, ölçek veya eğri bağı sessizce atlanmaz. Bağsız düz bir yazı seçin.",
        Entity::Insert(_) => "Blok kaynağı farklı Y ölçeği, parametre, kırpma veya mimari kayıt taşıyor. Bu kapsamda bu davranışlar yok sayılmaz.",
        Entity::Polyline(p) if p.zs.as_ref().is_some_and(|v| v.iter().any(Option::is_some)) => {
            "Kaynak çoklu çizgi yükseklik (Z) verisi içeriyor. Bu arama 2B çizgiler içindir; Z verisi atlanmadı. Yalnız 2B nesneleri kaynak seçin."
        }
        Entity::Polyline(p) if p.ms.as_ref().is_some_and(|v| v.iter().any(Option::is_some)) => {
            "Kaynak çoklu çizgi güzergâh ölçüsü (M) içeriyor. Bu veri sıradan 2B geometri sayılmadı. M verisi olmayan bir bileşeni kaynak seçin."
        }
        Entity::Polyline(p) if p.space_separator.is_some() => {
            "Kaynak çoklu çizgi mahal ayırıcıdır. Mahal bağlantısı yok sayılmadı. Bu arama için sıradan bir 2B bileşen seçin."
        }
        Entity::Polyline(p)
            if p.parts.as_ref().is_some_and(|v| !v.is_empty())
                || p.holes.as_ref().is_some_and(|v| !v.is_empty()) =>
        {
            "Kaynak çoklu çizgi ek parça veya delik içeriyor. Bu arama tek parçalı, deliksiz 2B çoklu çizgileri destekler; hiçbir parça atlanmadı."
        }
        Entity::Polyline(p) if p.pts.len() > repeats::PATH_VERTEX_LIMIT => {
            "Kaynak çoklu çizgi 4 096 köşe sınırını aşıyor. Geometri sadeleştirilmedi. Daha küçük bir bileşeni kaynak seçin."
        }
        Entity::Polyline(_) => {
            "Kaynak çoklu çizginin köşe, yay veya kenar genişliği bilgisi bu aramaya uygun değil. Sıfır uzunluklu geometri atlanmaz; kaynağı kontrol edin."
        }
        _ => {
            "Kaynak öğe bu aramada desteklenmiyor. Çizgi, 2B çoklu çizgi, çember, yay, düz yazı ve statik blok seçilebilir. Desteklenmeyen veya sıfır uzunluklu öğe atlanmadı."
        }
    }
}

fn check(doc: &Document, input: &BlocksFind) -> Result<Checked, Stop> {
    if input.source.is_empty() {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Tekrarı aranacak tek bir 2B bileşenin parçalarını seçin.",
            "source",
        ));
    }
    if input.source.len() > RECOGNITION_SOURCE_MAX {
        return Err(refuse(
            codes::RECOGNITION_SOURCE_LIMIT,
            "Kaynak en çok 128 nesne olabilir; tek bir bileşen seçin.",
            "source",
        ));
    }
    let tolerance = input.tolerance.unwrap_or(RECOGNITION_TOLERANCE);
    if !(tolerance.is_finite() && tolerance > 0.0 && tolerance <= 0.001) {
        return Err(refuse(
            codes::INVALID_TOLERANCE,
            "Tolerans sıfırdan büyük ve en çok 0,001 metre olmalı.",
            "tolerance",
        ));
    }
    let limit = input.limit.unwrap_or(1000);
    if !(1..=1000).contains(&limit) {
        return Err(refuse(
            codes::INVALID_LIMIT,
            "Eşleşme sınırı 1 ile 1000 arasında olmalı.",
            "limit",
        ));
    }
    if input
        .uids
        .as_ref()
        .is_some_and(|u| u.len() > RECOGNITION_SCOPE_MAX)
        || (input.uids.is_none()
            && doc.entities().take(RECOGNITION_SCOPE_MAX + 1).count() > RECOGNITION_SCOPE_MAX)
    {
        return Err(refuse(
            codes::RECOGNITION_SCOPE_LIMIT,
            "Arama kapsamı en çok 50 000 nesne olabilir; kapsamı daraltın.",
            "uids",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let source = named(doc, &input.source, "source")?;
    for (i, uid) in source.iter().enumerate() {
        let e = doc.slot_of(*uid).and_then(|s| doc.get(s)).ok_or_else(|| {
            refuse(
                codes::ENTITY_NOT_FOUND,
                "Kaynak çizimde yok; kaynağı yeniden seçin.",
                &format!("source[{i}]"),
            )
        })?;
        if !recognizable(e) {
            return Err(refuse(
                codes::RECOGNITION_UNSUPPORTED,
                unsupported_reason(e),
                &format!("source[{i}]"),
            ));
        }
    }
    let search: HashSet<_> = match &input.uids {
        Some(list) => named(doc, list, "uids")?.into_iter().collect(),
        None => doc.entities_with_uids().map(|(uid, _)| uid).collect(),
    };
    let originals: HashSet<_> = source.iter().copied().collect();
    let mut vertices = 0usize;
    for (uid, e) in doc.entities_with_uids() {
        if (search.contains(&uid) || originals.contains(&uid))
            && let Entity::Polyline(p) = e
        {
            vertices = vertices.saturating_add(p.pts.len());
            if vertices > repeats::VERTEX_LIMIT {
                return Err(refuse(
                    codes::RECOGNITION_SCOPE_LIMIT,
                    "Arama ve kaynak en çok toplam 200 000 çoklu çizgi köşesi içerebilir. Kapsamı daraltın.",
                    "uids",
                ));
            }
        }
    }
    Ok(Checked {
        source,
        search,
        options: Options {
            rotations: input.rotations.unwrap_or(true),
            mirrors: input.mirrors.unwrap_or(false),
            scales: input.scales.unwrap_or(false),
            text_variants: input.text_variants.unwrap_or(false),
            attribute_variants: input.attribute_variants.unwrap_or(false),
            tolerance,
            limit: limit as usize,
            ..Default::default()
        },
    })
}

fn find_in(
    doc: &Document,
    input: &BlocksFind,
    mut stopped: impl FnMut() -> bool,
) -> Result<BlocksFound, Stop> {
    if stopped() {
        return Err(Stop::Cancelled);
    }
    let checked = check(doc, input)?;
    let originals: HashSet<_> = checked.source.iter().copied().collect();
    let mut records = Vec::new();
    let mut scene: Vec<Shape> = Vec::new();
    let mut properties = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        if !checked.search.contains(&uid) && !originals.contains(&uid) {
            continue;
        }
        if !recognizable(e) {
            continue;
        }
        records.push((uid, e));
        scene.push(crate::geometry::shape(e));
        let mut p = e.base().clone();
        p.id = 0;
        if input.attribute_variants.unwrap_or(false)
            && let Entity::Insert(i) = e
            && let Some(block) = doc.block(i.block) {
                for a in &block.attributes {p.attrs.remove(&a.tag);}
            }
        properties.push(p);
    }
    let by_uid: HashMap<_, _> = records
        .iter()
        .enumerate()
        .map(|(i, (u, _))| (*u, i))
        .collect();
    let source: Vec<_> = checked.source.iter().map(|u| by_uid[u]).collect();
    let geometry_only = input.geometry_only.unwrap_or(false);
    let found = repeats::find(
        &scene,
        &source,
        checked.options,
        |i, j| {
            checked.search.contains(&records[j].0)
                && (geometry_only || properties[i] == properties[j])
        },
        &mut stopped,
    )
    .map_err(|e| match e {
        repeats::Error::Cancelled => Stop::Cancelled,
        repeats::Error::GeometryLimit => refuse(
            codes::RECOGNITION_SCOPE_LIMIT,
            "Aramadaki toplam çoklu çizgi köşe sınırı aşıldı; kapsamı daraltın.",
            "uids",
        ),
        _ => refuse(
            codes::RECOGNITION_UNSUPPORTED,
            "Kaynağın analitik yönü veya boyutu bulunamadı; başka bir bileşen seçin.",
            "source",
        ),
    })?;
    let roots: HashSet<_> = found.matches.iter().flat_map(|m| m.members.iter().copied()).chain(source.iter().copied()).collect();
    let blocks = definitions::checked(doc, roots.iter().filter_map(|i| match records[*i].1 {
        Entity::Insert(e) => Some(e.block), _ => None,
    }), &mut stopped)?;
    let font = crate::geometry::drawing_font(doc.settings().drawing_font);
    let mut boxes = HashMap::new();
    for i in roots {
        if stopped() { return Err(Stop::Cancelled); }
        let mut b = entity_bounds_in(&scene[i],font);
        if matches!(scene[i],Shape::Insert {..}) {
            b = kentos_geometry_core::block::pieces_bounds(b,&blocks.expand(&scene[i]),font);
        }
        if ![b.min_x,b.min_y,b.max_x,b.max_y].iter().all(|v|v.is_finite()) {
            return Err(refuse(codes::RECOGNITION_UNSUPPORTED,"Adayın çizim sınırı sonlu değil; kaynak değiştirilmedi.","source"));
        }
        boxes.insert(i,b);
    }
    let mut used = HashMap::new();
    for group in &found.matches {
        for &i in &group.members {
            *used.entry(i).or_insert(0usize) += 1;
        }
    }
    let mut links = kentos_domain::copy_links::Copies::new();
    let mut matches = Vec::new();
    for group in found.matches {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        let mut bounds = Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for &i in &group.members {
            let b = boxes[&i];
            bounds.min_x = bounds.min_x.min(b.min_x);
            bounds.min_y = bounds.min_y.min(b.min_y);
            bounds.max_x = bounds.max_x.max(b.max_x);
            bounds.max_y = bounds.max_y.max(b.max_y);
            links.insert(*records[i].0.as_bytes(), *records[i].0.as_bytes());
        }
        matches.push(BlockMatch {
            overlaps: group.members.iter().any(|i| used[i] > 1),
            members: group
                .members
                .iter()
                .map(|i| records[*i].0.to_string())
                .collect(),
            origin: Vec2 {
                x: group.origin.x,
                y: group.origin.y,
            },
            at: Vec2 {
                x: group.at.x,
                y: group.at.y,
            },
            rotation: group.rotation,
            scale: group.scale,
            mirror: group.mirror,
            bounds,
        });
    }
    let mut related = BTreeMap::new();
    for e in doc.entities() {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        if kentos_domain::copy_links::links_to(e, &links) {
            *related.entry(e.kind().to_owned()).or_insert(0) += 1;
        }
    }
    Ok(BlocksFound {
        source: checked.source.iter().map(Uuid::to_string).collect(),
        matches,
        truncated: found.truncated,
        comparisons: found.comparisons,
        related,
        revision: doc.revision().to_string(),
    })
}

/// The same query for the desktop's cancellable worker and command hosts. Never writes to the document.
pub fn inspect(
    doc: &Document,
    input: &BlocksFind,
    stopped: impl FnMut() -> bool,
) -> CommandResult<BlocksFound> {
    match find_in(doc, input, stopped) {
        Ok(output) => {
            let mut warnings = Vec::new();
            if output.truncated {
                warnings.push(CommandWarning { code:codes::RECOGNITION_TRUNCATED.into(),message:"Arama sınıra ulaştı; bulunanlar kısmi sonuçtur. Kapsamı daraltıp yeniden arayın.".into(),path:Some("uids".into()) });
            }
            if input.geometry_only == Some(true) {
                warnings.push(CommandWarning { code:codes::RECOGNITION_GEOMETRY_ONLY.into(),message:"Yalnız geometri karşılaştırıldı; katman, görünüş ve öznitelik farkları eşleşmeye engel sayılmadı.".into(),path:Some("geometryOnly".into()) });
            }
            for (on,path,message) in [
                (input.text_variants,"textVariants","Yazı değerleri farklı olabilir; konum, yön, yükseklik ve yazı biçimi doğrulandı. Değerler silinmedi; dönüştürme ayrı incelemedir."),
                (input.attribute_variants,"attributeVariants","Yalnız blokta tanımlı öznitelik etiketlerinin değer farkları göz ardı edildi. Diğer nesne verileri ve blok tanımı aynı kalmalıdır.")
            ] { if on==Some(true) {warnings.push(CommandWarning {code:codes::RECOGNITION_GEOMETRY_ONLY.into(),message:message.into(),path:Some(path.into())});} }
            CommandResult::Completed { output, warnings }
        }
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksFind) -> CommandResult<()> {
    if cx.stopped() {
        return CommandResult::Cancelled;
    }
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksFind) -> CommandResult<BlocksFound> {
    inspect(cx.doc, input, || cx.stopped())
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: BlocksFind) -> CommandResult<BlocksFound> {
    plan(cx, &input)
}
