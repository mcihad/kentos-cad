//! The lines Kaynaklar lists: sources, folders, kept ones, tables and previews, databases, services,
//! lists, projects and layers.

use super::*;

impl App {
    /// The rows as listed now: those the search keeps when it has words (docs/adr/0599 §5).
    pub(crate) fn source_lines(&self) -> Vec<Line> {
        let lines = self.all_source_lines();
        let words = search_words(&self.gis.sources.search);
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

    pub(super) fn all_source_lines(&self) -> Vec<Line> {
        let s = &self.gis.sources;
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
        // Veritabanları and Servisler: the programs' that have them (module_surfaces.rs).
        if self.shows_source_section(SourceSection::Databases) {
            self.database_lines(&mut lines);
        }
        if self.shows_source_section(SourceSection::Services) {
            self.service_lines(&mut lines);
        }
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

    pub(super) fn folder_lines(
        &self,
        path: &Path,
        label: String,
        root: bool,
        depth: usize,
        lines: &mut Vec<Line>,
    ) {
        let key = folder_key(path);
        let open = self.gis.sources.open.contains(&key);
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
        match self.gis.sources.listings.get(path) {
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
                        let open = self.gis.sources.open.contains(&key);
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
    pub(super) fn kept_lines(
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
        let open = self.gis.sources.open.contains(key);
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
                favourite: self.gis.sources.folders.is_favourite(k),
                depth: 1,
            });
        }
    }

    /// A GeoPackage's tables under it, a table's preview under the table (docs/adr/0599 §2).
    pub(super) fn table_lines(&self, origin: &Origin, depth: usize, lines: &mut Vec<Line>) {
        let key = tables_key(origin);
        match self.gis.sources.tables.get(origin) {
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
                for t in tables {
                    let at = depth;
                    let tkey = table_key(origin, &t.name);
                    let open = self.gis.sources.open.contains(&tkey);
                    lines.push(Line::FileTable {
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
    pub(super) fn bound_tables(&self, origin: &Origin) -> Vec<(String, String, String)> {
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

    pub(super) fn preview_lines(
        &self,
        key: &str,
        t: &SourceTable,
        depth: usize,
        lines: &mut Vec<Line>,
    ) {
        if let Some(extent) = t.extent {
            lines.push(Line::Extent {
                key: format!("{key}\0kapsam"),
                srid: t.srid,
                extent,
                name: t.name.clone(),
                depth,
            });
        }
        match self.gis.sources.samples.get(key) {
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

    /// Veritabanları: the device's connections, each open into its schemas and tables.
    pub(super) fn database_lines(&self, lines: &mut Vec<Line>) {
        let open = self.gis.sources.open.contains(DATABASES);
        lines.push(Line::Section {
            key: DATABASES,
            label: "Veritabanları",
            glyph: "sourceDatabase",
            open,
        });
        if !open {
            return;
        }
        let list = self.db_store.list();
        if list.is_empty() {
            lines.push(Line::Note {
                key: format!("{DATABASES}:yok"),
                label: "Bağlantı yok. Bir PostgreSQL veritabanı ekleyin.".to_owned(),
                warn: false,
                glyph: Some("sourceDatabase"),
                depth: 1,
                act: Some(db(Db::New(Then::Browse))),
            });
            lines.push(Line::Note {
                key: format!("{DATABASES}:dosya"),
                label: "Size verilen bir bağlantı dosyasını içe aktarın…".to_owned(),
                warn: false,
                glyph: Some("import"),
                depth: 1,
                act: Some(db(Db::Import)),
            });
            return;
        }
        for c in list {
            let key = connection_key(&c.name);
            let open = self.gis.sources.open.contains(&key);
            let browse = self.gis.sources.catalogs.get(&c.name);
            let facts = match browse {
                Some(Browse::Ready(catalog)) => Some(catalog.facts.line()),
                _ => None,
            };
            let name = c.name.clone();
            lines.push(Line::Database {
                key: key.clone(),
                connection: Box::new(c.clone()),
                facts,
                open,
            });
            if !open {
                continue;
            }
            match browse {
                None | Some(Browse::Reading) => {
                    lines.push(note(format!("{key}\0okunuyor"), "Bağlanılıyor…", false, 2));
                }
                Some(Browse::Failed(d)) => {
                    let password = matches!(d.problem, Problem::PasswordNeeded | Problem::Password);
                    lines.push(Line::Note {
                        key: format!("{key}\0hata"),
                        label: d.message.clone(),
                        warn: true,
                        glyph: None,
                        depth: 2,
                        act: None,
                    });
                    lines.push(Line::Note {
                        key: format!("{key}\0çare"),
                        label: if password {
                            "Parolayı yaz…".to_owned()
                        } else {
                            "Bağlantıyı düzenle…".to_owned()
                        },
                        warn: false,
                        glyph: Some(if password { "lock" } else { "edit" }),
                        depth: 2,
                        act: Some(if password {
                            msg(Event::AskPassword(name.clone()))
                        } else {
                            db(Db::Edit(name.clone(), Then::Browse))
                        }),
                    });
                }
                Some(Browse::Ready(catalog)) => {
                    let schemas = catalog.schemas(&c);
                    if schemas.is_empty() {
                        lines.push(note(
                            format!("{key}\0boş"),
                            "Listelenecek tablo yok.",
                            false,
                            2,
                        ));
                    }
                    for (schema, relations) in schemas {
                        let skey = format!("sc:{}\0{schema}", c.name);
                        // A schema stays open unless closed by hand.
                        let open = !self.gis.sources.open.contains(&format!("{skey}\0kapalı"));
                        lines.push(Line::Schema {
                            key: skey.clone(),
                            label: schema.to_owned(),
                            count: relations.len(),
                            depth: 2,
                            open,
                        });
                        if !open {
                            continue;
                        }
                        for r in relations {
                            lines.push(Line::Table {
                                key: format!("t:{}\0{}", c.name, r.table()),
                                connection: c.name.clone(),
                                relation: Box::new(r.clone()),
                                depth: 3,
                            });
                        }
                    }
                    let hidden = catalog.hidden(&c);
                    if hidden > 0 {
                        lines.push(Line::Note {
                            key: format!("{key}\0gizli"),
                            label: format!("{hidden} geometrisiz tablo gizli; göstermek için bağlantıyı düzenleyin."),
                            warn: false,
                            glyph: Some("table"),
                            depth: 2,
                            act: Some(db(Db::Edit(name.clone(), Then::Browse))),
                        });
                    }
                }
            }
        }
    }

    /// Servisler (docs/adr/0599 §4): the ready basemaps, then the services added before.
    pub(super) fn service_lines(&self, lines: &mut Vec<Line>) {
        let s = &self.gis.sources;
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

    pub(super) fn list_lines(
        &self,
        view: &'static str,
        label: &'static str,
        lines: &mut Vec<Line>,
    ) {
        let key = list_key(view);
        let open = self.gis.sources.open.contains(&key);
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
        match self.gis.sources.lists.get(view) {
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

    pub(super) fn project_lines(&self, p: &ProjectSummary, lines: &mut Vec<Line>) {
        let key = project_key(&p.id);
        let open = self.gis.sources.open.contains(&key);
        lines.push(Line::Project {
            key: key.clone(),
            project: Box::new(p.clone()),
            depth: 2,
            open,
        });
        if !open {
            return;
        }
        match self.gis.sources.drawings.get(&p.id) {
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

    pub(super) fn layer_lines(
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
            let open = !self.gis.sources.open.contains(&format!("{key}\0kapalı"));
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
}
