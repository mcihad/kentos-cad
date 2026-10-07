//! What every create command checks after its own input (docs/adr/0022,
//! 0027): the expected revision, then the layer; and how it writes. The
//! codes, paths and messages are the same for every command, so the shared
//! cases of each (`fixtures/commands/v1`) hold them the same way. The web's
//! counterpart is `apps/web/src/product/checks.ts`.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    CommandError, CommandResult, CommandWarning, Entity, LayerNodeType, MAX_LINE_WEIGHT,
    check_value,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::codes;

/// Why a call stops before anything is written.
pub(crate) enum Stop {
    Failed(CommandError),
    Conflict(CommandError),
}

impl<T> From<Stop> for CommandResult<T> {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::Failed(error) => CommandResult::Failed { error },
            Stop::Conflict(error) => CommandResult::Conflict { error },
        }
    }
}

pub(crate) fn error(code: &str, message: String, path: Option<String>) -> CommandError {
    CommandError {
        code: code.to_owned(),
        message,
        path,
        revision: None,
    }
}

/// The line weight an input gives, when it gives one: a number from 0 to
/// 100 mm (`invalid_line_weight`, docs/adr/0139); NaN and ±∞ are not. `path`
/// names it. The message names no value: Rust and JavaScript write some
/// numbers differently (∞, 1e21), and both sides answer in the same words.
pub(crate) fn line_weight(weight: Option<f64>, path: &str) -> Result<(), Stop> {
    match weight {
        Some(w) if !(0.0..=MAX_LINE_WEIGHT).contains(&w) => Err(Stop::Failed(error(
            codes::INVALID_LINE_WEIGHT,
            "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.".into(),
            Some(path.to_owned()),
        ))),
        _ => Ok(()),
    }
}

/// The attributes written to an object on `layer`, by its fields
/// (docs/adr/0199 §2): each value given to a field's key in its canonical
/// text, a null (taken away) checked as empty; in the names' order, the first
/// that does not keep its field's rules `invalid_attribute`, an empty value of
/// a required field `attribute_required`, at `{at}.{name}`. Keys no field
/// names, and every key on a layer without fields, stay as given.
pub(crate) fn field_values(
    doc: &Document,
    layer: &str,
    attrs: &BTreeMap<String, Option<String>>,
    at: &str,
) -> Result<BTreeMap<String, Option<String>>, Stop> {
    let Some(node) = doc.layers().get(layer).filter(|n| !n.fields.is_empty()) else {
        return Ok(attrs.clone());
    };
    let mut out = BTreeMap::new();
    for (key, value) in attrs {
        let Some(field) = node.fields.iter().find(|f| f.name == *key) else {
            out.insert(key.clone(), value.clone());
            continue;
        };
        match check_value(field, value.as_deref().unwrap_or_default()) {
            Ok(canonical) => {
                out.insert(key.clone(), value.as_ref().map(|_| canonical));
            }
            Err(refusal) => {
                let code = if refusal.code == "required" {
                    codes::ATTRIBUTE_REQUIRED
                } else {
                    codes::INVALID_ATTRIBUTE
                };
                return Err(Stop::Failed(error(
                    code,
                    refusal.message,
                    Some(format!("{at}.{key}")),
                )));
            }
        }
    }
    Ok(out)
}

/// A value that is NaN or ±∞, as a message names it: “2. noktanın doğu (Y)”.
pub(crate) fn not_finite(whose: &str, axis: &str, path: String) -> Stop {
    let name = if axis == "x" {
        "doğu (Y)"
    } else {
        "kuzey (X)"
    };
    Stop::Failed(error(
        codes::NOT_FINITE,
        format!(
            "{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."
        ),
        Some(path),
    ))
}

/// The first coordinate of `p` that is NaN or ±∞, x before y (`whose`: “Merkezin”).
pub(crate) fn point(p: kentos_contracts::Vec2, whose: &str, path: &str) -> Result<(), Stop> {
    for (axis, value) in [("x", p.x), ("y", p.y)] {
        if !value.is_finite() {
            return Err(not_finite(whose, axis, format!("{path}.{axis}")));
        }
    }
    Ok(())
}

/// A number that is NaN or ±∞ (a radius, an angle, an elevation), as its
/// message names it: “Yarıçap sonlu bir sayı değil …”.
pub(crate) fn finite(value: f64, what: &str, fix: &str, path: &str) -> Result<(), Stop> {
    if value.is_finite() {
        return Ok(());
    }
    Err(Stop::Failed(error(
        codes::NOT_FINITE,
        format!("{what} sonlu bir sayı değil (NaN ya da sonsuz). {fix}"),
        Some(path.into()),
    )))
}

/// A circle's or an arc's radius: above zero (`invalid_radius`), once it is finite.
pub(crate) fn radius(r: f64) -> Result<(), Stop> {
    if r > 0.0 {
        return Ok(());
    }
    Err(Stop::Failed(error(
        codes::INVALID_RADIUS,
        "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.".into(),
        Some("r".into()),
    )))
}

/// The expected revision, when given: decimal text, then the document's own
/// (`conflict` when not, with the revision now).
pub(crate) fn revision(doc: &Document, expected: Option<&str>) -> Result<(), Stop> {
    let Some(expected) = expected else {
        return Ok(());
    };
    if !is_revision_text(expected) {
        return Err(Stop::Failed(error(
            codes::INVALID_REVISION,
            format!(
                "Beklenen sürüm “{expected}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
            ),
            Some("expectedRevision".into()),
        )));
    }
    let current = doc.revision().to_string();
    if expected != current {
        return Err(Stop::Conflict(CommandError {
            revision: Some(current),
            ..error(
                codes::REVISION_CONFLICT,
                "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.".into(),
                Some("expectedRevision".into()),
            )
        }));
    }
    Ok(())
}

/// The layer the object goes on: known, a layer and not a group, not locked
/// (by itself or a group above it). A hidden one takes the object with a
/// warning. The locked and hidden texts are the drawing tools' own words
/// (web `tools/targetLayer.ts`), kept since the tools write through here.
pub(crate) fn layer(doc: &Document, id: &str) -> Result<Vec<CommandWarning>, Stop> {
    layer_at(doc, id, "layerId")
}

/// [`layer`], its refusals and warning at `path` (`changes[2].layerId`).
pub(crate) fn layer_at(doc: &Document, id: &str, path: &str) -> Result<Vec<CommandWarning>, Stop> {
    let layers = doc.layers();
    let at = || Some(path.to_owned());
    let Some(node) = layers.get(id) else {
        return Err(Stop::Failed(error(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            at(),
        )));
    };
    let name = &node.name;
    if node.kind != LayerNodeType::Layer {
        return Err(Stop::Failed(error(
            codes::NOT_A_LAYER,
            format!(
                "“{name}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin."
            ),
            at(),
        )));
    }
    if layers.is_locked(id) {
        return Err(Stop::Failed(error(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
            ),
            at(),
        )));
    }
    let mut warnings = Vec::new();
    if !layers.is_visible(id) {
        warnings.push(CommandWarning {
            code: codes::LAYER_HIDDEN.into(),
            message: format!("“{name}” katmanı gizli; çizilen nesne görünmeyecek."),
            path: at(),
        });
    }
    Ok(warnings)
}

/// The ids a command names (`cad.entities.delete`, `cad.entities.transform`):
/// at least one (`no_entities`, with the command's own sentence `nothing`),
/// each lowercase UUID text with hyphens (`invalid_uid`, the first that is not).
pub(crate) fn uids(uids: &[String], nothing: &str) -> Result<(), Stop> {
    if uids.is_empty() {
        return Err(Stop::Failed(error(
            codes::NO_ENTITIES,
            format!("{nothing} En az bir nesnenin kalıcı kimliğini verin."),
            Some("uids".into()),
        )));
    }
    for (i, uid) in uids.iter().enumerate() {
        if !is_uid_text(uid) {
            return Err(Stop::Failed(error(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
                ),
                Some(format!("uids[{i}]")),
            )));
        }
    }
    Ok(())
}

/// The objects the ids name, each once, in the input's order: its slot, the
/// object and its id. `entity_not_found` for the first the document lacks.
pub(crate) fn objects<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(Slot, &'a Entity, &'a String)>, Stop> {
    Ok(named(doc, uids)?
        .into_iter()
        .map(|(_, slot, entity, uid)| (slot, entity, uid))
        .collect())
}

/// [`objects`], each with where the input first names it (`uids[at]`), for
/// a refusal about one of them.
pub(crate) fn named<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(usize, Slot, &'a Entity, &'a String)>, Stop> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(uids.len());
    for (i, uid) in uids.iter().enumerate() {
        if !seen.insert(uid.as_str()) {
            continue;
        }
        let found = Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|slot| Some((slot, doc.get(slot)?)));
        let Some((slot, entity)) = found else {
            return Err(Stop::Failed(error(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                Some(format!("uids[{i}]")),
            )));
        };
        out.push((i, slot, entity, uid));
    }
    Ok(out)
}

/// A text that is empty or only white space, as Unicode's `White_Space` has
/// it (the web's `isBlank`; not JavaScript's `trim`, which takes U+FEFF and
/// leaves U+0085).
pub(crate) fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

/// A persistent id as the contract writes it: lowercase hexadecimal with
/// hyphens, 8-4-4-4-12 (the web's `isUuid`).
pub(crate) fn is_uid_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(b),
        })
}

/// Writes one object as one undo step (“Ekle”, the document's own `add`),
/// into the open transaction or group if one is: its slot, persistent id and
/// the revision after, or `slots_exhausted` when the document has no slot left.
pub(crate) fn add(
    doc: &mut Document,
    entity: Entity,
) -> Result<(u32, String, String), CommandError> {
    match doc.add(entity) {
        Ok(slot) => Ok(written(doc, slot)),
        Err(full) => Err(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)),
    }
}

fn written(doc: &Document, slot: Slot) -> (u32, String, String) {
    let uid = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
    (slot.0, uid, doc.revision().to_string())
}

/// Decimal text of a whole number, no sign, no leading zero: `0`, `12`.
/// Any length: a revision the document never had is a conflict, not an error.
fn is_revision_text(text: &str) -> bool {
    let digits = text.bytes().all(|b| b.is_ascii_digit());
    digits && !text.is_empty() && (text == "0" || !text.starts_with('0'))
}
