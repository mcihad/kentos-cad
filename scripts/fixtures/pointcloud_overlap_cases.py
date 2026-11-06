#!/usr/bin/env python3
"""The shared cases of `cad.pointclouds.overlap` (Tarama uyumu denetimi; plan/map-cad.md MAP-13, docs/adr/1500), without
KentOS code.

    python3 scripts/fixtures/pointcloud_overlap_cases.py           # writes the clouds and the cases
    python3 scripts/fixtures/pointcloud_overlap_cases.py --check   # writes nothing; compares

The clouds are text clouds written here (fixtures/pointcloud/v1/files/tarama-*.xyz, 3 decimals): a sloped ground
z = 850 + 0.05·x + 0.02·y with ±2 cm noise, strip A over x 0–30 m, strip B over x 10–40 m shifted up 4 cm,
strip C as B shifted 25 cm (a strip out of place), strip D as B with a car on the road (+0.6 m over 4 m × 2 m). No
point lies within 1.5 mm of a
whole metre in x or y, so no cell boundary is in doubt.

The rules, by hand: a cell (i, j) = (floor(x / c), floor(y / c)) gets a cloud's points; z = c₀ + c₁u + c₂v by least
squares with u, v from the cell's centre, c₀ by Cramer's rule, none with fewer than `least` points or a determinant
≤ 1e-9·n³·(c/2)⁴; the difference c₀(B) − c₀(A) over the cells both fill; RMSDz, mean, population standard deviation,
largest |dz|, the 20 worst cells (|dz| down, then the key); around a control point the same within the reach.
"""

import copy
import json
import math
import random
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.pointclouds.overlap.json"
FILES = ROOT / "fixtures" / "pointcloud" / "v1" / "files"
RULES = json.loads((ROOT / "catalog/v1/rules/lidar-uyum.json").read_text("utf-8"))
X0, Y0 = 487000.0, 4420000.0


def uid(n):
    return f"0192f5c0-1500-7000-8000-{n:012x}"


def ground(x, y):
    return 850.0 + 0.05 * x + 0.02 * y


def clean(v):
    """Away from a whole metre by more than 1.5 mm."""
    f = v - math.floor(v)
    return 0.0015 < f < 0.9985


def strip(seed, x_from, x_to, shift, car=False):
    rnd = random.Random(seed)
    rows = []
    for _ in range(int((x_to - x_from) * 20 * 3)):
        x, y = rnd.uniform(x_from, x_to), rnd.uniform(0, 20)
        x, y = round(x, 3), round(y, 3)
        if not (clean(x) and clean(y)):
            continue
        z = ground(x, y) + shift + rnd.uniform(-0.02, 0.02)
        if car and 20.0 < x < 24.0 and 8.0 < y < 10.0:
            z += 0.6
        rows.append((round(X0 + x, 3), round(Y0 + y, 3), round(z, 3)))
    return rows


CLOUDS = {"tarama-a.xyz": strip(1500, 0, 30, 0.0), "tarama-b.xyz": strip(1501, 10, 40, 0.04),
          "tarama-c.xyz": strip(1502, 10, 40, 0.25), "tarama-d.xyz": strip(1501, 10, 40, 0.04, car=True)}


def text(rows):
    return "X Y Z\n" + "".join(f"{x:.3f} {y:.3f} {z:.3f}\n" for x, y, z in rows)


def bounds(rows):
    return [min(r[0] for r in rows), min(r[1] for r in rows), min(r[2] for r in rows),
            max(r[0] for r in rows), max(r[1] for r in rows), max(r[2] for r in rows)]


def cloud(id_, name, layer_="tarama"):
    rows = CLOUDS[name]
    b = bounds(rows)
    return {"kind": "pointcloud", "id": id_, "layerId": layer_, "attrs": {},
            "sources": [{"file": f"$fixture:pointcloud/v1/files/{name}", "format": "xyz", "count": len(rows),
                         "bounds": b}], "bounds": b, "count": len(rows), "srid": 5254,
            "style": {"render": "classification", "size": 2}}


CONTROL = [("K1", 5.3, 5.7), ("K2", 15.4, 12.6), ("K3", 25.7, 4.4), ("K4", 34.6, 15.3), ("K5", 22.2, 9.1)]


def control_point(id_, name, x, y, z, layer_="gnss"):
    return {"kind": "point", "id": id_, "layerId": layer_, "attrs": {"Ad": name}, "p": {"x": X0 + x, "y": Y0 + y},
            "z": z}


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def setup():
    ents = [cloud(1, "tarama-a.xyz"), cloud(2, "tarama-b.xyz"), cloud(3, "tarama-c.xyz"), cloud(4, "tarama-d.xyz")]
    for k, (n, x, y) in enumerate(CONTROL):
        ents.append(control_point(5 + k, n, x, y, round(ground(x, y), 3)))
    ents.append(control_point(10, "Kotsuz", 12.0, 3.0, 0.0))
    del ents[-1]["z"]
    return {"format": "kentos.document", "version": 2, "name": "Tarama uyumu",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("tarama", "Taramalar"), layer("gnss", "GNSS kontrol noktaları")],
            "activeLayer": "tarama", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


class Sums:
    def __init__(self):
        self.n = self.u = self.v = self.uu = self.uv = self.vv = self.z = self.uz = self.vz = 0.0

    def add(self, u, v, z):
        self.n += 1
        self.u += u
        self.v += v
        self.uu += u * u
        self.uv += u * v
        self.vv += v * v
        self.z += z
        self.uz += u * z
        self.vz += v * z

    def height(self, least, h):
        if self.n < least:
            return None

        def det3(a):
            return (a[0] * (a[4] * a[8] - a[5] * a[7]) - a[1] * (a[3] * a[8] - a[5] * a[6]) +
                    a[2] * (a[3] * a[7] - a[4] * a[6]))
        d = det3([self.n, self.u, self.v, self.u, self.uu, self.uv, self.v, self.uv, self.vv])
        if abs(d) <= 1e-9 * self.n ** 3 * h ** 4:
            return None
        return det3([self.z, self.u, self.v, self.uz, self.uu, self.uv, self.vz, self.uv, self.vv]) / d


def check(names, cell=2.0, least=4, control=None, radius=None, scale=1000, rules=None, classes=None):
    rules = rules or RULES
    radius = cell if radius is None else radius
    s = setup()
    controls = []
    if control:
        for e in s["entities"]:
            if e["kind"] == "point" and e["layerId"] == control and "z" in e and e["attrs"].get("Ad", "").strip():
                controls.append((e["attrs"]["Ad"].strip(), e["p"]["x"], e["p"]["y"], e["z"]))
    cells = [{}, {}]
    around = [[Sums(), Sums()] for _ in controls]
    taken, read = [0, 0], [len(CLOUDS[n]) for n in names]
    for k, name in enumerate(names):
        for x, y, z in CLOUDS[name]:
            if classes is not None and 0 not in classes:  # a text cloud's points are class 0
                continue
            taken[k] += 1
            i, j = math.floor(x / cell), math.floor(y / cell)
            cx, cy = (i + 0.5) * cell, (j + 0.5) * cell
            cells[k].setdefault((i, j), Sums()).add(x - cx, y - cy, z)
            for c, sums in zip(controls, around):
                u, v = x - c[1], y - c[2]
                if abs(u) <= radius and abs(v) <= radius and math.hypot(u, v) <= radius:
                    sums[k].add(u, v, z)
    h = cell / 2
    planes = [{key: s_.height(least, h) for key, s_ in cells[k].items()} for k in (0, 1)]
    filled = [sum(1 for p in planes[k].values() if p is not None) for k in (0, 1)]
    diffs, degenerate = [], 0
    for key in sorted(cells[0]):
        if key not in cells[1] or cells[0][key].n < least or cells[1][key].n < least:
            continue
        a, b = planes[0][key], planes[1][key]
        if a is None or b is None:
            degenerate += 1
        else:
            diffs.append((key, b - a))
    n = len(diffs)
    if n:
        mean = sum(d for _, d in diffs) / n
        rmsd = math.sqrt(sum(d * d for _, d in diffs) / n)
        std = math.sqrt(sum((d - mean) ** 2 for _, d in diffs) / n)
        mx = max(abs(d) for _, d in diffs)
    else:
        mean = rmsd = std = mx = 0.0
    worst = sorted(diffs, key=lambda kd: (-abs(kd[1]), kd[0]))[:20]
    S = float(scale)
    a_, b_, c_ = rules["xy"]
    limit_z = rules["zFactor"] * (a_ * S * S + b_ * S + c_) / 100.0
    ctrl = []
    for c, sums in zip(controls, around):
        o = {"name": c[0], "z": c[3]}
        for key, k in (("dzA", 0), ("dzB", 1)):
            p = sums[k].height(least, radius)
            if p is not None:
                o[key] = p - c[3]
        ctrl.append(o)

    def stats(key):
        if not controls:
            return None
        v = [o[key] for o in ctrl if key in o]
        koh = math.sqrt(sum(d * d for d in v) / len(v)) if v else 0.0
        out = {"n": len(v), "kohZ": koh, "mean": sum(v) / len(v) if v else 0.0}
        if v:
            out["passed"] = koh <= limit_z
        return out
    notes = ["Her hücrede bulutun noktalarına düzlem uydurulur; fark iki düzlemin hücre merkezindeki değerlerinin farkıdır "
             "(ikinci − birinci), eğim fark sayılmaz.",
             f"Şeritler arası sınır {rules['id']} paketinin örnek değeridir; Türk mevzuatında LiDAR şerit uyumu ölçütü yok, "
             "kurum kendi değerini verir."]
    if not controls:
        notes.append("Kontrol noktası yok: bulutların mutlak doğruluğu bilinmiyor; bulut ölçüye kesin hazır sayılmaz.")
    if not diffs:
        notes.append("Örtüşen hücre yok: iki bulut aynı yeri yeterli noktayla kaplamıyor.")
    out = {"rules": rules["id"], "source": rules["source"], "cell": cell, "read": read, "taken": taken,
           "filled": filled, "cells": n, "area": n * cell * cell, "degenerate": degenerate, "rmsdZ": rmsd,
           "mean": mean, "std": std, "max": mx, "limit": rules["interswath"]}
    if n:
        out["passed"] = rmsd <= rules["interswath"]
    out["worst"] = [{"x": (i + 0.5) * cell, "y": (j + 0.5) * cell, "dz": d} for (i, j), d in worst]
    out["control"] = ctrl
    sa, sb = stats("dzA"), stats("dzB")
    if sa is not None:
        out["controlA"] = sa
    if sb is not None:
        out["controlB"] = sb
    out["limitZ"] = limit_z
    out["notes"] = notes
    return out


SAME = {"revision": "same", "canUndo": False}
IDS = {"tarama-a.xyz": 1, "tarama-b.xyz": 2, "tarama-c.xyz": 3, "tarama-d.xyz": 4}


def ok(name, clouds=("tarama-a.xyz", "tarama-b.xyz"), op="plan", **kw):
    inp = {"uids": [uid(IDS[c]) for c in clouds], **kw}
    args = {k: v for k, v in kw.items() if k in ("cell", "least", "control", "radius", "scale", "rules", "classes")}
    out = check(list(clouds), **args)
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}


def refused(name, inp, code, message, path_):
    return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}


def cases():
    base = check(["tarama-a.xyz", "tarama-b.xyz"], control="gnss")
    assert base["passed"] and abs(base["mean"] - 0.04) < 0.02, (base["rmsdZ"], base["mean"])
    bad = check(["tarama-a.xyz", "tarama-c.xyz"], control="gnss")
    assert not bad["passed"], bad["rmsdZ"]
    other = dict(copy.deepcopy(RULES), id="deneme-3cm", source="Deneme paketi (sentetik): 3 cm", interswath=0.03)
    two = [uid(1), uid(2)]
    car = check(["tarama-a.xyz", "tarama-d.xyz"])
    assert not car["passed"] and all(20 <= w["x"] - X0 <= 24 and 8 <= w["y"] - Y0 <= 10 for w in car["worst"][:2]), car["worst"][:3]
    return [
        ok("A ile B: eğimli zemin, 4 cm sistematik kayma; GNSS kontrolüyle", control="gnss"),
        ok("A ile D: yoldaki araç (örtülmüş yol) en kötü hücrelerde, sınırı aşar", ("tarama-a.xyz", "tarama-d.xyz")),
        ok("uygula: salt okunur", op="execute", control="gnss"),
        ok("doğrulama", op="validate", control="gnss"),
        ok("kontrolsüz: mutlak doğruluk bilinmez notu"),
        ok("A ile C: 25 cm kaymış şerit sınırı aşar", ("tarama-a.xyz", "tarama-c.xyz"), control="gnss"),
        ok("B ile A: sıra farkın işaretini çevirir", ("tarama-b.xyz", "tarama-a.xyz")),
        ok("1 m hücre", cell=1.0),
        ok("5 m hücre", cell=5.0),
        ok("hücrede en az 20 nokta", least=20),
        ok("kontrol çevresi 1 m", control="gnss", radius=1.0),
        ok("1/500: KOH_Z sınırı daha sıkı", control="gnss", scale=500),
        ok("başka kural paketi: 3 cm", rules=other),
        ok("yalnız sınıf 2: metin bulutu sınıf 0, nokta kalmaz", classes=[2]),
        ok("sınıf 0 süzgeci bütün noktaları alır", classes=[0]),
        ok("B ile C: örtüşen ama kaymış", ("tarama-b.xyz", "tarama-c.xyz"), control="gnss"),
        refused("tek bulut", {"uids": [uid(1)]}, "invalid_input",
                "İki ayrı nokta bulutu verin: karşılaştırılacak iki tarama.", "uids"),
        refused("aynı bulut iki kez", {"uids": [uid(1), uid(1)]}, "invalid_input",
                "İki ayrı nokta bulutu verin: karşılaştırılacak iki tarama.", "uids"),
        refused("bulut değil", {"uids": [uid(1), uid(5)]}, "not_a_pointcloud",
                "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.", "uids[1]"),
        refused("kimlik UUID değil", {"uids": [uid(1), "x"]}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da "
                "çizimden alın.", "uids[1]"),
        refused("çizimde olmayan bulut", {"uids": [uid(1), uid(99)]}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir "
                "nesnenin kimliğini verin.", "uids[1]"),
        refused("hücre 0", {"uids": two, "cell": 0.0}, "invalid_input",
                "Hücre kenarı sonlu ve sıfırdan büyük olmalı (yazılan 0).", "cell"),
        refused("en az 2 nokta", {"uids": two, "least": 2}, "invalid_input",
                "Bir düzlem için hücrede en az 3 nokta gerekir.", "least"),
        refused("kontrol katmanı yok", {"uids": two, "control": "yok"}, "layer_not_found",
                "“yok” katmanı çizimde yok.", "control"),
        refused("ölçek 0", {"uids": two, "scale": 0}, "invalid_input", "Ölçek paydası 0 olamaz.", "scale"),
        refused("kural paketinin kaynağı boş", {"uids": two, "rules": dict(copy.deepcopy(RULES), source="")},
                "invalid_input", "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan denetim "
                "yapılmaz.", "rules"),
    ]


def main():
    check_only = "--check" in sys.argv[1:]
    stale = []
    for name, rows in CLOUDS.items():
        path = FILES / name
        t = text(rows)
        if check_only:
            if not path.exists() or path.read_text("utf-8") != t:
                stale.append(name)
        else:
            path.write_text(t, "utf-8")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.pointclouds.overlap", "commandVersion": 1,
           "title": "Tarama uyumu denetimi",
           "note": "ADR 1500. Bağımsız başvuru scripts/fixtures/pointcloud_overlap_cases.py: düzlem uydurma ve "
                   "istatistik elle; bulutlar sentetik metin bulutlarıdır; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    out = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if check_only:
        ok_ = not stale and OUT.exists() and OUT.read_text("utf-8") == out
        print("Tarama uyumu ortak durumları güncel." if ok_ else f"güncel değil: {stale or OUT.relative_to(ROOT)}")
        return 0 if ok_ else 1
    OUT.write_text(out, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum), {len(CLOUDS)} bulut")
    return 0


if __name__ == "__main__":
    sys.exit(main())
