"""The shared cases of cad.shapes.edit (docs/adr/0622 §9): Biçim değiştir, Sürdür, Paralel kaydır, Sadeleştir, Kot ver
and Okunur yap.

    python3 scripts/fixtures/shape_edits_cases.py           # writes the file
    python3 scripts/fixtures/shape_edits_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.shapes.edit.json, without KentOS code. The shapes are straight edges on whole metres,
so the reference needs no rounding:

- Biçim değiştir (docs/adr/0173 §2–§3): an area's cells with the cells the sketch closes against its boundary added
  (it goes out and comes back) or taken (it goes in and comes back), its corners the cell corners; a line's stretch
  between the sketch's first and last meetings replaced by the sketch, the line a polyline in its place;
- Sürdür (§4): the path's points with the drawn ones after its end (or before its start), its own elevations kept
  and the new vertices' none;
- Paralel kaydır (docs/adr/0191): the edge nearest the point moved along its normal (an area's outward, a polyline's
  right of its walking direction), its neighbours' lines met; a target area's distance the one that gives it;
- Sadeleştir: a vertex on the straight line of its neighbours dropped, its deviation 0;
- Kot ver (docs/adr/0142): Sabit every vertex, Artır only the vertices that have one, Sıfırla none; a point, a line,
  a polyline and an area take them;
- Okunur yap (docs/adr/0145 §6): a text whose rotation is above 90° and at most 270° turned half round about its
  box's middle: its point moves w·(1 − 2a) along and h·(0.92 − 2b) up its old baseline (a, b the alignment's shares),
  for a middle-centred text h·(0.92 − 1) up only.
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
sys.path.insert(0, str(Path(__file__).resolve().parent))
import area_rules_cases as cellwork  # noqa: E402  (cells and corners: no KentOS code)

POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000
FILE = OUT / "cad.shapes.edit.json"


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def W(p):
    return {"x": X + p[0], "y": Y + p[1]}


A = rect(0, 0, 10, 10)
K = rect(0, 20, 10, 30)
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "yapi", "attrs": {"Ada": "101"}, "pts": [W(p) for p in A]},
    {"kind": "line", "id": 2, "layerId": "yapi", "attrs": {}, "a": W((20, 0)), "b": W((30, 0)), "za": 1, "zb": 2},
    {"kind": "polyline", "id": 3, "layerId": "yapi", "attrs": {}, "pts": [W(p) for p in [(40, 0), (50, 0), (50, 10)]]},
    {"kind": "polygon", "id": 4, "layerId": "kilitli", "attrs": {}, "pts": [W(p) for p in K]},
    {"kind": "point", "id": 5, "layerId": "yapi", "attrs": {}, "p": W((60, 0)), "z": 5},
    {"kind": "text", "id": 6, "layerId": "yapi", "attrs": {}, "p": W((70, 0)), "text": "Ada 5", "height": 1.25,
     "rotation": 180, "align": "middleCenter"},
    {"kind": "text", "id": 7, "layerId": "yapi", "attrs": {}, "p": W((70, 5)), "text": "Ada 6", "height": 1.25,
     "rotation": 0},
    {"kind": "polyline", "id": 8, "layerId": "yapi", "attrs": {},
     "pts": [W(p) for p in [(80, 0), (85, 0), (90, 0), (90, 5)]]},
    {"kind": "circle", "id": 9, "layerId": "yapi", "attrs": {}, "c": W((100, 5)), "r": 5},
    {"kind": "line", "id": 10, "layerId": "yapi", "attrs": {"Tür": "Sınır"}, "a": W((20, 20)), "b": W((30, 20))},
    {"kind": "polyline", "id": 11, "layerId": "yapi", "attrs": {}, "pts": [W(p) for p in [(110, 0), (120, 0)]],
     "parts": [{"pts": [W(p) for p in [(110, 5), (120, 5)]]}]},
]
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k != "entities"}, "entities": ENTITIES}
IDS = [e["id"] for e in ENTITIES]
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}


def U(i):
    return f"$uidOf:{i}"


def shape(cs):
    cn = sorted(cellwork.corners(cs))
    return {"corners": [[X + x, Y + y] for x, y in cn], "area": len(cs), "parts": len(cellwork.groups(cs)),
            "holes": len(cellwork.holes(cs))}


def edit(work):
    return {"work": work}


def out(changed=(), before=0, after=0, objects=1, skipped=0, count=0, distance=0):
    return {"changed": [U(i) for i in changed], "before": before, "after": after, "objects": objects,
            "skipped": skipped, "count": count, "distance": distance, "revision": "$current"}


def done(name, work, output, undo, shapes=None, fields=None):
    step = {"op": "execute", "input": edit(work), "result": {"status": "completed", "output": output, "warnings": []},
            "expect": {"ids": IDS, "canUndo": True, "dirty": True}}
    if shapes:
        step["expect"]["shapes"] = shapes
    if fields:
        step["expect"]["fields"] = fields
    back = {"op": "undo", "returns": undo, "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def refused(name, work, code, message, path, non_finite=None, extra=None):
    step = {"op": "execute", "input": {**edit(work), **(extra or {})}}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def length(pts):
    return sum(math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:]))


# ── Biçim değiştir ───────────────────────────────────────────────────────────────────────────────────────────────


def reshape_cases():
    grown = cellwork.cells(A) | cellwork.cells(rect(10, 2, 14, 8))
    cut = cellwork.cells(A) - cellwork.cells(rect(0, 2, 4, 8))
    line_after = [(20, 20), (22, 20), (22, 23), (26, 23), (26, 20), (30, 20)]
    return [
        done("Biçim değiştir: alanın dışına çıkıp dönen hat cep ekler",
             {"kind": "reshape", "id": U(1), "line": [W(p) for p in [(10, 2), (14, 2), (14, 8), (10, 8)]]},
             out(changed=[1], before=100, after=len(grown)), "Biçim değiştir", shapes={"1": shape(grown)}),
        done("Biçim değiştir: alanın içine girip dönen hat keser",
             {"kind": "reshape", "id": U(1), "line": [W(p) for p in [(0, 2), (4, 2), (4, 8), (0, 8)]]},
             out(changed=[1], before=100, after=len(cut)), "Biçim değiştir", shapes={"1": shape(cut)}),
        done("Biçim değiştir: çizgi buluşmalar arası değişir, çoklu çizgi olur",
             {"kind": "reshape", "id": U(10), "line": [W(p) for p in [(22, 20), (22, 23), (26, 23), (26, 20)]]},
             out(changed=[10], before=10, after=length(line_after)), "Biçim değiştir",
             fields={"10": {"kind": "polyline", "pts": [W(p) for p in line_after], "attrs": {"Tür": "Sınır"}}}),
        refused("Biçim değiştir: alana değmeyen hat",
                {"kind": "reshape", "id": U(1), "line": [W(p) for p in [(20, 40), (25, 45)]]}, "reshape_refused",
                "Hat alanın sınırını iki kez kesmeli ya da ona değmeli.", "work.line"),
        refused("Biçim değiştir: tek noktalı hat", {"kind": "reshape", "id": U(1), "line": [W((10, 5))]},
                "too_few_points", "Hat en az iki noktalı olmalı.", "work.line"),
        refused("Biçim değiştir: çok parçalı çoklu çizgi",
                {"kind": "reshape", "id": U(11), "line": [W(p) for p in [(112, 0), (112, 2), (116, 2), (116, 0)]]},
                "multi_part", "Bu işlem çok parçalı nesnede çalışmaz; önce Parçalara ayır ile parçalarına ayırın.",
                "work.id"),
        refused("Biçim değiştir: daire alınmaz",
                {"kind": "reshape", "id": U(9), "line": [W(p) for p in [(95, 0), (95, 10)]]}, "not_a_path",
                "Biçim değiştir alan, çizgi ve çoklu çizgi içindir.", "work.id"),
        refused("Biçim değiştir: kilitli katmandaki alan",
                {"kind": "reshape", "id": U(4), "line": [W(p) for p in [(10, 22), (14, 22), (14, 28), (10, 28)]]},
                "layer_locked",
                "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.",
                "work"),
        refused("Biçim değiştir: hattın noktası sonlu değil",
                {"kind": "reshape", "id": U(1), "line": [W(p) for p in [(10, 2), (14, 2), (14, 8), (10, 8)]]},
                "not_finite",
                "Hattın noktasının kuzey (X) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir "
                "sayıyla verin.", "work.line[1].y", non_finite={"work.line[1].y": "Infinity"}),
    ]


# ── Sürdür ───────────────────────────────────────────────────────────────────────────────────────────────────────


def continue_cases():
    return [
        done("Sürdür: çizgi sonundan, çoklu çizgi olur",
             {"kind": "continue", "id": U(10), "pts": [W((35, 20))]},
             out(changed=[10], before=10, after=15, count=1), "Sürdür",
             fields={"10": {"kind": "polyline", "pts": [W(p) for p in [(20, 20), (30, 20), (35, 20)]],
                            "attrs": {"Tür": "Sınır"}}}),
        done("Sürdür: başından, iki nokta",
             {"kind": "continue", "id": U(10), "fromStart": True, "pts": [W((20, 25)), W((15, 25))]},
             out(changed=[10], before=10, after=20, count=2), "Sürdür",
             fields={"10": {"pts": [W(p) for p in [(15, 25), (20, 25), (20, 20), (30, 20)]]}}),
        done("Sürdür: kotlu çizginin köşeleri kotunu korur, yeniler kotsuz",
             {"kind": "continue", "id": U(2), "pts": [W((35, 0))]},
             out(changed=[2], before=10, after=15, count=1), "Sürdür",
             fields={"2": {"pts": [W(p) for p in [(20, 0), (30, 0), (35, 0)]], "zs": [1, 2, None]}}),
        done("Sürdür: çoklu çizgiye sıfır bükümle",
             {"kind": "continue", "id": U(3), "pts": [W((40, 10))], "bulges": [0]},
             out(changed=[3], before=20, after=30, count=1), "Sürdür",
             fields={"3": {"pts": [W(p) for p in [(40, 0), (50, 0), (50, 10), (40, 10)]]}}),
        refused("Sürdür: büküm sayısı noktalarınkinden farklı",
                {"kind": "continue", "id": U(3), "pts": [W((40, 10))], "bulges": [0, 0]}, "bulge_count",
                "Çizilen her kenarın bir bükümü olmalı: 1 nokta için 2 büküm verildi.", "work.bulges"),
        refused("Sürdür: nokta yok", {"kind": "continue", "id": U(3), "pts": []}, "too_few_points",
                "Sürdürmek için en az bir nokta verin.", "work.pts"),
        refused("Sürdür: alan sürdürülmez", {"kind": "continue", "id": U(1), "pts": [W((0, -5))]}, "not_a_path",
                "Sürdür çizgi ve çoklu çizgi içindir.", "work.id"),
    ]


# ── Paralel kaydır ───────────────────────────────────────────────────────────────────────────────────────────────


def edge_shift_cases():
    return [
        done("Paralel kaydır: alanın kenarı dışarı 2 m",
             {"kind": "edgeShift", "id": U(1), "at": W((10, 5)), "distance": 2},
             out(changed=[1], before=100, after=120, distance=2), "Paralel kaydır",
             shapes={"1": shape(cellwork.cells(rect(0, 0, 12, 10)))}),
        done("Paralel kaydır: alanın kenarı içeri 3 m",
             {"kind": "edgeShift", "id": U(1), "at": W((5, 10)), "distance": -3},
             out(changed=[1], before=100, after=70, distance=-3), "Paralel kaydır",
             shapes={"1": shape(cellwork.cells(rect(0, 0, 10, 7)))}),
        done("Paralel kaydır: hedef alan 150 m²",
             {"kind": "edgeShift", "id": U(1), "at": W((10, 5)), "area": 150},
             out(changed=[1], before=100, after=150, distance=5), "Paralel kaydır",
             shapes={"1": shape(cellwork.cells(rect(0, 0, 15, 10)))}),
        done("Paralel kaydır: çoklu çizginin kenarı yürüme yönünün sağına",
             {"kind": "edgeShift", "id": U(3), "at": W((45, 0)), "distance": 1},
             out(changed=[3], before=20, after=21, distance=1), "Paralel kaydır",
             fields={"3": {"pts": [W(p) for p in [(40, -1), (50, -1), (50, 10)]]}}),
        refused("Paralel kaydır: daire", {"kind": "edgeShift", "id": U(9), "at": W((105, 5)), "distance": 1},
                "edge_shift_refused",
                "Paralel kaydır alanın ya da çoklu çizginin düz kenarında çalışır; böyle bir kenara tıklayın.",
                "work.id"),
        refused("Paralel kaydır: hem uzaklık hem alan",
                {"kind": "edgeShift", "id": U(1), "at": W((10, 5)), "distance": 1, "area": 120}, "invalid_input",
                "Kenarı bir uzaklıkla (distance) ya da hedef alanla (area) kaydırın; ikisi birden değil.",
                "work.distance"),
        refused("Paralel kaydır: çoklu çizgide hedef alan",
                {"kind": "edgeShift", "id": U(3), "at": W((45, 0)), "area": 50}, "edge_shift_refused",
                "Hedef alan yalnız alanlarda yazılır.", "work.area"),
    ]


# ── Sadeleştir, Kot ver, Okunur yap ──────────────────────────────────────────────────────────────────────────────


def simplify_cases():
    return [
        done("Sadeleştir: doğru üzerindeki köşe atılır",
             {"kind": "simplify", "ids": [U(8)], "tolerance": 0.01},
             out(changed=[8], count=1), "Sadeleştir",
             fields={"8": {"pts": [W(p) for p in [(80, 0), (90, 0), (90, 5)]]}}),
        done("Sadeleştir: atılacak köşesi olmayanlar sayılır",
             {"kind": "simplify", "ids": [U(8), U(1), U(9)], "tolerance": 0.01},
             out(changed=[8], skipped=2, count=1), "Sadeleştir"),
        refused("Sadeleştir: tolerans sıfır", {"kind": "simplify", "ids": [U(8)], "tolerance": 0}, "invalid_tolerance",
                "Tolerans sıfırdan büyük olmalı.", "work.tolerance"),
        refused("Sadeleştir: atılacak köşe yok", {"kind": "simplify", "ids": [U(1)], "tolerance": 0.01},
                "nothing_to_simplify", "Bu toleransla atılacak köşe yok; daha büyük bir tolerans yazın.", "work.ids"),
        refused("Sadeleştir: çoklu çizgi ya da alan yok", {"kind": "simplify", "ids": [U(9)], "tolerance": 0.01},
                "not_a_path",
                "Seçimde çoklu çizgi ya da kapalı alan yok; Sadeleştir yalnız onların köşelerini atar.", "work.ids"),
    ]


def elevation_cases():
    return [
        done("Kot ver, Sabit: alan ve nokta, yazı atlanır",
             {"kind": "elevation", "ids": [U(1), U(5), U(6)], "elevation": {"mode": "set", "z": 100}},
             out(changed=[1, 5], objects=2, skipped=1), "Kot ver",
             fields={"1": {"zs": [100, 100, 100, 100]}, "5": {"z": 100}}),
        done("Kot ver, Artır: yalnız kotu olan köşeler",
             {"kind": "elevation", "ids": [U(2), U(1)], "elevation": {"mode": "raise", "by": 1.5}},
             out(changed=[2], objects=1, skipped=1), "Kot ver", fields={"2": {"za": 2.5, "zb": 3.5}}),
        done("Kot ver, Sıfırla",
             {"kind": "elevation", "ids": [U(2), U(5)], "elevation": {"mode": "clear"}},
             out(changed=[2, 5], objects=2), "Kot ver", fields={"2": {"za": None, "zb": None}, "5": {"z": None}}),
        refused("Kot ver, Artır: kotlu köşe yok",
                {"kind": "elevation", "ids": [U(1)], "elevation": {"mode": "raise", "by": 1}}, "no_elevation",
                "Seçili nesnelerin hiçbir köşesinde kot yok; fark eklenecek bir şey bulunamadı.", "work.ids"),
        refused("Kot ver: kot alan nesne yok",
                {"kind": "elevation", "ids": [U(6), U(9)], "elevation": {"mode": "set", "z": 1}}, "not_a_path",
                "Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.", "work.ids"),
        refused("Kot ver: kot sonlu değil",
                {"kind": "elevation", "ids": [U(1)], "elevation": {"mode": "set", "z": 1}}, "not_finite",
                "Kot sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "work.elevation.z",
                non_finite={"work.elevation.z": "NaN"}),
    ]


def readable_cases():
    # Middle-centred: a = 0.5 moves it nothing along; b = 0.5 moves it h·(0.92 − 1) up its old baseline, which at
    # 180° points down the page: v = (−sin π, cos π).
    t = (180 * math.pi) / 180
    up = 1.25 * (0.92 - 2.0 * 0.5)
    p = {"x": X + 70 + math.cos(t) * 0.0 + (-math.sin(t)) * up, "y": Y + 0 + math.sin(t) * 0.0 + math.cos(t) * up}
    return [
        done("Okunur yap: baş aşağı yazı yarım tur döner",
             {"kind": "readable", "ids": [U(6), U(7)]},
             out(changed=[6], objects=1, skipped=1, count=1), "Okunur yap",
             fields={"6": {"rotation": 0, "p": p}}),
        {"name": "Okunur yap: hepsi okunuyorsa bir şey yazılmaz", "steps": [
            {"op": "execute", "input": edit({"kind": "readable", "ids": [U(7)]}),
             "result": {"status": "completed", "output": out(objects=0, skipped=1) | {"revision": "$current"},
                        "warnings": []},
             "expect": UNCHANGED}]},
        refused("Okunur yap: yazı yok", {"kind": "readable", "ids": [U(1)]}, "not_a_text",
                "Seçimde yazı yok. Okunur yap yazı nesnelerini çevirir; yazıları seçip yeniden deneyin.", "work.ids"),
    ]


def general_cases():
    return [
        refused("Olmayan kimlik", {"kind": "readable", "ids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]},
                "entity_not_found",
                "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
                "olabilir. Var olan bir nesnenin kimliğini verin.", "work.ids[0]"),
        refused("Kimlik değil", {"kind": "reshape", "id": "alan-1", "line": [W((0, 0)), W((1, 1))]}, "invalid_uid",
                "“alan-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "work.id"),
        refused("Sürüm yazısı geçersiz", {"kind": "readable", "ids": [U(6)]}, "invalid_revision",
                "Beklenen sürüm “012” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü "
                "belgeden ya da komutun planından alın.", "expectedRevision", extra={"expectedRevision": "012"}),
        {"name": "Sürüm değiştiyse hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "base"},
            {"op": "execute", "input": edit({"kind": "readable", "ids": [U(6)]}),
             "result": {"status": "completed", "output": "$any", "warnings": []}},
            {"op": "execute", "input": {**edit({"kind": "simplify", "ids": [U(8)], "tolerance": 0.01}),
                                        "expectedRevision": "$base"},
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin "
                            "şimdiki hâline göre yeniden hazırlayın.",
                 "path": "expectedRevision", "revision": "$current"}},
             "expect": {"revision": "same"}}]},
        {"name": "Plan: sadeleştirme yazılmaz", "steps": [
            {"op": "plan", "input": edit({"kind": "simplify", "ids": [U(8)], "tolerance": 0.01}),
             "result": {"status": "completed", "output": {"changed": "$any", "created": [], "removed": [],
                                                           "revision": "$current"}, "warnings": []},
             "expect": UNCHANGED}]},
    ]


def build():
    cases = reshape_cases() + continue_cases() + edge_shift_cases() + simplify_cases() + elevation_cases() \
        + readable_cases() + general_cases()
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.shapes.edit", "commandVersion": 1,
            "title": "Biçim işlemleri",
            "note": "Written by scripts/fixtures/shape_edits_cases.py from the edits' definitions (docs/adr/0622 §9, "
                    "0173, 0191, 0142, 0145); do not edit by hand.",
            "setup": SETUP, "cases": cases}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not FILE.exists() or FILE.read_text(encoding="utf-8") != text:
            print("cad.shapes.edit durumları güncel değil; betiği --check'siz çalıştırın.")
            sys.exit(1)
        print(f"cad.shapes.edit durumları güncel: {len(build()['cases'])} durum.")
        return
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
