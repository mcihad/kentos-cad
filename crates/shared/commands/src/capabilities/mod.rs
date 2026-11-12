//! What each command takes and how it answers (docs/adr/0253 §7): the
//! object kinds its operations apply to, the codes it refuses with and the
//! codes of its warnings. The catalog carries them (`CommandCapabilities`),
//! beside the meaning of every code (`codes::CODES`) and the codes any call
//! may answer with whatever its command (`PROTOCOL_CODES`).
//!
//! Kept true by tests: every code a shared case of `fixtures/commands/v1`
//! answers with is listed for its command, and a listed code no case reaches
//! is named with the reason (`kentos-native-application`, tests/capabilities.rs);
//! for the commands that pick objects by kind, an object of each kind is
//! tried: a listed kind is taken, another is refused with the listed code.

use crate::{CommandCapabilities, CommandTarget};

mod annotation;
mod arch;
mod data;
mod drawing;
mod files;
mod gis;
mod map;
mod road;

/// Every object kind, as `Entity` tags it.
pub const KINDS: [&str; 33] = [
    "point",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "nurbs",
    "xline",
    "ray",
    "text",
    "dimension",
    "hatch",
    "insert",
    "leader",
    "table",
    "image",
    "raster",
    "pointcloud",
    "wall",
    "opening",
    "space",
    "slab",
    "grid",
    "column",
    "beam",
    "roof",
    "railing",
    "stair",
    "ramp",
    "tag",
    "viewmark",
];

/// The codes any call may answer with, whatever its command: the
/// protocol's (docs/adr/0253 §4–§9).
pub const PROTOCOL_CODES: &[&str] = &[
    "invalid_call",
    "unknown_command",
    "server_command",
    "unknown_version",
    "invalid_input",
    "invalid_revision",
    "revision_conflict",
    "stale_state",
    "plan_changed",
    "request_reused",
    "integrity_failed",
    // Every write is held to its layers' types (docs/adr/0622 §5).
    "layer_geometry_mismatch",
    "document_changed",
    "already_committed",
    "unknown_job",
    // Every write that changes constrained objects is followed by the constraints' solve (docs/adr/0337 §1, §2).
    "constraint_conflict",
    "constraint_singular",
    "not_converged",
    "constraints_removed",
    "constraints_relaxed",
    // Every write that touches the objects of a network that keeps its subnetworks current (docs/adr/0562 §3).
    "subnetwork_dirty",
    // Every write that touches the routes or the events of a link that keeps them on their routes (docs/adr/0569 §3).
    "events_dirty",
    // Every write that changes or removes a kitchen run's object apart from its run (docs/adr/0727 §5).
    "casework_owned",
    // Every write that leaves a project's drawings stale while it draws them again by itself (docs/adr/0911 §3,
    // docs/adr/4740): what was drawn again, left as changed by hand, or could not be drawn.
    "drawings_rebuilt",
    "drawing_edited",
    "drawing_not_rebuilt",
];

/// The revision's two, every writing command's.
/// A new object's attributes by its layer's fields (TODOS.md FORM-O, ADR 0199 §2).
const ATTRS: &[&str] = &["invalid_attribute", "attribute_required"];
const REVISION: &[&str] = &["invalid_revision", "revision_conflict"];
/// A new object's layer.
const LAYER: &[&str] = &["layer_not_found", "not_a_layer", "layer_locked"];
/// Objects named by persistent id.
const OBJECTS: &[&str] = &[
    "no_entities",
    "invalid_uid",
    "entity_not_found",
    "layer_locked",
];

/// Forwarded sheet validation codes: the convenience create command uses the same core.
const SHEET_CORE: &[&str] = &[
    "bad_drawing_info",
    "duplicate_sheet_number",
    "bad_drawing_index",
    "not_fixed_map",
    "ambiguous_source",
    "invalid_extent",
    "bad_curve",
    "curve_too_complex",
    "asset_in_use",
    "asset_too_large",
    "bad_asset",
    "bad_atlas",
    "bad_binding",
    "bad_color",
    "bad_constraint",
    "bad_continuation",
    "bad_font",
    "bad_group",
    "bad_id",
    "bad_layout",
    "bad_level",
    "bad_link",
    "bad_name",
    "bad_order",
    "bad_patch",
    "bad_template",
    "bad_value",
    "bad_variable",
    "bad_variable_value",
    "duplicate_id",
    "duplicate_name",
    "duplicate_variable",
    "empty_group",
    "group_cycle",
    "ids_missing",
    "invalid_column_name",
    "item_in_use",
    "item_locked",
    "kind_change",
    "margins_too_large",
    "master_in_use",
    "missing_asset",
    "mixed_owners",
    "newer_schema",
    "no_items",
    "no_master",
    "out_of_range",
    "page_orientation",
    "page_size_mismatch",
    "reserved_id",
    "template_has_place",
    "template_too_large",
    "unknown_guide",
    "unknown_item",
    "unknown_master",
    "unknown_paper",
    "unknown_schema",
    "unknown_sheet",
    "unknown_variant",
];

struct Entry {
    id: &'static str,
    refusals: &'static [&'static [&'static str]],
    warnings: &'static [&'static str],
    targets: fn() -> Vec<CommandTarget>,
}

fn none() -> Vec<CommandTarget> {
    Vec::new()
}

fn target(
    operation: Option<&str>,
    kinds: &[&str],
    except: &[&str],
    refused_with: Option<&str>,
    note: Option<&str>,
) -> CommandTarget {
    CommandTarget {
        operation: operation.map(str::to_owned),
        kinds: kinds.iter().map(|k| (*k).to_owned()).collect(),
        except: except.iter().map(|k| (*k).to_owned()).collect(),
        refused_with: refused_with.map(str::to_owned),
        note: note.map(str::to_owned),
    }
}

/// The objects a tangent circle touches (docs/adr/0289).
pub const TANGENT_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "xline", "ray",
];
/// The curves the core offsets (docs/adr/0246).
pub const OFFSET_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "circle", "arc", "ellipse", "spline", "nurbs", "xline", "ray",
];
/// The curves the core cuts (docs/adr/0250).
pub const TRIM_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "circle", "arc", "ellipse", "xline", "ray",
];
/// The curves the core grows (docs/adr/0250).
pub const EXTEND_KINDS: &[&str] = &["line", "polyline", "arc", "ellipse"];
/// The objects Kır breaks (docs/adr/0254 §3, 0296).
pub const BREAK_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "ellipse", "nurbs", "xline", "ray",
];
/// The objects a chain is made of (docs/adr/0254 §4).
pub const JOIN_KINDS: &[&str] = &["line", "arc", "polyline"];
/// The objects Yönü çevir turns (docs/adr/0296).
pub const REVERSE_KINDS: &[&str] = &["line", "polyline", "polygon", "spline", "nurbs"];
/// The objects Böl divides (docs/adr/0254 §5).
pub const DIVIDE_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "ellipse", "spline", "nurbs",
];
/// The objects a UCS can be aligned to (docs/adr/0261 §3).
pub const UCS_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "hatch", "arc", "circle", "ellipse", "text", "insert", "point",
];
/// The closed objects Çoklu tara hatches (docs/adr/0254 §7).
pub const CLOSED_KINDS: &[&str] = &["polygon", "circle", "ellipse", "spline"];
/// A new object's look, as `cad.entities.create` checks it.
const LOOK: &[&str] = &["invalid_line_weight"];

/// The table, in its order: its parts (docs/adr/8054 §4).
const TABLES: &[&[Entry]] = &[
    drawing::ENTRIES,
    road::ENTRIES,
    map::ENTRIES,
    arch::ENTRIES,
    data::ENTRIES,
    annotation::ENTRIES,
    gis::ENTRIES,
    files::ENTRIES,
];

fn entries() -> impl Iterator<Item = &'static Entry> {
    TABLES.iter().flat_map(|t| t.iter())
}

fn fillet_targets() -> Vec<CommandTarget> {
    let mut targets = corner_targets();
    targets[1] = target(Some("lines"), &["line", "polyline", "arc", "circle"], &[], Some("not_a_corner"),
        Some("Bağımsız çizgi/yay/daire çiftleri analitik teğet çözüm kullanır; daireler bütün kalır, yaylar sonlu açı aralığında kırpılır. Açık çoklu çizginin yalnız düz uç kenarları, başka bir düz uç kenarla birleşebilir; yaylı yol, elips ve NURBS desteklenmez."));
    targets
}

fn corner_targets() -> Vec<CommandTarget> {
    vec![
        target(
            Some("vertex"),
            &["polyline", "polygon"],
            &[],
            Some("not_a_corner"),
            Some("İki düz kenar arasındaki köşe."),
        ),
        target(Some("lines"), &["line"], &[], Some("not_a_corner"), None),
        target(
            Some("all"),
            &["polyline", "polygon"],
            &[],
            Some("not_a_corner"),
            None,
        ),
    ]
}

/// What command `id` takes and how it answers; none for a command the
/// table does not know (the server's own: their answers are `ApiError`s).
pub fn capabilities_of(id: &str) -> Option<CommandCapabilities> {
    let entry = entries().find(|e| e.id == id)?;
    let mut refusals: Vec<String> = Vec::new();
    for list in entry.refusals {
        for code in *list {
            if !refusals.iter().any(|r| r == code) {
                refusals.push((*code).to_owned());
            }
        }
    }
    Some(CommandCapabilities {
        exchange: exchange_profile(id),
        targets: (entry.targets)(),
        refusals,
        warnings: {
            let mut warnings:Vec<String>=entry.warnings.iter().map(|w|(*w).to_owned()).collect();
            if matches!(id,"cad.entities.edit"|"cad.entities.transform"|"cad.entities.delete"|"cad.entities.trim"|"cad.entities.extend"|"cad.entities.break"|"cad.entities.join"|"cad.entities.fillet"|"cad.entities.chamfer"|"cad.entities.stretch"|"cad.entities.lengthen"|"cad.changeset.apply"|"cad.hatches.edit") {warnings.push("hatch_association_lost".into());}
            warnings
        },
    })
}

/// The ids the table lists, in its order.
pub fn listed() -> impl Iterator<Item = &'static str> {
    entries().map(|e| e.id)
}

/// A codec may decode more than the KentOS model can edit; never advertise both as one boolean.
fn exchange_profile(id: &str) -> Option<crate::CadFormatCapabilities> {
    if !matches!(
        id,
        "file.import.dxf" | "file.export.dxf" | "file.import.dwg" | "file.export.dwg"
    ) {
        return None;
    }
    let dwg = id.ends_with("dwg");
    let versions: Vec<String> = if dwg {
        crate::DwgVersion::ALL
            .iter()
            .map(|v| v.signature().to_owned())
            .collect()
    } else {
        [
            "AC1006", "AC1009", "AC1012", "AC1014", "AC1015", "AC1018", "AC1021", "AC1024",
            "AC1027", "AC1032",
        ]
        .iter()
        .map(|v| (*v).to_owned())
        .collect()
    };
    Some(crate::CadFormatCapabilities {
        format: if dwg { "DWG" } else { "DXF" }.into(),
        provider: if dwg { "OpenCADCodec, pinned Git revision; KentOS model-space bridge" } else { "KentOS ASCII DXF" }.into(),
        local: true, remote_upload: false, mid_decode_cancellation: !dwg,
        read_versions: versions,
        write_versions: if dwg { crate::DwgVersion::ALL.iter().map(|v| v.signature().to_owned()).collect() } else { vec!["AC1021".into()] },
        encodings: if dwg { vec!["binary DWG".into()] } else { vec!["ASCII UTF-8 (AC1021+)".into(), "ASCII ANSI_1252 / ANSI_1254 (legacy)".into()] },
        features: [
            ("modelSpace2d", "native", "native", "Temel entity, katman, stil, blok ve NURBS; bildirilen dönüşüm/kayıp raporuna bakın."),
            ("polylineIntegrity", "strict2d", "strict2d", "Düzlemsel genişlik/yay/kapanış/kot ve X/Y blok ölçeği korunur; 2B modelin temsil edemediği dönüşüm polyline_geometry_loss ile reddedilir. allowPolylineLoss açıkça seçilirse raporlu yaklaşık dönüşüm yapılır; eğik OCS tam yerel 3B desteği değildir."),
            ("annotation", "appearance", "appearance", "Desteklenen ölçü/yazı/kılavuz; çoklu annotative scale veya foreign ilişki ağının tamamı değil."),
            ("xref", "reference", "reference", "Kaynak yol, attach/overlay ve yerleştirme korunur; dış dosya kendiliğinden yüklenmez."),
            ("layoutViewport", "native2d", "native2d", "Gerçek pafta kitabı, 2B ortografik VIEWPORT ve temel kâğıt öğeleri; 3B/perspektif, kırpma, atlas ve ileri pafta öğeleri kayıp raporunda."),
            ("dynamicBlock", "appearance", "appearance", "Foreign evaluation graph düzenlenebilir sayılmaz; plain block görünüşü."),
            ("constraint", "reportedLoss", "reportedLoss", "Çözümlenmiş geometri; foreign kısıt graph'ı aktarılmaz."),
            ("aecProxy", "reportedLoss", "rejected", "Çözülebilen görünüş veya açık kayıp; bilinmeyen sınıf çalıştırılmaz."),
            ("opaqueMetadata", "reportedLoss", "reportedLoss", "Kaynak kimlikleri raporda; eşlenemeyen pointer/dictionary graph'ı kayıpsız yazılmaz."),
        ].iter().map(|(feature, read, write, note)| crate::CadFeatureSupport {
            feature: (*feature).into(), read: (*read).into(), write: (*write).into(), note: (*note).into()
        }).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_code_has_its_meaning() {
        for id in listed() {
            let c = capabilities_of(id).expect("listed");
            for code in c.refusals.iter().chain(&c.warnings) {
                assert!(
                    crate::codes::info(code).is_some(),
                    "{id}: {code} has no meaning in codes.rs"
                );
            }
            for t in &c.targets {
                for k in t.kinds.iter().chain(&t.except) {
                    assert!(k == "*" || KINDS.contains(&k.as_str()), "{id}: {k}");
                }
                if let Some(code) = &t.refused_with {
                    assert!(
                        c.refusals.contains(code),
                        "{id}: {code} refuses a kind but is not listed"
                    );
                }
            }
        }
        for code in PROTOCOL_CODES {
            assert!(crate::codes::info(code).is_some(), "{code}");
        }
    }
}
