#!/usr/bin/env python3
"""Writes fixtures/commands/v1/cad.subnetworks.find.json and cad.subnetworks.update.json: the shared cases of the
subnetwork commands (docs/adr/0557 §2), their expectations built here from the ADR's rules, without KentOS code.

The reference reads the very document each case sets up (its layers, objects and network definition) and works out
the answer the ADR describes: the controllers (the network's source junctions) and their names (the name field, else
the label, else the `Ad` attribute, else `Kaynak <order>`), the controllers of one name (Turkish case aside) as one
subnetwork, what each reaches (a search over (node, terminal) pairs, shared with the traces' reference in
network_terminals_cases.py: terminals' paths, closed edges and junctions, other controllers' nodes), what no controller
reaches, what two reach, the attribute's stale, missing and left-over values, and for the update each object's new
attribute and the layers it leaves alone. The desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/subnetworks_command_cases.py           # writes the files
    python3 scripts/fixtures/subnetworks_command_cases.py --check   # compares them with the ones on disk
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
_spec = importlib.util.spec_from_file_location("network_terminals_cases", ROOT / "scripts/fixtures/network_terminals_cases.py")
terminals_reference = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(terminals_reference)

OX, OY = 500000.0, 4400000.0
ATTRIBUTE = "Alt ağ"
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
LISTED = 1000


# ── The document a case sets up ────────────────────────────────────────────────────────────────────────────────


def node(id_, name, locked=False, fields=None):
    n = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": dict(STYLE), "children": []}
    if fields:
        n["fields"] = fields
    return n


def line(id_, layer, a, b, attrs=None):
    return {"kind": "line", "id": id_, "layerId": layer, "attrs": dict(attrs or {}), "a": {"x": OX + a[0], "y": OY + a[1]}, "b": {"x": OX + b[0], "y": OY + b[1]}}


def point(id_, layer, p, attrs=None, label=None):
    e = {"kind": "point", "id": id_, "layerId": layer, "attrs": dict(attrs or {}), "p": {"x": OX + p[0], "y": OY + p[1]}}
    if label is not None:
        e["label"] = label
    return e


def netdef(reach="downstream", name="Ad", attribute=ATTRIBUTE, device_terminals=None, paths=None, subnetworks=True, terminals=True):
    d = {
        "id": "ag-1",
        "name": "Elektrik",
        "kind": "utility",
        "edges": [{"layer": "boru"}, {"layer": "hat2"}, {"layer": "kilitli"}],
        "junctions": [
            {"layer": "kaynak", "role": "source"},
            {"layer": "vana", "role": "valve", "closed": "durum = 'kapalı'"},
            {"layer": "trafo", "role": "junction"},
        ],
        "connect": "ends",
        "tolerance": 0.01,
        "direction": {"kind": "digitized"},
    }
    if terminals:
        d["terminals"] = {"start": "uc_bas", "end": "uc_son", "devices": [{
            "layer": "trafo",
            "terminals": device_terminals or [{"id": "yg", "name": "Yüksek gerilim"}, {"id": "ag", "name": "Alçak gerilim"}],
            "paths": paths or [["yg", "ag"]],
        }]}
    if subnetworks:
        s = {"attribute": attribute, "reach": reach}
        if name is not None:
            s["name"] = name
        d["subnetworks"] = s
    return d


def scene(reach="downstream", name="Ad", attribute=ATTRIBUTE, s1=None, s2=None, valve="açık", preset=None, hat2_fields=None,
          locked=False, sources=True, subnetworks=True, extra=(), edge12=("yg", None), device_terminals=None, paths=None, terminals=True,
          net_override=None):
    """The default drawing: two sources (Merkez A at 0,0 and Merkez B at 500,0), a transformer at 100,0 (terminals yg
    and ag, one path), pipes 3 (0→100, ends at yg), 4 (100→200, starts at ag), 5 (200→300), 6 (400→300), 7 (500→400), an
    unfed pipe 8, a pipe 11 on a typed layer without the attribute's field, a pipe 12 on a locked layer from the
    transformer's yg side, an open valve at 300,0 both sources reach."""
    ents = [
        point(1, "kaynak", (0, 0), {"Ad": "Merkez A"} if s1 is None else s1),
        point(2, "kaynak", (500, 0), {"Ad": "Merkez B"} if s2 is None else s2),
        line(3, "boru", (0, 0), (100, 0), {"uc_son": "yg"}),
        line(4, "boru", (100, 0), (200, 0), {"uc_bas": "ag"}),
        line(5, "boru", (200, 0), (300, 0)),
        line(6, "boru", (400, 0), (300, 0)),
        line(7, "boru", (500, 0), (400, 0)),
        line(8, "boru", (700, 0), (800, 0)),
        point(9, "trafo", (100, 0), {"Ad": "T1"}),
        point(10, "vana", (300, 0), {"durum": valve}),
        line(11, "hat2", (200, 0), (200, 90)),
        line(12, "kilitli", (100, 0), (100, 60), {k: v for k, v in zip(("uc_bas", "uc_son"), edge12) if v is not None}),
    ]
    if not sources:
        ents = [e for e in ents if e["layerId"] != "kaynak"]
    ents += list(extra)
    for e in ents:
        if preset and e["id"] in preset:
            e["attrs"][attribute] = preset[e["id"]]
    network = netdef(reach, name, attribute, device_terminals, paths, subnetworks, terminals)
    if net_override:
        network.update(net_override)
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "Elektrik şebekesi",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow", "networks": [network]},
        "origin": {"x": OX, "y": OY},
        "layers": [node("boru", "Boru"), node("hat2", "Hat 2", fields=hat2_fields or [{"name": "Ad", "kind": "text"}]), node("kilitli", "Kilitli", locked=True),
                   node("kaynak", "Kaynak"), node("vana", "Vana"), node("trafo", "Trafo"), node("0", "0")],
        "activeLayer": "0",
        "entities": ents,
        "styles": {"items": [], "categories": []},
    }


# ── The reference ───────────────────────────────────────────────────────────────────────────────────────────────


def folded(text):
    return terminals_reference.folded(text)


def read(setup, network_id="ag-1"):
    """The network as the ADR builds it from the document: its objects in the definition's order, the names, the
    subnetworks and what the attribute holds."""
    net = next((n for n in setup["settings"]["networks"] if n["id"] == network_id), None)
    ents = setup["entities"]
    by_layer = lambda layer: [e for e in ents if e["layerId"] == layer]  # noqa: E731
    terminals = net.get("terminals")
    devices = terminals["devices"] if terminals else []
    edge_list, junction_list = [], []
    for layer in net["edges"]:
        for e in by_layer(layer["layer"]):
            if e["kind"] != "line":
                continue
            pts = [[e["a"]["x"], e["a"]["y"]], [e["b"]["x"], e["b"]["y"]]]
            ends = [e["attrs"].get(terminals["start"]) if terminals else None, e["attrs"].get(terminals["end"]) if terminals else None]
            shut = net.get("closed") is not None and e["attrs"].get("durum") == "kapalı"
            edge_list.append({"id": e["id"], "pts": pts, "terminals": ends, "layer": e["layerId"], **({"closed": True} if shut else {})})
    for layer in net["junctions"]:
        for e in by_layer(layer["layer"]):
            if e["kind"] != "point":
                continue
            closed = layer.get("closed") is not None and e["attrs"].get("durum") == "kapalı"
            device = next((i for i, d in enumerate(devices) if d["layer"] == layer["layer"]), None)
            junction_list.append({"id": e["id"], "p": [e["p"]["x"], e["p"]["y"]], "role": layer["role"], "closed": closed, "device": device, "layer": e["layerId"]})
    rules = {"connect": "ends", "tolerance": 0.01, "direction": net["direction"]}
    if terminals:
        rules["terminals"] = terminals
    sc = terminals_reference.Scene({"rules": rules, "edges": edge_list, "junctions": [{k: v for k, v in j.items() if k != "layer"} for j in junction_list]})
    return net, ents, edge_list, junction_list, sc


def uid(id_):
    return f"$uidOf:{id_}"


def naming(net, ents, junction_list):
    """The controllers (source junctions) in the graph's order, their names and the groups of one name."""
    subs = net["subnetworks"]
    controllers = [k for k, j in enumerate(junction_list) if j["role"] == "source"]
    entity = {e["id"]: e for e in ents}
    names, keys, groups = [], [], []
    for order, k in enumerate(controllers, 1):
        e = entity[junction_list[k]["id"]]
        given = lambda v: v.strip() if v is not None and v.strip() else None  # noqa: E731
        name = given(e["attrs"].get(subs["name"])) if subs.get("name") else None
        name = name or given(e.get("label")) or given(e["attrs"].get("Ad")) or f"Kaynak {order}"
        key = folded(name)
        if key in keys:
            groups[keys.index(key)].append(k)
        else:
            keys.append(key)
            names.append(name)
            groups.append([k])
    return controllers, names, groups


def objects(count_list):
    out = {"count": len(count_list)}
    if count_list:
        out["uids"] = [uid(i) for i in count_list[:LISTED]]
    return out


def find_answer(setup, members=False, network_id="ag-1"):
    """The answer of `cad.subnetworks.find`, and what the update needs: the objects wrong in the attribute."""
    net, ents, edge_list, junction_list, sc = read(setup, network_id)
    subs = net["subnetworks"]
    controllers, names, groups = naming(net, ents, junction_list)
    reach = subs["reach"]
    result = terminals_reference.subnet_case(sc, {"groups": groups, "reach": reach})["reached"]
    entity = {e["id"]: e for e in ents}
    universe = [e["id"] for e in edge_list]
    for j in junction_list:
        if j["id"] not in universe:
            universe.append(j["id"])
    attribute = subs["attribute"]
    unterminated = {u[0] for u in sc.unterminated}
    off_network = {j["id"] for j in junction_list if not any(tuple(j["p"]) in (e["a"], e["b"]) for e in sc.edges)}
    member_of = {}
    listed = []
    for k, (group, r) in enumerate(zip(groups, result)):
        edges = [edge_list[i]["id"] for i in r["edges"]]
        edges = [e for n, e in enumerate(edges) if e not in edges[:n]]
        junctions = [junction_list[i]["id"] for i in r["junctions"]]
        junctions = [j for n, j in enumerate(junctions) if j not in junctions[:n]]
        for i in edges + junctions:
            member_of.setdefault(i, [])
            if k not in member_of[i]:
                member_of[i].append(k)
        controllers_uids = []
        for c in group:
            u = uid(junction_list[c]["id"])
            if u not in controllers_uids:
                controllers_uids.append(u)
        reached = set(edges) | set(junctions)
        unverified = len(unterminated & set(edges)) + len(off_network & set(junctions))
        item = {"name": names[k], "controllers": controllers_uids, "edges": len(edges), "junctions": len(junctions), "length": r["length"], "unverified": unverified}
        if members:
            item["members"] = [uid(i) for i in (edges + junctions)[:LISTED]]
        listed.append(item)
        del reached
    orphans, shared, changed, missing, left_over, wrong = [], [], [], [], [], []
    for i in universe:
        want = "; ".join(names[k] for k in member_of[i]) if i in member_of else None
        now = (entity[i]["attrs"].get(attribute) or "").strip() or None
        if i not in member_of:
            orphans.append(i)
        elif len(member_of[i]) > 1:
            shared.append(i)
        if want is not None and now is not None and want != now:
            changed.append(i)
        elif want is not None and now is None:
            missing.append(i)
        elif want is None and now is not None:
            left_over.append(i)
        else:
            continue
        wrong.append((i, want))
    controller_ids = {junction_list[k]["id"] for k in controllers}
    out = {
        "network": net["id"],
        "attribute": attribute,
        "reach": reach,
        "controllers": len(controller_ids),
        "subnetworks": listed,
        "orphans": objects(orphans),
        "shared": objects(shared),
        "stale": {"changed": objects(changed), "missing": objects(missing), "leftOver": objects(left_over)},
        "revision": "$current",
    }
    return out, wrong, entity


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_=None, result=None, expect=None):
    s = {"op": op}
    if input_ is not None:
        s["input"] = input_
    if result is not None:
        s["result"] = result
    if expect is not None:
        s["expect"] = expect
    return s


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def case(name, setup, steps):
    return {"name": name, "setup": setup, "steps": steps}


# ── cad.subnetworks.find ────────────────────────────────────────────────────────────────────────────────────────


def find_cases():
    out = []

    def ok(name, setup, members=False, op="execute"):
        answer, _, _ = find_answer(setup, members)
        input_ = {"network": "ag-1", **({"members": True} if members else {})}
        out.append(case(name, setup, [step(op, input_, completed(answer), UNCHANGED)]))

    def refused(name, setup, code, message, network="ag-1"):
        out.append(case(name, setup, [step("execute", {"network": network}, failed(code, message, "network"), UNCHANGED)]))

    ok("iki kaynak: Merkez A ve Merkez B; ortak vana, kaynaksız boru, trafo terminalleri; hiçbir özniteliği yok", scene())
    ok("nesnelerin kimlikleriyle (members)", scene(), members=True)
    base = scene()
    answer, _, _ = find_answer(base)
    out.append(case("plan aynı cevabı verir; hiçbir şey yazılmaz", base, [step("plan", {"network": "ag-1"}, completed(answer), UNCHANGED)]))
    out.append(case("doğrula: hiçbir şey yazılmaz", base, [step("validate", {"network": "ag-1"}, {"status": "completed", "output": None, "warnings": []}, UNCHANGED)]))
    ok("Bağlı: her yönden; kaynaklar birbirinin düğümünde durur", scene(reach="connected"))
    ok("kapalı vana yolu keser: vana iki yanda ulaşılmış sayılır, ötesine geçilmez", scene(valve="kapalı"))
    closed_mid = scene()
    closed_mid["entities"][9] = point(10, "vana", (200, 0), {"durum": "kapalı"})
    ok("kapalı vana (200,0)'da: Merkez A yalnız ilk iki boruya kadar", closed_mid)
    ok("aynı adlı iki denetleyici (büyük küçük harf ayrımsız) tek alt ağ", scene(s1={"Ad": "Merkez"}, s2={"Ad": "MERKEZ"}))
    ok("Türkçe I: IŞIK ile ışık aynı ad", scene(s1={"Ad": "IŞIK"}, s2={"Ad": "ışık"}))
    ok("Türkçe I: ILIK ile ilik ayrı", scene(s1={"Ad": "ILIK"}, s2={"Ad": "ilik"}))
    labelled = scene(name=None, s1={}, s2={"Ad": "Öbür"})
    labelled["entities"][0]["label"] = "Etiketli kaynak"
    ok("ad alanı yoksa etiket, yoksa Ad özniteliği", labelled)
    ok("adı olmayan denetleyici: Kaynak <sıra>", scene(s1={}, s2={}, name="Ad"))
    ok("ad alanının değeri boşsa Ad özniteliğine düşer", scene(s1={"Ad": "A1", "Kod": "  "}, s2={"Ad": "B", "Kod": "K2"}, name="Kod"))
    ok("ad alanı yalnız birinde: öbürü Ad'a düşer", scene(name="Takma", s1={"Ad": "Merkez A"}, s2={"Takma": "Takma B"}))
    shut_edge = scene(net_override={"closed": "durum = 'kapalı'"})
    shut_edge["entities"][4]["attrs"]["durum"] = "kapalı"
    ok("kapalı kenar (5) izlenmez: Merkez A onun öncesinde biter, kenar kaynaksız kalır", shut_edge)
    updated = scene(preset={3: "Merkez A", 4: "Merkez A", 5: "Merkez A", 1: "Merkez A", 9: "Merkez A", 6: "Merkez B", 7: "Merkez B", 2: "Merkez B", 10: "Merkez A; Merkez B"})
    ok("öznitelik zaten doğru: ortak vana “Merkez A; Merkez B”, eskimiş yok (kilitli ve alansız katmanlar sorun değil)", updated)
    stale = scene(preset={3: "Merkez A", 4: "Merkez B", 5: "Merkez A", 1: "Merkez A", 9: "Merkez A", 6: "Merkez B", 7: "Merkez B", 2: "Merkez B", 8: "Eski", 10: "Merkez A"})
    ok("eskimiş: başka ad, eksik ortak ad, kaynaksız nesnede artakalan değer", stale)
    blank = scene(preset={3: "   ", 4: "Merkez A"})
    ok("boşluktan ibaret değer yok sayılır", blank)
    three = scene(device_terminals=[{"id": "yg", "name": "Yüksek gerilim"}, {"id": "ag", "name": "Alçak gerilim"}, {"id": "ort", "name": "Orta gerilim"}], edge12=("ort", None))
    ok("terminal yolu dışında kalan kenar (12) kaynaksız sayılır", three)
    ok("terminali adı olmayan uç hepsine geçer; güvenilir değil (unverified)", scene(edge12=(None, None)))
    ok("terminal adıyla da bağlanır: Alçak gerilim", scene(device_terminals=[{"id": "yg", "name": "Yüksek gerilim"}, {"id": "ag", "name": "Alçak gerilim"}]))
    ok("terminaller tanımsız: cihaz bir nokta", scene(terminals=False))
    off = scene(extra=[point(13, "kaynak", (900, 900), {"Ad": "Uzak"})])
    ok("kenara değmeyen kaynak yalnız kendisine ulaşır; güvenilir değil", off)
    refused("projede olmayan ağ", scene(), "unknown_network", "“ag-9” kimlikli ağ projede yok. Var olan bir ağın kimliğini verin.", network="ag-9")
    refused("alt ağları tanımlı olmayan ağ", scene(subnetworks=False), "no_subnetworks",
            "“Elektrik” ağının alt ağları tanımlı değil; ağın subnetworks alanını yazın (cad.network.define).")
    refused("çizimde Kaynak düğümü olmayan ağ", scene(sources=False), "no_controllers",
            "“Elektrik” ağının çizimde Kaynak düğümü yok; alt ağ denetleyicisi bulunamadı.")
    return out


# ── cad.subnetworks.update ───────────────────────────────────────────────────────────────────────────────────────


def skipped_layers(setup, wrong, entity, attribute):
    """The layers the update leaves alone and the warnings they give, in the order they first come among the wrong."""
    layers = []
    rows = {}
    for i, want in wrong:
        layer = entity[i]["layerId"]
        if layer not in rows:
            layers.append(layer)
        rows.setdefault(layer, []).append((i, want))
    nodes = {n["id"]: n for n in setup["layers"]}
    warnings, writes, skipped, invalid = [], [], 0, 0
    for layer in layers:
        n = nodes[layer]
        why = None
        if n["locked"]:
            why = "kilitli"
        elif n.get("fields") and not any(f["name"] == attribute for f in n["fields"]):
            why = f"“{attribute}” alanını taşımıyor"
        if why:
            skipped += len(rows[layer])
            warnings.append({"code": "subnetwork_layer_skipped", "message": f"“{n['name']}” katmanı {why}; {len(rows[layer])} nesnesine alt ağ adı yazılmadı.", "path": None})
            continue
        field = next((f for f in n.get("fields", []) if f["name"] == attribute), None)
        for i, want in rows[layer]:
            if field and want is not None and field.get("kind") == "text" and field.get("length") and len(want) > field["length"]:
                invalid += 1
            else:
                writes.append((i, want))
    if invalid:
        skipped += invalid
        warnings.append({"code": "values_skipped", "message": f"{invalid} nesnenin alt ağ adı “{attribute}” alanının kurallarına uymadığı için yazılmadı.", "path": None})
    return warnings, writes, skipped


def update_answer(setup):
    found, wrong, entity = find_answer(setup)
    attribute = found["attribute"]
    warnings, writes, skipped = skipped_layers(setup, wrong, entity, attribute)
    written = sum(1 for _, want in writes if want is not None)
    cleared = sum(1 for _, want in writes if want is None)
    out = {"found": found, "written": written, "skipped": skipped, "cleared": cleared, "revision": "$current"}
    return out, [{k: v for k, v in w.items() if not (k == "path" and v is None)} for w in warnings], writes, entity


def entity_attrs(entity, writes, attribute):
    """The objects' attributes after the writes: only those the update touched."""
    out = {}
    for i, want in writes:
        attrs = dict(entity[i]["attrs"])
        if want is None:
            attrs.pop(attribute, None)
        else:
            attrs[attribute] = want
        out[str(i)] = {"attrs": attrs}
    return out


def update_cases():
    out = []

    def written(name, setup, undo=False, again=False):
        answer, warnings, writes, entity = update_answer(setup)
        attribute = answer["found"]["attribute"]
        expect = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "fields": entity_attrs(entity, writes, attribute)} if writes else {"canUndo": False, "dirty": False, "revision": "same"}
        steps = [step("execute", {"network": "ag-1"}, completed(answer, warnings), expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Alt ağları güncelle", "expect": {"canUndo": False, "canRedo": True, "fields": {str(i): {"attrs": entity[i]["attrs"]} for i, _ in writes}}})
        if again:
            # The write is the attribute's own: the second time finds nothing to change.
            after = copy.deepcopy(setup)
            for e in after["entities"]:
                for i, want in writes:
                    if e["id"] == i:
                        if want is None:
                            e["attrs"].pop(attribute, None)
                        else:
                            e["attrs"][attribute] = want
            again_answer, again_warnings, again_writes, _ = update_answer(after)
            steps.append(step("execute", {"network": "ag-1"}, completed(again_answer, again_warnings), {"revision": "same"}))
        out.append(case(name, setup, steps))

    def refused(name, setup, input_, code, message, path="network", status="failed", extra=None):
        err = {"code": code, "message": message, "path": path, **(extra or {})}
        out.append(case(name, setup, [step("execute", input_, {"status": status, "error": err}, UNCHANGED)]))

    written("yaz: 9 nesneye ad, kilitli ve alansız katmandaki kenar atlanır (iki uyarı); ortak vana “Merkez A; Merkez B”", scene(), undo=True)
    written("yazdıktan sonra yeniden: değişecek nesne yok", scene(), again=True)
    base = scene()
    answer, warnings, _, _ = update_answer(base)
    out.append(case("plan yazmadan aynısını söyler", base, [step("plan", {"network": "ag-1"}, completed(answer, warnings), UNCHANGED)]))
    out.append(case("doğrula: uyarıları söyler, hiçbir şey yazılmaz", base, [step("validate", {"network": "ag-1"}, {"status": "completed", "output": None, "warnings": warnings}, UNCHANGED)]))
    written("öznitelik zaten doğru: yazılacak bir şey yok", scene(preset={3: "Merkez A", 4: "Merkez A", 5: "Merkez A", 1: "Merkez A", 9: "Merkez A", 6: "Merkez B", 7: "Merkez B", 2: "Merkez B", 10: "Merkez A; Merkez B", 11: "Merkez A", 12: "Merkez A"}))
    written("eskimiş değerler düzelir, kaynaksız nesnenin değeri alınır (cleared)", scene(preset={3: "Merkez A", 4: "Merkez B", 5: "Merkez A", 1: "Merkez A", 9: "Merkez A", 6: "Merkez B", 7: "Merkez B", 2: "Merkez B", 8: "Eski", 10: "Merkez A"}))
    written("Bağlı: hepsi iki ada sahip olur", scene(reach="connected"))
    written("kapalı vana ötesini keser; vana iki adı taşır", scene(valve="kapalı"))
    written("aynı adlı denetleyiciler tek ad yazar", scene(s1={"Ad": "Merkez"}, s2={"Ad": "MERKEZ"}))
    written("başka öznitelik adı (AltAg)", scene(attribute="AltAg"))
    written("etiketten ad: ad alanı yok", scene(name=None, s1={}, s2={"Ad": "Öbür"}))
    written("adı olmayan denetleyici Kaynak <sıra> alır", scene(s1={}, s2={}))
    typed = scene(hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text", "length": 40}])
    written("alanı olan katmana da yazılır (hat 2)", typed)
    short = scene(hat2_fields=[{"name": "Ad", "kind": "text"}, {"name": ATTRIBUTE, "kind": "text", "length": 5}])
    written("alanın uzunluğunu aşan ad yazılmaz (values_skipped)", short)
    unlocked = scene()
    unlocked["layers"][2]["locked"] = False
    written("kilidi açık katmana da yazılır", unlocked)
    written("terminal yolu dışındaki kenar kaynaksız: değeri varsa alınır", scene(device_terminals=[{"id": "yg", "name": "Yüksek gerilim"}, {"id": "ag", "name": "Alçak gerilim"}, {"id": "ort", "name": "Orta gerilim"}], edge12=("ort", None), preset={12: "Eski"}))
    written("kenara değmeyen kaynak da adını alır", scene(extra=[point(13, "kaynak", (900, 900), {"Ad": "Uzak"})]))
    refused("projede olmayan ağ", scene(), {"network": "ag-9"}, "unknown_network", "“ag-9” kimlikli ağ projede yok. Var olan bir ağın kimliğini verin.")
    refused("alt ağları tanımlı olmayan ağ", scene(subnetworks=False), {"network": "ag-1"}, "no_subnetworks",
            "“Elektrik” ağının alt ağları tanımlı değil; ağın subnetworks alanını yazın (cad.network.define).")
    refused("çizimde Kaynak düğümü olmayan ağ", scene(sources=False), {"network": "ag-1"}, "no_controllers",
            "“Elektrik” ağının çizimde Kaynak düğümü yok; alt ağ denetleyicisi bulunamadı.")
    refused("geçersiz beklenen sürüm", scene(), {"network": "ag-1", "expectedRevision": "on iki"}, "invalid_revision",
            "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", path="expectedRevision")
    answer, warnings, _, _ = update_answer(scene())
    out.append(case("sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz", scene(), [
        {"op": "captureRevision", "as": "once"},
        step("execute", {"network": "ag-1"}, completed(answer, warnings), {"revision": "changed"}),
        step("execute", {"network": "ag-1", "expectedRevision": "$once"},
             {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
             {"revision": "same"}),
    ]))
    shut_update = scene(net_override={"closed": "durum = 'kapalı'"})
    shut_update["entities"][4]["attrs"]["durum"] = "kapalı"
    written("kapalı kenar izlenmez: ad almaz", shut_update)
    return out


def build(command, title, note, cases):
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": command,
        "commandVersion": 1,
        "title": title,
        "note": note,
        "setup": cases[0]["setup"],
        "cases": cases,
    }


def files():
    find = find_cases()
    update = update_cases()
    return {
        ROOT / "fixtures/commands/v1/cad.subnetworks.find.json": build(
            "cad.subnetworks.find", "Alt ağları bul (ADR 0557 §2)",
            "scripts/fixtures/subnetworks_command_cases.py yazar; beklentiler ADR 0557'nin kurallarından, KentOS kodu olmadan (terminal yolları ve kapalı düğümler network_terminals_cases.py'nin aramasıyla). Her durum kendi çizimini kurar.", find),
        ROOT / "fixtures/commands/v1/cad.subnetworks.update.json": build(
            "cad.subnetworks.update", "Alt ağları güncelle (ADR 0557 §2)",
            "scripts/fixtures/subnetworks_command_cases.py yazar; beklentiler ADR 0557'nin kurallarından, KentOS kodu olmadan. `expect.fields` yalnız yazılan nesnelerin öznitelikleridir. Her durum kendi çizimini kurar.", update),
    }


def main():
    texts = {p: json.dumps(d, ensure_ascii=False, indent=2) + "\n" for p, d in files().items()}
    if "--check" in sys.argv:
        for p, t in texts.items():
            if not p.exists() or p.read_text(encoding="utf-8") != t:
                print(f"{p.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
                return 1
        print("Alt ağ komut durumları güncel:", ", ".join(f"{p.name} {len(files()[p]['cases'])}" for p in texts))
        return 0
    for p, t in texts.items():
        p.write_text(t, encoding="utf-8")
        print(f"{p.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
