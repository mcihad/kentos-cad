//! Uygulama menüsü (the web's `ui/appmenu/AppMenu.ts`, DESIGN.md §7.1.1):
//! the KentOS mark at the ribbon's corner opens it, like AutoCAD's
//! application menu. The file commands run down the left; the right side
//! shows what the pointed row holds (the import and export formats, the
//! cloud) or, at first, the drawing on screen with the recent files. Rows
//! run the same command ids as the ribbon and the command line, so a row
//! does exactly what they do; one the desktop does not run yet is dimmed
//! and says why, as in the ribbon. The keyboard walks the rows (↑ ↓, →
//! into the right side, ← back), Enter runs the row, Esc closes the menu.

use std::path::PathBuf;

use iced::Task;
use kentos_cloud::{ApiFailure, CatalogQuery};
use kentos_contracts::{CatalogView, ProjectPage, ProjectSummary};
use kentos_ui::icon::Icon;

use crate::app::{App, Message, Then};
use crate::catalog::{Standing, catalog};
use crate::cloud::words;

/// What the right side shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    /// Bu çizim: the drawing on screen, four quick starts and the recent files.
    #[default]
    Overview,
    Import,
    Export,
    Cloud,
}

/// A row of the left column: a command, or a pane to show.
struct Nav {
    label: &'static str,
    icon: Icon,
    target: Target,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Command(&'static str),
    Pane(Pane),
}

/// The left column, in the web's order.
const NAV: [Nav; 9] = [
    Nav {
        label: "Yeni",
        icon: Icon::DocumentNew,
        target: Target::Command("file.new"),
    },
    Nav {
        label: "Aç",
        icon: Icon::Open,
        target: Target::Command("file.open"),
    },
    Nav {
        label: "Kaydet",
        icon: Icon::Save,
        target: Target::Command("file.save"),
    },
    Nav {
        label: "Farklı kaydet",
        icon: Icon::SaveAs,
        target: Target::Command("file.saveAs"),
    },
    Nav {
        label: "İçe aktar",
        icon: Icon::Import,
        target: Target::Pane(Pane::Import),
    },
    Nav {
        label: "Dışa aktar",
        icon: Icon::Export,
        target: Target::Pane(Pane::Export),
    },
    Nav {
        label: "Bulut",
        icon: Icon::Globe,
        target: Target::Pane(Pane::Cloud),
    },
    Nav {
        label: "Yazdır ve pafta",
        icon: Icon::Print,
        target: Target::Command("file.print"),
    },
    Nav {
        label: "Proje ayarları",
        icon: Icon::Folder,
        target: Target::Command("file.settings"),
    },
];

/// A format of İçe aktar or Dışa aktar: its command and what it takes or gives (the web's `FORMATS`).
struct Format {
    id: &'static str,
    label: &'static str,
    detail: &'static str,
    badge: &'static str,
}

const IMPORTS: [Format; 5] = [
    Format {
        id: "file.import.dxf",
        label: "DXF",
        detail: "AutoCAD R12–2018: katmanlar, bloklar, ölçüler ve taramalar",
        badge: "DXF",
    },
    Format {
        id: "file.import.ncn",
        label: "Koordinat listesi",
        detail: "Netcad NCN, TXT ya da CSV nokta listesi",
        badge: "NCN",
    },
    Format {
        id: "file.import.ncz",
        label: "Netcad çizimi",
        detail: "Netcad NCZ dosyası",
        badge: "NCZ",
    },
    Format {
        id: "file.import.shp",
        label: "Shapefile",
        detail: "SHP, SHX, DBF, PRJ ve CPG birlikte; öznitelikleriyle",
        badge: "SHP",
    },
    Format {
        id: "file.import.geojson",
        label: "GeoJSON",
        detail: "RFC 7946; özellikler öznitelik olur",
        badge: "JSON",
    },
];

const EXPORTS: [Format; 4] = [
    Format {
        id: "file.export.dxf",
        label: "DXF",
        detail: "AutoCAD 2007: ölçüler DXF ölçüsü, Türkçe yazılar UTF-8",
        badge: "DXF",
    },
    Format {
        id: "file.export.ncn",
        label: "Koordinat listesi",
        detail: "Noktalar NCN, TXT ya da CSV olarak",
        badge: "NCN",
    },
    Format {
        id: "file.export.geojson",
        label: "GeoJSON",
        detail: "Öznitelikleriyle; WGS 84 projesi RFC 7946",
        badge: "JSON",
    },
    Format {
        id: "file.export.pdf",
        label: "PDF pafta",
        detail: "Ölçekli pafta çıktısı",
        badge: "PDF",
    },
];

/// The quick starts under the drawing (the web's tiles).
const TILES: [(&str, Icon, &str); 4] = [
    ("file.new", Icon::DocumentNew, "Yeni proje"),
    ("file.open", Icon::Open, "Dosya aç"),
    ("cloud.open", Icon::Globe, "Bulut projesi"),
    ("file.import.dxf", Icon::Import, "DXF içe aktar"),
];

/// The recent files the menu lists (the start screen lists them all).
const RECENT: usize = 5;
/// The account's recent cloud projects it lists.
const PROJECTS: u32 = 5;

/// Commands that need a drawing on screen.
const NEEDS_DRAWING: [&str; 7] = [
    "file.save",
    "file.saveAs",
    "file.settings",
    "file.print",
    "file.export.dxf",
    "file.export.ncn",
    "file.export.geojson",
];

/// Where the keyboard is: a row of the left column or of the right side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Nav(usize),
    Pane(usize),
}

/// The open menu.
#[derive(Default)]
pub struct State {
    pane: Pane,
    focus: Option<Focus>,
    projects: Projects,
    /// The request whose answer is awaited; a later answer to an earlier one is dropped.
    request: u64,
    /// Dropped with the menu: an unanswered request stops.
    loading: Option<iced::task::Handle>,
}

/// The account's recent cloud projects, asked for when the cloud pane first shows.
#[derive(Default)]
enum Projects {
    #[default]
    NotAsked,
    Loading,
    Loaded(Vec<ProjectSummary>),
    Failed,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The KentOS mark: opens the menu, or closes it.
    Toggle,
    /// A click beside the menu.
    Dismiss,
    /// The pointer on a row with ▸.
    Show(Pane),
    /// A command (a row, a tile, a format, a footer button): the menu closes, the command runs.
    Run(&'static str),
    /// A recent file: the drawing on screen is left, then it opens.
    OpenRecent(PathBuf),
    /// × beside a recent file.
    Forget(PathBuf),
    /// A recent cloud project: the drawing on screen is left, then it opens.
    OpenProject { tenant: String, project: String },
    /// The account's recent projects arrived (boxed: a message stays small).
    Projects {
        id: u64,
        result: Box<Result<ProjectPage, ApiFailure>>,
    },
}

fn event(e: Event) -> Message {
    Message::AppMenu(e)
}

impl App {
    pub(crate) fn app_menu_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Toggle => {
                if self.app_menu.take().is_none() {
                    self.app_menu = Some(State::default());
                }
            }
            Event::Dismiss => self.app_menu = None,
            Event::Show(pane) => return self.show_pane(pane, None),
            Event::Run(id) => {
                self.app_menu = None;
                return self.run(id);
            }
            Event::OpenRecent(path) => {
                self.app_menu = None;
                return self.open_recent(path);
            }
            Event::Forget(path) => self.recent.remove(&path),
            Event::OpenProject { tenant, project } => {
                // The opening window names the project and how it is kept, as from the catalog.
                let hint = self.app_menu.take().and_then(|s| match s.projects {
                    Projects::Loaded(list) => list
                        .into_iter()
                        .find(|p| p.id == project && p.tenant_id == tenant)
                        .map(|p| (p.name, p.storage)),
                    _ => None,
                });
                let (Some(tenant), Some(project)) =
                    (crate::cloud::uuid(&tenant), crate::cloud::uuid(&project))
                else {
                    self.warn("Bulut projesi açılamadı: sunucunun verdiği kimlik okunamadı.");
                    return Task::none();
                };
                self.cloud.open_hint = hint;
                return self.leave(Then::OpenCloud { tenant, project });
            }
            Event::Projects { id, result } => {
                if let Some(s) = &mut self.app_menu
                    && s.request == id
                {
                    s.loading = None;
                    s.projects = match *result {
                        Ok(page) => Projects::Loaded(page.projects),
                        Err(_) => Projects::Failed,
                    };
                }
            }
        }
        Task::none()
    }

    /// Shows `pane` on the right; the cloud's recent projects are asked for the first time it shows.
    fn show_pane(&mut self, pane: Pane, focus: Option<Focus>) -> Task<Message> {
        let id = self.cloud.next_id();
        let client = self.cloud.signed_in().cloned();
        let Some(s) = &mut self.app_menu else {
            return Task::none();
        };
        s.pane = pane;
        if focus.is_some() {
            s.focus = focus;
        }
        if pane != Pane::Cloud || !matches!(s.projects, Projects::NotAsked) {
            return Task::none();
        }
        let Some(client) = client else {
            return Task::none();
        };
        let mut query = CatalogQuery::new(CatalogView::Recent);
        query.sort = Some(words::view(CatalogView::Recent).sort);
        query.limit = Some(PROJECTS);
        s.projects = Projects::Loading;
        s.request = id;
        let (task, handle) = Task::perform(client.catalog(&query), move |result| {
            event(Event::Projects {
                id,
                result: Box::new(result),
            })
        })
        .abortable();
        s.loading = Some(handle.abort_on_drop());
        task
    }

    /// Whether a row of the left column runs.
    fn nav_enabled(&self, i: usize) -> bool {
        match NAV[i].target {
            Target::Command(id) => self.menu_runs(id),
            Target::Pane(_) => true,
        }
    }

    /// Whether a command runs from the menu: on the desktop, possible now,
    /// and with a drawing when it needs one.
    fn menu_runs(&self, id: &str) -> bool {
        catalog()
            .get(id)
            .is_some_and(|c| c.standing == Standing::Ported)
            && self.available(id)
            && (self.document.is_some() || !NEEDS_DRAWING.contains(&id))
    }

    /// Why a command does not run from the menu, in a few words.
    fn menu_why(&self, id: &str) -> Option<&'static str> {
        let command = catalog().get(id)?;
        match command.standing {
            Standing::Pending => Some("Geliştirme aşamasında"),
            Standing::OnTheWeb => Some("Web'de var; masaüstüne henüz taşınmadı"),
            Standing::Ported if self.document.is_none() && NEEDS_DRAWING.contains(&id) => {
                Some("Açık çizim yok")
            }
            Standing::Ported => None,
        }
    }

    /// The cloud pane's buttons: the web's, those not on the desktop yet dimmed.
    fn cloud_actions(&self) -> Vec<(&'static str, &'static str, Icon)> {
        if self.cloud.me.is_none() {
            return vec![
                ("cloud.signIn", "Giriş yap", Icon::Link),
                ("cloud.open", "Bu cihazdaki projeler", Icon::Globe),
            ];
        }
        let mut actions = vec![
            ("cloud.open", "Proje aç", Icon::Globe),
            ("cloud.upload", "Buluta yükle", Icon::Export),
        ];
        if self
            .document
            .as_ref()
            .is_some_and(|d| d.cloud_source().is_some())
        {
            actions.extend([
                ("cloud.history", "Geçmiş", Icon::Clock),
                ("cloud.share", "Paylaş", Icon::Link),
                ("cloud.rename", "Yeniden adlandır", Icon::Type),
                ("cloud.delete", "Sil", Icon::Close),
            ]);
        }
        actions
    }
}

mod cloud;
mod keys;
mod panes;
#[cfg(test)]
mod tests;
