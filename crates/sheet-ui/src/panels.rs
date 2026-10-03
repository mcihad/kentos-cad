//! The left panel in the sheet mode (design §11): the sheets of the book and
//! the items of the sheet in front. The tree shows the top of the drawing
//! order first, a group's items under it; its eye and lock are the core's
//! `Hide` and `Lock`, a row dragged before or after another reorders them.

use iced::widget::{button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill};
use kentos_sheet::model::{Item, Orientation};
use kentos_sheet::profile;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::ContextMenu;
use kentos_ui::widget::Menu;
use kentos_ui::widget::table::Column as TreeColumn;
use kentos_ui::widget::tree_view::{Node, Toggle, TreeView};

use crate::designer::Designer;
use crate::icons;
use crate::message::Message;

/// The panel's width.
pub const WIDTH: f32 = 252.0;

/// An item kind's icon.
pub fn kind_icon(item: &Item) -> Icon {
    match profile::tool_of(item) {
        "map" => icons::MAP,
        "overviewMap" => icons::OVERVIEW,
        "text" => Icon::Type,
        "scaleBar" => icons::SCALE_BAR,
        "northArrow" => icons::NORTH,
        "legend" => Icon::Legend,
        "picture" => icons::PICTURE,
        "shape" => icons::SHAPE,
        "line" => icons::LINE,
        "table" | "attributeTable" => Icon::Table,
        "coordinateList" => icons::COORDINATES,
        "titleBlock" => icons::TITLE_BLOCK,
        "border" => icons::BORDER,
        "group" => icons::GROUP,
        _ => icons::LAYOUT,
    }
}

fn section<'a>(
    title: &'a str,
    meta: String,
    action: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut r = row![
        label::caption(title).font(kentos_ui::theme::typography::ui_strong()),
        label::caption(meta).style(style::text::muted),
        space::horizontal(),
    ]
    .spacing(6)
    .align_y(Center);
    if let Some(a) = action {
        r = r.push(a);
    }
    container(r)
        .padding([6, 10])
        .width(Fill)
        .style(style::container::header)
        .into()
}

impl Designer {
    fn item_node<'a>(&'a self, item: &'a Item, rows: &[String]) -> Node<'a, Message> {
        let id = rows
            .iter()
            .position(|r| *r == item.id)
            .unwrap_or(usize::MAX);
        let selected = self.selection.contains(&item.id);
        let mut node = Node::new(item.name.as_str())
            .icon(icon(kind_icon(item)).size(16.0))
            .id(id)
            .selected(selected)
            .muted(item.hidden)
            .on_press(Message::TreeClick(item.id.clone(), false))
            .toggle(Toggle::visible(
                !item.hidden,
                Message::Hidden(item.id.clone(), !item.hidden),
            ))
            .toggle(Toggle::locked(
                item.locked,
                Message::Locked(item.id.clone(), !item.locked),
            ));
        let iid = item.id.clone();
        node = node.menu(move |_| {
            Menu::new()
                .item("Yalnız bunu seç", Message::TreeClick(iid.clone(), false))
                .item("Seçime ekle / çıkar", Message::TreeClick(iid.clone(), true))
                .separator()
                .item(
                    "Öne getir",
                    Message::Order(kentos_sheet::ops::ReorderTo::Front),
                )
                .item(
                    "Arkaya gönder",
                    Message::Order(kentos_sheet::ops::ReorderTo::Back),
                )
                .separator()
                .item("Çoğalt", Message::Duplicate)
                .item("Sil", Message::Delete)
        });
        if matches!(item.kind, kentos_sheet::kinds::ItemKind::Group(_)) {
            let open = self.expanded.contains(&item.id);
            node = node
                .folder()
                .expanded(open, Message::TreeExpand(item.id.clone()));
            for c in self
                .items()
                .iter()
                .rev()
                .filter(|c| c.group.as_deref() == Some(item.id.as_str()))
            {
                node = node.push(self.item_node(c, rows));
            }
        }
        node
    }

    /// The sheets and the item tree.
    pub fn left_panel(&self) -> Element<'_, Message> {
        // The sheets.
        let add = button(icon(Icon::Plus).size(14.0))
            .on_press(Message::NewSheet)
            .padding(3)
            .style(style::button::ghost);
        let mut sheets = column![].spacing(1).padding([4, 4]);
        for (i, s) in self.book.sheets.iter().enumerate() {
            let open = self.open.as_deref() == Some(s.id.as_str());
            let page = format!(
                "{} {}",
                crate::gallery::paper_text(&kentos_sheet::template::PaperChoice {
                    paper: s.page.paper,
                    orientation: s.page.orientation
                })
                .split(' ')
                .next()
                .unwrap_or_default(),
                if s.page.orientation == Orientation::Landscape {
                    "yatay"
                } else {
                    "dikey"
                }
            );
            let mut line = row![
                icon(icons::LAYOUT).size(16.0),
                label::body(s.name.as_str()).width(Fill),
            ]
            .spacing(8)
            .align_y(Center);
            // The web's “Yeni sürüm var” badge in the sheets' list.
            if self.template_newer(s) {
                line = line.push(label::caption("Yeni sürüm var").style(style::text::info));
            }
            let face = button(line.push(label::caption(page).style(style::text::muted)))
                .on_press(Message::Tab(i + 1))
                .width(Fill)
                .padding([4, 8])
                .style(style::button::row(open));
            let id = s.id.clone();
            sheets = sheets.push(ContextMenu::new(face, move |_| {
                Menu::new()
                    .item("Aç", Message::Tab(i + 1))
                    .item("Çoğalt", Message::DuplicateSheet(id.clone()))
                    .separator()
                    .item("Sil", Message::RemoveSheet(id.clone()))
                    .danger()
            }));
        }
        if self.book.sheets.is_empty() {
            sheets = sheets.push(
                container(label::muted(
                    "Henüz pafta yok. + ile ya da Şablondan… ile ekleyin.",
                ))
                .padding(8),
            );
        }

        // The items.
        let rows = self.tree_rows();
        let items = self.items();
        let mut tree = TreeView::new([TreeColumn::new("Ad").width(Fill)])
            .header(false)
            .empty("Bu paftada öğe yok.")
            .on_move(Message::TreeMove);
        for i in items.iter().rev().filter(|i| i.group.is_none()) {
            tree = tree.push(self.item_node(i, &rows));
        }
        let count = format!("{}", items.len());

        column![
            section(
                "Paftalar",
                self.book.sheets.len().to_string(),
                Some(add.into())
            ),
            sheets,
            section("Öğeler", count, None),
            scrollable(container(tree).padding([2, 2])).height(Fill),
        ]
        .width(WIDTH)
        .height(Fill)
        .into()
    }
}
