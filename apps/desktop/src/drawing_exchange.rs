//! Çizimler arası alışveriş (`file.saveSelection`, `file.takeFrom`,
//! `block.insertFile`; docs/adr/0193; the web's `app/drawingExchange.ts`
//! and `ui/io/TakeFromDialog.ts`): the rules are the domain's
//! (`kentos_domain::exchange`); this module asks for the files, shows Başka
//! çizimden al's window and puts what the rules give into the open drawing:
//! layers and blocks as one undo step, styles, the library, layer states and
//! settings as settings (not undo steps, docs/adr/0092, 0183 §1).

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use iced::futures::channel::mpsc;
use iced::widget::{Column, button, container, row, scrollable, text};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{
    BlockDefinition, BlockId, DocumentSnapshotV2, Entity, EntityId, LayerNode, LayerNodeType,
};
use kentos_domain::exchange::{self, Picks, Same, TAKEN_SETTINGS};
use kentos_domain::{NewLayer, Uuid};
use kentos_interaction::Level;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog as Frame, Elided, overlay};
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::{App, Dialog, Message, Picker};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TAKE_TITLE: &str = "Başka çizimden al";
const BLOCK_TITLE: &str = "Dosyadan blok ekle";
const TAKE: &str = "Al";
const CANCEL: &str = "Vazgeç";

/// The commands this module runs.
pub const COMMANDS: [&str; 3] = ["file.saveSelection", "file.takeFrom", "block.insertFile"];

/// What a chosen drawing file is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Take,
    Block,
}

/// Aynı adlı olanlar, as the segmented control says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameName {
    Skip,
    Replace,
}

impl fmt::Display for SameName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SameName::Skip => "Atla",
            SameName::Replace => "Değiştir",
        })
    }
}

impl SameName {
    fn rule(self) -> Same {
        match self {
            SameName::Skip => Same::Skip,
            SameName::Replace => Same::Replace,
        }
    }
}

/// A section of the window, in its order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Layers,
    Blocks,
    TextStyles,
    DimensionStyles,
    Library,
    LayerStates,
    Settings,
}

impl Section {
    const ALL: [Section; 7] = [
        Section::Layers,
        Section::Blocks,
        Section::TextStyles,
        Section::DimensionStyles,
        Section::Library,
        Section::LayerStates,
        Section::Settings,
    ];

    fn heading(self) -> &'static str {
        match self {
            Section::Layers => "Katmanlar",
            Section::Blocks => "Bloklar",
            Section::TextStyles => "Yazı stilleri",
            Section::DimensionStyles => "Ölçü stilleri",
            Section::Library => "Kitaplık",
            Section::LayerStates => "Katman durumları",
            Section::Settings => "Proje ayarları",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Section::Layers => "layers",
            Section::Blocks => "blocks",
            Section::TextStyles => "textStyles",
            Section::DimensionStyles => "dimensionStyles",
            Section::Library => "library",
            Section::LayerStates => "layerStates",
            Section::Settings => "settings",
        }
    }
}

/// A row of the window: what the picks name it by (a path, a name, an id),
/// its words, and whether this drawing has one of the same name (or id).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub section: Section,
    pub key: String,
    pub words: String,
    pub same: bool,
}

impl Row {
    fn id(&self) -> String {
        format!("{}:{}", self.section.key(), self.key)
    }
}

/// Başka çizimden al's window: the other drawing, its rows, what is checked.
#[derive(Debug, Clone)]
pub struct Window {
    name: String,
    theirs: Box<DocumentSnapshotV2>,
    rows: Vec<Row>,
    checked: BTreeSet<String>,
    same: SameName,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Where Seçilenleri dosyaya kaydet writes (none: the dialog cancelled).
    SaveTo(Option<PathBuf>),
    /// The selection's file written: what to say, or why not.
    Saved(Result<String, String>),
    /// A drawing file chosen (none: cancelled).
    Picked(Purpose, Option<PathBuf>),
    Check(String, bool),
    Same(SameName),
    Take,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::DrawingExchange(event)
}

const KIND_WORDS: [(&str, &str); 3] = [
    ("symbol", "Sembol"),
    ("asset", "Görüntü"),
    ("template", "Şablon"),
];

fn list(v: &Value) -> &[Value] {
    v.as_array().map_or(&[], Vec::as_slice)
}

fn name_of(v: &Value) -> &str {
    v["name"].as_str().unwrap_or("")
}

/// Every node with its path of names, in tree order.
fn walk(nodes: &[Value], above: &[String], out: &mut Vec<(Value, Vec<String>)>) {
    for n in nodes {
        let mut here = above.to_vec();
        here.push(name_of(n).to_owned());
        out.push((n.clone(), here.clone()));
        walk(list(&n["children"]), &here, out);
    }
}

/// What another drawing offers, row by row, each marked when this drawing
/// has the same (the web's `takeRows`).
pub fn take_rows(ours: &Value, theirs: &Value) -> Vec<Row> {
    let folded = |p: &[String]| p.iter().map(|s| exchange::fold(s)).collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut ours_walked = Vec::new();
    walk(list(&ours["layers"]), &[], &mut ours_walked);
    let our_paths: BTreeSet<Vec<String>> = ours_walked.iter().map(|(_, p)| folded(p)).collect();
    let mut theirs_walked = Vec::new();
    walk(list(&theirs["layers"]), &[], &mut theirs_walked);
    for (n, p) in theirs_walked {
        if n["type"] == "layer" {
            rows.push(Row {
                section: Section::Layers,
                key: p.join(" / "),
                words: p.join(" / "),
                same: our_paths.contains(&folded(&p)),
            });
        }
    }
    let our_blocks: BTreeSet<String> = list(&ours["blocks"])
        .iter()
        .map(|b| exchange::block_key(name_of(b)))
        .collect();
    for b in list(&theirs["blocks"]) {
        rows.push(Row {
            section: Section::Blocks,
            key: name_of(b).to_owned(),
            words: name_of(b).to_owned(),
            same: our_blocks.contains(&exchange::block_key(name_of(b))),
        });
    }
    for section in [
        Section::TextStyles,
        Section::DimensionStyles,
        Section::LayerStates,
    ] {
        let key = section.key();
        let mine: BTreeSet<String> = list(&ours["settings"][key])
            .iter()
            .map(|s| exchange::fold(name_of(s)))
            .collect();
        for s in list(&theirs["settings"][key]) {
            rows.push(Row {
                section,
                key: name_of(s).to_owned(),
                words: name_of(s).to_owned(),
                same: mine.contains(&exchange::fold(name_of(s))),
            });
        }
    }
    let our_items: BTreeSet<&str> = list(&ours["styles"]["items"])
        .iter()
        .filter_map(|it| it["id"].as_str())
        .collect();
    for it in list(&theirs["styles"]["items"]) {
        let id = it["id"].as_str().unwrap_or("");
        let kind = it["kind"].as_str().unwrap_or("");
        let kind_words = KIND_WORDS
            .iter()
            .find(|(k, _)| *k == kind)
            .map_or(kind, |(_, w)| *w);
        rows.push(Row {
            section: Section::Library,
            key: id.to_owned(),
            words: format!("{kind_words}: {}", it["name"].as_str().unwrap_or(id)),
            same: our_items.contains(id),
        });
    }
    rows.push(Row {
        section: Section::Settings,
        key: "settings".into(),
        words: "Birimler ve ondalıklar, açı birimi, çizim ölçeği, çizim yazı tipi, ölçme ayarları"
            .into(),
        same: false,
    });
    rows
}

/// The checked rows as Başka çizimden al's picks (the web's `picksOf`).
pub fn picks_of(rows: &[Row], checked: &BTreeSet<String>) -> Picks {
    let of = |s: Section| -> Vec<String> {
        rows.iter()
            .filter(|r| r.section == s && checked.contains(&r.id()))
            .map(|r| r.key.clone())
            .collect()
    };
    Picks {
        layers: of(Section::Layers),
        blocks: of(Section::Blocks),
        text_styles: of(Section::TextStyles),
        dimension_styles: of(Section::DimensionStyles),
        library: of(Section::Library),
        layer_states: of(Section::LayerStates),
        settings: checked.contains("settings:settings"),
    }
}

/// Every row checked but the project settings, as the window opens.
fn first_checked(rows: &[Row]) -> BTreeSet<String> {
    rows.iter()
        .filter(|r| r.section != Section::Settings)
        .map(Row::id)
        .collect()
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map_or_else(|| "Çizim".to_owned(), |s| s.to_string_lossy().into_owned())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// The selection's drawing written to `path` (docs/adr/0193 §1): what to say, or why not.
pub fn write_selection(
    drawing: &DocumentSnapshotV2,
    uids: &[EntityId],
    path: &Path,
) -> Result<String, String> {
    let made = exchange::selection(drawing, uids, &stem(path))
        .map_err(|why| format!("Seçilenler yazılamadı: {why}"))?;
    crate::saving::write_watched(
        &made,
        path,
        &AtomicBool::new(false),
        &mut |_| {},
        &crate::saving::Faults::NONE,
    )
    .map_err(|e| match e {
        crate::saving::SaveError::Failed(why) => format!("Seçilenler yazılamadı: {why}"),
        crate::saving::SaveError::Stopped => "Seçilenler yazılamadı: kayıt durduruldu.".to_owned(),
    })?;
    Ok(format!(
        "Seçilenler kaydedildi: {} nesne, {} blok → “{}”. Açık çizim değişmedi.",
        made.entities.len(),
        made.blocks.len(),
        file_name(path)
    ))
}

/// What went into the drawing as one undo step.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    layers: usize,
    blocks: usize,
    redefined: usize,
}

/// The layers of `nodes` the drawing has not, added under `parent`, and the
/// looks of those it has, in the open group (the web's `putLayersAndBlocks`).
fn put_nodes(
    model: &mut kentos_domain::Document,
    nodes: &[LayerNode],
    parent: Option<&str>,
    counts: &mut Counts,
) -> Result<(), String> {
    for n in nodes {
        match model.layers().get(&n.id).map(|m| m.style.clone()) {
            None => {
                let new = NewLayer {
                    id: Some(n.id.clone()),
                    name: n.name.clone(),
                    kind: n.kind,
                    visible: n.visible,
                    locked: n.locked,
                    style: n.style.clone(),
                    snap: n.snap.clone(),
                    fields: n.fields.clone(),
                    service: n.service.clone(),
                    feed: n.feed.clone(),
                };
                model.add_layer(new, parent, false).map_err(|r| r.0)?;
                counts.layers += 1;
            }
            Some(style) => {
                if style != n.style {
                    model.set_layer_style(&n.id, n.style.clone(), TAKE_TITLE);
                }
            }
        }
        let inside = if n.kind == LayerNodeType::Group {
            Some(n.id.as_str())
        } else {
            parent
        };
        put_nodes(model, &n.children, inside, counts)?;
    }
    Ok(())
}

/// Whether `nodes` would add a node to the drawing's tree or change a node's look.
fn changes_tree(model: &kentos_domain::Document, nodes: &[LayerNode]) -> bool {
    nodes.iter().any(|n| match model.layers().get(&n.id) {
        None => true,
        Some(m) => m.style != n.style || changes_tree(model, &n.children),
    })
}

/// The blocks of `blocks` the drawing has not, and the met ones redefined,
/// each once the blocks it places are there (the web's `putLayersAndBlocks`).
fn put_blocks(
    model: &mut kentos_domain::Document,
    blocks: &[BlockDefinition],
    counts: &mut Counts,
) -> Result<(), String> {
    let mut pending: Vec<BlockDefinition> = blocks.to_vec();
    let mut guard = pending.len() + 1;
    while !pending.is_empty() && guard > 0 {
        guard -= 1;
        let mut i = 0;
        while i < pending.len() {
            let b = &pending[i];
            // A definition may place only blocks the drawing has (docs/adr/0144): a redefinition waits too.
            let ready = b.entities.iter().all(|e| match e {
                Entity::Insert(ins) => ins.block == b.id || model.block(ins.block).is_some(),
                _ => true,
            });
            if !ready {
                i += 1;
                continue;
            }
            let b = pending.remove(i);
            if model.block(b.id).is_some() {
                if model.update_block(b).map_err(|r| r.0)? {
                    counts.redefined += 1;
                }
            } else {
                model.add_block(b).map_err(|r| r.0)?;
                counts.blocks += 1;
            }
        }
    }
    Ok(())
}

impl App {
    pub(crate) fn drawing_exchange_command(&mut self, id: &str) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        match id {
            "file.saveSelection" => {
                if self.selection.is_empty() {
                    self.warn("Seçilenleri dosyaya kaydet: önce kaydedilecek nesneleri seçin.");
                    return Task::none();
                }
                if let Picker::File(path) = &self.picker {
                    return Task::done(msg(Event::SaveTo(Some(path.clone()))));
                }
                let name = self
                    .document
                    .as_ref()
                    .map(|d| d.model.name().to_owned())
                    .unwrap_or_default();
                let suggested = format!(
                    "{} - seçim.kcad",
                    if name.is_empty() {
                        "Çizim"
                    } else {
                        name.as_str()
                    }
                );
                Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Seçilenleri dosyaya kaydet")
                            .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                            .set_file_name(suggested)
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::SaveTo(path)),
                )
            }
            "file.takeFrom" => self.pick_drawing(Purpose::Take),
            "block.insertFile" => self.pick_drawing(Purpose::Block),
            _ => Task::none(),
        }
    }

    fn pick_drawing(&mut self, purpose: Purpose) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Picked(purpose, Some(path.clone()))));
        }
        let title = match purpose {
            Purpose::Take => TAKE_TITLE,
            Purpose::Block => BLOCK_TITLE,
        };
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |path| msg(Event::Picked(purpose, path)),
        )
    }

    /// The other drawing read, or none (why said); its coordinate system's difference said too.
    fn read_other(&mut self, path: &Path, title: &str) -> Option<DocumentSnapshotV2> {
        let theirs = match crate::document::Document::read(path) {
            Ok(read) => read.model.to_snapshot_v2(),
            Err(why) => {
                self.warn(format!("{title}: “{}” okunamadı: {why}", file_name(path)));
                return None;
            }
        };
        let ours = self.document.as_ref()?.model.settings();
        if ours.srid != theirs.settings.srid || ours.custom_crs != theirs.settings.custom_crs {
            self.warn(format!(
                "{title}: Öbür çizimin koordinat sistemi bu çizimdekinden başka; koordinatlar dönüştürülmez."
            ));
        }
        Some(theirs)
    }

    pub(crate) fn drawing_exchange_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::SaveTo(None) | Event::Picked(_, None) => {}
            Event::SaveTo(Some(path)) => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let uids: Vec<EntityId> = self
                    .selection
                    .ids()
                    .iter()
                    .filter_map(|s| doc.model.uid(*s))
                    .map(|u| EntityId(*u.as_bytes()))
                    .collect();
                let drawing = doc.model.to_snapshot_v2();
                // Off the window's thread, as a save is (saving.rs).
                return iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
                    let done = write_selection(&drawing, &uids, &path);
                    let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                        &mut out,
                        msg(Event::Saved(done)),
                    ));
                });
            }
            Event::Saved(Ok(said)) => self.say(Level::Success, said),
            Event::Saved(Err(why)) => self.warn(why),
            Event::Picked(Purpose::Take, Some(path)) => self.open_take_from(&path),
            Event::Picked(Purpose::Block, Some(path)) => return self.insert_file_block(&path),
            Event::Check(k, on) => {
                if let Some(w) = self.take_from.as_mut() {
                    if on {
                        w.checked.insert(k);
                    } else {
                        w.checked.remove(&k);
                    }
                }
            }
            Event::Same(same) => {
                if let Some(w) = self.take_from.as_mut() {
                    w.same = same;
                }
            }
            Event::Take => {
                if let Some(w) = self.take_from.take() {
                    if self.take_into(&w) {
                        self.dialog = None;
                    } else {
                        self.take_from = Some(w);
                    }
                }
            }
            Event::Cancel => {
                self.take_from = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Başka çizimden al's window over the drawing in `path`.
    fn open_take_from(&mut self, path: &Path) {
        let Some(theirs) = self.read_other(path, TAKE_TITLE) else {
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let ours = serde_json::to_value(doc.model.to_snapshot_v2()).unwrap_or(Value::Null);
        let rows = take_rows(&ours, &serde_json::to_value(&theirs).unwrap_or(Value::Null));
        self.take_from = Some(Window {
            name: file_name(path),
            theirs: Box::new(theirs),
            checked: first_checked(&rows),
            rows,
            same: SameName::Skip,
        });
        self.dialog = Some(Dialog::TakeFrom);
    }

    /// The settings `result` holds, as settings (not an undo step): the
    /// styles, the layer states, the library and, with `units`, the units
    /// and the scale (the web's `putSettings`).
    fn put_settings(&mut self, result: &DocumentSnapshotV2, units: bool) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let mut settings = doc.model.settings().clone();
        settings
            .text_styles
            .clone_from(&result.settings.text_styles);
        settings
            .dimension_styles
            .clone_from(&result.settings.dimension_styles);
        settings
            .layer_states
            .clone_from(&result.settings.layer_states);
        if units {
            // By the contract's keys, as the web's: a key the other drawing leaves out takes its default.
            let theirs = serde_json::to_value(&result.settings).unwrap_or(Value::Null);
            let mut mine = serde_json::to_value(&settings).unwrap_or(Value::Null);
            if let Some(o) = mine.as_object_mut() {
                for key in TAKEN_SETTINGS {
                    match theirs.get(key) {
                        Some(v) => {
                            o.insert(key.to_owned(), v.clone());
                        }
                        None => {
                            o.remove(key);
                        }
                    }
                }
            }
            if let Ok(s) = serde_json::from_value(mine) {
                settings = s;
            }
        }
        // The annotations at the old general height follow a scale or heights taken (docs/adr/0205 §3).
        let before = doc.model.settings();
        let change = kentos_contracts::ScaleChange {
            from_scale: before.plot_scale,
            to_scale: settings.plot_scale,
            from: before.annotation.clone().unwrap_or_default(),
            to: settings.annotation.clone().unwrap_or_default(),
        };
        doc.model.set_settings(settings);
        doc.model.set_styles(result.styles.clone());
        if units {
            self.follow_annotations(change);
        }
    }

    /// Başka çizimden al's checked rows into the open drawing (docs/adr/0193 §2); whether they went in.
    fn take_into(&mut self, w: &Window) -> bool {
        let picks = picks_of(&w.rows, &w.checked);
        let locked = self.tree_locked();
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = match exchange::take(
            &doc.model.to_snapshot_v2(),
            &w.theirs,
            &picks,
            w.same.rule(),
        ) {
            Ok(result) => result,
            Err(why) => {
                self.warn(format!("{TAKE_TITLE}: {why}"));
                return false;
            }
        };
        if let Some(locked) = locked.filter(|_| changes_tree(&doc.model, &result.layers)) {
            self.warn(format!("{TAKE_TITLE}: {locked}"));
            return false;
        }
        let model = &mut doc.model;
        let group = model.begin_group(TAKE_TITLE);
        let mut counts = Counts::default();
        let done = put_nodes(model, &result.layers, None, &mut counts)
            .and_then(|()| put_blocks(model, &result.blocks, &mut counts));
        if let Err(why) = done {
            model.cancel_group(group);
            self.warn(format!("{TAKE_TITLE}: {why} Hiçbir şey alınmadı."));
            return false;
        }
        model.end_group(group);
        self.put_settings(&result, picks.settings);
        let parts: Vec<String> = [
            (counts.layers > 0).then(|| format!("{} katman ya da grup", counts.layers)),
            (counts.blocks > 0).then(|| format!("{} blok", counts.blocks)),
            (counts.redefined > 0).then(|| format!("{} blok yeniden tanımlandı", counts.redefined)),
        ]
        .into_iter()
        .flatten()
        .collect();
        let settings: Vec<&str> = [
            (!picks.text_styles.is_empty() || !picks.dimension_styles.is_empty())
                .then_some("stiller"),
            (!picks.library.is_empty()).then_some("kitaplık"),
            (!picks.layer_states.is_empty()).then_some("katman durumları"),
            picks.settings.then_some("proje ayarları"),
        ]
        .into_iter()
        .flatten()
        .collect();
        self.say(
            Level::Success,
            format!(
                "“{}” çiziminden alındı{}{}.",
                w.name,
                if parts.is_empty() {
                    String::new()
                } else {
                    format!(": {} (tek adımda geri alınır)", parts.join(", "))
                },
                if settings.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; {} ayar olarak (geri alma adımı değil)",
                        settings.join(", ")
                    )
                }
            ),
        );
        true
    }

    /// Kaynaklar's Katman olarak ekle (docs/adr/0199 §7; the web's
    /// `takeLayerInto`): `theirs`'s layer at `path` with its objects into the
    /// open drawing (`exchange::layer_take`): the layers, blocks and objects
    /// as one undo step, the styles and library items as settings; `from`
    /// names where it came from. The objects take new ids; a layer of ours
    /// that is locked takes none. Whether it went in.
    pub(crate) fn take_layer_into(
        &mut self,
        theirs: &DocumentSnapshotV2,
        path: &str,
        from: &str,
    ) -> bool {
        const TITLE: &str = "Katman olarak ekle";
        let locked = self.tree_locked();
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return false;
        };
        let made = match exchange::layer_take(&doc.model.to_snapshot_v2(), theirs, path) {
            Ok(made) => made,
            Err(why) => {
                self.warn(format!("{TITLE}: {why}"));
                return false;
            }
        };
        if let Some(locked) = locked.filter(|_| changes_tree(&doc.model, &made.drawing.layers)) {
            self.warn(format!("{TITLE}: {locked}"));
            return false;
        }
        let layers = doc.model.layers();
        let shut = made
            .objects
            .iter()
            .map(|e| e.base().layer_id.as_str())
            .find(|id| layers.get(id).is_some() && layers.is_locked(id))
            .map(|id| layers.path(id));
        if let Some(shut) = shut {
            self.warn(format!(
                "{TITLE}: “{shut}” katmanı kilitli; kilidini açıp yeniden deneyin."
            ));
            return false;
        }
        let model = &mut doc.model;
        let group = model.begin_group(TITLE);
        let mut counts = Counts::default();
        let objects = made.objects.len();
        let done = put_nodes(model, &made.drawing.layers, None, &mut counts)
            .and_then(|()| put_blocks(model, &made.drawing.blocks, &mut counts))
            .and_then(|()| {
                if made.objects.is_empty() {
                    return Ok(());
                }
                model
                    .add_many(made.objects.clone(), TITLE)
                    .map(|_| ())
                    .map_err(|_| "Çizimde yeni nesneye yer kalmadı.".to_owned())
            });
        if let Err(why) = done {
            model.cancel_group(group);
            self.warn(format!("{TITLE}: {why} Hiçbir şey alınmadı."));
            return false;
        }
        model.end_group(group);
        self.put_settings(&made.drawing, false);
        let parts: Vec<String> = [
            Some(format!("{objects} nesne")),
            (counts.layers > 0).then(|| format!("{} yeni katman ya da grup", counts.layers)),
            (counts.blocks > 0).then(|| format!("{} blok", counts.blocks)),
        ]
        .into_iter()
        .flatten()
        .collect();
        self.say(
            Level::Success,
            format!(
                "“{from}” içinden “{path}” alındı: {} (tek adımda geri alınır).",
                parts.join(", ")
            ),
        );
        true
    }

    /// Dosyadan blok ekle (docs/adr/0193 §3): the drawing in `path` as one block, then Blok ekle with it.
    fn insert_file_block(&mut self, path: &Path) -> Task<Message> {
        let Some(theirs) = self.read_other(path, BLOCK_TITLE) else {
            return Task::none();
        };
        let locked = self.tree_locked();
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let id = BlockId(*Uuid::now_v7().as_bytes());
        let made = match exchange::file_block(&doc.model.to_snapshot_v2(), &theirs, &stem(path), id)
        {
            Ok(made) => made,
            Err(why) => {
                self.warn(format!("{BLOCK_TITLE}: {why}"));
                return Task::none();
            }
        };
        if let Some(locked) = locked.filter(|_| made.layers > 0) {
            self.warn(format!("{BLOCK_TITLE}: {locked}"));
            return Task::none();
        }
        let model = &mut doc.model;
        let group = model.begin_group(BLOCK_TITLE);
        let mut counts = Counts::default();
        let done = put_nodes(model, &made.drawing.layers, None, &mut counts)
            .and_then(|()| put_blocks(model, &made.drawing.blocks, &mut counts));
        if let Err(why) = done {
            model.cancel_group(group);
            self.warn(format!("{BLOCK_TITLE}: {why} Hiçbir şey eklenmedi."));
            return Task::none();
        }
        model.end_group(group);
        self.put_settings(&made.drawing, false);
        let left: Vec<String> = [
            (made.images > 0).then(|| format!("{} resim", made.images)),
            (made.tables > 0).then(|| format!("{} tablo", made.tables)),
        ]
        .into_iter()
        .flatten()
        .collect();
        self.say(
            Level::Success,
            format!(
                "“{}” blok oldu: “{}”, {} nesne{}{}{}. Yerleştirme noktasına tıklayın.",
                file_name(path),
                made.name,
                made.objects,
                if made.nested > 0 {
                    format!(", {} iç blok", made.nested)
                } else {
                    String::new()
                },
                if counts.layers > 0 {
                    format!(", {} yeni katman", counts.layers)
                } else {
                    String::new()
                },
                if left.is_empty() {
                    String::new()
                } else {
                    format!("; {} bloğa konamadığı için alınmadı", left.join(" ve "))
                }
            ),
        );
        self.memory.block_insert = Some(id);
        self.start_tool(kentos_interaction::block_insert::ID)
    }

    pub(crate) fn take_from_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.take_from else {
            return text("").into();
        };
        let n = w.checked.len();
        let lines = vec![
            words::text_line(
                Kind::Info,
                format!(
                    "“{}” çiziminden işaretlenenler alınır. Katmanlar ve bloklar tek adımda geri alınır; stiller, kitaplık, katman durumları ve proje ayarları ayardır. Koordinat sistemi alınmaz.",
                    w.name
                ),
            ),
            if n == 0 {
                words::text_line(Kind::Info, "Alınacakları işaretleyin.")
            } else {
                words::text_line(Kind::Ok, format!("{n} öğe işaretli."))
            },
        ];
        let mut list = Column::new().spacing(0).padding([4, 0]);
        for section in Section::ALL {
            let of: Vec<&Row> = w.rows.iter().filter(|r| r.section == section).collect();
            if of.is_empty() {
                continue;
            }
            list = list.push(
                container(label::caption(format!(
                    "{} ({})",
                    section.heading(),
                    of.len()
                )))
                .padding([6, 10]),
            );
            for r in of {
                let k = r.id();
                let on = w.checked.contains(&k);
                let toggle = msg(Event::Check(k, !on));
                let boxed = check_box(
                    if on { Check::Checked } else { Check::Unchecked },
                    Some(toggle.clone()),
                );
                let words_el = Elided::new(r.words.clone())
                    .size(typography::caption())
                    .font(typography::ui())
                    .width(Fill);
                let mut face = row![boxed, words_el].spacing(6).align_y(Center);
                if r.same {
                    face = face.push(label::caption("bu çizimde var").style(
                        |theme: &iced::Theme| iced::widget::text::Style {
                            color: Some(Tokens::of(theme).muted),
                        },
                    ));
                }
                list = list.push(
                    button(face)
                        .on_press(toggle)
                        // Room on the right for the list's scroll bar.
                        .padding(iced::Padding {
                            top: 3.0,
                            right: 18.0,
                            bottom: 3.0,
                            left: 10.0,
                        })
                        .width(Fill)
                        .style(style::button::ghost),
                );
            }
        }
        let same = row![
            label::caption("Aynı adlı olanlar"),
            Segmented::new([SameName::Skip, SameName::Replace], w.same, |s| msg(
                Event::Same(s)
            )),
        ]
        .spacing(10)
        .align_y(Center);
        let body = Column::new()
            .spacing(12)
            .push(words::summary(lines))
            .push(
                container(scrollable(list).height(typography::scaled(300.0)))
                    .style(style::container::field_box)
                    .width(Fill),
            )
            .push(same);
        overlay::modal(
            Frame::new(TAKE_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(TAKE, (n > 0).then(|| msg(Event::Take))))
                .width(600.0),
            msg(Event::Cancel),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): a
    /// row's box by its words, Atla and Değiştir, Al and Vazgeç.
    pub(crate) fn take_from_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.take_from else {
            return Err(format!("{TAKE_TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Check(words, on) => {
                let Some(r) = w.rows.iter().find(|r| r.words == words) else {
                    return Err(format!("“{TAKE_TITLE}” penceresinde “{words}” kutusu yok"));
                };
                let k = r.id();
                (w.checked.contains(&k) != on).then(|| msg(Event::Check(k, on)))
            }
            Control::Press("Atla") => Some(msg(Event::Same(SameName::Skip))),
            Control::Press("Değiştir") => Some(msg(Event::Same(SameName::Replace))),
            Control::Press(TAKE) => (!w.checked.is_empty()).then(|| msg(Event::Take)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{TAKE_TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use kentos_domain::Slot;

    use super::*;
    use crate::files_testing::{drive, last_said, scratch};

    /// The traces' open drawing on screen; the file picker answers with `file`.
    fn app(file: PathBuf) -> App {
        let (mut app, _) = App::boot(None);
        let doc = crate::document::Document::read(&crate::traces::folder().join("exchange.kcad"))
            .expect("the drawing reads");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        app.picker = Picker::File(file);
        app
    }

    fn run(app: &mut App, id: &'static str) {
        let task = app.update(Message::Run(id));
        drive(app, task);
    }

    fn paths(nodes: &[LayerNode], above: &str, out: &mut Vec<String>) {
        for n in nodes {
            let here = if above.is_empty() {
                n.name.clone()
            } else {
                format!("{above} / {}", n.name)
            };
            out.push(here.clone());
            paths(&n.children, &here, out);
        }
    }

    #[test]
    fn the_selection_is_written_as_a_drawing_of_its_own_and_the_open_one_stays() {
        let path = scratch("secim").join("secim.kcad");
        let mut app = app(path.clone());
        // Parcel 101 and the Direk insert.
        app.selection.set([Slot(1), Slot(4)]);
        run(&mut app, "file.saveSelection");
        assert_eq!(
            last_said(&app),
            "Seçilenler kaydedildi: 2 nesne, 1 blok → “secim.kcad”. Açık çizim değişmedi."
        );
        let written = crate::document::Document::read(&path).expect("the written file reads");
        let snapshot = written.model.to_snapshot_v2();
        assert_eq!(snapshot.name, "secim");
        assert_eq!(snapshot.entities.len(), 2);
        assert_eq!(snapshot.blocks.len(), 1);
        let mut tree = Vec::new();
        paths(&snapshot.layers, "", &mut tree);
        assert_eq!(tree, ["Çizim", "Kadastro", "Kadastro / Parsel"]);
        let open = app.document.as_ref().expect("the drawing");
        assert!(!open.model.is_dirty(), "the open drawing is not changed");
        assert_eq!(open.model.entities().count(), 4);
    }

    #[test]
    fn without_a_selection_nothing_is_asked() {
        let path = scratch("secimsiz").join("secim.kcad");
        let mut app = app(path.clone());
        run(&mut app, "file.saveSelection");
        assert_eq!(
            last_said(&app),
            "Seçilenleri dosyaya kaydet: önce kaydedilecek nesneleri seçin."
        );
        assert!(!path.exists());
    }

    #[test]
    fn the_window_lists_what_the_other_drawing_offers_the_settings_unchecked() {
        let mut app = app(crate::traces::folder().join("altyapi-projesi.kcad"));
        run(&mut app, "file.takeFrom");
        assert_eq!(app.dialog, Some(Dialog::TakeFrom));
        let w = app.take_from.as_ref().expect("the window");
        // In the rule's order (the window shows them by section).
        let rows: Vec<(&str, bool)> = w.rows.iter().map(|r| (r.words.as_str(), r.same)).collect();
        assert_eq!(
            rows,
            [
                ("Kadastro / Parsel", true),
                ("Altyapı / Su hattı", false),
                ("Altyapı / Kanalizasyon", false),
                ("Aydınlatma", false),
                ("Kroki", false),
                ("Lamba", false),
                ("Direk", true),
                ("Rögar", false),
                ("BAŞLIK", true),
                ("Not", false),
                ("Altyapı", false),
                ("Yalnız altyapı", false),
                ("Sembol: Su hattı", false),
                (
                    "Birimler ve ondalıklar, açı birimi, çizim ölçeği, çizim yazı tipi, ölçme ayarları",
                    false
                ),
            ]
        );
        let picks = picks_of(&w.rows, &w.checked);
        assert!(!picks.settings);
        assert_eq!(picks.layers.len(), 5);
        assert_eq!(picks.blocks, ["Lamba", "Direk", "Rögar"]);
        // Vazgeç leaves the drawing as it was.
        let _ = app.update(msg(Event::Cancel));
        assert!(app.take_from.is_none() && app.dialog.is_none());
        assert!(!app.document.as_ref().expect("the drawing").model.is_dirty());
    }

    #[test]
    fn the_project_settings_come_only_when_checked() {
        let mut app = app(crate::traces::folder().join("altyapi-projesi.kcad"));
        run(&mut app, "file.takeFrom");
        let _ = app.update(msg(Event::Check("settings:settings".into(), true)));
        let _ = app.update(msg(Event::Take));
        let settings = app
            .document
            .as_ref()
            .expect("the drawing")
            .model
            .settings()
            .clone();
        assert_eq!(settings.length_decimals, 2);
        assert_eq!(settings.plot_scale, 1000.0);
        assert!(app.take_from.is_none());
    }
}
