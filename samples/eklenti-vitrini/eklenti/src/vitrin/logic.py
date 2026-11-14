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
- ``katmanlari_kur``: katman şeması; schema.json'daki grubu, katmanları, değer listesini ve ilişkiyi tek adımda kurar.
"""

import time

import numpy as np
import shapely
from shapely.geometry import Polygon, shape

from kentos import cad, ext


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
    """'yapi' katmanındaki yapılara sırayla "no" yazar (Y-01, Y-02 …). İlerleme çubuğunu ve Durdur'u göstermek için
    her yapıda yarım saniye bekler: Durdur'a basılırsa yazdıkları geri alınır."""
    yapi = _katman(ctx.doc, "yapi") if ctx.doc is not None else None
    if yapi is None:
        return ctx.message("Çizimde 'yapi' kimlikli ya da adlı katman yok.")
    kayitlar = list(ctx.doc.entities(layer=yapi, kinds=["polygon"]))
    for n, kayit in enumerate(kayitlar, start=1):
        ctx.progress(n - 1, len(kayitlar), f"{n}. yapı numaralanıyor")
        cad.entities.set(ctx.doc, uids=[kayit.uid], operation="attributes", attrs={"no": f"Y-{n:02d}"})
        time.sleep(0.5)
    return ctx.message(f"{len(kayitlar)} yapı numaralandı (Y-01 … Y-{len(kayitlar):02d}).")


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
