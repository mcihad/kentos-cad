# Mimari şerit — ARCH-UX-02 kabul kaydı

24 Ekim 2026, `codex/architecture-2d-workflows`; taban `af800a8f0`.
Karar: [ADR 0658](../../adr/0658-architectural-ribbon-and-narrow-cad-status.md).
Durum: ARCH-UX-02 sunum dilimi kabul edildi; genel mimari plan açık.

## Gerçek uygulamada gözlenenler

Kendi QA uygulaması, kendi test dosyası ve tercih kopyası kullanıldı.
Kullanıcının çizimi ve tercihleri değiştirilmedi. macOS Retina 2×;
1440×828 ve 1100×650 çizim istemcisi, açık tema, 18 px yazı.
Bu ekranların uygulama baytları SHA-256:
`15dcc1c72ffd65a9bcdfa6b73a83c3aea53bc4bfab698c0bf72621e0c1f5cdf6`.
Bunlar sonradan eklenen otomatik sekme açığa çıkarma ve iki numaralama
etiketi düzeltmesinden **önceki** gerçek ekranlardır.

1. [Mimari beş grup](01-main-18-light.png): Çiz/Yerleştir/Düzenle/Belgele/Proje.
2. [Duvarı böl başladı](02-split-started.png): 1440 px'de yazılı düğmeye tek
   tıklama aracı başlattı. Escape iptal etti, nesne oluşturulmadı.
3. [Seçili açıklık, panel kapalı](03-opening-context-panel-closed.png).
4. [Özellikler tek tıkla görünür](04-opening-properties-visible.png): aynı
   kapı seçimi ve 48 m² mahal korundu, mevcut kapı düzenleyicisi açıldı.
5. [Dar açıklık bağlamı](05-narrow-opening-context.png): başlık ve 1:100
   durum hücresi görünür.
6. [Sekme çubuğunu elle sola sürükleme](06-narrow-tabs-scrolled-to-start.png):
   ilk sekmelere erişildi, sabit hızlı erişim ve sağ uç korundu. Bu denemede
   yatay tekerlek hareketi doğrulanmadı; kaydırma tutamacı sürüklendi.
7. [Dikdörtgen duvar sırasında yardımcılar](07-narrow-aids-during-rectangle.png).
8. [Orto değişti, taslak sürdü](08-aids-toggle-preserves-draft.png): menü açık
   kalır; altta aynı ilk köşe istemi, çizimde aynı üç nesne ve 48 m² mahal.
   Orto tekrar eski değerine alındı; Escape menüyü kapattı, araç sürdü.
9. [Dar Düzenle menüsü](09-narrow-edit-group.png): adları görünen komutlar.
   Burada iki “Numarala” adının belirsizliği yakalandı; kaynakta “Aksları
   numarala” ve “Açıklıkları numarala” olarak düzeltildi. Bu resim düzeltme
   öncesini dürüstçe kaydeder.

Sonrasında elle kaydırılmış başlıktan seçim temizleme dönüşünde Mimari adı
kesildi. İlk ekran kabulü yeterli görülmedi: dönüş ve gerçek yerleşim sınırı
regresyonu eklendi, etkin başlığı açığa çıkaran dar sekme sarmalayıcısı yazıldı.
Dönüş düzeltmesi sonradan gerçek pencerede de geçti (aşağıdaki 10–17).

## Kullanım adımları ve kapsam

Tabanın 46 mimari komutu yeni gruplarda da 46; kayıp/ek yok. Geometri
araçları yeniden yazılmadı. 1440 px'de Duvarı böl/Açıklığı taşı/Mahali böl,
eski taşma menüsü+komut (2 adım) yerine doğrudan düğme (1 adım).
1100 px/18 px'de Düzenle katlanır, grup+adlı komut (2 adım). Bu karşılaştırma
katalog yerleşiminden; gerçek 1 tık gözlemi yukarıda yalnız Duvarı böl içindir.
Panel kapalıyken bağlamsal Özellikler tek tıkla gösterir; ikinci çağrının
paneli gizlememesi ve belge/taslak korunumu ayrıca test edilir.

GUI değişikliği modelin birim, ölçek, FP64, import veya render hesabını
değiştirmez. Bu kabul FPS, zoom titremesi veya bütün mimari planın kabulü
sayılmaz. Web UI çalıştırılmadı (ADR 0228). Python/yerel geometri testlerinin
önceki dilimdeki sonucu bu sunum diliminde yeniden çalışmış gibi sayılmaz.

## Otomasyon

- Tam masaüstü: **1269 geçti / 218 isteğe bağlı test atlandı** (`final6-desktop`, 98.23 s).
- UI paketi: **278 geçti**; 72 doküman örneği ignore (`--features snapshot`, `final4-ui`).
- Son hedef kümesi: **7 geçti** (`final7-targeted`); gerçek katlanmış menü tıklaması,
  13/18 px × iki genişlikte 5 bağlamın sığması, panel/taslak korunumu,
  büyük koordinat + seçili sayı + uzun bildirim ve etkin sekme dönüşü sınırları.
- Son GPU üretimi: **20 görüntü**, tek test geçti (37.70 s, `final7-gpu`).
  [GPU dizini](gpu/): ana şerit 13/18 px, duvar/açıklık/mahal 18 px;
  her biri 1100×650 / 1440×900 ve açık/koyu. Önceki 20 görüntünün tamamı
  incelendi; son yalnız durum metni düzeltmesinden sonra sorunlu 1100/18
  mahal görünümü yeniden incelendi. Otomatik görüntü gerçek fare testi değildir.
- İlgili clippy (`-D warnings`), ortak geometry WASM derlemesi,
  şerit/ported/manifest tutarlılığı, bağımlılık yönü (47 crate/61 hedef) ve
  `git diff --check` geçti. Web UI testleri çalıştırılmadı.
- Önceki Python/native geometri sonuçları yeni sayılara eklenmedi.

## Son gerçek pencere tekrarları

10–17 resimleri `515512ec2761ec2fd1718dab42676dfe4a1c4629f9426f66483f01f1ba60775d`
uygulamasından: [ana şerit](10-final-main-18-light.png),
[açıklık](11-final-narrow-opening.png), [elle sola kaydırma](12-final-manual-tab-scroll.png),
[Escape sonrası tam Mimari adı](13-final-return-mimari-visible.png),
[açık numaralama adları](14-final-narrow-edit.png),
[dar menüden başlayan araç](15-final-folded-tool-running.png),
[kapının mevcut özellikleri](16-final-properties.png),
[Orto ile devam eden taslak](17-final-aid-preserves-draft.png).

18–19 resimleri `0d27fdf8af92ff6d8f4a0afa8334437f62ad5fc87629d28df9c7806df6430f7e`
derlemesinden: [korunan yardımcı adı](18-final-aid-caption.png),
[menü kapandıktan sonra aynı taslak ve 1:100](19-final-status-draft.png).
Son metin, ölçümle aynı advanced shaping ve satır kırmama kuralını kullanır.
Nihai uygulama SHA-256
`ce4b7d6e9052099ad4729c0a192e33b5c0766168bf3a6c96c57aad91ca9a18d8`:
[1100×650 / 18 px gerçek pencere](20-accepted-narrow.png).
Kurtarma kopyaları Sonra ile yerinde bırakıldı. Bu dilimin gerçek senaryosu
okuma/seçme/araç başlatma/iptaldir; 48 m² ve üç nesne korunur. Geometri
oluşturma/undo regresyonları tam masaüstü etkileşim izleriyle sınandı;
önceki 48→24 bölme denemesi bu dilimde yeniden elle yapılmış sayılmaz.
