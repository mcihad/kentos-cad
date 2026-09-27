//! The SVG editor's keys (the web's `SvgEditor.key` and the file keys of
//! `svgFile.ts`): Esc gives up what is half done, then the node editing,
//! then the selection, then asks to close; the history, the path operations
//! and panels (Inkscape's keys), the node tool's keys before the tools'
//! letters, Delete, Enter, the stacking order, arrows nudging by the grid
//! (five times with Shift), zoom, flips and the tools' letters. While a
//! field has the keyboard only ↑ and ↓ act (they step a number field).

use iced::keyboard::Key;
use iced::keyboard::key::Named;

use super::ToolId;
use super::TOOLS;
use super::actions::{Action, NodeCmd, PathOp, Restack};
use super::files::FileCmd;
use super::state::{SvgEditor, Tab};
use crate::keys::KeyPress;
use crate::style::designer::Focus;

/// What the web's `KeyboardEvent.key` would say.
fn web_key(press: &KeyPress) -> String {
    match &press.key {
        Key::Character(c) => c.to_string(),
        Key::Named(n) => match n {
            Named::Escape => "Escape",
            Named::Enter => "Enter",
            Named::Delete => "Delete",
            Named::Backspace => "Backspace",
            Named::Insert => "Insert",
            Named::PageUp => "PageUp",
            Named::PageDown => "PageDown",
            Named::Home => "Home",
            Named::End => "End",
            Named::ArrowUp => "ArrowUp",
            Named::ArrowDown => "ArrowDown",
            Named::ArrowLeft => "ArrowLeft",
            Named::ArrowRight => "ArrowRight",
            Named::Space => " ",
            _ => "",
        }
        .to_owned(),
        _ => String::new(),
    }
}

/// What a key asks of the window beyond the editor itself.
#[derive(Clone, Debug, PartialEq)]
pub enum KeyOutcome {
    Nothing,
    /// Esc with nothing to give up: the window asks to close.
    Close,
    Undo,
    Redo,
    File(FileCmd),
}

impl SvgEditor {
    /// A key, with what holds the keyboard (`key`).
    pub fn key(&mut self, press: &KeyPress, focus: &Focus) -> KeyOutcome {
        let k = web_key(press);
        let m = press.modifiers;
        let ctrl = m.control() || m.logo();
        let (shift, alt) = (m.shift(), m.alt());
        if k == "Escape" {
            if self.cancel() {
                return KeyOutcome::Nothing;
            }
            if self.node_edit.is_some() {
                self.edit_nodes(None);
                return KeyOutcome::Nothing;
            }
            if !self.selection.is_empty() {
                self.select(Vec::new());
                return KeyOutcome::Nothing;
            }
            return KeyOutcome::Close;
        }
        if focus.any {
            // ↑ and ↓ step the number field that holds the keyboard.
            if (k == "ArrowUp" || k == "ArrowDown")
                && let Some(id) = &focus.id
                && let Some(key) = self.field_key(id)
            {
                self.num_step(&key, k == "ArrowUp", shift);
            }
            return KeyOutcome::Nothing;
        }
        let low = k.to_lowercase();
        // The files' keys (they come first on the web, in the capture phase).
        if ctrl && !alt {
            match (low.as_str(), shift) {
                ("o", false) => return KeyOutcome::File(FileCmd::OpenFile),
                ("s", true) => return KeyOutcome::File(FileCmd::SaveAs),
                ("e", true) => return KeyOutcome::File(FileCmd::Export),
                ("x", true) => return KeyOutcome::File(FileCmd::ToggleSource),
                _ => {}
            }
        }
        let nodes = self.node_edit.is_some() && self.tool == ToolId::Node;
        if ctrl && low == "z" {
            return if shift { KeyOutcome::Redo } else { KeyOutcome::Undo };
        }
        if ctrl && low == "y" {
            return KeyOutcome::Redo;
        }
        if ctrl && low == "d" {
            self.action(Action::Duplicate);
            return KeyOutcome::Nothing;
        }
        if ctrl && low == "g" {
            self.action(if shift { Action::Ungroup } else { Action::Group });
            return KeyOutcome::Nothing;
        }
        if ctrl && !shift && low == "a" {
            if nodes {
                self.select_all_nodes();
            } else {
                self.select_all();
            }
            return KeyOutcome::Nothing;
        }
        if ctrl {
            // Path operations and panels (Inkscape's keys).
            let op = match k.as_str() {
                "+" | "=" if !alt => Some(PathOp::Union),
                "-" if !alt => Some(PathOp::Difference),
                "*" => Some(PathOp::Intersection),
                "^" => Some(PathOp::Exclusion),
                "/" => Some(if alt { PathOp::Cut } else { PathOp::Division }),
                "(" => Some(PathOp::Inset),
                ")" => Some(PathOp::Outset),
                _ => match low.as_str() {
                    "k" => Some(if shift {
                        PathOp::BreakApart
                    } else {
                        PathOp::Combine
                    }),
                    "c" if shift => Some(PathOp::ToPath),
                    "c" if alt => Some(PathOp::StrokeToPath),
                    "l" if !shift => Some(PathOp::Simplify),
                    _ => None,
                },
            };
            if let Some(op) = op {
                self.path_op(op);
                return KeyOutcome::Nothing;
            }
            if low == "a" && shift {
                self.ui.tab = Tab::Align;
                self.touch();
            } else if low == "m" && shift {
                self.ui.tab = Tab::Transform;
                self.touch();
            } else if nodes && (k == "Delete" || k == "Backspace") {
                self.node_cmd(NodeCmd::Delete { keep_shape: false });
            }
            return KeyOutcome::Nothing;
        }
        if nodes {
            // The node tool's keys come before the tools' letters.
            let up = k.to_uppercase();
            if shift && !alt {
                let cmd = match up.as_str() {
                    "C" => Some(NodeCmd::Type("cusp")),
                    "S" => Some(NodeCmd::Type("smooth")),
                    "Y" => Some(NodeCmd::Type("symmetric")),
                    "A" => Some(NodeCmd::Type("auto")),
                    "J" => Some(NodeCmd::Join { merge: true }),
                    "K" => Some(NodeCmd::Join { merge: false }),
                    "B" => Some(NodeCmd::Break),
                    "L" => Some(NodeCmd::Segments { line: true }),
                    "U" => Some(NodeCmd::Segments { line: false }),
                    _ => None,
                };
                if let Some(c) = cmd {
                    self.node_cmd(c);
                    return KeyOutcome::Nothing;
                }
            }
            let chosen = !self.node_selected().is_empty();
            if k == "Insert" {
                self.node_cmd(NodeCmd::Insert);
                return KeyOutcome::Nothing;
            }
            if (k == "Delete" || k == "Backspace") && chosen {
                self.node_cmd(if alt {
                    NodeCmd::DeleteSegment
                } else {
                    NodeCmd::Delete { keep_shape: true }
                });
                return KeyOutcome::Nothing;
            }
            if let Some((dx, dy)) = self.arrow_step(&k, shift)
                && chosen
            {
                self.node_nudge(dx, dy);
                return KeyOutcome::Nothing;
            }
        }
        match k.as_str() {
            "Delete" | "Backspace" => {
                self.action(Action::Delete);
                return KeyOutcome::Nothing;
            }
            "Enter" => {
                self.finish_draft(false);
                return KeyOutcome::Nothing;
            }
            "PageUp" => {
                self.restack(Restack::Raise);
                return KeyOutcome::Nothing;
            }
            "PageDown" => {
                self.restack(Restack::Lower);
                return KeyOutcome::Nothing;
            }
            "Home" => {
                self.restack(Restack::Top);
                return KeyOutcome::Nothing;
            }
            "End" => {
                self.restack(Restack::Bottom);
                return KeyOutcome::Nothing;
            }
            "!" => {
                self.invert_selection();
                return KeyOutcome::Nothing;
            }
            "%" => {
                self.options.snap_objects = !self.options.snap_objects;
                self.snapper.reset();
                self.touch();
                return KeyOutcome::Nothing;
            }
            _ => {}
        }
        if let Some((dx, dy)) = self.arrow_step(&k, shift)
            && !self.selection.is_empty()
        {
            let ids = self.selection.clone();
            let m = kentos_svg_core::arrange::translate(dx, dy);
            self.edit("nudge", |ed| {
                for s in &mut ed.doc.shapes {
                    if ids.iter().any(|i| i == super::doc::id_of(s))
                        && !s.is("locked")
                        && let Ok(Some(t)) = kentos_svg_core::model::transform_shape(s, &m)
                    {
                        *s = t;
                    }
                }
            });
            return KeyOutcome::Nothing;
        }
        match k.as_str() {
            "0" => {
                self.camera.fit(&self.doc, &self.options);
                self.touch();
                return KeyOutcome::Nothing;
            }
            "+" | "=" => {
                self.camera.zoom_by(1.25, None);
                self.touch();
                return KeyOutcome::Nothing;
            }
            "-" => {
                self.camera.zoom_by(1.0 / 1.25, None);
                self.touch();
                return KeyOutcome::Nothing;
            }
            _ => {}
        }
        if !alt && (k == "h" || k == "H") {
            self.action(if shift { Action::FlipV } else { Action::FlipH });
            return KeyOutcome::Nothing;
        }
        if shift || alt {
            return KeyOutcome::Nothing;
        }
        let letter = kentos_interaction::upper_tr(&k).replace('İ', "I");
        if let Some(t) = TOOLS.iter().find(|t| t.key == letter) {
            self.set_tool(t.id);
        }
        KeyOutcome::Nothing
    }

    /// An arrow's step: the grid (1 without one), five times with Shift.
    fn arrow_step(&self, k: &str, shift: bool) -> Option<(f64, f64)> {
        let step = (if self.options.grid > 0.0 {
            self.options.grid
        } else {
            1.0
        }) * if shift { 5.0 } else { 1.0 };
        match k {
            "ArrowLeft" => Some((-step, 0.0)),
            "ArrowRight" => Some((step, 0.0)),
            "ArrowUp" => Some((0.0, -step)),
            "ArrowDown" => Some((0.0, step)),
            _ => None,
        }
    }

    /// A number field's key from its widget id (`svge:<key>`).
    fn field_key(&self, id: &iced::widget::Id) -> Option<String> {
        let fields = self.fields.borrow();
        fields
            .keys()
            .find(|k| super::panels::field_id(k) == *id)
            .cloned()
    }
}
