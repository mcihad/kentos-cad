"""The rules of a kitchen run (“Mutfak dizisi”, docs/adr/0706, 0727), written from the ADRs without KentOS code: what
`cad.casework.run` writes for an input, the walls its legs stand on, and what `cad.casework.edit` and the drawing's
following do to a run. `casework_cases.py` builds the shared cases from it.

A run's line is its path `from`, `through` (none, one corner: an L, two: a U), `to`; each leg one straight piece of it.
Its frame: the unit direction along a leg and the unit normal into the run's side (left: the direction turned a quarter
counter-clockwise; right: clockwise). The rules, in the ADRs' words:

- a corner is a right angle turning toward the cabinets' side; `corner` blind (the default: the leg before keeps the
  corner square, the leg after starts `depth` + 5 cm from it, a 5 cm corner filler) or open (both legs stop `depth`
  from the corner, its square a filler);
- each leg's usable span: from its start (0, after a corner as above, the fridge's 0.60 at the run's start) to its end
  (its length, before an open corner `depth` less, the fridge's 0.60 at the run's end); modules numbered from `from`
  along the path, laid while they fit (a hair, 1e-9, of slack), each `module` wide unless its own width is given
  (`widths`), the sink's 0.90, the dishwasher's and an oven's 0.60; the first leg of an L or a U laid back from its
  corner (its filler at the run's start), every other from its start (its filler at its end);
- a module: a base cabinet (KentOS's “Alt dolap”, its width “Genişlik”, written only when not its 0.60 default), or the
  dishwasher (“Bulaşık makinesi”), or the oven (“Fırın”) under the cooktop, its base point the middle of its back;
  turned along its leg, mirrored on the right side; its data its number, its kind and its door's hand (Sol, Sağ, Çift
  over 0.60; none for an appliance);
- the sink (“Evye”) 5 cm in from its module's start and the line, the cooktop (“Ocak”) at its module's middle 4 cm from
  the line, the fridge (“Buzdolabı”) at the middle of its 0.60, wall cabinets (“Üst dolap”) over the modules;
- fillers: a leg's leftover over 1 mm (a closed polyline 0.60 deep), the corners' (blind: 5 cm, open: the square);
- the countertop: one area along the path from the counter's start to its end (the fridge's span left out), `depth`
  deep, its corners mitred, holes for the sink (2 cm in from the sink) and the cooktop (2 cm in from it);
- a leg stands on a wall's face when both its ends lie within 1 mm of one of the face's straight edges, between its
  ends: the run keeps the wall, the edge, the face and the ends' distances along it, and follows the wall.
"""

import math

SLACK = 1e-9
SINK_WIDTH = 0.9
SINK_INSET = 0.05
COOKTOP_INSET = 0.04
APPLIANCE = 0.6
CABINET_DEPTH = 0.6
GAP = 0.05
TIE = 1e-3
NO_FILLER = 1e-3
SINK_CUT = (0.07, 0.83, 0.07, 0.53)
COOKTOP_CUT = (0.28, 0.06, 0.54)
KIND_WORDS = {"base": "Alt dolap", "sink": "Evye dolabı", "cooktop": "Ocak dolabı", "dishwasher": "Bulaşık makinesi",
              "oven": "Fırın"}
HAND_WORDS = {"left": "Sol", "right": "Sağ", "double": "Çift"}


def num(x):
    """A number as Rust's `{}` writes an f64."""
    x = float(x)
    if x.is_integer() and abs(x) < 1e16:
        return str(int(x))
    return repr(x)


def fixed(v, d):
    t = f"{abs(v):.{d}f}"
    return ("-" + t) if v < 0 and float(t) != 0 else t


def P(x, y):
    return {"x": float(x), "y": float(y)}


def js_hypot(a, b):
    """`Math.hypot(a, b)` as the core computes it: scaled and compensated."""
    xs = [abs(a), abs(b)]
    m = max(xs)
    if m == 0.0:
        return 0.0
    total, comp = 0.0, 0.0
    for x in xs:
        q = x / m
        summand = q * q - comp
        pre = total + summand
        comp = (pre - total) - summand
        total = pre
    return math.sqrt(total) * m


def slack(x):
    return 1e-9 * max(abs(x), 1.0)


def rust_round(k):
    return math.floor(k + 0.5) if k >= 0 else -math.floor(-k + 0.5)


def number_problem(rule, v):
    """The block parameter's rule (docs/adr/0318 §1): its least, its most, its step from the least."""
    lo, hi, step = rule.get("min"), rule.get("max"), rule.get("step")
    if lo is not None and v < lo - slack(lo):
        return f"en küçük değer {num(lo)}"
    if hi is not None and v > hi + slack(hi):
        return f"en büyük değer {num(hi)}"
    if step is not None:
        start = lo if lo is not None else 0.0
        k = (v - start) / step
        if abs(k - rust_round(k)) * step > slack(v):
            return f"{num(start)} değerinden {num(step)} adımlarla"
    return None


def width_param(block):
    for p in (block.get("dynamic") or {}).get("parameters", []):
        if p.get("kind") == "linear" and p.get("name") == "Genişlik":
            return p
    return None


def default_width(p):
    return js_hypot(p["to"]["x"] - p["from"]["x"], p["to"]["y"] - p["from"]["y"])


def width_values(p, w):
    d = default_width(p)
    return {} if abs(d - w) <= slack(d) else {"Genişlik": w}


class Frame:
    def __init__(self, a, b, left):
        dx, dy = b["x"] - a["x"], b["y"] - a["y"]
        self.length = js_hypot(dx, dy)
        self.a = a
        self.u = (dx / self.length, dy / self.length) if self.length > SLACK else (0.0, 0.0)
        self.n = (-self.u[1], self.u[0]) if left else (self.u[1], -self.u[0])

    def at(self, s, t):
        return {"x": self.a["x"] + self.u[0] * s + self.n[0] * t, "y": self.a["y"] + self.u[1] * s + self.n[1] * t}

    def rotation(self):
        return math.atan2(self.u[1], self.u[0])


def band(frame, a, b, depth, left):
    pts = [frame.at(a, 0.0), frame.at(b, 0.0), frame.at(b, depth), frame.at(a, depth)]
    return pts if left else list(reversed(pts))


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


# ── Walls ─────────────────────────────────────────────────────────────────

def left_distance(w):
    off = w.get("offset", 0.0)
    return off + {"left": 0.0, "center": w["width"] / 2.0, "right": w["width"]}[w.get("justify", "center")]


def offset_path(pts, d, closed):
    """A straight path offset `d` to its left, its corners mitred (the wall's face line)."""
    n = len(pts)
    edges = n if closed else n - 1
    lines = []
    for k in range(edges):
        a, b = pts[k], pts[(k + 1) % n]
        f = Frame(a, b, True)
        lines.append((f.at(0.0, d), f.u))
    out = []
    for k in range(n):
        if not closed and k == 0:
            out.append(lines[0][0])
            continue
        if not closed and k == n - 1:
            a, u = lines[-1]
            f = Frame(pts[-2], pts[-1], True)
            out.append(f.at(f.length, d))
            continue
        (p1, u1), (p2, u2) = lines[(k - 1) % edges], lines[k % edges]
        den = u1[0] * u2[1] - u1[1] * u2[0]
        if abs(den) < 1e-12:
            out.append(p2)
            continue
        t = ((p2["x"] - p1["x"]) * u2[1] - (p2["y"] - p1["y"]) * u2[0]) / den
        out.append({"x": p1["x"] + u1[0] * t, "y": p1["y"] + u1[1] * t})
    return out


def faces(w):
    """A wall's two face lines: its left face first, its right face."""
    l = left_distance(w)
    closed = bool(w.get("closed"))
    return [("left", offset_path(w["pts"], l, closed)), ("right", offset_path(w["pts"], l - w["width"], closed))]


def along(a, b, p):
    """`p`'s distance along a→b from a, and its distance off the line."""
    f = Frame(a, b, True)
    dx, dy = p["x"] - a["x"], p["y"] - a["y"]
    return dx * f.u[0] + dy * f.u[1], abs(dx * f.n[0] + dy * f.n[1]), f.length


def tie_of(walls, start, end):
    """The face edge a leg stands on (the first wall's, its left face before its right): its wall, edge and face; None."""
    for w in walls:
        for face, line in faces(w):
            closed = bool(w.get("closed"))
            n = len(line)
            for k in range(n if closed else n - 1):
                a, b = line[k], line[(k + 1) % n]
                s0, d0, length = along(a, b, start)
                s1, d1, _ = along(a, b, end)
                if d0 <= TIE and d1 <= TIE and -TIE <= s0 <= length + TIE and -TIE <= s1 <= length + TIE:
                    return {"wall": w["uid"], "edge": k, "face": face}
    return None


def face_edge(w, face, edge):
    line = dict(faces(w))[face]
    n = len(line)
    return line[edge], line[(edge + 1) % n]


def intersect(a1, b1, a2, b2):
    u = (b1["x"] - a1["x"], b1["y"] - a1["y"])
    v = (b2["x"] - a2["x"], b2["y"] - a2["y"])
    den = u[0] * v[1] - u[1] * v[0]
    t = ((a2["x"] - a1["x"]) * v[1] - (a2["y"] - a1["y"]) * v[0]) / den
    return {"x": a1["x"] + u[0] * t, "y": a1["y"] + u[1] * t}


# ── Blocks ────────────────────────────────────────────────────────────────

UNIT_METRES = {"mm": 0.001, "cm": 0.01, "m": 1.0, "in": 0.0254, "ft": 0.3048}


def entity_box(e):
    """The box of a block's object, in its own numbers: polylines, lines, circles and arcs (the kitchen blocks')."""
    k = e["kind"]
    if k in ("polyline", "polygon"):
        xs = [p["x"] for p in e["pts"]]
        ys = [p["y"] for p in e["pts"]]
        return [min(xs), min(ys), max(xs), max(ys)]
    if k == "line":
        return [min(e["a"]["x"], e["b"]["x"]), min(e["a"]["y"], e["b"]["y"]),
                max(e["a"]["x"], e["b"]["x"]), max(e["a"]["y"], e["b"]["y"])]
    if k == "circle":
        c, r = e["c"], e["r"]
        return [c["x"] - r, c["y"] - r, c["x"] + r, c["y"] + r]
    if k == "arc":
        c, r, a0, a1 = e["c"], e["r"], e["a0"], e["a1"]
        pts = [(c["x"] + r * math.cos(a0), c["y"] + r * math.sin(a0)),
               (c["x"] + r * math.cos(a1), c["y"] + r * math.sin(a1))]
        span = (a1 - a0) % (2 * math.pi)
        for q in range(8):
            t = q * math.pi / 2
            if (t - a0) % (2 * math.pi) <= span + 1e-12:
                pts.append((c["x"] + r * math.cos(t), c["y"] + r * math.sin(t)))
        return [min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts)]
    return None


def block_box(b):
    """A block's box at the origin, scale 1, its parameters' defaults: what it draws (texts left out)."""
    out = None
    for e in b.get("entities", []):
        x = entity_box(e)
        if x is None:
            continue
        out = x if out is None else [min(out[0], x[0]), min(out[1], x[1]), max(out[2], x[2]), max(out[3], x[3])]
    return out


def unit_factor(b, unit):
    """Metres per number of a block (docs/adr/0315 §4): a unitless block's are the drawing's unit's, else metres."""
    return UNIT_METRES[unit] if b.get("units") == "unitless" else 1.0


# ── A run ─────────────────────────────────────────────────────────────────

PARTS = ("module", "sink", "cooktop", "fridge", "upper", "filler", "corner", "counter")
COUNTER_HEIGHT = 0.9
CUT_IN = 0.02
FRIDGE = 0.6


class Built:
    """What a run writes: its objects in order with their places in the run, the blocks it brings, its answer's
    fields and its legs' walls."""

    def __init__(self):
        self.members = []  # (part, index or None, entity)
        self.added = []
        self.modules = []
        self.legs = []
        self.fridge = None
        self.filler = 0.0
        self.ties = []
        self.warnings = []
        self.path = []


def leg_path(j, n):
    """The input's path of leg j's end: a corner, or `to`."""
    return f"through[{j}]" if j < n - 1 else "to"


def cut(f, a, z, t0, t1, left):
    """A hole in the countertop: `a`..`z` along the leg, `t0`..`t1` in from its line, turning against the countertop."""
    pts = [f.at(a, t0), f.at(a, t1), f.at(z, t1), f.at(z, t0)]
    return pts if left else list(reversed(pts))


def openings_on(drawing, wall_uid):
    return [o for o in drawing.get("openings", []) if o["host"] == wall_uid]


def jamb_alongs(wall, o, frame):
    """An opening's two jambs as distances along a leg's line: on a straight one-piece wall, its base line's points at
    the opening's ends, projected."""
    a, b = wall["pts"][0], wall["pts"][1]
    w = Frame(a, b, True)
    at, width = o["at"], o["width"]
    anchor = o.get("anchor", "center")
    lo = {"start": at, "center": at - width / 2.0, "end": at - width}[anchor]
    out = []
    for s in (lo, lo + width):
        p = w.at(s, 0.0)
        out.append((p["x"] - frame.a["x"]) * frame.u[0] + (p["y"] - frame.a["y"]) * frame.u[1])
    return sorted(out)


def run(drawing, inp, library):
    """The command's answer for `inp` on a drawing (`blocks`, `layers` by id, `walls`, `openings`, `unit`): (Built, None)
    or (None, refusal). Library blocks are found by name in `library` when the drawing has none of the name."""
    def invalid(words, path):
        return None, refusal("invalid_casework", words, path)

    through = list(inp.get("through") or [])
    pts = [inp["from"]] + through + [inp["to"]]
    names = [("from", "Dizinin başının")] + [(f"through[{i}]", f"{i + 1}. köşenin") for i in range(len(through))] \
        + [("to", "Dizinin sonunun")]
    for p, (path, whose) in zip(pts, names):
        for axis, name in (("x", "doğu (Y)"), ("y", "kuzey (X)")):
            if not math.isfinite(p[axis]):
                return None, refusal("not_finite", f"{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). "
                                     "Koordinatı sonlu bir sayıyla verin.", f"{path}.{axis}")
    for key, what in (("module", "Modül eni"), ("depth", "Tezgâh derinliği")):
        if inp.get(key) is not None and not math.isfinite(inp[key]):
            return None, refusal("not_finite", f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.",
                                 key)
    own = list(inp.get("perModule") or [])
    for i, m in enumerate(own):
        if m.get("width") is not None and not math.isfinite(m["width"]):
            return None, refusal("not_finite", "Modülün eni sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı "
                                 "verin.", f"perModule[{i}].width")
    left = inp.get("side", "left") == "left"
    if len(through) > 2:
        return invalid("Dizi en çok iki köşelidir (U mutfak); köşeleri azaltın.", "through")
    n = len(pts) - 1
    frames = [Frame(pts[j], pts[j + 1], left) for j in range(n)]
    for j, f in enumerate(frames):
        if not f.length > SLACK:
            if n == 1:
                return invalid("Dizinin başı ile sonu aynı nokta; dolapların sırtının dayandığı çizgiyi verin.", "to")
            return invalid(f"{j + 1}. kolun başı ile sonu aynı nokta; kolun çizgisini verin.", leg_path(j, n))
    module = inp.get("module", 0.6)
    depth = inp.get("depth", 0.6)
    if not (0.3 - 1e-9 <= depth <= 1.2 + 1e-9):
        return invalid(f"Tezgâh derinliği {num(depth)} m; 0,30 ile 1,20 m arasında olmalı.", "depth")
    for k in range(1, n):
        a, b = frames[k - 1].u, frames[k].u
        dot = a[0] * b[0] + a[1] * b[1]
        cross = a[0] * b[1] - a[1] * b[0]
        if abs(dot) > 1e-6:
            return invalid(f"{k}. köşe dik değil; L ve U dizinin kolları dik birleşir.", f"through[{k - 1}]")
        if (cross > 0) != left:
            return invalid(f"{k}. köşe dolapların öbür yanına dönüyor; köşe dolapların durduğu yana dönmeli.",
                           f"through[{k - 1}]")
    seen = set()
    for i, m in enumerate(own):
        if m["module"] < 1:
            return invalid("Modül numarası 1'den başlar.", f"perModule[{i}].module")
        if m["module"] in seen:
            return invalid(f"{m['module']}. modülün ayarı iki kez verildi.", f"perModule[{i}].module")
        seen.add(m["module"])
    b = Built()

    def definition(name):
        have = next((x for x in drawing.get("blocks", []) if x["name"] == name), None)
        if have is not None:
            return have
        x = next(x for x in library if x["name"] == name)
        if x not in b.added:
            b.added.append(x)
        return x

    cabinet = definition("Alt dolap")
    width = width_param(cabinet)
    if width is None:
        return invalid("“Alt dolap” bloğunun “Genişlik” parametresi yok; dizi modülün enini bu parametreyle verir.",
                       "module")
    rule = width.get("rule", {})
    why = number_problem(rule, module)
    if why:
        return invalid(f"Modül eni {num(module)} m; “Alt dolap” bloğunun eni için {why}.", "module")
    unit = drawing.get("unit", "m")
    cab_factor = unit_factor(cabinet, unit)
    cab_box = block_box(cabinet)
    cab_depth = cab_box[3] * cab_factor
    sink, cooktop = inp.get("sink"), inp.get("cooktop")
    dishwasher, oven, fridge = inp.get("dishwasher"), inp.get("oven") is True, inp.get("fridge")
    corner = inp.get("corner", "blind")
    given = {m["module"]: m for m in own}
    where = {m["module"]: i for i, m in enumerate(own)}
    for i, m in enumerate(own):
        if m.get("width") is not None:
            why = number_problem(rule, m["width"])
            if why:
                return invalid(f"{m['module']}. modülün eni {num(m['width'])} m; “Alt dolap” bloğunun eni için "
                               f"{why}.", f"perModule[{i}].width")

    def width_of(i):
        if i in given and given[i].get("width") is not None:
            return given[i]["width"]
        if i == dishwasher or (oven and i == cooktop):
            return APPLIANCE
        if i == sink:
            return SINK_WIDTH
        if i == cooktop:
            return max(module, APPLIANCE)
        return module

    for key, whose, w in (("sink", "Evye", SINK_WIDTH), ("cooktop", "Ocak", max(module, APPLIANCE))):
        k = inp.get(key)
        if k is not None and not (k in given and given[k].get("width") is not None) and not (key == "cooktop" and oven):
            why = number_problem(rule, w)
            if why:
                return invalid(f"{whose} modülü {num(w)} m; “Alt dolap” bloğunun eni için {why}.", key)
    spans = []
    index = 1
    for j, f in enumerate(frames):
        if j == 0:
            s0 = FRIDGE if fridge == "start" else 0.0
        else:
            s0 = cab_depth + GAP if corner == "blind" else cab_depth
        if j == n - 1:
            s1 = f.length - (FRIDGE if fridge == "end" else 0.0)
        else:
            s1 = f.length if corner == "blind" else f.length - cab_depth
        usable = s1 - s0
        laid = []
        total = 0.0
        while True:
            w = width_of(index)
            if total + w > usable + SLACK:
                break
            laid.append((index, w))
            total += w
            index += 1
        if not laid:
            need = width_of(index)
            if n == 1:
                if fridge is None:
                    return invalid(f"Dizinin çizgisi {fixed(f.length, 3)} m; bir modüle ({fixed(need, 2)} m) "
                                   "yetmiyor. Çizgiyi uzatın ya da modülü daraltın.", "to")
                return invalid(f"Buzdolabından sonra kalan {fixed(usable, 3)} m bir modüle ({fixed(need, 2)} m) "
                               "yetmiyor.", "to")
            return invalid(f"{j + 1}. kolda köşeden ve uçtan sonra kalan {fixed(usable, 3)} m bir modüle "
                           f"({fixed(need, 2)} m) yetmiyor.", leg_path(j, n))
        spans.append((j, s0, s1, laid, total))
    count = index - 1
    for key, whose in (("sink", "Evye"), ("cooktop", "Ocak"), ("dishwasher", "Bulaşık makinesi")):
        k = inp.get(key)
        if k is not None and not 1 <= k <= count:
            return invalid(f"{whose} {k}. modülde; dizi {count} modül (1 ile {count} arası).", key)
    for i, m in enumerate(own):
        if m["module"] > count:
            return invalid(f"{m['module']}. modülün ayarı verildi; dizi {count} modül.", f"perModule[{i}].module")
    if sink is not None and sink == cooktop:
        return invalid("Evye ve ocak aynı modülde olamaz; ocağı başka bir modüle koyun.", "cooktop")
    if dishwasher is not None and dishwasher in (sink, cooktop):
        return invalid("Bulaşık makinesi evyenin ya da ocağın modülüne konamaz; başka bir modül seçin.", "dishwasher")
    if oven and cooktop is None:
        return invalid("Fırın ocağın altına konur; önce ocağın modülünü verin.", "oven")
    for i, m in enumerate(own):
        k = m["module"]
        appliance = k == dishwasher or (oven and k == cooktop)
        if appliance and m.get("width") is not None and abs(m["width"] - APPLIANCE) > SLACK:
            whose = "Bulaşık makinesi" if k == dishwasher else "Fırın"
            return invalid(f"{k}. modül {whose}; eni 0,60 m'dir.", f"perModule[{i}].width")
        if m.get("hand") is not None:
            if appliance:
                return invalid(f"{k}. modül bir cihaz; açılması verilmez.", f"perModule[{i}].hand")
            if width_of(k) > APPLIANCE + SLACK:
                return invalid(f"{k}. modül çift kapılı ({num(width_of(k))} m); açılması verilmez.",
                               f"perModule[{i}].hand")
    placed = []  # (index, leg, start, width)
    fills = []   # (leg, a, z, count on it)
    for j, s0, s1, laid, total in spans:
        x = s1 - total if (j == 0 and n > 1) else s0
        gap = (s0, s1 - total) if (j == 0 and n > 1) else (s0 + total, s1)
        for i, w in laid:
            placed.append((i, j, x, w))
            x += w
        leftover = gap[1] - gap[0]
        b.legs.append({"length": frames[j].length, "start": s0, "end": s1,
                       "filler": leftover if leftover > NO_FILLER else 0.0})
        if leftover > NO_FILLER:
            fills.append((j, gap[0], gap[1], len(laid)))
    by_index = {p[0]: p for p in placed}
    fixtures = []
    for key, name, whose in (("sink", "Evye", "Evye"), ("cooktop", "Ocak", "Ocak")):
        k = inp.get(key)
        if k is None:
            continue
        blk = definition(name)
        box = block_box(blk)
        fac = unit_factor(blk, unit)
        _, j, start, w = by_index[k]
        if key == "sink":
            at_s, at_t = start + SINK_INSET, SINK_INSET
        else:
            at_s, at_t = start + w / 2.0, COOKTOP_INSET
        a0, a1 = at_s + box[0] * fac, at_s + box[2] * fac
        t0, t1 = at_t + box[1] * fac, at_t + box[3] * fac
        if a0 < start - SLACK or a1 > start + w + SLACK:
            return invalid(f"{whose} ({fixed(a1 - a0, 2)} m) {k}. modüle ({fixed(w, 2)} m) sığmıyor; modülün enini "
                           "büyütün.", key)
        if t0 < -SLACK or t1 > depth + SLACK:
            return invalid(f"{whose} {fixed(t1, 2)} m derinlik ister; tezgâh {fixed(depth, 2)} m. Tezgâhı "
                           "derinleştirin.", "depth")
        fixtures.append((key, name, blk, fac, j, at_s, at_t, (a0, a1, t0, t1)))
    upper = None
    if inp.get("upper") is True:
        ub = definition("Üst dolap")
        up = width_param(ub)
        if up is None:
            return invalid("“Üst dolap” bloğunun “Genişlik” parametresi yok; dizi modülün enini bu parametreyle verir.",
                           "upper")
        widths_seen = []
        for _, _, _, w in placed:
            if w not in widths_seen:
                widths_seen.append(w)
        for w in widths_seen:
            why = number_problem(up.get("rule", {}), w)
            if why:
                return invalid(f"Üst dolap {num(w)} m; “Üst dolap” bloğunun eni için {why}.", "upper")
        upper = (ub, up)
    lw = inp.get("lineWeight")
    if lw is not None and not (0 <= lw <= 100):
        return None, refusal("invalid_line_weight", "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince "
                             "çizgidir). Bir kalınlık ya da “Katmana göre” seçin.", "lineWeight")
    rev = inp.get("expectedRevision")
    if rev is not None and not rev.startswith("$") and not (rev.isdigit() and (rev == "0" or not rev.startswith("0"))):
        return None, refusal("invalid_revision", f"Beklenen sürüm “{rev}” geçerli bir sürüm değil; sürüm “12” gibi bir "
                             "tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
    if rev == "$stale":
        out = refusal("revision_conflict", "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu "
                      "çizimin şimdiki hâline göre yeniden hazırlayın.", "expectedRevision", "conflict")
        out["error"]["revision"] = "$current"
        return None, out
    lid = inp["layerId"]
    node = drawing["layers"].get(lid)
    if node is None:
        return None, refusal("layer_not_found", f"“{lid}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini "
                             "verin.", "layerId")
    if node["type"] != "layer":
        return None, refusal("not_a_layer", f"“{node['name']}” bir katman grubu; nesne yalnız katmana eklenir. Grubun "
                             "içinden bir katman seçin.", "layerId")
    if node["locked"]:
        return None, refusal("layer_locked", f"“{node['name']}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya "
                             "da başka bir katmanı etkinleştirin.", "layerId")
    if not node["visible"]:
        b.warnings.append({"code": "layer_hidden", "message": f"“{node['name']}” katmanı gizli; çizilen nesne "
                           "görünmeyecek.", "path": "layerId"})

    base = {"layerId": lid}
    if inp.get("color") is not None:
        base["color"] = inp["color"]
    if inp.get("lineWeight") is not None:
        base["lineWeight"] = inp["lineWeight"]

    def insert(blk, p, f, params, attrs, fac):
        e = {"kind": "insert", **base, "attrs": attrs, "block": f"$blockOf:{blk['name']}", "p": p, "scale": fac,
             "rotation": f.rotation()}
        if not left:
            e["mirror"] = True
        if params:
            e["parameters"] = params
        return e

    def scaled(p, w, fac):
        return width_values(p, w / fac)

    washer = definition("Bulaşık makinesi") if dishwasher is not None else None
    ovenb = definition("Fırın") if oven else None
    fridgeb = definition("Buzdolabı") if fridge is not None else None
    for i, j, start, w in placed:
        f = frames[j]
        if i == dishwasher:
            kind, block, hand = "dishwasher", washer, None
        elif oven and i == cooktop:
            kind, block, hand = "cooktop", ovenb, None
        else:
            kind = "sink" if i == sink else "cooktop" if i == cooktop else "base"
            block = cabinet
            hand = "double" if w > APPLIANCE + SLACK else (given.get(i, {}).get("hand") or "left")
        word = "oven" if (oven and i == cooktop) else kind
        attrs = {"Modül": str(i), "Tür": KIND_WORDS[word]}
        if hand is not None:
            attrs["Açılma"] = HAND_WORDS[hand]
        fac = unit_factor(block, unit)
        if block is cabinet:
            e = insert(block, f.at(start, 0.0), f, scaled(width, w, fac), attrs, fac)
        else:
            e = insert(block, f.at(start + w / 2.0, 0.0), f, {}, attrs, fac)
        b.members.append(("module", i, e))
        m = {"number": i, "kind": kind, "start": start, "width": w}
        if j:
            m["leg"] = j
        if hand is not None:
            m["hand"] = hand
        if oven and i == cooktop:
            m["oven"] = True
        b.modules.append(m)
    holes = []
    for key, name, blk, fac, j, at_s, at_t, (a0, a1, t0, t1) in fixtures:
        b.members.append((key, None, insert(blk, frames[j].at(at_s, at_t), frames[j], {}, {}, fac)))
        holes.append(cut(frames[j], a0 + CUT_IN, a1 - CUT_IN, t0 + CUT_IN, t1 - CUT_IN, left))
    if fridgeb is not None:
        j = 0 if fridge == "start" else n - 1
        f = frames[j]
        at = FRIDGE / 2.0 if fridge == "start" else f.length - FRIDGE / 2.0
        b.members.append(("fridge", None, insert(fridgeb, f.at(at, 0.0), f, {}, {"Tür": "Buzdolabı"},
                                                 unit_factor(fridgeb, unit))))
        b.fridge = {"start": at - FRIDGE / 2.0, "width": FRIDGE}
        if j:
            b.fridge["leg"] = j
    if upper is not None:
        ub, up = upper
        fac = unit_factor(ub, unit)
        for i, j, start, w in placed:
            b.members.append(("upper", i, insert(ub, frames[j].at(start, 0.0), frames[j], scaled(up, w, fac),
                                                 {"Modül": str(i), "Tür": "Üst dolap"}, fac)))
    for j, a, z, laid in fills:
        b.members.append(("filler", j, {"kind": "polyline", **base, "attrs": {}, "pts": band(frames[j], a, z, cab_depth,
                                                                                        left), "closed": True}))
        leftover = z - a
        b.filler += leftover
        if n == 1:
            message = f"{laid} modülden sonra dizinin sonunda {fixed(leftover, 3)} m dolgu kalıyor."
            path = "to"
        else:
            message = f"{j + 1}. kolda {fixed(leftover, 3)} m dolgu kalıyor."
            path = "from" if j == 0 else leg_path(j, n)
        b.warnings.append({"code": "casework_filler", "message": message, "path": path})
    for k in range(1, n):
        f = frames[k]
        if corner == "blind":
            ring = band(f, cab_depth, cab_depth + GAP, cab_depth, left)
        else:
            ring = band(f, 0.0, cab_depth, cab_depth, left)
        b.members.append(("corner", k, {"kind": "polyline", **base, "attrs": {}, "pts": ring, "closed": True}))
    cs = FRIDGE if fridge == "start" else 0.0
    ce = frames[-1].length - (FRIDGE if fridge == "end" else 0.0)
    outer = [frames[0].at(cs, 0.0)] + [pts[k] for k in range(1, n)] + [frames[-1].at(ce, 0.0)]
    inner = [frames[-1].at(ce, depth)]
    for k in range(n - 1, 0, -1):
        na, nb = frames[k - 1].n, frames[k].n
        q = pts[k]
        inner.append({"x": q["x"] + na[0] * depth + nb[0] * depth, "y": q["y"] + na[1] * depth + nb[1] * depth})
    inner.append(frames[0].at(cs, depth))
    ring = outer + inner
    if not left:
        ring = list(reversed(ring))
    counter = {"kind": "polygon", **base, "attrs": {}, "pts": ring}
    if holes:
        counter["holes"] = [{"pts": h} for h in holes]
    b.members.append(("counter", None, counter))
    b.ties = [tie_of(drawing.get("walls", []), pts[j], pts[j + 1]) for j in range(n)]
    for j, tie in enumerate(b.ties):
        if tie is None:
            continue
        b.legs[j]["wall"] = tie["wall"]
        wall = next(w for w in drawing["walls"] if w["uid"] == tie["wall"])
        for o in openings_on(drawing, tie["wall"]):
            form = o.get("form", "door")
            low = form == "window" and o.get("sill") is not None and o["sill"] < COUNTER_HEIGHT - SLACK
            if form not in ("door", "opening") and not low:
                continue
            lo, hi = jamb_alongs(wall, o, frames[j])
            over = min(hi, frames[j].length) - max(lo, 0.0)
            if over <= NO_FILLER:
                continue
            mark = o.get("number") or "numarasız"
            if form == "window":
                words = (f"{j + 1}. kol {mark} penceresinin önünde ({fixed(over, 3)} m); denizliği tezgâhtan alçak "
                         f"({fixed(o['sill'], 2)} m).")
            else:
                what = "kapısının" if form == "door" else "boşluğunun"
                words = f"{j + 1}. kol {mark} {what} önüne geliyor ({fixed(over, 3)} m)."
            b.warnings.append({"code": "casework_clash", "message": words, "path": leg_path(j, n) if n > 1 else "to"})
    b.path = pts
    return b, None


# ── Following the walls ─────────────────────────────────────────────────────

def line_dir(a, b):
    f = Frame(a, b, True)
    return f.u


def followed_path(recipe, ties, changed):
    """The run's new path after its walls changed (`changed`: wall uid → (before, after)), or None when a tied face
    edge is gone or two legs no longer meet: each tied leg's line its wall's face edge now, the others' as they were;
    corners where the lines meet, the free ends their legs' old lengths from them; a one-leg run's ends their old
    distances from the nearer end of the face edge."""
    pts = [recipe["from"]] + list(recipe.get("through") or []) + [recipe["to"]]
    n = len(pts) - 1
    lines = []
    for j in range(n):
        u = line_dir(pts[j], pts[j + 1])
        tie = ties[j] if j < len(ties) else None
        if tie is None or tie["wall"] not in changed:
            lines.append((pts[j], u, None))
            continue
        before, after = changed[tie["wall"]]
        if after is None:
            return None
        old = dict(faces(before))[tie["face"]]
        new = dict(faces(after))[tie["face"]]
        closed = bool(after.get("closed"))
        edges = len(new) if closed else len(new) - 1
        if tie["edge"] >= edges:
            return None
        oa, ob = old[tie["edge"]], old[(tie["edge"] + 1) % len(old)]
        na, nb = new[tie["edge"]], new[(tie["edge"] + 1) % len(new)]
        ou, nu = line_dir(oa, ob), line_dir(na, nb)
        sign = 1.0 if u[0] * ou[0] + u[1] * ou[1] >= 0 else -1.0
        lines.append((na, (sign * nu[0], sign * nu[1]), (oa, ob, na, nb)))
    if n == 1:
        p, u, edge = lines[0]
        if edge is None:
            return pts
        oa, ob, na, nb = edge
        old_len = Frame(oa, ob, True).length
        nf = Frame(na, nb, True)
        out = []
        for q in pts:
            s, _, _ = along(oa, ob, q)
            out.append(nf.at(s, 0.0) if s <= old_len / 2.0 else nf.at(nf.length - (old_len - s), 0.0))
        return out
    corners = []
    for k in range(1, n):
        (p1, u1, _), (p2, u2, _) = lines[k - 1], lines[k]
        den = u1[0] * u2[1] - u1[1] * u2[0]
        if abs(den) < 1e-12:
            return None
        a2 = {"x": p2["x"] + u2[0], "y": p2["y"] + u2[1]}
        corners.append(intersect(p1, {"x": p1["x"] + u1[0], "y": p1["y"] + u1[1]}, p2, a2))
    first_len = Frame(pts[0], pts[1], True).length
    last_len = Frame(pts[-2], pts[-1], True).length
    u0, ul = lines[0][1], lines[-1][1]
    start = {"x": corners[0]["x"] - u0[0] * first_len, "y": corners[0]["y"] - u0[1] * first_len}
    end = {"x": corners[-1]["x"] + ul[0] * last_len, "y": corners[-1]["y"] + ul[1] * last_len}
    return [start] + corners + [end]


def with_path(recipe, path):
    r = dict(recipe)
    r["from"] = path[0]
    r["to"] = path[-1]
    if len(path) > 2:
        r["through"] = path[1:-1]
    else:
        r.pop("through", None)
    return r


# ── Editing ───────────────────────────────────────────────────────────────

def module_roles(recipe):
    return {k: recipe.get(k) for k in ("sink", "cooktop", "dishwasher")}


def edited_module(recipe, module, width=None, hand=None, holds=None):
    """`cad.casework.edit`'s module edit as a new recipe: the module's own values become `width` and `hand`, and when
    `holds` is given what it holds (the sink, the cooktop, the dishwasher, or a base cabinet)."""
    r = {k: v for k, v in recipe.items() if k != "perModule"}
    own = [dict(m) for m in recipe.get("perModule") or [] if m["module"] != module]
    entry = {"module": module}
    if width is not None:
        entry["width"] = width
    if hand is not None:
        entry["hand"] = hand
    if len(entry) > 1:
        own.append(entry)
    own.sort(key=lambda m: m["module"])
    if own:
        r["perModule"] = own
    if holds is not None:
        for key in ("sink", "cooktop", "dishwasher"):
            if r.get(key) == module and key != holds:
                del r[key]
        if holds != "base":
            r[holds] = module
        if r.get("cooktop") is None:
            r.pop("oven", None)
    return r
