# ADR 0165: Proje türleri — CAD ve CBS ayrı sahneler, proje sihirbazı

- **Durum:** kabul edildi (2026-10-03). Sahibin kararları (3 Ekim, dört soru; hepsinde önerilen seçenek). Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-03
- **Bağlam belgesi:** ADR 0049 (yeni proje ve proje ayarları), ADR 0052 (masaüstünde çalışma modları), ADR 0051 (şeridin pencereye sığması), ADR 0110 (artı imleci, kuzey oku, ölçek çubuğu), ADR 0164 (pafta düzenleri ve kip profilleri), ADR 0025 (`.kcad` v2), ADR 0023 (tipli ayarlar); CLAUDE.md §4.12 ve §5; DESIGN.md §7.3.2; [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md).

## Bağlam

Sahibin isteği (3 Ekim): “hibrit mod bence çok saçma, bazı araçlar hem CAD hem de gis olabilir ama hibrit mod olmaması lazım; ya CAD ya gis ya da ileride yapacağımız modlar … proje oluşturma sihirbaz ile olsun, ilk aşamada çok şık şekilde kişi proje türünü seçsin, sonraki aşamada hangi projeksiyon ya da CAD için hangisi uygunsa onu sorsun … sahnede CAD ve gis modları farklı, hem xy yapısı farklı hem de diğer durumları … yeni proje 1/2 milyon ölçeğinde açılıyor, bunun da seçilebilmesi lazım … menüleri bu yapılara göre düzenleyelim.”

Bugün:

- **Çalışma modu** (ADR 0052, `app/workspaces.ts`): Hibrit (varsayılan, her şey), CAD, CBS; 3D Plan ve Afet Analizi “Yakında”. Mod yalnız şeridi süzer, veriye ve sahneye dokunmaz.
- **Tek düzen:** her modda Türk ölçmeciliğinin düzeni: sağa Y, yukarı X, açı birimi grad (proje ayarı). Ama kutupsal giriş `@mesafe<açı`'nın açısı dereceyle, doğudan saat yönünün tersine ölçülür (`fixtures/point-input/v1`, “@10<90” yukarı) — iki düzen karışıktır.
- **Her proje bir koordinat sistemi taşır** (SRID zorunlu; KCAD, sunucu, PostGIS denetimi). Teknik çizim için “koordinat sistemi yok” seçeneği yoktur; pafta profilleri ise “CAD, koordinat sistemi yok” durumunu zaten tanır (`fixtures/sheet/v1/profiles/modes.json`).
- **Yeni proje penceresi:** ad, ölçek düğmeleri, mod kartları ve uzun bir koordinat sistemi listesi tek sayfada. Masaüstünde yeni proje yaklaşık 1:500 000'de açılır: başlangıç paftası (50 × 37,5 cm, ölçekte) çizim alanı boyut almadan önce 1 × 1 piksellik kameraya sığdırılır, sonra boyut gelince ölçek öyle kalır.

Araştırma (öbür programların sahnesi):

- **AutoCAD (model uzayı):** X sağa, Y yukarı; koordinat “X,Y”; açılar derece, doğu 0°, saat yönünün tersine; birimsiz çizim birimi (mm, cm, m); 0,0 başlangıç, sol altta koordinat ekseni simgesi (UCS); koyu zemin; ızgara; açıklama ölçeği; kuzey oku ve ölçek çubuğu yok.
- **QGIS, ArcGIS Pro (harita):** projenin koordinat sistemi ve harita birimi; durum çubuğunda koordinat (doğu, kuzey) ve yazılabilen ölçek (1:N); açık zemin; kuzey oku ve ölçek çubuğu süs olarak; katman ve öznitelik odaklı menüler.
- **Netcad, TKGM (Türk ölçmeciliği):** sağa Y (doğu), yukarı X (kuzey); koordinat listesi “Y, X”; açı grad (400ᵍ), doğrultu kuzeyden saat yönünde semt; TUREF/ITRF96 3° TM dilimleri (EPSG 5253–5259).

## Karar

### 1. Proje türü: CAD ya da CBS, Hibrit yok

- Proje türleri **CAD** (teknik çizim) ve **CBS** (coğrafi bilgi sistemi); 3D Plan ve Afet Analizi “Yakında”, ileride eklenecek türler aynı yerde kart olur. Hibrit kalkar.
- Tür projede kalır (`settings.workspace`) ve yeni projede sihirbazın ilk adımında seçilir. Proje ayarları'ndan değiştirilebilir; yerel bir CAD projesi CBS'ye geçerken koordinat sistemi sorulur (sistem atamak dönüşüm değildir; pencere bunu söyler).
- **Ortak araçlar** iki türde de vardır (TODOS.md §16.0'ın `HYB-*` işleri: kenet, topoloji, ölçme, nokta, oturtma …); her tür onları kendi sahnesinde, kendi düzeniyle ve kendi şeridinde sunar.
- **Sözleşme:** `Workspace` = `cad`, `gis`, `plan3d`, `disaster`. `"hybrid"` artık yazılmaz; okuyucular (KCAD v2, v1 JSON, ayarlar) onu ve modu olmayan eski dosyaları **sorulmamış** sayar (`workspace` yok).
- **Sorulmamış proje** (sahibin kararı): uygulamada açılınca bir kez CAD ve CBS kartlarıyla sorulur; cevap projeye yazılır (çizim kaydedilmemiş olur, geri alma adımı değildir — ADR 0049'daki proje ayarı kuralı). Pencere olmayan yerlerde (başsız sunucu, Python, MCP) ve soru cevaplanana dek sorulmamış proje **CBS** gibi gösterilir; değeri yazılmaz.
- “Yakında” türünü taşıyan dosya CBS gibi gösterilir (bugün Hibrit'ti).

### 2. Yerel CAD ve çizim birimi

- CAD projesi varsayılan olarak **yereldir** (sahibin kararı): koordinat sistemi yok, başlangıç 0,0. Kayıtlarda yerel sistem **SRID 0**'dır: “Yerel (koordinat sistemi yok)”, kayıt defterinde (`fixtures/crs/v1/registry.json`) `kind: "local"`. PostGIS'te SRID 0 “bilinmiyor” demektir; sunucunun `srid` sütunu ve geometri denetimi değişmez.
- İsteyen **gerçek koordinatlı CAD** seçer (ölçme çizimleri): CBS'deki gibi TM/UTM dilimi; CAD'in düzeni (§4) gerçek koordinatlarda da geçerlidir.
- **Çizim birimi** (`settings.drawingUnit`: `mm`, `cm`, `m`; yalnız yerel projede, varsayılan `m`): yazılan ve gösterilen uzunlukların, koordinatların ve alanların birimidir. **Geometri metrede saklanır**; birim kullanıcıyla buluşulan yerde çevrilir (yazma, okuma, `mm²` gibi alan birimi, DXF'in `$INSUNITS`'i). Böylece pafta ölçeği, yazı yükseklikleri, semboller, pafta düzenleri, ölçüler ve kenet aralıkları tek birimle (metre) kalır; gösterim kesinliği değişmez. Koordinat sistemli projede birim sistemin birimidir.
- **Birimin değdiği yerler** (adım 2b): okumalar (durum çubuğu, Öznitelikler, üzerine gelme kartı, Noktalar, komut satırının yankısı ve iletileri, ölçü değerleri) ve yazılanlar (noktalar: `x,y`, `@dx,dy` ve `@mesafe<açı`'nın mesafesi; araçların uzunluk, yarıçap, aralık, tolerans ve kotları; Öznitelikler'in ve Noktalar'ın hücreleri; bir hücrede birimin adı yazılabilir: `250 mm`). Açı, adet, çarpan ve kâğıt üstündeki yazı yüksekliği (mm) birimsizdir. Ortak `fixtures/point-input/v1` durumları birim taşır (`unit`).
- **Ölçme işi metrededir:** Hesap pencereleri (Poligon hesabı, Kutupsal alım, kestirmeler, Aplikasyon, Vektör oturtma, Kenar eşleme) ve İşlemler'in araçları arazi ölçüsüyle çalışır; okumaları ve yazılanları metrededir, alanları `m` yazar (`Formatter.metric`, `Format::metric`).
- **Sistem atanan proje metrededir:** Proje ayarları'nda koordinat sistemi seçilip kaydedilince birim düşer. `.kcad`'de birim şema 11'dir; metre yazılmaz.
- **Yerel projede dosya alışverişi:** DXF iki yönde birimiyle (`$INSUNITS`). Koordinat sistemi taşıyan CBS verisi (GeoJSON, Shapefile, NCZ) yerel projeye koordinatları olduğu gibi gelir ve pencere bunu söyler; CBS biçimlerine yazarken sistem yazılmaz ve uyarılır.
- **DXF ve birim** (adım 2c): yerel projede okuyucu dosyanın `$INSUNITS`'ine bakar. AutoCAD'in 1–24 kodlarından biri (inç, fit, mil, mm, cm, m, km … ABD ölçme birimleri) metreye çevrilir; birim bildirmeyen dosyanın (0, grup yok ya da bilinmeyen kod) değerleri çizimin birimindedir. Koordinat sistemli projede bugünkü gibi metre sayılır, başka birim bildiren dosya söylenir. Çeviri bütün nesnelere tek çarpanla uygulanır: koordinatlar, uzunluklar, yarıçaplar, yazı, ölçü ve kılavuz yükseklikleri, kotlar, tarama aralıkları, blokların tanımları; açılar, oranlar, yerleştirmenin ölçeği ve kalem kalınlıkları değişmez. Çarpan kesin tamsayılarla yazılır (mm: ÷1000, inç: ×254 ÷10000), yazılan değerin metreye çevrilmesiyle aynı yoldan. Yazıcı yerel projenin biriminde yazar: `$INSUNITS` 4, 5 ya da 6; koordinatlar, uzunluklar, çizgi tipi desenleri ve nokta işaretleri birimde; KentOS'un ek verisi (xdata) da. Rapor dosyanın birimini ve çeviriyi söyler.

### 3. Yeni proje sihirbazı

Tek pencerede üç adım; solda adımlar (numara, ad, tamamlananda işaret), altta Geri, İleri ve Oluştur:

1. **Proje türü:** büyük kartlar; her kartta türü anlatan bir resim (CAD: teknik çizim ve ölçüler; CBS: harita, katmanlar ve kuzey), ad, ne olduğu, üç madde. “Yakında” türleri altta soluk kartlar. Son seçilen tür seçili gelir (Uygulama ayarları → Yeni projeler).
2. **Koordinatlar:**
   - **CBS:** **il** seçimi (81 il, aranır) ve **koordinat sistemi**. İl seçilince boylamına göre TUREF/ITRF96 3° TM dilimi önerilir ve listenin başında “Önerilen” olarak durur; liste datumlara göre gruplu (TUREF TM, ED50 TM, UTM, coğrafi, web); Türkiye'nin 27°–45° boylamları bir şerit üstünde dilimleriyle, ilin yeri işaretli gösterilir.
   - **CAD:** **birim** (mm, cm, m kartları) ve **koordinatlar**: “Yerel (0,0)” ya da “Gerçek koordinatlı” (CBS'nin il ve sistem seçimi).
3. **Ölçek ve ayrıntılar:** proje adı; **ölçek** (CBS harita ölçekleri 1:500 … 1:50 000; CAD çizim ölçekleri 1:1 … 1:1000; ikisinde de elle yazılır; yerel CAD'de varsayılan 1:1, çünkü kalınlıklar kâğıt ölçeğinde çizilir: 1:1000'de 0,25 mm'lik kalem çizimde 25 cm olur ve mm'lik parçayı örter); katman şablonu (CBS: bugünkü kadastro paftası ağacı; CAD: teknik çizim katmanları — Çizim, Ölçü, Yazı, Tarama, Yardımcı, Eksen); çizim yazı tipi; seçimlerin özeti.

Başlangıç görünümü:

- **CBS:** seçilen ilin merkezi (enlem, boylamdan TM'ye ileri izdüşümle) seçilen ölçekte; il seçilmezse dilimin çalışma alanı merkezi (bugünkü `workAreaCentre`). İleri izdüşüm ortak çekirdekte (Krüger serisi, pafta çekirdeğinin `tm_inverse`'inin eşi), bağımsız başvuruyla sınanır (CLAUDE.md §14, §23.4). İllerin merkezleri iki platformun okuduğu bir fixture'dadır.
- **CAD:** A3 yatay kâğıt çizim ölçeğinde, 0,0 sol altta (AutoCAD'in yeni çizimi gibi).
- Başlangıç görünümü projenin **ana görünümü** olur (`homeView`): ilk kayıttan sonra da dosya oradan açılır.
- Masaüstünde görünüm, çizim alanı boyut aldıktan sonra kurulur (1:500 000 hatası).

### 4. Eksen ve açı düzeni (sahibin kararı)

| | CAD | CBS |
|---|---|---|
| Eksenler | sağa **X**, yukarı **Y** | sağa **Y** (doğu), yukarı **X** (kuzey) |
| Okuma | “X 120.000  Y 45.500” | “Y 486512.340  X 4420118.900” |
| Yazma | `x,y`, `@dx,dy` | `y,x`, `@dy,dx` |
| Kutupsal | `@mesafe<açı`: açı doğudan saat yönünün tersine | `@mesafe<semt`: semt kuzeyden saat yönünde |
| Açı birimi (varsayılan) | derece | grad |

- İki türde de sayılar aynı sırayla yazılır (önce sağa, sonra yukarı); yalnız eksenlerin adı ve açının başlangıcıyla yönü değişir. Bir koordinatı CAD'den CBS'ye kopyalamak anlamını korur.
- Açı birimi proje ayarıdır; türün varsayılanıyla başlar, değiştirilebilir.
- Okumalar (durum çubuğu, Öznitelikler, üzerine gelme kartı, Koordinat oku, koordinat listesi, Noktalar), istemler ve değer alanının ipucu (“mesafe · X,Y · @dX,dY · @mesafe<açı”) türe uyar.
- `fixtures/point-input/v1` durumları `convention` (`cad`, `gis`) taşır; bugünkü durumlar CAD'dir, CBS'nin semtli kutupsal durumları eklenir; iki okuyucu ikisini de geçer.

### 5. Sahne

- **CAD sahnesi:** sol altta koordinat ekseni simgesi (X ve Y okları; her zaman sol altta: 12 Ekim eki); kuzey oku ve ölçek çubuğu yok; okumalar çizim biriminde; zemin varsayılanı koyu (arduvaz).
- **CBS sahnesi:** kuzey oku (grid kuzeyi) ve ölçek çubuğu (bugünkü gibi); okumalar Y, X ve metrede; zemin temaya uyar.
- `appearance.drawingBackground` yeni varsayılanı **“Türe göre”** (`mode`): CAD'de arduvaz, CBS'de tema. Kullanıcı bir zemin seçerse her türde o kullanılır.
- **Ölçek seçici:** durum çubuğunun ekran ölçeği yazılabilir ve listeden seçilebilir olur (1:N); seçilince görünüm o ölçeğe yakınlaşır (QGIS'in ölçek kutusu gibi). İki türde de vardır.

### 6. Şerit: her türün kendi şeridi (sahibin kararı)

- **CAD:** Dosya, Giriş, Ekle, Açıklama, Değiştir, Görünüm, Yönet, Çıktı (AutoCAD'in “2D Drafting & Annotation” düzenine yakın: Giriş'te çizim, değiştir, katman, özellik ve blok; Ekle'de blok, dosya alma ve başvurular; Açıklama'da yazı, ölçü, kılavuz, tarama, tablo ve işaretleme; Yönet'te temizlik, stiller ve komutlar; Çıktı'da pafta, yazdırma ve dışa aktarma).
- **CBS:** Dosya, Giriş, Harita, Veri, Düzenle, Analiz, Ölçme, Görünüm, Çıktı (ArcGIS Pro, QGIS ve Netcad'e yakın: Harita'da gezinme, koordinat sistemi ve ölçme; Veri'de katman ekleme, içe/dışa aktarma, öznitelik tablosu ve ifadeler; Düzenle'de nesne oluşturma, değiştirme ve topoloji; Analiz'de işlem araçları, modeller ve arazi analizi; Ölçme'de poligon, kutupsal alım, kestirme, aplikasyon ve noktalar; Çıktı'da pafta, lejant ve dışa aktarma).
- Bağlamsal sekmeler (Seçim, Pafta) iki türde de kalır.
- Hiçbir komut kalkmaz: her komut iki türde de komut satırından ve kısayolundan çalışır; şerit türün işini öne çıkarır. Envanter her türün şeridini yazar (`layout.ribbonByMode`); masaüstü projenin türününkini gösterir.

### 7. Adımlar

İşler ADR 0142–0164'teki gibi adım adım, iki platformda, ortak fixture'larla:

1. **Hibrit kalkar:** sözleşme, okuyucular ve yazıcılar, ayarlar, iki platformun modları, pafta profilleri (hibrit profili kalkar), envanter, Python ve MCP tipleri, fixture'lar; sorulmamış projenin sorusu.
2. **Yerel CAD ve çizim birimi:** SRID 0, `drawingUnit`, biçimlendirici, KCAD ve sunucu, dosya alışverişinin kuralları.
3. **Yeni proje sihirbazı:** üç adım, iller ve TM ileri izdüşümü (bağımsız başvuruyla), başlangıç ve ana görünüm, masaüstünde boyut beklenmesi; Proje ayarları ve Uygulama ayarları → Yeni projeler.
   - 3a. Çekirdek ve veri: ileri TM izdüşümü (geometri çekirdeği `geodesy::tm_forward`, Krüger serisi 6. derece; PROJ'un değerleriyle `fixtures/geodesy/v1/tm-forward.json`, `scripts/fixtures/tm_cases.py --check`; web WASM'dan), 81 il (web'in `geo/provinces.ts`'i, iki platformun okuduğu `fixtures/crs/v1/provinces.json`), TUREF dilim önerisi masaüstünde de (kayıt defterinin `zoneSuggestions`'ı), başlangıç görünümü (CBS: ilin merkezi ya da dilimin çalışma alanı, ölçekte bir pafta; CAD: A3 yatay kâğıt, 0,0 sol altta) iki platformda aynı kuralla.
   - CAD'in katman şablonu (web `cadLayers`, masaüstü `cad_layers`) ve ana görünümün projeye yazılması 3a'nın ardından yapıldı; `fixtures/project/v1/new-project.json` ikisini de tutar.
   - 3b. Web'in sihirbazı: kuralları `model/newProjectWizard.ts` (adımlar, taslak, önerilen sistem, türün ölçekleri, adımların notları, özet, ilerlemeyi durduran neden), pencere `ui/settings/NewProjectWizard.ts`, sayfalar `wizardSteps.ts`, dilim şeridi `zoneStrip.ts`, kartların resimleri `ui/settings/art/*.svg`; kuralların yanıtları masaüstü için `fixtures/project/v1/wizard.json`.
   - 3c. Masaüstünün sihirbazı: aynı adımlar, kartlar (aynı SVG resimleri) ve dilim şeridi; kuralları `kentos_project::wizard`, `wizard.json`'u geçer; çerçevesi KentOS UI'ın raylı sihirbazı (`Wizard::rail`).
   - 3d. Uygulama ayarları → Yeni projeler'de tür ve yerel projenin birimi (`newProjects.drawingUnit`); sihirbaz oluşturduğu projenin türünü ve yerelse birimini sonraki sihirbaz için saklar. Masaüstünde görünümün boyut beklemesi 3a'yla yapıldı.
4. **Eksen ve açı düzeni:** okumalar, yazma ve kutupsal giriş (fixture'ıyla), açı varsayılanları.
   - 4a. Çekirdek: yazılan noktanın kutupsal açısı türün düzeniyle ve projenin açı biriminde (CAD: doğudan saat yönünün tersine; CBS: kuzeyden saat yönünde, semt); `fixtures/point-input/v1` durumları `convention` ve `angleUnit` taşır, iki okuyucu ikisini de geçer; yeni CAD projesinin açı birimi derece (`new-project.json`).
   - 4b. Eksen adları: biçimlendiricide türün eksenleri (doğunun ve kuzeyin adı: CAD'de X ve Y, CBS'de Y ve X); durum çubuğu, Öznitelikler, üzerine gelme kartı, Koordinat oku, koordinat listesi, Noktalar, istemler, değer alanının ipucu ve araçların adımları.
   - 4c. Doğrultu okumaları: CAD'de doğrultu açısı (doğudan, saat yönünün tersine) “Açı”, CBS'de “Semt”; Öznitelikler, Mesafe ölç ve koordinat listesi.
   - Hesap pencereleri ölçme işidir: metrede kaldıkları gibi (§2) Y,X ve semtle de kalırlar.
5. **Sahne:** koordinat ekseni simgesi, türe göre zemin, ölçek seçici.
   - Web'de çizim zemini ayarı yoktur: CAD projesinin çizimi her temada arduvazdır (`data-canvas='slate'`); masaüstünde “Türe göre” varsayılandır, kullanıcı başka zemin seçebilir.
   - Ölçek seçicinin listesi türün ölçekleridir (yeni proje sihirbazınınki; CBS'de 1:100.000 da); yakınlaşma bir gezinme adımıdır.
6. **Her türün kendi şeridi.**
   - Web'de türlerin sekmeleri `app/ribbon.ts`'tedir (`CAD_RIBBON_TABS`, `GIS_RIBBON_TABS`, `ribbonSpecs`): şerit yine menülerin bloklarından ve araç kataloğundan kurulur, tür yalnız hangi bloğun hangi sekmede durduğunu söyler. Envanter iki türün şeridini yazar (`layout.ribbonByMode`), masaüstü onu okur; komutların şerit yerleri “Tür › Sekme › Panel”dir.
   - CAD'in Giriş'i AutoCAD'inki gibidir: Çizim sekmesi yoktur; Çizim panelinde gündelik araçlarla Elips, Eğri ve Nokta görünür, öbür çizim araçları panelin ▾'sindedir (panel genişleticisi); Ölçme panelinin ▾'sinde hesap pencereleri; Açıklama ve Blok panelleri (Blok ekle büyük, ana araç). Açıklama sekmesinde her tür bir paneldir (Yazı, Ölçü, Kılavuz, Tarama, İşaretleme), AutoCAD'in Annotate'i gibi; menünün Açıklama bloğuna sonradan giren araç kendi panelinde çıkar. CBS'nin Giriş'inde Harita paneli vardır; CBS'de blok paneli Veri'dedir (çizimin kitaplığı), Düzenle nesne oluşturmaya ve değiştirmeye kalır. Blok ekle iki türde de panelinin başındadır. Panelin ▾'si seyrek araçların yanında türün seçtiği komutları da taşır (`rest`, `under`); türün seçtiği panel adını verdiği seyrek aracı da gösterir (Yönet › Temizlik).
   - Giriş'in panellerindeki “Tüm araçlar” açıcısı aynı türün sekmesine gider (CAD: Değiştir, Açıklama, Ekle; CBS: Düzenle, Harita).
   - Masaüstünün Python konsolu Komut satırına git'in yanındadır: CAD'de Yönet › Komut, CBS'de Analiz › Komut.
   - Hiçbir komut kaybolmaz: bir türün menülerinde gösterdiği her komut o türün şeridindedir (`workspaces.test.ts`), kataloğun her aracı bir türün şeridindedir (duman testi); iki türün her sekmesi 1100 px'e sığar ve 3000 px'te tam boyutundadır (masaüstünde `ribbon_tests`, web'de `e2e:layout`). Web'de katlanmış panelin ▾'si adın son sözcüğüyle birlikte kırılır (“Çizim yardımcıları ▾” iki satır).
   - CAD'in açıklama araçları CBS'nin arayüzünde yoktur (sahibin 6 Ekim sözü: “CAD sadece CAD arayüzünde”): Ölçülendirme ve ailesi
     (Zincir, Baz ve Hızlı ölçü), Tarama, Kılavuz, Çok satırlı yazı, Metin dosyası yerleştir, Revizyon bulutu (`app/workspaces.ts`'in
     gizleme listesi; envanterden masaüstüne). CBS'de Yazı ve Etiketleri yazıya çevir kalır. Gizli komut yazılarak çalışır, CBS
     projesindeki ölçü, kılavuz ve çok satırlı yazı çizilir ve düzenlenir: tür yalnız arayüzü seçer.

### 12 Ekim eki: koordinat ekseni simgesi her zaman sol altta

Sahibin bildirimi (12 Ekim): “cad modunda soldaki xy koordinat okları sayfanın ortasına geliyor, onların sürekli sol altta olması
gerekiyor”. Simge AutoCAD'in UCS simgesi gibi 0,0 ekrandayken orijinde duruyordu; yerel sistemli (SRID 0) CAD projesinde çizim 0,0
çevresinde olduğundan simge çizimin ortasına geliyordu. Artık iki platformda da her zaman çizim alanının sol altındadır (kenarlardan
30 px; web `drawUcsIcon`, masaüstü `map_marks::ucs_at`).

### 12 Ekim eki: ifadeler çizim biriminde

Sahibin bildirimi (12 Ekim): “mm birimler çizim birimi seçiyorum, etiket olarak $alan expression yazıyorum ama alan m2 cinsinden
yazılıyor uzunluk ta benzer şekilde, object inspector doğru ama expression doğru değil”. §2'nin “birimin değdiği yerler” listesinde
ifade dili unutulmuştu: `$alan`, `$uzunluk`, `$y`, `$x`, `$merkez_*`, kutunun değerleri ve başka katmana bakan işlevlerin ölçüleri
her yerde metreydi. Kural:

- **Okumalar projenin biriminde:** etiketin metni ve sınıfının koşulu (Etiketleri yazıya çevir de), bilgi kartı ve eylemler, katman
  süzgeci, Öznitelik tablosunun süzgeci, bunların ε'siyle açılan İfade oluşturucu ve İstatistik ile Grafik'in Alan ve Uzunluk
  sütunları (ADR 0227 §2) uzunluğu, koordinatı ve uzaklığı çizim biriminde, alanı onun karesinde (mm, mm²) okur; Öznitelikler'in
  gösterdiğiyle aynı.
- **Metrik bağlamlar metrede kalır:** İşlemler (araçların sayıları gibi ifadeleri de: “Çizginin parçası” `$uzunluk`'u metre okur),
  ağın maliyetleri ve semboller (boyları metrede). Uzaklığı ifadeden alan bir İşlemler aracı birimle bozulurdu.
- **Motor:** birim derlemenin şemasındadır (`Schema::unit`, web'de bağlamın `unit`'i: metrenin kaç birim olduğu); çözümleyici
  (`resolve`) geometri değerini yüklendiği yerde birimle (alanı karesiyle) çarpar, `uzaklık`, `kesişim_uzunluğu` sonuçlarını
  birimle, `kesişim_alanı`'nı karesiyle çarpar, `kesişir`, `kesişen_sayısı`, `kesişenler`'in verilen uzaklığını birime böler; iç
  ifadeler (`topla($alan)`) de aynı yoldan geçer. Dilde boşun çarpımı boştur: `$alan boş` değişmez. Metre projesinde ağaç
  yazıldığı gibi kalır, sonuçlar bit bit aynıdır. Okuma birimi `kentos_native_application::expression_unit::reading_unit`, web'de
  `readingUnit` (`model/projectSettings.ts`).
- **Yenilenme:** birim değişince iki platformun geometri deposu bütün katmanların etiket metinlerini ve süzgeçlerini yeniden kurar
  (anahtarlarında birim), İstatistik ve Grafik veriyi yeniden okur.
- **Oluşturucu:** `$alan`, `$uzunluk`, `$genişlik`, `$yükseklik`'in açıklaması birimi ve İşlemler kuralını söyler.
- **Doğrulama:** `fixtures/expression/v2/geometry.json`'un `units`'i her nesneyi cm ve mm'de tutar (bağımsız başvuru
  `expression_geometry.py`; metre değeri kesirle birimle çarpılır, bir kez yuvarlanır; tolerans birimde); iki platform okur
  (`typed.rs`, `geometryFixture.test.ts`). Başka katmana bakan işlevler mm ile metre arasında (`language.rs`), depolar uç uca: mm
  projesinde `$alan || ' / ' || $uzunluk` etiketi “3000000 / 7000” yazar, `$alan > 2000000` süzgeci 3 m²'lik alanı geçirir, birim
  metreye dönünce etiket “3 / 7” olur ve süzgeç ikisini de dışarıda bırakır (masaüstü `drawing_unit.rs`, web `picking.test.ts`);
  geometri sütunları üç birimde (`charts.rs`, `charts.test.ts`).

### 13 Ekim eki: katman stilinin değerleri çizim biriminde

Sahibin bildirimi (13 Ekim): “Katman stiline girdim kategori seçeneğini seçtim değer bölümüne $alan girdim değerlerden sınıfa
dediğimde buradaki değerler metrekare geliyor benim proje birimim mm olsa bile, bu da çok yanlış; daha önce ölçü birimi düzeltmesi
yaptık ama burada düzelmemiş”, ardından “orada başka varsa onları da düzeltsin”. 12 Ekim eki okumaları birime çevirirken katmanın
işleyicisini (kategorinin değeri, sınıfın sayısı, kuralın koşulu, tematik işleyicilerin değerleri) hiçbir listeye almamıştı; yalnız
“semboller (boyları metrede)” metrik sayılmıştı. İşleyicinin ifadeleri sembolün değil katmanın ifadesidir ve kullanıcı onları
Öznitelikler'in gösterdiğiyle karşılaştırır. Katman stili penceresi (`classify`, `tally`) ve çizim (`Program::read`) metrede
okuyordu: mm projesinde 3 m²'lik alan için pencere “3” yazıyor, çizim sınıfları 3'e göre çiziyordu; Öznitelikler ve etiket 3000000
mm² diyordu. Kural:

- **İşleyicinin okumaları projenin biriminde:** Kategorili'nin değeri, Aralıklı'nın ve Sürekli renk'in sayısı, Kurallı'nın koşulu
  (alt kurallar dahil), Orantılı sembol'ün ve İki değişkenli renk'in değerleri, Nokta yoğunluğu'nun ve Grafik'in değer listesi, Isı
  haritası'nın ağırlığı ve Kümeleme ile Yayma'nın içindeki tek noktaların işleyicisi. “Değerlerden sınıfla”, “Sınıfla”, “Verilerden
  al” ve Sınıfla'ların hepsi aynı ifadeyi aynı birimde okur, Nesne sütunu (`tally`) onunla sayar; çizim alanı, pafta haritası ve
  PDF aynı ifadeyi aynı birimde derler. Sınıfların sınırları, kategorilerin değerleri ve kuralların sayıları işleyicide birimle
  saklanır: mm projesinde 3 m²'lik alan 3000000'dır, sınıf 1750000 – 3000000 mm²'dir.
- **Metrik kalanlar, nedenleriyle:**
  - *Sembolün kendi ifadeleri* (veriye bağlı boy, kalınlık, açı, uzaklık, renk, metin ve açık/kapalı; Geometri üretici'nin
    adımları): sembol taşınır (Kitaplığım, `.kstil`), girdiği her projede aynı semboldür; ürettiği sayılar sembolün kendi
    biriminde boy ve uzaklıktır (kâğıt mm, piksel, metre) ve `$alan` yanında yazdığı `' m²'` gibi metin onunla birlikte taşınır.
    Birimle okusaydı aynı sembol metre projesinde bir boyda, mm projesinde başka boyda çizerdi. Sembol resimleri de (Stil
    yöneticisi, Sembol tasarımcısı, Lejant) örnek nesne üstünde, metrede.
  - *İşlemler* (araçların sayıları ve ifadeleri; İfadeyle seç ve Öznitelik hesapla dahil): 12 Ekim eki kuralı değişmedi, işlemin
    bütün sayıları (uzaklıklar, eşikler, çıktılar) metrededir ve bir ifade çoğu araçta o sayılardan birini verir. Açık: İfadeyle
    seç'in koşulu ile Öznitelik hesapla'nın değeri ve koşulu uzaklık vermeyen saf okumalardır; sahip isterse
    `ParamKind::Expression`'a “okuma” bayrağı konur, oluşturucunun açıklaması zaten söyler.
  - *Ağ maliyetleri* (süzgeçleri ve maliyetleri): uzunluk ÷ hız gibi fiziksel formüllerdir, metre ve saniyedir.
  - *Pafta tabloları ve koordinat listesi:* kâğıt çıktısıdır, sütun başlığı kendi birimini yazar (“Yüzölçümü (m²)”) ve çekirdek web'le
    ortaktır; sahnenin metresi.
  - *Isı haritasının yarıçapı, Kümeleme'nin ve Yayma'nın uzaklığı, nokta ve grafik boyları:* ifade değil sayıdır, kendi birimini
    seçer (m, px, kâğıt mm); Hover kartındaki tapu alanı (“Tapu alanı (m²)” özniteliği) hukuki değerdir.
- **Motor:** `kentos_style_core`'da programın ifadeleri iki yoldan derlenir: `Exprs::intern` (sembol, metre) ve
  `Exprs::intern_reading` (işleyici, programın okuma birimi); `Program::read_in(metin, birim)`, `Program::read` onun metredeki
  hâlidir. Aynı metin iki yerde geçse bile (işleyicinin `$uzunluk`'u ve sembolün genişliği) ayrı derlenir; metrede ikisi tek
  derlemedir. Masaüstünde `BuildOptions::reading_unit`, `Look::reading_unit` (çizim alanı `viewport.rs`, pafta haritası,
  `kentos-plot`'un PDF'i ve başsız sunucu), pencerede `ExprScope::unit` (`Source::scope` projenin ayarından alır); sınıf ve kural
  sayımı (`tally`) onunla derler. Birim `Look`'un parçasıdır: değişince sahne baştan kurulur (`StyledCache`), pafta ve PDF zaten
  belge sürümüyle yeniden kurulur.
- **Metre projesi:** hiçbir şey değişmez; birim 1 iken derleme ağacı yazıldığı gibi bırakır (12 Ekim eki), sonuçlar bit bit aynıdır.
- **Bilinen sınır:** projenin birimi değişince işleyicide saklanan sayılar ve kural metinleri çevrilmez (etiket ve süzgeç
  metinleri de çevrilmez): kullanıcı yeni birimle “Sınıfla”yı yeniden çalıştırır. Web'e ara verildiği için (ADR 0228) web'in
  Katman stili'nde, çiziminde ve paftasında bu kural yok; web'de açılan mm projesinin işleyicisi eski gibi metrede okunur.
  TODOS.md §0.2.
- **Doğrulama:** bağımsız başvuru `scripts/fixtures/style_units_cases.py` (kesirlerle; geometri eksene paralel olduğu için
  uzunluk ve alan tam) `fixtures/style/v1/units.json`'u yazar: üç birimde Değerlerden sınıfla'nın değerleri, farklı değerleri ve
  sayıları, iki ve dört sınıf, kategori, sınıf ve kural sayıları; masaüstü bunu `crates/native/style/tests/all/units.rs`'te
  geçer (web'in kaydettiği `classify.json`'a dokunulmadı: onu web'in kaydedicisi yeniden yazar). Aynı dosyada çizilen katman: her
  işleyici metrede ve birimde yazıldığında aynı toplulukları verir, birimin sayılarıyla başka birimde okununca vermez (Kategorili,
  Aralıklı, Kurallı, Sürekli renk, Orantılı sembol, İki değişkenli renk, Nokta yoğunluğu, Grafik, Isı haritası, Kümeleme ve
  Yayma'nın tek noktaları); sembolün `$uzunluk` genişliği ve Geometri üretici'nin `$uzunluk / 20` tampon uzaklığı her birimde aynı
  kalır; metre projesi `read` ile aynı. Uçtan uca pencere ve çizim alanı
  (`apps/desktop/src/style/layer_style/unit_tests.rs`: Değerlerden sınıfla, Sınıfla, Uygula, çizilen dolgular, birim
  değişince yeniden çizim, Kurallı, Sürekli renk'in Verilerden al'ı) ve pafta haritası (`crates/native/plot/tests/all/units.rs`).
  Her yerin bağlantısı tek tek bozularak sınandı (birimi yok sayan program, metrik ağırlık, birimli sembol, metrik sayım, metrik
  pencere, metrik çizim alanı ve harita): her seferinde ilgili testler düştü. Resimler: `tools_screens`'in `cad-mm-stil-kategorili`,
  `-aralikli`, `-kurallar`, `-cizim` sahneleri (`.run/shots/arac-cad-mm-stil-*`).

## Bu ADR'de olmayanlar

- CBS sahnesinde altlık harita karoları (dış XYZ ya da yayın kararı; CLAUDE.md §17).
- Coğrafi koordinat (enlem, boylam) okuma ve yazma; datum dönüşümü (ED50 ↔ TUREF).
- 3D Plan ve Afet Analizi türleri.

## Doğrulama

Her adımın testleri, ortak fixture'ları ve iki platformun resimleri (1440 × 900 ve 1100 × 650, açık ve koyu) adımın commit'indedir; envanter aynı commit'te güncellenir.
