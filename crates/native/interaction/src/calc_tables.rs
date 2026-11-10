//! Hesap tabloları (docs/adr/8037 §3; M37): the calculation tables a permit set's “Mimari hesap raporu” sheet shows
//! (Planlı Alanlar İmar Yönetmeliği 57(2)(a); the digital project regulation's EK-2 2.8.1: TAKS, emsal, otopark,
//! sığınak … hesapları): Metrekare cetveli, Emsal ve TAKS, Kat irtifakı, Otopark and Sığınak hesabı. The drawing's own
//! live table of each is taken as it is; one it lacks is made beside the drawing, under the others, at the plot
//! scale's table height (the same tables Metraj ▾ places by hand, docs/adr/8029–8031, 8036). The room list (Mahal
//! listesi, docs/adr/8038) is found and made the same way; it is a sheet of its own (EK-2 LS/MHL).

use kentos_commands::{
    CommandResult, EntitiesCreate, Entity, NewObject, ScheduleColumn, ScheduleField,
    ScheduleFilter, ScheduleOf, SchedulesCreate, TableSource, UnitKind,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, create, schedules_create, sheets_fit};

/// A calculation table of the drawing: what the sheet names it and where it lies, metres; a list's EK-2 kind when it is
/// a sheet of its own (the room list's MHL, the joinery list's MMC), none when it goes on the calculation sheet; the
/// scale it is drawn for when not the plot scale (the joinery list's 1/50).
#[derive(Clone, Debug, PartialEq)]
pub struct CalcTable {
    pub label: &'static str,
    pub bounds: [f64; 4],
    pub list: Option<&'static str>,
    pub scale: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Areas,
    Zoning,
    LandShares,
    Parking,
    Shelter,
    RoomList,
    JoineryList,
}

fn kind_of(source: &TableSource) -> Option<Kind> {
    match source {
        TableSource::Schedule {
            of: ScheduleOf::Spaces,
            columns,
            ..
        } if columns.iter().any(|c| c.field.is_room_list()) => Some(Kind::RoomList),
        TableSource::Schedule { by_level: true, .. } => Some(Kind::Areas),
        TableSource::Schedule { columns, .. }
            if columns.iter().any(|c| c.field == ScheduleField::LandShare) =>
        {
            Some(Kind::LandShares)
        }
        TableSource::Zoning {} => Some(Kind::Zoning),
        TableSource::Parking {} => Some(Kind::Parking),
        TableSource::Shelter {} => Some(Kind::Shelter),
        _ => None,
    }
}

/// The drawing's table of `kind`, the last made; the joinery list's insert (docs/adr/8038 §6).
fn found(doc: &Document, kind: Kind) -> Option<[f64; 4]> {
    if kind == Kind::JoineryList {
        return doc
            .entities()
            .filter(|e| matches!(e, Entity::Insert(i) if i.block == kentos_domain::joinery_list::JOINERY_LIST_ID))
            .last()
            .and_then(|e| sheets_fit::extent_of(doc, std::iter::once(e)));
    }
    doc.entities()
        .filter(
            |e| matches!(e, Entity::Table(t) if t.source.as_ref().and_then(kind_of) == Some(kind)),
        )
        .last()
        .and_then(|e| sheets_fit::extent_of(doc, std::iter::once(e)))
}

fn column(field: ScheduleField) -> ScheduleColumn {
    ScheduleColumn {
        field,
        title: None,
        attr: None,
    }
}

/// A schedule at `at`: the metrekare cetveli storey by storey, the kat irtifakı table of the independent units, or the
/// room list (tag_tools.rs's presets).
fn schedule(doc: &mut Document, kind: Kind, at: [f64; 2]) -> Result<(), String> {
    use ScheduleField::*;
    let (mut of, mut sort, mut total) = (ScheduleOf::Units, Vec::new(), Some(true));
    let (columns, filter, by_level) = match kind {
        Kind::Areas => (
            vec![Level, Number, Name, UnitKind, PaiyGross, PaiyNet],
            None,
            Some(true),
        ),
        Kind::RoomList => {
            (of, sort, total) = (ScheduleOf::Spaces, crate::tag_tools::room_list_sort(), None);
            (crate::tag_tools::ROOM_LIST.to_vec(), None, None)
        }
        _ => (
            vec![
                Number,
                Level,
                Nature,
                PaiyNet,
                PaiyGross,
                Annexes,
                AnnexNet,
                TotalGross,
                GeneralGross,
                LandShare,
            ],
            Some(ScheduleFilter {
                kinds: Some(vec![kentos_commands::UnitKind::Unit]),
                ..Default::default()
            }),
            None,
        ),
    };
    let input = SchedulesCreate {
        layer_id: doc.layers().active().to_owned(),
        at: kentos_commands::Vec2 { x: at[0], y: at[1] },
        of,
        columns: Some(columns.into_iter().map(column).collect()),
        filter,
        sort,
        group: None,
        total,
        by_level,
        height: None,
        color: None,
        line_weight: None,
        expected_revision: None,
    };
    match schedules_create::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => Ok(()),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Çizelge yazılamadı.".into()),
    }
}

/// The joinery list's block, as the drawing's types give it, and an insert of it with its top left at `at`.
fn joinery_list(doc: &mut Document, at: [f64; 2]) -> Result<(), String> {
    let block = doc.joinery_list_block();
    if doc
        .block(kentos_domain::joinery_list::JOINERY_LIST_ID)
        .is_some()
    {
        doc.update_block(block).map_err(|r| r.to_string())?;
    } else {
        doc.add_block(block).map_err(|r| r.to_string())?;
    }
    let input = kentos_commands::BlocksInsert {
        parameters: None,
        block: Some(kentos_domain::joinery_list::JOINERY_LIST_ID),
        name: None,
        at: kentos_commands::Vec2 { x: at[0], y: at[1] },
        scale: Some(1.0),
        y_scale: None,
        rotation: Some(0.0),
        mirror: None,
        layer_id: doc.layers().active().to_owned(),
        color: None,
        attributes: None,
        attrs: None,
        label: None,
        symbol: None,
        arch: Default::default(),
        expected_revision: None,
    };
    match kentos_native_application::blocks_insert::execute(&mut ExecutionContext::new(doc), input)
    {
        CommandResult::Completed { .. } => Ok(()),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Doğrama listesi yazılamadı.".into()),
    }
}

/// A live table of the drawing (Emsal ve TAKS, Otopark, Sığınak) at `at`, its rows the document's.
fn live(doc: &mut Document, source: TableSource, at: [f64; 2]) -> Result<(), String> {
    let written = match &source {
        TableSource::Zoning {} => doc.zoning_table(),
        TableSource::Parking {} => doc.parking_table(),
        _ => doc.shelter_table(),
    };
    let look = crate::table::Look {
        header: true,
        height: 2.5 / 1000.0 * doc.settings().plot_scale,
        ..Default::default()
    };
    let cells = kentos_geometry_core::ops::table::from_rows(written.rows, true);
    let geometry = crate::table::new_table(
        doc,
        &cells,
        &look,
        crate::Vec2::new(at[0], at[1]),
        Some(source),
    );
    let input = EntitiesCreate {
        layer_id: doc.layers().active().to_owned(),
        objects: vec![NewObject {
            geometry,
            color: None,
            line_weight: None,
            line_type: None,
            attrs: None,
            label: None,
            label_of: None,
            label_scale: None,
            symbol: None,
            order: None,
            transparency: None,
        }],
        operation: None,
        expected_revision: None,
    };
    match create::execute(&mut ExecutionContext::new(doc), input) {
        CommandResult::Completed { .. } => Ok(()),
        CommandResult::Failed { error } | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Tablo yazılamadı.".into()),
    }
}

/// The drawing's calculation tables in the sheet's order, the ones it lacks made beside it (to the right of everything
/// drawn, one under another); a table the drawing cannot fill (no İmar durumu, no independent unit, no room) is left
/// out.
pub fn ensure(doc: &mut Document) -> Result<Vec<CalcTable>, String> {
    let units = doc.program().units.iter().any(|u| u.kind == UnitKind::Unit);
    let zoning = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.zoning)
        .is_some();
    let rooms = doc.entities().any(|e| matches!(e, Entity::Space(_)));
    let joinery = !doc.joinery_rows().is_empty();
    let wanted = [
        // The room list first (MO: mahal listesi before the parking calculation); a sheet of its own.
        (Kind::RoomList, "Mahal listesi", rooms),
        // The joinery list (docs/adr/8038 §6): a sheet of its own at 1/50.
        (Kind::JoineryList, "Doğrama listesi", joinery),
        (Kind::Areas, "Metrekare cetveli", rooms),
        (Kind::Zoning, "Emsal ve TAKS", zoning),
        (Kind::LandShares, "Kat irtifakı", units),
        (Kind::Parking, "Otopark hesabı", units),
        (Kind::Shelter, "Sığınak hesabı", units),
    ];
    let gap = 0.02 * doc.settings().plot_scale;
    let all = sheets_fit::extent_of(doc, doc.entities()).unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let x = all[2] + 2.0 * gap;
    let mut y = all[3];
    let mut out = Vec::new();
    for (kind, label, wanted) in wanted {
        if !wanted {
            continue;
        }
        let bounds = match found(doc, kind) {
            Some(b) if kind == Kind::JoineryList => {
                // The list as the types are now (the document keeps it so; a drawing of before kept none).
                let block = doc.joinery_list_block();
                doc.update_block(block).map_err(|r| r.to_string())?;
                found(doc, kind).unwrap_or(b)
            }
            Some(b) => b,
            None => {
                match kind {
                    Kind::Areas | Kind::LandShares | Kind::RoomList => schedule(doc, kind, [x, y])?,
                    Kind::JoineryList => joinery_list(doc, [x, y])?,
                    Kind::Zoning => live(doc, TableSource::Zoning {}, [x, y])?,
                    Kind::Parking => live(doc, TableSource::Parking {}, [x, y])?,
                    Kind::Shelter => live(doc, TableSource::Shelter {}, [x, y])?,
                }
                let b = found(doc, kind).ok_or_else(|| format!("{label} yazılamadı."))?;
                y = b[1] - gap;
                b
            }
        };
        let (list, scale) = match kind {
            Kind::RoomList => (Some("MHL"), None),
            Kind::JoineryList => (
                Some("MMC"),
                Some(kentos_domain::joinery_list::JOINERY_LIST_SCALE),
            ),
            _ => (None, None),
        };
        out.push(CalcTable {
            label,
            bounds,
            list,
            scale,
        });
    }
    Ok(out)
}
