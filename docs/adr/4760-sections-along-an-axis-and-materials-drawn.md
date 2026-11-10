# 4760 — Aks boyunca kesit; kesitlerin ve malzeme bölgelerinin çizime yazılması

Tarih: 2026-11-10. plan/map-cad.md MAP-15, dokuzuncu tur. Durum: taslak. MAP-15'in sıradaki işi: malzeme bölgelerinin
çizime yazılması; kesitlerin çizime yazılması ve aks boyunca kesit.

İlgili:
- [ADR 4340](4340-surface-comparison-by-sections.md): iki yüzeyin farkı paralel kesitlerle (semtle);
- [ADR 3540](3540-material-zones-in-a-surface-comparison.md): malzeme bölgeleri (`zones`);
- [ADR 1840](1840-cut-and-fill-zones.md), [2340](2340-cut-fill-zone-labels-and-quantity-table.md),
  [2740](2740-cut-fill-zone-deepest-places.md): kazı ve dolgu bölgelerinin çizimi, yazıları, tablosu, en derin yerleri;
- [ADR 4740](4740-kept-sheet-contours-and-built-up-heights.md): kayıtlı çizimin `layer` kaynağı.

## Dayanak

- **Mevzuat** (alt ajanın notu `.run/research/map-cad/mevzuat/MAP-15.9.md`, 10 Kasım 2026'da okundu):
  - **KGM 2019 Karayolları Kesin ve Ön Projeleri Mühendislik Hizmetleri Teknik Şartnamesi:** enkesit eksene dik ya da
    istenen doğrultuda, ölçek 1/100 ya da 1/200, aralık en çok 10 m ve kırık noktalarda, dere tabanında, tepe üstünde
    ek kesit; arazi genellikle en az 50 m genişlikte; kesitin kilometresi, tabii zemin kotları ve eksene uzaklıkları,
    yarma ve dolgu alanları kesitin altında tablo olarak ya da üstünde; kübaj kesit alanları ve aralıklarıyla, idarenin
    kübaj cetvelleriyle; kabarma ve sıkışma katsayısı projenin araştırma raporundan. Hacim formülü adıyla geçmez.
  - **ÇŞB 2026 birim fiyat pozları:** kazı sınıfları (yumuşak toprak … çok sert kaya, batak); malzeme adı kullanıcının
    verisidir. **BÖHHBÜY Ek-2 UVDF §6.4** enkesitin veri biçimi (KesitNo, Km; Mesafe, Z, kod).
- **Benzer uygulamalar** (alt ajanın notu ve ana oturumun eki `.run/research/map-cad/uygulama/MAP-15.9.md`; ekran resimleri
  `MAP-15.9-resim/`, 10 Kasım 2026'da tarayıcıda bakıldı):
  - **Netcad NETSURF, Enkesit Çiz penceresi:** Enkesit Aralığı, Sola Genişlik, Sağa Genişlik, Ekstra Noktalar'da
    Eksen Çizgisi Noktalarında, Güzergah Başlangıç Km, Yüzeyler tablosu.
  - **Netcad, Enkesitlerden Hacim Hesabı:** yöntem KGM, (A1+A2+√(A1·A2))·L/3, (A1+A2)·L/2, DSİ; çizime yazılan cetvel
    Kilometre | Ara Uzaklık | Alan (Yarma, Dolma) | Hacim | Kümülatif Hacim | Brükner Değeri, kilometre 0+000.00
    biçiminde.
  - **Civil 3D 2024:** örnek çizgiler aks boyunca, sol ve sağ şerit genişliği, yatay geometri noktalarında kesit; kesit
    görünüşü ve malzeme listesi tabloları.
  - **OpenRoads:** End Area Volumes Report (istasyon başına alan, hacim, kütle ordinatı).

## Karar

1. **Aks boyunca kesit** (`road.terrains.compare`, kesit yöntemi; `kentos_road::volume_sections::along_axis`, ortak
   çekirdek): `axis` (çizgi ya da tek parçalı açık çoklu çizgi, yaylar dahil), `left`, `right` (m), `corners`.
   - Kesitler aksın başından her `interval`'da, son olarak aksın sonunda; `corners` ile iç köşelerinde de (Netcad'in
     Eksen Çizgisi Noktalarında'sı, Civil 3D'nin yatay geometri noktaları).
   - Kesit aksa diktir: kurpta yarıçap doğrultusunda, köşede iki parçanın gidişinin açıortayına dik, uçlarda
     parçasınınkine. Aksın solunda `left`, sağında `right` metre; iki yüzeyin örttüğü kısım sayılır.
   - İki kesit arasının boyu kilometrelerinin farkıdır (aks boyunca). Eğrilik için ağırlık merkezi düzeltmesi yapılmaz
     (kaynaklarda yok).
   - Cevapta `axis`, `length`; her kesitin `at`'ı ve uçları (`ends`). Semt ile eksen birlikte verilmez.
2. **Kazı ve dolgu çizimine kesitler** (`road.terrains.cutfill`'in `sections`'ı: `axis`, `interval`, `left`, `right`,
   `corners`, `formula`, `start`):
   - her kesit bir çizgi; solda kilometresi (`start` + istasyon, `k+mmm.cc`, cm'ye yarım yukarı), sağda alanları
     (“Kazı 6.28 m² · Dolgu 22.34 m²”), kesit boyunca, okunur yönde, uçların yarım yazı boyu dışında, maskeli;
   - `table` ile kübaj cetveli (Netcad'in sütunlarıyla): Km | Ara uzaklık | Kazı alanı | Dolgu alanı | Kazı hacmi |
     Dolgu hacmi | Kümülatif kazı | Kümülatif dolgu.
3. **Malzeme bölgeleri** (`road.terrains.cutfill`'in `materials`'ı, ADR 3540'ın bölgeleri): her bölgenin içine adı ve
   kazı, dolgu hacmi; `table` ile malzeme tablosu (Malzeme | Alan | Kazı | Dolgu | Kazı katsayısı | Dolgu katsayısı |
   Katsayılı net). Katsayı kullanıcınındır (KGM: projenin araştırma raporundan).
4. **Kayıtlı çizim:** kazı ve dolgu çizimi aksın ve malzeme sınırlarının katmanlarını da `layer` kaynağı olarak tutar
   (ADR 4740): aks ya da sınır değişince eskir.
5. **Ortak yardımcılar:** malzeme bölgelerinin denetimi, okunması ve ölçülmesi, eksenin okunması
   `road_terrains_compare`'den paylaşılır; kazı ve dolgu çiziminin yolları `materials[k]…`, `sections.…`.
6. **Masaüstü** (Yüzey analizi › Fark › Kesit): Eksen (Semtle ya da çizimdeki bir çizgi; seçili tek çizgi açılışta
   eksen olur), Sola ve Sağa genişlik (boş 25 m: KGM'nin “genellikle en az 50 m”i), Eksen noktalarında da, Başlangıç km.
   Kazı ve dolgu bölgelerini çiz kesitleri yazar; malzeme bölgeleri yalnız “Malzemeler (ad, hacim)” işaretliyken (TIN
   yönteminde, Hacim'in malzeme katmanından; adı adından tahmin edilen katman kendiliğinden yazılmaz).
7. **Tabloların yeri:** bölge tablosu, kübaj cetveli ve malzeme tablosu bir sütunda, bölgelerin ve kesitlerin (en uzun
   yazılarına yer bırakılarak) sağında, alt alta; kesit yazılarının üstüne binmez.

## Sınırlar

- Kesit sayfaları (kesitin kendi çizimi: iki yüzeyin çizgisi, kutu, veri satırları) yok; plandaki kesit çizgisi ve
  cetvel var.
- KGM ve DSİ yöntemlerinin formülü açık kaynakta yok; kesik koni formülü ve Brükner değeri sonraki iş.
- Netcad'in Üçgen Modelin Kesildiği Noktalarda ve Ekseni Kesen Çizgiler ek kesitleri yok.
- KentOS'un hacmi “KGM şartnamesine uygun” sayılmaz (idarenin kübaj cetveli ve onayı gerekir).

## Kanıt

- **Çekirdek** (`volume_sections` testleri): düz aks paralel kesitlerin sonucu; yarım halkada kesitler yarıçap
  doğrultusunda, hacim 400π'ye yakın; köşede açıortay; retler.
- **Bağımsız başvuru:** `road_terrain_compare_section_cases.py` (kafeslerin kesin doğrusal farkı; KentOS kodu yok) 28
  durumdan 43'e: düz, yaylı ve köşeli eksen, prizmoidal, tek yana genişlik, katsayılar, 9 ret.
  `road_terrain_cutfill_cases.py` 44'ten 57'ye: TM koordinatlarında yaylı eksen boyunca km 1+000'dan kesitler ve kübaj
  cetveli, eksen noktaları ve prizmoidal, düz eksen, malzeme bölgeleri ve tablosu, plan, 8 ret.
- Rust ve Python koşucuları ve yetenek testi geçer (yeni ret `entity_not_found`, yeni uyarı `compare_uncovered` kazı ve
  dolgu çiziminde). Bozma denemesi: yaylı eksenin uzunluğu 10⁻⁵ ve bir kesitin km'si oynatılınca koşucu tam o iki durumu
  (101'den 2) yakalar.
- **Python** (`python/tests/test_axis_sections.py`, testin kendi okumalarıyla: kurpta kesitin uçları ve merkez bir
  doğruda, uçlar merkezden 1,5 m yakın ve uzak; hacim testin kendi ortalama alanı; km'ler ve cetvelin kümülatifleri
  testin kendi hesabı; malzeme yazısı adı ve hacmi), **MCP** (`crates/native/mcp/tests/axis_sections.rs`), **masaüstü**
  (`compare_sections_tests`: pencerenin cevabı yaylı eksen durumununki; `surface_analysis_window_tests`: kesitler,
  yazıları ve cetvel durumunki, Malzemeler işaretliyken malzeme yazıları; çapraz denetimler). Resimler
  `.run/gui-evidence/aks-boyunca-kesit-4760/` (pencere, kesit grafiği ve tablosu, 1:100'de çizim).

## Sonra

- Kesit sayfaları (kesitin kendi çizimi) ve UVDF §6.4 biçiminde dışa aktarım.
- Kesik koni formülü; Brükner (kütle ordinatı) sütunu katsayılarla.
- Netcad'in ek kesitleri: üçgen modelin kenarlarında ve ekseni kesen çizgilerde.
- Web (TODOS §0.2).
