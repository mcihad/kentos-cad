# 0915 — KentOS Yol: tasarım aracının dönüş izi

Tarih: 2026-11-04. plan/road.md Y17 (ROAD-JUNC-06, 07; kavşağın kalanı). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D ve Vehicle Tracking:** tasarım aracı genişlik, etkin ön ve arka çıkıntı (dingilden gövdenin ucuna),
  çekicinin ve römorkun etkin dingil aralığı, mafsal (kingpin) ötelemesi ve çeki demiri boyuyla tanımlanır; sınırlar
  benimsenen standardın değerleridir
  ([Swept Path Analysis: Design Vehicle](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-E5C35EED-3DA5-4C23-AD69-D88717BD21D4.htm),
  [Junction Properties: Swept Path Analysis: Design Vehicles](https://help.autodesk.com/cloudhelp/2016/ENG/Autodesk-VehicleTracking-Help/files/GUID-DF57841D-4CE5-4435-97F2-7866FB7D82B4.htm),
  4 Kasım 2026'da okundu). Vehicle Tracking'in sürüş kipleri (AutoDrive, yay, izle, betik) bir yolu izletir.
- **AutoTURN (Transoft):** Civil 3D ve OpenRoads içinde çalışan yaygın dönüş izi aracı; araç kitaplıkları, IntelliPath ile
  kendiliğinden yol, 3B zarf
  ([AutoTURN](https://transoftsolutions.com/civil-and-transportation/software/swept-path-analysis/autoturn/),
  [Custom Vehicle Creation](https://www.transoftsolutions.com/custom-vehicle-creation-for-vehicle-tracking-and-swept-path-analysis/)).
  OpenRoads Designer'ın kendi dönüş izi belgesi bulunamadı; AutoTURN onun içinde de kullanılır. Austroads dönüş şablonları
  bir yarıçapta çeşitli dönüş açılarının zarfıdır
  ([Austroads design vehicles and turning path templates](https://swov.nl/en/publicatie/austroads-design-vehicles-and-turning-path-templates-guide)).
- **Netcad:** dönüş izi için çevrimiçi resmî kılavuz bulunamadı.
- **Matematik:** düşük hızda kaymasız bisiklet modeli: ön ve arka dingil arasında boyu dingil aralığı olan katı bir çubuk.
  Ön dingil doğru boyunca s kadar giderken çubuğun gidiş yönüyle açısı traktris izler: tan(φ/2) = tan(φ₀/2) e^(−s/L)
  ([Tire track geometry](https://arxiv.org/pdf/math/0405445)); kapalı biçimli çözüm genelde olmadığından yol kirişlerine
  bölünür, her kirişte traktris kesindir. Daire üzerinde kararlı durumda arka dingil √(R² − L²) yarıçapındadır, fazla
  iç kayma R − √(R² − L²); mafsalı arka dingilde römork için √(R² − L₁² − L₂²).

## Karar

1. **Çekirdek `kentos_road::swept` (Y17.1).** `Vehicle` (dingil aralığı, ön ve arka çıkıntı, genişlik; isteğe bağlı
   `Trailer`: mafsalın arka dingilden geriliği, mafsaldan römork dingiline uzaklık, römorkun ön ve arka çıkıntısı ve
   genişliği); `drive` ön dingili yolun kirişleri boyunca örnekleme aralığıyla sürer, araç başta ilk kirişe hizalıdır;
   her örnekte dingiller ve gövdelerin dört köşesi; en büyük iç kayma (arka dingilin ve römorkun, başlangıç hizasıyla
   uzatılmış yoldan uzaklığı); `clearance` gövdelerin engel çizgilerine (bordür) en az uzaklığı ve örneği, değdiği yerde
   sıfır. Aracın ölçüleri kullanıcınındır (kurumun tasarım aracı); KentOS araç vermez.
2. **Komut (Y17.2).** `road.swept.query`: bir aracı bir yol boyunca (noktalar; çizimdeki çizgi, açık çoklu çizgi ya da
   yay; ya da kavşağın bir çeyreği: bordür dönüşüne `offset` uzaklıkta, iki kolda `approach` boyunca düz) sürer;
   dingillerin ve gövdelerin köşelerinin izleri, iç kayma ve engellere (çizimin nesneleri, kavşakta çeyreğin bordürü) en
   az açıklık; `road.swept.draw` izleri ve her birkaç örnekte gövdeleri çizer.
3. **Masaüstü (Y17.3).** Kavşak'ta Dönüş izi penceresi.

Açık: çok römorklu araç ve çeki demiri, hıza bağlı (kaymalı) model, AutoDrive gibi kendiliğinden yol, 3B zarf; kavşak
kollarının boykesit ve dever sürekliliği (ROAD-JUNC-03), koridorların sahiplik önceliği (ROAD-JUNC-05), köprü ve tünel
zarfı (ROAD-JUNC-08, 09), artımlı yeniden kurma (ROAD-JUNC-10).

## Aşamalar

Y17.1 (yapıldı, çekirdek): §1. `crates/shared/road/src/swept.rs`, testleri `tests/all/swept.rs`: düz sürüşte arka
dingil yolda ve gövdenin köşeleri; R = 20 dairede üç turda arka dingil √(R² − L²)'ye, römorklu dört turda √(R² − L₁² − L₂²)'ye
yerleşir; dik köşeden sonra arka dingil traktrisin kapalı biçimindeki yerde (1e-9); bordüre açıklık, kesişen ve gövdenin
içindeki bordür sıfır; retler.

Y17.2 (yapıldı, komutlar): §2. `road.swept.query` (`crates/native/application/src/road_swept.rs`): yol `path`, `uid` ya
da `junction` (biri), `reverse`, `step`, `obstacles`; yaylar step'ten uzun olmayan kirişlerle (`swept::arc_places`,
`bulge_places`); cevap yolun boyu, örnekler, ön, arka ve römork dingilinin izleri, gövdelerin köşelerinin izleri, iç
kayma, en az açıklık (uzaklık, yolun neresinde, hangi gövde ve engel, bordür mü); uyarı `swept_touches`, retler
`invalid_swept`, `not_a_road_project`, `unknown_junction`, `invalid_junction` (katlı kavşakta dönüş yok),
`entity_not_found`. `road.swept.draw` tek adımda (“Araç izi çiz”). 33 + 20 ortak durum
(`scripts/fixtures/road_swept_cases.py`: traktris, gövdeler, iç kayma ve açıklık ADR'nin kuralıyla yeniden yazıldı;
kavşağın dönüşleri road_junction_cases.py'nin cevabından; KentOS kodu yok). Python `kentos.cad.road.swept`, MCP. Yolda
bulunan: aynı doğrudaki iki parçanın, örtüşmeseler de “değiyor” sayılması (çekirdekte düzeltildi, testi var).
