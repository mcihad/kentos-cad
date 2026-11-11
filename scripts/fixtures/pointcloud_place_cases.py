#!/usr/bin/env python3
"""The shared cases of `cad.pointclouds.place` (Bulutu konumla; plan/map-cad.md MAP-13, docs/adr/5100), without
KentOS code.

    python3 scripts/fixtures/pointcloud_place_cases.py           # writes the cases
    python3 scripts/fixtures/pointcloud_place_cases.py --check   # writes nothing; compares

Everything here is this file's own arithmetic, in other ways than the command's:
- Euler angles as the product of three elementary turns, Rz(yaw) · Ry(pitch) · Rx(roll) (OGC GeoPose 1.0 Basic-YPR:
  turns about the turned axes z, y′, x″ in that order; the same matrix as CloudCompare's `initFromParameters`), not the
  expanded rows;
- an axis and an angle by the unit quaternion (cos θ/2, sin θ/2 · n), not Rodrigues' rows;
- a map about a centre c: p ↦ R(p − c) + c + t;
- a place taken back by the rigid inverse (Rᵀ, −Rᵀt), not by cofactors;
- a box through a place: its eight corners moved;
- the rotation's angle by acos((trace − 1) / 2).
A text cloud's points are read from its lines here; Nokta sorgula after the place finds the moved point.
"""

import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.pointclouds.place.json"
XYZ = ROOT / "fixtures" / "pointcloud" / "v1" / "files" / "bulut.xyz"
WITHIN = 1e-6


def uid(n):
    return f"0192f5c0-5100-7000-8000-{n:012x}"


# --- 3 × 3 and 4 × 4 arithmetic (lists of rows) ---

def matmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(len(b))) for j in range(len(b[0]))] for i in range(len(a))]


def rz(d):
    c, s = math.cos(math.radians(d)), math.sin(math.radians(d))
    return [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]


def ry(d):
    c, s = math.cos(math.radians(d)), math.sin(math.radians(d))
    return [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]


def rx(d):
    c, s = math.cos(math.radians(d)), math.sin(math.radians(d))
    return [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]


def euler(yaw, pitch, roll):
    return matmul(matmul(rz(yaw), ry(pitch)), rx(roll))


def quaternion(axis, degrees):
    n = math.sqrt(sum(v * v for v in axis))
    h = math.radians(degrees) / 2.0
    w, x, y, z = math.cos(h), *(math.sin(h) * v / n for v in axis)
    return [
        [1 - 2 * (y * y + z * z), 2 * (x * y - w * z), 2 * (x * z + w * y)],
        [2 * (x * y + w * z), 1 - 2 * (x * x + z * z), 2 * (y * z - w * x)],
        [2 * (x * z - w * y), 2 * (y * z + w * x), 1 - 2 * (x * x + y * y)],
    ]


def about(r, c, t):
    """p ↦ R(p − c) + c + t as 16 numbers row by row."""
    m = []
    for i in range(3):
        m += [r[i][0], r[i][1], r[i][2], c[i] + t[i] - sum(r[i][k] * c[k] for k in range(3))]
    return m + [0.0, 0.0, 0.0, 1.0]


def rows(m):
    return [m[4 * i:4 * i + 4] for i in range(4)]


def flat(a):
    return [v for row in a for v in row]


def then(first, after):
    """`after` applied after `first`."""
    return flat(matmul(rows(after), rows(first)))


def back(m):
    """A rigid map's inverse: Rᵀ and −Rᵀ t."""
    r = [[m[4 * j + i] for j in range(3)] for i in range(3)]
    t = [m[3], m[7], m[11]]
    return about(r, [0.0, 0.0, 0.0], [-sum(r[i][k] * t[k] for k in range(3)) for i in range(3)])


def move(m, p):
    return [sum(m[4 * i + k] * p[k] for k in range(3)) + m[4 * i + 3] for i in range(3)]


def box(m, b):
    pts = [move(m, [b[0] if i & 1 == 0 else b[3], b[1] if i & 2 == 0 else b[4], b[2] if i & 4 == 0 else b[5]])
           for i in range(8)]
    return [min(p[k] for p in pts) for k in range(3)] + [max(p[k] for p in pts) for k in range(3)]


def angle(m):
    return math.degrees(math.acos(max(-1.0, min(1.0, (m[0] + m[5] + m[10] - 1.0) / 2.0))))


def middle(b):
    return [(b[k] + b[k + 3]) / 2.0 for k in range(3)]


IDENTITY = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]


# --- the drawing ---

FILE_BOUNDS = [500000.0, 4400000.0, 100.0, 500100.0, 4400100.0, 150.0]
HELD = about([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], [0.0] * 3, [10.0, 0.0, 0.0])


def source(name, pose=None):
    s = {"file": f"$fixture:pointcloud/v1/files/{name}", "format": "xyz", "count": 50, "bounds": FILE_BOUNDS}
    if pose:
        s["pose"] = pose
    return s


def cloud(i, layer, sources):
    b = [math.inf] * 3 + [-math.inf] * 3
    for s in sources:
        sb = box(s["pose"], s["bounds"]) if "pose" in s else s["bounds"]
        b = [min(b[k], sb[k]) for k in range(3)] + [max(b[k + 3], sb[k + 3]) for k in range(3)]
    return {"kind": "pointcloud", "id": i, "layerId": layer, "attrs": {}, "sources": sources, "bounds": b,
            "count": 50 * len(sources), "srid": 5254, "style": {"render": "elevation", "min": 100.0, "max": 150.0,
                                                                 "size": 2}}


def layer(lid, name, locked=False):
    return {"id": lid, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


SETUP = {
    "format": "kentos.document", "version": 2, "name": "Bulutu konumla",
    "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": 500.0, "workspace": "cad", "cadWork": "map"},
    "origin": {"x": 0.0, "y": 0.0},
    "layers": [layer("bulut", "Bulut"), layer("kilitli", "Kilitli", locked=True)],
    "activeLayer": "bulut", "styles": {"items": [], "categories": []},
    "entities": [
        cloud(1, "bulut", [source("bulut.xyz")]),
        cloud(2, "bulut", [source("bulut.xyz"), source("bulut.txt")]),
        cloud(3, "bulut", [source("bulut.xyz", HELD)]),
        cloud(4, "kilitli", [source("bulut.xyz")]),
        {"kind": "line", "id": 5, "layerId": "bulut", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 1.0, "y": 0.0}},
    ],
    "uids": [uid(i) for i in range(1, 6)],
}
IDS = [1, 2, 3, 4, 5]
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}


def entity(i):
    return next(e for e in SETUP["entities"] if e["id"] == i)


def points():
    out = []
    for line in XYZ.read_text().splitlines()[1:]:
        if line.strip():
            out.append([float(v) for v in line.split()[:3]])
    return out


def query_step(i, m, k=0):
    """Nokta sorgula a hand's breadth from the k-th point where the place `m` shows it."""
    p = move(m, points()[k])
    at = {"x": p[0] + 0.002, "y": p[1] + 0.003}
    return {"op": "execute", "command": "cad.pointclouds.query", "input": {"at": at, "radius": 0.5, "uids": [f"$uidOf:{i}"]},
            "within": WITHIN,
            "result": {"status": "completed", "output": {"point": {
                "cloud": f"$uidOf:{i}", "source": 0, "index": k, "x": p[0], "y": p[1], "z": p[2],
                "distance": math.hypot(0.002, 0.003), "class": 0, "intensity": 0, "returnNumber": 1,
                "numberOfReturns": 1, "pointSourceId": 0, "gpsTime": 0.0}, "read": 50 * len(entity(i)["sources"]), "revision": "$current"},
                "warnings": []},
            "expect": {"revision": "same"}}


def placed(name, i, place, m, query=True):
    """A place written: each file's place is `m` after its own; the answer and the box; undone after."""
    e = entity(i)
    poses = [then(s.get("pose", IDENTITY), m) for s in e["sources"]]
    bounds = cloud(0, "", [dict(s, pose=p) for s, p in zip(e["sources"], poses)])["bounds"]
    out = {"uid": f"$uidOf:{i}", "matrix": m, "poses": poses, "bounds": bounds,
           "shift": math.dist(move(m, middle(e["bounds"])), middle(e["bounds"])), "angle": angle(m),
           "changed": True, "revision": "$current"}
    steps = [{"op": "execute", "input": {"uid": f"$uidOf:{i}", "place": place}, "within": WITHIN,
              "result": {"status": "completed", "output": out, "warnings": []},
              "expect": {"ids": IDS, "canUndo": True, "dirty": True, "revision": "changed",
                         "fields": {str(i): {"bounds": bounds}}}}]
    if query:
        steps.append(query_step(i, poses[0]))
    steps.append({"op": "undo", "returns": "Bulutu konumla",
                  "expect": {"ids": IDS, "canUndo": False, "canRedo": True, "fields": {str(i): {"bounds": e["bounds"]}}}})
    return {"name": name, "steps": steps}


def refused(name, input, code, message, path, non_finite=None, status="failed"):
    step = {"op": "execute", "input": input}
    if non_finite:
        step["nonFinite"] = non_finite
    error = {"code": code, "message": message, "path": path}
    if status == "conflict":
        error["revision"] = "$current"
    step["result"] = {"status": status, "error": error}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


RIGID = ("Konum yalnız dönme ve ötelemedir (katı dönüşüm): ölçek, kayma ya da yansıma uzunlukları değiştirir. "
         "Böyle bir matrisi Dönüşüm uygula ile dosyaya yazın.")


def cases():
    c1 = [500050.0, 4400050.0, 125.0]
    mid1 = middle(entity(1)["bounds"])
    out = [
        placed("Euler: Z çevresinde çeyrek dönme, verilen merkez; sorgu döndürülen noktayı bulur", 1,
               {"kind": "euler", "yaw": 90.0, "pitch": 0.0, "roll": 0.0, "center": c1},
               about(euler(90.0, 0.0, 0.0), c1, [0.0] * 3)),
        placed("Euler: üç açı, merkez boş (kutunun ortası), öteleme", 1,
               {"kind": "euler", "yaw": 0.5, "pitch": -0.25, "roll": 0.1, "translation": [0.2, -0.1, 0.05]},
               about(euler(0.5, -0.25, 0.1), mid1, [0.2, -0.1, 0.05])),
        placed("Euler: yunuslama 90° (Z ekseni X'e yatar)", 1,
               {"kind": "euler", "yaw": 0.0, "pitch": 90.0, "roll": 0.0, "center": c1},
               about(euler(0.0, 90.0, 0.0), c1, [0.0] * 3)),
        placed("Euler: yalnız yatış, negatif", 1,
               {"kind": "euler", "yaw": 0.0, "pitch": 0.0, "roll": -2.5, "center": c1, "translation": [0.0, 0.0, 1.0]},
               about(euler(0.0, 0.0, -2.5), c1, [0.0, 0.0, 1.0])),
        placed("Matris: yalnız öteleme", 1, {"kind": "matrix", "matrix": about(euler(0, 0, 0), [0.0] * 3, [100.0, -50.0, 2.0])},
               about(euler(0, 0, 0), [0.0] * 3, [100.0, -50.0, 2.0])),
        placed("Matris: ağın matrisi gibi dönme ve öteleme", 1,
               {"kind": "matrix", "matrix": about(euler(0.03, -0.02, 0.01), mid1, [0.3, -0.2, 0.06])},
               about(euler(0.03, -0.02, 0.01), mid1, [0.3, -0.2, 0.06])),
        placed("Eksen ve açı: Z çevresinde 30°, verilen merkez", 1,
               {"kind": "axisAngle", "axis": [0.0, 0.0, 1.0], "angle": 30.0, "center": c1},
               about(quaternion([0.0, 0.0, 1.0], 30.0), c1, [0.0] * 3)),
        placed("Eksen ve açı: eğik eksen, öteleme, merkez boş", 1,
               {"kind": "axisAngle", "axis": [1.0, 1.0, 1.0], "angle": -12.0, "translation": [1.0, 2.0, 3.0]},
               about(quaternion([1.0, 1.0, 1.0], -12.0), mid1, [1.0, 2.0, 3.0])),
        placed("Konumlu bulut: yeni dönüşüm eskisinin ardından", 3,
               {"kind": "euler", "yaw": 90.0, "pitch": 0.0, "roll": 0.0, "center": c1},
               about(euler(90.0, 0.0, 0.0), c1, [0.0] * 3)),
        placed("İki dosyalı bulut: iki dosya aynı dönüşümle", 2,
               {"kind": "euler", "yaw": -45.0, "pitch": 1.0, "roll": 0.0, "center": c1, "translation": [5.0, 0.0, 0.0]},
               about(euler(-45.0, 1.0, 0.0), c1, [5.0, 0.0, 0.0])),
    ]
    # Konumu kaldır on the placed cloud: the move back is its place's inverse; the files where they say.
    e3 = entity(3)
    m = back(HELD)
    out.append({"name": "Konumu kaldır: dosya kendi yerinde", "steps": [
        {"op": "execute", "input": {"uid": "$uidOf:3", "place": {"kind": "remove"}}, "within": WITHIN,
         "result": {"status": "completed", "output": {
             "uid": "$uidOf:3", "matrix": m, "poses": [None], "bounds": FILE_BOUNDS,
             "shift": math.dist(move(m, middle(e3["bounds"])), middle(e3["bounds"])), "angle": 0.0, "changed": True,
             "revision": "$current"}, "warnings": []},
         "expect": {"ids": IDS, "canUndo": True, "dirty": True, "revision": "changed",
                    "fields": {"3": {"bounds": FILE_BOUNDS}}}},
        query_step(3, IDENTITY),
        {"op": "undo", "returns": "Bulutu konumla", "expect": {"ids": IDS, "canUndo": False, "canRedo": True,
                                                              "fields": {"3": {"bounds": e3["bounds"]}}}}]})
    out.append({"name": "Konumu kaldır: konumu olmayan bulutta hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": {"uid": "$uidOf:1", "place": {"kind": "remove"}},
         "result": {"status": "completed", "output": {
             "uid": "$uidOf:1", "matrix": IDENTITY, "poses": [None], "bounds": FILE_BOUNDS, "shift": 0.0, "angle": 0.0,
             "changed": False, "revision": "$current"}, "warnings": []},
         "expect": {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"}}]})
    scaled = about(euler(0, 0, 0), [0.0] * 3, [0.0] * 3)
    scaled[0] = 1.001
    mirror = list(IDENTITY)
    mirror[10] = -1.0
    perspective = list(IDENTITY)
    perspective[14] = 0.5
    flat_ = list(IDENTITY)
    flat_[10] = 0.0
    ok = {"uid": "$uidOf:1"}
    out += [
        refused("Ölçekli matris: konum yalnız katı dönüşüm", dict(ok, place={"kind": "matrix", "matrix": scaled}),
                "invalid_input", RIGID, "place"),
        refused("Yansıyan matris: determinant eksi", dict(ok, place={"kind": "matrix", "matrix": mirror}),
                "invalid_input", RIGID, "place"),
        refused("15 sayılı matris", dict(ok, place={"kind": "matrix", "matrix": IDENTITY[:15]}), "invalid_input",
                "Matris 16 sayıdır (4 satır, 4 sütun, satır satır); 15 sayı verildi.", "place.matrix"),
        refused("Son satırı 0 0 0 1 olmayan matris", dict(ok, place={"kind": "matrix", "matrix": perspective}),
                "invalid_input", "Matrisin son satırı 0 0 0 1 olmalı: yalnız afin dönüşüm uygulanır (perspektif yok).",
                "place"),
        refused("Tekil matris", dict(ok, place={"kind": "matrix", "matrix": flat_}), "invalid_input",
                "Matrisin dönüklük kısmı tekil (determinantı sıfır): noktaları bir düzleme ya da doğruya ezer.", "place"),
        refused("Sıfır eksen", dict(ok, place={"kind": "axisAngle", "axis": [0.0, 0.0, 0.0], "angle": 10.0}),
                "invalid_input", "Dönme ekseninin uzunluğu sıfır olamaz; ekseni üç sayıyla yazın (Z için 0 0 1).", "place"),
        refused("Sapma NaN", dict(ok, place={"kind": "euler", "yaw": 0.0, "pitch": 0.0, "roll": 0.0}), "not_finite",
                "Sapma sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "place.yaw",
                non_finite={"place.yaw": "NaN"}),
        refused("Matrisin bir sayısı sonsuz", dict(ok, place={"kind": "matrix", "matrix": IDENTITY}), "not_finite",
                "Matrisin bir sayısı sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "place.matrix",
                non_finite={"place.matrix[3]": "Infinity"}),
        refused("Geçersiz kimlik", {"uid": "bulut", "place": {"kind": "remove"}}, "invalid_uid",
                "“bulut” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "uid"),
        refused("Çizimde olmayan nesne", {"uid": uid(99), "place": {"kind": "remove"}}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nokta "
                "bulutunun kimliğini verin.", "uid"),
        refused("Çizgi: nokta bulutu değil", {"uid": "$uidOf:5", "place": {"kind": "remove"}}, "not_a_pointcloud",
                "Verilen nesne bir nokta bulutu değil; yalnız nokta bulutu konumlanır. Bir nokta bulutunun kimliğini "
                "verin.", "uid"),
        refused("Kilitli katmandaki bulut", {"uid": "$uidOf:4", "place": {"kind": "remove"}}, "layer_locked",
                "“Kilitli” katmanı kilitli; üzerindeki bulut konumlanamaz. Kilidi Katmanlar panelinden açın.", "uid"),
        refused("Beklenen sürüm geçersiz", dict(ok, place={"kind": "remove"}, expectedRevision="dün"), "invalid_revision",
                "Beklenen sürüm “dün” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya "
                "da komutun planından alın.", "expectedRevision"),
        refused("Beklenen sürüm eski: çakışma", dict(ok, place={"kind": "remove"}, expectedRevision="999"),
                "revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin "
                "şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", status="conflict"),
    ]
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.pointclouds.place", "commandVersion": 1,
           "title": "Bulutu konumla",
           "note": "Written by scripts/fixtures/pointcloud_place_cases.py from docs/adr/5100 (OGC GeoPose 1.0 Basic-YPR; "
                   "Euler as Rz·Ry·Rx, axis and angle by quaternion, inverse by Rᵀ); do not edit by hand.",
           "setup": SETUP, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        same = OUT.exists() and OUT.read_text() == text
        print("aynı" if same else f"farklı: {OUT.relative_to(ROOT)}")
        sys.exit(0 if same else 1)
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")


if __name__ == "__main__":
    main()
