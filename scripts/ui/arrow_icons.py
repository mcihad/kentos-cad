"""The leader arrowheads' icons (docs/adr/0205 §7): each arrowhead drawn by
itself in the icon set's 20 × 20 square from the rule's own shapes
(scripts/fixtures/leader_cases.py, written from the ADR), its tip at the
left (moved right as far as a head reaching behind its tip needs, so that
it stays 1.5 px inside the square), the leader's line running right from
where the rule starts it.

    python3 scripts/ui/arrow_icons.py           # writes the file
    python3 scripts/ui/arrow_icons.py --check   # writes nothing; compares

Writes apps/web/src/ui/arrowIcons.ts (`ARROW_ICONS`, joined into the icon
set; the desktop draws them from the inventory as it draws every icon). An
arrowhead is L = 9 icon pixels long; its areas are filled with the current
colour and not stroked (as the drawing fills them), its lines are stroked.
"""
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import leader_cases as rule  # noqa: E402  (the arrowheads' shapes, written from the ADR)

OUT = ROOT / "apps/web/src/ui/arrowIcons.ts"

# The tip, the line's end and the arrowhead's length, in icon pixels.
TIP_X, Y, END_X, L = 2.5, 10.0, 17.5, 9.0

# The icons by name and the arrowhead each draws (None: the filled triangle).
ICONS = [
    ("leaderArrowFilled", None),
    ("leaderArrowClosed", "closed"),
    ("leaderArrowOpen", "open"),
    ("leaderArrowOpen30", "open30"),
    ("leaderArrowOpen90", "open90"),
    ("leaderArrowDot", "dot"),
    ("leaderArrowDotSmall", "dotSmall"),
    ("leaderArrowDotBlank", "dotBlank"),
    ("leaderArrowOblique", "oblique"),
    ("leaderArrowArchTick", "archTick"),
    ("leaderArrowBoxFilled", "boxFilled"),
    ("leaderArrowBoxBlank", "boxBlank"),
    ("leaderArrowDatum", "datumFilled"),
    ("leaderArrowNone", "none"),
]


def num(v):
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def tip_of(fills, lines):
    """The tip's x: TIP_X, or further right when the head reaches behind it (a tick, a box)."""
    xs = [p[0] for ring in fills for p in ring] + [p[0] for pts, _ in lines for p in pts] + [0.0]
    return max(TIP_X, 1.5 - min(xs) * L)


def is_circle(pts):
    return len(pts) == 72


def icon(arrow):
    fills, lines, back = rule.head_of(arrow)
    tip = tip_of(fills, lines)

    def at(p):
        """A point of the head's frame (x along the line, y across, in L) in the icon (y down)."""
        return (tip + p[0] * L, Y - p[1] * L)

    out = [f'<path d="M{num(tip + back * L)} {num(Y)}H{num(END_X)}"/>']
    for ring in fills:
        if is_circle(ring):
            r = math.hypot(*ring[0]) * L
            out.append(f'<circle cx="{num(tip)}" cy="{num(Y)}" r="{num(r)}" fill="currentColor" stroke="none"/>')
        else:
            d = "".join(("M" if i == 0 else "L") + f"{num(x)} {num(y)}" for i, (x, y) in enumerate(map(at, ring)))
            out.append(f'<path d="{d}Z" fill="currentColor" stroke="none"/>')
    for pts, closed in lines:
        if is_circle(pts):
            r = math.hypot(*pts[0]) * L
            out.append(f'<circle cx="{num(tip)}" cy="{num(Y)}" r="{num(r)}"/>')
        else:
            d = "".join(("M" if i == 0 else "L") + f"{num(x)} {num(y)}" for i, (x, y) in enumerate(map(at, pts)))
            out.append(f'<path d="{d}{"Z" if closed else ""}"/>')
    return "".join(out)


def build():
    rows = "\n".join(f"  {name}: '{icon(arrow)}'," for name, arrow in ICONS)
    return (
        "/**\n"
        " * The leader arrowheads' icons (docs/adr/0205 §7): each arrowhead drawn by itself from the rule's shapes, its tip at\n"
        " * the left. Written by scripts/ui/arrow_icons.py (scripts/fixtures/leader_cases.py's shapes); do not edit by hand.\n"
        " */\n"
        f"export const ARROW_ICONS = {{\n{rows}\n}} as const;\n"
    )


def main():
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)}: kuraldan yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(ICONS)} simge kuralla aynı")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(ICONS)} simge)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
