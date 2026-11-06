#!/usr/bin/env python3
"""The shared cases of the drawing health check (cad.drawing.inspect with checks: health) and of cad.drawing.repair
(docs/adr/1005), written from the ADR without KentOS code.

    python3 scripts/fixtures/drawing_repair_cases.py           # writes fixtures/commands/v1/cad.drawing.repair.json
                                                               # and cad.drawing.inspect.health.json
    python3 scripts/fixtures/drawing_repair_cases.py --check   # writes nothing; compares

Health (ADR 1005 §1): a line's free end (no other straight edge's end within 1e-9) whose ray along the line meets the
inside of another object's straight edge within `maxGap` is short (the meeting ahead) or over (behind it, inside the
line); a straight edge within `maxAngle` of horizontal or vertical, not on it, deviates; with ten or more objects one
whose centre (its points' mean) lies farther than ten times the diagonal of the box of the 10th–90th percentile
centres (at least 0.1 m diagonal) from that box is stray; block definitions of one content (parts without their ids,
base, attributes) under different names are duplicates. Findings in that order; T junctions by line, its end before
its start.

Repair (ADR 1005 §2): a line's end moved along the line onto the nearest such meeting within `maxGap`; measured
(constraint or associative dimension) or locked geometry refused; all or nothing; one undo step “Onar”.
ADR 1012: a line turned about its pivot end onto the nearest axis (its other end at pivot + length × the axis'
unit vector), when off by at most `maxAngle` degrees (default 0.5); block definitions of one content merged into the
kept one (their inserts relinked, the merged definitions gone). Junctions first, then angles, then blocks; the
answer's recipe lists the steps in that order with the thresholds, the objects changed and the definitions removed.
"""

import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
BLOCK_A = "0192f5b0-6000-7000-8000-0000000000b1"
BLOCK_B = "0192f5b0-6000-7000-8000-0000000000b2"
BLOCK_C = "0192f5b0-6000-7000-8000-0000000000b3"
GHOST = "0192f5b0-6000-7000-8000-000000000999"


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": STYLE, "children": []}


LINES = {  # slot: (layer, a, b)
    1: ("cizim", (0.0, 0.0), (10.0, 0.0)),      # A: the base
    2: ("cizim", (5.0, 0.004), (5.0, 5.0)),     # B: 4 mm short of A
    3: ("cizim", (8.0, -0.006), (8.0, 5.0)),    # C: 6 mm over A
    4: ("cizim", (0.0, 3.0), (3.0, 3.02)),      # D: 0.38° off horizontal
    5: ("kilitli", (12.0, 0.003), (12.0, 4.0)), # E: locked, 3 mm short of F
    6: ("cizim", (11.0, 0.0), (15.0, 0.0)),     # F
    7: ("cizim", (20.0, 0.002), (20.0, 3.0)),   # G: 2 mm short of H
    8: ("cizim", (18.0, 0.0), (22.0, 0.0)),     # H
}
POINTS = {9: (1.0, 1.0), 10: (2.0, 2.0), 11: (3.0, 1.0), 12: (5000.0, 5000.0)}


def v(p):
    return {"x": p[0], "y": p[1]}


def setup():
    entities = [{"kind": "line", "id": i, "layerId": l, "attrs": {}, "a": v(a), "b": v(b)} for i, (l, a, b) in LINES.items()]
    entities += [{"kind": "point", "id": i, "layerId": "cizim", "attrs": {}, "p": v(p)} for i, p in POINTS.items()]
    part = {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}}
    blocks = [{"id": BLOCK_A, "name": "Kapı", "base": {"x": 0, "y": 0}, "entities": [part]},
              {"id": BLOCK_B, "name": "Kapı-kopya", "base": {"x": 0, "y": 0}, "entities": [dict(part, id=7)]},
              # Another content: a longer part.
              {"id": BLOCK_C, "name": "Pencere", "base": {"x": 0, "y": 0},
               "entities": [dict(part, id=1, b={"x": 2, "y": 0})]}]
    return {"format": "kentos.document", "version": 1, "name": "Çizim sağlığı",
            "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 100, "workspace": "cad", "drawingUnit": "m"},
            "origin": {"x": 0, "y": 0}, "layers": [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", True)],
            "activeLayer": "cizim", "blocks": blocks, "entities": entities, "styles": {"items": [], "categories": []}}


NEAR = 1e-9


def dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


SCOPE = [None]


def straights():
    return [(i, a, b) for i, (_, a, b) in LINES.items() if SCOPE[0] is None or i in SCOPE[0]]


def meet(e, d, a, b):
    ux, uy = b[0] - a[0], b[1] - a[1]
    den = d[0] * uy - d[1] * ux
    if abs(den) <= NEAR * dist(a, b):
        return None
    wx, wy = a[0] - e[0], a[1] - e[1]
    t = (wx * uy - wy * ux) / den
    u = (wx * d[1] - wy * d[0]) / den
    inside = NEAR / max(dist(a, b), NEAR)
    if inside < u < 1 - inside:
        return t, (e[0] + t * d[0], e[1] + t * d[1])
    return None


def junction(slot, which, gap):
    """The meeting of line `slot`'s end `which` with another line's inside within `gap`: (t, point, target)."""
    _, a, b = LINES[slot]
    end, frm = (b, a) if which == "end" else (a, b)
    length = dist(a, b)
    d = ((end[0] - frm[0]) / length, (end[1] - frm[1]) / length)
    best = None
    for j, p, q in straights():
        if j == slot:
            continue
        m = meet(end, d, p, q)
        if m and NEAR < abs(m[0]) <= gap and -m[0] < length and (best is None or abs(m[0]) < abs(best[0])):
            best = (m[0], m[1], j)
    return best


def free(slot, end):
    return not any(j != slot and (dist(p, end) <= NEAR or dist(q, end) <= NEAR) for j, p, q in straights())


def uid(slot):
    return f"$uidOf:{slot}"


def health(gap=0.01, angle=0.5, scope=None):
    SCOPE[0] = scope
    out = []
    lines = {i: l for i, l in LINES.items() if scope is None or i in scope}
    points = {i: p for i, p in POINTS.items() if scope is None or i in scope}
    for i, (_, a, b) in lines.items():
        for which, end in (("end", b), ("start", a)):
            if not free(i, end):
                continue
            m = junction(i, which, gap)
            if m:
                t, x, j = m
                code, words = ("t_junction_short", "kenara ulaşmıyor") if t > 0 else ("t_junction_overshoot", "kenarı geçiyor")
                out.append({"group": "health", "severity": "warning", "code": code, "uids": [uid(i), uid(j)],
                            "points": [v(end), v(x)],
                            "message": f"Çizginin {'bitiş' if which == 'end' else 'başlangıç'} ucu öbür nesnenin kenarına {abs(t):.4f} m {words}.",
                            "repair": "Ucu kesişime getirin: Uzat/Buda ya da cad.drawing.repair (ölçülü ve kısıtlı nesneye dokunmaz).",
                            "distance": abs(t)})
    tol = math.radians(angle)
    for i, (_, a, b) in lines.items():
        length = dist(a, b)
        ang = math.atan2(b[1] - a[1], b[0] - a[0])
        q = math.pi / 2
        dev = abs(ang - round(ang / q) * q)
        if 1e-12 < dev <= tol:
            out.append({"group": "health", "severity": "info", "code": "angle_deviation", "uids": [uid(i)],
                        "points": [v(a), v(b)],
                        "message": f"Kenar yatay ya da düşeyden {math.degrees(dev):.4f}° sapıyor; ucu {length * math.sin(dev):.4f} m kayık.",
                        "repair": "Bilerek eğikse bırakın; değilse Döndür ya da Ortogonal ile düzeltin.",
                        "distance": length * math.sin(dev)})
    centres = {i: ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2) for i, (_, a, b) in lines.items()}
    centres.update(points)
    xs = sorted(c[0] for c in centres.values())
    ys = sorted(c[1] for c in centres.values())
    pick = lambda s, f: s[int(math.floor((len(s) - 1) * f + 0.5))]
    x0, x1, y0, y1 = pick(xs, 0.1), pick(xs, 0.9), pick(ys, 0.1), pick(ys, 0.9)
    reach = 10 * max(math.hypot(x1 - x0, y1 - y0), 0.1)
    for i, c in (centres.items() if len(centres) >= 10 else []):
        dx = max(x0 - c[0], c[0] - x1, 0.0)
        dy = max(y0 - c[1], c[1] - y1, 0.0)
        away = math.hypot(dx, dy)
        if away > reach:
            out.append({"group": "health", "severity": "info", "code": "stray_object", "uids": [uid(i)], "points": [v(c)],
                        "message": f"Nesne çizimin ana bölgesinden {away:.3f} m uzakta; yanlışlıkla taşınmış ya da kopyalanmış olabilir.",
                        "repair": "Nesneye yakınlaştırıp inceleyin; gereksizse silin.", "distance": away})
    out.append({"group": "health", "severity": "info", "code": "block_definition_duplicate", "uids": [], "points": [],
                "message": "Aynı içerikli blok tanımları: Kapı, Kapı-kopya.",
                "repair": "Yerleştirmeleri tek tanıma bağlayıp öbürlerini kullanılmayan tanım olarak temizleyin."})
    SCOPE[0] = None
    return out


def inspected(findings, examined=12):
    return {"status": "completed", "output": {"findings": findings, "hatches": [], "unsupported": [], "examined": examined,
                                               "comparisons": "$any", "truncated": False, "revision": "$current"}, "warnings": []}


def health_cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    q = {"revision": "same", "canUndo": False}
    case("Sağlık: kısa ve taşan T, açı sapması, uzak nokta, aynı bloklar", [
        {"op": "execute", "input": {"checks": ["health"]}, "result": inspected(health()), "within": 1e-9, "expect": q}])
    case("Eşik 5 mm: 6 mm taşan ve kilitli dışındakiler", [
        {"op": "execute", "input": {"checks": ["health"], "maxGap": 0.005}, "result": inspected(health(gap=0.005)), "within": 1e-9, "expect": q}])
    case("Açı eşiği 0,1°: sapma bildirilmez", [
        {"op": "execute", "input": {"checks": ["health"], "maxAngle": 0.1}, "result": inspected(health(angle=0.1)), "within": 1e-9, "expect": q}])
    case("Açı eşiği 0: açı denetlenmez", [
        {"op": "execute", "input": {"checks": ["health"], "maxAngle": 0}, "result": inspected(health(angle=0)), "within": 1e-9, "expect": q}])
    case("Sağlık istenmezse varsayılan denetimler aynı kalır: sağlık bulgusu yok", [
        {"op": "execute", "input": {"checks": ["hatches"]}, "result": inspected([]), "expect": q}])
    def scoped(name, slots, **kw):
        case(name, [{"op": "execute", "input": {"checks": ["health"], "uids": [uid(s) for s in slots], **{
            {"gap": "maxGap", "angle": "maxAngle"}[k]: v for k, v in kw.items()}},
            "result": inspected(health(scope=set(slots), **kw), examined=len(slots)), "within": 1e-9, "expect": q}])
    scoped("Kapsam A ve B: kısa kalan uç", [1, 2])
    scoped("Kapsam A ve C: taşan uç", [1, 3])
    scoped("Kapsam E ve F: kilitli katmandaki kısa uç da bildirilir", [5, 6])
    scoped("Kapsam G ve H: 2 mm kısa", [7, 8])
    scoped("Kapsam D: yalnız açı sapması", [4])
    scoped("Kapsam dört nokta: onun altında uzak nokta aranmaz", [9, 10, 11, 12])
    scoped("Kapsam sekiz çizgi: uzak nokta yok", list(LINES))
    scoped("Eşik 3,5 mm: E ve G", list(LINES), gap=0.0035)
    scoped("Eşik 2,5 mm: yalnız G", list(LINES), gap=0.0025)
    scoped("Açı eşiği 0,4°: D bildirilir", [4], angle=0.4)
    scoped("Açı eşiği 0,38°: D bildirilmez", [4], angle=0.38)
    case("Sağlık ve tarama birlikte: tarama bulgusu yok, sağlık aynı", [
        {"op": "execute", "input": {"checks": ["health", "hatches"]}, "result": inspected(health()), "within": 1e-9, "expect": q}])
    case("Gizli katmanlar dahil: aynı", [
        {"op": "execute", "input": {"checks": ["health"], "includeHidden": True}, "result": inspected(health()), "within": 1e-9, "expect": q}])
    case("Plan aynı cevabı yazmadan verir", [
        {"op": "plan", "input": {"checks": ["health"]}, "result": inspected(health()), "within": 1e-9, "expect": q}])
    case("Açı eşiği 10°'den büyük: ret", [
        {"op": "execute", "input": {"checks": ["health"], "maxAngle": 11}, "result": {"status": "failed", "error": {
            "code": "invalid_angle_tolerance", "message": "Açı sapması eşiği 0 ile 10 derece arasında sonlu bir sayı olmalı.",
            "path": "maxAngle"}}, "expect": q}])
    case("Eksi açı eşiği: ret", [
        {"op": "execute", "input": {"checks": ["health"], "maxAngle": -1}, "result": {"status": "failed", "error": {
            "code": "invalid_angle_tolerance", "message": "Açı sapması eşiği 0 ile 10 derece arasında sonlu bir sayı olmalı.",
            "path": "maxAngle"}}, "expect": q}])
    return out


def repaired(fixes, gap=0.01):
    fixed = []
    lines = {i: [a, b] for i, (_, a, b) in LINES.items()}
    for slot, which, _target in fixes:
        a, b = lines[slot]
        t, x, _ = junction(slot, which, gap)
        end = b if which == "end" else a
        before = dist(a, b)
        if which == "end":
            lines[slot][1] = x
        else:
            lines[slot][0] = x
        fixed.append({"uid": uid(slot), "end": which, "moved": dist(end, x), "lengthBefore": before,
                      "lengthAfter": dist(*lines[slot])})
    steps = [{"kind": "junction", "uid": uid(s), "end": w, "target": uid(t)} for s, w, t in fixes]
    return {"fixed": fixed, "maxMoved": max(f["moved"] for f in fixed), "revision": "$current",
            "recipe": {"steps": steps, "maxGap": gap, "maxAngle": 0.5, "objects": len({f[0] for f in fixes}),
                       "definitionsRemoved": 0}}, lines


def turned(slot, pivot, lines):
    """Line `slot` turned about its `pivot` end onto the nearest axis: (new ends, deviation degrees, moved)."""
    a, b = lines[slot]
    p, q = (a, b) if pivot == "start" else (b, a)
    length = dist(p, q)
    angle = math.atan2(q[1] - p[1], q[0] - p[0])
    k = round(angle / (math.pi / 2))
    unit = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)][k % 4]
    to = (p[0] + length * unit[0], p[1] + length * unit[1])
    ends = [p, to] if pivot == "start" else [to, p]
    return ends, abs(angle - k * math.pi / 2) * 180 / math.pi, dist(q, to)


def entity(slot, a, b):
    return {"kind": "line", "id": slot, "layerId": LINES[slot][0], "attrs": {}, "a": v(a), "b": v(b)}


def fix(slot, which, target):
    return {"uid": uid(slot), "end": which, "target": uid(target)}


def refused(input_, code, message, path, conflict=False):
    err = {"code": code, "message": message, "path": path}
    if conflict:
        err["revision"] = "$current"
    return {"op": "execute", "input": input_, "result": {"status": "conflict" if conflict else "failed", "error": err},
            "expect": {"revision": "same", "canUndo": False}}


def repair_cases():
    out = []

    def case(name, steps):
        out.append({"name": name, "steps": steps})

    def ok(fixes, gap=None):
        answer, lines = repaired(fixes, gap or 0.01)
        input_ = {"fixes": [fix(*f) for f in fixes]}
        if gap is not None:
            input_["maxGap"] = gap
        touched = {f[0] for f in fixes}
        return {"op": "execute", "input": input_, "result": {"status": "completed", "output": answer, "warnings": []},
                "within": 1e-9, "expect": {"revision": "changed", "canUndo": True,
                                           "entities": {str(s): entity(s, *lines[s]) for s in touched}}}

    case("Kısa kalan B'nin başlangıcı A'ya: 4 mm, boy 5 m", [ok([(2, "start", 1)])])
    case("Taşan C'nin başlangıcı A'ya: 6 mm geri, boy 5 m", [ok([(3, "start", 1)])])
    case("İkisi birlikte, tek geri alma", [ok([(2, "start", 1), (3, "start", 1)]),
        {"op": "undo", "returns": "Onar", "expect": {"entities": {"2": entity(2, *LINES[2][1:]), "3": entity(3, *LINES[3][1:])}}}])
    case("Onarımdan sonra sağlık denetimi B için bulgu vermez", [ok([(2, "start", 1)]),
        {"op": "execute", "command": "cad.drawing.inspect", "input": {"checks": ["health"], "uids": [uid(1), uid(2)]},
         "result": inspected([f for f in health(scope={1, 2}) if f["code"] == "block_definition_duplicate"], examined=2),
         "expect": {"revision": "same"}}])
    answer, _ = repaired([(2, "start", 1)])
    case("Plan yazmaz, kaymayı söyler", [{"op": "plan", "input": {"fixes": [fix(2, "start", 1)]},
        "result": {"status": "completed", "output": answer, "warnings": []}, "within": 1e-9, "expect": {"revision": "same", "canUndo": False}}])
    case("Doğrulama yazmaz", [{"op": "validate", "input": {"fixes": [fix(2, "start", 1)]},
        "result": {"status": "completed", "output": None, "warnings": []}, "expect": {"revision": "same", "canUndo": False}}])
    case("Kilitli katmandaki E: ret", [refused({"fixes": [fix(5, "start", 6)]}, "layer_locked",
        "“Kilitli” katmanı kilitli; çizgi onarılamaz, hiçbir şey yazılmadı. Kilidi Katmanlar panelinden açın.", "fixes[0].uid")])
    measured = "Çizgi bir kısıtın ya da ilişkili ölçünün kaynağı; ölçülmüş geometri onarımla taşınmaz. Ucu kısıt ya da parametreyle düzeltin."
    case("Kısıtlı G: ret, ölçülmüş geometri taşınmaz", [
        {"op": "execute", "command": "cad.constraints.edit", "input": {"edit": {"kind": "add", "constraints": [
            {"rule": {"kind": "fix", "a": {"uid": uid(7), "element": {"kind": "point", "place": {"kind": "end"}}}}}]}},
         "result": "$any", "expect": {"revision": "changed"}},
        dict(refused({"fixes": [fix(7, "start", 8)]}, "repair_refused_measured", measured, "fixes[0].uid"),
             expect={"revision": "same", "canUndo": True})])
    missing = lambda gap: f"Çizginin ucu {gap} m içinde hedefin bir doğru kenarına değmiyor; denetimi yenileyin ya da eşiği büyütün."
    case("B'nin öbür ucu A'ya ulaşmaz: ret", [refused({"fixes": [fix(2, "end", 1)]}, "repair_target_missing", missing("0.01"), "fixes[0].target")])
    case("Eşik 1 mm: 4 mm'lik uç ulaşmaz: ret", [refused({"fixes": [fix(2, "start", 1)], "maxGap": 0.001}, "repair_target_missing",
        missing("0.001"), "fixes[0].target")])
    gap_words = "Onarım eşiği 0'dan büyük, en çok 1000 metre olmalı."
    case("Eşik 0: ret", [refused({"fixes": [fix(2, "start", 1)], "maxGap": 0}, "invalid_gap", gap_words, "maxGap")])
    case("Eşik 2000 m: ret", [refused({"fixes": [fix(2, "start", 1)], "maxGap": 2000}, "invalid_gap", gap_words, "maxGap")])
    nan = refused({"fixes": [fix(2, "start", 1)], "maxGap": 0.01}, "invalid_gap", gap_words, "maxGap")
    nan["nonFinite"] = {"maxGap": "NaN"}
    case("Eşik NaN: ret", [nan])
    case("Boş liste: ret", [refused({"fixes": []}, "invalid_input", "Onarım 1 ile 1000 düzeltme alır; 0 verildi.", "fixes")])
    case("Aynı uç iki kez: ret", [refused({"fixes": [fix(2, "start", 1), fix(2, "start", 1)]}, "invalid_input",
        "“$uidOf:2” çizgisinin aynı ucu iki kez verildi.", "fixes[1]")])
    bad_uid = "“X” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği denetimin bulgusundan alın."
    case("Kimlik UUID değil: ret", [refused({"fixes": [{"uid": "X", "end": "start", "target": uid(1)}]}, "invalid_uid", bad_uid, "fixes[0].uid")])
    case("Hedef UUID değil: ret", [refused({"fixes": [{"uid": uid(2), "end": "start", "target": "X"}]}, "invalid_uid", bad_uid, "fixes[0].target")])
    gone = f"“{GHOST}” kimlikli nesne çizimde yok; denetimi yenileyin."
    case("Olmayan çizgi: ret", [refused({"fixes": [{"uid": GHOST, "end": "start", "target": uid(1)}]}, "entity_not_found", gone, "fixes[0].uid")])
    case("Olmayan hedef: ret", [refused({"fixes": [{"uid": uid(2), "end": "start", "target": GHOST}]}, "entity_not_found", gone, "fixes[0].target")])
    case("Nokta onarılmaz: ret", [refused({"fixes": [fix(9, "start", 1)]}, "repair_unsupported",
        "Onarım yalnız çizgi ucunu taşır; bu nesne çizgi değil.", "fixes[0].uid")])
    case("Kendi kenarına: ret", [refused({"fixes": [fix(2, "start", 2)]}, "repair_unsupported",
        "Çizgi kendi kenarına getirilemez; başka bir hedef verin.", "fixes[0].target")])
    case("Beklenen revizyon eski: çatışma", [refused({"fixes": [fix(2, "start", 1)], "expectedRevision": "999"}, "revision_conflict",
        "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
        "expectedRevision", conflict=True)])
    case("Beklenen revizyon sayı değil: ret", [refused({"fixes": [fix(2, "start", 1)], "expectedRevision": "abc"}, "invalid_revision",
        "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
        "expectedRevision")])
    case("Biri kilitli: hiçbiri onarılmaz", [refused({"fixes": [fix(2, "start", 1), fix(5, "start", 6)]}, "layer_locked",
        "“Kilitli” katmanı kilitli; çizgi onarılamaz, hiçbir şey yazılmadı. Kilidi Katmanlar panelinden açın.", "fixes[1].uid")])

    # ── ADR 1012: angles and blocks ──
    def angled(angles, fixes=(), max_angle=None):
        answer, lines = repaired(list(fixes)) if fixes else ({"fixed": [], "maxMoved": 0.0, "revision": "$current",
            "recipe": {"steps": [], "maxGap": 0.01, "maxAngle": 0.5, "objects": 0, "definitionsRemoved": 0}},
            {i: [a, b] for i, (_, a, b) in LINES.items()})
        straightened = []
        for slot, pivot in angles:
            ends, dev, moved = turned(slot, pivot, lines)
            lines[slot] = ends
            straightened.append({"uid": uid(slot), "pivot": pivot, "deviation": dev, "moved": moved})
            answer["recipe"]["steps"].append({"kind": "angle", "uid": uid(slot), "pivot": pivot})
        answer["straightened"] = straightened
        if max_angle is not None:
            answer["recipe"]["maxAngle"] = max_angle
        answer["recipe"]["objects"] = len({f[0] for f in fixes} | {s for s, _ in angles})
        input_ = {"angles": [{"uid": uid(s), "pivot": p} for s, p in angles]}
        if fixes:
            input_["fixes"] = [fix(*f) for f in fixes]
        if max_angle is not None:
            input_["maxAngle"] = max_angle
        touched = {f[0] for f in fixes} | {s for s, _ in angles}
        return {"op": "execute", "input": input_, "result": {"status": "completed", "output": answer, "warnings": []},
                "within": 1e-9, "expect": {"revision": "changed", "canUndo": True,
                                           "entities": {str(s): entity(s, *lines[s]) for s in touched}}}

    case("Eğik D başlangıcı çevresinde yataya: boyu korunur; tek geri alma", [angled([(4, "start")]),
        {"op": "undo", "returns": "Onar", "expect": {"entities": {"4": entity(4, *LINES[4][1:])}}}])
    case("Eğik D bitişi çevresinde yataya", [angled([(4, "end")])])
    case("Önce uç, sonra açı: tarif sırası ve iki nesne", [angled([(4, "start")], fixes=[(2, "start", 1)])])
    case("Açı eşiği 1°: D döner", [angled([(4, "start")], max_angle=1.0)])
    dev = turned(4, "start", {i: [a, b] for i, (_, a, b) in LINES.items()})[1]
    case("Açı eşiği 0,1°: D 0,38° sapıyor, döndürülmez", [refused({"angles": [{"uid": uid(4), "pivot": "start"}], "maxAngle": 0.1},
        "repair_angle_too_large", f"Çizgi eksenden {dev:.4f}° sapıyor; eşik 0.1°. Bilerek eğikse bırakın, değilse eşiği büyütün ya da Döndür'le düzeltin.",
        "angles[0].uid")])
    tol_words = "Açı eşiği 0'dan büyük, en çok 10 derece olmalı."
    case("Açı eşiği 0: ret", [refused({"angles": [{"uid": uid(4), "pivot": "start"}], "maxAngle": 0}, "invalid_angle_tolerance", tol_words, "maxAngle")])
    case("Açı eşiği 20°: ret", [refused({"angles": [{"uid": uid(4), "pivot": "start"}], "maxAngle": 20}, "invalid_angle_tolerance", tol_words, "maxAngle")])
    nan_angle = refused({"angles": [{"uid": uid(4), "pivot": "start"}], "maxAngle": 0.5}, "invalid_angle_tolerance", tol_words, "maxAngle")
    nan_angle["nonFinite"] = {"maxAngle": "NaN"}
    case("Açı eşiği NaN: ret", [nan_angle])
    case("Aynı çizgi iki kez döndürülmez: ret", [refused({"angles": [{"uid": uid(4), "pivot": "start"}, {"uid": uid(4), "pivot": "end"}]},
        "invalid_input", "“$uidOf:4” çizgisi iki kez döndürülemez.", "angles[1]")])
    case("Nokta döndürülmez: ret", [refused({"angles": [{"uid": uid(9), "pivot": "start"}]}, "repair_unsupported",
        "Onarım yalnız çizgi ucunu taşır; bu nesne çizgi değil.", "angles[0].uid")])
    case("Kilitli E döndürülmez: ret", [refused({"angles": [{"uid": uid(5), "pivot": "start"}]}, "layer_locked",
        "“Kilitli” katmanı kilitli; çizgi onarılamaz, hiçbir şey yazılmadı. Kilidi Katmanlar panelinden açın.", "angles[0].uid")])

    insert = {"op": "execute", "command": "cad.entities.create",
              "input": {"layerId": "cizim", "objects": [{"geometry": {"kind": "insert", "block": BLOCK_B, "p": v((30.0, 30.0)),
                                                                     "scale": 1, "rotation": 0}}]},
              "result": "$any", "expect": {"revision": "changed"}}
    merged = {"fixed": [], "maxMoved": 0.0, "revision": "$current",
              "merged": [{"keep": "Kapı", "merged": ["Kapı-kopya"], "inserts": 1}],
              "recipe": {"steps": [{"kind": "blocks", "keep": "Kapı", "merge": ["Kapı-kopya"]}], "maxGap": 0.01,
                         "maxAngle": 0.5, "objects": 1, "definitionsRemoved": 1}}
    gone_block = "“Kapı-kopya” adlı blok tanımı çizimde yok; denetimi yenileyin."
    case("Kapı-kopya Kapı'da birleşir: yerleştirmesi Kapı'ya bağlanır, kopya tanım gider", [insert,
        {"op": "captureUid", "id": 13, "as": "kapi"},
        {"op": "execute", "input": {"blocks": [{"keep": "Kapı", "merge": ["Kapı-kopya"]}]},
         "result": {"status": "completed", "output": merged, "warnings": []},
         "expect": {"revision": "changed", "canUndo": True,
                    "entities": {"13": {"kind": "insert", "id": 13, "layerId": "cizim", "attrs": {}, "block": BLOCK_A,
                                        "p": v((30.0, 30.0)), "scale": 1, "rotation": 0}}}},
        refused({"blocks": [{"keep": "Kapı", "merge": ["Kapı-kopya"]}]}, "unknown_block", gone_block, "blocks[0].merge[0]")
        | {"expect": {"revision": "same", "canUndo": True}}])
    plan_merged = dict(merged, merged=[{"keep": "Kapı", "merged": ["Kapı-kopya"], "inserts": 0}],
                       recipe=dict(merged["recipe"], objects=0))
    case("Birleştirmenin planı yazmaz", [{"op": "plan", "input": {"blocks": [{"keep": "Kapı", "merge": ["Kapı-kopya"]}]},
        "result": {"status": "completed", "output": plan_merged, "warnings": []}, "expect": {"revision": "same", "canUndo": False}}])
    case("İçeriği farklı Pencere birleşmez: ret", [refused({"blocks": [{"keep": "Kapı", "merge": ["Pencere"]}]}, "repair_blocks_differ",
        "“Pencere” ile “Kapı” tanımlarının içeriği aynı değil; biri öbürünün yerine geçemez.", "blocks[0].merge[0]")])
    case("Olmayan tanım tutulamaz: ret", [refused({"blocks": [{"keep": "Yok", "merge": ["Kapı-kopya"]}]}, "unknown_block",
        "“Yok” adlı blok tanımı çizimde yok; denetimi yenileyin.", "blocks[0].keep")])
    case("Aynı tanım iki kez: ret", [refused({"blocks": [{"keep": "Kapı", "merge": ["Kapı"]}]}, "invalid_input",
        "“Kapı” tanımı iki kez verildi; bir tanım bir birleştirmede bir kez yer alır.", "blocks[0].merge[0]")])
    case("Boş birleştirme: ret", [refused({"blocks": [{"keep": "Kapı", "merge": []}]}, "invalid_input",
        "Bir birleştirme 1 ile 100 tanım alır; 0 verildi.", "blocks[0].merge")])
    return out


NOTE = ("ADR 1005. Bağımsız başvuru scripts/fixtures/drawing_repair_cases.py: kurallar ADR'den elle yazılır; KentOS kodu yok.")


def doc(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": setup(), "cases": cases}


def main():
    texts = {
        OUT / "cad.drawing.repair.json": doc("cad.drawing.repair", "Çizimi onar", repair_cases()),
        OUT / "cad.drawing.inspect.health.json": doc("cad.drawing.inspect", "Çizim sağlığı", health_cases()),
    }
    texts = {p: json.dumps(t, ensure_ascii=False, indent=1) + "\n" for p, t in texts.items()}
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in texts.items() if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} güncel değil.")
        if bad:
            return 1
        print("Çizim sağlığı ve onarım ortak durumları güncel.")
        return 0
    for p, t in texts.items():
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
