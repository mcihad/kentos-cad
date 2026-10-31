//! Modül yerleştir (docs/adr/0775 §2): a room module placed again as a linked instance. Its settings say which module,
//! its turn, whether it is mirrored, its level (Etkin kat, none, or a level), its phase and its numbering (a prefix
//! and a suffix on its spaces' and openings' numbers); the instance follows the pointer, its objects drawn from
//! `cad.modules.edit`'s plan, and a click places it in one undo step (“Oda modülü”).

use kentos_commands::{ArchPhase, ModuleNumbering, ModulesEdit, ModulesOperation};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, modules_edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::{Setting, choices};

pub const ID: &str = "modulePlace";
pub const LABEL: &str = "Modül yerleştir";

/// Which level a placement takes: Etkin kat, none, or one named.
#[derive(Clone, Debug, PartialEq)]
enum LevelChoice {
    Active,
    None,
    Named(String),
}

#[derive(Clone, Debug)]
pub struct ModulePlace {
    module: Option<String>,
    rotation: f64,
    mirror: bool,
    level: LevelChoice,
    phase: Option<ArchPhase>,
    prefix: String,
    suffix: String,
    hover: Option<Vec2>,
    /// The project's modules (id, name) and levels (id, name), read when the tool starts and as it draws.
    known: Vec<(String, String)>,
    levels: Vec<(String, String)>,
    drawn: Preview,
    why: Option<String>,
    count: usize,
}

impl Default for ModulePlace {
    fn default() -> Self {
        Self {
            module: None,
            rotation: 0.0,
            mirror: false,
            level: LevelChoice::Active,
            phase: None,
            prefix: String::new(),
            suffix: String::new(),
            hover: None,
            known: Vec::new(),
            levels: Vec::new(),
            drawn: Preview::default(),
            why: None,
            count: 0,
        }
    }
}

impl ModulePlace {
    pub fn new() -> Self {
        Self::default()
    }

    fn modules(cx: &Context<'_>) -> Vec<(String, String)> {
        cx.doc
            .settings()
            .arch
            .as_ref()
            .map(|a| {
                a.modules
                    .iter()
                    .filter(|m| !m.removed)
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn request(&self, cx: &Context<'_>, at: Vec2) -> Option<ModulesEdit> {
        let module = self
            .module
            .clone()
            .or_else(|| Self::modules(cx).first().map(|m| m.0.clone()))?;
        let level = match &self.level {
            LevelChoice::Active => cx.level.map(str::to_owned),
            LevelChoice::None => None,
            LevelChoice::Named(l) => Some(l.clone()),
        };
        let numbering = ModuleNumbering {
            prefix: (!self.prefix.is_empty()).then(|| self.prefix.clone()),
            suffix: (!self.suffix.is_empty()).then(|| self.suffix.clone()),
        };
        Some(ModulesEdit {
            operation: ModulesOperation::Place {
                module,
                at: points::wire(at),
                rotation: (self.rotation != 0.0).then_some(self.rotation),
                mirror: self.mirror,
                level,
                phase: self.phase,
                numbering: (!numbering.is_empty()).then_some(numbering),
            },
            expected_revision: None,
        })
    }

    fn learn(&mut self, cx: &Context<'_>) {
        self.known = Self::modules(cx);
        self.levels = cx
            .doc
            .settings()
            .arch
            .as_ref()
            .map(|a| {
                a.levels
                    .iter()
                    .map(|l| (l.id.clone(), l.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.learn(cx);
        self.drawn = Preview::default();
        self.why = None;
        self.count = 0;
        let Some(at) = self.hover else { return };
        let Some(input) = self.request(cx, at) else {
            self.why = Some("Projede oda modülü yok: önce bir banyoyu ya da odayı seçip Oda modülleri'nden tanımlayın.".into());
            return;
        };
        match modules_edit::placed_objects(cx.doc, &input) {
            Ok(objects) => {
                self.count = objects.len();
                for e in &objects {
                    self.drawn
                        .strokes
                        .extend(Outline::of(&shape(e), None, 1.5, Tone::Accent).strokes);
                }
            }
            Err(why) => self.why = Some(why),
        }
    }

    fn write(&mut self, cx: &mut Context<'_>, at: Vec2) -> bool {
        let Some(input) = self.request(cx, at) else {
            cx.say(Level::Warn, "Projede oda modülü yok: önce bir banyoyu ya da odayı seçip Oda modülleri'nden tanımlayın.");
            return false;
        };
        let result = modules_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(done) = points::written(result, cx) else {
            self.see(cx);
            return false;
        };
        cx.say(
            Level::Success,
            format!(
                "Modül yerleştirildi: {} örneği, {} nesne.",
                done.instance.unwrap_or_default(),
                done.created.len()
            ),
        );
        self.see(cx);
        true
    }

    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let words = upper_tr(js_trim(value));
        match key {
            "M" => {
                let list = Self::modules(cx);
                match list
                    .iter()
                    .find(|(id, name)| id == value || upper_tr(name) == words)
                {
                    Some((id, _)) => {
                        self.module = Some(id.clone());
                        true
                    }
                    None => false,
                }
            }
            "D" => match parse_number(js_trim(value).trim_end_matches('°'))
                .filter(|v| v.is_finite())
            {
                Some(deg) => {
                    self.rotation = deg.to_radians();
                    true
                }
                None => false,
            },
            "A" => match words.as_str() {
                "YES" | "AÇIK" | "EVET" => {
                    self.mirror = true;
                    true
                }
                "NO" | "KAPALI" | "HAYIR" => {
                    self.mirror = false;
                    true
                }
                _ => false,
            },
            "L" => {
                self.level = match value {
                    "active" => LevelChoice::Active,
                    "none" => LevelChoice::None,
                    other => {
                        let known = cx
                            .doc
                            .settings()
                            .arch
                            .as_ref()
                            .is_some_and(|a| a.levels.iter().any(|l| l.id == other));
                        if !known {
                            return false;
                        }
                        LevelChoice::Named(other.to_owned())
                    }
                };
                true
            }
            "E" => {
                self.phase = if value == "none" {
                    None
                } else {
                    match ArchPhase::ALL.iter().find(|p| serde_name(**p) == value) {
                        Some(p) => Some(*p),
                        None => return false,
                    }
                };
                true
            }
            "P" => {
                self.prefix = js_trim(value).to_owned();
                true
            }
            "S" => {
                self.suffix = js_trim(value).to_owned();
                true
            }
            _ => false,
        }
    }
}

fn serde_name(p: ArchPhase) -> String {
    match p {
        ArchPhase::Existing => "existing",
        ArchPhase::Demolish => "demolish",
        ArchPhase::New => "new",
        ArchPhase::Temporary => "temporary",
    }
    .to_owned()
}

impl Tool for ModulePlace {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.learn(cx);
        if self.module.is_none() {
            self.module = self.known.first().map(|m| m.0.clone());
        }
        Flow::Stay
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "modülün taban noktasını yerleştirmek için tıklayın")
            .option("Modül", "M")
            .option("Açı", "D")
            .option_with("Ayna", "A", if self.mirror { "Açık" } else { "Kapalı" })
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.hover = Some(p);
        self.write(cx, p)
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(js_trim(text)).as_str() {
            "A" => {
                self.mirror = !self.mirror;
                self.see(cx);
                true
            }
            _ => match cx.typed_point(text, None, self.hover) {
                Some(p) => self.accept_point(p, cx),
                None => false,
            },
        }
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        let mut rows = Vec::new();
        let module = self.module.clone().unwrap_or_default();
        let names: Vec<(&str, &str)> = self
            .known
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        if names.is_empty() {
            rows.push(Setting::text("M", "Modül", "Yok").read_only());
        } else {
            rows.push(Setting::choice("M", "Modül", choices(&names, &module)));
        }
        rows.push(Setting::text(
            "D",
            "Açı",
            format!("{}°", crate::fixed(self.rotation.to_degrees(), 2)),
        ));
        rows.push(Setting::toggle(
            "A",
            "Aynalı (kendi y ekseninde)",
            self.mirror,
        ));
        let level = match &self.level {
            LevelChoice::Active => "active".to_owned(),
            LevelChoice::None => "none".to_owned(),
            LevelChoice::Named(l) => l.clone(),
        };
        let mut levels: Vec<(&str, &str)> = vec![("active", "Etkin kat"), ("none", "Katsız")];
        levels.extend(self.levels.iter().map(|(a, b)| (a.as_str(), b.as_str())));
        rows.push(Setting::choice("L", "Kat", choices(&levels, &level)));
        let phase = self.phase.map_or_else(|| "none".to_owned(), serde_name);
        let mut phases: Vec<(String, String)> = vec![("none".into(), "Tanımdaki gibi".into())];
        phases.extend(
            ArchPhase::ALL
                .iter()
                .map(|p| (serde_name(*p), p.label().to_owned())),
        );
        let refs: Vec<(&str, &str)> = phases
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        rows.push(Setting::choice("E", "Evre", choices(&refs, &phase)));
        rows.push(Setting::text(
            "P",
            "Numara öneki",
            if self.prefix.is_empty() {
                "Yok".into()
            } else {
                self.prefix.clone()
            },
        ));
        rows.push(Setting::text(
            "S",
            "Numara soneki",
            if self.suffix.is_empty() {
                "Yok".into()
            } else {
                self.suffix.clone()
            },
        ));
        if self.count > 0 {
            rows.push(Setting::hint(
                "T",
                "Örnek",
                format!("{} nesne yerleşecek", self.count),
            ));
        }
        rows.push(Setting::hint(
            "H",
            "Oda modülü",
            "Örnek bağlıdır: kaynak güncellenince değişikliği alır. Duvar, kapı, mahal, etiket ve ölçü bağları örneğin \
             kendi nesnelerine kurulur; numaralar önek ve sonekle ayrılır.",
        ));
        if let Some(why) = &self.why {
            rows.push(Setting::notice("W", "Yerleştirilemiyor", why.clone()));
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

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if self.apply(key, value, cx) {
            self.see(cx);
            return true;
        }
        cx.say(
            Level::Warn,
            match key {
                "M" => {
                    "Projede bu adda modül yok; Oda modülleri penceresindeki adlardan birini yazın."
                }
                "D" => "Açıyı derece olarak bir sayıyla yazın; örneğin 90.",
                "L" => "Projede bu kat yok.",
                _ => "Geçerli bir değer seçin.",
            },
        );
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        out.tag = self.hover.map(|at| {
            let mut lines = vec![LABEL.to_owned()];
            match &self.why {
                Some(why) => lines.extend(crate::wall_dimensions::wrapped(why)),
                None => lines.push(format!(
                    "{}{}",
                    if self.mirror { "Aynalı · " } else { "" },
                    format_args!("{} nesne", self.count)
                )),
            }
            Tag { at, lines }
        });
        out
    }
}
