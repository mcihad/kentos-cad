# ADR 8129: Giydirme cephede panel yerine kapı ve pencere

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B9 (BIM-09'un kalanı: "giydirme
  cephenin panel yerine kapısı ve penceresi"), [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03 (IfcCurtainWall satırının
  kalanı).
- **Bağlam:** ADR [8070](8070-missing-building-elements.md) §7 (giydirme cephe: ızgara, K45; K19'da "giydirme cepheye
  açıklık konmaz, panelin yerine kapı ve pencere sonraki adımdır"), [0341](0341-openings-doors-and-windows.md) (açıklık duvarında),
  [8043](8043-elevation-derived-from-an-elevation-mark.md) §2 (kapının kesitteki çizgileri).
- **Dosya, `.kcad`, komut sürümü:** değişmedi. Açıklık bildiği alanlarla (duvarı, `at`, genişlik, alt ve üst kot) giydirme
  cephede durur; bu alanlar hücrelere oturtulur. IFC dosyasında giydirme cephenin kapısı ve penceresi onun parçasıdır.

## §1. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| buildingSMART IFC 4.3, `IfcDoor` ve `IfcWindow` | Kapı ya da pencere ya bir açıklığı doldurur (`IfcRelFillsElement`) ya da "bir eleman bileşiminin, tipik olarak bir `IfcCurtainWall`'un parçası" olur (`IfcRelAggregates`, `Decomposes`); bileşimin parçasıysa yerleşimi bileşime göredir | Giydirme cephenin kapısı ve penceresi onun parçası; açıklık elemanı yok (K301) |
| buildingSMART IFC 4.3, `IfcCurtainWall` | Bileşim olarak kurulur; bileşenleri varsa kendi 'Body' gösterimi olmamalı, gövdesi bileşenlerinin toplamıdır | K45 (EK-5'te `IfcPlate` yok, paneller giydirme cephenin gövdesi) değişmedi; IfcOpenShell'in doğrulayıcısı bunu denetlemez (ADR 8070); kapı ve pencere ayrı parça |
| Graphisoft, Archicad 27 yardımı, "Place Door- or Window-Type Panel in Curtain Wall" | Kapı ya da pencere bir paneli değiştirir; daha büyük kapı için iki panel aradaki çerçeve silinerek birleştirilir, sonra panelin türü kapı seçilir | Açıklık kapsadığı hücreleri doldurur; birden çok bölme birleşir, aradaki dikme orada yoktur (K298, K300) |
| Autodesk, Revit yardımı (2020), "Add a Door to a Curtain Wall" | Panel seçilip türü bir giydirme cephe kapısıyla değiştirilir; kapı değiştirdiği panelin boyutunu alır | Kapının eni ve yüksekliği hücrelerinindir; ızgara değişince yeniden oturur (K299) |
| PAİY, kapının en az yüksekliği (ADR 8045'in kapı denetimleri) | Kapı geçişi alçaltılamaz | Kapının üstü istenenden alçak olmaz: üstünün düştüğü sıranın kaydına kadar çıkar (K298) |

## §2. Karar

- **K298 — Hücreleri doldurur:** giydirme cephedeki kapı, pencere ya da boşluk ızgaranın hücrelerini panellerin yerine
  doldurur (`geom::curtain::fill`). Boyunca: ortasının durduğu düz parçanın, hücre ortalarını kapsadığı bölmeleri (hiçbiri
  değilse ortasının düştüğü bölme); pervazları dikmelerin yüzlerinde. Yukarı: penceredeki gibi ortaları içinde kalan
  sıralar (yoksa ortasının düştüğü sıra), alt ve üst kotu kayıtların yüzlerinde. Kapı ve alt kotu olmayan boşluk döşemeye
  iner (altında kayıt yok, alt kotu döşeme) ve üstünün düştüğü sıraya kadar çıkar: üstü istenenden alçak olmaz, bir kayıt
  çizgisindeyse orada durur. Örnek: 1,50 m'lik sıralarda 2,10 m'lik kapı 3,00'daki kaydın altına (2,975) çıkar.
- **K299 — Oturtma:** `cad.openings.create` ve `cad.openings.edit` açıklığı yazmadan önce oturtur (önizleme de oturmuş
  hâliyle); her adımın sonunda açıklıklar duvarlarıyla yerleşirken giydirme cephedekiler yeniden oturur: ızgara, çizgi ya
  da yükseklik değişince kapı yerinde kalır ve oradaki hücreleri alır. Niş ve baca giydirme cepheye konmaz (`not_a_wall`).
  Ayrı bir veri alanı yoktur: açıklığın `at`, genişlik, alt ve üst kotu hücrelerinindir.
- **K300 — Kesit, görünüş, plan, 3B:** kesit modelinde hücrelerin paneli, açıklığın içinden geçen kayıtlar (kapının alt
  kaydı da) ve içindeki dikmenin o parçası yoktur (dikme altında ve üstünde iki parça); kapının ve pencerenin çizgileri
  (çerçevesi, açılma çizgileri) duvardaki kapınınki gibi yüzlerde çizilir. Planda cam çizgisi pervazlarda durur, içindeki
  dikme çizilmez, yüzler duvardaki gibi açılır; kapının kanadı ve yayı dikmelerin yüzleri arasındadır.
- **K301 — IFC:** giydirme cephenin kapısı `IfcDoor`, penceresi `IfcWindow`; giydirme cephenin `IfcRelAggregates`'ında
  dikme ve kayıtlarla birlikte parçası, kendi katına ayrıca konmaz; `IfcOpeningElement` ve doldurma ilişkisi yazılmaz.
  Boyutları (`OverallHeight`, `OverallWidth`), `Pset_DoorCommon`/`Pset_WindowCommon.IsExternal` (giydirme cepheninki),
  yangın ve kaçış beyanları duvardaki kapınınki gibi. Giydirme cephedeki boşluk boş bir hücredir, kendi nesnesi yazılmaz.

## §3. Uygulama

- `crates/shared/geometry-core/src/geom/curtain.rs` (`Fill`, `fill`), `geom/wall.rs` (`Drawn.openings`, giydirme cephe
  planı), `geom/wall_join.rs` (geçen açıklıkların aralıkları).
- `crates/native/domain/src/opening_cells.rs` (`Document::fitted_to_cells`, `is_curtain_wall`), `opening_follow.rs`
  (adım sonunda yeniden oturtma).
- `crates/native/application/src/arch/openings.rs` (`placed`: oturtma ve niş/baca reddi; eski "Giydirme cepheye açıklık
  konmaz" reddi kalktı), `sections.rs` (`curtain`, `opening_lines`, `opening_look`).
- `crates/shared/ifc/src/arch.rs` (giydirme cephe kapısı ve penceresi parça).

## §4. Sınamalar

- **Çekirdek** (`curtain::an_opening_fills_the_cells_it_covers`, elle): 6 m'lik, 1,50/1,20'lik, 3 m'lik duvarda 0,90'lık
  kapı 1,525–2,975'e, üstü 2,10 → 2,95 (kayıt çizgisinde 2,00 → 1,975); 2 m'lik pencere iki bölmeye (1,525–4,475) ve orta
  sıraya; küçük pencere ortasının düştüğü hücreye; L'nin ikinci kolunda 7,525–8,95; duvar dışında yok.
  `wall::a_curtain_wall_s_door_stops_its_glass_and_takes_its_mullion_out`: iki bölmelik kapıda cam iki parça, 5 dikmeden 4'ü.
- **Uygulama** (`sections::a_curtain_wall_s_door_and_window_fill_its_cells`): kapı (2,25'te, 1,45 en, 2,95), pencere
  (4,4875'te, 2,925 en, 1,025–1,975); 12 panelden 7'si, 16 kayıttan 13'ü, dikmeler 6 parça (4,50'deki pencerenin altında ve
  üstünde); kapının çizgileri MM-KAPI-GENEL'de; niş `not_a_wall`; IFC'de kapı ve pencere giydirme cephenin parçası, 19
  eleman, açıklık elemanı yok, gövdesi 7 panel; ızgara 1,00'a inince kapı 2,50'de 0,95 enle yeniden oturur.
- **Ortak durumlar** (`fixtures/commands/v1/cad.openings.create.json`, `scripts/fixtures/opening_command_cases.py`, KentOS
  kodu olmadan): kendi çizimleriyle 6 yeni durum (kapı, kayıt çizgisindeki üst, iki bölmelik pencere, küçük boşluk,
  niş reddi, kapının planı); var olan 113 durum değişmedi, düzenleme durumları değişmedi.
- **Resimler:** `.run/gui-evidence/giydirme-kapi-8129/` (görünüş binasının doğusundaki giydirme cephede kapı ve iki
  bölmelik pencere: plan, doğu görünüşü, kuzeydoğudan 3B; açık ve koyu).
- Formats sınamalarının derlenmesi B7 ve B21'den beri bozuktu (`ArchInfo.external`, `element_type`, `WallFields.curtain`
  eksik ilklendirme); düzeltildi.

## §5. Kalanlar

- Kapı başı için ızgaradan ayrı bir kayıt yüksekliği (ızgara eşit sıralıdır; kapı yüksekliği sıra çizgileridir).
- Katın yüksekliği değişince giydirme cephedeki açıklık, duvarı ya da kendisi bir adımda değişene kadar eski hücrelerinde
  kalır (çizim ve model o anki alanlarını okur).
- Kapı ve pencere tipinin giydirme cepheye özgü çerçevesi (bugün tipin kasası çizilir).
