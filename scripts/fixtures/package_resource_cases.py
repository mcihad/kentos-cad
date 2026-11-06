#!/usr/bin/env python3
"""Paketle's resource report (docs/adr/1111), shared cases from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/file.package.resources.json (a headless file host's cases, played by
crates/native/headless/tests/headless/package_resources.rs) and the references' sources under
fixtures/package/v1/kaynaklar/ with the independent KCAD writer (kcad_v2_reference.py); `--check` compares and writes
nothing. The rules, as ADR 1111 §1 gives them:

- a row per (kind, name), sorted by kind (font, shapeFile, lineType, hatchPattern, plotStyle) and then by the name's
  characters; its users sorted the same way;
- **font**: each text style of the project, named by the style; its file the typeface file it names, else KentOS's face
  (`barlow` …). It is `builtIn` when it names no file, a KentOS face's family (Barlow, Arimo, Overpass, Quicksand,
  Architects Daughter, Courier Prime, IBM Plex Mono) or one of the families KentOS draws in an equivalent (Arial,
  Helvetica, Liberation Sans → Arimo; Courier New, Courier → Courier Prime), and `substituted` for an SHX file or any
  other family (with a note and `package_left`); `objects` the drawing's texts that take the style. A style a reference's
  source gives its own texts that the drawing has none of (by name, letters' case aside; never Standart) is a `carried`
  font row of that reference;
- **lineType**: each of the project's line types (`embedded`); and each one a layer (its pattern, else its built-in
  type) or an object (its own pattern or type, the drawing's and every block's, a reference's pieces too) names:
  `embedded` when the project has it (named as the project names it), `builtIn` for continuous/dashed/dashdot/dotted,
  `carried` when only a copied reference's source defines it, else `missing`; a layer's user is `katman:` and its path,
  an object counts in `objects`;
- **shapeFile**: each SHX file a project line type's shape names (lower-cased), `embedded` when the project's library
  holds its resource (`shx:` and the file's name lower-cased), else `missing`; its users `çizgi tipi:` and the type;
- **hatchPattern**: each of the project's patterns (`embedded`); each hatch: `solid`, `lines`, `cross`, `gradient`
  `builtIn`; a pattern by its name `embedded` when the project has it or the hatch carries its own lines, `carried` when
  only a reference's source defines it, else `missing`; `objects` the hatches;
- **plotStyle**: each plot profile of the sheet book (`embedded`), a sheet's profile (its own copy, else the book's)
  adding `pafta:` and its name; a sheet whose profile neither has is `missing` by the profile's id (a profile's named
  styles are whole: the sheet core's book check refuses one that assigns a style it has not, ADR 0609);
- the warning `package_resources` lists every `missing` and `carried` row.
"""

import argparse
import base64
import copy
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as k  # noqa: E402  (the independent KCAD v2 writer)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/file.package.resources.json"
SRC = ROOT / "fixtures/package/v1/kaynaklar"
KINDS = ["font", "shapeFile", "lineType", "hatchPattern", "plotStyle"]
WORDS = {"font": "yazı stili", "shapeFile": "SHX şekil dosyası", "lineType": "çizgi tipi",
         "hatchPattern": "tarama deseni", "plotStyle": "baskı stili"}
BUILT_IN_TYPES = ["continuous", "dashed", "dashdot", "dotted"]
OWN = ["barlow", "arimo", "overpass", "quicksand", "architectsdaughter", "courierprime", "ibmplexmono", "plexmono"]
EQUIVALENT = ["arial", "helvetica", "liberationsans", "couriernew", "courier", "cour"]
STYLES = {"", "regular", "normal", "roman", "medium", "semibold", "bold", "italic", "oblique", "bolditalic",
          "boldoblique", "400", "500", "600", "700", "400italic", "bd", "bi", "i", "b", "z"}
SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100.0,
            "workspace": "cad"}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def uid(n):
    return f"0192f5c1-8a02-7000-8000-{n:012d}"


def style(line_type="continuous", pattern=None):
    s = {"color": "fg", "lineType": line_type, "lineWeight": 0.25}
    if pattern:
        s["linePattern"] = pattern
    return s


def layer(id, name, st=None, kind="layer", children=None):
    return {"id": id, "name": name, "type": kind, "visible": True, "locked": False, "expanded": True,
            "style": st or style(), "children": children or []}


def line(i, layer_id="cizim", **look):
    return {"kind": "line", "id": i, "layerId": layer_id, "attrs": {}, "a": P(0, i), "b": P(4, i), **look}


def text(i, style_id, layer_id="cizim"):
    return {"kind": "text", "id": i, "layerId": layer_id, "attrs": {}, "p": P(0, i), "text": f"Yazı {i}",
            "height": 0.5, "rotation": 0.0, "textStyle": style_id}


def hatch(i, pattern):
    return {"kind": "hatch", "id": i, "layerId": "cizim", "attrs": {},
            "ring": [P(10, i), P(12, i), P(12, i + 1), P(10, i + 1)], "pattern": pattern}


def pattern(name, lines=None):
    p = {"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": name, "scale": 1.0}
    if lines:
        p["lines"] = lines
    return p


LINES = [{"angle": 45.0, "origin": [0.0, 0.0], "offset": [0.0, 3.175]}]
GAZ = {"name": "GAZ", "description": "Gaz hattı", "unit": "paper",
       "elements": [{"length": 6.0}, {"length": -2.0}]}
SHX_BYTES = b"AutoCAD-86 shapes 1.0\r\n\x1a"


def shx_item(file):
    return {"id": f"shx:{file.lower()}", "kind": "asset", "name": file, "path": ["CAD", "SHX"], "format": "shx",
            "file": file, "data": "data:application/x-autocad-shx;base64," + base64.b64encode(SHX_BYTES).decode(),
            "sha256": hashlib.sha256(SHX_BYTES).hexdigest()}


def text_style(id, name, font_file=None):
    s = {"id": id, "name": name, "font": "barlow"}
    if font_file:
        s["fontFile"] = font_file
    return s


def drawing(**over):
    d = {"format": "kentos.document", "version": 2, "name": "Paket kaynakları", "settings": copy.deepcopy(SETTINGS),
         "origin": P(0, 0), "layers": [layer("0", "0"), layer("cizim", "Çizim")], "activeLayer": "cizim",
         "entities": [], "styles": {"items": [], "categories": []}, "uids": []}
    d.update(copy.deepcopy(over))
    return d


def entities(d, *es):
    for e in es:
        d["entities"].append(copy.deepcopy(e))
        d["uids"].append(uid(e["id"]))
    return d


# ── the references' sources (written with the independent KCAD writer) ──────────────────────────────────────────

SOURCE_SINIR = drawing(name="Kadastro")
SOURCE_SINIR["settings"]["lineTypes"] = [{"name": "SINIR", "unit": "paper", "elements": [{"length": 3.0}, {"length": -1.0}]}]
SOURCE_SINIR["settings"]["hatchPatterns"] = [{"name": "TUGLA", "unit": "drawing", "lines": LINES}]
SOURCE_SINIR["settings"]["textStyles"] = [text_style("0192f5c1-8a02-7000-8000-0000000a0001", "Başlık"),
                                          text_style("0192f5c1-8a02-7000-8000-0000000a0002", "Not")]
SOURCE_SINIR["layers"] = [layer("0", "0"), layer("sinir", "Sınır", style("dashed", "SINIR"))]
entities(SOURCE_SINIR, line(1, "sinir"), text(2, "0192f5c1-8a02-7000-8000-0000000a0001", "0"),
         text(3, "0192f5c1-8a02-7000-8000-0000000a0002", "0"))
FILES = {"kadastro.kcad": k.container(k.document(SOURCE_SINIR))}
DIGEST = {n: hashlib.sha256(b).hexdigest() for n, b in FILES.items()}
XREF_ID = "0192f5c1-8a02-7000-8000-00000000b001"


def with_xref(d, *, found=True, pieces=(), layers=()):
    """`d` with the reference Kadastro: its group's layers and the pieces its definition holds."""
    path = "$fixture:package/v1/kaynaklar/kadastro.kcad" if found else "/yok/kadastro.kcad"
    d["layers"].append(layer("xr", "Dış başvuru: Kadastro", kind="group", children=list(layers)))
    d.setdefault("blocks", []).append({
        "id": XREF_ID, "name": "Kadastro", "base": P(0, 0), "units": "m",
        "xref": {"path": path, "digest": DIGEST["kadastro.kcad"], "group": "xr",
                 "objects": [uid(800 + n) for n, _ in enumerate(pieces)]},
        "entities": [copy.deepcopy(p) for p in pieces]})
    entities(d, {"kind": "insert", "id": 90, "layerId": "cizim", "attrs": {}, "block": XREF_ID, "p": P(0, 0),
                 "scale": 1.0, "rotation": 0.0})
    return d


def book(*sheets, profiles=()):
    return {"book": {"schema": "kentos.sheet/1", "sheets": list(sheets), "plotProfiles": list(profiles)}}


def sheet(id, name, profile=None, own=None):
    s = {"id": id, "name": name,
         "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                  "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
         "items": []}
    if profile:
        s["plot"] = {"settings": {"profile": profile}}
        if own:
            s["plot"]["settings"]["profileSnapshot"] = own
    return s


def profile(id, name, entries=("Ink",), assigned=(), default="Ink"):
    p = {"id": id, "name": name, "kind": "named",
         "entries": [{"key": {"type": "named", "name": n}, "style": {"color": {"type": "object"}, "opacity": 100}}
                     for n in entries],
         "assignments": [{"target": {"type": "modelLayer", "id": "cizim"}, "key": {"type": "named", "name": n}}
                         for n in assigned]}
    if default:
        p["defaultStyle"] = default
    return p


# ── the reference's rules ──────────────────────────────────────────────────────────────────────────────────────


def fold(s):
    return s.strip().lower()


def font_how(file):
    if file is None:
        return "builtIn"
    leaf = file.strip().lower().replace("\\", "/").rsplit("/", 1)[-1]
    if leaf.endswith(".shx"):
        return "substituted"
    for ext in (".ttf", ".otf", ".ttc", ".woff2", ".woff"):
        if leaf.endswith(ext):
            leaf = leaf[: -len(ext)]
            break
    n = "".join(c for c in leaf if c.isascii() and c.isalnum())
    # A family and then nothing or one of its weights and styles (ADR 0324 §5).
    if any(n.startswith(s) and n[len(s):] in STYLES for s in OWN + EQUIVALENT):
        return "builtIn"
    return "substituted"


def layer_paths(nodes, here=""):
    for n in nodes:
        path = n["name"] if not here else f"{here} / {n['name']}"
        if n["type"] == "layer":
            yield path, n
        yield from layer_paths(n.get("children", []), path)


def source_of(d):
    """What the copied reference's source defines, if it is found: its line types, patterns and its texts' styles."""
    out = {"lineTypes": set(), "patterns": set(), "styles": []}
    for b in d.get("blocks", []):
        x = b.get("xref")
        if not x or not x["path"].startswith("$fixture:"):
            continue
        s = SOURCE_SINIR
        out["lineTypes"] |= {fold(t["name"]) for t in s["settings"].get("lineTypes", [])}
        out["patterns"] |= {fold(p["name"]) for p in s["settings"].get("hatchPatterns", [])}
        ids = {t["id"]: t["name"] for t in s["settings"].get("textStyles", [])}
        for e in s["entities"]:
            name = ids.get(e.get("textStyle"))
            if name and not any(fold(name) == fold(n) for _, n in out["styles"]):
                out["styles"].append((b["name"], name))
    return out


class Rows:
    def __init__(self):
        self.rows = {}

    def row(self, kind, name, how):
        key = (kind, name)
        if key not in self.rows:
            self.rows[key] = {"kind": kind, "name": name, "how": how, "users": set(), "objects": 0}
        return self.rows[key]

    def out(self):
        listed = []
        for (kind, name), r in sorted(self.rows.items(), key=lambda kv: (KINDS.index(kv[0][0]), kv[0][1].encode())):
            o = {"kind": kind, "name": name, "how": r["how"]}
            if r.get("file"):
                o["file"] = r["file"]
            if r["users"]:
                o["users"] = sorted(r["users"], key=str.encode)
            if r["objects"]:
                o["objects"] = r["objects"]
            if r.get("note"):
                o["note"] = r["note"]
            listed.append(o)
        return listed


def report(d):
    rows = Rows()
    s = d["settings"]
    theirs = source_of(d)
    styles = s.get("textStyles", [])
    for t in styles:
        how = font_how(t.get("fontFile"))
        r = rows.row("font", t["name"], how)
        r["file"] = t.get("fontFile") or t["font"]
        r["users"].add(f"stil:{t['name']}")
        r["objects"] += sum(1 for e in d["entities"] if e["kind"] == "text" and e.get("textStyle") == t["id"])
        if how == "substituted":
            r["note"] = "KentOS'ta bu yazı tipi yok; yazıları yerine başka yüzle çizilir"
    ours = {fold(t["name"]) for t in styles}
    for xref, name in theirs["styles"]:
        if fold(name) in ours or fold(name) == "standart":
            continue
        r = rows.row("font", name, "carried")
        r["users"].add(f"xref:{xref}")
        r["note"] = "dış başvurunun kaynağındaki yazı stili; bu çizimde yok, yazıları Standart ile çizilir"
    project = {fold(t["name"]): t["name"] for t in s.get("lineTypes", [])}
    for t in s.get("lineTypes", []):
        rows.row("lineType", t["name"], "embedded")

    def line_type(name, user):
        if fold(name) in project:
            how, name = "embedded", project[fold(name)]
        elif name in BUILT_IN_TYPES:
            how = "builtIn"
        elif fold(name) in theirs["lineTypes"]:
            how = "carried"
        else:
            how = "missing"
        r = rows.row("lineType", name, how)
        if user:
            r["users"].add(user)
        else:
            r["objects"] += 1
        if "note" not in r:
            note = {"carried": "dış başvurunun kaynağında tanımlı; bu çizimde yok, en yakın yerleşik tiple çizilir",
                    "missing": "hiçbir yerde tanımlı değil; en yakın yerleşik tiple çizilir"}.get(how)
            if note:
                r["note"] = note

    for path, n in layer_paths(d["layers"]):
        line_type(n["style"].get("linePattern") or n["style"]["lineType"], f"katman:{path}")
    pieces = list(d["entities"]) + [e for b in d.get("blocks", []) for e in b["entities"]]
    for e in pieces:
        name = e.get("linePattern") or e.get("lineType")
        if name:
            line_type(name, None)
    library = {i["id"] for i in d["styles"]["items"] if i.get("kind") == "asset" and i.get("format") == "shx"}
    for t in s.get("lineTypes", []):
        for el in t["elements"]:
            sh = el.get("shape")
            if not sh:
                continue
            file = sh["file"].replace("\\", "/").rsplit("/", 1)[-1]
            have = f"shx:{file.lower()}" in library
            r = rows.row("shapeFile", file.lower(), "embedded" if have else "missing")
            r.setdefault("file", file)
            r["users"].add(f"çizgi tipi:{t['name']}")
            if not have:
                r["note"] = "projenin kitaplığında yok; çizgi tipinin şekilleri çizilemez"
    patterns = {fold(p["name"]): p["name"] for p in s.get("hatchPatterns", [])}
    for p in s.get("hatchPatterns", []):
        rows.row("hatchPattern", p["name"], "embedded")
    for e in pieces:
        if e["kind"] != "hatch":
            continue
        p = e["pattern"]
        note = None
        if p["type"] in ("solid", "lines", "cross", "gradient"):
            name, how = p["type"], "builtIn"
        else:
            name = p.get("name") or "adsız"
            if fold(name) in patterns:
                how = "embedded"
            elif p.get("lines"):
                how, note = "embedded", "desenin çizgileri taramanın kendisinde"
            elif fold(name) in theirs["patterns"]:
                how, note = "carried", "dış başvurunun kaynağında tanımlı; taramada çizgisi yok"
            else:
                how, note = "missing", "tanımı yok; tarama deseni çizilemez"
        name = patterns.get(fold(name), name)
        r = rows.row("hatchPattern", name, how)
        r["objects"] += 1
        if note and "note" not in r:
            r["note"] = note
    b = d.get("sheets", {}).get("book")
    if b:
        profiles = b.get("plotProfiles", [])
        for p in profiles:
            rows.row("plotStyle", p["name"], "embedded")
        for sh in b["sheets"]:
            pid = sh.get("plot", {}).get("settings", {}).get("profile")
            if not pid:
                continue
            own = sh["plot"]["settings"].get("profileSnapshot")
            p = own if own and own["id"] == pid else next((x for x in profiles if x["id"] == pid), None)
            if p:
                rows.row("plotStyle", p["name"], "embedded")["users"].add(f"pafta:{sh['name']}")
            else:
                r = rows.row("plotStyle", pid, "missing")
                r["users"].add(f"pafta:{sh['name']}")
                r["note"] = "paftanın baskı profili kitapta yok"
    listed = rows.out()
    warnings = []
    # Left as they were (ADR 0350 §4): a style drawn in a substitute face, a reference whose source is not found.
    substituted = any(t.get("fontFile") and font_how(t["fontFile"]) == "substituted" for t in styles)
    unfound = any(not b_["xref"]["path"].startswith("$fixture:") for b_ in d.get("blocks", []) if b_.get("xref"))
    if substituted or unfound:
        warnings.append({"code": "package_left"})
    findings = [r for r in listed if r["how"] in ("missing", "carried")]
    if findings:
        said = ", ".join(f"{r['name']} ({WORDS[r['kind']]}, "
                         f"{'eksik' if r['how'] == 'missing' else 'yalnız dış başvurunun kaynağında'})" for r in findings)
        warnings.append({"code": "package_resources",
                         "message": f"{len(findings)} kaynak pakette tanımlı değil ya da yalnız dış başvurunun kaynağında: "
                                    f"{said}. Projeye ekleyip yeniden paketleyin."})
    return listed, warnings


# ── the cases ──────────────────────────────────────────────────────────────────────────────────────────────────

cases = []


def add(name, d):
    assert len(name) > 10 and name not in [c["name"] for c in cases], name
    listed, warnings = report(d)
    cases.append({"name": name, "setup": d, "op": "plan",
                  "expect": {"status": "completed", "resources": listed, "warnings": warnings}})


add("Boş çizim: katmanların sürekli çizgisi KentOS'la gelir", drawing())
add("Kesikli katman ve noktalı nesne yerleşik tiplerdir",
    entities(drawing(layers=[layer("0", "0"), layer("cizim", "Çizim", style("dashed"))]), line(1, lineType="dotted")))
d = drawing(layers=[layer("0", "0"), layer("cizim", "Çizim"), layer("gaz", "Gaz", style("dashed", "GAZ"))])
d["settings"]["lineTypes"] = [GAZ]
add("Projenin desenli çizgi tipi katmanda: proje dosyasına gömülü", d)
add("Tanımsız desen katmanda eksik ve uyarılır",
    drawing(layers=[layer("0", "0"), layer("cizim", "Çizim"), layer("gaz", "Gaz", style("dashed", "GAZ"))]))
add("Tanımsız desen nesnede eksik, nesnesi sayılır",
    entities(drawing(), line(1, lineType="dashed", linePattern="SU"), line(2, lineType="dashed", linePattern="SU")))
d = drawing(layers=[layer("0", "0"), layer("cizim", "Çizim", style("dashed", "gaz"))])
d["settings"]["lineTypes"] = [GAZ]
add("Desen adı büyük küçük harfe bakmadan projenin adıyla", d)
d = drawing()
d["settings"]["lineTypes"] = [GAZ]
add("Kullanılmayan proje çizgi tipi de gömülü listelenir", d)
d = drawing()
d["settings"]["lineTypes"] = [{"name": "CIT", "unit": "paper", "elements": [
    {"length": 2.0}, {"length": 0.0, "shape": {"name": "CIRC1", "file": "ltypeshp.shx", "scale": 0.5}}, {"length": -1.0}]}]
d["styles"]["items"] = [shx_item("ltypeshp.shx")]
add("Şekilli çizgi tipinin SHX dosyası kitaplıkta gömülü", d)
d = drawing()
d["settings"]["lineTypes"] = [{"name": "CIT", "unit": "paper", "elements": [
    {"length": 2.0}, {"length": 0.0, "shape": {"name": "CIRC1", "file": "C:\\Fonts\\LtypeShp.shx", "scale": 0.5}},
    {"length": -1.0}]}]
add("Şekilli çizgi tipinin SHX dosyası yoksa eksik, dosya adıyla", d)
d = drawing()
d["settings"]["textStyles"] = [text_style("0192f5c1-8a02-7000-8000-0000000a0101", "Başlık")]
entities(d, text(1, "0192f5c1-8a02-7000-8000-0000000a0101"), text(2, "0192f5c1-8a02-7000-8000-0000000a0101"))
add("Yazı stilinin KentOS yüzü KentOS'la gelir, yazıları sayılır", d)
d = drawing()
d["settings"]["textStyles"] = [text_style("0192f5c1-8a02-7000-8000-0000000a0102", "Gövde", "arial.ttf")]
add("Arial'ı adlandıran stil eşdeğer KentOS yüzüyle gelir", d)
d = drawing()
d["settings"]["textStyles"] = [text_style("0192f5c1-8a02-7000-8000-0000000a0103", "Teknik", "romans.shx")]
entities(d, text(1, "0192f5c1-8a02-7000-8000-0000000a0103"))
add("SHX yazı tipli stil başka yüzle çizilir", d)
d = drawing()
d["settings"]["textStyles"] = [text_style("0192f5c1-8a02-7000-8000-0000000a0104", "Eski", "Garamond.ttf")]
add("KentOS'ta olmayan aile başka yüzle çizilir", d)
add("Dolu, çizgili ve geçişli taramalar yerleşiktir", entities(drawing(), hatch(1, {"type": "solid", "angle": 0.0, "spacing": 1.0}),
    hatch(2, {"type": "lines", "angle": 45.0, "spacing": 0.5}), hatch(3, {"type": "solid", "angle": 0.0, "spacing": 1.0}),
    hatch(4, {"type": "gradient", "angle": 0.0, "spacing": 1.0, "gradient": {"shape": "linear", "color2": "#FFFFFF"}})))
add("Çizgileri kendisinde olan desen gömülü sayılır", entities(drawing(), hatch(1, pattern("ANSI31", LINES))))
d = entities(drawing(), hatch(1, pattern("tugla", LINES)))
d["settings"]["hatchPatterns"] = [{"name": "TUGLA", "unit": "drawing", "lines": LINES}]
add("Projenin deseni adıyla gömülü", d)
d = drawing()
d["blocks"] = [{"id": "0192f5c1-8a02-7000-8000-00000000b101", "name": "Kapı", "base": P(0, 0), "units": "m",
                "entities": [line(1, "0", lineType="dashdot"), hatch(2, pattern("NET", LINES))]}]
entities(d, {"kind": "insert", "id": 1, "layerId": "cizim", "attrs": {}, "block": "0192f5c1-8a02-7000-8000-00000000b101",
             "p": P(0, 0), "scale": 1.0, "rotation": 0.0})
add("Bloğun parçalarının çizgi tipi ve deseni de sayılır", d)
add("Paftanın baskı profili kitapta gömülü", drawing(sheets=book(sheet("a-101", "A-101 Plan", "p-1"),
                                                           sheet("a-102", "A-102 Kesit", "p-1"),
                                                           profiles=[profile("p-1", "Renkli")])))
add("Paftanın baskı profili kitapta yoksa eksik; kullanılmayan profil de listelenir", drawing(sheets=book(
    sheet("a-101", "A-101 Plan", "p-9"), profiles=[profile("p-1", "Gri", entries=("İnce",), assigned=("İnce",),
                                                          default=None)])))
add("Paftanın kendi profil kopyası gömülüdür", drawing(sheets=book(
    sheet("a-101", "A-101 Plan", "p-7", own=profile("p-7", "Tek renk")), sheet("a-102", "A-102 Kesit"))))
add("Dış başvurunun katmanındaki desen yalnız kaynağında: taşınır ama çizimde yok",
    with_xref(drawing(), layers=[layer("xr-s", "Sınır", style("dashed", "SINIR"))], pieces=[line(1, "xr-s")]))
add("Dış başvurunun kaynağındaki yazı stilleri bu çizimde yoksa taşınır",
    with_xref(drawing(settings={**SETTINGS, "textStyles": [text_style("0192f5c1-8a02-7000-8000-0000000a0201", "not")]}),
              pieces=[line(1, "0")]))
add("Dış başvurunun parçasındaki tanımsız desen kaynağında da yoksa eksik",
    with_xref(drawing(), pieces=[line(1, "0", lineType="dashed", linePattern="KANAL"), hatch(2, pattern("TUGLA", LINES))]))
add("Kaynağı bulunamayan dış başvurunun deseni eksik sayılır",
    with_xref(drawing(), found=False, layers=[layer("xr-s", "Sınır", style("dashed", "SINIR"))], pieces=[line(1, "xr-s")]))


def doc():
    return {"format": "kentos.command-cases", "version": 1, "command": "file.package", "commandVersion": 1,
            "host": "headless-file", "title": "Paketle: yazı tipi, SHX, çizgi tipi, desen ve baskı stili raporu",
            "note": "scripts/fixtures/package_resource_cases.py yazar; ADR 1111'in kurallarından, KentOS kodu olmadan. "
                    "Başsız sunucunun tests/headless/package_resources.rs'i çalıştırır.",
            "cases": cases}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    assert len(cases) >= 20, len(cases)
    text_ = json.dumps(doc(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        stale = [p for p, b in [(OUT, text_.encode())] + [(SRC / n, b) for n, b in FILES.items()]
                 if not p.exists() or p.read_bytes() != b]
        if stale:
            print("güncel değil: " + ", ".join(str(p.relative_to(ROOT)) for p in stale))
            return 1
        print(f"file.package (kaynaklar): {len(cases)} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text_)
    SRC.mkdir(parents=True, exist_ok=True)
    for n, b in FILES.items():
        (SRC / n).write_bytes(b)
    print(f"{OUT.relative_to(ROOT)}: {len(cases)} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
