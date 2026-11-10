# 4700 — Ortofoto mozaiğinin birleşme çizgisinin ve mozaik elemanlarının KentOS'ta kurulması

Tarih: 2026-11-10. plan/map-cad.md MAP-12, dokuzuncu tur. Durum: taslak. ADR 4460'ın “Sonra”sındaki “KentOS'un kendi
dikiş çokgenlerini kurması (Voronoi, radyometri; Build Seamlines'ın yöntemleri)”.

İlgili:
- [ADR 4460](4460-orthophoto-seam-along-seamline-polygons.md): kayıklığın iki görüntünün mozaik elemanlarının ortak
  sınırı boyunca ölçülmesi; bu ADR'nin kurduğu elemanlar onun `seam.polygons`'ına doğrudan verilir;
- [ADR 1780](1780-two-orthophotos-compared.md), [ADR 3660](3660-orthophoto-seam-resampled-onto-the-first-grid.md):
  iki ortofotonun karşılaştırılması, ikincinin Keys kübik evrişimiyle birincinin ızgarasına örneklenmesi;
- [ADR 0517](0517-raster-mosaic-vrt.md): Sanal mozaik (VRT; örtüşmede sıra, piksel ya da tarih).

## Terimler

Sahibin düzeltmesiyle (10 Kasım 2026) kavramlar disiplinin resmî Türkçesiyle yazılır; kaynak TUCBS Ortogörüntü Veri
Tanımlama Dokümanı (Sürüm 1.0, RG 09.07.2020/31180 Mük.) ve BÖHHBÜY m. 67(6):

| Kavram | KentOS'ta | Kaynak |
| --- | --- | --- |
| seamline | **birleşme çizgisi** | TUCBS tanım 5: mozaiklemede görüntülerin faydalı alan sınırını çizen çizgi |
| seamline polygon (bir görüntünün mozaikte kullanılan bölümü) | **mozaik elemanı** | TUCBS MozaikEleman / TekliMozaikEleman |
| footprint | **ayak izi** (TUCBS'de “ayakizi”) | TUCBS Şekil 8; “kapsama alanı” sınırlayıcı dikdörtgendir, ayak izi değildir |
| radiometric difference | **radyometrik fark** | TUCBS: “radyometrik görüntü farkları” |
| iki görüntü arasındaki uyumsuzluk | **kayıklık** | BÖHHBÜY m. 67(6) |

Bu dilimle Ortofoto karşılaştırması'nın sözleri de değişti: “Dikiş ölçümü” → **Kayıklık ölçümü**, “Dikiş hattı” →
**Birleşme çizgisi**, “Seçili çokgenleri al” → **Seçili mozaik elemanlarını al**, “Hattı kaldır” → **Çizgiyi kaldır**;
komutların notları ve retleri de (cad.orthophoto.compare.json'un 33 satırı). Eski ADR'lerin (2460–4460) “dikiş”i bu
tablodaki karşılığıyla okunur.

## Dayanak

- **ArcGIS Pro, Build Seamlines**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/build-seamlines.html), 10 Kasım 2026'da
  okundu): her ayak izine bir birleşme çizgisi çokgeni; hesap yöntemleri GEOMETRY (ilk değer; ayak izlerinin
  kesişiminden), RADIOMETRY (örtüşmedeki piksel değerleri ve desenlerinden iki kesişme noktası arasında bir yol), EDGE
  DETECTION, VORONOI (“alan Voronoi diyagramı”), DISPARITY, COPY_FOOTPRINT; radyometriden önce renk düzeltmesi önerilir.
  Algoritmanın ayrıntısı belgede yoktur.
- **ArcGIS Pro, mozaik veri kümesinin birleşme çizgileri** ([kavram sayfası](https://doc.esri.com/en/arcgis-pro/latest/help/data/imagery/mosaic-dataset-seamlines-pro-.html)):
  elemanlar ayak izlerinin birleşimini boşluksuz ve örtüşmesiz döşer; üçlü düğümlerde birleşir.
- **Pan, Wang, Li, Li 2009** (IEEE TGRS 47(6) 1737–1744, “Automatic generation of seamline network using area Voronoi
  diagrams with overlap”): A_i ile A_j örtüşüyorsa A'_i, A_i'nin A_j dışındaki parçasıdır; örtüşmenin bir noktası A'_i'ye
  ve A'_j'ye eşit uzaksa birleşme çizgisindedir. Dörtgen örtüşmede (iki kesişme noktası A ve C) dört iç açının
  açıortayları: A'nın açıortayının en yakın kesiştiği E, C'ninkinin F; **AEFC** çizgidir. İnceltmede pikselin farkı
  bantların mutlak farklarının en büyüğü; çizgi iki uç arasında yol boyunca en büyük farkı en küçük yapan yol.
- **Chon, Kim, Lin 2010** (ISPRS J. 65, 86–92): önce en büyük yerel farkın en küçüğü (minimax), sonra o eşiğin altındaki
  piksellerde Dijkstra ile toplam maliyet.
- **Mevzuat** (alt ajanın notu `.run/research/map-cad/mevzuat/MAP-12.9.md`, okunan metinler):
  - BÖHHBÜY (RG 26.06.2018/30460 Mük.; 4.10.2025 değişikliği m. 67'ye dokunmadı) m. 67(6): mozaiklemede ortak alanda
    radyometrik düzeltme yapılmalı, kayıklık en çok ±2·YÖA. **Birleşme çizgisinin yeri ve yöntemi için resmî Türk kuralı
    yok;**
  - TUCBS Ortogörüntü: mozaik elemanları örtüşmez, birleşimleri ayak izini kapsar, her biri görüntü kaynağını taşır;
    birleşme çizgisi genellikle radyometrik farkın en az olduğu yerden ya da doğal sınırlardan geçer (tanım, ölçülebilir
    kural değil);
  - USGS Digital Orthoimagery Base Specification (2014) ve ASPRS Ed. 2 (2024) Ek III M.4: birleşme çizgisi çatıyı, kuleyi
    kesmez; uluslararası, Türkiye'de uygunluk diye gösterilmez.
- **Benzer uygulamalar** (alt ajanın notları `.run/research/map-cad/uygulama/MAP-12.9.md`, `R12-MAP-12.md`): Metashape'in
  Draw Polygon › Assign Images'ı ve çizgiyi yüksek nesnelerden uzaklaştıran “Refine seamlines”; Inpho OrthoVista'nın ve
  HxMap'in dışlama çokgenleri; Pix4D'nin bölgeye tek görüntü atayan Mosaic Editor'ü; Netcad'in Mozaik penceresi yalnız
  örtüşmenin ortalamasını alır, birleşme çizgisi kurmaz.

## Karar

1. **`cad.orthophoto.seamlines` v1** (yeni komut, belgeyi değiştirir, tek geri alma adımı “Mozaik elemanları”): iki
   ortofoto (`first`, `second`), ayak izleri (`footprints`: alan ya da düz kenarlı kapalı çoklu çizgi, dışbükey, deliksiz
   ve tek parçalı; verilmezse rasterlerin dört köşesi), yöntem (`voronoi`, eşit uzaklık, ilk değer; `radiometry`,
   radyometrik fark).
2. **Örtüşme ve uçlar** (`kentos_geometry_core::ops::seamlines`, ortak çekirdek): birinci ikincinin yarı düzlemleriyle
   kırpılır (doğrunun üstü içeride sayılır), en güneydeki (eşitse en batıdaki) köşesinden başlar. Örtüşmenin her kenarı
   yalnız ikincinin sınırındaysa birincinin ucu (ikinci görüntü orada, birincinin içinde biter), yalnız birincininkindeyse
   ikincinin ucu, ikisininkindeyse ortak sınırdır. Her uç tek bir koşu olmalı; değilse ret (ayak izleri artı gibi
   kesişiyor). Saat yönünün tersine giderken birincinin ucundan sonra ikincininkine ya bir köşede varılır (iki sınırın
   kesiştiği yer: `crossing`) ya da ortak sınırdan geçilerek; ortak sınırdaki uç, birincinin yanından ilk, iki görüntünün
   kendi bölgelerinden eşit uzak noktadır (`shared`; ikiye bölmeyle). Çizgi birincinin ucundan sonrakinden (başlangıç)
   ikincininkinden sonrakine (bitiş) gider; birincinin ucu solundadır.
3. **Eşit uzaklık (Voronoi)** (Pan ve ark. 2009): örtüşmenin iki görüntünün kendi bölgelerinden eşit uzak noktaları.
   Kirişin (başlangıç–bitiş) `step` metre aralıklı (ilk değer 1 m; en az iki aralık) dik doğrularında kök ikiye bölmeyle
   bulunur; her kökte her ucun en yakın öğesi bir kenar ya da bir köşedir. İki kenar, iki köşe ya da öbürünün kenar
   doğrusundaki köşe: kesim düzdür (açıortay, orta dikme), yalnız uçları yazılır; dönüm yeri sonraki kesimin doğrusuyla
   kesişimdir, aradaki kesim kaçmışsa kirişte ikiye bölünerek bulunur. Köşe ile kenar arası parabol: kökleri yazılır.
   Dörtgen örtüşmede sonuç makalenin AEFC'sidir; aralık düz kesimleri değiştirmez.
4. **Radyometrik fark** (Chon ve ark. 2010): ızgara birinci görüntünün `cell` × `cell` pikselidir (ilk değer 1; 1..64).
   Piksel merkezi örtüşmede (sınırı dahil) ve ikinci orada okunabiliyorsa okunur: merkezler çakışıyorsa (10⁻⁶ piksel)
   ikincinin kendi pikseli, değilse Keys kübik evrişimi (a = −1/2; 16 pikseli ikincide olmalı). Pikselin farkı ortak
   bantların mutlak farklarının en büyüğü; hücreninki piksellerininkilerin en büyüğü. Hücre bütün pikselleri okunur,
   merkezi eşit uzaklık çizgisine `corridor` (arama bandı) metreden yakın (verilirse) ve hiçbir piksel merkezi `avoid`
   katmanının alanlarında (geçilmeyecek alanlar: bina, köprü; USGS'nin “çatı kesilmez”i) değilse aranır. Eşit uzaklık
   çizgisinin başlangıcına ve bitişine en yakın aranan hücreler (kare uzaklıkla; eşitse satır sırasında ilki) arasında:
   önce en büyük farkı en küçük yol, sonra onu aşmayan hücrelerde en az maliyetli yol (hareket boyu 1 ya da √2 × (1 + iki
   hücrenin farklarının ortalaması); 8 komşu; kuyruk (anahtar, dizin) sırasıyla, hücre yalnız kesin düşüşte değişir).
   Çizgi: başlangıç, yolun dönen hücrelerinin merkezleri, bitiş. Cevapta `difference`: yolun en büyük farkı, yolun ve
   aranan hücrelerin ortalaması, sayılar, hücrenin yerdeki boyu. En çok 25 milyon piksel okunur.
5. **Mozaik elemanları:** birincininki kendi sınırında bitişten başlangıca (saat yönünün tersine, ikincinin ucunu
   içermeyen yan), sonra birleşme çizgisi; ikincininki başlangıçtan bitişe, sonra çizginin tersi. Çizginin noktalarını
   aynen paylaşırlar, örtüşmezler, iki ayak izinin birleşimini kaplarlar; ikisi de basit olmalı (değilse ret). Cevapta her
   elemanın halkası, alanı, örtüşmeden kullandığı (`kept`) ve yazılınca kimliği.
6. **Yazma:** iki alan `layer` katmanına; verilmezse `mozaik-elemanlari` (“Mozaik elemanları”), çizimde yoksa aynı
   adımda açılır (`opened`). Öznitelikler `Görüntü` (rasterin kalıcı kimliği; TUCBS'nin görüntü kaynağı) ve `Yöntem`.
   Kilitli katman reddedilir. `expectedRevision` eskiyse çatışma, hiçbir şey yazılmaz.
7. **Retler:** `invalid_uid`, `entity_not_found`, `not_a_raster`, `crs_mismatch` (`first`, `second`); `invalid_input`
   (aynı raster iki kez; `step`, `cell`, `corridor`; eşit uzaklıkta `cell`, `corridor`, `avoid`; ayak izi iki kez,
   çizimde yok, alan değil, yaylı, delikli ya da parçalı, dışbükey değil; örtüşme yok, biri öbürünün içinde, sınır ikiden
   çok el değiştiriyor; aranacak hücre yok, çok piksel, uçlar arasında yol yok; kendini kesen eleman); `layer_not_found`
   (`layer`, `avoid`), `layer_locked`; `invalid_revision`, `revision_conflict`; görüntünün dosyası.
8. **Masaüstü:** Raster sekmesinde Karşılaştır'ın yanında **Birleşme çizgisi** (pencere “Mozaik birleşme çizgisi”; yeni
   ikon `orthoSeamlines`, seçenek kâğıdı `.run/gui-evidence/ikon-birlesme-cizgisi/`). Form: Ortofotolar (aralarında Yer
   değiştir), Ayak izleri (Raster sınırları | Seçili alanlar, Seçili alanları al), Yöntem (Eşit uzaklık (Voronoi) |
   Radyometrik fark), yönteme göre Eğri kesimde aralık ya da Hücre ve arama bandı ile Geçilmeyecek alanlar. Plan her
   değişiklikte koşar; özetin altında yazma kendi bölümündedir (Yazılacak katman, **Mozaik elemanlarını çiz**); Plan (iki
   eleman, kesikli örtüşme, kırmızı geçilmeyecek alanlar, çizgi) ya da Noktalar tablosu. Çizince iki eleman seçilir:
   Ortofoto karşılaştırması › Seçili mozaik elemanlarını al onları ölçer.
9. **Kapsam işareti:** **CAD + CBS** (raster bağlam sekmesi), ADR 1780'deki gibi.

## Sınırlar

- İki görüntü: üç ve daha çok görüntünün birleşme çizgisi ağı (üçlü düğümler, Pan ve ark.'nın çoklu örtüşmesi) yok.
- Kenar algılama, paralaks ve harmanlama genişliği yok; radyometrik fark yöntemi renk dengelemesi yapmaz (BÖHHBÜY m.
  67(6) onu ister; not söyler).
- Ayak izi dışbükey olmalı; NoData'dan ayak izi çıkarılmaz (kullanıcı alanı verir). TUCBS'nin “faydalı ayakizi” görüntünün mozaikte kullanılan bölümüdür, NoData'dan çıkan ayak izi değil.
- Kurulan çizgi BÖHHBÜY'e “uygun” denmez: yönetmelik birleşme çizgisinin yerine kural koymaz; kayıklık ADR 1780/4460
  ile ayrıca ölçülür.
- Görüntüden türeyen nesnenin kaynağa izi MAP-01'in sahip kararını bekler; mozaik elemanı yalnız `Görüntü` özniteliğini
  taşır, türetme kaydı yazmaz.

## Kanıt

- **Çekirdek** (`kentos_geometry_core::ops::seamlines` testleri, 5): yan yana iki şerit (çizgi x = 515, uçlar ortak
  kenarlarda, elemanlar 180 m²); kaydırılmış şeritler, aralık 0,25 / 1 / 5 / 50 m'de aynı AEFC ve her noktada iki uca eşit
  uzaklık; retler (ayrık, değen, içinde, aynı, artı, içbükey, iki noktalı); saat yönünde, kapanış noktalı, düz kenarda
  köşeli ayak izi; ızgara yolunun duvar aralığından en büyük farkla geçmesi.
- **Bağımsız başvuru** (`scripts/fixtures/orthophoto_seamlines_cases.py`; KentOS kodu yok), 44 durum: örtüşme kesin
  kırpılır, eşit uzaklık çizgisinin kesimleri yoğun taramayla bulunur, dönüm yerleri kirişte ikiye bölmeyle 40 basamakta
  (KentOS doğruların kesişimini kullanır); radyometrik fark orthophoto_compare_cases.py'nin sentetik görüntülerinin
  piksel değerleriyle, kübik okuma o dosyanın Keys çekirdeğiyle. Yan yana, AEFC, aralık 50, karışık uçlar, parabollü iki
  durum, 9° dönük ikinci, saat yönünde kapalı çoklu çizgi; radyometrik fark (hücre 1 ve 2, arama bandı, bina, 8 cm'lik
  ikinci); iki yazma ve geri alma (yeni katman, verilen katman); 25 ret ve çatışma. Rust ve Python geçer; yetenek testi
  geçer. Bozma denemesi: AEFC'nin bir noktası 1 µm ve bir en büyük fark 1 oynatılınca tam o iki durum yakalanır.
- **Python** (`python/tests/test_orthophoto_seamlines.py`, testin kendi okumalarıyla): AEFC'nin her noktası iki uca eşit
  uzak; iki elemanın alanı birleşiminki; yazılan elemanlar Ortofoto karşılaştırması'nın ölçtüğü elemanlar (ortak sınır
  çizginin kendisi); bina çizginin içinde değil (milimetrede bir örnek); yolun en büyük farkı eşit uzaklık çizgisinin
  piksel sütununun en kötü farkından büyük değil; artı gibi kesişenler reddedilir.
- **MCP** (`crates/native/mcp/tests/orthophoto_seamlines.rs`): beş durum (yazılınca kimlikleriyle); kayıklık ölçümünün
  iki MCP testi yeni sözlerle geçer.
- **Masaüstü** (`ortho_seamlines_window_tests`): plan durumunkiler; seçili iki alan ayak izi; radyometrik fark ve bina;
  çizince iki eleman ve katman, seçim, Ortofoto karşılaştırması'nın ortak sınırı çizgi; tek geri alma katmanı da kaldırır;
  Ortofoto karşılaştırması'nın testleri yeni sözlerle geçer. Resimler `.run/gui-evidence/birlesme-cizgisi-4700/` (şeritteki
  yeri dahil).

## Sonra

- İkiden çok görüntünün birleşme çizgisi ağı (üçlü düğüm, çoklu örtüşmede farkın en küçük olduğu yer).
- Kenar algılama ve yüksek nesnelerden uzaklaştırma (Metashape'in Refine seamlines'ı); NoData'dan ayak izi
  (gdal_footprint, ArcGIS Build Footprints).
- Görüntüden türeyen nesnenin kaynağa izi (MAP-01'in sahip kararı).
- Web (TODOS §0.2).
