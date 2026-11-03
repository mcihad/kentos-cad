#!/usr/bin/env python3
"""The shared cases of `road.project.query` (plan/road.md Y13.1, docs/adr/0911 §1–§2; ROAD-MOD-04, 06): a road
project's tree and how each row stands.

The drawing is road_pay_cases.py's (surfaces, alignments, profiles, templates, sample line sets, superelevations,
corridors, volumes, guardrails, culverts, walls, the pay book) with road_junction_cases.py's junctions and
road_haul_command_cases.py's mass haul diagrams. The rows, their order and states are ADR 0911 §2's rules; a source's
stamp is §1's (16 hex digits of the SHA-256 of the canonical JSON of the definition, its numbers floats; a surface's,
of its sources' digest as road_terrain_command_cases.py gathers it); what a drawing holds is its command's reference's
count (road_corridor_command_cases.py, road_guardrail_cases.py, road_culvert_cases.py, road_wall_cases.py,
road_junction_cases.py, road_profile_command_cases.py, road_section_command_cases.py); deep, the corridors, volumes and
mass haul diagrams are their references' reports. No KentOS code.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_project_cases.py [--check]
"""

import copy
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import protocol_cases  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_culvert_cases as cv  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402
import road_haul_command_cases as hc  # noqa: E402
import road_junction_cases as jc  # noqa: E402
import road_pay_cases as pay  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_terrain_command_cases as tc  # noqa: E402
import road_volume_command_cases as vc  # noqa: E402
import road_wall_cases as wv  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
NOT_ROAD = failed("not_a_road_project", "Proje ağacı yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}

WORDS = {"road.alignments.label": "Km yazıları", "road.profiles.draw": "Boykesit çizimi",
         "road.sections.draw": "Enkesit çizimi", "road.corridors.draw": "Şevli plan",
         "road.guardrails.draw": "Korkuluk çizimi", "road.culverts.draw": "Menfez çizimi",
         "road.walls.draw": "Duvar çizimi", "road.junctions.draw": "Kavşak çizimi",
         "road.terrains.contours": "Eş yükselti eğrileri"}
KIND_WORDS = {"surface": "yüzey", "alignment": "aks", "profile": "boykesit", "template": "tip kesit",
              "sections": "kesit takımı", "superelevation": "dever", "corridor": "koridor", "guardrail": "otokorkuluk",
              "culvert": "menfez", "wall": "istinat duvarı", "junction": "kavşak"}
KIND_NAMED = {"surface": "yüzeyi", "alignment": "aksı", "profile": "boykesiti", "template": "tip kesiti",
              "sections": "kesit takımı", "superelevation": "deveri", "corridor": "koridoru", "guardrail": "otokorkuluğu",
              "culvert": "menfezi", "wall": "istinat duvarı", "junction": "kavşağı"}
LISTS = {"surface": "surfaces", "alignment": "alignments", "profile": "profiles", "template": "templates",
         "sections": "sections", "superelevation": "superelevations", "corridor": "corridors",
         "guardrail": "guardrails", "culvert": "culverts", "wall": "walls", "junction": "junctions"}
# A drawing command, the kind of its main source and the input's field naming it.
MAIN = {"road.alignments.label": ("alignment", "alignment"), "road.profiles.draw": ("profile", "profile"),
        "road.sections.draw": ("sections", "set"), "road.corridors.draw": ("corridor", "corridor"),
        "road.guardrails.draw": ("guardrail", "guardrail"), "road.culverts.draw": ("culvert", "culvert"),
        "road.walls.draw": ("wall", "wall"), "road.junctions.draw": ("junction", "junction"),
        "road.terrains.contours": ("surface", "surface")}


def base(work="road", road=None):
    """road_pay_cases.py's drawing with the junctions and mass haul diagrams; `road` changes its settings."""
    s = pay.setup(work=work)
    if work == "road":
        r = s["settings"]["road"]
        r["junctions"] = jc.road_of(jc.START)["junctions"]
        r["hauls"] = copy.deepcopy(hc.START)
        for k, v in (road or {}).items():
            r[k] = v
    return s


# ── Stamps (ADR 0911 §1) ────────────────────────────────────────────────────────────────────────────────────────────

def floats(v):
    if isinstance(v, bool) or v is None or isinstance(v, str):
        return v
    if isinstance(v, (int, float)):
        return float(v)
    if isinstance(v, list):
        return [floats(x) for x in v]
    return {k: floats(x) for k, x in v.items()}


def stamp(v):
    return hashlib.sha256(protocol_cases.canonical(floats(v)).encode("utf-8")).hexdigest()[:16]


class State:
    """The drawing as the steps leave it: its settings, layers and objects, and the drawings kept."""

    def __init__(self, setup):
        self.setup = copy.deepcopy(setup)
        self.drawings = []  # {id, command, main: (kind, id), layer, sources: [(kind, id, stamp)], n, present, edited}

    @property
    def road(self):
        return self.setup["settings"].get("road") or {}

    def find(self, kind, id_):
        return next((x for x in self.road.get(LISTS[kind], []) if x["id"] == id_), None)

    def name(self, kind, id_):
        x = self.find(kind, id_)
        return x["name"] if x else id_

    def surface_digest(self, d):
        tc.LAYERS = self.setup["layers"]
        tc.UIDS = self.setup["uids"]
        tc.ENTITIES = [(e["kind"], e["layerId"], {k: v for k, v in e.items() if k not in ("kind", "id", "layerId", "attrs")})
                       for e in self.setup["entities"]]
        return tc.digest(tc.gather(d)[0])

    def stamp_now(self, kind, id_):
        x = self.find(kind, id_)
        if x is None:
            return None
        return self.surface_digest(x)[:16] if kind == "surface" else stamp(x)

    def sources(self, kind, id_, out=None):
        """The closure, depth first, each once (ADR 0911 §1)."""
        out = [] if out is None else out
        if any(k == kind and i == id_ for k, i, _ in out):
            return out
        now = self.stamp_now(kind, id_)
        if now is None:
            return out
        out.append((kind, id_, now))
        x = self.find(kind, id_)
        named = []
        if kind == "alignment" and "offsetOf" in x:
            named.append(("alignment", x["offsetOf"]["alignment"]))
        elif kind == "profile":
            named.append(("alignment", x["alignment"]))
            named += [("surface", x["surface"])] if x.get("surface") else []
        elif kind == "sections":
            named.append(("alignment", x["alignment"]))
            for k, f in (("profile", "profile"), ("template", "template"), ("surface", "surface"),
                         ("superelevation", "superelevation")):
                named += [(k, x[f])] if x.get(f) else []
        elif kind == "superelevation":
            named.append(("alignment", x["alignment"]))
        elif kind == "corridor":
            named += [("alignment", x["alignment"]), ("profile", x["profile"])]
            named += [("template", r["template"]) for r in x["regions"]]
            named.append(("surface", x["surface"]))
            named += [("superelevation", x["superelevation"])] if x.get("superelevation") else []
        elif kind in ("guardrail", "culvert", "wall"):
            named.append(("corridor", x["corridor"]))
        elif kind == "junction":
            named += [("alignment", leg["alignment"]) for leg in x["legs"]]
        for k, i in named:
            self.sources(k, i, out)
        return out

    def draw(self, command, input_, n):
        kind, field = MAIN[command]
        main = input_[field]
        ids = [d["id"] for d in self.drawings]
        did = next(f"cz-{k}" for k in range(1, 10000) if f"cz-{k}" not in ids)
        self.drawings.append({"id": did, "command": command, "main": (kind, main), "layer": input_["layerId"],
                              "sources": self.sources(kind, main), "n": n, "present": n, "edited": False})
        for k in range(n):
            self.setup["entities"].append({"kind": "point", "id": len(self.setup["entities"]) + 1, "layerId": input_["layerId"],
                                           "attrs": {}, "p": {"x": 0.0, "y": 0.0}})
        return did


# ── The tree (ADR 0911 §2) ──────────────────────────────────────────────────────────────────────────────────────────

def node(kind, id_, name, parent, state, why=(), command=None, objects=None):
    out = {"kind": kind, "id": id_, "name": name, "state": state}
    if parent is not None:
        out["parent"] = parent
    if why:
        out["why"] = list(why)
    if command is not None:
        out["command"] = command
        out["objects"] = objects
    return out


def drawing_state(st, d):
    why = []
    kind, main = d["main"]
    gone = st.stamp_now(kind, main) is None
    if gone:
        why.append(f"Çizildiği {KIND_WORDS[kind]} ({main}) projede yok.")
    stale = False
    for k, i, then in d["sources"][1 if gone else 0:]:
        now = st.stamp_now(k, i)
        if now == then:
            continue
        stale = True
        if now is None:
            why.append(f"Dayandığı {KIND_WORDS[k]} ({i}) projede yok.")
        else:
            why.append(f"“{st.name(k, i)}” {KIND_NAMED[k]} çizildiğinden beri değişti.")
    missing = d["present"] < d["n"]
    if missing:
        why.append(f"{d['n']} nesnesinden {d['n'] - d['present']} tanesi çizimde yok.")
    edited = not missing and d["edited"]
    if edited:
        why.append("Nesneleri elle değişti; yeniden üretmek bu değişiklikleri siler.")
    state = "gone" if gone else "edited" if edited else "stale" if stale else "missing" if missing else "current"
    return state, why


def tree(st, deep=False):
    road, nodes = st.road, []
    layers = {}

    def walk(ns):
        for n in ns:
            layers[n["id"]] = n["name"]
            walk(n.get("children", []))
    walk(st.setup["layers"])

    def push(n):
        nodes.append(n)
        return len(nodes) - 1

    def drawings(kind, id_, parent):
        for d in st.drawings:
            if d["main"] == (kind, id_):
                place(d, parent)

    def place(d, parent):
        state, why = drawing_state(st, d)
        push(node("drawing", d["id"], f"{WORDS[d['command']]} · {layers.get(d['layer'], d['layer'])}", parent, state, why,
                  d["command"], d["present"]))

    rev = {a["id"]: a.get("revision", 0) for a in road.get("alignments", [])}
    for s in road.get("surfaces", []):
        digest = st.surface_digest(s)
        if "built" not in s:
            state, why = "new", ["Yüzey henüz kabul edilmedi."]
        elif s["built"] == digest:
            state, why = "current", []
        else:
            state, why = "stale", ["Yüzeyin kaynakları kabul edildiğinden beri değişti."]
        if deep:
            g = tc.gather(s)[0]
            try:
                tc.build(g)
            except tc.Unbuildable as e:
                state = "unbuildable"
                why.append(f"Yüzey kurulamıyor: {tc.PROBLEM[e.code]}.")
        at = push(node("surface", s["id"], s["name"], None, state, why))
        drawings("surface", s["id"], at)
    for a in road.get("alignments", []):
        at = push(node("alignment", a["id"], a["name"], None, "current"))
        drawings("alignment", a["id"], at)

        def changed(then):
            if a.get("revision", 0) == then:
                return []
            return [f"“{a['name']}” aksı değişti ({then}. revizyona göre kurulu, aks şimdi {a.get('revision', 0)}.); "
                    "aksa yeniden bağlayın."]
        for kind, key in (("profile", "profiles"), ("sections", "sections"), ("superelevation", "superelevations")):
            for x in road.get(key, []):
                if x["alignment"] != a["id"]:
                    continue
                why = changed(x.get("alignmentRevision", 0))
                n = push(node(kind, x["id"], x["name"], at, "stale" if why else "current", why))
                if kind != "superelevation":
                    drawings(kind, x["id"], n)
        for j in road.get("junctions", []):
            if j["legs"][0]["alignment"] != a["id"]:
                continue
            why = [f"Kolunun aksı “{st.name('alignment', leg['alignment'])}” değişti; kavşağı yeniden bağlayın."
                   for leg in j["legs"] if leg["alignment"] in rev and rev[leg["alignment"]] != leg.get("alignmentRevision", 0)]
            n = push(node("junction", j["id"], j["name"], at, "stale" if why else "current", why))
            drawings("junction", j["id"], n)
    for t in road.get("templates", []):
        push(node("template", t["id"], t["name"], None, "current"))
    corridors = road.get("corridors", [])
    for c in corridors:
        why, state = [], "current"
        if rev.get(c["alignment"]) != c.get("alignmentRevision", 0):
            state = "stale"
            why.append(f"“{st.name('alignment', c['alignment'])}” aksı değişti; koridoru aksa yeniden bağlayın.")
        if deep:
            r = ccmd.report(c)
            if r.get("problem"):
                state = "unbuildable"
                why.append(f"Koridor kurulamıyor: {r['problem']}")
            elif r["unsolved"]:
                state = "unsolved" if state == "current" else state
                why.append(f"{len(r['unsolved'])} kesitte şev araziyle buluşmuyor ya da kesit çözülmüyor.")
        at = push(node("corridor", c["id"], c["name"], None, state, why))
        drawings("corridor", c["id"], at)
        for v in road.get("volumes", []):
            if v["corridor"] != c["id"]:
                continue
            state, why = ("approved", [f"{v['approved']['at']} günü onaylandı."]) if v.get("approved") else ("current", [])
            if deep:
                r = vc.report(v, corridors)
                if r.get("problem"):
                    state = "unbuildable"
                    why.append(f"Kübaj hesaplanamıyor: {r['problem']}")
                elif r.get("approval", {}).get("changed"):
                    state = "changed"
                    why.append("Kaynakları onaydan sonra değişti; fark kübaj raporunda.")
            n = push(node("volume", v["id"], v["name"], at, state, why))
            for h in road.get("hauls", []):
                if h["volume"] != v["id"]:
                    continue
                state, why = "current", []
                if deep:
                    r = hc.report(h, road.get("volumes", []), corridors)
                    if r.get("problem"):
                        state = "unbuildable"
                        why.append(f"Brükner kurulamıyor: {r['problem']}")
                    elif r.get("stale"):
                        state = "stale"
                        why.append("Kübajı değişti.")
                push(node("haul", h["id"], h["name"], n, state, why))
        for kind, key in (("guardrail", "guardrails"), ("culvert", "culverts"), ("wall", "walls")):
            for x in road.get(key, []):
                if x["corridor"] != c["id"]:
                    continue
                why = [] if c.get("revision", 0) == x.get("corridorRevision", 0) else \
                    [f"“{c['name']}” koridoru değişti; koridora yeniden bağlayın."]
                n = push(node(kind, x["id"], x["name"], at, "stale" if why else "current", why))
                drawings(kind, x["id"], n)
    if road.get("pay"):
        b = road["pay"]
        state, why = "current", []
        if deep:
            q = pay.answer(b)[0]
            if any(line["stale"] for row in q["rows"] for line in row["lines"]):
                state, why = "stale", ["Metrajın kaynaklarından biri eskidi."]
        push(node("pay", "pay", f"Poz defteri · {b['source']} {b['version']}", None, state, why))
    for d in st.drawings:
        if not any(n["kind"] == "drawing" and n["id"] == d["id"] for n in nodes):
            place(d, None)
    count = lambda f: sum(1 for n in nodes if f(n))  # noqa: E731
    return {"nodes": nodes, "stale": count(lambda n: n["state"] == "stale"),
            "problems": count(lambda n: n["state"] in ("unbuildable", "unsolved", "changed", "missing", "edited", "gone")),
            "rebuildable": count(lambda n: n["kind"] == "drawing" and n["state"] in ("stale", "missing")),
            "deep": deep, "autoRebuild": bool(road.get("autoRebuild")), "revision": "$current"}


# ── The drawings' counts (their commands' references) ───────────────────────────────────────────────────────────────

def count(st, command, input_):
    r = st.road
    find = lambda key, id_: next(x for x in r[key] if x["id"] == id_)  # noqa: E731
    if command == "road.corridors.draw":
        return ccmd.drawing(find("corridors", input_["corridor"]), input_.get("tick", 5.0))[1]
    if command == "road.guardrails.draw":
        return len(grc.drawing(find("guardrails", input_["guardrail"]), input_.get("labels", False))[1])
    if command == "road.culverts.draw":
        return len(cv.drawing(find("culverts", input_["culvert"]), input_.get("labels", False))[1])
    if command == "road.walls.draw":
        return len(wv.drawing(find("walls", input_["wall"]), input_.get("labels", False))[1])
    if command == "road.junctions.draw":
        return len(jc.drawing(find("junctions", input_["junction"]), input_.get("labels", False))[1])
    if command == "road.profiles.draw":
        return pcmd.drawing(find("profiles", input_["profile"]), tuple(input_["at"]))[1]
    if command == "road.sections.draw":
        return scmd.drawing(find("sections", input_["set"]), tuple(input_["at"]))[1]
    raise ValueError(command)


# ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────────

CORRIDOR = ("road.corridors.draw", {"corridor": "c-1", "layerId": "0"})
GUARDRAIL = ("road.guardrails.draw", {"guardrail": "o-1", "layerId": "korkuluk"})
CULVERT = ("road.culverts.draw", {"culvert": "m-1", "layerId": "korkuluk"})
WALL = ("road.walls.draw", {"wall": "d-1", "layerId": "korkuluk"})
JUNCTION = ("road.junctions.draw", {"junction": "j-1", "layerId": "korkuluk"})
PROFILE = ("road.profiles.draw", {"profile": "b-1", "layerId": "0", "at": [0.0, -200.0]})
SECTIONS = ("road.sections.draw", {"set": "k-1", "layerId": "0", "at": [0.0, -100.0]})


def drawn(st, what):
    """A step drawing `what` (command, input) on the state: any answer, the drawing kept."""
    command, input_ = what
    n = count(st, command, input_)
    st.draw(command, input_, n)
    return {**step("execute", input_, ANY), "command": command}


def other(command, input_):
    return {**step("execute", input_, ANY), "command": command}


def asked(st, deep=False):
    """The tree asked; it writes nothing (after steps that did, only the revision is said to stay)."""
    input_ = {"deep": True} if deep else {}
    return step("execute", input_, completed(tree(st, deep)), UNCHANGED)


def cases():
    out = []

    def add(name, steps, setup=None):
        if len(steps) > 1:
            steps[-1] = {**steps[-1], "expect": {"revision": "same"}}
        out.append(case(name, steps, setup))

    s0 = base()
    st = State(s0)
    add("Hiç çizim yok: kaynaklar ağaç sırasıyla, eskiyenler nedenleriyle", [asked(st)])
    st = State(s0)
    add("Plan da sorulur: değişmez, aynı cevap", [{**asked(st), "op": "plan", "expect": UNCHANGED}])
    for what, title in ((CORRIDOR, "Şevli plan koridorun altında, güncel"),
                        (GUARDRAIL, "Korkuluk çizimi korkuluğun altında"),
                        (CULVERT, "Menfez çizimi menfezin altında"),
                        (WALL, "Duvar çizimi duvarın altında"),
                        (JUNCTION, "Kavşak çizimi kavşağın altında"),
                        (PROFILE, "Boykesit çizimi boykesitin altında"),
                        (SECTIONS, "Enkesit çizimi kesit takımının altında")):
        st = State(s0)
        add(title, [drawn(st, what), asked(st)])
    # The alignment renamed: its revision goes up; what is drawn from it is stale, the profiles, sets, superelevations,
    # corridors and junctions on it too.
    st = State(s0)
    first = drawn(st, CORRIDOR)
    a = st.find("alignment", "g-1")
    a["name"], a["revision"] = "Gövde", a.get("revision", 0) + 1
    add("Aks değişince şevli plan ve aksa bağlı olanlar eskir",
        [first, other("road.alignments.edit", {"alignment": "g-1", "operation": "set", "name": "Gövde"}), asked(st)])
    st = State(s0)
    first = drawn(st, CORRIDOR)
    t = st.find("template", "tk-2")
    t["name"], t["revision"] = "Tip 2b", t.get("revision", 0) + 1
    add("Tip kesit değişince şevli plan eskir, koridor eskimez",
        [first, other("road.templates.edit", {"template": "tk-2", "operation": "set", "name": "Tip 2b"}), asked(st)])
    st = State(s0)
    first = drawn(st, GUARDRAIL)
    target = len(s0["entities"]) + 1
    st.drawings[0]["present"] -= 1
    add("Çizimin bir nesnesi silinince eksik",
        [first, other("cad.entities.delete", {"uids": [f"$uidOf:{target}"]}), asked(st)])
    st = State(s0)
    first = drawn(st, GUARDRAIL)
    st.drawings[0]["edited"] = True
    add("Çizimin nesnesi elle değişince elle değişti",
        [first, other("cad.entities.set", {"uids": [f"$uidOf:{target}"], "operation": "color", "color": "#E5484D"}), asked(st)])
    st = State(s0)
    first = drawn(st, JUNCTION)
    st.road["junctions"] = [j for j in st.road["junctions"] if j["id"] != "j-1"]
    add("Kavşak silinince çizimi kaynaksız kalır, ağacın sonunda",
        [first, other("road.junctions.edit", {"junction": "j-1", "operation": "remove"}), asked(st)])
    st = State(s0)
    first = drawn(st, CORRIDOR)
    st.drawings.clear()
    st.setup["entities"] = copy.deepcopy(s0["entities"])
    add("Geri alınan çizim kaydıyla gider", [first, {"op": "undo", "returns": "Şevli plan"}, asked(st)])
    st = State(s0)
    one, two = drawn(st, CORRIDOR), None
    second = ("road.corridors.draw", {"corridor": "Ana koridor", "layerId": "korkuluk", "tick": 10.0})
    st2_count = ccmd.drawing(st.find("corridor", "c-1"), 10.0)[1]
    st.draw("road.corridors.draw", {"corridor": "c-1", "layerId": "korkuluk"}, st2_count)
    two = {**step("execute", second[1], ANY), "command": second[0]}
    add("Aynı koridorun iki çizimi, biri adıyla", [one, two, asked(st)])
    # Surfaces.
    s1 = base()
    t1 = next(s for s in s1["settings"]["road"]["surfaces"] if s["id"] == "t-1")
    t1["built"] = State(s1).surface_digest(t1)
    add("Kabul edilen yüzey güncel", [asked(State(s1))], s1)
    s2 = base()
    t1 = next(s for s in s2["settings"]["road"]["surfaces"] if s["id"] == "t-1")
    t1["built"] = "0" * 64
    add("Kaynakları değişen yüzey eskidi", [asked(State(s2))], s2)
    # Approved volume.
    s3 = base()
    vols = s3["settings"]["road"]["volumes"]
    vols[0] = vc.approved(vols[0], corridors=grc.CORRIDORS)
    add("Onaylı kübaj onaylı", [asked(State(s3))], s3)
    s4 = base(road={"autoRebuild": True})
    add("Kendiliğinden yeniden üret açık", [asked(State(s4))], s4)
    # Deep.
    add("Derin: kurulamayan yüzey ve koridorlar, çözülmeyen kesitler, kübajlar ve Brükner'ler", [asked(State(s0), deep=True)])
    s5 = base()
    vols = s5["settings"]["road"]["volumes"]
    approved = vc.approved(vols[0], corridors=grc.CORRIDORS)
    approved["approved"]["cut"] += 100.0
    vols[0] = approved
    add("Derin: onaylı kübajın kaynakları değişti", [asked(State(s5), deep=True)], s5)
    add("Yol projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], base(work="map"))
    return out


def file():
    return {"format": "kentos.command-cases", "version": 1, "command": "road.project.query", "commandVersion": 1,
            "title": "Yol projesinin ağacı",
            "note": "scripts/fixtures/road_project_cases.py yazar; çizim road_pay_cases.py'nin, kavşaklar "
                    "road_junction_cases.py'nin, Brükner'ler road_haul_command_cases.py'nindir. Satırlar, sıraları ve "
                    "durumları ADR 0911 §2'nin kuralıdır; kaynağın damgası §1'in; çizimin nesne sayısı komutunun "
                    "başvurusunun. KentOS kodu yok.",
            "setup": base(), "cases": cases()}


def main():
    path = OUT / "road.project.query.json"
    text = json.dumps(file(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if path.read_text() != text:
            sys.exit(f"{path.name}: güncel değil")
        print(f"{path.name}: güncel")
        return
    path.write_text(text)
    print(f"{path.name}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
