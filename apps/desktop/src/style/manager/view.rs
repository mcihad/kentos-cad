//! How Stil yöneticisi looks (the web's `StyleManager` and its `smgr__*`
//! rules): a bar with the search, the kinds and the Yeni, İçe aktar and
//! Dışa aktar menus; the source and category tree, the items as cards (only
//! the rows in view are built) and the details; the status, Kapat and, in
//! pick mode, Seç. The window has the web's size (1240 × 860 at most, 92 %
//! of the window's height), kept inside a smaller window with a margin. In
//! pick mode over Katman stili that window stays beneath.

use std::sync::Arc;

use iced::widget::tooltip::Position;
use iced::widget::{
    Column, Row, button, column, container, responsive, row, scrollable, space, stack, text_input,
};
use iced::{Border, Center, Element, Fill, Length, Theme};
use kentos_native_style::library::{CategoryNode, Source, StyleLibrary};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Column as TreeColumn, Node, TreeView, rename};
use kentos_ui::widget::{
    Confirm, Dialog, Menu, MenuButton, Segmented, Tip, VirtualList, overlay, tip, vertical_divider,
};

use super::details::{self, source_badge, symbol_of_item};
use super::{Event, KINDS, Manager, PickTarget, SEARCH, ev, ids_under, node_key};
use crate::app::{App, Message};
use crate::style::thumbs::{Look, Thumbs};

/// The window's size, the web's pixels at the default text size: its body
/// is as wide as the web's (whose columns reach the window's edges; here
/// the window keeps its margin around them).
const WIDTH: f32 = 1240.0 + 2.0 * MARGIN;
const HEIGHT: f32 = 860.0;
const MARGIN: f32 = 18.0;
/// The tree's and the details' widths; on a narrow window they give way to the list.
const TREE: f32 = 300.0;
const DETAILS: f32 = 340.0;
/// A card: its picture, its narrowest width, and the gap between cards.
const PICTURE: (f32, f32) = (116.0, 66.0);
const CARD_MIN: f32 = 132.0;
const GAP: f32 = 8.0;

/// A card's height: its padding, the picture, the name on two lines and, in
/// a search, the source's badge under it (the web's `.scard`).
fn card_height(searching: bool) -> f32 {
    let name = name_height();
    let mut h = 7.0 + PICTURE.1 + 6.0 + name + 8.0 + 2.0;
    if searching {
        h += 6.0 + typography::caption() + 3.0;
    }
    h.ceil()
}

/// Two lines of a card's name.
fn name_height() -> f32 {
    (typography::caption() * 1.3 * 2.0).ceil() + 2.0
}

impl App {
    /// Stil yöneticisi over the drawing (and over Katman stili when it picks for it).
    pub(crate) fn style_manager_view(&self) -> Element<'_, Message> {
        let Some(m) = &self.styles.manager else {
            return space().into();
        };
        let window: Element<'_, Message> = responsive(move |room| {
            let width = typography::from_default(WIDTH).min(room.width - 60.0);
            let height = typography::from_default(HEIGHT).min(room.height * 0.92);
            overlay::modal(self.style_manager_window(width, height), ev(Event::Close))
        })
        .into();
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        if matches!(
            m.pick.as_ref().map(|p| &p.target),
            Some(PickTarget::Slot(..))
        ) && self.styles.layer_style.is_some()
        {
            layers.push(self.layer_style_view());
        }
        layers.push(window);
        if let Some(ask) = self.delete_question(m) {
            layers.push(overlay::modal(ask, ev(Event::DeleteCancelled)));
        }
        stack(layers).into()
    }

    /// Sil's question (`askRemove`): the library keeps no undo.
    fn delete_question<'a>(&self, m: &Manager) -> Option<Element<'a, Message>> {
        let id = m.deleting.as_deref()?;
        let (item, source) = self.styles.library.get(id)?;
        let users = if item.kind() == kentos_native_style::library::ItemKind::Asset {
            self.styles.library.users_of(id).len()
        } else {
            0
        };
        let from = if source == Source::Project {
            "projenin kitaplığından"
        } else {
            "Kitaplığım’dan"
        };
        let mut ask = Confirm::new(
            "Kitaplıktan sil",
            ev(Event::DeleteConfirmed),
            ev(Event::DeleteCancelled),
        )
        .message(format!(
            "“{}” {from} silinsin mi? Bu geri alınamaz.",
            item.name()
        ))
        .confirm("Sil")
        .destructive();
        if users > 0 {
            ask = ask.detail(format!(
                "{users} sembol bu çizimi kullanıyor; silinirse onlarda boş kalır."
            ));
        }
        Some(ask.into())
    }

    fn style_manager_window(&self, width: f32, height: f32) -> Element<'_, Message> {
        let Some(m) = &self.styles.manager else {
            return space().into();
        };
        let lib = &self.styles.library;
        let title = m
            .pick
            .as_ref()
            .map_or_else(|| "Stil yöneticisi".to_owned(), |p| p.title.clone());
        let project_open = self.document.is_some();
        let palette = self.style_palette();
        let look = Look {
            palette: &palette,
            library: lib,
            images: &self.styles.images,
        };
        // A narrow window (or large text) takes from the side columns before the list.
        let tree_w = typography::from_default(TREE).min(width * 0.25).floor();
        let details_w = typography::from_default(DETAILS).min(width * 0.31).floor();
        // The preview fits the column, beside its padding and scroll bar.
        let preview_w = (details_w - 32.0 - 12.0).min(300.0);
        let details = details::details(
            m,
            lib,
            &self.styles.thumbs,
            &look,
            (self.selection.ids().len(), project_open),
            (preview_w, (preview_w * 172.0 / 300.0).round()),
        );
        let cols = row![
            container(tree_of(m, lib, project_open))
                .width(Length::Fixed(tree_w))
                .height(Fill)
                .padding([8, 6])
                .style(style::container::header),
            vertical_divider(),
            self.style_manager_list(m),
            vertical_divider(),
            container(
                scrollable(container(details).padding([14, 16]))
                    .direction(style::field::body_scrollbar())
                    .height(Fill)
            )
            .width(Length::Fixed(details_w))
            .height(Fill)
            .style(style::container::header),
        ]
        .height(Fill);
        let frame = container(cols).height(Fill).clip(true).style(frame);
        Dialog::new(title)
            .push(bar(m, width - 2.0 * MARGIN))
            .push(frame)
            .push(footer(m, lib))
            .width(typography::unscaled(width))
            .max_height(typography::unscaled(height))
            .into()
    }

    /// The middle column: where the list is and how many, then the cards.
    fn style_manager_list<'a>(&'a self, m: &'a Manager) -> Element<'a, Message> {
        let lib = &self.styles.library;
        let listed = m.listed(lib);
        let query = kentos_processing::text::js_trim(&m.query);
        let searching = !query.is_empty();
        let where_ = if searching {
            format!("“{query}” araması")
        } else {
            let mut place = vec![m.at.0.label().to_owned()];
            place.extend(m.at.1.iter().cloned());
            place.join(" › ")
        };
        let head = container(
            label::caption(format!("{where_}: {} öğe", listed.len())).style(style::text::muted),
        )
        .padding([8, 14])
        .width(Fill);
        let grid: Element<'a, Message> = if listed.is_empty() {
            container(
                label::body(if searching {
                    "Aramayla eşleşen sembol yok."
                } else {
                    "Bu kategoride sembol yok."
                })
                .style(style::text::muted),
            )
            .padding([40, 14])
            .center_x(Fill)
            .into()
        } else {
            let palette = Arc::new(self.style_palette());
            let images = self.styles.images.clone();
            let thumbs = &self.styles.thumbs;
            let selected = m.selected.clone();
            responsive(move |size| {
                let inner = (size.width - 28.0).max(1.0);
                let min = typography::from_default(CARD_MIN);
                let gap = typography::from_default(GAP);
                let per_row = (((inner + gap) / (min + gap)).floor() as usize).max(1);
                let card_w = ((inner - gap * (per_row - 1) as f32) / per_row as f32).floor();
                let card_h = card_height(searching);
                let rows = listed.len().div_ceil(per_row);
                let listed = listed.clone();
                let selected = selected.clone();
                let palette = palette.clone();
                let images = images.clone();
                let list = VirtualList::new(rows, card_h + gap, move |r| {
                    let look = Look {
                        palette: &palette,
                        library: lib,
                        images: &images,
                    };
                    let mut line = Row::new().spacing(gap);
                    for (id, source) in listed.iter().skip(r * per_row).take(per_row) {
                        let chosen = selected.as_deref() == Some(id.as_str());
                        line = line.push(card(
                            lib,
                            thumbs,
                            &look,
                            (id, *source),
                            (card_w, card_h),
                            chosen,
                            searching,
                        ));
                    }
                    container(line).padding([0, 14]).into()
                })
                .height(Fill);
                container(list)
                    .padding(iced::Padding {
                        top: 12.0,
                        ..iced::Padding::ZERO
                    })
                    .into()
            })
            .into()
        };
        container(column![head, horizontal_line(), grid].height(Fill))
            .width(Fill)
            .height(Fill)
            .style(style::container::field)
            .into()
    }
}

/// A one-pixel line in the border's colour, across.
fn horizontal_line<'a>() -> Element<'a, Message> {
    container(space())
        .width(Fill)
        .height(1)
        .style(style::container::grid_lines)
        .into()
}

/// The columns' frame: a hairline border with the window's small radius.
fn frame(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        border: Border {
            color: t.border,
            width: 1.0,
            radius: kentos_ui::theme::shape::radius(4.0).into(),
        },
        ..container::Style::default()
    }
}

/// The bar: search, kinds, and the Yeni sembol, İçe aktar and Dışa aktar menus.
/// The search field's width in a bar `width` wide: the web's 300 pixels at
/// most; on a narrow window (or with large text) it gives way before the
/// kinds and the buttons, whose widths are measured from their texts.
fn search_width(width: f32) -> f32 {
    let body = typography::body();
    let text = |s: &str| typography::text_width(s, body);
    let kinds: f32 = KINDS
        .iter()
        .map(|k| text(&k.to_string()) + 24.0 + 1.0)
        .sum::<f32>()
        + 2.0;
    let face = |s: &str, chevron: bool| {
        20.0 + 13.0 + 5.0 + text(s) + if chevron { 5.0 + 12.0 } else { 0.0 }
    };
    let buttons = face("Yeni sembol", false) + face("İçe aktar", true) + face("Dışa aktar", true);
    let room = width - kinds - buttons - 5.0 * 8.0 - 16.0;
    room.clamp(
        typography::from_default(120.0),
        typography::from_default(300.0),
    )
    .floor()
}

fn bar<'a>(m: &Manager, width: f32) -> Element<'a, Message> {
    let search = row![
        icon(Icon::Search).size(14.0).tone(Tone::Muted),
        text_input("Sembol ara: konut, sınır, tarama…", &m.query)
            .id(SEARCH)
            .on_input(|t| ev(Event::Search(t)))
            .padding([5, 0])
            .size(typography::body())
            .style(style::field::bare_input)
            .width(Fill),
    ]
    .spacing(6)
    .align_y(Center);
    let search = container(search)
        .padding([0, 8])
        .width(Length::Fixed(search_width(width)))
        .style(style::container::field_box);
    let new_menu = MenuButton::new(menu_face("plus", "Yeni sembol", false), || {
        Menu::new()
            .item("Alan sembolü", ev(Event::NewSymbol("fill")))
            .icon(crate::icons::from_web(Some("hatch")))
            .item("Çizgi sembolü", ev(Event::NewSymbol("line")))
            .icon(crate::icons::from_web(Some("symbolLine")))
            .item("İşaret sembolü", ev(Event::NewSymbol("marker")))
            .icon(crate::icons::from_web(Some("symbolMarker")))
            .separator()
            .item("SVG çizimi (düzenleyicide)…", ev(Event::NewDrawing))
            .icon(crate::icons::from_web(Some("edit")))
    });
    let import_menu = tip(
        MenuButton::new(menu_face("import", "İçe aktar", true), || {
            Menu::new()
                .item("Dosyadan…", ev(Event::ImportFile))
                .icon(crate::icons::from_web(Some("fileOpen")))
                .detail(".kstil, PNG ya da JPEG")
                .item("Panodan yapıştır", ev(Event::ImportClipboard))
                .icon(crate::icons::from_web(Some("paste")))
                .detail("Paylaşılan stil metni")
        }),
        Tip::new("Sembolleri kitaplığa ya da projeye alır"),
        Position::Bottom,
    );
    let export_menu = tip(
        MenuButton::new(menu_face("export", "Dışa aktar", true), || {
            Menu::new()
                .item("Dosyaya (.kstil)", ev(Event::ExportListed))
                .icon(crate::icons::from_web(Some("save")))
                .detail("Kullandıkları çizimlerle birlikte")
                .item("Panoya kopyala", ev(Event::ExportClipboard))
                .icon(crate::icons::from_web(Some("copy")))
                .detail("Bir iletiye yapıştırıp paylaşmak için")
        }),
        Tip::new("Listedeki sembolleri kullandıkları çizimlerle birlikte verir"),
        Position::Bottom,
    );
    row![
        search,
        Segmented::new(KINDS, m.kind, |k| ev(Event::Kind(k))),
        space::horizontal(),
        new_menu,
        import_menu,
        export_menu,
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

/// A menu button's face, as the web's small buttons.
fn menu_face<'a>(glyph: &str, text: &'static str, chevron: bool) -> Element<'a, Message> {
    let mut face = row![
        icon(crate::icons::from_web(Some(glyph))).size(13.0),
        label::body(text)
    ]
    .spacing(5)
    .align_y(Center);
    if chevron {
        face = face.push(icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted));
    }
    container(face)
        .padding([4, 10])
        .style(style::container::field_box)
        .into()
}

/// The footer: what the window last said, Kapat, and Seç in pick mode.
fn footer<'a>(m: &Manager, lib: &StyleLibrary) -> Element<'a, Message> {
    let status: Element<'a, Message> = match m.said() {
        Some((text, warn)) => {
            let warn = *warn;
            row![
                icon(if warn { Icon::Warning } else { Icon::Check })
                    .size(14.0)
                    .tone(if warn { Tone::Warning } else { Tone::Success }),
                label::body(text.clone()).style(move |t: &Theme| iced::widget::text::Style {
                    color: Some(if warn {
                        Tokens::of(t).warning
                    } else {
                        Tokens::of(t).muted
                    }),
                }),
            ]
            .spacing(6)
            .align_y(Center)
            .into()
        }
        None => space().into(),
    };
    let mut footer = row![
        container(status).width(Fill),
        button(label::body("Kapat"))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(Event::Close)),
    ]
    .spacing(8)
    .align_y(Center);
    if m.pick.is_some() {
        let can = m.selected.as_deref().is_some_and(|id| m.pickable(lib, id));
        let choose = button(
            row![
                icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                label::body("Seç").style(style::text::on_accent)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([5, 14])
        .style(style::button::primary)
        .on_press_maybe(can.then(|| ev(Event::Choose)));
        footer = footer.push(if can {
            Element::from(choose)
        } else {
            tip(
                choose,
                Tip::new("Önce uygun türde bir sembol seçin"),
                Position::Top,
            )
        });
    }
    footer.into()
}

/// A card: the item's picture and name (the source too in a search), chosen or not.
fn card<'a>(
    lib: &StyleLibrary,
    thumbs: &Thumbs,
    look: &Look<'_>,
    (id, source): (&str, Source),
    (width, height): (f32, f32),
    chosen: bool,
    searching: bool,
) -> Element<'a, Message> {
    let Some((item, _)) = lib.get(id) else {
        return space().into();
    };
    let symbol = symbol_of_item(item);
    // The name on two lines at most (the web's line clamp), cut when longer.
    let name_h = name_height();
    let mut body = Column::new()
        .spacing(6)
        .align_x(Center)
        .push(thumbs.picture(&symbol, None, PICTURE, None, look))
        .push(
            container(
                label::caption(item.name().to_owned())
                    .align_x(iced::alignment::Horizontal::Center)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            )
            .center_x(Fill)
            .height(Length::Fixed(name_h))
            .clip(true),
        );
    if searching {
        body = body.push(source_badge(source, false));
    }
    let face = button(body)
        .padding([7, 7])
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(move |t: &Theme, status: button::Status| card_style(t, status, chosen))
        .on_press(ev(Event::Press(id.to_owned())));
    let mut path = item.path().join(" › ");
    if path.is_empty() {
        path = source.label().to_owned();
    }
    tip(
        face,
        Tip::new(item.name().to_owned()).body(path),
        Position::Bottom,
    )
}

/// The web's `.scard`: the panel colour, a hairline, the accent when chosen.
fn card_style(theme: &Theme, status: button::Status, chosen: bool) -> button::Style {
    let t = Tokens::of(theme);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (color, width) = if chosen {
        (t.accent, 2.0)
    } else if hovered {
        (t.muted, 1.0)
    } else {
        (t.border, 1.0)
    };
    button::Style {
        background: Some(iced::Background::Color(t.surface)),
        text_color: t.text,
        border: Border {
            color,
            width,
            radius: kentos_ui::theme::shape::radius(6.0).into(),
        },
        ..button::Style::default()
    }
}

/// A tree node with its sub-categories, its count, its menu and, renaming, its field.
fn node<'a>(
    m: &Manager,
    lib: &'a StyleLibrary,
    source: Source,
    c: &CategoryNode,
    project_open: bool,
) -> Node<'a, Message> {
    let key = node_key(source, &c.path);
    let count = c.all_items().len();
    let open = m.expanded.contains(&key);
    let kids = !c.children.is_empty();
    let chosen = m.at.0 == source && m.at.1 == c.path;
    let mut n = Node::new(c.name.clone())
        .icon(icon(Icon::Folder).size(14.0).tone(Tone::Muted))
        .cells([count_cell(count)])
        .on_press(ev(Event::Go(source, c.path.clone(), kids)))
        .selected(chosen);
    if kids {
        n = n.expanded(open, ev(Event::Toggle(key)));
    }
    if let Some((s, p, name)) = &m.renaming
        && *s == source
        && *p == c.path
    {
        n = n.editor(rename(
            name,
            |t| ev(Event::RenameInput(t)),
            ev(Event::RenameDone),
            ev(Event::RenameCancel),
        ));
    }
    n = n.menu(category_menu(lib, source, c.path.clone(), project_open));
    if open {
        for child in &c.children {
            n = n.push(node(m, lib, source, child, project_open));
        }
    }
    n
}

/// A node's count, as the web's small grey number.
fn count_cell<'a>(count: usize) -> Element<'a, Message> {
    label::mono_caption(count.to_string())
        .style(style::text::muted)
        .into()
}

/// A node's menu: Yeni alt kategori, Yeniden adlandır, Dışa aktar (n) (`categoryMenu`).
fn category_menu<'a>(
    lib: &'a StyleLibrary,
    source: Source,
    path: Vec<String>,
    project_open: bool,
) -> impl Fn(iced::Point) -> Menu<Message> + 'a {
    let writable = source.editable() && (source != Source::Project || project_open);
    move |_| {
        // Counted when the menu opens: building every node's list at each frame would cost.
        let ids = ids_under(lib, source, &path);
        let name = path
            .last()
            .cloned()
            .unwrap_or_else(|| source.label().to_owned());
        Menu::new()
            .item(
                "Yeni alt kategori",
                writable.then(|| ev(Event::NewCategory(source, path.clone()))),
            )
            .icon(crate::icons::from_web(Some("folderAdd")))
            .item(
                "Yeniden adlandır",
                (writable && !path.is_empty()).then(|| ev(Event::Rename(source, path.clone()))),
            )
            .icon(crate::icons::from_web(Some("edit")))
            .shortcut("F2")
            .separator()
            .item(
                format!("Dışa aktar ({})", ids.len()),
                (!ids.is_empty()).then(|| ev(Event::Export(ids.clone(), name))),
            )
            .icon(crate::icons::from_web(Some("export")))
    }
}

/// The source and category tree.
fn tree_of<'a>(m: &Manager, lib: &'a StyleLibrary, project_open: bool) -> Element<'a, Message> {
    let trees = m.trees(lib);
    let roots: Vec<Node<'a, Message>> = trees
        .iter()
        .map(|(source, cats)| {
            let source = *source;
            let key = node_key(source, &[]);
            let count: usize = cats.iter().map(|c| c.all_items().len()).sum();
            let glyph = match source {
                Source::System => Icon::Lock,
                Source::User => crate::icons::from_web(Some("styles")),
                Source::Project => crate::icons::from_web(Some("save")),
            };
            let kids = !cats.is_empty();
            let open = m.expanded.contains(&key);
            let mut root = Node::new(source.label())
                .icon(icon(glyph).size(14.0).tone(Tone::Muted))
                .cells([count_cell(count)])
                .on_press(ev(Event::Go(source, Vec::new(), kids)))
                .selected(m.at.0 == source && m.at.1.is_empty())
                .menu(category_menu(lib, source, Vec::new(), project_open));
            if kids {
                root = root.expanded(open, ev(Event::Toggle(key)));
            }
            if open {
                for c in cats {
                    root = root.push(node(m, lib, source, c, project_open));
                }
            }
            root
        })
        .collect();
    TreeView::new([
        TreeColumn::new("Ad").width(Fill),
        TreeColumn::new("Öğe").width(44).align_right(),
    ])
    .header(false)
    .extend(roots)
    .empty("Kitaplık boş.")
    .height(Fill)
    .into()
}
