# 0919 — KentOS Yol: kavşakta iki koridorun örtüşmesi ve iki kez sayılan hacim

Tarih: 2026-11-04. plan/road.md Y17.6 (ROAD-JUNC-05). Durum: kabul, açıklar aşağıda.

## Dayanak

- **OpenRoads Designer:** Add Clipping Reference, aynı yüzeyi paylaşan birden çok koridorun örtüşen yerlerini kaldırır.
  Bentley'nin örneği bir yolun kestiği koridordur: kavşağın içindeki örtüşen öğeler silinir. Kırpma şekli başka bir
  koridor ya da bir dikdörtgen olabilir
  ([Add Clipping Reference](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/406410/GUID-DEAE8A44-10E7-4E2E-80FA-0E18F436AC6B.html),
  4 Kasım 2026'da okundu).
- **Civil 3D:** kavşak öğreticisi, kavşak yerleştikten sonra boşlukları kapatmak için koridor bölgelerinin yeniden
  düzenlenebileceğini söyler
  ([Creating and Editing a Corridor in the Intersection Area](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Tutorials/files/GUID-1A20CA5A-8087-4CAB-B71C-08121042BED8.htm)).
  Autodesk forumunda: örnek hatlar kavşağın iki yanında ayrı gruplara bölünürse kavşağın içi iki kez hesaplanır;
  yüzeyler arası hacim önerilir
  ([Computing volume using sample lines](https://forums.autodesk.com/t5/civil-3d-forum/computing-volume-using-sample-lines/td-p/6546020)).
  Grid volume surface, karşılaştırma ve taban yüzeyinin farkını verilen ızgara aralığının düğümlerinde alır. Bu bir
  yaklaşıktır; ızgara aralığı küçüldükçe sonuç doğrulaşır
  ([About Creating a Grid Volume Surface](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-62EC28FA-E636-4134-9153-4EE3328154A1.htm)).
- **Netcad:** NETPRO için kavşakta koridor örtüşmesini anlatan çevrimiçi resmî belge bulunamadı.

## Karar

### §1 Komut

`road.junctions.overlap` v1, yalnız okur. Girdi:

- `junction`; `corridors`: kolların sırasıyla iki koridor (yoksa her kolun aksının tek koridoru);
- `primary`: kavşağın alanı kendisinin sayılan kol (0 ya da 1), öbürü tali koridor;
- `cell`: ızgara aralığı (m); kullanıcınındır.

### §2 Hesap

İki koridorun üst yüzeyi (şevleri dahil üçgenleri) planda dünya koordinatında `cell` aralıklı ızgaranın hücre
ortalarında örneklenir. Izgara, iki yüzeyin kutularının kesişimini kapsayan hücrelerdir. Satırlar y'ye, hücreler x'e
göre artar. İki yüzeyin de üçgeninde kalan hücre örtüşmedir. Örtüşmenin alanı hücre sayısı × `cell`²'dir. Tali koridorun
yüzeyi kendi arazi yüzeyiyle karşılaştırılır: hücre başına `cell`² × (arazi − tasarım) yarma ya da (tasarım − arazi)
dolgu. Tali koridorun kübajı bu hacmi, ana koridorunki de aynı alanı saydığından ikisi toplanınca bu kısım iki kez
sayılır. Arazisi olmayan hücreler `uncovered`'dır. İki tasarım yüzeyinin farkı (ana − tali) ortalaması ve en büyüğüyle
verilir; en büyüğü yeriyle, eşitlikte ızgaranın ilk hücresi. Her aksta örtüşen hücre ortalarının en küçük ve en büyük
km'si verilir. Bu, tali koridorun bölgesinin kırpılacağı km aralığıdır.

### §3 Ret ve uyarılar

Ret sırası: `not_a_road_project`, `unknown_junction`, `invalid_junction`, `unknown_corridor`, koridorun kurulma
retleri, `invalid_overlap` (iki koridor değil, koridor kolun aksında değil, kolun koridoru yok ya da birden çok,
`primary` 0 ya da 1 değil, `cell` 0'dan büyük değil, en çok 1 000 000 hücre, tali koridorun yüzeyi kurulamıyor).
Uyarılar: `junction_stale`, `corridor_stale`, `junction_corridors_overlap` (örtüşme var).

### §4 Masaüstü

Kavşak penceresinde Koridor örtüşmesi: iki koridor, ana kol, ızgara aralığı; alan, iki kez sayılan yarma ve dolgu,
yüzeylerin farkı, kırpılacak km aralıkları; planda örtüşen hücreler.

## Açıklar

- Kırpmanın yazılması (tali koridorun bölgesini kesmek ya da kırpma şekli) açık; komut yalnız ölçer.
- TIN bindirmeyle kesin örtüşme hacmi açık; ızgara yaklaşıktır, aralık kullanıcınındır.
