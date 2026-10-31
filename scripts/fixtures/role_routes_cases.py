"""Writes fixtures/acl/v1/roles.json: what each project role may do through the server's own commands
(docs/adr/0765), built here without KentOS code from the rules the ADRs state.

A role's permissions (docs/adr/0015, 0411 §2, §6): a viewer reads, downloads and sees the history; a commenter
also comments; a Veri girişi reads, sees the history, comments and writes through forms but never downloads (a
download would hold the fields its forms do not show) and never writes objects freely; an editor writes objects,
runs jobs and uses forms; a manager also edits the project, shares it, designs and publishes forms. The tenant
lets viewers download (its default).

What each operation asks (docs/adr/0411, 0764): making, reshaping or deleting an object asks object writing;
changing a record's attribute asks object writing, or forms for a Veri girişi when the layer's live form shows the
field (a field the form does not show it may not write); changing the project (layer tree, settings) asks project
editing, and its forms' draft also asks form design, a new release form publishing; a download asks downloading;
sharing asks sharing.

    python3 scripts/fixtures/role_routes_cases.py           # writes the file
    python3 scripts/fixtures/role_routes_cases.py --check   # compares it with the one on disk
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/acl/v1/roles.json"

ROLES = {
    "viewer": {"read", "download", "history"},
    "commenter": {"read", "download", "history", "comment"},
    "entry": {"read", "history", "comment", "forms.use"},
    "editor": {"read", "download", "history", "comment", "feature.write", "jobs.run", "forms.use"},
    "manager": {"read", "download", "history", "comment", "feature.write", "jobs.run", "edit", "share",
                "forms.design", "forms.publish", "forms.use"},
}

# Each operation: what it asks, and whether a Veri girişi's forms may stand in for object writing.
OPERATIONS = [
    ("create", "Bir bina ekler", {"feature.write"}, False),
    ("editShown", "Formun gösterdiği Durum alanını değiştirir", {"feature.write"}, True),
    ("editHidden", "Formun göstermediği Not alanını değiştirir", {"feature.write"}, False),
    ("editShape", "Binanın şeklini değiştirir", {"feature.write"}, False),
    ("delete", "Binayı siler", {"feature.write"}, False),
    ("designForm", "Formun taslağını değiştirir", {"edit", "forms.design"}, False),
    ("publishForm", "Formun yeni sürümünü yayımlar", {"edit", "forms.publish"}, False),
    ("layers", "Katmanın adını değiştirir", {"edit"}, False),
    ("download", "Projeyi .kcad olarak indirir", {"download"}, False),
    ("share", "Projeyi başka birine izleyici olarak açar", {"share"}, False),
]


def allowed(role, needs, through_forms):
    have = ROLES[role]
    if needs <= have:
        return True
    # A Veri girişi writes a shown attribute through the layer's live form instead of writing objects.
    return through_forms and "forms.use" in have and "feature.write" not in have


def document():
    cases = []
    for role in ROLES:
        for op, words, needs, through_forms in OPERATIONS:
            cases.append({"name": f"{role}: {words}", "role": role, "operation": op,
                          "allowed": allowed(role, needs, through_forms)})
    return {"format": "kentos.role-routes", "version": 1,
            "note": "ADR 0765: her proje rolünün sunucunun kendi komutlarıyla neyi yapabildiği, ADR 0015 ve 0411'in "
                    "izin tablosundan ve ADR 0764'ün Veri girişi kuralından, KentOS kodu olmadan "
                    "(scripts/fixtures/role_routes_cases.py). Reddedilen her işlem 403'tür ve hiçbir şey yazmaz.",
            "roles": {r: sorted(p) for r, p in ROLES.items()}, "cases": cases}


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    n = len(ROLES) * len(OPERATIONS)
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: {n} durum güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {n} durum")


if __name__ == "__main__":
    main()
