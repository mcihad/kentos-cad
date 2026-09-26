//! `kentos-cad snapshot çıktı.png [çizim.kcad] [--sekme <id>] [--tema acik]
//! [--ayar anahtar=değer]… [--komut <id>]… [--tumu] [--merkez Y,X]
//! [--yakinlastir <kat>] [--iz <iz> [--adim <n>] [--yarida] [--varyant us|tr-q|hidpi]]`:
//! the window drawn without opening one (the KentOS UI snapshot renderer),
//! for visual checks and documentation. Nothing is written but the image;
//! `--ayar` chooses a typed setting in memory (`graphics.msaa=8`), never in
//! the user's settings file.
//!
//! `--iz` plays an interaction trace (fixtures/interaction/v1, traces.rs) on
//! its own drawing, up to step `--adim` (all by default), so the image shows
//! the app mid-drawing: the draft, the value field, the prompt. `--yarida`
//! plays that last step halfway: a drag stops with the button still down,
//! so the selection box shows (docs/adr/0029).
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

use iced::advanced::widget::operation;
use iced::{Point, Size};
use kentos_render_wgpu::Vec2;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, COMMAND_INPUT, Message};
use crate::catalog::catalog;
use crate::document::Document;
use crate::traces::{self, Player, Trace, VARIANTS, Variant};
use crate::viewport::Event;

const USAGE: &str = "kullanım: kentos-cad snapshot çıktı.png [çizim.kcad] [--sekme <id>] [--tema acik] \
                     [--ayar anahtar=değer]… [--komut <id>]… [--tumu] [--merkez Y,X] [--yakinlastir <kat>] \
                     [--iz <iz> [--adim <n>] [--yarida] [--varyant us|tr-q|hidpi]]";

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
    let mut trace: Option<Trace> = None;
    let mut steps: Option<usize> = None;
    let mut halfway = false;
    let mut variant = VARIANTS[0];
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
                // Through the settings (in memory here), as the theme commands do.
                let theme = match args.next().as_deref() {
                    Some("acik") => "light",
                    _ => "dark",
                };
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
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
            "--ayar" => {
                let text = args
                    .next()
                    .ok_or("--ayar bir anahtar=değer ister (ör. graphics.msaa=8)")?;
                setting(&mut app, &text)?;
            }
            "--iz" => {
                let id = args
                    .next()
                    .ok_or("--iz bir iz kimliği ister (ör. polygon-accept)")?;
                trace = Some(Trace::by_id(&id)?);
            }
            "--adim" => {
                let text = args.next().ok_or("--adim bir adım sayısı ister (ör. 5)")?;
                steps = Some(
                    text.parse()
                        .map_err(|_| format!("{text}: adım sayısı pozitif bir tam sayı olmalı"))?,
                );
            }
            "--yarida" => halfway = true,
            "--varyant" => {
                let id = args.next().ok_or("--varyant us, tr-q ya da hidpi ister")?;
                variant = Variant::by_id(&id)
                    .ok_or(format!("{id}: böyle bir varyant yok (us, tr-q, hidpi)"))?;
            }
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

    // A trace's 2× variant is drawn on a 2× screen.
    let scale = if trace.is_some() { variant.dpr } else { 1.0 };
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0))
        .map_err(|e| e.to_string())?
        .scale(scale);
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
    if let Some(trace) = &trace {
        // The trace's drawing first, so the drawing area is laid out and knows its place.
        let text = std::fs::read_to_string(traces::folder().join(&trace.document))
            .map_err(|e| format!("{}: {e}", trace.document))?;
        let doc = Document::new(
            kentos_contracts::DocumentSnapshotV1::from_json(&text).map_err(|e| e.to_string())?,
            None,
        )?;
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        snapshot.settle(&mut app, App::view, &mut update);
        let area = app.viewport.bounds;
        let file = traces::scratch_file(trace, variant);
        let mut player = Player::new(&mut app, trace, variant, area, file)?;
        // `--yarida`: the last step halfway (a selection box with the button still down).
        let (whole, half) = match (steps, halfway) {
            (Some(n), true) if n > 0 => (Some(n - 1), Some(n - 1)),
            _ => (steps, None),
        };
        for problem in player.play(whole) {
            eprintln!("Uyarı: {problem}");
        }
        if let Some(index) = half
            && let Err(problem) = player.halfway(index)
        {
            eprintln!("Uyarı: {problem}");
        }
        // The command line keeps the keyboard the trace left it with, so its suggestion list shows.
        let line = player.line_has_keyboard();
        drop(player);
        if line {
            snapshot.operate(
                app.view(),
                Box::new(operation::focusable::focus(COMMAND_INPUT.into())),
            );
        }
        snapshot.settle(&mut app, App::view, &mut update);
    }
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
    // The drawing area says what the device takes once it has drawn a frame (AA-01): one
    // frame first, so the settings window shows the device's sample counts and the value in use.
    let _ = snapshot.render(app.view(), &app.theme());
    app.sync_device();
    snapshot
        .render(app.view(), &app.theme())
        .save(&out)
        .map_err(|e| format!("{out}: {e}"))
}

/// `--ayar key=value`: a typed setting chosen before the image is drawn, as
/// the settings window would (JSON value, else text: `graphics.msaa=8`,
/// `graphics.hiDpi=false`, `appearance.theme=light`).
fn setting(app: &mut App, text: &str) -> Result<(), String> {
    let (key, value) = text.split_once('=').ok_or(format!(
        "{text}: ayar anahtar=değer biçiminde olmalı (ör. graphics.msaa=8)"
    ))?;
    let value = serde_json::from_str(value).unwrap_or_else(|_| serde_json::Value::from(value));
    let key = crate::settings::schema()
        .get(key.trim())
        .map(|d| d.key.as_str())
        .ok_or(format!("{key}: böyle bir ayar yok"))?;
    if let Some((_, code)) = app.settings.choose(&[(key, value)]).first() {
        return Err(format!("{text}: {}", code.message()));
    }
    app.apply_settings();
    Ok(())
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
