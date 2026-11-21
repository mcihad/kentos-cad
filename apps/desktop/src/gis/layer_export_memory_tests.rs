//! What exporting a million-row data layer to a file holds in memory (docs/adr/9112 §5): the streamed export
//! (`akis`) against the whole-layer way it replaced (`butun`: every row made an object, kept, then the formats core's
//! whole writer, as the export did before). Run by hand, one way at a time, each in its own process so its peak is its
//! own:
//!
//! ```text
//! KENTOS_DATA_PG=postgresql://user@127.0.0.1:55432/db PGPASSWORD=… KENTOS_OLCUM_YOL=akis KENTOS_OLCUM_BICIM=geojson \
//!   KENTOS_OLCUM_KLASOR=/a/folder/with/room /usr/bin/time -v <the desktop's test binary> \
//!   layer_export_memory_tests --ignored
//! ```
//!
//! The table is `public.perf_parseller` (a million parcels in EPSG:5256).

use std::path::PathBuf;
use std::time::Instant;

use kentos_native_application::ExecutionContext;

const TABLE: &str = "public.perf_parseller";

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

#[test]
#[ignore = "elle koşulur: bir milyon satırlık PostGIS tablosu ve bir klasör ister"]
fn a_million_rows_exported_streamed_or_whole() {
    let (Some(address), Some(folder)) = (env("KENTOS_DATA_PG"), env("KENTOS_OLCUM_KLASOR")) else {
        eprintln!("KENTOS_DATA_PG ya da KENTOS_OLCUM_KLASOR yok: ölçüm atlandı");
        return;
    };
    let way = env("KENTOS_OLCUM_YOL").unwrap_or_else(|| "akis".to_owned());
    let format = env("KENTOS_OLCUM_BICIM").unwrap_or_else(|| "geojson".to_owned());
    let folder = PathBuf::from(folder);
    std::fs::create_dir_all(&folder).expect("the folder");
    let ext = match format.as_str() {
        "shapefile" => "shp",
        other => other,
    };
    let path = folder.join(format!("parseller-{way}.{ext}"));
    crate::data_layers::register();
    let described = crate::data_layers::describe(&address, TABLE).expect("described");
    let mut app = crate::files_testing::app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.srid = 5256;
    doc.model.set_settings(settings);
    let added = app.add_data_layers(vec![described]).expect("added");
    let doc = app.document.as_mut().expect("a drawing");
    let node = doc.model.layers().get(&added[0]).cloned().expect("the layer");
    let started = Instant::now();
    let written = if way == "akis" {
        let input = serde_json::from_value(serde_json::json!({
            "layer": node.id, "format": format, "path": path.display().to_string(), "existing": "overwriteFile"
        }))
        .expect("an input");
        match kentos_native_application::layer_export::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            kentos_commands::CommandResult::Completed { output, .. } => output.written,
            other => panic!("not exported: {other:?}"),
        }
    } else {
        whole(&address, &node, &format, &path, doc.model.settings())
    };
    eprintln!("ölçüm {way} {format}: {written} nesne, {:.1} s", started.elapsed().as_secs_f64());
}

/// The way before: every row an object, all kept, then the whole writer into one buffer and the file.
fn whole(
    address: &str,
    node: &kentos_commands::LayerNode,
    format: &str,
    path: &std::path::Path,
    settings: &kentos_commands::ProjectSettings,
) -> u64 {
    let source = kentos_data::registry().source(address).expect("the source");
    let fields: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let request = kentos_data::Request { columns: Some(fields.clone()), geometry: true, ..kentos_data::Request::default() };
    let mut objects = Vec::new();
    source
        .features(TABLE, &request, &kentos_data::Cancel::new(), &mut |part| {
            for f in part {
                let attrs: serde_json::Map<String, serde_json::Value> = fields
                    .iter()
                    .zip(&f.values)
                    .filter_map(|(n, v)| {
                        kentos_native_application::geopackage::attribute_text(v).map(|t| (n.clone(), serde_json::Value::String(t)))
                    })
                    .collect();
                let shape = f.shape().cloned().unwrap_or(kentos_data::Geometry::Point(None));
                let id = u32::try_from(objects.len() + 1).unwrap_or(u32::MAX);
                if let Some(e) = kentos_native_application::geopackage::object_of(&shape, &node.id, id, attrs) {
                    objects.push(e);
                }
            }
            true
        })
        .expect("read");
    let count = objects.len() as u64;
    let system = kentos_project::systems::own(settings).and_then(|n| n.system).expect("a system");
    let choices = kentos_project::systems::choices(settings);
    match format {
        "geojson" => {
            let input = kentos_commands::GeoJsonWriteInput {
                entities: objects,
                layers: vec![kentos_commands::GeoJsonLayer { id: node.id.clone(), name: node.name.clone(), fields: node.fields.clone() }],
                srid: settings.srid,
                name: node.name.clone(),
                blocks: Vec::new(),
            };
            let (bytes, _) = kentos_formats::geojson::write_wgs84(&input, &system, &choices);
            std::fs::write(path, bytes).expect("written");
        }
        "kml" => {
            let input = kentos_commands::KmlWriteInput {
                entities: objects,
                layers: vec![kentos_commands::KmlLayer {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    path: Vec::new(),
                    color: node.style.color.clone(),
                    line_weight: None,
                }],
                name: node.name.clone(),
                blocks: Vec::new(),
                kmz: false,
            };
            let (bytes, _) = kentos_formats::kml::write::write(&input, &system, &choices);
            std::fs::write(path, bytes).expect("written");
        }
        _ => {
            use kentos_formats::shp::write as shp;
            let refs: Vec<&kentos_commands::Entity> = objects.iter().collect();
            let input = shp::Input {
                layers: vec![shp::Layer { id: &node.id, name: "parseller-butun", fields: &node.fields, objects: refs }],
                blocks: &[],
                prj: None,
                encoding: shp::DbfEncoding::Utf8,
            };
            let out = shp::write(&input).expect("written");
            let folder = path.parent().expect("a folder");
            for set in &out.sets {
                for (ext, bytes) in &set.files {
                    std::fs::write(folder.join(format!("{}.{ext}", set.name)), bytes).expect("written");
                }
            }
        }
    }
    count
}
