# 0624 — CAD birimleri, doğrudan açılış ve özellik girişi

23 Ekim 2026. Dal: `codex/cad-inspector-units-open`. Bu çalışma ana proje klasöründe bu dalda sürer; `main` değiştirilmez.

## İnceleme: model, birim ve ölçek aynı şey değildir

Bir duvarın fiziksel boyu, sayının hangi birimde yazıldığı, yazı/ölçü boyutları ve kağıda basılan ölçek ayrı konulardır.
Model gerçek boyutundadır: 4 m duvar, mm gösteriminde 4000, metre gösteriminde 4 görünür. Paftanın 1:50 olması duvarı
modelde 80 mm yapmaz. Blok dönüşümünün birim oranı da kullanıcının X/Y yerleştirme çarpanından ayrıdır.

Bu ayrım yalnız dokümantasyondan çıkarılmadı; aşağıdaki kodlar okundu:

| Uygulama / incelenen kaynak | Kodun yaptığı | KentOS için sonuç |
| --- | --- | --- |
| CADCraft `37150774c6d614776e049ade4c474fec8c0ef32e`, `crates/doc/src/header.rs`, `crates/io/src/dxf_read.rs`, `crates/engine/src/cmd/props.rs`, `crates/doc/src/anno.rs` | Metrik şablon `INSUNITS=4`, `TEXTSIZE=2.5`, `DIMSCALE=1`; DXF okurken şablonun inç birimini dosyaya miras bırakmaz, birim yoksa `INSUNITS=0` yapar. `units` başlık/gösterim alanlarını değiştirir. Anotasyon oranı kağıt/model çiftidir. | Yeni yerel CAD için mm / 1:1 makul metrik şablondur. Açılan dosyanın birimi yeni projenin varsayılanıyla karıştırılmaz. Birim bildirmeyen kaynak biliniyormuş gibi sunulmaz. |
| OpenCADStudio `3edb433eb44af41eb8251dd53fd3b27a66ca98a4`, `src/ui/window/drawing_units.rs`, `src/app/properties.rs::insert_unit_scale`, `src/app/commands/layers.rs::convert_drawing_units` | Birim/gösterim penceresi geometriyi taşımaz. Yeni INSERT için kaynak/ev sahibi fiziksel birim oranı kullanır; bir taraf birimsizse oran tahmin etmez. DWGUNITS ayrı, açık geometri dönüşümüdür; model ve pafta görünüm pencerelerini ayırır. | Kaynak birimi düzeltmek, gösterim birimini seçmek ve ölçekle çizmek farklı işlemler olarak anlatılmalı. Model dönüşümü kağıda ikinci kez uygulanmamalı. |
| QCAD `src/core/RUnit.cpp` | Birimlerin metre karşılıklarını ayrı dönüşüm tablosunda tutar; birimler arasında fiziksel oranla dönüşür. | KentOS'un ortak SI dönüşümü korunur; tekrar metre/mm çarpanı uygulayan ikinci UI yolu eklenmez. |
| LibreCAD `librecad/src/lib/engine/rs_units.cpp`, çizim tercihleri ve baskı kılavuzu | `RS_Units::convert` kaynak/hedef birim çarpanıyla çalışır. Çizim birimi, kağıt/print scale ve ölçülendirme genel ölçeği ayrı tercihlerdir. | Birim ve pafta ölçeğini tek seçici gibi sunmak doğru değildir. |
| QGIS `src/core/qgsunittypes.cpp`, CRS/birim kılavuzu | Uzunluk/alan/gösterim dönüşümleri ayrı; CRS'nin harita birimi ile mm/point/map-unit sembol boyutları aynı kavram değildir. | CAD düzeltmesi GIS'in koordinat sistemi ve harita ölçeği davranışını değiştirmez. |

KentOS geometriyi SI metrelerde tutar; referansların ham çizim birimini depolaması bizim depolama mimarisini
değiştirmeyi gerektirmez. Önemli olan kullanıcıya verilen sayının, ölçünün, blok boyunun ve dosya çıktısının fiziksel
olarak aynı sonucu vermesidir. API uzunlukları da sözleşmede belirtildiği gibi metredir; arayüz mm seçildi diye
Python/MCP girdileri sessizce mm'ye dönmez.

### Birincil kaynaklar

- [CADCraft: metrik başlık](https://github.com/storytold/cadcraft/blob/37150774c6d614776e049ade4c474fec8c0ef32e/crates/doc/src/header.rs),
  [DXF okuma](https://github.com/storytold/cadcraft/blob/37150774c6d614776e049ade4c474fec8c0ef32e/crates/io/src/dxf_read.rs),
  [birim komutu](https://github.com/storytold/cadcraft/blob/37150774c6d614776e049ade4c474fec8c0ef32e/crates/engine/src/cmd/props.rs).
- [OpenCADStudio: birim penceresi](https://github.com/HakanSeven12/OpenCADStudio/blob/3edb433eb44af41eb8251dd53fd3b27a66ca98a4/src/ui/window/drawing_units.rs),
  [blok birim oranı](https://github.com/HakanSeven12/OpenCADStudio/blob/3edb433eb44af41eb8251dd53fd3b27a66ca98a4/src/app/properties.rs),
  [DWGUNITS](https://github.com/HakanSeven12/OpenCADStudio/blob/3edb433eb44af41eb8251dd53fd3b27a66ca98a4/src/app/commands/layers.rs).
- [QCAD birim kodu](https://github.com/qcad/qcad/blob/master/src/core/RUnit.cpp),
  [LibreCAD birim kodu](https://github.com/LibreCAD/LibreCAD/blob/master/librecad/src/lib/engine/rs_units.cpp),
  [LibreCAD baskı](https://docs.librecad.org/en/latest/guides/completion.html).
- [QGIS birim kodu](https://github.com/qgis/QGIS/blob/master/src/core/qgsunittypes.cpp),
  [QGIS birim/CRS tercihleri](https://docs.qgis.org/3.44/en/docs/user_manual/introduction/qgis_configuration.html).
- [Autodesk: modelde gerçek boyut](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/How-to-resize-or-rescale-an-AutoCAD-drawing.html),
  [INSUNITS](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-A58A87BB-482B-4042-A00A-EEF55A2B4FD8.htm),
  [blok ekleme birimleri](https://help.autodesk.com/cloudhelp/2025/ENG/AutoCAD-Core/files/GUID-410E4A05-F789-4416-9D63-3D24A46DD740.htm).

## Kararlar ve uygulama

### 1. Yeni CAD

- Yeni yerel CAD'in varsayılanı mm, yazı/ölçü ölçeği 1:1. Önceden açıkça kaydedilmiş yeni-proje birimi tercihi
  korunur; mevcut projelerin birimi veya geometrisi değiştirilmez.
- Gerçek koordinatlı CAD ve GIS metre/CRS ile devam eder; harita/plan başlangıcı 1:1000 korunur.
- Sihirbaz modelin gerçek boyutta olduğunu söyler. CAD ölçek alanı “Yazı ve ölçü ölçeği” olarak anlatılır; pafta
  ölçeğinin ayrı olduğu belirtilir. Durum çubuğunda yazı ölçeğinin yanında çizim birimi görünür.
- CAD/CBS proje türünün sonradan değiştirilebildiğini söyleyen bayat sihirbaz metni düzeltildi; tür kilidi korunur.
- Birim kartlarında simge ve ad ayrıldı; açıklama seçilen kartın altında gösterilir. Kartlar pencere genişliği ve
  kullanıcının yazı boyutuna göre 5/3/2/1 sütuna geçer. Dar kartta simge, ad, açıklama ve seçim işaretinin birbirine
  sıkışması giderilir. Depolamanın teknik ayrıntısı yerine modelin gerçek boyutta olduğu anlatılır.

### 2. Aç / İçe aktar

- Aç: KCAD, DWG, DXF veya NCZ. Önceden Yeni proje gerekmez. Kaynak kendi CAD çizimini açar; mevcut çizime eklemez.
- İçe aktar: mevcut projeye ekleme, katman yönlendirme, kullanıcı tarafından açık kaynak birimi veya bilinen uzunluk
  seçimi. Hedef projenin gösterim birimini değiştirmez. [ADR 0621](0621-cad-import-physical-scale.md) geçerlidir.
- Ortak native `cad_open::read` desktop, başsız oturum, Python `Document.open` ve MCP `drawing.open` tarafından
  kullanılır. Hesap arayüz iş parçacığında yapılmaz. Geç/iptal edilmiş cevap ve değişen açık çizim kapıları korunur.
- DWG/DXF bildirilen biriminde bir kez metreye normalleştirilir; gösterim birimi kaynağı takip eder. Bilinmeyen veya
  bildirilmemiş birim metrik şablonun mm varsayımıyla, **varsayım olduğu raporda belirtilerek** okunur.
  Desteklenen gösterim dışındaki fiziksel birimler (ör. mikrometre) metreyle gösterilir; fiziksel oran korunur.
- NCZ'nin kaynak metrelik koordinatları yeniden mm'ye çevrilmez; bilinen dosya CRS'si alınır. Bilinmeyen CRS
  tahminle atanmaz. CAD proje türü korunur.
- Kaynak katman adları ve görünüşleri, bloklar, metin/ölçü/çizgi stilleri ve paftalar ortak import uygulayıcısından
  geçer. Başlangıçtaki açılış, undo ile boşaltılacak bir import adımı değildir. Sonraki düzenlemeler normal undo'dur.
- Kaydet hedefi özgün DWG/DXF/NCZ değildir. İlk kayıt KCAD ister; native/API kaynak CAD uzantısına kaydetmeyi
  reddeder. Kullanıcı DWG/DXF çıktısı isterse ayrı dışa aktarma komutunu kullanır.
- Açılışın birim/atlama raporu masaüstü geçmişine, native/MCP özetindeki `openedCad` alanına ve Python
  `DocumentInfo.opened_cad` alanına verilir. Kaynak dosya son açılanlarda görünür.
- Okuyucunun mm yedek birimi, açılış raporunda “milimetrelik hedef projeye dönüştürüldü” diye sunulmaz;
  açılışta alıcı proje yoktur. Rapor kaynak birimini ve fiziksel boyutun korunduğunu söyler.

### 3. Özellik hücresinin gecikmesi

- `EditCell::update` düzenleme açıkken **her olayda**, metin değişmese bile `build` + `invalidate_layout` yapıyordu.
  İmleç hareketi, tuş bırakma ve caret blink bile bütün pencereye yeniden yerleşim yükü veriyordu.
- Artık yalnız metin veya düzenleme durumu değişince yerleşim geçersizleşir. İmleç/seçim/blink iç girişin yeniden
  çizimini kullanır. Enter/odak kaybında tek commit, Escape ile vazgeçme davranışı korunur.
- Özellik paneli yalnız çoklu seçimi önbelleğe alıyordu. Tek bir uzun çoklu çizgi veya karmaşık blok da pahalıdır;
  artık tek nesnenin geometrik satırları belge oturumu/generation/seçim sürümüne göre önbellektedir.
  Dosya/servis özeti ve eşzamansız ek dosya eylemleri canlı kalır.

## Birimi gerçek ölçüyle doğrulanmamış dosya

Downloads `2-1-ev-kat-plani.dwg` daha önce incelenmişti: dosya mm, yatak tanımı mm, mevcut INSERT çarpanı 0,1.
Bu, yeni ölçek-1 yatakla eski yatağın on kat farkını açıklıyor. Gerçek duvar uzunluğu bilinmeden dosya “aslında cm”
diye değiştirilemez. Açılışta **özgün çarpan korunur** ve blok boyu farkı raporlanır. Birim etiketi değiştirmek de
geometriyi tek başına düzeltmez. Bilinen bir uzunlukla açık kalibrasyon veya doğrulanmış kaynak birimi gerekir.
24 bağımsız açılış durumunun biri bu tutarsızlığı özellikle korumayı sınar; doğru dosyaları otomatik on kat bozmak
bir düzeltme sayılmaz. [ADR 0621](0621-cad-import-physical-scale.md) ölçülen fiziksel karşılaştırmayı içerir.

## Kabul kapıları

- `scripts/fixtures/cad_open_cases.py`: KentOS kullanmadan Decimal SI oranlarıyla 24 durum; mm/cm/m/in/ft ve
  küçük/büyük boyutlar, birimsiz/eksik/bilinmeyen başlıklar, kaynak-blok birim oranı, hatalı INSERT çarpanı.
- Native uygulayıcıda açılış, blok boyu, kaynak birimi, iptal/bozuk veri, başlangıcın undo olmaması.
- Headless/Python/MCP'de yeni proje olmadan açılış; kaynak üzerine yazmayı ret; KCAD kaydet/yeniden aç ve kaynak
  baytlarının korunması. NCZ ve gerçek DWG codec'i de aynı yoldan geçer.
- Özellik hücresinde gerçek fare/klavye, hızlı ondalık yazma, Enter/Escape/blur; 120 boş olayın pencere yerleşimini
  geçersizleştirmemesi. Tek nesne panelinin edit/undo/başka belge sonrası yenilenmesi.
- Masaüstünde gerçek klavye/fareli görsel kontroller ve Downloads örneklerinin açıkça seçilerek çalıştırılan
  aç/kaydet/yeniden aç denetimi.

## Bu çalışmada doğrulananlar

23 Ekim 2026, macOS / geliştirme derlemesi:

- Python SDK'nın **258 testi geçti**. Yeni CAD açılış senaryoları DXF mm/inç, gerçek DWG ve NCZ'yi açar;
  kaynak üzerine kaydı reddeder, KCAD kaydını yeniden açar ve kaynak baytlarını karşılaştırır. Metreyle yazılmış
  eski kısıt senaryosuna birimi açıkça verildi; çıplak parametre ifadeleri çizim birimindedir, geometrik API
  koordinatlarının SI metre olması kuralı değişmedi.
- Ortak native uygulama entegrasyonunda 142, headless'ta 49, MCP'de 27, proje kurallarında 30 ve ayarlarda
  10 test geçti. Native CAD açılışının iki testinden biri bağımsız **24 fiziksel birim/blok durumunu** yürütür.
- Masaüstü açılışının 8 ve özellik panelinin 38 testi geçti. Ortak etkileşim izlerinin tamamı üç varyantta
  (US, Türkçe Q, 2× ekran) geçti. İsteğe bağlı GPU kabulünde gerçek hücreye yazma/Enter/Escape geçti;
  11 karakterin yazılması açık temada yaklaşık 34 ms sürdü. Bu, geliştirme makinesindeki ölçümdür.
- Tek nesnede 100.000 köşeli çoklu çizgi: ilk özellik paneli yaklaşık **5,42 ms**, önbellekten 100 tekrarın
  toplamı **0,96 ms**. Tuş vuruşunda geometriyi yeniden ölçme yükü kaldırıldı.
- Gerçek `EditCell` olaylarının 5 ve sihirbazın 7 testi geçti. Sihirbazın birim/ayrıntı sayfaları 1440×900
  ve 1100×650'de, açık/koyu temada, 13/18 yazı boyunda görüntülendi. PNG'ler açılarak kartların, açıklama
  satırlarının ve sabit alt düğmelerin taşmadığı doğrulandı; uzun içerik dikey kaydırılır.
- Son açılış raporu düzeltmesinden sonra 24 native durum ve Python açılışının iki testi yeni derlemeyle
  yeniden geçti. Kaynak biriminin korunması ve olmayan bir mm hedef projeye dönüşüm anlatılmaması sınandı.
- Downloads'taki yedi dosya ayrıca açıldı, KCAD olarak kaydedilip yeniden açıldı; özgün kaynaklar değişmedi:
  `deneme_suşehri.dxf` (71.820 nesne), `imarss.dxf` (3.250), `2-1-ev-kat-plani.dwg` (282),
  `toki-3+1-sc3-plan.dxf` (201, inç), `SIVAS_UIP_2025_06_HAZIRAN.NCZ` (453.534, metre),
  `Tek-katli-verandali-koy-evi-dwg.dwg` (9.730), `2-1-ev-kat-plani (1).dwg` (282).
- Üretilen SDK, CAD manifesti (884 satır), şerit ve bağımsız açılış fixture'ları tutarlı; mimari bağımlılık
  denetimi 47 crate / 61 hedefte geçti. Web uygulaması testleri ADR 0228 gereği çalıştırılmadı.
- Masaüstü ve MCP çalıştırılabilirleri, Python native paketi derlendi. Son kaynaklarla masaüstü/native/Python
  `clippy --all-targets -- -D warnings` geçti. Bu derlemeler geliştirme profilindedir; işletim sistemi dağıtım
  paketi veya lisanslı AutoCAD kabulü yerine geçmez.

Web arayüzü ADR 0228 nedeniyle ertelidir; ortak ayar sözleşmesi/üretilen katalog sürer. Bu çalışma lisanslı AutoCAD
ile görsel/düzenleme uyumluluk kabulünün veya bütün dosya üreticilerinin sertifikasyonu değildir.
