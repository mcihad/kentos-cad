"""Speaks kentos.ext_host's lines for the blueprint's İşlemler tool (crates/native/extension-tools tests): says how far
it is, logs and prints, asks KentOS for a buffer of its inputs (docs/adr/8057 §47), measures them and answers with the
descriptor's outputs; its expression function answers a batch of rows (§80). What it was asked to run is written
beside it (cagrilar.txt)."""
import json, os, sys, threading
here = os.path.dirname(os.path.abspath(__file__))
out = sys.stdout
def send(m):
    out.write(json.dumps(m) + "\n"); out.flush()
work, replies = [], {}
lock = threading.Condition()
def read():
    for line in sys.stdin:
        m = json.loads(line)
        with lock:
            if m["type"] == "reply":
                replies[m["id"]] = m
            elif m["type"] != "cancel":
                work.append(m)
            lock.notify_all()
    with lock:
        work.append(None); lock.notify_all()
threading.Thread(target=read, daemon=True).start()
def next_work():
    with lock:
        while not work:
            lock.wait()
        return work.pop(0)
def area(ring):
    s = 0.0
    for (x1, y1), (x2, y2) in zip(ring, ring[1:] + ring[:1]):
        s += x1 * y2 - x2 * y1
    return abs(s) / 2
hello = next_work()
send({"type": "ready", "protocol": 1, "python": "3", "kentos": "0"})
while True:
    m = next_work()
    if m is None or m["type"] == "quit":
        os._exit(0)
    i = m["id"]
    with open(os.path.join(here, "cagrilar.txt"), "a") as f:
        f.write(m["local"] + "\n")
    if m.get("kind") == "expression":
        # The blueprint's function, Turkish upper case; a row whose argument is "hata" fails alone (§80).
        values, failed, first = [], 0, ""
        for row in m["rows"]:
            if row and row[0] == "hata":
                values.append(None); failed += 1; first = first or "ValueError: istenen hata"
            else:
                values.append(None if not row or row[0] is None
                              else str(row[0]).replace("i", "İ").replace("ı", "I").upper())
        result = {"kind": "values", "values": values}
        if failed:
            result.update(failed=failed, error=first)
        send({"type": "done", "id": i, "ok": True, "result": result})
        continue
    if m["params"].get("hata"):
        # The code fails: what it printed, then its traceback.
        send({"type": "out", "id": i, "stream": "out", "text": "ara çıktı\n"})
        send({"type": "done", "id": i, "ok": False, "exception": "ValueError", "message": "istenen hata",
              "error": "Traceback (most recent call last):\n  File \"logic.py\", line 3\nValueError: istenen hata"})
        continue
    features = m["inputs"]["alanlar"]["features"]
    send({"type": "progress", "id": i, "done": 1, "total": 2, "text": "ölçülüyor"})
    send({"type": "log", "id": i, "level": "info", "text": "eklenti: %s" % hello["extension"]})
    send({"type": "out", "id": i, "stream": "out", "text": "yazdırılan satır\n"})
    send({"type": "call", "id": 77, "method": "processing.run", "params": {"tool": "geometry.buffer",
          "parameters": {"input": {"features": [{"geometry": f["geometry"], "attrs": f["entity"]["attrs"]}
                                                for f in features]},
                         "distance": 1, "layer": {"newName": "Tampon"}}}})
    with lock:
        while 77 not in replies:
            lock.wait()
        reply = replies.pop(77)
    made = sum(len(l["features"]) for l in reply["result"]["layers"]) if reply["ok"] else -1
    rows = [[n, area([tuple(p) for p in f["geometry"]["coordinates"][0][:-1]])] for n, f in enumerate(features, 1)]
    send({"type": "done", "id": i, "ok": True, "result": {"kind": "outputs",
          "outputs": {"tablo": {"columns": ["sira", "alan"], "rows": rows}, "toplam": sum(r[1] for r in rows)},
          "summary": "%d alan; tampon %d" % (len(rows), made)}})
