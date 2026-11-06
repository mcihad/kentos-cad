#!/usr/bin/env python3
"""The shared cases of accepted standards findings (docs/adr/1129), written from the ADR without KentOS code:

- fixtures/commands/v1/cad.standards.exceptions.json (`cad.standards.exceptions` v1: accept and withdraw)
- fixtures/commands/v1/cad.standards.check.exceptions.json (`cad.standards.check` v1: accepted and lapsed findings)

    python3 scripts/fixtures/standards_exceptions_cases.py           # writes the files
    python3 scripts/fixtures/standards_exceptions_cases.py --check   # writes nothing; compares

The findings are standards_cases.py's (its `report`, read as data). The rules, as the ADR gives them, after ArcGIS's
MarkAsException and ADR 1260's topology exceptions: an accepted finding names the finding (kind, subject, layer), the
standard's source (`origin.id`, else the standard's name) and its revision (`origin.revision`, else none), and keeps
what the finding said then (expected, found) and the reason, who and when. The check lists every finding; one an
acceptance against the same source names is `accepted` when the revision and both values are the same, else
`lapsed`, its reason given either way; `accepted` and `lapsed` are counted apart. The command accepts findings of
today's check (a reason required, at most 500 characters; one already accepted alike is left; one lapsed or with
another reason is written again as it is now) or withdraws acceptances against the source (the finding may be gone).
Refusals in order: the input's, the reason's, the standard's, the revision's, then each finding's. The accepted
findings are a setting: no undo step.
"""

import copy
import importlib.util
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


S = _load("standards_cases")
T = S.T
STANDARD = S.STANDARD
NAME = STANDARD["name"]
OFIS = {"id": "ofis-a", "revision": "2026.1"}
REASON = "Müşteri duvarların kırmızı basılmasını istedi."
OTHER = "Kurum eski paftalarla aynı görünmesini istiyor."


def base_issues():
    return S.report(STANDARD)["issues"]


def find(kind, subject):
    return next(i for i in base_issues() if i["kind"] == kind and i["subject"] == subject)


COLOR = find("layerColor", "A-DUVAR")
PLOT = find("layerPlot", "A-YAZI")
WEIGHT = find("layerWeight", "a-eksen")
FONT = find("textFont", "A-Yazı")
MISSING = find("missingLayer", "A-KAPI")


def key(i):
    k = {"kind": i["kind"], "subject": i["subject"]}
    if "layer" in i:
        k["layer"] = i["layer"]
    return k


def record(i, standard=NAME, revision="", reason=REASON, by=None, at=None, **over):
    """An accepted finding as the project keeps it."""
    r = {"kind": i["kind"], "subject": i["subject"]}
    if "layer" in i:
        r["layer"] = i["layer"]
    r["standard"] = standard
    if revision:
        r["revision"] = revision
    for f in ("expected", "found"):
        if f in i:
            r[f] = i[f]
    r["reason"] = reason
    if by is not None:
        r["by"] = by
    if at is not None:
        r["at"] = at
    r.update(over)
    return r


def state_of(issue, records, origin):
    """The check's state of `issue` under `records`, against `origin` (id, revision)."""
    for r in records:
        if (r["kind"], r["subject"], r.get("layer"), r["standard"]) == (issue["kind"], issue["subject"],
                                                                       issue.get("layer"), origin["id"]):
            same = (r.get("revision", "") == origin["revision"] and r.get("expected") == issue.get("expected")
                    and r.get("found") == issue.get("found"))
            return ("accepted" if same else "lapsed"), r["reason"]
    return None, None


def checked(records, origin=None):
    """The check's report with the records' states."""
    o = origin or {"id": NAME, "revision": ""}
    out = []
    accepted = lapsed = 0
    for i in base_issues():
        i = copy.deepcopy(i)
        state, reason = state_of(i, records, o)
        if state:
            i["state"], i["reason"] = state, reason
            accepted += state == "accepted"
            lapsed += state == "lapsed"
        out.append(i)
    r = {"standard": NAME, "issues": out, "fixable": sum(1 for i in out if i["fixable"])}
    if accepted:
        r["accepted"] = accepted
    if lapsed:
        r["lapsed"] = lapsed
    r["revision"] = "$current"
    return r


def doc(records=()):
    d = S.setup()
    if records:
        d["settings"]["standardExceptions"] = copy.deepcopy(list(records))
    return d


def same(d):
    return {"revision": "same", "canUndo": False, "settings": {"standardExceptions": d["settings"].get("standardExceptions")}}


# ── cad.standards.check with accepted findings ────────────────────────────────

def check_cases():
    out = []

    def case(name, records, input_=None, origin=None):
        d = doc(records)
        i = {"standard": STANDARD, **(input_ or {})}
        answer = checked(records, origin)
        out.append({"name": name, "setup": d, "steps": [
            T.step("execute", i, T.completed(answer), same(d)),
            T.step("plan", i, T.completed(answer), {"revision": "same"})]})

    case("Kabul yok: bütün bulgular açık", [])
    case("Kabul edilen bulgu listede kalır, kabul edilmiş sayılır", [record(COLOR)])
    case("İki kabul ayrı sayılır", [record(COLOR), record(PLOT, reason=OTHER)])
    case("Kim ve ne zaman kabulde kalır, durumu değiştirmez", [record(COLOR, by="ayse", at="2026-11-06T11:40:00+03:00")])
    case("Çizimdeki değer değişince kabul düşer (lapsed), gerekçesi söylenir", [record(COLOR, found="#FF0000")])
    case("Standardın beklediği değişince kabul düşer", [record(WEIGHT, expected="0.25")])
    case("Bulunan değeri olmayan bulgu da kabul edilir", [record(MISSING)])
    case("Kaynağın revizyonu verilince kabul o revizyona bağlıdır",
         [record(COLOR, standard="ofis-a", revision="2026.1")], {"origin": OFIS}, OFIS)
    case("Revizyon değişince kabul düşer",
         [record(COLOR, standard="ofis-a", revision="2026.1")], {"origin": {"id": "ofis-a", "revision": "2026.2"}},
         {"id": "ofis-a", "revision": "2026.2"})
    case("Revizyonsuz kabul revizyonlu denetimde düşer",
         [record(COLOR, standard="ofis-a")], {"origin": OFIS}, OFIS)
    case("Başka kaynağın kabulü bu standarda sayılmaz", [record(COLOR, standard="ofis-b")])
    case("Kaynak verilmezse standardın adı kaynaktır; kimliğe bağlı kabul sayılmaz",
         [record(COLOR, standard="ofis-a", revision="2026.1")])
    case("Katman başka olunca aynı bulgu değildir", [record(COLOR, layer="wall")])
    case("Tür başka olunca aynı bulgu değildir", [record(COLOR, kind="layerWeight")])
    case("Konu harfi harfine karşılaştırılır", [record(COLOR, subject="a-duvar")])
    case("Artık olmayan bulgunun kabulü raporda görünmez", [record(COLOR, subject="A-ESKİ", layer="eski")])
    case("Düşmüş ve kabul edilmiş birlikte sayılır", [record(COLOR), record(FONT, found="arimo")])
    case("Yazı tipi bulgusu kabul edilir", [record(FONT)])
    case("Katmansız ve katmanlı bulgu birlikte", [record(MISSING), record(PLOT)])
    case("Uygulanabilir sayısı kabulden etkilenmez", [record(COLOR), record(PLOT), record(WEIGHT)])
    case("Her bulgu kabul edilmiş", [record(i) for i in base_issues()])
    return out


# ── cad.standards.exceptions ──────────────────────────────────────────────────

def edited(changed, exceptions):
    return T.completed({"changed": changed, "exceptions": exceptions, "revision": "$current"})


def exceptions_cases():
    out = []

    def case(name, records, input_, result, after):
        d = doc(records)
        i = {"standard": STANDARD, **input_}
        if after is None:
            expect = same(d)
        else:
            expect = {"canUndo": False, "dirty": True, "settings": {"standardExceptions": after or None}}
        out.append({"name": name, "setup": d, "steps": [T.step("execute", i, result, expect)]})

    def refused(name, input_, code, message, path, records=()):
        case(name, records, input_, T.failed(code, message, path), None)

    accept = lambda *items, **kw: {"operation": "accept", "findings": [key(i) for i in items], "reason": REASON, **kw}
    withdraw = lambda *items, **kw: {"operation": "withdraw", "findings": [key(i) for i in items], **kw}

    case("Kabul et: gerekçesiyle kaydedilir; proje ayarıdır, geri alma adımı değildir", [], accept(COLOR),
         edited(1, 1), [record(COLOR)])
    case("Kabul et: kim ve ne zaman verilirse kaydedilir", [],
         accept(COLOR, by="ayse", at="2026-11-06T11:40:00+03:00"), edited(1, 1),
         [record(COLOR, by="ayse", at="2026-11-06T11:40:00+03:00")])
    case("İki bulgu birden, verildikleri sırayla", [], accept(PLOT, COLOR), edited(2, 2), [record(PLOT), record(COLOR)])
    case("Bağlı paketin kaynağıyla: kimliği ve revizyonu kaydedilir", [], accept(COLOR, origin=OFIS), edited(1, 1),
         [record(COLOR, standard="ofis-a", revision="2026.1")])
    case("Aynı gerekçeyle yeniden kabul: değişmez", [record(COLOR)], accept(COLOR), edited(0, 1), None)
    case("Başka gerekçeyle: yeniden yazılır", [record(COLOR)], {**accept(COLOR), "reason": OTHER}, edited(1, 1),
         [record(COLOR, reason=OTHER)])
    case("Düşmüş kabul bugünkü değerle yeniden yazılır", [record(COLOR, found="#FF0000")], accept(COLOR),
         edited(1, 1), [record(COLOR)])
    case("Yeni revizyonda yeniden kabul: eskisinin yerine", [record(COLOR, standard="ofis-a", revision="2026.1")],
         accept(COLOR, origin={"id": "ofis-a", "revision": "2026.2"}), edited(1, 1),
         [record(COLOR, standard="ofis-a", revision="2026.2")])
    case("Başka kaynak için ayrı kabul", [record(COLOR)], accept(COLOR, origin=OFIS), edited(1, 2),
         [record(COLOR), record(COLOR, standard="ofis-a", revision="2026.1")])
    case("Geri al: kabul kalkar, bulgu yeniden açıktır", [record(COLOR), record(PLOT)], withdraw(COLOR), edited(1, 1),
         [record(PLOT)])
    case("Son kabul geri alınınca liste kalkar", [record(COLOR)], withdraw(COLOR), edited(1, 0), [])
    case("Düşmüş kabul de geri alınır", [record(COLOR, found="#FF0000")], withdraw(COLOR), edited(1, 0), [])
    case("Bulgusu artık olmayan kabul geri alınır", [record(COLOR, subject="A-ESKİ", layer="eski")],
         withdraw({"kind": "layerColor", "subject": "A-ESKİ", "layer": "eski"}), edited(1, 0), [])
    d = doc()
    i = {"standard": STANDARD, **accept(COLOR)}
    out.append({"name": "Planla: ne yapacağını söyler, yazmaz", "setup": d,
                "steps": [T.step("plan", i, edited(1, 1), same(d))]})
    out.append({"name": "Doğrula: yazmaz", "setup": d, "steps": [T.step("validate", i, T.VALID, same(d))]})

    refused("Bulgu yok: ret", {"operation": "accept", "findings": [], "reason": REASON}, "invalid_input",
            "Bulgu verilmedi; cad.standards.check'in bulgularından en az birini (tür, konu, katman) verin.", "findings")
    refused("Aynı bulgu iki kez: ret", accept(COLOR, COLOR), "invalid_input",
            "“A-DUVAR” bulgusu birden çok kez verildi.", "findings[1]")
    refused("Geri almada gerekçe: ret", {**withdraw(COLOR), "reason": REASON}, "invalid_input",
            "Kabulü geri almak reason istemez; yalnız bulguları verin.", "reason", [record(COLOR)])
    refused("Geri almada kim: ret", {**withdraw(COLOR), "by": "ayse"}, "invalid_input",
            "Kabulü geri almak by istemez; yalnız bulguları verin.", "by", [record(COLOR)])
    refused("Boş kim: ret", accept(COLOR, by=" "), "invalid_input",
            "by boş, satır sonlu ya da 200 karakterden uzun olamaz.", "by")
    refused("Satır sonlu zaman: ret", accept(COLOR, at="2026-11-06\n11:40"), "invalid_input",
            "at boş, satır sonlu ya da 200 karakterden uzun olamaz.", "at")
    refused("Gerekçesiz kabul: ret", {"operation": "accept", "findings": [key(COLOR)]}, "reason_required",
            "Bulguyu kabul etmek için gerekçesini yazın (reason): neden standarttan ayrıldığı kayda geçer.", "reason")
    refused("Boşluktan ibaret gerekçe: ret", {**accept(COLOR), "reason": "   "}, "reason_required",
            "Bulguyu kabul etmek için gerekçesini yazın (reason): neden standarttan ayrıldığı kayda geçer.", "reason")
    refused("Uzun gerekçe: ret", {**accept(COLOR), "reason": "ö" * 501}, "invalid_input",
            "Gerekçe 501 karakter; en çok 500 karakter olur, kısaltın.", "reason")
    refused("Standart yok: ret", {"operation": "accept", "findings": [key(COLOR)], "reason": REASON, "standard": None},
            "no_standard", "Standardı adıyla (name: mimari-ofis) ya da kendisini (standard) verin.", "name")
    refused("Yerleşik standart yok: ret", {**accept(COLOR), "standard": None, "name": "yok"}, "standard_not_found",
            "“yok” adlı yerleşik standart yok; bunlardan birini verin: mimari-ofis.", "name")
    refused("Bugünkü denetimde olmayan bulgu: ret", accept({"kind": "layerColor", "subject": "A-KAPI"}),
            "finding_not_found",
            "Çizimin bugünkü denetiminde “A-KAPI” için layerColor bulgusu yok; önce cad.standards.check ile aynı "
            "standartla denetleyin.", "findings[0]")
    refused("İkinci bulgu yoksa hiçbiri yazılmaz", accept(COLOR, {"kind": "layerPlot", "subject": "A-DUVAR", "layer": "duvar"}),
            "finding_not_found",
            "Çizimin bugünkü denetiminde “A-DUVAR” için layerPlot bulgusu yok; önce cad.standards.check ile aynı "
            "standartla denetleyin.", "findings[1]")
    refused("Kabul edilmemiş bulguyu geri almak: ret", withdraw(COLOR), "finding_not_found",
            f"“A-DUVAR” için layerColor bulgusu “{NAME}” standardına karşı kabul edilmemiş; geri alınacak kabul yok.",
            "findings[0]")
    refused("Başka kaynağın kabulü geri alınamaz", withdraw(COLOR, origin=OFIS), "finding_not_found",
            "“A-DUVAR” için layerColor bulgusu “ofis-a” standardına karşı kabul edilmemiş; geri alınacak kabul yok.",
            "findings[0]", [record(COLOR)])
    refused("Sürüm yazısı geçersiz: ret", {**accept(COLOR), "expectedRevision": "on iki"}, "invalid_revision",
            S.INVALID_REVISION, "expectedRevision")
    d = doc()
    out.append({"name": "Beklenen sürüm eski: çatışma", "setup": d, "steps": [T.step(
        "execute", {"standard": STANDARD, **accept(COLOR), "expectedRevision": "999"},
        {"status": "conflict", "error": {"code": "revision_conflict", "message": S.CONFLICT, "path": "expectedRevision",
                                         "revision": "$current"}}, same(d))]})
    for c in out:
        for st in c["steps"]:
            if st.get("input", {}).get("standard", 1) is None:
                del st["input"]["standard"]
    return out


NOTE = ("ADR 1129. Bağımsız başvuru scripts/fixtures/standards_exceptions_cases.py: bulgular standards_cases.py'nin, "
        "kabul kuralları ADR'den elle; KentOS kodu yok.")


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": doc(), "cases": cases}


def files():
    return {
        OUT / "cad.standards.exceptions.json": build("cad.standards.exceptions", "Standart bulgusunu kabul et",
                                                     exceptions_cases()),
        OUT / "cad.standards.check.exceptions.json": build("cad.standards.check", "Standardı denetle: kabul edilmiş bulgular",
                                                           check_cases()),
    }


def main():
    bad = 0
    for path, d in files().items():
        text = json.dumps(d, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text("utf-8") != text:
                print(f"{path.relative_to(ROOT)} güncel değil")
                bad += 1
            continue
        path.write_text(text, "utf-8")
        print(f"yazıldı: {path.relative_to(ROOT)} ({len(d['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
