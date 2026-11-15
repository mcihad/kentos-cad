"""Eklenti Vitrini'nin işlevleri (samples/eklenti-vitrini; plan/eklenti.md ADIM 19).

Her işlev manifestteki yerel kimliğiyle işaretlidir: KentOS onları içe aktarmadan bilir, yalnız çağrıldığında bu
eklentinin kendi sürecinde ve kendi Python ortamında çalıştırır. Her biri bir durumu gösterir:

- ``katman_ozeti``: çizimi okuyan komut; satırlı rapor (katmanın sağ tık menüsünde de).
- ``yapilari_numarala``: çizime yazan komut; ilerleme, Durdur ve geri alma (bir çalıştırma tek geri alma adımı).
- ``hata_ornegi``: hata veren komut; KentOS hatayı, eklentinin günlüğünü ve Python iz kaydını açıkça gösterir.
- ``taks``: İşlemler aracı; shapely 2.0 ve NumPy ile hesaplar, yeni katman, tablo ve sayı döndürür.
- ``ada_parsel``: ifade işlevi; etiket metni, stil kuralı, katman süzgeci, İşlemler ve form hesabı çağırır.
- ``yapi_yerlestir``: harita aracı; manifestteki 10 × 12 m şablonu kullanıcının gösterdiği yere yazar.
- ``yapi_no``: denetim kuralı; numarasız yapıları bulur, her birine sıradaki numarayı yazan bir düzeltme önerir.
- ``parsel_cakisma``: KentOS'un ölçtüğü olgularla denetim (``facts.project``); çakışan ve arası boş parseller.
- ``katmanlari_kur``: katman şeması; schema.json'daki grubu, katmanları, değer listesini ve ilişkiyi tek adımda kurar.
- ``yapi_ara``: panelin düğmesi; "Yapı ara" panelinin değerleriyle çalışır, satırları panelin tablosuna gider.
- ``yapi_ozeti``: göstergeli panel; "Yapı ara"nın Özetle düğmesi, satırları gösterge kartları olur.
- ``deger_say``: seçicili panelin düğmesi; "Değer say"ın katmanı, alanı ve seçilen nesneleriyle değerleri sayar,
  bulduklarını panelin Seç kutusuna koyar (``ctx.panel``: çalışan kod panelini değiştirir).
- ``deger_sec``: Seç değişince çalışır; o değerdeki nesneleri çizimde seçer.
- ``katman_degerleri``: katmanın sağ tık menüsünden; "Değer say"ı o katman seçili açar (``ctx.params["layer"]``).
- ``csv_noktalar``: katmanın sağ tık menüsünden; pencerenin dosya seçicisiyle verilen CSV'deki noktaları ekler.
- ``parsel_karti``: kartlı panel; "Parsel kartı"nda alınan her parselin alanı, yapıları ve TAKS'ı bir kartta.
"""

import csv
import struct
import time
import zlib
from datetime import date, datetime

import numpy as np
import shapely
from shapely.geometry import Point, Polygon, shape

from kentos import cad, ext
from kentos import ui_panel as ui


def _yapraklar(nodes):
    """Katman ağacının katmanları (gruplar değil), ağaçtaki sırasıyla."""
    for node in nodes:
        if node.type == "layer":
            yield node
        yield from _yapraklar(node.children)


def _katman(doc, anahtar):
    """Kimliği ya da adı verilen katmanın kimliği; yoksa None. KentOS'un CBS şablonunda "parsel" ve "yapi" vardır."""
    for layer in _yapraklar(doc.layers()):
        if anahtar in (layer.id, layer.name):
            return layer.id
    return None


@ext.command("katman-ozeti")
def katman_ozeti(ctx: ext.CommandContext) -> ext.CommandResult:
    """Nesnesi olan her katmanın nesne sayısı ve alanlarının toplamı (m²)."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    satirlar = []
    for layer in _yapraklar(ctx.doc.layers()):
        kayitlar = list(ctx.doc.entities(layer=layer.id))
        if not kayitlar:
            continue
        alan = sum(Polygon(r.entity.pts).area for r in kayitlar if r.entity.kind == "polygon")
        satirlar.append({"katman": layer.name, "nesne": len(kayitlar), "alan (m²)": round(alan, 2)})
    toplam = sum(s["nesne"] for s in satirlar)
    return ctx.report(satirlar, title="Katman özeti", message=f"{len(satirlar)} katmanda {toplam} nesne.")


@ext.command("yapilari-numarala")
def yapilari_numarala(ctx: ext.CommandContext) -> ext.CommandResult:
    """'yapi' katmanındaki yapılara sırayla "no" yazar (Y-01, Y-02 …). Şeritten çalışınca önce öneki ve ilk numarayı
    sorar (ctx.dialog: KentOS'un penceresi, alanları kentos.ui_panel'in işlevleriyle, açıklaması yazılanla hesaplanır;
    İptal'de bir şey yazılmaz), bitince "Yapı ara" panelini açıp numaralananları listeler (ctx.open_panel); panelin
    düğmesinden çalışınca Y-01'den başlar. İlerleme çubuğunu ve Durdur'u göstermek için her yapıda yarım saniye bekler:
    Durdur'a basılırsa yazdıkları geri alınır."""
    yapi = _katman(ctx.doc, "yapi") if ctx.doc is not None else None
    if yapi is None:
        return ctx.message("Çizimde 'yapi' kimlikli ya da adlı katman yok.")
    kayitlar = list(ctx.doc.entities(layer=yapi, kinds=["polygon"]))
    onek, ilk = "Y-", 1
    # Panelden çalışınca ctx.params panelin değerlerini taşır; şeritten çalışınca boştur: kullanıcıya sorulur.
    if not ctx.params:
        cevap = ctx.dialog(
            "Yapıları numarala",
            # Alanlar kentos.ui_panel'in işlevleriyle: her biri bileşenin JSON'unu yazar, anahtarları adıyla ve tipiyle
            # (panelin JSON şemasından üretilir; yanlış yazılan bir anahtar Python'da TypeError olur). Açıklama hesaplıdır
            # (calculate): yazılan önek ve ilk numarayla "… B-10 … B-14 yazılır." olur, Python'suz; İlk numara boşken
            # kendi metni görünür.
            [
                ui.text(
                    f"'yapi' katmanındaki {len(kayitlar)} yapıya sırayla numara yazılır.",
                    calculate=(
                        f"eğer(boş(ilk), boş, '“yapi” katmanındaki {len(kayitlar)} yapıya ' || varsayılan(onek, '') "
                        f"|| doldur(ilk, 2) || ' … ' || varsayılan(onek, '') || doldur(ilk + {len(kayitlar) - 1}, 2) "
                        "|| ' yazılır.')"
                    ),
                ),
                # Önek ve İlk numara bir satırda yan yana (row): yer yoksa KentOS alt alta dizer.
                ui.text_field("onek", "Önek", value="Y-", row="numara"),
                ui.number_field("ilk", "İlk numara", value=1, min=1, required=True, row="numara"),
            ],
            ok="Numarala",
        )
        if cevap is None:
            return ctx.message("Vazgeçildi: numara yazılmadı.")
        onek, ilk = cevap.get("onek") or "", int(cevap.get("ilk") or 1)
    for n, kayit in enumerate(kayitlar, start=ilk):
        ctx.progress(n - ilk, len(kayitlar), f"{n - ilk + 1}. yapı numaralanıyor")
        cad.entities.set(ctx.doc, uids=[kayit.uid], operation="attributes", attrs={"no": f"{onek}{n:02d}"})
        time.sleep(0.5)
    son = ilk + len(kayitlar) - 1
    ozet = f"{len(kayitlar)} yapı numaralandı ({onek}{ilk:02d} … {onek}{son:02d})."
    if ctx.panel is None and kayitlar:
        # Şeritten çalıştı: yaptığını "Yapı ara" panelinde gösterir (ctx.open_panel: komut kendi panelini açar,
        # docs/adr/8057 §32). Satırlar Ara'nınkiyle aynı biçimde: tıklanan satır yapıyı gösterir.
        panel = ctx.open_panel("yapi-paneli")
        panel.set("sorgu", value=onek)
        panel.rows(
            "sonuc",
            [
                {"uid": k.uid, "no": f"{onek}{n:02d}", "alan (m²)": round(Polygon(k.entity.pts).area, 1)}
                for n, k in enumerate(kayitlar, start=ilk)
            ],
        )
        panel.say(ozet, "success")
    return ctx.message(ozet)


@ext.command("yakin-yapilar")
def yakin_yapilar(ctx: ext.CommandContext) -> ext.CommandResult:
    """"Yakın yapılar" panelinin Bul düğmesi ve noktası değişince: panelin nokta seçicisindeki noktaya (ctx.params
    ["nokta"]: [x, y], haritadan seçilir ya da yazılır) verilen uzaklıktaki yapılar, yakından uzağa; yapının içindeki
    noktanın uzaklığı 0'dır. Satırlar Ara'nınki gibi: tıklanan satır yapıyı gösterir."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    nokta = ctx.params.get("nokta")
    if not nokta:
        return ctx.message("Önce bir nokta verin: Haritadan seç ya da x, y.")
    uzaklik = float(ctx.params.get("uzaklik") or 25)
    yapi = _katman(ctx.doc, "yapi")
    if yapi is None:
        return ctx.message("Çizimde 'yapi' kimlikli ya da adlı katman yok.")
    merkez = Point(nokta)
    satirlar = []
    for kayit in ctx.doc.entities(layer=yapi, kinds=["polygon"]):
        mesafe = Polygon(kayit.entity.pts).distance(merkez)
        if mesafe <= uzaklik:
            satirlar.append(
                {"uid": kayit.uid, "no": kayit.entity.attrs.get("no") or "—", "uzaklık (m)": round(mesafe, 1)}
            )
    satirlar.sort(key=lambda s: s["uzaklık (m)"])
    return ctx.report(satirlar, title="Yakın yapılar", message=f"{uzaklik:g} m içinde {len(satirlar)} yapı.")


@ext.command("yapi-ara")
def yapi_ara(ctx: ext.CommandContext) -> ext.CommandResult:
    """"Yapı ara" panelinin düğmesi: numarasında aranan geçen yapıları alanlarıyla listeler. Panelin değerleri
    ctx.params'tadır (sorgu, numarasiz, katman: seçilen alan katmanının kimliği ya da None), satırlar panelin tablosunu
    doldurur (docs/adr/8057 §24)."""
    secilen = ctx.params.get("katman")
    yapi = (secilen or _katman(ctx.doc, "yapi")) if ctx.doc is not None else None
    if yapi is None:
        return ctx.message("Çizimde 'yapi' kimlikli ya da adlı katman yok.")
    sorgu = str(ctx.params.get("sorgu") or "").strip().casefold()
    numarasiz = bool(ctx.params.get("numarasiz"))
    satirlar = []
    for kayit in ctx.doc.entities(layer=yapi, kinds=["polygon"]):
        no = str((kayit.entity.attrs or {}).get("no", ""))
        if (numarasiz and no) or (sorgu and sorgu not in no.casefold()):
            continue
        # The row's uid: clicking it in the panel shows the building (not shown as a column).
        satirlar.append({"uid": kayit.uid, "no": no or "—", "alan (m²)": round(Polygon(kayit.entity.pts).area, 1)})
    return ctx.report(satirlar, title="Yapı ara", message=f"{len(satirlar)} yapı bulundu.")


@ext.command("yapi-ozeti")
def yapi_ozeti(ctx: ext.CommandContext) -> ext.CommandResult:
    """"Yapı ara" panelinin Özetle düğmesi: yapıların sayısı, taban alanlarının toplamı, numarasızların sayısı ve en
    büyük yapı, her biri bir satır; panel her satırı bir gösterge kartı yapar ("ölçü" adı, "değer" sayısı, "birim" ve
    "not"). En büyük yapının satırı uid taşır: kartına tıklamak onu gösterir (docs/adr/8057 §24). Panelden çalışınca
    taban alanlarını çubuk grafik olarak da çizer (``image``: NumPy ile çizilir, PNG'ye çevrilir; docs/adr/8057 §40)."""
    secilen = ctx.params.get("katman")
    yapi = (secilen or _katman(ctx.doc, "yapi")) if ctx.doc is not None else None
    if yapi is None:
        return ctx.message("Çizimde 'yapi' kimlikli ya da adlı katman yok.")
    kayitlar = list(ctx.doc.entities(layer=yapi, kinds=["polygon"]))
    if not kayitlar:
        return ctx.report([], message="Katmanda yapı yok.")
    alanlar = [(Polygon(k.entity.pts).area, k) for k in kayitlar]
    numarasiz = sum(1 for _, k in alanlar if not (k.entity.attrs or {}).get("no"))
    en_alan, en = max(alanlar, key=lambda a: a[0])
    satirlar = [
        {"ölçü": "Yapı", "değer": len(kayitlar), "birim": "adet"},
        {"ölçü": "Toplam taban alanı", "değer": round(sum(a for a, _ in alanlar), 1), "birim": "m²"},
        {"ölçü": "Numarasız", "değer": numarasiz, "birim": "adet"},
        {"ölçü": "En büyük yapı", "değer": round(en_alan, 1), "birim": "m²", "uid": en.uid,
         "not": (en.entity.attrs or {}).get("no") or "numarasız"},
    ]
    if ctx.panel is not None:
        # Göstergelerin altında taban alanları, büyükten küçüğe çubuklar: kodun çizdiği görsel (panelin image'ı).
        ctx.panel.set("alanlar", image=_cubuklar(sorted((a for a, _ in alanlar), reverse=True)))
    return ctx.report(satirlar, title="Yapı özeti", message=f"{len(kayitlar)} yapının özeti.")


def _cubuklar(degerler: list[float]) -> bytes:
    """Değerler için çubuk grafik (480 × 140 piksel): en büyüğü tam boy, mavi çubuklar beyaz zeminde, gri taban çizgisi.
    NumPy dizisine çizilir ve PNG'ye çevrilir; çizim kitaplığı gerekmez (matplotlib'in ``savefig`` baytları da olur)."""
    genislik, yukseklik, pay = 480, 140, 12
    resim = np.full((yukseklik, genislik, 3), 255, dtype=np.uint8)
    if degerler:
        en = max(degerler) or 1.0
        cubuk = max(4, (genislik - pay * (len(degerler) + 1)) // len(degerler))
        for i, deger in enumerate(degerler):
            boy = max(2, int((yukseklik - 2 * pay) * deger / en))
            x = pay + i * (cubuk + pay)
            resim[yukseklik - pay - boy : yukseklik - pay, x : x + cubuk] = (59, 130, 246)
    resim[yukseklik - pay, pay : genislik - pay] = (140, 140, 140)
    return _png(resim)


def _png(rgb: np.ndarray) -> bytes:
    """Bir RGB dizisini (yükseklik × genişlik × 3, uint8) PNG baytlarına çevirir: zlib ve struct, ek kitaplıksız."""
    yukseklik, genislik, _ = rgb.shape
    ham = b"".join(b"\x00" + rgb[y].tobytes() for y in range(yukseklik))

    def parca(tur: bytes, veri: bytes) -> bytes:
        return struct.pack(">I", len(veri)) + tur + veri + struct.pack(">I", zlib.crc32(tur + veri) & 0xFFFFFFFF)

    baslik = struct.pack(">IIBBBBB", genislik, yukseklik, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + parca(b"IHDR", baslik) + parca(b"IDAT", zlib.compress(ham, 9)) + parca(b"IEND", b"")


@ext.command("katmanlari-kur")
def katmanlari_kur(ctx: ext.CommandContext) -> ext.CommandResult:
    """Vitrin'in katman şemasını (schema.json: Vitrin grubu, Örnek tesis ve Bakım kaydı katmanları, Tesis türü listesi,
    bakım kaydından tesise ilişki) açık çizime kurar: var olana dokunmaz, eksiği ekler; tek geri alma adımıdır."""
    return ctx.apply_declarative_schema("schema.json")


@ext.command("hata-ornegi")
def hata_ornegi(ctx: ext.CommandContext) -> ext.CommandResult:
    """Bilerek hata verir: KentOS hatayı, eklentinin günlüğünü ve Python iz kaydını (Ayrıntı) açıkça gösterir;
    çizimde bir şey değişmez."""
    raise ValueError("Bu bir örnek hatadır: eklenti hata verdiğinde KentOS onu böyle gösterir.")


def _etiket(feature: ext.Feature, n: int) -> str:
    """Bir parselin tablodaki adı: ada/parsel öznitelikleri varsa onlar, yoksa sırası."""
    attrs = {k.casefold(): v for k, v in feature.attrs.items() if v}
    ada, parsel = attrs.get("ada"), attrs.get("parsel")
    return f"{ada}/{parsel}" if ada and parsel else (parsel or str(n))


@ext.processing("taks")
def taks(ctx: ext.ProcessingContext, parseller: ext.FeatureInput, yapilar: ext.FeatureInput, taks_siniri: float) -> dict:
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
        a2, t2, r3 = round(float(a), 2), round(float(t), 2), round(float(r), 3)
        cikti.append(
            ext.NewFeature(
                source=f,
                attrs={"parsel_alani_m2": a2, "taban_alani_m2": t2, "taks": r3, "taks_siniri": limit,
                       "yapi_sayisi": int(s), "durum": durum},
            )
        )
        # Tablo okunmak için: başlıklar Türkçe ve birimli; katmanın öznitelik adları ifadelerde kullanılmak için.
        tablo.append({"parsel": _etiket(f, n), "parsel alanı (m²)": a2, "taban alanı (m²)": t2, "TAKS": r3,
                      "sınır": limit, "durum": durum})
    ctx.select(f for f, x in zip(parseller, asiyor) if x)
    asan = int(asiyor.sum())
    ctx.log(f"{len(parsel)} parsel, {len(yapi)} yapı; shapely {shapely.__version__}, NumPy {np.__version__}.")
    sinir = f"{limit:g}".replace(".", ",")  # Türkçe ondalık virgül
    ctx.summary(f"{len(parsel)} parselin TAKS'ı hesaplandı; {asan} parsel plan notundaki {sinir} sınırını aşıyor.")
    return {"parseller": cikti, "tablo": tablo, "asan": asan}


@ext.processing("cekme")
def cekme(ctx: ext.ProcessingContext, yapilar: ext.FeatureInput, parseller: ext.FeatureInput, mesafe: float) -> dict:
    """Çekme mesafesi denetimi: her yapının çevresine KentOS'un yerleşik Tampon aracıyla (geometry.buffer) mesafe
    kadar alan çizilir; alan yapının oturduğu parselin dışına taşıyorsa yapı parsel sınırına bu mesafeden yakındır.
    Tamponu eklenti çizmez, KentOS'un aracı eklentiye verilen yapılar üzerinde çalışır (ctx.run_tool), kullanıcının
    çizimine dokunmadan; karşılaştırma shapely ile. Yapıyı tamponuna sırası (öznitelik "sira") bağlar. Her yapının
    kendi "cekme" özniteliğine durumu yazılır (ctx.update; girdinin tanımı writes: true)."""
    uzaklik = float(mesafe if mesafe is not None else 3)
    ctx.progress(0, 3, "KentOS'un Tampon aracı çalışıyor")
    verilen = [{"type": "Feature", "geometry": f.geometry, "properties": {"sira": n}} for n, f in enumerate(yapilar)]
    cevap = ctx.run_tool("geometry.buffer", input=verilen, distance=uzaklik)
    tampon = {int(t.attrs.get("sira", -1)): t for t in cevap.features()}
    ctx.progress(1, 3, "Tamponlar parsellerle karşılaştırılıyor")
    parsel = np.array([shape(f.geometry) for f in ctx.read_features(parseller)], dtype=object)
    agac = shapely.STRtree(parsel) if len(parsel) else None
    cikti, tasanlar = [], []
    for n, yapi in enumerate(yapilar):
        t = tampon.get(n)
        if t is None:
            continue
        alan = shape(t.geometry)
        oturdugu = shape(yapi.geometry).representative_point()
        icinde = [int(i) for i in agac.query(oturdugu, predicate="within")] if agac is not None else []
        tasan = round(float(alan.difference(parsel[icinde[0]]).area), 2) if icinde else round(float(alan.area), 2)
        durum = "uygun" if tasan <= 1e-6 else "taşıyor"
        if durum == "taşıyor":
            tasanlar.append(yapi)
        # Yapının kendisine de yazılır: girdinin tanımı "writes": true der (KentOS kaynağı başka türlü değiştirmez).
        ctx.update(yapi, cekme=durum)
        etiket = yapi.attrs.get("no") or f"{n + 1}. yapı"
        cikti.append(ext.NewFeature(geometry=t.geometry, attrs={
            "yapi": etiket, "mesafe_m": uzaklik, "durum": durum, "tasan_alan_m2": tasan}))
    ctx.progress(2, 3, "Sonuç yazılıyor")
    ctx.select(tasanlar)
    m = f"{uzaklik:g}".replace(".", ",")
    ctx.summary(f"{len(cikti)} yapının {m} m çekme mesafesi denetlendi; {len(tasanlar)} yapı parsel sınırına daha yakın.")
    return {"tamponlar": cikti, "tasan": len(tasanlar)}


@ext.expr("ada-parsel")
def ada_parsel(ada, parsel) -> str | None:
    """Bir parselin "ada/parsel" yazımı (101/1); ikisinden biri boşsa boş. İfadelerde:
    ext.tr.ornek.vitrin.ada-parsel(ada, parsel)."""

    def metin(deger):
        if deger is None:
            return ""
        if isinstance(deger, float) and deger.is_integer():
            return str(int(deger))
        return str(deger).strip()

    a, p = metin(ada), metin(parsel)
    return f"{a}/{p}" if a and p else None


@ext.map_tool("yapi-yerlestir")
def yapi_yerlestir(ctx: ext.MapToolContext, sekil: ext.Shape) -> ext.CommandResult:
    """Kullanıcının gösterdiği yere 10 × 12 m bir yapı yazar (manifestteki şablon). Her yerleştirme tek geri alma
    adımıdır; araç Esc'e kadar sürer."""
    return ctx.add_feature("yapi", sekil, {"kaynak": "Vitrin", "kat": "2"})


@ext.audit("parsel-cakisma")
def parsel_cakisma(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """'parsel' katmanında birbirinin üstüne binen parseller (hata) ve aralarında boşluk kalanlar (uyarı). Geometriyi
    KentOS ölçer (facts.project: Rust'ın çakışma ve boşluk denetimi, projenin toleransıyla); kural yalnız karar verir
    ve bulguyu parselin ada/parsel adıyla yazar. Düzeltme önermez: hangi parselin sınırı yanlış, ölçen karar verir."""
    katman = facts.project.layer("parsel")
    adlar = {f.id: f"{f.attrs.get('ada', '?')}/{f.attrs.get('parsel', '?')}" for f in facts.features("parsel")}
    bulgular = []
    for c in katman.overlaps:
        bulgular.append(
            ext.AuditFinding(
                message=f"{adlar.get(c.a, 'Bir parsel')} ile {adlar.get(c.b, 'bir parsel')} {c.area:.2f} m² çakışıyor.",
                severity="error",
                feature_id=c.a,
            )
        )
    for b in katman.gaps:
        cevre = ", ".join(adlar.get(u, "?") for u in b.around[:3]) or "parseller"
        bulgular.append(
            ext.AuditFinding(
                message=f"{cevre} arasında {b.area:.2f} m² boşluk kalıyor.",
                severity="warning",
                feature_id=b.around[0] if b.around else None,
            )
        )
    return bulgular


@ext.audit("yapi-no")
def yapi_no(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """'yapi' katmanında "no"su olmayan her yapı bir uyarıdır; düzeltmesi sıradaki numarayı (Y-03 …) yazar. Kural
    çizimi okumaz, KentOS'un verdiği katmanlara bakar; düzeltmeyi kullanıcı uygular (tek geri alma adımı)."""
    yapilar = facts.features("yapi") or facts.features("Yapı")
    kullanilan = {f.attrs.get("no") for f in yapilar}
    sira = 1
    bulgular = []
    for yapi in yapilar:
        if yapi.attrs.get("no"):
            continue
        while f"Y-{sira:02d}" in kullanilan:
            sira += 1
        no = f"Y-{sira:02d}"
        kullanilan.add(no)
        bulgular.append(
            ext.AuditFinding(
                message=f"Yapının numarası yok; sıradaki numara {no}.",
                severity="warning",
                feature_id=yapi.id,
                fix=ext.SemanticFix(
                    title=f"{no} yaz",
                    command="cad.entities.set",
                    args={"uids": [yapi.id], "operation": "attributes", "attrs": {"no": no}},
                ),
            )
        )
    if facts.cut:
        bulgular.append(ext.AuditFinding(message="Çizimin yalnız ilk nesneleri denetlendi.", severity="info"))
    return bulgular


@ext.command("deger-say")
def deger_say(ctx: ext.CommandContext) -> ext.CommandResult:
    """"Değer say" panelinin düğmesi: seçilen katmanın seçilen alanındaki değerleri sayar; nesneler alındıysa yalnız
    onlarda, alınmadıysa katmanın bütün nesnelerinde (docs/adr/8057 §24). Panelin değerleri ctx.params'tadır: katman
    (kimliği), alan (adı), nesneler (çizimdeki seçimden alınanların kalıcı kimlikleri; boş liste: hiçbiri). Satırlar
    nesne başınadır; panelin ağacı onları değere göre gruplar ve her grubun sayısını yazar."""
    katman, alan = ctx.params.get("katman"), ctx.params.get("alan")
    if ctx.doc is None or not katman or not alan:
        return ctx.message("Bir katman ve onun bir alanını seçin.")
    secilen = ctx.params.get("nesneler") or []
    kayitlar = [ctx.doc.entity(uid) for uid in secilen] if secilen else list(ctx.doc.entities(layer=katman))
    satirlar = []
    for n, kayit in enumerate(kayitlar, start=1):
        oznitelikler = kayit.entity.attrs or {}
        deger = str(oznitelikler.get(alan) or "—")
        # Nesnenin adı: sayılan alan dışındaki öznitelikleri ("parsel 1"), yoksa sırası.
        ad = ", ".join(f"{k} {v}" for k, v in oznitelikler.items() if k != alan and v) or f"{n}. nesne"
        satirlar.append({"uid": kayit.uid, "Değer": deger, "Nesne": ad})
    sayilar: dict[str, int] = {}
    for satir in satirlar:
        sayilar[satir["Değer"]] = sayilar.get(satir["Değer"], 0) + 1
    # Çoktan aza; eşitlerde değerin sırasıyla (ağaç grupları ilk satırlarının sırasıyla dizer).
    satirlar.sort(key=lambda s: (-sayilar[s["Değer"]], s["Değer"]))
    nerede = f"seçilen {len(satirlar)} nesnede" if secilen else f"katmanın {len(satirlar)} nesnesinde"
    if ctx.panel is not None:
        # Panelden çalıştı: bulunan değerler Seç kutusunun seçenekleri olur, çoktan aza ve sayılarıyla (docs/adr/8057
        # §32). Seçenekler veriden gelir: panelin JSON'u bunları bilemez. Kodun koyduğu değer onChange'i çalıştırmaz.
        sirali = sorted(sayilar, key=lambda d: (-sayilar[d], d))
        secenekler = [ui.choice("-", "Seçmeyin")] + [ui.choice(d, f"{d} ({sayilar[d]})") for d in sirali]
        ctx.panel.set("vurgula", options=secenekler, value="-")
        if sirali:
            ctx.panel.set("en-sik", text=f"En sık değer: “{sirali[0]}” ({sayilar[sirali[0]]} nesne).")
        else:
            ctx.panel.set("en-sik", text="Sayılacak nesne yok.")
    return ctx.report(satirlar, title="Değer say", message=f"{nerede} {len(sayilar)} farklı “{alan}” değeri.")


@ext.command("katman-degerleri")
def katman_degerleri(ctx: ext.CommandContext) -> ext.CommandResult:
    """Katmanın sağ tık menüsünden çalışır: "Değer say" panelini o katman seçili açar; alan seçilince değerler
    sayılır. Menüden çalışan komutun ctx.params'ında tıklanan katmanın kimliği vardır (``layer``; şeritten çalışınca
    yoktur, docs/adr/8057 §32)."""
    katman = ctx.params.get("layer")
    if not katman:
        return ctx.message("Bir katmanın sağ tık menüsünden çalışır: Değerlerini say.")
    panel = ctx.open_panel("deger-paneli")
    panel.set("katman", value=katman)
    panel.say("Bir alan seçin: değerleri sayılır.")
    return ctx.message("Değer say katmanla açıldı.")


@ext.command("csv-noktalar")
def csv_noktalar(ctx: ext.CommandContext) -> ext.CommandResult:
    """Katmanın sağ tık menüsünden: seçilen CSV dosyasındaki noktaları o katmana ekler (x, y sütunları TM36 metre;
    diğer sütunlar istenirse öznitelik olur). Dosyayı KentOS'un penceresi sorar (dosya seçici: yolu yazılır ya da Seç…
    ile seçilir); eklenti yalnız kullanıcının verdiği dosyayı okur (files.userSelected). Pencerenin iki sekmesi var:
    Dosya ve Seçenekler (ayırıcı, öznitelikler, dosyanın koordinat sistemi); sekmesiz açıklama her sayfada. Dosyanın
    sistemi çizimin sistemiyle başlar; başkası seçilirse noktalar KentOS'un dönüşümüyle (cad.crs.transform) çizimin
    sistemine taşınır. Tek geri alma adımıdır."""
    katman = ctx.params.get("layer")
    if not katman or ctx.doc is None:
        return ctx.message("Bir katmanın sağ tık menüsünden çalışır: CSV'den nokta ekle.")
    cevap = ctx.dialog(
        "CSV'den nokta ekle",
        [
            ui.text("Her satır bir nokta: x ve y sütunları, dosyanın koordinat sisteminde."),
            ui.file_picker(
                "dosya", "CSV dosyası", filters=[ui.file_filter("CSV", "csv")], required=True, tab="Dosya"
            ),
            ui.select(
                "ayirici",
                "Ayırıcı",
                [ui.choice(",", "Virgül (,)"), ui.choice(";", "Noktalı virgül (;)"), ui.choice("sekme", "Sekme")],
                value=",",
                tab="Seçenekler",
            ),
            ui.checkbox("oznitelik", "Diğer sütunlar öznitelik olsun", value=True, tab="Seçenekler"),
            ui.crs_picker("crs", "Dosyanın koordinat sistemi", required=True, tab="Seçenekler"),
        ],
        ok="Ekle",
    )
    if cevap is None:
        return ctx.message("Vazgeçildi: nokta eklenmedi.")
    okunan, atlanan = [], 0
    secilen = cevap.get("ayirici") or ","
    ayirici = "\t" if secilen == "sekme" else secilen
    with open(cevap["dosya"], newline="", encoding="utf-8-sig") as dosya:
        for satir in csv.DictReader(dosya, delimiter=ayirici):
            try:
                x, y = float(satir.pop("x")), float(satir.pop("y"))
            except (KeyError, TypeError, ValueError):
                atlanan += 1
                continue
            oznitelikler = {k: v for k, v in satir.items() if k and v} if cevap.get("oznitelik") else {}
            okunan.append((x, y, oznitelikler))
    # Dosyanın sisteminden çizimin sistemine: KentOS'un dönüşümü (aynı sistemde noktalar yerinde kalır).
    kod = int(cevap["crs"].split(":")[1])
    donen = cad.crs.transform(
        ctx.doc, points=[cad.CrsPoint(x=x, y=y) for x, y, _ in okunan], from_=cad.CrsSide(srid=kod)
    ).points
    eklenen, varamayan = 0, 0
    for (_, _, oznitelikler), nokta in zip(okunan, donen):
        if nokta is None:
            varamayan += 1
            continue
        cad.point.create(ctx.doc, layer_id=katman, p=[nokta.x, nokta.y], attrs=oznitelikler)
        eklenen += 1
    ek = f"; {atlanan} satırın x ya da y'si okunamadı" if atlanan else ""
    ek += f"; {varamayan} nokta çizimin sistemine dönüştürülemedi" if varamayan else ""
    return ctx.message(f"{eklenen} nokta eklendi ({cevap['crs']}){ek}.")


@ext.command("katman-csv")
def katman_csv(ctx: ext.CommandContext) -> ext.CommandResult:
    """Katmanın sağ tık menüsünden: katmanın nesnelerini seçilen yere CSV olarak yazar (x, y: nesnenin merkezi, TM36
    metre; öznitelikler sütun olur). Yeri KentOS'un penceresi sorar (kaydetme yeri: filePicker save; yolu yazılır ya da
    Seç… ile seçilir, uzantısız ada .csv eklenir); eklenti yalnız kullanıcının seçtiği yere yazar
    (files.userSelectedWrite). Çizime dokunmaz."""
    katman = ctx.params.get("layer")
    if not katman or ctx.doc is None:
        return ctx.message("Bir katmanın sağ tık menüsünden çalışır: Katmanı CSV'ye yaz.")
    cevap = ctx.dialog(
        "Katmanı CSV'ye yaz",
        [
            ui.text("Her nesne bir satır: x ve y nesnenin merkezi (TM36, metre); öznitelikler sütun olur."),
            ui.file_picker(
                "yer",
                "Kaydedilecek dosya",
                filters=[ui.file_filter("CSV", "csv")],
                save=True,
                default_name=f"{katman}.csv",
                required=True,
            ),
        ],
        ok="Yaz",
    )
    if cevap is None:
        return ctx.message("Vazgeçildi: dosya yazılmadı.")
    satirlar = [
        (merkez, k.entity.attrs or {}) for k in ctx.doc.entities(layer=katman) if (merkez := _merkez(k.entity))
    ]
    sutunlar = sorted({ad for _, oznitelikler in satirlar for ad in oznitelikler})
    with open(cevap["yer"], "w", newline="", encoding="utf-8") as dosya:
        yazici = csv.writer(dosya)
        yazici.writerow(["x", "y", *sutunlar])
        for (x, y), oznitelikler in satirlar:
            yazici.writerow([f"{x:.3f}", f"{y:.3f}", *(oznitelikler.get(ad, "") for ad in sutunlar)])
    return ctx.message(f"{len(satirlar)} nesne yazıldı: {cevap['yer']}")


def _merkez(nesne) -> tuple[float, float] | None:
    """Nesnenin merkezi: noktanın kendisi, alanın ağırlık merkezi, çizginin ilk noktası; şekli olmayanda yok."""
    if (p := getattr(nesne, "p", None)) is not None:
        return float(p.x), float(p.y)
    noktalar = getattr(nesne, "pts", None) or []
    if len(noktalar) > 2 and getattr(nesne, "kind", "") == "polygon":
        merkez = Polygon(noktalar).centroid
        return merkez.x, merkez.y
    return (float(noktalar[0][0]), float(noktalar[0][1])) if noktalar else None


@ext.command("yapi-tamponu")
def yapi_tamponu(ctx: ext.CommandContext) -> ext.CommandResult:
    """Vitrin sekmesinden: yapıların çevresine KentOS'un yerleşik Tampon aracıyla (geometry.buffer) sorulan uzaklıkta
    alan çizer, yeni bir katmana. Eklenti aracı kendisi yazmaz, KentOS'unkini çağırır (ctx.run_tool): araç bu komutun
    hakkıyla çalışır, yazdıkları komutun tek geri alma adımına girer."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    cevap = ctx.dialog(
        "Yapı tamponu",
        [
            ui.text("KentOS'un Tampon aracı yapıların çevresine bu uzaklıkta alan çizer; sonuç yeni katmanda."),
            ui.number_field("uzaklik", "Uzaklık", value=3, min=0.5, max=50, unit="m", required=True),
        ],
        ok="Çiz",
    )
    if cevap is None:
        return ctx.message("Vazgeçildi: tampon çizilmedi.")
    uzaklik = float(cevap["uzaklik"])
    sonuc = ctx.run_tool(
        "geometry.buffer",
        input=ext.layer_input("yapi"),
        distance=uzaklik,
        layer=ext.new_layer(f"Yapı tamponu {uzaklik:g} m"),
    )
    return ctx.message(f"{sonuc.summary} (KentOS'un Tampon aracı, {uzaklik:g} m).")


@ext.command("deger-sec")
def deger_sec(ctx: ext.CommandContext) -> ext.CommandResult:
    """"Değer say"ın Seç kutusu değişince çalışır: seçilen değerdeki nesneleri (alındıysa yalnız alınanlardan) çizimde
    seçer. Değer, sayımın Seç'e koyduğu seçeneklerden biridir; "-" hiçbirini seçmez."""
    katman, alan, deger = ctx.params.get("katman"), ctx.params.get("alan"), ctx.params.get("vurgula")
    if ctx.doc is None or not katman or not alan or deger in (None, "", "-"):
        return ctx.message("Seçim değişmedi.")
    secilen = ctx.params.get("nesneler") or []
    kayitlar = [ctx.doc.entity(uid) for uid in secilen] if secilen else list(ctx.doc.entities(layer=katman))
    uyanlar = [k.uid for k in kayitlar if str((k.entity.attrs or {}).get(alan) or "—") == deger]
    ctx.doc.select(uyanlar)
    return ctx.message(f"“{alan}” = “{deger}”: {len(uyanlar)} nesne seçildi.")


# Planlı Alanlar İmar Yönetmeliği'nin taban alanı katsayısı için örnek plan notu (TAKS aracının varsayılanı).
TAKS_SINIRI = 0.4


@ext.command("parsel-karti")
def parsel_karti(ctx: ext.CommandContext) -> ext.CommandResult:
    """"Parsel kartı" panelinin nesne seçicisi değişince çalışır: alınan her parsel bir satır, panel her satırı bir
    kart olarak gösterir (docs/adr/8057 §24): "parsel" sütunu kartın başlığı, "durum" yanında, diğerleri altında
    adlarıyla; satırın uid'si kartı tıklanınca parseli gösterir. Parsel katmanında olmayan nesneler atlanır."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    secilen = ctx.params.get("parseller") or []
    # Panelin tarih alanı ISO tarih verir (2026-11-15) ya da boşsa None: kartta Türkçe yazılır.
    gun = date.fromisoformat(ctx.params["tarih"]) if ctx.params.get("tarih") else date.today()
    # Saat alanı SS:DD verir (14:30); an alanı yerel ISO tarih ve saat (2026-11-20T17:00), boşsa None.
    zaman = gun.strftime("%d.%m.%Y") + (f" {ctx.params['saat']}" if ctx.params.get("saat") else "")
    gecerli = None
    if ctx.params.get("gecerli"):
        gecerli = datetime.fromisoformat(ctx.params["gecerli"]).strftime("%d.%m.%Y %H:%M") + "'e dek"
    parsel_katmani, yapi_katmani = _katman(ctx.doc, "parsel"), _katman(ctx.doc, "yapi")
    yapilar = []
    if yapi_katmani:
        yapilar = [Polygon(r.entity.pts) for r in ctx.doc.entities(layer=yapi_katmani, kinds=["polygon"])]
    satirlar, atlanan = [], 0
    for uid in secilen:
        kayit = ctx.doc.entity(uid)
        if kayit is None or kayit.entity.kind != "polygon" or kayit.entity.layer_id != parsel_katmani:
            atlanan += 1
            continue
        parsel = Polygon(kayit.entity.pts)
        # Bir yapının parselden taşan kısmı o parsele sayılmaz (TAKS aracı gibi).
        tabanlar = [a for a in (parsel.intersection(y).area for y in yapilar) if a > 1e-6]
        oran = sum(tabanlar) / parsel.area if parsel.area > 0 else 0.0
        oznitelikler = {k.casefold(): v for k, v in (kayit.entity.attrs or {}).items() if v}
        ada, no = oznitelikler.get("ada"), oznitelikler.get("parsel")
        bilgi = {
            "alan": ("alan (m²)", round(parsel.area, 1)),
            "yapi": ("yapı", len(tabanlar)),
            "taban": ("taban alanı (m²)", round(sum(tabanlar), 1)),
            "taks": ("TAKS", round(oran, 2)),
            "tarih": ("tarih", zaman),
        }
        satir = {
            "uid": kayit.uid,
            "parsel": f"{ada}/{no}" if ada and no else f"{len(satirlar) + 1}. parsel",
            "durum": "TAKS aşıyor" if oran > TAKS_SINIRI + 1e-9 else "uygun",
        }
        # Panelin çoklu seçimi seçilenlerin listesini verir (seçenek sırasıyla); yoksa hepsi.
        for anahtar in ctx.params.get("bilgiler", list(bilgi)):
            if anahtar in bilgi:
                ad, deger = bilgi[anahtar]
                satir[ad] = deger
        if gecerli:
            satir["geçerli"] = gecerli
        satirlar.append(satir)
    if not secilen:
        mesaj = "Parselleri çizimde seçip Seçimi al'a basın."
    else:
        mesaj = f"{len(satirlar)} parselin kartı."
        if atlanan:
            mesaj += f" {atlanan} nesne parsel katmanında değil, atlandı."
    # Satırsız da tablo verir: alınanlar temizlenince kartlar da gider (ileti verilseydi eski kartlar kalırdı).
    return ctx.report(satirlar, title="Parsel kartı", message=mesaj)
