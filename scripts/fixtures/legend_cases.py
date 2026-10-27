"""The shared cases of cad.legends.materials (Malzeme lejandı, docs/adr/0721 §5).

    python3 scripts/fixtures/legend_cases.py           # writes the file
    python3 scripts/fixtures/legend_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.legends.materials.json. The checks, their order, codes, paths and messages are written
here by hand from the ADR; the layer's and the revision's are every create command's. What the command makes is worked
out here without KentOS code:

- the rows: the walls in the drawing's order, each one's components from its left face; a row's name is the
  component's material (when not blank), else its own name; a component whose name and hatch are a row's already is
  that row, else a new one; a row's count is the walls that have it;
- the block “Malzeme lejandı” (made again in its place when the drawing has it), in the text's height h on paper times
  the drawing's scale's denominator over 1000 (H): the title “Malzeme lejandı” H high, its top left at the base point;
  row i's swatch a closed path 6H wide and 3H high, its top 2H + 4H·i below, then its hatch over the same four corners
  as the walls hatch it (a library pattern at its angle and its size times the scale over 1000, read from
  hatch_pattern_cases.py's library; “solid” filled; another name none), its colour the hatch's, then its name H high,
  its left middle 7H right, level with the swatch's middle; the objects numbered from 1 in that order;
- with `at`, an insert of it there, scale 1, unturned. One undo step “Malzeme lejandı”.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)
import hatch_pattern_cases as hatches  # noqa: E402  (the pattern library's reference)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
PLOT = 50
LIB = {p["name"].upper(): p for p in hatches.library()}
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False, kind="layer", children=()):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True, "style": style,
            "children": list(children)}


def plain(v):
    return int(v) if float(v).is_integer() else v


def P(x, y):
    return {"x": plain(x), "y": plain(y)}


def comp(name, thickness, material=None, hatch=None):
    c = {"name": name}
    if material is not None:
        c["material"] = material
    c["thickness"] = thickness
    if hatch is not None:
        c["hatch"] = hatch
    c["detail"] = "standard"
    return c


SIVA = comp("Sıva", 0.02, "Kireç sıva")
TUGLA = comp("Tuğla", 0.19, "Tuğla", {"pattern": "ANSI31", "scale": 1.0, "angle": 45.0})
TUGLA_0 = comp("Tuğla", 0.19, "Tuğla", {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0})
GAZBETON = comp("Gazbeton", 0.2, "Gazbeton", {"pattern": "ANSI37", "scale": 0.5, "angle": 0.0, "color": "#7A7A7A"})
YALITIM = comp("Isı yalıtımı", 0.05, "  ", {"pattern": "solid", "scale": 1.0, "angle": 0.0, "color": "#E5E5E5"})
BETON = comp("Betonarme", 0.2, "Betonarme", {"pattern": "KAYIP", "scale": 1.0, "angle": 0.0})


def wall(eid, y, components):
    w = {"kind": "wall", "id": eid, "layerId": "duvar", "attrs": {}, "pts": [P(0, y), P(5, y)], "width": 0.25,
         "justify": "center"}
    if components:
        w["components"] = components
    return w


ENTITIES = [
    wall(1, 0, [SIVA, TUGLA, SIVA]),
    wall(2, 2, [SIVA, TUGLA, SIVA]),
    wall(3, 4, [GAZBETON, SIVA]),
    wall(4, 6, []),
    wall(5, 8, [YALITIM, TUGLA_0]),
    wall(6, 10, [BETON]),
]
IDS = [e["id"] for e in ENTITIES]
NEXT = 7
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Malzeme lejandı",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": PLOT,
                 "workspace": "cad", "drawingFont": "barlow"},
    "origin": {"x": 0, "y": 0},
    "layers": [layer("duvar", "Duvar"), layer("lejant", "Lejant"), layer("kilitli", "Kilitli", locked=True),
               layer("gizli", "Gizli", visible=False), layer("grup", "Grup", kind="group", children=[layer("alt", "Alt")])],
    "activeLayer": "duvar", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
PLAIN = copy.deepcopy(SETUP)
PLAIN["name"] = "Bileşensiz duvarlar"
PLAIN["entities"] = [wall(1, 0, []), wall(2, 2, [])]
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
HIDDEN = "“Gizli” katmanı gizli; çizilen nesne görünmeyecek."
NO_MATERIALS = "Çizimde bileşenli duvar yok: lejanda yazılacak malzeme yok. Duvarlara bileşenli bir duvar stili verin."
NO_LEGEND = "Çizimde malzeme lejandı yok: yeniden yapılacak bir şey yok. Lejandın yerini (at) verin."
ZERO_BLOCK = "00000000-0000-0000-0000-000000000000"


def rows(entities):
    out = []
    for e in entities:
        if e["kind"] != "wall":
            continue
        seen = set()
        for c in e.get("components", []):
            name = c["material"] if c.get("material", "").strip() else c["name"]
            hatch = c.get("hatch")
            at = next((i for i, r in enumerate(out) if r["name"] == name and r["hatch"] == hatch), None)
            if at is None:
                out.append({"name": name, "hatch": hatch, "walls": 0})
                at = len(out) - 1
            if at not in seen:
                seen.add(at)
                out[at]["walls"] += 1
    return out


def materials(rs):
    out = []
    for r in rs:
        m = {"name": r["name"]}
        if r["hatch"] is not None:
            m["pattern"] = r["hatch"]["pattern"]
        m["walls"] = r["walls"]
        out.append(m)
    return out


def swatch_pattern(h):
    if h["pattern"].lower() == "solid":
        return {"type": "solid", "angle": 0.0, "spacing": 1.0}
    found = LIB.get(h["pattern"].upper())
    if found is None:
        return None
    return hatches.pattern_of(found["name"], h["angle"], h["scale"] * PLOT / 1000.0)


def block(rs, h=2.5):
    big = h * PLOT / 1000.0
    ents = []

    def push(e):
        ents.append({**e, "id": len(ents) + 1, "layerId": "", "attrs": {}})

    push({"kind": "text", "p": P(0.0, 0.0), "text": "Malzeme lejandı", "height": big, "rotation": 0.0, "align": "topLeft"})
    for i, r in enumerate(rs):
        top = -(2.0 * big + 4.0 * big * i)
        bottom = top - 3.0 * big
        ring = [P(0.0, bottom), P(6.0 * big, bottom), P(6.0 * big, top), P(0.0, top)]
        push({"kind": "polyline", "pts": ring, "closed": True})
        pattern = swatch_pattern(r["hatch"]) if r["hatch"] is not None else None
        if pattern is not None:
            e = {"kind": "hatch", "ring": ring, "pattern": pattern}
            if r["hatch"].get("color") is not None:
                e["color"] = r["hatch"]["color"]
            push(e)
        push({"kind": "text", "p": P(7.0 * big, top - 1.5 * big), "text": r["name"], "height": big, "rotation": 0.0,
              "align": "middleLeft"})
    return {"name": "Malzeme lejandı", "base": P(0, 0), "entities": ents,
            "description": "Duvarların malzemeleri: her birinin taraması ve adı (docs/adr/0721)."}


def insert(eid, at, layer_id="lejant", block_id="$blockOf:Malzeme lejandı"):
    return {"kind": "insert", "id": eid, "layerId": layer_id, "attrs": {}, "block": block_id, "p": P(*at), "scale": 1,
            "rotation": 0}


def inp(at=None, layer_id="lejant", **extra):
    out = {"layerId": layer_id, **extra}
    if at is not None:
        out = {"at": P(*at), **out}
    return out


def made(rs, uid=None, eid=None, remade=False):
    out = {"materials": materials(rs), "block": "$blockOf:Malzeme lejandı"}
    if remade:
        out["remade"] = True
    if uid is not None:
        out["uid"], out["id"] = uid, eid
    out["revision"] = "$current"
    return out


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    e = {"code": code, "message": message}
    if path is not None:
        e["path"] = path
    return {"status": "failed", "error": e}


def refused(name, input_, code, message, path, non_finite=None, setup=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = failed(code, message, path)
    step["expect"] = UNCHANGED if setup is None else {"canUndo": False, "dirty": False, "revision": "same"}
    case = {"name": name, "steps": [step]}
    if setup is not None:
        case["setup"] = setup
    return case


def placed(name, input_, at, rs, h=2.5, layer_id="lejant", warnings=()):
    return {"name": name, "steps": [
        {"op": "execute", "input": input_, "result": completed(made(rs, "$uidOf:%d" % NEXT, NEXT), warnings),
         "expect": {"ids": IDS + [NEXT], "entities": {str(NEXT): insert(NEXT, at, layer_id)}, "blocks": [block(rs, h)],
                    "canUndo": True, "dirty": True, "revision": "changed"}},
        {"op": "undo", "returns": "Malzeme lejandı", "expect": {"ids": IDS, "blocks": [], "canUndo": False, "canRedo": True}}]}


def cases():
    rs = rows(ENTITIES)
    # Kireç sıva, Tuğla at 45°, Gazbeton, Isı yalıtımı (a blank material), Tuğla at 0°, Betonarme (a pattern the library
    # lacks: an empty swatch).
    assert [r["name"] for r in rs] == ["Kireç sıva", "Tuğla", "Gazbeton", "Isı yalıtımı", "Tuğla", "Betonarme"]
    assert [r["walls"] for r in rs] == [3, 2, 1, 1, 1, 1]
    at = (12, 10)
    out = [
        placed("Lejant bir noktaya: altı satır, blok aynı adımda; adım “Malzeme lejandı”", inp(at), at, rs),
        placed("Yazı 3,5 mm", inp(at, height=3.5), at, rs, h=3.5),
        placed("Gizli katmana uyarıyla", inp(at, "gizli"), at, rs, layer_id="gizli",
               warnings=[{"code": "layer_hidden", "message": HIDDEN, "path": "layerId"}]),
        placed("Grubun içindeki katmana", inp(at, "alt"), at, rs, layer_id="alt"),
    ]
    # Made again in its place: a wall gone, the legend loses its row; the insert stays.
    gone = [e for e in ENTITIES if e["id"] != 3]
    out.append({"name": "Duvar silinince lejant yerinde yeniden yapılır: Gazbeton satırı gider", "steps": [
        {"op": "execute", "input": inp(at), "result": "$any"},
        {"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:3"]}, "result": "$any"},
        {"op": "execute", "input": inp(), "result": completed(made(rows(gone), remade=True)),
         "expect": {"ids": [i for i in IDS if i != 3] + [NEXT], "blocks": [block(rows(gone))],
                    "entities": {str(NEXT): insert(NEXT, at)}}},
        {"op": "undo", "returns": "Malzeme lejandı", "expect": {"blocks": [block(rs)]}},
    ]})
    out.append({"name": "Yeniden yaparken yer de verilirse bir yerleştirme daha", "steps": [
        {"op": "execute", "input": inp(at), "result": "$any"},
        {"op": "execute", "input": inp((40, 10)), "result": completed(made(rs, "$uidOf:%d" % (NEXT + 1), NEXT + 1, remade=True)),
         "expect": {"ids": IDS + [NEXT, NEXT + 1], "blocks": [block(rs)],
                    "entities": {str(NEXT): insert(NEXT, at), str(NEXT + 1): insert(NEXT + 1, (40, 10))}}},
    ]})
    out.append({"name": "Değişmeyen lejant yeniden istenince hiçbir şey yazılmaz", "steps": [
        {"op": "execute", "input": inp(at), "result": "$any"},
        {"op": "captureRevision", "as": "made"},
        {"op": "execute", "input": inp(), "result": completed(made(rs, remade=True)),
         "expect": {"ids": IDS + [NEXT], "revision": "same", "blocks": [block(rs)]}},
    ]})
    out.append({"name": "Yeniden yaparken katman okunmaz", "steps": [
        {"op": "execute", "input": inp(at), "result": "$any"},
        {"op": "execute", "command": "cad.entities.delete", "input": {"uids": ["$uidOf:6"]}, "result": "$any"},
        {"op": "execute", "input": inp(layer_id="yok"), "result": completed(made(rows(ENTITIES[:5]), remade=True)),
         "expect": {"blocks": [block(rows(ENTITIES[:5]))]}},
    ]})
    # The plan: the block with no id yet (zero), the insert on it.
    plan_block = {**block(rs), "id": ZERO_BLOCK}
    out.append({"name": "Plan: satırlar, blok ve yerleştirme (yuvası 0); hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": inp(at),
         "result": completed({"materials": materials(rs), "block": plan_block,
                              "insert": insert(0, at, block_id=ZERO_BLOCK), "revision": "$current"}),
         "expect": UNCHANGED}]})
    out += [
        refused("Bileşenli duvar yok", inp(at), "no_materials", NO_MATERIALS, None, setup=PLAIN),
        refused("Yer yok, lejant da yok", inp(), "no_legend", NO_LEGEND, "at"),
        refused("Yeri sonlu değil", inp(at), "not_finite",
                "Lejandın yerinin doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.",
                "at.x", {"at.x": "NaN"}),
        refused("Yükseklik sonlu değil", inp(at, height=2.5), "not_finite",
                "Lejandın yazı yüksekliği sonlu bir sayı değil (NaN ya da sonsuz). Yüksekliği kâğıt milimetresi olarak sonlu bir sayıyla verin.",
                "height", {"height": "Infinity"}),
        refused("Yükseklik sıfır", inp(at, height=0), "invalid_height",
                "Lejandın yazı yüksekliği sıfırdan büyük, en çok 50 mm olmalı; 0 verildi.", "height"),
        refused("Yükseklik 50 mm'den büyük", inp(at, height=60), "invalid_height",
                "Lejandın yazı yüksekliği sıfırdan büyük, en çok 50 mm olmalı; 60 verildi.", "height"),
        refused("Beklenen sürüm sayı değil", inp(at, expectedRevision="on iki"), "invalid_revision", INVALID_REVISION,
                "expectedRevision"),
        refused("Olmayan katman", inp(at, "yok"), "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Grup katman değil", inp(at, "grup"), "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        refused("Kilitli katman", inp(at, "kilitli"), "layer_locked",
                "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": inp(at), "result": "$any"},
            {"op": "execute", "input": inp(expectedRevision="$old"),
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision",
                                                        "revision": "$current"}}}]},
    ]
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.legends.materials", "commandVersion": 1,
            "title": "Malzeme lejandı",
            "note": "Written by scripts/fixtures/legend_cases.py from docs/adr/0721 §5 and the hatch library's reference; do not edit by hand.",
            "setup": SETUP, "cases": cases()}


def main() -> int:
    doc = build()
    assert_no_negative_zero(doc, "cad.legends.materials")
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = OUT / "cad.legends.materials.json"
    if "--check" in sys.argv[1:]:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.legends.materials durumları güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
