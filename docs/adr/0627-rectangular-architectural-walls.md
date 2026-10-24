# 0627 — Ölçüsü açık dikdörtgen mimari duvar

24 Ekim 2026. Dal: `codex/architecture-2d-workflows`.
Kapsam: [mimari planın](../../plan/architecture.md) `ARCH-DRAW-01` ilk dilimi.
Durum: ARCH-DRAW-01 işlevsel kabulü tamam; ortak GUI tabanının ve mimari yol
haritasının kalan kabulleri açıktır.

## Bağlam

Kullanıcı Mimari sekmesinin kolay ev planı çiziminde yetersiz kaldığını bildirdi;
önce ayrıntılı plan, ardından ekranda doğrulanan küçük değişiklikler istedi.
İlk dilim, dört köşeyi ayrı ayrı çizme/kapatma yerine iki köşe veya sayısal boyla
kapalı duvar üretir. Önceden çalışan serbest Duvar ve normal Dikdörtgen ayrı kalır.

[ADR 0626](0626-units-scale-and-true-size-texts.md) bu çalışmanın tabanıdır:
mimari başlangıç metre / 1:100, tek ad “Çizim ölçeği”, gerçek boy yazı ve hareket
sırasındaki katman/önbellek kuralı korunur. Bu dilim eski mm/1:1 varsayılanını
veya önceki yazı görünürlüğü önerilerini geri getirmez. Şablondaki cm/1:50 farkı
`ARCH-UX-03`'ün ayrı işidir.

## Karar

### 1. Ayrı ürün komutu ve mevcut duvar modeli

- `cad.walls.rectangle` v1, tipli `WallsRectangle` girdisini alır. `frame`, ortak
  `RectangleFrame`'dir: karşı köşeler, köşe + boylar + yön, alan + bir boy veya
  yönlü kenar. Koordinat ve uzunluklar metre, `rotation` radyandır; `edge` yönünü
  kendi kenarından alır. Masaüstü açıyı derece olarak sorup dönüştürür.
- `reference` yoksa `inner`; diğerleri `outer` ve `center`. İsteğe bağlı kalınlık,
  stil, yükseklik, mimari alanlar, öznitelikler, etiket, görünüş ve beklenen
  revision mevcut duvar komutunun kurallarıyla doğrulanır.
- Dikdörtgen mevcut şekil hesabıyla üretilir; **yalnız yeni komutun** yolu ilk
  köşesi korunarak saat yönünün tersine düzenlenir. Tıklama çeyreği iç/dış
  anlamını ters çeviremez. Serbest Duvar'ın çizim yönü değiştirilmez.
- Sonuç mevcut `wall` türünden tek kapalı nesne, dört kaynak köşe ve tek kalıcı
  kimliktir. Yeni kalıcı entity türü veya KCAD şema değişikliği gerekmez.
  Sonradan kapı/pencere için host ve mevcut mahal hesabı için sınır olabilir.
- Önizleme `WallsPlan`, yazma `WallsCreated` kullanır. Native uygulayıcı ortak
  `shapes::rectangle_ring` ve `walls_create` plan/validate/execute yollarını
  kullanır; arayüze ikinci bir duvar hesabı eklenmez.
- Dış komutun `recorded` adımında iç `walls_create::execute` ayrıca günlük satırı
  üretmez. Tek journal komutu `cad.walls.rectangle`, mevcut undo adı `Duvar`dır.
  Önizleme geometri, stil tablosu veya geçmişi değiştirmez.

### 2. Ölçünün hangi yüzü anlattığı

| Seçim | Verilen dikdörtgen | CCW yolun mevcut hiza karşılığı | 4 × 3 m, 0.2 m kalınlıkta iç alan |
| --- | --- | --- | --- |
| İç net (`inner`, varsayılan) | Duvarların iç yüzü | `left`; gövde dışta | 4 × 3 = 12 m² |
| Dış ölçü (`outer`) | Duvarların dış yüzü | `right`; gövde içte | 3.6 × 2.6 = 9.36 m² |
| Eksen (`center`) | Duvar orta çizgisi | `center` | 3.8 × 2.8 = 10.64 m² |

Kalınlık stilin gerçek çözülmüş kalınlığıdır. Sabit bileşenli stile verilen
uyumsuz açık kalınlık, mevcut duvar doğrulamasıyla reddedilir. Araç sabit stil
seçiliyken kalınlık değişimi yerine stil seçimini açıklayan uyarı verir.

Dış/eksen referansında net iki boydan biri sıfır veya negatifse `invalid_wall`,
`path=frame` döner. Kullanıcı boyları büyütebilir, kalınlığı azaltabilir veya
İç net seçebilir. Bu denetim dünya koordinatına eklenerek yuvarlanmış köşelerden
değil kaynak boylardan yapılır: Size'ın boyları, Area'nın alan/boyu, Edge'in
kenar boyu ve eni, Corners'ın yerel eksenlere izdüşümü.

Örneğin uzak başlangıçta π/4 döndürülmüş Size 4×3 ve dış referansta 1.5 m
kalınlık, kısa kenarı tam kapatır. Dünya köşelerinin yuvarlanmasıyla kalan yapay
bir pozitif parça kabul nedeni değildir. Eşik için yeni epsilon eklenmez.
Kaynakta bir ULP pozitif açıklık bulunması ile keyfî derecede küçük bir global
geometrinin çizilebilirliği ayrı kontrollerdir; mevcut duvar geometrisi denetimi sürer.

Duvar listesinin iç `walls[0].width/style/arch.level` hata yolları yeni girdide
`width/style/arch.level` olarak sunulur; türetilmiş köşe hataları `frame`'e gider.

### 3. Masaüstü aracı ve otomasyon

- Mimari › Duvar'da ayrı `tool.wallRectangle` komutu. İki köşe ya da Boyutlar ile
  ilk köşe, uzunluk/en ve açılma yönü alınır. Boy referansı (R), Kalınlık (E),
  Stil (S), Boyutlar (B), Döndür (D); sayısal boy modundan Köşeler (K) ile dönüş.
- Önizleme gerçek duvar gövdesini ve seçilen referans dikdörtgenini birlikte
  gösterir. Kalınlık, referans ve stil istemde okunabilir. Yazma başarısız olursa
  geometri oluşmaz, mevcut hata görünür kalır.
- Yeni araç mevcut duvar stil/kalınlık yardımcılarını kullanır. Normal
  Dikdörtgen'in dönüş/köşe belleği, serbest çizim ve genel tuş bitirme davranışı
  bu dilimin dışında kalır.
- Katalog, native registry ve başsız host aynı uygulayıcıyı sunar; Python SDK
  ortak katalogdan üretilir. MCP'nin normal komut keşif/çağrı yolu kullanılır.
  Arayüz ve script için farklı geometri kuralları bulunmaz.

## Sınırlar

Bu ilk dilim otomatik mahal/etiket üretmez, komşu duvarları silmez veya ortak
plan topolojisini yeniden yazmaz. Net uzaklıklı bölme, çok odalı hızlı çizim,
mimari palet, tip kitaplığı, otomatik cephe ölçüsü ve diğer araçlar planda ayrı
maddelerdir. Yalnız dikdörtgen duvarın eklenmesi kolay ev planı veya genel
uluslararası standart uygunluğu kabulünü tamamlamaz.

Web arayüzü [ADR 0228](0228-web-pause-desktop-and-cloud-first.md) gereği bekler;
ortak sözleşme, SDK ve başvurular sürer. Web dönüşünde kendi etkileşimi bu
komuta bağlanacak; ertelenen iş `TODOS.md` §0.2'dedir.

## Doğrulama ve açık sınırlar

[Kabul kaydı](../baseline/2026-10-24-architecture/README.md) gerçek ekranları,
örnek KCAD dosyalarını, hash'leri, test kapsamını ve ham süreleri içerir.

- Bağımsız Decimal/Taylor başvurusu **81 ortak durum**; dört çeyrek, dört frame,
  dönüş, üç referans, stiller, beş birim, uzak koordinat, ret ve undo. Mevcut
  sayısal toleranslar genişletilmedi.
- Native entegrasyon **149**, etkileşim **637**, masaüstü **1261**, MCP **29**
  test geçti. Headless yeni kabulü ve kaynak boy sınırı birim testi ayrı ayrı
  geçti. Native 22, interaction 1, desktop 218 ignored test çalıştırılmış sayılmaz.
- Python tam küme **262** geçti; son native paket yeniden derlenince yeni
  dikdörtgen duvar kabulünün üç testi tekrar geçti.
- Gerçek fare/klavye ile net 4×3 m oda, kapı, pencere, mahal → **12 m²**;
  duvar tek undo/redo; yarım taslakta sağ tuşta tek bitirme; KCAD kayıt/açma;
  18 px açık tema seçenekleri; sol üst boy etiketi; mevcut özellik panelinde
  200→250→undo 200 mm ve korunan hosted açıklıklar/net alan gözlendi.
- Otomatik GPU sahneleri iki tema × iki pencere boyunda toplam **16** görüntü
  üretti; 13/18 px pikseller gözle incelendi. Bunlar gerçek pencere testinden ayrıdır.
- İlgili yedi pakette all-targets clippy, ortak WASM derlemesi, yeni dosyalarda
  rustfmt, üreteçler ve bağımlılık yönü geçti. Repo genelindeki mevcut biçim
  farkları toplu yeniden biçimlendirmeyle bu değişikliğe taşınmadı.

Eşlenik debug Python/FFI plan ölçümünde yeni ve mevcut kapalı Duvar yolları aynı
çıktıyı verdi; 100 bin nesnede ortak O(n) belge taraması yaklaşık 20–28 ms
medyan/p95 düzeyindedir. Bu ölçüm GPU/FPS ya da titreme giderme kabulü değildir.
`ARCH-PERF-01` ayrı dizin/önbellek değerlendirmesini izler. Gerçek pan sürükleme
yer değiştirmesi bağımsız doğrulanamadığı için tam ortak gezinme GUI kabulü
`ARCH-BASE-01`'de açık kalır. Menü sonrası metin odağı ve birimli giriş yardımı
`ARCH-UX-01`'dedir. Bu dilimde ortak render, birim ve tuş sistemleri değişmedi.
