#!/usr/bin/env python3
"""`file.import.pdf`'s shared cases (docs/adr/0352; plan/cad.md CAD-IO-21–26).

    python3 scripts/fixtures/pdf_import_cases.py           # writes fixtures/commands/v1/file.import.pdf.json
    python3 scripts/fixtures/pdf_import_cases.py --check   # writes nothing; compares

No KentOS code: the pages are the model `scripts/fixtures/pdf_cases.py` drew the files from
(fixtures/pdf/v1/cases.json, written beside the files, never read back from them), and the import is ADR 0352's rule
written here again:

- a page point p (points, from the page's lower left corner as it shows) goes to at + p · 0.0254/72 · scale;
- the reader's points keep a curve within 0.01 pt; the import flattens each curve again at its tolerance (paper mm ×
  72/25.4 points): ceil(sqrt(0.75 d / tol)) pieces between 1 and 512, d the control points' larger distance from the
  chord (from the start when the chord has no length), at even steps of t;
- `nurbs`: a run of curves end to end is a cubic NURBS curve, its control points end to end, knots 0 0 0 0, 1 1 1, …,
  m m m m, `closed` when it ends where it starts; the straight runs between are lines and polylines (a closed part's
  closing edge is straight);
- a fill is an area of each part, flattened, its closing point and repeated points dropped, at least three points;
  white is the paper and is counted, not taken; a stroke is a line (two points, open) or a polyline (`closed` when its
  last point is its first or the part closes); black is the drawing's ink (no colour); the width on paper in mm rounded
  to hundredths (none for a hairline); a dash is the line type `dashed`;
- a text is a text at its start, height · scale · 0.0254/72, turn in degrees;
- in a region (mm, its edges in) a part is taken when every point the reader gives is in it, a text when its start is;
- objects go in order: each path's parts, each part's area then its stroke, then the texts; layers: what is in no group
  is “PDF”, then the page's groups; a layer goes to the one `layers` names, else to the drawing's of the same name, else
  to a new one (hidden when the PDF's default view hides it) in a group named after the file; ids `import-<name>`.
"""
import hashlib
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PDF = ROOT / "fixtures" / "pdf" / "v1"
OUT = ROOT / "fixtures" / "commands" / "v1" / "file.import.pdf.json"
PT = 0.0254 / 72.0
MM = 72.0 / 25.4
FLAT = 0.01
NOTES = [
    "PDF yazdırılanı taşır, çizimi değil: ölçüler, kılavuzlar, taramalar ve tablolar çizgileri ve yazılarıyla geldi; ilişkileri ve değerleri yok.",
    "Ölçüler kâğıttan okunan çizgilerdir: kaynağın kesinliğini değil, basılmış sayfanınkini taşır.",
]
MODEL = {c["file"]: c for c in json.loads((PDF / "cases.json").read_text(encoding="utf-8"))["cases"]}


def flatten(c, tol):
    p0, p1, p2, p3 = c
    dx, dy = p3[0] - p0[0], p3[1] - p0[1]
    ln = math.sqrt(dx * dx + dy * dy)

    def d(p):
        if ln < 1e-12:
            return math.hypot(p[0] - p0[0], p[1] - p0[1])
        return abs((p[0] - p0[0]) * dy - (p[1] - p0[1]) * dx) / ln

    n = min(max(math.ceil(math.sqrt(max(d(p1), d(p2)) * 0.75 / tol)), 1), 512)
    out = []
    for i in range(1, n + 1):
        t = i / n
        u = 1.0 - t
        a, b, cc, e = u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t
        out.append([a * p0[0] + b * p1[0] + cc * p2[0] + e * p3[0], a * p0[1] + b * p1[1] + cc * p2[1] + e * p3[1]])
    return out


def items(part):
    """A part's model as (point) and (curve) items in order."""
    return part["points"]


def points_at(part, tol):
    """The part's points with its curves flattened at `tol`."""
    out = []
    for it in items(part):
        if isinstance(it, dict):
            out.extend(flatten(it["curve"], tol))
        else:
            out.append(it)
    return out


def runs(part):
    """Straight runs and runs of curves, a closed part's closing edge straight."""
    out, straight = [], []
    for it in items(part):
        if isinstance(it, dict):
            c = it["curve"]
            if len(straight) >= 2:
                out.append(("straight", straight))
            straight = []
            if out and out[-1][0] == "curved" and out[-1][1][-1][3] == c[0]:
                out[-1][1].append(c)
            else:
                out.append(("curved", [c]))
            straight = [c[3]]
        else:
            straight.append(it)
    pts = items(part)
    first = pts[0] if pts and not isinstance(pts[0], dict) else None
    if part["closed"] and first is not None and straight and straight[-1] != first:
        straight.append(first)
    if len(straight) >= 2:
        out.append(("straight", straight))
    return out


class Place:
    def __init__(self, at, scale):
        self.at, self.k = at, PT * scale

    def p(self, q):
        return {"x": self.at[0] + q[0] * self.k, "y": self.at[1] + q[1] * self.k}


def color(c):
    return None if c == [0, 0, 0] else "#%02x%02x%02x" % tuple(c)


def look(e, c, weight, dashed):
    if c is not None:
        e["color"] = c
    if weight is not None:
        e["lineWeight"] = weight
    if dashed:
        e["lineType"] = "dashed"
    return e


def dedup(pts):
    out = []
    for p in pts:
        if not out or out[-1] != p:
            out.append(p)
    return out


def stroke_object(pts, closed, place):
    line = list(pts)
    if len(line) > 2 and line[0] == line[-1]:
        line.pop()
        closed = True
    line = dedup(line)
    if len(line) < 2:
        return None
    if len(line) == 2 and not closed:
        return {"kind": "line", "a": place.p(line[0]), "b": place.p(line[1])}
    e = {"kind": "polyline", "pts": [place.p(q) for q in line]}
    if closed:
        e["closed"] = True
    return e


def nurbs(pieces, place):
    ctrl = [place.p(pieces[0][0])]
    for c in pieces:
        ctrl.extend(place.p(q) for q in c[1:])
    m = len(pieces)
    knots = [0.0] * 4
    for k in range(1, m):
        knots += [float(k)] * 3
    knots += [float(m)] * 4
    e = {"kind": "nurbs", "degree": 3, "ctrl": ctrl, "knots": knots}
    if pieces[0][0] == pieces[-1][3]:
        e["closed"] = True
    return e


def round_half_up(x):
    return math.floor(x + 0.5)


def imported(file, page_no=1, *, region=None, scale=1.0, at=(0.0, 0.0), strokes=True, fills=True, texts=True,
             curves="polyline", tolerance=0.01, skip=()):
    """The objects (layer, entity) and the counts ADR 0352 takes from a page of the model."""
    page = MODEL[file]["pages"][str(page_no)]
    place = Place(at, scale)
    tol = tolerance * MM
    box = None
    if region is not None:
        (ax, ay), (bx, by) = region
        box = [min(ax, bx) * MM, min(ay, by) * MM, max(ax, bx) * MM, max(ay, by) * MM]

    def holds(q):
        return box[0] <= q[0] <= box[2] and box[1] <= q[1] <= box[3]

    objects, taken = [], {"lines": 0, "polylines": 0, "curves": 0, "areas": 0, "texts": 0}
    left = {"images": page["left"]["images"], "shadings": page["left"]["shadings"],
            "unreadable": page["left"]["unreadable"], "white": 0, "outside": 0, "truncated": page["left"]["truncated"]}
    kinds = {"line": "lines", "polyline": "polylines", "nurbs": "curves", "polygon": "areas", "text": "texts"}

    def name(i):
        return "PDF" if i is None else page["layers"][i]

    def push(layer, e):
        taken[kinds[e["kind"]]] += 1
        objects.append((layer, e))

    for path in page["paths"]:
        layer = name(path["layer"])
        fill = path["fill"] if fills else None
        stroke = path["stroke"] if strokes else None
        if fill is None and stroke is None:
            continue
        for part in path["parts"]:
            if box is not None and not all(holds(q) for q in points_at(part, FLAT)):
                left["outside"] += 1
                continue
            if fill is not None:
                if fill == [255, 255, 255]:
                    left["white"] += 1
                else:
                    ring = points_at(part, tol)
                    if len(ring) > 1 and ring[0] == ring[-1]:
                        ring.pop()
                    ring = dedup(ring)
                    if len(ring) >= 3:
                        push(layer, look({"kind": "polygon", "pts": [place.p(q) for q in ring]}, color(fill), None,
                                         False))
            if stroke is None:
                continue
            weight = round_half_up(stroke["width"] * 25.4 / 72.0 * 100.0) / 100.0 if stroke["width"] > 0 else None
            if weight == 0:
                weight = None

            def styled(e):
                return look(e, color(stroke["color"]), weight, stroke["dashed"])

            has_curve = any(isinstance(it, dict) for it in items(part))
            if curves == "nurbs" and has_curve:
                for kind, run in runs(part):
                    if kind == "straight":
                        e = stroke_object(run, False, place)
                        if e is not None:
                            push(layer, styled(e))
                    else:
                        push(layer, styled(nurbs(run, place)))
            else:
                e = stroke_object(points_at(part, tol), part["closed"], place)
                if e is not None:
                    push(layer, styled(e))
    if texts:
        for t in page["texts"]:
            if box is not None and not holds(t["p"]):
                left["outside"] += 1
                continue
            e = {"kind": "text", "p": place.p(t["p"]), "text": t["text"], "height": t["height"] * place.k,
                 "rotation": math.degrees(t["angle"])}
            push(name(t["layer"]), look(e, color(t["color"]), None, False))
    skipped = {fold(s) for s in skip}
    objects = [(l, e) for l, e in objects if fold(l) not in skipped]
    return page, objects, taken, left


def fold(s):
    return s.strip().upper().translate(str.maketrans("ÇĞİÖŞÜ", "CGIOSU"))


def slug(n):
    out = ""
    for c in fold(n).lower():
        if c.isascii() and (c.islower() or c.isdigit()):
            out += c
        elif not out.endswith("-"):
            out += "-"
    out = out.strip("-")
    return "import-" + (out or "katman")


def bounds(objects):
    xs, ys = [], []
    for _, e in objects:
        for key in ("a", "b", "p"):
            if key in e:
                xs.append(e[key]["x"])
                ys.append(e[key]["y"])
        for key in ("pts", "ctrl"):
            for q in e.get(key, []):
                xs.append(q["x"])
                ys.append(q["y"])
    return [min(xs), min(ys), max(xs), max(ys)] if xs else None


# ── The drawing the cases start from ──────────────────────────────────────

def layer(id_, name_, *, kind="layer", locked=False):
    return {"id": id_, "name": name_, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


SETUP = {
    "format": "kentos.document", "version": 1, "name": "PDF alma",
    "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                 "plotScale": 50, "workspace": "cad"},
    "origin": {"x": 0.0, "y": 0.0},
    "layers": [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True), layer("grup", "Grup", kind="group"),
               layer("yapi", "Yapılar")],
    "activeLayer": "cizim",
    "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": {"x": 5.0, "y": 5.0}}],
    "styles": {"items": [], "categories": []},
}
EXISTING = {fold(l["name"]): l["id"] for l in SETUP["layers"] if l["type"] == "layer"}


def file_facts(file):
    data = (PDF / file).read_bytes()
    return len(data), hashlib.sha256(data).hexdigest()


def fixture(file):
    return f"$fixture:pdf/v1/{file}"


def completed(file, inp, *, op="execute", page_no=1, layers_map=None, group=None, **kw):
    """A step that takes `file`'s page as the rule says: its result and what the drawing then holds."""
    page, objects, taken, left = imported(file, page_no, **kw)
    used = []
    for n in ["PDF"] + page["layers"]:
        if n not in used and any(l == n for l, _ in objects):
            used.append(n)
    layers_map = layers_map or {}
    group_name = file if group is None else group
    targets, made, nodes = {}, [], {}
    group_id = slug(group_name) if group_name and any(
        fold(n) not in {fold(k) for k in layers_map} and fold(n) not in EXISTING for n in used) else None
    for n in used:
        if n in layers_map:
            targets[n] = layers_map[n]
        elif fold(n) in EXISTING:
            targets[n] = EXISTING[fold(n)]
        else:
            lid = slug(n)
            targets[n] = lid
            made.append({"id": lid, "name": n})
            hidden = n in page["layers"] and page["hidden"][page["layers"].index(n)]
            nodes[lid] = {"name": n, "type": "layer", "parent": group_id, "visible": not hidden}
    if group_id is not None:
        nodes[group_id] = {"name": group_name, "type": "group", "parent": None}
    size, sha = file_facts(file)
    output = {"file": file, "bytes": size, "sha256": sha, "page": page_no, "pages": page["count"],
              "paper": [page["width"] / MM, page["height"] / MM], "objects": len(objects), "taken": taken,
              "left": left, "targets": targets, "layers": made, "notes": NOTES, "revision": "$current"}
    b = bounds(objects)
    if b is not None:
        output["bounds"] = b
    if group_id is not None:
        output["group"] = group_id
    warnings = []
    words = []
    if left["images"]:
        words.append(f"{left['images']} resim")
    if left["shadings"]:
        words.append(f"{left['shadings']} gölgeleme")
    if left["unreadable"]:
        words.append(f"{left['unreadable']} okunamayan harf (yazı tipinin harf eşlemesi yok)")
    if left["white"]:
        words.append(f"{left['white']} beyaz dolgu (kâğıt sayıldı)")
    if left["truncated"]:
        words.append("sayfanın sınırı aşan geri kalanı")
    if words:
        warnings.append({"code": "import_losses", "message": f"{file} sayfasından alınmadı: {', '.join(words)}.",
                         "path": "path"})
    step = {"op": op, "input": inp, "result": {"status": "completed", "output": output, "warnings": warnings},
            "within": 1e-9}
    if op == "plan":
        step["expect"] = {"ids": [1], "revision": "same", "canUndo": False}
        return step
    ids = list(range(1, 2 + len(objects)))
    fields = {}
    for k, (l, e) in enumerate(objects):
        fields[str(2 + k)] = {**e, "layerId": targets[l], "attrs": {}}
    step["expect"] = {"ids": ids, "revision": "changed", "canUndo": True, "fields": fields, "nodes": nodes}
    return step


def refused(inp, code, message, path):
    return {"op": "execute", "input": inp,
            "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
            "expect": {"ids": [1], "revision": "same", "canUndo": False}}


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def build():
    f = fixture
    cases = [
        case("Bütün sayfa: çizgiler, dolgular ve yazılar; siyah çizimin mürekkebi, kalınlık kâğıtta mm",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf")})),
        case("1:50 basılmış sayfa çizimin metresine büyür, sol alt köşesi verilen noktaya gelir",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf"), "scale": 50, "at": {"x": 1000.0, "y": 2000.0}},
                       scale=50.0, at=(1000.0, 2000.0))),
        case("Yalnız çizgiler",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf"), "fills": False, "texts": False},
                       fills=False, texts=False)),
        case("Yalnız dolgular",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf"), "strokes": False, "texts": False},
                       strokes=False, texts=False)),
        case("Yalnız yazılar",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf"), "strokes": False, "fills": False},
                       strokes=False, fills=False)),
        case("Bölge: yalnız bütün noktaları içinde kalan parçalar ve başı içindeki yazılar",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf"),
                                        "region": {"a": {"x": 0.0, "y": 0.0}, "b": {"x": 80.0, "y": 80.0}}},
                       region=((0.0, 0.0), (80.0, 80.0)))),
        case("Bölgenin köşeleri ters verilse de aynı dikdörtgen",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf"),
                                        "region": {"a": {"x": 80.0, "y": 80.0}, "b": {"x": 0.0, "y": 0.0}}},
                       region=((80.0, 80.0), (0.0, 0.0)))),
        case("Plan dosyayı okur, kutuyu ve katmanları verir, hiçbir şey yazmaz",
             completed("cizgiler.pdf", {"path": f("cizgiler.pdf")}, op="plan")),
        case("Bütün alınanlar tek geri alma adımı “PDF: ad.pdf”",
             completed("egri.pdf", {"path": f("egri.pdf")}),
             {"op": "undo", "returns": "PDF: egri.pdf", "expect": {"ids": [1], "canUndo": False, "canRedo": True}}),
        case("Geçersiz beklenen sürüm: invalid_revision",
             refused({"path": f("cizgiler.pdf"), "expectedRevision": "on iki"}, "invalid_revision",
                     "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
                     "expectedRevision")),
        case("Çizim değişmiş: revision_conflict, hiçbir şey okunmaz",
             {"op": "execute", "input": {"path": f("cizgiler.pdf"), "expectedRevision": "99"},
              "result": {"status": "conflict", "error": {
                  "code": "revision_conflict",
                  "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                  "path": "expectedRevision", "revision": "$current"}},
              "expect": {"ids": [1], "revision": "same", "canUndo": False}}),
        case("İsteğe bağlı içerik grupları katman: aynı adlı katmana, gizli grup gizli yeni katmana; resim ve gölgeleme söylenir",
             completed("katmanlar.pdf", {"path": f("katmanlar.pdf")})),
        case("skipLayers bir grubu dışarıda bırakır",
             completed("katmanlar.pdf", {"path": f("katmanlar.pdf"), "skipLayers": ["kotlar"]}, skip=("kotlar",))),
        case("layers bir grubu var olan katmana yönlendirir",
             completed("katmanlar.pdf", {"path": f("katmanlar.pdf"), "layers": {"Kotlar": "cizim"}},
                       layers_map={"Kotlar": "cizim"})),
        case("Boş grup adı yeni katmanları ağacın üstüne koyar",
             completed("katmanlar.pdf", {"path": f("katmanlar.pdf"), "group": ""}, group="")),
        case("Adı verilen grup",
             completed("katmanlar.pdf", {"path": f("katmanlar.pdf"), "group": "Altlıklar"}, group="Altlıklar")),
        case("Eğri varsayılan toleransla (0,01 mm) çoklu çizgi",
             completed("egri.pdf", {"path": f("egri.pdf")})),
        case("Eğri 0,5 mm toleransla daha az köşe",
             completed("egri.pdf", {"path": f("egri.pdf"), "tolerance": 0.5}, tolerance=0.5)),
        case("Eğri kesin NURBS: düz kenarlar çizgi",
             completed("egri.pdf", {"path": f("egri.pdf"), "curves": "nurbs"}, curves="nurbs")),
        case("Eğriler çoklu çizgi: kapalı daire, karışık yol, dolgulu yaprak, beyaz dolgu kâğıt",
             completed("egriler.pdf", {"path": f("egriler.pdf")})),
        case("Eğriler NURBS: daire kapalı tek eğri, karışık yolda çizgi–eğri–çizgi, yaprağın dolgusu alan",
             completed("egriler.pdf", {"path": f("egriler.pdf"), "curves": "nurbs"}, curves="nurbs")),
        case("NURBS, 1:100 ve dolgular olmadan",
             completed("egriler.pdf", {"path": f("egriler.pdf"), "curves": "nurbs", "scale": 100, "fills": False},
                       curves="nurbs", scale=100.0, fills=False)),
        case("Türkçe harfli yazı",
             completed("turkce.pdf", {"path": f("turkce.pdf")})),
        case("Döndürülmüş sayfanın ikinci sayfası",
             completed("donuk.pdf", {"path": f("donuk.pdf"), "page": 2}, page_no=2)),
        case("Döndürülmüş sayfanın üçüncü sayfası",
             completed("donuk.pdf", {"path": f("donuk.pdf"), "page": 3}, page_no=3)),
        case("Artımlı güncellenmiş dosya son hâliyle",
             completed("guncelleme.pdf", {"path": f("guncelleme.pdf")})),
        case("Göreli yol: invalid_file_path",
             refused({"path": "plan.pdf"}, "invalid_file_path",
                     "“plan.pdf” okunacak bir dosyanın mutlak yolu değil; PDF dosyasının tam yolunu verin (/klasör/ad.pdf).",
                     "path")),
        case("Ölçek sıfır: invalid_scale",
             refused({"path": f("cizgiler.pdf"), "scale": 0}, "invalid_scale",
                     "Ölçek 1:0 olamaz; sayfanın basıldığı ölçeğin paydasını verin (1 ile 1 000 000 arası, 50 → 1:50).",
                     "scale")),
        case("Eksi tolerans: invalid_tolerance",
             refused({"path": f("cizgiler.pdf"), "tolerance": -1}, "invalid_tolerance",
                     "Eğrinin toleransı -1 mm olamaz; kâğıtta sıfırdan büyük, en çok 100 mm bir uzaklık verin.",
                     "tolerance")),
        case("Eni olmayan bölge: invalid_pdf_region",
             refused({"path": f("cizgiler.pdf"), "region": {"a": {"x": 10, "y": 0}, "b": {"x": 10, "y": 50}}},
                     "invalid_pdf_region",
                     "Bölgenin eni ya da boyu yok; sayfada iki karşı köşesi farklı bir dikdörtgen verin (milimetre).",
                     "region")),
        case("Sayfanın dışındaki bölge: invalid_pdf_region",
             refused({"path": f("cizgiler.pdf"), "region": {"a": {"x": 300, "y": 0}, "b": {"x": 400, "y": 50}}},
                     "invalid_pdf_region",
                     "Bölge sayfanın dışında; sayfa 209.9 × 297.0 mm, bölgeyi onun sol alt köşesinden milimetreyle verin.",
                     "region")),
        case("Olmayan dosya: file_not_found",
             refused({"path": f("yok.pdf")}, "file_not_found", "$any", "path")),
        case("PDF olmayan dosya: import_unreadable",
             refused({"path": f("pdf-degil.pdf")}, "import_unreadable", "$any", "path")),
        case("Şifreli PDF: import_unreadable",
             refused({"path": f("sifreli.pdf")}, "import_unreadable",
                     "sifreli.pdf PDF olarak okunamadı: PDF şifreli; şifresi açılmış bir kopyasını verin.", "path")),
        case("Olmayan sayfa: pdf_page_not_found",
             refused({"path": f("donuk.pdf"), "page": 4}, "pdf_page_not_found",
                     "donuk.pdf dosyasında 4. sayfa yok; 1 ile 3 arasında bir sayfa verin.", "page")),
        case("Olmayan hedef katman: layer_not_found",
             refused({"path": f("cizgiler.pdf"), "layers": {"PDF": "yok"}}, "layer_not_found",
                     "“yok” kimlikli katman çizimde yok; “PDF” katmanının nesnelerini var olan bir katmanın kimliğine yönlendirin.",
                     "layers.PDF")),
        case("Hedef grup: not_a_layer",
             refused({"path": f("cizgiler.pdf"), "layers": {"PDF": "grup"}}, "not_a_layer",
                     "“Grup” bir grup; nesneler yalnız katmana konur. Grubun altındaki bir katmanı verin.",
                     "layers.PDF")),
        case("Kilitli hedef katman: layer_locked",
             refused({"path": f("cizgiler.pdf"), "layers": {"PDF": "kilitli"}}, "layer_locked",
                     "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir katman verin.",
                     "layers.PDF")),
        case("Yazısız sayfada yalnız yazı: nothing_to_import",
             refused({"path": f("egri.pdf"), "strokes": False}, "nothing_to_import",
                     "egri.pdf dosyasının 1. sayfasında seçilenlerden alınacak nesne yok; bölgeyi, katmanları ve alınacakları denetleyin.",
                     "path")),
        case("Bütün katmanlar dışarıda: nothing_to_import",
             refused({"path": f("cizgiler.pdf"), "skipLayers": ["PDF"]}, "nothing_to_import",
                     "cizgiler.pdf dosyasının 1. sayfasında seçilenlerden alınacak nesne yok; bölgeyi, katmanları ve alınacakları denetleyin.",
                     "skipLayers")),
    ]
    return json.dumps({
        "format": "kentos.command-cases", "version": 1, "command": "file.import.pdf", "commandVersion": 1,
        "title": "PDF içe aktar: sayfanın çizgileri, dolguları ve yazıları çizimin nesneleri",
        "note": "ADR 0352. Sayfalar fixtures/pdf/v1/cases.json'daki modelden (pdf_cases.py'nin çizdiği), içe alma ADR'nin "
                "kuralıyla burada yeniden yazıldı; KentOS kodu yok. Bağımsız başvuru: scripts/fixtures/pdf_import_cases.py.",
        "setup": SETUP, "cases": cases}, ensure_ascii=False, indent=2) + "\n"


def main():
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"güncel değil: {OUT.relative_to(ROOT)}")
        print(f"{len(json.loads(text)['cases'])} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(json.loads(text)['cases'])} durum yazıldı")


main()
