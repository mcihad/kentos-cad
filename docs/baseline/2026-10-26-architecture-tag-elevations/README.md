# Mahal kotu etiketleri — 26 Ekim 2026

ARCH-SPACE-02'nin sınırlı dilimi; [ADR 0687](../../adr/0687-room-tag-elevations.md).
Yeni komut, TS/TSX dosyası, KCAD alanı, birim/ölçek veya renderer değişikliği yok.
Mevcut anlamsal etiket komutu, Python ve MCP aynı hesabı kullanır.

## Değerlerin anlamı

- `{floorElevation}`: mahalın **bilinen katının** metreyle kayıtlı kotu.
- `{ceilingElevation}`: bu kot + mahalın **açıkça verilmiş** yüksekliği.
  Katın döşemeden döşemeye yüksekliği tavan sayılmaz.
- Bilinmeyen kat/yükseklik boş yazılır; bilinen sıfır `0.000`'dır.
  Yazım mevcut uzunluk kuralıyla çizim birimi ve basamağındadır.
- Kaynak değişimi aynı model undo adımındadır. Ayarlar mevcut sözleşmede
  model undo'suna ait değildir; bu yeni şablonların türetilmiş yazısı ayar,
  geçmiş/iptal ve dış değişiklik aktarımından sonra güncel kaynağı okur.
  Etiketin elle konmuş yeri, kolu, çerçevesi ve UUID'si değişmez.
- Varsayılan etiketler ve mevcut yükseklik/hacim fallback'i korunur. Mahal
  düşey offset'i/bitmiş döşeme veya asma tavan geometrisi modeli eklenmez.
  Eski KentOS yeni yer adlarını tanımayıp şablonu reddedebilir.

## Otomatik kontroller

`test-runs.json` her çağrının komutunu, zamanını, çıkışını ve ham günlük
hash'ini tutar. Erken denemeler son kabulün yerine sayılmaz:

- `red-elevations`: eski sürüm yeni kot yerlerini reddeder.
- `green-cases`: beş atanmamış kat durumunda referansın boş `arch: {}`
  yazımı kanonik dosyayla uyuşmadı; yalnız referansta boş alan kaldırıldı.
  Matematik beklentileri değiştirilmedi.
- `python-build`: eksik venv seçimi nedeniyle çalıştırıcı hatası;
  `python-build2` doğru proje/venv ile başarılıdır.
- `python-focused`: test yanlış `TagsPlan.entities` adını kullandı.
  Belgelenen `tags` ve planın sıfır slotu doğru karşılaştırıldıktan sonra
  `python-focused2` altı test/20 birim-yükseklik birleşiminde geçti.
- `settings-timing` ilk betikte on değerin üst ortancasını yazıyordu;
  kabul ölçümü standart `statistics.median` kullanan `settings-timing-final`'dır.

Başarılı kontroller:

- `cases-final`: **195** ortak durum. 47 eski create durumu, izin verilen
  token listesini söyleyen hata metni dışında aynıdır; **30 yeni** durum
  KentOS kodu çağırmayan Decimal referansından gelir. Beş birim × sıfır,
  negatif/üst kot, eksik yükseklik, atanmamış/bilinmeyen kat;
  validate/plan/execute/tek undo/redo ve bütün eski nesneler karşılaştırılır.
- `application-final`: **219 başarılı, 22 mevcut atlanan**; tüm ortak fixture'lar
  bu koşunun içinde de geçti. Atlananlar başarı sayılmaz.
- `domain-final`: **67 başarılı**; yeni ret/ayar/konum kontrolü dahil.
  Dış değişiklik için genişletilmiş son testin sonucu `domain-final`'dadır.
- `interaction`: **697 başarılı, bir mevcut atlanan**; eski görünür etiket,
  tutamaç, duvar, açıklık, mahal ve tek onay/iptal kontrolleri dahildir.
- `mcp`: gerçek hostta yeni kot içeren etiket + numaralandırma, plan/ret/
  tek undo/redo ve kimlik/konum korunumu; **bir başarılı test**.
- `python-all`: **309 başarılı**. Yeni altı test; kaynak yüksekliği ve kat,
  yerinde takip, geçerli plan, grup iptali, mevcut ayar altında undo/redo,
  kaynak silme, şablon düzenleme ve 20 gerçek KCAD kayıt-açma birleşimi.
  Mevcut SQLite testlerinin ResourceWarning bildirimleri günlükte korunur.
- `catalog`: üç başarılı katalog testi; SDK drift ve bağımsız tag referansı
  geçti. `kcad-reference`: **1170** mevcut dosya ve bağımsız okuyucu tutarlı.

- `properties-final2`: **42 başarılı, dört mevcut atlanan**; CAD mahal yüksekliği
  sayısal hücresi ve birim kuralları dahil. Sonrasında yalnız notun metni
  kısaltıldı; `clippy-final3` ve `build-final3` bu son kaynağı doğrular.
- `clippy-final`: ilgili contracts/domain/application/interaction/MCP/Python/
  desktop bütün hedefleri `-D warnings`; `wasm-contracts`: contracts WASM
  hedefi başarılıdır. Native domain WASM modülü değildir. Son masaüstü clippy
  ve derleme de geçti; ham ts-rs/gelecekteki bağımlılık uyarıları korunur.
- `independent-final`: **on tam GUI kaydı**, 19 ham ekran, paket/kaynak/günlük
  hash'leri ve iki gerçek profil bağımsız doğrulamadan geçti.

## Sınırlı süre ölçümü

`measure.py`, 1000 bağımsız mahal ve 1000 bağlı etiketle on farklı kat kotu
ayarını kamuya açık debug Python SDK'sından değiştirir. Her çağrıdan sonra
2000 nesnenin tamamı, kaynak UUID'leri, yerler ve bağımsız Decimal metni
karşılaştırılır. `settings-timing-final`: ortanca **8.259 ms**, en yüksek
**8.575 ms**; değişiklik yapmayan ilk çağrı **0.514 ms** ayrı tutulur.
`settings-timing.json` ham örnekleri içerir. Bu küçük debug örneklemi release,
GPU, bellek veya bütün uygulama için p95 performans kabulü değildir.

## Gerçek GUI ve bağımsız dosya

01 mevcut ekranı, 02 eski pakette yeni kot şablonunun görünür reddini gösterir.
Ham ekran baytları düzenlenmez; `gui-events.json` SHA256/zaman/paket bağlantısı
tutar. Testler kullanıcı profili yerine ayrı QA profiliyle yapılır.

03–08 yeni şablonun panel yardımını, `18,5` koordinat taslağını ve **tek Enter**
ile kaydı gösterir. 04–05 kaydırma sırasında alınmış tanısal görüntülerdir;
yardımın bütünü 06'da görünür. Açık18 geniş pencerede mahalın açık yüksekliği
yokken yeni etiket `Zemin 3.000`, boş `Tavan` yazar. Kaynak 11 üst kattadır;
katlar arası 3 m tavan değeri olarak kullanılmaz.

09 sayısal alandaki eski `(katın)` kesilmesini yakaladı. 10 ilk düzeltmenin
uzun notunun da kesildiğini gösterir; ikisi başarı kanıtı değildir. Son paket
11'de yalın `3.000 m` ve ayrı `Katlar arası` notunu eksiksiz gösterir.
Değişmemiş alanda Enter açık `Space.height` üretmedi. 12'de gerçek `275 cm`
girişi tek Enter ile 2.750 m olur, tavan 5.750 ve çizelge 2.750 izler.
13 tek undo'da açık yüksekliğin kalkmasını ve boş tavanı gösterir; redo
ayrıca disk kaydında doğrulanır. 14'te `0 m` görünür uyarıyla reddedilir.

15 koyu13 geniş, 16–18 koyu13 **2200×1366 px** dar penceredir. 16'da gerçek
`3,125 m` girişi tek Enter ile 3.125 m / 6.125 kotuna dönüşür. Sonra tek undo
2.750 m'ye döner. 17 hedef önizlemesidir; ilk Escape hedefi bırakır, ikinci
Escape araçtan çıkar, 18'de yeni nesne oluşmaz. 19 test uygulaması kapatılıp
yeni süreçte dosya açıldıktan sonraki geniş görünümü gösterir. Açılışta pencere
2880×1720 px'e döner; boyut kalıcılığı bu dilimin kabulü değildir.

`models.json` on gerçek Kaydet dosyasını ve hash'ini listeler. `verify.py`
bağımsız KCAD okuyucuyla başlangıçtaki 14 nesnenin tamamını, UUID sırasını,
ayarları ve elle yerleştirilmiş yeni etiketi karşılaştırır: yalnız beklenen
mahal yüksekliği, çizelge hücresi ve türetilmiş etiket yazısı değişebilir.
Eski sayısal görünümdeki kusurun karşılaştırması da aynı sentetik belgeyledir.
`after3-binary.json` son kaynak ve değişmez test paketini belirtir. Ara
paketler/screenshot'lar üzerine yazılmamıştır. `screenshot-dimensions.json`
ham görüntü boyutlarını kaydeder; tema/yazı gerçek ayar ekranından değiştirildi.

Bu dar kabul açık18/geniş işlemler ve koyu13/geniş-dar tekrarları kapsar;
her araç için sekizli tam boyut/font/tema matrisi iddiası değildir. Başlangıç
belgesinde bilerek üst üste konmuş eski mahal/tag yazıları aynen korunmuştur.

## Açık sınırlar

Bu dilim ARCH-SPACE-02'nin tamamı değildir. Kat öneki/ad/kullanım listeleri,
numara denetimi, Koru/Yenile, farklı pafta oranları ve tüm mimari iş akışı
matrisi açıktır. `{area}` mahalın seçilmiş ölçüm yönteminin alanıdır;
her yöntemde ayrı net alan yerinin hesabı bu dilimde eklenmez. Gerçek bulut
sunucusu/çok kullanıcı entegrasyonu veya BIM katmanlı tavan kabulü yapılmadı.
