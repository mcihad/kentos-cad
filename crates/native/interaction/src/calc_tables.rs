//! Hesap tabloları (docs/adr/3037 §3; M37): the calculation tables a permit set's “Mimari hesap raporu” sheet shows
//! (Planlı Alanlar İmar Yönetmeliği 57(2)(a); the digital project regulation's EK-2 2.8.1: TAKS, emsal, otopark,
//! sığınak … hesapları): Metrekare cetveli, Emsal ve TAKS, Kat irtifakı, Otopark and Sığınak hesabı. The drawing's own
//! live table of each is taken as it is; one it lacks is made beside the drawing, under the others, at the plot
//! scale's table height (the same tables Metraj ▾ places by hand, docs/adr/3029–3031, 3036).

use kentos_commands::{
    CommandResult, EntitiesCreate, Entity, NewObject, ScheduleColumn, ScheduleField,
    ScheduleFilter, ScheduleOf, SchedulesCreate, TableSource, UnitKind,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, create, schedules_create, sheets_fit};

/// A calculation table of the drawing: what the sheet names it and where it lies, metres.
#[derive(Clone, Debug, PartialEq)]
pub struct CalcTable {
    pub label: &'static str,
    pub bounds: [f64; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Areas,
    Zoning,
    LandShares,
    Parking,
    Shelter,
}

fn kind_of(source: &TableSource) -> Option<Kind> {
    match source {
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

/// The drawing's table of `kind`, the last made.
fn found(doc: &Document, kind: Kind) -> Option<[f64; 4]> {
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

/// A units schedule at `at`: the metrekare cetveli storey by storey, or the kat irtifakı table of the independent units
/// (tag_tools.rs's presets).
fn schedule(doc: &mut Document, kind: Kind, at: [f64; 2]) -> Result<(), String> {
    use ScheduleField::*;
    let (columns, filter, by_level) = match kind {
        Kind::Areas => (
            vec![Level, Number, Name, UnitKind, PaiyGross, PaiyNet],
            None,
            Some(true),
        ),
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
        of: ScheduleOf::Units,
        columns: Some(columns.into_iter().map(column).collect()),
        filter,
        sort: Vec::new(),
        group: None,
        total: Some(true),
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
    let wanted = [
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
            Some(b) => b,
            None => {
                match kind {
                    Kind::Areas | Kind::LandShares => schedule(doc, kind, [x, y])?,
                    Kind::Zoning => live(doc, TableSource::Zoning {}, [x, y])?,
                    Kind::Parking => live(doc, TableSource::Parking {}, [x, y])?,
                    Kind::Shelter => live(doc, TableSource::Shelter {}, [x, y])?,
                }
                let b = found(doc, kind).ok_or_else(|| format!("{label} yazılamadı."))?;
                y = b[1] - gap;
                b
            }
        };
        out.push(CalcTable { label, bounds });
    }
    Ok(out)
}
