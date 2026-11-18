//! The model viewer alone in one window, with no application around it (docs/adr/8061 §3.3 "Tek pencere örneği",
//! §6.1; docs/adr/8073 §4): what a separate viewer program would be. It reads the file on a thread of its own, hands
//! the viewer its model and the chosen object's details, and nothing else. A tie to the application that leaks into
//! the component breaks this example's build.
//!
//! ```text
//! cargo run -p kentos-model-view-ui --example viewer -- fixtures/arch/v1/ifc/kesit-binasi.ifc
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Task};
use kentos_ifc::read::{self as ifc, Limits, Model};
use kentos_model_view::Input;
use kentos_model_view_ui::{Host, Out, Viewer, look};
use kentos_ui::theme::{self, Accent, Mode, Tokens, typography};

struct App {
    viewer: Viewer,
    file: Option<(Arc<Vec<u8>>, Arc<Model>)>,
}

#[derive(Debug, Clone)]
enum Msg {
    Viewer(kentos_model_view_ui::Message),
    Read(Result<Arc<Read>, String>),
}

/// A file read: its bytes, the model, the viewer's input and what the status bar says of it.
#[derive(Debug)]
struct Read {
    bytes: Arc<Vec<u8>>,
    model: Arc<Model>,
    input: Input,
    schema: String,
}

fn main() -> iced::Result {
    typography::load();
    let path = std::env::args().nth(1).map_or_else(|| PathBuf::from("fixtures/arch/v1/ifc/kesit-binasi.ifc"), PathBuf::from);
    let title = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
    iced::application(
        move || (App { viewer: Viewer::new(title.clone()), file: None }, start(path.clone())),
        App::update,
        App::view,
    )
    .title(|app: &App| format!("{} — IFC modeli", app.viewer.title))
    .theme(|_: &App| theme::theme(Mode::Light, Accent::Blue))
    .default_font(typography::ui())
    .window_size(iced::Size::new(1400.0, 860.0))
    .run()
}

/// Reads the file on its own thread.
fn start(path: PathBuf) -> Task<Msg> {
    let (tx, rx) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read(&path).map(Arc::new));
    });
    Task::perform(async move { rx.await.unwrap_or_else(|_| Err("Okuma durdu.".to_owned())) }, Msg::Read)
}

fn read(path: &std::path::Path) -> Result<Read, String> {
    let never = || false;
    let bytes = std::fs::read(path).map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
    let model = ifc::read(&bytes, &Limits::default(), &never).map_err(|e| e.to_string())?;
    let mut bodies = model.bodies(&bytes, &never).map_err(|e| e.to_string())?;
    let openings = model.opening_bodies(&bytes, &never).map_err(|e| e.to_string())?;
    bodies.bodies.extend(openings.bodies);
    bodies.not_shown.extend(openings.not_shown);
    let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let input = ifc::view::input(&name, &model, &bodies);
    let schema = model.summary.schema.clone();
    Ok(Read { bytes: Arc::new(bytes), model: Arc::new(model), input, schema })
}

impl App {
    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Read(Ok(r)) => {
                let r = Arc::try_unwrap(r).unwrap_or_else(|r| Read {
                    bytes: r.bytes.clone(),
                    model: r.model.clone(),
                    input: r.input.clone(),
                    schema: r.schema.clone(),
                });
                self.viewer.load(r.input, vec![(r.schema, "Dosyanın şeması, başlığında yazdığı gibi.".to_owned())]);
                self.file = Some((r.bytes, r.model));
            }
            Msg::Read(Err(why)) => self.viewer.failed(why),
            Msg::Viewer(m) => match self.viewer.update(m) {
                Some(Out::Chosen(Some((object, _)))) => {
                    if let Some((bytes, model)) = &self.file
                        && let Ok(d) = model.details(bytes, object)
                    {
                        self.viewer.set_details(object, d);
                    }
                }
                Some(Out::Close) => return iced::exit(),
                Some(Out::Chosen(None) | Out::Cancel) | None => {}
            },
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Msg> {
        let t = Tokens::of(&theme::theme(Mode::Light, Accent::Blue));
        self.viewer.layout(Host {
            to: Msg::Viewer,
            end: Vec::new(),
            sky: look::sky(t.window, Some(t.accent)),
            quality: kentos_model_view_ui::scene::Quality::High,
            coordinate: Box::new(|v| format!("{v:.3}")),
            measure: Box::new(|v, unit| match look::digits(unit, 3, 2) {
                Some(d) => format!("{v:.d$}"),
                None => look::plain(v),
            }),
            east: "Y",
            north: "X",
        })
    }
}
