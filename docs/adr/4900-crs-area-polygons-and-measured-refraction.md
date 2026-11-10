# 4900 — EPSG'nin kullanım alanı çokgenleri; ölçülen k'nın projeye önerilmesi

Tarih: 2026-11-10. plan/map-cad.md MAP-02, onuncu tur. Durum: taslak. MAP-02'nin sıradaki işi: EPSG'nin çokgen kullanım
alanları; ölçülen k'nın projeye önerilmesi (ADR 4500'ün “Sonra”sı).

İlgili:
- [ADR 2500](2500-reproject-preview-shifts-and-reasons.md) §3, [ADR 3700](3700-coordinate-transform-point-by-point-areas.md): kayıttaki sistemlerin kullanım
  alanı (PROJ'un veritabanından kutu), noktanın notları (`outside_*`, kenardan 10⁻⁹° içi kenarda);
- [ADR 0169](0169-field-data.md) §3: projenin k'sı (öntanımlı 0,13); [ADR 4500](4500-traverse-reciprocal-zenith-angles.md):
  karşılıklı başucu açılarından çiftin k'sı.

## Dayanak

- **Mevzuat** (`.run/research/map-cad/mevzuat/MAP-02.10.md`, 10 Kasım 2026'da okundu):
  - BÖHHBÜY m. 10: TUREF, GRS80, üç derecelik dilimli Transvers Mercator; m. 79(1)(g): dönüşüm parametreleri yalnız proje
    alanında geçerlidir. Bir sistemin kullanım alanı çokgeni Türk mevzuatında tanımlanmamıştır; EPSG'ninki veri setidir,
    hukuki niteliği yoktur.
  - Kırılma katsayısının ölçülüp projeye alınmasına dair kural yok; Ek-7 indirgemeleri k içermez.
- **Kaynaklar ve benzer uygulamalar** (`.run/research/map-cad/uygulama/MAP-02.10.md`; ekran resimleri `MAP-02.10-resim/`,
  10 Kasım 2026'da tarayıcıda bakıldı):
  - **EPSG kayıt sitesi** (Extent 1525, EPSG Dataset v13.106): TUREF / TM30'un kullanım alanı çokgeni karadır (Marmara
    Denizi ve Antalya Körfezi dışarıda), kutusu bütün şerittir. Çokgenler `apps.epsg.org`'un `Extent/<kod>/polygon`'undan
    GeoJSON olarak alınır. PROJ'un veritabanı yalnız kutu tutar (IOGP GN 7-1 §5.7: kutu yaklaşıktır, kayıt seçmek içindir).
  - **EPSG kullanım koşulları** (8 Nisan 2016): veri kopyalanıp dağıtılabilir ve değer katan ticari pakete konabilir;
    IOGP'nin sahipliği belirtilir, koşullar iletilir, değiştirilmiş veri EPSG'ye atfedilmez.
  - **Netcad, Projeksiyon Parametreleri:** EPSG kodu, dilim, merkezi meridyen, ölçek çarpanı, sağa ve yukarı değer; kullanım
    alanı gösterilmez. Kırılma katsayısı ayarı wiki'de belgelenmemiş.
  - Ölçme yazılımlarında k elle girilen iş ayarıdır (tipik 0,13 ve 0,142, 0,2); ölçülen k'nın projeye önerildiği bir akış
    belgelenmemiş.

## Karar

1. **Çokgen veri dosyası** (`fixtures/crs/v1/area-polygons.json`, `scripts/fixtures/crs_area_polygons.py`): kayıttaki
   sistemlerin EPSG alanlarından çokgeni kutusundan başka olanların halkaları, EPSG Dataset v13.106'dan değiştirilmeden
   (IOGP'nin; koşulların adresi dosyada): TUREF ve ED50 TM27–TM45 (1524–1530), TUREF coğrafi (1237), ED50 / UTM 36–38
   (1639–1641); 11 alan, 18 sistem, 205 KB. Çokgeni kutusu olanlar (WGS 84 / UTM şeritleri, dünya) ve ED50 / UTM 35N'nin
   alanı (1638: 25 000 köşe, 1 MB) taşınmaz; kutuları okunur, dosyada nedeniyle yazılı.
2. **Denetim** (`kentos_project::crs::AreaOfUse::holds_near`, `Region`): çokgeni olan sistemde nokta halkaların içinde
   (çift-tek kuralı) ya da bir kenarına 10⁻⁹° yakınsa alanın içindedir; yoksa kutu (ADR 3700). `cad.crs.transform`'un
   nokta notları, `cad.layers.reproject`'in önizlemesi ve masaüstünün Koordinat dönüştür'ü aynı kuralı kullanır.
   Dönüşümün (operasyonun) alanı kutu kalır. Çokgenin içinde olmak doğruluk kanıtı değildir.
3. **Ölçülen k** (`cad.survey.points`'in çıktısında `refraction`, Poligon hesabı): karşılıklı çiftlerin k'ları birlikte:
   ortalama k̄ = Σkᵢ/n, çift sayısı; iki ve daha çok çiftte s = √(Σ(kᵢ − k̄)²/(n − 1)) ve ortalamanın s/√n'si. Her çift
   aynı ağırlıktadır (başka ağırlığın kaynağı yok).
4. **Projeye yazma** (`cad.settings.set`'in `refraction` anahtarı): k −1 ile 1 arasında (`invalid_refraction`); öntanımlı
   0,13 yazılmaz (ayarların kuralı). Ayar geri alma adımı değildir (bütün ayarlar gibi).
5. **Masaüstü** (Poligon hesabı): özetin altında “Ölçülen k = … (n çift, s, ortalamanın σ'sı); projenin k'sı …” ve
   **Projeye yaz**; yazılınca tek yönlü kenarlar yeni k ile hesaplanır, durum çubuğu söyler.
6. **Kapsam işareti:** CAD + CBS (ADR 4100, 4500 gibi).

## Sınırlar

- Operasyonların (datum dönüşümlerinin) alanı kutudur; çokgeni ileride.
- Çokgenler EPSG v13.106'dan, kutular PROJ'un v12.029'undan: çokgeni olan sistemde kutu kullanılmaz.
- Ölçülen k için en az çift sayısı ya da yayılım sınırı yoktur (kaynak yok); öneri kullanıcının onayıyla yazılır.
- Ağın k'sı (bütün karşılıklı gözlemlerden dengelemeyle) yok.

## Kanıt

- **Çekirdek** (`kentos-project` `crs` testleri): TM30'un çokgeni Marmara'yı dışarıda bırakır, kutusu içeride sayar;
  Kütahya ovası içeride; UTM 36N'nin çokgeni yok.
- **Bağımsız başvurular** (KentOS kodu yok; çokgen denetimi `scripts/fixtures/crs_area_test.py`'de ayrı yazıldı):
  `crs_vertical_cases.py` 32'den 33 duruma (Marmara noktası kaynak alanının dışında), `layer_crs_cases.py` değişmedi
  (eski durumların hiçbiri çokgenle değişmedi); `survey_points_cases.py` iki karşılıklı durumda `refraction` (ortalama,
  çift sayısı, s, σ); `settings_set_command_cases.py` 2 yeni durum (122): yazma ve öntanımlıya dönme, aralık dışı ret.
  Bozma denemesi: Marmara notu silinince ve ortalama 10⁻⁴ oynatılınca koşucu tam o iki durumu yakalar.
- **Python** (`python/tests/test_crs_area_polygons_measured_k.py`, testin kendi çift-tek kuralı ve ortalamasıyla, bağımsız
  okuyucuyla), **MCP** (`crates/native/mcp/tests/crs_area_polygons_measured_k.rs`), **masaüstü**
  (`calc::traverse_reciprocal_tests`: ölçülen k çiftin k'sı, Projeye yaz projenin k'sını yapar, tek yönlü kenar testin
  kendi (1 − k)·S²/2R'siyle). Resimler `.run/gui-evidence/olculen-k-4900/`.

## Sonra

- Datum dönüşümlerinin alanlarının çokgeni; koordinat sistemi penceresinde küçük haritada çokgen.
- Atlamalı (ortadan) trigonometrik nivelman; ağın k'sı.
- Web (TODOS §0.2).
