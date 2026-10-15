"""Özellik kopyala: shared cases of cad.entities.match (docs/adr/0304).

    python3 scripts/fixtures/match_command_cases.py           # writes the file
    python3 scripts/fixtures/match_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.match.json. The ADR's rules, computed here without KentOS code:

- The properties named (none named: all eleven) of the object `source` go to the objects `uids`, as one undo step
  “Özellik kopyala”; the source among them is left out.
- The object's own: `layer` (its layer), `color`, `lineType`, `lineTypeScale`, `transparency`, `symbol`: the source's
  value, or none when the source has none (the target's own is taken away); `lineWeight` the same, but only from an
  object drawn with lines to one drawn with lines (a point, a text, a dimension, a hatch, an insert, an image, a raster
  or a point cloud is not).
- The kind's, only between objects of one kind: `text` a text's face (style, font, bold, italic, slant), height and
  width factor; `dimension` a dimension's look (style, arrowheads, sizes, value's writing, line colours and weights),
  value height and background; `hatch` a hatch's pattern; `leader` a leader's arrowhead, its size, the note's height and
  background. Everything else stays (a text's words and turn, a dimension's points, a hatch's region, a leader's line).
- An object on a locked layer is left out with a warning (all locked: refused); moving to the source's layer keeps the
  layer's rules: a locked layer refuses (said at `source`), a hidden one warns (at `source`).
- The objects that change, in the input's order, are the output; when none changes nothing is written.
- The checks' order: the source's id, the ids (none, then each), the properties (an empty list), the revision, the
  source in the drawing, the targets in the drawing, the source alone, the locked targets, then the layer.
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.match.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
LABEL = "Özellik kopyala"
ALL = ["layer", "color", "lineType", "lineTypeScale", "lineWeight", "transparency", "symbol", "text", "dimension",
       "hatch", "leader"]
OWN = {"layer": "layerId", "color": "color", "lineType": "lineType", "lineTypeScale": "lineTypeScale",
       "transparency": "transparency", "symbol": "symbol"}
NOT_LINED = {"point", "text", "dimension", "hatch", "insert", "image", "raster", "pointcloud"}
FACE = ["textStyle", "font", "bold", "italic", "oblique"]
LOOK = ["dimStyle", "arrow", "arrowSize", "extOffset", "extBeyond", "textGap", "textPlace", "decimals", "unit", "prefix",
        "suffix", "font", "dimLineColor", "dimLineWeight", "dimLineType", "extColor", "extWeight", "extLineType",
        "textColor"]
KIND = {
    "text": ("text", FACE + ["height", "widthFactor"]),
    "dimension": ("dimension", LOOK + ["height", "mask"]),
    "hatch": ("hatch", ["pattern"]),
    "leader": ("leader", ["arrow", "arrowSize", "height", "mask"]),
}
GHOST = "0192f5a2-0000-7000-8000-00000000ffff"


def p(x, y):
    return {"x": x, "y": y}


def node(i, name, locked=False, visible=True):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def base(i, layer="cizim", **more):
    return {"id": i, "layerId": layer, "attrs": {}, **more}


SQUARE = [p(0, 0), p(10, 0), p(10, 10), p(0, 10)]
ENTITIES = [
    {"kind": "polygon", **base(1, "parsel", color="#e5484d", lineWeight=0.5, symbol="sys.parsel", lineType="dashed",
                               lineTypeScale=2, transparency=0.3, attrs={"Ada": "101"}), "pts": SQUARE},
    {"kind": "polygon", **base(2, color="#00ff00", label="P2"), "pts": [p(20, 0), p(30, 0), p(30, 10), p(20, 10)]},
    {"kind": "line", **base(3, lineWeight=0.25), "a": p(40, 0), "b": p(50, 0)},
    {"kind": "point", **base(4, color="#0000ff"), "p": p(60, 0)},
    {"kind": "text", **base(5, "yazi"), "p": p(0, 20), "text": "Başlık", "height": 2.5, "rotation": 0,
     "font": "plex-mono", "bold": True, "oblique": 15, "widthFactor": 0.8},
    {"kind": "text", **base(6), "p": p(20, 20), "text": "Not", "height": 1, "rotation": 30, "mask": True},
    {"kind": "dimension", **base(7), "a": p(0, 30), "b": p(10, 30), "offset": 2, "height": 1.5, "style": "aligned",
     "arrow": "open", "arrowSize": 1.2, "decimals": 3, "suffix": " m", "mask": True},
    {"kind": "dimension", **base(8), "a": p(20, 30), "b": p(30, 30), "offset": 1, "height": 1, "style": "linear",
     "angle": 0},
    {"kind": "hatch", **base(9), "ring": [p(0, 40), p(10, 40), p(10, 50), p(0, 50)],
     "pattern": {"type": "cross", "angle": 45, "spacing": 2}},
    {"kind": "hatch", **base(10), "ring": [p(20, 40), p(30, 40), p(30, 50), p(20, 50)],
     "pattern": {"type": "lines", "angle": 0, "spacing": 1.5}},
    {"kind": "leader", **base(11), "pts": [p(0, 60), p(4, 64), p(8, 64)], "text": "A", "height": 3, "rotation": 0,
     "arrow": "dot", "arrowSize": 2, "mask": True},
    {"kind": "leader", **base(12), "pts": [p(20, 60), p(24, 64), p(28, 64)], "text": "B", "height": 2, "rotation": 0},
    {"kind": "polyline", **base(13, "kilitli"), "pts": [p(0, 70), p(5, 70), p(5, 75)]},
    {"kind": "line", **base(14, "gizli", color="#123456"), "a": p(40, 70), "b": p(45, 70)},
    {"kind": "line", **base(15, "kapali", color="#abcdef"), "a": p(40, 80), "b": p(45, 80)},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
LAYER_NAMES = {"cizim": "Çizim", "parsel": "Parsel", "yazi": "Yazı", "kilitli": "Kilitli", "gizli": "Gizli",
               "kapali": "Kapalı"}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Özellik kopyala",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(0, 0),
        "layers": [node("cizim", "Çizim"), node("parsel", "Parsel"), node("yazi", "Yazı"),
                   node("kilitli", "Kilitli", locked=True), node("gizli", "Gizli", visible=False),
                   node("kapali", "Kapalı", locked=True)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


# ── The rules ──────────────────────────────────────────────────────────────────────────────────────────────────────

def lined(e):
    return e["kind"] not in NOT_LINED


def take(target, source, keys):
    for k in keys:
        if k in source:
            target[k] = copy.deepcopy(source[k])
        else:
            target.pop(k, None)


def matched(source, target, props):
    """The target after the properties of the source, or None when it does not change."""
    out = copy.deepcopy(target)
    for prop in props:
        if prop in OWN:
            take(out, source, [OWN[prop]])
        elif prop == "lineWeight":
            if lined(source) and lined(target):
                take(out, source, ["lineWeight"])
        else:
            kind, keys = KIND[prop]
            if source["kind"] == kind and target["kind"] == kind:
                take(out, source, keys)
    return None if out == target else out


class Refused(Exception):
    def __init__(self, code, message, path, status="failed"):
        super().__init__(message)
        self.code, self.message, self.path, self.status = code, message, path, status


def is_uid(text):
    return isinstance(text, str) and (text.startswith("$uidOf:") or text == GHOST)


def bad_uid(text, path):
    return Refused("invalid_uid", f"“{text}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                   "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da "
                   "çizimden alın.", path)


def not_found(uid, path):
    return Refused("entity_not_found", f"“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. "
                   "Var olan bir nesnenin kimliğini verin.", path)


def slot(uid):
    return int(uid.split(":")[1]) if uid.startswith("$uidOf:") else None


def answer(inp):
    """The command's result for `inp`: (the targets that change with their objects after, warnings)."""
    src = inp["source"]
    if not is_uid(src):
        raise bad_uid(src, "source")
    uids = inp["uids"]
    if not uids:
        raise Refused("no_entities", "Özellik verilecek nesne verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids")
    for i, u in enumerate(uids):
        if not is_uid(u):
            raise bad_uid(u, f"uids[{i}]")
    props = []
    for q in inp.get("properties", ALL):
        if q not in props:
            props.append(q)
    if not props:
        raise Refused("no_properties", "Kopyalanacak özellik seçilmedi. En az bir özellik verin ya da listeyi vermeyin (hepsi).",
                      "properties")
    rev = inp.get("expectedRevision")
    if rev is not None and not (rev.startswith("$") or rev.isdigit()):
        raise Refused("invalid_revision", f"Beklenen sürüm “{rev}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")
    if slot(src) is None:
        raise not_found(src, "source")
    source = BY_ID[slot(src)]
    found, seen = [], set()
    for i, u in enumerate(uids):
        if u in seen:
            continue
        seen.add(u)
        if slot(u) is None:
            raise not_found(u, f"uids[{i}]")
        found.append((i, slot(u)))
    found = [(i, s) for i, s in found if s != slot(src)]
    if not found:
        raise Refused("no_entities", "Kaynağın kendisi dışında özellik verilecek nesne yok. Başka nesneler verin.", "uids")
    locked = [(i, s) for i, s in found if BY_ID[s]["layerId"] in ("kilitli", "kapali")]
    open_ = [(i, s) for i, s in found if BY_ID[s]["layerId"] not in ("kilitli", "kapali")]
    warnings = []
    if not open_:
        i, s = locked[0]
        name = LAYER_NAMES[BY_ID[s]["layerId"]]
        raise Refused("layer_locked", f"“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden "
                      "açın.", f"uids[{i}]")
    if locked:
        warnings.append({"code": "layer_locked", "path": "uids", "message": f"{len(locked)} nesne kilitli katmanda olduğu "
                         "için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."})
    after = {s: matched(source, BY_ID[s], props) for _, s in open_}
    moved = [s for s in after if after[s] is not None and "layer" in props and BY_ID[s]["layerId"] != source["layerId"]]
    if moved and source["layerId"] in ("kilitli", "kapali"):
        raise Refused("layer_locked", f"“{LAYER_NAMES[source['layerId']]}” katmanı kilitli; nesneler ona taşınamaz. Kilidi "
                      "Katmanlar panelinden açın ya da başka bir katman seçin.", "source")
    if moved and source["layerId"] == "gizli":
        warnings.append({"code": "layer_hidden", "message": "“Gizli” katmanı gizli; taşınan nesneler görünmeyecek.",
                         "path": "source"})
    return [(s, after[s]) for _, s in open_ if after[s] is not None], warnings


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def capture(ids):
    return [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in ids]


def step(inp, op="execute"):
    try:
        changes, warnings = answer(inp)
    except Refused as r:
        return {"op": op, "input": inp, "result": {"status": r.status, "error": {"code": r.code, "message": r.message,
                                                                                 "path": r.path}},
                "expect": {"ids": IDS, "revision": "same", "canUndo": False}}
    if op == "validate":
        return {"op": op, "input": inp, "result": {"status": "completed", "output": None, "warnings": warnings},
                "expect": {"revision": "same"}}
    if op == "plan":
        return {"op": op, "input": inp, "result": {"status": "completed", "warnings": warnings, "output": {
            "changed": [e for _, e in changes], "created": [], "removed": [], "revision": "$current"}},
                "expect": {"revision": "same"}}
    out = {"op": op, "input": inp, "result": {"status": "completed", "warnings": warnings, "output": {
        "changed": [U(s) for s, _ in changes], "created": [], "removed": [], "revision": "$current"}}}
    if changes:
        out["expect"] = {"ids": IDS, "entities": {str(s): e for s, e in changes}, "revision": "changed", "canUndo": True}
    else:
        out["expect"] = {"ids": IDS, "revision": "same", "canUndo": False}
    return out


def case(name, inp, undo=False):
    s = step(inp)
    steps = [s]
    if undo:
        targets = [slot(u) for u in inp["uids"] if slot(u) is not None]
        steps.append({"op": "undo", "returns": LABEL, "expect": {"entities": {str(t): BY_ID[t] for t in targets}}})
    return {"name": name, "steps": steps}


def src(i, *targets, props=None, **more):
    inp = {"source": U(i), "uids": [U(t) if isinstance(t, int) else t for t in targets]}
    if props is not None:
        inp["properties"] = props
    inp.update(more)
    return inp


def cases():
    out = [
        case("Bütün özellikler: alan öbür alana katmanını, rengini, kalınlığını, çizgi tipini ve ölçeğini, saydamlığını ve "
             "sembolünü verir; öznitelik ve etiket kalır; tek adım “Özellik kopyala”, geri alınır", src(1, 2), undo=True),
        case("Yalnız katman ve renk", src(1, 2, props=["layer", "color"])),
        case("Çizgiye kalınlık da gider, noktaya gitmez (çizgiyle çizilmez); ikisi tek adımda", src(1, 3, 4), undo=True),
        case("Kaynağın kendi rengi yoksa hedefinki kalkar (katmana göre)", src(3, 2, props=["color"])),
        case("Kaynak çizgiyle çizilmiyorsa kalınlık verilmez: hiçbir şey değişmez, adım yazılmaz", src(4, 3, props=["lineWeight"])),
        case("Yazı yazıya yüzünü (yazı tipi, kalın, yatıklık), yüksekliğini ve genişlik çarpanını verir; sözleri, dönüşü ve "
             "zemini kalır", src(5, 6, props=["text"])),
        case("Yazının bütün özellikleri: katmanı da gider", src(5, 6)),
        case("Ölçü ölçüye görünüşünü (ok, ok boyu, basamak, sonek), değer yüksekliğini ve zeminini verir; noktaları ve türü kalır",
             src(7, 8, props=["dimension"])),
        case("Tarama taramaya desenini verir; bölgesi kalır", src(9, 10, props=["hatch"])),
        case("Kılavuz kılavuza okunu, ok boyunu, not yüksekliğini ve zeminini verir; çizgisi ve notu kalır",
             src(11, 12, props=["leader"])),
        case("Türün özellikleri yalnız aynı türe gider: yazının yazı özellikleri alana gitmez", src(5, 2, props=["text"])),
        case("Karışık hedefler: her biri alabildiğini alır", src(5, 6, 2, 3, props=["text", "color", "layer"])),
        case("Kaynak hedeflerin arasındaysa atlanır", src(1, 1, 2, props=["color"])),
        case("Kilitli katmandaki hedef uyarıyla atlanır", src(1, 13, 2, props=["color"])),
        case("Gizli katmandaki kaynağın katmanına taşınan nesneler uyarıyla", src(14, 2, 3, props=["layer"])),
        case("Kilitli katmandaki kaynaktan renk alınır", src(15, 3, props=["color"])),
        case("Kilitli katmandaki kaynağın katmanına taşınmaz: layer_locked, kaynakta", src(15, 3, props=["layer", "color"])),
        case("Zaten öyle olan nesne değişmez: hiçbir şey yazılmaz", src(3, 6, props=["color"])),
        case("Yinelenen özellik bir kez sayılır", src(1, 2, props=["color", "color"])),
        case("Boş özellik listesi: no_properties", src(1, 2, props=[])),
        case("Kaynağın kimliği UUID değil: invalid_uid, kaynakta", {"source": "on iki", "uids": [U(2)]}),
        case("Hedef verilmedi: no_entities", src(1)),
        case("Hedefin kimliği UUID değil: invalid_uid, yeriyle", src(1, 2, "on iki")),
        case("Kaynak çizimde yok: entity_not_found, kaynakta", {"source": GHOST, "uids": [U(2)]}),
        case("Hedef çizimde yok: entity_not_found, yeriyle", src(1, 2, GHOST)),
        case("Yalnız kaynağın kendisi: no_entities", src(1, 1)),
        case("Hepsi kilitli: layer_locked", src(1, 13)),
        case("Beklenen sürüm sayı değil: invalid_revision", src(1, 2, expectedRevision="on iki")),
        case("Retlerin sırası: kaynağın kimliği, hedefler, özellikler, sürüm",
             {"source": "x", "uids": [], "properties": [], "expectedRevision": "y"}),
    ]
    out.append({"name": "Retlerin sırası: hedefin kimliği özelliklerden, özellikler sürümden önce", "steps": [
        step({"source": U(1), "uids": ["x"], "properties": [], "expectedRevision": "y"}),
        step({"source": U(1), "uids": [U(2)], "properties": [], "expectedRevision": "y"}),
        step({"source": GHOST, "uids": [U(2)], "expectedRevision": "y"}),
    ]})
    out.append({"name": "Plan değişecek nesneleri gösterir; doğrulama yazmanın cevabını verir; hiçbir şey yazılmaz", "steps": [
        step(src(1, 3, 4), op="plan"),
        step(src(14, 2, props=["layer"]), op="validate"),
        step(src(15, 3, props=["layer"]), op="validate"),
    ]})
    out.append({"name": "Beklenen sürüm geçmişte kaldıysa conflict, hiçbir şey yazılmaz", "steps": [
        {"op": "captureRevision", "as": "old"},
        {"op": "execute", "command": "cad.entities.transform", "input": {"uids": [U(3)], "transform": {"kind": "move", "dx": 1, "dy": 0}},
         "result": "$any"},
        {"op": "execute", "input": src(1, 2, expectedRevision="$old"),
         "result": {"status": "conflict", "error": {
             "code": "revision_conflict", "path": "expectedRevision", "revision": "$current",
             "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."}},
         "expect": {"revision": "same"}}]})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.match", "commandVersion": 1,
            "title": "Özellik kopyala: nesnenin ve türün özellikleri, seçilen özellikler, kilitli ve gizli katman, retler",
            "note": "ADR 0304. Bağımsız başvuru: scripts/fixtures/match_command_cases.py.",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
