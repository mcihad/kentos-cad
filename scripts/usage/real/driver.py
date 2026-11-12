"""Drives the X display in $DISPLAY with real input events (XTEST), like a hand on the mouse and keyboard
(scripts/usage/real/README.md, docs/adr/8000). `driver.py SCENARIO OUT_DIR`.

Scenario lines, one per line; a line starting with '#' and anything after ' #' is a comment:
  fit TITLE               the window of this title over the whole screen, keyboard given to it (no window manager)
  wait MS                 time passes
  move X Y                the pointer goes there through the points between
  click X Y [right|middle], dclick X Y, drag X1 Y1 X2 Y2
  wheel X Y N             N wheel notches there: positive closer, negative farther
  key SPEC                Return, Escape, Home, F8, ctrl+a, shift+Tab …
  type TEXT               the text as keystrokes (Turkish letters too)
  shot NAME               the screen as OUT_DIR/NAME.png
  glide X1 Y1 X2 Y2 N MS  the pointer along a line in N moves MS apart (a tool drawn along a wall)
  settle MS               until the app is idle (under a tenth of a core for a quarter second), at most MS
  cpu NAME / cpu NAME end the app's processor and wall time between the two marks, printed (docs/adr/8051)
Points are screen pixels, as the pictures show them.
"""
import sys, time
from Xlib import X, XK, display
from Xlib.ext import xtest

d = display.Display()
out = sys.argv[2]
pos = [0, 0]
BUTTONS = {"left": 1, "middle": 2, "right": 3}

def sync(ms=0):
    d.sync()
    if ms:
        time.sleep(ms / 1000)

def move(x, y, steps=12):
    # A hand moves through the points between, so hover and snapping see the way.
    x0, y0 = pos
    for i in range(1, steps + 1):
        xi = round(x0 + (x - x0) * i / steps)
        yi = round(y0 + (y - y0) * i / steps)
        xtest.fake_input(d, X.MotionNotify, x=xi, y=yi)
        sync(12)
    pos[:] = [x, y]

def button(b, down):
    xtest.fake_input(d, X.ButtonPress if down else X.ButtonRelease, b)
    sync(40)

spare = None
def keycode_for(keysym):
    global spare
    code = d.keysym_to_keycode(keysym)
    if code:
        return code, d.keycode_to_keysym(code, 0) != keysym
    # Not on the keyboard: lend it a free key for the time of the press.
    if spare is None:
        first = d.display.info.min_keycode
        count = d.display.info.max_keycode - first + 1
        mapping = d.get_keyboard_mapping(first, count)
        for i, syms in enumerate(mapping):
            if not any(syms):
                spare = first + i
                break
    d.change_keyboard_mapping(spare, [(keysym, keysym)])
    sync(30)
    return spare, False

MODS = {"ctrl": "Control_L", "shift": "Shift_L", "alt": "Alt_L", "super": "Super_L"}

def press_keysym(keysym, mods=()):
    code, shifted = keycode_for(keysym)
    held = [d.keysym_to_keycode(XK.string_to_keysym(MODS[m])) for m in mods]
    if shifted:
        held.append(d.keysym_to_keycode(XK.string_to_keysym("Shift_L")))
    for m in held:
        xtest.fake_input(d, X.KeyPress, m)
    xtest.fake_input(d, X.KeyPress, code)
    sync(25)
    xtest.fake_input(d, X.KeyRelease, code)
    for m in reversed(held):
        xtest.fake_input(d, X.KeyRelease, m)
    sync(35)

def key(spec):
    parts = spec.split("+")
    name = parts[-1]
    keysym = XK.string_to_keysym(name)
    if keysym == 0 and len(name) == 1:
        keysym = char_keysym(name)
    if keysym == 0:
        raise SystemExit(f"unknown key: {spec}")
    press_keysym(keysym, [p.lower() for p in parts[:-1]])

def char_keysym(ch):
    names = {" ": "space", ",": "comma", ".": "period", "-": "minus", "+": "plus", "/": "slash",
             "(": "parenleft", ")": "parenright", "*": "asterisk", "@": "at", "<": "less", ">": "greater",
             ":": "colon", ";": "semicolon", "=": "equal", "_": "underscore", "'": "apostrophe"}
    if ch in names:
        return XK.string_to_keysym(names[ch])
    ks = XK.string_to_keysym(ch)
    if ks:
        return ks
    return 0x01000000 + ord(ch)  # a Unicode keysym

def type_text(text):
    for ch in text:
        press_keysym(char_keysym(ch))

def shot(name):
    # The X screen itself, as the window shows it now.
    from PIL import Image
    root = d.screen().root
    g = root.get_geometry()
    raw = root.get_image(0, 0, g.width, g.height, X.ZPixmap, 0xffffffff)
    Image.frombytes("RGB", (g.width, g.height), raw.data, "raw", "BGRX").save(f"{out}/{name}.png")

def fit(title):
    # No window manager on the screen: the app's window put over the whole screen, as a maximized window is.
    root = d.screen().root
    g = root.get_geometry()
    def named(w):
        # WM_NAME comes as bytes python-xlib reads as Latin-1; the app writes UTF-8 ("KentOS Mimari — Konut").
        name = w.get_wm_name()
        if isinstance(name, str):
            try:
                return name.encode("latin-1").decode("utf-8")
            except (UnicodeEncodeError, UnicodeDecodeError):
                return name
        return name

    seen = []
    for w in root.query_tree().children:
        try:
            name = named(w)
            if name:
                seen.append(name)
            if name == title:
                w.configure(x=0, y=0, width=g.width, height=g.height)
                # No window manager gives the keyboard to the window: give it as one would.
                w.set_input_focus(X.RevertToParent, X.CurrentTime)
                d.sync()
                return
        except Exception:
            pass
    # The windows there are said: a drawing's title is its own name, not its file's.
    raise SystemExit(f"no window: {title} (there: {', '.join(seen) or 'none'})")

def app_pid():
    # The app on this display: its environment names the display the driver drives, never the owner's own window.
    import os
    me = os.environ.get("DISPLAY")
    for pid in os.listdir("/proc"):
        if not pid.isdigit():
            continue
        try:
            if not open(f"/proc/{pid}/comm").read().strip().startswith("kentos-cad"):
                continue
            if f"DISPLAY={me}".encode() in open(f"/proc/{pid}/environ", "rb").read().split(b"\0"):
                return int(pid)
        except OSError:
            pass
    raise SystemExit("no app on this display")


def cpu_ms(pid):
    # Every thread's user and system time (fields 14 and 15), milliseconds.
    import os
    fields = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
    return (int(fields[11]) + int(fields[12])) * 1000.0 / os.sysconf("SC_CLK_TCK")


marks = {}


def cpu(name):
    # The app's processor time since the mark of the same name, and the wall's: "NAME" starts, "NAME end" ends.
    pid = app_pid()
    now = (cpu_ms(pid), time.monotonic() * 1000.0)
    if name.endswith(" end"):
        start = marks.pop(name[:-4])
        print(f"CPU {name[:-4]}: {now[0] - start[0]:.0f} ms işlemci, {now[1] - start[1]:.0f} ms duvar saati", flush=True)
    else:
        marks[name] = now


for raw in open(sys.argv[1], encoding="utf-8"):
    if raw.lstrip().startswith("#"):
        continue
    line = raw.rstrip("\n").split(" #", 1)[0]
    if not line.strip():
        continue
    word, _, rest = line.strip().partition(" ")
    a = rest.split()
    if word == "wait":
        sync(int(a[0]))
    elif word == "move":
        move(int(a[0]), int(a[1]))
    elif word == "click":
        move(int(a[0]), int(a[1]))
        b = BUTTONS[a[2]] if len(a) > 2 else 1
        button(b, True); button(b, False); sync(150)
    elif word == "dclick":
        move(int(a[0]), int(a[1]))
        for _ in range(2):
            button(1, True); button(1, False)
        sync(150)
    elif word == "drag":
        move(int(a[0]), int(a[1])); button(1, True)
        move(int(a[2]), int(a[3]), steps=24); button(1, False); sync(150)
    elif word == "wheel":
        # Wheel notches at a point: positive up (closer), negative down (farther), as X11's buttons 4 and 5.
        move(int(a[0]), int(a[1]))
        b = 4 if int(a[2]) > 0 else 5
        for _ in range(abs(int(a[2]))):
            button(b, True); button(b, False); sync(120)
        sync(300)
    elif word == "key":
        key(a[0]); sync(80)
    elif word == "type":
        type_text(rest); sync(80)
    elif word == "shot":
        sync(300); shot(a[0])
    elif word == "fit":
        fit(rest.strip()); sync(800)
    elif word == "glide":
        x0, y0, x1, y1, n, ms = (int(v) for v in a)
        xtest.fake_input(d, X.MotionNotify, x=x0, y=y0)
        sync(ms)
        for i in range(1, n + 1):
            xtest.fake_input(d, X.MotionNotify, x=round(x0 + (x1 - x0) * i / n), y=round(y0 + (y1 - y0) * i / n))
            sync(ms)
        pos[:] = [x1, y1]
    elif word == "settle":
        pid = app_pid()
        end = time.monotonic() + int(a[0]) / 1000
        last = cpu_ms(pid)
        while time.monotonic() < end:
            sync(250)
            now = cpu_ms(pid)
            if now - last < 25:
                break
            last = now
    elif word == "cpu":
        sync()
        cpu(rest.strip())
    else:
        raise SystemExit(f"unknown line: {line}")
    print("ok:", line.strip(), flush=True)
