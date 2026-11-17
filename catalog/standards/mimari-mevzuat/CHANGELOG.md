# Değişiklikler

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
