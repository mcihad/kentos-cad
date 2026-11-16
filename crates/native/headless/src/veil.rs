//! What an agent is given of a drawing (docs/adr/0738 §1). A field a form keeps out of examples (`redact`,
//! docs/adr/0443 §2) or a layer marks `withheld` (docs/adr/0638 §5) is read on screen and written by the
//! drawing's owner; no answer to an agent (MCP, the desktop's agents' link) carries its values. The owner's own
//! Python reads the drawing whole: the drawing is theirs (docs/adr/0550 §3).
//!
//! An answer is cut where its shapes hold a field's value: an attribute map (`attrs`, `values`) under the field's
//! name; a join view's cells and sums under a column of the field; a reconciliation's two sides of the field; a
//! statistic of the field. A value of a hidden field quoted in a text (“…”) is written as hidden too: a rule's
//! words name the value they refuse. The command history's rule writes the journal's inputs (docs/adr/0443 §2).
//! What is cut is written [`HIDDEN`]; the summary names the hidden fields.
//!
//! **Every field** (docs/adr/8057 §103, PY-AI-13): an agent the user or the organisation sends no attribute value
//! ([`Veil::everything`]) reads the drawing as if every field were hidden. A free text then hides a value wherever it
//! stands as words, not only quoted (an extension's finding writes `Malik Ali Veli` as it likes), except where the
//! answer names what it is (`id`, `layerId`, `kind` …: [`STRUCTURAL`]); a value of fewer than three letters is hidden
//! in an attribute map alone (`1`, `A` would cut every count). It may write a field's value: writing sends none out.

use std::collections::BTreeSet;

use kentos_domain::Document;
use kentos_native_application::python_call::{HIDDEN, redacted, redacted_names};

pub use every::STRUCTURAL;
use serde_json::{Map, Value};

/// The keys under which a call's texts are what it writes (an object's attribute values, a text's words, a
/// note, a file's path), not what it reads (docs/adr/0743 §1).
pub const WRITTEN: [&str; 9] = [
    "attrs", "values", "value", "text", "content", "note", "notes", "comment", "path",
];

/// The keys under which a call's texts name a thing of the drawing or of the call (an object, a layer, a tool, a
/// model and its steps), never a field or an expression (docs/adr/8057 §108). A layer's id is often a field's name
/// (`parsel`, `ada`): a run on the parcels' layer was refused for naming the field; a model an agent named
/// `parsel-tampon-birlesik` read as the expression `parsel - tampon - birlesik` and was refused too (the real window).
pub const NAMING: [&str; 18] = [
    "id", "uid", "uids", "layerId", "layer", "kind", "type", "scope", "command", "tool", "step",
    "output", "code", "rule", "version", "revision", "planId", "newName",
];

/// The drawing's hidden fields and the values they hold.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Veil {
    names: Vec<String>,
    values: BTreeSet<String>,
    /// Every field hidden ([`Veil::everything`]).
    everything: bool,
}

impl Veil {
    /// The fields `doc` hides from an agent and the values they hold; none when it hides none.
    pub fn of(doc: &Document) -> Option<Veil> {
        let names = redacted_names(doc);
        if names.is_empty() {
            return None;
        }
        let mut values = BTreeSet::new();
        for e in doc.entities() {
            for (k, v) in &e.base().attrs {
                if !v.is_empty() && names.contains(k) {
                    values.insert(v.clone());
                }
            }
        }
        Some(Veil {
            names,
            values,
            everything: false,
        })
    }

    /// A veil of `names` over `values` (tests and other hosts).
    pub fn new(names: Vec<String>, values: impl IntoIterator<Item = String>) -> Veil {
        Veil {
            names,
            values: values.into_iter().filter(|v| !v.is_empty()).collect(),
            everything: false,
        }
    }

    /// The hidden fields' names (and keys).
    pub fn names(&self) -> &[String] {
        &self.names
    }

    fn hides(&self, name: &str) -> bool {
        self.names.iter().any(|n| n == name)
    }

    /// Where an agent's call input names a hidden field (docs/adr/0743 §1): the first place in the input's own
    /// order (keys sorted, `a.b[2]`) and the field. A key that is the field's name names it, wherever it is; a
    /// text names it when it is the name, or an expression or a template (`{…}`) that reads the field. The texts
    /// a call writes ([`WRITTEN`]) are content, not read: only their keys are looked at.
    pub fn named_in(&self, input: &Value) -> Option<(String, String)> {
        let mut path = String::new();
        self.named_at(input, &mut path, false)
    }

    fn named_at(&self, v: &Value, path: &mut String, content: bool) -> Option<(String, String)> {
        match v {
            Value::String(s) if !content => self.named_by(s).map(|f| (path.clone(), f)),
            Value::Array(items) => items.iter().enumerate().find_map(|(i, item)| {
                let len = path.len();
                path.push_str(&format!("[{i}]"));
                let found = self.named_at(item, path, content);
                path.truncate(len);
                found
            }),
            Value::Object(map) => map.iter().find_map(|(k, item)| {
                let len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(k);
                // Every field hidden, its value may still be written: writing sends nothing out (§103).
                let writing = content && self.everything;
                // A model step's values are its tool's parameters, each given (`{"kind": "value", "value": …}`) as
                // the parameter's own would be, not written values (docs/adr/8057 §108).
                let step_values = k == "values" && map.contains_key("tool");
                let given =
                    k == "value" && map.get("kind").and_then(Value::as_str) == Some("value");
                let inner = if step_values || given {
                    content
                } else {
                    content || WRITTEN.contains(&k.as_str()) || NAMING.contains(&k.as_str())
                };
                let found = if self.hides(k) && !writing {
                    Some((path.clone(), k.clone()))
                } else {
                    self.named_at(item, path, inner)
                };
                path.truncate(len);
                found
            }),
            _ => None,
        }
    }

    fn named_by(&self, s: &str) -> Option<String> {
        if let Some(n) = self.names.iter().find(|n| *n == s) {
            return Some(n.clone());
        }
        if s.contains('{')
            && let Some(n) = self
                .names
                .iter()
                .find(|n| kentos_expression::editor::template_names_field(s, n))
        {
            return Some(n.clone());
        }
        let fields = kentos_expression::editor::fields_of(s);
        self.names.iter().find(|n| fields.contains(n)).cloned()
    }

    /// The hidden field `text` reads: it is the field's name, an expression or a template that reads it (an
    /// İşlemler tool's expression parameter, docs/adr/8057 §108).
    pub fn read_by(&self, text: &str) -> Option<String> {
        self.named_by(text)
    }

    /// The hidden field among `names`, written with commas between them (an İşlemler tool's field parameter).
    pub fn among(&self, names: &str) -> Option<String> {
        names
            .split(',')
            .map(str::trim)
            .find(|n| self.hides(n))
            .map(str::to_owned)
    }

    /// An agent's call refused because it names a hidden field ([`Veil::named_in`]), as the protocol answers.
    pub fn refusal(&self, input: &Value) -> Option<kentos_commands::CommandError> {
        let (at, field) = self.named_in(input)?;
        Some(self.refusal_at(&at, &field))
    }

    /// The refusal of a call that names hidden `field` at `at` (a place in its input, `a.b[2]`).
    pub fn refusal_at(&self, at: &str, field: &str) -> kentos_commands::CommandError {
        let message = if self.everything {
            format!(
                "Bu ajana öznitelik değerleri gönderilmiyor (kullanıcının ya da kurumun seçimi): “{field}” alanıyla \
                 süzme, sıralama ve hesap ({at}) çizimin sahibinindir. Değer gerektiren işi kullanıcıya bırakın."
            )
        } else {
            format!(
                "“{field}” alanı çizimde ajandan gizli ({at}): o alanla süzme, sıralama, hesap ve yazma çizimin \
                 sahibinindir. Çağrıyı bu alanı anmadan yeniden yapın."
            )
        };
        kentos_commands::CommandError {
            code: kentos_commands::codes::HIDDEN_FIELD.into(),
            message,
            path: Some(if at.is_empty() {
                "input".into()
            } else {
                format!("input.{at}")
            }),
            ..kentos_commands::CommandError::default()
        }
    }

    /// `answer` as an agent may read it.
    pub fn cut(&self, answer: Value) -> Value {
        self.cut_as(answer, false)
    }

    /// `answer` cut, its texts saying what a part is (`structural`) or what the drawing holds.
    fn cut_as(&self, answer: Value, structural: bool) -> Value {
        match answer {
            Value::Object(map) => Value::Object(self.cut_object(map)),
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .map(|v| self.cut_as(v, structural))
                    .collect(),
            ),
            Value::String(text) if structural => Value::String(self.quoted(&text)),
            Value::String(text) => Value::String(self.text(&text)),
            other => other,
        }
    }

    /// A drawing's summary as an agent reads it: cut, with the hidden fields named (`hidden`).
    pub fn summary(&self, summary: Value) -> Value {
        let mut summary = self.cut(summary);
        if let Some(fields) = summary.as_object_mut() {
            fields.insert("hidden".into(), Value::from(self.names.clone()));
            // Why, for the agent to say it right: not the drawing's hiding (measured: an agent told the user to open
            // the fields' privacy).
            if self.everything {
                fields.insert("valuesWithheld".into(), Value::from(every::WITHHELD));
            }
        }
        summary
    }

    /// A drawing's journal as an agent reads it: its calls' inputs as the command history writes them
    /// (docs/adr/0443 §2), then cut.
    pub fn journal(&self, journal: Value) -> Value {
        self.cut(redacted(&journal, &self.names))
    }

    /// A text with every hidden value it quotes (“…”) written as hidden; every field hidden, every value it holds as
    /// words too ([`Veil::everything`]).
    pub fn text(&self, text: &str) -> String {
        let quoted = self.quoted(text);
        if self.everything {
            self.words(&quoted)
        } else {
            quoted
        }
    }

    /// A text with every hidden value it quotes (“…”) written as hidden.
    fn quoted(&self, text: &str) -> String {
        if self.values.is_empty() || !text.contains('“') {
            return text.to_owned();
        }
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(open) = rest.find('“') {
            let (before, after) = rest.split_at(open + '“'.len_utf8());
            out.push_str(before);
            match after.find('”') {
                Some(close) => {
                    let inner = &after[..close];
                    out.push_str(if self.values.contains(inner) {
                        HIDDEN
                    } else {
                        inner
                    });
                    rest = &after[close..];
                }
                None => {
                    rest = after;
                    break;
                }
            }
        }
        out.push_str(rest);
        out
    }

    fn cut_object(&self, mut map: Map<String, Value>) -> Map<String, Value> {
        self.cut_join(&mut map);
        // A reconciliation's field: its name and the two sides' values (`ConflictField`, docs/adr/0538).
        if map
            .keys()
            .all(|k| matches!(k.as_str(), "name" | "layer" | "table"))
            && map
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| self.hides(n))
        {
            for side in ["layer", "table"] {
                hide(&mut map, side);
            }
        }
        // A statistic of the field (`QueriedStatistic`, docs/adr/0581).
        if map.contains_key("read")
            && map
                .get("field")
                .and_then(Value::as_str)
                .is_some_and(|n| self.hides(n))
        {
            for figure in ["read", "skipped", "sum", "mean", "min", "max", "std"] {
                hide(&mut map, figure);
            }
        }
        map.into_iter()
            .map(|(k, v)| {
                let v = match v {
                    Value::Object(attrs) if k == "attrs" || k == "values" => {
                        Value::Object(self.cut_attrs(attrs))
                    }
                    other => self.cut_as(other, STRUCTURAL.contains(&k.as_str())),
                };
                (k, v)
            })
            .collect()
    }

    /// An attribute map with the hidden fields' values hidden.
    fn cut_attrs(&self, attrs: Map<String, Value>) -> Map<String, Value> {
        attrs
            .into_iter()
            .map(|(k, v)| {
                let v = if self.hides(&k) && !v.is_null() {
                    Value::String(HIDDEN.into())
                } else {
                    self.cut(v)
                };
                (k, v)
            })
            .collect()
    }

    /// A join view's answer (`JoinsAnswer`, docs/adr/0527): the cells and sums under a hidden field's column.
    fn cut_join(&self, map: &mut Map<String, Value>) {
        let Some(columns) = map.get("columns").and_then(Value::as_array) else {
            return;
        };
        let hidden: Vec<usize> = columns
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.get("field")
                    .and_then(Value::as_str)
                    .is_some_and(|f| self.hides(f))
            })
            .map(|(i, _)| i)
            .collect();
        if hidden.is_empty() {
            return;
        }
        if let Some(Value::Array(rows)) = map.get_mut("rows") {
            for row in rows {
                if let Some(Value::Array(cells)) = row.get_mut("cells") {
                    for &i in &hidden {
                        if let Some(cell) = cells.get_mut(i).filter(|c| !c.is_null()) {
                            *cell = Value::String(HIDDEN.into());
                        }
                    }
                }
            }
        }
        if let Some(Value::Array(totals)) = map.get_mut("totals") {
            for total in totals {
                let column = total.get("column").and_then(Value::as_u64);
                if column.is_some_and(|c| hidden.contains(&(c as usize)))
                    && let Some(total) = total.as_object_mut()
                {
                    for figure in ["sum", "count", "skipped"] {
                        hide(total, figure);
                    }
                }
            }
        }
    }
}

/// `map`'s `key` written as hidden, when it holds a value.
fn hide(map: &mut Map<String, Value>, key: &str) {
    if let Some(v) = map.get_mut(key).filter(|v| !v.is_null()) {
        *v = Value::String(HIDDEN.into());
    }
}

mod every;
#[cfg(feature = "mod-gis")]
pub(crate) mod reads;
#[cfg(test)]
mod tests;
