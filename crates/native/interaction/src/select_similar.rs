//! Benzerini seç (`tool.selectSimilar`, docs/adr/0187 §4): the selected
//! objects are the examples, or the one clicked; every visible object
//! similar to one of them is selected. The web's `SelectSimilarTool`
//! (`apps/web/src/tools/selectSimilarTool.ts`) and its rule
//! (`apps/web/src/model/selectSimilar.ts`); both run
//! `fixtures/selection/v1/similar.json`.
//!
//! - An object is similar when it equals at least one example in every
//!   criterion that is on: Tür (T; its kind, and an insert's block too),
//!   Katman (K), Renk (R; its own colour, “by layer” a value of its own) and
//!   Sembol (S; its own symbol). All on at first, remembered from run to run;
//!   with none on every visible object is similar.
//! - The selection filter passes what it holds (§5). The result replaces the
//!   selection, or joins the selection the tool had when Shift was held at
//!   the click.
//! - The tool stays: a criterion changed selects again from the same
//!   examples, a click on another object makes it the example; Enter, Esc or
//!   a right click ends.

use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::selectable;
use crate::tool::{Context, Cursor, Flow, Memory, Pointer, Preview, Tool};

/// The tool's id: its command is `tool.selectSimilar`.
pub const ID: &str = "selectSimilar";
pub const LABEL: &str = "Benzerini seç";

/// What must be equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Criteria {
    pub kind: bool,
    pub layer: bool,
    pub color: bool,
    pub symbol: bool,
}

impl Criteria {
    pub const ALL: Criteria = Criteria {
        kind: true,
        layer: true,
        color: true,
        symbol: true,
    };

    /// The criteria that are on as the log says them: “tür, katman ve renk”.
    pub fn text(&self) -> String {
        let on: Vec<&str> = [
            (self.kind, "tür"),
            (self.layer, "katman"),
            (self.color, "renk"),
            (self.symbol, "sembol"),
        ]
        .into_iter()
        .filter_map(|(on, word)| on.then_some(word))
        .collect();
        match on.as_slice() {
            [] => "ölçütsüz: bütün nesneler".to_owned(),
            [one] => (*one).to_owned(),
            [rest @ .., last] => format!("{} ve {last}", rest.join(", ")),
        }
    }
}

/// The facts the criteria that are on compare, `None` where one is off:
/// the kind and an insert's block, the layer, the colour, the symbol.
type Key<'a> = (
    Option<(&'a str, Option<&'a str>)>,
    Option<&'a str>,
    Option<Option<&'a str>>,
    Option<Option<&'a str>>,
);

/// What an object is compared by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub id: Slot,
    pub kind: &'static str,
    /// An insert's block, none for any other object.
    pub block: Option<String>,
    pub layer: String,
    pub color: Option<String>,
    pub symbol: Option<String>,
}

impl Facts {
    pub fn of(id: Slot, e: &Entity) -> Facts {
        let b = e.base();
        Facts {
            id,
            kind: e.kind(),
            block: match e {
                Entity::Insert(i) => Some(i.block.to_text()),
                _ => None,
            },
            layer: b.layer_id.clone(),
            color: b.color.clone(),
            symbol: b.symbol.clone(),
        }
    }

    /// The facts the criteria compare.
    fn key(&self, c: Criteria) -> Key<'_> {
        (
            c.kind.then_some((self.kind, self.block.as_deref())),
            c.layer.then_some(self.layer.as_str()),
            c.color.then_some(self.color.as_deref()),
            c.symbol.then_some(self.symbol.as_deref()),
        )
    }
}

/// The objects similar to at least one of `examples`, in the objects' order.
pub fn similar_to(objects: &[Facts], examples: &[Facts], c: Criteria) -> Vec<Slot> {
    let wanted: Vec<_> = examples.iter().map(|e| e.key(c)).collect();
    objects
        .iter()
        .filter(|o| wanted.contains(&o.key(c)))
        .map(|o| o.id)
        .collect()
}

/// The criteria's options in the prompt and the keys that turn them.
const OPTIONS: [(&str, &str); 4] = [
    ("Tür", "T"),
    ("Katman", "K"),
    ("Renk", "R"),
    ("Sembol", "S"),
];

fn on(c: Criteria, i: usize) -> bool {
    [c.kind, c.layer, c.color, c.symbol][i]
}

fn turned(c: Criteria, i: usize) -> Criteria {
    let mut c = c;
    match i {
        0 => c.kind = !c.kind,
        1 => c.layer = !c.layer,
        2 => c.color = !c.color,
        _ => c.symbol = !c.symbol,
    }
    c
}

/// Benzerini seç.
#[derive(Clone, Debug, Default)]
pub struct SelectSimilar {
    examples: Vec<Slot>,
    /// What the selection was when Shift was held at the click: the result joins it.
    base: Vec<Slot>,
    /// What the session remembered, as of the last call (the prompt sees no context).
    memory: Memory,
}

impl SelectSimilar {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
    }

    /// Selects what is similar to the examples still in the drawing.
    fn select(&mut self, cx: &mut Context<'_>) {
        let examples: Vec<Facts> = self
            .examples
            .iter()
            .filter_map(|s| cx.doc.get(*s).map(|e| Facts::of(*s, e)))
            .collect();
        if examples.is_empty() {
            return;
        }
        let layers = cx.doc.layers();
        let objects: Vec<Facts> = cx
            .doc
            .entities()
            .filter(|e| layers.is_visible(&e.base().layer_id))
            .map(|e| Facts::of(Slot(e.base().id), e))
            .collect();
        let c = cx.memory.similar;
        let ids = selectable::ids(cx, similar_to(&objects, &examples, c));
        let n = ids.len();
        cx.selection.set(self.base.iter().copied().chain(ids));
        cx.say(
            Level::Info,
            format!("Benzer {n} nesne seçildi ({}).", c.text()),
        );
    }
}

impl Tool for SelectSimilar {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let mut prompt = if self.examples.is_empty() {
            Prompt::new(LABEL, "örnek nesneye tıklayın")
        } else {
            Prompt::new(
                LABEL,
                "ölçütleri değiştirin ya da başka bir örneğe tıklayın",
            )
        };
        for (i, (name, key)) in OPTIONS.iter().enumerate() {
            prompt = prompt.toggle(name, key, on(self.memory.similar, i));
        }
        if !self.examples.is_empty() {
            prompt = prompt.option("Bitir", "Enter");
        }
        prompt
    }

    /// The selection, when there is one, is the examples.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        if !cx.selection.is_empty() {
            self.examples = cx.selection.ids().to_vec();
            self.select(cx);
        }
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let hit = selectable::hover(cx, p.raw);
        cx.selection.set_hover(hit);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let Some(hit) = selectable::pick(cx, p.raw) else {
            cx.say(Level::Warn, "Örnek olacak bir nesneye tıklayın.");
            return;
        };
        self.base = if p.shift {
            cx.selection.ids().to_vec()
        } else {
            Vec::new()
        };
        self.examples = vec![hit];
        self.select(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let key = upper_tr(js_trim(text));
        let Some(i) = OPTIONS.iter().position(|(_, k)| *k == key) else {
            return false;
        };
        cx.memory.similar = turned(cx.memory.similar, i);
        self.see(cx);
        if !self.examples.is_empty() {
            self.select(cx);
        }
        true
    }

    /// Enter or a quick right click ends.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn point_count(&self) -> usize {
        0
    }

    /// An object is clicked, not a point: nothing to snap to.
    fn snaps(&self) -> bool {
        false
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The independent reference's cases (`fixtures/selection/v1/similar.json`,
    /// `scripts/fixtures/selection_cases.py`); the web runs the same file.
    #[test]
    fn the_shared_cases_hold() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/selection/v1/similar.json"
        );
        let f: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
        let text = |v: &serde_json::Value| v.as_str().map(str::to_owned);
        let objects: Vec<Facts> = f["objects"]
            .as_array()
            .expect("objects")
            .iter()
            .map(|o| Facts {
                id: Slot(o["id"].as_u64().expect("id") as u32),
                kind: crate::selectable::KINDS
                    .iter()
                    .copied()
                    .find(|k| Some(*k) == o["kind"].as_str())
                    .expect("a kind"),
                block: text(&o["block"]),
                layer: text(&o["layer"]).expect("a layer"),
                color: text(&o["color"]),
                symbol: text(&o["symbol"]),
            })
            .collect();
        for c in f["cases"].as_array().expect("cases") {
            let examples: Vec<Facts> = objects
                .iter()
                .filter(|o| {
                    c["examples"]
                        .as_array()
                        .expect("examples")
                        .iter()
                        .any(|e| e.as_u64() == Some(u64::from(o.id.0)))
                })
                .cloned()
                .collect();
            let on = |k: &str| c["criteria"][k].as_bool().expect("a criterion");
            let criteria = Criteria {
                kind: on("kind"),
                layer: on("layer"),
                color: on("color"),
                symbol: on("symbol"),
            };
            let want: Vec<Slot> = c["expect"]
                .as_array()
                .expect("expect")
                .iter()
                .map(|e| Slot(e.as_u64().expect("an id") as u32))
                .collect();
            assert_eq!(
                similar_to(&objects, &examples, criteria),
                want,
                "{}",
                c["name"]
            );
        }
    }

    fn facts(id: u32, kind: &'static str, layer: &str, color: Option<&str>) -> Facts {
        Facts {
            id: Slot(id),
            kind,
            block: None,
            layer: layer.to_owned(),
            color: color.map(str::to_owned),
            symbol: None,
        }
    }

    #[test]
    fn similar_objects_equal_an_example_in_every_criterion_on() {
        let objects = [
            facts(1, "polygon", "Parsel", None),
            facts(2, "polygon", "Parsel", Some("#E5484D")),
            facts(3, "polygon", "Yapı", None),
            facts(4, "line", "Parsel", None),
        ];
        let ex = [objects[0].clone()];
        assert_eq!(similar_to(&objects, &ex, Criteria::ALL), [Slot(1)]);
        let no_colour = Criteria {
            color: false,
            ..Criteria::ALL
        };
        assert_eq!(similar_to(&objects, &ex, no_colour), [Slot(1), Slot(2)]);
        let kind_only = Criteria {
            kind: true,
            layer: false,
            color: false,
            symbol: false,
        };
        assert_eq!(
            similar_to(&objects, &ex, kind_only),
            [Slot(1), Slot(2), Slot(3)]
        );
        let none = Criteria {
            kind: false,
            layer: false,
            color: false,
            symbol: false,
        };
        assert_eq!(similar_to(&objects, &ex, none).len(), 4);
        assert_eq!(none.text(), "ölçütsüz: bütün nesneler");
        assert_eq!(Criteria::ALL.text(), "tür, katman, renk ve sembol");
        assert_eq!(kind_only.text(), "tür");
    }
}
