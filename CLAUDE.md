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

- **Web katmanına ara (sahibin kesin kararı, 12 Ekim 2026; [ADR 0228](docs/adr/0228-web-pause-desktop-and-cloud-first.md)).**
  - Web'in son hâli **`1aad00d0`**'dır. Bundan sonra maddeler yalnız masaüstünde ve gerektiğinde bulutta (`kentosd`), Python'da ve
    MCP'de yapılır.
  - `apps/web/`'e yeni özellik yazılmaz. Web'in denetimleri (typecheck, Vitest, build, e2e, `e2e:interaction`, resimler, `pnpm inventory`)
    madde takımında koşmaz; `docs/inventory/web.*` `1aad00d0`'daki hâliyle donar.
  - Web'e masaüstü ve bulut tamamen bitince, sahibin sözüyle dönülür. Geri dönüşün planı TODOS.md §0.2 “Web'e ertelenenler”dir: yalnız
    masaüstünde biten her madde oraya satırını yazar.
  - Aynen sürenler:
    - ortak çekirdek (`crates/shared/*`) ve sözleşmeler (TS türleri ve katalog üretilmeye devam eder);
    - bağımsız Python başvuruları;
    - ortak fixture'lar ve etkileşim izleri (masaüstünde oynatılır);
    - WASM bağlayıcıları yalnız derlenir, yeni işleve bağlayıcı yazılmaz.
  - Yeni komutların arayüz kaydı (ad, ikon, açıklama, kısayol, şeritteki yer) web'in envanteri yerine `catalog/v1/additions.json`'dadır
    (ilk tüketicisi `GIS-40`).
  - Kod, ileride mobil (önce görüntüleme) ve web eklenecekmiş gibi modüler kalır:
    - hesap `crates/shared`'te;
    - arayüzden bağımsız uygulama katmanı `crates/native`'te (Iced, wgpu ve pencere bilmez; `pnpm arch:deps` zorlar);
    - çizim hattı `crates/render/wgpu`'da;
    - Iced yalnız `crates/ui`'da ve kabuklarda. Masaüstü kabuğu incedir: sunum olmayan iş `crates/native`'e ya da `crates/shared`'e yazılır.
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
  çok satırlı yazı: kutu genişliği ve sözcük sarma, satır aralığı, kalın, eğik, altı çizili, üst ve alt simge, renkli harfler, Çok
  satırlı yazı aracı (iki köşe, Satır aralığı), düzenleyici (çizimde canlı önizleme, Renk ▾, Simge ▾), çift tıkla düzenleme,
  Öznitelikler'in Kutu genişliği ve Satır aralığı; DXF'in MTEXT'i biçimleriyle tek yazı olarak gelir ve gider (ADR 0182, `.kcad` şema 20);
  yazı ve ölçü stilleri: projenin adlı stilleri (`.kcad` şema 21), nesne görünüşünü kendi alanlarında taşır (yazının yüzü: stil, yazı tipi,
  kalın, eğik, yatıklık; ölçünün görünüşü: dolu ok, açık ok, nokta, yok ya da çentik, boylar, değerin yeri, basamak, birim, önek, sonek,
  yazı tipi), stil değişince eski değerdekiler tek adımda izler; CAD projesinde Yazı stilleri ve Ölçü stilleri penceresi, araçların Stil (S)
  seçeneği (Çok satırlı yazı'nın Satır aralığı R), Öznitelikler'in stil satırları; DXF STYLE ve DIMSTYLE iki yönde, içe aktarılan stiller
  projeye adıyla eklenir, Standard Standart'tır (ADR 0183);
  tablo: hücreler, birleşik alanlar, sütun hizası, başlık satırı, çizgiler ve kalın çerçeve (`.kcad` şema 22); CAD projesinde Tablo ekle
  (boş, Excel .xlsx, ODS ya da CSV/TXT dosyasından, koordinat, alan ve öznitelik çizelgesi; önizleme, imleçten yerleştirme), Tabloyu düzenle
  (elektronik tablo gibi; çift tıkla), Tabloyu güncelle (kaynağına bağlı: çizelgenin nesneleri ya da yeniden seçilen dosya), Öznitelikler'in
  satırları; DXF'e adsız blok olarak gider, KENTOS verisiyle tablo olarak gelir (ADR 0184);
  koordinat yazımı: CAD projesinde Koordinat yaz (tıklanan, yazılan ya da `#ad` noktaya; yerindeki noktanın adı ve kotu) ve Köşelere koordinat
  yaz (seçili nokta, çizgi ve alanların bütün köşelerine, tek adımda); eğik kol ve yatay çizgi, üstünde doğu, altında kuzey ya da kolsuz
  satırlar; Şablon (`{Y}`, `{X}`, `{Z}`, `{ad}`), Basamak, Yön (Otomatik dışa), Yükseklik, Stil; Çizelge açıkken aynı nesnelerin koordinat
  çizelgesi imleçten yerleşir (ADR 0185);
  tarama ekleri: desen türü (adı, ölçeği ve çizgi aileleri taramanın içinde; `.kcad` şema 23), kitaplıkta ANSI, ISO ve genel 28 desen,
  kesikler, noktalar ve kayma iki platformun çizim hatlarında, degrade (doğrusal, silindir, küre; ters, ikinci renk); Tarama'nın Desen
  (menüde her desenin geniş örneği, adı yazılarak), Ölçek, Açı, İkinci renk, Ters, İlişkili ve Yazılar seçenekleri; Çoklu tara; ilişkili tarama kapalı
  nesnesini, adalarını ve boş bıraktığı yazıları iki belgede kayıttan önce izler, Öznitelikler'de İlişkiyi kopar; DXF'in HATCH'i desen
  satırları ve degradesiyle iki yönde (ADR 0186);
  seçim ekleri: Sıradakini seç (tıklamanın adayları çekirdeğin sırasıyla, “1/3 ▾” çipi ve listesi, Shift+Boşluk), Çokgenle seç
  (İçindekiler, Kesişenler, Dışındakiler), Önceki seçim, Benzerini seç (Tür, Katman, Renk, Sembol), Seçim süzgeci (durum çubuğunda Süzgeç,
  on yedi tür; tıklama, kutu, seçim araçları ve seçme adımları süzülür, dışarıda kalan söylenir; hücrenin menüsü ve Giriş › Seçim süzgeci ▾
  aynı işaret listesi: kısa adlar ve ikonlar, menü açık kalır, Bütün türler ve Hiçbir tür; Kenet türleri ve Çakışma'nın katmanları da açık kalır) (ADR 0187);
  nokta hesaplayıcı ekleri: Obje üzerinde nokta (nesnenin yolunda yakın uçtan uzaklık ve dik sapma; çizgi, yay, daire, elips, eğri, alanın
  dış sınırı), Km ve sapma (güzergâhın km'si, Başlangıç (B)), Nokta adından, Mesafe ve eğim (eğik mesafe ve yüzde eğim), Açıortay;
  `#ad` hesaplayıcının referansını da verir, takma adlar Türkçe işaretsiz de (`eğim` EGIM) (ADR 0188);
  Km yaz: güzergâhın aralıkla km'si (0+020 …) ve uçları, istasyonlarda dik işaret, çizginin yanında okunur km yazısı, isteğe bağlı
  enkesit ve sapmalı nokta (`Km` özniteliğiyle), tek adımda; CAD projesinde Açıklama › Km (ADR 0189);
  orta hat: iki kenarın (yol kenarları, dere kıyıları) ekseni, eşleşen kenarlarda kenar kenar ve yaylarıyla kesin, öbürlerinde uzunluk
  payıyla Adım'da bir örneklenerek; Zincir uç uca çizgileri tek kenar sayar; tek adımda çoklu çizgi, Çizim ▾'da (ADR 0190);
  paralel kaydır: alanın ya da çoklu çizginin düz kenarı kendine paralel kayar, komşuları uzar ya da kısalır; yazılan uzaklıkla, imleçle
  ya da Alan (A) ile hedef alana (alan projenin biriminde, uzaklık ikinci dereceden denklemin sıfıra yakın kökü); köşeler kotlarını
  korur, tek adımda, Değiştir ▾'da Ötele'nin yanında (ADR 0191);
  resim nesnesi: PNG ya da JPEG, sol alt köşesi, genişliği, yüksekliği, dönüşü ve aynasıyla; gömülü (projenin kitaplığında, kimliği
  içeriğinden) ya da masaüstünde bağlı (dosya yolu); kırpma sınırı resmin kendi kesirleriyle, saydamlık; Resim ekle (ikinci nokta ya da
  yazılan genişlik, Bağlı) ve Resmi kırp (Dikdörtgen, Çokgen, Kaldır), CAD'de Ekle › Resim, CBS'de Veri › Resim; yalnız kenarından seçilir;
  Öznitelikler'de Kaynak ve Göm; resmin kendi dokusu mip katmanlarıyla üç çizim hattında (`.kcad` şema 24, ADR 0192);
  çizimler arası alışveriş: Seçilenleri dosyaya kaydet (seçilenler kullandıkları katmanlar, bloklar, kitaplık ve ayarlarla yeni `.kcad`),
  Başka çizimden al (katmanlar yollarıyla, bloklar, yazı ve ölçü stilleri, kitaplık, katman durumları, proje ayarları; Atla ya da
  Değiştir; katmanlar ve bloklar tek adımda, ötekiler ayar), Dosyadan blok ekle (çizim dosyanın adıyla tek blok, sol alt köşesinden,
  ardından Blok ekle) (ADR 0193);
  hizala ve dağıt: seçilenler kutularının kenarı ya da ortasıyla başvuru nesnesine, noktaya ya da seçimin kutusuna (Sola, Ortala,
  Sağa, Üste, Ortaya, Alta), ya da eşit aralıkla (Yatay ve Dikey dağıt); her nesne kendi kaymasıyla, tek adımda;
  `cad.entities.transform`'un `arrange` türü (ADR 0194);
  görünüm kipleri: Renk kipi (Renkli, Tek renk, Gri), Dolgular, Alan sınırları ve Saydamlık açılıp kapanır, seçimin ve üzerine
  gelmenin vurgu rengi ve kalınlığı; kullanıcının tercihleri, çizim değişmez (ADR 0195);
  eğri boyunca yazı: yazının eğrisi kendi çerçevesinde (`.kcad` şema 25), harfler eğri boyunca; Eğri boyunca yazı ▾ (Eğri boyunca
  yazı, Eğriye oturt, Doğrultuya döndür, Düzleştir), Okunur yap eğriyi çevirir; DXF'e harflerinin bloğu ve KENTOS verisiyle (ADR 0196);
  çizim ekleri: İki daireye teğet (tıklamalara en yakın dış ya da iç teğet), Dördüncü köşe (nokta ya da paralelkenar alanı), Menzil
  halkaları (Aralık, Sayı, Işın) (ADR 0197);
  plan yolu çizimi: yol alan, türü ve genişliği öznitelik; Plan yolu (yol, yaya ve bisiklet yolu; kaldırımla taşıt yolu, refüj, eksen),
  Kavşak temizle (türlere göre birleştirme, ada ve kaldırım köşelerinin yuvarlanması), Refüj kapat (ADR 0198);
  katman alanları, öznitelik tablosu ve veri kaynakları: katmanın alanları (metin, tam sayı, ondalık sayı, tarih, evet/hayır;
  uzunluk, basamak, aralık, değer listesi, zorunlu, varsayılan; tek biçim `kentos_contracts::fields`; `.kcad` şema 26), `cad.entities.set`
  ve `.create` alanların kuralıyla yazar, Alanlar penceresi (Verilerden al), alt panelin Tablo sekmesi (Katman, Ara, Göster, İfade
  süzgeci, türe göre sıralama, hücrede düzenleme, uymayan değer uyarı zeminiyle), Öznitelikler'de alanlar türlerine göre; Shapefile
  ve GeoJSON alanları taşır; sağ dokta Kaynaklar (klasörler ve dosyaları içe aktarma penceresine, KentOS projelerinin katmanları
  nesneleriyle tek adımda) (ADR 0199);
  mekânsal ve öznitelik sorgusu: İşlemler'de Konuma göre seç (Kesişen, İçeren, İçinde kalan, Ayrık, Uzaklıkta, Merkezi içinde),
  İçindekinden bilgi al (Sayı, Toplam, Ortalama, En az, En çok, İlk değer), Çevreleyenden bilgi al, Özet istatistik (gruplu tablo,
  Panoya kopyala, CSV) ve Anahtarla birleştir (katmandan ya da CSV, TXT, XLSX dosyasından); sayıların kuralı `kentos.statistics/1`;
  İşlemler'in dosya parametresi, tablo çıktısı, aracın reddi ve çalıştırıcının katman alanları denetimi (ADR 0200);
  geometri işlemleri: İşlemler'in Geometri kategorisinde Tampon (eksi uzaklıkla içe, tek yanlı, halkalar, Birleştir, Uzaklık alanı;
  yaylar kesin), Kırp, Gruplayarak birleştir (kesin toplamlar), Kesişim, Fark, Simetrik fark ve Birleşim (Önek, Alan oranıyla paylaştır),
  Geçerliliği denetle (tablo ve seçim) ve Onar (rapor), Sadeleştir (rapor), Koordinat sistemine dönüştür (kayıttaki sistemden projeninkine,
  datum seçimleriyle); sonuç yeni katmana, kayıplar söylenir; çekirdek `ops::geoprocess` (ADR 0201);
  topoloji kuralları: projenin toleransı, kuralları ve istisnaları (`.kcad` şema 27); on üç kural, katman içi dokuzu (Çakışmamalı,
  Boşluk olmamalı, İnce alan, Yinelenmemeli, Sarkan uç, Kısa kenar, Küçük açı, Geçerli olmalı, Ortak sınırda köşe eksik olmamalı) ve
  katmanlar arası dördü (… ile çakışmamalı, … içinde kalmalı, Sınırı … sınırlarında olmalı, … çizgilerinin ucunda olmalı); Topolojiyi
  denetle alt panelin Topoloji sekmesinde (Kural, Açık / İstisna / Hepsi; satır bulguya gider ve onu çizimde gösterir), Düzelt ▾ tek adımda
  (`topologyFix`) ve denetim yinelenir, İstisna yap projede; Topoloji kuralları penceresi; çekirdek `ops::topology_rules` (ADR 0202);
  ağ dengelemesi: Yatay ağ dengelemesi (doğrultu serileri ve yatay kenarlar, sabit ya da ağırlıklı bilinen noktalar, adı yazılan
  çizimden; yaklaşık koordinatlar çizimden ve turlarla; en küçük kareler, hata elipsleri, artıklar, katkı, uyuşumsuz ölçü testi, m₀ ve model
  testi) ve Kot ağı dengelemesi (geometrik nivelman ya da trigonometrik); önsel doğruluklar Proje ayarları › Ölçme'de (`.kcad` şema 28);
  Çizime yaz aynı adlı noktaları taşır ya da kotlarını yazar, yenileri ekler, tek adımda; Karne editöründen aktarma; çekirdek
  `survey::adjust` (ADR 0203);
  raster katmanları: GeoTIFF ve TIFF (klasik ve BigTIFF; LZW, Deflate, PackBits, JPEG; 8–64 bit, palet, iç önizlemeler), dünya dosyalı PNG
  ve JPEG, kendi okuyucumuzla dosyanın gereken parçalarından; raster nesnesi (`.kcad` şema 29: dönüşüm, boy, bantlar, bağlı ya da gömülü
  kaynak, sistem, görünüş, saydamlık); 256'lık karolar, iç önizlemeler ya da bir kez hazırlanan önizleme piramidi (cihazın önbelleği,
  Durdur'lu panel); görünüş: renkli, gri, paletli, renk rampası, gölgeli kabartma (Horn), rampa ve gölge, gerdirme, nodata, örnekleme;
  çizim hattında raster atlası (stilli çizimin 6. sürümü); Raster ekle (sistemin kuralı, oturtulmamış yer), Raster stili (canlı önizleme),
  Raster oturt (Helmert, afin, projektif, polinom 2 ve 3, ince plaka; artıklar ve m0; afin dönüşümü değiştirir, ötekiler GeoTIFF'e yeniden
  örnekler), Koordinat oku'da rasterin değerleri; CBS'de Veri › Raster, CAD'de Ekle › Raster (ADR 0204);
  nokta bulutu, şimdilik yalnız masaüstünde (sahibin kararı, 8 Ekim; web'in şeridinde aynı düğmeler “Yalnız masaüstünde; web için
  hazırlanıyor” notuyla bekler): LAS 1.0–1.4, LAZ (laz-rs), COPC ve metin bulutları (XYZ, PTS, TXT, CSV); bağlı dosya, HTTP adresi (parça
  parça) ya da gömülü (32 MB'a kadar), birden çok dosya tek sanal bulut, .vpc okunur ve yazılır; `pointcloud` nesnesi (`.kcad` şema 31;
  rasterin adresi `url` de); COPC olmayan dosyanın dizini cihazın önbelleğinde bir kez (Durdur'lu panel); çizimde kat kat, karede en çok
  4 milyon nokta, düğümler iş parçacıklarında yalnız görünüşün alanlarıyla çözülür; görünüş: renkler, sınıflar, yükseklik ve yoğunluk
  rampası, dönüşler, tek renk, gizlenen sınıflar, piksel ya da metre boy, kare ya da yuvarlak, saydamlık; Nokta bulutu ekle, Nokta bulutu
  stili (canlı önizleme), XYZ sor, Sanal bulut olarak kaydet; İşlemler'in Nokta bulutu kategorisi: alan sorgusu, Seyrelt, Zemin süzgeci
  (SMRF), Yüksekliğe göre sınıfla, Bulutu kırp, Bulutları birleştir, Karola, Rasterleştir (GeoTIFF), Sınır çıkar; sonuçlar LAS, LAZ ya da
  COPC; Raster ekle'de Adres (GeoTIFF ve COG, HTTP aralıklarıyla) (ADR 0207);
  harita servisleri ve altlıklar: servis katmanı (XYZ ve TMS, WMS 1.1.1 ve 1.3.0, WMTS, OGC API Tiles, ArcGIS REST, Google Map Tiles API,
  vektör karolar MapLibre stiliyle), veri katmanı (WFS, OGC API Features, ArcGIS katmanı, GeoJSON adresi; Yenile) ve projenin bağlantıları
  (adreste parametre, başlık, kullanıcı adı ve parola, belirteç, ArcGIS belirteci, OAuth 2, Google anahtarı; gizli değerler cihazda), `.kcad`
  şema 32; karolar projenin sistemine ağla, vektör karonun noktaları da aynı ağla; Altlık ▾ (hazır altlıklar), Harita servisi, Servisten veri al,
  Servis bilgisi, Bağlantılar (CBS'de Harita, CAD'de Ekle); katmanın menüsü, rozeti ve Öznitelikler'deki bölümü; atıf şeridi ve kartı;
  `cad.layers.service`; web vekili `kentosd`'nin `/v1/proxy`'si; Python `kentos.services` (ADR 0208);
  ağ analizi: projenin ağları (kenar ve düğüm katmanları süzgeçleriyle, düğümün rolü Bağlantı, Kaynak ya da Vana ve kapalılığı, Uçlarda ya
  da Köşelerde bağlanma ve tolerans, yön, Uzunluk ve hızdan ya da alandan maliyetler, kapalı kenarlar; `.kcad` şema 33), ağ her analizde
  katmanlardan kurulan graf; Ağlar penceresi ve Denetle; En kısa yol (Sırayı iyileştir), Hizmet alanı (aralıkların çizgileri ve alanları,
  Birleşik ya da Ayrı, Disk ya da Halka), Şebeke izleme (Bağlı, Akış yukarı, Akış aşağı, Yalıtım ve beslemesiz kalanlar); İşlemler'in Ağ
  analizi kategorisi (En yakın tesis, Maliyet matrisi, Hizmet alanları); kurma ve aramalar web'de ağın işçisinde, masaüstünde ağın iş
  parçacığında; `cad.network.define`; Python `kentos.network`; CBS'de Analiz › Ağ analizi (ADR 0209);
  zaman ve senaryolar: katmanın zaman ayarı (başlangıç, bitiş, kimlik alanı, Birikimli; nesnenin zamanı özniteliklerinden), senaryo
  grubu ve ana katmanın yerine geçen katman (`.kcad` şema 34); geometri deposunda zamanlar ve pencere (tıklama, kenet, etiket ve Genel
  bakış süzülür); çizimin altında Zaman sürgüsü (Başa … Sona, adım ve birim, Anlık ve Aralık, Hız, Döngü; oynatma çizimi bekler), Zaman
  ayarları, Yeni sürüm oluştur ve Sona erdir, Veri karşılaştır'da tarafın tarihi, Zamanı karşılaştır, Geçmiş'te revizyonla Karşılaştır,
  Senaryo oluştur, Senaryoyu göster, Mevcut durum, Senaryoyu karşılaştır, Senaryoyu uygula; `cad.layers.time`, `cad.scenarios.edit`;
  Python `kentos.temporal`; CBS'de Harita › Zaman ve Senaryo (ADR 0210);
  katman süzgeci: katmanın koşulu (İfadeyle seç'in dili) ve ya da nesne listesi (`.kcad` şema 35); süzgeçten geçmeyen nesneler çizilmez,
  seçilmez, kenetlenmez, Öznitelik tablosunda, İşlemler'in girdilerinde ve Veride ara'da yoktur, ağaçta huni ve “geçen / bütün”; dosya
  alışverişi ve komutlar bütün nesnelerle çalışır; Katman süzgeci penceresi (ε, Seçimden al, önizleme), Seçimden süzgeç, Süzgeci kaldır,
  süzgeçten geçmeyen yeni nesne söylenir; `cad.layers.filter`; CBS'de Veri › Süzgeç, iki platformda Katmanlar'ın sağ tıkı (ADR 0211);
  etiket motoru: çizimin, paftanın ve Etiketleri yazıya çevir'in tek motoru (QGIS'in PAL'ı ve Maplex gibi: türe göre maliyetli adaylar,
  yönlü kutularla çakışma, engeller ve öncelik, açgözlü yerleşim ve bir adımlık iyileştirme, adaylar gerektikçe üretilir); katmanın
  etiketlemesi Tek etiket, Kurallı (sıralı sınıflar: ad, koşul, kendi stili) ya da Yok, engel ağırlığı ve türü (`.kcad` şema 36);
  metin şablonla ya da ifadeyle; noktada çevresinde ya da üstünde, çizgide paralel, kıvrık, yatay ya da yokuş yukarı eş yükselti,
  alanda yatay, eğik, çevre, sınır, parsel ya da köşe; yineleme, uzaklık, yalnız içine, sığmazsa dışarıda çağrı çizgisiyle; hale, zemin,
  gölge; yığma, kısaltma sözlüğü, küçültme; öncelik, çakışma, yinelenenler; nesnenin etiket iğneleri (`labelPins`: elle taşınan,
  döndürülen, gizlenen etiket); Etiketler penceresi (beş sekme, sınıflar, Engel, ε) ve Katmanlar'ın sağ tıkı, Etiketi taşı, döndür,
  sabitle (Çöz) ve gizle (Göster), Sabit etiketleri vurgula, Yerleşmeyen etiketleri göster; CBS'de Harita › Etiket;
  `cad.layers.labels` ve `cad.labels.pin`; paftanın PDF'ine zeminler ve çağrı çizgileri vektör olarak (ADR 0212);
  ek işleyiciler: Sürekli renk (rampadan ana renk), Orantılı sembol (Alan, Yarıçap, Flannery), İki değişkenli renk (n × n ızgara), Nokta
  yoğunluğu (alanın içinde tohumlu noktalar, katmanda en çok 1 000 000), Grafik (pasta, çubuk, yığılmış çubuk), Isı haritası (QGIS'in
  dördüncü dereceden çekirdeği, dinamik ya da sabit en büyük, görünümün resmi), Kümeleme ve Yayma (QGIS'in nokta uzaklığı kuralı; halka,
  iç içe halkalar, ızgara; tek noktalar bir iç işleyiciyle), Ters alan (alanların dışı; çift-tek ya da sarım); görünüme bağlı olanlar
  yakınlaştırma durunca ya da kutudan çıkınca yeniden kurulur; Katman stili'nde gruplu, ikonlu tür listesi ve formlar, lejant satırları,
  pafta; `cad.layers.renderer` (ADR 0213);
  ifade dili ekleri: tarih ve saat (yap, oku, parçalar, ekle, fark, biçimle, şimdi), `@` değerleri (projenin değişkenleri ve yerleşikler:
  proje adı, sistem, EPSG, ölçek, tarih, şimdi, kullanıcı, katman; `.kcad` şema 37), düzenli ifade (`regex-lite`), dizi ve eşleme (JSON),
  aynı ve başka katman üstünden toplama, Konuma göre seç'in ilişkileri, en yakının değeri, örtüşen alan ve uzunluk, başka katmandan değer;
  başka katmana bakanlar yalnız İşlemler'de (`compile_with` başka yerde reddeder), katman süzgecinin dışarıda bıraktıklarını okumaz;
  `@` değerleri İşlemler'de, İfade oluşturucu'da, Öznitelik tablosunun süzgecinde ve paftada; Proje ayarları › Değişkenler (ADR 0214);
  yakınlık analizi: İşlemler'in Yakınlık kategorisinde En yakını bul (uzaklık, hedefin alanları ve semt önekle; kenardan kenara ya da
  merkezden merkeze, en çok uzaklıkla), Uzaklık matrisi (en yakın k ya da hepsi; Liste, Matris, Özet), En yakın merkeze bağla ve En kısa
  çizgi (yeni katmanda çizgiler), Komşu alanlar (aynı doğrudaki düz ve aynı dairedeki yaylı ortak kenarlar toleransla, köşe ve örtüşme;
  tablo, komşu sayısı ve adları); hedefler kendi R-ağaçlarında en iyi önce aranır (`PackedTree::nearest`); CBS'de Analiz sekmesinin Analiz panelinde (ADR 0215);
  veri aktarma: İşlemler'in Öznitelik kategorisinde Veri aktar (hedef katmana, boş da olabilir, başka katmandan ya da CSV, TXT, XLSX
  dosyasından Anahtarla birleştir'in anahtar kuralıyla; Alan eşleme parametresi: hedef alan, kaynak sütun ya da yok, varsayılan, Aynı
  adlarla doldur; Üzerine yaz ya da Yalnız boşlara, tipli alanlar kurallarıyla, Geometriyi güncelle, Eşleşmeyenleri ekle, Yalnız raporla;
  rapor tablosu, tek adım); İşlemler'e `allowEmpty`, girdinin katmanı ve şeklin yaması; CBS'de Veri › Öznitelik (ADR 0216);
  katman bölme ve birleştirme: İşlemler'in Katman kategorisinde Özniteliğe göre böl (bir ya da birkaç alanın değerleri, yeni grupta değer
  başına katman, ilk nesnenin katmanının görünüşü ve alanlarıyla; Kopyala ya da Taşı, en çok 1000), Vektör katmanlarını birleştir (alan
  eşlemesiyle, Kaynak katman alanı, tipli hedefin kuralları), Seçilenleri çıkar, Yinelenenleri sil (alanlar ya da bütün öznitelikler,
  isteğe bağlı kesin geometri; ilki kalır), İfadeye göre sırala (`ops::order`: türler, doğal sıra, boşlar, Sıra alanı); İşlemler'e Katmanlar
  parametresi, aracın açtığı katmanlar ve gruplar, `like`; CBS'de Veri › Katman (ADR 0217);
  geometri dönüştürme: İşlemler'in Dönüştürme kategorisinde Nesneden nokta (ağırlık merkezi ya da iç nokta: GEOS'un tarama doğrusu
  yaylarıyla; her parça için), Köşeleri noktaya çevir (bütün köşeler, uçlar, orta; Köşe, Parça, Halka, Uzaklık), Noktalardan çizgi (ifade
  sırası, grup, kapalı), Semt ve uzunluktan çizgi, XY'den çizgi ve Tablodan elips (dosyadan ya da katmandan; satırlar çekirdekte, atlananlar
  raporda), Çizginin parçası (ifadeli uzaklıklar, yaylar kesin), Alanları çizgiye çevir, Çizgileri alana çevir (Atla, Kapat), Köşe ekle
  (aralıkla ya da sayıyla, yaylar yay); kotlar taşınır; CBS'de Veri › Toplu çevir (ADR 0218);
  geometri üretimi: İşlemler'in Geometri üretimi kategorisinde Voronoi alanları (çerçeve Genişletme yüzdesiyle; hücreler kesin Delaunay
  üçgenlerinin çevrel merkezlerinden, komşular köşelerini bit bit paylaşır), Delaunay üçgenleri (köşelerin adları ve kotları), Sınırlayıcı
  geometri (dışbükey zarf, sınır kutusu, en küçük alanlı döndürülmüş kutu, en küçük daire; her nesne, hepsi ya da alana göre; ölçüleri),
  İçbükey zarf (JTS'in ve GEOS'un kenar oranıyla aşındırması, delikler), Ulaşılmazlık kutbu (polylabel); CBS'de Analiz › Geometri ▾ (ADR 0219);
  örnekleme ve ağ üretimi: İşlemler'in Örnekleme kategorisinde Ağ oluştur (dikdörtgen, altıgen, üçgen ya da ağın çizgileri; aralığın
  katlarındaki genel kafes, komşu hücreler köşelerini bit bit paylaşır; Nesnelere değenler Konuma göre seç'in Kesişen'iyle; Satır ve Sütun),
  Düzenli noktalar (katlarda ya da hücre ortalarında, tohumlu kaydırma), Rastgele noktalar (alanda ve çizgide, ifadeli sayı, en az uzaklık;
  SplitMix64 ve xoshiro256** ile her platformda aynı noktalar), Hat boyunca nokta (aralıkla, paylarla ya da eşit parçalara; Uzaklık, Semt,
  kot) ve Alt bölümlere ayır (PostGIS'in ST_Subdivide'ı yaylı geometriye; alan doğru boyunca kesilir); CBS'de Analiz › Geometri ▾ (ADR 0220);
  kartografik genelleştirme: İşlemler'in Genelleştirme kategorisinde Yumuşat (Chaikin; En büyük açı, kotlar oranla, Ortak kenarları koru),
  Dik açılandır (iD ve QGIS'in kuralı), Bina sadeleştir (Ruas ve CartAGen: dik, paralel, ters paralel), Alanları kümele (kümenin kapanışının
  köprüleri, Girintileri doldur, Grupla, Nesne sayısı), Küçük parçaları ayıkla, Küçükleri komşuya kat (en uzun ortak sınır ya da en büyük
  komşu) ve Ortak kenarları koruyarak sadeleştir (kapsama grafı, GEOS'un CoverageSimplifier'ı); Geometri ▾ kategorilerle başlıklı; kesişim
  çekirdeği TOL ölçeğindeki karmaşada yüz yitirmez: TOL içindeki çemberler bir çember, yakın kesişim üçüncü kenarı da keser, köşelerin
  dengesi onarılır (ADR 0221);
  KML/KMZ, GPX ve içe aktarmada açık dönüşüm: KML/KMZ içe aktar (klasörler katman, yer imleri nesne; biçemin rengi ve kalınlığı, ad,
  açıklama, zaman, veriler; gx izleri; resim kaplamaları, modeller ve bağlantılar söylenir) ve dışa aktar (katmanlar grup yollarıyla
  klasör, paylaşılan biçemler, veriler; KMZ), GPX dışa aktar (yol noktaları, iz ya da rota, alanların sınırı kapalı iz); GeoJSON,
  Shapefile ve KML içe aktarmada başka sistemdeki dosya “Projenin sistemine çevir” onayıyla, dönüşümün yolu ve doğruluğu söylenerek
  çevrilir; GeoJSON dışa aktar'da “WGS 84'e çevir (RFC 7946)”; GNSS içe aktar'da rotalar, izler ve NMEA kaydı çizgi; Kaynaklar KML'yi
  tanır; `FORMATS_VERSION` 49 (ADR 0222);
  tablo alışverişi: XLSX ve ODS okuma (sayfalar akışla, `xml_scan`; tarih ve saat biçimli hücre ISO metniyle, 1900 ve 1904; ODS'nin
  tekrarları ve paragrafları LibreOffice Calc'ın okuduğu gibi) ve yazma (XLSX, ODS, CSV; sütunun türüyle sayı, tarih, evet/hayır, metin;
  kalın, donmuş ve süzgeçli başlık, Excel'in sayfa adı kuralı; başka yazılanlar raporda); İşlemler'in dosya parametresi 250 000 satıra
  kadar, Sayfa listesiyle, sınırı aşan söylenerek; Öznitelik tablosunda Dışa aktar ▾, İşlemler'in tablolarında Kaydet ▾; Tablodan nokta
  (dosya ya da katman, düzlem ya da kayıttaki bir sistemden açık dönüşümle, coğrafide DMS); Koordinat listesi çalışma kitabını Tablodan
  nokta'ya yönlendirir; `FORMATS_VERSION` 50 (ADR 0223);
  doğrusal referanslama: köşede ölçü (M; çizginin `ma`, `mb`'si, çoklu çizginin ve parçalarının `ms`'i; alanın yok; `.kcad` şema 39,
  `FORMATS_VERSION` 51), düzenlemeler M'yi kotun kurallarıyla taşır (`measure_lost`, `invalid_measures`), dönüşümler korur; Shapefile
  PolyLineM ve PolyLineZ'nin M'si okunur; İşlemler'in Doğrusal referans kategorisinde Rota oluştur (Uzunluk, Tek alan, İki alan;
  Koordinat önceliği, boşluklar), Rotayı kalibre et, Olayları rotada göster (nokta ve çizgi, öteleme, dosya ya da nesneler), Rota boyunca
  konumla (nokta, çizgi, alan; rotaların R-ağacı), Olayları çakıştır (Kesişim, Birleşim) ve M değerlerini çıkar; çekirdek `ops::lrs`;
  Öznitelikler'de, Koordinat listesinde ve kartta M; Km yaz'ın Ölçü (M)'si ölçülü rotada km'yi M'den yazar; CBS'de Analiz › Ağ analizi ▾
  (ADR 0224);
  nesne ekleri: nesnenin fotoğrafları ve belgeleri (`attachments`: gömülü fotoğraf, bağlı dosya ya da adres, not; `.kcad` şema 40,
  `FORMATS_VERSION` 52; `cad.entities.set`'in `attachments`'ı), Öznitelikler'de Ekler ve Eylemler, fotoğraf görüntüleyici, üzerine gelme
  kartında ilk fotoğraf; Konumlu fotoğraflar (kendi EXIF okuyucumuz, WGS 84'ten Koordinat dönüştür'ün yoluyla, nokta ve eki); katmanın
  eylemleri ve bilgi kartı (şablon `{ifade}`, `{$enlem}`, `{$boylam}`, adreste yüzde kodlama; `cad.layers.info`), Bilgi aracı ve Bilgi
  kartı ve eylemler penceresi, çizimin sağ tıkında Eylemler ▸; masaüstünde fotoğraflar kabuğun wgpu boru hattında (ADR 0225);
  sembol katmanı ekleri: Geometri üretici (15 adım: merkezler, köşeler, sınır, paralel, uzat, tampon, sınırlayıcı kutu, dışbükey örtü,
  daire, daire dilimi, düzgün çokgen, öteleme, döndür, ölçekle; değerleri veriyle), sembol katmanlarının çizim efektleri (gölge, dış parıltı,
  bulanıklaştır, iç gölge, iç parıltı), katmanın Katman çizimi (saydamlık, 13 karışım kipi, etiketlerin ve işaretlerin maskesi; `.kcad` şema
  41, `cad.layers.compose`); çizim alanında üç çizim hattında aynı planla (paketli haritalar, bir grubun efektleri tek geçişte); Sembol
  tasarımcısında adımlar (Adım ekle ikonlu) ve Efektler, Katman stili'nde Katman çizimi (ADR 0226);
  grafikler ve alan istatistikleri: alt panelin İstatistik sekmesi (katmanın bütün alanlarının özeti, seçilen alanın istatistikleri ve en çok
  500 değeri, değere tık seçer; özetler karede 8 ms) ve Grafik sekmesi (Çubuk, Pasta, Histogram, Kutu, Saçılım, Çizgi; katmandan ya da
  İşlemler'in sonuç tablosundan, ROC, Kesit, Zaman serisi ve Histogram kendi grafikleriyle; basmak seçer, Shift ekler, Ctrl çıkarır,
  Saçılım'da kutu, çizimin seçimi vurgulanır; ipucu, Grafik | Tablo, Panoya kopyala, PNG ve SVG); `data.statistics`, `data.chart`,
  `chart.*`; Katmanlar'ın sağ tıkında; hesap ve yerleşim çekirdekte (`ops::charts`, `kentos.charts/1`), web'e `ChartWork` (ADR 0227);
  adres arama, yalnız masaüstünde (web'e ara, ADR 0228): projenin adres verisi (kapı noktaları, yol orta hatları, mahalle, ilçe ve il
  alanları katmanlarıyla ve alanlarıyla; en düşük puan, Konumdan adres uzaklığı; `.kcad` şema 42, `cad.addresses.define`); adresin
  parçaları (Mah., Cad., Sk., Blv., No:, 12/A, posta kodu), yazım hatasına dayanıklı benzerlik, kapı, kapılar arası aradeğer, yol ve
  mahalle adayları puanlarıyla, Eşleşti, Belirsiz, Eşleşmedi, adres kodu, konumdan adres (`ops::geocode`); çevrimiçi sağlayıcılar
  (Nominatim, Photon, ArcGIS World, Google, kurum sunucuları) kartlarıyla: koşullar onaylanmadan kullanılmaz, hız, toplu iş ve saklama
  kuralları kodda, önbellek ve anahtarlar cihazda (`kentos_services::geocode`); alt panelin Adres sekmesi (Kaynak, Ara, Konumdan adres;
  Git ve işaret, Nokta ekle, Taşı), Konumdan adres aracı, Adres verisi ve Adres sağlayıcıları pencereleri, İşlemler'in Adres kategorisi
  (Adresleri eşle, Konumlara adres yaz); şeritte CBS › Harita ve CAD › Ekle'de Adres; web dururken yeni komutların arayüz kaydı
  `catalog/v1/additions.json`; Python `kentos.address` (ADR 0229);
  CAD işlem manifesti: arayüzün her komutunun, aracının ve yönteminin ürün işlemi düzeyi (semantic, geometry, processing, none,
  query, session, host) ve Python ile AI karşılığı `catalog/v1/operations.json`'da, üretilen `docs/cad/manifest.{json,md}` CAD
  tarafının boşluklarıyla; yeni komut eşlemesiyle gelir (ADR 0244; plan/cad.md, TODOS.md §16.5);
  ilişkili ölçü: nesnenin yerleri (uç, köşe, kenar ortası, merkez, düğüm) ve başvuru (`uid`, yer, kopma nedeni; `.kcad` şema 43,
  `FORMATS_VERSION` 55), çekirdeğin `ops::subref`'i (yapı aynıysa yer korunur, değiştiyse köşe bit bit aranır); Hizalı, Doğrusal,
  Koordinat, Semt ve Eğim'in kenetle bir yere oturan noktası bağlanır, belge kayıttan önce aynı adımda izler (`dimension_ties.rs`),
  kopan bağ nedeniyle kalır ve çizimde uyarı üçgeniyle görünür; Öznitelikler'de İlişki ve Serbest bırak, Açıklama › Ölçü'de Yeniden
  bağla, `cad.dimensions.reassociate`; Python ve MCP (ADR 0249);
  anlamsal Buda, Uzat, Köşe yuvarla ve Pah: `cad.entities.trim` ve `.extend` (seçimler ve açık sınırlar: bütün görünenler,
  dikdörtgen ya da verilenler; Çit'in sırası), `.fillet` ve `.chamfer` (yolun köşesi, iki çizgi ya da bütün köşeler; Kırp); komut
  bağlamı ev sahibinin geometri deposunu taşır (`ExecutionContext::with_store`), başsız sunucu kendisininkini kurar; araçlar
  önizlemeyi planla, yazmayı komutla yapar; `cad.entities.create` ilişkili ölçünün bağlı noktalarını yerlerinden yazar (ADR 0250);
  pafta kitabı projenin dosyasında: `.kcad` şema 44'ün `sheets`'i (pafta çekirdeğinin kitabı opak, resimlerin baytları SHA-256'larıyla;
  `FORMATS_VERSION` 56), belgede düzenleme ama geri alma adımı değil (`set_sheets`), pafta çekirdeğinin `project`'i; masaüstünde kitap
  çizimle açılır ve paftadaki her değişiklik çizime yazılır (`sheet_book.rs`), cihazda eski yerinde kalan kitap projeye alınır ve
  kaydedilince `pafta/kitaplar/tasindi/`'ye ayrılır, okunamayan kitap olduğu gibi kalır; veritabanı projesinde `project.sheets` (göç
  0017), `ProjectPatch.sheets`, kopya ve geri yükleme; bulut eşitlemesi onu meta olarak taşır (ADR 0251);
  ilk uçtan uca kabul çizimi: oda, düzeltme, ilişkili ölçü ve tarama, duvarlar, 1:50 ve 1:100 A3 paftalar, DXF ve PDF, başka ortamda
  açma tek senaryoda (`fixtures/acceptance/v1/room.json`, beklentiler `acceptance_room.py`'den) arayüzde, ajan bağlantısında, Python'da
  ve MCP'de aynı sonuçla, dosyalar bağımsız okuyucularla; `cad.sheets.edit` (pafta çekirdeğinin işlemleri, kâğıdın geçmişi `inverse`'le),
  `kentos-plot` (haritaların vektörleri, paftanın girdileri, yazı tipleri, başsız kitap PDF'i; haritaların yazıları henüz yalnız
  masaüstünde), başsız DXF (`kentos_headless::export`); Python `export_dxf`, `export_pdf`, `sheets`, MCP `drawing.exportDxf`,
  `drawing.exportPdf` (ADR 0252);
  komut protokolü: her yürütme belgenin tek adımı (`run_step`), nesne, blok, katman ağacı, ayar, stil ve pafta durum damgaları
  (kanonik JSON'un SHA-256'sı; sayılar en kısa, eşitlikte çift), çağrı zarfı ve cevabı (`requestId`, `expected`, `planId`), planın
  özeti ve parmak izi, idempotency defteri (`.kcad` şema 45, `FORMATS_VERSION` 57), kayıt `kentos.calls/1` ve tekrar oynatma (dosya
  komutlarının baytları karşılaştırılır), kataloğun `capabilities`'i ve kodların tek tablosu (`codes.rs`, `host` türü dahil),
  `CommandError`'ın `uids` ve `retryable`'ı, ortak değişiklik kümesi `cad.changeset.apply`, uzun işler (`start`, `job`, `cancel`;
  hesap, `staged`, kesinleşme), `file.export.dxf` ve `.pdf` atomik yazmayla; başsız sunucu, Python, MCP ve masaüstünün ajan bağlantısı
  aynı yoldan (ADR 0253);
  anlamsal Esnet, Kır, Birleştir, Böl, Alan oluştur ve Tarama: `cad.entities.stretch` (pencere, fark, seçim, Topoloji'nin komşuları),
  `.break`, `.join` (Zincir), `.divide`, `.boundary`, `.hatch` (nokta ya da Çoklu tara, desen adıyla); bölgeler ve komşular uygulama
  katmanında (`region.rs`, `RegionCache`, `topology_follow.rs`), alanın halkaları kanonik; araçlar önizlemeyi planla, yazmayı komutla
  yapar (ADR 0254); Ölçülendirme'nin on türü `cad.dimensions.create` ile: ölçülen nokta koordinat ya da yer, kenar nesne ve tıklanan
  yerle, yer `at` ya da yazılan uzaklık, stil adıyla; aracın önizlemesi `measured_in` (ADR 0255);
  birim değiştirme ve birimler: `cad.units.change` boyları (yalnız birim) ya da sayıları (çizim birimlerin oranıyla) korur, birim ve
  çizim tek adım (`Op::Units`), Proje ayarları sorar; ölçekleme kuralı sözleşmenin `units`'i (DXF'in birim çevirisi de: kutu, çerçeve,
  eğri artık ölçeklenir); komut şemalarındaki her sayının birimi `x-unit` (kataloğun `units`'i, birimsiz sayıyı test düşürür); ölçünün
  `basis`'i (sistem, `frame`, `method`: düzlem, elipsoit, zemin) ve toplu ölçü; özetin `conventions`'ı (eksen adları, kutupsal açı)
  (ADR 0256);
  DXF'te dosyanın birimi ve içe almanın komutu: birim tek kuralla (seçilen `fileUnit`, yerel projede bildirilen, sistemli projede
  metre, bildirmeyen dosyada projenin birimi; belirsiz dosya sessizce ölçeklenmez), `UnitDecision` ve uyarıları, `BLOCK_RECORD`'un
  70'i; DXF içe aktarma penceresinde “Dosyanın birimi” (kararın sözü, seçim dosyayı yeniden okur); `file.import.dxf` (pencere
  okuduğu dosyayla komutla yazar, büyük dosya kare kare komutun planı ve kaydıyla; Python `Document.import_dxf`, MCP); içe almanın
  kuralları uygulama katmanında (`import_apply`) (ADR 0257);
  kapalı ve genişlikli çoklu çizgi: çoklu çizginin `closed`'ı (kapalı çizgi, alan değil; gösterilen alanı `enclosed_area`) ve `widths`'i
  (köşe başına başlangıç ve bitiş genişliği, çizim uzunluğunda, kâğıt kalınlığından ayrı; `.kcad` şema 46, `FORMATS_VERSION` 59);
  çekirdekte yolun kapalılığı `is_closed_path`, şerit `geom::band` (`MIXED` kaydı, nesnenin renginde dolu), genişliğin taşınması
  `ops::widths` (`widths_lost`); `cad.polyline.create`'in `closed` ve `widths`'i, `invalid_polyline`; Çoklu çizgi'de Kapat (K) ve
  Genişlik (E), Öznitelikler'de Kapalı ve Genişlik; DXF'in 43, 40 ve 41'i ve `closedPolylines` (CAD projesinde kapalı çizgi) (ADR 0258);
  NURBS eğrisi: `nurbs` türü (derece, denetim noktaları, düğümler, ağırlıklar; kapalı, periyodik, geçiş noktaları, teğetler; `.kcad` şema 47,
  `FORMATS_VERSION` 60), çekirdek `geom::nurbs` ve `nurbs_build` (Piegl ve Tiller; geçiş noktalarından global interpolasyon), budama ve
  kırma `ops::nurbs_cuts`; `cad.nurbs.create`, NURBS eğrisi aracı (Yöntem, Derece, Kapat); DXF'in SPLINE'ı tam NURBS; daire, yay ve elips
  analitik, Catmull–Rom eğrisi ayrı tür; uzunluklar OpenCASCADE'le (`curves_occt.py`) (ADR 0259);
  nesnenin görünüşü: kendi çizgi tipi, ölçeği ve saydamlığı (`lineType`, `lineTypeScale`, `transparency`), bloğun nesnesinde katmana göre
  parçalar (`byLayer`), katmanın içinde çizim sırası (`order`; Öne getir, Arkaya gönder: `cad.entities.order`; stil motorunda sıra
  bantları), katmanın `plot`'u (Yazdır / Yazdırma, pafta atlar); `.kcad` şema 48, `FORMATS_VERSION` 61, sunucuda göç 0018
  (`feature.look`); `cad.entities.set`'in üç işlemi, oluşturan komutların `lineType`'ı (şeridin Çizgi tipi yeni nesnelere), DXF'in 6, 48,
  440 ve 290'ı (ADR 0260);
  kullanıcı koordinat sistemi (2B UCS): projenin `ucs`'i ve adlı UCS'leri (`namedUcs`; `.kcad` şema 49), `cad.ucs.set` (Dünya, Çerçeve,
  Orijin, Döndür, İki nokta, Nesneye hizala, Adlı, Kaydet, Sil; ayar, geri alma adımı değil, nesneler değişmez); çekirdek `tools::ucs`
  (`Frame`, `point_from_text_framed`, `constrain_cursor_in`, `object_frame`); yazılan nokta, Orto, kutupsal izleme, açı kilidi, ızgara,
  Karelaj, Doğrusal ölçü ve yeni taramanın açısı UCS'in eksenleriyle; durum çubuğu ve Koordinat oku UCS'te, eksen simgesi döner;
  Görünüm › Koordinatlar ve Önceki UCS (oturumun geçmişi); tel dünyadır: Python `Document.ucs`, `to_world`, `to_ucs`, MCP
  `drawing.toWorld`, `drawing.toUcs`, özetin `conventions.ucs`'i (ADR 0261);
  ayak ve inç: yerel projenin `in` ve `ft` birimleri (tam oran 254/10000 ve 3048/10000) ve `lengthStyle` (mimari `12'-6 1/2"`,
  mühendislik `12'-6.50"`, kesirli `150 1/2"`; `.kcad` şema 50), yazılan uzunlukta ayak ve inç işaretleri her projede (`5'6-1/2"`,
  çekirdeğin `tools::imperial`'i ve `point_text`'in `Len`'i), Proje ayarları ve sihirbazda İnç ve Ayak, `cad.settings.set`'in
  `lengthStyle`'ı (`style_needs_imperial`), `cad.units.change`'in oranı, DXF'in `$INSUNITS` 1 ve 2'si (ADR 0262);
  eğri üzerindeki yer ve blok içi alt nesne: başvurunun `along` yeri (kenar ya da eğri boyunca `t`; Yakın, Dik, Teğet ve Çeyrek
  kenetten), eğri aralığı (`SubRange`, `range_length`, `follow_range`), blok içi parçanın yolu (`path`; yerleştirme ya da tanım
  değişince belge izler, `follow_in`, `piece_gone`), `.kcad` şema 51, `FORMATS_VERSION` 62, Yeniden bağla ve ölçü komutları (ADR 0263);
  ileri CAD nesnelerinin kaynak modeli (dolu alan, bölge, wipeout, merkez işareti, eksen, çok kollu kılavuz; ADR 0264); modelin taşıma
  matrisi (bütün `.kcad` örnekleri kodekten, sütunlardan, bulutun kaynağından ve Python'dan; `tests/carriage.rs`, `test_carriage.py`) ve
  yeni şemanın altı adımlık kuralı (`kentos-kcad`'in `schemas` testi) (ADR 0265);
  yazılan uzunlukta metrik birim (`2.5m`, `250cm`) ve parantezli toplam (`(12.5+3*2)`), araçların uzunluk alanları tek yardımcıdan (ADR 0266);
  koordinat kilitleri: değer alanında doğu ve kuzey (projenin eksen adlarıyla X ve Y) ayrı kilitlenir, uzunlukla birlikte de (çemberle
  doğrunun imlece yakın kesişimi), kilidin çerçevesi UCS'tir ve çipte “(UCS)” der, kilitli koordinatın kesikli kılavuzu; `draft.lock.east`,
  `.north`, çekirdeğin `coordinate_point`'i (ADR 0267);
  çizim yardımlarının önceliği: koordinat kilidi, doğrultu kilidi, Dik açı, kenet ve izleme, Orto, kutupsal izleme (`Aid`,
  `Session::ruling_aid`); Orto ve kutupsal birbirini kapatmaz, Orto önce gelir; Shift ve F8/F10 önizlemeyi hemen yeniler, durum
  çubuğunda Orto'nun geçici hâli ve kutupsalın askısı; izlerin `hold` adımı ve `aid` beklentisi (ADR 0268);
  kenet türlerinin kapsam matrisi (`snap_coverage_cases.py`), Ekleme noktası türü (yazı, blok, tablo; `snap.insertion`), yayın ve yaylı
  kenarın çeyrekleri, tek seferlik Merkez'de eğrinin üstünden merkez (ADR 0269);
  nokta yapımları: salt okunur `cad.points.construct` (iki nokta ortası, uzantı, paralel, izleme, kutupsal, hat üzerinde), Python ve MCP'de;
  Geçici izleme noktası (`draft.trackPoint`), tek seferlik menüde Referans noktası ve İki nokta ortası (ADR 0270);
  kenet adayları arasında Tab ve Shift+Tab (çekirdeğin `snap_candidates`'i: ağırlıklı uzaklık, eşitlikte çizimin sırası), işarette
  nesnenin türü ve sıra (ADR 0271);
  tutamaç menüsü her tutamakta (Esnet, Taşı, Döndür, Ölçekle, Aynala; tutamacın noktası ilk nokta) ve köşe komutu
  `cad.entities.vertex` (köşe sil, köşe ekle, düz kenar, yaya dönüştür; topoloji komşularıyla); tutamaç menüsünün köşe işleri ve
  Köşe ekle/sil aracı onunla yazar (ADR 0279);
  tutamaç kipleri: Shift ile birden çok sıcak tutamaç, E Esnet, T Taşı, D Döndür, L Ölçekle, A Aynala, B taban noktası, R referans,
  K kopya (ADR 0280);
  yazılan adlar: seçenek adıyla ya da adının başıyla, komut satırında bağlama göre tam ad ve tekil kısaltma, belirsiz olan adaylarıyla
  söylenir; takma ad ve seçenek harfi çakışmaları testte (ADR 0281);
  AutoCAD takma adları: Uygulama ayarları › Görünüm › Komut satırı'nda kapalı başlayan tercih (`drafting.autocadAliases`); komut satırı
  AutoCAD'in acad.pgp adlarını da alır (RECTANG, LA, DT, U, UNDO, QSAVE; yöntemiyle JOG), profil veridir
  (`catalog/v1/aliases-autocad.json`), KentOS'un adları, yöntem adları ve tek tuş kısayolları önce gelir (B, CP, D, I, P, POLYGON, S, T
  eklenmez); öneri listesinde ve F1'de görünür, tuşlar ve seçenek harfleri değişmez (`alias_profile.rs`, ADR 0282);
  şeffaf komutlar: Kaydır, Pencere yakınlaştır, Mesafe ölç, Açı ölç ve Koordinat oku çalışan komutun ya da sıcak tutamacın üstünde
  çalışır (`Session::start_over`, `TRANSPARENT`), komut noktaları, seçenekleri, istemi, kilitleri, referansı ve Yapım kipiyle bekler,
  taslağı altta çizilir, Esc ya da Enter ile kaldığı yerden sürer; askıdakiler yığın (nokta hesaplayıcı şeffaf ölçmenin üstünde de);
  görünüm değişince ya da komut dönünce imleç yeniden verilir, önizleme yeni görünümde imlecin altında (`follow_view`); çizen araç
  şeffaf değildir (ADR 0283);
  komut geçmişinde çağrılar: Komut geçmişi çizimin kaydını (`kentos.calls/1`) okur ve her ürün komutunu Türkçe başlığı, düz Türkçe
  girdisi, yazdığı, uyarı ve hata kodlarıyla gösterir (aynı geri alma adımı birlikte, geri alınan soluk); yazmadan dönen çağrılar
  `call::recorded`'dan belgeye not edilir (`Document::note_refusal`, `refused_calls`; oturumun); her çağrının Python kodu
  (`kentos_native_application::python_call`) gösterilir ve kopyalanır, tekrar oynatma girdisi (`kentos.replay/1`) kopyalanır ya da
  kaydedilir; ortak komut durumlarının çağrıları `scripts/python/history_calls.py` ile SDK'ya karşı denetlenir (`history/`, ADR 0284);
  büyük seçimde Öznitelikler: karışık seçimde kilitsizler düzenlenir, kilitliler atlanır ve ilk satırda sayılır; Katman ortak ya da Çeşitli;
  Ortak öznitelikler (değer ya da “Çeşitli (k/n nesnede)”, bütün seçime tek adımda); 100 000 nesnede panel 48 ms (ADR 0286);
  çizim araçlarının yöntemleri: AutoCAD'le karşılaştırma ADR 0287'nin tablosunda; araçların yöntemleri arayüz kaydında
  (`catalog/v1/additions.json`'un `methods`'u, `AddedMethod`, `place_methods`); şeritte Yay'ın 11 yöntemi, Daire'nin Merkez, çap'ı, Elips
  ve Revizyon bulutu yöntemleri, Düzgün çokgen'in çember içinde, dışında, kenardan'ı; Dikdörtgen'in Alan'ı (A) ve Boyutlarla, Alanla
  yöntemleri, Yardımcı çizgi'nin Paralel'i (P, Noktadan), Revizyon bulutu'nun Nesne'si (N); hesap çekirdekte (`rect_side_for_area`,
  `xline_offset`, `cloud_around`), yazma araçların kendi komutlarıyla (ADR 0287);
  formlar (ADR 0400–0446; 0446 ajanın form işinin değerlendiricisi `kentos.forms_eval` ve `fixtures/forms-eval/v1`; 0445 `cad.forms.edit`'in `withdraw`'ı ve formun `withdrawn`'ı (`form_withdrawn`, `form_enforced`), bağ tablosu geometrisiz kayıt katmanı ve iki ilişkiyle; 0444 ilişkili kayıtlar öğesinin `min`, `max`'ı (`form_related_few`, `form_related_many`, gönderimin alt kayıtları sayılır), yayının `fields`'ı (`bound_fields_digest`) ve `cad.forms.validate`'in `fieldsChanged`'ı, Formla düzenle'de ⓘ, pencere ile komutun eşdeğerlik testi; 0443 `cad.schema.describe` (sayfa sayfa şema keşfi), alanın `redact`'ı ve komut geçmişinin gizleyen Python betiği, Python `cad.compatibility`, `cad.require`, `cad.FORM_PROFILE`, formun `translations`'ı (`form_translation_item`), grubun `across`'u, Sonraki sorun, kapanış sorusu, Kaydet'in bütün taslağı yazması; 0442 yayında senaryolar `form_tests_failed` ve kırılan kayıtlar `breaking`, `form_breaking`, sürüm farkının `areas`'ı ve `requiresAdded`, iş akışının `notes`'u ve `cad.forms.transition`'ın `notes`'u, eylemin `kind`'i (tool, recompute, step, show) ve `runs`'ı, `form_action_step`, `cad.attachments.inspect`, Formla düzenle'de Yeni sürümle aç ve Alanlara not; 0441 bölücü `divider`, eylem `action` (kayıtlı işlem aracı), ilişkili kayıtların `chart` görünümü, formun `comments`'i ve `cad.forms.edit`'in `comment`, `resolve`'u, `cad.forms.validate`'in `requires`'ı (`kentos_forms::requirements`), tasarımcıda paletler, cihazdaki bölüm kitaplığı ve kalan taslak (`forms/library.rs`); 0440 `cad.forms.submit`'in `children`'ı ve cevabın `changes`, `snapshot`, `undo`'su, silmenin `followed`'ı ve `relation_locked`, `relation_in_use`, `relation_cycle`, iş akışı satırının anlık görüntüsü, ilişkili kayıtların `cards` görünümü, iç içe form; 0439 başka katmandan listenin `sort`, `columns`, `limit`, `copy`, `stamp`, `duplicates`, `spatial`'ı, zincirin `fold`'u, `minSearch`, `cad.domains.export`, `cad.domains.edit`'in `import`'u, `cad.fields.migrate`, işlem araçlarında Yinelenen anahtar; 0438 kayıt kuralları `rules`, `cad.layers.rules`, `rule_broken`, kontrol zamanı `on`, ekranın yayını ve kaydı `release`, `opened`, tekil alan, hariç uç, birim, değerin günleri; 0437 yansıma, ağaç, adım adım, kopyala ve yapıştır; 0436 tarih ve saat aralığı; 0435 form paketi `cad.forms.export`, `cad.forms.import`; 0434 formun JSON Schema'sı ve `docs/specs/forms-v1.md`; 0433 formda konum `location`; 0432 taslak durumlar `drafts`; 0431 aranır liste ve etiketler `search`, `chips`; 0430 projenin değer listeleri `domains`, alanın `domain`'i, `cad.domains.edit`; 0429 iş akışının geçmişi `log`; 0428 kayıt iş akışı `workflow`, `cad.forms.transition`; 0427 sürümleri karşılaştır `cad.forms.compare`, `kentos_forms::compare`; 0426 sekmenin kuralı ve Alt türlerde göster; 0425 ilişkili kayıtların tablosu `view`, `columns`; 0424 büyük form ve katmanda süreler `forms_timing`; 0423 değer bileşimleri `contingents`, `cad.layers.contingents`, `invalid_combination`; 0422 alt türler `subtypes`, `cad.layers.subtypes`, `subtype_misfit`; 0421 Hesapları yenile `cad.forms.recompute`; 0420 form senaryoları `tests`, `cad.forms.test`, `kentos_forms::scenario`; 0419 alanın bölme ve birleştirme kuralı `split`, `merge`, `field_policy.rs`; 0418 kullanımdan kalkan liste değeri `retired`; 0417 değiştirilebilir hesap `overridable`; 0416 Öznitelik tablosunda form görünümü (Tablo | Form, `Entry::docked`); 0415 kaydı alt kayıtlarıyla çoğaltma `cad.records.duplicate`; 0414 alanın kullanıldığı yerler `cad.fields.usage`, yeniden adlandırmada başvuruların izlenmesi `kentos_forms::field_refs`; 0413 Öznitelik tablosunda koşullu biçim `tableStyles`, `cad.layers.table`; 0412 tarih ve saat, saat alan türleri; 0410 geometrisiz kayıt `record`, `cad.records.create`, Öznitelik tablosunda Kayıt ekle; 0411 `forms.design`, `forms.publish`, `forms.use`, Veri girişi rolü, sunucunun yazma bekçisi `form_guard.rs`): Tab ile alanlar arasında gezinme ve klavyeyle form öğeleri; katmanın veri giriş ekranı projede
  (`.kcad` şema 72 `forms`, alanın kararlı `id`'si), çekirdek `kentos-forms` (ön denetim, kuralların durumu, kaydın doğrulaması,
  yazma profili), `cad.forms.edit`, `.validate`, `.submit` ve `cad.entities.set`'te `form_rules`; Formlar penceresi (tasarımcı) ve
  Formla düzenle; tipli evet/hayır ve sayı alanları ifadelerde türüyle okunur; Python `kentos.cad.forms`, MCP; ifadeli varsayılan,
  hesaplanan alan, seçenek süzgeci (`@secenek`), tekil alan, son değeri kullan, uyarı denetimi, kapalı grup, gizlenenleri silme,
  şablonlu not; kurallarda nesnenin ölçüleri ve komutun `moment`, `user`'ı; belge formu uygulanan katmanın her yazmasını aynı
  adımda tamamlar (`kentos_domain`'in `form_follow.rs`'i); tasarımcıda geri alma, ağaçta sürükleme ve canlı önizleme, çizilen
  nesnenin formu (`drafting.formOnCreate`), toplu yazma `cad.forms.fill` ve Formla düzenle'nin toplu kipi; başka katmandan
  değer listesi (`lookup`: zincir, süzgeç, `$uid` ile nesneye bağ) ve ilişkili kayıtlar öğesi (`related`), `Document::form_world`;
  projenin ilişkileri (`relations`, `cad.relations.edit`; silme politikası belgede `relation_follow.rs`, `restrict`
  `cad.entities.delete`'te), İlişkiler penceresi, formda Bağla ve Bağı kopar; katmanın bütün kayıtları formla salt okunur
  `cad.forms.audit` ile (Kayıtları denetle), sayı alanında kaydırıcı (`slider`, aralık ister); formda ekler öğesi (kaydın
  fotoğrafları ve belgeleri, en az ve en çok), renk öğesi ve katmanın değerlerinden öneri (`suggest`); çok değerli alan
  (`LayerField.multiple`, kodlar `; ` ile) ve çoklu seçim öğesi, Formla düzenle'de kayıtlar arasında ◀ ▶; çoktan çoğa
  ilişki (çok değerli başvuru, silme politikası öbür bağları korur) ve ilişkili kayıtlarda Yeni;
  CBS planı ([plan/gis.md](plan/gis.md), ADR 0500–; 0513 servisten veri almanın bitişi (`feed::Ending`: complete, capped,
  short, pages; WFS 2.0 sunucunun kestiği sayfada sürer) ve OGC API'nin uygunluk profili (`profile::FeedProfile`; CQL2 metin
  süzgeci yalnız bildiren servise), Servisten veri al'da Uygunluk satırı; 0512 bağlı katmanda geniş değerler: eşitleme hücreleri sütunun okuma kuralıyla
  karşılaştırır (`geopackage::same_cell`; 2⁵³ üstü tam sayı, saat dilimi); 0511 işlem sonucunun soyu: girdilerin içerik özeti (`RunInput.digest`),
  `sink`'in tablolarında GeoPackage metadata uzantısıyla `kentos.lineage` 1 (`GeoPackage::add_metadata`); 0510 bağlı katmanın parçası: `cad.sources.edit`'in `extent`'i (alan
  sınırlı çalışma kopyası, `SourceLink.extent`; eşitleme alanın dışına dokunmaz, dışarı taşınan nesne kendi satırına),
  GeoPackage kutu sorgusu dizinsiz tabloda da süzer, `fids`; 0509 sağlayıcının ölçüm tabanı (`provider_timing`, 1 milyon parsel;
  `docs/perf/provider-2026-10-18.md`) ve bağlamada bellek (katmanın özeti ve dosyanın SHA-256'sı akışla, yeni katmana bağlamada
  satırlar bir kez); 0508 İşlemler'in araçları komut olarak: `cad.processing.tools` ve
  `cad.processing.run` (pencerenin çalıştırıcısı, açık değerler, nesneler kalıcı kimliklerle ya da girdinin `selection` ve
  `view`'u, plan kopyada; `sink` sonucu yeni bir GeoPackage'a yazar, çizim değişmez, soy `inputs`), ev sahipleri araçları
  kaydeder (`processing_host::register_tools`, `kentos_processing::command`);
  0507 toplu yazma: `cad.fields.write` (Alan hesapla: ifade, kapsam, boş
  sonuç; Yapıştır: satır başına değerler; değerler `cad.entities.set`'in kurallarıyla, tekil alan birlikte; bütün ya da hiç ya da
  uyanlar, plan önizleme ve sorunlar), Öznitelik tablosunda Alan hesapla… ve Yapıştır; 0506 taşınabilir proje: kaynağın göreli yolu (`SourceLink.relative`,
  kaydetmede yazılır, açılışta aynı içerikteki dosya bulunur, `source_places`), Kaynak ▸ Yeniden bağla, Paketle (`file.package`:
  proje ve `veri/` altında bağlı dosyaların kopyaları, bütün ya da hiç, SHA-256 denetimiyle; kalanlar `package_left`); 0505 PostGIS bağlı katmanı: `cad.sources.*` adresle, sağlayıcı
  `provider::TableProvider` ve kendi crate'i `kentos-postgis` (sqlx, TLS; masaüstü, Python ve MCP kaydeder), sürüm satırların
  SHA-256'sı, yazma kilit altında tek işlemde, `kentos_uids`, parolasız adres, masaüstünde PostGIS'e bağlan; 0504 kaynağın sistemi: bağlı katmanda `reproject` (tablo kendi sisteminde,
  yazma ona geri çevirerek), çekirdeğin `crs::transform_bounds`'u (sıklaştırma, antimeridyen); 0503 bağlı katman: GeoPackage tablosunun çalışma kopyası, projenin `sources`'ı
  (`.kcad` şema 74; dosyanın SHA-256'sı ve katmanın özeti), `cad.sources.describe`, `.edit`, `.sync` (tek SQLite işleminde yazma,
  `source_changed`, `pending_changes`), masaüstünde Bağlı katman olarak aç, rozet ve Kaynak ▸; 0502 GeoPackage: kendi crate'i `kentos-geopackage` (rusqlite, gömülü SQLite), `file.import.gpkg`,
  `file.export.gpkg`, masaüstünde içe aktarma penceresi; 0501 standart ilişkiler: kesin DE-9IM `ops::de9im`, on yüklem ve desen,
  `cad.entities.relate`; 0500 bindirmenin doğruluk korpusu `fixtures/overlay/v1`, GEO-01 kapandı);
  görünümler: adlı görünümler projenin ayarı (`.kcad` şema 52 `namedViews`, `cad.views.edit`: keep, rename, remove; Görünümler penceresi;
  Python, MCP), Görünümü döndür (iki nokta ya da açı, UCS'e göre, Kuzey yukarı; kamera dönüşü `crates/render/wgpu` kamerasında, dönmüş
  görünümde seçim kutusu çokgen sorgusu), Görünümü böl (yan yana iki model görünümü, etkin olan basmayla değişir) (ADR 0285);
  çoklu çizginin yay ikilileri ve kenar düzenlemeleri: Çoklu çizgi'de PLINE'ın Açı+Merkez/Yarıçap, Merkez+Açı/Uzunluk ve Yarıçap+Açı yayları,
  yay kipinde Genişlik (E) ve teğet yayla Kapat (K), kenar başına başlangıç ve bitiş genişliği; `cad.entities.vertex`'in Yaya dönüştür'ü
  noktadan (`through`) ya da yarıçapla (`radius`), Kenar genişliği (`width`; `cad.entities.edit`'in `EdgeWidth`'i); çekirdeğin `bulge_via`,
  `bulge_of_radius`'u; tutamaç menüsünde Noktadan ya da yarıçapla yay… ve Kenar genişliği… (Kenar düzenle) (ADR 0288);
  dikdörtgen ve düzgün çokgen komutları: `cad.rectangle.create` (iki köşe, boyut, alan ya da kenar ve en; dönüş; keskin, yuvarlak, pah)
  ve `cad.polygon.regular` (çember içinde, dışında, kenardan) halkayı çekirdekle hesaplar ve `cad.polygon.create`'in yolundan yazar;
  araçlar önizlemeyi komutun hesabından (`shapes::rectangle_ring`, `regular_ring`) alır, Çakışma ve Topoloji alanı değiştirince alanın
  yolundan yazar (`points::write_area_by`) (ADR 0290);
  eğri yapımı ve yöntemlerin doğrulanması: salt okunur `cad.curves.construct` (iki ve üç noktadan, TTY ve TTT daire; yayın on bir yöntemi ve
  Devam; eksen uçlarından ya da merkezden elips ve eliptik yay), Yay, Daire ve Elips eğriyi onun hesabından alır ve reddini söyler;
  bağımsız başvuru `curve_method_cases.py`, ortak durumlarda `within` (ADR 0289);
  dönüşüm kuralları: Taşı ve Kopyala'da Yer değiştirme (Y ya da temel noktada Enter; `Memory::last_displacement`), Kopyala'da Kip
  (`Memory::copy_single`) ve Dizi (adet, Sığdır; `cad.entities.array`'in `line`'ı), Döndür'de projenin açı düzeni
  (`Format::turn_from_typed`, `direction_from_typed`: CAD derece saat yönü tersine, CBS grad saat yönünde) ve Noktalar, Ölçekle'de
  Noktalar; her türün dönüşümü bağımsız başvurudan (`transform_kinds_cases.py`), dolu tarama deseniyle kalır (ADR 0291);
  aynada yazılar: `cad.entities.transform`'un aynasının `text`'i (readable varsayılan, mirrored; MIRRTEXT), çekirdeğin `TextPolicy`'si ve
  `text_frame`'i: okunur yazı aynadaki yerinde, baş aşağı olmadan, hizası değişerek; bloğun yazıları ve öznitelikleri her zaman okunur,
  değerleri değişmez; Aynala ve tutamaç kipinde Yazılar (Y, `Memory::mirror_text_mirrored`); web'in çağrısı eski kuralda (ADR 0292);
  Ötele'nin kesinliği ve raporları: delikli ve çok parçalı alanın delikleri öbür yana, fit-point eğrisi ve NURBS kirişleriyle (yaklaşık,
  `approximate_offset`), yarıçap ve alan genişliğinin çökmesi `radius_collapse` (tersine dönen halka yazılmaz), kendini kesen kopya
  `self_intersecting`; çekirdeğin `offset_noted`'ı, `offset_entity` web'in kayıtlarıyla aynı (ADR 0293);
  Buda ve Uzat'ta AutoCAD'in seçenekleri: Çit (`fence`) ve Kesişen (`crossing`) komutun girdisi, Uzantılar (`edgeMode`: çizgi doğrusuyla,
  yay çemberiyle, açık çoklu çizginin uç kenarları), hızlı kipte kesilmeyen nesnenin silinmesi (`eraseUncut`), kalan parçanın bütünüyle
  gitmesi; araçta K, boş yere tıklama, U ve Sil (R); tutamaç kipinin Döndür'ü projenin açı düzeniyle (ADR 0294);
  Köşe yuvarla ve Pah'ta iki nesne: çizgi ya da açık çoklu çizginin uç kenarı (`corner::side_of`, `lines_corner`), aynı yolun iki ucu
  yolu kapatır; Tüm köşeler uçları buluşan çizgilerde de (`corner::line_corners`), araçlar aynı işlevlerle (ADR 0295);
  Kır, Birleştir ve Yönü çevir'de taşınanlar: parçalar katmanı, görünüşü, öznitelikleri, kotları, ölçüleri, genişlikleri ve ekleri taşır,
  ilişkili ölçüler, bağlı yazılar ve taramalar izler ya da nedeniyle kopar; Yönü çevir `cad.entities.reverse` (ADR 0296);
  Esnet'te yalnız penceredeki köşeler: yayın köşeleri uçlarıdır (bir ucu taşınan yay kiriş yüksekliğini korur), bağlı olduğu nesneyi
  izleyen nesne (ilişkili tarama, bağlı yazı, ölçünün bağlı noktası) kendisi esnemez, nesnesini izler (`follows_object`); plan ve sonuç
  izleyenleri verir (`dependents`, belgenin `followers`'ı bir kopyada), Esnet önizlemesinde noktalı (ADR 0297);
  anlamsal Uzat-kısalt: `cad.entities.lengthen` (uç, Fark, Yüzde, Toplam ve Dinamik; yayda açıyla), elips yayı kendi elipsinde,
  uzayan ucun kotu ve ölçüsü uç kenar boyunca sürer; araç komutun planıyla önizler, Fark ve Toplam'da `<açı` (`Tool::takes_angle`) (ADR 0298);
  Böl ve Ölç'te blok: `cad.entities.divide`'ın `block`'u (adıyla, nesneye hizalı, ölçek ve dönüş), aralığın ilk uzaklığı `start`, açık
  nesnenin uçları `ends`; elips yayı gerçek yay uzunluğuyla (`division_places`); Böl'de Blok, Nokta, Hizala, Uçlar ve İlk (ADR 0299);
  ilişkili dizi: belgenin `arrays`'i (kaynak blok “Dizi n”, ilk yer, dizi komutunun yerleşimi, yer başına öğe: yerleştirmenin kimliği,
  elle silinince boş, elle değişince `kept`; `.kcad` şema 53, `FORMATS_VERSION` 63), `cad.entities.array`'in `associative`'i; adımın
  sonunda silinen öğe boşalır, değişen korunur, yolu değişen yol dizisi izler (`follow_arrays`); veritabanı projesinde meta veri (göç
  0019, `ProjectPatch.arrays`, görüntü ve açılış `arrays_kept`'le), başkasınınki `ExternalMeta.arrays` ile (ADR 0300);
  Diziyi düzenle: `cad.arrays.edit` (yerleşim: öğe numarasını, ızgarada satır ve sütununu korur, yeni yerlere yerleştirme, kalkanlar
  silinir; ilk yer; Değiştir; Sil; Sıfırla), `array_not_found`, `invalid_item`, `item_removed`; araç `tool.arrayEdit` öğeye tıklanınca
  dizinin seçenekleriyle, Taban'da plandan önizleme; durumlarda `$array:k` (ADR 0301);
  Dizi, Kutupsal dizi ve Yol boyunca dizi'de İlişkili (İ; kapalı başlar, `Memory::array_associative`), anlamsal Patlat
  `cad.entities.explode` (parçalar çekirdekten, bloğun nesneleri kendi katman ve görünüşleriyle, ilişkili dizinin öğesi bütün diziyi
  patlatır ve dizi kalkar, `not_explodable`); uygulama katmanı `project`'in `Format`'ını kullanır (ADR 0302);
  Eğriyi düzenle: `cad.curves.edit` (çoklu çizgide Kapat, Aç, Düzleştir, Eğriye çevir; geçiş noktalı eğride nokta ekle, taşı, sil,
  uç teğetleri; her eğride denetim noktası taşı, ağırlık, sil, düğüm ekle, kapat ve aç, toleransla çoklu çizgiye çevir; çekirdeğin
  `through_tangents`'ı), araç `tool.curveEdit` (ADR 0303);
  Özellik kopyala: `cad.entities.match` (kaynağın seçilen özellikleri: katman, renk, çizgi tipi ve ölçeği, kalınlık, saydamlık, sembol;
  aynı türe yazı, ölçü, tarama ve kılavuz özellikleri), istemde on anahtar (`Memory::match_off`) (ADR 0304);
  Çizimi temizle: `cad.entities.cleanup` (tolerans, yok sayılan özellikler; boş ve yinelenen nesneler, eş doğrultulu çizgiler, yolun
  gereksiz köşeleri; rapor kayıpla), çekirdek `ops::overkill`, aracın beş seçeneği (ADR 0305);
  Denetle: `cad.drawing.audit` (nesnesi olmayan bağlı yazı, ilişkili tarama ve ölçü bağı, projede olmayan stil; `fix` tek adımda onarır,
  kilitli katmandaki söylenir; kullanılmayan katman, grup, blok ve stiller raporda, `purge` stilleri temizler), kurallar
  `kentos_domain::audit`, araç `tool.audit` (ADR 0306);
  delikli ve çok parçalı nesnelerde düzenlemeler: envanter; Kır, Buda ve Uzat çok parçalı çizgide tıklanan parçada, parçalar aynı
  nesnede kalır (`ops::parts::part_near`, `polyline_of_pieces`; Buda'nın parça başına “kesildi” bayrağı) (ADR 0307);
  Patlat'ta eğriler: daire, yay, elips, eğri ve NURBS köşeleri üstünde, eşit uzunlukta bölümlü çoklu çizgi; `density` (count, step,
  deviation; varsayılan 0,01 m sapma), `invalid_density`; araç eğri seçilince açık kalır, Sayı (S), Aralık (A), Sapma (P), önizlemede
  köşeler; CAD ve CBS (ADR 0308);
  pano komutları: `cad.entities.copy` (salt okunur kopya: nesneler çizim sırasıyla, katman yolları, bloklar, adı geçen stiller,
  kitaplık, birim, taban noktası; aralarındaki bağlar kalır) ve `.paste` (katman yoluna, olmayan yol açılır; aynı adlı blok ve stil bu
  çizimin; kilitli katman bütün yapıştırmayı reddeder, gizli ve başka birim söylenir; `asBlock` tek blok “Pano”), Taban noktasıyla
  kopyala (`tool.copyBase`), Blok olarak yapıştır (`edit.pasteBlock`); `Document::to_snapshot_v2_of`, katman ve blok yazma
  `exchange_write.rs`'te (ADR 0309);
  çizim sırası ve maske: `cad.entities.order`'ın `above` ve `under`'ı (`reference`; başvurunun öbür yanı yer açmak için aynı adımla
  kayar, başka katman `not_same_layer`), Nesnenin üstüne ve altına araçları, Yazıları öne getir ve Taramaları arkaya gönder (seçim ve
  sıra); alanın maskesi `ObjectLook.wipeout` (`{ frame }`, yalnız alanda; `.kcad` şema 54, `FORMATS_VERSION` 64): zemin renginde opak,
  kendinden önce çizilenleri örter (maskeli katmanda her maske kendi bandı), yalnız sınırından seçilir (`Store::set_masks`); Maske aracı
  (yol aracının biçimi; Çerçeve, Nesne), `cad.polygon.create`'in ve `cad.entities.set`'in `wipeout`'u, Öznitelikler'de Maske; DXF'in
  WIPEOUT'u ertelendi (ADR 0310);
  katmanlar: dondurma (`frozen`, `.kcad` şema 55: çizilmez, seçilmez, kenetlenmez, kapsama ve paftaya girmez, Hepsini göster çözmez,
  etkin katman dondurulmaz; DXF'in 70 grubunun 1. biti), `cad.layers.set`'in `frozen`, `thawAll` ve `unisolate`'i (Yalıt'ın gizledikleri
  belgede, oturumun), durumların dondurma ve yazdırması, `export` ve `import`'u (`kentos.layer-states`, düğümler yollarıyla), salt okunur
  `cad.layers.find` (AutoCAD'in `wcmatch` örüntüsü ve özellikler) ve `cad.layers.translate` (LAYTRANS, plan önizleme); Katmanlar'da
  Dondur / Çöz, kar tanesi, örüntülü arama ve Süzgeç ▾, Katmanı dondur, Bütün katmanları çöz, Katmanları çevir penceresi (ADR 0311);
  çizim standardı: standart veri olarak (katmanlar adları, görünüşleri, Yazdır'ları ve topladıkları ad örüntüleriyle, ad örüntüleri,
  yazı tipleri, yazı ve ölçü stilleri, çizgi tipleri, katmana göre), yerleşik `mimari-ofis` (`catalog/v1/standards/`), salt okunur
  `cad.standards.check` ve ayrı, planlı `cad.standards.apply` (birleştir, ekle, düzelt, stiller), Çizim standardı penceresi (CAD Yönet,
  CBS Veri › Standart); görünüşün kaynağı tek kural (`resolve_look`: kendi, blokta katmana göre ya da yerleştirmeden BYBLOCK, çizimde
  katmandan), blok parçası çizgi tipini, ölçeğini ve saydamlığını taşır, DXF'te tanımın BYBLOCK ve BYLAYER çizgi tipi ve saydamlığı;
  kâğıdın renkleri tek tablo (`print_color`, `paper`), çıktılar temayı okumaz (ADR 0312);
  çizgi tipleri: projenin LIN desenli tipleri (`lineTypes`, öğeler çizgi, boşluk, nokta, yazı ve şekil; birim kâğıt ya da çizim;
  kaynak) ve çizgi tipi ölçeği (`lineTypeScale`, LTSCALE), katmanın ve nesnenin `linePattern`'ı, `lineType` en yakın yerleşik tip
  (`.kcad` şema 56, `FORMATS_VERSION` 65); LIN okuyucu ve yazıcı `kentos_formats::lin`, `cad.linetypes.edit` (load, add, remove, scale,
  export), DXF LTYPE desenleri, `$LTSCALE` ve PLINEGEN iki yönde; desen kâğıtta mm, çizim deseni çizim ölçeğiyle, × LTSCALE × nesnenin
  ölçeği, paftada kâğıtta aynı; faz yol boyunca sürer, en çok 8 sayı; yazı ve şekil saklanır, boşluk çizilir ve söylenir; Çizgi tipleri
  penceresi (Yönet ve Veri › Standart) (ADR 0313);
  haritanın katmanları, değişikliğin etkisi ve kitaplık: paftanın haritasında katman durumu ve katman başına geçersiz kılma
  (`layerOverrides`: dondur, yazdır, renk, çizgi tipi ya da desen, kalınlık; tek kural `kentos_plot::layers::map_layers`, model değişmez,
  `cad.sheets.edit` ile), değişikliğin etkisi tablosu `CHANGE_EFFECTS` (adım, adımın dışında ya da tercih; kirlilik; parçalar ve bulutta
  gidişleri; kataloğun `effects`'i, `change_effects.rs` komutlarla sınar), `cad.library.edit` (projenin kitaplığı: oluştur, güncelle,
  kopyala, ad, taşı, sil, `.kstil` al ve ver, sembol ver; kurallar `kentos-style-library`'de), `cad.blocks.edit`'in `duplicate`,
  `export` ve `import`'u (`kentos.blocks`, `.kblok`; kanonik), salt okunur `cad.styles.impact` (izleyenler, kendi değerini koruyanlar,
  kilitliler, blok parçaları ve yerleştirmeler, durumlar, haritalar, dış başvuru 0); Değişikliğin etkisi penceresi, Bloklar panelinde
  Kopyasını oluştur, Dışa aktar… ve Blokları al… (ADR 0314);
  blokta kararlı kimlik, katman kuralı, X/Y ölçeği ve birim: blok içi yol tanım değişince parçanın yerel kimliğiyle izlenir
  (`Blocks::path_in`, `follow_path`); 0'daki (`""`) parçanın `byLayer`'ı yerleştirmenin katmanından, kendi katmanındaki parça o
  katman görünürken görünür (`Piece::gates`, deponun `Item::veiled`'ı), Blok oluştur parçaları çizildiği gibi `byLayer`'la kopyalar,
  DXF'in BYLAYER'ı `byLayer` kalır; yerleştirmenin `yScale`'i (afin bükmeyle: daire ve yay kesin elips, yolun yayları 0,1 mm; Patlat
  eğilen iç bloğu reddeder; DXF 41/42) ve tanımın `units`'i (birimli gerçek boyutlu, birimsiz çizim birimiyle; `BLOCK_RECORD` 70);
  `.kcad` şema 57, `FORMATS_VERSION` 66; anlamsal `cad.blocks.insert` (Blok ekle, Y ölçeği), `cad.blocks.edit`'in `units`'i,
  Öznitelikler'de X/Y ölçeği, Bloklar panelinde Birim ▸ (ADR 0315);
  Blok düzenleyici (`block.edit`: tanımın nesneleri yerel kimlikleriyle ayrı çizimde, “0 (blok)” katmanı, Kaydet `cad.blocks.edit`'in
  `update`'iyle tek adım, Vazgeç), `file.export.block` (WBLOCK; taban başlangıçta, aynı blok aynı bayt) ve `file.import.block`
  (`drawing`: dosya tek blok, `blocks`: adlarıyla, `replace` yeniden tanımlar; çakışan kimlik yenilenir, resim ve tablo
  `objects_left_out`); Bloklar panelinde Dosyaya yaz… ve Başka çizimden yeniden tanımla…, Dosyadan blok ekle komutla; iç içe
  bloğun bir milyon parçalık sınırı `block_too_large` bütün okuyucularda ve sunucuda (ADR 0316);
  blok öznitelikleri ve kartlar: özniteliğin seçenekleri (görünmez, sabit, doğrula, ön ayarlı, taşınabilir, çok satırlı ve kutu
  genişliği, alan şablonu `{NO}/{TUR}`), yerleştirmenin `attributePlaces`'i, bloğun kartı (kategori yolu, etiketler, sürüm; `.kcad`
  şema 58, `FORMATS_VERSION` 67); `cad.blocks.edit`'in `sync` (ATTSYNC; `attributes` ve `update` de eşitler, rapor `AttributeSync`),
  `fields`, `place` ve `card`'ı, `cad.blocks.replace` (BLOCKREPLACE, `purge`) ve salt okunur `cad.blocks.query` (`count`, `instances`,
  `schedule`, `where`); Blok öznitelikleri'nde seçenekler, Öznitelik değerleri'nde doğrulama ve çok satır, Blok kartı, Blok çizelgesi
  (`block.schedule`), Bloklar panelinde Liste/Kartlar ve kategori; DXF ATTDEF'in 70'i (ADR 0317);
  dinamik bloklar: tanımın `dynamic`'i (doğrusal, kutupsal, açı, nokta, çevir, görünürlük, tablo parametreleri; taşı, esnet, ölçekle,
  dizi, döndür, aynala eylemleri; sayı kuralı ve `@ad` ifadesi; `.kcad` şema 59, `FORMATS_VERSION` 68), yerleştirmenin `parameters`'ı
  (yalnız varsayılandan farklılar), çekirdeğin varyantı (`block_dynamic::vary`, önbellek, iç içe), `cad.blocks.parameters` (get, set,
  reset), `cad.blocks.edit`'in `dynamic`'i, Blok ekle'nin değerleri; KentOS'un 16 bloklu kitaplığı (`catalog/v1/blocks/kentos-temel.kblok`,
  `import` `name: kentos-temel`, KentOS blok kitaplığı) (ADR 0318);
  dinamik bloğun tutamaçları ve değerleri: seçili dinamik yerleştirmenin parametre tutamaçları (doğrusal, kutupsal, açı, nokta, çevir;
  çekirdeğin `Dynamic::grips`, `value_at`, `Rule::nearest`; `Store::param_grips`, `param_value`, `varied_outlines`), çekilen değer
  kuralına uyar ve `cad.blocks.parameters` ile yazılır, açık çevirmeler tutamaçları aynalar (`kentos_interaction::param_grip`);
  Öznitelikler'de Blok parametreleri; gizli parça her yerde gizli; salt okunur `cad.blocks.proxy` (yerleştirmenin çizdiği düz
  nesneler, `vary_traced`) ve `cad.blocks.freeze` (Sabit bloğa çevir); DXF'e değerli yerleştirme çizdiğinin düz bloğu “Ad V1”
  olarak, raporda söylenerek (`block_variants::plain_for_export`, `DxfWriteInput.notes`); AutoCAD'in dinamik blok verisi taşınmaz
  ve gerçek korpusla sınanmadı (ADR 0319);
  her türün ilişkili ölçüsü: ölçünün başvurularının kuralı (`refs.rule`: `curve` yarıçap, çap, kırıklı yarıçap, yay uzunluğu ve
  yaydan açının merkezi eğrisinden, yarıçapın noktası yerinden dönülmüş; `edges` kenarlardan açının köşesi iki kenarın kesişimi,
  kollar boylarıyla; `.kcad` şema 60, `FORMATS_VERSION` 69), köşeden açının yerleri, Buda'da bağlar (parçalarda aranır, kesilen uç
  yeni uca), `cad.dimensions.create`'in `continue` ve `baseline`'ı (Zincir ve Baz araçları onunla, ilk noktası temelinkini izler),
  seçili ölçünün bağ vurgusu (`kentos_interaction::dimension_bonds`) (ADR 0320);
  ölçü aralığı, kırma ve değerin yazımı: ölçünün ve ölçü stilinin `options`'ı (sığdırma, yuvarlama, sıfırlar, ayak-inç, alternatif birim,
  tolerans, kırma genişliği), ölçünün `textShift` ve `breaks`'i (`.kcad` şema 61, `FORMATS_VERSION` 70), tek yazım kuralı
  `Format::dimension_value` (`<>` ölçülen değer; `<>`'siz yazı elle), `cad.dimensions.edit` (space, break, unbreak, moveText, text,
  options) ve salt okunur `cad.dimensions.query` (measured, shown, overridden, bond); şeritte Ölçü aralığı, Ölçüyü kır, Kırmayı kaldır,
  Öznitelikler'de Ölçülen değer ve seçenekler (ADR 0321);
  merkez işareti, eksen çizgisi ve kot: ölçünün üç yeni türü (`centerMark` eğrinin `curve` kuralıyla, `centerLine` iki düz kenarın yeni
  `pair` kuralıyla, `elevation` noktası ve köşesinin kotuyla; `.kcad` şema 62, `FORMATS_VERSION` 71), `cad.dimensions.create`'in
  `centerMark`, `centerLine` ve `elevation`'ı, araçlar `kentos_interaction::dimension_marks`; `cad.styles.edit`'in `export` ve `import`'u
  (`kentos.annotation-styles`, stil pencerelerinde Dışa aktar… ve Al…); yazının ve ölçünün stilden ayrılan alanları `overrides`
  (adımın elle değiştirdikleri, `kentos_domain::style_overrides`; stil değişince korunur; `.kcad` şema 63, `FORMATS_VERSION` 72),
  Öznitelikler'de Stilden ayrılan ve Stile döndür (ADR 0322);
  ölçekli açıklamalar: yazının, ölçünün ve kılavuzun `annotative`'i (kendi ölçeği `at`, diğer ölçekler ve her birinin ayrı yeri,
  doğrultusu, ölçünün uzaklığı ve değerinin yeri; `.kcad` şema 64, `FORMATS_VERSION` 73), temsil kuralı sözleşmede (`at_scale`,
  `rebase_annotative`), paftanın haritası ölçeğindeki kopyayı çizer (`Document::at_scale`, masaüstünün pafta kipi ve başsız PDF), çizim
  ölçeği değişince ölçekli nesne yeni ölçeğe taşınır (açık göç: eskiler değişmez); `cad.annotations.scales` (make, unmake, add, remove,
  set, reset), Öznitelikler'de Ölçekli, Kendi ölçeği, Diğer ölçekler, Ayrı değerler; CAD › Açıklama › Ölçek ve Ölçekteki yeri aracı
  (`kentos_interaction::annotative`); kâğıttaki ölçünün kuralı (ölçek bir kez) (ADR 0323);
  çok satırlı yazının düzeni ve yazı tipleri: yazının `layout`'u (girinti, sol girinti, sekme durakları yükseklik cinsinden, kutulu
  yazıda sütunlar; `.kcad` şema 65, `FORMATS_VERSION` 74) ve dilimin `stack` kesri (`/` çizgili, `^` çizgisiz, `#` yan yana); çekirdeğin
  yerleşimi sekmelerde parçalara ayrılır, asılı ilk satırın sol girintisi de duraktır; Öznitelikler'de Girinti, Sol girinti, Sekmeler,
  Sütunlar, düzenleyicide Kesir ve Liste ▾ (`list_lines`); MTEXT `\p`, `^I` ve `\S` iki yönde, sütunlar KENTOS verisinde; tek yazı tipi
  çözüm kuralı (`text::fonts::resolve`: kendisi, eşdeğeri, SHX, yok), salt okunur `cad.fonts.report` ve Yazı tipleri penceresi, stilin
  yerine çizilen yazı tipi `font_substituted`; paftanın PDF'i yazının genişlik çarpanını ve yatıklığını yazar (`MapText`) (ADR 0324);
  alanlar ve yazının komutları: yazının `fields`'ı (harflerden bir aralık; kaynağı nesnenin alanı, çevresi, uzunluğu, katmanı, etiketi,
  özniteliği, yazısı ya da yeri, ya da projenin adı, ölçeği, birimi, sistemi, değişkeni; biçimi; `.kcad` şema 66, `FORMATS_VERSION` 75),
  belge kayıttan önce yeniler (`kentos_domain::text_fields`: kaynağı değişen, elle düzenlenen, döngü ve kopuk `####`), `cad.fields.edit`
  (insert, remove, format, update), `cad.texts.edit` (replace, number, combine, split; Bul ve değiştir onunla), salt okunur
  `cad.texts.check`; Alan ekle, Alanları yenile, Numarala, Birleştir, Satırlara ayır, Yazı denetimi, Öznitelikler'in Alan satırları (ADR 0325);
  çok kollu kılavuz: kılavuzun `arms`'ı (ek kollar ortak kolun başına), `blocks`'u (blok içerik: kolun ucunda sıra hâlinde duran
  yerleştirmeler), uçların `tip` bağı ve `leaderStyle`'ı, projenin `leaderStyles`'ı (`.kcad` şema 67, `FORMATS_VERSION` 76); belge
  kayıttan önce uçları ve blokları izler (`kentos_domain::leader_ties`); `cad.leaders.edit` (addArm, removeArm, align, collect,
  content, blocks, look, style, bind), `cad.styles.edit`'in `leader` türü; Kol ekle, Kol sil, Hizala, Topla, Blok içerik, Kılavuz
  stilleri penceresi, Öznitelikler'de kollar ve uç bağları; DXF'in MULTILEADER kolları ve LEADER olarak kollar (ADR 0326);
  tablonun hücreleri, bağlı çizelge ve DXF biçimleri: tablonun `styles`'ı (aralıkların hizası, kalını, rengi, dolgusu, basamağı;
  sonraki öncekini ezer), `formulas`'ı (`=` ile; A1 başvuruları ve aralıklar, TOPLA … YUVARLA Türkçe ve İngilizce adlarıyla,
  `#DEĞER!` … `#DÖNGÜ!`, satır ve sütunla kayan başvurular; `kentos_contracts::table_formula`) ve `fields`'ı (ADR 0325'in kaynakları,
  Blok sayısı `blockCount`); blok çizelgesi (`TableSource::Blocks`); belge hesaplanan tabloları kayıttan önce yazar
  (`kentos_domain::table_cells`); dosya kaynağının `path`, `digest`, gömülü `copy` ve `overrides`'ı (keep, replace), Tabloyu
  güncelle'nin raporu, Dosyaya bağla; `.kcad` şema 68, `FORMATS_VERSION` 77, paketli kaydın 25. türü; `cad.tables.edit` (edit,
  refresh, link, unlink); Tabloyu düzenle'nin ikinci araç satırı ve biçimli ızgarası; DXF dışa aktar'da Ölçüler ve Kılavuzlar
  Yerel nesne ya da Blok görünüşü ve KentOS verisini yaz (`DxfWriteInput.annotations`; ACAD_TABLE ve MULTILEADER yazılmaz)
  (ADR 0327);
  taramanın kaynak sınırı: `boundary` (halkalar: nokta, büküm, elips yayı ve NURBS kenarları; `ring`/`holes` ondan çizilir,
  elle değişen halka sınırı düşürür; `.kcad` şema 69, `FORMATS_VERSION` 78), bölgeden kapalı eğriler geri bulunur
  (`geom::hatch_boundary`), dönüşümler, Esnet ve tutamaçlar taşır, DXF HATCH'in kenarlarıyla iki yönde; çizgilerle kapanan yüzün
  bağı (`bounds`; belge yüzü aynı adımda yeniden keser, bütünüyle taşınan yüz tohumunu götürür), Tarama'nın çizgi yolunda
  İlişkili (ADR 0328);
  taramanın bölgeleri ve düzenlenmesi: `cad.entities.hatch`'in `islandStyle`'ı (Dış, İç içe: adaların içindekiler sırayla ayrı
  taramalar, Yok say), `alsoAt` (çoklu bölge) ve `gap` (çizgi işinin boşta uçları en yakın çizgiye köprülenir, `gaps_closed` sayı,
  uzunluk ve alanla; bağda `gap`, `.kcad` şema 70, `FORMATS_VERSION` 79); `cad.hatches.edit` (Sınırı yeniden oluştur: alan ya da
  çoklu çizgi, bağlı; Yeniden seç; Ada ekle ve çıkar; İlişkiyi kopar), bağı değişen tarama yeniden kesilir
  (`Document::recut_hatch`); Taramayı düzenle aracı, Tarama'nın Boşluk (G) ve Adalar'ın iç içe değeri, Öznitelikler'de Sınır
  (ADR 0329);
  tarama desenleri, başlangıç ve ölçekli tarama: projenin desenleri (`hatchPatterns`: ad, birim mm, inç ya da çizim, PAT aileleri;
  kitaplığınkinden önce), PAT okuyucu ve yazıcı `kentos_formats::pat`, `cad.hatchpatterns.edit` (yükle, ekle, sil, dışa aktar;
  ayar), Tarama desenleri penceresi; desenin `origin`'i (çizgiler ona bağlanır, dönüşümler taşır), desen spec'inin `origin`'i
  (dünya, nokta, köşe, ortak kutu), `basis` (kâğıtta ya da çizimde) ve `annotative`'i; ölçekli tarama (`HatchEntity.annotative`,
  çizim ölçeğiyle taşınır); `cad.hatches.edit`'in Deseni değiştir ve Taramaları hizala'sı; Tarama'nın Ölçek tabanı (E),
  Başlangıç (S), Ölçekli (L), Özellik al (Z), Taramayı düzenle'nin Hizala (H)'sı; `.kcad` şema 71, `FORMATS_VERSION` 80
  (ADR 0331);
  taramanın saydamlığı, sırası, boş bıraktığı yazılar ve bütçesi: `cad.entities.hatch`'in `transparency`'si ve `drawOrder`'ı
  (none, back, front, behindBoundary, aboveBoundary; katmanın içinde, sınır başka katmandaysa `order_other_layer`), `NewObject`'in
  `transparency` ve `order`'ı, Tarama'nın Saydamlık (M) ve Sıra (O); yeniden kesilen taramanın bağlı yazıları aynı adımda turlarla,
  kendi etiketi boş bırakılmaz, `cutout_gone` ve `hatch_text_cycle` uyarıları, Denetle'de `missingCutout` ve `hatchTextCycle`; komutun
  toplam 200 000 çizgilik bütçesi ve işte iptal (`Stop::Cancelled`), ekranın tonu paftayı değiştirmez (ADR 0332);
  taramanın PDF'i ve DXF'te ilişkisi: degrade paftanın PDF'inde gölgelendiricinin kuralıyla vektör bantlar (doğrusal ve silindir
  şeritler, küre dışbükey dilimler; `kentos_plot::vectors::gradient`); DXF'in ilişkili taraması kaynaklarıyla (97, 330'lar),
  tohumuyla (98) gelir: okuyucu yer tutucu kimliklerle bağlar (`import_placeholder`), içe aktarma aynı adımda gerçek kimlikleri
  yazar (`tie_placeholders`); `DxfWriteInput.uids` ile `71 1`, yolların kaynakları ve nesnelerde `{ACAD_REACTORS` yazılır
  (`writer::hatch_ties`); belgenin tarama izleyicisi kapalı çoklu çizgiyi kapalı okur; bağımsız dosyalarla gidiş-dönüş
  `dxf_hatch_cases.py` (ADR 0333);
  kısıt modeli ve çözücü: belgenin `constraints`'i (`kentos_contracts::ConstraintSet`: kalıcı kimlikli on beş kural, parçalar
  `uid` ve yerle, parametreler ifadeleriyle, `constraint_expr`; `.kcad` şema 73, `Op::Constraints` tek adım), çekirdeğin
  `ops::constraints`'i (kendi çözücümüz: ikili sayılarla kesin Jakobiyen, gruplar, kısıtların sırasıyla seyrek LDLᵀ, en az
  değişikliğin Gauss–Newton'u; `fully`, `under`, `over`, `inconsistent`, `singular`, `nonConvergent`; fazla, çelişen, tekil, kopuk
  kısıtlar, serbestlik ve serbest nesneler; çelişkide hiçbir şey yazılmaz), bağlı 50 × 50 dikdörtgen 75 ms (ADR 0334);
  kısıt komutları ve aracı: `cad.constraints.edit` (add, remove, mode, flip, solve; yön çizimden, parametre adı d1/ang1/rad1/dia1,
  değer ifadeyle ya da ölçülen, başvuru ölçü; `hold`; çelişki, tekillik, fazlalık ve ters dönme `shape_flipped` retleri) ve salt
  okunur `cad.constraints.query`; uçta teğet; Kısıt aracı (`tool.constraint`, on yedi yöntem, önizleme komutun planı) CAD › Değiştir ve
  CBS › Düzenle'nin Kısıtlar panelinde; Python `kentos.cad.constraints`, `Document.constraints()`, MCP (ADR 0335);
  parametreler ve taşınma: `cad.parameters.edit` (add, set, rename, remove; ifadeyle, birimli, aralıklı; çizim yeni değerlerle aynı
  adımda çözülür), protokolün `constraints` damgası (`ProjectPart::Constraints`), veritabanı projesinin `constraints`'i (göç 0022,
  `ProjectPatch.constraints`, görüntüde `constraints_kept`), bulut eşitlemesinde meta olarak; Python `kentos.cad.parameters` (ADR 0336);
  düzenlemelerde kısıtlar: her yazan çağrının ardından aynı adımda çözüm (oynatılan parçalar tutulur, sabit eski yerinde, çelişkide
  hiçbir şey yazılmaz; kesen komutlar çözmez, bozduklarını kaldırır), gevşetme (`relaxConstraints`, `Document::set_relax`, Python
  `with doc.relaxed():`, Gevşeterek düzenle), silinen ve yapısı değişen nesnenin kısıtları `constraints_removed` ile, Kopyala ve
  Dizi'de aralarındaki kısıtlar yeni adlı ölçülerle (`constraints_not_copied`), çözüm işin durdurmasını dinler (ADR 0337);
  kısıt işaretleri ve otorite: çizimde her kısıtın işareti (türün ikonu, ölçünün adı ve değeri, tutmayan tehlike renginde; üzerine
  gelince bağlı parçalar; `kentos_interaction::constraint_marks`), Kısıtları göster, Parametreler penceresi (parametreler, kısıtlar,
  çelişenler işaretli; `cad.parameters.edit` ve `cad.constraints.edit` ile); başka ilişkinin taşıdığı nesneye kısıt konmaz
  (`constraint_governed`, `Document::governed`); DXF'te AutoCAD'in kısıt grupları okunmaz ve KentOS'unkiler yazılmaz, raporda
  söylenir (ADR 0338);
  duvar: `wall` türü (taban çizgisi, genişlik, hiza, öteleme, stil, bileşenler, uçlar, dolgu, yükseklik, kat ve evre; `.kcad` şema
  75, `FORMATS_VERSION` 82), yüzler ve bileşen sınırları çekirdekten türer (`geom::wall`), depo onu yerleştirme gibi açar; projenin
  duvar stilleri (`settings.wallStyles`, KentOS'un beş stili kullanılınca projeye alınır, `cad.styles.edit`'in `wall` türü ve izleyen
  duvarlar) ve katları (`settings.arch`, `cad.settings.set`'in `arch`'ı); görünüş şematik, standart ya da ayrıntılı (ayardan ya da
  ölçekten; paftanın haritası kendi ölçeğiyle); `cad.walls.create`, `.convert` (Tek tek, Çiftler) ve `.edit`; Duvar ve Çizgiden duvar
  araçları, Duvar stilleri, Proje ayarları › Mimari, Öznitelikler'in duvar satırları, seçim süzgecinde Duvar; CAD'de Mimari sekmesi
  (ek kaydının `tabs`'ı); DXF'e seçilen ayrıntıda düz parçalarla; Python `kentos.cad.walls`, MCP (ADR 0339);
  duvar birleşimleri ve düzenlemeleri: birleşim saklanmaz, duvarlardan çözülür (`geom::wall_join`: köşe gönye ya da alın, düz devam, T
  bileşen önceliğiyle geçişli, X; birleşmeyen dört kodla ve onaran uç birleşimiyle), depo değişen duvarların çevresini çözer
  (`store/wall_joins.rs`); bileşenin `priority`'si, uçların `joins`'u ve `cleanup` (`.kcad` şema 76, `FORMATS_VERSION` 83);
  `cad.walls.edit`'in split, join, extend, trim, flip, replaceStyle, joins, resetJoins'i ve salt okunur `cad.walls.joins`; duvarın yüzüne
  bağlı ölçü ve odanın taraması kalınlığı aynı adımda izler (duvarın yerleri `Vertex{part: çizgi, index: yol köşesi}`); Patlat ve DXF
  birleşmiş çizgilerle ve kaybı söyleyerek; Mimari › Duvar düzenle'nin yedi aracı, Birleşimleri denetle, uyarı üçgenleri (ADR 0340);
  açıklık, kapı ve pencere: `opening` türü (duvarın kalıcı kimliği, taban çizgisi boyunca `at` ve `anchor`, genişlik, biçim, kanat,
  açılış, menteşe, yan, açı, kasa, denizlik, lento, tür, numara; pervazları `place` belgeden yazılır, `broken`: hostGone, offWall,
  overlap; `.kcad` şema 77, `FORMATS_VERSION` 84, sunucuda göç 0024), KentOS'un yedi açıklık türü; çekirdeğin `geom::opening`'i
  (pervazlar, simge, `read_back`) ve birleşim çözücüsünün boşlukları (`solve_with`), deponun `set_hosts`'u; belge açıklığı duvarıyla
  izler (`kentos_domain::opening_follow`: duvar taşınınca, kalınlaşınca yeniden yazılır, açıklığın kendisi taşınınca, döndürülünce,
  aynalanınca duvarına okunur, sessiz yeniden barındırma yok), duvarın Böl, Birleştir, Uzat, Buda ve Çevir'i açıklıklarını taşır;
  `cad.openings.create` (at, nokta, köşeden uzaklık, kenar ortası; tür) ve `.edit` (set, move, flip, rehost, replaceType, renumber,
  copy); Mimari › Açıklık: Kapı, Pencere, Boşluk, Açıklığı çevir, Açıklığı taşı, Numarala; Öznitelikler'in açıklık satırları, seçim
  süzgecinde Açıklık, duvarsız açıklığın uyarı üçgeni; Patlat ve DXF simgenin düz parçalarıyla; Python `kentos.cad.openings`, MCP
  (ADR 0341);
  mahal ve metraj: `space` türü (halkalar ve delikler `parts`, duvarlardan bulunanın tohumu `seed`, yöntem `method`: net, duvar ekseni,
  brüt; numara, ad, kullanım, kat ve evre, yükseklik, kaplamalar, `broken`: lostEnclosure; `.kcad` şema 78, `FORMATS_VERSION` 85, sunucuda
  göç 0025), projenin yöntemi `settings.arch.spaceMethod`; çekirdeğin `geom::space`'i (duvarların birleşmiş yüzleri, tohumun yüzü, yönteme
  göre kenar kenar kaydırma, alan, çevre, kutup, tek çok satırlı etiket); belge duvarı değişen mahali aynı adımda yeniden bulur
  (`kentos_domain::space_follow`), kapanmayan nedeniyle kalır; `cad.spaces.create`, `.edit` (set, merge, split, rebound) ve salt okunur
  `.quantities` (mahaller, duvarlar, açıklıklar; kat, evre, kullanım, katman; kurallarıyla); Mimari › Mahal: Mahal (Yöntem, Numara, Ad,
  Kullanım, Çiz), Metraj (pano, XLSX, ODS, CSV), Mahali böl, Mahalleri birleştir; Öznitelikler'in mahal satırları, seçim süzgecinde
  Mahal, kapanmayan mahalin uyarı üçgeni; Python `kentos.cad.spaces`, MCP (ADR 0342);
  aks, kolon, merdiven ve rampa: `grid` (iki nokta, büküm, etiket telde `name`, balonlar ve çapı), `column` (merkez, dikdörtgen ya da daire,
  en, boy, dönüş, tür, etiket, dolgu, iki aksı `grids`, konumu `place`, `broken`: gridGone), `stair` (2–4 noktalı yürüme çizgisi, genişlik,
  rıht sayısı ve yüksekliği, basamak, sahanlık, kesme, numaralar) ve `ramp` (orta çizgi, genişlik, yükseklik, eğim, başlangıç kotu,
  sahanlık) türleri (`.kcad` şema 79, `FORMATS_VERSION` 86, sunucuda göç 0026); çekirdeğin `geom::arch`'ı (aksın balonları, kolonun
  çevresi, şerit, sahanlıklar, basamaklar, ok, kotlar, denetimler, aks kesişimleri); belge akslara bağlı kolonu kesişimde tutar, konumu
  “B-3” (`kentos_domain::arch_follow`), aks silinince bağ nedeniyle kopar; `cad.grids.create` (seri), `cad.columns.create` (aks
  kesişimleri), `cad.stairs.create`, `cad.ramps.create` ve `cad.arch.edit` (set, renumber, attach, detach); merdivenin ve rampanın
  denetimleri uyarıdır (Blondel, yolun yetmesi, katın yüksekliği, eğim); Mimari › Taşıyıcı (Aks, Kolon, Aksları numarala) ve Merdiven
  (Merdiven, Rampa); Öznitelikler'in satırları ve Denetim'i, seçim süzgecinde dört tür, bağı kopan kolonun uyarı üçgeni; Python
  `kentos.cad.grids`, `.columns`, `.stairs`, `.ramps`, `.arch`, MCP (ADR 0343);
  etiketler ve çizelgeler: `tag` türü (nesnesi `of` kalıcı kimlikle, yeri, şablonu `{number}` … `{attr:Ad}`, çerçevesi, yazı yüksekliği,
  kolu; yazısı `text` ve bağ noktası `anchor` belgenin; `broken`: targetGone; `.kcad` şema 80, `FORMATS_VERSION` 87, sunucuda göç 0027),
  çekirdeğin `geom::tag`'ı (çerçeve yazının ölçülmüş genişliğiyle, satırlar, kol; yazısı gerçek boy kuralında), belge etiketi nesnesiyle
  yazar ve taşır (`kentos_domain::tag_follow`, değerler `arch_values`); tablonun `schedule` kaynağı (açıklık, mahal, kolon, duvar;
  sütunlar, süzgeç, sıralama, grup, toplam; satırların nesneleri `members`), belge yalnız okuduğu türler değişince yeniden yazar
  (`kentos_domain::schedules`); `cad.tags.create` (Hepsini etiketle `all`), `cad.tags.edit`, salt okunur `cad.tags.check` (yinelenen
  numara, türsüz, duvarı kopuk, nesnesiz, etiketsiz), `cad.schedules.create`, `cad.schedules.edit` (`cell` kaynağa yazar, hesaplanan
  `cell_computed`; `set`), `cad.tables.edit` çizelgenin hücresini reddeder (`schedule_cell`); Mimari › Etiket (Etiket, Hepsini
  etiketle, Etiketleri denetle) ve Çizelge (Kapı ve pencere çizelgesi, Mahal çizelgesi), Tabloyu düzenle'nin çizelge yolu,
  Öznitelikler'in etiket satırları, seçim süzgecinde Etiket; Python `kentos.cad.tags`, `.schedules`, MCP (ADR 0344);
  görünüm işaretleri ve mahalin kaplamaları: `viewmark` türü (kesit, görünüş, detay; noktalar, yarıçap, balonun yeri, bakış,
  numara, hedef: adlı görünüm ya da paftadaki harita çerçevesi; yazısı `words` ve `broken: targetGone` belgenin; `.kcad` şema 81,
  `FORMATS_VERSION` 88, sunucuda göç 0028), adlı görünümün kararlı kimliği (`id`, `view_key`; yeniden adlandırma korur), çekirdeğin
  `geom::viewmark`'ı (balon yazıdan, bakış oku, kalın uçlar, kesikli detay çemberi); belge işaretin yazısını hedefinden işaretin
  adımında ve adım dışında görünümler ya da pafta kitabı değişince, geri almada yerinde yazar (`kentos_domain::viewmark_follow`);
  `cad.viewmarks.create`, `.edit` (set, retarget, detach), salt okunur `.references` ve `cad.spaces.finish` (mahale bağlı döşeme deseni
  ya da tavan ızgarası, aynı yüzün eskisinin yerine; tarama izleyicisi mahali dış sınır alır); Mimari › Görünüm (Kesit, Görünüş, Detay,
  Başvurular), Mahal'de Döşeme deseni ve Tavan ızgarası, Donatı (KentOS blok kitaplığı, Blok ekle); Öznitelikler'in işaret satırları,
  seçim süzgecinde Görünüm işareti, kopuk işaretin uyarı üçgeni; Python `kentos.cad.viewmarks`, MCP (ADR 0345);
  mimari şablonu ve evreler: çalışma şablonu veri olarak (`catalog/v1/templates/mimari.json`: cm, 1:50, katlar, mimari ofis standardı,
  KentOS blok kitaplığı, duvar stilleri, A3 paftalar) ve `cad.templates.apply` (eksik parçaları, var olanı koruyarak; katmanlar,
  stiller ve bloklar tek adım); projenin `arch.phaseView`'u (Tümü, Mevcut durum, Yıkım planı, Yeni durum; `.kcad` şema 82, `FORMATS_VERSION`
  89), görünüş kuralı `PhaseView::look` (gizli, kesikli, soluk; açıklık duvarının, etiket nesnesinin evresiyle), çizimde ve paftada
  `kentos_native_style::phases`, deponun `set_phase_hidden`'ı; etiketin `phases` süzgeci; Mimari › Proje'de Mimari şablonu ve Evre (ADR 0346);
  pafta seti ve mimari DXF: salt okunur `cad.sheets.set` (paftalar adlarının numarasıyla serilerde: A-1 Planlar … Diğer; harita
  çerçevesinin zemini sabit görünümünden, `kentos_sheet::display::fixed_corners`; ona bakan işaretler; `uids` ile etkilenen
  çerçeveler: kutusu kesişen `shows`, işaretin hedefi `marked`, zemini bilinmeyen `unknown`), Pafta seti penceresi (Mimari › Görünüm,
  Paftaya git); mimari nesneler DXF'e parçalarının adsız bloğu ve nesneyi ve kalıcı kimliğini KENTOS verisinde taşıyan INSERT'le
  (`Writer::arch_whole`), geri okumada kendileri, aralarındaki bağlar yer tutucularla (`Emitter::tie_arch`, `tie_placeholders`);
  otomatik kesit ve görünüşün modeli yalnız tasarım (ADR 0347);
  dış başvuru: blok tanımının `xref`'i (mutlak ve göreli yol, Ekle ya da Bindir, SHA-256 özeti, boşaltılmış, katman grubu, getiren,
  kaynak nesnelerin kimlikleri; `.kcad` şema 83, `FORMATS_VERSION` 90), içerik çizimde son yüklenen hâliyle, kaynağın katmanları
  “Dış başvuru: ad” grubunda, Ekle türündeki iç içe dış başvurular getirenle, Bindir olanlar gelmez; `cad.xrefs.edit` (attach,
  reload, unload, repath, detach, bind; yeniden yüklemede nesne yerel kimliğini korur, bağlar onunla gider; döngü `xref_cycle`) ve
  salt okunur `cad.xrefs.list` (yüklü, boşaltılmış, bulunamadı, değişti); kayıtta göreli yol; CAD › Ekle › Dış başvuru (Dış başvuru
  ekle, Dış başvurular), açılışta değişen ve bulunamayan söylenir; kurallar `kentos_domain::xref`; Python ve MCP (ADR 0348);
  bağlı dosyalar ve arama yolları: projenin `xrefPaths`'i (`.kcad` şema 84, `FORMATS_VERSION` 91; `cad.xrefs.edit`'in `paths`'i), kaynak
  kayıtlı yolda, göreli yolda, sonra arama yollarında aranır, denenen yollar söylenir (`links::find`); salt okunur `cad.assets.check`
  (dış başvurular, bağlı resim, raster, nokta bulutu, tablo, ek ve kaynaklar: bulunduğu yer, `via`, `ok`/`changed`/`missing`/…,
  denenenler); `cad.xrefs.list`'in `affected`'i (etkilenen paftalar ve adlı görünümler); masaüstü yüklü dış başvuruların dosyalarına beş
  saniyede bir bakar, değişen kaynağı çizimin üstünde kartla söyler (Yeniden yükle, Dış başvurular), kendiliğinden yüklemez; Dış
  başvurular penceresinde Arama yolları ve Diğer bağlı dosyalar (ADR 0349);
  resim altlığı ve taşınabilir paket: `cad.images.edit` (Göm: bağlı resimlerin dosyası kitaplığa, PNG ve JPEG başlıktan; Yeniden bağla;
  kilitli katmandakiler sayılarak atlanır), resim ve rasterler arama yollarıyla aranır (`links::find_in`), bulunamayan resim çizimde
  kesikli çerçeve ve uyarı üçgeniyle, dosyası değişen resim izleyiciyle yeniden okunur (çizim hattında resmin dokusu pikselleri
  değişince yeniden yapılır); Paketle dış başvuruların kaynaklarını bayt bayt (özetleri tutar) ve `paket.json`'u (`kentos.package/1`:
  proje, kopyalar, kalanlar, yazı yüzleri; yerine başka yüzle çizilen yazı stili `left`'te) yazar (ADR 0350);
  kenetin başvurusu: salt okunur `cad.snap.find` (tür, nokta, kimlik, yer, blok yolu, sürüm; cevabı ilişkili ölçüye olduğu gibi
  verilir), yazının, bloğun ve tablonun noktası `node` yeri, Koordinat oku kenetin nesnesini ve yerini yazar (ADR 0272);
  kenet basma anında kendi yerinde ve o anki çizimde alınır, Tab'ın adayları çizimin sürümüne bağlı (ADR 0273);
  blokların içine kenet: parçalar iç içe, dönük, ölçekli ve aynalı yerleştirmede kendi nesneleri gibi, kapalı parçanın ağırlık merkezi,
  ekleme noktası en dıştakinin, karar yerleştirmenin katmanının (gizli değil, kilitli olur, kendi türleri), dış başvuru ve mimari
  nesneler gelince aynı kuralla (ADR 0274);
  seçim tek açık sorgu: salt okunur `cad.entities.select` (pencere, kesişen, çit, çokgen, daire, içeren alan, tıklama, hepsi, verilenler,
  ters çevir, benzerleri; türler ve katmanlar, dışarıda kalanların sayısı), seçim araçları, Tümünü seç, Ters çevir ve Önceki seçim
  aynı sorguyla (`entities_select::select`, `selectable::query`), yalnız gösterilenler, kilitlininkiler seçilir; Python ve MCP (ADR 0275);
  alt nesne seçimi: tıklamanın alt nesnesi (en yakın nokta kenarından önce, kenarın `along` yeri, blokta parçanın yolu;
  `ops::subref::place_near`), salt okunur `cad.entities.pick` (adaylar en belirliden, kimlik, yer, yol), seçim aracında Ctrl/⌘ + tık
  (halka, günlük, çipte her adayın alt nesnesi), kısayol penceresinde Ctrl + tık ve Çizerken grubu (ADR 0276);
  fareyle çakışmalar: seçim kutusunun köşeleri çizimin noktaları (`SelectBox::worlds`; kaydırma ve yakınlaştırma kutuyu koparmaz,
  yönünü değiştirmez), izlerin ham fare adımı `mouse` ve `pointer-conflicts` izi (ADR 0277);
  kilitli, gizli katman ve seçim süzgeci için tek kural: gizli ve süzgeçten geçmeyen hiçbir seçimde yok, seçim süzgeci nesne seçen her
  adımda (kenar seçen araçlar da), kilitliler seçilir ama nesne kümesine iş yapan komutlar (`set` ve `order` dahil) onları sayıyla atlar
  (`checks::skip_locked`), ilişki kuran düzenlemeler bütün ya da hiç; tıklamada neden söylenir (`edge::pick_said`) (ADR 0278);
  yüzey analizi: İşlemler'in Yüzey analizi kategorisinde Eğim, Bakı, Gölgeli kabartma, Renkli kabartma, Eğrilik, Pürüzlülük (TRI, TPI,
  engebe), Güneşlenme ve Eş yükselti eğrileri; DEM şerit şerit okunur, sonuç önizleme katlı karolu GeoTIFF ve kaynağın hemen üstündeki
  yeni katmanda raster ya da kotlu çoklu çizgiler (Kot, Tür); masaüstünde İşlemler'in iş parçacığında ev sahibinin dosyalarıyla, web'de iş
  başına bir çözümleme işçisinde (`raster-wasm`; sonuç gömülür ya da indirilir); `analysis.slope` ve `map.contours` araçları açar; CBS
  şeridinde Raster sekmesi (rasterler ve raster çözümleme kategorileri); çekirdek `kentos-raster`, GIS-32–36'nın da altyapısı (ADR 0231);
  interpolasyon ve yoğunluk: İşlemler'in İnterpolasyon ve Yoğunluk kategorilerinde Ters uzaklık (IDW), Doğal komşu, Spline, Kriging (hata
  yüzeyiyle), TIN'den raster, Çekirdek yoğunluğu, Çizgi yoğunluğu; noktaların ve çizgi ve alanların kotlu köşelerinden ya da alan değerinden,
  çapraz doğrulama tablosu; sonuç girdinin katmanının hemen altında; geometri çekirdeğinde kesin Delaunay (`geom::delaunay`) (ADR 0232);
  raster işlemleri: İşlemler'in Raster işlemleri ve Raster istatistiği kategorilerinde Raster hesaplayıcı (harita cebiri: rasterler
  katmanlarının adlarıyla, bantlar `@`'la; yalnız andığı rasterler açılır, sonuç ilk andığının ızgarasında; ifade diline `ln`, `log10`,
  `log`, `üstel`, trigonometri, `derece`, `radyan`), Yeniden sınıflandır, Maskeyle kırp, Mozaik, Yeniden örnekle, Bölgesel istatistik
  (alana yazar, tablo), Histogram, Komşuluk istatistiği, Hücre istatistiği; sayıların kuralı `kentos.rasterstats/1` (çift-çift
  toplamlar); rasterlerin girdisinde ifadenin alanları bantların adları (ADR 0233);
  raster ve vektör: İşlemler'in Raster ve vektör kategorisinde Rasterleştir (alanlar, çizgiler, noktalar; sabit ya da alandan değer, altı
  çakışma kuralı, sonuç girdinin katmanının altında), Rasterden alan, Rasterden çizgi (Lü–Wang'lı inceltme), Rasterden nokta (adımla, her
  hücre, tepeler ve çukurlar); Taranmış harita kategorisinde Çizgi yakala (renge göre, tıklanan çizgi ve ona bağlılar, isteğe bağlı kot),
  Alan kapat ve Eğrilere kot ver (kesen çizginin sırasıyla); vektör sonuçlar rasterin hemen üstündeki yeni katmanda (ADR 0234);
  hidroloji: İşlemler'in Hidroloji kategorisinde Çukur doldur (taşma yüksekliğine ya da en küçük eğimle; derinlik; şeritlerde paralel
  doldurma), Akış yönü (D8, ESRI ya da TauDEM kodları, düzlükler Barnes'ın gradyanlarıyla), Akış birikimi (D8, D∞, çoklu yön; hücre,
  alan, özgül havza alanı), Topografik nemlilik indisi, Döküm noktası, Noktadan havza, Havzalar (ana, alt havzalar, güzergâhı kesen
  derelerin havzaları km'leriyle) ve Dere ağı (Strahler, Shreve, uzunluk, düşü, eğim); bütün DEM bellekte, 2²⁵ hücreye kadar; sonuçlar
  DEM'in hemen üstündeki yeni katmanda (ADR 0235);
  uzaklık ve maliyet: İşlemler'in Uzaklık ve maliyet kategorisinde Uzaklık yüzeyi (nesnelerden ya da rasterin değerli hücrelerinden;
  kesin uzaklık dönüşümü; uzaklık ya da en yakın kaynak), Birikimli maliyet (8 ya da at hamleleriyle 16 komşu; köşeden bağlı engel
  geçilmez; Yükseklik modeliyle yüzey uzunluğu ve en büyük boyuna eğim; maliyet ya da en ucuz kaynak), En düşük maliyetli yol (Yol,
  Kaynak, Maliyet, Uzunluk) ve Maliyet koridoru (yüzde ya da değer eşiği); bütün raster bellekte; nesnelerden uzaklık kaynakların
  katmanının, öbürleri maliyet rasterinin yanında (ADR 0236);
  çok ölçütlü uygunluk: İşlemler'in Uygunluk analizi kategorisinde Bulanık üyelik (Doğrusal, Üslü, Gauss, Büyük, Küçük, Yakın),
  Bulanık çakıştırma (Ve, Veya, Çarpım, Toplam, Gamma), Ağırlıklı toplam, Ağırlıklı çakıştırma (ortak ölçek, yüzde etkiler, sınıf
  tabloları ve kısıt; tam sayılarla kesin), İkili karşılaştırma (AHP: ağırlıklar ve tutarlılık oranı tablosu, isteğe bağlı ağırlıklı
  toplam) ve ROC ile doğrulama (AUC, eğrinin tablosu, en iyi eşik); çok rasterli araçlar girdilerin kesişiminde; İşlemler'in iki yeni
  parametre türü: rasterlere değer ve raster çiftleri, girdinin rasterlerinden tablolar; tablo veren çalıştırmadan sonra tablo görünür
  olur (ADR 0237);
  mekânsal istatistik: İşlemler'in Mekânsal istatistik kategorisinde Ortalama ve Ortanca merkez (ağırlıklı, gruplara göre), Standart
  uzaklık, Yön dağılımı (standart sapma elipsi), En yakın komşu (oran, z, p, desen), Moran I (sabit bant, ters uzaklık, k en yakın; satır
  standartlaştırma), Sıcak nokta (Gi*: kopyalar z, p ve güven sınıfıyla, katman sınıflara göre kategorili), DBSCAN ve k-ortalamalar
  (kopyalar küme numarasıyla, katman kümelere göre kategorili); nesnenin yeri `$merkez`, komşular k-d ağacıyla; sonuçlar girdinin
  katmanının üstünde; çekirdek `ops::spatial_stats` metinleri de yazar; CBS'de Analiz › Mekânsal istatistik (ADR 0238);
  uzaktan algılama: İşlemler'in Uzaktan algılama kategorisinde Bant birleştir (ortak ızgarada, en yakın ya da çift doğrusal), Bantlara
  ayır (bant başına GeoTIFF), Spektral indis (NDVI, GNDVI, SAVI, EVI, NDWI, MNDWI, NDBI, oran, normalize fark; ölçek ve öteleme ile
  yansıma), Denetimli sınıflandırma (eğitim alanlarının Sınıf metinleri doğal sırayla; en büyük olabilirlik ya da en yakın ortalama),
  Denetimsiz sınıflandırma (örnekten k-ortalamalar), Doğruluk analizi (karışıklık matrisi, üretici ve kullanıcı doğruluğu, kappa), Değişim
  tespiti (fark, oran, normalize fark; sınıf değişiminin “neden neye” matrisi) ve Görüntü birleştirme (ağırlıklı Brovey ya da basit
  ortalama, pankromatiğin ızgarasında); iki geçişli araçlar aynı işin geçişleri; tablolar, özetler ve uyarılar raster çekirdeğinin; raster
  okuyucusunda ExtraSamples'ın 0'ı veri, alfa değil; CBS'de Raster › Uzaktan algılama (ADR 0242);
  mesh ve çok boyutlu veri: kendi NetCDF klasik okuyucu ve yazıcımız (CDF-1, 2, 5; dosyanın gereken parçaları; `vsize` işaretsiz), CF
  düzenli ızgaranın değişkeni ve dilimi Raster ekle'den, UGRID 1.0 ve 2DM ile ASCII DAT (tek UGRID dosyasına çevrilir) Mesh ekle'den
  (`mesh.add`); mesh sanal ızgarada, karoları ağdan rasterleştirilir, ağ çizgileri (`edges`); rasterin `dataset`'i (`.kcad` şema 38),
  Zaman sürgüsünü izleyen rasterler, Raster stili'nde Veri seti, Öznitelikler'de boyutlar ve Ağ; İşlemler'in Çok boyutlu veri kategorisi
  (Kesit, Zaman serisi, Mesh hesaplayıcı), raster çözümleme araçları gösterilen dilimde; çekirdek `kentos_formats::multidim` ve
  `kentos_raster::multidim`; CBS'de Raster › Raster ▾ (ADR 0243);
  açıklamaların yükseklikleri ve ölçeği: yazı, kılavuz, ölçü, tablo, Koordinat yaz, Km yaz ve İşlemler'in yazılarının kâğıt yüksekliği
  projenin ayarı (`.kcad` şema 30; Proje ayarları › Ölçek ve yazılar), ölçek ya da genel yükseklik değişince genel yükseklikteki nesneler
  tek adımda izler (Yazı yüksekliklerini uydur), Ölçek yaz… ve türün ölçekleri, görünüş Kaybolmasın / Gerçek boy / Ekranda sabit
  (`graphics.annotationSize`), ölçünün çizgilerinin rengi, kalınlığı ve tipi (Ölçü stilleri'nin Çizgiler'i, Öznitelikler, DXF), kılavuzun
  AutoCAD gibi 14 ucu ve Ok boyu, DXF'te AutoCAD'in ok blokları (ADR 0205);
  paftada haritanın ölçeği: Çizim ölçeğini al, Görünüme sığdır, Görünümden al; koordinat listesinin kaynağı kimlikleriyle
  (Seçimi al), katman ya da canlı seçim, ne verdiği bir satırla, Çizimde göster, sütun başlıkları (ADR 0206);
  alan işlemleri: birleştir, kesiştir, çıkar, böl, alana ve çizgiye çevir, içine tıklayarak alan (ADR 0065);
  topolojik temizlik: uçlar ve köşeler var olan köşede birleşir, kısa uç uzar, taşan uç budanır, yazılan toleransla, önizlemeli tek adım (ADR 0148);
  topolojik düzenleme: durum çubuğundaki Topoloji açıkken tutamaç, tutamaç menüsü ve Esnet görünen ve kilitsiz komşuların ortak köşe ve kenarlarını da tek adımda değiştirir, kart ortak köşeyi sayar, Noktalar da seçeneğiyle (ADR 0160);
  çakışma denetimi: durum çubuğundaki Çakışma açıkken yeni alan (Kapalı alan, Parsel oluştur, Dikdörtgen, Düzgün çokgen, Daire dilimi, Alan olarak çiz) kendi katmanındaki ya da seçili katmanlardaki görünen alanlarla örtüşen kısmı çıkarılarak yazılır; Bitişik alan: yalnız yeni sınır çizilir, yolun görünümdeki komşu alanlarla kapattığı bölge imleçle dolar ve tek alan olarak yazılır, komşuların içindekiler delik; Topoloji açıkken yeni alan komşularıyla köşe köşe bağlanır, aynı adımda (ADR 0162);
  kenet ekleri: Ağırlık merkezi, Uzantı, Paralel ve Karelaj türleri (karelaj aralığı doğu ve kuzey), çizilmekte olan yola kenet, ölçek aralığında kenet; durum çubuğundaki Kenet hücresinin sağ tık menüsünde türler tek tek ve karelaj aralıkları; uçta durmak uzantısını, kenarda durmak doğrultusunu alır, yazılan mesafe uzantı ve paralel boyuncadır; Katmanlar'da katmanın kendi keneti (mıknatıs, Kenet ▸; `.kcad` şema 10) (ADR 0163);
  kayıtlı ölçüler: çizgi ve yayın Kayıtlı semt, Kayıtlı uzunluk, Kayıtlı yarıçap ve Kayıtlı yay uzunluğu öznitelikleri (metin, yazıldığı
  gibi), Çizgi'ye kutupsal yazılan uzunluk ve CBS'de grad iken semt kaydedilir; Kayıtlı ölçüleri denetle (toleranslar, Uyuyor, Farklı,
  Okunamadı; pano ve CSV) ve Kayıtlı ölçüleri çizimden yaz, tek adımda (ADR 0180);
  Genel bakış ve Büyüteç: çizim alanının üstünde kartlar; Genel bakış bütün çizimin çekirdekte çizilen resmi ve görünüm çerçevesi
  (basmak taşır, sürüklemek kaydırır, tekerlek, çift tık tümü), Büyüteç imlecin altını 2–16 kat çizim hattının ikinci kamerasıyla
  gösterir, imleç yaklaşınca öbür yana geçer (ADR 0181);
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
  etiketleri yazıya çevirme: katmanların etiketleri paftanın kuralıyla seçilen ölçekte yazı olur (Ölçek, Örtüşenler de, Zemin, Katman,
  standart yazı katmanı aynı adımda); Nesneye bağlı (B) açıkken yazı nesnesini bilir (`labelOf`, `labelScale`; `.kcad` şema 18), iki belge
  onu kayıttan önce nesnesiyle tutar (taşınınca yeniden yazılır, nesne silinince silinir, elle düzenlenince bağı kopar), bağlı yazısı olan
  nesnenin etiketi çizilmez; Öznitelikler'de Bağlı nesne ve Bağı kopar (ADR 0175);
  nesne şablonları: stil kitaplığının üçüncü öğe türü (araç, katman, renk, kalınlık, sembol, öznitelik, etiket; `.kstil` sürüm 2),
  Şablonla çiz: katmanı adıyla bulunur ya da yolundaki gruplarla açılır, renk ve kalınlık şablonun olur ve araç bitince döner, nesneler
  sembolünü, özniteliklerini ve etiketini alır, istemde şablonun adı, Son komutu yinele şablonu yineler; sağ dokta Şablonlar paneli
  (kategoriler, arama, son kullanılanlar, tıklayınca çizer); Şablon düzenleyici ve Seçili nesneden şablon; grup şablonu (üyeleri: aynı
  geometri, öteleme, köşelere nokta, ağırlık merkezinde yazı; ana nesneyle tek adımda); Şablonu uygula (seçili nesnelere, `template.apply`) (ADR 0176);
  katman yönetimi ekleri: Katmanı gizle, yalıt, kilitle, etkin yap ve Yalıtımı kaldır (nesneye tıklayarak), Katmanı eşle, Katmana
  kopyala, Kopyasını oluştur, Katmanları birleştir; Katman durumları (görünürlük, isteğe bağlı kilit ve stil; projenin ayarı, `.kcad`
  şema 19; Katmanlar panelinde Katman durumları ▾), Kullanılmayanları temizle (boş katmanlar ve gruplar, kullanılmayan bloklar,
  projenin kullanılmayan sembolleri ve varlıkları), Katman listesi (pano, CSV) (ADR 0177);
  veri karşılaştırma: iki katman, grup ya da çizim konumla ya da anahtar alanla eşlenir; eklenen, silinen, geometrisi ya da
  öznitelikleri değişen nesneler, rapor (pano, CSV) ve Karşılaştırma grubunda renkli fark katmanları (ADR 0179);
  ölçü noktası: Nokta'nın Ad, Kod ve Kot'u, ad her noktada artar, aynı yerde nokta varsa Düzelt, Ekle ya da Atla, `#ad` ile adlı noktanın yeri, Köşelere nokta (ADR 0152);
  veride arama: alt panelin Arama sekmesi (Ctrl+F, `data.search`): bütün katmanlarda nesnelerin etiketi, yazısı, blok adı ve öznitelik değerleri, alanlar
  düğmelerle seçilir, `*` kalıbı, Büyük küçük harf ve Tam sözcük, kapsam (bütün katmanlar, bir katman, yalnız seçim), satırdan yakınlaş ve seç, Hepsini seç;
  `Y,X` (CAD'de `X,Y`) yazılınca Git görünümü koordinata getirir ve çizimde kaydedilmeyen bir işaret koyar, İşareti kaldır (`data.unmark`) (ADR 0178);
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
  Web özellikleri envanter üzerinden adım adım masaüstüne taşındı; ADR 0228'den beri masaüstü öndedir, web geri dönüşte yetişir. İki platformda da yalnız şerit
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
Web ve native masaüstü hedeflenir; web'e ara verildi (ADR 0228), şimdi masaüstü ve bulut. İleride görüntüleme için bir mobil
uygulama da yapılacak; kod ona hazır, modüler tutulur. Mobil ya da dar ekran arayüzü şu an iş değildir.
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
`apps/web/package.json`, `Cargo.toml` ve `rust-toolchain.toml` dosyalarıdır. Web dururken (ADR 0228) web'in komutları (`pnpm dev`,
`typecheck`, `test`, `build`, `e2e*`, `inventory`, `shots.mjs`, web'in perf betikleri) madde takımında koşmaz. Aşağıda dururlar, geri
dönüşte kullanılır. Masaüstü ve ortak komutlar aynen geçerlidir.

```bash
make                     # gruplu komut listesi; servisler: make dev/run/desktop/stop/status
pnpm install
pnpm dev                 # Vite; yerel çizim API olmadan çalışır
pnpm typecheck
pnpm test                # Vitest; gerekli WASM paketlerini kontrol eder
pnpm build               # WASM kontrolü + tsc + Vite
pnpm rust:test           # web/sunucu crate'leri: cargo test + clippy -D warnings + bağımlılık yönü
pnpm rust:test:desktop   # kentos-ui, vitrin, pafta tasarımcısı (kentos-sheet-ui) ve masaüstü: cargo test + clippy
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
python3 scripts/fixtures/offset_command_cases.py --check   # cad.entities.offset durumlarını (çizgi, çoklu çizgi, alan, yay, daire, Noktadan geç, iki yana, Kaynağı sil, hedef katman, retler) ötelemenin tanımından kesirlerle denetle (ADR 0246)
KENTOS_SHOTS_ONLY=otele-onizleme,otele-iki-yana,otele-sonuc cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Ötele'nin anlamsal komutla önizlemesi, iki yanası ve sonucu, .run/shots/arac-otele-* (ADR 0246)
python3 scripts/fixtures/point_construct_cases.py --check   # cad.points.construct durumlarını (iki nokta ortası, uzantı, paralel, izleme, kutupsal, hat üzerinde, retler) yapımların tanımından, KentOS kodu olmadan denetle (ADR 0270)
python3 scripts/fixtures/snap_find_cases.py --check   # cad.snap.find durumlarını (türler, kimlik, yer, adaylar, gizli katman, iç içe bloğun parçası, retler) kenetin kurallarından ve alt nesne başvurusunun bağımsız betiğinden denetle (ADR 0272, 0274)
python3 scripts/fixtures/snap_block_cases.py --check   # blokların içine kenetin durumlarını (iç içe, dönük, ölçekli ve aynalı yerleştirme, kesişimler, ağırlık merkezi, ekleme noktası, gizli, kilitli ve türleri sınırlı katman) tanımlardan elle yerleştirilen parçalarla, KentOS kodu olmadan denetle; durumlar fixtures/snap/v1/blocks.json (ADR 0274)
python3 scripts/fixtures/entities_select_cases.py --check   # cad.entities.select durumlarını (pencere, kesişen, çit, çokgenin üç kipi, daire, içeren alan, tıklama, hepsi, verilenler, ters çevir, benzerleri, türler ve katmanlar, kilitli ve gizli katman, retler) seçimin kurallarından kesirlerle, KentOS kodu olmadan denetle (ADR 0275)
python3 scripts/fixtures/entities_pick_cases.py --check   # cad.entities.pick durumlarını (adayların sırası, köşe kenardan önce, kenar boyunca, blok içi yol ve yer, nokta nesnesi, süzgeç, gizli ve kilitli katman, retler) seçimin kuralından ve subref_cases.py'nin place_near'ından, KentOS kodu olmadan denetle (ADR 0276)
python3 scripts/fixtures/vertex_command_cases.py --check   # cad.entities.vertex durumlarını (köşe sil, köşe ekle: ortası ya da kenar üzerindeki yer, düz kenar, yaya dönüştür, delik ve parçalar, topoloji komşuları, retler) KentOS kodu olmadan, kesirlerle ve 60 basamaklı tan(π/8) ile denetle (ADR 0279)
python3 scripts/fixtures/snap_coverage_cases.py --check   # kenet türlerinin kapsam matrisini (her tür her nesne türünde, Ekleme noktası, yayın çeyreği, tek seferlik Merkez) bağımsız başvurudan denetle; durumlar fixtures/snap/v1/coverage.json (ADR 0269)
python3 scripts/fixtures/create_command_cases.py --check   # cad.entities.create durumlarını sözleşmenin kuralından denetle (ADR 0057)
python3 scripts/fixtures/set_command_cases.py --check   # cad.entities.set durumlarını (Öznitelikler: katman, renk, sembol, öznitelik, etiket) sözleşmenin kuralından denetle (ADR 0066)
cargo test --release -p kentos-interaction --test perf -- --ignored --nocapture   # masaüstü deposu: eşitleme, kenet ve seçme süreleri (ADR 0029)
cargo test --release -p kentos-expression --test perf -- --ignored --nocapture   # ifade motoru 10⁵ ve 10⁶ nesnede (ADR 0100; web yolu apps/web/scripts/perf/expression.test.ts)
python3 scripts/fixtures/expression_geometry.py --check   # ifadelerin geometri değerlerini (alan, ağırlık merkezi, kutu; metrede ve yerel projenin cm ve mm biriminde, ADR 0165 §2) bağımsız hesapla denetle (ADR 0100)
KENTOS_SHOTS_ONLY=cad-mm-etiket cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # mm projesinde `$alan` etiketi ve Öznitelikler aynı birimde, .run/shots/arac-cad-mm-etiket-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs units); ADR 0165'in “ifadeler çizim biriminde” eki)
KENTOS_SHOTS_ONLY=cad-mm-stil-kategorili,cad-mm-stil-aralikli,cad-mm-stil-kurallar,cad-mm-stil-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # mm projesinde Katman stili: $alan'ın değerleri mm², sınıflar ve çizilen odalar aynı birimde, .run/shots/arac-cad-mm-stil-* (ADR 0165'in 13 Ekim eki)
python3 scripts/fixtures/style_units_cases.py --check   # Katman stilinin değerlerini (Değerlerden sınıfla, Sınıfla, Nesne sütunu, kural sayıları) m, cm ve mm'de kesirlerle bağımsız başvurudan denetle; durumlar fixtures/style/v1/units.json, masaüstünün okuyucusu crates/native/style/tests/all/units.rs (ADR 0165'in 13 Ekim eki)
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
KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-desktop icons::screens -- --ignored --nocapture   # küçük ikonların kenarları (web setinden ve KentOS UI'den, 14, 16, 20 px, iki tema), .run/shots/ikon-kenarlari.png (ADR 0054'ün 12 Ekim eki)
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
python3 scripts/fixtures/dxf_write_reference.py --check   # DXF yazıcısının blok, öznitelik, yazı, kılavuz, çok satırlı yazı (MTEXT), eğri boyunca yazı ve yerel projenin birimi örneklerini KentOS kodu olmadan denetle (ADR 0144, 0145, 0146, 0165, 0182, 0196)
python3 scripts/fixtures/text_cases.py --check   # yazı kurallarını (Artır, Bul ve değiştir, Okunur yap) kurallardan denetle (ADR 0145)
python3 scripts/fixtures/annotation_style_cases.py --check   # yazı ve ölçü stillerinin kurallarını, uygulanmasını, izlenmesini ve ölçü değerinin yazımını bağımsız başvurudan denetle; durumlar fixtures/text/v1/styles.json (ADR 0183)
python3 scripts/fixtures/paragraph_cases.py --check   # çok satırlı yazının satırlarını, sarmasını, kutusunu ve düzenleyicinin dilimlerini yazı tiplerinin ölçülmüş ilerlemeleriyle bağımsız başvurudan denetle; durumlar fixtures/text/v1/paragraph.json (ADR 0182)
cargo test -p kentos-desktop paragraph_editor::tests::screens -- --ignored --nocapture   # çok satırlı yazının resimleri, .run/shots/paragraf-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs paragraph); ADR 0182)
python3 scripts/fixtures/table_cases.py --check   # tablonun yerleşimini, çerçevesini, tutamaçlarını, boylarını, çizelgelerini, düzenlemelerini ve güncellemesini yazı tiplerinin ölçülmüş ilerlemeleriyle bağımsız başvurudan denetle; durumlar fixtures/table/v1/cases.json (ADR 0184)
python3 scripts/fixtures/table_file_cases.py --check   # tablo dosyası okuyucusunu (metin tabloları, XLSX'in tarihleri, 1904 sistemi ve kaçışları, ODS'nin tekrarları, örtülü hücreleri, türleri ve paragrafları, LibreOffice'in yazdıkları, retler, sınırlar) zipfile ve xml.etree'yle yazılmış kuraldan, XLSX'leri openpyxl'le ve ODS'leri LibreOffice'le çapraz denetle; durumlar fixtures/table/v1/files.json (ADR 0184 §4, 0223 §1)
python3 scripts/fixtures/coordinate_label_cases.py --check   # Koordinat yaz'ın yerlerini ve adlarını, şablonun satırlarını (yer tutucular, değeri olmayan satır, birimler, basamak) ve yerleşimi (kol, dirsek, çizgi, dört yön, Otomatik, kolsuz satırlar) kesirlerle ve yazı tiplerinin ölçülmüş ilerlemeleriyle bağımsız başvurudan denetle; durumlar fixtures/coordinate-labels/v1/cases.json (ADR 0185)
KENTOS_SHOTS_ONLY=koordinat-yaz,koordinat-koseler,koordinat-cizelge,koordinat-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Koordinat yaz'ın resimleri, .run/shots/arac-koordinat-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs coordinates); ADR 0185)
python3 scripts/fixtures/hatch_pattern_cases.py --check   # tarama desenlerinin kitaplığını, kesiklerin çizilişini, boyalarını, bölgeye kesilen çizgi ve noktalarını, taşınmasını ve taramanın bölgesini ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/hatch/v1/cases.json (ADR 0186)
python3 scripts/fixtures/selection_cases.py --check   # Seçim eklerinin kurallarını (tıklamanın adayları ve sırası, Çokgenle seç'in üç kipi ve halka sorunları kesirlerle, Benzerini seç) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/selection/v1 (ADR 0187)
KENTOS_SHOTS_ONLY=secim-cip,secim-cip-liste,secim-cokgen,secim-cokgen-sonuc,secim-benzeri,secim-suzgec,secim-suzgec-menu,secim-suzgec-coklu,secim-suzgec-serit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # seçim eklerinin resimleri, .run/shots/arac-secim-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs selecting); ADR 0187)
python3 scripts/fixtures/point_calc_cases.py --check   # nokta hesaplayıcı eklerinin hesabını (yolun uzaklıktaki noktası ve sapması: kenarlar, bükümlü yaylar, daire, yay, elips ve eğri kendi yay uzunluklarıyla; okuma, km'nin okunuşu ve gösterim kuralıyla yazılışı, eğik mesafenin yatayı, açıortay) mpmath ile 50 basamaklı bağımsız başvurudan denetle; durumlar fixtures/point-calc/v1/cases.json (ADR 0188)
KENTOS_SHOTS_ONLY=hesap-obje,hesap-km,hesap-egim,hesap-aciortay,hesap-menu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # nokta hesaplayıcı eklerinin resimleri, .run/shots/arac-hesap-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs pointcalc); ADR 0188)
python3 scripts/fixtures/stationing_cases.py --check   # Km yaz'ın istasyonlarını (katlar ve uçlar, ters yön, kapalı yol, aralığın ondalığı, sınırlar) ve nesnelerini (işaret, okunur yazı çizginin yanında, enkesit, nokta) 50 basamaklı bağımsız başvurudan denetle; durumlar fixtures/stationing/v1/cases.json (ADR 0189)
KENTOS_SHOTS_ONLY=km-yaz,km-yaz-enkesit,km-yaz-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Km yaz'ın resimleri, .run/shots/arac-km-yaz-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs stationing); ADR 0189)
python3 scripts/fixtures/centerline_cases.py --check   # Orta hat'ın eksenini (eşleşen düz ve yaylı kenarlar, ters çizilmiş kenar, örnekleme, harita koordinatları, retler) mpmath ile 50 basamaklı bağımsız başvurudan denetle; durumlar fixtures/centerline/v1/cases.json (ADR 0190)
KENTOS_SHOTS_ONLY=orta-hat,orta-hat-dere,orta-hat-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Orta hat'ın resimleri, .run/shots/arac-orta-hat-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs centerline); ADR 0190)
python3 scripts/fixtures/arrange_cases.py --check   # Hizala ve dağıt'ın kaymalarını (altı hizalama, iki dağıtma, başvurunun kenarı ve ortası, seçimin kutusu) ADR'nin işlem sırasıyla çift duyarlıkta ve kesirlerle bağımsız başvurudan denetle; durumlar fixtures/arrange/v1/cases.json (ADR 0194)
python3 scripts/fixtures/text_along_cases.py --check   # Eğri boyunca yazı'nın kurallarını (harflerin yeri ve dönüşü, kutu, kayıtlar, doğrultu, Okunur yap, aynalamalar, eğrinin parçası, Düzleştir, Doğrultuya döndür) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/text/v1/along.json (ADR 0196)
python3 scripts/fixtures/drawing_extras_cases.py --check   # Çizim eklerinin kurallarını (iki dairenin ve yayın ortak teğetleri ve sırası, tıklamaların seçtiği teğet, dördüncü köşe, menzil halkalarının yarıçapları ve ışınları) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/drawing-extras/v1/cases.json (ADR 0197)
python3 scripts/fixtures/plan_road_cases.py --check   # Plan yolu çiziminin kurallarını (yolun, taşıt yolunun ve refüjün alanları, iç köşelerin yuvarlanması ve sayıları, iki çizginin refüj olarak kapanması) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/plan-road/v1/cases.json (ADR 0198)
python3 scripts/fixtures/layer_field_cases.py --check   # katman alanlarının değer kurallarını (tek biçim, tam ve ondalık sayı, tarih, evet/hayır, değer listesi, aralık, zorunlu, alanın sorunları, verilerden tür) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/layer-fields/v1 (ADR 0199)
python3 scripts/fixtures/feature_table_cases.py --check   # Öznitelik tablosunun sıralamasını (sayılar kesin, tarihler, evet/hayır, doğal metin, boş ve uymayan sonda), aramasını ve gösterilen satırlarını bağımsız başvurudan denetle; durumlar fixtures/feature-table/v1/cases.json (ADR 0199 §4)
python3 scripts/fixtures/source_list_cases.py --check   # Kaynaklar'ın klasör listesini (desteklenen uzantılar, Shapefile'ın parçaları, gizli adlar, doğal sıra) ADR'den yazılmış başvurudan denetle; durumlar fixtures/sources/v1/cases.json (ADR 0199 §7)
cargo test -p kentos-desktop sources::tests::screens -- --ignored --nocapture; cargo test -p kentos-desktop features::tests::screens -- --ignored --nocapture   # Kaynaklar ve Öznitelikler'in alanları, .run/shots/kaynaklar-*, oznitelik-alanlari-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs sources), shots.mjs fields; KentOS projeleri gerçek sunucuyla cloud-shots.mjs --only sources-project,sources-added; ADR 0199)
python3 scripts/fixtures/spatial_query_cases.py --check   # sorguların ilişkilerini (kesirlerle; dairede kesin uzaklık), merkezlerini, sayıların kuralını (kesin toplam, yarım çifte ortalama, örneklem sapması), Özet istatistik'in tablosunu ve anahtar eşlemeyi ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/spatial-query/v1/cases.json, araçların çalıştırmaları fixtures/processing/v1/queries.json (ADR 0200)
KENTOS_SHOTS_ONLY=ozet,birlestir cargo test -p kentos-desktop processing::query_tests::screens -- --ignored --nocapture   # sorgu araçlarının pencereleri, .run/shots/islem-sorgu-* (değişken yoksa bütün durumlar; web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs queries); ADR 0200)
python3 scripts/fixtures/geoprocess_cases.py --check   # geometri işlemlerini (tamponun kapalı biçimli alanları ve iç ve dış yerleri, Kırp, örtüşmeler kesirlerle, Gruplayarak birleştir, geçerlilik sorunlarının yerleri, Onar, Sadeleştir, PROJ'la dönüştürme, paylaştırma) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/geoprocess/v1/cases.json, araçların çalıştırmaları fixtures/processing/v1/geometry.json ve geometry.kcad (ADR 0201)
KENTOS_SHOTS_ONLY=tampon,gecerlilik cargo test -p kentos-desktop processing::geometry_tests::screens -- --ignored --nocapture   # geometri araçlarının pencereleri ve iki sonucu çizimde, .run/shots/islem-geometri-* (değişken yoksa bütün durumlar; web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs geometry); ADR 0201)
cargo test --release -p kentos-geometry-core --test all geoprocess::timing -- --ignored --nocapture   # 400 parselde Tampon, Gruplayarak birleştir ve Kesişim'in süreleri (ADR 0201)
python3 scripts/fixtures/topology_rules_cases.py --check   # topoloji kurallarının bulgularını (nesneler, yer, ölçü, düzeltmeler) ve düzeltmelerin şekillerini çekirdeğin örtüşmesine dayanmayan başvurudan (eksenlere paralel alanlar hücreleriyle kesirlerde, dairelerin örtüşmesi kapalı biçimde) denetle; durumlar fixtures/topology-rules/v1/cases.json (ADR 0202)
KENTOS_SHOTS_ONLY=bulgular,duzelt cargo test -p kentos-desktop topology::tests::screens -- --ignored --nocapture   # Topoloji sekmesi, Düzelt ▾, Topoloji kuralları penceresi ve şeridin Topoloji paneli, .run/shots/topoloji-* (değişken yoksa bütün durumlar; web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs topologyrules); ADR 0202)
cargo test --release -p kentos-geometry-core --test all topology_rules::timing -- --ignored --nocapture   # 10 000 parselde katman içi kuralların süreleri (ADR 0202)
python3 scripts/fixtures/network_adjust_cases.py --check   # ağ dengelemesini (yatay ağlar ve kot ağları: koordinatlar ve kotlar, doğrulukları ve hata elipsleri, yöneltmeler, artıklar, katkı, w, m₀, model testi, retler) ADR'den, KentOS kodu olmadan mpmath ile 50 basamakta bağımsız başvurudan denetle; durumlar fixtures/network-adjust/v1/cases.json (ADR 0203)
KENTOS_SHOTS_ONLY=yatay,uyusumsuz,kot cargo test -p kentos-desktop calc::network::tests::screens -- --ignored --nocapture   # Yatay ağ ve Kot ağı dengelemesi pencereleri, .run/shots/ag-* (değişken yoksa bütün durumlar; web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs network); ADR 0203)
cargo test --release -p kentos-geometry-core --test all network_adjust::timing -- --ignored --nocapture   # 144 ve 400 noktalı ızgara ağların dengeleme süreleri (ADR 0203)
python3 scripts/fixtures/raster_cases.py --check   # raster okuyucusunun dosyalarını (GDAL ve PIL'in yazdığı 21 GeoTIFF, TIFF ve PNG), katlarını, istatistiklerini ve karoların renklerini KentOS kodu olmadan GDAL'la denetle; gölgeli kabartma ve rampa gdaldem'le; durumlar fixtures/raster/v1/cases.json (ADR 0204)
python3 scripts/fixtures/raster_georef_cases.py --check   # Raster oturt'un dönüşümlerini (GDAL'ın GCP dönüştürücüsü, Helmert ve projektif numpy'la, tersler Newton'la) ve ince plakayla yeniden örneklemeyi gdalwarp'la çapraz denetle; durumlar fixtures/raster/v1/georef.json (ADR 0204 §6)
python3 scripts/fixtures/raster_scene.py --check   # rasterlerin ortak sahnesini (fixtures/interaction/v1/rasters.kcad ve rasters/: DEM, ortofoto, taranmış pafta) KentOS kodu olmadan GDAL'la yeniden üretip karşılaştır (ADR 0204)
KENTOS_SHOTS_ONLY=raster-vadi,raster-orto,raster-dem,raster-ekle,raster-ekle-tarama,raster-stili,raster-oturt cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # rasterlerin masaüstü resimleri, .run/shots/arac-raster-* (web'inkiler ve işçinin piramit ve yeniden örnekleme sahneleri: (cd apps/web && node scripts/e2e/shots.mjs rasters), WebGPU'yla --renderer webgpu; ADR 0204)
cargo test --release -p kentos-formats --test all raster_timing -- --ignored --nocapture   # raster karolarının, ilk görüntünün, piramit geçişinin ve ince plakalı yeniden örneklemenin süreleri (ADR 0204 §11)
python3 scripts/fixtures/pointcloud_cases.py --check   # nokta bulutu okuyucusunun örnek dosyalarını (laspy ve LASzip'in yazdığı LAS 1.2–1.4, biçimler 0–8, sistemler, ek baytlar, çok parçalı LAZ; metin bulutları; bozuk dosyalar) KentOS kodu olmadan denetle; durumlar fixtures/pointcloud/v1/cases.json (ADR 0207)
python3 scripts/fixtures/pointcloud_index_cases.py --check   # COPC dizinini (kutular, yapraklar, ızgaranın hücreleri, düğümlerin noktaları) KentOS kodu olmadan yeniden kurup karşılaştır; --verify KentOS'un yazdığı COPC'yi LASzip'le nokta nokta denetler; durumlar fixtures/pointcloud/v1/index.json (ADR 0207 §4)
python3 scripts/fixtures/pointcloud_ops_cases.py --check   # nokta bulutu işlemlerini (Seyrelt, Zemin süzgeci, Yüksekliğe göre sınıfla, Bulutu kırp, alan sorgusu, Karola, Rasterleştir, Sınır çıkar, Bulutları birleştir) ADR'nin tanımlarından laspy ve numpy'la denetle; durumlar fixtures/pointcloud/v1/ops.json (ADR 0207 §7)
python3 scripts/fixtures/pointcloud_scene.py --check   # nokta bulutlarının sahnesini (fixtures/interaction/v1/pointclouds.kcad ve pointclouds/koy.laz) KentOS kodu olmadan laspy ve LASzip'le yeniden üretip karşılaştır (ADR 0207)
python3 scripts/fixtures/pointcloud_command_cases.py --check   # nokta bulutunun masaüstüne özgü komut durumlarını (fixtures/commands/v1/desktop) sözleşmenin kurallarından denetle (ADR 0207 §10)
python3 scripts/fixtures/service_tiles_cases.py --check   # harita servislerinin karolarını (görünümün ızgaradaki kutusu ve pikseli, kat, görünen karolar ve sıraları, karonun ağı, quadkey) pyproj'la, KentOS kodu olmadan denetle; durumlar fixtures/services/v1/tiles.json (ADR 0208 §3)
python3 scripts/fixtures/service_mvt_cases.py --check   # vektör karo okuyucusunu GDAL'ın MVT sürücüsünün yazdığı ve okuduğu karoyla denetle; durumlar fixtures/services/v1/mvt.json ve mvt/kizilay.pbf (ADR 0208 §9)
python3 scripts/fixtures/service_caps_cases.py --check   # WMS, WMTS ve WFS yetenek okuyucularını OWSLib'le ve PROJ'un eksen sıralarıyla denetle; durumlar fixtures/services/v1/caps.json ve caps/*.xml (ADR 0208 §5, §6, §10)
python3 scripts/fixtures/service_rules_cases.py --check   # servis katmanının, veri kaynağının ve bağlantıların kurallarını bağımsız KCAD okuyucusunun kurallarıyla denetle; durumlar fixtures/services/v1/rules.json (ADR 0208 §2)
python3 scripts/fixtures/layer_service_command_cases.py --check   # cad.layers.service durumlarını sözleşmenin kurallarından denetle (ADR 0208 §15)
KENTOS_SHOTS_ONLY=servis-osm,servis-vektor,servis-pencere,servis-wms,servis-veri,servis-oznitelik,servis-bilgi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # harita servislerinin masaüstü resimleri (ağ gerekir), .run/shots/arac-servis-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs services); ADR 0208)
cargo test --release -p kentos-desktop perf::services -- --ignored --nocapture --test-threads=1   # görünen karolar, karo ağı, PNG çözümü, şehir karosunun MVT'si ve stili, önbellekten tam görünüm ve iki altlıklı kareler; gerçek karolar bir kez .run/perf/'e (ADR 0208 §16)
cargo test -p kentos-api proxy   # kentosd'nin servis vekili: adres kuralları, başlıklar, yerel sunucuyla yönlendirme, 32 MB, POST ve iç ağ reddi (ADR 0208 §13)
python3 scripts/fixtures/network_cases.py --check   # ağ analizini (graf, konumlar, rotalar ve sıraları, engeller, hizmet alanının çizgileri kesin ve alanları shapely'yle, en yakın tesis, maliyet matrisi, dört izleme, Denetle) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/network/v1/cases.json (ADR 0209)
python3 scripts/fixtures/network_rules_cases.py --check; python3 scripts/fixtures/network_form_cases.py --check   # ağ tanımının kurallarını ve Ağlar penceresinin form kurallarını bağımsız KCAD okuyucusunun kurallarıyla denetle; durumlar fixtures/network/v1/rules.json ve form.json (ADR 0209 §2, §10)
python3 scripts/fixtures/network_define_command_cases.py --check   # cad.network.define durumlarını sözleşmenin kurallarından denetle (ADR 0209 §11)
python3 scripts/fixtures/network_processing_cases.py --check; python3 scripts/fixtures/network_scene.py --check   # İşlemler'in ağ araçlarının durumlarını (fixtures/processing/v1/network.json ve network.kcad) ve izlerin sahnesini (fixtures/interaction/v1/networks.kcad) bağımsız başvurudan denetle (ADR 0209)
cargo test --release -p kentos-geometry-core --test all network::timing -- --ignored --nocapture   # 100 800 parçalı ızgara şehirde kurma, konum, Dijkstra, imlece yol, rota, izleme, hizmet alanının çizgileri ve alanları, Denetle; bütçeleriyle (ADR 0209 §12)
cargo test --release -p kentos-processing --test cases network_timing -- --ignored --nocapture   # aynı şehrin çizimden okunması ve bütün kurma (masaüstünün arayüz iş parçacığındaki payı, İşlemler ve Python; ADR 0209 §12)
cargo test -p kentos-desktop networks::tests::screens -- --ignored --nocapture   # Ağlar penceresi, İşlemler'in ağ araçları ve sonuçları, Analiz şeridi, .run/shots/aglar-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs networks); ADR 0209)
python3 scripts/fixtures/temporal_cases.py --check   # zaman değerlerinin okunuşunu, yazılışını ve gösterimini, aşağı yuvarlamayı, sürgünün konumlarını (takvimle ay ve yıl), son konumu ve açılış adımını, nesnenin zamanını, görünme tablosunu ve katmanın özetini ADR'nin dilbilgisi ve takvimiyle, KentOS kodu olmadan denetle; durumlar fixtures/temporal/v1/cases.json (ADR 0210 §3–§5)
python3 scripts/fixtures/temporal_rules_cases.py --check; python3 scripts/fixtures/temporal_command_cases.py --check   # zaman ayarının ve senaryo ağacının kurallarını (fixtures/temporal/v1/rules.json) ve cad.layers.time ile cad.scenarios.edit durumlarını sözleşmenin kurallarından denetle (ADR 0210 §2, §11)
python3 scripts/fixtures/temporal_scene.py --check   # zaman izlerinin ve resimlerin sahnesini (fixtures/interaction/v1/temporal.kcad) denetle (ADR 0210)
cargo test --release -p kentos-geometry-core --test all time::timing -- --ignored --nocapture   # 100 000 değerin zamanı, deponun zamanları alması, pencerenin süzgeci ve sürgünün konumları (ADR 0210 §12)
cargo test --release -p kentos-desktop style::perf::temporal -- --ignored --nocapture --test-threads=1   # 100 000 parsellik zamansal katmanda sürgünün bir adımı (zaman sırası ve karışık yıllar) ve Senaryo oluştur'un kopyası (ADR 0210 §12)
KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; TIME_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/time.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp   # web'de zamanların okunması ve 10 000 nesnede sürgünün bir adımı, gönderilen WASM'la (ADR 0210 §12)
KENTOS_SHOTS_ONLY=zaman-serit,zaman-surgu,zaman-surgu-2015,zaman-ayarlari,senaryo-olustur,senaryo-gosterilen,senaryo-uygula,zaman-karsilastir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # zaman ve senaryoların masaüstü resimleri, .run/shots/arac-zaman-*, arac-senaryo-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs temporal); ADR 0210)
KENTOS_TRACES_ONLY=time-slider,scenarios cargo test -p kentos-desktop every_trace_passes   # masaüstünde yalnız adı verilen izler
python3 scripts/fixtures/terrain_cases.py --check   # yüzey analizinin (eğim, bakı, gölgeli ve renkli kabartma, eğrilik, pürüzlülük, güneşlenme) durumlarını numpy ve GDAL'la, KentOS kodu olmadan denetle; tanımlar gdaldem'le çapraz denetlenir; durumlar ve DEM'ler fixtures/terrain/v1 (ADR 0231)
python3 scripts/fixtures/contour_cases.py --check   # eş yükselti eğrilerini (kareler, eyer, zincirleme, Douglas-Peucker, Kot yazısı) KentOS kodu olmadan denetle; gdal_contour'la çapraz denetim; durumlar fixtures/contours/v1 (ADR 0231 §9)
python3 scripts/fixtures/surface_processing_cases.py --check   # yüzey araçlarının İşlemler durumlarını (adlar, özetler, katmanlar, nesneler; yazılan dosyanın değerleri ve eğriler yüzey ve eğri başvurularına bağlı) denetle; durumlar fixtures/processing/v1/surface.json ve surface.kcad (ADR 0231 §10)
cargo test --release -p kentos-raster --test all timing -- --ignored --nocapture --test-threads=1   # yüzey analizinin süreleri 4096² DEM'de, 8 ve 1 iş parçacığıyla (ADR 0231 §11)
(cd apps/web && node ../../scripts/wasm/ensure.mjs --release && node scripts/perf/raster.mjs)   # aynı işler tarayıcının çözümleme işçisinde, release WASM'la; DEM'leri GDAL bir kez .run/perf'e yazar (ADR 0231 §11)
KENTOS_SHOTS_ONLY=yuzey-serit,yuzey-egim,yuzey-egim-cizim,yuzey-esyukselti-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # yüzey analizinin resimleri, .run/shots/arac-yuzey-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs surface); ADR 0231)
python3 scripts/fixtures/delaunay_cases.py --check   # Delaunay üçgenlemesini qhull'un (matplotlib) üçgenleriyle ve Python'un kesirleriyle tam boş çember denetiminden geçmiş başvuruyla, dejenere kümelerde sayılarla denetle; durumlar fixtures/delaunay/v1/cases.json (ADR 0232 §4)
python3 scripts/fixtures/interpolation_cases.py --check   # interpolasyonu ve yoğunluğu (noktaların toplanması, ızgara, IDW, TIN, Doğal komşu kesirli Voronoi'yle, Spline ve Kriging 40 basamaklı mpmath'le, variogram uydurması, çekirdek ve çizgi yoğunluğu, çapraz doğrulama) KentOS kodu olmadan denetle; IDW ve TIN GDAL'la çapraz denetlenir; birkaç dakika sürer; durumlar fixtures/interpolation/v1/cases.json (ADR 0232)
python3 scripts/fixtures/bessel_k0.py --check   # Spline'ın K₀'ının Chebyshev katsayılarını mpmath'ten yeniden üretip çekirdekteki kopyayla karşılaştır (ADR 0232 §8)
python3 scripts/fixtures/interpolation_processing_cases.py --check   # interpolasyon ve yoğunluğun İşlemler durumlarını denetle; yazılan dosyalar interpolasyon başvurusuna bağlı; durumlar fixtures/processing/v1/interpolation.json ve interpolation.kcad (ADR 0232 §13)
cargo test --release -p kentos-raster --test all interpolation_timing -- --ignored --nocapture --test-threads=1   # 100 000 noktadan 2048² ızgarada yedi işin ve 10⁶ noktanın Delaunay'ının süreleri (ADR 0232 §14)
KENTOS_SHOTS_ONLY=interp-serit,interp-idw-cizim,interp-capraz,yogunluk-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # interpolasyon ve yoğunluğun resimleri, .run/shots/arac-interp-*, arac-yogunluk-*, arac-cizgi-yogunlugu-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs interpolation); ADR 0232)
python3 scripts/fixtures/raster_ops_cases.py --check   # raster işlemlerini (hesaplayıcı, sınıflandırma, maskeyle kırpma, mozaik, yeniden örnekleme, bölgesel istatistik, histogram, komşuluk ve hücre istatistiği) ADR'den, KentOS kodu olmadan kesirler ve mpmath'le denetle; yeniden örnekleme gdalwarp'la çapraz denetlenir; durumlar fixtures/raster-ops/v1/cases.json (ADR 0233)
python3 scripts/fixtures/raster_ops_processing_cases.py --check   # raster işlemlerinin İşlemler durumlarını ve girdinin adlarını denetle; rasterleri GDAL yazar, yazılan dosyalar başvuruya bağlı; durumlar fixtures/processing/v1/raster-ops.json, raster-ops.kcad ve raster-ops/ (ADR 0233)
cargo test --release -p kentos-raster --test all ops_timing -- --ignored --nocapture --test-threads=1   # 4096² rasterlerde dokuz aracın on iki işinin süreleri, 10 000 parselle kırpma ve bölgesel istatistik (ADR 0233 §15)
KENTOS_SHOTS_ONLY=ops-serit,ops-hesap,ops-hesap-cizim,ops-bolge cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # raster işlemlerinin resimleri, .run/shots/arac-ops-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs rasterops); ADR 0233)
python3 scripts/fixtures/raster_vector_cases.py --check   # raster ve vektör araçlarını (yakma, bölgelerin halkaları, inceltme ve yollar, noktalar, Çizgi yakala, Alan kapat, Eğrilere kot ver) ADR'den, KentOS kodu olmadan denetle; GDAL'ın Polygonize ve RasterizeLayer'ıyla çapraz denetim; durumlar fixtures/raster-vector/v1/cases.json (ADR 0234)
python3 scripts/fixtures/raster_vector_processing_cases.py --check   # raster ve vektör araçlarının İşlemler durumlarını denetle; rasterleri GDAL yazar; durumlar fixtures/processing/v1/raster-vector.json, raster-vector.kcad ve raster-vector/ (ADR 0234)
python3 scripts/fixtures/scanned_scene.py --check   # taranmış paftanın sahnesini (fixtures/interaction/v1/scanned.kcad ve scanned/pafta.tif) GDAL'la yeniden üretip karşılaştır (ADR 0234)
cargo test --release -p kentos-raster --test all vector_timing -- --ignored --nocapture --test-threads=1   # 4096² ve 8192² rasterlerde yedi aracın süreleri; KENTOS_PHASES=1 Çizgi yakala'nın aşamalarını da yazar (ADR 0234 §11)
KENTOS_SHOTS_ONLY=vek-serit,vek-yakala-cizim,vek-kapat-cizim,vek-kot cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # raster ve vektör araçlarının resimleri, .run/shots/arac-vek-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs rastervector); ADR 0234)
python3 scripts/fixtures/hydrology_cases.py --check   # hidrolojinin sekiz aracını (doldurma, D8 ve düzlükler, D8, çoklu yön ve D∞ birikimi, TWI, döküm noktası, havzalar, dere ağı ve sıraları) ADR'den, KentOS kodu olmadan denetle; GRASS GIS'in r.terraflow, r.watershed ve r.water.outlet'iyle çapraz denetim (grass yoksa atlanır); durumlar fixtures/hydrology/v1/cases.json (ADR 0235)
python3 scripts/fixtures/hydrology_processing_cases.py --check   # hidroloji araçlarının İşlemler durumlarını denetle; rasterleri GDAL yazar, yazılan dosyalar başvuruya bağlı; durumlar fixtures/processing/v1/hydrology.json, hydrology.kcad ve hydrology/ (ADR 0235)
python3 scripts/fixtures/hydrology_scene.py --check   # hidroloji resimlerinin çizimini (fixtures/interaction/v1/hydrology.kcad: vadinin DEM'i, yol ekseni, çıkış noktaları) denetle (ADR 0235)
cargo test --release -p kentos-raster --test all hydro_timing -- --ignored --nocapture --test-threads=1   # 4096² DEM'de sekiz aracın dokuz işinin süreleri; KENTOS_PHASES=1 birikimin aşamalarını da yazar (ADR 0235 §12; web'inkiler: (cd apps/web && node scripts/perf/raster.mjs --only hydro))
python3 scripts/fixtures/distance_cases.py --check   # uzaklık ve maliyetin dört aracını (kesin düz uzaklık kaba kuvvetle, birikimli maliyet önceliğiyle Dijkstra'yla, geldiği komşu, yollar, koridor) ADR'den, KentOS kodu olmadan denetle; GRASS GIS'in r.cost ve r.grow.distance'ı ve GDAL'ın Proximity'siyle çapraz denetim (grass yoksa atlanır); durumlar fixtures/distance/v1/cases.json (ADR 0236)
python3 scripts/fixtures/distance_processing_cases.py --check   # uzaklık ve maliyet araçlarının İşlemler durumlarını denetle; rasterleri GDAL yazar, yazılan dosyalar başvuruya bağlı; durumlar fixtures/processing/v1/distance.json, distance.kcad ve distance/ (ADR 0236)
python3 scripts/fixtures/distance_scene.py --check   # uzaklık ve maliyet resimlerinin çizimini ve maliyet rasterini (fixtures/interaction/v1/distance.kcad, distance/maliyet.tif: vadinin DEM'i, eğimden maliyet, göl, köyler, yol) denetle (ADR 0236)
cargo test --release -p kentos-raster --test all distance_timing -- --ignored --nocapture --test-threads=1   # 4096² maliyet rasterinde dört aracın dokuz işinin süreleri; KENTOS_PHASES=1 aşamaları da yazar (ADR 0236 §8; web'inkiler: (cd apps/web && node scripts/perf/raster.mjs --only distance))
KENTOS_SHOTS_ONLY=uzk-serit,uzk-yol-cizim,uzk-koridor-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # uzaklık ve maliyetin resimleri, .run/shots/arac-uzk-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs distance); ADR 0236)
python3 scripts/fixtures/suitability_cases.py --check   # uygunluk analizinin altı aracını (üyelik işlevleri, bulanık işleçler, ağırlıklı toplam, tam sayılı ağırlıklı çakıştırma ve sınıf tabloları, AHP'nin özvektörü 50 basamakta, ROC'un AUC'si her çiftle kaba kuvvetle) ADR'den, KentOS kodu olmadan mpmath ve kesirlerle denetle; durumlar fixtures/suitability/v1/cases.json (ADR 0237)
python3 scripts/fixtures/suitability_processing_cases.py --check   # uygunluk araçlarının İşlemler durumlarını denetle; rasterleri GDAL yazar, yazılan dosyalar başvuruya bağlı; durumlar fixtures/processing/v1/suitability.json, suitability.kcad ve suitability/ (ADR 0237)
python3 scripts/fixtures/suitability_scene.py --check   # uygunluk resimlerinin çizimini ve ölçütlerini (fixtures/interaction/v1/suitability.kcad, suitability/: vadinin eğimi, yola uzaklık, arazi örtüsü, heyelan noktaları) denetle (ADR 0237)
cargo test --release -p kentos-raster --test all suitability_timing -- --ignored --nocapture --test-threads=1   # 4096² dört ölçütte altı aracın süreleri ve 15 ölçütlü AHP (ADR 0237 §11; web'inkiler: (cd apps/web && node scripts/perf/raster.mjs --only suitability))
KENTOS_SHOTS_ONLY=uyg-serit,uyg-cakistirma,uyg-cakistirma-cizim,uyg-ahp,uyg-roc cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # uygunluk analizinin resimleri, .run/shots/arac-uyg-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs suitability); ADR 0237)
python3 scripts/fixtures/spatial_stats_cases.py --check   # mekânsal istatistiğin dokuz aracını (yerler, merkezler, ortanca, standart uzaklık, elips, en yakın komşu, komşuluklar, Moran I, Gi*, DBSCAN, k-ortalamalar ve yazılan metinler) ADR'den, KentOS kodu olmadan kesirler ve 40 basamaklı mpmath'le denetle; durumlar fixtures/spatial-stats/v1/cases.json (ADR 0238)
python3 scripts/fixtures/spatial_stats_processing_cases.py --check; python3 scripts/fixtures/spatial_stats_scene.py --check   # mekânsal istatistik araçlarının İşlemler durumlarını (fixtures/processing/v1/spatial-stats.json, spatial-stats.kcad, spatial-stats-geo.kcad) ve resimlerin sahnesini (fixtures/interaction/v1/spatial-stats.kcad) denetle (ADR 0238)
cargo test --release -p kentos-geometry-core --test all spatial_stats_timing -- --ignored --nocapture --test-threads=1   # 100 000 noktada dokuz aracın süreleri (ADR 0238 §12; web'inkiler: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; STATS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/spatialStats.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp)
KENTOS_SHOTS_ONLY=ist-serit,ist-elips,ist-elips-cizim,ist-moran,ist-komsu,ist-sicak-cizim,ist-dbscan-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # mekânsal istatistiğin resimleri, .run/shots/arac-ist-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs stats); ADR 0238)
python3 scripts/fixtures/remote_cases.py --check   # uzaktan algılamanın sekiz aracını (bantlar, indisler, iki sınıflandırıcı mpmath'le, k-ortalamalar, karışıklık matrisi ve kappa, değişim, Brovey ve basit ortalama) ADR'den, KentOS kodu olmadan denetle; Brovey GDAL'ın VRTPansharpenedDataset'iyle bire bir çapraz denetlenir; durumlar fixtures/remote/v1/cases.json (ADR 0242)
python3 scripts/fixtures/remote_processing_cases.py --check; python3 scripts/fixtures/remote_scene.py --check   # uzaktan algılama araçlarının İşlemler durumlarını (fixtures/processing/v1/remote.json, remote.kcad, remote/) ve resimlerin sahnesini (fixtures/interaction/v1/remote.kcad ve remote/: vadinin dört bantlı uydu görüntüsü, iki yıl sonrası, çok bantlı ve pankromatik çifti) denetle (ADR 0242)
cargo test --release -p kentos-raster --test all remote_timing -- --ignored --nocapture --test-threads=1   # 4096² dört bantlı 16 bit görüntüde sekiz aracın süreleri (ADR 0242 §12; web'inkiler: (cd apps/web && node ../../scripts/wasm/ensure.mjs --release && node scripts/perf/raster.mjs --only remote))
KENTOS_SHOTS_ONLY=ua-serit,ua-indis,ua-indis-cizim,ua-denetimli,ua-denetimli-cizim,ua-dogruluk,ua-degisim-cizim,ua-birlestirme-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # uzaktan algılamanın resimleri, .run/shots/arac-ua-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs remote); ADR 0242)
python3 scripts/fixtures/multidim_cases.py --check   # mesh ve çok boyutlu veriyi (NetCDF klasik okuma ve yazma, CF ızgaraları ve zamanı, UGRID, 2DM ve DAT'ın UGRID'e çevrilmesi bayt bayt, rasterleştirme kesirlerle, Kesit, Zaman serisi, Mesh hesaplayıcı, 2,9 ve 4,8 GiB'lık başlıklar) KentOS kodu ve libnetcdf olmadan kendi NetCDF yazıcısıyla (netcdf_classic.py) denetle; GDAL'ın netCDF sürücüsü ve QGIS'in MDAL'ıyla çapraz denetim (yoksa atlanır); durumlar fixtures/multidim/v1 (ADR 0243)
python3 scripts/fixtures/multidim_processing_cases.py --check; python3 scripts/fixtures/multidim_scene.py --check   # Çok boyutlu veri araçlarının İşlemler durumlarını (fixtures/processing/v1/multidim.json, multidim.kcad, multidim/) ve resimlerin sahnesini (fixtures/interaction/v1/multidim.kcad ve multidim/: vadinin saatlik yağışı NetCDF ızgarası, derenin taşkını UGRID mesh'i, kesitler ve istasyonlar) denetle (ADR 0243)
cargo test --release -p kentos-raster --test all multidim_timing -- --ignored --nocapture --test-threads=1   # 1 000 değişkenli başlık, bir milyon yüzlü mesh'in açılışı ve karoları, zaman adımının ilk görünümü, 2DM ve DAT, Kesit, 3 GB'lık ızgarada Zaman serisi ve Mesh hesaplayıcı; KENTOS_PERF_FILES=klasör dosyaları web için de yazar (ADR 0243 §12; web'inkiler: (cd apps/web && node ../../scripts/wasm/ensure.mjs --release && node scripts/perf/raster.mjs --only multidim))
KENTOS_SHOTS_ONLY=md-serit,md-ekle,md-mesh-ekle,md-zaman,md-stil,md-kesit,md-seri,md-hesap-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # mesh ve çok boyutlu verinin resimleri, .run/shots/arac-md-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs multidim); ADR 0243)
KENTOS_SHOTS_ONLY=hid-serit,hid-dere-cizim,hid-havza-cizim,hid-guzergah-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # hidrolojinin resimleri, .run/shots/arac-hid-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs hydrology); ADR 0235)
KENTOS_SHOTS_ONLY=bulut-koy,bulut-siniflar,bulut-yukseklik,bulut-ekle,bulut-stili,bulut-xyz,bulut-zemin-penceresi,bulut-zemin-sonucu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # nokta bulutunun masaüstü resimleri, .run/shots/arac-bulut-* (ADR 0207)
cargo test --release -p kentos-desktop perf::clouds -- --ignored --nocapture --test-threads=1   # sentetik 4 milyon noktalı bulutta dizin, ilk görüntü, düğüm çözme, tam okuma, işlemler ve kareler (ADR 0207 §12; KENTOS_PERF_POINTS)
cargo test --release -p kentos-pointcloud --test all timing -- --ignored --nocapture   # aynı bulutta düğümün görünüşe göre çözülmesi (katman katman) ve LAZ yazma, tek ve dört iş parçacığıyla (önce perf::clouds dosyayı yazar; ADR 0207 §12)
python3 scripts/fixtures/layer_filter_command_cases.py --check   # cad.layers.filter durumlarını (süzgecin kuralları, derlenmeyen koşul, $sıra ve $ölçek, grup ve servis katmanı, kilitli katman, sayılar) sözleşmenin kurallarından denetle (ADR 0211 §5)
python3 scripts/fixtures/layer_tree_command_cases.py --check; python3 scripts/fixtures/style_edit_command_cases.py --check; python3 scripts/fixtures/settings_set_command_cases.py --check   # katman ağacının (cad.layers.edit, .set, .states), stillerin (cad.styles.edit) ve proje ayarlarının (cad.settings.set) durumlarını sözleşmenin kurallarından, KentOS kodu olmadan denetle; durumlar fixtures/commands/v1 (ADR 0248)
cargo test -p kentos-desktop layering::screens -- --ignored --nocapture   # Katmanlar paneli, komutlarla yazılan katman ve Katman durumları, .run/shots/katman-secimi-* (komutlar, durumlar; ADR 0248)
cargo test --release -p kentos-interaction --test perf layer_filter -- --ignored --nocapture   # 100 000 parselde katman süzgecinin depo ve çekirdek süreleri (öznitelik ve geometri ifadesi, nesne listesi) ve bir özniteliğin değişmesi (ADR 0211 §6)
KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; FILTER_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/filter.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp   # web'de katman süzgecinin depo ve değerlendirme süreleri, gönderilen WASM'la (ADR 0211 §6)
python3 scripts/fixtures/exchange_cases.py --check   # Çizimler arası alışverişin kurallarını (seçimin çizimi: budanan ağaç, iç içe bloklar, kitaplığın kullanılanları, düşen bağlar; Başka çizimden al: yollar, katlanan adlar, Atla ve Değiştir, kimliklerin ekleri, katman durumlarının yolları; Dosyadan blok ekle: ad sayısı, sol alt köşe, resim ve tablo; Kaynaklar'ın Katman olarak ekle'si: katman nesneleriyle, aynı adlı blok ve stil, açılan katmanın görünüşünün simgesi; senaryo bağları gelmez, zaman ayarı gelir; süzgecin koşulu gelir, nesne listesi gelmez) KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/exchange/v1/cases.json (ADR 0193, 0199 §7, 0210 §9, 0211 §1)
python3 scripts/fixtures/image_cases.py --check   # Resim ekle'nin çerçevesini (genişlik, yükseklik, dönüş) ve Resmi kırp'ın sınırını resmin kendi kesirleriyle (taşan, saran, saat yönünde, aynalı, dönük, dışarıda) kesirlerle bağımsız başvurudan denetle; durumlar fixtures/image/v1/cases.json (ADR 0192)
KENTOS_SHOTS_ONLY=resim-ekle,resim-ekle-yazildi,resim-kirp,resim-kirpildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # resim nesnesinin resimleri, .run/shots/arac-resim-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs images), WebGPU'yla --renderer webgpu; ADR 0192)
python3 scripts/fixtures/edge_shift_cases.py --check   # Paralel kaydır'ın kenarlarını (dışarı ve içeri, saat yönü, delik, çoklu çizginin serbest ucu, harita koordinatları, irrasyonel boy, halkadaki yay, retler) ve hedef alanın uzaklığını kesirlerle ve 50 basamaklı mpmath ile bağımsız başvurudan denetle; durumlar fixtures/edge-shift/v1/cases.json (ADR 0191)
KENTOS_SHOTS_ONLY=paralel-kaydir,paralel-kaydir-alan,paralel-kaydir-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Paralel kaydır'ın resimleri, .run/shots/arac-paralel-kaydir-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs edgeshift); ADR 0191)
python3 scripts/ui/hatch_icons.py --check   # desenlerin ikonlarını ve Desen menülerinin geniş örneklerini (apps/web/src/ui/hatchIcons.ts) desenlerin tanımından yeniden üretip karşılaştır; değişince --check'siz yazar (ADR 0186 §11)
KENTOS_SHOTS_ONLY=tarama-araci,tarama-desenler,tarama-coklu,tarama-iliskili,tarama-oznitelikler,tarama-desen-menusu,tarama-oznitelikler-desen cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # tarama eklerinin resimleri, .run/shots/arac-tarama-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs hatches); ADR 0186)
python3 scripts/fixtures/leader_cases.py --check   # kılavuzun yerleşimini (ok başı, kol, not) kuraldan denetle (ADR 0146)
python3 scripts/fixtures/annotation_scale_cases.py --check   # açıklamaların yüksekliklerini ve ölçek ya da genel yükseklik değişince izlemeyi bağımsız başvurudan denetle; durumlar fixtures/text/v1/scale.json (ADR 0205)
python3 scripts/fixtures/dimension_ext_calls.py --check   # TypeScript'ten kaydedilmiş ölçü çağrılarının uzatma çizgilerini (ext) kayıtlı çizgilerinden denetle (ADR 0205 §6)
python3 scripts/ui/arrow_icons.py --check   # kılavuzun ok uçlarının simgelerini (apps/web/src/ui/arrowIcons.ts) uçların tanımından yeniden üretip karşılaştır (ADR 0205 §7)
python3 scripts/fixtures/annotation_scene.py --check   # ADR 0205 resimlerinin sahnesini (fixtures/interaction/v1/annotations.kcad) denetle
cargo test -p kentos-desktop labels::annotation_screens -- --ignored --nocapture   # açıklamaların resimleri, .run/shots/aciklama-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs annotations); ADR 0205)
python3 scripts/fixtures/dimension_cases.py --check   # yeni ölçü türlerinin yerleşimini kurallardan denetle (ADR 0147)
python3 scripts/fixtures/quick_dimension_cases.py --check   # Hızlı ölçü'nün ölçülerini (taraf, uzaklık, ortak kenar) kurallardan denetle (ADR 0147 §7)
python3 scripts/fixtures/numeric_display.py --check   # gösterim kuralının durumlarını (yarımlar, gürültü, işaret, taşma) kuraldan denetle (ADR 0149)
python3 scripts/fixtures/measure_cases.py --check   # ölçülerin kesin değerlerini (uzunluk, alan, yay, elips, eğri, semt, açı, ölçü türleri) 50 basamaklı bağımsız başvuruyla denetle (ADR 0149)
python3 scripts/fixtures/topology_cases.py --check   # Topolojik temizliğin durumlarını (birleştirme, uzatma, budama, kenara taşıma, kot) kurallardan denetle (ADR 0148)
python3 scripts/fixtures/polygonize_cases.py --check   # Toplu alan'ın durumlarını (bölgeler, adalar, etiketler, var olan alan, boşta uçlar) kesin kesirlerle kurallardan denetle (ADR 0151)
python3 scripts/fixtures/vertex_points_cases.py --check   # Köşelere nokta'nın durumlarını (paylaşılan köşe, var olan nokta, kot, ad artımı) kurallardan denetle (ADR 0152)
python3 scripts/fixtures/label_engine_cases.py --check   # etiket motorunun yerleşimini (kartografik sıra, yinelenen ve kıvrık çizgi, yokuş yukarı eş yükselti, parsel ve sığdırma, engeller, öncelik, yinelenenler, iğneler, çağrı çizgisi, tıklama) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/labels/v1/cases.json (ADR 0212)
python3 scripts/fixtures/label_command_cases.py --check; python3 scripts/fixtures/label_engine_scene.py --check   # cad.layers.labels ve cad.labels.pin durumlarını sözleşmenin kurallarından, etiket motorunun sahnesini (fixtures/interaction/v1/label-engine.kcad) denetle (ADR 0212)
KENTOS_SHOTS_ONLY=etiket-sahne,etiket-yakin,etiket-pencere,etiket-pencere-yerlesim,etiket-pencere-bicim,etiket-pencere-sigdirma,etiket-pencere-oncelik,etiket-pencere-engel,etiket-tasi,etiket-dondur,etiket-gizle,etiket-yerlesmeyen,etiket-sabit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # etiket motorunun masaüstü resimleri, .run/shots/arac-etiket-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs labels); ADR 0212)
cargo test --release -p kentos-geometry-core --test all label_engine::timing -- --ignored --nocapture   # etiket motorunun süreleri: 2 000 nokta adı (40 px ve yoğun), 10 000 parsel, 25 tepenin 500 eş yükseltisi, 100 000 parsellik genel bakış (ADR 0212 §6)
python3 scripts/fixtures/renderer_cases.py --check   # ek işleyicilerin kurallarını (rampa, pay ve 256 basamak, boy, sınıf, nokta yoğunluğunun noktaları, grafik, gruplar, yayma, ısı haritası, ters alanın bölgesi) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; noktalar, gruplar ve ısı değerleri bit bit, ters alan shapely'yle; durumlar fixtures/renderers/v1/cases.json (ADR 0213)
python3 scripts/fixtures/renderer_command_cases.py --check; python3 scripts/fixtures/renderer_scene.py --check   # cad.layers.renderer durumlarını sözleşmenin kurallarından, ek işleyicilerin sahnesini (fixtures/interaction/v1/renderers.kcad) denetle (ADR 0213)
KENTOS_SHOTS_ONLY=isleyici-sahne,isleyici-isi,isleyici-pencere-liste,isleyici-lejant cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # ek işleyicilerin masaüstü resimleri, .run/shots/arac-isleyici-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs renderers); ADR 0213)
cargo test --release -p kentos-native-style --test all renderer_timing -- --ignored --nocapture --test-threads=1   # ek işleyicilerin kurulum süreleri bütçeleriyle (ADR 0213 §6); web'inkiler: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; RENDERER_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/renderers.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
cargo test --release -p kentos-interaction --test perf label_texts -- --ignored --nocapture   # 100 000 parselin iki ifadeli sınıfının etiket metinleri (ADR 0212 §6)
KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; LABEL_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/labels.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp   # aynı süreler web'de, gönderilen WASM'la (ADR 0212 §6)
python3 scripts/fixtures/expression_extras.py --check   # ifade dili eklerinin değerlerini (tarih ve saat, `@` değerleri, düzenli ifade, dizi ve eşleme, toplama, mekânsal ilişki, başka katmandan değer) KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/expression/v2/extras.json (ADR 0214)
python3 scripts/fixtures/variable_form_cases.py --check   # Proje ayarları › Değişkenler'in form kurallarını (adlar, türlere göre değerler, yeni satır, tür değişimi) ve yerleşik `@` değerlerini kurallardan denetle; durumlar fixtures/project/v1/variable-form.json (ADR 0214 §4)
cargo test -p kentos-desktop project::variables::tests::screens -- --ignored --nocapture; cargo test -p kentos-desktop expression::tests::world_screens -- --ignored --nocapture --test-threads=1   # Değişkenler ve İfade oluşturucu'nun `@` değerleriyle başka katmana bakan önizlemesi, .run/shots/degiskenler-*, ifade-degiskenler-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs variables); ADR 0214)
python3 scripts/fixtures/proximity_cases.py --check   # yakınlık araçlarının durumlarını (en yakın hedef, alanları ve semti, uzaklık matrisinin üç biçimi, en yakın merkez, komşular ve ortak düz ve yaylı kenarlar, köşe, örtüşme, en kısa çizgi) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/processing/v1/proximity.json ve proximity.kcad (ADR 0215)
cargo test -p kentos-desktop processing::proximity_tests::screens -- --ignored --nocapture --test-threads=1   # yakınlık araçlarının şeridi, pencereleri ve çizgileri, .run/shots/islem-yakinlik-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs proximity); ADR 0215)
python3 scripts/fixtures/transfer_cases.py --check; python3 scripts/fixtures/field_map_cases.py --check   # Veri aktar'ın durumlarını (alan eşleme, tipli alanlar, Yalnız boşlara, geometri, ekleme, Yalnız raporla, retler, boş, kilitli ve grup hedef) ve Alan eşleme alanının kurallarını (satırın sorunu, not, Aynı adlarla doldur, denetim, düzenlemeler) ADR'den, KentOS kodu olmadan yazılmış başvurulardan denetle; durumlar fixtures/processing/v1/transfer.json, field-map.json (ADR 0216)
cargo test -p kentos-desktop processing::transfer_tests::screens -- --ignored --nocapture --test-threads=1   # Veri aktar'ın şeridi, alan eşlemesi, sorunlu satırları, raporu ve çizimdeki sonucu, .run/shots/islem-aktar-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs transfer); ADR 0216)
cargo test --release -p kentos-processing --test cases transfer_timing -- --ignored --nocapture   # Veri aktar'ın 10 000 parsel ve 10 000 satırda bütün çalışması bütçesiyle (ADR 0216 §7); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; TRANSFER_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/transfer.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/layer_ops_cases.py --check   # katman işlemlerinin durumlarını (bölme, birleştirme, çıkarma, yinelenenler, sıralama; yeni gruplar ve katmanlar, tipli hedef alanları, kilitli katman, kotun sıfırının işareti) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/processing/v1/layers.json ve layers.kcad (ADR 0217)
cargo test -p kentos-desktop processing::layer_tools_tests::screens -- --ignored --nocapture --test-threads=1   # katman işlemlerinin şeridi, pencereleri, raporları ve bölmenin yeni grubu, .run/shots/islem-katman-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs layertools); ADR 0217)
cargo test --release -p kentos-processing --test cases layer_timing -- --ignored --nocapture   # 100 000 parselde bölme, yinelenenler, sıralama ve iki 50 000'lik katmanın birleştirilmesi bütçeleriyle (ADR 0217 §9); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; LAYERS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/layers.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/convert_cases.py --check   # geometri dönüştürmenin durumlarını (merkezler Green teoremiyle ve iç noktalar GEOS'un kendisinden ya da yaylı tarama doğrusuyla, köşeler, noktalardan çizgi, tablolardan çizgi ve elips, parçalar, halkalar, alanlar, köşe ekle; kotlar) mpmath ile 50 basamakta, KentOS kodu olmadan denetle; durumlar fixtures/processing/v1/convert.json, convert.kcad ve convert-*.csv (ADR 0218)
cargo test -p kentos-desktop processing::convert_tests::screens -- --ignored --nocapture --test-threads=1   # dönüştürme araçlarının şeridi, pencereleri ve sonuçları, .run/shots/islem-donustur-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs geoconvert); ADR 0218)
cargo test --release -p kentos-processing --test cases convert_timing -- --ignored --nocapture   # 100 000 parselde iç nokta, bütün köşeler ve köşe ekle, 100 000 noktadan sıralı çizgiler, 100 000 satırdan semt çizgileri bütçeleriyle (ADR 0218 §13); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; CONVERT_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/convert.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/generate_cases.py --check   # geometri üretiminin durumlarını (Voronoi hücreleri kesirlerle her öbür noktanın dik ortayından, Delaunay üçgenleri tam sayılarla boş çember denetiminden, zarflar, kutular 50 basamakta, daireler kesirlerle Welzl'le, içbükey zarf ADR'nin kuralıyla, kutup polylabel'ın tanımından; hepsi GEOS'la çapraz) KentOS kodu olmadan denetle; durumlar fixtures/processing/v1/generate.json ve generate.kcad (ADR 0219)
cargo test -p kentos-desktop processing::generate_tests::screens -- --ignored --nocapture --test-threads=1   # geometri üretiminin şeridi, pencereleri ve sonuçları, .run/shots/islem-uret-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs generate); ADR 0219)
cargo test --release -p kentos-processing --test cases generate_timing -- --ignored --nocapture   # 100 000 noktada Voronoi, Delaunay ve içbükey zarf, 100 000 parselde döndürülmüş kutu ve kutup bütçeleriyle (ADR 0219 §8); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; GENERATE_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/generate.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/sampling_cases.py --check   # örneklemenin durumlarını (ağlar ADR'nin kafes formülleriyle, Nesnelere değenler GEOS'un uzaklığıyla, alan içi kesirlerle, tohumlu sayılar Python'un tam sayılarıyla, yol noktaları yayların tanımından, parçalar GEOS'un kesişimiyle; belirsiz yakınlıklar denetlenir) KentOS kodu olmadan denetle; durumlar fixtures/processing/v1/sampling.json ve sampling.kcad (ADR 0220)
cargo test -p kentos-desktop processing::sampling_tests::screens -- --ignored --nocapture --test-threads=1   # örneklemenin şeridi, pencereleri ve sonuçları, .run/shots/islem-orn-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs sampling); ADR 0220)
cargo test --release -p kentos-processing --test cases sampling_timing -- --ignored --nocapture   # 100 000 parselde altıgen ağ (değenler), düzenli, rastgele ve hat boyunca noktalar, 100 000 köşelik on yıldızın alt bölümleri bütçeleriyle (ADR 0220 Doğrulama); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; SAMPLING_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/sampling.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/kml_cases.py --check   # KML ve KMZ okuyucusunun örneklerini (klasörler, biçemler, StyleMap, veriler, izler, karışık MultiGeometry, bozuk ve atlanan öğeler, KMZ) expat ve zipfile ile, taşımaları PROJ'la denetle ve yazıcının kaydedilmiş KML ve KMZ'sini (KENTOS_WRITE_KML=1 cargo test -p kentos-formats --test all kml) girdisine göre okuyup her köşeyi PROJ'la karşılaştır; KentOS kodu olmadan; durumlar fixtures/formats/v1/kml (ADR 0222)
python3 scripts/fixtures/wgs84_write_cases.py --check   # GPX'in (iz ve rota) ve WGS 84 GeoJSON'un kaydedilmiş çıktılarını (KENTOS_WRITE_WGS84=1 cargo test -p kentos-formats --test all wgs84) girdisine göre okuyup PROJ'la denetle; durumlar fixtures/formats/v1/wgs84 (ADR 0222)
cargo test -p kentos-desktop exchange::kml_tests::screens -- --ignored --nocapture   # KML/KMZ içe aktar (çevirme onayıyla), KML/KMZ ve GPX dışa aktar, GeoJSON'un WGS 84'ü ve GNSS'in çizgileri, .run/shots/kml-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs kml); ADR 0222)
cargo test --release -p kentos-formats --test all kml_timing -- --ignored --nocapture   # 100 000 nesnede KML ve KMZ yazma ve okuma, projenin sistemine çevirme, GPX ve WGS 84 GeoJSON yazma bütçeleriyle (ADR 0222 Doğrulama); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm:formats; KML_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/kml.test.ts --disable-console-intercept; rm apps/web/src/io/pkg/.stamp
python3 scripts/fixtures/workbook_cases.py --check   # çalışma kitabı yazıcısının kaydedilmiş dosyalarını (KENTOS_WRITE_WORKBOOK=1 cargo test -p kentos-formats --test all workbook) okuyucunun kuralıyla, XLSX'leri openpyxl'le ve ODS'leri LibreOffice'le denetle; durumlar fixtures/table/v1/written (ADR 0223 §2)
python3 scripts/fixtures/table_points_cases.py --check; python3 scripts/fixtures/feature_export_cases.py --check   # Tablodan nokta'nın durumlarını (düzlem ve WGS 84 ODS'si PROJ'la, katman, retler; fixtures/processing/v1/table-points*) ve Öznitelik tablosunun dışa aktarılan sayfasını (fixtures/feature-table/v1/export.json) KentOS kodu olmadan denetle (ADR 0223 §3, §5)
cargo test -p kentos-desktop processing::table_points_tests::screens -- --ignored --nocapture --test-threads=1   # Tablodan nokta'nın şeridi, penceresi, Sayfa listesi, Kaydet ▾, noktalar, Öznitelik tablosunun Dışa aktar ▾'si ve koordinat listesinde çalışma kitabı, .run/shots/islem-tablo-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs tableexchange); ADR 0223)
cargo test --release -p kentos-formats --test all workbook_timing -- --ignored --nocapture; cargo test --release -p kentos-processing --test cases table_points_timing -- --ignored --nocapture   # 100 000 satırın XLSX, ODS ve CSV yazılması, 250 000 satırlık XLSX ve ODS'nin okunması, 100 000 satırdan Tablodan nokta bütçeleriyle (ADR 0223 Doğrulama); web'inkiler: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm:formats; WORKBOOK_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/workbook.test.ts --disable-console-intercept; rm apps/web/src/io/pkg/.stamp (Tablodan nokta scripts/perf/convert.test.ts'te)
python3 scripts/fixtures/lrs_cases.py --check   # doğrusal referanslamayı (konum, ölçünün yeri, iki ölçü arası parçalar ötelemeyle, rota oluşturma, kalibrasyon, çakıştırma, özet, çizgi ve alanın konumu; yaylar açıyla) ADR'den, KentOS kodu olmadan 50 basamaklı mpmath'le denetle; durumlar fixtures/lrs/v1/cases.json (ADR 0224)
python3 scripts/fixtures/lrs_processing_cases.py --check; python3 scripts/fixtures/lrs_scene.py --check   # Doğrusal referans araçlarının İşlemler durumlarını (fixtures/processing/v1/lrs.json, lrs.kcad, lrs-*.csv) ve resimlerin yolunu (fixtures/interaction/v1/lrs.kcad ve lrs/: D-100 ve D-200, parçaları, km taşları, kazalar, kaplama, hız, korkuluk ve levha tabloları) denetle (ADR 0224)
cargo test -p kentos-desktop processing::lrs_tests::screens -- --ignored --nocapture --test-threads=1   # Doğrusal referans'ın şeridi, pencereleri, yeni rotalar ve M'leri, olaylar, tablolar, Öznitelikler'in M'si, Koordinat listesinin M sütunu ve kart, .run/shots/islem-dr-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs lrs); Km yaz ölçüyle KENTOS_SHOTS_ONLY=km-yaz-olcu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture ve shots.mjs stationing --only stationing-measure; ADR 0224)
cargo test --release -p kentos-geometry-core --test all lrs::timing -- --ignored --nocapture   # 1 000 rotada 100 000 nokta ve 10 000 çizgi olayı, 100 000 noktanın konumlanması, 10 000 parçadan rota, 100 000 olayın çakıştırılması ve kalibrasyon bütçeleriyle (ADR 0224 §4); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; LRS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/lrs.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/template_cases.py --check   # bilgi kartı ve eylem şablonlarının bölünmesini, doldurulmasını, adreste yüzde kodlamayı ve hedefin türünü ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/expression/v2/template.json (ADR 0225 §6)
python3 scripts/fixtures/exif_cases.py --check; python3 scripts/fixtures/photo_import_cases.py --check   # EXIF okuyucusunu (KentOS kodu olmadan yazılmış JPEG ve TIFF başlıkları, kesirler, bozuk ve eksik konumlar; fixtures/exif/v1) ve konumlu fotoğrafların noktalarını PROJ'la denetle (fixtures/photos/v1/import.json; ADR 0225 §3)
python3 scripts/fixtures/layer_info_command_cases.py --check; python3 scripts/fixtures/attachments_scene.py --check   # cad.layers.info durumlarını sözleşmenin kurallarından ve eklerin sahnesini (fixtures/interaction/v1/attachments.kcad, gömülü PNG fotoğraflar, belgeler/tapu-101-5.pdf) denetle (ADR 0225)
python3 scripts/fixtures/table_styles_command_cases.py --check   # cad.layers.table durumlarını ve koşullu biçimin hücre görünüşünü (fixtures/table-styles/v1/looks.json) ADR'nin kuralından, KentOS kodu olmadan denetle (ADR 0413)
python3 scripts/fixtures/field_usage_cases.py --check   # cad.fields.usage durumlarını ve cad.layers.edit'in yeniden adlandırmada ifade, şablon ve adları izlemesini dilin sözcüklerinin bağımsız kopyasıyla, KentOS kodu olmadan denetle (ADR 0414)
python3 scripts/fixtures/duplicate_records_cases.py --check   # cad.records.duplicate durumlarını (bileşik ilişkilerin alt kayıtları, sıra, tekil alan, kilitli katman, retler) KentOS kodu olmadan denetle (ADR 0415)
python3 scripts/fixtures/field_policy_cases.py --check   # alanın bölme ve birleştirme kuralının cad.entities.edit durumlarını (paylaştırma ve kalanlar, toplam, en küçük ve en büyük, boşaltma, varsayılan) KentOS kodu olmadan denetle (ADR 0419)
python3 scripts/fixtures/forms_test_cases.py --check   # cad.forms.test durumlarını formun bağımsız Python ikiziyle (forms_cases.py) denetle (ADR 0420)
python3 scripts/fixtures/forms_recompute_cases.py --check   # cad.forms.recompute durumlarını formun bağımsız Python ikiziyle (forms_cases.py) denetle (ADR 0421)
python3 scripts/fixtures/subtypes_cases.py --check   # alt türlerin kurallarını ve cad.layers.subtypes, cad.entities.set, cad.polygon.create ve cad.forms.validate'teki etkisini KentOS kodu olmadan denetle (ADR 0422)
python3 scripts/fixtures/contingents_cases.py --check   # değer bileşimlerinin kurallarını ve cad.layers.contingents, cad.entities.set, cad.polygon.create ve cad.forms.validate'teki etkisini KentOS kodu olmadan denetle (ADR 0423)
python3 scripts/fixtures/forms_compare_cases.py --check   # cad.forms.compare durumlarını sürüm farkının bağımsız Python yazımıyla denetle (ADR 0427)
python3 scripts/fixtures/forms_transition_cases.py --check   # cad.forms.transition durumlarını formun bağımsız Python ikiziyle denetle (ADR 0428)
python3 scripts/fixtures/layer_rules_cases.py --check   # kayıt kurallarının, tekil alanın, hariç ucun, ekranın yayını ve kaydının durumlarını (cad.layers.rules, cad.entities.set, cad.polygon.create, cad.forms.validate, cad.forms.submit) formların ve alanların bağımsız ikizleriyle, KentOS kodu olmadan denetle (ADR 0438)
python3 scripts/fixtures/forms_withdraw_cases.py --check   # formu yayından kaldırmanın durumlarını (dört yazma komutunun reddi, yeniden yayına alma, kuralları uygulanan form) formun ikiziyle, KentOS kodu olmadan denetle (ADR 0445)
python3 scripts/fixtures/overlay_cases.py --check   # bindirmenin doğruluk korpusunu (birleşim, kesişim, farklar; teğet ve zincirli tamponlar, delikler, şeritler, TM koordinatları) GEOS (GDAL OGR) ve 50 basamaklı kapalı formüllerle, KentOS kodu olmadan denetle; durumlar fixtures/overlay/v1/cases.json, Rust tarafı geometry-core'un overlay_cases testi (ADR 0500)
/usr/bin/python3 scripts/fixtures/relate_cases.py --check   # cad.entities.relate durumlarını (DE-9IM matrisleri, on yüklem, desen, daire, okunmayan nesne, retler) GEOS'la QGIS 4.2'nin Python'u üzerinden denetle (ADR 0501)
/usr/bin/python3 scripts/fixtures/geopackage_cases.py --check   # GeoPackage crate'inin okuduğunu QGIS 4.2.3'ün yazdığı qgis.gpkg'nin GDAL/OGR okuyuşuyla, KentOS'un yazdığı kentos.gpkg ve kentos-export.gpkg'yi OGR, QGIS 4.2 ve GDAL'ın validate_gpkg.py'siyle denetle; yazmak için KENTOS_WRITE_GPKG=1 cargo test -p kentos-geopackage ve -p kentos-headless --test headless geopackage (ADR 0502)
/usr/bin/python3 scripts/fixtures/gpkg_import_cases.py --check   # file.import.gpkg durumlarını OGR'ın okuduğundan ADR'nin kuralıyla kurulan nesnelerle denetle; --files küçük GeoPackage'ları GDAL'la yeniden yazar (ADR 0502)
/usr/bin/python3 scripts/fixtures/wide_values_cases.py --check   # bağlı katmanın gidiş-dönüşünü (2⁵³ üstü ve en küçük 64 bitlik tam sayı, 0, false, boş metin ve NULL, dilimli zaman) OGR'ın yazdığı genis.gpkg ile uygulamanın geri yazdığı kentos-genis.gpkg'yi sqlite3'le hücre hücre karşılaştırarak denetle; yazmak için KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all wide_values (ADR 0512)
python3 scripts/fixtures/feed_paging_cases.py --check   # servisten veri almanın bitişini (WFS 2.0 ve 1.1, OGC API, ArcGIS; sunucunun sayfa sınırı, eşleşen sayı, next'siz yanıt, en çok nesne, sayfa sınırı) sahte sunucuyla ve OGC API uygunluk sayfalarının profilini KentOS kodu olmadan denetle; durumlar fixtures/services/v1/paging.json, Rust tarafı kentos-services'in paging testi (ADR 0513)
KENTOS_SHOTS_ONLY=servis-ogc-uygunluk cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Servisten veri al'da OGC API'nin Uygunluk satırı ve desteklenmeyen süzgecin notu, çevrimdışı, .run/shots/arac-servis-ogc-uygunluk-* (ADR 0513)
cargo test --release -p kentos-native-application --test all provider_timing -- --ignored --nocapture   # 1 milyon parsellik sentetik GeoPackage'da (.run/perf, KENTOS_PERF_ROWS) sağlayıcının açma, sayma, ilk ve sonraki sayfa, görünen kutu ve bağlama süreleri ve süreç belleği (ADR 0509; docs/perf/provider-2026-10-18.md)
/usr/bin/python3 scripts/fixtures/processing_sink_cases.py --check   # cad.processing.run'ın sink'iyle yazılan G1 GeoPackage'larını (kesişim, tampon, birleştirme; KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all processing_sink yazar) OGR'la, KentOS kodu olmadan denetle: alanlar, öznitelik soyu, geçerlilik, sistem (ADR 0508 §4); soy belgesi sqlite3 ve validate_gpkg.py ile (ADR 0511)
python3 scripts/fixtures/processing_command_cases.py --check   # cad.processing.run ve .tools durumlarını İşlemler'in kendi ortak durumlarından (komutun sözleriyle) ve web'in donmuş envanterinden, KentOS kodu olmadan üret ve denetle (ADR 0508)
python3 scripts/fixtures/fields_write_cases.py --check   # cad.fields.write durumlarını (Alan hesapla, Yapıştır, tür ve tekil alan denetimi, policy, plan, retler) dilin ve alanların bağımsız Python başvurularıyla, KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.fields.write.json (ADR 0507)
cargo test -p kentos-desktop features::tests::write_screens -- --ignored --nocapture   # Alan hesapla ve Yapıştır pencerelerinin resimleri, .run/shots/alan-hesapla-* (ADR 0507)
python3 scripts/fixtures/package_cases.py   # Paketle'yi KentOS kodu olmadan denetle: başsız sunucunun paket testinin taşınan klasörü bağımsız KCAD okuyucusu ve hashlib'le okunur (kopyalar kaynaklarıyla bayt bayt, bağlar paketin içinde, bağlı katmanın göreli yolu ve sürümü, kalanlar) (ADR 0506)
/usr/bin/python3 scripts/fixtures/postgis_cases.py [--screens]   # PostGIS bağlı katmanını geçici docker PostGIS'inde (5432'ye dokunmaz; QGIS'in GeoPackage'ı ogr2ogr ile) uçtan uca: kentos-postgis'in testi, masaüstünün penceresi, sonuç psycopg2'yle; resimler .run/shots/postgis-* (ADR 0505)
python3 scripts/fixtures/crs_bounds_cases.py --check   # kutu dönüşümünü (sıklaştırma, köşeler, antimeridyende saran kutu) PROJ'un transform_bounds'uyla, KentOS kodu olmadan denetle; durumlar fixtures/geodesy/v1/bounds.json, Rust tarafı geometry-core'un crs testi (ADR 0504)
/usr/bin/python3 scripts/fixtures/bound_sources_cases.py --check   # cad.sources.describe, .edit ve .sync durumlarını dosyanın kendi tablolarından (sqlite3), OGR'ın nesnelerinden ve hashlib'in SHA-256'sından, KentOS kodu olmadan denetle; dosyaya yazanı KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all sources:: geçici kopyada sınar, kentos-synced.gpkg'yi geopackage_cases.py OGR'la okur (ADR 0503)
KENTOS_SHOTS_ONLY=gpkg cargo test -p kentos-desktop exchange::gpkg_tests::screens -- --ignored --nocapture   # GeoPackage içe aktar penceresi, çevirme onayı, alınmış çizim ve bağlı katman (Kaynağa bağla, rozet, okuma sorusu), .run/shots/gpkg-* (ADR 0502, 0503)
python3 scripts/fixtures/related_count_cases.py --check   # ilişkili kayıtlar öğesinin en az ve en çok sayısını cad.forms.submit'in alt kayıtlarıyla (bağla, kopar, yeni, gizli öğe, çoktan çoğa) KentOS kodu olmadan denetle (ADR 0444)
python3 scripts/fixtures/schema_describe_cases.py --check   # cad.schema.describe durumlarını (türler, katman, sözcük, tanımlar, sayfalar, imleç, retler) domains_cases.py'nin katmanlarıyla, KentOS kodu olmadan denetle (ADR 0443)
python3 scripts/fixtures/forms_publish_cases.py --check; python3 scripts/fixtures/attachments_inspect_cases.py --check   # yayının senaryolarını ve kırdığı kayıtları (cad.forms.edit.publish.json) ve eklerin bilgisini (cad.attachments.inspect.json, okuduğu dosyalar fixtures/attachments/v1, EXIF fixtures/exif/v1/cases.json'dan) formun ve senaryoların ikizleriyle, KentOS kodu olmadan denetle (ADR 0442)
python3 scripts/fixtures/sources_cases.py --check   # cad.domains.export, cad.domains.edit'in içe aktarması ve cad.fields.migrate durumlarını değer listelerinin ve alanların bağımsız kurallarıyla, KentOS kodu olmadan denetle (ADR 0439)
python3 scripts/fixtures/domains_cases.py --check   # cad.domains.edit durumlarını (listeler, bağlar, kodların göçü, retler) KentOS kodu olmadan denetle (ADR 0430)
python3 scripts/fixtures/form_schema_cases.py --check   # formun JSON Schema'sını (catalog/v1/forms/form-v1.schema.json) deponun bütün formlarıyla ve bozuk örneklerle jsonschema ile denetle (ADR 0434)
python3 scripts/fixtures/forms_package_cases.py --check   # cad.forms.export ve cad.forms.import durumlarını (paketin alanları, bağlar, yeniden yazılan ifadeler, eklenen alanlar ve listeler, retler) KentOS kodu olmadan denetle (ADR 0435)
cargo test --release -p kentos-native-application --test all forms_timing -- --ignored --nocapture --test-threads=1   # 100 000 kayıtlı katmanda 200 öğeli formun süreleri bütçeleriyle: kayıt, tuş, Kayıtları denetle, Hesapları yenile, toplu yazma (ADR 0424)
.run/py/bin/python -m unittest python/tests/test_examples.py   # formların geliştirici örnekleri (python/examples/forms: bina, alt türler ve bileşimler, malikler, adres) baştan sona çalışır
KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=ekler-oznitelikler,ekler-uzerine,ekler-kart,ekler-kart-foto,ekler-goruntuleyici,ekler-adres,ekler-bilgi-karti,ekler-konumlu,ekler-eylemler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # nesne eklerinin masaüstü resimleri (fotoğraflar wgpu ister), .run/shots/arac-ekler-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs attachments); ADR 0225)
python3 scripts/fixtures/geometry_generator_cases.py --check   # Geometri üretici'nin adımlarını (merkezler, köşeler, paralel, uzat, tampon uzaklık ve alan sınırlarıyla, kutu, örtü, daire, dilim, çokgen, öteleme, döndür, ölçekle, ifadeli değerler) ADR'den, KentOS kodu olmadan denetle; durumlar fixtures/style/v1/generator.json (ADR 0226 §1)
python3 scripts/fixtures/effects_cases.py --check; python3 scripts/fixtures/layer_compose_command_cases.py --check; python3 scripts/fixtures/symbol_extras_scene.py --check   # çizim efektlerinin, karışım kiplerinin ve maskelerin resimlerini numpy'la (fixtures/compose/v1/cases.json), cad.layers.compose durumlarını ve izin sahnesini (fixtures/interaction/v1/symbol-extras.kcad) KentOS kodu olmadan denetle (ADR 0226)
KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu; (cd apps/web && node scripts/e2e/compose.mjs)   # efektler, kipler ve maskeler gerçek GPU'da ve tarayıcının WebGPU ve WebGL2'sinde, 1× ve 4×, başvurunun resimleriyle (ADR 0226)
cargo test --release -p kentos-render-wgpu --test gpu composition_timing -- --ignored --nocapture; (cd apps/web && node scripts/e2e/compose.mjs --time); cargo test --release -p kentos-native-style --test all generator_timing -- --ignored --nocapture   # Katman çiziminin kareye eklediği masaüstünde ve web'de (makinenin GPU'sunda), Geometri üretici'nin kuruluma eklediği, bütçeleriyle (ADR 0226 §7)
KENTOS_SHOTS_ONLY=uretici,efekt,uretici-katman-ekle,uretici-adim-ekle cargo test -p kentos-desktop style::designer::screens -- --ignored --nocapture   # Sembol tasarımcısında Geometri üretici ve Efektler, .run/shots/sdes-* (web'inkiler: node apps/web/scripts/e2e/shots.mjs symboldesigner --only generator,effects,add-step-menu,add-layer-menu-into-generator; ADR 0226)
python3 scripts/fixtures/chart_cases.py --check; python3 scripts/fixtures/chart_scene.py --check   # grafiklerin ve alan istatistiklerinin kurallarını (alanların özetleri ve istatistikleri, altı grafiğin modeli, eksenler, 4 anlamlı basamak, yazı genişlikleri) Python'un kesirleriyle, KentOS kodu olmadan yazılmış başvurudan ve izlerin ve resimlerin sahnesini (fixtures/interaction/v1/charts.kcad) denetle; durumlar fixtures/charts/v1/cases.json (ADR 0227)
KENTOS_SHOTS_ONLY=grafik-istatistik,grafik-cubuk,grafik-pasta,grafik-histogram,grafik-kutu,grafik-sacilim,grafik-cizgi,grafik-ipucu,grafik-sacilim-secim,grafik-tablo cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # İstatistik ve Grafik sekmelerinin masaüstü resimleri, .run/shots/arac-grafik-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs charts); ADR 0227)
cargo test --release -p kentos-interaction --test perf charts_on_a_large_layer -- --ignored --nocapture; cargo test --release -p kentos-desktop charts::tests::painting_100000_points -- --ignored --nocapture   # 100 000 nesnede verinin kurulması, özet, alanın istatistikleri, altı grafiğin modeli ve yerleşimi, vurgu, tıklama ve masaüstünde saçılımın resmi bütçeleriyle (ADR 0227 §11); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; CHARTS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/charts.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/geocode_cases.py --check   # adres aramasının kurallarını (katlama ve sözcükler, adresin parçaları, adların benzerliği, dizin, adresten konuma adaylar ve puanları, konumdan adres, adresin yazılışı, alanların tanınması; metre ve milimetre projesi) ADR'den, KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/geocode/v1/cases.json (ADR 0229)
python3 scripts/fixtures/geocode_online_cases.py --check   # çevrimiçi sağlayıcıların isteklerini (parametreler, kodlama), yanıtlarının okunuşunu (OpenStreetMap'in gerçek yanıtları ve belgelerden yazılanlar, fixtures/geocode/v1/answers), adaylarının KentOS puanını ve toplu işin seçimini ADR'den denetle; durumlar fixtures/geocode/v1/online.json (ADR 0229 §10)
python3 scripts/fixtures/address_rules_cases.py --check; python3 scripts/fixtures/address_define_command_cases.py --check   # adres verisinin tanımının kurallarını bağımsız KCAD okuyucusunun kurallarıyla (fixtures/geocode/v1/rules.json) ve cad.addresses.define durumlarını sözleşmenin kurallarından denetle (ADR 0229 §2, §14)
python3 scripts/fixtures/address_processing_cases.py --check; python3 scripts/fixtures/address_scene.py --check   # Adres araçlarının İşlemler durumlarını (fixtures/processing/v1/address.json, address.kcad, adresler.csv) ve izlerin sahnesini ve izlerini (fixtures/interaction/v1/addresses.kcad, address-search.json, address-reverse.json) denetle (ADR 0229)
cargo test --release -p kentos-geometry-core --test all geocode::timing -- --ignored --nocapture   # 200 000 kapılı şehirde dizin, tam ve hatalı adres, 100 000 adresin ayrıştırılması ve toplu eşlenmesi, 100 000 konumdan adres bütçeleriyle (ADR 0229 §17)
KENTOS_SHOTS_ONLY=adres-serit,adres-serit-cad,adres-ara,adres-belirsiz,adres-kaynak,adres-konumdan,adres-verisi,adres-saglayicilar,adres-islem,adres-eslendi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # adres aramasının masaüstü resimleri, .run/shots/arac-adres-* (web'e ara, ADR 0228; ADR 0229)
KENTOS_TRACES_ONLY=address-search,address-reverse cargo test -p kentos-desktop every_trace_passes   # adres aramasının izleri masaüstünde, üç çeşitte (ADR 0229)
python3 scripts/fixtures/subref_cases.py --check; python3 scripts/fixtures/dimension_command_cases.py --check   # alt nesne yerlerini (noktalar, izleme, kenetin yeri) ve cad.dimensions.reassociate durumlarını KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/subref/v1/cases.json ve fixtures/commands/v1/cad.dimensions.reassociate.json (ADR 0249)
KENTOS_TRACES_ONLY=dimension-refs cargo test -p kentos-desktop every_trace_passes; ./target/debug/kentos-cad kullan dimension-refs   # ilişkili ölçünün izi (bağ, tutamaçla izleme, silmede kopma, Yeniden bağla) ve resimleri, .run/shots/kullanim/masaustu-dimension-refs-* (ADR 0249)
python3 scripts/fixtures/edge_command_cases.py --check   # Buda, Uzat, Köşe yuvarla ve Pah'ın ortak durumlarını (sınırların üç biçimi, Çit sırası, uyarılar, yolun köşesi, iki çizgi, Kırp, tüm köşeler, retler) KentOS kodu olmadan, kesirlerle ve 60 basamaklı aşkın değerlerle denetle; durumlar fixtures/commands/v1/cad.entities.{trim,extend,fillet,chamfer}.json (ADR 0250)
KENTOS_TRACES_ONLY=edge-commands cargo test -p kentos-desktop every_trace_passes; ./target/debug/kentos-cad kullan edge-commands   # araçların komutla önizlemesi ve yazması, .run/shots/kullanim/masaustu-edge-commands-* (ADR 0250)
cargo test -p kentos-desktop sheet_book; cargo test -p kentos-desktop sheet_book::tests::screens -- --ignored --nocapture   # pafta kitabı projenin dosyasında: dosyadan açma, paftadaki değişikliğin çizime yazılması, kaydedip yeniden açma, cihazdaki kitabın göçü ve kenara ayrılması, okunamayan kitap; resimler .run/shots/pafta-kitabi-* (ADR 0251)
KENTOS_TEST_DB=required cargo test -p kentos-application --test project_sheets   # veritabanı projesinin pafta kitabı: bilgi, görüntü, ProjectPatch.sheets (koru, değiştir, kaldır, ret), kopya ve kontrol noktasından geri yükleme; geçici veritabanında (ADR 0251)
python3 scripts/fixtures/acceptance_room.py --check; python3 scripts/fixtures/sheets_edit_command_cases.py --check   # ilk kabul çiziminin senaryosunu ve izin çizimini, cad.sheets.edit durumlarını KentOS kodu olmadan denetle (ADR 0252)
.run/py/bin/python -m unittest discover -s python/tests -p "test_acceptance.py"; cargo test -p kentos-mcp --test acceptance; cargo test -p kentos-plot   # kabul çizimi Python'dan ve MCP'den, başsız kitap PDF'i; DXF ve PDF bağımsız okunur, antet poppler'ın pdftotext'iyle (ADR 0252)
KENTOS_ACCEPTANCE_OUT=$PWD/.run/shots/kabul cargo test -p kentos-desktop acceptance_tests; KENTOS_TRACES_ONLY=acceptance-room cargo test -p kentos-desktop every_trace_passes; ./target/debug/kentos-cad kullan acceptance-room   # kabul çizimi masaüstünde (ajan bağlantısı, Dışa aktar › DXF, pafta kipinin PDF'i; dosyalar .run/shots/kabul) ve fareyle ve klavyeyle, resimler .run/shots/kullanim/masaustu-acceptance-room-* (ADR 0252)
python3 scripts/fixtures/protocol_cases.py --check   # komut protokolünün kurallarını (plan kimliği, parmak izi, özet, zarf, beklentiler, idempotency, sayıların kanonik yazımı) KentOS kodu olmadan denetle; durumlar fixtures/protocol/v1/cases.json, Rust'ta protocol_cases.rs (ADR 0253)
python3 scripts/fixtures/reshape_command_cases.py --check; python3 scripts/fixtures/region_command_cases.py --check; python3 scripts/fixtures/dimension_create_cases.py --check   # Esnet, Kır, Birleştir, Böl, Alan oluştur, Tarama ve Ölçü ekle'nin ortak durumlarını KentOS kodu olmadan, kesirlerle ve 60 basamaklı aşkın değerlerle denetle; durumlar fixtures/commands/v1/cad.entities.{stretch,break,join,divide,boundary,hatch}.json ve cad.dimensions.create.json (ADR 0254, 0255)
python3 scripts/fixtures/stretch_policy_cases.py --check   # Esnet'in yay kuralını (uçlar, kiriş yüksekliği), kesişen pencereyi ve izleyenleri (ilişkili ölçü ve tarama, follows_object, planın dependents'ı) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.entities.stretch.policy.json (ADR 0297)
python3 scripts/fixtures/lengthen_command_cases.py --check   # Uzat-kısalt'ın durumlarını (Fark, Yüzde, Toplam, Dinamik, yayda açı, elips yayı, kot, ölçü ve genişlik, retler) KentOS kodu olmadan, açılar ve elips yayları mpmath ile denetle; durumlar fixtures/commands/v1/cad.entities.lengthen.json (ADR 0298)
python3 scripts/fixtures/divide_place_cases.py --check   # Böl'ün blok, hizalama, ilk uzaklık ve uç durumlarını (çizgi, çoklu çizgi, yay, daire, elips yayı gerçek uzunlukla, retler) KentOS kodu olmadan mpmath'le denetle; durumlar fixtures/commands/v1/cad.entities.divide.places.json (ADR 0299)
python3 scripts/fixtures/array_assoc_cases.py --check   # ilişkili dizinin durumlarını (ızgara, doğru, kutupsal, yol; izleme, elle taşınan ve silinen öğe, geri alma, kilitli nesne, ilişkisiz dizi) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.entities.array.assoc.json (ADR 0300)
python3 scripts/fixtures/explode_command_cases.py --check   # Patlat'ın durumlarını (yollar, alanlar, bloğun yerleştirmesi, ilişkili dizi, patlamayan ve kilitli nesneler, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.entities.explode.json (ADR 0302)
KENTOS_SHOTS_ONLY=dizi-iliskili-secenek,dizi-patlat cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all array_assoc::explode_timing -- --ignored --nocapture   # İlişkili seçeneği ve ilişkili dizinin patlatılması (.run/shots/arac-dizi-iliskili-secenek-*, arac-dizi-patlat-*), 30 000 parçanın süresi (ADR 0302)
python3 scripts/fixtures/curve_edit_cases.py --check   # Eğriyi düzenle'nin durumlarını (çoklu çizginin dört düzenlemesi, geçiş ve denetim noktaları, teğetler, ağırlık, düğüm, çoklu çizgiye çevirme, retler) yapıların IEEE ikiziyle bit bit ve 50 basamaklı mpmath'le, KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.curves.edit.json (ADR 0303)
KENTOS_SHOTS_ONLY=egri-duzenle-coklu,egri-duzenle-gecis,egri-duzenle-denetim,egri-duzenle-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all curves_edit::timing -- --ignored --nocapture   # Eğriyi düzenle'nin resimleri (.run/shots/arac-egri-duzenle-*) ve 100 000 nesneli çizimde önizleme ve yazma süreleri (ADR 0303)
python3 scripts/fixtures/match_command_cases.py --check   # Özellik kopyala'nın durumlarını (nesnenin ve türün özellikleri, seçilenler, kalınlığın kuralı, kilitli ve gizli katman, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.entities.match.json (ADR 0304)
KENTOS_SHOTS_ONLY=ozellik-kopyala-kaynak,ozellik-kopyala-anahtar,ozellik-kopyala-yazi,ozellik-kopyala-olcu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all match_props::timing -- --ignored --nocapture   # Özellik kopyala'nın resimleri (.run/shots/arac-ozellik-kopyala-*) ve 100 000 nesnede süreleri (ADR 0304)
python3 scripts/fixtures/cleanup_command_cases.py --check   # Çizimi temizle'nin durumlarını (boş ve yinelenen nesneler türlere göre tolerans içinde, eş doğrultulu çizgiler, köşeler, yok sayılan özellikler, rapor ve kayıp, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.entities.cleanup.json (ADR 0305)
KENTOS_SHOTS_ONLY=temizle-bulunan,temizle-milimetre,temizle-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all cleanup_cmd::timing -- --ignored --nocapture   # Çizimi temizle'nin resimleri (.run/shots/arac-temizle-*) ve 100 000 nesnede plan ve yazma süreleri (ADR 0305)
python3 scripts/fixtures/audit_command_cases.py --check   # Denetle'nin durumlarını (bozuk bağlar ve kayıp stiller, onarım ve geri alma, kilitli katman, kullanılmayan katman, grup, blok ve stiller, stillerin temizliği, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.drawing.audit.json (ADR 0306)
KENTOS_SHOTS_ONLY=denetle-bulunan,denetle-stiller,denetle-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all audit_cmd::timing -- --ignored --nocapture   # Denetle'nin resimleri (.run/shots/arac-denetle-*) ve 100 000 nesnede plan ve yazma süreleri (ADR 0306)
python3 scripts/fixtures/multipart_edit_cases.py --check   # Kır, Buda ve Uzat'ın çok parçalı çizgideki durumlarını (tıklanan parça, parçalar tek nesnede, kot ve ölçü, hızlı kip, çit, pencere, retler) KentOS kodu olmadan kesirlerle denetle; durumlar fixtures/commands/v1/cad.entities.{break,trim,extend}.parts.json (ADR 0307)
python3 scripts/fixtures/explode_curve_cases.py --check   # Patlat'ın eğrilerdeki durumlarını (daire, yay, elips, eliptik yay, eğri; sayı, aralık, sapma, varsayılan; retler) KentOS kodu olmadan mpmath'le denetle; durumlar fixtures/commands/v1/cad.entities.explode.curves.json (ADR 0308)
KENTOS_SHOTS_ONLY=patlat-egri,patlat-sayi,patlat-yazildi,patlat-cbs cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Patlat'ın eğrilerdeki resimleri, .run/shots/arac-patlat-* (ADR 0308)
python3 scripts/fixtures/layer_find_cases.py --check   # Katmanları bul ve Katmanları çevir'in durumlarını (AutoCAD'in wcmatch örüntüsü belgesinden, özellikler, kurallar, standart katmanın eklenmesi ve stili, kilitli ve servis katmanı, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.layers.{find,translate}.json (ADR 0311)
KENTOS_SHOTS_ONLY=katman-dondur,katman-ara,katman-suzgec,katman-cevir,katman-durumlari cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all layers_find_cmd::timing -- --ignored --nocapture   # dondurma, örüntülü arama, Süzgeç ▾, Katmanları çevir ve durumların dışa ve içe aktarması (.run/shots/arac-katman-*), 500 katman ve 100 000 nesnede bulma ve çevirme süreleri (ADR 0311)
python3 scripts/fixtures/standards_cases.py --check; python3 scripts/fixtures/look_cases.py --check; python3 scripts/fixtures/print_color_cases.py --check   # Standardı denetle ve uygula'nın durumlarını (bulgular sırasıyla ve iletileriyle, birleştirme, ekleme, düzeltme, stiller, retler), görünüşün kaynağını (kendi, katmana göre, BYBLOCK, iç içe) ve kâğıdın ve DXF'in renklerini KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.standards.{check,apply}.json, fixtures/look/v1 (ADR 0312)
KENTOS_SHOTS_ONLY=standart-bulgular,standart-uygulandi,standart-serit,standart-blok cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all standards_cmd::timing -- --ignored --nocapture   # Çizim standardı penceresi, şeridin Standart paneli ve yerleştirmeden kesikli çizgi alan blok (.run/shots/arac-standart-*), 500 katman ve 100 000 nesnede denetleme ve uygulama süreleri (ADR 0312)
python3 scripts/fixtures/line_type_cases.py --check   # LIN okuma ve yazmayı (başlık, öğeler, yazı ve şekil, retler, en kısa yazım), desenin ölçeğini (kâğıt ve çizim deseni, LTSCALE, nesnenin ölçeği, birimler) ve cad.linetypes.edit durumlarını KentOS kodu olmadan denetle; durumlar fixtures/linetypes/v1/cases.json ve fixtures/commands/v1/cad.linetypes.edit.json (ADR 0313)
KENTOS_SHOTS_ONLY=cizgitipi-pencere,cizgitipi-cizim,cizgitipi-oznitelik cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-style --test all line_type_timing -- --ignored --nocapture   # Çizgi tipleri penceresi, LIN desenli plan ve Öznitelikler'in Çizgi tipi satırı (.run/shots/arac-cizgitipi-*), 100 000 desenli çizginin kurulumu (ADR 0313)
python3 scripts/fixtures/library_command_cases.py --check; python3 scripts/fixtures/style_impact_cases.py --check; python3 scripts/fixtures/blocks_command_cases.py --check; python3 scripts/fixtures/sheets_edit_command_cases.py --check   # cad.library.edit (oluştur, güncelle, kopyala, ad, taşı, sil, .kstil al ve ver, sembol ver, retler), cad.styles.impact (katman, çizgi tipi, yazı ve ölçü stili; blok parçaları, durumlar, haritalar), cad.blocks.edit'in kopya, dışa ve içe aktarması ve haritanın katman geçersiz kılmaları durumlarını KentOS kodu olmadan denetle (ADR 0314)
python3 scripts/fixtures/block_insert_cases.py --check; python3 scripts/fixtures/block_scale_cases.py --check; python3 scripts/fixtures/look_cases.py --check; python3 scripts/fixtures/subref_cases.py --check   # cad.blocks.insert (ad ve kimlik, X/Y ölçeği, birimsiz ve birimli blok, öznitelikler, retler), yerleştirmenin y ölçeğinde parçaların görüntüsü (elips, yay, yazı, yay kenarı) ve Patlat, 0 katmanının görünüşü, blok içi yolun yerel kimlikle izlenmesi; KentOS kodu olmadan (ADR 0315)
KENTOS_SHOTS_ONLY=blok-katman-kurali,blok-olcek,blok-olcek-oznitelik,blok-ekle-y cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-geometry-core --test all block_scales::timing -- --ignored --nocapture   # katman kuralının doğrulama çizimi, gerilmiş blok ve Öznitelikler'i, Blok ekle'nin Y ölçeği (.run/shots/arac-blok-*), 10 000 yerleştirmenin süresi (ADR 0315)
python3 scripts/fixtures/block_file_cases.py --check   # file.import.block'un durumlarını (çizim tek blok, adlar, çakışan ad ve kimlik, bloklar adlarıyla, replace, retler; kaynak fixtures/blocks/v1/ofis-bloklari.kcad) alışverişin bağımsız başvurusuyla, KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/file.import.block.json (ADR 0316)
KENTOS_SHOTS_ONLY=blok-duzenleyici,blok-duzenleyici-cizim,blok-duzenleyici-kaydedildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test -p kentos-desktop block_editor; cargo test -p kentos-headless --test headless a_block_is_written   # Blok düzenleyicinin resimleri (.run/shots/arac-blok-duzenleyici-*) ve akışı, WBLOCK ve dosyadan alma uçtan uca (ADR 0316)
python3 scripts/fixtures/block_attribute_cases.py --check   # öznitelik seçeneklerinin, eşitlemenin, alanların, yerlerin ve kartın (cad.blocks.edit), cad.blocks.replace ve cad.blocks.query durumlarını KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.blocks.{edit.attributes,replace,query}.json (ADR 0317)
python3 scripts/fixtures/dynamic_block_cases.py --check; python3 scripts/blocks/library.py --check   # dinamik blokların durumlarını (cad.blocks.parameters'ın oku, değiştir, sıfırla ve retleri; cad.blocks.edit'in dynamic'i, kuralları ve yerleştirmelerin değerleri; cad.blocks.proxy'nin varyantları ve cad.blocks.freeze) KentOS kodu olmadan ve KentOS blok kitaplığının metnini denetle; durumlar fixtures/commands/v1/cad.blocks.{parameters,edit.dynamic,proxy,freeze}.json (ADR 0318, 0319)
python3 scripts/fixtures/dimension_create_cases.py --check; python3 scripts/fixtures/dimension_follow_cases.py --check   # ölçü türlerinin bağlarını ve kurallarını, Zincir ve Baz'ı, izlemeyi (taşıma, ölçekleme, döndürme, silme, Buda'nın ucu ve ortası, Uzat) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.dimensions.{create,follow}.json (ADR 0320)
python3 scripts/fixtures/dimension_value_cases.py --check; python3 scripts/fixtures/dimension_edit_cases.py --check   # değerin yazımını (yuvarlama, sıfırlar, ayak-inç, tolerans, alternatif birim, <>) ve cad.dimensions.edit ile cad.dimensions.query durumlarını KentOS kodu olmadan denetle; durumlar fixtures/text/v1/dimension-values.json, fixtures/commands/v1/cad.dimensions.{edit,query}.json (ADR 0321)
KENTOS_SHOTS_ONLY=olcu-secenekler,olcu-aralik,olcu-kirma cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # ölçü seçenekleri ve Öznitelikler, aralık, kırma (.run/shots/arac-olcu-secenekler-*, -aralik-*, -kirma-*; ADR 0321)
python3 scripts/fixtures/style_edit_command_cases.py --check; python3 scripts/fixtures/annotation_style_cases.py --check   # stillerin dışa ve içe aktarması ve stilden ayrılan alanlar (elle değiştirme, stil değişince korunma, stile dönme, silme, geri alma) KentOS kodu olmadan; durumlar fixtures/commands/v1/cad.styles.edit.json, fixtures/text/v1/styles.json (ADR 0322)
KENTOS_SHOTS_ONLY=olcu-isaretler,olcu-isaret-onizleme,olcu-isaret-izleme,stil-dosya,stil-gecersiz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # merkez işareti, eksen çizgisi ve kot, izlemeleri, stil penceresinin Dışa aktar…/Al…'ı ve Stilden ayrılan (ADR 0322)
python3 scripts/fixtures/annotative_cases.py --check   # ölçekli açıklamaların komut durumlarını (altı işlem, retler, çizim ölçeğinin değişmesi, taşıma) ve temsillerini (her ölçekte çizilen, gizlenen, yeni ölçeğe taşınan) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.annotations.scales.json, fixtures/annotative/v1/cases.json (ADR 0323)
cargo test -p kentos-desktop annotative_sheet; KENTOS_SHOTS_ONLY=olcekli-oznitelik,olcekli-yeri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # kabul odasının 1:50 ve 1:100 paftaları masaüstünün PDF'iyle, değerin yüksekliği pdftotext -bbox ile; Öznitelikler'in ölçek satırları ve Ölçekteki yeri, .run/shots/arac-olcekli-* (ADR 0323)
python3 scripts/fixtures/leader_command_cases.py --check   # cad.leaders.edit durumlarını (kollar, hizala, topla, blok içeriğin yerleri, uçların bağı ve izlemesi, silmeler, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.leaders.edit.json (ADR 0326)
KENTOS_SHOTS_ONLY=kilavuz-kollar,kilavuz-hizala,kilavuz-stilleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # çok kollu kılavuzun resimleri, .run/shots/arac-kilavuz-* (ADR 0326)
python3 scripts/fixtures/table_command_cases.py --check   # cad.tables.edit durumlarını (hücreler, formüller ve hataları, kayan başvurular, biçimler, alanlar ve blok sayısı, blok çizelgesi, bağlı dosyanın güncellemesi ve kuralları, bağlama, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.tables.edit.json (ADR 0327)
KENTOS_SHOTS_ONLY=tablo-formul,tablo-baglanti,dxf-bicimler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all tables_cmd::timing -- --ignored --nocapture   # formüllü ve biçimli tablo, bağlı tablo ve blok çizelgesi, DXF'in biçimleri (.run/shots/arac-tablo-formul-*, -baglanti-*, arac-dxf-bicimler-*), 1001 formüllü tablonun süreleri (ADR 0327)
python3 scripts/fixtures/text_command_cases.py --check   # alanların (değerler, kayma, izleme, döngü, kopuk) ve yazının komutlarının (Bul ve değiştir, Numarala, Birleştir, Satırlara ayır, Yazı denetimi) durumlarını KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.{fields.edit,texts.edit,texts.check}.json (ADR 0325)
KENTOS_SHOTS_ONLY=yazi-alanlar,yazi-alan-ekle,yazi-numarala,yazi-denetimi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # alanların ve yazı komutlarının resimleri, .run/shots/arac-yazi-* (ADR 0325)
python3 scripts/fixtures/paragraph_cases.py --check; python3 scripts/fixtures/font_report_cases.py --check   # yazının düzeninin yerleşimini (girintiler, asılı liste, sekmeler, kesirler, sütunlar) ve cad.fonts.report durumlarını KentOS kodu olmadan denetle; durumlar fixtures/text/v1/paragraph.json, fixtures/commands/v1/cad.fonts.report.json (ADR 0324)
cargo test -p kentos-desktop text_outputs; KENTOS_SHOTS_ONLY=yazi-duzeni,yazi-liste,yazi-tipleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # aynı yazılar .kcad, DXF ve paftanın PDF'iyle (pdftotext), resimler .run/shots/arac-yazi-* (ADR 0324)
KENTOS_SHOTS_ONLY=tarama-yuz-bagi,tarama-kesin-sinir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test -p kentos-domain --test all hatch_boundaries; cargo test -p kentos-formats --test dxf_write a_hatch_boundary   # taramanın kaynak sınırı (yay, elips, NURBS kesin; DXF HATCH kenarlarıyla) ve çizgilerle kapanan yüzün bağı, .run/shots/arac-tarama-yuz-bagi-*, -kesin-sinir-* (ADR 0328)
python3 scripts/fixtures/hatch_edit_cases.py --check; KENTOS_SHOTS_ONLY=tarama-ic-ice,tarama-bosluk,tarama-duzenle cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # cad.hatches.edit'in 40 durumu KentOS kodu olmadan; iç içe adalar, boşluğu kapanan oda ve Taramayı düzenle'nin resimleri, .run/shots/arac-tarama-* (ADR 0329)
python3 scripts/fixtures/hatch_pattern_file_cases.py --check; KENTOS_SHOTS_ONLY=tarama-desenleri,tarama-hizala,tarama-baslangic,tarama-olcekli,tarama-desen-proje cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # PAT okuma ve yazma, cad.hatchpatterns.edit'in 30 durumu KentOS kodu olmadan; Tarama desenleri, Hizala, Başlangıç, ölçekli tarama ve Desen menüsünün proje desenleri, .run/shots/arac-tarama-* (ADR 0331)
cargo test -p kentos-domain --test all hatch_texts; cargo test -p kentos-native-application --lib budget_tests; KENTOS_SHOTS_ONLY=tarama-saydam,tarama-yazi-tasindi,tarama-sira cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # boş bırakılan yazıların izlenmesi ve döngüsü, bütçe ve iptal; yarı saydam dolu tarama sınırının arkasında, maskenin örttüğü desen, taşınan yazının boşluğu ve Sıra menüsü, .run/shots/arac-tarama-* (ADR 0332)
python3 scripts/fixtures/constraint_cases.py --check; KENTOS_CONSTRAINTS_OUT=$PWD/.run/kisit.json cargo test -p kentos-geometry-core --test all constraints; python3 scripts/fixtures/constraint_cases.py --outputs .run/kisit.json   # kısıt çözücüsünün 33 durumu bağımsız başvurudan (sympy ve 50 basamaklı mpmath: rank kısıtların sırasıyla, çözümler, fazla, çelişen, tekil, kopuk, serbestlik), serbestliği kalan çözümler başvurunun denklemleriyle (ADR 0334)
cargo test --release -p kentos-geometry-core --lib constraints::tests::timing -- --ignored --nocapture   # 1000 ayrı ve 10 × 10 … 50 × 50 bağlı dikdörtgenin çözüm süreleri (ADR 0334 §6)
python3 scripts/fixtures/parameter_command_cases.py --check; KENTOS_TEST_DB=required cargo test -p kentos-application --test project_constraints   # cad.parameters.edit'in durumları KentOS kodu olmadan ve veritabanı projesinde kısıtlar (ADR 0336)
python3 scripts/fixtures/constraint_follow_cases.py --check; KENTOS_SHOTS_ONLY=kisit-izle-onizleme,kisit-izle,kisit-izle-celiski,kisit-gevset cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # düzenlemelerde kısıtların 22 ortak durumu (izleme, çelişki, gevşetme, silme, köşe silme, kırma, kopyalar, dizi, pano) KentOS kodu olmadan ve resimleri, .run/shots/arac-kisit-izle-*, arac-kisit-gevset-* (ADR 0337)
python3 scripts/fixtures/wall_cases.py --check; python3 scripts/fixtures/wall_command_cases.py --check   # duvarın çizimini (yüzler, bileşen sınırları, alan, üç ayrıntının parçaları) ve cad.walls.create, .convert, .edit durumlarını KentOS kodu olmadan denetle; durumlar fixtures/walls/v1/cases.json ve fixtures/commands/v1/cad.walls.*.json (ADR 0339)
KENTOS_SHOTS_ONLY=duvar-serit,duvar-ciz,duvar-yazildi,duvar-ayrintili,duvar-sematik,duvar-oznitelikler,duvar-stilleri,duvar-cizgiden,duvar-katlar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; KENTOS_TRACES_ONLY=wall-tool cargo test -p kentos-desktop every_trace_passes   # duvarın resimleri (.run/shots/arac-duvar-*) ve Duvar aracının izi (ADR 0339)
python3 scripts/fixtures/opening_cases.py --check; python3 scripts/fixtures/opening_command_cases.py --check   # açıklığın pervazlarını, simgesini ve duvarın boşluğunu (çekirdeğin testi tests/all/openings.rs) ve cad.openings.create ile .edit'in 86 durumunu (duvarla gitmek dahil) KentOS kodu olmadan denetle; durumlar fixtures/walls/v1/openings.json, fixtures/commands/v1/cad.openings.*.json (ADR 0341)
KENTOS_SHOTS_ONLY=acik-serit,acik-oda,acik-ayrintili,acik-kapi,acik-pencere,acik-oznitelikler,acik-tasi,acik-duvar-yok cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # açıklıkların resimleri, .run/shots/arac-acik-* (ADR 0341)
python3 scripts/fixtures/space_cases.py --check; python3 scripts/fixtures/space_command_cases.py --check   # mahalin sınırını (duvarların yüzleri, delik, L oda, eğik oda, kalınlık basamağı, üç yöntem; çekirdeğin testi tests/all/spaces.rs) ve cad.spaces.create, .edit ile .quantities'in 84 durumunu (duvar taşınınca ve kalınlaşınca, kapanmayan mahal dahil) KentOS kodu olmadan kesirlerle denetle; durumlar fixtures/walls/v1/spaces.json, fixtures/commands/v1/cad.spaces.*.json (ADR 0342)
KENTOS_SHOTS_ONLY=mahal-serit,mahal-oda,mahal-arac,mahal-brut,mahal-oznitelikler,mahal-bol,mahal-metraj,mahal-metraj-duvar,mahal-kayip cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # mahallerin ve Metraj'ın resimleri, .run/shots/arac-mahal-* (ADR 0342)
python3 scripts/fixtures/arch_cases.py --check; python3 scripts/fixtures/arch_command_cases.py --check   # aksın balonlarını, kolonun köşelerini, merdivenin basamaklarını, sahanlıklarını ve denetimlerini, rampanın kolunu, eğimini ve kotlarını kesirlerle (çekirdeğin testi tests/all/arch.rs) ve cad.grids.create, cad.columns.create, cad.stairs.create, cad.ramps.create ile cad.arch.edit'in 168 durumunu (kolonun aksla gitmesi dahil) KentOS kodu olmadan denetle; durumlar fixtures/arch/v1/cases.json, fixtures/commands/v1/cad.{grids,columns,stairs,ramps}.create.json ve cad.arch.edit.json (ADR 0343)
KENTOS_SHOTS_ONLY=aks-serit,aks-sistem,aks-arac,kolon-arac,kolon-oznitelikler,merdiven-arac,merdiven-denetim,rampa,aks-numarala,aks-izleme cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # aks, kolon, merdiven ve rampanın resimleri, .run/shots/arac-aks-*, -kolon-*, -merdiven-*, -rampa-* (ADR 0343)
python3 scripts/fixtures/tag_cases.py --check; python3 scripts/fixtures/tag_command_cases.py --check   # etiketin çizimini (çerçeve, satırlar, kol; yazı tipinin ölçülmüş ilerlemeleriyle) ve cad.tags.create, .edit, .check ile cad.schedules.create, .edit'in 162 durumunu (değerler, şablonlar, izleme, çizelgenin satırları, sıra, grup, toplam, hücreden kaynağa yazma, retler) KentOS kodu olmadan denetle; durumlar fixtures/arch/v1/tags.json, fixtures/commands/v1/cad.{tags,schedules}.*.json (ADR 0344)
KENTOS_SHOTS_ONLY=mimari-etiket-serit,mimari-etiket-plan,mimari-etiket-arac,mimari-etiket-oznitelikler,mimari-etiket-denetim,cizelge-arac,cizelge-izleme cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # etiketlerin ve çizelgelerin resimleri, .run/shots/arac-mimari-etiket-*, arac-cizelge-* (ADR 0344)
python3 scripts/fixtures/wall_join_cases.py --check   # duvar birleşimlerini (köşe, alın, düz devam, T ve öncelikli geçiş, oda, X, üçlü uç, dört sorun ve onarımları) kesirlerle, KentOS kodu olmadan denetle; durumlar fixtures/walls/v1/joins.json, cad.walls.edit ve .joins'in birleşim uyarıları da bundan (ADR 0340)
KENTOS_SHOTS_ONLY=duvar-birlesim,duvar-birlesim-ayrintili,duvar-birlesim-x,duvar-duzenle-serit,duvar-birlesim-araci,duvar-birlesim-denetle,duvar-birlesim-oznitelikler,duvar-bol cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; KENTOS_TRACES_ONLY=wall-edit cargo test -p kentos-desktop every_trace_passes   # duvar birleşimlerinin ve düzenlemelerinin resimleri ve izi (ADR 0340)
cargo test -p kentos-interaction --test all constraint_marks; KENTOS_SHOTS_ONLY=kisit-isaretler,kisit-isaret-vurgu,kisit-parametreler,kisit-parametre-celiski cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # kısıt işaretleri, bağlı parçaların vurgusu ve Parametreler penceresi, çelişen değer; resimler .run/shots/arac-kisit-isaret*, arac-kisit-parametre* (ADR 0338)
python3 scripts/fixtures/constraint_command_cases.py --check   # cad.constraints.edit ve .query durumlarını (yön, parametreler, tutma, retler; çözümler kısıt başvurusundan) KentOS kodu olmadan denetle (ADR 0335)
KENTOS_SHOTS_ONLY=kisit-paralel,kisit-deger,kisit-genislik,kisit-celiski,kisit-teget cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; KENTOS_TRACES_ONLY=constraint-tool cargo test -p kentos-desktop every_trace_passes   # Kısıt aracının resimleri (.run/shots/arac-kisit-*) ve ortak izi (ADR 0335)
python3 scripts/fixtures/dxf_hatch_cases.py --check; cargo test -p kentos-headless --test headless dxf_hatches_go_round   # DXF HATCH'in bağımsız dosyalarla gidiş-dönüşü: yollar, ilişkili taramanın kaynakları, reaktörleri ve tohumu, desen aileleri, degradeler (KENTOS_WRITE_DXF=1 çıktıları fixtures/formats/v1/dxf-hatch/out'a yazar; ADR 0333)
KENTOS_PLOT_OUT=$PWD/.run/shots cargo test -p kentos-plot --test all a_gradient_and_a_dense_hatch; pdftoppm -r 150 -png .run/shots/tarama-pdf.pdf .run/shots/tarama-pdf   # degradenin ve sık desenin PDF'te vektörleri (ADR 0333)
KENTOS_SHOTS_ONLY=olcu-bag,olcu-yakin,olcu-izleme,olcu-zincir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # bağ vurgusu (yakında kâğıt zeminde de), izleyen ölçüler ve zincir, .run/shots/arac-olcu-bag-*, -yakin-*, -izleme-*, -zincir-* (ADR 0320)
cargo test --release -p kentos-desktop label_picture::tests::timing -- --ignored --nocapture   # 1 300 eğik yazının resmi, haleyle ve halesiz (ADR 0055'in 17 Ekim eki)
KENTOS_SHOTS_ONLY=blok-dinamik,blok-kitaplik,blok-kitaplik-kategori cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test -p kentos-geometry-core --test all block_dynamic   # KentOS blok kitaplığı ve dinamik blokların değerleri (.run/shots/arac-blok-*), varyantlar çekirdekte (ADR 0318)
KENTOS_SHOTS_ONLY=blok-tutamac,blok-parametreler,blok-sabit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test -p kentos-interaction --test all param_grips; cargo test -p kentos-headless --test headless a_varied   # dinamik bloğun tutamaçları, Öznitelikler'deki değerleri ve Sabit bloğa çevir (.run/shots/arac-blok-tutamac-*, -parametreler-*, -sabit-*), tutamakla yazma, DXF'e varyant (ADR 0319)
KENTOS_SHOTS_ONLY=blok-oznitelik-secenekleri,blok-oznitelik-degerleri,blok-karti,blok-cizelgesi,blok-kartlar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test -p kentos-geometry-core --test all block_attrs   # öznitelik seçenekleri, Öznitelik değerleri, Blok kartı, Blok çizelgesi ve Bloklar panelinin kartları (.run/shots/arac-blok-*), seçeneklere göre yazılar ve Patlat (ADR 0317)
KENTOS_SHOTS_ONLY=etki-katman,etki-cizgitipi,blok-kitapligi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test -p kentos-sheet-ui --test screens adr_0314 -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all library_files::timing -- --ignored --nocapture   # Değişikliğin etkisi, Bloklar panelinin kitaplık işleri (.run/shots/arac-etki-*, arac-blok-kitapligi-*), haritanın katmanları (.run/shots/sheet-desktop/*-0314-*), 100 000 nesnede etkinin süresi (ADR 0314)
python3 scripts/fixtures/order_command_cases.py --check   # cad.entities.order durumlarını (öne, arkaya, üstüne ve altına; yer açmak için kayanlar, sınır, başvurunun kuralı, başka katman, kilitli) KentOS kodu olmadan ADR'nin kuralından denetle; durumlar fixtures/commands/v1/cad.entities.order.json (ADR 0260, 0310)
KENTOS_SHOTS_ONLY=maske-ciz,maske-oznitelikler,maske-ustune,maske-cbs cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all order_cmd::timing -- --ignored --nocapture   # maskenin ve çizim sırasının resimleri (.run/shots/arac-maske-*) ve 100 000 nesnede sıranın süreleri (ADR 0310)
python3 scripts/fixtures/clipboard_command_cases.py --check   # Panoya kopyala ve Yapıştır'ın durumlarını (kopyanın parçaları ve bağları, taban noktası, katman yolları, ad çakışmaları, blok olarak yapıştırma, birim, kilitli ve gizli katman, retler) KentOS kodu olmadan alışverişin bağımsız başvurusuyla denetle; durumlar fixtures/commands/v1/cad.entities.{copy,paste}.json (ADR 0309)
KENTOS_SHOTS_ONLY=pano-tabanla,pano-baska-cizim,pano-katmanlar,pano-blok cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all clipboard_cmd::timing -- --ignored --nocapture   # pano komutlarının resimleri (.run/shots/arac-pano-*) ve 100 000 nesnede kopyalama ve yapıştırma süreleri (ADR 0309)
KENTOS_SHOTS_ONLY=parca-buda,parca-budandi,parca-kir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # çok parçalı çizgide Buda ve Kır'ın resimleri, .run/shots/arac-parca-* (ADR 0307)
python3 scripts/fixtures/array_edit_cases.py --check   # Diziyi düzenle'nin durumlarını (yerleşim, ilk yer, Değiştir, Sil, Sıfırla, plan, retler) KentOS kodu olmadan denetle; durumlar fixtures/commands/v1/cad.arrays.edit.json (ADR 0301)
KENTOS_SHOTS_ONLY=dizi-duzenle,dizi-duzenle-adet,dizi-duzenle-taban cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all array_assoc::edit_timing -- --ignored --nocapture   # Diziyi düzenle'nin resimleri (.run/shots/arac-dizi-duzenle-*) ve 100 × 100 ızgarada süreleri (ADR 0301)
KENTOS_SHOTS_ONLY=dizi-iliskili,dizi-iliskili-izler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all array_assoc::timing -- --ignored --nocapture; KENTOS_TEST_DB=required cargo test -p kentos-application --test project_arrays   # ilişkili dizinin resimleri (.run/shots/arac-dizi-iliskili-*), 10 000 öğenin kurulması ve yolun izlenmesi, veritabanı projesinde diziler (ADR 0300)
KENTOS_SHOTS_ONLY=bol-blok,bol-blok-yazildi,bol-blok-hizasiz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all divide::timing -- --ignored --nocapture   # Böl'ün hizalı blokları, .run/shots/arac-bol-blok-*; 10 000 bloğun planı ve yazılması (ADR 0299)
KENTOS_SHOTS_ONLY=uzat-kisalt-toplam,uzat-kisalt-yay-aci,uzat-kisalt-elips,uzat-kisalt-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all lengthen::timing -- --ignored --nocapture   # Uzat-kısalt'ın komutla önizlemesi ve sonucu (.run/shots/arac-uzat-kisalt-*), 100 000 nesnede önizleme ve yazma süreleri (ADR 0298)
KENTOS_SHOTS_ONLY=esnet-izleyenler,esnet-yazildi,esnet-yay cargo test -p kentos-desktop tools_screens -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all stretch_ties::timing -- --ignored --nocapture   # Esnet'in izleyenlerle önizlemesi, sonucu ve yayı (.run/shots/arac-esnet-*), 100 000 nesnede planın süresi (ADR 0297)
KENTOS_TRACES_ONLY=usage-semantic-tools cargo test -p kentos-desktop every_trace_passes; ./target/debug/kentos-cad kullan usage-semantic-tools   # araçların anlamsal komutlarla önizlemesi ve yazması, .run/shots/kullanim/masaustu-usage-semantic-tools-* (ADR 0254, 0255)
python3 scripts/fixtures/dxf_units_cases.py --check; python3 scripts/fixtures/dxf_import_command_cases.py --check   # DXF'in biriminin kararını (fixtures/formats/v1/dxf-units: 8 dosya, 24 durum) ve file.import.dxf'in 36 durumunu KentOS kodu olmadan ADR'nin kuralından ve DXF başvurusundan denetle (ADR 0257)
cargo test -p kentos-desktop exchange::tests::unit_screens -- --ignored --nocapture   # DXF içe aktarma penceresinin Dosyanın birimi satırı (bildirilen, bildirmeyen, seçilen, sistemli, blok birimi), .run/shots/aktar-*-2[2-7]-dxf-* (ADR 0257 §4)
python3 scripts/fixtures/polyline_extras_cases.py --check   # cad.polyline.create'in kapalı ve genişlikli 22 durumunu (kenar başına büküm ve genişlik, saklanan biçim, retlerin sırası ve iletileri) KentOS kodu olmadan ADR'nin kuralından denetle; durumlar fixtures/commands/v1/cad.polyline.create.json'ın “Kapalı ve genişlikli:” durumları (ADR 0258)
python3 scripts/fixtures/units_change_cases.py --check   # Birimi değiştir'in durumlarını (her nesne türünün uzunluk ve uzunluk olmayan alanları, beş yönün çarpanları, metre sanılan milimetrelik oda, kilitli katman, retler) KentOS kodu olmadan ADR'nin kuralından denetle; durumlar fixtures/commands/v1/cad.units.change.json (ADR 0256)
cargo test -p kentos-desktop project::unit_change::tests::screens -- --ignored --nocapture   # Proje ayarları'nın birim sorusu, .run/shots/birim-sorusu-* (ADR 0256 §2)
python3 scripts/cad/manifest.py --check   # CAD işlem manifesti: her arayüz komutunun, aracının ve yönteminin düzeyi (catalog/v1/operations.json), Python ve MCP karşılığı ve CAD tarafının boşlukları; eşlenmemiş öğe, olmayan komut ya da işlem ve eski çıktı düşer; --check'siz docs/cad/manifest.{json,md}'yi yazar (ADR 0244)
cargo test --release -p kentos-formats --test all exif::timing -- --ignored --nocapture; cargo test --release -p kentos-native-application --test all info_templates -- --ignored --nocapture; cargo test --release -p kentos-desktop a_card_opens_in_a_frame -- --ignored --nocapture   # 1 000 fotoğrafın EXIF'i, 10 000 nesnenin kart metni ve bir kartın açılması bütçeleriyle (ADR 0225 §7); web'inkiler: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm:formats; ATTACH_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/attachments.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp apps/web/src/io/pkg/.stamp
python3 scripts/fixtures/generalize_cases.py --check   # genelleştirmenin durumlarını (Chaikin, iD'nin dik açılandırması, Ruas'ın bina sadeleştirmesi, parçalar ve delikler, katılma, kapsama grafı ve Visvalingam–Whyatt, kümelerin kapanışı elle çözülmüş alanlarla ve GEOS'la çapraz) KentOS kodu olmadan denetle; durumlar fixtures/processing/v1/generalize.json ve generalize.kcad (ADR 0221)
python3 scripts/fixtures/closing_cases.py --check   # dönük binaların 45 bloğunun kapanışını (dışa tampon GEOS'un, içe aşındırma kesin; kesişim çekirdeğinin bir kez yüz yitirdiği bloklar dahil) KentOS kodu olmadan denetle; --stress FILE çekirdeğin rastgele blokların kapanışlarını (KENTOS_CLOSINGS_OUT=FILE cargo test -p kentos-geometry-core --test all closings_of_random_blocks -- --ignored) karşılaştırır; durumlar fixtures/geoprocess/v1/closings.json (ADR 0221 §10)
cargo test -p kentos-desktop processing::generalize_tests::screens -- --ignored --nocapture --test-threads=1   # genelleştirmenin şeridi, pencereleri ve sonuçları, .run/shots/islem-gen-* (web'inkiler: (cd apps/web && node scripts/e2e/shots.mjs generalize); ADR 0221)
cargo test --release -p kentos-processing --test cases generalize_timing -- --ignored --nocapture   # 10 000 parselde ortak kenarlarla yumuşatma, 10 000 binada dik açılandırma, sadeleştirme ve kümeleme, 100 000 parselde parça ayıklama, katma ve kapsama sadeleştirmesi bütçeleriyle (ADR 0221 Doğrulama; KENTOS_TIMING_ONLY); web'inki: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; GENERALIZE_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/generalize.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
cargo test --release -p kentos-geometry-core --test all proximity::timing -- --ignored --nocapture   # beş aracın aramaları 10 000 parselde bütçeleriyle (ADR 0215 §6); web'inkiler: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; PROXIMITY_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/proximity.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
cargo test --release -p kentos-expression --test perf measures_the_additions -- --ignored --nocapture   # ifade dili eklerinin süreleri bütçeleriyle (ADR 0214 §5); web'inkiler: KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; EXTRAS_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/expressionExtras.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
python3 scripts/fixtures/data_search_cases.py --check   # Veride ara'nın eşleşmesini (Türkçe katlama, `*`, Tam sözcük), alan seçimini, “+n” sayısını, sıralamayı, sınırı, nesneden kaydı ve öznitelik adlarını KentOS kodu olmadan yazılmış başvurudan denetle; durumlar fixtures/search/v1/cases.json (ADR 0178)
python3 scripts/fixtures/template_layer_cases.py --check   # nesne şablonunun katmanını bulma ve açma kuralını (yol tercihi, kilitli grup, açılacak gruplar) kurallardan denetle; durumlar fixtures/style/v1/template-layers.json (ADR 0176 §3)
python3 scripts/fixtures/template_from_object_cases.py --check   # Seçili nesneden şablon'u (araç, katman yolu ve görünüşü, noktanın adı ve kodu, yazının kâğıttaki yüksekliği, bloğun adı, retler) kesirle kurallardan denetle; durumlar fixtures/style/v1/template-from-object.json (ADR 0176 §4)
python3 scripts/fixtures/template_form_cases.py --check   # Şablon düzenleyicinin form kuralını (alanlar, öznitelik satırları, yazı ve blok, grup şablonunun üye satırları) kurallardan denetle; durumlar fixtures/style/v1/template-form.json (ADR 0176 §4, §5)
python3 scripts/fixtures/template_member_cases.py --check   # grup şablonunun üyelerinin geometrisini (açık şeklin sol ve sağ, kapalı şeklin iç ve dış ötelemeleri, keskin köşe, yarım çember, retler; ağırlık merkezi) kesin kesirlerle bağımsız başvurudan denetle; durumlar fixtures/template-members/v1/cases.json (ADR 0176 §5)
python3 scripts/fixtures/point_editor_cases.py --check   # Nokta editörünün hesaplarını (doğal sıra, tablonun süzgeç ve sıralaması, çift noktalar, bağlı köşeler) kurallardan denetle (ADR 0153)
python3 scripts/fixtures/point_edit_cases.py --check   # Nokta editörünün düzenlemelerini (hücreler, bağlı çizgiler, taslak satır) kurallardan denetle (ADR 0153 §3–§4)
python3 scripts/fixtures/point_batch_cases.py --check   # Nokta editörünün toplu işlemlerini (Yeniden adlandır, Sıralı numara ver, Katmana taşı, hedef satırlar) kurallardan denetle (ADR 0153 §5)
python3 scripts/fixtures/fit_cases.py --check   # Vektör oturtma'nın çözümünü (Helmert, afin, projektif; artıklar, m0, çözümsüzlükler) tam kesirle bağımsız başvurudan denetle (ADR 0156)
python3 scripts/fixtures/warp_cases.py --check   # Vektör oturtma'da nesnelerin dönüşmesini (afinde elips, projektifte 0,1 mm'lik köşeler, yazı ve blok kuralı, ufuk reddi) kurallardan denetle (ADR 0156)
python3 scripts/fixtures/rubber_warp_cases.py --check   # Kauçuk levha'da nesnelerin kurallarını (yalnız köşeler, en yakın benzerlik, sapma) bağımsız başvurudan denetle (ADR 0158 §3)
python3 scripts/fixtures/rubber_cases.py --check   # Kauçuk levha'nın ince plaka eğrisini (görüntüler, türev, çözümsüzlükler) mpmath ile 50 basamaklı bağımsız başvurudan denetle (ADR 0158)
python3 scripts/fixtures/edgematch_cases.py --check   # Kenar eşleme'nin bağlarını (aday, puan, bire bir eşleme, kavşak, eşsiz uç) ve yöntemlerini (Ucu taşı, Parça ekle, Köşeleri ayarla; üç buluşma yeri, kotlar) mpmath ile 50 basamaklı bağımsız başvurudan denetle (ADR 0159)
python3 scripts/fixtures/topology_edit_cases.py --check   # Topolojik düzenlemenin kurallarını (ortak köşe ve kenar, taşıma, köşe ekleme, kabarıklık, köşe silme, kilitli ve geçersiz komşu, düzenlemenin öncesinden ve sonrasından değişiklikler) kesin kesirlerle denetle (ADR 0160)
python3 scripts/fixtures/cogo_cases.py --check   # Kayıtlı ölçülerin kurallarını (çizginin ve yayın semti ve uzunlukları, denetimin farkları cc ve metrede, semtin dönüşü, okunamayan kayıt, kutupsal yazılanın kaydı: virgül, artı, yerel projenin mm ve cm'si) mpmath ve kesirle bağımsız başvurudan denetle; durumlar fixtures/cogo/v1/cases.json (ADR 0180)
python3 scripts/fixtures/navigation_cases.py --check   # Genel bakış ve Büyüteç'in kurallarını (kartların yeri ve büyütecin yan değiştirmesi, kapsamın resme sığdırılması, görünüm çerçevesi, basılan yer) ve Genel bakışın resmini (piksel kuralları: Bresenham, çift-tek dolgu, noktalar; düz geometride kesin, eğrilerde piksel kenarından uzak) bağımsız başvurudan denetle; durumlar fixtures/navigation/v1/cases.json (ADR 0181)
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
cargo test -p kentos-sheet-ui --test screens adr_0206 -- --ignored --nocapture   # haritanın ölçek düğmeleri ve koordinat listesinin kaynağı, .run/shots/sheet-desktop/*-0206-* (web'inkiler: node apps/web/scripts/e2e/sheet-shots.mjs --only harita-olcek-dugmeleri,koordinat-listesi-kaynak; ADR 0206)
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
  `~/.local/state/kentos-cad/`; yalnız yollar ve kısa bilgi, ADR 0050). Kaynaklar'ın klasörleri web'de IndexedDB
  `kentos.sources/folders`'ta (tarayıcının klasör tutamaçları), masaüstünde yanında `kaynak-klasorleri.json`'dadır (yollar; ADR 0199 §7). Masaüstünün Kitaplığım'ı
  (web'de `kentos.styles.v1`) `$XDG_DATA_HOME/kentos-cad/kitaplik.kstil`'dedir; okunamayan dosya
  `kitaplik-okunamadi-<zaman>.kstil` olarak ayrılır, üzerine yazılmaz (ADR 0092).
  Yerel çizimin kaydedilmemiş işinin kurtarma kopyaları IndexedDB `kentos.recovery/copies`'tedir;
  masaüstünde `$XDG_DATA_HOME/kentos-cad/kurtarma` (yoksa `~/.local/share/kentos-cad/kurtarma`)
  altında, her çalışan KentOS'un kilitli klasöründe (ADR 0030). Pafta kitabı masaüstünde projenin `.kcad`'indedir
  (ADR 0251); bu cihazdaki pafta şablonları ve şablon resimlerinin baytları `$XDG_DATA_HOME/kentos-cad/pafta/`'dadır (yoksa
  `~/.local/share/kentos-cad/pafta/`; ADR 0164); oradaki `kitaplar/<anahtar>.json` eski sürümlerin kitaplarıdır, proje açılınca
  projeye alınır ve kaydedilince `kitaplar/tasindi/`'ye ayrılır, silinmez. Web'in kitapları, şablonları ve resimleri hâlâ IndexedDB
  `kentos.sheets.v1`'dedir (sürüm 2: `books`, anahtarı `bulut/…`, `proje/…`, `dosya/…`, `oturum/…`; `templates`, hesabın bulut
  kitaplığının kopyaları da; `assets`, anahtarı SHA-256, yazılırken özeti denetlenir; sürüm 1'den yükseltme yalnız eksik depoyu
  açar; göçü TODOS.md §0.2'de). NTv2 ızgaraları cihazındır, projeninki değil: IndexedDB `kentos.grids` (`grids` bilgiler, `bytes` baytlar; anahtar
  SHA-256), masaüstünde `$XDG_DATA_HOME/kentos-cad/izgara/<sha256>.gsb` ve yanında `<sha256>.json` (ADR 0168 §4).
  Harita servislerinin bağlantılarının gizli değerleri çizime yazılmaz: web'de localStorage `kentos.connections.v1`, masaüstünde
  `~/.config/kentos-cad/baglantilar.json` (0600); okunamayan kayıt `…#unreadable-<zaman>` olarak ayrılır. Masaüstünün servis önbelleği
  `$XDG_CACHE_HOME/kentos-cad/servis/` (en çok 2 GB; ADR 0208 §3).
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
çalıştırın; farkı okuyup değişiklikle aynı commit'e koyun (docs/inventory/README.md). Web dururken (ADR 0228) envanter donmuştur;
yeni komutların kaydı `catalog/v1/additions.json`'dadır, web'de gerekenler TODOS.md §0.2'ye yazılır.
Çizimi ya da projeyi değiştiren yeni araç ve komut, §9.2'deki komut, Python ve MCP bütünlüğü olmadan tamamlandı sayılmaz.

## 9. Yeni iş ekleme tarifleri

### 9.1 Komut

Registry/handler, enable/watch, gerekiyorsa alias/kısayol ve menü kaydı ekleyin;
aynı iş için UI ve otomasyonda ayrı iş kuralı yazmayın. Belgeyi değiştiren ürün komutu §9.2'deki bütünlükle gelir: katalog
ve yetenekler, ortak durumlar ve bağımsız başvuru, Python SDK, MCP, manifest.

### 9.2 Araç

Mevcut araç ailesini seçin; `tools/catalog.ts`, `steps`, preview/input/iptal,
kilitli katman ve tek undo testlerini tamamlayın. Hesabı ortak Rust'a ekleyin.

**Araç = komut (sahibin kararı, 13 Ekim 2026).** Çizimi ya da projeyi değiştiren her yeni araç işini kendi anlamsal ürün
komutuyla yapar; aynı işi Python ve yapay zekâ ajanı, geometriyi kendileri hesaplamadan, aynı komutla yapar. Bunlardan biri
eksikse araç bitmiş sayılmaz:

- **Anlamsal komut:** girdisi kullanıcının verdiğidir (nesnelerin kalıcı kimlikleri, yerleri, tıklanan noktalar, sayılar,
  seçenekler), sonucu komut hesaplar; `validate`, `plan`, `execute`, `CommandResult`, protokolün kaydı ve tekrar oynatması
  (ADR 0250, 0253, 0254, 0255). Hazır geometri alan `cad.entities.create` ve `.edit` işlemleri yalnız komutun iç yazma yoludur.
- **Araç komutu kullanır:** önizleme komutun planından ya da onun hesabından (`plan_in`, `measured_in` gibi), yazma komutun
  `execute`'undan; hesap araçta ikinci kez yazılmaz. Aracın görünümü, seçimi ve seçenekleri girdinin açık alanlarıdır (CMD-07).
- **Katalog:** tanım, girdi, çıktı ve plan şemaları, örnekler, `capabilities` (hedef türler, ret ve uyarı kodları; anlamları
  `codes.rs`'te); `DESKTOP_COMMANDS`, `registry.rs`; başsız sunucu kayıttan çalıştırır.
- **Ortak durumlar:** `fixtures/commands/v1/<komut>.json`, en az 20 durum, KentOS kodu olmadan yazılmış bağımsız başvurudan
  (`scripts/fixtures/<konu>_cases.py --check`); yetenek testi (`capabilities.rs`: her kod bir durumda ya da gerekçesiyle, tür
  denemeleri) geçer.
- **Python ve MCP:** `python3 scripts/python/sdk.py` ile `kentos.cad` yeniden üretilir ve `pnpm py:test` geçer (ortak durumlar
  tipli katmandan da); MCP aracı katalogdan gelir, var olanı değiştiren komut `crates/native/mcp/src/tools.rs`'in yıkıcı
  listesine yazılır.
- **Manifest:** aracın düzeyi `catalog/v1/operations.json`'da `semantic`, `via`'sı komut; `python3 scripts/cad/manifest.py`
  (ADR 0244). Yeni araç `geometry` ya da `none` düzeyinde gelmez.
- **Kanıt:** izler değişmeden geçer; araç komutla çalışırken resimlenir (`kentos-cad kullan …`); aracın yazması kayıtta komutun
  adıyla görünür. Web dururken (ADR 0228) web'in işleyicisi ve aracı TODOS.md §0.2'ye yazılır.

Var olan `geometry` ve `none` düzeyindeki araçlar borçtur: `docs/cad/manifest.md` onları sayar; dokunulan araç bu kurala getirilir.

### 9.3 Ayar

Önce kapsamını belirleyin (§4.4); tip, varsayılan, validasyon, migration,
kalıcılık ve live-apply davranışını birlikte ekleyin. Ayar
`crates/shared/contracts/src/settings/schema.rs`'e eklenir, `KENTOS_WRITE_SETTINGS=1 cargo test
-p kentos-contracts settings` ile üretilir; yeni kural `fixtures/settings/v1` ve iki
çalıştırıcıyla birlikte değişir (ADR 0023).

### 9.4 Test

- Web davranışında `pnpm typecheck`, ilgili Vitest testleri ve kapsamına göre
  `pnpm test/build`; UI değişiminde tarayıcı e2e ve açık/koyu/Büyük yazı kontrolü. Web dururken (ADR 0228) bunlar koşmaz.
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
Bir maddenin baştan sona nasıl yapıldığı (sıra, denetimler, resimler, commit, paralel ajanların
numaraları ve çakışan dosyaları): [docs/MADDE-TARIFI.md](docs/MADDE-TARIFI.md).

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
  adımda; ortak iz `labels-to-text.json`) tamam; 4. adım (nesneye bağlı yazı) üç parçada: 4a (yazının `labelOf` ve `labelScale`'i, `.kcad`
  şema 18) ve 4b (çekirdeğin `label_text_of`'u, iki belgede kayıt öncesi izleme: web `model/linkedTexts.ts`, masaüstü `kentos_domain` `linked.rs`;
  ortak belge durumları `linked-texts.json`) ve 4c (komutlarda bağ, `invalid_link`, `link_not_found`, Bağı kopar; araçta Nesneye bağlı (B);
  çizimde etiketin yerini alma, deponun `set_text_labelled`'ı; ortak iz `labels-linked.json`; Öznitelikler'de Bağlı nesne; DXF penceresinin
  notu) tamam; `HYB-18` bitti (5 Ekim). Sıradaki `HYB-19` nesne şablonları ([ADR 0176](docs/adr/0176-object-templates.md); TODOS'taki “çizim
  kalemleri”, ad sahibin seçimi): şablon stil kitaplığının üçüncü öğe türü, şablonla çizmek, Şablonlar paneli, grup şablonu, Şablonu uygula;
  BÖHYY takımı sahibin tarifini bekler. 1. adım (şablonun kuralları `model/objectTemplate.ts`, `kentos_native_style::object_template`, ortak
  `object-templates.json`; kitaplıkta `template`, `.kstil` sürüm 2; Stil yöneticisinde Şablon türü) ve 2. adım (oluşturma komutlarının
  `symbol`'ü, eksik olanlarda `label`; ortak komut durumları) ve 3a (şablonla çizmek: katmanın kuralı `templates::find_layer`,
  `templateLayer`, başvuru `template_layer_cases.py`; koşu masaüstünde `templates.rs`, web'de `app/objectTemplates.ts`; damga
  `Context::template`, `tools/templateStamp.ts`; `template.draw`; ortak iz `template-draw.json`) ve 4a (Şablonlar paneli: masaüstü
  `templates_panel.rs`, web `ui/templates/TemplatesPanel.ts`; liste kuralı `object_template::listed`, `style/templateList.ts`, ortak
  `template-list.json`; `template.panel`) ve 4b (Şablon düzenleyici: masaüstü `template_editor.rs`, web `ui/templates/TemplateEditor.ts`;
  kuralları `template_form`, `model/templateForm.ts`; Seçili nesneden şablon `object_template::from_object`, `templateFromObject`; ortak
  `template-form.json`, `template-from-object.json`) ve 4c (şeridin Giriş'inde Şablonlar paneli: web `templateField`, masaüstü
  `templates_group`; dar kademede Özellikler'in alanları adları yerine ikonlarıyla, KentOS UI `Choice::label_icon`, web `dropdown--glyph`)
  ve 3b (nokta, yazı ve blok şablonları: masaüstü `seed_tool`, `give_tool_back`, web `tools/templateSeeds.ts`; ad dizisi şablon başına;
  bloğu olmayan şablon başlamaz; nesneden şablonda yazı kâğıtta mm; ortak iz `template-tools.json`) ve 5a (grup şablonunun modeli:
  `members`, `templateIssues`/`template_issues`, kitaplıkta `memberIssues`/`member_issues`, formun `MemberRow`'u; ortak
  `template-groups.json`) ve 5b (üyelerin geometrisi: çekirdek `ops::template_members`, işlemler `templateMemberOffsets` ve
  `templateMemberCentroid`, web `model/ops/templateMembers.ts`; başvuru `template_member_cases.py`) ve 5c (grup şablonuyla çizmek: web
  `tools/templateMembers.ts`'in `withMembers`'ı grup araçlarının yazma yerlerinde, masaüstünde `with_tool`'un belge grubu ve
  `template_members.rs`; belgenin `nextSlot`/`next_slot`'u; ortak iz `template-group.json`) ve 5d (düzenleyicide Grup üyeleri tablosu)
  ve 6. adım (Şablonu uygula: kural `templateApplication`, `object_template::application`, ortak `template-apply.json`; `cad.entities.set`'in
  `template` işlemi; web `applyTemplate`, masaüstü `apply_template`; `template.apply`, Şablonlar panelinde ve Stil yöneticisinde “Seçili
  nesnelere uygula”; ortak iz `template-apply.json`, izlerin `applyTemplate` eylemi) tamam; `HYB-19` bitti (5 Ekim; BÖHYY takımı sahibin
  tarifini bekler). Sıradaki `HYB-20` katman yönetimi ekleri ([ADR 0177](docs/adr/0177-layer-management-extras.md)): nesneden katman
  işlemleri, Katmanı eşle ve Katmana kopyala, Kopyasını oluştur ve Katmanları birleştir, Katman durumları (`.kcad` şema 19),
  Kullanılmayanları temizle, Katman listesi. 1. adım (nesneden katman işlemleri: web `tools/layerTools.ts`, masaüstü
  `kentos_interaction::layer_tools`; iki belgede `isolateLayers`/`isolate_layers`; Yalıtımı kaldır `layer.unisolate`; şeridin yerleşik
  panelinin ▾'i `under`; izlerin `hiddenLayers` ve `lockedLayers`'ı; ortak iz `layer-by-object.json`) ve 2. adım (Katmanı eşle ve
  Katmana kopyala: web `tools/layerMoveTool.ts`, masaüstü `kentos_interaction::layer_move`; ortak iz `layer-move.json`) ve 3. adım
  (Kopyasını oluştur ve Katmanları birleştir: web `app/layerActions.ts`, `ui/layers/MergeLayersDialog.ts`, masaüstü `layer_merge.rs`;
  ortak iz `layer-merge.json`) ve 4a (katman durumları: `ProjectSettings.layerStates`, `.kcad` şema 19, `layer-states.kcad`) tamam;
  kalanı tek parçada (sahibin 5 Ekim kararı: “her maddeyi tek parçada bitir”, bundan sonraki maddeler alt adımlara bölünmez): Katman
  durumları penceresi ve Katmanlar panelinin Katman durumları ▾'i (web `app/layerStates.ts`, `ui/layers/LayerStatesDialog.ts`, masaüstü
  `layer_states.rs`; kurallar `kentos_domain::layer_states`, `model/layerStates.ts`), Kullanılmayanları temizle (kural
  `kentos_domain::layer_purge`, `model/layerPurge.ts`: kalanların kullanmadığı en büyük küme; web `ui/layers/PurgeDialog.ts`, masaüstü
  `layer_purge.rs`), Katman listesi (pano ve CSV; web `app/layerList.ts`, masaüstü `layer_list.rs`); ortak durumlar `fixtures/layers/v1`
  bağımsız Python başvurularıyla, ortak izler `layer-states.json` ve `layer-purge.json`, izlerin `layers` beklentisi; web kaydının
  katman durumlarını ve katmanın kendi kenetini düşürmesi düzeldi (`io/kcad.ts` `projectHead`); `HYB-20` bitti (5 Ekim). `HYB-21`
  veri karşılaştırma ([ADR 0179](docs/adr/0179-data-compare.md)) tek parçada bitti (5 Ekim): çekirdek `ops::compare` (işlem
  `dataCompare`, bağımsız başvuru `compare_cases.py`, ortak durumlar `fixtures/compare/v1`), web `app/dataCompare.ts`,
  `ui/data/DataCompareDialog.ts`, masaüstü `data_compare.rs`; ortak iz `data-compare.json`. `HYB-22` veride arama
  ([ADR 0178](docs/adr/0178-data-search.md); sahibin izniyle bir kezlik alt ajanın dalında yapıldı, `main`'e birleşti) tek parça, iki
  platformda bitti (5 Ekim): çekirdek `text::edit::matches` ve `ops::data_search` (bağımsız başvuru `data_search_cases.py`, ortak
  `fixtures/search/v1`), web `ui/bottom/SearchPanel.ts` ve `model/dataSearch.ts`, masaüstü `search/` ve `kentos_interaction::data_search`;
  ortak iz `data-search.json` (oynatıcılarda `panel` eylemi ve `panel`, `search`, `mark` beklentileri). `HYB-23` kayıtlı ölçü (COGO)
  öznitelikleri ([ADR 0180](docs/adr/0180-recorded-measurements.md)) tek parçada bitti (6 Ekim): çekirdek `ops::cogo` (işlemler
  `cogoMeasure`, `cogoCheck`, `cogoRecord`; bağımsız başvuru `cogo_cases.py`, ortak `fixtures/cogo/v1`), Çizgi aracının kaydı (web
  `drawTools.ts`'in `typedText`'i, masaüstü `line.rs`'in `typed`'ı), web `app/cogo.ts` ve `ui/cogo/CogoCheckDialog.ts`, masaüstü
  `cogo.rs`; ortak iz `cogo.json`. `HYB-25` gezinme ekleri ([ADR 0181](docs/adr/0181-overview-and-magnifier.md)) tek parçada bitti
  (6 Ekim): çekirdek `store::overview` (Genel bakışın kapsamı ve resmi, web'e `overviewExtent`, `overviewPicture`) ve `tools::navigation`
  (kartların yeri, eşleme; web `viewport/navigation.ts`), bağımsız başvuru `navigation_cases.py` (ortak `fixtures/navigation/v1`, resim
  piksel piksel); büyüteç çizim hatlarında ikinci kamera (web `FrameState.lens`, masaüstü `Renderer::prepare_lens`/`draw_lens`, aynı
  ilkel türüyle); kartlar web `viewport/navigationCards.ts`, masaüstü `navigation_cards.rs`; yerleşimde `overview`, `magnifier`,
  `magnifierZoom`; ortak iz `overview.json`. §16.0 bitti; §16.1 (CAD) başladı: `CAD-18` kroki yazımı **[M]** (gösterim kurallarını
  sahip tarif edecek) beklerken `CAD-20` çok satırlı yazı ([ADR 0182](docs/adr/0182-paragraph-text.md)) tek parçada bitti (6 Ekim):
  sözleşmenin `Paragraph`'ı (`boxWidth`, `lineSpacing`, `runs`; `.kcad` şema 20, `FORMATS_VERSION` 30), çekirdek `text::paragraph`
  (satırlar, sarma, kalın tablo, düzenleyicinin dilimleri, `corner_box`; bağımsız başvuru `paragraph_cases.py`, ortak
  `fixtures/text/v1/paragraph.json`), deponun `LABEL_LINE`, `LABEL_PARAGRAPH_MASK`, `LABEL_PIECE_LINE` kayıtları; araç web
  `tools/paragraphTool.ts`, masaüstü `kentos_interaction::paragraph`; düzenleyici web `ui/shell/ParagraphEditor.ts`, masaüstü
  `paragraph_editor.rs`; DXF MTEXT biçimleriyle iki yönde (`dxf/strings.rs`'in `mtext_content`'i, `writer/entities/paragraph.rs`;
  `mtext.dxf`, `dxf-write/paragraphs`); `invalid_paragraph`; ortak iz `paragraph-text.json` (oynatıcılarda `paragraph` eylemi).
  `CAD-21` yazı ve ölçü stilleri ([ADR 0183](docs/adr/0183-text-and-dimension-styles.md)) tek parçada bitti (6 Ekim): sözleşmenin
  `TextFace`, `DimensionLook`, `TextStyleDef`, `DimensionStyleDef`'i ve kuralları (`annotation.rs`; bağımsız başvuru
  `annotation_style_cases.py`, ortak `fixtures/text/v1/styles.json`), `.kcad` şema 21 (`FORMATS_VERSION` 31), `cad.entities.edit`'in
  `textStyle` ve `dimensionStyle` işlemleri; pencere masaüstünde `annotation_styles.rs`, web'de `ui/annotation/StylesDialog.ts`
  (kaydetme `kentos_interaction::style_tables`, `app/styleTables.ts`); araçların Stil'i `kentos_interaction::styles`,
  `tools/styleOption.ts` (stil adının adımında Boşluk harftir: `takes_words`, `takesWords`); DXF'te `dxf/styles.rs`,
  `writer/styles.rs`, içe aktarmanın birleştirmesi `exchange/apply.rs`, `io/apply.ts`; ortak izler `text-styles.json`,
  `dimension-styles.json` (oynatıcılarda `face`, `look`). `CAD-22` tablo ([ADR 0184](docs/adr/0184-tables.md)) tek parçada bitti
  (6 Ekim; sahibin eki: kalın çerçeve): sözleşmenin `TableEntity`'si ve kuralları (`table.rs`), `.kcad` şema 22 (`FORMATS_VERSION` 32),
  çekirdek `geom::table`, `ops::table`, `ops::table_edit` (bağımsız başvuru `table_cases.py`, ortak `fixtures/table/v1/cases.json`),
  dosya okuyucusu `formats::table_file` (`table_file_cases.py`, `fixtures/table/v1/files`); pencereler masaüstünde `tables/`, web'de
  `ui/table/`, kuralları `kentos_interaction::table` ve `app/tables.ts`; yerleştirme `kentos_interaction::table_place`,
  `tools/tablePlaceTool.ts`; DXF adsız blok ve KENTOS verisi (`dxf-write/tables`); ortak iz `table.json` (oynatıcılarda `table`
  beklentisi, ok tuşları). `CAD-19` koordinat yazımı ([ADR 0185](docs/adr/0185-coordinate-labels.md)) tek parçada bitti (6 Ekim; ikonlar
  sahibin seçimi, yeni ikonlarda seçenekler sunulur): çekirdek `ops::coordinate_labels` (yerler ve adlar koordinat çizelgesiyle ortak, şablon,
  yerleşim; bağımsız başvuru `coordinate_label_cases.py`, ortak `fixtures/coordinate-labels/v1/cases.json`), `cad.entities.create`'in
  `coordinates` işlemi; araçlar masaüstünde `kentos_interaction::coordinate_labels`, web'de `tools/coordinateLabelTool.ts`; Çizelge Tablo
  ekle'nin yerleştirmesiyle (`ViewChange::PlaceTable`; web'in tablo yapma yardımcıları `tools/newTable.ts`); ortak iz `coordinate-labels.json`
  (oynatıcılar `{`, `}`, `|`, `=`'i yazar). `CAD-23` tarama ekleri ([ADR 0186](docs/adr/0186-hatch-extras.md)) tek parçada bitti (6 Ekim;
  ikonları sahibin seçimi): sözleşmenin `hatch` modülü (desen ve degrade türleri, `PatternLine`, `HatchGradient`, `HatchAssoc`), `.kcad`
  şema 23 (`FORMATS_VERSION` 33), çekirdek `geom::hatch_pattern` (kitaplık, boyalar, parçalar, `carried`), `ops::hatch_region`,
  `tools::hatch` (bağımsız başvuru `hatch_pattern_cases.py`, ortak `fixtures/hatch/v1/cases.json`); stil motorunda `hatchFill`'in
  `stagger`'ı ve `gradientFill` (çerçevesi partide çapaya göre), WGSL sözleşmesi 4. sürüm; ilişkili taramanın izlenmesi masaüstünde
  `kentos_domain` `hatch_ties.rs`, web'de `model/hatchTies.ts` (ortak `hatch-ties.json`); araçlar `kentos_interaction::hatch_options`,
  `hatch_selected`, web `tools/hatchOptions.ts`, `tools/hatchSelectedTool.ts`; Öznitelikler `properties::hatch_rows`,
  `ui/properties/hatchRows.ts`; Desen menülerinde 56 × 24'lük geniş örnekler (sahibin seçimi; KentOS UI menüsünün `preview`'ü ve
  `Glyph::wide`'ı, web `iconPreview`); DXF `dxf/emit/pattern.rs` ve yazıcı (`hatch-patterns.dxf`, `dxf-write/hatches`); ortak izler `hatches.json`,
  `hatch-patterns.json` (oynatıcılarda taramanın `pattern` ve `assoc` beklentisi). `CAD-25` seçim ekleri
  ([ADR 0187](docs/adr/0187-selection-extras.md)) tek parçada bitti (6 Ekim; ikonlar sahibin seçtikleri): çekirdekte `Store::hits` (`hit` aynı
  puandan) ve `store/polygon.rs` (`in_polygon`, `ring_problem`; bağımsız başvuru `selection_cases.py`, ortak `fixtures/selection/v1`);
  seçimin önceki seti ve çipi (`Selection`'ın `previous`, `Cycle`'ı; web `model/selection.ts`), süzgeç (`drafting.selectFilter`;
  `kentos_interaction::selectable`, `tools/selectable.ts`), araçlar `select_polygon.rs`, `select_similar.rs`, web `PolygonSelectTool`,
  `tools/selectSimilarTool.ts`; komutlar `selection_commands.rs`, `app/selectionCommands.ts`; çip `selection_chip.rs` (KentOS UI menüsünün
  `highlight`'ı, `beside`'ın menü iletimi), `ui/shell/SelectionChip.ts`; ortak iz `selection-extras.json` (oynatıcılarda `cycle`). `CAD-26`
  nokta hesaplayıcı ekleri ([ADR 0188](docs/adr/0188-point-calculator-extras.md)) tek parçada bitti (6 Ekim; ikonlar önerilen A seçenekleri,
  sahip uyurken): çekirdek `tools::point_calc` (`route_of`, `station`, `reading`, `km_value`, `km_text`, `slope`, `bisector`; eğride ayak
  eğrinin kendisinden; bağımsız başvuru `point_calc_cases.py`, ortak `fixtures/point-calc/v1`), masaüstünde `point_calc.rs` ve
  `point_calc/route.rs`, web'de `tools/pointCalc.ts` ve `tools/pointCalcRoute.ts`; adlı nokta `session::named_point_at`, `namedPointAt`;
  KentOS UI menüsünün kısayol sütunu ölçülen genişlikle; ortak iz `point-calc-extras.json` (oynatıcılarda `%`). `CAD-27` Km yaz
  ([ADR 0189](docs/adr/0189-stationing.md)) tek parçada bitti (6 Ekim; ikon sahip uyurken seçenek sayfasının B'si): çekirdek `ops::stationing`
  (bağımsız başvuru `stationing_cases.py`, ortak `fixtures/stationing/v1`), `CreateOperation::Stations`, masaüstü
  `kentos_interaction::station_labels`, web `tools/stationLabelTool.ts`; ortak iz `stationing.json`. Sahibin 6 Ekim gecesi kararı: CAD-36
  bitene kadar durmadan sırayla (`CAD-24` [M] sahibin tarifini bekler). `CAD-28` Orta hat ([ADR 0190](docs/adr/0190-centerline.md)) tek
  parçada bitti (6 Ekim; ikon seçenek sayfasının D'si): çekirdek `ops::centerline` (güzergâhın `Walk::edges`, `vertex_lengths`; bağımsız
  başvuru `centerline_cases.py`, ortak `fixtures/centerline/v1`), `CreateOperation::Centerline`, masaüstü `kentos_interaction::centerline`,
  web `tools/centerlineTool.ts`; ortak iz `centerline.json`. `CAD-29` Paralel kaydır ([ADR 0191](docs/adr/0191-parallel-shift.md)) tek
  parçada bitti (7 Ekim; ikon seçenek sayfasının B'si): çekirdek `ops::edge_shift` (`shifted`, `for_area`, `pick`; bağımsız başvuru
  `edge_shift_cases.py`, ortak `fixtures/edge-shift/v1`), `EditOperation::EdgeShift` (köşeler kotlarını yerlerinden alır), alan biriminden
  m²'ye `Format::area_to_square_metres`, `Formatter.areaToSquareMetres`; masaüstü `kentos_interaction::edge_shift`, web
  `tools/edgeShiftTool.ts`; ortak iz `edge-shift.json`. `CAD-30` Resim nesnesi ([ADR 0192](docs/adr/0192-image-object.md)) tek
  parçada bitti (7 Ekim; ikon seçenek sayfalarının A'ları): sözleşmenin `ImageEntity`'si ve kuralları (`image.rs`), `.kcad` şema 24
  (`FORMATS_VERSION` 34), çekirdek `geom::image`, `ops::image` (bağımsız başvuru `image_cases.py`, ortak `fixtures/image/v1`); stil
  motorunun `MODE_IMAGE`'i, stilli çizim hattının 5. sürümü (`imageFs`), resim dokuları `styled/pictures.rs`, web `render/pictures.ts`;
  masaüstü `kentos_interaction::image_insert`, `::image_clip`, `pictures.rs`; web `tools/imageTools.ts`, `tools/pictureFile.ts`,
  `ui/properties/imageRows.ts`; ortak izler `image-insert.json`, `image-clip.json` (oynatıcılarda resmin gösterilen kısmı). `CAD-31`
  çizimler arası alışveriş ([ADR 0193](docs/adr/0193-drawing-exchange.md)) tek parçada bitti (7 Ekim; ikonlar önerilen seçenekler):
  kurallar belge düzeyinde, sözleşmenin JSON'u üzerinde `kentos_domain::exchange` ve `model/exchange.ts` (bağımsız başvuru
  `exchange_cases.py`, ortak `fixtures/exchange/v1`); masaüstü `drawing_exchange.rs` (`NewLayer`'ın `snap`'i), web
  `app/drawingExchange.ts`, `ui/io/TakeFromDialog.ts`; ortak izler `take-from.json`, `block-insert-file.json` (web'in oynatıcısı dosya
  okuyan komutun aracını bekler). `CAD-32` hizala ve dağıt ([ADR 0194](docs/adr/0194-align-and-distribute.md)) tek parçada bitti
  (7 Ekim; ikonlar seçenek sayfasının A'ları): çekirdek `ops::arrange` (kutular deponun kuralıyla, `block::pieces_bounds`; bağımsız
  başvuru `arrange_cases.py`, ortak `fixtures/arrange/v1`), sözleşmenin `Transform::Arrange` ve `ArrangeMode`'u, komut iki platformda
  (`too_few_objects`), araç masaüstünde `kentos_interaction::align_distribute`, web'de `tools/alignDistributeTool.ts`; ortak iz
  `align-distribute.json`. `CAD-33` görünüm kipleri ([ADR 0195](docs/adr/0195-view-modes.md)) tek parçada bitti (7 Ekim; ikonlar
  önerilen seçenekler, Tek renk'in ikinci seçenek): ayarlar `graphics.colorMode`, `.fills`, `.areaEdges`, `.transparency`,
  `.highlightColor`, `.highlightWidth`; stil çekirdeğinin `build::View`'u (`BatchSink::hide`), renk kuralı masaüstünde
  `kentos_native_style::color`'ın `ViewColors`'ı, web'de `render/color.ts`; vurgu masaüstünde wgpu'nun ikinci çerçeve bağlaması, web'de
  `widenLines`; ortak durumlar `batches.json`'un görünüş durumları, ortak iz `view-modes.json`. `CAD-34` eğri boyunca yazı
  ([ADR 0196](docs/adr/0196-text-along-curve.md)) tek parçada bitti (7 Ekim; ikonlar önerilen seçenekler, Doğrultuya döndür'ün ikincisi):
  sözleşmenin `TextPath`'i ve `text_path_problem`'i (`invalid_path`), `.kcad` şema 25 (`FORMATS_VERSION` 35), çekirdek `text::along`
  (bağımsız başvuru `text_along_cases.py`, ortak `fixtures/text/v1/along.json`), deponun harf başına `LABEL_LINE` kayıtları
  (`TextPlace::by_records`), DXF `writer/entities/curved.rs` ve `emit.rs`'in `curved_of`'u (`dxf-write/curved`); araçlar masaüstünde
  `kentos_interaction::text_along`, web'de `tools/textAlongTool.ts`; ortak iz `text-along.json` (oynatıcılarda `curve` beklentisi).
  `CAD-35` çizim ekleri ([ADR 0197](docs/adr/0197-drawing-extras.md)) tek parçada bitti (7 Ekim; ikonlar önerilenler, Menzil
  halkaları'nınki 16 px'te Halka'ya benzediği için yelpaze): çekirdek `tools::drawing_extras` (bağımsız başvuru
  `drawing_extras_cases.py`, ortak `fixtures/drawing-extras/v1/cases.json`), `cad.entities.create`'in `tangentLine`, `fourthCorner`,
  `rangeRings` adımları; araçlar masaüstünde `kentos_interaction::drawing_extras`, web'de `tools/drawingExtrasTools.ts`; ortak iz
  `drawing-extras.json`; 7 Ekim eki (sahibin bildirimi): Menzil halkaları bir takımdan sonra kapanır, ilk değerleri 5 m ve 3 halka. `CAD-36`
  plan yolu çizimi ([ADR 0198](docs/adr/0198-plan-roads.md)) tek parçada bitti (7 Ekim; geometri genel, değer tablosu **[M]** sahibin
  tarifini bekler): çekirdek `ops::road` (bağımsız başvuru `plan_road_cases.py`, ortak `fixtures/plan-road/v1/cases.json`; köşeler
  `ops::reshape::corners_of_path_where` ve `turns_inward` ile), `cad.entities.create`'in `planRoad`, `cad.entities.edit`'in
  `roadJunctions` ve `medianClose` adımları; araçlar masaüstünde `kentos_interaction::plan_road`, web'de `tools/planRoadTools.ts`; ortak iz
  `plan-road.json`. §16.1'in sahibin sırası (CAD-27 … CAD-36) bitti; `CAD-18` ve `CAD-24` **[M]** sahibin tarifini bekler.
  §16.2 CBS başladı (sahibin seçimi “GIS-01”, kapsamı üç kararı: kaynaklar klasörler ve KentOS projeleri, değerler metin ve şema
  türü verir, form şemadan): `GIS-01` katman alanları, öznitelik tablosu ve veri kaynakları
  ([ADR 0199](docs/adr/0199-layer-fields-and-feature-table.md)) tek parçada bitti (7 Ekim; ikonlar sahibin seçtikleri): sözleşmenin
  `fields`'ı (bağımsız başvuru `layer_field_cases.py`), `.kcad` şema 26 (`FORMATS_VERSION` 36), belgenin “Alanlar” adımı, komutların
  alan denetimi, çekirdek `ops::feature_table` (`feature_table_cases.py`); masaüstü `features/`, `layer_fields.rs`, `sources/`, web
  `ui/bottom/FeatureTable.ts`, `ui/layers/LayerFieldsDialog.ts`, `ui/sources/SourcesPanel.ts`; katmanı nesneleriyle alma `layerTake`
  (`exchange_cases.py`'nin `layers`'ı), klasör listesi `source_list_cases.py`; ortak iz `feature-table.json` (adım düzeyinde
  `featureTable` beklentisi); KentOS UI'da yalnız ikonu kalan dok sekmesi 28 px. `GIS-02` mekânsal ve öznitelik sorgusu
  ([ADR 0200](docs/adr/0200-spatial-and-attribute-queries.md); kapsam sahibin kararları: dört araç ailesi, İşlemler araçları, ilişkiler
  sonraya; ikonlar sahibin seçtikleri) tek parçada bitti (7 Ekim): çekirdek `ops::spatial_query`, deponun `relate_pairs`'i,
  `ops::statistics` (bağımsız başvuru `spatial_query_cases.py`, ortak `fixtures/spatial-query/v1` ve `fixtures/processing/v1/queries.json`);
  beş araç web'de `processing/builtin/`, masaüstünde `kentos-processing`'in `builtin/`'ında; İşlemler'e dosya parametresi, tablo çıktısı,
  `refused` ve katman alanları denetimi (`writeCheck.ts`, `writes.rs`); KentOS UI'ın parçalı seçimi `fill_widths` ile. `GIS-03` geometri
  işlemleri ([ADR 0201](docs/adr/0201-geometry-operations.md); kapsam sahibin kararları: dört grubun hepsi, İşlemler araçları, sonuç yeni
  katmana; simgeler sahibin seçtikleri; çizimdeki Kes ve Birleştir'le karışmasın diye adlar Kırp ve Gruplayarak birleştir) tek parçada bitti
  (7 Ekim): çekirdek `ops::geoprocess` (`buffer`, `clip`, `validity`, `reproject`, `calls`) ve `statistics::apportion` (bağımsız başvuru
  `geoprocess_cases.py`, ortak `fixtures/geoprocess/v1` ve `fixtures/processing/v1/geometry.json`: yeni nesneler ölçüleriyle `addedShapes`,
  araçların varsayılanları); on bir araç web'de `processing/builtin/geometry/`, masaüstünde `kentos-processing`'in `builtin/geometry/`'sinde;
  web'de `RunJob.crs`, masaüstünde araç `kentos_project::systems`'ten (işlem grubu artık `project`'e bağlanır); sonuç tablolarında sayı
  sütunları sağda; KentOS UI'ın simge çizicisi `stroke-opacity` okur. `GIS-04` topoloji kuralları
  ([ADR 0202](docs/adr/0202-topology-rules.md); kapsam sahibin kararları: katman içi ve katmanlar arası kurallar, alt panelde sekme,
  bulgudan düzeltme, kurallar ve istisnalar projede; simgeler sahibin seçtikleri) tek parçada bitti (7 Ekim): sözleşmenin `topology`'si,
  `.kcad` şema 27 (`FORMATS_VERSION` 37), çekirdek `ops::topology_rules` (`check`, `fix`; bağımsız başvuru `topology_rules_cases.py`,
  ortak `fixtures/topology-rules/v1`), `cad.entities.edit`'in `topologyFix`'i; sekme web'de `ui/bottom/TopologyPanel.ts` (`topologyRun.ts`,
  `topologyPlan.ts`), masaüstünde `topology/`; pencere `ui/topology/TopologyRulesDialog.ts`, `topology/rules.rs`; bulgunun işareti
  `drawProblemMark`, `marks.rs`; ortak iz `topology-rules.json` (oynatıcılarda `topology` beklentisi, sekmede `pick`). `GIS-05` ağ
  dengelemesi ve kot ağı ([ADR 0203](docs/adr/0203-network-adjustment.md); kapsam sahibin kararları: yatay ağ ve kot ağı, önsel doğruluklar
  Proje ayarları › Ölçme'de, aynı adlı noktalar güncellenir, yenileri eklenir; prizmatik aplikasyon ve kanava iptal; simgeler sahibin
  seçtikleri) tek parçada bitti (8 Ekim): sözleşmenin önsel doğrulukları (`SurveySigmas`, `SIGMA_DEFAULTS`), `.kcad` şema 28
  (`FORMATS_VERSION` 38), çekirdek `survey::adjust` (`horizontal`, `levelling`, `linalg`, `chi2`; bağımsız başvuru `network_adjust_cases.py`,
  ortak `fixtures/network-adjust/v1`), Karne editörünün satırları `survey::fieldbook`'un `network_rows`, `level_rows`'u;
  `cad.entities.edit`'in `networkAdjust`, `levelAdjust`'ı, `cad.entities.create`'in `networkAdjust`'ı; pencereler web'de
  `ui/calc/NetworkDialog.ts`, masaüstünde `calc/network/`; sahne `fixtures/interaction/v1/network-adjust.kcad`. `GIS-08` raster
  katmanları ([ADR 0204](docs/adr/0204-raster-layers.md); kapsam sahibin kararları: GeoTIFF ile dünya dosyalı PNG ve JPEG kendi okuyucumuzla,
  bütün dönüşümleriyle oturtma, bantlar, rampa ve gölgeli kabartma, bağlı dosya ve küçüklerin gömülmesi; ilke “Performance First”; simgeler
  sahibin seçtikleri) tek parçada bitti (8 Ekim): sözleşmenin `raster`'ı, `.kcad` şema 29 (`FORMATS_VERSION` 39), biçim çekirdeği
  `formats::raster` (`tiff`, `geotiff`, `codec`, `png`, `source`, `style`, `pyramid`, `write`, `warp`, `place`; bağımsız başvurular
  `raster_cases.py`, `raster_georef_cases.py`, ortak `fixtures/raster/v1`), geometri çekirdeğinin `geom::raster` ve `ops::georef`'i, stilli
  çizimin 6. sürümü; masaüstünde `rasters/` ve `calc/raster_fit.rs`, çizim hattı `styled/raster_tiles.rs`; web'de `io/rasterWorker.ts`,
  `render/rasterService.ts`, `render/rasterPass.ts`, `ui/raster/`; sahne `fixtures/interaction/v1/rasters.kcad`. `GIS-09` nokta bulutu
  ([ADR 0207](docs/adr/0207-point-clouds-and-large-data.md); kapsam sahibin kararları: LAS, LAZ, COPC ve XYZ; LAZ için `laz`; 2B plan, kat kat
  ve renk kipleriyle; dört işlem grubu; dosya, URL ve sanal bulut; simgeler sahibin seçtikleri) tek parçada, **yalnız masaüstünde** bitti
  (8 Ekim; sahibin kararı: “Nokta bulutu çok ağır bir iş … Sadece masaüstü ile kalalım”, web'de düğmeler notuyla bekler, web'in modülü
  TODOS.md `GIS-09`'un alt maddesi): sözleşmenin `pointcloud`'u ve rasterin `url`'si, `.kcad` şema 31 (`FORMATS_VERSION` 41), çekirdek
  `kentos-pointcloud` (`source`, `las`, `chunks`, `copc`, `text`, `crs`, `index`, `write`, `vpc`, `look`, `nodes`, `place`, `ops`; bağımsız
  başvurular `pointcloud_cases.py`, `pointcloud_index_cases.py`, `pointcloud_ops_cases.py`), geometri çekirdeğinin `geom::pointcloud`'u,
  noktaların çizim hattı (`styled/points.rs`, `shaders/wgsl/points`, stilli çizimin 7. sürümü); masaüstünde `pointclouds/`, İşlemler'in
  `builtin/pointcloud/`'u ve `files`'ı, rasterin adresi (`rasters/tiles.rs`'in `Origin::Url`'si); komut durumları `fixtures/commands/v1/desktop`;
  sahne `fixtures/interaction/v1/pointclouds.kcad`; süreler `perf::clouds`. Sahip aksini söyleyene dek her modül iki platformda, gerektiğinde
  bulut tarafıyla yapılır; platform sorulmaz, ikonları sorulmadan seçilir (sahibin kararları, 8 Ekim akşamı). `GIS-10` harita servisleri
  ve altlıklar ([ADR 0208](docs/adr/0208-map-services-and-basemaps.md); kapsam sahibin sözleri: sayılanların hepsi, Google ve vektör karolar,
  bütün kimlik doğrulama türleri, GeoJSON; iki platform ve bulut) tek parçada bitti (9 Ekim): sözleşmenin `service`'i (`ServiceLayer`,
  `FeatureFeed`, `ServiceConnection`), `cad_layers` (`cad.layers.service`), `.kcad` şema 32 (`FORMATS_VERSION` 42); çekirdek
  `kentos-services` ve geometri çekirdeğinin `geom::tiles`'ı (bağımsız başvurular `service_tiles_cases.py` pyproj'la,
  `service_mvt_cases.py` GDAL'la, `service_caps_cases.py` OWSLib'le, `service_rules_cases.py`, `layer_service_command_cases.py`); WASM
  `crates/wasm/services-wasm`; masaüstünde `services/` (hub, `net`, `cache`, `secrets`, `window`, `feed_window`, `connections`, `info`,
  `overlay`), çizim hattında `styled/service_tiles.rs`; web'de `io/services/` (`worker.ts`, `feedWorker.ts`, `fetch.ts`),
  `render/serviceHub.ts`, `render/servicePass.ts`, `ui/services/`, `ui/bottom/ServiceInfoPanel.ts`; sunucuda `http/proxy.rs`; Python
  `python/kentos/services.py`; süreler `perf::services`. `GIS-11` ağ analizi ([ADR 0209](docs/adr/0209-network-analysis.md); kapsamı
  sahibin sözüyle ben belirledim, “Biliyorsun önceliğimiz performans”) tek parçada bitti (9 Ekim): sözleşmenin `network`'ü ve
  `cad_networks`'ü (`cad.network.define`), `.kcad` şema 33 (`FORMATS_VERSION` 43); çekirdek `ops::network` (`input`, `graph`, `search`,
  `route`, `area`, `closest`, `trace`, `check`, `session`; bağımsız başvuru `network_cases.py`, kurallar `network_rules_cases.py`, form
  `network_form_cases.py`), tamponların birleşimi `buffer::union_pieces` ve `overlay::overlay_buffers`; web'de ağın işçisi `io/network/`,
  `app/networks.ts`, araçlar `tools/network*Tool.ts`, `ui/networks/`; masaüstünde `networks/` (ağın iş parçacığı `engine.rs`) ve
  `kentos_interaction::network`; İşlemler'in Ağ analizi kategorisi iki platformda (`network_processing_cases.py`); Python `kentos.network`;
  ortak izler `network-*.json`, sahne `networks.kcad`; süreler `network::timing`, `network_timing`. Açık kalanlar TODOS.md `GEO-01`,
  `GEO-02`. Raster analizi kümesi (`GIS-31`…`GIS-36`) başka bir ajandadır (sahibin kararı, 9 Ekim; onun ADR'leri 0230'dan başlar), bu
  sırada atlanır; dosyalarına dokunulmaz. `GIS-12` zaman ve senaryolar ([ADR 0210](docs/adr/0210-temporal-layers-and-scenarios.md);
  kapsamı ben belirledim) tek parçada bitti (9 Ekim): sözleşmenin `temporal`'ı (`LayerTime`, `ScenarioInfo`, `scenarios_problem`),
  `cad_layers`'ın `LayersTime`'ı ve `cad_scenarios`'ı, `.kcad` şema 34 (`FORMATS_VERSION` 44); çekirdek `time` ve deponun zamanları
  (bağımsız başvurular `temporal_cases.py`, `temporal_rules_cases.py`, `temporal_command_cases.py`); web'de `app/timeSlider.ts`,
  `app/timeCommands.ts`, `app/scenarios.ts`, `ui/time/`, `tools/timeVersionTool.ts`; masaüstünde `temporal/` ve
  `kentos_interaction::time_version`, stilli çizimin önbelleği pencerenin farkını parçalarla kurar; sunucu ağacın kurallarını denetler;
  Python `kentos.temporal`; komut durumlarında `$layer:K`; ortak izler `time-*.json`, `scenarios.json`, sahne `temporal.kcad`; süreler
  `time::timing`, `style::perf::temporal`, `scripts/perf/time.test.ts`. `GIS-13` ve `GIS-14` mevzuatla düzenlenen işlerdir (sona kalır).
  `GIS-15` katman süzgeci ([ADR 0211](docs/adr/0211-layer-filter.md); kapsamı ben belirledim) tek parçada bitti (9 Ekim): sözleşmenin
  `layer_filter`'ı (`LayerFilter`, `filters_problem`), `cad_layers`'ın `LayersFilter`'ı, `.kcad` şema 35 (`FORMATS_VERSION` 45);
  değerlendirme `kentos_native_application::layer_filter` ve web `model/layerFilter.ts` (liste bir kez kimliklere çevrilir, koşul
  İfadeyle seç'in sütun motoruyla yalnız listedekilere), deponun süzgeç işareti (`set_filtered`, `view_shown`), masaüstünde `Spatial`'ın
  `sync_filters`'ı, web'de `PickIndex`'in `markWhole`'u ve `model/idMarks.ts`; web'de `app/layerFilterCommands.ts`,
  `ui/layers/LayerFilterDialog.ts`; masaüstünde `layer_filters.rs`; sunucu ağacın kuralını denetler (`http::filter_tests`); Python
  `kentos.cad.layers.filter`, MCP'de araç; komut durumları `layer_filter_command_cases.py`; ortak iz `layer-filter.json` (yeni beklenti
  `layerCounts`); süreler `perf layer_filter`, `scripts/perf/filter.test.ts`.
  Paralel dal `gis-31-36-raster-analysis` (sahibin sözü, 9 Ekim:
  “yeni bir branch açarak GIS-31 ve GIS-36 aralığını yapacağız”, “yüksek performans ilk önceliğimiz”; ADR numaraları 0231–0236 bu aralığa
  ayrıldı): `GIS-31` yüzey analizi ([ADR 0231](docs/adr/0231-raster-analysis-and-surface.md)) tek parçada bitti (9 Ekim): çekirdek
  `kentos-raster` (`job`, `terrain`, `relief`, `insolation`, `contours`, `out`, `par`; bağımsız başvurular `terrain_cases.py` gdaldem'le,
  `contour_cases.py` gdal_contour'la), WASM `crates/wasm/raster-wasm`; İşlemler'in `builtin/surface/`'u iki platformda, ev sahibinin
  `Files::open_raster`'ı, katman parametresinin `above`'u; masaüstünde `DesktopFiles::open_raster`, `rasters/tiles.rs`'in `open_reader`'ı; web'de
  `io/rasterAnalysis*.ts`, `processing/rasterHost.ts`, `app/rasterAnalysis.ts`; ortak durumlar `fixtures/processing/v1/surface.json`
  (`surface_processing_cases.py`); süreler ADR'nin Doğrulama'sında. `GIS-32` interpolasyon ve yoğunluk
  ([ADR 0232](docs/adr/0232-interpolation-and-density.md)) tek parçada bitti (9 Ekim): geometri çekirdeğinde `predicates::incircle` ve
  `geom::delaunay` (bağımsız başvuru `delaunay_cases.py`); raster çekirdeğinde `points`, `grid`, `index`, `solve`, `interp` (`natural`,
  `spline`, `kriging`), `density`, `from_points` (bağımsız başvurular `interpolation_cases.py`, `bessel_k0.py`), WASM `PointAnalysis`;
  İşlemler'in `builtin/interpolation/`'u iki platformda, katman parametresinin `below`'u, `Beside::named`, web RunContext'in `project`'i;
  ortak durumlar `fixtures/processing/v1/interpolation.json` (`interpolation_processing_cases.py`). `GIS-33` raster işlemleri
  ([ADR 0233](docs/adr/0233-raster-operations.md)) tek parçada bitti (9 Ekim): raster çekirdeğinde `dd`, `stats`, `inputs`, `areas`,
  `calc`, `reclass`, `focal`, `resample`, `ops` (`OpsSpec::reads`: hesaplayıcının okuduğu rasterler, rasterler açılmadan), `out`'un her
  örnek türü (bağımsız başvuru `raster_ops_cases.py`, gdalwarp'la çapraz denetim); ifade dilinin matematik işlevleri; WASM `OpsOpening`,
  `OpsAnalysis`, `opsReads`; İşlemler'in `builtin/raster_ops/`'u ve `rasterOps/`'u iki platformda, girdinin özetinde raster adları
  (`features::raster_order`, `rasterRun`), `RunResult.above`; ortak durumlar `fixtures/processing/v1/raster-ops.json`
  (`raster_ops_processing_cases.py`). `GIS-34` raster ve vektör dönüşümü
  ([ADR 0234](docs/adr/0234-raster-vector-conversion.md)) tek parçada bitti (9 Ekim): raster çekirdeğinde `vector` (`label`, `rings`,
  `thin`, `simplify`, `capture`, `work`), `rasterize`, `inputs`'un `Raw`'ı; geometri çekirdeğinde `ops::contour_elevations` (bağımsız
  başvuru `raster_vector_cases.py`, GDAL'la çapraz denetim); WASM `OpsAnalysis`'in nesneleri; İşlemler'in `builtin/raster_vector/`'u ve
  `rasterVector/`'u iki platformda, `Patch.zs`; ortak durumlar `fixtures/processing/v1/raster-vector.json`
  (`raster_vector_processing_cases.py`), taranmış paftanın sahnesi `scanned.kcad` (`scanned_scene.py`). `GIS-35` hidroloji
  ([ADR 0235](docs/adr/0235-hydrology.md); sahibin sözü “Bu konu çok önemli iyi araştır”) tek parçada bitti (9 Ekim): raster çekirdeğinde
  `hydro` (`surface`, `heap`, `fill`, `tiled`, `flow`, `accum`, `basins`, `streams`; bağımsız başvuru `hydrology_cases.py`, GRASS'la
  çapraz denetim), `ops`'un sekiz türü, `vector`'ün `fields` ve `numbers`'ı; WASM `featureFields`, `featureNumbers`; İşlemler'in
  `builtin/hydrology/`'si iki platformda; ortak durumlar `fixtures/processing/v1/hydrology.json` (`hydrology_processing_cases.py`),
  resimlerin sahnesi `hydrology.kcad` (`hydrology_scene.py`). `GIS-36` uzaklık ve maliyet
  ([ADR 0236](docs/adr/0236-distance-and-cost.md)) tek parçada bitti (9 Ekim): raster çekirdeğinde `distance` (`edt`, `network`, `mod`;
  bağımsız başvuru `distance_cases.py`, GRASS'ın r.cost ve r.grow.distance'ı ve GDAL'ın Proximity'siyle çapraz denetim), `ops`'un dört
  türü, `from_points`'in `PointTool::Distance`'ı; WASM notlarında `distance`; İşlemler'in `builtin/distance/`'ı iki platformda, web
  koşucusunun yerleşimi görünen girdiyle; ortak durumlar `fixtures/processing/v1/distance.json` (`distance_processing_cases.py`),
  resimlerin sahnesi `distance.kcad` (`distance_scene.py`). Dal 9 Ekim'de `main`'e birleşti (sahibin sözü: “36 bitince main ile
  birleştir ve main push yap”); raster çözümleme kümesi bitti. `GIS-16` etiket motoru ([ADR 0212](docs/adr/0212-label-engine.md); sahibin
  sözü “bu konu çok önemli, QGIS falan nasıl çözüyor incele ve ona göre yap”: QGIS'in PAL'ı, Maplex ve MapLibre'nin çakışma dizini
  araştırıldı) tek parçada bitti (10 Ekim): sözleşmenin `labels`'ı ve `cad_labels`'ı, `.kcad` şema 36 (`FORMATS_VERSION` 46);
  çekirdek `labels` (`engine`, `geom`, `style`, `text`) ve deponun `placing`'i, adaylar gruplar hâlinde gerektikçe (bağımsız başvuru
  `label_engine_cases.py`, 43 durum; komut durumları `label_command_cases.py`); metinler `kentos_native_application::label_texts` ve web
  `model/labelTexts.ts`; Etiketleri yazıya çevir motorun yerleşimiyle (`ops::label_text`, ADR 0175'in eki); web'de `app/labelCommands.ts`,
  `ui/labels/`, `tools/labelTools.ts`, `viewport/placedLabels.ts`; masaüstünde `labelling/`, `kentos_interaction::label_tools`; pafta PDF'ine
  zeminler ve çağrı çizgileri; sunucuda `label_pins` sütunu ve göç 0015 (tür denetimi kılavuz, tablo, resim, raster ve nokta bulutunu da
  alır); Python ve MCP; ortak iz `label-tools.json` (yeni beklenti `labelPins`); süreler ADR'nin Doğrulama'sında. `GIS-17` ek işleyiciler
  ([ADR 0213](docs/adr/0213-additional-renderers.md); kapsamı ben belirledim) tek parçada bitti (10 Ekim): stil çekirdeğinde dokuz tür
  (`model.rs`, `rules.rs`, `thematic.rs`, `dots.rs`, `charts.rs`, `groups.rs`, `heat.rs`, `inverted.rs`; kurulumun `build_layer_in`'i ve
  `ViewFrame`'i, `BatchSink::fan`; bağımsız başvuru `renderer_cases.py`, 124 durum); işleyici KCAD'de opak, şema değişmez; komut
  `cad.layers.renderer` iki platformda, başsız sunucuda, Python'da ve MCP'de (`renderer_command_cases.py`, 45 durum); masaüstünde stilli
  sahnenin `ViewBuilt`'i, resim kaynağının `put_made` ve `keep_made`'i, Katman stili'nin `thematic.rs`'i; web'de `ui/style/thematicPanels.ts`,
  `style/thematic.ts`; KentOS UI'da açıklamalı radyo satırlarının ikonları; ortak `legend.json` ve `batches.json`; resimlerin sahnesi
  `renderers.kcad` (`renderer_scene.py`); süreler `renderer_timing` ve `scripts/perf/renderers.test.ts`. `GIS-18` ifade dili ekleri
  ([ADR 0214](docs/adr/0214-expression-extras.md); kapsamı ben belirledim, `regex-lite` sahibin onayıyla) tek parçada bitti (10 Ekim):
  çekirdekte `dates`, `patterns`, `arrays`, `compound`, `resolve` ve `world` (`roles`, `aggregate`, `spatial`, `Session`), `library/extras.rs`'in
  71 işlevi; `compile_with` başka katmana bakanı şema izin vermedikçe reddeder (`WORLD_REFUSED`); sözleşmenin `variables`'ı, `.kcad` şema 37
  (`FORMATS_VERSION` 47); `kentos_project::variables` ve `variable_form`, web `model/projectVariables.ts` ve `model/variableForm.ts`; masaüstünün
  İşlemler'i sütun motorunda (`EntityObjects`, `evaluate_in`, `runner::world_layers`); web'de `exprCompileIn`, `evaluateExpressionIn`,
  `ExprWorld`, `RunJob.variables` ve `leftOut`; Proje ayarları › Değişkenler iki platformda (`project/variables.rs`, `variablesSection`);
  pafta `ProjectInfo.variables`; Python `ProjectVariable`; bağımsız başvurular `expression_extras.py` (190 kaynak), `variable_form_cases.py`,
  `spatial_query_cases.py`'nin ifade durumları; süreler `measures_the_additions` ve `scripts/perf/expressionExtras.test.ts`. `GIS-19`
  yakınlık analizi ([ADR 0215](docs/adr/0215-proximity-analysis.md); kapsamı ben belirledim) tek parçada bitti (10 Ekim): çekirdek
  `ops::proximity` (en yakın noktalar, ortak kenar, içlerin örtüşmesi kutularla daraltılarak) ve `store::proximity` (`nearest`, `neighbors`),
  `store::rtree`'nin en iyi önce araması (`PackedTree::nearest`, `Nearer`); İşlemler'in Yakınlık kategorisi iki platformda (web
  `processing/builtin/proximity/`, masaüstü `kentos-processing`'in `builtin/proximity/`); bağımsız başvuru `proximity_cases.py` (9 durum);
  pencereler `processing::proximity_tests`, `shots.mjs proximity`; süreler `proximity::timing` ve `scripts/perf/proximity.test.ts`. `GIS-20`
  anahtarla veri aktarma ([ADR 0216](docs/adr/0216-data-transfer.md); kapsamı ben belirledim) tek parçada bitti (10 Ekim): İşlemler'in
  Alan eşleme parametresi (web `fieldPlan.ts`'in `fieldMapView`'i ve `paramFields.ts`'in `fieldMapField`'ı, masaüstü `kentos_processing::field_map`
  ve `fields.rs`'in `field_map`'i), `allowEmpty`, `FeatureSet.layerId`, `RunContext.fields`, şeklin yaması (`Patch.geometry`); Veri aktar
  web `processing/builtin/transfer.ts`, masaüstü `builtin/transfer.rs`, kaynağın satırları Anahtarla birleştir'le ortak (`joinSource.ts`,
  `join_source.rs`); bağımsız başvurular `transfer_cases.py` (10 durum) ve `field_map_cases.py` (27 durum); pencereler
  `processing::transfer_tests`, `shots.mjs transfer`; süreler `transfer_timing` ve `scripts/perf/transfer.test.ts`. Masaüstünde dosyanın
  satırları kopyalanmadan okunur, çalıştırıcı yamanın parçalarını taşır. `GIS-21` katman bölme ve birleştirme
  ([ADR 0217](docs/adr/0217-layer-split-and-merge.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::order` (işlem
  `orderBy`, doğal sıra `text::natural`'ın `tokens_cmp`'siyle); İşlemler'in Katmanlar parametresi (`layers`), aracın katmanları
  (`RunResult.layers`, `ToolLayer`; çalıştırıcı tek adımda açar) ve `like`, masaüstünde `Patch.layer`; beş araç web
  `processing/builtin/layers/`, masaüstü `builtin/layers/`; Yinelenenleri sil değer sınıfları, ön süzgeç ve masaüstünde özetlenen şekille;
  bağımsız başvuru `layer_ops_cases.py` (16 durum); pencereler `processing::layer_tools_tests`, `shots.mjs layertools`; süreler
  `layer_timing` ve `scripts/perf/layers.test.ts`. `GIS-22` geometri dönüştürme
  ([ADR 0218](docs/adr/0218-geometry-conversion.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::convert`
  (`points`: merkez, GEOS'un iç noktası yaylarıyla, köşeler; `paths`: parça, halkalar, alan, köşe ekle; `table`: satırlar ve şekiller;
  web'e toplu `calls`); on araç web `processing/builtin/convert/`, masaüstü `builtin/convert/`, kategori `conversion`; ortak durumlarda
  hesaplanan alanlar için `toleranceFields`; çalıştırıcı geçmiş kaydında dosya satırlarını tutmuyor, hesap değerleri ödünç alıyor, büyük
  değerler ayrı iş parçacığında bırakılıyor; bağımsız başvuru `convert_cases.py` (23 durum, iç noktalar GEOS'la çapraz); pencereler
  `processing::convert_tests`, `shots.mjs geoconvert`; süreler `convert_timing` ve `scripts/perf/convert.test.ts`. `GIS-23` geometri
  üretimi ([ADR 0219](docs/adr/0219-geometry-generation.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::generate`
  (`voronoi`: çevrel merkezlerden hücreler, aynı çemberdekiler bir merkez, çerçeveye ortak kesişimlerle; `bounding`: kesin zarf, dönen
  kumpasla en küçük kutu, sabit tohumlu Welzl; `concave`: JTS'in aşındırması, delikler büyükten küçüğe; `pole`: etiket motorunun
  polylabel'ı; `points`: eğrilerin 0,1 mm'lik noktaları ve eksen uçları; web'e toplu `calls`); beş araç web `processing/builtin/generate/`,
  masaüstü `builtin/generate/`, kategori `generation`, birim `%`; web'de çekirdeğe yalnız geometri gider (`entityGeometry`); bağımsız başvuru
  `generate_cases.py` (20 durum, kesirler ve GEOS); pencereler `processing::generate_tests`, `shots.mjs generate`; süreler
  `generate_timing` ve `scripts/perf/generate.test.ts`. `GIS-24` örnekleme ve ağ üretimi
  ([ADR 0220](docs/adr/0220-sampling-and-grids.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::sampling` (`grid`,
  `cover`, `inside`, `regular`, `random`, `scatter`, `walk`, `along`, `cut`, `subdivide`, `calls`; web'e ağlar ve noktalar düz sayı
  dizileriyle); `WindingIndex::owned`; beş araç web `processing/builtin/sampling/`, masaüstü `builtin/sampling/`, kategori `sampling`;
  Alt bölümlere ayır'ın alanı genel overlay yerine doğru boyunca kesilir (`cut`: overlay'le aynı alanlar ve köşeler, 10 000 köşede 297 ms
  yerine 2 ms); bağımsız başvuru `sampling_cases.py` (29 durum, GEOS ve kesirler); pencereler `processing::sampling_tests`,
  `shots.mjs sampling`; süreler `sampling_timing` ve `scripts/perf/sampling.test.ts`. `GIS-25` kartografik genelleştirme
  ([ADR 0221](docs/adr/0221-cartographic-generalization.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): çekirdek `ops::generalize`
  (`smooth`, `orthogonal`, `building`, `coverage`: kapsama grafı köşe numaralarıyla, `simplify`: Visvalingam–Whyatt sayım ızgarasıyla,
  `parts`, `aggregate`: kümeler ve kapanış ayrı, `eliminate`, `calls`; web'e toplu işlemler, değişmeyen nesnenin şekli dönmez); yedi araç web
  `processing/builtin/generalize/`, masaüstü `builtin/generalize/` (kümeler iş parçacıklarında), kategori `generalization`; şeridin ▾ listesi
  İşlemler kategorileriyle başlıklı (`headed_menu_of`, `overflowRows`); kesişim çekirdeğinde üç sağlamlık kuralı (`geom::arrangement`'in
  `Circles`'ı, yakın kesişimlerin kesmesi, `balance`; ADR 0201'in eki); bağımsız başvurular `generalize_cases.py` (22 durum) ve
  `closing_cases.py` (45 blok; 2 700 rastgele blokla zorlandı); pencereler `processing::generalize_tests`, `shots.mjs generalize`; süreler
  `generalize_timing` ve `scripts/perf/generalize.test.ts`. `GIS-26` KML/KMZ, GPX ve içe aktarmada açık dönüşüm
  ([ADR 0222](docs/adr/0222-kml-gpx-and-explicit-transformation.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): biçim çekirdeğinde
  `kml` (`read`, `style`, `write`), `gnss::gpx_write`, `reproject` (içe aktarılanın projenin sistemine çevrilmesi), `export_geom` (GeoJSON,
  KML ve GPX'in ortak geometrisi, biçimin sözleriyle), `zip::write`, `geojson::write_wgs84`, GPX ve NMEA okuyucularının çizgileri
  (`GnssLine`); `FORMATS_VERSION` 49; WASM `readKml`, `writeKml`, `writeGpx`, `writeGeoJsonWgs84`, `reprojectImport`; koordinat sistemi
  sorusunun çevirme onayı iki platformda (`CrsQuestion::convertible`, `way`, `ready`); masaüstü `exchange/wgs84_export.rs`, web
  `ui/io/Wgs84ExportDialog.ts`; GNSS'in çizgileri `kentos_interaction::gnss::place_all`, `model/gnssImport.ts`'in `placeGnssAll`'ı;
  bağımsız başvurular `kml_cases.py` (7 örnek, 4 taşıma, yazıcının 3 dosyası) ve `wgs84_write_cases.py` (GPX ve GeoJSON), GNSS
  başvurularının çizgileri; pencereler `exchange::kml_tests`, `shots.mjs kml`; süreler `kml_timing` ve `scripts/perf/kml.test.ts`.
  `GIS-27` tablo alışverişi ([ADR 0223](docs/adr/0223-table-exchange.md); kapsamı ben belirledim) tek parçada bitti (11 Ekim): biçim
  çekirdeğinde `xml_scan` (akış okuyucusu: önekler önbellekte, etiketin türü baytından), `table_file`'ın `Limits`'i ve `read_with`'i
  (Tablo ekle 10 001 × 101, İşlemler 250 001 × 501, sessiz kısaltma kalktı), `xlsx`, `ods`, `dates` (openpyxl'in tarih kuralı),
  `workbook` (XLSX, ODS, CSV yazıcısı; hücreler dilim dilim, büyük parçalar `zip::Pack::Fast`'la), `coords::NOT_TEXT`; geometri
  çekirdeğinde `ops::convert::table::point_rows` (`convertPointRows`); `FORMATS_VERSION` 50; WASM `readTableFile(bytes, processing)`,
  `writeWorkbook`; İşlemler'in dosya alanında Sayfa listesi ve ayrı iş parçacığında okuma (masaüstü `FileRead`, `Sheet`), Tablodan nokta iki
  platformda, sonuç tablolarında Kaydet ▾ (`SaveTableAs`); masaüstü `table_write.rs`, web `ui/bottom/featureExport.ts`; Öznitelik tablosunun
  Dışa aktar ▾'si (`workbook_sheet`, `featureSheet`); bağımsız başvurular `table_file_cases.py`, `workbook_cases.py`, `table_points_cases.py`,
  `feature_export_cases.py`; pencereler `processing::table_points_tests`, `shots.mjs tableexchange`; süreler `workbook_timing`,
  `table_points_timing`, `scripts/perf/workbook.test.ts`, `scripts/perf/convert.test.ts`. `GIS-28` doğrusal referanslama
  ([ADR 0224](docs/adr/0224-linear-referencing.md); kapsamı ben belirledim) bitti (11 Ekim; iki commit: sahibin isteğiyle yarıda
  bırakılıp WIP olarak gitti, kalanı ikinci commit'te): sözleşmenin `ma`, `mb`, `ms`'i, `.kcad` şema 39
  (`FORMATS_VERSION` 51); düzenlemelerde M kotun kurallarıyla (`measure_lost`, `invalid_measures`); Shapefile'ın M'si; çekirdek `ops::lrs`
  (`Walk`, `create`, `calibrate`, `overlay`, `area`, `index`, `calls`; bağımsız başvuru `lrs_cases.py`, 59 durum); İşlemler'in
  `builtin/lrs/`'ı iki platformda (`lrs_processing_cases.py`, 14 durum; masaüstünde pencereden `processing::lrs_tests`); Öznitelikler,
  köşe tablosu ve kartta M; Km yaz'ın Ölçü (M)'si (`measure_stations`, `stationingMeasure`; ortak iz `stationing-measure.json`; hattın
  rotası web'de `model/ops/lrs.ts`'in `routeOf`'u, masaüstünde `kentos_native_application::geometry::route`); masaüstünün komut
  satırında yazılan tuş kendi seçeneğini seçer (KentOS UI `keyed`); resimlerin yolu `lrs_scene.py`; süreler `lrs::timing` ve
  `scripts/perf/lrs.test.ts`. `GIS-29` nesne ekleri ([ADR 0225](docs/adr/0225-object-attachments.md); kapsamı ben belirledim) tek
  parçada bitti (12 Ekim): sözleşmenin `attachment`'ı ve `layer_info`'su, `cad.layers.info`, `cad.entities.set`'in `attachments`'ı,
  `.kcad` şema 40 (`FORMATS_VERSION` 52), sunucuda göç 0016; çekirdek `kentos_expression::template` (`template_cases.py`),
  `kentos_formats::exif` (`exif_cases.py`), `kentos_interaction::photos` (`photo_import_cases.py`, PROJ) ve `::identify`,
  `kentos_native_application::info_templates` ve `layers_info` (`layer_info_command_cases.py`); masaüstünde `attachments.rs`,
  `photo_view.rs` (kendi wgpu boru hattı: yeni bağımlılık yok), `object_info.rs`, `info_card.rs`, `layer_info.rs`,
  `exchange/photo_import.rs`, `properties/rows/attachments.rs`; web'de `app/attachments.ts`, `app/objectActions.ts`, `ui/attachments/`,
  `ui/shell/InfoCard.ts`, `ui/layers/LayerInfoDialog.ts`, `ui/io/PhotoImportDialog.ts`; ortak iz `identify.json` (yeni beklenti
  `infoCard`, sahne `attachments_scene.py`); masaüstü testleri `attachments_tests`; süreler `exif::timing`, `info_templates`,
  `a_card_opens_in_a_frame`, `scripts/perf/attachments.test.ts`. `GIS-30` sembol katmanı ekleri
  ([ADR 0226](docs/adr/0226-symbol-layer-extras.md); kapsamı ben belirledim) tek parçada bitti (12 Ekim): sözleşmenin `layer_compose`'u
  (`LayerNode.compose`), `cad.layers.compose`, `.kcad` şema 41 (`FORMATS_VERSION` 53); stil çekirdeğinde `style/generate.rs` ve efekt grupları
  (`compile.rs`); çizim alanı `shaders/wgsl/compose` (sözleşmenin 2. sürümü), masaüstü `crates/render/wgpu/src/compose.rs`, web
  `render/composePlan.ts` ve iki arka ucun `composer.ts`'i (WebGL2'nin ikizi `composeShaders.ts`): paketli haritalar, x boyunca sütunlara
  ve y boyunca satırlara, bulanıklığın dokunuşları ikişer, grubun efektleri tek geçişte, çizim resminin geçişleri tek render geçişinde; ilk
  uygulama 19 ms'ydi, zaman damgalarıyla geçiş geçiş ölçülerek 6 ms'ye indi; tasarımcıda `generator.rs`, `layerForms.ts` (Adım ekle'de
  ikonlar, sahibin isteği), Katman stili'nde `layer_style/compose.rs`, `composeSection`; bağımsız başvurular `geometry_generator_cases.py`,
  `effects_cases.py` (gerçek GPU ve tarayıcıyla), `layer_compose_command_cases.py`; ortak iz `symbol-extras.json`; süreler ADR'nin
  Doğrulama'sında. Ardından sahibin bildirimleriyle iki düzeltme (12 Ekim): masaüstünde ikonlar kendi piksellerinde SVG belgesi
  olarak resvg ile çizilir (ADR 0054'ün eki); kendi hedefine çizen alanın resmi tam piksellere oturur, tasarımcının önizlemesinde
  ince çizgi ve dalga düzgün (ADR 0023'ün eki). Devir notu ve açık iş (yarı saydam çizginin köşesi iki kez boyanır):
  [docs/devir/2026-10-12-gis-30.md](docs/devir/2026-10-12-gis-30.md). `GIS-39` grafikler ve alan istatistikleri
  ([ADR 0227](docs/adr/0227-charts-and-field-statistics.md); kapsamı ben belirledim) tek parçada bitti (12 Ekim): çekirdek `ops::charts`
  (`data`, `fields`, `specs`, `model`, `ticks`, `fonts`, `layout`, `svg`, `work`, `json`; bağımsız başvuru `chart_cases.py`, 94 durum),
  `ops::statistics`'in `figures_of`'u ve hızlandırılması (ayırmasız `read_number`, Clinger'in `to_f64`'ü; sonuçlar bit bit aynı); WASM
  `ChartWork`; web'de `app/charts.ts`, `model/chartData.ts`, `model/ops/charts.ts`, `ui/bottom/StatisticsPanel.ts`, `ChartPanel.ts`,
  `ui/charts/`; masaüstünde `kentos_interaction::chart_data`, `statistics/`, `charts/` (şekiller tiny-skia ile ekranın pikselinde, PNG
  resvg ile); İşlemler penceresinde Grafik; ortak iz `charts.json` (oynatıcılarda `chartPress`, `chart` ve `statistics`), sahne
  `chart_scene.py`; süreler ADR'nin Doğrulama'sında (ilk ölçümde üç iş bütçeyi aşıyordu). Aynı commit'te sahibin bildirimiyle CAD'in
  koordinat ekseni simgesi her zaman sol altta (0,0 ekrandayken orijine gidiyordu; ADR 0165'in 12 Ekim eki, resmi `tools_screens`'in
  `cad-eksen-simgesi`'si) ve macOS'ta düşen üç denetim: pafta ve vitrinin birer testi (Ctrl yerine platformun komut tuşu,
  `Modifiers::COMMAND`) ve web'in iz oynatıcısının Ctrl'li tıklaması (macOS'ta Chrome onu sağ tık sayar; `tracePlayer.mjs` orada ⌘ basar;
  Veride ara'nın ve Topoloji kuralları'nın izleri). Ardından (12 Ekim) Grafik'in Tablo görünümü dar panelde sayılarını gösterir
  (nokta editörünün `ptable` kuralları ezildi) ve sahibin bildirimiyle ifadeler çizim biriminde (ADR 0165'in “ifadeler çizim
  biriminde” eki): mm ya da cm projesinde etiketin metni ve koşulu, bilgi kartı, katman süzgeci, tablo süzgeci, oluşturucu ve
  İstatistik/Grafik'in Alan ve Uzunluk'u çizim biriminde ve karesinde (Öznitelikler gibi); İşlemler, ağ maliyetleri ve semboller
  metrede. Birim derlemenin şemasında (`Schema::unit`, web bağlamının `unit`'i), çözümleyici geometri değerlerini ve başka katmana
  bakan işlevlerin ölçülerini çevirir (`resolve`); okuma birimi `expression_unit::reading_unit`, web `readingUnit`; birimli
  durumlar `geometry.json`'un `units`'inde; 13 Ekim'de sahibin bildirimiyle katmanın işleyicisinin ifadeleri de (Kategorili, Aralıklı,
  Kurallı, tematik işleyiciler: Katman stili penceresi, çizim alanı, pafta haritası ve PDF) okuma sayılır ve projenin biriminde
  okunur (ADR 0165'in 13 Ekim eki: `Program::read_in`, `BuildOptions::reading_unit`, `Look::reading_unit`, `ExprScope::unit`);
  sembolün kendi ifadeleri, İşlemler, ağ maliyetleri ve pafta tabloları metrede kalır. Sahibin kararı geldi (12 Ekim): **web katmanına ara**
  ([ADR 0228](docs/adr/0228-web-pause-desktop-and-cloud-first.md)). Web'in son hâli `1aad00d0`'dır; bundan sonra masaüstü ve
  gerektiğinde bulut. `GIS-40` adres arama ([ADR 0229](docs/adr/0229-address-search.md); kapsamı
  sahibin kurallarıyla ben belirledim) tek parçada, yalnız masaüstünde bitti (12 Ekim): yeni komutların arayüz kaydı
  `catalog/v1/additions.json` (komutlar, ikonlar, şerit yerleri; sözleşmenin kuralı `ui_catalog`, masaüstünün okuyucusu `catalog.rs`'in
  `place_additions`'ı; `catalog/README.md`); sözleşmenin `AddressLocator`'ı ve `cad_addresses`'i (`cad.addresses.define`), `.kcad` şema
  42 (`FORMATS_VERSION` 54); çekirdek `ops::geocode` (`fold`, `parse`, `similar`, `locator`, `forward`, `reverse`, `format`, `check`;
  harf maskesiyle kesin ön süzgeç, adaylar iki aşamada; bağımsız başvuru `geocode_cases.py`), sağlayıcılar `kentos_services::geocode`
  (`providers`, `request`, `read`, `score`; `geocode_online_cases.py`); masaüstünde `address/` (sekme, adres iş parçacığı `engine.rs`,
  sağlayıcıların ev sahibi `online.rs`, pencereler `data.rs` ve `providers.rs`), araç `kentos_interaction::address_reverse`, alanların
  tahmini `address_form`; İşlemler'in `kentos_processing::address`'i ve `builtin/address/`'i (ortak durumlar `address.json`); Python
  `kentos.address`; ortak izler `address-search.json`, `address-reverse.json` (yeni beklenti `address`), sahne `address_scene.py`; süreler
  ADR'nin Doğrulama'sında. Sağlayıcılarla canlı deneme yapılmadı. Ardından sahibin talimatı (12 Ekim): “Plan/CAD.md dosyası ile
  devam edelim”; planın §24.1'deki on iki iş TODOS.md §16.5'te (`CADP-01`…`CADP-12`). `CADP-01` CAD işlem manifesti
  ([ADR 0244](docs/adr/0244-cad-operation-manifest.md)) bitti (12 Ekim): eşleme `catalog/v1/operations.json`, üreteç
  `scripts/cad/manifest.py`, çıktı `docs/cad/manifest.{json,md}` (654 satır; CBS'ye özgü olmayan 449 satırın 243'ü Python ve AI için
  boşluk). `CADP-02` ([ADR 0247](docs/adr/0247-sheet-commands-on-the-desktop.md)) bitti: pafta komutlarının masaüstündeki yerleri
  `equivalents.json`'da, Ad ver yerinde (liste, ağaç, F2); `kentos-sheet-ui` masaüstü takımında. Sahibin izniyle (12 Ekim) iki paralel ajan: `CADP-03` (dal `cad-units`, ADR 0245; PR #21 birleşti: Python ve MCP'de yeni
  çizimin birimi ve sihirbazın parametreleri, cevaplarda `units`) ve `CADP-04` (dal `cad-offset`,
  ADR 0246; PR #22 birleşti: `cad.entities.offset`, araç komutun planıyla); ağır işler `/tmp/kentos-heavy.sh` kilidiyle sırayla. PR'leri ana ajan birleştirir; birleştirirken eşleme ve manifest
  yeniden yazılır.
  Paralel dal `gis-37-38-42-43` (sahibin sözü, 10 Ekim: “GIS-37, 38, 42, 43 maddelerini tamamla”, “Yeni branch içinde yap bunları”;
  ADR numaraları 0237, 0238, 0242, 0243): `GIS-37` çok ölçütlü uygunluk ([ADR 0237](docs/adr/0237-multi-criteria-suitability.md)) tek
  parçada bitti (10 Ekim): raster çekirdeğinde `suitability` (`pairwise`, `roc`), `ops`'un altı türü, `reclass::parse_with`; WASM
  notlarında `suit`, `roc()`; İşlemler'in `builtin/suitability/`'ı iki platformda; İşlemler'in iki yeni parametre türü (`rasterValues`,
  `rasterPairs`; masaüstünde `raster_rows`, web'de `fieldPlan.ts`'in satırları), tablo veren çalıştırmadan sonra tablo görünür olur;
  bağımsız başvuru `suitability_cases.py` (50 durum), ortak durumlar `fixtures/processing/v1/suitability.json`
  (`suitability_processing_cases.py`), resimlerin sahnesi `suitability.kcad` (`suitability_scene.py`); CBS'nin Raster sekmesinde Uygunluk
  analizi paneli, sekme 1100 px'e sığsın diye İnterpolasyon ve Yoğunluk tek panelde. `GIS-38` mekânsal istatistik
  ([ADR 0238](docs/adr/0238-spatial-statistics.md)) tek parçada bitti (10 Ekim): geometri çekirdeğinde `ops::spatial_stats` (`kdtree`,
  `centers`, `nearest`, `weights`, `autocorrelation`, `clusters`, `calls`; bağımsız başvuru `spatial_stats_cases.py`, 68 durum); web'in
  `model/ops/spatialStats.ts`'i; İşlemler'in `builtin/stats/`'u iki platformda; yeni katmanın görünüşünde kategorili görünüş
  (`NewLayerStyle.renderer`); ortak durumlar `fixtures/processing/v1/spatial-stats.json` (`spatial_stats_processing_cases.py`; yeni
  katmanın yeri `layerAbove`, karşılaştırıcılarda `r`, `major`, `ratio` geometri alanı), resimlerin sahnesi `spatial-stats.kcad`
  (`spatial_stats_scene.py`); CBS'nin Analiz sekmesinde (birleşimde Yakınlık'la Analiz paneline katıldı: ayrı panel 1100 px'te
  sığmıyordu). `GIS-42` uzaktan algılama
  ([ADR 0242](docs/adr/0242-remote-sensing.md)) tek parçada bitti (10 Ekim): raster çekirdeğinde `remote` (`spectral`, `classify`,
  `cluster`, `accuracy`, `work`; işin okuma geçişleri, `OpsFinished::Report`, notlarda `remote`, sonuca göre görünüş); bağımsız başvuru
  `remote_cases.py` (46 durum, Brovey GDAL'la bire bir); raster okuyucusunda ExtraSamples'ın 0'ı artık alfa değil (`raster_cases.py`'ye
  GDAL'ın alfa okuması ve iki dosya); İşlemler'in `builtin/remote/`'u iki platformda; ortak durumlar `fixtures/processing/v1/remote.json`
  (`remote_processing_cases.py`, `remoteOf`), resimlerin sahnesi `remote.kcad` (`remote_scene.py`); CBS'nin Raster sekmesinde Uzaktan
  algılama paneli (altı araç, ikisi ▾'de), sekme 1100 px'e sığsın diye Raster ve vektör ile Taranmış harita tek panelde. `GIS-43` mesh ve
  çok boyutlu veri ([ADR 0243](docs/adr/0243-mesh-and-multidimensional-data.md); sahibin sözü “o tamamen bitecek”) tek parçada bitti
  (10 Ekim): sözleşmenin `RasterDataset`'i ve görünüşün `edges`'i, `.kcad` şema 38 (`FORMATS_VERSION` 48; `main`'in şema 37'si proje değişkenlerinindir); biçim çekirdeğinde
  `multidim` (`netcdf`, `cf`, `ugrid`, `sms`, `mesh`, `cube`, `series`, `write`; bağımsız başvuru `multidim_cases.py` kendi NetCDF yazıcısı
  `netcdf_classic.py`'yle, GDAL ve QGIS'in MDAL'ıyla çapraz denetim), raster çekirdeğinde `multidim` (`points`, `profile`, `series`,
  `calc`); WASM `NetcdfFile`, `smsToUgrid`, `CubeOpening`, `MultidimAnalysis`; İşlemler'in `builtin/multidim/`'i iki platformda,
  `Files::open_cube`; masaüstünde `rasters/multidim.rs`, web'de `ui/raster/MultidimDialog.ts` ve çözümleme işçisinin birleştirilmiş
  okumaları (`readRuns`); ortak durumlar `fixtures/processing/v1/multidim.json` (`multidim_processing_cases.py`), resimlerin sahnesi
  `multidim.kcad` (`multidim_scene.py`). Yolda düzelenler: NetCDF okuyucusu `vsize`'ı işaretli okuyup 2–4 GiB'lık değişkeni reddediyordu;
  zaman sürgüsünün yazıları çekirdekten ve çubuk kendi genişliğine sığar (ADR 0210'un eki). Dal 10 Ekim'de `main`'e birleşti (sahibin
  sözü: “Görevin bitince github main ile birleştir ve main üzerine push yap”).
  `GIS-06` ve `GIS-07` mevzuatla
  düzenlenen işlerdir: yol haritasının en sonuna kalır, sahiple ayrı çalışma ister; §16.2 sırasında atlanır (sahibin kararı, 7 Ekim).
  `HYB-24` canlı GNSS ertelendi (sahibin kararı, 5 Ekim: elde alıcı yok); sıra gelince atlanır; sahip cihazı bulunca söyleyecek.
  4 Ekim: derleme ve test süreleri
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
  12 Ekim'den beri (ADR 0228) yalnız çekirdek, masaüstü ve gerektiğinde bulut. Resimler masaüstünün, izler masaüstünde oynatılır.
  Yalnız masaüstünde biten her madde TODOS.md §0.2'ye web'de gerekenleri yazar.

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
crates/shared/         contracts, geometry-core, expression, style-core, svg-core, formats, kcad, raster
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
Ayrıntı: ADR 0004/0008, TODOS.md §3/§20 ve [docs/specs/numeric-policy.md](docs/specs/numeric-policy.md) (dört tür sayı, toleranslar,
büyük koordinatta küçük ayrıntının testleri).

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
