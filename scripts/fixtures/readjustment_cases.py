#!/usr/bin/env python3
"""The shared cases of `cad.readjustment.distribute` (Düzenleme dağıtım taslağı; plan/map-cad.md MAP-11,
docs/adr/1460), without KentOS code: exact fractions and rounding by hand.

    python3 scripts/fixtures/readjustment_cases.py           # writes fixtures/commands/v1/cad.readjustment.distribute.json
    python3 scripts/fixtures/readjustment_cases.py --check   # writes nothing; compares

The rules (docs/adr/1460 §2):
- the deduction's exact total Σ area × ratio is rounded once to the policy's places by its mode;
- it is split over the parcels in proportion to their areas, each part rounded, the remainder (a whole number of
  units, else refused) given one unit at a time to the parts furthest short (ties: the records' order), or refused
  when the rule is `reject`;
- a parcel's net is its area minus its deduction; it is split over its holders by their exact shares the same way;
- the totals: registered = Σ deductions + Σ nets, Σ holders' values = Σ nets, Σ deductions = the rounded total.
"""

import copy
import json
import re
import sys
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.readjustment.distribute.json"
PLAIN = re.compile(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?$")


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


def frac(text):
    return Fraction(Decimal(text))


def places(text):
    return len(text.split(".")[1]) if "." in text else 0


def text(f, scale):
    """A fraction that is a multiple of 10^-scale, written with exactly `scale` places."""
    n = f * 10 ** scale
    assert n.denominator == 1
    q = n.numerator
    sign = "-" if q < 0 else ""
    q = abs(q)
    if scale == 0:
        return f"{sign}{q}"
    s = str(q).rjust(scale + 1, "0")
    return f"{sign}{s[:-scale]}.{s[-scale:]}"


def normal(f):
    """A decimal fraction written without trailing zeros (rust_decimal's normalize)."""
    s = 0
    while (f * 10 ** s).denominator != 1:
        s += 1
    t = text(f, s)
    return "0" if f == 0 else t


def round_to(f, scale, mode):
    n = f * 10 ** scale
    q, r = divmod(n.numerator, n.denominator)  # floor
    if r == 0:
        return Fraction(q, 10 ** scale)
    # q is the floor; the value lies between q and q + 1.
    frac_part = Fraction(r, n.denominator)
    neg = n < 0
    toward_zero = q + 1 if neg else q
    away = q if neg else q + 1
    half = frac_part if not neg else 1 - frac_part  # distance from the zero side, as a share of one unit
    if mode == "away_from_zero":
        v = away
    elif mode == "toward_zero":
        v = toward_zero
    elif mode == "toward_positive":
        v = q + 1
    elif mode == "toward_negative":
        v = q
    elif mode == "half_away_from_zero":
        v = away if half >= Fraction(1, 2) else toward_zero
    elif mode == "half_toward_zero":
        v = away if half > Fraction(1, 2) else toward_zero
    else:  # half_even
        if half > Fraction(1, 2):
            v = away
        elif half < Fraction(1, 2):
            v = toward_zero
        else:
            v = q if q % 2 == 0 else q + 1
    return Fraction(v, 10 ** scale)


def split(total, weights, scale, mode, rule):
    wsum = sum(weights, Fraction(0))
    exact = [total * w / wsum for w in weights]
    parts = [round_to(e, scale, mode) for e in exact]
    diff = total - sum(parts, Fraction(0))
    steps = diff * 10 ** scale
    assert steps.denominator == 1
    steps = steps.numerator
    if steps == 0:
        return parts, "0", []
    remainder = normal(round_to(diff, scale, "half_even"))
    if rule == "reject":
        raise Refused("needs_rule", f"Yuvarlanan değerler toplamı {remainder} kadar tutmuyor ve kural paketinin artık "
                                    "kuralı “reject”; hiçbir şey dağıtılmadı. Artık kuralı onaylı bir pakette verin.",
                      None)
    sign = 1 if steps > 0 else -1
    order = sorted(range(len(parts)), key=lambda i: (-(sign * (exact[i] - parts[i])), i))
    given = []
    for k in range(abs(steps)):
        i = order[k % len(order)]
        parts[i] += Fraction(sign, 10 ** scale)
        given.append(i)
    return parts, remainder, given


MODES = {"half_even", "half_away_from_zero", "half_toward_zero", "away_from_zero", "toward_zero", "toward_positive",
         "toward_negative"}


def run(inp):
    r = inp["rules"]
    for k, what in (("id", "kimliği"), ("source", "kaynak belgesi"), ("jurisdiction", "kurumu")):
        if not r[k].strip():
            raise Refused("invalid_input", f"Kural paketinin {what} boş; kuralın dayanağı yazılmadan dağıtım yapılmaz.",
                          f"rules.{k}")
    d = r["deduction"].strip()
    if not PLAIN.match(d) or d.startswith("-") or frac(d) >= 1:
        raise Refused("invalid_input", f"Kesinti oranı “{r['deduction']}” 0 ile 1 arasında (1 hariç) bir ondalık yazı "
                                       "olmalı.", "rules.deduction")
    if r["scale"] > 6:
        raise Refused("invalid_input", f"Basamak {r['scale']} en çok 6 olabilir.", "rules.scale")
    if r["rounding"].strip() not in MODES:
        raise Refused("invalid_input", f"“{r['rounding']}” bir yuvarlama yönü değil (half_even, half_away_from_zero …).",
                      "rules.rounding")
    if r["remainder"].strip() not in ("reject", "largest_remainder"):
        raise Refused("invalid_input", f"“{r['remainder']}” bir artık kuralı değil (reject, largest_remainder).",
                      "rules.remainder")
    if not inp["entries"]:
        raise Refused("invalid_input", "Kayıt yok: parsel, hak sahibi, tapu alanı ve pay verin.", "entries")
    scale, mode, rule = r["scale"], r["rounding"].strip(), r["remainder"].strip()
    parcels = []
    for i, e in enumerate(inp["entries"]):
        parcel, holder, area = e["parcel"].strip(), e["holder"].strip(), e["area"].strip()
        if not parcel or not holder:
            raise Refused("invalid_input", f"{i + 1}. kaydın parseli ya da hak sahibi boş.",
                          f"entries[{i}].parcel" if not parcel else f"entries[{i}].holder")
        if not PLAIN.match(area) or area.startswith("-") or places(area) > scale:
            raise Refused("invalid_input", f"{i + 1}. kaydın tapu alanı “{e['area']}” eksi olmayan, en çok {scale} "
                                           "basamaklı düz bir ondalık yazı olmalı.", f"entries[{i}].area")
        m = re.match(r"^([0-9]+)/([0-9]+)$", e["share"].strip())
        if not m or int(m.group(2)) == 0:
            raise Refused("invalid_input", f"{i + 1}. kaydın payı “{e['share']}” n/d biçiminde değil (payda sıfır olamaz).",
                          f"entries[{i}].share")
        share = Fraction(int(m.group(1)), int(m.group(2)))
        found = [p for p in parcels if p[0] == parcel]
        if found:
            p = found[0]
            if frac(p[1]) != frac(area):
                raise Refused("area_mismatch", f"“{parcel}” parselinin tapu alanı kayıtlarda farklı: {p[1]} ve {area}.",
                              f"entries[{i}].area")
            if any(h == holder for _, h, _ in p[2]):
                raise Refused("duplicate_holder", f"“{holder}” “{parcel}” parselinde iki kez var; payları tek kayıtta "
                                                  "toplayın.", f"entries[{i}].holder")
            p[2].append((i, holder, share))
        else:
            parcels.append((parcel, area, [(i, holder, share)]))
    for parcel, _, hs in parcels:
        s = sum((x for _, _, x in hs), Fraction(0))
        if s != 1:
            raise Refused("shares_not_whole", f"“{parcel}” parselinin payları toplamı {s.numerator}/{s.denominator}, tam "
                                              "1 değil.", f"entries[{hs[0][0]}].share")
    ratio = frac(d)
    exact = sum((frac(a) * ratio for _, a, _ in parcels), Fraction(0))
    total = round_to(exact, scale, mode)
    registered = sum((frac(a) for _, a, _ in parcels), Fraction(0))
    try:
        if registered == 0:
            dparts, drem, dgiven = [Fraction(0)] * len(parcels), "0", []
        else:
            dparts, drem, dgiven = split(total, [frac(a) for _, a, _ in parcels], scale, mode, rule)
    except Refused as x:
        x.path = "rules.deduction"
        raise
    out_parcels, holders, nets, held = [], [], [], []
    for (parcel, area, hs), ded in zip(parcels, dparts):
        net = frac(area) - ded
        try:
            parts, rem, given = split(net, [x for _, _, x in hs], scale, mode, rule)
        except Refused as x:
            x.path = f"entries[{hs[0][0]}].share"
            raise
        ps = []
        for (_, holder, share), v in zip(hs, parts):
            ex = net * share
            ps.append({"holder": holder, "share": f"{share.numerator}/{share.denominator}",
                       "exact": f"{ex.numerator}/{ex.denominator}", "value": text(v, scale)})
            held.append(v)
            found = [h for h in holders if h[0] == holder]
            if found:
                found[0][1].append(v)
            else:
                holders.append((holder, [v]))
        out_parcels.append({"parcel": parcel, "area": area, "deduction": text(ded, scale), "net": text(net, scale),
                            "remainder": rem, "remainderTo": [hs[k][1] for k in given], "parts": ps})
        nets.append(net)
    conserved = (sum(dparts, Fraction(0)) + sum(nets, Fraction(0)) == registered and sum(held, Fraction(0)) ==
                 sum(nets, Fraction(0)) and sum(dparts, Fraction(0)) == total)
    return {"rules": r["id"].strip(), "release": r["release"], "draft": True, "registered": text(registered, scale),
            "deductionExact": normal(exact), "deduction": text(total, scale), "deductionRemainder": drem,
            "deductionRemainderTo": [parcels[k][0] for k in dgiven], "net": text(sum(nets, Fraction(0)), scale),
            "held": text(sum(held, Fraction(0)), scale), "conserved": conserved, "parcels": out_parcels,
            "holders": [{"holder": h, "value": text(sum(v, Fraction(0)), scale)} for h, v in holders]}


RULES = {"id": "ornek-paket", "release": 1, "jurisdiction": "Örnek kurum (sentetik)",
         "source": "Sentetik deneme belgesi; yürürlükteki mevzuat değildir", "effective": "2026-01-01",
         "approved": False, "deduction": "0.40", "scale": 2, "rounding": "half_even",
         "remainder": "largest_remainder"}


def rules(**kw):
    return dict(copy.deepcopy(RULES), **kw)


def entry(parcel, holder, area, share):
    return {"parcel": parcel, "holder": holder, "area": area, "share": share}


BASIC = [entry("101/1", "H1", "1000", "1/3"), entry("101/1", "H2", "1000", "2/3"),
         entry("101/2", "H2", "500.50", "1/2"), entry("101/2", "H3", "500.50", "1/2")]
THIRDS = [entry("P", f"H{i}", "100.01", "1/3") for i in range(1, 4)]
SEVENTHS = [entry("P", f"H{i}", "100", "1/7") for i in range(1, 8)]

SAME = {"revision": "same", "canUndo": False}


def case(name, entries, op="plan", **kw):
    inp = {"rules": rules(**kw), "entries": copy.deepcopy(entries)}
    try:
        out = run(inp)
    except Refused as x:
        return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
            "code": x.code, "message": x.message, "path": x.path}}, "expect": SAME}]}
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME}]}


def cases():
    out = [
        case("iki parsel, %40 kesinti, iki basamak; artıkların gittiği yer", BASIC),
        case("uygula: salt okunur", BASIC, op="execute"),
        case("doğrulama", BASIC, op="validate"),
        case("üçte birler: 0,01'lik artık en çok eksik kalana", THIRDS),
        case("yedide birler", SEVENTHS),
        case("artık kuralı reject: artık varsa ret", THIRDS, remainder="reject"),
        case("artık kuralı reject: artık yoksa geçer", [entry("P", "H1", "100", "1/2"), entry("P", "H2", "100", "1/2")],
             remainder="reject"),
        case("yarımda çifte yuvarlama", [entry("P", "H1", "10.05", "1/2"), entry("P", "H2", "10.05", "1/2")],
             deduction="0"),
        case("yarımda sıfırdan uzağa", [entry("P", "H1", "10.05", "1/2"), entry("P", "H2", "10.05", "1/2")],
             deduction="0", rounding="half_away_from_zero"),
        case("sıfıra doğru: artık daha büyük", SEVENTHS, rounding="toward_zero"),
        case("basamak 0", BASIC[:2], scale=0),
        case("kesinti 0: net tapu alanı", BASIC, deduction="0"),
        case("büyük alan, ince oran", [entry("A", "H1", "123456789.12", "3/7"), entry("A", "H2", "123456789.12", "4/7")],
             deduction="0.333333"),
        case("hak sahibinin parseller üzerinden toplamı", BASIC + [entry("101/3", "H1", "250", "1/1")]),
        case("bütün alanlar sıfır", [entry("P", "H1", "0", "1/1")]),
        case("payda sadeleşir: 2/4 = 1/2", [entry("P", "H1", "100", "2/4"), entry("P", "H2", "100", "1/2")]),
        case("tapu alanı basamaktan ince", [entry("P", "H1", "100.125", "1/1")]),
        case("pay paydası sıfır", [entry("P", "H1", "100", "1/0")]),
        case("pay biçimi yanlış", [entry("P", "H1", "100", "0.5")]),
        case("paylar toplamı 1 değil", [entry("P", "H1", "100", "1/3"), entry("P", "H2", "100", "1/3")]),
        case("aynı parselde farklı tapu alanı", [entry("P", "H1", "100", "1/2"), entry("P", "H2", "100.5", "1/2")]),
        case("aynı parselde aynı hak sahibi iki kez", [entry("P", "H1", "100", "1/2"), entry("P", "H1", "100", "1/2")]),
        case("kayıt yok", []),
        case("boş hak sahibi", [entry("P", " ", "100", "1/1")]),
        case("kural paketinin kaynağı boş", BASIC, source="  "),
        case("kesinti oranı 1", BASIC, deduction="1"),
        case("kesinti oranı eksi", BASIC, deduction="-0.1"),
        case("basamak 7", BASIC, scale=7),
        case("bilinmeyen yuvarlama", BASIC, rounding="bankers"),
        case("bilinmeyen artık kuralı", BASIC, remainder="last"),
    ]
    return out


def setup():
    return {"format": "kentos.document", "version": 2, "name": "Düzenleme dağıtım taslağı",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                        "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
            "activeLayer": "0", "entities": [], "uids": [], "styles": {"items": [], "categories": []}}


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.readjustment.distribute",
           "commandVersion": 1, "title": "Düzenleme dağıtım taslağı",
           "note": "ADR 1460. Bağımsız başvuru scripts/fixtures/readjustment_cases.py: Python fractions ve elle "
                   "yuvarlama; KentOS kodu yok. Veriler sentetiktir; kural paketi yürürlükteki mevzuat değildir.",
           "setup": setup(), "cases": cases()}
    text_ = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text_
        print("Düzenleme dağıtım ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text_, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
