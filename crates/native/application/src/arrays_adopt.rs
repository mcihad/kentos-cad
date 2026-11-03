//! Metadata-only adoption of reviewed placements. UUIDs, slots, geometry and references stay intact.
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
};
use kentos_commands::{
    ArrayId, ArrayItem, ArrayLayout, ArraysAdopt, ArraysAdoptPlan, ArraysAdopted, AssocArray,
    CommandResult, Entity, InsertEntity, array_places, codes,
};
use kentos_domain::arrays::placements;
use kentos_domain::{Document, Uuid};
use std::collections::HashSet;

pub const LABEL: &str = "Mevcut blokları diziye bağla";
const TOLERANCE: f64 = 1e-6;

fn refused(message: impl Into<String>, path: impl Into<String>) -> Stop {
    Stop::failed(error(
        codes::ARRAY_ADOPTION_REFUSED,
        message.into(),
        Some(path.into()),
    ))
}

fn insert<'a>(doc: &'a Document, uid: Uuid, path: &str) -> Result<&'a InsertEntity, Stop> {
    let e = doc.slot_of(uid).and_then(|s| doc.get(s)).ok_or_else(|| {
        Stop::failed(error(
            codes::ENTITY_NOT_FOUND,
            "Yerleştirme çizimde bulunamadı; incelemeyi yenileyin.".into(),
            Some(path.into()),
        ))
    })?;
    let Entity::Insert(i) = e else {
        return Err(refused(
            "Yalnız mevcut blok yerleştirmeleri diziye bağlanabilir.",
            path,
        ));
    };
    if doc.layers().is_locked(&i.base.layer_id) {
        return Err(Stop::failed(error(
            codes::LAYER_LOCKED,
            "Yerleştirmenin katmanı kilitli; çizim değiştirilmedi.".into(),
            Some(path.into()),
        )));
    }
    Ok(i)
}

/// The first delivery accepts data-free inserts with a common reproducible appearance.
/// Their definition may contain other geometry: no definition or nested object is rewritten.
fn plain(i: &InsertEntity) -> bool {
    i.y_scale.is_none()
        && i.parameters.is_empty()
        && i.attribute_places.is_empty()
        && i.clipping.is_none()
        && i.arch == Default::default()
        && i.base.attrs.is_empty()
        && i.base.label.is_none()
        && i.base.symbol.is_none()
        && i.base.label_pins.is_empty()
        && i.base.attachments.is_empty()
}

fn same_appearance(a: &InsertEntity, b: &InsertEntity) -> bool {
    a.base.layer_id == b.base.layer_id
        && a.base.color == b.base.color
        && a.base.line_weight == b.base.line_weight
        && a.base.look == b.base.look
}

fn check(
    doc: &Document,
    input: &ArraysAdopt,
    stopped: &mut impl FnMut() -> bool,
) -> Result<ArraysAdoptPlan, Stop> {
    if stopped() {
        return Err(Stop::Cancelled);
    }
    if !checks::is_uid_text(&input.source) {
        return Err(Stop::failed(error(
            codes::INVALID_UID,
            "Kaynak kimliği küçük harfli, tireli UUID olmalı.".into(),
            Some("source".into()),
        )));
    }
    crate::array::check_layout(&input.layout)?;
    let tolerance = input.tolerance.unwrap_or(TOLERANCE);
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(Stop::failed(error(
            codes::INVALID_TOLERANCE,
            "Konum toleransı metre cinsinden sonlu ve sıfırdan büyük olmalı.".into(),
            Some("tolerance".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let count = array_places(&input.layout) as usize;
    if input.items.len() != count {
        return Err(refused(
            "Her dizi yeri için bir öğe verin; eksik yerde boş öğe kullanın.",
            "items",
        ));
    }
    let source_uid = Uuid::parse_str(&input.source)
        .map_err(|_| refused("Kaynak kimliği okunamadı.", "source"))?;
    let source = insert(doc, source_uid, "source")?;
    if input.items.first()
        != Some(&ArrayItem {
            uid: Some(kentos_commands::EntityId(source_uid.into_bytes())),
            kept: false,
        })
    {
        return Err(refused(
            "Kaynak, sıfır numaralı yerde ve korunmayan öğe olmalı.",
            "items[0]",
        ));
    }
    if !plain(source) {
        return Err(refused(
            "Bu kapsam veri, mimari kayıt, farklı Y ölçeği veya kırpma taşıyan yerleştirmeleri desteklemiyor; hiçbir veri atılmadı.",
            "source",
        ));
    }
    let definition = doc
        .blocks()
        .iter()
        .find(|b| b.id == source.block)
        .ok_or_else(|| refused("Kaynağın blok tanımı çizimde yok.", "source"))?;
    if definition.dynamic.is_some()
        || definition.xref.is_some()
        || !definition.attributes.is_empty()
    {
        return Err(refused(
            "Dinamik, dış referanslı veya öznitelik tanımlı blok bu kapsamda benimsetilmez.",
            "source",
        ));
    }
    // Index only selected identities, once. Calling array_of_item for every proposed
    // item scans every existing array repeatedly on large production drawings.
    let selected: HashSet<_> = input.items.iter().filter_map(|i| i.uid).collect();
    let mut owned = HashSet::new();
    for a in doc.arrays() {
        for i in &a.items {
            if stopped() {
                return Err(Stop::Cancelled);
            }
            if let Some(uid) = i.uid.filter(|u| selected.contains(u)) {
                owned.insert(uid);
            }
        }
    }
    if owned.contains(&kentos_commands::EntityId(source_uid.into_bytes())) {
        return Err(refused(
            "Kaynak zaten başka bir ilişkili dizinin öğesi.",
            "source",
        ));
    }
    let array = AssocArray {
        id: ArrayId([0; 16]),
        block: source.block,
        layer_id: source.base.layer_id.clone(),
        p: source.p,
        scale: source.scale,
        rotation: source.rotation,
        mirror: source.mirror,
        layout: input.layout.clone(),
        items: input.items.clone(),
    };
    crate::array::path_maps(doc, &array.layout)?;
    let path = if let ArrayLayout::Path { path, .. } = &array.layout {
        Uuid::parse_str(path)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| doc.get(s))
    } else {
        None
    };
    let places = placements(&array, path)
        .ok_or_else(|| refused("Dizinin yerleri sonlu olarak hesaplanamadı.", "layout"))?;
    if places.len() != count {
        return Err(refused(
            "Yerleşim beklenen sayıda dizi yeri üretmedi.",
            "layout",
        ));
    }
    let mut seen = HashSet::new();
    let (mut regular, mut missing, mut kept, mut deviation) = (0_u32, 0_u32, 0_u32, 0.0_f64);
    for (k, (item, at)) in input.items.iter().zip(&places).enumerate() {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        // Missing and explicitly kept places must also have finite proposed poses;
        // otherwise their later reset would create overflowing geometry.
        if !at.p.x.is_finite()
            || !at.p.y.is_finite()
            || !at.rotation.is_finite()
            || !at.scale.is_finite()
            || at.scale <= 0.0
        {
            return Err(refused(
                "Dizi yerlerinden biri sayı sınırını aşıyor; adet veya aralığı küçültün.",
                "layout",
            ));
        }
        let field = format!("items[{k}]");
        let Some(uid) = item.uid else {
            if item.kept {
                return Err(refused("Eksik öğe korunmuş olarak işaretlenemez.", field));
            }
            missing += 1;
            continue;
        };
        if !seen.insert(uid) {
            return Err(refused(
                "Bir yerleştirme iki dizi yerine bağlanamaz.",
                field,
            ));
        }
        if owned.contains(&uid) {
            return Err(refused(
                "Yerleştirme zaten başka bir ilişkili dizinin öğesi.",
                field,
            ));
        }
        let actual = insert(doc, Uuid::from_bytes(uid.0), &field)?;
        if !plain(actual) || !same_appearance(source, actual) {
            return Err(refused(
                "Yerleştirmelerin katmanı, görünüşü ve veri kapsamı ortak şablona uymuyor; veri veya görünüş değiştirilmedi.",
                field,
            ));
        }
        if item.kept {
            kept += 1;
            continue;
        }
        let distance = (actual.p.x - at.p.x).hypot(actual.p.y - at.p.y);
        let angle = (actual.rotation - at.rotation).rem_euclid(std::f64::consts::TAU);
        let angle = angle.min(std::f64::consts::TAU - angle);
        if actual.block != source.block
            || actual.mirror != at.mirror
            || !distance.is_finite()
            || distance > tolerance
            || !angle.is_finite()
            || angle > 1e-10
            || !actual.scale.is_finite()
            || actual.scale <= 0.0
            || (actual.scale - at.scale).abs() > at.scale.abs() * 1e-12
        {
            return Err(refused(
                "Yerleştirme incelenen örüntüye uymuyor. Yerleşimi düzeltin veya bu öğeyi açıkça korumaya alın; nesne kendiliğinden taşınmadı.",
                field,
            ));
        }
        deviation = deviation.max(distance);
        regular += 1;
    }
    if regular < 2 {
        return Err(refused(
            "En az iki normal öğe gerekli; bütün öğeler korumaya alınarak örüntü oluşturulamaz.",
            "items",
        ));
    }
    Ok(ArraysAdoptPlan {
        array,
        deviation,
        missing,
        kept,
        revision: doc.revision().to_string(),
    })
}

pub fn inspect(
    doc: &Document,
    input: &ArraysAdopt,
    mut stopped: impl FnMut() -> bool,
) -> CommandResult<ArraysAdoptPlan> {
    match check(doc, input, &mut stopped) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &ArraysAdopt) -> CommandResult<ArraysAdoptPlan> {
    inspect(cx.doc, input, || cx.stopped())
}
pub fn validate(cx: &ExecutionContext<'_>, input: &ArraysAdopt) -> CommandResult<()> {
    match check(cx.doc, input, &mut || cx.stopped()) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
pub fn execute(cx: &mut ExecutionContext<'_>, input: ArraysAdopt) -> CommandResult<ArraysAdopted> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ARRAYS_ADOPT,
        kentos_commands::CAD_ARRAYS_ADOPT_VERSION,
        input,
        write,
    )
}
fn write(cx: &mut ExecutionContext<'_>, input: ArraysAdopt) -> CommandResult<ArraysAdopted> {
    let mut plan = match check(cx.doc, &input, &mut || cx.stopped()) {
        Ok(plan) => plan,
        Err(stop) => return stop.into(),
    };
    if cx.stopped() {
        return CommandResult::Cancelled;
    }
    plan.array.id = ArrayId(Uuid::now_v7().into_bytes());
    let id = plan.array.id;
    let retained = plan
        .array
        .items
        .iter()
        .filter_map(|i| i.uid.map(|u| u.to_text()))
        .collect();
    if let Err(e) = cx.doc.transact(LABEL, |doc| doc.add_array(plan.array)) {
        return refused(e.to_string(), "items").into();
    }
    CommandResult::Completed {
        output: ArraysAdopted {
            array: id,
            retained,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
