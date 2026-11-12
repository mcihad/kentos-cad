//! A bound table's CHECK and unique constraints on the desktop (docs/adr/0755 §6): Alanlar says the list a CHECK gives a
//! field, a field the table keeps unique and the table's constraints with how much KentOS judges each; Formla düzenle
//! shows the list as a choice and a broken constraint on its field. Pictures in `.run/shots/kisit-*`.
//!
//! ```text
//! cargo test -p kentos-desktop --lib constraint_screens -- --ignored --nocapture --test-threads=1
//! ```

use std::path::PathBuf;

use iced::Size;
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;
use kentos_ui::widget::sizable::sizes;

use crate::app::{App, Message};
use crate::form_scenes::designer;
use crate::forms::designer::Event;
use crate::tools_scenes::run;

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

/// The owner's look: the text size, the face and the theme.
fn owners_look(app: &mut App) {
    let _ = app.settings.choose(&[
        ("appearance.textSize", serde_json::Value::from(14)),
        ("appearance.uiFont", serde_json::Value::from("jakarta")),
        ("appearance.theme", serde_json::Value::from("light")),
    ]);
    app.apply_settings();
}

/// The fields of a voucher table bound from PostGIS like the owner's `muhasebe.fis`: the use's list comes from its CHECK.
fn voucher_fields() -> serde_json::Value {
    serde_json::json!([
        { "name": "mali_yil", "kind": "integer", "required": true },
        { "name": "yevmiye_no", "kind": "integer", "required": true },
        { "name": "kod", "kind": "text" },
        { "name": "tur", "kind": "text", "length": 20,
          "values": [{ "code": "butce_disi", "label": "butce_disi" }, { "code": "is", "label": "is" },
                     { "code": "personel", "label": "personel" }],
          "column": { "check": "fis_tur_check" } },
        { "name": "oncelik", "kind": "integer" },
        { "name": "durum", "kind": "text", "length": 1 },
        { "name": "tutar", "kind": "decimal", "scale": 2 },
        { "name": "veri", "kind": "json" },
        { "name": "baslangic", "kind": "date" },
        { "name": "bitis", "kind": "date" }
    ])
}

/// The link of the voucher table: its CHECKs and unique constraints as PostgreSQL gives them.
fn voucher_link() -> serde_json::Value {
    serde_json::json!({
        "id": "kaynak-1", "kind": "postgis",
        "path": "postgresql://muhasebe@veri.belediye.local:5432/kent", "table": "muhasebe.fis", "layer": "fis",
        "revision": "1".repeat(64), "digest": "2".repeat(64), "srid": 5256,
        "checks": [
            { "name": "fis_durum_check", "sql": "(durum = ANY (ARRAY['A'::bpchar, 'P'::bpchar]))",
              "columns": ["durum"], "tableOnly": true },
            { "name": "fis_oncelik", "sql": "((oncelik >= 1) AND (oncelik <= 3))", "columns": ["oncelik"] },
            { "name": "fis_tarih", "sql": "((bitis IS NULL) OR (bitis > baslangic))", "columns": ["baslangic", "bitis"] },
            { "name": "fis_tur_check",
              "sql": "((tur)::text = ANY ((ARRAY['is'::character varying, 'personel'::character varying, 'butce_disi'::character varying])::text[]))",
              "columns": ["tur"] },
            { "name": "fis_tutar", "sql": "(tutar >= 0.01)", "columns": ["tutar"] },
            { "name": "fis_veri_check", "sql": "(jsonb_typeof(veri) = 'array'::text)", "columns": ["veri"] }
        ],
        "uniques": [
            { "name": "fis_yil_no", "columns": ["mali_yil", "yevmiye_no"], "nullsEqual": true },
            { "name": "fis_kod", "columns": ["kod"] }
        ],
        "writable": true
    })
}

/// Two vouchers of the table, the first with a sum the table refuses.
fn vouchers(app: &mut App) {
    let mut o = crate::tools_scenes::Objects::new();
    for (k, (no, kod, tur, tutar)) in [("1", "F-1", "is", "0"), ("2", "F-2", "personel", "50.5")]
        .into_iter()
        .enumerate()
    {
        let x = k as f64 * 50.0;
        let id = o.path(
            "fis",
            &[[x, 0.0], [x + 40.0, 0.0], [x + 40.0, 30.0], [x, 30.0]],
            true,
        );
        o.data(
            id,
            &[
                ("mali_yil", "2026"),
                ("yevmiye_no", no),
                ("kod", kod),
                ("tur", tur),
                ("oncelik", "2"),
                ("durum", "A"),
                ("tutar", tutar),
                ("veri", "[1]"),
                ("baslangic", "2026-01-01"),
                ("bitis", "2026-02-01"),
            ],
            None,
        );
    }
    let (fields, link) = (voucher_fields(), voucher_link());
    crate::tools_scenes::open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut layer = crate::tools_scenes::layer("fis", "Fiş", "#7A4FA3", 0.35);
        layer["fields"] = fields;
        layers.push(layer);
        d["layers"] = serde_json::Value::Array(layers);
        d["activeLayer"] = serde_json::json!("fis");
        d["settings"]["sources"] = serde_json::json!([link]);
    });
}

/// docs/adr/0755 §6: Alanlar's sentences and judgements, the field the table keeps unique, and a form's issue of a
/// broken constraint on its field.
#[test]
fn alanlar_and_the_form_say_the_tables_constraints() {
    let (mut app, _) = App::boot(None);
    vouchers(&mut app);
    let doc = app.document.as_ref().expect("open");
    let link = doc.model.settings().sources[0].clone();
    let fields = doc
        .model
        .layers()
        .get("fis")
        .expect("the layer")
        .fields
        .clone();
    let said = crate::layer_fields::constraints_said(&link, &fields);
    let line = |words: &str| {
        said.iter()
            .find(|(s, _)| s.contains(words))
            .map(|(_, j)| *j)
    };
    assert_eq!(
        line("fis_oncelik: oncelik 1 ile 3 arasında olmalı"),
        Some("KentOS da denetler")
    );
    assert_eq!(
        line("fis_tarih: bitis, baslangic alanından büyük olmalı"),
        Some("KentOS da denetler")
    );
    assert_eq!(
        line("fis_tur_check: tur şunlardan biri olmalı: is, personel, butce_disi"),
        Some("KentOS da denetler")
    );
    assert_eq!(line("fis_veri_check"), Some("KentOS kısmen denetler"));
    assert_eq!(line("fis_durum_check"), Some("Yalnız tablo denetler"));
    assert_eq!(
        line("fis_yil_no: mali_yil, yevmiye_no birlikte tekil, boşlar da aynı sayılır"),
        Some("KentOS da denetler")
    );
    assert!(crate::layer_fields::unique_in_table(&link, "kod"));
    assert!(!crate::layer_fields::unique_in_table(&link, "mali_yil"));
    let tur = fields.iter().find(|f| f.name == "tur").expect("tur");
    assert!(
        crate::layer_fields::column_said(tur.column.as_ref())
            .is_some_and(|s| s.contains("Değerler tablonun kısıtından (fis_tur_check)")),
        "{:?}",
        crate::layer_fields::column_said(tur.column.as_ref())
    );
    // The form's validation: the first voucher's sum breaks fis_tutar, told on its field.
    let doc = app.document.as_mut().expect("open");
    let made = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.forms.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "edit": { "kind": "create", "layer": "fis", "name": "Fiş" } }),
    )
    .expect("runs");
    assert_eq!(made["status"], "completed", "{made}");
    let uid = doc.model.uid(Slot(1)).expect("a uid").to_string();
    let checked = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.forms.validate",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "form": made["output"]["form"]["id"], "draft": true, "uid": uid }),
    )
    .expect("runs");
    let issues = checked["output"]["issues"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        issues.iter().any(|i| i["code"] == "form_layer_rule"
            && i["field"] == "tutar"
            && i["message"] == "Tablonun “fis_tutar” kısıtı: tutar en az 0.01 olmalı."),
        "{checked}"
    );
}

/// Alanlar on the use (its list from the table's CHECK) and on the code (unique in the table), with the table's
/// constraints; Formla düzenle on the voucher whose sum the table refuses. Pictures in `.run/shots/kisit-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn constraint_screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder");
    for (w, h) in [(1100.0, 650.0), (1440.0, 900.0)] {
        sizes::load(Default::default());
        let (mut app, _) = App::boot(None);
        owners_look(&mut app);
        let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        vouchers(&mut app);
        let shot = |snapshot: &mut Snapshot, app: &App, step: &str| {
            let file = out.join(format!("kisit-{step}-{w}x{h}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("a picture");
            println!("{}", file.display());
        };
        run(&mut app, "layer.fields");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Choose(3)));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "1-alanlar-liste");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Choose(2)));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "2-alanlar-tekil");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Cancel));
        snapshot.settle(&mut app, App::view, &mut update);
        run(&mut app, "data.forms");
        designer(&mut app, Event::NewForm);
        designer(&mut app, Event::NewLayer("fis".into()));
        designer(&mut app, Event::NewName("Fiş".into()));
        designer(&mut app, Event::Create);
        designer(&mut app, Event::Save);
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Forms(Event::Cancel));
        app.selection.set([Slot(1)]);
        run(&mut app, "data.formEdit");
        snapshot.settle(&mut app, App::view, &mut update);
        // The sum typed again: the table's constraint is told under its field.
        let _ = app.update(Message::FormEntry(crate::forms::entry::Event::Value(
            "tutar".into(),
            "-1".into(),
        )));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "3-formla-duzenle");
        let _ = app.update(Message::FormEntry(crate::forms::entry::Event::Cancel));
        snapshot.settle(&mut app, App::view, &mut update);
    }
    sizes::load(Default::default());
}
