# ADR 0812: Komut aşamasına göre onay açıklaması

- **Durum:** ilk dilim uygulandı, 2026-11-01; CAD-ADV-01 [~].
- **Başlangıç:** main `4f1d11fde`.
- **Kapsam:** native masaüstünün komut menüsü ve mevcut etkileşim istemleri.
- **Bağımlılık / dosya şeması / API:** değişiklik yok.

## Sorun ve başvuru

Gerçek pencerede Taşı başlatılıp bir dikdörtgen seçildiğinde sağ tuş menüsü
“Onayla / bitir” gösteriyordu. Bu aşamada Enter yalnız seçimi tamamlıyor;
temel nokta aşamasındaki Enter ise yer değiştirme girişine geçiyor. Aynı
etiket farklı geçişleri bitirme gibi sunuyordu.

[QCAD Trim kaynak kodu](https://github.com/qcad/qcad/blob/master/scripts/Modify/Trim/Trim.js)
`setState` içinde sınır ve budanacak nesne aşamalarının istemlerini, sağ tuş
iptal/geri açıklamalarını ayırır; `getOperation` önizlemesiyle `applyOperation`
yazımı ayrı tutulur. Buradan alınan ilke aşamayı açık anlatmaktır; kod kopyalanmadı.
[AutoCAD MOVE belgesi](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-47CE7325-84C0-4414-80A3-29DC98392709.htm)
seçim, temel nokta, ikinci nokta ve yer değiştirmeyi ayırır. AutoCAD'in ikinci
noktada Enter ile ilk noktayı vektör sayması KentOS'un mevcut hedefte Enter
ile taşımadan çıkmasıyla aynı değildir. Bu dilim giriş matematiğini veya bu
mevcut davranışı değiştirmez; ürünler birebir aynı diye sunulmaz.

## Karar

- `drawing_menus::command_menu`, `Prompt.options` içindeki `Enter` seçeneğinin
  açıklamasını kullanır. Ayrı bir menü durum makinesi kurulmaz; Enter açıklaması
  bulunmayan araçlarda mevcut “Onayla / bitir” kalır.
- Ortak `Modify` seçiminde boş seçim **Bitir**, dolu seçim **Seçimi tamamla**.
- Taşı/Kopyala temel noktada **Koordinat farkı gir**; yer değiştirmede son
  vektör varsa **Son yer değiştirmeyi uygula**, yoksa **İşlem yapmadan bitir**.
  Taşı'nın hedef aşamasında **Taşımadan bitir**. Kopyala'nın mevcut **Bitir**
  eylemi ve dizi modundan kopya yerleştirmeye dönüşü görünür.
- Buda/Uzat sınır seçiminde boş seçim **Görünen tüm kenarları kullan**, dolu
  seçim **Seçilen sınırları kullan**. Çitin mevcut Enter **Uygula** açıklaması
  da artık menüde kullanılır.
- Seçenekler aynı `Prompt` üzerinden komut satırında da düğmedir; mevcut
  `tool.confirm` kullanılır. Geometri, grip, hızlı sağ tuş ayarı, önizleme ve
  belge komutlarının uygulama kodları değiştirilmez. Python/MCP için yeni veya
  paralel bir yazma yolu eklenmez.
- Yeni Enter açıklamaları eski kısa seçenek adlarını belirsiz yapmamalı:
  `yer` → Yer değiştirme ve `tüm` → Tüm kenarlar regresyonları test edilir.

## Doğrulama

- `cargo test -p kentos-interaction --test all --quiet`: 825 geçti, 2 isteğe bağlı atlandı.
- `cargo test -p kentos-desktop --lib drawing_menus::tests --quiet`: 7 geçti.
  Yeni test gerçek viewport seçim olaylarından başlayarak menü açıklamasını,
  tek `tool.confirm` ile aşama geçişini ve boş vektör/hedefte bitirmenin belge
  revision'ını değiştirmediğini denetler. Mevcut grip ve hızlı sağ tuş testleri korunur.
- `cargo test -p kentos-desktop --lib semantic_tool_tests --quiet`: 5 geçti,
  3 isteğe bağlı resim testi atlandı; çizim araçlarının anlamsal yazma/echo yolu.
- `cargo build -p kentos-desktop --bin kentos-cad`: başarılı.
- Gerçek macOS penceresi, yeni derleme, ayrı QA uygulaması ve kendi deneme çizimi:
  seçim → menüden seçim tamamlama → temel nokta; hedefte menüden taşımadan
  bitir; hedef tıklamasıyla taşı ve tek Undo; kopyala, menüden bir kez bitir
  (2 nesne kalır), tek Undo (1 nesne); Taşı'da temel noktadan sonra Esc;
  Buda'da sınırı seçip menüden kullan, dış parçayı buda ve tek Undo;
  dar pencerede son yer değiştirmeyi menüden bir kez uygula ve tek Undo.
- Ham görüntüler ve çizim Git dışında
  `/tmp/kentos-cad-advanced-2026-11-01/` altında. `01-before-move-menu.png`
  önceki etiket; `02`–`04` seçim/taşımadan bitir; `05`–`06` taşı/Undo;
  `07`–`09` kopya/bitir/Undo/Esc; `10`–`12` Buda/sınır/Undo;
  `13`–`15` yaklaşık 1100×680 mantıksal piksel pencerede menü/son fark;
  `16-saved-test.png` repo dışına kaydedilen çizim; `17`–`18` son derlemede
  boş sınır seçiminden görünen bütün kenarları tek onayla kullanma. Bunlar elle gerçek
  fare/klavye testleridir; otomatik resim üretimiyle karıştırılmaz.

## Açık kalanlar

CAD-ADV-01 bütünüyle kabul edilmedi. Dar pencerede komut satırının aşama metni
mevcut düzen nedeniyle kısalıyor; menü eylemleri sığıyor. Bütün araçlarda
geçersiz girdinin/onaysız aşamanın sunumu, odak ve belge değiştirme,
bağlı/kilitli kaynaklar, önizleme/sonuç eşitliği, Uzat/Köşe yuvarla/Tarama/
Blok ekle/Ölçü'nün gerçek pencere kabulü sonraki dilimlerdir.
