"""The shared cases of cad.north.create (Kuzey oku, docs/adr/0721 §3).

    uv run --no-project --with pyproj python3 scripts/fixtures/north_cases.py           # writes the file
    uv run --no-project --with pyproj python3 scripts/fixtures/north_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.north.create.json. The checks, their order, codes, paths and messages are written here
by hand from the ADR; the layer's and the revision's are every create command's (cad.polygon.create's cases). What
the command makes is worked out here without KentOS code:

- the block is the drawing's own “Kuzey oku” when it has one, else KentOS's library's (read from the library's file,
  catalog/v1/blocks/kentos-temel.kblok, as data), added in the same step;
- its scale is the size on paper (15 mm when none) over 1000, times the drawing's scale's denominator, over the
  block's width east to west (its drawing's, texts aside), in that order of arithmetic;
- its turn: 0 for the plan's north, the angle given, or true north's angle from grid north at the point, read from
  PROJ (pyproj): the meridian convergence `get_factors().meridian_convergence`, in radians with its sign turned to a
  counter-clockwise angle; KentOS takes it from the meridian's points either side of the point, so true north is
  compared within 1e-9 rad.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
LIBRARY = json.loads((ROOT / "catalog/v1/blocks/kentos-temel.kblok").read_text(encoding="utf-8"))
X, Y = 487000, 4420000
PLOT = 100
style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False, kind="layer", children=()):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True, "style": style,
            "children": list(children)}


def plain(v):
    return int(v) if float(v).is_integer() else v


def P(x, y):
    return {"x": plain(x), "y": plain(y)}


ENTITIES = [{"kind": "line", "id": 3, "layerId": "isaret", "attrs": {}, "a": P(X, Y), "b": P(X + 10, Y)}]
IDS = [e["id"] for e in ENTITIES]
NEXT = 4
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Kuzey oku",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": PLOT,
                 "workspace": "cad", "drawingFont": "barlow"},
    "origin": {"x": X, "y": Y},
    "layers": [layer("isaret", "İşaret"), layer("kilitli", "Kilitli", locked=True), layer("gizli", "Gizli", visible=False),
               layer("grup", "Grup", kind="group", children=[layer("alt", "Alt")])],
    "activeLayer": "isaret", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
# A local project (no system): true north is not known.
LOCAL = copy.deepcopy(SETUP)
LOCAL["name"] = "Kuzey oku, yerel"
LOCAL["settings"]["srid"] = 0
# A drawing with its own north arrow: a square 2 m across and a line up, no text.
OWN_ID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7101"
OWN = copy.deepcopy(SETUP)
OWN["name"] = "Kuzey oku, kendi bloğu"
OWN["blocks"] = [{"id": OWN_ID, "name": "Kuzey oku", "base": P(0, 0), "entities": [
    {"kind": "polyline", "id": 1, "layerId": "", "attrs": {}, "pts": [P(-1, -1), P(1, -1), P(1, 1), P(-1, 1)], "closed": True},
    {"kind": "line", "id": 2, "layerId": "", "attrs": {}, "a": P(0, -1), "b": P(0, 1.5)}]}]
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
NO_TRUE_NORTH = ("Projenin koordinat sisteminde bu noktadan geçen meridyen yok (yerel proje ya da sistemin dışında bir nokta): "
                 "coğrafi kuzey bulunamaz. Plan kuzeyini ya da bir açı verin.")
HIDDEN = "“Gizli” katmanı gizli; çizilen nesne görünmeyecek."


def library_block(name):
    return next(b for b in LIBRARY["blocks"] if b["name"] == name)


def width(block):
    """The block's drawing east to west, texts aside: lines, paths and circles here."""
    xs = []
    for e in block["entities"]:
        if e["kind"] == "polyline":
            xs += [p["x"] for p in e["pts"]]
        elif e["kind"] == "line":
            xs += [e["a"]["x"], e["b"]["x"]]
        elif e["kind"] == "circle":
            xs += [e["c"]["x"] - e["r"], e["c"]["x"] + e["r"]]
        elif e["kind"] != "text":
            raise AssertionError(e["kind"])
    return max(xs) - min(xs)


def scale_of(block, size=15.0, plot=PLOT):
    return size / 1000.0 * plot / width(block)


NORTH = library_block("Kuzey oku")
NORTH_ID = NORTH["id"]
assert width(NORTH) == 1.0


def without_id(block):
    return {k: v for k, v in block.items() if k != "id"}


def true_north(x, y, epsg="EPSG:5256"):
    """True north's angle from grid north at (x, y), counter-clockwise radians, from PROJ's meridian convergence."""
    from pyproj import CRS, Proj, Transformer
    crs = CRS(epsg)
    lon, lat = Transformer.from_crs(crs, crs.geodetic_crs, always_xy=True).transform(x, y)
    # PROJ's convergence is grid north's angle from true north, clockwise positive (its sign checked below against the
    # meridians' bend: west of the central meridian, true north leans east of grid north).
    return math.radians(Proj(crs).get_factors(lon, lat).meridian_convergence)


def insert(eid, at, block_id, scale, rotation=0.0, layer_id="isaret", color=None):
    e = {"kind": "insert", "id": eid, "layerId": layer_id, "attrs": {}}
    if color is not None:
        e["color"] = color
    e.update({"block": block_id, "p": P(*at), "scale": plain(scale), "rotation": plain(rotation)})
    return e


def inp(at, layer_id="isaret", **extra):
    return {"at": P(*at), "layerId": layer_id, **extra}


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def written(name, input_, at, scale, rotation=0.0, added=True, convergence=None, block_id=NORTH_ID, warnings=(),
            layer_id="isaret", color=None, setup=None, blocks=None, within=None):
    out = {"uid": "$uid", "id": NEXT, "block": block_id, "rotation": plain(rotation), "revision": "$current"}
    if added:
        out["added"] = True
    if convergence is not None:
        out["convergence"] = convergence
    expect = {"ids": IDS + [NEXT], "entities": {str(NEXT): insert(NEXT, at, block_id, scale, rotation, layer_id, color)},
              "canUndo": True, "dirty": True, "revision": "changed"}
    if blocks is not None:
        expect["blocks"] = blocks
    step = {"op": "execute", "input": input_, "result": completed(out, warnings), "expect": expect}
    if within is not None:
        step["within"] = within
    undo = {"op": "undo", "returns": "Kuzey oku", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    if blocks is not None:
        undo["expect"]["blocks"] = [without_id(b) for b in (setup or SETUP).get("blocks", [])]
    case = {"name": name, "steps": [step, undo]}
    if setup is not None:
        case["setup"] = setup
    return case


def refused(name, input_, code, message, path, non_finite=None, setup=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = failed(code, message, path)
    step["expect"] = UNCHANGED
    case = {"name": name, "steps": [step]}
    if setup is not None:
        case["setup"] = setup
    return case


def not_finite_point(whose, axis):
    name = "doğu (Y)" if axis == "x" else "kuzey (X)"
    return f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def cases():
    at = (X + 10, Y + 10)
    s15 = scale_of(NORTH)
    west, east = (X + 20, Y + 30), (520000, Y + 30)
    tw, te = true_north(*west), true_north(*east)
    # West of the central meridian (36°) true north leans east (clockwise, a negative turn); east of it, west.
    assert tw < 0 < te, (tw, te)
    out = [
        written("Plan kuzeyi: kitaplığın oku aynı adımda eklenir, 15 mm; adım “Kuzey oku”", inp(at), at, s15,
                blocks=[without_id(NORTH)]),
        written("Kâğıtta 20 mm", inp(at, size=20), at, scale_of(NORTH, 20.0)),
        written("Kâğıtta 7,5 mm", inp(at, size=7.5), at, scale_of(NORTH, 7.5)),
        written("Planın kuzeyinden 30° saat yönünün tersine", inp(at, north={"kind": "angle", "angle": math.radians(30)}),
                at, s15, math.radians(30)),
        written("Planın kuzeyinden 90° saat yönünde", inp(at, north={"kind": "angle", "angle": -math.pi / 2}), at, s15,
                -math.pi / 2),
        written("Coğrafi kuzey, orta meridyenin batısında: doğuya yatar", inp(west, north={"kind": "true"}), west, s15, tw,
                convergence=tw, within=1e-9),
        written("Coğrafi kuzey, orta meridyenin doğusunda: batıya yatar", inp(east, north={"kind": "true"}), east, s15, te,
                convergence=te, within=1e-9),
        written("Coğrafi kuzey ve 25 mm", inp(west, north={"kind": "true"}, size=25), west, scale_of(NORTH, 25.0), tw,
                convergence=tw, within=1e-9),
        written("Renk verilmiş", inp(at, color="#E5484D"), at, s15, color="#E5484D"),
        written("Gizli katmana uyarıyla", inp(at, "gizli"), at, s15, layer_id="gizli",
                warnings=[{"code": "layer_hidden", "message": HIDDEN, "path": "layerId"}]),
        written("Grubun içindeki katmana", inp(at, "alt"), at, s15, layer_id="alt"),
        written("Çizimin kendi “Kuzey oku” bloğu kitaplığınkinden önce gelir: 2 m enli, eklenmez", inp(at), at,
                scale_of(OWN["blocks"][0]), block_id=OWN_ID, added=False, setup=OWN,
                blocks=[without_id(OWN["blocks"][0])]),
        written("Yerel projede plan kuzeyi yazılır", inp(at), at, s15, setup=LOCAL),
    ]
    # A second arrow: the block the first added is the drawing's now.
    first = inp(at)
    second = inp((X + 40, Y + 10), north={"kind": "angle", "angle": math.radians(45)})
    out.append({"name": "İkinci ok çizimin (ilkin eklediği) bloğunu kullanır", "steps": [
        {"op": "execute", "input": first, "result": "$any"},
        {"op": "execute", "input": second,
         "result": completed({"uid": "$uid", "id": NEXT + 1, "block": NORTH_ID, "rotation": math.radians(45), "revision": "$current"}),
         "expect": {"ids": IDS + [NEXT, NEXT + 1], "blocks": [without_id(NORTH)],
                    "entities": {str(NEXT + 1): insert(NEXT + 1, (X + 40, Y + 10), NORTH_ID, s15, math.radians(45))}}},
        {"op": "undo", "returns": "Kuzey oku", "expect": {"ids": IDS + [NEXT], "blocks": [without_id(NORTH)]}},
    ]})
    out.append({"name": "Plan: yerleştirme (yuvası 0), eklenecek blok ve dönüş; hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": inp(west, north={"kind": "true"}),
         "result": completed({"insert": insert(0, west, NORTH_ID, s15, tw), "block": NORTH, "rotation": tw,
                              "convergence": tw, "revision": "$current"}),
         "within": 1e-9, "expect": UNCHANGED}]})
    out += [
        refused("Yeri sonlu değil", inp(at), "not_finite", not_finite_point("Kuzey okunun yerinin", "x"), "at.x", {"at.x": "NaN"}),
        refused("Açı sonlu değil", inp(at, north={"kind": "angle", "angle": 1}), "not_finite",
                "Kuzeyin açısı sonlu bir sayı değil (NaN ya da sonsuz). Açıyı radyan olarak sonlu bir sayıyla verin.",
                "north.angle", {"north.angle": "Infinity"}),
        refused("Boy sonlu değil", inp(at, size=10), "not_finite",
                "Kuzey okunun boyu sonlu bir sayı değil (NaN ya da sonsuz). Boyu kâğıt milimetresi olarak sonlu bir sayıyla verin.",
                "size", {"size": "NaN"}),
        refused("Boy sıfır", inp(at, size=0), "invalid_scale",
                "Kuzey okunun boyu sıfırdan büyük, en çok 200 mm olmalı; 0 verildi. Kâğıttaki boyunu milimetre olarak verin.", "size"),
        refused("Boy eksi", inp(at, size=-5), "invalid_scale",
                "Kuzey okunun boyu sıfırdan büyük, en çok 200 mm olmalı; -5 verildi. Kâğıttaki boyunu milimetre olarak verin.", "size"),
        refused("Boy 200 mm'den büyük", inp(at, size=250), "invalid_scale",
                "Kuzey okunun boyu sıfırdan büyük, en çok 200 mm olmalı; 250 verildi. Kâğıttaki boyunu milimetre olarak verin.", "size"),
        refused("Beklenen sürüm sayı değil", inp(at, expectedRevision="on iki"), "invalid_revision", INVALID_REVISION,
                "expectedRevision"),
        refused("Yerel projede coğrafi kuzey yok", inp(at, north={"kind": "true"}), "no_true_north", NO_TRUE_NORTH, "north",
                setup=LOCAL),
        refused("Olmayan katman", inp(at, "yok"), "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Grup katman değil", inp(at, "grup"), "not_a_layer",
                "“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        refused("Kilitli katman", inp(at, "kilitli"), "layer_locked",
                "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": inp(at), "result": "$any"},
            {"op": "execute", "input": inp((X + 30, Y), expectedRevision="$old"),
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision",
                                                        "revision": "$current"}}}]},
    ]
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.north.create", "commandVersion": 1,
            "title": "Kuzey oku",
            "note": "Written by scripts/fixtures/north_cases.py from docs/adr/0721 §3 and PROJ's meridian convergence; do not edit by hand.",
            "setup": SETUP, "cases": cases()}


def main() -> int:
    doc = build()
    assert_no_negative_zero(doc, "cad.north.create")
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = OUT / "cad.north.create.json"
    if "--check" in sys.argv[1:]:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.north.create durumları güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
