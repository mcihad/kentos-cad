"""İmar Uygunluk Denetimi'nin işlevleri: yalnız @ext.* ile işaretli fonksiyonlar (plan/eklenti.md §3.4, ADIM 19).

Kimlikler manifestteki yerel kimliklerdir. shapely'nin 2.0 serisine bağlıdır (P05: MEBİS 2.2'yi kilitler); raporda
hangi sürümle çalıştığı yazar.
"""

import shapely
from shapely.geometry import Polygon

from kentos import ext

#: Plan notundaki yaklaşma mesafesi, metre, ayarı verilmediğinde.
VARSAYILAN_M = 5.0


def _katman(doc, ad):
    """Adı verilen katmanın kimliği; yoksa None."""
    for layer in doc.layers():
        if getattr(layer, "name", None) == ad:
            return layer.id
    return None


@ext.command("check-setbacks")
def check_setbacks(ctx: ext.CommandContext) -> ext.CommandResult:
    """'yapi' katmanındaki yapıların (seçili yapı varsa yalnız onların) içinde durduğu 'parsel'in sınırına en kısa
    uzaklığı; sınırdan azsa ya da yapı bir parselin içinde değilse bir satır. Bulunan yapılar çizimde seçili kalır."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok: yapı ve parsel katmanları olan bir çizim açın.")
    limit = float(ctx.settings.get("yaklasma_m", VARSAYILAN_M))
    yapi, parsel = _katman(ctx.doc, "yapi"), _katman(ctx.doc, "parsel")
    if yapi is None or parsel is None:
        return ctx.message("Çizimde 'yapi' ve 'parsel' adlı katmanlar yok.")
    parseller = [Polygon(r.entity.pts) for r in ctx.doc.entities(layer=parsel, kinds=["polygon"])]
    yapilar = list(ctx.doc.entities(layer=yapi, kinds=["polygon"]))
    secili = set(ctx.doc.selection())
    kapsam = "seçili"
    if any(r.uid in secili for r in yapilar):
        yapilar = [r for r in yapilar if r.uid in secili]
    else:
        kapsam = "bütün"
    rows, bulunan = [], []
    for n, record in enumerate(yapilar, start=1):
        ctx.progress(n - 1, len(yapilar), "Yapılar denetleniyor")
        bina = Polygon(record.entity.pts)
        icinde = [p for p in parseller if p.contains(bina.representative_point())]
        if not icinde:
            rows.append({"yapı": n, "durum": "parsel dışında"})
            bulunan.append(record.uid)
            continue
        mesafe = float(shapely.distance(bina, icinde[0].exterior))
        if mesafe < limit:
            rows.append({"yapı": n, "mesafe_m": round(mesafe, 2), "sınır_m": limit})
            bulunan.append(record.uid)
    # Bulunanlar seçili kalır: kullanıcı onları çizimde görür, sıradaki aracı onlara uygular.
    ctx.doc.select(bulunan)
    ctx.log(f"{len(yapilar)} {kapsam} yapı, {len(parseller)} parsel denetlendi; shapely {shapely.__version__}.")
    if bulunan:
        sonuc = (
            f"{len(yapilar)} yapı denetlendi; {len(bulunan)} yapı {limit:g} m sınırın içinde ya da parsel dışında, "
            "çizimde seçildi."
        )
    else:
        sonuc = f"{len(yapilar)} yapı denetlendi; hepsi {limit:g} m sınırın dışında."
    return ctx.report(rows, title="Yaklaşma mesafesi denetimi", message=f"{sonuc} (shapely {shapely.__version__})")
