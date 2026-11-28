#!/usr/bin/env python3
"""How far a file of stored numbers moved, number by number; and the move written into the file when it is small.

When a rule of the numeric base changes (docs/adr/9403: a length is now the nearest double), the answers a test keeps
on disk move in their last places. Such a file is rewritten only after this has read the difference: everything but
floating-point numbers must be the same, and no number may have moved further than the limit. The limit is in last
places (ulps) of the old value. A number that went through further arithmetic may ask for a bound of its own: a
relative one (--rel), or an absolute one in the number's unit (--abs; a coordinate of millions of metres moved by a
nanometre is five last places away and 2·10⁻¹⁶ of itself, and a millimetre is 2·10⁻¹⁰ of itself: for coordinates
and angles the absolute bound is the one that means something). The largest move is printed either way.

Two versions of a file, read:

  git show HEAD:fixtures/x.json | python3 scripts/fixtures/ulp_diff.py - fixtures/x.json            # at most 1 ulp
  git show HEAD:fixtures/x.json | python3 scripts/fixtures/ulp_diff.py - fixtures/x.json --ulps 4
  python3 scripts/fixtures/ulp_diff.py old.json new.json --rel 1e-12
  python3 scripts/fixtures/ulp_diff.py old.json new.json --abs 1e-8
  git show HEAD:fixtures/x.ifc | python3 scripts/fixtures/ulp_diff.py - fixtures/x.ifc --text   # a text file's numbers

A file recorded by a program that is gone (the web's recorders), patched: its frozen test, run with
KENTOS_WRITE_FROZEN=<folder>, writes what it computes now as a list of patches, `[{"at": [path…], "value": …}, …]`;
this writes those values into the file where they differ, each number that moved as its fewest digits (JavaScript's
form, the recorders'), every other byte as it was:

  python3 scripts/fixtures/ulp_diff.py fixtures/x.json --apply .run/frozen/x.patches.json

Nothing is written when a number moved further than the limit or anything but a number differs. Exit status 0 when
the two agree within the limit, 1 otherwise. Numbers written as text ("NaN", "-0") are compared as text.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
from decimal import Decimal


def ordinal(x: float) -> int:
    """The doubles in order as integers: neighbours differ by one, and −0 and +0 are the same place."""
    bits = struct.unpack("<q", struct.pack("<d", x))[0]
    return bits if bits >= 0 else -(bits & 0x7FFFFFFFFFFFFFFF)


def js_number(x: float) -> str:
    """A finite double as JavaScript writes it: the fewest digits that read back as it (ECMA-262, Number::toString)."""
    if x == 0:
        return "0"
    sign, digits, exponent = Decimal(repr(abs(x))).as_tuple()
    text = "".join(map(str, digits)).rstrip("0")
    n = len(digits) + exponent  # the value is 0.text × 10ⁿ
    k = len(text)
    if k <= n <= 21:
        body = text + "0" * (n - k)
    elif 0 < n <= 21:
        body = text[:n] + "." + text[n:]
    elif -6 < n <= 0:
        body = "0." + "0" * -n + text
    else:
        e = n - 1
        mantissa = text[0] + ("." + text[1:] if k > 1 else "")
        body = f"{mantissa}e{'+' if e > 0 else '-'}{abs(e)}"
    return ("-" if x < 0 else "") + body


class Reading:
    def __init__(self, ulps: int, rel: float, abs_: float = 0.0) -> None:
        self.ulps, self.rel, self.abs = ulps, rel, abs_
        self.numbers = self.moved = self.worst_ulps = 0
        self.worst_rel = self.worst_abs = 0.0
        self.worst_at = ""
        self.over: list[str] = []
        self.other: list[str] = []

    def number(self, a: float, b: float, at: str) -> bool:
        """Counts the pair; true when the number moved (within the limit or not)."""
        self.numbers += 1
        if a == b or (math.isnan(a) and math.isnan(b)):
            return False
        self.moved += 1
        if not (math.isfinite(a) and math.isfinite(b)):
            self.over.append(f"{at}: {a!r} → {b!r}")
            return True
        steps = abs(ordinal(a) - ordinal(b))
        rel = abs(a - b) / max(abs(a), abs(b))
        if steps > self.worst_ulps:
            self.worst_ulps, self.worst_at = steps, f"{at}: {a!r} → {b!r}"
        self.worst_rel = max(self.worst_rel, rel)
        self.worst_abs = max(self.worst_abs, abs(a - b))
        if steps > self.ulps and not rel <= self.rel and not abs(a - b) <= self.abs:
            self.over.append(f"{at}: {a!r} → {b!r} ({steps} ulp, bağıl {rel:.3g}, mutlak {abs(a - b):.3g})")
        return True

    def walk(self, a, b, at: str) -> None:
        if isinstance(a, bool) or isinstance(b, bool) or a is None or b is None:
            if a is not b:
                self.other.append(f"{at}: {a!r} → {b!r}")
        elif isinstance(a, (int, float)) and isinstance(b, (int, float)):
            if isinstance(a, int) and isinstance(b, int):
                if a != b:
                    self.other.append(f"{at}: {a} → {b}")
            else:
                self.number(float(a), float(b), at)
        elif isinstance(a, dict) and isinstance(b, dict):
            if list(a) != list(b):
                self.other.append(f"{at}: keys {sorted(set(a) ^ set(b))[:6]} or their order")
            for k in a:
                if k in b:
                    self.walk(a[k], b[k], f"{at}.{k}")
        elif isinstance(a, list) and isinstance(b, list):
            if len(a) != len(b):
                self.other.append(f"{at}: {len(a)} → {len(b)} items")
            for i, (x, y) in enumerate(zip(a, b)):
                self.walk(x, y, f"{at}[{i}]")
        elif a != b:
            self.other.append(f"{at}: {str(a)[:60]!r} → {str(b)[:60]!r}")

    def line(self, name: str) -> str:
        return (
            f"{name}: {self.numbers} sayı, {self.moved} değişti; en çok {self.worst_ulps} ulp "
            f"(bağıl {self.worst_rel:.3g}, mutlak {self.worst_abs:.3g}); sınırı aşan {len(self.over)}, "
            f"sayı dışı fark {len(self.other)}"
        )

    def details(self) -> None:
        if self.worst_at:
            print(f"  en uzak: {self.worst_at}")
        for text in (self.over + self.other)[:20]:
            print(f"  {text}")


# ── A file's text with every value's place in it ─────────────────────────────────────────────────────────────────

class Node:
    """A value of the text: its kind ('object', 'array', 'number', 'other'), where it is, and what it holds."""

    __slots__ = ("kind", "start", "end", "items")

    def __init__(self, kind: str, start: int, end: int, items=None) -> None:
        self.kind, self.start, self.end, self.items = kind, start, end, items


_WS = " \t\r\n"
_decoder = json.JSONDecoder()


def scan(text: str, i: int = 0) -> Node:
    """The value that starts at or after `i`."""
    while text[i] in _WS:
        i += 1
    c = text[i]
    if c == "{":
        items: dict[str, Node] = {}
        j = i + 1
        while True:
            while text[j] in _WS:
                j += 1
            if text[j] == "}":
                return Node("object", i, j + 1, items)
            key, j = _decoder.raw_decode(text, j)
            while text[j] in _WS:
                j += 1
            assert text[j] == ":", f"':' expected at {j}"
            child = scan(text, j + 1)
            items[key] = child
            j = child.end
            while text[j] in _WS:
                j += 1
            if text[j] == ",":
                j += 1
    if c == "[":
        elements: list[Node] = []
        j = i + 1
        while True:
            while text[j] in _WS:
                j += 1
            if text[j] == "]":
                return Node("array", i, j + 1, elements)
            child = scan(text, j)
            elements.append(child)
            j = child.end
            while text[j] in _WS:
                j += 1
            if text[j] == ",":
                j += 1
    if c == "-" or c.isdigit():
        j = i + 1
        while j < len(text) and text[j] in "+-.eE0123456789":
            j += 1
        return Node("number", i, j)
    _, j = _decoder.raw_decode(text, i)  # a string, true, false or null
    return Node("other", i, j)


def reconcile(text: str, node: Node, new, at: str, r: Reading, edits: list[tuple[int, int, str]]) -> None:
    """The file's value at `node` against what is computed now: the numbers that moved go into `edits`."""
    old_text = text[node.start:node.end]
    if node.kind == "object":
        # The keys' order is the file's: a patch's is its writer's (often sorted), and says nothing.
        if not isinstance(new, dict) or set(node.items) != set(new):
            r.other.append(f"{at}: an object's keys")
            return
        for key, child in node.items.items():
            reconcile(text, child, new[key], f"{at}.{key}", r, edits)
    elif node.kind == "array":
        if not isinstance(new, list) or len(node.items) != len(new):
            r.other.append(f"{at}: {len(node.items)} → {len(new) if isinstance(new, list) else new!r} items")
            return
        for i, child in enumerate(node.items):
            reconcile(text, child, new[i], f"{at}[{i}]", r, edits)
    elif node.kind == "number":
        if isinstance(new, bool) or not isinstance(new, (int, float)):
            r.other.append(f"{at}: {old_text} → {new!r}")
        elif r.number(float(old_text), float(new), at):
            edits.append((node.start, node.end, js_number(float(new))))
    else:
        old = json.loads(old_text)
        same = old == new and type(old) is type(new)
        # A number the file writes as text ("NaN", "-0") and the program gives as that number.
        if not same and isinstance(old, str) and isinstance(new, float):
            same = (old == "NaN" and math.isnan(new)) or (old == "-0" and new == 0 and math.copysign(1, new) < 0)
        if not same:
            r.other.append(f"{at}: {old_text[:60]} → {str(new)[:60]!r}")


def follow(root: Node, path: list) -> Node | None:
    node = root
    for step in path:
        if node.kind == "object" and isinstance(step, str) and step in node.items:
            node = node.items[step]
        elif node.kind == "array" and isinstance(step, int) and 0 <= step < len(node.items):
            node = node.items[step]
        else:
            return None
    return node


def apply(path: str, patches_path: str, r: Reading) -> int:
    text = open(path, encoding="utf-8").read()
    patches = json.load(open(patches_path, encoding="utf-8"))
    root = scan(text)
    edits: list[tuple[int, int, str]] = []
    for patch in patches:
        at = "".join(f"[{s}]" if isinstance(s, int) else f".{s}" for s in patch["at"])
        node = follow(root, patch["at"])
        if node is None:
            r.other.append(f"{at}: the file has no such place")
            continue
        reconcile(text, node, patch["value"], at, r, edits)
    print(r.line(path) + f"; {len(patches)} yama")
    r.details()
    if r.over or r.other:
        print("  Dosya yazılmadı.")
        return 1
    out = text
    for start, end, new in sorted(edits, reverse=True):
        out = out[:start] + new + out[end:]
    # What was written reads back as what was computed.
    written = json.loads(out)
    for patch in patches:
        value = written
        for step in patch["at"]:
            value = value[step]
        assert same(value, patch["value"]), ("the patched file does not read back", patch["at"])
    if out != text:
        open(path, "w", encoding="utf-8").write(out)
    print(f"  {len(edits)} sayı yazıldı.")
    return 0


def same(old, new) -> bool:
    """The file's value is the computed one: numbers by value (text for NaN and −0 as the number), keys in any order."""
    if isinstance(old, dict):
        return isinstance(new, dict) and set(old) == set(new) and all(same(v, new[k]) for k, v in old.items())
    if isinstance(old, list):
        return isinstance(new, list) and len(old) == len(new) and all(same(a, b) for a, b in zip(old, new))
    if isinstance(old, bool) or isinstance(new, bool) or old is None or new is None:
        return old is new
    if isinstance(old, (int, float)) and isinstance(new, (int, float)):
        return float(old) == float(new)
    if isinstance(old, str) and isinstance(new, float):
        return (old == "NaN" and math.isnan(new)) or (old == "-0" and new == 0 and math.copysign(1, new) < 0)
    return old == new


# A number of a text file: a decimal with a point or an exponent (a whole number is a count, an id or a code).
_REAL = __import__("re").compile(r"(?<![\w.#])-?(?:\d+\.\d*|\.\d+)(?:[eE][-+]?\d+)?(?![\w.])|(?<![\w.#])-?\d+[eE][-+]?\d+(?![\w.])")


def read_text(path: str) -> str:
    # The bytes as they are: no line ending is translated on one side and not on the other.
    data = sys.stdin.buffer.read() if path == "-" else open(path, "rb").read()
    return data.decode("utf-8", errors="surrogateescape")


def text_walk(old: str, new: str, r: Reading) -> None:
    """Two versions of a text file (IFC, DXF, XML): everything between the real numbers must be the same."""
    a_parts, b_parts = _REAL.split(old), _REAL.split(new)
    a_nums, b_nums = _REAL.findall(old), _REAL.findall(new)
    if len(a_nums) != len(b_nums) or a_parts != b_parts:
        at = next((i for i, (x, y) in enumerate(zip(a_parts, b_parts)) if x != y), min(len(a_parts), len(b_parts)))
        line = old.count("\n", 0, sum(len(p) for p in a_parts[:at]) + sum(len(n) for n in a_nums[:at])) + 1
        r.other.append(f"satır {line} dolayında sayı olmayan bir fark ({len(a_nums)} ve {len(b_nums)} sayı)")
        return
    offset = 0
    for i, (x, y) in enumerate(zip(a_nums, b_nums)):
        offset += len(a_parts[i])
        if x != y:
            r.number(float(x), float(y), f"satır {old.count(chr(10), 0, offset) + 1}")
        else:
            r.numbers += 1
        offset += len(x)


def load(path: str):
    text = sys.stdin.read() if path == "-" else open(path, encoding="utf-8").read()
    return json.loads(text)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("old")
    ap.add_argument("new", nargs="?")
    ap.add_argument("--apply", metavar="PATCHES", help="write a frozen test's patches into `old`")
    ap.add_argument("--ulps", type=int, default=1, help="how many last places a number may move (default 1)")
    ap.add_argument("--rel", type=float, default=0.0, help="or this relative difference, for derived values")
    ap.add_argument("--abs", type=float, default=0.0, dest="abs_", help="or this absolute difference, in the number's unit")
    ap.add_argument("--quiet", action="store_true", help="one line")
    ap.add_argument("--text", action="store_true", help="two text files (IFC, DXF, XML): their real numbers compared")
    args = ap.parse_args()
    r = Reading(args.ulps, args.rel, args.abs_)
    if args.apply:
        return apply(args.old, args.apply, r)
    if args.new is None:
        ap.error("two files to compare, or --apply")
    if args.text:
        text_walk(read_text(args.old), read_text(args.new), r)
    else:
        r.walk(load(args.old), load(args.new), "")
    print(r.line(args.new))
    if not args.quiet:
        r.details()
    return 1 if r.over or r.other else 0


if __name__ == "__main__":
    sys.exit(main())
