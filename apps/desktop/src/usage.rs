//! `kentos-cad kullan <iz | iz.json> [--cikti dizin] [--boyut 1440x900,1100x650]
//! [--tema koyu,acik] [--varyant us|tr-q|hidpi]`: a usage scenario played on
//! the real app, pictured step by step. A scenario is an interaction trace
//! (fixtures/interaction/v1, its README): the steps a user takes, with
//! `{"shot": "ad"}` steps where a picture is wanted. The trace's
//! expectations are checked on the way and said, so using the app and
//! testing it are one run. The web plays the same file
//! (`apps/web/scripts/e2e/use.mjs`); `scripts/usage/compare.py` puts the two
//! platforms' pictures side by side.
//!
//! Each size and theme plays the scenario from its start in a new app. The
//! pictures are `masaustu-<iz>-<nn>-<ad>-<G>x<Y>[-acik].png`, `nn` the
//! picture's place in the scenario; a scenario without shots gets one at its
//! end, `son`. Nothing is written but the pictures.

use std::path::{Path, PathBuf};

use iced::Size;
use iced::advanced::widget::operation;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, COMMAND_INPUT, Message};
use crate::document::Document;
use crate::traces::{self, Player, Trace, VARIANTS, Variant};

const USAGE: &str = "kullanım: kentos-cad kullan <iz | iz.json> [--cikti dizin] \
                     [--boyut 1440x900,1100x650] [--tema koyu,acik] [--varyant us|tr-q|hidpi]";

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let name = args.next().ok_or(USAGE)?;
    let trace = if name.ends_with(".json") {
        Trace::read(Path::new(&name))?
    } else {
        Trace::by_id(&name)?
    };
    let mut out = PathBuf::from(".run/shots/kullanim");
    let mut sizes = vec![Size::new(1440.0, 900.0), Size::new(1100.0, 650.0)];
    let mut themes = vec!["dark", "light"];
    let mut variant = VARIANTS[0];
    while let Some(arg) = args.next() {
        let mut value = |what: &str| args.next().ok_or(format!("{arg} {what} ister ({USAGE})"));
        match arg.as_str() {
            "--cikti" => out = PathBuf::from(value("bir dizin")?),
            "--boyut" => sizes = parse_sizes(&value("boyutlar")?)?,
            "--tema" => {
                themes = value("temalar")?
                    .split(',')
                    .map(|t| match t.trim() {
                        "koyu" => Ok("dark"),
                        "acik" => Ok("light"),
                        other => Err(format!("{other}: tema koyu ya da acik olmalı")),
                    })
                    .collect::<Result<_, _>>()?;
            }
            "--varyant" => {
                let id = value("us, tr-q ya da hidpi")?;
                variant = Variant::by_id(&id)
                    .ok_or(format!("{id}: böyle bir varyant yok (us, tr-q, hidpi)"))?;
            }
            other => return Err(format!("{other}: bilinmeyen seçenek ({USAGE})")),
        }
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut problems = Vec::new();
    for &size in &sizes {
        for &theme in &themes {
            problems.extend(play(&trace, variant, size, theme, &out)?);
        }
    }
    if problems.is_empty() {
        println!("{}: beklentilerin hepsi tuttu.", trace.id);
    } else {
        for problem in &problems {
            eprintln!("Uyarı: {problem}");
        }
        eprintln!("{}: {} beklenti tutmadı.", trace.id, problems.len());
    }
    Ok(())
}

/// Plays the scenario once at `size` in `theme`, writing its pictures; the
/// expectations that did not hold, each named with the size and theme.
fn play(
    trace: &Trace,
    variant: Variant,
    size: Size,
    theme: &str,
    out: &Path,
) -> Result<Vec<String>, String> {
    let (mut app, _) = App::boot(None);
    let _ = app
        .settings
        .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
    app.apply_settings();
    let mut snapshot = Snapshot::new(size)
        .map_err(|e| e.to_string())?
        .scale(variant.dpr);
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    // The drawing first, so the drawing area is laid out and knows its place.
    let text = std::fs::read_to_string(traces::folder().join(&trace.document))
        .map_err(|e| format!("{}: {e}", trace.document))?;
    let doc = Document::new(
        kentos_contracts::DocumentSnapshotV1::from_json(&text).map_err(|e| e.to_string())?,
        None,
    )?;
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    snapshot.settle(&mut app, App::view, &mut update);
    let area = app.viewport.bounds;
    let suffix = if theme == "light" { "-acik" } else { "" };
    let label = format!("{}x{}{suffix}", size.width, size.height);
    let count = std::cell::Cell::new(0usize);
    let mut failed = None;
    let mut take = |app: &mut App, name: &str, line: bool| {
        count.set(count.get() + 1);
        let count = count.get();
        // The command history open for the picture, so it shows what the tools said, where the
        // window has room for it (the status bar says the last message in a small one); closed
        // again while the steps play (its scrolling is no operation the trace player follows).
        app.command_expanded = size.height >= 800.0;
        snapshot.settle(app, App::view, &mut update);
        // At its newest line, as the app scrolls it when it shows (`message_log`).
        snapshot.operate(
            app.view(),
            Box::new(operation::scrollable::snap_to(
                crate::message_log::LIST.into(),
                iced::widget::scrollable::RelativeOffset::END.into(),
            )),
        );
        snapshot.settle(app, App::view, &mut update);
        if line {
            snapshot.operate(
                app.view(),
                Box::new(operation::focusable::focus(COMMAND_INPUT.into())),
            );
            snapshot.settle(app, App::view, &mut update);
        }
        let file = out.join(format!(
            "masaustu-{}-{count:02}-{name}-{label}.png",
            trace.id
        ));
        match snapshot.render(app.view(), &app.theme()).save(&file) {
            Ok(()) => println!("{}", file.display()),
            Err(e) => failed = Some(format!("{}: {e}", file.display())),
        }
        app.command_expanded = false;
    };
    let file = traces::scratch_file(trace, variant);
    let problems = {
        let mut player = Player::new(&mut app, trace, variant, area, file)?;
        let problems = player.play_shots(&mut take);
        if count.get() == 0 {
            let line = player.line_has_keyboard();
            take(player.app, "son", line);
        }
        problems
    };
    if let Some(error) = failed {
        return Err(error);
    }
    Ok(problems
        .into_iter()
        .map(|p| format!("{label}: {p}"))
        .collect())
}

/// `1440x900,1100x650`.
fn parse_sizes(text: &str) -> Result<Vec<Size>, String> {
    text.split(',')
        .map(|one| {
            one.trim()
                .split_once('x')
                .and_then(|(w, h)| Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?)))
                .filter(|(w, h)| *w >= 400.0 && *h >= 300.0)
                .map(|(w, h)| Size::new(w, h))
                .ok_or(format!("{one}: boyut GxY biçiminde olmalı (ör. 1440x900)"))
        })
        .collect()
}
