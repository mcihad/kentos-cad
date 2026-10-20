//! Layer actions by an object (docs/adr/0177 §1; the web's
//! `tools/layerTools.ts`): a click on an object acts on its layer.
//!
//! - Katmanı gizle and Katmanı kilitle go on, a layer a click; Katmanı etkin
//!   yap leaves after its click; Katmanı yalıt gathers the clicked objects'
//!   layers and isolates them on Enter or a quick right click (Ctrl+Z takes
//!   back the newest, Esc all of them); the drawing remembers what it hid
//!   for Yalıtımı kaldır (`cad.layers.set`'s `unisolate`, docs/adr/0311 §2).
//! - With objects selected when it starts, it acts on their layers at once
//!   (Katmanı etkin yap on the first one's) and leaves.
//! - These are the layer tree's own changes, as the Katmanlar panel's eye,
//!   lock and active layer, written through `cad.layers.set` (docs/adr/0248
//!   §2): the drawing changes, no undo step. A hidden
//!   layer's objects leave the selection.

use kentos_commands::{CommandResult, LayerNodeType, LayerSetOperation, LayersSet};
use kentos_domain::Slot;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, layers_set};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, Tone, Tool};

/// Katmanı gizle's id: its command is `tool.layerOff`.
pub const OFF_ID: &str = "layerOff";
/// Katmanı yalıt's id: its command is `tool.layerIsolate`.
pub const ISOLATE_ID: &str = "layerIsolate";
/// Katmanı kilitle's id: its command is `tool.layerLock`.
pub const LOCK_ID: &str = "layerLock";
/// Katmanı etkin yap's id: its command is `tool.layerMakeActive`.
pub const ACTIVE_ID: &str = "layerMakeActive";
/// Katmanı dondur's id: its command is `tool.layerFreeze` (docs/adr/0311 §1).
pub const FREEZE_ID: &str = "layerFreeze";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Off,
    Isolate,
    Lock,
    Active,
    Freeze,
}

impl Action {
    fn id(self) -> &'static str {
        match self {
            Action::Off => OFF_ID,
            Action::Isolate => ISOLATE_ID,
            Action::Lock => LOCK_ID,
            Action::Active => ACTIVE_ID,
            Action::Freeze => FREEZE_ID,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Action::Off => "Katmanı gizle",
            Action::Isolate => "Katmanı yalıt",
            Action::Lock => "Katmanı kilitle",
            Action::Active => "Katmanı etkin yap",
            Action::Freeze => "Katmanı dondur",
        }
    }

    fn step(self) -> &'static str {
        match self {
            Action::Off => "katmanı gizlenecek nesneye tıklayın",
            Action::Isolate => "yalıtılacak katmanların nesnelerine tıklayın",
            Action::Lock => "katmanı kilitlenecek nesneye tıklayın",
            Action::Active => "katmanı etkin yapılacak nesneye tıklayın",
            Action::Freeze => "katmanı dondurulacak nesneye tıklayın",
        }
    }

    /// The tag's second line: what a click does.
    fn click(self) -> &'static str {
        match self {
            Action::Off => "Tıklayın: gizle",
            Action::Isolate => "Tıklayın: yalıt",
            Action::Lock => "Tıklayın: kilitle",
            Action::Active => "Tıklayın: etkin yap",
            Action::Freeze => "Tıklayın: dondur",
        }
    }
}

/// “3 katman ve 1 grup”, “1 grup”: what a count of layers and groups says.
pub fn nodes_text(layers: usize, groups: usize) -> String {
    let parts: Vec<String> = [(layers, "katman"), (groups, "grup")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, word)| format!("{n} {word}"))
        .collect();
    parts.join(" ve ")
}

/// One of the four layer actions.
#[derive(Clone, Debug)]
pub struct LayerTool {
    action: Action,
    /// Katmanı yalıt's layers so far, by id, in click order, and the objects
    /// clicked for them.
    gathered: Vec<String>,
    /// Their names, for the prompt.
    gathered_names: Vec<String>,
    picked: Vec<Slot>,
    /// The hovered object's layer name and the pointer's world point, for the tag.
    hover: Option<(String, Vec2)>,
    /// The picked objects ringed, read when they were picked.
    drawn: Preview,
    done: bool,
}

impl LayerTool {
    fn with(action: Action) -> Self {
        Self {
            action,
            gathered: Vec::new(),
            gathered_names: Vec::new(),
            picked: Vec::new(),
            hover: None,
            drawn: Preview::default(),
            done: false,
        }
    }

    pub fn off() -> Self {
        Self::with(Action::Off)
    }

    pub fn isolate() -> Self {
        Self::with(Action::Isolate)
    }

    pub fn lock() -> Self {
        Self::with(Action::Lock)
    }

    pub fn make_active() -> Self {
        Self::with(Action::Active)
    }

    pub fn freeze() -> Self {
        Self::with(Action::Freeze)
    }

    /// The action on these layers (ids, in order, each once).
    fn act(&self, ids: &[String], cx: &mut Context<'_>) {
        match self.action {
            Action::Off => hide(ids, cx),
            Action::Isolate => isolate(ids, cx),
            Action::Lock => lock(ids, cx),
            Action::Freeze => freeze(ids, cx),
            Action::Active => {
                if let Some(id) = ids.first() {
                    make_active(id, cx);
                }
            }
        }
    }

    /// The ring around the objects clicked for Katmanı yalıt.
    fn redraw(&mut self, cx: &Context<'_>) {
        let mut preview = Preview::default();
        for slot in &self.picked {
            if let Some(e) = cx.doc.get(*slot) {
                let outline = Outline::of(&shape(e), None, 2.5, Tone::Accent);
                preview.strokes.extend(outline.strokes);
                preview.marks.extend(outline.marks);
            }
        }
        self.drawn = preview;
    }
}

/// A layer's name, or its id when the tree lacks it.
fn name(cx: &Context<'_>, id: &str) -> String {
    cx.doc
        .layers()
        .get(id)
        .map_or_else(|| id.to_owned(), |n| n.name.clone())
}

/// “Bina” katmanı, or “2 katman (Bina, Yol)”.
fn list(cx: &Context<'_>, ids: &[String]) -> String {
    match ids {
        [one] => format!("“{}” katmanı", name(cx, one)),
        _ => format!(
            "{} katman ({})",
            ids.len(),
            ids.iter()
                .map(|id| name(cx, id))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn hide(ids: &[String], cx: &mut Context<'_>) {
    let shown: Vec<String> = ids
        .iter()
        .filter(|id| cx.doc.layers().get(id).is_some_and(|n| n.visible))
        .cloned()
        .collect();
    if shown.is_empty() {
        let text = format!("{} zaten gizli.", list(cx, ids));
        cx.say(Level::Info, text);
        return;
    }
    for id in &shown {
        set_flag(cx.doc, LayerSetOperation::Visible, id, false);
    }
    drop_hidden(cx);
    let active = if shown.iter().any(|id| id == cx.doc.layers().active()) {
        " Etkin katman gizli: yeni nesneler gizli katmana çizilir."
    } else {
        ""
    };
    let text = format!("{} gizlendi.{active}", list(cx, &shown));
    cx.say(Level::Success, text);
}

fn isolate(ids: &[String], cx: &mut Context<'_>) {
    let mut shown_before = Vec::new();
    visible_nodes(cx.doc.layers().nodes(), &mut shown_before);
    let mut input = LayersSet::new(LayerSetOperation::Isolate, None);
    input.layers = Some(ids.to_vec());
    if !run(cx.doc, input) {
        let text = format!("Yalnız {} zaten görünüyor; değişen olmadı.", list(cx, ids));
        cx.say(Level::Info, text);
        return;
    }
    let hidden: Vec<String> = shown_before
        .into_iter()
        .filter(|id| cx.doc.layers().get(id).is_some_and(|n| !n.visible))
        .collect();
    drop_hidden(cx);
    let groups = hidden
        .iter()
        .filter(|id| {
            cx.doc
                .layers()
                .get(id)
                .is_some_and(|n| n.kind == LayerNodeType::Group)
        })
        .count();
    let text = format!(
        "{} yalıtıldı; öbür {} gizlendi. Yalıtımı kaldır onları geri getirir.",
        list(cx, ids),
        nodes_text(hidden.len() - groups, groups)
    );
    cx.say(Level::Success, text);
}

fn lock(ids: &[String], cx: &mut Context<'_>) {
    let open: Vec<String> = ids
        .iter()
        .filter(|id| !cx.doc.layers().is_locked(id))
        .cloned()
        .collect();
    if open.is_empty() {
        let text = format!("{} zaten kilitli.", list(cx, ids));
        cx.say(Level::Info, text);
        return;
    }
    for id in &open {
        set_flag(cx.doc, LayerSetOperation::Locked, id, true);
    }
    let text = format!("{} kilitlendi.", list(cx, &open));
    cx.say(Level::Success, text);
}

/// Katmanı dondur (docs/adr/0311 §1): the clicked objects' layers frozen
/// through `cad.layers.set`; the active layer is refused in the command's words.
fn freeze(ids: &[String], cx: &mut Context<'_>) {
    let thawed: Vec<String> = ids
        .iter()
        .filter(|id| !cx.doc.layers().is_frozen(id))
        .cloned()
        .collect();
    if thawed.is_empty() {
        let text = format!("{} zaten dondurulmuş.", list(cx, ids));
        cx.say(Level::Info, text);
        return;
    }
    let mut frozen = Vec::new();
    for id in &thawed {
        let mut input = LayersSet::new(LayerSetOperation::Frozen, Some(id.clone()));
        input.value = Some(true);
        match layers_set::execute(&mut ExecutionContext::new(cx.doc), input) {
            CommandResult::Completed { .. } => frozen.push(id.clone()),
            CommandResult::Failed { error } => cx.say(Level::Warn, error.message),
            _ => {}
        }
    }
    if frozen.is_empty() {
        return;
    }
    drop_hidden(cx);
    let text = format!(
        "{} donduruldu: çizilmez, seçilmez, kenetlenmez. Çözmek için Katmanlar panelinde sağ tık › Çöz ya da Bütün katmanları çöz.",
        list(cx, &frozen)
    );
    cx.say(Level::Success, text);
}

fn make_active(id: &str, cx: &mut Context<'_>) {
    let layer = name(cx, id);
    if cx.doc.layers().active() == id {
        cx.say(Level::Info, format!("“{layer}” katmanı zaten etkin."));
        return;
    }
    if cx.doc.layers().is_locked(id) {
        cx.say(
            Level::Warn,
            format!(
                "“{layer}” katmanı kilitli; etkin yapılmadı. Kilidini Katmanlar panelinden açın."
            ),
        );
        return;
    }
    let input = LayersSet::new(LayerSetOperation::Active, Some(id.to_owned()));
    match layers_set::execute(&mut ExecutionContext::new(cx.doc), input) {
        CommandResult::Failed { error } => cx.say(Level::Warn, error.message),
        _ => cx.say(Level::Success, format!("“{layer}” katmanı etkin yapıldı.")),
    }
}

/// `cad.layers.set` on the drawing, as the Katmanlar panel and Python write
/// the tree (docs/adr/0248 §2); whether it changed.
fn run(doc: &mut kentos_domain::Document, input: LayersSet) -> bool {
    matches!(
        layers_set::execute(&mut ExecutionContext::new(doc), input),
        CommandResult::Completed { output, .. } if output.changed
    )
}

/// A node's eye or lock set through `cad.layers.set`.
fn set_flag(doc: &mut kentos_domain::Document, op: LayerSetOperation, id: &str, value: bool) {
    let mut input = LayersSet::new(op, Some(id.to_owned()));
    input.value = Some(value);
    run(doc, input);
}

/// The ids of the shown nodes, in tree order.
fn visible_nodes(nodes: &[kentos_commands::LayerNode], out: &mut Vec<String>) {
    for node in nodes {
        if node.visible {
            out.push(node.id.clone());
        }
        visible_nodes(&node.children, out);
    }
}

/// The objects of layers now hidden leave the selection.
fn drop_hidden(cx: &mut Context<'_>) {
    let doc = &*cx.doc;
    let kept: Vec<Slot> = cx
        .selection
        .ids()
        .iter()
        .copied()
        .filter(|slot| {
            doc.get(*slot)
                .is_some_and(|e| doc.layers().is_visible(&e.base().layer_id))
        })
        .collect();
    if kept.len() < cx.selection.len() {
        cx.selection.set(kept);
    }
}

/// Yalıtımı kaldır (`layer.unisolate`, docs/adr/0177 §1, 0311 §2): the
/// layers and groups Katmanı yalıt hid since the last time, still hidden,
/// shown again through `cad.layers.set`; those changed by hand meanwhile
/// stay as they are. What to say, with its level.
pub fn unisolate(doc: &mut kentos_domain::Document) -> (Level, String) {
    let remembered = doc.layers().isolated().to_vec();
    if remembered.is_empty() {
        return (
            Level::Info,
            "Yalıtılmış katman yok: Katmanı yalıt bu çizimde katman gizlemedi.".to_owned(),
        );
    }
    let back: Vec<String> = remembered
        .into_iter()
        .filter(|id| doc.layers().get(id).is_some_and(|n| !n.visible))
        .collect();
    run(doc, LayersSet::new(LayerSetOperation::Unisolate, None));
    if back.is_empty() {
        return (
            Level::Info,
            "Katmanı yalıt’ın gizlediği katmanlar zaten görünüyor.".to_owned(),
        );
    }
    let groups = back
        .iter()
        .filter(|id| {
            doc.layers()
                .get(id)
                .is_some_and(|n| n.kind == LayerNodeType::Group)
        })
        .count();
    (
        Level::Success,
        format!(
            "Yalıtım kaldırıldı: {} yeniden gösterildi.",
            nodes_text(back.len() - groups, groups)
        ),
    )
}

impl Tool for LayerTool {
    fn id(&self) -> &'static str {
        self.action.id()
    }

    fn label(&self) -> &'static str {
        self.action.label()
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        let prompt = Prompt::new(self.action.label(), self.action.step());
        match self.action {
            Action::Isolate => {
                let prompt = if self.gathered.is_empty() {
                    prompt
                } else {
                    prompt
                        .note(format!(
                            "{} katman: {}",
                            self.gathered.len(),
                            self.gathered_names.join(", ")
                        ))
                        .then()
                };
                prompt.option("Uygula", "Enter")
            }
            Action::Active => prompt,
            Action::Off | Action::Lock | Action::Freeze => prompt.option("Bitir", "Enter"),
        }
    }

    /// Katmanı yalıt's gathered layers are its steps: Ctrl+Z takes back the newest.
    fn point_count(&self) -> usize {
        self.gathered.len()
    }

    /// Selected first: their layers at once, and the tool leaves.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        let mut layers: Vec<String> = Vec::new();
        for slot in cx.selection.ids() {
            if let Some(e) = cx.doc.get(*slot) {
                let id = &e.base().layer_id;
                if !layers.contains(id) {
                    layers.push(id.clone());
                }
            }
        }
        if layers.is_empty() {
            return Flow::Stay;
        }
        if self.action == Action::Active {
            layers.truncate(1);
        }
        self.act(&layers, cx);
        Flow::Exit
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
        cx.selection.set_hover(hit);
        self.hover = hit
            .and_then(|slot| cx.doc.get(slot))
            .map(|e| (name(cx, &e.base().layer_id), p.raw));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = cx.spatial.pick(p.raw, cx.pick_tolerance()) else {
            let text = format!(
                "Bir nesneye tıklayın: {} onun katmanında çalışır.",
                self.action.label()
            );
            cx.say(Level::Warn, text);
            return;
        };
        let Some(layer) = cx.doc.get(slot).map(|e| e.base().layer_id.clone()) else {
            return;
        };
        if self.action == Action::Isolate {
            if !self.gathered.contains(&layer) {
                self.gathered_names.push(name(cx, &layer));
                self.gathered.push(layer);
            }
            if !self.picked.contains(&slot) {
                self.picked.push(slot);
            }
            self.redraw(cx);
            return;
        }
        self.act(&[layer], cx);
        if self.action == Action::Active {
            self.done = true;
        }
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Enter and a quick right click: Katmanı yalıt isolates what it gathered;
    /// every action leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.action == Action::Isolate && !self.gathered.is_empty() {
            let ids = std::mem::take(&mut self.gathered);
            self.gathered_names.clear();
            self.act(&ids, cx);
        }
        Flow::Exit
    }

    /// Esc: Katmanı yalıt drops the layers it gathered; then it leaves.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.action != Action::Isolate || self.gathered.is_empty() {
            return false;
        }
        self.gathered.clear();
        self.gathered_names.clear();
        self.picked.clear();
        self.drawn = Preview::default();
        true
    }

    /// Ctrl+Z while gathering takes back the newest layer.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.action != Action::Isolate {
            return false;
        }
        let Some(layer) = self.gathered.pop() else {
            return false;
        };
        self.gathered_names.pop();
        let doc = &*cx.doc;
        self.picked
            .retain(|slot| doc.get(*slot).is_some_and(|e| e.base().layer_id != layer));
        self.redraw(cx);
        true
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = self.drawn.clone();
        preview.tag = self.hover.as_ref().map(|(layer, at)| Tag {
            at: *at,
            lines: vec![format!("Katman: {layer}"), self.action.click().to_owned()],
        });
        preview
    }
}
