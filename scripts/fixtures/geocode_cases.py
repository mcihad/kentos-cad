#!/usr/bin/env python3
"""Adres arama's shared cases (docs/adr/0229 §3–§9, §13.1), written from the ADR without KentOS code.

The ADR's rules are written here again: folding and words (§3), an address's parts (§4), names' similarity (§5), the
locator's index (§6), from an address to its places (§7), from a place to its address (§8), an address written (§9),
which attributes say what (§13.1). The scenes are drawn by hand: a project in metres and the same in millimetres, with
three neighbourhoods, a district and a province as squares, straight streets and doors. The cases are written to
fixtures/geocode/v1/cases.json; `--check` says whether the file is what this script writes.

Positions are float64 and compared by the core's test within 1e-6 of the project's unit (a street's middle and an
area's pole are computed by another algorithm there); scores, kinds, statuses, texts and orders are exact.
"""

import heapq
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geocode" / "v1" / "cases.json"

# ── §3 Folding and words ────────────────────────────────────────────────

FOLD = {
    "Ç": "c", "ç": "c", "Ğ": "g", "ğ": "g", "I": "i", "ı": "i", "İ": "i", "Ö": "o", "ö": "o",
    "Ş": "s", "ş": "s", "Ü": "u", "ü": "u", "Â": "a", "â": "a", "Î": "i", "î": "i", "Û": "u", "û": "u",
}

TAGS = {}
for words, tag in (
    (("mahalle", "mahallesi", "mah", "mh"), "MAHALLE"),
    (("koy", "koyu"), "KOY"),
    (("cadde", "caddesi", "cad", "cd"), "CADDE"),
    (("sokak", "sokagi", "sok", "sk"), "SOKAK"),
    (("bulvar", "bulvari", "blv", "bulv"), "BULVAR"),
    (("meydan", "meydani", "myd"), "MEYDAN"),
    (("yol", "yolu"), "YOL"),
    (("no", "numara", "nu", "nolu"), "NO"),
):
    for w in words:
        TAGS[w] = tag

STREET_TAGS = ("CADDE", "SOKAK", "BULVAR", "MEYDAN", "YOL")
AREA_TAGS = ("MAHALLE", "KOY")


def fold(text):
    out = []
    for c in text:
        if c in FOLD:
            out.append(FOLD[c])
        elif "̀" <= c <= "ͯ":
            continue
        else:
            low = c.lower()
            out.append(low if len(low) == 1 else c)
    return "".join(out)


def is_digit(c):
    return "0" <= c <= "9"


def tokens(folded):
    """Words and the marks / , -: a word is cut before an ASCII digit after a character that is not one."""
    out, word = [], ""
    for c in folded:
        if c.isalnum():
            if is_digit(c) and word and not is_digit(word[-1]):
                out.append(("w", word))
                word = ""
            word += c
            continue
        if word:
            out.append(("w", word))
            word = ""
        if c in "/,-":
            out.append((c, c))
    if word:
        out.append(("w", word))
    return out


def numbered_word(w):
    return bool(w) and is_digit(w[0])


def tag(token):
    return TAGS.get(token[1]) if token[0] == "w" else None


# ── §4 An address's parts ───────────────────────────────────────────────


def door_key(word):
    i = 0
    while i < len(word) and is_digit(word[i]):
        i += 1
    digits = word[:i].lstrip("0")
    if not digits and i > 0:
        digits = "0"
    return digits + word[i:]


def door_number(key):
    i = 0
    while i < len(key) and is_digit(key[i]):
        i += 1
    if i == 0 or i > 10:
        return None
    n = int(key[:i])
    return n if n <= 1_000_000_000 else None


class Reading:
    def __init__(self, toks):
        self.t = list(toks)
        self.taken = [False] * len(self.t)

    def word(self, i):
        if 0 <= i < len(self.t) and self.t[i][0] == "w":
            return self.t[i][1]
        return None

    def free(self, i):
        w = self.word(i)
        if w is None or self.taken[i] or w in TAGS:
            return None
        return w

    def tag(self, i):
        w = self.word(i)
        return TAGS.get(w) if w is not None else None

    def run_before(self, i):
        """§3's name read back from before i; its words become taken."""
        words, j = [], i - 1
        while j >= 0:
            w = self.free(j)
            if w is None:
                break
            if numbered_word(w) and j >= 2 and self.t[j - 1][0] == "/" and self.free(j - 2) is not None and numbered_word(self.free(j - 2)):
                words.append(self.free(j - 2) + "/" + w)
                self.taken[j] = self.taken[j - 1] = self.taken[j - 2] = True
                j -= 3
                continue
            words.append(w)
            self.taken[j] = True
            j -= 1
        return " ".join(reversed(words))

    def door_at(self, i):
        """§4.4: the numbered free word at i, a letter after / or -, an inner door after /."""
        w = self.free(i)
        if w is None or not numbered_word(w):
            return None
        key = door_key(w)
        self.taken[i] = True
        if i + 1 < len(self.t) and self.t[i + 1][0] in "/-":
            nxt = self.free(i + 2)
            if nxt is not None:
                if len(nxt) == 1 and not numbered_word(nxt):
                    key += nxt
                    self.taken[i + 1] = self.taken[i + 2] = True
                elif self.t[i + 1][0] == "/" and numbered_word(nxt):
                    self.taken[i + 1] = self.taken[i + 2] = True
        return key


def postcode_like(w):
    return len(w) == 5 and all(is_digit(c) for c in w) and 1 <= int(w[:2]) <= 81


def parse(text):
    r = Reading(tokens(fold(text)))
    p = {"postcode": None, "street": None, "streetKind": None, "door": None, "neighbourhood": None,
         "district": None, "province": None, "rest": [], "untagged": False}
    n = len(r.t)
    for i in range(n):
        w = r.word(i)
        if w is not None and postcode_like(w):
            p["postcode"] = w
            r.t[i] = (",", ",")
            break
    no = next((i for i in range(n) if r.tag(i) == "NO"), None)
    streets = [i for i in range(n) if r.tag(i) in STREET_TAGS]
    k = None
    if no is not None:
        before = [i for i in streets if i < no]
        if before:
            k = max(before)
    if k is None and streets:
        k = streets[-1]
    if k is not None:
        r.taken[k] = True
        name = r.run_before(k)
        if name:
            p["street"] = name
            p["streetKind"] = r.tag(k)
    if no is not None:
        r.taken[no] = True
        j = no + 1
        while j < n and r.t[j][0] == "-":
            j += 1
        p["door"] = r.door_at(j)
    elif k is not None:
        j = k + 1
        while j < n and r.t[j][0] == ",":
            j += 1
        p["door"] = r.door_at(j)
    areas = [i for i in range(n) if r.tag(i) in AREA_TAGS]
    m = None
    if k is not None:
        before = [i for i in areas if i < k]
        if before:
            m = max(before)
    if m is None and areas:
        m = areas[-1]
    if m is not None:
        r.taken[m] = True
        name = r.run_before(m)
        if name:
            p["neighbourhood"] = name
    if k is None and m is None:
        p["untagged"] = True
        if no is None:
            first = next((i for i in range(1, n) if r.free(i) is not None and numbered_word(r.free(i))), None)
            if first is not None:
                p["door"] = r.door_at(first)
        words = []
        for i in range(n):
            if r.t[i][0] != "w":
                break
            if r.taken[i]:
                if p["door"] is not None:
                    break
                continue
            if r.t[i][1] not in TAGS:
                words.append(r.t[i][1])
                r.taken[i] = True
        if words:
            p["street"] = " ".join(words)
    else:
        for s in range(n - 1, -1, -1):
            if r.t[s][0] != "/" or r.taken[s]:
                continue

            def plain(i):
                w = r.free(i)
                return w is not None and not numbered_word(w)

            a = s
            while a > 0 and plain(a - 1):
                a -= 1
            b = s + 1
            while b < n and plain(b):
                b += 1
            if a < s and b > s + 1:
                p["district"] = " ".join(r.t[i][1] for i in range(a, s))
                p["province"] = " ".join(r.t[i][1] for i in range(s + 1, b))
                for i in range(a, b):
                    r.taken[i] = True
                break
    p["rest"] = [r.t[i][1] for i in range(n) if r.free(i) is not None]
    return p


def words_key(text):
    return " ".join(w for kind, w in tokens(fold(text)) if kind == "w")


def street_value(text):
    r = Reading(tokens(fold(text)))
    k = next((i for i in range(len(r.t) - 1, -1, -1) if r.tag(i) in STREET_TAGS), None)
    if k is None:
        name, kind = words_key(text), None
    else:
        r.taken[k] = True
        name, kind = r.run_before(k), r.tag(k)
    return (name, kind) if name else None


def door_value(text):
    r = Reading(tokens(fold(text)))
    i = next((i for i in range(len(r.t)) if r.free(i) is not None and numbered_word(r.free(i))), None)
    return None if i is None else r.door_at(i)


def area_value(text):
    return " ".join(w for kind, w in tokens(fold(text)) if kind == "w" and TAGS.get(w) not in AREA_TAGS)


def from_columns(c):
    sv = street_value(c.get("street", ""))
    return {
        "postcode": c.get("postcode", "").strip() or None,
        "street": sv[0] if sv else None,
        "streetKind": sv[1] if sv else None,
        "door": door_value(c.get("door", "")),
        "neighbourhood": area_value(c.get("neighbourhood", "")) or None,
        "district": words_key(c.get("district", "")) or None,
        "province": words_key(c.get("province", "")) or None,
        "rest": [],
        "untagged": False,
    }


# ── §5 Similarity ───────────────────────────────────────────────────────


def levenshtein(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j - 1] + (ca != cb), prev[j] + 1, cur[j - 1] + 1))
        prev = cur
    return prev[-1]


def numbered(name):
    return bool(name) and all(numbered_word(w) for w in name.split(" "))


def similarity(a, b):
    if not a or not b:
        return 0
    if numbered(a) or numbered(b):
        return 100 if a == b else 0
    big = max(len(a), len(b))
    return (100 * (big - levenshtein(a, b))) // big


def similar(a, b):
    s = similarity(a, b)
    return s if s >= 80 else None


# ── Plane geometry of the scenes (straight lines, rings without arcs) ────


def seg_len(a, b):
    return math.hypot(b[0] - a[0], b[1] - a[1])


def path_middle(pts):
    lengths = [seg_len(pts[i], pts[i + 1]) for i in range(len(pts) - 1)]
    total = sum(lengths)
    s = total / 2
    for i, length in enumerate(lengths):
        if s <= length or i == len(lengths) - 1:
            t = s / length if length > 0 else 0.0
            a, b = pts[i], pts[i + 1]
            return total, (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
        s -= length
    return total, tuple(pts[0])


def dist_to_segment(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    ll = dx * dx + dy * dy
    t = 0.0 if ll == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / ll))
    q = (a[0] + dx * t, a[1] + dy * t)
    return math.hypot(p[0] - q[0], p[1] - q[1]), q


def dist_to_path(p, pts):
    best = (math.inf, tuple(p))
    for i in range(len(pts) - 1):
        d, q = dist_to_segment(p, pts[i], pts[i + 1])
        if d < best[0]:
            best = (d, q)
    return best


def on_ring(p, ring):
    n = len(ring)
    return any(dist_to_segment(p, ring[i], ring[(i + 1) % n])[0] <= 1e-9 for i in range(n))


def in_ring(p, ring):
    inside, n = False, len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]):
            x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            if p[0] < x:
                inside = not inside
    return inside


def in_area(p, rings):
    """Inside the area or on its edge (holes left out, their edges in)."""
    if any(on_ring(p, r) for r in rings):
        return True
    return in_ring(p, rings[0]) and not any(in_ring(p, h) for h in rings[1:])


def ring_area(ring):
    s = 0.0
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        s += a[0] * b[1] - b[0] * a[1]
    return abs(s) / 2


def area_size(rings):
    return ring_area(rings[0]) - sum(ring_area(h) for h in rings[1:])


def polylabel(rings, precision):
    """Mapbox's polylabel (the pole of inaccessibility), as ADR 0219 §6 names it."""

    def point_dist(x, y):
        inside, best = False, math.inf
        for ring in rings:
            n = len(ring)
            for i in range(n):
                a, b = ring[i], ring[i - 1]
                if (a[1] > y) != (b[1] > y) and x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0]:
                    inside = not inside
                best = min(best, dist_to_segment((x, y), a, b)[0])
        return best if inside else -best

    outer = rings[0]
    min_x = min(p[0] for p in outer)
    min_y = min(p[1] for p in outer)
    max_x = max(p[0] for p in outer)
    max_y = max(p[1] for p in outer)
    w, h = max_x - min_x, max_y - min_y
    size = min(w, h)
    if size == 0:
        return (min_x, min_y)
    half = size / 2
    queue, n = [], 0

    def push(x, y, hh):
        nonlocal n
        d = point_dist(x, y)
        heapq.heappush(queue, (-(d + hh * math.sqrt(2)), n, x, y, hh, d))
        n += 1

    x = min_x
    while x < max_x:
        y = min_y
        while y < max_y:
            push(x + half, y + half, half)
            y += size
        x += size
    # The centroid's cell, then the box's middle, as polylabel starts.
    area, cx, cy = 0.0, 0.0, 0.0
    for i in range(len(outer)):
        a, b = outer[i], outer[i - 1]
        f = a[0] * b[1] - b[0] * a[1]
        cx += (a[0] + b[0]) * f
        cy += (a[1] + b[1]) * f
        area += f * 3
    best = (outer[0][0], outer[0][1], point_dist(outer[0][0], outer[0][1])) if area == 0 else (cx / area, cy / area, point_dist(cx / area, cy / area))
    box = (min_x + w / 2, min_y + h / 2, point_dist(min_x + w / 2, min_y + h / 2))
    if box[2] > best[2]:
        best = box
    while queue:
        _, _, x, y, hh, d = heapq.heappop(queue)
        if d > best[2]:
            best = (x, y, d)
        if d + hh * math.sqrt(2) - best[2] <= precision:
            continue
        hh /= 2
        push(x - hh, y - hh, hh)
        push(x + hh, y - hh, hh)
        push(x - hh, y + hh, hh)
        push(x + hh, y + hh, hh)
    return (best[0], best[1])


# ── §6 The locator ──────────────────────────────────────────────────────


def build(scene):
    per_metre = scene["perMetre"]
    loc = {"perMetre": per_metre, "doors": [], "lines": [], "areas": [], "groups": [], "codes": {},
           "emptyStreets": 0, "unreadDoors": 0}
    for a in sorted(scene["areas"], key=lambda a: a["order"]):
        name = area_value(a["name"]) if a["level"] == "neighbourhood" else words_key(a["name"])
        if not name:
            continue
        loc["areas"].append({**a, "key": name, "districtKey": words_key(a.get("district", "")),
                             "size": area_size(a["rings"])})
    keys = {}
    for d in sorted(scene["doors"], key=lambda d: d["order"]):
        sv = street_value(d["street"])
        if sv is None:
            loc["emptyStreets"] += 1
            continue
        key = door_value(d["door"])
        if key is None:
            loc["unreadDoors"] += 1
            continue
        hood, district = area_value(d.get("neighbourhood", "")), words_key(d.get("district", ""))
        gk = (sv[0], hood, district)
        if gk not in keys:
            keys[gk] = len(loc["groups"])
            loc["groups"].append({"name": sv[0], "kind": sv[1], "hood": hood, "district": district,
                                  "doors": [], "lines": [], "order": d["order"], "postcodes": []})
        g = loc["groups"][keys[gk]]
        index = len(loc["doors"])
        g["doors"].append(index)
        pc = d.get("postcode", "").strip()
        if pc and pc not in g["postcodes"]:
            g["postcodes"].append(pc)
        code = d.get("code", "").strip()
        if code:
            loc["codes"].setdefault(code, []).append(index)
        texts = {k: d.get(k, "").strip() for k in ("street", "door", "neighbourhood", "district", "province", "postcode", "building", "code")}
        loc["doors"].append({"order": d["order"], "at": tuple(d["at"]), "key": key, "number": door_number(key),
                             "group": gk, "texts": texts})
    for s in sorted(scene["streets"], key=lambda s: s["order"]):
        sv = street_value(s["name"])
        if sv is None:
            continue
        length, middle = path_middle(s["pts"])
        own = area_value(s.get("neighbourhood", ""))
        hood_text = s.get("neighbourhood", "").strip()
        if not own:
            hood_text = ""
            for a in loc["areas"]:
                if a["level"] == "neighbourhood" and in_area(middle, a["rings"]):
                    own, hood_text = a["key"], a["name"].strip()
                    break
        loc["lines"].append({"order": s["order"], "pts": s["pts"], "length": length, "middle": middle, "name": sv[0],
                             "kind": sv[1], "hood": own, "district": words_key(s.get("district", "")),
                             "texts": {"street": s["name"].strip(), "door": "", "neighbourhood": hood_text,
                                       "district": s.get("district", "").strip(), "province": "", "postcode": "",
                                       "building": "", "code": ""}})

    def compatible(a, b):
        return a == b or a == "" or b == ""

    line_keys = {}
    for i, l in enumerate(loc["lines"]):
        if any(g["doors"] and g["name"] == l["name"] and compatible(g["hood"], l["hood"]) and compatible(g["district"], l["district"]) for g in loc["groups"]):
            continue
        lk = (l["name"], l["hood"], l["district"])
        if lk in line_keys:
            loc["groups"][line_keys[lk]]["lines"].append(i)
        else:
            line_keys[lk] = len(loc["groups"])
            loc["groups"].append({"name": l["name"], "kind": l["kind"], "hood": l["hood"], "district": l["district"],
                                  "doors": [], "lines": [i], "order": l["order"], "postcodes": []})
    order = sorted(range(len(loc["groups"])), key=lambda g: loc["groups"][g]["order"])
    loc["groups"] = [loc["groups"][g] for g in order]
    # Neighbourhoods: areas, then doors' groups, then lines; each key once with its first texts.
    hoods = {}

    def hood(key, text, district, province, order_):
        if key and key not in hoods:
            hoods[key] = {"key": key, "text": text, "district": district, "province": province, "order": order_}

    for a in loc["areas"]:
        if a["level"] == "neighbourhood":
            hood(a["key"], a["name"].strip(), a.get("district", "").strip(), "", a["order"])
    for d in loc["doors"]:
        hood(d["group"][1], d["texts"]["neighbourhood"], d["texts"]["district"], d["texts"]["province"], d["order"])
    for l in loc["lines"]:
        hood(l["hood"], l["texts"]["neighbourhood"], l["texts"]["district"], "", l["order"])
    loc["hoods"] = hoods
    districts = []
    for g in loc["groups"]:
        if g["district"] and g["district"] not in districts:
            districts.append(g["district"])
    for a in loc["areas"]:
        k = a["key"] if a["level"] == "district" else (a["districtKey"] if a["level"] == "neighbourhood" else "")
        if k and k not in districts:
            districts.append(k)
    loc["districts"] = districts
    return loc


def group_doors(loc, g):
    return [loc["doors"][i] for i in g["doors"]]


def street_place(loc, g):
    if g["doors"]:
        doors = sorted(group_doors(loc, g), key=lambda d: (d["number"] if d["number"] is not None else 2**64, d["key"], d["order"]))
        return doors[(len(doors) - 1) // 2]["at"]
    best = None
    for i in g["lines"]:
        l = loc["lines"][i]
        if best is None or l["length"] > best["length"]:
            best = l
    return best["middle"]


def hood_place(loc, key):
    best = None
    for a in loc["areas"]:
        if a["level"] == "neighbourhood" and a["key"] == key and (best is None or a["size"] > best["size"]):
            best = a
    if best is not None:
        return polylabel(best["rings"], 0.1 * loc["perMetre"])
    pts = [d["at"] for d in loc["doors"] if d["group"][1] == key]
    if pts:
        sx = sy = 0.0
        for p in pts:
            sx += p[0]
            sy += p[1]
        return (sx / len(pts), sy / len(pts))
    for g in loc["groups"]:
        if not g["doors"] and g["hood"] == key:
            return street_place(loc, g)
    return None


# ── §9 An address written ───────────────────────────────────────────────


def tagged_hood(text):
    return any(kind == "w" and TAGS.get(w) in AREA_TAGS for kind, w in tokens(fold(text)))


def label(t):
    parts = []
    if t["neighbourhood"]:
        parts.append(t["neighbourhood"] if tagged_hood(t["neighbourhood"]) else t["neighbourhood"] + " Mahallesi")
    if t["street"]:
        parts.append(t["street"])
    if t["door"]:
        parts.append("No:" + t["door"])
    if t["postcode"]:
        parts.append(t["postcode"])
    if t["district"] and t["province"]:
        parts.append(t["district"] + "/" + t["province"])
    elif t["district"]:
        parts.append(t["district"])
    elif t["province"]:
        parts.append(t["province"])
    return " ".join(parts)


# ── §7 From an address to its places ────────────────────────────────────

KIND_RANK = {"Kod": 0, "Kapı": 1, "Kapı (aradeğer)": 2, "Yol": 3, "Mahalle": 4}
EMPTY = {"street": "", "door": "", "neighbourhood": "", "district": "", "province": "", "postcode": "", "building": "", "code": ""}


def district_in(loc, rest):
    best = None
    for d in loc["districts"]:
        words = d.split(" ")
        n = len(words)
        if n > len(rest):
            continue
        at = next((i for i in range(len(rest) - n + 1) if rest[i:i + n] == words), None)
        if at is None:
            continue
        if best is None or n > best[0] or (n == best[0] and at < best[1]):
            best = (n, at, d)
    return best[2] if best else None


def similar_names(loc, key):
    names = []
    for g in loc["groups"]:
        if g["name"] not in names:
            names.append(g["name"])
    return [(n, s) for n in names if (s := similar(key, n)) is not None]


def first_texts(loc, g):
    if g["doors"]:
        t = loc["doors"][g["doors"][0]]["texts"]
        return {**EMPTY, "street": t["street"], "neighbourhood": t["neighbourhood"], "district": t["district"], "province": t["province"]}
    return dict(loc["lines"][g["lines"][0]]["texts"])


def cand(kind, score, at, texts, g, door_key_, order):
    return {"kind": kind, "score": score, "at": [at[0], at[1]], "texts": texts, "label": label(texts),
            "streetKey": g["name"], "hoodKey": g["hood"], "doorKey": door_key_, "order": order}


def street_candidate(loc, g, b, q):
    pen = 100 - b
    if q["kind"] and g["kind"] and q["kind"] != g["kind"]:
        pen += 5
    if q["hood"] and g["hood"]:
        m = similar(q["hood"], g["hood"])
        pen += (100 - m) if m is not None else 30
    if q["district"] and g["district"] and similar(q["district"], g["district"]) is None:
        pen += 20
    if q["postcode"] and g["postcodes"] and q["postcode"] not in g["postcodes"]:
        pen += 10

    def score(extra):
        return max(0, 100 - pen - extra)

    door = q["door"]
    if door is not None:
        doors = group_doors(loc, g)
        exact = [d for d in doors if d["key"] == door]
        if exact:
            d = exact[0]
            return cand("Kapı", score(0), d["at"], d["texts"], g, d["key"], d["order"])
        n = door_number(door)
        if n is not None:
            same = sorted((d for d in doors if d["number"] == n), key=lambda d: (d["key"] != str(n), d["key"], d["order"]))
            if same:
                d = same[0]
                return cand("Kapı", score(5), d["at"], d["texts"], g, d["key"], d["order"])
            side = [d for d in doors if d["number"] is not None and d["number"] % 2 == n % 2]
            below = [d["number"] for d in side if d["number"] < n]
            above = [d["number"] for d in side if d["number"] > n]
            if below and above:
                na, nb = max(below), min(above)
                a = sorted((d for d in side if d["number"] == na), key=lambda d: (d["key"], d["order"]))[0]
                c = sorted((d for d in side if d["number"] == nb), key=lambda d: (d["key"], d["order"]))[0]
                f = (n - na) / (nb - na)
                at = (a["at"][0] + (c["at"][0] - a["at"][0]) * f, a["at"][1] + (c["at"][1] - a["at"][1]) * f)
                texts = {**first_texts(loc, g), "door": door}
                return cand("Kapı (aradeğer)", score(10), at, texts, g, door, min(a["order"], c["order"]))
        return cand("Yol", score(25), street_place(loc, g), first_texts(loc, g), g, "", g["order"])
    return cand("Yol", score(15), street_place(loc, g), first_texts(loc, g), g, "", g["order"])


def sort_key(c):
    return (-c["score"], KIND_RANK[c["kind"]], c["streetKey"], c["hoodKey"], c["doorKey"], c["order"])


def status_of(cands, min_score):
    if not cands or cands[0]["score"] < min_score:
        return "Eşleşmedi"
    best = cands[0]
    tied = any(c["score"] == best["score"] and c["at"] != best["at"] for c in cands[1:])
    return "Belirsiz" if tied else "Eşleşti"


def find(loc, text, q, min_score, limit):
    code = text.strip()
    if code and code in loc["codes"]:
        out = []
        for i in loc["codes"][code]:
            d = loc["doors"][i]
            g = next(g for g in loc["groups"] if (g["name"], g["hood"], g["district"]) == d["group"] and g["doors"])
            out.append(cand("Kod", 100, d["at"], d["texts"], g, d["key"], d["order"]))
        out.sort(key=sort_key)
        out = out[:limit]
        return out, status_of(out, min_score)
    asked = {"street": q["street"], "kind": q["streetKind"], "door": q["door"], "hood": q["neighbourhood"],
             "district": q["district"] or district_in(loc, q["rest"]), "postcode": q["postcode"]}
    names = []
    if asked["street"]:
        names = similar_names(loc, asked["street"])
        words = asked["street"].split(" ")
        if not names and len(words) >= 2:
            for k in range(1, len(words)):
                tail = " ".join(words[k:])
                found = similar_names(loc, tail)
                if found:
                    if asked["hood"] is None:
                        asked["hood"] = " ".join(words[:k])
                    asked["street"] = tail
                    names = found
                    break
    out = []
    for name, b in names:
        for g in loc["groups"]:
            if g["name"] == name:
                out.append(street_candidate(loc, g, b, asked))
    hood = q["street"] if q["untagged"] else asked["hood"]
    if hood:
        for key, info in loc["hoods"].items():
            m = similar(hood, key)
            if m is None:
                continue
            at = hood_place(loc, key)
            if at is None:
                continue
            texts = {**EMPTY, "neighbourhood": info["text"], "district": info["district"], "province": info["province"]}
            out.append({"kind": "Mahalle", "score": max(0, m - 40), "at": [at[0], at[1]], "texts": texts, "label": label(texts),
                        "streetKey": "", "hoodKey": key, "doorKey": "", "order": info["order"]})
    out = [c for c in out if c["score"] >= 40]
    out.sort(key=sort_key)
    out = out[:limit]
    return out, status_of(out, min_score)


# ── §8 From a place to its address ──────────────────────────────────────


def reverse(loc, p, reach):
    units = reach * loc["perMetre"]
    door = None
    for i, d in enumerate(loc["doors"]):
        dd = math.hypot(d["at"][0] - p[0], d["at"][1] - p[1])
        if dd <= units and (door is None or (dd, d["order"]) < (door[1], loc["doors"][door[0]]["order"])):
            door = (i, dd)
    line = None
    if door is None:
        for i, l in enumerate(loc["lines"]):
            dd, q = dist_to_path(p, l["pts"])
            if dd <= units and (line is None or (dd, l["order"]) < (line[1], loc["lines"][line[0]]["order"])):
                line = (i, dd, q)

    def area_at(level):
        return next((a for a in loc["areas"] if a["level"] == level and in_area(p, a["rings"])), None)

    hood, district, province = area_at("neighbourhood"), area_at("district"), area_at("province")
    if door is None and line is None and hood is None and district is None and province is None:
        return None
    if door is not None:
        d = loc["doors"][door[0]]
        kind, at, distance, texts = "Kapı", d["at"], door[1] / loc["perMetre"], dict(d["texts"])
    elif line is not None:
        l = loc["lines"][line[0]]
        kind, at, distance, texts = "Yol", line[2], line[1] / loc["perMetre"], dict(l["texts"])
    else:
        kind, at, distance, texts = "Bölge", tuple(p), None, dict(EMPTY)

    def first(a, b):
        return a if a else b

    texts["neighbourhood"] = first(texts["neighbourhood"], hood["name"].strip() if hood else "")
    texts["district"] = first(first(texts["district"], hood.get("district", "").strip() if hood else ""), district["name"].strip() if district else "")
    texts["province"] = first(texts["province"], province["name"].strip() if province else "")
    return {"kind": kind, "at": [at[0], at[1]], "distance": distance, "texts": texts, "label": label(texts)}


# ── §13.1 Which attributes say what ─────────────────────────────────────

CODED = {"kod", "kodu", "id", "kimlik", "tip", "tipi", "tur", "turu", "no"}


def name_of(field):
    words = [w for kind, w in tokens(fold(field)) if kind == "w"]
    return words, "".join(words)


def r_code(w, c):
    return "uavt" in c or ("adres" in w and ("kod" in w or "kodu" in w))


def r_door(w, c):
    return "ic" not in w and ("kapi" in w or c.startswith("kapino") or "numara" in w or w == ["no"])


def r_postcode(w, c):
    return "posta" in w or c in ("pk", "postakodu")


def r_street(w, c):
    return not (CODED & set(w)) and any(x in c for x in ("yol", "sokak", "cadde", "csbm", "bulvar"))


def r_hood(w, c):
    return not (CODED & set(w)) and ("mahalle" in c or "mah" in w or "koy" in c)


def r_district(w, c):
    return not (CODED & set(w)) and "ilce" in c


def r_province(w, c):
    return not (CODED & set(w)) and ("il" in w or "sehir" in c or c == "iladi")


def r_building(w, c):
    return not (CODED & set(w)) and any(x in c for x in ("bina", "apartman", "site"))


def r_plain(w, c):
    return not (CODED & set(w)) and any(x in w for x in ("ad", "adi", "isim", "name"))


def take(fields, roles):
    names = [name_of(f) for f in fields]
    taken = [False] * len(fields)
    out = []
    for rule in roles:
        found = None
        for i, (w, c) in enumerate(names):
            if not taken[i] and rule(w, c):
                taken[i] = True
                found = fields[i]
                break
        out.append(found)
    return out


def guess_doors(fields):
    code, door, postcode, street, hood, district, province, building = take(
        fields, [r_code, r_door, r_postcode, r_street, r_hood, r_district, r_province, r_building])
    return {"street": street, "door": door, "neighbourhood": hood, "district": district, "province": province,
            "postcode": postcode, "building": building, "code": code}


def guess_streets(fields):
    name, hood, district = take(fields, [lambda w, c: r_street(w, c) or r_plain(w, c), r_hood, r_district])
    return {"name": name, "neighbourhood": hood, "district": district}


def guess_areas(level, fields):
    own = {"neighbourhood": r_hood, "district": r_district, "province": r_province}[level]

    def named(w, c):
        return r_plain(w, c) or own(w, c)

    if level == "neighbourhood":
        name, district = take(fields, [named, r_district])
        return {"name": name, "district": district}
    return {"name": take(fields, [named])[0], "district": None}


# ── The scenes ──────────────────────────────────────────────────────────


def square(x0, y0, x1, y1):
    return [[[x0, y0], [x1, y0], [x1, y1], [x0, y1]]]


def city(scale, ox, oy, per_metre):
    """Three neighbourhoods (Kızılay and Cumhuriyet side by side, Bahçelievler above Kızılay), Çankaya and Ankara round
    them; streets and doors as docs/adr/0229's cases need them. `scale` multiplies every coordinate."""

    def at(x, y):
        return [ox + x * scale, oy + y * scale]

    order = iter(range(1000))
    areas = [
        {"order": next(order), "level": "neighbourhood", "rings": [[at(*p) for p in square(0, 0, 200, 200)[0]]], "name": "Kızılay Mahallesi", "district": "Çankaya"},
        {"order": next(order), "level": "neighbourhood", "rings": [[at(*p) for p in square(200, 0, 400, 200)[0]]], "name": "Cumhuriyet Mahallesi", "district": "Çankaya"},
        {"order": next(order), "level": "neighbourhood", "rings": [[at(*p) for p in square(0, 200, 200, 400)[0]]], "name": "Bahçelievler", "district": "Çankaya"},
        {"order": next(order), "level": "district", "rings": [[at(*p) for p in square(-50, -50, 450, 450)[0]]], "name": "ÇANKAYA", "district": ""},
        {"order": next(order), "level": "province", "rings": [[at(*p) for p in square(-100, -100, 500, 500)[0]]], "name": "Ankara", "district": ""},
    ]
    streets = [
        {"order": next(order), "pts": [at(0, 100), at(200, 100)], "name": "Atatürk Bulvarı", "neighbourhood": "Kızılay", "district": ""},
        {"order": next(order), "pts": [at(200, 100), at(300, 100), at(400, 100)], "name": "Atatürk Bulvarı", "neighbourhood": "", "district": ""},
        {"order": next(order), "pts": [at(300, 0), at(300, 200)], "name": "1203/1 Sokak", "neighbourhood": "", "district": ""},
        {"order": next(order), "pts": [at(0, 300), at(150, 300), at(200, 300)], "name": "Gazi Mustafa Kemal Bulvarı", "neighbourhood": "", "district": ""},
        {"order": next(order), "pts": [at(420, 50), at(480, 50)], "name": "Ankara Yolu", "neighbourhood": "", "district": "Çankaya"},
    ]

    def door(x, y, street, number, hood, postcode="", code="", building=""):
        return {"order": next(order), "at": at(x, y), "street": street, "door": number, "neighbourhood": hood,
                "district": "Çankaya", "province": "Ankara", "postcode": postcode, "building": building, "code": code}

    k, c, b = "Kızılay", "Cumhuriyet", "Bahçelievler Mahallesi"
    doors = [
        door(10, 105, "Atatürk Bulvarı", "2", k, "06420", "1000000001"),
        door(30, 105, "ATATÜRK BULVARI", "4", k, "06420"),
        door(70, 105, "Atatürk Blv.", "8", k, "06420"),
        door(90, 105, "Atatürk Bulvarı", "10", k, "06420"),
        door(108, 105, "Atatürk Bulvarı", "12", k, "06420", building="Gül Apartmanı"),
        door(110, 105, "Atatürk Bulvarı", "12/A", k, "06420"),
        door(130, 105, "Atatürk Bulvarı", "14", k, "06420"),
        door(131, 105, "Atatürk Bulvarı", "14", k, "06420"),
        door(10, 95, "Atatürk Bulvarı", "1", k, "06420"),
        door(30, 95, "Atatürk Bulvarı", "3", k, "06420"),
        door(50, 95, "Atatürk Bulvarı", "5", k, "06420"),
        door(210, 105, "Atatürk Bulvarı", "2", c, "06100", "1000000002"),
        door(230, 105, "Atatürk Bulvarı", "4", c, "06100"),
        door(210, 95, "Atatürk Bulvarı", "1", c, "06100"),
        door(295, 20, "1203/1 Sokak", "1", c, "06100"),
        door(305, 20, "1203/1. Sk.", "2", c, "06100"),
        door(295, 40, "1203/1 Sokak", "003", c, "06100"),
        door(295, 60, "1203/1 Sokak", "5", c, "06100"),
        door(10, 210, "7. Cadde", "1", b),
        door(12, 215, "7. Cadde", "2", b),
        door(150, 150, "", "9", k),
        door(160, 150, "Atatürk Bulvarı", "A", k),
    ]
    return {"perMetre": per_metre, "doors": doors, "streets": streets, "areas": areas}


FORWARD = [
    "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara",
    "Atatürk Bulvarı No:2",
    "Kızılay Mahallesi Atatürk Bulvarı No:6",
    "Kızılay Mah. Atatürk Cad. No:4",
    "Kizilay Mh. Ataturk Blv No 12/A",
    "Kızılay Mah. Atatürk Blv. No:12/B",
    "Kızılay Mah. Atatürk Blv. No:99",
    "Atatrk Blv. No:4",
    "Cumhuriyet Mahallesi 1203/1. Sokak No: 3",
    "1203/2 Sokak No:1",
    "Kızılay",
    "Kızılay Atatürk Bulvarı No:4",
    "Gazi Mustafa Kemal Blv.",
    "1000000002",
    "Atatürk Blv. No:2 06100",
    "7. Cadde No:1",
    "Bahçelievler",
    "Atatürk 10",
    "Ankara Yolu",
    "Atatürk Bulvarı No:4 Çankaya",
    "Cumhuriyet Mh. Atatürk Bulvarı No:3",
    "İnönü Caddesi No:1",
]

COLUMNS = [
    {"neighbourhood": "Kızılay", "street": "ATATÜRK BULVARI", "door": "008", "district": "Çankaya"},
    {"neighbourhood": "Cumhuriyet Mahallesi", "street": "1203/1 Sokak", "door": "No: 5"},
    {"street": "Atatürk Bulvarı", "door": "6/A", "postcode": "06420"},
    {"neighbourhood": "Bahçelievler", "street": "", "door": ""},
]

REVERSE = [
    ((32, 106), 100.0),
    ((100, 300), 100.0),
    ((180, 380), 10.0),
    ((1000, 1000), 100.0),
    ((200, 10), 5.0),
    ((450, 52), 100.0),
    ((300, 30), 3.0),
]

PARSE = [
    "Kızılay Mah. Atatürk Blv. No:12/3 06420 Çankaya/Ankara",
    "Cumhuriyet Mahallesi 1203/1. Sokak No: 5/A",
    "Atatürk Cad. 15. Sok. 7",
    "Atatürk 12, Çankaya",
    "Atatürk Cad. No:5 Kızılay Mah.",
    "No-12 Gazi Sokağı",
    "Gölbaşı Köyü Merkez Sk. No:3 Gölbaşı / Ankara",
    "ANKARA YOLU NO:  0045-b",
    "Kızılay Atatürk Bulvarı 12",
    "  ",
    "Mah. Cad.",
    "İstiklal Cd. No:1/2 D:3",
]

SIMILARITY = [
    ("ataturk", "ataturk"), ("ataturk", "atatrk"), ("kizilay", "kizlay"), ("ataturk", "inonu"),
    ("1203/1", "1203/2"), ("1203/1", "1203/1"), ("15 temmuz", "15 temmz"), ("gazi mustafa kemal", "gazi mustafa kmal"),
    ("a", "b"), ("cumhuriyet", "cumhuriyt"), ("", "x"),
]

FOLDS = ["KIZILAY Mahallesi", "İĞNECİ ŞÜKRÜ Çağ", "i̇stanbul", "Âşık Î Û", "No12/A", "12a  ,1203/1. Sk.", "Çankaya-Ankara: 06420", "Ärzte ßtraße"]

FIELDS = [
    ("doors", ["OBJECTID", "MAHALLE_KODU", "MAHALLE_ADI", "YOL_ID", "YOL_ADI", "DIS_KAPI_NO", "IC_KAPI_NO", "ILCE", "IL_ADI", "POSTA_KODU", "BINA_ADI", "UAVT_KODU"]),
    ("doors", ["Ad", "Sokak", "No", "Mahalle", "İlçe", "Şehir", "PK"]),
    ("doors", ["csbm", "kapino", "mah_adi", "adres_kodu", "site_adi"]),
    ("streets", ["YOL_ID", "YOL_TIPI", "YOL_ADI", "MAHALLE", "ILCE_ADI"]),
    ("streets", ["ad", "uzunluk"]),
    ("neighbourhood", ["MAHALLE_KODU", "MAHALLE_ADI", "ILCE_ADI"]),
    ("district", ["ILCE_KODU", "ADI"]),
    ("province", ["IL", "NUFUS"]),
]


def found_json(cands, status):
    return {"status": status, "candidates": [
        {"kind": c["kind"], "score": c["score"], "at": c["at"], "label": c["label"], "texts": c["texts"], "order": c["order"]}
        for c in cands]}


def build_cases():
    scenes = {"metres": city(1.0, 500000.0, 4400000.0, 1.0), "millimetres": city(1000.0, 0.0, 0.0, 1000.0)}
    out = {"format": "kentos.geocode-cases", "version": 1,
           "note": "docs/adr/0229 §3–§9, §13.1; written by scripts/fixtures/geocode_cases.py from the ADR, without KentOS code",
           "folds": [{"text": t, "folded": fold(t), "tokens": [w for _, w in tokens(fold(t))]} for t in FOLDS],
           "parses": [{"text": t, "parsed": parse(t)} for t in PARSE],
           "similarities": [{"a": a, "b": b, "similarity": similarity(a, b)} for a, b in SIMILARITY],
           "fields": [{"kind": k, "fields": f, "guess": guess_doors(f) if k == "doors" else guess_streets(f) if k == "streets" else guess_areas(k, f)} for k, f in FIELDS],
           "scenes": []}
    for name, scene in scenes.items():
        loc = build(scene)
        s = {"name": name, "scene": scene, "minScore": 85, "limit": 20,
             "counts": {"doors": len(loc["doors"]), "streets": len(loc["groups"]), "lines": len(loc["lines"]),
                        "neighbourhoods": len(loc["hoods"]), "areas": len(loc["areas"]),
                        "emptyStreets": loc["emptyStreets"], "unreadDoors": loc["unreadDoors"]},
             "forward": [], "columns": [], "reverse": []}
        for text in FORWARD:
            cands, status = find(loc, text, parse(text), 85, 20)
            s["forward"].append({"text": text, **found_json(cands, status)})
        for cols in COLUMNS:
            cands, status = find(loc, "", from_columns(cols), 85, 20)
            s["columns"].append({"columns": cols, **found_json(cands, status)})
        scale = 1.0 if name == "metres" else 1000.0
        ox, oy = (500000.0, 4400000.0) if name == "metres" else (0.0, 0.0)
        for (x, y), reach in REVERSE:
            p = (ox + x * scale, oy + y * scale)
            s["reverse"].append({"at": [p[0], p[1]], "reach": reach, "place": reverse(loc, p, reach)})
        out["scenes"].append(s)
    return out


def main():
    text = json.dumps(build_cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} betiğin yazdığıyla aynı değil (--check olmadan çalıştırın ve farkı okuyun)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
