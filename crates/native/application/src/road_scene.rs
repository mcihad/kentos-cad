//! A road project's 3D scene (plan/road.md Y12.3, docs/adr/0910 §3): its terrain surfaces and its corridors' top
//! surfaces as indexed meshes in the source's coordinates, built now from their sources, and `file.export.gltf` v1,
//! which writes them as glTF 2.0 binary by `kentos-formats`' `gltf`, whole or not at all; the drawing does not change.
//! The desktop's 3D view draws the same meshes. The shared cases are `fixtures/commands/v1/file.export.gltf.json`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use kentos_commands::{CommandResult, CommandWarning, ExportGltf, GltfExported, SceneMeshKind, SceneMeshWritten};
use kentos_domain::Document;
use kentos_formats::gltf::{Material, Mesh, Scene, write};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_terrains::{self as terrains, fail};

/// A mesh of the scene: what it is made from, its source, its vertices (x east, y north, z up) and its triangles,
/// counterclockwise seen from above.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePart {
    pub kind: SceneMeshKind,
    pub id: String,
    pub name: String,
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

fn losses(message: String, path: &str) -> CommandWarning {
    CommandWarning { code: codes::EXPORT_LOSSES.into(), message, path: Some(path.into()) }
}

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_GLTF, words, path)
}

/// A triangle soup as an indexed mesh: a vertex once however many triangles share it (the same bits), in the order
/// they come.
fn indexed(soup: &[[[f64; 3]; 3]]) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let mut at: HashMap<[u64; 3], u32> = HashMap::new();
    let mut vertices = Vec::new();
    let triangles = soup
        .iter()
        .map(|t| {
            t.map(|p| {
                *at.entry(p.map(f64::to_bits)).or_insert_with(|| {
                    vertices.push(p);
                    (vertices.len() - 1) as u32
                })
            })
        })
        .collect();
    (vertices, triangles)
}

/// The chosen ones of a list by id or name (absent: every one, not asked by name), each once.
fn chosen<T>(
    list: &[T],
    names: Option<&[String]>,
    key: impl Fn(&T) -> (&str, &str),
    unknown: impl Fn(usize, &str) -> Stop,
) -> Result<Vec<(usize, bool)>, Stop> {
    let Some(names) = names else { return Ok((0..list.len()).map(|i| (i, false)).collect()) };
    let mut out: Vec<(usize, bool)> = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let at = list
            .iter()
            .position(|x| key(x).0 == n)
            .or_else(|| list.iter().position(|x| key(x).1.to_lowercase() == n.to_lowercase()))
            .ok_or_else(|| unknown(i, n))?;
        if !out.iter().any(|(j, _)| *j == at) {
            out.push((at, true));
        }
    }
    Ok(out)
}

/// The scene's meshes now: the chosen surfaces then the chosen corridors (absent: every one). One asked by name that
/// does not build, or a corridor with no top surface, refuses; one not asked by name is left out with a warning.
pub(crate) fn parts(
    doc: &Document,
    surfaces: Option<&[String]>,
    corridor_names: Option<&[String]>,
) -> Result<(Vec<ScenePart>, Vec<CommandWarning>), Stop> {
    let own = terrains::surfaces(doc);
    let picked = chosen(own, surfaces, |s| (&s.id, &s.name), |i, n| {
        fail(codes::UNKNOWN_TERRAIN, format!("“{n}” kimlikli ya da adlı yüzey projede yok."), &format!("surfaces[{i}]"))
    })?;
    let list = doc.road_corridors();
    let picked_corridors = chosen(list, corridor_names, |c| (&c.id, &c.name), |i, n| {
        fail(codes::UNKNOWN_CORRIDOR, format!("“{n}” kimlikli ya da adlı koridor projede yok."), &format!("corridors[{i}]"))
    })?;
    let (mut out, mut warnings) = (Vec::new(), Vec::new());
    for (i, asked) in picked {
        let def = &own[i];
        let made = terrains::make(doc, def);
        match made.built() {
            Ok(s) => out.push(ScenePart {
                kind: SceneMeshKind::Terrain,
                id: def.id.clone(),
                name: def.name.clone(),
                vertices: s.vertices.clone(),
                triangles: s.triangles.clone(),
            }),
            Err(stop) if asked => {
                return Err(bad("surfaces", format!("“{}” yüzeyi kurulamıyor: {}", def.name, corridors::words(stop))));
            }
            Err(_) => warnings.push(losses(format!("“{}” yüzeyi kurulamıyor; yazılmadı.", def.name), "surfaces")),
        }
    }
    for (i, asked) in picked_corridors {
        let def = &list[i];
        let why = match corridors::make(doc, def, false) {
            Ok(m) if !m.built.triangles.is_empty() => {
                let (vertices, triangles) = indexed(&m.built.triangles);
                out.push(ScenePart {
                    kind: SceneMeshKind::Corridor,
                    id: def.id.clone(),
                    name: def.name.clone(),
                    vertices,
                    triangles,
                });
                continue;
            }
            Ok(_) => "üst yüzeyi yok".to_owned(),
            Err(stop) => format!("kurulamıyor: {}", corridors::words(stop)),
        };
        if asked {
            return Err(bad("corridors", format!("“{}” koridoru {why}", def.name)));
        }
        warnings.push(losses(format!("“{}” koridoru {why}; yazılmadı.", def.name), "corridors"));
    }
    Ok((out, warnings))
}

/// The scene's origin: its bounds' middle to the whole metre.
pub fn origin_of(parts: &[ScenePart]) -> [f64; 3] {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in parts.iter().flat_map(|p| &p.vertices) {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
    }
    if lo[0] > hi[0] {
        return [0.0; 3];
    }
    std::array::from_fn(|a| ((lo[a] + hi[a]) / 2.0 + 0.5).floor())
}

/// The road project's scene for the desktop's 3D view: every surface and corridor that builds now, and the origin.
pub fn scene(doc: &Document) -> (Vec<ScenePart>, [f64; 3]) {
    if corridors::road_project(doc).is_err() {
        return (Vec::new(), [0.0; 3]);
    }
    let parts = parts(doc, None, None).map(|(p, _)| p).unwrap_or_default();
    let origin = origin_of(&parts);
    (parts, origin)
}

struct Planned {
    path: PathBuf,
    out: GltfExported,
    bytes: Vec<u8>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &ExportGltf) -> Result<Planned, Stop> {
    if doc.settings().cad_work != Some(kentos_commands::CadWork::Road) {
        return Err(fail(codes::NOT_A_ROAD_PROJECT, "3B sahne yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""));
    }
    let text = input.path.trim();
    if text.is_empty() || !Path::new(text).is_absolute() {
        return Err(fail(codes::INVALID_FILE_PATH, "glTF'in yolu boş ya da göreli; mutlak bir .glb yolu verin.".into(), "path"));
    }
    if !text.to_ascii_lowercase().ends_with(".glb") {
        return Err(fail(codes::INVALID_FILE_PATH, format!("“{text}” bir .glb dosyası değil."), "path"));
    }
    let path = PathBuf::from(text);
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if !folder.is_dir() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("“{}” klasörü yok; dosya var olan bir klasöre yazılır.", folder.display()),
            "path",
        ));
    }
    if path.exists() && !input.overwrite {
        return Err(fail(codes::FILE_EXISTS, format!("{} var; üzerine yazmak için overwrite verin.", path.display()), "path"));
    }
    let (parts, mut warnings) = parts(doc, input.surfaces.as_deref(), input.corridors.as_deref())?;
    if parts.is_empty() {
        return Err(bad("", "Yazılacak bir şey yok: seçilen yüzey ya da koridor yok.".into()));
    }
    let crs = kentos_project::systems::own(doc.settings())
        .map(|n| if n.code.starts_with("EPSG:") { n.code.clone() } else { n.name.clone() });
    if crs.is_none() {
        warnings.push(losses("Projenin koordinat sistemi yok; sahnenin koordinatları yerel.".into(), "path"));
    }
    let origin = origin_of(&parts);
    let mut error: f64 = 0.0;
    let mut meshes = Vec::new();
    let mut written = Vec::new();
    for p in &parts {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let positions = p
            .vertices
            .iter()
            .map(|v| {
                std::array::from_fn(|a| {
                    lo[a] = lo[a].min(v[a]);
                    hi[a] = hi[a].max(v[a]);
                    let d = v[a] - origin[a];
                    let f = d as f32;
                    error = error.max((f64::from(f) - d).abs());
                    f
                })
            })
            .collect();
        let terrain = p.kind == SceneMeshKind::Terrain;
        meshes.push(Mesh {
            name: p.name.clone(),
            positions,
            triangles: p.triangles.clone(),
            material: usize::from(!terrain),
            extras: json!({"kind": if terrain { "terrain" } else { "corridor" }, "id": p.id}),
        });
        written.push(SceneMeshWritten {
            kind: p.kind,
            id: p.id.clone(),
            name: p.name.clone(),
            vertices: p.vertices.len() as u32,
            triangles: p.triangles.len() as u32,
            min: lo,
            max: hi,
        });
    }
    let scene = Scene {
        generator: format!("KentOS Yol {}", env!("CARGO_PKG_VERSION")),
        root: doc.name().to_owned(),
        origin,
        extras: json!({"crs": crs, "origin": origin, "frame": "x doğu, y kuzey, z yukarı; glTF'te (x, z, -y)"}),
        materials: vec![
            Material { name: "Arazi".into(), color: [0.42, 0.48, 0.30, 1.0], double_sided: true },
            Material { name: "Koridor".into(), color: [0.55, 0.55, 0.57, 1.0], double_sided: true },
        ],
        meshes,
    };
    let bytes = write(&scene);
    let sha256 = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    let out = GltfExported {
        path: path.display().to_string(),
        meshes: written,
        origin,
        crs,
        error,
        bytes: bytes.len() as u64,
        sha256,
    };
    Ok(Planned { path, out, bytes, warnings })
}

/// The bytes written at `path` whole or not at all: beside it, read back whole, then moved over it.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".kentos-yaziliyor");
    let tmp = PathBuf::from(tmp);
    let done = (|| -> Result<(), String> {
        std::fs::write(&tmp, bytes).map_err(|e| format!("{} yazılamadı: {e}", tmp.display()))?;
        let back = std::fs::read(&tmp).map_err(|e| format!("{} geri okunamadı: {e}", tmp.display()))?;
        if back != bytes {
            return Err("yazılan dosya geri okununca aynı çıkmadı".into());
        }
        std::fs::rename(&tmp, path).map_err(|e| format!("{} yerine konamadı: {e}", path.display()))
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}

/// Checks `input` against the document; writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ExportGltf) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// What the file would hold; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ExportGltf) -> CommandResult<GltfExported> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: p.out, warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the file; the drawing does not change.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ExportGltf) -> CommandResult<GltfExported> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if let Err(why) = write_file(&p.path, &p.bytes) {
        return Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("glTF yazılamadı: {why}. Hiçbir şey değişmedi."),
            Some("path".into()),
        ))
        .into();
    }
    CommandResult::Completed { output: p.out, warnings: p.warnings }
}
