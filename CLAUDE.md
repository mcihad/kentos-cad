# KentOS CAD — Geliştirme kılavuzu

Güncelleme: 25 Eylül 2026. Bu dosya günlük geliştirme kurallarını ve mevcut
kodun sınırlarını tutar. Ayrıntılı gelecek işleri ve kabul kapıları
[TODOS.md](TODOS.md), görsel kurallar [DESIGN.md](DESIGN.md), karar gerekçeleri
[docs/adr/](docs/adr/) içindedir. Tamamlanan dilimlerin uzun geçmişini burada
tekrarlamayın; kod, test ve ilgili ADR'ye bağlantı verin.

Mevcut özellik ile hedefi ayırın. Kodun bulunması testlerin bu oturumda
çalıştırıldığı veya üretim kabulünün tamamlandığı anlamına gelmez.
Çelişkide kodu doğrulayın; ürün yönünde son kullanıcı kararlarını ve güncel
TODOS.md sınırlarını izleyin. Eski belgelerdeki bu dosyayı değiştirmeme veya
eski fazları izleme notları, kullanıcının bu sadeleştirme ve kapsam güncellemesiyle
aşılmıştır. Bölüm numaraları mevcut kod/ADR atıfları için korunmuştur.

## 0. Güncel karar özeti

- Web bağımsız TypeScript/DOM uygulamasıdır; desktop'ın web export'u değildir.
  Yalnız hesaplama/codec kütüphaneleri dar WASM bağlayıcılarıyla kullanılır.
  Web'in belge, komut, etkileşim, settings ve render orkestrasyonu TS'te kalır.
- Desktop Rust ile; `kentos-rc` 25 Eylül'de bu depoya `crates/ui`
  (`kentos-ui` / `kentos_ui`) olarak geçmişiyle alındı (ADR 0016). İlk masaüstü
  kabuğu `apps/desktop`'tadır (ADR 0017); ana CAD çizim alanı `crates/render/wgpu`
  ile Iced'in aygıtında çizilir (ADR 0019); araçları kapalı alan, çizgi, çoklu çizgi
  (ADR 0021, 0027), nokta, daire, yay, dikdörtgen, döndürülmüş dikdörtgen ve düzgün
  çokgendir (ADR 0032); elips, eğri, yardımcı çizgi, ışın, paralel çizgi, dik in ve dik çık, halka,
  revizyon bulutu, kot noktası ve böl (ADR 0057); yazı ve çizimin üstündeki yazı kutusu (ADR 0060);
  ölçülendirme: hizalı, doğrusal, açı, yarıçap ve çap (ADR 0061), koordinat, yay uzunluğu, kırıklı yarıçap, semt ve eğim, Açı'nın yaydan ve daireden yolları, döndürülmüş doğrusal, ölçü değerinin zemini, Hızlı ölçü; DXF'in koordinat ölçüsü, ARC_DIMENSION'ı ve LARGE_RADIAL_DIMENSION'ı gelir ve gider, semt ve eğim hizalı ölçü olarak KentOS verisiyle (ADR 0147); tarama: kapalı nesneyle ya da çizgilerle, adalarıyla (ADR 0062); köşe kotu: çizginin, çoklu çizginin ve alanın köşe kotları, Kot ver, Öznitelikler'in kot ve 3B uzunluk satırları (ADR 0142); çok parçalı alan: parçalar ve delikleri, Parçaları birleştir, Parçalara ayır, alan işlemlerinde Tek nesne (ADR 0143); blok: Blok oluştur, Blok ekle, Bloklar paneli, Blok öznitelikleri penceresi, Öznitelikler'de yerleştirme ve blok öznitelikleri, Patlat; DXF'in blokları tanım ve yerleştirme olarak gelir ve gider, öznitelikleri ATTDEF ve ATTRIB olarak, “Blokları patlat” seçeneğiyle (ADR 0144); yazı ekleri: yazının ve öznitelik tanımının on iki noktalı hizası ve genişlik çarpanı, yazının zemini, Yazı'nın Hiza, Genişlik, Zemin ve Artır seçenekleri, Okunur yap, Bul ve değiştir, Metin dosyası yerleştir; DXF'in 72/73'ü, 41'i, MTEXT'in yerleşim noktası ve zemini, NCZ'nin çapaları tahminsiz (ADR 0145); kılavuz: ok, kırık çizgi, kol ve not tek nesne, Kılavuz aracı (Ok, Yükseklik, Zemin), Öznitelikler'in Kılavuz bölümü, notun yerinde düzenlenmesi; DXF'in LEADER'ı bağlı MTEXT notuyla ve MULTILEADER gelir, kılavuz LEADER ve MTEXT olarak gider (ADR 0146);
  alan işlemleri: birleştir, kesiştir, çıkar, böl, alana ve çizgiye çevir, içine tıklayarak alan (ADR 0065);
  topolojik temizlik: uçlar ve köşeler var olan köşede birleşir, kısa uç uzar, taşan uç budanır, yazılan toleransla, önizlemeli tek adım (ADR 0148);
  topolojik düzenleme: durum çubuğundaki Topoloji açıkken tutamaç, tutamaç menüsü ve Esnet görünen ve kilitsiz komşuların ortak köşe ve kenarlarını da tek adımda değiştirir, kart ortak köşeyi sayar, Noktalar da seçeneğiyle (ADR 0160);
  çakışma denetimi: durum çubuğundaki Çakışma açıkken yeni alan (Kapalı alan, Parsel oluştur, Dikdörtgen, Düzgün çokgen, Daire dilimi, Alan olarak çiz) kendi katmanındaki ya da seçili katmanlardaki görünen alanlarla örtüşen kısmı çıkarılarak yazılır; Bitişik alan: yalnız yeni sınır çizilir, yolun görünümdeki komşu alanlarla kapattığı bölge imleçle dolar ve tek alan olarak yazılır, komşuların içindekiler delik; Topoloji açıkken yeni alan komşularıyla köşe köşe bağlanır, aynı adımda (ADR 0162);
  kenet ekleri: Ağırlık merkezi, Uzantı, Paralel ve Karelaj türleri (karelaj aralığı doğu ve kuzey), çizilmekte olan yola kenet, ölçek aralığında kenet; durum çubuğundaki Kenet hücresinin sağ tık menüsünde türler tek tek ve karelaj aralıkları; uçta durmak uzantısını, kenarda durmak doğrultusunu alır, yazılan mesafe uzantı ve paralel boyuncadır; Katmanlar'da katmanın kendi keneti (mıknatıs, Kenet ▸; `.kcad` şema 10) (ADR 0163);
  sayısallaştırma kilitleri: değer kartında sayı ve Tab ile uzunluk, `<açı` ile doğrultu (CBS'de semt), Sapma, Nesneye paralel ve dik (kenar ya da yay seçilerek), Kalıcı, Esc önce kilitleri kaldırır; Dik açı ve kapalı alanlarda Dik kapat (D); Referans noktası ve Yapım kipi; çizimde kesikli kılavuzlar, seçilen kenar ve “R”, sağ tıkta Kilit ▸ (ADR 0166);
  ikinci koordinat sistemi: projenin ayarı (`.kcad` şema 12), durum çubuğunda, Koordinat oku'da, Mesafe ölç ve Alan hesapla'da ikinci sistemin değerleri doğruluklarıyla (EPSG'nin yolları, “resmî dönüşüm değil”), coğrafide DMS ya da DD; Koordinat dönüştür (tek nokta, Çizimden, liste, panoya ve CSV; `crs.transform`) (ADR 0167);
  saha verisi: Leica GSI, Sokkia SDR, Topcon GTS-7, Nikon RAW, Trimble JobXML ve CSV/TXT karne (alet dosyası içerikten tanınır, metin karnede sütun eşleme), Karne editörü (gözlemler, Kullan, nokta adı;
  iki durumun indirgenmesi, indeks hatası, yatay uzunluk ve kot farkı yer eğriliği ve refraksiyonla; toleransı aşan fark; poligonun
  istasyon zinciri ve iki yönden kenarları), Kutupsal alım'a ve Poligon hesabı'na aktar; Proje ayarları › Ölçme: k (0,13), iki durumun
  ve poligonun toleransları (`.kcad` şema 14, 15), Kutupsal alım da k'yı uygular, Poligon hesabı kapanmaları toleranslarla karşılaştırır;
  zemin, elipsoit ve düzlem: Proje ayarları › Ölçme'de ortalama elipsoit yüksekliği (`.kcad` şema 16), yazılıyken Mesafe ölç ve Alan
  hesapla'nın elipsoit üstündeki (jeodezikler, düzlemin ölçeği) ve zemindeki (yükseklik çarpanı) satırları; “Uzunlukları projeksiyona
  indir” açıkken Kutupsal alım ve Poligon hesabı ölçülen uzunlukları düzleme indirir, Aplikasyon zemin uzunluklarını da verir (ölçek ve
  yükseklik çarpanı raporda); çekirdekte noktanın ve çizginin ölçeği, GeographicLib'in libm'li kopyası (ADR 0171);
  köşe tablosu: alt panelin Koordinat listesi tek çizgi, çoklu çizgi ya da alanda düzenlenir: Halka, Y, X, Z, işaretli Yarıçap, Kenar,
  Semt; seçili satırların köşeleri çizimde halkalı, yerinde düzenleme (“Köşe düzenle”), Satır ekle, Sil ve Delete; Topoloji açıkken
  komşular da (ADR 0172);
  delikler: Delik ekle (halka ilk köşesinin içinde olduğu alana, seçiliyse ona; var olan deliğe değen halka onunla birleşir, sınırı aşan
  reddedilir), Deliği sil ve Deliği doldur (imlecin altındaki delik vurgulu; dolduran yeni alan deliğin kotlarıyla, delik kalır),
  şeritte Delikler paneli; Sürdür: çizgi ya da çoklu çizgi ucundan (seçiliyse imlece yakın ucundan) sürer, ilk yay uçtaki teğetle,
  eski köşeler kotlarıyla; Biçim değiştir: alanın ya da çizginin üstünden çizilen düz kenarlı hatla, alanda kesim büyük parçayı bırakır,
  dışarıdan dolanan hat cebi ekler, çizgide buluşmalar arası değişir; önizleme yeni boyutu ve değişimi gösterir (ADR 0173);
  GNSS içe aktar: GPX ve NMEA konumları WGS 84'ten projenin sistemine doğruluğu ve dayanağıyla, adlı noktalar olarak (türler, adsızların ön eki ve
  numarası, kot elipsoit yüksekliği; çözüm, uydu, HDOP, zaman ve yükseklikler öznitelik), sistemi olmayan projeye alınmaz; Cihaza gönder: seçili
  noktalar, Aplikasyon'un ve Nokta editörünün noktaları Leica GSI-16 ve GSI-8, Topcon GTS-7, Trimble JobXML, Nikon RAW ya da CSV olarak, taşınamayan
  nokta nedeniyle söylenir (SDR33 sahibin örneğini bekliyor) (ADR 0169);
  özel koordinat sistemi: projenin ya da ikinci sistemin tanımı (TM, coğrafi, taban sisteme bağlı yerel; kayıttaki ya da projenin datumu, WGS 84'e 7 parametre; `.kcad` şema 13), projenin datum dönüşümleri (7 parametre ya da cihazın NTv2 ızgarası), Izgaralar; Özel koordinat sistemi penceresi: WKT ve PROJ'dan al ve kopyala, Kayıttakini seç, Deneme noktası, Ortak noktalardan hesapla (ADR 0168);
  izleyerek çizim: yol aracının İzle (İ) düğmesi açıkken çizgiye yakın tık çizginin üstüne oturur, iki nokta arası görünen çizgiler boyunca kısa yoldan, köşeleri ve yaylarıyla; Birleştir'in Zincir (Z) seçeneği tıklanan çizginin bağlı zincirini tek çoklu çizgi yapar; yol aracının Akış (A) düğmesi açıkken imleç Adım boyu (B) kadar ilerledikçe köşe bırakır (ADR 0161);
  toplu alan: çizgilerin kapattığı bütün bölgeler tek adımda alan olur, içteki yazı ya da adlı nokta özniteliği; etiketsiz, çok etiketli bölgeler ve boşta uçlar söylenir (ADR 0151);
  ölçü noktası: Nokta'nın Ad, Kod ve Kot'u, ad her noktada artar, aynı yerde nokta varsa Düzelt, Ekle ya da Atla, `#ad` ile adlı noktanın yeri, Köşelere nokta (ADR 0152);
  mesafe ölç, alan hesapla ve parsel oluştur (ADR 0067); seçili nesnelerin tutamaçları ve üzerine gelme kartı (ADR 0068);
  Hesap pencereleri: poligon hesabı, kutupsal alım, önden ve geriden kestirme, aplikasyon (ADR 0070, 0071);
  Vektör oturtma: kontrol noktalarından Helmert, afin ya da projektif dönüşüm, artıklar ve m0, Adla eşle, Kullan ile çift çıkarma,
  ya da Parametrelerle (taban noktası, Y ve X ölçeği, dönüklük, öteleme), ya da Kauçuk levha (bağlardan tam geçen ince plaka eğrisi,
  Sabit noktalar, Helmert'e göre yerel düzeltmeler; ADR 0158); seçili nesnelere, katmana ya da bütün çizime (kopya olarak da)
  `cad.entities.transform` ile tek adımda (ADR 0156);
  kenar eşleme: komşu paftaların kenarında buluşmayan çizgiler, çizginin devamı ölçütüyle (arama uzaklığı, açı toleransı, isteğe bağlı
  öznitelik ya da katman adı) eşlenir; komşunun ucunda, ortada ya da pafta sınırında buluşur; ucu taşı, parça ekle ya da köşeleri ayarla;
  bağlar tablosu (Kullan, Göster), `cad.entities.edit`'in `edgematch` işlemiyle tek adımda (ADR 0159);
  İşlemler: dört yerleşik araç ve Parsel ölçü yazıları modeli, tanımdan üretilen penceresiyle (ADR 0084);
  taşı, kopyala, döndür, ölçekle ve aynala (ADR 0037);
  ötele, buda, uzat, köşe yuvarla, pah, kır, birleştir, patlat, uzat-kısalt, köşe ekle/sil, esnet, dizi,
  kutupsal dizi ve hizala (ADR 0047);
  seçim, kenet ve silme (ADR 0029), pano (kes, kopyala, yapıştır, yerine yapıştır), kaydır, pencere
  ve seçime yakınlaştır ve son komutu yinele (ADR 0056); alt panel (komut geçmişi, koordinat listesi,
  uyarılar; saatli satırlar ve rozet, durum çubuğunda son ileti, ADR 0114), F4 ile paneller, tam ekran, komut arama ve sunucu denetimi (ADR 0058), çizim alanının
  sağ tık menüleri ve tek seferlik kenet vardır (ADR 0059); çizimin
  yazıları (yazı nesnesi, ölçü değeri, etiket; projenin yazı tipiyle), ölçü çizgileri ve yardımcı
  çizgiler çizilir (ADR 0055).
  Web özellikleri envanter üzerinden adım adım masaüstüne taşınır. İki platformda da yalnız şerit
  arayüzü vardır; web'in klasik arayüzü (menü çubuğu, araç çubuğu, kayan araç kutusu) kaldırıldı
  (sahibin kararları, 27 Eylül ve 2 Ekim; ADR 0155).
- Web WebGPU/WebGL2 renderer'larını korur. Uygun WGSL kaynakları native ile
  paylaşılabilir; native Iced/application/wgpu runtime'ı web'e derlenmez.
- Server'ın ana görevi kişisel/kurumsal proje saklama, erişim, yetkilendirme,
  sürümleme, senkronizasyon ve paylaşımdır. Martin rolü şimdiki kapsamda yoktur;
  olası tile/harita servisi ayrı bir gelecek kararıdır.
- Hem CAD hem GIS projeleri PostGIS'te saklanıp çalışılabilecek. CAD tanımı
  sessizce GIS polyline'ına indirgenmeyecek; kaynak ve türev ayrımı korunacak.
- `.kcad` yalnız kayıt/yükleme için açık, sürümlü binary snapshot'tır (KCAD v2, ADR 0025,
  `docs/specs/kcad-v2.md`); DB, SQL, kalıcı sorgu indeksi veya tile dosyası değildir.
  Web ve masaüstü v2 yazar; eski v1 JSON okunur, göçü kimlikleri ve kaynağı kaydeder.
  Cloud dosyaları da aynı formatı kullanacak.
- Python embed/SDK ve AI yüzeyi sürümlü command sözleşmelerini kullanacak;
  yetki/transaction kurallarını atlayan ayrı mutasyon yolu kurulmayacak.
- Yeni mimariyi bir seferde baştan yazmayın; çalışan web'i koruyan dikey dilimler
  ve TODOS.md'deki kabul kapılarıyla ilerleyin.

## 1. Ürün ve mevcut durum

KentOS; harita mühendisliği, kadastro, imar ve hassas CAD/GIS çalışmaları için
geliştirilir. 18 uygulaması, imar planı, ifraz/tevhit, yol, mimari ve dijital
ikiz projeleri gelişim yönüdür; hepsi bugün tamamlanmış ürün değildir.
Web ve native masaüstü hedeflenir; mobil/dar ekran şu an öncelik değildir.
Web kabuğunun mevcut alt sınırı 1100×600 px'tir. Arayüz Türkçe; kod,
tanımlayıcılar ve kod yorumları İngilizcedir. Marka KentOS, başlık KentOS CAD'dir.

| Kodda bulunan temel | Kaynak |
|---|---|
| TS/DOM web, komut/araç kataloğu, dinamik giriş, WebGL2/WebGPU | `apps/web/src/` |
| Rust geometri, sayısal politika, pick/snap deposu, stil/ifade, SVG ve format hesapları | `crates/shared/` |
| İfade dili (`kentos-expression`, ADR 0100): alanlar, `$` değerleri, işlevler, JavaScript sayı/metin anlamı (`js/`); bir kez derlenen, sabitleri katlanmış, aynı işi bir kez yapan program 256 nesnelik partilerle sütun sütun değerlendirilir (`program`, `exec`, `kernels`; kurallar `scalar`, `functions`), tek nesne aynı kurallarla ağaçtan (`walk`); adlar çağıranın şemasıyla çözülür (`compile_with`: tipli kullanıcı alanları, yoksa metin özniteliği), değerler çağıranın sütunlarından parti parti gelir (`host::Objects`), geometri değerleri (`$alan`, `$merkez_y`, `$min_y` … `$genişlik`) şekillerden yalnız okunanlar ve nesne başına bir kez (`geometry::Shapes`; web'de depoda, `GeometryStore.evaluateExpression`), bağımsız başvuru `fixtures/expression/v2/geometry.json`; İfade oluşturucunun dil hizmetleri (`editor`: sözcük türleri, hata ve uyarı aralıkları, tamamlama, imza, parantez eşi, ağaç, yardım, değerler; web'e `api` ile), iki platformda `fixtures/expression/v2/builder.json` (ADR 0100 §5); aynı metnin akışı (`editor::flow`, ADR 0101): metin durumdur (`?` boş giriş), düğümler, yerleri ve değişiklikler tek çekirdekten, iki platformda `fixtures/expression/v2/flow.json`; dil ekleri (ADR 0100 §4): `durum eğer … ise … yoksa … son` (CASE), `içinde`, `arasında`, `gibi`, `benzer`, `boş` / `boş değil` (IS NULL), `^` ve dokuz işlev, değerleri bağımsız Python başvurusuyla `fixtures/expression/v2/language.json` iki platformda (`scripts/fixtures/expression_language.py --check`), yeni dilde sütun motoru ile tek nesne yolunun rastgele karşılaştırması; `kentos_style_core::expr` ve `::js` onun yeniden dışa aktarımıdır; dondurulmuş yanıtlar `fixtures/expression/v1` iki platformda, eski ağaç değerlendiricisiyle rastgele karşılaştırma `tests/differential.rs` | `crates/shared/expression/`, `fixtures/expression/` |
| Dar WASM bağlayıcıları ve Rust → TS sözleşme üretimi | `crates/wasm/`, `crates/shared/contracts/` |
| Belge transaction/rollback, undo/redo; katman ya da grubu nesneleriyle tek geri alınabilir adımda silme, retleriyle (ADR 0072); yerel `.kcad`: KCAD v2 yazılır (biçim işçisinde doğrulanır), v1 JSON okunur (ADR 0025); tipli işçi sınırı, aşamalı ve durdurulabilir açılış, kayıt hataları, kaydedilmemiş işin yerel kurtarma kopyası (ADR 0030) | `model/document.ts`, `model/snapshot.ts`, `app/fileIO.ts`, `app/drawingFile.ts`, `app/fileAccess.ts`, `app/recovery.ts`, `io/kcad.ts`, `io/columns.ts`, `ui/io/OpeningDialog.ts` |
| KCAD v2 kodeki (kap, CBOR profili, şema, koklama), `kcad` aracı; bağımsız Python okuyucusu ve örnek dosyalar | `crates/shared/kcad/`, `tools/kcad/`, `fixtures/kcad/v2/`, `docs/specs/kcad-v2.md` |
| Axum/Tokio/SQLx API, PG/PostGIS, kimlik/tenant, `project.changes`, audit/outbox; proje sahipliği, kişisel alan ve paylaşım (ADR 0015); proje kataloğu ve yaşam döngüsü komutları, migration 0005 (ADR 0028); dosya olarak saklanan proje: doğrulanan yükleme, değişmez KCAD v2 revizyonları, klasör nesne deposu, migration 0006 (ADR 0031); veritabanı projesinin tek anlık `.kcad` görüntüsü (ADR 0033); kontrol noktaları (oluştur, listele, indir, sil) ve yeni proje olarak geri yükleme, migration 0008 (ADR 0034); bağlantıyla davet ve misafir, migration 0009–0010 (ADR 0035); yüklenen `.kcad`'in boş veritabanı projesine tek işlemde aktarımı (ADR 0036); öbür saklama biçimine yeni proje olarak dönüştürme (ADR 0039); katmanı düşen ağaç, katmanda nesne kaldıkça `@project` çakışmasıdır (ADR 0072); blok tanımları, migration 0012: `project.changes`'in `blocks`'u (`block:<id>` sürümleri), yerleştirmenin izdüşümü açılımın GeometryCollection'ı, `GET …/blocks` (ADR 0144 §5) | `apps/api/`, `crates/server/` |
| İfade oluşturucu (ADR 0100 §5, DESIGN.md §7.16): QGIS'in ifade penceresi gibi; İşlemler'in ifade alanının yanındaki ε ile açılır: renkli düzenleyici, tamamlama (Ctrl+Boşluk), imza, hata yerinde, işleç düğmeleri, nesne nesne önizleme, aranan ağaç, yardım ve alan değerleri; Tamam alana yazar. Masaüstünde önizleme nesnesi çizimden seçilir (`PickObjects::one`). Metin \| Akış (ADR 0101): aynı ifade düğümlerle; ağaçtan sürükleyip bırakma, çıkışı girişe bağlama, düğüm başına değer, denetçi, geri alma; her değişiklik metnindir | `ui/expression/`, `model/expression/builder*.ts`, `model/expression/flow.ts`, `apps/desktop/src/expression/` |
| Web cloud aç/yükle, autosave, IndexedDB taslak, WS/reconnect ve conflict, paylaşım penceresi, “Benimle paylaşılanlar”, açık projede rol/erişim değişikliği (ADR 0024), proje kataloğu: listeler, sunucuda arama/sayfalama, bilgiler, kopya, arşiv, çöp kutusu (ADR 0028); dosya projeleri (aç, Kaydet ile revizyon, çakışma), `.kcad` indirme, tek içe aktarımla yükleme, geçmiş ve kontrol noktaları, dönüştürme (ADR 0038); e-postayla davet, tek gösterimlik bağlantı ve kabul sayfası (ADR 0042); 8 MiB üstü dosya parçalı ve kaldığı yerden yüklenir, kesilen indirme `Range` ile sürer (ADR 0045); veritabanı projesinde blok tanımları nesnelerle gider ve gelir: izleyicisi, çakışması, taslağı, veri silmeden üstün, ad bir kez (ADR 0144 §5, `syncBlocks.ts`) | `app/cloud/`, `ui/cloud/` |
| KentOS UI bileşenleri (Iced 0.14) ve vitrini | `crates/ui/`, `apps/ui-showcase/` |
| Masaüstü kabuğu: şerit, katmanlar, özellikler, komut satırı, `.kcad` aç (v1, v2; aşamalı, durdurulabilir) / kaydet (v2; kendi iş parçacığında, paneli ve durdurmasıyla; geçici dosya ve doğrulama), kurtarma kopyası (ADR 0030), geri al/yinele; wgpu çizim alanı (çizgi/eğri/dolgu/nokta, kaydır/yakınlaştır); kapalı alan, çizgi ve çoklu çizgi araçları, değer alanı ve web'in tuş anlamları; seçim (tıklama, Shift, pencere/kesişim, üzerine gelme), kenet (F3), Sil; seçili nesnelerin tutamaçları (sürükle, sıcak tutamaç, yazılan nokta, Enter/Esc; kenar ortası yeni köşe) ve üzerine gelme kartı (ADR 0068; kenarda imlecin öbür yanına geçer, uzun adı kaydırır, değer alanı ve ipuçları da aynı kuralla, ADR 0082), sağ tıkta tutamaç menüsü (ADR 0074); Nokta hesapla: komut nokta beklerken altı yapı (yan nokta, kenar ve doğru kesişimi, hat üzerinde, açı ve mesafe, orta), noktası askıdaki komuta gider; takma adla, komut menüsünden ya da komut satırının çipinden (ADR 0083); İşlemler: Köşe noktalarını numarala, Kenar uzunluklarını yaz, Öznitelik hesapla, İfadeyle seç ve Parsel ölçü yazıları modeli; web'in `ToolDialog`'u gibi tanımdan üretilen pencere (Girdi, Ayarlar, Çıktı, Gelişmiş; açıklama, önizleme, Nerede çalışır, takma adlar; Sonuçları seç, Geri al; Haritadan göster), komutları ve takma adlarıyla, tek geri alma adımında yazar, son değerler `islemler.json`'da; alanlar çizimden seçilir (Sahneden seç: nokta, numaralamanın başlangıç köşesi, girdi nesneleri alanın türleriyle; ADR 0088); sağ dokta Katmanlar'ın yanında İşlemler sekmesi: araç kutusu (arama, kategoriler, Modeller) ve oturumun geçmişi (Yeniden aç, n nesneyi seç) (ADR 0084); 2 000 ve daha çok nesneli iş arka plandaki bir iş parçacığında, ilerleme çubuğu ve Durdur'la; Nerede çalışır web'inki gibi (Otomatik, Bu bilgisayarda, Arka planda; ADR 0124); model bütünüyle çizimin kopyasında çalışır, adımları çizimde tek adımda yeniden oynatılır (ADR 0125); Model tasarımcısı: web'inki gibi akış diyagramı (girdiler, adımlar, bağlantılar, kaynaklarıyla ayarlar, kendi geri alması), Kitaplığa ve `islemler.json`'a kaydeder, hazır modeli kopyası olarak açar, kullanıcının modelleri araç kutusundan çalışır (ADR 0116); Hesap pencereleri: Poligon hesabı (bağlı, kapalı, açık; kapanma hataları ve dengelemesi), Kutupsal alım (kotlarıyla), Önden ve Geriden kestirme (krokisiyle), Aplikasyon; web'inki gibi ölçü tablosu (Enter, ↑/↓, satır ekle/sil, elektronik tablodan yapıştırma; ADR 0075); bilinen nokta adla, Y,X ile ya da çizimden; rapor sistem panosuna, noktalar `cad.entities.create` ile pencerenin adındaki tek adımda çizime (ADR 0070, 0071); web'inki gibi düzenlenen Öznitelikler paneli: katman, renk, sembol, türe göre geometri değerleri, öznitelikler, çoklu seçimin ortakları ve toplamları (ADR 0063), ürün komutlarıyla yazar (ADR 0066); nokta, daire, yay, dikdörtgen ve düzgün çokgen araçları, şeritte yöntem menüleri; elips, eğri, yardımcı çizgi, ışın, paralel çizgi, dik in ve dik çık, halka, revizyon bulutu, kot noktası ve böl (ADR 0057); Yazı ve çizimin üstündeki yazı kutusu, çift tıkla yazı ve ölçü değeri düzenleme (ADR 0060); Ölçülendirme (hizalı, doğrusal, açı, yarıçap, çap) ve komut satırında değerlerin yazıldığı gibi yankısı (ADR 0061); Tarama: kapalı nesneyle ya da çizgilerle, adalar, sınır katmanı, dört desen (ADR 0062); alan işlemleri: birleştir, kesiştir, çıkar (iki seçimle), böl (noktalarla ya da çizgiyle, canlı parçalarıyla), alana ve çizgiye çevir, içine tıklayarak alan (ADR 0065); Mesafe ölç, Alan hesapla ve Parsel oluştur: yol aracının biçimleri, parsel `parsel` katmanına numarasıyla, tapu alanı boş (ADR 0067); taşı, kopyala, döndür, ölçekle ve aynala (ADR 0021, 0027, 0029, 0032, 0037); ötele, buda, uzat, köşe yuvarla, pah, kır, birleştir, patlat, uzat-kısalt, köşe ekle/sil, esnet, dizi, kutupsal dizi, hizala (ADR 0047); PiriCAD'in adlarıyla, KentOS'un mekaniğiyle iki platformda: tüm köşeleri yuvarla, tüm köşelere pah, Parçala (kesişimlerden, eşit parçalara, uzunluktan), Yönü çevir, Sadeleştir, Çizimi temizle, Özellik kopyala, daire dilimi, ara nokta, kesişim noktası (iki uzaklık, iki doğrultu, iki doğru), açı ölç, koordinat oku, zincir ve baz ölçü, Buda ve Uzat'ın çiti, Ötele'nin iki yana ve kaynağı sil seçenekleri, yol boyunca dizi (`cad.entities.array` `path`); hesapları çekirdekte (`ops::reshape`, `ops::split`, `tools::construct`, ADR 0140); Netcad planının hızlı P0'ları iki platformda (ADR 0141): önceki ve sonraki görünüm (oturumun 30 görünümlük geçmişi), Kapsam denetimi (çizimden çok uzak nesneleri seçer), Katmana ve Gruba yakınlaştır; Mesafe ölç'ün sabit ilk noktası, Alan hesapla'nın İçine tıkla ve Alan olarak çiz'i, Dik ayak ölç (Prizma); Çitle, Daireyle ve İçeren alanı seç (Seç ▾); Netcad adları (ALANSOR, CETVEL, XYZSOR, LIMITBUL …); depo sorguları çekirdekte (`store/select.rs`); çizimin üstünde komut şeridi (`drafting.commandBar`); yeni katman ve grup, DXF ve koordinat listesi al/ver (ADR 0048); Netcad NCZ al: akıllı nesneler sembolleriyle ve katmanları en üstte, paftalar dosyanın dilimindeki gerçek, dönük çerçeveleriyle, büyük DXF ve NCZ kare kare tek geri alma adımında, panel ve Durdur ile (ADR 0138; dağıtım koşulu TODOS.md `NCZ-01`); nesnenin kendi çizgi kalınlığı (DXF 370, NCZ kalemi; `.kcad` şema 3; ADR 0139); köşe kotu (çizgide `za`, `zb`; yolda ve deliklerinde `zs`, kotsuz köşe null; `.kcad` şema 4), düzenlemeler kotu çekirdeğin kurallarıyla taşır (`ops::elevation`, taşıyamayan uyarır), Kot ver (Sabit, Artır, Sıfırla; `cad.entities.edit`'in `elevation` işlemi), Öznitelikler'in Kot, 3B uzunluk ve 3B çevre satırları, tutamaçta, Koordinat oku'da ve üzerine gelme kartında kot; DXF, Shapefile, GeoJSON ve NCZ kotu taşır (ADR 0142); çok parçalı alan (`parts`, `.kcad` şema 5): çizim, seçme, kenet, tutamaç ve düzenlemeler parça parça, Parçaları birleştir, Parçalara ayır, birleştir, kesiştir ve çıkarda Tek nesne (T), Öznitelikler'de Parça sayısı, kartta Parça; GeoJSON MultiPolygon, Shapefile kaydı tek alan, DXF'e parça başına çoklu çizgi, PostGIS MultiPolygon (ADR 0143); GeoJSON al/ver, Shapefile al (dosyaları ya da `.zip`, ADR 0053); yeni proje sihirbazı (tür, koordinatlar ve il, ölçek ve ayrıntılar; web'inkiyle aynı kurallar `kentos_project::wizard`, ADR 0165 §3) ve proje ayarları (ADR 0049); uygulama menüsü, Başlangıç ekranı ve son dosyalar (ADR 0050); pencereye sığan şerit (paneller adım adım küçülür, seyrek araçlar ▾) ve Görünüm sekmesinde tema, vurgu, çizim zemini, yazı tipleri ve yazı boyutu (ADR 0051); Uygulama ayarları → Görünüm web'inki gibi: tema kartları, vurgu örnekleri ve özel renk, yazı tipi kartları, yazı boyutu adımları (ADR 0128); koyu ve açık tema web'in paletiyle, bir testle `tokens.css`'e bağlı (ADR 0129); şeridin hızlı erişim çubuğu (▾ menüsü, sağ tıkla ekleme ve çıkarma), sağ tık menüleri ve bölünmüş düğmenin son seçimi, `yerlesim.json`'da (`fixtures/shell/v1/ribbon.json`, ADR 0117); şeridin harf ipuçları (F6, tek başına Alt; kuralları `keytips.rs`) ve daraltılmış şeridin sekmesinin çizimin üstünde açılması (ADR 0118); şeridin kendi panelleri: Giriş'te Katmanlar (etkin katman) ve Özellikler (yeni nesnelerin rengi, çizgi tipi ve kalınlığı, çizim ölçeği), seçim varken sayısıyla bağlamsal Seçim sekmesi; çizim araçları etkin rengi komutun girdisine yazar (ADR 0089); komutlarda web'in ikonları (envanterden) ve §7.8'e uyan ipucu (ADR 0054); çizimin yazıları (yazı nesnesi, ölçü değeri, etiket; projenin yazı tipiyle, haleli, seyreltilmiş), ölçü çizgileri ve görünüme kırpılmış yardımcı çizgiler (ADR 0055); pano: Kes, Panoya kopyala, Yapıştır (aracıyla) ve Özgün koordinatlara yapıştır, oturumun panosuyla (sistem panosu yok); Kes `cad.entities.delete` ile, yapıştırma `cad.entities.create` ve `cad.entities.set` ile yazar (ADR 0074); Kaydır, Pencere yakınlaştır, Seçime yakınlaştır, Son komutu yinele (ADR 0056); alt panel (F2: komut geçmişi, koordinat listesi, uyarılar; temizle, kapat, sürükleyerek boy; web'in günlüğü: saatli ve düzeyli satırlar, 500 satır, Uyarılar rozeti, tek satırlık komut satırı ve durum çubuğunda son ileti, ADR 0114); web'in kabuk düzeni ve kalıcı yerleşimi: dok bütün yükseklikte, alt panel çizimin altında, boyutlar web'in kurallarıyla, `yerlesim.json` (ADR 0115), çizimde ızgara (F7) ve durum çubuğunda çizim yardımcıları, dar pencerede çekilen hücreler (ADR 0080), seçimi izleyen, web'inki gibi katman ağacı (etkin katman değişmez; göz, kilit, renk menüsü, çift tıkla etkin yapma, sağ tık menüsü, yeniden adlandırma, katman ya da grubu nesneleriyle tek adımda silme, ADR 0072; Katman ara ve ağacın klavyesi, ADR 0075), durum çubuğu arayüzün yazısıyla, F4 ile katman ve özellikler panelleri, tam ekran (sekme satırında düğmesi), Komut ara (Alt+Q; sekme satırındaki kutu, sonuç listesi, şeritte yerini gösterme, ADR 0077), Koordinat sistemi… ve sunucu denetimi (ADR 0058); sekme satırında web'inki gibi kaydedilmemiş noktası, Komut ara kutusu, koordinat sistemi düğmesi ve Yardım menüsü, dar pencerede adım adım (ADR 0064, 0077); çizim alanında web'in üç sağ tık menüsü (kısa tık, basılı tutma, Shift) ve tek seferlik kenet (ADR 0059); çizim alanında web'in artı imleci (kol boyu `appearance.crosshair`, nesne isteyen araçta kısa kollu ve seçim kutulu), kuzey oku ve ölçek çubuğu (ADR 0110); Nesne izleme: kenette durarak nokta alma ve bırakma, hizalara ve kesişimlerine kilitlenme, hizada yazılan mesafe, Shift+F3 (ADR 0085); proje türünün şeridi (her türün kendi şeridi envanterden), durum çubuğunda tür ve türsüz projenin sorusu (ADR 0052, 0165); bulut arayüzü (ADR 0041): giriş, katalog ve bu cihazdaki projeler, çevrimiçi ya da yerel kopyadan açma, dosya projesinin revizyonu, veritabanı projesinin kendiliğinden kaydı ve cihaz taslağı, başkalarının değişiklikleri, çakışma, buluta yükleme; açık projeyi yeniden adlandırma, çöpe taşıma, son revizyonu açma ve buluta dosya olarak kaydetme (ADR 0073); katalogda web'in yedi listesi (çöp kutusu dahil), arama, tür, sıralama ve kurum seçimi, seçili projenin bilgileri, favori, arşiv, çöp kutusu, geri yükleme, kalıcı silme ve `.kcad` indirme, web'in planıyla ve cihazın saatiyle (`fixtures/cloud/v1/catalog.json`, ADR 0086); Proje bilgileri, Kopyasını oluştur ve öbür saklama biçimine çevirme formları (`fixtures/cloud/v1/forms.json`, ADR 0112); Geçmiş sekmesi: kontrol noktaları ve revizyonlar, oluşturma, indirme, silme, yeni proje olarak geri yükleme ve açma, olayları bekleyerek izlenir (ADR 0087); durum çubuğunda web'in kayıt hücresi (aşamaları, lambası, tıklanınca sıradaki iş) ve sunucu hücresi, arkasında hesap menüsüyle (`fixtures/cloud/v1/cells.json`, ADR 0113); dosya projesinin olayları izlenir: başkasının revizyonu duyulur, bir kez söylenir ve sunulur (hücrede “Yeni revizyon”), Kaydet bilinen yeni revizyonun üstüne yüklemeden sorar, çakışma sorusunun dört cevabı (Yerel dosyaya kaydet dahil), yeniden eşitleme çizimi açmaz, Geçmiş'te “Açık çizim” (`fixtures/cloud/v1/file-revisions.json`, ADR 0119); Projeyi paylaş: erişenler ve kaynakları, rol değiştirme ve kaldırma, adla ya da e-postayla kişi arama (KentOS UI `Suggest`), e-postayla davet ve tek gösterimlik bağlantı, katalogdan ya da açık proje için `cloud.share` (`fixtures/cloud/v1/share.json`, ADR 0111); veritabanı projesinde `project.edit`'i olmayan katman ağacını değiştiremez (ADR 0078); başkasının sildiği katman, üzerinde gönderilmemiş nesne varken kalır ve geri gönderilir (ADR 0079); başkasının nesnesi çizimde olmayan katmanını bekler, katman gelince alınır; sunucu silmeyi reddedince başkasının kullandığı katman geri verilir (ADR 0081) | `apps/desktop/`, `apps/desktop/src/cloud/`, `apps/desktop/src/exchange/` |
| Native wgpu çizim hattı ve paylaşılan WGSL sözleşmesi; stilli katmanlar web'in `shaders/wgsl/styled` boru hatlarıyla, atlas tiny-skia ile; sözleşmenin 2. sürümünde atlas iki platformda çerçevenin grubunda (Iced'in iki bağlama grubu, ADR 0090); 3. sürümünde konumlar çapaya ortalanmış 65,536 km'lik karolarının merkezine göre, kamera kaba ve ince iki parça, dünyaya bağlı desenlerin evresi katlanır: çapadan binlerce km uzaktaki çizim de basamaksız (ADR 0157); tutulan resim: çizim değişmedikçe sahne yeniden çizilmez, resmi birleştirilir, üzerine gelinen nesnenin vurgusu pencerenin çözünürlüğünde üstüne çizilir (`FrameInput::keep_picture`, `overlays`; ADR 0120) | `crates/render/wgpu/`, `shaders/wgsl/` |
| Masaüstünün stil sayfası (`kentos-native-style`, ADR 0090): kitaplık (sistem, kullanıcı, proje; sistem kitaplığı web'in TypeScript'inden yazılan `.kstil`), düz görünüş, katmanın programı ve ifade tablosu, toplulukların renkleri ve atlas görüntüleri; `fixtures/style/v1/batches.json`'ı geçer. Masaüstünde gösterilen her katman stil motorundan çizilir, katmanlar yan yana kurulur; 8192 ve daha çok nesneli katman belgedeki yerlerine göre 4096'lık parçalarda kurulur: düzenleme yalnız kendi parçasını kurar ve yükler, görünmeyen parça çizilmez, partiler bütün katmanın sırasıyla çizilir (`batches::merged_order`, ADR 0121); SVG çizimleri roxmltree ve SVG çekirdeğinin okuyucularıyla (`apps/desktop/src/style/`). Katman stili penceresi (ADR 0091): beş işleyici, veriden sınıflar (`classify`, `classify.json`), işleyicinin tipli JSON'u (bilinmeyen alan ve tür korunur), Nesne sütunu çizimin sayısıyla (`tally`, iki platformda `tally.json`), sembol resimlerini çizimin stilli boru hatları çizer (`style/thumbs.rs`). Stil yöneticisi (ADR 0092): kaynak ve kategori ağacı, arama ve türler, kartlar, ayrıntılar ve alanları, kopyala, sil, `.kstil` ve pano ile içe ve dışa aktarma (`file`, iki platformda `kstil.json`), PNG ve JPEG görüntüler; `style.assign`, `style.clearSymbol` ve Katman stili yuvası için seçme kipi; Kitaplığım `kitaplik.kstil`'de, projenin kitaplığı çizimde (düzenleme, geri alma adımı değil). Lejant (ADR 0093): katman katman satırlar (`legend`, iki platformda `legend.json`), katman bırakma, PNG beyaz kâğıtta 2× (ekransız çizici, şerit şerit). Sembol tasarımcısı (ADR 0094): katman yığını, işaretin katmanları, canlı önizleme, türe göre formlar ve “ƒ”, kendi geri alması; modeli (katman türleri, yeni katman, özet, form yaması, liste düzenlemeleri) `designer`, iki platformda `designer.json`; Stil yöneticisinden (Düzenle, çift tık, Yeni sembol; sistem sembolünün kopyası) ve Katman stili yuvasından (Uygula yuvaya yazar) açılır; form parçaları `style/fields.rs`. SVG çizim düzenleyicisi (ADR 0095, `style/svgedit/`): dokuz araç, şekil listesi, cetvel ve kılavuzlar, kenet, düğüm aracı, ölçü, dört sekme, Yol, Nesne ve Seç menüleri, web'in tuşları, kendi geri alması; aç, içe alma penceresi, pano, bırakma, kitaplıktan aç, farklı kaydet, dışa aktarma (SVG, PNG), belge özellikleri, XML kaynağı, izleme altlığı, Bitmap izle; geometri SVG çekirdeğinden, kurallar (`svgEditModel.ts` ↔ `svgedit/`) iki platformda `svgedit.json`; Stil yöneticisinden, Sembol tasarımcısından ve `style.svgEditor` ile açılır, Kitaplığım'a yazar | `crates/native/style/`, `apps/desktop/src/style/` |
| Başsız komut sunucusu (`kentos-headless`, ADR 0130): penceresiz çizim (yeni proje ya da `.kcad` v2, v1), kataloğun masaüstü komutları adlarıyla `validate`, `plan`, `execute`, girdisi ve `CommandResult`'ı JSON; katalog şemalarıyla; katmanlar, sayfalı nesneler (kimlik imleci), ölçüler kaynak geometriden, geri alma, betiğin tek adımlık grubu, doğrulanan kayıt; sunucu komutları `server_command` ile reddedilir; Python (`kentos.cad`) ve MCP'nin tabanı. Proje modeli (`kentos-project`): koordinat sistemleri ve yeni projenin içeriği, masaüstünden ayrıldı | `crates/native/headless/`, `crates/native/project/` |
| Python SDK'sı (`kentos`, ADR 0131): `kentos.cad` altında kataloğun 35 komutu kendi adlarıyla (`kentos.cad.polygon.create`, `kentos.cad.project.share` …), katalogdan üretilen tipleriyle (girdi, çıktı ve plan; enum, etiketli birleşim, `UNSET` ile `None` ayrı), `.plan`, `.validate`, `.run`; `Document` (başsız çizim: yeni proje, `.kcad` aç ve kaydet, katmanlar, blok tanımları, sayfalı nesneler, ölçü, geri alma, betiğin tek adımı `group`), `Connection` (yerel hesap, `x-kentos-client: python`, proje kataloğu, sunucu komutları, projenin görüntüsü); üç hata ailesi; yerel modül `kentos._native` (PyO3, kararlı ABI 3.10+, maturin); katalog plan şemalarını da taşır. Masaüstünün Python konsolu (ADR 0132): alt panelin Python sekmesi, kod ayrı süreçte (`python -m kentos.host`; `KENTOS_PYTHON`, yoksa `.run/py`, yoksa `python3`), `doc` açık çizim; istekler `kentos_headless::rpc` ile açık çizimde, her çalıştırma tek geri alma adımı, hata ya da Durdur geri alır, çalışırken çizim başka düzenleme almaz; tamamlama (Ctrl+Boşluk, yazdıkça) ve imza yardımı konsolun Python'undan (`kentos._assist`, ADR 0135); Betik yüzü: renkli Python, satır numaraları, dosya, F5 ve seçimi çalıştır, hata satırına gitme, taslak `$XDG_STATE_HOME/kentos-cad/python-betik.json` (ADR 0136); konsol ve betik düzenleyicisi KentOS UI'ın `PythonRepl` ve `PythonEditor`'üdür (ADR 0132'nin 2 Ekim eki); ajan bağlantısı: kullanıcının açtığı `$XDG_RUNTIME_DIR/kentos-cad/masaustu.sock` (0600), MCP'nin `desktop.attach`'i açık çizimi okur ve komutlarla yazar, her yazma bir geri alma adımı ve “Ajan: …” satırı (ADR 0137) | `python/`, `crates/native/python/`, `scripts/python/` |
| MCP sunucusu (`kentos-mcp`, ADR 0133): stdio'da JSON-RPC, MCP 2026-07-28 (istek başına `_meta`, `server/discover`) ve eski istemciler için `initialize`; çizimler tutamaçla (`drawing.open`, `drawing.new` …, en çok 16), kataloğun 14 çizim komutu kendi adlarıyla (`drawing` ve `op` eklenmiş şemalar), sorgular (sayfa 100, en çok 1 000), kaynaklar (katalog, açık çizimler); komutun reddi `isError` araç sonucu; sunucu komutları (`project.*`, `project.list`, `project.open`) ortamdaki hesapla (`KENTOS_URL`, `KENTOS_LOGIN`, `KENTOS_PASSWORD`), `x-kentos-client: mcp` (ADR 0134) | `crates/native/mcp/` |
| Masaüstü belgesi (`kentos-domain`): web `CadDocument`'inin anlamı native olarak, ortak işlem fixture'larıyla sınanır | `crates/native/domain/`, `fixtures/document-ops/` |
| Masaüstünün bulut istemcisi (`kentos-cloud`, ADR 0040, 0043): yerel hesapla giriş, katalog, iki tür projeyi açma, dosya projesine revizyon, çizimden yeni proje, veritabanı projesine değişiklik, başkalarının değişiklikleri (olaylar sorarak izlenir, belgeye `apply_external` ile gelir), çakışmada benimki ya da sunucudaki, cihaz taslağı, çevrimdışı çalışma için projenin yerel kopyası; blok tanımları nesnelerle gider ve gelir, yerel kopyada da (ADR 0144 §5, `sync/blocks.rs`); arayüzü `apps/desktop/src/cloud/` (ADR 0041); WebSocket yok, olaylar bekleyerek sorulur (ADR 0044) | `crates/native/cloud/` |
| Masaüstünün işlem araçları (`kentos-processing`, ADR 0084): web'in `processing/` modüllerinin yerli karşılığı: parametreler ve doğrulama iletileri, kapsamlar, çalıştırıcı (kilitli katman, boş girdi, tek adım, yeni katman aynı adımda; web'in `RunJob`'u gibi hazırla, hesapla, bitir: hesap başka iş parçacığında çizimin okuma kopyasında, ADR 0124; modelin kopyada kaydı ve çizimde yeniden oynatılması, ADR 0125), modeller (`begin_group`, adım çalışmazsa geri alma), kayıt ve arama, dört yerleşik araç ve Parsel ölçü yazıları; modellerin düzenlenmesi ve tasarımcının planı (`model_edit`, `designer`; iki platformda `fixtures/processing/v1/designer.json`, ADR 0116); hesap ortak çekirdekten (köşe numaralama, kenar ölçüleri, ifade dili); iki platform `fixtures/processing/v1`'in 21 durumunu ve varsayılanlarını geçer | `crates/native/processing/`, `apps/desktop/src/processing/`, `fixtures/processing/` |
| Masaüstü araç oturumu (`kentos-interaction`): durumlar, veri olarak istem, kapalı alan/çoklu çizgi (tek yol aracı), seçim aracında tutamaçlar (deponun `grips`'i, çekirdeğin `move_grip`'i; ADR 0068) ve tutamaç menüsü (`grip_menu`, ADR 0074), çizgi, nokta, daire, yay, dikdörtgen, döndürülmüş dikdörtgen, düzgün çokgen, seçim ve Sil araçları; elips, eğri, yardımcı çizgi ve ışın, paralel çizgi, dik in ve dik çık, halka, revizyon bulutu, kot noktası ve böl; önizlemede dolgulu alan, kısa yazı, konacak nokta ve dik açı işareti (ADR 0057); Yazı: ev sahibinden yazı kutusu ister (`ViewChange::Text`), yazılanı `Tool::text_typed` ile alır (ADR 0060); Ölçülendirme (`dimension`): beş biçim, kenar ve daire seçme, işaretli yazılan mesafe (ADR 0061); Tarama (`hatch`): bölge deponun `enclosing`'inden ya da görünen çizgilerin yüzlerinden (`FaceIndex`), önizlemede kesikli alan ve soluk tarama çizgileri (ADR 0062); alan işlemleri (`area`, `boundary`): seçimden önce seçen tabana ikinci seçim, aşamanın kendi önizlemesi ve fareyi okuması eklendi, görünen çizgilerin yüzleri Tarama ile ortak (`faces`, ADR 0065), ürün komutlarıyla yazar (ADR 0069); çizimden nokta alma (`pick`, `ViewChange::Picked`, ADR 0070); askıya alma (`Session::nest`, `Tool::accept_point`) ve nokta hesaplayıcı (`point_calc`, ADR 0083); nesne izleme (`object_tracking`, `Pointer::tracked`, `Context::track_along`, ADR 0085); Öznitelikler'in ve yazı kutusunun yazmaları (`properties`: `cad.entities.set`, `cad.entities.edit`'in `properties` işlemi, ADR 0066); taşı, kopyala, döndür, ölçekle ve aynala (seçimden önce seçen ortak taban `modify`); kenar seçen taban (`edge`) ve on değiştirme aracı; esnet, dizi, kutupsal dizi ve hizala; Esc ile bir adım geri (`Tool::cancel`); istemin notları (ADR 0047); pano (`clipboard`) ve katalog dışı yapıştırma aracı (`paste`, `Session::run`, son komut sayılmaz); Kaydır ve Pencere yakınlaştır (`navigate`): araç görünüm değişikliğini `ViewChange` olarak ister, kabuk uygular; onay almayan araçta Enter son komutu yineler (ADR 0056); araçların oturum belleği (`Memory`); belgenin günlüğüyle izlenen geometri deposu (`Spatial`, ADR 0029); iki platform `fixtures/interaction/v1` izlerini ve `fixtures/point-input/v1` dilbilgisini geçer | `crates/native/interaction/` |
| Ürün komutları `cad.polygon.create`, `cad.line.create`, `cad.polyline.create`, `cad.point.create`, `cad.circle.create`, `cad.arc.create`, `cad.entities.delete`, `cad.entities.transform` v1: web ve masaüstü işleyicileri, `CommandResult`, katalogla eşit kayıtlar; kapalı alan, çizgi ve çoklu çizgi araçları bu komutlardan yazar, nokta, daire ve yay araçları kendi komutlarından, dikdörtgen ve düzgün çokgen `cad.polygon.create`'ten yazar, Sil aracı `cad.entities.delete` ile siler, değiştirme araçları `cad.entities.transform` ile yazar (ADR 0022, 0027, 0029, 0032, 0037); `cad.entities.edit` v1: kenar, köşe ve nesne araçları, Esnet, tutamaçlar ve tutamaç menüsü (`grip`, `vertexAdd`, `vertexRemove`, `straightEdge`, `arcEdge`; ADR 0074) bununla yazar; `cad.entities.array` v1: Dizi ve Kutupsal dizi; Hizala `cad.entities.transform`'un `align` dönüşümüyle (ADR 0047); `cad.entities.create` v1: kendi komutu olmayan çizim araçları (elips, eğri, yardımcı çizgi, ışın, halka; yazı, ADR 0060; ölçü, ADR 0061; tarama, adı “Tarama”, ADR 0062) ve birden çok nesneyi tek adımda yazanlar (paralel çizgi, dikler, böl) bununla yazar; revizyon bulutu `cad.polygon.create`, kot noktası `cad.point.create` ile (ADR 0057); `cad.entities.set` v1: nesnelerin katmanı, rengi, sembolü, öznitelikleri ve etiketi; iki platformda Öznitelikler bununla, geometri değerleri ve yerinde yazı düzenleyicisi `cad.entities.edit`'in `properties` işlemiyle yazar; yazının boş metni reddedilir (`empty_text`, ADR 0066); alan işlemleri `cad.entities.edit`'in `areaUnion`, `areaIntersect`, `areaSubtract`, `areaSplit`, `toArea`, `toPolyline` işlemleriyle, İçine tıklayarak alan `cad.entities.create`'in `boundary` işlemiyle yazar; kapalı alanın halkası yaylı kenarla 2 köşeli olabilir (ADR 0069); `cad.blocks.define` v1 ve `cad.blocks.edit` v1 (yeniden adlandır, yeniden tanımla, taban noktası, sil, temizle, öznitelik listesi): Blok oluştur ve Bloklar paneli bunlarla, yerleştirme `cad.entities.create`'in `insert` geometrisiyle, Patlat yerleştirmeyi `cad.entities.edit`'in kendi katman, renk, kalınlık, öznitelik ve etiketi olan `add`'leriyle yazar (ADR 0144) | `product/`, `crates/native/application/`, `fixtures/commands/` |
| Tipli ayarlar: şema, katmanlı çözüm, ortak durumlar; web servisi, masaüstü ayar dosyası ve penceresi, canlı MSAA/HiDPI (ADR 0023); görünüş anahtarları (tema, vurgu, yazı tipleri, piksel boyu) iki platformda ortak, eski anahtarlar okunurken çevrilir (ADR 0126) | `crates/shared/contracts/src/settings/`, `core/settings/`, `app/settings/`, `apps/desktop/src/settings*.rs`, `fixtures/settings/` |

Masaüstünde aplikasyon ve ifraz araçları, embed Python, tam AI yüzeyi,
genişletilmiş proje bazlı paylaşım ve kalıcı server worker kabulü gelecek
işlerdir. Mevcut tenant/cloud altyapısını yok saymayın; onu bu kapsamla tamamlayın.
Web'e göre verilen kısa dosya yolları `apps/web/src/` altındadır.

## 2. Çalıştırma

Komutlar depo kökünden çalışır; kesin kaynak `package.json`,
`apps/web/package.json`, `Cargo.toml` ve `rust-toolchain.toml` dosyalarıdır.

```bash
make                     # gruplu komut listesi; servisler: make dev/run/desktop/stop/status
pnpm install
pnpm dev                 # Vite; yerel çizim API olmadan çalışır
pnpm typecheck
pnpm test                # Vitest; gerekli WASM paketlerini kontrol eder
pnpm build               # WASM kontrolü + tsc + Vite
pnpm rust:test           # web/sunucu crate'leri: cargo test + clippy -D warnings + bağımlılık yönü
pnpm rust:test:desktop   # kentos-ui, vitrin ve masaüstü: cargo test + clippy
pnpm arch:deps           # yalnız bağımlılık yönü denetimi (ADR 0010, ARCH-01)
pnpm test:rust           # Rust ve WASM/format entegrasyon testleri
pnpm wasm                # değişen ortak kaynakların WASM paketlerini derle
pnpm e2e                 # gerçek tarayıcı duman testi
pnpm e2e:visual          # görsel karşılaştırma
pnpm e2e:layout          # her pencere, menü ve çubuk 1100×650 ve 1440×900'de, iki temada: taşma, kesik düğme ve yazı; resimler scripts/e2e/out/layout
pnpm e2e:interaction     # etkileşim izleri: poligon kabul izi, tuş anlamları (fixtures/interaction, ADR 0018); tarayıcılar yan yana, --browsers=N
cargo test -p kentos-desktop traces   # aynı izler masaüstünde, pencere açmadan, çekirdeklere paylaşılarak (ADR 0021)
./target/debug/kentos-cad kullan usage-parcel   # kullanım senaryosunu masaüstünde oynatıp adım adım resimle (.run/shots/kullanim; fixtures/interaction/README.md)
pnpm -C apps/web e2e:use usage-parcel   # aynı senaryo web'de, aynı adlarla web-…png
python3 scripts/usage/compare.py usage-parcel   # iki platformun resimleri yan yana
apps/desktop/scripts/cloud-live.sh   # masaüstünün bulut arayüzü gerçek kentosd ile (geliştirme veritabanı; görüntüler .run/shots/bulut-*; ADR 0041)
cargo test -p kentos-desktop cloud::file_follow_tests::revision_screens -- --ignored --nocapture   # dosya projesinin revizyon resimleri, .run/shots/bulut-revizyon-* (ADR 0119)
cargo test -p kentos-processing   # işlem araçlarının ortak durumları masaüstünde (fixtures/processing/v1, ADR 0084)
cargo test -p kentos-native-application   # ürün komutlarının durumları masaüstünde (fixtures/commands, ADR 0022, 0027, 0029, 0032, 0037, 0047, 0057, 0066)
python3 scripts/fixtures/transform_command_cases.py --check   # cad.entities.transform durumlarını dönüşümlerin tanımından denetle (ADR 0037; Kauçuk levha'nınkiler levhanın çekirdekle aynı işlem sırasıyla Python ikizinden, sheet_f64.py, ADR 0158)
python3 scripts/fixtures/edit_command_cases.py --check   # cad.entities.edit durumlarını sözleşmenin kuralından denetle (ADR 0047)
python3 scripts/fixtures/array_command_cases.py --check   # cad.entities.array durumlarını dizilerin tanımından denetle (ADR 0047)
python3 scripts/fixtures/create_command_cases.py --check   # cad.entities.create durumlarını sözleşmenin kuralından denetle (ADR 0057)
python3 scripts/fixtures/set_command_cases.py --check   # cad.entities.set durumlarını (Öznitelikler: katman, renk, sembol, öznitelik, etiket) sözleşmenin kuralından denetle (ADR 0066)
cargo test --release -p kentos-interaction --test perf -- --ignored --nocapture   # masaüstü deposu: eşitleme, kenet ve seçme süreleri (ADR 0029)
cargo test --release -p kentos-expression --test perf -- --ignored --nocapture   # ifade motoru 10⁵ ve 10⁶ nesnede (ADR 0100; web yolu apps/web/scripts/perf/expression.test.ts)
python3 scripts/fixtures/expression_geometry.py --check   # ifadelerin geometri değerlerini (alan, ağırlık merkezi, kutu) bağımsız hesapla denetle (ADR 0100)
KENTOS_WRITE_BUILDER=1 cargo test -p kentos-expression --test builder   # İfade oluşturucunun yanıtlarını (fixtures/expression/v2/builder.json) yeniden yaz; farkı okuyun (ADR 0100 §5)
python3 scripts/fixtures/expression_language.py --check   # dil eklerinin değerlerini (durum, içinde, arasında, gibi, benzer, boş, ^, yeni işlevler) bağımsız başvuruyla denetle (ADR 0100 §4)
(cd apps/web && node scripts/e2e/builder.mjs)   # İfade oluşturucu web'de klavye ve fareyle (açma, tamamlama, ağaç, değerler, önizleme, Vazgeç, Tamam)
(cd apps/web && node scripts/e2e/flow.mjs)   # İfadenin akışı web'de fareyle ve klavyeyle (düğümler, değer, bağlama ve ayırma, palet, silme ve geri alma, Tamam; ADR 0101)
KENTOS_WRITE_FLOW=1 cargo test -p kentos-expression --test flow   # akışın yanıtlarını (fixtures/expression/v2/flow.json) yeniden yaz; farkı okuyun (ADR 0101)
cargo test -p kentos-desktop expression::tests::flow_screens -- --ignored --nocapture   # masaüstünün akış resimleri, .run/shots/ifade-akisi-*
cargo test -p kentos-desktop expression::tests::screens -- --ignored --nocapture   # masaüstünün İfade oluşturucu resimleri, .run/shots/ifade-olusturucu-*
KENTOS_WRITE_SYSTEM_STYLES=1 pnpm -C apps/web exec vitest run scripts/style/system-library.test.ts   # sistem stil kitaplığının masaüstü kopyasını yeniden yaz; farkı okuyun (ADR 0090)
cargo test --release -p kentos-desktop style::perf -- --ignored --nocapture --test-threads=1   # stilli çizimin kurulum ve kare süreleri (ADR 0090)
KENTOS_DEMO_OUT=$PWD/.run/demo/demo.json pnpm -C apps/web exec vitest run scripts/style/demo-drawing.test.ts   # web'in örnek çizimi (gösterim kataloğu) dosyaya
cargo test -p kentos-desktop style::screens -- --ignored --nocapture; node apps/web/scripts/style/showcase-shots.mjs   # katalog görünümleri iki platformda, .run/shots/stil-*, web-stil-*
KENTOS_SHOTS_ONLY=acik,arama cargo test -p kentos-desktop style::screens::manager_screens -- --ignored --nocapture   # Stil yöneticisi resimleri, .run/shots/smgr-* (değişken yoksa bütün durumlar; ADR 0092)
KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-desktop style::screens::legend_screens -- --ignored --nocapture   # Lejant ve kaydettiği PNG, .run/shots/lejant-* (ADR 0093)
KENTOS_SHOTS_ONLY=tarama,soru cargo test -p kentos-desktop style::designer::screens -- --ignored --nocapture   # Sembol tasarımcısı resimleri, .run/shots/sdes-* (değişken yoksa bütün durumlar; web'inkiler: node apps/web/scripts/e2e/shots.mjs symboldesigner; ADR 0094)
GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-designer.test.ts   # sembol tasarımcısının modelini fixtures/style/v1/designer.json'a yeniden yaz; farkı okuyun (ADR 0094)
cargo test -p kentos-desktop processing::designer::tests::screens -- --ignored --nocapture   # Model tasarımcısı resimleri, .run/shots/model-* (KENTOS_SHOTS_ONLY=yeni,adim; web'inkiler: node apps/web/scripts/e2e/shots.mjs modeldesigner; ADR 0116)
cargo test -p kentos-desktop points::tests::screens -- --ignored --nocapture   # alt panelin Noktalar sekmesi (nokta editörü), .run/shots/noktalar-* (web'inkiler: node apps/web/scripts/e2e/shots.mjs pointeditor; ADR 0153)
cargo test -p kentos-desktop ribbon_bar::screens -- --ignored --nocapture   # şeridin hızlı erişim menüsü, sağ tık menüleri ve bölünmüş düğme listeleri, .run/shots/serit-* (web'inkiler: node apps/web/scripts/e2e/shots.mjs ribbon; ADR 0117)
cargo test -p kentos-desktop ribbon_keys::screens -- --ignored --nocapture   # şeridin harf ipuçları ve daraltılmış şeridin açılışı, .run/shots/serit-ipucu-*, serit-katli-* (ADR 0118)
cargo test -p kentos-desktop icon_tour -- --ignored --nocapture   # ikon turu: şeridin açılır listeleri, katmanın, proje türünün ve paftanın menüleri, .run/shots/ikon-turu (KENTOS_SHOTS_ONLY=daire,kose)
KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=olc,izle cargo test -p kentos-desktop style::svgedit::screens -- --ignored --nocapture   # SVG düzenleyicisi resimleri, .run/shots/svge-* (değişken yoksa bütün durumlar; altlık ve izleme wgpu ister; web'inkiler: node apps/web/scripts/e2e/shots.mjs svgedit; ADR 0095)
GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-svgedit.test.ts   # SVG düzenleyicisinin kurallarını fixtures/style/v1/svgedit.json'a yeniden yaz; farkı okuyun (ADR 0095)
KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings   # ayar şeması değişince settingsSchema.json'u yeniden yaz (ADR 0023)
cargo run -q -p kentos-kcad --bin kcad -- inspect|validate|sniff DOSYA   # KCAD v2 dosyasını incele (ADR 0025)
python3 tools/kcad/kcad.py validate DOSYA   # bağımsız Python okuyucusu
python3 scripts/fixtures/kcad_v2_reference.py --check   # örnek dosyaları bağımsız yazıcıyla denetle
python3 scripts/fixtures/gis_reference.py --check   # GeoJSON/Shapefile fixture'larını bağımsız okuyucuyla denetle (ADR 0046)
python3 scripts/fonts/drawing_fonts.py --check   # masaüstünün çizim yazı tiplerini web'in WOFF2'lerinden denetle (ADR 0055)
python3 scripts/fonts/ui_fonts.py --check   # KentOS UI'ın Noto Sans ve Roboto'sunu web'in WOFF2'lerinden denetle; --advance genişlik tahminlerini ölçer
KENTOS_WRITE_GIS_EXPORTS=1 cargo test -p kentos-formats --test gis   # GeoJSON yazıcısının örnek çıktısını yeniden yaz; farkı okuyun
KENTOS_WRITE_DXF=1 cargo test -p kentos-formats --test dxf_write   # DXF yazıcısının örnek çıktısını (fixtures/formats/v1/dxf-write) yeniden yaz; farkı okuyun
python3 scripts/fixtures/dxf_write_reference.py --check   # DXF yazıcısının blok, öznitelik, yazı, kılavuz ve yerel projenin birimi örneklerini KentOS kodu olmadan denetle (ADR 0144, 0145, 0146, 0165)
python3 scripts/fixtures/text_cases.py --check   # yazı kurallarını (Artır, Bul ve değiştir, Okunur yap) kurallardan denetle (ADR 0145)
python3 scripts/fixtures/leader_cases.py --check   # kılavuzun yerleşimini (ok başı, kol, not) kuraldan denetle (ADR 0146)
python3 scripts/fixtures/dimension_cases.py --check   # yeni ölçü türlerinin yerleşimini kurallardan denetle (ADR 0147)
python3 scripts/fixtures/quick_dimension_cases.py --check   # Hızlı ölçü'nün ölçülerini (taraf, uzaklık, ortak kenar) kurallardan denetle (ADR 0147 §7)
python3 scripts/fixtures/numeric_display.py --check   # gösterim kuralının durumlarını (yarımlar, gürültü, işaret, taşma) kuraldan denetle (ADR 0149)
python3 scripts/fixtures/measure_cases.py --check   # ölçülerin kesin değerlerini (uzunluk, alan, yay, elips, eğri, semt, açı, ölçü türleri) 50 basamaklı bağımsız başvuruyla denetle (ADR 0149)
python3 scripts/fixtures/topology_cases.py --check   # Topolojik temizliğin durumlarını (birleştirme, uzatma, budama, kenara taşıma, kot) kurallardan denetle (ADR 0148)
python3 scripts/fixtures/polygonize_cases.py --check   # Toplu alan'ın durumlarını (bölgeler, adalar, etiketler, var olan alan, boşta uçlar) kesin kesirlerle kurallardan denetle (ADR 0151)
python3 scripts/fixtures/vertex_points_cases.py --check   # Köşelere nokta'nın durumlarını (paylaşılan köşe, var olan nokta, kot, ad artımı) kurallardan denetle (ADR 0152)
python3 scripts/fixtures/label_text_cases.py --check   # Etiketleri yazıya çevir'in kuralını (dört yerleşim, büyüme ve üst sınır, ölçek aralığı, en küçük nesne, okunur yön, 8 px'lik hücrelerle inceltme) kesirle bağımsız başvurudan denetle (ADR 0175)
python3 scripts/fixtures/point_editor_cases.py --check   # Nokta editörünün hesaplarını (doğal sıra, tablonun süzgeç ve sıralaması, çift noktalar, bağlı köşeler) kurallardan denetle (ADR 0153)
python3 scripts/fixtures/point_edit_cases.py --check   # Nokta editörünün düzenlemelerini (hücreler, bağlı çizgiler, taslak satır) kurallardan denetle (ADR 0153 §3–§4)
python3 scripts/fixtures/point_batch_cases.py --check   # Nokta editörünün toplu işlemlerini (Yeniden adlandır, Sıralı numara ver, Katmana taşı, hedef satırlar) kurallardan denetle (ADR 0153 §5)
python3 scripts/fixtures/fit_cases.py --check   # Vektör oturtma'nın çözümünü (Helmert, afin, projektif; artıklar, m0, çözümsüzlükler) tam kesirle bağımsız başvurudan denetle (ADR 0156)
python3 scripts/fixtures/warp_cases.py --check   # Vektör oturtma'da nesnelerin dönüşmesini (afinde elips, projektifte 0,1 mm'lik köşeler, yazı ve blok kuralı, ufuk reddi) kurallardan denetle (ADR 0156)
python3 scripts/fixtures/rubber_warp_cases.py --check   # Kauçuk levha'da nesnelerin kurallarını (yalnız köşeler, en yakın benzerlik, sapma) bağımsız başvurudan denetle (ADR 0158 §3)
python3 scripts/fixtures/rubber_cases.py --check   # Kauçuk levha'nın ince plaka eğrisini (görüntüler, türev, çözümsüzlükler) mpmath ile 50 basamaklı bağımsız başvurudan denetle (ADR 0158)
python3 scripts/fixtures/edgematch_cases.py --check   # Kenar eşleme'nin bağlarını (aday, puan, bire bir eşleme, kavşak, eşsiz uç) ve yöntemlerini (Ucu taşı, Parça ekle, Köşeleri ayarla; üç buluşma yeri, kotlar) mpmath ile 50 basamaklı bağımsız başvurudan denetle (ADR 0159)
python3 scripts/fixtures/topology_edit_cases.py --check   # Topolojik düzenlemenin kurallarını (ortak köşe ve kenar, taşıma, köşe ekleme, kabarıklık, köşe silme, kilitli ve geçersiz komşu, düzenlemenin öncesinden ve sonrasından değişiklikler) kesin kesirlerle denetle (ADR 0160)
python3 scripts/fixtures/adjoin_cases.py --check   # Çakışma denetiminin kırpmasını ve Bitişik alan'ın bölgesini (komşular, delik, çok parça, sarkan uç, yaylı komşu) kesin kesirlerle bağımsız başvurudan denetle (ADR 0162)
python3 scripts/fixtures/lock_cases.py --check   # Sayısallaştırma kilitlerinin kurallarını (kilitli nokta, Orto ve kutupsal izlemeyle, açı ve sapma doğrultuları, Dik kapat, `<açı` kilit metni) kesin kesirlerle ve 50 basamaklı mpmath ile bağımsız başvurudan denetle (ADR 0166)
python3 scripts/fixtures/snap_cases.py --check   # Kenet eklerinin kurallarını (ağırlık merkezi, karelaj, uzantı, paralel, öncelikler, katmanın türleri, çizilmekte olan yol) kesin kesirlerle bağımsız başvurudan denetle (ADR 0163)
cargo test --release -p kentos-geometry-core --test adjoin -- --ignored --nocapture   # Bitişik alan'ın bir görünüm parselindeki süresi, önizleme bütçesi için (ADR 0162 §5)
python3 scripts/fixtures/trace_cases.py --check   # İzle'nin yollarını (kesişimden dönme, düz geçme, iki yoldan kısası, eşit yollar, yaylar, delik ve parça, ortak kenar, daire) ve Zincir'i 50 basamaklı bağımsız başvurudan denetle (ADR 0161)
python3 scripts/fixtures/crs_transform_cases.py --check   # koordinat dönüşümlerini (TM, UTM, coğrafi, Pseudo-Mercator; ED50, TUREF ve WGS 84 arası EPSG yolları) ve DMS yazılışını PROJ'la (pyproj) denetle; durumlar fixtures/geodesy/v1/transform.json (ADR 0167)
python3 scripts/fixtures/crs_measure_cases.py --check   # ikinci sistemin düzlemindeki uzunluk ve alanları (yollar, yaylı ve delikli alanlar; coğrafi ve Pseudo-Mercator redleri, yayların parçaları) PROJ'la denetle; durumlar fixtures/geodesy/v1/measure.json (ADR 0167 §2)
python3 scripts/fixtures/crs_custom_cases.py --check   # projenin koordinat sistemlerini (başlangıcı farklı TM, Bessel ve Krasovski datumları iki dönüklük kuralıyla, yerel sistemler, projenin datum seçimleri) PROJ hatlarıyla denetle; durumlar fixtures/geodesy/v1/custom.json (ADR 0168)
python3 scripts/fixtures/crs_text_cases.py --check   # WKT 1, WKT 2 ve PROJ dizesi okuma ve yazmayı (datum kuralı, redler, yerel sistemin DERIVEDPROJCRS'i) PROJ'un okuduğuyla denetle; durumlar fixtures/geodesy/v1/text.json (ADR 0168 §5)
python3 scripts/fixtures/crs_sweep.py   # dönüşümlerin rastgele fark testi: kayıttaki bütün çiftler, rastgele proje tanımları ve PROJ'un kendi +towgs84 yolu, pyproj'la ~150 000 karşılaştırma; çekirdeği examples/ops.rs ile çağırır (release derler); bilinen tek fark TUREF'e 0,1 mm (ADR 0168 Doğrulama)
python3 scripts/fixtures/crs_choice_form_cases.py --check   # Datum dönüşümleri'nin form kurallarını (ad, sayılar, üç parametre, doğruluk, ızgara; EPSG yollarının metni) kurallardan denetle; durumlar fixtures/crs/v1/choice-form.json (ADR 0168 §9 4a)
cargo test -p kentos-desktop project::choices::tests::screens -- --ignored --nocapture   # Datum dönüşümleri'nin resimleri, .run/shots/datum-donusumleri-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs datums))
python3 scripts/fixtures/crs_definition_form_cases.py --check   # Özel koordinat sistemi penceresinin form kurallarını (ad, TM'nin sayıları, projenin datumu, yerel sistemin tabanı ve düzlemi, kayıttakiyle aynı tanım) kurallardan denetle; durumlar fixtures/crs/v1/definition-form.json (ADR 0168 §9 4b)
python3 scripts/fixtures/field_reduce_cases.py --check   # karne indirgemesini (iki durum, indeks hatası, sıfırdan geçen okuma, tek II. durum, başucusuz doğrultu, yatay uzunluk, kot farkı ve k) mpmath ile 50 basamaklı bağımsız başvurudan denetle; durumlar fixtures/field/v1/reduce.json (ADR 0169 §3)
python3 scripts/fixtures/field_csv_cases.py --check   # CSV/TXT karne okuyucusunu (ayırıcı, virgüllü ondalık, istasyonlar, prizma yüksekliğinin taşınması, bozuk değerler) kurallardan denetle; durumlar fixtures/field/v1/csv.json (ADR 0169 §1)
python3 scripts/fixtures/survey_form_cases.py --check   # Proje ayarları › Ölçme'nin form kurallarını (k'nın aralığı ve varsayılanı, toleransların cc, ″ ve mm çevirisi, gösterim, iletiler) kurallardan denetle; durumlar fixtures/project/v1/survey-form.json (ADR 0169 §3)
cargo test -p kentos-desktop project::survey::tests::screens -- --ignored --nocapture   # Proje ayarları › Ölçme'nin resimleri, .run/shots/olcme-ayar-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs survey))
python3 scripts/fixtures/field_traverse_cases.py --check   # karnenin poligonunu (istasyon zinciri, kırılma açıları, iki yönden kenarlar, eksik gözlemler, birim çevirisi) indirgemenin mpmath başvurusuyla kurallardan denetle; durumlar fixtures/field/v1/traverse.json (ADR 0169 §3)
python3 scripts/fixtures/field_sniff_cases.py --check   # karne biçiminin içerikten tanınmasını (GSI sözcüğüyle başlayan ilk dolu satır, ilk beş kayıtta SDR başlığı; BOM, CR, STX, kod bloğu, metin karneler) kurallardan denetle; durumlar fixtures/field/v1/sniff.json (ADR 0169 §6)
python3 scripts/fixtures/field_samples.py   # Karne editörünün resimleri ve akış testleri için aynı karnenin elle yazılmış örneklerini tek gözlem tablosundan yeniden yaz: sample.gsi (GSI-16), sample.sdr (SDR33), sample.gt7 (GTS-7), sample-nikon.raw (Nikon RAW), sample.jxl (JobXML, derecede); her biçimin örneği aynı karneyi verir (fixtures/field/v1)
python3 scripts/fixtures/field_gts7_cases.py --check   # Topcon GTS-7 okuyucusunu (denetim sözcükleri, UNITS, DDD.MMSS ve gon, eksi yatay okuma, STN ve XYZ, BS/FS/SS, HV/SD, HD ve OFFSET, bozuk sayılar) Topcon Link kılavuzunun Ek C'sinden yazılmış başvurudan denetle; durumlar fixtures/field/v1/gts7.json (ADR 0169 §1)
python3 scripts/fixtures/gnss_gpx_cases.py --check   # GPX 1.1 okuyucusunu (yol, rota ve iz noktaları, ele ve geoidheight'tan elipsoit yüksekliği, fix, uydu, HDOP, fix none, bozuk konum ve değerler, XML değil, derin) şemadan yazılmış, expat'le okuyan başvurudan denetle; durumlar fixtures/gnss/v1/gpx.json (ADR 0169 §1)
python3 scripts/vendor/geographiclib.py --check   # GeographicLib'in libm'li kopyasını (crates/shared/geographiclib-rs) crates.io sürümünden ve kuraldan denetle; değişince yeniden yaz (ADR 0171 §5)
python3 scripts/fixtures/ground_survey_cases.py --check   # Hesap pencerelerinin zemin ile düzlem arası uzunluklarını (Kutupsal alım noktaları yerine koyar, Aplikasyon zemin uzunluğunu verir, Poligon hesabı noktalarını kapanmasız geri verir) bilinen düzlem noktalarından ve PROJ ile GeographicLib'in çarpanlarından denetle; durumlar fixtures/geodesy/v1/ground-survey.json (ADR 0171 §4)
python3 scripts/fixtures/reshape_cases.py --check   # Biçim değiştir'in, Sürdür'ün ve deliklerin durumlarını (alanda kırpma ve cep, çok parça, retler; çizgide iki ve tek buluşma, yaylı yol; sondan ve baştan sürdürme, uçların doğrultuları; delik ekleme, birleşme, silme ve halka) düz kenarlarda kesin kesirlerle halka ve yol ekleyen, yayda mpmath'le hesaplayan bağımsız başvurudan denetle; durumlar fixtures/reshape/v1/cases.json (ADR 0173)
python3 scripts/fixtures/vertex_table_cases.py --check   # Köşe tablosunun satırlarını ve yazmalarını (taşıma, kot, yarıçap büyüklüğünü koruyarak ve payıyla, köşe ekleme, çoklu silme, retler) kurallardan ve mpmath'le 50 basamaklı yarıçap ile büküm arası çeviriden denetle; durumlar fixtures/vertex-table/v1/cases.json (ADR 0172)
python3 scripts/fixtures/vertex_edit_cases.py --check   # Köşe tablosunun yazmalarını (hücreler, taslak satır, silme; iletiler, adımlar, açık kalan hücre; CAD'in eksen adları) kurallardan denetle; durumlar fixtures/vertex-table/v1/edits.json (ADR 0172)
cargo test -p kentos-desktop vertices::tests::screens -- --ignored --nocapture   # Köşe tablosunun resimleri, .run/shots/kose-tablosu-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs vertextable))
python3 scripts/fixtures/ground_cases.py --check   # zemin, elipsoit ve düzlemi (noktanın ve çizginin ölçeği PROJ'un get_factors'ıyla, jeodezik uzunluk GeographicLib'in C'siyle, yükseklik çarpanı mpmath'le, alanlar sık sınırla eşit alanlı izdüşümde; areaNoise köşe çokgeninin yuvarlama payı) bağımsız başvurudan denetle; durumlar fixtures/geodesy/v1/ground.json (ADR 0171)
python3 scripts/fixtures/gnss_nmea_cases.py --check   # NMEA 0183 okuyucusunu (GGA ve RMC, sağlama toplamı, kayıt öneki, kalite ve adları, ddmm.mmmm konum, birimler, geoit ayrımı, tarihli ve tarihsiz zaman) cümlelerden yazılmış başvurudan denetle; durumlar fixtures/gnss/v1/nmea.json (ADR 0169 §1)
python3 scripts/fixtures/field_write_cases.py --check   # cihaza gönderilen koordinat dosyalarını (Leica GSI-16 ve GSI-8, Topcon GTS-7, Trimble JobXML, Nikon RAW, CSV; taşınamayan ad, kod ve değerler) biçimlerin belgelerinden yazılmış başvurudan denetle; durumlar fixtures/field/v1/write.json (ADR 0169 §4)
python3 scripts/fixtures/gnss_import_cases.py --check   # GNSS içe aktarmanın noktalarını (WGS 84'ten projenin sistemine, adlar, kot elipsoit yüksekliği, öznitelikler, dönüşümün doğruluk metni) PROJ'la denetle; durumlar fixtures/gnss/v1/import.json (ADR 0169 §6)
python3 scripts/fixtures/gnss_samples.py   # GNSS içe aktar penceresinin resimleri ve akış testleri için örnek GPX ve NMEA'yı (fixtures/gnss/v1/sample.*, örnek çizimin parsel köşeleri) PROJ'la yeniden yaz; --check farkı arar
cargo test -p kentos-desktop exchange::field_send_tests::screens -- --ignored --nocapture   # Cihaza gönder penceresinin resimleri, .run/shots/cihaza-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs fieldsend))
cargo test -p kentos-desktop exchange::gnss_tests::screens -- --ignored --nocapture   # GNSS içe aktar penceresinin resimleri, .run/shots/gnss-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs gnss))
python3 scripts/fixtures/field_jobxml_cases.py --check   # Trimble JobXML okuyucusunu (FieldBook'un istasyon, prizma ve nokta kayıtları, ham yöntemler, silinmiş ve ortalanmış kayıtlar, bulunmayan istasyon ve prizma, XML değil, 256 düzeyden derin) şema 5.3'ten yazılmış, Python'un expat'iyle okuyan başvurudan denetle; durumlar fixtures/field/v1/jobxml.json (ADR 0169 §1)
python3 scripts/fixtures/field_nikon_cases.py --check   # Nikon RAW okuyucusunu (CO kayıtlarından birimler, DDDMMSS ve gon, Zenith ve Horizon, koordinat kayıtları ve ST, F1/F2/SS/CP/SO, bozuk sayılar) Nivo kılavuzundan yazılmış başvurudan denetle; durumlar fixtures/field/v1/nikon.json (ADR 0169 §1)
python3 scripts/fixtures/field_sdr_cases.py --check   # Sokkia SDR2x ve SDR33 okuyucusunu (sabit alanlar, işin birimleri, alet kaydının düşey açı seçeneği, prizma yüksekliği, F1/F2/MD, MC ve 08 sayıları, STX/ETX, bozuk değerler) Sokkia'nın belgesinden yazılmış başvurudan denetle; durumlar fixtures/field/v1/sdr.json (ADR 0169 §1)
cargo test -p kentos-desktop calc::fieldbook::tests::screens -- --ignored --nocapture   # Karne editörünün resimleri, .run/shots/karne-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs fieldbook))
python3 scripts/fixtures/field_gsi_cases.py --check   # Leica GSI-8 ve GSI-16 okuyucusunu (sözcükler, birim haneleri, DMS, karışık birimler, istasyon koordinatları, bozuk sözcük, mil, ayak, tam dönüşten büyük açı) Leica'nın belgesinden yazılmış başvurudan denetle; durumlar fixtures/field/v1/gsi.json (ADR 0169 §1)
python3 scripts/fixtures/crs_definition_fit_cases.py --check   # Ortak noktalardan hesapla'nın düzlemini (benzerlik ve afin, artıklar, m0, atlanan satırlar, çözümsüzlükler) Vektör oturtma'nın kesir başvurusu ve mpmath'le denetle; durumlar fixtures/crs/v1/definition-fit.json (ADR 0168 §9 4d)
python3 scripts/fixtures/crs_definition_text_cases.py --check   # Özel koordinat sistemi'nin WKT ve PROJ okumasını (tanım, Kayıttakini seç, ızgara notu, redlerin nedenleri) PROJ'un okuduğu sistemlerden ve kurallardan denetle; durumlar fixtures/crs/v1/definition-text.json (ADR 0168 §9 4c)
cargo test -p kentos-desktop project::custom_crs::tests::screens -- --ignored --nocapture   # Özel koordinat sistemi penceresinin ve Proje ayarları'nda tanımın resimleri, .run/shots/ozel-crs-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs definitions))
python3 scripts/fixtures/project_crs_cases.py --check   # projenin sistemlerinin çözümünü (kayıttaki ya da tanım, ikinci sistem, datum seçimleri; adları ve kodları) kurallardan ve kayıttan denetle; durumlar fixtures/geodesy/v1/project.json (ADR 0168 §9 3b)
cargo test -p kentos-desktop grids::tests::screens -- --ignored --nocapture   # Proje ayarları'nın Izgaralar grubunun resimleri, .run/shots/izgara-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs grids); ADR 0168 §4)
cargo test -p kentos-desktop second_crs::tests::custom_screens -- --ignored --nocapture   # projenin tanımlarının resimleri, .run/shots/ozel-sistem-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs customcrs); ADR 0168)
python3 scripts/fixtures/ntv2_cases.py --check   # NTv2 ızgaralarını (KentOS kodu olmadan yazılan Türkiye, büyük uçlu ve iç içe ızgaralar, bozuk dosyalar) ve kaymalarını PROJ'un hgridshift'iyle denetle; durumlar fixtures/geodesy/v1/ntv2.json (ADR 0168 §4)
python3 scripts/fixtures/crs_convert_cases.py --check   # Koordinat dönüştür'ün okuma ve yazmasını (eksen adları, sayı ve açı dilbilgisi, DMS ve DD, doğruluk metni, hatalar; projenin tanımları ve datum seçimleri) PROJ'la denetle; durumlar fixtures/crs/v1/convert.json (ADR 0167 §4, 0168 §9 3c)
python3 scripts/fixtures/tm_cases.py --check   # ileri TM izdüşümünün durumlarını (TUREF ve ED50 TM3, UTM; dilim kenarları) PROJ'un tmerc'iyle denetle; durumlar fixtures/geodesy/v1 (ADR 0165 §3)
python3 scripts/fixtures/fit_parameters.py --check   # Vektör oturtma'nın Parametrelerle'sini (Y ve X ölçeği, dönüklük → afinin doğrusal parçası) kuraldan denetle (ADR 0156 §7)
cargo test -p kentos-desktop calc::fit::tests::screens -- --ignored --nocapture   # Vektör oturtma penceresinin resimleri, .run/shots/oturt-* (web'inkiler: node apps/web/scripts/e2e/shots.mjs vectorfit; ADR 0156 §7)
KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-desktop calc::edgematch::tests::screens -- --ignored --nocapture   # Kenar eşleme penceresinin resimleri, .run/shots/kenar-* (web'inkiler: node apps/web/scripts/e2e/shots.mjs edgematch; ADR 0159)
cargo test -p kentos-desktop second_crs::tests::screens -- --ignored --nocapture   # ikinci koordinat sisteminin durum çubuğu, menüsü, Koordinat oku ve Proje ayarları resimleri, .run/shots/ikinci-sistem-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs secondcrs); ADR 0167)
cargo test -p kentos-desktop calc::convert::tests::screens -- --ignored --nocapture   # Koordinat dönüştür penceresinin resimleri, .run/shots/donustur-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs convert); ADR 0167 §4)
python3 scripts/fixtures/point_dedupe_cases.py --check   # Çift noktaları ayıkla'nın gruplarını, ortalamasını, bağlı çizgilerini ve İçe aktar sonrası hedefini kurallardan denetle (ADR 0153 §5)
cargo test --release -p kentos-geometry-core --test curve_perf -- --ignored --nocapture   # 2 000 eğri ve 500 elips arasında kenet, tıklama ve kesişim penceresi süreleri (ADR 0149 §5.3)
pnpm e2e:cloud           # gerçek API/PostGIS cloud akışı
KENTOS_E2E_DB=scratch pnpm e2e:cloud   # aynı akış geçici veritabanında (bu yapının migration'ları), kentos_cad'e dokunmadan
KENTOS_E2E_SERVER=…/target/debug node apps/web/scripts/e2e/cloud.mjs   # aynı akış başka bir yapının sunucusuyla (ADR 0038)
node scripts/wgsl/browser-check.mjs   # paylaşılan WGSL'yi Chrome WebGPU'da derler ve çizer
cargo test -p kentos-render-wgpu --test styled_precision   # stilli çizimin uzak karolarda hassasiyeti, gölgelendiricinin float32 adımlarıyla (ADR 0157)
(cd apps/web && node scripts/e2e/shots.mjs vectorfit --renderer webgpu)   # sahneler web'in WebGPU'suyla (SwiftShader); resimlerin adı -webgpu ile biter
KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu   # gerçek GPU'da hassasiyet
nice -n 10 bash scripts/perf/rebuild-times.sh [rust|wasm|all]   # gerçek bir düzenlemeden sonra yeniden derleme süreleri: test ikilileri, masaüstü, WASM'ın iki profili (ADR 0170)
pnpm perf:interaction    # etkileşim ölçümleri
pnpm perf:kcad           # KCAD v2 kaydet/aç ölçümü, tarayıcıda (ADR 0030; docs/perf)
cargo test --release -p kentos-desktop perf::kcad -- --ignored --nocapture --test-threads=1   # aynı ölçüm masaüstünde
cargo test --release -p kentos-desktop perf::frame -- --ignored --nocapture --test-threads=1   # masaüstünün karesi büyük çizimde: olay, uygulama, görünüm, düzen, çizim ve GPU; KENTOS_PERF_OUT=docs/perf ile rapor (ADR 0120)
pnpm py:test             # Python SDK: üretici güncel mi, maturin'le derleme (.run/py), testler (328 ortak komut durumu dahil), clippy (ADR 0131)
cargo test -p kentos-desktop python:: -- --include-ignored   # masaüstünün Python konsolu; gerçek Python'la olanı .run/py ister (ADR 0132)
cargo test -p kentos-desktop python::tests::screens -- --ignored --nocapture   # Python konsolu resimleri gerçek çalıştırmalarla, .run/shots/python-konsol-*
cargo test -p kentos-mcp   # MCP sunucusu: yeni ve eski istemci, araçlar, redler, kaynaklar, stdio süreci (ADR 0133)
claude mcp add kentos -- $PWD/target/debug/kentos-mcp   # MCP sunucusunu bir istemciye tanıtma örneği (önce cargo build -p kentos-mcp)
python3 scripts/python/sdk.py   # katalog değişince kentos.cad'in tiplerini ve sarmalayıcılarını yeniden yaz; --check farkı arar
node crates/wasm/sheet-wasm/tests/smoke.mjs   # pafta çekirdeğinin WASM sonuçları Rust testlerinin beklediğiyle aynı mı (ADR 0164)
node apps/web/scripts/e2e/sheet-shots.mjs   # pafta kipinin web resimleri, apps/web/scripts/e2e/out/shots/sheet/
node apps/web/scripts/e2e/sheet-pdf.mjs   # paftanın PDF ve GeoPDF'i poppler ve GDAL ile (pdfinfo, pdffonts, pdftotext, gdalinfo)
node apps/web/scripts/e2e/sheet-cloud.mjs   # şablon kitaplığı gerçek kentosd ile, kendi geçici docker veritabanında (5432'ye dokunmaz)
cargo test -p kentos-sheet-ui --test screens -- --ignored   # masaüstünün pafta tasarımcısı resimleri, .run/shots/sheet-desktop/
.run/py/bin/python scripts/python/live.py   # SDK ve MCP sunucusu gerçek kentosd ile, geçici veritabanında (önce: cargo build -p kentos-api --bin kentosd --example e2e_database; cargo build -p kentos-mcp)
pnpm inventory           # web özellik envanteri: docs/inventory/web.{json,md}
pnpm inventory:check     # envanter güncel değilse düşer
pnpm db:setup            # yalnız yerel geliştirme DB/rolleri, migration, seed
pnpm api                 # kentosd serve; varsayılan 127.0.0.1:8787
pnpm kentosd -- <komut>  # yönetim CLI; yetkili hedefte bilinçli kullanılır
```

- Rust araç zinciri sabittir; `.cargo/config.toml` derlemeyi 4 işle sınırlar. Geliştirme ve test derlemeleri
  yalnız satır tablolarıyla (bağımlılıklar hata ayıklama bilgisiz) derlenir; bir crate'in entegrasyon testleri tek
  ikilidir (`tests/all/main.rs`, her dosya bir modül; `--test` ile ayrı çalıştırılanlar `tests/`'te kendi
  dosyalarında); yeni test dosyası `tests/all/`'a modül olarak eklenir. WASM paketleri geliştirme sunucusunda,
  testlerde ve e2e'de `wasm-dev` profiliyle, `pnpm build` ve perf betiklerinde gönderilen `wasm` profiliyle
  derlenir (ADR 0170).
  `wasm-bindgen-cli` sürümü workspace bağımlılığıyla aynı olmalıdır
  (şu an `0.2.128`). `pnpm rust:wasm`, `rust:wasm:formats`, `rust:wasm:svg`
  paketleri ayrı derler; üretilen `pkg/` içeriğini elle düzenlemeyin.
- `dev/test/build/e2e`, `scripts/wasm/ensure.mjs` ile kaynak değişimini denetler.
  Ağır cargo, tarayıcı e2e ve benchmark süreçlerini eşzamanlı koşturmayın.
- Vite `/v1/` ve `/v1/ws` isteklerini API'ye iletir. Ayarlar
  `apps/api/src/config.rs`: ortam değişkenleri, ardından `.env.local`.
  DB bağlantı bilgilerini, OIDC sırlarını ve token'ları log'a/depoya yazmayın;
  migration/owner hesabını runtime hesabından ayırın. Dosya projelerinin nesne
  deposu `KENTOS_BLOB_DIR`'dir (yoksa env dosyasının yanında `.run/blobs`);
  üretimde veritabanıyla birlikte yedeklenen diskte olmalıdır (ADR 0031).
- DB testleri için `KENTOS_TEST_ADMIN_URL`; DB zorunluluğu için
  `KENTOS_TEST_DB=required` kullanılır. Atlanan DB testi geçmiş test değildir.
  Kurulum/seed komutlarını üretime veya bilinmeyen veritabanına uygulamayın.
- Geliştirmede `window.kentos` tanı yüzeyi vardır; production'da yoktur.
  WebGL2 varsayılan, WebGPU tercihe bağlıdır; `?renderer=webgpu|webgl2`
  başlangıç tercihini değiştirir. WebGPU başlatılamazsa uyarıyla fallback olur.
- `kentos.ui.v1`, `kentos.settings.v1` (tipli ayarlar, ADR 0023), `kentos.processing.v1`,
  `kentos.styles.v1` localStorage anahtarlarıdır. Eski `kentos.prefs.v1` bir kez taşınır
  ve yedek olarak kalır; kurtarılan kayıt `kentos.settings.v1.backup`'tadır. Masaüstü
  ayarları `~/.config/kentos-cad/ayarlar.json`'dadır (vitrinin `ayarlar`'ı ayrıdır); yerleşimi (web'in
  `kentos.ui.v1`'i: dok, alt panel, İşlemler, şerit) yanında `yerlesim.json`'dadır (ADR 0115).
  Gönderilmemiş cloud taslakları IndexedDB `kentos.cloud/drafts` içindedir (biçim 2,
  ADR 0026; okunamayan taslak `<anahtar>#unreadable-<zaman>` altında ayrıca saklanır).
  Davet bağlantısının belirteci yalnız sekmede, sessionStorage `kentos.invitation`'da durur;
  kabul ya da pencere kapanınca silinir, localStorage'a yazılmaz (ADR 0042). Masaüstünün bulut
  taslakları `$XDG_DATA_HOME/kentos-cad/bulut-taslak`, projelerin yerel kopyaları
  `…/kentos-cad/bulut-kopya` altındadır (ADR 0040, 0043).
  Masaüstünün son dosyaları `$XDG_STATE_HOME/kentos-cad/son-dosyalar.json`'dadır (yoksa
  `~/.local/state/kentos-cad/`; yalnız yollar ve kısa bilgi, ADR 0050). Masaüstünün Kitaplığım'ı
  (web'de `kentos.styles.v1`) `$XDG_DATA_HOME/kentos-cad/kitaplik.kstil`'dedir; okunamayan dosya
  `kitaplik-okunamadi-<zaman>.kstil` olarak ayrılır, üzerine yazılmaz (ADR 0092).
  Yerel çizimin kaydedilmemiş işinin kurtarma kopyaları IndexedDB `kentos.recovery/copies`'tedir;
  masaüstünde `$XDG_DATA_HOME/kentos-cad/kurtarma` (yoksa `~/.local/share/kentos-cad/kurtarma`)
  altında, her çalışan KentOS'un kilitli klasöründe (ADR 0030). Pafta kitapları, bu cihazdaki pafta
  şablonları ve paftalardaki resimlerin baytları IndexedDB `kentos.sheets.v1`'dedir (sürüm 2: `books`,
  anahtarı `bulut/…`, `proje/…`, `dosya/…`, `oturum/…`; `templates`, hesabın bulut kitaplığının kopyaları
  da; `assets`, anahtarı SHA-256, yazılırken özeti denetlenir; sürüm 1'den yükseltme yalnız eksik depoyu
  açar); masaüstünde `$XDG_DATA_HOME/kentos-cad/pafta/` (yoksa `~/.local/share/kentos-cad/pafta/`;
  ADR 0164). NTv2 ızgaraları cihazındır, projeninki değil: IndexedDB `kentos.grids` (`grids` bilgiler, `bytes` baytlar; anahtar
  SHA-256), masaüstünde `$XDG_DATA_HOME/kentos-cad/izgara/<sha256>.gsb` ve yanında `<sha256>.json` (ADR 0168 §4).
  Hata ayıklarken kullanıcı verisini izinsiz silmeyin.

## 3. Teknik kısıtlar

- TypeScript strict, `erasableSyntaxOnly`, `verbatimModuleSyntax`: `enum`,
  `namespace`, constructor parameter property kullanmayın; `import type`,
  union ve `as const` tercih edin. Web araç zinciri pnpm/Vite/TypeScript'tir.
- Web UI çatısı DOM `h()` (`ui/dom.ts`) ve `core/signal.ts`'tir.
  React/Vue/Lit geçişi yapmayın.
- Yeni runtime bağımlılığı veya sürüm değişikliği bakım, lisans, platform ve
  ölçüm gerekçesi ister; mevcut küçük/pure çekirdek sınırını koruyun.
  Yeni web runtime kütüphanesi küçük, tree-shaking'e uygun, MIT/BSD lisanslı,
  bakım altında ve DOM gerektirmeden worker'da kullanılabilir olmalı;
  eklemeden kullanıcı onayı alın. Rust sürümleri workspace/lockfile politikasına uysun.
- Tarayıcının ayırdığı `Ctrl+N/T/W`, `Ctrl+Shift+T`, `Alt+F/D/E`
  kısayollarını uygulama komutuna bağlamayın.

## 4. Web mimarisi

### 4.1 Katmanlar

Temel sıra: `core → geo → model → style/processing/product → render → viewport → tools → ui → app`.
Bu bir yerleşim sırasıdır: sonraki katman öncekini kullanır; ters yönde runtime
bağımlılığı kurulmaz. `AppContext`, viewport/tool arayüzleri için mevcut
`import type` istisnalarını koruyun; somut servisleri yukarıya bağımlı kılmayın.
`model`, `style`, `processing` DOM/UI bilmez; `render` UI/tools bilmez.
`io/` model/contracts düzeyinde, `wasm/` hesap bağlayıcısıdır. `product/` ürün
komutlarıdır (ADR 0013, 0022): belgeyi alır, DOM/UI bilmez; araçlar çağırır.
`contracts/generated/` Rust'tan üretilir, elle düzenlenmez.

### 4.2 Kompozisyon kökü

`app/createApp.ts` servisleri kurar. Tema/font ölçeği palette okumasından önce
uygulanır; yeni global singleton veya ikinci belge sahibi oluşturmayın.

### 4.3 AppContext

Özellikler servisleri `app/context.ts` arayüzünden alır. Somut kurulum ve
lifecycle kompozisyon kökündedir; reusable widget'ları `AppContext`'e bağlamayın
(mevcut komuta bağlı `CommandButton` istisnası korunur).

### 4.4 Durum kapsamları

| Kapsam | Sahip / kalıcılık |
|---|---|
| Proje verisi/ayarı | `CadDocument`, `doc.settings`; proje dosyası/cloud revision |
| Kullanıcı tercihi | `ctx.prefs` (tipli ayarların cephesi, `ctx.settingsStore`); web `kentos.settings.v1`, masaüstü `ayarlar.json` |
| Cihaz | grafik (`graphics.*`: arka uç, MSAA, HiDPI); başka cihaza taşınmaz. Kalite ayarları yalnız çizim alanını düşürür; arayüz, kaplamalar, önizlemeler ve etiketler her zaman tam kalitededir (sahibin kararı, ADR 0023 eki) |
| Yerleşim | `ctx.ui`; kullanıcıya özgü panel/ribbon düzeni |
| Oturum | seçim, etkin araç, drafting durumu, pano; proje verisi değil |

Çözüm sırası: varsayılan → kullanıcı → cihaz → oturum; proje ayarı yalnız projeden gelir;
kurum politikası üst sınırdır (ADR 0023).
Proje varsayılanını değiştirmek açık projenin değerini değiştirmez.
Proje değişikliği dirty/revision üretir. Ayar pencereleri taslakla çalışır;
Kaydet'te uygular. Aynı ayarı hem proje hem uygulama penceresinde çoğaltmayın.

### 4.5 Komutlar

`core/commands.ts` registry, `app/commands.ts` ve ilgili app modülleri handler
kaydeder. Menü, ribbon, kısayol ve komut satırı aynı command ID'yi çağırır.
`isEnabled/isChecked` bağımlılıklarını `watch` ile bildirin. Henüz olmayan
özellik `pending(...)` ile açıkça belirtilir; sessiz no-op düğme koymayın.
`app/menus.ts` ve `tools/catalog.ts` tek kaynaklardır; ribbon'a ayrı araç listesi
yazmayın. Mevcut UI registry, hedefteki tam async/headless command bus değildir.
Ürün komutları ayrı düzeydir (ADR 0013, 0022): katalog `contracts/generated/commandCatalog.json`,
işleyiciler `product/` (web) ve `crates/native/application` (masaüstü); iki kayıt katalogla,
davranış `fixtures/commands/v1` ile eşit tutulur. Girdi örtük arayüz durumunu okumaz (CMD-07);
sonuç `CommandResult`'tır.

### 4.6 Klavye ve odak

`e.key` ile Türkçe Q/F klavyeyi koruyun. Metin alanlarında yalnız izinli
kısayollar çalışır; dialog tuşları uygulamaya sızmaz. Aktif araç seçenekleri
ilgili harflerde önceliklidir; Enter/Boşluk odaklı kontrolün yerel anlamını korur.
Koordinat yazmaya başlamak command/dinamik girişi açar; desktop aynı davranışı
hedefler. Kısayol yardımı kayıt üzerinden üretilir.

### 4.7 Araçlar

`tools/catalog.ts` yeni aracın kimlik, grup, yöntem, kısayol ve `steps` kaynağıdır.
Araç DOM'a dokunmaz; mevcut `Tool`, araç aileleri ve `ctx.view` arayüzünü kullanır.
Akış/istem/önizleme TS'te, hesap Rust'tadır. Enter/onay, Esc/iptal ve nested
şeffaf araç davranışı korunur; iptal taslağı kalıcı geometriye dönüştürmez.
Kenet pointerdown/up'ta yeniden hesaplanır; eski pointermove sonucuna güvenilmez.
Masaüstünün karşılığı `kentos_interaction`'dır (ADR 0021): iki taraf `fixtures/interaction/v1`
izlerini ve yazılan değerin `fixtures/point-input/v1` dilbilgisini geçer; davranış
değişikliği ADR 0018'in sırasıyla yapılır. Kenet ve seçim iki platformda aynı Rust
deposundan gelir; masaüstü depoyu belgenin günlüğüyle (`changes_since`) izler, olay
başına yeniden kurmaz (ADR 0029). Araçların oturumlar arası hatırladıkları (web'de statik alanlar)
masaüstünde `Memory`'dir; iz onu başladığı gibi bırakır (ADR 0032).
Masaüstünde araç kamerayı kendisi değiştirmez; `Context.view_changes`'a `ViewChange` ekler, kabuk
çağrıdan sonra uygular. Pano oturumun durumudur, sistem panosu kullanılmaz; kesme web'deki gibi
`cad.entities.delete` ile “Kes” adımında siler, yapıştırma `cad.entities.create` ve `cad.entities.set` ile
“Yapıştır” adımında yazar (ADR 0056, 0074).

### 4.8 Belge ve kayıt

`CadDocument.add/update/remove`, toplu karşılıkları ve `transact` tek mutation
yoludur. Transaction hata verirse rollback; nested işlem savepoint; çoklu edit
tek mantıksal undo adımıdır. Katman ve grup eklemek ve silmek de geri alınabilir adımdır; içe
aktarmanın açtığı katmanlar onun adımındadır (ADR 0072, 0076). Async grup işlemlerinde mevcut
group/cancel yolunu kullanın.
`markSaved(revision)` yalnız gerçekten kaydedilen güncel sürümü temizler.
Kaydetme sürerken yapılan yeni değişikliği dirty=false yapmayın.
`.kcad` KCAD v2'dir (`DocumentSnapshotV2`, ADR 0025): web biçim işçisinde, masaüstü yerel
olarak aynı Rust kodeğiyle yazar; baytlar geri okunup doğrulanmadan dosyaya yazılmaz.
v1 JSON okunur ama üzerine yazılmaz: Kaydet v2'nin yerini sorar. Tür içerikten anlaşılır;
okuyucu sürüm/alan/SRID/kimlik doğrular.
`replaceWith` öncesi aday belge doğrulansın; başarısız açılış mevcut işi kaybettirmesin.
Büyük çizimde (ADR 0030) nesneler işçiye tipli sütunlarla geçer (`io/columns.ts` ↔
`crates/shared/kcad/src/columns.rs`; düzen iki tarafta ve `FORMATS_VERSION` ile birlikte değişir);
işçi baytları gönderilen sütunlarla ve başla karşılaştırır. Açılış aşamalı ve durdurulabilirdir:
belge yalnız bütün dosya okunup denetlenince tek adımda değişir; durdurulan, geride kalan ya da
sürerken çizimi değişen açılış hiçbir şeyi değiştirmez; açılış sürerken komut çalışmaz.
Kaydedilmemiş yerel çizimin kurtarma kopyası hiçbir `.kcad` dosyasının içinde ya da yanında değildir;
kayıt ya da bilerek bırakma siler, geri yüklenen kopya dosyasız ve kaydedilmemiş açılır.
Masaüstünün karşılığı `kentos_domain::Document`'tir; iki belge `fixtures/document-ops/v1`'i
geçer, davranış değişikliği fixture'la birlikte yapılır (ADR 0020).
Her nesnenin kalıcı `uid`'i vardır (ADR 0014): yeni nesne yeni `uid` alır, düzenleme ve
geri alma korur, `replace` yuvayı ve kimliği tutar. v2 her nesnenin `uid`'ini yazar ve korur;
v1 yazmaz, açılışta içerikten türetilir, v2 kaydı göç kaynağını ve proje kimliğini yazar. Bulutta nesnenin kimliği `uid`'idir (ADR 0026): açılış sunucunun
kimliğini `uid` yapar, `applyExternal` gelen kimliği alır ve aynı kimliğe dokunan geri
alma adımlarını düşürür; ayrı eşleme kurmayın.

### 4.8.1 Hesaplama çekirdeği

`model/geom`, `model/ops` ve diğer hesap cepheleri Rust çağrı adapter'larıdır.
Geometri algoritmasını yeniden TS'e yazmayın. `singleSource.test.ts`, native/WASM
fixture'ları ve bağımsız referanslar korunur. Yeni hesap için §14 ve ADR 0008'i izleyin.

### 4.9 Çizim hattı

- `RenderBackend` sözleşmesini ve WebGL2/WebGPU davranış uyumunu koruyun.
- CPU kaynak koordinatı f64; GPU'ya yerel orijine göre float32 fark gider. Stilli çizimde yerel orijin, konumun çapaya ortalanmış 65,536 km'lik karosunun merkezidir; kamera kaba ve ince iki parça gider (ADR 0157).
  Mutlak büyük dünya koordinatını GPU'ya yüklemeyin.
- Kirli katmanlar güncellenir; seçim/hover/ızgara ayrı cache/katmanlardır.
  Masaüstünde seçim ve üzerine gelme ayrı wgpu sahne parçasıdır; kenet işareti ve
  seçim kutusu Iced canvas'ıdır (ADR 0029). Iced her olayda bütün pencereyi çizer;
  masaüstünün çizim alanı resmini tutar ve yalnız gösterdiği değişince sahneyi yeniden
  çizer, üzerine gelme vurgusu üstüne çizilir. Yazıların sorgusu ve çok nesneli
  Öznitelikler de değişmeyen karede yeniden bulunmaz (ADR 0120). Çizimin yazıları da, web'deki gibi sahnenin
  üstünde Iced canvas'ıdır; neyin nerede çizileceğini ortak depo söyler (ADR 0055).
  `requestRender/requestOverlay` istekleri birleşir; her olayda tam belge rebuild yok.
- Canvas resize sonrası siyah kareyi önleyen mevcut senkron redraw istisnasını koruyun.
- Pick/snap/geometri kararları `PickIndex` → Rust store üzerinden gelir.
  Renderer kaynak geometriyi yuvarlamaz veya kalıcı belgeyi değiştirmez.

### 4.10 Arayüz

`Component` ve `DisposableStore` ile abonelik/dinleyici yaşam döngüsünü kapatın.
Uzun listelerde mevcut `TreeView`/`VirtualRows`'u kullanın; editte bütün ağacı
yeniden kurup odak/yeniden adlandırmayı kaybettirmeyin. Renk/font/ölçek DESIGN.md'dendir.
Uygulama içi sorular `ui/widgets/confirm.ts` (`confirmDialog`, `askUnsaved`,
`askRemove`) üzerinden sorulur; `alert/confirm` veya durum satırında soru yok.
Esc/×/arka plan vazgeçtir. Değişmemiş yeni düzenleyici için kaydet sorusu çıkarmayın.

### 4.11 Processing

`defineTool` metadata'sı UI/komut/parametrelerin kaynağıdır. Hesap salt okunur
girdiyle çalışır, `ChangeSet` üretir; runner sonucu doğrular ve tek undo adımı
uygular. Web Worker server worker değildir. İfade dilini genel JS/Python `eval`
ile değiştirmeyin. Ayrıntı: [docs/PROCESSING.md](docs/PROCESSING.md).

### 4.12 Proje türleri

Proje ya CAD ya CBS türündedir (ADR 0165; `app/workspaces.ts`, masaüstünde `modes.rs`);
Hibrit yoktur. Tür proje ayarıdır; sahneyi, eksen ve açı düzenini ve şeridi belirler, veri
modelini ve hesapları değiştirmez. Türü sorulmamış proje (eski Hibrit dahil) CBS gibi
gösterilir ve kullanıcının açtığında bir kez sorulur; başsız sunucu, Python ve MCP sormaz.
Şeritte olmayan komutun kısayoldan erişilebilir olması yetki verildiği anlamına gelmez;
tür güvenlik sınırı değildir.

## 5. Koordinat, birim ve gösterim

- Kodda `x = doğu`, `y = kuzey`; Türk ölçmeciliği UI'ında sıra
  `Y (sağa), X (yukarı)`dır. Noktalı ondalık ve `Y,X` girişi korunur.
- Her proje CRS/SRID taşır; metadata kaynağı `geo/crs.ts`'tir.
  SRID atamak koordinat dönüşümü değildir. Kaynak CRS bilinmiyorsa sorun;
  sessiz reprojection/fallback veya tahmin yapmayın.
- Geometri açısı ile kuzeyden saat yönünde ölçmecilik semtini karıştırmayın.
  Birim dönüşümü açık; metre, m², dönüm (1000 m²), hektar (10000 m²).
- UI sayıları `ctx.format` üzerinden gösterilir; `toFixed` iş kuralı değildir.
  Gösterim basamakları kaynak koordinatı değiştirmez; ayrıntılı doğruluk §23'tedir.

## 6. Performans

### 6.1 Bütçeler

Hedef/ölçüm kaynağı ADR 0005 ve `docs/perf/`'dir; ADR taslak durumunu korur.
Donanım/driver/browser/kalite/commit bilgisi olmadan karşılaştırma yapmayın.
TODOS.md §20 güncel kabul kapsamıdır; eski tile hedefleri bugünkü cloud kapısı değildir.

### 6.2 Sıcak yol kuralları

Pointermove/frame içinde allocation, JSON/FFI, bütün belge gezisi ve tam GPU
upload'ı azaltın; toplu tipli veri kullanın. Uzamsal sorgu indeksten, uzun iş
worker'dan, uzun liste sanallaştırmadan geçer. DOM okuma/yazmayı ayırın.
Optimizasyonu aynı veri ve kalite düzeyinde ölçün; geometri/etiketi gizlice eksiltmeyin.

### 6.3 Durum ve darboğazlar

Rust pick/snap deposu, katman dizinleri, kirli güncelleme ve ağaç sanallaştırması
mevcuttur; bunları yeni yapılacak işler gibi yeniden kurmayın. Güncel darboğazı
profil çıkararak belirleyin; eski tek-makine ölçümlerini bugünün sonucu diye aktarmayın.

## 7. CAD/GIS veri doğruluğu

Belge değişiklikleri geri alınabilir ve kilitli katman kurallarına uygun olmalı.
Kilitli katmandaki nesnenin kopyası da yapılmaz; dizilerde de (ADR 0037, 0047). Düzenleme (`cad.entities.edit`) bütün
yazılır ya da hiç: bir nesnesi kilitliyse hepsi reddedilir (ADR 0047).
Gizli katmana çizimde uyarı, atlanan/kayıplı nesnelerde açıklanabilir rapor gerekir.
Geometrik alan, kayıtlı/hukuki alan ve gösterim değeri ayrıdır; birbirini
otomatik ezmez. CAD analitik kaynak ve GIS izdüşümü §15'e uyar.
Topoloji, parsel/hisse ve resmî çıktılar bağımsız referans/kurum kabulü olmadan
tamamlanmış veya mevzuata uygun diye sunulmaz.

## 8. Kod kuralları

Çevredeki adlandırmayı/üslubu koruyun; yorum nedenini anlatsın. Tek dosya tek
sorumluluk taşısın; başında kısa amaç yorumu olsun, 400 satır üstünde ayrıştırmayı değerlendirin.
Signal'i sahibi değiştirir; UI command/belge API'sini kullanır.
Sabit UI rengi, vurgu, font boyu veya CDN fontu eklemeyin; CSS jetonları,
`readCanvasPalette()` ve yerel fontları kullanın. Katman rengi veridir;
renderer/UI davranışını sabit katman ID'sine göre dallandırmayın.
Hata mesajı nedeni ve çözümü söylesin. Global listener/GPU/worker kaynaklarının
dispose/cancel yolu olsun. Kullanıcının mevcut değişikliklerini koruyun.

### 8.1 Tamamlanma

İlgili testler, sözleşme/undo/yetki, UI gerekiyorsa gerçek klavye/fare ve
tema/yazı ölçeği kontrolü yapılır. Çalıştırılmayan kontrol ve nedenini bildirin.
Üretim iddiası veya tamamlandı işareti için TODOS.md kabul kapılarını kullanın.
Komut, araç, ayar, pencere, depo ya da `.kcad` alanı değiştiyse `pnpm inventory`
çalıştırın; farkı okuyup değişiklikle aynı commit'e koyun (docs/inventory/README.md).

## 9. Yeni iş ekleme tarifleri

### 9.1 Komut

Registry/handler, enable/watch, gerekiyorsa alias/kısayol ve menü kaydı ekleyin;
aynı iş için UI ve otomasyonda ayrı iş kuralı yazmayın.

### 9.2 Araç

Mevcut araç ailesini seçin; `tools/catalog.ts`, `steps`, preview/input/iptal,
kilitli katman ve tek undo testlerini tamamlayın. Hesabı ortak Rust'a ekleyin.

### 9.3 Ayar

Önce kapsamını belirleyin (§4.4); tip, varsayılan, validasyon, migration,
kalıcılık ve live-apply davranışını birlikte ekleyin. Ayar
`crates/shared/contracts/src/settings/schema.rs`'e eklenir, `KENTOS_WRITE_SETTINGS=1 cargo test
-p kentos-contracts settings` ile üretilir; yeni kural `fixtures/settings/v1` ve iki
çalıştırıcıyla birlikte değişir (ADR 0023).

### 9.4 Test

- Web davranışında `pnpm typecheck`, ilgili Vitest testleri ve kapsamına göre
  `pnpm test/build`; UI değişiminde tarayıcı e2e ve açık/koyu/Büyük yazı kontrolü.
- Rust/hesap değişiminde ilgili native testler, clippy, WASM fixture/bağımsız
  referanslar; sözleşme değişiminde Rust → TS üretimi ve drift denetimi.
  Sözleşme tipi ürün komutu kataloğunun şemasını da değiştirir:
  `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts catalog` (ADR 0013).
- DB/cloud değişiminde gerçek PostGIS entegrasyonu ve `KENTOS_TEST_DB=required`;
  offline/retry/conflict/tenant/permission senaryoları. Atlananları açıklayın.
- Golden/visual fixture güncellemesi otomatik onay değildir; farkı okuyun,
  algoritmanın kendi çıktısını tek doğruluk kanıtı saymayın. Toleransı büyütmeyin.
- Yalnız doküman değişiminde bağlantı, komut, durum ve tutarlılık kontrolü
  yeterlidir; kod testleri çalışmadıysa çalışmış gibi raporlamayın.

### 9.5 Render backend

WebGPU/WGSL ve WebGL2/GLSL yollarını aynı sahneyle doğrulayın; renk/alpha,
dash, piksel ölçeği, AA ve device-loss davranışını koruyun. Headless SwiftShader
sonuçları gerçek GPU benchmark'ı değildir; mevcut `apps/web/scripts/e2e/cdp.mjs` ayarlarını kullanın.

### 9.6 Processing aracı

`processing/builtin/` içinde bildirim ve katalog kaydı; pure hesap testi ve
runner üzerinden belge testi ekleyin. Ayrıntı `docs/PROCESSING.md`'dedir.

### 9.7 Dosya biçimi

Reader/writer `crates/shared/formats`, şema `contracts`, dar binding
`crates/wasm/formats-wasm`, web akışı `io/` ve `app/fileExchange.ts` içindedir.
DXF (`crates/wasm/dxf-wasm`) ve Netcad NCZ (`crates/shared/ncz`, `crates/wasm/ncz-wasm`)
kendi modüllerindedir; biçim işçisi onları ilk gerektiğinde yükler, sonuçları tipli sütunlarla
geçer ve büyük içe aktarma iki platformda kare kare, tek geri alma adımında yazılır (ADR 0138).
NCZ okuyucusu GPL-2.0-or-later bir kaynaktan türetildi: yazarın izni gelene dek onu içeren
derleme dağıtılmaz (TODOS.md `NCZ-01`).
Bozuk/kötücül girdide panic yok; boyut, nesting/karmaşıklık ve iptal sınırı vardır.
CRS sorulur, kayıp raporlanır, import tek undo olur; Rust ve WASM aynı fixture'ı
okur. Ayrıntı ADR 0009. GeoJSON (okuma/yazma) ve Shapefile (okuma) aynı yoldadır (ADR 0046):
kurallar ADR'de, bağımsız okuyucu `tools/formats/gis.py`, fixture'lar `fixtures/formats/v1/gis`;
dosyanın dediği koordinat sistemi gösterilir, projeninkinden başkaysa içe aktarma kapalıdır,
dönüşüm yoktur. Proje dosyası değişim biçimi değildir: kodek `crates/shared/kcad`,
spesifikasyon `docs/specs/kcad-v2.md`, karar ADR 0025; değişiklik spesifikasyon,
`fixtures/kcad/v2` (bağımsız Python yazıcısıyla), Rust kodeği ve `tools/kcad/kcad.py`
ile birlikte yapılır.

## 10. Yol haritası

Tek ayrıntılı yol haritası [TODOS.md](TODOS.md)'dir. Buraya ikinci checkbox
listesi, eski Faz A–F sırası veya her tamamlanan commit'in dökümünü eklemeyin.
Yapılmış işin durumunu §1'de kısa tutun; kanıtı test/ADR/ölçümde saklayın.

### 10.1 Devir notu (3 Ekim 2026)

İşi devralan için kalınan yer; bir sonraki devirde bu bölümü yenileyin.

- Bitti (3 Ekim, sahibin isteği: “hibrit mod iptal … sahne kurma CAD ve gis için ayrı … proje oluşturma sihirbazla … menüleri bu
  yapılara göre düzenleyelim”): proje türleri ([ADR 0165](docs/adr/0165-project-types-cad-gis.md), TODOS.md `PRJ-01`). 1. adım
  (Hibrit kalkar) iki platformda: sözleşmede `hybrid` yalnız okunur (`Workspace::LegacyHybrid`, `ProjectSettings::project_type`),
  belge ve v1 göçü onu yazmaz; türsüz proje CBS gösterilir ve kullanıcının açtığında bir kez sorulur (web `ProjectTypeDialog.ts`,
  masaüstü `project/ask_type.rs`); ayarlarda emekli değer (`RETIRED_VALUES`); envanterde `layout.ribbon` CBS'nin, `ribbonByMode`
  her türün; pafta profillerinde hibrit satırı yok; arayüzde “Çalışma modu” yerine “Proje türü”, bulut kataloğunun etiketi “İş
  türü”. 2. adımın yerel sistemi (SRID 0, “Yerel (koordinat sistemi yok)”) ve çizim birimi (`settings.drawingUnit`, `.kcad`
  şema 11; geometri metrede, birim yalnız okuma ve yazmada: `Formatter.toMetres`, `Format::to_metres`, `Context::typed_point`,
  `typed_length`, `point_from_text_in`; Hesap pencereleri ve İşlemler metrede) bitti; DXF iki yönde birimiyle (2c,
  `formats::units`) bitti. 3. adımdan ileri TM izdüşümü (`geodesy::tm_forward`, PROJ başvurusuyla), 81 il, ana görünüm, CAD'in
  katman şablonu ve web'in sihirbazı (`ui/settings/NewProjectWizard.ts`, kuralları `model/newProjectWizard.ts`, ortak
  `fixtures/project/v1/wizard.json`), masaüstünün sihirbazı (`project/wizard/`, kuralları `kentos_project::wizard`, KentOS UI
  `Wizard::rail`) ve yeni projelerin birimi (`newProjects.drawingUnit`) bitti; 3. adım tamam. 4. adım (eksen ve açı düzeni) bitti:
  kutupsal giriş türün düzeniyle (`point_input::polar_point_in`, `fixtures/point-input/v1`'in `convention`'ı), eksen adları ve
  doğrultular biçimlendiricide (`Format::axes`, `Formatter.axes`), yeni CAD projesi derece. 5. adım (sahne) bitti: CAD'de koordinat
  ekseni simgesi (`map_marks.rs`, `drawUcsIcon`) ve koyu zemin (“Türe göre”, web'de `data-canvas`), durum çubuğunda ölçek seçici
  (`screen_scale.rs`, `scaleSelector.ts`). 6. adım (her türün kendi şeridi) bitti: CAD Dosya, Giriş, Ekle, Açıklama, Değiştir,
  Görünüm, Yönet, Çıktı; CBS Dosya, Giriş, Harita, Veri, Düzenle, Analiz, Ölçme, Görünüm, Çıktı (web `app/ribbon.ts`'in
  `CAD_RIBBON_TABS`, `GIS_RIBBON_TABS`; envanterin `ribbonByMode`'u; ortak `fixtures/shell/v1/ribbon.json`). Resimlerde türü olan
  sahne `make_cad` (masaüstü testleri) ya da `shots.mjs`'in `type` alanıyla kurulur; web'in resim grubu `types`.

- Bitti (30 Eylül): yazı ekleri ([ADR 0145](docs/adr/0145-text-extras.md), TODOS.md `CAD-15`, `.kcad` şema 7),
  1–5. adımlar iki platformda: sözleşme ve şema 7; çekirdek ve çizim; komutlar; araçlar ve arayüz (Yazı'nın
  Hiza, Genişlik, Zemin ve Artır seçenekleri, Öznitelikler'in satırları, komut satırının “Diğer” çipi, Okunur yap,
  Bul ve değiştir, Metin dosyası yerleştir); biçimler (DXF okuma ve yazma, NCZ çapaları; `texts.dxf` ve
  `dxf-write/texts` bağımsız denetimle). Ondan önce blok (ADR 0144, `CAD-04`), çok parçalı alan (ADR 0143,
  `CAD-14`) ve köşe kotu (ADR 0142, `CAD-13`).
- Bitti (1 Ekim): kılavuz ([ADR 0146](docs/adr/0146-leader.md), TODOS.md `CAD-16`, `.kcad` şema 8), 1–5. adımlar iki
  platformda: sözleşme ve şema 8; yerleşim ve çizim; komutlar; Kılavuz aracı, Öznitelikler ve notun yerinde
  düzenlenmesi (ortak iz `leader.json`); DXF LEADER (bağlı MTEXT'iyle) ve MULTILEADER okuma, LEADER ve MTEXT yazma
  (`leaders.dxf`, `dxf-write/leaders` bağımsız denetimle). Arayüzdeki adı “Kılavuz”dur (sahibin seçimi).
- Bitti (1 Ekim): yeni ölçü türleri ([ADR 0147](docs/adr/0147-new-dimension-kinds.md), TODOS.md `CAD-17`, `.kcad`
  şema 9), 1–5. adımlar iki platformda: Koordinat, Yay uzunluğu, Kırıklı yarıçap, Semt, Eğim; ölçü değerinin zemini;
  Açı'nın yaydan ve daireden yolları, döndürülmüş doğrusal, Hızlı ölçü; hepsi şeritte Ölçülendirme ▾ (büyük düğme, her
  türün ikonu); yerleşimler bağımsız başvurularla (`fixtures/dimension/v1`), ortak izler; DXF koordinat (DIMENSION 6),
  ARC_DIMENSION ve LARGE_RADIAL_DIMENSION iki yönde, semt ve eğim hizalı ölçü olarak KentOS verisiyle, zemin DIMTFILL
  (`dimension-kinds.dxf`, `dxf-write/dimensions` bağımsız denetimle). Grupların anlamı AutoCAD'in kendi dosyasından
  doğrulandı; ezdxf'in ve LibreDWG'nin kırıklı yarıçap adları yanlıştır (ADR 0147 §10.5).
- Bitti (1 Ekim, sahibin bildirimi üzerine): ölçü doğruluğu ve gösterim yuvarlaması
  ([ADR 0149](docs/adr/0149-measurement-accuracy-and-display-rounding.md), TODOS.md `NUM-11`): tek gösterim kuralı
  (`display::fixed`, `core/displayNumber.ts`), 224 durumluk bağımsız ölçü denetimi (eğri uzunluğu ve basık elips düzeltildi),
  hesaplarda eğri ve elipsin 0,1 mm'lik açık sınırla temsili (`geom::curve_outline`).
- Bitti (1 Ekim): topolojik temizlik ([ADR 0148](docs/adr/0148-topology-cleanup.md), TODOS.md `HYB-01`), 1–3. adımlar
  iki platformda: çekirdek `ops::topology` (bağımsız başvuru `topology_cases.py`, 74 ortak durum); `cad.entities.edit`'in
  `topology` işlemi; Topolojik temizlik aracı (Değiştir › Nesne ▾; tolerans, Uçlar, Köşeler, Uzat, Buda; önizleme ve
  imlecin yanında sayılar; ortak iz `topology.json`).
- Sürmekte: TODOS.md §16'nın sırası (sahibin kararı, 1 Ekim): önce hibrit (§16.0), sonra CAD (§16.1), sonra CBS (§16.2).
  `HYB-02` çizgi ağından toplu alan ([ADR 0151](docs/adr/0151-polygonize.md)) bitti: çekirdek `ops::polygonize` (bağımsız
  başvuru `polygonize_cases.py`, 58 ortak durum), `cad.entities.create`'in `polygonize` işlemi, Toplu alan aracı iki
  platformda (ortak iz `polygonize.json`). `HYB-03` ölçü noktası ([ADR 0152](docs/adr/0152-survey-points.md)) bitti (2 Ekim):
  `#ad` dilbilgisi ve `ops::vertex_points` bağımsız başvurusuyla, `cad.entities.create`'in `vertexPoints` işlemi, Nokta'nın Ad, Kod
  ve Kot'u, aynı yer sorusu, `#ad` çözümü (adlı noktanın yeri araca tıklanmış gibi verilir) ve Köşelere nokta iki platformda
  (ortak izler `survey-points.json`, `vertex-points.json`; iz biçimi etiketi, öznitelikleri ve kotu da denetler). Sıradaki
  `HYB-04` nokta editörü ([ADR 0153](docs/adr/0153-point-editor.md)) bitti (2 Ekim): çekirdek (doğal sıra, tablo, çift
  noktalar, bağlı köşeler, bağımsız başvurusuyla); alt panelin Noktalar sekmesi (sıralama, süzgeç, seçim, Göster); yerinde
  düzenleme, Bağlı çizgiler izler, Satır ekle, Sil (`edits.json`); İşlemler ▾ ve satırın menüsü: Yeniden adlandır, Sıralı
  numara ver, Katmana taşı (`batch.json`), Çift noktaları ayıkla ve Çiftleri göster (`dedupe.json`), Dışa aktar (tablonun
  satırları, sırasıyla) ve İçe aktar (aynı adlar için ardından Çift noktaları ayıkla, Aynı ad; `dedupe.json`'un `imports`'u).
  `HYB-05` vektör oturtma ([ADR 0156](docs/adr/0156-vector-fit.md)): 1. adım (çözüm: `ops::fit`, bağımsız başvuru `fit_cases.py`)
  ve 2. adım (nesnelerin dönüşmesi: `ops::warp`, `warp_cases.py`) ve 3. adım (`cad.entities.transform`'un `similarity`, `affine`,
  `projective` türleri, adım “Oturt”), 4.1 (Vektör oturtma penceresi iki platformda: Kullan sütunu, Adla eşle, canlı çözüm,
  Uygula, rapor; sahne `vector-fit.kcad`) ve 4.2 (Parametrelerle: `scale_turn`, `fit_parameters.py`) tamam. 4.2'de bulunan
  çizim hattı sınırı da kapandı ([ADR 0157](docs/adr/0157-styled-far-tiles.md), `REN-07`'nin eki): stilli çizimin konumları
  karolarının merkezine göre, iki platformda ve üç çizim hattında (wgpu, WebGPU, WebGL2); en derin yakınlıkta milimetre altı
  `REN-16`'da açık. Kauçuk levha ([ADR 0158](docs/adr/0158-rubber-sheet.md)): 1. adım (ince plaka eğrisi `ops::rubber`,
  `rubber_cases.py`), 2. adım (nesneler: yalnız köşeler, `rubber_warp_cases.py`) ve 3. adım (`cad.entities.transform`'un
  `rubbersheet` türü, adım “Kauçuk levha”; durumlar levhanın Python ikiziyle bit bit, `sheet_f64.py`) ve 4. adım (Vektör
  oturtma'nın dördüncü dönüşümü, Sabit, yerel düzeltmeler) tamam. Kenar eşleme ([ADR 0159](docs/adr/0159-edgematch.md), `HYB-05`'in
  kalanı): 1. adım (çekirdek `ops::edgematch`, `edgematch_cases.py`), 2. adım (`cad.entities.edit`'in `edgematch` işlemi, adım
  “Kenar eşle”) ve 3. adım (pencere iki platformda, sahne `edgematch.kcad`) tamam; `HYB-05` bitti. `HYB-06` topolojik düzenleme
  ([ADR 0160](docs/adr/0160-topological-editing.md)): 1. adım (çekirdek `ops::topology_edit`: `apply`, `changes`;
  `topology_edit_cases.py`), 2. adım (durum çubuğunda Topoloji ve Noktalar da, tutamaçlar komşularla tek adımda, önizleme;
  ortak iz `topology-edit.json`) ve 3. adım (tutamaç menüsü, Esnet, kartta köşe sayısı) tamam; `HYB-06` bitti. `HYB-07` izleyerek
  çizim ([ADR 0161](docs/adr/0161-trace-drawing.md)): 1. adım (çekirdek `ops::trace` ve `ops::join::chain`, `trace_cases.py`) ve 2. adım
  (yol aracında İzle iki platformda, ortak iz `trace-draw.json`), 3. adım (Birleştir'in Zincir seçeneği, ortak iz `join-chain.json`)
  ve 4. adım (yol aracında Akış ve Adım boyu, ortak iz `stream-draw.json`) tamam; `HYB-07` bitti. `HYB-08` bitişik alan ve çakışma
  denetimi ([ADR 0162](docs/adr/0162-adjoining-areas.md)): 1. adım (çekirdek `ops::adjoin`, `adjoin_cases.py`) ve 2. adım (Çakışma kipi
  iki platformda, durum çubuğunda Çakışma; ortak iz `overlap.json`) , 3. adım (`cad.entities.create`'in `adjoin` işlemi, Bitişik alan aracı
  iki platformda; ortak iz `adjoin.json`) ve 4. adım (Topoloji açıkken komşulara köşe: çekirdek `junctions`, araçlar iki platformda; ortak iz
  `junctions.json`) tamam; `HYB-08` bitti. Sahibin isteğiyle araya girenler de bitti: PR 16 (KentOS UI) birleşti, alt panelin Python
  konsolu KentOS UI'ın `PythonRepl` ve `PythonEditor`'ü oldu (ADR 0132'nin eki), nokta editörünün arama kutusu düzeldi. `HYB-09`
  kenet ekleri ([ADR 0163](docs/adr/0163-snap-additions.md)): 1. adım (çekirdek: dört tür, `snap_ex`, çizilmekte olan yol, katman
  maskesi; `snap_cases.py`) ve 2. adım (ayarlar, türlerin komutları, Kenet hücresinin menüsü, karelaj aralıkları, ölçek aralığı, yol
  araçlarının taslağı, işaretler; ortak iz `snap-additions.json`), 3. adım (Uzantı ve Paralel'in alınması, yazılan mesafe; ortak iz
  `snap-acquire.json`) ve 4. adım (katman başına kenet, `.kcad` şema 10; ortak iz `layer-snap.json`) tamam; `HYB-09` bitti. `HYB-10`
  sayısallaştırma kilitleri ([ADR 0166](docs/adr/0166-digitizing-locks.md); sahibin “Todos ile devam”ı, 4 Ekim): 1. adım (çekirdek
  `tools::locks`: kilitli nokta, imlecin kuralı kilitlerle, açı ve sapma doğrultuları, Dik kapat, `<açı`; `lock_cases.py`) ve 2. adım
  (Uzunluk, Açı ve Sapma iki platformda: oturumun kilitleri, değer kartında Tab ve `<açı`, çipler, kılavuzlar, Kilit ▸, `draft.lock.*`;
  ortak iz `locks.json`) ve 3a (Nesneye paralel ve dik: `edge_direction`, kenar bekleyişi, `draft.lock.parallel` ve `.perpendicular`; ortak iz
  `lock-edges.json`), 3b (Dik açı `drafting.rightAngle`, Dik kapat (D); ortak iz `right-angle.json`) ve 4. adım (Referans noktası, Yapım
  kipi; ortak iz `lock-reference.json`) tamam; `HYB-10` bitti. `HYB-11` ikinci koordinat sistemi ve dönüştürücü
  ([ADR 0167](docs/adr/0167-second-crs-and-converter.md)): 1. adım (çekirdek `crs::transform`, geri TM, DMS; PROJ başvurusu
  `crs_transform_cases.py`) ve 2. adım (proje ayarı `secondSrid`, `.kcad` şema 12, `display.geographic`; okunuş `kentos_interaction::second`
  ve `model/secondCrs.ts`; durum çubuğunda iletiyle aynı yeri paylaşan hücre, sistem düğmelerinin sağ tık menüsünde İkinci sistem ▸,
  Proje ayarları'nda alan, Koordinat oku'nun ikinci satırı; ortak iz `second-crs.json`) ve 3. adım (Mesafe ölç ve Alan hesapla'nın ikinci
  sistemin düzlemindeki uzunluğu, alanı ve çevresi: çekirdek `crs::measure`, PROJ başvurusu `crs_measure_cases.py`) ve 4. adım
  (Koordinat dönüştür penceresi, `crs.transform`: tek nokta, Çizimden, liste, panoya ve CSV; okuma ve yazma `crs_convert_cases.py`)
  tamam; `HYB-11` bitti. `HYB-12` özel koordinat sistemi ([ADR 0168](docs/adr/0168-custom-crs-and-datum-transforms.md)):
  1a (çekirdek: projenin datumu, başlangıç enlemli TM, yerel sistem, projenin datum seçimleri; PROJ başvurusu
  `crs_custom_cases.py`) ve 1b (WKT ve PROJ okuma ve yazma, `crs::text`, `crs::wkt`; PROJ başvurusu `crs_text_cases.py`)
  ve 2. adım (NTv2 ızgarası `crs::ntv2`, PROJ'un hgridshift'i birebir; başvuru `ntv2_cases.py`) ve 3a (sözleşme
  `kentos_contracts::crs`, ayarlar `customCrs`, `secondCustomCrs`, `datumTransforms`, `ProjectSettings::sanitized`; `.kcad` şema 13 ve
  bağımsız Python okuyucu ve yazıcısı) ve 3b (çözücü `kentos_project::systems`, `model/projectCrs.ts`, başvuru `project_crs_cases.py`;
  ikinci sistem tanımlar ve seçimlerle; tanımın adı; tanımlı proje sistemli sayılır), 3c (dönüştürücüde projenin tanımları ve seçimleri)
  ve 3d (cihazın ızgara kitaplığı, `grids.rs`, `app/gridLibrary.ts`; Proje ayarları'nda Izgaralar) ve 4a (Proje ayarları'nda Datum
  dönüşümleri, form kuralları `choice_form`) ve 4b (Özel koordinat sistemi penceresi: masaüstü `project/custom_crs.rs` Proje ayarları'nın
  üstünde, web `ui/settings/CustomCrsDialog.ts`; form kuralları `definition_form`, `model/definitionForm.ts`, ortak durumlar
  `definition-form.json`; listelerde tanımın satırı, “Özel sistem…” ve Düzenle) ve 4c (pencerede WKT ve PROJ'dan al, kopyala, Kayıttakini
  seç, Deneme noktası; okuma kuralları `definition_form::read`, `readDefinition`, ortak durumlar `definition-text.json`) ve 4d (Ortak
  noktalardan hesapla: `fit_plane`, `fitPlane`, ortak durumlar `definition-fit.json`; Hesap pencerelerinin tablosuyla) tamam; `HYB-12` bitti.
  Sıradaki `HYB-13` saha verisi ([ADR 0169](docs/adr/0169-field-data.md); sahibin kararları 4 Ekim: dört total station biçimi, GPX ve
  NMEA, toleranslar proje ayarı ve değerlerini sahip verir, k = 0,13, alete okunan biçimlerle): 1. adım (çekirdek indirgeme
  `survey::fieldbook`, CSV karne `formats::field`, iki platformda; başvurular `field_reduce_cases.py`, `field_csv_cases.py`) ve 2. adım
  (Leica GSI-8 ve GSI-16, `formats::field::gsi`; başvuru `field_gsi_cases.py`) ve 3a (Proje ayarları › Ölçme: k ve toleranslar, `.kcad`
  şema 14, `survey_form`; Kutupsal alım k'yı uygular) ve 3b (Karne editörü iki platformda, `calc.fieldbook`; çekirdekte durumlar ve
  tolerans denetimi, `field::sniff`, `readFieldBook`) ve 3c (Kutupsal alım'a aktar: `polar_transfer`, `fieldPolar`, Geri bakış) tamam;
  3. adım bitti. 4a (Poligon hesabı'na aktar: `traverse_transfer`, `fieldTraverse`, Poligon bölümü) ve 4b (toleranslar: kenarın iki
  yönden farkı, poligonun kapanmaları, `.kcad` şema 15; `BookLeg.over`, `survey::traverse::closure`) tamam; 4. adım bitti. 5. adım
  dört parçada, her biri üreticinin belgesinden bağımsız başvurusuyla ve aynı karnenin örneğiyle (`field_samples.py`): 5a Sokkia SDR2x ve
  SDR33 (`formats::field::sdr`, başvuru `field_sdr_cases.py`; Sokkia'nın “Interfacing with the SOKKIA SDR Electronic Field Book”u) ve 5b
  Topcon GTS-7 (`formats::field::gts7`, kesin değerler `field::exact`; başvuru `field_gts7_cases.py`, Topcon Link Reference Manual'ın Ek C'si)
  ve 5d Nikon RAW (`formats::field::nikon`; başvuru `field_nikon_cases.py`, Nivo kılavuzu) ve 5c Trimble JobXML (`formats::field::jobxml`;
  başvuru `field_jobxml_cases.py`, şema 5.3; XML roxmltree ile, sahibin 4 Ekim onayıyla biçim çekirdeğinde de, derinlik koruması
  `formats::xml`) tamam; 5. adım bitti. 6. adım GNSS: 6a okuyucular (`formats::gnss`, `readGnss`; GPX 1.1 ve NMEA 0183, başvurular
  `gnss_gpx_cases.py`, `gnss_nmea_cases.py`) ve 6b GNSS içe aktar (`file.import.gnss`; kurallar `kentos_interaction::gnss`,
  `model/gnssImport.ts`, başvuru `gnss_import_cases.py`; pencere `exchange/gnss_import.rs`, `ui/io/GnssImportDialog.ts`; içe aktarma
  yolundan tek adım, yeni ya da var olan katman) tamam; 6. adım bitti. 7. adım cihaza gönderme: 7a yazıcılar (`formats::field::write`,
  `writeField`; başvuru `field_write_cases.py`) ve 7b Cihaza gönder penceresi (`field.send`; `exchange/field_send.rs`,
  `ui/io/FieldSendDialog.ts`; seçili noktalar, Aplikasyon, Nokta editörü) tamam; Sokkia SDR33 yazımı sahibin bir SDR33 dosyasını ya da
  Sokkia'nın belgesini bekliyor (iş kaydının kodları). `HYB-13` bundan başka bitti. `HYB-14` zemin, elipsoit ve düzlem
  ([ADR 0171](docs/adr/0171-ground-ellipsoid-plane.md); sahibin kararları 4 Ekim: `geographiclib-rs`, projenin ortalama elipsoit
  yüksekliği, “Uzunlukları projeksiyona indir” kapalı başlar): 1. adım (çekirdek `crs::ground`: noktanın ve çizginin ölçeği,
  `geodesy::tm_scale`, yükseklik çarpanı, jeodezik uzunluk, köşe çokgeni ve yayların alanı; başvuru `ground_cases.py`, PROJ ve
  GeographicLib'in C'si, alanlar eşit alanlı izdüşümde; GeographicLib libm'li kopyayla, `scripts/vendor/geographiclib.py`) ve 2. adım
  (Ölçme'de Ortalama elipsoit yüksekliği, `.kcad` şema 16 `groundHeight` ve `reduceToGrid`; yükseklik yazılıyken Mesafe ölç ve Alan
  hesapla'nın elipsoit ve zemin satırları, `kentos_interaction::ground`, `model/groundMeasures.ts`; ortak iz `ground-measures.json`)
  tamam; 3. adım (Hesap pencerelerinde “Uzunlukları projeksiyona indir”) iki parçada: 3a çekirdek (`crs::ground::Grid`,
  `grid_factor`; `survey::polar`, `survey::traverse` isteğe bağlı `grid` ile, iki ve üç geçiş; başvuru `ground_survey_cases.py`)
  ve 3b (Ölçme'de anahtar: yükseklik, sistem ve noktanın tek ölçeğiyle açılır, `survey_grid`, `why_not_grid`, `surveyGrid`,
  `whyNotGrid`; pencerelerin tabloları, özetleri ve raporları iki platformda) tamam; `HYB-14` bitti (5 Ekim). `HYB-15` köşe tablosu
  ([ADR 0172](docs/adr/0172-vertex-table.md)): 1. adım (çekirdek `ops::vertex_table`, başvuru `vertex_table_cases.py`), 2. adım
  (düzenlenen Koordinat listesi iki platformda: web `VertexTable.ts`, `vertexEdit.ts`, masaüstü `vertices/`; `edits.json`) ve 3. adım
  (Topoloji açıkken komşular: `neighbours_in`, `neighboursOf`; takma adlar) tamam; `HYB-15` bitti (5 Ekim). `HYB-16` biçim değiştirme,
  sürdürme ve delikler ([ADR 0173](docs/adr/0173-reshape-continue-holes.md)): 1. adım (`cad.entities.edit`'in beş işlemi; çekirdek
  `ops::holes`, `ops::reshape_by`; başvuru `reshape_cases.py`) ve 2. adım (Delik ekle yol aracının `Hole` biçimi, Deliği sil ve
  Deliği doldur iki platformda: masaüstü `kentos_interaction::holes`, web `holeTools.ts`; deponun `holes_at`'i; ortak iz `holes.json`)
  ve 3. adım (Sürdür: `ops::continuation`, yol aracının `Continue` biçimi, web `continueTool.ts`; ortak iz `continue.json`) ve 4. adım
  (Biçim değiştir: yol aracının `Reshape` biçimi, `kentos_interaction::reshape_by`, web `reshapeTool.ts`; ortak iz `reshape.json`) tamam;
  `HYB-16` bitti (5 Ekim). Sıradaki `HYB-17` çok parçalı çizgi ve çok noktalı nesne
  ([ADR 0174](docs/adr/0174-multi-part-lines-and-points.md)): 1. adım (sözleşme: çoklu çizginin ve noktanın `parts`'ı, `PointPart`;
  `.kcad` şema 17, `FORMATS_VERSION` 28; kodek, sütunlar, bağımsız Python okuyucu ve yazıcısı, `multi-part-lines.kcad`; iki belgede
  miras parçalar düşer) ve 2. adım (hesap: çekirdeğin `Shape`'i ve `area_parts`'ı üç türde, deponun `MARKERS` kaydı ve paket türleri 16, 17,
  düzenlemeler ve `MULTI_PART_REFUSED` retleri, `EntityGeometry`'nin `parts`'ı, kotlar, iki çizici) ve 3. adım (GeoJSON'ın MultiLineString ve
  MultiPoint'i, Shapefile'ın PolyLine ve MultiPoint kaydı tek nesne, `tools/formats/gis.py` de; GeoJSON'a ve DXF'e yazım, `export/cizgiler`,
  `dxf-write/parts`; EWKB'nin ve sunucunun MultiLineString ve MultiPoint'i) ve 4. adım (`partsJoin` ve `partsSplit`'in çizgi ve noktası:
  `kentos_interaction::line_parts`, web `tools/lineParts.ts`; ortak komut durumları) ve 5. adım (Öznitelikler'de Parça ve Nokta sayısı, kartta
  Parça ve Nokta, Koordinat listesinde parçalar ve noktalar; `usage-line-parts`) tamam; `HYB-17` bitti (5 Ekim). `HYB-18` etiketleri yazıya
  çevirme ([ADR 0175](docs/adr/0175-labels-to-text.md)): 1. adım (çekirdek `ops::label_text`: paftanın kuralı, `spot`, `fill_template`; `Store::label_texts`;
  başvuru `label_text_cases.py`; paftayla karşılaştırma `mapLabels.test.ts`; çizimin şablonu iki platformda çekirdeğin kuralıyla) ve 2. adım
  (`cad.entities.create`'in `labels` işlemi, adım “Etiketleri yazıya çevir”) ve 3. adım (araç iki platformda: web `labelsToTextTool.ts`,
  masaüstü `kentos_interaction::labels_to_text`; önizlemede soluk yazılar `drawTextGhost`, `labels::ghost`; standart yazı katmanı türüne göre adıyla aynı
  adımda; ortak iz `labels-to-text.json`) tamam; sıradaki 4. adım (nesneye bağlı yazı, `.kcad` şema 18). 4 Ekim: derleme ve test süreleri
  ([ADR 0170](docs/adr/0170-build-and-test-times.md)). Sahibin sorusu üzerine (4 Ekim) pyproj'la rastgele fark testi eklendi (`crs_sweep.py`; PROJ'un kendi `+towgs84`
  yolu dahil, bilinen tek fark TUREF'e 0,1 mm, ADR 0168 Doğrulama). 3 Ekim:
  pafta düzeni dalı (PR #17, [ADR 0164](docs/adr/0164-sheet-layouts.md)) sahibin sözüyle `main`'e birleşti; birleştirmeden
  sonraki ayrı işler `docs/sheet/integration.md` §6'da. 2 Ekim: web'in klasik arayüzü kaldırıldı, iki platformda yalnız şerit var (sahibin
  kararı, [ADR 0155](docs/adr/0155-web-ribbon-only.md)). PDF, yazdırma ve pafta çıktısı (§16.4) en
  sondadır, zamanını sahip söyleyecek. İşler ADR 0142–0148'deki gibi: önce ADR ve adımları, sonra adım adım iki platformda,
  ortak fixture'larla.
- Web `pnpm e2e` duman testi 2 Ekim'de bütünüyle geçiyor (203 denetim). ADR 0143'ten önce de düşen
  üç testten araç kutusununki araç kutusuyla kalktı (ADR 0155); nokta hesaplayıcının “yan nokta 30/5”
  adımı ve “Layers panel: counts follow add, undo and redo” araç kutusu kalkınca iki koşuda geçti (kök
  nedenleri araştırılmadı). Önceki dilimlerin bozduğu dört denetim aynı gün düzeltildi: Uyarılar
  sekmesinin yeri (Noktalar), gizli “Diğer” çipi, DXF içe aktarmada Blokları patlat, CAD modunun
  Ölçme sekmesi.
- Çalışma düzeni: çekirdek, masaüstü ve web aynı oturumda (29 Eylül akşamından beri alt ajan yok); her
  iş iki platformda kullanılarak resimlenir, commit'lenir; main belli noktalarda (tamamlanan adımlardan sonra) push'lanır (sahibin isteği, 30 Eylül).

## 11. Teknik borç

Borçları TODOS.md'de ilgili görev/kabul koşuluyla izleyin. Mevcut kodda
olmayan kusuru eski nottan hareketle yeniden düzeltmeye çalışmayın:
transaction rollback, gerçek kaydet/aç, backend, cloud, stil/SVG Rust çekirdeği
ve sanallaştırma temelleri zaten vardır. Native desktop, geniş
paylaşım, typed attributes ve otomasyonun kalan kapsamı ayrı gelecek iştir.
Production DB TLS ve bağımsız gerçek ortam doğrulaması gibi eksikleri özellik
varlığına bakarak kapatmayın.

## 12. Dosya haritası ve başvuru

```text
apps/web/              bağımsız TypeScript/DOM uygulaması
apps/api/              kentosd: HTTP/WS, auth ve yönetim CLI
apps/desktop/          masaüstü kabuğu (kentos-cad): Iced + KentOS UI
apps/ui-showcase/      KentOS UI bileşen vitrini
python/                Python SDK'sı: paket `kentos`, tipli katman `kentos.cad` (katalogdan üretilir), testleri
crates/ui/             KentOS UI bileşen kütüphanesi (kentos-ui)
crates/native/         native belge (domain), ürün komutları (application), araç oturumu (interaction), işlem araçları (processing), proje modeli (project), başsız komut sunucusu (headless), Python eklentisi (python), MCP sunucusu (mcp) ve bulut istemcisi (cloud); web'e derlenmez
crates/render/wgpu/    native wgpu çizim hattı (Iced bilmez)
shaders/wgsl/          paylaşılabilir WGSL ve sürümlü düzen sözleşmesi
crates/shared/         contracts, geometry-core, expression, style-core, svg-core, formats, kcad
crates/wasm/           yalnız hesap/codec bağlayıcıları
crates/server/         application ve postgres
fixtures/              sürümlü ortak test verisi
scripts/               ortak build/fixture araçları
tools/                 bağımsız araçlar (KCAD v2 Python okuyucusu)
docs/adr/              karar ve geçiş kanıtları
docs/perf/             tarihli ölçümler ve ortam sınırlamaları
docs/inventory/        web özellik envanteri (üretilir) ve elle notları
docs/baseline/         tarihli başlangıç kayıtları: test, e2e, fixture, vitrin
docs/deps/             bağımlılık kaydı
```

`crates/ui`, `apps/ui-showcase`, `apps/desktop` (ilk kabuk), `crates/native/domain`
(belge, ADR 0020), `crates/native/interaction` (araç oturumu, ADR 0021), `crates/native/application`
(ürün komutları, ADR 0022),
`crates/render/wgpu` ve `shaders/wgsl` (ADR 0019) kuruldu.
Stil sistemi: [docs/STYLE.md](docs/STYLE.md). Processing:
[docs/PROCESSING.md](docs/PROCESSING.md). Tarihli devir notları:
[docs/DEVIR.md](docs/DEVIR.md); eski durum/faz notlarını güncel kod ve
TODOS.md ile karşılaştırın.

## 13. Rust backend ve depolama sınırı

Backend vardır: Axum/Tokio/SQLx ile PostGIS, kimlik/tenant, proje işlemleri ve WS.
Cloud dosya modunda binary revizyonlar nesne deposunda, katalog/izinler DB'de
olabilir; projeyi buluta koymak zorunlu CAD→GIS import'u değildir. Saklama biçimi
(`database`/`file`) proje açılırken seçilir ve değişmez (öbür biçime geçiş, kaynağı değiştirmeyen
`project.convert` ile yeni projedir, ADR 0039); dosya revizyonu yükleme →
doğrulama → `project.file.commit` ile yazılır, depo ile DB arasında ortak işlem
varsayılmaz (ADR 0031). Canlı DB
modunda PostgreSQL/PostGIS otoritedir; projenin `.kcad` görüntüsü tek bir salt okunur
`REPEATABLE READ` işlemde, kilit almadan okunur (`Db::snapshot`, ADR 0033). Özel DB/WAL/MVCC motoru tasarlamayın.
Sunucu transaction'ı istemcinin undo/transaction sorumluluğunun yerine geçmez.

### 13.1 Entegrasyon

Mevcut `project.changes` expected version, idempotency, audit/outbox temellerini
genişletin. UI command registry ile bu protokolü tek kavram sanmayın.
Yerel hızlı edit, dosya dayanıklılığı ve server commit onayı ayrı anlam taşır.
Oluşturulan nesnenin sürümü commit'in veri revizyonudur; silinen kimlik yeniden
oluşturulabilir, eski sürüme dayanan değişiklik çakışmadır (ADR 0026).
Sunucu hatası `ApiError`'dur: sabit kod (`error`), Türkçe ileti, bilinen alanın yolu, çakışmada
revizyon, yeniden deneme bilgisi (`retryable`, `retryAfter`; ARCH-07, ADR 0013). Alanı bilinen
doğrulamada `AppError::invalid_at` kullanın; istemci yeniden denemeyi bu alanlardan karar verir.

## 14. Workspace ve tek hesaplama kaynağı

Saf Rust hesap crate'leri DOM, Iced, SQLx, ağ veya host runtime bilmez.
Web bunları WASM, desktop/server native çağırır. Rust'tan üretilen veri/komut
sözleşmesi ortak olabilir; web uygulama runtime'ı ortaklaştırılmaz.
Sıcak yollarda toplu tipli buffer kullanın; nesne başına JSON/FFI üretmeyin.
Mevcut `jsmath`/`libm`, lint ve deterministic davranış kurallarını koruyun;
`unwrap/expect` ile kullanıcı girdisinde panic üretmeyin.
Yeni hesap çağrısına sınır/rastgele fixture, native/WASM ve bağımsız referans
ekleyin. Taşıma ayrıntıları [ADR 0008](docs/adr/0008-shared-core-boundary.md)'dedir.

## 15. CAD ve PostGIS kaynak otoritesi

Mevcut `crates/server/application/src/cad.rs`: basit geometride `geom`,
analitik CAD'de `cad_definition` kaynak; GIS geometri türevdir. Kaynak türü,
projection sürümü/toleransı ve revision birlikte korunur. Gelecekteki blok,
ölçü, ilişki ve proje metadata'sı da round-trip sözleşmesine dahil edilir.
Doğrudan PostGIS projesinde `.kcad` metadata/bağlantı referansı taşıyabilir;
parola taşımaz. Tam snapshot/export seçimi açık olmalı; aynı veriye iki
bağımsız yazılabilir otorite kurmayın. Ayrıntı TODOS.md §10–11, ADR 0006.

## 16. Tenant, proje yetkisi ve paylaşım

Mevcut tenant/rol altyapısını koruyup proje bazlı yetkiyle genişletin.
Proje erişimi proje düzeyindedir (ADR 0015): rolü `kentos.project_role`, izinleri
`application/src/access.rs` verir. Projeye dokunan her kullanım durumu, HTTP yolu, ürün komutu
ve WS aboneliği `access::project` ile başlar; yazanlar proje kilidi altında yeniden sorar.
Erişilemeyen proje var olmayan gibi 404'tür. Yaşam döngüsü komutları katalogdadır (ADR 0028):
arşiv `project.edit`, çöp kutusu/geri yükleme/kalıcı silme `project.delete` ister; kalıcı
silme yalnız çöpten ve projenin adıyla onaylıdır, denetim kaydı kalır; çöpteki proje
`KENTOS_TRASH_RETENTION_DAYS` (varsayılan 30) gün sonra kalıcı silinir. Projeye bağlı satırlar yalnız `app.project_id`
kapsamında görünür.
Kimlik/izin server'da doğrulanır; tenant üyeliği her projeyi görme hakkı değildir.
Paylaşılacak kişi yalnız arayanın görebildiği kişiler arasında aranır (projenin kurumu;
kişisel projede arayanın kurumları); hesabın varlığını sızdıran genel e-posta/giriş adı
araması yoktur (ADR 0024).
API, WS, sorgu, dosya/asset, job, Python ve AI aynı erişim modelini kullanır.
Kurum dışına yol bağlantıyla davettir (ADR 0035): yalnız davetteki e-postanın doğrulanmış hesabı kabul eder;
kurum dışından biri yalnız o projeyi gören misafir olur, kurum misafiri kapatabilir (`tenant.allow_guests`).
Kişisel ve kurumsal sahiplik, davet, paylaşım ve izin iptali TODOS.md §12'dedir.
RLS/pool context ve runtime/admin rolleri test edilir. İndirilmiş kopyanın
geri alınabileceğini veya görüntüleme izninin DRM sağladığını iddia etmeyin.

## 17. Harita yayını: olası gelecek kapsamı

Martin entegrasyonu veya Martin eşdeğeri şu an görev değildir; cloud
saklama/açma/paylaşım için MVT/TileJSON servisi zorunlu tutulmaz.
Mevcut outbox/event güncelliği sync için korunur; gelecekte yayın için de
kullanılabilir. Ayrı yayın kararı gerekirse TODOS.md §12.5 izlenir.

### 17.1 Gelecek değerlendirmesi

Somut ihtiyaç olmadan tile crate'i, compatibility endpoint'i veya yayın fazı
başlatmayın. Olası yayın türevleri `.kcad` ya da CAD kaynak modelinin yerine geçmez.

## 18. Command, Python/AI ve ağır işler

Ürün komutları tipli/sürümlü, yetkili, iptal/önizleme/sonuç sözleşmeli olmalı.
Python ve AI bunları sarar; repository/SQL/DOM'dan iş kurallarını atlamaz.
Web processing worker'ı mevcut; kalıcı server worker ayrı teslimdir.
Server job tasarımında lease/heartbeat/fencing, idempotency, staged sonuç,
revision/yetkiyi commit anında doğrulama ve kaynak kotaları gerekir.
Uzun hesap API event loop/UI thread'de çalışmaz; socket kapanması işi yok etmez.

## 19. Teslim ve kabul

Faz sırası TODOS.md §22'dedir. Kütüphane eklemek veya ekran göstermek tek başına
teslim değildir. Proje bulutu önceliği eski yayın fazlarının yerini alır.

### 19.0 Kabul kapıları

İlgili correctness, platform uyumu, yetki, veri kaybı/restore ve performans
testlerinin kanıtını verin. Test atlamayı başarı, golden üretmeyi bağımsız
doğrulama, çalışan bir örneği bütün ürünün tamamlanması saymayın.

## 20. Lazy load ve modül yaşam döngüsü

### 20.1 Mevcut sınırlar

Stil/SVG düzenleyicisi, format pencereleri, ribbon ve ağır modüllerin mevcut
JS/CSS/WASM yükleme sınırlarını koruyun; başlangıca gereksiz import eklemeyin.

### 20.2 Yükleme

Yükleme async, hatası görünür ve yeniden denenebilir olsun. İptal/geç kalan
sonuç açık belgenin veya yeni oturumun üstüne uygulanmasın.

### 20.3 Yaşam döngüsü

Aç/kapat, tema/backend değişimi ve yeniden login'de listener, worker,
GPU kaynağı ve task temizlenir; mount/unmount tekrarları sızıntı yaratmasın.

### 20.4 Ölçüm

Startup ve modül açılışını üretim bundle'ında ölçün. WASM boyutu izlenir,
ancak kaldırılmış keyfi başlangıç boyut sınırını yeniden eklemeyin.
Python/3D/ileri analiz sıradan 2D açılışa zorunlu yük olmamalı.

## 21. Bağlantı, proje açılışı ve autosave

### 21.1 Sürekli bağlantı

Mevcut WS heartbeat/reconnect/replay/resync akışını koruyun (web). Masaüstü olayları
uzun sorguyla izler (`GET …/events?wait=`, ADR 0044). Auth, offline,
uyumsuz protokol ve server hatası ayrı durumlardır; socket açık diye veri güncel değildir.

### 21.2 Asenkron açılış

Yerel/cloud açılış iptal edilebilir ve generation/revision kontrollüdür.
Geç dönen yükleme kullanıcının yeni belgesini ezmez; doğrulanmamış aday belge
mevcut çalışmanın yerine geçirilmez. Büyük veride bounded bellek kullanın.

### 21.3 Otomatik kayıt

Masaüstü internetsiz çalışır, web çevrimiçi kalır (sahibin kararı, ADR 0043). Masaüstünde bulut
projesi yerel kopyadan (sunucunun son bilinen hâli) açılır, iş cihaz taslağında bekler, bağlantı
dönünce kendiliğinden eşitlenir; kopyaya sunucunun hâlindeki değişiklik taslaktan önce yazılır.
Çizim komutları sunucuda çalışmaz.

Cloud taslağı kalıcı kuyruğa alınmadan güvende denmez. Expected revision,
idempotency ve conflict davranışı korunur; başkasının güncel işi sessizce ezilmez.
Komut ACK, binary snapshot finalize ve cihazda kayıt ayrı durumlardır.
Yavaş kaydetme sırasında değişen belge eski revision adına temizlenmez.
Dosya projesi kendiliğinden kaydedilmez; `FileCommitted`'dan önce kaydedildi denmez;
başkasının revizyonu kendiliğinden yüklenmez (ADR 0038).

### 21.4 Doğrulama

Offline/reconnect, tekrar istek, ACK kaybı, mixed-client, quota, yetki iptali,
upload/finalize crash ve restore senaryolarını native/web/server sınırında sınayın.

## 22. Gelecekte 3D şehir, yol ve mimari

TODOS.md §17–19 ileri kapsamı tutar. Semantik parsel/yapı/yol/terrain kaynağı
ile render mesh/LOD/3D Tiles ürününü ayırın. Web 3D renderer bağımsız kalır;
paylaşılabilir hesap/WGSL tekrar kullanılır. Bu hedefler bugünkü cloud'a
Martin rolü veya bütün desktop uygulamasını WASM yapma şartı getirmez.

## 23. Sayısal ve kadastral doğruluk

Kaynak sınır, koordinat, alan ve hak doğruluğu performanstan önce gelir.
f64/decimal veya ortak Rust tek başına sıfır hata/kadastral uygunluk kanıtı değildir.

### 23.1 Kayıpsız sayı yolu

Kaynağın hassasiyet, birim, CRS/datum ve gerekiyorsa özgün ondalık değerlerini
koruyun. Kesin decimal/rasyonel değerler JS Number üzerinden geçmez;
decimal string ve pay/payda sözleşmesi kullanılır. `NUMERIC` ölçeğinin örtük
yuvarlamasına güvenmeyin; taşma/NaN/Infinity kontrollü hatadır.
Geometrik alan, kayıtlı alan ve gösterim ayrı anlamlardır.

### 23.2 Yuvarlama

`numeric_policy_id/version`, birim, ölçek, eşitlik yönü, ara adımlar ve
dağıtım artığı kuralı açık/sürümlü olmalı. Gösterim metni hesap girdisi değildir.
`toFixed`, `Math.round`, SQL round veya kütüphane varsayılanını iş kuralı yapmayın.
Hisse/alan toplamı korunur; artığı rastgele son parsele vermeyin.
Onaysız veya belirsiz kuralda nihai kayıt tamamlanmaz.

### 23.3 Geometrik kararlar

Robust/adaptive gerektiğinde exact predicates kullanın; kesin predicate
üretilen koordinatın kesinliğini garanti etmez. Seçim piksel toleransı,
snap/topoloji toleransı, hesap hata sınırı ve kaynak belirsizliği ayrıdır.
Gizli snap/grid/repair/simplify veya büyük epsilon ile hata örtmeyin.
Alan/uzunluk kaynaktan ve açık düzlemsel/elipsoidal/3D yöntemle hesaplanır;
render tessellation'ı nihai ölçü değildir. CRS grid/epoch/sürümü kaydedilir;
eksik grid'de sessiz düşük doğruluk fallback'i yapılmaz.
Eşiği kesen hata aralığında hassasiyeti artırın; hâlâ belirsizse `needs_review`
veya atomik ret, dayanağı kayıtlı çözüm gerekir.

### 23.4 Bağımsız doğrulama

Native/WASM eşitliği yanında bağımsız yüksek hassasiyetli referans gerekir.
Sınır, yarım değer, negatif, yakın paralel/teğet, büyük koordinat, delik/eğri,
ortak sınır ve alan/hisse korunumu fixture'larını koruyun.
Nihai decimal, yuvarlama yönü ve topolojik karar platformlar arasında aynı
olmalıdır; farkta server sonucunu sessiz seçmeyin. Algoritma/build, kaynak
revision, numeric policy ve dönüşüm provenance'ı sonucu yeniden üretilebilir kılsın.
Mevcut toleransı büyütmek veya beklenen fixture'ı hataya göre yenilemek yasaktır.
Ayrıntı: ADR 0004/0008 ve TODOS.md §3/§20.

## 24. Ürün kapsamının izlenmesi

### 24.1 Veri/provider

Typed schema, domain/ilişki/form, provider ve PostGIS kapsamı TODOS.md §3/§11/§16'dadır.
Mevcut string özniteliği tipli model tamamlandı diye sunmayın.

### 24.2 Workflow ve otomasyon

Stil, form, processing, pafta, Python ve AI aynı veri/komut sözleşmesine
bağlanır; ayrıntıları TODOS.md §13–16'dan izleyin.

### 24.3 Paylaşım

Proje paylaşımı ile harita yayını ayrı özelliklerdir. Şimdiki hedef kişisel/
kurumsal projelerin yetkili saklama/erişim/senkronizasyonudur; TODOS.md §12.

### 24.4 Operasyon

TLS, sır yönetimi, kotalar, backup/PITR + object restore, audit ve dağıtım
TODOS.md §21'de izlenir. Restore denemesi olmadan backup çalışıyor denmez.
Yerel geliştirme kolaylıklarını production güvenlik ayarı gibi kullanmayın.
