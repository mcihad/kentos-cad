"""Patlat: shared cases of cad.entities.explode (docs/adr/0302).

    python3 scripts/fixtures/explode_command_cases.py           # writes the file
    python3 scripts/fixtures/explode_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.explode.json. The ADR's rules, computed here without KentOS code:

- A path's pieces are its edges, in order, a closed polyline's closing edge last; an area's its ring's edges (the
  closing one last), then its holes', then its other parts' (docs/adr/0143); a multi-part polyline its parts' after
  its own. A piece is a new object on the object's layer with its colour and line weight, no attributes.
- A block's insert opens one level: each of the definition's objects placed as the insert places it (scaled about
  the base point, moved to the insert's point); it keeps its own layer when the drawing has it (else the insert's),
  its colour and line weight (else the insert's).
- Objects are written in the input's order: each object's removal, then its pieces; the pieces get the next slots.
- An item of an associative array (docs/adr/0300) brings every item of its array, which is gone in the same step.
- A line or a point does not explode: left out with a warning naming the count and the first one's words (a circle
  becomes a polyline, docs/adr/0308; its cases are explode_curve_cases.py's),
  the refusal when none explodes; an object on a locked layer the same with the locked words (docs/adr/0278).
- The checks' order: no_entities, invalid_uid, the revision, entity_not_found, layer_locked, not_explodable.
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.explode.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
LABEL = "Patlat"
BLOCK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e01"
NOT_BASIC = "Bu nesne zaten temel bir nesne; patlatılacak bir şey yok."
LOCKED = "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."


def num(v):
    v = float(v)
    return int(v) if v.is_integer() else v


def p(x, y):
    return {"x": num(x), "y": num(y)}


def node(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def base(i, layer="cizim", **more):
    return {"id": i, "layerId": layer, "attrs": {}, **more}


OPEN = {"kind": "polyline", **base(1, color="#ff0000", lineWeight=0.5), "pts": [p(0, 0), p(10, 0), p(10, 5)]}
AREA = {"kind": "polygon", **base(2, attrs={"Ada": "101"}), "pts": [p(20, 0), p(30, 0), p(30, 10), p(20, 10)],
        "holes": [{"pts": [p(22, 2), p(24, 2), p(24, 4), p(22, 4)]}]}
LINE = {"kind": "line", **base(3), "a": p(40, 0), "b": p(45, 0)}
CIRCLE = {"kind": "circle", **base(4), "c": p(50, 0), "r": 1}
LOCKED_PATH = {"kind": "polyline", **base(5, "kilitli"), "pts": [p(0, 20), p(5, 20), p(5, 25)]}
INSERT = {"kind": "insert", **base(6, color="#0000ff"), "block": BLOCK, "p": p(60, 0), "scale": 2, "rotation": 0}
PARTS = {"kind": "polyline", **base(7), "pts": [p(0, 40), p(5, 40)], "parts": [{"pts": [p(0, 45), p(5, 45)]}]}
CLOSED = {"kind": "polyline", **base(8), "pts": [p(70, 0), p(75, 0), p(75, 5)], "closed": True}
POINT = {"kind": "point", **base(9), "p": p(80, 0)}
ENTITIES = [OPEN, AREA, LINE, CIRCLE, LOCKED_PATH, INSERT, PARTS, CLOSED, POINT]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = 10
DEFINITION = {"id": BLOCK, "name": "Rögar", "base": p(0, 0), "entities": [
    {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": p(0, 0), "b": p(1, 0)},
    {"kind": "circle", "id": 2, "layerId": "agac", "attrs": {}, "color": "#00ff00", "lineWeight": 0.35, "c": p(0, 1), "r": 0.5},
]}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Patlat",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(0, 0),
        "layers": [node("cizim", "Çizim"), node("agac", "Ağaç"), node("kilitli", "Kilitli", locked=True)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
        "blocks": [copy.deepcopy(DEFINITION)],
    }


# ── The rules ──────────────────────────────────────────────────────────────────────────────────────────────────────

def look(e):
    """What a piece takes from its object: its layer, colour and line weight."""
    out = {"layerId": e["layerId"], "attrs": {}}
    for k in ("color", "lineWeight"):
        if k in e:
            out[k] = e[k]
    return out


def edges(pts, closed):
    pairs = list(zip(pts, pts[1:]))
    if closed:
        pairs.append((pts[-1], pts[0]))
    return pairs


def pieces(e):
    """The object's pieces without slots, in order."""
    kind = e["kind"]
    if kind == "polyline":
        rings = [(e["pts"], bool(e.get("closed")))] + [(part["pts"], False) for part in e.get("parts", [])]
    elif kind == "polygon":
        rings = [(e["pts"], True)] + [(h["pts"], True) for h in e.get("holes", [])]
    elif kind == "insert":
        out = []
        s, at = e["scale"], e["p"]
        place = lambda q: p(at["x"] + s * q["x"], at["y"] + s * q["y"])  # noqa: E731  (base (0, 0), no turn)
        for d in DEFINITION["entities"]:
            own = {"layerId": d["layerId"] or e["layerId"], "attrs": {}}
            for k in ("color", "lineWeight"):
                if k in d:
                    own[k] = d[k]
                elif k in e:
                    own[k] = e[k]
            if d["kind"] == "line":
                out.append({"kind": "line", **own, "a": place(d["a"]), "b": place(d["b"])})
            else:
                out.append({"kind": "circle", **own, "c": place(d["c"]), "r": num(s * d["r"])})
        return out
    else:
        raise AssertionError(kind)
    return [{"kind": "line", **look(e), "a": a, "b": b} for pts, closed in rings for a, b in edges(pts, closed)]


def exploded(ids, start=NEXT):
    """The pieces of the objects `ids` with their slots, and the slots."""
    out, slots, slot = {}, [], start
    for i in ids:
        for piece in pieces(BY_ID[i]):
            out[str(slot)] = {**piece, "id": slot}
            slots.append(slot)
            slot += 1
    return out, slots


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def done(removed, created, warnings=()):
    return {"status": "completed", "warnings": list(warnings),
            "output": {"changed": [], "created": [U(s) for s in created], "removed": [f"$uid:o{i}" for i in removed],
                       "revision": "$current"}}


def capture(ids):
    return [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in ids]


def written(name, ids, input_ids=None, warnings=(), undo=False, more_ids=()):
    made, slots = exploded(ids)
    kept = [i for i in IDS if i not in ids]
    steps = capture(ids) + [{"op": "execute", "input": {"uids": [U(i) for i in (input_ids or ids)]},
                             "result": done(ids, slots, warnings),
                             "expect": {"ids": sorted(kept + slots), "entities": made, "revision": "changed", "canUndo": True}}]
    if undo:
        steps.append({"op": "undo", "returns": LABEL, "expect": {"ids": IDS, "entities": {str(i): BY_ID[i] for i in ids}}})
    return {"name": name, "steps": steps}


def refused(name, input_, code, message, path, status="failed"):
    return {"name": name, "steps": [{"op": "execute", "input": input_,
                                     "result": {"status": status, "error": {"code": code, "message": message, "path": path}},
                                     "expect": {"ids": IDS, "revision": "same", "canUndo": False}}]}


def skipped(n, words):
    return {"code": "not_explodable", "message": f"{n} nesne patlatılamadığı için atlandı: {words}", "path": "uids"}


def cases():
    out = [
        written("Açık çoklu çizgi iki çizgi olur; rengi ve kalınlığı geçer; geri alınır", [1], undo=True),
        written("Delikli alan kenarları ve deliğin kenarları; öznitelikler parçalara geçmez", [2]),
        written("Kapalı çoklu çizginin kapanış kenarı da çizgi olur", [8]),
        written("Çok parçalı çoklu çizginin öbür parçası da", [7]),
        written("Bloğun yerleştirmesi tanımın nesnelerine açılır: ölçekli, katmanı ya da yerleştirmeninki, rengi ya da yerleştirmeninki",
                [6], undo=True),
        written("İki nesne birlikte, verildikleri sırayla: silme ve parçaları", [1, 2]),
        written("Aynı kimlik iki kez: bir kez patlar", [1], input_ids=[1, 1]),
        written("Çizgi ile çoklu çizgi: çizgi uyarıyla atlanır", [1], input_ids=[3, 1], warnings=[skipped(1, NOT_BASIC)]),
        written("Kilitli katmandaki ile açık olan: kilitli uyarıyla atlanır", [1], input_ids=[5, 1],
                warnings=[{"code": "layer_locked", "path": "uids",
                           "message": "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."}]),
        written("Kilitli, patlamayan ve patlayan: iki uyarı, kilitli önce", [2], input_ids=[5, 9, 2],
                warnings=[{"code": "layer_locked", "path": "uids",
                           "message": "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."},
                          skipped(1, NOT_BASIC)]),
        refused("Yalnız çizgi: not_explodable", {"uids": [U(3)]}, "not_explodable", NOT_BASIC, "uids[0]"),
        refused("Yalnız nokta: not_explodable", {"uids": [U(9)]}, "not_explodable", NOT_BASIC, "uids[0]"),
        refused("Nokta ve çizgi: not_explodable, ilkinin sözüyle ve yeriyle", {"uids": [U(9), U(3)]}, "not_explodable", NOT_BASIC, "uids[0]"),
        refused("Yalnız kilitli: layer_locked", {"uids": [U(5)]}, "layer_locked", LOCKED, "uids[0]"),
        refused("Boş liste: no_entities", {"uids": []}, "no_entities", "Patlatılacak nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"),
        refused("Kimlik UUID değil: invalid_uid", {"uids": ["on iki"]}, "invalid_uid",
                "“on iki” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.",
                "uids[0]"),
        refused("Olmayan nesne: entity_not_found", {"uids": ["0192f5a2-0000-7000-8000-00000000ffff"]}, "entity_not_found",
                "“0192f5a2-0000-7000-8000-00000000ffff” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                "uids[0]"),
        refused("Beklenen sürüm sayı değil: invalid_revision", {"uids": [U(1)], "expectedRevision": "on iki"}, "invalid_revision",
                "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                "expectedRevision"),
    ]
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "command": "cad.entities.transform", "input": {"uids": [U(3)], "transform": {"kind": "move", "dx": 1, "dy": 0}}, "result": "$any"},
        {"op": "execute", "input": {"uids": [U(1)], "expectedRevision": "$old"},
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."}},
         "expect": {"revision": "same"}}]})
    made, slots = exploded([2])
    out.append({"name": "Plan parçaları gösterir; hiçbir şey yazılmaz", "steps": capture([2]) + [
        {"op": "plan", "input": {"uids": [U(2)]},
         "result": {"status": "completed", "warnings": [], "output": {
             "changed": [], "created": [{**made[str(s)], "id": 0} for s in slots], "removed": ["$uid:o2"], "revision": "$current"}},
         "expect": {"ids": IDS, "revision": "same"}}]})

    # ── an associative array ──
    grid = {"kind": "grid", "rows": 1, "cols": 3, "dx": 5, "dy": 0}
    items = [NEXT, NEXT + 1, NEXT + 2]
    record = {"block": "$blockOf:Dizi 1", "layerId": "cizim", "p": p(50, 0), "scale": 1, "rotation": 0, "layout": grid,
              "items": [{"uid": U(s)} for s in items]}
    make = {"op": "execute", "command": "cad.entities.array", "input": {"uids": [U(4)], "layout": grid, "associative": True},
            "result": "$any", "expect": {"arrays": [record]}}
    # The item given first, then the array's others in their order: 55, 50, 60.
    circles = {str(s): {"kind": "circle", "id": s, "layerId": "cizim", "attrs": {}, "c": p(x, 0), "r": 1}
               for x, s in zip([55, 50, 60], range(NEXT + 3, NEXT + 6))}
    after = sorted([i for i in IDS if i != 4] + list(range(NEXT + 3, NEXT + 6)))
    captured = [{"op": "captureUid", "id": s, "as": f"o{s}"} for s in items]
    out.append({"name": "İlişkili dizinin bir öğesi bütün diziyi patlatır: öğeler bloğun nesnelerine açılır, dizi gider; geri alınınca döner",
                "steps": [make] + captured + [
                    {"op": "execute", "input": {"uids": [U(items[1])]},
                     "result": {"status": "completed", "warnings": [], "output": {
                         "changed": [], "created": [U(s) for s in range(NEXT + 3, NEXT + 6)],
                         "removed": [f"$uid:o{s}" for s in [items[1], items[0], items[2]]], "revision": "$current"}},
                     "expect": {"ids": after, "entities": circles, "arrays": []}},
                    {"op": "undo", "returns": LABEL, "expect": {"arrays": [record]}}]})
    out.append({"name": "Aynı dizinin iki öğesi: dizi bir kez patlar",
                "steps": [make] + captured + [
                    {"op": "execute", "input": {"uids": [U(items[2]), U(items[0])]},
                     "result": {"status": "completed", "warnings": [], "output": {
                         "changed": [], "created": "$any",
                         "removed": [f"$uid:o{s}" for s in [items[2], items[0], items[1]]], "revision": "$current"}},
                     "expect": {"ids": after, "arrays": []}}]})
    out.append({"name": "Dizinin öğesiyle başka bir nesne: ikisi de patlar, dizi gider",
                "steps": [make] + captured + capture([1]) + [
                    {"op": "execute", "input": {"uids": [U(1), U(items[0])]},
                     "result": {"status": "completed", "warnings": [], "output": {
                         "changed": [], "created": "$any",
                         "removed": ["$uid:o1"] + [f"$uid:o{s}" for s in items], "revision": "$current"}},
                     "expect": {"arrays": []}}]})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.explode", "commandVersion": 1,
            "title": "Patlat: yollar, alanlar, bloklar, ilişkili diziler, patlamayanlar ve retler",
            "note": "ADR 0302. Bağımsız başvuru: scripts/fixtures/explode_command_cases.py.",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
