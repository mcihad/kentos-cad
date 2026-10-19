#!/usr/bin/env python3
"""The shared cases of the many-to-many references in bound layers and of layers written together (docs/adr/0532):
`cad.sources.relate`, `cad.records.page`, `cad.sources.sync`'s `with` and `cad.forms.submit`'s `commit`.

    python3 scripts/fixtures/source_relations_cases.py           # writes fixtures/commands/v1/cad.{sources.relate,records.page,sources.sync.together,forms.submit.commit}.json
    python3 scripts/fixtures/source_relations_cases.py --check   # compares them with the ones on disk
    /usr/bin/python3 scripts/fixtures/source_relations_cases.py --verify   # reads the files KentOS wrote with sqlite3 and OGR

Nothing here uses KentOS code. The files the cases read are written by `source_relations_files.py` with sqlite3 from the
GeoPackage core tables and OGC 18-000's related tables; a file's revision is its SHA-256 by hashlib; the state of a
reference follows from what the file holds (a `kentos_references` relation with a valid mapping table is mapped, another
well-formed relation is foreign, one whose mapping table is not as the profile says is broken, none is text). The mapping
rows a layer's references make are the pairs of the child's fid and each parent id's fid, in the child's fid order and the
field's order; a duplicate id in a field is one row. A page is the records in lasting-id order after the layers' filters,
the cursor the last id; a relation's parents come in the field's order. Messages are the ADR's words.
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import source_relations_files as files  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
ANY = "$any"
SAME = {"revision": "same"}
FAKE = "f" * 64
STALE = "e" * 64


def fx(name):
    return f"$fixture:geopackage/v1/{name}"


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def conflict(code, message, path):
    return {"status": "conflict", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, expect=None, op="execute", command=None):
    s = {"op": op, "input": inp, "result": result, "expect": expect or SAME}
    if command:
        s["command"] = command
    return s


def other(command, inp, expect=None):
    """A step of another command whose answer is not this file's matter."""
    return step(inp, {"status": "completed", "output": ANY, "warnings": ANY}, expect or {"revision": "changed"},
                command=command)


def case(name, *steps, setup_=None):
    c = {"name": name, "steps": list(steps)}
    if setup_ is not None:
        c["setup"] = setup_
    return c


def node(id_, name, fields=None):
    n = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
         "style": copy.deepcopy(STYLE), "children": []}
    if fields:
        n["fields"] = fields
    return n


MALIK, PARSEL, BINA = "import-malik", "import-parsel", "import-bina"
MALIK_FIELDS = [{"name": "ad", "kind": "text"}, {"name": "tur", "kind": "text"}]
PARSEL_FIELDS = [{"name": "ada", "kind": "text"}, {"name": "parsel_no", "kind": "text"},
                 {"name": "malikler", "kind": "text", "multiple": True}]
BINA_FIELDS = [{"name": "parsel", "kind": "text"}, {"name": "kat", "kind": "text"}]
RELATIONS = [
    {"id": "parsel-malik", "name": "Parselin malikleri", "parent": MALIK, "child": PARSEL, "field": "malikler"},
    {"id": "bina-parsel", "name": "Parselin binaları", "parent": PARSEL, "child": BINA, "field": "parsel"},
]


def link(layer, table, n, name, writable=True, revision=None):
    return {"id": f"kaynak-{n}", "kind": "geopackage", "path": fx(name), "table": table, "layer": layer,
            "revision": revision or (files.sha(name) if (files.DIR / name).exists() else FAKE), "digest": FAKE, "srid": 5256, "writable": writable}


def static(name="iliskili.gpkg", sources="all", relations=RELATIONS, writable=True, revision=None, malik_name=None,
           missing=False, readonly=()):
    """A drawing with the three layers, their fields and the project's relations, and the layers bound to the tables of
    `name`: no objects (the cases that need them link the tables)."""
    malik_name = malik_name or name
    path = "yok.gpkg" if missing else name
    srcs = []
    if sources in ("all", "child", "parent"):
        if sources in ("all", "parent"):
            srcs.append(link(MALIK, "malik", 1, "yok.gpkg" if missing else malik_name,
                             writable and MALIK not in readonly, revision))
        if sources in ("all", "child"):
            srcs.append(link(PARSEL, "parsel", 2, path, writable and PARSEL not in readonly, revision))
        if sources == "all":
            srcs.append(link(BINA, "bina", 3, path, writable and BINA not in readonly, revision))
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000, "workspace": "gis", "relations": copy.deepcopy(relations)}
    if srcs:
        settings["sources"] = srcs
    return {"format": "kentos.document", "version": 1, "name": "İlişkili kayıtlar", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [node("cizim", "Çizim"), node(MALIK, "Malik", copy.deepcopy(MALIK_FIELDS)),
                       node(PARSEL, "Parsel", copy.deepcopy(PARSEL_FIELDS)), node(BINA, "Bina", copy.deepcopy(BINA_FIELDS))],
            "activeLayer": "cizim",
            "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": {"x": 487500.0, "y": 4420500.0}}],
            "styles": {"items": [], "categories": []}}


def preamble(name="iliskili.gpkg"):
    """The three tables linked to layers (slots 2–4 the owners, 5–7 the parcels, 8–9 the buildings), the parcels'
    `malikler` a many-to-many reference, the project's two relations."""
    empty = {"format": "kentos.document", "version": 1, "name": "İlişkili kayıtlar",
             "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                          "plotScale": 1000, "workspace": "gis"},
             "origin": {"x": 0.0, "y": 0.0}, "layers": [node("cizim", "Çizim")], "activeLayer": "cizim",
             "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": {"x": 487500.0, "y": 4420500.0}}],
             "styles": {"items": [], "categories": []}}
    steps = [other("cad.sources.edit", {"edit": {"kind": "link", "path": fx(name), "table": t}}) for t in ("malik", "parsel", "bina")]
    steps.append(other("cad.layers.edit", {"operation": "fields", "layer": PARSEL, "fields": copy.deepcopy(PARSEL_FIELDS)}))
    steps += [other("cad.relations.edit", {"edit": {"kind": "create", "relation": r}}) for r in RELATIONS]
    return empty, steps


def slot(table, fid):
    return {"malik": 1, "parsel": 4, "bina": 7}[table] + fid


def set_attrs(table, fid, attrs, expect=None):
    return other("cad.entities.set", {"uids": [f"$uidOf:{slot(table, fid)}"], "operation": "attributes", "attrs": attrs},
                 expect)


# ── cad.sources.relate ─────────────────────────────────────────────────────

def related(relation, state, mapping=None, rows=0, child=None, parent=None, why=None, source=None):
    out = {"relation": relation, "state": state}
    if mapping:
        out["mapping"] = mapping
    out["rows"] = rows
    if child:
        out["child"] = child
    if parent:
        out["parent"] = parent
    if why:
        out["why"] = why
    if source:
        out["source"] = source
    out["revision"] = "$current"
    return out


def relate_in(relation, op, mapping=None):
    i = {"relation": relation, "action": op}
    if mapping:
        i["mapping"] = mapping
    return i


NAME = {"parsel-malik": "Parselin malikleri", "bina-parsel": "Parselin binaları"}
FOREIGN_WHY = "“attributes” ilişkisi KentOS'un “kentos_references” temsili değil"
BROKEN_WHY = "“parsel_malik” eşleme tablosunda related_id sütunu yok"
REVERSE_WHY = ("“kentos_references” ilişkisi ters yönde (taban malik, ilgili parsel); KentOS tabanı alt katman yazar")


def unbound(op):
    return failed("relation_unbound", f"“{NAME['parsel-malik']}” ilişkisinin üst ({MALIK}) ya da alt ({PARSEL}) katmanı "
                  "bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın.", "relation")


def refused_map(name, relation_state, code, message, path="relation", mapping=None, setup_=None, rel="parsel-malik"):
    return case(name, step(relate_in(rel, "map", mapping), failed(code, message, path), op="plan"), setup_=setup_)


def relate_cases():
    sha = files.sha
    t = "iliskili.gpkg"
    cases = []
    check = lambda name, setup_, result: case(name, step(relate_in("parsel-malik", "check"), result), setup_=setup_)
    cases += [
        check("metin sütunu: iki katman aynı dosyanın tablolarına bağlı, dosyada ilişki yok",
              static(t), done(related("parsel-malik", "text", child="parsel", parent="malik", source=sha(t)))),
        check("eşli ilişki: KentOS'un ilişkisi ve eşleme tablosunun satırları",
              static("iliskili-eslenmis.gpkg"),
              done(related("parsel-malik", "mapped", "parsel_malik", 4, "parsel", "malik", source=sha("iliskili-eslenmis.gpkg")))),
        check("eşli ilişki: üst tabloda alan sütunu olmasa da",
              static("iliskili-eslenmis-sutunsuz.gpkg"),
              done(related("parsel-malik", "mapped", "parsel_malik", 4, "parsel", "malik",
                           source=sha("iliskili-eslenmis-sutunsuz.gpkg")))),
        check("başka temsilli ilişki yabancıdır: nedeni söylenir",
              static("iliskili-yabanci.gpkg"),
              done(related("parsel-malik", "foreign", child="parsel", parent="malik", why=FOREIGN_WHY,
                           source=sha("iliskili-yabanci.gpkg")))),
        check("profile uymayan ilişki bozuktur",
              static("iliskili-bozuk.gpkg"),
              done(related("parsel-malik", "broken", child="parsel", parent="malik", why=BROKEN_WHY,
                           source=sha("iliskili-bozuk.gpkg")))),
        check("ters yönde yazılmış ilişki yabancıdır",
              static("iliskili-ters.gpkg"),
              done(related("parsel-malik", "foreign", child="parsel", parent="malik", why=REVERSE_WHY,
                           source=sha("iliskili-ters.gpkg")))),
        case("tek kimlikli alan eşlenecek bir şey değildir",
             step(relate_in("bina-parsel", "check"),
                  done(related("bina-parsel", "single", child="bina", parent="parsel", source=sha(t)))),
             setup_=static(t)),
        check("bağlı olmayan katman: bağlı değil", static(sources="none"), done(related("parsel-malik", "unbound"))),
        check("yalnız alt katman bağlı", static(sources="child"), done(related("parsel-malik", "unbound"))),
        check("yalnız üst katman bağlı", static(sources="parent"), done(related("parsel-malik", "unbound"))),
        check("katmanlar ayrı dosyalarda: bölünmüş",
              static(t, malik_name="malikler.gpkg"),
              done(related("parsel-malik", "split", child="parsel", parent="malik"))),
        case("olmayan ilişki",
             step(relate_in("yok", "check"),
                  failed("unknown_relation", "“yok” kimlikli bir ilişki yok; cad.schema.describe ilişkileri listeler.",
                         "relation")),
             setup_=static(t)),
        case("dosya yerinde yok",
             step(relate_in("parsel-malik", "check"),
                  failed("source_gone", f"{fx('yok.gpkg')} yerinde yok; dosyanın yeni yerini cad.sources.edit relink "
                         "ile verin.", "relation")),
             setup_=static(t, missing=True)),
    ]
    # map: the plan says what it would write.
    cases += [
        case("eşleme planı: yazılacak ilişki ve eşleme tablosunun adı; nesnesiz katmanlarda satır yok",
             step(relate_in("parsel-malik", "map"),
                  done(related("parsel-malik", "mapped", "parsel_malik", 0, "parsel", "malik", source=sha(t))), op="plan"),
             setup_=static(t)),
        case("eşleme tablosunun adı verilebilir",
             step(relate_in("parsel-malik", "map", "sahiplik"),
                  done(related("parsel-malik", "mapped", "sahiplik", 0, "parsel", "malik", source=sha(t))), op="plan"),
             setup_=static(t)),
        case("eşleme adı rakamla başlayamaz",
             step(relate_in("parsel-malik", "map", "1sahiplik"),
                  failed("invalid_mapping", "“1sahiplik” bir eşleme tablosu adı olamaz: harfle ya da _ ile başlamalı, "
                         "yalnız harf, rakam ve _ taşımalı.", "mapping"), op="plan"),
             setup_=static(t)),
        case("eşleme adı GeoPackage'ın tablo adlarıyla başlayamaz",
             step(relate_in("parsel-malik", "map", "gpkg_sahiplik"),
                  failed("invalid_mapping", "“gpkg_sahiplik” bir eşleme tablosu adı olamaz: GeoPackage'ın ve SQLite'ın "
                         "kendi tablo adlarıyla başlayamaz.", "mapping"), op="plan"),
             setup_=static(t)),
        case("eşleme adı en çok 64 karakter",
             step(relate_in("parsel-malik", "map", "a" * 65),
                  failed("invalid_mapping", f"“{'a' * 65}” bir eşleme tablosu adı olamaz: en çok 64 karakter olur.",
                         "mapping"), op="plan"),
             setup_=static(t)),
        case("dosyada o adda tablo varsa (büyük küçük harfe bakılmaz) eşleme reddedilir",
             step(relate_in("parsel-malik", "map", "PARSEL"),
                  failed("invalid_mapping", "Dosyada “PARSEL” adlı bir tablo zaten var; başka bir ad verin.", "mapping"),
                  op="plan"),
             setup_=static(t)),
        case("eşli ilişki ikinci kez eşlenmez",
             step(relate_in("parsel-malik", "map"),
                  failed("relation_mapped", "“Parselin malikleri” ilişkisi dosyada zaten eşli.", "relation"), op="plan"),
             setup_=static("iliskili-eslenmis.gpkg")),
        case("yabancı temsilin üzerine yazılmaz, metne de çevrilmez",
             step(relate_in("parsel-malik", "map"),
                  failed("relation_unsupported", "parsel ile malik tabloları dosyada başka bir temsille ilişkili: "
                         f"{FOREIGN_WHY}. KentOS onun üzerine yazmaz ve metne çevirmez.", "relation"), op="plan"),
             setup_=static("iliskili-yabanci.gpkg")),
        case("bozuk ilişkinin üzerine yazılmaz",
             step(relate_in("parsel-malik", "map"),
                  failed("relation_unsupported", "parsel ile malik tabloları dosyada başka bir temsille ilişkili: "
                         f"{BROKEN_WHY}. KentOS onun üzerine yazmaz ve metne çevirmez.", "relation"), op="plan"),
             setup_=static("iliskili-bozuk.gpkg")),
        case("ters yöndeki ilişkinin üzerine yazılmaz",
             step(relate_in("parsel-malik", "map"),
                  failed("relation_unsupported", "parsel ile malik tabloları dosyada başka bir temsille ilişkili: "
                         f"{REVERSE_WHY}. KentOS onun üzerine yazmaz ve metne çevirmez.", "relation"), op="plan"),
             setup_=static("iliskili-ters.gpkg")),
        case("tek kimlikli alan eşlenmez",
             step(relate_in("bina-parsel", "map"),
                  failed("relation_not_multiple", "“Parselin binaları” ilişkisinin ‘parsel’ alanı tek kimlik tutuyor "
                         "(çok değerli değil); eşlenecek bir çoktan çoğa başvuru yok. Alanı Alanlar penceresinden çok "
                         "değerli yapın.", "relation"), op="plan"),
             setup_=static(t)),
        case("bağlı olmayan katmanlar eşlenmez",
             step(relate_in("parsel-malik", "map"), unbound("map"), op="plan"), setup_=static(sources="none")),
        case("ayrı dosyalardaki katmanlar eşlenmez: tek işlem ikisini tutamaz",
             step(relate_in("parsel-malik", "map"),
                  failed("transaction_split", "“Parselin malikleri” ilişkisinin katmanları tek GeoPackage dosyasında "
                         f"değil ({fx(t)} ve {fx('malikler.gpkg')}); eşleme iki tabloyu tek işlemde ister.", "relation"),
                  op="plan"),
             setup_=static(t, malik_name="malikler.gpkg")),
        case("katmanlar dosyayla son eşleştiğinden beri dosya değişmişse eşlenmez",
             step(relate_in("parsel-malik", "map"),
                  conflict("source_changed", f"{fx(t)} katmanlar onunla son eşleştiğinden beri değişti; hiçbir şey "
                           "yazılmadı. Önce tablodan okuyun (cad.sources.sync read).", "relation"), op="plan"),
             setup_=static(t, revision=STALE)),
        case("salt okunur tabloya eşleme yazılmaz",
             step(relate_in("parsel-malik", "map"),
                  failed("source_readonly", f"{fx(t)} dosyasının “parsel” tablosuna yazılamaz (salt okunur).", "relation"),
                  op="plan"),
             setup_=static(t, writable=False)),
        case("eşleme denetimi: yalnız denetler",
             step(relate_in("parsel-malik", "map"), done(None), op="validate"), setup_=static(t)),
        case("eşleme denetimi: reddi söyler",
             step(relate_in("parsel-malik", "map", "9"),
                  failed("invalid_mapping", "“9” bir eşleme tablosu adı olamaz: harfle ya da _ ile başlamalı, yalnız "
                         "harf, rakam ve _ taşımalı.", "mapping"), op="validate"),
             setup_=static(t)),
    ]
    # unmap
    cases += [
        case("eşlemeyi kaldırma planı: ilişki metin sütununa döner",
             step(relate_in("parsel-malik", "unmap"),
                  done(related("parsel-malik", "text", child="parsel", parent="malik", source=sha("iliskili-eslenmis.gpkg"))),
                  op="plan"),
             setup_=static("iliskili-eslenmis.gpkg")),
        case("eşli olmayan ilişkinin eşlemesi yok",
             step(relate_in("parsel-malik", "unmap"),
                  failed("relation_not_mapped", "“Parselin malikleri” ilişkisi dosyada eşli değil; kaldırılacak bir "
                         "eşleme yok.", "relation"), op="plan"),
             setup_=static(t)),
        case("tek kimlikli alanın eşlemesi yok",
             step(relate_in("bina-parsel", "unmap"),
                  failed("relation_not_mapped", "“Parselin binaları” ilişkisi dosyada eşli değil; kaldırılacak bir "
                         "eşleme yok.", "relation"), op="plan"),
             setup_=static(t)),
        case("yabancı ilişki KentOS'un eşlemesi değildir",
             step(relate_in("parsel-malik", "unmap"),
                  failed("relation_not_mapped", "“Parselin malikleri” ilişkisi dosyada eşli değil; kaldırılacak bir "
                         "eşleme yok.", "relation"), op="plan"),
             setup_=static("iliskili-yabanci.gpkg")),
        case("bağlı olmayan katmanlarda eşleme kaldırılmaz",
             step(relate_in("parsel-malik", "unmap"), unbound("unmap"), op="plan"), setup_=static(sources="none")),
    ]
    # map plans with objects: the rows the layers' references make
    empty, pre = preamble(t)
    rel = lambda rows: done(related("parsel-malik", "mapped", "parsel_malik", rows, "parsel", "malik", source=sha(t)))
    dangling = ("{uid} kayıtlı import-parsel katmanı nesnesinin ‘malikler’ alanı {id} kimliğini tutuyor; "
                "import-malik katmanında bu kimlikte nesne yok, eşleme satırı kurulamaz. Başvuruyu düzeltin.")
    ghost = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"
    cases += [
        case("eşleme planı: katmanların başvurularından dört satır (parsel, malik)",
             *pre, step(relate_in("parsel-malik", "map"), rel(4), op="plan"), setup_=empty),
        case("bir parselin maliki çıkarılınca satır eksilir",
             *pre, set_attrs("parsel", 1, {"malikler": files.owner_uid(1)}),
             step(relate_in("parsel-malik", "map"), rel(3), op="plan"), setup_=empty),
        case("alanda yinelenen kimlik tek satırdır",
             *pre, set_attrs("parsel", 1, {"malikler": f"{files.owner_uid(1)}; {files.owner_uid(2)}; {files.owner_uid(1)}"},
                        SAME),
             step(relate_in("parsel-malik", "map"), rel(4), op="plan"), setup_=empty),
        case("malikleri boşaltılan parselin satırı yok",
             *pre, set_attrs("parsel", 1, {"malikler": ""}),
             step(relate_in("parsel-malik", "map"), rel(2), op="plan"), setup_=empty),
        case("olmayan kimliğe giden başvuru eşlemeyi reddettirir",
             *pre, set_attrs("parsel", 3, {"malikler": ghost}),
             step(relate_in("parsel-malik", "map"),
                  failed("relation_dangling", dangling.format(uid=files.PARCELS[2][1], id=ghost), "layer"), op="plan"),
             setup_=empty),
        case("çizimde olup tabloda olmayan malike başvuru: tabloya önce yazılmalı",
             *pre,
             other("cad.records.create", {"layerId": MALIK, "rows": [{"attrs": {"ad": "Yeni Malik", "tur": "Gerçek"}}]}),
             {"op": "captureUid", "id": 10, "as": "yeni"},
             set_attrs("parsel", 3, {"malikler": "$uid:yeni"}),
             step(relate_in("parsel-malik", "map"),
                  failed("relation_dangling", f"{files.PARCELS[2][1]} kayıtlı import-parsel katmanı nesnesi $uid:yeni "
                         "kimlikli import-malik nesnesini tutuyor; o malik tablosunda yok. Üst katmanı da birlikte "
                         "yazın (with).", "layer"), op="plan"),
             setup_=empty),
        case("başka bir ilişki metin sütunu olarak kalmış dosyada da eşleme planı kurulur (eşli dosya ise reddedilir)",
             *preamble("iliskili-eslenmis.gpkg")[1],
             step(relate_in("parsel-malik", "map"),
                  failed("relation_mapped", "“Parselin malikleri” ilişkisi dosyada zaten eşli.", "relation"), op="plan"),
             setup_=preamble("iliskili-eslenmis.gpkg")[0]),
    ]
    return cases


# ── cad.records.page ───────────────────────────────────────────────────────

def owner_items():
    return [(u, {"ad": ad, "tur": tur}) for _, u, ad, tur in files.OWNERS]


def parcel_items():
    out = []
    for _, u, ada, no, owners in files.PARCELS:
        a = {"ada": ada, "parsel_no": no}
        if owners:
            a["malikler"] = "; ".join(files.owner_uid(o) for o in owners)
        out.append((u, a))
    return out


def building_items():
    return [(u, {"parsel": files.parcel_uid(p), "kat": kat}) for _, u, p, kat in files.BUILDINGS]


def by_uid(items):
    return sorted(items, key=lambda i: i[0])


def cut(items, limit=50, cursor=None):
    """A page of an ordered list: the records after `cursor`, at most `limit`, the next cursor, the total."""
    start = 0 if cursor is None else [u for u, _ in items].index(cursor) + 1
    chunk = items[start:start + limit]
    nxt = chunk[-1][0] if chunk and start + len(chunk) < len(items) else None
    return chunk, nxt, len(items)


TABLE = {MALIK: ("malik", 1, "iliskili.gpkg"), PARSEL: ("parsel", 2, "iliskili.gpkg"), BINA: ("bina", 3, "iliskili.gpkg")}


def paged(layer, items, limit=50, cursor=None, edited=False, source=True, total=None):
    chunk, nxt, n = cut(items, limit, cursor)
    out = {"layer": layer, "total": n if total is None else total,
           "records": [{"uid": u, "attrs": a} for u, a in chunk]}
    if nxt:
        out["next"] = nxt
    if source:
        table, n_, name = TABLE[layer]
        out["source"] = {"id": f"kaynak-{n_}", "kind": "geopackage", "table": table, "revision": files.sha(name),
                         "edited": edited}
    out["revision"] = "$current"
    return out


def page_in(**kw):
    return kw


def page_cases():
    empty, pre = preamble()
    sha = files.sha("iliskili.gpkg")
    owners, parcels, buildings = by_uid(owner_items()), by_uid(parcel_items()), by_uid(building_items())
    o1, o2, o3 = (files.owner_uid(i) for i in (1, 2, 3))
    p1, p2, p3 = (files.parcel_uid(i) for i in (1, 2, 3))
    child_of = lambda owner: [(u, a) for u, a in parcels if owner in a.get("malikler", "").split("; ")]
    case_ = lambda name, *steps: case(name, *pre, *steps, setup_=empty)
    ok = lambda inp, result: step(inp, done(result), op="execute")
    bad = lambda inp, code, message, path, kind=failed: step(inp, kind(code, message, path))
    cases = [
        case_("katmanın bütün kayıtları: kimlik sırasıyla, toplam ve kaynağın sürümüyle", ok({"layer": MALIK}, paged(MALIK, owners))),
        case_("parseller: öznitelikleriyle, çok değerli başvuru olduğu gibi", ok({"layer": PARSEL}, paged(PARSEL, parcels))),
        case_("binalar", ok({"layer": BINA}, paged(BINA, buildings))),
        case_("sayfa sayfa: ilk sayfa 2 kayıt ve imleç, ikinci sayfa kalan",
              ok({"layer": MALIK, "limit": 2}, paged(MALIK, owners, 2)),
              ok({"layer": MALIK, "limit": 2, "cursor": owners[1][0]}, paged(MALIK, owners, 2, owners[1][0]))),
        case_("tek kayıtlık sayfalar: son sayfada imleç yok",
              ok({"layer": PARSEL, "limit": 1, "cursor": parcels[1][0]}, paged(PARSEL, parcels, 1, parcels[1][0]))),
        case_("sözcükle arama büyük küçük harfe bakmaz ve Türkçe I'yı bilir: “SİVAS” Sivas Belediyesi'ni bulur",
              ok({"layer": MALIK, "query": "SİVAS BELEDİYESİ"}, paged(MALIK, [o for o in owners if o[1]["ad"] == "Sivas Belediyesi"]))),
        case_("arama harfi sadeleştirmez: “ayse” Ayşe'yi bulmaz",
              ok({"layer": MALIK, "query": "ayse"}, paged(MALIK, []))),
        case_("bütün sözcükler aranır",
              ok({"layer": MALIK, "query": "mehmet demir"}, paged(MALIK, [o for o in owners if o[1]["ad"] == "Mehmet Demir"]))),
        case_("eşleşen yoksa boş sayfa, toplam 0", ok({"layer": MALIK, "query": "yokyokyok"}, paged(MALIK, []))),
        case_("yalnız adı verilen öznitelikler döner",
              ok({"layer": MALIK, "fields": ["ad"]}, {**paged(MALIK, owners), "records": [{"uid": u, "attrs": {"ad": a["ad"]}} for u, a in owners]})),
        case_("malikin parselleri: alt katmanın alanı onu tutanlar (çoktan çoğa)",
              ok({"relation": "parsel-malik", "of": o2}, paged(PARSEL, child_of(o2)))),
        case_("tek parselli malik", ok({"relation": "parsel-malik", "of": o1}, paged(PARSEL, child_of(o1)))),
        case_("başka parselin maliki", ok({"relation": "parsel-malik", "of": o3, "side": "children"}, paged(PARSEL, child_of(o3)))),
        case_("parselin malikleri: üst katman, alanın sırasıyla (parents)",
              ok({"relation": "parsel-malik", "of": p1, "side": "parents"}, paged(MALIK, [o for o in owners if o[0] in (o1, o2)]))),
        case_("başka parselin malikleri", ok({"relation": "parsel-malik", "of": p2, "side": "parents"},
                                             paged(MALIK, [o for o in owners if o[0] in (o2, o3)]))),
        case_("maliksiz parsel: boş sayfa", ok({"relation": "parsel-malik", "of": p3, "side": "parents"}, paged(MALIK, []))),
        case_("alanın sırası kimlik sırası değilse parents alanın sırasıyla gelir, sayfalar da öyle",
              set_attrs("parsel", 1, {"malikler": f"{o2}; {o1}"}),
              ok({"relation": "parsel-malik", "of": p1, "side": "parents", "limit": 1},
                 paged(MALIK, [o for o in owners if o[0] == o2] + [o for o in owners if o[0] == o1], 1, edited=True)
                 | {"total": 2, "source": {"id": "kaynak-1", "kind": "geopackage", "table": "malik", "revision": sha, "edited": False}}),
              ok({"relation": "parsel-malik", "of": p1, "side": "parents", "limit": 1, "cursor": o2},
                 paged(MALIK, [o for o in owners if o[0] == o2] + [o for o in owners if o[0] == o1], 1, o2)
                 | {"total": 2, "source": {"id": "kaynak-1", "kind": "geopackage", "table": "malik", "revision": sha, "edited": False}})),
        case_("tek kimlikli ilişki: parselin binaları",
              ok({"relation": "bina-parsel", "of": p1}, paged(BINA, [b for b in buildings if b[1]["parsel"] == p1]))),
        case_("tek kimlikli ilişki: binanın parseli", ok({"relation": "bina-parsel", "of": files.BUILDINGS[1][1], "side": "parents"},
                                                         paged(PARSEL, [p for p in parcels if p[0] == p2]))),
        case_("katman süzgeci dışarıda bıraktığını ne kayıt ne toplam olarak gösterir",
              other("cad.layers.filter", {"layer": PARSEL, "filter": {"expression": "ada = '102'"}}),
              ok({"layer": PARSEL}, paged(PARSEL, [p for p in parcels if p[1]["ada"] == "102"])),
              ok({"relation": "parsel-malik", "of": o2}, paged(PARSEL, []))),
        case_("katmanda kaynağın bilmediği değişiklik varsa edited",
              set_attrs("malik", 1, {"tur": "Tüzel"}),
              ok({"layer": MALIK}, paged(MALIK, [(u, {**a, "tur": "Tüzel"} if u == o1 else a) for u, a in owners], edited=True))),
        case_("beklenen kaynak sürümü aynıysa okunur",
              ok({"layer": MALIK, "expectedSource": sha}, paged(MALIK, owners))),
        case_("kaynak sürümü ilk sayfadan farklıysa sonraki sayfa okunmaz",
              bad({"layer": MALIK, "expectedSource": STALE}, "source_changed",
                  f"“{MALIK}” katmanının kaynağı ilk sayfadan sonra değişti ({sha[:12]}…); sonraki sayfa aynı verinin "
                  "sayfası olmaz. Listeyi baştan okuyun.", "expectedSource", conflict)),
        case_("beklenen çizim sürümü değiştiyse okunmaz",
              step({"layer": MALIK, "expectedRevision": "1"},
                   {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra "
                    "değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                    "path": "expectedRevision", "revision": "$current"}})),
        case_("geçersiz sürüm yazısı", bad({"layer": MALIK, "expectedRevision": "12.0"}, "invalid_revision",
                                          "Beklenen sürüm “12.0” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                                          "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")),
        case_("sayfa boyu 0 olamaz", bad({"layer": MALIK, "limit": 0}, "invalid_count",
                                        "Bir sayfada 1 ile 500 arasında kayıt okunur; 0 verildi.", "limit")),
        case_("sayfa boyu en çok 500", bad({"layer": MALIK, "limit": 501}, "invalid_count",
                                         "Bir sayfada 1 ile 500 arasında kayıt okunur; 501 verildi.", "limit")),
        case_("imleç bir kimlik olmalı", bad({"layer": MALIK, "cursor": "x"}, "invalid_cursor",
                                            "“x” bu listenin sayfa imleci değil; bir önceki cevabın next'ini verin.", "cursor")),
        case_("imlecin kimliği listede olmalı", bad({"layer": MALIK, "cursor": p1}, "invalid_cursor",
                                                   f"“{p1}” bu listede yok; bir önceki cevabın next'ini verin.", "cursor")),
        case_("katman ya da ilişki verilmeli", bad({}, "invalid_input",
                                                  "Katmanı (layer) ya da bir ilişkiyi (relation ve of) verin; ikisi birden "
                                                  "ya da hiçbiri olmaz.", "layer")),
        case_("katman ve ilişki birlikte olmaz", bad({"layer": MALIK, "relation": "parsel-malik", "of": o1}, "invalid_input",
                                                    "Katmanı (layer) ya da bir ilişkiyi (relation ve of) verin; ikisi birden "
                                                    "ya da hiçbiri olmaz.", "layer")),
        case_("of ilişkisiz verilmez", bad({"layer": MALIK, "of": o1}, "invalid_input",
                                          "of ve side ilişkiyle (relation) birlikte verilir.", "of")),
        case_("ilişkide of verilmeli", bad({"relation": "parsel-malik"}, "invalid_input",
                                          "İlişki okunurken of (kaydın kimliği) verilmeli.", "of")),
        case_("of bir kimlik olmalı", bad({"relation": "parsel-malik", "of": "x"}, "invalid_input",
                                         "“x” bir kalıcı kimlik değil (küçük harfli, tireli UUID).", "of")),
        case_("olmayan katman", bad({"layer": "yok"}, "layer_not_found",
                                   "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer")),
        case_("olmayan ilişki", bad({"relation": "yok", "of": o1}, "unknown_relation",
                                   "“yok” kimlikli bir ilişki yok; cad.schema.describe ilişkileri listeler.", "relation")),
        case_("çizimde olmayan kayıt", bad({"relation": "parsel-malik", "of": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0000"},
                                          "entity_not_found", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0000 kimlikli nesne çizimde yok.", "of")),
        case_("kaydın katmanı ilişkinin o tarafının katmanı olmalı",
              bad({"relation": "parsel-malik", "of": o1, "side": "parents"}, "form_layer_mismatch",
                  f"{o1} nesnesi “{MALIK}” katmanında; “Parselin malikleri” ilişkisinde alt tarafı “{PARSEL}” katmanıdır.", "of")),
        case_("olmayan alan döndürülmez", bad({"layer": MALIK, "fields": ["yok"]}, "field_not_found",
                                              "“yok” alanı “malik” katmanında yok.", "fields.0")),
        case_("plan okumaktır: çizim değişmez", step({"layer": MALIK}, done(paged(MALIK, owners)), op="plan")),
        case_("denetim yalnız denetler", step({"layer": MALIK}, done(None), op="validate")),
    ]
    return cases


# ── cad.sources.sync: layers written together ──────────────────────────────

def synced(source, added=0, modified=0, removed=0, layers=None, relations=None, columns=None):
    out = {"added": added, "modified": modified, "removed": removed}
    if columns:
        out["columns"] = columns
    out["source"] = source
    out["revision"] = "$current"
    if layers:
        out["layers"] = [{"layer": n, "added": a, "modified": m, "removed": r} for n, a, m, r in layers]
    if relations:
        out["relations"] = [{"relation": n, "added": a, "removed": r} for n, a, r in relations]
    return out


def write_in(layer, *with_, direction="write", **extra):
    i = {"layer": layer, "direction": direction}
    if with_:
        i["with"] = list(with_)
    i.update(extra)
    return i


def warning(message, code="relation_unmapped"):
    return {"code": code, "message": message, "path": "layer"}


TEXT_NOTE = ("“parsel-malik” ilişkisinin ‘malikler’ alanı parsel tablosunda “; ” ile birleşmiş metin sütunu olarak "
             "kaldı (cad.sources.relate map ile ilişki profiline taşınır).")


def foreign_note(why):
    return (f"“parsel-malik” ilişkisi: {why}; ‘malikler’ alanı yalnız metin sütunu olarak yazıldı, dosyadaki "
            "ilişkinin eşleme satırlarına dokunulmadı.")


def together_cases():
    t, m = "iliskili.gpkg", "iliskili-eslenmis.gpkg"
    o1, o2, o3 = (files.owner_uid(i) for i in (1, 2, 3))
    ghost = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"
    dangling = ("{uid} kayıtlı import-parsel katmanı nesnesinin ‘malikler’ alanı {id} kimliğini tutuyor; "
                "import-malik katmanında bu kimlikte nesne yok, eşleme satırı kurulamaz. Başvuruyu düzeltin.")
    empty_t, pre_t = preamble(t)
    empty_m, pre_m = preamble(m)
    plan = lambda inp, result, expect=None: step(inp, result, expect, op="plan")
    ring = [{"x": 487200.0, "y": 4420000.0}, {"x": 487210.0, "y": 4420000.0}, {"x": 487210.0, "y": 4420010.0}]
    new_owner = other("cad.records.create", {"layerId": MALIK, "rows": [{"attrs": {"ad": "Zeynep Kaya", "tur": "Gerçek"}}]})
    new_parcel = [other("cad.polygon.create", {"layerId": PARSEL, "pts": ring})]

    def with_new(owner_ref):
        return [new_owner, {"op": "captureUid", "id": 10, "as": "yeni"}, *new_parcel,
                {"op": "captureUid", "id": 11, "as": "yeniparsel"},
                other("cad.entities.set", {"uids": ["$uidOf:11"], "operation": "attributes",
                                           "attrs": {"malikler": owner_ref}})]

    sha_t, sha_m = files.sha(t), files.sha(m)
    cases = [
        case("metin sütunu: üst ve alt katman aynı işlemde; her katmanın payı, ilişki eşli değilse uyarı",
             *pre_t, set_attrs("malik", 1, {"tur": "Tüzel"}), set_attrs("parsel", 1, {"ada": "999"}),
             plan(write_in(PARSEL, MALIK),
                  done(synced(sha_t, modified=2, layers=[(PARSEL, 0, 1, 0), (MALIK, 0, 1, 0)]), [warning(TEXT_NOTE)])),
             setup_=empty_t),
        case("tek katman yazılırken de eşlenmemiş çoktan çoğa başvuru metin olarak kaldığı söylenir",
             *pre_t, plan(write_in(PARSEL), done(synced(sha_t), [warning(TEXT_NOTE)])), setup_=empty_t),
        case("eşli ilişki: yazılacak bir şey yok, eşleme tablosu da aynı",
             *pre_m, plan(write_in(PARSEL, MALIK), done(synced(sha_m, layers=[(PARSEL, 0, 0, 0), (MALIK, 0, 0, 0)],
                                                           relations=[("parsel-malik", 0, 0)]))),
             setup_=empty_m),
        case("eşli ilişki: parsele eklenen malik eşleme satırı olarak yazılır, sütun yazılmaz",
             *pre_m, set_attrs("parsel", 3, {"malikler": f"{o1}; {o2}"}),
             plan(write_in(PARSEL), done(synced(sha_m, relations=[("parsel-malik", 2, 0)]))), setup_=empty_m),
        case("eşli ilişki: parselden çıkarılan malikin satırı kalkar",
             *pre_m, set_attrs("parsel", 1, {"malikler": o1}),
             plan(write_in(PARSEL), done(synced(sha_m, relations=[("parsel-malik", 0, 1)]))), setup_=empty_m),
        case("eşli ilişki: boşaltılan parselin iki satırı da kalkar",
             *pre_m, set_attrs("parsel", 1, {"malikler": ""}),
             plan(write_in(PARSEL), done(synced(sha_m, relations=[("parsel-malik", 0, 2)]))), setup_=empty_m),
        case("eşli ilişki: silinen malik ve eşleme satırı birlikte gider, parselin satırı değişmez",
             *pre_m, other("cad.entities.delete", {"uids": [f"$uidOf:{slot('malik', 3)}"]}),
             plan(write_in(PARSEL, MALIK),
                  done(synced(sha_m, removed=1, layers=[(PARSEL, 0, 0, 0), (MALIK, 0, 0, 1)],
                              relations=[("parsel-malik", 0, 1)]))),
             setup_=empty_m),
        case("eşli ilişki: yeni malik ve onu tutan yeni parsel tek işlemde",
             *pre_m, *with_new("$uid:yeni"),
             plan(write_in(PARSEL, MALIK),
                  done(synced(sha_m, added=2, layers=[(PARSEL, 1, 0, 0), (MALIK, 1, 0, 0)],
                              relations=[("parsel-malik", 1, 0)]))),
             setup_=empty_m),
        case("eşli ilişki: yeni parsel var olan maliki tutar, parsel tek başına yazılır",
             *pre_m, *with_new(o1),
             plan(write_in(PARSEL), done(synced(sha_m, added=1, relations=[("parsel-malik", 1, 0)]))), setup_=empty_m),
        case("eşli ilişki: yalnız malikler yazılırken de ilişki sayılır",
             *pre_m, set_attrs("malik", 2, {"tur": "Tüzel"}),
             plan(write_in(MALIK), done(synced(sha_m, modified=1, relations=[("parsel-malik", 0, 0)]))), setup_=empty_m),
        case("eşli ilişki: tabloda olmayan yeni malike başvuran parsel tek başına yazılmaz",
             *pre_m, *with_new("$uid:yeni"),
             plan(write_in(PARSEL),
                  failed("relation_dangling", "$uid:yeniparsel kayıtlı import-parsel katmanı nesnesi $uid:yeni kimlikli "
                         "import-malik nesnesini tutuyor; o malik tablosunda yok. Üst katmanı da birlikte yazın (with).",
                         "layer")),
             setup_=empty_m),
        case("eşli ilişki: olmayan kimliğe giden başvuru yazmayı reddettirir",
             *pre_m, set_attrs("parsel", 3, {"malikler": ghost}),
             plan(write_in(PARSEL), failed("relation_dangling", dangling.format(uid=files.PARCELS[2][1], id=ghost), "layer")),
             setup_=empty_m),
        case("başka temsilli ilişki: parsel metin sütunu olarak yazılır, eşleme satırlarına dokunulmaz, uyarılır",
             *preamble("iliskili-yabanci.gpkg")[1], set_attrs("parsel", 1, {"ada": "999"}),
             plan(write_in(PARSEL), done(synced(files.sha("iliskili-yabanci.gpkg"), modified=1),
                                         [warning(foreign_note(FOREIGN_WHY))])),
             setup_=preamble("iliskili-yabanci.gpkg")[0]),
        case("bozuk ilişki de uyarılır, yazılır",
             *preamble("iliskili-bozuk.gpkg")[1], set_attrs("parsel", 1, {"ada": "999"}),
             plan(write_in(PARSEL), done(synced(files.sha("iliskili-bozuk.gpkg"), modified=1),
                                         [warning(foreign_note(BROKEN_WHY))])),
             setup_=preamble("iliskili-bozuk.gpkg")[0]),
        case("ters yönde yazılmış ilişki yabancıdır",
             *preamble("iliskili-ters.gpkg")[1], set_attrs("parsel", 1, {"ada": "999"}),
             plan(write_in(PARSEL), done(synced(files.sha("iliskili-ters.gpkg"), modified=1),
                                         [warning(foreign_note(REVERSE_WHY))])),
             setup_=preamble("iliskili-ters.gpkg")[0]),
        case("katmanlardan biri tabloya yazılamazsa hiçbiri yazılmaz",
             *pre_t, set_attrs("parsel", 1, {"ada": "999"}),
             other("cad.polygon.create", {"layerId": MALIK, "pts": ring}),
             plan(write_in(PARSEL, MALIK), failed("geometry_mismatch", ANY, "with")),
             setup_=empty_t),
        case("eşli ilişkinin katmanı okunurken alan eşleme satırlarından dolar (sütun yokken de)",
             *preamble("iliskili-eslenmis-sutunsuz.gpkg")[1],
             step({"layer": PARSEL, "direction": "read"}, done(synced(files.sha("iliskili-eslenmis-sutunsuz.gpkg"), modified=2)),
                  {"revision": "changed", "fields": {"5": {"kind": "polygon", "layerId": PARSEL,
                   "attrs": {"ada": "101", "parsel_no": "5", "malikler": f"{o1}; {o2}"}, "pts": ANY},
                   "6": {"kind": "polygon", "layerId": PARSEL,
                   "attrs": {"ada": "101", "parsel_no": "6", "malikler": f"{o2}; {o3}"}, "pts": ANY}}}),
             setup_=preamble("iliskili-eslenmis-sutunsuz.gpkg")[0]),
        case("katmanlar ayrı dosyalarda: birlikte yazılmaz",
             plan(write_in(PARSEL, MALIK),
                  failed("transaction_split", f"“{MALIK}” katmanı {fx('malikler.gpkg')} kaynağında, “{PARSEL}” katmanı "
                         f"{fx(t)} dosyasında; birlikte yazılacak katmanlar tek GeoPackage dosyasında olmalı. "
                         "Katmanları ayrı ayrı yazın.", "with")),
             setup_=static(t, malik_name="malikler.gpkg")),
        case("aynı katman iki kez verilemez",
             plan(write_in(PARSEL, PARSEL),
                  failed("invalid_input", "“import-parsel” katmanı birden çok kez verildi; her katman bir kez yazılır.",
                         "with")),
             setup_=static(t)),
        case("with'teki katman bağlı olmalı",
             plan(write_in(PARSEL, "cizim"),
                  failed("source_not_found", "“cizim” katmanı bir kaynağa bağlı değil; önce cad.sources.edit ile "
                         "bağlayın.", "with")),
             setup_=static(t)),
        case("with yalnız yazma içindir",
             plan(write_in(PARSEL, MALIK, direction="read"),
                  failed("invalid_input", "with yalnız yazma (direction write) içindir: okuma her katmanı ayrı alır.",
                         "with")),
             setup_=static(t)),
        case("with'teki katmanın tablosu salt okunursa hiçbiri yazılmaz",
             plan(write_in(PARSEL, MALIK),
                  failed("source_readonly", f"{fx(t)} dosyasının “malik” tablosuna yazılamaz (salt okunur).", "with")),
             setup_=static(t, readonly=(MALIK,))),
        case("dosya katmanlar onunla son eşleştiğinden beri değiştiyse birlikte yazılmaz",
             plan(write_in(PARSEL, MALIK),
                  conflict("source_changed", f"{fx(t)} katman onunla son eşleştiğinden beri değişti; üzerine yazılmadı. "
                           "Önce tablodan okuyun (direction read) ya da dosyanın kopyasını bağlayın.", "layer")),
             setup_=static(t, revision=STALE)),
        case("dosya yerinde yoksa",
             plan(write_in(PARSEL, MALIK),
                  failed("source_gone", f"{fx('yok.gpkg')} yerinde yok; dosyanın yeni yerini cad.sources.edit relink ile "
                         "verin.", "layer")),
             setup_=static(t, missing=True)),
        case("plan hiçbir şey yazmaz: katmanlar dosyayla eşleşmiş kalır, denetim yalnız denetler",
             *pre_t, set_attrs("parsel", 1, {"ada": "999"}),
             step(write_in(PARSEL, MALIK), done(None), op="validate"), setup_=empty_t),
    ]
    return cases


# ── cad.forms.submit: commit ───────────────────────────────────────────────

OWNER_FORM, PARCEL_FORM = "malik-formu", "parsel-formu"


def forms_pre(name, link_malik=None):
    """The linked world with a form on the owners' layer and one on the parcels'."""
    empty, pre = preamble(name)
    if link_malik:
        pre[0] = other("cad.sources.edit", {"edit": {"kind": "link", "path": fx(link_malik), "table": "malik"}})
    pre += [other("cad.forms.edit", {"edit": {"kind": "create", "id": OWNER_FORM, "name": "Malik formu", "layer": MALIK}}),
            other("cad.forms.edit", {"edit": {"kind": "create", "id": PARCEL_FORM, "name": "Parsel formu", "layer": PARSEL}})]
    return empty, pre


def child(relation, uid, unlink=False, **values):
    c = {"relation": relation, "uid": uid}
    if values:
        c["values"] = values
    if unlink:
        c["unlink"] = True
    return c


def submit_in(form, uid, children=(), commit=True, **values):
    i = {"form": form, "draft": True, "uid": uid}
    if values:
        i["values"] = values
    if children:
        i["children"] = list(children)
    if commit:
        i["commit"] = True
    return i


def submitted(uid, attrs, children=(), changes=(), committed=None, changed=False):
    out = {"uid": uid, "attrs": attrs, "changed": changed}
    if children:
        out["children"] = [{"uid": u, "relation": r, **({"created": True} if created else {}), "attrs": a, "changed": ch}
                           for u, r, created, a, ch in children]
    if changes:
        out["changes"] = [{"uid": u, "layer": layer, **({"created": True} if created else {}), "fields": f}
                          for u, layer, created, f in changes]
    out["snapshot"] = ANY
    if committed:
        out["committed"] = committed
    out["revision"] = "$current"
    return out


def commit_cases():
    t, m = "iliskili.gpkg", "iliskili-eslenmis.gpkg"
    o1, o2, o3 = (files.owner_uid(i) for i in (1, 2, 3))
    p1, p2, p3 = (files.parcel_uid(i) for i in (1, 2, 3))
    b1, b2 = (b[1] for b in files.BUILDINGS)
    owner1 = {"ad": "Ayşe Yıldız", "tur": "Gerçek"}
    parcel3 = {"ada": "102", "parsel_no": "1"}
    ghost = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"
    new_building = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1003"
    sha_t, sha_m = files.sha(t), files.sha(m)
    empty_m, pre_m = forms_pre(m)
    empty_t, pre_t = forms_pre(t)
    plan = lambda inp, result: step(inp, result, op="plan")
    pm = lambda *steps, name: case(name, *pre_m, *steps, setup_=empty_m)
    pt = lambda *steps, name: case(name, *pre_t, *steps, setup_=empty_t)
    link_p3 = child("parsel-malik", p3)
    link_layers = lambda sha, a, b, rel, **kw: synced(sha, layers=[(MALIK, 0, 0, 0), (PARSEL, 0, a, 0)][:2], relations=rel,
                                                      **kw)
    cases = [
        pm(plan(submit_in(OWNER_FORM, o1, [link_p3]),
                done(submitted(o1, owner1, [(p3, "parsel-malik", False, {**parcel3, "malikler": o1}, True)],
                               [(p3, PARSEL, False, ["malikler"])],
                               synced(sha_m, layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 0, 0)], relations=[("parsel-malik", 1, 0)])))),
           name="eşli ilişki: malike bağlanan parsel kayıtla birlikte dosyaya, eşleme satırı olarak, tek işlemde"),
        pm(plan(submit_in(OWNER_FORM, o1, [link_p3], commit=False),
                done(submitted(o1, owner1, [(p3, "parsel-malik", False, {**parcel3, "malikler": o1}, True)],
                               [(p3, PARSEL, False, ["malikler"])]))),
           name="commit yoksa dosyaya bir şey yazılmaz, cevapta committed da yok"),
        pm(plan(submit_in(OWNER_FORM, o2, [child("parsel-malik", p1, unlink=True)]),
                done(submitted(o2, {"ad": "Mehmet Demir", "tur": "Gerçek"},
                               [(p1, "parsel-malik", False, {"ada": "101", "parsel_no": "5", "malikler": o1}, True)],
                               [(p1, PARSEL, False, ["malikler"])],
                               synced(sha_m, layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 0, 0)], relations=[("parsel-malik", 0, 1)])))),
           name="eşli ilişki: bağı koparılan parselin eşleme satırı kalkar"),
        pm(plan(submit_in(OWNER_FORM, o1, [link_p3], tur="Tüzel"),
                done(submitted(o1, {"ad": "Ayşe Yıldız", "tur": "Tüzel"},
                               [(p3, "parsel-malik", False, {**parcel3, "malikler": o1}, True)],
                               [(o1, MALIK, False, ["tur"]), (p3, PARSEL, False, ["malikler"])],
                               synced(sha_m, modified=1, layers=[(MALIK, 0, 1, 0), (PARSEL, 0, 0, 0)],
                                      relations=[("parsel-malik", 1, 0)]), changed=True))),
           name="kaydın kendi değeri ve alt kayıt bağı aynı işlemde: malik satırı güncellenir, eşleme satırı eklenir"),
        pm(plan(submit_in(OWNER_FORM, o3, [child("parsel-malik", p1), child("parsel-malik", p3)]),
                done(submitted(o3, {"ad": "Sivas Belediyesi", "tur": "Tüzel"},
                               [(p1, "parsel-malik", False, {"ada": "101", "parsel_no": "5", "malikler": f"{o1}; {o2}; {o3}"}, True),
                                (p3, "parsel-malik", False, {**parcel3, "malikler": o3}, True)],
                               [(p1, PARSEL, False, ["malikler"]), (p3, PARSEL, False, ["malikler"])],
                               synced(sha_m, layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 0, 0)], relations=[("parsel-malik", 2, 0)])))),
           name="iki parsel bir malike bağlanır: iki eşleme satırı"),
        pm(plan(submit_in(OWNER_FORM, o2, [child("parsel-malik", p1, unlink=True), child("parsel-malik", p2, unlink=True)]),
                done(submitted(o2, {"ad": "Mehmet Demir", "tur": "Gerçek"},
                               [(p1, "parsel-malik", False, {"ada": "101", "parsel_no": "5", "malikler": o1}, True),
                                (p2, "parsel-malik", False, {"ada": "101", "parsel_no": "6", "malikler": o3}, True)],
                               [(p1, PARSEL, False, ["malikler"]), (p2, PARSEL, False, ["malikler"])],
                               synced(sha_m, layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 0, 0)], relations=[("parsel-malik", 0, 2)])))),
           name="bir malikin iki parselle bağı birlikte koparılır: iki eşleme satırı kalkar"),
        pm(plan(submit_in(PARCEL_FORM, p3, [child("bina-parsel", b1)]),
                done(submitted(p3, parcel3, [(b1, "bina-parsel", False, {"parsel": p3, "kat": "2"}, True)],
                               [(b1, BINA, False, ["parsel"])],
                               synced(sha_m, modified=1, layers=[(PARSEL, 0, 0, 0), (BINA, 0, 1, 0)],
                                      relations=[("parsel-malik", 0, 0)])))),
           name="parsele bağlanan bina: bina satırı güncellenir, parselin eşleme tablosu aynı kalır"),
        pm(plan(submit_in(PARCEL_FORM, p1, [child("bina-parsel", b1, unlink=True)]),
                done(submitted(p1, {"ada": "101", "parsel_no": "5", "malikler": f"{o1}; {o2}"},
                               [(b1, "bina-parsel", False, {"kat": "2"}, True)], [(b1, BINA, False, ["parsel"])],
                               synced(sha_m, modified=1, layers=[(PARSEL, 0, 0, 0), (BINA, 0, 1, 0)],
                                      relations=[("parsel-malik", 0, 0)])))),
           name="parselden kopan bina: tek kimlikli alan boşalır, satır güncellenir"),
        pm(plan(submit_in(PARCEL_FORM, p3, [child("bina-parsel", new_building, kat="3")]),
                done(submitted(p3, parcel3, [(new_building, "bina-parsel", True, {"kat": "3", "parsel": p3}, True)],
                               [(new_building, BINA, True, ["kat", "parsel"])],
                               synced(sha_m, added=1, layers=[(PARSEL, 0, 0, 0), (BINA, 1, 0, 0)],
                                      relations=[("parsel-malik", 0, 0)])))),
           name="parsele eklenen yeni bina kaydı: bina tablosuna yeni satır olarak, tek işlemde"),
        pm(plan(submit_in(OWNER_FORM, o1),
                done(submitted(o1, owner1, committed=synced(sha_m, relations=[("parsel-malik", 0, 0)])))),
           name="değişiklik yoksa dosyaya yazılacak bir şey de yoktur; ilişki yine sayılır"),
        pt(plan(submit_in(OWNER_FORM, o1, [link_p3]),
                done(submitted(o1, owner1, [(p3, "parsel-malik", False, {**parcel3, "malikler": o1}, True)],
                               [(p3, PARSEL, False, ["malikler"])],
                               synced(sha_t, modified=1, layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 1, 0)])),
                     [warning(TEXT_NOTE)])),
           name="metin sütunu: bağ parselin sütununa yazılır, eşlenmediği söylenir"),
        pt(plan(submit_in(OWNER_FORM, o2, [child("parsel-malik", p1, unlink=True)]),
                done(submitted(o2, {"ad": "Mehmet Demir", "tur": "Gerçek"},
                               [(p1, "parsel-malik", False, {"ada": "101", "parsel_no": "5", "malikler": o1}, True)],
                               [(p1, PARSEL, False, ["malikler"])],
                               synced(sha_t, modified=1, layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 1, 0)])),
                     [warning(TEXT_NOTE)])),
           name="metin sütunu: bağı koparılan parselin sütunu güncellenir"),
        pt(plan(submit_in(OWNER_FORM, o1, tur="Tüzel"),
                done(submitted(o1, {"ad": "Ayşe Yıldız", "tur": "Tüzel"}, changes=[(o1, MALIK, False, ["tur"])],
                               committed=synced(sha_t, modified=1), changed=True),
                     [warning(TEXT_NOTE)])),
           name="yalnız kaydın değeri: tek katman, katmanların dökümü yok"),
        case("yabancı temsilli ilişki: bağ metin sütununa yazılır, eşleme satırlarına dokunulmaz, uyarılır",
             *forms_pre("iliskili-yabanci.gpkg")[1],
             plan(submit_in(OWNER_FORM, o1, [link_p3]),
                  done(submitted(o1, owner1, [(p3, "parsel-malik", False, {**parcel3, "malikler": o1}, True)],
                                 [(p3, PARSEL, False, ["malikler"])],
                                 synced(files.sha("iliskili-yabanci.gpkg"), modified=1,
                                        layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 1, 0)])),
                       [warning(foreign_note(FOREIGN_WHY))])),
             setup_=forms_pre("iliskili-yabanci.gpkg")[0]),
        case("eşli, sütunsuz tablo: alan sütun olarak eklenmez, yalnız eşleme satırı yazılır",
             *forms_pre("iliskili-eslenmis-sutunsuz.gpkg")[1],
             step({"layer": PARSEL, "direction": "read"}, {"status": "completed", "output": ANY, "warnings": ANY},
                  {"revision": "changed"}, command="cad.sources.sync"),
             plan(submit_in(OWNER_FORM, o1, [link_p3]),
                  done(submitted(o1, owner1, [(p3, "parsel-malik", False, {**parcel3, "malikler": o1}, True)],
                                 [(p3, PARSEL, False, ["malikler"])],
                                 synced(files.sha("iliskili-eslenmis-sutunsuz.gpkg"),
                                        layers=[(MALIK, 0, 0, 0), (PARSEL, 0, 0, 0)], relations=[("parsel-malik", 1, 0)])))),
             setup_=forms_pre("iliskili-eslenmis-sutunsuz.gpkg")[0]),
        pm(other("cad.entities.set", {"uids": [f"$uidOf:{slot('parsel', 2)}"], "operation": "attributes",
                                      "attrs": {"malikler": ghost}}),
           plan(submit_in(OWNER_FORM, o1, [link_p3]),
                failed("relation_dangling", f"{p2} kayıtlı import-parsel katmanı nesnesinin ‘malikler’ alanı {ghost} "
                       "kimliğini tutuyor; import-malik katmanında bu kimlikte nesne yok, eşleme satırı kurulamaz. "
                       "Başvuruyu düzeltin.", "layer")),
           name="dosyadaki başka bir parselin olmayan kimliğe başvurusu gönderimi reddettirir: ne çizime ne dosyaya yazılır"),
        case("katmanlar ayrı dosyalarda: kayıt dosyaya birlikte yazılamaz",
             *forms_pre(t, link_malik="malikler.gpkg")[1],
             plan(submit_in(OWNER_FORM, o1, [link_p3]),
                  failed("transaction_split", f"“{PARSEL}” katmanı {fx(t)} kaynağında, “{MALIK}” katmanı "
                         f"{fx('malikler.gpkg')} dosyasında; birlikte yazılacak katmanlar tek GeoPackage dosyasında "
                         "olmalı. Katmanları ayrı ayrı yazın.", "with")),
             setup_=forms_pre(t, link_malik="malikler.gpkg")[0]),
        case("katmanlardan hiçbiri bir kaynağa bağlı değilse commit reddedilir",
             other("cad.forms.edit", {"edit": {"kind": "create", "id": "cizim-formu", "name": "Çizim formu", "layer": "cizim"}}),
             plan({"form": "cizim-formu", "draft": True, "uid": "$uidOf:1", "commit": True},
                  failed("source_not_found", "Kaynağa yazma istendi ama kaydın ve alt kayıtlarının katmanlarından hiçbiri "
                         "bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın.", "commit")),
             setup_=static(sources="none")),
        pm(step(submit_in(OWNER_FORM, o1, [link_p3]), done(None), op="validate"),
           name="denetim: commit'in dosyası da denetlenir"),
        case("denetim: commit'in reddi gönderimin reddidir",
             *forms_pre(t, link_malik="malikler.gpkg")[1],
             step(submit_in(OWNER_FORM, o1, [link_p3]),
                  failed("transaction_split", f"“{PARSEL}” katmanı {fx(t)} kaynağında, “{MALIK}” katmanı "
                         f"{fx('malikler.gpkg')} dosyasında; birlikte yazılacak katmanlar tek GeoPackage dosyasında "
                         "olmalı. Katmanları ayrı ayrı yazın.", "with"), op="validate"),
             setup_=forms_pre(t, link_malik="malikler.gpkg")[0]),
    ]
    return cases


def document(command, title, note, cases, version=1, setup_=None):
    d = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": version, "title": title,
         "note": note}
    if setup_ is not None:
        d["setup"] = setup_
    d["cases"] = cases
    return d


def build():
    out = {}
    out["cad.sources.relate.json"] = document(
        "cad.sources.relate", "İlişkiyi dosyaya eşle",
        "Çoktan çoğa başvurunun bağlı GeoPackage'da nasıl tutulduğu (docs/adr/0532 §2). Dosyalar "
        "scripts/fixtures/source_relations_files.py'nin; beklentiler dosyanın kendi tablolarından, KentOS kodu olmadan.",
        relate_cases(), setup_=static("iliskili.gpkg"))
    out["cad.records.page.json"] = document(
        "cad.records.page", "Kayıtları sayfa sayfa oku",
        "Bir katmanın ya da bir ilişkinin kayıtları sayfa sayfa, kaynağın sürümüyle (docs/adr/0532 §3). Beklentiler "
        "dosyaların satırlarından ve ADR'nin sırasından, KentOS kodu olmadan.",
        page_cases(), setup_=preamble()[0])
    out["cad.sources.sync.together.json"] = document(
        "cad.sources.sync", "Kaynakla eşitle: katmanlar tek işlemde, eşli çoktan çoğa ilişki",
        "with'le birlikte yazılan katmanlar ve cad.sources.relate'le eşlenen ilişkinin eşleme satırları (docs/adr/0532 §1, "
        "§2). Dosyaya yazan execute ortak durumlarda yok (fixture dosyası değişirdi); onu geçici kopyada "
        "tests/all/source_relations.rs sınar ve yazdığı dosyayı --verify okur.",
        together_cases(), setup_=preamble()[0])
    out["cad.forms.submit.commit.json"] = document(
        "cad.forms.submit", "Formla kaydet: kaydın ve alt kayıtlarının kaynağa tek işlemde yazılması",
        "commit'li gönderim (docs/adr/0532 §1): kayıt ve alt kayıtları, bağlı katmanları tek GeoPackage dosyasındaysa, "
        "aynı adımın ardından dosyaya tek SQLite işleminde yazılır; dosyanın reddi gönderimin reddidir. Dosyaya yazan "
        "execute ortak durumlarda yok (fixture dosyası değişirdi); onu tests/all/source_relations.rs geçici kopyada sınar.",
        commit_cases(), setup_=preamble()[0])
    return out


# ── the files KentOS wrote ─────────────────────────────────────────────────

def verify():
    """Reads the files `KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all source_relations` keeps
    with sqlite3 and GDAL's OGR (its relationship API and validate_gpkg.py): the relation as a reader sees it, the
    mapping's rows, the tables' rows and the lasting ids, the columns a mapped reference left alone."""
    import sqlite3
    from osgeo import gdal, ogr
    from osgeo_utils.samples import validate_gpkg
    gdal.UseExceptions()
    problems = []

    def check(ok, what):
        if not ok:
            problems.append(what)

    def rows(path, sql):
        c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        return c.execute(sql).fetchall()

    def relations(path):
        ds = gdal.OpenEx(str(path), gdal.OF_VECTOR)
        out = {}
        for name in ds.GetRelationshipNames() or []:
            r = ds.GetRelationship(name)
            out[name] = (r.GetLeftTableName(), r.GetRightTableName(), r.GetMappingTableName(),
                         r.GetCardinality() == gdal.GRC_MANY_TO_MANY, list(r.GetLeftMappingTableFields()),
                         list(r.GetRightMappingTableFields()))
        return out

    D = files.DIR
    # 1. A submission with commit: the owner's row and the parcel's link in one transaction.
    f = D / "kentos-iliskili-form.gpkg"
    check(not validate_gpkg.check(str(f), abort_at_first_error=False), f"{f.name}: GDAL's validate_gpkg found errors")
    check(relations(f) == {"kentos_references": ("parsel", "malik", "parsel_malik", True, ["base_id"], ["related_id"])},
          f"{f.name}: OGR does not read the relation as a many-to-many parsel → malik")
    check(sorted(rows(f, "SELECT base_id, related_id FROM parsel_malik")) == sorted(files.PAIRS + [(3, 1)]),
          f"{f.name}: the mapping rows are not the parcels' owners and the new link")
    check(rows(f, "SELECT tur FROM malik WHERE fid = 1") == [("Tüzel",)], f"{f.name}: the owner's row was not written")
    check(rows(f, "SELECT malikler FROM parsel WHERE fid = 3") == [(None,)],
          f"{f.name}: a mapped reference's column was written (it must keep what it held)")
    # 2. Layers written together: an owner, a parcel and the mapping row, all or nothing.
    f = D / "kentos-iliskili-birlikte.gpkg"
    check(not validate_gpkg.check(str(f), abort_at_first_error=False), f"{f.name}: GDAL's validate_gpkg found errors")
    m = rows(f, "SELECT base_id, related_id FROM parsel_malik")
    check(len(m) == 5 and (4, 4) in m, f"{f.name}: the new parcel's owner is not in the mapping")
    owners = {r[0] for r in rows(f, "SELECT fid FROM malik")}
    parcels = {r[0] for r in rows(f, "SELECT fid FROM parsel")}
    check(owners == {1, 2, 3, 4} and parcels == {1, 2, 3, 4}, f"{f.name}: the tables' rows")
    check(all(b in parcels and r in owners for b, r in m), f"{f.name}: a mapping row names a row that is not there")
    uids = dict(rows(f, "SELECT table_name || ':' || fid, uid FROM kentos_uids"))
    check("malik:4" in uids and "parsel:4" in uids, f"{f.name}: the new rows have no lasting ids")
    # 3. A mapping taken out: the relation, its table and its extension rows are gone, the references' column untouched.
    f = D / "kentos-iliskili-eslenmis.gpkg"
    check(not validate_gpkg.check(str(f), abort_at_first_error=False), f"{f.name}: GDAL's validate_gpkg found errors")
    check(relations(f) == {}, f"{f.name}: a relation is still declared")
    check(not rows(f, "SELECT name FROM sqlite_master WHERE name = 'parsel_malik'"), f"{f.name}: the mapping table is still there")
    check(not rows(f, "SELECT * FROM gpkg_extensions WHERE table_name = 'parsel_malik'"),
          f"{f.name}: the mapping table's extension row is still there")
    check(rows(f, "SELECT malikler FROM parsel WHERE fid = 3") == [(None,)],
          f"{f.name}: a mapped reference's column was written while it was mapped")
    for p in problems:
        print("FAIL", p)
    if problems:
        sys.exit(1)
    print("verified: 3 files")


def main():
    if "--verify" in sys.argv:
        verify()
        return
    check = "--check" in sys.argv
    bad = []
    for name, doc in build().items():
        text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text() != text:
                bad.append(name)
        else:
            path.write_text(text)
            print(f"{name}: {len(doc['cases'])} cases")
    if bad:
        print("differs:", ", ".join(bad))
        sys.exit(1)


if __name__ == "__main__":
    main()
