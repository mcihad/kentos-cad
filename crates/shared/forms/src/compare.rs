//! What changed from one version of a form to another (docs/adr/0427): each item added, removed, moved
//! (under another tab or group, or elsewhere among the siblings both versions share) or changed (its own
//! properties, its items aside), then the form's own `clearHidden` and scenarios. The changes come in
//! the newer version's order, the removed ones after them in the older one's.

use kentos_contracts::{FormBody, FormChange, FormChangeKind, FormItem};
use serde_json::Value;

/// Each item with its parent's id, in the form's order.
fn flat(items: &[FormItem], parent: Option<&str>, out: &mut Vec<(String, Option<String>, Value)>) {
    for i in items {
        let mut own = serde_json::to_value(i).unwrap_or(Value::Null);
        if let Some(map) = own.as_object_mut() {
            map.remove("items");
        }
        out.push((i.id().to_owned(), parent.map(str::to_owned), own));
        flat(i.items(), Some(i.id()), out);
    }
}

/// The properties' names in the words (the form's JSON names stay in `properties`).
const WORDS: [(&str, &str); 28] = [
    ("label", "ad"),
    ("field", "alan"),
    ("help", "yardım"),
    ("placeholder", "yer tutucu"),
    ("widget", "giriş biçimi"),
    ("readOnly", "salt okunur"),
    ("required", "zorunlu"),
    ("visibleWhen", "görünme koşulu"),
    ("requiredWhen", "zorunluluk koşulu"),
    ("readOnlyWhen", "salt okunurluk koşulu"),
    ("default", "varsayılan"),
    ("defaultExpr", "varsayılan ifadesi"),
    ("calculate", "hesap"),
    ("choiceFilter", "seçenek süzgeci"),
    ("unique", "tekil"),
    ("reuseLast", "son değer"),
    ("overridable", "değiştirilebilir"),
    ("suggest", "öneriler"),
    ("lookup", "başka katmandan liste"),
    ("checks", "denetimler"),
    ("columns", "sütunlar"),
    ("collapsed", "kapalı açılır"),
    ("text", "metin"),
    ("layer", "katman"),
    ("view", "görünüm"),
    ("accept", "kabul edilen"),
    ("min", "en az"),
    ("max", "en çok"),
];

fn word(key: &str) -> &str {
    WORDS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(key, |(_, w)| w)
}

/// What an item is called in the words: its label, else its field, else its id.
fn name_of(own: &Value, id: &str) -> String {
    own.get("label")
        .or_else(|| own.get("field"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(id)
        .to_owned()
}

/// The ids of `parent`'s children in `list` that are in `both`, in order.
fn siblings<'a>(
    list: &'a [(String, Option<String>, Value)],
    parent: Option<&str>,
    both: &[&str],
) -> Vec<&'a str> {
    list.iter()
        .filter(|(id, p, _)| p.as_deref() == parent && both.contains(&id.as_str()))
        .map(|(id, _, _)| id.as_str())
        .collect()
}

/// The changes from `old` to `new`.
pub fn compare(old: &FormBody, new: &FormBody) -> Vec<FormChange> {
    let (mut a, mut b) = (Vec::new(), Vec::new());
    flat(&old.items, None, &mut a);
    flat(&new.items, None, &mut b);
    let both: Vec<&str> = b
        .iter()
        .filter(|(id, _, _)| a.iter().any(|(x, _, _)| x == id))
        .map(|(id, _, _)| id.as_str())
        .collect();
    let mut out = Vec::new();
    for (id, parent, own) in &b {
        let name = name_of(own, id);
        let Some((_, was_parent, was)) = a.iter().find(|(x, _, _)| x == id) else {
            out.push(FormChange {
                item: Some(id.clone()),
                kind: FormChangeKind::Added,
                properties: Vec::new(),
                words: format!("“{name}” eklendi."),
            });
            continue;
        };
        let moved = was_parent != parent || {
            let then = siblings(&a, was_parent.as_deref(), &both);
            let now = siblings(&b, parent.as_deref(), &both);
            then.iter().position(|x| x == id) != now.iter().position(|x| x == id)
        };
        if moved {
            out.push(FormChange {
                item: Some(id.clone()),
                kind: FormChangeKind::Moved,
                properties: Vec::new(),
                words: format!("“{name}” taşındı."),
            });
        }
        if was != own {
            let mut keys: Vec<String> = Vec::new();
            for map in [was.as_object(), own.as_object()].into_iter().flatten() {
                for k in map.keys() {
                    if was.get(k) != own.get(k) && !keys.contains(k) {
                        keys.push(k.clone());
                    }
                }
            }
            keys.sort();
            out.push(FormChange {
                item: Some(id.clone()),
                kind: FormChangeKind::Changed,
                words: format!(
                    "“{name}” değişti: {}.",
                    keys.iter().map(|k| word(k)).collect::<Vec<_>>().join(", ")
                ),
                properties: keys,
            });
        }
    }
    for (id, _, own) in &a {
        if !b.iter().any(|(x, _, _)| x == id) {
            out.push(FormChange {
                item: Some(id.clone()),
                kind: FormChangeKind::Removed,
                properties: Vec::new(),
                words: format!("“{}” kaldırıldı.", name_of(own, id)),
            });
        }
    }
    if old.clear_hidden != new.clear_hidden {
        out.push(FormChange {
            item: None,
            kind: FormChangeKind::Changed,
            properties: vec!["clearHidden".into()],
            words: if new.clear_hidden {
                "Gizlenenler silinir.".into()
            } else {
                "Gizlenenler artık silinmez.".into()
            },
        });
    }
    if old.tests != new.tests {
        out.push(FormChange {
            item: None,
            kind: FormChangeKind::Changed,
            properties: vec!["tests".into()],
            words: format!(
                "Senaryolar değişti: {} senaryo, önce {}.",
                new.tests.len(),
                old.tests.len()
            ),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::FormField;

    #[test]
    fn a_change_of_each_kind_is_said_in_order() {
        let field = |id: &str, f: &str| FormItem::Field(FormField::new(id, f));
        let old = FormBody {
            revision: 1,
            items: vec![field("a", "A"), field("b", "B"), field("c", "C")],
            clear_hidden: false,
            tests: Vec::new(),
        };
        let mut changed = FormField::new("b", "B");
        changed.required = true;
        let new = FormBody {
            revision: 2,
            items: vec![field("c", "C"), FormItem::Field(changed), field("d", "D")],
            clear_hidden: true,
            tests: Vec::new(),
        };
        let words: Vec<String> = compare(&old, &new).into_iter().map(|c| c.words).collect();
        assert_eq!(
            words,
            [
                "“C” taşındı.",
                "“B” taşındı.",
                "“B” değişti: zorunlu.",
                "“D” eklendi.",
                "“A” kaldırıldı.",
                "Gizlenenler silinir."
            ]
        );
    }
}
