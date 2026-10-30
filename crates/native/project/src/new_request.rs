//! A new project asked by name (docs/adr/0245): what the headless host, the
//! Python SDK (`Document.new`) and the MCP server's `drawing.new` take. Its
//! fields are the Yeni proje wizard's choices, and the project they make is
//! the wizard's: a [`Draft`] is filled from them, stopped where the wizard
//! stops it ([`Draft::blocked`]) and made with [`Draft::options`]. What a
//! window cannot ask at all (a local CBS project, a unit for a project with
//! a coordinate system, a system and the local card at once) is refused
//! here rather than dropped, since a script cannot see what was ignored.
//!
//! Geometry stays in metres in the project's plane whatever the unit; the
//! drawing unit is only how the windows read and write lengths
//! (docs/adr/0165 §2).

use kentos_contracts::{DrawingFont, DrawingUnit, Workspace};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::crs::LOCAL_SRID;
use crate::new_project::{NEW_PROJECT_NAME, NewProject};
use crate::provinces::province;
use crate::wizard::{Coords, Draft, Kind, Step};

/// The fields a request may have, as the refusal of another names them.
pub const FIELDS: [&str; 8] = [
    "name",
    "workspace",
    "coords",
    "srid",
    "province",
    "drawingUnit",
    "plotScale",
    "drawingFont",
];

/// What a new project is asked with; none: the wizard's own choice.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NewRequest {
    pub name: Option<String>,
    /// The project's type: CAD or CBS (others are not made yet).
    pub workspace: Option<Workspace>,
    /// A CAD project's coordinates: local (no system, SRID 0) or real.
    pub coords: Option<Coords>,
    /// The coordinate system; 0 is the local one.
    pub srid: Option<u32>,
    /// The province (plate code) it opens on; its zone is the system when none is given.
    pub province: Option<u32>,
    /// A local project's unit (mm, cm, m).
    pub drawing_unit: Option<DrawingUnit>,
    /// The plot scale's denominator (1000: 1:1000).
    pub plot_scale: Option<f64>,
    pub drawing_font: Option<DrawingFont>,
}

/// Why a request makes no project: a stable code and the reason in Turkish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// `invalid_input`, or `unknown_system` for a system the registry lacks.
    pub code: &'static str,
    pub message: String,
}

fn invalid(message: impl Into<String>) -> Refusal {
    Refusal {
        code: "invalid_input",
        message: message.into(),
    }
}

fn field<T: DeserializeOwned>(
    fields: &Map<String, Value>,
    key: &str,
    what: &str,
) -> Result<Option<T>, Refusal> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|_| invalid(format!("{key}: {what} olmalı."))),
    }
}

impl NewRequest {
    /// A request from its JSON object (camelCase names; null is absent).
    pub fn read(input: &Value) -> Result<Self, Refusal> {
        let empty = Map::new();
        let fields = match input {
            Value::Null => &empty,
            Value::Object(f) => f,
            _ => return Err(invalid("Yeni çizimin seçenekleri bir JSON nesnesi olmalı.")),
        };
        if let Some(other) = fields.keys().find(|k| !FIELDS.contains(&k.as_str())) {
            return Err(invalid(format!(
                "“{other}” yeni çizimin bir seçeneği değil: {}.",
                FIELDS.join(", ")
            )));
        }
        Ok(Self {
            name: field(fields, "name", "metin")?,
            workspace: field(fields, "workspace", "cad ya da gis")?,
            coords: field(fields, "coords", "local ya da real")?,
            srid: field(
                fields,
                "srid",
                "bir EPSG kodu (ör. 5256) ya da yerel için 0",
            )?,
            province: field(fields, "province", "ilin plaka kodu (1–81)")?,
            drawing_unit: field(fields, "drawingUnit", "mm, cm ya da m")?,
            plot_scale: field(
                fields,
                "plotScale",
                "çizim ölçeğinin paydası (1000: 1:1000)",
            )?,
            drawing_font: field(
                fields,
                "drawingFont",
                "bilinen bir yazı tipi (barlow, arimo …)",
            )?,
        })
    }

    /// The project the wizard makes of these choices.
    pub fn project(&self) -> Result<NewProject, Refusal> {
        let kind = match self.workspace.unwrap_or(Workspace::Gis) {
            Workspace::Cad => Kind::Cad,
            Workspace::Gis => Kind::Gis,
            _ => {
                return Err(invalid(
                    "workspace: cad ya da gis (öbür türler henüz yapılmıyor).",
                ));
            }
        };
        let coords = match (kind, self.coords, self.srid) {
            (Kind::Gis, Some(Coords::Local), _) | (Kind::Gis, None, Some(LOCAL_SRID)) => {
                return Err(invalid(
                    "Yerel (koordinat sistemsiz) proje yalnız CAD projesidir; CBS projesinin bir koordinat sistemi vardır.",
                ));
            }
            (_, Some(Coords::Local), Some(s)) if s != LOCAL_SRID => {
                return Err(invalid(format!(
                    "coords local iken srid verilmez (EPSG:{s}): yerel projenin koordinat sistemi yoktur."
                )));
            }
            (_, Some(Coords::Real), Some(LOCAL_SRID)) => {
                return Err(invalid(
                    "coords real iken srid 0 olamaz: projenin koordinat sistemini verin (ör. 5256).",
                ));
            }
            (_, Some(c), _) => c,
            (Kind::Cad, None, None | Some(LOCAL_SRID)) => Coords::Local,
            (_, None, _) => Coords::Real,
        };
        let local = coords == Coords::Local;
        if let Some(code) = self.province {
            if province(code).is_none() {
                return Err(invalid(format!(
                    "province: {code} bir il plaka kodu değil (1–81)."
                )));
            }
            if local {
                return Err(invalid(
                    "province yalnız koordinat sistemli projede verilir; yerel proje 0,0'dan başlar.",
                ));
            }
        }
        if let (false, Some(u)) = (local, self.drawing_unit)
            && u != DrawingUnit::M
        {
            return Err(invalid(
                "drawingUnit yalnız yerel (koordinat sistemsiz) CAD projesinde seçilir; koordinat sistemli proje metrede çizilir.",
            ));
        }
        if let Some(s) = self.plot_scale
            && !(s.is_finite() && s > 0.0)
        {
            return Err(invalid(
                "plotScale sıfırdan büyük bir sayı olmalı (1000: 1:1000).",
            ));
        }
        let draft = Draft {
            kind,
            coords,
            // The wizard's first work, a building in metres (docs/adr/0626 §1), when none is asked.
            unit: self.drawing_unit.unwrap_or(DrawingUnit::M),
            province: self.province,
            srid: if local { None } else { self.srid },
            plot_scale: self.plot_scale,
            name: self
                .name
                .clone()
                .unwrap_or_else(|| NEW_PROJECT_NAME.to_owned()),
            font: self.drawing_font.unwrap_or(DrawingFont::Barlow),
            // The wizard falls back on the app's default system; a script has
            // none, so a system is asked, never guessed.
            fallback_srid: LOCAL_SRID,
            annotation: None,
        };
        if !local && draft.srid.or_else(|| draft.suggested_srid()).is_none() {
            return Err(invalid(
                "srid gerekli: projenin koordinat sistemi (ör. 5256); ya da province verin, ilin TUREF dilimi alınır.",
            ));
        }
        if let Some(why) = draft.blocked(Step::Coords) {
            return Err(Refusal {
                code: "unknown_system",
                message: why,
            });
        }
        if let Some(why) = draft.blocked(Step::Details) {
            return Err(invalid(why));
        }
        Ok(draft.options())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn made(input: Value) -> Result<NewProject, Refusal> {
        NewRequest::read(&input)?.project()
    }

    #[test]
    fn a_local_cad_project_in_millimetres_is_the_wizards() {
        let p = made(json!({ "name": "Daire", "workspace": "cad", "drawingUnit": "mm" }))
            .expect("a project");
        let d = Draft {
            kind: Kind::Cad,
            coords: Coords::Local,
            unit: DrawingUnit::Mm,
            province: None,
            srid: None,
            plot_scale: None,
            name: "Daire".into(),
            font: DrawingFont::Barlow,
            fallback_srid: 5256,
            annotation: None,
        };
        assert_eq!(p, d.options());
        assert_eq!(
            (p.srid, p.plot_scale, p.drawing_unit),
            (LOCAL_SRID, 1.0, Some(DrawingUnit::Mm))
        );
        // srid 0 says the same.
        assert_eq!(
            made(json!({ "name": "Daire", "workspace": "cad", "srid": 0, "drawingUnit": "mm" })),
            Ok(p)
        );
    }

    #[test]
    fn a_project_with_a_system_is_metres_and_its_system_is_asked_or_the_provinces() {
        let p = made(json!({ "srid": 5256 })).expect("a project");
        assert_eq!(
            (
                p.name.as_str(),
                p.workspace,
                p.srid,
                p.plot_scale,
                p.drawing_unit
            ),
            (NEW_PROJECT_NAME, Workspace::Gis, 5256, 1000.0, None)
        );
        let sivas = made(json!({ "workspace": "cad", "coords": "real", "province": 58 }))
            .expect("a project");
        assert_eq!(
            (sivas.srid, sivas.province, sivas.drawing_unit),
            (5256, Some(58), None)
        );
        assert_eq!(
            made(json!({ "srid": 5256, "drawingUnit": "m" })).map(|p| p.drawing_unit),
            Ok(None)
        );
    }

    /// A new CAD project's work is the wizard's (docs/adr/0801): a building in metres, a plan on a system, a detail
    /// in millimetres; a local unit no work has none, nor has a CBS project.
    #[test]
    fn a_new_cad_project_takes_its_works_work() {
        use kentos_contracts::CadWork;
        let work = |input: Value| made(input).expect("a project").cad_work;
        assert_eq!(
            work(json!({ "workspace": "cad" })),
            Some(CadWork::Architecture)
        );
        assert_eq!(
            work(json!({ "workspace": "cad", "srid": 5256 })),
            Some(CadWork::Map)
        );
        assert_eq!(
            work(json!({ "workspace": "cad", "coords": "real", "province": 58 })),
            Some(CadWork::Map)
        );
        assert_eq!(
            work(json!({ "workspace": "cad", "drawingUnit": "mm" })),
            Some(CadWork::Detail)
        );
        assert_eq!(
            work(json!({ "workspace": "cad", "drawingUnit": "cm" })),
            None
        );
        assert_eq!(work(json!({ "srid": 5256 })), None);
        // The project it makes carries it.
        let snapshot = crate::new_project::new_project(
            &made(json!({ "workspace": "cad", "drawingUnit": "mm" })).expect("a project"),
        )
        .expect("a drawing");
        assert_eq!(snapshot.settings.cad_work, Some(CadWork::Detail));
    }

    #[test]
    fn what_a_window_cannot_ask_is_refused_with_its_reason() {
        let code = |input: Value| made(input).map_err(|r| r.code).err();
        assert_eq!(code(json!({ "name": "CRS'siz" })), Some("invalid_input"));
        assert_eq!(code(json!({ "srid": 4 })), Some("unknown_system"));
        assert_eq!(code(json!({ "srid": 0 })), Some("invalid_input"));
        assert_eq!(
            code(json!({ "workspace": "gis", "coords": "local" })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "srid": 5256, "drawingUnit": "mm" })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "workspace": "cad", "coords": "local", "srid": 5256 })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "workspace": "cad", "coords": "real", "srid": 0 })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "workspace": "cad", "province": 6 })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "srid": 5256, "province": 99 })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "workspace": "plan3d", "srid": 5256 })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "srid": 5256, "plotScale": 0 })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "srid": 5256, "drawingUnit": "inch" })),
            Some("invalid_input")
        );
        assert_eq!(
            code(json!({ "srid": 5256, "unit": "mm" })),
            Some("invalid_input")
        );
        assert_eq!(code(json!([1])), Some("invalid_input"));
        // The wizard's own words where it stops.
        assert_eq!(
            made(json!({ "srid": 5256, "name": " " }))
                .err()
                .map(|r| r.message),
            Some("Proje adı boş olamaz.".to_owned())
        );
        assert_eq!(
            made(json!({ "srid": 4 })).err().map(|r| r.message),
            Some("EPSG:4 bu sürümde tanımlı değil; listeden bir sistem seçin.".to_owned())
        );
    }
}
