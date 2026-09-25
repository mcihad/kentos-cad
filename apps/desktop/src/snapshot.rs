//! `kentos-cad snapshot çıktı.png [çizim.kcad] [--sekme <id>] [--tema acik]
//! [--komut <id>]… [--tumu] [--merkez Y,X] [--yakinlastir <kat>]`: the window
//! drawn without opening one (the KentOS UI snapshot renderer), for visual
//! checks and documentation. Nothing is written but the image.
//!
//! `--komut` runs a web command id after the drawing is opened
//! (`help.shortcuts`, `edit.undo`); what a command would start in the
//! background (a file dialog, a save) is not run.
//!
//! The drawing area is KentOS's wgpu pipeline, so it is drawn only by the
//! GPU renderer (the default; `KENTOS_SNAPSHOT_BACKEND=tiny-skia` leaves it
//! empty). The view options act after the drawing opened at its start view:
//! `--tumu` shows everything, `--merkez` puts a world point (Y east, X north)
//! in the middle, `--yakinlastir` zooms about the middle by a factor.

use std::path::Path;

use iced::{Point, Size};
use kentos_render_wgpu::Vec2;
use kentos_ui::snapshot::Snapshot;
use kentos_ui::theme::Mode;

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::document::Document;
use crate::viewport::Event;

const USAGE: &str = "kullanım: kentos-cad snapshot çıktı.png [çizim.kcad] [--sekme <id>] [--tema acik] \
                     [--komut <id>]… [--tumu] [--merkez Y,X] [--yakinlastir <kat>]";

/// A view option, applied in the order given once the drawing is on screen.
enum View {
    Extents,
    Center(Vec2),
    Zoom(f64),
}

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let out = args.next().ok_or(USAGE)?;
    let (mut app, _) = App::boot(None);
    let mut commands = Vec::new();
    let mut views = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sekme" => {
                let id = args
                    .next()
                    .ok_or("--sekme bir sekme kimliği ister (ör. home, draw)")?;
                app.tab = catalog()
                    .tabs()
                    .find(|tab| tab.id == id)
                    .map(|tab| tab.id)
                    .ok_or(format!("{id}: böyle bir şerit sekmesi yok"))?;
            }
            "--tema" => {
                app.mode = match args.next().as_deref() {
                    Some("acik") => Mode::Light,
                    _ => Mode::Dark,
                };
            }
            "--komut" => {
                let id = args
                    .next()
                    .ok_or("--komut bir komut kimliği ister (ör. help.shortcuts, edit.undo)")?;
                let command = catalog()
                    .get(&id)
                    .ok_or(format!("{id}: böyle bir komut yok"))?;
                commands.push(command.id);
            }
            "--tumu" => views.push(View::Extents),
            "--merkez" => {
                let text = args
                    .next()
                    .ok_or("--merkez bir nokta ister: Y,X (ör. 486524.34,4420199.52)")?;
                views.push(View::Center(point(&text)?));
            }
            "--yakinlastir" => {
                let text = args.next().ok_or("--yakinlastir bir kat ister (ör. 200)")?;
                let factor: f64 = text
                    .parse()
                    .ok()
                    .filter(|f: &f64| f.is_finite() && *f > 0.0)
                    .ok_or(format!(
                        "{text}: yakınlaştırma katı pozitif bir sayı olmalı"
                    ))?;
                views.push(View::Zoom(factor));
            }
            path => {
                let doc = Document::read(Path::new(path))?;
                let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            }
        }
    }
    // After the drawing is open, whatever order the arguments came in.
    for id in commands {
        let _ = app.update(Message::Run(id));
    }

    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).map_err(|e| e.to_string())?;
    if snapshot.renderer_name() != "wgpu" && app.document.is_some() {
        eprintln!(
            "Uyarı: {} çizicisi KentOS'un wgpu çizim alanını çizmez; alan boş görünecek \
             (KENTOS_SNAPSHOT_BACKEND=wgpu).",
            snapshot.renderer_name()
        );
    }
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    for view in views {
        let event = match view {
            View::Extents => Event::Extents,
            View::Center(p) => Event::CenterOn(p),
            View::Zoom(factor) => Event::Zoomed {
                factor,
                at: Point::new(
                    (app.viewport.camera.width / 2.0) as f32,
                    (app.viewport.camera.height / 2.0) as f32,
                ),
            },
        };
        let _ = app.update(Message::Viewport(event));
        snapshot.settle(&mut app, App::view, &mut update);
    }
    snapshot
        .render(app.view(), &app.theme())
        .save(&out)
        .map_err(|e| format!("{out}: {e}"))
}

/// `Y,X` with a decimal point (CLAUDE.md §5): east, then north.
fn point(text: &str) -> Result<Vec2, String> {
    let parsed = text
        .split_once(',')
        .and_then(|(y, x)| Some((y.trim().parse::<f64>().ok()?, x.trim().parse::<f64>().ok()?)))
        .filter(|(y, x)| y.is_finite() && x.is_finite());
    match parsed {
        Some((east, north)) => Ok(Vec2::new(east, north)),
        None => Err(format!(
            "{text}: nokta Y,X biçiminde olmalı, ondalık ayırıcı nokta (ör. 486524.34,4420199.52)"
        )),
    }
}
