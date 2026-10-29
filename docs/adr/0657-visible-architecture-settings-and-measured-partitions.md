# 0657 — Görünür mimari ayarlar ve ölçülü düz bölme

24 Ekim 2026 · `codex/architecture-2d-workflows`.
Kapsam: `plan/architecture.md` ARCH-UX-01 ve ARCH-DRAW-02'nin düz duvar dilimi.
Kabul sonuçları ve gerçek ekranlar: [kabul kaydı](../baseline/2026-10-24-architecture-tools/README.md).

## Korunan taban

ADR 0626'nın metre / 1:100 mimari başlangıcı, tek Çizim ölçeği, gerçek boy yazı,
FP64 belge koordinatları ve hareket sırasındaki render/önbellek kararları değişmez.
Import, birim dönüşümü, genel sağ tuş/klavye sistemi veya mevcut duvar geometri
motoru yeniden yazılmaz. Yeni duvar normal `wall` nesnesidir; KCAD şeması değişmez.

## Etkin araç ayarları

- Interaction `Tool::settings/configure` görünür alan tanımlarını ve uygulamasını
  sağlar. Varsayılan yöntemler boştur; mevcut diğer araçlar etkilenmez. Desktop
  `architecture_settings.rs` yalnız bu tanımları mevcut PropertySheet/EditCell/menu
  bileşenleriyle sunar; belge hesabı içermez.
- Duvar: stil, gerçek kalınlık, hiza, öteleme. Dikdörtgen duvar: stil, kalınlık,
  iç/dış/eksen referansı, iki boy, açı. Kapı/pencere/boşluk: tür, çözülmüş en,
  serbest/ortala, uçtan uzaklık; kapıda açılma yanı ve menteşe.
  Mahal: alan yöntemi, numara, ad, kullanım, duvarlardan bul/çiz sınırı.
- Başlık **Yeni nesnenin araç ayarları**; altında **Seçili nesnenin özellikleri**
  ayrı kalır. Başlatılan mimari araç mevcut Öznitelikler panelini görünür yapar.
  Panel küçültülmüşse kaydırılır; mevcut panel büyütme/katlama davranışı korunur.
- EditCell Enter/odak kaybıyla uygular; her tuşta belge yazılmaz. Escape hücre
  düzenlemesini iptal eder. Commit araç kimliği + geçerli/düzenlenebilir alanla
  sınanır; eski araçtan geciken olay yeni araca uygulanamaz.
- Sayısal alanlar mevcut birim ayrıştırıcısını kullanır: `250mm`, `0.25m`, `8"`;
  dikdörtgen boyu `8m,6m`. Sabit bileşenli stilin kalınlığı salt okunur;
  ortalanmış açıklıkta uçtan mesafe etkin değildir. Geçersiz değer önceki geçerli
  değeri korur; araca özgü uyarı görünür kalır. Mahal adı `G` gibi bir metin olabilir.
- Yalnız görünür ayarı olan mimari aracın komut seçeneği açıldığında komut girişine
  odak verilir. Eski genel kısayol yönlendirmesi değiştirilmez.
- Gerçek 18 px denemesinde çok satırlı yardımın alanları aşağı ittiği görüldü;
  başlık ve açıklama kısaltıldı. Birim örnekleri geçersiz giriş yardımında ve
  ilgili komut isteminde kalır.

## Ölçülü bölmenin sözleşmesi

`cad.walls.partition` v1 / `WallsPartition`; native plan/validate/execute,
Python `cad.walls.partition`, headless ve MCP aynı uygulayıcıyı kullanır.

- `source={wall: kalıcı UUID, edge: sıfır tabanlı indis}` açık bir kaynak seçer.
  `side=left|right` kaynak yolun yönüne göredir; varsayılan left.
- `measure=clear|axis`; `distance` metre. Varsayılan clear, pozitif ve sonlu.
  Kaynağın gerçek hiza/ötelemesi ve çözülmüş kalınlığı okunur.
- `wall` mevcut `WallsCreate` alanlarını taşır: hedef katman, görünüş, stil,
  öznitelikler, label, beklenen revision. Tam bir `NewWall`, `pts=[]` verilir;
  kapalı/yay/eksenden farklı hiza/sıfırdan farklı offset reddedilir. Yol komutça
  hesaplanır. Kullanıcı eksik bıraktıysa kaynak mimari bilgisi ve yüksekliği alınır.
- `extent.kind=source`: kaynak segmentin başlangıç ve sonuna paralel.
  `between`: açıkça seçilen iki duvar segmentinin eksenleriyle kesişir; T birleşimi
  mevcut duvar motorunca çözülür. Sonuç seçilen sonlu sınır kenarlarına ulaşmalıdır.
  Paralel, erişilmeyen, aynı yerdeki sınırlar veya farklı katlar reddedilir.
  `length`: kaynak başlangıcından `anchor`, pozitif `length`, forward/backward/both.
  İki yönde uzunluğun yarısı her yandadır. Kaynak segmentin boyunu aşma açıkça reddedilir.
- İlk dilim düz kaynak ve düz sınırlar içindir. Seçilen yay segmenti `invalid_wall`
  ile açıkça reddedilir; sessiz düzleştirme yoktur.
  29 Ekim eki: [ADR 0734](0734-measured-partitions-on-arc-edges.md) yaylı kaynak ve sınırı alır (aynı merkezli
  yay); bu ret kalktı.
- Hedef kilitliyse yazılmaz. Kilitli kaynak/sınır yalnız okunabilir; değiştirilmez.
  Eksik kimlik, geçersiz stil/boy, revision çatışması atomiktir.
- Plan `WallPartitionPlan` döndürür: mevcut WallsPlan, kaynak yüzü, ölçü çizgisi,
  `clearDistance`, `axisDistance`. Plan belgeyi veya stil tablosunu yazmaz.
  Execute tek dış journal komutu ve tek `Duvar` undo adımı üretir.

### Sayısal yöntem

Hesap shared geometry-core `geom::wall_partition` içindedir. Kaynak başlangıcında
yerel FP64 çerçeve kurulur; normal/doğrultu burada hesaplanır. Dünya başlangıcı
sonuçlara en son eklenir. Global epsilon veya görünmez snap toleransı büyütülmez.
JS/WASM/native bit uyumu için mevcut `jsmath::js_hypot/js_min/js_max` kullanılır.

Kaynak sol yüzünün yola göre uzaklığı `L`, kalınlığı `ws` ise eksen uzaklığı
`L−ws/2`'dir. Yeni kalınlık `wn` için net mesafeden eksen ayrımı
`a = d + (ws+wn)/2`; eksen girdisinde kalan net `d−(ws+wn)/2` olur ve pozitif
olmalıdır. Örneğin 0.20 m kaynak, 0.15 m bölme, 3 m net için eksenler 3.175 m'dir.
Hizası iç yüze verilmiş kaynağın yolu ekseni olmadığı için yeni yolun dünya
koordinatını bu 3.175 değerinden doğrudan üretmek yanlış olur; L de hesaba girer.

### Masaüstü seçimi ve onay

- Mimari › Duvar'da **Ölçülü bölme**. Kaynak seçilir, yan/net veya eksen/mesafe/
  kalınlık/uçlar görünür panelden değişir; iki sınır seçimi adım adım istenir.
- Görünen duvar geometrisi/yüz çizgisi mevcut pick aperture ile seçilir; boş
  iki yüz arasındaki her noktanın seçilebilir olduğu varsayılmaz. Genel kenar araçları
  semantik duvarları dışladığından onların `pick_edge` yolu kullanılmaz.
  Kaynak segment vurgulanır; birden çok kaynak adayı varsa **Kaynak duvar** listesi
  katman/nesne/kenarı gösterip seçim değişimine izin verir. Geniş sabit dünya
  yarıçapıyla görünmeyen duvar benimsenmez. Katman ve tür süzgeci korunur.
- Önizlemede gerçek duvar gövdesi, kaynak yüz, net ölçü çizgisi, iki mesafe ve
  duvar motorunun uyarıları görünür. Önizleme yazmaz. Enter / sağ tuş Onayla
  tamamlanmış bölmeyi tek kez yazıp aracı bitirir. Sol tıklama yazıp yeni kaynak
  için devam eder. Escape ayardan, son sınırdan, kaynaktan ve araçtan geri çıkar.
- Oluşan duvar mevcut taşıma/düzenleme, hosted açıklıklar, mahal hesabı ve ilişkili
  ölçü sistemine katılır. Bu bir **kalıcı uzaklık kısıtı** değildir: kaynak sonradan
  taşınırsa bölmenin 3 m uzaklığı otomatik korunmaz. Böyle bir kısıt ayrı model,
  çakışma çözümü ve kabul gerektirir; bu dilimde varmış gibi sunulmaz.

## Otomasyon ve sınırlar

Ortak katalog, komut manifesti, Python SDK ve native registry aynı kimliği taşır.
SDK üreticisinde tagged-union varsayılanını bir import adı gibi yazan hata,
factory ifadelerini import hesabından ayırarak düzeltildi. Ayrıca dönüşüm
gerektirmeyen alanlarda erken dönüşten önce factory varsayılanı uygulanır;
`UNSET` yerel kodlayıcıya sızmaz. Aynı üretici düzeltmesi `sheets.create`'in
atlanmış options alanını da boş sözlük yapar. `extent` verilmeden Python çağrısı
ve 265 testin tamamı kabul testindedir. İç WallsCreate doğrulamalarının ret/uyarı
yolları dış istekteki `wall.*` alanlarına taşınır.

Web UI ADR 0228 gereği bekler; ortak katalog ve SDK üretilir, WASM derlenir.
Web arayüz testleri çalıştırılmaz. Global zoom/titreme performansı, tüm mimari
araçların yeniden tasarımı, kalıcı kısıtlar ve yaylı bölme bu kabulün dışındadır.

## 27 Ekim 2026 eki: yan imleçle, uzaklık ölçüyle

Daire yürüyüşünde (ADR 0344'ün 27 Ekim eki) Ölçülü bölme'nin “Kaynağın yanı: Sol/Sağ” seçimi duvarın çizildiği yöne
bağlıydı; kullanıcı yolun yönünü bilmeden tahmin ediyordu, önizlemedeki uzaklık da sayısız düz bir çizgiydi.

- **İmlecin yanı** (varsayılan): kaynak kenar seçildikten sonra bölme imlecin kenarın hangi yanında olduğuna gider (AutoCAD'in
  Ötele'si gibi). Sol ya da Sağ seçilince imleç artık yanı değiştirmez; İmlecin yanı yeniden seçilebilir. Komut değişmedi:
  yan girdide açıktır (`side`).
- Önizlemedeki uzaklık, geçici ölçüler gibi, sayısı zeminli bir ölçüdür (net ya da eksen, Uzaklık türüne göre).

Doğrulama: `cargo test -p kentos-interaction --test all partition_side` (imlecin batısında ve doğusunda bölme, seçilen Sol
imleçle değişmez, önizlemede zeminli sayı); resim `KENTOS_SHOTS_ONLY=daire-bolme-onizleme`.
