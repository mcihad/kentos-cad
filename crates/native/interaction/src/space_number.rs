//! Select/click rooms and review the shared semantic numbering plan before one write.
use crate::edge::Outline;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Label, OptionChoice, Pointer, Preview, Tone, Tool};
use crate::tool_settings::{Setting, choices};
use crate::{Format, Level, Vec2, points};
use kentos_commands::{CommandResult, Entity, SpaceNumberOrder, SpacesNumber};
use kentos_native_application::{ExecutionContext, geometry::shape, spaces_number};

pub const ID: &str = "spaceNumber";
pub const LABEL: &str = "Mahal numarala";

pub struct Number {
    uids: Vec<String>,
    first: String,
    directional: bool,
    direction: Vec<Vec2>,
    asking: bool,
    revision: Option<u64>,
    drawn: Preview,
    error: Option<String>,
    last: String,
    prefix_revision: Option<u64>,
    prefixes: Vec<OptionChoice>,
}
impl Default for Number {
    fn default() -> Self {
        Self {
            uids: vec![],
            first: "Z-001".into(),
            directional: false,
            direction: vec![],
            asking: false,
            revision: None,
            drawn: Preview::default(),
            error: None,
            last: String::new(),
            prefix_revision: None,
            prefixes: Vec::new(),
        }
    }
}
impl Number {
    fn waiting_direction(&self) -> bool {
        self.directional && self.direction.len() < 2
    }
    fn request(&self) -> Option<SpacesNumber> {
        if self.waiting_direction() {
            return None;
        }
        Some(SpacesNumber {
            uids: self.uids.clone(),
            first: self.first.clone(),
            order: if self.directional {
                let [a, b] = self.direction.as_slice() else {
                    return None;
                };
                SpaceNumberOrder::Direction {
                    from: points::wire(*a),
                    to: points::wire(*b),
                }
            } else {
                SpaceNumberOrder::Input
            },
            expected_revision: self.revision.map(|r| r.to_string()),
        })
    }
    fn options(&self) -> Vec<OptionChoice> {
        choices(
            &[("input", "Tıklama sırası"), ("direction", "Yön izdüşümü")],
            if self.directional {
                "direction"
            } else {
                "input"
            },
        )
    }
    fn see(&mut self, cx: &Context<'_>) {
        if self.prefix_revision != Some(cx.doc.revision()) {
            self.prefixes = cx.doc.settings().arch.as_ref().map_or_else(Vec::new, |a| {
                a.levels
                    .iter()
                    .filter(|l| a.space_number_prefixes.contains_key(&l.id))
                    .map(|l| OptionChoice {
                        typed: l.id.clone(),
                        label: l.name.clone(),
                        checked: false,
                        icon: None,
                        preview: None,
                        command: None,
                    })
                    .collect()
            });
            self.prefix_revision = Some(cx.doc.revision());
        }
        self.revision = Some(cx.doc.revision());
        self.drawn = Preview::default();
        self.error = None;
        self.last.clear();
        if self.directional {
            self.drawn.path = self.direction.clone();
            for (i, p) in self.direction.iter().enumerate() {
                self.drawn.squares.push(*p);
                self.drawn.labels.push(Label {
                    at: *p,
                    text: if i == 0 {
                        "Yön başlangıcı"
                    } else {
                        "Yön bitişi"
                    }
                    .into(),
                    offset: [8., -8.],
                    tone: Tone::Snap,
                });
            }
        }
        // Keep picks visible while a direction is still being specified, or a plan is refused.
        for uid in &self.uids {
            if let Some(e) = uid
                .parse()
                .ok()
                .and_then(|u| cx.doc.slot_of(u))
                .and_then(|s| cx.doc.get(s))
            {
                self.drawn
                    .strokes
                    .extend(Outline::of(&shape(e), Some([5., 3.]), 2., Tone::Accent).strokes);
            }
        }
        if self.uids.is_empty() {
            return;
        }
        let Some(input) = self.request() else {
            return;
        };
        match spaces_number::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.spaces {
                    if let Entity::Space(s) = e {
                        let number = s.space.number.clone().unwrap_or_default();
                        self.last.clone_from(&number);
                        if let Some(p) = cx.doc.tag_anchor(e) {
                            self.drawn.labels.push(Label {
                                at: Vec2::new(p.x, p.y),
                                // The original room number is still drawn underneath.
                                // Keep the proposed label short at large UI text sizes.
                                text: format!("→ {number}"),
                                offset: [8., -28.],
                                tone: Tone::Accent,
                            });
                        }
                    }
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.error = Some(error.message),
            _ => {}
        }
        if self.error.is_some() {
            for stroke in &mut self.drawn.strokes {
                stroke.tone = Tone::Danger;
            }
        }
    }

    fn direction_point(&mut self, p: Vec2, cx: &Context<'_>) {
        self.direction.push(p);
        self.see(cx);
    }
}
impl Tool for Number {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }
    fn snaps(&self) -> bool {
        false
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.uids = cx
            .selection
            .ids()
            .iter()
            .filter(|&&s| matches!(cx.doc.get(s), Some(Entity::Space(_))))
            .filter_map(|&s| cx.doc.uid(s).map(|u| u.to_string()))
            .collect();
        if self.uids.len() != cx.selection.len() {
            cx.say(Level::Info, "Önseçimden yalnız mahal nesneleri alındı.");
        }
        self.see(cx);
        Flow::Stay
    }
    fn point_count(&self) -> usize {
        self.uids.len()
    }
    fn prompt(&self) -> Prompt {
        let step = if self.asking {
            "ilk numarayı yazın: Z-001 gibi"
        } else if self.waiting_direction() {
            if self.direction.is_empty() {
                "sıralama yönünün başlangıcını seçin"
            } else {
                "sıralama yönünün bitişini seçin"
            }
        } else {
            "mahallere sırayla tıklayın; yeniden tıklama çıkarır, Enter önizlemeyi uygular"
        };
        Prompt::new(LABEL, step)
            .option("İlk numara", "N")
            .option("Sıralama", "S")
            .option("Yönü seç", "Y")
            .option("Geri al", "G")
            .option("Uygula", "Enter")
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.revision != Some(cx.doc.revision()) {
            self.see(cx);
        }
        self.drawn.path = if self.directional {
            let mut path = self.direction.clone();
            if path.len() == 1 {
                path.push(p.raw);
            }
            path
        } else {
            vec![]
        };
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.waiting_direction() {
            self.direction_point(p.raw, cx);
            return;
        }
        let accepts = |slot| {
            cx.doc.get(slot).is_some_and(|e| {
                matches!(e, Entity::Space(_))
                    && crate::selectable::entity_allowed(cx.draft.select_kinds, e)
                    && crate::selectable::on_selected_layer(cx.doc, e)
            })
        };
        let picked = cx
            .spatial
            .hits(p.raw, cx.pick_tolerance())
            .into_iter()
            .find(|&slot| accepts(slot))
            .or_else(|| {
                // This tool explicitly asks for a point inside a room. The ordinary
                // pick policy stays untouched. Use visible indexed candidates and
                // the same analytic outer/hole test as room geometry, not label hits.
                cx.spatial
                    .store()
                    .near(p.raw, 0.)
                    .into_iter()
                    .filter_map(|item| {
                        let slot = crate::spatial::slot(item.id)?;
                        if !accepts(slot) {
                            return None;
                        }
                        let Entity::Space(room) = cx.doc.get(slot)? else {
                            return None;
                        };
                        let areas = kentos_native_application::spaces::areas(&room.space.parts);
                        areas
                            .iter()
                            .any(|a| kentos_geometry_core::geom::region::inside_area(a, p.raw))
                            .then(|| (slot, kentos_geometry_core::geom::space::area(&areas)))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(slot, _)| slot)
            });
        let Some(uid) = picked.and_then(|slot| cx.doc.uid(slot).map(|u| u.to_string())) else {
            cx.say(Level::Info, "Bir mahalın içine veya sınırına tıklayın.");
            return;
        };
        if let Some(at) = self.uids.iter().position(|u| u == &uid) {
            self.uids.remove(at);
        } else {
            self.uids.push(uid);
        }
        self.see(cx);
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let Some(input) = self.request() else {
            cx.say(Level::Info, "Yön için iki farklı nokta seçin.");
            return Flow::Stay;
        };
        if let Some(output) = points::written(
            spaces_number::execute(&mut ExecutionContext::new(cx.doc), input),
            cx,
        ) {
            cx.say(
                Level::Success,
                format!(
                    "{} mahal numaralandı; etiket ve çizelge bağları korundu.",
                    output.kept.len()
                ),
            );
            Flow::Exit
        } else {
            self.see(cx);
            Flow::Stay
        }
    }
    fn cancel(&mut self, _: &mut Context<'_>) -> bool {
        false
    }
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let changed = if self.directional && !self.direction.is_empty() {
            self.direction.pop().is_some()
        } else {
            self.uids.pop().is_some()
        };
        if changed {
            self.see(cx);
        }
        changed
    }
    fn takes_words(&self) -> bool {
        self.asking
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.asking {
            let valid = self.configure("N", text, cx);
            if valid {
                self.asking = false;
            }
            return true;
        }
        match upper_tr(text.trim()).as_str() {
            "N" => {
                self.asking = true;
                true
            }
            "S" => self.configure(
                "S",
                if self.directional {
                    "input"
                } else {
                    "direction"
                },
                cx,
            ),
            "Y" => self.configure("Y", "pick", cx),
            "G" => self.undo_step(cx),
            _ if self.waiting_direction() => {
                if let Some(p) = cx.typed_point(text, self.direction.last().copied(), None) {
                    self.direction_point(p, cx);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
    fn settings(&self, _: &Format) -> Vec<Setting> {
        let mut rows = vec![
            Setting::text("N", "İlk numara", &self.first),
            Setting::choice("S", "Sıralama", self.options()),
            Setting::text("A", "Seçilen mahal", self.uids.len().to_string()).read_only(),
        ];
        if !self.prefixes.is_empty() {
            let mut row = Setting::choice("P", "Kat önekinden", self.prefixes.clone());
            row.value = "Kat seç".into();
            rows.push(row);
        }
        if self.directional {
            rows.push(Setting::choice(
                "Y",
                "Yön",
                choices(&[("pick", "Çizimden iki nokta seç")], "pick"),
            ));
            rows.push(Setting::notice("D", "Yön durumu", if self.direction.len() == 2 { "Yön hazır; mahallere tıklayabilir veya uygulayabilirsiniz." } else { "Başlangıç ve bitiş noktalarını seçin. Yön izdüşümü sıralar; satır/sütun rotası kurmaz." }));
        }
        if !self.last.is_empty() {
            rows.push(Setting::text("Z", "Son numara", &self.last).read_only());
        }
        rows.push(Setting::notice("H", "Önizleme", "Tıklama ekler/çıkarır. Enter bir kez uygular; Esc vazgeçer. Seçim dışındaki aynı numara işlemi durdurur."));
        if let Some(error) = &self.error {
            rows.push(Setting::notice("E", "Uygulanamıyor", error));
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match (key, value) {
            ("P", level) => {
                let Some(first) = cx
                    .doc
                    .settings()
                    .arch
                    .as_ref()
                    .and_then(|a| a.space_first_number(level))
                else {
                    return false;
                };
                self.first = first;
                self.asking = false;
            }
            ("N", _) => {
                if !kentos_commands::valid_space_number(value) {
                    cx.say(Level::Warn, "İlk numara en çok 256 karakter olmalı ve rakamla bitmeli: Z-001 gibi. Başında/sonunda boşluk veya kontrol karakteri kullanmayın.");
                    return false;
                }
                self.first = value.into();
                self.asking = false;
            }
            ("S", "input" | "direction") => {
                self.directional = value == "direction";
            }
            ("Y", "pick") => {
                self.directional = true;
                self.direction.clear();
            }
            _ => return false,
        }
        self.see(cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        match key {
            "S" => self.options(),
            "Y" => choices(&[("pick", "Çizimden iki nokta seç")], "pick"),
            _ => vec![],
        }
    }
    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }
    fn preview(&self, _: &Format) -> Preview {
        self.drawn.clone()
    }
}
