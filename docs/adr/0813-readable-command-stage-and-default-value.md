# ADR 0813: Okunabilir komut aşaması ve açık varsayılan değer

- **Durum:** uygulandı, 2026-11-01; CAD-ADV-01 bütünü [~].
- **Başlangıç:** main `0b222c490`; ADR 0812'nin devamı.
- **Kapsam:** komut satırı yerleşimi, mevcut komut menüsünün açıklaması.
- **Bağımlılık / dosya şeması / Python–MCP komutları:** değişiklik yok.

## Gözlem ve dayanak

Gerçek dar masaüstü penceresinde Taşı'nın temel nokta istemi seçenekler
yanında “1 nesne için te…” oluyordu. Komut satırı sabit bir asgari metin
genişliği ayırıyordu. Ayrıca `Elided.reserve`, Iced Row'un çocuklarını
yerleştirmeden önce çıkardığı boşlukları bir kez daha çıkarıyordu.

[AutoCAD Dynamic Input belgesi](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-3EBD4C17-F0A5-49FA-B131-4AABE2E727DB.htm)
beklenen girdinin ve seçeneklerin erişilebilir sunumunu örnekler. Bu karar
tam bir Dynamic Input sistemi eklemez; mevcut satırı okunabilir yapar.
[QCAD Round belgesi](https://qcad.org/en/manual/scripts/pro/modify/roundpro/doc/roundpro/)
yarıçapın araç seçeneklerinde açık girilmesini gösterir.
[AutoCAD Fillet belgesi](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-696681C1-4B8B-4995-8903-CCE6686179EA.htm)
de yarıçapı ayrı seçenek olarak tanımlar. Rakip geometri kodu kopyalanmadı.

## Karar

- Aşama metni için ortak tipografinin ölçtüğü genişlik, satırın yarısına
  kadar korunur. Mevcut asgari giriş alanı ve sabit durum seçenekleri korunur.
  Sığmayan ikincil seçenekler mevcut Diğer menüsüne gider; klavye eşlemeleri kalır.
- Satır boşlukları yalnız bir kez ayrılır. Sabit menüler veya sabitlenmiş
  seçenekler tek başına sığmıyorsa boş bir Diğer menüsü üretilmez.
- Yine kısalan açıklama mevcut `Tip.when_cut` ile tam okunabilir; kısa
  metin için gereksiz ipucu çıkmaz. Mevcut gecikme ve ekran sınırlandırma kullanılır.
- Sağ tuş komut menüsü `Prompt` içindeki Enter açıklamasına mevcut `value`
  bilgisini de ekler: **Son yarıçap: 1.000 m**. Ayrı durum/varsayılan değer
  hesabı eklenmez; tek `tool.confirm` yolu kullanılır.
- Geometri ve araç aşaması kodu değiştirilmedi. Python/MCP'nin kullandığı
  anlamsal komutlar aynıdır; ikinci bir yazma veya hesap yolu eklenmedi.

## Doğrulama

- `cargo test -p kentos-ui --lib --features snapshot --quiet`: **321 geçti**.
  Yeni test gerçek komut satırını 790 mantıksal pikselde yazılım renderer'ı
  ile yerleştirir; komut etiketi, ikonlu Nokta hesabı menüsü, giriş ve sağ
  kontroller varken temel nokta isteminin kesilmediğini denetler. İkonlu
  örnek boşluk düzeltmesinden önce başarısız, sonra başarılı oldu. Sabit
  seçeneklerden boş taşma menüsü üretilmediği ayrıca test edilir.
- `cargo test -p kentos-desktop --lib drawing_menus::tests --quiet`:
  **8 geçti**. Yeni test gerçek viewport olaylarıyla düz kenarlı fixture
  köşesini seçer; menüde son yarıçapı, menü açılırken sıfır revision değişimini,
  tek onayla geometri değişimini ve tek Undo ile eski geometrinin dönüşünü sınar.
- `cargo build -p kentos-desktop --bin kentos-cad`: başarılı.
- Gerçek macOS uygulamasında yaklaşık 1100×680 mantıksal piksel pencere:
  Taşı istemi tam okunuyor; Diğer → Koordinat farkı gir doğru aşamaya geçiyor
  ve çizimi değiştirmiyor. Uzat'ta sınır seç → sağ tuş → Seçilen sınırları
  kullan bir tıkta hedef aşamasına geçiyor. Kısa çizgi sınıra uzuyor; tek
  Undo eski ucu getiriyor. Sınır seçiminde Esc sonrası geçmişte ek işlem yok:
  sonraki Undo yalnız test çizgisini kaldırıyor.
- Köşe yuvarla gerçek pencerede: klavyeyle 1 m yarıçap, ikinci köşede sağ
  tuştan Son yarıçap: 1.000 m ile tek uygulama, tek Undo ile yalnız ikinci
  köşenin dönüşü; üçüncü köşede Esc ve tek Undo ile ilk köşenin dönüşü.
  Faredeki anlık farklı yarıçap önizlemesi son yarıçap kısayolunun sonucu
  olarak sunulmaz; bu ayrım ileri önizleme incelemesinde açık kalır.

Ham görüntüler, test çizimi ve günlükler Git dışında
`/tmp/kentos-cad-advanced-2026-11-01/part-2/` altındadır.
`01` eski dar satır; `02` ara deneme (son kabul değildir); `03` tam ipucu;
`04`–`09` yarıçap/menü/tek uygulama/Undo/Esc; **`10` son derlemede tam istem**;
`11`–`12` taşan Enter seçeneği ve aşaması; `13`–`18` Uzat sınır/uygulama/Undo/Esc.
İlk yuvarlama kabulü aynı menü kodlu ara derlemede, son dar satır ve Uzat
kabulü boşluk düzeltmesini içeren son derlemede yapıldı. Görüntüler gerçek
fare/klavye kullanımından alındı; repo içine görüntü eklenmedi.

## Açık kapsam

Tarama, Blok ekle ve Ölçü'nün gerçek pencere kabulü; tüm araçlarda geçersiz
girdi, kilitli/bağlı nesne etkisi, odak/belge değişimi ve önizleme/sonuç
eşitliği henüz tamamlanmadı. CAD-ADV-01 bu dilimle bütünüyle işaretlenmez.
