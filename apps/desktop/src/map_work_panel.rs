//! Harita işi (plan/map-cad.md MAP-01, docs/adr/1200 §4): a map project's work structure in the dock, as
//! `map.work.query` reads it, drawn by KentOS UI's status tree (the road project panel's pattern): six headings (survey
//! campaigns, reference sources, working data, designs, check results, delivery packages), each item with its status or
//! its problem as a pill and its derivations under it. An item's row selects its layers' objects and its layer in
//! Katmanlar; a derivation's row selects its objects and shows them; the row's button relinks a missing file, accepts a
//! changed one or moves the status on (`map.work.edit`). An approved item whose objects changed since its approval has
//! a row under it, Değişenler, whose press selects the objects changed and added (docs/adr/2880). Above:
//! the kind and the name of a new item, added with the layer chosen in Katmanlar (else the active one) and, by its
//! second button, a file; Dosyaları oku reads every file's SHA-256. The tree is read again only when the drawing's
//! revision or the depth changes.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    WorkDerivationRow, MapWorkEdit, MapWorkOperation, MapWorkTree, WorkChanges, WorkFileState as F, WorkKind as K,
    WorkRow, WorkState as S, WorkStatus as W,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, map_work, map_work_edit};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::widget::{Status, StatusItem, StatusTree};

use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    /// An item's row, by id: its objects selected, its first layer chosen in Katmanlar.
    Item(String),
    /// A derivation's row, by id: its objects selected and shown.
    Derivation(String),
    /// Değişenler under an approved item, by the item's id: the objects changed and added since its approval selected
    /// and shown (docs/adr/2880 §4).
    Changes(String),
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
    /// The item the selection is recorded as derived from (`derive`), and the recording.
    DeriveFrom(String),
    Derive,
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
    /// The item with a file the selection is recorded as derived from (Seçimi türet).
    derive_from: Option<String>,
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
            derive_from: None,
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
    ("campaign", K::Campaign, "Arazi ölçümleri", "importGnss"),
    ("reference", K::Reference, "Altlıklar", "sourceLinked"),
    ("working", K::Working, "Çalışma verisi", "layers"),
    ("design", K::Design, "Tasarımlar", "surveyNetwork"),
    ("check", K::Check, "Denetimler", "check"),
    ("delivery", K::Delivery, "Teslimler", "export"),
];

const KIND_CHOICES: [crate::style::fields::Choice; 6] = [
    ("campaign", "Arazi ölçümü"),
    ("reference", "Altlık"),
    ("working", "Çalışma verisi"),
    ("design", "Tasarım"),
    ("check", "Denetim"),
    ("delivery", "Teslim"),
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
        // Short: the row's name keeps its room in the dock; the tip says what changed (docs/adr/2080 §3).
        S::Revised => "Onay eskidi",
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
        // A moved project's files are found by their relative paths and the search paths (docs/adr/1201 §1).
        let drawing = doc.path.as_ref().filter(|p| p.is_absolute()).map(|p| p.display().to_string());
        let answer = map_work::preview(&doc.model, deep, None, drawing.as_deref());
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
            Event::Changes(id) => self.select_changes(&id),
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
            Event::DeriveFrom(id) => self.map_work.derive_from = Some(id),
            Event::Derive => self.map_work_derive(),
        }
        Task::none()
    }

    /// The lasting ids of the selected objects.
    fn selected_uids(&self) -> Vec<String> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        self.selection.ids().iter().filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect()
    }

    /// Where each selected object comes from in the map's work (read quickly: the items' revisions, a design's content
    /// revision worked out once).
    fn selected_sources(&self) -> Vec<kentos_commands::WorkObjectSource> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        map_work::sources_of(&doc.model, &self.selected_uids())
    }

    /// `map.workSource` (docs/adr/1201 §3): one selected object's source said, its item's layer chosen in Katmanlar
    /// and Harita işi brought to the front: Civil 3D's contextual tab's way to the source of what is selected.
    pub(crate) fn map_work_show_source(&mut self) {
        let Some(src) = self.selected_sources().into_iter().next() else {
            self.say(Level::Warn, "Seçili nesne harita işinin bir öğesinde değil.");
            return;
        };
        let Some(work) = self.document.as_ref().and_then(|d| d.model.map_work().cloned()) else { return };
        let name = |id: &Option<String>| {
            id.as_deref().and_then(|i| work.item(i)).map_or_else(|| "?".to_owned(), |i| i.name.clone())
        };
        let words = match src.role {
            kentos_commands::WorkObjectRole::Derived => format!(
                "Nesne “{}” kaynağından türetildi ({}, {}); {}.",
                name(&src.source),
                src.derivation.clone().unwrap_or_default(),
                src.operation.clone().unwrap_or_default(),
                if src.current == Some(false) { "kaynağı sonra değişti, güncel değil" } else { "güncel" }
            ),
            kentos_commands::WorkObjectRole::Source => format!("Nesne “{}” öğesinin kaynak verisi.", name(&src.source)),
            kentos_commands::WorkObjectRole::Free => match &src.item {
                Some(_) => format!("Nesne “{}” öğesinde, kullanıcının bağımsız çizimi.", name(&src.item)),
                None => "Nesne harita işinin bir öğesinde değil; kullanıcının bağımsız çizimi.".to_owned(),
            },
        };
        let item = src.source.clone().or(src.item.clone());
        if let Some(first) = item.as_deref().and_then(|i| work.item(i)).and_then(|i| i.layers.first()) {
            self.selected_layer = Some(first.clone());
        }
        if item.is_some() {
            // Its row not hidden by a filter.
            self.map_work.filter = None;
        }
        if self.open_panel(crate::app::Panel::MapWork) {
            if !self.right_panel_shown() {
                self.toggle_right_panel();
            }
            self.docks.update(kentos_ui::widget::docking::Event::Selected(crate::app::Panel::MapWork));
        }
        self.say(Level::Info, words);
    }

    /// `map.workRelease` (docs/adr/1201 §3): the selected derived objects let go of their derivations, one step.
    pub(crate) fn map_work_release(&mut self) {
        let uids: Vec<String> = self
            .selected_sources()
            .into_iter()
            .filter(|s| s.role == kentos_commands::WorkObjectRole::Derived)
            .map(|s| s.uid)
            .collect();
        if uids.is_empty() {
            self.say(Level::Warn, "Seçimde türetilmiş nesne yok; bağımsız yapılacak bir şey yok.");
            return;
        }
        if let Some(out) = self.map_work_edit(MapWorkOperation::Release { uids }) {
            let gone = match out.released {
                0 => String::new(),
                n => format!(" {n} türetme kaydı boşaldı ve kalktı."),
            };
            self.say(Level::Success, format!("{} nesne bağımsız çizim oldu; kaynağını izlemiyor.{gone}", out.freed));
        }
    }

    /// Seçimi türet: the selected objects recorded as made from the item chosen, at its revision now.
    fn map_work_derive(&mut self) {
        let Some(source) = self.map_work.derive_from.clone().or_else(|| self.source_items().first().map(|i| i.0.clone()))
        else {
            return;
        };
        let uids = self.underived_uids();
        if uids.is_empty() {
            return;
        }
        let operation = MapWorkOperation::Derive { source, uids, operation: "Elle türetme kaydı".into() };
        if let Some(out) = self.map_work_edit(operation) {
            self.say(Level::Success, format!("Seçim türetildi diye kaydedildi ({}).", out.derivation.unwrap_or_default()));
        }
    }

    /// The selected objects no derivation names yet: what Seçimi türet records (a derived one is already another's).
    fn underived_uids(&self) -> Vec<String> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        let Some(work) = doc.model.map_work() else { return self.selected_uids() };
        // One pass over the records' objects (the panel draws every frame; a record may hold an import's 10⁵ points).
        let mut left: Vec<String> = self.selected_uids();
        let mut wanted: HashSet<String> = left.iter().cloned().collect();
        for x in work.derivations.iter().flat_map(|d| d.uids.iter()) {
            if wanted.is_empty() {
                break;
            }
            wanted.remove(&x.to_text());
        }
        left.retain(|u| wanted.contains(u));
        left
    }

    /// The items the selection can be recorded as made from, by id and name: those with a file, and those without
    /// one (a design, by its content revision; docs/adr/2080 §2) that hold none of the selected objects.
    pub(crate) fn source_items(&self) -> Vec<(String, String)> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        let Some(work) = doc.model.map_work() else { return Vec::new() };
        let mut holding: HashSet<&str> = HashSet::new();
        for slot in self.selection.ids() {
            if let Some(e) = doc.model.get(*slot)
                && let Some(i) = map_work::item_of_layer(&doc.model, work, &e.base().layer_id)
            {
                holding.insert(i.id.as_str());
            }
        }
        work.items
            .iter()
            .filter(|i| i.file.is_some() || !holding.contains(i.id.as_str()))
            .map(|i| (i.id.clone(), i.name.clone()))
            .collect()
    }

    /// Whether `map.workSource` and `map.workRelease` have something to work on (`App::available`): roles only, nothing
    /// worked out (the ribbon asks every frame).
    pub(crate) fn map_work_available(&self, id: &str) -> bool {
        if self.selection.is_empty() || self.selection.len() > 10_000 {
            return false;
        }
        let Some(doc) = self.document.as_ref() else { return false };
        let mut roles = self.selected_uids().into_iter().filter_map(|u| map_work::role_of(&doc.model, &u));
        match id {
            "map.workSource" => roles.any(|(role, item)| item.is_some() || role != kentos_commands::WorkObjectRole::Free),
            _ => roles.any(|(role, _)| role == kentos_commands::WorkObjectRole::Derived),
        }
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

    /// Değişenler (docs/adr/2880 §4): the objects changed and added since the item's approval selected
    /// and shown, as the tree lists them (at most `WORK_CHANGES_LISTED`); the removed ones are gone from the drawing.
    fn select_changes(&mut self, id: &str) {
        let Some(Ok(tree)) = self.map_work_read() else { return };
        let Some(c) = tree.items.iter().find(|r| r.id == id).and_then(|r| r.changes.clone()) else { return };
        let Some(doc) = self.document.as_ref() else { return };
        let slots: Vec<Slot> =
            c.uids.iter().filter_map(|u| Uuid::parse_str(u).ok()).filter_map(|u| doc.model.slot_of(u)).collect();
        let n = slots.len();
        // Docs/adr/3680: a removed object as approved, when the approval kept its objects' values.
        let gone = match (c.removed, removed_words(&c)) {
            (0, _) => String::new(),
            (r, None) => format!("; silinen {r} nesne çizimde yok"),
            (r, Some(was)) => format!("; silinen {r} nesne çizimde yok, onaylanan hâli: {was}"),
        };
        let attributes = attribute_words(&c).map_or(String::new(), |w| format!(" ({w})"));
        // Docs/adr/3280: the changed objects' parts, when the approval kept them.
        let which = c.parts.as_ref().map_or(String::new(), |p| {
            let named: Vec<String> =
                [(p.geometry, "geometrisi"), (p.attributes, "öznitelikleri"), (p.layer, "katmanı"), (p.look, "görünüşü")]
                    .into_iter()
                    .filter(|(k, _)| *k > 0)
                    .map(|(k, w)| format!("{k} nesnenin {w}"))
                    .collect();
            if named.is_empty() { String::new() } else { format!("; değişen: {}{attributes}", named.join(", ")) }
        });
        let words = if n == 0 {
            format!("Onaydan sonra değişen ya da eklenen nesne çizimde yok{gone}.")
        } else {
            let first = if (c.changed + c.added) as usize > n { format!(" (ilk {n})") } else { String::new() };
            self.select_slots(slots);
            format!("Onaydan sonra değişen ya da eklenen {n} nesne seçildi{first}{which}{gone}.")
        };
        self.say(Level::Info, words);
    }

    pub(crate) fn map_work_view(&self) -> Element<'_, Message> {
        let Some(read) = self.map_work_read() else {
            return container(label::caption("Açık çizim yok.")).padding([8, 12]).into();
        };
        let tree = match read {
            Ok(t) => t,
            Err(why) => return container(label::caption(why)).padding([8, 12]).into(),
        };
        // One row (the owner, 6 Kasım 2026: four rows of buttons took the panel's room): the new item's kind and name,
        // then icon buttons whose words are their tips: add with the layer chosen, add with a file, read the files.
        let tools = row![
            container(crate::style::fields::select(&KIND_CHOICES, self.map_work.kind, |k| msg(Event::Kind(k))))
                .width(118),
            crate::style::fields::input(None, "Yeni öğenin adı", &self.map_work.name, false, false)
                .on_input(|n| msg(Event::Name(n)))
                .on_submit(msg(Event::Add(false))),
            crate::style::strip::icon_button(
                Icon::Layers,
                "Ekle: Katmanlar'da seçili katmanla (Enter)",
                Some(msg(Event::Add(false))),
            ),
            crate::style::strip::icon_button(Icon::Folder, "Ekle: dosyasıyla…", Some(msg(Event::Add(true)))),
            crate::style::strip::icon_toggle(
                Icon::Hash,
                "Dosyaları oku: içeriklerini (SHA-256) kabul edilen revizyonla karşılaştır",
                self.map_work.deep,
                msg(Event::Deep(!self.map_work.deep)),
            ),
        ]
        .spacing(4)
        .align_y(Center);
        let mut head = column![tools].spacing(6).padding([4, 12]).width(Fill);
        // Seçimi türet (docs/adr/1201 §3): the selection recorded as made from an item with a file or a design
        // (docs/adr/2080 §2).
        let sources = self.source_items();
        let free = if sources.is_empty() || self.selection.len() > 10_000 { 0 } else { self.underived_uids().len() };
        if free > 0 {
            let chosen = self
                .map_work
                .derive_from
                .clone()
                .filter(|c| sources.iter().any(|s| s.0 == *c))
                .unwrap_or_else(|| sources[0].0.clone());
            let pick = crate::style::fields::select_owned(sources, &chosen, |id| msg(Event::DeriveFrom(id)));
            head = head.push(
                row![
                    label::caption(format!("{free} nesnenin kaynağı:")),
                    container(pick).width(Fill),
                    crate::style::strip::icon_button(
                        Icon::Link,
                        "Seçimi türet: seçili nesneleri bu kaynaktan türetilmiş diye kaydet",
                        Some(msg(Event::Derive)),
                    ),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
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
        detail.push(format!("Dosyası: {} (revizyon {}).", f.path, short(&f.revision)));
    }
    if let Some(c) = &r.content {
        detail.push(format!("İçerik revizyonu {}.", short(c)));
    }
    if let Some(a) = &r.approved {
        detail.push(format!("Onaylanan revizyon {}.", short(a)));
    }
    if let Some(c) = &r.changes {
        detail.push(changes_words(c));
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
        S::Incomplete | S::Changed | S::Revised => (Status::Warning, state_words(r.state)),
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
        // Moved on since its approval (docs/adr/2080 §3): approving again stamps today's revision.
        (_, W::Approved) if r.state == S::Revised => item.action(
            icon(Icon::Check).size(13.0),
            "Yeniden onayla: bugünkü revizyonu damgala (kurum onayı değildir)",
            msg(Event::Status(r.id.clone(), W::Approved)),
        ),
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
    if let Some(c) = &r.changes {
        item = item.push(changes_item(r, c));
    }
    item
}

/// A value as said: a text in quotes, anything else as written.
fn value_words(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("“{s}”"),
        other => other.to_string(),
    }
}

/// docs/adr/3680: the attributes that changed since the approval, each by name with its value as approved and now
/// (“yok”: not there), the first three of them; none when the approval kept no values.
pub(crate) fn attribute_words(c: &WorkChanges) -> Option<String> {
    let all: Vec<String> = c
        .objects
        .iter()
        .flat_map(|o| &o.attributes)
        .map(|a| {
            let side = |v: &Option<serde_json::Value>| v.as_ref().map_or_else(|| "yok".to_owned(), value_words);
            format!("{}: {} → {}", a.name, side(&a.approved), side(&a.now))
        })
        .collect();
    if all.is_empty() {
        return None;
    }
    let more = all.len().saturating_sub(3);
    let first = all.into_iter().take(3).collect::<Vec<_>>().join("; ");
    Some(if more > 0 { format!("{first} ve {more} değişiklik daha") } else { first })
}

/// docs/adr/3680: the removed objects as approved, the first three: each its kind, layer and attributes.
pub(crate) fn removed_words(c: &WorkChanges) -> Option<String> {
    if c.removed_objects.is_empty() {
        return None;
    }
    let said: Vec<String> = c
        .removed_objects
        .iter()
        .take(3)
        .map(|r| {
            let kind = crate::selecting::kind_title(r.approved.get("kind").and_then(serde_json::Value::as_str).unwrap_or(""));
            let layer = r.approved.get("layerId").and_then(serde_json::Value::as_str).unwrap_or("");
            let attrs: Vec<String> = r
                .approved
                .get("attrs")
                .and_then(serde_json::Value::as_object)
                .map(|m| m.iter().map(|(k, v)| format!("{k} {}", value_words(v))).collect())
                .unwrap_or_default();
            if attrs.is_empty() { format!("{kind} ({layer})") } else { format!("{kind} ({layer}; {})", attrs.join(", ")) }
        })
        .collect();
    let more = c.removed_objects.len().saturating_sub(3);
    Some(if more > 0 { format!("{} ve {more} nesne daha", said.join("; ")) } else { said.join("; ") })
}

/// What an approved item's objects did since its approval, in one sentence (docs/adr/2880 §4); with the approval's
/// parts (docs/adr/3280), which parts of the changed objects changed, each part's count.
pub(crate) fn changes_words(c: &WorkChanges) -> String {
    let which = c.parts.as_ref().map_or(String::new(), |p| {
        let named: Vec<String> = [(p.geometry, "geometri"), (p.attributes, "öznitelik"), (p.layer, "katman"), (p.look, "görünüş")]
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, w)| format!("{w} {n}"))
            .collect();
        if named.is_empty() { String::new() } else { format!(" ({})", named.join(", ")) }
    });
    let parts: Vec<String> = [(c.changed, "değişti"), (c.added, "eklendi"), (c.removed, "silindi")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| if w == "değişti" { format!("{n} nesne {w}{which}") } else { format!("{n} nesne {w}") })
        .collect();
    match parts.is_empty() {
        true => "Onaydan sonra nesneleri değişti.".to_owned(),
        false => format!("Onaydan sonra {}.", parts.join(", ")),
    }
}

/// Değişenler, the row under an approved item whose objects changed since its approval: how many, what each did in
/// the tip, its press selecting the objects changed and added, as a derivation's row selects its objects. Short, so
/// that it reads in the narrow dock under its item. The pill is information, not a second problem: the item's own row
/// already counts as one.
fn changes_item<'a>(r: &WorkRow, c: &WorkChanges) -> StatusItem<'a, Message> {
    let there = (c.changed + c.added) as usize;
    let mut detail = vec![changes_words(c)];
    // Docs/adr/3680: the attributes by name with their values, the removed objects as approved.
    if let Some(w) = attribute_words(c) {
        detail.push(format!("Değişen öznitelikler: {w}."));
    }
    if let Some(w) = removed_words(c) {
        detail.push(format!("Silinenlerin onaylanan hâli: {w}."));
    }
    detail.push(match (there, c.removed) {
        (0, _) => "Silinenler çizimde yok; seçilecek nesne yok.".to_owned(),
        (_, 0) => "Basınca değişen ve eklenen nesneler seçilir ve gösterilir.".to_owned(),
        _ => "Basınca değişen ve eklenen nesneler seçilir ve gösterilir; silinenler çizimde yok.".to_owned(),
    });
    if there > c.uids.len() {
        detail.push(format!("İlk {} nesne seçilir.", c.uids.len()));
    }
    let total = c.changed + c.added + c.removed;
    let mut item = StatusItem::new(format!("ch:{}", r.id), "Değişenler")
        .icon(icon(Icon::Select).size(14.0))
        .status(Status::Info, format!("{total} nesne"))
        .detail(detail.join(" "));
    if there > 0 {
        item = item.on_press(msg(Event::Changes(r.id.clone())));
    }
    item
}

/// A revision's first 12 hex digits.
fn short(revision: &str) -> &str {
    revision.get(..12).unwrap_or(revision)
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
    detail.push(format!("Kaynağın revizyonu {}.", short(&d.revision)));
    StatusItem::new(format!("tr:{}", d.id), d.operation.clone())
        .icon(icon(Icon::Layers).size(14.0))
        .meta(meta)
        .status(tone, derivation_words(d.state))
        .detail(detail.join(" "))
        .on_press(msg(Event::Derivation(d.id.clone())))
}
