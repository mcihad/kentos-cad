//! Var olan katmanla birleştir (docs/adr/0628 §5, §6, §8): the import windows' section that takes a file's rows into
//! the drawing's layer they go onto. The rows meet its objects by a key column (or by their lasting ids); a matched
//! object is updated, only filled or left; the others are added or left; a key the file repeats stops the import, or
//! its first row is taken, or none; the rows refused are written to a CSV beside the file. The layer is the one the
//! file's name meets unless another is chosen. The section is the command's `merge`, `rejects` and target layer: the
//! window's plan counts as the choices change, a script gives the same.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use iced::Element;
use iced::widget::Column;
use kentos_commands::{
    CommandWarning, ImportMerge, MergeDuplicates, MergeMatched, MergeReport, MergeUnmatched,
};
use kentos_domain::Document;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};

use super::words::{self, Kind as Line};
use crate::app::Message;

/// What a matched object takes, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matched(pub MergeMatched);

impl fmt::Display for Matched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            MergeMatched::Update => "Güncelle",
            MergeMatched::FillEmpty => "Yalnız boşları doldur",
            MergeMatched::Keep => "Dokunma",
        })
    }
}

/// What a row no object matches does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unmatched(pub MergeUnmatched);

impl fmt::Display for Unmatched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            MergeUnmatched::Add => "Ekle",
            MergeUnmatched::Skip => "Alma",
        })
    }
}

/// What a key the file repeats does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duplicates(pub MergeDuplicates);

impl fmt::Display for Duplicates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            MergeDuplicates::Stop => "İçe aktarmayı durdur",
            MergeDuplicates::First => "İlkini al",
            MergeDuplicates::None => "Hiçbirini alma",
        })
    }
}

#[derive(Debug, Clone)]
pub enum MergeEvent {
    On,
    /// The drawing's layer the rows go onto among the section's (0: the one the file's name meets).
    Layer(usize),
    /// The key among the section's names (0: the lasting id where the rows have one).
    Key(usize),
    /// The layer's attribute the key meets (0: the key's own name).
    TargetKey(usize),
    Matched(Matched),
    Geometry,
    Unmatched(Unmatched),
    Duplicates(Duplicates),
    Rejects,
}

/// The section's choices.
#[derive(Debug, Clone)]
pub struct MergeForm {
    pub on: bool,
    /// The column naming a row; none: its lasting id.
    pub key: Option<String>,
    /// The layer's attribute the key meets; none: the key's own name.
    pub target_key: Option<String>,
    pub matched: MergeMatched,
    pub geometry: bool,
    pub unmatched: MergeUnmatched,
    pub duplicates: MergeDuplicates,
    /// Whether the rows refused are written beside the file.
    pub rejects: bool,
    /// The drawing's layer chosen for the rows; none: the one the file's name meets, else a new one.
    pub layer: Option<String>,
    /// Whether the file's rows have lasting ids (a GeoJSON's, a Shapefile's and a KML's do not: a key is asked).
    pub ids: bool,
    source: PathBuf,
    /// The drawing's layer the rows go onto, as the last plan said; none: every table makes a new one.
    target: Option<String>,
    /// The keys offered: the target's fields, else its objects' attribute names.
    names: Vec<String>,
    /// The layers offered for the rows: id and path (not locked, not a service's).
    layers: Vec<(String, String)>,
    /// The file's tables the last plan read (a GeoPackage's or a GML's), each sent onto the chosen layer.
    tables: Vec<String>,
}

impl MergeForm {
    pub fn new(source: &Path, ids: bool) -> Self {
        MergeForm {
            on: false,
            key: None,
            target_key: None,
            matched: MergeMatched::Update,
            geometry: true,
            unmatched: MergeUnmatched::Add,
            duplicates: MergeDuplicates::Stop,
            rejects: false,
            layer: None,
            ids,
            source: source.to_path_buf(),
            target: None,
            names: Vec::new(),
            layers: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// What a plan said: the tables' layers (`targets`) and those it makes (`made`); the first that is the drawing's
    /// is the target. A plan that failed says nothing: the last stays.
    pub fn learn<'a>(
        &mut self,
        doc: &Document,
        targets: impl IntoIterator<Item = &'a str>,
        made: &[&str],
    ) {
        self.target = targets
            .into_iter()
            .find(|t| !made.contains(t) && doc.layers().get(t).is_some())
            .map(str::to_owned);
        self.names = self
            .target
            .as_deref()
            .map_or_else(Vec::new, |t| layer_names(doc, t));
        self.offer(doc);
    }

    /// The drawing's layers offered, and with one chosen its keys: what the section shows whatever the last plan said
    /// (a plan refused before a layer is chosen still lets one be chosen).
    pub fn offer(&mut self, doc: &Document) {
        let tree = doc.layers();
        self.layers = tree
            .leaves()
            .iter()
            .filter(|n| n.service.is_none() && !tree.is_locked(&n.id))
            .map(|n| (n.id.clone(), tree.path(&n.id)))
            .collect();
        if self.on
            && let Some(id) = &self.layer
            && tree.get(id).is_some()
        {
            self.target = Some(id.clone());
            self.names = layer_names(doc, id);
        }
    }

    /// The file's tables a plan read: a chosen layer takes them all ([`Self::layers_map`]).
    pub fn learn_tables(&mut self, tables: impl IntoIterator<Item = String>) {
        self.tables = tables.into_iter().collect();
    }

    /// The keys offered: the target's fields, else its objects' attribute names.
    #[cfg(test)]
    pub fn view_names(&self) -> &[String] {
        &self.names
    }

    /// The layers offered for the rows: id and path, in the tree's order.
    #[cfg(test)]
    pub fn layer_choices(&self) -> &[(String, String)] {
        &self.layers
    }

    /// The command's target layer for a file of one table; none while the section is off or none is chosen.
    pub fn layer(&self) -> Option<String> {
        self.on.then(|| self.layer.clone()).flatten()
    }

    /// The command's table → layer for a file of tables: every table the last plan read onto the chosen layer.
    pub fn layers_map(&self) -> BTreeMap<String, String> {
        self.layer()
            .map(|id| {
                self.tables
                    .iter()
                    .map(|t| (t.clone(), id.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The command's `merge`; none while the section is off.
    pub fn input(&self) -> Option<ImportMerge> {
        self.on.then(|| ImportMerge {
            key: self.key.clone(),
            target_key: self.target_key.clone(),
            matched: self.matched,
            geometry: self.geometry,
            unmatched: self.unmatched,
            duplicates: self.duplicates,
        })
    }

    /// The command's `rejects`: `<file>-reddedilenler.csv` beside the file.
    pub fn rejects(&self) -> Option<String> {
        (self.on && self.rejects).then(|| self.rejects_path().display().to_string())
    }

    pub fn rejects_path(&self) -> PathBuf {
        let stem = self
            .source
            .file_stem()
            .map_or_else(|| "dosya".to_owned(), |s| s.to_string_lossy().into_owned());
        self.source
            .with_file_name(format!("{stem}-reddedilenler.csv"))
    }

    /// The choice `e` makes.
    pub fn apply(&mut self, e: MergeEvent) {
        let names = &self.names;
        match e {
            MergeEvent::On => self.on = !self.on,
            MergeEvent::Layer(i) => {
                let layer = i
                    .checked_sub(1)
                    .and_then(|k| self.layers.get(k))
                    .map(|(id, _)| id.clone());
                if layer != self.layer {
                    // Another layer's fields: the keys are asked again, the target is the next plan's.
                    self.key = None;
                    self.target_key = None;
                    self.target = None;
                    self.names.clear();
                }
                self.layer = layer;
            }
            MergeEvent::Key(i) => {
                let at = if self.ids { i.checked_sub(1) } else { Some(i) };
                self.key = at.and_then(|k| names.get(k).cloned());
            }
            MergeEvent::TargetKey(i) => {
                self.target_key = i.checked_sub(1).and_then(|k| names.get(k).cloned())
            }
            MergeEvent::Matched(m) => self.matched = m.0,
            MergeEvent::Geometry => self.geometry = !self.geometry,
            MergeEvent::Unmatched(u) => self.unmatched = u.0,
            MergeEvent::Duplicates(d) => self.duplicates = d.0,
            MergeEvent::Rejects => self.rejects = !self.rejects,
        }
    }

    /// The section: the switch, and while on its choices.
    pub fn view<'a>(
        &'a self,
        to: impl Fn(MergeEvent) -> Message + Clone + 'a,
    ) -> Element<'a, Message> {
        let names = &self.names;
        let target = self.target.as_deref();
        let mut body = Column::new().spacing(10).push(words::check(
            self.on,
            "Var olan katmanla birleştir",
            Some(to(MergeEvent::On)),
        ));
        if !self.on {
            return body.into();
        }
        let layers = std::iter::once(Choice::new("Adı eşleşen katman".to_owned())).chain(
            self.layers
                .iter()
                .map(|(_, path)| Choice::new(path.clone())),
        );
        let layer_at = match &self.layer {
            None => Some(0),
            Some(id) => self.layers.iter().position(|(l, _)| l == id).map(|i| i + 1),
        };
        let on_layer = to.clone();
        body = body.push(words::field(
            "Katman",
            Select::new(layers, layer_at, move |i| on_layer(MergeEvent::Layer(i))),
            target.is_none().then(|| {
                "Dosyadaki adıyla eşleşen katman yok; satırlar yeni katmana gider. Birleştirmek için bir katman seçin."
                    .to_owned()
            }),
        ));
        let Some(target) = target else {
            return body.into();
        };
        let mut keys: Vec<Choice> = Vec::new();
        if self.ids {
            keys.push(Choice::new("Kalıcı kimlik".to_owned()));
        }
        keys.extend(names.iter().map(|n| Choice::new(n.clone())));
        let key_at = match &self.key {
            None if self.ids => Some(0),
            None => None,
            Some(k) => names
                .iter()
                .position(|n| n == k)
                .map(|i| i + usize::from(self.ids)),
        };
        let on_key = to.clone();
        body = body.push(words::field(
            "Anahtar",
            Select::new(keys, key_at, move |i| on_key(MergeEvent::Key(i))).placeholder("Sütun seçin"),
            Some(format!(
                "Satırlar “{}” katmanının nesneleriyle bu değerle eşleşir; sayılar sayı olarak (007 ve 7 aynı).",
                // The layer by its path, as the Katman list says it.
                self.layers
                    .iter()
                    .find(|(id, _)| id == target)
                    .map_or(target, |(_, path)| path.as_str())
            )),
        ));
        if self.key.is_some() {
            let targets = std::iter::once(Choice::new("Anahtarla aynı ad".to_owned()))
                .chain(names.iter().map(|n| Choice::new(n.clone())));
            let at = match &self.target_key {
                None => Some(0),
                Some(k) => names.iter().position(|n| n == k).map(|i| i + 1),
            };
            let on_target = to.clone();
            body = body.push(words::field(
                "Katmandaki alan",
                Select::new(targets, at, move |i| on_target(MergeEvent::TargetKey(i))),
                None,
            ));
        }
        let on_matched = to.clone();
        body = body.push(words::field(
            "Eşleşen nesne",
            Segmented::new(
                [
                    MergeMatched::Update,
                    MergeMatched::FillEmpty,
                    MergeMatched::Keep,
                ]
                .map(Matched),
                Matched(self.matched),
                move |m| on_matched(MergeEvent::Matched(m)),
            ),
            Some(
                match self.matched {
                    MergeMatched::Update => {
                        "Öznitelikler dosyanınkiler olur; dosyada boş hücre nesnedekini boşaltır."
                    }
                    MergeMatched::FillEmpty => {
                        "Yalnız nesnede boş olan öznitelikler dosyadan dolar."
                    }
                    MergeMatched::Keep => "Eşleşen nesneye dokunulmaz.",
                }
                .to_owned(),
            ),
        ));
        if self.matched != MergeMatched::Keep {
            body = body.push(words::check(
                self.geometry,
                "Geometriyi de güncelle",
                Some(to(MergeEvent::Geometry)),
            ));
        }
        let on_unmatched = to.clone();
        body = body.push(words::field(
            "Eşleşmeyen satır",
            Segmented::new(
                [MergeUnmatched::Add, MergeUnmatched::Skip].map(Unmatched),
                Unmatched(self.unmatched),
                move |u| on_unmatched(MergeEvent::Unmatched(u)),
            ),
            None,
        ));
        let on_duplicates = to.clone();
        body = body.push(words::field(
            "Dosyada yinelenen anahtar",
            Segmented::new(
                [
                    MergeDuplicates::Stop,
                    MergeDuplicates::First,
                    MergeDuplicates::None,
                ]
                .map(Duplicates),
                Duplicates(self.duplicates),
                move |d| on_duplicates(MergeEvent::Duplicates(d)),
            ),
            None,
        ));
        let file = self
            .rejects_path()
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        body = body.push(words::check(
            self.rejects,
            format!("Reddedilen satırları dosyaya yaz ({file})"),
            Some(to(MergeEvent::Rejects)),
        ));
        body.into()
    }
}

/// A plan's first line while it merges: the rows meet a layer's objects (its counts follow in [`report_lines`]).
pub const MERGE_LINE: &str =
    "Satırlar var olan katmanla birleştirilecek; tek adımda, Ctrl+Z geri alır.";

/// Whether a warning says what the merges' lines already say (the rows refused): a window shows it once.
pub fn said_by_merge(w: &CommandWarning, merged: &[MergeReport]) -> bool {
    !merged.is_empty() && w.code == kentos_commands::codes::ROWS_REJECTED
}

/// The plan's merges as summary lines: per layer (by its path, `path`) what is added, updated, left and refused, and
/// why.
pub fn report_lines<'a>(
    merged: &[MergeReport],
    path: impl Fn(&str) -> String,
) -> Vec<Element<'a, Message>> {
    merged
        .iter()
        .map(|r| {
            let mut parts = vec![
                format!("{} eklenecek", r.added),
                format!("{} güncellenecek", r.updated),
            ];
            if r.unchanged > 0 {
                parts.push(format!("{} değişmeyecek", r.unchanged));
            }
            if r.skipped > 0 {
                parts.push(format!("{} alınmayacak", r.skipped));
            }
            if r.rejected > 0 {
                let why: Vec<String> = r
                    .reasons
                    .iter()
                    .map(|(k, n)| format!("{n} satırın {}", kentos_commands::reject_words(k)))
                    .collect();
                parts.push(format!("{} reddedilecek: {}", r.rejected, why.join(", ")));
            }
            words::text_line(
                if r.rejected > 0 { Line::Warn } else { Line::Ok },
                format!(
                    "“{}” → “{}”: {}.",
                    r.table,
                    path(&r.layer),
                    parts.join(", ")
                ),
            )
        })
        .collect()
}

/// The names a layer's objects are keyed by: its fields, else its objects' attribute names (sorted, at most 64).
pub fn layer_names(doc: &Document, layer: &str) -> Vec<String> {
    if let Some(node) = doc.layers().get(layer)
        && !node.fields.is_empty()
    {
        return node.fields.iter().map(|f| f.name.clone()).collect();
    }
    let mut names = std::collections::BTreeSet::new();
    for (_, slot) in doc.layer_slots(layer) {
        if let Some(e) = doc.get(slot) {
            names.extend(e.base().attrs.keys().cloned());
        }
        if names.len() >= 64 {
            break;
        }
    }
    names.into_iter().take(64).collect()
}

/// What an import says when done: the objects it added, and with a merge those it changed and the rows it refused.
pub fn taken_words(file: &str, objects: u32, merged: &[MergeReport]) -> String {
    if merged.is_empty() {
        return format!("“{file}” alındı: {objects} nesne; Ctrl+Z geri alır.");
    }
    let updated: u32 = merged.iter().map(|r| r.updated).sum();
    let rejected: u32 = merged.iter().map(|r| r.rejected).sum();
    let mut parts = vec![
        format!("{objects} nesne eklendi"),
        format!("{updated} nesne güncellendi"),
    ];
    if rejected > 0 {
        parts.push(format!("{rejected} satır reddedildi"));
    }
    format!(
        "“{file}” birleştirildi: {}; Ctrl+Z geri alır.",
        parts.join(", ")
    )
}
