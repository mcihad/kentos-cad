"""Çizimi temizle: shared cases of cad.entities.cleanup (docs/adr/0305).

    python3 scripts/fixtures/cleanup_command_cases.py           # writes the file
    python3 scripts/fixtures/cleanup_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.cleanup.json. The ADR's rules, computed here without KentOS code; distances
are sqrt(dx·dx + dy·dy), rounded at each step, in the ADR's order:

- the scope: the objects named, in their order (each once), those on a locked layer left out with a warning; with none
  named, every object on a visible, unlocked layer in the drawing's order;
- empty objects (a line, an arc's radius, a circle's radius, a path's reach from its first vertex within the tolerance;
  a text of no words) go;
- duplicates: an object of the kind, own properties (but the ignored) and data of a kept one whose points match it
  within the tolerance (a point; a line's ends either way, elevations and measures with them; a circle's centre and
  radius; an arc's centre, radius and ends; a path's vertices in order, reversed or, closed, from any start, its
  bulges matching under that turn, elevations, measures and widths only in order) goes; the first match is looked
  for among the kept ones whose found points (a line's ends, an open path's ends, a closed path's vertices, a point, a
  centre) lie in the probe's cell (2·tolerance wide, at least 1 µm) or a neighbour, the kept ones in their order; the
  other kinds go when every field but the base's is equal;
- lines without elevations or measures of one group on one line that overlap (share a length; with endToEnd also meet
  end to end within the tolerance) merge into the first: a line joins the first cluster (in creation order) whose first line has both its ends within the tolerance of
  it and its ends within the tolerance of the first line's, and whose span along the first line meets the line's;
  clusters bridged by a line merge into the earlier; the first line reaches the farthest ends along it;
- a path's vertices that add nothing go (the ADR's rule); the kept path takes the one non-straight edge among the
  dropped ones after each kept vertex, and widths from the first and last edge of the dropped run.
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.cleanup.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
LABEL = "Çizimi temizle"
GHOST = "0192f5a2-0000-7000-8000-00000000ffff"
DEFAULT_TOL = 1e-6
BASE = ["id", "layerId", "color", "attrs", "label", "symbol", "lineWeight", "labelPins", "attachments", "lineType",
        "lineTypeScale", "transparency", "order", "byLayer"]
IGNORE = {"layer": "layerId", "color": "color", "lineType": "lineType", "lineTypeScale": "lineTypeScale",
          "lineWeight": "lineWeight", "transparency": "transparency", "symbol": "symbol"}


def p(x, y):
    return {"x": x, "y": y}


def node(i, name, locked=False, visible=True):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def base(i, layer="cizim", **more):
    return {"id": i, "layerId": layer, "attrs": {}, **more}


def line(i, a, b, **more):
    return {"kind": "line", **base(i, **more), "a": p(*a), "b": p(*b)}


def path(i, pts, kind="polyline", **more):
    fields = {k: v for k, v in more.items() if k in ("bulges", "zs", "ms", "widths", "closed")}
    own = {k: v for k, v in more.items() if k not in fields}
    return {"kind": kind, **base(i, **own), "pts": [p(*q) for q in pts], **fields}


ENTITIES = [
    line(1, (0, 0), (10, 0)),
    line(2, (10, 0), (0, 0)),
    line(3, (0, 0.0005), (10, 0)),
    line(4, (5, 0), (15, 0)),
    line(5, (15, 0), (20, 0)),
    line(6, (0, 5), (0, 5)),
    line(7, (0, 0), (10, 0), color="#ff0000"),
    {"kind": "circle", **base(8), "c": p(30, 0), "r": 2},
    {"kind": "circle", **base(9), "c": p(30, 0.0004), "r": 2.0003},
    {"kind": "arc", **base(10), "c": p(40, 0), "r": 3, "a0": 0, "a1": 1.5},
    {"kind": "arc", **base(11), "c": p(40, 0), "r": 3, "a0": 0, "a1": 1.5},
    path(12, [(0, 10), (0, 10), (5, 10), (10, 10), (10, 15)]),
    path(13, [(10, 15), (10, 10), (5, 10), (0, 10), (0, 10)]),
    path(14, [(0, 20), (10, 20), (10, 30), (0, 30)], kind="polygon"),
    path(15, [(10, 30), (10, 20), (0, 20), (0, 30)], kind="polygon"),
    path(16, [(0, 40), (5, 40), (10, 40)], zs=[None, 7, None]),
    path(17, [(0, 50), (0, 50), (10, 50)], bulges=[0, 0.5, 0]),
    path(18, [(0, 60), (5, 60), (10, 60)], widths=[[0.1, 0.1], [0.2, 0.2], [0, 0]]),
    path(19, [(0, 70), (5, 70), (10, 70)], widths=[[0.1, 0.2], [0.2, 0.3], [0, 0]]),
    {"kind": "text", **base(20), "p": p(50, 10), "text": "  ", "height": 1, "rotation": 0},
    line(21, (0, 80), (5, 80), layer="kilitli"),
    line(22, (0, 90), (5, 90), layer="gizli"),
    line(23, (0, 0), (10, 0), attrs={"Ad": "yol"}),
    path(24, [(20, 20), (25, 20), (30, 20), (30, 30), (20, 30)], closed=True),
    {"kind": "point", **base(25), "p": p(60, 0)},
    {"kind": "point", **base(26), "p": p(60, 0.0001)},
    line(27, (100, 0), (100.0004, 0)),
    line(28, (3, 0), (4, 0)),
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
LOCKED = {"kilitli"}
HIDDEN = {"gizli"}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Çizimi temizle",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(0, 0),
        "layers": [node("cizim", "Çizim"), node("kilitli", "Kilitli", locked=True), node("gizli", "Gizli", visible=False)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


# ── Geometry ───────────────────────────────────────────────────────────────────────────────────────────────────────

def pt(q):
    return (float(q["x"]), float(q["y"]))


def far(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    return math.sqrt(dx * dx + dy * dy)


def off_line(q, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    length = math.sqrt(dx * dx + dy * dy)
    if length == 0.0:
        return far(q, a)
    return abs((q[0] - a[0]) * dy - (q[1] - a[1]) * dx) / length


def apart(a, b):
    if len(a) != len(b):
        return None
    m = 0.0
    for q, r in zip(a, b):
        m = max(m, far(q, r))
    return m


def same_points(a, b, closed, tol):
    n = len(a)
    if n != len(b) or n == 0:
        return None
    rev = list(reversed(b))
    if not closed:
        for pts, turn in ((b, ("same", 0)), (rev, ("reversed", 0))):
            d = apart(a, pts)
            if d is not None and d <= tol:
                return d, turn
        return None
    for pts, name in ((b, "from"), (rev, "reversedFrom")):
        for start in range(n):
            shifted = [pts[(start + i) % n] for i in range(n)]
            d = apart(a, shifted)
            if d is not None and d <= tol:
                return d, (name, start)
    return None


def thin(pts, bulges, closed, tol, straight, keeps):
    n = len(pts)
    least = 3 if closed else 2
    if n <= least:
        return None

    def bulge(i):
        return bulges[i] if bulges is not None and i < len(bulges) else 0.0

    kept, loss = [0], 0.0
    inner = n if closed else n - 1
    for i in range(1, inner):
        prev, nxt = kept[-1], (i + 1) % n
        drop = None
        if not keeps(i) and bulge(i - 1) == 0.0:
            d = far(pts[i], pts[prev])
            if bulge(i) == 0.0:
                off = off_line(pts[i], pts[prev], pts[nxt])
                ux, uy = pts[nxt][0] - pts[prev][0], pts[nxt][1] - pts[prev][1]
                span = math.sqrt(ux * ux + uy * uy)
                t = 0.0 if span == 0.0 else ((pts[i][0] - pts[prev][0]) * ux + (pts[i][1] - pts[prev][1]) * uy) / span
                if (straight or d <= tol) and off <= tol and -tol <= t <= span + tol:
                    drop = off
            elif d <= tol and prev == i - 1:
                drop = d
        if drop is None:
            kept.append(i)
        else:
            loss = max(loss, drop)
    if not closed:
        end = n - 1
        prev = kept[-1]
        if prev == end - 1 and prev != 0 and not keeps(prev) and bulge(prev) == 0.0:
            d = far(pts[end], pts[prev])
            if d <= tol:
                loss = max(loss, d)
                kept.pop()
        kept.append(end)
    return (kept, loss) if len(kept) < n and len(kept) >= least else None


def kept_bulges(bulges, kept, n, closed):
    out = []
    for j, k in enumerate(kept):
        to = kept[j + 1] if j + 1 < len(kept) else (n if closed else k + 1)
        b = 0.0
        for i in range(k, to):
            v = bulges[i % n] if i % n < len(bulges) else 0.0
            if v != 0.0:
                b = v
                break
        out.append(b)
    return out


def merge_lines(lines, groups, tol, touching):
    def unit(i):
        a, b = lines[i]
        length = far(a, b)
        return ((b[0] - a[0]) / length, (b[1] - a[1]) / length)

    def along(i, q):
        ux, uy = unit(i)
        a = lines[i][0]
        return (q[0] - a[0]) * ux + (q[1] - a[1]) * uy

    def on(k, j):
        a, b = lines[k]
        return off_line(lines[j][0], a, b) <= tol and off_line(lines[j][1], a, b) <= tol

    clusters = []  # [kept, members, lo, hi]
    for i in range(len(lines)):
        if far(lines[i][0], lines[i][1]) <= tol:
            continue
        a, b = lines[i]
        joined = None
        for c in range(len(clusters)):
            cl = clusters[c]
            if not cl[1]:
                continue
            k = cl[0]
            if groups[k] != groups[i] or not on(k, i) or not on(i, k):
                continue
            s, e = along(k, a), along(k, b)
            s, e = min(s, e), max(s, e)
            meets = (s <= cl[3] + tol and e >= cl[2] - tol) if touching else (s < cl[3] and e > cl[2])
            if not meets:
                continue
            if joined is None:
                cl[1].append(i)
                cl[2], cl[3] = min(cl[2], s), max(cl[3], e)
                joined = c
            else:
                keep, take = (c, joined) if clusters[c][0] < clusters[joined][0] else (joined, c)
                moved, clusters[take][1] = clusters[take][1], []
                kk = clusters[keep][0]
                for m in moved:
                    for q in lines[m]:
                        t = along(kk, q)
                        clusters[keep][2] = min(clusters[keep][2], t)
                        clusters[keep][3] = max(clusters[keep][3], t)
                clusters[keep][1].extend(moved)
                joined = keep
        if joined is None:
            clusters.append([i, [i], 0.0, far(a, b)])
    out = []
    for cl in clusters:
        if len(cl[1]) < 2:
            continue
        members = sorted(cl[1])
        k = members[0]
        lo = (0.0, (k, 0))
        hi = (far(lines[k][0], lines[k][1]), (k, 1))
        loss = 0.0
        for m in members[1:]:
            for end, q in enumerate(lines[m]):
                t = along(k, q)
                if t < lo[0]:
                    lo = (t, (m, end))
                if t > hi[0]:
                    hi = (t, (m, end))
                loss = max(loss, off_line(q, lines[k][0], lines[k][1]))
        ka, (ux, uy) = lines[k][0], unit(k)
        point = lambda t: (ka[0] + ux * t, ka[1] + uy * t)  # noqa: E731
        na = lines[k][0] if lo[1] == (k, 0) else point(lo[0])
        nb = lines[k][1] if hi[1] == (k, 1) else point(hi[0])
        out.append({"kept": k, "absorbed": members[1:], "a": na, "b": nb, "loss": loss})
    return sorted(out, key=lambda m: m["kept"])


# ── The rules ──────────────────────────────────────────────────────────────────────────────────────────────────────

class Refused(Exception):
    def __init__(self, code, message, path, status="failed"):
        super().__init__(message)
        self.code, self.message, self.path, self.status = code, message, path, status


def key(e, ignore):
    b = {k: e[k] for k in BASE if k in e and k not in ("id", "order")}
    for i in ignore:
        b.pop(IGNORE[i], None)
    return (e["kind"], json.dumps(b, sort_keys=True))


def rest(e):
    return json.dumps({k: v for k, v in e.items() if k not in BASE}, sort_keys=True)


def empty(e, tol):
    k = e["kind"]
    if k == "line":
        d = far(pt(e["a"]), pt(e["b"]))
        return d if d <= tol else None
    if k in ("circle", "arc"):
        return e["r"] if e["r"] <= tol else None
    if k in ("polyline", "polygon") and "parts" not in e and "holes" not in e:
        first = pt(e["pts"][0])
        reach = 0.0
        for q in e["pts"]:
            reach = max(reach, far(pt(q), first))
        return reach if reach <= tol else None
    if k == "text":
        return 0.0 if not e["text"].strip() else None
    return None


def same_opt(a, b, tol):
    if a is None and b is None:
        return True
    if a is None or b is None:
        return False
    return abs(a - b) <= tol


def matched_bulge(b, i, n, turn):
    def at(k):
        return b[k % n] if k % n < len(b) else 0.0
    name, s = turn
    if name == "same":
        return at(i)
    if name == "from":
        return at(s + i)
    if name == "reversed":
        return -at(2 * n - 2 - i)
    return -at(2 * n - 2 - s % n - i)


def same_path(a, b, polygon, tol):
    if any(k in x for x in (a, b) for k in ("parts", "holes")):
        return None
    closed = polygon or a.get("closed", False)
    if a.get("closed", False) != b.get("closed", False):
        return None
    pa, pb = [pt(q) for q in a["pts"]], [pt(q) for q in b["pts"]]
    m = same_points(pa, pb, closed, tol)
    if m is None:
        return None
    d, turn = m
    extras = any(k in x for x in (a, b) for k in ("zs", "ms", "widths"))
    if extras:
        if turn != ("same", 0):
            return None
        for k in ("zs", "ms"):
            x, y = a.get(k), b.get(k)
            if (x is None) != (y is None):
                return None
            if x is not None and (len(x) != len(y) or not all(same_opt(s, t, tol) for s, t in zip(x, y))):
                return None
        if a.get("widths") != b.get("widths"):
            return None
    n = len(pa)
    ba, bb = a.get("bulges", []), b.get("bulges", [])
    edges = n if closed else n - 1
    for i in range(edges):
        x = ba[i] if i < len(ba) else 0.0
        if abs(x - matched_bulge(bb, i, n, turn)) > 1e-9:
            return None
    return d


def same(a, b, tol):
    k = a["kind"]
    if k == "point":
        if "parts" in a or "parts" in b:
            return 0.0 if rest(a) == rest(b) else None
        d = far(pt(a["p"]), pt(b["p"]))
        return d if d <= tol and same_opt(a.get("z"), b.get("z"), tol) else None
    if k == "line":
        xa, xb, ya, yb = pt(a["a"]), pt(a["b"]), pt(b["a"]), pt(b["b"])
        fwd = max(far(xa, ya), far(xb, yb))
        if fwd <= tol and all(same_opt(a.get(f), b.get(f), tol) for f in ("za", "zb", "ma", "mb")):
            return fwd
        back = max(far(xa, yb), far(xb, ya))
        if back <= tol and same_opt(a.get("za"), b.get("zb"), tol) and same_opt(a.get("zb"), b.get("za"), tol) \
                and same_opt(a.get("ma"), b.get("mb"), tol) and same_opt(a.get("mb"), b.get("ma"), tol):
            return back
        return None
    if k == "circle":
        d = max(far(pt(a["c"]), pt(b["c"])), abs(a["r"] - b["r"]))
        return d if d <= tol else None
    if k == "arc":
        def end(e, ang):
            return (e["c"]["x"] + e["r"] * math.cos(ang), e["c"]["y"] + e["r"] * math.sin(ang))
        d = max(max(far(pt(a["c"]), pt(b["c"])), abs(a["r"] - b["r"])),
                max(far(end(a, a["a0"]), end(b, b["a0"])), far(end(a, a["a1"]), end(b, b["a1"]))))
        return d if d <= tol else None
    if k in ("polyline", "polygon"):
        return same_path(a, b, k == "polygon", tol)
    return 0.0 if rest(a) == rest(b) else None


def anchors(e):
    k = e["kind"]
    if k == "point":
        return [pt(e["p"])]
    if k == "line":
        return [pt(e["a"]), pt(e["b"])]
    if k in ("circle", "arc"):
        return [pt(e["c"])]
    if k in ("polyline", "polygon"):
        if k == "polygon" or e.get("closed"):
            return [pt(q) for q in e["pts"]]
        return [pt(e["pts"][0]), pt(e["pts"][-1])]
    return []


def probe(e):
    k = e["kind"]
    if k == "point":
        return pt(e["p"])
    if k == "line":
        return pt(e["a"])
    if k in ("circle", "arc"):
        return pt(e["c"])
    if k in ("polyline", "polygon"):
        return pt(e["pts"][0])
    return None


def is_uid(text):
    return isinstance(text, str) and (text.startswith("$uidOf:") or text == GHOST)


def answer(inp):
    """(changed: [(id, entity)], removed: [id], report, warnings)."""
    uids = inp.get("uids")
    if uids is not None:
        if not uids:
            raise Refused("no_entities", "Temizlenecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids")
        for i, u in enumerate(uids):
            if not is_uid(u):
                raise Refused("invalid_uid", f"“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                              "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya "
                              "da çizimden alın.", f"uids[{i}]")
    tol = inp.get("tolerance", DEFAULT_TOL)
    if isinstance(tol, float) and not math.isfinite(tol):
        raise Refused("not_finite", "Tolerans sonlu bir sayı değil (NaN ya da sonsuz). Toleransı sonlu bir sayıyla verin.",
                      "tolerance")
    if not 0 <= tol <= 10:
        raise Refused("invalid_tolerance", "Tolerans 0 ile 10 m arasında olmalı. Başka bir tolerans verin.", "tolerance")
    rev = inp.get("expectedRevision")
    if rev is not None and not (rev.startswith("$") or rev.isdigit()):
        raise Refused("invalid_revision", f"Beklenen sürüm “{rev}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
    warnings = []
    if uids is not None:
        found, seen = [], set()
        for i, u in enumerate(uids):
            if u in seen:
                continue
            seen.add(u)
            if u == GHOST:
                raise Refused("entity_not_found", f"“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. "
                              "Var olan bir nesnenin kimliğini verin.", f"uids[{i}]")
            found.append((i, int(u.split(":")[1])))
        locked = [(i, s) for i, s in found if BY_ID[s]["layerId"] in LOCKED]
        scope = [s for i, s in found if BY_ID[s]["layerId"] not in LOCKED]
        if not scope:
            raise Refused("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden "
                          "açın.", f"uids[{locked[0][0]}]")
        if locked:
            warnings.append({"code": "layer_locked", "path": "uids", "message": f"{len(locked)} nesne kilitli katmanda olduğu "
                             "için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."})
    else:
        scope = [e["id"] for e in ENTITIES if e["layerId"] not in LOCKED | HIDDEN]
    items = [BY_ID[s] for s in scope]
    ignore = inp.get("ignore", [])
    keys = [key(e, ignore) for e in items]
    report = {"empty": 0, "duplicates": 0, "merged": 0, "merging": 0, "vertices": 0, "paths": 0, "loss": 0.0}
    gone = [False] * len(items)
    for i, e in enumerate(items):
        d = empty(e, tol)
        if d is not None:
            gone[i] = True
            report["empty"] += 1
            report["loss"] = max(report["loss"], d)
    cell = max(2.0 * tol, 1e-6)

    def at(q):
        return (math.floor(q[0] / cell), math.floor(q[1] / cell))

    grid, exact = {}, {}
    for i, e in enumerate(items):
        if gone[i]:
            continue
        k = keys[i]
        q = probe(e)
        if q is not None:
            x, y = at(q)
            near = sorted({j for cx in (x - 1, x, x + 1) for cy in (y - 1, y, y + 1) for j in grid.get((k, cx, cy), [])})
            found = next((d for d in (same(items[j], e, tol) for j in near) if d is not None), None)
        else:
            found = 0.0 if (k, rest(e)) in exact else None
        if found is not None:
            gone[i] = True
            report["duplicates"] += 1
            report["loss"] = max(report["loss"], found)
        elif q is not None:
            for a in anchors(e):
                lst = grid.setdefault((k, *at(a)), [])
                if i not in lst:
                    lst.append(i)
        else:
            exact[(k, rest(e))] = i
    updated = {}
    if inp.get("overlaps", True):
        lines = [(i, e) for i, e in enumerate(items) if not gone[i] and e["kind"] == "line"
                 and not any(f in e for f in ("za", "zb", "ma", "mb"))]
        group_of = {}
        groups = [group_of.setdefault(keys[i], len(group_of)) for i, _ in lines]
        ends = [(pt(e["a"]), pt(e["b"])) for _, e in lines]
        for m in merge_lines(ends, groups, tol, inp.get("endToEnd", False)):
            kept = lines[m["kept"]][0]
            for a in m["absorbed"]:
                gone[lines[a][0]] = True
                report["merged"] += 1
            report["merging"] += 1
            report["loss"] = max(report["loss"], m["loss"])
            out = {k: v for k, v in items[kept].items()}
            out["a"], out["b"] = p(*m["a"]), p(*m["b"])
            updated[kept] = out
    straight = inp.get("straightVertices", True)
    for i, e in enumerate(items):
        if gone[i] or e["kind"] not in ("polyline", "polygon") or "parts" in e or "holes" in e:
            continue
        polygon = e["kind"] == "polygon"
        closed = polygon or e.get("closed", False)
        n = len(e["pts"])
        pts = [pt(q) for q in e["pts"]]
        widths = e.get("widths")

        def keeps(k, e=e, widths=widths, n=n):
            if any(e.get(f) is not None and k < len(e[f]) and e[f][k] is not None for f in ("zs", "ms")):
                return True
            if widths is not None:
                before = widths[(k + n - 1) % n] if (k + n - 1) % n < len(widths) else [0, 0]
                after = widths[k] if k < len(widths) else [0, 0]
                return before[1] != after[0]
            return False

        t = thin(pts, e.get("bulges"), closed, tol, straight, keeps)
        if t is None:
            continue
        kept, loss = t
        report["vertices"] += n - len(kept)
        report["paths"] += 1
        report["loss"] = max(report["loss"], loss)
        out = {k: v for k, v in e.items() if k not in ("bulges", "zs", "ms", "widths")}
        out["pts"] = [e["pts"][k] for k in kept]
        if "bulges" in e:
            b = kept_bulges(e["bulges"], kept, n, closed)
            if any(x != 0.0 for x in b):
                out["bulges"] = b
        for f in ("zs", "ms"):
            if f in e:
                out[f] = [e[f][k] for k in kept]
        if widths is not None and not polygon:
            nw = []
            for j, k in enumerate(kept):
                last = kept[j + 1] - 1 if j + 1 < len(kept) else (n - 1 if closed else k)
                nw.append([widths[k][0], widths[last][1]])
            out["widths"] = nw
        updated[i] = out
    removed = [items[i]["id"] for i in range(len(items)) if gone[i]]
    changed = [(items[i]["id"], updated[i]) for i in sorted(updated)]
    return changed, removed, report, warnings


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def step(inp, op="execute"):
    try:
        changed, removed, report, warnings = answer(inp)
    except Refused as r:
        return {"op": op, "input": inp, "result": {"status": r.status, "error": {"code": r.code, "message": r.message,
                                                                                 "path": r.path}},
                "expect": {"ids": IDS, "revision": "same", "canUndo": False}}
    if op == "validate":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": None, "warnings": warnings},
                "expect": {"revision": "same"}}
    if op == "plan":
        return {"op": op, "input": inp, "result": {"status": "completed", "warnings": warnings, "output": {
            "changed": [e for _, e in changed], "removed": [U(s) for s in removed], "report": report,
            "revision": "$current"}}, "expect": {"revision": "same"}}
    out = {"op": op, "input": inp, "result": {"status": "completed", "warnings": warnings, "output": {
        "changed": [U(s) for s, _ in changed], "removed": [f"$uid:o{s}" for s in removed], "report": report,
        "revision": "$current"}}}
    out["captures"] = removed
    if changed or removed:
        out["expect"] = {"ids": [i for i in IDS if i not in removed], "entities": {str(s): e for s, e in changed},
                         "revision": "changed", "canUndo": True}
    else:
        out["expect"] = {"ids": IDS, "revision": "same", "canUndo": False}
    return out


def captured(s):
    """The ids of what the step removes, taken before it."""
    return [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in s.pop("captures", [])] + [s]


def case(name, inp, undo=False):
    steps = captured(step(inp))
    if undo:
        steps.append({"op": "undo", "returns": LABEL, "expect": {"ids": IDS}})
    return {"name": name, "steps": steps}


def on(*ids, **more):
    return {"uids": [U(i) if isinstance(i, int) else i for i in ids], **more}


def cases():
    out = [
        case("Yinelenen çizgi silinir, ilki kalır: tam aynısı ve ters yönlüsü; tek adım “Çizimi temizle”, geri alınır",
             on(1, 2), undo=True),
        case("Tolerans içinde yinelenen: 0,5 mm uzaktaki uç milimetre toleransla aynı, kayıp raporda",
             on(1, 3, tolerance=0.001, overlaps=False)),
        case("Varsayılan toleransla (0,001 mm) 0,5 mm uzaktaki çizgi yinelenen değil", on(1, 3, overlaps=False)),
        case("Üst üste binen eş doğrultulu çizgiler ilkine katılır: en uzak uçlara uzanır", on(1, 4)),
        case("Uç uca çizgiler yalnız endToEnd ile katılır", on(4, 5, endToEnd=True)),
        case("Uç uca çizgiler endToEnd olmadan ayrı kalır", on(4, 5)),
        case("İçteki kısa çizgi kapsayana katılır: kapsayanın uçları yerinde kalır", on(1, 28)),
        case("Bir köprü iki kümeyi birleştirir: üçü tek çizgi olur", on(1, 5, 4, endToEnd=True)),
        case("Boş çizgi silinir", on(6)),
        case("Toleranstan kısa çizgi boş sayılır", on(27, tolerance=0.001)),
        case("Rengi farklı yinelenen kalır; renk yok sayılınca silinir", on(1, 7, overlaps=False)),
        case("Renk yok sayılınca rengi farklı yinelenen de silinir", on(1, 7, ignore=["color"], overlaps=False)),
        case("Öznitelikleri farklı nesne yinelenen sayılmaz (veri kaybolmaz)", on(1, 23, ignore=["color"], overlaps=False)),
        case("Daireler: merkez ve yarıçap tolerans içinde", on(8, 9, tolerance=0.001)),
        case("Yaylar: merkez, yarıçap ve uçlar", on(10, 11)),
        case("Ters çizilmiş yinelenen çoklu çizgi silinir; kalanın tekrarlanan ve doğrudaki köşeleri atılır", on(12, 13)),
        case("Doğrudaki köşeler kalsın: yalnız tekrarlanan köşe atılır", on(12, straightVertices=False)),
        case("Başka köşeden başlayan ters yönlü kapalı alan yinelenendir", on(14, 15)),
        case("Kendi kotu olan köşe kalır", on(16)),
        case("Kendi kenarı yay olan tekrarlanan köşe atılır: yay bir önceki köşeden başlar", on(17)),
        case("Genişliğin değiştiği köşe kalır; sürekli genişlikte atılır, kenarın genişliği uçlarından", on(18, 19)),
        case("Kapalı çoklu çizginin doğrudaki köşesi atılır", on(24)),
        case("Sözsüz yazı boş nesnedir", on(20)),
        case("Noktalar tolerans içinde yinelenir", on(25, 26, tolerance=0.001)),
        case("Kilitli katmandaki nesne uyarıyla atlanır", on(21, 1, 2)),
        case("Bütün çizim: görünen ve kilitsiz katmanlar, milimetre toleransla", {"tolerance": 0.001}),
        case("Temizlenecek bir şey yoksa hiçbir şey yazılmaz", on(8, 10)),
        case("Hedef listesi boş: no_entities", {"uids": []}),
        case("Kimlik UUID değil: invalid_uid", on(1, "on iki")),
        case("Nesne çizimde yok: entity_not_found", on(1, GHOST)),
        case("Hepsi kilitli: layer_locked", on(21)),
        case("Eksi tolerans: invalid_tolerance", on(1, tolerance=-1)),
        case("10 m'den büyük tolerans: invalid_tolerance", on(1, tolerance=11)),
        case("Beklenen sürüm sayı değil: invalid_revision", on(1, expectedRevision="on iki")),
    ]
    nan = step(on(1, tolerance=1))
    nan["nonFinite"] = {"tolerance": "NaN"}
    nan["result"] = {"status": "failed", "error": {"code": "not_finite", "path": "tolerance", "message":
                     "Tolerans sonlu bir sayı değil (NaN ya da sonsuz). Toleransı sonlu bir sayıyla verin."}}
    nan["expect"] = {"ids": IDS, "revision": "same", "canUndo": False}
    nan.pop("captures", None)
    out.append({"name": "Tolerans NaN: not_finite", "steps": [nan]})
    out.append({"name": "Plan değişecekleri ve raporu gösterir; doğrulama yazmanın cevabını verir; hiçbir şey yazılmaz",
                "steps": [step(on(1, 2, 4, 12), op="plan"), step(on(21, 1), op="validate")]})
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "command": "cad.entities.transform", "input": {"uids": [U(8)], "transform": {"kind": "move", "dx": 1, "dy": 0}},
         "result": "$any"},
        {"op": "execute", "input": on(1, 2, expectedRevision="$old"),
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."}},
         "expect": {"revision": "same"}}]})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.cleanup", "commandVersion": 1,
            "title": "Çizimi temizle: boş ve yinelenen nesneler, eş doğrultulu çizgiler, gereksiz köşeler, tolerans ve rapor",
            "note": "ADR 0305. Bağımsız başvuru: scripts/fixtures/cleanup_command_cases.py.",
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
