//! Hizmet alanı (docs/adr/0209 §6; the web's `tools/networkAreaTool.ts`):
//! facilities clicked; the network within each break of the chosen cost
//! shows at once (its lines, asked first) and then its areas (the lines'
//! buffers, Kenar payı wide; each band paler than the one inside it).
//! Aralıklar (R) are rising numbers in the cost's unit; Yön (Y) from or to the
//! facilities; Biçim (B) discs or rings; Birleşik (İ) one area a band for all
//! facilities or one each; Çizgiler (Ç) writes the lines too. Enter writes
//! the areas to Hizmet alanı (and the lines to Hizmet alanı çizgileri) in one
//! step.

use std::collections::BTreeMap;

use kentos_contracts::{NetworkCostKind, NetworkDef, NetworkKind};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::jsmath::TAU;
use kentos_geometry_core::ops::network::Served;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::edit_geometry;

use super::base::{Base, Changed, REACH_PX, no_network_words, outline, refusal_words, stroke};
use super::result::{AREA_LAYER, AREA_LINES_LAYER, ResultWrite, line_geometry, write_results};
use super::{Answer, NetworkQuestion, NetworkReply};
use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Area, Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tone, Tool, Values,
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
}

#[derive(Default)]
pub struct NetArea {
    base: Base,
    asking: Option<Asking>,
    shown: Option<Shown>,
    refusal: Option<String>,
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
            "Ç" => m.network_lines = !m.network_lines,
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
                        ("Başlangıç", number(start)),
                        ("Bitiş", number(breaks[a.band])),
                    ]),
                ))
            })
            .collect();
        let count = areas.len();
        let mut writes = vec![ResultWrite {
            layer: &AREA_LAYER,
            objects: areas,
        }];
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
                                ("Aralık", format!("{}–{}", plain(lo), plain(breaks[l.band]))),
                            ]),
                        )
                    })
                    .collect(),
            });
        }
        if write_results(LABEL, &writes, cx).is_none() {
            return;
        }
        let also = if self.seen.lines {
            format!(" ve {lines} çizgi")
        } else {
            String::new()
        };
        cx.say(
            Level::Success,
            format!("{LABEL}: {count} alan{also} yazıldı."),
        );
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
            return Prompt::new(LABEL, no_network_words()).option("Ağlar", "G");
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
        let prompt = Prompt::new(LABEL, step)
            .option_with("Ağ", "A", n.name.clone())
            .option_with("Maliyet", "M", cost);
        let prompt = if self.base.barrier_next {
            prompt.option_with("Engel", "E", "sonraki tıklama")
        } else {
            prompt.option("Engel", "E")
        };
        let prompt = prompt
            .option_with("Aralıklar", "R", self.breaks_text())
            .option_with(
                "Yön",
                "Y",
                if self.seen.toward {
                    "tesise"
                } else {
                    "tesisten"
                },
            )
            .option_with("Biçim", "B", if self.seen.rings { "halka" } else { "disk" })
            .option_with("Birleşik", "İ", on(!self.seen.separate))
            .option_with("Kenar payı", "K", f.length(self.seen.trim))
            .option_with("Çizgiler", "Ç", on(self.seen.lines));
        if self.shown.is_some() {
            prompt.option("Uygula", "Enter")
        } else {
            prompt
        }
    }

    fn point_count(&self) -> usize {
        self.base.points.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.look(cx);
        self.prepare(cx);
        Flow::Stay
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
