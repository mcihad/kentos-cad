#!/usr/bin/env python3
"""What an interaction trace asks of its user, section by section (docs/adr/0776 §1): the counts a usability review
reads, from the trace file alone (no KentOS code). A section starts at a step whose ``note`` begins with “§”.

- ``route``: how the tool was found: ``şerit`` (a ``run`` step, a ribbon button), ``komut satırı`` (a command's name
  typed), ``—`` (the selection tool, no tool).
- ``clicks``: presses on the drawing (``click``, ``clickNumber``, ``clickLock``, ``doubleClick``, ``rightClick``, ``drag``).
- ``hover``: the pointer brought somewhere without a press (``move``, ``rest``) — what decides a side or a number.
- ``keys``: keys pressed (Enter, Esc, a letter on the drawing, Shift+Space …).
- ``typed``: values typed on the command line (a tool's option letter not counted).
- ``options``: option letters typed.
- ``focus``: the keyboard moved between the drawing and the command line, either way.
- ``retyped``: a value typed again for the same tool option it was typed for before (what the tool did not keep).
- ``recoveries``: steps taken to get past what the app did not do at first (``note`` beginning “Recovery”), and
  Shift+Space to reach the object under another.
- ``shots``: the pictures it takes.

    python3 scripts/acceptance/trace_metrics.py fixtures/interaction/v1/acceptance-flat.json [--markdown]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

CLICKS = ("click", "clickNumber", "clickLock", "doubleClick", "rightClick", "drag")


def sections(trace: dict) -> list[dict]:
    rows: list[dict] = []
    row: dict | None = None
    line = False  # the command line has the keyboard
    last_option: str | None = None
    tool: str | None = None
    seen: set[tuple[str | None, str, str]] = set()
    for step in trace["steps"]:
        note = step.get("note", "")
        if note.startswith("§") or row is None:
            row = {"section": note.lstrip("§ ").strip() or "(başlangıç)", "route": "—", "clicks": 0, "hover": 0,
                   "keys": 0, "typed": 0, "options": 0, "focus": 0, "retyped": 0, "recoveries": 0, "shots": 0,
                   "steps": 0}
            rows.append(row)
        row["steps"] += 1
        if "run" in step:
            row["route"] = "şerit" if row["route"] == "—" else row["route"]
            tool = step["run"]
        if any(k in step for k in CLICKS):
            row["clicks"] += 1
            if line:
                row["focus"] += 1
                line = False
        if "move" in step or "rest" in step:
            row["hover"] += 1
        if "shot" in step:
            row["shots"] += 1
        if step.get("focus") == "commandLine":
            if not line:
                row["focus"] += 1
            line = True
        if "text" in step:
            text = step["text"]
            if len(text) == 1 and text.isalpha():
                row["options"] += 1
                last_option = text.upper()
            else:
                row["typed"] += 1
                key = (tool, last_option or "", text)
                if last_option and key in seen:
                    row["retyped"] += 1
                seen.add(key)
                last_option = None
        if "key" in step:
            row["keys"] += 1
            if step["key"] == "Shift+Space":
                row["recoveries"] += 1
        if note.startswith("Recovery"):
            row["recoveries"] += 1
        if "saveAndReopen" in step:
            row["route"] = "Ctrl+S, Ctrl+O"
    return rows


COLUMNS = ["section", "route", "clicks", "hover", "keys", "options", "typed", "retyped", "focus", "recoveries", "shots", "steps"]
HEAD = ["Bölüm", "Araç yolu", "Tık", "İmleç", "Tuş", "Seçenek", "Değer", "Tekrar", "Odak", "Kurtarma", "Resim", "Adım"]


def main(argv: list[str]) -> int:
    trace = json.loads(Path(argv[0]).read_text(encoding="utf-8"))
    rows = sections(trace)
    total = {c: sum(r[c] for r in rows) for c in COLUMNS[2:]}
    if "--markdown" in argv:
        print("| " + " | ".join(HEAD) + " |")
        print("|" + "---|" * len(HEAD))
        for r in rows + [{"section": "**Toplam**", "route": "", **total}]:
            print("| " + " | ".join(str(r[c]) for c in COLUMNS) + " |")
    else:
        print(json.dumps({"trace": trace["id"], "sections": rows, "total": total}, ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
