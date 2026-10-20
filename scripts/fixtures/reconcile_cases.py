#!/usr/bin/env python3
"""The shared cases of cad.sources.reconcile (docs/adr/0538, plan/gis.md CBS-37): a bound layer and its table, both
changed since they last matched, put together object by object.

    python3 scripts/fixtures/reconcile_cases.py           # writes fixtures/commands/v1/cad.sources.reconcile.json
    python3 scripts/fixtures/reconcile_cases.py --check   # compares
    python3 scripts/fixtures/reconcile_cases.py --files   # GDAL writes uzlas.gpkg and uzlas-qgis.gpkg
    python3 scripts/fixtures/reconcile_cases.py --written # OGR reads kentos-uzlasti.gpkg (KENTOS_WRITE_GPKG=1 cargo
                                                          # test -p kentos-native-application --test all reconcile)

No KentOS code. The two files are written here with GDAL's OGR: `uzlas.gpkg` holds ten parcels; `uzlas-qgis.gpkg` is the
same file edited as a QGIS user would (an attribute, a geometry moved, rows deleted, a row added). A case copies the
first over a temporary file, binds it, edits the layer through the commands, copies the second over the file (the edit
in QGIS) and reconciles. A table's rows are read with gpkg_import_cases.py's OGR reader, as the import reads them.

The rule (ADR 0538 §2) is written here from the ADR: against the objects as they were when the layer and the table
last matched (the first file, as the link read it), each side either did nothing, added, changed or removed an object.
One side only: that side wins. Both removed, or both changed to the same kind, geometry and attributes: agreed. Both
changed otherwise, or one removed what the other changed: a conflict, its differing attributes by name (each side's
value, none where a side has none: a removed object's are all the other side's) and whether the kind or geometry
differs; it goes the way `take` or `rest` says.
Conflicts are listed in the lasting ids' byte order. The counts into the table are those of the write that follows:
an object of the layer's side that the table lacks is added, one it holds otherwise changed, one the layer lacks
removed; an unresolved conflict is left as the table holds it.
"""
import copy
import hashlib
import json
import shutil
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import gpkg_import_cases as gi  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/cad.sources.reconcile.json"
A = "uzlas.gpkg"
B = "uzlas-qgis.gpkg"
WRITTEN = "kentos-uzlasti.gpkg"
LAYER = "import-parsel"
ANY = "$any"
MALIK = ["Ahmet", "Ayşe", "Can", "Deniz", "Ece", "Fatma", "Gül", "Hakan", "İpek", "Kemal"]


def square(x, y, side=20.0):
    return [(x, y), (x + side, y), (x + side, y + side), (x, y + side), (x, y)]


def write_files():
    """A: ten parcels in TUREF / TM36; B: A as a QGIS user edited it."""
    from osgeo import ogr, osr
    a = DIR / A
    if a.exists():
        a.unlink()
    drv = ogr.GetDriverByName("GPKG")
    ds = drv.CreateDataSource(str(a))
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    lyr = ds.CreateLayer("parsel", srs, ogr.wkbMultiPolygon, ["FID=fid", "GEOMETRY_NAME=geom"])
    f_ada = ogr.FieldDefn("ada", ogr.OFTString)
    f_ada.SetWidth(20)
    lyr.CreateField(f_ada)
    lyr.CreateField(ogr.FieldDefn("parsel_no", ogr.OFTInteger))
    f_malik = ogr.FieldDefn("malik", ogr.OFTString)
    f_malik.SetWidth(40)
    lyr.CreateField(f_malik)

    def poly(x, y):
        ring = ogr.Geometry(ogr.wkbLinearRing)
        for px, py in square(x, y):
            ring.AddPoint_2D(px, py)
        p = ogr.Geometry(ogr.wkbPolygon)
        p.AddGeometry(ring)
        m = ogr.Geometry(ogr.wkbMultiPolygon)
        m.AddGeometry(p)
        return m

    for i in range(1, 11):
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(i)
        f.SetField("ada", str(100 + i))
        f.SetField("parsel_no", i)
        f.SetField("malik", MALIK[i - 1])
        f.SetGeometry(poly(487000.0 + 30 * (i - 1), 4420000.0))
        lyr.CreateFeature(f)
    ds = None
    b = DIR / B
    shutil.copyfile(a, b)
    ds = ogr.Open(str(b), 1)
    lyr = ds.GetLayerByName("parsel")

    def edit(fid, **values):
        f = lyr.GetFeature(fid)
        for k, v in values.items():
            if k == "geom":
                f.SetGeometry(v)
            else:
                f.SetField(k, v)
        lyr.SetFeature(f)

    edit(1, ada="201")
    edit(2, geom=poly(487035.0, 4420000.0))
    lyr.DeleteFeature(3)
    edit(4, malik="Zehra")
    edit(5, ada="205")
    lyr.DeleteFeature(6)
    edit(7, ada="207")
    edit(10, malik="Ortak")
    f = ogr.Feature(lyr.GetLayerDefn())
    f.SetField("ada", "111")
    f.SetField("parsel_no", 11)
    f.SetField("malik", "Yeni")
    f.SetGeometry(poly(487300.0, 4420000.0))
    lyr.CreateFeature(f)
    ds = None


def rows(name):
    t = next(t for t in gi.read(name)["tables"] if t["name"] == "parsel")
    return {r["fid"]: r for r in t["rows"]}


def obj(row):
    """An object as the layer holds a row: kind, geometry, attributes (gpkg_import_cases.py's reading)."""
    return {"kind": row["kind"], "attrs": dict(row["attrs"]), **copy.deepcopy(row["geometry"])}


def essence(o):
    return {k: v for k, v in o.items() if k not in ("layerId",)}


# ── The steps ────────────────────────────────────────────────────────────────

def copy_step(name, tmp):
    return {"op": "copyFile", "from": f"$fixture:geopackage/v1/{name}", "to": f"$tmp:{tmp}"}


def link_step(tmp):
    return bs.other("cad.sources.edit", {"edit": {"kind": "link", "path": f"$tmp:{tmp}", "table": "parsel"}})


class Layer:
    """The bound layer as the steps leave it: objects by slot, each with its uid (None: a new one)."""

    def __init__(self):
        a = rows(A)
        self.slots = {}
        self.uid = {}
        for k, fid in enumerate(sorted(a)):
            slot = 2 + k
            self.slots[slot] = obj(a[fid])
            self.uid[slot] = a[fid]["uid"]
        self.next = 2 + len(a)

    def slot_of(self, fid):
        u = rows(A)[fid]["uid"]
        return next(s for s, x in self.uid.items() if x == u)

    def set(self, fid, attrs):
        s = self.slot_of(fid)
        for k, v in attrs.items():
            if v is None:
                self.slots[s]["attrs"].pop(k, None)
            else:
                self.slots[s]["attrs"][k] = v
        return bs.other("cad.entities.set", {"uids": [self.uid[s]], "attrs": attrs, "operation": "attributes"})

    def delete(self, fid):
        s = self.slot_of(fid)
        u = self.uid.pop(s)
        del self.slots[s]
        return bs.other("cad.entities.delete", {"uids": [u]})

    def add_polygon(self, x, y):
        pts = [{"x": px, "y": py} for px, py in square(x, y)[:-1]]
        s = self.next
        self.next += 1
        self.slots[s] = {"kind": "polygon", "attrs": {}, "pts": pts}
        self.uid[s] = None
        return bs.other("cad.polygon.create", {"layerId": LAYER, "pts": pts})


EDITS = [("set", 4, {"malik": "Veli"}), ("set", 5, {"parsel_no": "50"}), ("set", 6, {"malik": "Mehmet"}),
         ("delete", 7, None), ("set", 8, {"malik": "Selin"}), ("delete", 9, None), ("set", 10, {"malik": "Ortak"}),
         ("add", (487400.0, 4420000.0), None)]


def edit_steps(layer, edits=EDITS):
    out = []
    for what, fid, attrs in edits:
        if what == "set":
            out.append(layer.set(fid, attrs))
        elif what == "delete":
            out.append(layer.delete(fid))
        else:
            out.append(layer.add_polygon(*fid))
    return out


def same_object(a, b):
    """Kind, geometry and attributes the same (numbers as numbers)."""
    return json.dumps(essence(a), sort_keys=True) == json.dumps(essence(b), sort_keys=True)


def reconcile(layer, table_name, take=None, rest=None):
    """The answer of a reconcile of `layer` against the file `table_name` (the first file the base), and what the
    layer and the table become: (answer without source and revision, the layer after, the rows written)."""
    base = {r["uid"]: obj(r) for r in rows(A).values()}
    table = {r["uid"]: obj(r) for r in rows(table_name).values()}
    mine = {u: o for s, o in layer.slots.items() if (u := layer.uid[s]) is not None}
    new = [s for s, u in layer.uid.items() if u is None]
    take = take or {}
    ids = sorted(set(base) | set(mine) | set(table), key=lambda u: uuid.UUID(u).bytes)

    def did(b, now):
        if b is None:
            return "added" if now is not None else None
        if now is None:
            return "removed"
        return None if same_object(b, now) else "changed"

    conflicts, ways, agreed = [], {}, 0
    for u in ids:
        b, l, t = base.get(u), mine.get(u), table.get(u)
        ld, td = did(b, l), did(b, t)
        if ld is None and td is None:
            continue
        if td is None:
            ways[u] = "layer"
        elif ld is None:
            ways[u] = "table"
        elif ld == "removed" and td == "removed":
            ways[u] = "agreed"
        elif "removed" in (ld, td):
            c = {"object": u, "kind": "removedInLayer" if ld == "removed" else "removedInTable", "geometry": True}
            # The side that holds it gives every attribute; the other has none.
            held, side = (t, "table") if ld == "removed" else (l, "layer")
            if held["attrs"]:
                c["fields"] = [{"name": n, side: v} for n, v in sorted(held["attrs"].items())]
            conflicts.append(c)
            ways[u] = c
        elif same_object(l, t):
            ways[u] = "agreed"
        else:
            la, ta = l["attrs"], t["attrs"]
            fields = []
            for n in sorted(set(la) | set(ta)):
                if la.get(n) != ta.get(n):
                    f = {"name": n}
                    if n in la:
                        f["layer"] = la[n]
                    if n in ta:
                        f["table"] = ta[n]
                    fields.append(f)
            geom = json.dumps({k: v for k, v in l.items() if k != "attrs"}, sort_keys=True) != json.dumps(
                {k: v for k, v in t.items() if k != "attrs"}, sort_keys=True)
            c = {"object": u, "kind": "changed", "geometry": geom}
            if fields:
                c["fields"] = fields
            conflicts.append(c)
            ways[u] = c
        if ways[u] == "agreed":
            agreed += 1
    for c in conflicts:
        side = take.get(c["object"], rest)
        if side:
            c["side"] = side
    unresolved = sum(1 for c in conflicts if "side" not in c)

    # The layer after: the table's side taken into it.
    after = {s: copy.deepcopy(o) for s, o in layer.slots.items()}
    uid_after = dict(layer.uid)
    frm = {"added": 0, "modified": 0, "removed": 0}
    nxt = layer.next
    for u in ids:
        w = ways.get(u)
        if w is None or w in ("layer", "agreed"):
            continue
        if isinstance(w, dict) and w.get("side") == "layer":
            continue
        counted = not (isinstance(w, dict) and "side" not in w)
        s = next((s for s, x in uid_after.items() if x == u), None)
        t = table.get(u)
        if s is not None and t is not None:
            after[s] = copy.deepcopy(t)
            key = "modified"
        elif t is not None:
            after[nxt] = copy.deepcopy(t)
            uid_after[nxt] = u
            nxt += 1
            key = "added"
        elif s is not None:
            del after[s]
            del uid_after[s]
            key = "removed"
        else:
            continue
        if counted:
            frm[key] += 1
    # The write: the layer after against the table.
    to = {"added": 0, "modified": 0, "removed": 0}
    held = {uid_after[s]: o for s, o in after.items() if uid_after[s] is not None}
    for u, o in held.items():
        if u not in table:
            to["added"] += 1
        elif not same_object(o, table[u]):
            to["modified"] += 1
    to["added"] += sum(1 for s in after if uid_after[s] is None)
    to["removed"] = sum(1 for u in table if u not in held)
    answer = {"fromTable": frm, "toTable": to, "agreed": agreed}
    if conflicts:
        answer["conflicts"] = conflicts
    answer["unresolved"] = unresolved
    return answer, after, uid_after


def answer_of(core, source):
    return {**core, "source": source, "revision": "$current"}


def fields_of(after):
    return {str(s): {**o, "layerId": LAYER} for s, o in sorted(after.items())}


def bound(tmp, edits=EDITS, then=B):
    """The steps up to a reconcile: the first file bound, the layer edited, the file changed in QGIS."""
    layer = Layer()
    steps = [copy_step(A, tmp), link_step(tmp), *edit_steps(layer, edits)]
    if then:
        steps.append(copy_step(then, tmp))
    return layer, steps


def plan_case(name, tmp, inp=None, take_=None, rest=None, edits=EDITS, then=B):
    layer, steps = bound(tmp, edits, then)
    core, _, _ = reconcile(layer, then or A, take_, rest)
    inp = {"layer": LAYER, **(inp or {})}
    steps.append(bs.step(inp, bs.done(answer_of(core, bs.sha(then or A))), op="plan"))
    return bs.case(name, *steps), layer


def ids_after(after):
    return [1, *sorted(after)]


def cases():
    a, b = rows(A), rows(B)
    u = {fid: r["uid"] for fid, r in a.items()}
    out = []
    c, _ = plan_case("plan iki yanın değişikliklerini ve dört çatışmayı söyler, hiçbir şey yazmaz", "uz-plan.gpkg")
    out.append(c)
    c, _ = plan_case("kalanlar katmanın yanına: tabloya yazılacaklar artar", "uz-katman.gpkg", {"rest": "layer"},
                     rest="layer")
    out.append(c)
    c, _ = plan_case("kalanlar tablonun yanına: katmana alınacaklar artar", "uz-tablo.gpkg", {"rest": "table"},
                     rest="table")
    out.append(c)
    take_ = {u[4]: "table", u[5]: "layer"}
    c, _ = plan_case("çatışmalar tek tek, kalanlar katmanın yanına", "uz-tek.gpkg",
                     {"take": [{"object": u[4], "side": "table"}, {"object": u[5], "side": "layer"}],
                      "rest": "layer"}, take_=take_, rest="layer")
    out.append(c)
    c, _ = plan_case("yalnız tablo değişti: hepsi katmana alınır, çatışma yok", "uz-yalniz-tablo.gpkg", edits=[])
    out.append(c)
    c, _ = plan_case("yalnız katman değişti: hepsi tabloya yazılır", "uz-yalniz-katman.gpkg", then=None)
    out.append(c)
    c, _ = plan_case("hiçbir yan değişmedi: hepsi sıfır", "uz-degismedi.gpkg", edits=[], then=None)
    out.append(c)
    c, _ = plan_case("ikisi de sildiyse uyuşur", "uz-ikisi-sildi.gpkg", edits=[("delete", 3, None),
                                                                              ("delete", 6, None)])
    out.append(c)

    # Executing with every conflict resolved: the table's side into the layer, the layer's to the file.
    for name, tmp, rest in (("yürütme: kalanlar tablonun yanına, katman tablonun hâlini alır", "uz-yurut-tablo.gpkg",
                             "table"),
                            ("yürütme: kalanlar katmanın yanına, dosyaya katmanınkiler yazılır", "uz-yurut-katman.gpkg",
                             "layer")):
        layer, steps = bound(tmp)
        core, after, _ = reconcile(layer, B, rest=rest)
        steps.append(bs.step({"layer": LAYER, "rest": rest}, bs.done(answer_of(core, ANY)),
                             {"ids": ids_after(after), "fields": fields_of(after), "revision": "changed",
                              "canUndo": True}))
        # The base is taken again: the two match now.
        steps.append(bs.step({"layer": LAYER},
                             bs.done({"fromTable": {"added": 0, "modified": 0, "removed": 0},
                                      "toTable": {"added": 0, "modified": 0, "removed": 0}, "agreed": 0,
                                      "unresolved": 0, "source": ANY, "revision": "$current"}), op="plan"))
        out.append(bs.case(name, *steps))
    layer, steps = bound("uz-geri.gpkg")
    core, after, _ = reconcile(layer, B, rest="table")
    before = fields_of(layer.slots)
    steps.append(bs.step({"layer": LAYER, "rest": "table"}, bs.done(answer_of(core, ANY)),
                         {"revision": "changed", "canUndo": True}))
    steps.append({"op": "undo", "returns": "Uzlaştır: parsel",
                  "expect": {"ids": ids_after(layer.slots), "fields": before, "revision": "changed"}})
    out.append(bs.case("geri almak katmanı uzlaştırmadan önceki hâline döndürür", *steps))
    layer, steps = bound("uz-yalniz-tablo-yurut.gpkg", edits=[])
    core, after, _ = reconcile(layer, B)
    steps.append(bs.step({"layer": LAYER}, bs.done(answer_of(core, ANY)),
                         {"ids": ids_after(after), "fields": fields_of(after), "revision": "changed",
                          "canUndo": True}))
    out.append(bs.case("çatışma yoksa yan seçmeden yürür: katman tablonun hâlini alır", *steps))
    c, _ = plan_case("geçerleme: yan seçilmişse yürür", "uz-gecerle.gpkg", {"rest": "layer"}, rest="layer")
    c["steps"][-1] = bs.step({"layer": LAYER, "rest": "layer"}, bs.done(None), op="validate")
    out.append(c)

    # Refusals.
    unresolved_msg = ("4 çatışma çözülmedi; hiçbir şey yazılmadı. Her biri için take ile ya da kalanlar için rest ile "
                      "bir yan seçin (layer: katmanınki, table: tablonunki).")
    layer, steps = bound("uz-cozulmedi.gpkg")
    steps.append(bs.step({"layer": LAYER}, bs.conflict("unresolved_conflicts", unresolved_msg, "take")))
    out.append(bs.case("çözülmemiş çatışmayla yürümez, hiçbir şey yazılmaz", *steps))
    layer, steps = bound("uz-cozulmedi-g.gpkg")
    steps.append(bs.step({"layer": LAYER}, bs.conflict("unresolved_conflicts", unresolved_msg, "take"),
                         op="validate"))
    out.append(bs.case("geçerleme çözülmemiş çatışmayı reddeder", *steps))
    layer, steps = bound("uz-cozulmedi-tek.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[4], "side": "layer"}]},
                         bs.conflict("unresolved_conflicts", unresolved_msg.replace("4 çatışma", "3 çatışma"), "take")))
    out.append(bs.case("biri çözülse de kalanlar yürütmeyi durdurur", *steps))
    layer, steps = bound("uz-cakisma-degil.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[1], "side": "layer"}]},
                         bs.failed("not_a_conflict", f"{u[1]} bir çatışma değil; yan yalnız plan'ın conflicts'indeki "
                                   "nesneler için seçilir.", "take.0.object"), op="plan"))
    out.append(bs.case("yalnız tablonun değiştirdiği nesne için yan seçilmez", *steps))
    layer, steps = bound("uz-kimlik-degil.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[4], "side": "layer"},
                                                    {"object": "parsel-4", "side": "table"}]},
                         bs.failed("not_a_conflict", "parsel-4 bir çatışma değil; yan yalnız plan'ın conflicts'indeki "
                                   "nesneler için seçilir.", "take.1.object"), op="plan"))
    out.append(bs.case("kalıcı kimlik olmayan nesne", *steps))
    layer, steps = bound("uz-iki-kez.gpkg")
    steps.append(bs.step({"layer": LAYER, "take": [{"object": u[4], "side": "layer"},
                                                    {"object": u[4], "side": "table"}]},
                         bs.failed("invalid_input", f"{u[4]} için iki kez yan seçilmiş; her çatışma bir kez.",
                                   "take.1.object"), op="plan"))
    out.append(bs.case("bir çatışmaya iki kez yan seçilmez", *steps))
    layer, steps = bound("uz-kayip.gpkg", edits=[*EDITS, ("set", 8, {"durum": "x"})])
    core, _, _ = reconcile(layer, B, rest="layer")
    steps.append(bs.step({"layer": LAYER, "rest": "layer"},
                         bs.done(answer_of(core, bs.sha(B)),
                                 [{"code": "export_losses", "message": "parsel: katmanın alanı olmayan öznitelikler "
                                   "yazılmadı (durum); Alanlar penceresinden alan yapın.", "path": "layer"}]),
                         op="plan"))
    out.append(bs.case("alanı olmayan öznitelik yazılmaz, söylenir", *steps))
    layer, steps = bound("uz-nokta.gpkg", edits=[])
    steps.insert(2, bs.other("cad.point.create", {"layerId": LAYER, "p": gi.P(487500, 4420100)}))
    steps.insert(3, {"op": "captureUid", "id": 12, "as": "nokta"})
    steps.append(bs.step({"layer": LAYER}, bs.failed("geometry_mismatch", "$uid:nokta kimlikli nesne tabloya "
                                                     "yazılamaz: POINT geometrisi MULTIPOLYGON sütununa yazılamaz.",
                                                     "layer"), op="plan"))
    out.append(bs.case("tablonun sütununa uymayan nesne", *steps))
    out.append(bs.case("bağlı olmayan katman",
                       bs.step({"layer": "cizim"},
                               bs.failed("source_not_found", "“cizim” katmanı bir kaynağa bağlı değil; önce "
                                         "cad.sources.edit ile bağlayın.", "layer"), op="plan")))

    def link_to(layer_id, name=A, **over):
        s = {**bs.link("parsel", layer_id, name=name), "digest": "f" * 64, **over}
        return s

    bagli = bs.layer("bagli")
    out.append(bs.case("tabanı olmayan bağ uzlaştırılmaz",
                       bs.step({"layer": "bagli"},
                               bs.failed("no_source_base", "“bagli” katmanının dosyayla son eşleşmesinden tabanı yok "
                                         "(bu sürümden önce bağlanmış ya da 200000 nesneden büyük); iki yanın "
                                         "değişiklikleri ayrılamaz. Önce yazın (cad.sources.sync write) ya da "
                                         "tablodan okuyun (read).", "layer"), op="plan"),
                       setup_=bs.setup([bagli], sources=[link_to("bagli")])))
    out.append(bs.case("çizimde olmayan bağlı katman",
                       bs.step({"layer": "silinen"},
                               bs.failed("layer_not_found", "Bağlı “silinen” katmanı çizimde yok; bağı çözün "
                                         "(cad.sources.edit unlink).", "layer"), op="plan"),
                       setup_=bs.setup(sources=[link_to("silinen")])))
    out.append(bs.case("kilitli katman",
                       bs.step({"layer": "kilitli"},
                               bs.failed("layer_locked", "“kilitli” katmanı kilitli; kilidi Katmanlar panelinden "
                                         "açın.", "layer"), op="plan"),
                       setup_=bs.setup(sources=[link_to("kilitli")])))
    gone = link_to("bagli", path="$fixture:geopackage/v1/yok.gpkg")
    out.append(bs.case("yerinde olmayan dosya",
                       bs.step({"layer": "bagli"},
                               bs.failed("source_gone", f"{gone['path']} yerinde yok; dosyanın yeni yerini "
                                         "cad.sources.edit relink ile verin.", "layer"), op="plan"),
                       setup_=bs.setup([bagli], sources=[gone])))
    ro = link_to("bagli", writable=False)
    out.append(bs.case("salt okunur tablo",
                       bs.step({"layer": "bagli"},
                               bs.failed("source_readonly", f"{ro['path']} dosyasının “parsel” tablosuna yazılamaz "
                                         "(salt okunur); uzlaştırmak katmanın yanını yazar.", "layer"), op="plan"),
                       setup_=bs.setup([bagli], sources=[ro])))
    broken = {**link_to("bagli", name="bozuk.gpkg")}
    out.append(bs.case("okunmayan dosya",
                       bs.step({"layer": "bagli"},
                               {"status": "failed", "error": {"code": "import_unreadable", "message": ANY,
                                                               "path": "layer"}}, op="plan"),
                       setup_=bs.setup([bagli], sources=[broken])))
    missing = {**link_to("bagli"), "table": "yok"}
    out.append(bs.case("dosyada olmayan tablo",
                       bs.step({"layer": "bagli"},
                               {"status": "failed", "error": {"code": "table_not_found", "message": ANY,
                                                               "path": "layer"}}, op="plan"),
                       setup_=bs.setup([bagli], sources=[missing])))
    local = bs.setup([bagli], sources=[link_to("bagli")])
    local["settings"]["srid"] = 0
    out.append(bs.case("koordinat sistemi olmayan projede başka sistemdeki tablo",
                       bs.step({"layer": "bagli"},
                               bs.failed("crs_mismatch", "“bagli” eşitlenemez: tablo EPSG:5256 sisteminde; projenin "
                                         "koordinat sistemi yok (yerel proje), çevrilemez.", "layer"), op="plan"),
                       setup_=local))
    derived = {**link_to("bagli"), "table": "Parsel", "path": "$fixture:gis-derivation/v1/ada.gpkg",
               "revision": hashlib.sha256((ROOT / "fixtures/gis-derivation/v1/ada.gpkg").read_bytes()).hexdigest()}
    out.append(bs.case("çizimden türetilmiş tablo uzlaştırılmaz",
                       bs.step({"layer": "bagli"},
                               {"status": "failed", "error": {"code": "derived_dataset", "message": ANY,
                                                               "path": "layer"}}, op="plan"),
                       setup_=bs.setup([bagli], sources=[derived])))
    return out


def written():
    """kentos-uzlasti.gpkg (tests/all/sources.rs, rest: layer) by OGR: each row as the reconcile says."""
    path = DIR / WRITTEN
    if not path.exists():
        return f"{WRITTEN} yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all reconcile"
    layer = Layer()
    edit_steps(layer)
    _, after, uid_after = reconcile(layer, B, rest="layer")
    got = rows(WRITTEN)
    by_uid = {r["uid"]: obj(r) for r in got.values()}
    want = {uid_after[s]: o for s, o in after.items() if uid_after[s] is not None}
    news = [o for s, o in after.items() if uid_after[s] is None]
    problems = []
    for u, o in want.items():
        g = by_uid.pop(u, None)
        if g is None:
            problems.append(f"{u}: satır yok")
        elif not same_object(_multi(o), g):
            problems.append(f"{u}: {g} ≠ {o}")
    for o in news:
        match = next((k for k, g in by_uid.items() if same_object(_multi(o), g)), None)
        if match is None:
            problems.append(f"yeni nesne yazılmamış: {o}")
        else:
            by_uid.pop(match)
    problems.extend(f"{k}: fazla satır" for k in by_uid)
    return "; ".join(problems) or None


def _multi(o):
    """An object as its row reads back from a MULTIPOLYGON column: the import's reading of a one-part polygon."""
    return o


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    if "--written" in sys.argv:
        why = written()
        if why:
            print(why)
            return 1
        print(f"{WRITTEN}: uzlaştırmanın yazdığı satırlar doğru")
        return 0
    data = {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.reconcile",
            "commandVersion": 1, "setup": bs.SETUP, "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum güncel")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
