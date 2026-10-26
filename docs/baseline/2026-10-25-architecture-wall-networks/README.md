# Büyük ve karışık mimari duvar ağları — kabul kaydı

25 Ekim 2026, `codex/architecture-2d-workflows`, [ADR 0669](../../adr/0669-large-architectural-wall-networks.md).
ARCH-DRAW-03 [~] kalır; genel mimari yol haritası açık.

## Ürün değişikliği

1. Ortak oda sınırında her kenar×kaynak için yoğun tablo yerine sıralı seyrek
   sahiplik kullanılır. Arrangement'ın kesişme/noding, tam köşe, analitik yay,
   sıralama ve 1e-6 m tolerans hesabı aynıdır; mevcut overlay tüketicisi yoğun kalır.
2. 200.000 kaynak köşesi, 200.000 parça, 400.000 sahiplik hücresi sayım kotaları
   vardır. Bunlar kesin RAM veya süre garantisi değildir; noding'in geçici
   kesişme/kesme listeleri çıktı kotalarıyla sınırlanmış sayılmaz.
3. Aynı taraftan çakışık kenar ön kontrolde reddedilir. Çok sayıda aynı oda
   için büyük grafik kurulmaz. Çok hatalı girdinin ilk ret gerekçesi değişebilir;
   `invalid_wall` ve atomik davranış korunur.
4. Kapalı kontur önizlemesi 400 duvarda kesilmez. Bütün kabul edilmiş kontur
   planı görünür; Tek tek/Çiftler ve genel değiştirme araçlarının eski sınırı korunur.
5. Yeni API, sözleşme, TS dosyası, KCAD şeması, ölçek, birim, kamera veya
   render matematiği yoktur. Python ve MCP mevcut komutu kullanır.

## Bağımsız geometri ve otomasyon

- `scripts/fixtures/architecture_mixed_contours.py`: KentOS kullanmadan ölçülü
  dikdörtgen ve L oda, avlu/dolgu, çok parçalı alan, kısmi ortak sınır, analitik
  daire ve yarım daireden yedi kaynak üretir. Beklenen dokuz duvar, altı kapalı,
  üç açık, iki analitik; beş ortak parça/9 m ve toplam `115 + 8π` m.
- Native: m/mm/ft × yerel/uzak koordinat × iki halka yönü × Koru/Sil × iki
  seçim sırası = 48 birleşim. İlk kaynak sahipliği de bağımsız beklenene göre
  denetlenir. Plan yazmaz; normal duvar, tek undo/redo, kaynak görünümü/özniteliği/katı.
- Altı hatalı karma seçim: açık çoklu çizgi, kotlu poligon, kilitli kaynak,
  çelişkili referans, bilinmeyen kat, yinelenen oda. Hem plan hem execute
  bütün belgeyi değişmeden bırakır; Koru kapalı olsa da hiçbir kaynak silinmez.
- 40×40 native oda: 3.276 duvar, 11.480 m; tek undo ile 1.600 özgün kaynak.
  Dört köşede ikişer temel kenar aynı devamlı duvardadır; 3.280 temel kenar
  sayısı ile 3.276 duvar sayısı bu yüzden farklıdır.
- Çekirdek: yoğun/seyrek tam kenar/köşe/sayı eşitliği, yay/kesici/ters yön
  iptali, sayım kotasında atomik ret; 1.600 bitişik, 10.000 ayrık oda ve
  2.000 yinelenen oda regresyonları. Eski iki geçerli büyük girdi Budget ile düşer.
- Etkileşim: 501 ayrık ve 501 bitişik odanın son sınırına kadar tam önizleme;
  önizleme belgeye yazmaz. `preview-before.log` eski 400 sınırındaki hatayı,
  düzeltme sonrası log aynı testin geçtiğini tutar.
- Python: tipli plan, ortak sınır uyarısı, bağımsız analitik uzunluk, Koru/Sil,
  UUID'yle nesne korunumu, undo/redo ve KCAD. MCP: gerçek sunucuda karma girdi,
  aynı dokuz duvar, plan/execute, tek geçmiş adımı ve tekrar ret.
- Mevcut 118 `cad.walls.convert` durumu ve diğer duvar komutlarının bağımsız
  referansları değişmedi. Diğer araçlar genel testlerde korunur.

## Gerçek arayüz

Yalnız CUA fare/klavye; ayrı test uygulaması, ayrı ayar dizini ve bu kabul için
üretilmiş dosyalar. Kullanıcı çizimi değiştirilmedi. `gui-binary-before.json`
eski programı, `gui-binary-after.json` tam önizleme düzeltmesini içeren
programı tanımlar. Görünüm koyu/18 px (`gui-appearance.json`).

- 01–03: aynı 1.600 kaynak eski sürümde bütçe reddi; Eksen seçimi de reddi gidermez.
- 04–06: yeni sürümde aynı seçim; bütün 40×40 grid önizlemede. 3.276 duvar,
  3.120 ortak parça/10.920 m uyarısı, Eksen/0.200 m/Sil görünür. Sağ tuş
  Onayla / bitir seçeneğine tek tıklama 1.600 kaynağı 3.276 normal duvara dönüştürür.
- 07–14: tek undo/redo, kaydetme, tekerlekle büyütme, gerçek Kaydır aracı,
  Home ve kapatıp yeniden açma. Nesne sayısı, renk, birleşimler ve 1:100 kontrolü.
  İlk teker çağrısı çok fazla büyüttü (09); Home sonrası küçük hareketle
  düzeltildi (10). Odak dışı `P` denemesi Kaydır'ı açmadı ve sürükleme 24
  duvar seçti (11); şeritten gerçek Kaydır açılıp gezinme doğrulandı (13).
- 15–19: yedi karma kaynak birlikte seçildi; dokuz duvarın tamamı, Koru ve
  ortak kenar uyarısı önizlemede; tek sağ tuş onayı, 7→16, tek undo/redo ve kayıt.
- 20–21: aynı kaynaklarda İç net seçilince açık ret. Onay denemesi de
  hiçbir duvar yazmaz; kaydedilen dosya yedi özgün kaynakla aynıdır.
- 22–24: Eksen/Sil ile 7→9; bir sağ tuş onayı, kaydet/kapat/aç. Daire ve
  yaylı poligon analitik duvar olarak görünür.
- 25–26: gerçek 1100×650 pencerede Home bütününü sığdırır; daire duvarına
  fareyle seçim, Duvar bağlamsal şeridi ve özellikleri erişilebilir.
- 27–33: `Infallible` düzeltmesi dahil son kaynak yeniden derlendi
  (`gui-binary-final-source.json`). Kullanıcının kilidi açmasının ardından
  pencere sistemin büyüt eylemiyle ekrana getirildi ve kenarlarından gerçek
  1100×650 boyutuna ayarlandı. Fareyle 1.600 kaynak seçildi; yöntem ve referans
  görünür alanlardan değiştirildi. Tam 3.276 önizleme, sağ tuş menüsüne bir
  tıklamayla onay, tek undo/redo ve her aşamada kayıt yeniden doğrulandı.
  Son programın üç KCAD çıktısı da önceki geometriyle bağımsız karşılaştırılır.

Geniş iç pencere 1440×828 (ham JPEG 2880×1720); dar iç pencere 1100×650
(ham JPEG 2200×1364). Başlık 32 mantıksal pikseldir. Görüntüler ham CUA JPEG'leridir;
yeniden boyutlandırılmış yapay ekran değildir. `gui-events.json` elle tutulan
adım/ekran kaydıdır, uygulamanın kendi olay kaydedicisi değildir. Daraltmada
yuvarlak köşe dışına denk gelen bir sürükleme reddedildi; taze ekran üzerinden
kenarların ortasından daraltma tamamlandı. Bu kayıt tam 1440×900 × bütün tema/font
matrisi iddiası taşımaz.
27 numaralı ilk açılış görüntüsü 2880×1726'dır; son dar pencere görüntüleri
28–33 yine 2200×1364'tür. Erişim aksaması kabulün yerine sayılmadı;
kilit açıldıktan sonraki fare işlemleri ayrıca kaydedildi.

`python3 docs/baseline/2026-10-25-architecture-wall-networks/verify.py`, SDK/native
geometri kullanmadan bağımsız KCAD okuyucuyla bütün kaydedilmiş sonuçları denetler:
gridin 3.280 temel kenarı tam bir kez, toplam 11.480 m; karma dokuz duvarın tam
verisi, yayları, sahipliği, `115 + 8π` m; kaynak UUID/öznitelik/görünüm/ayar,
undo/redo, referans ret ve yeniden açma. Geçici sayısal `id`'ler silme sonrası
kayıtta sıkışabilir; kalıcı UUID ve nesne verisi karşılaştırılır.

## Performans ölçümünün kapsamı

`performance/` ham stdout, `/usr/bin/time -l` çıktıları, örnekler ve özetleri
tutar. `environment.json` kaynak sürümü, derleyici ve ölçülen program hash'lerini
kaydeder. Release test programı her çağrıda bilinen tam ağ sayısını/uzunluğunu
denetler. Her durum için beş yeni süreçte on çağrı: beş ilk/soğuk ve 45 sıcak
örnek. 100/500/1.000 odada sürümler süreç başına dönüşümlü çalışır.
“Soğuk”, yeni sürecin ilk hesap çağrısıdır; işletim sistemi önbelleği sıfırlanmaz.
Derlemeyle çakışan bir ön koşu kabul edilmedi; tabloda yalnız derlemeler ve
testler bittikten sonra alınan final ölçümü yer alır.

Süre yalnız `wall_network::build` hesabıdır; kaynak oluşturma, doğrulama ve
native belge/GUI dahil değildir. Bellek bütün sürecin tepe RSS değeridir;
girdi, test koşucusu, doğrulama ve tekrarlı çağrıları da içerir. Çağrı başına
net tahsis, GPU belleği veya GUI gecikmesi sayılmaz. 1.600/3.000 bitişik ve
10.000 ayrık odada eski yolun kabul ettiği eşdeğer bir başarı ölçümü yoktur;
eski reddin daha kısa süresi bir hız karşılaştırması değildir.

Tekrar ölçüm: release `wall_network_perf` test ikilileri hazırlanır;
`measure.py --before <eski-ikili> --after <yeni-ikili> --output <dizin>`.
Bu koşucu macOS'un byte cinsinden RSS çıktısını bekler; yalnız bağımsız
başarılı durumları zamanlar. Eski ret ve red/green testler ayrı loglardadır.


Son kaynakta ölçülen medyanlar (süre ms, süreç tepe RSS MiB):

| Durum | Eski ilk/sıcak | Yeni ilk/sıcak | Eski/yeni RSS |
|---|---:|---:|---:|
| 100 bitişik oda | 0.480/0.359 | 0.471/0.352 | 3.52/3.25 |
| 500 bitişik oda | 2.575/2.150 | 2.044/1.819 | 10.86/5.03 |
| 1000 bitişik oda | 6.234/5.090 | 4.117/3.889 | 30.67/7.23 |
| 1600 bitişik oda | Kabul yok | 7.449/7.144 | —/9.05 |
| 3000 bitişik oda | Kabul yok | 13.632/13.139 | —/18.19 |
| 10000 ayrık oda | Kabul yok | 36.479/35.517 | —/49.12 |

## Testler ve açık kapsam

Ham sonuçlar `logs/`, komutlar/çıkış durumları ve dosya hash'leri
`test-results.json` içindedir. İlk genel Python koşusunda yeni test sayısal
yerel id'yi kalıcı kimlik gibi karşılaştırdı; Sil sonrası açılışta id 8→1
olduğu için düştü. Test UUID ve bütün nesne verisine göre düzeltildi;
ilk başarısız kayıt saklandı. İlk clippy kontrolü sınırsız yoğun yolun
`expect` çağrısını reddetti; bu yol `Infallible` ile tip düzeyinde hatasız
hale getirildi. Lint kuralı gevşetilmedi. Son kaynak kontrolleri ayrıca kaydedilir.

Son kaynakta çekirdek **781**, native/interaction/MCP/desktop **2.302**,
yeniden derlenen Python FFI ile **280** test geçti: toplam **3.363** başarılı
test. Rust koşularında toplam 279 mevcut ignored test çalıştırılmadı; tam
workspace veya bütün ignored/GPU matrisinin geçtiği iddia edilmez. Seçili beş
paketin bütün hedeflerinde clippy `-D warnings`, masaüstü derlemesi ve optimize
WASM derlemesi geçti. Bağımsız duvar referansları, SDK, komut manifesti,
şerit ve bağımlılık yönü kontrolleri de başarılıdır.

Tam gerçek tema/font/pencere matrisi, native komut/GUI/GPU uçtan uca
gecikme/bellek/süre ölçümleri ve genel mimari yol haritası açıktır. Çekirdek
ölçümü bu kapıları kapatmaz. Etkin/görünür kat yönetimi ve mahalın katı,
yalnız dönüşümün hedef katıyla tamamlanmış sayılmaz.
