#!/usr/bin/env python3
"""cad.delivery.edit v1 (“Teslim kaydı”, docs/adr/8099): the shared cases, written from the ADR without KentOS code.

The drawing: a wall and a room, two review notes; its architecture settings hold no record, or (a second drawing) the
first package's delivery, or (a third) that delivery returned with its reason and both notes.

The rules (ADR 8099 §2):
- `add`: an id given is a lowercase hyphenated UUID (else invalid_uid at `change.id`) the record does not hold (else
  duplicate_delivery at `change.id`); its number one more than the largest. The texts' edges trimmed; an optional
  text left empty is not written. Then the record's checks: a revision of three digits (`change.revision`), the
  manifest's 64 lowercase hexadecimal digits (`change.package.manifest`), a folder or a source at most 300 characters
  (`change.package.path`, `change.package.source`), the authority said and at most 300 (`change.to`), a real day
  YYYY-AA-GG from 1900 to 2200 (`change.at`), the publisher and the receipt at most 300 (`change.by`,
  `change.receipt`); a control character but a line break or a tab is refused like a length: invalid_delivery.
- `respond`: the id's form (invalid_uid at `change.id`), the record named (delivery_not_found); each note's form
  (invalid_uid at `change.notes[j]`) and the drawing's note (review_note_not_found), then a note given twice
  (invalid_delivery at `change.notes[j]`, the second); the answer's day real (`change.at`) and not before the delivery's
  (`change.at`); a return's reason said (`change.reason`), a reason at most 4000 (`change.reason`), its receipt at most
  300 (`change.receipt`). It takes the place of the answer the record has.
- `remove`: the id's form, the record named; the record taken away, the review notes kept.
- Then expectedRevision (invalid_revision, revision_conflict). Any refusal writes nothing.
- Nothing changed (the same answer): no undo step, the drawing not dirty. Otherwise one undo step “Teslim kaydı”. The
  answer: the changed record's id and number, the drawing's record then, the revision.
"""
import argparse
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.delivery.edit.json"
COMMAND = "cad.delivery.edit"
STEP = "Teslim kaydı"
LABEL = 300
TEXT = 4000


def U(n):
    return f"0192f5a1-8099-7000-8000-{n:012x}"


D1, D2, N1, N2 = U(0xd1), U(0xd2), U(0xa1), U(0xa2)
M1, M2 = "3f0a5c1e9b7d2846a1c0e5f3b8d9274c6e1f0a3b5c7d9e2f4a6b8c0d1e3f5a7b", "b" * 64
SIVAS = "Sivas Belediyesi İmar ve Şehircilik Müdürlüğü"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def first():
    return {"id": D1, "number": 1, "revision": "000", "package": {"manifest": M1, "files": 14}, "to": SIVAS,
            "channel": "epys", "at": "2026-11-17"}


def returned():
    d = first()
    d["response"] = {"outcome": "returned", "at": "2026-11-24", "reason": "Kat yüksekliği planın notunu aşıyor.",
                     "notes": [N1, N2]}
    return d


def drawing(deliveries=None):
    ents = [
        {"kind": "wall", "id": 1, "layerId": "0", "attrs": {}, "pts": [P(0, 0), P(6, 0)], "width": 0.25,
         "justify": "center"},
        {"kind": "space", "id": 2, "layerId": "0", "attrs": {},
         "parts": [{"outer": {"pts": [P(0, 0.125), P(6, 0.125), P(6, 4), P(0, 4)]}}], "name": "Salon"},
    ]
    settings = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 50.0, "workspace": "cad",
                "reviewNotes": [{"id": N1, "number": 1, "text": "Kat yüksekliği 4,40 m; planın notu 4,00 m."},
                                {"id": N2, "number": 2, "text": "Salonun kapısı 0,80 m."}]}
    if deliveries is not None:
        settings["arch"] = {"deliveries": deliveries}
    return {
        "format": "kentos.document", "version": 2, "name": "Teslim kaydı",
        "settings": settings,
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [layer("0", "0")],
        "activeLayer": "0",
        "entities": ents,
        "uids": [U(e["id"]) for e in ents],
        "styles": {"items": [], "categories": []},
    }


def refusal(code, path, status="failed"):
    out = {"status": status, "error": {"code": code, "message": "$any", "path": path}}
    if status == "conflict":
        out["error"]["revision"] = "$current"
    return out


def is_uuid(s):
    return (isinstance(s, str) and len(s) == 36 and all(
        (c == "-") if i in (8, 13, 18, 23) else (c in "0123456789abcdef") for i, c in enumerate(s)))


def is_day(s):
    if not (isinstance(s, str) and len(s) == 10 and s[4] == "-" and s[7] == "-"):
        return False
    try:
        y, m, d = int(s[0:4]), int(s[5:7]), int(s[8:10])
    except ValueError:
        return False
    leap = (y % 4 == 0 and y % 100 != 0) or y % 400 == 0
    days = {1: 31, 2: 29 if leap else 28, 3: 31, 4: 30, 5: 31, 6: 30, 7: 31, 8: 31, 9: 30, 10: 31, 11: 30, 12: 31}
    return 1900 <= y <= 2200 and m in days and 1 <= d <= days[m]


def bad_text(s, most):
    return len(s) > most or any(ord(c) < 32 and c not in "\n\t" or ord(c) == 127 for c in s)


def said(s):
    return s.strip() if isinstance(s, str) and s.strip() else None


def record_of(d):
    return copy.deepcopy(((d["settings"].get("arch") or {}).get("deliveries")) or [])


def run(d, inp):
    """The result and the drawing after, as ADR 8099 §2 says."""
    c = inp["change"]
    now = record_of(d)
    kind = c["kind"]
    if kind == "add":
        if "id" in c:
            if not is_uuid(c["id"]):
                return refusal("invalid_uid", "change.id"), d
            if any(x["id"] == c["id"] for x in now):
                return refusal("duplicate_delivery", "change.id"), d
        new = {"id": c["id"], "number": max([x["number"] for x in now], default=0) + 1, "revision": c["revision"].strip()}
        package = {"manifest": c["package"]["manifest"], "files": c["package"]["files"]}
        for k in ("path", "source"):
            if said(c["package"].get(k)):
                package[k] = said(c["package"][k])
        new["package"] = package
        new["to"] = c["to"].strip()
        new["channel"] = c.get("channel", "epys")
        new["at"] = c["at"].strip()
        for k in ("by", "receipt"):
            if said(c.get(k)):
                new[k] = said(c[k])
        rev = new["revision"]
        if not (len(rev) == 3 and rev.isdigit() and rev.isascii()):
            return refusal("invalid_delivery", "change.revision"), d
        m = package["manifest"]
        if not (len(m) == 64 and all(ch in "0123456789abcdef" for ch in m)):
            return refusal("invalid_delivery", "change.package.manifest"), d
        for k in ("path", "source"):
            if k in package and bad_text(package[k], LABEL):
                return refusal("invalid_delivery", f"change.package.{k}"), d
        if not new["to"] or bad_text(new["to"], LABEL):
            return refusal("invalid_delivery", "change.to"), d
        if not is_day(new["at"]):
            return refusal("invalid_delivery", "change.at"), d
        for k in ("by", "receipt"):
            if k in new and bad_text(new[k], LABEL):
                return refusal("invalid_delivery", f"change.{k}"), d
        now.append(new)
        changed = new
    else:
        if not is_uuid(c["id"]):
            return refusal("invalid_uid", "change.id"), d
        i = next((i for i, x in enumerate(now) if x["id"] == c["id"]), None)
        if i is None:
            return refusal("delivery_not_found", "change.id"), d
        if kind == "remove":
            changed = now.pop(i)
        else:
            notes = c.get("notes", [])
            held = [n["id"] for n in d["settings"]["reviewNotes"]]
            for j, n in enumerate(notes):
                if not is_uuid(n):
                    return refusal("invalid_uid", f"change.notes[{j}]"), d
                if n not in held:
                    return refusal("review_note_not_found", f"change.notes[{j}]"), d
            for j, n in enumerate(notes):
                if n in notes[:j]:
                    return refusal("invalid_delivery", f"change.notes[{j}]"), d
            resp = {"outcome": c["outcome"], "at": c["at"].strip()}
            if said(c.get("reason")):
                resp["reason"] = said(c["reason"])
            if notes:
                resp["notes"] = list(notes)
            if said(c.get("receipt")):
                resp["receipt"] = said(c["receipt"])
            if not is_day(resp["at"]) or resp["at"] < now[i]["at"]:
                return refusal("invalid_delivery", "change.at"), d
            if resp["outcome"] == "returned" and "reason" not in resp:
                return refusal("invalid_delivery", "change.reason"), d
            if "reason" in resp and bad_text(resp["reason"], TEXT):
                return refusal("invalid_delivery", "change.reason"), d
            if "receipt" in resp and bad_text(resp["receipt"], LABEL):
                return refusal("invalid_delivery", "change.receipt"), d
            now[i]["response"] = resp
            changed = now[i]
    rev = inp.get("expectedRevision")
    if rev is not None:
        if not rev.isdigit() or (len(rev) > 1 and rev[0] == "0"):
            return refusal("invalid_revision", "expectedRevision"), d
        return refusal("revision_conflict", "expectedRevision", "conflict"), d
    after = copy.deepcopy(d)
    if now != record_of(d):
        arch = after["settings"].setdefault("arch", {})
        if now:
            arch["deliveries"] = now
        else:
            arch.pop("deliveries", None)
        # No empty architecture settings are kept.
        if not arch:
            del after["settings"]["arch"]
    output = {"id": changed["id"], "number": changed["number"], "deliveries": now, "revision": "$current"}
    return {"status": "completed", "output": output, "warnings": []}, after


def state(d, dirty=None, can_undo=None, revision=None):
    """What is held: the architecture settings (the record in them)."""
    out = {"settings": {"arch": d["settings"].get("arch")}}
    if dirty is not None:
        out["dirty"] = dirty
    if can_undo is not None:
        out["canUndo"] = can_undo
    if revision is not None:
        out["revision"] = revision
    return out


def add(**over):
    c = {"kind": "add", "id": D2, "revision": "001", "package": {"manifest": M2, "files": 15}, "to": SIVAS,
         "channel": "epys", "at": "2026-11-28"}
    c.update(over)
    return c


def respond(**over):
    c = {"kind": "respond", "id": D1, "outcome": "accepted", "at": "2026-11-24"}
    c.update(over)
    return c


def cases():
    blank, held, back = drawing(), drawing([first()]), drawing([returned()])
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
    case("İlk paket e-PYS'ye, mimari ayarı olmayan çizimde",
         written(add(id=D1, revision="000", package={"manifest": M1, "files": 14}, at="2026-11-17")))
    case("Basılı teslim, yayımlayan, makbuz, yayın klasörü ve kaynak sürümle",
         written(add(id=D1, revision="000", channel="paper", by="Mimar Ayşe Yılmaz", receipt="2026/1187",
                     package={"manifest": M1, "files": 14, "path": "/Projeler/Konut/Yayin-000", "source": "42"})))
    case("İkinci paket bir sonraki numarayı alır", written(add(), setup=held), setup=held)
    case("İdarenin yazılımıyla teslim", written(add(channel="authority"), setup=held), setup=held)
    case("Metinlerin iki ucundaki boşluk alınır",
         written(add(id=D1, revision=" 000 ", to="  Sivas Belediyesi \t", by=" Mimar Ayşe Yılmaz ")))
    case("Boş makbuz ve boş klasör yazılmaz",
         written(add(id=D1, revision="000", receipt="   ", package={"manifest": M1, "files": 14, "path": " "})))
    case("Kabul", written(respond(), setup=held), setup=held)
    case("İade, gerekçesi ve iki inceleme notuyla",
         written(respond(outcome="returned", reason="Kat yüksekliği planın notunu aşıyor.", notes=[N1, N2]),
                 setup=held), setup=held)
    case("Yeni cevap eskisinin yerini alır", written(respond(at="2026-11-30", receipt="2026/1201"), setup=back),
         setup=back)
    case("Cevap teslimle aynı gün", written(respond(at="2026-11-17"), setup=held), setup=held)
    case("Kabulün gerekçesi de yazılır", written(respond(reason="Uygun görüldü."), setup=held), setup=held)
    case("Kayıt kaldırılır, inceleme notları kalır", written({"kind": "remove", "id": D1}, setup=back), setup=back)
    two = drawing([returned(), {**first(), "id": D2, "number": 2, "revision": "001", "package": {"manifest": M2,
                                                                                              "files": 15},
                                "at": "2026-11-28"}])
    case("Bir kaydın kaldırılması öbürünün numarasını değiştirmez", written({"kind": "remove", "id": D1}, setup=two),
         setup=two)
    case("2028 artık yılının 29 Şubatı", written(add(id=D1, revision="000", at="2028-02-29")))
    # Nothing changed.
    same = returned()["response"]
    case("Aynı cevap: değişiklik yok",
         unchanged(respond(outcome="returned", at=same["at"], reason=same["reason"], notes=same["notes"]), back),
         setup=back)
    # Refused.
    case("Kimlik UUID değil", refused(add(id="d1")))
    case("Kimlik büyük harfli", refused(add(id=D2.upper()), setup=held), setup=held)
    case("Kimlik zaten var", refused(add(id=D1), setup=held), setup=held)
    case("Revizyon iki rakam", refused(add(revision="01"), setup=held), setup=held)
    case("Revizyon harfli", refused(add(revision="A01"), setup=held), setup=held)
    case("Paketin SHA-256'sı büyük harfli", refused(add(package={"manifest": M1.upper(), "files": 14}), setup=held),
         setup=held)
    case("Paketin SHA-256'sı kısa", refused(add(package={"manifest": M1[:40], "files": 14}), setup=held), setup=held)
    case("Kurum boş", refused(add(to="  "), setup=held), setup=held)
    case("301 karakterlik kurum", refused(add(to="K" * (LABEL + 1)), setup=held), setup=held)
    case("Gün 31 Şubat", refused(add(at="2026-02-31"), setup=held), setup=held)
    case("Gün GG.AA.YYYY", refused(add(at="28.11.2026"), setup=held), setup=held)
    case("2027'nin 29 Şubatı", refused(add(at="2027-02-29"), setup=held), setup=held)
    case("Denetim karakterli makbuz", refused(add(receipt="2026\u00071187"), setup=held), setup=held)
    case("Olmayan kayda cevap", refused(respond(id=D2), setup=held), setup=held)
    case("Cevabın kimliği UUID değil", refused(respond(id="d1"), setup=held), setup=held)
    case("İade gerekçesiz", refused(respond(outcome="returned"), setup=held), setup=held)
    case("İade boş gerekçeyle", refused(respond(outcome="returned", reason="  "), setup=held), setup=held)
    case("Cevap tesliminden önce", refused(respond(at="2026-11-16"), setup=held), setup=held)
    case("Olmayan inceleme notu", refused(respond(outcome="returned", reason="Bak", notes=[U(0xa9)]), setup=held),
         setup=held)
    case("Not kimliği UUID değil", refused(respond(outcome="returned", reason="Bak", notes=["a1"]), setup=held),
         setup=held)
    case("Aynı not iki kez", refused(respond(outcome="returned", reason="Bak", notes=[N1, N1]), setup=held),
         setup=held)
    case("4001 karakterlik gerekçe", refused(respond(reason="g" * (TEXT + 1)), setup=held), setup=held)
    case("Olmayan kaydı kaldır", refused({"kind": "remove", "id": D2}, setup=held), setup=held)
    case("Sürüm sayı değil", refused(add(), setup=held, expectedRevision="abc"), setup=held)
    case("Çizim başka sürümde", refused(add(), setup=held, expectedRevision="999"), setup=held)
    return out


def document_of_cases():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Teslim kaydı: paket eklenir, cevap verilir (kabul, gerekçeli iade), kayıt kaldırılır; retler",
            "note": "ADR 8099. Beklenenler scripts/fixtures/delivery_cases.py'nin ADR'den yazılmış kurallarından, "
                    "KentOS kodu olmadan.",
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
