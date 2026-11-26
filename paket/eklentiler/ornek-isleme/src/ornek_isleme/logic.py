"""Alan dilimleri: Python ile yazılmış bir İşlemler aracının örneği (CLAUDE.md §26).

Bir işlem aracı iki parçadır:

- **Tanım** (tools/alan-dilimleri.tool.json): adı, kategorisi (`category`), çıktığı modüller (`modules`), parametreleri
  ve çıktıları. KentOS aracı kodu çalıştırmadan bu dosyadan okur; pencere, komut satırı, model, Python ve MCP aynı
  tanımı gösterir.
- **İş** (bu dosya): `@ext.processing("<kimlik>")` ile işaretli fonksiyon. Parametreleri tanımdaki adlarıyla alır;
  girdiler veri olarak gelir (her nesne GeoJSON geometrisi ve öznitelikleriyle), çıktılar tanımdaki adlarıyla döner.
  KentOS dönen her şeyi tanıma göre denetler ve tek geri alma adımında yazar.

Kütüphane gerekmez: alan, halkaların düzlem koordinatlarından (shoelace) hesaplanır.
"""

from kentos import ext


def halka_alani(halka) -> float:
    """Bir halkanın işaretsiz alanı: köşeleri [x, y] olan kapalı ya da açık liste."""
    toplam = 0.0
    n = len(halka)
    for i in range(n):
        x1, y1 = halka[i][0], halka[i][1]
        x2, y2 = halka[(i + 1) % n][0], halka[(i + 1) % n][1]
        toplam += x1 * y2 - x2 * y1
    return abs(toplam) / 2.0


def alan(geometri: dict) -> float:
    """GeoJSON Polygon ya da MultiPolygon'un alanı: dış halkalardan delikler düşülür."""
    tur = geometri.get("type")
    if tur == "Polygon":
        parcalar = [geometri["coordinates"]]
    elif tur == "MultiPolygon":
        parcalar = geometri["coordinates"]
    else:
        return 0.0
    return sum(halka_alani(p[0]) - sum(halka_alani(d) for d in p[1:]) for p in parcalar if p)


def sinirlari_oku(metin: str) -> list[float]:
    """“500, 1000, 5000” → [500.0, 1000.0, 5000.0]; boş, sayı olmayan ya da artmayan sınırda nedenini söyleyen hata."""
    parcalar = [p.strip().replace(" ", "") for p in (metin or "").split(",") if p.strip()]
    if not parcalar:
        raise ValueError("Sınır yok: artan sıralı alan sınırlarını virgülle yazın (ör. 500, 1000, 5000).")
    try:
        sinirlar = [float(p) for p in parcalar]
    except ValueError:
        raise ValueError(f"“{metin}” sayı değil: sınırları virgülle ayrılmış sayılar olarak yazın.") from None
    if any(b <= a for a, b in zip(sinirlar, sinirlar[1:])) or sinirlar[0] <= 0:
        raise ValueError("Sınırlar sıfırdan büyük ve artan sırada olmalı (ör. 500, 1000, 5000).")
    return sinirlar


def _sayi(deger: float) -> str:
    return f"{deger:g}"


def dilim_adi(alan_m2: float, sinirlar: list[float]) -> str:
    """Alanın dilimi: ilk sınırdan küçükse “500 m² altı”, iki sınır arasındaysa “500–1000 m²”, son sınır ve üstü."""
    if alan_m2 < sinirlar[0]:
        return f"{_sayi(sinirlar[0])} m² altı"
    for alt, ust in zip(sinirlar, sinirlar[1:]):
        if alan_m2 < ust:
            return f"{_sayi(alt)}–{_sayi(ust)} m²"
    return f"{_sayi(sinirlar[-1])} m² ve üstü"


def dilim_adlari(sinirlar: list[float]) -> list[str]:
    """Bütün dilimlerin adları, küçükten büyüğe: tablo her dilimi, alanı olmasa da yazar."""
    adlar = [f"{_sayi(sinirlar[0])} m² altı"]
    adlar += [f"{_sayi(a)}–{_sayi(b)} m²" for a, b in zip(sinirlar, sinirlar[1:])]
    adlar.append(f"{_sayi(sinirlar[-1])} m² ve üstü")
    return adlar


@ext.processing("alan-dilimleri")
def alan_dilimleri(ctx: ext.ProcessingContext, alanlar: ext.FeatureInput, sinirlar: str) -> dict:
    """Her alanın alanı ve dilimi yeni katmana; dilimlerin sayısı ve toplam alanı tabloya."""
    esikler = sinirlari_oku(sinirlar)
    nesneler = list(ctx.read_features(alanlar))
    sayilar = {ad: [0, 0.0] for ad in dilim_adlari(esikler)}
    cikti = []
    for n, f in enumerate(nesneler):
        ctx.progress(n, len(nesneler), "Alanlar hesaplanıyor")
        alan_m2 = alan(f.geometry)
        dilim = dilim_adi(alan_m2, esikler)
        sayilar[dilim][0] += 1
        sayilar[dilim][1] += alan_m2
        cikti.append(ext.NewFeature(source=f, attrs={"alan_m2": round(alan_m2, 2), "dilim": dilim}))
    tablo = [
        {"dilim": ad, "alan sayısı": sayi, "toplam alan (m²)": round(toplam, 2)}
        for ad, (sayi, toplam) in sayilar.items()
    ]
    ctx.summary(f"{len(nesneler)} alan {len(sayilar)} dilime ayrıldı.")
    return {"alanlar": cikti, "tablo": tablo, "sayi": len(nesneler)}
