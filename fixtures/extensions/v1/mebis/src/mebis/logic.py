"""MEBİS'in işlevleri: yalnız @ext.* ile işaretli saf fonksiyonlar (plan/eklenti.md §3.4).

Kimlikler manifestteki yerel kimliklerdir; KentOS onları içe aktarmadan bilir. Bu dosya ADIM 19'un kabul hedefidir:
`kentos.ext` adları ve bağlam API'si o adımda kesinleşir.
"""

from kentos import cad, ext

#: The grave statuses' words, as schema.json's mezar_durumu list gives them.
DURUM = {"BOS": "Boş", "DOLU": "Dolu", "AYRILMIS": "Ayrılmış"}


@ext.command("init-schema")
def init_schema(ctx: ext.CommandContext) -> ext.CommandResult:
    """schema.json'daki ada, parsel, mezar yeri ve defin katmanlarını tek adımda kurar."""
    return ctx.apply_declarative_schema("schema.json")


@ext.command("search")
def search(ctx: ext.CommandContext) -> ext.CommandResult:
    """Arama panelinin düğmesi: mezar no ya da durumunda sorgu geçen mezar yerlerini listeler. Panelin değerleri
    ctx.params'tadır (kimlikleriyle), satırlar panelin tablosunu doldurur; satırın uid'i tıklanınca mezarı gösterir."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    try:
        layer = ctx.doc.layer("mezar_yeri")
    except cad.UnknownObject:
        return ctx.message("Çizimde mezar yeri katmanı yok: önce “MEBİS katmanlarını kur”.")
    query = str(ctx.params.get("query") or "").strip().casefold()
    rows = []
    for record in ctx.doc.entities(layer=layer.id):
        # The object's attributes are on its entity; the record holds its persistent id and state stamp.
        attrs = dict(record.entity.attrs or {})
        status = attrs.get("durum", "")
        words = DURUM.get(status, status)
        # Its code or its word: "bos" and "boş" both find an empty grave.
        if query and query not in " ".join([*map(str, attrs.values()), words]).casefold():
            continue
        rows.append({"uid": record.uid, "Mezar no": attrs.get("mezar_no") or "—", "Ada no": attrs.get("ada_no") or "—",
                     "Durum": words or "—"})
    return ctx.report(rows, title="Mezar sorgulama", message=f"{len(rows)} mezar yeri bulundu.")


@ext.map_tool("single-grave")
def on_single_grave_placed(ctx: ext.MapToolContext, shape) -> ext.CommandResult:
    """Yerleştirilen 1,10 × 2,20 m şablonu mezar yeri olarak yazar; durumu 'BOS' başlar."""
    return ctx.add_feature("mezar_yeri", shape, {"durum": "BOS"})


@ext.processing("occupancy-by-island")
def occupancy_by_island(ctx: ext.ProcessingContext, islands, graves) -> dict:
    stats: dict[str, dict[str, int]] = {}
    # read_features answers Durdur between the objects (ext.Stopped): what the run wrote is taken back.
    for g in ctx.read_features(graves):
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
                        args={"uids": [grave.id], "operation": "attributes", "attrs": {"durum": "DOLU"}},
                    ),
                )
            )
    return findings


@ext.expr("mezar-etiketi")
def mebis_mezar_etiketi(mezar_no: str, durum: str, merhum_adi: str | None = None) -> str:
    return f"{mezar_no}\n({merhum_adi})" if durum == "DOLU" and merhum_adi else f"{mezar_no} [{durum}]"
