"""The desktop's own cases of the product commands on point clouds (docs/adr/0207 §10).

    python3 scripts/fixtures/pointcloud_command_cases.py           # writes the files
    python3 scripts/fixtures/pointcloud_command_cases.py --check   # writes nothing; compares

Point clouds are the desktop's (the owner's decision, 8 October 2026: the web
adds, draws and processes none yet), so their cases are in
fixtures/commands/v1/desktop/, which only the desktop's runner reads
(crates/native/application/tests/all/fixtures.rs); the web's runner reads the
folder above. When the web takes clouds, these move up beside the others.

Writes cad.entities.create.json (Nokta bulutu ekle: linked, addressed and
embedded files, a virtual cloud, the look's defaults no fields; the cloud's
rules in their order, `invalid_pointcloud`, `not_finite`, `unknown_asset`, the
layer), cad.entities.edit.json (Nokta bulutu stili: the look and the opacity
written in place; the rules; a locked layer), cad.entities.transform.json and
cad.entities.array.json (`pointcloud_fixed`: a cloud's place is its files') and
cad.blocks.define.json (`pointcloud_in_block`). The checks, their order,
codes, paths and messages are written here by hand from the ADR and the
contract's rules (crates/shared/contracts/src/pointcloud.rs), not from an
implementation's output.

`--check` rebuilds the files in memory and compares them with the ones on disk.
"""
import json
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "commands" / "v1" / "desktop"

style = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def layer(i, name, visible=True, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": locked, "expanded": True, "style": style, "children": []}


def P(x, y):
    return {"x": x, "y": y}


# ── The drawing ─────────────────────────────────────────────────────────

KOY_BOUNDS = [487600, 4420300, 838.991, 487719.838, 4420389.999, 862.618]
ADA_BOUNDS = [487700, 4420300, 840, 487800, 4420400, 870.5]
EMBEDDED_BOUNDS = [487650, 4420320, 844.12, 487651, 4420320.5, 844.25]
CLOUD_ASSET = "pointcloud-0011223344556677"
RASTER_ASSET = "raster-0011223344556677"
MISSING_ASSET = "pointcloud-ffffffffffffffff"


def cloud(sources, style_, count=None, bounds=None, opacity=None, srid=5256):
    """A cloud's geometry: its files, their points and bounds together (unless given), its look."""
    g = {"kind": "pointcloud", "sources": sources}
    if bounds is None:
        b = [s["bounds"] for s in sources]
        bounds = [min(x[0] for x in b), min(x[1] for x in b), min(x[2] for x in b),
                  max(x[3] for x in b), max(x[4] for x in b), max(x[5] for x in b)] if b else KOY_BOUNDS
    g["bounds"] = bounds
    g["count"] = sum(s["count"] for s in sources) if count is None else count
    g["srid"] = srid
    g["style"] = style_
    if opacity is not None:
        g["opacity"] = opacity
    return g


def linked(file="bulut/koy.laz", fmt="laz", count=71212, bounds=KOY_BOUNDS):
    return {"file": file, "format": fmt, "count": count, "bounds": bounds}


RGB = {"render": "rgb", "size": 2}
KOY = cloud([linked()], RGB)
ON_LOCKED = cloud([linked("bulut/eski.laz")], {"render": "classification", "size": 2})

ENTITIES = [
    {"kind": "line", "id": 1, "layerId": "yapi", "attrs": {}, "a": P(487600, 4420290), "b": P(487620, 4420290)},
    {"id": 2, "layerId": "bulut", "attrs": {"Kaynak": "LiDAR"}, **KOY},
    {"id": 3, "layerId": "kilitli", "attrs": {}, **ON_LOCKED},
    {"kind": "circle", "id": 4, "layerId": "yapi", "attrs": {}, "c": P(487640, 4420290), "r": 5},
]
# The objects' order: a document stores the kind first.
ENTITIES = [{"kind": e["kind"], **{k: v for k, v in e.items() if k != "kind"}} for e in ENTITIES]
IDS = [e["id"] for e in ENTITIES]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Nokta bulutu komutları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 487600, "y": 4420300},
    "layers": [
        layer("yapi", "Yapı"),
        layer("bulut", "Bulut"),
        layer("kilitli", "Kilitli katman", locked=True),
        layer("gizli", "Gizli katman", visible=False),
    ],
    "activeLayer": "bulut",
    "entities": ENTITIES,
    "styles": {"items": [
        {"kind": "asset", "id": CLOUD_ASSET, "name": "ornek", "path": ["Nokta bulutları"], "format": "xyz", "data": "data:application/octet-stream;base64,NDg3NjUwIDQ0MjAzMjAgODQ0LjEyCg=="},
        {"kind": "asset", "id": RASTER_ASSET, "name": "dem", "path": ["Rasterler"], "format": "tiff", "data": "data:image/tiff;base64,SUkqAA==", "width": 50, "height": 40},
    ], "categories": []},
}

# ── What the commands answer (the contract) ─────────────────────────────

NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def locked(name):
    return failed("layer_locked", f"“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId")


def hidden(name):
    return {"code": "layer_hidden", "message": f"“{name}” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}


def not_finite(n, list_="objects", what="nesnenin"):
    return failed("not_finite", f"{n}. {what} geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin.", f"{list_}[{n - 1}].geometry")


def number(v):
    """A number as Rust's `Display` writes an f64: no fraction when whole."""
    return str(int(v)) if float(v).is_integer() else repr(float(v))


# The contract's rules (PointCloudFields::problem, PointCloudStyle::problem), in their order.
NO_FILES = "Nokta bulutunun en az bir dosyası olmalı."


def one_source(i):
    return f"{i}. dosyanın kaynağı ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri."


def not_http(u):
    return f"“{u}” bir HTTP ya da HTTPS adresi değil (http:// ya da https:// ile başlamalı)."


def file_bounds(i):
    return f"{i}. dosyanın kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı."


CLOUD_BOUNDS = "Nokta bulutunun kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı."
NOT_SUM = "Nokta bulutunun nokta sayısı dosyalarınkinin toplamı olmalı."
RAMPS = "Gri, Arazi, Spektral, Viridis, Mavi-kırmızı, Sıcaklık"


def no_ramp(r):
    return f"“{r}” diye bir renk rampası yok; {RAMPS} rampalarından biri seçilmeli."


RANGE = "Görünüşün aralığı iki sonlu sayı olmalı, en küçük en büyükten küçük."
RAMP_RANGE = "Rampalı görünüşün aralığı (en küçük ve en büyük) verilmeli."
SIZE = f"Noktanın boyu {number(0.5)} ile {number(32)} arasında olmalı."
HIDDEN = "Gizlenen sınıflar küçükten büyüğe ve birer kez yazılmalı."


def opacity_message(o):
    return f"Nokta bulutunun donukluğu {number(0.1)} ile {number(1)} arasında olmalı; {number(o)} verildi."


def unknown_asset(asset):
    return (f"“{asset}” kimlikli nokta bulutu projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir. "
            "Projenin kitaplığındaki bir LAS, LAZ, COPC ya da XYZ bulutunun kimliğini verin.")


FIXED = "Nokta bulutu taşınmaz, döndürülmez, ölçeklenmez, aynalanmaz ve kopyalanmaz: konumu dosyasındadır. Bulutu seçimden çıkarın."
IN_BLOCK = "Seçilenlerde nokta bulutu var; nokta bulutu bloğa konamaz. Bulutu seçimden çıkarın."


def stored(geometry):
    """A cloud as the document stores it: the look's defaults are no fields
    (not inverted, 16-bit colours, nothing hidden, pixels, round points)."""
    out = json.loads(json.dumps(geometry))
    st = out["style"]
    for k, default in (("invert", False), ("rgb8", False), ("hidden", []), ("sizeUnit", "px"), ("shape", "round")):
        if st.get(k) == default:
            st.pop(k)
    return out


def made(geometry, slot, layer_id, attrs=None):
    out = stored(geometry)
    out["id"] = slot
    out["layerId"] = layer_id
    out["attrs"] = attrs or {}
    return out


def uid(i):
    return f"$uidOf:{i}"


def refused(result, input_, expect=None, **extra):
    """A refused step: nothing written, the history as it was (`expect`, after an undo)."""
    return {"op": "execute", "input": input_, "result": result, "expect": expect or NOTHING, **extra}


# After an undo: nothing written, the redo kept.
UNTOUCHED = {"ids": IDS, "revision": "same"}


def file(command, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1,
            "title": title, "note": note, "setup": SETUP, "cases": cases}


DESKTOP = "Yalnız masaüstünün durumları (ADR 0207 §10): nokta bulutunu şimdilik masaüstü ekler, çizer ve işler; web'in koşucusu bu klasörü okumaz."

# ── cad.entities.create ─────────────────────────────────────────────────

ADDRESSED = cloud([{"url": "https://ornek.org/kentos/ada-1.copc.laz", "format": "copc", "count": 1250000, "bounds": ADA_BOUNDS}],
                  {"render": "classification", "hidden": [7, 18], "size": 1.5, "sizeUnit": "m", "shape": "square"}, opacity=0.8)
VIRTUAL = cloud([linked("bulut/ada-2.laz", count=980000, bounds=[487800, 4420300, 841.25, 487900, 4420400, 868]),
                 {"asset": CLOUD_ASSET, "format": "xyz", "count": 3, "bounds": EMBEDDED_BOUNDS}],
                {"render": "elevation", "ramp": "Spektral", "invert": True, "min": 838, "max": 871, "rgb8": True, "size": 3,
                 "hidden": [], "sizeUnit": "px", "shape": "round"}, srid=0)


def create(objects, layer_id="bulut", **more):
    return {"layerId": layer_id, "operation": "pointCloud", "objects": [{"geometry": g} for g in objects], **more}


def with_style(**changes):
    return {**KOY, "style": {**KOY["style"], **changes}}


def with_source(**changes):
    s = {**KOY["sources"][0], **changes}
    return {**KOY, "sources": [{k: v for k, v in s.items() if v is not None}]}


CREATE_CASES = [
    {
        "name": "Nokta bulutu ekle: bağlı, adresten okunan ve gömülü dosyalı sanal bulut tek adımda yazılır, adı “Nokta bulutu ekle”; görünüşün varsayılanları alan değildir (ADR 0207 §3, §10)",
        "steps": [
            {"op": "execute", "input": create([KOY, ADDRESSED, VIRTUAL]),
             "result": {"status": "completed", "output": {"created": [uid(5), uid(6), uid(7)], "ids": [5, 6, 7], "revision": "$current"}, "warnings": []},
             "expect": {"ids": IDS + [5, 6, 7], "entities": {"5": made(KOY, 5, "bulut"), "6": made(ADDRESSED, 6, "bulut"), "7": made(VIRTUAL, 7, "bulut")},
                        "uids": {"5": "new", "6": "new", "7": "new"}, "canUndo": True, "dirty": True, "revision": "changed"}},
            {"op": "undo", "returns": "Nokta bulutu ekle", "expect": {"ids": IDS, "canUndo": False, "canRedo": True}},
            {"op": "redo", "returns": "Nokta bulutu ekle", "expect": {"ids": IDS + [5, 6, 7], "entities": {"6": made(ADDRESSED, 6, "bulut")}}},
        ],
    },
    {
        "name": "gizli katmana eklenen bulut yazılır, layer_hidden uyarısıyla; kilitli katmana eklenmez (ADR 0207 §10)",
        "steps": [
            {"op": "execute", "input": create([KOY], "gizli"),
             "result": {"status": "completed", "output": {"created": [uid(5)], "ids": [5], "revision": "$current"}, "warnings": [hidden("Gizli katman")]},
             "expect": {"ids": IDS + [5], "entities": {"5": made(KOY, 5, "gizli")}, "revision": "changed"}},
            {"op": "undo", "returns": "Nokta bulutu ekle", "expect": {"ids": IDS}},
            refused(locked("Kilitli katman"), create([KOY], "kilitli"), UNTOUCHED),
        ],
    },
    {
        "name": "nokta bulutunun dosyaları sırayla: invalid_pointcloud, yolu nesnenin geometrisi; hiçbir şey yazılmaz (ADR 0207 §3, §10)",
        "steps": [
            refused(failed("invalid_pointcloud", NO_FILES, "objects[0].geometry"), create([cloud([], RGB, count=0, bounds=KOY_BOUNDS)])),
            refused(failed("invalid_pointcloud", one_source(1), "objects[1].geometry"),
                    create([KOY, with_source(url="https://ornek.org/kentos/koy.laz")])),
            refused(failed("invalid_pointcloud", one_source(1), "objects[0].geometry"), create([with_source(file=None)])),
            refused(failed("invalid_pointcloud", one_source(1), "objects[0].geometry"), create([with_source(file="   ")])),
            refused(failed("invalid_pointcloud", not_http("ftp://ornek.org/koy.laz"), "objects[0].geometry"),
                    create([with_source(file=None, url="ftp://ornek.org/koy.laz")])),
            refused(failed("invalid_pointcloud", file_bounds(1), "objects[0].geometry"),
                    create([with_source(bounds=[487720, 4420300, 838.991, 487600, 4420390, 862.618])])),
            refused(failed("invalid_pointcloud", CLOUD_BOUNDS, "objects[0].geometry"),
                    create([{**KOY, "bounds": [487600, 4420300, 870, 487720, 4420390, 838]}])),
            refused(failed("invalid_pointcloud", NOT_SUM, "objects[0].geometry"), create([{**KOY, "count": 71213}])),
        ],
    },
    {
        "name": "nokta bulutunun görünüşü ve donukluğu sırayla: rampa, aralık, rampalı görünüşün aralığı, boy, gizlenen sınıflar, donukluk; invalid_pointcloud (ADR 0207 §5, §10)",
        "steps": [
            refused(failed("invalid_pointcloud", no_ramp("Gökkuşağı"), "objects[0].geometry"), create([with_style(ramp="Gökkuşağı")])),
            refused(failed("invalid_pointcloud", RANGE, "objects[0].geometry"), create([with_style(min=10)])),
            refused(failed("invalid_pointcloud", RANGE, "objects[0].geometry"), create([with_style(min=10, max=10)])),
            refused(failed("invalid_pointcloud", RAMP_RANGE, "objects[0].geometry"), create([with_style(render="intensity")])),
            refused(failed("invalid_pointcloud", SIZE, "objects[0].geometry"), create([with_style(size=0.25)])),
            refused(failed("invalid_pointcloud", SIZE, "objects[0].geometry"), create([with_style(size=33)])),
            refused(failed("invalid_pointcloud", HIDDEN, "objects[0].geometry"), create([with_style(hidden=[7, 2])])),
            refused(failed("invalid_pointcloud", HIDDEN, "objects[0].geometry"), create([with_style(hidden=[7, 7])])),
            refused(failed("invalid_pointcloud", opacity_message(0.05), "objects[0].geometry"), create([{**KOY, "opacity": 0.05}])),
            refused(failed("invalid_pointcloud", opacity_message(1.5), "objects[0].geometry"), create([{**KOY, "opacity": 1.5}])),
        ],
    },
    {
        "name": "sonlu olmayan sayı önce not_finite; kitaplıkta olmayan ya da bulut olmayan varlık unknown_asset, yolu dosyanın asset'i; katmandan sonra (ADR 0207 §10)",
        "steps": [
            refused(not_finite(1), create([KOY]), nonFinite={"objects[0].geometry.bounds[2]": "NaN"}),
            refused(not_finite(2), create([KOY, KOY]), nonFinite={"objects[1].geometry.sources[0].bounds[5]": "Infinity"}),
            refused(not_finite(1), create([KOY]), nonFinite={"objects[0].geometry.style.size": "-Infinity"}),
            refused(not_finite(1), create([{**KOY, "opacity": 0.5}]), nonFinite={"objects[0].geometry.opacity": "NaN"}),
            refused(failed("unknown_asset", unknown_asset(MISSING_ASSET), "objects[1].geometry.sources[1].asset"),
                    create([KOY, {**VIRTUAL, "sources": [VIRTUAL["sources"][0], {**VIRTUAL["sources"][1], "asset": MISSING_ASSET}]}])),
            refused(failed("unknown_asset", unknown_asset(RASTER_ASSET), "objects[0].geometry.sources[0].asset"),
                    create([cloud([{"asset": RASTER_ASSET, "format": "laz", "count": 3, "bounds": EMBEDDED_BOUNDS}], RGB)])),
            refused(locked("Kilitli katman"), create([cloud([{"asset": MISSING_ASSET, "format": "laz", "count": 3, "bounds": EMBEDDED_BOUNDS}], RGB)], "kilitli")),
        ],
    },
]

# ── cad.entities.edit ───────────────────────────────────────────────────

RESTYLED = {**KOY, "style": {"render": "elevation", "ramp": "Viridis", "min": 839, "max": 863, "size": 0.5, "sizeUnit": "m", "shape": "square", "hidden": [7]}, "opacity": 0.6}
PLAIN = {**KOY, "style": {"render": "returns", "size": 4, "invert": False, "rgb8": False, "hidden": [], "sizeUnit": "px", "shape": "round"}}


def restyle(geometry, at=2, operation="pointCloudStyle"):
    return {"operation": operation, "changes": [{"kind": "update", "uid": uid(at), "geometry": geometry}]}


def changed(*slots):
    return {"status": "completed", "output": {"changed": [uid(s) for s in slots], "created": [], "removed": [], "revision": "$current"}, "warnings": []}


EDIT_LOCKED = "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."

EDIT_CASES = [
    {
        "name": "Nokta bulutu stili: görünüş ve donukluk yerinde yazılır, kimliği ve öznitelikleri kalır; görünüşün varsayılanları alan değildir; adı “Nokta bulutu stili” (ADR 0207 §5, §10)",
        "steps": [
            {"op": "captureUid", "id": 2, "as": "koy"},
            {"op": "execute", "input": restyle(RESTYLED), "result": changed(2),
             "expect": {"ids": IDS, "entities": {"2": made(RESTYLED, 2, "bulut", {"Kaynak": "LiDAR"})}, "uids": {"2": "koy"}, "canUndo": True, "revision": "changed"}},
            {"op": "execute", "input": restyle(PLAIN), "result": changed(2),
             "expect": {"entities": {"2": made(PLAIN, 2, "bulut", {"Kaynak": "LiDAR"})}, "revision": "changed"}},
            {"op": "undo", "returns": "Nokta bulutu stili", "expect": {"entities": {"2": made(RESTYLED, 2, "bulut", {"Kaynak": "LiDAR"})}}},
            {"op": "undo", "returns": "Nokta bulutu stili", "expect": {"entities": {"2": made(KOY, 2, "bulut", {"Kaynak": "LiDAR"})}, "uids": {"2": "koy"}, "canUndo": False}},
        ],
    },
    {
        "name": "Nokta bulutu stili, ret: görünüşün ve dosyaların kuralları, sonlu olmayan sayı, kitaplıkta olmayan varlık, kilitli katmandaki bulut; hiçbir şey yazılmaz (ADR 0207 §10)",
        "steps": [
            refused(failed("invalid_pointcloud", RAMP_RANGE, "changes[0].geometry"), restyle({**KOY, "style": {"render": "elevation", "size": 2}})),
            refused(failed("invalid_pointcloud", SIZE, "changes[0].geometry"), restyle(with_style(size=40))),
            refused(failed("invalid_pointcloud", opacity_message(0), "changes[0].geometry"), restyle({**KOY, "opacity": 0})),
            refused(failed("invalid_pointcloud", NOT_SUM, "changes[0].geometry"), restyle({**KOY, "count": 1})),
            refused(not_finite(1, "changes", "değişikliğin"), restyle({**KOY, "opacity": 0.5}), nonFinite={"changes[0].geometry.opacity": "Infinity"}),
            refused(failed("unknown_asset", unknown_asset(MISSING_ASSET), "changes[0].geometry.sources[0].asset"),
                    restyle(cloud([{"asset": MISSING_ASSET, "format": "xyz", "count": 71212, "bounds": KOY_BOUNDS}], RGB))),
            refused(failed("layer_locked", EDIT_LOCKED, "changes[0].uid"), restyle(ON_LOCKED, at=3)),
        ],
    },
]

# ── cad.entities.transform and cad.entities.array ───────────────────────


def transform(uids, t, copy=False):
    out = {"uids": [uid(i) for i in uids], "transform": t}
    if copy:
        out["copy"] = True
    return out


MOVE = {"kind": "move", "dx": 10, "dy": 5}

TRANSFORM_CASES = [
    {
        "name": "nokta bulutu taşınmaz, döndürülmez, ölçeklenmez, aynalanmaz ve kopyalanmaz: pointcloud_fixed, yolu bulutun yeri; hiçbir şey yazılmaz (ADR 0207 §3)",
        "steps": [
            refused(failed("pointcloud_fixed", FIXED, "uids[1]"), transform([1, 2], MOVE)),
            refused(failed("pointcloud_fixed", FIXED, "uids[0]"), transform([2], {"kind": "rotate", "center": P(487650, 4420340), "angle": 0.5})),
            refused(failed("pointcloud_fixed", FIXED, "uids[0]"), transform([2], {"kind": "scale", "center": P(487650, 4420340), "factor": 2})),
            refused(failed("pointcloud_fixed", FIXED, "uids[0]"), transform([2], {"kind": "mirror", "a": P(487650, 4420300), "b": P(487650, 4420400)})),
            refused(failed("pointcloud_fixed", FIXED, "uids[0]"), transform([2], MOVE, copy=True)),
        ],
    },
    {
        "name": "kilitli katmandaki bulut öbür kilitliler gibi atlanır; seçimin kalanı taşınır (ADR 0037, 0207 §3)",
        "steps": [
            {"op": "execute", "input": transform([3, 1], MOVE),
             "result": {"status": "completed", "output": {"changed": [uid(1)], "created": [], "locked": [uid(3)], "revision": "$current"},
                        "warnings": [{"code": "layer_locked", "message": "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}]},
             "expect": {"ids": IDS, "entities": {"1": {"kind": "line", "id": 1, "layerId": "yapi", "attrs": {}, "a": P(487610, 4420295), "b": P(487630, 4420295)},
                                                 "3": made(ON_LOCKED, 3, "kilitli")}, "revision": "changed"}},
            {"op": "undo", "returns": "Taşı", "expect": {"entities": {"1": ENTITIES[0]}}},
            refused(failed("layer_locked", "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.", "uids"), transform([3], MOVE), UNTOUCHED),
        ],
    },
]

ARRAY_CASES = [
    {
        "name": "dizi nokta bulutunu kopyalamaz: pointcloud_fixed, yolu bulutun yeri; kilitli katmandaki bulut atlanır (ADR 0207 §3)",
        "steps": [
            refused(failed("pointcloud_fixed", FIXED, "uids[1]"), {"uids": [uid(1), uid(2)], "layout": {"kind": "grid", "rows": 1, "cols": 3, "dx": 50, "dy": 0}}),
            refused(failed("pointcloud_fixed", FIXED, "uids[0]"), {"uids": [uid(2)], "layout": {"kind": "polar", "center": P(487650, 4420340), "count": 4, "fill": 360, "rotate": True}}),
            {"op": "execute", "input": {"uids": [uid(3), uid(1)], "layout": {"kind": "grid", "rows": 1, "cols": 2, "dx": 30, "dy": 0}},
             "result": {"status": "completed", "output": {"created": [uid(5)], "locked": [uid(3)], "revision": "$current"},
                        "warnings": [{"code": "layer_locked", "message": "1 nesne kilitli katmanda olduğu için atlandı. Kopyalamak için katmanın kilidini Katmanlar panelinden açın.", "path": "uids"}]},
             "expect": {"ids": IDS + [5], "entities": {"5": {"kind": "line", "id": 5, "layerId": "yapi", "attrs": {}, "a": P(487630, 4420290), "b": P(487650, 4420290)}}, "revision": "changed"}},
        ],
    },
]

# ── cad.blocks.define ───────────────────────────────────────────────────

BLOCK_CASES = [
    {
        "name": "nokta bulutu bloğa konamaz: pointcloud_in_block, ilk bulutun yeriyle (ADR 0207 §3)",
        "steps": [
            refused(failed("pointcloud_in_block", IN_BLOCK, "uids[1]"), {"name": "Bulutlu", "base": P(487600, 4420300), "uids": [uid(1), uid(2), uid(4)]}),
            refused(failed("pointcloud_in_block", IN_BLOCK, "uids[0]"), {"name": "Bulutlu", "base": P(487600, 4420300), "uids": [uid(2)]}),
        ],
    },
]

FILES = {
    "cad.entities.create.json": file("cad.entities.create", "Nokta bulutu ekle (masaüstü)", DESKTOP, CREATE_CASES),
    "cad.entities.edit.json": file("cad.entities.edit", "Nokta bulutu stili (masaüstü)", DESKTOP, EDIT_CASES),
    "cad.entities.transform.json": file("cad.entities.transform", "Nokta bulutunun yeri (masaüstü)", DESKTOP, TRANSFORM_CASES),
    "cad.entities.array.json": file("cad.entities.array", "Nokta bulutu dizide (masaüstü)", DESKTOP, ARRAY_CASES),
    "cad.blocks.define.json": file("cad.blocks.define", "Nokta bulutu blokta (masaüstü)", DESKTOP, BLOCK_CASES),
}


def text_of(d):
    return json.dumps(d, ensure_ascii=False, indent=1) + "\n"


def main():
    check = "--check" in sys.argv[1:]
    bad = []
    for name, d in FILES.items():
        path = OUT / name
        text = text_of(d)
        if check:
            if not path.exists() or path.read_text() != text:
                bad.append(name)
        else:
            OUT.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
    if check:
        if bad:
            print("farklı: " + ", ".join(bad) + "; yeniden yazın: python3 scripts/fixtures/pointcloud_command_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{len(FILES)} dosya güncel: {OUT.relative_to(OUT.parents[3])}")
    else:
        print(f"{len(FILES)} dosya yazıldı: {OUT.relative_to(OUT.parents[3])}")


if __name__ == "__main__":
    main()
