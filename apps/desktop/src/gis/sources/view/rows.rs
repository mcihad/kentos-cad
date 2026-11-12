//! A line's row in Kaynaklar: a source's name, kind, state and actions; a file's table, a dragged row and
//! the rows' buttons.

use super::*;

impl App {
    pub(super) fn source_row<'a>(&'a self, line: &Line, chosen: bool) -> Element<'a, Message> {
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
            } => {
                let face = self.row_button(
                    row![caret(*open), glyph(g), name((*label).to_owned(), true)],
                    toggle(key),
                    chosen,
                );
                if *key == DATABASES {
                    ContextMenu::new(face, |_| databases_menu()).into()
                } else {
                    face
                }
            }
            Line::Database {
                key,
                connection,
                facts,
                open,
            } => {
                let c = connection.as_ref();
                let face = self.row_button(
                    row![
                        indent(1),
                        caret(*open),
                        glyph("sourceDatabase"),
                        name(c.name.clone(), false),
                        meta(c.host.clone()),
                    ],
                    toggle(key),
                    chosen,
                );
                let mut body = c.summary();
                if let Some(f) = facts {
                    body.push('\n');
                    body.push_str(f);
                }
                body.push_str("\nAçınca şemaları ve tabloları listelenir; sağ tıkla düzenleyin.");
                let face = tip(
                    face,
                    Tip::new(c.name.clone()).body(body),
                    tooltip::Position::Left,
                );
                let name = c.name.clone();
                ContextMenu::new(face, move |_| connection_menu(&name)).into()
            }
            Line::Schema {
                key,
                label,
                count,
                depth,
                open,
            } => self.row_button(
                row![
                    indent(*depth),
                    caret(*open),
                    glyph("folder"),
                    name(label.clone(), false),
                    meta(format!("{count}")),
                ],
                msg(Event::Toggle(format!("{key}\0kapalı"))),
                chosen,
            ),
            Line::Table {
                key,
                connection,
                relation,
                depth,
            } => self.table_row(key, connection, relation, *depth, chosen),
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
            Line::FileTable {
                key,
                origin,
                table,
                bound,
                depth,
                open,
            } => self.file_table_row(key, origin, table, bound.as_ref(), *depth, *open, chosen),
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

    /// A GeoPackage table's row (docs/adr/0599 §2): its name, kind, rows and system, or the layer it is bound to; a click
    /// opens its preview, the button and a drop bind it (or go to its layer when bound).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn file_table_row<'a>(
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
                    .padding(kentos_ui::theme::metrics::icon_small(15.0))
                    .style(style::button::ghost),
                Tip::new("Katmana git")
                    .body("Tablo bu çizimde bağlı; katmanını Katmanlar'da seçer ve gösterir."),
                tooltip::Position::Left,
            ),
            None => tip(
                button(icon(from_web(Some("layerAdd"))).size(15.0))
                    .on_press(add.clone())
                    .padding(kentos_ui::theme::metrics::icon_small(15.0))
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
                text(t.name.clone())
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
                super::super::SAMPLE_ROWS,
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
        let copy = Some(msg(Event::OpenTable(origin.clone(), t.name.clone(), false)));
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
    pub(super) fn drag_row<'a>(
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
    pub(super) fn row_button<'a>(
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
