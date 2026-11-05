"""Writes fixtures/commands/v1/road.alignments.impact.json: the shared cases of `road.alignments.impact` (plan/road.md
Y57, docs/adr/0963; ROAD-ALIGN-10), their expectations built here without KentOS code.

The lines are scripts/fixtures/road_alignment_cases.py's (mpmath, the clothoid by its series). An old station's place
has its foot on the new line where the distance is least: found on float places every 0.5 m, refined by golden-section
search in mpmath; at a line's end only a place on it has a foot. Each element is sampled at 32 points (KentOS: 64),
where the gap crosses the tolerance is bisected, the greatest gap of a stretch refined by golden-section search. The
element mapping is road_alignment_command_cases.py's.

The drawing is road_profile_impact_command_cases.py's: profiles, section sets, corridors and the superelevation
schedules on the main alignment, the main volume and haul plan approved.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with scipy \\
        --with shapely python3 scripts/fixtures/road_alignment_impact_command_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_alignment_command_cases as acmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_profile_impact_command_cases as pimp  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402

mp.mp.dps = 25
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.alignments.impact.json"
completed, failed, step, case = pimp.completed, pimp.failed, pimp.step, pimp.case
UNCHANGED = pimp.UNCHANGED
hc = pimp.hc
W = 1e-6
TOLERANCE = 1e-3
SHIFT = 1e-6
N = 32


class Refused(Exception):
    def __init__(self, result):
        super().__init__(result["error"]["message"])
        self.result = result


class Line:
    def __init__(self, d, alignments):
        self.b = acmd.build(alignments, d)
        self.els, self.length = self.b.els, self.b.length
        self.pts = aref.fast_points(self.els, 0.5)

    def at(self, s):
        return aref.place_at(self.els, mp.mpf(s))[1]

    def km_text(self, s):
        return acmd.km_text(aref.km_of(self.b.regions, mp.mpf(s)))

    def foot(self, p, ends=False):
        """The station of the least distance from `p` and the distance; none past an end."""
        px, py = float(p[0]), float(p[1])
        d = [(x - px) ** 2 + (y - py) ** 2 for _, x, y in self.pts]
        n = len(d)
        # Every local least of the grid refined, the least kept: near the switch between two tangents two minima are
        # nearly equal and the grid may rank them wrongly.
        lows = [i for i in range(n) if (i == 0 or d[i] <= d[i - 1]) and (i == n - 1 or d[i] <= d[i + 1])]
        best = None
        for i in lows:
            if d[i] > min(d) + 4.0:
                continue
            lo, hi = self.pts[max(i - 1, 0)][0], self.pts[min(i + 1, n - 1)][0]
            t = min(max(aref.golden(lambda t: aref.dist2(self.els, p, t), lo, hi), mp.mpf(0)), self.length)
            q = self.at(t)
            dd = mp.hypot(p[0] - q[0], p[1] - q[1])
            if best is None or dd < best[1]:
                best = (t, dd)
        s, dist = best
        if not ends and (s <= mp.mpf("1e-7") or s >= self.length - mp.mpf("1e-7")) and dist > mp.mpf("1e-7"):
            return None
        return s, dist


def gap(line, other, s):
    f = other.foot(line.at(s))
    return mp.inf if f is None else f[1]


def off(line, other, tol):
    out, start = [], None
    cuts = [e.station for e in line.els] + [line.length]
    over = lambda s: gap(line, other, s) > tol  # noqa: E731
    for a, b in zip(cuts, cuts[1:]):
        if b - a < mp.mpf("1e-12"):
            continue
        xs = [a + (b - a) * k / N for k in range(N + 1)]
        flags = [over(x) for x in xs]
        if start is None and flags[0]:
            start = xs[0]
        for (x0, f0), (x1, f1) in zip(zip(xs, flags), zip(xs[1:], flags[1:])):
            if f0 != f1:
                lo, hi = x0, x1
                for _ in range(60):
                    m = (lo + hi) / 2
                    if over(m) == f0:
                        lo = m
                    else:
                        hi = m
                x = (lo + hi) / 2
                if start is None:
                    start = x
                else:
                    out.append([start, x])
                    start = None
    if start is not None:
        out.append([start, line.length])
    res = []
    for a, b in out:
        if b - a <= mp.mpf("1e-9"):
            continue
        res.append({"from": float(a), "to": float(b), **most(line, other, a, b)})
    return res


def most(line, other, a, b):
    xs = [a + (b - a) * k / 64 for k in range(65)]
    vals = [gap(line, other, x) for x in xs]
    if any(v == mp.inf for v in vals):
        return {}
    k = max(range(65), key=lambda i: vals[i])
    lo, hi = xs[max(k - 1, 0)], xs[min(k + 1, 64)]
    best = aref.golden(lambda t: -gap(line, other, t), lo, hi)
    return {"most": float(max(gap(line, other, best), vals[k]))}


def item_words(owner, item):
    head, _, rest = item.partition(" ")
    if head == "region":
        k, end = rest.split(" ")
        return f"{k}. bölge {'başı' if end == 'from' else 'sonu'}"
    if head == "station":
        return f"{rest}. km"
    if head == "row":
        return f"{rest}. satır"
    if head == "curve":
        return f"{rest} kurbu"
    if owner == "profile":
        return f"PVI {item}"
    return {"from": "başı", "to": "sonu"}.get(item, item)


def answer(inp, volumes=pimp.VOLUMES, hauls=pimp.HAULS):
    alignments = pcmd.ALIGNMENTS
    name = inp["alignment"]
    d = next((a for a in alignments if a["id"] == name), None) or next(
        (a for a in alignments if a["name"].lower() == name.lower()), None)
    if d is None:
        raise Refused(failed("unknown_alignment", f"“{name}” kimlikli ya da adlı aks projede yok.", "alignment"))
    if "offsetOf" in d:
        raise Refused(failed("invalid_alignment", "Paralel aksın PI'ı olmaz: kaynağını izler; kaynağı düzenleyin.",
                             "alignment"))
    tol = inp.get("tolerance", TOLERANCE)
    if not 0 < tol <= 1:
        raise Refused(failed("invalid_alignment_impact", "Etki toleransı 0'dan büyük, en çok 1 m olur.", "tolerance"))
    if len(inp["pis"]) < 2:
        raise Refused(acmd.rule("aksın 2 ile 10000 arası noktası olur", "pis"))
    new_d = {**d, "pis": inp["pis"]}
    before = Line(d, alignments)
    after = Line(new_d, [new_d if a["id"] == d["id"] else a for a in alignments])
    removed = off(before, after, tol)
    added = off(after, before, tol)
    for r in removed:
        r["fromText"], r["toText"] = before.km_text(r["from"]), before.km_text(r["to"])
    for r in added:
        r["fromText"], r["toText"] = after.km_text(r["from"]), after.km_text(r["to"])

    def mapped(s):
        f = after.foot(before.at(s))
        return float(f[0]) if f is not None and f[1] <= tol else None

    def shift(s):
        # The nearest station on the new line, its end included at any distance.
        f = after.foot(before.at(s), ends=True)
        return float(f[0] - mp.mpf(s))

    runs, at = [], 0.0
    for a, b in [(r["from"], r["to"]) for r in removed] + [(float(before.length), float(before.length))]:
        if a - at > 1e-9:
            runs.append({"from": at, "to": a, "fromText": before.km_text(at), "toText": before.km_text(a),
                         "shiftFrom": shift(at), "shiftTo": shift(a)})
        at = b
    refs = []

    def name_(owner, id_, nm, item, s):
        to = mapped(s)
        if to is None or abs(to - s) > SHIFT:
            r = {"owner": owner, "id": id_, "name": nm, "item": item, "station": float(s)}
            if to is not None:
                r["to"] = to
            refs.append(r)

    for p in hc.ccmd.PROFILES:
        if p["alignment"] == d["id"]:
            for v in p["pvis"]:
                name_("profile", p["id"], p["name"], v["id"], v["station"])
    for s_ in [scmd.S1, scmd.S2]:
        if s_["alignment"] != d["id"]:
            continue
        st = s_["stations"]
        if st["method"] in ("range", "breaks"):
            name_("sectionSet", s_["id"], s_["name"], "from", st["from"])
            name_("sectionSet", s_["id"], s_["name"], "to", st["to"])
        elif st["method"] == "stations":
            for k, x in enumerate(st["stations"]):
                name_("sectionSet", s_["id"], s_["name"], f"station {k + 1}", x)
    touched = []
    for c in hc.CORRIDORS:
        if c["alignment"] != d["id"]:
            continue
        n = len(refs)
        for k, r in enumerate(c["regions"]):
            name_("corridor", c["id"], c["name"], f"region {k + 1} from", r["from"])
            name_("corridor", c["id"], c["name"], f"region {k + 1} to", r["to"])
        for k, x in enumerate(c.get("stations", [])):
            name_("corridor", c["id"], c["name"], f"station {k + 1}", x)
        over = any(st["from"] < r["to"] and st["to"] > r["from"] for st in removed for r in c["regions"])
        if over or len(refs) > n:
            touched.append(c["id"])
    new_pis = {p["id"] for p in inp["pis"]}
    for sd in hc.ccmd.SUPERS:
        if sd["alignment"] != d["id"]:
            continue
        m = sd["method"]
        if m["method"] == "stations":
            for k, r in enumerate(m["rows"]):
                name_("superelevation", sd["id"], sd["name"], f"row {k + 1}", r["station"])
        else:
            curved = {p["id"] for p in inp["pis"] if p.get("radius") or p.get("spiralIn") or p.get("spiralOut")}
            for cv in m["curves"]:
                rep = next((x for x in before.b.reports if x["pi"] == cv["pi"]), None)
                if rep is not None and cv["pi"] not in curved:
                    refs.append({"owner": "superelevation", "id": sd["id"], "name": sd["name"],
                                 "item": f"curve {cv['pi']}", "station": float(rep["stations"][0])})
    parallels = [{"id": a["id"], "name": a["name"]} for a in alignments
                 if a.get("offsetOf", {}).get("alignment") == d["id"]]
    vols = [{"id": v["id"], "name": v["name"], "approved": "approved" in v} for v in volumes if v["corridor"] in touched]
    hs = [{"id": h["id"], "name": h["name"], "approved": "approved" in h} for h in hauls
          if any(v["id"] == h["volume"] for v in vols)]
    warnings = []
    lost = [f"{r['name']} {item_words(r['owner'], r['item'])}" for r in refs if "to" not in r]
    if lost:
        more = f" ve {len(lost) - 8} yer daha" if len(lost) > 8 else ""
        warnings.append({"code": "alignment_impact_unresolved",
                         "message": f"“{d['name']}” aksının yeni PI'larıyla şu km'lerin yeri aksın üstünde kalmıyor: "
                                    f"{', '.join(lost[:8])}{more}. Kaydedilirse bunlar elle yeniden verilmeli.",
                         "path": "pis"})
    told = [f"“{v['name']}” kübajı" for v in vols if v["approved"]] + \
           [f"“{h['name']}” taşıma planı" for h in hs if h["approved"]]
    if told:
        warnings.append({"code": "alignment_impact_approved",
                         "message": f"“{d['name']}” aksının yeni PI'ları onaylı hesapları değiştirir: {', '.join(told)}. "
                                    "Yazılınca onayları eskir; yeni bir hesap onaylanmalı.",
                         "path": "pis"})
    out = {"alignment": d["id"], "mapping": acmd.mapping(before.b, after.b), "removed": removed, "added": added,
           "runs": runs, "refs": refs, "parallels": parallels, "volumes": vols, "hauls": hs,
           "clear": not removed and not added and not refs}
    return out, warnings


def cases():
    base = hc.setup(pimp.HAULS, pimp.VOLUMES)
    road = hc.road_of(pimp.HAULS, pimp.VOLUMES)
    out = []

    def ok(name, inp, op="execute", volumes=pimp.VOLUMES, hauls=pimp.HAULS, setup_=None):
        o, w = answer(inp, volumes, hauls)
        result = completed(None, w) if op == "validate" else completed(o, w)
        s = step(op, inp, result, {**UNCHANGED, "road": road if setup_ is None else hc.road_of(hauls, volumes)})
        s["within"] = W
        c = case(name, [s])
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)
        print(f"  {name[:40]:40} rem {[(round(r['from'], 2), round(r['to'], 2)) for r in o['removed']]} "
              f"add {[(round(r['from'], 2), round(r['to'], 2)) for r in o['added']]} refs {len(o['refs'])}", flush=True)

    def no(name, inp, result=None, setup_=None):
        if result is None:
            try:
                answer(inp)
            except Refused as r:
                result = r.result
            else:
                raise AssertionError(name)
        c = case(name, [step("execute", inp, result, {**UNCHANGED, "road": None if setup_ else road})])
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

    main = copy.deepcopy(pcmd.MAIN_PIS)

    def with_(i, **kw):
        p = copy.deepcopy(main)
        p[i] = {**p[i], **kw}
        return p

    ok("Aynı PI'lar: değişiklik yok", {"alignment": "g-1", "pis": main})
    ok("Yarıçap 150'den 180'e", {"alignment": "g-1", "pis": with_(1, radius=180.0)})
    ok("Adıyla, klotoidler 40'tan 60'a", {"alignment": "Ana aks", "pis": with_(1, spiralIn=60.0, spiralOut=60.0)})
    # The last tangent runs from p1 (300, 60) to b (560, 360): a tenth further along it.
    ok("Bitiş doğrultusunda uzar", {"alignment": "g-1", "pis": with_(2, x=586.0, y=390.0)})
    # The first tangent runs from a (20, 50) to p1 (300, 60): 20 m of x forward along it.
    ok("Başlangıç ileri alınır", {"alignment": "g-1", "pis": with_(0, x=40.0, y=50.0 + 10.0 * 20.0 / 280.0)})
    ok("Bitiş yana kayar", {"alignment": "g-1", "pis": with_(2, x=600.0, y=340.0)})
    ok("Başlangıç geri çekilir", {"alignment": "g-1", "pis": with_(0, x=0.0, y=49.2857142857143)})
    ok("Kurp kaldırılır: kırık", {"alignment": "g-1", "pis": with_(1, radius=0.0, spiralIn=0.0, spiralOut=0.0)})
    ok("Kurbun PI'ı kaldırılır", {"alignment": "g-1", "pis": [main[0], main[2]]})
    ok("Yarıçap 1 cm toleransla", {"alignment": "g-1", "pis": with_(1, radius=150.001), "tolerance": 0.01})
    ok("Yarıçap 1 mm toleransla", {"alignment": "g-1", "pis": with_(1, radius=150.5)})
    ok("Doğrulama", {"alignment": "g-1", "pis": with_(1, radius=180.0)}, op="validate")
    ok("Planı", {"alignment": "g-1", "pis": with_(1, radius=180.0)}, op="plan")
    unapproved = [pimp.vcmd.V1, pimp.vcmd.V2, pimp.hap.pc.V3]
    ok("Onaysız kübaj ve plan", {"alignment": "g-1", "pis": with_(1, radius=180.0)},
       volumes=unapproved, hauls=pimp.hap.START, setup_=hc.setup(pimp.hap.START, unapproved))
    no("Olmayan aks", {"alignment": "yok", "pis": main})
    no("Paralel aks", {"alignment": "g-3", "pis": main})
    no("Sıfır tolerans", {"alignment": "g-1", "pis": main, "tolerance": 0.0})
    no("Çok büyük tolerans", {"alignment": "g-1", "pis": main, "tolerance": 2.0})
    no("Tek PI", {"alignment": "g-1", "pis": main[:1]})
    no("Yol projesi değil", {"alignment": "g-1", "pis": main}, pcmd.NOT_ROAD, setup_=hc.setup(work="map"))
    return base, out


def main_():
    base, cs = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.alignments.impact", "commandVersion": 1,
           "title": "Aks değişikliğinin etkisi",
           "note": "scripts/fixtures/road_alignment_impact_command_cases.py yazar; akslar road_alignment_cases.py'nin "
                   "(mpmath), ayak 0,5 m'lik yerlerde en yakından altın oranla, eleman başına 32 örnek ve ikiye bölmeyle; "
                   "KentOS kodu yok. Çizim road_profile_impact_command_cases.py'nindir.",
           "setup": base, "cases": cs}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(cs)} durum güncel")
        return
    OUT.write_text(text)
    print(f"yazıldı: {OUT} ({len(cs)} durum)")


if __name__ == "__main__":
    main_()
