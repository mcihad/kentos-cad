"""MEBİS'in işlevleri: yalnız @ext.* ile işaretli saf fonksiyonlar (plan/eklenti.md §3.4).

Kimlikler manifestteki yerel kimliklerdir; KentOS onları içe aktarmadan bilir. Bu dosya ADIM 19'un kabul hedefidir:
`kentos.ext` adları ve bağlam API'si o adımda kesinleşir.
"""

from kentos import ext


@ext.command("init-schema")
def init_schema(ctx: ext.CommandContext) -> ext.CommandResult:
    """schema.json'daki ada, parsel, mezar yeri ve defin katmanlarını tek adımda kurar."""
    return ctx.apply_declarative_schema("schema.json")


@ext.command("search")
def search(ctx: ext.CommandContext) -> ext.CommandResult:
    """Arama panelinin düğmesi: mezar no ya da durumunda sorgu geçen mezar yerlerini listeler. Panelin değerleri
    ctx.params'tadır (kimlikleriyle), satırlar panelin tablosunu doldurur."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    query = str(ctx.params.get("query") or "").strip().casefold()
    rows = []
    for record in ctx.doc.entities(layer="mezar_yeri"):
        attrs = dict(getattr(record, "attrs", None) or {})
        if query and query not in " ".join(str(v) for v in attrs.values()).casefold():
            continue
        rows.append({"mezar_no": attrs.get("mezar_no", ""), "ada_no": attrs.get("ada_no", ""),
                     "durum": attrs.get("durum", "")})
    return ctx.report(rows, title="Mezar sorgulama", message=f"{len(rows)} mezar yeri bulundu.")


@ext.map_tool("single-grave")
def on_single_grave_placed(ctx: ext.MapToolContext, shape) -> ext.CommandResult:
    """Yerleştirilen 1,10 × 2,20 m şablonu mezar yeri olarak yazar; durumu 'BOS' başlar."""
    return ctx.add_feature("mezar_yeri", shape, {"durum": "BOS"})


@ext.processing("occupancy-by-island")
def occupancy_by_island(ctx: ext.ProcessingContext, islands, graves) -> dict:
    stats: dict[str, dict[str, int]] = {}
    for g in ctx.read_features(graves):
        if ctx.is_stopped():
            return ctx.cancelled()
        row = stats.setdefault(g.attrs.get("ada_no", "BELIRSIZ"), {"toplam": 0, "dolu": 0})
        row["toplam"] += 1
        row["dolu"] += g.attrs.get("durum") == "DOLU"
    return {"report": [{"ada_no": k, **v} for k, v in stats.items()]}


@ext.audit("status-mismatch")
def audit_status_mismatch(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    findings = []
    burials = facts.group_related("defin_kaydi", foreign_key="mezar_id")
    for grave in facts.features("mezar_yeri"):
        if grave.attrs.get("durum") == "BOS" and burials.get(grave.id):
            findings.append(
                ext.AuditFinding(
                    severity="error",
                    feature_id=grave.id,
                    message=f"Mezar ({grave.attrs.get('mezar_no')}) için defin kaydı var ama durumu 'BOŞ'.",
                    fix=ext.SemanticFix(
                        title="Durumu 'DOLU' yap",
                        command="cad.entities.set",
                        args={"uids": [grave.id], "attrs": {"durum": "DOLU"}},
                    ),
                )
            )
    return findings


@ext.expr("mezar-etiketi")
def mebis_mezar_etiketi(mezar_no: str, durum: str, merhum_adi: str | None = None) -> str:
    return f"{mezar_no}\n({merhum_adi})" if durum == "DOLU" and merhum_adi else f"{mezar_no} [{durum}]"
