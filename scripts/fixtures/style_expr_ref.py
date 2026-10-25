"""Stil dosyalarında işlevli ifadeler (docs/adr/0635 §6), KentOS kodu olmadan: KentOS'un ifadesi ile QGIS'inki arasında
sözcük sözcük çeviri. İki dil işleçleri ve QGIS'in anahtar sözcüklerini paylaşır (KentOS `AND`, `CASE … END`, `IS NULL`,
`IN`, `BETWEEN`, `LIKE`'ı kendi sözcüğü sayar); alanın yazılışı (`[ad]` ve `"ad"`; KentOS'ta `"…"` yazı, QGIS'te alan) ve
işlevlerin adları ayrıdır. Yalnız iki dilde aynı anlama gelen işlevler (ADR 0600'ün QGIS'le denetlediği) geçer; başka bir
işlev, `$` ya da `@` değeri ya da bir yanın bilmediği işleç olan ifade geçmez.
"""

from __future__ import annotations

FUNCTIONS = [("yuvarla", "round"), ("metin", "to_string"), ("sayı", "to_real"), ("tamsayı", "to_int"),
             ("mutlak", "abs"), ("min", "min"), ("max", "max"), ("büyük", "upper"), ("küçük", "lower"),
             ("kırp", "trim"), ("uzunluk", "length"), ("parça", "substr"), ("doldur", "lpad"), ("sağdoldur", "rpad"),
             ("değiştir", "replace"), ("başlar", "starts_with"), ("eğer", "if"), ("varsayılan", "coalesce"),
             ("kök", "sqrt"), ("tavan", "ceil"), ("taban", "floor"), ("pi", "pi"), ("ln", "ln"), ("log10", "log10"),
             ("log", "log"), ("üstel", "exp"), ("sin", "sin"), ("cos", "cos"), ("tan", "tan"), ("asin", "asin"),
             ("acos", "acos"), ("atan", "atan"), ("atan2", "atan2"), ("derece", "degrees"), ("radyan", "radians"),
             ("sol", "left"), ("sağ", "right"), ("bul", "strpos"), ("birleştir", "concat")]

FOLD = {"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "I": "I", "i": "I", "İ": "I", "ö": "O", "Ö": "O",
        "ş": "S", "Ş": "S", "ü": "U", "Ü": "U"}

KEYWORDS = {"VE": "AND", "AND": "AND", "VEYA": "OR", "OR": "OR", "DEGIL": "NOT", "NOT": "NOT", "DOGRU": "TRUE",
            "TRUE": "TRUE", "YANLIS": "FALSE", "FALSE": "FALSE", "DURUM": "CASE", "CASE": "CASE", "EGER": "WHEN",
            "WHEN": "WHEN", "ISE": "THEN", "THEN": "THEN", "YOKSA": "ELSE", "ELSE": "ELSE", "SON": "END",
            "END": "END", "ICINDE": "IN", "IN": "IN", "ARASINDA": "BETWEEN", "BETWEEN": "BETWEEN", "GIBI": "LIKE",
            "LIKE": "LIKE", "BENZER": "ILIKE", "ILIKE": "ILIKE", "IS": "IS", "BOS": "NULL", "NULL": "NULL"}

BARE_KEYWORDS = {"ve", "veya", "değil", "boş", "gibi", "arasında", "and", "or", "not", "is", "null", "like", "between",
                 "doğru", "yanlış", "true", "false"}

OPS = ["<=", ">=", "!=", "<>", "==", "||", "=", "<", ">", "+", "-", "*", "/", "%", "(", ")", ",", "^"]


def fold(w):
    return "".join(FOLD.get(c, c.upper() if c.isascii() else c) for c in w)


def function(w):
    f = fold(w)
    for k, q in FUNCTIONS:
        if fold(k) == f or fold(q) == f:
            return k, q
    return None


def keyword(w):
    return KEYWORDS.get(fold(w))


def bare_field(w):
    return bool(w) and (w[0].isalpha() or w[0] == "_") and all(c.isalnum() or c == "_" for c in w) \
        and w.lower() not in BARE_KEYWORDS


def tokens(src, qgis):
    c = src
    out = []
    i = 0

    def quoted(i, q):
        s = []
        j = i + 1
        while True:
            if j >= len(c):
                return None
            x = c[j]
            if x == q:
                if j + 1 < len(c) and c[j + 1] == q:
                    s.append(q)
                    j += 2
                    continue
                return "".join(s), j + 1
            s.append(x)
            j += 1

    while i < len(c):
        ch = c[i]
        if ch.isspace():
            i += 1
        elif not qgis and ch == "[":
            j = c.find("]", i + 1)
            if j < 0:
                return None
            name = c[i + 1:j].strip()
            if not name:
                return None
            out.append(("field", name))
            i = j + 1
        elif ch in ("'", '"'):
            q = quoted(i, ch)
            if q is None:
                return None
            if qgis and ch == '"':
                if not q[0]:
                    return None
                out.append(("field", q[0]))
            else:
                out.append(("str", q[0]))
            i = q[1]
        elif ch.isdigit() and ch.isascii() or (ch == "." and i + 1 < len(c) and c[i + 1].isdigit()):
            j = i + 1
            while j < len(c) and ((c[j].isdigit() and c[j].isascii()) or c[j] == "."
                                  or (c[j] in "eE" and j + 1 < len(c) and (c[j + 1].isdigit() or c[j + 1] in "+-"))):
                if c[j] in "eE":
                    j += 1
                j += 1
            text = c[i:j]
            try:
                float(text)
            except ValueError:
                return None
            out.append(("num", text))
            i = j
        elif ch.isalpha() or ch == "_":
            j = i + 1
            while j < len(c) and (c[j].isalnum() or c[j] == "_"):
                j += 1
            out.append(("word", c[i:j]))
            i = j
        else:
            op = next((o for o in OPS if c.startswith(o, i)), None)
            if op is None:
                return None
            out.append(("op", op))
            i += len(op)
    return out or None


def quote(s, q):
    return q + s.replace(q, q + q) + q


def join(parts):
    s = ""
    prev = None
    for p, fn in parts:
        tight = prev is None or prev[0] == "(" or p == ")" or p == "," or (p == "(" and prev[1])
        if not tight:
            s += " "
        s += p
        prev = (p, fn)
    return s


def is_operand(t):
    if t is None:
        return False
    k, v = t
    if k in ("field", "str", "num"):
        return True
    if k == "op":
        return v == ")"
    return keyword(v) is None or keyword(v) in ("TRUE", "FALSE")


def to_qgis(kentos):
    toks = tokens(kentos, False)
    if toks is None:
        return None
    out = []
    i = 0
    while i < len(toks):
        k, v = toks[i]
        nxt = toks[i + 1] if i + 1 < len(toks) else None
        next_open = nxt == ("op", "(")
        if k == "field":
            out.append((quote(v, '"'), False))
        elif k == "str":
            out.append((quote(v, "'"), False))
        elif k == "num":
            out.append((v, False))
        elif k == "op":
            out.append(("=" if v == "==" else v, False))
        elif next_open and (keyword(v) is None or fold(v) == "EGER"):
            f = function(v)
            if f is None:
                return None
            out.append((f[1], True))
        else:
            kw = keyword(v)
            if kw == "NULL" and is_operand(toks[i - 1] if i > 0 else None):
                if nxt is not None and nxt[0] == "word" and keyword(nxt[1]) == "NOT":
                    i += 1
                    out.append(("IS NOT NULL", False))
                else:
                    out.append(("IS NULL", False))
            elif kw is not None:
                out.append((kw, False))
            elif bare_field(v):
                out.append((quote(v, '"'), False))
            else:
                return None
        i += 1
    return join(out)


def to_kentos(qgis):
    toks = tokens(qgis, True)
    if toks is None:
        return None
    out = []
    for i, (k, v) in enumerate(toks):
        next_open = i + 1 < len(toks) and toks[i + 1] == ("op", "(")
        if k == "field":
            out.append((f"[{v}]", False))
        elif k == "str":
            out.append((quote(v, "'"), False))
        elif k in ("num", "op"):
            out.append((v, False))
        elif next_open and keyword(v) is None:
            f = function(v)
            if f is None:
                return None
            out.append((f[0], True))
        else:
            kw = keyword(v)
            out.append((kw, False) if kw is not None else (f"[{v}]", False))
    return join(out)


if __name__ == "__main__":
    cases = [("büyük([ad]) = 'A'", "upper(\"ad\") = 'A'"),
             ("yuvarla([alan] / 1000, 1) > 2 AND [tür] IN ('konut', 'ticaret')",
              "round(\"alan\" / 1000, 1) > 2 AND \"tür\" IN ('konut', 'ticaret')"),
             ("CASE WHEN [kat] > 3 THEN 'yüksek' ELSE 'alçak' END",
              "CASE WHEN \"kat\" > 3 THEN 'yüksek' ELSE 'alçak' END"),
             ("[ad] IS NULL", "\"ad\" IS NULL"), ("[ad] IS NOT NULL", "\"ad\" IS NOT NULL"),
             ("varsayılan([ad], '?') || ' (' || metin([no]) || ')'",
              "coalesce(\"ad\", '?') || ' (' || to_string(\"no\") || ')'")]
    for k, q in cases:
        assert to_qgis(k) == q, (k, to_qgis(k))
        assert to_kentos(q) == k, (q, to_kentos(q))
    assert to_qgis("[ad] boş değil ve büyük(ad) gibi 'A%'") == "\"ad\" IS NOT NULL AND upper(\"ad\") LIKE 'A%'"
    assert to_qgis("$alan > 10") is None and to_kentos("regexp_match(\"ad\", 'x')") is None
    print("ok")
