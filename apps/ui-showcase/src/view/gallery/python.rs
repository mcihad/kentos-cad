use iced::widget::{button, column, container, responsive, row, space, text};
use iced::{Element, Fill};

use kentos_ui::icon::{Icon, icon};
use kentos_ui::style;
use kentos_ui::theme::{motion, typography};
use kentos_ui::widget::Switch;
use kentos_ui::widget::python::{PythonEditor, PythonRepl, RunStatus, font};

use crate::app::Showcase;
use crate::message::Message;
use crate::python::{EXAMPLES, Event, OUTPUT};

fn event(event: Event) -> Message {
    Message::PythonStudio(event)
}

impl Showcase {
    pub(super) fn python_page(&self) -> Vec<Element<'_, Message>> {
        let studio = &self.python_studio;
        let mut examples = row![
            text("Örnek betik")
                .size(typography::body())
                .style(style::text::muted)
        ]
        .spacing(8)
        .align_y(iced::Center);
        for (index, (name, _)) in EXAMPLES.iter().enumerate() {
            examples = examples.push(
                button(text(*name).size(typography::body()))
                    .on_press(event(Event::Example(index)))
                    .padding([6, 11])
                    .style(style::button::segment(index == studio.example)),
            );
        }
        examples = examples.push(space::horizontal()).push(
            Switch::new(motion::reduced(), |reduced| {
                event(Event::ReducedMotion(reduced))
            })
            .label("Hareketi azalt"),
        );
        let stage = responsive(move |bounds| {
            let mut editor =
                PythonEditor::new(&studio.editor.content, |action| event(Event::Edit(action)))
                    .title("geometry.py")
                    .modified(studio.editor.modified())
                    .status(studio.repl.status())
                    .revision(studio.editor.revision())
                    .completions(&studio.editor.completion, |event| {
                        crate::message::Message::PythonStudio(Event::Complete(event))
                    })
                    .on_undo(event(Event::Undo))
                    .on_redo(event(Event::Redo));
            if studio.repl.status() != RunStatus::Running {
                editor = editor.on_run(event(Event::RunScript));
            }
            let repl = PythonRepl::new(&studio.repl, |e| event(Event::Repl(e))).output_id(OUTPUT);
            if bounds.width < typography::scaled(850.0) {
                column![
                    editor.height(typography::scaled(380.0)),
                    repl.height(typography::scaled(340.0))
                ]
                .spacing(18)
                .into()
            } else {
                row![
                    container(editor).width(iced::Length::FillPortion(3)),
                    container(repl).width(iced::Length::FillPortion(2)),
                ]
                .spacing(18)
                .height(typography::scaled(470.0))
                .into()
            }
        })
        .height(iced::Length::Shrink);
        let tools = row![
            text("JetBrains Mono")
                .font(font())
                .size(typography::caption())
                .style(style::text::muted),
            text("Ctrl+Space IntelliSense · Tab tamamla · F5 çalıştır")
                .size(typography::caption())
                .style(style::text::muted),
            space::horizontal(),
            button(
                row![
                    icon(Icon::Copy).size(13.0),
                    text("Kodu kopyala").size(typography::caption())
                ]
                .spacing(6)
                .align_y(iced::Center)
            )
            .on_press(event(Event::Copy))
            .padding([5, 9])
            .style(style::button::flat),
            button(
                row![
                    icon(Icon::Retry).size(13.0),
                    text("Oturumu sıfırla").size(typography::caption())
                ]
                .spacing(6)
                .align_y(iced::Center)
            )
            .on_press(event(Event::Restart))
            .padding([5, 9])
            .style(style::button::flat),
        ]
        .spacing(16)
        .align_y(iced::Center);
        let details = column![
            text("Editör ve REPL aynı Python oturumunu paylaşır. Betiği çalıştırdıktan sonra REPL'de length veya perimeter(points) yazabilirsiniz.").size(typography::body()).style(style::text::muted),
            text("Odak halesi, kayan satır vurgusu, çalıştırma ışığı ve sonuç geçişleri pencerenin kare olaylarıyla çizilir. Uzun işlem örneğinde Durdur düğmesini deneyin.").size(typography::body()).style(style::text::muted),
            text("import ma, from pathlib import Po, points.ap veya perimeter(po yazın. Öneriler gerçek Python modüllerini, betik tanımlarını, parametreleri ve REPL oturumunu kullanır.").size(typography::body()).style(style::text::muted),
        ].spacing(5);
        vec![
            examples.into(),
            stage.into(),
            tools.into(),
            container(details).width(Fill).into(),
        ]
    }
}
