"""Writes fixtures/commands/v1/cad.layers.service.json: the shared cases of `cad.layers.service` v1 (docs/adr/0208 §15),
their expectations built here from the contract's rules (crates/shared/contracts/src/cad_layers.rs: the checks and
their order, the codes, the paths and the words; where a node goes; the step's name), without KentOS code. The web
(apps/web/src/product/fixtures.test.ts) and the desktop (crates/native/application/tests/all/fixtures.rs) run them.

    python3 scripts/fixtures/layer_service_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_service_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.service.json"

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
# A new layer's look: the layer tree's defaults (web `LayerStore.make`, desktop `default_style`).
NEW_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}

OSM = {"kind": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png", "maxZoom": 19, "attribution": "© OpenStreetMap katkıcıları", "preset": "osm-standard"}
WMS = {"kind": "wms", "url": "https://cbs.example.gov.tr/geoserver/wms", "layers": ["imar:plan"], "srid": 5256, "transparent": True, "connection": "kurum"}
WFS = {"kind": "wfs", "url": "https://cbs.example.gov.tr/geoserver/wfs", "name": "tkgm:parsel", "srid": 5256, "connection": "kurum"}
KURUM = {"id": "kurum", "name": "Kurumun CBS sunucusu", "origin": "https://cbs.example.gov.tr", "auth": "basic"}
HGM = {"id": "hgm", "name": "HGM ATLAS", "origin": "https://atlas.harita.gov.tr", "auth": "query", "names": ["apikey"]}


def node(id_, name, type_="layer", children=None, style=STYLE, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True, "style": dict(style), "children": children or [], **extra}


def new_layer(name, **extra):
    """The node `add` and `addFeed` make: the tree's defaults, its id the one the command gives (`$layer`)."""
    return node("$layer", name, style=NEW_STYLE, **extra)


SETUP_LAYERS = [
    node("parsel", "Parsel (WFS)", feed=dict(WFS)),
    node("altlik", "Altlıklar", "group", [node("osm", "OpenStreetMap", service=dict(OSM)), node("imar", "İmar planı", service=dict(WMS))]),
    node("0", "0"),
]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Servis katmanları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "connections": [dict(KURUM)]},
    "origin": {"x": 500000, "y": 4400000},
    "layers": SETUP_LAYERS,
    "activeLayer": "0",
    "entities": [
        {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"parselNo": "104/7"}, "pts": [{"x": 500000, "y": 4400000}, {"x": 500020, "y": 4400000}, {"x": 500020, "y": 4400020}, {"x": 500000, "y": 4400020}]},
        {"kind": "point", "id": 2, "layerId": "0", "attrs": {}, "p": {"x": 500010, "y": 4400030}},
    ],
    "styles": {"items": [], "categories": []},
}

LABEL = {"add": "Harita servisi ekle", "addFeed": "Veri katmanı ekle", "update": "Servis katmanını değiştir", "remove": "Servis katmanını sil"}


def tree():
    return copy.deepcopy(SETUP_LAYERS)


def find(nodes, id_):
    for i, n in enumerate(nodes):
        if n["id"] == id_:
            return nodes, i
        hit = find(n["children"], id_)
        if hit:
            return hit
    return None


def parent_of(nodes, id_, parent=None):
    for n in nodes:
        if n["id"] == id_:
            return parent
        found = parent_of(n["children"], id_, n)
        if found is not False:
            return found
    return False


def inserted(layers, new, parent=None, index=None):
    """Where a new node goes: into `parent` when it is a group, into the group of `parent` when it is a layer, else the
    top; at `index` among that group's nodes (first on top), last when none or past the end."""
    out = copy.deepcopy(layers)
    home = out
    if parent is not None and find(out, parent):
        siblings, i = find(out, parent)
        target = siblings[i]
        if target["type"] == "group":
            home = target["children"]
            target["expanded"] = True
        else:
            group = parent_of(out, parent)
            home = group["children"] if group else out
    at = len(home) if index is None else min(index, len(home))
    home.insert(at, new)
    return out


def replaced(layers, id_, **fields):
    out = copy.deepcopy(layers)
    siblings, i = find(out, id_)
    siblings[i] = {**siblings[i], **fields}
    return out


def removed(layers, id_):
    out = copy.deepcopy(layers)
    siblings, i = find(out, id_)
    del siblings[i]
    return out


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_=None, result=None, expect=None, **extra):
    s = {"op": op}
    if input_ is not None:
        s["input"] = input_
    if result is not None:
        s["result"] = result
    s.update(extra)
    if expect is not None:
        s["expect"] = expect
    return s


WRITTEN = completed({"layer": "$layer", "revision": "$current"})
UNCHANGED = {"ids": [1, 2], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree(), "connections": [KURUM]})]}


def cases():
    out = []
    osm_new = new_layer("OpenStreetMap", service=dict(OSM))
    with_osm = inserted(tree(), osm_new)
    out.append({
        "name": "Ekle: OSM altlığı ağacın sonuna, her şeyin altına; adımı “Harita servisi ekle”, geri alınır ve yinelenir",
        "steps": [
            step("execute", {"operation": "add", "name": "OpenStreetMap", "service": OSM}, WRITTEN, {"ids": [1, 2], "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": with_osm}),
            step("undo", returns=LABEL["add"], expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=LABEL["add"], expect={"layers": with_osm, "canUndo": True}),
        ],
    })
    out.append({
        "name": "Ekle: grubun başına (parent bir grup, index 0)",
        "steps": [step("execute", {"operation": "add", "name": "Topo", "parent": "altlik", "index": 0, "service": {"kind": "xyz", "url": "https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png", "subdomains": ["a", "b", "c"], "maxZoom": 17, "preset": "osm-topo"}}, WRITTEN,
                       {"layers": inserted(tree(), new_layer("Topo", service={"kind": "xyz", "url": "https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png", "subdomains": ["a", "b", "c"], "maxZoom": 17, "preset": "osm-topo"}), "altlik", 0)})],
    })
    out.append({
        "name": "Ekle: bir katmanın yanına (parent bir katman) onun grubunun sonuna",
        "steps": [step("execute", {"operation": "add", "name": "Uydu", "parent": "osm", "service": {"kind": "xyz", "url": "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}", "maxZoom": 19}}, WRITTEN,
                       {"layers": inserted(tree(), new_layer("Uydu", service={"kind": "xyz", "url": "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}", "maxZoom": 19}), "osm")})],
    })
    out.append({
        "name": "Ekle: index sondan büyükse sona; bilinmeyen parent ağacın en üstü",
        "steps": [step("execute", {"operation": "add", "name": "OpenStreetMap", "parent": "yok", "index": 99, "service": OSM}, WRITTEN, {"layers": inserted(tree(), copy.deepcopy(osm_new))})],
    })
    hgm_service = {"kind": "wmts", "url": "https://atlas.harita.gov.tr/wmts", "layers": ["harita"], "grid": {"srid": 3857, "matrices": [{"id": "0", "resolution": 156543.03392804097, "x0": -20037508.342789244, "y0": 20037508.342789244, "tileWidth": 256, "tileHeight": 256, "matrixWidth": 1, "matrixHeight": 1}]}, "matrixSet": "WebMercatorQuad", "connection": "hgm"}
    out.append({
        "name": "Ekle: bağlantısıyla; bağlantı aynı adımda projeye eklenir, geri almada kalır (proje ayarı)",
        "steps": [
            step("execute", {"operation": "add", "name": "HGM Harita", "service": hgm_service, "connections": [HGM]}, WRITTEN,
                 {"layers": inserted(tree(), new_layer("HGM Harita", service=hgm_service)), "connections": [KURUM, HGM], "dirty": True}),
            step("undo", returns=LABEL["add"], expect={"layers": tree(), "connections": [KURUM, HGM]}),
        ],
    })
    kurum2 = {"id": "kurum", "name": "Kurum (belirteçle)", "origin": "https://cbs.example.gov.tr", "auth": "bearer"}
    wms2 = {**WMS, "layers": ["imar:yapi"]}
    out.append({
        "name": "Ekle: aynı kimlikli bağlantı yerindekinin yerini alır",
        "steps": [step("execute", {"operation": "add", "name": "Yapılar", "service": wms2, "connections": [kurum2]}, WRITTEN,
                       {"layers": inserted(tree(), new_layer("Yapılar", service=wms2)), "connections": [kurum2]})],
    })
    feed = {"kind": "geojson", "url": "https://data.example.com/duraklar.geojson", "srid": 4326}
    fields = [{"name": "ad", "kind": "text"}, {"name": "hat", "kind": "integer"}]
    out.append({
        "name": "Veri katmanı ekle: kaynağı ve alanlarıyla; adımı “Veri katmanı ekle”",
        "steps": [
            step("execute", {"operation": "addFeed", "name": "Duraklar", "parent": "0", "index": 0, "feed": feed, "fields": fields}, WRITTEN,
                 {"layers": inserted(tree(), new_layer("Duraklar", feed=feed, fields=fields), "0", 0)}),
            step("undo", returns=LABEL["addFeed"], expect={"layers": tree()}),
        ],
    })
    osm2 = {**OSM, "opacity": 0.6}
    out.append({
        "name": "Değiştir: adı ve donukluğu tek adımda; adımı “Servis katmanını değiştir”, geri alınır",
        "steps": [
            step("execute", {"operation": "update", "layer": "osm", "name": "  OSM  ", "service": osm2}, completed({"layer": "osm", "revision": "$current"}),
                 {"layers": replaced(tree(), "osm", name="OSM", service=osm2), "canUndo": True, "revision": "changed"}),
            step("undo", returns=LABEL["update"], expect={"layers": tree()}),
            step("redo", returns=LABEL["update"], expect={"layers": replaced(tree(), "osm", name="OSM", service=osm2)}),
        ],
    })
    wfs2 = {**WFS, "limit": 1000, "fetched": "2026-10-08T09:30:00Z"}
    out.append({
        "name": "Değiştir: veri katmanının kaynağı; nesneleri kalır",
        "steps": [step("execute", {"operation": "update", "layer": "parsel", "feed": wfs2}, completed({"layer": "parsel", "revision": "$current"}),
                       {"ids": [1, 2], "layers": replaced(tree(), "parsel", feed=wfs2)})],
    })
    out.append({
        "name": "Değiştir: yalnız ad",
        "steps": [step("execute", {"operation": "update", "layer": "imar", "name": "İmar (WMS)"}, completed({"layer": "imar", "revision": "$current"}),
                       {"layers": replaced(tree(), "imar", name="İmar (WMS)"), "canUndo": True})],
    })
    out.append({
        "name": "Değiştir: aynı değerler bir şey yazmaz",
        "steps": [step("execute", {"operation": "update", "layer": "osm", "name": "OpenStreetMap", "service": OSM}, completed({"layer": "osm", "revision": "$current"}), UNCHANGED)],
    })
    out.append({
        "name": "Sil: servis katmanı; adımı “Servis katmanını sil”, geri alınır, yerine döner",
        "steps": [
            step("execute", {"operation": "remove", "layer": "osm"}, completed({"layer": "osm", "revision": "$current"}), {"layers": removed(tree(), "osm"), "canUndo": True, "revision": "changed"}),
            step("undo", returns=LABEL["remove"], expect={"layers": tree()}),
        ],
    })
    active = copy.deepcopy(SETUP)
    active["activeLayer"] = "osm"
    out.append({
        "name": "Sil: etkin katman silinmez; belgenin sözüyle reddedilir",
        "setup": active,
        "steps": [step("execute", {"operation": "remove", "layer": "osm"}, failed("layer_refused", "“OpenStreetMap” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin.", "layer"), {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"operation": "add", "name": "OpenStreetMap", "service": OSM}, {"status": "completed", "output": None, "warnings": []}, UNCHANGED)],
    })
    out.append({
        "name": "Plan: eklenecek katman ve bağlantılar; yazmaz",
        "steps": [step("plan", {"operation": "add", "name": "HGM Harita", "service": hgm_service, "connections": [HGM]},
                       completed({"node": new_layer("HGM Harita", service=hgm_service), "connections": [KURUM, HGM], "revision": "$current"}), {**UNCHANGED, "layers": tree(), "connections": [KURUM]})],
    })
    planned = replaced(tree(), "osm", name="OSM", service=osm2)
    out.append({
        "name": "Plan: değişecek katman; yazmaz",
        "steps": [step("plan", {"operation": "update", "layer": "osm", "name": "OSM", "service": osm2},
                       completed({"node": find(planned, "osm")[0][find(planned, "osm")[1]], "connections": [KURUM], "revision": "$current"}), UNCHANGED)],
    })
    out.append({
        "name": "Plan: silmede katman yok, bağlantılar olduğu gibi",
        "steps": [step("plan", {"operation": "remove", "layer": "imar"}, completed({"connections": [KURUM], "revision": "$current"}), UNCHANGED)],
    })
    # The refusals, in the contract's order.
    out.append(refused("katmanı olmayan değiştirme", {"operation": "update", "name": "OSM"}, "no_layer", "Değiştirilecek ya da silinecek katmanın kimliğini (layer) verin.", "layer"))
    out.append(refused("boş ad (yalnız boşluk)", {"operation": "add", "name": " \t", "service": OSM}, "empty_name", "Katmanın adı boş olamaz; bir ad verin.", "name"))
    out.append(refused("değiştirmede boş ad", {"operation": "update", "layer": "osm", "name": ""}, "empty_name", "Katmanın adı boş olamaz; bir ad verin.", "name"))
    out.append(refused("servissiz ekleme", {"operation": "add", "name": "Boş"}, "no_service", "Eklenecek servis (service) verilmedi; servisin türünü ve adresini verin.", "service"))
    out.append(refused("kaynaksız veri katmanı", {"operation": "addFeed", "name": "Boş"}, "no_feed", "Veri katmanının kaynağı (feed) verilmedi; servisin türünü, adresini ve tür adını verin.", "feed"))
    out.append(refused("servis ve kaynak birlikte", {"operation": "add", "name": "İkisi", "service": OSM, "feed": WFS}, "service_and_feed", "Bir katman ya servisten çizilir ya nesnelerini bir kaynaktan alır; service ile feed birlikte verilmez.", "feed"))
    out.append(refused("kuralına uymayan servis (XYZ şablonunda satır yok)", {"operation": "add", "name": "Bozuk", "service": {"kind": "xyz", "url": "https://tile.example.com/{z}/{x}.png"}}, "invalid_service", "XYZ şablonunda {z}, {x} ve {y} (ya da {-y}) ya da {quadkey} olmalı.", "service"))
    out.append(refused("kuralına uymayan kaynak (WFS'in tür adı yok)", {"operation": "addFeed", "name": "Bozuk", "feed": {"kind": "wfs", "url": "https://cbs.example.gov.tr/wfs"}}, "invalid_feed", "WFS kaynağının tür, koleksiyon ya da katman adı (name) olmalı.", "feed"))
    out.append(refused("aynı adlı iki alan", {"operation": "addFeed", "name": "Duraklar", "feed": feed, "fields": [{"name": "ad", "kind": "text"}, {"name": "AD", "kind": "text"}]}, "invalid_fields", "“AD” adlı iki alan var; alan adları bir kez kullanılır.", "fields"))
    out.append(refused("kuralına uymayan bağlantı (kökeninde yol)", {"operation": "add", "name": "HGM", "service": OSM, "connections": [{**HGM, "origin": "https://atlas.harita.gov.tr/wmts"}]}, "invalid_connection",
                       "“HGM ATLAS” bağlantısı: Bağlantının kökeni küçük harfle şema ve makine (https://ornek.gov.tr, isteğe bağlı :kapı) olmalı, yolu olmamalı; “https://atlas.harita.gov.tr/wmts” değil.", "connections"))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "add", "name": "OSM", "service": OSM, "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"operation": "update", "layer": "imar", "name": "İmar (WMS)"}, completed({"layer": "imar", "revision": "$current"}), {"revision": "changed"}),
            step("execute", {"operation": "remove", "layer": "osm", "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "layers": replaced(tree(), "imar", name="İmar (WMS)")}),
        ],
    })
    out.append(refused("çizimde olmayan katman", {"operation": "remove", "layer": "yok"}, "layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"operation": "update", "layer": "altlik", "name": "Altlık"}, "not_a_layer", "“Altlıklar” bir katman grubu; servis ve veri kaynağı yalnız katmanın olur.", "layer"))
    out.append(refused("servis katmanı olmayanı silme", {"operation": "remove", "layer": "parsel"}, "not_a_service_layer",
                       "“Parsel (WFS)” bir servis katmanı değil; bu komut yalnız servis katmanını siler. Katmanı Katmanlar panelinden silin.", "layer"))
    out.append(refused("servis katmanı olmayana servis", {"operation": "update", "layer": "0", "service": OSM}, "not_a_service_layer", "“0” bir servis katmanı değil; servisi yalnız servis katmanının değişir.", "layer"))
    out.append(refused("kaynağı olmayana kaynak", {"operation": "update", "layer": "osm", "feed": WFS}, "not_a_service_layer", "“OpenStreetMap” katmanının veri kaynağı yok; kaynak yalnız veri katmanının değişir.", "layer"))
    out.append(refused("projede olmayan bağlantı", {"operation": "add", "name": "HGM", "service": {**OSM, "connection": "hgm"}}, "unknown_connection",
                       "“hgm” bağlantısı projede yok; bağlantıyı connections ile birlikte verin ya da Bağlantılar penceresinden ekleyin.", "service.connection"))
    out.append(refused("kaynağın projede olmayan bağlantısı", {"operation": "update", "layer": "parsel", "feed": {**WFS, "connection": "eski"}}, "unknown_connection",
                       "“eski” bağlantısı projede yok; bağlantıyı connections ile birlikte verin ya da Bağlantılar penceresinden ekleyin.", "feed.connection"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.service",
        "commandVersion": 1,
        "title": "Servis katmanı: harita servisi ve veri katmanı ekle, değiştir, sil (ADR 0208 §15)",
        "note": "scripts/fixtures/layer_service_command_cases.py yazar; beklentiler sözleşmenin kurallarından (crates/shared/contracts/src/cad_layers.rs) KentOS kodu olmadan kurulur. `$layer` komutun eklediği (plan: ekleyeceği) katmanın kimliğidir: ağaçta olmayan bir `layer-N`; `expect.layers` katman ağacının tamamı, `expect.connections` projenin bağlantılarıdır.",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"cad.layers.service durumları güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
