//! Gölge (docs/adr/8177 §3.3): the building's shadow on the ground at ±0.00 on a day at an hour, or every hour of the
//! day from 09:00 to 15:00. Its outlines are previewed as the settings change; Enter (or a click) writes it on
//! MM-XXXX-GOLGE, one undo step, the old shadows taken away, and the tool leaves. The place is the project's coordinate
//! system's, else the latitude (E) and longitude (B) typed; the time zone (Z) Türkiye's UTC+3 until changed. What was
//! typed and chosen is kept for as long as the app lives (`Memory::sun_shadow`).

use kentos_geometry_core::geom::sun;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::shadows;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};
use crate::tool_settings::{Setting, choices};

pub const ID: &str = "sunShadow";

/// The days a shadow is drawn for most: the winter solstice, the equinoxes, the summer solstice (their key, name, day
/// and month); 3B görünüm's Güneş offers them too.
pub const DAYS: [(&str, &str, u32, u32); 4] = [
    ("aralik", "21 Aralık", 21, 12),
    ("mart", "21 Mart", 21, 3),
    ("haziran", "21 Haziran", 21, 6),
    ("eylul", "23 Eylül", 23, 9),
];

/// The hours: one of them, or the day's from 09:00 to 15:00 hour by hour.
const HOURS: [(&str, &str); 4] = [("9", "09:00"), ("12", "12:00"), ("15", "15:00"), ("gun", "09:00–15:00 saatlik")];

/// What the tool keeps from one run to the next: the latitude and longitude typed, the time zone (UTC+hours), the
/// day's and the hours' places in their lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kept {
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub zone: f64,
    pub day: usize,
    pub hours: usize,
}

impl Kept {
    /// A new app's: no place, Türkiye's time, 21 Aralık at 12:00.
    pub const DEFAULT: Kept = Kept { lat: None, lon: None, zone: 3.0, day: 0, hours: 1 };
}

impl Default for Kept {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A value asked for by its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Lat,
    Lon,
    Zone,
}

/// Gölge.
#[derive(Clone, Debug, Default)]
pub struct ShadowTool {
    kept: Kept,
    /// Whether the project's coordinate system places the building: else Enlem and Boylam are the prompt's options.
    located: bool,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    /// The shadows as last made for the preview, at the drawing's revision then, and what to say of them: Enter writes
    /// them without making them again. Or why none.
    made: Option<(u64, Vec<shadows::Hour>, String)>,
    outlines: Vec<Stroke>,
    why: Option<String>,
    done: bool,
}

impl ShadowTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The clock times drawn.
    fn hour_list(&self) -> Vec<f64> {
        match HOURS[self.kept.hours].0 {
            "gun" => (9..=15).map(f64::from).collect(),
            h => vec![h.parse().unwrap_or(12.0)],
        }
    }

    /// The shadows now, each time's; what to say of them; or why none.
    fn shadows(&self, cx: &Context<'_>) -> Result<(Vec<shadows::Hour>, String), String> {
        let k = &self.kept;
        let place = shadows::place(cx.doc, k.lat.zip(k.lon), k.zone)?;
        let (_, name, d, m) = DAYS[k.day];
        let day = sun::day_of_year(d, m).unwrap_or(1);
        // The hours the sun is up, one model for them all, side by side.
        let up: Vec<_> = (self.hour_list().into_iter())
            .map(|h| (h, shadows::sun_at(&place, day, h)))
            .filter(|(_, (s, _))| s.elevation > 0.5)
            .collect();
        let first = up.first().map(|(_, (s, _))| *s);
        let caster = shadows::caster(cx.doc);
        let areas = shadows::shadows_of(&caster, &up.iter().map(|(_, (_, plan))| *plan).collect::<Vec<_>>());
        let out: Vec<shadows::Hour> =
            up.iter().zip(areas).map(|((h, (_, plan)), a)| (shadows::clock(*h), *plan, a)).collect();
        let when = HOURS[k.hours].1;
        if out.iter().all(|(_, _, a)| a.is_empty()) {
            return Err(format!("{name} {when}: güneş ufkun altında ya da binanın gölgesi yok."));
        }
        let mut said = match (out.len(), first) {
            (1, Some(s)) => format!(
                "{name} {}, güneş yüksekliği {}°, azimut {}°",
                out[0].0,
                (s.elevation * 10.0).round() / 10.0,
                s.azimuth.round()
            ),
            _ => format!("{name} {when}, {} saat", out.len()),
        };
        if !place.notes.is_empty() {
            said.push_str(&format!(" ({})", place.notes.join("; ")));
        }
        Ok((out, said.replace('.', ",")))
    }

    /// The preview made again: the shadows' outlines, or why none.
    fn see(&mut self, cx: &Context<'_>) {
        self.outlines.clear();
        self.made = None;
        match self.shadows(cx) {
            Ok((list, said)) => {
                self.why = None;
                for a in list.iter().flat_map(|(_, _, areas)| areas) {
                    let pts: Vec<Vec2> = a.outer.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
                    self.outlines.push(Stroke::solid(pts, true));
                }
                self.made = Some((cx.doc.revision(), list, said));
            }
            Err(why) => self.why = Some(why),
        }
    }

    /// Kept for the next run, and the preview made again.
    fn changed(&mut self, cx: &mut Context<'_>) {
        cx.memory.sun_shadow = self.kept;
        self.see(cx);
    }

    /// Writes the shadows on their layer; says what it drew, or why not.
    fn write(&mut self, cx: &mut Context<'_>) {
        let made = match self.made.take() {
            Some((revision, list, said)) if revision == cx.doc.revision() => Ok((list, said)),
            _ => self.shadows(cx),
        };
        let (list, said) = match made {
            Ok(done) => done,
            Err(why) => {
                cx.say(Level::Warn, why);
                return;
            }
        };
        let group = cx.doc.begin_group(shadows::STEP);
        let mut out = Err(String::new());
        let written = crate::arch_layer::writing(&crate::arch_layer::Kind::Shadow, cx, |layer, cx| {
            out = shadows::draw(cx.doc, layer, &list);
            out.is_ok()
        });
        match (written, out) {
            (true, Ok(_)) => {
                cx.doc.end_group(group);
                cx.say(Level::Success, format!("Gölge çizildi: {said}."));
                self.done = true;
            }
            (_, Err(why)) => {
                cx.doc.cancel_group(group);
                if !why.is_empty() {
                    cx.say(Level::Warn, why);
                }
            }
            (false, Ok(_)) => cx.doc.cancel_group(group),
        }
    }

    /// Sets `key` from `value`; whether it took.
    fn apply(&mut self, key: &str, value: &str) -> bool {
        let n = parse_number(js_trim(value).replace(',', ".").as_str());
        let k = &mut self.kept;
        match (key, n) {
            ("E", Some(v)) if (-90.0..=90.0).contains(&v) => k.lat = Some(v),
            ("B", Some(v)) if (-180.0..=180.0).contains(&v) => k.lon = Some(v),
            ("Z", Some(v)) if (-12.0..=14.0).contains(&v) => k.zone = v,
            _ => return false,
        }
        true
    }
}

impl Tool for ShadowTool {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        shadows::STEP
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(crate::arch_layer::Kind::Shadow)
    }

    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        let k = &self.kept;
        let unplaced = !self.located && (k.lat.is_none() || k.lon.is_none());
        let words = match self.ask {
            Some(Ask::Lat) => "binanın enlemini yazın (derece, kuzey artı)",
            Some(Ask::Lon) => "binanın boylamını yazın (derece, doğu artı)",
            Some(Ask::Zone) => "saat dilimini yazın (UTC'den saat farkı)",
            // Short: the options beside it must show (the preview's tag says why in full).
            None if unplaced => "binanın enlemini (E) ve boylamını (B) verin",
            None => "gölgeyi çizmek için Enter'a basın ya da tıklayın",
        };
        let prompt = Prompt::new(shadows::STEP, words)
            .option_with("Tarih", "T", DAYS[k.day].1)
            .option_with("Saat", "S", HOURS[k.hours].1);
        if self.located {
            return prompt;
        }
        // A letter reaches the tool only as the prompt's option.
        let degrees = |v: Option<f64>| v.map_or_else(|| "—".to_owned(), |v| format!("{v}°").replace('.', ","));
        prompt.option_with("Enlem", "E", degrees(k.lat)).option_with("Boylam", "B", degrees(k.lon))
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.kept = cx.memory.sun_shadow;
        self.located = shadows::located(cx.doc);
        self.see(cx);
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        self.write(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(ask) = self.ask {
            let key = match ask {
                Ask::Lat => "E",
                Ask::Lon => "B",
                Ask::Zone => "Z",
            };
            if !self.apply(key, text) {
                cx.say(Level::Warn, "Değer okunamadı: enlem −90…90, boylam −180…180, saat dilimi −12…14.");
                return true;
            }
            self.ask = None;
            self.changed(cx);
            return true;
        }
        match upper_tr(js_trim(text)).as_str() {
            "T" => self.kept.day = (self.kept.day + 1) % DAYS.len(),
            "S" => self.kept.hours = (self.kept.hours + 1) % HOURS.len(),
            "E" => self.ask = Some(Ask::Lat),
            "B" => self.ask = Some(Ask::Lon),
            "Z" => self.ask = Some(Ask::Zone),
            _ => return false,
        }
        if self.ask.is_none() {
            self.changed(cx);
        }
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.ask.take().is_some()
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        let k = &self.kept;
        let number = |v: Option<f64>| v.map_or_else(|| "—".to_owned(), |v| v.to_string().replace('.', ","));
        vec![
            Setting::choice("T", "Tarih", choices(&DAYS.map(|(key, name, _, _)| (key, name)), DAYS[k.day].0)),
            Setting::choice("S", "Saat", choices(&HOURS, HOURS[k.hours].0)),
            Setting::text("E", "Enlem", number(k.lat)),
            Setting::text("B", "Boylam", number(k.lon)),
            Setting::text("Z", "Saat dilimi (UTC+)", k.zone.to_string().replace('.', ",")),
            Setting::hint(
                "P",
                shadows::STEP,
                "Binanın ±0,00'daki zemine düşen gölgesi: seçilen gün ve saatte, ya da 09:00–15:00 arası saat saat. \
                 Konum projenin koordinat sisteminden, yoksa yazılan enlem ve boylamdan; coğrafi kuzey projeden ya da \
                 kuzey okundan, yoksa plan kuzeyi. Saat dilimi Türkiye için UTC+3. Gölge yeniden çizilince eskisinin \
                 yerini alır.",
            ),
        ]
    }

    fn option_choices(&self, key: &str) -> Vec<crate::tool::OptionChoice> {
        self.settings(&Format::default()).into_iter().find(|s| s.key == key).map_or_else(Vec::new, |s| s.choices)
    }

    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let took = match key {
            "T" => DAYS.iter().position(|d| d.0 == value).map(|i| self.kept.day = i).is_some(),
            "S" => HOURS.iter().position(|h| h.0 == value).map(|i| self.kept.hours = i).is_some(),
            _ => self.apply(key, value),
        };
        if took {
            self.ask = None;
            self.changed(cx);
        }
        took
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.write(cx);
        if self.done { Flow::Exit } else { Flow::Stay }
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            strokes: self.outlines.clone(),
            tag: self.hover.map(|at| {
                let said = self.made.as_ref().map(|(_, _, said)| said.clone()).unwrap_or_default();
                Tag { at, lines: vec![self.why.clone().unwrap_or(said)] }
            }),
            tag_tone: if self.why.is_some() { Tone::Danger } else { Tone::Accent },
            ..Preview::default()
        }
    }
}
