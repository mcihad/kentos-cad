# 2320 — Pafta eğrilerinde tepe ve çukur kotları (m. 78(1)(ğ))

Tarih: 2026-11-08. plan/map-cad.md MAP-14, üçüncü tur. Durum: taslak. Plan satırı: “Yüzeyden … eş yükselti ve
pafta çıktısı mevzuatın çizim kurallarıyla üretilsin.”

İlgili: [ADR 1820](1820-sheet-contours-by-article-78.md), BÖHHBÜY m. 78'e göre pafta eğrileri. Bu ADR onun
“Sonra”sındaki “m. 78(1)(ğ) karakteristik nokta kotları” maddesinin tepe ve çukur kısmını kapatır.

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460 Mükerrer; ADR 1820'nin kopyası, Çanakkale Onsekiz Mart Üniversitesi Harita
  Mühendisliği, PDF'in metni 8 Kasım 2026'da okundu, s. 6986):
  - **m. 78(1)(ğ):** arazi topoğrafyasını tamamlamaya yardımcı olacak tepe, çukur, şev, dip ve üstlerindeki
    karakteristik noktalar ile gerekli görülen diğer noktalar paftada işaretlenir ve yükseklik değerleri dm'ye kadar
    yazılır;
  - **Ek-3 0004:** kot noktasının yazısı 1,2 mm'dir (ADR 1820'nin okuması).
- **KentOS bugün** (ADR 1820): eğriler, kalın eğrilerin kot yazıları ve detayların kesilmesi vardı. Tepe ve çukurun kotu
  yoktu: en içteki kapalı eğri tepenin yalnız bir aralık altını söylüyordu.

## Karar

1. **`spots` girdisi** (`road.terrains.sheetcontours`, isteğe bağlı, ilk değer kapalı). Açıkken:
   - **En içteki kapalı eğri:** kalın ya da ara, detaylarca açılmamış (kapalı), içinde başka böyle bir eğrinin ilk
     noktası olmayan eğri (tek-çift kuralı).
   - **Nokta:** o eğrinin içindeki yüzey köşelerinden eğrinin kotunun üstündeki en yüksek olanı (**tepe**), yoksa
     altındaki en alçak olanı (**çukur**).
     - Eşitlikte önceki köşe alınır.
     - Kotu eğrininkine eşit köşe eğrinin üstündedir, sayılmaz.
     - **Detayın içindeki köşe sayılmaz** (bina, alan olarak çizilmiş yol, şev sınırı). Orada zemin gösterilmez;
       m. 78(1)(g)'nin eğri için söylediği detay kuralının kot için karşılığıdır.
   - **Yazı:** kot dm'ye kadar, yarım yukarı yuvarlanmış tek ondalık (899,38 → “899.4”). Yazı gösterimdir; noktanın
     kotu kesin değeriyle yazılır.
   - **Yazılanlar:** köşede kotlu bir nokta ve sağında, yazı yüksekliğinin yarısı kadar ötede, ortadan sola hizalı
     maskeli yazı. Yazı yüksekliği paketin `spotHeight`'ıdır (mm) × ölçek.
2. **Kural paketi:** `esyukselti-bohhbuy-2018`'e `spotHeight: 1.2` (Ek-3 0004) eklendi. Paketin sürümü 2 oldu, kaynağı
   (ğ)'yi anar. `spots` istenip paket `spotHeight` vermiyorsa ret: `invalid_contours` (`rules`). Değer uydurulmaz.
3. **Kapsam sınırı:**
   - (ğ)'nin tepe ve çukuru yapılır;
   - **şev, dip ve üstü** ile “gerekli görülen diğer noktalar” yapılmaz. Not bunu söyler: şev kenarının çizgisi ya da
     ölçmecinin seçimi gerekir.
4. **Cevap:** `spots` (tür, yer, kot, yazı) ve bir not: kaç nokta, yazı yüksekliği, kural ve kapsam sınırı.
5. **Masaüstü** (Pafta eğrileri):
   - Kurallar satırına **Tepe ve çukur kotları (ğ)** kutusu (ilk değer kapalı, komutunki gibi);
   - özette her nokta “Tepe kotu 897.6 (Y …, X …)” diye;
   - Çiz eğriler ve yazılarla birlikte noktaları da tek adımda yazar.
6. **Kapsam işareti:** ADR 1820'deki gibi **Harita** (Arazi › Yüzey).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/road_sheet_contour_cases.py` (uv ile `triangle`), KentOS kodu yok.
  - En içteki eğri ve köşeler elle aranır. Tek-çift testinin aritmetiği iki tarafta aynıdır.
  - Yeni bir yüzey eklendi: ilk yüzeyin yanında 81 sarsılmış noktadan bir çanak (8 m derin, dibi 892,3 m). Düzgün
    ızgara Delaunay köşegenlerini belirsiz bırakırdı; bu yüzden noktalar sarsıldı.
  - Yeni durumlar:
    - tepe: 895 eğrisinin içinde 899,38 m → “899.4”;
    - çukur: 893 eğrisinin içinde 892,305 m → “892.3”;
    - plan yazmaz;
    - detaylarla: tepe köşesi binanın içinde kaldığı için binanın dışındaki en yüksek köşe, 897,645 m → “897.6”;
    - `spotHeight`'sız paketle ret.
  - 32 durum (27'den). Eski durumların cevaplarında yalnız paketin kaynağı ve yeni yüzeyin listesi değişti; iki durum
    paketi girdisinde taşıdığı için girdisi değişti.
  - Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/sheet_spot_heights.rs`): durumların çizimi `.kcad` olarak açılır. Planın tepe,
  bina dışı tepe ve çukur noktaları durumlarınkidir.
- **Masaüstü** (`sheet_contours_window_tests.rs`, `the_summit_height_is_the_shared_case_s`):
  - kutu işaretlenince plan durumun tepesidir (pencere detayları işaretli açılır);
  - Çiz iki nesne daha yazar.

  Çapraz masaüstü denetimleri 134 geçer. Resimler `.run/gui-evidence/tepe-kotu-2320/`.

## Sonra

- Şev üstü ve dibi kotları yapıldı: [ADR 3120](3120-sheet-slope-top-and-bottom-heights.md) (şevlerin üst ve dip
  çizgilerinin köşeleri). Ölçmecinin seçtiği öbür noktalar kaldı.
- Yardımcı eğrinin 5/1 mm deseni: `cad.entities.create` nesneye proje çizgi tipini (`linePattern`) veremiyor. Önce
  komutun nesne biçimine eklenmeli. Yapıldı: [ADR 2720](2720-auxiliary-contour-pattern.md).
- Yüzey birleştirme, eğrilerin yeniden kurulan çizimlere girmesi (ADR 1820 Sonra).
