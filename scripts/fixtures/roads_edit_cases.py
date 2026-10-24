"""The shared cases of cad.roads.edit (docs/adr/0622 §9, 0198): Plan yolu, Kavşak temizle and Refüj kapat.

    python3 scripts/fixtures/roads_edit_cases.py           # writes the file
    python3 scripts/fixtures/roads_edit_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.roads.edit.json, without KentOS code. The geometry is plan_road_cases.py's (written
from docs/adr/0198 on its own): the road's corridor, the carriageway inside its kerbs, the median closed with half
circles; the inner corners rounded; two lines closed into a median. The areas are compared by their corners (any
vertex first) and their vertices' area, as the runner's `shapes` does; the junctions' areas are unions whose outline
is written here by hand (a cross of two streets, a T), then rounded by the reference.

- Plan yolu: the road (Tür its kind, Genişlik its width), with kerbs the carriageway (Taşıt yolu, its width), with a
  median the median (Refüj), with `axisLine` the axis (Yol ekseni), in that order on the layer, one step “Plan yolu”;
  kerbs and median only on a Yol.
- Kavşak temizle: the given areas but Refüj and Yol ekseni, joined kind by kind (Tür), each kind's inner corners
  rounded with the kerb radius for Taşıt yolu and the block radius otherwise; the first area of a kind takes the
  joined one, the others go; one step “Kavşak temizle”.
- Refüj kapat: the first line becomes the median's area, the second goes; one step “Refüj kapat”.
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILE = OUT / "cad.roads.edit.json"
sys.path.insert(0, str(Path(__file__).resolve().parent))
import plan_road_cases as road_ref  # noqa: E402

POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
WITHIN = 1e-9
KIND, WIDTH = "Tür", "Genişlik"


def P(x, y):
    return {"x": x, "y": y}


def area(id_, ring, kind, holes=None, layer="yapi"):
    e = {"kind": "polygon", "id": id_, "layerId": layer, "attrs": {KIND: kind}, "pts": [P(*p) for p in ring]}
    if holes:
        e["holes"] = [{"pts": [P(*p) for p in h]} for h in holes]
    return e


def line(id_, pts, layer="yapi", bulges=None):
    e = {"kind": "polyline", "id": id_, "layerId": layer, "attrs": {}, "pts": [P(*p) for p in pts]}
    if bulges:
        e["bulges"] = bulges
    return e


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


ENTITIES = [
    # A cross of two streets: the union is plan_road_cases' PLUS.
    area(1, rect(0, 25, 60, 35), "Yol"),
    area(2, rect(25, 0, 35, 60), "Yol"),
    # Their carriageways, their own kind.
    area(3, rect(0, 27, 60, 33), "Taşıt yolu"),
    area(4, rect(27, 0, 33, 60), "Taşıt yolu"),
    # A T: the union is plan_road_cases' TEE, shifted.
    area(5, [(100 + x, y) for x, y in rect(0, 10, 40, 20)], "Yol"),
    area(6, [(100 + x, y) for x, y in rect(16, 0, 24, 10)], "Yol"),
    # A ring road's island: the hole's corners are inner.
    area(7, [(200 + x, y) for x, y in road_ref.RING_OUT], "Yol", holes=[[(200 + x, y) for x, y in road_ref.RING_HOLE]]),
    # A median left as it is.
    area(8, rect(0, 80, 10, 82), "Refüj"),
    # Two kerbs of a median, the second running back.
    line(9, [(0, 100), (30, 100)]),
    line(10, [(30, 104), (0, 104)]),
    line(11, [(0, 120), (20, 120), (30, 125)]),
    line(12, [(0, 123), (19.2, 123), (28.6, 127.7)]),
    {"kind": "polyline", "id": 13, "layerId": "yapi", "attrs": {}, "pts": [P(0, 140), P(10, 140)],
     "parts": [{"pts": [P(0, 145), P(10, 145)]}]},
    line(14, [(0, 160), (10, 160)], layer="kilitli"),
    {"kind": "point", "id": 15, "layerId": "yapi", "attrs": {}, "p": P(0, 200)},
]
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k != "entities"}, "entities": ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}
BY = {e["id"]: e for e in ENTITIES}


def U(i):
    return f"$uidOf:{i}"


def summary(rings):
    """The runner's summary of an area: its corners sorted, its vertices' net area to a millionth, parts, holes."""
    def shoelace(r):
        ox, oy = r[0]
        n = len(r)
        return abs(sum((r[i][0] - ox) * (r[(i + 1) % n][1] - oy) - (r[(i + 1) % n][0] - ox) * (r[i][1] - oy)
                       for i in range(n))) / 2.0
    outer, holes = rings[0], rings[1:]
    net = shoelace(outer) - sum(shoelace(h) for h in holes)
    corners = sorted(p for r in rings for p in r)
    return {"corners": [[x, y] for x, y in corners], "area": round(net * 1e6) / 1e6, "parts": 1, "holes": len(holes)}


def out(changed=(), created=(), removed=(), count=0, skipped=0):
    # A removed object's id is captured before the step (`$uid:gone<N>`): its slot is empty after.
    return {"changed": [U(i) for i in changed], "created": [U(i) for i in created],
            "removed": [f"$uid:gone{i}" for i in removed],
            "count": count, "skipped": skipped, "revision": "$current"}


def done(name, work, output, undo, ids, shapes=None, fields=None, layer=None):
    inp = {"work": work}
    if layer:
        inp["layerId"] = layer
    expect = {"ids": ids, "canUndo": True, "dirty": True}
    if shapes:
        expect["shapes"] = shapes
    if fields:
        expect["fields"] = fields
    step = {"op": "execute", "input": inp, "within": WITHIN,
            "result": {"status": "completed", "output": output, "warnings": []}, "expect": expect}
    back = {"op": "undo", "returns": undo, "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    gone = sorted({int(u[len("$uid:gone"):]) for u in output["removed"]})
    captures = [{"op": "captureUid", "id": i, "as": f"gone{i}"} for i in gone]
    return {"name": name, "steps": captures + [step, back]}


def refused(name, work, code, message, path, layer=None, non_finite=None, extra=None):
    inp = {"work": work}
    if layer:
        inp["layerId"] = layer
    inp.update(extra or {})
    step = {"op": "execute", "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def num(v):
    return int(v) if float(v).is_integer() else v


# ── Plan yolu ────────────────────────────────────────────────────────────────────────────────────────────────────


def plan_case(name, axis, kind, width, kerb=0, median=0, axis_line=False):
    work = {"kind": "plan", "axis": [P(*p) for p in axis], "road": kind, "width": width}
    if kerb:
        work["kerb"] = kerb
    if median:
        work["median"] = median
    if axis_line:
        work["axisLine"] = True
    word = {"road": "Yol", "footway": "Yaya yolu", "cycleway": "Bisiklet yolu"}[kind]
    want = road_ref.road([P(*p) for p in axis], width, kerb, median)
    made = [(want["road"], word, width)]
    if "carriageway" in want:
        made.append((want["carriageway"], "Taşıt yolu", width - 2 * kerb))
    if "median" in want:
        made.append((want["median"], "Refüj", median))
    shapes, fields, n = {}, {}, NEXT
    for a, k, w in made:
        ring = [(p["x"], p["y"]) for p in a["outer"]["pts"]]
        shapes[str(n)] = summary([ring])
        fields[str(n)] = {"kind": "polygon", "layerId": "yapi", "attrs": {KIND: k, WIDTH: str(num(w))}}
        n += 1
    if axis_line:
        fields[str(n)] = {"kind": "polyline", "pts": [P(*p) for p in axis], "attrs": {KIND: "Yol ekseni"}}
        n += 1
    created = list(range(NEXT, n))
    return done(name, work, out(created=created, count=len(created)), "Plan yolu", IDS + created, shapes, fields,
                layer="yapi")


def hidden_case():
    c = plan_case("Plan yolu: gizli katmana uyarıyla", [(0, 480), (20, 480)], "footway", 4)
    step = c["steps"][0]
    step["input"]["layerId"] = "gizli"
    for f in step["expect"]["fields"].values():
        f["layerId"] = "gizli"
    step["result"]["warnings"] = [{"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne "
                                   "görünmeyecek.", "path": "layerId"}]
    return c


def plan_cases():
    return [
        plan_case("Plan yolu: düz yol", [(0, 300), (60, 300)], "road", 10),
        plan_case("Plan yolu: kırık eksen, kaldırımlı", [(0, 320), (40, 330), (70, 310)], "road", 15, kerb=3),
        plan_case("Plan yolu: refüjlü bulvar", [(0, 360), (50, 360), (90, 390)], "road", 30, kerb=4, median=2),
        plan_case("Plan yolu: yaya yolu ve ekseni", [(30, 420), (0, 420)], "footway", 5, axis_line=True),
        plan_case("Plan yolu: bisiklet yolu", [(0, 440), (20, 440), (20, 460)], "cycleway", 2.5),
        refused("Plan yolu: tek noktalı eksen", {"kind": "plan", "axis": [P(0, 0)], "road": "road", "width": 10},
                "too_few_points", "Eksen en az iki noktalı olmalı.", "work.axis", layer="yapi"),
        refused("Plan yolu: genişlik sıfır", {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 0},
                "invalid_distance", "Yolun genişliği sıfırdan büyük olmalı.", "work.width", layer="yapi"),
        refused("Plan yolu: kaldırımlar sığmıyor",
                {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10, "kerb": 5},
                "invalid_distance", "Kaldırımlar yolun genişliğine sığmıyor; Kaldırım'ı ya da Genişlik'i değiştirin.",
                "work.kerb", layer="yapi"),
        refused("Plan yolu: refüj sığmıyor",
                {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10, "kerb": 2, "median": 6},
                "invalid_distance", "Refüj taşıt yoluna sığmıyor; Refüj'ü ya da Genişlik'i değiştirin.",
                "work.median", layer="yapi"),
        refused("Plan yolu: kaldırım yalnız yolda",
                {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "footway", "width": 5, "kerb": 1},
                "invalid_input", "Kaldırım ve refüj yalnız Yol'da olur.", "work.kerb", layer="yapi"),
        refused("Plan yolu: katman yok", {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10},
                "invalid_input", "Plan yolu'nun katmanını verin (layerId).", "layerId"),
        refused("Plan yolu: kilitli katmana yazılmaz",
                {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10}, "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                "etkinleştirin.", "layerId", layer="kilitli"),
        refused("Plan yolu: olmayan katman", {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10},
                "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                "layerId", layer="yok"),
        refused("Plan yolu: katman değil grup",
                {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10}, "not_a_layer",
                "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId",
                layer="pafta"),
        hidden_case(),
        refused("Plan yolu: sürüm yazısı geçersiz",
                {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10}, "invalid_revision",
                "Beklenen sürüm “012” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                "ya da komutun planından alın.", "expectedRevision", layer="yapi", extra={"expectedRevision": "012"}),
        refused("Plan yolu: eksen sonlu değil",
                {"kind": "plan", "axis": [P(0, 0), P(10, 0)], "road": "road", "width": 10}, "not_finite",
                "Eksenin noktasının doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir "
                "sayıyla verin.", "work.axis[1].x", layer="yapi", non_finite={"work.axis[1].x": "NaN"}),
    ]


# ── Kavşak temizle ───────────────────────────────────────────────────────────────────────────────────────────────


def rounded(ring, holes, r):
    o, _, done_, left = road_ref.round_inner(ring, road_ref.signed(ring) > 0, r)
    rings = [o]
    for h in holes:
        hp, _, d2, l2 = road_ref.round_inner(h, road_ref.signed(h) < 0, r)
        rings.append(hp)
        done_ += d2
        left += l2
    return rings, done_, left


def junction_cases():
    plus, d1, l1 = rounded(road_ref.PLUS, [], 6)
    plus_k, d2, l2 = rounded([(x, y) for x, y in road_ref.PLUS], [], 3)
    # The carriageways' cross: 27..33 across 0..60 both ways.
    cross = [(27, 0), (33, 0), (33, 27), (60, 27), (60, 33), (33, 33), (33, 60), (27, 60), (27, 33), (0, 33),
             (0, 27), (27, 27)]
    cross_r, d3, l3 = rounded(cross, [], 2)
    tee, d4, l4 = rounded([(100 + x, y) for x, y in road_ref.TEE], [], 3)
    ring, d5, l5 = rounded([(200 + x, y) for x, y in road_ref.RING_OUT], [[(200 + x, y) for x, y in road_ref.RING_HOLE]], 5)
    del plus_k, d2, l2
    j = lambda ids, block, kerb: {"kind": "junctions", "ids": [U(i) for i in ids], "blockRadius": block,  # noqa: E731
                                  "kerbRadius": kerb}
    return [
        done("Kavşak temizle: iki sokak birleşir, iç köşeleri yuvarlanır", j([1, 2], 6, 3),
             out(changed=[1], removed=[2], count=d1, skipped=l1), "Kavşak temizle", [i for i in IDS if i != 2],
             shapes={"1": summary(plus)}),
        done("Kavşak temizle: türler ayrı birleşir, taşıt yolu kaldırım yarıçapıyla", j([1, 2, 3, 4], 6, 2),
             out(changed=[1, 3], removed=[2, 4], count=d1 + d3, skipped=l1 + l3), "Kavşak temizle",
             [i for i in IDS if i not in (2, 4)], shapes={"1": summary(plus), "3": summary(cross_r)}),
        done("Kavşak temizle: T kavşak", j([5, 6], 3, 2), out(changed=[5], removed=[6], count=d4, skipped=l4),
             "Kavşak temizle", [i for i in IDS if i != 6], shapes={"5": summary(tee)}),
        done("Kavşak temizle: adanın deliğinin köşeleri", j([7], 5, 2), out(changed=[7], count=d5, skipped=l5),
             "Kavşak temizle", IDS, shapes={"7": summary(ring)}),
        done("Kavşak temizle: refüj birleşmez", j([1, 2, 8], 6, 3),
             out(changed=[1], removed=[2], count=d1, skipped=l1), "Kavşak temizle", [i for i in IDS if i != 2],
             shapes={"1": summary(plus)}),
        refused("Kavşak temizle: yol alanı yok", j([8, 9], 6, 3), "not_an_area", "Seçimde yol alanı yok.", "work.ids"),
        refused("Kavşak temizle: yarıçap eksi", j([1, 2], -1, 3), "invalid_radius",
                "Köşe yarıçapı sıfır ya da büyük olmalı; 0 yuvarlamaz.", "work.blockRadius"),
    ]


# ── Refüj kapat ──────────────────────────────────────────────────────────────────────────────────────────────────


def median_case(name, a, b, rnd=True):
    pts_a = [(p["x"], p["y"]) for p in BY[a]["pts"]]
    pts_b = [(p["x"], p["y"]) for p in BY[b]["pts"]]
    pts, _ = road_ref.close_median(pts_a, None, pts_b, None, rnd)
    work = {"kind": "median", "first": U(a), "second": U(b)}
    if not rnd:
        work["round"] = False
    return done(name, work, out(changed=[a], removed=[b], count=1), "Refüj kapat", [i for i in IDS if i != b],
                shapes={str(a): summary([pts])}, fields={str(a): {"kind": "polygon"}})


def median_cases():
    return [
        median_case("Refüj kapat: ters yönlü kenar çevrilir, uçlar yarım daire", 9, 10),
        median_case("Refüj kapat: düz uçlar", 9, 10, rnd=False),
        median_case("Refüj kapat: kırık kenarlar", 11, 12),
        refused("Refüj kapat: alan kenar olamaz", {"kind": "median", "first": U(1), "second": U(10)}, "not_a_path",
                "Refüjün kenarı çizgi ya da çoklu çizgi olmalı.", "work.first"),
        refused("Refüj kapat: çok parçalı çizgi", {"kind": "median", "first": U(9), "second": U(13)}, "multi_part",
                "Çok parçalı çizgi refüjün kenarı olamaz; parçalarına ayırın.", "work.second"),
        refused("Refüj kapat: aynı çizgi", {"kind": "median", "first": U(9), "second": U(9)}, "invalid_input",
                "Refüjün iki ayrı kenarını verin.", "work.second"),
        refused("Refüj kapat: kilitli katmandaki kenar", {"kind": "median", "first": U(9), "second": U(14)},
                "layer_locked",
                "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.",
                "work"),
    ]


def general_cases():
    return [
        refused("Olmayan kimlik", {"kind": "median", "first": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "second": U(10)},
                "entity_not_found",
                "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
                "olabilir. Var olan bir nesnenin kimliğini verin.", "work.first"),
        refused("Kimlik değil", {"kind": "junctions", "ids": ["yol"], "blockRadius": 6, "kerbRadius": 3},
                "invalid_uid", "“yol” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "work.ids[0]"),
        {"name": "Plan: yazılmaz", "steps": [
            {"op": "plan", "input": {"work": {"kind": "median", "first": U(9), "second": U(10)}},
             "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": UNCHANGED}]},
        {"name": "Sürüm değiştiyse hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "base"},
            {"op": "execute", "input": {"work": {"kind": "median", "first": U(11), "second": U(12)}},
             "result": {"status": "completed", "output": "$any", "warnings": []}},
            {"op": "execute", "input": {"work": {"kind": "median", "first": U(9), "second": U(10)},
                                        "expectedRevision": "$base"},
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin "
                            "şimdiki hâline göre yeniden hazırlayın.",
                 "path": "expectedRevision", "revision": "$current"}},
             "expect": {"revision": "same"}}]},
    ]


def build():
    cases = plan_cases() + junction_cases() + median_cases() + general_cases()
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.roads.edit", "commandVersion": 1,
            "title": "Plan yolu çizimi",
            "note": "Written by scripts/fixtures/roads_edit_cases.py from plan_road_cases.py's rules (docs/adr/0622 §9, "
                    "0198); do not edit by hand.",
            "setup": SETUP, "cases": cases}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not FILE.exists() or FILE.read_text(encoding="utf-8") != text:
            print("cad.roads.edit durumları güncel değil; betiği --check'siz çalıştırın.")
            sys.exit(1)
        print(f"cad.roads.edit durumları güncel: {len(build()['cases'])} durum.")
        return
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
