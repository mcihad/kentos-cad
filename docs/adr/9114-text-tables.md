# ADR 9114: Yazı tabloları — taşımada yazılar kendi tablolarına gider, tablodan yazı olarak çizilir

- **Durum:** kabul edildi (2026-11-22). Plan: [plan/veri.md](../../plan/veri.md) VY-17 (ikinci tur).
- **İstek:** sahibin sözü (21 Kasım 2026): “Hepsini düzelt eksikleri tamamla.” Kapatılan kalan: ADR 9107 §9, “Taşıma
  tabloya yazılamayan nesneleri (yazı, ölçü) ayrı bir katmana kendisi almaz”. Yazı tutan bir katman taşınmıyordu
  (`unwritable_objects`).
- **Sözleşmeler:**
  - Veri katmanına `text` eşlemesi eklendi (`DataText`, §1).
  - `.kcad`: yazı eşlemesi olan proje **CBS verisi 5** ile yazılır (`GIS_WITH_TEXT_TABLES`). Daha eski bir sürümün
    dosyasında `text` bilinmeyen alandır. Yazı katmanı olmayan proje bayt bayt aynı yazılır.
  - `cad.layers.migrate` çıktısına `texts` eklendi (`MigratedTexts`, §2).
  - Yeni uyarı kodu: `text_details_left`.
  - `cad.layers.data` eşlemeyi alır ve denetler.
  - Katalog, TypeScript ve Python yeniden üretildi.
- **Bağımlılık:** yok.

## §1 Karar: yazı tablosu, satırları yazı olan veri katmanıdır

**Sorun:** bir tablonun geometri sütunu yazıyı tutmaz. Yazı bir noktadır, ama onu yazı yapan şeyler (sözleri,
yüksekliği, açısı, hizası) özniteliktir. CAD çiziminde bunlar nesnenin kendisindedir.

**Masaüstü CBS:**

- QGIS'in DWG/DXF içe aktarıcısı yazıları GeoPackage'ın `texts` tablosuna yazar.
  - Katman sembol çizmez (boş işleyici), yalnız etiket çizer.
  - Etiketin metni `text` sütunu, boyu `height` sütunu (harita biriminde), rengi `color` sütunudur.
  - Yatay ve düşey hiza `alignh` ve `alignv` sütunlarından gelir. Dönüşü `360-angle` ifadesidir: QGIS etiketi saat
    yönünde döndürür, açı tersine yazılmıştır.
  - Etiket her zaman çizilir (`AlwaysShow`), çakışma yüzünden atılmaz.
- GDAL'ın DXF sürücüsü TEXT ve MTEXT'i nokta detayı olarak okur. Yazının parçaları LABEL stil dizesindedir: metin,
  açı, boy, hiza, renk.
- **BÖHHBÜY Ek-2**, Ulusal Veri Değişim Formatı (UVDF) §3.2.6.1 “Yazi”, bir yazıyı şu parçalarla tutar:
  - Koor: noktası;
  - Metin;
  - Aci: yatay eksenle açısı;
  - Boyut: milimetre cinsinden yüksekliği;
  - GenislikFaktoru;
  - DayamaNoktasi: uygulama noktası;
  - Font.

**Karar:** veri katmanının `text` eşlemesi, satırlarının yazı olduğunu ve hangi sütunun neyi tuttuğunu söyler:

| Alan | Sütun (KentOS'un yazdığı) | Okuma |
|---|---|---|
| `text` | `metin` | yazının sözleri; boşsa satır yazı değildir |
| `height` | `yukseklik` | metre; boş, sayı değil ya da 0'dan büyük değilse projenin yazı yüksekliği |
| `angle` | `aci` | derece, doğudan saat yönünün tersine; boşsa 0 |
| `align` | `hiza` | Yazı'nın adları (“sol alt”, “orta”, “sağ üst” …) ya da sözleşmenin (`bottomLeft`); başka bir söz taban çizgisinin solu |
| `width` | `genislik` | genişlik çarpanı; boş, 1 ya da (0, 100] dışındaysa yok (1) |

- **Yükseklik metredir.** UVDF'nin Boyut'u kâğıt milimetresidir; dönüşüm yükseklik = Boyut × ölçek / 1000'dir.
- **Projenin yazı yüksekliği:** Yazı aracınınkiyle aynıdır, projenin yazı boyu çizim ölçeğinde.
- **Okuma:** sayılarda ondalık virgül de okunur. Hiza adı Yazı aracının katlamasıyla okunur: bitişik ya da ayrı,
  Türkçe imli ya da imsiz.
- **Eşleme başka programın tablosuna da uyar:** sütun adları serbesttir, yalnız sözler zorunludur.
- **Denetim (`problem`):**
  - sütun adları boş olamaz;
  - bir sütun yazının iki parçası olamaz;
  - yazı sütunu anahtar ya da geometri olamaz;
  - tablonun geometrisi nokta olmalıdır (POINT ya da MULTIPOINT; çok noktalıda ilk nokta).
- **Arayüzde:** katmanın türü “Veri katmanı (yazı)” diye yazılır.

## §2 Karar: taşımada yazılar kendi tablolarına

`cad.layers.migrate`, katmanın yazılarını ana tablonun yanındaki `<tablo>_yazilar` tablosuna yazar: bir nokta ve §1'in
beş sütunu, projenin sisteminde.

- **Tablonun adı:** 63 harfi geçmez; gerekirse ana tablonun adı kısaltılır.
- **Ana tablo:** dışa aktarmanın planı yazıları ayrı tutar (`texts_apart`). Yazılar ne yazılır ne de “yazılamadı”
  diye sayılır.
- **Yazma sırası:** yazılar ana tablodan sonra kendi işleminde yazılır.
  - Yazılamazlarsa ana tablo geri alınır. Taşımanın yaptığı bir GeoPackage bütün olarak silinir; hiçbir şey yarım
    taşınmaz.
- **Katmanlar:**
  - Ana tablonun veri katmanı katmanın yerine geçer: adı, stili, etkinliği.
  - Yazıların veri katmanı onun üstüne konur, adı “<ad> (yazılar)”, rengi katmanınki. Üstte olduğu için yazılar
    noktaların üstünde çizilir.
  - Hepsi tek geri alma adımıdır.
- **Yalnız yazı tutan katman:** yazılar katmanın kendi tablosuna yazılır (ek almaz). Yazı katmanı katmanın yerine,
  adıyla geçer. Eskiden `nothing_to_export` ile reddediliyordu.
- **Hâlâ reddedilenler:** ölçü, blok, tarama ve tablonun tutamadığı başka bir nesne `unwritable_objects` ile reddedilir.
  - İleti artık ilk nesnenin türünü söyler: “ölçü bir geometri sütununa yazılmaz”.
- **Tablonun tutmadıkları:** yazı stili ya da yazı tipi, zemin, eğri, ölçekler, nesne bağı ve paragraf biçimi yazılmaz.
  Bunlardan biri olan yazılar sayılır ve `text_details_left` uyarısıyla söylenir.
- **Hedefte tablo varsa:** yazıların tablosu varsa da `target_exists` ile reddedilir; `table` ikisinin de adını
  değiştirir.
- **Çıktı:** `texts`: yazı katmanı, tablosu, satır sayısı. Plan da yazıları sayar.
- **Pencere:** Veri kaynağına taşı… penceresi ne olacağını önceden söyler: “20 nesne tablonun satırları olur; 22 yazı
  yanındaki “eski_direkler_yazilar” tablosuna.” Bildirim yazıların tablosunu da söyler.

## §3 Karar: yazı katmanı yazı olarak çizilir

Yazı katmanının her satırı söylediği yazı nesnesi olur (`data_text::object`).

- Bu nesneler çizimin kendi yazılarının çizildiği yoldan çizilir: projenin yazı tipi, hiza, çok satır, genişlik
  çarpanı. Yazı çakışma yüzünden atılmaz.
- Stil motoru yazıya sembol çizmez (`Shape::Text → None`). Bu yüzden okunan nesneler etiket belgesinde her zaman
  tutulur (`labelled`) ve oradan çizilir.
- Okuma, eşlemenin sütunlarını da ister.
- **Sınır:** etiketlerin 20 000 satır sınırı burada da geçerlidir. Daha geniş bir görünümde yazılar çizilmez; o ölçekte
  okunmazlar da.

## §4 Karar: yazı katmanı yazı olarak düzenlenir

- **Alma:** düzenlemede alınan satır yazı nesnesi olarak gelir.
- **Kaydetme:** yazı nesnesi satırına döner (`as_row`): yerinde bir nokta, parçaları kendi sütunlarında, öteki
  değerlerinin üstüne.
  - Çizilen yazının kendisi olduğu için aynı sütundaki özniteliğe üstün gelir.
- **Değişiklik ve çakışma denetimi:** başkasının değiştirdiği satır bu normalleştirilmiş satırlar üzerinden bulunur.
  Yazının yeri değişince de bulunur.
- **Yeni yazı:** yazı katmanı yazıyı alır. Katman bir nokta katmanı olduğu için çizimin bütünlük kuralı yazıyı
  reddediyordu; yazı katmanı için açıldı.
  - Yazı aracıyla yazılan yeni yazı yeni satır olur.

## §5 Planda olmayan kararlarım

1. **Eşleme veri katmanında, etiket ayarında değil.** Etiket ayarında satıra bağlı boy, açı ve hiza yoktur. Böyle bir
   etiket motoru yazının taban çizgisi hizasını, çok satırı ve genişlik çarpanını da ayrıca öğrenmek zorunda kalırdı.
   Satırı yazı nesnesi yapmak, CAD yazısının çizimini olduğu gibi kullanır.
2. **Planın dört sütununa hiza ve genişlik eklendi.** Hiza olmadan ortalı ve sağa yaslı yazılar kayar. UVDF'de ve
   QGIS'in tablosunda ikisi de vardır.
3. **Hiza Türkçe adıyla yazılır** (“sol taban”, “orta”). Okurken sözleşmenin adı da kabul edilir.
4. **Genişlik 1 “yok” okunur.** KentOS yazısı 1'i yazmaz (ADR 0145); tabloda 1 yazılır.
5. **Yalnız yazı tutan katman ek almaz:** tablonun adı katmanınkidir.
6. **Yazılar yazılamazsa ana tablo geri alınır.**
7. **`text_details_left` uyarısı:** hiçbir şey sessizce düşmesin diye (ADR 9107 §8 “hiçbir şey kaybolmaz”).
8. **Çizimin bütünlük kuralı yazı katmanında yazıyı kabul eder** (§4).
9. **Dışa aktarma planına `texts_apart` bayrağı eklendi.** Dışa aktarmanın kendisi değişmedi: bir çizim katmanını
   dışa aktarırken yazılar eskisi gibi atlanır ve sayılır.

## §6 Kanıt

**Sınamalar:**

- Sözleşme (`kentos-contracts`):
  - satır yazıya, yazı satıra döner;
  - boş ve garip hücreler varsayılanı alır;
  - Yazı'nın 12 hiza adı, bitişik, imsiz ve büyük harfle yazılmışları ve sözleşme adları okunur;
  - yazı tablosu nokta tablosudur ve eşleme dosyadan aynen okunur.
- Uygulama: satır yazı nesnesi olur ve öteki değerleri kalır; sözsüz satır yazı değildir; tablonun adı 63 harfi
  geçmez.
- Masaüstü:
  - **Taşıma:** noktalar ve yazılar taşınınca yazı katmanı üstte, adıyla ve renginde gelir. Okununca satırları kendi
    sözleri, yükseklikleri, açıları, hizaları ve genişlikleriyle yazı olur.
  - **Düzenleme:** yazı katmanında satır yazı olarak alınır. Sözü, açısı, yeri ve hizası değiştirilir, yeni bir yazı
    yazılır. Kaydedilince tablo yeni değerleriyle geri okunur (`metin` = “Ada 102”, `hiza` = “sağ üst”).
- **`cad.layers.migrate`:** 28 durum, 6'sı yeni ya da değişen:
  - yalnız yazı tutan katman (iki yazı, biri döndürülmüş, ortalı ve dar);
  - nokta ve yazı, yazı tablosunun satırları yazı katmanından okunur;
  - plan yazıları sayar;
  - zeminli yazının uyarısı;
  - yazıların tablosu var;
  - ölçü hâlâ reddedilir.
- **`cad.layers.data`:** 38 durum, 3'ü yeni: yazı katmanı ekleme, alan tablosunda ret, aynı sütun iki parça.
- **`.kcad`:**
  - `text-tables.kcad` iki okuyucuda da okunur ve aynı baytlarla yeniden yazılır;
  - CBS verisi 4'te yazı eşlemesi `unknown_field` alır;
  - alan tablosunda eşleme ve iki parça için aynı sütun `bad_value` alır.

  Rust ve Python okuyucuları 1880 dosyanın hepsinde `expected.json` ile aynı sonucu veriyor.
- Bütün ortak durumlar ve yetenek denetimi Rust'ta geçti.
- Python'da veri katmanının dört dosyası (oturum, taşıma, veri, satırlar) geçti. Bozuk bir beklentiyi (yazı
  satırının sözü) Python koşucusu yakaladı.

**Gerçek pencere:** projede 20 direk ve 22 yazı tutan “Eski direkler” katmanı. Görüntüler
`.run/gui-evidence/veri/vy17/sonra`.

1. Veri kaynağına taşı… penceresi “20 nesne tablonun satırları olur; 22 yazı yanındaki “eski_direkler_yazilar”
   tablosuna” dedi.
2. Taşı'dan sonra:
   - harita önceki hâliyle aynıydı, artık iki tablodan çiziliyordu;
   - ağaçta “Eski direkler (yazılar)” direklerin üstündeydi;
   - projede 0 nesne kaldı;
   - bildirim yazıların tablosunu söyledi.
3. Yakından bakınca 30° döndürülmüş direk numaraları ve ortalı, 0,85 dar “Kuzeydoğu hattı” tablodan çiziliyordu.
4. Yazı katmanının tablosunda 22 satır vardı; sütunlar metin, yukseklik, aci, hiza, genislik.

**Dosyanın kendisi:**

- sqlite3 ile okundu:
  - iki tablo da EPSG:5256 POINT;
  - yazı tablosunun sütunları `metin TEXT`, `yukseklik DOUBLE`, `aci DOUBLE`, `hiza TEXT(16)`, `genislik DOUBLE`;
  - 20 + 22 satır ve mekânsal indeks;
  - “Kuzeydoğu hattı” satırı 6,0 · 30,0 · orta · 0,85.
- Kurulu masaüstü CBS'nin kitaplığı iki tabloyu geçerli, EPSG:5256'da, 20 ve 22 detayla okudu. Türkçe sözler
  bozulmadı.

## §7 Kapsam dışı ve kalan

- **Ölçü** hâlâ taşınmaz. Ölçü bir çizgi, oklar ve bir yazıdır; parçalanarak taşınması ayrı bir karar.
- **Tabloya yazılmayanlar:** yazı stili ya da yazı tipi, zemin, eğri boyunca yazı, açıklama ölçekleri, nesne bağı ve
  paragraf biçimi tabloya yazılmaz; söylenir (§2).
- **Öznitelik düzenleme:** yazı katmanında metin, yükseklik, açı, hiza ve genişlik Öznitelikler'den değil, yazının
  kendisinden düzenlenir; aynı sütunun özniteliği kaydederken yazının değeriyle yazılır (§4).
- **Web:** ertelendi (ADR 0228). Python ve MCP yazı katmanını `cad.layers.data` ile, `text` eşlemesini vererek ekler.

## Uygulama

- **Sözleşme:**
  - `crates/shared/contracts/src/gis/data_text.rs` (yeni): `DataText`, `TextParts`, `TextCell`, hiza adları;
  - `data_layer.rs`: `text`, denetim, “Veri katmanı (yazı)”;
  - `core/codes.rs`: `text_details_left`.
- **Komut:**
  - `crates/shared/commands/src/gis/cad_layers_migrate.rs`: `texts`, `MigratedTexts`;
  - `capabilities/gis.rs`: uyarı.
- **Uygulama:**
  - `crates/native/application/src/gis/data_text.rs` (yeni);
  - `layers_migrate_texts.rs` (yeni);
  - `layers_migrate.rs`: sayım, yazma, katmanlar;
  - `layer_export.rs`: `texts_apart`;
  - `data_write.rs`: alma ve kaydetme, `as_row`;
  - `layers_create.rs` ve `layer_export_sinks.rs`: `text: None`.
- **Alan:** `crates/native/domain/src/integrity.rs` (yazı katmanı yazıyı alır).
- **`.kcad`:**
  - `crates/shared/kcad/src/lib.rs`: `GIS_WITH_TEXT_TABLES`;
  - `encode.rs`: sürüm;
  - `decode/layers.rs`: kapı;
  - `tools/kcad/kcad.py`: okuyucu ve denetimi;
  - `scripts/fixtures/kcad_v2_reference.py`: yazıcı ve dört örnek;
  - `fixtures/kcad/v2/text-tables.json` ve `expected.json`: elle.
- **Masaüstü:**
  - `apps/desktop/src/gis/data_view.rs`: yazı olarak çizim;
  - `layer_migrate.rs`: pencerenin sözü ve bildirim;
  - `layer_migrate_tests.rs`: iki yeni sınama;
  - `data_layers.rs`.
- **Ortak durumlar:** `scripts/fixtures/layer_migrate_command_cases.py`, `layer_data_command_cases.py`.
