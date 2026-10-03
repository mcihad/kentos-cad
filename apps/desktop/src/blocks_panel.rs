//! Bloklar (the web's `ui/blocks/BlocksPanel.ts`, docs/adr/0144 §6): the
//! drawing's blocks in the dock's upper slot, a tab beside Katmanlar and
//! İşlemler. Each row shows the block as it is drawn (its outline from the
//! geometry store at the origin, fitted), its name and description, and how
//! many inserts place it in the drawing; the block Blok ekle places next is
//! marked. A click chooses it, a second click soon after (or Enter) places it
//! (Blok ekle), F2 renames it in the row, Delete deletes an unused one; the
//! row's menu also changes the base point, redefines it from the selection
//! and selects its inserts. Beside the search, buttons make a block (Blok
//! oluştur), place the chosen one and purge the unused ones. Every change goes through
//! `cad.blocks.edit` in the words of `kentos_interaction::blocks`, the web's.

use std::time::{Duration, Instant};

use iced::keyboard::key::Named;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Canvas, Column, button, column, container, row, text, tooltip};
use iced::{Center, Element, Fill, Point, Rectangle, Renderer, Size, Task, Theme, mouse};
use kentos_contracts::BlockId;
use kentos_contracts::blocks::{Placements, name_key};
use kentos_domain::Slot;
use kentos_interaction::blocks::{self as actions, Said};
use kentos_interaction::pick::PickPoint;
use kentos_interaction::{Level, Vec2};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{ContextMenu, Menu, SearchBox, Tip, VirtualList, tip, tree_view};
use kentos_ui::{label, style};

use crate::app::{App, Message, Panel};
use crate::keys::KeyPress;

/// A second press this soon on the same row places the block (the layer tree's).
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// The search box's id.
const SEARCH: &str = "blok-ara";
/// A row's height and its picture's side, at the body text's size (the web's `--blk-row-h`).
const ROW: f32 = 46.0;
const THUMB: f32 = 34.0;

/// What the panel keeps while the app runs.
#[derive(Debug, Default)]
pub struct PanelState {
    query: String,
    /// The row chosen with the mouse or the keys: the keyboard's row.
    focused: Option<BlockId>,
    /// The last row pressed and when: a second press soon after places it.
    pressed: Option<(BlockId, Instant)>,
    /// The row renamed in place, and its text.
    renaming: Option<(BlockId, String)>,
    /// A point asked for on the drawing, and what it is for.
    picking: Option<Picking>,
    /// Whether the list has the keyboard (a row was pressed).
    pub(crate) keyboard: bool,
}

/// What a point shown on the drawing is for.
#[derive(Debug, Clone)]
enum Picking {
    /// The new base point, shown on this insert of the block.
    Rebase { block: BlockId, insert: Slot },
    /// The base point of the block made of these objects.
    Redefine { block: BlockId, uids: Vec<String> },
    /// A row's place in Blok öznitelikleri, shown on this insert (block_attributes.rs).
    AttributePlace { insert: Slot, row: usize },
}

#[derive(Debug, Clone)]
pub enum Event {
    Search(String),
    Pressed(BlockId),
    Insert(BlockId),
    Rename(BlockId),
    RenameInput(String),
    RenameDone,
    RenameCancel,
    Rebase(BlockId),
    Redefine(BlockId),
    /// Blok öznitelikleri for the block (block_attributes.rs).
    Attributes(BlockId),
    SelectInserts(BlockId),
    Remove(BlockId),
}

fn msg(event: Event) -> Message {
    Message::BlocksPanel(event)
}

/// A listed block: its id, name, description and placements.
struct Listed {
    id: BlockId,
    name: String,
    about: Option<String>,
    placed: Placements,
}

impl App {
    /// The drawing's blocks as listed now: in the drawing's order, those the
    /// search finds (name or description, Turkish case folded).
    fn listed_blocks(&self) -> Vec<Listed> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let model = &doc.model;
        let placed = actions::placements_of(model);
        let query = name_key(self.blocks_panel.query.trim());
        model
            .blocks()
            .iter()
            .zip(placed)
            .filter(|(b, _)| {
                query.is_empty()
                    || name_key(&b.name).contains(&query)
                    || b.description
                        .as_deref()
                        .is_some_and(|d| name_key(d).contains(&query))
            })
            .map(|(b, placed)| Listed {
                id: b.id,
                name: b.name.clone(),
                about: b.description.clone(),
                placed,
            })
            .collect()
    }

    fn say_all(&mut self, said: Vec<Said>) {
        for (level, line) in said {
            self.say(level, line);
        }
    }

    /// `block.panel`: the dock shows the Bloklar tab.
    pub(crate) fn show_blocks_panel(&mut self) {
        if !self.right_panel_shown() {
            self.toggle_right_panel();
        }
        self.docks
            .update(kentos_ui::widget::docking::Event::Selected(Panel::Blocks));
    }

    /// `block.purge`: every block no insert uses, in one step.
    pub(crate) fn purge_blocks(&mut self) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let said = actions::purge(&mut doc.model);
        self.say_all(said);
    }

    pub(crate) fn blocks_panel_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Search(text) => {
                self.blocks_panel.query = text;
                self.blocks_panel.keyboard = false;
            }
            Event::Pressed(id) => {
                let now = Instant::now();
                let double = self
                    .blocks_panel
                    .pressed
                    .is_some_and(|(last, at)| last == id && now.duration_since(at) <= DOUBLE_CLICK);
                self.blocks_panel.pressed = (!double).then_some((id, now));
                self.blocks_choose(id);
                if double {
                    return self.blocks_insert(id);
                }
            }
            Event::Insert(id) => return self.blocks_insert(id),
            Event::Rename(id) => return self.blocks_rename(id),
            Event::RenameInput(text) => {
                if let Some((_, name)) = self.blocks_panel.renaming.as_mut() {
                    *name = text;
                }
            }
            Event::RenameDone => self.blocks_rename_done(),
            Event::RenameCancel => self.blocks_panel.renaming = None,
            Event::Rebase(id) => self.blocks_rebase(id),
            Event::Redefine(id) => self.blocks_redefine(id),
            Event::Attributes(id) => self.open_block_attributes(id),
            Event::SelectInserts(id) => {
                if let Some(doc) = &self.document {
                    let (level, line) =
                        actions::select_inserts(&doc.model, &mut self.selection, id);
                    self.say(level, line);
                }
            }
            Event::Remove(id) => self.blocks_remove(id),
        }
        Task::none()
    }

    /// A row chosen: it is the keyboard's row and the block Blok ekle places next.
    fn blocks_choose(&mut self, id: BlockId) {
        self.blocks_panel.focused = Some(id);
        self.blocks_panel.keyboard = true;
        self.layers_keyboard = false;
        self.memory.block_insert = Some(id);
    }

    /// Blok ekle with this block.
    fn blocks_insert(&mut self, id: BlockId) -> Task<Message> {
        self.memory.block_insert = Some(id);
        self.blocks_panel.keyboard = false;
        self.start_tool(kentos_interaction::block_insert::ID)
    }

    /// The name becomes a text box in its row.
    fn blocks_rename(&mut self, id: BlockId) -> Task<Message> {
        let Some(name) = self
            .document
            .as_ref()
            .and_then(|d| d.model.block(id))
            .map(|b| b.name.clone())
        else {
            return Task::none();
        };
        self.blocks_panel.renaming = Some((id, name));
        let field = iced::widget::Id::new(tree_view::RENAME);
        Task::batch([
            iced::widget::operation::focus(field.clone()),
            iced::widget::operation::select_all(field),
        ])
    }

    /// Enter or a click elsewhere: a new name, trimmed, when it is one.
    fn blocks_rename_done(&mut self) {
        let Some((id, text)) = self.blocks_panel.renaming.take() else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let name = text.trim();
        let was = doc.model.block(id).map(|b| b.name.clone());
        if name.is_empty() || was.as_deref() == Some(name) {
            return;
        }
        let (_, said) = actions::rename(&mut doc.model, id, name);
        self.say_all(said);
    }

    fn blocks_remove(&mut self, id: BlockId) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        if let Some(why) = actions::remove_refusal(&doc.model, id) {
            return self.warn(why);
        }
        let said = actions::remove(&mut doc.model, id);
        self.say_all(said);
    }

    /// Taban noktasını değiştir: the new base point is shown on an insert of
    /// the block (the selected one, or its only one).
    fn blocks_rebase(&mut self, id: BlockId) {
        let Some(doc) = &self.document else {
            return;
        };
        let name = doc
            .model
            .block(id)
            .map(|b| b.name.clone())
            .unwrap_or_default();
        match actions::rebase_insert(&doc.model, &self.selection, id) {
            Err(why) => self.warn(why),
            Ok(insert) => {
                self.blocks_panel.picking = Some(Picking::Rebase { block: id, insert });
                self.blocks_pick(
                    format!("Taban noktası: {name}"),
                    format!("“{name}” için yeni taban noktası"),
                );
            }
        }
    }

    /// Seçili nesnelerle yeniden tanımla: the base point is shown next.
    fn blocks_redefine(&mut self, id: BlockId) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let name = model.block(id).map(|b| b.name.clone()).unwrap_or_default();
        let uids: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        if uids.is_empty() {
            let why = actions::redefine_refusal(model, id);
            return self.warn(why);
        }
        self.blocks_panel.picking = Some(Picking::Redefine { block: id, uids });
        self.blocks_pick(
            format!("Yeniden tanımla: {name}"),
            format!("“{name}” için taban noktası"),
        );
    }

    /// Blok öznitelikleri's Sahneden seç: a row's place shown on `insert`;
    /// the window comes back with it (block_attributes.rs).
    pub(crate) fn blocks_pick_place(&mut self, insert: Slot, row: usize, name: String) {
        self.blocks_panel.picking = Some(Picking::AttributePlace { insert, row });
        self.blocks_pick(
            format!("Öznitelik yeri: {name}"),
            format!("{}. özniteliğin yeri", row + 1),
        );
    }

    /// Asks for a point on the drawing (Çizimden), as the web's `PickPointTool`.
    fn blocks_pick(&mut self, command: String, field: String) {
        self.say(Level::Command, command);
        self.blocks_panel.keyboard = false;
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new("", field)));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// The point a block action asked for (input.rs); false when none asked.
    pub(crate) fn blocks_picked(&mut self, p: Option<Vec2>) -> bool {
        let Some(picking) = self.blocks_panel.picking.take() else {
            return false;
        };
        // A row's place goes back to Blok öznitelikleri, which comes back even when none was shown.
        if let Picking::AttributePlace { insert, row } = picking {
            let local = match (p, &self.document) {
                (Some(p), Some(doc)) => {
                    self.spatial.sync(&doc.model);
                    self.spatial
                        .store()
                        .insert_local(f64::from(insert.0), p)
                        .map(|q| kentos_contracts::Vec2 { x: q.x, y: q.y })
                }
                _ => None,
            };
            self.block_attributes_picked(row, local);
            return true;
        }
        let (Some(p), Some(doc)) = (p, self.document.as_mut()) else {
            return true;
        };
        let said = match picking {
            Picking::Rebase { block, insert } => {
                self.spatial.sync(&doc.model);
                match self.spatial.store().insert_local(f64::from(insert.0), p) {
                    Some(base) => actions::rebase(&mut doc.model, block, base),
                    None => Vec::new(),
                }
            }
            Picking::Redefine { block, uids } => actions::redefine(&mut doc.model, block, uids, p),
            Picking::AttributePlace { .. } => Vec::new(),
        };
        self.say_all(said);
        true
    }

    /// The list's keys while it has the keyboard: ↑ ↓ choose, Enter places,
    /// F2 renames, Delete deletes, Esc gives the keyboard back to the drawing.
    pub(crate) fn blocks_key(&mut self, press: &KeyPress) -> Option<Task<Message>> {
        if self.blocks_panel.renaming.is_some() {
            return None;
        }
        let rows = self.listed_blocks();
        if rows.is_empty() {
            return None;
        }
        let at = rows
            .iter()
            .position(|r| Some(r.id) == self.blocks_panel.focused);
        let step = |d: isize| {
            let n = rows.len() as isize;
            let i = at.map_or(0, |i| i as isize + d).clamp(0, n - 1);
            rows.get(i as usize).map(|r| r.id)
        };
        let id = self.blocks_panel.focused;
        match press.named()? {
            Named::ArrowDown => step(1).map(|id| self.blocks_choose(id)),
            Named::ArrowUp => step(-1).map(|id| self.blocks_choose(id)),
            Named::Home => rows.first().map(|r| r.id).map(|id| self.blocks_choose(id)),
            Named::End => rows.last().map(|r| r.id).map(|id| self.blocks_choose(id)),
            Named::Enter => return id.map(|id| self.blocks_insert(id)),
            Named::F2 => return id.map(|id| self.blocks_rename(id)),
            Named::Delete => id.map(|id| self.blocks_remove(id)),
            Named::Escape => {
                self.blocks_panel.keyboard = false;
                Some(())
            }
            _ => return None,
        }?;
        Some(Task::none())
    }

    /// The header's meta: how many blocks the drawing has (the head holds the dock's tabs).
    pub(crate) fn blocks_panel_actions(&self) -> Element<'_, Message> {
        let count = self.document.as_ref().map_or(0, |d| d.model.blocks().len());
        let meta = if count > 0 {
            format!("{count} blok")
        } else {
            String::new()
        };
        label::caption(meta).into()
    }

    /// A button beside the search: the command's icon, its title in the tip.
    fn blocks_button(&self, id: &'static str) -> Element<'_, Message> {
        let command = crate::catalog::catalog().get(id);
        let title = command.map_or(id, |c| c.title);
        let glyph = command.map_or(Icon::Button, |c| c.icon);
        tip(
            button(icon(glyph).size(16.0))
                .on_press_maybe(self.available(id).then_some(Message::Run(id)))
                .padding([4, 5])
                .style(style::button::ghost),
            Tip::new(title.to_owned()),
            tooltip::Position::Bottom,
        )
    }

    /// The panel's body: the search box and the list.
    pub(crate) fn blocks_panel_view(&self) -> Element<'_, Message> {
        let search = SearchBox::new(self.blocks_panel.query.clone(), "Blok ara", |t| {
            msg(Event::Search(t))
        })
        .id(SEARCH)
        .fill()
        .height(28.0);
        // Blok oluştur, Blok ekle and Blokları temizle beside the search (the web's toolbar).
        let search = container(
            row![
                search,
                self.blocks_button("tool.blockDefine"),
                self.blocks_button("tool.blockInsert"),
                self.blocks_button("block.purge"),
            ]
            .spacing(2)
            .align_y(Center),
        )
        .padding([6, 8])
        .width(Fill);
        let rows = self.listed_blocks();
        if rows.is_empty() {
            let words = if self.blocks_panel.query.trim().is_empty() {
                "Çizimde blok yok. Seçili nesnelerden Blok oluştur ile bir blok tanımlayın."
            } else {
                "Aramayla eşleşen blok yok. Başka bir ad deneyin."
            };
            return column![
                search,
                container(label::caption(words))
                    .padding([8, 12])
                    .width(Fill)
            ]
            .into();
        }
        let chosen = self.memory.block_insert;
        let focused = self.blocks_panel.focused;
        let reveal = rows.iter().position(|r| Some(r.id) == focused);
        let list = VirtualList::new(rows.len(), typography::scaled(ROW), move |i| {
            let r = &rows[i];
            self.block_row(r, Some(r.id) == chosen, Some(r.id) == focused)
        })
        .reveal(reveal)
        .height(Fill);
        column![search, list].into()
    }

    /// One row: the picture, the name over the description, the count.
    fn block_row<'a>(&'a self, r: &Listed, chosen: bool, focused: bool) -> Element<'a, Message> {
        let id = r.id;
        let paths = self.spatial.store().insert_outlines(
            &id.to_text(),
            Vec2::new(0.0, 0.0),
            1.0,
            0.0,
            false,
        );
        let side = typography::scaled(THUMB);
        let picture = container(Canvas::new(Thumb { paths }).width(side).height(side))
            .style(move |theme: &Theme| thumb_tile(theme, chosen));
        let name: Element<'a, Message> = match &self.blocks_panel.renaming {
            Some((renamed, text)) if *renamed == id => tree_view::rename(
                text,
                |t| msg(Event::RenameInput(t)),
                msg(Event::RenameDone),
                msg(Event::RenameCancel),
            ),
            _ => {
                let font = if chosen {
                    typography::ui_strong()
                } else {
                    typography::ui()
                };
                text(r.name.clone())
                    .font(font)
                    .size(typography::body())
                    .wrapping(text::Wrapping::None)
                    .into()
            }
        };
        let mut words = Column::new().spacing(1).width(Fill).push(name);
        if let Some(about) = &r.about {
            words = words.push(
                label::caption(about.clone())
                    .wrapping(text::Wrapping::None)
                    .style(|t: &Theme| text::Style {
                        color: Some(Tokens::of(t).faint),
                    }),
            );
        }
        let count = label::caption(r.placed.drawing.to_string());
        let face = row![picture, words, count]
            .spacing(8)
            .padding([0, 8])
            .align_y(Center)
            .height(Fill);
        let face = button(face)
            .on_press(msg(Event::Pressed(id)))
            .padding(0)
            .width(Fill)
            .height(Fill)
            .style(style::button::row(focused));
        let body = [r.about.clone(), Some(actions::placed_text(r.placed))]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
        let face = tip(
            face,
            Tip::new(r.name.clone()).body(body),
            tooltip::Position::Left,
        );
        ContextMenu::new(face, move |_| self.block_menu(id)).into()
    }

    /// The row's menu (the web's `menuFor`).
    fn block_menu(&self, id: BlockId) -> Menu<Message> {
        let Some(doc) = &self.document else {
            return Menu::new();
        };
        let model = &doc.model;
        let via = actions::rebase_insert(model, &self.selection, id);
        let refusal = actions::remove_refusal(model, id);
        let inserts = actions::inserts_of(model, id).len();
        let selected = !self.selection.is_empty();
        let mut menu = Menu::new()
            .item("Blok ekle", msg(Event::Insert(id)))
            .icon(crate::icons::from_web(Some("blockInsert")))
            .shortcut("Enter")
            .item("Yeniden adlandır", msg(Event::Rename(id)))
            .icon(crate::icons::from_web(Some("edit")))
            .shortcut("F2")
            .item("Öznitelikler…", msg(Event::Attributes(id)))
            .icon(crate::icons::from_web(Some("blockAttributes")));
        let attributes = model.block(id).map_or(0, |b| b.attributes.len());
        if attributes > 0 {
            menu = menu.hint(attributes.to_string());
        }
        menu = menu
            .item(
                "Taban noktasını değiştir…",
                via.is_ok().then(|| msg(Event::Rebase(id))),
            )
            .icon(crate::icons::from_web(Some("blockBase")));
        if let Err(why) = via {
            menu = menu.detail(why);
        }
        menu = menu
            .item(
                "Seçili nesnelerle yeniden tanımla…",
                selected.then(|| msg(Event::Redefine(id))),
            )
            .icon(crate::icons::from_web(Some("blockDefine")));
        if !selected {
            menu = menu.detail("Önce bloğun yeni nesnelerini seçin.");
        }
        menu = menu
            .item(
                "Yerleştirmelerini seç",
                (inserts > 0).then(|| msg(Event::SelectInserts(id))),
            )
            .icon(crate::icons::from_web(Some("select")))
            .hint(inserts.to_string())
            .separator()
            .item("Sil", refusal.is_none().then(|| msg(Event::Remove(id))))
            .icon(crate::icons::from_web(Some("trash")))
            .shortcut("Delete");
        if let Some(why) = refusal {
            menu = menu.detail(why);
        }
        menu
    }
}

/// The picture's tile: the header's ground, its edge the accent on the block Blok ekle places next.
fn thumb_tile(theme: &Theme, chosen: bool) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(t.header.into()),
        border: iced::Border {
            color: if chosen { t.accent } else { t.border },
            width: 1.0,
            radius: style::button::radius().into(),
        },
        ..container::Style::default()
    }
}

/// A block's outline paths (`flags, n, x0, y0, …`; flags 0 open, 1 closed,
/// 2 a mark) fitted into the canvas, north up, 3 pixels in from its edges;
/// marks are small squares (the web's `drawThumb`).
struct Thumb {
    paths: Vec<f64>,
}

impl<M> canvas::Program<M> for Thumb {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let paths = &self.paths;
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        let mut i = 0;
        while i + 1 < paths.len() {
            let n = paths[i + 1] as usize;
            for k in 0..n {
                let (x, y) = (paths[i + 2 + 2 * k], paths[i + 3 + 2 * k]);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            i += 2 + 2 * n;
        }
        if !min_x.is_finite() {
            return vec![frame.into_geometry()];
        }
        let side = f64::from(bounds.width.min(bounds.height));
        let pad = 3.0;
        let span = (max_x - min_x).max(max_y - min_y);
        let span = if span > 0.0 { span } else { 1.0 };
        let s = (side - 2.0 * pad) / span;
        let (cx, cy) = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let (w, h) = (f64::from(bounds.width), f64::from(bounds.height));
        let at = |x: f64, y: f64| {
            Point::new(
                (w / 2.0 + (x - cx) * s) as f32,
                (h / 2.0 - (y - cy) * s) as f32,
            )
        };
        let ink = Tokens::of(theme).muted;
        let stroke = Stroke::default()
            .with_color(ink)
            .with_width(1.0)
            .with_line_join(canvas::LineJoin::Round)
            .with_line_cap(canvas::LineCap::Round);
        let mut i = 0;
        while i + 1 < paths.len() {
            let flags = paths[i] as u32;
            let n = paths[i + 1] as usize;
            if flags == 2 && n > 0 {
                let p = at(paths[i + 2], paths[i + 3]);
                frame.fill_rectangle(Point::new(p.x - 1.5, p.y - 1.5), Size::new(3.0, 3.0), ink);
            } else if n > 0 {
                let path = Path::new(|b| {
                    for k in 0..n {
                        let p = at(paths[i + 2 + 2 * k], paths[i + 3 + 2 * k]);
                        if k == 0 {
                            b.move_to(p);
                        } else {
                            b.line_to(p);
                        }
                    }
                    if flags == 1 {
                        b.close();
                    }
                });
                frame.stroke(&path, stroke);
            }
            i += 2 + 2 * n;
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{BlockId, Entity};
    use kentos_domain::Slot;
    use kentos_interaction::Vec2;

    use super::Event;
    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, last_said};

    fn send(app: &mut App, event: Event) {
        let _ = app.update(Message::BlocksPanel(event));
    }

    /// The sample drawing's first two objects on layers that are not locked
    /// made into “Vana” from (10, 20) through Blok oluştur's window, put in
    /// their place: the block and the insert that took their place.
    fn with_block(replace: bool) -> (App, BlockId, Vec<Slot>) {
        let mut app = app_with_drawing();
        let model = &app.document.as_ref().expect("open").model;
        let slots: Vec<Slot> = model
            .entities()
            .filter(|e| !model.layers().is_locked(&e.base().layer_id))
            .take(2)
            .map(|e| Slot(e.base().id))
            .collect();
        app.selection.set(slots.clone());
        app.open_block_define(Vec2::new(10.0, 20.0));
        let _ = app.update(Message::Blocks(crate::blocks::Event::Name("Vana".into())));
        if !replace {
            let _ = app.update(Message::Blocks(crate::blocks::Event::Replace));
        }
        let _ = app.update(Message::Blocks(crate::blocks::Event::Create));
        let model = &app.document.as_ref().expect("open").model;
        let id = model.blocks().last().expect("defined").id;
        (app, id, slots)
    }

    fn names(app: &App) -> Vec<String> {
        app.listed_blocks().into_iter().map(|r| r.name).collect()
    }

    #[test]
    fn the_list_shows_each_block_with_its_placements_and_the_search_finds_names() {
        let (mut app, _, _) = with_block(true);
        let rows = app.listed_blocks();
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].name.as_str(), rows[0].placed.drawing), ("Vana", 1));
        send(&mut app, Event::Search("VAN".into()));
        assert_eq!(names(&app), ["Vana"]);
        send(&mut app, Event::Search("rögar".into()));
        assert!(names(&app).is_empty());
        // The empty list says why (the web's words).
        send(&mut app, Event::Search(String::new()));
        assert_eq!(names(&app), ["Vana"]);
    }

    #[test]
    fn a_click_chooses_the_block_and_a_second_one_soon_after_places_it() {
        let (mut app, id, _) = with_block(true);
        app.memory.block_insert = None;
        send(&mut app, Event::Pressed(id));
        assert_eq!(app.memory.block_insert, Some(id));
        assert!(app.blocks_panel.keyboard, "the list has the keyboard");
        assert_ne!(app.session.tool_id(), kentos_interaction::block_insert::ID);
        send(&mut app, Event::Pressed(id));
        assert_eq!(app.session.tool_id(), kentos_interaction::block_insert::ID);
        assert!(app.session.prompt().text().contains("“Vana”"));
    }

    #[test]
    fn a_block_is_renamed_in_its_row_and_a_taken_name_is_refused() {
        let (mut app, id, _) = with_block(true);
        send(&mut app, Event::Rename(id));
        assert!(app.blocks_panel.renaming.is_some());
        send(&mut app, Event::RenameInput("  Su vanası ".into()));
        send(&mut app, Event::RenameDone);
        assert_eq!(names(&app), ["Su vanası"]);
        assert_eq!(last_said(&app), "“Vana” bloğunun adı “Su vanası” oldu.");
        // Esc keeps the name.
        send(&mut app, Event::Rename(id));
        send(&mut app, Event::RenameInput("Başka".into()));
        send(&mut app, Event::RenameCancel);
        assert_eq!(names(&app), ["Su vanası"]);
    }

    #[test]
    fn a_placed_block_is_not_deleted_and_the_unused_are_purged() {
        let (mut app, used, _) = with_block(true);
        send(&mut app, Event::Remove(used));
        assert_eq!(
            last_said(&app),
            "“Vana” bloğu kullanılıyor (çizimde 1 yerleştirmesi); önce onları silin ya da patlatın."
        );
        assert_eq!(names(&app), ["Vana"]);
        // A second block, its objects kept: no insert places it.
        let model = &app.document.as_ref().expect("open").model;
        let slot = model
            .entities()
            .find(|e| {
                !matches!(e, Entity::Insert(_)) && !model.layers().is_locked(&e.base().layer_id)
            })
            .map(|e| Slot(e.base().id))
            .expect("an object");
        app.selection.set([slot]);
        app.open_block_define(Vec2::new(0.0, 0.0));
        let _ = app.update(Message::Blocks(crate::blocks::Event::Replace));
        let _ = app.update(Message::Blocks(crate::blocks::Event::Create));
        assert_eq!(names(&app), ["Vana", "Blok 1"]);
        let _ = app.update(Message::Run("block.purge"));
        assert_eq!(names(&app), ["Vana"]);
        assert_eq!(last_said(&app), "1 kullanılmayan blok silindi.");
        let _ = app.update(Message::Run("block.purge"));
        assert_eq!(last_said(&app), "Kullanılmayan blok yok.");
    }

    #[test]
    fn a_new_base_point_is_shown_on_the_insert_and_the_block_is_redefined_from_the_selection() {
        let (mut app, id, _) = with_block(true);
        // The insert at (10, 20), unscaled: a point shown on it is the definition's own.
        send(&mut app, Event::Rebase(id));
        assert_eq!(app.session.tool_id(), kentos_interaction::pick::ID);
        assert!(app.blocks_picked(Some(Vec2::new(12.0, 21.0))));
        let model = &app.document.as_ref().expect("open").model;
        let base = model.block(id).expect("kept").base;
        assert_eq!((base.x, base.y), (12.0, 21.0));
        assert_eq!(last_said(&app), "“Vana” bloğunun taban noktası değişti.");
        // Nothing selected: the redefinition asks for the objects first.
        app.selection.clear();
        send(&mut app, Event::Redefine(id));
        assert_eq!(
            last_said(&app),
            "“Vana” bloğunun yeni nesnelerini önce seçin."
        );
        // Another object, the base point shown next: the block is made of it now.
        let model = &app.document.as_ref().expect("open").model;
        let other = model
            .entities()
            .find(|e| {
                !matches!(e, Entity::Insert(_)) && !model.layers().is_locked(&e.base().layer_id)
            })
            .map(|e| Slot(e.base().id))
            .expect("an object");
        app.selection.set([other]);
        send(&mut app, Event::Redefine(id));
        assert!(app.blocks_picked(Some(Vec2::new(0.0, 0.0))));
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(model.block(id).expect("kept").entities.len(), 1);
        assert_eq!(
            last_said(&app),
            "“Vana” bloğu 1 nesneyle yeniden tanımlandı."
        );
        // A point nobody asked for is the calculators'.
        assert!(!app.blocks_picked(Some(Vec2::new(0.0, 0.0))));
    }

    #[test]
    fn the_base_point_needs_an_insert_to_be_shown_on() {
        let (mut app, id, _) = with_block(false);
        send(&mut app, Event::Rebase(id));
        assert_eq!(
            last_said(&app),
            "“Vana” bloğu çizimde yerleştirilmemiş: yeni taban noktası bir yerleştirmesinde gösterilir. Önce Blok ekle ile yerleştirin."
        );
        assert_ne!(app.session.tool_id(), kentos_interaction::pick::ID);
    }
}
