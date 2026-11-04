# 0922 — KentOS Yol: aksın ve boykesitin tasarım denetimi

Tarih: 2026-11-04. plan/road.md Y21 (ROAD-ALIGN-11, ROAD-ALIGN-15, ROAD-PROFILE-13). Durum: kabul, açıklar aşağıda.

## Dayanak

- **Civil 3D:** tasarım denetimleri çizgi, kurp, klotoid ve teğet kesişimi (bir kurbun klotoidleriyle grubu) için
  yazılır; ifadeler doğru ya da yanlış döner; denetimler bir küme olarak aksa uygulanır, ihlaller aksın görünüşünde
  uyarıyla işaretlenir ([About Alignment Design Checks](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-A35495A0-3C0B-468B-A632-67E75861755E.htm),
  [Working with Design Checks](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Tutorials/files/GUID-18AF0EFE-37DA-4B04-B663-7BC6B4A5CCF3.htm),
  4 Kasım 2026'da okundu).
- **OpenRoads Designer:** sivil geometrinin tasarım standartlarında en kısa ve en uzun aliyman ve kırık denetimi
  (Insure Tangency) vardır ([Civil Geometry Design Standards](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/405971/GUID-361B45F6-7BCA-857E-D2C9-53B81FC259E3.html)).
- **Bentley Civil Designer:** ardışık kurp çiftlerini tek, ters, kırık sırtlı (aynı yöne dönen, arada kısa aliyman)
  ve birleşik kurp olarak sınıflar ve aradaki aliymanı denetler.
- **Netcad:** NETPRO'nun AASHTO hız–yarıçap ile kurp çakışmasını kaldırması plan/road.md §3'te; çevrimiçi resmî
  denetim belgesi bulunamadı.
- Kurumların değerleri birbirinden ayrıdır (aliyman en az 300 m'den 15V'ye, ters kurpta yarıçap oranı 1,5:1 gibi
  örnekler eyalet kılavuzlarında); değerler tasarım standardınındır (ROAD-DEC-01), KentOS sayı vermez.

## Karar

### §1 Aks

`kentos_road::alignment::checks`. Ardışık iki kurp bir çifttir: ters (öbür yöne dönen) ya da aynı yöne. Aradaki
aliyman ST'den TS'ye ölçülür. 1 mm'den (`MEET`) kısaysa çift bitişiktir. Kurallar (hepsi isteğe bağlı, verilmeyen
denetlenmez):

- `leastRadius`: kurbun yarıçapı.
- `leastArc`: daire kısmının boyu (SC–CS).
- `leastSpiral`: her klotoidin boyu.
- `leastTangent`: iki kurp arasındaki aliyman. Bitişik çift ve aksın uçları sayılmaz.
- `leastReverse`: ters çiftin aliymanı. Bitişikse 0'dır.
- `leastBrokenBack`: aynı yöne dönen bitişik olmayan çiftin aliymanı.
- `mostRadiusRatio`: bitişik çiftin büyük yarıçapının küçüğüne oranı.
- `kinks`: yarıçapı olmayan, yönü dönen PI. Değeri dönüş açısıdır (derece), sınırı yoktur.

`road.alignments.check` v1 yalnız okur. Cevap: kurp çiftleri (ters mi, bitişik mi, aliyman ve km'leri) ve kırılan
kurallar km sırasıyla (aynı km'de kural sırasıyla). Her bulgunun kuralı, PI'leri, km aralığı, değeri, sınırı ve
Türkçe sözü vardır. Paralel aksın kendi PI'si yoktur, denetlenmez.

### §2 Boykesit

`kentos_road::profile::checks`. Kurallar:

- `mostGrade` ve `leastGrade`: iki PVI arasındaki eğimin mutlak değeri, oran olarak.
- `leastCrestK` ve `leastSagK`: K = L/|A|, A yüzde olarak.
- `leastCurve`: düşey kurbun yatay boyu.
- `leastGradeLength`: ardışık iki kurp arasındaki düz eğim.

`road.profiles.check` v1 yalnız okur. Boykesitin aksı değiştiyse `profile_stale` uyarısı verilir.

### §3 Retler ve masaüstü

Ret: `not_a_road_project`, `unknown_alignment` ya da `unknown_road_profile`, `invalid_design_check` (hiç kural
yok, 0'dan büyük olmayan sınır, 1'den küçük oran, paralel aks). Uyarı: `design_rule_broken` (kaç yer).
Güzergâh panelinde Tasarım denetimi penceresi: Aks ya da Boykesit seçilir. Planda aksın elemanları km boyunca
çizilir (aliyman, klotoid, kurp), boykesitte de çizgi; kuralı bozan aralıklar kırmızıdır. Yanda bulgular ve kurp
çiftleri yer alır. Eğimler pencerede yüzde yazılır, komuta oran olarak gider.

## Doğrulama

21 + 21 ortak durum (`scripts/fixtures/road_design_check_cases.py`). Aksın kurpları road_alignment_cases.py'nin
40 basamaklı hesabından, boykesit road_vertical_cases.py'nin kesirlerinden alınır; KentOS kodu kullanılmaz. Durumlar:
ters çift (klotoidli), kırık sırt, teğetleri tam buluşan bitişik birleşik çift, kurpsuz kırık, paralel aks; parabol,
daire ve eski (aksı değişmiş) boykesit. Python `kentos.road`, MCP, masaüstü penceresinin testi ve resimleri
(`.run/gui-evidence/yol-y21-denetim-*`).

## Açıklar

- Hıza bağlı kurallar (tasarım hızı aralıklarıyla değişen sınır, Civil 3D'nin ifadeli denetimi) ve standart
  profilinden gelen kural kümesi (ROAD-DEC-01).
- Kurallar kümesinin projede saklanması ve aksa bağlanması (Civil 3D'nin design check set'i), ihlalin planda işareti.
- Ove klotoid ve ters kurbun deverle birlikte denetimi (ROAD-DEVER-12).
