#!/usr/bin/env python3
"""Where an agent's call names a field the drawing hides from agents (docs/adr/0743 §1), written from the ADR
without KentOS code: fixtures/agent-veil/v1/oracle.json. Each case gives the hidden fields' names, a call's input
and where it names one: the path (`a.b[2]`) and the field, or none.

The rules (ADR 0743 §1):
- the input is walked in its own order: an object's keys sorted by their UTF-8 bytes, an array's items in order;
  the first place found is told;
- a key that is a hidden field's name names it, wherever it is (an attribute map's key too);
- under the keys whose texts a call writes (`attrs`, `values`, `value`, `text`, `content`, `note`, `notes`,
  `comment`, `path`) the texts are content: only keys are looked at below them;
- any other text names a hidden field when it is the field's name; or, when it holds `{`, a template whose `{…}`
  expressions read the field; or an expression that reads it (the first hidden name, in the names' order);
- an expression reads a field by a bracketed name (`[TC]`, trimmed) or by a bare word that is neither called
  (`Malik(`) nor a keyword (`ve`, `veya`, `değil`, `and`, `or`, `not`, `doğru`, `yanlış`, `boş`, `true`, `false`,
  `null`); a text in quotes (`'…'`, `"…"`, a doubled quote inside), a number, a `$` or `@` name and a comment
  (`//` to the line's end, `/* … */`) read none.

`--check` compares the file with what this writes; without it the file is written.
"""

import json
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures/agent-veil/v1/oracle.json"
WRITTEN = {"attrs", "values", "value", "text", "content", "note", "notes", "comment", "path"}
KEYWORDS = {"VE", "AND", "VEYA", "OR", "DEGIL", "NOT", "DOGRU", "TRUE", "YANLIS", "FALSE", "BOS", "NULL"}
FOLD = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                      "ş": "S", "Ş": "S", "ü": "U", "Ü": "U"})


def fold(word: str) -> str:
    return word.translate(FOLD).upper()


def word_char(c: str) -> bool:
    return c.isalnum() or c == "_"


def fields_of(src: str) -> list[str]:
    """The fields an expression reads, each once, in the order they first come."""
    out: list[str] = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c.isspace():
            i += 1
        elif src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            j = src.find("*/", i + 2)
            i = n if j < 0 else j + 2
        elif c.isdigit():
            while i < n and (src[i].isdigit() or src[i] == "."):
                i += 1
        elif c in "'\"":
            j = i + 1
            while j < n:
                if src[j] == c:
                    if j + 1 < n and src[j + 1] == c:
                        j += 2
                        continue
                    j += 1
                    break
                j += 1
            i = j
        elif c == "[":
            j = src.find("]", i)
            name = (src[i + 1:] if j < 0 else src[i + 1:j]).strip()
            i = n if j < 0 else j + 1
            if name and name not in out:
                out.append(name)
        elif c in "$@":
            i += 1
            while i < n and word_char(src[i]):
                i += 1
        elif word_char(c):
            j = i
            while j < n and word_char(src[j]):
                j += 1
            word = src[i:j]
            k = j
            while k < n and src[k].isspace():
                k += 1
            called = k < n and src[k] == "("
            if not called and fold(word) not in KEYWORDS and word not in out:
                out.append(word)
            i = j
        else:
            i += 1
    return out


def template_exprs(s: str) -> list[str] | None:
    """A template's `{…}` expressions; none when a brace does not close."""
    out = []
    i = 0
    while True:
        a = s.find("{", i)
        if a < 0:
            return out
        b = s.find("}", a)
        if b < 0:
            return None
        out.append(s[a + 1:b])
        i = b + 1


def named_by(s: str, names: list[str]) -> str | None:
    if s in names:
        return s
    if "{" in s:
        exprs = template_exprs(s)
        if exprs is not None:
            for name in names:
                if any(name in fields_of(e) for e in exprs):
                    return name
    fields = fields_of(s)
    return next((name for name in names if name in fields), None)


def named_at(v, path: str, names: list[str], content: bool):
    if isinstance(v, str):
        if content:
            return None
        f = named_by(v, names)
        return None if f is None else [path, f]
    if isinstance(v, list):
        for i, item in enumerate(v):
            found = named_at(item, f"{path}[{i}]", names, content)
            if found:
                return found
        return None
    if isinstance(v, dict):
        for k in sorted(v, key=lambda k: k.encode("utf-8")):
            at = k if not path else f"{path}.{k}"
            if k in names:
                return [at, k]
            found = named_at(v[k], at, names, content or k in WRITTEN)
            if found:
                return found
    return None


N = ["Malik", "TC"]
N3 = ["Tapu No", "Malik", "TC"]

CASES = [
    ("Gizli alan anmayan sorgu", N, {"layer": "bina", "filter": "Ada = '101' ve Parsel > 3"}),
    ("Süzgeç gizli alanı okuyor", N, {"layer": "bina", "filter": "Malik = 'Ali'"}),
    ("İşlevin içinde", N, {"write": {"kind": "calculate", "field": "Ada", "expression": "upper(Malik)"}}),
    ("Köşeli adla", N, {"write": {"kind": "calculate", "field": "Ada", "expression": "[ TC ] || ''"}}),
    ("Tırnaktaki ad bir metindir", N, {"write": {"kind": "calculate", "field": "Ada", "expression": "'Malik'"}}),
    ("Çift tırnaktaki ad da metindir", N, {"expression": "\"TC\" || Ada"}),
    ("Çağrılan sözcük işlevdir", N, {"expression": "Malik(1)"}),
    ("Daha uzun bir sözcük başka alandır", N, {"expression": "Malik_eski || TCK"}),
    ("Anahtar sözcükler alan değildir", N, {"expression": "boş veya doğru ve değil yanlış"}),
    ("Değişkenler alan değildir", N, {"expression": "$alan > 10 ve @proje_adi = 'x'"}),
    ("Açıklamadaki ad okunmaz", N, {"expression": "Ada // Malik\n|| '/' /* TC */"}),
    ("Sıralama alanı adıyla", N, {"order": [{"field": "Ada"}, {"field": "TC", "descending": True}]}),
    ("Alan listesi", N, {"fields": ["Ada", "Malik"]}),
    ("Boşluklu ad yalnız tam yazılışıyla", N3, {"fields": ["Tapu No"]}),
    ("Boşluklu ad köşeli ifadede", N3, {"filter": "[Tapu No] gibi '12%'"}),
    ("Boşluklu adın sözcükleri ayrı alanlardır", N3, {"filter": "Tapu = 1 ve No = 2"}),
    ("Şablonun ifadesi", N, {"labels": {"template": "Ada {Ada} sahibi {Malik}"}}),
    ("Şablonun düz yazısı", N, {"labels": {"template": "{Ada}"}}),
    ("Özniteliğin değeri içerik", N, {"attrs": {"Ada": "Malik bilgisi yok"}}),
    ("Özniteliğin anahtarı gizli alan", N, {"layerId": "bina", "attrs": {"Ada": "103", "TC": "1"}}),
    ("Yazının içeriği okunmaz", N, {"text": "Malik ve TC", "content": {"runs": [{"text": "TC"}]}}),
    ("Not ve yol içerik", N, {"note": "Malik", "path": "/tmp/Malik.csv", "comment": "TC"}),
    ("İçerik altında anahtar yine sayılır", N, {"values": [{"Malik": "x"}]}),
    ("Dizinin ikinci öğesi", N, {"steps": [{"expression": "Ada"}, {"expression": "TC * 2"}]}),
    ("Anahtarlar bayt sırasıyla: ilk bulunan", N, {"zeta": "Malik", "alfa": "TC"}),
    ("Adların sırası: ilk gizli ad", N, {"expression": "TC || Malik"}),
    ("Türkçe büyük harfle anahtar sözcük", N, {"expression": "Ada VEYA Parsel DEĞİL boş"}),
    ("Kök bir metin", N, "Malik"),
    ("Sayılar ve mantık değerleri", N, {"limit": 10, "on": True, "ratio": 0.5, "none": None}),
    ("Gizli alan yok", [], {"filter": "Malik = 'Ali'"}),
]


def build() -> dict:
    cases = []
    for name, names, input_ in CASES:
        found = named_at(input_, "", names, False)
        cases.append({
            "name": name,
            "names": names,
            "input": input_,
            "named": None if found is None else {"path": found[0], "field": found[1]},
        })
    return {
        "format": "kentos.agent-oracle/1",
        "note": "Written by scripts/fixtures/agent_oracle_cases.py from docs/adr/0743 §1 without KentOS code.",
        "cases": cases,
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT} güncel değil; betiği --check olmadan çalıştırın.", file=sys.stderr)
            return 1
        print(f"{OUT}: {len(CASES)} durum güncel.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT}: {len(CASES)} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
