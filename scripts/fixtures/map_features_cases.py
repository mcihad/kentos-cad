#!/usr/bin/env python3
"""The shared cases of `map.features.code` (Kodla çiz; plan/map-cad.md MAP-04, docs/adr/1360), written from the ADR
without KentOS code.

    python3 scripts/fixtures/map_features_cases.py           # writes fixtures/commands/v1/map.features.code.json
    python3 scripts/fixtures/map_features_cases.py --check   # writes nothing; compares

The drawing is a map project with the KentOS profile `harita-varliklari`'s layers for the classes the cases name, a
survey layer of coded points (measured in this order), a hidden and a locked layer. The rules, written here by hand:

- a point's code is its `Kod` field (or the field given), trimmed; empty: not coded. Its first word upper-cased is
  capital letters then digits: the letters are the code, the digits the string's number; anything else is a code no
  class has. The words after it, upper-cased, are controls: B, K, S; others are reported once each;
- the class is the profile's class whose `codes` hold the code; none: `unknown_code`, the points stay;
- a point class's point goes on the class's layer (moved when it is on another);
- a line or area class's points join strings by (class, number), in the points' order: B ends an open string first and
  begins a new one with the point; the point joins its string; K then closes and ends it, else S ends it; strings still
  open end after the last point, in the order they were begun;
- an ended line string of at least 2 points (3 when closed) is a polyline (closed with K), an area string closed with K
  of at least 3 points a polygon; elevations only when every point has one; the new object's `Kod` is its first point's
  first word; otherwise `string_too_short` or `area_not_closed`;
- without `uids`: every point on a shown, unlocked layer, in the drawing's order. One undo step “Kodla çiz”; nothing to
  move or make writes nothing.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "map.features.code.json"
PROFILE = json.loads((ROOT / "catalog/v1/profiles/harita-varliklari.json").read_text("utf-8"))
X0, Y0 = 500000.0, 4400000.0
G = "Harita varlıkları"

# The class layers the drawing has (id, the class's layer name).
CLASS_LAYERS = {"kontrol-noktasi": "hv-kn", "rogar": "hv-rog", "agac": "hv-agc", "bordur": "hv-brd",
                "bina-izi": "hv-bna", "yesil-alan": "hv-ysl"}


def layer(id_, name, kind="layer", visible=True, locked=False, children=()):
    return {"id": id_, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": list(children)}


def class_layer(cid):
    c = next(c for c in PROFILE["classes"] if c["id"] == cid)
    return layer(CLASS_LAYERS[cid], c["layer"][-1])


def uid(n):
    return f"0192f5c0-1360-7000-8000-{n:012x}"


# The coded points, in the order they were measured: (code text, east, north, z or None, layer).
POINTS = [
    ("KN", 0, 0, 850.10, "olcum"),          # 1 a control point
    ("ROG", 10, 0, 850.20, "olcum"),        # 2 a manhole
    ("BRD1", 0, 10, 850.30, "olcum"),       # 3 kerb 1 begins
    ("BRD1", 10, 10, 850.40, "olcum"),      # 4
    ("BRD2", 0, 20, None, "olcum"),         # 5 kerb 2 interleaved
    ("BRD1", 20, 12, 850.50, "olcum"),      # 6 kerb 1 goes on
    ("BRD2", 10, 22, None, "olcum"),        # 7
    ("BNA", 30, 0, 851.0, "olcum"),         # 8 a building's corners
    ("bna", 40, 0, 851.0, "olcum"),         # 9 lower case reads the same
    ("BNA", 40, 10, 851.0, "olcum"),        # 10
    ("BNA K", 30, 10, 851.0, "olcum"),      # 11 closed
    ("AGC", 50, 50, None, "olcum"),         # 12 a tree
    ("XYZ", 60, 60, None, "olcum"),         # 13 a code no class has
    ("AGC", 55, 50, None, "hv-agc"),        # 14 a tree already on its layer
    ("", 70, 70, None, "olcum"),            # 15 not coded
    ("ROG", 80, 0, None, "gizli"),          # 16 on a hidden layer
    ("ROG", 90, 0, None, "kilitli"),        # 17 on a locked layer
]


def point_entity(i, text, x, y, z, layer_id):
    e = {"kind": "point", "id": i, "layerId": layer_id, "attrs": {"Ad": f"P{i}"} | ({"Kod": text} if text else {}),
         "p": {"x": X0 + x, "y": Y0 + y}}
    if z is not None:
        e["z"] = z
    return e


def setup(points=POINTS, locked_class=False, missing=()):
    kids = [class_layer(c) for c in CLASS_LAYERS if c not in missing]
    if locked_class:
        kids = [dict(k, locked=True) if k["id"] == "hv-brd" else k for k in kids]
    layers = [layer("0", "0"), layer("hv", G, "group", children=kids), layer("olcum", "Ölçüm"),
              layer("gizli", "Gizli", visible=False), layer("kilitli", "Kilitli", locked=True)]
    ents = [point_entity(i + 1, *p) for i, p in enumerate(points)]
    return {"format": "kentos.document", "version": 2, "name": "Kodla çiz",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "olcum", "entities": ents,
            "uids": [uid(i + 1) for i in range(len(ents))], "styles": {"items": [], "categories": []}}


# ── The rules ────────────────────────────────────────────────────────────────────────────────────────────────────────


def split(text):
    words = text.split()
    if not words:
        return None
    first = words[0].upper()
    letters = ""
    for ch in first:
        if "A" <= ch <= "Z":
            letters += ch
        else:
            break
    digits = first[len(letters):]
    if not letters or not all("0" <= c <= "9" for c in digits):
        return None
    return letters, digits, [w.upper() for w in words[1:]], first


def code_points(d, uids=None, field="Kod"):
    """(rows, moves {slot: layer}, makes [(layer, geometry dict, token)], unknown [(code, n)], warnings) or a refusal."""
    layers = {}

    def walk(nodes, parent_visible=True, parent_locked=False):
        for n in nodes:
            v, l = n["visible"] and parent_visible, n["locked"] or parent_locked
            layers[n["id"]] = (v, l, n["name"])
            walk(n["children"], v, l)
    walk(d["layers"])
    by_uid = dict(zip(d["uids"], d["entities"]))
    if uids is None:
        pts = [e for e in d["entities"] if e["kind"] == "point" and layers[e["layerId"]][0] and not layers[e["layerId"]][1]]
    else:
        pts = [by_uid[u] for u in uids]
    coded, unknown, controls_bad = [], {}, []
    for e in pts:
        text = e["attrs"].get(field, "").strip()
        if not text:
            continue
        s = split(text)
        if s is None:
            raw = text.split()[0].upper()
            unknown[raw] = unknown.get(raw, 0) + 1
            continue
        code, number, controls, token = s
        ci = next((i for i, c in enumerate(PROFILE["classes"]) if code in c.get("codes", [])), None)
        if ci is None:
            unknown[code] = unknown.get(code, 0) + 1
            continue
        for c in controls:
            if c not in ("B", "K", "S") and c not in controls_bad:
                controls_bad.append(c)
        coded.append((e, ci, number, controls, token))
    if not coded:
        return ("no_coded_points", f"“{field}” alanında profilin bir sınıfının kodunu taşıyan nokta yok; noktaların kodunu "
                                   "yazın ya da alanın adını verin.", "uids")
    named = sorted({ci for _, ci, _, _, _ in coded})
    layer_of = {}
    for ci in named:
        c = PROFILE["classes"][ci]
        lid = CLASS_LAYERS.get(c["id"])
        present = lid is not None and lid in layers
        if not present:
            return ("class_layer_missing", f"“{c['title']}” sınıfının katmanı ({' › '.join(c['layer'])}) çizimde yok; önce "
                                           f"“{PROFILE['name']}” profilini uygulayın.", "name")
        layer_of[ci] = lid
    rows = {ci: {"class": PROFILE["classes"][ci]["id"], "title": PROFILE["classes"][ci]["title"], "layer": layer_of[ci],
                 "points": 0, "moved": 0, "lines": 0, "areas": 0} for ci in named}
    moves, makes = {}, []
    opened, strings = [], {}
    short = unclosed = 0

    def finish(key, closed):
        nonlocal short, unclosed
        ci, members = key[0], strings.pop(key)
        geom = PROFILE["classes"][ci]["geometry"]
        pts = [{"x": m["p"]["x"], "y": m["p"]["y"]} for m in members]
        zs = [m["z"] for m in members] if all("z" in m for m in members) else None
        token = split(members[0]["attrs"][field].strip())[3]
        if geom == "line":
            if len(pts) < 2 or (closed and len(pts) < 3):
                short += 1
                return
            g = {"kind": "polyline", "pts": pts}
            if closed:
                g["closed"] = True
            rows[ci]["lines"] += 1
        else:
            if not closed:
                unclosed += 1
                return
            if len(pts) < 3:
                short += 1
                return
            g = {"kind": "polygon", "pts": pts}
            rows[ci]["areas"] += 1
        if zs is not None:
            g["zs"] = zs
        makes.append((layer_of[ci], g, token))

    for e, ci, number, controls, token in coded:
        rows[ci]["points"] += 1
        geom = PROFILE["classes"][ci]["geometry"]
        if geom == "point":
            if e["layerId"] != layer_of[ci]:
                if layers[e["layerId"]][1]:
                    return ("layer_locked", f"“{layers[e['layerId']][2]}” katmanı kilitli: üstündeki kodlu nokta sınıfının "
                                            "katmanına taşınamaz. Kilidi açın.", "uids")
                moves[e["id"]] = layer_of[ci]
                rows[ci]["moved"] += 1
            continue
        key = (ci, number)
        if "B" in controls and key in strings:
            finish(key, False)
        if key not in strings:
            opened = [k for k in opened if k != key] + [key]
            strings[key] = []
        strings[key].append(e)
        if "K" in controls:
            finish(key, True)
        elif "S" in controls:
            finish(key, False)
    for key in opened:
        if key in strings:
            finish(key, False)
    for lid, _, _ in makes:
        if layers[lid][1]:
            return ("layer_locked", f"“{G} / {layers[lid][2]}” katmanı kilitli: Kodla çiz oraya çizemez. Kilidi açın.", "name")
    warnings = []
    for code, n in unknown.items():
        warnings.append({"code": "unknown_code", "message": f"“{code}” kodu “{PROFILE['name']}” profilinin hiçbir "
                                                            f"sınıfında yok; {n} nokta olduğu gibi bırakıldı."})
    if controls_bad:
        warnings.append({"code": "control_unknown", "message": ", ".join(f"“{c}”" for c in controls_bad)
                         + " denetim kodu değil (B başla, K kapat, S son); yok sayıldı."})
    if short:
        warnings.append({"code": "string_too_short", "message": f"{short} dizi çizilmedi: çizgi için en az 2, kapalı çizgi "
                                                                 "ve alan için en az 3 nokta gerekir."})
    if unclosed:
        warnings.append({"code": "area_not_closed", "message": f"{unclosed} alan dizisi K ile kapatılmadığı için alan "
                                                                "çizilmedi."})
    ordered = [rows[ci] for ci in named]
    return ordered, moves, makes, [{"code": c, "points": n} for c, n in unknown.items()], warnings


# ── Steps ────────────────────────────────────────────────────────────────────────────────────────────────────────────

SAME = {"revision": "same", "canUndo": False}


def run(d, input_, op="execute"):
    r = code_points(d, input_.get("uids"), input_.get("field", "Kod"))
    if isinstance(r[0], str):
        code, message, path = r
        return {"op": op, "input": input_, "result": {"status": "failed", "error": {"code": code, "message": message,
                                                                                        "path": path}}, "expect": SAME}
    rows, moves, makes, unknown, warnings = r
    out = {"classes": rows, "revision": "$current"}
    if unknown:
        out["unknown"] = unknown
    if op != "execute":
        result = {"status": "completed", "output": None if op == "validate" else out, "warnings": warnings}
        return {"op": op, "input": input_, "result": result, "expect": SAME}
    if makes:
        out["created"] = "$any"
    n = len(d["entities"])
    fields = {str(slot): {"layerId": lid} for slot, lid in moves.items()}
    for k, (lid, g, token) in enumerate(makes):
        fields[str(n + 1 + k)] = dict(g, layerId=lid, attrs={"Kod": token})
    # Nothing to move and nothing to make: nothing written, no undo step.
    expect = {"ids": list(range(1, n + 1 + len(makes))), "fields": fields, "revision": "changed", "canUndo": True} \
        if moves or makes else SAME
    return {"op": "execute", "input": input_, "result": {"status": "completed", "output": out, "warnings": warnings},
            "expect": expect}


def case(name, d, input_, op="execute", undo=False, note=None):
    steps = [run(d, input_, op)]
    if undo:
        steps.append({"op": "undo", "returns": "Kodla çiz",
                      "expect": {"ids": list(range(1, len(d["entities"]) + 1)), "canRedo": True}})
    c = {"name": name}
    if note:
        c["note"] = note
    c["setup"] = d
    c["steps"] = steps
    return c


def cases():
    base = setup()
    P = [("BRD", 0, 0, None, "olcum"), ("BRD", 10, 0, None, "olcum"), ("BRD B", 20, 0, None, "olcum"),
         ("BRD", 30, 0, None, "olcum")]
    out = [
        case("Bütün kodlu noktalar: kontrol noktası, rögar ve ağaç katmanlarına; iki bordür dizisi, kapalı bina izi; "
             "bilinmeyen kod uyarıyla kalır; gizli ve kilitli katmandakiler alınmaz; tek adım", base,
             {"name": "harita-varliklari"}, undo=True),
        case("Plan aynı sınıfları söyler, yazmaz", base, {"name": "harita-varliklari"}, op="plan"),
        case("Doğrulama yazmaz, uyarıları söyler", base, {"name": "harita-varliklari"}, op="validate"),
        case("Verilen noktalar verilen sırayla: bordür dizisi ters sırada", base,
             {"name": "harita-varliklari", "uids": [uid(6), uid(4), uid(3)]}),
        case("B yeni dizi başlatır: iki bordür", setup(P), {"name": "harita-varliklari"}),
        case("S diziyi bitirir; sonraki nokta yeni dizi açar ve tek noktası kısa kalır",
             setup([("BRD", 0, 0, None, "olcum"), ("BRD S", 10, 0, None, "olcum"), ("BRD", 20, 0, None, "olcum")]),
             {"name": "harita-varliklari"}),
        case("Çizgi sınıfında K kapalı çoklu çizgi",
             setup([("BRD", 0, 0, 1.0, "olcum"), ("BRD", 10, 0, 2.0, "olcum"), ("BRD K", 10, 10, 3.0, "olcum")]),
             {"name": "harita-varliklari"}),
        case("Alan K ile kapatılmazsa çizilmez",
             setup([("BNA", 0, 0, None, "olcum"), ("BNA", 10, 0, None, "olcum"), ("BNA", 10, 10, None, "olcum")]),
             {"name": "harita-varliklari"}),
        case("İki noktalık alan dizisi kısa",
             setup([("YSL", 0, 0, None, "olcum"), ("YSL K", 10, 0, None, "olcum")]), {"name": "harita-varliklari"}),
        case("Bir noktası kotsuz dizinin köşeleri kotsuz",
             setup([("BRD", 0, 0, 1.0, "olcum"), ("BRD", 10, 0, None, "olcum")]), {"name": "harita-varliklari"}),
        case("Bilinmeyen denetim kodu bir kez söylenir",
             setup([("BRD X", 0, 0, None, "olcum"), ("BRD X", 10, 0, None, "olcum"), ("BRD Y", 20, 0, None, "olcum")]),
             {"name": "harita-varliklari"}),
        case("Rakamla başlayan kod sınıfsızdır", setup([("1BRD", 0, 0, None, "olcum"), ("KN", 5, 5, None, "olcum")]),
             {"name": "harita-varliklari"}),
        case("Zaten sınıfının katmanındaki nokta taşınmaz", setup([("AGC", 0, 0, None, "hv-agc")]),
             {"name": "harita-varliklari"}),
        case("Kod başka alanda: field", setup_field(), {"name": "harita-varliklari", "field": "SahaKodu"}),
        refused("Kodlu nokta yok", setup([("", 0, 0, None, "olcum")]), {"name": "harita-varliklari"}),
        refused("Yalnız bilinmeyen kod: kodlu nokta yok", setup([("XYZ", 0, 0, None, "olcum")]), {"name": "harita-varliklari"}),
        refused("Sınıfın katmanı çizimde yok", setup([("BRD", 0, 0, None, "olcum"), ("BRD", 1, 0, None, "olcum")],
                                                     missing=("bordur",)), {"name": "harita-varliklari"}),
        refused("Sınıfın katmanı kilitli", setup([("BRD", 0, 0, None, "olcum"), ("BRD", 1, 0, None, "olcum")],
                                                 locked_class=True), {"name": "harita-varliklari"}),
        refused("Kilitli katmandaki verilen nokta taşınamaz", base, {"name": "harita-varliklari", "uids": [uid(17)]}),
    ]
    out.append({"name": "Olmayan profil", "setup": base, "steps": [{"op": "execute", "input": {"name": "yok"}, "result": {
        "status": "failed", "error": {"code": "unknown_profile", "message":
                                      "“yok” adlı veri profili yok; KentOS'un profilleri: tucbs-temel, harita-varliklari.",
                                      "path": "name"}}, "expect": SAME}]})
    out.append({"name": "Boş alan adı", "setup": base, "steps": [{"op": "execute", "input": {"name": "harita-varliklari",
                "field": " "}, "result": {"status": "failed", "error": {"code": "invalid_input", "message":
                                          "Kod alanının adı boş olamaz.", "path": "field"}}, "expect": SAME}]})
    out.append({"name": "Nokta olmayan nesne", "setup": with_line(), "steps": [{"op": "execute", "input": {
        "name": "harita-varliklari", "uids": [uid(2)]}, "result": {"status": "failed", "error": {
            "code": "not_a_point", "message": f"“{uid(2)}” bir nokta değil; Kodla çiz kodlu noktaları işler.",
            "path": "uids[0]"}}, "expect": SAME}]})
    out.append({"name": "Çizimde olmayan nesne", "setup": base, "steps": [{"op": "execute", "input": {
        "name": "harita-varliklari", "uids": [uid(999)]}, "result": {"status": "failed", "error": {
            "code": "entity_not_found", "message": f"“{uid(999)}” kimlikli nesne çizimde yok.", "path": "uids[0]"}},
        "expect": SAME}]})
    out.append({"name": "Kimlik UUID değil", "setup": base, "steps": [{"op": "execute", "input": {
        "name": "harita-varliklari", "uids": ["P1"]}, "result": {"status": "failed", "error": {
            "code": "invalid_uid", "message": "“P1” geçerli bir nesne kimliği değil.", "path": "uids[0]"}},
        "expect": SAME}]})
    out.append({"name": "Sürüm yazısı geçersiz", "setup": base, "steps": [{"op": "execute", "input": {
        "name": "harita-varliklari", "expectedRevision": "abc"}, "result": {"status": "failed", "error": {
            "code": "invalid_revision", "message": "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                                                   "yazısıdır. Sürümü belgeden ya da komutun planından alın.",
            "path": "expectedRevision"}}, "expect": SAME}]})
    out.append({"name": "Beklenen sürüm eski: çatışma, hiçbir şey yazılmaz", "setup": base, "steps": [{"op": "execute",
                "input": {"name": "harita-varliklari", "expectedRevision": "7"}, "result": {"status": "conflict", "error": {
                    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. "
                                                            "Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                    "path": "expectedRevision", "revision": "$current"}}, "expect": SAME}]})
    return out


def setup_field():
    d = setup([("ROG", 0, 0, None, "olcum")])
    e = d["entities"][0]
    e["attrs"] = {"Ad": "P1", "SahaKodu": "ROG", "Kod": "XYZ"}
    return d


def with_line():
    d = setup([("ROG", 0, 0, None, "olcum")])
    d["entities"].append({"kind": "line", "id": 2, "layerId": "olcum", "attrs": {},
                          "a": {"x": X0, "y": Y0}, "b": {"x": X0 + 1, "y": Y0}})
    d["uids"].append(uid(2))
    return d


def refused(name, d, input_):
    return {"name": name, "setup": d, "steps": [run(d, input_)]}


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "map.features.code", "commandVersion": 1,
           "title": "Kodla çiz",
           "note": "ADR 1360. Bağımsız başvuru scripts/fixtures/map_features_cases.py: kodun okunuşu, dizilerin "
                   "birleştirilmesi ve iletiler ADR'den elle; profil catalog/v1/profiles/harita-varliklari.json; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print("Kodla çiz ortak durumları güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
