//! Opening array picks and settings; native cad.openings.edit owns every result.
use crate::edge::Outline;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::{Setting, choices};
use crate::{Format, Level, Vec2, points};
use kentos_commands::*;
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, geometry::shape, openings_edit};

pub const ID: &str = "openingArray";
pub const LABEL: &str = "Açıklık dağıt";

pub struct Array {
    source: Option<String>,
    source_label: String,
    fit: bool,
    count: u32,
    distance: f64,
    reference: OpeningArraySpacing,
    direction: WallEnd,
    margins: [f64; 2],
    numbered: bool,
    numbering: OpeningArrayNumbering,
    asking: Option<String>,
    revision: Option<u64>,
    drawn: Preview,
}
impl Default for Array {
    fn default() -> Self {
        Self {
            source: None,
            source_label: "Açıklık seçin".into(),
            fit: false,
            count: 3,
            distance: 1.0,
            reference: OpeningArraySpacing::Clear,
            direction: WallEnd::End,
            margins: [2.0, 1.0],
            numbered: false,
            numbering: OpeningArrayNumbering {
                prefix: "K".into(),
                start: 1,
                step: 1,
            },
            asking: None,
            revision: None,
            drawn: Preview::default(),
        }
    }
}
impl Array {
    fn request(&self) -> Option<OpeningsEdit> {
        Some(OpeningsEdit {
            uids: vec![self.source.clone()?],
            expected_revision: self.revision.map(|r| r.to_string()),
            operation: OpeningsOperation::Array {
                layout: if self.fit {
                    OpeningArrayLayout::Fit {
                        count: self.count,
                        start_margin: self.margins[0],
                        end_margin: self.margins[1],
                    }
                } else {
                    OpeningArrayLayout::Spacing {
                        count: self.count,
                        distance: self.distance,
                        reference: self.reference,
                        direction: self.direction,
                    }
                },
                numbering: self.numbered.then(|| self.numbering.clone()),
            },
        })
    }
    fn pick(&mut self, slot: Slot, cx: &Context<'_>) {
        let Some(Entity::Opening(o)) = cx.doc.get(slot) else {
            return;
        };
        self.source = cx.doc.uid(slot).map(|u| u.to_string());
        self.source_label = format!("#{} · {}", slot.0, o.opening.form.label());
        self.see(cx);
    }
    fn at(p: &Pointer, cx: &Context<'_>) -> Option<Slot> {
        cx.spatial
            .hits(p.raw, cx.pick_tolerance())
            .into_iter()
            .find(|s| {
                cx.doc.get(*s).is_some_and(|e| {
                    matches!(e, Entity::Opening(_))
                        && crate::selectable::entity_allowed(cx.draft.select_kinds, e)
                        && crate::selectable::on_selected_layer(cx.doc, e)
                })
            })
    }
    fn see(&mut self, cx: &Context<'_>) {
        self.revision = Some(cx.doc.revision());
        self.drawn = Preview::default();
        let Some(input) = self.request() else { return };
        let mut tag_at = Vec2::new(0.0, 0.0);
        if let Some(e) = self
            .source
            .as_deref()
            .and_then(|u| u.parse().ok())
            .and_then(|u| cx.doc.slot_of(u))
            .and_then(|s| cx.doc.get(s))
        {
            self.drawn
                .strokes
                .extend(Outline::of(&shape(e), Some([6.0, 4.0]), 2.5, Tone::Accent).strokes);
            if let Entity::Opening(o) = e
                && let Some(p) = o.opening.place.first()
            {
                tag_at = Vec2::new(p.x, p.y);
            }
        }
        match openings_edit::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.added {
                    self.drawn.strokes.extend(
                        Outline::of(&shape(e), Some([4.0, 3.0]), 1.5, Tone::Accent).strokes,
                    );
                }
                self.drawn.tag = Some(Tag {
                    at: tag_at,
                    lines: vec![
                        format!("{} yeni açıklık · kaynak korunur", output.added.len()),
                        "Önizleme · Enter ile uygula · Esc ile vazgeç".into(),
                    ],
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                // Keep the entire explanation readable in the drawing tag.
                let mut lines = vec![String::new()];
                for word in error.message.split_whitespace() {
                    if lines
                        .last()
                        .is_some_and(|s| s.chars().count() + word.chars().count() > 62)
                    {
                        lines.push(String::new());
                    }
                    if let Some(last) = lines.last_mut() {
                        if !last.is_empty() {
                            last.push(' ');
                        }
                        last.push_str(word);
                    }
                }
                self.drawn.tag = Some(Tag { at: tag_at, lines });
                self.drawn.tag_tone = Tone::Danger;
            }
            _ => {}
        }
    }
}
impl Tool for Array {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn prompt(&self) -> Prompt {
        if let Some(key) = &self.asking {
            return Prompt::new(LABEL, format!("{key} değerini yazın")).option("Geri", "G");
        }
        Prompt::new(
            LABEL,
            if self.source.is_some() {
                "ayarları ve önizlemeyi kontrol edin; Enter ile kopyaları ekleyin"
            } else {
                "kaynak kapı, pencere veya boşluğa tıklayın"
            },
        )
        .option_with("Kopya adedi", "A", self.count.to_string())
        .option("Yöntem", "M")
        .option("Başka kaynak", "K")
        .option("Uygula", "Enter")
    }
    fn point_count(&self) -> usize {
        usize::from(self.source.is_some())
    }
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }
    fn snaps(&self) -> bool {
        false
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        let ids = cx.selection.ids();
        if let [slot] = ids {
            self.pick(*slot, cx);
        }
        Flow::Stay
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        cx.selection.set_hover(if self.source.is_none() {
            Self::at(p, cx)
        } else {
            None
        });
        if self.source.is_some() && self.revision != Some(cx.doc.revision()) {
            self.see(cx);
        }
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if let Some(slot) = Self::at(p, cx) {
            self.pick(slot, cx);
        } else {
            cx.say(
                Level::Info,
                "Bir kaynak kapı, pencere veya boşluk seçin; uygula için Enter kullanın.",
            );
        }
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(key) = self.asking.take() {
            return self.configure(&key, text, cx);
        }
        let key = upper_tr(text.trim());
        if key == "K" {
            self.source = None;
            self.source_label = "Açıklık seçin".into();
            self.see(cx);
            return true;
        }
        if self
            .settings(&cx.format())
            .iter()
            .any(|s| s.key == key && s.editable)
        {
            self.asking = Some(key);
            true
        } else {
            false
        }
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            return Flow::Stay;
        }
        let Some(input) = self.request() else {
            return Flow::Exit;
        };
        let result = openings_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            cx.say(
                Level::Success,
                format!("{} yeni açıklık eklendi; kaynak korundu.", self.count),
            );
            Flow::Exit
        } else {
            self.see(cx);
            Flow::Stay
        }
    }
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.asking.take().is_some()
    }
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.source.take().is_some() {
            self.source_label = "Açıklık seçin".into();
            self.see(cx);
            true
        } else {
            false
        }
    }
    fn preview(&self, _: &Format) -> Preview {
        self.drawn.clone()
    }
    fn settings(&self, f: &Format) -> Vec<Setting> {
        let mut rows = vec![
            Setting::text("C", "Kaynak", self.source_label.clone()).read_only(),
            Setting::text("A", "Kopya adedi", self.count.to_string()),
            Setting::choice(
                "M",
                "Dağıtım",
                choices(
                    &[
                        ("spacing", "Aralıkla çoğalt"),
                        ("fit", "Uç paylarıyla dağıt"),
                    ],
                    if self.fit { "fit" } else { "spacing" },
                ),
            ),
        ];
        if self.fit {
            rows.extend([
                Setting::text("B", "Başlangıç payı", f.length(self.margins[0])),
                Setting::text("S", "Bitiş payı", f.length(self.margins[1])),
            ]);
        } else {
            rows.extend([
                Setting::choice(
                    "R",
                    "Aralık türü",
                    choices(
                        &[
                            ("clear", "Net pervaz arası"),
                            ("centers", "Merkezler arası"),
                        ],
                        if self.reference == OpeningArraySpacing::Clear {
                            "clear"
                        } else {
                            "centers"
                        },
                    ),
                ),
                Setting::text("U", "Aralık", f.length(self.distance)),
                Setting::choice(
                    "Y",
                    "Yön",
                    choices(
                        &[
                            ("end", "Duvar sonuna doğru"),
                            ("start", "Duvar başına doğru"),
                        ],
                        if self.direction == WallEnd::End {
                            "end"
                        } else {
                            "start"
                        },
                    ),
                ),
            ]);
        }
        rows.push(Setting::choice(
            "N",
            "Yeni numaralar",
            choices(
                &[("off", "Numarasız"), ("on", "Önek ve sırayla")],
                if self.numbered { "on" } else { "off" },
            ),
        ));
        if self.numbered {
            rows.extend([
                Setting::text("Ö", "Önek", &self.numbering.prefix),
                Setting::text("İ", "İlk numara", self.numbering.start.to_string()),
                Setting::text("D", "Numara adımı", self.numbering.step.to_string()),
            ]);
        }
        rows
    }
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.settings(&Format::default())
            .into_iter()
            .find(|s| s.key == key)
            .map_or_else(Vec::new, |s| s.choices)
    }
    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }
    fn takes_words(&self) -> bool {
        self.asking.as_deref() == Some("Ö")
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let canonical = self
            .option_choices(key)
            .into_iter()
            .find(|c| {
                upper_tr(&c.typed) == upper_tr(value.trim())
                    || upper_tr(&c.label) == upper_tr(value.trim())
            })
            .map(|c| c.typed);
        let value = canonical.as_deref().unwrap_or(value);
        let valid = match (key, value) {
            ("M", "spacing" | "fit") => {
                self.fit = value == "fit";
                true
            }
            ("R", "clear" | "centers") => {
                self.reference = if value == "clear" {
                    OpeningArraySpacing::Clear
                } else {
                    OpeningArraySpacing::Centers
                };
                true
            }
            ("Y", "start" | "end") => {
                self.direction = if value == "start" {
                    WallEnd::Start
                } else {
                    WallEnd::End
                };
                true
            }
            ("N", "on" | "off") => {
                self.numbered = value == "on";
                true
            }
            ("A" | "İ" | "D", _) => match value.trim().parse::<u32>() {
                Ok(n) if (key != "A" || (1..=10000).contains(&n)) && (key != "D" || n > 0) => {
                    match key {
                        "A" => self.count = n,
                        "İ" => self.numbering.start = n,
                        _ => self.numbering.step = n,
                    };
                    true
                }
                _ => false,
            },
            ("U" | "B" | "S", _) => match cx
                .typed_length(value)
                .filter(|v| v.is_finite() && *v >= 0.0)
            {
                Some(n) => {
                    match key {
                        "U" => self.distance = n,
                        "B" => self.margins[0] = n,
                        _ => self.margins[1] = n,
                    };
                    true
                }
                _ => false,
            },
            ("Ö", _) if value.chars().count() <= 128 => {
                self.numbering.prefix = value.into();
                true
            }
            _ => false,
        };
        if valid {
            self.see(cx);
        } else {
            cx.say(Level::Warn,"Değer uygulanmadı: adet 1–10000, numara adımı pozitif tam sayı, uzaklık sıfır veya pozitif uzunluk olmalı.");
        }
        valid
    }
}
