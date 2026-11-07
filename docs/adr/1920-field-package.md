# 1920 — Saha paketi: tasarımdan sahaya giden tek dosya (`cad.field.package`)

Tarih: 2026-11-07. plan/map-cad.md MAP-19, ikinci tur. Durum: taslak. Plan satırı: “Tasarımdan sahaya çıkacak paketi
açık tanımla: seçilen revizyon, nokta/hat/yüzey/varlık kimlikleri, CRS/epok, kot referansı, kabul eşiği ve gerekli
kalite notları. Sadece isimsiz koordinat listesi olmasın.” İlgili:

- [ADR 1700](1700-asbuilt-check.md): Uygulama kontrolü. Paket tasarımı onun okuduğu gibi okur; tasarım özeti onunkidir.
- [ADR 0169](0169-field-data.md) §7: Cihaza gönder (`field.send`). Noktaları cihazın kendi biçiminde, milimetreye
  yuvarlayarak yazar. Paket onun yerine geçmez.
- [ADR 1031](1031-road-field-files-say-their-systems.md) ve [ADR 0519](0519-vertical-crs-and-epoch.md): sahaya giden
  dosyanın koordinat ve kot sistemini söylemesi; projenin epoku ve kot sistemi.

## Dayanak (7 Kasım 2026'da okundu)

- **Trimble Access:**
  - Bir iş (job) şunları taşır: koordinat sistemi, birimler, bağlı dosyalar (Layer manager), özellik kütüphanesi,
    medya, Reference / Description / Operator / Notes alanları
    ([Creating a job](https://help.fieldsystems.trimble.com/trimble-access/latest/en/job-create.htm)).
  - Ofiste kurulan iş Trimble Connect ile sahaya gider; iş durumu değişince (“In progress”, “Fieldwork complete”)
    buluta yüklenir.
  - TBC'nin JobXML'inden kurulan iş yalnız elle girilmiş nokta verisini taşır.
  - Ekip için talimat, Sync Manager'da işin açıklamasına yazılır
    ([Sync Manager](https://help.fieldsystems.trimble.com/trimble-sync-manager/get-started.htm)).
  - Aplikasyonda tasarım ile aplike edilen nokta arasındaki farklar kaydedilmeden önce gösterilir. Yatay tolerans
    kullanıcının ayarıdır; 0 verilirse her seferinde gösterilir
    ([As-staked point details](https://help.fieldsystems.trimble.com/trimble-access/latest/en/stakeout-as-staked-point-details.htm)).
- **LandXML 1.2, buildingSMART MVD başlığı:**
  - Tek bir koordinat sistemi olur. EPSG kullanılıyorsa `name` ve `epsgCode` gerekir. Düşey sistem yataydan ayrıysa
    `verticalCoordinateSystemName` gerekir.
  - `Application`'ın `timeStamp`'i `yyyy-mm-ddThh:mm:ss` biçimindedir
    ([Header](https://buildingsmart.fi/en_GB/infra/bSI_LandXML12_MVD/pages/1_Header.html)).
  - Epok için bir alan yoktur.
- **12d Field:** projenin tamamı sahaya götürülür. Saha dosyası kalite güvencesi için bir denetim izi tutar
  ([12d Surveying](https://www.12d.com/product/surveying.html)).
- **Leica Captivate:**
  - Tasarım noktaları ve çizgileri LandXML ya da ASCII listesinden işe kontrol noktası olarak alınır.
  - Infinity'nin Design to Field modülü verinin bütünlüğünü sahaya göndermeden önce denetler
    ([Hexagon blog](https://blog.hexagongeosystems.com/leica-captivate-education-series-ensuring-data-integrity-when-easily-sending-from-a-to-b/)).
  - Sürüm ya da özetten söz edilmez.
- **Türk uygulaması:**
  - TKGM'nin 2023/5 sayılı genelgesinin 3. maddesinden okunan tanımlar:
    - aplikasyon: sınırların tapu planına uygun olarak zeminde işaretlenmesi;
    - aplikasyon krokisi: sınırlar, sınır noktaları, ölçüleri ve röperleri
    ([genelge](https://www.lexpera.com.tr/mevzuat/genelgeler/tapu-ve-kadastro-genel-mudurlugunun-12-04-2023-tarihli-ve-2023-5-sayili-genelgesi-2023-5)).
  - Aplikasyonun esaslarını veren 17. maddeye erişilemedi (abonelik).
  - Aplikasyon kontrolü için sayısal bir sınır ADR 1700'de de bulunamamıştı. Sınırlar kullanıcınındır.
- **KentOS bugün:**
  - Cihaza gönder adlı noktaları cihazın biçiminde yazıyordu.
  - Uygulama kontrolü ölçülen kaydı tasarım özetiyle denetliyordu (`expectedDesign`). Ama bu özeti, revizyonu,
    kimlikleri, sistemi ve sınırları birlikte sahaya taşıyan bir dosya yoktu.

Okunan kaynakların hiçbiri sahaya giden veriye, dönen kaydın denetleyebileceği bir tasarım özeti koymuyor. Bu, ADR
1700'ün kararından gelen KentOS kararıdır.

## Karar

1. **`cad.field.package` v1**: yazar, ama yalnız dosyayı; çizim değişmez, geri alma adımı yoktur. Harita, Yol ve CBS'de;
   yüzey Harita ve Yol'da.
   - **Girdi:**
     - `design`: tasarım noktalarının katmanı; adları `field`'dadır, yoksa `Ad`;
     - `lines`: tasarım çizgilerinin katmanı; en az biri;
     - `surface`: yüzeyin kimliği ya da adı;
     - `horizontal`, `vertical`, `quality`: kabul sınırları (m);
     - `notes`: kalite notları; en çok 20, her biri en çok 500 harf;
     - `path`: mutlak bir `.zip` yolu; `overwrite`;
     - `stamp`: `YYYY-MM-DDThh:mm:ss`; yoksa şimdi.
   - **Tasarım** `cad.asbuilt.check`'in okuduğu gibidir. Noktalar katmanın noktalarıdır, çizim sırasıyla. Çizgiler
     katmanın çizgileri ve çoklu çizgileridir. Özet onun kuralıdır (SHA-256): sahada pakete göre ölçülen kayıt
     `expectedDesign` ile denetlenir. Eski tasarımla ölçülen kayıt, yeni tasarıma sessizce bağlanmaz.
2. **Paket** tek `.zip`, üç dosyadır; sıraları sabittir:
   - **`saha-paketi.json`** (`kentos.field-package` v1) şunları taşır:
     - proje adı, damga, çizimin revizyonu, tasarım özeti (`designDigest`);
     - sistem: EPSG kodu ve adı; yerel projede `null`;
     - epok ve kot sistemi (projenin sözleriyle); proje söylemiyorsa `null`, yani “söylenmedi”;
     - kabul sınırları: yalnız verilenler;
     - notlar;
     - yüzey: kimlik, ad, kaynaklarının özeti;
     - nokta ve çizgi sayıları ve dosyaların listesi.
   - **`noktalar.csv`**:
     - sütunlar `Ad;Y;X;Z;Kimlik`; Y doğu, X kuzey (Türk ölçmeciliğinin sırası);
     - her sayı geri okununca aynı çıkan en kısa yazımla, hiçbir şey yuvarlanmadan; kotu olmayanın Z'si boş;
     - Kimlik nesnenin kalıcı kimliğidir.
   - **`cizgiler.csv`**: sütunlar `Kimlik;Sıra;Y;X;Bulge`; her çizginin köşeleri sırasıyla (kapalı çoklu çizginin
     ilki sonunda yine).

   Dosya önce yanına geçici olarak yazılır, sonra adı değiştirilir. Böylece ya bütün yazılır ya hiç.
3. **Retler**, bu sırayla:
   - `invalid_field_package` (yeni): tasarım yok; 0'dan büyük olmayan sınır; boş öznitelik adı; 20'den çok not ya
     da 500 harften uzun not; tarih olmayan damga;
   - `layer_not_found` (`design`, `lines`);
   - `not_a_road_project` ve yüzeyin kendi retleri (`unknown_terrain`, `terrain_unbuildable`);
   - `invalid_file_path`: boş, göreli, `.zip` olmayan yol ya da olmayan klasör;
   - `file_exists`, `file_unwritable`.
4. **Masaüstü:** Uygulama kontrolü penceresinde “Saha paketi” bölümü vardır:
   - Kalite notları kutusu: her satır bir not; boş satırlar atılır.
   - **Saha paketi yaz…** düğmesi: dosyanın adı sorulur, uzantı yoksa `.zip` eklenir. Paket formun tasarımı (noktalar,
     çizgiler, yüzey, ad alanı) ve sınırlarıyla yazılır. Üzerine yazmayı kaydetme penceresi sorar.
   - İleti nokta, çizgi ve not sayısını söyler.

   Formun toleransları tek etiket sütununa alındı (“Tolerans yatay / düşey (m)”; satır kutusuyla başlar). Yeni şerit
   düğmesi yoktur; yeri Ölçme › Saha › Uygulama kontrolü'dür (Harita, Yol, CBS).
5. **Kapsam dışı:**
   - Cihaz biçimleri (JobXML, DBX, RAW) Cihaza gönder'in işidir.
   - Canlı cihaz ve makine kontrolü adaptörleri, gerçek donanım ve SDK kapısındadır.
   - Paket bir aplikasyon krokisi ya da resmî belge değildir; mevzuata uygunluk iddia etmez.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/field_package_cases.py`, 29 durum; KentOS kodu yok.
  - Uygulama kontrolünün çizimi kullanılır. Noktalar, kimlikler, çizgiler ve tasarım özeti onun kuralıyla elle
    yazılır; dosyanın baytları ve SHA-256'sı komutundur.
  - Durumlar:
    - noktalar, bordür ve yüzey; yalnız noktalar ve yalnız bordür;
    - sınırsız ve notsuz; yalnız yatay sınır; ad alanı;
    - plan, doğrulama; CBS ve Yol projesi;
    - epoku (2005.0) ve kot sistemi (TUDKA-99, TG-20) olan proje;
    - overwrite ile üzerine yazma;
    - 18 ret.
  - Kasıtlı bozulan bir kot sözü durumu düşürdü.

  Rust ve Python geçer.
- **Bağımsız okuyucu** (`python/tests/test_field_package.py`): paket Python'un kendi `zipfile`, `csv` ve `json`
  modülleriyle okunur. Denetlenenler:
  - üç dosya, sırasıyla;
  - noktaların adları, Y doğu / X kuzey, kotları çizimdekinin aynısı (yuvarlama yok), kimlikleri;
  - çizgilerin köşeleri;
  - özet, sistem, yalnız verilen sınırlar, notlar, yüzey; epok ve kot sistemi, proje söylediğinde;
  - cevabın boyutu ve SHA-256'sı dosyanınki;
  - overwrite'sız yazma reddedilir ve dosya olduğu gibi kalır.
- **Masaüstü** (`asbuilt_window_tests::the_field_package_is_written_from_the_form`):
  - formun sınırları ve iki satır not pakete girer, boş satır atılır;
  - özet Uygulama kontrolü durumunun `designDigest`'idir;
  - ileti “8 nokta, 2 çizgi, yüzey Tasarım, 2 not” der; çizimin revizyonu değişmez.

  Resimler `.run/gui-evidence/uygulama-kontrolu-1700/` altındadır.
- MCP testleri geçer: komut katalogdan araç olur, `Query` olarak işaretlidir.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("uygulama.kcad")
r = cad.field.package(doc, design="tasarim", lines="bordur", surface="Tasarım", horizontal=0.02, vertical=0.03,
                      notes=["Aplikasyona KN1 ve KN2'den başlanır."], path="/proje/saha/saha-paketi.zip")
print(r.digest, r.sha256, len(r.points))
# Sahadan dönen kayıt, paketin özetiyle denetlenir:
cad.asbuilt.check(doc, measured="olcum", design="tasarim", lines="bordur", expected_design=r.digest)
```

MCP: `{"tool": "cad.field.package", ...}`.

## Sonra

- İş türleri (as-built, aplikasyon, deformasyon) ve zaman serisi.
- Sahadan gelen kaydın paketle birlikte geri alınması: paketin özetiyle otomatik `expectedDesign`.
- Yetkili inceleme ve kabul edilen gerçekleşen revizyon kaydı.
- Varlık kimlikleri: altyapı ağı ve yol nesneleri.
- Cihaz biçimlerine paketten çıkış (Cihaza gönder ile).
- Mobil istemcinin aynı sözleşmeyi okuması.
