//! “Bu koordinatlar hangi sistemde?” (the web's `CrsQuestion`,
//! apps/web/src/ui/io/common.ts). A source file carries no reliable
//! coordinate system, so the user says which; the project's is the default.
//! Another system blocks the import: datum and zone transformations do not
//! exist yet and coordinates are never reprojected silently (CLAUDE.md §5).
//! The systems are the web's registry (fixtures/crs/v1/registry.json), in its
//! order and grouped by datum as the web's list is.

use std::sync::OnceLock;

use iced::Element;
use iced::widget::column;
use kentos_ui::label;
use kentos_ui::widget::Banner;
use kentos_ui::widget::select::{Choice, Select};

/// A coordinate system of the registry.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct System {
    pub srid: u32,
    pub name: String,
    /// `projected` or `geographic`.
    pub kind: String,
    /// `TUREF`, `ED50` or `WGS84`.
    pub datum: String,
}

/// The registry's systems, their datums in the order they first appear.
pub fn systems() -> &'static [System] {
    static SYSTEMS: OnceLock<Vec<System>> = OnceLock::new();
    SYSTEMS.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Registry {
            systems: Vec<System>,
        }
        let all = serde_json::from_str::<Registry>(include_str!(
            "../../../../fixtures/crs/v1/registry.json"
        ))
        .map(|r| r.systems)
        .unwrap_or_default();
        // The web's list: one group per datum, in the order the datums first come.
        let mut datums: Vec<&str> = Vec::new();
        for s in &all {
            if !datums.contains(&s.datum.as_str()) {
                datums.push(&s.datum);
            }
        }
        datums
            .iter()
            .flat_map(|d| all.iter().filter(move |s| s.datum == *d).cloned())
            .collect()
    })
}

pub fn system(srid: u32) -> Option<&'static System> {
    systems().iter().find(|s| s.srid == srid)
}

/// The web's `DATUM_LABEL`.
pub fn datum_label(datum: &str) -> &str {
    match datum {
        "TUREF" => "TUREF (ITRF96)",
        "ED50" => "ED50",
        "WGS84" => "WGS 84",
        other => other,
    }
}

/// The answer to the question: the source's system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrsQuestion {
    pub srid: u32,
}

impl CrsQuestion {
    /// The project's system, the default answer.
    pub fn new(project: u32) -> Self {
        Self { srid: project }
    }

    /// Whether the file's system is the project's: the only case that can be imported.
    pub fn matches(&self, project: u32) -> bool {
        self.srid == project
    }

    /// Under the list: the hint when the systems agree, else why the import is off.
    fn note(&self, project: u32) -> Result<String, String> {
        let project_name =
            system(project).map_or_else(|| format!("EPSG:{project}"), |s| s.name.clone());
        let source = system(self.srid);
        let (Some(source), Some(target)) = (source, system(project)) else {
            return Ok(format!(
                "Projenin sistemi ({project_name}). Koordinatlar olduğu gibi alınır; dönüştürülmez, yuvarlanmaz."
            ));
        };
        if self.matches(project) {
            return Ok(format!(
                "Projenin sistemi ({project_name}). Koordinatlar olduğu gibi alınır; dönüştürülmez, yuvarlanmaz."
            ));
        }
        let what = if source.datum != target.datum {
            format!(
                "{} → {} datum dönüşümü",
                datum_label(&source.datum),
                datum_label(&target.datum)
            )
        } else if source.kind != target.kind {
            "coğrafi ile projeksiyonlu sistem arasında dönüşüm".to_owned()
        } else {
            "dilim dönüşümü".to_owned()
        };
        Err(format!(
            "Koordinatlar dönüştürülemez. Proje {project_name} (EPSG:{project}) sisteminde. {} koordinatlarını almak için {what} gerekir; bu dönüşüm henüz yok (geliştirme aşamasında) ve koordinatlar sessizce dönüştürülmez, bu yüzden içe aktarma kapalı. Dosya aslında projenin sistemindeyse onu seçin; proje de bu sistemdeyse projenin sistemini Proje ayarları → Koordinat sistemi'nden atayın.",
            source.name
        ))
    }

    /// The question: the list of systems and the note under it.
    pub fn view<'a, Message: Clone + 'a>(
        &self,
        project: u32,
        on_pick: impl Fn(u32) -> Message + 'a,
    ) -> Element<'a, Message> {
        let list = systems();
        let choices = list.iter().map(|s| {
            let own = if s.srid == project {
                ", projenin sistemi"
            } else {
                ""
            };
            Choice::new(format!("{} (EPSG:{}){own}", s.name, s.srid)).detail(datum_label(&s.datum))
        });
        let selected = list.iter().position(|s| s.srid == self.srid);
        let pick = Select::new(choices, selected, move |i| {
            on_pick(list.get(i).map_or(project, |s| s.srid))
        });
        let note: Element<'a, Message> = match self.note(project) {
            Ok(hint) => label::caption(hint).into(),
            Err(warning) => Banner::warning(warning).into(),
        };
        column![
            label::caption("Bu koordinatlar hangi sistemde?"),
            pick,
            note
        ]
        .spacing(6)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_the_web_registry_grouped_by_datum() {
        let datums: Vec<&str> = systems().iter().map(|s| s.datum.as_str()).collect();
        let first_ed50 = datums.iter().position(|d| *d == "ED50").expect("ED50");
        let first_wgs = datums.iter().position(|d| *d == "WGS84").expect("WGS 84");
        assert!(datums[..first_ed50].iter().all(|d| *d == "TUREF"));
        assert!(datums[first_ed50..first_wgs].iter().all(|d| *d == "ED50"));
        assert!(datums[first_wgs..].iter().all(|d| *d == "WGS84"));
        assert_eq!(system(5256).map(|s| s.name.as_str()), Some("TUREF / TM36"));
    }

    #[test]
    fn only_the_projects_system_imports_and_the_reason_is_named() {
        let q = CrsQuestion::new(5256);
        assert!(q.matches(5256) && q.note(5256).is_ok());
        let zone = CrsQuestion { srid: 5254 }
            .note(5256)
            .expect_err("another zone");
        assert!(zone.contains("dilim dönüşümü"), "{zone}");
        let datum = CrsQuestion { srid: 2322 }.note(5256).expect_err("ED50");
        assert!(
            datum.contains("ED50 → TUREF (ITRF96) datum dönüşümü"),
            "{datum}"
        );
        let kind = CrsQuestion { srid: 5252 }
            .note(5256)
            .expect_err("geographic");
        assert!(kind.contains("coğrafi ile projeksiyonlu"), "{kind}");
    }
}
