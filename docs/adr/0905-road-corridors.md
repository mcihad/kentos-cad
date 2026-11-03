# ADR 0905 — Koridor: bölgeler, km örneklemesi, öznitelik çizgileri, üst yüzey ve şevli kotlu plan

Tarih: 2026-11-03. plan/road.md Y7 (ROAD-COR-05…17, ROAD-MOD-06). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** koridor bir ya da birkaç taban çizgisinin (aks ve profil) bölgelerinden kurulur; her bölgenin başı, sonu,
  kesiti (assembly) ve sıklığı vardır. Sıklık aliymanda, kurpta ve klotoidde ayrı aralık; kurpta aralık yerine *By
  Curvature* (orta ordinat, yani kirişin yaydan sapması) seçilebilir; yatay geometri noktalarında, dever kritik
  noktalarında, profil geometri noktalarında, profilin tepe ve çukurunda kesit eklenir; yatay ve düşey ayarın ikisi aynı
  yere düşerse küçük aralık kazanır. Kullanıcı km'leri elle ekler. Bölgeler geometriye ya da km'ye kilitlenir. Koridor
  yüzeyi kodlu bağlantılardan (top, datum), öznitelik çizgileri kodlu noktalardan (crown, ETW, daylight) kurulur.
  [Baseline and Region Parameters](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-84DF6A7F-47E9-47BC-A40D-EDD1558B5F5B.htm),
  [Corridor Region Properties](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-A23AC175-DA52-496C-8982-3AE31BAC903E.htm),
  [Frequency to Apply Assemblies](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-4D57F0BE-4EAF-4CBB-AE80-F9C6F45A7AE1.htm),
  [WisDOT corridor properties](https://c3dkb.dot.wi.gov/Content/c3d/rdwy-dsn/crdr-fndmntl/crdr-prprtis.htm).
- **OpenRoads Designer:** koridor yatay ve düşey geometriden kurulur, şablon kullanıcının verdiği km aralığına şablon
  yerleşimiyle (*template drop*) bırakılır; kesitler yerleşimin aralığıyla; koridordan arazi modeli kurulabilir (dinamik
  değil). [New Template Drop](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/406410/GUID-8730EE1D-F9E8-46FB-B655-1D8D2D5BCD40.html),
  [FHWA ORD manual, proposed model](https://highways.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13271/ord-manual22-proposed-model_1.pdf),
  [CDOT: creating and manipulating the corridor](https://www.codot.gov/business/designsupport/cadd/ord/openroads-designer-10-10-for-roadway-designers-self-paced-training/creating-and-manipulating-corridor/creating-and-manipulating-the-corridor.pdf).
- **Netcad Netpro:** yol platformu tanımı, şevli kotlu plan (platform kenarları, şev ayakları, şev taraması, kırmızı
  kotlar) ve kırmızı kot özeti ([NETCAD NETPRO](https://www.academia.edu/4871392/NETCAD_NETPRO),
  [Yol Projesi (Netpro)](https://publish.mersin.edu.tr/index.php/geoengbooks/article/download/722/620)); Netpro 12'de
  dinamik revizyon ve kendiliğinden yeniden üretim seçeneği (plan/road.md R-E kaynakları).

## Karar

1. **Kaynak (ROAD-COR-06, 17).** Koridor `settings.road.corridors[]`: ad, aks, boykesit, yüzey, isteğe bağlı dever
   çizelgesi, şerit genişliği (sol ve sağ arama genişliği, enkesitteki gibi), sıklık ve sıralı bölgeler. Bölge: baş ve son
   km (aksın istasyonu, m), tip kesit. Bölgeler kesişmez (`Overlap` ret), aksın dışına taşmaz, boş olamaz; aralarında
   boşluk olabilir ve raporda “kesit yok” aralığıdır. Kaynakların revizyonları saklanır; aks değişince koridor eskir
   (ADR 0901, 0903'ün kuralı); yazma tek adım “Koridor”.
2. **Km örneklemesi (ROAD-COR-08, 09).** Her bölgede: bölgenin başı ve sonu; aliymanda aralığın katları (aksın
   istasyonunda 0'dan); kurpta ve klotoidde elemanın eşit bölünmesi, adım kurp aralığından ve isteğe bağlı sapma
   toleransından (en küçük yarıçap R'de kirişin sapması t: adım ≤ 2·√(2Rt − t²)) küçük olanı; yatay geometri noktaları
   (TS, SC, CS, ST), profilin PVI'leri ile düşey kurp uçları, tepe ve çukuru, dever kritik km'leri ve kullanıcının
   km'leri seçilirse. 1 mm'den yakın km'ler bir km olur; nedeni güçlü olan kalır (bölge, geometri, dever, profil,
   kullanıcı, kurp, aralık sırasıyla). Kurpta adım sapma toleransını sağlar: kiriş yaydan t'den çok sapmaz.
3. **Kesit.** Her km'de enkesitin kuralı (ADR 0903): kesit hattı, arazi, profilin kotunda tip kesit, dever varsa şeritler
   eğimini alır (ADR 0904 §3). Kesit kurulmazsa (profilin dışı, arazinin dışı) km çözülmemiştir; bir yanın şevi
   araziyle buluşmazsa o yan açıktır. Çözülmeyen ve açık km'ler sol ve sağ için aralık aralık raporlanır (ROAD-COR-10, 16).
4. **Öznitelik çizgileri (ROAD-COR-11).** Kesitin noktaları kodludur: `axis`; her yanda bileşenlerin dış kenarları
   `left.c1…cN` (bileşenin türüyle: şerit, banket), menteşe `left.hinge` (son bileşenin kenarı), hendeğin şev ayağı ve
   taban sonu `left.ditchToe`, `left.ditchBottom`, şev ayağı `left.daylight`. Aynı kodun ardışık km'lerdeki noktaları
   3B çizgi olur; kod bir km'de yoksa çizgi orada kırılır.
5. **Üst yüzey (ROAD-COR-11, 13).** Ardışık iki çözülmüş km'nin kesit çizgileri (soldan sağa, açık yanda menteşeye
   kadar) şerit üçgenlemesiyle bağlanır: iki çizgi üstünde baştan ilerlenir, her adımda plandaki kısa köşegen seçilir,
   eşitlikte (1 nm içinde: dikdörtgenin iki köşegeni eşittir, yuvarlama seçmemeli) öndeki çizgi ilerler. Üçgenler plan alanı sıfır ya da ters dönmüşse (kesitler planda kesişiyor: dar kurpta
   geniş şerit) yüzey o aralıkta kurulmaz, `corridor_folded` uyarısı km aralığıyla verilir.
6. **Şevli kotlu plan (Netpro).** Çizim komutu menteşe ve şev ayağı çizgilerini, platform kenarlarını, şev taramasını
   (menteşeden şev ayağına kısa ve uzun çizgiler, yarmada ve dolguda ayrı katman rengi) ve km'lerde kırmızı kotu yazar;
   tek adım “Şevli plan”.
7. **Dinamik yeniden üretim (ROAD-MOD-06, ROAD-COR-14, 15).** Koridorun sonucu saklanmaz, her okumada kaynaklardan
   kurulur; kaynağın revizyonu değişince rapor “eski” der, Aksa yeniden bağla tek adımdır. Çizilmiş şevli plan bir
   türevdir: kaynağı değişince rapor onu eski sayar, yeniden çizim tek adımdır. Artımlı kurma (yalnız etkilenen km'ler)
   ve kendiliğinden yeniden üretim sonraki aşamadır.

## Aşamalar

Y7.1 (yapıldı, `4ad9fad85`): `kentos_road::corridor` (bölgelerin denetimi, km örneklemesi, öznitelik çizgileri, şerit
üçgenlemesi, çözülmeyen aralıklar ve katlanan şeritler); 26 ortak durum (`road_corridor_cases.py`, aks başvurusu ve
kesirler). Y7.2 (yapıldı, `b4e6aad8b`): `road.corridors.edit` (set, rebind, remove; tek adım “Koridor”) ve `.query`
(km'ler nedenleriyle, boşluklar, çözülmeyen aralıklar, katlanan şeritler, öznitelik çizgileri, isteğe bağlı noktalar ve
üst yüzeyin üçgenleri); koridorun kullandığı tip kesit, boykesit, dever ve aks silinmez; 35 + 22 ortak durum
(`road_corridor_command_cases.py`: kesitler enkesit başvurusundan); `.kcad` şema 160'ın `corridors`'ı ve bozuk örneği;
Python, MCP. Y7.3 (yapıldı, `5ece6c106`): `road.corridors.draw` şevli kotlu plan (köşe kotlu öznitelik çizgileri, şev
taraması, kırmızı kotlar; tek adım “Şevli plan”), 26 ortak durum. Y7.4: Koridor penceresi (`corridor_window.rs`;
`corridor.editor`, `corridor.new`, `corridor.slopePlan`; şeritte Koridor sekmesi): kaynaklar, örnekleme, bölgeler,
yazdıkça kurulan plan (üst yüzey, öznitelik çizgileri, çözülmeyen km'ler), uyarılar, Aksa yeniden bağla ve Şevli plan
çiz. Resimler `.run/gui-evidence/yol-y7-*`.

## Sonra

Datum ve formasyon yüzeyi (üstyapı katmanları gelince), birden çok taban çizgisi ve kavşak, hedefe bağlı genişlik ve kot
(ROAD-COR-07), artımlı kurma, kendiliğinden yeniden üretim, 3B katı model (ROAD-COR-12, 18).
