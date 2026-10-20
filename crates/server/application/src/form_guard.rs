//! The forms' guard on the server (docs/adr/0411): which permissions a settings change needs, what a
//! Veri girişi (`forms.use` without `feature.write`) may write, and a form's rules on every write to a
//! layer whose form enforces them. Pure: the caller reads the stored settings, objects and other
//! layers, and turns an answer into its refusal.

use std::collections::BTreeMap;

use kentos_contracts::{
    Entity, FormBody, FormIssue, FormItem, FormSeverity, LayerField, LayerNode, ProjectForm,
    ProjectPermission, ProjectSettings, forms_problem, relations_problem,
};
use kentos_expression::{Unit, Variable};
use kentos_forms::reach::{Caller, Reach};
use kentos_forms::{OtherLayer, Prepared, World};

/// The permissions a settings change from `before` to `after` needs besides `project.edit`: a
/// release added, changed or removed `forms.publish`; anything else of the forms, or the relations,
/// `forms.design` (docs/adr/0411 §3).
pub fn settings_needs(before: &ProjectSettings, after: &ProjectSettings) -> Vec<ProjectPermission> {
    let mut needs = Vec::new();
    let releases = |forms: &[ProjectForm]| {
        forms
            .iter()
            .map(|f| (f.id.clone(), serde_json::to_value(&f.releases).ok()))
            .collect::<BTreeMap<_, _>>()
    };
    let designs = |forms: &[ProjectForm]| {
        forms
            .iter()
            .map(|f| {
                let mut f = f.clone();
                f.releases.clear();
                (f.id.clone(), serde_json::to_value(&f).ok())
            })
            .collect::<BTreeMap<_, _>>()
    };
    let published = releases(&before.forms)
        .into_iter()
        .filter(|(_, r)| {
            r.as_ref()
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty())
        })
        .collect::<BTreeMap<_, _>>();
    let published_after = releases(&after.forms)
        .into_iter()
        .filter(|(_, r)| {
            r.as_ref()
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty())
        })
        .collect::<BTreeMap<_, _>>();
    if published != published_after {
        needs.push(ProjectPermission::FormsPublish);
    }
    let relations = |s: &ProjectSettings| serde_json::to_value(&s.relations).ok();
    if designs(&before.forms) != designs(&after.forms) || relations(before) != relations(after) {
        needs.push(ProjectPermission::FormsDesign);
    }
    needs
}

/// Why the change from `before` to `after` alters a published version (docs/adr/0411 §3): a version
/// kept is kept as it was, a new one comes after the last, and only the oldest ones may go (a form
/// holds `MOST_FORM_RELEASES`); none when it keeps that.
pub fn releases_problem(before: &ProjectSettings, after: &ProjectSettings) -> Option<String> {
    for old in &before.forms {
        let Some(new) = after.forms.iter().find(|f| f.id == old.id) else {
            continue;
        };
        let last = old.releases.last().map_or(0, |r| r.version);
        let same = |a: &kentos_contracts::FormRelease| {
            old.releases
                .iter()
                .find(|b| b.version == a.version)
                .map(|b| serde_json::to_value(a).ok() == serde_json::to_value(b).ok())
        };
        let first_kept = new.releases.first().map_or(u32::MAX, |r| r.version);
        // The last published version stays, or a newer one follows it.
        let kept = new.releases.last().map_or(0, |r| r.version) >= last
            && new
                .releases
                .iter()
                .all(|r| same(r).unwrap_or(r.version > last))
            && old.releases.iter().all(|r| {
                new.releases.iter().any(|n| n.version == r.version) || r.version < first_kept
            });
        if !kept {
            return Some(format!(
                "“{}” formunun yayımlanmış sürümleri değiştirilemez ya da silinemez; yeni bir sürüm yayımlayın.",
                old.name
            ));
        }
    }
    None
}

/// Why `after`'s forms or relations break the file format's rules, with the field's path; none
/// when they keep them.
pub fn settings_problem(after: &ProjectSettings) -> Option<(&'static str, String)> {
    if let Some(why) = forms_problem(&after.forms) {
        return Some(("project.settings.forms", why));
    }
    relations_problem(&after.relations).map(|why| ("project.settings.relations", why))
}

/// The layers `body` reads besides its own: its lists' layers and its related items' (docs/adr/0404).
pub fn world_layers(body: &FormBody) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, item) in body.walk() {
        let layer = match item {
            FormItem::Field(f) => f.lookup.as_ref().map(|l| l.layer.clone()),
            FormItem::Related { layer, .. } => Some(layer.clone()),
            _ => None,
        };
        if let Some(l) = layer
            && !out.contains(&l)
        {
            out.push(l);
        }
    }
    out
}

/// The world a form reads: each of `layers` with its name, fields and stored objects (`rows`: a
/// layer id, a persistent id and its attributes).
pub fn world_of(tree: &[LayerNode], rows: &[(String, String, BTreeMap<String, String>)]) -> World {
    let mut world = World::new();
    for (layer, uid, attrs) in rows {
        let entry = world.entry(layer.clone()).or_insert_with(|| {
            let node = find(tree, layer);
            OtherLayer {
                name: node.map(|n| n.name.clone()).unwrap_or_default(),
                fields: node.map(|n| n.fields.clone()).unwrap_or_default(),
                rows: Vec::new(),
                // The server reads no shapes: a spatial list is not narrowed here (docs/adr/0439 §5).
                shapes: Vec::new(),
            }
        });
        entry.rows.push((uid.clone(), attrs.clone()));
    }
    world
}

fn find<'a>(nodes: &'a [LayerNode], id: &str) -> Option<&'a LayerNode> {
    nodes.iter().find_map(|n| {
        if n.id == id {
            Some(n)
        } else {
            find(&n.children, id)
        }
    })
}

/// The one resolver of what `caller` reads and takes out of the project's records (docs/adr/0550),
/// over its forms, its layers and the `@` values it is judged with.
pub fn reach_with(
    settings: &ProjectSettings,
    tree: &[LayerNode],
    variables: &[Variable],
    caller: Caller,
) -> Reach {
    let empty: &[LayerField] = &[];
    Reach::new(
        &settings.forms,
        |layer| find(tree, layer).map_or(empty, |n| n.fields.as_slice()),
        variables,
        Unit::per_metre(settings.unit().per_metre()),
        caller,
    )
}

/// `e` as the caller of `reach` reads it: the attributes its forms hide left out (a Veri girişi's; for
/// anyone else every attribute is read).
pub fn projected(mut e: Entity, reach: &Reach) -> Entity {
    let layer = e.base().layer_id.clone();
    let attrs = reach.readable(&layer, &e.base().attrs);
    e.base_mut().attrs = attrs;
    e
}

/// `e` as a copy of the whole project carries it for the caller of `reach` (docs/adr/0550 §3): the
/// attributes forms mark `redact` cut, unless the caller designs forms.
pub fn cut(mut e: Entity, reach: &Reach) -> Entity {
    let layer = e.base().layer_id.clone();
    let attrs = reach.copy_attrs(&layer, &e.base().attrs);
    e.base_mut().attrs = attrs;
    e
}

/// A Veri girişi's `after` with what it does not read kept from `stored` (docs/adr/0411 §6): the
/// attributes its layer's forms do not show come from the stored copy; the ones they show are the
/// writer's.
pub fn merged(mut after: Entity, stored: &Entity, reach: &Reach) -> Entity {
    let layer = after.base().layer_id.clone();
    let attrs = &mut after.base_mut().attrs;
    attrs.retain(|k, _| reach.level(&layer, k).readable());
    for (k, v) in &stored.base().attrs {
        if !reach.level(&layer, k).readable() {
            attrs.insert(k.clone(), v.clone());
        }
    }
    after
}

/// What a form judges with: the project's variables (the moment's left out, as the document does),
/// its reading unit, its layer tree and the world its forms read.
pub struct Judge<'a> {
    pub settings: &'a ProjectSettings,
    pub tree: &'a [LayerNode],
    pub variables: Vec<Variable>,
    pub unit: Unit,
    pub world: World,
}

impl Judge<'_> {
    fn fields(&self, layer: &str) -> Vec<LayerField> {
        find(self.tree, layer)
            .map(|n| n.fields.clone())
            .unwrap_or_default()
    }

    fn prepared(&self, form: &ProjectForm, body: &FormBody) -> Result<Prepared, String> {
        Prepared::new_with(
            body,
            &self.fields(&form.layer),
            &self.variables,
            self.unit,
            &self.world,
        )
        .map(|p| {
            let node = find(self.tree, &form.layer);
            p.with_subtypes(node.and_then(|n| n.subtypes.as_ref()))
                .with_contingents(node.map_or(&[][..], |n| n.contingents.as_slice()))
                .with_rules(node.map_or(&[][..], |n| n.rules.as_slice()))
        })
        .map_err(|issues| first_error(&issues))
    }

    fn object(&self, e: &Entity, uid: &str) -> kentos_forms::Object {
        let layer = find(self.tree, &e.base().layer_id).map_or("", |n| n.name.as_str());
        let mut object = kentos_forms::object_of(e, layer, 0.0);
        object.uid = Some(uid.to_owned());
        object.attachments = Some(kentos_forms::Attached::of(&e.base().attachments));
        object
    }

    /// A write to `after` (from `before`, none for a new object) on a layer whose forms enforce their
    /// rules (docs/adr/0411 §4, the document's `enforced`): the reason of the first broken rule, with
    /// its form's name, when one is broken. A write that changes none of the layer's fields is not
    /// the forms'.
    pub fn enforced(&self, uid: &str, before: Option<&Entity>, after: &Entity) -> Option<String> {
        let layer = &after.base().layer_id;
        let empty = BTreeMap::new();
        let before_attrs = before.map_or(&empty, |e| &e.base().attrs);
        let after_attrs = &after.base().attrs;
        let fields = self.fields(layer);
        if fields
            .iter()
            .all(|f| before_attrs.get(&f.name) == after_attrs.get(&f.name))
        {
            return None;
        }
        for form in self
            .settings
            .forms
            .iter()
            .filter(|f| f.enforce && f.layer == *layer)
        {
            let Some(release) = form.releases.last() else {
                continue;
            };
            let words = |why: String| {
                format!(
                    "“{}” formunun kuralları: {why} Değeri formla düzenleyin.",
                    form.name
                )
            };
            let prepared = match self.prepared(form, &release.body) {
                Ok(p) => p,
                Err(why) => return Some(words(why)),
            };
            let issues = prepared.enforce(before_attrs, after_attrs, &self.object(after, uid));
            if kentos_forms::has_errors(&issues) {
                return Some(words(first_error(&issues)));
            }
        }
        None
    }

    /// A Veri girişi's change of `before` to `after` (docs/adr/0411 §2): its attributes alone,
    /// written through its layer's published form. Why not, when it may not.
    pub fn entry_write(&self, uid: &str, before: &Entity, after: &Entity) -> Result<(), String> {
        let mut probe = after.clone();
        probe.base_mut().attrs = before.base().attrs.clone();
        probe.base_mut().id = before.base().id;
        let same = serde_json::to_value(&probe).ok() == serde_json::to_value(before).ok();
        if !same {
            return Err(
                "Veri girişi yalnız kayıtların özniteliklerini formla düzenler; nesnenin şekli, katmanı, görünüşü, etiketi ve ekleri değişmez."
                    .into(),
            );
        }
        let layer = &before.base().layer_id;
        let forms: Vec<&ProjectForm> = self
            .settings
            .forms
            .iter()
            .filter(|f| f.layer == *layer && !f.releases.is_empty())
            .collect();
        let Some(form) = forms.iter().find(|f| f.enforce).or(forms.first()) else {
            return Err(
                "Bu katmanın yayımlanmış formu yok; Veri girişi kayıtları yalnız yayımlanmış formla düzenler."
                    .into(),
            );
        };
        let fields = self.fields(layer);
        let (b, a) = (&before.base().attrs, &after.base().attrs);
        let keys: std::collections::BTreeSet<&String> = b.keys().chain(a.keys()).collect();
        let mut values = Vec::new();
        for key in keys {
            if b.get(key) == a.get(key) {
                continue;
            }
            match fields.iter().find(|f| f.name == **key) {
                Some(f) => {
                    values.push((f.key().to_owned(), a.get(key).cloned().unwrap_or_default()))
                }
                None => {
                    return Err(format!(
                        "“{key}” katmanın alanı değil; Veri girişi yalnız formun alanlarını yazar."
                    ));
                }
            }
        }
        let release = form.releases.last().expect("a published form");
        let prepared = self.prepared(form, &release.body)?;
        prepared
            .apply(b, &values, &self.object(after, uid), false)
            .map(|_| ())
            .map_err(|issues| format!("“{}” formu: {}", form.name, first_error(&issues)))
    }
}

/// The first error's message, else the first issue's.
fn first_error(issues: &[FormIssue]) -> String {
    issues
        .iter()
        .find(|i| i.severity == FormSeverity::Error)
        .or(issues.first())
        .map_or_else(String::new, |i| i.message.clone())
}

/// The `@` values a project's forms judge with on the server for `actor`: the project's variables
/// and the built-in ones, the signed-in person's name and e-mail (docs/adr/0539), the moment's left
/// out as the document does.
pub fn variables_for(
    name: &str,
    settings: &ProjectSettings,
    actor: &crate::identity::Actor,
) -> Vec<Variable> {
    let mut variables = kentos_project::variables::expression_variables_of(
        name,
        settings,
        0.0,
        &actor.display_name,
        actor.email.as_deref().unwrap_or(""),
    );
    variables.retain(|v| !kentos_forms::MOMENT.contains(&v.name.as_str()));
    variables
}

/// Whether a stored object is within the reach of the caller of `reach` (docs/adr/0539, 0550).
pub fn in_reach(reach: &Reach, tree: &[LayerNode], e: &Entity, uid: &str) -> bool {
    let layer = &e.base().layer_id;
    let name = find(tree, layer).map_or("", |n| n.name.as_str());
    let mut object = kentos_forms::object_of(e, name, 0.0);
    object.uid = Some(uid.to_owned());
    reach.sees(layer, &e.base().attrs, &object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings(forms: serde_json::Value) -> ProjectSettings {
        let mut s: ProjectSettings = serde_json::from_value(json!({
            "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2",
            "angleUnit": "grad", "plotScale": 1000.0, "workspace": "gis"
        }))
        .expect("settings");
        s.forms = serde_json::from_value(forms).expect("forms");
        s
    }

    fn form(enforce: bool, released: bool) -> serde_json::Value {
        let body = json!({ "revision": 1, "items": [
            { "type": "field", "id": "durum", "field": "Durum", "required": true },
            { "type": "field", "id": "kat", "field": "Kat" }
        ]});
        let typed: FormBody = serde_json::from_value(body.clone()).expect("a body");
        let releases = if released {
            json!([{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }])
        } else {
            json!([])
        };
        json!({ "id": "bina", "name": "Bina", "layer": "bina", "draft": body, "releases": releases, "enforce": enforce })
    }

    fn tree() -> Vec<LayerNode> {
        serde_json::from_value(json!([{
            "id": "bina", "name": "Bina", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
            "fields": [
                { "name": "Durum", "kind": "text", "values": [{ "code": "iyi", "label": "İyi" }, { "code": "kotu", "label": "Kötü" }] },
                { "name": "Kat", "kind": "integer", "min": "1" },
                { "name": "Not", "kind": "text" }
            ]
        }]))
        .expect("a tree")
    }

    fn building(attrs: serde_json::Value, x: f64) -> Entity {
        serde_json::from_value(json!({
            "kind": "polygon", "id": 1, "layerId": "bina", "attrs": attrs,
            "pts": [{ "x": 0.0, "y": 0.0 }, { "x": x, "y": 0.0 }, { "x": x, "y": 8.0 }]
        }))
        .expect("a building")
    }

    fn judge<'a>(s: &'a ProjectSettings, t: &'a [LayerNode]) -> Judge<'a> {
        Judge {
            settings: s,
            tree: t,
            variables: Vec::new(),
            unit: Unit::METRE,
            world: World::new(),
        }
    }

    /// §3: a release needs forms.publish; the draft, enforcing or a new form forms.design.
    #[test]
    fn a_settings_change_needs_the_forms_permissions() {
        let none = settings(json!([]));
        let draft = settings(json!([form(false, false)]));
        let published = settings(json!([form(false, true)]));
        let enforced = settings(json!([form(true, true)]));
        assert_eq!(settings_needs(&none, &none), []);
        assert_eq!(
            settings_needs(&none, &draft),
            [ProjectPermission::FormsDesign]
        );
        assert_eq!(
            settings_needs(&draft, &published),
            [ProjectPermission::FormsPublish]
        );
        assert_eq!(
            settings_needs(&published, &enforced),
            [ProjectPermission::FormsDesign]
        );
        assert_eq!(
            settings_needs(&none, &published),
            [
                ProjectPermission::FormsPublish,
                ProjectPermission::FormsDesign
            ]
        );
        assert!(settings_problem(&published).is_none());
        // A release is kept as it was: changing or dropping one is refused; a new one is not.
        assert_eq!(releases_problem(&draft, &published), None);
        assert!(releases_problem(&published, &draft).is_some());
        let mut altered = published.clone();
        altered.forms[0].releases[0].note = Some("sonradan".into());
        assert!(releases_problem(&published, &altered).is_some());
        let mut pruned = published.clone();
        pruned.forms[0].releases[0].version = 2;
        assert_eq!(
            releases_problem(&published, &pruned),
            None,
            "the oldest goes, a new one comes"
        );
        let mut removed = published.clone();
        removed.forms.clear();
        assert_eq!(releases_problem(&published, &removed), None);
    }

    /// §2: a Veri girişi changes the attributes the form shows, through its rules; not a shape, a
    /// field the form does not show, or an attribute that is no field.
    #[test]
    fn a_data_entry_writes_attributes_through_the_form_only() {
        let (s, t) = (settings(json!([form(false, true)])), tree());
        let j = judge(&s, &t);
        let before = building(json!({ "Durum": "iyi", "Kat": "2" }), 10.0);
        let ok = building(json!({ "Durum": "kotu", "Kat": "3" }), 10.0);
        assert_eq!(j.entry_write("u", &before, &ok), Ok(()));
        let moved = building(json!({ "Durum": "iyi", "Kat": "2" }), 12.0);
        assert!(
            j.entry_write("u", &before, &moved)
                .unwrap_err()
                .contains("şekli")
        );
        let hidden = building(json!({ "Durum": "iyi", "Kat": "2", "Not": "x" }), 10.0);
        assert!(
            j.entry_write("u", &before, &hidden)
                .unwrap_err()
                .contains("göstermiyor")
        );
        let stray = building(json!({ "Durum": "iyi", "Kat": "2", "Ek": "x" }), 10.0);
        assert!(
            j.entry_write("u", &before, &stray)
                .unwrap_err()
                .contains("alanı değil")
        );
        let broken = building(json!({ "Durum": "iyi", "Kat": "0" }), 10.0);
        assert!(j.entry_write("u", &before, &broken).is_err());
        let unpublished = settings(json!([form(false, false)]));
        assert!(
            judge(&unpublished, &t)
                .entry_write("u", &before, &ok)
                .unwrap_err()
                .contains("yayımlanmış formu yok")
        );
    }

    /// §6: a Veri girişi reads the fields the published form shows; its write keeps the others.
    #[test]
    fn a_data_entry_reads_the_forms_fields_and_keeps_the_others() {
        let (s, t) = (settings(json!([form(false, true)])), tree());
        let reach = reach_with(&s, &t, &[], Caller::THROUGH_FORMS);
        let stored = building(
            json!({ "Durum": "iyi", "Kat": "2", "Not": "gizli", "Ek": "x" }),
            10.0,
        );
        let read = projected(stored.clone(), &reach);
        assert_eq!(
            read.base().attrs,
            building(json!({ "Durum": "iyi", "Kat": "2" }), 10.0)
                .base()
                .attrs
        );
        let mut written = read.clone();
        written.base_mut().attrs.insert("Kat".into(), "3".into());
        let kept = merged(written, &stored, &reach);
        assert_eq!(
            kept.base().attrs,
            building(
                json!({ "Durum": "iyi", "Kat": "3", "Not": "gizli", "Ek": "x" }),
                10.0
            )
            .base()
            .attrs
        );
        assert_eq!(judge(&s, &t).entry_write("u", &stored, &kept), Ok(()));
        // A draft is no published form: nothing is hidden.
        let draft = reach_with(
            &settings(json!([form(false, false)])),
            &t,
            &[],
            Caller::THROUGH_FORMS,
        );
        assert_eq!(
            projected(stored.clone(), &draft).base().attrs,
            stored.base().attrs
        );
    }

    /// §4: a form that enforces its rules refuses a write that breaks them, from anyone; a write that
    /// changes no field, or a form that does not enforce, is not refused.
    #[test]
    fn an_enforcing_form_guards_every_write() {
        let t = tree();
        let enforced = settings(json!([form(true, true)]));
        let j = judge(&enforced, &t);
        let before = building(json!({ "Durum": "iyi", "Kat": "2" }), 10.0);
        let empty = building(json!({ "Kat": "2" }), 12.0);
        assert!(
            j.enforced("u", Some(&before), &empty)
                .is_some_and(|w| w.starts_with("“Bina” formunun"))
        );
        let reshaped = building(json!({ "Durum": "iyi", "Kat": "2" }), 12.0);
        assert_eq!(j.enforced("u", Some(&before), &reshaped), None);
        assert!(j.enforced("u", None, &empty).is_some());
        let open = settings(json!([form(false, true)]));
        assert_eq!(judge(&open, &t).enforced("u", Some(&before), &empty), None);
    }
}
