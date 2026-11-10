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
import road_grades  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402
import road_haul_command_cases as hc  # noqa: E402
import road_junction_cases as jc  # noqa: E402
import road_pay_cases as pay  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_terrain_command_cases as tc  # noqa: E402
import road_volume_command_cases as vc  # noqa: E402
import road_wall_cases as wv  # noqa: E402
import pipes_profile_cases as ppc  # noqa: E402
import road_terrain_zeroline_cases as zc  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
NOT_ROAD = failed("not_a_road_project", "Proje ağacı yol ve harita projesinde (KentOS Yol, KentOS Harita) olur; bu çizim "
                                       "ikisinden biri değil.")
ANY = {"status": "completed", "output": "$any", "warnings": "$any"}

WORDS = {"road.alignments.label": "Km yazıları", "road.profiles.draw": "Boykesit çizimi",
         "road.sections.draw": "Enkesit çizimi", "road.corridors.draw": "Şevli plan",
         "road.guardrails.draw": "Korkuluk çizimi", "road.culverts.draw": "Menfez çizimi",
         "road.walls.draw": "Duvar çizimi", "road.junctions.draw": "Kavşak çizimi",
         "road.terrains.contours": "Eş yükselti eğrileri", "road.terrains.zeroline": "Sıfır hattı",
         "cad.pipes.profile": "Hat profili"}
KIND_WORDS = {"surface": "yüzey", "alignment": "aks", "profile": "boykesit", "template": "tip kesit",
              "sections": "kesit takımı", "superelevation": "dever", "corridor": "koridor", "guardrail": "otokorkuluk",
              "culvert": "menfez", "wall": "istinat duvarı", "junction": "kavşak", "network": "ağ"}
KIND_NAMED = {"surface": "yüzeyi", "alignment": "aksı", "profile": "boykesiti", "template": "tip kesiti",
              "sections": "kesit takımı", "superelevation": "deveri", "corridor": "koridoru", "guardrail": "otokorkuluğu",
              "culvert": "menfezi", "wall": "istinat duvarı", "junction": "kavşağı", "network": "ağı"}
LISTS = {"surface": "surfaces", "alignment": "alignments", "profile": "profiles", "template": "templates",
         "sections": "sections", "superelevation": "superelevations", "corridor": "corridors",
         "guardrail": "guardrails", "culvert": "culverts", "wall": "walls", "junction": "junctions"}
# A drawing command, the kind of its main source and the input's field naming it.
MAIN = {"road.alignments.label": ("alignment", "alignment"), "road.profiles.draw": ("profile", "profile"),
        "road.sections.draw": ("sections", "set"), "road.corridors.draw": ("corridor", "corridor"),
        "road.guardrails.draw": ("guardrail", "guardrail"), "road.culverts.draw": ("culvert", "culvert"),
        "road.walls.draw": ("wall", "wall"), "road.junctions.draw": ("junction", "junction"),
        "road.terrains.contours": ("surface", "surface"), "road.terrains.zeroline": ("surface", "base"),
        "cad.pipes.profile": ("network", "network")}


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
        if kind == "network":
            return next((x for x in self.setup["settings"].get("networks", []) if x["id"] == id_), None)
        return next((x for x in self.road.get(LISTS[kind], []) if x["id"] == id_), None)

    def name(self, kind, id_):
        x = self.find(kind, id_)
        return x["name"] if x else id_

    def surface_digest(self, d):
        tc.LAYERS = self.setup["layers"]
        tc.UIDS = self.setup["uids"]
        tc.ENTITIES = [(e["kind"], e["layerId"], {k: v for k, v in e.items() if k not in ("kind", "id", "layerId", "attrs")})
                       for e in self.setup["entities"]]
        # The terrain cases' gather reads their drawing with the pad's objects and layer (docs/adr/3920): this project's
        # drawing stands for both.
        tc.ALL_UIDS, tc.ALL_ENTITIES, tc.PAD_LAYERS = tc.UIDS, tc.ENTITIES, tc.LAYERS
        return tc.digest(tc.gather(d)[0])

    def stamp_now(self, kind, id_):
        x = self.find(kind, id_)
        if x is None:
            return None
        if kind == "network":
            # Docs/adr/3560: the definition and the objects of its edge and junction layers; only whether it changed is
            # read here (the tree says no stamp), so the objects are taken as the steps leave them.
            layers = {l["layer"] for l in x.get("edges", []) + x.get("junctions", [])}
            return stamp({"network": x, "objects": [e for e in self.setup["entities"] if e["layerId"] in layers]})
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

    def resolve(self, kind, text):
        xs = self.setup["settings"].get("networks", []) if kind == "network" else self.road.get(LISTS[kind], [])
        x = next((x for x in xs if x["id"] == text), None) or next((x for x in xs if x["name"].lower() == text.lower()), None)
        return x["id"]

    def drawing_sources(self, command, input_):
        """A drawing's sources: its main source's closure; a zero line's comparison surface too (ADR 1720)."""
        kind, field = MAIN[command]
        out = self.sources(kind, self.resolve(kind, input_[field]))
        if command == "road.terrains.zeroline":
            self.sources("surface", self.resolve("surface", input_["comparison"]), out)
        if command == "cad.pipes.profile":
            # Docs/adr/3560: the surface its ground is read from, the network whose pipes cross it.
            if input_.get("ground", {}).get("from") == "surface":
                self.sources("surface", self.resolve("surface", input_["ground"]["surface"]), out)
            if "crossing" in input_:
                self.sources("network", self.resolve("network", input_["crossing"]["network"]), out)
        return out

    def draw(self, command, input_, n):
        kind, field = MAIN[command]
        main = self.resolve(kind, input_[field])
        ids = [d["id"] for d in self.drawings]
        did = next(f"cz-{k}" for k in range(1, 10000) if f"cz-{k}" not in ids)
        self.drawings.append({"id": did, "command": command, "main": (kind, main), "layer": input_["layerId"],
                              "sources": self.drawing_sources(command, input_), "n": n, "present": n, "edited": False})
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
    for n in st.setup["settings"].get("networks", []):
        if any(d["main"] == ("network", n["id"]) for d in st.drawings):
            at = push(node("network", n["id"], n["name"], None, "current"))
            drawings("network", n["id"], at)
    for d in st.drawings:
        if not any(n["kind"] == "drawing" and n["id"] == d["id"] for n in nodes):
            place(d, None)
    count = lambda f: sum(1 for n in nodes if f(n))  # noqa: E731
    return {"nodes": nodes, "stale": count(lambda n: n["state"] == "stale"),
            "problems": count(lambda n: n["state"] in ("unbuildable", "unsolved", "changed", "missing", "edited", "gone")),
            "rebuildable": count(lambda n: n["kind"] == "drawing" and n["state"] in ("stale", "missing")),
            "deep": deep, "autoRebuild": bool(road.get("autoRebuild")), "regional": not road.get("rebuildWhole"),
            "revision": "$current"}


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
    if command == "road.terrains.zeroline":
        # The zero lines of the two lattices (road_terrain_zeroline_cases.py's reference).
        b, c = (st.resolve("surface", input_[f]) for f in ("base", "comparison"))
        return len(zc.zero_lines(b, c)[0])
    if command == "cad.pipes.profile":
        # The run's profile as pipes_profile_cases.py counts it (an attribute the profile does not read changes nothing).
        return ppc.profile(input_["network"], input_["start"], diameter=input_.get("diameter"), ground_=input_.get("ground"))[1]
    raise ValueError(command)


# ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────────

CORRIDOR = ("road.corridors.draw", {"corridor": "c-1", "layerId": "0"})
GUARDRAIL = ("road.guardrails.draw", {"guardrail": "o-1", "layerId": "korkuluk"})
CULVERT = ("road.culverts.draw", {"culvert": "m-1", "layerId": "korkuluk"})
WALL = ("road.walls.draw", {"wall": "d-1", "layerId": "korkuluk"})
JUNCTION = ("road.junctions.draw", {"junction": "j-1", "layerId": "korkuluk"})
PROFILE = ("road.profiles.draw", {"profile": "b-1", "layerId": "0", "at": [0.0, -200.0]})
SECTIONS = ("road.sections.draw", {"set": "k-1", "layerId": "0", "at": [0.0, -100.0]})


def warns(st, command, input_):
    """What drawing `input_` warns now (the command's reference)."""
    r = st.road
    find = lambda key, id_: next(x for x in r[key] if x["id"] == id_)  # noqa: E731
    if command == "road.corridors.draw":
        return ccmd.drawing(find("corridors", input_["corridor"]), input_.get("tick", 5.0))[2]
    if command == "road.guardrails.draw":
        return grc.drawing(find("guardrails", input_["guardrail"]), input_.get("labels", False))[2]
    if command == "road.culverts.draw":
        return cv.drawing(find("culverts", input_["culvert"]), input_.get("labels", False))[2]
    if command == "road.walls.draw":
        return wv.drawing(find("walls", input_["wall"]), input_.get("labels", False))[2]
    if command == "road.junctions.draw":
        return jc.drawing(find("junctions", input_["junction"]), input_.get("labels", False))[2]
    return []


INPUTS = {}  # a drawing's id → its command and input as drawn


def rebuild(st, d):
    """Draws `d` again on the state: its objects removed, as many new, its sources' stamps now; what it says."""
    command, input_ = INPUTS[(id(st), d["id"])]
    removed = d["present"]
    n = count(st, command, input_)
    d.update(sources=st.drawing_sources(command, input_), n=n, present=n, edited=False)
    return {"drawing": d["id"], "command": command, "removed": removed, "uids": "$any"}, warns(st, command, input_)


def drawn(st, what):
    """A step drawing `what` (command, input) on the state: any answer, the drawing kept."""
    command, input_ = what
    n = count(st, command, input_)
    did = st.draw(command, input_, n)
    kind, field = MAIN[command]
    INPUTS[(id(st), did)] = (command, {**input_, field: st.resolve(kind, input_[field])})
    return {**step("execute", input_, ANY), "command": command}


def other(command, input_):
    return {**step("execute", input_, ANY), "command": command}


def asked(st, deep=False):
    """The tree asked; it writes nothing (after steps that did, only the revision is said to stay)."""
    input_ = {"deep": True} if deep else {}
    return {**step("execute", input_, completed(tree(st, deep)), UNCHANGED), "command": "road.project.query"}


# ── A map project (docs/adr/1720) ──────────────────────────────────────────────────────────────────────────────────
ZEROLINE = ("road.terrains.zeroline", {"layerId": "sifir", "base": "Düz", "comparison": "Tümsek"})


def map_setup(auto=False):
    """road_terrain_zeroline_cases.py's map project (its lattices); `auto`: drawing again by itself on."""
    s = zc.setup("map")
    if auto:
        s["settings"]["road"]["autoRebuild"] = True
    return s


# ── A map project's pipe run profile kept with its network (docs/adr/3560) ─────────────────────────────────────────
PIPE_PROFILE = ("cad.pipes.profile", {"network": "atiksu", "start": ppc.P_UID("P1"),
                                      "inverts": {"from": "fields", "start": "Akar baş", "end": "Akar son"},
                                      "diameter": {"field": "Çap", "unit": "mm"}, "ground": {"from": "surface", "surface": "Zemin"},
                                      "layerId": "profil", "at": [487000.0, 4419700.0]})


def pipes_setup(auto=False):
    """pipes_profile_cases.py's map project (the sewer and storm networks, the ground surface); `auto`: drawing again by
    itself on."""
    s = ppc.setup("map")
    if auto:
        s["settings"]["road"]["autoRebuild"] = True
    return s


def changed_pipe(st, warnings=()):
    """P1's material written (an attribute the profile does not read): the sewer network's objects change, its stamp
    too."""
    uid = ppc.P_UID("P1")
    e = next(e for e, u in zip(st.setup["entities"], st.setup["uids"]) if u == uid)
    e["attrs"] = {**e["attrs"], "Malzeme": "PVC"}
    return {**step("execute", {"uids": [uid], "attrs": {"Malzeme": "PVC"}, "operation": "attributes"},
                   {"status": "completed", "output": "$any", "warnings": list(warnings)}),
            "command": "cad.entities.set"}


def changed_height(st, warnings=()):
    """The mound's corner (FX, 0) set lower (119.4 → 119.0, still under the plane: the zero line stays) by an edit of
    its surface: the mound's stamp changes."""
    t = st.find("surface", "t-10")
    t["edits"] = [{"op": "setHeight", "x": zc.FX, "y": 0.0, "z": 119.0}]
    return {**step("execute", {"operation": "set", "surface": copy.deepcopy(t)},
                   {"status": "completed", "output": "$any", "warnings": list(warnings)}),
            "command": "road.terrains.edit"}


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
    did = st.draw("road.corridors.draw", {"corridor": "c-1", "layerId": "korkuluk"}, st2_count)
    INPUTS[(id(st), did)] = ("road.corridors.draw", {"corridor": "c-1", "layerId": "korkuluk", "tick": 10.0})
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
    add("Yol ve harita projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], base(work="architecture"))
    # A map project's tree (docs/adr/1720): its surfaces and the zero line drawn from two of them.
    sm = map_setup()
    st = State(sm)
    a = drawn(st, ZEROLINE)
    add("Harita projesi: yüzeyler ve sıfır hattı çizimi", [a, asked(st)], sm)
    st = State(sm)
    a = drawn(st, ZEROLINE)
    b = changed_height(st)
    add("Harita projesi: tümseğin bir kotu değişince sıfır hattı eskir", [a, b, asked(st)], sm)
    # Docs/adr/3560: a pipe run profile under its network; a pipe's attribute changed makes it stale.
    sp = pipes_setup()
    st = State(sp)
    add("Harita projesi: hat profili ağının altında, güncel", [drawn(st, PIPE_PROFILE), asked(st)], sp)
    st = State(sp)
    a = drawn(st, PIPE_PROFILE)
    b = changed_pipe(st)
    add("Harita projesi: ağın bir borusu değişince hat profili güncel değil", [a, b, asked(st)], sp)
    return out


def renamed_template(st, name="Tip 2b", tid="tk-2"):
    t = st.find("template", tid)
    t["name"], t["revision"] = name, t.get("revision", 0) + 1
    return other("road.templates.edit", {"template": tid, "operation": "set", "name": name})


def locked_layer(st, layer="korkuluk"):
    def walk(ns):
        for n in ns:
            if n["id"] == layer:
                n["locked"] = True
            walk(n.get("children", []))
    walk(st.setup["layers"])
    return other("cad.layers.set", {"operation": "locked", "layer": layer, "value": True})


def rebuilt(st, input_, ids=None, left=(), op="execute"):
    """The rebuild step: the drawings `ids` (absent: the stale and missing ones) drawn again on the state."""
    chosen = [d for d in st.drawings if (d["id"] in ids if ids is not None else drawing_state(st, d)[0] in ("stale", "missing"))]
    if ids is not None:
        chosen.sort(key=lambda d: ids.index(d["id"]))
    out, ws = [], []
    if op == "plan":
        snapshot = copy.deepcopy(st.drawings)
    for d in chosen:
        r, w = rebuild(st, d)
        out.append({**r, "uids": "$any" if op == "execute" else []})
        ws += w
    if op == "plan":
        st.drawings[:] = snapshot
    output = {"rebuilt": out, "revision": "$current"}
    if left:
        output["left"] = list(left)
    return step(op, input_, completed(output, ws))


def rebuild_cases():
    out = []

    def add(name, steps, setup=None):
        if len(steps) > 1 and steps[-1].get("expect") in (None, UNCHANGED):
            steps[-1] = {**steps[-1], "expect": {"revision": "same"}}
        out.append(case(name, steps, setup))

    s0 = base()
    st = State(s0)
    a = drawn(st, CORRIDOR)
    add("Güncel çizim kalır: yeniden üretilecek bir şey yok", [a, {**rebuilt(st, {}), "expect": {"revision": "same"}}])
    st = State(s0)
    a, b = drawn(st, CORRIDOR), renamed_template(st)
    r = rebuilt(st, {})
    add("Tip kesit değişince eskiyen şevli plan yeniden üretilir, ağaç güncel", [a, b, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a, b, c = drawn(st, CORRIDOR), drawn(st, GUARDRAIL), renamed_template(st)
    r = rebuilt(st, {})
    add("İki eskiyen çizim birlikte, tek adımda", [a, b, c, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a, b, c = drawn(st, CORRIDOR), drawn(st, GUARDRAIL), renamed_template(st)
    r = rebuilt(st, {"drawings": ["cz-2"]}, ["cz-2"])
    add("Yalnız adı verilen yeniden üretilir, öbürü eskimiş kalır", [a, b, c, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a = drawn(st, GUARDRAIL)
    target = len(s0["entities"]) + 1
    st.drawings[0]["present"] -= 1
    b = other("cad.entities.delete", {"uids": [f"$uidOf:{target}"]})
    r = rebuilt(st, {})
    add("Nesnesi silinen çizim yeniden üretilince bütün", [a, b, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a = drawn(st, CORRIDOR)
    r = rebuilt(st, {"drawings": ["cz-1"]}, ["cz-1"])
    add("Güncel çizim adıyla istenince yeniden üretilir", [a, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a = drawn(st, GUARDRAIL)
    st.drawings[0]["edited"] = True
    b = other("cad.entities.set", {"uids": [f"$uidOf:{target}"], "operation": "color", "color": "#E5484D"})
    c = renamed_template(st)
    left = [{"drawing": "cz-1", "why": "edited", "message": "Nesneleri elle değişti; adıyla ve edited ile yeniden üretilir."}]
    add("Elle değişen çizim hepsinde kalır, söylenir", [a, b, c, {**rebuilt(st, {}, left=left), "expect": {"revision": "same"}}])
    st = State(s0)
    a = drawn(st, GUARDRAIL)
    st.drawings[0]["edited"] = True
    b = other("cad.entities.set", {"uids": [f"$uidOf:{target}"], "operation": "color", "color": "#E5484D"})
    add("Elle değişen çizim adıyla, edited olmadan reddedilir", [a, b, step("execute", {"drawings": ["cz-1"]}, failed(
        "drawing_edited", "“cz-1” çiziminin nesneleri elle değişti; yeniden üretmek bu değişiklikleri siler. edited ile isteyin.",
        "edited"), {"revision": "same"})])
    st = State(s0)
    a = drawn(st, GUARDRAIL)
    st.drawings[0]["edited"] = True
    b = other("cad.entities.set", {"uids": [f"$uidOf:{target}"], "operation": "color", "color": "#E5484D"})
    r = rebuilt(st, {"drawings": ["cz-1"], "edited": True}, ["cz-1"])
    add("Elle değişen çizim edited ile yeniden üretilir, değişikliği gider", [a, b, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a = drawn(st, JUNCTION)
    st.road["junctions"] = [j for j in st.road["junctions"] if j["id"] != "j-1"]
    b = other("road.junctions.edit", {"junction": "j-1", "operation": "remove"})
    left = [{"drawing": "cz-1", "why": "gone", "message": "Kaynağı projede yok."}]
    add("Kaynağı silinen çizim hepsinde kalır", [a, b, {**rebuilt(st, {}, left=left), "expect": {"revision": "same"}}])
    st = State(s0)
    a = drawn(st, JUNCTION)
    b = other("road.junctions.edit", {"junction": "j-1", "operation": "remove"})
    add("Kaynağı silinen çizim adıyla reddedilir", [a, b, step("execute", {"drawings": ["cz-1"]}, failed(
        "drawing_source_gone", "“cz-1” çiziminin kaynağı projede yok; yeniden üretilemez, isterseniz nesnelerini silin.",
        "drawings"), {"revision": "same"})])
    add("Bilinmeyen çizim", [step("execute", {"drawings": ["cz-9"]}, failed(
        "unknown_drawing", "“cz-9” yol çizimi projede yok.", "drawings"), UNCHANGED)])
    st = State(s0)
    a, b = drawn(st, GUARDRAIL), renamed_template(st)
    c = locked_layer(st)
    add("Kilitlenen katmandaki çizim yeniden üretilemez, hiçbir şey yazılmaz", [a, b, c, step("execute", {}, failed(
        "layer_locked", "“Korkuluk” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
        "layerId"), {"revision": "same"})])
    st = State(s0)
    a, b = drawn(st, CORRIDOR), renamed_template(st)
    snapshot = copy.deepcopy(st.drawings)
    p = rebuilt(st, {}, op="plan")
    add("Plan: yazmadan söyler", [a, b, {**p, "expect": {"revision": "same"}}, asked(st)])
    st = State(s0)
    a, b = drawn(st, CORRIDOR), renamed_template(st)
    add("Doğrulama: yazmaz", [a, b, step("validate", {}, completed(None, warns(st, *CORRIDOR)), {"revision": "same"})])
    st = State(s0)
    a, b = drawn(st, CORRIDOR), renamed_template(st)
    before = copy.deepcopy(st.drawings)
    r = rebuilt(st, {})
    st.drawings[:] = before
    add("Geri alınınca çizim yine eskimiş", [a, b, {**r, "expect": {"revision": "changed"}},
                                             {"op": "undo", "returns": "Yeniden üret"}, asked(st)])
    st = State(s0)
    a, b = drawn(st, PROFILE), other("road.profiles.edit", {"profile": "b-1", "operation": "set", "name": "Proje kotu 2"})
    pr = st.find("profile", "b-1")
    pr["name"], pr["revision"] = "Proje kotu 2", pr.get("revision", 0) + 1
    r = rebuilt(st, {})
    add("Boykesit değişince boykesit çizimi yeniden üretilir", [a, b, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a = drawn(st, SECTIONS)
    b = renamed_template(st, "Tip 1b", st.find("sections", "k-1").get("template", "tk-1"))
    r = rebuilt(st, {})
    add("Tip kesit değişince enkesit çizimi yeniden üretilir", [a, b, {**r, "expect": {"revision": "changed"}}, asked(st)])
    st = State(s0)
    a, b, c = drawn(st, CULVERT), drawn(st, WALL), renamed_template(st)
    r = rebuilt(st, {"drawings": ["cz-2", "cz-1", "cz-2"]}, ["cz-2", "cz-1"])
    add("Adı verilenler verildiği sırayla, yinelenen bir kez", [a, b, c, {**r, "expect": {"revision": "changed"}}, asked(st)])
    add("Beklenen revizyon tutmuyor", [step("execute", {"expectedRevision": "99"}, ccmd.CONFLICT,
                                            UNCHANGED)])
    add("Revizyon sayı değil", [step("execute", {"expectedRevision": "on"}, ccmd.BAD_REVISION, UNCHANGED)])
    add("Yol ve harita projesi değil", [step("execute", {}, NOT_ROAD, UNCHANGED)], base(work="architecture"))
    # A map project's stale zero line drawn again (docs/adr/1720).
    sm = map_setup()
    st = State(sm)
    a = drawn(st, ZEROLINE)
    b = changed_height(st)
    add("Harita projesi: eskiyen sıfır hattı yeniden üretilir", [a, b, rebuilt(st, {}), {**asked(st), "expect": {"revision": "same"}}], sm)
    # Docs/adr/3560: the stale pipe run profile drawn again from its network.
    sp = pipes_setup()
    st = State(sp)
    a = drawn(st, PIPE_PROFILE)
    b = changed_pipe(st)
    add("Harita projesi: eskiyen hat profili ağından yeniden üretilir", [a, b, rebuilt(st, {}), {**asked(st), "expect": {"revision": "same"}}], sp)
    return out


def edit_cases():
    out = []

    def add(name, steps, setup=None):
        out.append(case(name, steps, setup))

    s0 = base()
    on = base(road={"autoRebuild": True})
    road_on = copy.deepcopy(on["settings"]["road"])
    edited = lambda v, r=True: completed({"autoRebuild": v, "regional": r, "revision": "$current"})  # noqa: E731
    nothing = failed("nothing_to_set", "Değişecek bir şey yok: kendiliğinden yeniden üretme zaten böyle.", "autoRebuild")
    add("Açılır", [step("execute", {"autoRebuild": True}, edited(True),
                        {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "road": road_on})])
    add("Kapanır", [step("execute", {"autoRebuild": False}, edited(False),
                         {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed",
                          "road": s0["settings"]["road"]})], on)
    add("Zaten açık", [step("execute", {"autoRebuild": True}, nothing, UNCHANGED)], on)
    add("Zaten kapalı", [step("execute", {"autoRebuild": False}, nothing, UNCHANGED)])
    add("Değer yok", [step("execute", {}, nothing, UNCHANGED)])
    # Docs/adr/4740: whether drawing again writes only what changed (the project's first choice) or draws whole.
    whole_road = {**copy.deepcopy(s0["settings"]["road"]), "rebuildWhole": True}
    whole = base(road={"rebuildWhole": True})
    add("Yalnız değişeni yaz kapanır: yeniden üretim bütün çizer",
        [step("execute", {"regional": False}, edited(False, False),
              {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "road": whole_road}),
         {"op": "undo", "returns": "Yalnız değişeni yaz",
          "expect": {"canUndo": False, "canRedo": True, "road": s0["settings"]["road"]}}])
    add("Yalnız değişeni yaz yeniden açılır", [step("execute", {"regional": True}, edited(False, True),
                                                    {"canUndo": True, "dirty": True, "revision": "changed",
                                                     "road": s0["settings"]["road"]})], whole)
    add("Zaten yalnız değişeni yazıyor",
        [step("execute", {"regional": True},
              failed("nothing_to_set", "Değişecek bir şey yok: yeniden üretim zaten böyle yazıyor.", "regional"),
              UNCHANGED)])
    add("İkisi birden tek adımda", [step("execute", {"autoRebuild": True, "regional": False}, edited(True, False),
                                         {"canUndo": True, "dirty": True, "revision": "changed",
                                          "road": {**copy.deepcopy(road_on), "rebuildWhole": True}}),
                                    {"op": "undo", "returns": "Yeniden üretim",
                                     "expect": {"canUndo": False, "canRedo": True, "road": s0["settings"]["road"]}}])
    add("Kendiliğinden açıkken yalnız değişeni yaz kapanır", [step("execute", {"autoRebuild": True, "regional": False},
                                                                  edited(True, False),
                                                                  {"canUndo": True, "dirty": True, "revision": "changed",
                                                                   "road": {**copy.deepcopy(road_on), "rebuildWhole": True}}),
                                                             {"op": "undo", "returns": "Yalnız değişeni yaz"}], on)
    add("Plan yazmaz: yalnız değişeni yaz", [step("plan", {"regional": False}, edited(False, False), UNCHANGED)])
    add("Plan yazmaz", [step("plan", {"autoRebuild": True}, edited(True), UNCHANGED)])
    add("Doğrulama yazmaz", [step("validate", {"autoRebuild": True}, completed(None), UNCHANGED)])
    add("Geri alınır", [step("execute", {"autoRebuild": True}, edited(True), {"revision": "changed"}),
                        {"op": "undo", "returns": "Kendiliğinden yeniden üret",
                         "expect": {"canUndo": False, "canRedo": True, "road": s0["settings"]["road"]}}])
    add("Beklenen revizyon tutmuyor", [step("execute", {"autoRebuild": True, "expectedRevision": "99"}, ccmd.CONFLICT, UNCHANGED)])
    add("Revizyon sayı değil", [step("execute", {"autoRebuild": True, "expectedRevision": "on"}, ccmd.BAD_REVISION, UNCHANGED)])
    add("Yol ve harita projesi değil", [step("execute", {"autoRebuild": True}, NOT_ROAD, UNCHANGED)], base(work="architecture"))

    def rebuilt_by_itself(st, names):
        return [{"code": "drawings_rebuilt", "message": f"Eskiyen yol çizimleri kendiliğinden yeniden üretildi: {', '.join(names)}."}]

    # Drawing again by itself: the rename's step draws the stale drawings again.
    st = State(on)
    a = drawn(st, CORRIDOR)
    t = st.find("template", "tk-2")
    t["name"], t["revision"] = "Tip 2b", t.get("revision", 0) + 1
    rebuild(st, st.drawings[0])
    b = {**step("execute", {"template": "tk-2", "operation": "set", "name": "Tip 2b"},
                {"status": "completed", "output": "$any", "warnings": rebuilt_by_itself(st, ["cz-1"])}), "command": "road.templates.edit"}
    add("Açıkken tip kesit değişince şevli plan aynı adımda yeniden üretilir", [a, b, {**asked(st), "expect": {"revision": "same"}}], on)
    st = State(on)
    a, b2 = drawn(st, CORRIDOR), drawn(st, GUARDRAIL)
    t = st.find("template", "tk-2")
    t["name"], t["revision"] = "Tip 2b", t.get("revision", 0) + 1
    for d in st.drawings:
        rebuild(st, d)
    c = {**step("execute", {"template": "tk-2", "operation": "set", "name": "Tip 2b"},
                {"status": "completed", "output": "$any", "warnings": rebuilt_by_itself(st, ["cz-1", "cz-2"])}),
         "command": "road.templates.edit"}
    add("Açıkken iki çizim birlikte", [a, b2, c, {**asked(st), "expect": {"revision": "same"}}], on)
    st = State(on)
    a = drawn(st, GUARDRAIL)
    target = len(on["entities"]) + 1
    st.drawings[0]["edited"] = True
    b = other("cad.entities.set", {"uids": [f"$uidOf:{target}"], "operation": "color", "color": "#E5484D"})
    t = st.find("template", "tk-2")
    t["name"], t["revision"] = "Tip 2b", t.get("revision", 0) + 1
    c = {**step("execute", {"template": "tk-2", "operation": "set", "name": "Tip 2b"},
                {"status": "completed", "output": "$any", "warnings": [{"code": "drawing_edited",
                 "message": "“cz-1” çizimi eskidi ama nesneleri elle değiştiği için kendiliğinden yeniden üretilmedi."}]}),
         "command": "road.templates.edit"}
    add("Açıkken elle değişen çizim kalır, söylenir", [a, b, c, {**asked(st), "expect": {"revision": "same"}}], on)
    st = State(on)
    a = drawn(st, GUARDRAIL)
    b = locked_layer(st)
    t = st.find("template", "tk-2")
    t["name"], t["revision"] = "Tip 2b", t.get("revision", 0) + 1
    c = {**step("execute", {"template": "tk-2", "operation": "set", "name": "Tip 2b"},
                {"status": "completed", "output": "$any", "warnings": [{"code": "drawing_not_rebuilt",
                 "message": "“cz-1” çizimi kendiliğinden yeniden üretilemedi: “Korkuluk” katmanı kilitli. Kilidi Katmanlar "
                            "panelinden açın ya da başka bir katmanı etkinleştirin."}]}), "command": "road.templates.edit"}
    add("Açıkken üretilemeyen çizim eskimiş kalır, söylenir", [a, b, c, {**asked(st), "expect": {"revision": "same"}}], on)
    st = State(on)
    a = drawn(st, CORRIDOR)
    t = st.find("template", "tk-2")
    b = {**step("execute", {"template": "tk-2", "operation": "set", "name": "Tip 2b"},
                {"status": "completed", "output": "$any", "warnings": rebuilt_by_itself(st, ["cz-1"])}), "command": "road.templates.edit"}
    add("Açıkken geri alınan değişiklik çizimi de geri alır", [a, b, {"op": "undo", "returns": "Tip kesit"},
                                                              {**asked(st), "expect": {"revision": "same"}}], on)
    st = State(s0)
    a, b = drawn(st, CORRIDOR), renamed_template(st)
    add("Kapalıyken çizim eskimiş kalır", [a, b, {**asked(st), "expect": {"revision": "same"}}])
    st = State(on)
    a = drawn(st, PROFILE)
    pr = st.find("profile", "b-1")
    pr["name"], pr["revision"] = "Proje kotu 2", pr.get("revision", 0) + 1
    rebuild(st, st.drawings[0])
    b = {**step("execute", {"profile": "b-1", "operation": "set", "name": "Proje kotu 2"},
                {"status": "completed", "output": "$any", "warnings": rebuilt_by_itself(st, ["cz-1"])}), "command": "road.profiles.edit"}
    add("Açıkken boykesit değişince boykesit çizimi yeniden üretilir", [a, b, {**asked(st), "expect": {"revision": "same"}}], on)
    st = State(on)
    a = drawn(st, GUARDRAIL)
    st.drawings[0]["present"] -= 1
    b = other("cad.entities.delete", {"uids": [f"$uidOf:{target}"]})
    add("Açıkken silinen nesne kendiliğinden geri gelmez", [a, b, {**asked(st), "expect": {"revision": "same"}}], on)
    st = State(on)
    a = drawn(st, JUNCTION)
    st.road["junctions"] = [j for j in st.road["junctions"] if j["id"] != "j-1"]
    b = {**step("execute", {"junction": "j-1", "operation": "remove"}, {"status": "completed", "output": "$any", "warnings": []}),
         "command": "road.junctions.edit"}
    add("Açıkken kaynağı silinen çizim kalır, yeniden üretilmez", [a, b, {**asked(st), "expect": {"revision": "same"}}], on)
    # A map project draws its stale zero line again by itself (docs/adr/1720).
    sm = map_setup(auto=True)
    st = State(sm)
    a = drawn(st, ZEROLINE)
    t = st.find("surface", "t-10")
    t["edits"] = [{"op": "setHeight", "x": zc.FX, "y": 0.0, "z": 119.0}]
    rebuild(st, st.drawings[0])
    b = {**step("execute", {"operation": "set", "surface": copy.deepcopy(t)},
                {"status": "completed", "output": "$any", "warnings": rebuilt_by_itself(st, ["cz-1"])}),
         "command": "road.terrains.edit"}
    add("Harita projesinde açıkken yüzey değişince sıfır hattı aynı adımda yeniden üretilir",
        [a, b, {**asked(st), "expect": {"revision": "same"}}], sm)
    # Docs/adr/3560: on, the step that changes a pipe draws the pipe run profile again.
    sp = pipes_setup(auto=True)
    st = State(sp)
    a = drawn(st, PIPE_PROFILE)
    uid = ppc.P_UID("P1")
    e = next(e for e, u in zip(st.setup["entities"], st.setup["uids"]) if u == uid)
    e["attrs"] = {**e["attrs"], "Malzeme": "PVC"}
    rebuild(st, st.drawings[0])
    b = {**step("execute", {"uids": [uid], "attrs": {"Malzeme": "PVC"}, "operation": "attributes"},
                {"status": "completed", "output": "$any", "warnings": rebuilt_by_itself(st, ["cz-1"])}),
         "command": "cad.entities.set"}
    add("Harita projesinde açıkken ağın borusu değişince hat profili aynı adımda yeniden üretilir",
        [a, b, {**asked(st), "expect": {"revision": "same"}}], sp)
    return out


def file():
    return {"format": "kentos.command-cases", "version": 1, "command": "road.project.query", "commandVersion": 1,
            "title": "Yol projesinin ağacı",
            "note": "scripts/fixtures/road_project_cases.py yazar; çizim road_pay_cases.py'nin, kavşaklar "
                    "road_junction_cases.py'nin, Brükner'ler road_haul_command_cases.py'nindir. Satırlar, sıraları ve "
                    "durumları ADR 0911 §2'nin kuralıdır; kaynağın damgası §1'in; çizimin nesne sayısı komutunun "
                    "başvurusunun. KentOS kodu yok.",
            "setup": base(), "cases": cases()}


NOTE = ("scripts/fixtures/road_project_cases.py yazar; çizim road_pay_cases.py'nin, kavşaklar road_junction_cases.py'nin, "
        "Brükner'ler road_haul_command_cases.py'nindir. Satırlar, sıraları ve durumları ADR 0911 §2'nin, yeniden üretme §3'ün "
        "kuralıdır; kaynağın damgası §1'in; çizimin nesne sayısı ve uyarıları komutunun başvurusunun. KentOS kodu yok.")


def files():
    yield "road.project.query.json", file()
    yield "road.project.rebuild.json", {"format": "kentos.command-cases", "version": 1, "command": "road.project.rebuild",
                                        "commandVersion": 1, "title": "Yol çizimlerini yeniden üret", "note": NOTE,
                                        "setup": base(), "cases": rebuild_cases()}
    yield "road.project.edit.json", {"format": "kentos.command-cases", "version": 1, "command": "road.project.edit",
                                     "commandVersion": 1, "title": "Kendiliğinden yeniden üret", "note": NOTE,
                                     "setup": base(), "cases": edit_cases()}


def main():
    for name, content in files():
        path = OUT / name
        content = road_grades.fixture(content)
        text = json.dumps(content, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv:
            if path.read_text() != text:
                sys.exit(f"{path.name}: güncel değil")
            print(f"{path.name}: güncel")
            continue
        path.write_text(text)
        print(f"{path.name}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
