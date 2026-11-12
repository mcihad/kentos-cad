#!/usr/bin/env python3
"""The shared cases of `cad.asbuilt.review` (İzleme incelemesi, docs/adr/5220; plan/map-cad.md MAP-19), without KentOS
code.

    python3 scripts/fixtures/asbuilt_review_cases.py           # writes fixtures/commands/v1/cad.asbuilt.review.json
    python3 scripts/fixtures/asbuilt_review_cases.py --check   # writes nothing; compares

The drawing and the monitoring job's answer are asbuilt_series_cases.py's (by hand there). A review keeps, under its
name: the job as asked (layers, epochs and days, this epoch's day, the names' attribute, control marks, limits, the
line's confidence, the thresholds with the source the user wrote), each mark (with a series its track's last movement,
its speed, the line's or else the mean rate, its state and the limits beyond; without, its movement from the earlier
epoch), the digest and the reviewer's statement. The digest is SHA-256 of the job's layers, earlier epochs oldest
first, then `design`, then `measured`, each `k<TAB>layer` and its points `p<TAB>name<TAB>x<TAB>y<TAB>z` (six decimals,
`-` without a height) in the drawing's order, written here from the setup's own points. A review is current while
that digest is the one kept. The kept numbers are compared within 1e-6 in the answer; in the settings, compared
exactly, their places are held and their values any.
"""

import copy
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import asbuilt_cases as ab  # noqa: E402
import asbuilt_series_cases as sc  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "commands" / "v1" / "cad.asbuilt.review.json"
STEP = "İzleme incelemesi"
BAD = "invalid_input"

REVIEWER = {"role": "control", "title": "Harita Mühendisi", "name": "A. Yılmaz", "date": "2026-10-20",
            "phrase": "checked", "signature": "wet"}
SEEN = {"role": "approver", "title": "İmar ve Şehircilik Müdürü", "name": "B. Kaya", "date": "2026-10-22",
        "phrase": "seen", "signature": "electronic", "note": "Alarm noktaları için ek ölçü istendi.\nİkinci satır."}
LEVELS = {"warning": {"horizontal": 0.0075, "vertical": 0.012}, "alarm": {"horizontal": 0.03, "vertical": 0.04}}
SOURCE = "İdarenin izleme şartnamesi (2026/14), madde 7"
WITH_LIMITS = {**sc.SERIES, "thresholds": LEVELS}
FOUR = {**sc.SERIES, "measured": "donem4", "design": "donem3", "date": sc.DAYS["donem4"], "trend": {"confidence": 0.95},
        "epochs": [{"layer": k, "date": sc.DAYS[k]} for k in ("donem1", "donem2", "donem3")],
        "thresholds": {"warning": {"horizontal": 0.0075, "horizontalRate": 0.015},
                       "alarm": {"horizontal": 0.03, "horizontalRate": 0.06}}}
TWO = {"kind": "monitoring", "measured": "donem3", "design": "donem2", "horizontal": 0.008}


def setup(fourth=False, work="map", reviews=None, move=None):
    """The series' drawing; `reviews` kept in its settings; `move` (layer, name, dx): a point moved after them."""
    s = sc.setup(work, fourth=fourth)
    if reviews:
        s["settings"]["monitoringReviews"] = reviews
    if move is not None:
        layer, name, dx = move
        e = next(e for e in s["entities"] if e["layerId"] == layer and e["attrs"].get("Ad") == name)
        e["p"] = {"x": e["p"]["x"] + dx, "y": e["p"]["y"]}
    return s


def digest(s, job):
    """The digest of the points `job` reads in the setup `s` (the module's rule)."""
    field = (job.get("field") or "Ad").strip()
    layers = [e["layer"] for e in job.get("epochs", [])] + [job["design"], job["measured"]]
    text = ""
    for layer in layers:
        text += f"k\t{layer}\n"
        for e in s["entities"]:
            if e["kind"] != "point" or e["layerId"] != layer:
                continue
            z = "-" if e.get("z") is None else "%.6f" % e["z"]
            text += "p\t%s\t%.6f\t%.6f\t%s\n" % (e["attrs"].get(field, "").strip(), e["p"]["x"], e["p"]["y"], z)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def job_of(check, source):
    job = {"measured": check["measured"], "design": check["design"]}
    if check.get("epochs"):
        job["epochs"] = [dict(e) for e in check["epochs"]]
    for key in ("date", "field", "control", "horizontal", "vertical"):
        if check.get(key) not in (None, []):
            job[key] = check[key]
    if check.get("trend"):
        job["confidence"] = check["trend"]["confidence"]
    if check.get("thresholds"):
        job["thresholds"] = {**copy.deepcopy(check["thresholds"]), "source": source}
    return job


def marks_of(answer):
    """Each mark as reviewed (the module's rule): the track's last step, else the point's movement."""
    out, seen = [], set()
    if answer.get("tracks"):
        for t in answer["tracks"]:
            last = t["steps"][-1]
            if last is None or t["name"] in seen:
                continue
            seen.add(t["name"])
            m = {"name": t["name"], "horizontal": last["horizontal"]}
            if last.get("dz") is not None:
                m["vertical"] = last["dz"]
            speed = t["fit"]["speed"] if "fit" in t else t.get("rateHorizontal")
            if speed is not None:
                m["speed"] = speed
            if t.get("state"):
                m["state"] = t["state"]
            if t.get("exceeded"):
                m["exceeded"] = t["exceeded"]
            out.append(m)
        return out
    for p in answer["points"]:
        if p.get("horizontal") is None or p["name"] in seen:
            continue
        seen.add(p["name"])
        m = {"name": p["name"], "horizontal": p["horizontal"]}
        if p.get("dh") is not None:
            m["vertical"] = p["dh"]
        out.append(m)
    return out


def record(name, check, reviewer, s, source=None):
    """The review as the project keeps it, made on the setup `s`."""
    job = job_of(check, source)
    return {"name": name, "job": job, "digest": digest(s, job), "marks": marks_of(sc.answer(check)),
            "reviewer": dict(reviewer)}


def held(r):
    """A kept review for the exactly compared settings: its numbers' places held, their values any."""
    r = copy.deepcopy(r)
    r["marks"] = [{k: ("$any" if k in ("horizontal", "vertical", "speed") else v) for k, v in m.items()} for m in r["marks"]]
    return r


def row(r, s):
    states = [m.get("state") for m in r["marks"]]
    return {"name": r["name"], "reviewer": r["reviewer"]["name"], "date": r["reviewer"]["date"],
            "phrase": r["reviewer"]["phrase"], "alarm": states.count("alarm"), "warning": states.count("warning"),
            "current": digest(s, r["job"]) == r["digest"]}


def keep_case(name, check, reviewer=REVIEWER, source=None, before=(), op="execute", s=None, note=None, title="Ekim incelemesi"):
    """Keeping `check` under `title` on the setup `s` (the series' own when none) that keeps `before`."""
    s = copy.deepcopy(s) if s is not None else setup()
    if before:
        s["settings"]["monitoringReviews"] = [copy.deepcopy(r) for r in before]
    r = record(title, check, reviewer, s, source)
    after = [copy.deepcopy(x) for x in before]
    at = next((k for k, x in enumerate(after) if x["name"] == title), None)
    if at is None:
        after.append(r)
    else:
        after[at] = r
    out = {"name": title, "kept": r, "reviews": [row(x, s) for x in after], "revision": "$current"}
    if at is not None:
        out["replaced"] = True
    i = {"name": title, "check": check, "reviewer": reviewer}
    if source is not None:
        i["thresholdsSource"] = source
    return written(name, s, i, out, after, before, op, note)


def written(name, s, i, out, after, before, op, note=None):
    case = {"name": name}
    if note:
        case["note"] = note
    case["setup"] = s
    if op != "execute":
        result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
        case["steps"] = [{"op": op, "input": i, "result": result, "expect": ab.SAME, "within": ab.W}]
        return case
    case["steps"] = [
        {"op": "execute", "input": i, "result": {"status": "completed", "output": out, "warnings": []},
         "expect": {"revision": "changed", "canUndo": True,
                    "settings": {"monitoringReviews": [held(x) for x in after] or None}}, "within": ab.W},
        {"op": "undo", "returns": STEP, "expect": {"settings": {"monitoringReviews": [held(x) for x in before] or None}}},
    ]
    return case


def remove_case(name, title, before, s=None):
    s = copy.deepcopy(s) if s is not None else setup()
    s["settings"]["monitoringReviews"] = [copy.deepcopy(r) for r in before]
    after = [copy.deepcopy(r) for r in before if r["name"] != title]
    out = {"name": title, "removed": True, "reviews": [row(x, s) for x in after], "revision": "$current"}
    return written(name, s, {"name": title, "remove": True}, out, after, before, "execute")


def refused(name, i, code, message, path_, s=None):
    return ab.refused(name, i, code, message, path_, setup_=s)


def cases():
    base = setup()
    first = record("Temmuz incelemesi", WITH_LIMITS, REVIEWER, base, SOURCE)
    other = record("Nisan incelemesi", TWO, SEEN, base)
    four_setup = setup(fourth=True)
    t = {m["name"]: m for m in first["marks"]}
    assert {m.get("state") for m in first["marks"]} >= {"alarm", "warning", "normal"}, t
    two = record("x", TWO, REVIEWER, base)
    assert all("state" not in m and "speed" not in m for m in two["marks"]), two
    # A review made before M1 of the third epoch was moved back 4 mm: the points it read are not today's.
    stale = record("Temmuz incelemesi", WITH_LIMITS, REVIEWER, setup(move=("donem3", "M1", 0.004)), SOURCE)
    assert row(stale, base)["current"] is False and row(first, base)["current"] is True
    lone = {k: v for k, v in WITH_LIMITS.items() if k != "thresholds"}
    keep = lambda **over: {"name": "Ekim incelemesi", "check": WITH_LIMITS, "thresholdsSource": SOURCE,  # noqa: E731
                           "reviewer": REVIEWER, **over}
    drop = lambda d, *keys: {k: v for k, v in d.items() if k not in keys}  # noqa: E731
    no_day = "İncelemenin günü “2026-02-30” YYYY-AA-GG biçiminde bir gün değil."
    return [
        keep_case("üç dönemli izleme eşikleri ve kaynağıyla saklanır: noktaların durumu, iş, özet, inceleyen", WITH_LIMITS,
                  source=SOURCE,
                  note="Kontrol mühendisi “Kontrol edilmiştir” ve ıslak imza; M2 alarmda, iki nokta uyarıda."),
        keep_case("incelemenin planı yazmaz", WITH_LIMITS, source=SOURCE, op="plan"),
        keep_case("incelemenin doğrulaması", WITH_LIMITS, source=SOURCE, op="validate"),
        keep_case("dört dönem, doğru ve hız eşikleriyle: hız doğrununki", FOUR, source=SOURCE, s=four_setup),
        keep_case("dizisiz iki dönem: noktaların hareketi, durum ve hız yok", TWO),
        keep_case("eşiksiz, yalnız doğru: kaynak yazılmaz", {**sc.SERIES, "trend": {"confidence": 0.95}}),
        keep_case("onaylayan “Görülmüştür” ve güvenli e-imzayla, iki satırlı notla", WITH_LIMITS, reviewer=SEEN,
                  source=SOURCE),
        keep_case("aynı ad yenisiyle değişir", WITH_LIMITS, reviewer=SEEN, source=SOURCE, before=[first],
                  title="Temmuz incelemesi"),
        keep_case("ikinci inceleme birincinin yanına; ikisi de güncel", WITH_LIMITS, source=SOURCE, before=[first]),
        keep_case("katman incelemeden sonra değişti: eski inceleme eski sonuca ait, yenisi güncel", WITH_LIMITS,
                  source=SOURCE, before=[stale]),
        keep_case("ad özniteliği açıkça verilir", {**WITH_LIMITS, "field": "Ad"}, source=SOURCE),
        keep_case("Yol projesinde", WITH_LIMITS, source=SOURCE, s=setup(work="road")),
        keep_case("CBS projesinde", WITH_LIMITS, source=SOURCE, s=setup(work="gis")),
        remove_case("inceleme kaldırılır; öbürü kalır", "Temmuz incelemesi", [first, other]),
        remove_case("tek inceleme kaldırılır: proje inceleme tutmaz", "Nisan incelemesi", [other]),
        refused("adın kenarında boşluk", keep(name=" Ekim"), BAD,
                "İncelemenin adı boş, 120 karakterden uzun ya da kenarında boşluk var.", "name"),
        refused("kaldırırken denetim verilmiş", {"name": "Temmuz incelemesi", "remove": True, "check": WITH_LIMITS},
                BAD, "Kaldırırken yalnız incelemenin adını verin; denetim, inceleyen ve kaynak yazılmaz.", "remove",
                s=setup(reviews=[first])),
        refused("kaldırılacak inceleme yok", {"name": "Yok", "remove": True}, "unknown_monitoring_review",
                "Projede “Yok” adıyla saklanan izleme incelemesi yok.", "name", s=setup(reviews=[first])),
        refused("denetim yok", drop(keep(), "check", "thresholdsSource"), BAD,
                "İncelenecek izleme işinin denetimini (check) verin.", "check"),
        refused("inceleyen yok", drop(keep(), "reviewer"), BAD,
                "İnceleyenin rolünü, unvanını, adını, gününü, ibaresini ve imza türünü (reviewer) verin.", "reviewer"),
        refused("izleme işi değil", keep(check=ab.FULL, thresholdsSource=None) | {"thresholdsSource": SOURCE}, BAD,
                "İnceleme kaydı izleme işinindir (check.kind: monitoring).", "check.kind"),
        refused("eşiğin kaynağı yok", drop(keep(), "thresholdsSource"), BAD,
                "Eşiklerin kaynağını yazın: kim, hangi belge ya da kararla, ne zaman belirledi. KentOS eşik vermez.",
                "thresholdsSource"),
        refused("eşiksiz kaynak", keep(check=lone), BAD, "Eşik verilmedi; kaynağı yazılmaz.", "thresholdsSource"),
        refused("kaynak boşlukla başlıyor", keep(thresholdsSource=" şartname"), BAD,
                "Eşiklerin kaynağı boş, 500 karakterden uzun ya da kenarında boşluk var.", "thresholdsSource"),
        refused("inceleme günü takvimde yok", keep(reviewer={**REVIEWER, "date": "2026-02-30"}), BAD, no_day,
                "reviewer"),
        refused("inceleyenin unvanı boş", keep(reviewer={**REVIEWER, "title": ""}), BAD,
                "İnceleyenin unvanı boş, çok uzun ya da kenarında boşluk var.", "reviewer"),
        refused("denetimin reddi geçer: dönemin katmanı yok",
                keep(check={**WITH_LIMITS, "epochs": [{"layer": "yok"}]}), "layer_not_found",
                "“yok” kimlikli katman çizimde yok.", "epochs[0].layer"),
        refused("denetimin reddi geçer: güven 1'den büyük",
                keep(check={**WITH_LIMITS, "trend": {"confidence": 95.0}}), "invalid_asbuilt_check",
                "Güven düzeyi 0 ile 1 arasında bir sayı olmalı (ör. 0.95).", "trend.confidence"),
        refused("sürüm yazısı geçersiz", keep(expectedRevision="abc"), "invalid_revision",
                "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                "ya da komutun planından alın.", "expectedRevision"),
        {"name": "eski sürüme göre hazırlanmış: yazılmaz", "steps": [{
            "op": "execute", "input": keep(expectedRevision="7"),
            "result": {"status": "conflict", "error": {
                "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey "
                "yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
                "revision": "$current"}}, "expect": ab.SAME}]},
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.asbuilt.review", "commandVersion": 1,
           "title": "İzleme incelemesi",
           "note": "ADR 5220. Bağımsız başvuru scripts/fixtures/asbuilt_review_cases.py: çizim ve izleme işinin cevabı "
                   "asbuilt_series_cases.py'nin (elle); işin kaydı, noktaların son hareketi ve durumu, okunan "
                   "noktaların SHA-256 özeti ve güncellik betikte; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        good = OUT.exists() and OUT.read_text("utf-8") == text
        print("İzleme incelemesi ortak durumları güncel." if good else f"{OUT.name} güncel değil.")
        return 0 if good else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.name} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
