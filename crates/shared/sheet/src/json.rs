//! The one JSON merge of item patches (`SetItemProps`) and presets: objects merge key by key,
//! everything else replaces. A tagged union whose `type` changes is replaced whole: its old
//! variant's fields mean nothing to the new one.

use serde_json::Value;

fn type_changes(target: &Value, patch: &Value) -> bool {
    match (target.get("type"), patch.get("type")) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    }
}

/// `patch` merged into `target`.
pub fn merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                match t.get_mut(k) {
                    Some(existing)
                        if existing.is_object() && v.is_object() && !type_changes(existing, v) =>
                    {
                        merge(existing, v)
                    }
                    _ => {
                        t.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (t, p) => *t = p.clone(),
    }
}

/// The patch that takes `old` back where `patch` changes it (`null` where `old` had no value).
pub fn reverse(old: &Value, patch: &Value) -> Value {
    match patch {
        Value::Object(p) => {
            let mut r = serde_json::Map::new();
            for (k, v) in p {
                let back = match (old.get(k), v) {
                    (Some(o @ Value::Object(_)), Value::Object(_)) if !type_changes(o, v) => {
                        reverse(o, v)
                    }
                    (Some(o), _) => o.clone(),
                    (None, _) => Value::Null,
                };
                r.insert(k.clone(), back);
            }
            Value::Object(r)
        }
        _ => old.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_changed_variant_is_replaced_and_comes_back_whole() {
        let old = json!({ "view": { "type": "fixed", "scale": 1000, "rotation": 0 }, "x": 1 });
        let patch = json!({ "view": { "type": "atlas", "policy": { "type": "fit" } } });
        let mut t = old.clone();
        merge(&mut t, &patch);
        assert_eq!(
            t,
            json!({ "view": { "type": "atlas", "policy": { "type": "fit" } }, "x": 1 })
        );
        let back = reverse(&old, &patch);
        merge(&mut t, &back);
        assert_eq!(t, old);
        let same = json!({ "view": { "type": "fixed", "scale": 500 } });
        let mut t = old.clone();
        merge(&mut t, &same);
        assert_eq!(t["view"]["rotation"], 0);
        merge(&mut t, &reverse(&old, &same));
        assert_eq!(t, old);
    }
}
