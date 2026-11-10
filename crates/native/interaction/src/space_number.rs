//! Select/click rooms and review the shared semantic numbering plan before one write.
//!
//! EK-3 (docs/adr/8027; 2026 Dijital Hazırlama Yönetmeliği, EK-3 Mahal İsimlendirme Esasları): a room's number is
//! Kat_Bölüm_Sıra, three characters each, joined by “_” — its storey's code (EK-2 Tablo 2.14: K00 the ground floor, K01
//! the first, B01 the first basement, NNN none), the project's section or use code, and its place counted from 001,
//! clockwise. With EK-3 on, the first number is made from Etkin kat, the rooms go clockwise round their middle from
//! north, and with none clicked Enter numbers every room of the storey.
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
    /// Saat yönü (EK-3 3.2.3): the rooms clockwise round their middle, from north.
    clockwise: bool,
    /// EK-3's Kat_Bölüm_Sıra and its section code.
    ek3: bool,
    section: String,
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
            clockwise: false,
            ek3: false,
            section: "001".into(),
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
    fn request(&self, cx: &Context<'_>) -> Option<SpacesNumber> {
        self.request_for(&self.uids, cx)
    }

    /// The command's input for `uids` (EK-3's preview: every room of the storey before any is clicked).
    fn request_for(&self, uids: &[String], cx: &Context<'_>) -> Option<SpacesNumber> {
        if self.waiting_direction() {
            return None;
        }
        Some(SpacesNumber {
            uids: if self.clockwise && !self.directional {
                clockwise(uids, cx)
            } else {
                uids.to_vec()
            },
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
            &[
                ("input", "Tıklama sırası"),
                ("direction", "Yön izdüşümü"),
                ("clockwise", "Saat yönü (EK-3)"),
            ],
            if self.directional {
                "direction"
            } else if self.clockwise {
                "clockwise"
            } else {
                "input"
            },
        )
    }

    /// EK-3's first number for Etkin kat: its code, the section, 001.
    fn ek3_first(&self, cx: &Context<'_>) -> String {
        let levels = cx
            .doc
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.as_slice())
            .unwrap_or_default();
        format!("{}_{}_001", ek3_level(levels, cx.level), self.section)
    }

    /// With EK-3 and none clicked: every room of Etkin kat.
    fn storey_rooms(cx: &Context<'_>) -> Vec<String> {
        cx.doc
            .entities()
            .filter_map(|e| match e {
                Entity::Space(s) if s.space.arch.level.as_deref() == cx.level => {
                    cx.doc.uid(kentos_domain::Slot(s.base.id))
                }
                _ => None,
            })
            .map(|u| u.to_string())
            .collect()
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
        // EK-3 with none clicked: the whole storey's numbers shown before Enter (docs/adr/8027).
        let uids = if self.ek3 && self.uids.is_empty() {
            Self::storey_rooms(cx)
        } else {
            self.uids.clone()
        };
        if uids.is_empty() {
            return;
        }
        let Some(input) = self.request_for(&uids, cx) else {
            return;
        };
        match spaces_number::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.spaces {
                    if let Entity::Space(s) = e {
                        let number = s.space.number.clone().unwrap_or_default();
                        self.last.clone_from(&number);
                        if let Some(p) = cx.doc.tag_anchor(e) {
                            let text = format!("→ {number}");
                            // About its middle over the room's tag (a UI letter is some 6.6 px wide at 11 px), so that
                            // a narrow room's stays inside it; the room's present number is still drawn underneath.
                            let half = text.chars().count() as f32 * 3.3;
                            self.drawn.labels.push(Label {
                                at: Vec2::new(p.x, p.y),
                                text,
                                offset: [-half, -28.],
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
            .option_with(
                "EK-3 numarası",
                "E",
                if self.ek3 { "Açık" } else { "Kapalı" },
            )
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
        // EK-3 with none clicked: the whole storey (docs/adr/8027).
        if self.ek3 && self.uids.is_empty() {
            self.uids = Self::storey_rooms(cx);
            if self.uids.is_empty() {
                cx.say(Level::Info, "Etkin katta numaralanacak mahal yok.");
                return Flow::Stay;
            }
        }
        let Some(input) = self.request(cx) else {
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
            // EK-3 on or off from the keyboard (docs/adr/8027).
            "E" => self.configure("E", if self.ek3 { "no" } else { "yes" }, cx),
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
            Setting::toggle("E", "EK-3 numarası (Kat_Bölüm_Sıra)", self.ek3),
            Setting::text("N", "İlk numara", &self.first),
            Setting::choice("S", "Sıralama", self.options()),
            Setting::text("A", "Seçilen mahal", self.uids.len().to_string()).read_only(),
        ];
        if self.ek3 {
            rows.insert(
                1,
                Setting::text("B", "Bölüm/Fonksiyon (3 karakter)", &self.section),
            );
        }
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
            ("S", "input" | "direction" | "clockwise") => {
                self.directional = value == "direction";
                self.clockwise = value == "clockwise";
            }
            ("E", _) => {
                self.ek3 = matches!(
                    crate::prompt::fold_tr(value.trim()).as_str(),
                    "YES" | "AÇIK" | "ACIK" | "EVET" | "1" | "TRUE"
                );
                if self.ek3 {
                    self.first = self.ek3_first(cx);
                    self.clockwise = true;
                    self.directional = false;
                }
                self.asking = false;
            }
            ("B", _) => {
                let code = upper_tr(value.trim());
                if code.chars().count() != 3 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
                    cx.say(
                        Level::Warn,
                        "Bölüm/Fonksiyon üç harf ya da rakamdır (EK-3 3.2.2): 001, D01, ORT gibi.",
                    );
                    return false;
                }
                self.section = code;
                if self.ek3 {
                    self.first = self.ek3_first(cx);
                }
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

/// EK-2 Tablo 2.14's code of storey `level` among `levels`: the ground floor K00 (the one named Zemin, else the one
/// nearest ±0.00), those above K01, K02 …, those below B01, B02 …; NNN for a room of no storey (Kattan bağımsız).
pub fn ek3_level(levels: &[kentos_commands::ArchLevel], level: Option<&str>) -> String {
    let Some(id) = level else {
        return "NNN".into();
    };
    let mut order: Vec<&kentos_commands::ArchLevel> = levels.iter().collect();
    order.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let Some(at) = order.iter().position(|l| l.id == id) else {
        return "NNN".into();
    };
    let ground = order
        .iter()
        .position(|l| crate::prompt::fold_tr(&l.name).contains("ZEMIN"))
        .or_else(|| {
            order
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.elevation.abs().total_cmp(&b.1.elevation.abs()))
                .map(|(i, _)| i)
        })
        .unwrap_or(0);
    let k = at as i64 - ground as i64;
    if k >= 0 {
        format!("K{k:02}")
    } else {
        format!("B{:02}", -k)
    }
}

/// `uids`' rooms clockwise round their labels' middle, from north (EK-3 3.2.3: “saat yönünde arttırılır”).
fn clockwise(uids: &[String], cx: &Context<'_>) -> Vec<String> {
    let mut placed: Vec<(String, Vec2)> = uids
        .iter()
        .filter_map(|u| {
            let e = u
                .parse()
                .ok()
                .and_then(|u| cx.doc.slot_of(u))
                .and_then(|s| cx.doc.get(s))?;
            let kentos_geometry_core::entity::Shape::Space { parts, .. } = shape(e) else {
                return None;
            };
            Some((
                u.clone(),
                kentos_geometry_core::geom::space::label_at(&parts)?,
            ))
        })
        .collect();
    if placed.is_empty() {
        return uids.to_vec();
    }
    let n = placed.len() as f64;
    let c = Vec2::new(
        placed.iter().map(|(_, p)| p.x).sum::<f64>() / n,
        placed.iter().map(|(_, p)| p.y).sum::<f64>() / n,
    );
    let turn = |p: Vec2| {
        let a = (p.x - c.x).atan2(p.y - c.y);
        if a < 0.0 {
            a + std::f64::consts::TAU
        } else {
            a
        }
    };
    placed.sort_by(|a, b| turn(a.1).total_cmp(&turn(b.1)).then(a.0.cmp(&b.0)));
    placed.into_iter().map(|(u, _)| u).collect()
}

#[cfg(test)]
mod ek3_tests {
    use super::ek3_level;
    use kentos_commands::ArchLevel;

    fn level(id: &str, name: &str, elevation: f64) -> ArchLevel {
        ArchLevel {
            id: id.into(),
            name: name.into(),
            elevation,
            height: None,
        }
    }

    #[test]
    fn storeys_are_coded_as_ek2_table_2_14_says() {
        let levels = [
            level("b", "Bodrum kat", -3.0),
            level("z", "Zemin kat", 0.0),
            level("1", "1. kat", 3.0),
            level("2", "2. kat", 6.0),
        ];
        assert_eq!(ek3_level(&levels, Some("z")), "K00");
        assert_eq!(ek3_level(&levels, Some("1")), "K01");
        assert_eq!(ek3_level(&levels, Some("2")), "K02");
        assert_eq!(ek3_level(&levels, Some("b")), "B01");
        assert_eq!(ek3_level(&levels, None), "NNN");
        // No Zemin by name: the one nearest ±0.00 is the ground floor.
        let unnamed = [level("a", "A", -0.1), level("c", "C", 2.9)];
        assert_eq!(ek3_level(&unnamed, Some("c")), "K01");
    }
}
