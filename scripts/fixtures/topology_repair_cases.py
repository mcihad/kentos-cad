#!/usr/bin/env python3
"""The shared cases of `cad.topology.repair` (docs/adr/1260 §4, plan/map-cad.md MAP-07), written without KentOS code.

    python3 scripts/fixtures/topology_repair_cases.py           # writes fixtures/commands/v1/cad.topology.repair.json
    python3 scripts/fixtures/topology_repair_cases.py --check   # compares

The findings and their lasting keys are the topology references' (topology_rules_cases.py: cells of the coordinates
and exact fractions; topology_check_cases.py: ADR 0524's FNV-1a key). What each fix makes is written here from ADR
0202 §4 for the scene's axis-parallel parcels, as the rings the fix leaves:

- Birinci nesneden çıkar (A − B), İkinci nesneden çıkar (B − A): the difference of two rectangles, by hand;
- Köşeyi sil on a short edge: the vertex at the edge's end goes (ADR 0202 §4), the two edges beside it become one;
- Sil on a two-vertex line's short edge: the line goes.

What the recipe reports is then measured here, exactly: an area by the shoelace over fractions; a source vertex kept
when the new ring has a vertex at its place, else moved by its distance to the nearest point of the new ring (a
segment's nearest point over fractions, the square root at 50 digits with `decimal`), the change of the areas' sum, the
open findings before and after (the references' check over the rings). The scene: topology_check_cases.py's five
parcels (A1 and A2 overlap in 2 × 6 m, A3 and A4 in 1 × 10 m, A5 has a 0.5 m edge), rules r1 (Çakışmamalı) and r2
(Kısa kenar, 1 m).
"""
import copy
import json
import sys
from decimal import Decimal, getcontext
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import topology_check_cases as tc  # noqa: E402
import topology_rules_cases as tr  # noqa: E402

getcontext().prec = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.topology.repair.json"
ANY = "$any"
E, N = tr.E, tr.N
P = tr.P
STEP_NAME = "Topoloji düzelt"
WITHIN = 1e-9

# ── the scene ──────────────────────────────────────────────────────────

A1, A2, A3, A4, A5 = (tc.uid(k) for k in range(1, 6))
RINGS = {
    A1: tr.rect(0, 0, 10, 10),
    A2: tr.rect(8, 2, 18, 8),
    A3: tr.rect(20, 0, 30, 10),
    A4: tr.rect(29, 0, 40, 10),
    A5: [P(50, 0), P(60, 0), P(60, 10), P(F(101, 2), 10), P(50, 10)],
}
ORDER = [A1, A2, A3, A4, A5]
# What each fix leaves (ADR 0202 §4), written by hand.
A1_LESS_A2 = [P(0, 0), P(10, 0), P(10, 2), P(8, 2), P(8, 8), P(10, 8), P(10, 10), P(0, 10)]
A2_LESS_A1 = tr.rect(10, 2, 18, 8)
A3_LESS_A4 = tr.rect(20, 0, 29, 10)
A5_LESS_CORNER = [P(50, 0), P(60, 0), P(60, 10), P(F(101, 2), 10)]


def shoelace(ring):
    s = F(0)
    for k, a in enumerate(ring):
        b = ring[(k + 1) % len(ring)]
        s += (a[0] - E) * (b[1] - N) - (b[0] - E) * (a[1] - N)
    return abs(s) / 2


def sqrt(q):
    """The square root of a fraction to 50 digits, as the nearest double."""
    return float((Decimal(q.numerator) / Decimal(q.denominator)).sqrt())


def seg_d2(p, a, b):
    """The squared distance from p to the segment ab, exactly."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    L = dx * dx + dy * dy
    t = F(0) if L == 0 else max(F(0), min(F(1), ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L))
    qx, qy = a[0] + t * dx, a[1] + t * dy
    return (p[0] - qx) ** 2 + (p[1] - qy) ** 2


def ring_d2(p, ring):
    return min(seg_d2(p, ring[k], ring[(k + 1) % len(ring)]) for k in range(len(ring)))


def shift(before, after):
    """Source vertices kept (the new ring has a vertex there) and moved (by their distance to the new ring)."""
    kept, moved = 0, []
    for v in before:
        if v in after:
            kept += 1
        else:
            moved.append(sqrt(ring_d2(v, after)))
    return kept, moved


def objs_of(rings, layers=None, lines=()):
    out = []
    for u in ORDER:
        if u in rings:
            o = tr.area_obj((layers or {}).get(u, "parsel"), rings[u])
            o.uid = u
            out.append(o)
    for u, a, b in lines:
        o = tr.line_obj("parsel", a, b)
        o.uid = u
        out.append(o)
    return out


def findings(rings, rules=tc.RULES, layers=None, lines=(), exceptions=(), errors=()):
    return tc.found(objs_of(rings, layers, lines), rules, exceptions=exceptions, errors=errors)


def open_count(fs):
    return sum(1 for f in fs if not f["exception"])


# ── the answer ─────────────────────────────────────────────────────────


def change(uid, before, after, steps, layer="parsel"):
    """An area the recipe changes (`after` None: deleted)."""
    c = {"uid": uid, "layer": layer}
    a0 = shoelace(before)
    if after is None:
        c.update(removed=True, areaBefore=float(a0), kept=0, moved=0, maxMoved=0.0, totalMoved=0.0, steps=steps)
        return c, [], float(-a0)
    kept, moved = shift(before, after)
    a1 = shoelace(after)
    c.update(areaBefore=float(a0), areaAfter=float(a1), kept=kept, moved=len(moved),
             maxMoved=max(moved, default=0.0), totalMoved=float(sum(Decimal(m) for m in moved)), steps=steps)
    return c, moved, float(a1 - a0)


def applied(f, fix):
    return {"key": f["key"], "fix": fix, "rule": f["rule"], "problem": f["problem"], "uids": f["uids"]}


def repaired(recipe, changes, before, after, precision=None):
    moved = [m for _, ms, _ in changes for m in ms]
    out = {"recipe": recipe, "changes": [c for c, _, _ in changes], "moved": len(moved),
           "maxMoved": max(moved, default=0.0), "totalMoved": float(sum(Decimal(m) for m in moved))}
    if precision is not None:
        out["belowPrecision"] = sum(1 for m in moved if m < precision)
    out.update(areaChange=float(sum(F(d) for _, _, d in changes)), openBefore=before, openAfter=after,
               revision="$current")
    return out


def done(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, expect=None, op="execute", command=None, within=None):
    s = {"op": op, "input": inp, "result": result, "expect": expect or {"revision": "same"}}
    if command:
        s["command"] = command
    if within is not None:
        s["within"] = within
    return s


def case(name, *steps, setup_=None):
    c = {"name": name, "steps": list(steps)}
    if setup_ is not None:
        c["setup"] = setup_
    return c


def steps_of(*pairs):
    return {"steps": [{"key": f["key"], "fix": fix} for f, fix in pairs]}


def corners(ring):
    return sorted([float(p[0]), float(p[1])] for p in ring)


def shape(ring):
    return {"corners": corners(ring), "area": round(float(shoelace(ring)) * 1e6) / 1e6, "parts": 1, "holes": 0}


NOT_FOUND = "Çizimin bugünkü denetiminde “{}” anahtarlı bulgu yok; önce cad.topology.check ile denetleyin."
GONE = ("Tarifin ilk {} adımından sonra çizimde “{}” anahtarlı bulgu yok: önceki adımlar onu giderdi ya da değiştirdi. "
        "Adımların sırasını ve anahtarlarını önizlemeyle denetleyin.")
NOT_OFFERED = "Bu bulgu “{}” düzeltmesini sunmuyor; sunduğu düzeltmeler: {}."
STEPS_TEXT = "Onarım tarifi 1 ile 1000 arasında adım ister; her adım bir bulgunun anahtarını ve düzeltmesini verir."
EXCEPTION_TEXT = ("“{}” anahtarlı bulgu istisna: bilerek bırakıldı. Onarmak için önce istisnayı kaldırın "
                  "(cad.topology.exceptions, unmark).")
LOCKED_TEXT = ("“{}” katmanı kilitli; tarifin {}. adımı üzerindeki nesneyi değiştirir. Hiçbir şey yazılmadı: kilidi "
               "Katmanlar panelinden açın ya da adımı tariften çıkarın.")
BEYOND_TEXT = ("Tarifin {}. adımı “{}” nesnesinin bir kaynak köşesini {:.4f} m taşıyor; kalite profilinin (“{}”) kabul "
               "eşiği {} m. Hiçbir şey yazılmadı: adımı tariften çıkarın ya da bulguyu ölçüyle düzeltin.")


def cases():
    out = []
    c = lambda name, *steps, setup_=None: out.append(case(name, *steps, setup_=setup_))  # noqa: E731
    fs = findings(RINGS)
    over, over2 = fs[0], fs[1]
    short = next(f for f in fs if f["rule"] == "r2")
    assert over["uids"] == [A1, A2] and over2["uids"] == [A3, A4] and short["uids"] == [A5], fs
    assert over["fixes"] == ["subtractFirst", "subtractSecond"] and short["fixes"] == ["removeVertex"], fs
    before = open_count(fs)
    assert before == 3

    # The fixes as ADR 0202 §4 writes them, and the open findings they leave.
    one = {**RINGS, A1: A1_LESS_A2}
    assert shoelace(A1_LESS_A2) == 88
    s1 = repaired([applied(over, "subtractFirst")], [change(A1, RINGS[A1], A1_LESS_A2, [0])], before,
                  open_count(findings(one)))
    assert s1["openAfter"] == 2 and s1["moved"] == 0, s1
    two = {**RINGS, A2: A2_LESS_A1}
    s2 = repaired([applied(over, "subtractSecond")], [change(A2, RINGS[A2], A2_LESS_A1, [0])], before,
                  open_count(findings(two)))
    assert s2["moved"] == 2 and s2["maxMoved"] == 2.0, s2
    three = {**RINGS, A3: A3_LESS_A4}
    s3 = repaired([applied(over2, "subtractFirst")], [change(A3, RINGS[A3], A3_LESS_A4, [0])], before,
                  open_count(findings(three)))
    # A5 without its corner has a slanted edge the cells cannot take; it is alone in its box, so the open findings left
    # are the two overlaps.
    s5 = repaired([applied(short, "removeVertex")], [change(A5, RINGS[A5], A5_LESS_CORNER, [0])], before, 2)
    assert s5["moved"] == 1 and abs(s5["maxMoved"] - 10 / 401 ** 0.5) < 1e-15 and s5["areaChange"] == -2.5, s5
    both = repaired([applied(over, "subtractFirst"), applied(short, "removeVertex")],
                    [change(A1, RINGS[A1], A1_LESS_A2, [0]), change(A5, RINGS[A5], A5_LESS_CORNER, [1])], before, 1)
    assert both["areaChange"] == -14.5, both
    recipe = steps_of((over, "subtractFirst"), (short, "removeVertex"))

    c("plan: ilk nesneden çıkar; A1 küçülür, kaynak köşeleri yerinde kalır, hiçbir şey yazılmaz",
      step(steps_of((over, "subtractFirst")), done(s1), op="plan", within=WITHIN))
    c("plan: ikinci nesneden çıkar; A2'nin iki köşesi 2 m taşınır",
      step(steps_of((over, "subtractSecond")), done(s2), op="plan", within=WITHIN))
    c("plan: A3'ten A4'ü çıkar; iki köşe 1 m taşınır",
      step(steps_of((over2, "subtractFirst")), done(s3), op="plan", within=WITHIN))
    c("plan: kısa kenarın köşesini sil; köşe yeni kenara 10/√401 m uzak, alan 2,5 m² azalır",
      step(steps_of((short, "removeVertex")), done(s5), op="plan", within=WITHIN))
    c("plan: iki adımlı tarif, değişiklikler çizim sırasıyla, açık bulgu 3'ten 1'e",
      step(recipe, done(both), op="plan", within=WITHIN))
    c("uygula: tek geri alma adımı, nesnelerin yeni biçimi",
      step(recipe, done(both), {"revision": "changed", "canUndo": True, "dirty": True,
                                "shapes": {"1": shape(A1_LESS_A2), "5": shape(A5_LESS_CORNER)}}, within=WITHIN))
    c("uygulanan tarif geri alınınca çizim önceki hâlinde",
      step(recipe, done(both), {"revision": "changed"}, within=WITHIN),
      {"op": "undo", "returns": STEP_NAME, "expect": {"shapes": {"1": shape(RINGS[A1]), "5": shape(RINGS[A5])}}},
      step({}, tc.done(tc.answer(fs)), command="cad.topology.check"))
    # The same recipe over the project saved and opened again: the same answer, the same change set.
    c("yeniden açılan projede aynı tarif aynı değişiklik kümesini verir",
      step(recipe, done(both), op="plan", within=WITHIN), {"op": "reopen"},
      step(recipe, done(both), op="plan", within=WITHIN))
    c("yeniden açılan projede uygulanan tarif aynı biçimleri yazar",
      {"op": "reopen"},
      step(recipe, done(both), {"revision": "changed", "canUndo": True,
                                "shapes": {"1": shape(A1_LESS_A2), "5": shape(A5_LESS_CORNER)}}, within=WITHIN))
    c("uygulandıktan sonra kalan tek açık bulgu da onarılır",
      step(recipe, done(both), {"revision": "changed"}, within=WITHIN),
      step(steps_of((over2, "subtractFirst")),
           done(repaired([applied(over2, "subtractFirst")], [change(A3, RINGS[A3], A3_LESS_A4, [0])], 1, 0)),
           op="plan", within=WITHIN))
    # Refusals: nothing is written.
    c("adımsız tarif reddedilir", step({"steps": []}, failed("invalid_input", STEPS_TEXT, "steps")))
    c("1000'den çok adım reddedilir",
      step({"steps": [{"key": over["key"], "fix": "subtractFirst"}] * 1001}, failed("invalid_input", STEPS_TEXT, "steps")))
    c("bilinmeyen anahtar", step({"steps": [{"key": "0000000000000000", "fix": "subtractFirst"}]},
                                 failed("finding_not_found", NOT_FOUND.format("0000000000000000"), "steps[0].key")))
    c("önceki adımın giderdiği bulgu ikinci kez onarılamaz, hiçbir şey yazılmaz",
      step(steps_of((over, "subtractFirst"), (over, "subtractFirst")),
           failed("finding_not_found", GONE.format(1, over["key"]), "steps[1].key")),
      step({}, tc.done(tc.answer(fs)), command="cad.topology.check"))
    c("bulgunun sunmadığı düzeltme",
      step(steps_of((over, "removeVertex")),
           failed("fix_unavailable", NOT_OFFERED.format("removeVertex", ", ".join(over["fixes"])), "steps[0].fix")))
    x1 = tc.exc_of(over)
    c("istisna olan bulgu onarılmaz",
      step(steps_of((over, "subtractFirst")), failed("repair_refused_exception", EXCEPTION_TEXT.format(over["key"]),
                                                      "steps[0].key")),
      setup_=tc.setup(exceptions=[x1]))
    c("hata diye onaylanmış bulgu onarılır; açık sayılır",
      step(steps_of((over, "subtractFirst")), done(s1), op="plan", within=WITHIN),
      setup_=tc.setup(exceptions=[dict(x1, decision="error", reason="Sayısallaştırma hatası")]))
    # A locked neighbour: A5 on a locked layer with its own short-edge rule. The first step is the drawing's, the
    # second the locked layer's: the whole recipe is refused, A1 stays.
    r3 = tr.rule("r3", "mustNotHaveShortEdges", "kilitli", value=1)
    locked = tc.setup(rules=[*tc.RULES, r3], layers=("parsel", "kilitli"))
    locked["layers"][1]["locked"] = True
    locked["entities"][4]["layerId"] = "kilitli"
    lfs = findings(RINGS, rules=[*tc.RULES, r3], layers={A5: "kilitli"})
    lshort = next(f for f in lfs if f["rule"] == "r3")
    assert lshort["uids"] == [A5] and not any(f["rule"] == "r2" for f in lfs), lfs
    c("kilitli katmandaki komşuyu değiştiren adım bütün tarifi reddeder; ilk adım da yazılmaz",
      step(steps_of((over, "subtractFirst"), (lshort, "removeVertex")),
           failed("layer_locked", LOCKED_TEXT.format("Kilitli", 2), "steps[1]")),
      step({}, tc.done(tc.answer(lfs, rules=3)), command="cad.topology.check"),
      setup_=locked)
    # The quality profile: the acceptance threshold and the storage precision (ADR 1260 §2).
    strict = tc.setup(profile=dict(tc.PROFILE))
    c("kabul eşiği: köşeyi 0,05 m'den uzağa taşıyan adım reddedilir",
      step(steps_of((short, "removeVertex")),
           failed("repair_beyond_acceptance",
                  BEYOND_TEXT.format(1, A5, 10 / 401 ** 0.5, "Kadastro güncelleme", 0.05), "steps[0]")),
      setup_=strict)
    c("kabul eşiği: iki adımda en uzağa taşıyan adım söylenir",
      step(steps_of((over, "subtractFirst"), (short, "removeVertex")),
           failed("repair_beyond_acceptance",
                  BEYOND_TEXT.format(2, A5, 10 / 401 ** 0.5, "Kadastro güncelleme", 0.05), "steps[1]")),
      setup_=strict)
    c("kabul eşiği: kaynak köşesini taşımayan adım geçer; hassasiyetin altında taşınan yok",
      step(steps_of((over, "subtractFirst")), done({**s1, "belowPrecision": 0}), op="plan", within=WITHIN),
      setup_=strict)
    wide = tc.setup(profile=dict(tc.PROFILE, acceptance=1.0))
    s5p = repaired([applied(short, "removeVertex")], [change(A5, RINGS[A5], A5_LESS_CORNER, [0])], before, 2,
                   precision=tc.PROFILE["precision"])
    c("kabul eşiği 1 m: köşe silinir, taşınma raporlanır, yuvarlanmaz",
      step(steps_of((short, "removeVertex")), done(s5p), op="plan", within=WITHIN), setup_=wide)
    c("kabul eşiği 1 m: 2 m taşıyan ikinci nesneden çıkar reddedilir",
      step(steps_of((over, "subtractSecond")),
           failed("repair_beyond_acceptance", BEYOND_TEXT.format(1, A2, 2.0, "Kadastro güncelleme", "1"), "steps[0]")),
      setup_=wide)
    # A short line deleted: the change says it is removed and its length.
    L = tc.uid(6)
    a, b = P(70, 20), P(F(141, 2), 20)
    lined = tc.setup()
    lined["entities"].append({"kind": "line", "a": tr.jp(a), "b": tr.jp(b), "id": 6, "layerId": "parsel", "attrs": {}})
    lined["uids"].append(L)
    lfs2 = findings(RINGS, lines=[(L, a, b)])
    lshort2 = next(f for f in lfs2 if f["uids"] == [L])
    assert lshort2["fixes"] == ["deleteObject"], lshort2
    gone = repaired([applied(lshort2, "deleteObject")],
                    [({"uid": L, "layer": "parsel", "removed": True, "lengthBefore": 0.5, "kept": 0, "moved": 0,
                       "maxMoved": 0.0, "totalMoved": 0.0, "steps": [0]}, [], 0.0)], open_count(lfs2), 3)
    c("kısa çizgi silinir: removed ve boyu",
      step(steps_of((lshort2, "deleteObject")), done(gone), {"revision": "changed", "canUndo": True}, within=WITHIN),
      setup_=lined)
    c("beklenen sürüm tutmazsa çakışma",
      step(dict(recipe, expectedRevision="999"),
           {"status": "conflict", "error": {
               "code": "revision_conflict",
               "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki "
                          "hâline göre yeniden hazırlayın.",
               "path": "expectedRevision", "revision": "$current"}}))
    c("beklenen sürüm yazı değilse ret",
      step(dict(recipe, expectedRevision="x"),
           failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                  "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")))
    c("beklenen sürüm tutarsa yazılır",
      step(dict(recipe, expectedRevision="$current"), done(both), {"revision": "changed", "canUndo": True},
           within=WITHIN))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.topology.repair", "commandVersion": 1,
        "title": "Topoloji onarım tarifi (ADR 1260)",
        "note": "Bulgular ve anahtarlar topology_rules_cases.py ve topology_check_cases.py başvurularından; düzeltmelerin "
                "bıraktığı halkalar ADR 0202 §4'ten elle; alan kesirlerle, kaynak köşenin yeni halkaya uzaklığı kesirlerle "
                "ve 50 basamaklı karekökle. scripts/fixtures/topology_repair_cases.py yazar.",
        "setup": tc.setup(),
        "cases": cases(),
    }


def main():
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"güncel değil: {OUT.name}; betiği --check'siz çalıştırın")
        print(f"{OUT.name}: {len(data['cases'])} durum")
        return
    OUT.write_text(text)
    print(f"{OUT.name}: {len(data['cases'])} durum yazıldı")


if __name__ == "__main__":
    _ = copy
    main()
