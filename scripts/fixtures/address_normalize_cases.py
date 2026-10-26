#!/usr/bin/env python3
"""Writes fixtures/commands/v1/cad.addresses.normalize.json: the shared cases of `cad.addresses.normalize` v1
(docs/adr/0647 §2), without KentOS code. The groups are the check's (scripts/fixtures/address_check_command_cases.py,
whose locator is the address reference's, geocode_cases.py): a street's doors (ADR 0579), or doors grouped by the
neighbourhood's and the district's keys, or by the district's (ADR 0647 §1), each that writes the name in more than one
way. Each group's spelling is the first of `prefer` it writes, else its most written (on a tie the one written first);
every door of it whose trimmed name is another gets that spelling in the door layer's field.

    python3 scripts/fixtures/address_normalize_cases.py           # writes the file
    python3 scripts/fixtures/address_normalize_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import address_check_command_cases as ac  # noqa: E402
import geocode_cases as ref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.addresses.normalize.json"
STEP = "Adres yazılışlarını düzelt"
FIELD = {"street": "street", "neighbourhood": "neighbourhood", "district": "district"}
WORDS = {"neighbourhood": "mahalle", "district": "ilçe"}


def groups(loc, what):
    """The groups that write the name in more than one way: (doors, spellings most written first)."""
    if what == "street":
        keyed = [[loc["doors"][i] for i in g["doors"]] for g in loc["groups"]]
    else:
        by = {}
        for d in loc["doors"]:
            k = (d["group"][1], d["group"][2]) if what == "neighbourhood" else d["group"][2]
            if (what == "neighbourhood" and not d["group"][1]) or (what == "district" and not d["group"][2]):
                continue
            by.setdefault(k, []).append(d)
        keyed = list(by.values())
    out = []
    for doors in keyed:
        written = []
        for d in doors:
            t = d["texts"][FIELD[what]]
            for w in written:
                if w[0] == t:
                    w[1] += 1
                    break
            else:
                written.append([t, 1, d["order"]])
        if len(written) < 2:
            continue
        written.sort(key=lambda w: (-w[1], w[2]))
        out.append((doors, [w[0] for w in written]))
    out.sort(key=lambda g: min(d["order"] for d in g[0]))
    return out


def normalize(setup, locator, what="street", prefer=()):
    """The answer and the doors' new attributes by entity id."""
    loc = ref.build(ac.scene(setup, locator))
    field = locator["doors"][FIELD[what]]
    ents = setup["entities"]
    answer, changed = [], {}
    for doors, spellings in groups(loc, what):
        chosen = next((p.strip() for p in prefer if p.strip() in spellings), spellings[0])
        n = 0
        for d in doors:
            if d["texts"][FIELD[what]] != chosen:
                e = ents[d["order"]]
                attrs = dict(changed.get(e["id"], e["attrs"]))
                attrs[field] = chosen
                changed[e["id"]] = attrs
                n += 1
        answer.append({"to": chosen, "from": [s for s in spellings if s != chosen], "doors": n})
    order = {e["id"]: k for k, e in enumerate(ents)}
    ids = sorted(changed, key=lambda i: order[i])
    out = {"locator": locator["id"], "what": what, "groups": answer, "written": len(ids),
           "uids": [f"$uidOf:{i}" for i in ids], "revision": "$current"}
    return out, changed


def entities(setup, changed):
    out = {}
    for e in setup["entities"]:
        if e["id"] in changed:
            out[str(e["id"])] = {**e, "attrs": changed[e["id"]]}
    return out


def ok(name, input_, setup=None, op="execute", undo=False):
    s = setup or ac.SETUP
    loc = next(l for l in s["settings"]["addressLocators"] if l["id"] == input_.get("locator", "adres-1"))
    out, changed = normalize(s, loc, input_.get("what", "street"), input_.get("prefer", ()))
    wrote = op == "execute" and bool(changed)
    expect = {"revision": "changed" if wrote else "same"}
    if wrote:
        expect["entities"] = entities(s, changed)
    steps = [{"op": op, "input": input_, "result": {"status": "completed", "output": out, "warnings": []},
              "expect": expect}]
    if undo and wrote:
        steps.append({"op": "undo", "returns": STEP, "expect": {"entities": entities(s, {i: e["attrs"] for i, e in
                                                                                        ((e["id"], e) for e in s["entities"])
                                                                                        if i in changed}),
                                                                "revision": "changed"}})
    c = {"name": name, "steps": steps}
    if setup is not None:
        c["setup"] = setup
    return c


def refused(name, input_, code, message, path, setup=None, status="failed", extra=None):
    err = {"code": code, "message": message}
    if path is not None:
        err["path"] = path
    err.update(extra or {})
    c = {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": status, "error": err},
                                  "expect": {"revision": "same"}}]}
    if setup is not None:
        c["setup"] = setup
    return c


def with_entities(ents, locators=None, locked=False, fields=None):
    s = ac.with_entities(ents, locators)
    if locked or fields:
        s = copy.deepcopy(s)
        for n in s["layers"]:
            if n["id"] == "kapi":
                n["locked"] = locked
                if fields:
                    n["fields"] = fields
    return s


def cases():
    d = ac.door
    out = [
        ok("Yollar: her grup en çok yazılanına", {}, undo=True),
        ok("Yol, adıyla: aynı sonuç", {"what": "street"}),
        ok("Tercih edilen yazılış seçilir", {"prefer": ["Atatürk Blv.", "Gazi Mustafa Kemal Cd."]}),
        ok("Grubun yazmadığı tercih yok sayılır, en çok yazılan seçilir", {"prefer": ["Gazi Bulvarı"]}),
        ok("Tercihin baştaki ve sondaki boşluğu yok sayılır", {"prefer": ["  Atatürk Blv. "]}),
        ok("Mahalleler", {"what": "neighbourhood"}),
        ok("Mahalle, tercihle", {"what": "neighbourhood", "prefer": ["Kızılay Mahallesi"]}),
        ok("İlçeler", {"what": "district"}),
        ok("Plan aynı cevabı verir, yazmaz", {"what": "district"}, op="plan"),
    ]
    tie = [d(1, "kapi", "Okul Sk.", "1"), d(2, "kapi", "Okul Sokak", "2"), d(3, "kapi", "Okul Sokak", "3"),
           d(4, "kapi", "Okul Sk.", "4")]
    out.append(ok("Eşit yazılış: önce yazılan seçilir", {}, setup=with_entities(tie)))
    spaces = [d(1, "kapi", "  Lale Sk. ", "1"), d(2, "kapi", "Lale Sk.", "2"), d(3, "kapi", "LALE SK.", "3")]
    out.append(ok("Boşluk farkı yazılmaz", {}, setup=with_entities(spaces)))
    clean = [d(1, "kapi", "Atatürk Bulvarı", "1"), d(2, "kapi", "Atatürk Bulvarı", "2")]
    out.append(ok("Temiz veri: hiçbir şey yazılmaz, adım açılmaz", {}, setup=with_entities(clean)))
    two = [d(1, "kapi", "Okul Sk.", "1", hood="Kızılay"), d(2, "kapi", "OKUL SOKAK", "1", hood="Bahçelievler")]
    out.append(ok("İki mahalledeki aynı ad iki yoldur: yazılmaz", {}, setup=with_entities(two)))
    names = [d(1, "kapi", "Lale Sk.", "1", hood="Yenimahalle", district="Çankaya"),
             d(2, "kapi", "Gül Sk.", "1", hood="YENİMAHALLE Mahallesi", district="ÇANKAYA"),
             d(3, "kapi", "Lale Sk.", "1", hood="Yenimahalle", district="Keçiören"),
             d(4, "kapi", "Gül Sk.", "2", hood="Yeni Mahalle", district="Çankaya"),
             d(5, "kapi", "Lale Sk.", "3", hood="", district="çankaya")]
    out.append(ok("İlçe üç biçimde: hepsi en çok yazılana", {"what": "district"}, setup=with_entities(names)))
    out.append(ok("Mahalle iki ilçede: yalnız kendi ilçesindekiler", {"what": "neighbourhood"},
                  setup=with_entities(names)))
    f2 = {"street": "YOL", "door": "NO"}
    second = [d(1, "kapi2", "İnönü Sk.", "1", fields=f2), d(2, "kapi2", "İNÖNÜ SOKAK", "1", fields=f2),
              d(3, "kapi2", "İnönü Sokağı", "2", fields=f2), d(4, "kapi2", "inönü sk", "3", fields=f2)]
    out.append(ok("İkinci adres verisi: başka alan adı", {"locator": "adres-2"}, setup=with_entities(second)))
    many = []
    for k in range(6):
        many.append(d(2 * k + 1, "kapi", f"Sokak {k + 1}", "1", x=k))
        many.append(d(2 * k + 2, "kapi", f"SOKAK {k + 1}", "2", x=k + 0.5))
    out.append(ok("Altı yol: hepsi düzelir (sınır yok)", {}, setup=with_entities(many)))
    out += [
        refused("Mahalle alanı olmayan adres verisi", {"locator": "adres-2", "what": "neighbourhood"}, "invalid_input",
                "“Yalnız kapılar” adres verisinin kapılarında mahalle alanı yok; önce Adres verisi penceresinde onu "
                "seçin.", "what"),
        refused("İlçe alanı olmayan adres verisi", {"locator": "adres-2", "what": "district"}, "invalid_input",
                "“Yalnız kapılar” adres verisinin kapılarında ilçe alanı yok; önce Adres verisi penceresinde onu seçin.",
                "what"),
        refused("Boş tercih", {"prefer": ["Atatürk Bulvarı", "  "]}, "invalid_input",
                "Tercih edilen yazılış boş olamaz.", "prefer[1]"),
        refused("Kapı katmanı kilitli", {}, "layer_locked",
                "“Kapı” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "locator", setup=with_entities(ac.ENTITIES, locked=True)),
        refused("Olmayan adres verisi", {"locator": "yok"}, "unknown_locator",
                "Projede “yok” kimlikli adres verisi yok.", "locator"),
        refused("Adres verisi olmayan proje", {}, "no_address_data",
                "Projede adres verisi yok; önce Adres verisi penceresinde ya da cad.addresses.define ile tanımlayın.",
                None, setup=with_entities(ac.ENTITIES, [])),
        refused("Geçersiz beklenen sürüm", {"expectedRevision": "x"}, "invalid_revision",
                "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya "
                "da komutun planından alın.", "expectedRevision"),
        refused("Çizim değişmiş", {"expectedRevision": "999"}, "revision_conflict",
                "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
                "yeniden hazırlayın.", "expectedRevision", status="conflict", extra={"revision": "$current"}),
        {"name": "validate yalnız denetler", "steps": [{"op": "validate", "input": {},
                                                       "result": {"status": "completed", "output": None,
                                                                  "warnings": []},
                                                       "expect": {"revision": "same"}}]},
    ]
    # A field's rule the spelling no longer fits: the whole command is refused.
    ruled = [d(1, "kapi", "Kısa Sk.", "1"), d(2, "kapi", "Kısa Sk.", "2"), d(3, "kapi", "KISA SOKAĞI", "3")]
    fields = [{"name": "YOL_ADI", "alias": "Yol adı", "kind": "text", "length": 9}]
    out.append(refused("Alanın kuralına uymayan yazılış: hiçbir şey yazılmaz", {"prefer": ["KISA SOKAĞI"]},
                       "invalid_attribute", "“Yol adı” alanı en çok 9 karakter alır; 11 karakter verildi.",
                       "what.YOL_ADI", setup=with_entities(ruled, fields=fields)))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.addresses.normalize", "commandVersion": 1,
            "title": "Adres yazılışlarını düzelt",
            "note": "ADR 0647 §2. Bağımsız başvuru scripts/fixtures/address_normalize_cases.py: gruplar denetimin "
                    "(address_check_command_cases.py, geocode_cases.py), seçim ve yazma ADR'nin kuralıyla; KentOS "
                    "kodu yok.",
            "setup": ac.SETUP, "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın.")
        print(f"{OUT.relative_to(ROOT)} güncel: {len(build()['cases'])} durum.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
