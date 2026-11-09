#!/usr/bin/env python3
"""Each map's own survey quality on a sheet (plan/map-cad.md MAP-18, docs/adr/2800), without KentOS code.

    python3 scripts/fixtures/sheet_survey_scope_cases.py           # writes fixtures/sheet/v1/survey-scope.json
    python3 scripts/fixtures/sheet_survey_scope_cases.py --check   # writes nothing; compares

The drawing is fixtures/kcad/v2/map-survey-network.json (kn-1 accepted as a mean, no σ; kn-2 by the network adjustment
da-1, σ 4.0 / 3.5 / 9.0 mm, C3) with kn-2's point object drawn, two more accepted points (kn-3 by a second adjustment
da-2, C2; kn-4 with a large σ) and their objects, and a sheet of three maps and their texts. The rules, by hand:

- a point is where its object is drawn (the archive keeps the object's persistent id), in the project's system;
- a map shows the points inside its content's four corners on the ground: the frame (no padding, no label band) at the
  map's scale around its centre, turned by its view's turn; the edges count;
- the words are docs/adr/2400's: the greatest √(σN² + σE²) and σ up of the points shown, “±0,5 cm”; the adjustments
  those points name, in the archive's order; the note's sentence, the points without σ counted. A map that shows none:
  the note “Haritanın içinde GNSS ile kabul edilmiş nokta yok.”, the values empty;
- a text tied to a map reads that map's words, an untied text the sheet's first map's;
- a value that is none prints as its name in angle quotes, `‹konum_dogrulugu?›` (the sheet's design §7); a text that
  reads one, and has no variable of the name, is a `survey_quality_missing` finding.

Map B is turned 45°: its ground is a diamond. kn-4 lies inside the diamond's box but outside the diamond: counted, its
σ (20 / 20 / 30 mm) would make B's words ±2,8 cm and ±3,0 cm.

docs/adr/3180's maps (docs/adr/3200): a map's own clip shape bounds what it shows. Its corners are in the content
frame's millionths (left to right, top to bottom); each is carried to the ground as the frame's corners are, and a point
is shown when it is inside the shape's ring or on an edge (even-odd, the side decided in exact fractions). Map D's frame
holds kn-3 and kn-4, its L-shaped clip leaves the top right quarter, where kn-4 is, out: D says kn-3's words. Map E's
rectangle keeps the frame's lower half: of kn-1 and kn-2 only kn-1, whose accuracy is not recorded.
"""

import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "sheet" / "v1" / "survey-scope.json"
BASE = ROOT / "fixtures" / "kcad" / "v2" / "map-survey-network.json"
NONE_SHOWN = "Haritanın içinde GNSS ile kabul edilmiş nokta yok."
UID = "0192f5c0-2800-7000-8000-{:012x}"

# The accepted points' objects (east, north in the project's system).
PLACES = {"kn-1": (544604.5188026087, 4404866.510293009), "kn-2": (544620.0, 4404880.0),
          "kn-3": (544800.0, 4404900.0), "kn-4": (544822.0, 4404922.0)}
# The maps: frame (left, top, width, height) in µm, centre, scale, view turn in millidegrees.
MAPS = [("a", (20000, 20000, 100000, 80000), (544612.0, 4404873.0), 500, 0),
        ("b", (140000, 20000, 80000, 80000), (544800.0, 4404900.0), 500, 45000),
        ("c", (240000, 20000, 60000, 40000), (545000.0, 4405000.0), 500, 0),
        ("d", (240000, 64000, 80000, 50000), (544810.0, 4404910.0), 500, 0),
        ("e", (330000, 20000, 60000, 60000), (544612.0, 4404873.0), 500, 0)]
# docs/adr/3200: the clip shapes, in the content frame's millionths (left to right, top to bottom).
CLIPS = {"d": {"type": "polygon", "points": [[0, 0], [500000, 0], [500000, 500000], [1000000, 500000], [1000000, 1000000],
                                             [0, 1000000]]},
         "e": {"type": "rectangle", "bounds": [0, 500000, 1000000, 1000000]}}
# The texts: id, the map tied to (none: the first map's), content.
TEXTS = [("a_konum", "a", "A: [% @konum_dogrulugu %]"),
         ("a_not", "a", "[% @olcme_kalitesi %]"),
         ("b_konum", "b", "B: [% @konum_dogrulugu %] [% @yukseklik_dogrulugu %]"),
         ("b_ag", "b", "[% @ag_dengelemesi %]"),
         ("b_not", "b", "[% @olcme_kalitesi %]"),
         ("c_not", "c", "C: [% @olcme_kalitesi %]"),
         ("c_konum", "c", "[% @konum_dogrulugu %]"),
         ("serbest", None, "Serbest: [% @konum_dogrulugu %]"),
         ("d_konum", "d", "D: [% @konum_dogrulugu %] [% @yukseklik_dogrulugu %]"),
         ("e_not", "e", "E: [% @olcme_kalitesi %]")]


def drawing():
    d = json.loads(BASE.read_text("utf-8"))
    survey = d["settings"]["mapSurvey"]
    kn2 = next(p for p in survey["points"] if p["id"] == "kn-2")
    da1 = survey["adjustments"][0]
    da2 = {**copy.deepcopy(da1), "id": "da-2", "class": "C2", "m0": 1.05, "dof": 5, "freeM0": 1.05, "freeDof": 5}
    survey["adjustments"].append(da2)
    for n, (pid, mark, sigma, adj) in enumerate([("kn-3", "N4", [0.006, 0.008, 0.012], "da-2"),
                                                 ("kn-4", "N5", [0.02, 0.02, 0.03], "da-2")]):
        p = copy.deepcopy(kn2)
        p.update({"id": pid, "mark": mark, "uid": UID.format(3 + n)})
        p["result"].update({"sigma": sigma, "adjustment": adj})
        survey["points"].append(p)
    # Each point's object: kn-1's is the drawing's point 1; the others are added.
    for pid in ("kn-2", "kn-3", "kn-4"):
        p = next(q for q in survey["points"] if q["id"] == pid)
        e = {"kind": "point", "id": len(d["entities"]) + 1, "layerId": "0",
             "attrs": {"Ad": p["mark"], "Kabul": pid}, "p": {"x": PLACES[pid][0], "y": PLACES[pid][1]}}
        d["entities"].append(e)
        d["uids"].append(p["uid"])
    return d


def sheet():
    items = []
    for mid, (left, top, w, h), (cx, cy), scale, turn in MAPS:
        # A 0.35 mm frame line: the owner sees what each map covers (it does not change the content's corners).
        kind = {"type": "map", "view": {"type": "fixed", "center": {"x": cx, "y": cy}, "scale": scale, "rotation": turn}}
        if mid in CLIPS:
            kind["clip"] = CLIPS[mid]
        items.append({"id": mid, "name": mid.upper(), "frame": {"left": left, "top": top, "width": w, "height": h},
                      "border": {"width": 350}, "kind": kind})
    for k, (tid, mid, content) in enumerate(TEXTS):
        kind = {"type": "text", "content": content}
        if mid:
            kind["map"] = mid
        items.append({"id": tid, "name": tid, "frame": {"left": 20000, "top": 120000 + 16000 * k, "width": 380000,
                                                        "height": 12000}, "kind": kind})
    return {"id": "pafta", "name": "Pafta",
            "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                     "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
            "items": items}


# ── The rules, by hand ────────────────────────────────────────────────────────────────────────────────────────────
def corners(frame, centre, scale, turn):
    """The content's corners on the ground: half the frame (µm → m at the scale) around the centre, turned."""
    hw, hh = frame[2] / 1e6 * scale / 2, frame[3] / 1e6 * scale / 2
    t = math.radians(turn / 1000.0)
    c, s = math.cos(t), math.sin(t)
    return [(centre[0] + x * c - y * s, centre[1] + x * s + y * c) for x, y in ((-hw, hh), (hw, hh), (hw, -hh), (-hw, -hh))]


def inside(quad, q):
    side = 0
    for i in range(4):
        (ax, ay), (bx, by) = quad[i], quad[(i + 1) % 4]
        cross = (bx - ax) * (q[1] - ay) - (by - ay) * (q[0] - ax)
        if abs(cross) > 1e-9:
            if side and (cross > 0) != (side > 0):
                return False
            side = 1 if cross > 0 else -1
    return True


def clip_ring(mid, frame, centre, scale, turn):
    """docs/adr/3200: the ground the map shows, its clip shape's corners carried as the frame's are: the millionths
    (u, v) are the frame's point (u − ½, ½ − v) of its width and height from the centre, turned."""
    clip = CLIPS.get(mid)
    if clip is None:
        return corners(frame, centre, scale, turn)
    if clip["type"] == "rectangle":
        l, t, r, b = clip["bounds"]
        pts = [(l, t), (r, t), (r, b), (l, b)]
    else:
        pts = [tuple(q) for q in clip["points"]]
    w, h = frame[2] / 1e6 * scale, frame[3] / 1e6 * scale
    a = math.radians(turn / 1000.0)
    c, s = math.cos(a), math.sin(a)
    out = []
    for u, v in pts:
        x, y = (u / 1e6 - 0.5) * w, (0.5 - v / 1e6) * h
        out.append((centre[0] + x * c - y * s, centre[1] + x * s + y * c))
    return out


def inside_ring(ring, q):
    """Inside a simple ring or on an edge, in exact fractions: on an edge, else the crossings to the right counted."""
    qx, qy = F(q[0]), F(q[1])
    odd = False
    n = len(ring)
    for i in range(n):
        (ax, ay), (bx, by) = [(F(x), F(y)) for x, y in (ring[i], ring[(i + 1) % n])]
        turn = (bx - ax) * (qy - ay) - (by - ay) * (qx - ax)
        if turn == 0 and min(ax, bx) <= qx <= max(ax, bx) and min(ay, by) <= qy <= max(ay, by):
            return True
        if (ay > qy) != (by > qy):
            left = turn > 0 if by > ay else turn < 0
            if left:
                odd = not odd
    return odd


def cm(metres):
    return f"±{metres * 100:.1f} cm".replace(".", ",")


def facts(survey, shown):
    """docs/adr/2400's words of the points `shown` (ids)."""
    points = [p for p in survey["points"] if p["id"] in shown]
    if not points:
        return {"position": "", "height": "", "network": "", "note": NONE_SHOWN}
    sigmas = [p["result"]["sigma"] for p in points if p["result"].get("sigma")]
    pos = max((math.hypot(s[0], s[1]) for s in sigmas), default=None)
    hgt = max((s[2] for s in sigmas), default=None)
    used = [a for a in survey["adjustments"] if any(p["result"].get("adjustment") == a["id"] for p in points)]
    rules = lambda a: None if a.get("rulesPassed") is None else ("kurallar sağlandı" if a["rulesPassed"] else "kurallar sağlanmadı")  # noqa: E731

    def long_(a):
        words = [a["id"]]
        if a.get("class"):
            words.append(f"{a['class']} sınıfı")
        if rules(a):
            words.append(f"{rules(a)} ({a['rules']})" if a.get("rules") else rules(a))
        if a.get("m0") is not None:
            words.append(f"m0 {a['m0']:.2f}".replace(".", ","))
        words.append(f"serbestlik {a['dof']}")
        return ", ".join(words)

    def short(a):
        more = ([a["class"]] if a.get("class") else []) + ([rules(a)] if rules(a) else [])
        return f"{a['id']} ({', '.join(more)})" if more else a["id"]

    note = f"GNSS ile {len(points)} nokta"
    note += (f": en büyük nokta konum hatası {cm(pos)}, yükseklik hatası {cm(hgt)} (ortalama hata)" if sigmas
             else ": doğrulukları kayıtlı değil")
    if used:
        note += "; ağ dengelemesi " + ", ".join(short(a) for a in used)
    note += "."
    if sigmas and len(points) > len(sigmas):
        note += f" {len(points) - len(sigmas)} noktanın doğruluğu kayıtlı değil."
    return {"position": cm(pos) if sigmas else "", "height": cm(hgt) if sigmas else "",
            "network": "; ".join(long_(a) for a in used), "note": note}


def main():
    d = drawing()
    survey = d["settings"]["mapSurvey"]
    shown, words = {}, {}
    for mid, frame, centre, scale, turn in MAPS:
        if mid in CLIPS:
            ring = clip_ring(mid, frame, centre, scale, turn)
            shown[mid] = [pid for pid, q in PLACES.items() if inside_ring(ring, q)]
        else:
            quad = corners(frame, centre, scale, turn)
            shown[mid] = [pid for pid, q in PLACES.items() if inside(quad, q)]
        words[mid] = facts(survey, shown[mid])
    assert shown == {"a": ["kn-1", "kn-2"], "b": ["kn-3"], "c": [], "d": ["kn-3"], "e": ["kn-1"]}, shown
    # docs/adr/3200: D's frame holds kn-4 too, E's kn-2: their clips leave them out.
    for mid, pid in (("d", "kn-4"), ("e", "kn-2")):
        m = next(x for x in MAPS if x[0] == mid)
        assert inside(corners(*m[1:]), PLACES[pid]), (mid, pid)
    # kn-4 is in B's box, out of its diamond.
    quad = corners(*MAPS[1][1:])
    xs, ys = [p[0] for p in quad], [p[1] for p in quad]
    assert min(xs) <= PLACES["kn-4"][0] <= max(xs) and min(ys) <= PLACES["kn-4"][1] <= max(ys)
    fills = {"konum_dogrulugu": "position", "yukseklik_dogrulugu": "height", "ag_dengelemesi": "network",
             "olcme_kalitesi": "note"}
    texts, findings = {}, []
    for tid, mid, content in TEXTS:
        w = words[mid or MAPS[0][0]]
        out, empty = content, []
        for name, key in fills.items():
            if f"@{name}" in content:
                # A value that is none prints as its name in angle quotes (the sheet's design §7).
                out = out.replace(f"[% @{name} %]", w[key] or f"‹{name}?›")
                if not w[key]:
                    empty.append(name)
        texts[tid] = " ".join(out.split())
        if empty:
            findings.append({"item": tid, "code": "survey_quality_missing"})
    doc = {"format": "kentos.sheet-survey-scope", "version": 1,
           "source": "scripts/fixtures/sheet_survey_scope_cases.py (docs/adr/2800), KentOS kodu yok",
           "drawing": d, "sheet": sheet(), "shown": shown, "maps": words, "texts": texts, "findings": findings}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("Haritanın ölçme kalitesi durumları güncel." if ok else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(texts)} yazı, {len(findings)} bulgu)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
