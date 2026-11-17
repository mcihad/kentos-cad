#!/usr/bin/env python3
"""cad.form.answer v1 (“Kontrol formunun cevapları”, docs/adr/8077): the shared cases, written from the ADR without
KentOS code.

The drawing: a wall and a room; its architecture settings hold no answer, or (a second drawing) the answers to the 1st
and 7th questions, the 1st with a note.

The rules (ADR 8077 §2):
- `answers` empty: nothing_to_set at `answers`.
- Each change in its order: a question given before: invalid_input at `answers[i].no`; null with a note or a basis:
  invalid_input at `answers[i].note` (the note first), else `answers[i].basis`; null: its number 1–21 (else
  invalid_input at `answers[i].no`), its answer, note and basis taken away. Evet/Hayır: its note and its basis the given
  ones with their ends' white space taken away, else the ones it has (none: empty); then its number 1–21
  (`answers[i].no`), its note at most 300 characters and without a control character (`answers[i].note`), its basis at
  most 600 and without one (`answers[i].basis`).
- Then expectedRevision (invalid_revision, revision_conflict). Any refusal writes nothing.
- The answers by question; a note empty is not written. Nothing changed: no undo step, the drawing not dirty.
- One undo step “Kontrol formunun cevapları”. The answer: the drawing's answers then, by question; the revision.
"""
import argparse
import copy
import json
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.form.answer.json"
COMMAND = "cad.form.answer"
STEP = "Kontrol formunun cevapları"
MAX_NOTE = 300
MAX_BASIS = 600


def U(n):
    return f"0192f5a1-8077-7000-8000-{n:012x}"


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(id, name):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def drawing(answers=None):
    ents = [
        {"kind": "wall", "id": 1, "layerId": "0", "attrs": {}, "pts": [P(0, 0), P(6, 0)], "width": 0.25,
         "justify": "center"},
        {"kind": "space", "id": 2, "layerId": "0", "attrs": {},
         "parts": [{"outer": {"pts": [P(0, 0.125), P(6, 0.125), P(6, 4), P(0, 4)]}}], "name": "Salon"},
    ]
    settings = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 50.0, "workspace": "cad"}
    if answers is not None:
        settings["arch"] = {"formAnswers": answers}
    return {
        "format": "kentos.document", "version": 2, "name": "Kontrol formu",
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


def answers_of(d):
    return copy.deepcopy(((d["settings"].get("arch") or {}).get("formAnswers")) or [])


def control(s):
    return any(unicodedata.category(c) == "Cc" for c in s)


def run(d, inp):
    """The result and the drawing after, as ADR 8077 §2 says."""
    changes = inp["answers"]
    if not changes:
        return refusal("nothing_to_set", "answers"), d
    now = answers_of(d)
    for i, c in enumerate(changes):
        if any(x["no"] == c["no"] for x in changes[:i]):
            return refusal("invalid_input", f"answers[{i}].no"), d
        if c["yes"] is None:
            if "note" in c:
                return refusal("invalid_input", f"answers[{i}].note"), d
            if "basis" in c:
                return refusal("invalid_input", f"answers[{i}].basis"), d
            if not 1 <= c["no"] <= 21:
                return refusal("invalid_input", f"answers[{i}].no"), d
            now = [a for a in now if a["no"] != c["no"]]
            continue
        held = next((a for a in now if a["no"] == c["no"]), {})
        note = c["note"].strip() if "note" in c else held.get("note", "")
        basis = c["basis"].strip() if "basis" in c else held.get("basis", "")
        if not 1 <= c["no"] <= 21:
            return refusal("invalid_input", f"answers[{i}].no"), d
        if len(note) > MAX_NOTE or control(note):
            return refusal("invalid_input", f"answers[{i}].note"), d
        if len(basis) > MAX_BASIS or control(basis):
            return refusal("invalid_input", f"answers[{i}].basis"), d
        a = {"no": c["no"], "yes": c["yes"]}
        if note:
            a["note"] = note
        if basis:
            a["basis"] = basis
        now = [x for x in now if x["no"] != c["no"]] + [a]
    rev = inp.get("expectedRevision")
    if rev is not None:
        if not rev.isdigit() or (len(rev) > 1 and rev[0] == "0"):
            return refusal("invalid_revision", "expectedRevision"), d
        return refusal("revision_conflict", "expectedRevision", "conflict"), d
    now.sort(key=lambda a: a["no"])
    after = copy.deepcopy(d)
    if now != answers_of(d):
        arch = after["settings"].setdefault("arch", {})
        if now:
            arch["formAnswers"] = now
        else:
            arch.pop("formAnswers", None)
        # No empty architecture settings are kept.
        if not arch:
            del after["settings"]["arch"]
    return {"status": "completed", "output": {"answers": now, "revision": "$current"}, "warnings": []}, after


def state(d, dirty=None, can_undo=None, revision=None):
    """What is held: the architecture settings (the answers in them)."""
    out = {"settings": {"arch": d["settings"].get("arch")}}
    if dirty is not None:
        out["dirty"] = dirty
    if can_undo is not None:
        out["canUndo"] = can_undo
    if revision is not None:
        out["revision"] = revision
    return out


def cases():
    blank = drawing()
    held = drawing([{"no": 1, "yes": True, "note": "Bakanlığın sınırı yayımlanmadı"}, {"no": 7, "yes": True}])
    out = []

    def case(name, steps, setup=None):
        c = {"name": name, "steps": steps}
        if setup is not None:
            c["setup"] = setup
        out.append(c)

    def written(answers, setup=None, **extra):
        src = setup or blank
        inp = {"answers": answers, **extra}
        result, after = run(src, inp)
        assert result["status"] == "completed", (inp, result)
        assert after != src, inp
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(after, dirty=True, can_undo=True, revision="changed")},
                {"op": "undo", "returns": STEP, "expect": state(src, revision="changed")},
                {"op": "redo", "returns": STEP, "expect": state(after, revision="changed")}]

    def unchanged(answers, setup=None):
        src = setup or blank
        inp = {"answers": answers}
        result, after = run(src, inp)
        assert result["status"] == "completed" and after == src, (inp, result)
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(src, dirty=False, can_undo=False, revision="same")}]

    def refused(answers, setup=None, **extra):
        src = setup or blank
        inp = {"answers": answers, **extra}
        result, _ = run(src, inp)
        assert result["status"] != "completed", (inp, result)
        return [{"op": "execute", "input": inp, "result": result,
                 "expect": state(src, dirty=False, can_undo=False, revision="same")}]

    # Written.
    case("Dosya boyutu Evet, notuyla; mimari ayarı olmayan çizimde",
         written([{"no": 1, "yes": True, "note": "Bakanlığın sınırı yayımlanmadı"}]))
    case("Gelişim seviyesi Hayır, notsuz", written([{"no": 7, "yes": False}]))
    case("Üç soru birlikte, soru sırasıyla", written([{"no": 10, "yes": True}, {"no": 1, "yes": True}, {"no": 7, "yes": True}]))
    case("Var olan cevap Hayır'a döner, notu kalır", written([{"no": 1, "yes": False}], setup=held), setup=held)
    case("Yeni not eskisinin yerini alır", written([{"no": 1, "yes": True, "note": "Sınır 2027'de"}], setup=held),
         setup=held)
    case("Boş not notu kaldırır", written([{"no": 1, "yes": True, "note": ""}], setup=held), setup=held)
    case("null cevabı ve notu kaldırır", written([{"no": 1, "yes": None}], setup=held), setup=held)
    case("Son cevap da kalkınca liste boşalır",
         written([{"no": 1, "yes": None}, {"no": 7, "yes": None}], setup=held), setup=held)
    case("Notun iki ucundaki boşluk alınır", written([{"no": 10, "yes": True, "note": "  Emsal tablosu kontrol edildi \t"}]))
    case("300 karakterlik not", written([{"no": 1, "yes": True, "note": "a" * MAX_NOTE}]))
    case("Notta Türkçe harfler", written([{"no": 7, "yes": True, "note": "Gelişim seviyesi LOD 300; müellif İ.Ş.Ç."}]))
    case("Dosyadan cevaplanan soruya verilen cevap da tutulur", written([{"no": 9, "yes": True}]))
    case("Yeni soru eklenir, öbürleri kalır", written([{"no": 10, "yes": False, "note": "Bir zon eksik"}], setup=held),
         setup=held)
    based = drawing([{"no": 10, "yes": True,
                      "basis": "2 zon (2 DAHIL); alanların emsal hesap tablosuyla karşılaştırılması müellifin."}])
    case("Cevap dayandığı kanıtı tutar", written([{"no": 10, "yes": True, "basis":
                                                  " 2 zon (2 DAHIL); alanların emsal hesap tablosuyla karşılaştırılması müellifin. "}]))
    case("Kanıt verilmezse var olan kalır", written([{"no": 10, "yes": False}], setup=based), setup=based)
    case("Yeni kanıt eskisinin yerini alır", written([{"no": 10, "yes": True, "basis": "3 zon (3 DAHIL)"}], setup=based),
         setup=based)
    case("null kanıtı da kaldırır", written([{"no": 10, "yes": None}], setup=based), setup=based)
    # Nothing changed.
    case("Aynı cevap: değişiklik yok", unchanged([{"no": 7, "yes": True}], setup=held), setup=held)
    case("Cevabı olmayan sorunun null'ı: değişiklik yok", unchanged([{"no": 10, "yes": None}], setup=held), setup=held)
    # Refused.
    case("Boş liste", refused([]))
    case("0. soru", refused([{"no": 0, "yes": True}]))
    case("22. soru", refused([{"no": 22, "yes": True}]))
    case("22. sorunun null'ı", refused([{"no": 22, "yes": None}]))
    case("Aynı soru iki kez", refused([{"no": 1, "yes": True}, {"no": 1, "yes": False}]))
    case("null ile not", refused([{"no": 1, "yes": None, "note": "Not"}], setup=held), setup=held)
    case("301 karakterlik not", refused([{"no": 1, "yes": True, "note": "a" * (MAX_NOTE + 1)}]))
    case("Satır sonlu not", refused([{"no": 1, "yes": True, "note": "İlk satır\nİkinci satır"}]))
    case("null ile kanıt", refused([{"no": 10, "yes": None, "basis": "2 zon"}], setup=based), setup=based)
    case("601 karakterlik kanıt", refused([{"no": 10, "yes": True, "basis": "k" * (MAX_BASIS + 1)}]))
    case("Satır sonlu kanıt", refused([{"no": 10, "yes": True, "basis": "2 zon\n2 DAHIL"}]))
    case("Biri geçersizse hiçbiri yazılmaz", refused([{"no": 1, "yes": True}, {"no": 30, "yes": True}], setup=held),
         setup=held)
    case("Sürüm sayı değil", refused([{"no": 1, "yes": True}], expectedRevision="abc"))
    case("Çizim başka sürümde", refused([{"no": 1, "yes": True}], expectedRevision="999"))
    return out


def document_of_cases():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1,
            "title": "Kontrol formunun cevapları: Evet/Hayır ve not, kaldırma, değişmeyen çağrı; retler",
            "note": "ADR 8077. Beklenenler scripts/fixtures/form_answers_cases.py'nin ADR'den yazılmış kurallarından, "
                    "KentOS kodu olmadan.",
            "setup": drawing(), "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    doc = document_of_cases()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND}: {len(doc['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
