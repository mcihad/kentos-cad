"""Writes fixtures/commands/v1/cad.forms.recompute.json: the shared cases of `cad.forms.recompute` v1 (Hesapları yenile,
docs/adr/0421), their expectations built here without KentOS code: every record of the form's layer completed by the
independent form twin of scripts/fixtures/forms_cases.py (`Form.complete`, no defaults), a record whose completion
has an error left as it was and named in `failed`, the ones that change written as one undo step.

    python3 scripts/fixtures/forms_recompute_cases.py           # writes the file
    python3 scripts/fixtures/forms_recompute_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import forms_cases as fc  # noqa: E402  (the form twin, written from the ADRs without KentOS code)

OUT = ROOT / "fixtures/commands/v1/cad.forms.recompute.json"
ANY = fc.ANY
BODY = fc.CALC_BODY
FIELDS = fc.CALC_FIELDS

# The layer's records: the fill cases' ones, and three more for clearing, overriding and a calculation that does not fit.
ENTITIES = fc.FILL_ENTITIES + [
    {"kind": "polygon", "id": 7, "layerId": "bina",
     "attrs": {"Durum": "iyi", "Kat": "4", "Açıklama": "Eski not, artık gizli.", "Yükseklik": "50"}, "pts": fc.square(100, 0)},
    {"kind": "polygon", "id": 8, "layerId": "bina", "attrs": {"Durum": "iyi", "Kat": "20", "Puan": "3"}, "pts": fc.square(120, 0)},
]
IDS = [e["id"] for e in ENTITIES]
ON_LAYER = [e["id"] for e in ENTITIES if e["layerId"] == "bina"]
ATTRS = {e["id"]: e["attrs"] for e in ENTITIES}
LOCKED = "“Kilitli bina” katmanı kilitli; kayıtlar değiştirilemez. Kilidi Katmanlar panelinden açın."


def body_with(change):
    b = copy.deepcopy(BODY)
    change(b)
    assert fc.kc.body_problem(b) is None
    return b


def item(b, i):
    def walk(items):
        for x in items:
            if x.get("id") == i:
                return x
            found = walk(x.get("items", []))
            if found:
                return found
    return walk(b["items"])


def form(body=BODY, fid="hesap", name="Bina hesabı", layer="bina", released=True, draft=None):
    f = {"id": fid, "name": name, "layer": layer, "draft": copy.deepcopy(draft or body)}
    if released:
        f["releases"] = [fc.release_of(body, 1)]
    return f


def recompute(inp, forms, body, variables=(), now=None, op="execute", undo=0):
    """The step and the records afterwards: each completed by the twin, by the layer's order."""
    now = dict(now or ATTRS)
    f = fc.Form(body, FIELDS, variables)
    changed, failed, after = [], [], dict(now)
    for k in ON_LAYER:
        done, issues = f.complete(dict(now[k]), fc.OBJECT_OF.get(k, fc.SQUARE), False)
        if fc.errors(issues):
            failed.append(k)
        elif done != now[k]:
            changed.append(k)
            after[k] = done
    out = {"checked": len(ON_LAYER), "changed": [f"$uidOf:{k}" for k in changed]}
    if failed:
        out["failed"] = [f"$uidOf:{k}" for k in failed]
    if not inp.get("draft"):
        out["version"] = 1
    out["revision"] = "$current"
    warnings = [{"code": "form_calc_failed", "message": ANY, "path": "form"}] if failed else []
    result = {"status": "completed", "output": out, "warnings": warnings}
    if op == "plan":
        expect = {"ids": IDS, "fields": {str(k): {"attrs": now[k]} for k in ON_LAYER}, "canUndo": undo > 0,
                  "canRedo": False, "revision": "same"}
        return {"op": "plan", "input": inp, "result": result, "expect": expect}, now
    expect = {"ids": IDS, "fields": {str(k): {"attrs": after[k]} for k in IDS}, "canUndo": undo + bool(changed) > 0,
              "canRedo": False}
    expect.update({"dirty": True, "revision": "changed"} if changed else {"revision": "same"})
    return {"op": "execute", "input": inp, "result": result, "expect": expect}, after


def refused(inp, code, message, path, status="failed"):
    result = fc.failed(code, message, path, status)
    return {"op": "execute", "input": inp, "result": result,
            "expect": {"ids": IDS, "fields": {str(k): {"attrs": ATTRS[k]} for k in IDS}, "canUndo": False,
                       "canRedo": False, "revision": "same", "dirty": False}}


CASES = []


def case(name, steps, forms):
    CASES.append({"name": name, "setup": fc.setup(forms, FIELDS, ENTITIES), "steps": steps})


def cases():
    F = [form()]
    one, after = recompute({"form": "hesap"}, F, BODY)
    case("son yayınla bütün kayıtlar: değişenler tek adımda, sığmayan hesap kalır", [one], F)
    two, _ = recompute({"form": "hesap"}, F, BODY, now=after, undo=1)
    case("ikinci kez yenilemek bir şey değiştirmez", [one, two], F)
    plan, _ = recompute({"form": "hesap"}, F, BODY, op="plan")
    case("plan yazmaz, değişecekleri söyler", [plan], F)
    undo = {"op": "undo", "expect": {"ids": IDS, "fields": {str(k): {"attrs": ATTRS[k]} for k in IDS}, "canUndo": False,
                                     "canRedo": True}}
    case("geri alma bütün kayıtları birlikte geri getirir", [one, undo], F)

    # The draft differs from the release: four metres a floor.
    four = body_with(lambda b: item(b, "yukseklik").update(calculate="Kat * 4"))
    D = [form(BODY, draft=four)]
    one, _ = recompute({"form": "hesap", "draft": True}, D, four)
    case("taslakla: taslağın hesabı", [one], D)
    one, _ = recompute({"form": "hesap"}, D, BODY)
    case("taslak varken yayınla: yayının hesabı", [one], D)

    hidden = body_with(lambda b: b.update(clearHidden=True))
    H = [form(hidden)]
    one, _ = recompute({"form": "hesap"}, H, hidden)
    case("gizlenenleri silen form: gizli açıklamalar boşalır", [one], H)

    over = body_with(lambda b: item(b, "yukseklik").update(overridable=True))
    O = [form(over)]
    one, _ = recompute({"form": "hesap"}, O, over)
    case("değiştirilebilir hesap: elle yazılan yükseklik kalır", [one], O)

    no_calc = body_with(lambda b: [item(b, i).pop("calculate") for i in ("yukseklik", "taban", "riskli", "puan")])
    N = [form(no_calc)]
    one, _ = recompute({"form": "hesap"}, N, no_calc)
    case("hesabı olmayan form: hiçbir kayıt değişmez", [one], N)

    who = body_with(lambda b: item(b, "kayit").update(calculate="@kullanici", defaultExpr=None) or item(b, "kayit").pop("defaultExpr"))
    W = [form(who)]
    one, _ = recompute({"form": "hesap"}, W, who)
    case("kullanıcıyı okuyan hesap: kullanıcı verilmeyince dokunulmaz", [one], W)
    one, _ = recompute({"form": "hesap", **fc.MOMENT_GIVEN}, W, who, variables=fc.GIVEN)
    case("kullanıcıyı okuyan hesap: verilen kullanıcı yazılır", [one], W)
    one, _ = recompute({"form": "hesap", "expectedRevision": "$current"}, F, BODY)
    case("beklenen sürüm şimdiki: yazılır", [one], F)

    taslak = form(BODY, fid="taslak", name="Taslak", released=False)
    one, _ = recompute({"form": "taslak", "draft": True}, [taslak], BODY)
    case("yayınlanmamış form taslakla yenilenir", [one], [taslak])
    case("yayınlanmamış form taslaksız reddedilir", [refused({"form": "taslak"}, "form_unpublished", ANY, "draft")], [taslak])
    case("olmayan form", [refused({"form": "yok"}, "unknown_form", ANY, "form")], F)
    case("geçersiz an formdan önce", [refused(
        {"form": "yok", "moment": "dün"}, "invalid_moment",
        "“dün” bir an değil. Bir tarih (2026-10-16) ya da tarih ve yerel saat (2026-10-16T14:30:00) yazın.", "moment")], F)
    case("geçersiz kullanıcı", [refused(
        {"form": "hesap", "user": " Ayşe"}, "invalid_user",
        "Kullanıcının adı boş olamaz, başında ve sonunda boşluk olmaz, en çok 64 karakterdir.", "user")], F)
    case("geçersiz beklenen sürüm", [refused({"form": "hesap", "expectedRevision": "07"}, "invalid_revision", fc.REVISION,
                                             "expectedRevision")], F)
    case("çakışan beklenen sürüm", [refused({"form": "hesap", "expectedRevision": "999"}, "revision_conflict", fc.CONFLICT,
                                            "expectedRevision", "conflict")], F)
    broken = {**form(BODY), "draft": copy.deepcopy(fc.BROKEN), "releases": []}
    case("ön denetimde hatası olan form", [refused({"form": "hesap", "draft": True}, "form_preflight", ANY, "form")], [broken])
    locked = form(fc.BODY, fid="kilit", name="Kilitli bina", layer="kilitli")
    case("kilitli katman", [refused({"form": "kilit"}, "layer_locked", LOCKED, "form")], [locked])
    return CASES


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.forms.recompute", "commandVersion": 1,
            "title": "Hesapları yenile",
            "note": "ADR 0421: formun katmanındaki her kayıt formun bugünkü kurallarıyla tamamlanır; varsayılanlar dolmaz. "
                    "Written by scripts/fixtures/forms_recompute_cases.py without KentOS code; do not edit by hand.",
            "setup": fc.setup([form()], FIELDS, ENTITIES), "cases": cases()}


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    n = len(json.loads(text)["cases"])
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({n} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({n} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
