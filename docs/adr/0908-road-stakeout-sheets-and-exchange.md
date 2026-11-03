# ADR 0908 — Aplikasyon cetveli, yol paftaları ve mühendislik alışverişi

Tarih: 2026-11-03. plan/road.md Y10 (ROAD-SEC-09…11; ROAD-IO-05, 06, 13, 14). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** Corridor Points raporu seçilen nokta kodlarının bir aksa göre km ve ötelemesini, Cross Section Points
  raporu her km aralığında kesitin noktalarının X, Y, Z, eksenden ötelemesini ve kodunu verir; aynı kodlar boyuna
  özellik çizgileri olur. Slope Stake raporu şevin araziyle buluştuğu noktayı verir.
  [Corridor Points Report](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-63DCE117-BE1E-4749-859D-BAFC73716D41.htm),
  [Cross Section Points Report](https://help.autodesk.com/cloudhelp/2022/ENU/Civil3D-UserGuide/files/GUID-A6227729-D501-4E92-8A86-D408C737B79C.htm),
  [Slope Stake Report](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-E890AF6E-8606-45F6-82E2-B4AC424EF57B.htm).
- **OpenRoads Designer:** Cross Section Grade Book koridorun tasarım öğelerini kesit hatlarıyla kesiştikleri yerde
  eksene göre km, öteleme ve kotla, bir türü kuzey ve doğuyla listeler (Home › Civil Analysis).
  [Cross Section Report](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/353439/GUID-22028E55-D2A7-441C-86FE-03414C5D252B.html).
- **Zemin ve ızgara:** sahada ölçülen uzunluk zemindedir; ızgara uzunluğu zemin uzunluğu × çizginin ölçeği ×
  yüksekliğin çarpanıdır (ADR 0171 §3). Aplikasyonda koordinatlar ızgarada kalır, ötelemenin zemin boyu ayrıca verilir.

## Karar

1. **Aplikasyon cetveli (ROAD-SEC-09).** `road.stakeout.query` koridorun kesitlerindeki kodlu noktaları okur, hiçbir şey
   yazmaz. Kodlar özellik çizgilerininkidir (ADR 0905): `axis`, `left.c1 … left.cN`, `left.ditchToe`,
   `left.ditchBottom`, `left.hinge`, `left.daylight` ve sağın aynıları. Bir satır: km ve km yazısı, bölge, kod, yan,
   eksenden öteleme (sağa artı, plan boyu), Y (doğu), X (kuzey), Z. km'ler koridorun kendi km'leri (aralıkta) ya da
   her `step` metre (step'in katları ve aralığın uçları, her bölgede; iki bölgenin sınırındaki km ikisinde de).
   Kesitsiz km nokta vermez, `stakeout_unsolved` uyarısıyla söylenir; koridorun kendi uyarıları da gelir.
2. **Kod süzgeci.** Tam kod ya da her iki yan için kodun sözcüğü (`daylight`). Koridorun hiçbir tip kesitinin
   vermediği kod `invalid_stakeout` ile kodların listesiyle reddedilir; hendeksiz tip kesitte `ditchToe` yoktur.
3. **Zemin boyu.** `distances: ground` ötelemeyi ayrıca zemin boyu olarak verir: eksenden noktaya çizginin ölçeği
   (uçlarda ve ortada noktanın ölçeği, Simpson) ve projenin ortalama elipsoit yüksekliğinin çarpanı (çizginin
   azimutunda Euler yarıçapı) ile bölünür; Y, X, Z ızgarada kalır. Yükseklik ya da tek ölçekli sistem yoksa
   `stakeout_no_ground`. Varsayılan `grid`; hangi kuralın kullanıldığını `method` söyler.
4. **Sınırlar.** En çok 20 000 km ve 200 000 nokta; step [0,1; 10 000] m.
5. **Pencere.** Koridor sekmesinde Aplikasyon paneli, `stakeout.table`: koridor, aralık, step, kodlar, öteleme türü;
   yazdıkça okunur, noktalar planda yanlarına göre renkli çizilir, ilk 400 satır listelenir; Cetveli yaz ADR 0223'ün
   yazıcısıyla Excel, ODS ya da CSV yazar. Çizim değişmez.

6. **LandXML 1.2 yazma (ROAD-IO-05).** `file.export.landxml` arazi yüzeylerini (`Surface/Definition surfType="TIN"`:
   noktalar ve üçgenler), akslarını (`Alignment/CoordGeom`: `Line`, `Curve` crvType arc, `Spiral` spiType clothoid;
   `StaEquation`) ve boykesitlerini (`Profile/ProfAlign`: `PVI`, `ParaCurve`, `UnsymParaCurve`, `CircCurve`) landxml.org'un
   LandXML 1.2 şemasına göre yazar. Yer “kuzey doğu”, uzunluk metre, doğrultu ve açı ondalık derece (doğrultu kuzeyden
   saat yönünde, şemanın `direction`'ı). Eğrinin merkezi iç yanında, PI teğetlerin buluştuğu yer (yarım daireden büyük
   eğride yok); klotoidin PI'sı uç teğetlerinin buluştuğu yer, teğetler paralelse orta noktası; teğet uçta yarıçap
   `INF`. Elemanların, km denklemlerinin (`staInternal`) ve PVI'ların km'leri iç km'dir (başlangıç km'si + aks
   boyunca uzaklık), şemanın tanımıyla. Paralel aks LandXML'de yoktur, yazılmaz ve `export_losses` ile söylenir;
   kurulamayan yüzey ya da aks seçilmişse ret, seçilmemişse uyarı. Dosya yanına yazılır, okuyucuyla geri okunur, sonra
   yerine konur. Koridor sekmesinin Aplikasyon panelinde LandXML ver.

## Aşamalar

Y10.1 aplikasyon cetveli (komut, başvuru, Python, MCP, pencere); Y10.2 LandXML'e aks, boykesit ve yüzey yazma ve
aks ile boykesit okuma (ROAD-IO-05); Y10.3 IFC 4.3 aks (ROAD-IO-06); Y10.4 cihaza aks ve kesit gönderme (ROAD-IO-13);
Y10.5 plan–boykesit ve enkesit paftaları (ROAD-SEC-11); Y10.6 çoklu çizgiden aks (ROAD-IO-14).

Y10.1 (yapıldı): 25 ortak durum (`road_stakeout_cases.py`; kesitler road_corridor_command_cases.py'nin başvurusundan,
zemin boyu PROJ'un nokta ölçekleri ve Euler yarıçapıyla, KentOS kodu yok), Python, MCP; pencere `stakeout_window.rs`.

Y10.2 (yapıldı, yazma): 22 ortak durum (`road_landxml_cases.py`; akslar road_alignment_cases.py'nin, yüzey Triangle'ın,
boykesitler road_profile_command_cases.py'nin başvurusundan); KentOS'un yazdığı dosya (`fixtures/road/v1/landxml/out`)
`--written` ile xmllint'te landxml.org'un XSD'sine karşı geçerli ve her yer, PI, merkez, doğrultu, yarıçap, km ve PVI
başvuruyla aynı; Python, MCP, masaüstünde LandXML ver. Aks ve boykesit okuma (LandXML al'ın genişlemesi) açık.

## Sonra

Makine kontrolü ve yapılan iş ölçüsü (ROAD-SEC-10), şev kazığı ofset işareti (hinge'den sabit uzaklık), kodların
kullanıcı adları.
