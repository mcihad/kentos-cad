# Değişiklikler

## 0.51.0 — 19 Kasım 2026

- Çıkmadan saçak (`cati_olculeri`; docs/adr/8164): PAİY 42/3 "Binada çıkma yapılması halinde saçak genişliği çıkmadan
  itibaren 0.50 metreyi aşamaz." Olguların çatı biçiminde `overProjection` (çatının altındaki çıkmaların ön yüzünden
  saçak çizgisine en büyük uzaklık, m; çıkmanın yanları sayılmaz); aşan çatı `paiy_eaves_past_projection`. Satır yalnız
  çatının altında çıkma varken yazılır; planın saçak notu 42/3'ü bağlamaz. 2 yeni bağımsız durum (27).

## 0.50.0 — 19 Kasım 2026

- Binanın kendi plan notları (docs/adr/8163; kademe hattı, PAİY 4/1-mm, 7/12, 7/14): olguların katları binasının
  kendi notlarını (`planNotes`) ve adını (`buildingName`) taşır; binanın yazdığı ölçü projenin aynı ölçüdeki notunun
  yerine geçer (`katman.plan_notlari`). `bina_yuksekligi` ve `tahvil` her binayı kendi notlarıyla (ileti binanın
  adıyla), `kat_yuksekligi` binaları ayırarak, `subasman` her zemin katı, `cati_olculeri` her çatıyı, `cikmalar` her
  çıkmayı katının binasının notlarıyla okur. Binanın notunun satırı adıyla ayrıdır ("Plan notu: bina yüksekliği (A
  Blok)"), bulgunun kaynağı "A Blok plan notu (…)"; projenin notunu okuyan bina yoksa satırı kapsam dışı. Hiçbir bina
  not söylemiyorsa sonuçlar değişmedi. 9 yeni bağımsız durum (bina yüksekliği 3, kat yüksekliği 1, tahvil 2, subasman
  1, çatı 1, çıkma 1).

## 0.49.0 — 19 Kasım 2026

- Planın çıkma ve bodrum notları (docs/adr/8162): `cikmalar` planın `projectionMost` notunu 41/1'in taşma ölçüsünün
  (1,50 m) yerine okur (0: çıkma yapılamaz; 41 Beşinci Bölümde, planla değiştirilebilir), sınıra 3,00 m ve komşuya
  2,00 m 41/1'in kalır; notun satırı "Plan notu: çıkma". `bina_yuksekligi` planın `basementsMost` notuyla her binanın
  zemin katının altındaki katlarını sayar (`paiy_basement_count_over`). 5 yeni bağımsız durum (çıkma 2, bodrum 3).

## 0.48.0 — 19 Kasım 2026

- Saçağın parsel sınırı (`cati_olculeri`; docs/adr/8161): PAİY 42/2 "Saçaklar parsel sınırlarını aşamaz". Olguların
  çatı biçiminde `outside` (saçağın vaziyet planının parselini aştığı en uzak yer, m; vaziyet planı yoksa yok); aşan
  çatı `paiy_eaves_over_parcel`. Satır yalnız vaziyet planıyla yazılır. 2 yeni bağımsız durum (24).

## 0.47.0 — 19 Kasım 2026

- Planın çatı notları (`cati_olculeri`; docs/adr/8160): `ridgeMost` (mahya), `roofSlopeMost` (çatı eğimi, %) ve
  `eavesMost` (saçak) PAİY 40/4, 42/1 ve Sivas 40/5-g'nin değerlerinin önüne geçer (69/3; 40 ve 42 planla
  değiştirilebilir, 69/2 saymaz): not bağlar, yönetmeliklerin değeri iletide yanında; notun satırı sonucu söyler, notun
  bağladığı madde kapsam dışı. Eğim notu Sivas'sız da okunur (`paiy_roof_slope_steep`, düzeltmesi kenarları notun
  sınırına indirir). 6 yeni bağımsız durum (22).

## 0.46.0 — 19 Kasım 2026

- Çatının ölçüleri (`cati_olculeri`; docs/adr/8159; Mimari denetimin Rust kurallarından taşındı): PAİY 40/4 mahya
  saçak seviyesinden en çok 5,00 m, 42/1 saçak 1,00 m, 40/5 hava bacası çatı örtüsünü en çok 0,60 m aşar, BYKHY 58/2
  duman bacası mahyanın en az 0,80 m üstüne çıkar; Sivas seçiliyse 40/5-g mahya 4,00 m ve eğim %40, 40/5-e-2 çatı
  penceresi 1,20 × 1,50 m, 40/5-e-3 cepheye 1 m ve birbirine 2 m, 42/5 kar siperliği. Olguların çatılarında `form`
  (eğimli mi, mahya, saçak, eğimler, kar siperliği, ortası, parçalar, pencere aralıkları). Bulgular yerini (`at`) ve
  düzeltmesini (`fix`: `cad.arch.edit`'in çatı üstündeki işlemi; hava bacası 0,60 m'ye, duman bacası 0,80 m'ye, eğim
  %40'a, kar siperliği saçaktan 30 cm) taşır. Sürüm tablosunda 40/4, 40/5, 42/1, 58/2 ve Sivas'ın 40/5-g, 40/5-e-2,
  40/5-e-3, 42/5'i. 16 bağımsız durum.

## 0.45.0 — 19 Kasım 2026

- Planın subasman kotu hükmü (`subasman`; docs/adr/8158): projenin plan notları `plinthMost` (en çok) ve `plinthLeast`
  (en az, ±0,00'ın altında eksi) 4/1-yyy'nin +1,20 ve ±0,00'ının yerine geçer; notu aşan zemin kat planın notunun
  maddesiyle kalır, notun satırı sonucunu söyler; iki uç da plandaysa 4/1-yyy kapsam dışı, biri plandaysa öbür ucu
  okur. 14 bağımsız durum (6'sı yeni). `pyproject.toml` ve `uv.lock`'un sürümü manifestinkiyle eşitlendi (0.31.0'da
  kalmıştı).

## 0.44.0 — 19 Kasım 2026

- Tahvil (`tahvil`; docs/adr/8157): PAİY 9/1-c. Planın yalnız bina yüksekliği varsa kat adedine, yalnız kat adedi
  varsa yüksekliğe çevrilir: n katın yüksekliği 1,20 m subasman + zemin katın + (n − 1) normal katın 28/1'deki en çok
  yüksekliği (bölgenin bendi başvuru gününün hâliyle, planın kat yüksekliği notları önce; Sivas seçiliyse sıkı olanı).
  Binanın kat adedi ya da yüksekliği tahvili aşarsa uzman incelemesi (idareler subasmanı ve 28/2'yi farklı okur).
  `bina-yuksekligi` katkısı tahvilin bulgularını da verir. 16 bağımsız durum. Sürüm tablosunda 9/1-c'nin 1.10.2017
  hâli.

## 0.43.0 — 19 Kasım 2026

- Otel ve yurt yatak odasının kapısından kaçış uzaklığı (`yatak_kacisi`; docs/adr/8156): BYKHY 50/4 ve 50/5. Olguların
  mahallerinde `doorExit` (kapılarından en yakınının dış yanından en yakın kat çıkışına yürüyüş: uzunluğu, tek yönlü
  kısmı, ulaştığı çıkış sayısı, kapının açıldığı mahal). Tek çıkışa ulaşan odanın bütün yolu tek yön, iki çıkışa
  ulaşanın tek yönlü kısmı tek yön ve bütünü iki yön sınırına; Ek-5/B'nin otel (15/20, 30/45 m) ya da yurt (15/30,
  45/75 m) satırı; süitin iç odası süitin çıkış kapısından. `konaklama.sinif`: otel ve yurt sözcükleri birlikte geçen
  bina oteldir (önce yurt sayılıyordu; 50/3'e 1 durum). 21 bağımsız durum. Sürüm tablosunda 50/4'ün Ek-5/B'nin 9.7.2015 hâliyle metni.

## 0.42.0 — 19 Kasım 2026

- Binanın kendi ±0,00'ı (docs/adr/8154): kurallar katın kotunu binasının sıfırından okur (`katman.kot`: kot eksi
  katın `zero`'su): ±0,00 üstündeki katlar (`kat_yuksekligi.ustteki`), asansörün ve kaçış kapılarının bodrumu, yangın
  dayanımının, kuyuların ve emsalin zemin katı, kullanımın zemin katı, bina yüksekliği. Planın kat adedi bina bina:
  en çok katlı bina okunur (önce bütün binaların katları birlikte sayılıyordu). Bina yüksekliği 11, kat yüksekliği
  48 bağımsız durum (4'ü yeni).

## 0.41.0 — 19 Kasım 2026

- Subasman kotu (`subasman`; docs/adr/8153): PAİY 4/1-yyy. Her binanın zemin katı (adında Zemin, yoksa ±0,00'a en
  yakın, asma kat değil) ±0,00 ile +1,20 arasında; olguların katları binanın kendi ±0,00'ını (`zero`) taşır. 8
  bağımsız durum. Sürüm tablosunda 4/1-yyy'nin 1.10.2017 hâli.

## 0.40.0 — 19 Kasım 2026

- Çıkmalar (`cikmalar`; docs/adr/8152): PAİY 41/1 ve 41/2. Olguların `projections`'ı (zemin katın üstündeki katlarda
  mahallerin taban alanı dışındaki parçaları: açık ya da kapalı, alanı, katın kotu, alt kotu, parsel kenarlarına
  uzaklıkları). Yol cephesinde yapı yaklaşma sınırından en çok 1,50 m; yan, arka ve kamusal alan bahçesinde en çok
  1,50 m ve parsel sınırına en az 3,00 m; bitişik nizamda açık çıkma komşu sınırına en az 2,00 m; çıkma altı zeminden
  en az 2,40 m (döşemesiz çıkmada eksik veri). 12 bağımsız durum. Sürüm tablosunda 41/1 ve 41/2'nin 1.10.2017 hâli.

## 0.39.0 — 19 Kasım 2026

- 20/3'ün içte uygulanan ısı ve ses yalıtımı (`kaplamalar`; docs/adr/8151): olguların `insulations`'ı (duvarın iç
  yüzündeki Yalıtım katmanı, malzemesi, sınıfı, baktığı mahaller, iç yüzün bulunup bulunmadığı). Duvar kaplamasıyla aynı
  istek, katmanın baktığı mahallere göre; aynı malzemeli katmanlar tek bulguda; iç yüzü bulunamayan dış duvarda yetmeyen
  sınıf uzman incelemesi. `kaplamalar` katkısı 20/3'ün bulgularını da verir (8138'in duvar kaplaması bulgusu eksikti).
  Kaplamalar 39 bağımsız durum (5'i yeni).

## 0.38.0 — 19 Kasım 2026

- Kalın taş duvarlar bina bina (`emsal`; docs/adr/8150): olguların `masonry.storeys`'i (katın duvar payı, taban alanı
  payı, kalın duvarları). Bir binası kendi taşıyıcı sistemini söylüyorsa 20/8-j ve 5/8 her geleneksel binanın
  duvarlarını kendi kat sayısıyla okur: 3 katı aşan binanınkiler kalır, öbürlerininki geçer (eski uzman incelemesi
  kalktı); satırlar birleşir. Emsal 52 bağımsız durum (2'si yeni).

## 0.37.0 — 19 Kasım 2026

- 22/1-g (`emsal`; docs/adr/8149): olguların `shelter` sınırı (sığınağın alanı, binanın gerekli sığınak alanı brütüyle,
  aşanı). Satır sayımı söyler, geçti; başvuru 7.11.2025'ten önceyse gerekli alan Sığınak Yönetmeliği'nin önceki
  hâline bağlı: uzman incelemesi; bina programında bağımsız bölüm yoksa ve sığınak varsa eksik veri. Sürüm tablosunda
  22/1-g'nin 1.10.2017 hâli. Emsal 50 bağımsız durum (4'ü yeni); eski durumların satırlarına 22/1-g eklendi.

## 0.36.0 — 19 Kasım 2026

- 22/1-j (`emsal`; docs/adr/8147): toprak altındaki bodrumun ortak yönetim, toplantı, spor ve sosyal mekânları
  (olguların `amenity` sınırı: alanı, katlar alanının %5'i ve en çok 1000 m² olan sınırı, aşanı, ortak alan mı). Ortak
  alan değilse kaldı; ortaksa ticari amacı ve yalnız binada oturanların kullanımına yönelik olduğu çizimden okunmaz,
  uzman incelemesi. Sürüm tablosunda 22/1-j'nin 25.7.2019 hâli. Emsal 46 bağımsız durum (5'i yeni); eski durumların
  satırlarına 22/1-j eklendi (böyle mekân yok: kapsam dışı; 25.7.2019'dan önceki başvuruda satır yok).

## 0.35.0 — 18 Kasım 2026

- Binanın kendi taşıyıcı sistemi (docs/adr/8145): olguların katları binasının kendi sistemini (`structuralSystem`,
  `traditional`) taşır. `kat_yuksekligi` bir binası kendi sistemini söylüyorsa da katları binalarına ayırır (28/1-d bina
  bina). `emsal`: 22/1-m her binanın taşlık, dış sofa ve eyvanını kendi kat sayısıyla okur; kalın taş duvarların (5/8,
  20/8-j) toplamı bina bina ayrılmadığından birden çok geleneksel binanın biri 3 katı aşıyorsa uzman incelemesi. Kat
  yüksekliği 47, emsal 41 (birer yeni) bağımsız durum.

## 0.34.0 — 18 Kasım 2026

- Binanın kendi bölgesi (`kat_yuksekligi`; docs/adr/8144): olguların katları binasını (`building`) ve binasının kendi
  bölgesini (`zone`) taşır. Bir binası kendi bölgesini söylüyorsa katlar binalarına ayrılır, her binanınkiler kendi
  bölgesinin 28/1 bendiyle denetlenir (28/1-d'nin kat sayısı binanınkidir); aynı maddenin satırları birleşir. Hiçbir
  bina söylemiyorsa kural eskisi gibi. Kat yüksekliği 46 (2 yeni) bağımsız durum.

## 0.33.0 — 18 Kasım 2026

- Komşu binaya bitişik duvar (`bitisik`, yeni denetim; docs/adr/8142): vaziyet planının yeni "Bitişik (komşu bina)"
  kenarı boyunca, her katta, olguların yeni `building.party`'si (duvarlar ve açıklıklar, beyan edilen sınıfları ve
  uzunluklarıyla; ayrık nizamda okunmaz). PAİY 39/4 (1.10.2017): bitişik duvarda pencere ve kapı kaldı
  (`paiy_party_wall_opening`). BYKHY 25/1 (19.12.2007): bitişik duvar en az EI 90, taşıyıcı beyanlıysa REI 90; sınıfsız
  eksik veri, azsa kaldı (`bykhy_party_fire_wall`). BYKHY 25/2: yangın duvarında boşluk kaldı; kapı ve pencere en az
  E 45 değilse kaldı, sınıfsızsa eksik veri; kapının sınıfında C ya da S yoksa eksik veri; yeten kapının ve pencerenin
  kaçınılmazlığı (pencerenin sabitliği) uzman incelemesi (`bykhy_party_wall_opening`). 16 bağımsız durum.

## 0.32.0 — 18 Kasım 2026

- Yapı nizamı (docs/adr/8141): olguların yeni `building.layout`'u (projenin İmar durumu'ndaki Yapı nizamı: `detached`,
  `attached`, `block`, `twin`; PAİY 4/1-pppp, ikiz blok sayılır). `catilar`: yüksek olmayan binada bitişik nizamda 28/3
  (a ve b) yüksek binadaki gibi uygulanır (eksik veri, kaldı), ayrık, blok ya da ikiz nizamda kapsam dışı; nizam
  yazılmamışsa eskisi gibi uzman incelemesi, sözü nizamın nereye yazılacağını söyler. `kaplamalar`: 26/4 ayrık nizamdaki
  müstakil konutta kapsam dışı, başka nizamdaki tek konutlu binada uygulanır; nizamsız tek konutlu binada uzman
  incelemesi. Çatılar 37 (8 yeni), kaplamalar 34 (4 yeni) bağımsız durum.

## 0.31.0 — 18 Kasım 2026

- Çatının oturduğu döşeme (`catilar`; docs/adr/8139): BYKHY 28/3-a, yüksek binada (ve bitişik nizamda) çatının oturduğu
  döşeme yatay yangın kesici niteliğinde, en az EI 30 (Ek-3/B 10'un yangın kesicisi). Olguların yeni `roofs[].seat`'i:
  çatının katının üstünde, üstü o kotta (1 cm içinde) ve dış çizgisinin ortasını tutan döşeme ve teras döşemeleri.
  Yüksek binada döşeme yoksa ya da sınıfı beyan edilmemişse eksik veri, E ve I ile en az 30 değilse kaldı; yüksek
  olmayan binada tutmayan çatı uzman incelemesi (nizam çizimde yok) (`bykhy_roof_seat_fire_stop`). 28/3'ün başlığı
  "Yüksek ve bitişik nizam binada çatının döşemesi ve yanmazlığı".

## 0.30.0 — 18 Kasım 2026

- Mahallerin kaplamaları (`kaplamalar`; docs/adr/8138): mahal listesinin yazdığı döşeme, duvar ve tavan kaplaması adı
  projenin aynı adlı (ya da kimlikli) malzemesidir (olguların yeni `spaces[].finishes`'i); aynı adlı kaplamalar tek
  bulguda, mahalleriyle. Döşemeninki 26/2'nin, tavanınki 26/4'ün kaplamasıdır; yeni madde BYKHY 20/3 (9.7.2015):
  duvarlarda iç kaplamalar en az normal alevlenici, yüksek binada ve 100 kişiyi aşan sinema, tiyatro, konferans, düğün
  salonunda en az zor alevlenici; adı sayılmayan 100 kişiyi aşan mahalde normal alevlenici kaplama uzman incelemesi
  (`bykhy_wall_lining_reaction`). Malzemeler'de o adla malzeme yoksa eksik veri. İçte uygulanan ısı ve ses yalıtımı
  okunmaz.

## 0.29.0 — 18 Kasım 2026

- Korunumlu şaft ve yangın kesici (`dayanim`; docs/adr/8136): BYKHY Ek-3/B 8, korunumlu olduğu beyan edilen şaftın
  (yönetmelik tanımlamaz, müellif söyler; olguların yeni `shafts[].protected`'ı) duvarları REI 120, her yüzü; şaftın
  bir katındaki beyan katlarca bütün şafta geçer, katlarında Evet ve Hayır birlikteyse uzman incelemesi, çevresinde duvar
  çizilmemişse eksik veri, açık yeri varsa kaldı. Ek-3/B 10, yangın kesici beyan edilen duvar ya da döşeme (olguların
  yeni `elements[].fireStop`'u) EI 30, her yüzü: kendi satırı E ve I ile en az 30 istiyorsa o yeter, istemiyorsa (yalnız R)
  ayrıca okunur. Bulgu kodu `bykhy_element_resistance`. Ek-3'ün metnine yangın kesici ve asma tavan satırları eklendi.

## 0.28.0 — 18 Kasım 2026

- Asansör kuyusunun havalandırması (`kuyular`; docs/adr/8135): BYKHY 62/4'ün ilk cümlesi (19.12.2007), asansör
  kuyusunda en az 0,1 m² olmak üzere kuyu alanının 0,025 katı kadar havalandırma ve dumandan arındırma bacası ya da
  basınçlandırma. Katlarca tek kuyu (olguların yeni `shafts[].stack`'i) bir kez okunur: alanı katlarındaki en büyük
  alanı (`shafts[].area`), beyanı katlarından birinin (`shafts[].vent`: bacanın alanı ya da basınçlandırma). Baca
  istenenden küçükse kaldı, beyansız ya da alanı ölçülemeyen kuyu eksik veri, katlarında farklı beyan uzman incelemesi
  (`bykhy_elevator_shaft_vent`). Kapısı olmayan kuyu da 62/4'ün kapsamındadır; maddenin başlığı "Asansör kuyusunun
  havalandırması ve kapıları", metni derlenmiş resmî metinle birebir.

## 0.27.0 — 18 Kasım 2026

- Çatıların yanıcılığı (`catilar`, yeni katkı `catilar`; docs/adr/8134): BYKHY 28/2 (9.9.2009) çatı kaplamasının
  malzemesinin beyan edilen TS EN 13501-5 dış yangın sınıfı BROOF değilse kaldı, beyansız eksik veri
  `bykhy_roof_covering_class`; kaplamanın altındaki kaplama tahtası, su ve ısı yalıtımı en az zor alevlenici (hiç yanmaz
  kaplamanın tahtası normal alevlenici yeter; zor yanıcı kaplamada yanmazlık yazılı değil: uzman incelemesi)
  `bykhy_roof_underlay_reaction`; 28/3-b yüksek binada çatının taşıyıcısı (mertek) ve kaplaması yanmaz (hiç yanmaz),
  yüksek olmayan binada yanmaz olmayan çatı uzman incelemesi (bitişik nizamda kalır, nizam çizimde yok)
  `bykhy_roof_noncombustible`. Olgulara çatılar (`roofs`: katmanlar üstten alta, malzemenin yanıcılık ve dış yangın
  sınıfı). Metinler derlenmiş resmî metinle birebir.

## 0.26.0 — 18 Kasım 2026

- Kazan dairesi ve yakıt deposu (`kazan`, yeni katkı `kazan`; docs/adr/8113): BYKHY 54/2 kazan dairesinin komşu
  mahallerle ayrımı (duvar ve pencere en az EI 120, duvarsız yer ve eşiksiz boşluk yok; beyansız eksik veri)
  `bykhy_boiler_room_separation`; 54/4 (9.9.2009) kapısı merdivene ya da hol ve koridor olmayan mahale açılırsa kaldı,
  dışarı açılırsa uzman incelemesi `bykhy_boiler_room_door`; 54/5 net alanı 100 m²'yi aşan kazan dairesinde iki çıkış
  kapısı (aşmayan tek kapılıda ısıl kapasite çizimde yok: uzman incelemesi) `bykhy_boiler_room_exits`, kapıları en az 90
  dakika, duman sızdırmaz, kendiliğinden kapanır `bykhy_boiler_room_door_rating`; 56/1 yakıt deposu ile kazan dairesi
  arasında en az EI 120 `bykhy_fuel_store_separation`. Olgulara mahalin net alanı (`spaces[].net`). Metinler resmî
  metinle birebir.

## 0.25.0 — 18 Kasım 2026

- Otel yatak odasının içindeki kaçış uzaklığı (`konaklama`, kullanım sınıfı katkısında; docs/adr/8111): BYKHY 50/3,
  otel, motel ve pansiyonda (yurt ve yatakhane değil) odanın en uzak noktasından kapısına yürüyüş (olguların yeni
  `spaces[].toDoor`'u: duvarlardan 0,40 m önden, odanın içinden) 15 m'yi aşan tek kapılı oda kaldı
  (`bykhy_hotel_room_distance`, 50/3-a); tamamı yağmurlamalı otelde sınır 20 m (50/3-b), 20 m'yi aşan iki kapılı oda uzman
  incelemesi; kapısı çizilmemiş oda eksik veri. Metin resmî metinle birebir.

## 0.24.0 — 18 Kasım 2026

- Sağlık yapıları (`saglik`, yeni katkı `saglik`; docs/adr/8110): BYKHY 49/1-b (9.9.2009) hastanenin ve bakımevinin
  300 m²'yi aşan yatılan katı tek yangın kompartımanıysa kaldı (`bykhy_patient_floor_compartments`); iki ya da daha çok
  kompartımanda "en az yarısı büyüklüğünde" şartı ve yatay tahliye alanının 2,8 m²/kişi hesabı uzman incelemesi
  (kompartımanların alanlarıyla). 49/2: hastanede adı Koridor olan mahalin en dar yeri 2,00 m'den azsa kaldı
  (`bykhy_hospital_corridor`); en dar yer olguların yeni `spaces[].least` ölçüsü: iki yanında kendi genişliğinde daire
  kalan en kısa kesit (L, T, artı ya da holle birleşen koridorun en büyük dairesi değil). Metinler resmî metinle birebir.
  Kullanım sınıfı katkısı artık yalnız hasta odası kapısı bulgularını alır.

## 0.23.0 — 17 Kasım 2026

- Yağmurlama zorunluluğu (`yagmurlama`, yeni katkı `yagmurlama`; docs/adr/8108): BYKHY 96/2'nin bentleri, proje
  yağmurlamayı beyan etmemişse `bykhy_sprinklers_required`: a) 30,50 m'yi aşan konut harici (karma da), b) 51,50 m'yi
  geçen konut, c) alanlarının toplamı 600 m²'yi aşan kapalı otopark, ç) 100 odayı aşan katlı konaklama yapısı (yatak
  sayısı çizimde yok: odası 100'ü geçmeyen katlıda eksik veri) ve 21,50 m'yi aşan yataklı tesis, d) 2000 m²'yi aşan katlı
  mağaza, alışveriş, ticaret ve eğlence yeri (karma binada ticaret mahalleri), e) 1000 m²'yi aşan parlayıcı madde yapısı
  (uzman incelemesi). Alanlar olguların mahal alanları (dış kontur, PAİY 4/1-ğ-1). Metin resmî metinle birebir. 26 bağımsız
  durum.

## 0.22.0 — 17 Kasım 2026

- Kullanım sınıfına özel kaçış hükümleri (`kullanim`, yeni katkı `kullanim`; docs/adr/8107): binanın kullanımı bina
  programından (yoksa mahal adlarından, sıkı okumayla) okunur. BYKHY 49/1-a: sağlık yapısında 15 kişiyi aşan hasta
  yatak odasının 2 kapısı (`bykhy_patient_room_doors`); 50/1-a: otelde yatak odası ile iç koridor arasındaki duvar en
  az 60 dakika (`bykhy_hotel_room_wall`), kat sayısı ikiden fazla otelde her katta 2 çıkış (`bykhy_hotel_storey_exits`;
  yatak sayısı çizimde yok: yapı yüksekliği 15,50 m'nin altındaysa eksik veri); 50/1-b: iç koridora açılan kapı en az
  30 dakika ve kendiliğinden kapanır (`bykhy_hotel_room_door`); 52 (9.9.2009): fabrika, imalathane, mağaza, dükkân, depo,
  büro ve ayakta tedavi merkezinde 2 bağımsız kaçış merdiveni, tek merdivende (a) yapı yüksekliği ve (b) katın kişisi
  tutmazsa kaldı, tutarsa (c)–(d) uzman incelemesi (`bykhy_single_stair`). Metinler mevzuat.gov.tr'nin derlenmiş
  metninden (SHA-256 d82d06d2…). 24 bağımsız durum.

## 0.21.0 — 17 Kasım 2026

- Bina yüksekliği ve kat adedi (`bina_yuksekligi`, yeni katkı `bina-yuksekligi`): PAİY 5/11'e göre planın bina
  yüksekliği ve kat adedi aşılamaz; değerler projenin plan notlarıdır (`buildingHeight`, `storeys`). Bina yüksekliği
  ±0,00'dan (binanın kot aldığı nokta) en üst katın tavan döşemesinin üstüne (4/1-n, 4/1-ppp); kat adedi ±0,00 ve
  üstündeki zemin ve normal katlar (asma kat ve çatı arası sayılmaz). Aşan `paiy_building_height_over`,
  `paiy_storey_count_over`; en üst katın yüksekliği yoksa eksik veri. Satırlar "Plan notu: bina yüksekliği", "Plan notu:
  kat adedi". 8 bağımsız durum.
- Asansör (`asansor`): 34/1'in "uygulama imar planına göre kat adedi" önce parselin plan notundan (`storeys`), yoksa
  künyenin `kat_adedi`'sinden okunur; ileti nereden okunduğunu söyler. 1 bağımsız durum.

## 0.20.0 — 17 Kasım 2026

- Sivas 22/k (`emsal.sivas_duvarlar`): yerel yönetmelik Sivas seçiliyse geleneksel kesme taş yığma binanın kalın taş
  duvarları "toplamı %30'u geçmemek üzere" sayılır: tablonun %30'un içinde saydığına duvarların kalın kısmı eklenince
  sınırı aşarsa kaldı (`local_masonry_walls_over`; PAİY 5/8 onları %30'un dışında sayar, sıkı olan bağlar), emsal
  yazılmamışsa eksik veri. Olgular `building.exemptCap`'i (5/8'in %30'u ve içinde sayılan) taşır. 4 bağımsız durum.

## 0.19.0 — 17 Kasım 2026

- Kalın taş duvarlar (`emsal.duvarlar`): geleneksel kesme taş yığma binada taşıyıcı yığma taş duvarların 25 cm'yi (iç
  duvarda 12,50 cm'yi) aşan kısmı emsal harici, %30 dışında (PAİY 5/8, 31.12.2022'den), dış duvarlarınki taban alanı
  dışında (20/8-j, 9.10.2021'den); tablonun saydığı paylar olguların `building.masonry`'sinde. 12,50 cm'den kalın,
  taşıyıcılığı beyan edilmemiş duvar eksik veri (`paiy_masonry_wall_undeclared`); zemin dahil 3 kattan çok katlı binada
  ya da 9.10.2021'den önceki başvuruda kaldı, 9.10.2021–30.12.2022 başvurusunda duvarlar 22/1-m'de, %30'un içindeydi:
  uzman incelemesi (`paiy_masonry_walls_not_exempt`). 6 bağımsız durum.

## 0.18.0 — 17 Kasım 2026

- Geleneksel kesme taş yığma bina: projenin taşıyıcı sistem beyanı (`structuralSystem`: `cutStoneMasonry`, `traditional`)
  okunur. Kat yüksekliği (`kat_yuksekligi`): konut bölgesinde (konut alanı ya da zemin katında ticaret yapılabilen konut
  alanı) zemin dahil 3 katı geçmeyen binada zemin ve normal katlarda 4,50 m (PAİY 28/1-d, 9.10.2021'den; Sivas 28/1-d);
  daha eski başvuruda bölgenin kendi bendi. Emsal (`emsal`): tablonun emsal harici saydığı taşlık, dış sofa ve eyvan
  (22/1-m) bina zemin dahil 3 kattan çoksa, mahal zemin katta değilse ya da başvuru bendin eklendiği 9.10.2021'den önceyse
  kaldı; tutuyorsa bahçeyle bağlantısı okunmadığı için uzman incelemesi (`paiy_traditional_not_exempt`); balkonları
  22/1-l'nin %20'siyle sayılır. Güne bağlı emsal bulgusu aday günlerin mahallerini seçer. 16 bağımsız durum.

## 0.17.0 — 17 Kasım 2026

- Kapı ölçüleri (`kapi_olculeri`): 39/1-b'nin çift kanadı: bina girişinde bir kanadın net genişliği en az 1,00 m (ilk
  kanat geniş ya da eşitse tek başına net geçişi; ikinci kanat genişse uzman incelemesi; `paiy_door_leaf_below`); 39/1-ç:
  döner bina giriş kapısının yanında (aynı mahale açılan) döner olmayan bir bina giriş kapısı yoksa kaldı
  (`paiy_revolving_door_alone`), döner kapının kendi net genişliği okunmaz. Olgular kapıların `leaves` ve `clearFirst`'ü.
  5 bağımsız durum.

## 0.16.0 — 17 Kasım 2026

- 62/4'ün ikinci cümlesi (`kuyular`): bodruma da inen asansörün bodrum kattaki kapısının önü yangın güvenlik holü ya da
  korunmuş koridor (4/dd, 36/1-a: bitişik mahallerden Ek-3/C'nin bodrum süresince EI ile ayrılmış, açık yeri olmayan);
  değilse kaldı, ayrımın sınıfı yoksa ya da Ek-3/C'nin süresi okunmuyorsa eksik veri (`bykhy_elevator_basement_lobby`).
  `dayanim.bodrum_suresi` Ek-3/C'nin bodrum süresini paylaşır. 7 bağımsız durum.

## 0.15.0 — 17 Kasım 2026

- Kaplamaların yanıcılığı (`kaplamalar`): BYKHY 26/2 (döşeme kaplaması en az normal alevlenici, yüksek binada en az zor
  alevlenici; `bykhy_floor_covering_reaction`) ve 26/4 (tavan kaplaması ve asma tavan en az zor alevlenici; tek konutlu
  binada nizam çizimde olmadığı için tutmayan tavan uzman incelemesi; `bykhy_ceiling_reaction`). Malzemenin beyan edilen
  TS EN 13501-1 sınıfı Ek-2/Ç'nin tablolarıyla (döşeme ve öbürü, kural setinde veri) yanıcılık özelliğine çevrilir. Olgular
  `building.coverings`. 16 bağımsız durum.

## 0.14.0 — 17 Kasım 2026

- Beyan edilen roller (`dayanim`): taşıyıcı iç duvar Ek-3/B 2 (R, Ek-3/C kadar); taşıyıcı dış duvar ve kompartıman
  duvarı R'yi de ister (REI); taşıyıcı olmayan dış duvarda Ek-3/C'nin not (3)'ü 120'yi 90'a indirir, uzman incelemesi
  kalmaz; kaçış yolu olan çatı Ek-3/B 4-a, alt yüzeyden REI 30. Olgular `elements[].loadBearing` ve `escape`. 7 bağımsız
  durum.

## 0.13.0 — 17 Kasım 2026

- Asma tavan (`dayanim`): Ek-3/B 11, alt taraftan EI 30; Ek-3/C'nin sütunundan bağımsız (ev satırının süre vermediği
  yükseklikte de). Olgularda IFC türü CEILING olan kaplama (`elements[].kind` `ceiling`). 4 bağımsız durum.

## 0.12.0 — 17 Kasım 2026

- Yangın kompartımanları IFC için (`mimari.kompartimanlar`): her kompartımanın mahalleri, katları (alttan üste) ve Ek-4'ün
  saydığı alanı; en alt katı ve ilk mahali öne. Sınır 24/7'nin başvuru günündeki hâlinin süresiyle (gün yoksa en yeni
  hâl); gün maddenin bilinen ilk hâlinden önceyse kompartıman yok ve nedeni (`note`). Masaüstü, MCP ve komut satırı
  binanın IFC'sini yazarken sorar; her kompartıman dosyada mahallerinin `IfcZone`'udur.

## 0.11.0 — 17 Kasım 2026

- Şaftlar ve asansörler (`kuyular`): BYKHY 25/3 (yüksek binada, 4/ccc: bina yüksekliği 21,50 m'den ya da
  yapı yüksekliği 30,50 m'den fazla, tesisat şaftı ve bacanın duvarı en az EI 120, kapağı en az E 90 ve duman sızdırmaz,
  açık yer yok; `bykhy_shaft_resistance`), 41/9 (elektrik ve mekanik tesisat şaftının kapağı kaçış merdiveni yuvasına ya
  da yangın güvenlik holüne açılmaz; çöp bacası sayılmaz; `bykhy_shaft_cover_stair`), 62/2 (asansör kuyusu en az EI 60;
  duvarındaki pencere ve boşluk da; `bykhy_elevator_shaft`), 62/4 (asansör kapısı koridor, hol ve benzeri dışında
  kullanım alanına açılmaz; `bykhy_elevator_door_room`), 62/6 (yangın merdiveni yuvasına açılmaz;
  `bykhy_elevator_door_stair`), 62/7 ve Ek-3/B 12 (9.7.2015'ten: kat kapısı yapı yüksekliği 51,50 m'den yüksek binada
  E 60, alçakta E 30, tam 51,50'de uzman incelemesi; `bykhy_elevator_door_resistance`). Olgular `building.shafts`,
  `structureHeight`, kapıların `elevator` ve `passage`'ı. 23 bağımsız durum.
- Ek-3/B 4 (`dayanim`): teras döşemesi (döşeme görevi yapan çatı, 4-b) REI, çatı (dıştan yangına maruz, 4-c) EI, Ek-3/C
  kadar. 2 bağımsız durum.

## 0.10.0 — 17 Kasım 2026

- Katlar arası açıklıklar: döşeme boşluğu, şaft olmayan boşluk mahal ve merdiven (olgular `links`) iki
  katın mahallerini aynı kompartımana koyar; Ek-4 alanı katlar boyunca toplanır. BYKHY 24/4 (9.7.2015): 21,50 m'yi aşan
  konut harici, 30,50 m'yi aşan konut binasında bu yüksekliğin üstündeki katlarda en çok üç kat bir kompartıman
  (`bykhy_compartment_storeys_over`); kaçış merdiveni yuvası sayılmaz. 24/5 (9.9.2009): merdiven yuvası olmayan boşluk
  4/g'nin atriumu, duman kontrolü ve 90 m²'den küçükse 45 cm duman perdesi çizimde okunmaz (`bykhy_atrium`, uzman
  incelemesi). 7 bağımsız durum; eski 22 durum tek katlı: 24/4 ve 24/5 kapsam dışı.

## 0.9.0 — 17 Kasım 2026

- Bölge: kat yükseklikleri parselin plandaki kullanımına göre (`building.zone`): ticaret alanı 28/1-a,
  karma alan 28/1-b (öbür katlar konutsa 3,60, değilse 4,00), konut alanı 28/1-c, zemin katında ticaret yapılabilen konut
  alanı 28/1-ç (asma katlı zemin katın 5,5'i 12.8.2023'te kalktı); öbür kullanımda 28/1 sınır vermez. Sivas 28/1-a, b, ç
  (zeminde 5,00, asma katlı 6,50). Asma kat (`storeys[].mezzanine`) zemin katın parçası: asma katlı zemin katın
  yüksekliği üstündeki ilk kata kadar. Bölge yazılmamışsa konut projesinde konut alanı sayılır, öbüründe eksik veri.
  12 bağımsız durum; 4 eski durumun beklentisi bölgeyle değişti.

## 0.8.0 — 17 Kasım 2026

- Plan notu katmanı: projenin yazdığı parsel plan notları (`building.planNotes`: zemin kat yüksekliği,
  kat yüksekliği; değeri ve yeri) kat yüksekliği kuralında yerel yönetmeliğin ve PAİY'nin önüne geçer (69/3; 28/1 kat
  yüksekliklerini plana bırakır). Notun bağladığı katta 28/1-c ve Sivas 28/1-c kapsam dışıdır, değerleri farklıysa
  iletide yanında yazılır; kural setinde notun kendi satırı ("Plan notu: zemin kat"). Plan notu konut projesi olmayan
  binada da okunur. `katman.PlanNotu`; 7 bağımsız durum.

## 0.7.0 — 17 Kasım 2026

- Kat yüksekliği ve kapı ölçüleri (`kat_yuksekligi`, `kapi_olculeri`): Planlı Alanlar İmar Yönetmeliği
  28/1-c (1.10.2017: konut bölgesinde 3,60 m; 12.8.2023: bodrum hariç 3 katı geçmeyen binada 4,00 m) ve 39/1 (1.10.2017:
  balkon ve tuvalet kapısı 0,80 m'ye düşürülebilir; 13.8.2025: düşürülemez), Sivas 28/1-c ve 39/1. Mimari denetimin Rust
  kurallarından taşındı; kodlar aynı. Bölümün dış kapısı bölüm girişi, ortak alanınki bina girişi; programın dış alanı
  dışarıdır. İki katmanın yargısı `katman.py`'de (sıkı bağlar, gevşeğin tuttuğu çelişki). Gün yoksa her kat ve kapı ayrı
  denenir (`yururluk.parcalarla`): güne bağlı olmayanın bulgusu kalır. Olgular `building.commons`, katların `height`,
  `bedrooms`'u, kapıların `clear`, `height`, `sides`, `toilet`, `balcony`'si. 41 bağımsız durum.

## 0.6.0 — 17 Kasım 2026

- Yangına dayanım süreleri (`dayanim`): BYKHY Ek-3/B ve Ek-3/C. Taşıyıcı sistemin kolonu ve kirişi R,
  döşemeler REI (bodrumla zemin kat arası en az 90, iki katlı evin ikinci katı 30), dış duvarlar ve kompartıman duvarları
  EI, korunumlu yangın merdiveni yuvası REI 120 (holüne 60); kullanım sınıfına, yağmurlamaya, bina yüksekliğine ve
  bodrum derinliğine göre Ek-3/C'nin dakikası. Tablolar PDF sayfalarından veri olarak. Yağmurlamasız konut dışı bina
  30,50 m'yi aşamaz. Sınıf okuyucu `yangin_sinifi.okuma` her harfi okur. Olgular `building.elements`, `height`,
  `basementDepth`. 18 bağımsız durum.

## 0.5.0 — 17 Kasım 2026

- Yangın kompartımanları (`kompartiman`): BYKHY 4/tt, 24/2, 24/6 ve Ek-4, 24/7. İki mahal, aralarındaki
  her duvar ve açıklık en az EI 60 ve açık yer yoksa ayrı kompartımandadır (döşemeler hep ayırır, 26/1). Kazan dairesi,
  otopark, elektrik, trafo, orta gerilim, jeneratör odasının ayrımları kompartıman duvarı (24/2); kompartımanın alanı
  Ek-4'ün kullanım sınıfını aşmaz, yağmurlama notları (24/6); EI 60 duvarda yangına dayanıksız açıklık yok (24/7). Sınıf
  okuması `yangin_sinifi.py`'de (kapılarınki de). Olgular `building.spaces`, `separations`, `sprinklers`. 22 bağımsız
  durum.

## 0.4.0 — 17 Kasım 2026

- Merdiven altı (`merdiven_alti`): Planlı Alanlar İmar Yönetmeliği 31/6. KentOS merdivenin altında,
  başladığı döşemeden 1,80 m'den alçak yerleri ölçer ve bağımsız bölüm net alanından düşer (4/1-ğ-3); kural alçak
  boşluğun bir mahale açık mı (kaldı), dolap, depo ya da merdiven altı mahalinde mi (geçti) olduğuna bakar; döşemesi
  olmayan merdivende eksik veri. Olgular `building.underStairs`.

## 0.3.0 — 17 Kasım 2026

- Emsal sınırları (`emsal`): Planlı Alanlar İmar Yönetmeliği 22/1-ç (emsal harici sayılan kapıcı
  dairesinin 44/1'e göre zorunluluğu: konut bölümü ve bağımsız bölüm sayısından; ısıtmaya bağlı ara eksik veri), 22/1-f
  (kütüphanenin alışveriş merkezi ya da 40'tan fazla bağımsız bölüm, normal kat, ortak alan koşulları; ortak merdivenle
  bağlantı ve dışarıdan erişim uzman incelemesi), 22/1-k ve 22/1-l'nin tablodaki sayımı özet satırında. Sınırları KentOS
  Emsal ve TAKS tablosunda sayar; olguların `building.limits`'inden okunur. 22 bağımsız durum.

## 0.2.0 — 17 Kasım 2026

- Kaçış yolu kapıları (`kapilar`): BYKHY 47/2 (kullanıcı yükü 50'yi aşan mekânın çıkış kapısı kaçış
  yönüne açılır; döner kanatlı olmayan kapı uzman incelemesi) ve 47/3 (kaçış merdiveni ve yangın güvenlik holü kapısı
  duman sızdırmaz, kendiliğinden kapanır; 4'ten az katta 60, bodrumda ya da 4'ten fazla katta 90 dakika; bodrumsuz tam 4
  kat uzman incelemesi; beyan yoksa eksik veri). Kapının sınıfı TS EN 13501-2 yazımıyla okunur. 20 bağımsız durum.
- Sınamalar paketin dışında (`catalog/standards/tests/mimari-mevzuat/`): doğrulayıcı paketin `sys.path` değiştirmesini
  reddeder.

## 0.1.0 — 17 Kasım 2026

- İlk sürüm: başvuru gününe göre kural seti (`yururluk`, `data/surumler.json`)
  ve asansör kuralı (`asansor`): Planlı Alanlar İmar Yönetmeliği 34/1'in 28.7.2018, 11.7.2021 ve 1.7.2026 hâlleri,
  34/8'in 11.7.2021 ve 1.7.2026 hâlleri; Sivas İmar Yönetmeliği 34/1 ve 34/8. Bulgu durumu: kaldı, eksik veri (başvuru
  tarihi yok ve sonuç ona göre değişiyor; okunamayan tarih), uzman incelemesi (metni kural setinde olmayan gün, yalnız
  teknik hacimli bodrum). Maddelerin özet satırları geçti, kaldı, eksik veri, uzman incelemesi ya da kapsam dışı.
