# Tek uzunluk alanında gösterilen birimle düzenleme

25 Ekim 2026 · ARCH-UX-01 takip bulgusu · [ADR 0661](../../adr/0661-formatted-single-length-fields.md).

## Uygulanan sınır

Gösterilen `0.200 m` metnini `0.300 m` olarak düzenlemek artık geçerlidir.
Ortak tek uzunluk okuyucusu boşluklu metrik birim ve ondalık `in`/`ft` kabul eder.
Koordinat/iki uzunluk söz dizimi, fiziksel SI model, import, çizim ölçeği,
yazı boyu ve renderer değiştirilmedi. Yeni komut veya veri sözleşmesi yoktur.

## Gerçek pencere kabulü

Ayrı QA uygulaması ve ayrı config/data klasöründe sentetik 100013 duvarlı
KCAD kopyası kullanıldı. macOS gerçek fare/klavye; 2880×1720 piksel, açık tema,
18 px. Çalıştırılan debug binary SHA-256:
`66294cd509c53147a8b3b7f20d279c447ed9a686235d71efd58aeb3a89f0569e`.
Bu bir hız ölçümü veya tüm mimari planın kabulü değildir.

| Ekran | İşlem ve gözlenen sonuç |
|---|---|
| [01](01-open.png) | Başlangıç dosyası, 100013 nesne. |
| [02](02-spaced-value-edit.png), [03](03-spaced-value-applied.png) | Dikdörtgen duvarın ilk köşesi kondu; alan açıkça odaklanıp `0.300 m` yazıldı. Enter sonrası 0.300 m, aynı ilk köşe, önizleme ve nesne sayısı. |
| [04](04-foot-value-applied.png) | `1 ft` Enter; üç ondalıkta 0.305 m görünür. Gerçek 0.3048 m native testiyle denetlenir. |
| [05](05-centimetres-blur.png) | `25 cm` yazılıp Kalınlık etiketine tıklandı; 0.250 m, aynı taslak. |
| [06](06-escape-keeps-width-and-corner.png) | `700 mm` düzenlemesi Escape ile iptal; önceki 0.250 m ve ilk köşe kaldı. |
| [07](07-post-escape-input-observation.png), [08](08-recovered-after-focus-test.png) | Deneme hatası: Escape sonrasında alan yeniden odaklanmadan yazıldı. Genel kısayol/komut alanına giden giriş bir nesne üretti. Bu geçersiz alan testi sayılmadı; iptal ve tek undo ile 100013 nesneye dönüldü. |
| [09](09-invalid-length-rejected.png) | Araç yeniden başlatıldı, ilk köşe kondu ve Kalınlık alanı açıkça odaklandı. `1 2 m` Enter ile reddedildi: 0.250 m, aynı taslak, 100013 nesne. |
| [10](10-created-wall.png) | İkinci köşe ile bir normal kapalı duvar oluştu: 100014 nesne. |
| [11](11-one-undo.png) | Tek undo yeni duvarı kaldırdı: 100013. |
| [12](12-redo-saved.png), [13](13-save-complete.png) | Tek redo: 100014; kaydetme tamamlandıktan sonra kirli işareti kalktı. |

Kaydedilen dosya uygulamadan bağımsız `tools/kcad/kcad.py` ile okundu:
100013 eski UID ve bütün nesne alanları birebir aynı; eklenen tek nesne dört
köşeli, kapalı, **0.25 m** kalınlıkta normal duvar. Birim m, çizim ölçeği 1:100.
[Makine tarafından doğrulanan kayıt](saved-file-check.json) geometri ve dosya hash'ini içerir.
Kaynak büyük test dosyaları `.run` altında tutuldu; kullanıcı projelerine yazılmadı.

## Otomatik kontroller

- Yeni ortak okuyucu testi önce eski kodda `0.300 m` için kırmızıydı; düzeltmeyle geçti.
- Geometry-core: **770 geçti / 29 ignored**; eski ortak point-input fixture'ları dahil.
- Native interaction: **706 geçti / 6 ignored**. Beş proje biriminde gösterilen
  değerin geri okunması; mm/m/in/ft fiziksel değerleri; hatalı girişte korunma;
  snapshot değişmezliği; ilk köşe ve son duvarın gerçek kalınlığı sınandı.
- Desktop: **1274 geçti / 219 ignored**, `--test-threads=1`, 98.10 s.
- Geometry-core / interaction / desktop bütün target'larda clippy `-D warnings` geçti.
- WASM bağlayıcı derlemesi geçti. Web UI ADR 0228 nedeniyle çalıştırılmadı.
- Test çıktıları `.run/architecture-input/{red,core-interaction,desktop-tests,clippy,wasm}.log`.
  TS üretimi bu dilimin yok sayılan `contracts-ts` klasörüne yönlendirildi.

Bu dilim yalnız tek uzunluk alanının parser uyumsuzluğunu kapatır. Ortak
klavye odak davranışının bütün araçlarda incelenmesi ve ARCH-PERF-01'in
pencere/GPU gecikme kabulü ayrı açık işlerdir.
