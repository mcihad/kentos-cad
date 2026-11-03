#!/usr/bin/env python3
"""The shared cases of `file.export.gltf` (plan/road.md Y12.3, docs/adr/0910 §3), and the check of a file KentOS wrote.

The drawing is road_corridor_command_cases.py's: its terrain the profile reference's 120 points triangulated by
Triangle, its corridors' top surfaces road_corridor_cases.py's; no KentOS code. A case says what the file holds (its
bytes and digest aside): each mesh's vertices (a corridor's triangles' corners once), triangles and bounds, the origin
(the bounds' middle to the whole metre) and the largest float32 gap of a vertex relative to it (numpy's float32, the
IEEE rounding Rust's `as f32` does).
`--written` reads the GLB KentOS wrote (`KENTOS_WRITE_GLTF=1 cargo test -p kentos-native-application --test all
gltf_files`) with its own GLB reader (the Khronos layout: header, JSON chunk padded with spaces, BIN with zeros),
checks every mesh's triangles, as the root's translation plus their float32 positions, against the references' within
the reported gap, loads it again with trimesh (a second, independent glTF reader), and runs Khronos' glTF-Validator
where it is installed (`npm install --prefix .run/gltf-validator gltf-validator`).

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_scene_cases.py [--check]
    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with trimesh \\
        python3 scripts/fixtures/road_scene_cases.py --written
"""

import copy
import json
import math
import struct
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/file.export.gltf.json"
WRITTEN = ROOT / "fixtures/road/v1/gltf/out/yol.glb"
THERE = "$fixture:road/v1/gltf/var.glb"

completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED = pcmd.UNCHANGED
C1, C2 = ccmd.C1, ccmd.C2
C3 = {**copy.deepcopy(C2), "id": "c-3", "name": "Yüzeysiz koridor", "surface": "t-2"}
SETUP = ccmd.setup()
SETUP_C3 = ccmd.setup([C1, C2, C3])
LOCAL = copy.deepcopy(SETUP)
LOCAL["settings"]["srid"] = 0


def losses(message, path):
    return {"code": "export_losses", "message": message, "path": path}


def terrain():
    xy = pcmd.TIN.xy
    return [[x, y, z] for (x, y), z in zip(xy, pcmd.TIN.z)], [list(t) for t in pcmd.TIN.tris]


def corridor(c):
    """The corridor's top surface as an indexed mesh, a corner once however many triangles share it."""
    _, _, tris, _ = ccmd.make(c)
    at, vertices, out = {}, [], []
    for t in tris:
        row = []
        for p in t:
            key = (float(p[0]), float(p[1]), float(p[2]))
            if key not in at:
                at[key] = len(vertices)
                vertices.append(list(key))
            row.append(at[key])
        out.append(row)
    return vertices, out


SURFACES = [{"id": "t-1", "name": "Mevcut zemin", "mesh": terrain}, {"id": "t-2", "name": "Kurulamaz", "mesh": None}]
UNBUILT = "Kaynaklardan yüzey kurulamıyor: kotlu yer üçten az ya da hepsi bir doğru üstünde."


def pick(items, names, code, what, field):
    if names is None:
        return [(x, False) for x in items], None
    out = []
    for i, n in enumerate(names):
        x = next((y for y in items if y["id"] == n), None) or \
            next((y for y in items if y["name"].lower() == n.lower()), None)
        if x is None:
            return None, failed(code, f"“{n}” kimlikli ya da adlı {what} projede yok.", f"{field}[{i}]")
        if all(x is not y for y, _ in out):
            out.append((x, True))
    return out, None


def answer(input_, corridors=(C1, C2), srid=5254):
    """The command's result and warnings for `input_` (the path already good)."""
    warnings, meshes = [], []
    surfaces, refused = pick(SURFACES, input_.get("surfaces"), "unknown_terrain", "yüzey", "surfaces")
    if refused:
        return refused, []
    chosen, refused = pick(list(corridors), input_.get("corridors"), "unknown_corridor", "koridor", "corridors")
    if refused:
        return refused, []
    for s, asked in surfaces:
        if s["mesh"] is None:
            if asked:
                return failed("invalid_gltf", f"“{s['name']}” yüzeyi kurulamıyor: {UNBUILT}", "surfaces"), []
            warnings.append(losses(f"“{s['name']}” yüzeyi kurulamıyor; yazılmadı.", "surfaces"))
            continue
        v, t = s["mesh"]()
        meshes.append(("terrain", s["id"], s["name"], v, t))
    for c, asked in chosen:
        v, t = corridor(c)
        if not t:
            if asked:
                return failed("invalid_gltf", f"“{c['name']}” koridoru üst yüzeyi yok", "corridors"), []
            warnings.append(losses(f"“{c['name']}” koridoru üst yüzeyi yok; yazılmadı.", "corridors"))
            continue
        meshes.append(("corridor", c["id"], c["name"], v, t))
    if not meshes:
        return failed("invalid_gltf", "Yazılacak bir şey yok: seçilen yüzey ya da koridor yok."), []
    if not srid:
        warnings.append(losses("Projenin koordinat sistemi yok; sahnenin koordinatları yerel.", "path"))
    allv = [p for m in meshes for p in m[3]]
    lo = [min(p[a] for p in allv) for a in range(3)]
    hi = [max(p[a] for p in allv) for a in range(3)]
    origin = [math.floor((lo[a] + hi[a]) / 2 + 0.5) for a in range(3)]
    error = 0.0
    out = []
    for kind, id_, name, v, t in meshes:
        arr = np.array(v, dtype=np.float64) - np.array(origin, dtype=np.float64)
        error = max(error, float(np.max(np.abs(arr.astype(np.float32).astype(np.float64) - arr))))
        out.append({"kind": kind, "id": id_, "name": name, "vertices": len(v), "triangles": len(t),
                    "min": [min(p[a] for p in v) for a in range(3)], "max": [max(p[a] for p in v) for a in range(3)]})
    result = {"path": input_["path"], "meshes": out, "origin": [float(o) for o in origin]}
    if srid:
        result["crs"] = f"EPSG:{srid}"
    result.update({"error": error, "bytes": "$any", "sha256": "$any"})
    return completed(result), warnings


def ok(name, input_, op="execute", setup_=None, corridors=(C1, C2), srid=5254):
    result, warnings = answer(input_, corridors, srid)
    if op == "validate" and result["status"] == "completed":
        result = completed(None)
    result["warnings"] = warnings
    return case(name, [step(op, input_, result, UNCHANGED)], setup_)


def no(name, input_, result, setup_=None):
    return case(name, [step("execute", input_, result, UNCHANGED)], setup_)


def cases():
    names = iter(range(1, 100))

    def tmp():
        return f"$tmp:yol-{next(names)}.glb"
    bad_path = failed("invalid_file_path", "glTF'in yolu boş ya da göreli; mutlak bir .glb yolu verin.", "path")
    return [
        ok("Bütün yüzeyler ve koridorlar; kurulamayan yüzey yazılmaz", {"path": tmp()}),
        ok("Planı", {"path": tmp()}, op="plan"),
        ok("Doğrulama", {"path": tmp()}, op="validate"),
        ok("Yalnız zemin", {"path": tmp(), "surfaces": ["Mevcut zemin"], "corridors": []}),
        ok("Yalnız ana koridor", {"path": tmp(), "surfaces": [], "corridors": ["c-1"]}),
        ok("Zemin ve yan koridor adla", {"path": tmp(), "surfaces": ["t-1"], "corridors": ["Yan koridor"]}),
        ok("Kimlik ve adla bir kez", {"path": tmp(), "surfaces": [], "corridors": ["c-2", "YAN koridor", "c-1"]}),
        ok("Var olan dosyanın üstüne, planla", {"path": THERE, "overwrite": True}, op="plan"),
        ok("Seçilmeyen yüzeysiz koridor uyarıyla", {"path": tmp()}, setup_=SETUP_C3, corridors=(C1, C2, C3)),
        ok("Koordinat sistemsiz proje", {"path": tmp(), "surfaces": ["t-1"], "corridors": []}, setup_=LOCAL, srid=0),
        no("Adla seçilen yüzeysiz koridor", {"path": tmp(), "corridors": ["Yüzeysiz koridor"]},
           failed("invalid_gltf", "“Yüzeysiz koridor” koridoru üst yüzeyi yok", "corridors"), SETUP_C3),
        no("Adla seçilen kurulamayan yüzey", {"path": tmp(), "surfaces": ["Kurulamaz"]},
           failed("invalid_gltf", f"“Kurulamaz” yüzeyi kurulamıyor: {UNBUILT}", "surfaces")),
        no("Yol projesi değil", {"path": tmp()},
           failed("not_a_road_project", "3B sahne yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
           ccmd.setup(work="map")),
        no("Boş yol", {"path": ""}, bad_path),
        no("Göreli yol", {"path": "yol.glb"}, bad_path),
        no("GLB değil", {"path": "$tmp:yol.gltf"}, failed("invalid_file_path", "“$tmp:yol.gltf” bir .glb dosyası değil.", "path")),
        no("Klasörü yok", {"path": "$tmp:yok/yol.glb"},
           failed("invalid_file_path", "“$tmp:yok” klasörü yok; dosya var olan bir klasöre yazılır.", "path")),
        no("Dosya var", {"path": THERE}, failed("file_exists", f"{THERE} var; üzerine yazmak için overwrite verin.", "path")),
        no("Olmayan yüzey", {"path": tmp(), "surfaces": ["t-1", "yok"]},
           failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surfaces[1]")),
        no("Olmayan koridor", {"path": tmp(), "corridors": ["yok"]},
           failed("unknown_corridor", "“yok” kimlikli ya da adlı koridor projede yok.", "corridors[0]")),
        no("Boş listeler: yazılacak bir şey yok", {"path": tmp(), "surfaces": [], "corridors": []},
           failed("invalid_gltf", "Yazılacak bir şey yok: seçilen yüzey ya da koridor yok.")),
    ]


# ── The written file ──────────────────────────────────────────────────────────────────────────────────────────────

def read_glb(data):
    magic, version, length = struct.unpack_from("<III", data, 0)
    assert magic == 0x46546C67 and version == 2 and length == len(data), (magic, version, length)
    jl, jt = struct.unpack_from("<II", data, 12)
    assert jt == 0x4E4F534A and jl % 4 == 0
    text = data[20:20 + jl]
    assert text == text.rstrip(b" ") + b" " * (len(text) - len(text.rstrip(b" ")))
    doc = json.loads(text)
    bl, bt = struct.unpack_from("<II", data, 20 + jl)
    assert bt == 0x004E4942 and bl % 4 == 0 and 28 + jl + bl == len(data)
    return doc, data[28 + jl:28 + jl + bl]


def check_written():
    data = WRITTEN.read_bytes()
    doc, bin_ = read_glb(data)
    assert doc["asset"]["version"] == "2.0"
    root = doc["nodes"][0]
    tx, ty, tz = root["translation"]
    origin = [tx, -tz, ty]
    result, _ = answer({"path": str(WRITTEN)})
    out = result["output"]
    assert origin == out["origin"], (origin, out["origin"])
    want = {"t-1": terrain(), "c-1": corridor(C1), "c-2": corridor(C2)}
    triangles = 0
    for k, m in zip(root["children"], doc["meshes"]):
        node = doc["nodes"][k]
        v_ref, t_ref = want[node["extras"]["id"]]
        prim = m["primitives"][0]
        pa = doc["accessors"][prim["attributes"]["POSITION"]]
        ia = doc["accessors"][prim["indices"]]
        pv = doc["bufferViews"][pa["bufferView"]]
        iv = doc["bufferViews"][ia["bufferView"]]
        gl = np.frombuffer(bin_[pv["byteOffset"]:pv["byteOffset"] + pv["byteLength"]], dtype="<f4").reshape(-1, 3)
        idx = np.frombuffer(bin_[iv["byteOffset"]:iv["byteOffset"] + iv["byteLength"]], dtype="<u4").reshape(-1, 3)
        assert list(gl.min(axis=0)) == pa["min"] and list(gl.max(axis=0)) == pa["max"], m["name"]
        world = np.stack([gl[:, 0], -gl[:, 2], gl[:, 1]], axis=1).astype(np.float64) + np.array(origin)
        ours = sorted(tuple(sorted(tuple(np.round(world[i], 4)) for i in t)) for t in idx)
        theirs = sorted(tuple(sorted(tuple(np.round(np.array(v_ref[i], dtype=np.float64), 4)) for i in t))
                        for t in t_ref)
        assert len(ours) == len(theirs), (m["name"], len(ours), len(theirs))
        gap = max(abs(a - b) for x, y in zip(ours, theirs) for p, q in zip(x, y) for a, b in zip(p, q))
        assert gap <= 1e-4 + out["error"], (m["name"], gap)
        # Counterclockwise seen from above, as the source.
        for t in idx[:50]:
            a, b, c = (world[i] for i in t)
            assert (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) > 0, m["name"]
        triangles += len(idx)
    # A second, independent reader: trimesh's glTF loader (its own GLB parsing, flat normals as the spec asks).
    import trimesh
    loaded = trimesh.load(str(WRITTEN), force="scene")
    faces = sum(len(g.faces) for g in loaded.geometry.values())
    assert faces == triangles and len(loaded.geometry) == len(doc["meshes"]), (faces, triangles)
    lo, hi = loaded.bounds
    want_lo = [min(m["min"][a] for m in out["meshes"]) for a in range(3)]
    want_hi = [max(m["max"][a] for m in out["meshes"]) for a in range(3)]
    gl_lo = [want_lo[0] - origin[0], want_lo[2] - origin[2], -(want_hi[1] - origin[1])]
    gl_hi = [want_hi[0] - origin[0], want_hi[2] - origin[2], -(want_lo[1] - origin[1])]
    t = np.array([tx, ty, tz])
    assert np.allclose(lo, np.array(gl_lo) + t, atol=1e-3) and np.allclose(hi, np.array(gl_hi) + t, atol=1e-3), (lo, hi)
    said = "trimesh aynı ağları ve sınırları okudu"
    # Khronos' glTF-Validator where it is installed (`npm install --prefix .run/gltf-validator gltf-validator`).
    validator = ROOT / ".run/gltf-validator"
    if (validator / "node_modules/gltf-validator").is_dir():
        report = subprocess.run(
            ["node", "-e", "const v=require('gltf-validator');const fs=require('fs');"
             "v.validateBytes(new Uint8Array(fs.readFileSync(process.argv[1]))).then(r=>console.log(JSON.stringify(r.issues)))",
             str(WRITTEN)], cwd=validator, capture_output=True, text=True, check=True)
        issues = json.loads(report.stdout)
        assert issues["numErrors"] == 0 and issues["numWarnings"] == 0, issues["messages"][:5]
        said += f"; glTF-Validator: {issues['numErrors']} hata, {issues['numWarnings']} uyarı"
    else:
        said += "; glTF-Validator kurulu değil, atlandı"
    print(f"{WRITTEN.name}: {len(doc['meshes'])} ağ, {triangles} üçgen başvuruyla aynı (en çok {out['error']:.2e} m "
          f"float32 farkı); {said}")


def main():
    if "--written" in sys.argv:
        check_written()
        return
    data = {"format": "kentos.command-cases", "version": 1, "command": "file.export.gltf", "commandVersion": 1,
            "title": "glTF dışa aktar",
            "note": "scripts/fixtures/road_scene_cases.py yazar; arazi profil başvurusunun Triangle üçgenlemesi, "
                    "koridorların üst yüzeyi road_corridor_cases.py'nin başvurusundandır; KentOS kodu yok. Dosyanın "
                    "içeriği --written ile kendi GLB okuyucusu ve Khronos glTF-Validator'la denetlenir.",
            "setup": SETUP, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
