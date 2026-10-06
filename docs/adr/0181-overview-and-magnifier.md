# ADR 0181: Genel bakış ve Büyüteç

- **Durum:** kabul edildi (2026-10-06); tamam (2026-10-06, iki platformda, tek parçada). Sıra sahibin kararıdır: TODOS.md §16.0'ın yirmi beşinci işi `HYB-25` (`HYB-24` canlı GNSS,
  sahibin 5 Ekim kararıyla, alıcı bulunana dek atlanır); sahibin 5 Ekim kararıyla madde tek parçada biter. Ayrıntılar bu ADR'nin
  varsayılanlarıdır. QGIS'in Overview paneli, ArcGIS'in Overview ve Magnifier pencereleri, AutoCAD'in eski Aerial View'u
  (DSVIEWER) örnektir; Netcad'de karşılığı yoktur.
- **Bağlam belgesi:** TODOS.md `HYB-25`, `CAD-05` (ikinci pencere ve yer imleri ayrı iştir); CLAUDE.md §4.4 (yerleşim kullanıcının),
  §4.9 (çizim hattı: kaynak koordinat f64, GPU'ya yerel fark), §6.2 (sıcak yolda bütün belge gezisi yok); DESIGN.md (kart, gölge,
  vurgu); ADR 0055 (çizimin yazıları kaplama), ADR 0120 (masaüstünün tutulan resmi), ADR 0157 (stilli çizimin karoları).

## Bağlam

Büyük bir çizimde yakın çalışırken nerede olunduğu kaybolur: kullanıcı uzaklaştırıp yeniden yakınlaştırır. Hassas çizimde de
imlecin altındaki küçük ayrıntı (iki yakın köşe, kenet adayı) görünümü değiştirmeden görülmek istenir. KentOS'ta ikisi de yoktur.

## Karar

### 1. Genel bakış

- **Genel bakış** (`view.overview`, aç ya da kapat; Görünüm sekmesi, Görünüm menüsünün Paneller bölümü) çizim alanının sol üst
  köşesinde bir karttır: başlık satırı (“Genel bakış” ve kapatma düğmesi) ve altında 240 × 160 piksellik resim. Açık ya da kapalı
  oluşu yerleşimdir (`overview`; web `kentos.ui.v1`, masaüstü `yerlesim.json`, ortak kuralları `fixtures/shell/v1/layout.json`),
  çizime yazılmaz.
- Resim çizimin bütününü gösterir: görünen katmanlardaki nesnelerin kapsamı (§3), kenarlarında 6 piksel pay. Nesneler katmanlarının
  renginde ince çizgilerle, alanlar soluk dolguyla çizilir; resimde bir pikselden küçük kalan nesne bir noktadır. Yazı, ölçü ve
  kılavuz bir noktadır; yardımcı çizgi ve ışın çizilmez ve kapsama girmez.
- Resim çekirdekte, deponun nesnelerinden çizilir (§3); çizim, katmanların görünürlüğü ya da rengi, tema ya da kartın boyu değişince
  250 ms sonra yeniden çizilir, görünüm değişince çizilmez. Görünüm çerçevesi resmin üstünde vurgu renginde dikdörtgendir (soluk
  dolgulu); 6 pikselden küçükse yerinde bir artı.
- Fare: resme basmak görünümün merkezini oraya getirir (ölçek değişmez), basılı sürüklemek görünümü sürükler; tekerlek görünümü
  merkezinde yakınlaştırır ya da uzaklaştırır (çizim alanınınki gibi); çift tık **Tümünü göster**dir.
- Çizimde gösterilecek nesne yoksa kartta “Çizimde gösterilecek nesne yok.” yazar.

### 2. Büyüteç

- **Büyüteç** (`view.magnifier`, aç ya da kapat) imlecin altını büyüten 220 × 220 piksellik karttır: başlık satırında “Büyüteç”,
  büyütme düğmeleri 2×, 4×, 8×, 16× (varsayılan 4×) ve kapatma düğmesi. Açık oluşu ve büyütmesi yerleşimdir (`magnifier`,
  `magnifierZoom`; saklanan büyütme 2 ile 16 arasına getirilir, gösterilen en yakın adımdır).
- Kartın içi çizimin kendisidir, imlecin çizimdeki yeri ortada, görünümün ölçeğinin büyütme katı yakınlıkta: çizim hattının aynı
  katmanları (görünen katmanlar, ızgara, seçim ve üzerine gelme vurgusu) bir ikinci kamerayla, çizimin yazıları ve işaretler (kenet
  işareti, tutamaçlar, aracın önizlemesi) bu kamerayla; kuzey oku, ölçek çubuğu ve eksenler büyüteçte çizilmez. Semboller görünümün
  seçimiyle çizilir (ölçek aralığı, okunurluk, atlas resimleri): büyüteç çizimi o anki hâliyle büyütür, ikinci bir çizim kurmaz.
  Ortada küçük bir artı imlecin yeridir: ortadan 3 piksel boşlukla 9 piksellik kollar, çizimin mürekkep renginde.
- İmleç çizimden çıkınca son yer kalır.
- Kart çizim alanının sağ üst köşesindedir (CBS'de kuzey okunun altında). İmleç kartın 16 piksel yakınına gelince kart öbür yana
  (sol üst; Genel bakış açıksa onun altına, sığmazsa yanına) geçer, oradayken yaklaşılınca geri döner (§4).
- Büyüteç yalnız gösterir: imleç kartın üstünde değilken çizim alanının olaylarını değiştirmez.

### 3. Genel bakışın resmi (çekirdek)

- **Kapsam:** görünen katmanlardaki, yardımcı çizgi ve ışın dışındaki nesnelerin sınır kutularının birleşimi. Kapsamın genişliği ya
  da yüksekliği 1 m'den azsa merkezinin çevresinde 1 m'ye büyütülür.
- **Ölçek ve yer:** içerik W × H piksel, pay P = 6: k = min((W − 2P) / genişlik, (H − 2P) / yükseklik) piksel/metre; kapsamın merkezi
  (cx, cy) resmin ortasındadır: u = W/2 + (x − cx)·k, v = H/2 − (y − cy)·k; tersi x = cx + (u − W/2)/k, y = cy − (v − H/2)/k.
- **Resim:** Wp × Hp aygıt pikseli (Wp = round(W·dpr)), kp = k·dpr; noktanın pikseli (⌊Wp/2 + (x − cx)·kp⌋, ⌊Hp/2 − (y − cy)·kp⌋).
  Pikseller saydam başlar; nesneler belgenin sırasıyla:
  - sınır kutusu resimde 2 pikselden küçükse (iki yanı da) ya da yazı, ölçü, kılavuzsa: kutunun merkezinin pikselinde d × d nokta
    (d = max(1, round(dpr))), katmanın rengi, tam örtük;
  - alanlar (kapalı alan, daire, elips, tarama; parçaları ve delikleriyle): çift-tek kuralıyla dolgu, piksel merkezlerinde (satır j'de
    y = j + 0,5; kesişen kenarın x'i ax + (y − ay)·(bx − ax)/(by − ay); [⌈x₀ − 0,5⌉, ⌈x₁ − 0,5⌉) aralığı), katmanın rengi 64 örtüklükle,
    yalnız tam örtük olmayan piksellere;
  - sonra çizgileri (alanların halkaları, çizgi, çoklu çizgi, yay, eğri; blok yerleştirmesinin parçaları): her kenar iki ucunun
    pikselleri arasında Bresenham'la, tam örtük. Daire ve yay n = max(4, ⌈|açıklık|·r·kp / 4⌉) (en çok 720) eşit parçalı, elips ve eğri
    çekirdeğin dış çizgisiyle (0,5 piksel sapma);
  - nokta nesnesi d × d nokta.
- Katmanların rengini çağıran verir (temanın mürekkebiyle çözülmüş); nesnenin kendi rengi kullanılmaz.

### 4. Kartların yeri (iki platformda ortak)

Çizim alanı W × H mantıksal piksel; kenar payı 8, kartlar arası 8; kart 1 piksellik çerçeve ve başlık satırı (yüksekliği h, çağıranın;
varsayılan 24) ile içeriğinden oluşur:

- Genel bakış: (8, 8), boyu 242 × (h + 162).
- Büyüteç sağda: x = W − 8 − 222, y = 8 (CBS'de kuzey oku için 60); boyu 222 × (h + 222).
- Büyüteç solda: x = 8, y = 8 ya da Genel bakış açıksa onun altı (8 + h + 162 + 8); orası alana sığmıyorsa Genel bakışın yanı
  (x = 8 + 242 + 8, y = 8).
- Yan değiştirme: imleç kartın 16 piksel genişletilmiş dikdörtgenindeyse kart öbür yana geçer; öbür yan da imlece o kadar yakınsa
  yerinde kalır.

### 5. Çizim hatları

- Web (WebGL2 ve WebGPU): kare çiziminin sonunda büyütecin dikdörtgeni makasla ayrılır, temizlenir ve kalıcı katmanlar, ızgara ve
  kaplamalar ikinci kameranın tek biçimli değerleriyle (WebGPU'da ikinci çerçeve tamponu ve bağlama grubu) yeniden çizilir; görünüm
  kararları (görünürlük, atlas) ana kameranındır, yeniden hesaplanmaz. 2B kaplama (yazılar, işaretler) ana kamerayla çizilirken
  büyütecin dikdörtgeni dışarıda kalır; büyütecin kendi yazıları ve işaretleri ikinci kamerayla o dikdörtgene çizilir.
- Masaüstü (wgpu): büyüteç kartının içi ayrı bir gölgelendirici öğesidir; çizim alanınınkiyle aynı ilkel türüdür (Iced çiziciyi
  ilkel türüne göre tutar: aynı türden ilkel aynı `Renderer`'ı kullanır), çizim alanının GPU'daki parçalarını ve stilli katmanlarını
  kendi çerçeve tamponuyla, yalnız penceresine düşenleri çizer (`Renderer::prepare_lens`, `draw_lens`; stilli katmanlarda
  `StyledGpu::prepare_lens`, `draw_lens`); yazılar ve işaretler kartın üstünde aynı katmanlarla (`labels::lens_layer`,
  `preview::layer`) ikinci kamerayla.
- Genel bakışın resmi web'de kartın kendi tuvalinde (`putImageData`), masaüstünde tuvalde aynı renkli piksel dizileri dikdörtgen
  olarak (resim değişene dek önbellekte; Iced'in resim özelliği açılmadı, yeni bağımlılık yok); görünüm çerçevesi üstünde çizilir.

### 6. Komutlar ve iz

- `view.overview` (Genel bakış; takma adlar GENELBAKIS, KUSBAKISI, DSVIEWER, OVERVIEW) ve `view.magnifier` (Büyüteç; BUYUTEC,
  MAGNIFIER), Görünüm menüsünün Paneller bölümünde ve şeritte; işaretli düğme.
- Ortak izin eylemleri: `overview` (Genel bakışın resminde çizimin bu noktasına basmak; oynatıcı noktayı §3'ün kuralıyla karta
  çevirir) ve `magnifier` (büyütme düğmesine yazısıyla basmak: `"8×"`); beklentileri `overview` (kapalıysa `null`; açıksa kapsam
  `extent`, `view.center`'a göre) ve `magnifier` (kapalıysa `null`; açıksa `zoom`, `side` ve `center`).

## Kapsam dışı

- İkinci çizim penceresi, adlı görünümler (yer imleri): `CAD-05`.
- Genel bakışta katman seçimi (QGIS'in “Show in Overview”'u): bütün görünen katmanlar çizilir.
- Büyüteçten tıklayarak çizim (ArcGIS'te de yoktur); büyüteci sürükleyerek taşımak.

## Uygulama

Tek parçada: çekirdekte Genel bakışın kapsamı ve resmi (`store::overview`: `Store::overview_extent`, `Store::overview_picture`;
web'e `GeometryStore`'un `overviewExtent` ve `overviewPicture` bağlayıcılarıyla) ve kartların yeri ile Genel bakışın eşlemesi
(`tools::navigation`; web'de `viewport/navigation.ts`), ortak durumlar `fixtures/navigation/v1/cases.json` bağımsız başvurudan
(`scripts/fixtures/navigation_cases.py`; resimler kesin piksel kurallarıyla, iki platform piksel piksel); web'in iki çizim hattında
(`FrameState.lens`, WebGL2'de makas, WebGPU'da ikinci çerçeve tamponu) ve masaüstünün wgpu hattında büyüteç; kartlar (web
`viewport/navigationCards.ts`, masaüstü `navigation_cards.rs`), komutlar, ikonlar, şerit ve menü; yerleşim alanları
(`fixtures/shell/v1/layout.json`); ortak iz (`fixtures/interaction/v1/overview.json`, `overview.kcad`; oynatıcılarda `overview` ve
`magnifier` eylemleri ve beklentileri); envanter.

Tamam (6 Ekim): iki platform ortak durumları ve ortak izi geçer; resimler `.run/shots/kullanim/*-overview-*`, web'inki WebGPU'yla
da denendi.

## Doğrulama

- Kartların yeri, eşleme ve resim ortak durumlarla, bağımsız başvurudan; iki platform aynı dosyayı geçer.
- Ortak iz: aç, kapat, Genel bakışta basma, büyütme düğmeleri, kartın yan değiştirmesi; resimler iki temada, 1440 × 900 ve
  1100 × 650; web'de WebGPU'yla da.
