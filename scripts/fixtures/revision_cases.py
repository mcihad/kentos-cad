"""The shared cases of cad.revisions.mark (Revizyon işaretle, docs/adr/0721 §4).

    python3 scripts/fixtures/revision_cases.py           # writes the file
    python3 scripts/fixtures/revision_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.revisions.mark.json. The checks, their order, codes, paths and messages are written
here by hand from the ADR; the cloud's are cad.revcloud.create's and its ring is that command's reference's
(scripts/fixtures/shape_command_cases.py: the frame's corners, each side divided into chords, every bulge 0.5). What
the command adds is worked out here without KentOS code:

- the cloud's attributes: `Revizyon` the number, `Tarih` and `Açıklama` when given;
- the delta: the drawing's own block “Revizyon” when it has one, else KentOS's library's (read from the library's file
  as data) added in the same step; its scale the size on paper (6 mm when none) over 1000, times the drawing's scale's
  denominator, over the triangle's width east to west (its texts aside); its centre `at`, else the cloud's corners'
  most east and north, each plus half an arc and half the delta's size in the drawing; its `REV` the number and the
  date and description as its other data. One undo step “Revizyon”.

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
from affine_reference import assert_no_negative_zero  # noqa: E402  (beside this file)
import shape_command_cases as shapes  # noqa: E402  (the revision cloud's reference)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
LIBRARY = json.loads((ROOT / "catalog/v1/blocks/kentos-temel.kblok").read_text(encoding="utf-8"))
SETUP = shapes.SETUP
IDS, NEXT = shapes.IDS, shapes.NEXT
PLOT = SETUP["settings"]["plotScale"]
UNCHANGED = shapes.UNCHANGED
DELTA = next(b for b in LIBRARY["blocks"] if b["name"] == "Revizyon")
DELTA_ID = DELTA["id"]
INVALID_NUMBER = ("Revizyonun numarası boş olmayan, kenarı boşluksuz, denetim karaktersiz ve en çok 16 karakter olmalı "
                  "(1, A, B2 gibi).")
INVALID_DATE = "Revizyonun tarihi kenarı boşluksuz, denetim karaktersiz ve en çok 32 karakter olmalı."
INVALID_DESCRIPTION = "Revizyonun açıklaması kenarı boşluksuz, denetim karaktersiz ve en çok 240 karakter olmalı."


def width(block):
    xs = []
    for e in block["entities"]:
        if e["kind"] == "polyline":
            xs += [p["x"] for p in e["pts"]]
        elif e["kind"] == "line":
            xs += [e["a"]["x"], e["b"]["x"]]
        elif e["kind"] != "text":
            raise AssertionError(e["kind"])
    return max(xs) - min(xs)


assert width(DELTA) == 0.3 - (-0.3)


def without_id(block):
    return {k: v for k, v in block.items() if k != "id"}


def scale_of(size=6.0):
    return size / 1000.0 * PLOT / width(DELTA)


def default_at(ring, arc, size=6.0):
    """Off the cloud's top right: its corners' most east and north, out by half an arc and half the delta."""
    pts, _ = shapes.cloud(ring, float(arc))
    east = max(p[0] for p in pts)
    north = max(p[1] for p in pts)
    half = size / 1000.0 * PLOT / 2.0
    return (east + arc / 2.0 + half, north + arc / 2.0 + half)


def mark(frame, arc, number, layer="yapi", **extra):
    return {"layerId": layer, "frame": frame, "arc": arc, "number": number, **extra}


def attrs_of(number, date=None, description=None, tag=False):
    a = {"REV" if tag else "Revizyon": number}
    if date is not None:
        a["Tarih"] = date
    if description is not None:
        a["Açıklama"] = description
    return a


def delta(eid, at, number, size=6.0, layer="yapi", color=None, date=None, description=None):
    e = {"kind": "insert", "id": eid, "layerId": layer}
    if color is not None:
        e["color"] = color
    e.update({"attrs": attrs_of(number, date, description, tag=True), "block": DELTA_ID, "p": shapes.W(at),
              "scale": shapes.num(scale_of(size)), "rotation": 0})
    return e


def cloud_entity(ring, arc, number, layer="yapi", eid=NEXT, color=None, line_weight=None, date=None, description=None):
    pts, bulges = shapes.cloud(ring, float(arc))
    extra = {"attrs": attrs_of(number, date, description)}
    if color is not None:
        extra["color"] = color
    if line_weight is not None:
        extra["lineWeight"] = line_weight
    return shapes.polygon_entity(pts, bulges, layer, extra, id_=eid)


def written(name, input_, ring, arc, number, at=None, size=6.0, layer="yapi", color=None, line_weight=None, date=None,
            description=None, warnings=(), added=True, blocks=True):
    at = at or default_at(ring, arc, size)
    out = {"cloud": "$uidOf:%d" % NEXT, "cloudId": NEXT, "tag": "$uidOf:%d" % (NEXT + 1), "tagId": NEXT + 1, "block": DELTA_ID}
    if added:
        out["added"] = True
    out["revision"] = "$current"
    expect = {"ids": IDS + [NEXT, NEXT + 1], "canUndo": True, "dirty": True, "revision": "changed",
              "entities": {str(NEXT): cloud_entity(ring, arc, number, layer, NEXT, color, line_weight, date, description),
                           str(NEXT + 1): delta(NEXT + 1, at, number, size, layer, color, date, description)}}
    if blocks:
        expect["blocks"] = [without_id(DELTA)]
    undo = {"op": "undo", "returns": "Revizyon", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    if blocks:
        undo["expect"]["blocks"] = []
    return {"name": name, "steps": [
        {"op": "execute", "input": input_, "result": {"status": "completed", "output": out, "warnings": list(warnings)},
         "expect": expect},
        undo]}


def refused(name, input_, code, message, path, non_finite=None):
    return shapes.refused(name, input_, code, message, path, non_finite)


def cases():
    a, b = shapes.at(10, 10), shapes.at(30, 20)
    rect = shapes.rect_frame(a, b)
    ring = shapes.rect_ring(a, b)
    tri = [shapes.at(0, 0), shapes.at(40, 0), shapes.at(40, 30)]  # the setup's area 9 on Sınır
    hidden = {"code": "layer_hidden", "message": shapes.HIDDEN, "path": "layerId"}
    out = [
        written("A revizyonu dikdörtgende: bulut ve üçgen, kitaplığın bloğu aynı adımda; adım “Revizyon”",
                mark(rect, 2, "A"), ring, 2, "A"),
        written("Tarih ve açıklama ikisine de yazılır", mark(rect, 2, "B", date="27.10.2026", description="Mutfak tezgâhı uzatıldı"),
                ring, 2, "B", date="27.10.2026", description="Mutfak tezgâhı uzatıldı"),
        written("Üçgenin yeri verilmiş", mark(rect, 2, "C", at=shapes.W(shapes.at(5, 25))), ring, 2, "C", at=shapes.at(5, 25)),
        written("Üçgen kâğıtta 10 mm", mark(rect, 2, "2", size=10), ring, 2, "2", size=10.0),
        written("Yay boyu kenarı bölmüyor: üçgen yine sağ üstte", mark(rect, 3, "D"), ring, 3, "D"),
        written("Üçgen çerçevesi", mark(shapes.poly_frame(tri), 5, "E"), tri, 5, "E"),
        written("Nesnenin çevresine: Sınır'daki alan", mark({"kind": "object", "uid": "$uidOf:9"}, 10, "F"), tri, 10, "F"),
        written("Renk ve kalınlık: bulut ve üçgen renkli, kalınlık bulutun", mark(rect, 2, "G", color="#E5484D", lineWeight=0.5),
                ring, 2, "G", color="#E5484D", line_weight=0.5),
        written("Gizli katmana uyarıyla", mark(rect, 2, "H", layer="gizli"), ring, 2, "H", layer="gizli", warnings=[hidden]),
        written("16 karakterlik numara ve 240 karakterlik açıklama", mark(rect, 2, "R-2026-10-27/001", description="ç" * 240),
                ring, 2, "R-2026-10-27/001", description="ç" * 240),
    ]
    # A second mark: the delta block the first added is the drawing's now.
    first, second = mark(rect, 2, "A"), mark(shapes.poly_frame(tri), 5, "B")
    out.append({"name": "İkinci işaret çizimin (ilkin eklediği) üçgen bloğunu kullanır", "steps": [
        {"op": "execute", "input": first, "result": "$any"},
        {"op": "execute", "input": second,
         "result": {"status": "completed", "warnings": [],
                    "output": {"cloud": "$uidOf:%d" % (NEXT + 2), "cloudId": NEXT + 2, "tag": "$uidOf:%d" % (NEXT + 3),
                               "tagId": NEXT + 3, "block": DELTA_ID, "revision": "$current"}},
         "expect": {"ids": IDS + [NEXT, NEXT + 1, NEXT + 2, NEXT + 3], "blocks": [without_id(DELTA)],
                    "entities": {str(NEXT + 2): cloud_entity(tri, 5, "B", eid=NEXT + 2),
                                 str(NEXT + 3): delta(NEXT + 3, default_at(tri, 5), "B")}}},
        {"op": "undo", "returns": "Revizyon", "expect": {"ids": IDS + [NEXT, NEXT + 1], "blocks": [without_id(DELTA)]}},
    ]})
    pts, bulges = shapes.cloud(ring, 2.0)
    out.append({"name": "Plan: bulut ve üçgen (yuvaları 0), eklenecek blok; hiçbir şey yazılmaz", "steps": [
        {"op": "plan", "input": mark(rect, 2, "A"),
         "result": {"status": "completed", "warnings": [],
                    "output": {"cloud": cloud_entity(ring, 2, "A", eid=0), "tag": delta(0, default_at(ring, 2), "A"),
                               "block": DELTA, "revision": "$current"}},
         "expect": UNCHANGED}]})
    base = mark(rect, 2, "A")
    e = shapes.POLY_ERR
    out += [
        refused("Numara boş", mark(rect, 2, ""), "invalid_mark", INVALID_NUMBER, "number"),
        refused("Numaranın başında boşluk", mark(rect, 2, " A"), "invalid_mark", INVALID_NUMBER, "number"),
        refused("Numarada satır sonu", mark(rect, 2, "A\nB"), "invalid_mark", INVALID_NUMBER, "number"),
        refused("Numara 17 karakter", mark(rect, 2, "R-2026-10-27/0001"), "invalid_mark", INVALID_NUMBER, "number"),
        refused("Tarih 33 karakter", mark(rect, 2, "A", date="2" * 33), "invalid_mark", INVALID_DATE, "date"),
        refused("Açıklamanın sonunda boşluk", mark(rect, 2, "A", description="Tezgâh "), "invalid_mark", INVALID_DESCRIPTION,
                "description"),
        refused("Açıklama 241 karakter", mark(rect, 2, "A", description="ç" * 241), "invalid_mark", INVALID_DESCRIPTION,
                "description"),
        refused("Üçgenin yeri sonlu değil", mark(rect, 2, "A", at=shapes.W(shapes.at(5, 25))), "not_finite",
                shapes.not_finite_point("Revizyon üçgeninin yerinin", "y"), "at.y", {"at.y": "NaN"}),
        refused("Üçgenin boyu sonlu değil", mark(rect, 2, "A", size=6), "not_finite",
                "Revizyon üçgeninin boyu sonlu bir sayı değil (NaN ya da sonsuz). Boyu kâğıt milimetresi olarak sonlu bir sayıyla verin.",
                "size", {"size": "Infinity"}),
        refused("Üçgenin boyu sıfır", mark(rect, 2, "A", size=0), "invalid_scale",
                "Revizyon üçgeninin boyu sıfırdan büyük, en çok 100 mm olmalı; 0 verildi. Kâğıttaki boyunu milimetre olarak verin.", "size"),
        refused("Üçgenin boyu 100 mm'den büyük", mark(rect, 2, "A", size=150), "invalid_scale",
                "Revizyon üçgeninin boyu sıfırdan büyük, en çok 100 mm olmalı; 150 verildi. Kâğıttaki boyunu milimetre olarak verin.",
                "size"),
        refused("Yay boyu sıfır: bulutun reddi", mark(rect, 0, "A"), "invalid_distance", "Yay boyu sıfırdan büyük olmalı.", "arc"),
        refused("Dikdörtgenin yüksekliği yok", mark(shapes.rect_frame(a, shapes.at(30, 10)), 2, "A"), "degenerate_shape",
                shapes.CLOUD_RECT_DEGENERATE, "frame"),
        refused("Çokgenin iki köşesi", mark(shapes.poly_frame(tri[:2]), 5, "A"), "too_few_corners", shapes.CLOUD_FEW, "frame.pts"),
        refused("Olmayan nesne", mark({"kind": "object", "uid": shapes.NO_SUCH}, 5, "A"), "entity_not_found",
                f"“{shapes.NO_SUCH}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin "
                "kimliğini verin.", "frame.uid"),
        refused("Nokta kapalı değil", mark({"kind": "object", "uid": "$uidOf:4"}, 5, "A"), "not_closed", shapes.CLOUD_OPEN, "frame.uid"),
        refused("10 000'den çok yay", mark(rect, 0.001, "A"), "too_many_points", shapes.CLOUD_MANY, "arc"),
        refused("Kalınlık 100 mm'den büyük", {**base, "lineWeight": 101}, "invalid_line_weight", e["invalid_line_weight"]["message"],
                "lineWeight"),
        refused("Kilitli katman", {**base, "layerId": "kilitli"}, "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"),
        refused("Olmayan katman", {**base, "layerId": "yok"}, "layer_not_found", e["layer_not_found"]["message"], "layerId"),
        refused("Katman değil grup", {**base, "layerId": "pafta"}, "not_a_layer", e["not_a_layer"]["message"], "layerId"),
        refused("Beklenen sürüm sayı değil", {**base, "expectedRevision": "012"}, "invalid_revision", e["invalid_revision"]["message"],
                "expectedRevision"),
        {"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "old"},
            {"op": "execute", "input": base, "result": "$any"},
            {"op": "execute", "input": {**base, "expectedRevision": "$old"},
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": e["revision_conflict"]["message"],
                                                        "path": "expectedRevision", "revision": "$current"}}}]},
    ]
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.revisions.mark", "commandVersion": 1,
            "title": "Revizyon işaretle",
            "note": "Written by scripts/fixtures/revision_cases.py from docs/adr/0721 §4 and cad.revcloud.create's reference; do not edit by hand.",
            "setup": SETUP, "cases": cases()}


def main() -> int:
    doc = build()
    assert_no_negative_zero(doc, "cad.revisions.mark")
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    path = OUT / "cad.revisions.mark.json"
    if "--check" in sys.argv[1:]:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"cad.revisions.mark durumları güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
