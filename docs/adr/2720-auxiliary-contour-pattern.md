# 2720 — Yardımcı eş yükselti eğrisinin 5/1 mm deseni ve oluşturmada proje çizgi tipi

Tarih: 2026-11-08. plan/map-cad.md MAP-14, dördüncü tur. Durum: taslak. ADR 1820 ve 2320'nin “Sonra”sındaki “yardımcı
eğrinin 5/1 mm deseni (komutun nesne biçimi önce `linePattern` almalı)”.

İlgili:
- [ADR 1820](1820-sheet-contours-by-article-78.md): pafta eş yükselti eğrileri (m. 78);
- [ADR 2320](2320-sheet-summit-and-pit-heights.md): tepe ve çukur kotları;
- [ADR 0313](0313-line-type-patterns.md): çizgi tipleri; LIN desenleri, ölçek ve faz.

## Dayanak

- **BÖHHBÜY 2018** (RG 26.06.2018/30460 Mük.), Ek-3, işaret 0003 (ADR 1820'nin okuması, PDF'in sayfa resmi 7 Kasım 2026):
  - yardımcı eğri 0,13 mm, kesik çizgi: 5 mm çizgi, 1 mm boşluk;
  - ara eğriler çok açık olduğunda ve yapısal değişiklik varsa çizilir;
  - m. 78(1)(e): paftada eğri arası 20 mm'yi aşınca araya bir kesik yardımcı eğri girer.
- **ADR 0313:**
  - proje çizgi tipleri ayarlardadır, `paper` birimli tip kâğıtta mm'dir, nesne ve katman tipi adıyla anar;
  - tablo proje ayarıdır, geri alma adımı değildir;
  - `cad.entities.set` ve `cad.layers.edit` `linePattern` alıyordu, `cad.entities.create` almıyordu.
- **KentOS bugün:**
  - pafta eğrileri yardımcı eğriyi yerleşik Kesikli tiple çiziyordu (kâğıtta 3 + 1,5 mm gibi);
  - Ek-3'ün 5/1 mm'si veri olarak yoktu, komut nesneye desen veremiyordu.

## Karar

1. **`cad.entities.create`'in nesnesine `linePattern`** (ek alan; komut sürümü değişmez):
   - projenin çizgi tiplerinden biri adıyla;
   - ADR 0313'ün `cad.entities.set` kuralı oluşturmada da geçerlidir: tek başına verilen desenin en yakın yerleşik tipi
     nesnenin `lineType`'ı olur, birlikte verilen tip kalır;
   - projede olmayan ad `line_type_not_found` (`objects[i].linePattern`, `cad.entities.set`'in iletisi).
   - Komutların oluşturduğu nesneler (82 yer) alanı boş verir; davranış değişmez.
2. **Kural paketinde `auxiliaryPattern`** (kâğıtta mm: çizgi artı, boşluk eksi):
   - `bohhbuy-2018-m78` sürüm 3: `[5, −1]`, kaynakta Ek-3 0003;
   - yoksa yardımcı eğri yerleşik Kesikli tiple çizilir;
   - bozuk desen (12'den çok öğe, sonlu olmayan, çizgisiz) paketin eksikliği sayılır (`invalid_contours`).
3. **`road.terrains.sheetcontours`:**
   - yardımcı eğri varsa ve paket desen veriyorsa yardımcı eğriler “Yardımcı eğri (‹paket›)” proje çizgi tipini anar,
     yerleşik tipleri Kesikli;
   - tip projede yoksa paketin deseniyle eklenir (`paper`, kaynak paketin kimliği); varsa projedeki kullanılır,
     değiştirilmez (kullanıcının tipi);
   - cevabın `pattern`'ı tipin adı; not deseni ve kuralı söyler.
4. **Geri alma:** eğriler ve yazılar tek adımdır (“Pafta eş yükselti eğrileri”). Eklenen tip ADR 0313 gereği proje
   ayarıdır, geri almada kalır; gerekmezse Çizgi tipleri'nden kaldırılır.
5. **Kapsam işareti:** ADR 1820'deki gibi **Harita + Yol**.

## Kanıt

- **Ortak durumlar:**
  - `scripts/fixtures/create_command_cases.py`, 86 durum (84'ten): desen tek başına ve tiple; olmayan ad reddedilir.
  - `scripts/fixtures/road_sheet_contour_cases.py` (uv ile `triangle`), 36 durum (32'den):
    - desen ayara eklenir, geri almada kalır; projede aynı adlı tip varsa projedeki kalır;
    - desensiz paket yerleşik Kesikli'yle; bozuk desen reddedilir.

  Eski durumların eğrileri aynı; paketin sürümü ve kaynak yazısı değişti, notlara desen satırı eklendi. Rust ve Python
  geçer.
- **MCP** (`crates/native/mcp/tests/auxiliary_contour_pattern.rs`):
  - eğriler çizilir, cevap tipin adını verir;
  - `.kcad` olarak kaydedilip okununca tip 5/−1 `paper`, her yardımcı eğri onu anar.
- **Masaüstü** (`sheet_contours_window_tests.rs`):
  - Çiz'den sonra tip projede, yardımcı eğriler onu anar, yerleşik tipleri Kesikli;
  - geri alma eğrileri kaldırır, tipi bırakır.

  Çapraz denetimler geçer. Resim `.run/gui-evidence/yardimci-egri-deseni-2720/`: 1:1000'de 5 m çizgi, 1 m boşluk.

## Sonra

- Şev üstü ve dibi kotları yapıldı: [ADR 3120](3120-sheet-slope-top-and-bottom-heights.md).
- Yüzey birleştirme; bölgesel yeniden kurma (MAP-14'ün öbür maddeleri).
- Web (TODOS §0.2).
