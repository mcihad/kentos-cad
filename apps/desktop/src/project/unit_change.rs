//! Proje ayarları' question when a local project's unit changes with a
//! drawing in it (docs/adr/0256 §2): keep the sizes (only the unit lengths
//! are read in changes) or keep the numbers (the drawing is scaled by the
//! units' ratio). The answer is written by `cad.units.change`; the question
//! shows the command's plan: how many objects and definitions, the factor,
//! what stays and, when the numbers cannot be kept (a locked layer), why.

use iced::Element;
use iced::widget::column;
use kentos_commands::{CommandResult, DrawingUnit, UnitGeometry, UnitsChange};
use kentos_interaction::ground::trimmed;
use kentos_native_application::units_change;
use kentos_ui::label;
use kentos_ui::widget::{Banner, Dialog, overlay};

use crate::exchange::words;

/// The question as the plan of keeping the numbers answers it.
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub from: DrawingUnit,
    pub to: DrawingUnit,
    pub objects: u32,
    pub blocks: u32,
    /// What the drawing's lengths are multiplied by when the numbers stay.
    pub factor: f64,
    /// What keeping the numbers leaves as it is (the plan's warnings).
    pub notes: Vec<String>,
    /// Why the numbers cannot be kept (a locked layer); none when they can.
    pub refused: Option<String>,
}

impl Question {
    /// The question for a change from `from` to `to` in `doc`.
    pub fn of(doc: &kentos_domain::Document, from: DrawingUnit, to: DrawingUnit) -> Self {
        let input = UnitsChange {
            from,
            to,
            geometry: UnitGeometry::Scale,
            expected_revision: None,
        };
        let mut q = Self {
            from,
            to,
            objects: 0,
            blocks: 0,
            factor: 1.0,
            notes: Vec::new(),
            refused: None,
        };
        match units_change::plan_in(doc, &input) {
            CommandResult::Completed { output, warnings } => {
                q.objects = output.objects;
                q.blocks = output.blocks;
                q.factor = output.factor;
                q.notes = warnings.into_iter().map(|w| w.message).collect();
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => q.refused = Some(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
        q
    }

    /// “4000 mm”: a length of four metres in `u`.
    fn sample(u: DrawingUnit, metres: f64) -> String {
        format!("{} {}", trimmed(metres * u.per_metre()), u.mark())
    }

    /// The ratio as a sign and a whole number: “×1000”, “÷10”.
    fn ratio(&self) -> String {
        if self.factor >= 1.0 {
            format!("×{}", trimmed(self.factor))
        } else {
            format!("÷{}", trimmed(1.0 / self.factor))
        }
    }

    /// What the drawing holds, in words.
    fn held(&self) -> String {
        match (self.objects, self.blocks) {
            (n, 0) => format!("{n} nesne"),
            (0, b) => format!("{b} blok tanımı"),
            (n, b) => format!("{n} nesne ve {b} blok tanımı"),
        }
    }
}

/// The question over Proje ayarları, which waits under it: Boyları korunsun
/// (the default), Sayıları korunsun, Vazgeç.
pub fn view<'a, M: Clone + 'a>(
    q: &'a Question,
    answer: impl Fn(Option<UnitGeometry>) -> M,
) -> Element<'a, M> {
    let (from, to) = (q.from.mark(), q.to.mark());
    let old = Question::sample(q.from, 4.0);
    let keep = column![
        label::strong("Boyları korunsun"),
        label::caption(format!(
            "Çizim değişmez; uzunluklar {to} okunur ve yazılır: {old}'lik bir duvar {} okunur.",
            Question::sample(q.to, 4.0)
        )),
    ]
    .spacing(2);
    let mut scale = column![
        label::strong("Sayıları korunsun"),
        label::caption(format!(
            "Çizim {} ölçeklenir: {old}'lik bir duvar {} {to} olur. Yanlış birimle gelmiş çizimi düzeltmek içindir.",
            q.ratio(),
            trimmed(4.0 * q.from.per_metre()),
        )),
    ]
    .spacing(2);
    for note in &q.notes {
        scale = scale.push(label::caption(note.clone()));
    }
    let mut body = column![
        label::body(format!(
            "Çizim birimi {from}'den {to}'ye değişiyor. Çizimdeki {} ne olsun?",
            q.held()
        )),
        keep,
        scale,
    ]
    .spacing(12);
    if let Some(why) = &q.refused {
        body = body.push(Banner::warning(why.clone()));
    }
    body = body.push(label::caption(
        "İkisi de tek adımdır; Geri al (Ctrl+Z) birimi ve çizimi birlikte geri getirir.",
    ));
    overlay::blocking(
        Dialog::new("Çizim birimi değişiyor")
            .push(body)
            .action(words::secondary("Vazgeç", Some(answer(None))))
            .action(words::secondary(
                "Sayıları korunsun",
                q.refused
                    .is_none()
                    .then(|| answer(Some(UnitGeometry::Scale))),
            ))
            .action(words::primary(
                "Boyları korunsun",
                Some(answer(Some(UnitGeometry::Keep))),
            ))
            .width(560.0),
    )
}

#[cfg(test)]
mod tests {
    use kentos_commands::{DrawingUnit, Entity, EntityBase, PathEntity, ProjectSettings, Vec2};

    use crate::app::{App, Message};
    use crate::project::{SettingsEvent, settings_message};

    fn send(app: &mut App, e: SettingsEvent) {
        let _ = app.update(settings_message(e));
    }

    /// A local CAD project in millimetres with a 4 × 3 m room, or empty.
    fn room(app: &mut App, with_room: bool) {
        crate::files_testing::make_cad(app);
        let doc = app.document.as_mut().expect("a drawing");
        // The sample's objects go: the question counts this drawing's.
        let slots: Vec<_> = doc
            .model
            .entities()
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        doc.model.remove(&slots);
        let settings = ProjectSettings {
            srid: 0,
            drawing_unit: Some(DrawingUnit::Mm),
            plot_scale: 50.0,
            ..doc.model.settings().clone()
        };
        doc.model.set_settings(settings);
        if with_room {
            let layer = doc.model.layers().active().to_owned();
            let p = |x: f64, y: f64| Vec2 { x, y };
            doc.model
                .add(Entity::Polygon(PathEntity {
                    space_separator: None,
                    base: EntityBase {
                        look: Default::default(),
                        id: 0,
                        layer_id: layer,
                        color: None,
                        attrs: Default::default(),
                        label: None,
                        symbol: None,
                        line_weight: None,
                        label_pins: Vec::new(),
                        attachments: Vec::new(),
                    },
                    pts: vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 3.0), p(0.0, 3.0)],
                    ms: None,
                    bulges: None,
                    holes: None,
                    zs: None,
                    parts: None,
                    closed: false,
                    widths: None,
                }))
                .expect("a slot");
        }
    }

    fn corner(app: &App) -> f64 {
        let doc = app.document.as_ref().expect("a drawing");
        match doc.model.entities().last() {
            Some(Entity::Polygon(p)) => p.pts[2].x,
            other => panic!("a polygon, not {other:?}"),
        }
    }

    fn question(app: &App) -> Option<super::Question> {
        match &app.project {
            Some(crate::project::Window::Settings(s)) => s.unit_question.clone(),
            _ => None,
        }
    }

    fn open_and_change(app: &mut App, to: DrawingUnit) {
        let _ = app.update(Message::Run("file.settings"));
        send(app, SettingsEvent::DrawingUnit(to));
        send(app, SettingsEvent::Save);
    }

    /// Kaydet asks; Boyları korunsun changes only the unit; undo gives it back.
    #[test]
    fn the_sizes_kept_change_only_the_unit() {
        let mut app = crate::files_testing::app_with_drawing();
        room(&mut app, true);
        open_and_change(&mut app, DrawingUnit::M);
        let q = question(&app).expect("Kaydet asks");
        assert_eq!(
            (q.from, q.to, q.objects, q.factor),
            (DrawingUnit::Mm, DrawingUnit::M, 1, 1000.0)
        );
        assert_eq!(
            app.document
                .as_ref()
                .map(|d| d.model.settings().drawing_unit),
            Some(Some(DrawingUnit::Mm)),
            "nothing is written while it asks"
        );
        send(
            &mut app,
            SettingsEvent::UnitAnswer(Some(kentos_commands::UnitGeometry::Keep)),
        );
        assert!(app.project.is_none(), "the window closes once saved");
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.model.settings().drawing_unit, None, "metres");
        assert_eq!(corner(&app), 4.0);
        let _ = app.update(Message::Run("edit.undo"));
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.model.settings().drawing_unit, Some(DrawingUnit::Mm));
    }

    /// Sayıları korunsun scales the drawing: 4 m becomes 4000 m, read 4000 m.
    #[test]
    fn the_numbers_kept_scale_the_drawing() {
        let mut app = crate::files_testing::app_with_drawing();
        room(&mut app, true);
        open_and_change(&mut app, DrawingUnit::M);
        send(
            &mut app,
            SettingsEvent::UnitAnswer(Some(kentos_commands::UnitGeometry::Scale)),
        );
        assert_eq!(corner(&app), 4000.0);
        assert!(crate::files_testing::last_said(&app).contains("Proje ayarları kaydedildi"));
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(corner(&app), 4.0);
    }

    /// Vazgeç goes back to the window; an empty drawing is not asked about.
    #[test]
    fn vazgec_goes_back_and_an_empty_drawing_is_not_asked() {
        let mut app = crate::files_testing::app_with_drawing();
        room(&mut app, true);
        open_and_change(&mut app, DrawingUnit::Cm);
        send(&mut app, SettingsEvent::UnitAnswer(None));
        assert!(question(&app).is_none());
        assert!(app.project.is_some(), "Proje ayarları waits");
        let mut app = crate::files_testing::app_with_drawing();
        room(&mut app, false);
        open_and_change(&mut app, DrawingUnit::Cm);
        assert!(app.project.is_none(), "saved without a question");
        assert_eq!(
            app.document
                .as_ref()
                .map(|d| d.model.settings().drawing_unit),
            Some(Some(DrawingUnit::Cm))
        );
    }

    /// The question's pictures: mm to m, both themes, 1440×900 and 1100×650.
    #[test]
    #[ignore = "writes pictures: cargo test -p kentos-desktop project::unit_change::tests::screens -- --ignored --nocapture"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (theme, w, h) in [
            ("dark", 1440.0, 900.0),
            ("light", 1440.0, 900.0),
            ("dark", 1100.0, 650.0),
            ("light", 1100.0, 650.0),
        ] {
            let mut app = crate::files_testing::app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            room(&mut app, true);
            open_and_change(&mut app, DrawingUnit::M);
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("birim-sorusu-{w}x{h}-{theme}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
