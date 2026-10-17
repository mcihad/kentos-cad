//! The form's published JSON Schema (docs/adr/0434): a project's form (`ProjectForm`, its draft and
//! releases) as the contract writes it, for tools outside KentOS. Written to
//! `catalog/v1/forms/form-v1.schema.json`; a test keeps the file current.

/// The format a form is exchanged in, and its version: what the schema's `$id` names.
pub const FORM_FORMAT: &str = "kentos.form";
pub const FORM_FORMAT_VERSION: u32 = 1;

/// The schema of a project's form, version 1.
#[cfg(feature = "schema")]
pub fn form_schema() -> serde_json::Value {
    let mut v = schemars::schema_for!(crate::ProjectForm).to_value();
    if let Some(o) = v.as_object_mut() {
        o.insert(
            "$id".into(),
            format!("https://kentos.dev/schemas/{FORM_FORMAT}/v{FORM_FORMAT_VERSION}.json").into(),
        );
        o.insert("title".into(), "KentOS formu, sürüm 1".into());
        o.insert(
            "description".into(),
            "Projenin bir formu: kimliği, adı, katmanı, taslağı ve yayınları (docs/adr/0401, 0434). Bilinmeyen alan reddedilir; \
             yayının özeti gövdesinin kanonik JSON'unun SHA-256'sıdır."
                .into(),
        );
    }
    v
}

#[cfg(all(test, feature = "schema"))]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn schema_file() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../catalog/v1/forms/form-v1.schema.json")
    }

    #[test]
    fn the_form_schema_file_is_current() {
        let text = format!(
            "{}\n",
            serde_json::to_string_pretty(&form_schema()).expect("the schema serializes")
        );
        let path = schema_file();
        if std::env::var("KENTOS_WRITE_CATALOG").as_deref() == Ok("1") {
            std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder");
            std::fs::write(&path, &text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            return;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            on_disk == text,
            "{} is out of date: run `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts form_schema`, read the diff, commit it",
            path.display()
        );
    }
}
