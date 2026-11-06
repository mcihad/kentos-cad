//! Harita işi (plan/map-cad.md MAP-01, docs/adr/1200 §4): a map project's work structure in the dock, as
//! `map.work.query` reads it, drawn by KentOS UI's status tree (the road project panel's pattern): six headings (survey
//! campaigns, reference sources, working data, designs, check results, delivery packages), each item with its status or
//! its problem as a pill and its derivations under it. An item's row selects its layers' objects and its layer in
//! Katmanlar; a derivation's row selects its objects and shows them; the row's button relinks a missing file, accepts a
//! changed one or moves the status on (`map.work.edit`). Above: the kind and the name of a new item, added with the
//! layer chosen in Katmanlar (else the active one) and, by its second button, a file; Dosyaları oku reads every file's
//! SHA-256. The tree is read again only when the drawing's revision or the depth changes.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{button, column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    WorkDerivationRow, MapWorkEdit, MapWorkOperation, MapWorkTree, WorkFileState as F, WorkKind as K, WorkRow,
    WorkState as S, WorkStatus as W,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, map_work, map_work_edit};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::widget::tree_view::Check;
use kentos_ui::widget::{Status, StatusItem, StatusTree};

use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    /// An item's row, by id: its objects selected, its first layer chosen in Katmanlar.
    Item(String),
    /// A derivation's row, by id: its objects selected and shown.
    Derivation(String),
    /// The new item's kind and name.
    Kind(&'static str),
    Name(String),
    /// Add the new item with the layer chosen; `true`: ask for its file first.
    Add(bool),
    /// The new item's file chosen (none: the dialog was closed).
    AddFile(Option<PathBuf>),
    /// Ask for an item's new file; the file chosen.
    Relink(String),
    Relinked(String, Option<PathBuf>),
    /// Accept an item's changed file.
    Refresh(String),
    /// Set an item's status.
    Status(String, W),
    /// Read every file (deep), or back to looking for them.
    Deep(bool),
    Fold(String),
    Filter(Option<Status>),
}

fn msg(e: Event) -> Message {
    Message::MapWork(e)
}

type Read = (String, bool, Result<MapWorkTree, String>);

/// The panel's state: deep or quick, the new item's kind and name, the user's folds, the filter, and the last tree.
pub struct PanelState {
    pub deep: bool,
    kind: &'static str,
    name: String,
    toggled: HashSet<String>,
    filter: Option<Status>,
    read: RefCell<Option<Read>>,
}

impl Default for PanelState {
    fn default() -> Self {
        PanelState {
            deep: false,
            kind: "campaign",
            name: String::new(),
            toggled: HashSet::new(),
            filter: None,
            read: RefCell::new(None),
        }
    }
}

impl std::fmt::Debug for PanelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanelState").field("deep", &self.deep).finish()
    }
}

/// The kinds by their contract name, the heading and the icon of each.
const KINDS: [(&str, K, &str, &str); 6] = [
    ("campaign", K::Campaign, "Ölçüm kampanyaları", "importGnss"),
    ("reference", K::Reference, "Referans kaynaklar", "sourceLinked"),
    ("working", K::Working, "Çalışma verisi", "layers"),
    ("design", K::Design, "Tasarımlar", "surveyNetwork"),
    ("check", K::Check, "Denetim sonuçları", "check"),
    ("delivery", K::Delivery, "Teslim paketleri", "export"),
];

const KIND_CHOICES: [crate::style::fields::Choice; 6] = [
    ("campaign", "Ölçüm kampanyası"),
    ("reference", "Referans kaynak"),
    ("working", "Çalışma verisi"),
    ("design", "Tasarım"),
    ("check", "Denetim sonucu"),
    ("delivery", "Teslim paketi"),
];

fn kind_of(name: &str) -> K {
    KINDS.iter().find(|k| k.0 == name).map_or(K::Working, |k| k.1)
}

/// The status in words, and what it says beside: the office's record, never an institution's approval.
pub(crate) fn status_words(s: W) -> &'static str {
    match s {
        W::Draft => "Taslak",
        W::Review => "Denetlenecek",
        W::Approved => "Onaylı teslim revizyonu",
    }
}

/// A problem in words.
pub(crate) fn state_words(s: S) -> &'static str {
    match s {
        S::Current => "Güncel",
        S::Missing => "Dosyası yok",
        S::Changed => "Dosyası değişti",
        S::Incomplete => "Katmanı yok",
        S::Stale => "Onaydan sonra kaynağı değişti",
    }
}

fn derivation_words(s: S) -> &'static str {
    match s {
        S::Stale => "Güncel değil",
        S::Missing => "Nesnesi eksik",
        _ => "Güncel",
    }
}

impl App {
    /// The tree as the drawing is now, read again when its revision or the depth changed.
    pub(crate) fn map_work_read(&self) -> Option<Result<MapWorkTree, String>> {
        let doc = self.document.as_ref()?;
        let revision = doc.model.revision().to_string();
        let deep = self.map_work.deep;
        let mut kept = self.map_work.read.borrow_mut();
        if let Some((r, d, answer)) = kept.as_ref()
            && *r == revision
            && *d == deep
        {
            return Some(answer.clone());
        }
        let answer = map_work::preview(&doc.model, deep, None);
        *kept = Some((revision, deep, answer.clone()));
        Some(answer)
    }

    fn map_work_edit(&mut self, operation: MapWorkOperation) -> Option<kentos_commands::MapWorkEdited> {
        let doc = self.document.as_mut()?;
        let input = MapWorkEdit { operation, expected_revision: None };
        let result = map_work_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        self.answered(result)
    }

    /// The layer a new item takes: the one chosen in Katmanlar, else the active one.
    fn map_work_layer(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        Some(self.selected_layer.clone().unwrap_or_else(|| doc.model.layers().active().to_owned()))
    }

    fn map_work_add(&mut self, file: Option<PathBuf>) {
        let name = self.map_work.name.trim().to_owned();
        let name = if name.is_empty() {
            KIND_CHOICES.iter().find(|c| c.0 == self.map_work.kind).map_or("Öğe", |c| c.1).to_owned()
        } else {
            name
        };
        let operation = MapWorkOperation::Add {
            kind: kind_of(self.map_work.kind),
            name: name.clone(),
            layers: self.map_work_layer().into_iter().collect(),
            file: file.map(|p| p.to_string_lossy().into_owned()),
        };
        if let Some(out) = self.map_work_edit(operation) {
            self.map_work.name.clear();
            self.say(Level::Success, format!("“{name}” harita işine eklendi ({}).", out.item.unwrap_or_default()));
        }
    }

    fn pick_file(title: &'static str, then: impl FnOnce(Option<PathBuf>) -> Event + Send + 'static) -> Task<Message> {
        Task::perform(
            async move { rfd::AsyncFileDialog::new().set_title(title).pick_file().await.map(|f| f.path().to_path_buf()) },
            move |path| msg(then(path)),
        )
    }

    pub(crate) fn map_work_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Item(id) => self.select_item(&id),
            Event::Derivation(id) => self.select_derivation(&id),
            Event::Kind(k) => self.map_work.kind = k,
            Event::Name(n) => self.map_work.name = n,
            Event::Add(false) => self.map_work_add(None),
            Event::Add(true) => return Self::pick_file("Kaynağın dosyası", Event::AddFile),
            Event::AddFile(Some(path)) => self.map_work_add(Some(path)),
            Event::AddFile(None) => {}
            Event::Relink(id) => {
                return Self::pick_file("Kaynağın yeni yeri", move |p| Event::Relinked(id, p));
            }
            Event::Relinked(id, Some(path)) => {
                let file = path.to_string_lossy().into_owned();
                if let Some(out) = self.map_work_edit(MapWorkOperation::Relink { item: id, file }) {
                    let words = match out.relinked {
                        0 => "Kaynak yeniden bağlandı.".to_owned(),
                        n => format!("Kaynak yeniden bağlandı; {n} resim de yeni dosyayı gösteriyor."),
                    };
                    self.say(Level::Success, words);
                }
            }
            Event::Relinked(_, None) => {}
            Event::Refresh(id) => {
                if let Some(out) = self.map_work_edit(MapWorkOperation::Refresh { item: id }) {
                    self.say(Level::Info, format!("Kaynağın yeni içeriği kabul edildi; {} türetme güncel değil.", out.stale));
                }
            }
            Event::Status(id, status) => {
                let op = MapWorkOperation::Set { item: id, name: None, status: Some(status), layers: None };
                if self.map_work_edit(op).is_some() {
                    let words = match status {
                        W::Approved => "Onaylı teslim revizyonu kaydedildi; bu ofisin kaydıdır, kurum onayı değildir.",
                        W::Review => "Öğe denetlenecek.",
                        W::Draft => "Öğe taslağa döndü.",
                    };
                    self.say(Level::Info, words.to_owned());
                }
            }
            Event::Deep(on) => self.map_work.deep = on,
            Event::Fold(key) => {
                if !self.map_work.toggled.remove(&key) {
                    self.map_work.toggled.insert(key);
                }
            }
            Event::Filter(f) => self.map_work.filter = f,
        }
        Task::none()
    }

    fn select_slots(&mut self, slots: Vec<Slot>) {
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    fn select_item(&mut self, id: &str) {
        let Some(doc) = self.document.as_ref() else { return };
        let Some(item) = doc.model.map_work().and_then(|w| w.item(id)).cloned() else { return };
        let layers = doc.model.layers();
        let slots: Vec<Slot> = item
            .layers
            .iter()
            .flat_map(|l| layers.leaves_of(l))
            .flat_map(|leaf| doc.model.by_layer_with_uids(&leaf.id).map(|(u, _)| u).collect::<Vec<_>>())
            .filter_map(|u| doc.model.slot_of(u))
            .collect();
        if let Some(first) = item.layers.first() {
            self.selected_layer = Some(first.clone());
        }
        self.select_slots(slots);
    }

    fn select_derivation(&mut self, id: &str) {
        let Some(doc) = self.document.as_ref() else { return };
        let Some(d) = doc.model.map_work().and_then(|w| w.derivations.iter().find(|d| d.id == id)).cloned() else {
            return;
        };
        let slots = d.uids.iter().filter_map(|u| doc.model.slot_of(Uuid::from_bytes(u.0))).collect();
        self.select_slots(slots);
    }

    pub(crate) fn map_work_view(&self) -> Element<'_, Message> {
        let Some(read) = self.map_work_read() else {
            return container(label::caption("Açık çizim yok.")).padding([8, 12]).into();
        };
        let tree = match read {
            Ok(t) => t,
            Err(why) => return container(label::caption(why)).padding([8, 12]).into(),
        };
        let add = row![
            container(crate::style::fields::select(&KIND_CHOICES, self.map_work.kind, |k| msg(Event::Kind(k))))
                .width(150),
            crate::style::fields::input(None, "Adı", &self.map_work.name, false, false)
                .on_input(|n| msg(Event::Name(n)))
                .on_submit(msg(Event::Add(false))),
        ]
        .spacing(6)
        .align_y(Center);
        let buttons = row![
            button(label::body("Katmanla ekle"))
                .style(kentos_ui::style::button::secondary)
                .padding([3, 10])
                .on_press(msg(Event::Add(false))),
            button(label::body("Dosyayla ekle…"))
                .style(kentos_ui::style::button::secondary)
                .padding([3, 10])
                .on_press(msg(Event::Add(true))),
        ]
        .spacing(6);
        let on = |b: bool| if b { Check::Checked } else { Check::Unchecked };
        let deep = crate::style::fields::check(
            on(self.map_work.deep),
            "Dosyaları oku (SHA-256)".to_owned(),
            Some(msg(Event::Deep(!self.map_work.deep))),
        );
        let head = column![add, buttons, deep].spacing(6).padding([4, 12]).width(Fill);
        let mut items = Vec::new();
        for (name, kind, heading, glyph) in KINDS {
            let rows: Vec<&WorkRow> = tree.items.iter().filter(|i| i.kind == kind).collect();
            if rows.is_empty() {
                continue;
            }
            let mut group = StatusItem::new(format!("kind:{name}"), heading)
                .icon(icon(crate::icons::from_web(Some(glyph))).size(14.0))
                .meta(format!("{} öğe", rows.len()));
            for r in rows {
                let mut item = work_item(r);
                for d in tree.derivations.iter().filter(|d| d.source == r.id) {
                    item = item.push(derivation_item(d));
                }
                group = group.push(item);
            }
            items.push(group);
        }
        let body = StatusTree::new(items)
            .toggled(&self.map_work.toggled, |key| msg(Event::Fold(key)))
            .filter(self.map_work.filter, |f| msg(Event::Filter(f)))
            .counted("Güncel değil", "Sorun")
            .empty("Harita işinde öğe yok: türünü ve adını yazıp Katmanlar'da seçili katmanla ya da dosyasıyla ekleyin.");
        column![head, kentos_ui::widget::horizontal_divider(), body.view()].spacing(4).padding([6, 0]).into()
    }
}

/// An item's row: its name, its layers and object count faint, its problem or its status as a pill, why and the
/// office's-record note in the tip, and the button that mends it or moves its status on.
fn work_item<'a>(r: &WorkRow) -> StatusItem<'a, Message> {
    let mut detail: Vec<String> = r.why.clone();
    if let Some(f) = &r.file {
        detail.push(format!("Dosyası: {} (revizyon {}).", f.path, &f.revision[..12.min(f.revision.len())]));
    }
    if r.status == W::Approved {
        detail.push("Onay ofisin teslim kaydıdır; kurum onayı değildir.".into());
    }
    let (tone, words) = match r.state {
        S::Current => (
            match r.status {
                // A status is the office's record, not a problem: it never counts as one.
                W::Draft | W::Review => Status::Neutral,
                W::Approved => Status::Info,
            },
            status_words(r.status),
        ),
        S::Incomplete | S::Changed => (Status::Warning, state_words(r.state)),
        S::Missing | S::Stale => (Status::Danger, state_words(r.state)),
    };
    let meta = if r.layers.is_empty() { "katmansız".to_owned() } else { format!("{} nesne", r.objects) };
    let mut item = StatusItem::new(format!("item:{}", r.id), r.name.clone())
        .meta(meta)
        .status(tone, words)
        .detail(detail.join(" "))
        .on_press(msg(Event::Item(r.id.clone())));
    let file = r.file.as_ref().map(|f| f.state);
    item = match (file, r.status) {
        (Some(F::Missing), _) => {
            item.action(icon(Icon::Folder).size(13.0), "Yeniden bağla…", msg(Event::Relink(r.id.clone())))
        }
        (Some(F::Changed), _) => {
            item.action(icon(Icon::Redo).size(13.0), "Yenile: yeni içeriği kabul et", msg(Event::Refresh(r.id.clone())))
        }
        (_, W::Draft) => item.action(
            icon(Icon::Check).size(13.0),
            "Denetime gönder",
            msg(Event::Status(r.id.clone(), W::Review)),
        ),
        (_, W::Review) => item.action(
            icon(Icon::Check).size(13.0),
            "Onayla: ofisin teslim revizyonu (kurum onayı değildir)",
            msg(Event::Status(r.id.clone(), W::Approved)),
        ),
        (_, W::Approved) => {
            item.action(icon(Icon::Undo).size(13.0), "Taslağa döndür", msg(Event::Status(r.id.clone(), W::Draft)))
        }
    };
    item
}

/// A derivation's row: its operation, its object count, its state; why in the tip.
fn derivation_item<'a>(d: &WorkDerivationRow) -> StatusItem<'a, Message> {
    let tone = match d.state {
        S::Current => Status::Ok,
        S::Stale | S::Missing => Status::Warning,
        _ => Status::Neutral,
    };
    let meta = if d.objects == d.recorded {
        format!("{} nesne", d.objects)
    } else {
        format!("{}/{} nesne", d.objects, d.recorded)
    };
    let mut detail = d.why.clone();
    detail.push(format!("Kaynağın revizyonu {}.", &d.revision[..12.min(d.revision.len())]));
    StatusItem::new(format!("tr:{}", d.id), d.operation.clone())
        .icon(icon(Icon::Layers).size(14.0))
        .meta(meta)
        .status(tone, derivation_words(d.state))
        .detail(detail.join(" "))
        .on_press(msg(Event::Derivation(d.id.clone())))
}
