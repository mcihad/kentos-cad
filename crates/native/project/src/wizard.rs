//! What the Yeni proje wizard asks, as data (docs/adr/0165 §3, 0626 §1; the web's
//! `model/newProjectWizard.ts`): its choices, what each step offers for them,
//! what the rail and the summary say, and the project they make. A CAD project
//! is asked only what it is for ([`WORKS`]): its coordinates, unit and drawing
//! scale come with it; a CBS project its province and system; both their name.
//! The texts' heights and the typeface are Proje ayarları's. The desktop's
//! window (apps/desktop/src/project/wizard) draws it; fixtures/project/v1/wizard.json
//! holds its answers for a few drafts (written here while the web waits, ADR 0228).

use kentos_contracts::{AnnotationHeights, AnnotationKind, DrawingFont, DrawingUnit, Workspace};

use crate::crs::{LOCAL_SRID, System, system, systems, turef_zone_for};
use crate::new_project::{NEW_PROJECT_NAME, NewProject, js_number};
use crate::provinces::{Province, lower_tr, province};

/// The steps, in order: the project's type, its coordinates, its scale and details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Type,
    Coords,
    Details,
}

pub const STEPS: [Step; 3] = [Step::Type, Step::Coords, Step::Details];

impl Step {
    /// The step's name, as the rail writes it: a CAD project's second step asks what it is for, a CBS
    /// project's where it is (docs/adr/0626 §1).
    pub fn name(self, kind: Kind) -> &'static str {
        match (self, kind) {
            (Self::Type, _) => "Proje türü",
            (Self::Coords, Kind::Cad) => "Çizim",
            (Self::Coords, Kind::Gis) => "Konum",
            (Self::Details, _) => "Proje adı",
        }
    }

    /// Its place among the steps (0 for the first).
    pub fn index(self) -> usize {
        STEPS.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// The two types a wizard makes; the others are announced, not chosen yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Cad,
    Gis,
}

impl Kind {
    pub fn workspace(self) -> Workspace {
        match self {
            Self::Cad => Workspace::Cad,
            Self::Gis => Workspace::Gis,
        }
    }
}

/// A CAD project's coordinates: none (a drawing of its own, from 0,0) or a real coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Coords {
    Local,
    Real,
}

/// What a CAD drawing is for (docs/adr/0626 §1): the wizard's one question about it. Its coordinates and
/// unit come with it, its drawing scale too ([`Draft::scale`]); Proje ayarları changes any of them later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    pub name: &'static str,
    /// What it is for and what it brings, under its name.
    pub note: &'static str,
    /// Its card's icon, by the icons' names.
    pub icon: &'static str,
    pub coords: Coords,
    /// A local drawing's unit; a drawing on real coordinates is in metres.
    pub unit: DrawingUnit,
}

/// The three a CAD project is made for: a building in metres at 1:100, a map or a zoning plan on the
/// province's system in metres at 1:1000, a part or a detail in millimetres at 1:1.
pub const WORKS: [Work; 3] = [
    Work {
        name: "Mimari proje",
        note: "Bina, kat planı, kesit ve görünüş. Metre, 1:100; çizim 0,0’dan başlar.",
        icon: "space",
        coords: Coords::Local,
        unit: DrawingUnit::M,
    },
    Work {
        name: "Harita / imar planı",
        note: "Vaziyet, imar, halihazır ve altyapı. Metre, 1:1000; ilin koordinat sisteminde.",
        icon: "parcel",
        coords: Coords::Real,
        unit: DrawingUnit::M,
    },
    Work {
        name: "Detay / imalat",
        note: "Parça, detay ve imalat çizimi. Milimetre, 1:1; çizim 0,0’dan başlar.",
        icon: "dimLinear",
        coords: Coords::Local,
        unit: DrawingUnit::Mm,
    },
];

/// What the wizard holds while it is open; what is not chosen follows the others.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    #[serde(rename = "type")]
    pub kind: Kind,
    /// A CAD project's coordinates; a CBS project always has a system.
    pub coords: Coords,
    /// A local project's unit.
    pub unit: DrawingUnit,
    /// The province's plate code, whose centre the project opens on and whose zone is suggested.
    pub province: Option<u32>,
    /// The system chosen; none: the zone suggested for the province, else the app's default.
    pub srid: Option<u32>,
    /// The scale chosen (1:N); none: the type's own.
    pub plot_scale: Option<f64>,
    pub name: String,
    pub font: DrawingFont,
    /// The app's default system for new projects (Uygulama ayarları → Yeni projeler).
    pub fallback_srid: u32,
    /// The annotation heights on paper the user typed (mm); none: the kinds' own
    /// (docs/adr/0205 §1, 0622 §2).
    #[serde(default)]
    pub annotation: Option<AnnotationHeights>,
}

/// Map scales for a CBS project, drawing scales for a CAD one (1:N).
pub const GIS_SCALES: [f64; 7] = [500.0, 1000.0, 2000.0, 5000.0, 10_000.0, 25_000.0, 50_000.0];
pub const CAD_SCALES: [f64; 10] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0];
/// A plan's scales (docs/adr/0622 §2): a CAD drawing in metres is a plan, from a site's
/// 1:100 to a zoning plan's 1:1000 and 1:5000.
pub const PLAN_SCALES: [f64; 8] = [
    100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10_000.0, 25_000.0,
];

/// A CAD drawing's scales in its unit (docs/adr/0622 §2): a plan's in metres, the drawing
/// scales in any other.
pub fn cad_scales(unit: DrawingUnit) -> &'static [f64] {
    match unit {
        DrawingUnit::M => &PLAN_SCALES,
        DrawingUnit::Mm | DrawingUnit::Cm | DrawingUnit::In | DrawingUnit::Ft => &CAD_SCALES,
    }
}

/// The scale a local CAD drawing in this unit starts at (docs/adr/0626 §1): a part in millimetres 1:1, a
/// room in centimetres 1:50, a building in metres 1:100; feet and inches 1:1. A drawing on real
/// coordinates is a plan at 1:1000.
pub fn cad_default_scale(unit: DrawingUnit) -> f64 {
    match unit {
        DrawingUnit::Mm | DrawingUnit::In | DrawingUnit::Ft => 1.0,
        DrawingUnit::Cm => 50.0,
        DrawingUnit::M => 100.0,
    }
}

/// An annotation `mm` high on paper at `1:scale` as the drawing measures it in its unit:
/// `2,5 m`, `5 cm`, `2,5 mm` (docs/adr/0622 §2).
pub fn drawn_height_text(mm: f64, scale: f64, unit: DrawingUnit) -> String {
    let metres = kentos_contracts::paper_height(mm, scale);
    let (value, mark) = match unit {
        DrawingUnit::Mm => (metres * 1000.0, "mm"),
        DrawingUnit::Cm => (metres * 100.0, "cm"),
        DrawingUnit::M => (metres, "m"),
        DrawingUnit::In => (metres / 0.0254, "in"),
        DrawingUnit::Ft => (metres / 0.3048, "ft"),
    };
    format!("{} {mark}", tr_number(value))
}

/// A unit a local project is drawn in, with what it is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitChoice {
    pub id: DrawingUnit,
    pub name: &'static str,
    pub mark: &'static str,
    pub note: &'static str,
}

pub const UNITS: [UnitChoice; 5] = [
    UnitChoice {
        id: DrawingUnit::Mm,
        name: "Milimetre",
        mark: "mm",
        note: "Mimari çizim, detay ve imalat",
    },
    UnitChoice {
        id: DrawingUnit::Cm,
        name: "Santimetre",
        mark: "cm",
        note: "Santimetre ile hazırlanan çizimler",
    },
    UnitChoice {
        id: DrawingUnit::M,
        name: "Metre",
        mark: "m",
        note: "Vaziyet, imar, harita ve altyapı",
    },
    // Feet and inches (docs/adr/0262 §1).
    UnitChoice {
        id: DrawingUnit::In,
        name: "İnç",
        mark: "in",
        note: "Ayak ve inçle çalışan ofisler, imalat",
    },
    UnitChoice {
        id: DrawingUnit::Ft,
        name: "Ayak",
        mark: "ft",
        note: "Ayak ve inçle mimari ve vaziyet",
    },
];

fn unit_choice(id: DrawingUnit) -> &'static UnitChoice {
    UNITS.iter().find(|u| u.id == id).unwrap_or(&UNITS[2])
}

/// The layers the project starts with, as the summary names them.
fn layers(kind: Kind) -> &'static str {
    match kind {
        Kind::Cad => "Teknik çizim: Çizim, Ölçü, Yazı, Tarama, Yardımcı, Eksen",
        Kind::Gis => "Kadastro paftası: Taslak, Kadastro, Ulaşım, Topografya, Jeodezi, Pafta",
    }
}

/// A scale as written: 1:25.000 with Turkish digit groups (the web's `scaleText`).
pub fn scale_text(n: f64) -> String {
    format!("1:{}", tr_number(n))
}

/// The plot scales a project's Ölçek offers (docs/adr/0205 §4): its type's
/// (a CAD project's drawing scales, else the map scales), with the current
/// one in its place when it is none of them. The web's `projectScales`.
pub fn project_scales(cad: bool, unit: DrawingUnit, current: f64) -> Vec<f64> {
    let mut out: Vec<f64> = if cad {
        cad_scales(unit).to_vec()
    } else {
        GIS_SCALES.to_vec()
    };
    if current.is_finite() && current > 0.0 && !out.contains(&current) {
        out.push(current);
        out.sort_by(f64::total_cmp);
    }
    out
}

/// `n.toLocaleString('tr-TR')`: digits grouped by dots, at most three decimals after a comma.
fn tr_number(n: f64) -> String {
    let thousandths = (n.abs() * 1000.0).round();
    let whole = (thousandths / 1000.0).trunc();
    let fraction = thousandths - whole * 1000.0;
    let digits = js_number(whole);
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if fraction > 0.0 {
        let f = format!("{:03}", fraction as u32);
        out.push(',');
        out.push_str(f.trim_end_matches('0'));
    }
    if n < 0.0 && thousandths > 0.0 {
        format!("-{out}")
    } else {
        out
    }
}

/// A system as a sentence names it (the web's `crsTitle`).
fn title(s: &System) -> String {
    if s.is_local() {
        "Yerel (koordinat sistemi yok)".to_owned()
    } else {
        format!("{} (EPSG:{})", s.name, s.srid)
    }
}

impl Draft {
    /// A new wizard's draft: the app's last type, default system, typeface and unit.
    pub fn initial(
        kind: Workspace,
        fallback_srid: u32,
        font: DrawingFont,
        unit: Option<DrawingUnit>,
    ) -> Self {
        Self {
            kind: if kind == Workspace::Cad {
                Kind::Cad
            } else {
                Kind::Gis
            },
            coords: Coords::Local,
            unit: unit.unwrap_or(DrawingUnit::M),
            province: None,
            srid: None,
            plot_scale: None,
            name: NEW_PROJECT_NAME.to_owned(),
            font,
            fallback_srid: if fallback_srid != LOCAL_SRID && system(fallback_srid).is_some() {
                fallback_srid
            } else {
                5256
            },
            annotation: None,
        }
    }

    /// The work a CAD draft is made for, when its coordinates and unit are one's (a request may ask others).
    pub fn work(&self) -> Option<&'static Work> {
        if self.kind != Kind::Cad {
            return None;
        }
        WORKS
            .iter()
            .find(|w| w.coords == self.coords && (w.coords == Coords::Real || w.unit == self.unit))
    }

    /// The draft made for `work`: its coordinates and unit, the scale its own.
    pub fn choose_work(&mut self, work: &Work) {
        self.coords = work.coords;
        self.unit = work.unit;
        self.plot_scale = None;
    }

    /// Whether the project is a drawing of its own, with no coordinate system.
    pub fn is_local(&self) -> bool {
        self.kind == Kind::Cad && self.coords == Coords::Local
    }

    /// The province chosen, if any.
    pub fn province(&self) -> Option<&'static Province> {
        self.province.and_then(province)
    }

    /// The TUREF zone of the province's longitude, if a province is chosen.
    pub fn suggested_srid(&self) -> Option<u32> {
        self.province()
            .and_then(|p| turef_zone_for(p.lon))
            .map(|s| s.srid)
    }

    /// The project's system: the local one, or the one chosen, suggested or defaulted.
    pub fn srid(&self) -> u32 {
        if self.is_local() {
            LOCAL_SRID
        } else {
            self.srid
                .or_else(|| self.suggested_srid())
                .unwrap_or(self.fallback_srid)
        }
    }

    /// The scales offered for the draft: a map's, or a drawing's in its unit
    /// (a CAD project on real coordinates is a plan in metres; docs/adr/0622 §2).
    pub fn scales(&self) -> &'static [f64] {
        match self.kind {
            Kind::Cad => cad_scales(self.unit()),
            Kind::Gis => &GIS_SCALES,
        }
    }

    /// The project's scale: the one chosen, else its unit's for a local drawing (a
    /// plan in metres 1:1000, docs/adr/0622 §2) and 1:1000 for any other.
    pub fn scale(&self) -> f64 {
        self.plot_scale.unwrap_or(if self.is_local() {
            cad_default_scale(self.unit)
        } else {
            1000.0
        })
    }

    /// A kind of annotation's height on paper, mm: the one typed, else its own.
    pub fn annotation_mm(&self, kind: AnnotationKind) -> f64 {
        self.annotation
            .as_ref()
            .map_or_else(|| kind.default_mm(), |a| a.mm(kind))
    }

    /// The project's unit: a local drawing's own, metres for any other.
    pub fn unit(&self) -> DrawingUnit {
        if self.is_local() {
            self.unit
        } else {
            DrawingUnit::M
        }
    }

    /// The systems a project with coordinates chooses from: the suggested
    /// one first, then every other in the list's order (the local one is
    /// chosen on its own card).
    pub fn system_choices(&self) -> Vec<(&'static System, bool)> {
        let suggested = self.suggested_srid();
        let all = systems().iter().filter(|s| !s.is_local());
        all.clone()
            .filter(|s| Some(s.srid) == suggested)
            .map(|s| (s, true))
            .chain(
                all.filter(|s| Some(s.srid) != suggested)
                    .map(|s| (s, false)),
            )
            .collect()
    }

    /// What the rail writes under a step's name: the choice made there.
    pub fn note(&self, step: Step) -> String {
        match step {
            Step::Type => match self.kind {
                Kind::Cad => "CAD · teknik çizim".to_owned(),
                Kind::Gis => "CBS · harita".to_owned(),
            },
            Step::Coords => {
                // A CAD drawing by what it is for and its unit and scale; a plan and a CBS project by where.
                if self.is_local() {
                    let what = self.work().map_or("Yerel", |w| w.name);
                    return format!(
                        "{what} · {} · {}",
                        unit_choice(self.unit).mark,
                        scale_text(self.scale())
                    );
                }
                [
                    self.province().map(|p| p.name.as_str()),
                    system(self.srid()).map(|s| s.name.as_str()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ")
            }
            Step::Details => self.shown_name().to_owned(),
        }
    }

    fn shown_name(&self) -> &str {
        match self.name.trim() {
            "" => NEW_PROJECT_NAME,
            name => name,
        }
    }

    /// The summary of the last step, line by line: what the project is, where, in which unit and at which
    /// scale, what it starts with (docs/adr/0626 §1).
    pub fn summary(&self) -> Vec<(&'static str, String)> {
        let local = self.is_local();
        let srid = self.srid();
        let p = self.province();
        let mut lines = vec![(
            "Tür",
            match self.kind {
                Kind::Cad => "CAD, teknik çizim",
                Kind::Gis => "CBS, coğrafi bilgi sistemi",
            }
            .to_owned(),
        )];
        if let Some(w) = self.work() {
            lines.push(("Çizim", w.name.to_owned()));
        }
        lines.push((
            "Koordinatlar",
            if local {
                "Yerel: koordinat sistemi yok, çizim 0,0’dan başlar".to_owned()
            } else {
                system(srid).map_or_else(|| format!("EPSG:{srid}"), title)
            },
        ));
        if let (false, Some(p)) = (local, p) {
            lines.push(("İl", p.name.clone()));
        }
        lines.push(("Birim", lower_tr(unit_choice(self.unit()).name)));
        lines.push((
            match self.kind {
                Kind::Cad => "Çizim ölçeği",
                Kind::Gis => "Harita ölçeği",
            },
            scale_text(self.scale()),
        ));
        lines.push(("Katmanlar", layers(self.kind).to_owned()));
        lines.push((
            "Açılış",
            if local {
                "A3 kâğıt, yatay; 0,0 sol altta".to_owned()
            } else if let Some(p) = p {
                format!("{} merkezinde bir pafta", p.name)
            } else {
                "Dilimin çalışma alanında bir pafta".to_owned()
            },
        ));
        lines
    }

    /// Why the step cannot be left forward, or none.
    pub fn blocked(&self, step: Step) -> Option<String> {
        match step {
            Step::Details if self.name.trim().is_empty() => {
                Some("Proje adı boş olamaz.".to_owned())
            }
            Step::Coords if !self.is_local() && system(self.srid()).is_none() => Some(format!(
                "EPSG:{} bu sürümde tanımlı değil; listeden bir sistem seçin.",
                self.srid()
            )),
            _ => None,
        }
    }

    /// The project the draft makes.
    pub fn options(&self) -> NewProject {
        let local = self.is_local();
        NewProject {
            name: self.name.clone(),
            srid: self.srid(),
            plot_scale: self.scale(),
            workspace: self.kind.workspace(),
            drawing_font: self.font,
            province: if local { None } else { self.province },
            drawing_unit: local.then_some(self.unit),
            annotation: self.annotation.clone().and_then(AnnotationHeights::sanitized),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    const FIXTURE: &str = include_str!("../../../../fixtures/project/v1/wizard.json");

    /// A number as JSON writes it: a whole one without its point, as the web wrote them.
    fn num(x: f64) -> Value {
        if x.fract() == 0.0 && x.abs() < 1e15 {
            Value::from(x as i64)
        } else {
            Value::from(x)
        }
    }

    /// A draft's answers as the fixture holds them.
    fn answers(d: &Draft) -> Value {
        let o = d.options();
        let mut options = serde_json::json!({
            "name": o.name,
            "srid": o.srid,
            "plotScale": num(o.plot_scale),
            "workspace": o.workspace,
            "drawingFont": o.drawing_font,
        });
        if let Some(p) = o.province {
            options["province"] = Value::from(p);
        }
        if let Some(u) = o.drawing_unit {
            options["drawingUnit"] = serde_json::to_value(u).expect("a unit");
        }
        if let Some(a) = &o.annotation {
            options["annotation"] = serde_json::to_value(a).expect("heights");
        }
        serde_json::json!({
            "srid": d.srid(),
            "suggested": d.suggested_srid(),
            "scale": num(d.scale()),
            "unit": d.unit(),
            "notes": STEPS.iter().map(|s| d.note(*s)).collect::<Vec<_>>(),
            "summary": d.summary().into_iter().map(|(k, v)| vec![k.to_owned(), v]).collect::<Vec<_>>(),
            "firstSystem": d.system_choices().first().map(|(s, _)| s.srid),
            "options": options,
        })
    }

    /// The wizard's answers for the fixture's drafts (fixtures/project/v1/wizard.json). The web recorded
    /// them first (newProjectWizardFixture.ts); while it waits (ADR 0228) the desktop writes them:
    /// `KENTOS_WRITE_WIZARD=1 cargo test -p kentos-project wizard` (docs/adr/0626 §1).
    #[test]
    fn the_wizard_answers_as_the_fixture_says() {
        let mut file: Value = serde_json::from_str(FIXTURE).expect("the fixture reads");
        assert_eq!(file["format"], "kentos.new-project-wizard");
        let write = std::env::var_os("KENTOS_WRITE_WIZARD").is_some();
        let cases = file["cases"].as_array_mut().expect("cases");
        assert!(cases.len() >= 7);
        for case in cases.iter_mut() {
            let name = case["name"].as_str().expect("a name").to_owned();
            let d: Draft = serde_json::from_value(case["draft"].clone()).expect("a draft");
            let got = answers(&d);
            if write {
                for (k, v) in got.as_object().expect("answers") {
                    case[k] = v.clone();
                }
                continue;
            }
            for (k, v) in got.as_object().expect("answers") {
                assert_eq!(&case[k], v, "{name}: {k}");
            }
        }
        if write {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/project/v1/wizard.json");
            let text = serde_json::to_string_pretty(&file).expect("the fixture writes") + "\n";
            std::fs::write(path, text).expect("the fixture is written");
        }
    }

    #[test]
    fn a_new_draft_starts_on_the_apps_choices_and_its_steps_say_what_stops_them() {
        let d = Draft::initial(Workspace::Plan3d, 0, DrawingFont::Arimo, None);
        // Only CAD and CBS are made yet; the local system is no default for a system.
        assert_eq!(
            (d.kind, d.fallback_srid, d.unit, d.font),
            (Kind::Gis, 5256, DrawingUnit::M, DrawingFont::Arimo)
        );
        let d = Draft::initial(
            Workspace::Cad,
            5254,
            DrawingFont::Barlow,
            Some(DrawingUnit::Mm),
        );
        assert!(d.is_local());
        assert_eq!(
            (d.srid(), d.scale(), d.unit()),
            (LOCAL_SRID, 1.0, DrawingUnit::Mm)
        );
        assert_eq!(d.blocked(Step::Coords), None);
        let real = Draft {
            coords: Coords::Real,
            srid: Some(1234),
            ..d.clone()
        };
        assert_eq!(
            real.blocked(Step::Coords).as_deref(),
            Some("EPSG:1234 bu sürümde tanımlı değil; listeden bir sistem seçin.")
        );
        let unnamed = Draft {
            name: "  ".into(),
            ..d
        };
        assert_eq!(
            unnamed.blocked(Step::Details).as_deref(),
            Some("Proje adı boş olamaz.")
        );
        assert_eq!(unnamed.note(Step::Details), "Yeni proje");
        assert_eq!(STEPS.map(Step::index), [0, 1, 2]);
    }

    /// A CAD project is asked what it is for (docs/adr/0626 §1): each work brings its coordinates, unit and
    /// drawing scale, and the rail and the summary say it.
    #[test]
    fn a_work_brings_its_coordinates_unit_and_scale() {
        let mut d = Draft::initial(Workspace::Cad, 5256, DrawingFont::Barlow, None);
        assert_eq!(d.work().map(|w| w.name), Some("Mimari proje"));
        let mut seen = Vec::new();
        for w in &WORKS {
            d.plot_scale = Some(20.0);
            d.choose_work(w);
            assert_eq!(d.work(), Some(w));
            let o = d.options();
            seen.push((w.name, o.srid, d.unit(), o.plot_scale, o.drawing_unit));
        }
        assert_eq!(
            seen,
            [
                ("Mimari proje", LOCAL_SRID, DrawingUnit::M, 100.0, Some(DrawingUnit::M)),
                ("Harita / imar planı", 5256, DrawingUnit::M, 1000.0, None),
                ("Detay / imalat", LOCAL_SRID, DrawingUnit::Mm, 1.0, Some(DrawingUnit::Mm)),
            ]
        );
        assert_eq!(d.note(Step::Coords), "Detay / imalat · mm · 1:1");
        assert_eq!(
            d.summary().into_iter().take(2).collect::<Vec<_>>(),
            [("Tür", "CAD, teknik çizim".to_owned()), ("Çizim", "Detay / imalat".to_owned())]
        );
        // A unit no work has is the local drawing's own.
        d.unit = DrawingUnit::Cm;
        assert_eq!((d.work(), d.note(Step::Coords).as_str()), (None, "Yerel · cm · 1:50"));
        assert_eq!(
            [Step::Type, Step::Coords, Step::Details].map(|s| s.name(Kind::Cad)),
            ["Proje türü", "Çizim", "Proje adı"]
        );
        assert_eq!(Step::Coords.name(Kind::Gis), "Konum");
    }

    #[test]
    fn scales_are_written_in_turkish_groups() {
        assert_eq!(
            [1.0, 1000.0, 25_000.0, 1_000_000.0, 2.5, 0.125].map(scale_text),
            [
                "1:1",
                "1:1.000",
                "1:25.000",
                "1:1.000.000",
                "1:2,5",
                "1:0,125"
            ]
        );
    }
}
