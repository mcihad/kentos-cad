#!/usr/bin/env python3
"""Kurtar: the shared cases of file.recover (docs/adr/1013), written without KentOS code.

    python3 scripts/fixtures/recover_cases.py           # writes fixtures/kcad/recover/v1/*.kcad and
                                                        # fixtures/commands/v1/file.recover.json
    python3 scripts/fixtures/recover_cases.py --check   # writes nothing; compares

The damaged files are made here from the valid fixtures/kcad/v2/drawing.kcad, byte by byte, and what the recovery
must give follows from how each damage was made, the object ids read by the independent reader tools/kcad/kcad.py:

- the checksum's last byte changed, or seven bytes added after it: every object comes back, nothing is lost, and the
  container says which;
- object k's 16-byte id (after its CBOR head 0x50) replaced by object k−1's: object k alone is lost, with the strict
  reader's `duplicate_uid`; the others come back in their order;
- the file cut five bytes into object k's id: objects 0…k−1 come back; the keys that come after the objects in the
  profile's key order and are required (`settings`, then `activeLayer`) are put in place, said in that order; object
  k, read only as far as the cut, is said lost (k + 1 objects read), not dropped unsaid;
- a file shorter than a header, or a JSON text: nothing to recover (`kcad_unrecoverable`);
- the source read whole: said whole, no copy written.

The copy goes to `<name>-kurtarilan.kcad` beside the source unless `target` says; it is never the source and never
over an existing file. The runner (crates/native/headless/tests/headless/recover_cases.rs) copies each case's file to
its work folder as `cizim.kcad`; `$work:` names a path there.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "kcad"))
import kcad  # noqa: E402  the independent reader

SOURCE = ROOT / "fixtures" / "kcad" / "v2" / "drawing.kcad"
FILES = ROOT / "fixtures" / "kcad" / "recover" / "v1"
OUT = ROOT / "fixtures" / "commands" / "v1" / "file.recover.json"
DEFAULT = "$work:cizim-kurtarilan.kcad"


def uids():
    return kcad.read(SOURCE.read_bytes())[1]["uids"]


def place(data, uid):
    needle = bytes([0x50]) + bytes.fromhex(uid.replace("-", ""))
    at = data.find(needle)
    assert at >= 0 and data.find(needle, at + 1) < 0, "the id is once in the file"
    return at


def files():
    data = SOURCE.read_bytes()
    ids = uids()
    out = {"butun.kcad": data}
    flipped = bytearray(data)
    flipped[-1] ^= 0xFF
    out["ozet.kcad"] = bytes(flipped)
    out["fazla.kcad"] = data + b"\x00" * 7
    for k in (1, 7, len(ids) - 1):
        dup = bytearray(data)
        at = place(data, ids[k])
        dup[at + 1:at + 17] = bytes.fromhex(ids[k - 1].replace("-", ""))
        out[f"kimlik-{k}.kcad"] = bytes(dup)
    for k in (2, 8):
        out[f"kesik-{k}.kcad"] = data[:place(data, ids[k]) + 5]
    out["baslik.kcad"] = data[:20]
    out["json.kcad"] = b'{"format": "kentos.document", "version": 1}'
    return out


def recovered(read, kept, lost, container=(), whole=False, written=None, target=DEFAULT):
    return {"target": target, "whole": whole, "written": written, "containerHas": list(container),
            "objectsRead": read, "objectsKept": len(kept), "lost": lost, "uids": kept}


def cases():
    ids = uids()
    n = len(ids)
    out = []

    def case(name, file, expect, input_=None, op="plan", make=()):
        out.append({"name": name, "file": file, "op": op, "input": input_ or {}, "make": list(make), "expect": expect})

    def refused(code, path):
        return {"status": "failed", "error": {"code": code, "path": path}}

    case("Sağlam dosya: bütün, kopya yazılmaz", "butun.kcad", recovered(n, ids, [], whole=True, written=False))
    case("Sağlam dosya, yürüt: yine yazılmaz", "butun.kcad", recovered(n, ids, [], whole=True, written=False), op="execute")
    case("Özeti tutmayan dosya: bütün nesneler gelir", "ozet.kcad",
         recovered(n, ids, [], container=["özeti tutmuyor"], written=False))
    case("Sonunda fazla bayt: bütün nesneler gelir", "fazla.kcad",
         recovered(n, ids, [], container=["fazla bayt"], written=False))
    for k in (1, 7, n - 1):
        kept = [u for i, u in enumerate(ids) if i != k]
        case(f"{k}. nesne {k - 1}. nesnenin kimliğini taşıyor: yalnız o çıkar", f"kimlik-{k}.kcad",
             recovered(n, kept, [{"part": f"entities[{k}]", "uid": ids[k - 1].replace("-", ""), "code": "duplicate_uid"}],
                       container=["özeti tutmuyor"], written=False))
    for k in (2, 8):
        case(f"{k}. nesnenin içinde kesik: önceki {k} nesne gelir, ayarlar ve etkin katman yerine konur", f"kesik-{k}.kcad",
             dict(recovered(k + 1, ids[:k], [{"part": "document.settings"}, {"part": "document.activeLayer"},
                                             {"part": f"entities[{k}]"}],
                            container=["kesik"], written=False)))
    case("Başlıktan kısa dosya: kurtarılamaz", "baslik.kcad", refused("kcad_unrecoverable", "path"))
    case("KCAD değil (JSON): kurtarılamaz", "json.kcad", refused("kcad_unrecoverable", "path"))
    case("Plan yazmaz", "kimlik-7.kcad",
         recovered(n, [u for i, u in enumerate(ids) if i != 7],
                   [{"part": "entities[7]", "uid": ids[6].replace("-", ""), "code": "duplicate_uid"}],
                   container=["özeti tutmuyor"], written=False))
    case("Doğrulama yazmaz", "kimlik-7.kcad", {"status": "completed", "output": None}, op="validate")
    case("Yürüt: kopya kaynağın yanına yazılır, açılır", "kimlik-7.kcad",
         recovered(n, [u for i, u in enumerate(ids) if i != 7],
                   [{"part": "entities[7]", "uid": ids[6].replace("-", ""), "code": "duplicate_uid"}],
                   container=["özeti tutmuyor"], written=True), op="execute")
    case("Yürüt, verilen hedefe", "kesik-8.kcad",
         recovered(9, ids[:8], [{"part": "document.settings"}, {"part": "document.activeLayer"}, {"part": "entities[8]"}],
                   container=["kesik"], written=True, target="$work:baska.kcad"),
         input_={"target": "$work:baska.kcad"}, op="execute")
    case("Kaynak göreli yol: ret", "ozet.kcad", refused("invalid_file_path", "path"), input_={"path": "cizim.kcad"})
    case("Kaynak .kcad değil: ret", "ozet.kcad", refused("invalid_file_path", "path"), input_={"path": "$work:cizim.dxf"})
    case("Hedef göreli: ret", "ozet.kcad", refused("invalid_file_path", "target"), input_={"target": "baska.kcad"})
    case("Hedef .kcad değil: ret", "ozet.kcad", refused("invalid_file_path", "target"), input_={"target": "$work:baska.txt"})
    case("Hedef kaynağın kendisi: ret", "ozet.kcad", refused("invalid_file_path", "target"), input_={"target": "$work:cizim.kcad"})
    case("Kaynak yok: ret", "ozet.kcad", refused("file_not_found", "path"), input_={"path": "$work:yok.kcad"})
    case("Hedef zaten var: ret, üzerine yazılmaz", "ozet.kcad", refused("file_exists", "target"), op="execute",
         make=["cizim-kurtarilan.kcad"])
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "file.recover", "commandVersion": 1,
            "host": "headless-file", "title": "Bozuk çizimi kurtar",
            "note": "ADR 1013. Bağımsız başvuru scripts/fixtures/recover_cases.py: hasarlar geçerli drawing.kcad'den bayt bayt "
                    "yapılır, beklenen sonuç hasarın yapılışından ve tools/kcad/kcad.py'nin okuduğu kimliklerden gelir; KentOS kodu yok.",
            "cases": cases()}


def main():
    made = files()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        same = same and all((FILES / name).exists() and (FILES / name).read_bytes() == data for name, data in made.items())
        print("Kurtarma ortak durumları güncel." if same else "Kurtarma ortak durumları güncel değil.")
        return 0 if same else 1
    FILES.mkdir(parents=True, exist_ok=True)
    for name, data in made.items():
        (FILES / name).write_bytes(data)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum), {len(made)} dosya")
    return 0


if __name__ == "__main__":
    sys.exit(main())
