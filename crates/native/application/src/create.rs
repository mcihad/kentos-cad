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
//! 5. every linked text's object is the drawing's;
//! 6. the attributes given, by the layer's fields (docs/adr/0199 §2), objects
//!    in order, names in theirs: written in their canonical text, and the
//!    fields' defaults fill what an object does not give.

use std::collections::BTreeMap;

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
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
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
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (a command that creates through this one, docs/adr/0254).
pub fn plan_in(doc: &Document, input: &EntitiesCreate) -> CommandResult<EntitiesCreatePlan> {
    match check(doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: EntitiesCreatePlan {
                entities: entities(input, &checked.attrs, &checked.placed),
                revision: doc.revision().to_string(),
            },
            warnings: checked.warnings,
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
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_ENTITIES_CREATE,
        kentos_contracts::CAD_ENTITIES_CREATE_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesCreate,
) -> CommandResult<EntitiesCreated> {
    let Checked {
        warnings,
        attrs,
        placed,
    } = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let slots = match cx
        .doc
        .add_many(entities(&input, &attrs, &placed), label(input.operation))
    {
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
        Some(CreateOperation::Raster) => "Raster ekle",
        Some(CreateOperation::PointCloud) => "Nokta bulutu ekle",
        Some(CreateOperation::TextAlong) => "Eğri boyunca yazı",
        Some(CreateOperation::TangentLine) => "İki daireye teğet",
        Some(CreateOperation::FourthCorner) => "Dördüncü köşe",
        Some(CreateOperation::RangeRings) => "Menzil halkaları",
        Some(CreateOperation::PlanRoad) => "Plan yolu",
        // Yatay ağ dengelemesi (docs/adr/0203 §8).
        Some(CreateOperation::NetworkAdjust) => "Yatay ağ dengelemesi",
    }
}

/// What the checks give when the input may be written: the warnings, and
/// each object's attributes as they will be stored.
struct Checked {
    warnings: Vec<CommandWarning>,
    attrs: Vec<BTreeMap<String, String>>,
    /// Each object's measured points read from the places its live references name (docs/adr/0250).
    placed: Vec<Vec<(&'static str, kentos_contracts::Vec2)>>,
}

/// The checks in the contract's order: why nothing may be written, or what
/// will be when it may.
fn check(doc: &Document, input: &EntitiesCreate) -> Result<Checked, Stop> {
    if input.objects.is_empty() {
        return Err(Stop::failed(error(
            codes::NO_OBJECTS,
            "Eklenecek nesne verilmedi. En az bir nesne verin.".into(),
            Some("objects".into()),
        )));
    }
    for (i, object) in input.objects.iter().enumerate() {
        check_geometry(&object.geometry, "objects", i, "nesnenin")?;
        checks::line_weight(object.line_weight, &format!("objects[{i}].lineWeight"))?;
        if let Some(x) = object.transparency
            && !(x.is_finite() && (0.0..=kentos_contracts::MAX_TRANSPARENCY).contains(&x))
        {
            return Err(Stop::failed(error(
                codes::INVALID_TRANSPARENCY,
                format!("Saydamlık {x}; 0 ile 0,9 arasında olmalı (%0–90)."),
                Some(format!("objects[{i}].transparency")),
            )));
        }
        check_link(object, i)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut warnings = checks::layer(doc, &input.layer_id)?;
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
            return Err(Stop::failed(error(
                codes::LINK_NOT_FOUND,
                format!(
                    "“{of}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Çizimdeki bir nesnenin kimliğini verin."
                ),
                Some(format!("objects[{i}].labelOf")),
            )));
        }
    }
    // The values given to the layer's fields in their canonical text, and its (and the record's
    // subtype's) defaults where an object gives none (docs/adr/0199 §2, 0422 §2).
    let mut attrs = Vec::with_capacity(input.objects.len());
    let mut rules = checks::LayerRules::new(doc);
    for (i, object) in input.objects.iter().enumerate() {
        let given: BTreeMap<String, Option<String>> = object
            .attrs
            .iter()
            .flatten()
            .map(|(k, v)| (k.clone(), Some(v.clone())))
            .collect();
        let at = format!("objects[{i}].attrs");
        let mut own: BTreeMap<String, String> =
            checks::field_values(doc, &input.layer_id, &given, &at)?
                .into_iter()
                .filter_map(|(k, v)| Some((k, v?)))
                .collect();
        checks::new_defaults(doc, &input.layer_id, &mut own);
        checks::subtype_fit(doc, &input.layer_id, None, &own, &at)?;
        checks::combination_fit(doc, &input.layer_id, None, &own, &at)?;
        warnings.extend(rules.fit(&input.layer_id, None, &own, None, &at)?);
        attrs.push(own);
    }
    let writes: Vec<checks::UniqueWrite<'_>> = attrs
        .iter()
        .enumerate()
        .map(|(i, a)| checks::UniqueWrite {
            slot: None,
            layer: &input.layer_id,
            before: None,
            after: a,
            at: format!("objects[{i}].attrs"),
        })
        .collect();
    checks::unique_fit(doc, &writes)?;
    let placed = placed_points(doc, input)?;
    Ok(Checked {
        warnings,
        attrs,
        placed,
    })
}

/// Every object's measured points read from the places its live references name (docs/adr/0249 §1,
/// 0250): an associative dimension is made by its places, its points following from them, so a script
/// or an agent gives the places and not the coordinates. A reference to an object the drawing lacks is
/// `entity_not_found`, to a place its object has not `no_such_place`; a broken reference's point is as given, and
/// so are the points of references under a rule (docs/adr/0320 §1).
fn placed_points(
    doc: &Document,
    input: &EntitiesCreate,
) -> Result<Vec<Vec<(&'static str, kentos_contracts::Vec2)>>, Stop> {
    let mut out = Vec::with_capacity(input.objects.len());
    for (i, object) in input.objects.iter().enumerate() {
        let mut own = Vec::new();
        if let EntityGeometry::Dimension {
            refs: Some(refs), ..
        } = &object.geometry
        {
            // A rule's points are its own (docs/adr/0320 §1): an angle's arms are not its edges' places, a radius's
            // point may lie past its arc's end; they are as given.
            for (name, r) in refs
                .each()
                .filter(|(_, r)| r.live() && refs.rule.is_none())
            {
                let at = format!("objects[{i}].geometry.refs.{name}");
                let uid = Uuid::from_bytes(r.uid.0);
                let Some(e) = doc.slot_of(uid).and_then(|s| doc.get(s)) else {
                    return Err(Stop::failed(error(
                        codes::ENTITY_NOT_FOUND,
                        format!(
                            "“{uid}” kimlikli nesne çizimde yok: ölçünün noktası onun yerine bağlanamaz. Çizimdeki bir nesnenin kimliğini verin."
                        ),
                        Some(format!("{at}.uid")),
                    )));
                };
                // A place on a piece inside an insert is read through its block path (docs/adr/0263 §3).
                let p = match crate::blocks::place_point(doc.blocks(), e, &r.path, r.place) {
                    Ok(p) => p,
                    Err(crate::blocks::PlaceMiss::Piece) => {
                        return Err(Stop::failed(error(
                            codes::PIECE_GONE,
                            crate::blocks::NO_PIECE.into(),
                            Some(format!("{at}.path")),
                        )));
                    }
                    Err(crate::blocks::PlaceMiss::Place) => {
                        return Err(Stop::failed(error(
                            codes::NO_SUCH_PLACE,
                            "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor.".into(),
                            Some(format!("{at}.place")),
                        )));
                    }
                };
                own.push((name, kentos_contracts::Vec2 { x: p.x, y: p.y }));
            }
        }
        out.push(own);
    }
    Ok(out)
}

/// A new object's link to the object whose label it writes, when it has one
/// (docs/adr/0175 §4): a text's, both fields, the id a persistent id's text,
/// the scale finite and over 0 (`invalid_link`).
fn check_link(object: &NewObject, i: usize) -> Result<(), Stop> {
    let refuse = |field: &str, message: String| {
        Err(Stop::failed(error(
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
        return Err(Stop::failed(error(
            codes::INVALID_PATH,
            "Nesneye bağlı yazının eğrisi olmaz. Önce bağı koparın ya da eğriyi kaldırın.".into(),
            Some(format!("objects[{i}].geometry.path")),
        )));
    }
    Ok(())
}

/// The objects `input` describes, as the document stores them: slot 0 (given
/// when written), its own copies of every field, the attributes `attrs`
/// (each object's, as the checks gave them).
fn entities(
    input: &EntitiesCreate,
    attrs: &[BTreeMap<String, String>],
    placed: &[Vec<(&'static str, kentos_contracts::Vec2)>],
) -> Vec<Entity> {
    input
        .objects
        .iter()
        .zip(attrs)
        .zip(placed)
        .map(|((object, attrs), placed)| {
            let mut entity = entity_of(
                &object.geometry,
                EntityBase {
                    look: kentos_contracts::ObjectLook {
                        line_type: object.line_type,
                        transparency: object.transparency,
                        order: object.order.filter(|o| *o != 0),
                        ..Default::default()
                    },
                    id: 0,
                    layer_id: input.layer_id.clone(),
                    color: object.color.clone(),
                    attrs: attrs.clone(),
                    label: object.label.clone(),
                    symbol: object.symbol.clone(),
                    line_weight: object.line_weight,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
            );
            // An associative dimension's bound points are where their places are (docs/adr/0250).
            if let Entity::Dimension(d) = &mut entity {
                for (name, p) in placed {
                    match *name {
                        "a" => d.a = *p,
                        "b" => d.b = *p,
                        _ => d.c = Some(*p),
                    }
                }
            }
            // A linked text knows its object (docs/adr/0175 §4).
            if let Entity::Text(text) = &mut entity {
                text.label_of = object.label_of.as_deref().and_then(EntityId::parse);
                text.label_scale = object.label_of.as_ref().and(object.label_scale);
            }
            entity
        })
        .collect()
}
