#!/usr/bin/env python3
"""`road.profiles.draw` with KGM's earthwork rows under the frame (plan/road.md Y89, docs/adr/0995), without KentOS
code.

The volume table is road_volume_command_cases.py's (`compute`), the balance points road_haul_command_cases.py's
(`read`, its own mass diagram), the drawing's frame, columns and object count road_profile_command_cases.py's
`drawing`. Each balance section, from the volume's first station to its last cut at the balance points: HAFİR the raw
cut, HAFİRLİ FAK. the cut by its factor, İMLA the raw fill, the net (factored cut less factored fill) DEPO where
positive and ARİYET where negative; a volume row cut by a balance point shared by length. DN at the balance points, at
the start when the balance line is 0, at the end when the ordinate with its sites comes back to the line.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with shapely \\
        python3 scripts/fixtures/road_profile_command_cases.py   # writes road.profiles.draw.json with these cases
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import road_grades  # noqa: E402
import road_haul_command_cases as hcmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

ROWS = ["HAFİR", "HAFİRLİ FAK.", "İMLA", "ARİYET", "DEPO"]


def volume_text(v):
    return "-" if abs(v) < 0.5 else f"{v:.0f}"


def earthwork(h):
    v = next(x for x in hcmd.VOLUMES if x["id"] == h["volume"])
    rep, _ = vcmd.compute(v, hcmd.CORRIDORS)
    report, _ = hcmd.read(h)
    rows = rep["rows"]
    first, last = rows[0]["from"], rows[-1]["to"]
    inner = [s for s in report["balancePoints"] if first + 1e-9 < s < last - 1e-9]
    cuts = [first] + inner + [last]
    sections = []
    for a, b in zip(cuts, cuts[1:]):
        cut = fcut = fill = ffill = 0.0
        for r in rows:
            lo, hi = max(r["from"], a), min(r["to"], b)
            if hi <= lo or r["to"] <= r["from"]:
                continue
            k = (hi - lo) / (r["to"] - r["from"])
            c, f = r.get("cut", 0.0), r.get("fill", 0.0)
            cut += k * c
            fcut += k * c * r.get("cutFactor", 1.0)
            fill += k * f
            ffill += k * f * r.get("fillFactor", 1.0)
        net = fcut - ffill
        sections.append({"from": a, "to": b, "values": [cut, fcut, fill, max(-net, 0.0), max(net, 0.0)]})
    dn = list(inner)
    if abs(h.get("balance", 0.0)) <= 1e-9:
        dn.insert(0, first)
    sites = sum(s["volume"] if s["kind"] == "borrow" else -s["volume"] for s in h.get("sites", []))
    end = rows[-1]["mass"] + sites
    if abs(end - h.get("balance", 0.0)) <= 1e-9 * max(abs(rep["cut"]), abs(rep["fill"]), 1.0):
        dn.append(last)
    return {"sections": sections, "balance": dn}


def rows_count(e, s0, s1):
    """The rows' objects: the names' column's two edges; per row its line and name; each section in the view its five
    totals; a line at each section's end inside the view; DN at each balance point in it."""
    shown = [q for q in e["sections"] if q["to"] > s0 + 1e-9 and q["from"] < s1 - 1e-9]
    ends = sorted({round(s, 9) for q in e["sections"] for s in (q["from"], q["to"]) if s0 + 1e-9 < s < s1 - 1e-9})
    dn = [s for s in e["balance"] if s0 - 1e-9 < s < s1 + 1e-9]
    return 2 + 2 * len(ROWS) + 5 * len(shown) + len(ends) + len(dn)


def cases():
    base = hcmd.setup()
    first = len(base["entities"]) + 1
    road = base["settings"]["road"]
    out = []

    def ok(name, input_, warns=()):
        h = next(x for x in road["hauls"] if input_["haul"] in (x["id"], x["name"]))
        d = next(x for x in road["profiles"] if x["id"] == input_["profile"])
        kw = {k: input_[k] for k in ("from", "to") if k in input_}
        drawn, n, _ = pcmd.drawing(d, input_["at"], bands="kgm", frm=kw.get("from"), to=kw.get("to"))
        s0 = d["pvis"][0]["station"] if "from" not in input_ else input_["from"]
        s1 = d["pvis"][-1]["station"] if "to" not in input_ else input_["to"]
        e = earthwork(h)
        n += rows_count(e, s0, s1)
        result = {"status": "completed", "output": {**drawn, "uids": "$any", "revision": "$current"},
                  "warnings": list(warns)}
        expect = {**pcmd.STEPPED, "road": road, "ids": list(range(1, first + n))}
        out.append({"name": name, "steps": [{"op": "execute", "input": input_, "result": result, "expect": expect,
                                             "within": 1e-6}],
                    "earthwork": e, "profile": d["id"], "view": [s0, s1]})

    def no(name, input_, code, message, path):
        out.append({"name": name, "steps": [{"op": "execute", "input": input_,
                    "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
                    "expect": {**pcmd.UNCHANGED, "road": road}}]})

    at = {"layerId": "0", "at": [0.0, -200.0], "bands": "kgm"}
    stale = [pcmd.stale_warning(next(x for x in road["profiles"] if x["id"] == "b-2"))]
    ok("Hafriyat satırları: tek denge kesimi, fazlası DEPO", {**at, "profile": "b-1", "haul": "h-1"})
    ok("Döküm alanlı Brükner: dört kesim, DN'ler", {**at, "profile": "b-2", "haul": "Yan Brükner"}, warns=stale)
    ok("Görünümün parçası: km 100–300", {**at, "profile": "b-1", "haul": "h-1", "from": 100.0, "to": 300.0})
    no("Hafriyat satırları KentOS bantlarıyla: ret", {**at, "profile": "b-1", "haul": "h-1", "bands": "kentos"},
       "invalid_label", "Hafriyat satırları yalnız KGM bantlarıyla (bands: kgm) çizilir.", "haul")
    no("Olmayan Brükner: ret", {**at, "profile": "b-1", "haul": "yok"}, "unknown_haul",
       "“yok” kimlikli ya da adlı Brükner projede yok.", "haul")
    no("Başka aksın Brükneri: ret", {**at, "profile": "b-1", "haul": "h-2"}, "invalid_label",
       "“Yan Brükner” Brüknerinin kübajı boykesitin aksında değil.", "haul")
    return base, out


BANDS = ROOT / "fixtures/road/v1/earthwork-bands.json"


def main():
    """The command cases go into road.profiles.draw.json (road_profile_command_cases.py writes it); this writes each
    drawing case's sections, which KentOS's own are held to (crates/native/application/src/road_earthwork_bands.rs)."""
    base, cs = cases()
    data = {"note": "ADR 0995. scripts/fixtures/road_earthwork_band_cases.py yazar; KentOS kodu yok.", "setup": base,
            "cases": [{"name": c["name"], "haul": c["steps"][0]["input"]["haul"], "profile": c["profile"],
                       "earthwork": c["earthwork"]} for c in cs if "earthwork" in c]}
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not BANDS.exists() or BANDS.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{BANDS.name} güncel değil: python3 {Path(__file__).name}")
        print(f"{BANDS.name}: güncel")
        return
    BANDS.write_text(text, encoding="utf-8")
    print(f"{BANDS.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
