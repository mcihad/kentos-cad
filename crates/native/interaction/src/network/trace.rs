//! Şebeke izleme (docs/adr/0209 §8; the web's `tools/networkTraceTool.ts`):
//! starting points clicked (a broken pipe, a feeder); Tür (T) traces what is
//! connected (Bağlı), what the edges' directions lead to or from (Akış aşağı,
//! Akış yukarı), or what closing the nearest open valves isolates (Yalıtım:
//! the valves to close, and what no source feeds once they are). The trace
//! shows at once; Enter selects its objects (and the valves) and says their
//! length, counts and the valves' names.

use kentos_contracts::{NetworkDef, NetworkKind};
use kentos_domain::Slot;
use kentos_geometry_core::ops::network::{TraceAnswer, TraceKind};
use kentos_geometry_core::tools::point_text::js_trim;

use super::base::{Base, Changed, REACH_PX, no_network_words, outline, refusal_words, stroke};
use super::{Answer, NetworkQuestion, NetworkReply};
use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, OptionChoice, Pointer, Preview, Tone, Tool,
};

/// The tool's id: its command is `tool.netTrace`.
pub const ID: &str = "netTrace";
pub const LABEL: &str = "Şebeke izleme";

/// Tür's values in its key's order.
pub const TRACE_KINDS: [(TraceKind, &str); 4] = [
    (TraceKind::Connected, "bağlı"),
    (TraceKind::Downstream, "akış aşağı"),
    (TraceKind::Upstream, "akış yukarı"),
    (TraceKind::Isolation, "yalıtım"),
];

pub fn kind_name(k: TraceKind) -> &'static str {
    TRACE_KINDS
        .iter()
        .find(|(v, _)| *v == k)
        .map_or("bağlı", |(_, n)| n)
}

/// What the prompt and the preview show (the host's world as last seen).
#[derive(Clone, Debug, Default)]
struct Seen {
    network: Option<NetworkDef>,
    list: Vec<NetworkDef>,
    kind: Option<TraceKind>,
    format: Option<Format>,
}

/// A trace's answer with its lines and the lines no longer fed, as drawn.
type Traced = (TraceAnswer, Vec<Vec<Vec2>>, Vec<Vec<Vec2>>);

#[derive(Default)]
pub struct NetTrace {
    base: Base,
    traced: Option<Traced>,
    refusal: Option<String>,
    waiting: Option<u64>,
    seen: Seen,
}

impl NetTrace {
    pub fn new() -> Self {
        Self::default()
    }

    fn kind(&self) -> TraceKind {
        self.seen.kind.unwrap_or(TraceKind::Connected)
    }

    fn look(&mut self, cx: &Context<'_>) {
        self.seen = Seen {
            network: Base::network(cx, NetworkKind::Utility).cloned(),
            list: Base::list(cx).to_vec(),
            kind: Some(cx.memory.network_trace),
            format: Some(cx.format()),
        };
    }

    fn changed(&mut self, cx: &mut Context<'_>) {
        self.look(cx);
        self.traced = None;
        self.refusal = None;
        self.waiting = None;
        self.base.problem = None;
        let Some(id) = self.seen.network.as_ref().map(|n| n.id.clone()) else {
            return;
        };
        let starts = self.base.stops();
        if starts.is_empty() {
            return;
        }
        let q = NetworkQuestion::Trace {
            starts,
            barriers: self.base.barriers(),
            reach: Base::reach(cx),
            kind: self.kind(),
        };
        self.waiting = Some(self.base.ask(&id, q, cx));
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

    /// The trace in words: length, objects, valves, and what is no longer fed.
    fn summary(&self, t: &TraceAnswer) -> String {
        let f = self.seen.format.unwrap_or_default();
        let mut words = vec![format!("{}, {} nesne", f.length(t.length), t.objects.len())];
        if self.kind() == TraceKind::Isolation {
            words.push(format!("{} vana kapatılır", t.valves.len()));
            if !t.unfed.is_empty() {
                words.push(format!(
                    "beslemesiz kalan {}, {} nesne",
                    f.length(t.unfed_length),
                    t.unfed.len()
                ));
            }
        }
        words.join("; ")
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> Option<Changed> {
        if key == "T" {
            let at = TRACE_KINDS
                .iter()
                .position(|(k, _)| *k == cx.memory.network_trace)
                .unwrap_or(0);
            cx.memory.network_trace = TRACE_KINDS[(at + 1) % TRACE_KINDS.len()].0;
            return Some(Changed::Ask);
        }
        self.base.option(key, NetworkKind::Utility, false, cx)
    }

    /// A valve's name: its label, else its `Ad`, else its number.
    fn valve_name(id: f64, cx: &Context<'_>) -> String {
        let e = super::base::slot_of(id).and_then(|s| cx.doc.get(s));
        e.and_then(|e| {
            e.base()
                .label
                .as_deref()
                .map(js_trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
        })
        .or_else(|| {
            e.and_then(|e| {
                e.base()
                    .attrs
                    .get("Ad")
                    .map(|a| js_trim(a).to_owned())
                    .filter(|a| !a.is_empty())
            })
        })
        .unwrap_or_else(|| format!("#{}", crate::format::js_number(id)))
    }
}

impl Tool for NetTrace {
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
        let count = self.base.stops().len();
        let step = if let Some(p) = &self.base.problem {
            p.clone()
        } else if count == 0 {
            "izlemenin başlayacağı yere tıklayın ya da Y,X yazın".to_owned()
        } else if let Some(why) = &self.refusal {
            why.clone()
        } else if let Some((t, _, _)) = &self.traced {
            format!(
                "{}; başka başlangıca tıklayın ya da Enter ile seçin",
                self.summary(t)
            )
        } else {
            format!("{count} başlangıç; izleniyor")
        };
        let prompt = Prompt::new(LABEL, step).option_with("Ağ", "A", n.name.clone());
        let prompt = if self.base.barrier_next {
            prompt.option_with("Engel", "E", "sonraki tıklama")
        } else {
            prompt.option("Engel", "E")
        };
        let prompt = prompt.option_with("Tür", "T", kind_name(self.kind()));
        if self.traced.is_some() {
            prompt.option("Seç", "Enter")
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

    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let id = self.seen.network.as_ref().map(|n| n.id.clone());
        self.base.place(p.world, id.as_deref(), LABEL, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
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
        if key == "T" {
            return TRACE_KINDS
                .iter()
                .map(|&(k, name)| OptionChoice {
                    label: name.to_owned(),
                    typed: name.to_owned(),
                    icon: None,
                    preview: None,
                    checked: k == self.kind(),
                    command: None,
                })
                .collect();
        }
        Base::option_choices(key, &self.seen.list, self.seen.network.as_ref(), 0, false)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key == "T" {
            let Some(&(k, _)) = TRACE_KINDS.iter().find(|(_, n)| *n == typed.trim()) else {
                return false;
            };
            cx.memory.network_trace = k;
            self.after(Changed::Ask, cx);
            return true;
        }
        match self
            .base
            .choose(key, typed, NetworkKind::Utility, false, cx)
        {
            Some(did) => {
                self.after(did, cx);
                true
            }
            None => false,
        }
    }

    /// Enter: the trace's objects (and Yalıtım's valves) are selected and said; the tool leaves. With no start it leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let Some((t, _, _)) = &self.traced else {
            if self.base.points.is_empty() {
                return Flow::Exit;
            }
            let why = self
                .refusal
                .clone()
                .unwrap_or_else(|| "izlenen bir şey yok.".to_owned());
            cx.say(Level::Warn, format!("{LABEL}: {why}"));
            return Flow::Stay;
        };
        let isolation = self.kind() == TraceKind::Isolation;
        let ids: Vec<Slot> = t
            .objects
            .iter()
            .copied()
            .chain(t.valves.iter().filter(|_| isolation).map(|v| v.id))
            .filter_map(super::base::slot_of)
            .filter(|s| cx.doc.get(*s).is_some())
            .collect();
        cx.selection.set(ids);
        let mut said = format!("{LABEL} ({}): {}", kind_name(self.kind()), self.summary(t));
        if isolation && !t.valves.is_empty() {
            let names: Vec<String> = t
                .valves
                .iter()
                .map(|v| Self::valve_name(v.id, cx))
                .collect();
            said.push_str(&format!("; kapatılacak vanalar: {}", names.join(", ")));
        }
        cx.say(Level::Success, format!("{said}."));
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
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
            Answer::Trace(r) if self.waiting == Some(reply.ticket) => {
                self.waiting = None;
                match r {
                    Ok(t) => {
                        let lines = t.lines.iter().map(outline).collect();
                        let unfed = t.unfed_lines.iter().map(outline).collect();
                        self.traced = Some((t, lines, unfed));
                    }
                    Err(why) => {
                        let words = refusal_words(why, &format!("{REACH_PX} piksel"));
                        cx.say(Level::Warn, format!("{LABEL}: {words}"));
                        self.refusal = Some(words);
                    }
                }
            }
            Answer::Failed(why) => {
                if self.base.problem.as_deref() != Some(why.as_str()) {
                    cx.say(Level::Warn, why.clone());
                }
                self.base.problem = Some(why);
                self.waiting = None;
            }
            _ => {}
        }
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        if let Some((t, lines, unfed)) = &self.traced {
            for l in lines {
                out.strokes.push(stroke(l.clone(), 3.0, None, Tone::Accent));
            }
            for l in unfed {
                out.strokes
                    .push(stroke(l.clone(), 2.0, Some([6.0, 4.0]), Tone::Danger));
            }
            for v in &t.valves {
                out.markers.push(Marker {
                    at: Vec2::new(v.x, v.y),
                    shape: MarkerShape::Square(5.0),
                    tone: Tone::Danger,
                });
            }
        }
        self.base.draw_points(&mut out, false);
        out
    }
}
