//! The SVG editor's shape list (the web's `svgObjects.ts`), front on top:
//! the eye hides a shape, the lock keeps it from being picked or moved on
//! the canvas, a double click on the name renames it (Enter keeps, a click
//! elsewhere keeps too), and dragging a row moves the shape in the stack
//! (a line shows where it goes).

use iced::widget::{Column, button, column, container, mouse_area, row, space, text_input};
use iced::{Center, Element, Fill, Theme};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};

use super::change;
use super::doc::{id_of, shape_name};
use super::state::SvgEditor;
use crate::app::Message;

/// A row's height at the default text size.
const ROW: f32 = 28.0;

pub fn row_height() -> f32 {
    typography::scaled(ROW)
}

/// A row being dragged: its shape, where the drag began and the gap it would drop into.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drag {
    pub id: String,
    pub y0: Option<f32>,
    pub active: bool,
    /// The gap, 0 above the first (front) row.
    pub drop: Option<usize>,
}

impl SvgEditor {
    /// A row pressed: it is chosen (Shift adds or takes away), and a drag may begin.
    pub fn row_pressed(&mut self, id: &str, shift: bool) {
        self.finish_rename(true);
        if shift {
            let mut next = self.selection.clone();
            if next.iter().any(|s| s == id) {
                next.retain(|s| s != id);
            } else {
                next.push(id.to_owned());
            }
            self.select(next);
        } else {
            self.select(vec![id.to_owned()]);
        }
        self.list_drag = Some(Drag {
            id: id.to_owned(),
            ..Drag::default()
        });
    }

    /// The pointer over the list while a row is held (y in the list's content).
    pub fn list_moved(&mut self, y: f32) {
        let n = self.doc.shapes.len();
        let Some(d) = &mut self.list_drag else {
            return;
        };
        let y0 = *d.y0.get_or_insert(y);
        if !d.active && (y - y0).abs() < 4.0 {
            return;
        }
        d.active = true;
        d.drop = Some(((y / row_height()) + 0.5).floor().clamp(0.0, n as f32) as usize);
    }

    /// The row let go: the shape moves to the gap (one undo step).
    pub fn list_released(&mut self) {
        let Some(d) = self.list_drag.take() else {
            return;
        };
        let (true, Some(drop)) = (d.active, d.drop) else {
            return;
        };
        let n = self.doc.shapes.len();
        let Some(from) = self.doc.shapes.iter().position(|s| id_of(s) == d.id) else {
            return;
        };
        // Rows run front to back: gap k sits above row k, over the shape at n − 1 − k.
        let mut to = n - drop.min(n);
        if from < to {
            to -= 1;
        }
        if to == from {
            return;
        }
        let key = format!("order:{}", d.id);
        self.edit(&key, move |ed| {
            let s = ed.doc.shapes.remove(from);
            let at = to.min(ed.doc.shapes.len());
            ed.doc.shapes.insert(at, s);
        });
    }

    /// Hides or shows, locks or unlocks a shape.
    pub fn flip_flag(&mut self, id: &str, key: &'static str) {
        let id = id.to_owned();
        self.edit(&format!("{key}:{id}"), move |ed| {
            if let Some(s) = ed.doc.shapes.iter_mut().find(|s| id_of(s) == id) {
                if s.is(key) {
                    s.set_undefined(key);
                } else {
                    s.set(key, kentos_geometry_core::api::json::Json::Bool(true));
                }
            }
        });
    }

    pub fn start_rename(&mut self, id: &str) {
        let Some(s) = self.doc.shape(id) else {
            return;
        };
        self.renaming = Some((id.to_owned(), s.text("name").unwrap_or("").to_owned()));
    }

    /// The rename ends; `keep` writes the name (none when blank).
    pub fn finish_rename(&mut self, keep: bool) {
        let Some((id, text)) = self.renaming.take() else {
            return;
        };
        let v = kentos_processing::text::js_trim(&text).to_owned();
        let now = self
            .doc
            .shape(&id)
            .and_then(|s| s.text("name"))
            .unwrap_or("")
            .to_owned();
        if keep && v != now {
            self.edit(&format!("name:{id}"), move |ed| {
                if let Some(s) = ed.doc.shapes.iter_mut().find(|s| id_of(s) == id) {
                    if v.is_empty() {
                        s.set_undefined("name");
                    } else {
                        s.set_text("name", &v);
                    }
                }
            });
        }
    }
}

fn flag_button<'a>(glyph: Icon, words: &str, on: bool, press: Message) -> Element<'a, Message> {
    let side = typography::scaled(22.0);
    let b = button(
        container(icon(glyph).size(13.0).tone(if on {
            kentos_ui::icon::Tone::Accent
        } else {
            kentos_ui::icon::Tone::Muted
        }))
        .center_x(side)
        .center_y(side),
    )
    .padding(0)
    .style(ui_style::button::ghost)
    .on_press(press);
    kentos_ui::widget::tip(
        b,
        kentos_ui::widget::Tip::new(words.to_owned()),
        iced::widget::tooltip::Position::Right,
    )
}

/// The list's rows, front on top.
pub fn shape_list<'a>(ed: &'a SvgEditor) -> Element<'a, Message> {
    if ed.doc.shapes.is_empty() {
        return label::caption(
            "Henüz şekil yok: soldaki araçlarla çizin ya da bir SVG dosyası ekleyin.",
        )
        .style(ui_style::text::muted)
        .into();
    }
    let n = ed.doc.shapes.len();
    let drop = ed
        .list_drag
        .as_ref()
        .filter(|d| d.active)
        .and_then(|d| d.drop);
    let mut rows = Column::new();
    for (k, s) in ed.doc.shapes.iter().rev().enumerate() {
        let id = id_of(s).to_owned();
        let hidden = s.is("hidden");
        let locked = s.is("locked");
        let chosen = ed.is_selected(&id);
        let eye = {
            let id = id.clone();
            flag_button(
                if hidden { Icon::EyeOff } else { Icon::Eye },
                if hidden { "Göster" } else { "Gizle" },
                !hidden,
                change(move |ed| ed.flip_flag(&id, "hidden")),
            )
        };
        let lock = {
            let id = id.clone();
            flag_button(
                if locked { Icon::Lock } else { Icon::Unlock },
                if locked {
                    "Kilidi aç"
                } else {
                    "Kilitle: tuvalde seçilmez ve taşınmaz"
                },
                locked,
                change(move |ed| ed.flip_flag(&id, "locked")),
            )
        };
        let name: Element<'a, Message> = match &ed.renaming {
            Some((rid, text)) if *rid == id => text_input(
                &shape_name(&{
                    let mut c = s.clone();
                    c.remove("name");
                    c
                }),
                text,
            )
            .id(iced::widget::Id::from("svge:rename"))
            .on_input(|t| {
                change(move |ed| {
                    if let Some(r) = &mut ed.renaming {
                        r.1 = t.clone();
                    }
                })
            })
            .on_submit(change(|ed| ed.finish_rename(true)))
            .padding([2, 6])
            .size(typography::body())
            .style(ui_style::field::validated(false))
            .width(Fill)
            .into(),
            _ => label::body(shape_name(s))
                .style(move |t: &Theme| iced::widget::text::Style {
                    color: Some(if hidden {
                        Tokens::of(t).muted
                    } else {
                        Tokens::of(t).text
                    }),
                })
                .width(Fill)
                .into(),
        };
        let mut line = row![eye, lock, name]
            .spacing(2)
            .align_y(Center)
            .height(row_height());
        if s.is("group") {
            line = line.push(label::caption("▣").style(ui_style::text::muted));
        }
        let face = container(line.padding([0, 4]))
            .width(Fill)
            .style(move |t: &Theme| {
                let tk = Tokens::of(t);
                container::Style {
                    background: chosen.then(|| iced::Background::Color(tk.selection())),
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..iced::Border::default()
                    },
                    ..container::Style::default()
                }
            });
        let press_id = id.clone();
        let dbl_id = id.clone();
        rows = rows.push(drop_line(drop == Some(k)));
        rows = rows.push(
            mouse_area(face)
                .on_press(change(move |ed| {
                    let shift = ed.shift_held;
                    ed.row_pressed(&press_id, shift);
                }))
                .on_double_click(change(move |ed| ed.start_rename(&dbl_id))),
        );
    }
    rows = rows.push(drop_line(drop == Some(n)));
    mouse_area(rows)
        .on_move(|p| change(move |ed| ed.list_moved(p.y)))
        .on_release(change(|ed| ed.list_released()))
        .into()
}

/// Where a dragged row would go: an accent line, or nothing.
fn drop_line<'a>(on: bool) -> Element<'a, Message> {
    if !on {
        return space().height(0).into();
    }
    container(space())
        .height(2)
        .width(Fill)
        .style(|t: &Theme| container::Style {
            background: Some(iced::Background::Color(Tokens::of(t).accent)),
            ..container::Style::default()
        })
        .into()
}

/// The left column's list with its title.
pub fn list_column<'a>(ed: &'a SvgEditor) -> Element<'a, Message> {
    column![
        label::caption("Şekiller (öndeki üstte)")
            .font(typography::ui_strong())
            .style(ui_style::text::muted),
        shape_list(ed),
    ]
    .spacing(6)
    .into()
}
