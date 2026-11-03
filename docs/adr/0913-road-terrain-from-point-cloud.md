# 0913 — KentOS Yol: nokta bulutundan arazi

Tarih: 2026-11-03. plan/road.md Y15 (ROAD-PC-04, 09, 10, 11, 12; ROAD-TIN-14). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** Create TIN Surface from Point Cloud sihirbazı bulutun alanlarını ve sınıflarını seçer (sınıf süzgeci:
  ör. yalnız Ground), noktalar arasındaki uzaklıkla seyreltir (Distance Between Points: yüzeyin boyutunu belirgin
  küçültür) ve zemin olmayan noktalar için üç yol sunar: Planar Average (ortalama kotun üstündekileri atar, hızlı, daha
  az ayıklar), Kriging Interpolation (eğrilere enterpolasyon, üstündekileri atar, daha iyi ama yavaş) ve No Filter
  (seçilen alanların bütün noktaları; önceden sınıflanmış bulut için)
  ([Non-Ground Point Filtering Page](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-FBA7DB53-C315-4560-9A79-28D407D40A1A.htm),
  [Creating surfaces from classified data](https://www.autodesk.com/learn/ondemand/module/creating-surfaces-from-point-cloud-data-in-civil-3d/unit/6hgzIBoP8nCHjRGYQNENlV),
  3 Kasım 2026'da okundu). Büyük bulutta forumun önerisi daha seyrek almak ya da küçük yüzeyler kurup birleştirmektir
  ([Large Point Cloud Surface Creation](https://forums.autodesk.com/t5/civil-3d-forum/large-point-cloud-surface-creation/m-p/5888802/highlight/true)).
- **OpenRoads / OpenRail Designer:** Terrain from Point Cloud komutu bir süzgeçle (sınıf: zemin, alçak nokta, su) ve
  özellik tanımı ile üçgenleme seçeneğiyle arazi modelini kurar; TIN süzgeci noktaları üçgen düzlemlerine bir Z
  toleransı içinde tutar, Reinsert Points önceki geçişte atılıp şimdi tolerans içinde kalanları geri koyar, yineleme en
  çok beştir ([OpenRail Designer, Creating the Terrain Model](https://DOCS.BENTLEY.COM/LiveContent/web/OpenRail%20Designer%20Help-v15/en/GUID-DF0E2C2B-5B05-4453-A461-4F1D3358C19D.html),
  [NHDOT, Creating a Terrain from LiDAR Data](https://dot.nh.gov/doing-business-nhdot/engineers-consultants/cadd/creating-terrain-lidar-data)).
  Uygulayıcıların gözlemi: süzgeçten zemin olmayan noktalar yine de geçer; temizlemek elle yapılır.
- **Netcad:** Netpro ve Netcad'in nokta bulutu modülleri için çevrimiçi resmî kılavuz bulunamadı; Türkiye'de yaygın iş
  akışı ASPRS sınıflarıyla (2 zemin) zemin alt kümesini çıkarıp sayısal arazi modeli kurmaktır.
- **ASPRS LAS 1.4:** sınıf 2 zemin, 3–5 bitki, 6 bina, 7 alçak gürültü, 9 su, 17 köprü tabliyesi, 18 yüksek gürültü,
  19 üst yapı (overhead structure); dönüş numarası ve dönüş sayısı (son dönüş zemine en yakındır).
- **Matematik:** seyreltme bir ızgaradır: hücre anahtarı ⌊x/c⌋, ⌊y/c⌋; hücrenin temsilcisi merkezine plandaki en yakın
  nokta (eşitlikte ilk okunan: okuma sırasından bağımsız sonuç). Dik hücrede (kot yayılımı > Δz) en alçak ve en yüksek
  nokta da kalır: bordür ve şev ayağı kaybolmaz. Kırık hat erimindeki her nokta kalır. Seyreltme hatası atılan her
  noktanın kalan noktaların yüzeyine düşey farkıdır (en büyük ve karesel ortalama); yüzeyin dışında kalanlar ayrı
  sayılır. Kapsama ızgaranın alandaki hücrelerinden nokta tutanların payıdır; yoğunluk okunan zemin noktası bölü alan.

## Karar

1. **Çekirdek `kentos_road::cloud_ground` (Y15.1).** `Thinner` noktaları okundukça alır: hücrenin merkezine en yakını,
   `z_tolerance` ile dik hücrenin en alçağı ve en yükseği, `near` ile kırık hatların erimindeki her nokta; `finish`
   kalanları okuma sırasıyla, nedenleriyle sayılı verir. `coverage` alanın (ya da hücre kutusunun) hücrelerini, dolu
   olanları, payı ve yoğunluğu; `residuals` atılan noktaların yüzeye düşey farkını (en büyük, RMS, sayı, dışarıda
   kalan). Sınıf ve dönüş süzgeci çağıranındır: çekirdek yalnız seçilmiş zemin noktalarını alır.
2. **Komut (Y15.2).** `road.terrains.cloud`: bulutlar (kimlikleriyle), sınıflar, dönüş (hepsi, ilk, son, tek),
   isteğe bağlı alan (kapalı çoklu çizgi ya da alan), hücre, kot toleransı ve kırık hat erimi kullanıcınındır; KentOS
   sayı vermez. Köprü tabliyesi (17) ve üst yapı (19) zemin yüzeyine katılmaz (reddedilir, ROAD-PC-10); bitki, bina ve
   gürültü sınıfları uyarılır. Kalan noktalar bir katmana çok noktalı nesneler olarak yazılır (tek geri alma adımı),
   katman yüzeyin nokta kaynağına eklenir ya da yeni yüzey kurulur; yüzeyin tanımı bulut kaydını (bulutlar, süzgeç,
   seyreltme, okunan, kalan, kapsama, yoğunluk, hata) taşır. Seyreltilmiş yüzey analizin yüzeyidir; gösterim için
   ayrıca sadeleştirme yoktur (ROAD-TIN-14).
3. **Masaüstü (Y15.3).** Arazi sekmesinde Buluttan arazi penceresi: bulut, sınıflar ve dönüş, alan, hücre, kot
   toleransı, kırık hat erimi, hedef katman ve yüzey; okunan, kalan, kapsama, yoğunluk ve hata yazıldıkça.

Açık: zemin olmayan noktaların KentOS'ta yeniden sınıflanması (mevcut Zemin süzgeci, SMRF; PMF ve bez yöntemleri,
ROAD-PC-05, 06), sınıf değişikliklerinin kalıcı yaması (ROAD-PC-08), şerit uyumsuzluğu denetimi (ROAD-PC-07), bulutun
kaynağı değişince yüzeyin eskidiğini bilmek, milyarlarca noktada parça parça çalışma (ROAD-PC-14).

## Aşamalar

Y15.1 (yapıldı, çekirdek): §1. `crates/shared/road/src/cloud_ground.rs`, testleri `tests/all/cloud_ground.rs`: 0,5 m
kafeste 1 m hücrenin merkeze en yakın (eşitlikte ilk okunan) noktası, okuma sırası ters çevrilince merkezdeki nokta
yine kalır, 0,6 m'lik basamakta dik hücrelerin en yükseği de kalır, kırık hattın 0,3 m eriminde iki sıranın bütün
noktaları kalır, alanın ve hücre kutusunun kapsaması ve yoğunluğu, düzlemde atılan noktaların hatası sıfır, kalan
noktaların atladığı tümseğin hatası yüksekliği, değerlerin retleri. `Surface::height_walking` art arda çok noktanın
yürüyüşünü sürdürür.

Y15.2 (yapıldı, komut): §2. `road.terrains.cloud` (`crates/native/application/src/road_terrain_cloud.rs`; girdi
`TerrainsFromCloud`): bulutlar bir kez taranır (`pointcloud_areas::Scan`), sınıf, dönüş ve alan süzgecinden geçen
noktalar seyreltilir, kalanlar 10 000'lik çok noktalı nesneler olarak katmana yazılır (tek adım “Buluttan arazi”),
katman yüzeyin nokta kaynağına eklenir ya da yeni yüzey kurulur (yeni yüzey kabul edilmiş yazılır, var olanın `built`'i
değişmez: kaynağı değiştiği için eskir). Hata, atılan noktaların ikinci taramayla yeni yüzeye düşey farkıdır. Yüzeyin
tanımına `clouds[]` (`SurfaceCloudDef`; `.kcad` şema 162'de, `road-work.kcad` örneği ve
`broken/road-surface-cloud-kept-past-read.kcad`). Bir çağrı en çok 2 milyon nokta yazar. Retler
`invalid_cloud_terrain`, `cloud_class_not_ground` (17, 19), `cloud_ground_empty`; uyarı `cloud_class_above_ground`
(3–7, 18). 40 ortak durum (`scripts/fixtures/road_cloud_terrain_cases.py`: kendi LAS dosyası
`fixtures/pointcloud/v1/files/road-ground.las` laspy ile, seyreltme ve kapsama ADR'nin kuralıyla, hata kendi Bowyer–Watson
üçgenlemesine karşı, üçgen sayısı 2n − 2 − h ile denetlenir; KentOS kodu yok). İlk karşılaştırmada üçgenlemenin süper
üçgeni küçük kalıp üç dış üçgeni düşürdüğünden başvurunun hatası farklıydı; başvuru düzeldi, KentOS değişmedi. Python
`kentos.cad.road.terrains.cloud`, MCP.

Y15.3 (yapıldı, masaüstü): §3. Arazi › Yüzey'de Buluttan (`terrain.cloud`, `apps/desktop/src/terrain_cloud_window.rs`):
bulut, dönüş, hücre, kot toleransı, kırık hat erimi, ASPRS sınıfları (0–11; köprü tabliyesi ve üst yapı sunulmaz), alan
seçimden, yeni ya da projedeki yüzey, katman. Önizle komutun planını okuma kopyasında çalıştırır (bulut okunur, hiçbir
şey yazılmaz): okunan, süzgeçten geçen, kalan ve nedenleri, kapsama, yoğunluk, seyreltme hatası, yüzeyin dışında kalan;
değer değişince önizleme düşer, Uygula yalnız önizlenmişken açıktır. Testler `terrain_cloud_window_tests.rs`, resimler
`.run/gui-evidence/yol-y15-bulut-*`. Resimlerde bulunan: stilli çizimde nokta bulutunun planı alan gölgelendiricisinin
hassas sürümüyle çiziliyordu ama çizim alt parçaların (low) ikinci tamponunu bağlamıyordu; wgpu doğrulaması her bulut
resminde düşüyordu (`crates/render/wgpu/src/styled/gpu.rs`, main'den gelen hassasiyet işinin eksiği). Bulut çizimi de
alt parçaları bağlar.
