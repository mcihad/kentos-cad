# Görünür mimari ayarlar ve ölçülü düz bölme — kabul kaydı

24 Ekim 2026 · dal `codex/architecture-2d-workflows` · önceki taban `10ab270c9`.
[Plan](../../../plan/architecture.md) ARCH-UX-01 ve ARCH-DRAW-02'nin düz dilimi;
[ADR 0657](../../adr/0657-visible-architecture-settings-and-measured-partitions.md).

## Gerçek masaüstü denemesi

Yerel KentOS/Iced/wgpu uygulaması macOS'ta fare/klavye ile kullanıldı. Aşağıdaki
01–07 görüntüleri gerçek **2880×1720 piksel** pencereden alınmıştır; açık tema,
18 px arayüz yazısı. `.run/KentOS Architecture QA.app` ayrı uygulama kimliği ve
ayrı `XDG_CONFIG_HOME` kullanır. Özgün kullanıcı çizimi/tercihi değiştirilmedi.
Önceki kabul dosyasının SHA256 değeri değişmedi:
`c929e8f2506afd8a5f70966292ada17cca80bda93ebb8744091f33a0324f762a`.

1. Yeni CAD › Mimari sihirbazında **metre / 1:100 / yerel başlangıç** görüldü.
   Dikdörtgen duvarın görünür Kalınlık alanında `250mm`, sonra `200mm` uygulandı.
   Boylar `8m,6m`, iç net referans; `(0,0)` ve `(8,6)` ile tek duvar çerçevesi çizildi.
2. Kapı ayarında `900mm`, yerleşimde **Ortala** seçildi; alt duvara fareyle kapı
   yerleştirildi. Mahal ayarında **Net (iç yüzler)**, `Z-01`, `Alt oda` girildi.
   Odanın içine tıklayınca **48.00 m²**, toplam üç nesne görüldü.
3. Ayrı [before.kcad](before.kcad) kaydedildi. Üst Katmanlar paneli katlanıp
   Öznitelikler paneli büyütüldü; yeni nesne ayarları ve seçili nesne özellikleri
   ayrı başlıkta kaldı. İlk uzun yardım metni bu denemede alanları aşağı ittiği
   için kısa başlık/tek satır yardım kullanıldı.
4. Mimari › **Ölçülü bölme**, kalınlık `150mm`; alt duvarın **görünen yüz çizgisi**
   kaynak seçildi. Net uzaklık `3m`, uçlar **İki duvar arasında**; sol ve sağ
   duvarların görünen yüz çizgilerine tıklandı. Önizleme **3.000 m net / 3.175 m
   eksen** gösterdi. Önizlemede mahal hâlâ 48 m² idi; henüz belgeye yazılmadı.
5. Sağ tuş menüsü açıldı ve **Onayla / bitir** bir kez seçildi. Araç kapandı,
   nesne sayısı dört oldu, alt mahal **24.00 m²** oldu. Kapı simgesi ve host
   duvarın açıklığı korundu; iki yanda birleşim görüldü.
6. Bir **Geri al** ile üç nesne / **48 m²**; bir **Yinele** ile dört nesne /
   **24 m²** görüldü. Sonuç ayrıca kaydedilip [after.kcad](after.kcad) kopyası alındı.
7. Yeni bölme taslağında **Eksen arası = 2m**, **Verilen boy**, başlangıç `2m`,
   boy `2m`, yön **İki yön** seçildi. Önizleme **1.825 m net / 2.000 m eksen**
   gösterdi; dokuz alan büyütülmüş panelde kullanıldı. Escape ile taslak iptal
   edildi; ek duvar oluşmadı.
8. Dikdörtgen duvar kalınlığı `250mm` yapıldı; `-1m` + Enter reddedildi ve alan
   **0.250 m** kaldı. Sonra yazılan `5m`, Escape ile iptal edilince önceki
   **0.250 m** geri geldi. Mevcut dört nesne değişmedi.

Arayüzden kaydedilen **bu iki dosya** ayrıca Python `Document.open` ile okundu:
before = 3 nesne / 48 m², after = 4 nesne / 24 m²; ikisi de metre / 1:100.
Dosya özetleri [test sonuçlarında](test-results.txt) kayıtlıdır.

Görünen yüzlerin arasındaki boşluğa tıklama her yakınlaştırmada duvar seçimi
anlamına gelmez; mevcut geometri seçme toleransı kullanılır. Bu işte genel
seçme motoru veya görünmez dünya yarıçapı değiştirilmedi. Birden çok adayın
listeden değiştirilmesi interaction testiyle doğrulandı; bu senaryonun gerçek
pencerede ayrıca yürütüldüğü iddia edilmez.

| Ekran | Gösterdiği sonuç |
| --- | --- |
| [01](01-before-48m2.png) | Kapılı, 48 m² ilk oda |
| [02](02-between-preview.png) | Net/eksen önizlemesi ve görünür ayarlar |
| [03](03-single-confirm-menu.png) | Tek Onayla / bitir seçeneği |
| [04](04-after-24m2.png) | Sonuç, dört nesne, 24 m² ve korunan kapı |
| [05](05-undo-48m2.png) | Tek undo sonrası 48 m² |
| [06](06-axis-both-directions.png) | Eksen ölçüsü, verilen boy ve iki yön |
| [07](07-invalid-width-keeps-250mm.png) | Geçersiz kalınlık önceki değeri korur |

## Otomasyonla doğrulanan kapsam

[Test sonuçları](test-results.txt) son başarılı çalıştırmaların özetidir.
Test adedi alt durum sayısı değildir: bağımsız 90 durum native fixture testinin
içindedir; bir GUI izinin üç varyantı da tek ana test içinde yürür.

| Küme | Geçen | Atlanan |
| --- | ---: | ---: |
| Commands | 792 | 0 |
| Native application entegrasyonu | 153 | 22 |
| Interaction | 642 | 1 |
| Desktop | 1263 | 218 |
| Headless | 51 | 0 |
| MCP | 30 | 0 |
| Python SDK | 265 | 0 |

- Native: farklı kaynak hiza/öteleme, farklı kalınlık, kaynak yüksekliği ve mimari
  bilgisinin mirası; üç uç biçimi, üç yön; kilitli hedef, eksik/yanlış kaynak,
  paralel veya erişilmeyen sınır, yay ve revision reddinde atomiklik. Kaynak,
  kapı ve mahal korunumu; tek journal komutu ve undo/redo.
- Interaction/Desktop: görünür alanlarla belgeyi yazmadan ayar/önizleme; ilk
  köşe korunumu; sabit stil kalınlığı salt okunur; eski araçtan gelen gecikmiş
  commit reddi; düz Çizgi'nin etkilenmemesi; literal `G` mahal adı; aday seçimi.
- `fixtures/interaction/v1/wall-partition.json`: duvar çerçevesi, iki sınır,
  tek sağ tuş onayı, journal yeniden oynatma, undo/redo, Escape ve dosya açma;
  **US / TR-Q / HiDPI**. Desktop tam kümesi mevcut diğer izleri de geçti.
- Python: beş birim × yerel/uzak başlangıçta aynı fiziksel model; kapı ve 24 m²
  mahal; bir undo; KCAD kaydet/aç; projede birim/1:100 korunumu. Bölmeyi 0.5 m
  taşımada mahal **24→28 m²**, mevcut ilişkili ölçünün hedef köşesi 0.5 m taşınır;
  undo her ikisini geri getirir. Bu, kalıcı 3 m uzaklık kısıtı değildir.
- MCP/Headless: komut keşfi, yazmayan plan, execute ve tek undo. Python'da
  `extent` verilmediğinde varsayılan tagged union ayrıca sınandı. Ortak SDK
  üreticisinin factory varsayılan/import hataları düzeltildi; tam SDK kümesi geçti.
- İlgili tüm hedeflerde Clippy `-D warnings`, shared WASM derlemesi, bağımlılık
  yönü (47 crate / 61 hedef), katalog/SDK/şerit/manifest üretim kontrolleri geçti.
  Web UI ADR 0228 gereği çalıştırılmadı; ortak katalog güncel tutuldu.

### Bağımsız sayısal referans

`scripts/fixtures/walls_partition_cases.py` KentOS kodu veya çıktısını çağırmaz.
Python `Fraction` ile beklenen koordinatları üretir. 72 geometri durumu:
**6 doğrultu × 3 hiza × 2 yan × 2 ölçü biçimi**; doğrultular dört ana yön ve
iki rasyonel 3–4–5 yönüdür. Bunlara ret/uyarı, kilitli kaynak ve yazmayan
validate durumları eklenince **90** olur. 0.15/0.25 m bölme, öteleme, beş birim,
`(0,0)` ve `(500000,4400000)` başlangıçları kullanılır. Önceki kesinlik eşiği
büyütülmedi: yerelde **1e-12**, uzakta **1e-8**.

### GPU ekranları ve görsel sınırlar

[gpu/](gpu/) altında üç sahne × iki boyut (1440×900, 1100×650) × açık/koyu =
**12 görüntü** vardır. Bunlar gerçek Iced/wgpu ekran testinden alınmış,
gözle incelenmiş görüntülerdir; fiziksel pencereyi fareyle kullanma kanıtı
yukarıdaki 01–07'dir. İki sahne 13 px, büyük yazı sahnesi 18 px kullanır.

Alanlar birbirinin üzerine binmiyor. Küçük panelde kalan alanlara kaydırılarak
erişilir; gerçek pencerede Katmanlar'ı katlayarak panel büyütüldü. Özellikle
1100×650 / 18 px'de şerit ikonlara daralıyor ve durum çubuğunun son kısmı
sığmıyor. Bu mevcut genel duyarlılık sorunu **ARCH-UX-02'de açık**; bütün
arayüzün bu boyutta kusursuz olduğu veya bütün alanların kaydırmasız sığdığı
iddia edilmez. Görüntüler zoom titremesi/FPS ölçümü değildir.

## Kalan kapsam

- Yaylı kaynak/sınır açıkça reddedilir; ayrı kabul bekler.
- Kaynak taşındığında mesafeyi koruyan kalıcı kısıt yoktur. Sonuç normal duvardır.
- Genel pan/zoom/render, import/birim sistemi ve ortak sağ tuş/klavye mantığı
  değiştirilmedi. Bütün CAD/mimari yol haritası tamamlandı sayılmaz.
- Kaydet/aç otomasyonla geçti; bu dilimin sonuç dosyasını kapatıp yeniden açma
  için ayrıca yeni bir gerçek pencere ekranı alınmadı. Son ek ekran alma
  denemelerinde arayüz yakalama aracı zaman aşımına uğradı; önceki 01–07 gerçek
  denemeleri ve kaydedilmiş dosyalar yukarıdadır.

Yeniden doğrulama: `python3 scripts/fixtures/walls_partition_cases.py --check`,
`python3 scripts/python/sdk.py --check`, `python3 scripts/ui/ribbon.py --check`,
`python3 scripts/cad/manifest.py --check`; native testlerde `walls_partition`,
interaction testlerinde `architecture_settings`, Python'da `test_wall_partition`.
