//! Hizmet alanı (docs/adr/0209 §6; the web's `tools/networkAreaTool.ts`):
//! facilities clicked; the network within each break of the chosen cost
//! shows at once (its lines, asked first) and then its areas (the lines'
//! buffers, Kenar payı wide; each band paler than the one inside it).
//! Aralıklar (R) are rising numbers in the cost's unit; Yön (Y) from or to the
//! facilities; Biçim (B) discs or rings; Birleşik (İ) one area a band for all
//! facilities or one each; Çizgiler (Ç) writes the lines too. Enter writes
//! the areas to Yaklaşık hizmet alanı (and the lines to Ulaşılan yollar) in one
//! step. The breaks and Kenar payı come first on the line and stand in the
//! tool's settings from the start; the areas are said to be the lines'
//! surroundings, the lines the reach (docs/adr/9333).

use std::collections::BTreeMap;

use kentos_commands::{NetworkCostKind, NetworkDef, NetworkKind};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::jsmath::TAU;
use kentos_geometry_core::ops::network::Served;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::edit_geometry;

use serde_json::{Value, json};

use super::base::{Base, Changed, PointKind, REACH_PX, outline, refusal_words, stroke};
use super::record::{self, AnalysisRun};
use super::result::{AREA_LAYER, AREA_LINES_LAYER, ResultWrite, line_geometry, write_results};
use super::{Answer, NetworkQuestion, NetworkReply};
use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Area, Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tone, Tool, Values, ViewChange,
};

/// The tool's id: its command is `tool.netServiceArea`.
pub const ID: &str = "netServiceArea";
pub const LABEL: &str = "Hizmet alanı";
/// At most this many breaks.
pub const MAX_BREAKS: usize = 10;

/// What a key asked for, waiting for its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asking {
    Breaks,
    Trim,
}

/// A number as a label shows it: the display rule's, without trailing zeros (“0–300”).
pub fn plain(v: f64) -> String {
    let s = fixed(v, 6);
    let s = s.trim_end_matches('0');
    s.strip_suffix('.').unwrap_or(s).to_owned()
}

/// The kind of a network's cost `c`: 0 the length, 1 a speed, 2 a field (Hizmet alanı's breaks are kept for each).
pub fn cost_kind(n: &NetworkDef, c: usize) -> usize {
    match c.checked_sub(1).and_then(|i| n.costs.get(i)) {
        None => 0,
        Some(cost) if cost.kind == NetworkCostKind::Speed => 1,
        Some(_) => 2,
    }
}

/// The breaks of a kind of cost when none are typed: 500, 1000, 1500 m; 5, 10, 15 dk; 10, 20, 30.
pub fn default_breaks(kind: usize) -> [f64; 3] {
    match kind {
        0 => [500.0, 1000.0, 1500.0],
        1 => [5.0, 10.0, 15.0],
        _ => [10.0, 20.0, 30.0],
    }
}

/// The breaks of cost `c` of network `n`, in its unit: the ones last typed for its kind, else its kind's.
pub fn breaks_of(n: &NetworkDef, c: usize, kept: &[Values; 3]) -> Vec<f64> {
    let kind = cost_kind(n, c);
    let typed = kept[kind];
    if typed.is_empty() {
        default_breaks(kind).to_vec()
    } else {
        typed.as_slice().to_vec()
    }
}

/// The answer drawn: the lines' points and the areas' rings with their bands.
#[derive(Clone, Debug, Default)]
struct Shown {
    served: Served,
    lines: Vec<Vec<Vec2>>,
    areas: Vec<(usize, Vec<Vec<Vec2>>)>,
    full: bool,
}

/// What the prompt and the preview show (the host's world as last seen).
#[derive(Clone, Debug, Default)]
struct Seen {
    network: Option<NetworkDef>,
    list: Vec<NetworkDef>,
    cost: usize,
    breaks: Vec<f64>,
    format: Option<Format>,
    toward: bool,
    rings: bool,
    separate: bool,
    trim: f64,
    lines: bool,
    /// The approximate areas are written (T35).
    areas: bool,
    time: Option<u32>,
    profile: String,
    day: Option<i64>,
    /// The drawing's one line layer a network can be made of (docs/adr/9332).
    offer: Option<(String, String)>,
}

#[derive(Default)]
pub struct NetArea {
    base: Base,
    asking: Option<Asking>,
    shown: Option<Shown>,
    refusal: Option<String>,
    /// What cuts the area off, as last said (docs/adr/0640 §2): said again only when it changes.
    cut_said: Option<String>,
    /// The two tickets of the newest question: lines, then areas.
    waiting: [Option<u64>; 2],
    seen: Seen,
}

/// A polygon's rings as the preview draws them (its parts and holes).
fn rings_of(shape: &Shape) -> Vec<Vec<Vec2>> {
    let ring = |pts: &[Vec2], bulges: &Option<Vec<f64>>| {
        bulge_path_outline(pts, bulges.as_deref(), true, TAU / 96.0)
    };
    let mut out = Vec::new();
    if let Shape::Polygon {
        pts,
        bulges,
        holes,
        parts,
    } = shape
    {
        out.push(ring(pts, bulges));
        for h in holes.iter().flatten() {
            out.push(ring(&h.pts, &h.bulges));
        }
        for p in parts.iter().flatten() {
            out.push(ring(&p.pts, &p.bulges));
            for h in p.holes.iter().flatten() {
                out.push(ring(&h.pts, &h.bulges));
            }
        }
    }
    out
}

impl NetArea {
    pub fn new() -> Self {
        Self::default()
    }

    fn look(&mut self, cx: &Context<'_>) {
        let n = Base::network(cx, NetworkKind::Road).cloned();
        let m = &cx.memory;
        let cost = n.as_ref().map_or(0, |n| Base::cost_index(n, cx));
        self.seen = Seen {
            breaks: n
                .as_ref()
                .map(|n| breaks_of(n, cost, &m.network_breaks))
                .unwrap_or_default(),
            list: Base::list(cx).to_vec(),
            cost,
            network: n,
            format: Some(cx.format()),
            toward: m.network_toward,
            rings: m.network_rings,
            separate: m.network_separate,
            trim: m.network_trim,
            lines: m.network_lines,
            areas: m.network_areas,
            time: m.network_time,
            profile: m.network_profile.as_str().to_owned(),
            day: m.network_day,
            offer: Base::offer(cx),
        };
    }

    fn changed(&mut self, cx: &mut Context<'_>) {
        self.look(cx);
        self.shown = None;
        self.refusal = None;
        self.waiting = [None, None];
        self.base.problem = None;
        let Some(id) = self.seen.network.as_ref().map(|n| n.id.clone()) else {
            return;
        };
        let facilities = self.base.stops();
        if facilities.is_empty() {
            return;
        }
        let s = &self.seen;
        let q = |areas| NetworkQuestion::Area {
            facilities: facilities.clone(),
            breaks: s.breaks.clone(),
            reach: Base::reach(cx),
            cost: s.cost,
            toward: s.toward,
            separate: s.separate,
            barriers: self.base.barriers(),
            trim: s.trim,
            rings: s.rings,
            areas,
        };
        let (lines, full) = (q(false), q(true));
        // The lines first (they come at once), then the areas.
        self.waiting = [
            Some(self.base.ask(&id, lines, cx)),
            Some(self.base.ask(&id, full, cx)),
        ];
    }

    fn prepare(&mut self, cx: &mut Context<'_>) {
        if let Some(id) = self.seen.network.as_ref().map(|n| n.id.clone()) {
            self.base.ask(&id, NetworkQuestion::Prepare, cx);
        }
    }

    fn after(&mut self, did: Changed, cx: &mut Context<'_>) {
        match did {
            Changed::Network => {
                self.changed(cx);
                self.prepare(cx);
            }
            Changed::Ask => self.changed(cx),
            Changed::Show => self.look(cx),
        }
    }

    /// The breaks as written, in the cost's unit.
    fn breaks_text(&self) -> String {
        let (Some(n), Some(f)) = (&self.seen.network, &self.seen.format) else {
            return String::new();
        };
        let c = self.seen.cost;
        let unit = match cost_kind(n, c) {
            0 => f.length_unit_label().to_owned(),
            1 => "dk".to_owned(),
            _ => n.costs[c - 1].unit.clone(),
        };
        let shown: Vec<String> = self
            .seen
            .breaks
            .iter()
            .map(|&b| plain(if c == 0 { f.from_metres(b) } else { b }))
            .collect();
        let mut out = shown.join(" ");
        if !unit.is_empty() {
            out.push(' ');
            out.push_str(&unit);
        }
        out
    }

    /// The unit of the breaks as a result's columns name it (the second round's T33): metres for Uzunluk (the breaks are
    /// kept in metres), minutes for a speed (as typed), a field's own; none for a field without one.
    fn breaks_unit(&self) -> Option<String> {
        self.seen.network.as_ref()?.breaks_unit(self.seen.cost).map(str::to_owned)
    }

    /// A column's name with the breaks' unit: “Bitiş (m)”.
    fn with_unit(&self, name: &str) -> String {
        self.breaks_unit().map_or_else(|| name.to_owned(), |u| format!("{name} ({u})"))
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> Option<Changed> {
        let m = &mut cx.memory;
        match key {
            "R" => self.asking = Some(Asking::Breaks),
            "K" => self.asking = Some(Asking::Trim),
            "Y" => {
                m.network_toward = !m.network_toward;
                return Some(Changed::Ask);
            }
            "B" => {
                m.network_rings = !m.network_rings;
                return Some(Changed::Ask);
            }
            "İ" => {
                m.network_separate = !m.network_separate;
                return Some(Changed::Ask);
            }
            "Ç" => {
                m.network_lines = !m.network_lines;
                // One output at least: the areas come back with the lines off (T35).
                if !m.network_lines && !m.network_areas {
                    m.network_areas = true;
                }
            }
            _ => return self.base.option(key, NetworkKind::Road, true, cx),
        }
        Some(Changed::Show)
    }

    /// A value typed for what is asked: a wrong one is said and asked again.
    fn answer(&mut self, text: &str, cx: &mut Context<'_>) {
        let Some(what) = self.asking else {
            return;
        };
        let t = js_trim(text);
        let refused = |cx: &mut Context<'_>, why: String| {
            cx.say(Level::Warn, format!("{why}; “{t}” yazıldı."))
        };
        let f = cx.format();
        match what {
            Asking::Breaks => {
                let values: Vec<Option<f64>> = t
                    .split(|c: char| c.is_whitespace() || c == ';')
                    .filter(|v| !v.is_empty())
                    .map(parse_number)
                    .collect();
                let ok = !values.is_empty()
                    && values.len() <= MAX_BREAKS
                    && values.iter().enumerate().all(|(i, v)| matches!(v, Some(v) if *v > 0.0 && v.is_finite() && (i == 0 || values[i - 1].is_some_and(|p| *v > p))));
                if !ok {
                    refused(
                        cx,
                        format!(
                            "Aralıklar artan, sıfırdan büyük sayılar olmalı (en çok {MAX_BREAKS})"
                        ),
                    );
                    return;
                }
                let Some(n) = Base::network(cx, NetworkKind::Road).cloned() else {
                    return;
                };
                let c = Base::cost_index(&n, cx);
                let values: Vec<f64> = values
                    .into_iter()
                    .flatten()
                    .map(|v| if c == 0 { f.to_metres(v) } else { v })
                    .collect();
                if let Some(kept) = Values::from_slice(&values) {
                    cx.memory.network_breaks[cost_kind(&n, c)] = kept;
                }
            }
            Asking::Trim => match parse_number(t) {
                Some(v) if v > 0.0 && v.is_finite() && f.to_metres(v) <= 10_000.0 => {
                    cx.memory.network_trim = f.to_metres(v)
                }
                _ => {
                    refused(
                        cx,
                        "Kenar payı sıfırdan büyük bir uzunluk olmalı (en çok 10 km)".to_owned(),
                    );
                    return;
                }
            },
        }
        self.asking = None;
        self.changed(cx);
    }

    /// Enter: the areas (and the lines) written; the facilities go.
    fn write(&mut self, cx: &mut Context<'_>) {
        if !self.seen.areas && !self.seen.lines {
            cx.say(
                Level::Warn,
                format!("{LABEL}: yazılacak çıktı seçilmedi; Yaklaşık alan ya da Ulaşılan yollar anahtarını açın."),
            );
            return;
        }
        let Some(shown) = self.shown.as_ref().filter(|s| s.full) else {
            let why = self.refusal.clone().unwrap_or_else(|| {
                "alanlar henüz hesaplanmadı; biraz bekleyip yeniden deneyin.".to_owned()
            });
            cx.say(Level::Warn, format!("{LABEL}: {why}"));
            return;
        };
        let Some(n) = self.seen.network.clone() else {
            return;
        };
        let c = self.seen.cost;
        let cost = n.cost_names()[c].to_owned();
        let breaks = self.seen.breaks.clone();
        let decimals = cx.format().length_decimals;
        let number = |v: f64| fixed(v, if c == 0 { decimals } else { 2 });
        let all = (1..=self.base.stops().len())
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let facility = |f: Option<usize>| f.map_or_else(|| all.clone(), |i| (i + 1).to_string());
        let rings = self.seen.rings;
        let (start_column, end_column, band_column) =
            (self.with_unit("Başlangıç"), self.with_unit("Bitiş"), self.with_unit("Aralık"));
        let attrs = |pairs: Vec<(&str, String)>| {
            pairs
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect::<BTreeMap<String, String>>()
        };
        let areas: Vec<_> = shown
            .served
            .areas
            .iter()
            .filter_map(|a| {
                let g = edit_geometry(a.shape.clone()?)?;
                let start = if rings && a.band > 0 {
                    breaks[a.band - 1]
                } else {
                    0.0
                };
                Some((
                    g,
                    attrs(vec![
                        ("Ağ", n.name.clone()),
                        ("Maliyet", cost.clone()),
                        ("Tesis", facility(a.facility)),
                        (start_column.as_str(), number(start)),
                        (end_column.as_str(), number(breaks[a.band])),
                    ]),
                ))
            })
            .collect();
        let count = areas.len();
        // The run as İşlemler keeps it (docs/adr/9334), taken before the write: the facilities go after it.
        let started = record::now_ms();
        let mut values = record::common_values(&n, Some(c), &self.base, cx);
        values.insert("breaks".to_owned(), json!(breaks));
        values.insert("trim".to_owned(), json!(self.seen.trim));
        values.insert("rings".to_owned(), json!(rings));
        values.insert("toward".to_owned(), json!(self.seen.toward));
        values.insert("separate".to_owned(), json!(self.seen.separate));
        values.insert("lines".to_owned(), json!(self.seen.lines));
        values.insert("areas".to_owned(), json!(self.seen.areas));
        let tool = format!("tool.{ID}");
        let derived = record::derivation(&tool, LABEL, &values, &n, started, cx);
        let f = cx.format();
        let mut words = record::common_words(&n, Some(c), "Tesisler", &self.base, &f);
        words.extend([
            format!("Aralıklar: {}", self.breaks_text()),
            format!("Kenar payı: {}", f.length(self.seen.trim)),
            format!("Biçim: {}", if rings { "halka" } else { "disk" }),
            format!("Yön: {}", if self.seen.toward { "tesise" } else { "tesisten" }),
        ]);
        let records_in = record::records_in(&n, cx);
        let mut writes = Vec::new();
        if self.seen.areas {
            writes.push(ResultWrite {
                layer: &AREA_LAYER,
                objects: areas,
            });
        }
        let lines = shown.served.lines.len();
        if self.seen.lines {
            writes.push(ResultWrite {
                layer: &AREA_LINES_LAYER,
                objects: shown
                    .served
                    .lines
                    .iter()
                    .map(|l| {
                        let lo = if l.band > 0 { breaks[l.band - 1] } else { 0.0 };
                        (
                            line_geometry(&l.line),
                            attrs(vec![
                                ("Ağ", n.name.clone()),
                                ("Maliyet", cost.clone()),
                                ("Tesis", facility(l.facility)),
                                (band_column.as_str(), format!("{}–{}", plain(lo), plain(breaks[l.band]))),
                            ]),
                        )
                    })
                    .collect(),
            });
        }
        let Some(written) = write_results(LABEL, &writes, Some(&derived), cx) else {
            return;
        };
        if self.seen.areas {
            words.push(record::layer_words("Çıktı katmanı", AREA_LAYER.name, &written.opened));
        }
        if self.seen.lines {
            words.push(record::layer_words("Ulaşılan yollar", AREA_LINES_LAYER.name, &written.opened));
        }
        // What the areas are and where the reach itself is (docs/adr/9333).
        let trim = cx.format().length(self.seen.trim);
        let said = if !self.seen.areas {
            format!("{LABEL}: {lines} ulaşılan yol parçası yazıldı.")
        } else if self.seen.lines {
            format!("{LABEL}: {count} yaklaşık alan ve {lines} ulaşılan yol parçası yazıldı; alan, yolların {trim} çevresidir.")
        } else {
            format!(
                "{LABEL}: {count} yaklaşık alan yazıldı; alan, ulaşılan yolların {trim} çevresidir. Ulaşılan yollar \
                 için Çizgiler (Ç) seçeneğini açın."
            )
        };
        cx.say(Level::Success, said.clone());
        cx.view_changes.push(ViewChange::Ran(Box::new(AnalysisRun {
            tool,
            label: LABEL.to_owned(),
            values,
            words,
            summary: said,
            added: written.slots,
            touched: Vec::new(),
            records_in,
            started,
        })));
        self.base.points.clear();
        self.changed(cx);
    }
}

impl Tool for NetArea {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let Some(n) = &self.seen.network else {
            return Base::no_network_prompt(LABEL, self.seen.offer.as_ref());
        };
        let f = self.seen.format.unwrap_or_default();
        match self.asking {
            Some(Asking::Breaks) => {
                return Prompt::new(LABEL, format!("aralıkları artan sayılarla yazın, aralarında boşluk (ör. 5 10 15; en çok {MAX_BREAKS})"))
                    .note(format!("şimdi: {}", self.breaks_text()));
            }
            Some(Asking::Trim) => {
                return Prompt::new(
                    LABEL,
                    format!(
                        "alanın çizgilerden payını yazın (şimdi: {})",
                        f.length(self.seen.trim)
                    ),
                );
            }
            None => {}
        }
        let on = |b: bool| if b { "açık" } else { "kapalı" };
        let count = self.base.stops().len();
        let step = if let Some(p) = &self.base.problem {
            p.clone()
        } else if count == 0 {
            "tesise tıklayın ya da Y,X yazın".to_owned()
        } else if let Some(why) = &self.refusal {
            why.clone()
        } else if let Some(s) = &self.shown {
            let reached = s.served.areas.iter().filter(|a| a.shape.is_some()).count();
            let areas = if s.full {
                format!(", {reached} alan")
            } else {
                ", alanlar hesaplanıyor".to_owned()
            };
            format!(
                "{count} tesis; {} çizgi{areas}; başka tesise tıklayın ya da Enter ile yazın",
                s.served.lines.len()
            )
        } else {
            format!("{count} tesis; hesaplanıyor")
        };
        let cost = n.cost_names()[self.seen.cost].to_owned();
        // The breaks and the margin first (docs/adr/9333): a narrow line folds its last options into Diğer, and the
        // network and its cost stand in the tool's settings too. Engel before the shape's options, so that its
        // “sonraki tıklama” stays in sight.
        let prompt = Prompt::new(LABEL, step)
            .option_with("Aralıklar", "R", self.breaks_text())
            .option_with("Kenar payı", "K", f.length(self.seen.trim))
            .option_with("Ağ", "A", n.name.clone())
            .option_with("Maliyet", "M", cost)
            .option_with("Çizgiler", "Ç", on(self.seen.lines));
        let prompt = if self.base.barrier_next {
            prompt.option_with("Engel", "E", "sonraki tıklama")
        } else {
            prompt.option("Engel", "E")
        };
        let prompt = prompt
            .option_with("Biçim", "B", if self.seen.rings { "halka" } else { "disk" })
            .option_with(
                "Yön",
                "Y",
                if self.seen.toward {
                    "tesise"
                } else {
                    "tesisten"
                },
            )
            .option_with("Birleşik", "İ", on(!self.seen.separate));
        let prompt = Base::access_options(
            Base::time_option(prompt, n, self.seen.time),
            n,
            &self.seen.profile,
            self.seen.day,
        );
        if self.shown.is_some() {
            prompt.option("Uygula", "Enter")
        } else {
            prompt
        }
    }

    fn point_count(&self) -> usize {
        self.base.points.len()
    }

    /// The network as it is, from the start (docs/adr/9332): its name, cost and unit, how its lines join; then the
    /// breaks, the margin, whether the lines are written, and what the areas are (docs/adr/9333).
    fn settings(&self, format: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::Setting;
        let mut rows = Base::settings(self.seen.network.as_ref(), self.seen.cost, self.seen.offer.as_ref(), format);
        if self.seen.network.is_none() {
            return rows;
        }
        // Editable here as on the command line (the second round's T35): the breaks in their unit, the margin, and
        // which outputs are written; the outputs' layers and columns said before the run.
        let unit = self.breaks_unit();
        let breaks: Vec<String> = self
            .seen
            .breaks
            .iter()
            .map(|&b| plain(if self.seen.cost == 0 { format.from_metres(b) } else { b }))
            .collect();
        let breaks_label = match self.seen.cost {
            0 => format!("Aralıklar ({})", format.length_unit_label()),
            _ => unit.as_ref().map_or_else(|| "Aralıklar".to_owned(), |u| format!("Aralıklar ({u})")),
        };
        rows.push(Setting::text("serviceBreaks", breaks_label, breaks.join(" ")));
        rows.push(Setting::text("serviceTrim", "Kenar payı", format.length(self.seen.trim)));
        rows.push(Setting::toggle("serviceAreas", "Yaklaşık alan", self.seen.areas));
        rows.push(Setting::toggle("serviceLines", "Ulaşılan yollar", self.seen.lines));
        let mut out = Vec::new();
        if self.seen.areas {
            out.push(format!(
                "“{}” ({}, {}): ulaşılan yolların {} çevresi, yaklaşık",
                AREA_LAYER.name,
                self.with_unit("Başlangıç"),
                self.with_unit("Bitiş"),
                format.length(self.seen.trim)
            ));
        }
        if self.seen.lines {
            out.push(format!("“{}” ({}): ağın ulaşılan parçaları", AREA_LINES_LAYER.name, self.with_unit("Aralık")));
        }
        let text = if out.is_empty() {
            "Çıktı seçilmedi: Yaklaşık alan ya da Ulaşılan yollar anahtarını açın.".to_owned()
        } else {
            format!("Çıktılar: {}.", out.join("; "))
        };
        rows.push(if out.is_empty() {
            Setting::notice("serviceAreaNote", "Çıktı", text)
        } else {
            Setting::hint("serviceAreaNote", "Çıktı", text)
        });
        rows
    }

    /// The pane's settings written (T35): the breaks and the margin as the command line takes them (a unit after them
    /// is the one shown), the outputs' switches as Ç is; the last output is not switched off.
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "serviceBreaks" | "serviceTrim" => {
                // The unit the pane shows after the numbers: dropped, the numbers are in it.
                let words: Vec<&str> = value.split_whitespace().collect();
                let keep = words.iter().rposition(|w| w.chars().any(|c| c.is_ascii_digit())).map_or(0, |i| i + 1);
                let numbers = words[..keep].join(" ");
                self.asking = Some(if key == "serviceBreaks" { Asking::Breaks } else { Asking::Trim });
                self.answer(&numbers, cx);
                // A refused value leaves it asked on the line: the pane's value goes back as it was.
                if self.asking.is_some() {
                    self.asking = None;
                    return false;
                }
                true
            }
            "serviceAreas" | "serviceLines" => {
                let on = match value {
                    "yes" => true,
                    "no" => false,
                    _ => return false,
                };
                let m = &mut cx.memory;
                let other = if key == "serviceAreas" { m.network_lines } else { m.network_areas };
                if !on && !other {
                    cx.say(
                        Level::Warn,
                        format!("{LABEL}: en az bir çıktı yazılır; önce öbür çıktıyı açın."),
                    );
                    return false;
                }
                if key == "serviceAreas" {
                    m.network_areas = on;
                } else {
                    m.network_lines = on;
                }
                self.look(cx);
                true
            }
            _ => false,
        }
    }

    /// A network defined, changed or removed while the tool runs (docs/adr/9332): seen again; another network, the
    /// run starts over at its first facility.
    fn document_changed(&mut self, cx: &Context<'_>) {
        let before = self.seen.network.clone();
        self.look(cx);
        if self.seen.network != before {
            self.base.reset();
            self.shown = None;
            self.refusal = None;
            self.waiting = [None, None];
        }
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.look(cx);
        self.prepare(cx);
        Flow::Stay
    }

    /// Yeniden aç (docs/adr/9334): the network, cost, breaks, margin, shape, way and the lines' switch put back, the
    /// facilities and barriers placed again; the areas come as they did.
    fn reopen(&mut self, values: &BTreeMap<String, Value>, cx: &mut Context<'_>) -> bool {
        let points = record::reopen_common(values, cx);
        let flag = |key: &str| values.get(key).and_then(Value::as_bool);
        let m = &mut cx.memory;
        for (key, slot) in [
            ("toward", &mut m.network_toward),
            ("rings", &mut m.network_rings),
            ("separate", &mut m.network_separate),
            ("lines", &mut m.network_lines),
            ("areas", &mut m.network_areas),
        ] {
            if let Some(v) = flag(key) {
                *slot = v;
            }
        }
        if let Some(t) = values
            .get("trim")
            .and_then(Value::as_f64)
            .filter(|t| *t > 0.0 && t.is_finite() && *t <= 10_000.0)
        {
            m.network_trim = t;
        }
        self.base.reset();
        self.asking = None;
        self.shown = None;
        self.refusal = None;
        self.waiting = [None, None];
        self.look(cx);
        // The breaks, for the cost the network has now: rising numbers above nought, as typed ones are.
        if let (Some(n), Some(list)) = (self.seen.network.clone(), values.get("breaks").and_then(Value::as_array)) {
            let breaks: Vec<f64> = list.iter().filter_map(Value::as_f64).collect();
            let rising = breaks.len() == list.len()
                && !breaks.is_empty()
                && breaks.len() <= MAX_BREAKS
                && breaks.iter().enumerate().all(|(i, b)| *b > 0.0 && b.is_finite() && (i == 0 || *b > breaks[i - 1]));
            if rising && let Some(kept) = Values::from_slice(&breaks) {
                cx.memory.network_breaks[cost_kind(&n, self.seen.cost)] = kept;
            }
            self.look(cx);
        }
        self.prepare(cx);
        let id = self.seen.network.as_ref().map(|n| n.id.clone());
        for (kind, at) in points {
            self.base.barrier_next = kind == PointKind::Barrier;
            self.base.place(at, id.as_deref(), LABEL, cx);
        }
        true
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn takes_words(&self) -> bool {
        self.asking == Some(Asking::Breaks)
    }

    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking.is_some() {
            return;
        }
        let id = self.seen.network.as_ref().map(|n| n.id.clone());
        self.base.place(p.world, id.as_deref(), LABEL, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        // While a value is asked, everything typed is that value (Aralıklar has spaces).
        if self.asking.is_some() {
            self.answer(text, cx);
            return true;
        }
        let t = js_trim(text);
        let last = self.base.points.last().map(|p| p.at);
        if let Some(at) = cx.typed_point(t, last, None) {
            let id = self.seen.network.as_ref().map(|n| n.id.clone());
            self.base.place(at, id.as_deref(), LABEL, cx);
            return true;
        }
        let Some(did) = self.option(&upper_tr(t), cx) else {
            return false;
        };
        self.after(did, cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        Base::option_choices(
            key,
            &self.seen.list,
            self.seen.network.as_ref(),
            self.seen.cost,
            true,
        )
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        match self.base.choose(key, typed, NetworkKind::Road, true, cx) {
            Some(did) => {
                self.after(did, cx);
                true
            }
            None => false,
        }
    }

    /// Enter: a value being asked ends; with areas they are written and the facilities go; with none the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            self.look(cx);
            return Flow::Stay;
        }
        if self.base.points.is_empty() {
            return Flow::Exit;
        }
        self.write(cx);
        Flow::Stay
    }

    /// Esc: a value being asked goes first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            self.look(cx);
            return true;
        }
        match self.base.cancel() {
            Some(did) => {
                self.after(did, cx);
                true
            }
            None => false,
        }
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    fn network_answered(&mut self, reply: NetworkReply, cx: &mut Context<'_>) {
        match reply.answer {
            Answer::Place(place) => {
                if self.base.found(reply.ticket, place, LABEL, cx) && place.is_some() {
                    self.changed(cx);
                }
            }
            Answer::Area(r) if self.waiting.contains(&Some(reply.ticket)) => {
                let full = self.waiting[1] == Some(reply.ticket);
                if full {
                    self.waiting = [None, None];
                } else {
                    self.waiting[0] = None;
                }
                match r {
                    Ok(served) => {
                        let words = served.cut.words();
                        if words != self.cut_said {
                            if let Some(w) = &words {
                                cx.say(Level::Info, format!("{LABEL}: {w}."));
                            }
                            self.cut_said = words;
                        }
                        // The lines' answer does not replace the areas' when those came first.
                        if full || !self.shown.as_ref().is_some_and(|s| s.full) {
                            let lines = served.lines.iter().map(|l| outline(&l.line)).collect();
                            let areas = served
                                .areas
                                .iter()
                                .filter_map(|a| Some((a.band, rings_of(a.shape.as_ref()?))))
                                .collect();
                            self.shown = Some(Shown {
                                served,
                                lines,
                                areas,
                                full,
                            });
                        }
                    }
                    Err(why) => {
                        let words = refusal_words(why, &format!("{REACH_PX} piksel"));
                        if self.refusal.is_none() {
                            cx.say(Level::Warn, format!("{LABEL}: {words}"));
                        }
                        self.refusal = Some(words);
                        self.shown = None;
                    }
                }
            }
            Answer::Failed(why) => {
                if self.base.problem.as_deref() != Some(why.as_str()) {
                    cx.say(Level::Warn, why.clone());
                }
                self.base.problem = Some(why);
                self.waiting = [None, None];
            }
            _ => {}
        }
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        if let Some(s) = &self.shown {
            let bands = self.seen.breaks.len().max(2) - 1;
            for (band, rings) in &s.areas {
                out.areas.push(Area {
                    rings: rings.clone(),
                    fill: 0.42 - 0.3 * (*band as f32) / bands as f32,
                    width: 1.0,
                    dash: None,
                    fill_tone: Tone::Accent,
                });
            }
            for l in &s.lines {
                out.strokes.push(stroke(l.clone(), 1.5, None, Tone::Accent));
            }
        }
        self.base.draw_points(&mut out, true);
        out
    }
}
