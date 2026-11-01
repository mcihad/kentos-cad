# ADR 0814: Tarama değer girişi ve yerleştirmeden dönme

- **Durum:** uygulandı, 2026-11-01; CAD-ADV-01 [~].
- **Başlangıç:** main `21ca1cf30`; ADR 0812 ve 0813'ün devamı.
- **Kapsam:** native Tarama değer aşaması; Ölçü ve Blok ekle Enter açıklaması.
- **Dosya şeması / bağımlılık / Python–MCP API:** değişiklik yok.

## Gerçek pencerede bulunan sorun

Tarama → Ölçek seçildiğinde sayı beklenirken kapalı bölgeye tıklamak
değer girişini sessizce bırakıyor, eski ölçekle yeni tarama ekliyordu.
Deneme çizimi 2 nesneden 3'e çıktı. `Hatch::pointer_down`, yalnız başlangıç
noktası/özellik alma tıklamasını işledikten sonra diğer bütün `asking`
durumlarını sıfırlayıp `fill` çağırıyordu. `pointer_move` da değer aşamasında
tarama önizlemesi hesaplayabiliyordu.

Aynı incelemede Ölçü yerleşimindeki sağ tuş “Onayla / bitir” eyleminin
ölçüyü çizmeden ilk noktaya döndüğü doğrulandı. Blok ekle Enter'ı ise
bekleyen görüntüyü yerleştirmeden araçtan çıkar. Genel etiket bu farkları
anlatmıyordu. Geçerli bu davranışları değiştirmek bu dilimin amacı değildir.

## İnternet dayanağı ve KentOS kararı

- [AutoCAD HATCH](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-27C104F2-B687-4025-B50B-A58E37329832.htm)
  bölge içi nokta, sınır nesnesi seçimi ve ayarları ayrı girdiler olarak sunar.
  KentOS'ta sayısal ayarın cevabı bölge seçimi sayılmamalı. AutoCAD'in her
  tarama yazma adımının KentOS ile aynı olduğu iddia edilmez.
- [QCAD Insert Block](https://qcad.org/en/manual/scripts/block/insertblock/doc/insertblock/)
  blok seçimini, dönüş/ölçek ayarını ve fare veya konsoldan konumu ayırır.
  KentOS mevcut boyut/birim etiketi ve ayar panelini kullanmaya devam eder.
- [BricsCAD DIMALIGNED](https://help.bricsys.com/en-us/document/command-reference/d/dimaligned-command)
  iki ölçülen nokta ile ölçü çizgisi yerini ayrı adımlar olarak tanımlar.
  KentOS'un yerleşimde Enter ile sıfırlaması burada değiştirilmez; kullanıcı
  çizim yapılacağını sanmasın diye mevcut sonuç açık adlandırılır.

## Uygulama

- Tarama `Asking::Scale/Angle/Colour/Gap/Transparency` açıkken sahne
  tıklaması belgeye yazmaz, aşamayı bırakmaz; komut satırından değer
  girilmesini veya Enter/Esc ile dönülmesini anlatır. Başlangıç ve Özellik al
  tıklaması önceki ortak `options::clicked` yolunda işlemeye devam eder.
- Değer veya kaynak beklenirken dolacak bölge önizlemesi üretilmez.
  Normal bölge seçimi ve `entities_hatch::execute` yolu korunur.
- Tarama Enter açıklamaları: değer için **Değiştirmeden dön**, başlangıç
  için **Nokta seçmeden dön**, kaynak için **Özellik almadan dön**.
- Ölçü `placing()` aşamasında **Ölçü çizmeden başa dön**; Blok ekle'de
  **Yerleştirmeden bitir**. Mevcut `Prompt` seçenekleri komut satırı ve
  sağ tuş menüsünün ortak verisidir; ikinci durum makinesi kurulmaz.
- Hiçbir yeni hesap/yazma yolu yok. Uygulanan tarama, blok ve ölçü yine
  ortak ürün komutlarına gider. Python çıktısı/API/MCP şeması değiştirilmedi.

## Doğrulama

- `cargo test -p kentos-interaction --test all --quiet`: **830 geçti,
  2 isteğe bağlı atlandı**. Yeni regresyon; beş değer türünde geçersiz giriş,
  önizleme yokluğu, tıklamada aynı revision/nesne sayısı/aşama, tek Enter ile
  bölge seçimine dönüş, normal tarama ve tek Undo'yu denetler. Ek testler
  Esc'de sıfır yazım/geçmiş, geçerli değerle devam, meşru başlangıç noktası
  tıklaması, ölçüyü yazmadan sıfırlama ve blok önizlemesini ek kopyasız bitirmedir.
  İlk regresyon düzeltmeden önce başarısız oldu; GUI'de de hatalı yazım görüldü.
- `cargo test -p kentos-desktop --lib drawing_menus::tests --quiet`:
  **8 geçti**. `semantic_tool_tests`: **5 geçti, 3 isteğe bağlı atlandı**.
  `cargo build -p kentos-desktop --bin kentos-cad`: başarılı.
- Gerçek macOS QA uygulamasında kendi çizimi, yaklaşık 1100×680 mantıksal
  piksel pencere: sıfır ölçek uyarısı, sayı beklerken tıklamada 2 nesnenin
  korunması, tek menü eylemiyle geri dönüş; normal taramada aynı sınır/desen
  önizlemesi ve uygulama, 3 → 2 tek Undo, değer aşamasında Esc.
- Hizalı ölçüde iki uç → farede konum → uygulama: önizleme ve son değer
  15.061; çizgi/uzatma konumu korunuyor. Önizlemenin imleç yanındaki etiketi
  nihai kâğıt boyutlu ölçü yazısıyla aynı ekran boyunda sayılmaz. Sonra
  yeni menüyle tek tıkta 2 nesneyi koruyarak başa dönüş, klavyeden 2 m
  uzaklıkla ölçü yerleştirme ve tek Undo doğrulandı.
- Blok için repo dışına kopyalanan `fixtures/interaction/v1/blocks.kcad`:
  Rögar önizlemesi ve 1.500 × 1.500 m boyut etiketi; tıklayarak yerleştirme,
  menüden tek bitirme (13 nesne, 4 Rögar), tek Undo (12 nesne, 3 Rögar),
  bekleyen ayardan Esc ile çıkış. Karmaşık dinamik/öznitelikli blok kabulü
  bu basit blok denemesiyle tamamlanmış sayılmaz.
- Son derleme ayrıca başlangıç noktası aşamasında açıldı: altı Başlangıç
  seçimiyle nokta istemi, meşru tıklamadan sonra hâlâ 2 nesne; Özellik al'da
  doğru menü açıklaması ve tek eylemle yazmadan geri dönüş doğrulandı.

Ham GUI kanıtı ve günlükler Git dışında
`/tmp/kentos-cad-advanced-2026-11-01/part-3/` altında:
`01`–`02` hata; `03`–`05` ilk ölçü incelemesi; `06`–`11` güvenli tarama;
`12`–`14` açık ölçü menüsü ve klavye yerleşimi; `15`–`19` blok/bitir/Undo/Esc;
`20`–`22` son derlemede meşru nokta ve özellik alma aşamaları.
Kaynak fixture yerine repo dışındaki kopyası kullanıldı.

## Açık kabul

CAD-ADV-01 bütünü tamamlanmadı: diğer araç/aşama açıklamaları, bütün ölçü
kipleri, öznitelik/dinamik bloklar, karmaşık tarama sınırları; odak ve belge
değişimi, kilitli/bağlı kaynak etkisi, bütün önizleme/sonuç matrisi açık.
Bu dilim renderer, birim motoru veya ölçek varsayılanını değiştirmez.
