#!/usr/bin/env python3
"""Bağlı görünümlerin bağı, ekran ölçeği ve görünüm başına zaman (docs/adr/0576 §1, §2): the cases of
`fixtures/views/v1/links.json`, written from the ADR without KentOS code.

    python3 scripts/fixtures/view_link_cases.py          # fixtures/views/v1/links.json'u yazar
    python3 scripts/fixtures/view_link_cases.py --check  # hiçbir şey yazmaz; karşılaştırır

- **Bağ**: Yok öbür görünümü olduğu gibi bırakır; Merkez yalnız merkezi alır (ölçek ve dönüklük öbürünün kalır); Merkez ve ölçek
  merkezi, ölçeği ve dönüklüğü alır.
- **Ekran ölçeği**: piksel/metre ölçeğin 96 dpi ekrandaki 1:N'i, N = 1 / (ölçek · 0,00026458): aynı çift duyarlıklı işlemler.
- **Görünümün kendi zamanı**: sürgünün ızgarası (çapa, adım, son konum) üzerinde anına en yakın konum (eşit uzaklıkta önceki),
  penceresi bir an ya da bir sonraki konuma kadar; ◀ ve ▶ konumu bir adım oynatır, ızgaranın içinde. Ay ve yıl takvimle
  eklenir (ayın son gününü aşan gün ayın son günü); konumlar Python'un `datetime`'ıyla, en yakın konum kaba kuvvetle bulunur.
"""

import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "views" / "v1" / "links.json"
METRES_PER_PX = 0.00026458
EPOCH = datetime(1970, 1, 1, tzinfo=timezone.utc)
FIXED = {"second": 1000, "minute": 60_000, "hour": 3_600_000, "day": 86_400_000, "week": 604_800_000}


def ms(y, mo=1, d=1, h=0, mi=0):
    return int((datetime(y, mo, d, h, mi, tzinfo=timezone.utc) - EPOCH).total_seconds() * 1000)


def position(anchor, step, k):
    n, unit = step["n"], step["unit"]
    if unit in FIXED:
        return anchor + k * n * FIXED[unit]
    t = EPOCH + timedelta(milliseconds=anchor)
    months = (12 if unit == "year" else 1) * n * k
    total = t.year * 12 + (t.month - 1) + months
    y, m = divmod(total, 12)
    m += 1
    last_day = ((datetime(y + (m == 12), m % 12 + 1, 1, tzinfo=timezone.utc) - timedelta(days=1)).day)
    d = min(t.day, last_day)
    day = datetime(y, m, d, tzinfo=timezone.utc)
    into = (t - t.replace(hour=0, minute=0, second=0, microsecond=0)) // timedelta(milliseconds=1)
    return int((day - EPOCH) // timedelta(milliseconds=1)) + into


def nearest(anchor, step, last, moment):
    """The position whose moment is nearest `moment`; the earlier of two as near (brute force)."""
    best = 0
    for k in range(last + 1):
        if abs(moment - position(anchor, step, k)) < abs(moment - position(anchor, step, best)):
            best = k
    return best


def window(anchor, step, last, ranged, moment):
    k = nearest(anchor, step, last, moment)
    a = position(anchor, step, k)
    return k, ([a, position(anchor, step, k + 1)] if ranged else [a])


def frame(x, y, scale, twist):
    return {"centre": [x, y], "scale": scale, "twist": twist}


def follow(link, active, other):
    if link == "off":
        return other
    if link == "centre":
        return {**other, "centre": active["centre"]}
    return active


def cases():
    out = {"format": "kentos.view-link-cases", "version": 1, "follow": [], "scale": [], "window": [], "walk": []}
    active = frame(487_250.5, 4_420_300.25, 0.4, 0.0)
    other = frame(486_900.0, 4_419_950.75, 1.6, 0.5235987755982988)
    twisted = frame(487_250.5, 4_420_300.25, 0.4, 1.0471975511965976)
    for link in ("off", "centre", "centreScale"):
        for a, b in ((active, other), (twisted, other), (active, active)):
            out["follow"].append({"link": link, "active": a, "other": b, "expect": follow(link, a, b)})
    # The scales of a status bar's list and a few in between: 1:N from pixels per metre, and a scale made from N.
    for scale in (0.0001, 0.05, 0.4, 1.0, 3.7795, 3779.5276, 12345.678):
        out["scale"].append({"scale": scale, "expect": 1.0 / (scale * METRES_PER_PX)})
    for n in (500, 1000, 2000, 5000, 25_000):
        out["scale"].append({"scale": 1.0 / (n * METRES_PER_PX), "expect": 1.0 / ((1.0 / (n * METRES_PER_PX)) * METRES_PER_PX), "n": n})
    # The time a view of its own shows: 2015 on one side, 2023 on the other.
    years = {"n": 1, "unit": "year"}
    anchor = ms(2000)
    for ranged in (False, True):
        for moment in (ms(2015), ms(2023), ms(2015, 7, 1), ms(2015, 7, 3), ms(1990), ms(2100), ms(2000), ms(2030)):
            k, w = window(anchor, years, 30, ranged, moment)
            out["window"].append({"anchor": anchor, "step": years, "last": 30, "ranged": ranged, "moment": moment,
                                  "expect": {"position": k, "window": w}})
    # Months by the calendar from the 31st (a short month's last day), two-year and hourly steps.
    months = {"n": 1, "unit": "month"}
    for moment in (ms(2020, 2, 28), ms(2020, 3, 15), ms(2020, 5, 1)):
        k, w = window(ms(2020, 1, 31), months, 12, False, moment)
        out["window"].append({"anchor": ms(2020, 1, 31), "step": months, "last": 12, "ranged": False, "moment": moment,
                              "expect": {"position": k, "window": w}})
    two_years = {"n": 2, "unit": "year"}
    for moment in (ms(2011), ms(2012, 6, 1), ms(2013)):
        k, w = window(ms(2010), two_years, 10, True, moment)
        out["window"].append({"anchor": ms(2010), "step": two_years, "last": 10, "ranged": True, "moment": moment,
                              "expect": {"position": k, "window": w}})
    hours = {"n": 6, "unit": "hour"}
    for moment in (ms(2024, 3, 1, 5), ms(2024, 3, 1, 9), ms(2024, 3, 1, 12)):
        k, w = window(ms(2024, 3, 1), hours, 8, False, moment)
        out["window"].append({"anchor": ms(2024, 3, 1), "step": hours, "last": 8, "ranged": False, "moment": moment,
                              "expect": {"position": k, "window": w}})
    # ◀ and ▶: a step from where the view stands, kept inside the grid.
    for moment, by in ((ms(2015), 1), (ms(2015), -1), (ms(2000), -1), (ms(2030), 1), (ms(2015, 7, 3), 1), (ms(2015, 7, 3), -2)):
        k = min(max(nearest(anchor, years, 30, moment) + by, 0), 30)
        out["walk"].append({"anchor": anchor, "step": years, "last": 30, "moment": moment, "by": by,
                            "expect": position(anchor, years, k)})
    return out


def self_checks():
    assert position(ms(2020, 1, 31), {"n": 1, "unit": "month"}, 1) == ms(2020, 2, 29)
    assert position(ms(2020, 1, 31), {"n": 1, "unit": "month"}, 2) == ms(2020, 3, 31)
    assert position(ms(2000), {"n": 2, "unit": "year"}, 3) == ms(2006)
    assert nearest(ms(2000), {"n": 1, "unit": "year"}, 30, ms(2015, 7, 2)) == 15  # 182 days after, 184 before
    assert nearest(ms(2000), {"n": 1, "unit": "year"}, 30, ms(2015, 7, 3)) == 16  # 183 after, 182 before
    # Halfway between two positions the earlier one.
    assert nearest(ms(2000), {"n": 1, "unit": "day"}, 10, ms(2000, 1, 1, 12)) == 0
    # A scale and its 1:N come round: the scale of 1:N shows as N.
    for n in (500, 2000, 25_000):
        assert abs(1.0 / ((1.0 / (n * METRES_PER_PX)) * METRES_PER_PX) - n) < 1e-9 * n


def main():
    self_checks()
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check olmadan çalıştırıp farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
