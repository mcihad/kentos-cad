"""The shared cases of the product command cad.ucs.set: the drawing's 2D user
coordinate system (docs/adr/0261).

    python3 scripts/fixtures/ucs_command_cases.py           # writes the file
    python3 scripts/fixtures/ucs_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.ucs.set.json. The rule is written here from
the ADR, without KentOS code: a UCS is an origin in the world and the angle
of its x axis from the world's x (east), counter-clockwise, in radians, kept
in [0, 2π); the world UCS is no setting at all. The angles the cases reach
are ones every libm gives alike (axes and diagonals: atan2 and atan of them
are exact constants), so the rule's numbers are bit for bit the command's.
The checks, their order, codes, paths and messages are written by hand from
the contract (crates/shared/contracts/src/cad_ucs.rs).

`--check` rebuilds the file in memory and compares it with the one on disk.
"""
import json
import math
import sys

TAU = 2.0 * math.pi
FAR = 1.0e7
SAME = 1.0e-6
MOST_NAMED = 100

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def P(x, y):
    return {"x": x, "y": y}


E0, N0 = 487000, 4420000

ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "attrs": {}, "a": P(E0, N0), "b": P(E0 + 20, N0)},
    {"kind": "polygon", "id": 2, "layerId": "yapi", "attrs": {"Parsel": "7"}, "pts": [P(E0 + 30, N0), P(E0 + 50, N0), P(E0 + 50, N0 + 20), P(E0 + 30, N0 + 20)],
     "holes": [{"pts": [P(E0 + 35, N0 + 5), P(E0 + 45, N0 + 5), P(E0 + 45, N0 + 15), P(E0 + 35, N0 + 15)]}]},
    {"kind": "point", "id": 3, "layerId": "yapi", "attrs": {}, "p": P(E0 + 60, N0)},
    {"kind": "line", "id": 4, "layerId": "yol", "attrs": {}, "a": P(E0, N0 + 30), "b": P(E0, N0 + 50)},
    {"kind": "arc", "id": 5, "layerId": "yol", "attrs": {}, "c": P(E0 + 70, N0 + 30), "r": 5, "a0": 0, "a1": math.pi / 2},
    {"kind": "circle", "id": 6, "layerId": "yol", "attrs": {}, "c": P(E0 + 80, N0 + 30), "r": 3},
    {"kind": "text", "id": 7, "layerId": "yapi", "attrs": {}, "p": P(E0, N0 + 10), "text": "Not", "height": 2.5, "rotation": 90},
    {"kind": "spline", "id": 8, "layerId": "yapi", "attrs": {}, "pts": [P(E0, N0 + 60), P(E0 + 5, N0 + 65), P(E0 + 10, N0 + 60)], "closed": False},
    {"kind": "polyline", "id": 9, "layerId": "yol", "attrs": {}, "pts": [P(E0 + 100, N0), P(E0 + 110, N0), P(E0 + 110, N0 + 10)], "bulges": [1, 0, 0]},
    {"kind": "ellipse", "id": 10, "layerId": "yol", "attrs": {}, "c": P(E0 + 120, N0), "major": P(-6, -6), "ratio": 0.5, "t0": 0, "t1": 0},
    {"kind": "line", "id": 11, "layerId": "kilitli", "attrs": {}, "a": P(E0, N0 + 80), "b": P(E0, N0 + 70)},
    {"kind": "hatch", "id": 12, "layerId": "yapi", "attrs": {}, "ring": [P(E0 + 130, N0), P(E0 + 140, N0), P(E0 + 140, N0 + 10)],
     "pattern": {"type": "solid", "angle": 0, "spacing": 1}},
]
LAYERS = [
    layer("yapi", "Yapı"),
    layer("yol", "Yol"),
    layer("kilitli", "Kilitli katman", locked=True),
]
SETTINGS = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 1000, "workspace": "cad", "drawingFont": "barlow"}


def setup(settings=SETTINGS):
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Ürün komutu",
        "settings": settings,
        "origin": P(E0, N0),
        "layers": LAYERS,
        "activeLayer": "yapi",
        "entities": ENTITIES,
        "styles": {"items": [], "categories": []},
    }


SETUP = setup()
IDS = [e["id"] for e in ENTITIES]

# ── The rule (docs/adr/0261 §1–§3) ─────────────────────────────────────


def norm(a):
    """An angle in [0, 2π): fmod's remainder, a negative one turned once."""
    r = math.fmod(a, TAU)
    if r < 0.0:
        r += TAU
    if r >= TAU:
        r = 0.0
    return r + 0.0


def frame(origin, angle):
    return {"origin": P(origin[0], origin[1]), "angle": norm(angle)}


WORLD = {"origin": P(0, 0), "angle": 0.0}


def is_world(u):
    return u["origin"]["x"] == 0 and u["origin"]["y"] == 0 and u["angle"] == 0


def dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def xy(p):
    return (p["x"], p["y"])


def seg_distance(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
    t = max(0.0, min(1.0, t))
    return dist(p, (a[0] + t * dx, a[1] + t * dy))


def arc_distance(p, a, b, bulge):
    """The distance from p to the arc of a bulged edge: the circle when p's
    direction from the centre falls within the sweep, else the nearer end."""
    cx, cy = b[0] - a[0], b[1] - a[1]
    length = math.hypot(cx, cy)
    off = (length / 2) * (1 - bulge * bulge) / (2 * bulge)
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    n = (-cy / length, cx / length)
    c = (m[0] + n[0] * off, m[1] + n[1] * off)
    r = dist(c, a)
    sweep = 4 * math.atan(bulge)
    start = math.atan2(a[1] - c[1], a[0] - c[0])
    at = math.atan2(p[1] - c[1], p[0] - c[0])
    turn = (at - start) % TAU if sweep > 0 else (start - at) % TAU
    if turn <= abs(sweep):
        return abs(dist(c, p) - r)
    return min(dist(p, a), dist(p, b))


def rings(e):
    """The paths of an object, each (points, bulges, closed), in the ADR's order."""
    k = e["kind"]
    if k == "polygon":
        out = [(e["pts"], e.get("bulges"), True)]
        out += [(h["pts"], h.get("bulges"), True) for h in e.get("holes", [])]
        for part in e.get("parts", []):
            out.append((part["pts"], part.get("bulges"), True))
            out += [(h["pts"], h.get("bulges"), True) for h in part.get("holes", [])]
        return out
    if k == "polyline":
        out = [(e["pts"], e.get("bulges"), bool(e.get("closed")))]
        out += [(part["pts"], part.get("bulges"), False) for part in e.get("parts", [])]
        return out
    if k == "hatch":
        return [(e["ring"], None, True)] + [(h, None, True) for h in e.get("holes", [])]
    return []


def edge_frame(e, at):
    """The nearest edge's near end and the x axis along it (ADR §3)."""
    best = None
    for pts, bulges, closed in rings(e):
        n = len(pts)
        count = n if closed else n - 1
        for i in range(count):
            a, b = xy(pts[i]), xy(pts[(i + 1) % n])
            bulge = (bulges or [0] * n)[i] if bulges else 0
            d = arc_distance(at, a, b, bulge) if bulge else seg_distance(at, a, b)
            if best is None or d < best[0]:
                best = (d, a, b, bulge)
    _, a, b, bulge = best
    chord = math.atan2(b[1] - a[1], b[0] - a[0])
    half = 2 * math.atan(bulge) if bulge else 0.0
    if dist(at, a) <= dist(at, b):
        return frame(a, chord - half)
    return frame(b, chord + math.pi + half)


def object_frame(e, at, current):
    k = e["kind"]
    if k == "line":
        a, b = xy(e["a"]), xy(e["b"])
        if dist(at, a) <= dist(at, b):
            return frame(a, math.atan2(b[1] - a[1], b[0] - a[0]))
        return frame(b, math.atan2(a[1] - b[1], a[0] - b[0]))
    if k in ("polyline", "polygon", "hatch"):
        return edge_frame(e, at)
    if k == "arc":
        c = xy(e["c"])
        ends = [(e["a0"], (c[0] + e["r"] * math.cos(e["a0"]), c[1] + e["r"] * math.sin(e["a0"]))),
                (e["a1"], (c[0] + e["r"] * math.cos(e["a1"]), c[1] + e["r"] * math.sin(e["a1"])))]
        angle = ends[0][0] if dist(at, ends[0][1]) <= dist(at, ends[1][1]) else ends[1][0]
        return frame(c, angle)
    if k == "circle":
        c = xy(e["c"])
        return frame(c, math.atan2(at[1] - c[1], at[0] - c[0]))
    if k == "ellipse":
        return frame(xy(e["c"]), math.atan2(e["major"]["y"], e["major"]["x"]))
    if k == "text":
        return frame(xy(e["p"]), (e["rotation"] * math.pi) / 180.0)
    if k == "point":
        return frame(xy(e["p"]), current["angle"])
    return None


def by_id(i):
    return next(e for e in ENTITIES if e["id"] == i)


def named_of(u, named):
    return next((n["name"] for n in named if n["origin"] == u["origin"] and n["angle"] == u["angle"]), None)


def out_ucs(u, named):
    o = {"origin": u["origin"], "angle": u["angle"]}
    name = named_of(u, named)
    if name is not None:
        o["name"] = name
    return o


def setting(u):
    return None if is_world(u) else u


# ── Results ───────────────────────────────────────────────────────────


def done(u, named, changed, warnings=()):
    return {"status": "completed", "output": {"ucs": out_ucs(u, named), "named": [n["name"] for n in named], "changed": changed, "revision": "$current"}, "warnings": list(warnings)}


def after(u, named, changed):
    """What the drawing holds after a write: the settings, and a settings change's marks."""
    expect = {"ids": IDS, "settings": {"ucs": setting(u), "namedUcs": named or None}, "canUndo": False, "canRedo": False}
    if changed:
        expect.update({"dirty": True, "revision": "changed"})
    else:
        # Nothing written: the revision stays (the drawing may be dirty from a step before).
        expect.update({"revision": "same"})
    return expect


def failed(code, message, path=None):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
NON_FINITE = "UCS'in sayıları sonlu olmalı; NaN ya da sonsuz bir değer verildi. Sayıyı yeniden verin."
FAR_TEXT = "UCS'in noktası çizimden çok uzak: koordinatlar en çok 10 000 km olur. Çizimdeki bir noktayı verin."
NAME_TEXT = "UCS adı boş olamaz, en çok 64 karakterdir ve denetim karakteri içermez. Başka bir ad yazın."
SAME_TEXT = "İki nokta aynı yerde; x ekseninin yönü bulunamadı. İkinci noktayı başlangıçtan uzakta verin."
TOO_MANY = "Projede en çok 100 adlı UCS olur. Kullanmadığınız birini silin ya da var olanın üzerine kaydedin."
UID_MESSAGE = "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
MISSING = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"
MISSING_MESSAGE = f"“{MISSING}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
REVISION = "Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."


def unknown(name):
    return failed("unknown_ucs", f"“{name}” adlı bir UCS yok. Adlı UCS'lerden birini seçin ya da geçerli UCS'i bu adla kaydedin.", "to.name")


def no_axis(word):
    return failed("object_without_axis", f"{word} nesnesinden UCS kurulamaz. Çizgi, çoklu çizgi, alan, tarama, yay, daire, elips, yazı, blok ya da nokta seçin.", "to.uid")


def replaced(name):
    return {"code": "ucs_replaced", "message": f"“{name}” adlı UCS'in yerine geçerli UCS kaydedildi.", "path": "to.name"}


def uid(i):
    return f"$uidOf:{i}"


def run(to, **more):
    return {"op": "execute", "input": {"to": to, **more}}


class Drawing:
    """The settings the cases go through, by the ADR's rule."""

    def __init__(self, named=None):
        self.ucs = dict(WORLD)
        self.named = list(named or [])

    def step(self, to, result=None, expect=None, **more):
        """A written step: the rule's new UCS and names, and the step's result and expectation."""
        old = (self.ucs, list(self.named))
        warnings = []
        k = to["kind"]
        if k == "world":
            self.ucs = dict(WORLD)
        elif k == "frame":
            self.ucs = frame(xy(to["origin"]), to["angle"])
        elif k == "origin":
            self.ucs = frame(xy(to["point"]), self.ucs["angle"])
        elif k == "rotate":
            self.ucs = frame(xy(self.ucs["origin"]), self.ucs["angle"] + to["angle"])
        elif k == "twoPoints":
            o, x = xy(to["origin"]), xy(to["xPoint"])
            self.ucs = frame(o, math.atan2(x[1] - o[1], x[0] - o[0]))
        elif k == "object":
            i = int(to["uid"].split(":")[1])
            self.ucs = object_frame(by_id(i), xy(to["at"]), self.ucs)
        elif k == "named":
            n = next(n for n in self.named if n["name"] == to["name"].strip())
            self.ucs = {"origin": n["origin"], "angle": n["angle"]}
        elif k == "save":
            name = to["name"].strip()
            entry = {"name": name, **self.ucs}
            at = next((j for j, n in enumerate(self.named) if n["name"] == name), None)
            if at is None:
                self.named.append(entry)
            else:
                self.named[at] = entry
                warnings.append(replaced(name))
        elif k == "delete":
            self.named = [n for n in self.named if n["name"] != to["name"].strip()]
        changed = (setting(self.ucs), self.named) != (setting(old[0]), old[1])
        step = run(to, **more)
        # Copies: the steps after this one change the drawing's lists, not this step's answer.
        u, named = json.loads(json.dumps(self.ucs)), json.loads(json.dumps(self.named))
        step["result"] = result or done(u, named, changed, warnings)
        step["expect"] = expect or after(u, named, changed)
        return step


cases = []

# ── Setting it ─────────────────────────────────────────────────────────

d = Drawing()
s1 = d.step({"kind": "twoPoints", "origin": P(E0, N0), "xPoint": P(E0 + 10, N0 + 10)})
assert d.ucs["angle"] == math.pi / 4
s2 = d.step({"kind": "twoPoints", "origin": P(E0, N0), "xPoint": P(E0 + 3, N0 + 3)})
cases.append({
    "name": "İki nokta: başlangıç ilk nokta, x ekseni ikinciye (45°); proje ayarıdır, geri alma adımı değildir; aynı UCS'e yeniden geçmek bir şey değiştirmez",
    "steps": [s1, s2],
})

d = Drawing()
steps = [d.step({"kind": "frame", "origin": P(E0 + 100, N0 + 50), "angle": 1.0}), d.step({"kind": "world"}), d.step({"kind": "world"})]
assert d.ucs == WORLD
cases.append({
    "name": "Çerçeve verildiği gibi; Dünya UCS'i ayardan kaldırır (yoksa dünya); zaten dünyadayken bir şey değişmez",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "frame", "origin": P(E0, N0), "angle": -math.pi / 2}), d.step({"kind": "frame", "origin": P(E0, N0), "angle": 7.0})]
cases.append({
    "name": "açı [0, 2π) aralığına indirgenerek saklanır: −π/2 3π/2, 7 7 − 2π olur",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "frame", "origin": P(0, 0), "angle": TAU})]
assert d.ucs == WORLD
cases.append({
    "name": "başlangıcı (0, 0), açısı 2π olan çerçeve dünyadır: ayar yazılmaz",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "frame", "origin": P(E0, N0), "angle": 1.0}), d.step({"kind": "origin", "point": P(E0 + 5, N0 + 7)})]
assert d.ucs["angle"] == 1.0
cases.append({
    "name": "Orijin: başlangıç taşınır, eksenlerin açısı kalır",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "rotate", "angle": math.pi / 2}), d.step({"kind": "rotate", "angle": math.pi / 2}), d.step({"kind": "rotate", "angle": math.pi})]
assert d.ucs == WORLD
cases.append({
    "name": "Döndür: açı eskisine eklenir, başlangıç kalır; tam tur dünyaya döner",
    "steps": steps,
})

# ── Aligned to an object (docs/adr/0261 §3) ──────────────────────────

d = Drawing()
steps = [d.step({"kind": "object", "uid": uid(1), "at": P(E0 + 18, N0 + 1)}), d.step({"kind": "object", "uid": uid(1), "at": P(E0 + 2, N0 - 1)})]
cases.append({
    "name": "Nesne, çizgi: başlangıç tıklanan yere yakın uç, x ekseni öbür uca (B'den A'ya 180°, A'dan B'ye 0°)",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "object", "uid": uid(2), "at": P(E0 + 48, N0 + 21)}), d.step({"kind": "object", "uid": uid(2), "at": P(E0 + 37, N0 + 6)})]
assert d.ucs["origin"] == P(E0 + 35, N0 + 5) and d.ucs["angle"] == 0.0
cases.append({
    "name": "Nesne, alan: tıklanan yere en yakın kenar, onun yakın ucu başlangıç, x ekseni kenar boyunca; deliğin kenarı da",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "object", "uid": uid(9), "at": P(E0 + 103, N0 - 4.5)}), d.step({"kind": "object", "uid": uid(9), "at": P(E0 + 109, N0 - 0.5)})]
assert d.ucs["origin"] == P(E0 + 110, N0) and d.ucs["angle"] == norm(-math.pi / 2)
cases.append({
    "name": "Nesne, çoklu çizginin yaylı kenarı: x ekseni yayın o uçtaki teğeti (yarım çemberin başında aşağı, sonunda yay boyunca geri dönerken de aşağı: 270°)",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "object", "uid": uid(5), "at": P(E0 + 74, N0 + 32)}), d.step({"kind": "object", "uid": uid(5), "at": P(E0 + 71, N0 + 36)})]
cases.append({
    "name": "Nesne, yay: başlangıç merkez, x ekseni tıklanan yere yakın uca (0°, 90°)",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "object", "uid": uid(6), "at": P(E0 + 80, N0 + 34)}), d.step({"kind": "object", "uid": uid(10), "at": P(E0 + 125, N0)})]
cases.append({
    "name": "Nesne, daire: başlangıç merkez, x ekseni tıklanan yere (90°); elips: büyük eksen (225°)",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "object", "uid": uid(7), "at": P(E0, N0 + 11)}), d.step({"kind": "object", "uid": uid(12), "at": P(E0 + 139, N0 + 4)})]
cases.append({
    "name": "Nesne, yazı: yerleştirme noktası ve dönüşü (90°); tarama: halkasının en yakın kenarı",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "rotate", "angle": math.pi / 4}), d.step({"kind": "object", "uid": uid(3), "at": P(E0 + 60, N0 + 1)})]
assert d.ucs["angle"] == math.pi / 4
cases.append({
    "name": "Nesne, nokta: başlangıç nokta, eksenlerin açısı geçerli UCS'inki",
    "steps": steps,
})

d = Drawing()
steps = [d.step({"kind": "object", "uid": uid(11), "at": P(E0, N0 + 79)})]
cases.append({
    "name": "kilitli katmandaki nesneye hizalanmak serbesttir: nesne değişmez, yalnız okunur",
    "steps": steps,
})

cases.append({
    "name": "Nesne, eğri: eksen yok, ret; olmayan ve geçersiz kimlik",
    "steps": [
        {**run({"kind": "object", "uid": uid(8), "at": P(E0 + 5, N0 + 64)}), "result": no_axis("Eğri"), "expect": NOTHING},
        {**run({"kind": "object", "uid": MISSING, "at": P(E0, N0)}), "result": failed("entity_not_found", MISSING_MESSAGE, "to.uid"), "expect": NOTHING},
        {**run({"kind": "object", "uid": "duvar", "at": P(E0, N0)}), "result": failed("invalid_uid", UID_MESSAGE.format("duvar"), "to.uid"), "expect": NOTHING},
    ],
})

# ── Named ──────────────────────────────────────────────────────────────

d = Drawing()
steps = [
    d.step({"kind": "twoPoints", "origin": P(E0, N0), "xPoint": P(E0, N0 + 10)}),
    d.step({"kind": "save", "name": "  Bina  "}),
    d.step({"kind": "world"}),
    d.step({"kind": "named", "name": "Bina"}),
]
assert d.named[0]["name"] == "Bina" and d.ucs["angle"] == math.pi / 2
cases.append({
    "name": "Kaydet ve Adlı UCS: ad kırpılır; geçerli UCS adlı birine eşitse çıktı adını söyler; dünyadan geri yüklenir",
    "steps": steps,
})

d = Drawing()
steps = [
    d.step({"kind": "save", "name": "Dünya"}),
    d.step({"kind": "rotate", "angle": math.pi}),
    d.step({"kind": "save", "name": "Ters"}),
    d.step({"kind": "rotate", "angle": math.pi / 2}),
    d.step({"kind": "save", "name": "Dünya"}),
]
assert [n["name"] for n in d.named] == ["Dünya", "Ters"] and d.named[0]["angle"] == norm(1.5 * math.pi)
cases.append({
    "name": "Kaydet aynı adla geçerli UCS'i öbürünün yerine koyar (uyarıyla), sırası kalır; dünya da bir ad alabilir",
    "steps": steps,
})

d = Drawing()
steps = [
    d.step({"kind": "rotate", "angle": math.pi}),
    d.step({"kind": "save", "name": "Ters"}),
    d.step({"kind": "delete", "name": "Ters"}),
]
# After the writes before them the drawing is dirty: a refusal only leaves the revision as it was.
LATER = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same"}
steps.append({**run({"kind": "delete", "name": "Ters"}), "result": unknown("Ters"), "expect": LATER})
steps.append({**run({"kind": "named", "name": "Yok"}), "result": unknown("Yok"), "expect": LATER})
assert d.ucs["angle"] == math.pi and d.named == []
cases.append({
    "name": "Sil adlı UCS'i siler, geçerli UCS kalır; olmayan ad reddedilir",
    "steps": steps,
})

cases.append({
    "name": "geçersiz ad: boş, 64 karakterden uzun, denetim karakterli",
    "steps": [
        {**run({"kind": "save", "name": "   "}), "result": failed("invalid_ucs_name", NAME_TEXT, "to.name"), "expect": NOTHING},
        {**run({"kind": "save", "name": "A" * 65}), "result": failed("invalid_ucs_name", NAME_TEXT, "to.name"), "expect": NOTHING},
        {**run({"kind": "save", "name": "A\tB"}), "result": failed("invalid_ucs_name", NAME_TEXT, "to.name"), "expect": NOTHING},
        {**run({"kind": "named", "name": ""}), "result": failed("invalid_ucs_name", NAME_TEXT, "to.name"), "expect": NOTHING},
    ],
})

MANY = [{"name": f"U{k}", "origin": P(E0 + k, N0), "angle": 0.0} for k in range(MOST_NAMED)]
d = Drawing(MANY)
steps = [{**run({"kind": "save", "name": "Yeni"}), "result": failed("too_many_ucs", TOO_MANY, "to.name"), "expect": NOTHING}]
steps.append(d.step({"kind": "save", "name": "U7"}))
cases.append({
    "name": "100 adlı UCS varken yeni ad reddedilir; var olanın üzerine kaydetmek serbesttir",
    "setup": setup({**SETTINGS, "namedUcs": MANY}),
    "steps": steps,
})

# ── Refusals ───────────────────────────────────────────────────────────

cases.append({
    "name": "İki nokta aynı yerde (1 µm'den yakın): x ekseninin yönü yok",
    "steps": [
        {**run({"kind": "twoPoints", "origin": P(E0, N0), "xPoint": P(E0, N0 + 5e-7)}), "result": failed("same_points", SAME_TEXT, "to.xPoint"), "expect": NOTHING},
    ],
})

cases.append({
    "name": "10 000 km'den uzak nokta reddedilir; sonlu olmayan sayı her şeyden önce",
    "steps": [
        {**run({"kind": "frame", "origin": P(2e7, 0), "angle": 0}), "result": failed("far_point", FAR_TEXT, "to.origin"), "expect": NOTHING},
        {**run({"kind": "origin", "point": P(0, -1.5e7)}), "result": failed("far_point", FAR_TEXT, "to.point"), "expect": NOTHING},
        {**run({"kind": "object", "uid": uid(1), "at": P(3e7, 0)}), "result": failed("far_point", FAR_TEXT, "to.at"), "expect": NOTHING},
        {**run({"kind": "frame", "origin": P(2e7, 0), "angle": 0}), "nonFinite": {"to.angle": "NaN"}, "result": failed("not_finite", NON_FINITE, "to.angle"), "expect": NOTHING},
        {**run({"kind": "rotate", "angle": 1}), "nonFinite": {"to.angle": "Infinity"}, "result": failed("not_finite", NON_FINITE, "to.angle"), "expect": NOTHING},
        {**run({"kind": "twoPoints", "origin": P(E0, N0), "xPoint": P(E0, N0)}), "nonFinite": {"to.xPoint.x": "NaN"}, "result": failed("not_finite", NON_FINITE, "to.xPoint"), "expect": NOTHING},
    ],
})

d = Drawing()
cases.append({
    "name": "beklenen sürüm: geçersiz yazı ve eskisi reddedilir; doğrusu yazar",
    "steps": [
        {"op": "captureRevision", "as": "r0"},
        {**run({"kind": "rotate", "angle": 1}, expectedRevision="07"), "result": failed("invalid_revision", REVISION, "expectedRevision"), "expect": NOTHING},
        d.step({"kind": "rotate", "angle": 1}, expectedRevision="$r0"),
        {**run({"kind": "rotate", "angle": 1}, expectedRevision="$r0"), "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}}, "expect": {**NOTHING, "dirty": True}},
    ],
})


# ── The file ─────────────────────────────────────────────────────────


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def write(command, title, note, cases):
    out = ["{"]
    head = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title, "note": note}
    for key, value in head.items():
        out.append(f'  "{key}": {compact(value)},')
    s = SETUP
    out.append('  "setup": {')
    for key in ["format", "version", "name", "settings", "origin"]:
        out.append(f'    "{key}": {compact(s[key])},')
    out.append('    "layers": [')
    out.append(",\n".join(f"      {compact(item)}" for item in s["layers"]))
    out.append("    ],")
    out.append(f'    "activeLayer": {compact(s["activeLayer"])},')
    out.append('    "entities": [')
    out.append(",\n".join(f"      {compact(e)}" for e in s["entities"]))
    out.append("    ],")
    out.append(f'    "styles": {compact(s["styles"])}')
    out.append("  },")
    out.append('  "cases": [')
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},']
        if "note" in c:
            lines.append(f'      "note": {compact(c["note"])},')
        if "setup" in c:
            lines.append(f'      "setup": {compact(c["setup"])},')
        lines.append('      "steps": [')
        lines.append(",\n".join(f"        {compact(st)}" for st in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    text = "\n".join(out) + "\n"
    path = f"fixtures/commands/v1/{command}.json"
    if "--check" in sys.argv[1:]:
        with open(path, encoding="utf-8") as f:
            if f.read() != text:
                sys.exit(f"{path} is not what this script writes: run it without --check and read the difference.")
        return
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


write(
    "cad.ucs.set",
    "Kullanıcı koordinat sistemi",
    "ADR 0261 §1–§3: UCS dünyada bir başlangıç ve x ekseninin doğudan saat yönünün tersine açısıdır (radyan, [0, 2π)); dünya UCS'i "
    "ayar değildir. Dünya, Çerçeve, Orijin, Döndür, İki nokta, Nesne, Adlı, Kaydet ve Sil; proje ayarıdır, geri alma adımı değildir, "
    "nesneler değişmez. Kural ve beklenenler scripts/fixtures/ucs_command_cases.py'de KentOS kodu olmadan yazılmıştır.",
    cases,
)
print(f"{len(cases)} cases" + (" match" if "--check" in sys.argv[1:] else " written"))
