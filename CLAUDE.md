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
  ölçülendirme: hizalı, doğrusal, açı, yarıçap ve çap (ADR 0061); tarama: kapalı nesneyle ya da çizgilerle, adalarıyla (ADR 0062);
  alan işlemleri: birleştir, kesiştir, çıkar, böl, alana ve çizgiye çevir, içine tıklayarak alan (ADR 0065);
  mesafe ölç, alan hesapla ve parsel oluştur (ADR 0067); seçili nesnelerin tutamaçları ve üzerine gelme kartı (ADR 0068);
  Hesap pencereleri: poligon hesabı, kutupsal alım, önden ve geriden kestirme, aplikasyon (ADR 0070, 0071);
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
  Web özellikleri envanter üzerinden adım adım masaüstüne taşınır. Masaüstünde yalnız şerit arayüzü
  vardır; klasik arayüz (menü çubuğu, araç çubuğu, kayan araç kutusu) yalnız web'dedir (sahibin kararı,
  27 Eylül); envanterde masaüstü için anlamsızdır.
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
| Axum/Tokio/SQLx API, PG/PostGIS, kimlik/tenant, `project.changes`, audit/outbox; proje sahipliği, kişisel alan ve paylaşım (ADR 0015); proje kataloğu ve yaşam döngüsü komutları, migration 0005 (ADR 0028); dosya olarak saklanan proje: doğrulanan yükleme, değişmez KCAD v2 revizyonları, klasör nesne deposu, migration 0006 (ADR 0031); veritabanı projesinin tek anlık `.kcad` görüntüsü (ADR 0033); kontrol noktaları (oluştur, listele, indir, sil) ve yeni proje olarak geri yükleme, migration 0008 (ADR 0034); bağlantıyla davet ve misafir, migration 0009–0010 (ADR 0035); yüklenen `.kcad`'in boş veritabanı projesine tek işlemde aktarımı (ADR 0036); öbür saklama biçimine yeni proje olarak dönüştürme (ADR 0039); katmanı düşen ağaç, katmanda nesne kaldıkça `@project` çakışmasıdır (ADR 0072) | `apps/api/`, `crates/server/` |
| İfade oluşturucu (ADR 0100 §5, DESIGN.md §7.16): QGIS'in ifade penceresi gibi; İşlemler'in ifade alanının yanındaki ε ile açılır: renkli düzenleyici, tamamlama (Ctrl+Boşluk), imza, hata yerinde, işleç düğmeleri, nesne nesne önizleme, aranan ağaç, yardım ve alan değerleri; Tamam alana yazar. Masaüstünde önizleme nesnesi çizimden seçilir (`PickObjects::one`). Metin \| Akış (ADR 0101): aynı ifade düğümlerle; ağaçtan sürükleyip bırakma, çıkışı girişe bağlama, düğüm başına değer, denetçi, geri alma; her değişiklik metnindir | `ui/expression/`, `model/expression/builder*.ts`, `model/expression/flow.ts`, `apps/desktop/src/expression/` |
| Web cloud aç/yükle, autosave, IndexedDB taslak, WS/reconnect ve conflict, paylaşım penceresi, “Benimle paylaşılanlar”, açık projede rol/erişim değişikliği (ADR 0024), proje kataloğu: listeler, sunucuda arama/sayfalama, bilgiler, kopya, arşiv, çöp kutusu (ADR 0028); dosya projeleri (aç, Kaydet ile revizyon, çakışma), `.kcad` indirme, tek içe aktarımla yükleme, geçmiş ve kontrol noktaları, dönüştürme (ADR 0038); e-postayla davet, tek gösterimlik bağlantı ve kabul sayfası (ADR 0042); 8 MiB üstü dosya parçalı ve kaldığı yerden yüklenir, kesilen indirme `Range` ile sürer (ADR 0045) | `app/cloud/`, `ui/cloud/` |
| KentOS UI bileşenleri (Iced 0.14) ve vitrini | `crates/ui/`, `apps/ui-showcase/` |
| Masaüstü kabuğu: şerit, katmanlar, özellikler, komut satırı, `.kcad` aç (v1, v2; aşamalı, durdurulabilir) / kaydet (v2; kendi iş parçacığında, paneli ve durdurmasıyla; geçici dosya ve doğrulama), kurtarma kopyası (ADR 0030), geri al/yinele; wgpu çizim alanı (çizgi/eğri/dolgu/nokta, kaydır/yakınlaştır); kapalı alan, çizgi ve çoklu çizgi araçları, değer alanı ve web'in tuş anlamları; seçim (tıklama, Shift, pencere/kesişim, üzerine gelme), kenet (F3), Sil; seçili nesnelerin tutamaçları (sürükle, sıcak tutamaç, yazılan nokta, Enter/Esc; kenar ortası yeni köşe) ve üzerine gelme kartı (ADR 0068; kenarda imlecin öbür yanına geçer, uzun adı kaydırır, değer alanı ve ipuçları da aynı kuralla, ADR 0082), sağ tıkta tutamaç menüsü (ADR 0074); Nokta hesapla: komut nokta beklerken altı yapı (yan nokta, kenar ve doğru kesişimi, hat üzerinde, açı ve mesafe, orta), noktası askıdaki komuta gider; takma adla, komut menüsünden ya da komut satırının çipinden (ADR 0083); İşlemler: Köşe noktalarını numarala, Kenar uzunluklarını yaz, Öznitelik hesapla, İfadeyle seç ve Parsel ölçü yazıları modeli; web'in `ToolDialog`'u gibi tanımdan üretilen pencere (Girdi, Ayarlar, Çıktı, Gelişmiş; açıklama, önizleme, Nerede çalışır, takma adlar; Sonuçları seç, Geri al; Haritadan göster), komutları ve takma adlarıyla, tek geri alma adımında yazar, son değerler `islemler.json`'da; alanlar çizimden seçilir (Sahneden seç: nokta, numaralamanın başlangıç köşesi, girdi nesneleri alanın türleriyle; ADR 0088); sağ dokta Katmanlar'ın yanında İşlemler sekmesi: araç kutusu (arama, kategoriler, Modeller) ve oturumun geçmişi (Yeniden aç, n nesneyi seç) (ADR 0084); Model tasarımcısı: web'inki gibi akış diyagramı (girdiler, adımlar, bağlantılar, kaynaklarıyla ayarlar, kendi geri alması), Kitaplığa ve `islemler.json`'a kaydeder, hazır modeli kopyası olarak açar, kullanıcının modelleri araç kutusundan çalışır (ADR 0116); Hesap pencereleri: Poligon hesabı (bağlı, kapalı, açık; kapanma hataları ve dengelemesi), Kutupsal alım (kotlarıyla), Önden ve Geriden kestirme (krokisiyle), Aplikasyon; web'inki gibi ölçü tablosu (Enter, ↑/↓, satır ekle/sil, elektronik tablodan yapıştırma; ADR 0075); bilinen nokta adla, Y,X ile ya da çizimden; rapor sistem panosuna, noktalar `cad.entities.create` ile pencerenin adındaki tek adımda çizime (ADR 0070, 0071); web'inki gibi düzenlenen Öznitelikler paneli: katman, renk, sembol, türe göre geometri değerleri, öznitelikler, çoklu seçimin ortakları ve toplamları (ADR 0063), ürün komutlarıyla yazar (ADR 0066); nokta, daire, yay, dikdörtgen ve düzgün çokgen araçları, şeritte yöntem menüleri; elips, eğri, yardımcı çizgi, ışın, paralel çizgi, dik in ve dik çık, halka, revizyon bulutu, kot noktası ve böl (ADR 0057); Yazı ve çizimin üstündeki yazı kutusu, çift tıkla yazı ve ölçü değeri düzenleme (ADR 0060); Ölçülendirme (hizalı, doğrusal, açı, yarıçap, çap) ve komut satırında değerlerin yazıldığı gibi yankısı (ADR 0061); Tarama: kapalı nesneyle ya da çizgilerle, adalar, sınır katmanı, dört desen (ADR 0062); alan işlemleri: birleştir, kesiştir, çıkar (iki seçimle), böl (noktalarla ya da çizgiyle, canlı parçalarıyla), alana ve çizgiye çevir, içine tıklayarak alan (ADR 0065); Mesafe ölç, Alan hesapla ve Parsel oluştur: yol aracının biçimleri, parsel `parsel` katmanına numarasıyla, tapu alanı boş (ADR 0067); taşı, kopyala, döndür, ölçekle ve aynala (ADR 0021, 0027, 0029, 0032, 0037); ötele, buda, uzat, köşe yuvarla, pah, kır, birleştir, patlat, uzat-kısalt, köşe ekle/sil, esnet, dizi, kutupsal dizi, hizala (ADR 0047); çizimin üstünde komut şeridi (`drafting.commandBar`); yeni katman ve grup, DXF ve koordinat listesi al/ver (ADR 0048); GeoJSON al/ver, Shapefile al (dosyaları ya da `.zip`, ADR 0053); yeni proje ve proje ayarları (ADR 0049); uygulama menüsü, Başlangıç ekranı ve son dosyalar (ADR 0050); pencereye sığan şerit (paneller adım adım küçülür, seyrek araçlar ▾) ve Görünüm sekmesinde tema, vurgu, çizim zemini, yazı tipleri ve yazı boyutu (ADR 0051); şeridin hızlı erişim çubuğu (▾ menüsü, sağ tıkla ekleme ve çıkarma), sağ tık menüleri ve bölünmüş düğmenin son seçimi, `yerlesim.json`'da (`fixtures/shell/v1/ribbon.json`, ADR 0117); şeridin kendi panelleri: Giriş'te Katmanlar (etkin katman) ve Özellikler (yeni nesnelerin rengi, çizgi tipi ve kalınlığı, çizim ölçeği), seçim varken sayısıyla bağlamsal Seçim sekmesi; çizim araçları etkin rengi komutun girdisine yazar (ADR 0089); komutlarda web'in ikonları (envanterden) ve §7.8'e uyan ipucu (ADR 0054); çizimin yazıları (yazı nesnesi, ölçü değeri, etiket; projenin yazı tipiyle, haleli, seyreltilmiş), ölçü çizgileri ve görünüme kırpılmış yardımcı çizgiler (ADR 0055); pano: Kes, Panoya kopyala, Yapıştır (aracıyla) ve Özgün koordinatlara yapıştır, oturumun panosuyla (sistem panosu yok); Kes `cad.entities.delete` ile, yapıştırma `cad.entities.create` ve `cad.entities.set` ile yazar (ADR 0074); Kaydır, Pencere yakınlaştır, Seçime yakınlaştır, Son komutu yinele (ADR 0056); alt panel (F2: komut geçmişi, koordinat listesi, uyarılar; temizle, kapat, sürükleyerek boy; web'in günlüğü: saatli ve düzeyli satırlar, 500 satır, Uyarılar rozeti, tek satırlık komut satırı ve durum çubuğunda son ileti, ADR 0114); web'in kabuk düzeni ve kalıcı yerleşimi: dok bütün yükseklikte, alt panel çizimin altında, boyutlar web'in kurallarıyla, `yerlesim.json` (ADR 0115), çizimde ızgara (F7) ve durum çubuğunda çizim yardımcıları, dar pencerede çekilen hücreler (ADR 0080), seçimi izleyen, web'inki gibi katman ağacı (etkin katman değişmez; göz, kilit, renk menüsü, çift tıkla etkin yapma, sağ tık menüsü, yeniden adlandırma, katman ya da grubu nesneleriyle tek adımda silme, ADR 0072; Katman ara ve ağacın klavyesi, ADR 0075), durum çubuğu arayüzün yazısıyla, F4 ile katman ve özellikler panelleri, tam ekran (sekme satırında düğmesi), Komut ara (Alt+Q; sekme satırındaki kutu, sonuç listesi, şeritte yerini gösterme, ADR 0077), Koordinat sistemi… ve sunucu denetimi (ADR 0058); sekme satırında web'inki gibi kaydedilmemiş noktası, Komut ara kutusu, koordinat sistemi düğmesi ve Yardım menüsü, dar pencerede adım adım (ADR 0064, 0077); çizim alanında web'in üç sağ tık menüsü (kısa tık, basılı tutma, Shift) ve tek seferlik kenet (ADR 0059); çizim alanında web'in artı imleci (kol boyu `appearance.crosshair`, nesne isteyen araçta kısa kollu ve seçim kutulu), kuzey oku ve ölçek çubuğu (ADR 0110); Nesne izleme: kenette durarak nokta alma ve bırakma, hizalara ve kesişimlerine kilitlenme, hizada yazılan mesafe, Shift+F3 (ADR 0085); çalışma modunun şeridi süzmesi ve durum çubuğunda mod (ADR 0052); bulut arayüzü (ADR 0041): giriş, katalog ve bu cihazdaki projeler, çevrimiçi ya da yerel kopyadan açma, dosya projesinin revizyonu, veritabanı projesinin kendiliğinden kaydı ve cihaz taslağı, başkalarının değişiklikleri, çakışma, buluta yükleme; açık projeyi yeniden adlandırma, çöpe taşıma, son revizyonu açma ve buluta dosya olarak kaydetme (ADR 0073); katalogda web'in yedi listesi (çöp kutusu dahil), arama, tür, sıralama ve kurum seçimi, seçili projenin bilgileri, favori, arşiv, çöp kutusu, geri yükleme, kalıcı silme ve `.kcad` indirme, web'in planıyla ve cihazın saatiyle (`fixtures/cloud/v1/catalog.json`, ADR 0086); Proje bilgileri, Kopyasını oluştur ve öbür saklama biçimine çevirme formları (`fixtures/cloud/v1/forms.json`, ADR 0112); Geçmiş sekmesi: kontrol noktaları ve revizyonlar, oluşturma, indirme, silme, yeni proje olarak geri yükleme ve açma, olayları bekleyerek izlenir (ADR 0087); durum çubuğunda web'in kayıt hücresi (aşamaları, lambası, tıklanınca sıradaki iş) ve sunucu hücresi, arkasında hesap menüsüyle (`fixtures/cloud/v1/cells.json`, ADR 0113); Projeyi paylaş: erişenler ve kaynakları, rol değiştirme ve kaldırma, adla ya da e-postayla kişi arama (KentOS UI `Suggest`), e-postayla davet ve tek gösterimlik bağlantı, katalogdan ya da açık proje için `cloud.share` (`fixtures/cloud/v1/share.json`, ADR 0111); veritabanı projesinde `project.edit`'i olmayan katman ağacını değiştiremez (ADR 0078); başkasının sildiği katman, üzerinde gönderilmemiş nesne varken kalır ve geri gönderilir (ADR 0079); başkasının nesnesi çizimde olmayan katmanını bekler, katman gelince alınır; sunucu silmeyi reddedince başkasının kullandığı katman geri verilir (ADR 0081) | `apps/desktop/`, `apps/desktop/src/cloud/`, `apps/desktop/src/exchange/` |
| Native wgpu çizim hattı ve paylaşılan WGSL sözleşmesi; stilli katmanlar web'in `shaders/wgsl/styled` boru hatlarıyla, atlas tiny-skia ile; sözleşmenin 2. sürümünde atlas iki platformda çerçevenin grubunda (Iced'in iki bağlama grubu, ADR 0090) | `crates/render/wgpu/`, `shaders/wgsl/` |
| Masaüstünün stil sayfası (`kentos-native-style`, ADR 0090): kitaplık (sistem, kullanıcı, proje; sistem kitaplığı web'in TypeScript'inden yazılan `.kstil`), düz görünüş, katmanın programı ve ifade tablosu, toplulukların renkleri ve atlas görüntüleri; `fixtures/style/v1/batches.json`'ı geçer. Masaüstünde gösterilen her katman stil motorundan çizilir, katmanlar yan yana kurulur; SVG çizimleri roxmltree ve SVG çekirdeğinin okuyucularıyla (`apps/desktop/src/style/`). Katman stili penceresi (ADR 0091): beş işleyici, veriden sınıflar (`classify`, `classify.json`), işleyicinin tipli JSON'u (bilinmeyen alan ve tür korunur), Nesne sütunu çizimin sayısıyla (`tally`, iki platformda `tally.json`), sembol resimlerini çizimin stilli boru hatları çizer (`style/thumbs.rs`). Stil yöneticisi (ADR 0092): kaynak ve kategori ağacı, arama ve türler, kartlar, ayrıntılar ve alanları, kopyala, sil, `.kstil` ve pano ile içe ve dışa aktarma (`file`, iki platformda `kstil.json`), PNG ve JPEG görüntüler; `style.assign`, `style.clearSymbol` ve Katman stili yuvası için seçme kipi; Kitaplığım `kitaplik.kstil`'de, projenin kitaplığı çizimde (düzenleme, geri alma adımı değil). Lejant (ADR 0093): katman katman satırlar (`legend`, iki platformda `legend.json`), katman bırakma, PNG beyaz kâğıtta 2× (ekransız çizici, şerit şerit). Sembol tasarımcısı (ADR 0094): katman yığını, işaretin katmanları, canlı önizleme, türe göre formlar ve “ƒ”, kendi geri alması; modeli (katman türleri, yeni katman, özet, form yaması, liste düzenlemeleri) `designer`, iki platformda `designer.json`; Stil yöneticisinden (Düzenle, çift tık, Yeni sembol; sistem sembolünün kopyası) ve Katman stili yuvasından (Uygula yuvaya yazar) açılır; form parçaları `style/fields.rs`. SVG çizim düzenleyicisi (ADR 0095, `style/svgedit/`): dokuz araç, şekil listesi, cetvel ve kılavuzlar, kenet, düğüm aracı, ölçü, dört sekme, Yol, Nesne ve Seç menüleri, web'in tuşları, kendi geri alması; aç, içe alma penceresi, pano, bırakma, kitaplıktan aç, farklı kaydet, dışa aktarma (SVG, PNG), belge özellikleri, XML kaynağı, izleme altlığı, Bitmap izle; geometri SVG çekirdeğinden, kurallar (`svgEditModel.ts` ↔ `svgedit/`) iki platformda `svgedit.json`; Stil yöneticisinden, Sembol tasarımcısından ve `style.svgEditor` ile açılır, Kitaplığım'a yazar | `crates/native/style/`, `apps/desktop/src/style/` |
| Masaüstü belgesi (`kentos-domain`): web `CadDocument`'inin anlamı native olarak, ortak işlem fixture'larıyla sınanır | `crates/native/domain/`, `fixtures/document-ops/` |
| Masaüstünün bulut istemcisi (`kentos-cloud`, ADR 0040, 0043): yerel hesapla giriş, katalog, iki tür projeyi açma, dosya projesine revizyon, çizimden yeni proje, veritabanı projesine değişiklik, başkalarının değişiklikleri (olaylar sorarak izlenir, belgeye `apply_external` ile gelir), çakışmada benimki ya da sunucudaki, cihaz taslağı, çevrimdışı çalışma için projenin yerel kopyası; arayüzü `apps/desktop/src/cloud/` (ADR 0041); WebSocket yok, olaylar bekleyerek sorulur (ADR 0044) | `crates/native/cloud/` |
| Masaüstünün işlem araçları (`kentos-processing`, ADR 0084): web'in `processing/` modüllerinin yerli karşılığı: parametreler ve doğrulama iletileri, kapsamlar, çalıştırıcı (kilitli katman, boş girdi, tek adım, yeni katman aynı adımda), modeller (`begin_group`, adım çalışmazsa geri alma), kayıt ve arama, dört yerleşik araç ve Parsel ölçü yazıları; modellerin düzenlenmesi ve tasarımcının planı (`model_edit`, `designer`; iki platformda `fixtures/processing/v1/designer.json`, ADR 0116); hesap ortak çekirdekten (köşe numaralama, kenar ölçüleri, ifade dili); iki platform `fixtures/processing/v1`'in 21 durumunu ve varsayılanlarını geçer | `crates/native/processing/`, `apps/desktop/src/processing/`, `fixtures/processing/` |
| Masaüstü araç oturumu (`kentos-interaction`): durumlar, veri olarak istem, kapalı alan/çoklu çizgi (tek yol aracı), seçim aracında tutamaçlar (deponun `grips`'i, çekirdeğin `move_grip`'i; ADR 0068) ve tutamaç menüsü (`grip_menu`, ADR 0074), çizgi, nokta, daire, yay, dikdörtgen, döndürülmüş dikdörtgen, düzgün çokgen, seçim ve Sil araçları; elips, eğri, yardımcı çizgi ve ışın, paralel çizgi, dik in ve dik çık, halka, revizyon bulutu, kot noktası ve böl; önizlemede dolgulu alan, kısa yazı, konacak nokta ve dik açı işareti (ADR 0057); Yazı: ev sahibinden yazı kutusu ister (`ViewChange::Text`), yazılanı `Tool::text_typed` ile alır (ADR 0060); Ölçülendirme (`dimension`): beş biçim, kenar ve daire seçme, işaretli yazılan mesafe (ADR 0061); Tarama (`hatch`): bölge deponun `enclosing`'inden ya da görünen çizgilerin yüzlerinden (`FaceIndex`), önizlemede kesikli alan ve soluk tarama çizgileri (ADR 0062); alan işlemleri (`area`, `boundary`): seçimden önce seçen tabana ikinci seçim, aşamanın kendi önizlemesi ve fareyi okuması eklendi, görünen çizgilerin yüzleri Tarama ile ortak (`faces`, ADR 0065), ürün komutlarıyla yazar (ADR 0069); çizimden nokta alma (`pick`, `ViewChange::Picked`, ADR 0070); askıya alma (`Session::nest`, `Tool::accept_point`) ve nokta hesaplayıcı (`point_calc`, ADR 0083); nesne izleme (`object_tracking`, `Pointer::tracked`, `Context::track_along`, ADR 0085); Öznitelikler'in ve yazı kutusunun yazmaları (`properties`: `cad.entities.set`, `cad.entities.edit`'in `properties` işlemi, ADR 0066); taşı, kopyala, döndür, ölçekle ve aynala (seçimden önce seçen ortak taban `modify`); kenar seçen taban (`edge`) ve on değiştirme aracı; esnet, dizi, kutupsal dizi ve hizala; Esc ile bir adım geri (`Tool::cancel`); istemin notları (ADR 0047); pano (`clipboard`) ve katalog dışı yapıştırma aracı (`paste`, `Session::run`, son komut sayılmaz); Kaydır ve Pencere yakınlaştır (`navigate`): araç görünüm değişikliğini `ViewChange` olarak ister, kabuk uygular; onay almayan araçta Enter son komutu yineler (ADR 0056); araçların oturum belleği (`Memory`); belgenin günlüğüyle izlenen geometri deposu (`Spatial`, ADR 0029); iki platform `fixtures/interaction/v1` izlerini ve `fixtures/point-input/v1` dilbilgisini geçer | `crates/native/interaction/` |
| Ürün komutları `cad.polygon.create`, `cad.line.create`, `cad.polyline.create`, `cad.point.create`, `cad.circle.create`, `cad.arc.create`, `cad.entities.delete`, `cad.entities.transform` v1: web ve masaüstü işleyicileri, `CommandResult`, katalogla eşit kayıtlar; kapalı alan, çizgi ve çoklu çizgi araçları bu komutlardan yazar, nokta, daire ve yay araçları kendi komutlarından, dikdörtgen ve düzgün çokgen `cad.polygon.create`'ten yazar, Sil aracı `cad.entities.delete` ile siler, değiştirme araçları `cad.entities.transform` ile yazar (ADR 0022, 0027, 0029, 0032, 0037); `cad.entities.edit` v1: kenar, köşe ve nesne araçları, Esnet, tutamaçlar ve tutamaç menüsü (`grip`, `vertexAdd`, `vertexRemove`, `straightEdge`, `arcEdge`; ADR 0074) bununla yazar; `cad.entities.array` v1: Dizi ve Kutupsal dizi; Hizala `cad.entities.transform`'un `align` dönüşümüyle (ADR 0047); `cad.entities.create` v1: kendi komutu olmayan çizim araçları (elips, eğri, yardımcı çizgi, ışın, halka; yazı, ADR 0060; ölçü, ADR 0061; tarama, adı “Tarama”, ADR 0062) ve birden çok nesneyi tek adımda yazanlar (paralel çizgi, dikler, böl) bununla yazar; revizyon bulutu `cad.polygon.create`, kot noktası `cad.point.create` ile (ADR 0057); `cad.entities.set` v1: nesnelerin katmanı, rengi, sembolü, öznitelikleri ve etiketi; iki platformda Öznitelikler bununla, geometri değerleri ve yerinde yazı düzenleyicisi `cad.entities.edit`'in `properties` işlemiyle yazar; yazının boş metni reddedilir (`empty_text`, ADR 0066); alan işlemleri `cad.entities.edit`'in `areaUnion`, `areaIntersect`, `areaSubtract`, `areaSplit`, `toArea`, `toPolyline` işlemleriyle, İçine tıklayarak alan `cad.entities.create`'in `boundary` işlemiyle yazar; kapalı alanın halkası yaylı kenarla 2 köşeli olabilir (ADR 0069) | `product/`, `crates/native/application/`, `fixtures/commands/` |
| Tipli ayarlar: şema, katmanlı çözüm, ortak durumlar; web servisi, masaüstü ayar dosyası ve penceresi, canlı MSAA/HiDPI (ADR 0023) | `crates/shared/contracts/src/settings/`, `core/settings/`, `app/settings/`, `apps/desktop/src/settings*.rs`, `fixtures/settings/` |

Masaüstünde aplikasyon ve ifraz araçları, şeridin harf ipuçları, embed Python, tam AI yüzeyi,
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
pnpm e2e:interaction     # etkileşim izleri: poligon kabul izi, tuş anlamları (fixtures/interaction, ADR 0018)
cargo test -p kentos-desktop traces   # aynı izler masaüstünde, pencere açmadan (ADR 0021)
apps/desktop/scripts/cloud-live.sh   # masaüstünün bulut arayüzü gerçek kentosd ile (geliştirme veritabanı; görüntüler .run/shots/bulut-*; ADR 0041)
cargo test -p kentos-processing   # işlem araçlarının ortak durumları masaüstünde (fixtures/processing/v1, ADR 0084)
cargo test -p kentos-native-application   # ürün komutlarının durumları masaüstünde (fixtures/commands, ADR 0022, 0027, 0029, 0032, 0037, 0047, 0057, 0066)
python3 scripts/fixtures/transform_command_cases.py --check   # cad.entities.transform durumlarını dönüşümlerin tanımından denetle (ADR 0037)
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
cargo test -p kentos-desktop ribbon_bar::screens -- --ignored --nocapture   # şeridin hızlı erişim menüsü, sağ tık menüleri ve bölünmüş düğme listeleri, .run/shots/serit-* (web'inkiler: node apps/web/scripts/e2e/shots.mjs ribbon; ADR 0117)
KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=olc,izle cargo test -p kentos-desktop style::svgedit::screens -- --ignored --nocapture   # SVG düzenleyicisi resimleri, .run/shots/svge-* (değişken yoksa bütün durumlar; altlık ve izleme wgpu ister; web'inkiler: node apps/web/scripts/e2e/shots.mjs svgedit; ADR 0095)
GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-svgedit.test.ts   # SVG düzenleyicisinin kurallarını fixtures/style/v1/svgedit.json'a yeniden yaz; farkı okuyun (ADR 0095)
KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings   # ayar şeması değişince settingsSchema.json'u yeniden yaz (ADR 0023)
cargo run -q -p kentos-kcad --bin kcad -- inspect|validate|sniff DOSYA   # KCAD v2 dosyasını incele (ADR 0025)
python3 tools/kcad/kcad.py validate DOSYA   # bağımsız Python okuyucusu
python3 scripts/fixtures/kcad_v2_reference.py --check   # örnek dosyaları bağımsız yazıcıyla denetle
python3 scripts/fixtures/gis_reference.py --check   # GeoJSON/Shapefile fixture'larını bağımsız okuyucuyla denetle (ADR 0046)
python3 scripts/fonts/drawing_fonts.py --check   # masaüstünün çizim yazı tiplerini web'in WOFF2'lerinden denetle (ADR 0055)
KENTOS_WRITE_GIS_EXPORTS=1 cargo test -p kentos-formats --test gis   # GeoJSON yazıcısının örnek çıktısını yeniden yaz; farkı okuyun
pnpm e2e:cloud           # gerçek API/PostGIS cloud akışı
KENTOS_E2E_DB=scratch pnpm e2e:cloud   # aynı akış geçici veritabanında (bu yapının migration'ları), kentos_cad'e dokunmadan
KENTOS_E2E_SERVER=…/target/debug node apps/web/scripts/e2e/cloud.mjs   # aynı akış başka bir yapının sunucusuyla (ADR 0038)
node scripts/wgsl/browser-check.mjs   # paylaşılan WGSL'yi Chrome WebGPU'da derler ve çizer
KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu   # gerçek GPU'da hassasiyet
pnpm perf:interaction    # etkileşim ölçümleri
pnpm perf:kcad           # KCAD v2 kaydet/aç ölçümü, tarayıcıda (ADR 0030; docs/perf)
cargo test --release -p kentos-desktop perf::kcad -- --ignored --nocapture --test-threads=1   # aynı ölçüm masaüstünde
pnpm inventory           # web özellik envanteri: docs/inventory/web.{json,md}
pnpm inventory:check     # envanter güncel değilse düşer
pnpm db:setup            # yalnız yerel geliştirme DB/rolleri, migration, seed
pnpm api                 # kentosd serve; varsayılan 127.0.0.1:8787
pnpm kentosd -- <komut>  # yönetim CLI; yetkili hedefte bilinçli kullanılır
```

- Rust araç zinciri sabittir; `.cargo/config.toml` derlemeyi 4 işle sınırlar.
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
  altında, her çalışan KentOS'un kilitli klasöründe (ADR 0030). Hata ayıklarken kullanıcı
  verisini izinsiz silmeyin.

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
- CPU kaynak koordinatı f64; GPU'ya yerel orijine göre float32 fark gider.
  Mutlak büyük dünya koordinatını GPU'ya yüklemeyin.
- Kirli katmanlar güncellenir; seçim/hover/ızgara ayrı cache/katmanlardır.
  Masaüstünde seçim ve üzerine gelme ayrı wgpu sahne parçasıdır; kenet işareti ve
  seçim kutusu Iced canvas'ıdır (ADR 0029). Çizimin yazıları da, web'deki gibi sahnenin
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

### 4.12 Çalışma modları

`app/workspaces.ts` içindeki CAD/CBS/Hibrit modları sunuşu süzer; veri modelini
ve hesapları değiştirmez. Mod proje ayarıdır. Gizlenen komutun kısayoldan
erişilebilir olması yetki verildiği anlamına gelmez; UI modu güvenlik sınırı değildir.

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
crates/ui/             KentOS UI bileşen kütüphanesi (kentos-ui)
crates/native/         native belge (domain), ürün komutları (application), araç oturumu (interaction), işlem araçları (processing) ve bulut istemcisi (cloud); web'e derlenmez
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
