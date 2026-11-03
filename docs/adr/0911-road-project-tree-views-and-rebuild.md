# 0911 — KentOS Yol: proje ağacı, görünüşler ve yeniden üretme

Tarih: 2026-11-03. plan/road.md Y13 (ROAD-MOD-04, ROAD-MOD-05, ROAD-MOD-06). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Proje ağacı:** Civil 3D'nin Toolspace › Prospector'ı çizimin sivil nesnelerini (yüzeyler, akslar ve altında
  boykesitleri, kesit hattı grupları, koridorlar) bir ağaçta gösterir; yeniden kurulması gereken koridorun adının
  yanında uyarı simgesi durur; sağ tık Rebuild, koleksiyonda Rebuild All, koridorda Rebuild - Automatic (işaretli
  seçenek) vardır; Rebuild All koridorları bağımlılık sırasıyla kurar
  ([To Rebuild Corridors](https://help.autodesk.com/cloudhelp/2020/ENU/Civil3D-UserGuide/files/GUID-3817AEDA-B6B6-474E-BC88-ED026C5E8B69.htm),
  [New in Corridor Workflows](https://help.autodesk.com/cloudhelp/2022/ENU/Civil3D-WhatsNew/files/GUID-7F42C4CA-9391-4B22-A4C3-3B0F94C79EB5.htm),
  3 Kasım 2026'da okundu). Kavşağın bölgelerini yeniden kurmak elle yapılan değişikliklerin üstüne yazar
  ([About Recreating Corridor Regions in Intersections](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-9C208221-1FD3-4275-A98D-0EFE17B59596.htm)).
  OpenRoads'un Explorer › OpenRoads Model sekmesi etkin modelin sivil nesnelerini aks, arazi modeli ve koridor
  kategorilerinde gösterir; kategoride Zoom, Isolate ve kuralları kilitle ya da çöz, öğede Properties ve çift tıkla aç
  ([OpenRoads Model](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20CONNECT%20Edition-v13/en/GUID-5A5A17CB-B6EC-4FA0-8462-9BB3A0971C7F.html)).
  Netpro proje ekranında projenin dosyalarını (*.KTB, *.KSE, *.KSP) gösterir; Netpro 12'de hesaplar kendiliğinden
  yeniden üretilebilir (plan/road.md ROAD-MOD-06).
- **Görünüşler:** OpenRoads'un Dynamic Sections'ı koridorun ya da yatay geometrinin taban çizgisinde bir MicroStation
  görünümünde enkesit açar; kesitler arasında kaydırılır, düşey abartması vardır; Locate Station Via Datapoint planda
  tıklanan yerin kesitini gösterir
  ([Dynamic Sections](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20Help-v19/en/dynamicsectionosd.html)).
  Synchronize View plan ve boykesit görünümlerini birlikte yürütür, imleç birincil düşey geometride gezdikçe enkesit
  görünümü en yakın kesiti gösterir
  ([Synchronize View](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20Help-v20/en/GUID-F513AF90-2FFF-4804-9DD3-6979096D381E.html)).
- **KentOS'ta bugün:** yol kaynakları projenin yol ayarındadır (`RoadSettings`): yüzeyler kaynaklarının özetiyle
  ([ADR 0900](0900-road-terrain-surface.md) §10), boykesit, kesit hattı takımı, dever ve koridor aksın sürümüyle
  ([ADR 0902](0902-road-vertical-alignment-and-profile.md)–[0905](0905-road-corridors.md)), korkuluk, menfez ve duvar
  koridorun sürümüyle ([ADR 0909](0909-road-structures-and-safety.md)); her biri kendi sorgusunda “eskidi” der.
  Çizime yazan komutlar (`road.alignments.label`, `.profiles.draw`, `.sections.draw`, `.corridors.draw`,
  `.guardrails.draw`, `.culverts.draw`, `.walls.draw`, `.junctions.draw`, `road.terrains.contours`,
  `road.routes.draw`) düz nesneler yazar; nereden geldikleri kaydedilmez: kaynak değişince çizimin eskidiği
  söylenemez, yeniden çizmek eskisini elle silmeyi ister.

## Karar

1. **Yol çizimlerinin kaydı (`RoadSettings.drawings`).** Çizime yazan dokuz yol komutu (`road.alignments.label`,
   `.profiles.draw`, `.sections.draw`, `.corridors.draw`, `.guardrails.draw`, `.culverts.draw`, `.walls.draw`,
   `.junctions.draw`, `road.terrains.contours`), yazdığı adımda projenin yol ayarına bir çizim satırı ekler: kimlik
   (`cz-1` …; dever ve duvarın `d-` kimlikleriyle karışmasın), komut, komutun girdisi (ana kaynak adla verilse de
   kimliğiyle, beklenen revizyon olmadan), kaynakları ve yazdığı nesnelerin kalıcı kimlikleri sırasıyla, nesnelerin
   özeti. Ön güzergâh (`road.routes.draw`) bir aramanın sonucudur, kaçınılan katmanları kaynak olarak tutulamaz;
   kaydedilmez.
   - *Kaynaklar:* çizimin ana kaynağı (yüzey, aks, boykesit, kesit takımı, koridor, korkuluk, menfez, duvar, kavşak)
     ve onun dayandıklarının kapanışı: boykesit → aksı (ötelenmiş aksta tabanı da) ve yüzeyi; kesit takımı → aksı,
     boykesiti, tip kesiti, yüzeyi, deveri; koridor → aksı, boykesiti, bölgelerinin tip kesitleri, yüzeyi, deveri;
     korkuluk, menfez, duvar → koridoru ve onun kapanışı; kavşak → kollarının aksları. Her kaynak türü, kimliği ve
     özetiyle: tanımının kanonik JSON'unun damgası ([ADR 0253](0253-command-protocol.md) §2, 16 onaltılık hane);
     tanımın sayıları kayan sayı olarak yazılır (dosyadan `20` diye dönen `20.0` aynıdır); yüzeyde kaynaklarının bugünkü
     özetinin (`sources::gather`: girdisi ve yöntemi; kabul edilip edilmediği değil) ilk 16 hanesi. Sürüm numarası
     yerine tanımın kendisi: her alanın değişmesi görünür, sürüm artırmayı unutan bir yazma gözden kaçmaz.
   - *Nesnelerin özeti:* yazılan nesnelerin JSON'larının sırasıyla damgası; elle değişen çizim böyle anlaşılır.
   - Kayıt yazmanın geri alma adımındadır (`Op::RoadDrawings`); `.kcad` şema 161'in yol ayarında (birleşmedi).
     En çok 1000 çizim, çizim başına en çok 1 000 000 nesne.
2. **Proje ağacı `road.project.query` (salt okunur).** Yol projesinin kaynakları ağaç sırasıyla: yüzeyler (altında
   eş yükselti eğrileri ve ön güzergâh çizimleri), akslar (altında boykesitler, kesit takımları, deverler, kavşaklar
   ve aksın km yazıları), tip kesitler, koridorlar (altında kübajlar ve onların Brükner'leri, korkuluklar, menfezler,
   duvarlar, şevli planlar), pafta takımları, poz defteri. Çizim, ana kaynağının çocuğudur. Her satır türü, kimliği,
   adı, ebeveyni ve durumuyla, durumun nedenleri Türkçe cümlelerle:
   - `current` güncel; `new` (yüzey hiç kabul edilmedi); `stale` kaynağı değişti (tanımın kendi sorgusunun kuralı:
     aksın sürümü, koridorun sürümü; yüzeyde kaynakların özeti; çizimde kaynak özetlerinden biri farklı ya da kaynak
     yeni); `unbuildable` (yüzey kurulamıyor), `unsolved` (koridorda çözülemeyen aralık var), `approved` (onaylı
     kübaj, kaynakları aynı), `changed` (onaylı kübajın kaynakları değişti); çizimde `missing` (nesnelerinden bazıları
     çizimde yok), `edited` (nesneleri elle değişti), `gone` (ana kaynağı silindi).
   - `deep: false` yalnız tanımlarla ve yüzeylerin kaynak özetiyle sorar (hızlı, her karede); `deep: true` yüzeyleri ve
     koridorları kurar, onaylı kübajları hesaplar (`unbuildable`, `unsolved`, `changed`).
   - Sayılar: eskimiş, sorunlu ve eskimiş çizim sayıları.
3. **Yeniden üret `road.project.rebuild`.** Verilen çizimler (yoksa eskimiş ve eksik olanların hepsi) kayıtlı
   girdileriyle yeniden yazılır, tek geri alma adımı “Yeniden üret”: eski nesneleri silinir, aynı katmana yenileri
   yazılır, kaydın kaynakları, kimlikleri ve özeti yenilenir. Elle değişmiş çizim yalnız açıkça verilirse yeniden
   üretilir (Civil 3D'nin kavşak bölgelerindeki gibi elle yapılan değişiklik yitirilir; KentOS önce sorar). Ana
   kaynağı silinmiş çizim yeniden üretilmez (`drawing_source_gone`). Onaylı kübaj çizim değildir, değişmez.
   - *Kendiliğinden yeniden üret* (`RoadSettings.autoRebuild`, kapalı başlar): açıkken yol tanımını değiştiren bir
     yazma, eskittiği ve elle değişmemiş çizimleri aynı adımda yeniden üretir; elle değişmişleri ve kaynağı silinmişleri
     bırakır ve söyler.
4. **Masaüstünde Yol projesi paneli.** Yol modülünde sağ dokta proje ağacı: satırın simgesi türünün, durumu renkli
   rozetle (Civil 3D'nin uyarı simgesi gibi), nedeni ipucunda; satıra basmak kaynağı seçer ve planda gösterir, çift
   tık düzenleyicisini açar (Güzergâh, Boykesit, Enkesit, Dever, Koridor, Kübaj, Brükner, Otokorkuluk, Menfez,
   İstinat, Kavşak); çizim satırında Yeniden üret, panelin başında Hepsini yeniden üret ve Kendiliğinden yeniden üret.
5. **Bölünmüş görünüşler.** Seçili aksın boykesiti ve km'sindeki enkesit planın yanında, aynı pencerede: km kaydırıcısı
   (ya da yazılan km) üç görünüşü birlikte yürütür; planda aksa dik km çizgisi, boykesitte km'nin düşey çizgisi,
   enkesitte o km'nin kesiti (kesit takımının ya da koridorun). Boykesitin düşey abartması yalnız görünüşündür
   (ROAD-PROFILE-06).

## Aşamalar

Y13.1 (yapıldı, kayıt ve ağaç): §1 ve §2. `road.project.query` 23 ortak durum (`road_project_cases.py`; çizim
road_pay_cases.py'nin, kavşaklar ve Brükner'ler kendi başvurularının; damgalar ADR'nin kuralıyla, yüzey özeti
road_terrain_command_cases.py'nin `gather`'ıyla, çizimin nesne sayısı komutunun başvurusundan, derin raporlar
koridor, kübaj ve Brükner başvurularından; KentOS kodu yok): yedi çizim komutunun satırı kaynağının altında, aks ve tip
kesit değişince eskiyen çizim, silinen nesneyle eksik, elle değişen, kaynağı silinen, geri alınan, aynı koridorun iki
çizimi, kabul edilmiş ve eskimiş yüzey, onaylı kübaj, kendiliğinden yeniden üret, derin ağaç ve değişen onaylı kübaj,
yol projesi değil. Var olan 1297 yol durumu çizim kaydıyla değişmeden geçer (koşucular `road`'un `drawings`'ini
yalnız durum andığında karşılaştırır). Sözleşmenin kuralı birim testli (`a_drawing_record_is_judged`), `.kcad` şema
161'in `road-work.kcad` örneği çizim ve `autoRebuild` taşır, `broken/road-drawing-bad-stamp.kcad` reddedilir
(bağımsız okuyucu da). Python ve MCP. Y13.2: §3. Y13.3: §4. Y13.4: §5.

## Sonra

Pafta takımlarının yeniden üretilmesi (kâğıttaki yerleşimi kullanıcının), yeniden üretmede nesnelerin kimliklerini
rolleriyle koruma (oda iç görünüşündeki gibi, [ADR 0799](0799-derived-room-interior-elevations.md)), etkilenen km
aralıklarının planda gösterilmesi, koridorun yalnız etkilenen bölgelerinin yeniden kurulması.
