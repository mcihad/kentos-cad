# Ortak oda konturundan tek duvar ağı — kabul kaydı

25 Ekim 2026 · `codex/architecture-2d-workflows` · [ADR 0638](../../adr/0638-shared-room-wall-boundaries.md)

Mevcut `cad.walls.convert(mode=contours)` aynı hedef katmanda Eksen referansı
ile bitişik odaların tam/kısmi düz veya analitik yaylı ortak sınırını bir kez
üretir. Çıktı normal duvardır; yeni dosya şeması yoktur. Önizleme mevcut
sahne birleşim hesabını kullanır. İç net/Dış ölçü çelişkisi açıklanarak
atomik reddedilir; kaynak görünüm/öznitelik sahipliği ve mevcut 1e-6 m
birleştirme toleransı uyarıda belirtilir.

## Gerçek arayüz

İzole QA uygulaması, metre/1:100; koyu tema/18 px ve açık tema/13 px. Test edilen
uygulamanın özeti `gui-binary.json` içindedir. Test binary'si özellik kodunu
kapsar; sonrasında tamamlanan `wall_shared_boundary` katalog yetenek listesi
bu binary'den sonraki metadata düzeltmesidir. Kaynaklar `before.kcad`:
ikisi tam 3 m düz sınır, ikisi kısmi 1 m düz sınır, ikisi R=4 m çeyrek yay
paylaşan altı oda alanıdır. Bütün seçimler gerçek fareyle yapılmıştır.
`gui-events.json`, gerçekten gönderilen CUA tıklama/klavye adımlarının elle
tutulmuş kronolojik kaydıdır; uygulama içi olay kaydedici veya yapay replay
çıktısı değildir.

| Ekran | Gerçek etkileşim / gözlem |
| --- | --- |
| `01-before.jpg` | Altı ölçülü kaynak, dönüşüm öncesi |
| `02-inner-conflict.jpg` | İki bitişik oda + Kapalı kontur + İç net; Enter işlemi reddeder |
| `03-shared-preview.jpg` | Görünür ayarlardan Eksen/Koru; üç duvar, 3 m tek ortak kenar, birleşmiş önizleme |
| `04-single-context-confirm.jpg` | Sağ tuş menüsünde Onayla/bitir; **bir** tıklama dönüşümü tamamlar |
| `05-straight-applied.jpg`, `06-straight-undo.jpg` | Kaydet, tek geri al, kaydet; yinele ile üç duvar geri gelir |
| `07-partial-preview.jpg`, `08-partial-applied.jpg` | Kısmi 1 m sınır; Enter bir kez uygular, iki T birleşimi |
| `09-arc-preview.jpg`, `10-all-applied.jpg` | Analitik 6.283185 m ortak yay; Enter bir kez uygular, 15 nesne |
| `11-arc-remove-preview.jpg`, `12-arc-remove-applied.jpg` | Kaynak Sil; 2 kaynak silinir, 3 duvar eklenir; tek undo/redo; son durumda 13 nesne |
| `13-repeat-refused.jpg` | Korunan A odasının içinden fareyle seçim; önceki duvarlar açık parçalara ayrılmış olsa da tekrar Enter atomik reddedilir |
| `14-zoom-junctions.jpg` | Fare tekeriyle yakın görünüm; birleşim ve tek bölme görünümü |
| `15-reopened.jpg` | Uygulamayı kapat/aç; Home sonrası bütün ağ, 13 nesne ve 1:100 |
| `16-narrow-1100x650.jpg`, `17-narrow-contour-refusal.jpg` | Pencere köşesini gerçek fareyle sürükleyerek **1100×650 iç alan**; Mimari araç, yöntem listesi, görünür ayarlar ve okunur ret açıklaması |
| `18-light-13-narrow-preview.jpg`, `19-light-13-narrow-applied.jpg` | Açık tema/13 px, gerçek 1100×650 iç alan; yöntem/Eksen/Sil seçimi, okunur paylaşım uyarısı, tek Enter ve tek undo; 7/6 nesnelik kayıt |
| `20-regression-room-12m2.jpg`, `21-regression-width-025.jpg`, `22-regression-free-wall.jpg` | Mevcut Dikdörtgen duvar ile 4×3 m net oda; fareyle 0.9 m kapı/1.2 m pencere ve mahal → 12.00 m²; mahal undo/redo; özellikte .2→.25 m ve undo; serbest 3+3 m L duvarını tek sağ tuş onayıyla bitirme ve tek undo |

Geniş pencere iç alanı 1440×828'dir; ekran görüntüleri Retina 2× olduğu için
2880×1720 pikseldir (başlık çubuğu dahil). Dar pencerenin ham kaydı
2200×1364'tür. Bu kayıt **1440×900 veya tam tema/font matrisi kabulü değildir**.
Kayıtlar ekranı yeniden boyutlandırarak üretilmemiş ham JPEG'lerdir. Açık
temada CAD çizim zemininin mevcut “Türe göre” seçimi korunur. Ek açık tema
kaynağı izole başlatıcıya verilip yeniden açılmıştır; macOS “Go to” dosya
seçicisini otomasyonla tamamlamaya yönelik deneme başarılı kabul edilmemiştir.

## Dosyadan bağımsız ölçüm

`python3 docs/baseline/2026-10-25-architecture-shared-wall-contours/verify.py`

Bağımsız KCAD okuyucusu, KentOS hesap modülü kullanmadan gerçek GUI kayıtlarını
inceler. Kaynak ölçülerinden beklenen tekil eksen uzunlukları **25 m**, **23 m**
ve **12+5π m**. Ortak bölme/çeyrek yay bir kez bulunur; yay doğrusal parçalara
çevrilmemiştir. İlk kaynağın öznitelik/görünümü, kaynak UID'leri ve ayarlar,
Koru/Sil, tek undo/redo, tekrar ret ve yeniden açma doğrulanır. `arc-remove`
sonrası dosya yerel sayısal ID'leri sıkıştırılır; karşılaştırma kalıcı UID
ile yapılır. Birleşmiş ilk iki odanın bağımsız gövde alanı beklentisi
`8.2×3.2−2×3.8×2.8 = 4.96 m²` olup Rust testinde gerçek solver ile kıyaslanır;
ham duvar gövdelerinin birleşimsiz toplamı bu değer diye raporlanmaz.

## Otomatik kontroller

Ham komut kayıtları ve son sonuçlar `logs/` ve `test-results.json` içinde
saklanır. Önceki 69 dönüşüm vakasının aynen korunduğu
`fixture-prefix-check.json` ile kayıtlıdır; bilinen U/bölme/U yollarından
**24 bağımsız vaka** eklendi (toplam 93). Eski create/edit/join vakaları
beklenti üretmek için yeni algoritmaya bağlanmadı.

- Çekirdek: tam/kısmi düz sınır, uzak koordinat, analitik daire, tekrar/kesişme/nokta teması politikası.
- Native: 8 hedef test; beş proje birimi × iki koordinat × iki yön × Koru/Sil;
  25 m tekil eksen, 4.96 m² birleşmiş gövde, iki ayrı T düğümü, tek undo/redo,
  kaynak öznitelikleri, farklı katman, İç net/Dış ölçü reddi ve analitik yay.
- Etkileşim: 4 hedef test; salt okunur önizleme, görünür seçenekler, ilk kaynak
  sahipliği/tolerans mesajı, tek onay ve undo.
- Python: ortak komutta plan/execute, undo/redo, bölme duvarına kapı host'u,
  KCAD kaydet/aç ve tekrar reddi.
- MCP: aynı komutun salt okunur planı, execute/undo/redo ve tekrar reddi.

Tam koşular: geometry-core alt kümeleri 521+3+246+2+1, interaction 63+660,
native application 51+176, desktop 1280 ve commands 799 test geçti; bilinçli
ignored testler ham kayıtlarda ayrı. İlgili crate'lerin bütün hedeflerinde
clippy `-D warnings`, optimize WASM, SDK/manifest/şerit ve bağımlılık yönü
kontrolleri geçti. Python koşularının kapsamı ve düzeltmesi aşağıda ayrıdır.

İlk tam koşuda uyarı yetenek listesinde eksikti; liste düzeltildi.
Sonraki sözleşme kontrolü uyarının kod anlamını da zorunlu tuttu; ortak
`codes.rs` kaydı eklendi, katalog/SDK yeniden üretildi ve 799 test geçti.
Python tam kümede 278 testin 277'si geçti; yeni test yanlışlıkla `.plan()`ın
Python uyarısı yükseltmesini bekliyordu. Mevcut SDK bunu `.answer(op='plan')`
yanıtının `warnings` alanında verir. Ürün davranışı değiştirilmeden test bu
sözleşmeye düzeltildi; aynı dosyanın 3 testi son koşuda geçti. Bu kayıt ikinci
bir tam 278 test koşusu yapıldığı iddiası değildir.
Geliştirme testleri açık yaylı duvarın son sıfır bulgesinin kayıt düzenine
uydurulması gereğini yakaladı ve kod düzeltildi. T düğümü ile solver olay
kaydı sayısının aynı olmadığı testte düzeltildi. Uzak koordinatta gövde
alanı doğrulaması, mutlak FP64 koordinatın ULP'si × 64 m çevre sınırı ve
1e-12 m² yerel hesap payını kullanır; ürünün toleransı büyütülmedi.

## Açık kabul

ARCH-DRAW-03 **[~]**. Semantik kat/level ayrımı, karışık kaynakların daha geniş
matrisi, büyük ağ süre/tepe bellek kabulü ve tam tema/font/pencere matrisi açık.
Başlangıç yoğun ownership tablo bütçesi 8 milyon f64'tür; bu bütün süreç
belleğinin ölçülmüş üst sınırı değildir. Mevcut Each/Pairs davranışı, tekil
kontur ve kaynak referansları, import/ölçek/render sayısal altyapısı korunur.
