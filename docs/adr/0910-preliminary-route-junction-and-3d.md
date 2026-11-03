# 0910 — KentOS Yol: ön güzergâh, kavşak ve 3B

Tarih: 2026-11-03. plan/road.md Y12 (ROAD-ALIGN-13; ROAD-JUNC; 3d.md bağlantısı). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Koridor analizi:** Netpro'nun Koridor Analizi ve Bentley OpenRoads ConceptStation'ın ön güzergâh araçları aks
  tasarımından önce arazide en düşük maliyetli geçişi arar; Esri'nin Cost Path ve Optimal Path aracı aynı ilkeyle
  (birikimli maliyet yüzeyi ve geri izleme) çalışır. KentOS'un raster uzaklık ve maliyet çözümlemesi
  ([ADR 0236](0236-distance-and-cost.md)) ve uygunluk analizi ([ADR 0237](0237-multi-criteria-suitability.md)) vardır;
  yol projesinde arazi bir TIN yüzeyidir ([ADR 0900](0900-road-terrain-surface.md)), maliyet onun eğiminden gelir.
- **Bulunan yol aks değildir:** plan ROAD-ALIGN-13 “ortaya çıkan raster path teknik aks olarak ayrıca fit/QA/onaydan
  geçsin” der. KentOS'ta bu, çizgiden aks oturtmadır ([ADR 0908](0908-road-stakeout-sheets-and-exchange.md) §12,
  `road.alignments.fit`).
- **Kavşak:** Autodesk Civil 3D'nin kavşak sihirbazı
  ([To Create an Intersection](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-6C3D8C2B-890B-4A78-90D6-DC71AC82FDC3.htm))
  iki aksın kesişiminde kurulur; kenarlar akslara ötelenmiş hatlardır, bordür dönüşü çeyrek başına yarıçaplı daire yayı
  (ya da pah, üç merkezli yay), “Copy These To All Quadrants” bir değeri bütün çeyreklere verir. KentOS'un ilk kavşağı
  daire yayıdır; pah ve üç merkezli yay, kenarın kendi hattı (ötelenmiş aks) ve kavşak boykesitleri sonraya.
- **Sınırlar:** en dik eğim ve tırmanmanın ağırlığı kurumun ve projenin seçimidir; KentOS değer vermez
  (plan/road.md ROAD-DEC-01).

## Karar

1. **Ön güzergâh (ROAD-ALIGN-13).** `road.routes.find` arazi yüzeyinde iki yer arasında en düşük maliyetli yolu bulur,
   hiçbir şey yazmaz; `road.routes.draw` onu bir katmana kotlu çoklu çizgi olarak çizer (tek adım “Ön güzergâh”). Izgara
   iki yerin kutusu ve çevresinde `margin`, `cell` aralıklıdır (en çok iki milyon düğüm). Düğümün kotu yüzeyden
   okunur ve mikrometreye yuvarlanır (aynı arazi her yerde aynı yolu versin diye); yüzeyin dışındaki ve `avoid`
   katmanlarının alanlarındaki (dış halkanın içi, deliklerin dışı, köşeler düz birleşir, yaylar kirişleriyle)
   düğümlere girilmez. Bir düğüm 16 komşusuna bağlıdır (düz, çapraz ve at hamlesi; yol 45°'lik dönüşlere
   bağlanmasın diye); adım plan boyu artı `climb` × aştığı kot farkı tutar; `grade`'ten (yüzde) dik adım atılmaz.
   En küçük maliyet önce, eşitlikte küçük düğüm önce yerleşir; maliyet yalnız kesin küçülünce değişir, komşular sabit
   sırada denenir. Yolun başı ve sonu sorulan yerlerdir, arası düğümler; `tolerance` ile Douglas–Peucker (planda).
   Cevap: noktalar, plan boyu, toplam kot farkı, en dik bölüm, maliyet, düğüm ve yerleşen düğüm sayısı. At hamlesi
   uçları denetler; tek düğüm kalınlığındaki yasak şeritten atlayabilir: alan en az iki hücre geniş olmalıdır.
   Retler: `invalid_route`, `route_blocked` (baş ya da son girilemeyen düğümde), `route_unreached` (eğim yetmiyor).
   Masaüstünde Güzergâh › Ön güzergâh: yüzey, iki yer (Y, X), hücre, pay, en dik eğim, tırmanma ağırlığı,
   sadeleştirme, kaçınılacak katmanlar; yazıldıkça plan önizlemesi (kaçınılan alanlarla) ve özet; Planda çiz.

2. **Kavşak (ROAD-JUNC-01, 02, 04).** Kavşak yol ayarlarında `junctions[]`'tır: kalıcı kimlik, iki kol (`legs`:
   her biri PI'larıyla kurulan bir aks, aksın kurulduğu sürüm, `left` ve `right` kenarların aksa uzaklığı), hangi
   kesişim (`at`, birinci kolun km sırasıyla), tür (`atGrade` hemzemin: kollar bağlı, dört çeyrekte bordür dönüş yayı;
   `separated` katlı: kollar bağlı değil, yay yok; plan ROAD-JUNC-02: köprünün altındaki yol grafikte bağlanmaz) ve
   hemzeminde `radius` (her çeyrek) ile isteğe bağlı `radii` (dört çeyrek: birinci kolun sağı ile ikincinin sağı, sağ ile
   sol, sol ile sol, sol ile sağ). Kesişim iki aksın her metrede örneklenmesi ve kesişen parçaların iki km üstünde
   Newton yöntemiyle inceltilmesiyle bulunur. Kollar kesişimde teğetleri boyunca düz kabul edilir: her çeyrekte yay iki
   kenara teğet ve ikisinin dışında kalan daire yayıdır (merkez iki kenarın yarıçap kadar dışa ötelenmiş hatlarının
   kesişimi, teğet noktaları merkezin kenarlara izdüşümü), teğet noktalarının kollar boyunca km'si kesişimin km'si artı
   teğet boyunca uzaklık. Bir kol yayların eriminde kurptaysa `junction_on_curve` (yay kolu düz kabul eder); bir kolun
   aksı değişince `junction_stale`, `rebind` aksların bugünkü sürümünü alır. Kesişmeyen kollar, olmayan kesişim ve
   kesişimde paralel kollar ret (`invalid_junction`). Komutlar `road.junctions.edit` (tek adım “Kavşak”),
   `road.junctions.query`, `road.junctions.draw` (hemzeminde dört yay; `labels` ile kesişimde ad ve açı; katlıda ret).
   Masaüstünde Güzergâh › Kavşak.

## Aşamalar

Y12.1 (yapıldı, ön güzergâh): `road.routes.find` 26, `.draw` 22 ortak durum (`road_route_cases.py`; arazinin kotları
road_profile_cases.py'nin TIN'inden, arama §1'den; KentOS kodu yok; sınırlar durumlarındır). Çekirdek
`kentos_road::route`. Python, MCP (`tests/road_routes.rs`: bul, yasak alandan dolaş, çiz ve aks olarak oturt),
masaüstünde Güzergâh › Ön güzergâh. Ayarı yazılmaz, `.kcad` değişmez.

Y12.2 (yapıldı, kavşak): `road.junctions.edit` 29, `.query` 20, `.draw` 21 ortak durum (`road_junction_cases.py`;
akslar road_alignment_cases.py'nin 40 basamaklı başvurusundan, kesişim 40 basamakta Newton'la, yaylar §2'den; KentOS
kodu yok). Çekirdek `kentos_road::junction`, KCAD şema 161'in `junctions`'ı bağımsız okuyucuda
(`broken/road-junction-on-parallel-axis.kcad`; örnek `road-work.kcad`'e bağlantı yolu aksı ve kavşağı eklendi). Python,
MCP (`tests/road_routes.rs`: örneğin kavşağı sorulur, genişletilir, çizilir, katlıya çevrilir), masaüstünde Güzergâh ›
Kavşak: kollar, kenarlar, tür, yarıçap ve çeyrek yarıçapları; yazıldıkça kesişimin çizimi, km'ler, açı ve çeyreklerin
tablosu; Kaydet, Sil, Planda çiz.

## Sonra

Kavşak (Y12.2: kalıcı kavşak modeli, kollarının aks başvuruları ve km'leri, bordür dönüş yayları), 3B sahne (Y12.3,
3d.md), maliyetin arazi örtüsü ve kamulaştırma rasterleriyle çok ölçütlü bileşimi (ADR 0237'nin uygunluk yüzeyi
maliyet olarak), birden çok seçenek yolun karşılaştırılması.
