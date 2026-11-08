# 2420 — Sahadan dönen kaydın kendi saha paketiyle denetlenmesi

Tarih: 2026-11-08. plan/map-cad.md MAP-19, üçüncü tur. Durum: taslak. Plan satırları:
- “Tasarımdan sahaya çıkacak paketi açık tanımla: seçilen revizyon, … CRS/epok, kot referansı, kabul eşiği”;
- kabul: “Eski tasarım revizyonuyla ölçülen kayıt daha sonra yeni tasarıma sessiz bağlanmasın.”

İlgili:
- [ADR 1700](1700-asbuilt-check.md), Uygulama kontrolü (`cad.asbuilt.check`);
- [ADR 1920](1920-field-package.md), saha paketi (`cad.field.package`). “Sonra”sındaki “sahadan gelen kaydın paketle
  birlikte geri alınması: paketin özetiyle otomatik `expectedDesign`” maddesini bu ADR kapatır.

## Dayanak

- **Trimble Business Center, Import Esri File Geodatabases**
  ([belge](https://help.fieldsystems.trimble.com/tbc/18740.htm), 8 Kasım 2026'da okundu):
  - projenin koordinat sistemi dosyanınkiyle aynı olmalıdır; değilse kullanıcı bir çözüm seçer;
  - üç seçenek: projenin sistemini dosyanınkine çevirmek, iki sistemi tutup koordinatları dönüştürmek, ikisini aynı
    saymak.
- **Cansel, TBC verimliliği** ([yazı](https://www.cansel.ca/store/cansel/en/true-north-spring-2018-tbc-efficiency)):
  Trimble Access'in `.job` ve `.jxl` dosyaları kendi koordinat sistemi ayarlarını taşır. Boş bir projeye içe
  aktarılınca sistem farkı uyarısı çıkar.
- **KentOS bugün:**
  - saha paketi (`saha-paketi.zip`) tasarım özetini, kabul sınırlarını, sistemi, epoku ve kot sistemini taşıyordu;
  - Uygulama kontrolü bunları ancak elle girilirse kullanıyordu: özeti `expectedDesign` olarak, sınırları ayrı ayrı.
    Paketin sistemi hiç karşılaştırılmıyordu.

## Karar

1. **`package` girdisi** (`cad.asbuilt.check`, isteğe bağlı): kaydın yapıldığı saha paketinin `.zip`'i (mutlak yol).
   Kontrol paketten yalnız `saha-paketi.json`'ı okur (`kentos.field-package` 1).
2. **Tasarım özeti:** paketin `designDigest`'i beklenen özettir. Tasarım değiştiyse `designChanged` söylenir,
   karşılaştırma yine yapılır (ADR 1700 §5). `expectedDesign` de verilmiş ve paketinkinden farklıysa ret.
3. **Kabul sınırları:** paketin yatay ve düşey sınırları, girdi vermediyse uygulanır; girdininki önce gelir. Kalite
   sınırı yalnız σ'nın özniteliği (`qualityField`) verilmişse uygulanır. Cevaptaki `taken` hangilerinin paketten
   alındığını söyler.
4. **Sistem:** paketin EPSG kodu, epoku ve kot sistemi projeninkiyle karşılaştırılır (`sameSystem`, `sameEpoch`,
   `sameHeights`).
   - KentOS koordinat dönüştürmez. Ölçülen noktalar zaten çizimdedir.
   - Fark yalnız söylenir; TBC'deki gibi çözümü kullanıcı seçer.
5. **Cevapta `package`:** yol, tarih, revizyon, özet, sistem, epok, kot sistemi, üç karşılaştırma, sınırlar ve `taken`.
6. **Retler** (sorunun kurallarından sonra, katmanlardan önce; yerleri `package`):
   - `invalid_file_path`: boş, göreli ya da `.zip` olmayan yol;
   - `file_not_found`;
   - `invalid_asbuilt_check`: zip açılmıyor, `saha-paketi.json` yok, biçimi ya da sürümü başka, özeti yok;
   - `invalid_asbuilt_check` (`expectedDesign`): beklenen özet paketinkinden farklı.
7. **Masaüstü** (Uygulama kontrolü):
   - **Kaydın saha paketi** satırı: paketin adı, **Paketi seç…** ve **Paketi kaldır**;
   - özette paketin tarihi, revizyonu ve paketten alınan sınırlar;
   - farklı sistem, epok ya da kot sistemi için birer uyarı.
8. **Kapsam işareti:** ADR 1700'deki gibi **Harita + Yol + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/asbuilt_cases.py`, KentOS kodu yok.
  - Paketler `fixtures/asbuilt/v1/files/` altında Python'un `zipfile`'ıyla yazılır: sıkıştırmasız, sabit tarihli;
    bayt bayt yeniden üretilir.
  - Üç paket:
    - `saha-paketi.zip`: bu tasarımın özeti, EPSG:5256, sınırlar 0,02 / 0,03 / 0,05 m;
    - `eski-paket.zip`: başka özet, EPSG:2320, epok 2005,0, ortometrik kot, yatay sınır 0,06 m;
    - `paket-degil.zip`: içinde yalnız bir not.
  - Yeni durumlar:
    - sınırları paketten (cevap tam durumunkiyle aynı, artı `package`);
    - girdinin sınırı önce gelir;
    - σ özniteliği olmadan kalite sınırı alınmaz;
    - eski paket: tasarım değişti, sistem, epok ve kot farklı;
    - aynı beklenen özetle;
    - 5 ret.
  - 36 durum (26'dan). Eski durumlar aynıdır.
  - Sınır, bir noktanın tam uzaklığına denk gelmeyecek biçimde seçildi. 0,05 m'de P2'nin uzaklığı büyük koordinatta
    Rust'ta 0,0499999998, Python'da 0,0500000000000007 çıkıyordu.
  - Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/asbuilt_package.rs`):
  - durumların çizimi `.kcad` olarak açılır, paketler yollarıyla verilir;
  - iki durumun sayıları ve paket bilgisi durumlarınkidir;
  - paket olmayan zip `saha-paketi.json` diyerek reddedilir.
- **Masaüstü** (`asbuilt_window_tests::the_record_s_package_gives_the_digest_and_the_limits`):
  - paket seçilince durumun sayıları ve `taken`;
  - eski paket tasarım değişimini ve üç farkı söyler;
  - Paketi kaldır paketi çıkarır.

  Çapraz masaüstü denetimleri 137 geçer. Resimler `.run/gui-evidence/saha-paketi-donus-2420/`.

## Sonra

- İş türleri (as-built, aplikasyon, deformasyon) ve zaman serisi.
- Paketin tasarım noktalarıyla (eski tasarım) karşılaştırma seçeneği: bugün tasarım çizimden okunur, değişim söylenir.
- Yetkili inceleme ve kabul edilen gerçekleşen revizyon kaydı.
- Cihaz biçimlerine paketten çıkış; canlı cihaz.
