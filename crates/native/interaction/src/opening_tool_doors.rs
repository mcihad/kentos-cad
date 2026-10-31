//! Kapı's operation and its special doors' settings (docs/adr/0777 §5): Açılış (Ç) the operation over the type's
//! (Kanatlı, Sürme, Sabit, Döner, Cebe giren, Katlanır), Kanat (K) the leaves (a revolving door's wings, a folding
//! door's panels), Tambur (J) a revolving door's drum, Cep payı (P) a pocket door's lip, İki yana (İ) a folding door's
//! panels to both jambs, Bindirme (S) two sliding leaves' overlap and Ray (H) a sliding door's track. They are the
//! command's input (CMD-07): `cad.openings.create` writes them, and its plan is the preview.

use kentos_commands::{OpeningBuild, OpeningForm, OpeningOperation};

use super::{Ask, OpeningTool};
use crate::format::Format;
use crate::log::Level;
use crate::tool::{Context, OptionChoice};
use crate::tool_settings::{Setting, choices};

/// What Kapı puts over its type's operation (none: the type's).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Moves {
    pub operation: Option<OpeningOperation>,
    pub leaves: Option<u8>,
    pub drum: Option<f64>,
    pub lip: Option<f64>,
    pub both_sides: bool,
    pub track: bool,
    pub overlap: Option<f64>,
}

/// The operations' names as the settings write them.
const OPERATIONS: [(OpeningOperation, &str, &str); 6] = [
    (OpeningOperation::Swing, "swing", "door"),
    (OpeningOperation::Slide, "slide", "doorSliding"),
    (OpeningOperation::Fixed, "fixed", "windowFixed"),
    (OpeningOperation::Revolving, "revolving", "doorRevolving"),
    (OpeningOperation::Pocket, "pocket", "doorPocket"),
    (OpeningOperation::Folding, "folding", "doorFolding"),
];

impl OpeningTool {
    /// The operation it puts: its own, else its type's.
    pub(super) fn operation_now(&self) -> OpeningOperation {
        self.moves
            .operation
            .or_else(|| self.kind_def().map(|t| t.operation))
            .unwrap_or_default()
    }

    /// The leaves it puts: its own, else its type's when the operation takes them, else the operation's least.
    pub(super) fn leaves_now(&self) -> u8 {
        let (least, most) = self.operation_now().leaf_range();
        let typed = self.kind_def().map_or(1, |t| t.leaves);
        self.moves
            .leaves
            .unwrap_or(if (least..=most).contains(&typed) {
                typed
            } else {
                least
            })
    }

    /// Its operation, leaves and construction for `cad.openings.create`: only what it puts over the type's, a
    /// construction's field only for its operation.
    pub(super) fn door_input(
        &self,
    ) -> (Option<OpeningOperation>, Option<u8>, Option<OpeningBuild>) {
        if self.form != OpeningForm::Door {
            return (None, None, None);
        }
        let op = self.operation_now();
        let mut b = OpeningBuild::default();
        match op {
            OpeningOperation::Revolving => b.drum = self.moves.drum,
            OpeningOperation::Pocket => b.lip = self.moves.lip,
            OpeningOperation::Folding => b.both_sides = self.moves.both_sides.then_some(true),
            OpeningOperation::Slide => {
                b.track = self.moves.track.then_some(true);
                b.overlap = self.moves.overlap.filter(|_| self.leaves_now() == 2);
            }
            _ => {}
        }
        let leaves = (self.moves.leaves.is_some() || self.moves.operation.is_some())
            .then(|| self.leaves_now());
        (self.moves.operation, leaves, (!b.is_empty()).then_some(b))
    }

    /// Açılış's choices: the type's first, then each operation.
    pub(super) fn operation_choices(&self) -> Vec<OptionChoice> {
        let typed = self
            .kind_def()
            .map_or(OpeningOperation::Swing, |t| t.operation);
        let mut out = vec![OptionChoice {
            typed: "type".into(),
            label: format!("Türün ({})", typed.label()),
            icon: None,
            preview: None,
            checked: self.moves.operation.is_none(),
            command: None,
        }];
        out.extend(OPERATIONS.iter().map(|(op, name, icon)| OptionChoice {
            typed: (*name).into(),
            label: op.label().into(),
            icon: Some(icon),
            preview: None,
            checked: self.moves.operation == Some(*op),
            command: None,
        }));
        out
    }

    /// The door's settings rows after Menteşe.
    pub(super) fn door_settings(&self, f: &Format, rows: &mut Vec<Setting>) {
        if self.form != OpeningForm::Door {
            return;
        }
        rows.push(Setting::choice("Ç", "Açılış", self.operation_choices()));
        let op = self.operation_now();
        let (least, most) = op.leaf_range();
        let word = if op == OpeningOperation::Folding {
            "Panel"
        } else {
            "Kanat"
        };
        let leaves: Vec<(String, String)> = (least..=most)
            .map(|n| (n.to_string(), n.to_string()))
            .collect();
        let pairs: Vec<(&str, &str)> = leaves
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        rows.push(Setting::choice(
            "K",
            word,
            choices(&pairs, &self.leaves_now().to_string()),
        ));
        let on_off = [("on", "Evet"), ("off", "Hayır")];
        match op {
            OpeningOperation::Revolving => rows.push(Setting::text(
                "J",
                "Tambur çapı",
                self.moves
                    .drum
                    .map_or_else(|| "Işık kadar".to_owned(), |d| f.length(d)),
            )),
            OpeningOperation::Pocket => rows.push(Setting::text(
                "P",
                "Cepten kalan pay",
                f.length(self.moves.lip.unwrap_or(kentos_commands::POCKET_LIP)),
            )),
            OpeningOperation::Folding => rows.push(Setting::choice(
                "İ",
                "İki yana toplanır",
                choices(&on_off, if self.moves.both_sides { "on" } else { "off" }),
            )),
            OpeningOperation::Slide => {
                if self.leaves_now() == 2 {
                    rows.push(Setting::text(
                        "S",
                        "Kanat bindirmesi",
                        self.moves
                            .overlap
                            .map_or_else(|| "%5".to_owned(), |o| f.length(o)),
                    ));
                }
                rows.push(Setting::choice(
                    "H",
                    "Ray çizilir",
                    choices(&on_off, if self.moves.track { "on" } else { "off" }),
                ));
            }
            _ => {}
        }
    }

    /// A door setting chosen or typed: `None` when `key` is not one.
    pub(super) fn door_configure(
        &mut self,
        key: &str,
        value: &str,
        cx: &mut Context<'_>,
    ) -> Option<bool> {
        if self.form != OpeningForm::Door {
            return None;
        }
        match key {
            "Ç" => {
                self.moves.operation = match value {
                    "type" => None,
                    v => Some(OPERATIONS.iter().find(|(_, n, _)| *n == v)?.0),
                };
                // Leaves out of the new operation's range go back to its own.
                let (least, most) = self.operation_now().leaf_range();
                if self
                    .moves
                    .leaves
                    .is_some_and(|n| !(least..=most).contains(&n))
                {
                    self.moves.leaves = None;
                }
            }
            "K" => match value.trim().parse::<u8>() {
                Ok(n) if (1..=6).contains(&n) => self.moves.leaves = Some(n),
                _ => {
                    cx.say(
                        Level::Warn,
                        "Kanat sayısı 1 ile 6 arasında bir tam sayı olmalı.",
                    );
                    return Some(false);
                }
            },
            "İ" => self.moves.both_sides = value == "on",
            "H" => self.moves.track = value == "on",
            "J" | "P" | "S" => {
                let t = value.trim();
                let back = t.is_empty() || t.to_lowercase().starts_with("ışık") || t == "%5";
                let v = if back {
                    None
                } else {
                    match cx
                        .typed_length(value)
                        .filter(|v| v.is_finite() && *v >= 0.0)
                    {
                        Some(v) => Some(v),
                        None => {
                            cx.say(Level::Warn, "Bir uzunluk yazın: 10cm veya 0.10m gibi.");
                            return Some(false);
                        }
                    }
                };
                match key {
                    "J" => self.moves.drum = v,
                    "P" => self.moves.lip = v,
                    _ => self.moves.overlap = v,
                }
            }
            _ => return None,
        }
        self.see(cx);
        Some(true)
    }

    /// A door key typed on the command line: Ç turns the operation on, K, J, P and S ask, İ and H toggle.
    pub(super) fn door_key(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if self.form != OpeningForm::Door {
            return false;
        }
        match key {
            "Ç" => {
                let now = self.operation_now();
                let at = OPERATIONS
                    .iter()
                    .position(|(o, _, _)| *o == now)
                    .unwrap_or(0);
                let next = OPERATIONS[(at + 1) % OPERATIONS.len()].1;
                return self.door_configure("Ç", next, cx).unwrap_or(false);
            }
            "K" => self.ask = Some(Ask::Leaves),
            "J" if self.operation_now() == OpeningOperation::Revolving => {
                self.ask = Some(Ask::Drum)
            }
            "P" if self.operation_now() == OpeningOperation::Pocket => self.ask = Some(Ask::Lip),
            "S" if self.operation_now() == OpeningOperation::Slide => self.ask = Some(Ask::Overlap),
            "İ" if self.operation_now() == OpeningOperation::Folding => {
                self.moves.both_sides = !self.moves.both_sides;
            }
            "H" if self.operation_now() == OpeningOperation::Slide => {
                self.moves.track = !self.moves.track
            }
            _ => return false,
        }
        self.see(cx);
        true
    }

    /// A number asked for after K, J, P or S: `None` when the tool asks for none of them.
    pub(super) fn door_answer(
        &mut self,
        ask: Ask,
        text: &str,
        cx: &mut Context<'_>,
    ) -> Option<bool> {
        let key = match ask {
            Ask::Leaves => "K",
            Ask::Drum => "J",
            Ask::Lip => "P",
            Ask::Overlap => "S",
            _ => return None,
        };
        let done = self.door_configure(key, text, cx).unwrap_or(false);
        if done {
            self.ask = None;
        }
        Some(done)
    }

    /// The words the prompt says of the door's operation and its special settings.
    pub(super) fn door_prompt(
        &self,
        mut p: crate::prompt::Prompt,
        f: &Format,
    ) -> crate::prompt::Prompt {
        if self.form != OpeningForm::Door {
            return p;
        }
        let op = self.operation_now();
        p = p.option_with("Açılış", "Ç", op.label()).option_with(
            if op == OpeningOperation::Folding {
                "Panel"
            } else {
                "Kanat"
            },
            "K",
            self.leaves_now().to_string(),
        );
        match op {
            OpeningOperation::Revolving => p.option_with(
                "Tambur",
                "J",
                self.moves
                    .drum
                    .map_or_else(|| "Işık kadar".to_owned(), |d| f.length(d)),
            ),
            OpeningOperation::Pocket => p.option_with(
                "Cep payı",
                "P",
                f.length(self.moves.lip.unwrap_or(kentos_commands::POCKET_LIP)),
            ),
            OpeningOperation::Folding => p.toggle("İki yana", "İ", self.moves.both_sides),
            OpeningOperation::Slide => {
                if self.leaves_now() == 2 {
                    p = p.option_with(
                        "Bindirme",
                        "S",
                        self.moves
                            .overlap
                            .map_or_else(|| "%5".to_owned(), |o| f.length(o)),
                    );
                }
                p.toggle("Ray", "H", self.moves.track)
            }
            _ => p,
        }
    }
}
