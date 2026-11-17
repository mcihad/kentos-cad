//! A panel's file picker at Kapı 1 (docs/adr/8057 §41, §44): the kinds it offers, and where a file is to be written
//! (`save`, `defaultName`).

use serde_json::{Map, Value};

/// File picker `c`'s problems, each said at its place under `at` by `add`.
pub(super) fn check(c: &Map<String, Value>, at: &str, add: &mut impl FnMut(String, String)) {
    match c.get("filters") {
        None => {}
        Some(Value::Array(filters)) if filters.len() <= 10 => {
            for (j, f) in filters.iter().enumerate() {
                let name = f.get("name").and_then(Value::as_str).map(str::trim);
                let extensions = f.get("extensions").and_then(Value::as_array);
                let fits = name.is_some_and(|n| !n.is_empty() && n.chars().count() <= 40)
                    && extensions.is_some_and(|e| {
                        !e.is_empty()
                            && e.iter().all(|x| {
                                x.as_str().is_some_and(|x| {
                                    (1..=10).contains(&x.len())
                                        && x.bytes()
                                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                                })
                            })
                    });
                if !fits {
                    add(
                        format!("{at}/filters/{j}"),
                        "Dosya türü bir ad (en çok 40 karakter) ve noktasız uzantılar ister, ör. \
                         {\"name\": \"CSV\", \"extensions\": [\"csv\"]}."
                            .to_owned(),
                    );
                }
            }
        }
        Some(_) => add(
            format!("{at}/filters"),
            "filters en çok 10 dosya türünün listesi olmalı.".to_owned(),
        ),
    }
    // A save picker's offered name: a file's name, and only where a file is to be written (§44).
    let saves = c.get("save") == Some(&Value::Bool(true));
    if c.get("save").is_some_and(|s| !s.is_boolean()) {
        add(
            format!("{at}/save"),
            "save evet ya da hayır (true/false) olmalı.".to_owned(),
        );
    }
    match c.get("defaultName") {
        None => {}
        Some(_) if !saves => add(
            format!("{at}/defaultName"),
            "defaultName yalnız kaydetme yerinde (save: true) olur: okunacak dosyanın önerilen adı olmaz."
                .to_owned(),
        ),
        Some(Value::String(name))
            if !name.trim().is_empty()
                && name.chars().count() <= 120
                && !name.contains(['/', '\\', '\n', '\r']) => {}
        Some(_) => add(
            format!("{at}/defaultName"),
            "defaultName bir dosya adı olmalı (klasörsüz, en çok 120 karakter), ör. \"parseller.csv\"."
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    /// A save picker's offered name is a file's name, and only where a file is to be written; `save` is yes or no;
    /// kinds need a name and extensions without dots.
    #[test]
    fn a_save_pickers_name_is_a_files_and_only_where_one_is_written() {
        let said = |c: serde_json::Value| {
            let mut out: Vec<String> = Vec::new();
            super::check(c.as_object().unwrap(), "/body/0", &mut |at, _| out.push(at));
            out
        };
        let picker = |extra: serde_json::Value| {
            let mut c = json!({ "type": "filePicker", "id": "yer", "label": "Yer",
                                "filters": [{ "name": "CSV", "extensions": ["csv"] }] });
            c.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            c
        };
        assert!(
            said(picker(
                json!({ "save": true, "defaultName": "parseller.csv" })
            ))
            .is_empty()
        );
        assert_eq!(
            said(picker(json!({ "defaultName": "parseller.csv" }))),
            ["/body/0/defaultName"]
        );
        assert_eq!(
            said(picker(
                json!({ "save": true, "defaultName": "../parseller.csv" })
            )),
            ["/body/0/defaultName"]
        );
        assert_eq!(said(picker(json!({ "save": "evet" }))), ["/body/0/save"]);
        assert_eq!(
            said(picker(
                json!({ "filters": [{ "name": "CSV", "extensions": [".csv"] }] })
            )),
            ["/body/0/filters/0"]
        );
    }
}
