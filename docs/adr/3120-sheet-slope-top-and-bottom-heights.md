# 3120 — Pafta eğrilerinde şev üstü ve dibi kotları (m. 78(1)(ğ))

Tarih: 2026-11-09. plan/map-cad.md MAP-14, beşinci tur. Durum: taslak. ADR 2320'nin “Sonra”sındaki “şev üstü ve dibi
kotları (şev sınırı çizgisinden)”.

İlgili:
- [ADR 1820](1820-sheet-contours-by-article-78.md): BÖHHBÜY m. 78'e göre pafta eğrileri (`road.terrains.sheetcontours`);
- [ADR 2320](2320-sheet-summit-and-pit-heights.md): tepe ve çukur kotları (`spots`, kot noktası yazısı);
- [ADR 2720](2720-auxiliary-contour-pattern.md): yardımcı eğrinin deseni.

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460 Mükerrer; ADR 1820 ve 2320'nin okuduğu metin):
  - **m. 78(1)(ğ):** arazi topoğrafyasını tamamlamaya yardımcı olacak tepe, çukur, şev, dip ve üstlerindeki
    karakteristik noktalar paftada işaretlenir ve yükseklik değerleri dm'ye kadar yazılır;
  - **Ek-3 0004:** kot noktasının yazısı 1,2 mm (paketin `spotHeight`'ı).
  - Yönetmelik şevin hangi noktasının “karakteristik” olduğunu, aralığını ya da sayısını söylemez.
- **KentOS bugün** (ADR 2320):
  - tepe ve çukur yazılıyordu. Not “şev üstü ve dibi işaretlenmedi” diyordu: şev kenarının çizgisi ya da ölçmecinin
    seçimi gerekir;
  - şevin çizgileri çizimde ölçmecinin kendi çizdiği nesnelerdir (üst çizgi, dip çizgisi). Köşeleri ölçülen
    noktalardır.

## Karar

1. **`slopes` girdisi** (`road.terrains.sheetcontours`, ek alan; komut sürümü değişmez): `top` şevlerin üst
   çizgilerinin katmanları, `bottom` dip çizgilerinin katmanları. İkisi de boşsa istenmemiş sayılır.
2. **Karakteristik nokta:** bu katmanlardaki çizgi ve çoklu çizgilerin (parçaları dahil) her köşesi.
   - Önce üst katmanlar, sonra dip katmanları; çizimin sırasıyla, köşelerin sırasıyla;
   - aynı yer (aynı koordinatlar) bir kez: iki çizginin ortak köşesi tek nokta;
   - kot yüzeyden: köşenin üçgeninin düzlemi. Şev çizgisi yüzeyin kırıklığıysa köşesi yüzeyin köşesidir, kot aynıdır;
   - yüzeyin dışındaki (ya da boşluğundaki) köşe yazılmaz, sayılır;
   - detay kuralı uygulanmaz: köşeleri ölçmeci seçmiştir (tepe ve çukurda detayın içindeki köşe sayılmaz, ADR 2320).
   - Ara nokta, aralık ya da açı eşiği uydurulmaz: yönetmelik vermez; köşeyi ölçmecinin ölçtüğü nokta sayarız.
3. **Yazı:** tepe ve çukurunki gibi: kot dm'ye kadar, yarım yukarı (863,62 → “863.6”); köşede kotlu nokta ve sağında
   maskeli yazı. Yazı yüksekliği paketin `spotHeight`'ı × ölçek; vermeyen pakette ret (`invalid_contours`, `rules`,
   “şev kotları yazılmaz”).
4. **Cevap:** `spots`'ta türler `slopeTop` ve `slopeBottom`, tepe ve çukurdan sonra. Not: üstte ve dipte kaç nokta,
   yazı yüksekliği, kaynağın kuralı, yüzeyin dışında kalan köşe sayısı. Şevler istenince tepe notundaki “şev üstü ve
   dibi işaretlenmedi” kalkar.
5. **Retler:** olmayan katman `layer_not_found` (`slopes.top`, `slopes.bottom`, ““‹ad›” katmanı çizimde yok.”).
6. **Masaüstü** (Pafta eğrileri):
   - **Şevler (ğ)** satırı: **Şev kotları** kutusu (ilk değer kapalı, tepe ve çukurunki gibi), **Üstü** ve **Dibi** katman
     seçimleri (Yok seçilebilir);
   - katmanlar adlarından tahmin edilir: “şev” ile “üst”; “şev” ile “dip”, “dib” (dibi), “alt” ya da “taban”. Tahmin
     edilenler detay listesinden çıkar;
   - Çiz'in iletisi “karakteristik nokta kotu” der.
7. **Kapsam işareti:** ADR 1820'deki gibi **Harita + Yol** (Arazi › Yüzey).

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/road_sheet_contour_cases.py`, uv ile `triangle`; KentOS kodu yok):
  - çizime şevin üst çizgisi (dört köşeli çoklu çizgi ve son köşesinden yüzeyin dışına, y 320'ye giden çizgi) ve dip
    çizgisi (beş köşe) eklendi, sarp yamacın iki yanında. Önceki nesnelerin kimlikleri aynı kalsın diye en sona;
  - başvuru köşelerin kotunu Triangle'ın üçgeninin düzleminden kendisi hesaplar;
  - yeni durumlar: üst ve dip birlikte (ortak köşe bir kez, yüzeyin dışındaki köşe sayılır; üstte 4, dipte 5 nokta), plan
    yazmaz, yalnız üst, tepeyle birlikte (tepe önce), boş katmanlar istenmemiş sayılır, `spotHeight`'sız paket, olmayan
    üst ve dip katmanı. Ek durumlar dosya büyümesin diye 1/5000'de;
  - 44 durum (36'dan). Eski durumların çiziminde iki katman ve üç nesne var, cevapları aynı. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/sheet_spot_heights.rs`): şev kotlarının ve tepeyle birlikte durumun planı
  durumlarınki (kotlar 10⁻⁹ m içinde).
- **Masaüstü** (`sheet_contours_window_tests.rs`, `the_slope_heights_are_the_shared_case_s`):
  - Şev üstü ve Şev dibi katmanları tahmin edilir, detaylara girmez; kutu kapalıyken nokta yok;
  - kutu işaretlenince noktalar ortak durumunkidir; not yüzeyin dışındaki bir köşeyi söyler;
  - Dibi Yok seçilince yalnız dört üst nokta; Çiz her nokta için nokta ve yazı yazar.

  Pafta eğrilerinin öbür testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/sev-kotlari-3120/`.

## Sonra

- ~~Ölçmecinin seçtiği öbür noktalar (m. 78(1)(ğ)'nin “gerekli görülen diğer noktalar”ı): bir nokta katmanı.~~
  [ADR 3520](3520-sheet-chosen-point-heights.md).
- ~~Noktanın kendi ölçülmüş kotunun yüzeyinkinden ayrıldığı yerin bildirilmesi.~~ ADR 3520: notta en büyük fark ve yeri.
- Yüzey birleştirme; bölgesel yeniden kurma (MAP-14'ün öbür maddeleri).
- Web (TODOS §0.2).
