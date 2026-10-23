//! Physical source calibration: editing a reference invalidates the preview until explicitly applied.
use iced::{
    Element, Task,
    widget::{column, row, text_input, button},
};
use kentos_ui::{label, style};
use super::{State, Event, event};
use crate::app::{App, Message};
use crate::exchange::Window;

fn reference(s: &State) -> Option<kentos_commands::CadImportScale> {
    let value = |t: &str| t.trim().replace(',', ".").parse::<f64>().ok();
    let r = kentos_commands::CadImportScale {
        source_length: value(&s.reference_source)?,
        real_length: value(&s.reference_real)?,
    };
    r.factor().map(|_| r)
}
pub(super) fn view<'a>(s: &'a State) -> Element<'a, Message> {
    let field = |title: &'static str, text: &'a str, on: fn(String) -> Event| {
        column![
            label::caption(title),
            text_input("Uzunluk", text)
                .on_input(move |v| event(on(v)))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input)
        ]
        .spacing(4)
    };
    let mut apply = button(label::body("Ölçüyü uygula"))
        .padding([6, 12])
        .style(style::button::secondary);
    if reference(s).is_some() {
        apply = apply.on_press(event(Event::ReferenceApply));
    }
    column![label::body("Bilinen bir uzunlukla ölçeği doğrula"),row![field("Dosyadaki ölçü",&s.reference_source,Event::ReferenceSource),field("Gerçek uzunluk (metre)",&s.reference_real,Event::ReferenceReal),apply].spacing(12).align_y(iced::Alignment::End),label::caption("Örneğin dosyada 1250 olan bir duvar gerçekte 12,5 m ise bu iki değeri girin. Gerçek ölçü bilinmiyorsa tahmin yapılmaz. Dosya birimi seçerek bu doğrulamayı kaldırabilirsiniz.")].spacing(6).into()
}
impl App {
    pub(super) fn drawing_reference_event(&mut self, e: Event) -> Task<Message> {
        let Some(Window::DrawingImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::ReferenceApply => {
                let Some(r) = reference(s) else {
                    return Task::none();
                };
                s.reference_pending = false;
                s.scale_reference = Some(r);
                s.file_unit = None;
                return self.read_drawing();
            }
            Event::ReferenceSource(text) => s.reference_source = text,
            Event::ReferenceReal(text) => s.reference_real = text,
            _ => return Task::none(),
        }
        s.stop_reading();
        s.read = super::READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        s.reading = None;
        s.result = None;
        s.scale_reference = None;
        s.reference_pending = true;
        s.failed = None;
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        files_testing::{app_with_drawing, drive},
        app::Picker,
    };
    use crate::exchange::{
        tests::{fixture, run, send, count},
        Event as X,
    };
    fn send_scale(app: &mut App, e: Event) {
        send(app, X::DrawingImport(e));
    }
    #[test]
    fn import_scale_reference_is_explicit_reopenable_and_one_undo_without_changing_project_units() {
        let mut app = app_with_drawing();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let n = count(&app);
        app.picker = Picker::File(fixture("dxf-units/mm.dxf"));
        run(&mut app, "file.import.dxf");
        let Some(Window::DrawingImport(s)) = &app.exchange else {
            panic!("dialog")
        };
        assert_eq!(
            s.result.as_ref().unwrap().unit.as_ref().unwrap().factor,
            0.001,
            "GIS also honours declared source mm"
        );
        send_scale(&mut app, Event::ReferenceSource("4000".into()));
        // Switching an unrelated read option must not apply the unfinished reference.
        send_scale(&mut app, Event::Closed(Some(kentos_commands::ClosedPolylines::Line)));
        send_scale(&mut app, Event::Run);
        assert_eq!(
            count(&app),
            n,
            "unapplied reference cannot commit the old preview"
        );
        send_scale(&mut app, Event::ReferenceReal("12,5".into()));
        send_scale(&mut app, Event::ReferenceApply);
        let Some(Window::DrawingImport(s)) = &app.exchange else {
            panic!("dialog")
        };
        assert_eq!(
            s.result.as_ref().unwrap().unit.as_ref().unwrap().factor,
            12.5 / 4000.
        );
        send_scale(&mut app, Event::Run);
        let doc = &app.document.as_ref().unwrap().model;
        assert!(
            doc.entities().any(
                |e| matches!(e,kentos_commands::Entity::Line(l)if(l.b.x-l.a.x-12.5).abs()<1e-10)
            )
        );
        assert_eq!(doc.settings(), &before.settings);
        assert_eq!(
            doc.journal().last().unwrap().input.as_ref().unwrap()["scaleReference"],
            serde_json::json!({"sourceLength":4000.,"realLength":12.5})
        );
        let task = app.run("edit.undo");
        drive(&mut app, task);
        assert_eq!(count(&app), n);
    }
    #[test]
    #[ignore = "GPU import unit controls with local user DWG, run explicitly"]
    fn import_scale_user_dwg_screens() {
        crate::drawing_fonts::load();
        kentos_ui::theme::motion::set_reduced(true);
        let path = std::path::PathBuf::from("/Users/cihad/Downloads/2-1-ev-kat-plani.dwg");
        assert!(path.is_file());
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        for (theme, text) in [("light", 13), ("dark", 13), ("dark", 18)] {
            let mut app = app_with_drawing();
            app.settings.choose(&[
                ("appearance.theme", serde_json::json!(theme)),
                ("appearance.textSize", serde_json::json!(text)),
            ]);
            app.apply_settings();
            app.picker = Picker::File(path.clone());
            run(&mut app, "file.import.dwg");
            let mut snap =
                kentos_ui::snapshot::Snapshot::new(iced::Size::new(1280., 900.)).unwrap();
            let mut update = |app: &mut App, m: Message| {
                let task = app.update(m);
                drive(app, task);
            };
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join(format!("scale-source-{theme}-{text}.png")))
                .unwrap();
            send_scale(&mut app, Event::ReferenceSource("1250".into()));
            send_scale(&mut app, Event::ReferenceReal("12,5".into()));
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join(format!("scale-reference-input-{theme}-{text}.png")))
                .unwrap();
            snap.input(
                &mut app,
                App::view,
                &mut update,
                kentos_ui::snapshot::Input::Click(iced::Point::new(
                    if text == 18 { 1190. } else { 1054. },
                    426.,
                )),
            );
            let Some(Window::DrawingImport(s)) = &app.exchange else {
                panic!("dialog")
            };
            assert!(
                s.scale_reference.is_some(),
                "real Apply click must activate calibration"
            );
            assert_eq!(
                s.result.as_ref().unwrap().unit.as_ref().unwrap().factor,
                0.01
            );
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join(format!("scale-reference-{theme}-{text}.png")))
                .unwrap();
            send_scale(&mut app, Event::Run);
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join(format!("scale-house-{theme}-{text}.png")))
                .unwrap();
        }
    }
}
