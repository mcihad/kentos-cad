# 5120 — Kesit kontrolü; m. 63(2) ızgarasının taban noktası ve dönmesi; katman damgası dizinden

Tarih: 2026-11-11. plan/map-cad.md MAP-14, onuncu tur. Durum: taslak. ADR 4740'ın “Sonra”sındaki üç iş: kesit
kontrolünün kendisi; m. 63(2) ızgarasının taban noktası ve dönmesi; katman damgasının katman dizininden okunması.

İlgili: [ADR 4740](4740-kept-sheet-contours-and-built-up-heights.md) (pafta eğrileri kayıtlı çizim, m. 63(2) kotları,
m. 93(1)(c) karşılaştırması), [ADR 1820](1820-sheet-contours-by-article-78.md) (pafta eğrileri).

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460 mükerrer), metinden okundu (`.run/research/map-cad/mevzuat/pdf/bohhbuy/bohhbuy-pdf.txt`,
  11 Kasım 2026):
  - m. 93(1): “Kesit kontrolü, yersel ve fotogrametri yöntemi ile yapılan paftalarda yüksekliği bilinen iki kontrol
    noktası arasında … kesitler alınarak yapılır.” (a) uygun dağılımda ve değişik eğimli yerlerde; (b) kesit doğrultusu
    üzerinde en fazla 20 m aralıklarla noktalar, geometrik ya da trigonometrik nivelmanla yükseklik; (c) ölçülen ve eş
    yükseklik eğrilerinden hesaplanan yüksekliklerin farklarının karesel ortalama hatası m. 91'e göre KOH_Z'nin 1,5
    katından küçük.
  - m. 63(2): “Yerleşik alanlar ve yollarda harita üzerinde yaklaşık 2 cm’de bir … yükseklik değerleri verilir.”
    Izgaranın taban noktası ya da yönü için hüküm yok.
- **Civil 3D 2023**, “To Create a Grid of Points at Surface Elevations” (resim
  `.run/research/map-cad/uygulama/MAP-14.10-resim/civil3d-2023-grid-of-points.jpg`; not `MAP-14.10.md`): ızgaranın taban
  noktası (sol alt köşe), hücre boyu, sağ üst köşe ve isteğe bağlı dönme açısı; kotlar yüzeyden. Kesit kontrolünün
  kendisi için benzer bir yazılım belgesi bulunmadı; yöntem maddenin kendisi ve eğrilerden kot okumanın klasik yolu
  (kesitin kestiği iki komşu eğri arasında doğrusal enterpolasyon).

## Karar

1. **`road.terrains.sectioncheck` v1, Kesit kontrolü** (yeni, salt okunur). Kesit, kendi kotu olan iki kontrol
   noktası arasındaki doğrudur (`from`, `to`); kesit noktaları (`points`) kendi kotu olan noktalardır.
   - Nokta kesite izdüşümüyle okunur: başlangıçtan uzaklık ve kesitten uzaklık. Ayağı uçların dışındaysa `outside`;
     `corridor` verilirse ondan uzaksa `offLine` (sınır yoksa yalnız uzaklık söylenir; yönetmelik sınır vermez).
   - Eğri katmanlarındaki (`contours`) her köşesi (her parçasının) aynı kotta olan çoklu çizgiler eş yükselti
     eğrisidir; öbürleri sayılır (`unlevelled`), okunmaz. Kesitle kesişimleri bulunur (kapalının son kenarı da; yay
     kirişiyle); aynı kottaki iki kesişim 1e-9 m içindeyse birdir (eğrinin köşesinden geçen kesit).
   - Noktanın eğrilerden kotu: önündeki (ya da üstündeki) son ve ardındaki ilk kesişimin kotları arasında doğrusal
     enterpolasyon; bir yanında kesişim yoksa `noContour`.
   - Ölçülen − eğrilerden farkların KOH'u ve en büyüğü; sınır paketin `sectionCheck`'inden 1,5 · KOH_Z (KOH_Z m.
     91(2)'nin ölçeğe göre). `within` KOH < sınır (“küçük olmalıdır”).
   - Kesitin yerleri (iki uç ve kesitteki noktalar) paketin `spacing`'inden (m. 93(1)(b): 20 m) uzaksa `gaps`.
   - Uyarılar `section_check_exceeded` (eski kod) ve yeni `section_spacing_exceeded`. Yeni ret `not_a_height_point`
     (nokta değil ya da kendi kotu yok). Uygunluk ya da kurum kabulü söylenmez.
2. **Kural paketi** `bohhbuy-2018-m78` sürüm 5: `sectionCheck.spacing` 20 m (m. 93(1)(b)); kaynak metnine 93(1)(b).
   Pafta eğrilerinin cevabındaki paket sürümü buna göre değişti.
3. **m. 63(2) ızgarası:** `builtUp.origin` (taban noktası; yoksa koordinat başlangıcı) ve `builtUp.rotation`
   (radyan, doğudan saat yönü tersine; yoksa 0). Izgara noktaları taban noktası artı dönen doğu ve kuzey eksenlerinde
   aralığın tam katları; yerleşik alanın köşeleri dönen eksenlerde okunur, satır satır. Verilmeyince sonuç eskisinin
   aynısıdır (eski durumlar değişmedi). Sonlu olmayan değer `invalid_contours`. İkisi ölçmecinindir.
4. **Katman damgası** (ADR 4740 §1) katmanın nesnelerini katman dizininden okur (`Document::by_layer`, çizimin
   sırasıyla); bütün çizim gezilmez. Damga aynıdır.
5. **Masaüstü.**
   - Pafta eğrileri'ne **Izgara (63/2)** satırı: Başlangıç Y, X ve Açı (projenin açı birimi ve yönü; projenin dönüş
     sözleri gibi).
   - Pafta eğrileri'nin yanında **Kesit kontrolü…**: seçili, kendi kotu olan noktaların en uzak ikisi uçlar, öbürleri
     kesit noktaları; eğri katmanı Pafta eğrileri'nin katmanı ya da adından; Kesitten uzaklık (m). Özet KOH'u sınırla,
     20 m aralığı, okunmayan çizgileri söyler; tablo noktaları kesit boyunca: uzaklık, kesitten uzaklık, ölçülen,
     eğrilerden, fark, arasındaki eğriler, durum.
6. **Kapsam işareti:** **Harita + Yol** (ADR 4740 gibi); kesit kontrolü yüzey istemez, eğriler çizimde olması yeter.

## Sınırlar

- Nivelmanla ölçüldüğü KentOS'ça doğrulanmaz: noktanın kendi kotu ölçülmüş sayılır; m. 93(1)(a)'nın “uygun dağılım
  ve değişik eğimli yerler” şartı kullanıcınındır.
- Eğrinin yayı kirişiyle okunur; tepe ya da çukur gibi aynı kottaki iki eğri arasında kot o kottur.
- Karolu hesap (gerçek veri) ölçülmedi.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/section_check_cases.py`; KentOS kodu yok; kesişim Cramer kuralıyla):
  `road.terrains.sectioncheck`, 35 durum (21 sonuç, 13 ret, plan, doğrulama): zikzak eğrinin köşesi, kapalı halka,
  iki parçalı eğri, kotu karışık çizgi, çapraz ve ters kesit, kesit dışı ve uzak nokta, ölçek, aralıksız ve 25 m'lik
  paket. Bozma denemesi: bir eğrilerden kot ve bir aralık değiştirilince koşucu tam o ikisini yakalar.
- **Pafta eğrileri** (`scripts/fixtures/road_sheet_contour_cases.py`): 74 durum (3 yeni: 45° döndürülmüş ve kaydırılmış
  ızgara, yalnız kaydırılmış, yalnız döndürülmüş); betik noktaların dönen eksenlerde aralığın katı olduğunu da
  denetler. Yol projesi durumları (katman damgası) geçer.
- **Python** (`python/tests/test_section_check.py`): testin kendi aritmetiğiyle 101,5 ve 102 + 7/15, KOH, KOH_Z
  formülü ve iki aralık; kesit dışı nokta. Python ortak durumları (sectioncheck, sheetcontours) geçer.
- **MCP** (`crates/native/mcp/tests/section_check.rs`).
- **Masaüstü** (`section_check_window_tests`, 4 test; pafta eğrileri testleri): dönen ızgara ortak durumun kotları;
  seçimden kesit ortak durumun cevabı; sınırla uzak nokta; az nokta. Çapraz denetimler 161 test. Resimler
  `.run/gui-evidence/kesit-kontrolu-5120/`. Gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Sonra

- Karolu hesap (gerçek veri).
- Kesitin çizimde gösterilmesi (kesit doğrusu, noktaların farkı); kesitlerin projede saklanması.
- Web (TODOS §0.2).
