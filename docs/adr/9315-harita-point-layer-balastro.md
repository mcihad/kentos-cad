# ADR 9315: Yeni Harita projesinin Nokta katmanı ve balastro sembolü

- **Durum:** kabul edildi (2026-11-26). Sahibin sözleri (26 Kasım 2026):
  - “yeni bir standart harita cad projesi açtığımızda bir de nokta katmanı gelsin standart olarak ve bu noktanın
    sembolojisi balastro olsun, semboloji kütüphanesinde yoksa ekle”;
  - araştırma kısa tutuldu: “stil ayarlarından sembolojiyi kişi kendisi ayarlamayı öğrenir … sadece sen şeklini yap
    yeterli”.
- **İlgili:**
  - [ADR 0138](0138-netcad-ncz-and-progressive-import.md) (katman ağacının üstü üstte çizilir);
  - [ADR 0165](0165-project-types-cad-gis.md) §3 (yeni projenin katmanları);
  - [ADR 9118](9118-general-symbol-library.md) (Genel kütüphane);
  - [ADR 9311](9311-arial-times-point-names-labels-off.md) (etiketler kapalı başlar);
  - [ADR 9314](9314-point-names-follow-their-points.md) (nokta adı yazıdır).

## Araştırma (26 Kasım 2026'da okundu)

- **BÖHHBÜY:** “Kadastral amaçlı ölçülerde parsellerin köşe ve kırık noktalarına balastro veya bilgisayar destekli
  çizim sistemi ile 0.75 mm çapında küçük daireler çizilir”
  ([Büyük Ölçekli Harita ve Harita Bilgileri Üretim Yönetmeliği](https://www.harita.gov.tr/uploads/files/regulations/buyuk-olcekli-harita-ve-harita-bilgileri-uretim-yonetmeligi-65.pdf)).
  Balastro adı bu daireleri çizen küçük pergelden gelir (MEB, Harita Çizim Uygulamaları modülü).
- **Netcad:** balastro çizgisel objelerin baş ve son noktalarına konan yuvarlak semboldür; çizgiler sembolün içinde
  görünmez, uçlarından kırpılır. Nokta, doğru, yay ve çoklu doğruda otomatik uygulanır; yarıçapı Görünüm Ayarları'ndan
  verilir. “Türkiye harita standartlarına ait bir gösterim” olarak anılır
  ([Görünüm Ayarları › Balastrolar](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=217385043)).
- **Okunamayan:** yarıçapın birimi (kâğıt milimetresi mi, ekran pikseli mi) bulunan kaynaklarda yazmıyor. Sahip araştırmanın
  burada kesilmesini istedi: boyutu ve görünüşü kişi stil ayarlarından değiştirir.

## Karar

### 1. Balastro sembolü

- **Yeri:** Genel kütüphane, “Genel › Noktalar › Ölçme”, kimliği `genel.nokta.olcme.balastro`
  (`scripts/style/general_library.py`).
- **Neden Genel kütüphane:** BÖHHBÜY kütüphanesi Ek-3'ün numaralı 277 işaretidir ve sınaması yalnız onları kabul eder.
  Balastronun numarası yoktur; kuralı yönetmeliğin kadastral çizim maddesindedir.
- **Şekli:** 0,75 mm çaplı daire (kâğıt milimetresi, çizim ölçeğinde), içi kâğıt rengiyle (`paper`) dolu, 0,1 mm
  mürekkep çizgili.
  - Kâğıt rengi dolgu, noktaya gelen çizgilerin daire içinde görünmemesini sağlar. Nokta katmanı çizginin üstünde
    çizildiğinde Netcad'in kırpmasıyla aynı görünür.
  - Çizgi kalınlığını yönetmelik söylemez; 0,1 mm verildi.
- **Değiştirmek:** boyut, renk ve çizgi kalınlığı katmanın stil ayarlarından değiştirilir. Sembol tasarımcısında kopyası
  düzenlenir.

### 2. Yeni Harita projesinin Nokta katmanı

- **Katman:** yeni Harita CAD projesi (`CadWork::Map`) çizim katmanlarının üstünde “Nokta” katmanıyla başlar
  (`new_project::cad_layers_for`, `point_layer`). Ağacın en üstündedir: en son çizilir, Çizim'in çizgilerini örter
  (ADR 0138).
- **Sembolojisi:** tek sembollü çizici (`{"type": "single", "symbols": {"marker": {"ref": "genel.nokta.olcme.balastro"}}}`).
- **Etiketleri** kapalıdır (ADR 9311); nokta adları yazıdır (ADR 9314).
- **Etkin katman:** yine Çizim'dir. Nokta katmanını kişi seçer.

### 3. Modüller

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Harita | var | Sahibin isteği; ölçme ve kadastro noktaları Harita'nındır |
| Yol | yok | Yol projesinin katmanları kendi işinden gelir; balastro sembolü kütüphanede herkese açıktır |
| Mimari | yok | Binanın çiziminde ölçme noktası katmanı yoktur |
| CBS | yok | CBS projesi boş başlar, katmanları kaynaklardan eklenir (ADR 9302); sembol kütüphanede vardır |

### 4. Kanallar

| Kanal | Var / yok | Nasıl |
|---|---|---|
| Python | var | `cad.Document.new(..., cad_work="map")` aynı şablonla Nokta katmanını kurar |
| MCP | var | Yeni proje isteği aynı şablondan |
| Yapay zekâ ajanı | var | MCP ile aynı |
| Eklenti | var | Yeni proje aynı şablondan; sembol kütüphaneden `ref` ile kullanılır |

## Doğrulama

26 Kasım 2026'da koşuldu:

- `cargo test -p kentos-project`: yeni Harita projesi Nokta katmanıyla başlar. Çizicisi balastroyu gösterir,
  etiketleri kapalıdır, etkin katman Çizim'dir; Mimari ve Yol Nokta katmansız başlar
  (`a_map_project_starts_with_a_point_layer_of_balastros`).
- `cargo test -p kentos-native-style`: Genel kütüphanede `genel.nokta.olcme.balastro` vardır.
- `python3 scripts/style/general_library.py --check`: kütüphane dosyası günceldir (341 sembol).
- **Resim:** `.run/gui-evidence/kullanim-9315/nokta-katmani-balastro.png` (`balastro_screens`). Nokta katmanında
  üç adlı nokta, Çizim'de onları birleştiren üçgen var. Çizgiler balastroların içinde görünmüyor; adlar noktanın sağ
  üstünde yazı. Büyütülerek bakıldı.
