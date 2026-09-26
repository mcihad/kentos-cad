//! “Bu koordinatlar hangi sistemde?” (the web's `CrsQuestion`,
//! apps/web/src/ui/io/common.ts). A source file carries no reliable
//! coordinate system, so the user says which; the project's is the default.
//! Another system blocks the import: datum and zone transformations do not
//! exist yet and coordinates are never reprojected silently (CLAUDE.md §5).
//! The systems are the web's registry (fixtures/crs/v1/registry.json), in its
//! order and grouped by datum as the web's list is; a new project and the
//! project settings choose from the same list.

use std::sync::OnceLock;

use iced::widget::{Column, button, column, container, row, scrollable, text, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::typography;
use kentos_ui::widget::Banner;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

/// A coordinate system of the registry.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    pub srid: u32,
    pub name: String,
    /// `projected` or `geographic`.
    pub kind: String,
    /// `TUREF`, `ED50` or `WGS84`.
    pub datum: String,
    /// `Transverse Mercator`, `UTM` or `Pseudo-Mercator`; none for a geographic system.
    #[serde(default)]
    pub projection: Option<String>,
    #[serde(default)]
    pub false_easting: Option<f64>,
    #[serde(default)]
    pub false_northing: Option<f64>,
    pub ellipsoid: String,
    #[serde(default)]
    pub central_meridian: Option<f64>,
    #[serde(default)]
    pub scale_factor: Option<f64>,
    /// `metre` or `degree`.
    pub unit: String,
    /// Where the system is meant for (“25.5°–28.5° D (3° dilim)”).
    #[serde(default)]
    pub area: Option<String>,
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
            "../../../fixtures/crs/v1/registry.json"
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

/// What the picker is for (the web's `CrsPickerOptions.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickFor {
    /// A new, empty project: nothing to reproject.
    New,
    /// The open project's system: assigned, never a transformation.
    Assign,
}

/// The systems a search finds (the web's `searchCrs`): the start of the
/// SRID (`epsg:` left out), or a part of the name or the area.
pub fn search(query: &str) -> Vec<&'static System> {
    let q = query.trim().to_lowercase();
    let q = q
        .strip_prefix("epsg:")
        .or_else(|| q.strip_prefix("epsg"))
        .unwrap_or(&q)
        .trim();
    systems()
        .iter()
        .filter(|s| {
            q.is_empty()
                || s.srid.to_string().starts_with(q)
                || s.name.to_lowercase().contains(q)
                || s.area
                    .as_deref()
                    .is_some_and(|a| a.to_lowercase().contains(q))
        })
        .collect()
}

/// The searchable list of systems grouped by datum, the chosen one's card
/// and its parameters, and the notes (the web's `crsPicker`).
pub fn picker<'a, Message: Clone + 'a>(
    value: u32,
    initial: u32,
    default_srid: u32,
    mode: PickFor,
    query: &str,
    on_pick: impl Fn(u32) -> Message + 'a,
    on_search: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let Some(chosen) = system(value) else {
        return label::caption(format!("EPSG:{value} bu sürümde tanımlı değil.")).into();
    };
    let changed = value != initial && mode == PickFor::Assign;
    let heading = match (mode, changed) {
        (PickFor::New, _) => "Yeni projenin koordinat sistemi",
        (PickFor::Assign, true) => "Kaydedince projeye atanacak sistem",
        (PickFor::Assign, false) => "Projenin koordinat sistemi",
    };
    let current = container(
        column![
            label::caption(heading),
            row![
                text(chosen.name.clone())
                    .font(typography::ui_strong())
                    .size(typography::body()),
                container(label::mono_caption(format!("EPSG:{}", chosen.srid)))
                    .padding([1, 6])
                    .style(style::container::badge),
            ]
            .spacing(8)
            .align_y(Center),
        ]
        .spacing(4),
    )
    .padding([8, 10])
    .width(Fill)
    .style(move |theme: &iced::Theme| {
        let base = style::container::bordered(theme);
        if !changed {
            return base;
        }
        // About to change: the web's amber edge (“değişecek”).
        iced::widget::container::Style {
            border: iced::Border {
                color: kentos_ui::theme::Tokens::of(theme).warning,
                ..base.border
            },
            ..base
        }
    });

    let found = search(query);
    let digits: String = query.chars().filter(char::is_ascii_digit).collect();
    let unknown = !digits.is_empty()
        && digits.len() >= 4
        && query
            .trim()
            .trim_start_matches(|c: char| !c.is_ascii_digit())
            .chars()
            .all(|c| c.is_ascii_digit())
        && digits.parse::<u32>().ok().and_then(system).is_none();
    let mut list = Column::new().spacing(2);
    let mut datum = "";
    for s in &found {
        if s.datum != datum {
            datum = &s.datum;
            list = list.push(container(label::caption(datum_label(datum))).padding(
                iced::Padding {
                    top: 6.0,
                    right: 4.0,
                    bottom: 2.0,
                    left: 4.0,
                },
            ));
        }
        let tag: Element<'a, Message> = if s.srid == default_srid {
            container(label::caption("varsayılan"))
                .padding([0, 5])
                .style(style::container::badge)
                .into()
        } else {
            label::caption("").into()
        };
        let face = row![
            container(label::mono(s.srid.to_string())).width(52),
            row![label::body(s.name.clone()), tag]
                .spacing(6)
                .align_y(Center)
                .width(Fill),
            label::caption(s.area.clone().unwrap_or_default()),
        ]
        .spacing(8)
        .align_y(Center);
        list = list.push(
            button(face)
                .on_press(on_pick(s.srid))
                .padding([4, 6])
                .width(Fill)
                .style(style::button::list_item(s.srid == value)),
        );
    }
    if found.is_empty() {
        list = list.push(container(label::muted("Eşleşen koordinat sistemi yok.")).padding(8));
    }
    let search_box = text_input("SRID ya da ad yazın, ör. 5256 veya TM36", query)
        .on_input(on_search)
        .padding([5, 8])
        .size(typography::body())
        .style(style::field::input);
    let mut browser = Column::new().spacing(6).push(
        row![icon(Icon::Search).size(14.0).tone(Tone::Muted), search_box]
            .spacing(6)
            .align_y(Center),
    );
    if unknown {
        browser = browser.push(label::caption(format!(
            "EPSG:{digits} bu sürümde tanımlı değil. Listedeki sistemlerden birini seçin."
        )));
    }
    let browser = browser.push(
        container(
            scrollable(list)
                .direction(style::field::thin_scrollbar())
                .height(Length::Fixed(230.0)),
        )
        .style(style::container::bordered),
    );

    let mut details: Vec<(&str, String)> = vec![
        (
            "Tür",
            if chosen.kind == "projected" {
                "Projeksiyonlu (metre)".to_owned()
            } else {
                "Coğrafi (derece)".to_owned()
            },
        ),
        ("Datum", datum_label(&chosen.datum).to_owned()),
        ("Elipsoid", chosen.ellipsoid.clone()),
    ];
    if let Some(p) = &chosen.projection {
        details.push((
            "Projeksiyon",
            if p == "UTM" {
                "UTM (Transverse Mercator)".to_owned()
            } else {
                p.clone()
            },
        ));
    }
    if let Some(cm) = chosen.central_meridian {
        details.push(("Orta meridyen", format!("{}° D", js_number(cm))));
    }
    if let Some(k) = chosen.scale_factor {
        details.push(("Ölçek faktörü", js_number(k)));
    }
    if let Some(e) = chosen.false_easting {
        details.push(("Sağa öteleme", format!("{} m", grouped(e))));
    }
    details.push(("Kapsam", chosen.area.clone().unwrap_or_default()));
    let mut card = Column::new().spacing(4).push(
        text(chosen.name.clone())
            .font(typography::ui_strong())
            .size(typography::body()),
    );
    for (k, v) in details {
        card = card.push(row![container(label::caption(k)).width(110), label::body(v)].spacing(8));
    }
    if chosen.kind == "projected" {
        card = card.push(label::caption(
            "Eksen sırası: Y sağa değer, X yukarı değer.",
        ));
    }
    let card = container(card)
        .padding(10)
        .width(Fill)
        .style(style::container::bordered);

    let mut notes = Column::new().spacing(6);
    if mode == PickFor::Assign && value != initial {
        let before = system(initial);
        let datum_note = match before {
            Some(b) if b.datum != chosen.datum => format!(
                " {} → {} geçişi için datum dönüşümü gerekir (geliştirme aşamasında).",
                datum_label(&b.datum),
                datum_label(&chosen.datum)
            ),
            _ => String::new(),
        };
        notes = notes.push(Banner::warning(format!(
            "Koordinatlar dönüştürülmez. Kaydettiğinizde proje yalnızca {} olarak etiketlenir; mevcut Y/X değerleri aynı kalır.{datum_note}",
            chosen.name
        )));
    }
    if chosen.kind == "geographic" {
        notes = notes.push(Banner::info(
            "Coğrafi sistemlerde birim derecedir. Çizim ve ölçüm araçları metre cinsinden projeksiyonlu bir sistem bekler.",
        ));
    }
    column![
        current,
        row![
            container(browser).width(Length::FillPortion(3)),
            container(card).width(Length::FillPortion(2))
        ]
        .spacing(12),
        notes
    ]
    .spacing(10)
    .into()
}

/// A number as JavaScript writes it (`${27}` → “27”, `${0.9996}` → “0.9996”).
fn js_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// A whole number in Turkish grouping (`500000` → “500.000”, the web's `toLocaleString('tr-TR')`).
pub fn grouped(v: f64) -> String {
    let n = v.round() as i64;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
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
    fn a_search_finds_by_code_name_or_area_as_on_the_web() {
        let codes = |q: &str| search(q).iter().map(|s| s.srid).collect::<Vec<_>>();
        assert_eq!(codes("EPSG:5256"), [5256]);
        assert!(codes("tm36").contains(&5256) && codes("tm36").contains(&2322));
        assert_eq!(codes("").len(), systems().len());
        assert!(codes("coğrafi").contains(&4326));
        assert_eq!(grouped(500_000.0), "500.000");
        assert_eq!(grouped(25_000.0), "25.000");
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
