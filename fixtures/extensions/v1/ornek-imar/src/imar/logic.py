"""İmar Uygunluk Denetimi'nin işlevleri: yalnız @ext.* ile işaretli fonksiyonlar (plan/eklenti.md §3.4, ADIM 19).

Kimlikler manifestteki yerel kimliklerdir. shapely'nin 2.0 serisine bağlıdır (P05: MEBİS 2.2'yi kilitler); raporda
hangi sürümle çalıştığı yazar. TAKS aracı (İşlemler) girdilerini veri olarak alır; her girdi GeoJSON konuştuğu için
shapely onu doğrudan okur (shape). NumPy da bir örnektir: eklenti ortamındaki her kütüphaneyi kullanabilir.
"""

import numpy as np
import shapely
from shapely.geometry import Polygon, shape

from kentos import ext

#: Plan notundaki yaklaşma mesafesi, metre, ayarı verilmediğinde.
VARSAYILAN_M = 5.0


def _katman(doc, ad):
    """Kimliği ya da adı verilen katmanın kimliği (KentOS'un CBS şablonunda "parsel" ve "yapi" kimlikli katmanlar);
    yoksa None."""
    for layer in doc.layers():
        if layer.id == ad or getattr(layer, "name", None) == ad:
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
        return ctx.message("Çizimde 'yapi' ve 'parsel' kimlikli ya da adlı katmanlar yok.")
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


def _etiket(feature: ext.Feature, n: int) -> str:
    """Bir parselin tablodaki adı: ada/parsel öznitelikleri varsa onlar, yoksa sırası."""
    attrs = {k.casefold(): v for k, v in feature.attrs.items() if v}
    ada, parsel = attrs.get("ada"), attrs.get("parsel")
    if ada and parsel:
        return f"{ada}/{parsel}"
    return parsel or attrs.get("no") or str(n)


@ext.processing("taks")
def taks(
    ctx: ext.ProcessingContext,
    parseller: ext.FeatureInput,
    yapilar: ext.FeatureInput,
    taks_siniri: float,
) -> dict:
    """Her imar parselinin taban alanı katsayısı (Planlı Alanlar İmar Yönetmeliği md. 4: taban alanının imar parseli
    alanına oranı): yapıların parselin içinde kalan alanlarının toplamı / parsel alanı. Kesişimler shapely 2.0'ın
    vektörel işlemleriyle (STRtree), toplamlar NumPy ile (bincount); KentOS'a giden yalnız sonuçtur."""
    limit = float(taks_siniri if taks_siniri is not None else 0.4)
    ctx.progress(0, 3, "Şekiller okunuyor")
    parsel = np.array([shape(f.geometry) for f in ctx.read_features(parseller)], dtype=object)
    yapi = np.array([shape(f.geometry) for f in ctx.read_features(yapilar)], dtype=object)
    ctx.progress(1, 3, "Yapılar parsellerle kesiştiriliyor")
    taban = np.zeros(len(parsel))
    sayi = np.zeros(len(parsel), dtype=int)
    if len(parsel) and len(yapi):
        # Her (parsel, yapı) kesişen çifti bir kez; bir yapının parselden taşan kısmı o parsele sayılmaz.
        i_parsel, i_yapi = shapely.STRtree(yapi).query(parsel, predicate="intersects")
        kesisim = shapely.area(shapely.intersection(parsel[i_parsel], yapi[i_yapi]))
        icinde = kesisim > 1e-6
        taban = np.bincount(i_parsel[icinde], weights=kesisim[icinde], minlength=len(parsel))
        sayi = np.bincount(i_parsel[icinde], minlength=len(parsel))
    ctx.progress(2, 3, "TAKS hesaplanıyor")
    alan = shapely.area(parsel).astype(float) if len(parsel) else np.zeros(0)
    oran = np.divide(taban, alan, out=np.zeros_like(taban), where=alan > 0)
    asiyor = oran > limit + 1e-9
    cikti, tablo = [], []
    for n, (f, a, t, r, s, x) in enumerate(zip(parseller, alan, taban, oran, sayi, asiyor), start=1):
        durum = "aşıyor" if x else "uygun"
        cikti.append(
            ext.NewFeature(
                source=f,
                attrs={
                    "parsel_alani_m2": round(float(a), 2),
                    "taban_alani_m2": round(float(t), 2),
                    "taks": round(float(r), 3),
                    "taks_siniri": limit,
                    "yapi_sayisi": int(s),
                    "durum": durum,
                },
            )
        )
        tablo.append(
            {
                "parsel": _etiket(f, n),
                "parsel alanı (m²)": round(float(a), 2),
                "taban alanı (m²)": round(float(t), 2),
                "TAKS": round(float(r), 3),
                "sınır": limit,
                "durum": durum,
            }
        )
    ctx.select(f for f, x in zip(parseller, asiyor) if x)
    asan = int(asiyor.sum())
    ctx.log(f"{len(parsel)} parsel, {len(yapi)} yapı; shapely {shapely.__version__}, NumPy {np.__version__}.")
    sinir = f"{limit:g}".replace(".", ",")  # Türkçe ondalık virgül
    # Yalnız hesabı söyler: sonuç çizime de bir dosyaya da gidebilir, nereye gittiğini KentOS söyler.
    ctx.summary(f"{len(parsel)} parselin TAKS'ı hesaplandı; {asan} parsel plan notundaki {sinir} sınırını aşıyor.")
    return {"parseller": cikti, "tablo": tablo, "asan": asan}


@ext.expr("ada-parsel")
def ada_parsel(ada, parsel) -> str | None:
    """Bir parselin "ada/parsel" yazımı (101/1); ikisinden biri boşsa boş. İfadelerde:
    ext.tr.ornek.imar-denetim.ada-parsel(Ada, Parsel)."""
    def metin(deger):
        if deger is None:
            return ""
        if isinstance(deger, float) and deger.is_integer():
            return str(int(deger))
        return str(deger).strip()

    a, p = metin(ada), metin(parsel)
    return f"{a}/{p}" if a and p else None
