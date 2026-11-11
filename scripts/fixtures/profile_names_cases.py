#!/usr/bin/env python3
"""The shared cases of `cad.profiles.names` and of a mapping's form of names (plan/map-cad.md MAP-17, docs/adr/5180),
written from the ADR without KentOS code.

    python3 scripts/fixtures/profile_names_cases.py           # writes the cases
    python3 scripts/fixtures/profile_names_cases.py --check   # compares

The rules (ADR 5180), written here by hand:

- a name's words: runs of letters and digits; a capital starts a new word after a small letter or a digit, and after a
  capital when a small letter follows it; a digit stays with the word before it;
- `ascii`: ç ğ ı İ ö ş ü â î û (and their capitals) become c g i I o s u a i u; any other letter beyond ASCII cannot be
  spelled; `keep`: the letters stay, cased the Turkish way (I ↔ ı, İ ↔ i);
- `lowerCamelCase`: the first word small; a later word of capitals only (two letters or more) stays, any other later
  word gets a capital first letter and the rest small; joined without a space;
- `upperSnakeCase`: every word in capitals, joined by `_`;
- a field's name in the institution: the mapping's own (`fields`) first, then the form, then the profile's;
- two names of a class clash when they are the same folded the Turkish way (I → ı, İ → i, then small letters); each of
  the two rows says so; a row the form cannot spell has no name and does not clash;
- a mapping with a form holds only when no row has a problem; `cad.profiles.check` and `.apply` read the profile through
  it as through a mapping that wrote every changed name itself.

The check and apply steps' answers are profile_cases.py's reference rules, run on the profile read through the mapping.
"""
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILES = ROOT / "fixtures/profiles/v1/files"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PC = _load("profile_cases")
step, completed, failed, VALID = PC.step, PC.completed, PC.failed, PC.VALID
NAMES, CHECK, APPLY = "cad.profiles.names", "cad.profiles.check", "cad.profiles.apply"
PROFILE, SMALL, SIVAS = PC.PROFILE, PC.SMALL, PC.SIVAS
_FOLDER = [0]


def folder():
    _FOLDER[0] += 1
    return f"pn{_FOLDER[0]:02d}"


# ── The rules ────────────────────────────────────────────────────────────────

TO_ASCII = {"ç": "c", "Ç": "C", "ğ": "g", "Ğ": "G", "ı": "i", "İ": "I", "ö": "o", "Ö": "O", "ş": "s", "Ş": "S",
            "ü": "u", "Ü": "U", "â": "a", "Â": "A", "î": "i", "Î": "I", "û": "u", "Û": "U"}
CASE_WORD = {"lowerCamelCase": "lowerCamelCase", "upperSnakeCase": "BÜYÜK_HARF"}


def words(name):
    out, w = [], ""
    for i, c in enumerate(name):
        if not c.isalnum():
            if w:
                out.append(w)
                w = ""
            continue
        if w and c.isupper():
            before = name[i - 1]
            after = name[i + 1] if i + 1 < len(name) else ""
            if before.islower() or before.isnumeric() or (before.isupper() and after.islower()):
                out.append(w)
                w = ""
        w += c
    if w:
        out.append(w)
    return out


def small(s, turkish):
    if turkish:
        s = s.replace("I", "ı").replace("İ", "i")
    return s.lower()


def capital(s, turkish):
    if turkish:
        s = s.replace("i", "İ").replace("ı", "I")
    return s.upper()


def abbreviation(w):
    letters = [c for c in w if c.isalpha()]
    return len(letters) >= 2 and all(c.isupper() for c in letters)


def spelled(name, form):
    """(name, None) or (None, why)."""
    ws = words(name)
    if not ws:
        return None, "adında harf ya da rakam yok"
    turkish = form.get("letters", "keep") == "keep"
    if not turkish:
        out = []
        for w in ws:
            t = ""
            for c in w:
                a = TO_ASCII.get(c, c)
                if ord(a) > 127:
                    return None, f"“{c}” harfinin ASCII karşılığı yok"
                t += a
            out.append(t)
        ws = out
    if form["case"] == "lowerCamelCase":
        parts = []
        for k, w in enumerate(ws):
            if k == 0:
                parts.append(small(w, turkish))
            elif abbreviation(w):
                parts.append(w)
            else:
                parts.append(capital(w[0], turkish) + small(w[1:], turkish))
        return "".join(parts), None
    return "_".join(capital(w, turkish) for w in ws), None


def fold(s):
    out = ""
    for c in s:
        if c == "I":
            out += "ı"
        elif c == "İ":
            out += "i"
        else:
            low = c.lower()
            out += low if len(low) == 1 else c
    return out


def rows_of(p, m, form):
    rows = []
    for c in p["classes"]:
        mc = next((x for x in (m or {}).get("classes", []) if x["class"] == c["id"]), None)
        here = []
        for f in c["fields"]:
            written = next((x for x in (mc or {}).get("fields", []) if x["profile"] == f["name"]), None)
            row = {"class": c["id"], "classTitle": c["title"], "field": f["name"]}
            if written:
                row.update(name=written["name"], source="mapping")
            elif form:
                name, why = spelled(f["name"], form)
                if name is None:
                    row.update(name="", source="form", problem="unfit",
                               message=f"“{f['name']}” alanının adı {CASE_WORD[form['case']]} biçiminde yazılamıyor: {why}; kurumdaki adını eşlemede yazın.")
                else:
                    row.update(name=name, source="form")
            else:
                row.update(name=f["name"], source="profile")
            here.append(row)
        for r in here:
            if "problem" in r:
                continue
            same = [o for o in here if o.get("problem") != "unfit" and fold(o["name"]) == fold(r["name"])]
            if len(same) < 2:
                continue
            formed = any(o["source"] == "form" for o in same)
            r["_clash"] = True
            r["message"] = (f"“{r['name']}” adı “{c['title']}” sınıfında başka bir alanın adı olurdu; birinin kurumdaki adını eşlemede yazın."
                            if formed else f"“{r['name']}” adı “{c['title']}” sınıfında başka bir alanın adı olurdu.")
        for r in here:
            if r.pop("_clash", False):
                r["problem"] = "clash"
                # The message key comes after the problem in the answer; order does not matter to the runner.
        rows.extend(here)
    return rows


def answer(p, m=None, form=None):
    """The output of cad.profiles.names; `form` the input's names, else the mapping's."""
    form = form if form is not None else (m or {}).get("names")
    if form is not None:
        form = {"case": form["case"], "letters": form.get("letters", "keep")}
    rows = rows_of(p, m, form)
    unfit = sum(1 for r in rows if r.get("problem") == "unfit")
    clashes = sum(1 for r in rows if r.get("problem") == "clash")
    out = {"profile": p["name"]}
    if m is not None:
        out["mapping"] = m["name"]
    if form is not None:
        out["names"] = form
    out.update(rows=rows, renamed=sum(1 for r in rows if "problem" not in r and r["name"] != r["field"]),
               unfit=unfit, clashes=clashes, holds=unfit == 0 and clashes == 0)
    return out


def written_out(p, m):
    """The mapping as one that writes every name its form changes itself: what check and apply read."""
    rows = rows_of(p, m, m.get("names"))
    out = copy.deepcopy(m)
    out.pop("names", None)
    for c in p["classes"]:
        changed = [r for r in rows if r["class"] == c["id"] and r["source"] == "form" and r["name"] != r["field"]]
        if not changed:
            continue
        mc = next((x for x in out["classes"] if x["class"] == c["id"]), None)
        if mc is None:
            mc = {"class": c["id"]}
            out["classes"].append(mc)
        mc.setdefault("fields", []).extend({"profile": r["field"], "name": r["name"]} for r in changed)
    return out


def through(p, m):
    return PC.through(p, written_out(p, m))


def first_problem(p, m):
    return next((r["message"] for r in rows_of(p, m, m.get("names")) if "message" in r), None)


# ── Profiles and mappings ────────────────────────────────────────────────────

def field(name, kind="text", **extra):
    return {"name": name, "kind": kind, **extra}


# A profile whose fields are named as people write them: Turkish letters, spaces, abbreviations, digits.
ADLAR = {
    "format": "kentos.data-profile", "version": 1, "name": "adlar", "title": "Ad biçimi deneme profili",
    "description": "Türkçe harfli, boşluklu, kısaltmalı ve rakamlı alan adları.", "basis": "Deneme profili.",
    "classes": [
        {"id": "detay", "title": "Detay", "theme": "Harita", "layer": ["Harita", "Detay"], "color": "#30A46C",
         "geometry": "point", "key": "TUCBS No",
         "fields": [field("TUCBS No"), field("Ölçü yöntemi"), field("Arazi Kullanımı Durumu 5000"), field("Adres 1 il"),
                    field("Işık şiddeti", "decimal", scale=1), field("Nitelik 01"), field("ParselNo"), field("sYLKK")]},
        {"id": "yapi", "title": "Yapı", "theme": "Harita", "layer": ["Harita", "Yapı"], "color": "#46A758",
         "geometry": "area", "fields": [field("Kat adedi", "integer"), field("Çatı türü"), field("İşlev")]},
    ],
}
# Two fields of one class the forms put on one name, a name with a letter beyond Turkish, one with no letter.
CAKISAN = {
    "format": "kentos.data-profile", "version": 1, "name": "cakisan", "title": "Çakışan adlar profili",
    "description": "Biçimlerin aynı ada düşürdüğü ve yazamadığı alanlar.", "basis": "Deneme profili.",
    "classes": [
        {"id": "ada", "title": "Ada", "theme": "Kadastro", "layer": ["Kadastro", "Ada"], "color": "#30A46C",
         "geometry": "area", "fields": [field("AdaNo"), field("Ada no"), field("Café"), field("#")]},
        {"id": "pafta", "title": "Pafta", "theme": "Kadastro", "layer": ["Kadastro", "Pafta"], "color": "#46A758",
         "geometry": "area", "fields": [field("PaftaNo")]},
    ],
}


def mapping(name, title, profile, classes, names=None, **extra):
    m = PC.mapping(name, title, profile, classes, **extra)
    if names is not None:
        m = {k: v for k, v in m.items() if k != "classes"}
        m["names"] = names
        m["classes"] = classes
    return m


CAMEL = {"case": "lowerCamelCase", "letters": "keep"}
CAMEL_ASCII = {"case": "lowerCamelCase", "letters": "ascii"}
SNAKE = {"case": "upperSnakeCase", "letters": "keep"}
SNAKE_ASCII = {"case": "upperSnakeCase", "letters": "ascii"}
# The form alone: no class named.
BUYUK = mapping("buyuk-harf", "Büyük harfli adlar", "tucbs-temel", [], names=SNAKE_ASCII)
# Sivas's own names first, the form for the rest.
SIVAS_FORM = mapping("sivas-bicim", "Sivas eşlemesi ve biçimi", "tucbs-temel", SIVAS["classes"], names=SNAKE_ASCII)
# The clash of CAKISAN mended by the mapping: AdaNo's name written.
CAKISAN_MENDED = mapping("cakisan-duzeltilmis", "Çakışmayı gideren eşleme", "cakisan",
                         [{"class": "ada", "fields": PC.renames([("AdaNo", "ADA_NUMARASI"), ("Café", "KAFE"), ("#", "NUMARA")])}],
                         names=SNAKE_ASCII)
BUYUK_FILE = "buyuk-harf.eslem.json"


# ── Cases ────────────────────────────────────────────────────────────────────

def ok_case(name, inp, out, setup=None, plan=True, before=()):
    steps = list(before)
    if plan:
        steps.append(step("plan", inp, completed(out), {"revision": "same"}))
    steps.append(step("execute", inp, completed(out), {"revision": "same", "dirty": False}))
    return {"name": name, "setup": setup or PC.empty_doc(), "steps": steps}


def refusal(name, inp, code, message, path, before=()):
    return {"name": name, "setup": PC.empty_doc(), "steps": list(before) + [
        step("execute", inp, failed(code, message, path), {"revision": "same", "dirty": False})]}


def on(command, s):
    """A step of another command (the plan, execute and validate steps of profile_cases.py's cases)."""
    if s["op"] in ("plan", "execute", "validate"):
        s = dict(s, command=command)
    return s


def names_cases():
    cases = []
    cases.append(ok_case("Eşleme ve biçim yoksa adlar profilin adlarıdır", {"name": "tucbs-temel"}, answer(PROFILE)))
    cases.append(ok_case("lowerCamelCase, Türkçe harfler korunur: MahalleKodu → mahalleKodu, Hmax → hmax",
                         {"name": "tucbs-temel", "names": CAMEL}, answer(PROFILE, form=CAMEL)))
    cases.append(ok_case("BÜYÜK_HARF, ASCII: KapiNo → KAPI_NO, Ilce → ILCE",
                         {"name": "tucbs-temel", "names": SNAKE_ASCII}, answer(PROFILE, form=SNAKE_ASCII)))
    cases.append(ok_case("BÜYÜK_HARF, Türkçe harflerle: KapiNo → KAPİ_NO (i'nin büyüğü İ)",
                         {"name": "tucbs-temel", "names": SNAKE}, answer(PROFILE, form=SNAKE)))
    cases.append(ok_case("Harfler söylenmezse korunur (letters keep)",
                         {"name": "tucbs-temel", "names": {"case": "lowerCamelCase"}}, answer(PROFILE, form={"case": "lowerCamelCase"}), plan=False))
    for title, form in (("lowerCamelCase, Türkçe harfler korunur: Işık şiddeti → ışıkŞiddeti, TUCBS No → tucbsNo", CAMEL),
                        ("lowerCamelCase, ASCII: Arazi Kullanımı Durumu 5000 → araziKullanimiDurumu5000, sYLKK kalır", CAMEL_ASCII),
                        ("BÜYÜK_HARF, ASCII: Ölçü yöntemi → OLCU_YONTEMI, Adres 1 il → ADRES_1_IL, Nitelik 01 → NITELIK_01", SNAKE_ASCII),
                        ("BÜYÜK_HARF, Türkçe harflerle: Çatı türü → ÇATI_TÜRÜ, İşlev → İŞLEV", SNAKE)):
        cases.append(ok_case(f"Adları insanların yazdığı profil, {title}", {"profile": ADLAR, "names": form}, answer(ADLAR, form=form)))
    cases.append(ok_case("Biçimin aynı ada düşürdüğü iki alan (AdaNo, Ada no) çakışır; ASCII'de Café yazılamaz, # hiç yazılamaz",
                         {"profile": CAKISAN, "names": SNAKE_ASCII}, answer(CAKISAN, form=SNAKE_ASCII)))
    cases.append(ok_case("Türkçe harfler korununca Café yazılır; çakışma ve # kalır",
                         {"profile": CAKISAN, "names": CAMEL}, answer(CAKISAN, form=CAMEL)))
    cases.append(ok_case("Eşleme çakışan ve yazılamayan adları kendisi yazınca eşleme tutar",
                         {"profile": CAKISAN, "mapping": CAKISAN_MENDED}, answer(CAKISAN, CAKISAN_MENDED)))
    cases.append(ok_case("Biçimsiz kurum eşlemesi: yazdığı adlar eşlemeden, öbürleri profilin",
                         {"name": "tucbs-temel", "mapping": SIVAS}, answer(PROFILE, SIVAS)))
    cases.append(ok_case("Eşlemenin kendi biçimi: yazdıkları önce gelir, kalanlar BÜYÜK_HARF",
                         {"name": "tucbs-temel", "mapping": SIVAS_FORM}, answer(PROFILE, SIVAS_FORM)))
    cases.append(ok_case("Verilen biçim eşlemenin biçiminin yerine okunur (önizleme)",
                         {"name": "tucbs-temel", "mapping": SIVAS_FORM, "names": CAMEL}, answer(PROFILE, SIVAS_FORM, CAMEL)))
    cases.append(ok_case("Biçimsiz eşlemeye biçim verilerek önizlenir",
                         {"name": "tucbs-temel", "mapping": SIVAS, "names": SNAKE_ASCII}, answer(PROFILE, SIVAS, SNAKE_ASCII)))
    cases.append(ok_case("Yalnız biçimden oluşan eşleme (sınıfsız) geçerlidir",
                         {"name": "tucbs-temel", "mapping": BUYUK}, answer(PROFILE, BUYUK)))
    clash = PC.with_class(0, fields=PC.renames([("AdaNo", "ParselNo")]), codes=[])
    cases.append(ok_case("Eşlemenin yazdığı ad başka alanın adına düşerse iki satır da söylenir (ret değil, satırda)",
                         {"name": "tucbs-temel", "mapping": clash}, answer(PROFILE, clash), plan=False))
    folded = PC.with_class(0, fields=PC.renames([("AdaNo", "parselno")]), codes=[])
    cases.append(ok_case("Çakışma Türkçe katlanmış adla bulunur: parselno ParselNo'nun adıdır",
                         {"name": "tucbs-temel", "mapping": folded}, answer(PROFILE, folded), plan=False))
    f = folder()
    cases.append(ok_case("Eşleme adıyla, projenin klasöründen",
                         {"name": "tucbs-temel", "mappingName": "buyuk-harf", "drawing": f"$tmp:{f}/cizim.kcad"},
                         answer(PROFILE, BUYUK), plan=False, before=PC.file_steps(f, **{BUYUK_FILE: PC.FIX + BUYUK_FILE})))
    cases.append({"name": "Doğrulama", "setup": PC.empty_doc(), "steps": [
        step("validate", {"name": "tucbs-temel", "names": SNAKE_ASCII}, VALID)]})
    # Refusals: the profile's and the mapping's shape; the names are rows.
    cases.append(refusal("profil yok", {"name": "yok"}, "unknown_profile", PC.UNKNOWN, "name"))
    title, bad_profile, why = PC.INVALID[0]
    cases.append(refusal(f"geçersiz profil: {title}", {"profile": bad_profile, "names": CAMEL}, "invalid_profile",
                         f"Veri profili geçersiz: {why}", "profile"))
    cases.append(refusal("profil verilmemiş", {"names": CAMEL}, "unknown_profile", PC.NONE_GIVEN, "name"))
    cases.append(refusal("eşleme başka profil için", {"name": "tucbs-temel", "mapping": PC.MAP_SMALL}, "invalid_profile_mapping",
                         f"Eşleme geçersiz: {PC.mapping_problem(PC.MAP_SMALL, PROFILE)}", "mapping"))
    no_class = mapping("bos", "Boş eşleme", "tucbs-temel", [])
    cases.append(refusal("biçimsiz ve sınıfsız eşleme", {"name": "tucbs-temel", "mapping": no_class}, "invalid_profile_mapping",
                         "Eşleme geçersiz: Eşlemede 1 ile 256 arası sınıf olur.", "mapping"))
    cases.append(refusal("eşleme adı verilmiş, çizimin dosyası yok", {"name": "tucbs-temel", "mappingName": "yok"}, "unknown_mapping",
                         PC.NO_MAPPING, "mappingName"))
    return cases


def through_cases():
    """Check and apply read the profile through a mapping's form, and refuse a mapping whose names do not hold."""
    cases = []
    prof = through(PROFILE, BUYUK)
    inp = {"name": "tucbs-temel", "mapping": BUYUK}
    rep = PC.report(PC.linked(PC.empty_doc(), []), prof, "buyuk-harf")
    cases.append({"name": "Veri profilini denetle biçimle okur: eksik alanlar BÜYÜK_HARF adlarıyla söylenir", "setup": PC.empty_doc(),
                  "steps": [step("execute", inp, completed(rep), {"revision": "same"}, command=CHECK)]})
    c, _after, _out = PC.apply_case("Veri profilini uygula biçimle okur: katmanlar profilin yolunda, alanlar BÜYÜK_HARF adlarıyla; tek adım geri alınır",
                                    PC.empty_doc(), inp, profile=prof)
    c["steps"] = [on(APPLY, s) for s in c["steps"]]
    cases.append(c)
    prof = through(PROFILE, SIVAS_FORM)
    inp = {"name": "tucbs-temel", "mapping": SIVAS_FORM}
    rep = PC.report(PC.linked(PC.institution_doc(), PC.INST_LINKS), prof, "sivas-bicim")
    cases.append({"name": "Kurumun yazdığı adlar ve biçim birlikte: kurumun çiziminde eşlemenin adları bulunur, kalanlar BÜYÜK_HARF adıyla eksik görünür",
                  "setup": PC.institution_doc(),
                  "steps": PC.link_steps(PC.INST_LINKS) + [step("execute", inp, completed(rep), {"revision": "same"}, command=CHECK)]})
    bad = mapping("cakisan-bicim", "Çakışan biçim", "cakisan", [], names=SNAKE_ASCII)
    for command in (CHECK, APPLY):
        cases.append({"name": f"{command}: biçimin çakıştırdığı eşleme reddedilir", "setup": PC.empty_doc(), "steps": [
            step("execute", {"profile": CAKISAN, "mapping": bad},
                 failed("invalid_profile_mapping", f"Eşleme geçersiz: {first_problem(CAKISAN, bad)}", "mapping"),
                 {"revision": "same", "dirty": False}, command=command)]})
    return cases


def files():
    doc = {"format": "kentos.command-cases", "version": 1, "command": NAMES, "commandVersion": 1,
           "title": "Kurumdaki alan adları",
           "note": "Written by scripts/fixtures/profile_names_cases.py from docs/adr/5180's rules; do not edit by hand.",
           "setup": PC.empty_doc(), "cases": names_cases() + through_cases()}
    return {OUT / f"{NAMES}.json": doc, FILES / BUYUK_FILE: BUYUK}


def main() -> int:
    bad = 0
    for path, value in files().items():
        text = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                print(f"{path.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
                bad += 1
            else:
                print(f"{path.name} güncel" + (f": {len(value['cases'])} durum." if "cases" in value else "."))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)}" + (f" ({len(value['cases'])} durum)" if "cases" in value else ""))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
