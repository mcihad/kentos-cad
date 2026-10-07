//! `cad.entities.create` v1 (docs/adr/0057): new objects of any kind on a
//! named layer, as one undo step named “Ekle” or after the drawing tool.
//! The desktop's handler over the native document; the web's is
//! `apps/web/src/product/entitiesCreate.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.entities.create.json`.
//!
//! Elips, Eğri, Yardımcı çizgi, Işın, Halka, Paralel çizgi, Dik in, Dik çık
//! and Böl compute the geometry with the shared core and write it here
//! (TODOS.md CMD-07); nothing is computed in this module.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one object;
//! 2. every geometry, in order, by `cad.entities.edit`'s rules: enough
//!    points for its kind, every number finite, a circle's or an arc's
//!    radius above zero, an insert's scale above zero; its line weight
//!    from 0 to 100 mm when given; its link to the object whose label it
//!    writes, when given, a text's, whole and well formed (docs/adr/0175 §4);
//! 3. the expected revision, then the layer (every create command's, `checks.rs`);
//! 4. every insert's block is the drawing's (docs/adr/0144);
//! 5. every linked text's object is the drawing's.

use kentos_contracts::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntitiesCreatePlan,
    EntitiesCreated, Entity, EntityBase, EntityGeometry, EntityId, NewObject, label_scale_ok,
};
use kentos_domain::{Document, Uuid, labels};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::edit::{check_blocks, check_geometry, check_styles};
use crate::geometry::entity_of;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the objects as they would be stored, and
/// the revision to expect for exactly that; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesCreate,
) -> CommandResult<EntitiesCreatePlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: EntitiesCreatePlan {
                entities: entities(input),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the objects in their order as one undo
/// step (the document's own `add_many`), into the open transaction or group
/// if one is. Nothing is written when the document has no slot left for them.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesCreate,
) -> CommandResult<EntitiesCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    let slots = match cx.doc.add_many(entities(&input), label(input.operation)) {
        Ok(slots) => slots,
        Err(full) => {
            return CommandResult::Failed {
                error: error(codes::SLOTS_EXHAUSTED, full.to_string(), None),
            };
        }
    };
    let doc = &*cx.doc;
    CommandResult::Completed {
        output: EntitiesCreated {
            created: slots
                .iter()
                .filter_map(|slot| doc.uid(*slot))
                .map(|uid| uid.to_string())
                .collect(),
            ids: slots.iter().map(|slot| slot.0).collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// The undo step's name: the drawing tool's when it has its own, else the
/// document's “Ekle” (docs/adr/0057).
pub fn label(operation: Option<CreateOperation>) -> &'static str {
    match operation {
        None => labels::ADD,
        Some(CreateOperation::Parallel) => "Paralel çizgi",
        Some(CreateOperation::PerpendicularIn) => "Dik in",
        Some(CreateOperation::PerpendicularOut) => "Dik çık",
        Some(CreateOperation::Divide) => "Böl",
        Some(CreateOperation::Hatch) => "Tarama",
        // İçine tıklayarak alan (docs/adr/0065, 0069).
        Some(CreateOperation::Boundary) => "Alan oluştur",
        // The Hesap windows' “Çizime ekle”, named after the windows (docs/adr/0070, 0071).
        Some(CreateOperation::Traverse) => "Poligon hesabı",
        Some(CreateOperation::PolarSurvey) => "Kutupsal alım",
        Some(CreateOperation::ForwardIntersection) => "Önden kestirme",
        Some(CreateOperation::Resection) => "Geriden kestirme",
        // The drawing tools of docs/adr/0140.
        Some(CreateOperation::PointsBetween) => "Ara nokta",
        Some(CreateOperation::IntersectPoint) => "Kesişim noktası",
        Some(CreateOperation::DimensionChain) => "Zincir ölçü",
        Some(CreateOperation::DimensionBaseline) => "Baz ölçü",
        // Metin dosyası yerleştir (docs/adr/0145 §6).
        Some(CreateOperation::TextFile) => "Metin dosyası yerleştir",
        // Kılavuz (docs/adr/0146 §6).
        Some(CreateOperation::Leader) => "Kılavuz",
        // Toplu alan (docs/adr/0151).
        Some(CreateOperation::Polygonize) => "Toplu alan",
        // Köşelere nokta (docs/adr/0152).
        Some(CreateOperation::VertexPoints) => "Köşelere nokta",
        // Bitişik alan (docs/adr/0162 §3).
        Some(CreateOperation::Adjoin) => "Bitişik alan",
        // Etiketleri yazıya çevir (docs/adr/0175 §2).
        Some(CreateOperation::Labels) => "Etiketleri yazıya çevir",
        // Tablo ekle (docs/adr/0184 §6).
        Some(CreateOperation::Table) => "Tablo",
        // Koordinat yaz (docs/adr/0185 §1).
        Some(CreateOperation::Coordinates) => "Koordinat yaz",
        // Km yaz (docs/adr/0189 §5).
        Some(CreateOperation::Stations) => "Km yaz",
        Some(CreateOperation::Centerline) => "Orta hat",
        Some(CreateOperation::Image) => "Resim ekle",
        Some(CreateOperation::TextAlong) => "Eğri boyunca yazı",
        Some(CreateOperation::TangentLine) => "İki daireye teğet",
        Some(CreateOperation::FourthCorner) => "Dördüncü köşe",
        Some(CreateOperation::RangeRings) => "Menzil halkaları",
        Some(CreateOperation::PlanRoad) => "Plan yolu",
    }
}

/// The checks in the contract's order: why nothing may be written, or the
/// warnings when it may.
fn check(doc: &Document, input: &EntitiesCreate) -> Result<Vec<CommandWarning>, Stop> {
    if input.objects.is_empty() {
        return Err(Stop::Failed(error(
            codes::NO_OBJECTS,
            "Eklenecek nesne verilmedi. En az bir nesne verin.".into(),
            Some("objects".into()),
        )));
    }
    for (i, object) in input.objects.iter().enumerate() {
        check_geometry(&object.geometry, "objects", i, "nesnenin")?;
        checks::line_weight(object.line_weight, &format!("objects[{i}].lineWeight"))?;
        check_link(object, i)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    // An insert's block is the drawing's (docs/adr/0144).
    check_blocks(
        doc,
        input
            .objects
            .iter()
            .enumerate()
            .map(|(i, o)| (i, &o.geometry)),
        "objects",
    )?;
    // A text's and a dimension's style is the project's (docs/adr/0183 §9).
    check_styles(
        doc,
        input
            .objects
            .iter()
            .enumerate()
            .map(|(i, o)| (i, &o.geometry)),
        "objects",
    )?;
    // A linked text's object is the drawing's (docs/adr/0175 §4).
    for (i, object) in input.objects.iter().enumerate() {
        if let Some(of) = &object.label_of
            && Uuid::parse_str(of)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .is_none()
        {
            return Err(Stop::Failed(error(
                codes::LINK_NOT_FOUND,
                format!(
                    "“{of}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Çizimdeki bir nesnenin kimliğini verin."
                ),
                Some(format!("objects[{i}].labelOf")),
            )));
        }
    }
    Ok(warnings)
}

/// A new object's link to the object whose label it writes, when it has one
/// (docs/adr/0175 §4): a text's, both fields, the id a persistent id's text,
/// the scale finite and over 0 (`invalid_link`).
fn check_link(object: &NewObject, i: usize) -> Result<(), Stop> {
    let refuse = |field: &str, message: String| {
        Err(Stop::Failed(error(
            codes::INVALID_LINK,
            message,
            Some(format!("objects[{i}].{field}")),
        )))
    };
    let (of, scale) = (&object.label_of, object.label_scale);
    if of.is_none() && scale.is_none() {
        return Ok(());
    }
    if !matches!(object.geometry, EntityGeometry::Text { .. }) {
        let field = if of.is_some() {
            "labelOf"
        } else {
            "labelScale"
        };
        return refuse(
            field,
            format!(
                "Yalnız yazı bir nesnenin etiketine bağlanır; {}. nesne yazı değil. Bağı kaldırın.",
                i + 1
            ),
        );
    }
    let (Some(of), Some(scale)) = (of, scale) else {
        let field = if of.is_none() {
            "labelOf"
        } else {
            "labelScale"
        };
        return refuse(
            field,
            "Bağlı yazının nesnesi ve ölçeği birlikte verilir. İkisini birden verin ya da hiçbirini vermeyin.".into(),
        );
    };
    if !checks::is_uid_text(of) {
        return refuse(
            "labelOf",
            format!("Bağlı nesnenin kimliği küçük harfli, tireli bir UUID olmalı; “{of}” verildi."),
        );
    }
    if !label_scale_ok(scale) {
        return refuse(
            "labelScale",
            "Bağlı yazının ölçeği (1:N'deki N) sonlu ve sıfırdan büyük olmalı. Ölçeği düzeltin."
                .into(),
        );
    }
    // A linked text has no curve (docs/adr/0196 §1).
    if let EntityGeometry::Text { path: Some(_), .. } = &object.geometry {
        return Err(Stop::Failed(error(
            codes::INVALID_PATH,
            "Nesneye bağlı yazının eğrisi olmaz. Önce bağı koparın ya da eğriyi kaldırın.".into(),
            Some(format!("objects[{i}].geometry.path")),
        )));
    }
    Ok(())
}

/// The objects `input` describes, as the document stores them: slot 0 (given
/// when written), its own copies of every field.
fn entities(input: &EntitiesCreate) -> Vec<Entity> {
    input
        .objects
        .iter()
        .map(|object| {
            let mut entity = entity_of(
                &object.geometry,
                EntityBase {
                    id: 0,
                    layer_id: input.layer_id.clone(),
                    color: object.color.clone(),
                    attrs: object.attrs.clone().unwrap_or_default(),
                    label: object.label.clone(),
                    symbol: object.symbol.clone(),
                    line_weight: object.line_weight,
                },
            );
            // A linked text knows its object (docs/adr/0175 §4).
            if let Entity::Text(text) = &mut entity {
                text.label_of = object.label_of.as_deref().and_then(EntityId::parse);
                text.label_scale = object.label_of.as_ref().and(object.label_scale);
            }
            entity
        })
        .collect()
}
