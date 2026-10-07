//! Kaynaklar's view (the web's `SourcesPanel`): Klasör ekle over one list of
//! rows: the two sections, the folders and their files, the KentOS lists,
//! projects and their layers; a row opens and closes with its caret, a file
//! or a layer carries what it is and its Katman olarak ekle button, a double
//! click adds it too, and its menu.

use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, mouse_area, row, space, text, tooltip};
use iced::{Center, Element, Fill, Length, Theme};
use kentos_contracts::{LayerNode, LayerNodeType, ProjectStorage, ProjectSummary};
use kentos_interaction::sources::SourceFile;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{ContextMenu, Menu, Tip, VirtualList, tip};
use kentos_ui::{label, style};

use super::{
    Event, FOLDERS, KENTOS, LIMIT, LISTS, ProjectDrawing, Read, folder_key, folder_name, list_key,
    msg, project_key,
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
            | Line::List { key, .. }
            | Line::Project { key, .. }
            | Line::Group { key, .. }
            | Line::Layer { key, .. }
            | Line::Note { key, .. } => key,
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
    /// The rows as listed now.
    pub(crate) fn source_lines(&self) -> Vec<Line> {
        let s = &self.sources;
        let mut lines = Vec::new();
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
                    lines.push(Line::File {
                        key: format!("f:{}", path.join(&file.name).display()),
                        folder: path.to_path_buf(),
                        file: file.clone(),
                        depth: depth + 1,
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
        let bar = container(row![add, space::horizontal()].align_y(Center))
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
            } => {
                let add = msg(Event::AddFile(folder.clone(), file.clone()));
                let face = mouse_area(self.row_button(
                    row![
                        indent(*depth),
                        space().width(Length::Fixed(12.0)),
                        glyph(file.kind.icon()),
                        name(file.name.clone(), false),
                        meta(file.kind.label().to_owned()),
                        add_button(add.clone()),
                    ],
                    msg(Event::Focus(key.clone())),
                    chosen,
                ))
                .on_double_click(add.clone());
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
                    Menu::new()
                        .item("Katman olarak ekle…", add.clone())
                        .icon(from_web(Some("layerAdd")))
                        .shortcut("Enter")
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
