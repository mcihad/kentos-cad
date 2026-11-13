//! JSON that keeps its order (docs/adr/8056 §4). A provider's own items go back to it exactly as they came (the research
//! note's “Ortak iç konuşma modeli”): a tool's input as the model wrote it, a thought signature on the part it came on.
//! `serde_json::Value` sorts an object's keys, so a request body is built here instead: objects keep their order, and a
//! piece of JSON text kept from an answer ([`Json::Raw`]) is written as it is.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Number, Value};

/// A JSON value in order, or JSON text taken as it is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
    /// JSON text kept from an answer, written unchanged. Never read from text.
    Raw(String),
}

impl Json {
    /// The value as JSON text, in order.
    pub(crate) fn text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Number(n) => out.push_str(&n.to_string()),
            Json::String(s) => write_string(s, out),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(pairs) => {
                out.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
            Json::Raw(text) => out.push_str(text),
        }
    }

    /// An object's member.
    pub(crate) fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// An array's items; none for anything else.
    pub(crate) fn items(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            _ => &[],
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    // A string's JSON text never fails to build.
    out.push_str(&Value::String(s.to_owned()).to_string());
}

impl From<Value> for Json {
    fn from(v: Value) -> Json {
        match v {
            Value::Null => Json::Null,
            Value::Bool(b) => Json::Bool(b),
            Value::Number(n) => Json::Number(n),
            Value::String(s) => Json::String(s),
            Value::Array(items) => Json::Array(items.into_iter().map(Json::from).collect()),
            Value::Object(map) => {
                Json::Object(map.into_iter().map(|(k, v)| (k, Json::from(v))).collect())
            }
        }
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::String(s.to_owned())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::String(s)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}

impl From<u64> for Json {
    fn from(n: u64) -> Json {
        Json::Number(n.into())
    }
}

impl From<Vec<Json>> for Json {
    fn from(items: Vec<Json>) -> Json {
        Json::Array(items)
    }
}

impl From<Obj> for Json {
    fn from(o: Obj) -> Json {
        Json::Object(o.0)
    }
}

/// An object built member by member, in the order written.
#[derive(Clone, Debug, Default)]
pub(crate) struct Obj(Vec<(String, Json)>);

impl Obj {
    pub(crate) fn new() -> Obj {
        Obj(Vec::new())
    }

    pub(crate) fn put(mut self, key: &str, value: impl Into<Json>) -> Obj {
        self.0.push((key.to_owned(), value.into()));
        self
    }

    /// The member only when `value` is some.
    pub(crate) fn put_some(self, key: &str, value: Option<impl Into<Json>>) -> Obj {
        match value {
            Some(v) => self.put(key, v),
            None => self,
        }
    }

    pub(crate) fn text(self) -> String {
        Json::from(self).text()
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Json, D::Error> {
        deserializer.deserialize_any(InOrder)
    }
}

/// Reads JSON keeping an object's members in the order they came.
struct InOrder;

impl<'de> Visitor<'de> for InOrder {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, b: bool) -> Result<Json, E> {
        Ok(Json::Bool(b))
    }

    fn visit_i64<E: de::Error>(self, n: i64) -> Result<Json, E> {
        Ok(Json::Number(n.into()))
    }

    fn visit_u64<E: de::Error>(self, n: u64) -> Result<Json, E> {
        Ok(Json::Number(n.into()))
    }

    fn visit_f64<E: de::Error>(self, n: f64) -> Result<Json, E> {
        Ok(Number::from_f64(n).map_or(Json::Null, Json::Number))
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Json, E> {
        Ok(Json::String(s.to_owned()))
    }

    fn visit_string<E: de::Error>(self, s: String) -> Result<Json, E> {
        Ok(Json::String(s))
    }

    fn visit_none<E: de::Error>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element::<Json>()? {
            items.push(item);
        }
        Ok(Json::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut pairs = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Json>()? {
            pairs.push((key, value));
        }
        Ok(Json::Object(pairs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_read_back_keeps_the_order_and_raw_text_goes_out_unchanged() {
        let text =
            r#"{"mode":"plan","command":"cad.lines.create","n":[1,-2,2.5,null,true],"s":"ğ\"\n"}"#;
        let j: Json = serde_json::from_str(text).unwrap();
        assert_eq!(j.text(), text);
        assert_eq!(
            j.get("command"),
            Some(&Json::String("cad.lines.create".into()))
        );
        assert_eq!(j.get("n").unwrap().items().len(), 5);
        let body = Obj::new()
            .put("model", "m")
            .put("input", Json::Raw(r#"{"z": 1, "a": 2}"#.into()))
            .put_some("system", None::<&str>)
            .put("stream", true)
            .text();
        assert_eq!(
            body,
            r#"{"model":"m","input":{"z": 1, "a": 2},"stream":true}"#
        );
        // What serde_json would have sorted stays as built.
        assert_eq!(
            Obj::new().put("b", 1u64).put("a", 2u64).text(),
            r#"{"b":1,"a":2}"#
        );
    }
}
