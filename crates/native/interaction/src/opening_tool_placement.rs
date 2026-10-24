//! Ephemeral precision-placement controls for the existing opening tools.
//! Geometry and measurements come from the native creation plan.
use super::*;
use crate::tool_settings::choices;
use kentos_commands::{OpeningDistanceReference as Reference, OpeningFields, OpeningFrom};

#[derive(Clone, Debug, Default)]
pub(super) struct Placement {
    pub from: OpeningFrom,
    pub reference: Reference,
    pub two_points: bool,
    pub copy: bool,
    pub first: Option<(Slot, Vec2)>,
    pub format: Format,
    pub info: Vec<String>,
    pub side_point: Option<Vec2>,
}
impl Placement {
    pub fn method_label(&self) -> &'static str {
        if self.copy {
            "Örnekten al"
        } else if self.two_points {
            "İki pervaz"
        } else {
            "Tek tık"
        }
    }
    pub fn end_label(&self) -> &'static str {
        match self.from {
            OpeningFrom::Near => "Yakın uç",
            OpeningFrom::Start => "Başlangıç",
            OpeningFrom::End => "Bitiş",
        }
    }
    pub fn reference_label(&self) -> &'static str {
        match self.reference {
            Reference::Jamb => "Yakın pervaz",
            Reference::Center => "Merkez",
            Reference::Hinge => "Menteşe",
        }
    }
}
impl OpeningTool {
    pub(super) fn placement_choices(&self, key: &str) -> Option<Vec<OptionChoice>> {
        let p = &self.placement;
        Some(match key {
            "B" => choices(
                &[
                    ("single", "Tek tık"),
                    ("two", "İki pervaz"),
                    ("copy", "Örnekten al"),
                ],
                if p.copy {
                    "copy"
                } else if p.two_points {
                    "two"
                } else {
                    "single"
                },
            ),
            "F" => choices(
                &[
                    ("near", "Yakın uç"),
                    ("start", "Başlangıç"),
                    ("end", "Bitiş"),
                ],
                match p.from {
                    OpeningFrom::Near => "near",
                    OpeningFrom::Start => "start",
                    OpeningFrom::End => "end",
                },
            ),
            "R" => choices(
                &[
                    ("jamb", "Yakın pervaz"),
                    ("center", "Merkez"),
                    ("hinge", "Menteşe"),
                ],
                match p.reference {
                    Reference::Jamb => "jamb",
                    Reference::Center => "center",
                    Reference::Hinge => "hinge",
                },
            ),
            _ => return None,
        })
    }
    pub(super) fn placement_input(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        let value = match key {
            "G" if self.placement.copy => return self.placement_configure("B", "single", cx),
            "B" => {
                if self.placement.two_points {
                    "copy"
                } else if self.placement.copy {
                    "single"
                } else {
                    "two"
                }
            }
            "F" => match self.placement.from {
                OpeningFrom::Near => "start",
                OpeningFrom::Start => "end",
                OpeningFrom::End => "near",
            },
            "R" => match self.placement.reference {
                Reference::Jamb => "center",
                Reference::Center => "hinge",
                Reference::Hinge => "jamb",
            },
            _ => return false,
        };
        self.placement_configure(key, value, cx)
    }
    pub(super) fn placement_configure(
        &mut self,
        key: &str,
        value: &str,
        cx: &mut Context<'_>,
    ) -> bool {
        match (key, value) {
            ("B", "single" | "two" | "copy") => {
                self.placement.first = None;
                self.placement.two_points = value == "two";
                self.placement.copy = value == "copy";
            }
            ("F", "near" | "start" | "end") => {
                self.placement.from = match value {
                    "start" => OpeningFrom::Start,
                    "end" => OpeningFrom::End,
                    _ => OpeningFrom::Near,
                }
            }
            ("R", "jamb" | "center" | "hinge") => {
                self.placement.reference = match value {
                    "center" => Reference::Center,
                    "hinge" => Reference::Hinge,
                    _ => Reference::Jamb,
                }
            }
            ("A", _) => {
                let Some(slot) = value.parse().ok().and_then(|uid| cx.doc.slot_of(uid)) else {
                    return false;
                };
                return self.take_sample(slot, cx);
            }
            _ => return false,
        }
        self.ask = None;
        self.see(cx);
        true
    }
    pub(super) fn sample_at(&self, at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        cx.spatial.hits(at, cx.pick_tolerance()).into_iter().find(
            |s| matches!(cx.doc.get(*s),Some(Entity::Opening(o)) if o.opening.form==self.form),
        )
    }
    pub(super) fn take_sample(&mut self, slot: Slot, cx: &mut Context<'_>) -> bool {
        let Some(Entity::Opening(o)) = cx
            .doc
            .get(slot)
            .filter(|e| matches!(e,Entity::Opening(o) if o.opening.form==self.form))
        else {
            return false;
        };
        if let Some(style) = &o.opening.style
            && !builtin_opening_types()
                .iter()
                .any(|t| &t.id == style && t.form == self.form)
        {
            cx.say(
                Level::Warn,
                "Örneğin kayıtlı açıklık türü bulunamıyor; mevcut ayarlar korundu.",
            );
            return false;
        }
        self.kind = o.opening.style.clone();
        self.width = Some(o.opening.width);
        self.placement.copy = false;
        self.placement.two_points = false;
        self.placement.first = None;
        self.see(cx);
        cx.say(
            Level::Success,
            "Örnekten tür ve genişlik alındı; yerleştirilecek duvarı seçin.",
        );
        true
    }
    pub(super) fn describe(&mut self, o: &OpeningFields, cx: &Context<'_>) {
        let f = &self.placement.format;
        if let Some(m) = kentos_native_application::opening_placement::measure(cx.doc, o) {
            self.placement
                .info
                .push(format!("Duvar · En {}", f.length(m.width)));
            self.placement.info.push(format!(
                "Net köşe payı: Baş {} · Son {}",
                f.length(m.start_clear),
                f.length(m.end_clear)
            ));
            if self.form == OpeningForm::Door {
                self.placement.info.push(format!(
                    "Açılma: {} · Menteşe: {}",
                    match o.side {
                        OpeningSide::Left => "Sol",
                        OpeningSide::Right => "Sağ",
                    },
                    if o.hand == OpeningHand::Start {
                        "Baş"
                    } else {
                        "Son"
                    }
                ));
            }
        }
    }
}

impl OpeningTool {
    pub(super) fn placement_settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::{Setting, choices};
        let Some(kind) = builtin_opening_types()
            .into_iter()
            .find(|t| self.kind.as_ref() == Some(&t.id))
            .or_else(|| {
                builtin_opening_types()
                    .into_iter()
                    .find(|t| t.form == self.form)
            })
        else {
            return Vec::new();
        };
        let mut distance =
            Setting::text("U", "Uçtan uzaklık", f.length(self.distance.unwrap_or(0.0)));
        distance.editable = !self.middle;
        let mut rows = vec![
            Setting::choice("T", "Tür", self.option_choices("T")),
            Setting::text("E", "Genişlik", f.length(self.width.unwrap_or(kind.width))),
            Setting::choice(
                "O",
                "Yerleştirme",
                choices(
                    &[
                        ("free", "Serbest"),
                        ("distance", "Uçtan uzaklık"),
                        ("center", "Ortala"),
                    ],
                    if self.middle {
                        "center"
                    } else if self.distance.is_some() {
                        "distance"
                    } else {
                        "free"
                    },
                ),
            ),
            distance,
        ];
        rows.insert(0, Setting::choice("B", "Yöntem", self.option_choices("B")));
        rows.push(Setting::choice("F", "Duvar ucu", self.option_choices("F")));
        rows.push(Setting::choice(
            "R",
            "Ölçü noktası",
            self.option_choices("R"),
        ));
        if self.placement.two_points || self.placement.copy {
            rows.retain(|row| !matches!(row.key, "E" | "O" | "U" | "F" | "R"));
        } else if self.middle {
            // The remembered offset does not apply to a centred opening.
            rows.retain(|row| !matches!(row.key, "U" | "F" | "R"));
        }
        if self.form == OpeningForm::Door {
            rows.push(Setting::choice(
                "Y",
                "Açılma yanı",
                self.option_choices("Y"),
            ));
            rows.push(Setting::choice(
                "M",
                "Menteşe",
                choices(
                    &[("start", "Baş"), ("end", "Son")],
                    if self.hand == OpeningHand::Start {
                        "start"
                    } else {
                        "end"
                    },
                ),
            ));
        }
        rows
    }
    pub(super) fn apply_settings(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "B" | "F" | "R" | "A") {
            return self.placement_configure(key, value, cx);
        }
        if matches!(key, "T" | "Y") {
            return self.choose_option(key, value, cx);
        }
        match (key, value) {
            ("O", "free" | "distance" | "center") => {
                self.middle = value == "center";
                if value == "free" {
                    self.distance = None;
                }
                if value == "distance" {
                    self.distance = Some(self.distance.unwrap_or(0.0));
                }
            }
            ("M", "start" | "end") if self.form == OpeningForm::Door => {
                self.hand = if value == "start" {
                    OpeningHand::Start
                } else {
                    OpeningHand::End
                };
            }
            ("E" | "U", _) => {
                let Some(v) = cx.typed_length(value).filter(|v| v.is_finite()) else {
                    cx.say(Level::Warn, "Bir uzunluk yazın: 900mm veya 0.9m gibi.");
                    return false;
                };
                if (key == "E" && !(v > 0.0 && v <= 100.0)) || (key == "U" && v < 0.0) {
                    cx.say(
                        Level::Warn,
                        "Genişlik 0–100 m arasında, uzaklık sıfır veya pozitif olmalı.",
                    );
                    return false;
                }
                if key == "E" {
                    self.width = Some(v);
                } else {
                    // In the visible field zero is a real flush-to-end distance.
                    // Legacy command-line U / 0 still clears its old offset mode.
                    self.distance = Some(v);
                }
            }
            _ => return false,
        }
        self.see(cx);
        true
    }
}

/// Keep a native refusal readable beside the pointer at large UI text sizes.
/// The full diagnostic is preserved, with breaks only at word boundaries.
pub(super) fn error_lines(label: &str, why: &str) -> Vec<String> {
    let mut lines = vec![label.to_owned()];
    let mut line = String::new();
    let mut length = 0;
    for word in why.split_whitespace() {
        let count = word.chars().count();
        if length > 0 && length + count + 1 > 64 {
            lines.push(std::mem::take(&mut line));
            length = 0;
        }
        if length > 0 {
            line.push(' ');
            length += 1;
        }
        line.push_str(word);
        length += count;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
