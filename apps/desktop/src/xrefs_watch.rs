//! External references followed while the drawing is open (docs/adr/0349 §3; plan/cad.md `CAD-REF-09`): every five
//! seconds each loaded reference's file is looked at (its size and change time; the SHA-256 only when they moved), and
//! a source that changed is told by a card over the drawing with the sheets and named views it affects — Yeniden yükle,
//! Dış başvurular, Kapat. Nothing is loaded or saved by itself. When the drawing opens, `cad.assets.check` says the
//! linked files that changed or are missing. The linked pictures the drawing shows are looked at too (docs/adr/0350
//! §3): a file that changed or came back is read again and drawn, quietly (its pixels are not the drawing's data).

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt as _, Stream};
use iced::widget::{Column, Row, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{AssetKind, AssetState, AssetsCheck, BlockId, XrefState, XrefsList, XrefsOperation};
use kentos_native_application::assets_check;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// How often the references' files are looked at.
const EVERY: Duration = Duration::from_secs(5);

/// A source that changed, as the card tells it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notice {
    pub names: Vec<String>,
    pub sheets: Vec<String>,
    pub views: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Watch {
    /// Each loaded reference's file as last seen: its size and change time.
    stamps: HashMap<BlockId, (u64, SystemTime)>,
    /// The SHA-256 of a changed source already told, so a closed card comes back only when the file changes again.
    told: HashMap<BlockId, String>,
    pub notice: Option<Notice>,
    /// Each linked picture's file as last seen (none: not there).
    pictures: HashMap<String, Option<(u64, SystemTime)>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Tick,
    Reload,
    Open,
    Close,
}

fn msg(e: Event) -> Message {
    Message::XrefWatch(e)
}

/// The ticks while a drawing has a loaded reference.
pub fn ticks() -> impl Stream<Item = Message> {
    let (mut out, ticks) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(EVERY);
            if iced::futures::executor::block_on(out.send(msg(Event::Tick))).is_err() {
                break;
            }
        }
    });
    ticks
}

fn stamp(path: &Path) -> Option<(u64, SystemTime)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.len(), m.modified().ok()?))
}

impl App {
    fn drawing_file(&self) -> Option<String> {
        self.document.as_ref()?.path.as_ref().map(|p| p.display().to_string())
    }

    /// Whether the open drawing has a loaded external reference or a linked picture to follow.
    pub(crate) fn xrefs_watching(&self) -> bool {
        self.document.as_ref().is_some_and(|d| {
            d.model.blocks().iter().any(|b| b.xref.as_ref().is_some_and(|x| !x.unloaded))
                || self.styles.images.has_files()
        })
    }

    /// The linked pictures' files looked at: one that changed, came back or went is read again and drawn.
    fn pictures_tick(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let folder = doc.path.as_deref().and_then(Path::parent).map(Path::to_path_buf);
        let search = doc.model.settings().xref_paths.clone();
        let seen: Vec<(String, Option<(u64, SystemTime)>)> = self
            .styles
            .images
            .files()
            .into_iter()
            .map(|(file, _)| {
                let now = stamp(&crate::pictures::locate(&file, folder.as_deref(), &search));
                (file, now)
            })
            .collect();
        let mut again = Vec::new();
        for (file, now) in seen {
            if let Some(before) = self.xref_watch.pictures.insert(file.clone(), now)
                && before != now
            {
                again.push(file);
            }
        }
        let mut said = Vec::new();
        if let Some(doc) = self.document.as_ref() {
            for file in again {
                let key = format!("file:{file}");
                let bitmap = crate::pictures::fetch(&key, &doc.model, folder.as_deref());
                let read = bitmap.is_some();
                self.styles.images.put_bitmap(key, bitmap);
                let name = Path::new(&file).file_name().map_or_else(|| file.clone(), |n| n.to_string_lossy().into_owned());
                said.push(if read {
                    format!("{name} değişti; resim yeniden okundu.")
                } else {
                    format!("{name} okunamıyor; resmin yerinde uyarı çizildi.")
                });
            }
        }
        for s in said {
            self.output(s);
        }
    }

    /// The external references' files, with the SHA-256 compared (`cad.assets.check`).
    fn xref_assets(&self) -> Vec<kentos_commands::AssetStatus> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        let input = AssetsCheck { drawing: self.drawing_file(), kinds: Some(vec![AssetKind::Xref]), digests: Some(true) };
        assets_check::check_in(&doc.model, &input).assets
    }

    /// The drawing just opened: its linked files' state said, its references' files seen.
    pub(crate) fn xrefs_check_opened(&mut self) {
        self.xref_watch = Watch::default();
        let Some(doc) = self.document.as_ref() else { return };
        // The references with their SHA-256; the other links only looked for (a large raster is not read).
        let xrefs = self.xref_assets();
        let others = assets_check::check_in(
            &doc.model,
            &AssetsCheck { drawing: self.drawing_file(), kinds: None, digests: Some(false) },
        )
        .assets;
        let nested: Vec<BlockId> = doc
            .model
            .blocks()
            .iter()
            .filter(|b| b.xref.as_ref().is_some_and(|x| x.parent.is_some()))
            .map(|b| b.id)
            .collect();
        let changed: Vec<String> =
            xrefs.iter().filter(|a| a.state == AssetState::Changed).map(|a| a.name.clone()).collect();
        let missing: Vec<String> = xrefs
            .iter()
            .filter(|a| a.state == AssetState::Missing && a.block.is_none_or(|b| !nested.contains(&b)))
            .map(|a| a.name.clone())
            .collect();
        let lost: Vec<String> = others
            .iter()
            .filter(|a| a.kind != AssetKind::Xref && a.state == AssetState::Missing)
            .map(|a| a.name.clone())
            .collect();
        for a in &xrefs {
            if let (Some(b), Some(found)) = (a.block, &a.found)
                && let Some(s) = stamp(Path::new(found))
            {
                self.xref_watch.stamps.insert(b, s);
            }
            if let (Some(b), Some(now)) = (a.block, &a.current) {
                self.xref_watch.told.insert(b, now.clone());
            }
        }
        if !changed.is_empty() {
            self.warn(format!(
                "Dış başvuruların kaynağı değişti: {}. Çizim son yüklenen hâlini gösteriyor; yenisini almak için Ekle › Dış başvurular'da Yeniden yükle'ye basın.",
                changed.join(", ")
            ));
        }
        if !missing.is_empty() {
            self.warn(format!(
                "Dış başvuruların kaynağı bulunamadı: {}. Çizim son yüklenen hâlini gösteriyor; Ekle › Dış başvurular'da Yolu değiştir… ile dosyayı gösterin ya da arama yolu ekleyin.",
                missing.join(", ")
            ));
        }
        if !lost.is_empty() {
            let shown: Vec<&str> = lost.iter().take(5).map(String::as_str).collect();
            let more = lost.len().saturating_sub(shown.len());
            self.warn(format!(
                "{} bağlı dosya bulunamadı: {}{}. Ekle › Dış başvurular'ın Diğer bağlı dosyalar bölümü hepsini sayar.",
                lost.len(),
                shown.join(", "),
                if more > 0 { format!(" ve {more} dosya daha") } else { String::new() }
            ));
        }
    }

    /// After a command: each loaded reference's file as it is now is the one seen.
    pub(crate) fn xref_watch_seen(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let drawing = self.drawing_file();
        let search = doc.model.settings().xref_paths.clone();
        let mut stamps = HashMap::new();
        for b in doc.model.blocks() {
            let Some(x) = b.xref.as_ref().filter(|x| !x.unloaded) else { continue };
            let found = kentos_native_application::links::find(&x.path, x.relative.as_deref(), drawing.as_deref(), &search);
            if let Some(s) = found.at.as_deref().and_then(stamp) {
                stamps.insert(b.id, s);
            }
        }
        self.xref_watch.stamps = stamps;
    }

    /// The references' files looked at: a source that changed since it was told is told by the card.
    fn xrefs_tick(&mut self) {
        self.pictures_tick();
        let Some(doc) = self.document.as_ref() else { return };
        let drawing = self.drawing_file();
        let search = doc.model.settings().xref_paths.clone();
        let mut moved = false;
        for b in doc.model.blocks() {
            let Some(x) = b.xref.as_ref().filter(|x| !x.unloaded) else { continue };
            let found = kentos_native_application::links::find(&x.path, x.relative.as_deref(), drawing.as_deref(), &search);
            let now = found.at.as_deref().and_then(stamp);
            let before = self.xref_watch.stamps.get(&b.id).copied();
            if now != before {
                moved = true;
                match now {
                    Some(s) => self.xref_watch.stamps.insert(b.id, s),
                    None => self.xref_watch.stamps.remove(&b.id),
                };
            }
        }
        if !moved {
            return;
        }
        let changed: Vec<(BlockId, String, String)> = self
            .xref_assets()
            .into_iter()
            .filter(|a| a.state == AssetState::Changed)
            .filter_map(|a| Some((a.block?, a.name, a.current?)))
            .filter(|(b, _, now)| self.xref_watch.told.get(b) != Some(now))
            .collect();
        if changed.is_empty() {
            return;
        }
        let listed = self.document.as_ref().and_then(|doc| {
            let input = XrefsList { drawing: drawing.clone(), affected: true };
            match kentos_native_application::xrefs_list::plan(
                &kentos_native_application::ExecutionContext::new(&mut doc.model.clone()),
                &input,
            ) {
                kentos_commands::CommandResult::Completed { output, .. } => Some(output.xrefs),
                _ => None,
            }
        });
        let mut notice = Notice::default();
        for (b, name, now) in changed {
            self.xref_watch.told.insert(b, now);
            notice.names.push(name.clone());
            if let Some(x) = listed.as_ref().and_then(|l| l.iter().find(|x| x.name == name)) {
                for s in &x.sheets {
                    if !notice.sheets.contains(s) {
                        notice.sheets.push(s.clone());
                    }
                }
                for v in &x.views {
                    if !notice.views.contains(v) {
                        notice.views.push(v.clone());
                    }
                }
            }
        }
        self.warn(format!(
            "Dış başvurunun kaynağı değişti: {}. Çizim kendiliğinden yeniden yüklenmez.",
            notice.names.join(", ")
        ));
        // The window, when open, shows the new state.
        if self.dialog == Some(crate::app::Dialog::Xrefs) {
            self.open_xrefs();
        }
        self.xref_watch.notice = Some(notice);
    }

    pub(crate) fn xref_watch_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Tick => self.xrefs_tick(),
            Event::Reload => {
                let names = self.xref_watch.notice.take().map(|n| n.names).unwrap_or_default();
                let listed: Vec<(String, XrefState)> = self
                    .document
                    .as_ref()
                    .map(|_| self.xrefs_state())
                    .unwrap_or_default();
                for name in names {
                    if listed.iter().any(|(n, s)| *n == name && *s == XrefState::Changed) {
                        let _ = self.xref_reload(name);
                    }
                }
            }
            Event::Open => {
                self.xref_watch.notice = None;
                self.open_xrefs();
            }
            Event::Close => self.xref_watch.notice = None,
        }
        Task::none()
    }

    /// The references' names and states.
    fn xrefs_state(&self) -> Vec<(String, XrefState)> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        let input = XrefsList { drawing: self.drawing_file(), affected: false };
        kentos_native_application::xrefs::list_of(&doc.model, &input)
            .xrefs
            .into_iter()
            .map(|x| (x.name, x.state))
            .collect()
    }

    /// Yeniden yükle from the card: the same command as the window's.
    fn xref_reload(&mut self, name: String) -> Option<()> {
        let done = self.xref_run(XrefsOperation::Reload { name })?;
        self.say(
            kentos_interaction::Level::Success,
            format!("“{}” yeniden yüklendi: {} nesne (tek adımda geri alınır).", done.name, done.objects),
        );
        Some(())
    }

    /// The card over the drawing, while a changed source is told.
    pub(crate) fn xref_notice_view(&self) -> Option<Element<'_, Message>> {
        let n = self.xref_watch.notice.as_ref()?;
        let what = if n.names.len() == 1 {
            format!("“{}” dış başvurusunun kaynağı değişti.", n.names[0])
        } else {
            format!("{} dış başvurunun kaynağı değişti: {}.", n.names.len(), n.names.join(", "))
        };
        let mut seen = Vec::new();
        if !n.sheets.is_empty() {
            seen.push(format!("{} paftayı ({})", n.sheets.len(), n.sheets.join(", ")));
        }
        if !n.views.is_empty() {
            seen.push(format!("{} görünümü ({})", n.views.len(), n.views.join(", ")));
        }
        let effect = if seen.is_empty() {
            "Çizim son yüklenen hâlini gösteriyor.".to_owned()
        } else {
            format!("{} etkiler. Çizim son yüklenen hâlini gösteriyor.", seen.join(" ve "))
        };
        let words = Column::new().spacing(2).push(label::strong(what)).push(label::caption(effect));
        let actions = Row::new()
            .spacing(6)
            .push(
                button(label::body("Yeniden yükle"))
                    .on_press(msg(Event::Reload))
                    .padding([4, 10])
                    .style(style::button::primary),
            )
            .push(
                button(label::body("Dış başvurular"))
                    .on_press(msg(Event::Open))
                    .padding([4, 10])
                    .style(style::button::secondary),
            )
            .push(
                button(icon(Icon::Close).size(14.0))
                    .on_press(msg(Event::Close))
                    .padding(4)
                    .style(style::button::ghost),
            );
        let card = container(
            row![icon(Icon::Warning).size(18.0).tone(Tone::Warning), container(words).width(Fill), actions]
                .spacing(12)
                .align_y(Center),
        )
        .padding([10, 14])
        .max_width(720.0)
        .style(style::container::popover);
        Some(container(card).padding(12).center_x(Fill).into())
    }
}
