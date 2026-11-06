#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.hauls.approve.json: the shared cases of KentOS Yol's haul plan approval (plan/road.md
Y48, docs/adr/0954; ROAD-HAUL-08), their expectations built here without KentOS code. The drawing, its corridors,
volumes and diagrams are road_haul_plan_cases.py's; the plan approved is that script's (HiGHS through scipy's linprog).

An approval records the plan as planned now with its objective: its number (one after the diagram's approvals so far),
the day and a note, the diagram's and its volume's revisions and the volume's approval, the moment, the volume moved,
the cost's total and each site's capacity and use. A new approval of an approved plan sends the old one to the history,
ended on the new one's day; a revoked one goes there ended with `revoked`, its day and why. A plan that cannot place
everything is not approved.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj --with scipy \\
        python3 scripts/fixtures/road_haul_approve_cases.py [--check]
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_grades  # noqa: E402
import road_haul_command_cases as hc  # noqa: E402
import road_haul_plan_cases as pc  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-3
completed, failed, step, case = hc.completed, hc.failed, hc.step, hc.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = hc.UNCHANGED, hc.STEPPED, hc.CONFLICT, hc.BAD_REVISION
H1, H2, H3, VOLUMES = pc.H1, pc.H2, pc.H3, pc.VOLUMES
STEP = "Taşıma planı onayı"
TWO = [pc.site("D1", "waste", 40.0, 10000.0), pc.site("D2", "waste", 590.0, 25000.0)]
P = {"currency": "TL", "overhaul": 800.0, "waste": 40.0, "borrow": 60.0}
HA = {**H1, "sites": TWO}
START = [HA, H2, H3]


def day(d):
    b = d.encode()
    return len(b) == 10 and all(c == ord("-") if k in (4, 7) else chr(c).isdigit() for k, c in enumerate(b))


def rule(h, j=None):
    """Docs/adr/0954's rule on the diagram's approvals: the first breach's words and path, else none."""
    def one(a, path):
        end = a.get("ended", {})
        if not day(a["at"]) or ("at" in end and not day(end["at"])):
            return path, "onay ve bitiş günleri YYYY-AA-GG biçiminde olmalı"
        if len(a.get("note", "")) > 500 or len(end.get("note", "")) > 500:
            return path, "onay ve bitiş notları en çok 500 karakter olur"
        return None
    if "approved" in h:
        p = one(h["approved"], "approved")
        if p:
            return p
    for k, a in enumerate(h.get("history", [])):
        p = one(a, f"history[{k}]")
        if p:
            return p
    return None


def approve(inp, hauls, volumes=VOLUMES):
    """The diagrams after `inp`, and the one it wrote."""
    h = pc.find(inp["haul"], hauls)
    at = next(i for i, x in enumerate(hauls) if x["id"] == h["id"])
    new = dict(h)
    if inp.get("revoke"):
        if "approved" not in h:
            raise hc.Refused("haul_not_approved",
                             f"“{h['name']}” Brüknerinin onaylı taşıma planı yok; kaldırılacak onay yok.", "haul")
        end = {"revoked": True}
        if "at" in inp:
            end["at"] = inp["at"]
        if "note" in inp:
            end["note"] = inp["note"]
        new = {k: v for k, v in h.items() if k != "approved"}
        new["history"] = list(h.get("history", [])) + [{**h["approved"], "ended": end}]
    else:
        if "at" not in inp:
            raise hc.Refused("invalid_haul", "Onay için günü (at, YYYY-AA-GG) verin.", "at")
        objective = inp.get("objective", "moment")
        plan = pc.answer({"haul": h["id"], "objective": objective}, hauls, volumes)[0]
        if not plan["feasible"]:
            raise hc.Refused("invalid_haul",
                             f"“{h['name']}” taşıma planı her şeyi yerleştiremiyor (karşılanmayan dolgu {plan['shortage']:.3f} m³, "
                             f"yerleşmeyen yarma {plan['surplus']:.3f} m³); kurulamayan plan onaylanmaz.", "haul")
        numbers = [a["number"] for a in h.get("history", [])] + ([h["approved"]["number"]] if "approved" in h else [])
        new["approved"] = pc.approval_of(h, plan, inp["at"], volumes, max(numbers, default=0) + 1, inp.get("note"))
        if "approved" in h:
            new["history"] = list(h.get("history", [])) + [{**h["approved"], "ended": {"at": inp["at"]}}]
    broken = rule(new)
    if broken:
        raise hc.Refused("invalid_haul", f"{STEP} kurallarına uymuyor: {broken[1]}.", broken[0])
    out = list(hauls)
    out[at] = new
    return out, new["id"]


def written(after, target, volumes=VOLUMES):
    return {"hauls": hc.listed(after), "revision": "$current",
            "haul": hc.report(next(h for h in after if h["id"] == target), volumes)}


def cases():
    out = []

    def ok(name, inp, before=START, volumes=VOLUMES, op="execute", undo=False):
        after, target = approve(inp, before, volumes)
        base = hc.setup(before, volumes)
        result = completed(None) if op == "validate" else completed(written(after, target, volumes))
        writes = op == "execute"
        road = hc.road_of(after if writes else before, volumes)
        steps = [step(op, inp, result, {**(STEPPED if writes else UNCHANGED), "road": road})]
        steps[0]["within"] = W
        if undo:
            steps.append({"op": "undo", "returns": STEP, "expect": {"road": hc.road_of(before, volumes), "canRedo": True}})
        out.append(case(name, steps, base))

    def no(name, inp, before=START, volumes=VOLUMES, result=None, setup_=None):
        if result is None:
            try:
                approve(inp, before, volumes)
                raise AssertionError(f"{name}: kabul edildi")
            except hc.Refused as r:
                result = r.result
        base = setup_ or hc.setup(before, volumes)
        road = base["settings"].get("road")
        out.append(case(name, [step("execute", inp, result, {**UNCHANGED, "road": road})], base))

    ok("Moment planı onaylanır", {"haul": "h-1", "at": "2026-11-04"}, undo=True)
    ok("Notla ve adla onaylanır", {"haul": "Ana Brükner", "at": "2026-11-04", "note": "Şantiye şefiyle görüşüldü"})
    hp = {**H1, "cost": P, "sites": TWO}
    ok("Ekonomik plan onaylanır", {"haul": "h-1", "at": "2026-11-05", "objective": "cost"}, [hp, H2, H3])
    once, _ = approve({"haul": "h-1", "at": "2026-11-01", "note": "İlk"}, START)
    grown = [{**once[0], "revision": 2, "sites": [pc.site("D1", "waste", 40.0, 14000.0), TWO[1]]}, H2, H3]
    ok("Yeniden onay eskisini geçmişe yazar", {"haul": "h-1", "at": "2026-11-04"}, grown, undo=True)
    ok("Onay kaldırılır", {"haul": "h-1", "revoke": True, "at": "2026-11-04", "note": "Döküm alanı iptal"}, once, undo=True)
    ok("Günsüz kaldırma", {"haul": "h-1", "revoke": True}, once)
    gone, _ = approve({"haul": "h-1", "revoke": True, "at": "2026-11-02"}, once)
    ok("Kaldırılmış onaydan sonra yeni onay 2. olur", {"haul": "h-1", "at": "2026-11-04"}, gone)
    signed_volumes = [vcmd.approved(vcmd.V1), vcmd.V2, pc.V3]
    ok("Onaylı kübajın onay numarası yazılır", {"haul": "h-1", "at": "2026-11-04"}, START, signed_volumes)
    ok("Yan Brükner: ödünç ve döküm alanlarıyla", {"haul": "h-2", "at": "2026-11-04"},
       [HA, {**H2, "sites": [pc.site("Ödünç", "borrow", 50.0, 2000.0), pc.site("Döküm 1", "waste", 400.0, 20000.0)]}, H3])
    ok("Plan yazmaz", {"haul": "h-1", "at": "2026-11-04"}, op="plan")
    ok("Doğrulama yazmaz", {"haul": "h-1", "at": "2026-11-04"}, op="validate")

    no("Yol projesi değil", {"haul": "h-1", "at": "2026-11-04"}, result=hc.NOT_ROAD, setup_=hc.setup(work="map"))
    no("Olmayan Brükner", {"haul": "yok", "at": "2026-11-04"})
    no("Onaysız planın onayı kaldırılmaz", {"haul": "h-1", "revoke": True})
    no("Günsüz onay", {"haul": "h-1"})
    no("Kurulamayan plan onaylanmaz", {"haul": "h-1", "at": "2026-11-04"}, [H1, H2, H3])
    no("Fiyatsız ekonomik plan onaylanmaz", {"haul": "h-1", "at": "2026-11-04", "objective": "cost"})
    no("Günün biçimi yanlış", {"haul": "h-1", "at": "04.11.2026"})
    no("Uzun not", {"haul": "h-1", "at": "2026-11-04", "note": "x" * 501})
    no("Kaldırmanın günü yanlış", {"haul": "h-1", "revoke": True, "at": "2026-1-4"}, once)
    no("Geçersiz beklenen sürüm", {"haul": "h-1", "at": "2026-11-04", "expectedRevision": "on"}, result=BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"haul": "h-1", "at": "2026-11-04", "expectedRevision": "41"}, result=CONFLICT)
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.hauls.approve", "commandVersion": 1,
           "title": "Taşıma planını onayla",
           "note": "scripts/fixtures/road_haul_approve_cases.py yazar; çizim, koridorlar, kübajlar, Brüknerler ve plan "
                   "road_haul_plan_cases.py'nin (HiGHS, scipy linprog); onay planın o günkü sayılarını ve dayandığı "
                   "sürümleri yazar, biten onay geçmişe gider; KentOS kodu yok. `expect.road` projenin yol ayarlarıdır.",
           "setup": hc.setup(START, VOLUMES), "cases": found}
    doc = road_grades.fixture(doc)
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.hauls.approve.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.hauls.approve.json")
    else:
        path.write_text(text)
    print(f"road.hauls.approve.json: {len(found)} durum")


if __name__ == "__main__":
    main()
