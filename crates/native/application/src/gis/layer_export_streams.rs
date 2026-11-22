//! The GeoJSON, Shapefile and KML targets of `cad.layers.export`, written as the rows come (docs/adr/9112 §1): each
//! row made an object again, in the system written, and handed to the formats core's stream for its format, so a
//! layer of a million rows never stands whole in memory. GeoJSON and KML are written to `<file>.part` and put in place
//! at the end; a Shapefile's sets are kept beside their place until the end (the stream's own spools). Stopped or
//! failed, nothing of them stays and a file there before is as it was.

use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};

use kentos_commands::{ExportEncoding, ExportFormat, LayerField};
use kentos_data::{NewFeature, Value};
use kentos_formats::blocks::Placing;

use crate::checks::Stop;
use crate::layer_export::{Plan, unreachable};
use crate::layer_export_sinks::{Sink, cell, part_of, put_in_place};

/// A row made an object of the layer again (its geometry none: a point without one, as the writers take it).
fn object_of(r: &NewFeature, layer: &str, id: u32) -> Option<kentos_commands::Entity> {
    let attrs: serde_json::Map<String, serde_json::Value> = r
        .values
        .iter()
        .filter(|(_, v)| !matches!(v, Value::Null))
        .map(|(n, v)| (n.clone(), serde_json::Value::String(cell(v))))
        .collect();
    let shape = r.geometry.clone().unwrap_or(kentos_data::Geometry::Point(None));
    crate::geopackage::object_of(&shape, layer, id, attrs)
}

/// The project's system and its datum choices: what the WGS 84 writers move the rows from.
fn wgs84_of(plan: &Plan<'_>) -> Result<(kentos_geometry_core::crs::System, Vec<kentos_geometry_core::crs::Choice>), Stop> {
    let system = kentos_project::systems::own(plan.settings)
        .and_then(|n| n.system)
        .ok_or_else(|| unreachable("projenin koordinat sistemi WGS 84'e çevrilemiyor"))?;
    Ok((system, kentos_project::systems::choices(plan.settings)))
}

fn folder_made(path: &Path) -> Result<(), Stop> {
    if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
        std::fs::create_dir_all(folder).map_err(unreachable)?;
    }
    Ok(())
}

/// The target for a GeoJSON, Shapefile or KML file.
pub(crate) fn open(plan: &Plan<'_>, path: PathBuf) -> Result<Box<dyn Sink>, Stop> {
    folder_made(&path)?;
    let fields: Vec<LayerField> = plan.fields.iter().map(|f| f.written()).collect();
    let name = plan.input.name.clone().unwrap_or_else(|| plan.node.name.clone());
    let layer = plan.node.id.clone();
    match plan.input.format {
        ExportFormat::Geojson => {
            let project_coordinates = plan.input.options.as_ref().and_then(|o| o.project_coordinates) == Some(true);
            let wgs84 = if project_coordinates || plan.settings.srid == 4326 { None } else { Some(wgs84_of(plan)?) };
            let part = part_of(&path);
            let out = BufWriter::new(File::create(&part).map_err(unreachable)?);
            let stream = kentos_formats::geojson::Stream::begin(
                out,
                &name,
                plan.srid.unwrap_or(plan.settings.srid),
                &[kentos_commands::GeoJsonLayer { id: layer.clone(), name: name.clone(), fields }],
                Placing::new(&[]),
                wgs84,
            );
            Ok(Box::new(GeoJson { stream: Some(stream), part, path, layer, made: 0 }))
        }
        ExportFormat::Kml => {
            let (system, choices) = wgs84_of(plan)?;
            let symbols = crate::layer_export_kml::KmlSymbols::of(plan);
            let part = part_of(&path);
            let out = BufWriter::new(File::create(&part).map_err(unreachable)?);
            let stream = kentos_formats::kml::LayerStream::begin(
                out,
                &name,
                &name,
                &symbols.styles,
                system,
                choices,
                Placing::new(&[]),
            );
            Ok(Box::new(Kml { stream: Some(stream), part, path, layer, made: 0, symbols }))
        }
        _ => {
            let stem = path.file_stem().map_or_else(|| "katman".to_owned(), |s| s.to_string_lossy().into_owned());
            let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
            let encoding = match plan.input.encoding.unwrap_or_default() {
                ExportEncoding::Utf8 => kentos_formats::shp::write::DbfEncoding::Utf8,
                ExportEncoding::Windows1254 => kentos_formats::shp::write::DbfEncoding::Windows1254,
            };
            let stream = kentos_formats::shp::stream::Stream::begin(
                &folder,
                &stem,
                &fields,
                plan.srid.and_then(crate::shapefile::esri),
                encoding,
                Placing::new(&[]),
            );
            Ok(Box::new(Shapefile { stream: Some(stream), layer, made: 0 }))
        }
    }
}

fn written_back(result: (Result<BufWriter<File>, std::io::Error>, kentos_commands::ExportReport)) -> Result<(), String> {
    let mut out = result.0.map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}

struct GeoJson {
    stream: Option<kentos_formats::geojson::Stream<BufWriter<File>>>,
    part: PathBuf,
    path: PathBuf,
    layer: String,
    made: u32,
}

impl Sink for GeoJson {
    fn write(&mut self, rows: &[NewFeature], _objects: &[&kentos_commands::Entity]) -> Result<(), String> {
        let Some(stream) = self.stream.as_mut() else { return Ok(()) };
        for r in rows {
            self.made += 1;
            if let Some(e) = object_of(r, &self.layer, self.made) {
                stream.put(&e);
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<String>, String> {
        let Some(stream) = self.stream.take() else { return Ok(Vec::new()) };
        if let Err(why) = written_back(stream.finish()) {
            let _ = std::fs::remove_file(&self.part);
            return Err(why);
        }
        put_in_place(&self.part, &self.path)?;
        Ok(vec![self.path.display().to_string()])
    }

    fn abandon(mut self: Box<Self>) {
        drop(self.stream.take());
        let _ = std::fs::remove_file(&self.part);
    }
}

struct Kml {
    stream: Option<kentos_formats::kml::LayerStream<BufWriter<File>>>,
    part: PathBuf,
    path: PathBuf,
    layer: String,
    made: u32,
    /// The styles and each placemark's (layer_export_kml.rs, docs/adr/9112 §4).
    symbols: crate::layer_export_kml::KmlSymbols,
}

impl Sink for Kml {
    fn write(&mut self, rows: &[NewFeature], objects: &[&kentos_commands::Entity]) -> Result<(), String> {
        let Some(stream) = self.stream.as_mut() else { return Ok(()) };
        let styles = self.symbols.styles_of(objects);
        for (i, r) in rows.iter().enumerate() {
            self.made += 1;
            if let Some(e) = object_of(r, &self.layer, self.made) {
                stream.put(&e, styles.get(i).copied().flatten());
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<String>, String> {
        let Some(stream) = self.stream.take() else { return Ok(Vec::new()) };
        if let Err(why) = written_back(stream.finish()) {
            let _ = std::fs::remove_file(&self.part);
            return Err(why);
        }
        put_in_place(&self.part, &self.path)?;
        Ok(vec![self.path.display().to_string()])
    }

    fn abandon(mut self: Box<Self>) {
        drop(self.stream.take());
        let _ = std::fs::remove_file(&self.part);
    }
}

struct Shapefile {
    stream: Option<kentos_formats::shp::stream::Stream>,
    layer: String,
    made: u32,
}

impl Sink for Shapefile {
    fn write(&mut self, rows: &[NewFeature], _objects: &[&kentos_commands::Entity]) -> Result<(), String> {
        let Some(stream) = self.stream.as_mut() else { return Ok(()) };
        for r in rows {
            self.made += 1;
            if let Some(e) = object_of(r, &self.layer, self.made) {
                stream.put(&e);
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<Vec<String>, String> {
        let Some(stream) = self.stream.take() else { return Ok(Vec::new()) };
        let (files, _) = stream.finish()?;
        Ok(files.iter().map(|f| f.display().to_string()).collect())
    }

    fn abandon(mut self: Box<Self>) {
        if let Some(stream) = self.stream.take() {
            stream.abandon();
        }
    }
}
