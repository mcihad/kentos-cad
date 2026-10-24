"""The shared cases of cad.survey.points (docs/adr/0622 §9, 0070, 0071): Poligon hesabı, Kutupsal alım, Önden and
Geriden kestirme.

    python3 scripts/fixtures/survey_points_cases.py           # writes the file
    python3 scripts/fixtures/survey_points_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.survey.points.json, without KentOS code. The cases are built backwards from points
whose places are known: the observations are what an instrument at the known points would read of them, and the
command must give the points back (within 1e-6 m). The conventions, from the ADRs:

- A bearing is from grid north, clockwise: atan2(Δeast, Δnorth), in [0, a full turn); grad a full turn of 400, degrees
  of 360.
- Poligon hesabı: the angle at a point is clockwise from the line back (to the back point, then to the point before)
  to the line ahead; a leg's bearing is the one back plus the angle. A connected traverse ends on its end point, its
  angle there towards the fore point; a closed one comes back to its start, its last angle towards the back point.
  The points are the legs' ends, a known end not among them; named in order, P1 … where none is given. Without
  misclosure the points are where they were.
- Kutupsal alım: the readings oriented on the back point read at its reading: a point's bearing is the back's bearing
  less the back reading plus its reading. With a zenith angle z the horizontal length is the slope length times
  sin z and the height difference the slope length times cos z plus (1 − k)·h²/(2·6 371 000) plus the instrument's
  height less the target's (k the project's, 0.13); the point's height the station's plus that.
- Önden kestirme: at A the angle α runs clockwise from B to the new point, at B the angle β clockwise from the new point
  to A. Geriden kestirme: at the new point α runs clockwise from A to B, β from B to C.
- Each point is written named (its label and `Ad`), with its kind as `Tür` (Poligon noktası, Alım noktası, Kestirme
  noktası) and its height as z and `Z (m)` (three decimals); one undo step named after the window.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
FILE = OUT / "cad.survey.points.json"
POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000
WITHIN = 1e-6
TURN = {"grad": 400.0, "deg": 360.0}
K = 0.13
R = 6371000.0


def W(p):
    return {"x": X + p[0], "y": Y + p[1]}


def point(id_, name, p, layer="yapi"):
    return {"kind": "point", "id": id_, "layerId": layer, "attrs": {"Ad": name}, "label": name, "p": W(p)}


KNOWN = {"A": (0, 0), "R": (0, 100), "E": (300, 60), "F": (400, 60), "B": (200, 0), "C": (100, 200)}
ENTITIES = [point(i + 1, n, p) for i, (n, p) in enumerate(KNOWN.items())]
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k != "entities"}, "entities": ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}


def bearing(a, b, unit="grad"):
    t = math.atan2(b[0] - a[0], b[1] - a[1]) % (2 * math.pi)
    return t * TURN[unit] / (2 * math.pi)


def turn_of(v, unit="grad"):
    return v % TURN[unit]


def dist(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def named(n):
    return {"kind": "named", "name": n}


def at(p):
    return {"kind": "at", "at": W(p)}


def written(name, p, kind, z=None):
    f = {"kind": "point", "label": name, "p": W(p), "attrs": {"Ad": name, "Tür": kind}}
    if z is not None:
        f["z"] = z
        f["attrs"]["Z (m)"] = f"{z:.3f}"
    return f


def done(name, work, points, undo, extra=None, misclosure=None):
    inp = {"layerId": "yapi", "work": work, **(extra or {})}
    n = len(points)
    output = {"ids": [NEXT + i for i in range(n)], "uids": [f"$uidOf:{NEXT + i}" for i in range(n)],
              "revision": "$current"}
    if misclosure is not None:
        output.update(misclosure)
    step = {"op": "execute", "input": inp, "within": WITHIN,
            "result": {"status": "completed", "output": output, "warnings": []},
            "expect": {"ids": IDS + [NEXT + i for i in range(n)],
                       "fields": {str(NEXT + i): f for i, f in enumerate(points)}, "canUndo": True, "dirty": True}}
    back = {"op": "undo", "returns": undo, "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def refused(name, work, code, message, path, extra=None, non_finite=None):
    step = {"op": "execute", "input": {"layerId": "yapi", "work": work, **(extra or {})}}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


# ── Poligon hesabı ───────────────────────────────────────────────────────────────────────────────────────────────


def traverse_obs(start, back, path, fore=None, unit="grad"):
    """The angles and lengths of a traverse through `path` (start first, its last the end), oriented on `back` at the
    start and on `fore` at the end when given."""
    angles, lengths = [], []
    prev = back
    for i in range(len(path) - 1):
        here, ahead = path[i], path[i + 1]
        angles.append(turn_of(bearing(here, ahead, unit) - bearing(here, prev, unit), unit))
        lengths.append(dist(here, ahead))
        prev = here
    if fore is not None:
        end = path[-1]
        angles.append(turn_of(bearing(end, fore, unit) - bearing(end, path[-2], unit), unit))
    return angles, lengths


def traverse_cases():
    A, Rp, E, F = KNOWN["A"], KNOWN["R"], KNOWN["E"], KNOWN["F"]
    p1, p2 = (90, 40), (190, 70)
    ang, ln = traverse_obs(A, Rp, [A, p1, p2, E], fore=F)
    connected = {"kind": "traverse", "shape": "connected", "start": named("A"), "back": named("R"), "end": named("E"),
                 "fore": named("F"), "angles": ang, "distances": ln, "names": ["P101", "P102"]}
    q1, q2, q3 = (80, -60), (160, 10), (90, 90)
    cang, cln = traverse_obs(A, Rp, [A, q1, q2, q3, A], fore=Rp)
    closed = {"kind": "traverse", "shape": "closed", "start": named("A"), "back": named("R"), "angles": cang,
              "distances": cln, "names": ["K1", "K2", "K3"]}
    o1, o2 = (-50, 40), (-120, 30)
    oang, oln = traverse_obs(A, Rp, [A, o1, o2])
    opened = {"kind": "traverse", "shape": "open", "start": named("A"), "back": named("R"), "angles": oang,
              "distances": oln}
    dang, dln = traverse_obs(A, Rp, [A, p1, p2, E], fore=F, unit="deg")
    deg = {**connected, "angles": dang, "distances": dln}
    unoriented = {k: v for k, v in connected.items() if k != "fore"}
    unoriented["angles"] = ang[:-1]
    zero = {"angleMisclosure": 0, "linearMisclosure": 0}
    return [
        done("Poligon hesabı: bağlı, iki uçta yöneltilmiş", connected,
             [written("P101", p1, "Poligon noktası"), written("P102", p2, "Poligon noktası")], "Poligon hesabı",
             misclosure=zero),
        done("Poligon hesabı: kapalı, başlangıca dönüyor", closed,
             [written("K1", q1, "Poligon noktası"), written("K2", q2, "Poligon noktası"),
              written("K3", q3, "Poligon noktası")], "Poligon hesabı", misclosure=zero),
        done("Poligon hesabı: açık, adsız noktalar P1, P2", opened,
             [written("P1", o1, "Poligon noktası"), written("P2", o2, "Poligon noktası")], "Poligon hesabı"),
        done("Poligon hesabı: derecede", deg,
             [written("P101", p1, "Poligon noktası"), written("P102", p2, "Poligon noktası")], "Poligon hesabı",
             extra={"unit": "deg"}, misclosure=zero),
        done("Poligon hesabı: bitişte yöneltmesiz", unoriented,
             [written("P101", p1, "Poligon noktası"), written("P102", p2, "Poligon noktası")], "Poligon hesabı",
             misclosure={"linearMisclosure": 0}),
        done("Poligon hesabı: bilinen noktalar koordinatla", {**connected, "start": at(A), "back": at(Rp),
                                                              "end": at(E), "fore": at(F)},
             [written("P101", p1, "Poligon noktası"), written("P102", p2, "Poligon noktası")], "Poligon hesabı",
             misclosure=zero),
        refused("Poligon hesabı: olmayan nokta", {**connected, "start": named("Z9")}, "point_not_found",
                "“Z9” adlı nokta çizimde yok. Adını denetleyin ya da koordinatlarını verin.", "work.start.name"),
        refused("Poligon hesabı: bağlı poligonun bitişi yok", {k: v for k, v in connected.items() if k != "end"},
                "survey_refused", "Bağlı poligonun bitiş noktasını verin.", "work.end"),
        refused("Poligon hesabı: açı sayısı uymuyor", {**connected, "angles": ang[:2]}, "survey_refused",
                "3 kenarlı bu poligonda 4 kırılma açısı olmalı, 2 verildi.", "work"),
        refused("Poligon hesabı: kenar sıfır", {**opened, "distances": [oln[0], 0]}, "survey_refused",
                "2. kenarın uzunluğu sıfırdan büyük olmalı.", "work"),
        refused("Poligon hesabı: başlangıç ve bakılan aynı", {**opened, "back": named("a")}, "survey_refused",
                "Başlangıç noktası ile başlangıçta bakılan nokta aynı yerde; doğrultu tanımsız.", "work"),
        refused("Poligon hesabı: açı sonlu değil", connected, "not_finite",
                "Açı sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", "work.angles[1]",
                non_finite={"work.angles[1]": "NaN"}),
        refused("Poligon hesabı: kilitli katmana yazılmaz", connected, "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
                "etkinleştirin.", "layerId", extra={"layerId": "kilitli"}),
    ]


# ── Kutupsal alım ────────────────────────────────────────────────────────────────────────────────────────────────


def polar_cases():
    A, Rp = KNOWN["A"], KNOWN["R"]
    t1, t2, t3 = (40, 30), (-25, 60), (70, -45)
    back_reading = 50.0

    def reading(t):
        return turn_of(bearing(A, t) - bearing(A, Rp) + back_reading)

    shots = [{"name": n, "reading": reading(t), "distance": dist(A, t)} for n, t in (("T1", t1), ("T2", t2), ("T3", t3))]
    work = {"kind": "polar", "station": named("A"), "back": named("R"), "backReading": back_reading, "shots": shots}
    # A shot with a zenith angle: slope length s and z chosen, the point where its horizontal length puts it.
    z_g, slope = 98.0, 60.0
    z = math.radians(z_g * 0.9)
    horizontal = slope * math.sin(z)
    u = math.atan2(t1[0] - A[0], t1[1] - A[1])
    tz = (A[0] + horizontal * math.sin(u), A[1] + horizontal * math.cos(u))
    station_z, i_h, t_h = 100.0, 1.5, 1.8
    dz = slope * math.cos(z) + (1 - K) * horizontal ** 2 / (2 * R) + i_h - t_h
    zwork = {"kind": "polar", "station": named("A"), "back": named("R"), "backReading": back_reading,
             "stationZ": station_z, "instrumentHeight": i_h,
             "shots": [{"name": "H1", "reading": reading(t1), "distance": slope, "zenith": z_g, "targetHeight": t_h}]}
    return [
        done("Kutupsal alım: üç nokta, bakılan noktada 50g okuma", work,
             [written("T1", t1, "Alım noktası"), written("T2", t2, "Alım noktası"), written("T3", t3, "Alım noktası")],
             "Kutupsal alım"),
        done("Kutupsal alım: başucu açısıyla eğik uzunluk ve kot", zwork,
             [written("H1", tz, "Alım noktası", z=station_z + dz)], "Kutupsal alım"),
        refused("Kutupsal alım: alım yok", {**work, "shots": []}, "survey_refused", "En az bir alım verin.",
                "work.shots"),
        refused("Kutupsal alım: uzunluk sıfır", {**work, "shots": [{"name": "T1", "reading": 10, "distance": 0}]},
                "survey_refused", "1. noktanın uzunluğu sıfırdan büyük olmalı.", "work"),
        refused("Kutupsal alım: olmayan bakılan nokta", {**work, "back": named("Yok")}, "point_not_found",
                "“Yok” adlı nokta çizimde yok. Adını denetleyin ya da koordinatlarını verin.", "work.back.name"),
    ]


# ── Kestirmeler ──────────────────────────────────────────────────────────────────────────────────────────────────


def intersection_cases():
    A, B, C = KNOWN["A"], KNOWN["B"], KNOWN["C"]
    P = (110, -70)  # right of A → B
    alpha = turn_of(bearing(A, P) - bearing(A, B))
    beta = turn_of(bearing(B, A) - bearing(B, P))
    Q = (60, 80)
    ra = turn_of(bearing(Q, B) - bearing(Q, A))
    rb = turn_of(bearing(Q, C) - bearing(Q, B))
    return [
        done("Önden kestirme: A ve B'den", {"kind": "forward", "a": named("A"), "b": named("B"), "alpha": alpha,
                                             "beta": beta, "name": "Ö1"},
             [written("Ö1", P, "Kestirme noktası")], "Önden kestirme"),
        done("Geriden kestirme: üç bilinen noktadan", {"kind": "resection", "a": named("A"), "b": named("B"),
                                                         "c": named("C"), "alpha": ra, "beta": rb, "name": "G1"},
             [written("G1", Q, "Kestirme noktası")], "Geriden kestirme"),
        refused("Önden kestirme: açılar üçgen kurmuyor", {"kind": "forward", "a": named("A"), "b": named("B"),
                                                          "alpha": 150, "beta": 100, "name": "Ö2"}, "survey_refused",
                "Açılar üçgen kurmuyor: ikisi de sıfırdan büyük, toplamları yarım turdan küçük olmalı.", "work"),
        refused("Geriden kestirme: noktalar aynı doğrultuda", {"kind": "resection", "a": named("A"), "b": named("B"),
                                                               "c": named("C"), "alpha": 0, "beta": rb, "name": "G2"},
                "survey_refused", "Açılar sıfır ya da yarım tur olamaz: noktalar aynı doğrultuda görünüyor.", "work"),
    ]


def general_cases():
    A = KNOWN["A"]
    fw = {"kind": "forward", "a": named("A"), "b": named("B"), "alpha": 50, "beta": 50, "name": "Ö5"}
    hidden = done("Gizli katmana uyarıyla", {**fw, "name": "Ö6"}, [], "Önden kestirme")
    hidden["steps"][0]["input"]["layerId"] = "gizli"
    hidden["steps"][0]["result"]["output"] = "$any"
    hidden["steps"][0]["result"]["warnings"] = [
        {"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}]
    hidden["steps"][0]["expect"] = {"dirty": True}
    return [
        hidden,
        refused("Olmayan katman", fw, "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId",
                extra={"layerId": "yok"}),
        refused("Katman değil grup", fw, "not_a_layer",
                "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId",
                extra={"layerId": "pafta"}),
        refused("Sürüm yazısı geçersiz", fw, "invalid_revision",
                "Beklenen sürüm “012” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                "ya da komutun planından alın.", "expectedRevision", extra={"expectedRevision": "012"}),
        {"name": "Sürüm değiştiyse hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "base"},
            {"op": "execute", "input": {"layerId": "yapi", "work": fw},
             "result": {"status": "completed", "output": "$any", "warnings": []}},
            {"op": "execute", "input": {"layerId": "yapi", "work": {**fw, "name": "Ö7"}, "expectedRevision": "$base"},
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin "
                            "şimdiki hâline göre yeniden hazırlayın.",
                 "path": "expectedRevision", "revision": "$current"}},
             "expect": {"revision": "same"}}]},
        refused("Bilinen nokta sonlu değil", {"kind": "forward", "a": at(A), "b": named("B"), "alpha": 50, "beta": 50,
                                              "name": "Ö3"}, "not_finite",
                "Bilinen noktanın doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir "
                "sayıyla verin.", "work.a.at.x", non_finite={"work.a.at.x": "NaN"}),
        {"name": "Plan: yazılmaz", "steps": [
            {"op": "plan", "input": {"layerId": "yapi", "work": {"kind": "forward", "a": named("A"), "b": named("B"),
                                                                  "alpha": 50, "beta": 50, "name": "Ö4"}},
             "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": UNCHANGED}]},
    ]


def build():
    cases = traverse_cases() + polar_cases() + intersection_cases() + general_cases()
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.survey.points", "commandVersion": 1,
            "title": "Ölçme hesabı",
            "note": "Written by scripts/fixtures/survey_points_cases.py from the windows' conventions (docs/adr/0622 §9, "
                    "0070, 0071); do not edit by hand.",
            "setup": SETUP, "cases": cases}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not FILE.exists() or FILE.read_text(encoding="utf-8") != text:
            print("cad.survey.points durumları güncel değil; betiği --check'siz çalıştırın.")
            sys.exit(1)
        print(f"cad.survey.points durumları güncel: {len(build()['cases'])} durum.")
        return
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
