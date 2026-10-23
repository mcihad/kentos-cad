//! Kaynaklar's view (the web's `SourcesPanel`): Klasör ekle over one list of
//! rows: the two sections, the folders and their files, the KentOS lists,
//! projects and their layers; a row opens and closes with its caret, a file
//! or a layer carries what it is and its Katman olarak ekle button, a double
//! click adds it too, and its menu. The source browser (docs/adr/0599): Son kullanılanlar and Sık kullanılanlar
//! first, a GeoPackage's tables and a table's preview, Veritabanları and Servisler, the search box over the rows; a
//! file, table, layer or basemap row pressed and let go over the drawing adds it there.

use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, mouse_area, row, space, text, text_input, tooltip};
use iced::{Center, Element, Fill, Length, Theme};
use kentos_commands::{LayerNode, LayerNodeType, ProjectStorage, ProjectSummary, SourceTable};
use kentos_interaction::sources::{
    SourceFile, SourceKind, search_matches, search_shown, search_words,
};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{ContextMenu, Menu, Tip, VirtualList, tip};
use kentos_ui::{label, style};

use super::{
    DATABASES, Event, FAVOURITES, FOLDERS, KENTOS, Kept, LIMIT, LISTS, Origin, PRESETS,
    ProjectDrawing, RECENT, Read, SAVED, SERVICES, SavedService, folder_key, folder_name, list_key,
    msg, project_key, service_key, table_key, tables_key,
};
use crate::app::{App, Message};
use crate::icons::from_web;

/// A row's height at the body text's size (the tree's).
const ROW: f32 = 28.0;
/// A level's indent.
const INDENT: f32 = 14.0;

/// A listed row.
pub(crate) enum Line {
    Section {
        key: &'static str,
        label: &'static str,
        glyph: &'static str,
        open: bool,
    },
    Folder {
        key: String,
        path: PathBuf,
        label: String,
        root: bool,
        depth: usize,
        open: bool,
    },
    File {
        key: String,
        folder: PathBuf,
        file: SourceFile,
        depth: usize,
        /// A GeoPackage opens into its tables (docs/adr/0599 §2); none for a file that does not open.
        open: Option<bool>,
    },
    /// A table of a GeoPackage or of a database, and the layer it is bound to (its id and name).
    Table {
        key: String,
        origin: Origin,
        table: Box<SourceTable>,
        bound: Option<(String, String)>,
        depth: usize,
        open: bool,
    },
    /// A table's declared box, with Kapsamı göster.
    Extent {
        key: String,
        srid: Option<u32>,
        extent: [f64; 4],
        name: String,
        depth: usize,
    },
    Database {
        key: String,
        address: String,
        depth: usize,
        open: bool,
    },
    /// Veritabanı ekle's address being typed.
    DatabaseInput { key: String, draft: String },
    /// A group that starts closed (Hazır altlıklar) or open (Kayıtlı servisler), by the open set.
    Branch {
        key: String,
        label: &'static str,
        glyph: &'static str,
        depth: usize,
        open: bool,
    },
    Preset {
        key: String,
        command: String,
        name: String,
        group: String,
        glyph: String,
        depth: usize,
    },
    Service {
        key: String,
        at: usize,
        service: SavedService,
        depth: usize,
        open: bool,
    },
    ServiceLayer {
        key: String,
        service: SavedService,
        item: String,
        title: String,
        depth: usize,
    },
    /// A recent or favourite source.
    Kept {
        key: String,
        kept: Kept,
        favourite: bool,
        depth: usize,
    },
    List {
        key: String,
        label: &'static str,
        mine: bool,
        depth: usize,
        open: bool,
    },
    Project {
        key: String,
        project: Box<ProjectSummary>,
        depth: usize,
        open: bool,
    },
    Group {
        key: String,
        label: String,
        depth: usize,
        open: bool,
    },
    Layer {
        key: String,
        project: String,
        project_name: String,
        path: String,
        label: String,
        count: usize,
        depth: usize,
    },
    Note {
        key: String,
        label: String,
        warn: bool,
        glyph: Option<&'static str>,
        depth: usize,
        act: Option<Message>,
    },
}

impl Line {
    pub(crate) fn key(&self) -> &str {
        match self {
            Line::Section { key, .. } => key,
            Line::Folder { key, .. }
            | Line::File { key, .. }
            | Line::Table { key, .. }
            | Line::Extent { key, .. }
            | Line::Database { key, .. }
            | Line::DatabaseInput { key, .. }
            | Line::Branch { key, .. }
            | Line::Preset { key, .. }
            | Line::Service { key, .. }
            | Line::ServiceLayer { key, .. }
            | Line::Kept { key, .. }
            | Line::List { key, .. }
            | Line::Project { key, .. }
            | Line::Group { key, .. }
            | Line::Layer { key, .. }
            | Line::Note { key, .. } => key,
        }
    }

    /// How deep it is: the sections at 0.
    pub(crate) fn depth(&self) -> usize {
        match self {
            Line::Section { .. } => 0,
            Line::DatabaseInput { .. } => 1,
            Line::Folder { depth, .. }
            | Line::File { depth, .. }
            | Line::Table { depth, .. }
            | Line::Extent { depth, .. }
            | Line::Database { depth, .. }
            | Line::Branch { depth, .. }
            | Line::Preset { depth, .. }
            | Line::Service { depth, .. }
            | Line::ServiceLayer { depth, .. }
            | Line::Kept { depth, .. }
            | Line::List { depth, .. }
            | Line::Project { depth, .. }
            | Line::Group { depth, .. }
            | Line::Layer { depth, .. }
            | Line::Note { depth, .. } => *depth,
        }
    }

    /// The words the search looks in (docs/adr/0599 §5): a note and the preview's rows none.
    pub(crate) fn fields(&self) -> Vec<String> {
        match self {
            Line::Section { label, .. } | Line::Branch { label, .. } | Line::List { label, .. } => {
                vec![(*label).to_owned()]
            }
            Line::Folder { label, path, .. } => vec![label.clone(), path.display().to_string()],
            Line::File { folder, file, .. } => vec![
                file.name.clone(),
                file.kind.label().to_owned(),
                folder.join(&file.name).display().to_string(),
            ],
            Line::Table {
                origin,
                table,
                bound,
                ..
            } => {
                let mut out = vec![table.name.clone()];
                out.extend(table.geometry.clone());
                out.push(geometry_word(table.geometry.as_deref()).to_owned());
                out.extend(table.srid.map(|c| format!("EPSG:{c}")));
                out.extend(bound.as_ref().map(|(_, name)| format!("Bağlı: {name}")));
                if let Origin::Database(_) = origin
                    && let Some((schema, _)) = table.name.split_once('.')
                {
                    out.push(schema.to_owned());
                }
                out.push(origin.path());
                out
            }
            Line::Database { address, .. } => vec![address.clone()],
            Line::Preset { name, group, .. } => {
                vec![name.clone(), group.clone(), "Hazır altlık".to_owned()]
            }
            Line::Service { service, .. } => vec![
                service.name.clone(),
                service.kind.label().to_owned(),
                service.url.clone(),
            ],
            Line::ServiceLayer {
                service,
                item,
                title,
                ..
            } => vec![title.clone(), item.clone(), service.kind.label().to_owned()],
            Line::Kept { kept, .. } => kept.fields(),
            Line::Project { project, .. } => vec![project.name.clone()],
            Line::Group { label, .. } => vec![label.clone()],
            Line::Layer {
                label,
                path,
                project_name,
                ..
            } => vec![label.clone(), path.clone(), project_name.clone()],
            Line::Extent { .. } | Line::DatabaseInput { .. } | Line::Note { .. } => Vec::new(),
        }
    }
}

fn note(key: String, label: impl Into<String>, warn: bool, depth: usize) -> Line {
    Line::Note {
        key,
        label: label.into(),
        warn,
        glyph: None,
        depth,
        act: None,
    }
}

impl App {
    /// The rows as listed now: those the search keeps when it has words (docs/adr/0599 §5).
    pub(crate) fn source_lines(&self) -> Vec<Line> {
        let lines = self.all_source_lines();
        let words = search_words(&self.sources.search);
        if words.is_empty() {
            return lines;
        }
        let depths: Vec<usize> = lines.iter().map(Line::depth).collect();
        let matched: Vec<bool> = lines
            .iter()
            .map(|l| {
                let fields = l.fields();
                !fields.is_empty()
                    && search_matches(
                        &words,
                        &fields.iter().map(String::as_str).collect::<Vec<_>>(),
                    )
            })
            .collect();
        let found = matched.iter().filter(|m| **m).count();
        let shown = search_shown(&depths, &matched);
        let mut out = vec![note(
            "arama:not".to_owned(),
            if found == 0 {
                "Eşleşen satır yok. Yalnız açılmış satırlar aranır.".to_owned()
            } else {
                format!("{found} satır eşleşti. Yalnız açılmış satırlar aranır.")
            },
            false,
            0,
        )];
        out.extend(
            lines
                .into_iter()
                .zip(shown)
                .filter_map(|(l, keep)| keep.then_some(l)),
        );
        out
    }

    fn all_source_lines(&self) -> Vec<Line> {
        let s = &self.sources;
        let mut lines = Vec::new();
        self.kept_lines(
            RECENT,
            "Son kullanılanlar",
            "history",
            s.folders.recent(),
            &mut lines,
        );
        self.kept_lines(
            FAVOURITES,
            "Sık kullanılanlar",
            "star",
            s.folders.favourites(),
            &mut lines,
        );
        let open = s.open.contains(FOLDERS);
        lines.push(Line::Section {
            key: FOLDERS,
            label: "Klasörler",
            glyph: "folder",
            open,
        });
        if open {
            if s.folders.list().is_empty() {
                lines.push(Line::Note {
                    key: format!("{FOLDERS}:yok"),
                    label: "Klasör yok. Klasör ekle ile bir klasör ekleyin.".to_owned(),
                    warn: false,
                    glyph: Some("folderAdd"),
                    depth: 1,
                    act: Some(msg(Event::AddFolder)),
                });
            }
            for path in s.folders.list() {
                self.folder_lines(path, folder_name(path), true, 1, &mut lines);
            }
        }
        self.database_lines(&mut lines);
        self.service_lines(&mut lines);
        let open = s.open.contains(KENTOS);
        lines.push(Line::Section {
            key: KENTOS,
            label: "KentOS",
            glyph: "cloud",
            open,
        });
        if open {
            if self.cloud.signed_in().is_none() {
                lines.push(Line::Note {
                    key: format!("{KENTOS}:giris"),
                    label: "Projeleri görmek için giriş yapın.".to_owned(),
                    warn: false,
                    glyph: Some("signIn"),
                    depth: 1,
                    act: Some(Message::Run("cloud.signIn")),
                });
            } else {
                for (_, view, label) in LISTS {
                    self.list_lines(view, label, &mut lines);
                }
            }
        }
        lines
    }

    fn folder_lines(
        &self,
        path: &Path,
        label: String,
        root: bool,
        depth: usize,
        lines: &mut Vec<Line>,
    ) {
        let key = folder_key(path);
        let open = self.sources.open.contains(&key);
        lines.push(Line::Folder {
            key: key.clone(),
            path: path.to_path_buf(),
            label,
            root,
            depth,
            open,
        });
        if !open {
            return;
        }
        match self.sources.listings.get(path) {
            None | Some(Read::Reading) => {
                lines.push(note(
                    format!("{key}\0okunuyor"),
                    "Okunuyor…",
                    false,
                    depth + 1,
                ));
            }
            Some(Read::Failed(why)) => {
                lines.push(note(format!("{key}\0hata"), why.clone(), true, depth + 1));
            }
            Some(Read::Ready(listing)) => {
                for name in &listing.folders {
                    self.folder_lines(&path.join(name), name.clone(), false, depth + 1, lines);
                }
                for file in &listing.files {
                    if file.kind == SourceKind::GeoPackage {
                        let origin = Origin::File(path.join(&file.name));
                        let key = tables_key(&origin);
                        let open = self.sources.open.contains(&key);
                        lines.push(Line::File {
                            key,
                            folder: path.to_path_buf(),
                            file: file.clone(),
                            depth: depth + 1,
                            open: Some(open),
                        });
                        if open {
                            self.table_lines(&origin, depth + 2, lines);
                        }
                        continue;
                    }
                    lines.push(Line::File {
                        key: format!("f:{}", path.join(&file.name).display()),
                        folder: path.to_path_buf(),
                        file: file.clone(),
                        depth: depth + 1,
                        open: None,
                    });
                }
                if listing.folders.is_empty() && listing.files.is_empty() {
                    lines.push(note(
                        format!("{key}\0boş"),
                        "Eklenebilecek dosya yok.",
                        false,
                        depth + 1,
                    ));
                }
            }
        }
    }

    /// Son kullanılanlar or Sık kullanılanlar (docs/adr/0599 §6): not shown while empty.
    fn kept_lines(
        &self,
        key: &'static str,
        label: &'static str,
        glyph: &'static str,
        kept: &[Kept],
        lines: &mut Vec<Line>,
    ) {
        if kept.is_empty() {
            return;
        }
        let open = self.sources.open.contains(key);
        lines.push(Line::Section {
            key,
            label,
            glyph,
            open,
        });
        if !open {
            return;
        }
        for (i, k) in kept.iter().enumerate() {
            lines.push(Line::Kept {
                key: format!("{key}:{i}"),
                kept: k.clone(),
                favourite: self.sources.folders.is_favourite(k),
                depth: 1,
            });
        }
    }

    /// A GeoPackage's or a database's tables under it, a table's preview under the table (docs/adr/0599 §2, §3).
    fn table_lines(&self, origin: &Origin, depth: usize, lines: &mut Vec<Line>) {
        let key = tables_key(origin);
        match self.sources.tables.get(origin) {
            None | Some(Read::Reading) => {
                lines.push(note(format!("{key}\0okunuyor"), "Okunuyor…", false, depth));
            }
            Some(Read::Failed(why)) => {
                lines.push(note(format!("{key}\0hata"), why.clone(), true, depth))
            }
            Some(Read::Ready(tables)) => {
                if tables.is_empty() {
                    lines.push(note(
                        format!("{key}\0boş"),
                        "Okunabilen tablo yok.",
                        false,
                        depth,
                    ));
                }
                let bound = self.bound_tables(origin);
                let mut schema: Option<&str> = None;
                let mut shown = true;
                for t in tables {
                    let mut at = depth;
                    if let Origin::Database(address) = origin {
                        let here = t.name.split_once('.').map_or("public", |(s, _)| s);
                        let group = format!("c:{address}\0{here}");
                        if schema != Some(here) {
                            schema = Some(here);
                            shown = !self.sources.open.contains(&format!("{group}\0kapalı"));
                            lines.push(Line::Group {
                                key: group,
                                label: here.to_owned(),
                                depth,
                                open: shown,
                            });
                        }
                        if !shown {
                            continue;
                        }
                        at = depth + 1;
                    }
                    let tkey = table_key(origin, &t.name);
                    let open = self.sources.open.contains(&tkey);
                    lines.push(Line::Table {
                        key: tkey.clone(),
                        origin: origin.clone(),
                        table: Box::new(t.clone()),
                        bound: bound
                            .iter()
                            .find(|(name, ..)| *name == t.name)
                            .map(|(_, id, l)| (id.clone(), l.clone())),
                        depth: at,
                        open,
                    });
                    if open {
                        self.preview_lines(&tkey, t, at + 1, lines);
                    }
                }
            }
        }
    }

    /// The tables of `origin` bound in the open drawing: their names, the layers' ids and names.
    fn bound_tables(&self, origin: &Origin) -> Vec<(String, String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let path = origin.path();
        doc.settings()
            .sources
            .iter()
            .filter(|l| {
                l.path == path || matches!(origin, Origin::File(p) if Path::new(&l.path) == p)
            })
            .filter_map(|l| {
                let layer = doc.model.layers().get(&l.layer)?;
                Some((l.table.clone(), l.layer.clone(), layer.name.clone()))
            })
            .collect()
    }

    fn preview_lines(&self, key: &str, t: &SourceTable, depth: usize, lines: &mut Vec<Line>) {
        if let Some(extent) = t.extent {
            lines.push(Line::Extent {
                key: format!("{key}\0kapsam"),
                srid: t.srid,
                extent,
                name: t.name.clone(),
                depth,
            });
        }
        match self.sources.samples.get(key) {
            None | Some(Read::Reading) => {
                lines.push(note(
                    format!("{key}\0okunuyor"),
                    "Örnek satırlar okunuyor…",
                    false,
                    depth,
                ));
            }
            Some(Read::Failed(why)) => {
                lines.push(note(format!("{key}\0hata"), why.clone(), true, depth))
            }
            Some(Read::Ready(rows)) => {
                if rows.is_empty() {
                    lines.push(note(
                        format!("{key}\0boş"),
                        "Tabloda kayıt yok.",
                        false,
                        depth,
                    ));
                }
                for (i, words) in rows.iter().enumerate() {
                    lines.push(note(
                        format!("{key}\0satır{i}"),
                        words.clone(),
                        false,
                        depth,
                    ));
                }
            }
        }
    }

    /// Veritabanları (docs/adr/0599 §3).
    fn database_lines(&self, lines: &mut Vec<Line>) {
        let s = &self.sources;
        let open = s.open.contains(DATABASES);
        lines.push(Line::Section {
            key: DATABASES,
            label: "Veritabanları",
            glyph: "sourceDatabase",
            open,
        });
        if !open {
            return;
        }
        if let Some(draft) = &s.database_draft {
            lines.push(Line::DatabaseInput {
                key: format!("{DATABASES}:yeni"),
                draft: draft.clone(),
            });
        } else if s.folders.databases().is_empty() {
            lines.push(Line::Note {
                key: format!("{DATABASES}:yok"),
                label: "Veritabanı yok. Veritabanı ekle ile bir PostGIS adresi ekleyin.".to_owned(),
                warn: false,
                glyph: Some("sourceDatabase"),
                depth: 1,
                act: Some(msg(Event::DatabaseAdd)),
            });
        }
        for address in s.folders.databases() {
            let origin = Origin::Database(address.clone());
            let key = tables_key(&origin);
            let open = s.open.contains(&key);
            lines.push(Line::Database {
                key,
                address: address.clone(),
                depth: 1,
                open,
            });
            if open {
                self.table_lines(&origin, 2, lines);
            }
        }
    }

    /// Servisler (docs/adr/0599 §4): the ready basemaps, then the services added before.
    fn service_lines(&self, lines: &mut Vec<Line>) {
        let s = &self.sources;
        let open = s.open.contains(SERVICES);
        lines.push(Line::Section {
            key: SERVICES,
            label: "Servisler",
            glyph: "serviceAdd",
            open,
        });
        if !open {
            return;
        }
        let presets_open = s.open.contains(PRESETS);
        lines.push(Line::Branch {
            key: PRESETS.to_owned(),
            label: "Hazır altlıklar",
            glyph: "basemap",
            depth: 1,
            open: presets_open,
        });
        if presets_open {
            for p in &kentos_services::presets::catalog().presets {
                lines.push(Line::Preset {
                    key: format!("{PRESETS}:{}", p.id),
                    command: kentos_services::presets::command_of(&p.id),
                    name: p.name.clone(),
                    group: p.group.clone(),
                    glyph: p.icon.clone(),
                    depth: 2,
                });
            }
        }
        let saved_open = s.open.contains(SAVED);
        lines.push(Line::Branch {
            key: SAVED.to_owned(),
            label: "Kayıtlı servisler",
            glyph: "serviceSettings",
            depth: 1,
            open: saved_open,
        });
        if !saved_open {
            return;
        }
        if s.folders.services().is_empty() {
            lines.push(Line::Note {
                key: format!("{SAVED}:yok"),
                label: "Kayıtlı servis yok. Harita servisi ile eklenen servisler burada kalır."
                    .to_owned(),
                warn: false,
                glyph: Some("serviceAdd"),
                depth: 2,
                act: Some(Message::Run("service.add")),
            });
        }
        for (at, service) in s.folders.services().iter().enumerate() {
            let key = service_key(service);
            let open = s.open.contains(&key);
            lines.push(Line::Service {
                key: key.clone(),
                at,
                service: service.clone(),
                depth: 2,
                open,
            });
            if !open {
                continue;
            }
            match s.services.get(&key) {
                None | Some(Read::Reading) => {
                    lines.push(note(
                        format!("{key}\0okunuyor"),
                        "Servis okunuyor…",
                        false,
                        3,
                    ));
                }
                Some(Read::Failed(why)) => {
                    lines.push(note(format!("{key}\0hata"), why.clone(), true, 3))
                }
                Some(Read::Ready(c)) => {
                    let items: Vec<_> = c
                        .offer()
                        .map(|o| o.items.iter().filter(|i| i.pickable).collect())
                        .unwrap_or_default();
                    if items.is_empty() {
                        lines.push(note(
                            format!("{key}\0boş"),
                            "Seçilebilen katman yok.",
                            false,
                            3,
                        ));
                    }
                    for item in items {
                        lines.push(Line::ServiceLayer {
                            key: format!("{key}\0{}", item.id),
                            service: service.clone(),
                            item: item.id.clone(),
                            title: if item.title.is_empty() {
                                item.id.clone()
                            } else {
                                item.title.clone()
                            },
                            depth: 3,
                        });
                    }
                }
            }
        }
    }

    fn list_lines(&self, view: &'static str, label: &'static str, lines: &mut Vec<Line>) {
        let key = list_key(view);
        let open = self.sources.open.contains(&key);
        lines.push(Line::List {
            key: key.clone(),
            label,
            mine: view == "mine",
            depth: 1,
            open,
        });
        if !open {
            return;
        }
        match self.sources.lists.get(view) {
            None | Some(Read::Reading) => {
                lines.push(note(format!("{key}\0okunuyor"), "Okunuyor…", false, 2));
            }
            Some(Read::Failed(why)) => {
                lines.push(note(format!("{key}\0hata"), why.clone(), true, 2))
            }
            Some(Read::Ready((projects, total))) => {
                for p in projects {
                    self.project_lines(p, lines);
                }
                if projects.is_empty() {
                    lines.push(note(format!("{key}\0boş"), "Proje yok.", false, 2));
                }
                if *total > projects.len() as u64 {
                    lines.push(note(
                        format!("{key}\0fazla"),
                        format!("İlk {LIMIT} proje gösteriliyor (toplam {total})."),
                        false,
                        2,
                    ));
                }
            }
        }
    }

    fn project_lines(&self, p: &ProjectSummary, lines: &mut Vec<Line>) {
        let key = project_key(&p.id);
        let open = self.sources.open.contains(&key);
        lines.push(Line::Project {
            key: key.clone(),
            project: Box::new(p.clone()),
            depth: 2,
            open,
        });
        if !open {
            return;
        }
        match self.sources.drawings.get(&p.id) {
            None | Some(Read::Reading) => {
                lines.push(note(
                    format!("{key}\0iniyor"),
                    "Çizimi indiriliyor…",
                    false,
                    3,
                ));
            }
            Some(Read::Failed(why)) => {
                lines.push(note(format!("{key}\0hata"), why.clone(), true, 3))
            }
            Some(Read::Ready(d)) => {
                let before = lines.len();
                self.layer_lines(p, d, &d.drawing.layers, &[], 3, lines);
                if lines.len() == before {
                    lines.push(note(format!("{key}\0boş"), "Projede katman yok.", false, 3));
                }
            }
        }
    }

    fn layer_lines(
        &self,
        p: &ProjectSummary,
        d: &ProjectDrawing,
        nodes: &[LayerNode],
        above: &[String],
        depth: usize,
        lines: &mut Vec<Line>,
    ) {
        for n in nodes {
            let mut here = above.to_vec();
            here.push(n.name.clone());
            let key = format!("y:{}\0{}", p.id, here.join("\0"));
            if n.kind == LayerNodeType::Layer {
                lines.push(Line::Layer {
                    key,
                    project: p.id.clone(),
                    project_name: p.name.clone(),
                    path: here.join(" / "),
                    label: n.name.clone(),
                    count: d.counts.get(&n.id).copied().unwrap_or(0),
                    depth,
                });
                continue;
            }
            // A group of the project's tree: open unless closed by hand.
            let open = !self.sources.open.contains(&format!("{key}\0kapalı"));
            lines.push(Line::Group {
                key: key.clone(),
                label: n.name.clone(),
                depth,
                open,
            });
            if open {
                self.layer_lines(p, d, &n.children, &here, depth + 1, lines);
            }
        }
    }

    /// The panel's body: Klasör ekle over the rows.
    pub(crate) fn sources_view(&self) -> Element<'_, Message> {
        let add = tip(
            button(
                row![
                    icon(from_web(Some("folderAdd"))).size(16.0),
                    label::body("Klasör ekle")
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press(msg(Event::AddFolder))
            .padding([4, 8])
            .style(style::button::ghost),
            Tip::new("Klasör ekle").body(
                "Bilgisayarınızdaki bir klasörü listeye ekler; içindeki GeoJSON, Shapefile, DXF, NCZ, GPX, NMEA ve koordinat listesi dosyaları katman olarak eklenebilir.",
            ),
            tooltip::Position::Bottom,
        );
        let database = tip(
            button(icon(from_web(Some("sourceDatabase"))).size(16.0))
                .on_press(msg(Event::DatabaseAdd))
                .padding([4, 6])
                .style(style::button::ghost),
            Tip::new("Veritabanı ekle").body(
                "Bir PostGIS veritabanının adresini Veritabanları'na ekler; tabloları ve view'ları açılınca listelenir.",
            ),
            tooltip::Position::Bottom,
        );
        let service = tip(
            button(icon(from_web(Some("serviceAdd"))).size(16.0))
                .on_press(Message::Run("service.add"))
                .padding([4, 6])
                .style(style::button::ghost),
            Tip::new("Servis ekle").body(
                "Harita servisi penceresini açar; eklenen servis Servisler'in Kayıtlı servisler'inde kalır.",
            ),
            tooltip::Position::Bottom,
        );
        let search = text_input("Kaynaklarda ara: ad, tür, EPSG, şema", &self.sources.search)
            .on_input(|t| msg(Event::Search(t)))
            .size(typography::body())
            .padding([4, 8])
            .width(Fill);
        let bar = container(
            column![
                row![add, space::horizontal(), database, service].align_y(Center),
                search
            ]
            .spacing(4),
        )
        .padding([4, 6])
        .width(Fill);
        let lines = self.source_lines();
        let focused = self.sources.focused.clone();
        let list = VirtualList::new(lines.len(), typography::scaled(ROW), move |i| {
            let line = &lines[i];
            let chosen = focused.as_deref() == Some(line.key());
            self.source_row(line, chosen)
        })
        .height(Fill);
        column![bar, list].into()
    }

    fn source_row<'a>(&'a self, line: &Line, chosen: bool) -> Element<'a, Message> {
        let caret = |open: bool| {
            icon(if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            })
            .size(12.0)
        };
        let glyph = |name: &str| icon(from_web(Some(name))).size(15.0);
        let name = |words: String, strong: bool| {
            text(words)
                .font(if strong {
                    typography::ui_strong()
                } else {
                    typography::ui()
                })
                .size(typography::body())
                .width(Fill)
                .wrapping(text::Wrapping::None)
        };
        let meta = |words: String| {
            label::caption(words)
                .wrapping(text::Wrapping::None)
                .style(|t: &Theme| text::Style {
                    color: Some(Tokens::of(t).faint),
                })
        };
        let indent = |depth: usize| space().width(Length::Fixed(depth as f32 * INDENT));
        let toggle = |key: &str| msg(Event::Toggle(key.to_owned()));
        match line {
            Line::Section {
                key,
                label,
                glyph: g,
                open,
            } => self.row_button(
                row![caret(*open), glyph(g), name((*label).to_owned(), true)],
                toggle(key),
                chosen,
            ),
            Line::Folder {
                key,
                path,
                label,
                root,
                depth,
                open,
            } => {
                let face = self.row_button(
                    row![
                        indent(*depth),
                        caret(*open),
                        glyph("folder"),
                        name(label.clone(), false)
                    ],
                    toggle(key),
                    chosen,
                );
                let face = tip(
                    face,
                    Tip::new(label.clone()).body(format!(
                        "{}\nKapatıp açınca yeniden okunur.",
                        path.display()
                    )),
                    tooltip::Position::Left,
                );
                if *root {
                    let path = path.clone();
                    ContextMenu::new(face, move |_| {
                        Menu::new()
                            .item("Listeden kaldır", msg(Event::Remove(path.clone())))
                            .icon(from_web(Some("trash")))
                            .detail("Klasör ve dosyaları silinmez; yalnız bu listeden çıkar.")
                    })
                    .into()
                } else {
                    face
                }
            }
            Line::File {
                key,
                folder,
                file,
                depth,
                open,
            } => {
                let add = msg(Event::AddFile(folder.clone(), file.clone()));
                let lead: Element<'a, Message> = match open {
                    Some(open) => caret(*open).into(),
                    None => space().width(Length::Fixed(12.0)).into(),
                };
                let click = match open {
                    Some(_) => toggle(key),
                    None => msg(Event::Focus(key.clone())),
                };
                let face = self.drag_row(
                    row![
                        indent(*depth),
                        lead,
                        glyph(file.kind.icon()),
                        name(file.name.clone(), false),
                        meta(file.kind.label().to_owned()),
                        add_button(add.clone()),
                    ],
                    key,
                    Some(click),
                    add.clone(),
                    chosen,
                );
                let kept = Kept::File {
                    path: folder.join(&file.name),
                };
                let favourite = self.sources.folders.is_favourite(&kept);
                let parts = if file.parts.len() > 1 {
                    format!(" ({})", file.parts.join(", "))
                } else {
                    String::new()
                };
                let face = tip(
                    face,
                    Tip::new(file.name.clone()).body(format!(
                        "{}{parts}\nÇift tık ya da Katman olarak ekle: içe aktarma penceresi açılır.",
                        file.kind.label()
                    )),
                    tooltip::Position::Left,
                );
                ContextMenu::new(face, move |_| {
                    star_item(
                        Menu::new()
                            .item("Katman olarak ekle…", add.clone())
                            .icon(from_web(Some("layerAdd")))
                            .shortcut("Enter"),
                        &kept,
                        favourite,
                    )
                })
                .into()
            }
            Line::Table {
                key,
                origin,
                table,
                bound,
                depth,
                open,
            } => self.table_row(key, origin, table, bound.as_ref(), *depth, *open, chosen),
            Line::Extent {
                key,
                srid,
                extent,
                name: table,
                depth,
            } => {
                let [x0, y0, x1, y1] = *extent;
                let system =
                    srid.map_or_else(|| "sistem bilinmiyor".to_owned(), |c| format!("EPSG:{c}"));
                let face = self.row_button(
                    row![
                        indent(*depth),
                        space().width(Length::Fixed(12.0)),
                        glyph("serviceExtent"),
                        name("Kapsamı göster".to_owned(), false),
                        meta(system.clone()),
                    ],
                    msg(Event::ShowExtent(*srid, *extent, table.clone())),
                    chosen,
                );
                let _ = key;
                tip(
                    face,
                    Tip::new("Kapsamı göster").body(format!(
                        "Kaynağın bildirdiği kutu ({system}): {x0:.2}, {y0:.2} – {x1:.2}, {y1:.2}.\nProjenin sistemine çevrilir, görünüm ona gelir ve çizimde kesikli bir çerçeveyle gösterilir; çerçeve kaydedilmez, İşareti kaldır siler."
                    )),
                    tooltip::Position::Left,
                )
            }
            Line::Database {
                key,
                address,
                depth,
                open,
            } => {
                let face = self.row_button(
                    row![
                        indent(*depth),
                        caret(*open),
                        glyph("sourceDatabase"),
                        name(super::database_name(address), false),
                        meta(super::database_host(address)),
                    ],
                    toggle(key),
                    chosen,
                );
                let face = tip(
                    face,
                    Tip::new(address.clone()).body(
                        "PostGIS veritabanı. Açılınca tabloları ve view'ları şemalarıyla listelenir; parola PGPASSWORD ya da ~/.pgpass'ten gelir.",
                    ),
                    tooltip::Position::Left,
                );
                let address = address.clone();
                ContextMenu::new(face, move |_| {
                    Menu::new()
                        .item(
                            "Listeden kaldır",
                            msg(Event::DatabaseRemove(address.clone())),
                        )
                        .icon(from_web(Some("trash")))
                        .detail("Veritabanına dokunulmaz; yalnız bu listeden çıkar.")
                })
                .into()
            }
            Line::DatabaseInput { draft, .. } => {
                let input = text_input("postgresql://kullanıcı@sunucu:5432/veritabanı", draft)
                    .on_input(|t| msg(Event::DatabaseDraft(t)))
                    .on_submit(msg(Event::DatabaseKeep))
                    .size(typography::body())
                    .padding([3, 6])
                    .width(Fill);
                container(
                    row![
                        indent(1),
                        input,
                        button(label::body("Ekle"))
                            .on_press(msg(Event::DatabaseKeep))
                            .padding([2, 8])
                            .style(style::button::primary),
                        button(label::body("Vazgeç"))
                            .on_press(msg(Event::DatabaseCancel))
                            .padding([2, 8])
                            .style(style::button::ghost),
                    ]
                    .spacing(4)
                    .padding([0, 8])
                    .align_y(Center)
                    .height(Fill),
                )
                .width(Fill)
                .height(Fill)
                .into()
            }
            Line::Branch {
                key,
                label: words,
                glyph: g,
                depth,
                open,
            } => self.row_button(
                row![
                    indent(*depth),
                    caret(*open),
                    glyph(g),
                    name((*words).to_owned(), false)
                ],
                toggle(key),
                chosen,
            ),
            Line::Preset {
                key,
                command,
                name: title,
                glyph: g,
                depth,
                ..
            } => {
                let kept = Kept::Basemap {
                    command: command.clone(),
                    name: title.clone(),
                };
                let add = msg(Event::Use(kept.clone()));
                let face = self.drag_row(
                    row![
                        indent(*depth),
                        space().width(Length::Fixed(12.0)),
                        glyph(g),
                        name(title.clone(), false),
                        add_button(add.clone()),
                    ],
                    key,
                    None,
                    add.clone(),
                    chosen,
                );
                let favourite = self.sources.folders.is_favourite(&kept);
                ContextMenu::new(face, move |_| {
                    star_item(
                        Menu::new()
                            .item("Altlık olarak ekle", add.clone())
                            .icon(from_web(Some("layerAdd"))),
                        &kept,
                        favourite,
                    )
                })
                .into()
            }
            Line::Service {
                key,
                at,
                service,
                depth,
                open,
            } => {
                let kept = Kept::Service {
                    service: service.clone(),
                };
                let add = msg(Event::Use(kept.clone()));
                let face = self.row_button(
                    row![
                        indent(*depth),
                        caret(*open),
                        glyph("serviceAdd"),
                        name(service.name.clone(), false),
                        meta(service.kind.label().to_owned()),
                        add_button(add.clone()),
                    ],
                    toggle(key),
                    chosen,
                );
                let face = tip(
                    face,
                    Tip::new(service.name.clone()).body(format!(
                        "{} · {}\nAçılınca katmanları listelenir.",
                        service.kind.label(),
                        service.url
                    )),
                    tooltip::Position::Left,
                );
                let favourite = self.sources.folders.is_favourite(&kept);
                let at = *at;
                ContextMenu::new(face, move |_| {
                    star_item(
                        Menu::new()
                            .item("Katman olarak ekle…", add.clone())
                            .icon(from_web(Some("layerAdd"))),
                        &kept,
                        favourite,
                    )
                    .item("Listeden kaldır", msg(Event::ServiceRemove(at)))
                    .icon(from_web(Some("trash")))
                    .detail("Servise ve çizimdeki katmanlarına dokunulmaz.")
                })
                .into()
            }
            Line::ServiceLayer {
                key,
                service,
                item,
                title,
                depth,
            } => {
                let kept = Kept::ServiceLayer {
                    service: service.clone(),
                    item: item.clone(),
                    title: title.clone(),
                };
                let add = msg(Event::Use(kept.clone()));
                let face = self.drag_row(
                    row![
                        indent(*depth),
                        space().width(Length::Fixed(12.0)),
                        glyph("layers"),
                        name(title.clone(), false),
                        add_button(add.clone()),
                    ],
                    key,
                    None,
                    add.clone(),
                    chosen,
                );
                let favourite = self.sources.folders.is_favourite(&kept);
                ContextMenu::new(face, move |_| {
                    star_item(
                        Menu::new()
                            .item("Katman olarak ekle…", add.clone())
                            .icon(from_web(Some("layerAdd"))),
                        &kept,
                        favourite,
                    )
                })
                .into()
            }
            Line::Kept {
                key,
                kept,
                favourite,
                depth,
            } => {
                let add = msg(Event::Use(kept.clone()));
                let missing = kept.missing();
                let said = if missing {
                    label::caption("bulunamadı")
                        .wrapping(text::Wrapping::None)
                        .style(|t: &Theme| text::Style {
                            color: Some(Tokens::of(t).warning),
                        })
                } else {
                    meta(kept.meta())
                };
                let face = self.drag_row(
                    row![
                        indent(*depth),
                        space().width(Length::Fixed(12.0)),
                        glyph(kept.glyph()),
                        name(kept.label(), false),
                        said,
                        add_button(add.clone()),
                    ],
                    key,
                    None,
                    add.clone(),
                    chosen,
                );
                let kept = kept.clone();
                let favourite = *favourite;
                ContextMenu::new(face, move |_| {
                    star_item(
                        Menu::new()
                            .item("Katman olarak ekle…", add.clone())
                            .icon(from_web(Some("layerAdd"))),
                        &kept,
                        favourite,
                    )
                })
                .into()
            }
            Line::List {
                key,
                label,
                mine,
                depth,
                open,
            } => self.row_button(
                row![
                    indent(*depth),
                    caret(*open),
                    glyph(if *mine { "folder" } else { "share" }),
                    name((*label).to_owned(), false)
                ],
                toggle(key),
                chosen,
            ),
            Line::Project {
                key,
                project,
                depth,
                open,
            } => {
                let file = project.storage == ProjectStorage::File;
                let face = self.row_button(
                    row![
                        indent(*depth),
                        caret(*open),
                        glyph(if file { "fileOpen" } else { "server" }),
                        name(project.name.clone(), false),
                        meta(if file { "dosya" } else { "veritabanı" }.to_owned()),
                    ],
                    toggle(key),
                    chosen,
                );
                let body = [
                    Some(project.description.clone()).filter(|d| !d.is_empty()),
                    Some(format!(
                        "{} · {}",
                        project.tenant_name,
                        if file {
                            "dosya projesi"
                        } else {
                            "veritabanı projesi"
                        }
                    )),
                    Some("Açılınca çizimi indirilir ve katmanları listelenir.".to_owned()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n");
                tip(
                    face,
                    Tip::new(project.name.clone()).body(body),
                    tooltip::Position::Left,
                )
            }
            Line::Group {
                key,
                label,
                depth,
                open,
            } => self.row_button(
                row![
                    indent(*depth),
                    caret(*open),
                    glyph("folder"),
                    name(label.clone(), false)
                ],
                msg(Event::Toggle(format!("{key}\0kapalı"))),
                chosen,
            ),
            Line::Layer {
                key,
                project,
                project_name,
                path,
                label,
                count,
                depth,
            } => {
                let add = msg(Event::AddLayer(project.clone(), path.clone()));
                let face = mouse_area(self.row_button(
                    row![
                        indent(*depth),
                        space().width(Length::Fixed(12.0)),
                        glyph("layers"),
                        name(label.clone(), false),
                        meta(format!("{count} nesne")),
                        add_button(add.clone()),
                    ],
                    msg(Event::Focus(key.clone())),
                    chosen,
                ))
                .on_double_click(add.clone());
                let face = tip(
                    face,
                    Tip::new(path.clone()).body(format!(
                        "{project_name} · {count} nesne\nÇift tık ya da Katman olarak ekle: katman nesneleriyle bu çizime alınır."
                    )),
                    tooltip::Position::Left,
                );
                ContextMenu::new(face, move |_| {
                    Menu::new()
                        .item("Katman olarak ekle", add.clone())
                        .icon(from_web(Some("layerAdd")))
                        .shortcut("Enter")
                })
                .into()
            }
            Line::Note {
                label,
                warn,
                glyph: g,
                depth,
                act,
                ..
            } => {
                let warn = *warn;
                let words = text(label.clone())
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill)
                    .wrapping(text::Wrapping::None)
                    .style(move |t: &Theme| text::Style {
                        color: Some(if warn {
                            Tokens::of(t).warning
                        } else {
                            Tokens::of(t).faint
                        }),
                    });
                let mut face = row![indent(*depth), space().width(Length::Fixed(12.0))];
                if let Some(g) = g {
                    face = face.push(glyph(g));
                }
                let face = face.push(words);
                match act {
                    Some(act) => self.row_button(face, act.clone(), chosen),
                    None => container(face.spacing(6).padding([0, 8]).align_y(Center).height(Fill))
                        .width(Fill)
                        .height(Fill)
                        .into(),
                }
            }
        }
    }

    /// A table's row (docs/adr/0599 §2, §3): its name, kind, rows and system, or the layer it is bound to; a click
    /// opens its preview, the button and a drop bind it (or go to its layer when bound).
    #[allow(clippy::too_many_arguments)]
    fn table_row<'a>(
        &'a self,
        key: &str,
        origin: &Origin,
        t: &SourceTable,
        bound: Option<&(String, String)>,
        depth: usize,
        open: bool,
        chosen: bool,
    ) -> Element<'a, Message> {
        let caret = icon(if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        })
        .size(12.0);
        let words = match bound {
            Some((_, layer)) => format!("Bağlı: {layer}"),
            None => {
                let mut w = geometry_word(t.geometry.as_deref()).to_owned();
                w.push_str(&format!(" · {}", t.rows));
                if let Some(c) = t.srid {
                    w.push_str(&format!(" · EPSG:{c}"));
                }
                w
            }
        };
        let bound_layer = bound.map(|(id, _)| id.clone());
        let bind = msg(Event::OpenTable(origin.clone(), t.name.clone(), true));
        let add = match &bound_layer {
            Some(id) => msg(Event::GoToLayer(id.clone())),
            None => bind.clone(),
        };
        let action = match &bound_layer {
            Some(_) => tip(
                button(icon(from_web(Some("zoomSelection"))).size(15.0))
                    .on_press(add.clone())
                    .padding([2, 4])
                    .style(style::button::ghost),
                Tip::new("Katmana git")
                    .body("Tablo bu çizimde bağlı; katmanını Katmanlar'da seçer ve gösterir."),
                tooltip::Position::Left,
            ),
            None => tip(
                button(icon(from_web(Some("layerAdd"))).size(15.0))
                    .on_press(add.clone())
                    .padding([2, 4])
                    .style(style::button::ghost),
                Tip::new("Bağlı katman olarak aç"),
                tooltip::Position::Left,
            ),
        };
        let words =
            label::caption(words)
                .wrapping(text::Wrapping::None)
                .style(move |th: &Theme| text::Style {
                    color: Some(if bound_layer.is_some() {
                        Tokens::of(th).accent
                    } else {
                        Tokens::of(th).faint
                    }),
                });
        let face = self.drag_row(
            row![
                space().width(Length::Fixed(depth as f32 * INDENT)),
                caret,
                icon(from_web(Some(if t.geometry.is_some() {
                    "layers"
                } else {
                    "table"
                })))
                .size(15.0),
                // Under its schema's group a database's table is named without the schema.
                text(match origin {
                    Origin::Database(_) => t
                        .name
                        .split_once('.')
                        .map_or_else(|| t.name.clone(), |(_, n)| n.to_owned()),
                    Origin::File(_) => t.name.clone(),
                })
                .font(typography::ui())
                .size(typography::body())
                .width(Fill)
                .wrapping(text::Wrapping::None),
                words,
                action,
            ],
            key,
            Some(msg(Event::Toggle(key.to_owned()))),
            add.clone(),
            chosen,
        );
        let face = tip(
            face,
            Tip::new(t.name.clone()).body(format!(
                "{} · {} kayıt{}\n{}\nAçılınca kapsamı ve ilk {} kaydı görünür. Çift tık ya da çizime sürükleme {}.",
                t.geometry.as_deref().unwrap_or("geometrisiz tablo"),
                t.rows,
                t.srid.map_or_else(String::new, |c| format!(" · EPSG:{c}")),
                origin.path(),
                super::SAMPLE_ROWS,
                if bound.is_some() {
                    "katmanına gider"
                } else {
                    "bağlı katman olarak açar"
                }
            )),
            tooltip::Position::Left,
        );
        let kept = origin.kept(&t.name);
        let favourite = self.sources.folders.is_favourite(&kept);
        let copy = matches!(origin, Origin::File(_))
            .then(|| msg(Event::OpenTable(origin.clone(), t.name.clone(), false)));
        let extent = t
            .extent
            .map(|e| msg(Event::ShowExtent(t.srid, e, t.name.clone())));
        let bound = bound.is_some();
        ContextMenu::new(face, move |_| {
            let mut menu = if bound {
                Menu::new()
                    .item("Katmana git", add.clone())
                    .icon(from_web(Some("zoomSelection")))
                    .detail("Tablo bu çizimde zaten bağlı; ikinci kez bağlanmaz.")
            } else {
                Menu::new()
                    .item("Bağlı katman olarak aç…", bind.clone())
                    .icon(from_web(Some("layerAdd")))
            };
            if let Some(copy) = &copy {
                menu = menu
                    .item("Kopya olarak al…", copy.clone())
                    .icon(from_web(Some("importGpkg")))
                    .detail("Tablonun nesneleri çizime kopyalanır; kaynağa bağlanmaz.");
            }
            if let Some(extent) = &extent {
                menu = menu
                    .item("Kapsamı göster", extent.clone())
                    .icon(from_web(Some("serviceExtent")));
            }
            star_item(menu, &kept, favourite)
        })
        .into()
    }

    /// A row that can be dragged onto the drawing (docs/adr/0599 §8): pressing it starts the drag; let go over the
    /// drawing it adds, elsewhere it is a click; a double click adds at once.
    fn drag_row<'a>(
        &self,
        face: iced::widget::Row<'a, Message>,
        key: &str,
        click: Option<Message>,
        add: Message,
        chosen: bool,
    ) -> Element<'a, Message> {
        let dragged = self.sources.drag.as_ref().is_some_and(|d| d.key == key);
        let face = mouse_area(face.spacing(6).padding([0, 8]).align_y(Center).height(Fill))
            .on_press(msg(Event::Press(
                key.to_owned(),
                click.map(Box::new),
                Box::new(add.clone()),
            )))
            .on_double_click(add)
            .interaction(if dragged {
                iced::mouse::Interaction::Grabbing
            } else {
                iced::mouse::Interaction::Pointer
            });
        // The press is the area's: the button only draws the row (and takes the keyboard).
        button(face)
            .on_press(msg(Event::Focus(key.to_owned())))
            .padding(0)
            .width(Fill)
            .height(Fill)
            .style(style::button::row(chosen || dragged))
            .into()
    }

    /// A row's face as a button over the whole row.
    fn row_button<'a>(
        &self,
        face: iced::widget::Row<'a, Message>,
        on: Message,
        chosen: bool,
    ) -> Element<'a, Message> {
        button(face.spacing(6).padding([0, 8]).align_y(Center).height(Fill))
            .on_press(on)
            .padding(0)
            .width(Fill)
            .height(Fill)
            .style(style::button::row(chosen))
            .into()
    }
}

/// The row's Katman olarak ekle: always there, as the mouse should not have to look for it.
fn add_button<'a>(on: Message) -> Element<'a, Message> {
    tip(
        button(icon(from_web(Some("layerAdd"))).size(15.0))
            .on_press(on)
            .padding([2, 4])
            .style(style::button::ghost),
        Tip::new("Katman olarak ekle"),
        tooltip::Position::Left,
    )
}

/// Sık kullanılanlara ekle or Sık kullanılanlardan çıkar, after a row's menu (docs/adr/0599 §6).
fn star_item(menu: Menu<Message>, kept: &Kept, favourite: bool) -> Menu<Message> {
    if favourite {
        menu.item("Sık kullanılanlardan çıkar", msg(Event::Star(kept.clone())))
            .icon(from_web(Some("star")))
    } else {
        menu.item("Sık kullanılanlara ekle", msg(Event::Star(kept.clone())))
            .icon(from_web(Some("star")))
    }
}

/// A geometry type as the row says it, short: alan, çizgi, nokta, or tablo for an attribute table.
fn geometry_word(geometry: Option<&str>) -> &'static str {
    let Some(g) = geometry else { return "tablo" };
    let g = g.to_ascii_uppercase();
    if g.contains("POLYGON") || g.contains("SURFACE") {
        "alan"
    } else if g.contains("LINE") || g.contains("CURVE") {
        "çizgi"
    } else if g.contains("POINT") {
        "nokta"
    } else {
        "geometri"
    }
}
