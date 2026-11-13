//! Kaynaklar's view (the web's `SourcesPanel`): Klasör ekle and Bağlantı ekle
//! over one list of rows: the sections, the folders and their files, the
//! database connections, their schemas and tables (docs/adr/0623), the KentOS
//! lists, projects and their layers; a row opens and closes with its caret, a
//! file, a table or a layer carries what it is and its Katman olarak ekle
//! button, a double click adds it too, and its menu. The source browser (docs/adr/0599): Son kullanılanlar and Sık
//! kullanılanlar first, a GeoPackage's tables and a table's preview, Servisler, the search box over the rows; a file,
//! table, layer or basemap row pressed and let go over the drawing adds it there.

use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, mouse_area, row, space, text, tooltip};
use iced::{Center, Element, Fill, Length, Theme};
use kentos_commands::{LayerNode, LayerNodeType, ProjectStorage, ProjectSummary, SourceTable};
use kentos_interaction::sources::{
    SourceFile, SourceKind, search_matches, search_shown, search_words,
};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{ContextMenu, Menu, Tip, VirtualList, tip};
use kentos_ui::{label, style};

use kentos_postgis::connections::DbConnection;
use kentos_postgis::probe::{Problem, Relation};

use super::{
    Browse, DATABASES, EXTENSIONS, Event, FAVOURITES, FOLDERS, KENTOS, Kept, LIMIT, LISTS, Origin,
    PRESETS, ProjectDrawing, RECENT, Read, SAVED, SERVICES, SavedService, connection_key,
    folder_key, folder_name, list_key, msg, project_key, service_key, table_key, tables_key,
};
use crate::app::{App, Message};
use crate::db_connections::{Event as Db, Then, msg as db};
use crate::icons::from_web;
use crate::module_surfaces::SourceSection;

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
    /// A table of a GeoPackage, and the layer it is bound to (its id and name).
    FileTable {
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
    /// A data source a running extension brings (docs/adr/8057 §3): its extension, its local id, its title, the
    /// extension's name and the kind's word.
    Extension {
        key: String,
        extension: String,
        local: String,
        title: String,
        owner: String,
        kind: &'static str,
        depth: usize,
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
    /// A database connection (docs/adr/0623).
    Database {
        key: String,
        connection: Box<DbConnection>,
        facts: Option<String>,
        open: bool,
    },
    /// A connection's schema.
    Schema {
        key: String,
        label: String,
        count: usize,
        depth: usize,
        open: bool,
    },
    /// A schema's table, view or materialized view.
    Table {
        key: String,
        connection: String,
        relation: Box<Relation>,
        depth: usize,
    },
}

impl Line {
    pub(crate) fn key(&self) -> &str {
        match self {
            Line::Section { key, .. } => key,
            Line::Folder { key, .. }
            | Line::File { key, .. }
            | Line::FileTable { key, .. }
            | Line::Extent { key, .. }
            | Line::Database { key, .. }
            | Line::Schema { key, .. }
            | Line::Table { key, .. }
            | Line::Branch { key, .. }
            | Line::Preset { key, .. }
            | Line::Extension { key, .. }
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
            Line::Database { .. } => 1,
            Line::Folder { depth, .. }
            | Line::File { depth, .. }
            | Line::FileTable { depth, .. }
            | Line::Schema { depth, .. }
            | Line::Table { depth, .. }
            | Line::Extent { depth, .. }
            | Line::Branch { depth, .. }
            | Line::Preset { depth, .. }
            | Line::Extension { depth, .. }
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
            Line::FileTable {
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
                out.push(origin.path());
                out
            }
            Line::Database { connection, .. } => {
                vec![connection.name.clone(), connection.host.clone()]
            }
            Line::Schema { label, .. } => vec![label.clone()],
            Line::Table { relation, .. } => {
                let mut out = vec![relation.table(), relation.schema.clone()];
                out.extend(relation.geometry.clone());
                out.push(geometry_word(relation.geometry.as_deref()).to_owned());
                out.extend(relation.srid.map(|c| format!("EPSG:{c}")));
                out.extend(relation.comment.clone());
                out
            }
            Line::Preset { name, group, .. } => {
                vec![name.clone(), group.clone(), "Hazır altlık".to_owned()]
            }
            Line::Extension {
                title, owner, kind, ..
            } => vec![
                title.clone(),
                owner.clone(),
                (*kind).to_owned(),
                "Eklenti".to_owned(),
            ],
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
            Line::Extent { .. } | Line::Note { .. } => Vec::new(),
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
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::ghost),
            Tip::new("Klasör ekle").body(
                "Bilgisayarınızdaki bir klasörü listeye ekler; içindeki GeoJSON, Shapefile, DXF, NCZ, GPX, NMEA ve koordinat listesi dosyaları katman olarak eklenebilir.",
            ),
            tooltip::Position::Bottom,
        );
        let database = tip(
            button(icon(from_web(Some("sourceDatabase"))).size(16.0))
                .on_press(db(Db::New(Then::Browse)))
                .padding(kentos_ui::theme::metrics::icon_small(16.0))
                .style(style::button::ghost),
            Tip::new("Bağlantı ekle").body(
                "Bir PostgreSQL/PostGIS veritabanı bağlantısı tanımlar: sunucu, veritabanı, kullanıcı ve parola. Tabloları Veritabanları altında listelenir, çift tıkla bağlı katman olur.",
            ),
            tooltip::Position::Bottom,
        );
        let service = tip(
            button(icon(from_web(Some("serviceAdd"))).size(16.0))
                .on_press(Message::Run("service.add"))
                .padding(kentos_ui::theme::metrics::icon_small(16.0))
                .style(style::button::ghost),
            Tip::new("Servis ekle").body(
                "Harita servisi penceresini açar; eklenen servis Servisler'in Kayıtlı servisler'inde kalır.",
            ),
            tooltip::Position::Bottom,
        );
        let search = kentos_ui::widget::text_field(
            "Kaynaklarda ara: ad, tür, EPSG, şema",
            &self.gis.sources.search,
        )
        .on_input(|t| msg(Event::Search(t)))
        .width(Fill);
        // Bağlantı ekle and Servis ekle with their sections (module_surfaces.rs).
        let database = self
            .shows_source_section(SourceSection::Databases)
            .then_some(database);
        let service = self
            .shows_source_section(SourceSection::Services)
            .then_some(service);
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
        let focused = self.gis.sources.focused.clone();
        let list = VirtualList::new(lines.len(), typography::scaled(ROW), move |i| {
            let line = &lines[i];
            let chosen = focused.as_deref() == Some(line.key());
            self.source_row(line, chosen)
        })
        .height(Fill);
        column![bar, list].into()
    }

}

impl App {
    /// The question of a table in another system (Katman olarak ekle).
    pub(crate) fn sources_reproject_view(&self) -> Option<Element<'_, Message>> {
        let ask = self.gis.sources.reproject.as_ref()?;
        let project = self.document.as_ref().map_or(0, |d| d.settings().srid);
        let detail = if project == 0 {
            "Proje yerel (koordinat sistemi yok): tablonun koordinatları çevrilemez. Tabloyu bağlamak için projeyi bir koordinat sisteminde açın.".to_owned()
        } else {
            format!(
                "Tablo kendi sisteminde (EPSG:{}) kalır; katmanın nesneleri projenin sistemine (EPSG:{project}) çevrilerek gösterilir, Kaynağa yaz onları tablonun sistemine geri çevirerek yazar.",
                ask.srid
            )
        };
        Some(kentos_ui::widget::overlay::modal(
            kentos_ui::widget::Confirm::new(
                "Koordinat sistemi farklı",
                msg(Event::Reproject(true)),
                msg(Event::Reproject(false)),
            )
            .message(format!(
                "“{}” EPSG:{} sisteminde, proje EPSG:{project} sisteminde.",
                ask.table, ask.srid
            ))
            .detail(detail)
            .confirm("Projenin sistemine çevirerek bağla"),
            msg(Event::Reproject(false)),
        ))
    }

    /// A connection's table: its geometry's glyph, its name, what it is and roughly how many rows, and Katman
    /// olarak ekle; one a bound layer cannot read is faint and says why.
    fn table_row<'a>(
        &'a self,
        key: &str,
        connection: &str,
        r: &Relation,
        depth: usize,
        chosen: bool,
    ) -> Element<'a, Message> {
        let why = r.why_not();
        let glyph = icon(from_web(Some(geometry_glyph(r.geometry.as_deref())))).size(15.0);
        let faint = why.is_some();
        let words = text(r.name.clone())
            .font(typography::ui())
            .size(typography::body())
            .width(Fill)
            .wrapping(text::Wrapping::None)
            .style(move |t: &Theme| text::Style {
                color: faint.then(|| Tokens::of(t).faint),
            });
        let what = crate::exchange::postgis::describe(r);
        // Short beside the name, as QGIS's Browser keeps its rows: the whole of it in the tip.
        let short = if faint {
            "bağlanamaz".to_owned()
        } else if matches!(
            r.kind,
            kentos_postgis::probe::RelationKind::View
                | kentos_postgis::probe::RelationKind::MaterializedView
        ) {
            "view".to_owned()
        } else {
            r.estimate
                .map(|n| format!("≈{}", crate::exchange::postgis::grouped(n)))
                .unwrap_or_default()
        };
        let meta = label::caption(short)
            .wrapping(text::Wrapping::None)
            .style(|t: &Theme| text::Style {
                color: Some(Tokens::of(t).faint),
            });
        let add = msg(Event::AddTable(connection.to_owned(), r.table(), false));
        let mut face = row![
            space().width(Length::Fixed(depth as f32 * INDENT)),
            space().width(Length::Fixed(12.0)),
            glyph,
            words,
            meta,
        ];
        if !faint {
            face = face.push(add_button(add.clone()));
        }
        let mut body = format!("{} · {what}", r.table());
        if let Some(c) = &r.comment {
            body.push_str(&format!("\n{c}"));
        }
        match &why {
            Some(w) => body.push_str(&format!("\n{w}")),
            None => body.push_str("\nÇift tık ya da Katman olarak ekle: bağlı katman olarak açılır; değişiklikler Kaynağa yaz ile veritabanına yazılır."),
        }
        let row = mouse_area(self.row_button(face, msg(Event::Focus(key.to_owned())), chosen));
        let row = if faint {
            row
        } else {
            row.on_double_click(add.clone())
        };
        let face = tip(
            row,
            Tip::new(r.name.clone()).body(body),
            tooltip::Position::Left,
        );
        if faint {
            return face;
        }
        let area = msg(Event::AddTable(connection.to_owned(), r.table(), true));
        ContextMenu::new(face, move |_| {
            Menu::new()
                .item("Katman olarak ekle", add.clone())
                .icon(from_web(Some("layerAdd")))
                .shortcut("Enter")
                .item("Yalnız görünen alanla ekle", area.clone())
                .icon(from_web(Some("zoomWindow")))
                .detail("Yalnız kutusu görünen alana değen satırlar alınır; milyon satırlı tabloda ilk harita hemen gelir.")
        })
        .into()
    }
}

/// Veritabanları's menu.
fn databases_menu() -> Menu<Message> {
    Menu::new()
        .item("Yeni bağlantı…", db(Db::New(Then::Browse)))
        .icon(from_web(Some("sourceDatabase")))
        .separator()
        .item("Bağlantıları içe aktar…", db(Db::Import))
        .icon(from_web(Some("import")))
        .detail("Başka bir KentOS'ta Bağlantıları dışa aktar ile yazılan dosya.")
        .item("Bağlantıları dışa aktar…", db(Db::Export(false)))
        .icon(from_web(Some("export")))
        .detail("Ekip arkadaşlarınıza aynı bağlantıları vermek için; parolalar yazılmaz.")
        .item("Parolalarıyla dışa aktar…", db(Db::Export(true)))
        .icon(from_web(Some("export")))
        .detail("Kaydedilmiş parolalar dosyaya düz metin yazılır; dosyayı paylaşırken dikkat edin.")
}

/// A connection's menu.
fn connection_menu(name: &str) -> Menu<Message> {
    let n = name.to_owned();
    Menu::new()
        .item("Yenile", msg(Event::Refresh(n.clone())))
        .icon(from_web(Some("sourceRead")))
        .detail("Şemalar ve tablolar yeniden okunur.")
        .item("Düzenle…", db(Db::Edit(n.clone(), Then::Browse)))
        .icon(from_web(Some("edit")))
        .item("Kopyasını oluştur…", db(Db::Duplicate(n.clone())))
        .icon(from_web(Some("copy")))
        .separator()
        .item("Sil", db(Db::AskRemove(n)))
        .icon(from_web(Some("trash")))
}

/// A geometry type's glyph: point, line, area, mixed; a table without one.
fn geometry_glyph(geometry: Option<&str>) -> &'static str {
    let Some(g) = geometry else { return "table" };
    let g = g.to_ascii_uppercase();
    if g.contains("POINT") {
        "point"
    } else if g.contains("LINE") || g.contains("CURVE") && !g.contains("POLYGON") {
        "polyline"
    } else if g.contains("POLYGON") || g.contains("SURFACE") {
        "polygon"
    } else {
        "layers"
    }
}

/// The row's Katman olarak ekle: always there, as the mouse should not have to look for it.
fn add_button<'a>(on: Message) -> Element<'a, Message> {
    tip(
        button(icon(from_web(Some("layerAdd"))).size(15.0))
            .on_press(on)
            .padding(kentos_ui::theme::metrics::icon_small(15.0))
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

#[path = "view/lines.rs"]
mod lines;

#[path = "view/rows.rs"]
mod rows;
