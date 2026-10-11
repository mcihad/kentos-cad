#!/usr/bin/env python3
"""Independent reference of templates (docs/adr/0225 §6, §4).

Writes fixtures/expression/v2/template.json from the ADR's rules alone, with
Python's standard library and no KentOS code. The expression core
(`kentos_expression::template`, its unit test `the_shared_cases_pass`) and the
web through WASM (`apps/web/src/model/infoTemplates.wasm.test.ts`) must give
exactly the expected pieces, errors, texts and kinds for every case.

The rules:

1. A template is read in UTF-16 code units; positions are 1-based.
2. Outside an expression `{{` is a plain `{` and `}}` a plain `}`; a lone `}`
   is an error at it.
3. `{` opens an expression, which ends at the first `}` outside a quoted text
   (`'…'` or `"…"`, a doubled quote inside is one quote) and outside a
   bracketed field name (`[…]`, up to the first `]`). An expression that does
   not close, or meets another `{` first, is an error at its `{`; one that is
   empty or only white space (JavaScript's `trim`) an error at its `{`.
4. An expression that is only `$enlem` or `$boylam` (trimmed) is the object's
   latitude or longitude; any other is the language's, its place the unit
   after its `{`.
5. A template whose first piece is text starting (after JavaScript's `trim`)
   with `http://`, `https://` or `mailto:` in any case is an address's: every
   value put in is percent-encoded (each UTF-8 byte but A–Z, a–z, 0–9 and
   `-._~` as %XX, upper case).
6. Filling puts each expression's value in order (none: nothing) and the
   place's latitude and longitude with eight decimals by the display rule
   (docs/adr/0149: the exact binary value rounded a half away from zero; no
   minus sign when no digit but 0 is written); without a place, nothing.
7. A filled target trimmed (JavaScript's `trim`) is: refused when empty; an
   address when it starts with http://, https:// or mailto: in any case,
   something follows and it has no white space (Unicode White_Space), else
   refused; refused when it starts with another scheme (a letter, then
   letters, digits, `+`, `-` or `.`, then `:`) unless that is a drive letter
   followed by `\\` or `/`; else a file.

    python3 scripts/fixtures/template_cases.py           # write the fixture
    python3 scripts/fixtures/template_cases.py --check   # compare
"""

import json
import sys
from decimal import ROUND_HALF_UP, Decimal, getcontext
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/expression/v2/template.json"

getcontext().prec = 200

# JavaScript's `trim`: WhiteSpace and LineTerminator (ECMA-262 §12.2, §12.3).
JS_SPACE = set("\t\n\v\f\r \u00a0\u1680\u2028\u2029\u202f\u205f\u3000\ufeff") | {chr(c) for c in range(0x2000, 0x200B)}
# Unicode White_Space (Rust's `char::is_whitespace`).
WHITE_SPACE = set("\t\n\v\f\r \u0085\u00a0\u1680\u2028\u2029\u202f\u205f\u3000") | {chr(c) for c in range(0x2000, 0x200B)}

UNCLOSED = "“{” kapanmamış: ifadeyi “}” ile kapatın (düz ayraç için {{ yazın)."
LONE = "“}” tek başına; düz ayraç için }} yazın."
EMPTY = "“{}” boş: araya bir ifade yazın."


def units(s):
    b = s.encode("utf-16-le")
    return [b[i] | (b[i + 1] << 8) for i in range(0, len(b), 2)]


def text_of(u):
    return b"".join(bytes([c & 0xFF, c >> 8]) for c in u).decode("utf-16-le", errors="replace")


def ascii_lower(s):
    return "".join(c.lower() if "A" <= c <= "Z" else c for c in s)


def js_trim(s):
    a, b = 0, len(s)
    while a < b and s[a] in JS_SPACE:
        a += 1
    while b > a and s[b - 1] in JS_SPACE:
        b -= 1
    return s[a:b]


def parse(t):
    u = units(t)
    pieces, plain, i = [], [], 0
    O, C = ord("{"), ord("}")

    def flush():
        if plain:
            pieces.append({"kind": "text", "text": text_of(plain)})
            plain.clear()

    while i < len(u):
        c = u[i]
        if c == O and i + 1 < len(u) and u[i + 1] == O:
            plain.append(O)
            i += 2
            continue
        if c == C and i + 1 < len(u) and u[i + 1] == C:
            plain.append(C)
            i += 2
            continue
        if c == C:
            return {"error": {"at": i + 1, "message": LONE}}
        if c == O:
            j, end = i + 1, None
            while j < len(u):
                d = u[j]
                if d in (ord("'"), ord('"')):
                    k = j + 1
                    while True:
                        if k >= len(u):
                            k = None
                            break
                        if u[k] == d:
                            if k + 1 < len(u) and u[k + 1] == d:
                                k += 2
                                continue
                            break
                        k += 1
                    if k is None:
                        break
                    j = k + 1
                    continue
                if d == ord("["):
                    try:
                        j = u.index(ord("]"), j) + 1
                    except ValueError:
                        break
                    continue
                if d == C:
                    end = j
                    break
                if d == O:
                    break
                j += 1
            if end is None:
                return {"error": {"at": i + 1, "message": UNCLOSED}}
            source = text_of(u[i + 1:end])
            inner = js_trim(source)
            if not inner:
                return {"error": {"at": i + 1, "message": EMPTY}}
            flush()
            if inner == "$enlem":
                pieces.append({"kind": "lat"})
            elif inner == "$boylam":
                pieces.append({"kind": "lon"})
            else:
                pieces.append({"kind": "expr", "text": source, "at": i + 2})
            i = end + 1
            continue
        plain.append(c)
        i += 1
    flush()
    lead = pieces[0]["text"] if pieces and pieces[0]["kind"] == "text" else ""
    lower = ascii_lower(js_trim(lead))
    encoded = any(lower.startswith(s) for s in ("http://", "https://", "mailto:"))
    return {"pieces": pieces, "encoded": encoded}


def encode(s):
    keep = set(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~")
    return "".join(chr(b) if b in keep else f"%{b:02X}" for b in s.encode("utf-8"))


def degrees(v):
    y = Decimal(abs(v)).quantize(Decimal(1).scaleb(-8), rounding=ROUND_HALF_UP)
    text = format(y, "f")
    return "-" + text if v < 0 and any(c in "123456789" for c in text) else text


def fill(t, values, place):
    p = parse(t)
    assert "error" not in p, t
    out, k = [], 0
    put = (lambda v: out.append(encode(v))) if p["encoded"] else out.append
    for piece in p["pieces"]:
        if piece["kind"] == "text":
            out.append(piece["text"])
        elif piece["kind"] == "expr":
            v = values[k] if k < len(values) else None
            if v is not None:
                put(v)
            k += 1
        elif place is not None:
            put(degrees(place[0] if piece["kind"] == "lat" else place[1]))
    return "".join(out)


def target(t):
    s = js_trim(t)
    if not s:
        return "refused"
    lower = ascii_lower(s)
    for scheme in ("http://", "https://", "mailto:"):
        if lower.startswith(scheme):
            rest = s[len(scheme):]
            return "refused" if not rest or any(c in WHITE_SPACE for c in s) else "address"
    colon = s.find(":")
    if colon > 0 and s[0].isascii() and s[0].isalpha() and all(c.isascii() and (c.isalnum() or c in "+-.") for c in s[:colon]):
        drive = colon == 1 and len(s) > 2 and s[2] in "\\/"
        if not drive:
            return "refused"
    return "file"


PARSE = [
    "",
    "Düz metin",
    "Parsel {Ada}/{Parsel}",
    "{Ada}",
    "{{düz}}",
    "{{{Ada}}}",
    "a {{ b }} c",
    "{ $enlem },{$boylam}",
    "https://www.google.com/maps?q={$enlem},{$boylam}",
    "  HTTPS://a.org/{Ad}",
    "mailto:{Eposta}?subject=Parsel%20{Ada}",
    "ftp://a.org/{Ad}",
    "belgeler/{Ada}-{Parsel}.pdf",
    "{'}' || [a}b]}",
    "{\"x}\"\"y\" || 'a''}b'}",
    "{[Tapu alanı]} m²",
    "Alan: {yuvarla($alan, 2)} m²\nTapu: {[Tapu alanı]} m²",
    "😀{Ad}",
    "ç{Ad}😀{Soyad}",
    "{$enlemx}",
    "{ $boylam}",
    "{$alan}",
    "{@proje_adı}",
    "a {Ada",
    "a } b",
    "{Ada}}",
    "x{ }",
    "x{\u00a0\u3000}",
    "{}",
    "{a {b}",
    "{'a}",
    "{[a}",
    "{\"a}",
    "😀}",
    "}}{{",
    "{Ada}{Parsel}",
]

FILL = [
    ("Parsel {Ada}/{Parsel}", ["101", "5"], None),
    ("Parsel {Ada}/{Parsel}", [None, "5"], None),
    ("Parsel {Ada}/{Parsel}", ["101"], None),
    ("https://parselsorgu.example/{Ada}/{Parsel}", ["101", "5/A"], None),
    ("https://a.org/?q={Ad}", ["Ayşe & Ali ~-._"], None),
    ("https://a.org/{Ad}", ["😀"], None),
    ("mailto:{Eposta}", ["a@b.org"], None),
    ("belgeler/{Ada} {Parsel}.pdf", ["1 2", "ş"], None),
    ("{Adres}", ["https://kurum.example/a b"], None),
    ("https://www.google.com/maps?q={$enlem},{$boylam}", [], [41.0123456789, 28.987654321]),
    ("https://www.google.com/maps?q={$enlem},{$boylam}", [], [-33.8688197, -0.000000001]),
    ("https://www.google.com/maps?q={$enlem},{$boylam}", [], None),
    ("Yer: {$enlem} {$boylam}", [], [39.925533, 32.866287]),
    ("Yer: {$enlem}", [], [0.000000005, 0.0]),
    ("Yer: {$enlem}", [], [-0.000000004, 0.0]),
    ("{{{Ada}}}", ["7"], None),
    ("Düz", [], None),
]

TARGET = [
    "https://a.org",
    "http://a.org/x?y=1",
    "  https://a.org  ",
    "HTTPS://A.ORG",
    "mailto:a@b.org",
    "MailTo:a@b.org",
    "https://",
    "mailto:",
    "https://a.org/a b",
    "https://a.org/a\u00a0b",
    "javascript:alert(1)",
    "JavaScript:alert(1)",
    "data:text/html,<b>x</b>",
    "file:///etc/passwd",
    "ftp://a.org",
    "C:\\belgeler\\a.pdf",
    "c:/belgeler/a.pdf",
    "C:belgeler",
    "belgeler/101-5.pdf",
    "/home/kullanici/a.pdf",
    "../a.pdf",
    "a:b",
    "1a:b",
    "ç:b",
    "",
    "   ",
    "\u3000",
]


def build():
    return {
        "format": "kentos.template-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/template_cases.py from docs/adr/0225 §6 alone; positions are 1-based UTF-16.",
        "parse": [{"template": t, **parse(t)} for t in PARSE],
        "fill": [{"template": t, "values": v, **({"place": p} if p is not None else {}), "text": fill(t, v, p)} for t, v, p in FILL],
        "target": [{"target": t, "kind": target(t)} for t in TARGET],
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT} güncel değil: python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın ve farkı okuyun")
        print(f"Şablon durumları tutarlı: {len(PARSE)} bölme, {len(FILL)} doldurma, {len(TARGET)} hedef.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı")


if __name__ == "__main__":
    main()
