//! Arazi zemini (docs/adr/8174 §5): a click on a survey point makes the site's natural ground of its layer's points (Tür
//! T: or its graded ground), a terrain surface the sections draw (PAİY 57/2-a, EK-4 4.30). The natural ground without
//! ±0.00's elevation gives it: the mean of its heights at the building's corners (PAİY 12/4). With both grounds and
//! the parcel, the garden's earthwork is said: within the parcel, out of the building's footprint (hafriyat). A
//! setting, as a surface of the road's is, not an undo step.

use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::site_ground;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const ID: &str = "siteGround";

/// Arazi zemini.
#[derive(Clone, Debug, Default)]
pub struct GroundTool {
    /// The graded ground (Tesviye zemin), else the natural one (Tabii zemin).
    graded: bool,
    hover: Option<Vec2>,
    /// The survey point under the pointer and its layer's name.
    point: Option<(Slot, String)>,
}

/// Metres as a plan writes heights: 1185,42.
fn m2(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

impl GroundTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn name(&self) -> &'static str {
        if self.graded { site_ground::GRADED_NAME } else { site_ground::NATURAL_NAME }
    }

    /// The survey point (a point with a height) under `at`, and its layer's id.
    fn point_at(at: Vec2, cx: &Context<'_>) -> Option<(Slot, String)> {
        cx.spatial.hits(at, cx.pick_tolerance()).into_iter().find_map(|s| match cx.doc.get(s) {
            Some(Entity::Point(p)) if p.z.is_some() => Some((s, p.base.layer_id.clone())),
            _ => None,
        })
    }

    /// Makes the ground of the layer `layer`; says what it made, ±0.00 when it gave it, and the earthwork.
    fn write(&self, layer: &str, cx: &mut Context<'_>) {
        let (settings, made) = match site_ground::with_ground(cx.doc, layer, self.graded) {
            Ok(done) => done,
            Err(why) => {
                cx.say(Level::Warn, why);
                return;
            }
        };
        cx.doc.set_settings(settings);
        // A flat ground's one height once.
        let heights = if made.high - made.low < 0.005 {
            m2(made.low)
        } else {
            format!("{}–{}", m2(made.low), m2(made.high))
        };
        cx.say(Level::Success, format!("{} kuruldu: {} üçgen, kot {heights} m.", self.name(), made.triangles));
        // ±0.00 from the natural ground at the building's corners (PAİY 12/4), when the site has none.
        let datum = cx.doc.settings().arch.as_ref().and_then(|a| a.site.as_ref()).and_then(|s| s.datum);
        if !self.graded && datum.is_none() {
            if let Some(d) = site_ground::datum_from_corners(cx.doc) {
                let mut settings = cx.doc.settings().clone();
                let site = settings.arch.get_or_insert_with(Default::default).site.get_or_insert_with(Default::default);
                site.datum = Some((d * 100.0).round() / 100.0);
                cx.doc.set_settings(settings);
                cx.say(
                    Level::Info,
                    format!("±0,00 = {} m: bina köşelerinin tabii zemin kotlarının ortalaması (PAİY 12/4).", m2(d)),
                );
            } else {
                cx.say(
                    Level::Info,
                    "±0,00'ın mutlak kotu yok ve bina köşeleri bulunamadı: zemin kesitte çizilmez. Kotu Proje ayarları, İmar durumu'ndan verin.",
                );
            }
        }
        if let Some(e) = site_ground::earthwork(cx.doc) {
            cx.say(
                Level::Info,
                format!(
                    "Bahçe hafriyatı, parselde bina oturumu dışında: kazı {} m³, dolgu {} m³ ({} m²).",
                    m2(e.cut),
                    m2(e.fill),
                    m2(e.area)
                ),
            );
        }
    }
}

impl Tool for GroundTool {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        "Arazi zemini"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        Prompt::new("Arazi zemini", "kotlu noktaların katmanından bir noktaya tıklayın").option_with("Tür", "T", self.name())
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.point = Self::point_at(p.raw, cx);
        cx.selection.set_hover(self.point.as_ref().map(|(s, _)| *s));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match Self::point_at(p.raw, cx) {
            Some((_, layer)) => self.write(&layer, cx),
            None => cx.say(Level::Info, "Burada kotlu nokta yok: arazinin ölçü noktalarından birine tıklayın."),
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if upper_tr(js_trim(text)) == "T" {
            self.graded = !self.graded;
            let _ = cx;
            return true;
        }
        false
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        vec![
            Setting::choice(
                "T",
                "Tür",
                crate::tool_settings::choices(
                    &[("natural", site_ground::NATURAL_NAME), ("graded", site_ground::GRADED_NAME)],
                    if self.graded { "graded" } else { "natural" },
                ),
            ),
            Setting::hint(
                "P",
                "Zemin",
                "Tıklanan noktanın katmanındaki kotlu noktalardan bir arazi yüzeyi kurulur ve vaziyetin tabii ya da tesviye zemini olur. Kesitler onu çizer: tabii zemin kesikli, tesviye zemin kalın. Tabii zemin kurulurken ±0,00 yoksa bina köşelerinin tabii zemin ortalamasıdır (PAİY 12/4). İki zemin ve parsel varsa bahçe hafriyatı söylenir: parselde, bina oturumu dışında.",
            ),
        ]
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.settings(&Format::default()).into_iter().find(|s| s.key == key).map_or_else(Vec::new, |s| s.choices)
    }

    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }

    fn configure(&mut self, key: &str, value: &str, _cx: &mut Context<'_>) -> bool {
        match (key, value) {
            ("T", "natural") => self.graded = false,
            ("T", "graded") => self.graded = true,
            _ => return false,
        }
        true
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            tag: self.hover.map(|at| Tag {
                at,
                lines: vec![match &self.point {
                    Some(_) => format!("{}: bu katmanın noktaları", self.name()),
                    None => self.name().to_owned(),
                }],
            }),
            tag_tone: Tone::Accent,
            ..Preview::default()
        }
    }
}
