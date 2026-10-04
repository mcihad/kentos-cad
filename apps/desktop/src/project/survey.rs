//! Proje ayarları › Ölçme (docs/adr/0169 §3): the project's refraction
//! coefficient k, its mean ellipsoidal height for the ground values
//! (docs/adr/0171 §2) and the tolerances a field book's two faces are
//! checked against, typed as `kentos_project::survey_form` reads them (the
//! web's `surveySection` in `ui/settings/ProjectSettingsDialog.ts`). An
//! empty tolerance is not checked; what does not hold is said under its
//! field, and Kaydet waits.

use iced::widget::{Column, column, row, text_input};
use iced::{Center, Element};
use kentos_contracts::AngleUnit;
use kentos_project::survey_form::{self, Field};
use kentos_ui::theme::typography;
use kentos_ui::widget::{Banner, Switch};
use kentos_ui::{label, style};

use super::{group, setting};
use crate::app::Message;

/// A typed field with its unit, and under it what is wrong.
fn field<'a>(
    value: &'a str,
    placeholder: &'a str,
    unit: &'a str,
    problem: Option<&'static str>,
    on: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .on_input(on)
        .padding([5, 8])
        .size(typography::body())
        .width(120)
        .style(style::field::input);
    let mut c = Column::new().spacing(4).push(
        row![
            kentos_ui::widget::focus_ring(input),
            label::body(unit).width(28)
        ]
        .spacing(8)
        .align_y(Center),
    );
    if let Some(p) = problem {
        c = c.push(label::caption(p).style(style::text::danger).width(156));
    }
    c.into()
}

/// The section: k, the ground's height and the reduction to the grid
/// (docs/adr/0171 §2, §4), then the tolerances in the project's angle unit.
/// `reduce` is the switch as kept beside the texts; `why_not`, why it
/// cannot be turned on with these settings.
pub(super) fn view<'a>(
    texts: &'a [String; 8],
    unit: AngleUnit,
    reduce: bool,
    why_not: Option<&'static str>,
    on: impl Fn(Field, String) -> Message + Copy + 'a,
    on_reduce: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let read = survey_form::read(texts, unit);
    let mark = survey_form::angle_mark(unit);
    let typed = |f: Field, placeholder: &'a str, unit: &'a str| {
        let i = Field::ALL.iter().position(|g| *g == f).unwrap_or(0);
        field(&texts[i], placeholder, unit, read.problems[i], move |t| {
            on(f, t)
        })
    };
    column![
        group(
            "İndirgeme",
            setting(
                "Refraksiyon katsayısı (k)",
                Some("Trigonometrik kot farkına yer eğriliği ve refraksiyon düzeltmesi: (1 − k)·D²/2R, R = 6 371 000 m. Boş bırakılırsa 0.13. Karne ve Kutupsal alım bunu kullanır."),
                typed(Field::Refraction, "0.13", ""),
            ),
        ),
        group(
            "Zemin",
            Column::new()
                .spacing(10)
                .push(setting(
                    "Ortalama elipsoit yüksekliği",
                    Some("Mesafe ölç ve Alan hesapla zemin uzunluk ve alanlarını bu yükseklikte verir. Noktaların kotları kullanılmaz; boş bırakılırsa zemin değeri verilmez."),
                    typed(Field::GroundHeight, "", "m"),
                ))
                .push(setting(
                    "Hesap pencereleri",
                    Some(why_not.unwrap_or("Kutupsal alım ve Poligon hesabı ölçülen yatay uzunlukları düzleme indirir, Aplikasyon zemin uzunluklarını da verir; ölçek ve yükseklik çarpanı raporda.")),
                    match why_not {
                        None => Switch::new(reduce, on_reduce),
                        Some(_) => Switch::disabled(false),
                    }
                    .label("Uzunlukları projeksiyona indir"),
                )),
        ),
        group(
            "Toleranslar",
            Column::new()
                .spacing(10)
                .push(Banner::info(
                    "Boş bırakılan denetlenmez; farklar karnede yine gösterilir. Aşan değer karnede uyarı rengindedir, hesaba aktarmayı durdurmaz.",
                ))
                .push(setting(
                    "İki durum yatay açı farkı",
                    Some("Bir hedefin I. ve II. durum okumaları, yarım tur farkıyla."),
                    typed(Field::FaceHz, "", mark),
                ))
                .push(setting(
                    "İndeks hatası",
                    Some("Düşey açının iki durumundan bulunan indeks hatası."),
                    typed(Field::Index, "", mark),
                ))
                .push(setting(
                    "İki durum uzunluk farkı",
                    Some("Bir hedefin iki durumdaki eğik uzunlukları."),
                    typed(Field::FaceSlope, "", "mm"),
                )),
        ),
        group(
            "Poligon",
            Column::new()
                .spacing(10)
                .push(setting(
                    "Kenarın iki yönden farkı",
                    Some("Bir poligon kenarının iki ucundan ölçülen yatay uzunlukları."),
                    typed(Field::TwoWay, "", "mm"),
                ))
                .push(setting(
                    "Açı kapanması",
                    Some("Poligon hesabı'nın açı kapanma hatası fβ."),
                    typed(Field::TraverseAngle, "", mark),
                ))
                .push(setting(
                    "Koordinat kapanması",
                    Some("Poligon hesabı'nın koordinat kapanma hatası fs."),
                    typed(Field::TraverseCoord, "", "mm"),
                )),
        ),
    ]
    .spacing(16)
    .into()
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use kentos_contracts::{AngleUnit, SurveySettings};
    use kentos_project::survey_form::Field;

    use crate::app::{App, Message};
    use crate::project::settings::Section;
    use crate::project::{SettingsEvent, settings_message};

    fn send(app: &mut App, e: SettingsEvent) {
        let _ = app.update(settings_message(e));
    }

    fn typed(app: &mut App, field: Field, text: &str) {
        send(app, SettingsEvent::Survey(field, text.to_owned()));
    }

    /// Ölçme (docs/adr/0169 §3): what does not hold keeps Kaydet waiting;
    /// k and the tolerances go into the project in radians and metres, the
    /// angles' texts follow the angle unit, and the section's reset leaves
    /// the defaults. Kutupsal alım reads the project's k.
    #[test]
    fn the_survey_settings_are_typed_checked_and_saved() {
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Survey));
        typed(&mut app, Field::Refraction, "0,14");
        typed(&mut app, Field::FaceHz, "20");
        typed(&mut app, Field::Index, "on");
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_some(), "Kaydet waits for the index error");
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().survey, None);
        typed(&mut app, Field::Index, "10");
        typed(&mut app, Field::FaceSlope, "5");
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_none());
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert_eq!(
            settings.survey,
            Some(SurveySettings {
                refraction: Some(0.14),
                face_hz: Some(20.0 * PI / 2_000_000.0),
                index: Some(10.0 * PI / 2_000_000.0),
                face_slope: Some(0.005),
                ..SurveySettings::default()
            })
        );
        assert_eq!(settings.refraction(), 0.14);
        // In degrees the same tolerances read in arc seconds.
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::AngleUnit(AngleUnit::Deg));
        let Some(crate::project::Window::Settings(s)) = &app.project else {
            panic!("Proje ayarları is open");
        };
        assert_eq!(
            kentos_project::survey_form::texts(s_survey(s).as_ref(), AngleUnit::Deg),
            ["0.14", "6.48", "3.24", "5", "", "", "", ""]
        );
        send(&mut app, SettingsEvent::Section(Section::Survey));
        send(&mut app, SettingsEvent::ResetSection);
        send(&mut app, SettingsEvent::Save);
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert_eq!((settings.refraction(), settings.survey), (0.13, None));
    }

    fn s_survey(s: &crate::project::settings::State) -> Option<SurveySettings> {
        s.draft_survey()
    }

    /// Ortalama elipsoit yüksekliği (docs/adr/0171 §2): metres within
    /// [−500, 9000], Kaydet waits for one beyond; the reduction to the grid
    /// the project has stays while a height does (§4).
    #[test]
    fn the_ground_height_is_typed_checked_and_saved() {
        let mut app = crate::files_testing::app_with_drawing();
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.survey = Some(SurveySettings {
            ground_height: Some(120.0),
            reduce_to_grid: Some(true),
            ..SurveySettings::default()
        });
        doc.model.set_settings(settings);
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Survey));
        typed(&mut app, Field::GroundHeight, "9500");
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_some(), "Kaydet waits for the height");
        typed(&mut app, Field::GroundHeight, "850,5");
        typed(&mut app, Field::Refraction, "0.14");
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_none());
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert_eq!(
            settings.survey,
            Some(SurveySettings {
                refraction: Some(0.14),
                ground_height: Some(850.5),
                reduce_to_grid: Some(true),
                ..SurveySettings::default()
            })
        );
        assert_eq!(settings.ground_height(), Some(850.5));
        assert!(settings.reduces_to_grid());
        // Without a height the reduction goes too.
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Survey));
        typed(&mut app, Field::GroundHeight, "");
        send(&mut app, SettingsEvent::Save);
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert_eq!(
            settings.survey,
            Some(SurveySettings {
                refraction: Some(0.14),
                ..SurveySettings::default()
            })
        );
    }

    /// Uzunlukları projeksiyona indir (docs/adr/0171 §4): turned on with a
    /// height, saved; a project without a system to take lengths to cannot
    /// keep it (its reason said under the switch), the height stays.
    #[test]
    fn the_reduction_needs_a_height_and_a_scale() {
        use kentos_interaction::ground::{NEEDS_HEIGHT, NEEDS_SYSTEM, why_not_grid};

        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Survey));
        let Some(crate::project::Window::Settings(s)) = &app.project else {
            panic!("Proje ayarları is open");
        };
        assert_eq!(why_not_grid(&s.draft_settings()), Some(NEEDS_HEIGHT));
        typed(&mut app, Field::GroundHeight, "850");
        let Some(crate::project::Window::Settings(s)) = &app.project else {
            panic!("Proje ayarları is open");
        };
        assert_eq!(why_not_grid(&s.draft_settings()), None);
        send(&mut app, SettingsEvent::Reduce(true));
        send(&mut app, SettingsEvent::Save);
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert!(settings.reduces_to_grid());
        // A local project has no grid to take lengths to.
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Crs(0));
        let Some(crate::project::Window::Settings(s)) = &app.project else {
            panic!("Proje ayarları is open");
        };
        assert_eq!(why_not_grid(&s.draft_settings()), Some(NEEDS_SYSTEM));
        send(&mut app, SettingsEvent::Save);
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert!(!settings.reduces_to_grid());
        assert_eq!(settings.ground_height(), Some(850.0));
    }

    /// Kutupsal alım computes its heights with the project's k: the same as
    /// the core's with that refraction (docs/adr/0169 §3).
    #[test]
    fn kutupsal_alim_uses_the_projects_k() {
        use kentos_geometry_core::survey::polar::{PolarInput, Shot, polar_survey};
        use kentos_interaction::Vec2;

        let mut app = crate::files_testing::app_with_drawing();
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.survey = Some(SurveySettings {
            refraction: Some(0.2),
            ..SurveySettings::default()
        });
        doc.model.set_settings(settings);
        let mut form = crate::calc::polar::Form {
            station: "1000,2000".to_owned(),
            back: "1000,2500".to_owned(),
            station_z: "100".to_owned(),
            instrument_height: "1.5".to_owned(),
            ..Default::default()
        };
        form.rows[0] = [
            "P1".to_owned(),
            "50".to_owned(),
            "400".to_owned(),
            "98".to_owned(),
            "1.7".to_owned(),
        ];
        let doc = app.document.as_ref().expect("a drawing");
        let read = form.compute(&doc.model);
        let points = read.points.expect("computed");
        let core = |refraction| {
            polar_survey(&PolarInput {
                unit: "grad".to_owned(),
                station: Vec2::new(1000.0, 2000.0),
                back: Vec2::new(1000.0, 2500.0),
                back_reading: 0.0,
                station_z: Some(100.0),
                instrument_height: Some(1.5),
                shots: vec![Shot {
                    reading: 50.0,
                    distance: 400.0,
                    zenith: Some(98.0),
                    target_height: Some(1.7),
                }],
                refraction,
                grid: None,
            })
            .expect("the core computes")
        };
        assert_eq!(points[0].dz, core(Some(0.2))[0].dz);
        // 400 m at k = 0.2: (1 − k)·D²/2R ≈ 10 mm more than without.
        let without = core(None)[0].dz.expect("a height difference");
        let with = points[0].dz.expect("a height difference");
        assert!(
            (with - without - 0.0100).abs() < 0.0002,
            "{with} − {without}"
        );
    }

    /// Ölçme's pictures, in the light theme at 1440×900 and the dark at
    /// 1100×650: empty, typed, with what does not hold, and the height typed
    /// with the reduction to the grid on (docs/adr/0171 §4).
    #[test]
    #[ignore = "writes pictures: cargo test -p kentos-desktop project::survey::tests::screens -- --ignored --nocapture"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let shots: [(&str, &[(Field, &str)]); 4] = [
            ("bos", &[]),
            (
                "dolu",
                &[
                    (Field::Refraction, "0.14"),
                    (Field::GroundHeight, "850"),
                    (Field::FaceHz, "20"),
                    (Field::Index, "10"),
                    (Field::FaceSlope, "5"),
                    (Field::TwoWay, "10"),
                    (Field::TraverseAngle, "60"),
                    (Field::TraverseCoord, "30"),
                ],
            ),
            (
                "hata",
                &[
                    (Field::Refraction, "1.5"),
                    (Field::GroundHeight, "9500"),
                    (Field::FaceHz, "0"),
                    (Field::Index, "on"),
                    (Field::FaceSlope, "5"),
                    (Field::TraverseCoord, "-2"),
                ],
            ),
            ("indir", &[(Field::GroundHeight, "850")]),
        ];
        for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
            for (name, fields) in shots {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                let _ = app.update(Message::Run("file.settings"));
                send(&mut app, SettingsEvent::Section(Section::Survey));
                for (f, t) in fields {
                    typed(&mut app, *f, t);
                }
                if name == "indir" {
                    send(&mut app, SettingsEvent::Reduce(true));
                }
                app.follow.flash = None;
                let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("olcme-ayar-{name}-{w}x{h}-{theme}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
