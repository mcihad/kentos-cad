# 0918 — KentOS Yol: hemzemin kavşakta tali yolun boykesitini ana yolun sırtına bağlamak

Tarih: 2026-11-04. plan/road.md Y17.5 (ROAD-JUNC-03). Durum: kabul, açıklar aşağıda.

## Dayanak

- **Civil 3D:** kavşak sihirbazının iki türü var. Birincisi Primary Road Crown Maintained: ana yolun sırtı, boykesiti ve
  kenarları olduğu gibi kalır; tali yolun boykesiti ana yolun kenarına ve kesişim noktasına uydurulur. İkincisi All
  Crowns Maintained: tali yol kesişimde ana yolun kotuna kilitlenir, kenarlar bordür dönüşlerinin boykesitleriyle
  harmanlanır. Ana yolun boykesiti hiç değiştirilmez
  ([About Creating Intersections](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-4DD6FCBD-5F3A-4C14-8233-862B872049AB.htm),
  4 Kasım 2026'da okundu). Ders kitabı ve öğretici iki ek PVI'yi anlatır: tali yolun eksen boykesitinde, ana yolun
  kaplama kenarlarında, ana yolun sırtını kavşak boyunca korumak için kilitli PVI'lar; ortadaki PVI kesişimdedir
  ([Editing the Vertical Geometry of an Intersection](https://help.autodesk.com/cloudhelp/2017/ENU/Civil3D-Tutorials/files/GUID-D156D6A4-DFFA-4CF3-BEC7-6BD112DF7DFB.htm),
  [Utah Üniversitesi çalışma kitabı](https://my.civil.utah.edu/~bartlett/CVEEN%201400/workbooks/Work%20Book%209%20-%20AutoCAD%20Civil%203D_Intersection.pdf)).
  Tali yol boykesitinin kuralları (Secondary Road Profile Rules): en büyük eğim, en büyük eğim değişimi ve eğimin
  hangi uzaklıkta uydurulacağı
  ([Secondary Road Profile Rules Dialog Box](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-7EF75EBB-F5E2-41B8-AC09-B08CB9AB2DBE.htm),
  [To Edit Side Road Profiles](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-0BAAAAE3-0BC2-4A0F-B0ED-8F4AA3D4452D.htm)).
- **OpenRoads ConceptStation:** ana yol kavşak sırtının kotunu belirler ve düşeyde oynamaz; tali yol ona göre düşeyde
  uyar ([ConceptStation Help](https://docs.bentley.com/LiveContent/web/OpenRoads%20ConceptStation%20Help-v15/en/GUID-8113B124-E254-431A-AB62-6985254720EB.html)).
- **Netcad:** NETPRO için kavşakta tali yol boykesitini bağlamayı anlatan çevrimiçi resmî belge bulunamadı.

## Karar

### §1 Komut

`road.junctions.tie` v1, tek geri alma adımı “Kavşağa bağla”. Yalnız hemzemin kavşakta. Girdi:

- `junction`; `primary`: sırtı korunan kol (0 ya da 1); `profiles`: kolların sırasıyla iki boykesit (yoksa her kolun
  aksının tek boykesiti);
- ana yolun enine eğimi ya `crossfall` (sol ve sağ, yüzde, eksenden dışa, düşen eksi) ya da `superelevation` (ana yolun
  aksındaki dever çizelgesi); biri verilir, KentOS eğim önermez;
- `maxGradeChange` (yüzde, isteğe bağlı): bağlanan PVI'larda eğim değişimi bunu aşarsa `junction_grade_change` uyarısı.

### §2 Hesap

Tali yolun ekseni ana yolun sol ve sağ kenarını iki yerde keser: kesişimin iki yanında tali yolun km'si boyunca 0,25 m
adımla taranan, ana yola izdüşen ötelemenin kenar değerini geçtiği ve kesişime en yakın aralık, ikiye bölmeyle
(1e-12 m). Üç hedef kot:

- kesişimde ana yolun boykesit kotu;
- her kenarda ana yolun o kenardaki km'deki boykesit kotu + enine eğim × kenarın genişliği (eğim o km'de).

Tali yol boykesitinde iki kenar km'si arasındaki (uçlar dahil) PVI'lar kaldırılır, yerine eğrisiz üç PVI yazılır
(`<kavşak>.a`, `.x`, `.b`, tali yolun km sırasıyla); öbür PVI'lar ve kurpları kalır. Ana yolun boykesiti değişmez.
Cevap her bağın tali ve ana km'sini, ana yoldaki ötelemesini, hedef kotu, önceki kotu ve farkı; kaldırılan PVI'ları;
bağın iki yanındaki eğimleri ve üç PVI'daki eğim değişimini (yüzde) verir.

### §3 Ret ve uyarılar

Ret sırası: `not_a_road_project`, `unknown_junction`, `invalid_junction` (okunamayan ya da ayrı seviyeli kavşak),
`unknown_road_profile`, `invalid_road_profile` ve `profile_curves_overlap` (kurulamayan ya da bağdan sonra kurulamayan boykesit),
`unknown_superelevation`, `invalid_superelevation`, `invalid_tie` (primary 0 ya da 1 değil, iki boykesit değil,
boykesit kolun aksında değil, kolun boykesiti yok ya da birden çok, `crossfall` ile `superelevation`'ın ikisi ya da
hiçbiri, dever ana yolun aksında değil, eğim sonlu değil, `maxGradeChange` eksi, tali yol ana yolun kenarını
kesmiyor, boykesit bağı örtmüyor, bağın PVI kimliği başka bir PVI'da), `invalid_revision`, `revision_conflict`.
Uyarılar: `junction_stale`, `profile_stale`, `superelevation_stale`, `junction_grade_change`.

### §4 Masaüstü

Kavşak penceresinde hemzemin kavşağın altında Boykesit bağı: sırtı korunan kol, iki boykesit, sol ve sağ enine eğim
ya da dever, en büyük eğim değişimi; bağlar ve eğimler yazdıkça okunur, Bağla yazar.

## Açıklar

- All Crowns Maintained ve bordür dönüşlerinin kenar boykesitleri açık.
- Eğimin bir uzaklıkta uydurulması (Distance Rule) yok: bağın dışındaki PVI'lar olduğu gibi kalır; yeni eğimler
  raporlanır ve uyarılır, kendiliğinden düzeltilmez.
- Bağ eğrisizdir; kesişimde düşey kurp yazılmaz.
