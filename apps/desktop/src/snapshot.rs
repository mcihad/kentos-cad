//! `kentos-cad snapshot çıktı.png [çizim.kcad] [--sekme <id>] [--tema acik]`:
//! the window drawn without opening one (the KentOS UI snapshot renderer),
//! for visual checks and documentation. Nothing is written but the image.

use std::path::Path;

use iced::Size;
use kentos_ui::snapshot::Snapshot;
use kentos_ui::theme::Mode;

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::document::Document;

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let out = args.next().ok_or(
        "kullanım: kentos-cad snapshot çıktı.png [çizim.kcad] [--sekme <id>] [--tema acik]",
    )?;
    let (mut app, _) = App::boot(None);
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
            path => {
                let doc = Document::read(Path::new(path))?;
                let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
            }
        }
    }

    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).map_err(|e| e.to_string())?;
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    snapshot
        .render(app.view(), &app.theme())
        .save(&out)
        .map_err(|e| format!("{out}: {e}"))
}
