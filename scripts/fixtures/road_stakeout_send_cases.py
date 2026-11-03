#!/usr/bin/env python3
"""The shared cases of `road.stakeout.send` (plan/road.md Y10.4, docs/adr/0908 §9; ROAD-IO-13).

The stakeout table's points by road_stakeout_cases.py's reference (no KentOS code), sent: to an instrument's
coordinate file by field_write_cases.py's writers (the text, its bytes and SHA-256 here), named by number from `first`,
a code the format cannot carry left off its points; or as a LandXML string road: the corridor's alignment and profile
and each code's points as strings (docs/adr/0908 §9's rule written here again), the text's bytes and digest aside.
`--written` reads the string road KentOS wrote (`KENTOS_WRITE_LANDXML=1 cargo test -p kentos-native-application
--test all stakeout_send`): xmllint checks it against the LandXML 1.2 schema and its strings are compared with these.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_stakeout_send_cases.py [--check | --written]
"""

import copy
import hashlib
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import field_write_cases as fw  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_stakeout_cases as sref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.stakeout.send.json"
XSD = ROOT / "fixtures/road/v1/landxml/LandXML-1.2.xsd"
WRITTEN = ROOT / "fixtures/road/v1/landxml/out/aplikasyon.xml"
NS = "{http://www.landxml.org/schema/LandXML-1.2}"

completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
STAMP = "2026-11-03T09:30:00"
MOST_FIRST = 99_999_999
FORMATS = {"landxml": ("xml", "LandXML"), "gsi16": ("gsi", "Leica GSI-16"), "gsi8": ("gsi", "Leica GSI-8"),
           "gts7": ("xyz", "Topcon GTS-7"), "jobxml": ("jxl", "Trimble JobXML"), "nikon": ("raw", "Nikon RAW"),
           "csv": ("csv", "CSV")}


def bad(path, message):
    return failed("invalid_stakeout_send", message, path or None)


def losses(message):
    return {"code": "export_losses", "message": message, "path": "format"}


def strings(rows):
    """Each code's strings: a string follows the table's sections ((station, region) in order) and breaks at one
    without the code; a point equal to the one before is taken once; one-point strings are counted, not kept."""
    keys, at = [], []
    for r in rows:
        k = (r["station"], r["region"])
        if not keys or keys[-1] != k:
            keys.append(k)
        at.append(len(keys) - 1)
    order = []
    for r in rows:
        if r["code"] not in order:
            order.append(r["code"])
    out, short = [], 0
    for code in order:
        runs, last = [], None
        for r, k in zip(rows, at):
            if r["code"] != code:
                continue
            p = [r["x"], r["y"], r["z"]]
            if last is not None and runs and last + 1 >= k:
                if not all(abs(p[i] - runs[-1][-1][i]) <= 1e-9 for i in range(3)):
                    runs[-1].append(p)
            else:
                runs.append([p])
            last = k
        long = [r for r in runs if len(r) >= 2]
        short += len(runs) - len(long)
        for i, pts in enumerate(long):
            out.append({"name": f"{code} ({i + 1})" if len(long) > 1 else code, "points": pts})
    return out, short


def sha(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def answer(input_):
    """The result and the warnings of an `execute` (the file's content aside), as KentOS answers it."""
    q = {k: input_[k] for k in ("corridor", "from", "to", "step", "codes") if k in input_}
    if not any(c["id"] == input_["corridor"] or c["name"] == input_["corridor"] for c in sref.CORRIDORS):
        return failed("unknown_corridor", f"“{input_['corridor']}” kimlikli ya da adlı koridor projede yok.",
                      "corridor"), []
    try:
        table, warnings = sref.table(sref.by(input_["corridor"]), q)
    except sref.Refused as r:
        return r.result, []
    fmt = input_["format"]
    ext, name = FORMATS[fmt]
    path = input_["path"].strip()
    if not path or not (path.startswith("/") or path.startswith("$")):
        return failed("invalid_file_path", f"Dosyanın yolu boş ya da göreli; mutlak bir .{ext} yolu verin.", "path"), []
    if not path.lower().endswith(f".{ext}"):
        return failed("invalid_file_path", f"“{path}” bir .{ext} dosyası değil ({name}).", "path"), []
    if ":yok/" in path:
        folder = path.rsplit("/", 1)[0]
        return failed("invalid_file_path", f"“{folder}” klasörü yok; dosya var olan bir klasöre yazılır.", "path"), []
    if path.startswith("$fixture:") and not input_.get("overwrite"):
        return failed("file_exists", f"{path} var; üzerine yazmak için overwrite verin.", "path"), []
    first = input_.get("first", 1)
    if not 1 <= first <= MOST_FIRST:
        return bad("first", f"İlk nokta numarası 1 ile {MOST_FIRST} arası olmalı."), []
    stamp = input_.get("stamp")
    if stamp is not None and not sref_good_stamp(stamp):
        return bad("stamp", f"“{stamp}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın."), []
    rows = table["rows"]
    if not rows:
        return bad("", "Gönderilecek nokta yok: aralıkta ya da kodlarda kesit noktası çıkmadı."), []
    out = {"path": path, "format": fmt, "corridor": table["corridor"], "name": table["name"],
           "stations": table["stations"], "points": 0, "left": [], "codesDropped": 0, "strings": 0}
    if fmt == "landxml":
        c = sref.by(input_["corridor"])
        features, short = strings(rows)
        if short:
            warnings.append(losses(f"{short} dizi tek noktalı (o kodun komşu km'lerde noktası yok); string olarak "
                                   "yazılmadı."))
        if not features:
            return bad("", "Gönderilecek dizi yok: hiçbir kodun iki ardışık km'de noktası yok."), []
        alignment = next(a for a in ccmd.road_of(ccmd.START)["alignments"] if a["id"] == c["alignment"])
        profile = next(p for p in ccmd.PROFILES if p["id"] == c["profile"])
        out.update(points=sum(len(f["points"]) for f in features), strings=len(features),
                   alignment=alignment["name"], profile=profile["name"], bytes="$any", sha256="$any")
        return completed(out), warnings
    stamp = stamp or STAMP
    job = (input_.get("job") or "").strip() or None
    fits, dropped, points = {}, [], []
    for i, r in enumerate(rows):
        if r["code"] not in fits:
            probe = {"name": "1", "east": 0.0, "north": 0.0, "elevation": 0.0, "code": r["code"]}
            fits[r["code"]] = not fw.write([probe], {"format": fmt, "job": "KentOS", "stamp": stamp})["skipped"]
            if not fits[r["code"]]:
                dropped.append(r["code"])
        if not fits[r["code"]]:
            out["codesDropped"] += 1
        points.append({"name": str(first + i), "east": r["x"], "north": r["y"], "elevation": r["z"],
                       "code": r["code"] if fits[r["code"]] else None})
    written = fw.write(points, {"format": fmt, "job": job or table["name"], "stamp": stamp})
    if "error" in written and job is None:
        written = fw.write(points, {"format": fmt, "job": "KentOS", "stamp": stamp})
    if "error" in written:
        return bad("job", written["error"]), []
    if dropped:
        warnings.append({"code": "stakeout_code_dropped", "path": "format",
                         "message": f"{name} şu kodları taşıyamıyor; {out['codesDropped']} noktanın kodu yazılmadı: "
                                    f"{', '.join(dropped)}."})
    out["left"] = [{"name": s["name"], "problem": s["problem"]} for s in written["skipped"]]
    if written["written"] == 0:
        first_why = f" İlki: {out['left'][0]['problem']}" if out["left"] else ""
        return bad("format", f"{name} dosyasına yazılabilen nokta yok.{first_why}"), []
    if out["left"]:
        warnings.append(losses(f"{len(out['left'])} nokta {name} dosyasında taşınamıyor; yazılmadı."))
    out["points"] = written["written"]
    text = written["text"]
    out["bytes"] = len(text.encode("utf-8"))
    out["sha256"] = sha(text)
    return completed(out), warnings


def sref_good_stamp(t):
    if "T" not in t:
        return False
    d, h = t.split("T", 1)
    ok = len(d) == 10 and d[4] == "-" and d[7] == "-" and len(h) == 8 and h[2] == ":" and h[5] == ":"
    return ok and all(ch.isdigit() for ch in (d + h).replace("-", "").replace(":", ""))


def ok(name, input_, op="execute"):
    result, warnings = answer(input_)
    assert result["status"] == "completed", (name, result)
    if op == "validate":
        result = completed(None)
    result["warnings"] = warnings
    s = step(op, input_, result, UNCHANGED)
    s["within"] = 1e-6
    return case(name, [s])


def no(name, input_, setup_=None):
    result, _ = answer(input_)
    assert result["status"] == "failed", (name, result)
    return case(name, [step("execute", input_, result, UNCHANGED)], setup_)


def cases():
    names = iter(range(1, 100))

    def tmp(ext):
        return f"$tmp:aplikasyon-{next(names)}.{ext}"
    there = "$fixture:road/v1/landxml/var.xml"
    base = {"corridor": "Ana koridor", "from": 100.0, "to": 200.0, "step": 20.0, "stamp": STAMP}
    return [
        ok("String yol: ana koridorun km 100–200'ü her 20 m, bütün kodlar", {**base, "format": "landxml", "path": tmp("xml")}),
        ok("String yol: bölge sınırı km 300'de, yalnız şev noktaları",
           {**base, "from": 260.0, "to": 340.0, "codes": ["hinge", "daylight"], "format": "landxml", "path": tmp("xml")}),
        ok("String yol: yan koridor, deverli, kendi boykesitiyle",
           {"corridor": "Yan koridor", "from": 100.0, "to": 200.0, "step": 25.0, "format": "landxml",
            "path": tmp("xml"), "stamp": STAMP}),
        no("String yol: tek km, hiçbir dizi iki noktalı değil", {**base, "from": 150.0, "to": 150.0,
                                                                "format": "landxml", "path": tmp("xml")}),
        ok("String yol: planı, var olan dosyanın üstüne", {**base, "format": "landxml", "path": there,
                                                          "overwrite": True}, op="plan"),
        ok("Leica GSI-16: kodlar ve noktalar", {**base, "format": "gsi16", "path": tmp("gsi")}),
        ok("Leica GSI-8: uzun kodlar düşer", {**base, "format": "gsi8", "path": tmp("gsi")}),
        ok("Leica GSI-8: dokuz haneli numaralar yazılmaz",
           {**base, "codes": ["axis", "daylight"], "format": "gsi8", "path": tmp("gsi"), "first": 99_999_995}),
        ok("Topcon GTS-7: şev ayakları 1001'den", {**base, "codes": ["daylight"], "format": "gts7", "path": tmp("xyz"),
                                                    "first": 1001}),
        ok("Trimble JobXML: iş adıyla", {**base, "codes": ["axis", "hinge"], "format": "jobxml", "path": tmp("jxl"),
                                          "job": "Ana yol aplikasyon"}),
        ok("Trimble JobXML: iş adı koridorun adı", {**base, "format": "jobxml", "path": tmp("jxl")}),
        ok("Nikon RAW: yan koridor", {"corridor": "c-2", "from": 300.0, "to": 400.0, "step": 50.0, "format": "nikon",
                                      "path": tmp("raw"), "stamp": STAMP}),
        ok("CSV: koridorun kendi km'leri", {"corridor": "Ana koridor", "from": 100.0, "to": 160.0, "format": "csv",
                                             "path": tmp("csv"), "stamp": STAMP}),
        ok("Yalnız doğrulama", {**base, "format": "csv", "path": tmp("csv")}, op="validate"),
        ok("Saatsiz CSV", {"corridor": "Ana koridor", "from": 100.0, "to": 140.0, "step": 20.0, "format": "csv",
                           "path": tmp("csv")}),
        case("Yol projesi değil", [step("execute", {**base, "format": "csv", "path": tmp("csv")},
                                        failed("not_a_road_project", ccmd.NOT_ROAD["error"]["message"]), UNCHANGED)],
             sref.setup(work="map")),
        no("Olmayan koridor", {**base, "corridor": "yok", "format": "csv", "path": tmp("csv")}),
        no("Aralık koridorun dışında", {**base, "to": 900.0, "format": "csv", "path": tmp("csv")}),
        no("Boş yol", {**base, "format": "gsi16", "path": " "}),
        no("Göreli yol", {**base, "format": "gsi16", "path": "aplikasyon.gsi"}),
        no("Biçimin uzantısı değil", {**base, "format": "jobxml", "path": "$tmp:aplikasyon.xml"}),
        no("Klasörü yok", {**base, "format": "csv", "path": "$tmp:yok/aplikasyon.csv"}),
        no("Dosya var", {**base, "format": "landxml", "path": there}),
        no("İlk numara sıfır", {**base, "format": "csv", "path": tmp("csv"), "first": 0}),
        no("İlk numara çok büyük", {**base, "format": "csv", "path": tmp("csv"), "first": 100_000_000}),
        no("Okunmayan saat", {**base, "format": "csv", "path": tmp("csv"), "stamp": "3 Kasım"}),
        no("Yüzeysiz koridor: nokta yok", {"corridor": "Yüzeysiz koridor", "step": 10.0, "format": "csv",
                                          "path": tmp("csv")}),
        no("Nikon RAW: ASCII olmayan iş adı", {**base, "format": "nikon", "path": tmp("raw"), "job": "Şantiye"}),
        no("Trimble JobXML: iş adında denetim karakteri", {**base, "format": "jobxml", "path": tmp("jxl"),
                                                          "job": "Ana\nyol"}),
    ]


# ── The written string road ───────────────────────────────────────────────────────────────────────────────────────


def check_written():
    subprocess.run(["xmllint", "--noout", "--schema", str(XSD), str(WRITTEN)], check=True)
    root = ET.parse(WRITTEN).getroot()
    q = {"corridor": "Ana koridor", "from": 100.0, "to": 200.0, "step": 20.0}
    table, _ = sref.table(sref.by("Ana koridor"), q)
    want, _ = strings(table["rows"])
    plans = root.findall(f"{NS}PlanFeatures")
    assert [p.get("name") for p in plans] == ["Ana koridor"], [p.get("name") for p in plans]
    got = plans[0].findall(f"{NS}PlanFeature")
    assert [f.get("name") for f in got] == [f["name"] for f in want], [f.get("name") for f in got]
    for f, w in zip(got, want):
        lines = f.findall(f"{NS}CoordGeom/{NS}Line")
        assert len(lines) == len(w["points"]) - 1, f.get("name")
        for i, line in enumerate(lines):
            for tag, p in (("Start", w["points"][i]), ("End", w["points"][i + 1])):
                n, e, z = [float(v) for v in line.find(f"{NS}{tag}").text.split()]
                for a, b in ((e, p[0]), (n, p[1]), (z, p[2])):
                    if abs(a - b) > 1e-6:
                        raise SystemExit(f"{f.get('name')}: {a} ≠ {b}")
    aligns = root.findall(f"{NS}Alignments/{NS}Alignment")
    assert [a.get("name") for a in aligns] == ["Ana aks"], [a.get("name") for a in aligns]
    profs = aligns[0].findall(f"{NS}Profile")
    want_profile = next(p for p in ccmd.PROFILES if p["id"] == "b-1")["name"]
    assert [p.get("name") for p in profs] == [want_profile], [p.get("name") for p in profs]
    print(f"{WRITTEN.name}: şemaya uyuyor, {len(got)} dizi ve aks başvuruyla aynı")


def main():
    if "--written" in sys.argv:
        check_written()
        return
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.stakeout.send", "commandVersion": 1,
            "title": "Aplikasyonu alete gönder",
            "note": "scripts/fixtures/road_stakeout_send_cases.py yazar; noktalar road_stakeout_cases.py'nin "
                    "başvurusundan, alet dosyaları field_write_cases.py'nin yazıcılarıyla (metin, boy ve SHA-256), "
                    "string yolun dizileri ADR 0908 §9'un kuralıyla; KentOS kodu yok. String yolun içeriği "
                    "--written ile xmllint ve başvuruyla denetlenir.",
            "setup": sref.setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
