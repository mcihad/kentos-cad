//! A query read as a live data layer (docs/adr/9113 §3) over a GeoPackage: added read-only and named after its table,
//! its rows read from the source as the table's are (the attribute table counts the query's, not the table's), its key
//! checked to tell the rows apart, editing refused.

use kentos_data::QueryTable;

#[test]
fn a_query_is_added_as_a_live_read_only_layer_and_its_key_is_checked() {
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("sorgu-katmani", 40);
    let address = path.display().to_string();
    let parks = QueryTable {
        sql: "SELECT fid, ada, kullanim, geom FROM parseller WHERE kullanim = 'Park'".into(),
        key: "fid".into(),
        geometry: Some("geom".into()),
    };
    let described = crate::data_layers::describe_query(&address, &parks, None).expect("described");
    assert_eq!(described.name, "Sorgu: parseller");
    assert!(described.data.read_only, "a query is read-only");
    assert_eq!(described.data.table, parks.text());
    let fields: Vec<&str> = described.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(fields, ["ada", "kullanim"]);
    assert_eq!(described.data.place_words().rsplit(" › ").next(), Some("SQL sorgusu"));

    // A key two rows share is refused, with how many values it has.
    let shared = QueryTable { key: "ada".into(), ..parks.clone() };
    let why = crate::data_layers::describe_query(&address, &shared, None).expect_err("ada repeats");
    assert!(why.contains("benzersiz değil") && why.contains("20 satırında"), "{why}");

    // Added and named: its rows are the query's, read from the source.
    let mut app = crate::files_testing::app_with_drawing();
    let named = crate::data_layers::describe_query(&address, &parks, Some("Parklar")).expect("described");
    let added = app.add_data_layers(vec![named]).expect("added");
    let node = app.document.as_ref().expect("a drawing").model.layers().get(&added[0]).cloned().expect("the layer");
    assert_eq!(node.name, "Parklar");
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let input = serde_json::from_value(serde_json::json!({ "layer": node.id, "count": true, "limit": 100 })).expect("an input");
    let read = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false).expect("read");
    assert_eq!(read.total, Some(20), "the query's 20 parks, not the table's 40");
    assert!(read.rows.iter().all(|r| r.attrs.get("kullanim").map(String::as_str) == Some("Park")));

    // Editing a query layer is refused.
    let refused = app.document.as_mut().expect("a drawing").model.start_editing(&node.id).expect_err("read-only");
    assert!(refused.0.contains("salt okunur"), "{refused}");
    let _ = std::fs::remove_file(&path);
}
