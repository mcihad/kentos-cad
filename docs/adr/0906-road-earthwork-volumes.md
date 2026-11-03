# ADR 0906 — Kübaj: kesit ve yüzey hacmi, kübaj kuralları, kabarma ve sıkışma, kübaj cetveli

Tarih: 2026-11-03. plan/road.md Y8 (ROAD-VOL-01…21). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** kesit hattı takımının malzemeleri üç yöntemle hesaplanır: ortalama alan (Average End Area), prizmoidal
  (iki kesitin ortasında ek kesitle) ve bileşik (kesit hatları arasında poligonlar kurulup sınırlanan hacimler). Kurp
  düzeltmesi ayrı ayardır; yüzeyler arası hacim (volume surface) iki TIN'in farkıdır.
  [About Analyzing Sectional Volumes](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-1E3A9CD9-DFD6-4455-9EB1-D8E46721A0B4.htm),
  [Montana DOT, Procedure EWK](https://ftp.mdt.mt.gov/other/webdata/external/esdc/library/Procedure-EWK.pdf).
- **OpenRoads Designer:** End Area Volumes Report kesitlerden ortalama alanla kazı, dolgu ve net verir (kurp düzeltmesi
  yok); Create Cut Fill Volumes 3B modelden üçgenlemeli hacim; ikisi arasındaki fark raporlanır.
  [End Area Volumes Report](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/353439/GUID-9410DF83-95BE-4F09-8A91-A0D88DA79E76.html),
  [FHWA ORD manual, quantities](https://highways.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13246/ord-manual20-quantities.pdf),
  [Oregon DOT, earthwork volumes](https://oregon.gov/odot/EAST/Documents/EarthworkVolumes.pdf).
- **Netcad Netpro:** kübaj yöntemleri adlı kurallardır (Yol–Zemin, Formasyon–Zemin, Yol–Bitkisel Toprak, Formasyon–
  Bitkisel Toprak, Sıyırma Kazısı); kabarma ve sıkışma katsayıları; sıyırma ve diş kazısı ayrı kesitler; Excel'e kübaj
  cetveli ([NETCAD NETPRO](https://wiki.netcad.com.tr/display/HELP/NETPRO), plan/road.md §2).
- Kabarma ve sıkışma katsayıları, bitkisel toprak kalınlığı ve diş kazısının basamak ölçüleri zemin raporundan gelir;
  KentOS öntanımlı sayı vermez (ADR 0902 §7).

## Karar

1. **Tanım (ROAD-VOL-02).** Aynı düşey datumda `d = z_tasarım − z_zemin`; dolgu `∫max(d, 0)dA`, kazı `∫max(−d, 0)dA`, plan
   alanında. Kazı ve dolgu ayrı tutulur; net yalnız bilgi içindir, kazı ya da dolgu yerine geçmez.
2. **Kesit yöntemi (ROAD-VOL-03).** Ardışık iki çözülmüş kesit arasında ortalama alan `L·(A₀ + A₁)/2`, kazı ve dolgu ayrı;
   prizmoidal `L·(A₀ + 4Aₘ + A₁)/6` gerçek orta kesitle. Bir kesiti çözülmemiş aralık hacimsiz sayılmaz, raporda “hacim
   yok” aralığıdır. Kurp düzeltmesi yoktur; rapor bunu söyler.
3. **Yüzey yöntemi (ROAD-VOL-04, 05, 07, 08).** Tasarım ve zemin üçgen ağları planda üst üste konur: her tasarım üçgeni
   değdiği her zemin üçgeniyle kırpılır (ikisi de dışbükey, ikisinde de düzlem), parça `d`'nin işaret değiştirdiği
   doğruda bölünür, her bölümün hacmi `A·(d₀ + d₁ + d₂)/3` (doğrusal `d` için kesin) işaretine göre dolguya ya da kazıya
   eklenir. Tasarımın zeminin örtmediği alanı ayrıca raporlanır, sıfır kazı ya da dolgu sayılmaz. Toplama sırası sabittir
   (tasarım üçgenlerinin, sonra ızgara hücrelerinin sırası; ROAD-VOL-13); yuvarlama yalnız raporda (ROAD-VOL-16).
4. **Kübaj kuralları (ROAD-VOL-19).** Adlı, sürümlü kural: hangi koridor, hangi iki yüzey (koridorun üst yüzeyi ya da
   formasyonu; arazi ya da bitkisel toprağın altı), hangi yöntem, hangi km aralığı. Kabarma ve sıkışma katsayıları km
   aralıklarına tablo olarak (ROAD-VOL-21); gevşek ve sıkışık hacim açık dönüşümle.
6. **Sıyırma ve diş kazısı (ROAD-VOL-20).** Bitkisel toprak km aralıklarına kalınlıkla verilir (kesitin km'sini içeren
   satır; son satırın sonu dahil). Kesitin izinde (tasarım çizgisinin açıklığı, arazinin olduğu yerde) zemin kalınlık
   kadar indirilir; kazı ve dolgu indirilmiş zemine göre hesaplanır, bitkisel toprak ayrı kalemdir: kalınlık × iz
   genişliği. Bu, Trimble'ın takeoff yolundaki “bitkisel toprağı alınmış arazi” yüzeyi ve Civil 3D'nin datum ile arazi
   karşılaştırmasıdır ([Trimble, topsoil strip/replace](https://help.fieldsystems.trimble.com/tbc/12644.htm),
   [Civil 3D quantity takeoff](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-Tutorials/files/GUID-764CEBBB-F573-40B1-9611-9C55B0D9B9CB.htm)).
   Diş kazısı: indirilmiş zeminin enine eğimi verilen yüzdeden dik olduğu yerde dolgunun altına yükseklik `h` basamak
   kazılır; testere dişinin ortalama derinliği `h/2` olduğundan kesitte alanı o genişlik × `h/2`'dir (eğimden bağımsız).
   Eşik ve basamak yüksekliği projenin yönetmeliğinden ve zemin raporundan gelir; KentOS sayı vermez. Diş kazısı ayrı
   kalemdir, aynı malzemeyle doldurulur sayılır: kazıya ve dolguya katılmaz. Yüzeyler arası yöntemde sıyırma ve diş
   kazısı yoktur (izin poligonu sonraki aşama).
5. **Cetvel ve onay (ROAD-VOL-17, 18, 21).** Kübaj cetveli km satırlarıyla (alanlar, hacimler, birikimli toplamlar),
   XLSX/ODS/CSV ADR 0223'ün yazıcısıyla. Onaylanan kübaj değişmez; kaynak değişince yeni hesap ve fark.

## Aşamalar

Y8.1 çekirdek (`kentos_road::volume`: ortalama alan, prizmoidal, iki yüzey arası hacim) ve bağımsız başvuru; Y8.2
kübaj komutları (`road.volumes.*`: koridorun kesitlerinden ve üst yüzeyiyle araziden hacim, kabarma ve sıkışma, cetvel),
Python, MCP; Y8.3 Kübaj penceresi ve cetvelin dosyaya yazılması; sıyırma ve diş kazısı, onay ve fark.

Y8.1 (yapıldı): `kentos_road::volume` (`end_areas`, `prismoidal`, `between_surfaces`); 14 yüzey durumu (kontrol örneği
`d = x − 5`: 125 m³ kazı, 125 m³ dolgu; prizma, eğik düzlem, yükseltilmiş merkez, yarı örten zemin, boşluklu zemin, kayık
iki ağ, saat yönündeki üçgenler, rastgele dört zemin) ve kesit durumları kesirlerle (`road_volume_cases.py`).

Y8.2 (yapıldı): `road.volumes.edit` (set, remove; yeniden bağlanmaz, koridor değiştirilmez; tek adım “Kübaj”) ve
`.query` (kübaj cetveli: ardışık iki kesit arası satırlar, kazı ve dolgu, aralığın ortasının katsayısı, kabarma ve
sıkışmayla düzeltilmiş birikimli toplamlar ve kütle ordinatı; ham ve düzeltilmiş toplamlar; yüzey yönteminde plan
alanları ve örtülmeyen alan; hacmi olmayan aralıklar). Prizmoidalin orta kesiti koridorun kesit kuralıyla kurulur
(`road_corridors::Sampler`); yüzey yöntemi koridorun üst yüzeyini şerit şerit (`corridor::Built::strips`) bir kez
kurulan arazi ızgarasıyla (`volume::Ground`) ölçer. Kübajın kullandığı koridor silinmez (`corridor_in_use`). Katsayı
satırı aralığın ortasını içeren satırdır (son satırın sonu dahil); katsayısız aralıkta düzeltilmiş hacim ham hacimdir
ve `volume_unfactored` uyarısı verilir. 30 + 22 ortak durum (`road_volume_command_cases.py`: yüzeyler arası hacim
`road_volume_cases.py`'nin kesirli bindirmesiyle şerit şerit); `.kcad` şema 161'in `volumes`'ı ve bozuk örneği; Python,
MCP. Yolda bulunan: koridorun şerit üçgenlemesi eşit köşegenlerde (dikdörtgen) yuvarlamaya göre farklı köşegen
seçiyordu; eşitlik artık 1 nm içinde (`corridor::SAME_DIAGONAL`, ADR 0905 §5) ve iki tarafta aynı.

Y8.3: Kübaj penceresi (`volume_window.rs`; `volume.editor`, `volume.new`, `volume.export`; Koridor sekmesinde Kübaj
paneli). Kübaj yazdıkça `road.volumes.query`'nin kuralıyla hesaplanır: kütle diyagramı (birikimli düzeltilmiş kazı ve
dolgu, kütle ordinatı, hacimsiz aralıklar), toplamlar, uyarılar ve cetvel. Kaydet `road.volumes.edit` ile yazar;
Cetveli yaz cetveli (`road_volumes::sheet`: km, uzunluk, kazı, dolgu, katsayılar, düzeltilmiş hacimler, birikimli
toplamlar, kütle ordinatı ve toplam satırı) ADR 0223'ün yazıcısıyla Excel, OpenDocument ya da CSV olarak yazar.
Sıyırma, diş kazısı, onay ve fark sonraki aşamadır (Y8.4).

Y8.4: §6. Çekirdekte `section::template::earthwork` (indirilmiş zemine göre kazı ve dolgu, bitkisel toprak, diş
kazısı); kübajın `strip` ve `bench`'i (`.kcad` şema 161), satırda ve toplamda hacimleri, cetvelde sütunları; pencerede
Sıyırma tablosu ve Diş kazısı alanları. 37 + 28 ortak durum (`road_volume_command_cases.py`: kesitin kendisi enkesit
başvurusundan, toprak işi kesirlerle). Onay ve fark Y8.5'tedir.

## Sonra

Bileşik kesit hacmi (kesitler arası poligonlar), kurp düzeltmesi, malzeme katmanları ve jeoloji, raster yöntemi, GPU ile
önizleme ve hata bütçesi (ROAD-VOL-09…14).
