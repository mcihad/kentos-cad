#!/usr/bin/env python3
"""The shared cases of `cad.pointclouds.network` (Tarama ağı dengelemesi; plan/map-cad.md MAP-13, docs/adr/4720),
without KentOS code.

    python3 scripts/fixtures/pointcloud_network_cases.py           # writes the scans and the cases
    python3 scripts/fixtures/pointcloud_network_cases.py --check   # writes nothing; compares

The scans: pieces of pointcloud_register_cases.py's ground (a slope with a mound, a ridge and a roof), sampled six
points a square metre, each moved by its own known rotation and translation about the same middle (the truth). Each
link is registered by that file's registration (its `register`, the ADR 1800/2300/2700 rules written there), and its
final normal matrix about the registration's centre is read from it. The network, by hand (docs/adr/4720):
- the centre: the scans' boxes' middles' mean;
- a link's observation: its rotation (ω, φ, κ) and its translation moved to the centre, t + (R − I)(centre − c); its
  weight Jᵀ·N·J, J the identity with [d]× (d = centre − c) in its lower left block;
- the normal equations of the free scans (N, −N blocks), solved here by numpy; a Cholesky pivot at or under 10⁻¹² of
  the largest diagonal (written here again) leaves the network undetermined;
- the loops: a breadth-first tree from the held scans (one node) over the links in order; a link off it closes a loop,
  misclosure = a's path sum + m − b's;
- each scan's matrix p ↦ R(p − centre) + centre + t, R = Rz(κ)·Ry(φ)·Rx(ω).
The truth holds them: each moved scan's adjusted matrix takes its points back to the ground within a few millimetres.
"""

import copy
import json
import math
import sys
from collections import deque
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import pointcloud_register_cases as rc  # noqa: E402  the registration, the ground, the clouds' files

OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.pointclouds.network.json"
FILES = ROOT / "fixtures" / "pointcloud" / "v1" / "files"
X0, Y0 = rc.X0, rc.Y0
SAME = {"revision": "same", "canUndo": False}


def uid(n):
    return f"0192f5c0-4720-7000-8000-{n:012x}"


def deg(v):
    return [math.radians(x) for x in v]


# Each scan: its seed, its ground piece (x0, x1, y0, y1), its misalignment (degrees; metres) about rc.MID.
SCANS = {
    "ag-a.xyz": (4720, (0.0, 24.0, 0.0, 24.0), None),
    "ag-b.xyz": (4721, (10.0, 30.0, 0.0, 24.0), (deg([0.02, -0.03, 0.05]), [0.30, -0.20, 0.06])),
    "ag-c.xyz": (4722, (2.0, 28.0, 4.0, 20.0), (deg([-0.03, 0.02, -0.04]), [-0.15, 0.25, -0.04])),
    "ag-d.xyz": (4723, (6.0, 30.0, 0.0, 16.0), (deg([0.01, 0.025, 0.03]), [0.10, 0.12, -0.05])),
}
FLAT = {
    "ag-e.xyz": (4724, (100.0, 112.0, 0.0, 15.0), None),
    "ag-f.xyz": (4725, (106.0, 120.0, 0.0, 15.0), (deg([0.0, 0.0, 0.0]), [0.20, -0.10, 0.04])),
    "ag-g.xyz": (4726, (102.0, 116.0, 0.0, 15.0), (deg([0.0, 0.0, 0.0]), [-0.10, 0.15, -0.03])),
}


def piece(seed, area, moved, flat=False):
    rnd = rc.random.Random(seed)
    x0, x1, y0, y1 = area
    rows = []
    r = rc.rot(moved[0]) if moved else None
    for _ in range(int((x1 - x0) * (y1 - y0) * 6)):
        x, y = rnd.uniform(x0, x1), rnd.uniform(y0, y1)
        z = (850.0 + 0.0 * x if flat else rc.ground(x, y)) + rnd.uniform(-0.01, 0.01)
        p = [X0 + x, Y0 + y, z]
        if moved:
            d = [p[i] - rc.MID[i] for i in range(3)]
            p = [sum(r[i][j] * d[j] for j in range(3)) + rc.MID[i] + moved[1][i] for i in range(3)]
        rows.append((round(p[0], 3), round(p[1], 3), round(p[2], 3)))
    return rows


for name, (seed, area, moved) in SCANS.items():
    rc.CLOUDS[name] = piece(seed, area, moved)
for name, (seed, area, moved) in FLAT.items():
    rc.CLOUDS[name] = piece(seed, area, moved, flat=True)
NAMES = list(SCANS) + list(FLAT)
IDS = {n: k + 1 for k, n in enumerate(NAMES)}


def setup(names=NAMES):
    ents = [rc.cloud(IDS[n], n) for n in names]
    return {"format": "kentos.document", "version": 2, "name": "Tarama ağı",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [rc.layer("0", "0"), rc.layer("tarama", "Taramalar")],
            "activeLayer": "tarama", "entities": ents, "uids": [uid(IDS[n]) for n in names],
            "styles": {"items": [], "categories": []}}


BASE = setup()


# ── The network, by hand ─────────────────────────────────────────────────────────────────────────────────────────
class Fault(Exception):
    pass


def moved_obs(out, n, centre):
    c, t, w = out["centre"], out["translation"], out["rotation"]
    mm = out["matrix"]
    r = [[mm[0], mm[1], mm[2]], [mm[4], mm[5], mm[6]], [mm[8], mm[9], mm[10]]]
    d = [centre[i] - c[i] for i in range(3)]
    rd = [sum(r[i][k] * d[k] for k in range(3)) for i in range(3)]
    m = [w[0], w[1], w[2]] + [t[i] + rd[i] - d[i] for i in range(3)]
    j = np.eye(6)
    j[3:6, 0:3] = [[0.0, -d[2], d[1]], [d[2], 0.0, -d[0]], [-d[1], d[0], 0.0]]
    return np.array(m), j.T @ np.array(n) @ j


def cholesky_ok(a):
    """The Cholesky pivots' rule: every one above 10⁻¹² of the largest diagonal."""
    n = len(a)
    if n == 0:
        return True
    big = max(abs(a[i][i]) for i in range(n))
    if not big > 0.0:
        return False
    low = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(i + 1):
            s = a[i][j] - sum(low[i][k] * low[j][k] for k in range(j))
            if i == j:
                if not s > 1e-12 * big:
                    return False
                low[i][i] = math.sqrt(s)
            else:
                low[i][j] = s / low[j][j]
    return True


def network(names, fixed, links=None, **opts):
    """The answer for scans `names` (file names), `fixed` (file names), `links` (pairs of file names or none)."""
    rows = {n: rc.CLOUDS[n] for n in names}
    boxes = {n: rc.bounds(rows[n]) for n in names}
    if links is None:
        links = []
        for i in range(len(names)):
            for k in range(i + 1, len(names)):
                p, q = boxes[names[i]], boxes[names[k]]
                if max(p[0], q[0]) < min(p[3], q[3]) and max(p[1], q[1]) < min(p[4], q[4]):
                    links.append((names[i], names[k]))
    centre = [sum((boxes[n][i] + boxes[n][i + 3]) / 2.0 for n in names) / len(names) for i in range(3)]
    obs, answers = [], []
    for a, b in links:
        out = rc.register([a, b], **opts)
        if out["pairs"] < 6:
            raise Fault(("pairs", a, b, out["pairs"]))
        m, n = moved_obs(out, rc.LAST["normal"], centre)
        obs.append((names.index(a), names.index(b), m, n))
        answers.append(out)
    held = [n in fixed for n in names]
    # The tree, links in order, from the held scans as one node.
    reached = list(held)
    total = [np.zeros(6) for _ in names]
    parent = [None] * len(names)
    on_tree = [False] * len(obs)
    queue = deque([None])
    while queue:
        node = queue.popleft()
        at = (lambda k: held[k]) if node is None else (lambda k, node=node: k == node)
        for li, (a, b, m, _) in enumerate(obs):
            if at(a) and not reached[b]:
                frm, to, stp = a, b, m
            elif at(b) and not reached[a]:
                frm, to, stp = b, a, -m
            else:
                continue
            reached[to] = True
            total[to] = total[frm] + stp
            parent[to] = frm
            on_tree[li] = True
            queue.append(to)
    if not all(reached):
        raise Fault(("unjoined", names[reached.index(False)]))

    def up(k):
        p = [k]
        while parent[k] is not None:
            k = parent[k]
            p.append(k)
        return p
    loops = []
    for li, (a, b, m, _) in enumerate(obs):
        if on_tree[li]:
            continue
        w = total[a] + m - total[b]
        ua, ub = up(a), up(b)
        common = next((i for i, k in enumerate(ua) if k in ub), None)
        if common is None:
            path = ua + ub[::-1]
        else:
            path = ua[:common + 1] + ub[:ub.index(ua[common])][::-1]
        loops.append({"scans": [uid(IDS[names[k]]) for k in path], "rotation": float(np.linalg.norm(w[:3])),
                      "translation": float(np.linalg.norm(w[3:]))})
    free = [k for k in range(len(names)) if not held[k]]
    size = 6 * len(free)
    a_ = np.zeros((size, size))
    rhs = np.zeros(size)
    for a, b, m, n in obs:
        ia = free.index(a) if a in free else None
        ib = free.index(b) if b in free else None
        nm = n @ m
        if ib is not None:
            a_[6 * ib:6 * ib + 6, 6 * ib:6 * ib + 6] += n
            rhs[6 * ib:6 * ib + 6] += nm
        if ia is not None:
            a_[6 * ia:6 * ia + 6, 6 * ia:6 * ia + 6] += n
            rhs[6 * ia:6 * ia + 6] -= nm
        if ia is not None and ib is not None:
            a_[6 * ia:6 * ia + 6, 6 * ib:6 * ib + 6] -= n
            a_[6 * ib:6 * ib + 6, 6 * ia:6 * ia + 6] -= n
    if not cholesky_ok(a_.tolist()):
        raise Fault(("undetermined",))
    sol = np.linalg.solve(a_, rhs) if size else np.zeros(0)
    inv = np.linalg.inv(a_) if size else np.zeros((0, 0))
    x = [np.zeros(6) for _ in names]
    horizontal = [None] * len(names)
    for i, k in enumerate(free):
        x[k] = sol[6 * i:6 * i + 6]
        t = inv[6 * i + 3:6 * i + 6, 6 * i + 3:6 * i + 6]
        p_, q_, d_ = t[0][0], t[0][1], t[1][1]
        plan_max = (p_ + d_) / 2.0 + math.sqrt(((p_ - d_) / 2.0) ** 2 + q_ * q_)
        horizontal[k] = math.sqrt(max(t[2][2], 0.0) / plan_max) if plan_max > 0.0 else 0.0
    scans = []
    for k, n in enumerate(names):
        r = rc.rot(list(x[k][:3]))
        rcen = [sum(r[i][j] * centre[j] for j in range(3)) for i in range(3)]
        tt = [centre[i] + x[k][3 + i] - rcen[i] for i in range(3)]
        scan = {"uid": uid(IDS[n]), "fixed": held[k], "rotation": [float(v) for v in x[k][:3]],
                "translation": [float(v) for v in x[k][3:]],
                "matrix": [r[0][0], r[0][1], r[0][2], tt[0], r[1][0], r[1][1], r[1][2], tt[1], r[2][0], r[2][1],
                           r[2][2], tt[2], 0.0, 0.0, 0.0, 1.0]}
        if horizontal[k] is not None:
            scan["horizontal"] = horizontal[k]
        scans.append(scan)
    out_links, qsum, psum = [], 0.0, 0
    for (a, b, m, n), out in zip(obs, answers):
        res = x[b] - x[a] - m
        q = float(res @ n @ res)
        qsum += q
        psum += out["pairs"]
        out_links.append({"scans": [uid(IDS[names[a]]), uid(IDS[names[b]])], "pairs": out["pairs"],
                          "overlap": out["overlap"], "rmsAfter": out["rmsAfter"], "strength": out["strength"],
                          "observed": [float(v) for v in m], "residual": [float(v) for v in res],
                          "shift": math.sqrt(max(q, 0.0) / out["pairs"])})
    shift = math.sqrt(max(qsum, 0.0) / psum) if psum else 0.0
    assert abs(shift * 1e4 % 1 - 0.5) > 1e-6, shift
    notes = [f"Tarama ağı: {len(names)} tarama ({sum(held)} sabit), {len(obs)} bağ, {len(loops)} döngü; ağın RMS "
             f"kayması {shift * 1000.0:.1f} mm.",
             "Her bağ Tarama kaydı gibi kaydedildi ve son normal matrisiyle ağırlıklandı; düzeltmeler ağın merkezi "
             "çevresinde birinci derecedendir (küçük açılar)."]
    if loops:
        notes.append("Döngünün kapanmaması bağların kayıtlarının birbirini ne kadar tutmadığını söyler; dengeleme onu "
                     "bağlara ağırlıklarıyla dağıtır.")
    for (a, b, _, _), out in zip(obs, answers):
        if out["horizontal"] < 0.1:
            notes.append(f"“{uid(IDS[names[a]])}” ile “{uid(IDS[names[b]])}” bağında geometri yatay ötelemeyi zayıf "
                         "tutuyor: bu bağ ağda düşeyde anlamlı, yatayda az.")
    for k, n in enumerate(names):
        if horizontal[k] is not None and horizontal[k] < 0.1:
            assert abs(horizontal[k] * 100 % 1 - 0.5) > 1e-6, horizontal[k]
            notes.append(f"“{uid(IDS[n])}” taramasının yatay konumunu ağ zayıf belirliyor (oran {horizontal[k]:.2f}): düz "
                         "zeminde yatay öteleme ölçülmez; düzeltmesi düşeyde anlamlıdır.")
    notes.append("Ağın mutlak konumu sabit taramalarındır; kontrol noktalarıyla Tarama kaydı'nda ayrıca denetlenir. "
                 "Taramalar yerinde durur; her birini matrisiyle Dönüşüm uygula taşır.")
    return {"centre": centre, "scans": scans, "links": out_links, "loops": loops, "shift": shift, "notes": notes}


def truth_holds(answer, names, within=0.01):
    """Each moved scan's adjusted matrix takes its moved points back to the ground within `within` metres (a point near
    its box's middle, through the scan's own misalignment and back)."""
    by = {s["uid"]: s for s in answer["scans"]}
    for n in names:
        seed, area, moved = {**SCANS, **FLAT}[n]
        if not moved:
            continue
        x, y = X0 + (area[0] + area[1]) / 2.0, Y0 + (area[2] + area[3]) / 2.0
        p = [x, y, rc.ground(x - X0, y - Y0)]
        r = rc.rot(moved[0])
        d = [p[i] - rc.MID[i] for i in range(3)]
        q = [sum(r[i][j] * d[j] for j in range(3)) + rc.MID[i] + moved[1][i] for i in range(3)]
        mm = by[uid(IDS[n])]["matrix"]
        back = [mm[4 * i] * q[0] + mm[4 * i + 1] * q[1] + mm[4 * i + 2] * q[2] + mm[4 * i + 3] for i in range(3)]
        err = math.dist(back, p)
        assert err < within, (n, err)


def ok(name, names, fixed, links=None, op="plan", within=1e-6, s=None, **opts):
    answer = network(names, fixed, links, **opts)
    inp = {"uids": [uid(IDS[n]) for n in names], "fixed": [uid(IDS[n]) for n in fixed]}
    if links is not None:
        inp["links"] = [[uid(IDS[a]), uid(IDS[b])] for a, b in links]
    inp.update(opts)
    case = {"name": name, "steps": [{"op": op, "input": inp, "result": {
        "status": "completed", "output": None if op == "validate" else answer, "warnings": []}, "expect": SAME,
        "within": within}]}
    if s is not None:
        case["setup"] = s
    return case, answer


def refused(name, inp, code, message, path_, s=None):
    case = {"name": name, "steps": [{"op": "execute", "input": inp, "result": {
        "status": "failed", "error": {"code": code, "message": message, "path": path_}}, "expect": SAME}]}
    if s is not None:
        case["setup"] = s
    return case


def cases():
    a, b, c, d = "ag-a.xyz", "ag-b.xyz", "ag-c.xyz", "ag-d.xyz"
    out = []
    huber = {"kind": "huber"}
    case, ans = ok("üç tarama, A sabit, Huber ağırlığıyla: üç bağ, bir döngü; bilinen kaymalar 1 cm içinde", [a, b, c],
                   [a], robust=huber)
    truth_holds(ans, [a, b, c])
    assert len(ans["links"]) == 3 and len(ans["loops"]) == 1, ans
    out.append(case)
    case, ans = ok("dört tarama, A sabit, Huber: altı bağ, üç döngü; bilinen kaymalar 1 cm içinde", [a, b, c, d], [a],
                   robust=huber)
    truth_holds(ans, [a, b, c, d])
    assert len(ans["links"]) == 6 and len(ans["loops"]) == 3, ans
    out.append(case)
    case, ans = ok("iki sabit tarama (A ve D): döngüler sabit taramalardan geçer", [a, b, c, d], [a, d], robust=huber)
    out.append(case)
    case, ans = ok("bağlar verilir: zincir A–B, B–C, döngü yok, artık yok", [a, b, c], [a], links=[(a, b), (b, c)],
                   robust=huber)
    assert ans["loops"] == [] and all(abs(v) < 1e-12 for l in ans["links"] for v in l["residual"]), ans
    out.append(case)
    case, ans = ok("ağırlıksız: çatı kenarındaki eşleşmeler kayıtları birkaç santimetre saptırır", [a, b, c], [a])
    truth_holds(ans, [a, b, c], within=0.03)
    out.append(case)
    out.append(ok("doğrulama", [a, b, c], [a], op="validate", robust=huber)[0])
    out.append(ok("sıra değişince aynı ağ: C, A, B", [c, a, b], [a], robust=huber)[0])
    three = {"uids": [uid(1), uid(2), uid(3)], "fixed": [uid(1)]}
    out.append(refused("iki tarama", {"uids": [uid(1), uid(2)], "fixed": [uid(1)]}, "invalid_input",
                       "Tarama ağı en az üç taramayla kurulur; iki tarama için Tarama kaydı'nı kullanın.", "uids"))
    out.append(refused("aynı tarama iki kez", {"uids": [uid(1), uid(2), f" {uid(1)}"], "fixed": [uid(1)]},
                       "invalid_input", f"“{uid(1)}” taraması iki kez verildi.", "uids[2]"))
    out.append(refused("sabit tarama yok", dict(three, fixed=[]), "invalid_input",
                       "En az bir taramayı sabit verin: ağın konumu sabit taramalarındır.", "fixed"))
    out.append(refused("hepsi sabit", dict(three, fixed=[uid(1), uid(2), uid(3)]), "invalid_input",
                       "Bütün taramalar sabit: dengelenecek tarama kalmadı.", "fixed"))
    out.append(refused("sabit tarama ağda yok", dict(three, fixed=[uid(4)]), "invalid_input",
                       f"Sabit tarama “{uid(4)}” ağın taramaları arasında yok.", "fixed[0]"))
    out.append(refused("bağ iki kez", dict(three, links=[[uid(1), uid(2)], [uid(2), uid(1)]]), "invalid_input",
                       "2. bağ iki kez verildi.", "links[1]"))
    out.append(refused("bağ taramayı kendine bağlıyor", dict(three, links=[[uid(1), uid(1)]]), "invalid_input",
                       "1. bağ bir taramayı kendine bağlıyor.", "links[0]"))
    out.append(refused("bağın taraması ağda yok", dict(three, links=[[uid(1), uid(4)]]), "invalid_input",
                       "1. bağın taraması ağın taramaları arasında yok.", "links[0]"))
    out.append(refused("C hiçbir sabit taramaya bağlanmıyor", dict(three, links=[[uid(1), uid(2)]]), "invalid_input",
                       f"“{uid(3)}” taraması bağlarla hiçbir sabit taramaya bağlanmıyor; bir bağ ekleyin ya da onu da "
                       "sabit yapın.", "links"))
    e, f, g = "ag-e.xyz", "ag-f.xyz", "ag-g.xyz"
    case, ans = ok("düz zemin: ağ yatay konumu zayıf belirler, not söyler; düşey anlamlı", [e, f, g], [e])
    assert all(s_["horizontal"] < 0.1 for s_ in ans["scans"] if not s_["fixed"]), ans["scans"]
    out.append(case)
    far = {"uids": [uid(IDS[a]), uid(IDS[b]), uid(IDS[e])], "fixed": [uid(IDS[a])],
           "links": [[uid(IDS[a]), uid(IDS[b])], [uid(IDS[b]), uid(IDS[e])]]}
    try:
        network([a, b, e], [a], links=[(a, b), (b, e)])
        raise AssertionError("a link without overlap should be refused")
    except Fault as fault:
        assert fault.args[0][0] == "pairs", fault
        pairs = fault.args[0][3]
    out.append(refused("örtüşmeyen bağ: eşleşme yok", far, "invalid_input",
                       f"“{uid(IDS[b])}” ile “{uid(IDS[e])}” taramaları arasında {pairs} eşleşme var; bir bağ için en az 6 "
                       "gerekir. Erişimi büyütün ya da bu bağı çıkarın.", "links"))
    out.append(refused("kaydın kendi reddi: voksel 0", dict(three, spacing=0.0), "invalid_input",
                       "Voksel kenarı sonlu ve sıfırdan büyük olmalı (yazılan 0).", "spacing"))
    out.append(refused("kimlik UUID değil", dict(three, uids=["x", uid(2), uid(3)]), "invalid_uid",
                       "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                       "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da "
                       "çizimden alın.", "uids[0]"))
    out.append(refused("çizimde olmayan tarama", dict(three, uids=[uid(1), uid(2), uid(99)]), "entity_not_found",
                       f"“{uid(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir "
                       "nesnenin kimliğini verin.", "uids[2]"))
    with_point = copy.deepcopy(BASE)
    with_point["entities"].append({"kind": "point", "id": 8, "layerId": "0", "attrs": {},
                                   "p": {"x": X0 + 5.0, "y": Y0 + 5.0}})
    with_point["uids"].append(uid(8))
    out.append(refused("nokta bulutu olmayan nesne", dict(three, uids=[uid(1), uid(2), uid(8)]), "not_a_pointcloud",
                       "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.", "uids[2]",
                       s=with_point))
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.pointclouds.network", "commandVersion": 1,
           "title": "Tarama ağı dengelemesi",
           "note": "ADR 4720. Bağımsız başvuru scripts/fixtures/pointcloud_network_cases.py: bağlar "
                   "pointcloud_register_cases.py'nin kaydıyla, ağ elle (normal denklemler numpy ile, Cholesky kuralı, "
                   "döngüler); bilinen kaymalar bağımsız doğrudur. KentOS kodu yok.",
           "setup": BASE, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    files = {n: rc.text(rc.CLOUDS[n]) for n in NAMES}
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text and all(
            (FILES / n).exists() and (FILES / n).read_text("utf-8") == t for n, t in files.items())
        print("Tarama ağı ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    FILES.mkdir(parents=True, exist_ok=True)
    for n, t in files.items():
        (FILES / n).write_text(t, "utf-8")
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum), {len(files)} bulut")
    return 0


if __name__ == "__main__":
    sys.exit(main())
