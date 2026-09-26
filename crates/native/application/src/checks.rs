//! What every create command checks after its own input (docs/adr/0022,
//! 0027): the expected revision, then the layer; and how it writes. The
//! codes, paths and messages are the same for every command, so the shared
//! cases of each (`fixtures/commands/v1`) hold them the same way. The web's
//! counterpart is `apps/web/src/product/checks.ts`.

use kentos_contracts::{CommandError, CommandResult, CommandWarning, Entity, LayerNodeType};
use kentos_domain::{Document, Slot};

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
    let layers = doc.layers();
    let at = || Some("layerId".to_owned());
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
