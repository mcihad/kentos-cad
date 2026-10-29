#!/usr/bin/env python3
"""The shared cases of the server jobs' result cache and of rotated backups (docs/adr/0739), written from the ADR's
rules without KentOS code: the cache key (§2) is the SHA-256 of a canonical JSON (protocol_cases.py's, docs/adr/0253
§2) of the kind, the input without what does not change the result, the project's revision and cursor, the reach's
stamp, what built the file (the software) and, for the atlas and the tool, the user and the day; a backup folder's
name and order and the rotation (§4) are the ADR's.

    python3 scripts/fixtures/job_cache_cases.py           # writes fixtures/server-jobs/v1/cache-rotation.json
    python3 scripts/fixtures/job_cache_cases.py --check   # fails when the file differs
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from protocol_cases import canonical  # noqa: E402  (the independent canonical JSON)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "server-jobs" / "v1" / "cache-rotation.json"

FORMAT = "kentos.job-cache/1"
NOT_RESULT = {"expectedRevision", "cpuSeconds", "memoryMb", "label"}


def cached(kind: str, input) -> bool:
    if kind in ("project.export", "project.atlas"):
        return True
    if kind == "project.processing":
        mode = input.get("mode") if isinstance(input, dict) else None
        return mode == "sink"
    return False


def record(kind: str, input, state: dict) -> str | None:
    if not cached(kind, input):
        return None
    if input is None:
        input = {}
    if isinstance(input, dict):
        input = {k: v for k, v in input.items() if k not in NOT_RESULT}
    personal = kind != "project.export"
    return canonical(
        {
            "format": FORMAT,
            "kind": kind,
            "input": input,
            "revision": str(state["revision"]),
            "cursor": str(state["cursor"]),
            "reach": state.get("reach"),
            "user": state["user"] if personal else None,
            "day": state["day"] if personal else None,
            "software": state["software"],
        }
    )


def key(kind: str, input, state: dict) -> str | None:
    text = record(kind, input, state)
    return None if text is None else hashlib.sha256(text.encode("utf-8")).hexdigest()


NAME = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}Z)(?:-([1-9]\d*))?$")


def order(name: str):
    m = NAME.match(name)
    if not m:
        return None
    nth = int(m.group(2)) if m.group(2) else 1
    if m.group(2) is not None and nth < 2:
        return None
    return [m.group(1), nth]


def rotation(found: list[list], keep: int) -> list[str]:
    keep = max(keep, 1)
    named = [(f, order(f[0])) for f in found if order(f[0]) is not None]
    named.sort(key=lambda x: x[1], reverse=True)
    newest = next((o for f, o in named if f[1]), None)
    removed, complete = [], 0
    for (name, is_complete), o in named:
        if is_complete:
            complete += 1
            if complete > keep:
                removed.append(name)
        elif newest is not None and o < newest:
            removed.append(name)
    removed.sort(key=order)
    return removed


SERVER = "0.1.0+48211736:1761739445000000000"
WORKER = SERVER + "|kentos-jobrun+20971520:1761739400000000000"


def state(revision=4, cursor=9, reach="r1", user="u-1", day="2026-10-29", software=SERVER):
    return {"revision": revision, "cursor": cursor, "reach": reach, "user": user, "day": day, "software": software}


def key_cases() -> list[dict]:
    atlas = {"sheet": "pafta-1", "fields": ["Ada"], "label": "Ada atlası"}
    sink = {"tool": "geometry.buffer", "mode": "sink", "parameters": {"distance": 5}}
    rows = [
        ("Dışa aktarma: girdisiz", "project.export", None, state()),
        ("Dışa aktarma: boş girdi aynı anahtar", "project.export", {}, state()),
        ("Dışa aktarma: expectedRevision anahtara girmez", "project.export", {"expectedRevision": "4"}, state()),
        ("Dışa aktarma: başka kullanıcı aynı anahtar", "project.export", {}, state(user="u-2")),
        ("Dışa aktarma: başka gün aynı anahtar", "project.export", {}, state(day="2026-10-30")),
        ("Dışa aktarma: yeni revizyon", "project.export", {}, state(revision=5, cursor=10)),
        ("Dışa aktarma: yalnız imleç ilerledi", "project.export", {}, state(cursor=11)),
        ("Dışa aktarma: başka erişim kapsamı", "project.export", {}, state(reach="r2")),
        ("Dışa aktarma: kesilmeyen kopya", "project.export", {}, state(reach=None)),
        ("Dışa aktarma: sunucu yeniden derlendi", "project.export", {}, state(software="0.1.0+48211736:1761739999000000000")),
        ("Dışa aktarma: sunucunun başka sürümü", "project.export", {}, state(software="0.2.0+48211736:1761739445000000000")),
        ("Atlas", "project.atlas", atlas, state(software=WORKER)),
        ("Atlas: etiket ad, anahtara girmez", "project.atlas", {k: v for k, v in atlas.items() if k != "label"}, state(software=WORKER)),
        ("Atlas: bütçe anahtara girmez", "project.atlas", {**atlas, "cpuSeconds": 60, "memoryMb": 256}, state(software=WORKER)),
        ("Atlas: başka kullanıcı başka anahtar", "project.atlas", atlas, state(user="u-2", software=WORKER)),
        ("Atlas: ertesi gün başka anahtar", "project.atlas", atlas, state(day="2026-10-30", software=WORKER)),
        ("Atlas: başka alanlar", "project.atlas", {**atlas, "fields": ["Ada", "Parsel"]}, state(software=WORKER)),
        ("Atlas: başka zaman", "project.atlas", {**atlas, "time": "2020-01-01"}, state(software=WORKER)),
        ("Atlas: araç programı yeniden derlendi", "project.atlas", atlas,
         state(software=SERVER + "|kentos-jobrun+20971520:1761739999000000000")),
        ("Araç dosyaya", "project.processing", sink, state(software=WORKER)),
        ("Araç dosyaya: anahtarların sırası önemsiz", "project.processing",
         {"parameters": {"distance": 5}, "mode": "sink", "tool": "geometry.buffer"}, state(software=WORKER)),
        ("Araç dosyaya: bütçe ve beklenen revizyon girmez", "project.processing",
         {**sink, "cpuSeconds": 30, "memoryMb": 512, "expectedRevision": "4"}, state(software=WORKER)),
        ("Araç dosyaya: başka değer", "project.processing", {**sink, "parameters": {"distance": 6}}, state(software=WORKER)),
        ("Araç dosyaya: seçim", "project.processing", {**sink, "selection": ["0190-a", "0190-b"]}, state(software=WORKER)),
        ("Araç dosyaya: Türkçe değer", "project.processing",
         {**sink, "parameters": {"field": "Malik adı", "value": "Çağrı \"ŞİMŞEK\""}}, state(software=WORKER)),
        ("Araç projeye yazar: önbellek yok", "project.processing", {"tool": "geometry.buffer", "mode": "commit"}, state()),
        ("Araç kipi yok (projeye yazar): önbellek yok", "project.processing", {"tool": "geometry.buffer"}, state()),
        ("Kontrol noktası: önbellek yok", "project.checkpoint", {"name": "Teslim"}, state()),
        ("İçe aktarma: önbellek yok", "project.import", {"upload": "0190-u"}, state()),
    ]
    out = []
    for name, kind, input, st in rows:
        out.append({
            "name": name,
            "kind": kind,
            "input": input,
            "state": st,
            "record": record(kind, input, st),
            "key": key(kind, input, st),
        })
    return out


def rotation_cases() -> list[dict]:
    t = lambda h, m=0, s=0, n=None: f"2026-10-{h // 24 + 20:02d}T{h % 24:02d}-{m:02d}-{s:02d}Z" + (f"-{n}" if n else "")
    rows = [
        ("Az yedek: hiçbiri silinmez", [[t(0), True], [t(24), True]], 3),
        ("Fazlası en eskiden silinir", [[t(0), True], [t(24), True], [t(48), True], [t(72), True]], 2),
        ("Bir yedek saklanınca yalnız en yenisi kalır", [[t(0), True], [t(24), True], [t(48), True]], 1),
        ("Sıfır saklama bir sayılır", [[t(0), True], [t(24), True]], 0),
        ("Başarısız deneme en yeni tamdan eskiyse silinir", [[t(0), True], [t(10), False], [t(24), True]], 5),
        ("En yeni tamdan yeni yarım yedek sürüyor olabilir: kalır", [[t(0), True], [t(24), True], [t(30), False]], 5),
        ("Hiç tam yedek yoksa yarımlar kalır", [[t(0), False], [t(24), False]], 2),
        ("Aynı saniyenin ikinci yedeği sonra gelir", [[t(5, 1, 1), True], [t(5, 1, 1, 2), True], [t(5, 1, 1, 3), True]], 2),
        ("On ikinci yedek ikinciden sonra gelir", [[t(5, 1, 1, 12), True], [t(5, 1, 1, 2), True], [t(5, 1, 1), True]], 1),
        ("Yedek adı olmayan klasörlere dokunulmaz", [["notlar", False], ["eski", True], [t(0), True], [t(24), True]], 1),
        ("Sırasız verilen yedekler", [[t(72), True], [t(0), True], [t(48), False], [t(24), True]], 2),
        ("Tamlar arasında yarım: tamlar saklanır, yarım gider", [[t(0), True], [t(12), False], [t(24), True], [t(36), False], [t(48), True]], 3),
        ("Yedi günlük döndürme", [[t(d * 24), True] for d in range(10)], 7),
    ]
    return [{"name": n, "found": f, "keep": k, "removed": rotation(f, k)} for n, f, k in rows]


def name_cases() -> list[dict]:
    names = [
        "2026-10-29T12-04-05Z",
        "2026-10-29T12-04-05Z-2",
        "2026-10-29T12-04-05Z-12",
        "2026-10-29T12-04-05Z-1",
        "2026-10-29T12-04-05Z-02",
        "2026-10-29T12-04-05Z-",
        "2026-10-29T12:04:05Z",
        "2026-10-29 12-04-05Z",
        "2026-10-29T12-04-05",
        "yedek",
        "2026-10-29T12-04-05Z.yarim",
    ]
    return [{"name": n, "order": order(n)} for n in names]


def build() -> dict:
    return {
        "about": "docs/adr/0739: the result cache's keys (§2) and rotated backups (§4); scripts/fixtures/job_cache_cases.py",
        "keys": key_cases(),
        "rotations": rotation_cases(),
        "names": name_cases(),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    data = build()
    # Same keys where the ADR says the input or the asker does not change the result; others differ.
    keys = {c["name"]: c["key"] for c in data["keys"]}
    assert keys["Dışa aktarma: girdisiz"] == keys["Dışa aktarma: boş girdi aynı anahtar"] == keys[
        "Dışa aktarma: expectedRevision anahtara girmez"] == keys["Dışa aktarma: başka kullanıcı aynı anahtar"]
    assert keys["Atlas"] == keys["Atlas: etiket ad, anahtara girmez"] == keys["Atlas: bütçe anahtara girmez"]
    assert keys["Atlas"] != keys["Atlas: başka kullanıcı başka anahtar"] != keys["Atlas: ertesi gün başka anahtar"]
    assert keys["Araç dosyaya"] == keys["Araç dosyaya: anahtarların sırası önemsiz"]
    assert keys["Dışa aktarma: girdisiz"] != keys["Dışa aktarma: sunucu yeniden derlendi"] != keys[
        "Dışa aktarma: sunucunun başka sürümü"]
    assert keys["Atlas"] != keys["Atlas: araç programı yeniden derlendi"]
    assert sum(1 for c in data["keys"] if c["key"] is None) == 4
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    cases = len(data["keys"]) + len(data["rotations"]) + len(data["names"])
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{cases} durum güncel")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{cases} durum yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
