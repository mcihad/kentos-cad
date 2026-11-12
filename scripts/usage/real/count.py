"""What a real-window scenario asks of the hand (docs/adr/8000): clicks, keystrokes (every key of a chord), typed
values and their letters, pointer moves, wheel notches, tool starts; and the Keystroke-Level Model's time.

KLM operators and times as Kieras gives them ("Using the Keystroke-Level Model to Estimate Execution Times", after
Card, Moran & Newell 1983): K .28 s (average non-secretarial typist), P 1.1 s, B .1 s (a click is BB), H .4 s,
M 1.2 s. Rules kept the same for every scenario, as the KLM asks (consistency over placement):
  - click: P + BB; double click: P + 4B; move and glide (one hand's sweep along a line): P; drag: 2P + 2B; wheel: B per
    notch; settle and cpu ask nothing of the hand;
  - key: K per key of the chord; type: K per letter;
  - H whenever the hand changes between keyboard and mouse (it starts on the mouse);
  - M at each tool start (a line whose comment begins with "araç:") and before each typed value.
`count.py SCENARIO.txt` prints the counts and the time.
"""
import sys

K, P, B, H, M = 0.28, 1.1, 0.1, 0.4, 1.2


def count(path):
    c = dict(tik=0, tus=0, deger=0, harf=0, hareket=0, teker=0, arac=0)
    ops = dict(K=0, P=0, B=0, H=0, M=0)
    hand = "mouse"

    def to(h):
        nonlocal hand
        if hand != h:
            ops["H"] += 1
            hand = h

    for raw in open(path, encoding="utf-8"):
        if raw.lstrip().startswith("#"):
            continue
        line, _, comment = raw.rstrip("\n").partition(" #")
        word, _, rest = line.strip().partition(" ")
        if word in ("click", "dclick"):
            to("mouse")
            c["tik"] += 1 if word == "click" else 2
            ops["P"] += 1
            ops["B"] += 2 if word == "click" else 4
            if comment.strip().startswith("araç:"):
                c["arac"] += 1
                ops["M"] += 1
        elif word in ("move", "glide"):
            to("mouse")
            c["hareket"] += 1
            ops["P"] += 1
        elif word == "drag":
            to("mouse")
            c["tik"] += 1
            ops["P"] += 2
            ops["B"] += 2
        elif word == "wheel":
            to("mouse")
            n = abs(int(rest.split()[2]))
            c["teker"] += n
            ops["B"] += n
        elif word == "key":
            to("keyboard")
            n = len(rest.strip().split("+"))
            c["tus"] += n
            ops["K"] += n
        elif word == "type":
            to("keyboard")
            c["deger"] += 1
            c["harf"] += len(rest)
            ops["K"] += len(rest)
            ops["M"] += 1
    c["etkilesim"] = c["tik"] + c["tus"] + c["deger"] + c["hareket"] + c["teker"]
    seconds = ops["K"] * K + ops["P"] * P + ops["B"] * B + ops["H"] * H + ops["M"] * M
    return c, ops, seconds


if __name__ == "__main__":
    c, ops, seconds = count(sys.argv[1])
    print(
        f"{c['etkilesim']} etkileşim: {c['tik']} tık, {c['tus']} tuş, {c['deger']} değer ({c['harf']} harf), "
        f"{c['hareket']} hareket, {c['teker']} tekerlek; {c['arac']} araç"
    )
    print(
        f"KLM: K {ops['K']}, P {ops['P']}, B {ops['B']}, H {ops['H']}, M {ops['M']} → {seconds:.1f} s "
        f"({int(seconds // 60)} dk {seconds % 60:.0f} s)"
    )
