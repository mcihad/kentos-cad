"""Writes the shared cases of the colours on paper and in DXF (docs/adr/0312 §5): fixtures/look/v1/print.json.

A colour is a theme colour (`fg`, `fg-dim`, `ink`, `paper`, the drawing's `label`) or a true colour (`#rrggbb`). The
screen draws a theme colour in the interface's theme; paper and files never do:

- on paper (a sheet, its PDF, its legend) a theme colour is the paper's: white paper, black ink, `fg` and `label`
  #111111, `fg-dim` #555555 (design §9a); a true colour as it is;
- in DXF a theme colour is the CAD palette's (AutoCAD Color Index): `ink` and `fg` 7 (black on white, white on
  black), `fg-dim` 8, `paper` 255; a true colour its nearest index with its 420 true colour. The indices of the
  pure colours below are AutoCAD's own (1 red … 9 light grey).

Written from the rule above, without KentOS code; the contracts (`print_color`) and the DXF writer (`aci::from_app`)
run them.

    python3 scripts/fixtures/print_color_cases.py           # writes the file
    python3 scripts/fixtures/print_color_cases.py --check   # compares it with the one on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/look/v1/print.json"

PAPER = {"paper": "#ffffff", "ink": "#000000", "fg": "#111111", "fg-dim": "#555555", "label": "#111111"}
THEME_ACI = {"ink": 7, "fg": 7, "fg-dim": 8, "paper": 255}
PURE = {"#FF0000": 1, "#FFFF00": 2, "#00FF00": 3, "#00FFFF": 4, "#0000FF": 5, "#FF00FF": 6, "#808080": 8, "#C0C0C0": 9}


def case(color):
    if color in THEME_ACI:
        dxf = {"aci": THEME_ACI[color]}
    elif color.upper() in PURE:
        hex_ = color.upper()
        dxf = {"aci": PURE[hex_], "rgb": int(hex_[1:], 16)}
    else:
        dxf = None
    out = {"color": color, "paper": PAPER.get(color, color)}
    if dxf is not None:
        out["dxf"] = dxf
    return out


def main() -> int:
    colours = ["ink", "fg", "fg-dim", "paper", "label", *PURE, "#ff0000", "#E5484D", "#0090FF"]
    doc = {"format": "kentos.print-colour-cases", "version": 1,
           "note": "Written by scripts/fixtures/print_color_cases.py from docs/adr/0312 §5; do not edit by hand.",
           "cases": [case(c) for c in colours]}
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.name} güncel: {len(doc['cases'])} durum.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
