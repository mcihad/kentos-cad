"""İmar Uygunluk Denetimi'nin işlevleri: yalnız @ext.* ile işaretli saf fonksiyonlar (plan/eklenti.md §3.4).

Kimlikler manifestteki yerel kimliklerdir. `kentos.ext` adları ve bağlam API'si ADIM 19'da kesinleşir; bu dosya o
adımın kabul hedeflerinden biridir. shapely'nin 2.0 serisine bağlıdır (P05: MEBİS 2.2'yi kilitler).
"""

import shapely

from kentos import ext


@ext.command("check-setbacks")
def check_setbacks(ctx: ext.CommandContext) -> ext.CommandResult:
    """Seçili yapıların parsel sınırına en kısa uzaklığını hesaplar; plan notundaki mesafeden azsa işaretler."""
    limit = float(ctx.settings.get("yaklasma_m", 5.0))
    findings = []
    for building in ctx.selection("yapi"):
        parcel = ctx.related(building, "parsel")
        if parcel is None:
            continue
        gap = shapely.distance(building.shape, parcel.shape.boundary)
        if gap < limit:
            findings.append({"yapi": building.id, "mesafe_m": round(gap, 2), "sinir_m": limit})
    return ctx.report(findings, title="Yaklaşma mesafesi denetimi")
