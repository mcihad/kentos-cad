//! glTF 2.0 binary (GLB) of triangle meshes (plan/road.md Y12.3, docs/adr/0910 §3), after the Khronos specification
//! (<https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html>): metres, right-handed, +Y up; a 12-byte header
//! (`glTF`, version 2, the file's length) and two chunks, JSON padded with spaces and BIN with zeros to four bytes.
//!
//! A scene is one root node whose `translation` carries the scene's origin in full (a JSON number, f64), with a child
//! node per mesh; the meshes' positions are float32 *relative to that origin* (plan/3d.md SCENE-PREC-04), so a
//! projected coordinate of millions of metres keeps its millimetres. Positions come in the drawing's frame (x east,
//! y north, z up) and are written in glTF's (east, up, −north): a proper rotation, so a triangle counterclockwise seen
//! from above stays front-facing seen from +Y. No normals are written: the specification has the reader make flat
//! ones. Each POSITION accessor carries its exact float32 min and max, as the specification requires.

use serde_json::{Value, json};

/// A material: its name, its base colour (linear RGBA) and whether both faces show.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    pub color: [f32; 4],
    pub double_sided: bool,
}

/// A mesh: its name, positions relative to the scene's origin (drawing frame: x east, y north, z up), triangles
/// counterclockwise seen from above, the material's index and what its node carries in `extras` (`Value::Null`:
/// nothing).
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub material: usize,
    pub extras: Value,
}

/// A scene: the generator's name, the root node's name, its origin (drawing frame), what the root carries in
/// `extras`, the materials and the meshes.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub generator: String,
    pub root: String,
    pub origin: [f64; 3],
    pub extras: Value,
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,
}

const MAGIC: u32 = 0x4654_6C67;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;
const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// The drawing frame's (east, north, up) as glTF's (x, y, z).
fn to_gltf<T: Copy + std::ops::Neg<Output = T>>(p: [T; 3]) -> [T; 3] {
    [p[0], p[2], -p[1]]
}

/// glTF's (x, y, z) as the drawing frame's (east, north, up).
fn from_gltf<T: Copy + std::ops::Neg<Output = T>>(p: [T; 3]) -> [T; 3] {
    [p[0], -p[2], p[1]]
}

fn pad(bytes: &mut Vec<u8>, with: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(with);
    }
}

/// The scene as a GLB file.
pub fn write(scene: &Scene) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let (mut views, mut accessors, mut meshes, mut nodes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (k, m) in scene.meshes.iter().enumerate() {
        let gl: Vec<[f32; 3]> = m.positions.iter().map(|p| to_gltf(*p)).collect();
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &gl {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        let at = bin.len();
        for p in &gl {
            for v in p {
                bin.extend_from_slice(&v.to_le_bytes());
            }
        }
        views.push(json!({"buffer": 0, "byteOffset": at, "byteLength": bin.len() - at, "target": ARRAY_BUFFER}));
        accessors.push(json!({"bufferView": views.len() - 1, "componentType": FLOAT, "count": gl.len(), "type": "VEC3",
                              "min": lo.map(f64::from), "max": hi.map(f64::from)}));
        let at = bin.len();
        for t in &m.triangles {
            for i in t {
                bin.extend_from_slice(&i.to_le_bytes());
            }
        }
        views.push(json!({"buffer": 0, "byteOffset": at, "byteLength": bin.len() - at, "target": ELEMENT_ARRAY_BUFFER}));
        accessors.push(json!({"bufferView": views.len() - 1, "componentType": UNSIGNED_INT,
                              "count": m.triangles.len() * 3, "type": "SCALAR"}));
        meshes.push(json!({"name": m.name, "primitives": [{"attributes": {"POSITION": accessors.len() - 2},
                           "indices": accessors.len() - 1, "material": m.material, "mode": 4}]}));
        let mut node = json!({"name": m.name, "mesh": k});
        if !m.extras.is_null() {
            node["extras"] = m.extras.clone();
        }
        nodes.push(node);
    }
    let mut root = json!({"name": scene.root, "translation": to_gltf(scene.origin),
                          "children": (1..=scene.meshes.len()).collect::<Vec<_>>()});
    if !scene.extras.is_null() {
        root["extras"] = scene.extras.clone();
    }
    nodes.insert(0, root);
    let materials: Vec<Value> = scene
        .materials
        .iter()
        .map(|m| {
            json!({"name": m.name, "doubleSided": m.double_sided,
                   "pbrMetallicRoughness": {"baseColorFactor": m.color.map(f64::from), "metallicFactor": 0.0,
                                            "roughnessFactor": 1.0}})
        })
        .collect();
    let mut doc = json!({
        "asset": {"version": "2.0", "generator": scene.generator},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
    });
    if !bin.is_empty() {
        doc["buffers"] = json!([{"byteLength": bin.len()}]);
    } else {
        doc.as_object_mut().map(|o| o.remove("bufferViews"));
    }
    let mut text = serde_json::to_vec(&doc).unwrap_or_default();
    pad(&mut text, b' ');
    pad(&mut bin, 0);
    let length = 12 + 8 + text.len() + if bin.is_empty() { 0 } else { 8 + bin.len() };
    let mut out = Vec::with_capacity(length);
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(length as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(&JSON_CHUNK.to_le_bytes());
    out.extend_from_slice(&text);
    if !bin.is_empty() {
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(&BIN_CHUNK.to_le_bytes());
        out.extend_from_slice(&bin);
    }
    out
}

/// A mesh read back: its name, positions (drawing frame, relative to the origin) and triangles.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadMesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

/// A GLB file KentOS wrote, read back: the root's origin (drawing frame) and the meshes. Only what [`write`] writes is
/// read; anything else is an error, not a guess.
pub fn read(bytes: &[u8]) -> Result<([f64; 3], Vec<ReadMesh>), String> {
    let word = |at: usize| -> Result<u32, String> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| "dosya kısa".to_owned())
    };
    if word(0)? != MAGIC || word(4)? != 2 || word(8)? as usize != bytes.len() {
        return Err("GLB başlığı değil".into());
    }
    let jl = word(12)? as usize;
    if word(16)? != JSON_CHUNK {
        return Err("JSON parçası yok".into());
    }
    let text = bytes.get(20..20 + jl).ok_or("JSON parçası kısa")?;
    let doc: Value = serde_json::from_slice(text).map_err(|e| format!("JSON okunmadı: {e}"))?;
    let bin = if bytes.len() > 20 + jl {
        let bl = word(20 + jl)? as usize;
        if word(24 + jl)? != BIN_CHUNK {
            return Err("BIN parçası değil".into());
        }
        bytes.get(28 + jl..28 + jl + bl).ok_or("BIN parçası kısa")?
    } else {
        &[]
    };
    let num = |v: &Value| v.as_f64().ok_or_else(|| "sayı değil".to_owned());
    let t = &doc["nodes"][0]["translation"];
    let origin = from_gltf([num(&t[0])?, num(&t[1])?, num(&t[2])?]);
    let view = |acc: &Value| -> Result<&[u8], String> {
        let v = &doc["bufferViews"][acc["bufferView"].as_u64().ok_or("görünüm yok")? as usize];
        let at = v["byteOffset"].as_u64().unwrap_or(0) as usize;
        let len = v["byteLength"].as_u64().ok_or("boy yok")? as usize;
        bin.get(at..at + len).ok_or_else(|| "görünüm taşıyor".to_owned())
    };
    let mut out = Vec::new();
    for m in doc["meshes"].as_array().ok_or("ağ yok")? {
        let p = &m["primitives"][0];
        let pa = &doc["accessors"][p["attributes"]["POSITION"].as_u64().ok_or("konum yok")? as usize];
        let ia = &doc["accessors"][p["indices"].as_u64().ok_or("dizin yok")? as usize];
        let f = |c: &[u8]| f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        let positions = view(pa)?.chunks_exact(12).map(|c| from_gltf([f(&c[0..4]), f(&c[4..8]), f(&c[8..12])])).collect();
        let idx: Vec<u32> = view(ia)?.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        let triangles = idx.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
        out.push(ReadMesh { name: m["name"].as_str().unwrap_or_default().to_owned(), positions, triangles });
    }
    Ok((origin, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_goes_out_and_back() {
        let scene = Scene {
            generator: "KentOS".into(),
            root: "Yol".into(),
            origin: [500_123.25, 4_400_456.5, 1_001.125],
            extras: json!({"crs": "EPSG:5254"}),
            materials: vec![Material { name: "Arazi".into(), color: [0.5, 0.6, 0.4, 1.0], double_sided: true }],
            meshes: vec![Mesh {
                name: "Zemin".into(),
                positions: vec![[0.0, 0.0, 0.0], [10.0, 0.0, 1.0], [0.0, 10.0, -1.5]],
                triangles: vec![[0, 1, 2]],
                material: 0,
                extras: Value::Null,
            }],
        };
        let bytes = write(&scene);
        assert!(bytes.len().is_multiple_of(4));
        assert_eq!(&bytes[0..4], b"glTF");
        let (origin, meshes) = read(&bytes).unwrap();
        assert_eq!(origin, scene.origin);
        assert_eq!(meshes[0].positions, scene.meshes[0].positions);
        assert_eq!(meshes[0].triangles, scene.meshes[0].triangles);
        // Seen from glTF's +Y (up) the triangle stays counterclockwise: (x, -z) keeps the plan's orientation.
        let text = String::from_utf8_lossy(&bytes[20..20 + u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize])
            .into_owned();
        let doc: Value = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(doc["accessors"][0]["max"], json!([10.0, 1.0, 0.0]));
        assert_eq!(doc["accessors"][0]["min"], json!([0.0, -1.5, -10.0]));
    }
}
