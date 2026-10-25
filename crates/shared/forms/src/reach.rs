//! The one resolver of what a caller may read and take out (docs/adr/0550; plan/gis.md CBS-38): which
//! records of a layer, and which of its fields, reach the caller on screen, in a file, in a count, in a
//! statistic, in an agent's query. Pure: the host gives the project's forms, the layers' fields, the `@`
//! values and the caller's class; every read path, export and job asks the answer here instead of
//! carrying a check of its own.
//!
//! - **The caller** is a class, not a role (the roles and permissions stay where they are,
//!   `kentos_application::access`): [`Caller::through_forms`] (a Veri girişi: reads and writes through
//!   forms alone), [`Caller::reader`] (anyone else who reads the project: viewer, editor …) and
//!   [`Caller::whole`] (all rights over the data: the owner of a local drawing, the headless server; also
//!   a holder of `forms.design`, who may change the marks below anyway).
//! - **A field** has a [`Level`]: `Hidden` (a Veri girişi does not read it: its layer's live forms do not
//!   show it), `Redacted` (a form marks it `redact`, docs/adr/0443: readable where people work with the
//!   record, in no file, copy, count, statistic, chart or agent's answer) or `Open`.
//! - **A record** reaches a Veri girişi when one of its layer's live forms reaches it (docs/adr/0539). A
//!   layer whose forms are all withdrawn reaches nothing: the end of a form's use never opens its records.
//!   Everyone else reads every record.
//! - **A copy of the whole project** (the project's `.kcad`, an export job) is built from the records:
//!   [`Reach::copy_attrs`] cuts the redacted fields unless the caller designs forms. A copy that is stored
//!   as it was written (a file project's revision, a checkpoint) cannot be cut: [`Reach::whole_copy_ok`]
//!   says whether the caller may take it at all.
//! - **The stamp** ([`Reach::stamp`]) is the decision as text: a job built from a stamp that no longer holds
//!   when it finishes is refused (docs/adr/0550 §5).

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{FormItem, LayerField, ProjectForm};
use kentos_expression::{Unit, Variable};

use crate::record::{Attrs, Object};
use crate::scope::Scopes;

/// How far a field reaches a caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Not read at all.
    Hidden,
    /// Read where people work with the record; in nothing that leaves.
    Redacted,
    /// Read and taken out.
    Open,
}

impl Level {
    /// Whether the value is shown to the caller where the record is worked with.
    pub fn readable(self) -> bool {
        self != Level::Hidden
    }

    /// Whether the value may go into a file, a copy, a count, a statistic, a chart, an agent's answer.
    pub fn exportable(self) -> bool {
        self == Level::Open
    }
}

/// Who asks: a class of caller (docs/adr/0550 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller {
    /// Reads and writes through forms alone (a Veri girişi, docs/adr/0411 §6).
    pub through_forms: bool,
    /// May design forms (`forms.design`): takes whole copies, for the marks are the caller's to change.
    pub designs: bool,
}

impl Caller {
    /// A Veri girişi.
    pub const THROUGH_FORMS: Caller = Caller {
        through_forms: true,
        designs: false,
    };
    /// Anyone else who reads the project without designing its forms.
    pub const READER: Caller = Caller {
        through_forms: false,
        designs: false,
    };
    /// All rights over the data: a local drawing's owner, the headless server, a form designer.
    pub const WHOLE: Caller = Caller {
        through_forms: false,
        designs: true,
    };

    pub fn through_forms() -> Caller {
        Caller::THROUGH_FORMS
    }

    pub fn reader() -> Caller {
        Caller::READER
    }

    pub fn whole() -> Caller {
        Caller::WHOLE
    }
}

/// A form is live when it is published and not withdrawn.
pub fn live(form: &ProjectForm) -> bool {
    !form.releases.is_empty() && !form.withdrawn
}

/// How a layer is bound by forms.
#[derive(Clone, Debug)]
enum Binding {
    /// Live forms: the fields their latest releases show.
    Live(BTreeSet<String>),
    /// Published forms, none live: a Veri girişi reaches nothing.
    Closed,
}

/// The layer field a form item names, when the layer has it.
fn field_of<'a>(fields: &'a [LayerField], key: &str) -> Option<&'a LayerField> {
    fields.iter().find(|f| f.key() == key)
}

/// What `redact` marks on `form`'s layer, as attribute names, in its draft and its latest release: the
/// layer fields' names (an item names a key; an attribute is stored under the field's name), the key
/// itself when the layer has no such field.
fn marked(form: &ProjectForm, fields: &[LayerField]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let bodies = std::iter::once(&form.draft).chain(form.releases.last().map(|r| &r.body));
    for body in bodies {
        for (_, item) in body.walk() {
            if let FormItem::Field(f) = item
                && f.redact
            {
                out.insert(
                    field_of(fields, &f.field).map_or_else(|| f.field.clone(), |l| l.name.clone()),
                );
            }
        }
    }
    out
}

/// The layer fields' keys and names the project's forms mark `redact`, in any form's draft or latest
/// release (docs/adr/0443 §2, docs/adr/0550 §1): what the recorder, a table's file, a statistic and an
/// atlas leave out. An earlier release's mark does not count once a later release drops it.
pub fn redacted_names<'a>(
    forms: &[ProjectForm],
    fields_of: impl Fn(&str) -> &'a [LayerField],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for form in forms {
        let fields = fields_of(&form.layer);
        let bodies = std::iter::once(&form.draft).chain(form.releases.last().map(|r| &r.body));
        for body in bodies {
            for (_, item) in body.walk() {
                if let FormItem::Field(f) = item
                    && f.redact
                {
                    let names = std::iter::once(f.field.clone()).chain(
                        fields
                            .iter()
                            .filter(|l| l.key() == f.field)
                            .map(|l| l.name.clone()),
                    );
                    for n in names {
                        if !out.contains(&n) {
                            out.push(n);
                        }
                    }
                }
            }
        }
    }
    out
}

/// The names of the fields marked `withheld` (docs/adr/0638 §5), in the layers given: their values do not leave
/// the drawing, as a form's `redact` keeps them, with no form needed.
pub fn withheld_names<'a>(fields: impl IntoIterator<Item = &'a LayerField>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in fields.into_iter().filter(|f| f.withheld) {
        if !out.contains(&f.name) {
            out.push(f.name.clone());
        }
    }
    out
}

/// What a caller reaches in a project.
#[derive(Clone, Debug)]
pub struct Reach {
    caller: Caller,
    layers: BTreeMap<String, Binding>,
    redacted: BTreeMap<String, BTreeSet<String>>,
    scopes: Scopes,
}

impl Reach {
    /// The reach of `caller` over `forms`' layers (`fields_of` gives a layer's fields; `variables` and
    /// `unit` are what the forms' scopes are judged with).
    pub fn new<'a>(
        forms: &[ProjectForm],
        fields_of: impl Fn(&str) -> &'a [LayerField] + Copy,
        variables: &[Variable],
        unit: Unit,
        caller: Caller,
    ) -> Reach {
        let mut layers: BTreeMap<String, Binding> = BTreeMap::new();
        let mut redacted: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for form in forms {
            let fields = fields_of(&form.layer);
            let marks = marked(form, fields);
            if !marks.is_empty() {
                redacted
                    .entry(form.layer.clone())
                    .or_default()
                    .extend(marks);
            }
            let Some(release) = form.releases.last() else {
                continue;
            };
            let entry = layers.entry(form.layer.clone()).or_insert(Binding::Closed);
            if form.withdrawn {
                continue;
            }
            let Binding::Live(shown) = entry else {
                *entry = Binding::Live(BTreeSet::new());
                if let Binding::Live(shown) = entry {
                    show(&release.body.walk(), fields, shown);
                }
                continue;
            };
            show(&release.body.walk(), fields, shown);
        }
        Reach {
            caller,
            layers,
            redacted,
            scopes: Scopes::new(forms, fields_of, variables, unit),
        }
    }

    pub fn caller(&self) -> Caller {
        self.caller
    }

    /// The reach with the fields `layers` mark `withheld` (docs/adr/0638 §5) as a form's `redact` marks them:
    /// readable where people work with the record, never taken out by one who does not design forms.
    pub fn withholding<'a>(
        mut self,
        layers: impl IntoIterator<Item = (&'a str, &'a [LayerField])>,
    ) -> Reach {
        for (layer, fields) in layers {
            let names = withheld_names(fields);
            if !names.is_empty() {
                self.redacted
                    .entry(layer.to_owned())
                    .or_default()
                    .extend(names);
            }
        }
        self
    }

    /// How far attribute `attr` of a `layer` record reaches the caller.
    pub fn level(&self, layer: &str, attr: &str) -> Level {
        let shown = !self.caller.through_forms
            || match self.layers.get(layer) {
                None => true,
                Some(Binding::Live(names)) => names.contains(attr),
                Some(Binding::Closed) => false,
            };
        if !shown {
            return Level::Hidden;
        }
        if self.redacted.get(layer).is_some_and(|s| s.contains(attr)) {
            Level::Redacted
        } else {
            Level::Open
        }
    }

    /// `attrs` as the caller reads them: the hidden ones left out.
    pub fn readable(&self, layer: &str, attrs: &Attrs) -> Attrs {
        self.kept(layer, attrs, Level::readable)
    }

    /// `attrs` as they may leave: only the open ones (a file, a copy, a count, a statistic, a chart, an
    /// agent's answer).
    pub fn exportable(&self, layer: &str, attrs: &Attrs) -> Attrs {
        self.kept(layer, attrs, Level::exportable)
    }

    /// `attrs` as a copy of the whole project carries them: whole for a form designer, otherwise only
    /// the open ones.
    pub fn copy_attrs(&self, layer: &str, attrs: &Attrs) -> Attrs {
        if self.caller.designs {
            attrs.clone()
        } else {
            self.exportable(layer, attrs)
        }
    }

    fn kept(&self, layer: &str, attrs: &Attrs, keep: fn(Level) -> bool) -> Attrs {
        attrs
            .iter()
            .filter(|(k, _)| keep(self.level(layer, k)))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Whether the record is within reach: always for anyone but a Veri girişi; for one, when a live form
    /// of its layer reaches it (none when the layer's forms are all withdrawn).
    pub fn sees(&self, layer: &str, attrs: &Attrs, object: &Object) -> bool {
        if !self.caller.through_forms {
            return true;
        }
        match self.layers.get(layer) {
            Some(Binding::Closed) => false,
            _ => self.scopes.sees(layer, attrs, object),
        }
    }

    /// The layers whose records the caller may not all reach: scoped ones and closed ones. Empty for
    /// anyone but a Veri girişi.
    pub fn limited_layers(&self) -> Vec<String> {
        if !self.caller.through_forms {
            return Vec::new();
        }
        let mut out = self.scopes.scoped_layers();
        for (layer, binding) in &self.layers {
            if matches!(binding, Binding::Closed) && !out.contains(layer) {
                out.push(layer.clone());
            }
        }
        out.sort();
        out
    }

    /// Whether any layer's records are narrowed for the caller.
    pub fn narrows_rows(&self) -> bool {
        !self.limited_layers().is_empty()
    }

    /// The fields a Veri girişi reads on each layer its live forms bind, by name (a layer no form binds
    /// is not here: all of its attributes are read; a closed layer has an empty set).
    pub fn shown_fields(&self) -> BTreeMap<String, BTreeSet<String>> {
        self.layers
            .iter()
            .map(|(l, b)| {
                (
                    l.clone(),
                    match b {
                        Binding::Live(s) => s.clone(),
                        Binding::Closed => BTreeSet::new(),
                    },
                )
            })
            .collect()
    }

    /// Whether any field is marked `redact`.
    pub fn has_redacted(&self) -> bool {
        !self.redacted.is_empty()
    }

    /// The names a `layer`'s forms mark `redact`.
    pub fn redacted_in(&self, layer: &str) -> Vec<String> {
        self.redacted
            .get(layer)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Whether the caller may take a copy that is stored as it was written and cannot be cut (a file
    /// project's revision, a checkpoint): when nothing is marked `redact`, or the caller designs forms.
    pub fn whole_copy_ok(&self) -> bool {
        self.caller.designs || !self.has_redacted()
    }

    /// The decision as canonical text: who, which fields are shown and marked on which layers, which
    /// layers are narrowed or closed. Two reaches with the same stamp give the same answer to every
    /// question except a scope's value for a record.
    pub fn stamp(&self) -> String {
        format!(
            "through_forms={};designs={};shown={:?};redacted={:?};limited={:?};scoped={:?}",
            self.caller.through_forms,
            self.caller.designs,
            self.shown_fields(),
            self.redacted,
            self.limited_layers(),
            self.scopes.scoped_layers(),
        )
    }
}

/// The attribute names `body`'s field items show on a layer with `fields`.
fn show(items: &[(usize, &FormItem)], fields: &[LayerField], shown: &mut BTreeSet<String>) {
    for (_, item) in items {
        if let FormItem::Field(f) = item
            && let Some(field) = field_of(fields, &f.field)
        {
            shown.insert(field.name.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::FormBody;
    use kentos_expression::Value;
    use serde_json::json;

    fn fields() -> Vec<LayerField> {
        serde_json::from_value(json!([
            { "name": "Sorumlu", "kind": "text" },
            { "name": "Malik", "kind": "text" },
            { "name": "Kat", "kind": "integer" },
            { "name": "Not", "kind": "text" }
        ]))
        .unwrap()
    }

    fn item(field: &str, redact: bool) -> serde_json::Value {
        json!({ "type": "field", "id": format!("i-{field}"), "field": field, "redact": redact })
    }

    fn form(
        id: &str,
        layer: &str,
        items: &[serde_json::Value],
        scope: Option<&str>,
        published: bool,
        withdrawn: bool,
    ) -> ProjectForm {
        let body = json!({ "revision": 1, "items": items, "scope": scope });
        let typed: FormBody = serde_json::from_value(body.clone()).unwrap();
        let releases = if published {
            json!([{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }])
        } else {
            json!([])
        };
        serde_json::from_value(json!({ "id": id, "name": id, "layer": layer, "draft": body,
            "releases": releases, "withdrawn": withdrawn }))
        .unwrap()
    }

    fn reach(forms: &[ProjectForm], caller: Caller) -> Reach {
        let f = fields();
        let me = vec![Variable {
            name: "kullanici_eposta".into(),
            value: Value::text("ali@k.test".to_owned()),
            description: String::new(),
        }];
        Reach::new(forms, |_| f.as_slice(), &me, Unit::METRE, caller)
    }

    fn attrs(pairs: &[(&str, &str)]) -> Attrs {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn names(a: &Attrs) -> Vec<&str> {
        a.keys().map(String::as_str).collect()
    }

    #[test]
    fn a_veri_girisi_reads_what_its_live_forms_show_and_a_redacted_field_does_not_leave() {
        let forms = [form(
            "f1",
            "bina",
            &[item("Sorumlu", false), item("Malik", true)],
            None,
            true,
            false,
        )];
        let row = attrs(&[("Sorumlu", "ali"), ("Malik", "x"), ("Kat", "3")]);
        let entry = reach(&forms, Caller::through_forms());
        assert_eq!(entry.level("bina", "Sorumlu"), Level::Open);
        assert_eq!(entry.level("bina", "Malik"), Level::Redacted);
        assert_eq!(entry.level("bina", "Kat"), Level::Hidden);
        assert_eq!(names(&entry.readable("bina", &row)), ["Malik", "Sorumlu"]);
        assert_eq!(names(&entry.exportable("bina", &row)), ["Sorumlu"]);
        // A layer no form binds is read whole.
        assert_eq!(entry.level("yol", "Kat"), Level::Open);
        assert_eq!(
            names(&entry.readable("yol", &row)),
            ["Kat", "Malik", "Sorumlu"]
        );
    }

    #[test]
    fn a_reader_reads_every_field_and_a_redacted_one_does_not_leave_a_cut_copy() {
        let forms = [form(
            "f1",
            "bina",
            &[item("Malik", true)],
            None,
            true,
            false,
        )];
        let row = attrs(&[("Sorumlu", "ali"), ("Malik", "x")]);
        let reader = reach(&forms, Caller::reader());
        assert_eq!(names(&reader.readable("bina", &row)), ["Malik", "Sorumlu"]);
        assert_eq!(names(&reader.exportable("bina", &row)), ["Sorumlu"]);
        assert_eq!(names(&reader.copy_attrs("bina", &row)), ["Sorumlu"]);
        assert!(
            !reader.whole_copy_ok(),
            "a stored copy cannot be cut, so it is not given"
        );
        // A designer takes the whole copy: the mark is theirs to change; the rows' outputs still leave it out.
        let designer = reach(&forms, Caller::whole());
        assert_eq!(
            names(&designer.copy_attrs("bina", &row)),
            ["Malik", "Sorumlu"]
        );
        assert_eq!(names(&designer.exportable("bina", &row)), ["Sorumlu"]);
        assert!(designer.whole_copy_ok());
        // Nothing marked: everyone takes the whole copy.
        let plain = reach(
            &[form(
                "f",
                "bina",
                &[item("Malik", false)],
                None,
                true,
                false,
            )],
            Caller::reader(),
        );
        assert!(plain.whole_copy_ok() && !plain.has_redacted());
    }

    #[test]
    fn a_mark_in_a_draft_binds_too() {
        let mut f = form("f1", "bina", &[item("Sorumlu", false)], None, true, false);
        f.draft = serde_json::from_value(json!({ "revision": 2, "items": [item("Malik", true)] }))
            .unwrap();
        let r = reach(&[f], Caller::reader());
        assert_eq!(r.level("bina", "Malik"), Level::Redacted);
        assert_eq!(r.redacted_in("bina"), vec!["Malik".to_owned()]);
    }

    #[test]
    fn records_reach_a_veri_girisi_by_the_live_forms_scopes_and_a_closed_layer_reaches_nothing() {
        let o = Object::default();
        let scoped = reach(
            &[form(
                "f1",
                "bina",
                &[item("Sorumlu", false)],
                Some("Sorumlu = @kullanici_eposta"),
                true,
                false,
            )],
            Caller::through_forms(),
        );
        assert!(scoped.narrows_rows());
        assert_eq!(scoped.limited_layers(), vec!["bina".to_owned()]);
        assert!(scoped.sees("bina", &attrs(&[("Sorumlu", "ali@k.test")]), &o));
        assert!(!scoped.sees("bina", &attrs(&[("Sorumlu", "ayse@k.test")]), &o));
        assert!(scoped.sees("yol", &attrs(&[]), &o), "a layer no form binds");
        // Everyone else reads every record.
        let reader = reach(
            &[form(
                "f1",
                "bina",
                &[item("Sorumlu", false)],
                Some("Sorumlu = @kullanici_eposta"),
                true,
                false,
            )],
            Caller::reader(),
        );
        assert!(!reader.narrows_rows() && reader.limited_layers().is_empty());
        assert!(reader.sees("bina", &attrs(&[("Sorumlu", "ayse@k.test")]), &o));
        // The only form is withdrawn: the layer is closed, whatever its scope was.
        let closed = reach(
            &[form(
                "f1",
                "bina",
                &[item("Sorumlu", false)],
                Some("Sorumlu = @kullanici_eposta"),
                true,
                true,
            )],
            Caller::through_forms(),
        );
        assert_eq!(closed.limited_layers(), vec!["bina".to_owned()]);
        assert!(!closed.sees("bina", &attrs(&[("Sorumlu", "ali@k.test")]), &o));
        assert_eq!(closed.level("bina", "Sorumlu"), Level::Hidden);
        // A draft never published binds nothing.
        let draft = reach(
            &[form(
                "f1",
                "bina",
                &[item("Sorumlu", false)],
                Some("Kat > 9"),
                false,
                false,
            )],
            Caller::through_forms(),
        );
        assert!(!draft.narrows_rows());
        assert!(draft.sees("bina", &attrs(&[]), &o));
        // A live form beside a withdrawn one keeps the layer open through the live one's scope.
        let both = reach(
            &[
                form(
                    "f1",
                    "bina",
                    &[item("Sorumlu", false)],
                    Some("Kat > 3"),
                    true,
                    true,
                ),
                form(
                    "f2",
                    "bina",
                    &[item("Kat", false)],
                    Some("Kat > 5"),
                    true,
                    false,
                ),
            ],
            Caller::through_forms(),
        );
        assert!(both.sees("bina", &attrs(&[("Kat", "6")]), &o));
        assert!(!both.sees("bina", &attrs(&[("Kat", "4")]), &o));
        assert_eq!(both.level("bina", "Kat"), Level::Open);
        assert_eq!(
            both.level("bina", "Sorumlu"),
            Level::Hidden,
            "a withdrawn form shows nothing"
        );
    }

    #[test]
    fn the_stamp_changes_with_every_decision_and_with_nothing_else() {
        let base = || form("f1", "bina", &[item("Sorumlu", false)], None, true, false);
        let a = reach(&[base()], Caller::reader()).stamp();
        assert_eq!(a, reach(&[base()], Caller::reader()).stamp());
        assert_ne!(a, reach(&[base()], Caller::whole()).stamp(), "who");
        assert_ne!(
            a,
            reach(
                &[form(
                    "f1",
                    "bina",
                    &[item("Sorumlu", true)],
                    None,
                    true,
                    false
                )],
                Caller::reader()
            )
            .stamp(),
            "a mark"
        );
        assert_ne!(
            reach(&[base()], Caller::through_forms()).stamp(),
            reach(
                &[form("f1", "bina", &[item("Kat", false)], None, true, false)],
                Caller::through_forms()
            )
            .stamp(),
            "what is shown"
        );
        assert_ne!(
            a,
            reach(
                &[form(
                    "f1",
                    "bina",
                    &[item("Sorumlu", false)],
                    Some("Kat > 1"),
                    true,
                    false
                )],
                Caller::reader()
            )
            .stamp(),
            "a scope"
        );
        // A form's name or the order of unrelated forms does not change what is reached.
        let mut renamed = base();
        renamed.name = "Başka ad".into();
        assert_eq!(a, reach(&[renamed], Caller::reader()).stamp());
    }
}
