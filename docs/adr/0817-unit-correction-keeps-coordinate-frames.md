# ADR 0817: Birim düzeltmesinde yerel koordinat başlangıçları çizimle birlikte kalır

- **Durum:** uygulandı, 1 Kasım 2026. CAD-ADV-02'nin ilk dilimi; ana madde [~].
- **Başlangıç:** main `309c6ffc9`.
- **Kapsam:** `cad.units.change`, onun Undo durumu ve Proje ayarları taslağı.
- **Korunan:** metre tabanlı geometri, mevcut proje varsayılanları, çizim/pafta
  ölçeği, renderer, blok normalleştirme, API alanları ve komut sürümü.

## Araştırma ve kod incelemesi

- [AutoCAD blok birimleri ve yerleştirme ölçeği](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-6C46049D-8636-442D-8BAC-CF4FD515FDC0.htm)
  blok birimi ile kullanıcı yerleştirme çarpanını ayırır.
- [QCAD baskı kılavuzu](https://qcad.org/en/tutorials/printing/printing_and_pdf_export/)
  çizimin gerçek boyutunu kâğıda yerleştirme ölçeğinden ayırır.
- [AutoCAD UCS açıklaması](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-E658D5E7-EE5C-4A06-BF34-F71CDB363A71.htm)
  giriş eksenlerinin değişebildiğini, nesnelerin dünya koordinatlarında
  tutulduğunu açıklar. KentOS'un mevcut world/metre API kararı korunur;
  rakibin bütün komut sözdizimi aynen alınmaz.

İncelenen KentOS yolları: `units_change.rs`, domain `units.rs/history.rs`,
`project/unit_change.rs/settings.rs`, geometri çekirdeği `tools/ucs.rs`,
DXF `decide_units` ve `block_units::normalize`, `blocks_insert::unit_factor`,
clipboard komutları, Python birim/UCS/açma testleri ve GPU hassasiyet testleri.

Mevcut DXF normalleştirmesi tanımları kendi fiziksel birimine göre metreye
çevirip mevcut INSERT çarpanlarını telafi eder. Panoda model zaten metredir;
hedef çizim birimi ikinci bir dönüşüm nedeni değildir. Bu yolların yerine
yeni bir ölçek motoru veya sezgisel “ev santimetre olmalı” kararı konmadı.

## Tekrarlanan iki sorun

1. `geometry: scale`, nesneleri, blokları, ana görünüşü ve GPU başlangıcını
   dönüştürüyor; `settings.ucs.origin` ve `named_ucs` başlangıçlarını
   değiştirmiyordu. Metre tabanlı kaynak ve giriş eksenleri ayrılıyordu.
2. İlk düzeltme komut testinden geçmesine rağmen gerçek Proje ayarları
   akışında tekrar bozuldu. `save_project_settings`, birim komutundan sonra
   eski form taslağını kaydedip dönüştürülmüş UCS'yi geri eziyordu.

Her iki hata da düzeltmeden önce başarısız regresyonla kaydedildi. İkinci
hata, gerçek GUI tekrarının çekirdek testine ek gerekliliğini gösterdi.

## Karar ve uygulama

- `geometry: keep`: geometri ve bütün UCS başlangıçları aynı kalır; yalnız
  sayıları okuma/girme birimi değişir. 4000 mm uzunluk, 4 m okunur.
- `geometry: scale`: dünya sıfırına göre mevcut birim oranı UCS
  başlangıçlarına da uygulanır. Eksen yönleri, UCS adları ve kimlikleri
  değişmez. 4000 mm, kullanıcının açık seçimiyle 4000 m olur.
- `UnitState` geçerli/kayıtlı UCS'yi de taşır. Böylece birim, geometri,
  ana görünüş/GPU başlangıcı ve UCS aynı Undo/Redo adımındadır. Dosya
  şeması değişmez; UCS zaten proje ayarı olarak kaydedilir.
- Undo, birim adımının değiştirmediği UCS alanını geri yazmaz. Boyları
  koruyan birim değişiminden sonra kullanıcı başka UCS seçmişse, birim
  Undo'su bu bağımsız seçimi silmez.
- Plan ve uygulama aynı `next_state` hesabını kullanır. Dönüşüm geçerli
  veya kayıtlı UCS'yi dosyanın izin verdiği ±10.000 km sınırından çıkaracaksa
  mevcut `far_point` koduyla, **hiçbir şey yazmadan** reddedilir. Kullanıcıya
  boyları koruma veya UCS başlangıcını önce düzenleme yolu gösterilir.
- Proje ayarları taslağı başarılı komuttan sonra güncel UCS'yi alır;
  eski taslak dönüşümü geri alamaz. Diğer form alanlarının davranışı korunur.
- Onay sorusu UCS'nin de çizimle birlikte ölçekleneceğini açıklar. Reddedilen
  işlemde bile nesne/blok sayısı ve gerçek çarpan gösterilir; “0 nesne / ×1”
  gibi yanıltıcı başlangıç değerleri bırakılmaz.
- Komut kataloğu, mevcut TypeScript türünün açıklaması ve üretilmiş Python
  dokümanı güncellendi. Girdi/çıktı alanı eklenmedi. Python, MCP ve GUI aynı
  ürün komutunu kullanır. Başka ajanın katalog girdileri bu değişime katılmadı.

## Doğrulama

Gerçek macOS QA penceresi, yaklaşık 1100×680 mantıksal piksel:

1. Test çizimi: fiziksel 4×3m kapalı alan, gösterim mm, UCS dünya (10,20),
   açı 90°, aynı başlangıçlı kayıtlı UCS. Müşteri dosyası kullanılmadı.
2. Eski sürümde mm→m “Sayıları korunsun” sonrası köşenin yerel koordinatı
   yaklaşık (19980,−9990)m oluyordu. Kaynak dosya değiştirilmedi.
3. Son sürümde aynı düzeltme ve Redo sonrası köşeye **Uç nokta** kenetleme:
   yerel koordinat tam (0,0)m. Tek Undo sonrası aynı köşe (0,0)mm ve fiziksel
   4×3m. Bu kabulde yuvarlanmış serbest fare konumu kesin koordinat sayılmaz.
4. “Boyları korunsun” ile mm→m: alan **12,00m²**, çevre **14,000m**.
   Yeni ölçek veya birim tahmini uygulanmıyor.
5. Başlangıcı 100.000m olan UCS'yi ×1000 büyütme: açıklama bütünüyle okunur,
   **1 nesne / ×1000** doğru, “Sayıları korunsun” pasif; “Boyları korunsun”
   kullanılabilir. Reddedilen komut belgeye yazmıyor.

Otomatik kontroller:

- Native uygulama entegrasyonu: **270 geçti, 37 isteğe bağlı atlandı**.
  Yeni kontroller: planın yazmaması, keep/scale ile geçerli/kayıtlı UCS,
  yönlerin korunması, tek Undo/Redo, geçerli veya kayıtlı UCS'nin sınır
  dışı kalmasının atomik reddi ve keep sonrası bağımsız UCS seçimi.
- Masaüstü: **1560 geçti, 256 isteğe bağlı atlandı**. Yeni uçtan uca test,
  formun Save→UnitAnswer→Save yolunun doğru UCS'yi koruduğunu ve tek Undo'yu
  doğrular. `TMPDIR=/tmp` mevcut Unix socket yol sınırı nedeniyle kullanıldı.
- Güncel native Python modülüyle **3 UCS + 10 birim + 2 CAD açma testi**
  geçti. Yeni Python senaryosu plan/uygulama, 4→4000m ölçü, KCAD byte
  kaydet–aç, geçerli/kayıtlı UCS ve tek Undo/Redo'yu birlikte doğrular.
  Var olan birim/açma testleri DXF mm/inç, DWG ve NCZ yollarını da çalıştırır.
- DXF karma blok birimleri: **3 test geçti**; çoklu kaynak birimi, iç içe
  blok, kullanıcı dosya birimi seçimi ve patlatılmış sonuç karşılaştırması.
- CPU/GPU sayısal hassasiyet karşılaştırması: **7 test geçti**.
- `KENTOS_GPU_TESTS=1` ile gerçek **Apple M5 / Metal**: `on_a_real_gpu`
  geçti; TM koordinatında alt piksel kaydırma, mevcut çizim pipeline'ları,
  önbellek, MSAA, HiDPI, yoğun tarama ve kalem genişliği kontrolleri çalıştı.
  Ortam değişkeni olmadan erken dönen ilk deneme GPU kabulü sayılmadı.
- Derleme ve Clippy başarılı; mevcut `ts-rs` serde ayrıştırma uyarıları
  devam ediyor. Python SDK üretim kontrolü başarılı; yeni TS dosyası yok.

Kanıtlar repo dışında `/tmp/kentos-cad-advanced-02-2026-11-01/` altındadır.
`02` ilk hatanın, `04` formun eski UCS'yi geri yazmasının kanıtıdır;
başarılı sonuç diye sayılmaz. `08`–`11` son GUI kabulüdür. `05`–`06`
serbest imleç gözlemidir; `07` yalnız adlı UCS aracını açma denemesidir,
kayıtlı değerlerin GUI kabulü değildir. Kayıtlı değerler Rust/Python'da
kesin değerlerle doğrulanır. Manifest görüntü, günlük ve ikili özetini tutar.

## Kalan CAD-ADV-02 kabulü

Bu dilim birim sisteminin bütünü tamamlandı anlamına gelmez. Ana madde
[~] kalır: aynı 1m/4×3m örneğinin açma–ekleme–pano–blok yollarında birleşik
GUI matrisi; birimsiz/çelişkili dosyanın karar ve önizleme akışı; dinamik
blok/ilişkili dizi/ölçü tarifleri ve paftanın birim düzeltmesi sonrası
birlikte kabulü; döndürülmüş kamera/UCS ve büyük koordinatta aynı dosyanın
kaydet–aç tekrarı tamamlanacaktır. Önceden var olan testler, bu birleşik
kabul yapılmış gibi gösterilmez.
