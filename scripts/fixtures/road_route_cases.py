#!/usr/bin/env python3
"""The shared cases of `road.routes.find` and `road.routes.draw` (plan/road.md Y12.1, docs/adr/0910 §1; KentOS Yol's
preliminary routes).

The terrain's heights are road_profile_cases.py's TIN (the surface's own reference); the grid, the least-cost search
over the 16 neighbours (a step's plan length plus `climb` times its rise or fall, steps steeper than `grade` not
taken, the least cost first and the least node among equal ones, a cost kept unless strictly bettered, the neighbours
tried in a fixed order), the nodes' heights rounded to the micrometre, the areas avoided (even–odd, the vertices joined
straight) and Douglas–Peucker are ADR 0910 §1's. No KentOS code. The limits are the cases' own, not a standard's.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_route_cases.py [--check]
"""

import copy
import heapq
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = ccmd.UNCHANGED, ccmd.STEPPED, ccmd.CONFLICT, ccmd.BAD_REVISION
W = 1e-6
TIN = pcmd.TIN
NOT_ROAD = failed("not_a_road_project",
                  "Ön güzergâh yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
STEPS = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (-1, -1), (1, -1),
         (2, 1), (1, 2), (-1, 2), (-2, 1), (-2, -1), (-1, -2), (1, -2), (2, -1)]
# An area to avoid across the ridge, with a hole, and a second one.
YASAK = [{"x": 250.37, "y": 110.37}, {"x": 330.37, "y": 110.37}, {"x": 330.37, "y": 270.37}, {"x": 250.37, "y": 270.37}]
HOLE = [{"x": 270.37, "y": 160.37}, {"x": 310.37, "y": 160.37}, {"x": 310.37, "y": 200.37}, {"x": 270.37, "y": 200.37}]
GOL = [{"x": 150.37, "y": 40.37}, {"x": 210.37, "y": 30.37}, {"x": 220.37, "y": 90.37}, {"x": 160.37, "y": 100.37}]


def setup(work="road"):
    base = grc.setup([], work)
    base["layers"] = base["layers"] + [grc.node("yasak", "Yasak alan"), grc.node("guzergah", "Güzergâh")]
    n = len(base["entities"])
    base["entities"] = base["entities"] + [
        {"kind": "polygon", "id": n + 1, "layerId": "yasak", "attrs": {}, "pts": YASAK, "holes": [{"pts": HOLE}]},
        {"kind": "polygon", "id": n + 2, "layerId": "yasak", "attrs": {}, "pts": GOL}]
    base["uids"] = base["uids"] + [f"0192f5a0-7c3e-7d4a-9b1e-{900000000000 + k:012d}" for k in (1, 2)]
    return base


FIRST = len(setup()["entities"]) + 1
ROAD = setup()["settings"]["road"]


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, words):
    return Refused("invalid_route", words, path)


def inside(ring, p):
    odd = False
    n = len(ring)
    for k in range(n):
        a, b = ring[k], ring[(k + n - 1) % n]
        if (a["y"] > p[1]) != (b["y"] > p[1]) and p[0] < (b["x"] - a["x"]) * (p[1] - a["y"]) / (b["y"] - a["y"]) + a["x"]:
            odd = not odd
    return odd


def rounded(z):
    return math.floor(z * 1e6 + 0.5) / 1e6


def find(ask, entities=None):
    s = ask["surface"]
    surfaces = ROAD["surfaces"]
    sdef = next((x for x in surfaces if x["id"] == s), None) or next(
        (x for x in surfaces if x["name"].lower() == s.lower()), None)
    if sdef is None:
        raise Refused("unknown_terrain", f"“{s}” kimlikli ya da adlı yüzey projede yok.", "surface")
    cell, margin, grade, climb = ask["cell"], ask["margin"], ask["grade"], ask["climb"]
    if not 0.1 <= cell <= 1000:
        raise bad("cell", "Hücre 0,1 ile 1000 m arası olur.")
    if not 0 <= margin <= 100000:
        raise bad("margin", "Pay 0 ile 100 000 m arası olur.")
    if not 0 < grade <= 1000:
        raise bad("grade", "En dik eğim 0'dan büyük, en çok %1000 olur.")
    if not 0 <= climb <= 1000:
        raise bad("climb", "Tırmanmanın ağırlığı 0 ile 1000 arası olur.")
    if ask.get("tolerance") is not None and not 0 <= ask["tolerance"] <= 1000:
        raise bad("tolerance", "Sadeleştirme toleransı 0 ile 1000 m arası olur.")
    a, b = (ask["from"]["x"], ask["from"]["y"]), (ask["to"]["x"], ask["to"]["y"])
    ox, oy = min(a[0], b[0]) - margin, min(a[1], b[1]) - margin
    cols = math.ceil((abs(a[0] - b[0]) + 2 * margin) / cell) + 1
    rows = math.ceil((abs(a[1] - b[1]) + 2 * margin) / cell) + 1
    if cols * rows > 2_000_000:
        raise bad("cell", "Izgara iki milyondan çok düğüm olur; hücreyi büyütün ya da payı küçültün.")
    layers = {node["id"] for node in all_nodes(setup()["layers"])}
    for i, lid in enumerate(ask.get("avoid", [])):
        if lid not in layers:
            raise bad(f"avoid[{i}]", f"“{lid}” kimlikli katman çizimde yok.")
    if sdef["id"] != "t-1":
        raise bad("surface", f"“{sdef['name']}” yüzeyi kurulamıyor.")
    areas = []
    for e in (entities if entities is not None else setup()["entities"]):
        if e["kind"] == "polygon" and e["layerId"] in ask.get("avoid", []):
            areas.append([e["pts"]] + [h["pts"] for h in e.get("holes", [])])
    place = lambda n: (ox + (n % cols) * cell, oy + (n // cols) * cell)  # noqa: E731
    heights = []
    for n in range(cols * rows):
        p = place(n)
        z = TIN.height(p[0], p[1])
        free = not any(inside(ar[0], p) and not any(inside(h, p) for h in ar[1:]) for ar in areas)
        heights.append(rounded(z) if z is not None and free else None)

    def nearest(p):
        i = min(max(math.floor((p[0] - ox) / cell + 0.5), 0), cols - 1)
        j = min(max(math.floor((p[1] - oy) / cell + 0.5), 0), rows - 1)
        return j * cols + i
    na, nb = nearest(a), nearest(b)
    if heights[na] is None:
        raise Refused("route_blocked", "Rotanın başı yüzeyin dışında ya da kaçınılacak bir alanda.", "from")
    if heights[nb] is None:
        raise Refused("route_blocked", "Rotanın sonu yüzeyin dışında ya da kaçınılacak bir alanda.", "to")
    lengths = [math.sqrt((di * cell) * (di * cell) + (dj * cell) * (dj * cell)) for di, dj in STEPS]
    dist = [math.inf] * (cols * rows)
    frm = [-1] * (cols * rows)
    done = [False] * (cols * rows)
    dist[na] = 0.0
    heap = [(0.0, na)]
    settled = 0
    steepest = grade / 100.0
    while heap:
        d, u = heapq.heappop(heap)
        if done[u] or d > dist[u]:
            continue
        done[u] = True
        settled += 1
        if u == nb:
            break
        ui, uj = u % cols, u // cols
        zu = heights[u]
        for k, (di, dj) in enumerate(STEPS):
            vi, vj = ui + di, uj + dj
            if vi < 0 or vj < 0 or vi >= cols or vj >= rows:
                continue
            v = vj * cols + vi
            zv = heights[v]
            if zv is None or done[v]:
                continue
            rise = abs(zv - zu)
            if rise > steepest * lengths[k] + 1e-9:
                continue
            nxt = d + (lengths[k] + climb * rise)
            if nxt < dist[v]:
                dist[v] = nxt
                frm[v] = u
                heapq.heappush(heap, (nxt, v))
    if math.isinf(dist[nb]):
        raise Refused("route_unreached", f"İki yer arasında %{num(grade)} eğimi aşmayan yol yok.", "grade")
    nodes = [nb]
    while nodes[-1] != na:
        nodes.append(frm[nodes[-1]])
    nodes.reverse()
    line = [[*place(n), heights[n]] for n in nodes]
    if len(line) == 1:
        line = [[a[0], a[1], heights[na]], [b[0], b[1], heights[nb]]]
    else:
        line[0] = [a[0], a[1], heights[na]]
        line[-1] = [b[0], b[1], heights[nb]]
    kept = simplified(line, ask.get("tolerance") or 0.0)
    length = climb_ = steep = 0.0
    for p, q in zip(kept, kept[1:]):
        dx, dy = q[0] - p[0], q[1] - p[1]
        lng = math.sqrt(dx * dx + dy * dy)
        rise = abs(q[2] - p[2])
        length += lng
        climb_ += rise
        if lng > 0:
            steep = max(steep, rise / lng * 100.0)
    return {"points": [{"x": p[0], "y": p[1], "z": p[2]} for p in kept], "length": length, "climb": climb_,
            "steepest": steep, "cost": dist[nb], "nodes": cols * rows, "settled": settled}


def num(v):
    return ccmd.num(v)


def all_nodes(nodes):
    for n in nodes:
        yield n
        yield from all_nodes(n.get("children", []))


def simplified(line, tol):
    if len(line) < 3 or tol <= 0:
        return [list(p) for p in line]
    keep = [False] * len(line)
    keep[0] = keep[-1] = True
    stack = [(0, len(line) - 1)]
    while stack:
        i, j = stack.pop()
        a, b = line[i], line[j]
        dx, dy = b[0] - a[0], b[1] - a[1]
        span = math.sqrt(dx * dx + dy * dy)
        far, at = 0.0, 0
        for k in range(i + 1, j):
            p = line[k]
            d = abs((p[0] - a[0]) * dy - (p[1] - a[1]) * dx) / span if span > 0 else \
                math.sqrt((p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)
            if d > far:
                far, at = d, k
        if far > tol:
            keep[at] = True
            stack.append((at, j))
            stack.append((i, at))
    return [list(p) for p, k in zip(line, keep) if k]


ASK = {"surface": "Mevcut zemin", "from": {"x": 70.0, "y": 80.0}, "to": {"x": 520.0, "y": 280.0}, "cell": 10.0,
       "margin": 30.0, "grade": 8.0, "climb": 10.0}


def find_cases():
    cases = []

    def ok(name, ask):
        out = find(ask)
        s = step("execute", ask, completed(out), {**UNCHANGED, "road": ROAD})
        s["within"] = W
        cases.append(case(name, [s]))

    def no(name, ask, setup_=None):
        try:
            find(ask)
        except Refused as r:
            cases.append(case(name, [step("execute", ask, r.result, UNCHANGED)], setup_))
            return
        raise AssertionError(f"{name}: refuses")

    ok("Vadiden sırta", ASK)
    ok("Yüzeyin kimliğiyle", {**ASK, "surface": "t-1"})
    ok("Yasak alandan kaçarak", {**ASK, "avoid": ["yasak"]})
    ok("Sadeleştirerek", {**ASK, "avoid": ["yasak"], "tolerance": 3.0})
    ok("Tırmanma pahalı", {**ASK, "climb": 100.0})
    ok("Tırmanma bedava", {**ASK, "climb": 0.0})
    ok("Dik eğime izin", {**ASK, "grade": 30.0})
    ok("İri ızgara", {**ASK, "cell": 25.0, "margin": 25.0})
    ok("Payısız", {**ASK, "margin": 0.0, "cell": 15.0})
    ok("Ters yönde", {**ASK, "from": ASK["to"], "to": ASK["from"]})
    ok("Kısa yol", {**ASK, "to": {"x": 95.0, "y": 85.0}})
    ok("Aynı düğüm", {**ASK, "to": {"x": 72.0, "y": 81.0}})
    ok("Deliğin içinde kalan", {**ASK, "from": {"x": 281.0, "y": 171.0}, "to": {"x": 299.0, "y": 189.0},
                                "avoid": ["yasak"], "margin": 5.0, "cell": 5.0})
    ok("Boş katmandan kaçmak", {**ASK, "avoid": ["korkuluk"]})
    cases.append(case("Yol projesi değil", [step("execute", ASK, NOT_ROAD, UNCHANGED)], setup("map")))
    no("Olmayan yüzey", {**ASK, "surface": "yok"})
    no("Kurulamayan yüzey", {**ASK, "surface": "Kurulamaz"})
    no("Sıfır hücre", {**ASK, "cell": 0.0})
    no("Sıfır eğim", {**ASK, "grade": 0.0})
    no("Eksi tırmanma", {**ASK, "climb": -1.0})
    no("Çok büyük ızgara", {**ASK, "cell": 0.2})
    no("Olmayan katman", {**ASK, "avoid": ["yok"]})
    no("Başı yasak alanda", {**ASK, "from": {"x": 260.0, "y": 130.0}, "avoid": ["yasak"]})
    no("Sonu yüzeyin dışında", {**ASK, "to": {"x": 300.0, "y": 450.0}, "margin": 0.0})
    no("Eğim yetmiyor", {**ASK, "grade": 0.5})
    no("Delikten çıkılamaz", {**ASK, "from": {"x": 290.0, "y": 180.0}, "to": {"x": 420.0, "y": 200.0}, "avoid": ["yasak"]})
    return cases


def draw_cases():
    cases = []
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}

    def ok(name, ask, op="execute", undo=False, extra=()):
        route = find(ask)
        out = {"route": route, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        obj = {"pts": [{"x": p["x"], "y": p["y"]} for p in route["points"]], "zs": [p["z"] for p in route["points"]]}
        warns = list(extra)
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": ROAD}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": ROAD}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": ROAD, "ids": list(range(1, FIRST + 1)), "fields": {str(FIRST): obj}}
        s = step(op, ask, result, expect)
        s["within"] = W
        steps = [s]
        if undo:
            steps.append({"op": "undo", "returns": "Ön güzergâh", "expect": {"ids": list(range(1, FIRST)), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, ask, result, setup_=None, road=True):
        cases.append(case(name, [step("execute", ask, result, {**UNCHANGED, "road": ROAD if road else None})], setup_))

    base = {**ASK, "avoid": ["yasak"], "tolerance": 3.0, "layerId": "guzergah"}
    ok("Ön güzergâh planda", base)
    ok("Sadeleştirmeden", {k: v for k, v in base.items() if k != "tolerance"})
    ok("Yasaksız", {k: v for k, v in base.items() if k != "avoid"})
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, extra=[hidden])
    ok("Gruptaki katmana", {**base, "layerId": "yol-korkuluk"})
    ok("Planı", base, op="plan")
    ok("Doğrulama", base, op="validate")
    ok("Tek geri alma adımı", base, undo=True)
    ok("Dik eğimle", {**base, "grade": 30.0})
    ok("Ters yönde", {**base, "from": ASK["to"], "to": ASK["from"]})
    ok("İri ızgara", {**base, "cell": 25.0, "margin": 25.0})
    ok("Tırmanma pahalı", {**base, "climb": 100.0})
    for name, change in (("Eğim yetmiyor", {"grade": 0.5}), ("Sıfır hücre", {"cell": 0.0}),
                         ("Başı yasak alanda", {"from": {"x": 260.0, "y": 130.0}})):
        try:
            find({**base, **change})
        except Refused as r:
            no(name, {**base, **change}, r.result)
    no("Yol projesi değil", base, NOT_ROAD, setup("map"), road=False)
    no("Olmayan yüzey", {**base, "surface": "yok"},
       failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Olmayan katman", {**base, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Katman grubu", {**base, "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.",
              "layerId"))
    no("Kilitli katman", {**base, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Geçersiz beklenen sürüm", {**base, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**base, "expectedRevision": "41"}, CONFLICT)
    return cases


def file(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_route_cases.py yazar; arazinin kotları road_profile_cases.py'nin TIN'inden, "
                    "ızgara, arama, yasak alanlar ve sadeleştirme ADR 0910 §1'den; KentOS kodu yok. Sınırlar "
                    "durumlarındır, bir standardın değil.",
            "setup": setup(), "cases": cases}


def main():
    files = {"road.routes.find": file("road.routes.find", "Ön güzergâh bul", find_cases()),
             "road.routes.draw": file("road.routes.draw", "Ön güzergâh çiz", draw_cases())}
    check = "--check" in sys.argv
    for name, data in files.items():
        path = OUT / f"{name}.json"
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if check:
            if path.read_text(encoding="utf-8") != text:
                raise SystemExit(f"{path} güncel değil: python3 {Path(__file__).name}")
            print(f"{path.name}: {len(data['cases'])} durum güncel")
        else:
            path.write_text(text, encoding="utf-8")
            print(f"{path.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
