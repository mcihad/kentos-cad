"""Writes fixtures/commands/v1/file.import.dxf.json: the shared cases of `file.import.dxf` (docs/adr/0257 §3), and the two
DXF files only these cases read (fixtures/formats/v1/dxf-import).

Written without KentOS code, from the ADR and the DXF reference:

- the files are the units' fixtures (fixtures/formats/v1/dxf-units, written by dxf_units_cases.py, whose rule of a
  file's unit is used here) and two more written here: one with no object, one with an object the reader does not
  take (a 3DSOLID);
- a file's unit by the ADR's rule (§1); its warnings in the ADR's words; the numbers turned into metres as the units'
  cases turn them (one multiplication, then one division);
- each of the file's layers onto the drawing layer `layers` names, else onto the drawing's layer of the same name
  (case and Turkish marks aside), else onto a new layer named as in the file, id `import-<name folded>`
  (`-2`, `-3` … when taken), the new layers in a group named after the file (`group`, or none for an empty one), found
  at the top of the tree by its name or made, its id from its name likewise, after the layers';
- a block the drawing has the name of goes in as “Ad (2)”, “Ad (3)” …;
- the refusals in the ADR's order and words;
- the objects' fields the cases look at (the rest is the reader's, which these cases do not know): a line's ends, a
  circle's centre and radius, a text's place, height and words, a note box's width, an insert's place, scale and
  block.

    python3 scripts/fixtures/dxf_import_command_cases.py           # writes the cases and the two files
    python3 scripts/fixtures/dxf_import_command_cases.py --check   # compares them with the ones on disk
"""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import dxf_units_cases as units  # noqa: E402  (the files and the rule of a DXF's unit)

OUT = ROOT / "fixtures/commands/v1/file.import.dxf.json"
UNITS_DIR = "formats/v1/dxf-units"
IMPORT_DIR = "formats/v1/dxf-import"

# ── The two files only these cases read ─────────────────────────────────

EMPTY = units.dxf(4, [], "")
LOSSES = units.dxf(4, [], units.line(0.0, 0.0, 2500.0, 0.0) + units.pairs(
    (0, "3DSOLID"), (8, "DUVAR"), (100, "AcDbEntity"), (100, "AcDbModelerGeometry"), (70, 1), (1, "ACIS")))


def lwpolyline(pts, closed=False, constant=None, widths=None, layer="DUVAR"):
    """A LWPOLYLINE: its vertices, closed (70), a constant width (43) or each vertex's start and end (40, 41)."""
    groups = [(0, "LWPOLYLINE"), (8, layer), (100, "AcDbEntity"), (100, "AcDbPolyline"), (90, len(pts)),
              (70, 1 if closed else 0)]
    if constant is not None:
        groups.append((43, constant))
    for i, (x, y) in enumerate(pts):
        groups += [(10, x), (20, y)]
        if widths:
            start, end = widths[i]
            groups += [(40, start)] if start else []
            groups += [(41, end)] if end else []
    return units.pairs(*groups)


# Closed and wide polylines (docs/adr/0258), in mm: a closed room, a 500 mm wide wall line, a closed tapered frame.
SQUARE_MM = [(0.0, 0.0), (4000.0, 0.0), (4000.0, 3000.0), (0.0, 3000.0)]
WALL_MM = [(6000.0, 0.0), (10000.0, 0.0), (10000.0, 3000.0)]
FRAME_MM = [(12000.0, 0.0), (16000.0, 0.0), (16000.0, 3000.0)]
FRAME_WIDTHS_MM = [(200.0, 400.0), (400.0, 400.0), (400.0, 200.0)]
CLOSED = units.dxf(4, [], lwpolyline(SQUARE_MM, closed=True) + lwpolyline(WALL_MM, constant=500.0)
                   + lwpolyline(FRAME_MM, closed=True, widths=FRAME_WIDTHS_MM))
EXTRA = {"empty.dxf": EMPTY, "losses.dxf": LOSSES, "closed.dxf": CLOSED}


def text_of(rel: str) -> str:
    """A file's text: the units' fixtures as their script writes them, the two of this script."""
    folder, name = rel.rsplit("/", 1)
    if folder == UNITS_DIR:
        return units.dxf(*units.FILES[name])
    return EXTRA[name]


# ── The drawings imported into ──────────────────────────────────────────

def layer(id: str, name: str, locked: bool = False) -> dict:
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def group(id: str, name: str, children: list) -> dict:
    return {"id": id, "name": name, "type": "group", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": children}


def setup(unit: str | None = "m", duvar: str | None = "Duvar", duvar_locked: bool = False, kapi: bool = False,
          mimari: bool = False) -> dict:
    """A drawing: a local project in `unit` (None: one with a coordinate system), the layers Çizim, the file's
    DUVAR's namesake (`duvar`, maybe locked), Kilitli (locked), the group Grup with İç, maybe the group Mimari; one
    point; maybe a block KAPI."""
    settings = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 100, "workspace": "cad"}
    if unit is None:
        settings["srid"] = 5256
        settings["workspace"] = "gis"
    elif unit != "m":
        settings["drawingUnit"] = unit
    layers = [layer("cizim", "Çizim")]
    if duvar:
        layers.append(layer("duvar", duvar, duvar_locked))
    layers += [layer("kilitli", "Kilitli", True), group("grup", "Grup", [layer("ic", "İç")])]
    if mimari:
        layers.append(group("mimari", "Mimari", []))
    doc = {"format": "kentos.document", "version": 1, "name": "İçe alma", "settings": settings,
           "origin": {"x": 0, "y": 0}, "layers": layers, "activeLayer": "cizim",
           "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": {"x": 0.0, "y": 0.0}}],
           "styles": {"items": [], "categories": []}}
    if kapi:
        doc["blocks"] = [{"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001", "name": "KAPI", "base": {"x": 0, "y": 0},
                          "entities": [{"kind": "line", "id": 1, "layerId": "0", "attrs": {},
                                        "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}}]}]
    return doc


def tree_ids(doc: dict) -> set:
    out = set()
    stack = list(doc["layers"])
    while stack:
        n = stack.pop()
        out.add(n["id"])
        stack.extend(n.get("children", []))
    return out


# ── The rules ───────────────────────────────────────────────────────────

FOLD = {"Ç": "C", "Ğ": "G", "İ": "I", "Ö": "O", "Ş": "S", "Ü": "U"}


def fold(text: str) -> str:
    """Trimmed, upper case, the Turkish letters as plain ones (`İ` and `I` alike)."""
    return "".join(FOLD.get(c, c) for c in text.strip().upper())


def fresh_id(name: str, taken: set) -> str:
    """`import-` and the name folded, lower case, a hyphen for each run of other characters; `-2` … when taken."""
    slug = ""
    for c in fold(name).lower():
        if ("a" <= c <= "z") or ("0" <= c <= "9"):
            slug += c
        elif not slug.endswith("-"):
            slug += "-"
    slug = slug.strip("-") or "katman"
    base = out = f"import-{slug}"
    k = 2
    while out in taken:
        out = f"{base}-{k}"
        k += 1
    taken.add(out)
    return out


def decision(rel: str, project: str, file_unit: str | None) -> tuple[dict, list]:
    """The unit the values are read in, and its warnings in the ADR's words (§1, §2)."""
    name = rel.rsplit("/", 1)[1]
    declared = units.FILES[name][0] if rel.startswith(UNITS_DIR) else 4
    known = declared if declared in units.AUTOCAD else None
    if file_unit is not None:
        used, source, assumed = units.CODES[file_unit], "chosen", False
    elif project == "crs":
        used, source, assumed = 6, "metres", known is None
    elif known is not None:
        used, source, assumed = known, "declared", False
    else:
        used, source, assumed = units.CODES[project], "project", True
    out = units.decision(declared, used, source, assumed)
    records = units.FILES[name][1] if rel.startswith(UNITS_DIR) else []
    differ = [{"name": n, "code": u} for n, u, _ in records
              if u not in (None, 0) and u != (declared or 0) and not n.startswith("*")]
    if differ:
        out["blocks"] = differ
    warnings = []
    if assumed:
        why = "Dosya birim bildirmiyor" if declared in (None, 0) else f"Dosyanın bildirdiği birim kodu ({declared}) tanınmıyor"
        where = "metre" if source == "metres" else f"çizimin biriminde ({out['name']})"
        warnings.append({"code": "unit_assumed", "path": "fileUnit", "message":
                         f"{why}; değerler ölçeklenmeden {where} sayıldı. Dosyanın birimi başkaysa onu seçin (fileUnit)."})
    if source == "metres" and known is not None and known != 6:
        warnings.append({"code": "unit_ignored", "path": "fileUnit", "message":
                         f"Dosya birimini {units.AUTOCAD[known][1]} olarak bildiriyor; koordinat sistemi olan proje dosyayı "
                         "metre okur, değerler ölçeklenmedi. Ölçeklemek için dosyanın birimini seçin (fileUnit)."})
    if differ:
        listed = ", ".join(f"{b['name']} ({units.AUTOCAD.get(b['code'], (None, 'bilinmeyen birim'))[1]})" for b in differ)
        warnings.append({"code": "block_units_differ", "path": "fileUnit", "message":
                         f"Bazı blokların birimi dosyanınkinden başka: {listed}. Yerleştirmelerin ölçeği bu oranı taşır; "
                         "bloklar ayrıca ölçeklenmedi."})
    return out, warnings


def p(x: float, y: float) -> dict:
    return {"x": x, "y": y}


def objects(rel: str, used: int, target: str, explode: bool, block: str, lines: bool = False) -> dict:
    """The fields these cases look at of the objects a file gives, from slot 2 (the drawing's point is 1); `lines`:
    closed polylines are read as closed lines (docs/adr/0258 §1)."""
    name = rel.rsplit("/", 1)[1]
    k = lambda v: units.metres(v, used)  # noqa: E731
    line = lambda a, b, c, d: {"kind": "line", "layerId": target, "a": p(k(a), k(b)), "b": p(k(c), k(d))}  # noqa: E731
    if name in ("mm.dxf", "unitless.dxf", "no-header.dxf", "unknown-code.dxf"):
        out = {
            "2": line(0.0, 0.0, 4000.0, 0.0),
            "3": {"kind": "circle", "layerId": target, "c": p(k(1000.0), k(500.0)), "r": k(250.0)},
            "4": {"kind": "text", "layerId": target, "text": "Salon", "p": p(k(100.0), k(3000.0)), "height": k(250.0)},
            "5": {"kind": "text", "layerId": target, "text": "Not kutusu", "p": p(k(100.0), k(2500.0)),
                  "height": k(200.0), "boxWidth": k(3000.0)},
        }
        # Exploded, the door's line (on the block's layer 0) is on the insert's layer, placed by it.
        out["6"] = (line(1000.0, 0.0, 1900.0, 0.0) if explode else
                    {"kind": "insert", "layerId": target, "block": f"$blockOf:{block}", "p": p(k(1000.0), k(0.0)),
                     "scale": 1.0})
        return out
    if name == "inches.dxf":
        return {"2": line(0.0, 0.0, 100.0, 0.0),
                "3": {"kind": "circle", "layerId": target, "c": p(k(10.0), k(10.0)), "r": k(5.0)}}
    if name == "us-feet.dxf":
        return {"2": line(0.0, 0.0, 3937.0, 0.0)}
    if name == "block-units.dxf":
        return {"2": line(0.0, 0.0, 12.5, 0.0),
                "3": {"kind": "insert", "layerId": target, "block": "$blockOf:PANO", "p": p(k(5.0), k(5.0)), "scale": 0.001}}
    if name == "micrometres.dxf":
        return {"2": line(0.0, 0.0, 2_500_000.0, 0.0)}
    if name == "losses.dxf":
        return {"2": line(0.0, 0.0, 2500.0, 0.0)}
    if name == "closed.dxf":
        pts = lambda ps: [p(k(x), k(y)) for x, y in ps]  # noqa: E731
        wide = lambda ws: [[k(a), k(b)] for a, b in ws]  # noqa: E731
        # An area has no widths (said in the report); an open line's last vertex none.
        room = {"kind": "polyline", "closed": True} if lines else {"kind": "polygon"}
        frame = ({"kind": "polyline", "closed": True, "widths": wide(FRAME_WIDTHS_MM)} if lines
                 else {"kind": "polygon"})
        return {
            "2": {**room, "layerId": target, "pts": pts(SQUARE_MM)},
            "3": {"kind": "polyline", "layerId": target, "pts": pts(WALL_MM),
                  "widths": wide([(500.0, 500.0), (500.0, 500.0), (0.0, 0.0)])},
            "4": {**frame, "layerId": target, "pts": pts(FRAME_MM)},
        }
    raise ValueError(name)


def counts(rel: str, explode: bool) -> tuple[int, int]:
    """The objects a file gives and the block definitions it brings."""
    name = rel.rsplit("/", 1)[1]
    if name in ("mm.dxf", "unitless.dxf", "no-header.dxf", "unknown-code.dxf"):
        return 5, 0 if explode else 1
    return {"inches.dxf": (2, 0), "us-feet.dxf": (1, 0), "block-units.dxf": (2, 1), "micrometres.dxf": (1, 0),
            "losses.dxf": (1, 0), "empty.dxf": (0, 0), "closed.dxf": (3, 0)}[name]


def completed(rel: str, doc: dict, project: str, *, file_unit: str | None = None, explode: bool = False,
              layers: dict | None = None, group_name: str | None = None, op: str = "execute",
              closed: str | None = None) -> tuple[dict, dict]:
    """The step importing `rel` into `doc` and what it gives: its result, and what the drawing then holds."""
    name = rel.rsplit("/", 1)[1]
    text = text_of(rel).encode()
    unit, warnings = decision(rel, project, file_unit)
    if rel == f"{IMPORT_DIR}/losses.dxf":
        warnings.append({"code": "import_losses", "message":
                         "Okuyucu dosyanın 1 öğesini alamadı (3DSOLID); nedenleri report.skipped'te."})
    n, blocks = counts(rel, explode)
    # The file's one layer with objects is DUVAR.
    taken = tree_ids(doc)
    made = []
    routed = {fold(k): v for k, v in (layers or {}).items()}
    if "DUVAR" in routed:
        target = routed["DUVAR"]
    else:
        same = [l for l in doc["layers"] if l["type"] == "layer" and fold(l["name"]) == "DUVAR"]
        target = same[0]["id"] if same else fresh_id("DUVAR", taken)
        if not same:
            made.append({"id": target, "name": "DUVAR"})
    group_id = None
    if made:
        g = name if group_name is None else group_name.strip()
        if g:
            found = [n_ for n_ in doc["layers"] if n_["type"] == "group" and fold(n_["name"]) == fold(g)]
            group_id = found[0]["id"] if found else fresh_id(g, taken)
    renamed = []
    block = "PANO" if name == "block-units.dxf" else "KAPI"
    if blocks and doc.get("blocks") and block == "KAPI":
        block = "KAPI (2)"
        renamed = [{"from": "KAPI", "to": block}]
    # Closed polylines: as asked, else a CAD project's closed lines, any other's areas (docs/adr/0258 §1).
    choice = closed or ("line" if doc["settings"].get("workspace") == "cad" else "area")
    output = {
        "file": name, "bytes": len(text), "sha256": hashlib.sha256(text).hexdigest(), "unit": unit,
        "closedPolylines": choice,
        "objects": n, "targets": {"DUVAR": target}, "layers": made, "blocks": blocks,
        "textStyles": 0, "dimensionStyles": 0, "report": "$any", "revision": "$current",
    }
    if group_id:
        output["group"] = group_id
    if renamed:
        output["renamed"] = renamed
    step = {"op": op, "input": {"path": f"$fixture:{rel}"}, "result": {"status": "completed", "output": output,
                                                                        "warnings": warnings}}
    if file_unit:
        step["input"]["fileUnit"] = file_unit
    if explode:
        step["input"]["explodeBlocks"] = True
    if closed:
        step["input"]["closedPolylines"] = closed
    if layers:
        step["input"]["layers"] = layers
    if group_name is not None:
        step["input"]["group"] = group_name
    expect = {}
    if op == "execute":
        expect = {"ids": [1, *range(2, 2 + n)], "fields": objects(rel, unit["used"], target, explode, block,
                                                                     choice == "line"),
                  "revision": "changed", "canUndo": True}
        if made:
            nodes = {m["id"]: {"name": m["name"], "type": "layer", "parent": group_id} for m in made}
            if group_id and group_id not in tree_ids(doc):
                nodes[group_id] = {"name": name if group_name is None else group_name.strip(), "type": "group",
                                   "parent": None}
            expect["nodes"] = nodes
        if blocks and not explode:
            expect["blockIds"] = {block: "new"}
    else:
        expect = {"ids": [1], "revision": "same", "canUndo": False}
    step["expect"] = expect
    return step, output


def failed(path_input: dict, code: str, message: str, path: str, kind: str = "failed", **extra) -> dict:
    error = {"code": code, "message": message, "path": path, **extra}
    return {"op": "execute", "input": path_input, "result": {"status": kind, "error": error},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}


MM = f"{UNITS_DIR}/mm.dxf"


def cases() -> list:
    out = []

    def case(name: str, steps: list, doc: dict | None = None) -> None:
        c = {"name": name, "steps": steps}
        if doc is not None:
            c["setup"] = doc
        out.append(c)

    m = setup()
    # The unit (§1).
    case("mm DXF metrelik projeye: dosyanın bildirdiği mm, nesneler metrede; DUVAR çizimin Duvar katmanına",
         [completed(MM, m, "m")[0]])
    case("mm DXF mm projesine: aynı metreler, sayılar aynen", [completed(MM, setup("mm"), "mm")[0]], setup("mm"))
    case("mm DXF koordinat sistemli projeye: metre sayılır, ölçeklenmez, unit_ignored söyler",
         [completed(MM, setup(None), "crs")[0]], setup(None))
    case("mm DXF koordinat sistemli projeye, birimi seçilerek: mm'den metreye çevrilir",
         [completed(MM, setup(None), "crs", file_unit="mm")[0]], setup(None))
    case("Birim bildirmeyen DXF mm projesine: sayılar projenin biriminde, unit_assumed",
         [completed(f"{UNITS_DIR}/unitless.dxf", setup("mm"), "mm")[0]], setup("mm"))
    case("Birim bildirmeyen DXF koordinat sistemli projeye: metre sayılır, unit_assumed",
         [completed(f"{UNITS_DIR}/unitless.dxf", setup(None), "crs")[0]], setup(None))
    case("Birim bildirmeyen DXF, santimetre seçilerek",
         [completed(f"{UNITS_DIR}/unitless.dxf", m, "m", file_unit="cm")[0]])
    case("Başlığı olmayan DXF cm projesine: projenin cm'si",
         [completed(f"{UNITS_DIR}/no-header.dxf", setup("cm"), "cm")[0]], setup("cm"))
    case("Tanınmayan birim kodu (99): projenin birimi, unit_assumed kodu söyler",
         [completed(f"{UNITS_DIR}/unknown-code.dxf", m, "m")[0]])
    case("İnç DXF metrelik projeye: 25,4 mm'lik inç", [completed(f"{UNITS_DIR}/inches.dxf", m, "m")[0]])
    case("ABD ölçme fiti bildiren DXF, uluslararası fit seçilerek",
         [completed(f"{UNITS_DIR}/us-feet.dxf", m, "m", file_unit="ft")[0]])
    case("Bloğu milimetrede metrelik DXF: block_units_differ; *Model_Space söylenmez",
         [completed(f"{UNITS_DIR}/block-units.dxf", m, "m")[0]])
    case("Mikrometre bildiren DXF koordinat sistemli projeye: AutoCAD'in tanıdığı birim, unit_ignored",
         [completed(f"{UNITS_DIR}/micrometres.dxf", setup(None), "crs")[0]], setup(None))
    # Plan, validate, undo.
    case("plan dosyayı okur, hiçbir şey yazmaz, execute'un aynısını söyler", [completed(MM, m, "m", op="plan")[0]])
    case("validate", [{"op": "validate", "input": {"path": f"$fixture:{MM}"},
                       "result": {"status": "completed", "output": None, "warnings": []},
                       "expect": {"ids": [1], "revision": "same", "canUndo": False}}])
    step, _ = completed(MM, m, "m")
    case("Tek geri alma adımı: “DXF: mm.dxf”, geri alınca nesneler ve blok gider",
         [step, {"op": "undo", "returns": "DXF: mm.dxf", "expect": {"ids": [1], "canUndo": False, "canRedo": True}}])
    # Layers and groups (§3).
    bare = setup(duvar=None)
    case("Çizimde aynı adlı katman yok: yeni DUVAR katmanı dosyanın adındaki grupta", [completed(MM, bare, "m")[0]], bare)
    case("group boş: yeni katman ağacın üstünde", [completed(MM, bare, "m", group_name="")[0]], bare)
    with_group = setup(duvar=None, mimari=True)
    case("group var olan grubu adıyla bulur (büyük küçük harf ayırmadan)",
         [completed(MM, with_group, "m", group_name="MİMARİ")[0]], with_group)
    case("group yoksa açılır", [completed(MM, bare, "m", group_name="Mimari")[0]], bare)
    case("layers DUVAR'ı Çizim katmanına yönlendirir (kaynak adı büyük küçük harf ayırmadan)",
         [completed(MM, m, "m", layers={"Duvar": "cizim"})[0]])
    case("explodeBlocks: bloklar patlatılır, tanım gelmez; kapının çizgisi yerleştirmenin katmanında",
         [completed(MM, m, "m", explode=True)[0]])
    with_kapi = setup(kapi=True)
    case("Çizimde KAPI bloğu var: gelen blok “KAPI (2)” adıyla, renamed söyler",
         [completed(MM, with_kapi, "m")[0]], with_kapi)
    case("Okuyucunun alamadığı nesne (3DSOLID): import_losses",
         [completed(f"{IMPORT_DIR}/losses.dxf", m, "m")[0]])
    # Closed and wide polylines (docs/adr/0258).
    closed_dxf = f"{IMPORT_DIR}/closed.dxf"
    case("Kapalı ve genişlikli çoklu çizgiler CAD projesine: kapalı çizgi kalır, genişlikleri metrede",
         [completed(closed_dxf, m, "m")[0]])
    crs = setup(None)
    case("Kapalı çoklu çizgiler koordinat sistemli (CBS) projeye: alan olur, genişliği alınmaz; açık çizgi genişliğiyle",
         [completed(closed_dxf, crs, "crs")[0]], crs)
    case("closedPolylines line: CBS projesinde de kapalı çizgi, genişliğiyle",
         [completed(closed_dxf, crs, "crs", closed="line")[0]], crs)
    case("closedPolylines area: CAD projesinde de alan",
         [completed(closed_dxf, m, "m", closed="area")[0]])
    # Refusals, in the ADR's order.
    case("Göreli yol: invalid_file_path", [failed({"path": "mm.dxf"}, "invalid_file_path",
         "“mm.dxf” mutlak bir dosya yolu değil; okunacak dosyanın tam yolunu verin (/klasör/ad.dxf).", "path")])
    case("Klasör: invalid_file_path", [failed({"path": f"$fixture:{UNITS_DIR}"}, "invalid_file_path",
         f"$fixture:{UNITS_DIR} bir klasör; okunacak DXF dosyasının adını da verin.", "path")])
    case("Geçersiz beklenen sürüm: invalid_revision", [failed(
        {"path": f"$fixture:{MM}", "expectedRevision": "on iki"}, "invalid_revision",
        "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
        "komutun planından alın.", "expectedRevision")])
    case("Çizim değişmiş: revision_conflict, hiçbir şey okunmaz", [failed(
        {"path": f"$fixture:{MM}", "expectedRevision": "99"}, "revision_conflict",
        "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden "
        "hazırlayın.", "expectedRevision", kind="conflict", revision="$current")])
    case("Dosya yok: file_not_found", [failed({"path": f"$fixture:{UNITS_DIR}/yok.dxf"}, "file_not_found",
         f"$fixture:{UNITS_DIR}/yok.dxf yok; dosyanın yolunu denetleyin.", "path")])
    case("DXF değil: import_unreadable (okuyucunun nedeniyle)", [failed(
        {"path": f"$fixture:{UNITS_DIR}/cases.json"}, "import_unreadable", "$any", "path")])
    case("layers olmayan katmana: layer_not_found", [failed(
        {"path": f"$fixture:{MM}", "layers": {"DUVAR": "yok"}}, "layer_not_found",
        "“yok” kimlikli katman çizimde yok; “DUVAR” katmanının nesnelerini var olan bir katmanın kimliğine yönlendirin.",
        "layers.DUVAR")])
    case("layers bir gruba: not_a_layer", [failed(
        {"path": f"$fixture:{MM}", "layers": {"DUVAR": "grup"}}, "not_a_layer",
        "“Grup” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı verin.", "layers.DUVAR")])
    case("layers kilitli katmana: layer_locked", [failed(
        {"path": f"$fixture:{MM}", "layers": {"DUVAR": "kilitli"}}, "layer_locked",
        "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir katman verin.", "layers.DUVAR")])
    locked = setup(duvar_locked=True)
    case("Aynı adlı katman kilitli: layer_locked", [failed(
        {"path": f"$fixture:{MM}"}, "layer_locked",
        "Dosyanın “DUVAR” katmanı çizimdeki aynı adlı “Duvar” katmanına gider ama o katman kilitli. Kilidi açın, katmanı "
        "skipLayers ile dışarıda bırakın ya da layers ile başka bir katmana yönlendirin.", "layers")], locked)
    case("Bütün katmanlar dışarıda: nothing_to_import", [failed(
        {"path": f"$fixture:{MM}", "skipLayers": ["duvar"]}, "nothing_to_import",
        "mm.dxf dosyasının seçilen katmanlarında nesne yok; skipLayers'ı denetleyin.", "skipLayers")])
    case("Nesnesi olmayan DXF: nothing_to_import", [failed(
        {"path": f"$fixture:{IMPORT_DIR}/empty.dxf"}, "nothing_to_import", "empty.dxf dosyasında alınacak nesne yok.",
        "path")])
    return out


def build() -> tuple[dict, str]:
    doc = {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "file.import.dxf",
        "commandVersion": 1,
        "title": "DXF içe aktar",
        "note": "Written by scripts/fixtures/dxf_import_command_cases.py from docs/adr/0257 and the DXF reference; do not "
                "edit by hand. The files are fixtures/formats/v1/dxf-units (dxf_units_cases.py) and dxf-import.",
        "setup": setup(),
        "cases": cases(),
    }
    return EXTRA, json.dumps(doc, ensure_ascii=False, indent=2) + "\n"


def main() -> int:
    files, text = build()
    folder = ROOT / "fixtures" / IMPORT_DIR
    want = {**{folder / n: t for n, t in files.items()}, OUT: text}
    if "--check" in sys.argv[1:]:
        bad = [str(p.relative_to(ROOT)) for p, t in want.items() if not p.exists() or p.read_text(encoding="utf-8") != t]
        if bad:
            print(f"file.import.dxf durumları yeniden üretilenle aynı değil: {', '.join(bad)} (betiği --check olmadan "
                  "çalıştırın)", file=sys.stderr)
            return 1
        print(f"file.import.dxf durumları güncel: {len(json.loads(text)['cases'])} durum.")
        return 0
    folder.mkdir(parents=True, exist_ok=True)
    for p, t in want.items():
        p.write_text(t, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(json.loads(text)['cases'])} durum) ve {folder.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
