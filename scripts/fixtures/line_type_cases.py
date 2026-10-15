"""Writes the shared cases of line type patterns (docs/adr/0313 §1–§3): fixtures/linetypes/v1/cases.json.

- `lin`: LIN files read as AutoCAD's documentation writes them (the "Custom Linetypes" pages, without its files):
  a header `*NAME[,description]`, then `A,` and up to 12 descriptors: a dash (positive length), a gap (negative), a
  dot (0), or a complex element in brackets, `["text",STYLE,S=…,R=…|A=…|U=…,X=…,Y=…]` or `[SHAPE,file.shx,…]`, placed
  where the pattern has come to (it takes no length of its own). Comments start with `;;`; lines are trimmed; names
  are kept as written and compared without case. Each type its elements, its unit (`drawing`) and its source file;
  what does not read is said with its line.
- `scale`: an element's length in the drawing, metres: a paper pattern's millimetres times the drawing's scale over
  1000, a drawing pattern's lengths in the project's unit (its ratio to the metre); times the project's line type scale (LTSCALE) and the
  object's (CELTSCALE). The pattern's period is the sum of the lengths' sizes.
- `written`: the types written back as LIN, lengths in the shortest form that reads back.

Written from the rules above, without KentOS code. The formats core (`formats::lin`) and the contracts run them.

    python3 scripts/fixtures/line_type_cases.py           # writes the file
    python3 scripts/fixtures/line_type_cases.py --check   # compares it with the one on disk
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/linetypes/v1/cases.json"
MAX_ELEMENTS = 12
# A project unit in metres as a ratio of whole numbers (a value times mul over div), as the contract writes it.
UNIT_M = {"mm": (1.0, 1000.0), "cm": (1.0, 100.0), "m": (1.0, 1.0), "in": (254.0, 10000.0), "ft": (3048.0, 10000.0)}


def number(text):
    """A LIN number: `.5`, `-0.25`, `1e-2`; none for anything else."""
    t = text.strip()
    if not re.fullmatch(r"[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?", t):
        return None
    return float(t)


def angle(text):
    """A rotation in degrees: `0`, `90d`, `1.5708r`, `100g`."""
    t = text.strip().lower()
    k = 1.0
    if t.endswith("d"):
        t = t[:-1]
    elif t.endswith("r"):
        t, k = t[:-1], 180.0 / 3.141592653589793
    elif t.endswith("g"):
        t, k = t[:-1], 0.9
    v = number(t)
    return None if v is None else v * k


def split_top(body):
    """The descriptors: split at commas outside brackets and quotes."""
    out, cur, depth, quoted = [], "", 0, False
    for c in body:
        if c == '"':
            quoted = not quoted
        elif c == "[" and not quoted:
            depth += 1
        elif c == "]" and not quoted:
            depth -= 1
        if c == "," and depth == 0 and not quoted:
            out.append(cur)
            cur = ""
            continue
        cur += c
    out.append(cur)
    return out, depth == 0 and not quoted


def complex_element(text):
    """A bracketed element: (element, None) or (None, why)."""
    inner = text.strip()[1:-1]
    parts, _ = split_top(inner)
    parts = [p.strip() for p in parts]
    if len(parts) < 2:
        return None, "karmaşık öğe en az iki parça ister (yazı ve stil ya da şekil ve dosya)"
    head, second, rest = parts[0], parts[1], parts[2:]
    out = {"scale": 1.0, "rotation": 0.0, "absolute": False, "x": 0.0, "y": 0.0}
    for r in rest:
        if "=" not in r:
            return None, f"“{r}” bir dönüşüm değil (S=, R=, A=, U=, X=, Y=)"
        k, v = (s.strip() for s in r.split("=", 1))
        k = k.upper()
        if k == "S":
            val = number(v)
            if val is None or val <= 0:
                return None, f"ölçek “{v}” 0'dan büyük bir sayı değil"
            out["scale"] = val
        elif k in ("R", "A", "U"):
            val = angle(v)
            if val is None:
                return None, f"açı “{v}” okunmuyor"
            out["rotation"] = val
            out["absolute"] = k == "A"
            if k == "U":
                out["upright"] = True
        elif k in ("X", "Y"):
            val = number(v)
            if val is None:
                return None, f"kayma “{v}” bir sayı değil"
            out[k.lower()] = val
        else:
            return None, f"“{k}” bilinen bir dönüşüm değil (S, R, A, U, X, Y)"
    if head.startswith('"'):
        if not head.endswith('"') or len(head) < 2:
            return None, "yazı tırnakla kapanmıyor"
        return {"length": 0.0, "text": {"text": head[1:-1], "style": second, **out}}, None
    return {"length": 0.0, "shape": {"name": head, "file": second, **out}}, None


def read_lin(text, source):
    types, problems, header = [], [], None
    for i, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith(";;"):
            continue
        if line.startswith("*"):
            if header is not None:
                problems.append({"line": header[2], "message": f"“{header[0]}” çizgi tipinin deseni yok (A, satırı)"})
            name, _, desc = line[1:].partition(",")
            name = name.strip()
            if not name:
                problems.append({"line": i, "message": "çizgi tipinin adı boş"})
                header = None
                continue
            header = (name, desc.strip(), i)
            continue
        if header is None:
            problems.append({"line": i, "message": "desen bir *AD satırının altında olmalı"})
            continue
        name, desc, _ = header
        header = None
        parts, closed = split_top(line)
        if not closed:
            problems.append({"line": i, "message": f"“{name}”: köşeli parantez ya da tırnak kapanmıyor"})
            continue
        if parts[0].strip().upper() != "A":
            problems.append({"line": i, "message": f"“{name}”: desen A ile başlamalı (hizalama)"})
            continue
        elements, why = [], None
        for p in parts[1:]:
            p = p.strip()
            if p.startswith("["):
                e, why = complex_element(p)
                if e is None:
                    break
                elements.append(e)
            else:
                v = number(p)
                if v is None:
                    why = f"“{p}” bir uzunluk değil"
                    break
                elements.append({"length": v})
        if why is not None:
            problems.append({"line": i, "message": f"“{name}”: {why}"})
            continue
        if len(elements) > MAX_ELEMENTS:
            problems.append({"line": i, "message": f"“{name}”: desen en çok {MAX_ELEMENTS} öğe olur, {len(elements)} var"})
            continue
        lengths = [e["length"] for e in elements if "text" not in e and "shape" not in e]
        if not lengths or lengths[0] < 0:
            problems.append({"line": i, "message": f"“{name}”: desen bir çizgi ya da noktayla başlamalı"})
            continue
        if any(t["name"].lower() == name.lower() for t in types):
            problems.append({"line": i, "message": f"“{name}” dosyada iki kez var; ilki alındı"})
            continue
        t = {"name": name, "elements": elements, "unit": "drawing", "source": source}
        if desc:
            t["description"] = desc
        types.append(t)
    if header is not None:
        problems.append({"line": header[2], "message": f"“{header[0]}” çizgi tipinin deseni yok (A, satırı)"})
    return {"types": types, "problems": problems}


def lin_case(name, text, source="deneme.lin"):
    return {"name": name, "text": text, "source": source, "expect": read_lin(text, source)}


def lin_cases():
    return [
        lin_case("Basit kesikli", "*KESIK,Kesikli __ __ __\nA,.5,-.25\n"),
        lin_case("Nokta ve çizgi, açıklamasız", "*NOKTAKESIK\nA,1,-.25,0,-.25\n"),
        lin_case("Yorumlar ve boş satırlar atlanır", ";; KentOS deneme dosyası\n\n*KESIK,Kesikli\n;; ara\nA,.5,-.25\n"),
        lin_case("Birden çok tip, sırasıyla", "*A1,Birinci\nA,1,-1\n*B2,İkinci\nA,2,-1,0,-1\n"),
        lin_case("Yazılı tip (göreli dönüş)", '*GAZ,Gaz hattı ---- GAZ ----\nA,.5,-.2,["GAZ",STANDARD,S=.1,R=0.0,X=-0.1,Y=-.05],-.25\n'),
        lin_case("Yazılı tip (mutlak dönüş, derece işaretli)", '*SU,Su ---- SU ----\nA,.5,-.2,["SU",STANDARD,S=.1,A=90d,X=-0.1,Y=-.05],-.25\n'),
        lin_case("Okunur yazı (U)", '*KABLO\nA,1,-.3,["K",STANDARD,S=.2,U=0.0,X=-.1,Y=-.1],-.3\n'),
        lin_case("Şekilli tip (SHX)", "*SINIR,Sınır --o--o--\nA,.25,-.1,[CIRC1,ltypeshp.shx,X=-.1,S=.1],-.1,1\n"),
        lin_case("Radyan ve grad açılar", '*R1\nA,1,-.5,["X",STANDARD,R=1.5708r],-.5\n*G1\nA,1,-.5,["Y",STANDARD,R=100g],-.5\n'),
        lin_case("12 öğe sınırda", "*ON2\nA,1,-1,1,-1,1,-1,1,-1,1,-1,1,-1\n"),
        lin_case("13 öğe reddedilir", "*ON3\nA,1,-1,1,-1,1,-1,1,-1,1,-1,1,-1,1\n"),
        lin_case("A ile başlamayan desen", "*YANLIS\nB,1,-1\n"),
        lin_case("Boşlukla başlayan desen", "*BOSLUK\nA,-1,1\n"),
        lin_case("Sayı olmayan uzunluk", "*SAYI\nA,1,-bir\n"),
        lin_case("Kapanmayan köşeli parantez", '*ACIK\nA,1,["X",STANDARD,S=1\n'),
        lin_case("Bilinmeyen dönüşüm", '*DON\nA,1,["X",STANDARD,Q=1],-1\n'),
        lin_case("Desensiz başlık", "*BOS,Desensiz\n*IKI\nA,1,-1\n"),
        lin_case("Başlıksız desen", "A,1,-1\n"),
        lin_case("Aynı ad iki kez (harf büyüklüğü sayılmaz)", "*Kesik\nA,1,-1\n*KESIK\nA,2,-2\n"),
        lin_case("Türkçe adlar ve açıklama", "*SINIR_ÇİZGİSİ,İmar sınırı ─ · ─\nA,2,-.5,0,-.5\n"),
        lin_case("Pencere satır sonları ve kenar boşlukları", "*KESIK , Kesikli\r\n  A , .5 , -.25  \r\n"),
    ]


def world(length, unit, plot_scale, drawing_unit, ltscale, object_scale):
    mul, div = UNIT_M[drawing_unit]
    base = length * plot_scale / 1000.0 if unit == "paper" else length * mul / div
    return base * ltscale * object_scale


def scale_case(name, lengths, unit, plot_scale=1000, drawing_unit="m", ltscale=1.0, object_scale=1.0):
    got = [world(x, unit, plot_scale, drawing_unit, ltscale, object_scale) for x in lengths]
    return {"name": name, "lengths": lengths, "unit": unit, "plotScale": plot_scale, "drawingUnit": drawing_unit,
            "ltscale": ltscale, "objectScale": object_scale,
            "expect": {"lengths": got, "period": sum(abs(x) for x in got)}}


def scale_cases():
    iso = [12.0, -3.0]
    return [
        scale_case("Kâğıt deseni 1:1000'de: mm metre olur", [3.0, -1.5], "paper"),
        scale_case("Kâğıt deseni 1:50'de", [3.0, -1.5], "paper", plot_scale=50),
        scale_case("Kâğıt deseni, LTSCALE 2", [3.0, -1.5], "paper", ltscale=2.0),
        scale_case("Kâğıt deseni, nesnenin ölçeği 0,5", [3.0, -1.5], "paper", object_scale=0.5),
        scale_case("Kâğıt deseni, ikisi birden çarpılır", [5.0, -1.2, 0.0, -1.2], "paper", plot_scale=200, ltscale=1.5,
                   object_scale=2.0),
        scale_case("Çizim deseni metre projesinde", iso, "drawing"),
        scale_case("Çizim deseni milimetre projesinde (ISO deseni)", iso, "drawing", drawing_unit="mm"),
        scale_case("Çizim deseni santimetre projesinde", iso, "drawing", drawing_unit="cm"),
        scale_case("Çizim deseni inç projesinde", [0.5, -0.25], "drawing", drawing_unit="in"),
        scale_case("Çizim deseni ayak projesinde", [0.5, -0.25], "drawing", drawing_unit="ft"),
        scale_case("Çizim deseni çizim ölçeğinden bağımsız", iso, "drawing", drawing_unit="mm", plot_scale=50),
        scale_case("Çizim deseni, LTSCALE 10 ve nesnenin ölçeği", iso, "drawing", drawing_unit="mm", ltscale=10.0,
                   object_scale=0.25),
        scale_case("Nokta uzunluksuz kalır", [0.0, -1.0], "paper", plot_scale=500, ltscale=3.0),
    ]


def short(x):
    """The shortest LIN form that reads back: no trailing zeros, `.5` not `0.5`."""
    if x == int(x):
        return str(int(x))
    t = repr(x)
    if t.startswith("0."):
        t = t[1:]
    elif t.startswith("-0."):
        t = "-" + t[2:]
    return t


def element_text(e):
    if "text" in e or "shape" in e:
        c = e.get("text") or e.get("shape")
        head = f'"{c["text"]}",{c["style"]}' if "text" in e else f'{c["name"]},{c["file"]}'
        parts = [head]
        if c["scale"] != 1.0:
            parts.append(f"S={short(c['scale'])}")
        if c.get("upright"):
            parts.append(f"U={short(c['rotation'])}")
        elif c["absolute"]:
            parts.append(f"A={short(c['rotation'])}")
        elif c["rotation"] != 0.0:
            parts.append(f"R={short(c['rotation'])}")
        if c["x"] != 0.0:
            parts.append(f"X={short(c['x'])}")
        if c["y"] != 0.0:
            parts.append(f"Y={short(c['y'])}")
        return "[" + ",".join(parts) + "]"
    return short(e["length"])


def write_lin(types):
    out = []
    for t in types:
        head = "*" + t["name"] + ("," + t["description"] if t.get("description") else "")
        out.append(head)
        out.append("A," + ",".join(element_text(e) for e in t["elements"]))
    return "\n".join(out) + "\n"


def written_cases():
    out = []
    for c in lin_cases():
        types = c["expect"]["types"]
        if types and not c["expect"]["problems"]:
            text = write_lin(types)
            again = read_lin(text, c["source"])["types"]
            assert again == types, c["name"]
            out.append({"name": c["name"], "types": types, "expect": text})
    return out


def main() -> int:
    doc = {"format": "kentos.linetype-cases", "version": 1,
           "note": "Written by scripts/fixtures/line_type_cases.py from AutoCAD's LIN documentation and docs/adr/0313; do not edit by hand.",
           "maxElements": MAX_ELEMENTS, "lin": lin_cases(), "scale": scale_cases(), "written": written_cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.name} güncel: {len(doc['lin'])} LIN, {len(doc['scale'])} ölçek, {len(doc['written'])} yazım durumu.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0




# ── cad.linetypes.edit (docs/adr/0313 §2) ────────────────────────────────────
#
# Written to fixtures/commands/v1/cad.linetypes.edit.json by `commands()`; the tree, the objects and the steps' form
# are layer_tree_command_cases.py's.

import importlib.util as _ilu


def _tree_module():
    spec = _ilu.spec_from_file_location("layer_tree_command_cases", Path(__file__).with_name("layer_tree_command_cases.py"))
    module = _ilu.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def canonical(t):
    """A type as the contract writes it: a complex element's placing without its defaults, no empty description."""
    out = {"name": t["name"]}
    if t.get("description"):
        out["description"] = t["description"]
    els = []
    for e in t["elements"]:
        el = {"length": e["length"]}
        for key in ("text", "shape"):
            if key in e:
                c = dict(e[key])
                for k, d in (("scale", 1.0), ("rotation", 0.0), ("absolute", False), ("upright", False), ("x", 0.0), ("y", 0.0)):
                    if c.get(k, d) == d:
                        c.pop(k, None)
                el[key] = c
        els.append(el)
    out["elements"] = els
    out["unit"] = t["unit"]
    if t.get("source"):
        out["source"] = t["source"]
    return out


def full(t):
    """A canonical type with every placing key, as `write_lin` reads it."""
    out = dict(t)
    els = []
    for e in t["elements"]:
        el = dict(e)
        for key in ("text", "shape"):
            if key in el:
                c = {"scale": 1.0, "rotation": 0.0, "absolute": False, "x": 0.0, "y": 0.0, **el[key]}
                el[key] = c
        els.append(el)
    out["elements"] = els
    return out


PROJECT_TYPES = [
    {"name": "KESIK", "description": "Kesikli", "elements": [{"length": 3.0}, {"length": -1.5}], "unit": "paper"},
    {"name": "GAZ", "description": "Gaz hattı", "elements": [{"length": 0.5}, {"length": -0.2},
                                                             {"length": 0.0, "text": {"text": "GAZ", "style": "STANDARD", "scale": 0.1, "x": -0.1, "y": -0.05}},
                                                             {"length": -0.25}], "unit": "drawing", "source": "deneme.lin"},
    {"name": "BOS", "elements": [{"length": 1.0}, {"length": -1.0}], "unit": "paper"},
]
FILE = ("*KESIK,Kesikli (dosyadan)\nA,6,-2\n"
        "*SU,Su hattı ---- SU ----\nA,.5,-.2,[\"SU\",STANDARD,S=.1,X=-0.1,Y=-.05],-.25\n"
        "*NOKTA\nA,0,-.5\n")
BAD_FILE = "*IYI\nA,1,-1\n*KOTU\nB,1,-1\n*BOSLUK\nA,-1,1\n"


def commands():
    T = _tree_module()
    node, step, completed, failed, VALID = T.node, T.step, T.completed, T.failed, T.VALID
    styled = {"color": "#E5484D", "lineType": "dashed", "linePattern": "GAZ", "lineWeight": 0.35}
    layers = [node("parsel", "Parsel"), node("gaz", "Gaz", style=styled)]

    def ent(id_, layer, **own):
        return {"kind": "line", "id": id_, "layerId": layer, "attrs": {}, "a": {"x": T.X, "y": T.Y}, "b": {"x": T.X + 10, "y": T.Y}, **own}

    entities = [ent(1, "parsel", lineType="dashed", linePattern="KESIK"), ent(2, "gaz")]
    ids = [e["id"] for e in entities]

    def setup(types=PROJECT_TYPES, **more):
        return T.setup(layers, entities, "parsel", lineTypes=[canonical(t) for t in types], **more)

    def done(**kw):
        out = {"changed": False, "added": [], "replaced": [], "skipped": [], "removed": [], "problems": []}
        out.update(kw)
        out["revision"] = "$current"
        return out

    def settings_after(types, scale=None):
        exp = {"lineTypes": [canonical(t) for t in types] if types else None, "lineTypeScale": scale}
        return exp

    def wrote(name, input_, output, types, scale=None, setup_doc=None):
        d = setup_doc or setup()
        return {"name": name, "setup": d, "steps": [
            step("plan", input_, completed(output), {"revision": "same"}),
            step("execute", input_, completed(output),
                 {"ids": [e["id"] for e in d["entities"]], "settings": settings_after(types, scale), "dirty": output["changed"],
                  "canUndo": False})]}

    def refused(name, input_, code, message, path, setup_doc=None):
        d = setup_doc or setup()
        return {"name": name, "setup": d, "steps": [step("execute", input_, failed(code, message, path), T.unchanged(d["layers"], ids))]}

    read = read_lin(FILE, "kentos.lin")
    kesik_file, su, nokta = (canonical(t) for t in read["types"])
    out = []
    out.append(wrote("Yükle: dosyanın yeni tipleri eklenir, projede aynı adlı olan kalır",
                     {"operation": "load", "lin": FILE, "source": "kentos.lin"},
                     done(changed=True, added=["SU", "NOKTA"], skipped=["KESIK"]), PROJECT_TYPES + [su, nokta]))
    out.append(wrote("Yükle: replace ile aynı adlı olan değişir, yerinde",
                     {"operation": "load", "lin": FILE, "source": "kentos.lin", "replace": True},
                     done(changed=True, added=["SU", "NOKTA"], replaced=["KESIK"]), [kesik_file] + PROJECT_TYPES[1:] + [su, nokta]))
    out.append(wrote("Yükle: yalnız adı verilenler (harf büyüklüğüne bakılmaz)",
                     {"operation": "load", "lin": FILE, "source": "kentos.lin", "names": ["su"]},
                     done(changed=True, added=["SU"]), PROJECT_TYPES + [su]))
    out.append(wrote("Yükle: dosyada olmayan ad söylenir",
                     {"operation": "load", "lin": FILE, "source": "kentos.lin", "names": ["SU", "YOK"]},
                     done(changed=True, added=["SU"], problems=[{"line": 0, "message": "“YOK” dosyada yok"}]), PROJECT_TYPES + [su]))
    bad = read_lin(BAD_FILE, "bozuk.lin")
    out.append(wrote("Yükle: okunmayan satırlar söylenir, okunanlar alınır",
                     {"operation": "load", "lin": BAD_FILE, "source": "bozuk.lin"},
                     done(changed=True, added=["IYI"], problems=bad["problems"]),
                     PROJECT_TYPES + [canonical(t) for t in bad["types"]]))
    out.append(wrote("Yükle: kaynak verilmezse dosya.lin",
                     {"operation": "load", "lin": "*TEK\nA,2,-1\n"},
                     done(changed=True, added=["TEK"]), PROJECT_TYPES + [canonical(read_lin("*TEK\nA,2,-1\n", "dosya.lin")["types"][0])]))
    out.append(wrote("Yükle: hiçbiri okunmazsa değişen yok",
                     {"operation": "load", "lin": "*KOTU\nB,1\n"},
                     done(problems=read_lin("*KOTU\nB,1\n", "dosya.lin")["problems"]), PROJECT_TYPES))
    out.append(wrote("Yükle: projesinde tip olmayan çizime",
                     {"operation": "load", "lin": FILE, "source": "kentos.lin"},
                     done(changed=True, added=["KESIK", "SU", "NOKTA"]), [kesik_file, su, nokta],
                     setup_doc=T.setup(layers, [ent(2, "gaz")], "parsel")))
    yeni = {"name": "ZIKZAK", "elements": [{"length": 2.0}, {"length": -0.5}, {"length": 0.0}, {"length": -0.5}], "unit": "paper"}
    out.append(wrote("Ekle: yeni tip sona", {"operation": "add", "lineType": yeni}, done(changed=True, added=["ZIKZAK"]),
                     PROJECT_TYPES + [yeni]))
    kesik2 = {"name": "Kesik", "elements": [{"length": 5.0}, {"length": -2.0}], "unit": "paper"}
    out.append(wrote("Ekle: replace ile aynı adlı yerinde değişir", {"operation": "add", "lineType": kesik2, "replace": True},
                     done(changed=True, replaced=["Kesik"]), [kesik2] + PROJECT_TYPES[1:]))
    out.append(wrote("Sil: kullanılmayan tip", {"operation": "remove", "name": "bos"}, done(changed=True, removed=["bos"]),
                     PROJECT_TYPES[:2]))
    out.append(wrote("Ölçek: 2,5", {"operation": "scale", "scale": 2.5}, done(changed=True, scale=2.5), PROJECT_TYPES, 2.5))
    out.append(wrote("Ölçek: 1 ölçeği kaldırır", {"operation": "scale", "scale": 1.0}, done(changed=True), PROJECT_TYPES,
                     None, setup_doc=setup(lineTypeScale=4.0)))
    out.append(wrote("Ölçek: aynı ölçek değişiklik değil", {"operation": "scale", "scale": 4.0}, done(scale=4.0), PROJECT_TYPES,
                     4.0, setup_doc=setup(lineTypeScale=4.0)))
    lin_all = write_lin([full(canonical(t)) for t in PROJECT_TYPES])
    out.append(wrote("Dışa aktar: bütün tipler LIN metni olarak; çizim değişmez", {"operation": "export"}, done(lin=lin_all),
                     PROJECT_TYPES))
    out.append(wrote("Dışa aktar: yalnız adı verilenler, projenin sırasıyla", {"operation": "export", "names": ["bos", "GAZ"]},
                     done(lin=write_lin([full(canonical(t)) for t in PROJECT_TYPES[1:]])), PROJECT_TYPES))
    out.append({"name": "Doğrula: yazmaz", "setup": setup(),
                "steps": [step("validate", {"operation": "remove", "name": "BOS"}, VALID, T.unchanged(layers, ids))]})
    out.append(refused("yükle: metin yok", {"operation": "load"}, "no_lin", "Yüklenecek LIN dosyasının metnini verin (lin).", "lin"))
    out.append(refused("yükle: boş metin", {"operation": "load", "lin": "  \n"}, "no_lin", "Yüklenecek LIN dosyasının metnini verin (lin).", "lin"))
    out.append(refused("ekle: tip yok", {"operation": "add"}, "no_definition", "Eklenecek çizgi tipini verin (lineType).", "lineType"))
    out.append(refused("ekle: aynı adlı var", {"operation": "add", "lineType": kesik2}, "invalid_line_type",
                       "“Kesik” adlı çizgi tipi projede var. Değiştirmek için replace verin ya da başka bir ad seçin.", "lineType"))
    long = {"name": "UZUN", "elements": [{"length": 1.0 if k % 2 == 0 else -1.0} for k in range(13)], "unit": "paper"}
    out.append(refused("ekle: 13 öğe", {"operation": "add", "lineType": long}, "invalid_line_type",
                       "Çizgi tipi geçersiz: “UZUN” çizgi tipinin deseni en çok 12 öğe olur, 13 var.", "lineType"))
    gap_first = {"name": "BOSLUK", "elements": [{"length": -1.0}, {"length": 1.0}], "unit": "paper"}
    out.append(refused("ekle: boşlukla başlayan", {"operation": "add", "lineType": gap_first}, "invalid_line_type",
                       "Çizgi tipi geçersiz: “BOSLUK” çizgi tipinin deseni bir çizgi ya da noktayla başlamalı.", "lineType"))
    out.append(refused("sil: ad yok", {"operation": "remove"}, "empty_name", "Silinecek çizgi tipinin adını verin (name).", "name"))
    out.append(refused("sil: projede yok", {"operation": "remove", "name": "YOK"}, "line_type_not_found",
                       "“YOK” adlı çizgi tipi projede yok.", "name"))
    out.append(refused("sil: katmanda kullanılıyor", {"operation": "remove", "name": "GAZ"}, "line_type_in_use",
                       "“GAZ” çizgi tipi 1 katmanda, 0 katman durumunda ve 0 nesnede kullanılıyor; önce onları başka bir çizgi tipine geçirin.", "name"))
    out.append(refused("sil: nesnede kullanılıyor", {"operation": "remove", "name": "kesik"}, "line_type_in_use",
                       "“kesik” çizgi tipi 0 katmanda, 0 katman durumunda ve 1 nesnede kullanılıyor; önce onları başka bir çizgi tipine geçirin.", "name"))
    for bad_scale in (0.0, -1.0, 1500.0):
        out.append(refused(f"ölçek: {bad_scale} aralık dışı", {"operation": "scale", "scale": bad_scale}, "invalid_line_type_scale",
                           "Çizgi tipi ölçeği 0'dan büyük, en çok 1000 olmalı (scale); 1 ölçeksizdir.", "scale"))
    out.append(refused("ölçek: verilmedi", {"operation": "scale"}, "invalid_line_type_scale",
                       "Çizgi tipi ölçeği 0'dan büyük, en çok 1000 olmalı (scale); 1 ölçeksizdir.", "scale"))
    out.append(refused("dışa aktar: projede olmayan ad", {"operation": "export", "names": ["YOK"]}, "line_type_not_found",
                       "“YOK” adlı çizgi tipi projede yok.", "names"))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "scale", "scale": 2.0, "expectedRevision": "on iki"},
                       "invalid_revision", T.INVALID_REVISION, "expectedRevision"))
    out.append({**T.conflict_case("sürüm çakışması: hiçbir şey yazılmaz",
                                  step("execute", {"operation": "scale", "scale": 2.0}, completed(done(changed=True, scale=2.0)),
                                       {"revision": "changed"}),
                                  {"operation": "scale", "scale": 3.0}, {}), "setup": setup()})
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.linetypes.edit", "commandVersion": 1,
            "title": "Çizgi tipleri",
            "note": "Written by scripts/fixtures/line_type_cases.py from AutoCAD's LIN documentation, the contract's rules and docs/adr/0313; do not edit by hand.",
            "setup": setup(), "cases": out}


def write_commands(check):
    path = ROOT / "fixtures/commands/v1/cad.linetypes.edit.json"
    doc = commands()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{path.name} güncel: {len(doc['cases'])} durum.")
        return 0
    path.write_text(text, encoding="utf-8")
    print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main() | write_commands("--check" in sys.argv[1:]))
