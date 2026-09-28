"""The console's help while code is typed (docs/adr/0135): the names that
can follow what is written (completion) and the signature of the call the
cursor is in, from the console's own names.

Nothing written runs: a dotted name is followed attribute by attribute
without calling anything (`inspect.getattr_static`); a property is followed
through the type its getter says it returns (`doc.settings.` lists a
ProjectSettings' fields), and a call's result is not guessed.
"""

from __future__ import annotations

import builtins
import inspect
import keyword
import re
import typing
from typing import Any

_MISSING = object()
_DOTTED = re.compile(r"((?:[^\W\d]\w*\.)*)([^\W\d]\w*)?$")
_CALLEE = re.compile(r"((?:[^\W\d]\w*\.)*[^\W\d]\w*)\s*$")
_KEYWORD_ARG = re.compile(r"\s*([^\W\d]\w*)\s*=(?!=)")
MOST = 100


def _returns(fget: Any) -> Any:
    """The class a property's getter says it returns, or nothing."""
    try:
        hint = typing.get_type_hints(fget).get("return")
    except Exception:  # noqa: BLE001 - a hint that does not resolve is no hint
        return _MISSING
    return hint if inspect.isclass(hint) else _MISSING


def resolve(names: dict[str, Any], dotted: str) -> Any:
    """What a dotted name is, without running anything; `_MISSING` when unknown."""
    parts = dotted.split(".")
    obj = names.get(parts[0], getattr(builtins, parts[0], _MISSING))
    for part in parts[1:]:
        if obj is _MISSING:
            return _MISSING
        try:
            found = inspect.getattr_static(obj, part)
        except AttributeError:
            return _MISSING
        if isinstance(found, property):
            obj = _returns(found.fget)
        elif isinstance(found, (staticmethod, classmethod)) or inspect.isfunction(found):
            obj = getattr(obj, part)
        elif type(found).__name__ in ("member_descriptor", "getset_descriptor"):
            # A slot's or a built-in's value: reading it runs no code of ours.
            try:
                obj = getattr(obj, part)
            except AttributeError:
                return _MISSING
        else:
            obj = found
    return obj


def _clean(signature: str) -> str:
    """Annotations written as text (`from __future__ import annotations`) without their quotes."""
    return re.sub(r"'([^']*)'", r"\1", signature)


def _kind_and_detail(owner: Any, name: str) -> tuple[str, str]:
    try:
        found = inspect.getattr_static(owner, name) if owner is not None else _MISSING
    except AttributeError:
        found = _MISSING
    if isinstance(found, property):
        returned = _returns(found.fget)
        return "property", "" if returned is _MISSING else returned.__name__
    value = found
    if owner is not None and not isinstance(found, (staticmethod, classmethod)) and found is not _MISSING:
        if type(found).__name__ in ("member_descriptor", "getset_descriptor"):
            try:
                value = getattr(owner, name)
            except AttributeError:
                return "value", ""
    if isinstance(value, (staticmethod, classmethod)):
        value = value.__func__
    if inspect.ismodule(value):
        return "module", _first_line(inspect.getdoc(value))
    if inspect.isclass(value):
        return "class", _first_line(inspect.getdoc(value))
    if callable(value):
        try:
            return "function", _clean(str(inspect.signature(value)))
        except (TypeError, ValueError):
            return "function", ""
    return "value", type(value).__name__


def plain(text: str) -> str:
    """Help without its markup (``code``, :class:`Name`), as a line shows it."""
    text = re.sub(r":\w+:`([^`]*)`", r"\1", text)
    return text.replace("``", "")


def _first_line(doc: str | None) -> str:
    return plain((doc or "").strip().split("\n", 1)[0])[:120]


def complete(names: dict[str, Any], code: str, cursor: int) -> dict[str, Any]:
    """The names that can end the word before `cursor`: `start` is where it begins."""
    text = code[:cursor]
    found = _DOTTED.search(text)
    chain = found.group(1) if found else ""
    word = (found.group(2) if found else "") or ""
    start = cursor - len(word)
    if chain:
        owner = resolve(names, chain[:-1])
        if owner is _MISSING:
            return {"start": start, "items": []}
        candidates = sorted(set(dir(owner)))
    else:
        owner = None
        candidates = sorted(set(names) | set(dir(builtins)) | set(keyword.kwlist))
    private = word.startswith("_")
    matched = [c for c in candidates if c.startswith(word) and (private or not c.startswith("_"))]
    if not matched:
        low = word.lower()
        matched = [c for c in candidates if c.lower().startswith(low) and (private or not c.startswith("_"))]
    items = []
    for name in matched[:MOST]:
        if owner is None:
            if keyword.iskeyword(name):
                kind, detail = "keyword", ""
            else:
                value = names.get(name, getattr(builtins, name, None))
                kind, detail = _kind_and_detail(None, name) if value is None else _describe(value)
        else:
            kind, detail = _kind_and_detail(owner, name)
        items.append({"text": name, "kind": kind, "detail": detail})
    return {"start": start, "items": items}


def _describe(value: Any) -> tuple[str, str]:
    if inspect.ismodule(value):
        return "module", _first_line(inspect.getdoc(value))
    if inspect.isclass(value):
        return "class", _first_line(inspect.getdoc(value))
    if callable(value):
        try:
            return "function", _clean(str(inspect.signature(value)))
        except (TypeError, ValueError):
            return "function", ""
    return "value", type(value).__name__


def _open_paren(text: str) -> int | None:
    """Where the innermost call still open before the end begins, outside strings."""
    stack: list[int] = []
    quote: str | None = None
    i = 0
    while i < len(text):
        c = text[i]
        if quote:
            if c == "\\":
                i += 1
            elif text.startswith(quote, i):
                i += len(quote) - 1
                quote = None
        elif c == "#":
            end = text.find("\n", i)
            i = len(text) if end < 0 else end
            continue
        elif c in "'\"":
            quote = c * 3 if text.startswith(c * 3, i) else c
            i += len(quote) - 1
        elif c in "([{":
            stack.append(i)
        elif c in ")]}" and stack:
            stack.pop()
        i += 1
    while stack and text[stack[-1]] != "(":
        stack.pop()
    return stack[-1] if stack else None


def signature(names: dict[str, Any], code: str, cursor: int) -> dict[str, Any] | None:
    """The call the cursor is in: its label, its help and the argument being written."""
    text = code[:cursor]
    at = _open_paren(text)
    if at is None:
        return None
    callee = _CALLEE.search(text[:at])
    if callee is None:
        return None
    target = resolve(names, callee.group(1))
    if target is _MISSING or not callable(target):
        return None
    try:
        sig = inspect.signature(target)
    except (TypeError, ValueError):
        return None
    written = text[at + 1 :]
    depth = 0
    last = 0
    for i, c in enumerate(written):
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == "," and depth == 0:
            last = i + 1
    current = written[last:]
    named = _KEYWORD_ARG.match(current)
    argument = None
    if named and named.group(1) in sig.parameters:
        argument = _clean(str(sig.parameters[named.group(1)]))
    else:
        # By place: the positional parameters, the commas before the cursor counting them.
        index = written[:last].count(",") if last else 0
        positional = [
            p for p in sig.parameters.values() if p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD)
        ]
        if index < len(positional):
            argument = _clean(str(positional[index]))
    doc = inspect.getdoc(target) or ""
    return {
        "label": callee.group(1).rsplit(".", 1)[-1] + _clean(str(sig)),
        "doc": plain(doc.split("\n\n", 1)[0].replace("\n", " "))[:400],
        "argument": argument,
    }
