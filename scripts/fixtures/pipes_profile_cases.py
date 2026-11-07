#!/usr/bin/env python3
"""The shared cases of `cad.pipes.profile` (Hat profili; plan/map-cad.md MAP-16, docs/adr/1860), without KentOS code.

    python3 scripts/fixtures/pipes_profile_cases.py           # writes fixtures/commands/v1/cad.pipes.profile.json
    python3 scripts/fixtures/pipes_profile_cases.py --check   # writes nothing; compares

The drawing is pipes_check_cases.py's (the ground lattice, the sewer and the storm networks) with a layer for the
profile and the usual hidden, locked and group layers. The rules, by hand:

- the run: from the start pipe along the flow (drawn, or against it where `Akış` = `ters`), through every node exactly
  one pipe not yet taken leaves; it ends at the pipe asked (`end`), where none leaves (`outlet`) or where more than one
  does (`split`); a node is a shared end;
- a pipe's length, inverts, slope, diameter and least cover as pipes_check_cases.py's report reads them; its distances
  along the run; the ground along it at its vertices and every lattice line crossing (a curved pipe: its vertices),
  the way it flows, a place repeated at a node taken once; with the structures' cover levels, the ground is the nodes';
- each node's structure (the junction at the place, its `Ad`), the ground there (the lattice's height or the structure's
  `Kapak kotu`), the run's inverts in and out, the depth (the ground less the lower);
- the datum a metre under the lowest elevation shown (ground, inverts, crowns, nodes' ground), floored; the top a metre
  over the highest, ceiled;
- the objects drawn, counted: the frame; the grid (a step of 1, 2, 5, 10 … giving at most ten, a line at each height
  inside the frame, the height written on both sides); the ground, the inverts' and the crowns' lines (each a run of two
  or more known places); a line for each node with a ground and an invert; the bands' ten lines across, three down and a
  tick at every node; the nine bands' names; each node's name, distance, ground, inverts and depth where known; each
  pipe's slope and diameter where known and its length.

    The objects' places are the command's; the cases hold the profile's values and the objects' count.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pipes_check_cases as check  # noqa: E402
import road_drainage_cases as drainage  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.pipes.profile.json"
X0, Y0 = check.X0, check.Y0
W = 1e-6
ANY = "$any"
AT = [X0, Y0 - 300.0]
UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
BANDS = 9
SEWER_IN = check.SEWER_IN
SEWER_D = {"field": "Çap", "unit": "mm"}
ZEMIN = check.ZEMIN
KAPAK = check.KAPAK
STORM_D = check.STORM_D


def setup(work="map", extra=False):
    s = check.setup(work)
    s["layers"] += [check.layer("profil", "Profil"),
                    {**check.layer("gizli", "Gizli"), "visible": False},
                    {**check.layer("kilitli", "Kilitli"), "locked": True},
                    {**check.layer("grup", "Grup"), "type": "group"}]
    if extra:
        # A pipe leaving B3 to the south beside P3: the flow splits there.
        n = len(s["entities"]) + 1
        s["entities"].append({"kind": "line", "id": n, "layerId": "atiksu",
                              "attrs": {"Ad": "P10", "Akar baş": "97.0", "Akar son": "96.8", "Çap": "300"},
                              "a": check.P((100, 50)), "b": check.P((130, 30))})
        s["uids"].append(check.uid(n))
    return s


def entity_count(s):
    return len(s["entities"])


# ── The network as the drawing has it ─────────────────────────────────────────────────────────────────────────────
def network(name, extra=False):
    """The pipes (check.Pipe with their uid) and the structures {place: (uid, attrs)} of a network."""
    pipes, structures = check.drawing()[name]
    if name == "atiksu":
        st = {check.M[m]: (check.uid(231 + m), {"Ad": f"B{m}"}) for m in sorted(check.M)}
        if extra:
            p = check.Pipe(258, "P10", [(100, 50), (130, 30)], None,
                           {"Ad": "P10", "Akar baş": "97.0", "Akar son": "96.8", "Çap": "300"}, None, "atiksu")
            pipes = pipes + [p]
    else:
        st = {}
        for k, (place, cover) in enumerate(check.S):
            attrs = {"Ad": f"I{k + 1}"}
            if cover is not None:
                attrs["Kapak kotu"] = cover
            st[place] = (check.uid(250 + k), attrs)
    return pipes, st


def key(p):
    return (round(p[0], 6), round(p[1], 6))


def grid_step(datum, top):
    return next((g for g in (1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0) if (top - datum) / g <= 10.0),
                1000.0)


def grid_heights(datum, top):
    g = grid_step(datum, top)
    k = math.ceil(datum / g)
    out = []
    while k * g <= top + 1e-9:
        out.append(k * g)
        k += 1
    return out


def runs(points):
    out, open_ = [], False
    for s, z in points:
        if z is None:
            open_ = False
            continue
        if not open_:
            out.append([])
            open_ = True
        out[-1].append((s, z))
    return [r for r in out if len(r) >= 2]


def profile(net, start, end=None, inverts="fields", diameter=None, ground_=None, datum=None, exaggeration=None,
            extra=False):
    pipes, structures = network(net, extra)
    by_uid = {check.uid(p.n): i for i, p in enumerate(pipes)}
    flows = []
    for p in pipes:
        a, b = key(p.pts[0]), key(p.pts[-1])
        flows.append((b, a) if p.attrs.get("Akış") == "ters" else (a, b))
    first = by_uid[start]
    last = by_uid[end] if end is not None else None
    run, seen = [first], {first}
    while True:
        at = run[-1]
        if at == last:
            ends = "end"
            break
        nxt = [i for i, f in enumerate(flows) if f[0] == flows[at][1] and i not in seen]
        if not nxt:
            ends = "outlet"
            break
        if len(nxt) > 1:
            ends = "split"
            break
        seen.add(nxt[0])
        run.append(nxt[0])
    rows, ground, inv_pts, nodes = [], [], [], []
    s0 = 0.0
    for idx, i in enumerate(run):
        p = pipes[i]
        rev = p.attrs.get("Akış") == "ters"
        L = p.length
        start_at, end_at = p.pts[0], p.pts[-1]
        up_at, down_at = (end_at, start_at) if rev else (start_at, end_at)
        if idx == 0:
            nodes.append(up_at)
        nodes.append(down_at)
        if inverts == "fields":
            a, b = check.number(p.attrs.get("Akar baş")), check.number(p.attrs.get("Akar son"))
            ends_ = (a, b) if a is not None and b is not None else None
        else:
            ends_ = None
        i0, i1 = p.invert(ends_, 0.0), p.invert(ends_, L)
        up, down = (i1, i0) if rev else (i0, i1)
        slope = (up - down) / L if up is not None and down is not None else None
        dm = None
        if diameter:
            v = check.number(p.attrs.get(diameter["field"]))
            dm = v * check.UNITS[diameter["unit"]] if v is not None and v > 0 else None
        crown = 0.0 if not diameter else dm

        def along(st):
            return s0 + (L - st if rev else st)
        if inverts == "heights" and p.heights is not None:
            pts = sorted((along(st), z) for st, z in zip(p.st, p.heights))
        else:
            pts = sorted((along(st), z) for st, z in ((0.0, i0), (L, i1)) if z is not None)
        if not pts:
            inv_pts.append((s0, None, None))
        for s, z in pts:
            inv_pts.append((s, z, None if crown is None else z + crown))
        if ground_ and ground_["from"] == "surface":
            samples = [(s, q, check.ground(*q)) for s, q in p.samples()]
        elif ground_:
            def level(q):
                st = structures.get(key(q))
                return check.number(st[1].get(ground_["field"])) if st else None
            samples = [(0.0, start_at, level(start_at)), (L, end_at, level(end_at))]
        else:
            samples = []
        mn = None
        if crown is not None:
            for st, _, z in samples:
                inv = p.invert(ends_, st)
                if z is not None and inv is not None:
                    c = z - (inv + crown)
                    mn = c if mn is None else min(mn, c)
        if ground_ and ground_["from"] == "surface":
            for s, z in sorted((along(st), z) for st, _, z in samples):
                if not ground or abs(ground[-1]["station"] - s) > 1e-9:
                    ground.append({"station": s, **({"z": z} if z is not None else {})})
        row = {"uid": check.uid(p.n), "name": p.attrs["Ad"], "from": s0, "to": s0 + L, "length": L}
        if rev:
            row["reversed"] = True
        for k_, v in (("upstream", up), ("downstream", down), ("slope", slope), ("diameter", dm), ("minCover", mn)):
            if v is not None:
                row[k_] = v
        rows.append(row)
        s0 += L
    out_nodes = []
    for n_i, place in enumerate(nodes):
        st = structures.get(key(place))
        station = 0.0 if n_i == 0 else rows[n_i - 1]["to"]
        if ground_ and ground_["from"] == "surface":
            g = check.ground(*place)
        elif ground_:
            g = check.number(st[1].get(ground_["field"])) if st else None
        else:
            g = None
        inlet = rows[n_i - 1].get("downstream") if n_i > 0 else None
        outlet = rows[n_i].get("upstream") if n_i < len(rows) else None
        low = min([v for v in (inlet, outlet) if v is not None], default=None)
        node = {"station": station}
        if st:
            node["uid"], node["name"] = st[0], st[1]["Ad"]
        for k_, v in (("ground", g), ("inlet", inlet), ("outlet", outlet),
                      ("depth", g - low if g is not None and low is not None else None)):
            if v is not None:
                node[k_] = v
        out_nodes.append(node)
    if ground_ and ground_["from"] == "structures":
        ground = [{"station": n["station"], **({"z": n["ground"]} if "ground" in n else {})} for n in out_nodes]
    zs = [g["z"] for g in ground if "z" in g]
    zs += [v for _, z, c in inv_pts for v in (z, c) if v is not None]
    zs += [n["ground"] for n in out_nodes if "ground" in n]
    if not zs:
        return None
    lo, hi = min(zs), max(zs)
    datum_ = math.floor(lo) - 1.0 if datum is None else datum
    if datum is not None and datum > lo + 1e-9:
        return ("datum", lo)
    top = math.ceil(hi) + 1.0
    out = {"network": net, "pipes": rows, "structures": out_nodes}
    if ground:
        out["ground"] = ground
    out.update({"length": s0, "ends": ends, "datum": datum_, "top": top,
                "exaggeration": 10.0 if exaggeration is None else exaggeration})
    # The objects drawn, counted.
    heights = grid_heights(datum_, top)
    count = 1 + sum(1 for v in heights if datum_ + 1e-9 < v < top - 1e-9) + 2 * len(heights)
    count += len(runs((g["station"], g.get("z")) for g in ground))
    count += len(runs((s, z) for s, z, _ in inv_pts))
    if diameter:
        count += len(runs((s, c) for s, _, c in inv_pts))
    count += sum(1 for n in out_nodes if "ground" in n and ("inlet" in n or "outlet" in n))
    count += (BANDS + 1) + 3 + len(out_nodes) + BANDS
    count += sum(1 + sum(1 for k_ in ("name", "ground", "inlet", "outlet", "depth") if k_ in n) for n in out_nodes)
    count += sum(1 + sum(1 for k_ in ("slope", "diameter") if k_ in r) for r in rows)
    return out, count


def P_UID(name, net="atiksu"):
    pipes, _ = network(net, extra=True)
    return next(check.uid(p.n) for p in pipes if p.attrs["Ad"] == name)


def failed(code, message, path=None):
    err = {"code": code, "message": message}
    if path is not None:
        err["path"] = path
    return {"status": "failed", "error": err}


def cases():
    out = []
    base_n = entity_count(setup())

    def ok(name, inp, op="execute", warns=(), note=None, setup_=None, extra=False, net="atiksu"):
        args = dict(start=inp["start"], end=inp.get("end"), inverts=inp["inverts"]["from"],
                    diameter=inp.get("diameter"), ground_=inp.get("ground"), datum=inp.get("datum"),
                    exaggeration=inp.get("exaggeration"), extra=extra)
        o, count = profile(inp["network"] if inp["network"] in ("atiksu", "yagmur") else net, **args)
        o = dict(o)
        o["revision"] = "$current"
        written = op == "execute"
        n = entity_count(setup_ if isinstance(setup_, dict) else setup(extra=extra))
        if written:
            o["uids"] = [ANY] * count
        result = ({"status": "completed", "output": o, "warnings": list(warns)} if op != "validate"
                  else {"status": "completed", "output": None, "warnings": list(warns)})
        expect = ({**STEPPED, "ids": list(range(1, n + count + 1))} if written
                  else {**UNCHANGED, "ids": list(range(1, n + 1))})
        c = {"name": name, **({"note": note} if note else {}),
             "steps": [{"op": op, "input": inp, "result": result, "expect": expect, "within": W}]}
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)
        return o

    def no(name, inp, result, setup_=None):
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": {**UNCHANGED}}]}
        if setup_ is not None:
            c["setup"] = setup_
        out.append(c)

    def sewer(**over):
        inp = {"network": "atiksu", "start": P_UID("P1"), "inverts": SEWER_IN, "diameter": SEWER_D, "ground": ZEMIN,
               "layerId": "profil", "at": AT, **over}
        return {k_: v for k_, v in inp.items() if v is not None}

    def storm(**over):
        inp = {"network": "yagmur", "start": P_UID("Q1", "yagmur"), "inverts": {"from": "heights"},
               "diameter": STORM_D, "ground": KAPAK, "layerId": "profil", "at": AT, **over}
        return {k_: v for k_, v in inp.items() if v is not None}

    main = ok("Atıksu ana hattı P1'den çıkışa: altı boru", sewer(),
              note="P1–P4 y = 50 boyunca, P8 düz, P9 kafesin dışına çıkar (zemin orada yok); B9 çıkış")
    assert [p["name"] for p in main["pipes"]] == ["P1", "P2", "P3", "P4", "P8", "P9"], main["pipes"]
    assert main["ends"] == "outlet"
    ok("Bitiş borusuyla: P1'den P3'e", sewer(end=P_UID("P3")))
    ok("Ortadan başlayan: P2'den", sewer(start=P_UID("P2")))
    split = ok("Çatal: B3'ten iki boru çıkar, hat orada biter", sewer(start=P_UID("P2")), setup_=setup(extra=True),
               extra=True)
    assert split["ends"] == "split" and len(split["pipes"]) == 1, split
    st = ok("Yağmursuyu: köşe kotları, ters akan Q2, yaylı Q3, zemin ızgaraların kapak kotundan", storm(), net="yagmur")
    assert [p["name"] for p in st["pipes"]] == ["Q1", "Q2", "Q3"] and st["pipes"][1].get("reversed"), st["pipes"]
    ok("Yağmursuyu yüzeyle: yaylı boruda zemin yalnız köşelerde", storm(ground=ZEMIN), net="yagmur")
    ok("Zeminsiz: yalnız akar ve tepe", sewer(ground=None))
    ok("Çapsız: tepe çizgisi yok, örtü akara", sewer(diameter=None))
    ok("Tek başına P7: akar kotu yok, yalnız zemin", sewer(start=P_UID("P7")))
    ok("Kıyas kotu verilmiş", sewer(datum=90.0))
    ok("Abartı 5, yazı 3 mm", sewer(exaggeration=5.0, height=3.0))
    ok("Ağ adıyla", sewer(network="Atıksu"))
    ok("Plan yazmaz", sewer(), op="plan")
    ok("Doğrulama yazmaz", sewer(), op="validate")
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    ok("Gizli katmana uyarıyla", sewer(layerId="gizli"), warns=[hidden])
    ok("Yol projesinde de", sewer(), setup_=setup("road"))
    ok("CBS projesinde kapak kotuyla", storm(), setup_=setup("gis"), net="yagmur")
    # Refusals.
    bad = "invalid_pipe_check"
    no("Boş akar kotu özniteliği", sewer(inverts={"from": "fields", "start": " ", "end": "x"}),
       failed(bad, "Baş akar kotunun özniteliği boş; özniteliğin adını yazın.", "inverts.start"))
    no("Abartı sıfır", sewer(exaggeration=0.0),
       failed(bad, "Düşey abartı 0'dan büyük bir sayı olmalı (yatay ölçeğin kaç katı).", "exaggeration"))
    no("Yazı boyu eksi", sewer(height=-2.0), failed(bad, "Yazı boyu 0'dan büyük bir sayı olmalı (kâğıtta mm).", "height"))
    no("Olmayan ağ", sewer(network="kanal"),
       failed("unknown_network", "“kanal” kimlikli ya da adlı ağ projede yok; Ağlar'da tanımlı bir ağı seçin.", "network"))
    no("Yol ağı", sewer(network="yollar"),
       failed("not_a_pipe_network", "“Yollar” bir yol ağı; boru hattı denetimi altyapı ağlarını denetler.", "network"))
    no("Olmayan yüzey", sewer(ground={"from": "surface", "surface": "Kazı"}),
       failed("unknown_terrain", "“Kazı” kimlikli ya da adlı yüzey projede yok.", "ground.surface"))
    no("Kurulamayan yüzey", sewer(ground={"from": "surface", "surface": "Boş"}),
       {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    no("CBS projesinde yüzey", sewer(), failed(check.NOT_MAP[0], check.NOT_MAP[1]), setup_=setup("gis"))
    no("Ağın borusu değil", sewer(start=P_UID("Q1", "yagmur")),
       failed("invalid_pipe_run", f"“{P_UID('Q1', 'yagmur')}” “Atıksu” ağının bir borusu değil.", "start"))
    no("Bitiş ağın borusu değil", sewer(end="yok"), failed("invalid_pipe_run", "“yok” “Atıksu” ağının bir borusu değil.",
                                                          "end"))
    no("Bitiş akış yönünde değil", sewer(start=P_UID("P3"), end=P_UID("P1")),
       failed("invalid_pipe_run", f"“{P_UID('P1')}” borusu akış yönünde “{P_UID('P3')}” borusunun aşağısında değil.",
              "end"))
    no("Çizilecek kot yok", sewer(start=P_UID("P7"), ground=None),
       failed("invalid_pipe_run", "Hatta çizilecek kot yok: borularda akar kotu ve zemin okunamadı (öznitelik "
                                  "adlarını ya da zemini denetleyin).", "inverts"))
    lo = profile("atiksu", P_UID("P1"), inverts="fields", diameter=SEWER_D, ground_=ZEMIN, datum=1000.0)[1]
    no("Kıyas kotu en alçak kottan yüksek", sewer(datum=1000.0),
       failed(bad, f"Kıyas kotu (datum) en alçak kottan ({lo:.2f}) yüksek olamaz.", "datum"))
    no("Kilitli katman", sewer(layerId="kilitli"),
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                              "etkinleştirin.", "layerId"))
    no("Katman grubu", sewer(layerId="grup"), {"status": "failed", "error": {"code": "not_a_layer", "message": ANY,
                                                                          "path": "layerId"}})
    no("Katman yok", sewer(layerId="yok"),
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Geçersiz beklenen sürüm", sewer(expectedRevision="on"), drainage.BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", sewer(expectedRevision="41"), drainage.CONFLICT)
    assert base_n == 257, base_n
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.pipes.profile", "commandVersion": 1,
           "title": "Hat profili: cazibeli hattın boykesiti",
           "note": "ADR 1860. Bağımsız başvuru scripts/fixtures/pipes_profile_cases.py: boru hattı denetiminin çizimi; "
                   "hat akış yönünde, değerler denetimin kurallarıyla, zemin kafesin kesişimlerinde, çizilen nesneler "
                   "kurallarıyla sayılır; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Hat profili ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
