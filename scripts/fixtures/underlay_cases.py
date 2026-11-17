#!/usr/bin/env python3
"""cad.underlay.edit v1 (“Altlık model”, docs/adr/8101): the shared cases, written from the ADR without KentOS code.

The drawing: one wall on TUREF / TM36 (EPSG:5256), ±0.00 at 1285.4 m (its IFC system can be said); a second drawing
with no system. The files are the repository's: KentOS's local-system building on EPSG:5256 (a scaled map conversion),
buildingSMART's IFC 4.3 architecture sample on EPSG:32760 (millimetres, Scale 0.001), its IFC2X3 sample (no map
conversion), and a file that is not IFC. Each file is read here for its SHA-256 and size (its bytes), its schema (the
header's FILE_SCHEMA), its target system (IFCPROJECTEDCRS's name) and whether it has a map conversion.

The rules (ADR 8101 §2):
- `add`: an id given is a lowercase hyphenated UUID (invalid_uid at `change.id`) the drawing does not hold
  (duplicate_model_underlay at `change.id`); the file's path absolute (invalid_file_path at `change.file`), the file
  there (file_not_found), read as IFC (model_underlay_unreadable); placed (model_underlay_not_placed at
  `change.placement`): `origin` always; `map` (the default) only when the file has a map conversion to the very system
  the project's IFC files are written in (and the project can say one: a system and ±0.00's height), with no other
  height datum and a scale its length unit explains. The name is the file's name unless given; texts trimmed, an empty
  optional text not written; shown. Then the list's checks (invalid_model_underlay at `change.<field>`), the count (50,
  too_many_model_underlays at `change`).
- `set`: the id (invalid_uid, model_underlay_not_found); at least one field (nothing_to_set at `change`); an empty
  discipline or revision takes it away; a placement other than the one held reads the file again and places it.
- `reload`: the id; the file (or `file`, its new place) read again and placed as held; the digest, size and schema
  renewed.
- `remove`: the id; the underlay taken away.
- Then expectedRevision. A refusal writes nothing; nothing changed: no undo step. Otherwise one step “Altlık model”.
"""
import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.underlay.edit.json"
COMMAND = "cad.underlay.edit"
STEP = "Altlık model"
MOST = 50
LABEL = 200
SYSTEM = "EPSG:5256"

LOCAL = "arch/v1/ifc/yerel-sistem.ifc"
BSI43 = "ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc"
BSI2X3 = "ifc/v1/disaridan/bsi-ifc2x3-building-architecture.ifc"
NOT_IFC = "arch/v1/ifc/var.ifc"
MISSING = "ifc/v1/yok.ifc"


def U(n):
    return f"0192f5a1-8101-7000-8000-{n:012x}"


A, B = U(0xa), U(0xb)


def fixture(rel):
    return "$fixture:" + rel


def read(rel):
    """What the reference reads of a file: none when missing; its digest, size, schema, target system, conversion."""
    path = ROOT / "fixtures" / rel
    if not path.exists():
        return None
    data = path.read_bytes()
    text = data.decode("latin-1")
    schema = re.search(r"FILE_SCHEMA\s*\(\s*\(\s*'([^']+)'", text)
    crs = re.search(r"IFCPROJECTEDCRS\s*\(\s*'([^']*)'", text)
    return {"sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data),
            "schema": schema.group(1) if schema else None, "crs": crs.group(1) if crs else None,
            "map": "IFCMAPCONVERSION" in text}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def drawing(underlays=None, system=True):
    ents = [{"kind": "wall", "id": 1, "layerId": "0", "attrs": {}, "pts": [P(0, 0), P(6, 0)], "width": 0.25,
             "justify": "center"}]
    settings = {"srid": 5256 if system else 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2",
                "angleUnit": "deg", "plotScale": 50.0, "workspace": "cad", "arch": {"site": {"datum": 1285.4}}}
    if underlays:
        settings["arch"]["underlays"] = underlays
    return {
        "format": "kentos.document", "version": 2, "name": "Altlık model",
        "settings": settings,
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("0", "0")],
        "activeLayer": "0",
        "entities": ents,
        "uids": [U(e["id"]) for e in ents],
        "styles": {"items": [], "categories": []},
    }


def record(id, rel, **over):
    f = read(rel)
    r = {"id": id, "name": Path(rel).name, "file": fixture(rel), "sha256": f["sha256"], "bytes": f["bytes"],
         "schema": f["schema"]}
    r.update(over)
    return r


def refusal(code, path, status="failed"):
    out = {"status": status, "error": {"code": code, "message": "$any", "path": path}}
    if status == "conflict":
        out["error"]["revision"] = "$current"
    return out


def is_uuid(s):
    return (isinstance(s, str) and len(s) == 36 and all(
        (c == "-") if i in (8, 13, 18, 23) else (c in "0123456789abcdef") for i, c in enumerate(s)))


def said(s):
    return s.strip() if isinstance(s, str) and s.strip() else None


def bad_label(s, most):
    return (not s.strip()) or s.strip() != s or len(s) > most or any(ord(c) < 32 or ord(c) == 127 for c in s)


def held(d):
    return copy.deepcopy(d["settings"]["arch"].get("underlays") or [])


def placed(d, f, placement):
    """Whether the file read `f` is placed `placement` in drawing `d` (ADR 8101 §2)."""
    if placement == "origin":
        return True
    # The project says its system: a projected one and ±0.00's height. The files here are metres or say their
    # millimetres in their Scale, so the scale rule never refuses one of them.
    if d["settings"]["srid"] == 0:
        return False
    return f["map"] and f["crs"] == SYSTEM


def take(d, rel_or_path, placement, at):
    """The file at `rel_or_path` (a `$fixture:` path or another text) read and placed; its record's file parts."""
    if not rel_or_path.startswith("$fixture:"):
        return refusal("invalid_file_path", at), None
    rel = rel_or_path[len("$fixture:"):]
    f = read(rel)
    if f is None:
        return refusal("file_not_found", at), None
    if f["schema"] is None:
        return refusal("model_underlay_unreadable", at), None
    if not placed(d, f, placement):
        return refusal("model_underlay_not_placed", "change.placement"), None
    return None, f


def run(d, inp):
    """The result and the drawing after, as ADR 8101 §2 says."""
    c = inp["change"]
    now = held(d)
    kind = c["kind"]
    if kind == "add":
        if "id" in c:
            if not is_uuid(c["id"]):
                return refusal("invalid_uid", "change.id"), d
            if any(x["id"] == c["id"] for x in now):
                return refusal("duplicate_model_underlay", "change.id"), d
        placement = c.get("placement", "map")
        path = c["file"].strip()
        no, f = take(d, path, placement, "change.file")
        if no:
            return no, d
        new = {"id": c["id"], "name": said(c.get("name")) or Path(path).name, "file": path,
               "sha256": f["sha256"], "bytes": f["bytes"], "schema": f["schema"]}
        for k in ("discipline", "revision"):
            if said(c.get(k)):
                new[k] = said(c[k])
        if placement != "map":
            new["placement"] = placement
        now.append(new)
        changed = new
    else:
        if not is_uuid(c["id"]):
            return refusal("invalid_uid", "change.id"), d
        i = next((i for i, x in enumerate(now) if x["id"] == c["id"]), None)
        if i is None:
            return refusal("model_underlay_not_found", "change.id"), d
        u = now[i]
        if kind == "remove":
            changed = now.pop(i)
        elif kind == "set":
            keys = [k for k in ("name", "discipline", "revision", "placement", "shown") if k in c]
            if not keys:
                return refusal("nothing_to_set", "change"), d
            if "name" in c:
                u["name"] = c["name"].strip()
            for k in ("discipline", "revision"):
                if k in c:
                    if said(c[k]):
                        u[k] = said(c[k])
                    else:
                        u.pop(k, None)
            if "shown" in c:
                if c["shown"]:
                    u.pop("shown", None)
                else:
                    u["shown"] = False
            if "placement" in c and c["placement"] != u.get("placement", "map"):
                no, f = take(d, u["file"], c["placement"], "change.placement")
                if no:
                    return no, d
                u.update(sha256=f["sha256"], bytes=f["bytes"], schema=f["schema"])
                if c["placement"] == "map":
                    u.pop("placement", None)
                else:
                    u["placement"] = c["placement"]
            changed = u
        else:  # reload
            path = said(c.get("file")) or u["file"]
            no, f = take(d, path, u.get("placement", "map"), "change.file" if "file" in c else "change.id")
            if no:
                return no, d
            u.update(file=path, sha256=f["sha256"], bytes=f["bytes"], schema=f["schema"])
            changed = u
    if len(now) > MOST:
        return refusal("too_many_model_underlays", "change"), d
    for k in ("name", "discipline", "revision"):
        if k in changed and bad_label(changed[k], LABEL):
            return refusal("invalid_model_underlay", f"change.{k}"), d
    rev = inp.get("expectedRevision")
    if rev is not None:
        if not rev.isdigit() or (len(rev) > 1 and rev[0] == "0"):
            return refusal("invalid_revision", "expectedRevision"), d
        return refusal("revision_conflict", "expectedRevision", "conflict"), d
    after = copy.deepcopy(d)
    if now != held(d):
        arch = after["settings"]["arch"]
        if now:
            arch["underlays"] = now
        else:
            arch.pop("underlays", None)
    output = {"id": changed["id"], "underlays": now, "revision": "$current"}
    return {"status": "completed", "output": output, "warnings": []}, after


def state(d, dirty=None, can_undo=None, revision=None):
    """What is held: the architecture settings (the underlays in them)."""
    out = {"settings": {"arch": d["settings"].get("arch")}}
    if dirty is not None:
        out["dirty"] = dirty
    if can_undo is not None:
        out["canUndo"] = can_undo
    if revision is not None:
        out["revision"] = revision
    return out


def add(rel, **over):
    c = {"kind": "add", "id": A, "file": fixture(rel)}
    c.update(over)
    return c


def cases():
    blank, nosys = drawing(), drawing(system=False)
    one = drawing([record(A, LOCAL, discipline="Statik", revision="R1")])
    origin = drawing([record(A, BSI43, placement="origin")])
    full = drawing([record(U(0x100 + i), LOCAL) for i in range(MOST)])
    out = []

    def case(name, steps, setup=None):
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    def written(change, setup=None, **extra):
        src = setup or blank
        inp = {"change": change, **extra}
        result, after = run(src, inp)
        assert result["status"] == "completed", (inp, result)
        assert after != src, inp
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(after, dirty=True, can_undo=True, revision="changed")},
                {"op": "undo", "returns": STEP, "expect": state(src, revision="changed")},
                {"op": "redo", "returns": STEP, "expect": state(after, revision="changed")}]

    def unchanged(change, setup):
        inp = {"change": change}
        result, after = run(setup, inp)
        assert result["status"] == "completed" and after == setup, (inp, result)
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(setup, dirty=False, can_undo=False, revision="same")}]

    def refused(change, setup=None, **extra):
        src = setup or blank
        inp = {"change": change, **extra}
        result, _ = run(src, inp)
        assert result["status"] != "completed", (inp, result)
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(src, dirty=False, can_undo=False, revision="same")}]

    # Written.
    case("Projenin sisteminde harita dönüşümlü model", written(add(LOCAL)))
    case("Adı, disiplini ve revizyonuyla; metinlerin uçları alınır",
         written(add(LOCAL, name="  Statik model ", discipline=" Statik", revision="R2 ")))
    case("Boş disiplin yazılmaz", written(add(LOCAL, discipline="   ")))
    case("Başka sistemdeki IFC 4.3 örneği başlangıç noktasına", written(add(BSI43, placement="origin")))
    case("Harita dönüşümsüz IFC2X3 başlangıç noktasına", written(add(BSI2X3, placement="origin")))
    case("Sistemsiz projede başlangıç noktasına", written(add(LOCAL, placement="origin"), setup=nosys), setup=nosys)
    case("İkinci altlık listenin sonuna", written(add(BSI43, id=B, placement="origin"), setup=one), setup=one)
    case("Gizlenir", written({"kind": "set", "id": A, "shown": False}, setup=one), setup=one)
    case("Yeniden adlandırılır ve revizyonu kaldırılır",
         written({"kind": "set", "id": A, "name": "Statik R3", "revision": ""}, setup=one), setup=one)
    case("Başlangıç noktasından harita dönüşümüne geçemeyen model yerinde kalır, adı değişir",
         written({"kind": "set", "id": A, "name": "Mimari örnek"}, setup=origin), setup=origin)
    case("Harita dönüşümünden başlangıç noktasına", written({"kind": "set", "id": A, "placement": "origin"}, setup=one),
         setup=one)
    case("Yeni yerinden yeniden okunur: başka dosya, başka şema",
         written({"kind": "reload", "id": A, "file": fixture(BSI2X3)}, setup=origin), setup=origin)
    case("Kaldırılır, dosya yerinde kalır", written({"kind": "remove", "id": A}, setup=one), setup=one)
    # Nothing changed.
    case("Değişmemiş dosya yeniden okunur: değişiklik yok", unchanged({"kind": "reload", "id": A}, one), setup=one)
    case("Aynı ad: değişiklik yok", unchanged({"kind": "set", "id": A, "discipline": "Statik"}, one), setup=one)
    # Refused.
    case("Başka koordinat sistemindeki model harita dönüşümüyle yerleştirilmez", refused(add(BSI43)))
    case("Harita dönüşümü olmayan model yerleştirilmez", refused(add(BSI2X3)))
    case("Sistemsiz projede harita dönüşümüyle yerleştirilmez", refused(add(LOCAL), setup=nosys), setup=nosys)
    case("Başlangıç noktasındaki örnek harita dönüşümüne geçmez",
         refused({"kind": "set", "id": A, "placement": "map"}, setup=origin), setup=origin)
    case("Göreli yol", refused(add(LOCAL, file="statik.ifc")))
    case("Olmayan dosya", refused(add(MISSING)))
    case("IFC olmayan dosya", refused(add(NOT_IFC)))
    case("Kimlik UUID değil", refused(add(LOCAL, id="a")))
    case("Kimlik zaten var", refused(add(LOCAL), setup=one), setup=one)
    case("Denetim karakterli ad", refused(add(LOCAL, name="Statik\u0007model")))
    case("201 karakterlik revizyon", refused(add(LOCAL, revision="R" * (LABEL + 1))))
    case("Elli birinci altlık", refused(add(LOCAL, id=A), setup=full), setup=full)
    case("Değiştirilecek bir şey yok", refused({"kind": "set", "id": A}, setup=one), setup=one)
    case("Olmayan altlığı kaldır", refused({"kind": "remove", "id": B}, setup=one), setup=one)
    case("Yeni yeri olmayan dosya", refused({"kind": "reload", "id": A, "file": fixture(MISSING)}, setup=one),
         setup=one)
    case("Sürüm sayı değil", refused(add(LOCAL), expectedRevision="abc"))
    case("Çizim başka sürümde", refused(add(LOCAL), expectedRevision="999"))
    return out


def document_of_cases():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Altlık model: IFC dosyası eklenir, yerleştirilir ya da nedeniyle reddedilir, ayarlanır, yeniden "
                     "okunur, kaldırılır",
            "note": "ADR 8101. Beklenenler scripts/fixtures/underlay_cases.py'nin ADR'den yazılmış kurallarından; "
                    "dosyaların SHA-256'sı, boyu, şeması ve sistemi betikte baytlarından okunur, KentOS kodu yok.",
            "setup": drawing(), "cases": cases()}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="the file on disk is what this writes")
    args = ap.parse_args()
    text = json.dumps(document_of_cases(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)",
                  file=sys.stderr)
            return 1
        print(f"{COMMAND} durumları güncel: {len(json.loads(text)['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(json.loads(text)['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
