# KentOS CAD kılavuzu — §0 Güncel karar özeti

> [CLAUDE.md](../../CLAUDE.md)'nin 0. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §0…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 0. Güncel karar özeti

- **Main, yerel GUI kanıtları ve UX önceliği (27 Ekim 2026; kullanıcı kararı).**
  Mimari dal `3ad0decc7` ile main'e birleşti; çalışma main'de sürer.
  Gerçek fare/klavye ve ekran görüntüsüyle doğrulama sürer, ancak ham GUI
  görüntüleri Git'e eklenmez; yeni kayıtlar `.run/gui-evidence/` altındadır.
  Eski baseline görüntüleri yerel ignored kopya ve metin manifestiyle korunur;
  görüntü yokluğu doğrulama başarısı sayılmaz. Ortak kullanım ve geri getirme
  yolu [baseline kılavuzundadır](../../docs/baseline/README.md).
  Her yeni mimari adımda üreticinin resmî internet belgeleri araştırılır;
  kaynak ve KentOS tasarım kararı ilgili plan/ADR'ye yazılır. Görünür ayar,
  anlaşılır önizleme, az işlem, tek onay/Esc/undo ve dar ekran kullanılabilirliği
  kabul koşuludur. `plan/architecture.md` yalnız kanıtlanan alt maddelerle
  ilerletilir; bütün görevler bitmiş gibi sunulmaz.

- **Mimari ileri planın ikinci turu (7 Kasım 2026; [plan/architecture-advanced.md](../../plan/architecture-advanced.md), dal `arch-adv-ileri`).**
  - **Odak:** sahibin hedefi "çok pratik ama AutoCAD kadar ayrıntılı" mimari çizimdir.
  - **Araştırma:** üç kod denetimi, beş kabul senaryosunun güncel ikiliyle oynatılması ve dört internet araştırması
    yapıldı; notlar [docs/research/2026-11-07-mimari/](../research/2026-11-07-mimari/) altında.
  - **Plan:** yeni paketler ADV-08…26, iş sırası M1…M51 (§5), madde talimatı (§6).
  - **Numaralar:** mimari ileri ADR'leri 3000'den başlar.
  - **Katman ve mahal numarası:** 2026 Dijital Yönetmeliği'nin EK-4 ve EK-3'üne bağlanır (RG 05.08.2026/33331, yürürlük
    01.09.2027).
  - **Tek ad, tek harf (M4, [ADR 8003](../adr/8003-option-letters-and-terms.md)):**
    - İstem, araç paneli ve Öznitelikler aynı adı kullanır. Duvarın Kalınlık'ı ile çizginin Çizgi kalınlığı ayrıdır.
    - Sözleşme harfleri: E Kalınlık/Genişlik/En, G Geri, K Kapat, Y Yükseklik, S Stil, T Tür, L Kat, Ö Öteleme, D Açı.
    - Shift'li harf seçenektir; seçenek olmayan harf çalışan komutu bitirmez.
    - CAD'de imleç Açı, CBS'de Semt yazar.
    - Arayüzde kelimesi kelimesine çeviri bırakılmaz (Benimse → Var olanı kullan).
  - **Komut geçmişi (M5, [ADR 8004](../adr/8004-readable-command-history.md)):**
    - Mimari komutları satırda kimliksiz Türkçe özetle yazılır.
    - Komut, alanlar ve Python "Ayrıntı" düğmesinde.
  - **Dar şerit ve Öznitelikler (M6, [ADR 8005](../adr/8005-narrow-tabs-and-building-first.md)):**
    - 1200 px altında sığmayan sekmeler ⋯ menüsüne girer; açık, bağlam ve programın kendi sekmesi görünür kalır.
    - Yapı elemanında Öznitelikler kendi bölümüyle başlar.
  - **Düzenleyiciler (M7, [ADR 8006](../adr/8006-editors-ask-and-pick-from-drawing.md)):**
    - Kendi kaydı olan düzenleyici değişiklik varken yan tık, × ve Esc'te `kentos_ui::widget::unsaved` ile sorar.
    - Seçimle çalışan pencerede "Çizimden seç" pencereyi çeker, Enter geri getirir.
  - **Başlangıç (M8, [ADR 8007](../adr/8007-mimari-start-ground-floor-hatched-wall.md)):**
    - İşi Mimari olan yeni proje Zemin kat'la açılır (`ground_floor`: ±0,00, 3,00 m; PAİY 28).
    - KentOS Mimari'de yeni proje Mimari sekmesinde açılır; duvar araçları Dış duvar 25 ile başlar, ilk duvar poşelidir.
    - Şablon m ve 1:100'dür. Ayarsız eski `cad.templates.apply` çağrısı cm ve 1:50 yazmayı sürdürür.
  - **Katman anahtarı (M9, [ADR 8008](../adr/8008-ek4-layer-key.md)):**
    - Mimari nesne türünün EK-4 katmanına yazılır: `MM-<öğe>-<malzeme/tip/gösterim>` (`kentos_contracts::ek4`,
      `kentos_interaction::arch_layer`). Katman ilk nesneyle, aynı geri alma adımında açılır.
    - KentOS Mimari anahtarı açık başlatır; araç panelinin Katman satırı Etkin katman'a çevirir.
    - DXF'te EK-4 katmanındaki taramalar "-T" katmanına gider (4.14.5).
  - **Kalem hiyerarşisi (M10, [ADR 8009](../adr/8009-pen-hierarchy-of-parts.md)):**
    - Açıklık parçaları rollüdür (`opening::Part`); parça mutlak kalınlık değil nesne kaleminin payını taşır
      (`PieceStroke::weight_scale`).
    - Kasa ve kanat bütün kalem; yay, cam, denizlik ve duvarın malzeme sınırları yarısı (ISO 128-23 ince = kalın / 2).
  - **Kapı ve pencere ölçeğe göre (M11, [ADR 8010](../adr/8010-openings-by-scale.md)):**
    - Parçalar duvarın ayrıntı düzeyiyle çizilir (`opening::pieces_at`, `Part::shown_at`).
      - 1:200 kanat ve cam; 1:100 ve yay, pencere yüzleri; 1:50 ve 1:20 bütün parçalar.
      - Kasası çizilmeyen pencere duvar ucundan ucuna uzanır.
    - Depo açıklığı görünüşe bağlı nesnelerden sayar; DXF düz parçaları aynı görünüşü alır. Yakalama ve Patlat bütün
      sembolü kullanır.
  - **Kolon, merdiven, rampa, döşeme kenarı, mobilya ölçeğe göre (M12, [ADR 8011](../adr/8011-elements-by-scale.md)):**
    - En sade düzey `wall::View::simple` (şematik ve 1:150'den küçük); `arch::pieces_in`, `slab::pieces_in`,
      `block::outline_pieces` ona göre çizer. `arch::pieces` (önizleme, fikstür) bütündür.
    - 1:50'de kolonun ölçüsü (30/50) ve merdivenin rıht/basamağı (11 × 17.5/28) yazılır.
    - Mobilya: kütüphane kartının kategorisi "Mobilya/"; en sade düzeyde bütünü saran kapalı parçalar kalır.
  - **Kesme yüksekliği (M13, [ADR 8012](../adr/8012-plan-cut-and-overhead.md)):**
    - Pencere de yüksekliğine göre kesilir (`OpeningForm::cut_by_height`); bütünüyle kesmenin üstündeki pencere duvarı
      kesmez, kesikli çizilir. Kapı her zaman kesilir.
    - Kesme yüksekliği projenin Mimari ayarıdır (`arch.cutHeight`, `.kcad` 217; 0,30–3,00 m). Yoksa 1,10; yeni Mimari
      projesi 1,20 yazar. Okuyan her yer `ProjectSettings::plan_cut()`'ı sorar.
    - Bir üst katın döşemesi etkin katın planında halkalarıyla, kesikli ve kalemin yarısıyla çizilir
      (`level_view::shows_below`, `LevelLooks::overhead`, `slab::Look::overhead`); seçilmez, kenetlenir. Başka kattan
      görünen nesneler `Phasing::drawn_across` ile görünüşe not edilir, yoksa ekran yeniden kurulmaz.
  - **Ölçeğe göre çizgi grubu ([ADR 8013](../adr/8013-line-groups-by-scale.md)):**
    - `arch.lineGroups` (`.kcad` 218): `byScale` TS 88-23 / ISO 128-23 grubunu ölçeğe göre seçer (1:20'ye dek 0,7,
      1:50'ye dek 0,5, 1:100'e dek 0,35, ötesi 0,25); yoksa her nesne kendi kaleminde. Yeni Mimari projesi `byScale`.
    - Nesnenin kalemi 1:50'deki genişliğidir; pay `wall::View::pens`, depo modelin parçalarında (`takes_line_group`)
      `weight_scale` ile çarpar. Açıklama öğeleri ve DXF kendi kaleminde.
  - **TS 5319 taramaları ve yalıtım simgesi (M14, [ADR 8014](../adr/8014-ts-5319-hatches-and-insulation.md)):**
    - Kitaplıkta `ts` grubu (SN46, SN47, SN48, SN49, SN21, SN22, SN23, SN54, SN55, SN58, SN5).
    - Duvar katmanının deseni SN55 ise katmanın iki kenarı arasında tek zikzak çizilir (`wall::FITTED_INSULATION`);
      bağımsız taramada döşenmiş satırlar.
    - Kolon Ayrıntılı'da SN46 (`arch::CONCRETE`); kolon aracı yeni kolonu dolu yazar.
  - **Mimari şeridin yeniden düzeni (M15, [ADR 8015](../adr/8015-architecture-ribbon-rearranged.md)):**
    - Paneller Duvar · Yerleştir · Mahal · Donatı · Düzenle · Belgele · Proje; görünen düğme 37 → 27, en çok 30.
    - Büyükler Duvar ▾, Kapı, Pencere, Mahal ▾, Donatı ▾ (Mutfak, Kitaplık), Ölçü ▾; Etiket ▾ Hepsini etiketle'yi taşır.
    - Proje'de Kat ▾ (Üst kat, Alt kat, kat görünümü); `arch.settings`'in adı "Mimari ayarları" (önce "Katlar").
    - `scripts/ui/ribbon.py` yeniden tek kaynak: Harita ve Yol'un elle yaptığı eklemeler betiğe taşındı, `--check` geçer.
  - **Mimari ikon seti (M16, [ADR 8016](../adr/8016-architecture-icon-set.md)):**
    - Mimari sekmesinde iki komut aynı ikonu taşımaz; binanın kendi araçları başka bir çizim aracının ikonunu almaz
      (`arch_icons.rs`'in üç bekçisi). Yeni mimari komut kendi ikonunu `catalog/v1/additions.json` › `icons`'a çizer.
    - Çizelgeler solda listelediğinin resmi, sağda liste çizgileriyle çizilir.
    - İkon tablosu: `cargo test -p kentos-desktop --lib arch_icons::icon_table -- --ignored`.
  - **Bina nesnesinin bağlamı (M17, [ADR 8017](../adr/8017-building-context-tabs-right-click-card.md)):**
    - Sağ tıkın ilk bölümü seçimin bina sekmesinin (duvar, açıklık, mahal, döşeme, mimari nesne) Seçim ve Düzenle'den sonraki
      ilk panelidir, başlığı sekmenin adı (`App::building_tools`); kaynak şeridin kendisi.
    - Kart bina nesnesinde `Document::arch_value`'yu okur: duvar Stil, Kalınlık, Kat; açıklık Tür, Genişlik, Numara
      (başlığı biçimi); mahal Numara, Ad, Kat, Alan. Duvarın alanı yazılmaz.
  - **Aracın seçenekleri imlecin yanında (M18, [ADR 8018](../adr/8018-tool-settings-beside-the-cursor.md)):**
    - `Tool::cursor_settings` değer kutusunun sunacağı ayar anahtarlarını söyler; yazma `configure` ile, panelin yolundan.
    - Tab: Uzunluk → Açı → ayarlar; boş alan kilitlenmeden geçilir. ↓ çizimin üstünde kutuyu ilk ayarda açar (Tab kenet
      adaylarınındır). Seçenekli ayar kelimenin başıyla (so, o, sa) ya da ↑↓ ile.
  - **Geçici ölçüden kalıcı ölçü (M19, [ADR 8019](../adr/8019-temporary-distance-made-permanent.md)):**
    - Duvarın yüze olan net mesafesinde sayının kilide göre öbür yanında + (`Switch::Permanent`): tıklanınca
      `cad.dimensions.interior`'ın iki yüzü kesen çizgisiyle tek ölçü, iki ucu yüzüne bağlı, Mimari'nin ölçü katmanında.
  - **Kenet bir binanın diliyle (M20, [ADR 8020](../adr/8020-snap-names-in-a-building-s-words.md)):**
    - `snap_names::name` depodaki şekilden noktanın çizgisini adlandırır: Duvar yüzü, Çekirdek yüzü (öncelik 1'in sınırı),
      Katman sınırı, Duvar ekseni, Taban çizgisi; Açıklık ortası, Açıklık kenarı. Kenet işareti "{tür} · {ad}" yazar.
  - **Oda aracı (M21, [ADR 8021](../adr/8021-room-tool.md)):**
    - `tool.room` = `WallRectangle::room`: duvarlar, ortasından tohumlanan mahal ve etiketi bir `begin_group`'ta; reddedilen
      parça hepsini geri alır. Mahal adı açık seçimdir (`Setting::open`: liste önerir, her ad alınır).
  - **Kapanınca mahal ve canlı döngü alanı (M22, [ADR 8022](../adr/8022-room-on-closing-and-live-loop-area.md)):**
    - Duvar'ın `Memory::wall_room` (M): Kapat kapalı duvarı ve kutup noktasından tohumlanan mahali tek adımda yazar; ilk
      değeri Kapalı. Çizerken etikette "Döngü alanı" (yol üzerinden, imleçten ilk köşeye kapanan halka).
  - **Odayı kenarından büyütme (M23, [ADR 8023](../adr/8023-room-edge-grips.md)):**
    - Mahal döşemesinden seçilir (`store/pick.rs`: net alanıyla alan isabeti; kenarlar ve daha küçük alanlar önce; döşeme
      hariç). Seçili mahalin duvar yüzündeki her düz kenarında `room_grip` tutamacı: Duvarı kaydır'ın planı, kart iç
      ölçü + alan; yazılan sayı yeni iç ölçü, +/− ile fark. Sınırı çizilmiş mahalde ve kilitli duvarda tutamaç yok.
  - **Donatı araması ve duvara yaslı yerleşim (M24, [ADR 8024](../adr/8024-fixture-search-and-wall-placement.md)):**
    - Komut ara kitaplığın Islak hacim/Mobilya/Mutfak bloklarını bulur (`fixture_search`, eş anlamlılar `WORDS`); Enter
      yalnız o tanımı alır ve Blok ekle'yi başlatır. Mimari'de `block_to_wall` açık; donatı `fixture_kind` ile
      MM-TEFR-MOBILYA / VITRIFIYE / MUTFAK katmanına (`Kind::Furniture(tip)`).
  - **Donatının kullanım alanı denetimde (M25, [ADR 8025](../adr/8025-fixture-use-zones-in-the-audit.md)):**
    - `ArchAuditCheck::Fixtures` (yalnız adıyla): `arch_fixture_zone_blocked`, `arch_fixture_zone_door` (düzeltme: kapıyı
      çevir). Alan `Document::fixture_zone` ("Boşluk zarfı") ya da eski bloklarda `fixture_audit::OLDER`; izdüşüm görünüş
      durumunun gizlediğini saymaz (`hidden_by`). Panel seçili bulgunun alanını kesik kırmızı çizer.
  - **Planı tek komutla ölçülendirmek (M26, [ADR 8026](../adr/8026-dimension-the-plan-in-one-command.md)):**
    - `tool.planDimensions`: dış duvarlar duvar gövdelerinin birleşik dış çizgisinden; `around` cephe tarifi, Ön proje
      [Duvarlar, Akslar, Toplam], Uygulama + Açıklıklar (1:50'de kendiliğinden); iç kesenler `cuts_for` (her mahal %70 ve
      %30'undan, paylaşılan, ≥ 6 mm); hepsi tek adım, MM-XXXX-OLCU.
  - **EK-3 mahal numarası ve elips etiket (M27, [ADR 8027](../adr/8027-ek3-room-numbers-and-ellipse-tags.md)):**
    - Mahal numarala `ek3` (E): ilk numara `ek3_level` (Zemin/kotu 0'a en yakın K00, K01…, B01…, NNN) + bölüm (B, 3
      karakter, varsayılan 001) + 001; `clockwise` etiket noktalarının ortası çevresinde kuzeyden; tıklamadan Enter katın
      mahalleri. `TagShape::Ellipse`: yalnız ilk satır elipste (√2 eksen, `ELLIPSE_GAP` 0,2 h), KCAD şema 219.
  - **Doğrama kodları ve etiketin yeri (M28, [ADR 8028](../adr/8028-joinery-codes-and-tag-places.md)):**
    - `OpeningType.code` (KCAD 220); `cad.openingtypes.edit` `code` ve `codes` (`opening_codes::plan`: K, GK bir yanı
      odasız kapı, P, B; tipsiz açıklığa biçim+ölçü tipi). Etiket `{code}`, `{size}` (cm, 90/210). `tool.openingCodes`
      kodlar + etiketler tek adım. `tags::opening_places`: araçlar etiketin yerini ağırlıkla seçer (duvar, yay, ölçü,
      etiket, uzaklık, başka duvarın ardı) ve `at` verir; komutun varsayılan yeri ADR 0344'teki gibi.
    - `ScheduleField::Code` (Kod; araç var olan kapı-pencere çizelgesine ekler, Çizelge hazırında var). Numarala
      `byRoom` (M): `opening_codes::room_of` hizmet edilen mahal (geçiş mahalli `PASSAGES` ayrılır), -A, -B …; araç
      türe göre K, P, B önekleriyle tek adımda çalışır.
  - **Bağımsız bölümün yönetmelik alanları (M29, [ADR 8029](../adr/8029-regulation-areas-of-independent-units.md)):**
    - `Document::paiy_areas`: brüt = grubun mahallerinin `Gross` çizgileri − öbür mahallerin `Centerline` çizgileri
      (kat kat; ortak duvar-cephe köşesinin örtüşmesi yarı yarıya); net = iç yüz alanları (boşluk ve dış mahal hariç,
      sıva stilde); pay `kentos_contracts::shares` brütler üstünden. Sütunlar `PaiyNet`, `PaiyGross`, `PaiyShare`,
      `GeneralGross` (bölüm çizelgesi, KCAD 221); `Gross`'un başlığı "Net + pay".
    - Kat kat bölüm çizelgesi `TableSource::Schedule.by_level` (`unit_level_rows`, `PaiyGroup::levels`); hazır
      `SchedulePreset::AreaTable` = `tool.scheduleAreas` "Metrekare cetveli".
  - **Emsal ve TAKS (M30, [ADR 8030](../adr/8030-emsal-taks-zoning-badge-and-table.md)):**
    - `ArchSettings.zoning` (parsel, `far`, `coverage`; KCAD 222), `SpaceBrief.emsal` / `emsal_by_use` (dış mahal %30
      içinde). `Document::zoning_report`: oda oda M29 konturu; katlar alanı = dahil + %30'u aşan; sınır 0,30 × parsel ×
      emsal; taban alanı zemin kat (ad "Zemin", yoksa ±0,00'a en yakın); `levels` kat kat.
    - `zoning_badge` (durum çubuğu: sınırlı, sınırsız, ikon + "!"; sürüm başına önbellek), Öznitelikler › İmar durumu.
    - `TableSource::Zoning {}` = `arch.zoningTable` "Emsal ve TAKS tablosu" (`zoning_table`: 57/3 satırları, kat kat,
      aşım `OVER_COLOR`); `follow_tables` mahal değişince, `refresh_zoning_tables` ayar/geri alma sonrasında adımsız.
  - **Kat irtifakı (M31, [ADR 8031](../adr/8031-condominium-units-annexes-and-land-shares.md)):**
    - `UnitKind::Annex` + `annexOf`, `nature`, `landFactor`, `BuildingProgram.landTotal` (KCAD 223). `PaiyGroup`
      `annex_gross/net`, `total()` (4/ğ-4), `general()` = toplam + pay; pay tartısı toplam brüt.
    - `kentos_contracts::land_shares` (`kentos.land/1`: toplam brüt × katsayı, payda program/parsel/toplam, en büyük
      kalan); `Document::land_shares`. Sütunlar `Nature`, `AnnexNet`, `AnnexGross`, `Annexes`, `TotalGross`,
      `LandShare`; hazır `SchedulePreset::LandShares` = `tool.scheduleLandShares` (`land_share_notes`: payda, KMK 12/a
      numara, niteliksiz bölüm).
  - **Vaziyet planı (M32, [ADR 8032](../adr/8032-site-plan-parcel-setbacks-and-datum.md)):**
    - `ArchSettings.site` = `SitePlan` (parsel kimliği, `SiteEdge` tür + mesafe, `datum`, `envelope`; KCAD 224).
      `SiteEdgeKind::setback(floors)`; `Document::site_floors` (23/1-d), `site_report[_for]`, `footprint()`.
    - Yaklaşma sınırı = parsel − kenar şeritleri (`band`); `follow_site` adımda, `refresh_site` ayar/geri alma
      sonrası. `tool.siteParcel` (`parcel_tool.rs`; Harita'nın `tool.parcel`'inden ayrı); rozet/Öznitelikler/tablo.
    - Ölçüler (Ö): `draw_marks` cephe ortası hizalı ölçüler (`SiteEdgeReport::near`, `facing`), ±0,00 `Elevation` ve
      kot yazısı; `MARK_ATTR` = `Vaziyet`, yeniden çalışınca eskileri silinir; hepsi tek grupta.
  - **Mevzuat denetimi (M33, [ADR 8033](../adr/8033-regulation-rules-rooms-storeys-doors.md)):**
    - `ArchAuditCheck::Regulation` (yalnız adıyla; panel hepsini adıyla ister), `regulation_audit.rs`: `Article` (madde
      + RG; `named` bulgunun `rule`'u, `short` iletinin sonu, değişiklik tarihli), `kind_of` (ıslak hacim önce),
      `RoomKind::limits` (29/1, 29/3); kat 28/1-c (yalnız konut projesinde); kapı 39/1 (giriş türü `sides` ile).
  - **Merdiven, rampa, korkuluk, ışıklık, asansör ve yerel yönetmelik (M34, [ADR 8034](../adr/8034-regulation-stairs-shafts-elevators-local-rules.md)):**
    - `Article.source` (`Source::Paiy`/`Sivas`); `said()` + `judge()`: sıkı yerel değer bağlar (öbürü iletide), yerel
      gevşek değerin tuttuğu ölçü `regulation_conflict` (rule "yerel ↔ ulusal"). `ArchSettings.local_rules`
      (`LocalRules::Sivas`, KCAD 225); Proje ayarları › İmar durumu › Yerel imar yönetmeliği.
    - `shaft_of` (Asansör, Işıklık/Aydınlık, Hava bacası; asansör holü değil), `technical` (34/1 bodrum sayımı),
      `storeys()`; merdivenin yeri `StairIn` (durduğu mahalin bölümü). Düzeltme `Fix` → `cad.arch.edit`.
    - Denetim tablosunun sütunları sürüklenir (`ArchAuditPanel.columns`).
  - **Yangın kaçışı (M35, [ADR 8035](../adr/8035-fire-escape-routes-distances-load-exits.md)):**
    - `kentos_domain::escape::escape(doc)`: kat kat yürüme grafiği (kapı/boşluk yan noktaları 10 cm, ayırıcıda metrede
      bir geçiş, iç köşe 0,40 m), çıkışlar (Merdiven adlı ortak mahal, açık merdiven ayağı/başı, zemin ve altında dış
      kapı), sahibe göre yürüme (33/6-b); `RoomEscape` (exit: rota + tek yön + çıkış sayısı; entrance: 48/2),
      `DoorEscape` (48/4). `dwelling_nature`, `stair_name` burada.
    - `fire_audit.rs` (application): `Use` (Ek-5/B satırı: `walks`, `dead_end`, `per_unit`), `use_of`, `load_of`
      (Ek-5/A), kurallar `bykhy_*`; Mevzuat grubunun sonunda çağrılır. `ArchSettings.sprinklers` (KCAD 226), Proje
      ayarları › Yangın güvenliği.
    - `fire_audit::walks` (ölçülen yollar, sınırlarıyla) → `fire_marks::draw` (çizgi + ok başı + "tek yön 28,07 m >
      15,00 m"; `MARK_ATTR` = `Kaçış`, yeniden çizimde eskiler silinir) → `tool.escapeRoutes` (`escape_tool.rs`, Proje ▾).
  - **Otopark ve sığınak (M36, [ADR 8036](../adr/8036-parking-and-shelter-provisions.md)):**
    - `kentos_domain::room_names`: `RoomKind`, `fold`/`words`/`has`/`kind_of`, `dwelling_nature`, `stair_name` (Mevzuat
      ve kaçış aynı adları buradan okur).
    - `kentos_domain::provisions`: `parking_report`/`shelter_report` (program bölümleri + `paiy_areas` brütü; Ek-1
      `parking_use`, `dwelling_parking`; 8/a `dwelling_persons`, 7/a–c `exempt`), `parking_table`/`shelter_table`
      (`ZoningTable`). `TableSource::Parking`/`Shelter` (KCAD 227) Emsal tablosu gibi `reads_settings` ile yenilenir.
    - `provisions_audit.rs` (application): `parking_short`, `shelter_short`, `shelter_exit_count`, `shelter_height_low`;
      kaçış kurallarından sonra. Masaüstü `place_provision_table` (Metraj ▾ › Otopark hesabı, Sığınak hesabı).
  - **PDF/A ve Türk pafta takımı (M37, [ADR 8037](../adr/8037-turkish-title-block-sheet-sets-pdfa.md)):**
    - `kentos_sheet::pdf::archive` (`PdfOptions.pdfa`): XMP (`pdfaid` 2/B), bilgi ↔ XMP aynı, sRGB ICC (kodda
      üretilir, v2.1 mntr), `GTS_PDFA1`; CMYK JPEG `pdfa_cmyk_picture`.
    - `kentos_formats::pdf::pdfa::pdfa2_report`: projenin PDF okuyucusuyla PDF/A-2 maddeleri (6.1.2–6.9); yayın her
      PDF/A dosyasını teslimden önce okur (`pdfa_rule` yayını durdurur). `file.publish.sheets` `pdfa`.
    - `expr::Scope::lookup`: cevapsız (null) pafta değişkeni kitabın/çizimin aynı adlı değerine, o da yoksa yerleşik
      değere yol verir. Yerleşik `@pafta_no`, `@pafta_basligi`, `@revizyon` (pafta seti bilgisi).
    - `Document::title_facts` (domain): `@arsa_alani`, `@emsale_konu_alan`, `@yapi_insaat_alani`, `@kat_adedi`,
      `@bb_sayisi`, `@kullanim_amaci` …; `kentos_plot::inputs::drawing_variables` çizim değişkenlerinden sonra ekler
      (masaüstü `sheet_drawing_variables` önbelleği, çizimin kuşağına göre).
    - `kentos_sheet::permit` (`PARTICULARS`, `level_codes`, `paper_for`, `permit_sheets`, `FILE_NAMING`) +
      `kentos_native_application::permit_set` (künye → `set_settings`, takım → `cad.sheets.edit`) + masaüstü
      `exchange/permit_set.rs` (Mimari › Proje › Ruhsat takımı; sonra Paftaları toplu yayınla hazır ve denetlenmiş).
      Yayın adında `{@ad}` (`naming_value_missing`).
    - `kentos_interaction::calc_tables::ensure`: hesap tabloları (metrekare cetveli, emsal, kat irtifakı, otopark,
      sığınak) bulunur ya da çizimin sağına kurulur → `PermitSet.tables` → `PageKind::Tables` (`permit::shelf`, HS/MIR).
  - **Mahal listesi (M38 1. bölüm, [ADR 8038](../adr/8038-room-list-and-graphic-joinery-list.md)):**
    - `SpaceFinishes.sill`/`stair`/`railing` (KCAD 228, `needs_room_list`); `SpaceChange.base`/`sill`/`stair`/`railing`;
      Öznitelikler "Döşeme kaplaması" (eski "Zemin kaplaması") … "Küpeşte ve korkuluk".
    - `kentos_domain::room_openings`: `opening_sides`, `room_openings()` (ADR 8028'in kuralı; "K01 (2), P02");
      `room_names::passage`. Çizelge alanları `sillFinish`, `stairFinish`, `railingFinish`, `doors`, `windows`
      (`is_room_list`); kapı-pencere çizelgesi ve listenin kapı/pencere sütunları `Op::OpeningTypes`'ı izler.
    - Kat sütununa göre sıralama katların kotuna göredir (bütün çizelgeler).
    - `tag_tools::ROOM_LIST`, `room_list_sort`, `SchedulePreset::RoomList`, `tool.scheduleRoomList` (Çizelge ▾'de
      Mahal kaplama çizelgesinin yerinde; menü en çok altı).
    - Takım: `CalcTable.list` (MHL) → `PermitSet.lists` (`ListSheet`) → `Page::list` (LS/MHL/NNN, kendi kâğıdı,
      `olcek` "—", başlıksız); `permit::padded`: tablo pencereleri kâğıtta 1,5 mm pay.
  - **Doğrama listesi (M38 2. bölüm, ADR 8038 §5–§6):**
    - `OpeningType.material`, `u_value` (KCAD 229, `has_specs`); `OpeningTypeChange.material`/`u_value`; yalnız
      bunlar değişince tipin sürümü artmaz, açıklıklar değişmez. Birim `W/m2K`.
    - `joinery_audit.rs` (application): `window_u_high` (TS 825:2024, `Source::Ts825`), Mevzuat'ın sonunda.
    - `kentos_domain::joinery_list`: `JOINERY_LIST_ID` (sabit), `joinery_rows`, `joinery_list_block` (Türkiye'nin
      açılım çizgisi: menteşe köşelerinden kol tarafının ortasına), `follow_joinery` (history.rs, tablolardan sonra),
      `extent`.
    - `kentos_interaction::joinery_list_tool` (`tool.joineryList`, Etiket ▾), `calc_tables` `Kind::JoineryList`
      (`CalcTable.scale` 50) → `ListSheet.scaled` → `Page.scaled` (antet ölçeği yazar).
  - **Tadilat durumu (M39 1. bölüm, [ADR 8039](../adr/8039-renovation-graphic-language.md)):**
    - Kavramın adı "Evre" değil **Tadilat durumu**; değerler Mimarlar Odası'nın sözcükleriyle Mevcut, İptal, İlave, Geçici;
      şeritte Tadilat ▾, komutlar "Tadilat: Mevcut durum" … (dosya ve komut adları aynı: `phase`, `phaseView`).
    - Kapı ve pencerenin kendi tadilat durumu (`phase`, KCAD 230; yoksa duvarınki); Öznitelikler'de satırı.
    - Tadilat görünümünün gizlediği duvar birleşmez, gizlediği açıklık duvarı kesmez (ekran, PDF ve ekrandaki pafta;
      kendi görünümlü harita kendi kopyasından). Dayanak PAİY 4(1)(y), 4(1)(dddd), 58; Mimarlar Odası tadilat tekniği.
    - Araçlar yeni çizileni görünümün durumuyla yazar (`Context::new_arch`, `phased`): Mevcut durum ve Yıkım planında
      Mevcut, Yeni durumda İlave, Tümü'nde durumsuz; aks yazmaz; kapı duvarınkinden farklıysa kendi durumu
      (`NewOpening.phase`). Tadilat ▾ "Bundan sonra çizilenler İlave olur" der.
    - DXF: parçalar durum katmanında (`ek4_state_layer`: MM-DUVA-TUGLA-IPTAL kesikli, -ILAVE, -MEVCUT, -GECICI; taraması
      onun -T'si); görünümün gizlediği yazılmaz ve söylenir (tasarım seçenekleri gibi), duvarlar o durumdaki gibi kesilir.
      `Entity::own_phase` döşemeyi ve mahal ayırıcıyı da okur (önce okumuyordu).
    - Tadilat projesi görünümü (`PhaseView::Renovation`, KCAD 231): Mimarlar Odası'nın üç tekniği tek planda.
      - Mevcut düz kontur (duvarın düz dolgusu yok), ilave gövdesi ANSI31 taralı, iptal kesikli.
      - Kalan duvarda kapatılan açıklık taralı dolgu, açılan açıklıkta kaldırılan duvar kesikli.
      - `Entity::renovation_mark` → `Store::set_renovation`.
      - Tadilat ▾ › Tadilat lejantı (`tool.renovationLegend`, blok MM-XXXX-TADILATLEJANTI).
    - Tadilat takımı (ADR 8039 §6): Ruhsat takımı penceresinde Tadilat projesi kutusu (`PermitSet.renovation`; çizim
      tadilatsa işaretli, `permit_set::renovates`).
      - Her kat üç pafta: mevcut durum, tadilat projesi, yeni durum (EK-2 sıra 01–03).
      - Haritalar takımın adlı görünümlerine bağlı (`permit::Page::linked`, `Source::ViewLink`; "Ruhsat takımı · Mevcut
        durum" …; tarif yalnız tadilat görünümü ve ölçek; kat haritanın). Vaziyet planı tadilat projesi tekniğiyle.
      - Görünümler ayar olarak yazılır; yeniden kurulumda aynı adla yerinde yenilenir.
    - Mahal kendi durumundaki duvarlarla sınırlanır (ADR 0798 §3). Tadilatta değişip durumu boş ya da Mevcut kalan oda
      paftada olmayan duvarı oda sınırı olarak gösterir; pencere ve takım bunları adıyla söyler
      (`permit_set::rooms_to_restate`). Doğru akış: değişen oda İptal, yenisi Yeni durumda Mahal aracıyla.
    - Görünüşün tek kuralı `Entity::look_in` (ekran, pafta, PDF, DXF, kapsam): Tadilat projesinde iptal oda ve etiketi
      gizli, yıkım planında kesikli.
  - **Düşey model (M40, [ADR 8040](../adr/8040-vertical-model.md)):**
    - Duvarın yüksekliği tek kuraldan (`Document::wall_height`): kendi yüksekliği; yoksa betonarmeden betonarmeye,
      altındaki döşemenin taşıyıcı üstünden üstündeki en alçak döşemenin taşıyıcı altına, katın yüksekliğini
      geçmeden; döşeme yoksa katın yüksekliği. Metraj, değer sütunları ve iç görünüşler okur. Öznitelikler:
      "Üst döşemenin altı · 2.850" / "Katın · 3.000".
    - Proje ayarları › Mimari: bir katın yüksekliği değişince üstündeki katların kotu aynı farkla kayar (`stack`).
    - Döşeme katmanları (`SlabFields.layers`, `.kcad` şema 232): `thickness` taşıyıcının kalınlığı kalır; `over`
      (kaplama, şap, ısı/su yalıtımı) ve `under` (sıva). Döşemenin üstü bitmiş döşemedir. Öznitelikler'de dört hazır
      diziliş (`slab_build_ups`); Metraj katman katman (alanla, şap hacimle).
    - Mahalin iç yüksekliği tek kuraldan (`Document::room_height`, `.kcad` şema 233): döşemesi kendi kot farkı
      (`floorOffset`), yoksa altındaki döşemenin üstü, yoksa katın kotu; tavanı asma tavan (`suspendedCeiling`), yazılmış
      iç yükseklik, üst döşemenin sıvalı altı, yoksa katın tepesi (tavan değil, üst sınır). Öznitelikler, Metraj, değer
      sütunları, etiketler, iç görünüşler ve sığınak denetimi okur. Denetim › Mevzuat: PAİY 28/4 (2,60), 28/5 ve 28/6
      (2,20), mahal adından sınıflanır (`room_names::height_class`); kod `paiy_inside_height_below`.
    - Öznitelikler'de yükseklik satırları sayıyı hücrede, nereden geldiğini altındaki notta söyler (ADR 0687): "İç
      yükseklik 2.760 / Üst döşemenin altına kadar".
  - **Kiriş (M41 1. bölüm, [ADR 8041](../adr/8041-beams-lintels-column-wall-joins.md) §1):**
    - `Entity::Beam` (`.kcad` şema 234): ekseni, gövde genişliği, yüksekliği (döşeme içinde, TBDY 2018 7.4.1.1 b), hizası,
      kot farkı, numarası, malzemesi. Tepesi katın üstündeki döşemenin taşıyıcı üstü (yoksa katın tepesi).
    - Planda bandı kesikli ve ince (üst kat döşemesinin kalemi); yazısı kolonunki gibi: 1:200'de yok, 1:100'de numarası,
      1:50'den "K101 25/50", bandın yanında, okunacak yöne dönük. Yazı açısı derecedir (`Shape::Text.rotation`).
    - Duvar kirişin altında biter (`WallTop::Beam`, "Kirişin altına kadar"); metraj ve iç görünüşler aynı kuralı okur.
    - Mimari › Yerleştir › Kiriş: varsayılan 25/50, numara her kirişten sonra ilerler; katmanı MM-TASI-KIRIS (EK-2'de
      Kiriş öğesi yok, Taşıyıcı altında). Seçim süzgecinde Kiriş türü (listenin sonunda, eski süzgeç bitleri korunur).
  - **Kolon–duvar düğümü (M41 2. bölüm, [ADR 8041](../adr/8041-beams-lintels-column-wall-joins.md) §2):**
    - `ColumnFields.material` (`.kcad` şema 235; yoksa Betonarme). Depo ve DXF yazıcısı aynı kuralla
      (`geom::column_join`): aynı kattaki kolon duvarın çizgilerini, kapaklarını ve bantlarını keser (duvarlar
      birleştikten sonra). Kolon, kendi malzemesindeki (katlanmış ad) bir duvar katmanıyla alan paylaşıyorsa o katmanın
      görünüşünü alır, dış çizgisi katmanın içinde çizilmez: perde + kolon tek kesit (TS EN ISO 128-3, 7.2).
    - Depo duvarların kesilmemiş çizimini (`wall_whole`) ve kolonların ayak izini (`column_feet`) tutar; kolon değişince
      yeri `wall_dirty`'ye düşer. SN46 (1:20) yalnız betonarme kolonda. Etiketin `{size}`'ı kolonda "30/50", "Ø40".
  - **Lento ve hatıl (M41 3. bölüm, [ADR 8041](../adr/8041-beams-lintels-column-wall-joins.md) §3):**
    - `BeamFields.role` (kiriş, `lintel`, `bondBeam`) ve `baseOffset` (alt yüzü katın döşemesinden; kot farkıyla birlikte
      olmaz), `.kcad` şema 236. Duvar yalnız kirişin altında biter.
    - Lento açıklıktan kurulur (`arch_elements::lintel_over`): sövelerden 20 cm taşar, duvar kalınlığında, 20 cm, alt
      yüzü açıklığın başı (TBDY 2018 11.5.3'ün en azları 20 cm ve 15 cm). Kiriş aracının Tür'ü (T) Lento iken açıklığa
      tek tıklama. Katmanlar MM-LENT-GENEL, MM-HATL-GENEL.
    - Metraj › Kirişler: kirişin hacmi döşemenin altında kalan kısmı (döşemenin hacmiyle iki kez sayılmaz), lento ve
      hatılınki tamamı; toplam `beamVolume`.
  - **Kesit çıkar (M42 1. bölüm, [ADR 8042](../adr/8042-section-derived-from-a-section-mark.md)):**
    - Kesit, işaretinin bloğudur: kimliği işaretin kimliğinden türer, adı `MM-XXXX-KESITB` (EK-4 Tablo 4.4), taban
      noktası çizginin sol ucunda ±0,00. `cad.sections.edit` (derive, refresh) ve `cad.sections.query` (güncel, eski,
      çıkarılmadı, kaynağı yok); eski = yeniden çizim aynı nesneleri vermiyor.
    - Çekirdek `geom::section`: kesilen (iki yanda taban alanı olan katı; düzlemin üstündeki yüz kesilmez) ve görünen
      (bakana dönük yüzler, önlerinde bir şey yokken; kesilen düzlemin içindekini de gizler). Bağımsız başvuru 17 durum.
    - Çizim EK-4'e göre: kesilen malzeme malzeme tek çevre, `-T` katmanında taralı; görünen `-1`…`-4`; kotlar ve kat
      yükseklikleri binanın ucundan 1 m; aks balonları tek sırada; mahal adı hiçbir çizgiye değmeyen ilk yerde.
    - Durum çubuğunda "B-B kesiti eski" → tıklayınca seçilir; Öznitelikler › Durum › Yeniden çiz. Plandaki kesit çizgisi
      noktalı kesikli (EK-4 4.28).
  - **Kesit paftası ve denetimi (M42 2. bölüm, [ADR 8042](../adr/8042-section-derived-from-a-section-mark.md) §6–§7):**
    - Kesit çizimin ölçeğinde çizilir (Çizim ölçeği; 1/50 sabit değil); yerleşimi `MM-XXXX-KESIT` katmanında.
    - Ruhsat takımı her yerleştirilmiş kesite planlardan sonra paftasını kurar (`PageKind::Section`; EK-2 KS/MMC, konum
      "B-B"; kesitin ölçeğinde, sığdığı en küçük A kâğıdında); işaret paftanın penceresine bağlanır, balon "B / UP-06".
      Vaziyet ve kat planı pencereleri yalnız kesit taşıyan katmanları dondurur. Eski kesit ve ölçek farkı takım notunda.
    - Yayın: basılacak paftanın penceresinde (dondurulmamış) eski kesit varsa `section_stale` uyarısı
      (`sections_query::stale_on_sheets`).
    - Mevzuat: `paiy_sections_missing` (ikiden az çıkarılmış kesit), `paiy_section_stair_missing` (hiçbiri ortak merdiveni
      kesmiyor; `sections::cut_owners`), PAİY 57/2-a-6.
  - **Görünüş çıkar (M43 1. bölüm, [ADR 8043](../adr/8043-elevation-derived-from-an-elevation-mark.md)):**
    - Görünüş işaretinin bloğu (`MM-XXXX-GORUNUSKUZEY`), yerleşimi `MM-XXXX-GORUNUS` katmanında; komutlar kesitinkiler
      (`cad.sections.edit`/`query`), geri alma adları "Görünüş çıkar", "Görünüşü yeniden çiz". Detay işareti reddedilir.
    - Düzlem balondan, bakışa dik, modelin genişliği ±1 m. Ad cephenin yüzünün planın kuzeyine (kuzey oku, yoksa
      yukarı) göre en yakın sekiz yönden biri: "Kuzey görünüşü"; ok dönünce görünüş eskir, Yeniden çiz adı da günceller.
    - Kapı ve pencere taban alanı olmayan kendi katısı (çekirdek: taban alanı olmayan katının yalnız çizgileri); çevre,
      kasa içi, Türkiye'nin açılım çizgileri (ADR 8038 §6); `MM-KAPI-GENEL-1`, `MM-PENC-GENEL-1`. ±0,00'ın altında görünen
      kesik (MO). Mahal adı yok.
    - Durum çubuğu "Kuzey görünüşü eski" / "1 kesit, 2 görünüş eski"; Öznitelikler Tür "Görünüş · plandan çıkarılır".
      Ruhsat takımında kesitlerden sonra GR paftası, Konum KUZ/KDG/DOG/GDG/GUN/GBT/BAT/KBT.
  - **Ek yeri çizilmez (M43 2. bölüm, [ADR 8043](../adr/8043-elevation-derived-from-an-elevation-mark.md) §7):**
    - Çekirdek: prizma yüzünün kenarının iki yanında aynı düzlemde prizma yüzü varsa ek yeridir, çizilmez; iki katının
      kaplaması (`Solid.finish`: duvar katmanının proje malzemesi ya da malzeme adı) varsa ve farklıysa çizilir; kaplamasız
      katı (kiriş, kolon, döşeme) hizalı yüzünkini alır. Köşe (iki düzlem) ve işaretler her zaman çizilir. Kesitte de.
    - Görünüşün derinlik dilimi en yakın yüzden, en az 1 m'lik dilimlerle (kesit düzleminden); düz cephe bütünüyle `-1`.
  - **Tümü (M43 2. bölüm, ADR 8043 §8):** Görünüş çıkar'da T: çizilmemiş bütün görünüşler (kuzeyden saat yönünde,
    çizimin sağında yan yana), tek adım "Görünüşleri çıkar" (`sections_edit::derive_facades`); görünüş işareti yoksa önce
    dördü, katıların kuzeye dönük kutusunun her yanından 4 m öteye (`sections::facade_marks`, `FACADE_MARK_OFF`).
  - **Mevzuat: her cephenin görünüşü (M43 2. bölüm, ADR 8043 §9):** `paiy_elevations_missing` (PAİY 57/2-a-6; MO "bütün
    görünüşler çizilir"); cepheler `sections::building_facings` — katıların taban birleşiminin dış çizgisi, kenarlar kuzey
    okuna göre sekiz yöne, en uzunun beşte birinden ve 2 m'den uzun yön bir cephe; çıkarılmış görünüşün yönü
    `facing_rank`. `regulation_audit::CODES` 34.
  - **Cephe kaplaması (M43 2. bölüm, ADR 8043 §10):** görünüşte cephe 25 cm aralıkla taranır (`geom::section::seen_at`:
    en önde görünen prizma yüzü), açıklıklar atlanır; 0,25 m²'den çok görünen her kaplama (`Solid.finish` → proje
    malzemesinin adı) bir kez, kat yüksekliklerinin sağında, `MM-XXXX-ETIKET`'te; çizgisi yazıya en yakın noktasından.
  - **Çatı (M44 1. bölüm, [ADR 8044](../adr/8044-roof-and-roof-plan.md) §1–§4):**
    - `Entity::Roof`: saçak çizgisi ve her kenarın eğimi (yüzde; `null` kalkan, Revit'in "Defines slope"u); tür adı
      eğimlerden okunur (Kırma, Beşik, Tek eğimli, Düz). Saçak katının üstünde (yükseklik ya da bir üst katın tabanı;
      üstünde kat yoksa kendi tabanı) + kot farkı. `.kcad` şema 237, tür 32; `cad.roofs.create`; `MM-CATI-<kaplama>`.
    - Çekirdek `geom::roof`: yüzler eğimli düzlemlerin alt zarfı (dışbükey saçakta düz iskelet); hatlar saçak, kalkan,
      mahya, kırma, dere; içe dönen saçak henüz reddedilir. Planda her yüze ok ve `%33`, mahyaya `+4.65`.
    - Araç (Yerleştir › Çatı, `CT`): Binadan (duvarların dış yüzü + Saçak, çatı katında duvar yoksa alttaki kat) ya da
      Köşeleri çiz; ön kenar tıklanana en yakın ya da ilk kenar. Harfler: Eğim M, Saçak A, Kalınlık E, Kot farkı F,
      Kaplama P, Kat L. Mahya 5,00 m'yi aşınca önizleme PAİY 40'ı söyler. Rampa Merdiven ▾'ye alındı (30 düğme sınırı).
    - Kendi katının planında yalnız saçak kesikli (MO 14.5.3 u "Saçak … nokta nokta"), seçilir; bütünü saçağın durduğu
      üst katın planında ve Bütün katlar'da (`level_view::LevelLooks::roofs`).

- **KentOS Harita: iş yapısı ve kaynak kaydı (6 Kasım 2026; [plan/map-cad.md](../../plan/map-cad.md) MAP-01, [ADR 1200](../../docs/adr/1200-map-work-structure-and-provenance.md)).**
  Harita projesinin `settings.mapWork`'ü (`.kcad` şema 188): öğeler (kampanya, referans, çalışma, tasarım,
  denetim, teslim; katmanları ve varsa dosyası ile SHA-256 revizyonu) ve türetme kayıtları (nesneler, kaynak öğe,
  revizyon, işlem). Anlam katman profilinde (nesne katmanının ya da üstündeki grubun öğesinin), ilişki kayıttadır; yeni
  nesne türü yok. `map.work.query`/`.edit`; Harita işi paneli ve Öznitelikler'in Kaynağı bölümü yalnız Harita'da.
  Onay ofisin teslim kaydıdır, kurum onayı değildir. Komutların kaydı kendiliğinden yazması sonraki iştir.
  [ADR 1201](../../docs/adr/1201-map-work-relative-files-relink-and-contextual-tools.md): öğe dosyasının göreli yolu
  (şema 193; açılışta, `map.work.query drawing`'de, Paketle ve `cad.assets.check`'te), yeniden bağlamada aynı içeriği
  yükleyen dış başvurunun da geçmesi, `release` (Bağımsız yap), bağlamsal sekmelerde yalnız Harita'nın **Harita işi**
  paneli (Kaynağını göster, Bağımsız yap) ve panelde Seçimi türet. Bağlı GeoPackage'ın bağı geri alma dışında kaldığı için
  yeniden bağlamaya katılmaz, uyarılır.
- **Ölçeklerin özeti ve `cad.crs.factors` (6 Kasım 2026; MAP-02, [ADR 1231](../../docs/adr/1231-scale-summary-and-crs-factors.md)).**
  Durum çubuğunun ölçek menüsünde (CAD ve CBS) alt menü: çizim ölçeği, ekran, projeksiyon/yükseklik/birleşik çarpan;
  yerel projede yok. Python/MCP için salt okunur `cad.crs.factors`.
- **Topolojik düzenlemede kilitli komşuda durdur (6 Kasım 2026; MAP-07, [ADR 1261](../../docs/adr/1261-topological-editing-stops-at-a-locked-neighbour.md)).**
  `drafting.topologyStopOnLocked` (ilk değer kapalı: ADR 0160'taki ayırma); açıkken kilitli komşuyla paylaşılan sınır
  hiç yazılmaz, `shared_boundary_locked`.
- **Saha kodları ve Kodla çiz (6 Kasım 2026; MAP-04, [ADR 1360](../../docs/adr/1360-field-codes-and-kodla-ciz.md)).**
  Veri profilinin sınıflarında `codes`; yerleşik `harita-varliklari` profili; `map.features.code` (Harita + GIS).
- **GNSS baz ağı dengelemesi (6 Kasım 2026; MAP-05, [ADR 1380](../../docs/adr/1380-gnss-baseline-network-adjustment.md)).**
  `cad.survey.baselines` salt okunur: korelasyonlu ECEF bazlar, sabit/ağırlıklı kontrol, minimal datum, m0/χ²/Baarda.
- **Aday uzlaştırma (7 Kasım 2026; MAP-04, [ADR 1760](../../docs/adr/1760-asset-candidate-reconciliation.md)).**
  `map.features.reconcile`: iki kaynağın adayları yarıçapla gruplanır (uyuşan, tek, çelişen, var olan); tek kabul,
  çelişki düzeltme ister; adaylar `Eşleştirme` (8 Kasım 2026'ya dek `Uzlaştırma`) ile işaretlenir ve tekrar okunmaz; köken öznitelikte; `.kcad` değişmedi.
- **Arşivden GNSS ağı (7 Kasım 2026; MAP-03/05, [ADR 1740](../../docs/adr/1740-gnss-network-from-archive.md)).**
  `.kcad` şema 199 (`mapSurvey`'de `bz-N` baz, `bn-N` bilinen nokta, `da-N` dengeleme; `method: network`); GVX 1.0
  okuyucu; `map.survey.network` iki aşama (serbest, bilinen noktalara dayalı), BÖHHBÜY C1/C2/C3 kural paketi
  (`catalog/v1/rules/gnss-ag-bohhbuy-2018.json`); `map.survey.edit network` kabul (uyuşumsuzda, düşen testte, aynı ağda
  ret); masaüstü Ölçme › GNSS arşivi › Ağı dengele.
- **Harita'da proje ağacı (7 Kasım 2026; MAP-20, [ADR 1720](../../docs/adr/1720-map-project-tree-and-rebuild.md)).**
  `.kcad` şema 198 (Harita'nın `road.drawings`/`autoRebuild`, sıfır hattı çizimi); `road.project.*` Harita'yı kabul
  eder; sıfır hattı iki yüzeyin damgasıyla tutulur.
- **Geçirimsiz alan ve erişim (7 Kasım 2026; MAP-10, [ADR 2040](../../docs/adr/2040-impact-impervious-and-access.md)).**
  `cad.scenarios.impact` + `impervious` (kullanıcının payları) ve `access` (kullanıcının uzaklığı; ağırlık merkezinden
  çekirdeğin kesin uzaklığı); değer verilmezse ölçü yok; ret `invalid_input`.
- **Yol devamlılığı ve üst plan kapsaması (7 Kasım 2026; MAP-09, [ADR 2020](../../docs/adr/2020-zoning-road-continuity-and-upper-plan.md)).**
  `cad.zoning.check` + `outsideUpper` (alt plan üst planın dışında) ve `roadGap` (kopuk yol aksı; Yollar katmanı
  isteğe bağlı); sarkan uç kuralı bağlantı olarak kendi katmanının alanlarını alır, plan sınırları yolların katmanına konur.
- **Sınır grafiği (7 Kasım 2026; MAP-08, [ADR 2000](../../docs/adr/2000-parcel-boundary-graph.md)).** `cad.parcels.boundaries`
  salt okunur: köşeler, sınırlar (iki yanındaki parsel), komşuluk; sorunlar `missingVertex`, `splitCorner`, `overlap`;
  kimlikler geometriden türetilir, kalıcı kayıt değil; Parsel alan denetimi › Sınır grafiği….
- **Topoloji denetimi İşlem araç kutusunda (7 Kasım 2026; MAP-07, [ADR 1980](../../docs/adr/1980-topology-check-in-processing.md)).**
  İşlem aracı `topology.check` (Topolojiyi kurallarla denetle) `cad.topology.check`'in denetimini
  (`topology_check::check_on`) salt okunur çalıştırır: bulgular tabloda, nesneler seçili; komutu
  `processing.run.topology.check` additions.json'da.
- **Paralel kaydırmada ortak sınır (7 Kasım 2026; MAP-06, [ADR 1960](../../docs/adr/1960-parallel-shift-shared-boundary.md)).**
  `cad.shapes.edit` › `edgeShift` + `topology`: komşu parsel ve çizgi uçları aynı adımda kayar, toplam alan korunur,
  `neighbours` boyları söyler; kilitli komşu `neighbour_locked` / `shared_boundary_locked`; çekirdek kuralı: kabarıklığı
  değişen kenar düzenleme sonrası uçlarıyla adlandırılır (yaylı ortak kenar).
- **İş grafiği (7 Kasım 2026; MAP-20, [ADR 1940](../../docs/adr/1940-project-graph.md)).** `road.project.graph` salt
  okunur: katman → yüzey → çizim → pafta kenarları, çizimin tarifi (komut, sürüm, girdi, kaynak damgaları o zaman/şimdi);
  güncel olmayan çizimi gösteren pafta teslim edilemez; pafta zemini baskı denetimiyle ortak (`map_ground`,
  `drawing_within`); Proje ağacı › İş grafiği…; işlem sürümü ve çıktı revizyonu henüz kayıtta yok.
- **Saha paketi (7 Kasım 2026; MAP-19, [ADR 1920](../../docs/adr/1920-field-package.md)).** `cad.field.package`: tasarım
  (noktalar, çizgiler, yüzey) tek `.zip`'te; revizyon, Uygulama kontrolünün tasarım özeti, kalıcı kimlikler, EPSG,
  epok, kot sistemi, yalnız verilen sınırlar ve notlar; sayılar yuvarlanmaz; ret `invalid_field_package`; çizim değişmez.
- **Uygulama kontrolü (7 Kasım 2026; MAP-19, [ADR 1700](../../docs/adr/1700-asbuilt-check.md)).** `cad.asbuilt.check`
  salt okunur: ölçülen ↔ tasarım noktası/yüzeyi/çizgisi; sınırlar kullanıcının; `designDigest`/`designChanged`; ret
  `invalid_asbuilt_check`.
- **Paftada koordinat bilgisi (7 Kasım 2026; MAP-18, [ADR 1680](../../docs/adr/1680-sheet-coordinate-facts.md)).**
  `kentos_project::facts`; pafta `@datum`, `@elipsoit`, `@projeksiyon`, `@kot_sistemi`, `@epok`, `@ikinci_sistem`,
  `@epsg_kodu` (ayrılmış değil); baskı bulgusu `crs_text_mismatch`.
- **Dışa aktarma kaybı denetimi (7 Kasım 2026; MAP-17, [ADR 1660](../../docs/adr/1660-exchange-round-trip-check.md)).**
  `file.exchange.check` (headless): KentOS yazıcısı + okuyucusuyla gidiş-dönüş; `numeric` ve `losses` ayrı; ret
  `exchange_unreadable`; GeoParquet'in yay merkezi ilk noktaya göreli.
- **Paftada kuzey bilgisi (7 Kasım 2026; MAP-18, [ADR 1900](../../docs/adr/1900-sheet-north-facts.md)).** Pafta yazısı
  `@yakinsama`, `@manyetik_sapma` (WMM2025, paftanın tarihinde), `@grid_manyetik` okur: bağlı haritanın ya da ilk
  haritanın merkezinde, kuzey diyagramının sözleri ve toplamıyla; bilinmeyen boştur.
- **Destek matrisi (7 Kasım 2026; MAP-17, [ADR 1880](../../docs/adr/1880-exchange-support-matrix.md)).**
  `file.exchange.matrix`: açık çizimin altı biçimle gidiş-dönüşü, on özellik satırı (korunur, yaklaşık, dönüşür,
  eklenir, kaybolur); her sütun file.exchange.check'in cevabı; pencerede Tüm biçimler.
- **Hat profili (7 Kasım 2026; MAP-16, [ADR 1860](../../docs/adr/1860-pipe-run-profile.md)).** `cad.pipes.profile`: bir
  borudan akış yönünde hattın boykesiti (zemin, akar, tepe, bacalar, dokuz bant), değerler denetiminkiler, çizim yol
  boykesitinin kurallarıyla (×10, 2,5 mm); ret `invalid_pipe_run`; Boru hattı denetimi penceresinde Hat profilini çiz.
- **Boru hattı denetimi (7 Kasım 2026; MAP-16, [ADR 1640](../../docs/adr/1640-gravity-pipe-check.md)).** `cad.pipes.check`
  salt okunur: altyapı ağının boruları cazibeli hat; örtü/eğim sınırları kullanıcınındır (EK-1 okunamadı, değer yok);
  birim hatası önerilir, düzeltilmez; retler `invalid_pipe_check`, `not_a_pipe_network`.
- **Kazı ve dolgu bölgeleri (7 Kasım 2026; MAP-15, [ADR 1840](../../docs/adr/1840-cut-and-fill-zones.md)).**
  `road.terrains.cutfill`: farkın bölgeleri delikli alan (bantlı ya da kazı/dolgu), alan ve hacim öznitelikte, toplamlar
  compare'inki; çizim iki yüzeyle tutulur, `.kcad` şema 205; sıfır payı 1 nm, halka kuralı 1 µm; ret `no_cut_fill`.
- **Sıfır hattı (7 Kasım 2026; MAP-15, [ADR 1620](../../docs/adr/1620-cut-fill-zero-line.md)).** `road.terrains.zeroline`:
  iki yüzeyin kazı-dolgu sınırı, 3B çoklu çizgi, tek adım; ret `no_zero_line`.
- **Hacim yöntemi ve katsayılar (7 Kasım 2026; MAP-15, [ADR 1600](../../docs/adr/1600-surface-volume-method-and-factors.md)).**
  `road.terrains.compare`: `method` tin | grid (+`cell`), ızgarada `exact` TIN yanında; `cutFactor`/`fillFactor` →
  `adjusted`, geometrik hacim değişmez; katsayı kullanıcınındır.
- **Harita projesinde tesviye ve drenaj (7 Kasım 2026; MAP-15, [ADR 1580](../../docs/adr/1580-map-project-grading-and-drainage.md)).**
  `road.platforms.*` ve `road.drainage.*` Harita'da da; `.kcad` şema 197 harita projesinin `road.platforms`'ını yazar.
- **Yüzey düzenlemeleri (7 Kasım 2026; MAP-14, [ADR 1560](../../docs/adr/1560-terrain-surface-edits.md)).**
  `TerrainSurfaceDef.edits` (addPoint, setHeight, addEdge, removeTriangle) her kuruluşta sırayla; yapılamayan
  `left`'te `edits[k]`; `.kcad` şema 196; Arazi yüzeyleri › Düzenlemeler çizimden seçimle.
- **Noktanın görünüşü ve düzenlenişi (6 Kasım 2026, sahibinin isteği; [ADR 1540](../../docs/adr/1540-point-display-and-editing.md)).**
  Simgesiz katmanın noktası dolu nokta; Giriş'te büyük Nokta; Görünüm'de Nokta adları/kodları (`graphics.pointNames`,
  `graphics.pointCodes`); listede seçili satırın Ad/Kod'u tek tıkla; Çift noktaları ayıkla › Ayıklananlar: Katmana taşı;
  üstteki kutu “Menülerde ara”.
- **Harita projesinde arazi yüzeyi (6 Kasım 2026; MAP-14, [ADR 1520](../../docs/adr/1520-map-project-terrain-surfaces.md)).**
  `road.terrains.*` Harita'da da çalışır (`holds_surfaces`); `.kcad` şema 195 harita projesinin yalnız `road.surfaces`'ını
  yazar; eğri aralığı BÖHHBÜY m. 78(1)(c)'ye göre ölçekten başlar.
- **Tarama uyumu denetimi (6 Kasım 2026; MAP-13, [ADR 1500](../../docs/adr/1500-scan-overlap-check.md)).**
  `cad.pointclouds.overlap` (Harita + CBS, salt okunur): şeritler arası RMSDz ve KOH_Z, kural paketi
  `catalog/v1/rules/lidar-uyum.json` (şerit sınırı örnek, Türk mevzuatında yok).
- **Tarama kaydı denetimi (7 Kasım 2026; MAP-13, [ADR 1800](../../docs/adr/1800-scan-registration-check.md)).**
  `cad.pointclouds.register` (Harita + CBS, salt okunur): vokselli nokta-düzlem ICP (Low 2004); dönüşüm tarifi ve 4×4
  matris, önce/sonra RMS, güç ve yatay güç (düz zemin), güvenilir kapsam (`lidar-uyum` örnek sınırı), kontrol KOH_Z'si.
- **Pafta eş yükselti eğrileri (7 Kasım 2026; MAP-14, [ADR 1820](../../docs/adr/1820-sheet-contours-by-article-78.md)).**
  `road.terrains.sheetcontours` (Harita + Yol, tek adım): BÖHHBÜY m. 78(1)(c)–(g)/Ek-3 kural paketi
  `catalog/v1/rules/esyukselti-bohhbuy-2018.json`; dik/düz üçgen kuralı, maskeli kot yazısı, detay kesme.
- **Ortofoto doğruluk denetimi (6 Kasım 2026; MAP-12, [ADR 1480](../../docs/adr/1480-orthophoto-accuracy-check.md)).**
  `cad.orthophoto.check` (raster sekmesi, salt okunur): bağımsız denetim noktaları, BÖHHBÜY m. 91(2)/54 kural paketi
  `catalog/v1/rules/ortofoto-bohhbuy-2018.json`.
- **İki ortofotonun karşılaştırılması (7 Kasım 2026; MAP-12, [ADR 1780](../../docs/adr/1780-two-orthophotos-compared.md)).**
  `cad.orthophoto.compare` (raster sekmesi › Karşılaştır, salt okunur): aynı noktaların kayması, sistematik kayma; sınır
  paketin `mosaic`'i × iri YÖA (m. 67(6), paket sürüm 2); arazi noktalarıyla her görüntünün kendi denetimi.
- **Düzenleme dağıtım taslağı (6 Kasım 2026; MAP-11, [ADR 1460](../../docs/adr/1460-readjustment-distribution-draft.md)).**
  `cad.readjustment.distribute` (Harita + CBS, salt okunur, hep taslak): oranlar kural paketinin verisi, sayılar kesin.
- **Düzenleme kapsamı (7 Kasım 2026; MAP-11, [ADR 2060](../../docs/adr/2060-readjustment-scope-and-separation-sheet.md)).**
  `cad.readjustment.scope` (Harita + CBS, salt okunur): seçili düzenleme sınırına giren parseller, ayırma çapının
  sayıları ve Yönetmelik m. 31'e göre esas alan; belirsizde esas yok. Pencere esasları kayıtlara yazar.
- **Harita işinde içerik revizyonu ve onay damgası (8 Kasım 2026; MAP-01, [ADR 2080](../../docs/adr/2080-map-work-content-revision-and-approval-stamp.md)).**
  Dosyası olmayan öğenin revizyonu tuttuğu nesnelerin özetidir (`kentos.work-content/1`): tasarımdan türetilir, değişince
  kayıt güncel değil. Onay revizyonu damgalar (`.kcad` şema 206); sonra değişen öğe “Onay eskidi”. Yalnız Harita.
- **İçe aktarmada birim bir kez (8 Kasım 2026; MAP-02, [ADR 2100](../../docs/adr/2100-import-unit-once-heights-and-raised-blocks.md)).**
  DXF/DWG okuyucusu birimi bir kez uygular (ezdxf'le beş birimde sınandı). Kotlu bloğun yükseltilmiş ya da Z'de
  ölçeklenmiş yerleştirmesi patlatılır, kotları dosyadaki yerinde; düz sembolün Z'si notla söylenir. CAD + CBS.
- **GNSS ağı ölçme epokunda (8 Kasım 2026; MAP-03, [ADR 2120](../../docs/adr/2120-network-at-the-measurement-epoch.md)).**
  Ölçme epoku girdiden ya da bazların gözlem zamanlarından; bilinen noktalar arazi ölçümünün hız ızgarasıyla oraya
  taşınır (BÖHHBÜY 15(1)(a)), ızgarasızken ret. Harita + CBS.
- **Alan adaylarının örtüşmeyle uzlaştırılması (8 Kasım 2026; MAP-04, [ADR 2140](../../docs/adr/2140-area-candidates-by-overlap.md)).**
  Bina izi ve yeşil alan adayları kesişim/birleşim oranı 0,5'i aşınca (PASCAL VOC) bir gruptur; en güvenilir iz
  Örtüşme ile yazılır. Harita; komut Python ve MCP'de.
- **GNSS ağında Madde 15(1)(c) (8 Kasım 2026; MAP-05, [ADR 2160](../../docs/adr/2160-network-similarity-article-15-1-c.md)).**
  Üç ya da daha çok bilinen noktada verilen ve serbest koordinatlar arasında 3B benzerlik; ölçeğin t testi ve C1'in
  ±3 ppm sınırı (kural paketi sürüm 2). Harita + CBS.
- **Eğimle kot ver'de aralık sahneden (8 Kasım 2026; MAP-06, [ADR 2180](../../docs/adr/2180-grade-range-picked-on-the-drawing.md)).**
  Sahneden seç iki tıkla başlangıç ve bitiş köşesini alır; komut numarayla aynı kalır. CAD (Harita + Yol).
- **Topoloji bulguları hata katmanında (8 Kasım 2026; MAP-07, [ADR 2200](../../docs/adr/2200-topology-findings-on-an-error-layer.md)).**
  Topolojiyi kurallarla denetle istenirse bulguları yerlerinde halka nokta olarak yazar: nesnelerin kalıcı kimlikleri
  ve bulgunun anahtarıyla, tek geri alma adımı (QGIS Check validity, ArcGIS Export Topology Errors). Harita + Yol + CBS.
- **Sınır grafiği çizimde, yaylı sınırda eksik köşe (8 Kasım 2026; MAP-08, [ADR 2220](../../docs/adr/2220-boundary-graph-on-the-drawing-and-arc-corners.md)).**
  `missingVertex` yayda da aranır (çembere 1 µm, kirişin yay yanında). Çizimde göster sorun halkalarını, K ve S
  numaralarını çizimin üstünde bırakır, çizim değiştikçe yeniler; İşareti kaldır siler. Komut ADR 2000'deki gibi.
- **İmar planında yola giren karar (8 Kasım 2026; MAP-09, [ADR 2240](../../docs/adr/2240-zoning-decision-inside-road-width.md)).**
  `cad.zoning.check`'in `roadWidth`'i: genişliği yazılmış yolun şeridine (aksın iki yanında yarı genişlik) yolun
  planının yol olmayan kararı giremez (PAİY m. 9(1)(b)). Harita + CBS.
- **Senaryo ölçülerinde erişim çizimde (8 Kasım 2026; MAP-10, [ADR 2260](../../docs/adr/2260-scenario-access-on-the-drawing.md)).**
  Cevapta `reach` erişen, erişmeyen ve hedef alanları adlandırır; Erişimi çizimde göster senaryonun tarafını yeşil,
  kırmızı ve kesikli çerçeveyle bırakır, İşareti kaldır siler. Harita + CBS.
- **Ayırma çapının parçaları ve koordinatları (8 Kasım 2026; MAP-11, [ADR 2280](../../docs/adr/2280-separation-sheet-parts-and-coordinates.md)).**
  `cad.readjustment.scope` kısmen giren parselin parçalarını kanonik halkalarla verir (m. 13(2)); Ayırma çapı görünümü
  köşeleri Y, X ile listeler, Çapı çizime yaz parçaları “Ayırma çapı” katmanına tek adımda yazar. Harita + CBS.
- **Ayırma çapı paftaları (8 Kasım 2026; MAP-11, [ADR 2680](../../docs/adr/2680-separation-sheets.md)).**
  `cad.readjustment.sheets`: kısmen giren her parsel için `sys:ayirma-capi` paftası; noktalar parsel başına numaralı,
  “Ayırma çapı noktaları › Ada/Parsel” katmanında; fark kesin, girmeyen kısma kalan m. 31(2); tek adım. Harita + CBS.
- **Tarama kaydında kaba hatayı atan ağırlıklar (8 Kasım 2026; MAP-13, [ADR 2300](../../docs/adr/2300-scan-registration-robust-weights.md)).**
  `cad.pointclouds.register`'ın `robust`'u: Huber (k 1,345, ölçek 1,4826 × ortanca |artık|) ya da kırpılmış ICP
  (kullanıcının payı); cevapta ağırlık raporu; Tarama kaydı'nda Ağırlık satırı. Harita + CBS.
- **Tarama kaydında kenar ve örtülme ayıklaması (8 Kasım 2026; MAP-13, [ADR 2700](../../docs/adr/2700-scan-registration-rejections.md)).**
  `cad.pointclouds.register`'ın `reject`'i: sabit taramanın kenarındaki (PCL açı ölçütü, erişim yarıçapı) ve normalleri
  kullanıcının açısından çok ayrılan eşleşmeler her adımın dışında; cevapta sayılar; Ayıkla satırı. Harita + CBS.
- **Pafta eğrilerinde tepe ve çukur kotları (8 Kasım 2026; MAP-14, [ADR 2320](../../docs/adr/2320-sheet-summit-and-pit-heights.md)).**
  `road.terrains.sheetcontours`'un `spots`'u (m. 78(1)(ğ)): en içteki kapalı eğrinin içindeki en yüksek ya da en alçak
  köşe, detay içindeki sayılmadan, kotu dm'ye kadar (Ek-3 0004, paket sürüm 2). Şev üstü ve dibi yok. Harita.
- **Yardımcı eğrinin 5/1 mm deseni (8 Kasım 2026; MAP-14, [ADR 2720](../../docs/adr/2720-auxiliary-contour-pattern.md)).**
  `cad.entities.create`'e `linePattern` (proje çizgi tipi adıyla); kural paketinde `auxiliaryPattern` (Ek-3 0003);
  pafta eğrileri yardımcı eğriyi “Yardımcı eğri (paket)” tipiyle çizer, tip yoksa ekler. Harita + Yol.
- **Kazı ve dolgu bölgelerinin yazısı ve miktar tablosu (8 Kasım 2026; MAP-15, [ADR 2340](../../docs/adr/2340-cut-fill-zone-labels-and-quantity-table.md)).**
  `road.terrains.cutfill`'in `labels`'ı (tür ve hacim, iç noktada) ve `table`'ı (bölgeler, kazı, dolgu, net); tutulan
  çizimle birlikte yeniden çizilir. Harita + Yol.
- **Kazı ve dolgu bölgesinin en derin yeri (8 Kasım 2026; MAP-15, [ADR 2740](../../docs/adr/2740-cut-fill-zone-deepest-places.md)).**
  `road.terrains.cutfill`: her bölgenin en derin yeri ve derinliği çekirdeğin parça köşelerinden (1 nm içinde eşitse en
  batı, sonra en güney); `depths` nokta ve “derinlik/yükseklik” yazısı; Fark'ta En derin yer. Harita + Yol.
- **Başka bir altyapı ağının borularına açıklık (8 Kasım 2026; MAP-16, [ADR 2360](../../docs/adr/2360-pipe-clearance-to-another-network.md)).**
  `cad.pipes.check`'in `clearance`'ı: kesişimde düşey açıklık ve üstteki boru, kesişmeyen çiftte yatay açıklık; en az
  açıklıklar ve üst taraf kullanıcının, iç çaplarla (et kalınlığı katılmaz). Harita + Yol + CBS.
- **Hat profilinde kesişen borular (8 Kasım 2026; MAP-16, [ADR 2760](../../docs/adr/2760-crossing-pipes-in-the-pipe-profile.md)).**
  `cad.pipes.profile`'ın `crossing`'i: başka ağın hattı planda kesen boruları açıklığın kurallarıyla, `crossings`'te
  uzaklık, akar ve çapla; akarı bilinen her biri çap × abartı elips (çapsızı nokta), üstünde ad ve akar kotu; kotsuz
  boru çizilmez. Boru hattı denetimi'nde kesişen borular kutusu. Harita + Yol + CBS.
- **Dışa aktarma denetiminde bloklar, tablolar ve ilişkiler (8 Kasım 2026; MAP-17, [ADR 2380](../../docs/adr/2380-exchange-blocks-tables-relations.md)).**
  `file.exchange.check` blok tanımı ve ilişki olmadan kurulan kopyaya okur; blockChanged, tableChanged, relationLost;
  matriste Bloklar, Tablolar, İlişkiler. DXF tabloyu KentOS XDATA'sıyla taşır, başka program çizgi ve yazı görür.
- **Teslim paketinde devir dosyaları (8 Kasım 2026; MAP-17, [ADR 2780](../../docs/adr/2780-delivery-package-open-files.md)).**
  `file.package`'ın `exchange`'i: paketin nesneleri açık biçimlerde `devir/`'e, her dosya teslim edildiği hâliyle geri
  okunur; matris ve denetimler cevapta ve `paket.json`'da, Türkçesi `devir/devir-raporu.csv`'de (sözler
  `kentos_commands::exchange_words`'te ortak); Paketle'de Devir dosyaları. CAD + CBS.
- **Paftada ölçme kalite notu (8 Kasım 2026; MAP-18, [ADR 2400](../../docs/adr/2400-sheet-survey-quality-note.md)).**
  `@konum_dogrulugu`, `@yukseklik_dogrulugu`, `@ag_dengelemesi`, `@olcme_kalitesi` projenin GNSS arşivinden
  (`kentos_project::survey_facts`); arşivsiz projede baskı denetimi `survey_quality_missing` der. CAD + CBS.
- **Haritanın gösterdiği noktaların kalitesi (8 Kasım 2026; MAP-18, [ADR 2800](../../docs/adr/2800-map-scoped-survey-quality.md)).**
  Not ve değerler haritanın çerçevesinin yerdeki köşeleri içindeki kabul noktalarından (`survey_facts_where`,
  `kentos_plot::survey_scope`, `RenderInputs.map_surveys`); bağlı yazı haritasınınkini, bağsız yazı ilk haritanınkini
  okur; nokta göstermeyen harita bunu söyler. CAD + CBS.
- **Dönen kaydın kendi saha paketiyle denetlenmesi (8 Kasım 2026; MAP-19, [ADR 2420](../../docs/adr/2420-field-record-checked-with-its-package.md)).**
  `cad.asbuilt.check`'in `package`'ı: özet beklenen olur, boş sınırlar paketten, sistem/epok/kot farkı söylenir
  (dönüştürülmez). Harita + Yol + CBS.
- **Kaydın kendi paketinin tasarımıyla denetlenmesi (8 Kasım 2026; MAP-19, [ADR 2820](../../docs/adr/2820-record-checked-with-its-package-design.md)).**
  `cad.asbuilt.check`'in `packageDesign`'ı: tasarım paketin noktaları ve çizgileri, özetine geri okunarak; katmanlar
  yalnız çizimdeki değişimi söyler. Uygulama kontrolü'nde paketin tasarımıyla. Harita + Yol + CBS.
- **Çizim kaydında işlem sürümü ve çıktı revizyonu (8 Kasım 2026; MAP-20, [ADR 2840](../../docs/adr/2840-drawing-record-version-and-revision.md)).**
  `RoadDrawingDef`'in `version` ve `revision`'ı, `.kcad` şema 209; İş grafiği'nin tarifinde `made`/`revision`, masaüstünde
  Revizyon sütunu. Eski kayıtta yok, tahmin edilmez. Harita + Yol.
- **Ortofoto dikiş ölçümü dikiş hattı boyunca (8 Kasım 2026; MAP-12, [ADR 2860](../../docs/adr/2860-orthophoto-seam-along-a-drawn-line.md)).**
  `cad.orthophoto.compare`'in `seam.line`'ı: yerler hattın başından adım aralıklı, örtüşmedekiler ölçülür; `station`,
  `seam.line` (uzunluk, yerler, dışarıdakiler). Ortofoto karşılaştırması'nda Dikiş hattı. CAD + CBS.
- **Harita işinde onaydan sonra değişen nesneler (8 Kasım 2026; MAP-01, [ADR 2880](../../docs/adr/2880-map-work-objects-changed-since-approval.md)).**
  Onay dosyasız öğenin nesnelerini özetleriyle tutar (`approvedObjects`, listenin özeti damgadır; `.kcad` şema 210).
  Onayı eskimiş öğenin `changes`'i eklenen, silinen ve değişen nesneleri sayar; Harita işi'nde Değişenler satırı onları
  seçer. Yalnız Harita.
- **Ölçme çarpanları çizginin kendi yüksekliğinde (9 Kasım 2026; MAP-02, [ADR 2900](../../docs/adr/2900-survey-factors-at-each-line-s-own-height.md)).**
  `cad.crs.factors`'ta `heights`, `elevations` (kotlar projenin kotlarıyla ve geoit modeliyle elipsoit yüksekliğine;
  tahmin yok) ve `lines`. Kutupsal alım ve `cad.survey.points` her alımı uçlarının ortalama elipsoit yüksekliğinde
  indirir; pencerede h sütunu. CAD + CBS.
- **GNSS arşivi panelinde bazlar ve dengelemeler (9 Kasım 2026; MAP-03, [ADR 2920](../../docs/adr/2920-archive-panel-baselines-and-adjustments.md)).**
  Bazlar, Bilinen noktalar ve Dengelemeler grupları; satırlar noktaları seçer, düğmeler nedenle dışlar ya da geri alır;
  sonradan dışlanan bazı olan dengeleme güncel değil. Sözleşme değişmedi. Yalnız Harita.
- **Detay eşleştirmede ızgara dizini (9 Kasım 2026; MAP-04, [ADR 2940](../../docs/adr/2940-reconcile-grid-index.md)).**
  `map.features.reconcile` yalnız kutuları buluşan çiftleri dener ve var olan varlığı ızgaradan arar; cevap bütün
  çiftlerinkiyle aynı, süre doğrusal (debug, 10 000 kümede 38 s'den 0,25 s'ye). Harita.
- **Hata elipsleri çizimde (9 Kasım 2026; MAP-05, [ADR 2960](../../docs/adr/2960-network-error-ellipses-drawn.md)).**
  `map.survey.edit network`'ün `ellipses`'i: kabulde her noktanın elipsi büyütülerek, büyük ekseni meridyen
  yakınsaması çıkarılmış yönde, aynı adımda ve dayanan kaydıyla çizilir; Ağ dengelemesi'nde Hata elipsleri. Harita.
- **Eğimle kot ver'de aralık çizimde (9 Kasım 2026; MAP-06, [ADR 2980](../../docs/adr/2980-grade-range-marked-on-the-drawing.md)).**
  Pencere açıkken başlangıç ve bitiş köşeleri halkalı, aradaki yol (yayıyla) vurgulu; Sahneden seç'te bitiş beklenirken
  yalnız başlangıç. Komut değişmedi. CAD (Harita + Yol).
- **Paralel kaydır önizlemesinde birlikte kayan komşu köşeler (9 Kasım 2026; MAP-07, [ADR 3000](../../docs/adr/3000-shift-preview-marks-carried-vertices.md)).**
  Topolojik düzenleme açıkken yer değiştirecek her komşu köşe gideceği yerde halkalı, etiket sayısını söyler. Komut
  değişmedi. CAD + CBS'nin tipli katman düzenlemesi.
- **Sınır grafiği'nde satır çizimde (9 Kasım 2026; MAP-08, [ADR 3020](../../docs/adr/3020-boundary-graph-row-on-the-drawing.md)).**
  Tabloda seçilen satıra Çizimde göster gider ve onu vurgu renginde öne çıkarır (sınır eğrisi boyunca, köşe ve sorun
  çift halkayla); kimliğiyle tutulur. Komut değişmedi. Harita + CBS'nin tipli parsel katmanı.
- **Yola giren kısımlar çizimde (9 Kasım 2026; MAP-09, [ADR 3040](../../docs/adr/3040-road-width-overlaps-hatched.md)).**
  İmar planı denetiminin Seç ve göster'i yol genişliğine giren karar ve parsel kısımlarını İşareti kaldır'a dek tarar;
  özet parsel başına toplamı (birden çok yoldan) söyler. Komutun cevabı aynı. Harita + CBS.
- **Altyapıya yakınlık çizimde (9 Kasım 2026; MAP-10, [ADR 3060](../../docs/adr/3060-infrastructure-nearness-on-the-drawing.md)).**
  `cad.scenarios.impact`'in `near`'ı yakın/uzak alanları ve hatları kimlikleriyle verir; Senaryo ölçüleri'nde
  Altyapıyı çizimde göster. Yakınlıktır, kapasite ya da bağlantı hakkı değil. Harita + CBS.
- **Dış parçaların parsel numaraları (9 Kasım 2026; MAP-11, [ADR 3080](../../docs/adr/3080-separation-sheet-outside-part-numbers.md)).**
  `cad.readjustment.sheets`'in `numbers`'ı: dışarıda kalan parçalara verilmiş numaralar Kısımlar'da (m. 13(3));
  Ayırma çapı görünümünde Dış parça no kutusu. KentOS numara vermez; tescil bildirimi TKGM'nin biçimini bekler. Harita + CBS.
- **Dönüşüm uygula (9 Kasım 2026; MAP-13, [ADR 3100](../../docs/adr/3100-apply-a-transform-to-a-point-cloud.md)).**
  İşlemler aracı `pointcloud.transform`: bulut 16 sayılık afin matrisle yeni dosyaya taşınır (kaynağın ızgarası, yarım
  sıfırdan uzağa); Tarama kaydı'nda Buluta uygula kaydın matrisini getirir. Kaynak ve koordinat sistemi değişmez. Harita + CBS.
- **Şev üstü ve dibi kotları (9 Kasım 2026; MAP-14, [ADR 3120](../../docs/adr/3120-sheet-slope-top-and-bottom-heights.md)).**
  `road.terrains.sheetcontours`'un `slopes`'u: şevlerin üst ve dip çizgilerinin köşeleri, kotları yüzeyden dm'ye
  (m. 78(1)(ğ)); Pafta eğrileri'nde Şevler (ğ) satırı. Aralık ya da eşik uydurulmaz. Harita + Yol.
- **Stok hacmi (9 Kasım 2026; MAP-15, [ADR 3140](../../docs/adr/3140-stockpile-volume-over-a-boundary-base.md)).**
  `road.terrains.stockpile` (salt okunur): yığının hacmi sınırının köşelerinden çizilen tabana göre (en alçak, en yüksek,
  ortalama, eğimli düzlem, kot; Pix4D'nin tabanları); Arazi › Analiz › Stok hacmi. Geometrik hacimdir. Harita + Yol.
- **Boru kesişimleri planda (9 Kasım 2026; MAP-16, [ADR 3160](../../docs/adr/3160-pipe-crossings-on-the-drawing.md)).**
  Boru hattı denetimi'nde Kesişimleri göster: kesişimler ve yakın geçişler açıklıklarıyla, komutun bulgusunun renginde;
  İşareti kaldır'a dek kalır, çizim değişince yeniden hesaplanır. Komut değişmedi. Harita + Yol + CBS.
- **Paketin ızgaraları (9 Kasım 2026; MAP-17, [ADR 3180](../../docs/adr/3180-delivery-package-grids.md)).**
  Paketle projenin adladığı NTv2, geoit ve hız ızgaralarını cihazın kütüphanesinden `izgaralar/<SHA-256>` olarak taşır
  (dosya türü `grid`); bulunmayanı ayarın yeriyle söyler. CAD + CBS.
- **Kalite notu kırpma şekliyle (9 Kasım 2026; MAP-18, [ADR 3200](../../docs/adr/3200-sheet-survey-quality-by-clip-shape.md)).**
  Haritanın ölçme kalitesi notu çerçeve yerine haritanın kırpma şeklinin (dikdörtgen, çokgen, atlas nesnesi) gösterdiği
  noktalarla; içerilme kesin yüklemle. CAD + CBS.
- **İzlemede dönemlerden doğru ve iki eşik (10 Kasım 2026; MAP-19, [ADR 4840](../../docs/adr/4840-monitoring-trend-and-thresholds.md)).**
  İzleme dizisine `trend` (güven): üç ya da daha çok günlü dönemde noktanın doğu, kuzey ve kot doğrusu, σ'ları, Student
  t'yle anlamlı hareket (`fit`); `thresholds` (uyarı, alarm) her noktaya `state` ve `exceeded`. Eşikler kullanıcının,
  yönetmelik değeri yok. Uygulama kontrolü › İzleme › Dönem dizisi. Harita + Yol + CBS.
- **Atlas sayfasının görünüşü (10 Kasım 2026; MAP-18, [ADR 4820](../../docs/adr/4820-atlas-page-look.md)).**
  `file.export.atlas` sayfalar yapılınca her sayfanın `look`'u (çizdiğinin SHA-256'sı, yayının damgaları dışarıda) ve
  değişen sayfanın `changes[].look`'u (same: yalnız çizmediği değişti); kayıtta da. CAD + CBS.
- **Tanımlı özniteliğin boşluklu etiketinin geri adı (10 Kasım 2026; MAP-17, [ADR 4800](../../docs/adr/4800-dxf-defined-attribute-name-back.md)).**
  Etiketi adından başka yazılan ATTDEF'in KentOS verisinde `tagname`; geri okumada tanım ve eklemelerin değerleri adı alır,
  dosya kazanır (büyük harfli etiket uyar, değişen etiket ad olur ve söylenir). İstem ad kaynağı değil. CAD + CBS.
- **Dış çap, hattın dış duvarı, kanal yönetmeliklerinin kural paketi (10 Kasım 2026; MAP-16, [ADR 4780](../../docs/adr/4780-pipe-outer-diameter-own-walls-and-sewer-rules.md)).**
  `diameter.outer` (et = (dış − iç) / 2; etle uyuşmazsa `wallMismatch`); hat profilinde kendi borularının dış duvarları;
  `cad.pipes.check`'in `rules`'u `catalog/v1/rules/kanal-csb-2017.json`'dan (atıksu EK-1, yağmursuyu Çizelge 1.4):
  yazılmayan sınırlar paketten, `class`, `noRule`, `exceptionalSlope`, `longSpan`. Boru hattı denetimi › Kural. Harita +
  Yol + CBS.
- **Ayırma çapında sığmayan numara kılavuzla dışarıda (10 Kasım 2026; MAP-11, [ADR 4680](../../docs/adr/4680-separation-number-on-a-leader.md)).**
  `cad.readjustment.sheets`'in `numberLeader`'ı: numara tam boyda sığmazsa küçültülmeden, yatay, kılavuzla dışarıda
  (uç parçanın etiket yeri, iniş doğu sınırdan bir boy öte); cevapta `leader`. Düzenleme › Ayırma çapı › Sığmayan
  numara. Harita ve CBS.
- **Senaryo ölçülerinde eğim sınıfları ve bakı (10 Kasım 2026; MAP-10, [ADR 4660](../../docs/adr/4660-scenario-slope-classes-and-aspect.md)).**
  `cad.scenarios.impact`'in `slope.classes`'ı (kullanıcının sınırları, %) ve `slope.aspect`'i (sekiz yön ve Düz, ArcGIS'in
  Surface Aspect'i) ölçü satırları olarak. Senaryo ölçüleri › Eğim sınıfları (%) / bakı. Harita + CBS.
- **İmar denetiminde sembol uyuşmazlığı; hata katmanları GeoPackage'a (10 Kasım 2026; MAP-09, [ADR 4640](../../docs/adr/4640-zoning-symbol-mismatch-and-error-geopackage.md)).**
  `cad.zoning.check`'in `symbolMismatch`'i (sembol karardan, tablo ADR 4240'ın); imar ve topoloji hata katmanları İşlem'in
  Sonucun yeri ile GeoPackage tablosu olur, çizim değişmez. Harita + CBS.
- **İfrazda komşunun sınırının bölünmesi (10 Kasım 2026; MAP-08, [ADR 4620](../../docs/adr/4620-subdivision-splits-neighbour-boundary.md)).**
  `cad.parcels.record`: yeni köşe güncel komşunun düz sınırının içindeyse (1 µm) ve parçalar belgenin sınırlarıysa
  sınır bölünür, komşunun halkası parçaları alır, eski sınır kapanır; `boundariesSplit`, `neighbours`. Harita + CBS.
- **Topoloji bulgusunun Öznitelikler'den işaretlenmesi (10 Kasım 2026; MAP-07, [ADR 4600](../../docs/adr/4600-topology-findings-marked-from-attributes.md)).**
  Öznitelikler › Topoloji: Gerekçe satırı ve bulgunun durumu açılır listede (`cad.topology.exceptions`); çoklu seçimde
  bulgular duruma göre sayılır. Komut değişmedi. Harita + CBS.
- **Yüzeyden kot ver'de göreli kot ve aralık (10 Kasım 2026; MAP-06, [ADR 4580](../../docs/adr/4580-drape-relative-height-and-range.md)).**
  `cad.entities.drape`'in `offset`'i (yüzeyin kotu artı fark) ve `range`'i (tek nesnenin köşe aralığı, dış halka; kırık
  noktası yalnız aradaki kenarlarda); yeni `wrong_place`. Eğimle kot ver › Yüzeyden › Kot farkı ve Aralık. Kotlar yüzeye
  bağlı kalmaz. CAD (Harita + Yol).
- **GNSS ağının seyrek çözümü (10 Kasım 2026; MAP-05, [ADR 4560](../../docs/adr/4560-gnss-network-sparse-solution.md)).**
  `gnss::adjust`: ters Cuthill–McKee sırası, zarfında Cholesky, Takahashi ile seçili ters; 1500 bilinmeyen sınırı yerine
  zarf en çok 30 milyon girdi (19 600 istasyon 6 s, sürüm). GNSS baz ağı özeti ağın büyüklüğünü söyler. Harita + CBS.
- **Detay eşleştirmede kayıtlı kararların yeniden uygulanması (10 Kasım 2026; MAP-04, [ADR 4540](../../docs/adr/4540-reapply-recorded-reconcile-decisions.md)).**
  `map.features.reconcile`'in `reapply`'ı: yeni çıkarımın kararsız adayı aynı kaynağın yakınındaki eşleştirilmiş
  adayının notundaki kararı alır (gruplama kuralı, en yakını); çıktı `reapplied`, yeni not önceki notu anar. Detay
  eşleştirme › Önceki kararlar. Harita.
- **GNSS baz oturumlarının korelasyonları, yeni noktaların hızı, sınırın projede saklanması (11 Kasım 2026; MAP-03, [ADR 4920](../../docs/adr/4920-gnss-sessions-new-point-velocity-kept-limits.md)).**
  GVX'in SESSION/CCM_BLOCK'ları baz oturumu `bo-N` olarak saklanır (şema 240); `map.survey.network` ve `edit network`'te
  `correlations: session` oturumu tam kovaryansıyla dengeler, bazlar korelasyonlu w-testiyle sınanır (verilmezse `none`,
  söylenir); `velocities` yeni noktaların hızını arazi ölçümünün hız ızgarasından okur, kabul edilen nokta ızgaranın adı
  ve SHA-256'sıyla tutar (hız uydurulmaz); `edit tolerance` kabul farkı sınırını kaynağıyla projeye yazar, `places`
  onu okur. GNSS ağ dengelemesi › Oturum korelasyonları, Yeni noktaların hızı; GNSS arşivi › Projeye yaz. Harita + CBS.
- **Bilinen noktanın kabul farkı sınırı (10 Kasım 2026; MAP-03, [ADR 4520](../../docs/adr/4520-known-place-tolerance.md)).**
  `map.survey.query`'nin `tolerance`'ı (`places` ile): yatay ve düşey sınır; yerde `horizontal` ve `beyond`. GNSS
  arşivi › Kabul farkı sınırı (oturumluk). Harita.
- **Poligon hesabında karşılıklı başucu açıları (10 Kasım 2026; MAP-02, [ADR 4500](../../docs/adr/4500-traverse-reciprocal-zenith-angles.md)).**
  `cad.survey.points`'in `heights.back`'i: geri nişanlı kenarın kot farkı iki yönün ortalaması (k düşer), aynı iki uç
  arasındaki çiftin k'sı `refractions`'ta (Baselga vd. 2014, denklem 11). Poligon hesabı › Karşılıklı. CAD + CBS.
- **EPSG'nin kullanım alanı çokgenleri; ölçülen k (10 Kasım 2026; MAP-02, [ADR 4900](../../docs/adr/4900-crs-area-polygons-and-measured-refraction.md)).**
  Kayıttaki sistemlerin alanı EPSG çokgeniyle denetlenir (`fixtures/crs/v1/area-polygons.json`, v13.106, IOGP'nin);
  çokgeni kutusu olanlarda ve ED50 / UTM 35N'de kutu. `cad.survey.points`'te `refraction` (çiftlerin ortalaması),
  `cad.settings.set`'te `refraction`; Poligon hesabında Projeye yaz. CAD + CBS.
- **Harita işinde tipli katman profili, köşe köşe kayma ve silinenin şekli (10 Kasım 2026; MAP-01, [ADR 4880](../../docs/adr/4880-map-work-typed-layer-profile-shift-and-removed-shape.md)).**
  Öğenin veri profili (`profile`, şema 239): `map.work.edit profile` sınıfların katmanını ve alanlarını ekler, öğeye
  katar; bulgusu olan öğe onaylanmaz (`approval_profile`). `changes.objects[].shift`: köşe köşe en büyük kayma, KOH,
  kesin Hausdorff (`shape_shift`). Değişenler silinenleri de çizer, kaymaları tabloda gösterir. Yalnız Harita.
- **Onaydan sonra geometrisi değişenin onaylanan şekli (10 Kasım 2026; MAP-01, [ADR 4480](../../docs/adr/4480-map-work-approved-shape-in-the-drawing.md)).**
  `map.work.query`'nin değişen nesnesinde `approved` (geometrisi değiştiyse); Harita işi › Değişenler onaylanan şekli
  kesik çizgiyle çizimin üstünde gösterir (approved_layer.rs). Yalnız Harita.
- **Dikiş ölçümü dikiş çokgenlerinin ortak sınırı boyunca (10 Kasım 2026; MAP-12, [ADR 4460](../../docs/adr/4460-orthophoto-seam-along-seamline-polygons.md)).**
  `cad.orthophoto.compare`'in `seam.polygons`'ı: iki görüntünün mozaikteki dikiş çokgenlerinin 1 mm içindeki ortak
  sınırı boyunca yerler (`seam.shared`, her yerin `part`'ı). Ortofoto karşılaştırması › Seçili çokgenleri al. CAD + CBS.
- **Aks boyunca kesit; kesitler ve malzeme bölgeleri çizimde (10 Kasım 2026; MAP-15, [ADR 4760](../../docs/adr/4760-sections-along-an-axis-and-materials-drawn.md)).**
  `road.terrains.compare`'in kesit yöntemi çizimdeki bir eksen boyunca da ölçer (`axis`, `left`, `right`, `corners`;
  kurpta yarıçap doğrultusunda). `road.terrains.cutfill` kesitleri (`sections`: çizgi, km, alanlar, `start`, kübaj
  cetveli) ve malzeme bölgelerini (`materials`: adı ve hacmi, malzeme tablosu) yazar. Yüzey analizi › Fark › Kesit:
  Eksen, genişlikler, Eksen noktalarında da, Başlangıç km. Harita + Yol.
- **Pafta eğrileri kayıtlı çizim; m. 63(2) kotları ve m. 93(1)(c) karşılaştırması (10 Kasım 2026; MAP-14, [ADR 4740](../../docs/adr/4740-kept-sheet-contours-and-built-up-heights.md)).**
  `road.terrains.sheetcontours` çizimi yüzeyin ve okuduğu katmanların damgasıyla tutulur (yeni kaynak türü `layer`),
  eskir ve yeniden üretilir. Yalnız değişeni yaz sırayı korur: aynı renkli yazısız çizgiler kalır, maskeli yazı ve nokta
  bir çizgi yazılınca yeniden yazılır; Proje ağacı'ndaki kutu projenin ayarıdır (`road.rebuildWhole`, `road.project.edit`
  `regional`; kendiliğinden yeniden üretim de uyar; `.kcad` şema 238). `builtUp`: yollarda ve yerleşik alanda paftada
  2 cm'de bir kot; `check`: seçilen noktaların ölçülen kotu yüzeyle m. 93(1)(c)'nin sınırına göre (uyarı
  `section_check_exceeded`). Pafta eğrileri › Yerleşik alan (63/2). Harita + Yol.
- **Tarama ağı dengelemesi (10 Kasım 2026; MAP-13, [ADR 4720](../../docs/adr/4720-scan-network-adjustment.md)).**
  Yeni `cad.pointclouds.network`: üç ve daha çok tarama bir GNSS ağı gibi birlikte dengelenir; bağlar Tarama kaydı gibi
  kaydedilip normal matrisleriyle ağırlıklanır, sabit taramalar yerinde kalır; döngü kapanmaları ve her taramanın yatay
  gücü söylenir. Nokta bulutu › Tarama ağı; Dönüşüm uygula taramayı matrisiyle taşır. Harita + CBS.
- **Birleşme çizgisi ve mozaik elemanlarının KentOS'ta kurulması (10 Kasım 2026; MAP-12, [ADR 4700](../../docs/adr/4700-orthophoto-seamline-polygons-built.md)).**
  Yeni `cad.orthophoto.seamlines`: iki ortofotonun birleşme çizgisi ve mozaik elemanları (TUCBS Ortogörüntü), eşit uzaklık
  (Voronoi; Pan ve ark. 2009, düz kesimler kesin) ya da radyometrik fark (en büyük farkın en küçüğü, sonra en az maliyet;
  arama bandı, geçilmeyecek alanlar); tek geri alma adımında Mozaik elemanları katmanına. Raster › Birleşme çizgisi.
  Terimler: birleşme çizgisi, mozaik elemanı, ayak izi (TUCBS “ayakizi”), kayıklık; Ortofoto karşılaştırması'nın “dikiş”
  sözleri de değişti. Mevzuat çizginin yerine kural koymaz (BÖHHBÜY m. 67(6)). CAD + CBS.
- **Yedi çizimin kendi hesabının içinde durması; tarifin katmanının alanları (10 Kasım 2026; MAP-20, [ADR 4860](../../docs/adr/4860-stop-within-seven-drawings-and-recipe-layer-fields.md)).**
  Etiket, boykesit, otokorkuluk, menfez, kavşak, platform ve boru hattı profili çizimleri durdurmayı kendi hesabının
  içinde duyar. Tarif katmanın alanlarını (bağlı tablonun sütunu hariç), değer listelerini ve alt türlerini taşır;
  Tariften üret aynı değerli listeye bağlar, başka değerlinin yanına “<ad>_1” ekler. Harita + Yol.
- **Tarifin katmanının saydamlığı, türü ve Yazdır'ı (10 Kasım 2026; MAP-20, [ADR 4440](../../docs/adr/4440-recipe-layer-transparency-type-and-print.md)).**
  İş grafiği'nin tarifi katmanın saydamlığını ve karışım kipini (maskesiz), türünü ve Yazdır'ını taşır; Tariften üret
  eklediği katmana verir. Çizimin nesnesini katmanının türü tutmuyorsa `layer_geometry_mismatch`, plan da. Harita + Yol.
- **İzlemenin dönem dizisi (10 Kasım 2026; MAP-19, [ADR 4420](../../docs/adr/4420-monitoring-series-of-epochs.md)).**
  `cad.asbuilt.check`'in izleme işinde `epochs` ve `date`: her nokta ilk ölçüldüğü döneme göre izlenir (`tracks`),
  günlerle ortalama hız. Uygulama kontrolü › İzleme › Önceki dönemler. Harita + Yol + CBS.
- **Atlasın güncelleme PDF'i (10 Kasım 2026; MAP-18, [ADR 4400](../../docs/adr/4400-atlas-update-pdf-of-changed-pages.md)).**
  `file.export.atlas`'ın `changedOnly`'si: yalnız değişen ve yeni sayfalar, kendi numaralarıyla (PDF sayfa etiketleri),
  `written`; kayıt bütün atlasın; `atlas_unchanged`. CAD + CBS.
- **Boşluklu öznitelik adının ATTRIB etiketi (10 Kasım 2026; MAP-17, [ADR 4380](../../docs/adr/4380-dxf-attribute-tag-for-a-name-with-a-space.md)).**
  Tanımı olmayan öznitelik “_” etiketli görünmez ATTRIB, adı KentOS verisinde (`tagname`); geri okumada dosya kazanır.
  CAD + CBS.
- **Kesişen borunun dış duvarı (10 Kasım 2026; MAP-16, [ADR 4360](../../docs/adr/4360-crossing-pipe-outer-wall-and-live-check.md)).**
  `cad.pipes.profile` kesişen ağın `diameter.wall`'unu okur: dış elips ve arası dolu halka (kesin sınır), `wall`
  cevapta. Boru hattı denetimi açıkken çizim değişince yeniden okunur. Harita + Yol + CBS.
- **Karşılaştırmada kesit yöntemi (10 Kasım 2026; MAP-15, [ADR 4340](../../docs/adr/4340-surface-comparison-by-sections.md)).**
  `road.terrains.compare`'in `method: sections`'ı: eksene dik paralel kesitler, alanlar kesin integral, hacim ortalama
  alan ya da prizmoidal (ortadaki gerçek kesit); kesin TIN yanında. Yüzey analizi › Fark › Kesit. Harita + Yol.
- **Yalnız değişeni yaz (10 Kasım 2026; MAP-14, [ADR 4320](../../docs/adr/4320-regional-redraw-of-surface-drawings.md)).**
  `road.project.rebuild`'in `regional`'ı: çizim kopyada yeniden yapılıp tam karşılaştırılır; aynı çıkan yazılmaz, çizgi
  çiziminde aynı çıkan çizgiler kimliğiyle kalır, öbür değişen çizim bütün; `kept`, `region`. Proje ağacı'nda Yalnız
  değişeni yaz (öntanımlı açık). Harita + Yol.
- **Nokta bulutunda eksen ve açı (10 Kasım 2026; MAP-13, [ADR 4300](../../docs/adr/4300-point-cloud-axis-angle.md)).**
  Dönüşüm uygula'da Eksen ve açı: Rodrigues satırları sabit sırayla, merkez çevresinde, öteleme; tek matris olarak
  geçmişe. Euler açıları yok (sırası kaynakta yazılı değil). Harita + CBS.
- **Dış parçanın numarası parçaya sığar (9 Kasım 2026; MAP-11, [ADR 4280](../../docs/adr/4280-separation-number-fitted-in-its-part.md)).**
  `cad.readjustment.sheets`'in `numberHeight`'ı (kâğıtta mm): numara parçanın içinde yazı; yatay ya da en uzun kenarı
  boyunca, onda bir adımlarla küçülür; `labels`'ta boy, dönüş, pay, sığma. Harita ve CBS.
- **Senaryo ölçülerinde arazi eğimi (9 Kasım 2026; MAP-10, [ADR 4260](../../docs/adr/4260-scenario-terrain-slope.md)).**
  `cad.scenarios.impact`'in `slope`'u: yüzey üçgenlerinin düzleminden alan ağırlıklı ortalama eğim, kullanıcının
  eşiğiyle dik alan ve payı, yüzey dışı alan (veri eksiği). Harita + CBS.
- **Sembolü karardan ver (9 Kasım 2026; MAP-09, [ADR 4240](../../docs/adr/4240-zoning-symbol-from-decision.md)).**
  `cad.zoning.symbolize`: karar alanının MPYY sembolü kademesi ve Kullanım'ından (KentOS'un ad eşlemesi
  `imar-plani-semboller.json` ya da kullanıcının seçimi); karar hiç yazılmaz. Harita + CBS.
- **Parsel kayıtları (9 Kasım 2026; MAP-08, [ADR 4220](../../docs/adr/4220-parcel-register-documents-and-boundaries.md)).**
  Çizimden ayrı `settings.parcelRegister` (`.kcad` şema 216): belgeler, parseller ve paylaşılan sınırlar (halka sınır
  zinciri). `cad.parcels.record` belge yazar ve kapatır, `cad.parcels.records` çizimle okur. Harita + CBS.
- **Köşe sil'de birleşen kenarın içindeki köşeler (9 Kasım 2026; MAP-07, [ADR 4200](../../docs/adr/4200-vertex-delete-carries-inner-vertices.md)).**
  Topolojik düzenleme açıkken Köşe sil iki düz kenarı birleştirince içlerindeki komşu köşeler birleşen kenarın aynı boy
  payına gider (`topology_edit::merged`); açık yolun ucu ve yay eskisi gibi. CAD (Harita + Yol) + CBS.
- **Yüzeyden kot ver (9 Kasım 2026; MAP-06, [ADR 4180](../../docs/adr/4180-elevations-from-surface.md)).**
  Yeni `cad.entities.drape`: her köşe projenin arazi yüzeyinin kotunu alır, dışındaki korur; `breaks` ile TIN
  kenarlarında ara kırık noktaları; yeni `off_surface`. Eğimle kot ver › Hedef Yüzeyden. CAD (Harita + Yol).
- **Hata elipsi etiketinin boş yere konması (9 Kasım 2026; MAP-05, [ADR 4160](../../docs/adr/4160-network-ellipse-label-places.md)).**
  Etiket önce doğuda; elips, çubuk ya da önceki etikete düşerse QGIS'in kartografik sırasıyla ilk boş yerde, yoksa
  doğuda çakışıyor; `labelsMoved`, `labelsCrowded`. Harita.
- **Detay eşleştirmenin kararları inceleme notu (9 Kasım 2026; MAP-04, [ADR 4140](../../docs/adr/4140-reconcile-decisions-as-review-notes.md)).**
  Tutan her karar çözülmüş bir inceleme notudur (`decision`, yazar, `reviewedAt`, adaya ve varlığa damgalı);
  varlık `İnceleme`/`Gözden geçiren` taşımaz; `.kcad` şema 215. Harita + CBS.
- **Bilinen noktanın projedeki kendi yeri (9 Kasım 2026; MAP-03, [ADR 4120](../../docs/adr/4120-known-point-own-place-in-the-project.md)).**
  `map.survey.query`'nin `places`'i: `knownPlaces` (epok önce kendi hızıyla ya da hız ızgarasıyla, sonra zincir; kabul
  noktasına fark ya da neden). GNSS arşivi'nde kabul noktası olmayan baz ucu bilinen noktanın yerinden üçgenle. Harita.
- **Poligon hesabında trigonometrik kotlar (9 Kasım 2026; MAP-02, [ADR 4100](../../docs/adr/4100-traverse-trigonometric-heights.md)).**
  `cad.survey.points`'in poligonunda `work.heights`: başucu açısı, alet ve işaret yüksekliğiyle
  Δh = S·cot z + i − t + (1 − k)·S²/2R; kot kapanması uzunluklarla dağıtılır (`heightMisclosure`); projeksiyona
  indirmede her kenar kendi H + N ortalamasında. Poligon hesabı › Kotlar. CAD + CBS.
- **Onaylı hâline getir (9 Kasım 2026; MAP-01, [ADR 4080](../../docs/adr/4080-map-work-restore-as-approved.md)).**
  Yeni `map.work.restore`: onaylı öğenin silinen nesneleri aynı kimlikle, değişenleri onaylanan değerleriyle geri gelir;
  eklenenler kalır. Harita işi › Değişenler'de ↺. Yalnız Harita.
- **Dikiş ölçümünde dönük birinci görüntü (9 Kasım 2026; MAP-12, [ADR 4060](../../docs/adr/4060-orthophoto-seam-on-a-turned-first-grid.md)).**
  `seam.resample`'da dönük birinci ızgaranın kendisidir: yerler onun piksel merkezleri, kayma doğu ve kuzey;
  `seam.turned`. Kuzeye bakan birincide hesap aynen. CAD + CBS.
- **Tarifin katmanının grubu ve tam stili; çizimlerin içinde durdurma (9 Kasım 2026; MAP-20, [ADR 4040](../../docs/adr/4040-recipe-layer-groups-style-and-stop-within-drawings.md)).**
  Tarif katmanın gruplarını, bütün stilini ve proje çizgi türünü taşır; Tariften üret katmanı gruplarına ekler.
  Enkesit, istinat duvarı, eş yükselti, kazı-dolgu ve sıfır hattı kendi hesabının içinde ilerler ve durur. Harita + Yol.
- **Uygulama kontrolünde yüzeye en yakın uzaklık (9 Kasım 2026; MAP-19, [ADR 4020](../../docs/adr/4020-asbuilt-nearest-distance-to-surface.md)).**
  `cad.asbuilt.check` her noktanın yüzeye en kısa 3B uzaklığını (`nearest`, işareti düşey farkınki) ve yerini verir;
  `surfaceBy: nearest` sınırı ona uygular. Harita + Yol + CBS.
- **Atlasın kaynak kaydı ve değişen sayfalar (9 Kasım 2026; MAP-18, [ADR 4000](../../docs/adr/4000-atlas-update-from-its-record.md)).**
  Atlası PDF yap PDF'in yanına `<ad>.kaynaklar.json` yazar; `previous` ile her sayfa önceki atlasa göre same, changed
  (nedenleriyle), new; gone. CAD + CBS.
- **DXF'te tanımı olmayan öznitelikler (9 Kasım 2026; MAP-17, [ADR 3980](../../docs/adr/3980-dxf-attributes-without-definition.md)).**
  Blok ekinin tanımı olmayan öznitelikleri görünmez ATTRIB (`70 = 1`) olarak yazılır; adı etiket olamayan ya da çok
  satırlı olan KentOS verisinde kalır, söylenir. CAD + CBS.
- **Boru hattı denetiminde et kalınlığı (9 Kasım 2026; MAP-16, [ADR 3960](../../docs/adr/3960-pipe-wall-thickness.md)).**
  `cad.pipes.check`'in çapına `wall`: örtü borunun dış tepesinden (Civil 3D'nin), açıklıklar dıştan dışa; etsiz boru
  `noWall`. Harita + Yol + CBS.
- **Stokta köşelerden üçgenlenmiş taban (9 Kasım 2026; MAP-15, [ADR 3940](../../docs/adr/3940-stockpile-triangulated-base.md)).**
  `road.terrains.stockpile`'a `triangulated`: köşeler kotlarıyla kısıtlı Delaunay'la üçgenlenir, hacim parça parça kesin
  tümlenir; dört köşe bir çemberdeyse notta söylenir. Harita + Yol.
- **Yüzey yapıştırma (9 Kasım 2026; MAP-14, [ADR 3920](../../docs/adr/3920-terrain-surface-paste.md)).**
  Tarifin düzenlemelerine Yüzey yapıştır (`paste`): başka bir yüzeyin üçgenleri aynen gelir, altında kalan kayıtlar
  gider; döngü yalnız tariflerden söylenir; `.kcad` şema 214. Harita + Yol.
- **Nokta bulutunda son dönüşümü geri alma (9 Kasım 2026; MAP-13, [ADR 3900](../../docs/adr/3900-point-cloud-undo-last-transform.md)).**
  Dönüşüm uygula › Son dönüşümü geri al: geçmişin son matrisinin tersi uygulanır, adım geçmişten düşer. Harita + CBS.
- **Dış parça numarasının retinde parselin adı ve nedeni (9 Kasım 2026; MAP-11, [ADR 3880](../../docs/adr/3880-outside-part-number-refusal-names-the-parcel.md)).**
  `cad.readjustment.sheets` ve Dış parça no, numara alamayan parseli Ada/Parsel adıyla ve nedeniyle söyler. Harita ve
  CBS.
- **Altyapı bağları katmanda (9 Kasım 2026; MAP-10, [ADR 3860](../../docs/adr/3860-scenario-links-drawn-on-a-layer.md)).**
  Yeni İşlem aracı Altyapı bağlarını çiz (`scenarios.links`): senaryo ölçüsünün bağları “Altyapı bağları” katmanına
  çizgi olarak, tek geri alma adımı. Harita + CBS.
- **İmar denetimi İşlem araç kutusunda, bölgeler hata katmanında (9 Kasım 2026; MAP-09, [ADR 3840](../../docs/adr/3840-zoning-check-in-processing-and-error-layer.md)).**
  Yeni İşlem aracı İmar planını denetle (`zoning.check`); Bulguları hata katmanına yaz yola giren her bölgeyi “İmar
  bulguları” katmanına alan olarak yazar, tek geri alma adımı. Harita + CBS.
- **Sınır grafiği çizimde: kirişli yayın şeridi, tek sözde sorunlar (9 Kasım 2026; MAP-08, [ADR 3820](../../docs/adr/3820-boundary-graph-strip-and-shared-rings.md)).**
  Çizimde göster kirişli yayın şeridini boyar; ekranda halkası buluşan sorunlar tek halka ve sayılı tek söz olur.
  Sözleşme değişmez. Harita + CBS.
- **Seçili nesnenin topoloji bulguları (9 Kasım 2026; MAP-07, [ADR 3800](../../docs/adr/3800-topology-findings-of-the-selected-object.md)).**
  `cad.topology.check`'in `uids` süzgeci; Öznitelikler'de tek nesnenin Topoloji bölümü (son denetimden, düğmesi sekmeyi
  bulguda açar) ve Formla düzenle'de kaydın bulguları. Harita + CBS.
- **Referanstan kot ver (9 Kasım 2026; MAP-06, [ADR 3780](../../docs/adr/3780-raise-lower-by-reference.md)).**
  Yeni `cad.entities.raise` (Civil 3D'nin Raise/Lower by Reference'ı): köşe referanstan eğimle ya da kot farkıyla kot
  alır, nesnenin bütün kotları aynı farkla kayar. Eğimle kot ver › Hedef Referanstan. CAD (Harita + Yol).
- **Hata elipslerinin etiketi (9 Kasım 2026; MAP-05, [ADR 3760](../../docs/adr/3760-network-ellipse-labels.md)).**
  `map.survey.edit network`'ün `ellipses.labels`'ı her elipsin doğusuna yarı eksenleri (kot çubuğuyla kot yarı
  aralığını da) mm olarak yazar; boy kâğıtta 2,5 mm × ölçek paydası / 1000; Ağ dengelemesi'nde Etiket. Harita.
- **Detay eşleştirmenin sürüm derlemesinde ölçümü (9 Kasım 2026; MAP-04, [ADR 3740](../../docs/adr/3740-reconcile-measured-in-a-release-build.md)).**
  `map.features.reconcile`'ın planı sürümde 40 000 kümede 0,215 s, 100 000 kümede 0,567 s (debug 1,042 ve 2,676 s);
  aynı commit ve veri, Apple M5. Kod değişmedi. Harita.
- **GNSS ağının haritada görünüşü (9 Kasım 2026; MAP-03, [ADR 3720](../../docs/adr/3720-gnss-network-drawn-on-the-map.md)).**
  GNSS arşivi › Bazlar › Ağı göster: iki ucu kabul noktası olan bütün bazlar haritada, dışlanan kırmızı noktalı; hiçbir
  şey yazılmaz. Harita.
- **Koordinat dönüştür'de nokta başına karar (9 Kasım 2026; MAP-02, [ADR 3700](../../docs/adr/3700-coordinate-transform-point-by-point-areas.md)).**
  `cad.crs.transform`'un `flagged`'i her noktanın dışında kaldığı kullanım alanlarını (kaynak, dönüşüm, hedef) söyler;
  nokta yine dönüştürülür; kenara 10⁻⁹° yakın nokta kenarda. Pencerede Alan dışı sütunu. CAD + CBS.
- **Onayın nesnelerin değerlerini tutması (9 Kasım 2026; MAP-01, [ADR 3680](../../docs/adr/3680-map-work-approved-values.md)).**
  Onay `approvedValues`'i yazar (özetin alındığı metin, en çok 4 MiB; `.kcad` şema 213); `map.work.query` değişen
  özniteliğin adını, onaylanan ve şimdiki değerini, silinen nesnenin onaylanan hâlini söyler. Yalnız Harita.
- **Dikişte ikinci görüntünün yeniden örneklenmesi (9 Kasım 2026; MAP-12, [ADR 3660](../../docs/adr/3660-orthophoto-seam-resampled-onto-the-first-grid.md)).**
  `cad.orthophoto.compare`'in `seam.resample`'ı: pikselleri farklı boyda ya da dönük ikinci görüntü birincinin ızgarasına
  kübik evrişimle örneklenir, ölçüm birincinin pikselleriyle; dönük birinci reddedilir. CAD + CBS.
- **Tarifin katmanları adıyla, çizimin içinde durdurma (9 Kasım 2026; MAP-20, [ADR 3640](../../docs/adr/3640-recipe-layers-and-stop-within-a-drawing.md)).**
  Tarif çizimin katmanının tanımını taşır; Tariften üret katmanı `layers` ile verilen, adıyla bulunan ya da tanımıyla
  eklenen katmana çizer (iki aynı adlı: sorulur); önce önizleme. Koridorun şevli planı durdurmayı kesitler arasında duyar.
  Harita + Yol.
- **Uygulama kontrolünün iş türleri ve izleme (9 Kasım 2026; MAP-19, [ADR 3620](../../docs/adr/3620-asbuilt-job-kinds-and-monitoring.md)).**
  `cad.asbuilt.check`'e `kind` (as-built, aplikasyon, izleme) ve `control`: izlemede bu dönem önceki dönemle adla
  karşılaştırılır (fark = hareket; yüzey, çizgi, yer, paket reddedilir), oynayan kontrol noktası söylenir. Harita + Yol + CBS.
- **Atlas sayfalarının kaynak dökümü (9 Kasım 2026; MAP-18, [ADR 3600](../../docs/adr/3600-atlas-page-sources.md)).**
  `file.export.atlas`'ın cevabında her sayfanın kaynakları (kendi nesnesi önce, haritaların sayfadaki zeminine değenler) ve
  nesne özetleri; Atlası PDF yap önizlemesinde Kaynak sütunu. CAD + CBS.
- **Paketten açılan projede ızgaralar bulunur (9 Kasım 2026; MAP-17, [ADR 3580](../../docs/adr/3580-package-grids-found-on-opening.md)).**
  Açılışta adlanan ızgara çekirdekte, kitaplıkta, yoksa yanındaki `izgaralar/`'da (SHA-256 denetimiyle) aranır ve
  çekirdeğe okunur; kitaplık yazılmaz. CAD + CBS.
- **Hat profili ağıyla tutulur (9 Kasım 2026; MAP-16, [ADR 3560](../../docs/adr/3560-pipe-profile-kept-with-its-network.md)).**
  Yeni kaynak türü ağ; ağ değişince profil Proje ağacında güncel değil, yeniden üretilir; `.kcad` şema 212. Tutulan
  çizimlerin nesne damgası artık `id`'siz (kaydedip açınca “elle değişti” hatası giderildi). Harita + Yol.
- **Yüzey farkında malzeme bölgeleri (9 Kasım 2026; MAP-15, [ADR 3540](../../docs/adr/3540-material-zones-in-a-surface-comparison.md)).**
  `road.terrains.compare`'e `zones` (malzemenin adı, kapalı sınır, kendi kazı/dolgu katsayıları): her bölgede kesin TIN–TIN
  farkı, bölge dışı ve malzemeye göre katsayılı toplam; bölgeler örtüşemez. Harita + Yol.
- **Pafta eğrilerinde seçilen noktaların kotları (9 Kasım 2026; MAP-14, [ADR 3520](../../docs/adr/3520-sheet-chosen-point-heights.md)).**
  `road.terrains.sheetcontours`'a `points` (m. 78(1)(ğ)'nin “gerekli görülen diğer noktalar”ı): kendi kotu varsa o,
  yoksa yüzeyden; yalnız kot yazısı; not en büyük ölçü–yüzey farkını söyler. Harita + Yol.
- **Bulutta ters matris ve dönüşüm geçmişi (9 Kasım 2026; MAP-13, [ADR 3500](../../docs/adr/3500-point-cloud-inverse-and-history.md)).**
  Dönüşüm uygula'da Tersini uygula (`Affine::inverse`, sabit hesap sırası); yazılan bulut Kaynak ve Dönüşümler
  özniteliklerini taşır, zincirde geçmiş büyür. Harita + CBS.
- **Ayırma çapında dış parçanın numarası çizimde (9 Kasım 2026; MAP-11, [ADR 3480](../../docs/adr/3480-separation-sheet-piece-numbers-drawn.md)).**
  Numaralı dış parça parselin “dış parçalar” katmanında alan olarak (Parsel, Kısım, ParselNo) ve içinde numaralı
  noktayla; Koordinat listesi yalnız köşeleri okur; cevapta `labels`. Harita + CBS.
- **Senaryoda alanın en yakın hatta bağı (9 Kasım 2026; MAP-10, [ADR 3460](../../docs/adr/3460-scenario-area-to-nearest-line.md)).**
  `cad.scenarios.impact`'in `near`'ı her ölçülen alanı en yakın altyapı nesnesine bağlar (nesne, merkez, en yakın yer,
  uzaklık; eşitlerde çizimde önce gelen); Çizimde göster bağları çizer. Harita + CBS.
- **İmar denetiminde bölgeler cevapta (9 Kasım 2026; MAP-09, [ADR 3440](../../docs/adr/3440-zoning-regions-in-the-answer.md)).**
  `cad.zoning.check`'in yol genişliği ve yola giren parsel bulguları sayılan örtüşme parçalarını `regions`'ta taşır
  (kanonik halkalar); Python ve MCP çizebilir. Harita + CBS.
- **Kirişlerle çizilmiş yay (9 Kasım 2026; MAP-08, [ADR 3420](../../docs/adr/3420-boundary-arc-drawn-with-chords.md)).**
  Sınır grafiğinde `approxArc`: yayı köşeleri yayda (1 µm ya da `arcTolerance`) duran bir sınır zinciriyle çizen komşu;
  şeridin alanı ve en geniş yeri; zincirin köşeleri ayrıca eksik köşe sayılmaz. Harita + CBS.
- **Değişen kenarın içindeki komşu köşeler (9 Kasım 2026; MAP-07, [ADR 3400](../../docs/adr/3400-inner-vertices-follow-a-changed-edge.md)).**
  Topolojik düzenlemede tutamaç, Köşe tablosu, Esnet ve köşe komutu kenarın içindeki komşu köşeleri kenarın aynı kesrine
  taşır (`topology_follow::neighbours`); yayı değişen kenarda komşu parçalar yayın parçası olur. CAD + CBS.
- **Eğimle kot ver'de yüksek/alçak nokta (9 Kasım 2026; MAP-06, [ADR 3380](../../docs/adr/3380-grade-high-low-point.md)).**
  `cad.entities.grade`'in `ahead`/`back`'i: başlangıçtan ve bitişten iki eğimin kesiştiği yere köşe (yay çemberinde
  kalır), iki kola göre kotlar; paralel ya da aralık dışı `no_intersection`. Pencerede Hedef › Yüksek/alçak nokta. CAD.
- **Kot çubuğu (9 Kasım 2026; MAP-05, [ADR 3360](../../docs/adr/3360-network-height-bars-drawn.md)).** Ağ kabulünde
  elipslerle birlikte her noktanın kot yarı aralığı, elipsle aynı büyütmede çentikli düşey çubuk (`ellipses.heights`).
  Harita.
- **Detay eşleştirmede yaylı aday (9 Kasım 2026; MAP-04, [ADR 3340](../../docs/adr/3340-reconcile-arc-candidates.md)).**
  Çoklu çizginin yayları yay olarak ölçülür (Hausdorff'a yayın sekizde birleri, yaya kesin uzaklık); kutular yayın
  şişkinliğini tutar. Harita.
- **GNSS bazı haritada (9 Kasım 2026; MAP-03, [ADR 3320](../../docs/adr/3320-gnss-baseline-drawn-on-the-map.md)).**
  GNSS arşivi › Bazlar satırı bazı iki ucunun kabul noktaları arasında geçici kesikli çizgi, ok ve uzunlukla çizer;
  İşareti kaldır ile gider. Harita.
- **Aplikasyon'da çizginin kendi yüksekliği (9 Kasım 2026; MAP-02, [ADR 3300](../../docs/adr/3300-stakeout-lines-at-their-own-height.md)).**
  Kotlu hedefin çizgisi istasyonla hedefin elipsoit yüksekliklerinin ortalamasında (ADR 2900'ün kuralı); kotsuz hedef
  ortalamada; h sütunu. CAD + CBS.
- **Onaydan sonra neyin değiştiği (9 Kasım 2026; MAP-01, [ADR 3280](../../docs/adr/3280-map-work-changed-object-parts.md)).**
  Onay her nesnenin geometri, öznitelik, katman ve görünüş özetini tutar (`approvedParts`, `.kcad` şema 211);
  `changes` değişen parçaları söyler. Yalnız Harita.
- **Dikişte çift kübik piksel altı ve yaylı hat (9 Kasım 2026; MAP-12, [ADR 3260](../../docs/adr/3260-orthophoto-seam-bicubic-and-arcs.md)).**
  `seam.subpixel: bicubic` ikinci görüntüyü kübik evrişimle (Keys, a = −1/2) yeniden örnekler; dikiş hattının yayları
  yay boyunca ölçülür. CAD + CBS.
- **Tariften üret (9 Kasım 2026; MAP-20, [ADR 3240](../../docs/adr/3240-produce-a-recipe-in-another-project.md)).**
  `road.project.produce` İş grafiği'nin tarifini başka projede çizer: kaynaklar adlarıyla, katman kimliğiyle; var olan
  çizim bırakılır; adıyla istenenler hepsi ya da hiçbiri. İş grafiği'nde Tariften üret…. Harita + Yol.
- **Saha paketinin yüzeyi (9 Kasım 2026; MAP-19, [ADR 3220](../../docs/adr/3220-field-package-surface.md)).** Paket
  yüzeyin kendisini `yuzey.xml` (LandXML TIN, SHA-256'sı bildirgede) olarak taşır; paketin tasarımıyla kontrolde yüzey
  farkı paketin yüzeyiyle, `surfaceChanged` çizimin yüzeyinin değiştiğini söyler. Harita + Yol + CBS.
- **Yeniden üretim bir iş (8 Kasım 2026; MAP-20, [ADR 2440](../../docs/adr/2440-rebuild-as-a-job-progress-and-stop.md)).**
  `road.project.rebuild` iş olarak ilerler ve durur (durdurulan hiçbir şey yazmaz); proje ağacında ilerleme çubuğu ve
  Durdur; GUI, Python ve MCP aynı sonucu verir. Harita + Yol.
- **Dikiş ölçümü (8 Kasım 2026; MAP-12, [ADR 2460](../../docs/adr/2460-orthophoto-seam-measured-in-the-images.md)).**
  `cad.orthophoto.compare`'in `seam`'i: örtüşmede NCC ve parabolle piksel altı kayma, yer durumları (ok, low, edge,
  flat), m. 67(6) sınırı; pikseller aynı boyda ve dönüksüz olmalı. CAD + CBS.
- **Rolüne göre bağlamsal araçlar (8 Kasım 2026; MAP-01, [ADR 2480](../../docs/adr/2480-contextual-tools-by-object-role.md)).**
  Şerit panelinin `roles` alanı (surface, network, zoning): seçimin hepsi o roldeyse Yüzey, Altyapı, İmar panelleri;
  pencereler seçimin yüzeyi ya da ağıyla açılır (`selection_roles.rs`). Harita + Yol.
- **Katmanın koordinatlarını dönüştürmenin önizlemesi (8 Kasım 2026; MAP-02, [ADR 2500](../../docs/adr/2500-reproject-preview-shifts-and-reasons.md)).**
  `cad.layers.reproject`'in cevabı (plan da) nesne başına kayma, EPSG kullanım alanlarının notları, toplu kayma ve
  zincirin kararıdır; ret her nedeni sayısıyla söyler (yazı, daire, yay artık “sistemin dışında” değil). Masaüstünde
  Koordinat sistemi ▸ Dönüştür önizleme penceresi açar, Uygula yazar. Harita + CBS.
- **Bilinen noktanın kendi hızı (8 Kasım 2026; MAP-03, [ADR 2520](../../docs/adr/2520-known-point-own-velocity.md)).**
  `bn-N`'nin yer merkezli hızı (m/yıl, epokuyla) noktayı ölçme epokuna doğrusal taşır (IERS 2010 §4.1.2); yoksa hız
  ızgarası. `moved[].by`; `.kcad` şema 207. Ağ ızgarasız da dengelenir, yeni noktaların karara dönmesi ızgara ister.
  Harita + CBS.
- **Detay eşleştirmede alan kuralları ve alan girişi (8 Kasım 2026; MAP-04, [ADR 2540](../../docs/adr/2540-reconcile-field-rules-and-entry.md)).**
  `map.features.reconcile` varlığın değerlerini sınıf katmanının alan kurallarından (`check_value`) geçirir, kanonik
  yazar; uymayan `invalid`'de nedeniyle, grup yazılmaz. Pencerede Alan gir (listeden seçim). Harita.
- **Haritacılığın kelimeleriyle üç ad (8 Kasım 2026, sahibin onayıyla).** Adayları uzlaştır → **Detay eşleştirme** (işaret
  özniteliği Eşleştirme; eski Uzlaştırma okunur), Arşivden ağ dengelemesi → **GNSS ağ dengelemesi**, katman menüsünde
  Sistem ata / Yeniden izdüşür → Koordinat sistemi ▸ **Tanımla / Dönüştür**, raster için **Rasteri projenin sistemine
  çevir**. Kimlikler aynı, eski adlar arama takma adı. Kayıt ADR 2500, 2540, 2560.
- **GNSS ağında güven bölgesi (8 Kasım 2026; MAP-05, [ADR 2560](../../docs/adr/2560-network-confidence-regions.md)).**
  Yeni noktaların hata elipsi (yarı eksenler √χ²₂, yön, kotun aralığı √χ²₁) sınıfın güveninde, yoksa 0,95; dairede yön
  yok. Pencerede özet ve Noktalar sütunu. Harita + CBS.
- **Paralel kaydır'da referans ve Bu uzaklığı koru (8 Kasım 2026; MAP-06, [ADR 2580](../../docs/adr/2580-parallel-shift-reference-and-kept-distance.md)).**
  `edgeShift`'in `reference`'ı kenarı başka bir düz kenara uzaklıkla getirir; `keep` uzaklığı harita bağı yazar (sınır
  kenarı `boundary`, mimari bağların çözümüyle izler; dönen referans, kilitli komşu, çelişki hiçbir şey yazmaz). `.kcad`
  şema 208. Araçta Referans (R), Koru (K). CAD (Harita + Yol) ve CBS'nin tipli katman düzenlemesi.
- **Ortak kenarın ortasındaki köşeler (8 Kasım 2026; MAP-07, [ADR 2600](../../docs/adr/2600-shared-edge-junctions-and-inner-vertices.md)).**
  Paralel kaydır'ın Topolojik düzenlemesi kenarın içindeki komşu köşeleri (üçlü birleşim, ara köşe, delik, çizgi ucu)
  kenarla taşır (`carried`); kısalan kenarın dışına düşen ve yayın ortasındaki köşe açıklamayla reddedilir.
- **Sınır grafiğinde yay ile kiriş uyuşmazlığı (8 Kasım 2026; MAP-08, [ADR 2620](../../docs/adr/2620-boundary-arc-chord-mismatch.md)).**
  `cad.parcels.boundaries`'in `arcChord` sorunu: aynı iki köşe arasında yay ile kiriş (ya da iki farklı yay); iki sınır,
  şeridin alanı ve ortadaki ayrılış. Pencerede Fark sütunu. Harita + CBS.
- **İmar planı denetiminde yola giren parsel (8 Kasım 2026; MAP-09, [ADR 2640](../../docs/adr/2640-zoning-parcel-inside-road-width.md)).**
  `cad.zoning.check`'in `roadParcel`'i: yolun planda yazılan genişliğine giren parsel katmanı parseli, alanı ve
  Ada/Parsel'iyle; yasal sonuç iddia edilmez. Harita + CBS.
- **Senaryo ölçülerinde altyapıya yakınlık (8 Kasım 2026; MAP-10, [ADR 2660](../../docs/adr/2660-scenario-infrastructure-proximity.md)).**
  `cad.scenarios.impact`'in `infrastructure`'ı: altyapı katmanındaki en yakın nesneye kullanıcının uzaklığında olan
  alan ve payı; senaryonun değiştirdiği katmanda her taraf kendi hattıyla. Harita + CBS.
- **Senaryo ölçüleri (6 Kasım 2026; MAP-10, [ADR 1440](../../docs/adr/1440-design-alternative-impact.md)).** Eski adı
  Alternatif etkisi (sahibin isteğiyle 7 Kasım 2026'da değişti). `cad.scenarios.impact` (Harita + CBS, salt okunur):
  senaryo ↔ mevcut durum, birimli ölçüler, değişen kararlar, taban damgasıyla eskime; tek puan yok.
- **İmar planı denetimi (6 Kasım 2026; MAP-09, [ADR 1420](../../docs/adr/1420-zoning-plan-typed-check.md)).**
  Yerleşik `imar-plani` profili ve `cad.zoning.check` (Harita + CBS, salt okunur); karar renkten değil tipli değerden;
  MPYY/PlanGML uygunluğu iddia edilmez.
- **Parsel alan denetimi (6 Kasım 2026; MAP-08, [ADR 1400](../../docs/adr/1400-parcel-area-check-and-identity.md)).**
  `cad.parcels.check` (Harita + CBS, salt okunur): hesap alanı ↔ tapu alanı, 2022 Yönetmeliği Madde 8(2) yanılma
  sınırı; parsel kimliği il/ilçe/mahalle/ada/parsel birlikte (`cad.parcels.create` de numarayı adada arar).
- **Taşınma raporu (6 Kasım 2026; MAP-07, [ADR 1262](../../docs/adr/1262-topology-clean-move-report.md)).**
  `cad.topology.clean` (Harita + CBS): Topolojik temizlik komut olarak, her taşınma ve kabul eşiğiyle; araç nesnelerini
  `topology_clean.rs`'ten alır. Formlar her zaman alt alta değil: kısa ve birbirine ait denetimler bir satırda.
- **Eğimle kot ver (6 Kasım 2026; MAP-06, [ADR 1290](../../docs/adr/1290-grade-between-vertices.md)).** `cad.entities.grade`
  (CAD: Harita, Yol). Pencereler kitin `Form`'uyla: tek etiket sütunu, eşit genişlikte kutular.
- **KentOS Yol: arazi, aks, boykesit, enkesit, koridor, kübaj ve yol projesi (3 Kasım 2026; [plan/road.md](../../plan/road.md), [ADR 0900](../../docs/adr/0900-road-terrain-surface.md)–[0911](../../docs/adr/0911-road-project-tree-views-and-rebuild.md)).**
  Ayrı ürün modülü (`CadWork::Road`, `Module::Road`, `kentos-yol`; dal `road-01-yuzey`'den birleşti). Hesap
  `crates/shared/road` (`kentos-road`: kısıtlı Delaunay `kentos.cdt/1`, aks ve km denklemleri, düşey kurplar, tip kesit
  ve kesit hattı, dever, koridor, uç alan/prizmoidal ve yüzey hacmi, Brükner, menfez, duvar, otokorkuluk, kavşak, ön
  güzergâh); imar planı da kullanabilsin diye paylaşılır. Projenin yol ayarları `settings.road` (`.kcad` şema 162,
  yalnız yol projesinde; yüzeyler, akslar, boykesitler, tip kesitler, kesit takımları, deverler, koridorlar, kübajlar,
  Brüknerler, pafta takımları, korkuluk, menfez, duvar, poz defteri, kavşaklar, yol çizimlerinin kaydı); sonuçlar
  yazılmaz, açılışta kaynaklardan kurulur. Komutlar `road.*` (terrains, alignments, profiles, templates, sections,
  superelevations, corridors, volumes, hauls, stakeout, sheets, guardrails, culverts, walls, pay, junctions, routes,
  project) ve `file.import/export.landxml`, `file.export.ifc` (IFC 4.3 IfcAlignment); her biri ≥20 bağımsız Python
  başvurulu ortak durumla, Python `kentos.road`, MCP. Masaüstünde Arazi, Güzergâh ve Koridor sekmeleri ve pencereleri,
  3B sahne, Yol projesi paneli (KentOS UI'ın durum hapı ve durum ağacıyla, eskiyen çizimleri yeniden üretme,
  kendiliğinden yeniden üret) ve Görünüşler (plan, boykesit ve enkesit tek km'de). Standart profili (KGM) sahibin
  kaynağını bekler, değer uydurulmaz. Resimler snapshot'la; gerçek pencerede elle kabul açık.

- **KentOS Yol: üstyapı, formasyon, kübaj kuralları ve Yol pencerelerinin düzeni (4 Kasım 2026; [ADR 0975](../../docs/adr/0975-road-pavement-formation-and-volume-rules.md), plan/road.md Y26, dal `road-03-ustyapi`).**
  - **Üstyapı:** tip kesitin `pavement`'ıdır: tabakalar yukarıdan aşağı, bitmiş yüze paralel; kenar dik ya da şevli
    (Civil 3D'nin şeridi ve ShoulderExtendAll'u). Bitmiş çizginin üstüne ve kesitin izinin dışına taşmaz.
    `kentos_road::section::pavement` tabaka alanlarını ve formasyonu kesin integralle hesaplar.
  - **Kübaj kuralları:** `rule` NETPRO'nun adlarını taşır (Yol–Zemin, Formasyon–Zemin, Yol–Bitkisel Toprak,
    Formasyon–Bitkisel Toprak). Kural yoksa davranış öncekidir. Formasyondan kazı yolun kazısı + zeminin altındaki
    üstyapıdır, dolgu yolun dolgusu − üstündekidir. Cetvel tabaka hacimlerini taşır; uyarı kodu `pavement_unbuilt`'tir.
  - **Pencere düzeni:** sahibin “dialog formları çok karmaşık” sözü üzerine Yol pencereleri tek iskelettedir
    (`road_dialog::RoadDialog`, DESIGN.md §7.19). Solda gruplu form, sağda çizim, en çok dört özet ve parçalı seçimle tek
    tablo, alt çubukta tek birincil eylem vardır. Yalnız okuyan pencerede Önizle yoktur. Denetim sınırları formun son
    bölümüdür ve kaydedilmemiş tanımı okur (Dever, Y27.3; oran sınırı 1:n'nin n'i). Yardımcı pencere üst pencerenin
    yerine açılır, “Geri” ile döner, sonucu üstün formuna yazar (Genişletme önerisi, Y28.3). Çizim alanı
    görüntüleyicidir (`widget::picture::Picture`, ADR 0937): yakınlaştır, kaydır, PNG kaydet; büyüyen pencerede
    `widget::grow::Grows` ile kalan yeri alır; `layout` görüntüleyicinin durumunu değiştirmez (diyalog iki kez yerleştirir). Tablolar kitin veri tablosudur (`road_dialog::table`/`sheet`/`sheet_fill`/`edit_table`, ADR 0938); sütun genişliği yazı alanıdır, kit payları ekler. Taşınanlar: Kırmızı kot
    önerisi, Kübaj, tip kesit düzenleyicisi. `Form::wide` eklendi.
  - **KCAD şema 166:** üstyapı ve kural `SCHEMA_WITH_ROAD_PAVEMENT` ile yazılır. Yayımlanmış şemaya (162) alan
    eklemek, eski programın örnek projeye “bozuk” demesine yol açtı. Yeni okuyucu daha yeni şemaya “daha yeni bir
    sürümle yazılmış” der. Sonrakiler aynı kuralla: 167 deverin dönme ekseni (Y27), 168 kesit takımının `clip`'i,
    `breaks` yöntemi ve kurp aralığı (Y29, ADR 0933). Yeni bir yol alanı = yeni şema, kapısı `decode.rs`'te.
  - **Kesit hatlarının yeri (Y29):** arazi kırıklarından seyreltilen kesitler eksen arazisini sabit aralıktan iyi
    taşır (0,10 m toleransla 67 kesit en çok 0,044 m; her 5 m 174 kesit 0,078 m). İç kurpta buluşan kollar
    yakından uzağa kırpılır; dairede her çift merkezde buluştuğu için sıra yuvarlamaya bırakılmaz.
  - **Kurpta kübaj (Y30):** ortalama alan kurpta sistematik sapar, sıklaştırma gidermez (R 60 m deneyde −%3,8 her
    aralıkta). Etkin alan A + κQ (Pappus) 10 m'de bile +%0,3. Düzeltme yalnız ortalama alanda, isteğe bağlı.
  - **Örnek proje:** `samples/yol_ornegi.py` Yol komutlarıyla `samples/yol-ornegi.kcad`'i yazar.
  - **KGM bantları (6 Kasım 2026, [ADR 0980](../../docs/adr/0980-road-profile-bands-as-kgm-asks.md)):** `road.profiles.draw` `bands: kgm` ile KGM 2019 şartnamesinin sırasını çizer (Kırmızı Kot, Siyah Kot, Kilometre, Yatay Geometri, Düşey Geometri, Dever); Türk paftasında "A=" sapma açısıdır (grad), klotoid "Kl. A=" yazılır.
  - **PI'lardan ove (6 Kasım 2026, [ADR 0979](../../docs/adr/0979-road-ove-clothoid-on-the-pis.md)):** aks noktasının `ove` bayrağı kurbu sonrakine ove klotoidiyle bağlar, A PI'lardan çözülür (ortadaki aliyman klotoidin başlangıç teğeti); KCAD şema 188; Türkiye'de ove için resmî kural bulunamadı (KGM 2019 yalnız klotoidi şart koşar).
  - **Eğimler oran (6 Kasım 2026, [ADR 0977](../../docs/adr/0977-road-grades-as-ratios.md)):** enine eğim, dever, menfez, platform ve diş kazısı eğimleri kaynakta ve komutlarda orandır (−0,025 = %−2,5, birim `grade`); KCAD şema 187 eski dosyanın yüzdesini 100'e bölerek okur; pencereler yüzde gösterir (`road_dialog::percent_text`/`grade_of`).
  - **Ölçüm:** 19 çekirdek ve 26 komut durumu bağımsız kesirli referanstan; 17091 ortak durum geçer. Ana dalda kalan iki
    Python hatası düzeltildi ya da ayrıldı: propose'un katalog örneği düzeltildi; pafta font ön denetimi bu makinenin
    font ortamından düşer ve bu işle ilgisizdir.

- **Oda modülleri ve kontrollü tekrar kullanımı (31 Ekim 2026; [ADR 0775](../../docs/adr/0775-room-modules-and-controlled-reuse.md), ADV-05 [~]).**
  Tek kattaki düz duvarlı oda (duvar, açıklık, mahal, donatı, ölçü, etiket, yazı) modül olur: tanım `settings.arch.modules`'ta
  kendi çerçevesinde (`.kcad` şema 142 opt-in, `FORMATS_VERSION` 102), örnek ayrı kimlik eşlemesiyle sıradan mimari nesneler;
  bloğa gömülmez. `cad.modules.edit` (define, place: taban, açı, ayna, kat, tadilat durumu, numara öneki/soneki; update planıyla,
  kilitli atlanır, istisna ve silinen çatışma; except, restore, detach, remove: örnekler `sourceGone` ile kalır),
  `cad.modules.query`; her yazma tek adım “Oda modülü”. Güncellemede açıklamanın yeri (etiket, yazı, ölçü yazısı) örneğin
  kalır. Aynalı örnekte kapı aynı yanda, açılışı end. Oda modülleri penceresi, Modül yerleştir aracı, Öznitelikler'de Modül
  bölümü; DXF raporu “Oda modülü örneği”. Tekrarlayan kat, daire modülü, tasarım seçeneği, tanımdan sonra üye ekleme açık.
  43 + 22 ortak durum (`module_cases.py`).

- **Tasarım seçenekleri (31 Ekim 2026; [ADR 0788](../../docs/adr/0788-design-options.md), ADV-05 [~]).**
  Planın bir parçası için sınırlı alternatifler: `settings.arch.options` (küme, birincil, seçenekler, üyeler kalıcı
  kimlikle; `.kcad` şema 149, opt-in), hiçbir seçenekte olmayan nesne ana model. `cad.options.edit` (define,
  copy: bağımsız kopya aynı yerde, bağlar kopyalara; addMembers, removeMembers, primary, accept: seçenek ana model olur,
  öbürleri silinir, küme kalkar; delete, rename; tek adım “Tasarım seçeneği”; otomatik birleştirme yok); ana model
  seçeneğe, seçenek başka seçeneğe bağlanamaz `option_leaning`. `cad.options.query` (sayılar, mahal alanı, duvar boyu,
  `compare`: Veri karşılaştır'ın konum kuralıyla aynı/değişen/eklenen/kaldırılan, alan farkı). Çizim, pafta, Metraj,
  çizelge, Mimari denetim ve DXF birincili gösterir; görünüş tarifinin `options`'ı bağlı haritada başka seçeneği
  (`map_phasing_of`, haritanın deposu `Spatial::set_option_view`). Seçenekler birbirini görmez: gösterilmeyenin duvarı
  birleşmez ve açıklığı duvarı kesmez (`Store::set_option_out`), iki seçeneğin açıklığı çakışmaz, mahal sınırını ana
  model ve kendi seçeneğinde bulur (`SpaceSearch::found_as`); birincil değişince mahaller, açıklıklar ve çizelgeler aynı
  adımda izler. Mimari › Proje ▾ Tasarım seçenekleri (`arch.options`), Görünümler'de tarifin Tasarım seçeneği (şablona bağlı
  tarifte de görünümün; şablon seçenek taşımaz). 49 + 23
  ortak durum (`option_cases.py`). Seçenek bileşimi, seçenekler arası eşleme ve gerçek pencerede elle kabul açık.

- **Oda modüllerinin ikinci dilimi: üye ekle ve çıkar, elle değişen, daire ve kat tekrarı (31 Ekim 2026; [ADR 0779](../../docs/adr/0779-room-modules-members-drift-and-flats.md), ADV-05 [~]).**
  `cad.modules.edit`'in `addMembers`'ı (örnekte çizilen nesneler tanıma katılır, öbür bağlı örneklere kendi matrisleriyle
  gelir; kilitli örnek atlanır, sonraki güncellemede eksiklerini alır: geride kalan örnek bütünüyle eşitlenir),
  `removeMembers`'ı (anahtar tanımdan çıkar, verilen nesne kalır, öbür karşılıklar silinir, istisna ya da kilitli olan
  kalır `module_member_released`; kalan üye ona bağlıysa `module_member_needed`), `repeat`'i (kaynak kattaki bağlı
  örnekler hedef katlara aynı yerde, numara ve bölüm hedefin ya da örneğe özel satırın; `module_repeat_empty`,
  `module_repeat_taken`) ve `place`'in ve örneğin `unit`'i (bina programının bölümü, daire modülü; `.kcad` şema 145
  opt-in). Elle değişen nesne hesaplanır (`modules_edit::drifted_keys`: tanım örneğin matrisiyle, belgenin yazdıkları ve
  açıklamanın yeri hariç, 1e-6 içinde); `cad.modules.query`'nin `drifted`'ı, güncellemede `module_drift_overwritten`.
  Oda modülleri'nde Seçimi örneğe ekle, Seçimi modülden çıkar, Katları tekrarla; Öznitelikler'de Kaynakla satırı ve Modülden
  çıkar; `arch.moduleAddMembers`, `arch.moduleRemoveMembers`; Modül yerleştir'de Bölüm. 52 + 22 ortak durum
  (`module_flats_cases.py`). Tasarım seçeneği, çok katlı tek modül açık; gerçek pencerede elle denenmedi.

- **Çok katlı oda modülü ve kattaki modülsüz nesnelerin tekrarı (31 Ekim 2026; [ADR 0796](../../docs/adr/0796-multi-storey-modules-and-loose-level-repeat.md), ADV-05 [~]).**
  Birden çok kattaki nesneler tek modül olur: taban en alçak kat, duvar, mahal ve donatının `storey`'i taban üstündeki yeri
  (katlar kotlarına göre, `modules_edit::storey_order`; `.kcad` şema 152, opt-in); `place` ve `repeat` üst katları
  hedefin üstündeki katlara yazar, yetmeyen kat `module_storey_missing`; katlı ve katsız nesne birlikte `module_levels`;
  güncelleme kat değişikliğini taşır, Modüle ekle üst kattaki nesneyi o katın üyesi yapar; `cad.modules.query`'nin `storeys`'i
  ve örneğin `levels`'ı. `repeat`'in `loose`'u: kaynak katın modülsüz duvar, açıklık, mahal, donatı, ölçü ve etiketleri düz
  kopya olarak her hedefe, bağları kopyalara ve tekrarlanan örneklerin nesnelerine çevrilir, mahal numarası hedefin
  numbering'iyle ya da katın önekiyle; kalanlar `report.left` (`module`, `option`, `locked`, `kind`, `outside`) ve `module_repeat_left`,
  cevabın `copied`'ı; tek geri alma adımı. Oda modülleri'nde “n katlı”, örneğin katları ve Modülsüz nesneler de. 38 + 23 ortak
  durum (`module_storeys_cases.py`). Gerçek pencerede elle denenmedi.

- **Katları tekrarla, hedefte aynısı olanı yeniden kopyalamaz (1 Kasım 2026; [ADR 0810](../../docs/adr/0810-loose-repeat-skips-what-is-there.md), ADV-05 [~]).**
  `repeat`'in `loose` kopyası, hedef katta modülsüz ve seçeneksiz eşit bir nesne varsa (belgenin yazdıkları dışında,
  `same_object`) yapılmaz; bağlar aynılarına gider, aynısı yeni bulunan kalmayana dek aranır. `report.matched` ve
  `module_repeat_matched`; `again` yine kopyalar; ne örnek ne kopya varsa `module_repeat_taken`. Oda modülleri'nde
  Hedefte aynısı olanı da kopyala, gövde kayar. Şema değişmez. 23 ortak durum (`module_repeat_matched_cases.py`).

- **Oda modülünün tek katı (1 Kasım 2026; [ADR 0811](../../docs/adr/0811-single-storey-module-instances.md), ADV-05 [~]).**
  Archicad'in Single Story'si: `place`'in `storey`'i çok katlı modülün yalnız o katının üyelerini verilen katta
  yerleştirir (üstünde kat gerekmez; tek katlı modülde, son katın ötesinde ve katsız `invalid_input`); örnek
  `storey`'ini tutar (`.kcad` şema 155, opt-in). Kat taşımayan üyenin katı dayandığınınki (`modules_more::floor_of`).
  Güncelleme örneğe yalnız tuttuğu kattaki üyeleri götürür; başka kata geçen üyenin nesnesi kalır (`why: storey`,
  `module_storey_left`), kata gelen üye eklenir; Modüle ekle yalnız onun katından; tekrar aynı katla. Modül yerleştir'de
  Tek kat (K), Oda modülleri'nde “yalnız modülün n. katı”. 23 + 20 ortak durum (`module_single_storey_cases.py`).

- **Tasarım değişikliği yalnız ulaştığını okur (31 Ekim 2026; [ADR 0782](../../docs/adr/0782-design-change-reads-only-what-it-reaches.md), ADV-07 [~]).**
  Bağlı döşemenin sınırı katın bütün duvarlarından değil, tohumu çevreleyen duvar parçalarından (erim kutuları 1 m'den
  yakın duvarlar bir parça; tohumu tutan, onlara değen ve ilk duvarın parçası) bulunur, bindirmenin başlangıcı aynı
  kaldığından sonuç bit bit aynı (`kentos_domain::wall_pieces`, `slab_from_every_wall`); yerleri dışında aynı çizelgeler
  adımda bir kez çıkarılır (`follow_tables`'ın `alike_but_placed`'i). 100 dairede Mesafe 1 151 → 38,7 ms, öznitelik
  544 → 14,8 ms, değişiklikte bellek +%4,4 (binanın iki çizelgesiyle); her dairenin çizelgeleriyle (200) komut 81,5 ms,
  karenin 340 ms'si ve belleği yazı katmanının bütün görünümü yeniden kurmasıdır (açık). Rastgele 48 düzenlemede parça
  ile bütün kat aynı (`flat_pieces`).

- **Model alanında kalem kalınlığı ve birlikte kopyalanan bağlar (31 Ekim 2026; [ADR 0793](../../docs/adr/0793-model-pen-widths-and-copied-links.md), ADV-07 [~]).**
  Çizim alanında kâğıt kalemi (çizgi ve tarama genişliği, mm) çizim ölçeğinden derine yakınlaşınca kâğıttaki genişliğini
  korur, dünyayla şişmez; uzaklaşınca incelir (en az 1 px): stil bloğunun `origin.z`'si, WGSL `weightK`, düzen sözleşmesi 9;
  `Look::hold_weights` yalnız çizim alanında; sembolün yere ölçülen genişliği (`m`) tutulmaz (`Batches::ground`, stilin
  JSON'u aynı). Kopyala, Dizi ve Yapıştır birlikte kopyalanan nesneye bağı kopyaya geçirir (`kentos_domain::copy_links`):
  etiket her üçünde, ölçünün bağları ve tablonun nesneleri Yapıştır'da. 21 + 24 + 24 ortak durum (`copy_links_cases.py`),
  gerçek GPU testi `paper_widths`, resimler `arac-kalite-*`, `arac-kalite-etiket-kopya`.

- **Yazı katmanı nesne nesne (31 Ekim 2026; [ADR 0786](../../docs/adr/0786-text-layer-object-by-object.md), ADV-07 [~]).**
  Gerçek boy ve kâğıt kuralında görünümün yazısı nesne nesne parçalardır (`Store::labels_shown_parts`); `LabelMemo`
  (deponun `Item::stamp`'i, blok parçaları, tablonun hücre kutusu) değişmeyen nesnenin parçasını yeniden okumaz, çizelge
  64 satırlık öbeklerle (`TableGeom::cells_near`, `cells_near_in`) ve görünümün yakınında okunur; masaüstünde
  `Spatial::label_parts`, şeritler parça parça (`BandMemo`) ve tek ağ (`band_mesh`); etiket motoru her soruda bütünüyle
  yerleşir; CBS'nin büyüyen yazıları tek parça. `perf::flat` değişikliği izler (`follow_document`). 100 daire 200
  çizelgede tasarım değişikliğinin karesi 351 → 163 ms (komut dışı 268 → 81), bellek değişiklik başına +3 062 → +191 MB;
  sonuç baştan kurmayla bit bit aynı (`label_memo`, `labels::parts_tests`). Model alanında kalem kalınlığı açık.

- **2+1 dairenin ölçülen kalite kabulü (31 Ekim 2026; [ADR 0776](../../docs/adr/0776-measured-quality-acceptance-of-an-apartment.md), ADV-07 [~]).**
  Aynı daire üç yoldan: masaüstünde gerçek klavye ve fare olaylarıyla iz (`acceptance-flat.json`, 167 adım, üç çeşit;
  oynatıcıya `clickNumber`: seçili duvarın geçici ölçüsüne tıklar), bölüm bölüm tık/tuş/değer/odak/kurtarma ölçüsü
  (`scripts/acceptance/trace_metrics.py`); Python, MCP ve masaüstünün ajan bağlantısıyla aynı anlamsal akış
  (`fixtures/acceptance/v1/flat.json`, 27 adım, beklenenler `acceptance_flat.py`'de KentOS kodu olmadan; her yazan adım
  geri alınıp yinelenir, iki ret aynı kod, kaydet-aç sonrası aynı kimlikler; `--compare` üç çizimi bağımsız KCAD
  okuyucusuyla aynı bulur); büyük koordinatta milimetre, ölçek/birim/yakınlık ayrımı testleri ve 1:20/1:50/1:100
  resimleri (`quality_scenes.rs`); 1, 20 ve 100 dairede kare, önizleme, seçme, değişiklik, Metraj ve bellek
  (`perf::flat`). Düzeltilen: ölçü mahal ve döşemede alan verir (`query::item`), Metraj'ın boş toplamı +0, duvar
  gövdesinin içine tıklamak duvarı bulur ve odanın yüzdeki kenarından önce gelir (`store/pick.rs`), Döşeme isteminde Kaynağa bağlı (B). Açık: 100 dairede tasarım
  değişikliği 1,86 s (karesel, iptalsiz), değişiklik sırasında bellek 1,7 → 3,2 GB, model alanında kalem kalınlığı dünya biriminde (ADR 0793 kapattı)
  (derin yakınlıkta 1 mm ayrıntı banda karışır). Gerçek pencerede elle kabul,
  acemi kullanıcı ve araç bulma süresi ölçülmedi; pafta GUI'de oynatılmadı.

- **Üç ve dört kanatlı sürme, paftanın PDF metni (31 Ekim 2026; [ADR 0792](../../docs/adr/0792-multi-leaf-sliding-doors-and-pdf-text.md), ADV-03d [~]).**
  Sürme kapı ve pencere 1–4 kanat; yapının `tracks`'i (2/3, yalnız 3–4 kanatta) ve dört kanatta `bothSides` (ortadan iki
  yana). Kanatlar toplandıkları pervazdan k'ıncı rayda, raylardan sonrakiler sabit (`slide_panels`, `geom::opening::slide_leaves`):
  OXO, OXX, OXOO, OXXO; komşu raylar `overlap` kadar biner, net geçiş toplanınca kalan en geniş boşluk. Planda kanat bantları,
  toplandığı pervaza oklar, raylar; `door_zones` boş. `sizes`'ın `tracks`, `panels`'ı, çizelge Ray sayısı ve Kanat düzeni,
  etiket `{tracks}`, `{panels}`; Kapı aracında Ray sayısı (V) ve İki yana (İ), Öznitelikler ve tip penceresinde aynısı.
  `.kcad` şema 150 (opt-in). 1:50 paftanın PDF'i `pdftotext -bbox` ile okunur: numara etiketinde ve satırında birer kez,
  2,5 mm, kırpmanın dışında yazı yok. 22 + 23 + 20 + 22 + 21 ortak durum (`special_doors_cases.py`,
  `special_doors_command_cases.py`). XOX, eşit olmayan kanat ve gerçek pencere kabulü açık.

- **XOX ve eşit olmayan iki kanatlı sürme (1 Kasım 2026; [ADR 0807](../../docs/adr/0807-xox-and-unequal-sliding-leaves.md), ADV-03 [~]).**
  Üç kanatlı sürmenin `bothSides`'ı XOX: ortadaki kanat 0. rayda sabit, iki yandakiler 1. rayda ortaya kayar, kanat
  (ışık + 2b)/3, net geçiş w − b, yalnız iki rayla; iki kanatlı sürmenin `split`'i birinci kanadın payı (kanatlar
  pay·ışık + b/2, net geçiş ışık − büyük kanat, planda `two_leaf_meet`). Kapı'da İ “Ortası sabit (XOX)”, Ş “Birinci
  kanadın payı”; Öznitelikler'de aynısı ve Kanatlar. `.kcad` şema 153 (opt-in). 22 + 20 ortak durum
  (`special_doors_command_cases.py`), 8 çizim durumu (`special_doors_cases.py`). Dört kanatta eşit olmayan kanat açık.

- **Tasarım seçeneklerinin bileşimleri (1 Kasım 2026; [ADR 0808](../../docs/adr/0808-design-option-combinations.md), ADV-05 [~]).**
  Projenin `arch.optionCombos`'u (Archicad'in Design Option Combinations'ı): adlı bileşim, kümelerin birer seçeneği,
  anmadığı küme birincilini gösterir. `cad.options.edit`'in `saveCombo` (seçimsiz şimdiki birincilleri tutar, aynı ad
  seçimleri değiştirir), `applyCombo` (seçenekleri birincil yapar), `deleteCombo` (tarifte kullanılan
  `option_combo_in_use`); Kabul et ve Seçeneği sil kalkan seçimi düşürür (`option_combo_dropped`). Tarifin `combo`'su
  kendi seçmediği kümelerde bileşimi gösterir (`drawn_views`); şablon taşımaz. `cad.options.query`'nin `combos`'u
  (seçimler, şimdiki mi, planın nesne, alan, duvar boyu). `.kcad` şema 154 (opt-in). Tasarım seçenekleri'nde Bileşimler,
  Görünümler'de Bileşim. 33 + 21 ortak durum (`option_combo_cases.py`). Karşılaştırma ADR 0809'da.

- **İki bileşimin karşılaştırması (1 Kasım 2026; [ADR 0809](../../docs/adr/0809-comparing-option-combinations.md), ADV-05 [~]).**
  `cad.options.query`'nin `compareCombos`'u (`a`, `b` kimlik ya da adla; `search`, `tolerance`): iki bileşimin ayrı
  gösterdiği her küme iki seçenek gibi karşılaştırılır (`combosCompared.sets`, Veri karşılaştır'ın konum kuralı), aynı
  gösterilenler `alike`, iki planın nesneleri ve alan, duvar boyu farkı (b − a); olmayan `option_combo_not_found`,
  kendisiyle `invalid_input`. Bileşimler'de seçilen bileşim bir başkasıyla Karşılaştır; pencerenin gövdesi kayar (dar
  ekranda düğmeler görünür). Şema değişmez. 26 ortak durum (`option_combo_compare_cases.py`).

- **Kabul korpusunun öbür dört örneği (1 Kasım 2026; [ADR 0798](../../docs/adr/0798-acceptance-corpus-samples.md), ADV-07 [~]).**
  Ölçülü değişiklik, iki katlı konut, eğrisel/şaftlı örnek ve GDAL'ın DXF altlığında tadilat; her biri senaryo
  (`fixtures/acceptance/v1/<id>.json`, 20–22 adım, beklenenler `acceptance_corpus.py`'de KentOS kodu olmadan), Python, MCP ve
  masaüstünün ajan bağlantısıyla aynı (`--compare` üç çizimi bağımsız okur), gerçek girdiyle iz (`acceptance-<id>.json`,
  oynatıcıya `clickLock`) ve `perf::corpus`. Düzeltilen: mahalin sınırı tadilat durumunda duranlar (`space_follow::phase_bounds`; var
  olan oda yeni duvarla, yeni oda yıkılacakla sınırlanmaz), döşemeden iç noktayla Kes öntanımlı oda, Kaynağı koru'yla kilitli
  altlıktan Çizgiden duvar (araç duvarları etkin katmana yazar), açıklığın boşluğuna tıklamak açıklığı seçer. Açık: araçlarda
  tadilat durumu, duvar genişliğinin araçlar arası paylaşılması, `settings.set`'in `arch`'ı bütün, etiket motorunun artımlı yerleşimi.

- **Özel kapıların çarpışması, yaylı duvarda cep, cebin metrajı (31 Ekim 2026; [ADR 0787](../../docs/adr/0787-special-door-clashes-arc-pockets-pocket-quantities.md), ADV-03c [~]).**
  `geom::opening::door_zones` (kanat dilimi, döner tambur, katlanır panellerin alanı, raylı sürmenin yolu); `cad.arch.audit`'in
  Kapılar grubu hepsini sınar (`arch_door_swing_clash`, ileti neyin çarptığını söyler), düzeltmesi Çevir'in ilk temiz
  çevirmesi (öbür yüz, menteşe yanı, ikisi; döner kapıda ve kilitli katmanda yok). Cebe giren kapı yaylı duvarda: cep ve
  `opening_pocket_short` yay boyunca, kanatlar merkez çevresinde kavisli bant (`arc_pocket_leaves`). `cad.spaces.quantities`'in
  duvar satırında `pocketsVolume` (boy × en × kapı yüksekliği, yayda duvarın ortasına ölçekli) hacimden düşülür, yüzeyler
  düşmez; Metraj'da Cep hacmi. 22 + 21 + 23 + 20 ortak durum (`door_clash_cases.py`, `arc_pocket_cases.py`). Üç-dört kanatlı
  sürme, nişin hacmi ve gerçek pencere kabulü açık.

- **Döner, cebe giren ve katlanır kapı; kayar kapının rayı (31 Ekim 2026; [ADR 0777](../../docs/adr/0777-revolving-pocket-folding-doors.md), ADV-03b [~]).**
  Açılışın `revolving` (3–4 kanat; `build.drum` tamburun iç çapı, ışıktan büyük değil; plan: tambur, iki tambur duvarı yayı,
  kanatlar; net geçiş D·cos(180°/n)), `pocket` (1–2 kanat; `build.lip`; cep duvarın ortasında kanat + 2 cm, boyu
  w − lip − fit − kasa + 1 cm; duvar cep + 2 × 2,5 cm'den ince `opening_pocket_thin`, cep ucu/köşeyi ya da başka açıklığı
  aşarsa `opening_pocket_short`; planda duvarın içinde oyuk, pervazda açık, `geom::opening::holes`), `folding` (2–6 panel,
  `build.bothSides` çift panelde; zikzak, plan 30°; net geçiş ışık − n·tl); sürmenin `build.overlap`'ı ve `build.track`'i
  (tek kanat yüzde rayda). Kapı tipleri, istisna ve izleme raporu aynı; çizelge/etiket `operation`, `leaves`, `drum`,
  `pocket`; `sizes`'ın `drum`, `pockets`'i. Kapı aracında Açılış (Ç), Kanat (K), Tambur (J), Cep payı (P), İki yana (İ),
  Bindirme (S), Ray (H); Öznitelikler ve tip penceresi açılışa göre. `.kcad` şema 144 (opt-in), `FORMATS_VERSION`
  103. DXF blok görünüşüyle (CIRCLE, ARC, LINE), 1:50 PDF `pdftoppm` ile. Döner/katlanır kapının çarpışma denetimi,
  yaylı duvarda cep ve gerçek pencerede deneme açık.

- **Görünüş şablonu ve kalem seti (31 Ekim 2026; [ADR 0783](../../docs/adr/0783-view-templates-and-pen-sets.md), ADV-06 [~]).**
  Görünüş tarifinin `pens`'i (14 türün kâğıt kalınlığı ve rengi; `PenCategory::of_kind`), `mono`'su (çizgiler siyah, taramalar
  gri; kalemin rengi önce) ve `template`'i; projenin `settings.viewTemplates`'i (en çok 50; kimlik `t-` ve adın FNV-1a'sı, kat,
  kırpma ya da şablon taşımaz). `cad.views.edit`'in `template` (kaydet, değiştir, kaldır: bağlı görünümler alanları kendi
  tarifleri olarak tutar `template_detached`) ve `templateRename`'i; bağlı tarifte yalnız kat ve kırpma görünümündür.
  `resolved_named_views` tek çözüm: pafta, başsız kitap, `cad.sheets.set` ve gezgin şablon değişince birlikte yeniden çizer;
  kalem ve siyah beyaz `map_phasing_of`'tan ekran, kitap ve PDF'e. Görünümler penceresinde Şablon, Şablondan satırı, Siyah
  beyaz, kalem kalınlıkları, Şablon olarak kaydet; gövde kayar. `.kcad` şema 146 opt-in. 29 + 20 ortak durum
  (`view_template_cases.py`). Pafta kipinde ekrandaki etiketlerin yeniden renklenmesi, koşullu kalem ve gerçek pencere kabulü açık.

- **Görünüş tarifinde duvar ayrıntısı ve kırpma (31 Ekim 2026; [ADR 0781](../../docs/adr/0781-view-recipe-wall-detail-and-crop.md), ADV-06 [~]).**
  Tarifin `detail`'i (Şematik, Uygulama, Ayrıntılı; Revit'in görünüş Detail Level'ı) ölçeğin ve projenin ayrıntısından önce gelir
  (`walls::view_with`); bağlı haritanın `MapPrim.wall_detail`'i, masaüstünün ve başsız kitabın haritası deposunu bununla kurar
  (`Spatial::set_wall_detail`, `walls_differ_with`, anahtarda ayrıntı). `crop` bağlı haritayı görünümün alanına kırpar
  (`viewport::view_ring`, `feature_clip`; atlas nesnesi önce); `cad.sheets.set`'in `shows`'u alanın kutusunu da ister. `.kcad`
  şema 143 opt-in. Görünümler penceresinde Duvar ayrıntısı ve Alanına kırp. 33 + 22 ortak durum (`view_recipe_cases.py`,
  `view_recipe_crop_cases.py`). Kırpma çerçevesinin ekranda düzenlenmesi, kalem seti ve görünüş şablonu açık.

- **Kırpma çerçevesi (31 Ekim 2026; [ADR 0789](../../docs/adr/0789-crop-frame-on-screen.md), ADV-06 [~]).** `cad.views.edit`'in
  `area`'sı adlı görünümün ortasını, enini, boyunu ve isteğe bağlı dönüşünü yazar; kimlik, ad, yer ve tarif kalır; tek geri alma
  adımı “Görünümün alanı

- **Türetilmiş çizim: oda iç görünüşü (31 Ekim 2026; [ADR 0799](../../docs/adr/0799-derived-room-interior-elevations.md), ADV-06 [~]).**
  Bir mahalin her düz yanı odanın içinden, yalnız modelin bildiğiyle 2B çizilir: yüzü o yanda olan duvarlar, yükseklik (mahalin,
  yoksa duvarların en alçağı), açıklıklar denizlik ve lentolarıyla (lentosuz kesikli), numaralar, başlık ve iki ölçü; kendi
  katmanında (İç görünüşler grubu), tek adım “Oda iç görünüşü”. Kayıt katmanın türetim kaydında (ADR 0633; mahal, yanlar, rol →
  kimlik, özet; şema değişmez). `cad.elevations.edit` (derive, refresh: aynı kimliklerle, release) ve `cad.elevations.query`
  (current, stale, gone; missing, edited); yaylı yan, boşluk, kolon, merdiven, rampa, donatı, baca, kopuk açıklık çizilmez,
  `leftOut`'ta. İşaret başvurudur, çizim üretmez: araç İşaret ▾'de değil, Belgele ▾'de. Öznitelikler'de İç görünüş bölümü
  (Durum, Yeniden üret, Bağı kopar). 35 + 22 ortak durum (`room_elevation_cases.py`). Kanat çizimi, döşeme/tavan kesiti,
  2,5B kesit ve gerçek pencere kabulü açık.

- **Görünümün alanını kesen haritanın ön denetimi (31 Ekim 2026; [ADR 0794](../../docs/adr/0794-map-view-cut-preflight.md), ADV-06 [~]).**
  Pafta ön denetiminin `map_view_cut` uyarısı: bağlı görünümün tarifi ölçeği veriyorsa ve alanı o ölçekte çerçevenin
  içeriğinden büyükse taşan yön ve miktarıyla söylenir (`kentos_sheet::preflight::view_overrun`); ölçeksiz tarif sığdırılır,
  uyarı yok. ADR 0789'un 1:20 incelemesinin açığı; düzeltme kendiliğinden yapılmaz.

- **Proje gezgini (31 Ekim 2026; [ADR 0778](../../docs/adr/0778-project-navigator.md), ADV-06 [~]).**
  Salt okunur `cad.navigator.read`: katlar (nesne sayısı, tarifi o katı çizen görünümler, o katı çizen haritalar), `unleveled`,
  adlı görünümler (tarifin katı, sürümü, ölçeği, bağlı haritalar), paftalar (`cad.sheets.set`'in çerçeveleri), çizelgeler
  (üye sayısı); `uid` ile nesnenin göründüğü yerler: katı, onu çizen görünümler (alanın dönük kutusu ve tarif), haritalar
  (`shows`), onu listeleyen çizelgeler, etiketleri. Kural tek yerde (`sheets_set::recipe_draws`). Masaüstünde Gezgin paneli
  (`Panel::Navigator`, kentos-cad ve Mimari; `arch.navigator`): satır kata etkin kat, görünüme göster, paftaya aç, çizelgeye
  seç ve göster; seçili nesne bölümü üstte. Şema değişmez. Elle gerçek pencerede denenmedi.

- **Görünüş tarifi (31 Ekim 2026; [ADR 0774](../../docs/adr/0774-architectural-view-recipes.md), ADV-06 [~]).**
  Adlı görünümün `recipe`'i (`.kcad` şema 141 opt-in): kat (alt kat soluk), tadilat görünümü, katman durumu, gizlenen türler (ölçü, etiket,
  yazı, tarama, blok, mahal, aks, görünüm işareti) ve ölçek; sürümlü, `cad.views.edit`'in `recipe`'i yazar ya da kaldırır
  (`invalid_recipe`, `level_not_found`, `state_not_found`). Görünüme bağlı paftanın haritası (ADR 0599) tarifle çizer: ölçeği,
  katı, katman durumu, tadilat görünümü ve gizlenenleri (ekran, başsız kitap, PDF; `linked_recipe`, `map_phasing_of`); model ve ekranın kat
  görünümü değişmez. `cad.sheets.set`'in çerçevesi `view`, `missing`, `level`, `recipe` söyler, `shows` haritanın çizdiğini sayar.
  Görünümler penceresinde Görünüş tarifi ve beş hazır tarif. Proje gezgini ADR 0778; kalem seti, türetilmiş çizim açık. 25 + 22 ortak durum
  (`view_recipe_cases.py`, `view_recipe_sheet_cases.py`).

- **Bina ve kanat penceresi, ortak alan payı (31 Ekim 2026; [ADR 0784](../../docs/adr/0784-building-wing-window-and-common-area-shares.md), ADV-04 [~]).**
  Bina ve kanadın `levels`'ı (kanadınki binasınınkilerden); `cad.program.edit` kaldırılan bina ya da kanat bölümlerde
  yazılıysa `building_in_use` (numaralarıyla), projede olmayan kat `level_not_found`; denetimin `program_level_outside`'ı.
  Programın `shareRule`'u (`equal`, `net`, `weight`; bölümün `shareWeight`'i tam ondalık metin), kural `kentos.share/1`:
  her ortak alan kendi havuzu (kanadı, yoksa binası, yoksa bütün program), 0,01 m² adım (tam ikili değer, yarıda çift),
  `⌊T·w/W⌋` ve kalan adımlar en büyük kalanlara (eşitlikte programdaki sıra), paylar havuza tam eşit; paylaştırılamayan
  havuz nedeniyle (`share_unshared`). `cad.program.check`'in `shares`'ı ve bölüm satırının `share`'i (net, pay, brüt;
  süzgeçten bağımsız, ondalık metin), etiket `{share}`, `{gross}` (hazır “Bölüm payı”), çizelge Pay/Brüt, Öznitelikler.
  Bina programı'nda Binalar sekmesi, Bölümler'de Bina/Kanat/Pay ağırlığı, Ortak alan payı, Durum'da Paylar. `.kcad`
  şema 147 (opt-in). 20 + 22 ortak durum (`program_share_cases.py`). Resmî alan profili, arsa payı, `of: units` çizelgesi
  ve gerçek pencere kabulü açık.

- **Bölüm çizelgesi (31 Ekim 2026; [ADR 0795](../../docs/adr/0795-unit-schedule.md), ADV-04c [~]).**
  Çizelgenin `of: units`'i: bina programının her bölümü (bağımsız bölüm, ortak alan, dış alan) bir satır, programın sırasıyla;
  satırın `members`'ı bölümün mahalleri, `units`'i bölümün kimliği (mahalsiz bölüm de satır). Sütunlar Bölüm türü, Bina, Kanat,
  Kayıtlı alan, Pay ağırlığı ve No, Ad, Kat, Alan (her mahal bir kez), Pay, Brüt (`kentos.share/1`), Tasarım hedefi, Hedef farkı,
  Mahal sayısı; Pay, Brüt ve Kayıtlı alan gruplarda ve toplamda toplanır (`ScheduleField::summed_in`). Süzgeç kat, bina, kanat ve
  tür (`building_not_found`). No, Ad, Kayıtlı alan ve Pay ağırlığı hücreleri `cad.schedules.edit` ile programa yazılır (tek adım,
  programın reddi `invalid_cell_value`); öbürleri `cell_computed`. Mahal, program ya da kat değişince aynı adımda yeniden yazılır.
  Mimari › Çizelge ▾ › Bölüm çizelgesi, Çizelge ayarları'nda süzgeçler. `.kcad` şema 151 (opt-in). 25 + 25 + 20 ortak durum
  (`unit_schedule_cases.py`). Balkon sütunu, resmî alan profili ve arsa payı açık.

- **Bina programı: bağımsız bölümler, tasarım hedefi ve geometriyi izleyen denetimler (31 Ekim 2026; [ADR 0773](../../docs/adr/0773-building-program-units-and-design-brief.md), ADV-04 [~]).**
  Mimari ayarların `program`'ı (bina ve kanat, bağımsız bölüm/ortak alan/dış alan, kullanıma göre tasarım hedefi: alan aralığı,
  net en, kapılar arası net geçiş, komşu ve geçilen kullanımlar, dış mahal; donatının kullanım alanı; hedeflerin sürümü) ve
  mahalin `unit`'i (`.kcad` şema 140); kat ile bölüm zorunlu ağaç değil (dubleks iki kat), katman adı anlam taşımaz.
  `cad.program.edit` (program + `assign`, tek adım “Bina programı”, mahalli bölüm `unit_in_use`), `cad.spaces.edit`/`create`'in
  `unit`'i. Salt okunur `cad.program.check` (uzun iş, durdurulabilir): bölüm toplamları her mahali bir kez sayar, mahal başına
  alan, net en, net geçiş ve en dar yeri, hedef farkı, komşu/geçiş; bulgular Mimari denetim'in `program` grubu (kural
  `kentos.program/1`, yer santimetrede; 1 mm istisnayı bozmaz, 10 cm açar). Böl/Birleştir'in `decisions`'ı (kimliği süren,
  numara, kaplama, etiket, donatı) yazmadan söylenir. Etiket `{unit}`, `{target}`, `{targetDiff}`, çizelge Bölüm/Tasarım
  hedefi/Hedef farkı, Öznitelikler'de Bölüm; Mimari › Proje › Bina programı (Bölümler, Hedefler, Durum; Denetle iş
  parçacığında, Durdur). Hedef yasal sınır değil; bina/kanat penceresi, pay hesabı ve gerçek pencere kabulü açık.

- **Kapı ve pencere tipleri, örnek istisnası ve net geçiş (31 Ekim 2026; [ADR 0772](../../docs/adr/0772-door-window-types-instances-clear-width.md), ADV-03 [~]).**
  Projenin tipleri `settings.arch.openingTypes` (`.kcad` şema 139 opt-in); açıklık tipi kimliği, sürümü (`typeVersion`) ve
  istisnalarıyla (`overrides`) izler. `cad.openingtypes.edit` (tanımla, değiştir, adlandır, sil, kitaplıktan al ve yaz;
  izleyenler tek adım, rapor applied/kept/skipped/refused), `cad.openings.edit`'in detach/push/reset/refresh'i,
  `cad.openings.query` (Aynı tiptekileri seç). `width` kaba boşluk kalır; kasa dış eni, kanat eni ve 90°'de net geçiş
  hesaplanır, çizelge ve etiketlerde. İsteğe bağlı `build`: montaj boşluğu, kalın ve eşit olmayan kanat, kasa derinliği,
  eşik, pervaz, denizlik, kayıtlar, kesikli kullanım alanı. `cad.arch.audit`'in Kapılar grubu (fiziksel, net geçiş, proje
  kuralı; `minClearWidth`), mutfak dizisi kapının dilimini söyler. Masaüstünde Kapı ve pencere tipleri penceresi, Açıklık
  sekmesinde Tip paneli, Öznitelikler'de Tip bölümü. Döner, cebe giren, katlanır kapı ve yatay kaydın çizimi açık; elle gerçek
  pencerede denenmedi. 9 Kasım eki: Yeni tip ▾ önce biçimi sorar (Kapı, Pencere, Boşluk, Niş, Baca) ya da seçili tipi
  kopyalar; önceden hep seçili tipin kopyasıydı ve pencere tipi açmanın yolu görünmüyordu.

- **Malzeme kitaplığı, Duvar stilleri'nde malzeme seçicisi, döşemenin mahale bağlanması (31 Ekim 2026; [ADR 0791](../../docs/adr/0791-material-library-style-picker-space-binding.md), ADV-02c [~]).**
  Çizimin dışında malzeme kitaplığı `kentos.materials` 1 (`.kmalzeme`; KentOS'un `catalog/v1/materials/kentos-temel.json`,
  21 yaygın yapı malzemesi). `cad.materials.edit`'in `import`'u (library ya da name `kentos-temel`, materials, onConflict
  refuse/keep/replace; değiştirilen malzemenin bağlıları aynı adımda izler; `report.taken`, `replaced`, `kept`, uyarı
  `material_kept`; bozuk kitaplık `invalid_library`) ve `export`'u (cevabın `library`'si, çizim değişmez).
  `cad.styles.edit` duvar stilinin bağlı bileşenine malzemenin görünüşünü verir, olmayan malzemeye `material_not_found`.
  Malzemeler › Kitaplık (klasörler `malzeme-kitapliklari.json`, projedeki durum, koru/değiştir, plan, Kitaplığa yaz…);
  Duvar stilleri'nde Malzeme seçicisi (bağlı satırın taraması ve kalemi salt okunur); Öznitelikler'in Sınır satırında
  katın mahalleri (“Mahale bağlı · 101 Salon”). `.kcad` değişmez. 30 + 21 ortak durum (`material_library_cases.py`,
  `style_materials_cases.py`). IFC, kitaplık kaynağının izlenmesi ve gerçek pencere kabulü açık.

- **Malzeme kimliği ve doğramaya kadar dönüş (31 Ekim 2026; [ADR 0785](../../docs/adr/0785-material-identity-and-wrap-to-frame.md), ADV-02b [~]).**
  Projenin malzemeleri `settings.arch.materials` (kimlik, ad, kategori Taşıyıcı/Yalıtım/Kaplama/Membran/Diğer, metraj alan ya
  da hacim, kesitin taraması ve kalemi; en çok 256); duvar bileşeni ve döşeme `materialId` ile bağlanır, bağlının metni,
  taraması ve kalemi malzemenindir. `cad.materials.edit` (define, update, remove: bağlıysa `material_in_use` ve `uids`;
  assign: wallStyle/wall/slab, yoksa bağdan ayırır; adopt: Metinlerden al, görünüşü farklı aynı metin bağlanmaz,
  `material_adopt_left`), tek adım “Malzeme”, kilitli katmandaki eski değerinde (`material_locked`). Malzeme lejandı ve
  Metraj (`materials`, Malzemeler sekmesi) kimlikle gruplar. Dönen katmanlar kasası derin kapı ve pencerenin sövesinde
  kasanın yüzüne kadar döner (`HoleFrame`, `wall_wrap::plan_framed`; sığmazsa `m`). Malzemeler penceresi, Öznitelikler'de
  Malzemeler bölümü. `.kcad` şema 148 (opt-in). 34 ortak durum (`materials_cases.py`), 5 dönüş durumu
  (`wall_join_cases.py`). Duvar stilleri'nde malzeme seçicisi, malzeme kitaplığı, IFC ve gerçek pencere kabulü açık.

- **Yapı bileşenleri ve kaynağa bağlı sınırlar (31 Ekim 2026; [ADR 0771](../../docs/adr/0771-building-parts-and-bound-boundaries.md), ADV-02 [~]).**
  Düğüm kararı duvarın ucunda (`joins.startPass`/`endPass`: T'de her bileşenin konağın kaç bileşenini geçtiği; null önceliklere
  döner); köşe, düz devam, serbest uç ve eğrisel birleşimde `wall_node_unsupported`; `cad.walls.joins`'in `nodes`'u her ucun
  `decided`'ını ve T'nin `passes`'ını verir. Odanın köşesinden kenarın doğrultusunda çıkan uç öbür kenara T birleşir
  (`host_tangent`; Python başvurusu ile çekirdek aynı). `cad.walls.edit`'in `set` ve `replaceStyle`'ının `keep`'i (Yol, Eksen,
  Sol/Sağ bitmiş yüz, Sol/Sağ çekirdek yüzü) kalınlık ya da stil değişince ötelemeyi kaydırır; planın `effects`'i (ADR 0770'in
  `WallEffects`'i) mahal alanları ve açıklık derinlikleri (`depths`). Bileşenin `wrap`'ı (Döner): dış katmanlar serbest uçta ve
  boydan boya açıklığın sövesinde döner (`geom::wall_wrap`). Döşemenin `bound`'u (duvarlar: iç nokta + yüz kuralı, ya da mahal;
  öteleme; kopuk `lostEnclosure`, `sourceGone`; `arch_slab_broken`, Düzelt: Ayır); `cad.slabs.create`'in `bound`/`space`'i,
  `cad.slabs.edit`'in `bind`/`detach`'i, bağlıda `subtract`/`add` `slab_bound`. Öznitelikler: Kalınlıkta sabit, Baş/Son düğümü,
  Sınır; Döşeme aracında Kaynağa bağlı (B); Duvar stilleri'nde Döner. `.kcad` şema 138 (opt-in). Malzeme kimliği ve doğramaya
  kadar dönüş açık; gerçek pencerede elle denenmedi.

- **Bağlantılı odalar ve kalıcı mimari bağlar (31 Ekim 2026; [ADR 0770](../../docs/adr/0770-linked-rooms-and-architectural-bonds.md), ADV-01 [~]).**
  `cad.walls.rectangle`'ın `adopt`'u: kenarın referans çizgisine (İç net: odaya bakan yüz, Eksen, Dış ölçü: dış yüz) 1 mm
  içinde düşen aynı kattaki paralel duvarın gövdesi benimsenir, kalanlar açık duvarlar (`ops::wall_adopt`; hepsi benimsenirse
  `nothing_to_set`); araçta Ortak duvar (O). Kaydır'ın köşesinde aynı doğrultudaki duvar ucu izler. `cad.walls.edit`'in
  Kaydır, Mesafe ve Boy'unda `effects` (iki yanın net ölçüsü önce/sonra, mahal alanları, açıklıklar, ölçüler, donatı, bağların
  kaydırdığı duvarlar; okuma kopyasında adımın tamamı); Mesafe'nin `keep`'i (Bu ölçüyü koru) aynı adımda bağ yazar, mesafe
  değişmese de. Kısıtın parçasında duvar çizgisi (`wall`: kenar, `leftFace` … `rightCore`) ve pervaz (`jamb`); mesafe, paralel,
  eşit aralık, sabit; mimari bağdaştırıcı (`ops::arch_bonds`, `arch_bonds.rs`) kenarı kendine dik, açıklığı duvarı boyunca
  en az değişiklikle kaydırır, kaydırmayı Kaydır'ın kuralıyla uygular; tutulanlar yazmanın oynattıkları, kilitli katman, eşit
  aralığın uçları; çelişki `constraint_conflict`. Masaüstünde yazı kutusu yazıldıkça araca (`Tool::text_draft`): Duvarın
  mesafesi etki kartı; sayının üstünde kilit karesi (bağla / bağı kaldır), bağlı yanın Enter'ı parametreyi değiştirir.
  `.kcad` şema 137 (opt-in), `FORMATS_VERSION` 100. 31 ortak bağ durumu (`arch_bond_cases.py`), 26 benimseme. Gerçek
  pencerede elle denenmedi; çelişen bağın çizimde vurgusu ve 20 ms bütçesi açık.

- **Yoldaki bağlar ve Bağı göster/kaldır (31 Ekim 2026; [ADR 0780](../../docs/adr/0780-bonds-in-the-way.md), ADV-01 [~]).**
  `constraint_conflict`'in yeni `bonds`'u (`CommandError.bonds`, `BondInTheWay`): çelişen her mimari bağın parçaları,
  tuttuğu (`kept`) ve yazmanın bıraktığı (`measured`) değer, tek başına kaldırılınca yazmayı geçirip geçirmediği
  (`frees`); ileti geçirenleri adlandırır. `cad.walls.edit`'in planı yürütmeyle aynı adımı (`run_step`) açar: bağlar
  önizlemede izler ve reddeder (önceden izlemiyordu). `cad.constraints.query`'nin `ids`'i, `architectural`'ı ve satırın
  `kept`'i; mimari bağın `holds`'u kendi ölçümüyle; kaldırma `cad.constraints.edit` `remove`. Duvarın mesafesi'nde ret
  kartı yoldaki bağları sayar, bağlar yerinde tehlike renginde (kalın çizgi, kesikli bağ, “× d1 — kaldırılınca geçer”).
  Bütçe: 200 duvar/50 bağ önizleme 2,4 ms, ret 0,5 ms; 400/100 6,9 ms ve 2,0 ms (`arch_bond_timing`). Şema değişmez.
  22 + 22 ortak durum (`arch_bond_conflict_cases.py`). Gerçek pencerede elle denenmedi.

- **Öznitelikler'den yazılan değerin bağ kartı (31 Ekim 2026; [ADR 0797](../../docs/adr/0797-properties-bond-card.md), ADV-01 [~]).**
  Öznitelikler panelinden yazılan değer bağlarca reddedilince ADR 0790'ın kartı açılır (“Bağı kaldır”, “Bu iki bağı
  kaldır”): `kentos_interaction::properties::take_bonds_refused`; geçen yazma kartı kapatır.
  Geç çelişkide (Kaydır kilitli katmandaki duvarı sürüklerken) denenen kümeler önce yalnız denklemlerle süzülür
  (`refused_again`): 400 duvarda en kötü ret 366 → 22 ms (`arch_bond_timing`'in `late` çizimi).

- **Sürüklerken yoldaki bağlar ve birlikte kaldırılınca geçen bağlar (31 Ekim 2026; [ADR 0790](../../docs/adr/0790-bonds-while-dragging-and-together.md), ADV-01 [~]).**
  `cad.entities.edit` ve `cad.openings.edit`'in planı, bağın andığı nesneyi değiştirdiğinde komutu okuma kopyasında yürütür
  ve bağların reddini execute gibi verir (`constraint_follow::bonds_refusal`). Tutamaçla sürüklemede, yeni Duvarı kaydır
  aracında (`tool.wallShift`, `cad.walls.edit` `shift`; Mimari › Düzenle ▾), Boy'da, açıklığın mesafesinde ve Açıklığı
  taşı'da bağlar imleç hareket ederken yerinde işaretlenir (`bond_refusal::Refused`). Hiçbir bağ tek başına geçirmiyorsa
  `BondInTheWay.freesWith` en küçük kümeyi (iki, yoksa üç bağ; en çok 48 deneme) verir, küme ilk çelişkinin dışından üye
  alabilir (`bonds`'ta sonda, `uids` değişmez); ileti “şu iki bağ birlikte kaldırılınca geçer”. Reddedilen yazmadan sonra
  çizimin üstünde kart: “Bağı kaldır: d1” ya da “Bu iki bağı kaldır” tek `cad.constraints.edit` `remove` yazar.
  Sürüklemenin önizlemesi 200/50'de 1,7 ms, 400/100'de 6,1 ms. Şema değişmez. 23 ortak durum
  (`arch_bond_conflict_cases.py`), iz `wall-bond-drag.json`. Gerçek pencerede elle denenmedi.

- **Rollerin sunucuda yapabildikleri matrisle (31 Ekim 2026; [ADR 0765](../../docs/adr/0765-role-matrix-on-the-server.md), dal `formlar-surer`).**
  Beş rol × on işlem (ekle, gösterilen ve gizli alanı yaz, şekil, sil, form taslağı, yayın, katman ağacı, indir, paylaş)
  sunucunun kendi komutlarıyla (`tests/project_roles.rs`), beklentiler bağımsız başvurudan (`role_routes_cases.py`, 50 durum);
  reddedilen her işlem 403 ve meta sürümü ile veri revizyonu değişmez. Kod değişmedi.

- **Üst kaydın formu alt kayda izin vermez (31 Ekim 2026; [ADR 0764](../../docs/adr/0764-child-records-are-judged-apart.md), dal `formlar-surer`).**
  Veri girişinin yazma kuralı tek yerde: `kentos_forms::entry_write_refusal` (canlı formun gösterdiği alan okunur, yokluğu
  silinmesidir; göstermediği gizli, yokluğu onu korur, farklı değeri yazmadır; canlı formsuz katman `NoForm`), `Reach::through_live_form`.
  Sunucu `project.changes`'te gizli alana yazmayı birleştirmeden önce 403 ile reddeder (önceden `merged` onu sessizce eskisine
  döndürüyordu); masaüstü belgesi `ObjectPolicy::AttributesOnly`'de aynı kuralla adımı geri alır (`entry_write_faults`).
  İlişkili kayıtlar öğesinin `ChildPermits.child` (`ChildEntry`): Değiştir alt katmanın canlı formunu, Bağla ve Bağı kopar o formun
  bağ alanını ister. Satır kapsamı sunucuda. 20 + 20 ortak durum (`child_writes_cases.py`), resim `form-alt-kayit-izni`.

- **Formun listelerinin kaynağı: ayrı durum, kod ve çıkış yolu (31 Ekim 2026; [ADR 0763](../../docs/adr/0763-form-list-sources-states.md), dal `formlar-surer`).**
  Salt okunur `cad.forms.sources` (`form`, `draft`, `waitMs` 100–30 000, yoksa 5 000): formun başka katmandan listelerinin ve
  ilişkili kayıtlarının katmanları öğe sırasıyla, her birinin durumu `local`, `ready`, `offline`, `timeout`, `denied`, `missing`,
  `changed`, `failed`; kodları `form_source_*` (`kentos_forms::codes`), çıkış yolu `retry`, `credentials`, `relink`, `read`; liste her
  durumda çalışma kopyasından. GeoPackage dosyayla (yok, tablo yok, okunamıyor, SHA-256 farklı), veritabanı satır okumadan
  `TableProvider::reach` ile (PostGIS'te `connect_within` ve `probe::Diagnosis`'in türü, `to_regclass`, `has_table_privilege`,
  `42501`). Birden çok eşleşme ve değişen kopya zaten `form_lookup_duplicate`, `form_lookup_stale`. Masaüstünde Formla düzenle
  kaynakları `kentos-form-kaynak` iş parçacığında sorar, hazır olmayanı başlıkta kendi satırında Kaynaktan oku, Yeniden bağla…,
  Bağlantıyı düzenle… ve Yeniden dene ile söyler. 22 ortak durum (`form_sources_cases.py`), veritabanı durumları kentos-postgis'te
  ve geçici docker PostGIS'te. Elle gerçek pencerede denenmedi.

- **İlişkili kayıtlar öğesinin eylemleri ve izinleri (31 Ekim 2026; [ADR 0762](../../docs/adr/0762-related-item-actions.md), dal `formlar-surer`).**
  `FormItem::Related.actions` (`RelatedAction`: create, update, delete, link, unlink, view, show; yazılmazsa silme dışındakiler,
  boş liste salt okunur liste; şema değişmez). Formun sunduğu ile kullanıcının izni ayrı: `kentos_forms::related_offer`
  (`ChildPermits`: kayıt yazılmamış, kaydın katmanı kilitli ya da salt okunur, alt katman kilitli, rol nesne yazamaz, Veri girişi
  oluşturamaz ve silemez); sunulmayan görünmez, izni olmayan soluk ve gerekçesi ipucunda ve bölümün altında. `cad.forms.submit`'in
  `children`'ı formun sunduğuyla sınanır (`child_actions`, `relation_offer`; `form_related_action`, yolu `children[i]`). Sil satırda
  sorulur, `cad.entities.delete`; yalnız okuyan öğeden açılan alt kayıt salt okunur (`Entry.read_only`). Tasarımcıda Eylemler.
  26 ortak durum (`related_actions_cases.py`).

- **Formda ilişkili kayıtların toplamı (31 Ekim 2026; [ADR 0761](../../docs/adr/0761-related-records-aggregate.md), dal `formlar-surer`).**
  `ilişkili_toplam(ilişki, işlem, ifade, koşul)` (İngilizce `relation_aggregate`): projedeki ilişkinin alt kayıtları üstünde
  `katman_toplamı`'nın toplamaları, iç ifade alt kaydın alanlarıyla; yalnız formun kurallarında derlenir (`Schema.relations`,
  `form_schema_in`, başka yerde `RELATIONS_REFUSED`). Form alt kayıtları bir kez üst kimliğe göre dizinler (`relatives::Relatives`,
  `Object.related`, `OtherLayer.relations`), geometri yalnız okunursa (`related_needs`); bilinmeyen ilişki `form_relation_unknown`.
  Belge alt kayıt değişince üst kayıtları aynı adımda turlarla tamamlar (`follow_forms`, en çok 8 tur, yalnız gereken formlar);
  sunucunun bekçisi değişiklik kümesini dünyaya bindirir (`overlay_children`); Formla düzenle yenilenince taban kaydın yeni
  hâli. Şema değişmez. 29 ortak durum (`relatives`); 10 000 malikte bir malik eklemek 13 ms.

- **Alandan çıkınca denetim (30 Ekim 2026; [ADR 0760](../../docs/adr/0760-check-on-leaving-the-field.md)).**
  Denetimin `on`'u `leave` da olur (“Alandan çıkınca”): değer değişirken yalnız çıkılan öğede, kaydederken ve iş akışında
  hep sorulur (`Prepared::asked`, `validate_left`, `assess_left`, `kentos_forms::Left`); komutun doğrulaması azalmaz.
  KentOS UI'ın `focus_watch`'ı parçanın odak giriş ve çıkışını haber verir (basış, tuş, dokunma sonrası yalnız kendi
  ağacında; Tab bırakılışta görülür); Formla düzenle'de her alan sarılı (`Event::Focus`), seçilen değer hemen çıkılmış
  sayılır (`chosen_item`). Şema değişmez (formlar şema 72'de bütün). 24 ortak durum (`moments`), resimler `form-cikinca-*`.

- **Formda haritadan seçilen değer ve sunucuda mekânsal liste (30 Ekim 2026; [ADR 0759](../../docs/adr/0759-map-pick-and-spatial-guard.md)).**
  Formla düzenle'de başka katmandan listesi olan, tek değerli, yazılabilir alanın yanında Haritadan seç: pencere çekilir (bağlı
  form yerinde kalır), `PickObjects::on_layer` yalnız listenin katmanından tek nesne alır (seçim süzgeci ve Otomatik seçim sayılmaz),
  Esc vazgeçer, gizli ya da dondurulmuş katman seçimi başlatmaz (`forms/map_pick.rs`). Değer çekirdekte listenin kuralıyla bulunur
  (`Prepared::picked`, `List::picked`): liste o kodu bu kayda sunuyorsa yazılır, sunmuyorsa neden (`PickMiss`: boş anahtar, süzgeç,
  zincirin boş alanı, zincir, listede olmayan, geometrisiz kayıt, konuma uymayan ve en yakının etiketi, ilk nesnesi uymayan yinelenen
  kod). Sunucunun bekçisi mekânsal listenin katmanının şekillerini yazılan kayıtların kutusuyla, toleransla genişletilmiş `&&`
  sorgusuyla okur (toleranssız en yakın bütün katmanı; `form_guard::spatial_reach`, `changes::spatial_shapes`); masaüstünün reddettiği
  parsel buluta da yazılamaz. 27 ortak durum (`forms_cases.py`'nin `picked`'i), PostGIS'te `project_form_spatial`. FORM-SRC-08 [x].

- **Katmanın dizinleri ve anahtarları (30 Ekim 2026; [ADR 0758](../../docs/adr/0758-layer-keys-and-indexes.md)).**
  Katman düğümünün `indexes`'i (`.kcad` şema 136 opt-in; ad, 1–32 alanın kararlı anahtarı, `unique`, `nullsEqual`;
  `layer_indexes_problem`), `cad.layers.indexes` (tek adım “Dizinler”, `invalid_indexes`, kayıtların zaten bozduğu yeni ya da
  sıkılaşan tekil dizin `index_not_unique` ve `uids`, bağlı katmanda `index_not_in_source`). Tekil dizin katmanın anahtarı:
  her yazmada `checks::unique_clash` (boş değer değer değil, `nullsEqual` değilse; ret yazmanın değiştirdiği ilk alanda),
  `cad.fields.write` yalnız çakışan kümeyi tutanları çıkarır, formda `Prepared::with_indexes` (`index/unique`), Kaydı çoğalt
  katmanın tekil alanlarını ve anahtarlarını boş bırakır. Kaldırılan
  alanı anan dizin bütünüyle gider. GeoPackage'a `CREATE [UNIQUE] INDEX "{tablo}_{ad}"`, okurken `kentos_geopackage::indexes`
  (kısmi, ifade, bayt, fid dizinleri alınmaz). Dizinler penceresi `data.layerIndexes`. 130 ortak durum
  (`layer_indexes_cases.py`), dosyalar SQLite ve GDAL ile.

- **Tablo bağlamak arayüzü kilitlemez (30 Ekim 2026; [ADR 0757](../../docs/adr/0757-binding-without-freezing.md)).**
  Sahibin sözü: “50.000 satırı olan bir tablom var … 3 4 belki de 10 saniye kadar sistemi kilitliyor … qgis içinde bu
  problemlerin hiçbiri yok”. `cad.sources.edit`'in bağlaması masaüstünde iş olarak okuma kopyasında (`sources::edit_job`:
  zarfsız, protokolün özeti yok; adımları `BIND_STEPS`, `Steps::at` durmayı sorar, durdurulan `cancelled`), yaması tek geri
  alma adımı. Pencere hemen kapanır; köşede tablo, adım, yaklaşık satır, yumuşak çubuk ve Durdur (`exchange/binding.rs`,
  panel 0,15 s'den sonra); sürerken çizim düzenleme almaz, Esc durdurur. Veritabanı tabloları hep iş parçacığında (kataloğun
  tahmini eski olabilir), GeoPackage'ın 2 000 satırdan az tabloları hemen. Taban (`base::kept_of`) ve satırların nesneye
  çevrilmesi (`table_of_rows`) iş parçacıklarında, nesneler taşınır (`take_in_project`). 50 000 satırda 3,05 s arayüzde → 1,41 s
  arka planda, arayüzün payı 22 ms (`a_whole_table_binds_in_time`, `perf::bind`). Resimlerde bulunan: işin yaması çizime
  nesne ekleyince sıradaki yuva ilerlemiyordu (`Document::apply_patch`; görünüm yeni nesnelere gitmiyor, sonra çizilen nesne
  işinkinin yuvasını alırdı); Katmanlar'ın Öğe sütunu rozetleri ve en uzun sayıyı sayar (“50000” kesiliyordu).

- **Topoloji denetimi büyük çizimde donmaz (30 Ekim 2026; [ADR 0756](../../docs/adr/0756-topology-check-without-freezing.md)).**
  Sahibin sözü: “kilitlenme yerine yumuşak bir ilerleme çubuğu … iptal edilebilir şekilde”. Kuralların adayları paketli
  R-ağacından (`topology_rules::Boxes`: kutu sorgusu, sonra eski kesin test; bulgular bit bit aynı), Sarkan uç ve …
  ucunda olmalı en yakın aramayla, … içinde kalmalı örtünün birleşimini kurmadan: 10 000 parselde Sarkan uç 185 s → 0,15 s,
  içinde kalmalı 4,0 s → 0,09 s; 100 000 parselde her kural 2 s'nin altında (`topology_rules::timing`, `KENTOS_PERF_GRID`).
  `Watch` (ilerleme, ölçülemeyen adımın `stretch`'i, durma): `check_watched`, `recheck_watched`, `run_watched`, komutun işi
  (`JobWatch`). Masaüstünde kuralların katmanlarında 2 000 nesneden çoğu okuma kopyasında `kentos-topoloji` iş
  parçacığında (`topology/running.rs`): dönen gösterge, yüzde, her karede yumuşayan çubuk, Durdur (hemen; son bulgular
  kalır); Düzelt, İstisna yap ve Değişenleri denetle aynı yoldan, cevap yalnız başladığı çizime.

- **Sekmeler daralmaz: oklar ve liste (30 Ekim 2026; [ADR 0806](../../docs/adr/0806-tabs-keep-their-width.md), dal `launcher-zarif`).**
  Sahibin sözü: “sekmelerin boyutları daralmasın sadece sağa sola kaydırma okları çıksın veya bir tane açılır menü kalsın … aynı zamanda oklar da olsun”. `Tabs`, yuvanın yığını ve alt panel tek kuralla (`tabs::Scroll`): sekme başlığı kadar geniş, kapatmanın yeri ayrılı, ikonuna inmez; sığmayan şerit kayar: sağda `‹ ›` (her basış bir sekme), tekerlek, kesilen kenar solar; `⌄` bütün sekmeler (yuvada `⋯`), seçilen görünür olur. Yuvada panelin eylemleri sekmelerin yanına sığmazsa kendi satırına iner (`Frame::toolbar`), kısa bilgi `Pane::meta` düşer. Şerit `metrics::row`. Elle gerçek pencerede denenmedi.

- **Veri tablosu: başlık bandı, satır çizgisi, hücre, sürüklenen sütun (30 Ekim 2026; [ADR 0805](../../docs/adr/0805-data-tables-head-rows-cells.md), dal `launcher-zarif`).**
  Sahibin sözü: “başlıklar içerikle bütünleşmiş ve satır gibi gözüküyor”. Başlık satırla aynı boyda kendi bandında (`Tokens::table_head`), yarı kalın, altında `border_strong`; bütün hücresi basılır, ok sıralıyken vurgu renginde, imleçte soluk, sağa hizalıda yazının solunda. Satır `RowBox`: en az `metrics::row`, altında `Tokens::rule`, tıklanmasa da aydınlanır. Hücreler bitişik (`cell_pads`), `Cell` çizgiden çizgiye doldurur (`Cell::tint`), yazı kaymaz; `table::text`, `table::number` “…” ile biter. `Table::grid()` sütun çizgileri; `Column::resizable`, `on_fit`. Yatay kayan tabloda dikey çubuk görünen kenarda. Elle kurulan başlıklar `table::band`, `table::title`; Metraj, Blok çizelgesi, Sözlük, Hesap sonuçları `Table`; Hesap tablosu `Cell`. Kilit: `widget/table/tests.rs`. Elle gerçek pencerede denenmedi.

- **Bileşenlerin tek dili: tek yükseklik, hücre ve çerçeve (30 Ekim 2026; [ADR 0804](../../docs/adr/0804-one-control-height-cells-and-frames.md), dal `launcher-zarif`).**
  Sahibin sözleri: “girdi bileşenlerden tamamını yüksekliği ayrı olmamalı aynı olmalı, combo ile input aynı olmalı”, “arkaplanındaki ızgara hücresini şık bir şekilde doldurması gerekli”, “kapatma düğmesi isteğe bağlı”, “boyutlandırma indikatörü çok sıradan”, “tüm bileşenleri kısacası gözden geçir”.
  Bütün girdiler ve eylem düğmeleri `metrics::control` (28), satırlar `metrics::row` (aynısı); ikinci girdi boyu yok (`.inline()`, `borderless`, `SearchBox::height`, `metrics::BUTTON` kalktı). Uygulamalar `text_input` kurmaz: `widget::text_field`, `bare_field`; düğmeler `metrics::action`, `action_small`, `icon`, `icon_small` (218 ham giriş, ~430 düğme geçti). Izgara hücresi `widget::Cell`: düzenleyici çizgiden çizgiye doldurur, yazılabilir gömük zeminde, salt okunur düz, durumları tek yerde, yazı kaymaz; `InputField`, `Select`, `NumberInput`, `DatePicker`, `TimePicker`, `ColorPicker`, `Segmented`'in `.cell()`'i. `PropertyGrid`, `PropertySheet`, `Inspector` tek satır (`sheet_row::SheetRow`, tam piksel) ve tek başlık. `Dialog::on_close`, `modal`, `no_close`; köşe tutamacı çerçevenin kendisi (`sizable::frame_mark`, yüzen pencereler de). Üzerine gelme `border_strong`, odak `accent_line`, hale yok (`focus_ring` kalktı). Seçili satır yumuşak zemin. Küçük yazı `Tokens::soft` (AA). Sözcükler mono değil. Kilit: `crates/ui/tests/control_heights.rs`. Elle gerçek pencerede denenmedi.

- **Launcher'ın görünüşü: masa, pafta ve levhalar (30 Ekim 2026; [ADR 0803](../../docs/adr/0803-launcher-sheet-and-plates.md), dal `launcher-zarif`).**
  Sahibin isteği: “daha estetik ve zarif”, “tüm uygulamalarımızın paneli”, “daha şık ve kaliteli”. Kenar çubuğu masada, sayfa üstünde yatan
  paftada; kart kutuları kalktı: resim (özgün 16:9, 800 × 450, `pictures.py`) ve altında yazı, zemin yalnız imleç üstündeyken, pencerede en çok bir
  dolu Başlat. Sayfa: çıkmış uygulamalar, Son dosyalar ile Güncelleme merkezi, Yakında (küçük ve soluk). Özet şeridi başlığın altında cümle, Uygulama
  ekle başlıkta, Tümünü güncelle Güncellemeler sayfasında. Üç uygulama her pencerede tek satırda (`cards::columns`), artan pikseller aralıklara
  (`Plan::gap_before`). Masada imar planı çizimi (`scripts/launcher/art.py`, `assets/art/imar.svg`). Kurallar DESIGN.md §7.18. Fare ve yazı olayları
  gerçek pencere ağacında sınanır (`view/interaction_tests.rs`); gerçek pencerede el ile denenmedi. Yeni bağımlılık yok.

- **Bağlı tablonun CHECK ve tekil kısıtları (30 Ekim 2026; [ADR0755](../../docs/adr/0755-bound-table-check-and-unique-constraints.md)).**
  Bağın `checks`'i (ad, SQL metni, sütunlar, `tableOnly`) ve `uniques`'i (sütunlar, `nullsEqual`), alanın `column.check`'i
  (KCAD 134 opt-in); bağlamada ve her okumada tablodan (GeoPackage `CREATE TABLE` metni ve `PRAGMA index_list`,
  `kentos_geopackage::constraints`; PostgreSQL `pg_constraint` ve kısıtsız tekil `pg_index`, `kentos_postgis::constraints`).
  CHECK kendi SQL'iyle ve SQL'in üç değerli anlamıyla sorulur (`kentos_forms::table_checks`: iki lehçe, SQLite'ın
  eğilimleri, kesin ondalıkla kayan sayının uyuşması); KentOS yalnız tablonun reddedeceğini reddeder, bilinmez geçer;
  `Judged` bütünüyle, kısmen, yalnız tablo. Bozulan CHECK `rule_broken` (Türkçe cümlesiyle; formda `form_layer_rule`),
  tekil kısıt `not_unique` (`checks::table_unique_fit`, `Prepared::with_table`). Tek sütunlu IN listesi alanın değer
  listesi, `numeric(p, s)` ondalık alanın ölçeği. Tablonun reddi `source_check`, `source_unique`, `source_reference`,
  `source_not_null` (SQLSTATE ve SQLite iletisinden), okunamayan kısıt `constraints_unread`. Alanlar'da “Tablonun
  kısıtları”. 59 durum ve 183 kayıt SQLite ile PostgreSQL'in kendi cevabıyla (`table_checks_cases.py`), 25 komut durumu
  (`source_constraints_cases.py`), PostGIS'te `muhasebe.fis`.

- **Modüllerin sıkı ayrımı (30 Ekim 2026; [ADR 0802](../../docs/adr/0802-strict-module-separation.md)).**
  Sahibin sözleri: “her biri ayrı uygulama olarak çalışacak”, “bizde iki tür cad var mimari ve harita”, “arama kısmında
  ilgili modülde olmayan özellik aramada çıkmasın”, “blok gis içinde olmasa da olur, adres mimari içinde olmasa da olur”.
  Tek program dosyası: `kentos-cad` açıldığı adı okur (`kentos-mimari`, `kentos-harita`, `kentos-gis` ona bağlantı,
  `scripts/desktop/module_links.py`; ya da `--modul`); release'te semboller silinir (4 × 255 MB yerine ~216 MB). Her modül
  kendi pencere kimliği ve `.desktop` kaydıyla; launcher modülü kendi adıyla, yoksa `kentos-cad --modul` ile başlatır. İki CAD işi: `CadWork { Architecture, Map }` (Detay
  kalktı; yerel CAD Mimari'nindir). Yeni proje: Mimari sihirbazsız, Harita ve GIS türsüz iki adım. Modülün komutları
  (`Catalog::module_has`, `App::offers`): Komut ara, komut satırı, kısayollar, `App::run`, hızlı erişim, F1, menüler;
  başka modülün komutu `not_offered`'ın cümlesiyle reddedilir; kabuk komutları da modüle göre (`ribbon.json`'un
  `modules`'ü). Modülün yüzeyleri (`module_surfaces.rs`): alt panelin sekmeleri, sağ panelin bölmeleri, durum çubuğu,
  ayar pencerelerinin bölümleri, katman menüsü, uygulama menüsünün biçimleri, Kaynaklar, bırakılan dosyalar.
  Araçların seçenekleri `Draft.offers`'tan (`ToolOption`: Mimari'de Semt, nokta adları, kayıtlı ölçü yok; GIS'te blok ve
  genişlik yok). Her programın kendi yerleşim dosyası (`yerlesim-<modül>.json`) ve ajan soketi (`masaustu-<modül>.sock`,
  MCP `desktop.attach`'ın `program`'ı); bulut kataloğu, kurtarma kopyaları ve pafta galerisi programa göre. CBS analizi
  GIS'in, çizim standardı Mimari'nin; Mimari'de Ölçme sekmesi yok (sorgu Görünüm'de). `kentos-cad` her şeyi sunar.

- **KentOS Launcher ve ayrı modül programları (30 Ekim 2026; [ADR 0800](../../docs/adr/0800-kentos-launcher.md), [ADR 0801](../../docs/adr/0801-kentos-modules-and-their-programs.md)).**
  Sahibin isteği: görseldeki gibi bir launcher, Mimari CAD, Harita CAD ve GIS ayrı programlar, ileride Rust olmayan
  yazılımlar da. `kentos-launcher` (`apps/launcher`) uygulamaları `kentos.launcher-app/1` manifestlerinden okur (gömülüler,
  `uygulamalar/` klasörleri; herhangi bir program ya da web adresi), ayrı süreçte başlatır, Son dosyalar'ı gösterir;
  güncelleme servisi yoktur, düğmeleri nedenleriyle kapalıdır. Kartlar: Mimari, Harita, GIS; Yakında YOL, 3D, Afet (sahibin
  resimleriyle). Masaüstü artık kütüphanedir (`kentos_desktop`, birim testleri `cargo test -p kentos-desktop --lib`);
  program `kentos-cad` (hepsi, `--modul mimari|harita|cbs`; modül adlarıyla da, ADR 0802 §1). CAD
  projesinin işi `cadWork` (`.kcad` şema 135, opt-in): şerit işe göre (Mimari: Mimari sekmesi, Ölçme'de yalnız Sorgu;
  Harita: Plan, Ölçme, altlıklar, servisler ve kaynaklar), işi olmayan CAD çizimi bir kez sorulur, başka modülün dosyası
  o modülün programına devredilir. Ortak sözleşme `crates/native/launch` (modüller, komut satırı, `programs::find`, son
  dosyalar). Her programın yalnız kendi kodunu taşıması, paketleme ve modül başına sürüm açıktır (TODOS.md `OPS-10`,
  `OPS-16`, `OPS-17`).

- **Bağlı tablonun hesaplanan ve kimlik sütunları (30 Ekim 2026; [ADR0754](../../docs/adr/0754-bound-table-generated-and-identity-columns.md)).**
  Tablonun kendisinin yaptığı değer sütunun varsayılanının yerinde, DDL'in yan tümcesiyle (`kentos_geopackage::made`,
  `Making`): `GENERATED ALWAYS AS (…) STORED|VIRTUAL`, `GENERATED ALWAYS AS IDENTITY`, `GENERATED BY DEFAULT AS IDENTITY`;
  alanın `column.made`'ine geçer (şema değişmez; `FieldColumn::making`, `ColumnMade`, `LayerField::made_always`). GeoPackage
  `PRAGMA table_xinfo` ve CREATE metninden (`generation_of`), PostgreSQL `attidentity`, `attgenerated` (`attributes_sql`,
  sürüme göre). Hesaplanan ve ALWAYS kimlik hiç yazılmaz: UPDATE kurmaz, INSERT adlandırmaz, ikisinde de tablodan geri
  okunur ve aynı adımda nesneye geçer (`Written.refreshed`, `Edited.refreshed`, `take_filled`: NULL kaldırır). Çizimde var
  olan kaydın değeri değişmez: `attribute_generated` (`checks::made_fit`; `cad.entities.set`, `cad.fields.write`,
  `cad.forms.submit`). Formda salt okunur ve “Veritabanı hesaplar: …”, Öznitelikler ve tablonun hücresi düzenlenmez, Alanlar
  söyler. PostGIS testleri sırayla koşar. 26 ortak durum (`source_generated_cases.py`), PostGIS'te sahibinkine benzeyen
  `muhasebe.hesap_plani`.

- **Bağlı tablonun yabancı anahtarları ve veritabanının verdiği değerler (30 Ekim 2026; [ADR0753](../../docs/adr/0753-bound-table-foreign-keys-and-made-defaults.md)).**
  Alanın `column`'u (KCAD 133 opt-in): `made` (tablonun yeni satıra verdiği değerin SQL'i, `now()`, `uuidv7()`) ve `refers`
  (yabancı anahtarın tablosu, sütunu, etiket sütunu). Bağlama ve okuma (`sources::references`) tek sütunlu anahtarı
  (`PRAGMA foreign_key_list`, `pg_constraint`) alanın değer listesi yapar: başvurulan satırlar, en çok 10 000, etiket adı
  tanınan (`ad`, `adi`, `name` …) ya da ilk metin sütunu (`provider::label_column`), aynı etiket “(kod)” ile ayrılır, doğal
  sırada; her okuma yeniler, anahtar kalkınca liste kalkar, alanın kendi listesi kalır; eksik liste `reference_values_missing`.
  Formun yeni kaydında “Veritabanı verir: …”, Alanlar'da sütunun sözü, gövdesi alçak pencerede kayar. `field_problem`'in tekrar
  denetimi doğrusal. 24 ortak durum (`source_references_cases.py`), PostGIS'te sahibinkine benzeyen tablolar.

- **Yeni kaydın varsayılanları her yolda aynı sırayla (30 Ekim 2026; [ADR0752](../../docs/adr/0752-new-record-defaults-everywhere.md)).**
  Verilen değer, formun düz varsayılanı, alt türün ve alanın varsayılanı (formda olmayan alanlar da; formun seçtiği alt
  türle), formun ifadesi (önce dolanları okur), ifade boş ya da bekliyorsa alanınki. Kural tek yerde
  (`kentos_commands::new_record_defaults`); formun oturumu (`Prepared::complete`) ve çizim komutları
  (`checks::new_defaults`, kuralları uygulanan formun düz varsayılanı `Document::form_defaults` ile önce) onu kullanır.
  Önizleme yeni kayıtta katmanın varsayılanlarını gösterir; `cad.forms.validate`'in `defaults`'u komutun yazacağını verir.
  47 ortak durum (`form_defaults_cases.py`), alt tür ve bileşim durumları ikizlerle yeniden yazıldı.

- **Formda bileşenin ikonu, iki sütun, saat dilimli tarihin seçicisi (30 Ekim 2026; [ADR0751](../../docs/adr/0751-forms-widget-icons-columns-and-instants.md)).**
  Tasarımcının ağacında ve Öğe türü listesinde her alan kaydın göstereceği bileşenin ikonuyla (`widget_glyph`; tür
  tarihi saatten, büyük sayıyı metinden, JSON'u ve baytları ayırır; yeni ikonlar `designer_view::glyphs`). Formla
  düzenle'de ardışık alanlar sayfa yazı boyuyla büyüyen 880 pikseli geçince iki sütuna akar (`TWO_COLUMNS`, KentOS UI
  `Responsive`; sıra ve Tab aynı, önizleme ve alt panel tek sütun). Saat dilimli tarih (`instant`) tarih ve saat
  seçicisiyle, değerin dilimi korunur, boşsa cihazınki (`instant_parts`, `instant_text`); seçicilerin “Şimdi”si
  değerin diliminde ya da cihazın saatinde (`now_at`, `device_now`; önceden UTC). Resimler `district_form_screens`.

- **Bağlı tablonun zorunlu ve varsayılanlı sütunları (30 Ekim 2026; [ADR0750](../../docs/adr/0750-bound-table-not-null-and-defaults.md)).**
  Sütun alan olurken (`geopackage::field_of`) NOT NULL ve varsayılansız sütun zorunlu, sabit varsayılan
  (`literal_default`: tırnaklı metin, sayı, true/false, SQLite'ın 0/1'i) alanın varsayılanı; hesaplanan (`now()`,
  `uuidv7()`) alana konmaz. Tablodan okuma kuralları var olan alanlara katar, kaldırmaz. Yeni satırın boş değeri
  varsayılanlı sütunda tablonun (`gpkg::left_to_default`; PostgreSQL'de `DEFAULT`, GeoPackage'ta sütun listesinden çıkar),
  döndürdüğü değer aynı adımda nesnede (`Edited.filled`, `Inserted.filled`, `sources::take_filled`). NOT NULL'u boş
  bırakan yazma `source_not_null` (planda da). `gpkg_extensions` yoksa açılır (`ensure_extensions`). 21 ortak durum
  (`source_defaults_cases.py`), PostGIS'te sahibinkine benzeyen tablo (`postgis_cases.py --only a_new_row_takes`).

- **Formların pencereleri her boyda ve yazı boyunda sığar (30 Ekim 2026; [ADR0749](../../docs/adr/0749-forms-windows-fit.md)).**
  KentOS UI: `Segmented` yerinden taşmaz (KentOS UI `Responsive`'iyle en çok genişliği bilir; sığmayınca ikonları,
  yoksa `shrunk_widths` ile kısalan yazılar, `Tip::when_cut`); `segmented::natural_width`; `Panes` (yan bölmeler pay,
  en az ve en çok, ölçülen en az `.least`, önce sağ geri verir); `fallback(geniş, dar)`. Form tasarımcısı `Panes` ile,
  sağ bölme dört sekmede aynı; araç çubuğu sığmayınca ikonlarına iner, Geri al ve Yinele ikinci satırda. Tasarımcı,
  Formla düzenle, Alt türler, Değer listeleri ve Değer bileşimleri'nin gövdesi esnek (`Dialog::fill`, en çok eski
  yüksekliği). Sahibin görünüşüyle (14 px, Jakarta, açık) resimler `form_fit_screens`, 92 sahnenin düğme taraması.

- **Öznitelikler'de kısalan ad ve değer, “Çeşitli” (29 Ekim 2026; [ADR0748](../../docs/adr/0748-inspector-cut-texts-and-mixed-values.md)).**
  Satırın adı `Elided`; `EditCell` düzenlenmezken değerini alanına sığdırır (`elided::fit`, “…”), basış önce
  gösterilen metne verilir (iced imleci önceki metinden bulur; Türkçe harfin ortası), “…”ya basmak imleci sona koyar.
  `Tip::when_cut` ipucunu yalnız kısalan yazıda açar (`elided::Cut`, `Operation::custom`); ad, değer, liste ve hücre.
  `property_grid::varies`: “Çeşitli” soluk, hücresi boş düzenlenir, boş bırakılan bir şey göndermez (AutoCAD'in
  *VARIES*'ı). İpucu arayüzün saatini okur (`clock::now`). Sahibin çiziminde 1:81'de de harf başına 13 ms.

- **Öznitelikler'in satırları ve hücreleri (29 Ekim 2026; [ADR0747](../../docs/adr/0747-inspector-rows-and-cells.md)).**
  KentOS UI'ın `SheetRow`'u satırı birim yapar: üzerine gelince bütün satır aydınlanır, değeri düzenlenirken solunda
  2 px vurgu çubuğu (satırın içindeki odaklı metin girişinden). Hücre ve liste değer sütununu doldurur (`VALUE_PAD` 3,
  `CELL` 24), üç tür değerin yazısı aynı hizada (`VALUE_TEXT` 11), birim kenardan `UNIT_PAD`; `EditCell` her
  yerleşimde yeniden kurulduğu için iced'in “devre dışı” saydığı değeri artık metin renginde, boş değer soluk “—”.
  Sahibin 71 820 nesnelik çiziminde harf başına süre `inspector_probe.rs` ile ölçülür.

- **Boyutlanan ve boyunu hatırlayan pencereler (29 Ekim 2026; [ADR0746](../../docs/adr/0746-resizable-remembered-dialogs.md)).**
  KentOS UI'ın her `Dialog`'u `Sizable` ile kenarından ve köşesinden boyutlandırılır, başlıktaki düğme ya da çift tıkla
  tam ekran olur (arkadaki pencere eksi 16 px); boy anahtarıyla (öntanımlı başlık, `Dialog::key`) `sizable::sizes`'ta,
  masaüstünde `pencereler.json`'da (`dialog_sizes.rs`). Boyu verilmiş kutuda `Fit` kalan yeri son esnek gövdeye verir;
  Formla düzenle (760 × 460) ve Formlar'ın bölmeleri kalan yeri alır. `Dialog::fixed()` boyutlanmaz.

- **PostGIS'te geography ve geometrisiz tablo (29 Ekim 2026; [ADR0745](../../docs/adr/0745-postgis-geography-and-empty-geometry.md)).**
  `geography` sütunu katalogda ve bağlamada okunur (`sütun::geometry`, sistemsiz 4326), salt okunur; bütün satırları
  geometrisiz tablo `rows_without_geometry` uyarısıyla bağlanır, masaüstünün bildirimi uyarıyı gövdesinde yazar.

- **Yaylı kaynak ve sınırla ölçülü bölme (29 Ekim 2026; [ADR0734](../../docs/adr/0734-measured-partitions-on-arc-edges.md)).**
  `cad.walls.partition` yaylı kenarı artık reddetmez: bölme kaynağın merkezinde aynı merkezli yaydır (Revit'in
  CreateOffset'i), `r_p = r − sgn(θ)·öteleme` (saat yönü tersine yayın solu merkeze doğru), net/eksen uzaklık radyal.
  `length` bölmenin kendi ekseni boyunca; `between` düz ya da yaylı sınırların eksenleriyle, kaynağın ortasına en yakın
  buluşma. Merkezi aşan bölme, ulaşılmayan ve aynı merkezli sınır atomik ret. Plan `sourceFaceBulge`; araç yayı
  çizildiği yerden seçer, yanı içte/dışta izler. 22 bağımsız yay durumu (50 basamak) + 90 düz, gerçek GUI.

- **Çizelgenin kalanı: donatı, kaplama, gruplar, ayarlar, dosya (29 Ekim 2026; [ADR0733](../../docs/adr/0733-schedules-fixtures-finishes-groups-export.md)).**
  Blok yerleştirmesinin `arch`'ı (kat, tadilat durumu; KCAD 131 opt-in, tipli sütunlarda 5. alan): Blok ekle etkin katı yazar,
  mutfak modülü duvarının katını alır, `cad.levels.assign` yerleştirmeyi kabul eder, kat ve tadilat görünümü izler.
  Çizelgenin `of: fixtures`'ı (Tür, Kategori: kartın, Mahal: bloğun çiziminin ortası, Kat, Adet; dış başvuru blokları
  sayılmaz), mahalin kaplama alanları ve `finishes.base`, Süpürgelik boyu (`kentos_domain::passages`, Metraj'la aynı),
  `filter.types`, `sort[].header`/`footer` (grup başlığı satır boyunca, ara toplam “‹değer› toplamı”); `members` başlıktan
  sonraki her satır için (başlık/ara toplam boş), ara toplam satırı düzenlenmez (`schedule_summary_row`). Salt okunur
  `cad.schedules.query` (sütunların yazılırlığı, satır rolleri), `file.export.table` (CSV/XLSX/ODS, gruplar, Tam sayılar:
  15 anlamlı basamak). Çizelge ayarları penceresi, Dosyaya yaz, Tabloyu düzenle'de hücre açıklaması, Kaplama ve Donatı
  çizelgesi araçları, Metraj'da Tam sayılar. 66 bağımsız ortak durum (`schedule_extras_cases.py`).

- **Mimari denetimin kalanı: kapsam, kalıcı istisna, yeni denetimler, toplu düzeltme (29 Ekim 2026; [ADR0732](../../docs/adr/0732-architectural-audit-scope-exceptions-fixes.md)).**
  Bulgunun kalıcı `key`'i (`kod:kimlikler`, uç adı, `#n`); `cad.arch.audit`'in `within`'i (katlar, nesneler; açıklığın
  katı duvarından), `minLength`, `exceptions` (hide/show/only), cevabın `outside`, `excepted`, `stale`'i. İstisna proje
  ayarı `arch.auditExceptions` (KCAD 130 opt-in; anahtar, kod, nesneler, bulgunun kanonik JSON özeti, not), yazan
  `cad.arch.exceptions` (mark/unmark/prune, tek geri alma adımı); bulgu değişince istisna düşer (`lapsed`,
  `exception_lapsed`). Yeni denetimler: `arch_wall_short` ve `arch_wall_overlap` (aynısı silinir), `arch_separator_overlap`,
  `tag_empty`, merdivenin ve rampanın kendi uyarıları (Rampalar grubu), `arch_casework_broken` (Mutfak dizileri).
  Masaüstü: Ctrl/⌘ ve Shift ile satırlar, Düzelt (n) tek `cad.changeset.apply` (“Denetim düzeltmeleri”), düzeltme
  okuma kopyasında çizimde önizlenir (silinen tehlike renginde), satırın sağ tık menüsü, not sorusu, Kapsam ▾,
  İstisnalar ▾, Eşikler ▾. 80 + 25 ortak durum (`arch_audit_cases.py`), masaüstü testleri, gerçek GUI. ARCH-DOC-03 [x].

- **Görünüş başına duvar ayrıntısı ve taze PDF matrisi (29 Ekim 2026; [ADR0696 eki](../../docs/adr/0696-model-wall-detail-and-dense-hairlines.md)).**
  Bölünmüş görünümün her görünümü duvarları kendi kamerasının ayrıntısıyla çizer (Revit'in görünüş Detail Level'ı,
  Archicad'in pencere başına yapı gösterimi): depo etkin görünümün açılımını tutar (seçme, kenet), öbürleri deponun
  birleşmiş duvarlarından kendi `wall::View`'larında (`Store::expanded_at`); stil çekirdeğinin `View.walls`'ı geometriyi
  de parça stillerini de o görünüşten okutur. %10 dönüş aralığı görünüş başına; etkinleşen görünüm ayrıntısını depoya
  taşır (`set_screen_wall_scale_from`), Kaydırmalı karşılaştır etkinin ayrıntısı, açık proje ayrıntısı hepsinin; kalıcı
  alan yok. Pafta/PDF `view_at`: Dış duvar 25'in 1:20/1:50/1:100/1:200 haritaları masaüstünde ve başsız kitapta
  ayrıntılı/uygulama/şematik/şematik, model 1:15'te de (`wall_detail_sheet_tests`, `pdftoppm`). ARCH-DIM-01 [x].

- **Ölçü dizisinin kalanı: pafta ölçekleri, sığmayan yazı, yuvarlama, kopya (29 Ekim 2026; [ADR0730](../../docs/adr/0730-dimension-set-sheets-text-rounding-copy.md)).**
  Tarifin `scales`'ı (pafta ölçekleri) üyelerin `annotative`'ini tarifin hesabıyla yazar: her ölçekte sıra başvuru
  çizgisinden `s/P` katı uzakta (`ops::dimension_layout::scaled_offset`), kâğıtta her paftada aynı; temsiller tarifindir.
  `tight: shift` sığmayan yazıyı sağ, sol, üst, alt adaylarından ilk boşa koyar (`place_values`; elle taşınan yazı kalır,
  engel sayılır, üyenin `shift`'i tarifin son yeri; `resetText` döndürür). Yuvarlanmış parçalar toplamı tutmazsa plan
  `rounding` ve `dimension_set_rounding` (dağıtılmaz). Bütünüyle seçilmiş tarif Kopyala/Dizi/pano ile kopyalara, Taşı/
  Döndür/Ölçekle/Aynala ile birlikte gider (aynalamada yol ters: yüz aynı, kenar yeni yolda; açıklık pervazdan okunur).
  KCAD 129 opt-in; sunucu temsilleri denetler ve ADR0695 ekinin tek sayılan aralık kuralını artık bilir. Araçta Pafta
  ölçekleri ve Sığmayan yazı (yeni tarif Kaydır), Öznitelikler'de aynı satırlar ve Yazıları tarife döndür. PDF'te 1:50 ve
  1:100 paftada aynı kâğıt boyu (`dimension_set_sheet_tests.rs`).

- **Cephe, duvar grubu ve mahal ölçü dizileri (28 Ekim 2026; [ADR0729](../../docs/adr/0729-facade-and-space-dimension-recipes.md)).**
  Ölçü dizisi tarifinin `span`'ı (KCAD 128 opt-in): kaynak duvarlar (`wall` + `walls`) ya da mahal (`space`), doğrultu
  (okunur yöne çevrilir), `reference` dış yüzler/çekirdek (en güçlü bileşen bandı), kat. Ortak çekirdek
  `ops::dimension_span` ve ortak toplayıcı `kentos_formats::dimension_spans`: dik kenar noktaları, uçtaki köşe duvarı,
  saplanan/kesişen duvar (eğik/yaylı ise `dimension_set_wall_skipped`), açıklığın yakın pervazları, s yanından
  görünürlük, dik akslar; mahalde dış halka, delikler ve s'ye bakan kenarın açıklıkları. Sıralar `openings`, `walls`,
  `grids`, `overall`; boş sıra yer kaplamaz; `around` dört yan. Planın `rows`/`skipped`/`tight`'ı ve sığmayan yazı
  `dimension_text_tight` (gizlenmez). İzleme aynı adımda: bölme parçası kaynaklara katılır, birleşen yerini alır,
  tek duvar tarifi bölünürse cepheye döner, kimlik aynı çift ya da nanometre aralığı; karışık kat `mixedSources`.
  `cad.dimensionsets.edit`'in `Sources` ve `reference`'ı, `span`'lı tarifte sıra uzaklığı en dış yüzden. Araçta Kaynak
  türü (Duvar/Cephe/Mahal), Doğrultu, Ölçülen çizgi, Yanlar, Duvarlar/Akslar sırası; Öznitelikler'de kaynak ve sıralar.
  Veritabanı projesinde sunucu `project.changes` v2'de tarifi aynı kuralla kilit altında planlar (`check_span_rows`:
  kaynaklar, akslar, kutudaki duvar ve açıklıklar PostGIS'ten), üyelik eksik ya da ölçü kaymışsa atomik ret; masaüstünün
  eşitlemesi bütün kaynakları bağlı sayar. 70 bağımsız ortak durum (`dimension_spans_cases.py`).

- **Geçici ölçünün kalanı: aksa mesafe, duvar boyu, iki aday (28 Ekim 2026; [ADR0728](../../docs/adr/0728-temporary-distances-grids-length-references.md)).**
  Duvar, kolon ve açıklığın geçici ölçüsünde aks ikinci adaydır ve nesnenin ekseninden ölçülür (duvar kenarının iki
  yüzünün ortası, kolonun ortası, pervazların ortası; düz aks, katında ya da katsız, sonsuz doğru; `ops::grid_ref`).
  İki adaylı yanda referansın ucunda kare, basınca öbürüne geçer; seçim oturumundur (`temporary_refs::Choices`),
  çizime yazılmaz. Kayabilen ucu olan kenarın boyu (yolun ucu; T ve çok duvarlı uç kaymaz, `free_ends`): sabit uç
  dolu, `cad.walls.edit`'in `length`'i (`anchor`), köşedeki duvar Kaydır'la izler, uç uca duvar kısalır.
  Düzenlemelerin `to`'su (`face`, `grid`); sorgularda `leftGrid`/`rightGrid`/`length`, `grids`, `startGrid`/`endGrid`.
  Kilitli referans ölçer, yerinde kalır; dönük UCS ve büyük koordinatta aynı; Python/MCP aynı komutlar.

- **Komut kataloğu dosyasında paylaşılan tanımlar (28 Ekim 2026; [ADR 0731](../../docs/adr/0731-command-catalog-shared-definitions.md)).**
  `commandCatalog.json` kataloğun 2. sürümüdür: şemaların `$defs`'i kataloğun üst düzey `$defs`'inde, her tanım bir kez
  (`CommandCatalog::shared`); 56,6 MB → 4,25 MB, büyüme tür sayısıyla. Bellekteki `catalog()` (MCP, başsız sunucu, Python'un
  yerli modülü) her şemayı yine kendi `$defs`'iyle verir. Dosyayı okuyan betik tanımları kökten alır (`sdk.py`, `coverage.py`,
  `history_calls.py`); şemayı tek başına isteyen `standalone`'ın kuralını uygular.

- **Mutfak dizisi: L ve U, cihazlar, kalıcı dizi (28 Ekim 2026; [ADR0727](../../docs/adr/0727-kitchen-runs-corners-appliances-follow.md)).**
  `cad.casework.run`'ın `through`'u (en çok iki dik köşe), `corner` (kör, açık), `dishwasher`, `oven`, `fridge`,
  `perModule` (modülün eni ve kapısı); tezgâh tek alan, evye/ocak kesimleri; modüllerde Modül/Tür/Açılma. Dizi belgede
  kayıttır (`casework`, KCAD 127 opt-in): tarif, kolların duvar yüzü bağları, üyeler; `follow_casework` bağlı duvar
  değişince aynı adımda yeniden dizer (`wallGone`, `notBuilt`), bütün üyeler birlikte taşınınca diziyi taşır; üye
  diziden ayrı değişmez (`casework_owned`, protokol kodu). `cad.casework.edit` (module, recipe, release),
  `cad.casework.query`, `casework_clash`; kör köşede önü kapanan modül (`blind`) ve oradaki evye/ocak/bulaşık
  makinesi `casework_blind_corner`. Araçta Biçim (B: Düz/L/U, köşe dikliğe izdüşer), Otomatik kör modülü atlar;
  Öznitelikler'de Mutfak dizisi bölümü. Bulutta proje meta (göç 0035).

- **Blok kitaplığı: DWG/DXF klasörleri, gerçek boy ve birim (28 Ekim 2026; [ADR0726](../../docs/adr/0726-block-library-real-size.md)).**
  Bloklar'da Bu çizim | Kitaplık: cihazın kitaplık klasörleri (`blok-kitapliklari.json`), alt klasörler kategori, çizimin
  blokları resim, gerçek en × boy ve birimle; birim bildirmeyen DWG/DXF'te önce birim seçilir. `file.import.block` DWG
  ve DXF okur (`cad_open::read_in`), `sourceUnit` (bildirmeyen dosya onsuz `source_unit_unknown`), `category`, cevapta
  `sizes` ve `unit`. Blok ekle hayaletin yanında gerçek en × boyu, birimi ve dönüşü söyler, taban noktasını işaretler;
  görünür ayarlarda dinamik bloğun satırları ve Varyant (V); Öznitelikler'de En × boy ve Bloğun birimi.

- **Aks ve kolon: eğrisel ağ, kesik aks, hiza, L kolon (28 Ekim 2026; [ADR0725](../../docs/adr/0725-curved-grids-column-forms.md)).**
  `cad.grids.create`'in `radial`'ı radyal aksları (açılar iki yönde, toplamı tam turdan az) ve yayları
  (`[tan(süpürme/4), 0]`) yazar, yay kalan turun dörtte birinden fazla uzamaz; kesişim kolonları radyal yönde. Kesik aks
  `ends` (kâğıtta mm), kolonun `anchor`'ı (dokuz nokta, `at` dayanak noktası, `column_center`) ve L (`lShape`, `legs`,
  altı köşe, aynada çeyrek tur az ve kollar takas), KCAD 126 opt-in. Araçta “L/n”, İki nokta'da Eşit böl, Kolon
  aracının ayarları; Denetim'in Akslar grubu (`arch_grid_label_duplicate`, sil ya da adlandır). Aksın ölçü yerleri
  (`subref` ailesi 10); ölçü ve kolon çizelgesinin Yer'i aksı aynı adımda izler.

- **Merdiven: kat yüksekliği, yönetmelik profili, üst katta gösterim (28 Ekim 2026; [ADR0724](../../docs/adr/0724-stair-levels-profiles-upper-plan.md)).**
  `cad.stairs.create`'in `heightFromLevel`'ı merdiveni katının yüksekliğiyle tasarlar; belge kat yüksekliği ya da rıht
  sayısı değişince rıht yüksekliğini yazar (`level_marks.rs`, rıht sayısı korunur), KCAD 125 opt-in; `cad.arch.edit`
  bağlar, `false`/kendi `riserHeight`'ı kaldırır. Döner merdivende merkez direği (`post`, araçta M). Profiller
  `catalog/v1/arch-profiles/` (`kentos.arch-profile`; İmar Yönetmeliği md. 31'in konut/asansörsüz/diğer/daire içi
  değerleri, mevzuat onayı değil); `cad.arch.audit`'in `profiles`'ı (bir rampa, bir merdiven) ve `stairs` grubu yedi
  kodla, üçü Düzelt'li. Üst katta gösterim: alttaki katın kesmeli merdiveni etkin katta yukarıdan
  (`stair_upper_pieces`, `LevelLooks.upper`, seçilmez); pafta haritası kendi katıyla ayrı depo, ekran yukarıdan
  çizerken paftalar çizimin deposundan (`draws_otherwise`). Metraj `stairs` satırları ve Merdivenler sekmesi (yedi
  sayfa). Gerçek PDF'te 1. kat paftası merdiveni yukarıdan çizer (`stair_levels.rs`, `pdftoppm`).

- **Rampa: ara sahanlık, kata bağlı başlangıç, metraj, erişilebilirlik (28 Ekim 2026; [ADR0723](../../docs/adr/0723-ramp-landings-levels-accessibility.md)).**
  `cad.ramps.create`'in `maxRun`'ı aşan kola düz köşe ve sahanlık ekler (`geom::arch::ramp_with_landings`);
  `baseOffset` başlangıcı katın kotu + fark yapar, belge kat değişince yazar (`level_marks.rs`, etiketlerden önce),
  KCAD 124 opt-in; `cad.arch.edit` `baseOffset` bağlar, `base` ya da `null` bağı kaldırır. Rampa aracında En uzun
  kol (L) ve Başlangıç kotu (Z: boş/kat etkin kat, kat±fark, sayı kendi). Metraj `ramps` satırları (`ramp_measure`)
  ve Rampalar sekmesi; `cad.arch.audit`'in profili (ADR 0724'ten beri `profiles`, `catalog/v1/arch-profiles/`, TS
  9111 özeti; mevzuat onayı değil) `accessibility` grubu: eğim bandı, uzun kol, kısa sahanlık ve dar dönüş, son ikisi Düzelt'li;
  masaüstünde öntanımlı açık. Planda okun yanında eğim yazısı (DXF ve PDF de). Genişlik profili açık.

- **Döşeme, balkon, teras (28 Ekim 2026; [ADR0722](../../docs/adr/0722-slabs.md)).**
  Yeni `slab` türü mahalin şekliyle çizilir (ikinci alan motoru yok): Döşeme/Balkon/Teras, kalınlık, kata göre kot
  farkı, malzeme, metraj dışı. `cad.slabs.create` çizilmiş, duvarların dış yüzlerinden (`outline`) ya da odadan
  (`room`); `cad.slabs.edit` Değiştir/Kenarı ötele/Kes/Ekle. Kenar ötelemesinin dört başarısızlığı atomik ret.
  Korkuluk/parapet yalnız serbest kenarlarda, belge duvarla izler (`follow_slabs`). Metraj: döşeme net alanı (boşluk
  mahalleri düşülür), hacim, üst kot; mahalde süpürgelik (kapı ve döşemeye inen boşluk düşülür) ve kaplama geçişleri.
  KCAD 123 (opt-in), `FORMATS_VERSION` 98, göç 0034, DXF mimari INSERT. Katmanlı yapı, kesit/3B, katlar arası şaft açık.

- **Mimari işaretler (28 Ekim 2026; [ADR0721](../../docs/adr/0721-architectural-symbols.md)).**
  Görünüm işareti hedef paftanın çizim numarasını okur (yeniden numaralama, kopya, `targetGone`). Kot `level
  {id, offset?}` ile kata bağlanır, kat değişince izler (KCAD 121). `cad.north.create` (plan, coğrafi kuzey: meridyen
  yakınsaması, açı), `cad.revisions.mark` (bulut + kitaplığın “Revizyon” üçgeni, numara/tarih/açıklama),
  `cad.legends.materials` (duvar bileşenlerinin taraması ve adı); Revizyon listesi sütunlu blok çizelgesidir
  (`TableSource::Blocks.columns`, KCAD 122). Duvarın yüzleri stil değişse de ilk ve son çizgisidir
  (`subref::wall_face`). Kabul PDF'te `pdftotext`/`pdffonts` ile; pafta başına revizyon çizelgesi açık.

- **Etkin kat dosyada (27 Ekim 2026; [ADR0720](../../docs/adr/0720-active-level-in-the-file.md)).**
  Çizim kaydedildiği etkin katta ve kat görünümüyle açılır: `arch.activeLevel` (kat kimliği, Katsız `null`) ve
  `levelView`, KCAD 120 opt-in; kaydetme ve kurtarma kopyası ekrandakini anlık görüntüye yazar
  (`active_level::with_level_state`), belge değişmez; sekme başına kat. `cad.settings.set` kaldırılan katı anan her
  mimari türü ve pafta haritasını sayıp kimlikleriyle reddeder (`settings_set::levels_in_use`); Proje ayarları uyarıyı
  ve Nesneleri seç'i gösterir. Üst kat/Alt kat (`view.levels.up/down`, Ctrl+↑/↓); `Tool::level_changed` ile araçlar
  katı hemen izler.

- **Niş, baca ve tesisat boşluğu (27 Ekim 2026; [ADR0719](../../docs/adr/0719-niches-flues-service-holes.md)).**
  Açıklığın `niche` (bir yüzden `depth` içeri) ve `flue` (duvarın içinde, `inset` yoksa ortada) biçimleri, yüz `side`;
  KCAD 119 opt-in. Sığmayan derinlik `recess_too_deep`, sonradan incelen duvarda `tooDeep`. Boşluk/niş/baca
  `PLAN_CUT_HEIGHT` (1,10 m) kesmeyi kesiyorsa duvarı keser (çözücüde `opening::Hole`/`Recess`, nişin arkası ve kalan
  dolgu, `Drawn.recesses`), kesmiyorsa kesik çizgi (`opening::pieces`); kapı/pencere her zaman kesilir. Kotları ayrı ve
  plan birini kesen iki açıklık aynı yerde durur (`clashes_with`). Metrajda niş/baca duvardan düşülmez; ölçü dizisi
  nişi kendi yüzünde ölçer. Mimari › Boşluk ▾ (Boşluk/Niş/Baca), araç ayarları ve Öznitelikler.

- **Kat planı paftaları (27 Ekim 2026; [ADR0718](../../docs/adr/0718-level-plan-sheets.md)).**
  Her kat için pafta tek adımda: pafta çekirdeği `levels::level_sheets` (tasarımcının Yeni ▾
  ve `cad.sheets.levels` aynı yol), haritası o katı çizer, bütün haritalar katların nesnelerinin
  kutusunda aynı ölçekte (ölçek/merkez verilmezse sığan standart ölçek). Numara A-101'den
  (kullanılanlar atlanır), ad “A-101 Zemin kat planı”, alt küme Mimari › Kat planları; paftası
  olan kat atlanır (`again` ile `-2`). Pafta › Yeni ▾ ve CAD'in Çıktı › Pafta › Kat planları.

- **Kat ver (27 Ekim 2026; [ADR0717](../../docs/adr/0717-assign-levels.md)).**
  `cad.levels.assign`: verilen mimari nesnelere ya da (uids yoksa) Denetim'in katsız
  saydıklarına projenin katı, yer değişmez, tek adım “Kat ver”; kilitli atlanır, mahal
  yeni katında yeniden bulunur (kapanmazsa `arch_space_broken`, okuma kopyasında önceden).
  Denetim'de kat bulgusunun Düzelt'i etkin katı (`cad.arch.audit`'in `level`'i), Kata ver ▾
  (Seç ve göster, sonra kat); Öznitelikler'de karışık mimari seçimde ortak Kat. Ortak durum
  koşucusu komut başına çerçevede (`run_typed`): bütün durumlar 2 MiB yığında geçer.

- **Paftada kat (27 Ekim 2026; [ADR0716](../../docs/adr/0716-sheet-map-level.md)).**
  Pafta haritasının kendi Kat'ı (`MapItem.level`: `id`, `below`; Revit'in kat
  planı ve altlığı, görünüm kopyası yok): katın ve katsız nesneler, alt kat kâğıtta
  yarım ton; ekranın kat görünümü paftayı değiştirmez. `setViewport.level` bütün
  yazılır, projede olmayan kat `level_not_found` (atomik), ön denetimde
  `map_level_missing`; denetçide Kat ve Alt kat soluk; yerinde model düzenleme
  haritanın katıyla. Görünüm penceresinin kapsamı artık geometrik (`touching_ring`).

- **Kat görünümü (27 Ekim 2026; [ADR0715](../../docs/adr/0715-level-view.md)).**
  Oturumun durumu: Bütün katlar / Yalnız etkin kat / Etkin kat ve alt kat (alt
  kat %60 soluk, yazıları da; kenetlenir, seçilmez: Revit altlığı). Nesnenin katı
  `Entity::level` (açıklık duvarının, etiket nesnesinin, kaplama mahalin, ölçü
  bağlarının); çekirdek depoda `set_unpicked`/`pickable`, `Spatial::set_level_view`,
  tıklamada etkin kat önce (üst üste duvarda kapı/pencere etkin kata). Kat menüsü,
  hücre ikonu, Görünüm › Kat görünümü ▾ (`view.levels.*`). Paftanın katı ADR0716'da.

- **Açıklığın numarası yerleştirirken (27 Ekim 2026; [ADR0714](../../docs/adr/0714-opening-numbers-on-placement.md)).**
  Revit'in Mark'ı gibi: Kapı/Pencere/Boşluk'un Numara (N) ayarı Otomatik (aynı
  biçimin son numarasından artar, genişliği korur; yoksa K1/P1/B1), yazılan
  numaradan sırayla, Yok; kullanılan atlanır; numara `cad.openings.create`
  girdisinde. Etiketin yazısı zeminli: duvar üstündeki etiket okunur.

- **Etkin kat (27 Ekim 2026; [ADR0713](../../docs/adr/0713-active-level.md)).**
  Archicad'in geçerli katı gibi: çizim zemin katta açılır (kotu 0, yoksa ilk);
  durum çubuğunun Kat hücresi ve yeni nesne çizen araçların ilk ayar satırı
  katı seçer (Katsız da). Duvar, dikdörtgen duvar, bölme, kolon, merdiven,
  rampa ve aks ağı etkin katla yazar (`Context::level`, `level_arch`; komut
  girdisinde açık); mahal/toplu mahal/ayırıcı/iç zincir o katta duvar varken
  izler, elle seçilen kalır. Oturumundur, dosyaya yazılmaz; kat görünümü yok.
  Belgele'de Duvar ölçüleri ile İç ölçü zinciri tek “Ölçü ▾”. Ek: merdivenin
  Yükseklik'i yazılmadıkça etkin katın yüksekliği (“3.060 m · katın”; boş ya da
  “kat” döner), `stair_rise_level` uyarısı kendiliğinden kalkar.

- **Etiketli mahal ve boş etiket (27 Ekim 2026; [ADR0344 eki](../../docs/adr/0344-tags-and-schedules.md)).**
  Daire yürüyüşünde (`walkthrough_scenes.rs`) bulundu: etiketi olan mahal kendi
  yazısını çizmez (`Document::tagged_spaces`, `Store::set_tagged_spaces`; ekran ve
  pafta), şablonun boş satırları atılır, numarasız açıklığın boş etiketi
  `tag_empty` uyarısıyla ve önizlemede söylenir. Kapı ve pencere çizelgesinin
  Duvar sütunu stilsiz duvarda kalınlığı yazar (“0.250 m”; önceden boştu).

- **Kolonun geçici ölçüsü (27 Ekim 2026; [ADR0712](../../docs/adr/0712-column-temporary-distances.md)).**
  Seçili kolonun dört yanında aynı kattaki en yakın paralel yüze (duvarın bakan
  yüzü ya da öbür kolonun yanı) net mesafe; sayıya bas-yaz-Enter kolonu o yana
  dik kaydırır, karşı yüz sabit, kolonu düşen mahal aynı adımda izler, tek undo.
  Akslara bağlı kolon `column_on_grids`: sayıya basınca neden ve çözüm söylenir.
  `cad.arch.edit` `distance` ve salt okunur `cad.columns.distances` (Python/MCP);
  çekirdek `ops::column_shift`. 22+23 bağımsız durum, 3 çekirdek+4 araç+2 SDK
  testi, gerçek GUI. Üç geçici ölçüde sayı çizgiyi örter, tıklama en yakın
  sayıya gider. Aksa mesafe, duvar boyu, kilitli referans açık.

- **Kesen çizgiyle iç ölçü zinciri (27 Ekim 2026; [ADR0711](../../docs/adr/0711-interior-dimension-chains.md)).**
  Archicad'in iç ölçülendirmesi: kesen çizginin kestiği duvar yüzleri (dış yüzler
  ya da bütün katmanlar) ve kolonlar (kenar/merkez) sırayla doğrusal ölçü zinciri,
  tek undo; başka gövdenin içindeki kesişim görünmez yüz, yay yüz atlanır ve
  söylenir. Yeni `cross` kuralı (KCAD 118 opt-in): nokta yüzün doğrusunun kesen
  çizgiyi kestiği yer, ölçü çizgisi dünyada yerinde; duvar kayınca değer değişir.
  Duvar yüzü ve kolonda `along`/`center` yerleri. `cad.dimensions.interior`
  (Python/MCP), çekirdek `ops::interior_dims`; 44 bağımsız durum (izleme dahil),
  5 çekirdek+4 araç testi, gerçek GUI. Üyelik kalıcı değil; açıklık/aks açık.
  Aynı gün: Duvar ölçüleri açıklıksız kenara aynı boyu iki kez yazmıyor (ADR0695 eki).

- **Duvarın geçici ölçüsü (27 Ekim 2026; [ADR0710](../../docs/adr/0710-wall-temporary-distances.md)).**
  Seçili duvarın imlecin yakın olduğu düz kenarında iki yandaki en yakın paralel
  duvarın bakan yüzüne net mesafe; sayıya bas-yaz-Enter kenarı kendine dik
  kaydırır, karşı duvar sabit, köşe/T/bitişik köşeler izler, açıklıklar yerinde,
  tek undo. `cad.walls.edit` `shift`/`distance` ve salt okunur
  `cad.walls.distances` (Python/MCP); çekirdek `ops::wall_shift`. 22+32 bağımsız
  durum, 4 araç testi, gerçek GUI. Kolon, duvar boyu, kilitli referans açık.

- **Döner merdiven (27 Ekim 2026; [ADR0709](../../docs/adr/0709-curved-stairs.md)).**
  Merdivenin iki noktalı yürüme çizgisi yay olabilir (`bulges`, KCAD 117 opt-in):
  rıhtlar yay boyunca radyal, bant ve korkuluk eş merkezli yay, `fit` yayı son
  rıhtta bitirir; ayna yayı çevirir. Döner merdiven aracı Merdiven ▾'da ve
  Merdiven'in Biçim'inde: merkez, başlangıç, imleçle biriken yön ya da yazılan
  dönüş; etiket/Öznitelikler yarıçap, sola/sağa ve iç basamağı söyler (bilgi).
  10+26 bağımsız durum, KCAD/Python/MCP, araç testleri; panel seçimlerinin
  (`right`, `clicked`, `fit`) Türkçe büyük harfle reddi bulundu, düzeltildi ve
  `setting_choices` ile korunur. Merkez direği/karışık kol açık.

- **Mahal boşluğu: şaft ve döşeme boşluğu (27 Ekim 2026; [ADR0708](../../docs/adr/0708-space-voids.md)).**
  Mahalin isteğe bağlı `void`'u (KCAD şema 116, opt-in): planda parçanın içinde
  kırpılmış çapraz (`geom::space::void_cross`), Metraj'da `spaceArea` dışı ve
  ayrı `voidArea`, Boşluk sütunu; denetimde küçük/dar bulgusu yok. Mahal
  aracında Boşluk (V), Öznitelikler'de Boşluk satırı; `cad.spaces.create/edit`.
  Ortak durumlar bağımsız başvurulardan, KCAD/Python/araç testleri, gerçek GUI
  resimleri. ARCH-ELEM-01 [~]: döşeme konturu, balkon/korkuluk, katlar arası
  şaft ve çizelge toplamı açık.

- **Aks ağı (27 Ekim 2026; [ADR0707](../../docs/adr/0707-grid-systems.md)).**
  `cad.grids.create`'in isteğe bağlı `system`'i: iki yönde aralık listeleri,
  açı, ilk adlar (1, A), uzatma (öntanımlı 1,5 m), balonlar ve kesişimlere
  akslarına bağlı kolonlar tek adımda; akslar kimliklerini yazılmadan önce
  alır. Aks ağı aracı (Aks ▾) imleçle gelir, tıklama yerleştirir, İki nokta
  açıyı imleçten alır; “3*6” aralık yazımı. 36 ortak durum, 3+1 araç testi,
  Python/MCP, gerçek GUI resimleri. ARCH-ELEM-03 [~]: eğrisel ağ, kolon hizası,
  L kolon ve eşit bölme açık.

- **Düz mutfak dizisi (27 Ekim 2026; [ADR0706](../../docs/adr/0706-kitchen-casework-runs.md)).**
  Yeni `cad.casework.run`: çizgi boyunca modül enli Alt dolaplar (sağ yanda
  aynalı), 0,90 m evye modülü, ocak, isteğe bağlı üst dolap, kalan için dolgu
  (uyarıyla) ve tezgâh alanı tek geri alma adımında; çizimde aynı adlı blok
  yoksa KentOS kitaplığınınki aynı adımda gelir. Mutfak dizisi aracı başı, sonu
  ve tarafı tıklatır, planı bloklarıyla önizler; Evye/Ocak Otomatik görünür.
  36 ortak durum bağımsız başvurudan, 3+1 araç testi, Python/MCP, gerçek GUI
  resimleri. ARCH-LIB-02 [~]: köşe/L-U tek komut, cihaz modülleri, duvar izleme açık.

- **Eğimden rampa ve korkuluk (27 Ekim 2026; [ADR0705](../../docs/adr/0705-ramps-by-slope-and-railings.md)).**
  `cad.ramps.create`'in `fit`'i son kolu |kot farkı| ÷ eğim uzunluğunda bitirir
  (eğimsiz, kot farkı sıfır ya da sığmayan rampa ret); üst üste sahanlık bir
  kez sayılır. Merdiven/rampa `railings` (sol/sağ/iki yan) KCAD şema 115
  opt-in; merdivende kesmenin üstü kesikli. Araç ve Öznitelikler'de görünür.
  ARCH-STAIR-02 [~]; ara sahanlık, kot izleme ve erişilebilirlik profili açık.

- **Yükseklikten 2B merdiven (27 Ekim 2026; [ADR0704](../../docs/adr/0704-stairs-from-height.md)).**
  `cad.stairs.create` isteğe bağlı `height`/`maxRiser` (rıht sayısı ⌈H/Rmax⌉,
  öntanımlı 0,175 m), `cutHeight` ve `fit` (son kol son rıhtta biter) alır;
  saklanan nesne ve KCAD aynı. Arada basamak sığmayan iki dönüş tek sahanlık;
  kesmenin üstü kesikli. Araç bir tıklama+yönle, görünür ayarlarla, komutun
  planını çizer. 30+14 ortak durum bağımsız başvurudan, 3 araç testi, gerçek
  GUI resimleri. ARCH-STAIR-01 [~]: kat görünümü, döner merdiven, kat
  yüksekliğine bağlılık ve kurum profili açık. Öntanımlılar yönetmelik değildir.

- **Mimari proje denetimi (27 Ekim 2026; [ADR0703](../../docs/adr/0703-architectural-project-audit.md)).**
  Salt okunur `cad.arch.audit` ve alt panelin Denetim sekmesi: birleşmeyen duvar
  (`cad.walls.joins`'un bulguları), öbür duvara `maxGap` kala biten ve birbirini
  ıskalayan uçlar (her kat ayrı), kopuk açıklık/mahal/ölçü/dizi/kolon/işaret,
  çok küçük ya da dar mahal, yinelenen numara, katsız nesne. Bulgu açık
  düzeltmesini var olan komutun çağrısı olarak taşır (`fix.step`); Düzelt tek
  geri alma adımı, denetim yinelenir. Bulgu yokluğu uygunluk değildir.
  35 ortak durum bağımsız başvurudan; Revit Review Warnings ve Archicad Update
  Zones araştırıldı. Yeni KCAD/birim/ölçek/render yok; ARCH-DOC-03 [~].

- **Mobilya, vitrifiye ve mutfak kitaplığı (27 Ekim 2026; [ADR0702](../../docs/adr/0702-furniture-and-fixture-library.md)).**
  KentOS blok kitaplığına gerçek boylu 21 blok (kimlik 17–37; ilk 16 bayt bayt
  aynı), vitrifiyelerde boşluk zarfı görünürlüğü (mevzuat iddiası değil), Blok
  ekle'de Duvara hizala (H): taban yüzde, ön odaya dönük; yazma mevcut
  `cad.blocks.insert`. ARCH-LIB-01 [~]; DWG blok kategorisi ve önizleme
  etiketi açık.

- **Açıklığın geçici ölçüleri (27 Ekim 2026; [ADR0701](../../docs/adr/0701-opening-temporary-dimensions.md)).**
  Seçili tek açıklığın imlecin yakın olduğu yüzde en yakın referanslara
  (yüz ucu, aynı kenardaki açıklık, kesen duvar yüzü) mesafeleri; sayıya
  basınca yazı kutusu, Enter `cad.openings.edit` `distance` ile taşır,
  referans sabit, tek geri alma. `cad.openings.distances` salt okunur sorgu;
  24+24 ortak durum. ARCH-EDIT-01 [~]: duvar/kolon geçici ölçüsü açık.

- **Ölçü dizisi duvarın her düz kenarında (27 Ekim 2026; [ADR0700](../../docs/adr/0700-dimension-sets-on-wall-edges.md)).**
  Tarifin `edge`'i kapalı ya da çok kenarlı duvarın düz kenarını adlandırır,
  `around` bütün düz kenarlara tek adımda dizi yazar; kenar köşe ekleme/silmede
  izlenir, izlenemezse dizi kopar. KCAD şema 114 (opt-in), ARCH-DIM-01 [~].

- **Stil tablosu geri almada (27 Ekim 2026; [ADR0699](../../docs/adr/0699-style-tables-in-undo.md)).**
  `cad.styles.edit` stilleri izleyen nesnelerle aynı geri alma adımında yazar;
  yalnız ad değişikliği de adımdır (Duvar stilleri'nde ad değiştirme geri alınır).

- **Dar pencerede Duvar stilleri (27 Ekim 2026; [ADR0698](../../docs/adr/0698-scrollable-wall-style-dialog.md)).**
  1100×682 pencere ve 18 px yazıda sağ form kaydırılır, alt eylemler görünür
  kalır; iptal taslağı bırakır. Eski ad değişikliğinin geri alınmaması
  ADR0699 ile kapandı. Nihai özellik kabulü verilmedi.

- **Büyük yazıda uygulama menüsü (27 Ekim 2026; [ADR0697](../../docs/adr/0697-bounded-application-menu.md)).**
  KentOS menüsünün komut listesi pencereye sığar ve tekerlekle kaydırılır;
  sol ve sağ bölme iç içe kaydırılmaz. Nihai özellik kabulü verilmedi.

- **Yakınlaştırmaya göre duvar ayrıntısı (27 Ekim 2026; [ADR0696](../../docs/adr/0696-model-wall-detail-and-dense-hairlines.md)).**
  Açık Şematik/Uygulama/Ayrıntılı seçimi korunur; otomatikte model alanı
  kameranın ekran paydasıyla 1:30/1:75 eşiklerini kullanır. `plotScale`,
  yazı, ölçü ve geometri değişmez; yoğun sıfır genişlikli GPU taraması
  düzeltildi. Kabul 29 Ekim ekiyle (bölünmüş görünüm, PDF matrisi; yukarıda).

- **Kalıcı ilişkili duvar ölçü dizileri (27 Ekim 2026; [ADR0695](../../docs/adr/0695-associative-wall-dimension-sets.md)).**
  `cad.dimensionsets.*`: tek düz duvarın yüzünde açıklık uçları/merkezleri,
  aralıklar ve toplam sırası; üyeler kalıcı duvar/açıklık yerlerine bağlı,
  aynı çift aynı ölçü kimliğini korur, kopan dizi görünür `broken` kalır, tek
  ölçü silmek parçayı bastırır. KCAD şema 113 (opt-in); GUI/Python/MCP aynı
  komutlar. ARCH-DIM-01'in ilk dilimi; kenar ve bütün kenarlar ADR0700'de.

- **Açıklığın gerçek noktalarına bağlı ölçü (26 Ekim 2026; [ADR0694](../../docs/adr/0694-opening-dimension-places.md)).**
  Mevcut ölçü kapı/pencere/boşluğun dört jamb köşesi ve açık başlangıç/
  bitiş/merkez başvurusunu okur; kanat/kasa parçasını kaynak saymaz.
  GUI'de bulunan kesilmiş duvar ucu sahipliği ortak snap'te düzeltildi;
  genişlik/konum/host kalınlığı/bölme/rehost aynı UID ile izlenir, kopuk
  açıklığın son görüntü köşesi geçerli bağ olmaz. Yayda doğrusal değer kiriştir.
  [Kabul](../../docs/baseline/2026-10-26-architecture-opening-dimensions/README.md):
  342 Python/76 MCP/809 commands; son524+269 core,63+709 interaction,
  52+224 application ve domain takımları; build/clippy/WASM/SDK geçti.
  Gerçek eski/yeni kenet, tek Enter/undo/redo/silme/Esc/yeni süreçte açma;
  36 ham ekran (iki tanısal),21 tam model ve dört ayrı tanısal kopya,
  eski1828 KCAD aynı. Açık13/koyu13-16 geniş/dar; tam matris açık.
  Yeni TS/şema/komut/birim/ölçek/render yok; ARCH-DIM-01 [~], otomatik
  ölçü sırası/üyelik ve çoklu görünüş/pafta kabulü ayrı açık iştir.

- **Görünür etiket içerikleri (26 Ekim 2026; [ADR0693](../../docs/adr/0693-visible-tag-content-examples.md)).**
  Etiket/Hepsini etiketle'de sekiz Hazır içerik, ayrı serbest Şablon ve
  kısa Özel değeri; Tür/Koru mevcut yenileme semantiğini korur. Seçimler
  yalnız taslaktır, mevcut cad.tags.create içerik metnine dönüşür.
  [Kabul](../../docs/baseline/2026-10-26-architecture-tag-contents/README.md):
  63+708 interaction, desktop build/iki pakette all-targets clippy; gerçek
  tek onay/undo/redo/kaynak kotu/toplu yenileme/ret/iptal/temiz kaydet-aç.
  34 ham ekran (yedi tanısal)/15 tam model, eski1810 KCAD baytı aynı.
  Açık13 geniş/koyu18 geniş-dar; tam matris ve ARCH-SPACE-02 [~] sürer.
  Yeni TS/şema/API/birim/ölçek/render yok; Python/MCP bu dilimde tekrar koşulmadı.

- **Toplu etikette kaynak katı (26 Ekim 2026; [ADR0692](../../docs/adr/0692-bulk-tag-levels.md)).**
  Mevcut `cad.tags.create/all.levels`: yok/null tüm katlar, açık kimlikler/
  atanmamış/birleşim; host duvarından açıklık katı, bilinmeyene kat uydurma
  yok. Koru/Yenile yalnız uygun kaynakları işler; görünür Kaynak katı ve
  hedef Etiket katmanı ayrıdır. [Kabul](../../docs/baseline/2026-10-26-architecture-tag-levels/README.md):
  26 yeni bağımsız durum/ilgili290, 340 Python/75 MCP, 809 commands,
  52+221 application/63+704 interaction; clippy/build/WASM/SDK geçti.
  Gerçek tek onay/ret/iptal/undo/redo/yeni süreçte açma, açık13/koyu18 geniş/
  dar; 43 ham ekran (tanısallar dahil)/27 tam model. Eski1781 KCAD aynı.
  Bir mevcut TS tipi güncel; yeni TS/TSX/kalıcı alan/birim/ölçek/render yok.
  ARCH-SPACE-02 [~]; çoklu görünüş/pafta bağlamı ve tam matris açık.

- **Toplu etikette Koru/Yenile (26 Ekim 2026; [ADR0691](../../docs/adr/0691-bulk-tag-refresh.md)).**
  Mevcut `cad.tags.create` isteğe bağlı `all.refresh`; yalnız hedef etiket
  katmanında açık alanları yeniler, kimlik/elle yer/kol/diğer katmanlar ve
  eski yok/null/false korunur. Görünür ayarlar ve sayılı plan, karma yazım
  tek undo; klavye hedef değişimi önce yeniden önizler. [Kabul](../../docs/baseline/2026-10-26-architecture-tag-refresh/README.md):
  ilgili264 ortak durum/31 yeni, 335 Python, 809 commands, son63+702
  interaction/51+221 application, gerçek MCP, clippy/build/WASM/SDK.
  35 ham ekran/24 tam model +bir tanısal disk; eski1756 KCAD baytı aynı.
  Üç mevcut TS tipi güncel, yeni TS/TSX/kalıcı şema/birim/ölçek/renderer yok.
  ARCH-SPACE-02 [~]; mimari kat/görünüş/pafta kapsamı ve tam matris açık.

- **Mahal proje listeleri ve açık kat öneki (26 Ekim 2026; [ADR0690](../../docs/adr/0690-project-room-choices.md)).**
  Projeyle sıralı ad/kullanım listeleri ve kata bağlı önek; ayrı öneri
  alanları serbest metni korur, kat değişimi taslak numarayı değiştirmez.
  Liste kaydı kaynakları yeniden adlandırmaz/numaralandırmaz; mevcut ayar
  geçmişi/tek mahal undo korunur. Opt-in KCAD 112; eski 1170 dosya baytı aynı.
  DXF'in tercih kaybı yazmadan önce ve ortak raporda uyarılır.
  [Kabul](../../docs/baseline/2026-10-26-architecture-room-choices/README.md):
  385 ortak durum/329 Python/dört MCP/iki desktop; native takımlar,
  clippy/build/WASM/1203 KCAD. Gerçek GUI açıklama yerini düzeltti;
  42 ham ekran/23 tam bağımsız kayıt. 30.000 nesnede debug tercih yazımı
  ortancası 15,183 ms; genel FPS garantisi değildir. İki mevcut TS tipi
  yenilendi, yeni TS/TSX dosyası yok. Koru/Yenile ADR0691’de; çoklu pafta/tam matris ve ARCH-SPACE-02 [~].

- **Numarasız mahal denetimi (26 Ekim 2026; [ADR 0689](../../docs/adr/0689-room-number-review.md)).**
  Mevcut etiket denetiminde görünür seçenek; ad/kat/UUID ve kaynak Göster.
  API yok/null/false eski sonuçları, tam metin gruplarını ve sırayı korur;
  sorgu modele/geçmişe yazmaz. [Kabul](../../docs/baseline/2026-10-26-architecture-space-identity/README.md):
  233 ortak durum/323 Python/üç MCP/iki desktop, native takımlar ve
  clippy/build/WASM/1170 KCAD; 18 ham ekran/12 tam bağımsız dosya.
  10.000 mahal+20.000 çizgide debug sorgu 241,663 ms, 9.998 eksikte 452,363 ms
  ortanca; genel ekran garantisi değildir. ARCH-SPACE-02 [~]; kat öneki/
  ad/kullanım listeleri, Koru/Yenile ve tam matris açık. Yeni TS/TSX dosyası,
  KCAD alanı/birim/ölçek/renderer değişikliği yoktur.

- **Ayrı net mahal etiketi (26 Ekim 2026; [ADR 0688](../../docs/adr/0688-room-tag-net-area.md)).**
  `{area}` seçilmiş sınırda kalır; `{netArea}` aynı kaynak/kat/kapsam/kolon
  tercihinin iç yüzlerini okur. Kopuk/çizilmiş non-net bilinmiyorsa boş;
  kaynak/ayar/geçmiş/dış güncellemede metin izler, elle yer korunur.
  [Kabul](../../docs/baseline/2026-10-26-architecture-tag-net-area/README.md):
  225 ortak durum/30 yeni, 318 Python, 68 domain/697 interaction/219
  application/iki MCP; clippy/build/WASM/1170 KCAD, 21 ham ekran/15 tam
  bağımsız kayıt. 200 mahal/etikette debug hareket ortancası26,854ms;
  +20.000 çizgi/legacy kolon Düşme yolunda348,204ms, genel garanti değildir.
  ARCH-SPACE-02 [~]; listeler/KoruYenile/tam matris açık. Yeni TS/şema/birim/
  ölçek/renderer yoktur.

- **Mahal kotu etiketleri (26 Ekim 2026; [ADR 0687](../../docs/adr/0687-room-tag-elevations.md)).**
  Mevcut şablonda bilinen kat kotu ve yalnız açık mahal yüksekliğinden tavan
  (M40'tan beri bitmiş döşeme ve tavan: ADR 8040 §3);
  bilinmeyen boş, katlar arası yükseklik tavan sayılmaz. Ayar/geçmiş/dış
  değişiklikte yazı yenilenir, elle konum korunur. Gerçek GUI sayısal
  yükseklik taşmasını yakalayıp düzeltti. [Kabul](../../docs/baseline/2026-10-26-architecture-tag-elevations/README.md):
  195 ortak durum, 309 Python, 697 interaction/219 application/67 domain/
  bir MCP; clippy/build/WASM, 19 ham ekran/on bağımsız tam kayıt.
  ARCH-SPACE-02 [~]; ayrı net alan, listeler, Koru/Yenile ve tam matris açık.
  Yeni TS/şema/birim/ölçek/renderer yoktur.

- **Yeni araçta ayar başlangıcı (26 Ekim 2026; [ADR 0686](../../docs/adr/0686-reveal-new-tool-settings.md)).**
  Masaüstü Öznitelikler gövdesi tek scroll'unu adresler; ayarlı yeni araç
  başlatma üstü gösterir, aynı araçta düzenleme/önizleme kaydırmayı korur.
  [Kabul](../../docs/baseline/2026-10-26-architecture-settings-scroll/README.md):
  eski 508 px kusuru, 62 ilgili test/clippy/build; gerçek koyu13 geniş/açık18
  dar geçiş, özellik/undo/panel kapat-aç/iptal; 19 ham ekran/beş tam dosya.
  Genel docking/matris açık; model/ölçek/render/komut/SDK/TS değişmedi.

- **Görünür etiket ayarları (26 Ekim 2026; [ADR 0685](../../docs/adr/0685-visible-tag-settings.md)).**
  Mevcut `cad.tags.create` ile şablon/çerçeve/kol/kağıt mm boyu; panelde yeni
  ayar/seçim ayrıdır. Gerçek GUI'nin yakaladığı ilk yapıştırma modifier
  önbelleği CommandLine/EditCell'de düzeltildi, uzun komut seçeneği özetlendi.
  [Kabul](../../docs/baseline/2026-10-26-architecture-tag-settings/README.md):
  697 interaction/40 UI-host/üç mevcut ayar/üç mevcut Python, clippy/build;
  26 ham ekran/11 bağımsız tam dosya, ret/tek onay/undo/redo/izleme/kaydet-aç.
  Önceki panel kaydırmasını yeni araca taşımama ADR0686 ile düzeltildi;
  zengin etiketler açık, ARCH-SPACE-02 [~]. Yeni komut/şema/SDK/TS/birim/render yoktur.

- **Mahal etiketi konum tutamacı (26 Ekim 2026; [ADR 0684](../../docs/adr/0684-tag-grip-placement.md)).**
  Tag'ın genel geometriye çevrilemeyen yazımı mevcut anlamsal Move'a yönlenir;
  yalnız sıcak UUID'ler, kaynak/şablon/ankraj korunur. Karışık sıcak tür ve
  sonradan kilitlenen katman atomik rettir. [Kabul](../../docs/baseline/2026-10-26-architecture-tag-follow/README.md):
  gerçek sürükle→40 cm duvar→20.16 m², kaynak silme, şablon, undo/redo,
  önizleme/iptal/kaydet-aç; 22 ham görüntü/18 bağımsız dosya, 692 interaction,
  219 application/303 Python/bir mevcut MCP testi; clippy/build. ARCH-SPACE-02
  [~]; Koru/Yenile, kot şablonları ve tam matris açık. Yeni TS/şema/render yoktur.

- **Odaklı komut/özellik alanında Kaydet (26 Ekim 2026; [ADR 0683](../../docs/adr/0683-command-line-chords.md)).**
  CommandLine ve EditCell, Cmd/Ctrl karakter kısayollarını iç metin girişinden
  önce host'a bırakır; A/C/X/V, AltGr ve taslak korunur. Kaydet tamponu
  onaylamaz; mevcut Enter/odak kaybı değişmez. [Kabul](../../docs/baseline/2026-10-26-commandline-save/README.md):
  gerçek eski/yeni hatası, 38 ilgili test, clippy/build; 15 ham görüntü ve
  dokuz bağımsız disk kopyası. Diğer giriş bileşenleri ve tam mimari matris
  açık; komut/model/Python/MCP/TS değişikliği yoktur.

- **Açık mahal numaralandırma (26 Ekim 2026; [ADR 0682](../../docs/adr/0682-explicit-room-numbering.md)).**
  Tıklama/yön izdüşümü, tam ilk numara ve sıfır dolgusu; UUID/etiket/çizelge
  tek undo'da korunur. Seçim dışı aynı tam numara atomik reddir; plan/Python/MCP
  aynı hesabı kullanır. [Kabul](../../docs/baseline/2026-10-26-architecture-space-number/README.md):
  68 bağımsız durum, 300 Python, 270 application/6 interaction/7 MCP;
  52 ham görüntü ve 20 disk kopyası (tanısal olanlar ayrıdır), gerçek tek onay,
  undo/redo/kaydet-aç. GUI'de bulunan boş iç seçim ve uzun önizleme düzeltildi.
  ARCH-SPACE-02 [~]; listeler/etiket şablonları/tam matris açık. Yeni TS yoktur.

- **Metraj CSV ve kayıt sayısı (26 Ekim 2026; [ADR 0681](../../docs/adr/0681-selected-quantity-csv.md)).**
  CSV açık sekmeyi, XLSX/ODS aynı süzgeçte üç tabloyu yazar; bu ayrım görünürdür.
  Ortak desktop dosya bildirimi CSV'nin gerçekten yazılan ilk sayfasını sayar.
  [Kabul](../../docs/baseline/2026-10-26-architecture-quantity-export/README.md): gerçek
  CSV 200/400/0, kitaplar üç sayfa/600; iptal/yeniden açma, bağımsız dosya/model,
  17 ham görüntü ve üç desktop testi/clippy/build. Uzun sayı→metin hassasiyet
  politikası aynıdır. ARCH-DOC-02 [~]; şema/Python/MCP/TS değişikliği yoktur.

- **Sorgu ömründe ortak metraj araması (26 Ekim 2026; [ADR 0680](../../docs/adr/0680-query-scoped-room-quantity-search.md)).**
  Kaynak UUID/kat/kapsam ve belge sırası korunarak aynı raporun sınır ağları
  paylaşılır; immutable sorgu dışında cache yaşamaz, legacy rota aynı kalır.
  100 eski/yeni tam cevap eşit; 300 Python, 268 application/103 domain/6 MCP.
  Aynı 400 mahal/21.000 nesneli debug belgede sıcak sorgu ortancası
  8646,943→83,765 ms; yeni süreç ilk sorgusu 815,623 ms ayrıca kayıtlıdır.
  Release/p95/bellek/GPU garantisi değildir. Tablo genişliği ve sağ/alt
  kaydırma kenarı 22 ham görüntü/gerçek CSV/bağımsız KCAD ile doğrulandı;
  [kabul](../../docs/baseline/2026-10-25-architecture-quantity-cache/README.md).
  ARCH-SPACE-01/PERF-01 [~]; genel araçlar ve tam matris açık. Yeni TS yoktur.

- **Kalıcı mahal ayırıcı ve güvenilir metraj uyarısı (25 Ekim 2026; [ADR 0679](../../docs/adr/0679-virtual-room-separators.md)).**
  Görünür Mahal ayırıcı fiziksel kalınlıksız analitik sınırdır; kat, tadilat durumu, kaynak
  UUID'si, mahal→etiket→çizelge tek undo'da korunur. Genel destekli edit/kopya
  rolü taşır, desteklenmeyen temsil ve blok içine alma atomik reddir.
  `cad.spaces.separators`/Python/MCP, opt-in KCAD 111 ve DXF ortaktır.
  Her kopuk mahal Metraj/API/CSV'de son sınır uyarısı taşır; geçerli raporun
  eski sütunları korunur. [Kabul](../../docs/baseline/2026-10-25-architecture-space-separators/README.md):
  105 bağımsız durum, 300 Python, 35 GUI kaydı ve 76 ham görüntü.
  ARCH-SPACE-01 [~]: metraj sorgu optimizasyonu ADR 0680'dedir; genel araç denetimi ve bütün
  iş akışlarının tam matrisi açık. Yeni TS/TSX uygulama kaynağı yoktur.

- **Mahalın kalıcı kaynak seçimi (25 Ekim 2026; [ADR 0678](../../docs/adr/0678-persistent-room-source-selection.md)).**
  Açık duvar/kolon UUID alt kümesi kat/kapsamla birlikte saklanır; kaynak
  kaybı son sınırı/uyarıyı korur. Görünür tek/toplu kaynak tercihi ve mevcut
  mahal listesi önizleme/tek onay/undo yolunu kullanır. Tam kopya grupları
  UUID'lere yeniden bağlanır, kısmi gruplar bağımsız çizilmiş sınıra dönüşür
  ve uyarılır. Python/MCP, opt-in KCAD 110 ve DXF kimlik eşlemesi ortaktır.
  [Kabul](../../docs/baseline/2026-10-25-architecture-space-sources/README.md):
  63 bağımsız ortak durum, 30 GUI kaydı, 53 ham görüntü; yeni TS/TSX kaynağı yok.
  ARCH-SPACE-01 [~]: sanal ayırıcı, uçtan uca performans ve tam matris açık.

- **Kolon/merdiven/rampa özellik birimleri (25 Ekim 2026; [ADR 0677](../../docs/adr/0677-architectural-element-property-units.md)).**
  CAD fiziksel en/boy/rıht/basamak/sahanlık/yükseklik/kot açık uzunluk ekini
  korur; açı/adet/yüzde/kağıt mm/CBS legacy anlamı aynı. Gerçek kolon `40 cm`
  kusuru ve bağlı mahal, tek onay/ret/Escape/undo/redo/imperial boy doğrulandı;
  10 bağımsız kayıt, 41 özellik başarılı/4 atlanan. Genel merdiven/rampa
  tasarımcısı ve tam GUI matrisi kabulü değildir; yeni TS/şema yoktur.

- **Duvar/açıklık özellik birimleri (25 Ekim 2026; [ADR 0676](../../docs/adr/0676-architectural-property-length-units.md)).**
  Yalnız CAD fiziksel uzunluk hücreleri ortak okuyucuyla açık mm/cm/m/in/ft
  ekini korur; birimsiz giriş çizim birimindedir. Gerçek `25 cm` duvar ve
  `120 cm` kapı kusuru yeniden üretilip düzeltildi. 29 ham görüntü/26 bağımsız
  kayıt, tek onay/ret/Escape/undo/redo; 40 özellik testi başarılı, 4 atlandı.
  Model komutları, açı, CBS legacy yolu, ölçek ve şema korunur. Diğer mimari
  nesne uzunluk alanları ve tüm GUI matrisi açık; ARCH-BASE-01 [~].

- **Eksiksiz CAD ölçü okuma ve mahal yüksekliği (25 Ekim 2026; [ADR 0675](../../docs/adr/0675-readable-cad-measurements.md)).**
  Salt okunur birimli sayılar hücreye sığmazsa tam genişlikte büyür, sığarsa
  eski iki sütuna döner; basamak/birim/ölçek değişmez. Gerçek GUI testinde
  bulunan mahal yüksekliğinin `3 m` ekini atma kusuru yalnız CAD'de ortak
  uzunluk okuyucusuyla düzeltildi; birimsiz değer çizim biriminde, geçersiz ek
  atomik rettir. 22 ham görüntü/11 bağımsız kayıt, sekiz görünüm ve gerçek
  tek Enter/Escape/undo/redo doğrulandı. Diğer özelliklerin eski okuyucusu,
  uzun adları ve tüm mimari GUI matrisi açık; ARCH-BASE-01 [~].

- **Atanmamış mahal kaynakları ve iç nokta (25 Ekim 2026; [ADR 0674](../../docs/adr/0674-unassigned-space-sources.md)).**
  Yeni açık `unassigned` kapsamı yalnız kat atanmamış duvar/kolonları kullanır;
  eski boş Kat/Tüm katlar davranışı aynıdır. Tek/toplu araç, Öznitelikler,
  follower, Python/MCP ve opt-in KCAD 109 aynı kapsamı korur. Yeni İç nokta seç
  görünür önizleme ve tek onayla mevcut rebound komutunu kullanır; duvar gövdesi
  oda değildir. Güvenli olmayan kapsam değişimi atomik ret, kaynak kaybı son
  sınır/uyarı, undo/redo aynı UUID'dir. [Kabul](../../docs/baseline/2026-10-25-architecture-space-scope/README.md):
  56 gerçek görüntü, 29 bağımsız kayıt; dar 1100×650, geniş yaklaşık 1440×825,
  iki tema/yazı. ARCH-SPACE-01 [~]: seçilmiş kaynak listesi/sanal ayırıcı,
  performans ve tüm iş akışlarının tam matrisi açık. Yeni TS/TSX kaynağı yok.

- **Kalıcı mahal kolon düşümü (25 Ekim 2026; [ADR 0672](../../docs/adr/0672-space-column-deductions.md)).**
  Düşme/Yalnız net/Her yöntemde açık tercihi analitik kolon birleşimini yalnız
  ilgili katın duvar yüzünden düşer. Eski mahal/API varsayılanı korunur; yeni
  GUI varsayılanı Yalnız net. Aks→kolon→mahal→etiket/çizelge aynı komut/undo;
  tam kaplanma son geçerli sınırı ve açık uyarıyı korur. Etkin tercih KCAD 103,
  eski dosya aynı bayt/şemadır. Python/MCP ortak komutları kullanır.
  Gerçek GUI’de bulunan daire önizleme dolgusu ve dar panel uyarısı düzeltildi;
  [kabul kaydı](../../docs/baseline/2026-10-25-architecture-space-columns/README.md).
  ARCH-SPACE-01 [~]: sanal sınır, kaynak alt kümesi, uçtan uca performans ve
  bütün iş akışlarının tam matrisi açık. Yeni TS/TSX kaynak dosyası yoktur.

- **CAD GPU konum değişmezliği ([ADR 0673](../../docs/adr/0673-invariant-cad-gpu-positions.md)).**
  Ortak CAD WGSL konum çıktıları `@invariant` taşır. Apple M5/Metal’de eski
  gerçek GPU testi başarısızken aynı toleransla yakın/sıfır/uzak origin ve iki
  eksen geçer; gerçek önizleme/zoom/pan hizası doğrulandı. Birim, ölçek, kamera
  ve buffer biçimi değişmedi. Diğer donanımlar ve performans için evrensel
  kabul değildir.

- **Önizlemeli toplu mahal (25 Ekim 2026; [ADR 0671](../../docs/adr/0671-bulk-room-discovery.md)).**
  `cad.spaces.discover` açık kat/Tüm katlar ağından analitik kapalı boşlukları
  bulur; duvar malzemesini oda saymaz. Bölge yalnız temsil noktasıyla oda seçer.
  Mevcut mahal UUID'leri korunur; elle çizilmiş kısmi çakışma bütün yazmayı
  reddeder. Ayrı Mimari aracı, Python/MCP plan/execute, tek onay/undo ve boş
  tekrar semantiği vardır. Çoklu oluşturma/follower ortak geçici sorgu dizinini
  paylaşır; eski yakın/tam arama korunur. ARCH-SPACE-01 [~]: kolon, kalıcı sanal
  sınır/kaynak alt kümesi ve tam GUI/uçtan uca performans kapıları açıktır.
  KCAD şeması/birim/render değişmez; yeni TS/TSX kaynağı yoktur.

- **Mahal sınırında açık kat (25 Ekim 2026; [ADR 0670](../../docs/adr/0670-level-bound-architectural-spaces.md)).**
  `space_found`, açık `arch.level` için yakın ve tam aramada yalnız aynı kimliğin
  duvarlarını alır; kot eşitliği yeterli değildir. Katı olmayan legacy mahal ve
  çağrıların bütün çizim davranışı korunur, araçta **Tüm katlar** yazılır.
  Mahalın görünür Kat (L) seçimi mevcut komut alanını kullanır; kat değişimi
  plan/follower/tek undo'da sınırı yeniden bulur, kapanmazsa lostEnclosure korunur.
  ARCH-SPACE-01 [~]; sonraki toplu bulma dilimi ADR 0671'dedir; kolon/sanal
  sınır ve tam GUI matrisi açıktır.
  Yeni sözleşme/TS türü/KCAD şeması yoktur.

- **Büyük mimari kontur ağı (25 Ekim 2026; [ADR 0669](../../docs/adr/0669-large-architectural-wall-networks.md)).**
  Duvar ağı aynı arrangement noding hesabında seyrek kaynak sahipliği kullanır;
  mevcut overlay yoğun yolu ve toleransı korunur. Açık kaynak/parça/hücre
  kotaları sayım sınırıdır, kesin RAM garantisi değildir. Kontur önizlemesi
  400 duvarda kesilmez; eski Tek tek/Çiftler sınırı korunur. 48 karışık kaynak
  birleşimi ve gerçek 1.600 oda → 3.276 duvar/11.480 m kabulü; çekirdek release
  bellek/süre ayrı ölçülür. ARCH-DRAW-03 [~]: tam GUI matrisi, uçtan uca
  performans ve diğer kat iş akışları açık. Yeni sözleşme/TS/KCAD şeması yoktur.
  Kabul: `docs/baseline/2026-10-25-architecture-wall-networks/`.

- **Duvar dönüşümünde açık kat (25 Ekim 2026; [ADR 0668](../../docs/adr/0668-architectural-wall-level-isolation.md)).**
  Mevcut `cad.walls.convert.arch`, Each/Pairs/Contours çıktılarının mimari bilgisidir;
  bilinmeyen kat atomik ret. Görünür Kat (L) seçimi yeni araç oturumunda katsız başlar;
  kaynak özniteliğinden tahmin edilmez. Ortak ağ/tekrar aynı katman ve katta;
  köşe/T/çapraz birleşimleri aynı semantik kat kimliği içinde, katsızlar ayrı gruptur.
  KCAD şeması/FP64/birim/ölçek/kamera/render değişmez. ARCH-DRAW-03 [~]; büyük ağ,
  geniş karışık kaynak/tam GUI matrisi ve diğer kat iş akışları açık.
  Kabul: `docs/baseline/2026-10-25-architecture-wall-levels/`.

- **Ortak oda sınırından tek duvar ağı (25 Ekim 2026; [ADR 0667](../../docs/adr/0667-shared-room-wall-boundaries.md)).**
  Mevcut `cad.walls.convert(mode=contours)`, aynı hedef katmanda Eksen ile
  tam/kısmi düz ve analitik yaylı sınırı tekilleştirir. Mevcut arrangement
  toleransı **1e-6 m**; uyarı toleransı, paylaşım uzunluğunu ve ilk kaynak
  görünüm/öznitelik sahipliğini söyler. İç net/Dış ölçü veya asimetrik stil
  çelişkisi atomik ret. Mevcut duvar/host, birleşim, undo, Python ve MCP yolu;
  yeni dosya şeması yok. ARCH-DRAW-03 [~]: kat semantiği, büyük ağ ve tam GUI
  matrisi açık. Kabul: `docs/baseline/2026-10-25-architecture-shared-wall-contours/`.
- **Kapalı konturdan duvar (25 Ekim 2026; [ADR 0666](../../docs/adr/0666-closed-contour-walls.md)).**
  Mevcut `cad.walls.convert(mode=contours)` dış/delik halkalarını yönlendirir; İç net/Dış ölçü/Eksen ve Koru/Sil
  görünür ayarlardadır. Önizleme aynı native plandır; yazma tek undo, hatalı kaynak kümesi atomik ret.
  Eski Each/Pairs korunur. Değen ortak sınırları birleştirme ve farklı kat entegrasyonu açık;
  ARCH-DRAW-03 [~], tam gerçek pencere matrisi bitmiş sayılmaz. SI/ölçek/import/render değişmez.

- **Açıklık dağıtımı (25 Ekim 2026; [ADR 0665](../../docs/adr/0665-hosted-opening-arrays.md)).**
  Mevcut `cad.openings.edit.array`, kaynağı koruyarak aynı hostun analitik yolunda
  yeni kopyaları adet/aralık veya uç paylarıyla dağıtır; isteğe bağlı benzersiz numaralar.
  Mimari/Açıklık şeridindeki ayrı araç aynı planı gösterir; ret kısmi yazmaz, tek onay/undo.
  Python/MCP/bağımsız hesap ve gerçek düz/yaylı GUI işlevsel kabulü geçti;
  ARCH-OPEN-02'nin tam gerçek boyut/font/tema ve birleşik gezinme matrisi açıktır.


- **Native fare olay grubu (25 Ekim 2026; [ADR 0664](../../docs/adr/0664-native-pointer-event-batches.md)).**
  Viewport basış/bırakış/teker konumunu aynı Iced teslimindeki hareket sırasından alır;
  grubun son cursor konumuyla basış başlamaz. Menü/erişilemeyen cursor kapısı korunur,
  kare/çıkış/bounds kaydı temizler. Kamera matematiği/model/ölçek değişmez.
  Tek tek olay veren Snapshot::input bu hatayı sınamaz; toplu update regresyonu vardır.

- **Açıklık merkez tutamacı (25 Ekim 2026; [ADR 0663](../../docs/adr/0663-hosted-opening-centre-grip.md)).**
  Tek açıklığın Esnet tutamacı mevcut `cad.openings.edit` Rehost/Copy yolunu kullanır;
  genel geometriye çevrilemeyip sessizce atlanmaz. Karışık sıcak tutamaç seçimi açık ret verir,
  kısmi yazmaz. Yeni TS/SDK/model yoktur. Birleşik native ve gerçek kaydet/aç kabulü geçti;
  tam gerçek pencere matrisi ve Kaydır kabulü açık kalır.

- **Hassas mimari açıklık yerleştirme (25 Ekim 2026; [ADR 0662](../../docs/adr/0662-precise-opening-placement.md)).**
  Aynı `cad.openings.create` komutuna isteğe bağlı mesafe referansı ve iki pervaz girdisi eklendi;
  eski v1 davranışı korunur. GUI tür/en örnekleme ve analitik host boyunca net pay gösterir.
  KCAD nesne modeli/ölçek/render değişmez. Tutamaç/host/bölme kabulü ADR 0663’te takip edildi; tam gerçek ekran matrisi
  ve kaydırma kabulü açıktır. Gerçek özet/durum çubuğu/kayıt ölçeği 1:100'dür.

- **Tek uzunluk alanında birimli giriş (25 Ekim 2026; [ADR 0661](../../docs/adr/0661-formatted-single-length-fields.md)).**
  Gösterilen `0.300 m`, boşluklu metrik ve ondalık `in`/`ft` ortak `parse_length` ile okunur.
  Nokta/koordinat ve iki uzunluk söz dizimi korunur; SI model/ölçek/import/render değişmez.
  Gerçek Enter/blur/iptal/ret ve kaydetme, bağımsız eski nesne korunumu ile doğrulandı.

- **Mimari duvar aday dizini (24 Ekim 2026; [ADR 0660](../../docs/adr/0660-architectural-preview-candidates.md)).**
  Native komşuluk araması geçici FP64 PackedTree kutularıyla daralır; belge sırası ve güncel birleşim hesabı korunur.
  Duvar yazma/silme ve undo/redo/iptal/dış edit geçersiz kılar; okuma kopyaları bağımsız yenilenir. KCAD/schema değişmez.
  Soğuk kurulum maliyeti ve sıcak kazanç ayrı ölçülür. ARCH-PERF-01'in gerçek pencere/GPU kabulü hâlâ açıktır.

- **Güvenli mimari şablonu (24 Ekim 2026; [ADR 0659](../../docs/adr/0659-safe-architectural-template-preview.md)).**
  GUI önce fark gösterir, birim/çizim ölçeğini koruyarak açılır. Açık `settingsMode=keep/template` mevcut
  tanım/font/katman durumlarını korur; bütün paket ayar ve paftalarla tek undo/redo adımıdır.
  Sistemli CAD metreyle kalır; GUI ofis seçeneği bu projede m/1:50 gönderir. Eski alanı göndermeyen v1
  otomasyonunun davranışı korunur. Plan/revision/oturum kontrolü aynı native komutta Python/MCP'ye açıktır.


- **Mimari şerit ve dar CAD durumu (24 Ekim 2026; [ADR 0658](../../docs/adr/0658-architectural-ribbon-and-narrow-cad-status.md)).**
  Mimari araçlar Çiz/Yerleştir/Düzenle/Belgele/Proje gruplarındadır. Mimari bağlamlar adları koruyarak daralır;
  kapalı Öznitelikler paneli `view.properties` ile seçim/taslak bozulmadan gösterilir. Dar başlık etkin sekmeyi
  açığa çıkarır; Yardımcılar menüsü çizim taslağını korur. Diğer sekmeler/CBS opt-in olmayan yerleşimi korur.

- **Görünür mimari ayarlar ve ölçülü düz bölme (24 Ekim 2026; [ADR 0657](../../docs/adr/0657-visible-architecture-settings-and-measured-partitions.md)).**
  Etkin aracın yeni nesne ayarları seçim özelliklerinden ayrıdır; Enter/blur mevcut parser ile uygular, taslak belgeye yazılmaz.
  `cad.walls.partition` açık kaynak/sınır kenarlarıyla net veya eksen mesafesinde tek normal duvar üretir; GUI/Python/MCP/headless ortaktır.
  Düz kaynak ilk dilimdir; kalıcı uzaklık kısıtı oluşturmaz. ADR 0626'nın ölçek/yazı/render kararları korunur.

- **Dikdörtgen mimari duvar (24 Ekim 2026; [ADR 0656](../../docs/adr/0656-rectangular-architectural-walls.md), `ARCH-DRAW-01`).**
  Ayrı `tool.wallRectangle` / `cad.walls.rectangle`, İç net / Dış ölçü / Eksen referansıyla tek kapalı mevcut duvar üretir.
  Yeni komutun yolu ilk köşesi korunarak CCW düzenlenir; önizleme ve yazma mevcut şekil/duvar uygulayıcılarını kullanır.
  Serbest Duvar, normal Dikdörtgen, render ve ADR 0626'nın metre/1:100, tek çizim ölçeği ve gerçek boy yazı kararları korunur.
  İlk aracın işlevsel kabulü ve gerçek ekranları `docs/baseline/2026-10-24-architecture/` içindedir; genel mimari yol haritası açıktır.
- **CAD blok XYZ dönüşümü (23 Ekim 2026; [ADR 0625](../../docs/adr/0625-cad-block-xyz-placement.md)).**
  Eğik/iç içe INSERT dönüşümleri önce tam XYZ'de birleştirilir; desteklenen düz nesnelerin açık patlatmasında
  XY ve kotlar korunur. SOLID/TRACE/3DFACE köşe kotları atılmaz. Eğik yay/genişlik için genel afin kaynak
  modeli henüz yoktur; mevcut varsayılan kayıp reddi korunur ve CAD-IO-15 açık kalır.
- **CAD açılışı ve birimler (23 Ekim 2026; [ADR 0624](../../docs/adr/0624-cad-units-direct-open-and-property-input.md)).**
  - Yeni yerel CAD'in başlangıcı mm / 1:1 yazı ölçeğidir; kaydedilmiş açık kullanıcı birimi tercihi korunur.
    Gerçek koordinatlı CAD ve GIS metre/harita ölçeğiyle devam eder. Model gerçek boyuttadır; pafta ve yazı ölçeği ayrıdır.
  - DWG/DXF/NCZ, Yeni proje olmadan Aç ile ayrı CAD çizimi olur; kaynak birimi dosyadan gelir. Kaydet KCAD ister,
    özgün CAD kaynağına yazmaz. Native `cad_open`, desktop/Python/MCP'nin ortak yoludur; birim ve okuma raporu verilir.
  - Bildirilmeyen kaynak birimi mm varsayımıyla ve açık raporla okunur; yanlış başlık gerçek uzunluk bilinmeden
    tahminle düzeltilmez. İçe aktarın açık kaynak birimi/bilinen ölçü yolu ADR 0621'deki gibi sürer.
  - Özellik hücresi yalnız yazı/düzenleme durumu değişince yeniden yerleşir; tek karmaşık nesnenin geometrik
    özellikleri de belge/seçim değişene kadar önbellektedir. Eşzamansız ek dosya eylemleri canlıdır.
- **Web katmanına ara (sahibin kesin kararı, 12 Ekim 2026; [ADR 0228](../../docs/adr/0228-web-pause-desktop-and-cloud-first.md)).**
  - Web'in son hâli **`1aad00d0`**'dır. Bundan sonra maddeler yalnız masaüstünde ve gerektiğinde bulutta (`kentosd`), Python'da ve
    MCP'de yapılır.
  - `apps/web/`'e yeni özellik yazılmaz. Web'in denetimleri (typecheck, Vitest, build, e2e, `e2e:interaction`, resimler, `pnpm inventory`)
    madde takımında koşmaz; `docs/inventory/web.*` `1aad00d0`'daki hâliyle donar.
  - Web'e masaüstü ve bulut tamamen bitince, sahibin sözüyle dönülür. Geri dönüşün planı TODOS.md §0.2 “Web'e ertelenenler”dir: yalnız
    masaüstünde biten her madde oraya satırını yazar.
  - Aynen sürenler:
    - ortak çekirdek (`crates/shared/*`) ve sözleşmeler (TS türleri ve katalog üretilmeye devam eder);
    - bağımsız Python başvuruları;
    - ortak fixture'lar ve etkileşim izleri (masaüstünde oynatılır);
    - WASM bağlayıcıları yalnız derlenir, yeni işleve bağlayıcı yazılmaz.
  - Yeni komutların arayüz kaydı (ad, ikon, açıklama, kısayol) web'in envanteri yerine `catalog/v1/additions.json`'dadır
    (ilk tüketicisi `GIS-40`); şeritteki yeri masaüstünün şerididir: `catalog/v1/ribbon.json` (`scripts/ui/ribbon.py`, ADR 0622).
  - Kod, ileride mobil (önce görüntüleme) ve web eklenecekmiş gibi modüler kalır:
    - hesap `crates/shared`'te;
    - arayüzden bağımsız uygulama katmanı `crates/native`'te (Iced, wgpu ve pencere bilmez; `pnpm arch:deps` zorlar);
    - çizim hattı `crates/render/wgpu`'da;
    - Iced yalnız `crates/ui`'da ve kabuklarda. Masaüstü kabuğu incedir: sunum olmayan iş `crates/native`'e ya da `crates/shared`'e yazılır.
- **Arayüz düzeni (sahibin isteği, 23 Ekim 2026; [ADR 0622](../../docs/adr/0622-interface-arrangement.md), dal `ux/arayuz-duzeni`).**
  - Proje türü projeyle başlar, değişmez. CAD ile CBS arasında uygulama içinde geçiş yoktur; ileride dönüştürme olacak.
  - Metrelik CAD çizimi plandır (1:1000 ile başlar). Sihirbaz yazı yüksekliklerini çizimdeki boylarıyla söyler. CAD'in durum
    çubuğunda çizim ölçeği vardır.
  - Masaüstünün şeridi kendi dosyasındadır (`catalog/v1/ribbon.json`), web'in donmuş şeridini okumaz:
    - CAD sekmeleri: Dosya, Giriş, Plan, Ölçme, Ekle, Açıklama, Değiştir, Mimari, Görünüm, Yönet, Çıktı.
    - CBS sekmeleri: Dosya, Harita, Düzenle, Veri, Analiz, Ölçme, Görünüm, Çıktı.
    - Her düğme 1440 px'te yazılıdır. Bir komut ana sekmelerde bir kez geçer.
    - İşlem araçları ana sekmelerde değil, araç kutusunda ve bağlamsal sekmelerdedir.
  - Seçim türüne göre bağlamsal sekme açılır ve öne gelir (`drafting.contextualTabs`): Nokta, Çizgi, Alan, Yazı, Ölçü,
    Kılavuz, Tarama, Blok, Tablo, Resim, Raster, Nokta bulutu, Duvar, Açıklık, Mahal, Mimari nesne. Karışık seçimde Seçim
    sekmesi açılır.
  - Sağ tık AutoCAD'deki gibi o anki işin menüsünü açar ve hiç nokta koymaz; tutamaçla taşırken tutamaç menüsü açılır.
    Kısa sağ tıkın Enter olması `drafting.rightClickEnter` tercihidir, kapalı başlar. Ortak izler sağ tıkı Enter olarak
    oynatır.
  - `drafting.pythonEcho` açıkken her ürün komutu ve elle yapılan her seçim Python konsoluna "Komut" satırı olarak,
    çalıştırılabilir kodla yazılır (`python/echo.rs`, `python_call::python_echo`). Seçimin SDK karşılığı
    `Document.select` ve `Document.selection`'dır. Native oturum ve MCP aynı seçimi sunar: `drawing.select`, `drawing.selection`
    (ADR 0622 §9); kimlik listesi bütünüyle doğrulanır, tekrarları teke iner.
  - Katman türü (`LayerNode.geometry`: nokta, çizgi, alan, açıklama; yoksa Karışık; `.kcad` şema 102): türlü katmana
    başka türden yazma çizimin denetiminde `layer_geometry_mismatch` ile reddedilir. `cad.layers.edit`'in `geometry`'si
    türü verir. Ağaçta türün işareti katmanın rengindedir; uymayan çizim araçları kapalıdır (`layer_types.rs`).
  - Otomatik seçim (`ProjectSettings.autoSelect`, `edit.autoSelect`, `cad.settings.set`'in `autoSelect`'i): kapalıyken
    seçen her adım yalnız etkin katmanın nesnelerini alır (`selectable::only_layer`). CBS'de kapalı, öbür projelerde açık
    başlar.
  - Araçların Python ve MCP denetimi (§8): CAD manifesti kapsamı masaüstünün şeridinden okur, İşlemler araçlarını
    `cad.processing.run` ile sayar; MCP'nin `readOnlyHint`'i kataloğun `effect`'inden gelir. Anlamsal komutu olmayan
    araçların sıralı listesi TODOS.md `UX-14`'tedir.
- **Birim, çizim ölçeği ve yazıların boyu (sahibin kararları, 24 Ekim 2026; [ADR 0626](../../docs/adr/0626-units-scale-and-true-size-texts.md)).**
  - Yeni CAD projesi yalnız “Ne çiziyorsunuz?” diye sorulur (`kentos_project::wizard::WORKS`):
    - Mimari proje: metre, 1:100, yerel;
    - Harita / imar planı: metre, 1:1000, ilin sisteminde;
    - Detay / imalat: milimetre, 1:1, yerel (ADR 0802 ile kalktı: iki CAD işi var, yerel CAD Mimari'nindir).

    Yazı boyları ve yazı tipi Proje ayarları'ndadır. Bu kural ADR 0624'ün mm / 1:1 varsayılanının ve ADR 0622 §2'nin
    sihirbaz yükseklikleri ile durum çubuğunun yerine geçer.
  - Yazı ve ölçü boylarını belirleyen ölçeğin tek adı “Çizim ölçeği”dir. Durum çubuğunda tek ölçek vardır:
    - CAD'de çizim ölçeği; birim imlecin koordinatının yanındadır;
    - CBS'de görünümün “1:N”'i.

    Proje ayarları'nda “Birim ve ölçek” tek bölümdür.
  - Açılan DWG ve DXF'in çizim ölçeği yazılarının ve ölçülerinin ortanca boyundan çıkar (2,5 kâğıt mm; `opened_scale`).
    Ölçek 1'den başkaysa dosya o ölçekle bir kez daha okunur; kullanıcıya sorulmaz.
  - `graphics.annotationSize`'ın varsayılanı “Türe göre”dir: CAD'de gerçek boy, CBS'de kaybolmasın
    (`LabelSize::for_project`).
    - Gerçek boy kâğıdın kuralıdır (`LabelSize::Print`) ve hiçbir yazıyı gizlemez.
    - 3 pikselden küçük yazı soluk şerit olarak çizilir. CAD'de hale yoktur.
    - Yerleşim görünümün çevresiyle (her yanda bir görünüm) istenir. Görünüm o kutudan çıkınca hareket sürerken de
      yeniden istenir.
    - Kaydırma sürerken yazı katmanı yeniden kurulmaz: son resim görünümün kaydırmasıyla konur (`labels/layer.rs`,
      görünümün dörtte biri payla). Durulunca, alan dışına çıkınca ve ölçek değişince (tekerlek) yeniden kurulur;
      harf ya da şerit kararı her zaman o anki ölçeğindir. Okunamayan yazının şeridi, renk ve kalınlık başına tek
      geçişte çizilen çizgidir.
- **Veritabanı bağlantıları (sahibin isteği, 23 Ekim 2026; [ADR 0623](../../docs/adr/0623-database-connections.md)).**
  - PostgreSQL bağlantısı QGIS'teki gibi bir pencereyle tanımlanır: ad, sunucu, bağlantı noktası, veritabanı,
    kullanıcı, isteğe bağlı kayıtlı parola, SSL, Bağlantıyı dene. Adres yazılmaz.
  - Bağlantılar cihazdadır (`veritabani-baglantilari.json`, 0600, `kentos_postgis::connections`). Parola sırası: oturum,
    cihaz, `PGPASSWORD`, `~/.pgpass`. Çizim parola taşımaz.
  - Kaynaklar › Veritabanları bağlantı › şema › tablo ağacıdır; tablolar katalogdan tahmini satırla okunur
    (`kentos_postgis::probe`), hatalar düz Türkçe söylenir.
  - QGIS'ten alma ve QGIS'e verme yoktur; ekip paylaşımı KentOS'un kendi bağlantı dosyasıyladır.
- Web bağımsız TypeScript/DOM uygulamasıdır; desktop'ın web export'u değildir.
  Yalnız hesaplama/codec kütüphaneleri dar WASM bağlayıcılarıyla kullanılır.
  Web'in belge, komut, etkileşim, settings ve render orkestrasyonu TS'te kalır.
- Desktop Rust ile; `kentos-rc` 25 Eylül'de bu depoya `crates/ui`
  (`kentos-ui` / `kentos_ui`) olarak geçmişiyle alındı (ADR 0016). İlk masaüstü
  kabuğu `apps/desktop`'tadır (ADR 0017); ana CAD çizim alanı `crates/render/wgpu`
  ile Iced'in aygıtında çizilir (ADR 0019); araçları kapalı alan, çizgi, çoklu çizgi
  (ADR 0021, 0027), nokta, daire, yay, dikdörtgen, döndürülmüş dikdörtgen ve düzgün
  çokgendir (ADR 0032); elips, eğri, yardımcı çizgi, ışın, paralel çizgi, dik in ve dik çık, halka,
  revizyon bulutu, kot noktası ve böl (ADR 0057); yazı ve çizimin üstündeki yazı kutusu (ADR 0060);
  ölçülendirme: hizalı, doğrusal, açı, yarıçap ve çap (ADR 0061), koordinat, yay uzunluğu, kırıklı yarıçap, semt ve eğim, Açı'nın yaydan ve daireden yolları, döndürülmüş doğrusal, ölçü değerinin zemini, Hızlı ölçü; DXF'in koordinat ölçüsü, ARC_DIMENSION'ı ve LARGE_RADIAL_DIMENSION'ı gelir ve gider, semt ve eğim hizalı ölçü olarak KentOS verisiyle (ADR 0147); tarama: kapalı nesneyle ya da çizgilerle, adalarıyla (ADR 0062); köşe kotu: çizginin, çoklu çizginin ve alanın köşe kotları, Kot ver, Öznitelikler'in kot ve 3B uzunluk satırları (ADR 0142); çok parçalı alan: parçalar ve delikleri, Parçaları birleştir, Parçalara ayır, alan işlemlerinde Tek nesne (ADR 0143); blok: Blok oluştur, Blok ekle, Bloklar paneli, Blok öznitelikleri penceresi, Öznitelikler'de yerleştirme ve blok öznitelikleri, Patlat; DXF'in blokları tanım ve yerleştirme olarak gelir ve gider, öznitelikleri ATTDEF ve ATTRIB olarak, “Blokları patlat” seçeneğiyle (ADR 0144); yazı ekleri: yazının ve öznitelik tanımının on iki noktalı hizası ve genişlik çarpanı, yazının zemini, Yazı'nın Hiza, Genişlik, Zemin ve Artır seçenekleri, Okunur yap, Bul ve değiştir, Metin dosyası yerleştir; DXF'in 72/73'ü, 41'i, MTEXT'in yerleşim noktası ve zemini, NCZ'nin çapaları tahminsiz (ADR 0145); kılavuz: ok, kırık çizgi, kol ve not tek nesne, Kılavuz aracı (Ok, Yükseklik, Zemin), Öznitelikler'in Kılavuz bölümü, notun yerinde düzenlenmesi; DXF'in LEADER'ı bağlı MTEXT notuyla ve MULTILEADER gelir, kılavuz LEADER ve MTEXT olarak gider (ADR 0146);
  çok satırlı yazı: kutu genişliği ve sözcük sarma, satır aralığı, kalın, eğik, altı çizili, üst ve alt simge, renkli harfler, Çok
  satırlı yazı aracı (iki köşe, Satır aralığı), düzenleyici (çizimde canlı önizleme, Renk ▾, Simge ▾), çift tıkla düzenleme,
  Öznitelikler'in Kutu genişliği ve Satır aralığı; DXF'in MTEXT'i biçimleriyle tek yazı olarak gelir ve gider (ADR 0182, `.kcad` şema 20);
  yazı ve ölçü stilleri: projenin adlı stilleri (`.kcad` şema 21), nesne görünüşünü kendi alanlarında taşır (yazının yüzü: stil, yazı tipi,
  kalın, eğik, yatıklık; ölçünün görünüşü: dolu ok, açık ok, nokta, yok ya da çentik, boylar, değerin yeri, basamak, birim, önek, sonek,
  yazı tipi), stil değişince eski değerdekiler tek adımda izler; CAD projesinde Yazı stilleri ve Ölçü stilleri penceresi, araçların Stil (S)
  seçeneği (Çok satırlı yazı'nın Satır aralığı R), Öznitelikler'in stil satırları; DXF STYLE ve DIMSTYLE iki yönde, içe aktarılan stiller
  projeye adıyla eklenir, Standard Standart'tır (ADR 0183);
  tablo: hücreler, birleşik alanlar, sütun hizası, başlık satırı, çizgiler ve kalın çerçeve (`.kcad` şema 22); CAD projesinde Tablo ekle
  (boş, Excel .xlsx, ODS ya da CSV/TXT dosyasından, koordinat, alan ve öznitelik çizelgesi; önizleme, imleçten yerleştirme), Tabloyu düzenle
  (elektronik tablo gibi; çift tıkla), Tabloyu güncelle (kaynağına bağlı: çizelgenin nesneleri ya da yeniden seçilen dosya), Öznitelikler'in
  satırları; DXF'e adsız blok olarak gider, KENTOS verisiyle tablo olarak gelir (ADR 0184);
  koordinat yazımı: CAD projesinde Koordinat yaz (tıklanan, yazılan ya da `#ad` noktaya; yerindeki noktanın adı ve kotu) ve Köşelere koordinat
  yaz (seçili nokta, çizgi ve alanların bütün köşelerine, tek adımda); eğik kol ve yatay çizgi, üstünde doğu, altında kuzey ya da kolsuz
  satırlar; Şablon (`{Y}`, `{X}`, `{Z}`, `{ad}`), Basamak, Yön (Otomatik dışa), Yükseklik, Stil; Çizelge açıkken aynı nesnelerin koordinat
  çizelgesi imleçten yerleşir (ADR 0185);
  tarama ekleri: desen türü (adı, ölçeği ve çizgi aileleri taramanın içinde; `.kcad` şema 23), kitaplıkta ANSI, ISO ve genel 28 desen,
  kesikler, noktalar ve kayma iki platformun çizim hatlarında, degrade (doğrusal, silindir, küre; ters, ikinci renk); Tarama'nın Desen
  (menüde her desenin geniş örneği, adı yazılarak), Ölçek, Açı, İkinci renk, Ters, İlişkili ve Yazılar seçenekleri; Çoklu tara; ilişkili tarama kapalı
  nesnesini, adalarını ve boş bıraktığı yazıları iki belgede kayıttan önce izler, Öznitelikler'de İlişkiyi kopar; DXF'in HATCH'i desen
  satırları ve degradesiyle iki yönde (ADR 0186);
  seçim ekleri: Sıradakini seç (tıklamanın adayları çekirdeğin sırasıyla, “1/3 ▾” çipi ve listesi, Shift+Boşluk), Çokgenle seç
  (İçindekiler, Kesişenler, Dışındakiler), Önceki seçim, Benzerini seç (Tür, Katman, Renk, Sembol), Seçim süzgeci (durum çubuğunda Süzgeç,
  on yedi tür; tıklama, kutu, seçim araçları ve seçme adımları süzülür, dışarıda kalan söylenir; hücrenin menüsü ve Giriş › Seçim süzgeci ▾
  aynı işaret listesi: kısa adlar ve ikonlar, menü açık kalır, Bütün türler ve Hiçbir tür; Kenet türleri ve Çakışma'nın katmanları da açık kalır) (ADR 0187);
  nokta hesaplayıcı ekleri: Obje üzerinde nokta (nesnenin yolunda yakın uçtan uzaklık ve dik sapma; çizgi, yay, daire, elips, eğri, alanın
  dış sınırı), Km ve sapma (güzergâhın km'si, Başlangıç (B)), Nokta adından, Mesafe ve eğim (eğik mesafe ve yüzde eğim), Açıortay;
  `#ad` hesaplayıcının referansını da verir, takma adlar Türkçe işaretsiz de (`eğim` EGIM) (ADR 0188);
  Km yaz: güzergâhın aralıkla km'si (0+020 …) ve uçları, istasyonlarda dik işaret, çizginin yanında okunur km yazısı, isteğe bağlı
  enkesit ve sapmalı nokta (`Km` özniteliğiyle), tek adımda; CAD projesinde Açıklama › Km (ADR 0189);
  orta hat: iki kenarın (yol kenarları, dere kıyıları) ekseni, eşleşen kenarlarda kenar kenar ve yaylarıyla kesin, öbürlerinde uzunluk
  payıyla Adım'da bir örneklenerek; Zincir uç uca çizgileri tek kenar sayar; tek adımda çoklu çizgi, Çizim ▾'da (ADR 0190);
  paralel kaydır: alanın ya da çoklu çizginin düz kenarı kendine paralel kayar, komşuları uzar ya da kısalır; yazılan uzaklıkla, imleçle
  ya da Alan (A) ile hedef alana (alan projenin biriminde, uzaklık ikinci dereceden denklemin sıfıra yakın kökü); köşeler kotlarını
  korur, tek adımda, Değiştir ▾'da Ötele'nin yanında (ADR 0191);
  resim nesnesi: PNG ya da JPEG, sol alt köşesi, genişliği, yüksekliği, dönüşü ve aynasıyla; gömülü (projenin kitaplığında, kimliği
  içeriğinden) ya da masaüstünde bağlı (dosya yolu); kırpma sınırı resmin kendi kesirleriyle, saydamlık; Resim ekle (ikinci nokta ya da
  yazılan genişlik, Bağlı) ve Resmi kırp (Dikdörtgen, Çokgen, Kaldır), CAD'de Ekle › Resim, CBS'de Veri › Resim; yalnız kenarından seçilir;
  Öznitelikler'de Kaynak ve Göm; resmin kendi dokusu mip katmanlarıyla üç çizim hattında (`.kcad` şema 24, ADR 0192);
  çizimler arası alışveriş: Seçilenleri dosyaya kaydet (seçilenler kullandıkları katmanlar, bloklar, kitaplık ve ayarlarla yeni `.kcad`),
  Başka çizimden al (katmanlar yollarıyla, bloklar, yazı ve ölçü stilleri, kitaplık, katman durumları, proje ayarları; Atla ya da
  Değiştir; katmanlar ve bloklar tek adımda, ötekiler ayar), Dosyadan blok ekle (çizim dosyanın adıyla tek blok, sol alt köşesinden,
  ardından Blok ekle) (ADR 0193);
  hizala ve dağıt: seçilenler kutularının kenarı ya da ortasıyla başvuru nesnesine, noktaya ya da seçimin kutusuna (Sola, Ortala,
  Sağa, Üste, Ortaya, Alta), ya da eşit aralıkla (Yatay ve Dikey dağıt); her nesne kendi kaymasıyla, tek adımda;
  `cad.entities.transform`'un `arrange` türü (ADR 0194);
  görünüm kipleri: Renk kipi (Renkli, Tek renk, Gri), Dolgular, Alan sınırları ve Saydamlık açılıp kapanır, seçimin ve üzerine
  gelmenin vurgu rengi ve kalınlığı; kullanıcının tercihleri, çizim değişmez (ADR 0195);
  eğri boyunca yazı: yazının eğrisi kendi çerçevesinde (`.kcad` şema 25), harfler eğri boyunca; Eğri boyunca yazı ▾ (Eğri boyunca
  yazı, Eğriye oturt, Doğrultuya döndür, Düzleştir), Okunur yap eğriyi çevirir; DXF'e harflerinin bloğu ve KENTOS verisiyle (ADR 0196);
  çizim ekleri: İki daireye teğet (tıklamalara en yakın dış ya da iç teğet), Dördüncü köşe (nokta ya da paralelkenar alanı), Menzil
  halkaları (Aralık, Sayı, Işın) (ADR 0197);
  plan yolu çizimi: yol alan, türü ve genişliği öznitelik; Plan yolu (yol, yaya ve bisiklet yolu; kaldırımla taşıt yolu, refüj, eksen),
  Kavşak temizle (türlere göre birleştirme, ada ve kaldırım köşelerinin yuvarlanması), Refüj kapat (ADR 0198);
  katman alanları, öznitelik tablosu ve veri kaynakları: katmanın alanları (metin, tam sayı, ondalık sayı, tarih, evet/hayır;
  uzunluk, basamak, aralık, değer listesi, zorunlu, varsayılan; tek biçim `kentos_contracts::fields`; `.kcad` şema 26), `cad.entities.set`
  ve `.create` alanların kuralıyla yazar, Alanlar penceresi (Verilerden al), alt panelin Tablo sekmesi (Katman, Ara, Göster, İfade
  süzgeci, türe göre sıralama, hücrede düzenleme, uymayan değer uyarı zeminiyle), Öznitelikler'de alanlar türlerine göre; Shapefile
  ve GeoJSON alanları taşır; sağ dokta Kaynaklar (klasörler ve dosyaları içe aktarma penceresine, KentOS projelerinin katmanları
  nesneleriyle tek adımda) (ADR 0199);
  mekânsal ve öznitelik sorgusu: İşlemler'de Konuma göre seç (Kesişen, İçeren, İçinde kalan, Ayrık, Uzaklıkta, Merkezi içinde),
  İçindekinden bilgi al (Sayı, Toplam, Ortalama, En az, En çok, İlk değer), Çevreleyenden bilgi al, Özet istatistik (gruplu tablo,
  Panoya kopyala, CSV) ve Anahtarla birleştir (katmandan ya da CSV, TXT, XLSX dosyasından); sayıların kuralı `kentos.statistics/1`;
  İşlemler'in dosya parametresi, tablo çıktısı, aracın reddi ve çalıştırıcının katman alanları denetimi (ADR 0200);
  geometri işlemleri: İşlemler'in Geometri kategorisinde Tampon (eksi uzaklıkla içe, tek yanlı, halkalar, Birleştir, Uzaklık alanı;
  yaylar kesin), Kırp, Gruplayarak birleştir (kesin toplamlar), Kesişim, Fark, Simetrik fark ve Birleşim (Önek, Alan oranıyla paylaştır),
  Geçerliliği denetle (tablo ve seçim) ve Onar (rapor), Sadeleştir (rapor), Koordinat sistemine dönüştür (kayıttaki sistemden projeninkine,
  datum seçimleriyle); sonuç yeni katmana, kayıplar söylenir; çekirdek `ops::geoprocess` (ADR 0201);
  topoloji kuralları: projenin toleransı, kuralları ve istisnaları (`.kcad` şema 27); on üç kural, katman içi dokuzu (Çakışmamalı,
  Boşluk olmamalı, İnce alan, Yinelenmemeli, Sarkan uç, Kısa kenar, Küçük açı, Geçerli olmalı, Ortak sınırda köşe eksik olmamalı) ve
  katmanlar arası dördü (… ile çakışmamalı, … içinde kalmalı, Sınırı … sınırlarında olmalı, … çizgilerinin ucunda olmalı); Topolojiyi
  denetle alt panelin Topoloji sekmesinde (Kural, Açık / İstisna / Hepsi; satır bulguya gider ve onu çizimde gösterir), Düzelt ▾ tek adımda
  (`topologyFix`) ve denetim yinelenir, İstisna yap projede; Topoloji kuralları penceresi; çekirdek `ops::topology_rules` (ADR 0202);
  ağ dengelemesi: Yatay ağ dengelemesi (doğrultu serileri ve yatay kenarlar, sabit ya da ağırlıklı bilinen noktalar, adı yazılan
  çizimden; yaklaşık koordinatlar çizimden ve turlarla; en küçük kareler, hata elipsleri, artıklar, katkı, uyuşumsuz ölçü testi, m₀ ve model
  testi) ve Kot ağı dengelemesi (geometrik nivelman ya da trigonometrik); önsel doğruluklar Proje ayarları › Ölçme'de (`.kcad` şema 28);
  Çizime yaz aynı adlı noktaları taşır ya da kotlarını yazar, yenileri ekler, tek adımda; Karne editöründen aktarma; çekirdek
  `survey::adjust` (ADR 0203);
  raster katmanları: GeoTIFF ve TIFF (klasik ve BigTIFF; LZW, Deflate, PackBits, JPEG; 8–64 bit, palet, iç önizlemeler), dünya dosyalı PNG
  ve JPEG, kendi okuyucumuzla dosyanın gereken parçalarından; raster nesnesi (`.kcad` şema 29: dönüşüm, boy, bantlar, bağlı ya da gömülü
  kaynak, sistem, görünüş, saydamlık); 256'lık karolar, iç önizlemeler ya da bir kez hazırlanan önizleme piramidi (cihazın önbelleği,
  Durdur'lu panel); görünüş: renkli, gri, paletli, renk rampası, gölgeli kabartma (Horn), rampa ve gölge, gerdirme, nodata, örnekleme;
  çizim hattında raster atlası (stilli çizimin 6. sürümü); Raster ekle (sistemin kuralı, oturtulmamış yer), Raster stili (canlı önizleme),
  Raster oturt (Helmert, afin, projektif, polinom 2 ve 3, ince plaka; artıklar ve m0; afin dönüşümü değiştirir, ötekiler GeoTIFF'e yeniden
  örnekler), Koordinat oku'da rasterin değerleri; CBS'de Veri › Raster, CAD'de Ekle › Raster (ADR 0204);
  nokta bulutu, şimdilik yalnız masaüstünde (sahibin kararı, 8 Ekim; web'in şeridinde aynı düğmeler “Yalnız masaüstünde; web için
  hazırlanıyor” notuyla bekler): LAS 1.0–1.4, LAZ (laz-rs), COPC ve metin bulutları (XYZ, PTS, TXT, CSV); bağlı dosya, HTTP adresi (parça
  parça) ya da gömülü (32 MB'a kadar), birden çok dosya tek sanal bulut, .vpc okunur ve yazılır; `pointcloud` nesnesi (`.kcad` şema 31;
  rasterin adresi `url` de); COPC olmayan dosyanın dizini cihazın önbelleğinde bir kez (Durdur'lu panel); çizimde kat kat, karede en çok
  4 milyon nokta, düğümler iş parçacıklarında yalnız görünüşün alanlarıyla çözülür; görünüş: renkler, sınıflar, yükseklik ve yoğunluk
  rampası, dönüşler, tek renk, gizlenen sınıflar, piksel ya da metre boy, kare ya da yuvarlak, saydamlık; Nokta bulutu ekle, Nokta bulutu
  stili (canlı önizleme), XYZ sor, Sanal bulut olarak kaydet; İşlemler'in Nokta bulutu kategorisi: alan sorgusu, Seyrelt, Zemin süzgeci
  (SMRF), Yüksekliğe göre sınıfla, Bulutu kırp, Bulutları birleştir, Karola, Rasterleştir (GeoTIFF), Sınır çıkar; sonuçlar LAS, LAZ ya da
  COPC; Raster ekle'de Adres (GeoTIFF ve COG, HTTP aralıklarıyla) (ADR 0207);
  harita servisleri ve altlıklar: servis katmanı (XYZ ve TMS, WMS 1.1.1 ve 1.3.0, WMTS, OGC API Tiles, ArcGIS REST, Google Map Tiles API,
  vektör karolar MapLibre stiliyle), veri katmanı (WFS, OGC API Features, ArcGIS katmanı, GeoJSON adresi; Yenile) ve projenin bağlantıları
  (adreste parametre, başlık, kullanıcı adı ve parola, belirteç, ArcGIS belirteci, OAuth 2, Google anahtarı; gizli değerler cihazda), `.kcad`
  şema 32; karolar projenin sistemine ağla, vektör karonun noktaları da aynı ağla; Altlık ▾ (hazır altlıklar), Harita servisi, Servisten veri al,
  Servis bilgisi, Bağlantılar (CBS'de Harita, CAD'de Ekle); katmanın menüsü, rozeti ve Öznitelikler'deki bölümü; atıf şeridi ve kartı;
  `cad.layers.service`; web vekili `kentosd`'nin `/v1/proxy`'si; Python `kentos.services` (ADR 0208);
  ağ analizi: projenin ağları (kenar ve düğüm katmanları süzgeçleriyle, düğümün rolü Bağlantı, Kaynak ya da Vana ve kapalılığı, Uçlarda ya
  da Köşelerde bağlanma ve tolerans, yön, Uzunluk ve hızdan ya da alandan maliyetler, kapalı kenarlar; `.kcad` şema 33), ağ her analizde
  katmanlardan kurulan graf; Ağlar penceresi ve Denetle; En kısa yol (Sırayı iyileştir), Hizmet alanı (aralıkların çizgileri ve alanları,
  Birleşik ya da Ayrı, Disk ya da Halka), Şebeke izleme (Bağlı, Akış yukarı, Akış aşağı, Yalıtım ve beslemesiz kalanlar); İşlemler'in Ağ
  analizi kategorisi (En yakın tesis, Maliyet matrisi, Hizmet alanları); kurma ve aramalar web'de ağın işçisinde, masaüstünde ağın iş
  parçacığında; `cad.network.define`; Python `kentos.network`; CBS'de Analiz › Ağ analizi (ADR 0209);
  zaman ve senaryolar: katmanın zaman ayarı (başlangıç, bitiş, kimlik alanı, Birikimli; nesnenin zamanı özniteliklerinden), senaryo
  grubu ve ana katmanın yerine geçen katman (`.kcad` şema 34); geometri deposunda zamanlar ve pencere (tıklama, kenet, etiket ve Genel
  bakış süzülür); çizimin altında Zaman sürgüsü (Başa … Sona, adım ve birim, Anlık ve Aralık, Hız, Döngü; oynatma çizimi bekler), Zaman
  ayarları, Yeni sürüm oluştur ve Sona erdir, Veri karşılaştır'da tarafın tarihi, Zamanı karşılaştır, Geçmiş'te revizyonla Karşılaştır,
  Senaryo oluştur, Senaryoyu göster, Mevcut durum, Senaryoyu karşılaştır, Senaryoyu uygula; `cad.layers.time`, `cad.scenarios.edit`;
  Python `kentos.temporal`; CBS'de Harita › Zaman ve Senaryo (ADR 0210);
  katman süzgeci: katmanın koşulu (İfadeyle seç'in dili) ve ya da nesne listesi (`.kcad` şema 35); süzgeçten geçmeyen nesneler çizilmez,
  seçilmez, kenetlenmez, Öznitelik tablosunda, İşlemler'in girdilerinde ve Veride ara'da yoktur, ağaçta huni ve “geçen / bütün”; dosya
  alışverişi ve komutlar bütün nesnelerle çalışır; Katman süzgeci penceresi (ε, Seçimden al, önizleme), Seçimden süzgeç, Süzgeci kaldır,
  süzgeçten geçmeyen yeni nesne söylenir; `cad.layers.filter`; CBS'de Veri › Süzgeç, iki platformda Katmanlar'ın sağ tıkı (ADR 0211);
  etiket motoru: çizimin, paftanın ve Etiketleri yazıya çevir'in tek motoru (QGIS'in PAL'ı ve Maplex gibi: türe göre maliyetli adaylar,
  yönlü kutularla çakışma, engeller ve öncelik, açgözlü yerleşim ve bir adımlık iyileştirme, adaylar gerektikçe üretilir); katmanın
  etiketlemesi Tek etiket, Kurallı (sıralı sınıflar: ad, koşul, kendi stili) ya da Yok, engel ağırlığı ve türü (`.kcad` şema 36);
  metin şablonla ya da ifadeyle; noktada çevresinde ya da üstünde, çizgide paralel, kıvrık, yatay ya da yokuş yukarı eş yükselti,
  alanda yatay, eğik, çevre, sınır, parsel ya da köşe; yineleme, uzaklık, yalnız içine, sığmazsa dışarıda çağrı çizgisiyle; hale, zemin,
  gölge; yığma, kısaltma sözlüğü, küçültme; öncelik, çakışma, yinelenenler; nesnenin etiket iğneleri (`labelPins`: elle taşınan,
  döndürülen, gizlenen etiket); Etiketler penceresi (beş sekme, sınıflar, Engel, ε) ve Katmanlar'ın sağ tıkı, Etiketi taşı, döndür,
  sabitle (Çöz) ve gizle (Göster), Sabit etiketleri vurgula, Yerleşmeyen etiketleri göster; CBS'de Harita › Etiket;
  `cad.layers.labels` ve `cad.labels.pin`; paftanın PDF'ine zeminler ve çağrı çizgileri vektör olarak (ADR 0212);
  ek işleyiciler: Sürekli renk (rampadan ana renk), Orantılı sembol (Alan, Yarıçap, Flannery), İki değişkenli renk (n × n ızgara), Nokta
  yoğunluğu (alanın içinde tohumlu noktalar, katmanda en çok 1 000 000), Grafik (pasta, çubuk, yığılmış çubuk), Isı haritası (QGIS'in
  dördüncü dereceden çekirdeği, dinamik ya da sabit en büyük, görünümün resmi), Kümeleme ve Yayma (QGIS'in nokta uzaklığı kuralı; halka,
  iç içe halkalar, ızgara; tek noktalar bir iç işleyiciyle), Ters alan (alanların dışı; çift-tek ya da sarım); görünüme bağlı olanlar
  yakınlaştırma durunca ya da kutudan çıkınca yeniden kurulur; Katman stili'nde gruplu, ikonlu tür listesi ve formlar, lejant satırları,
  pafta; `cad.layers.renderer` (ADR 0213);
  ifade dili ekleri: tarih ve saat (yap, oku, parçalar, ekle, fark, biçimle, şimdi), `@` değerleri (projenin değişkenleri ve yerleşikler:
  proje adı, sistem, EPSG, ölçek, tarih, şimdi, kullanıcı, katman; `.kcad` şema 37), düzenli ifade (`regex-lite`), dizi ve eşleme (JSON),
  aynı ve başka katman üstünden toplama, Konuma göre seç'in ilişkileri, en yakının değeri, örtüşen alan ve uzunluk, başka katmandan değer;
  başka katmana bakanlar yalnız İşlemler'de (`compile_with` başka yerde reddeder), katman süzgecinin dışarıda bıraktıklarını okumaz;
  `@` değerleri İşlemler'de, İfade oluşturucu'da, Öznitelik tablosunun süzgecinde ve paftada; Proje ayarları › Değişkenler (ADR 0214);
  yakınlık analizi: İşlemler'in Yakınlık kategorisinde En yakını bul (uzaklık, hedefin alanları ve semt önekle; kenardan kenara ya da
  merkezden merkeze, en çok uzaklıkla), Uzaklık matrisi (en yakın k ya da hepsi; Liste, Matris, Özet), En yakın merkeze bağla ve En kısa
  çizgi (yeni katmanda çizgiler), Komşu alanlar (aynı doğrudaki düz ve aynı dairedeki yaylı ortak kenarlar toleransla, köşe ve örtüşme;
  tablo, komşu sayısı ve adları); hedefler kendi R-ağaçlarında en iyi önce aranır (`PackedTree::nearest`); CBS'de Analiz sekmesinin Analiz panelinde (ADR 0215);
  veri aktarma: İşlemler'in Öznitelik kategorisinde Veri aktar (hedef katmana, boş da olabilir, başka katmandan ya da CSV, TXT, XLSX
  dosyasından Anahtarla birleştir'in anahtar kuralıyla; Alan eşleme parametresi: hedef alan, kaynak sütun ya da yok, varsayılan, Aynı
  adlarla doldur; Üzerine yaz ya da Yalnız boşlara, tipli alanlar kurallarıyla, Geometriyi güncelle, Eşleşmeyenleri ekle, Yalnız raporla;
  rapor tablosu, tek adım); İşlemler'e `allowEmpty`, girdinin katmanı ve şeklin yaması; CBS'de Veri › Öznitelik (ADR 0216);
  katman bölme ve birleştirme: İşlemler'in Katman kategorisinde Özniteliğe göre böl (bir ya da birkaç alanın değerleri, yeni grupta değer
  başına katman, ilk nesnenin katmanının görünüşü ve alanlarıyla; Kopyala ya da Taşı, en çok 1000), Vektör katmanlarını birleştir (alan
  eşlemesiyle, Kaynak katman alanı, tipli hedefin kuralları), Seçilenleri çıkar, Yinelenenleri sil (alanlar ya da bütün öznitelikler,
  isteğe bağlı kesin geometri; ilki kalır), İfadeye göre sırala (`ops::order`: türler, doğal sıra, boşlar, Sıra alanı); İşlemler'e Katmanlar
  parametresi, aracın açtığı katmanlar ve gruplar, `like`; CBS'de Veri › Katman (ADR 0217);
  geometri dönüştürme: İşlemler'in Dönüştürme kategorisinde Nesneden nokta (ağırlık merkezi ya da iç nokta: GEOS'un tarama doğrusu
  yaylarıyla; her parça için), Köşeleri noktaya çevir (bütün köşeler, uçlar, orta; Köşe, Parça, Halka, Uzaklık), Noktalardan çizgi (ifade
  sırası, grup, kapalı), Semt ve uzunluktan çizgi, XY'den çizgi ve Tablodan elips (dosyadan ya da katmandan; satırlar çekirdekte, atlananlar
  raporda), Çizginin parçası (ifadeli uzaklıklar, yaylar kesin), Alanları çizgiye çevir, Çizgileri alana çevir (Atla, Kapat), Köşe ekle
  (aralıkla ya da sayıyla, yaylar yay); kotlar taşınır; CBS'de Veri › Toplu çevir (ADR 0218);
  geometri üretimi: İşlemler'in Geometri üretimi kategorisinde Voronoi alanları (çerçeve Genişletme yüzdesiyle; hücreler kesin Delaunay
  üçgenlerinin çevrel merkezlerinden, komşular köşelerini bit bit paylaşır), Delaunay üçgenleri (köşelerin adları ve kotları), Sınırlayıcı
  geometri (dışbükey zarf, sınır kutusu, en küçük alanlı döndürülmüş kutu, en küçük daire; her nesne, hepsi ya da alana göre; ölçüleri),
  İçbükey zarf (JTS'in ve GEOS'un kenar oranıyla aşındırması, delikler), Ulaşılmazlık kutbu (polylabel); CBS'de Analiz › Geometri ▾ (ADR 0219);
  örnekleme ve ağ üretimi: İşlemler'in Örnekleme kategorisinde Ağ oluştur (dikdörtgen, altıgen, üçgen ya da ağın çizgileri; aralığın
  katlarındaki genel kafes, komşu hücreler köşelerini bit bit paylaşır; Nesnelere değenler Konuma göre seç'in Kesişen'iyle; Satır ve Sütun),
  Düzenli noktalar (katlarda ya da hücre ortalarında, tohumlu kaydırma), Rastgele noktalar (alanda ve çizgide, ifadeli sayı, en az uzaklık;
  SplitMix64 ve xoshiro256** ile her platformda aynı noktalar), Hat boyunca nokta (aralıkla, paylarla ya da eşit parçalara; Uzaklık, Semt,
  kot) ve Alt bölümlere ayır (PostGIS'in ST_Subdivide'ı yaylı geometriye; alan doğru boyunca kesilir); CBS'de Analiz › Geometri ▾ (ADR 0220);
  kartografik genelleştirme: İşlemler'in Genelleştirme kategorisinde Yumuşat (Chaikin; En büyük açı, kotlar oranla, Ortak kenarları koru),
  Dik açılandır (iD ve QGIS'in kuralı), Bina sadeleştir (Ruas ve CartAGen: dik, paralel, ters paralel), Alanları kümele (kümenin kapanışının
  köprüleri, Girintileri doldur, Grupla, Nesne sayısı), Küçük parçaları ayıkla, Küçükleri komşuya kat (en uzun ortak sınır ya da en büyük
  komşu) ve Ortak kenarları koruyarak sadeleştir (kapsama grafı, GEOS'un CoverageSimplifier'ı); Geometri ▾ kategorilerle başlıklı; kesişim
  çekirdeği TOL ölçeğindeki karmaşada yüz yitirmez: TOL içindeki çemberler bir çember, yakın kesişim üçüncü kenarı da keser, köşelerin
  dengesi onarılır (ADR 0221);
  KML/KMZ, GPX ve içe aktarmada açık dönüşüm: KML/KMZ içe aktar (klasörler katman, yer imleri nesne; biçemin rengi ve kalınlığı, ad,
  açıklama, zaman, veriler; gx izleri; resim kaplamaları, modeller ve bağlantılar söylenir) ve dışa aktar (katmanlar grup yollarıyla
  klasör, paylaşılan biçemler, veriler; KMZ), GPX dışa aktar (yol noktaları, iz ya da rota, alanların sınırı kapalı iz); GeoJSON,
  Shapefile ve KML içe aktarmada başka sistemdeki dosya “Projenin sistemine çevir” onayıyla, dönüşümün yolu ve doğruluğu söylenerek
  çevrilir; GeoJSON dışa aktar'da “WGS 84'e çevir (RFC 7946)”; GNSS içe aktar'da rotalar, izler ve NMEA kaydı çizgi; Kaynaklar KML'yi
  tanır; `FORMATS_VERSION` 49 (ADR 0222);
  tablo alışverişi: XLSX ve ODS okuma (sayfalar akışla, `xml_scan`; tarih ve saat biçimli hücre ISO metniyle, 1900 ve 1904; ODS'nin
  tekrarları ve paragrafları LibreOffice Calc'ın okuduğu gibi) ve yazma (XLSX, ODS, CSV; sütunun türüyle sayı, tarih, evet/hayır, metin;
  kalın, donmuş ve süzgeçli başlık, Excel'in sayfa adı kuralı; başka yazılanlar raporda); İşlemler'in dosya parametresi 250 000 satıra
  kadar, Sayfa listesiyle, sınırı aşan söylenerek; Öznitelik tablosunda Dışa aktar ▾, İşlemler'in tablolarında Kaydet ▾; Tablodan nokta
  (dosya ya da katman, düzlem ya da kayıttaki bir sistemden açık dönüşümle, coğrafide DMS); Koordinat listesi çalışma kitabını Tablodan
  nokta'ya yönlendirir; `FORMATS_VERSION` 50 (ADR 0223);
  doğrusal referanslama: köşede ölçü (M; çizginin `ma`, `mb`'si, çoklu çizginin ve parçalarının `ms`'i; alanın yok; `.kcad` şema 39,
  `FORMATS_VERSION` 51), düzenlemeler M'yi kotun kurallarıyla taşır (`measure_lost`, `invalid_measures`), dönüşümler korur; Shapefile
  PolyLineM ve PolyLineZ'nin M'si okunur; İşlemler'in Doğrusal referans kategorisinde Rota oluştur (Uzunluk, Tek alan, İki alan;
  Koordinat önceliği, boşluklar), Rotayı kalibre et, Olayları rotada göster (nokta ve çizgi, öteleme, dosya ya da nesneler), Rota boyunca
  konumla (nokta, çizgi, alan; rotaların R-ağacı), Olayları çakıştır (Kesişim, Birleşim) ve M değerlerini çıkar; çekirdek `ops::lrs`;
  Öznitelikler'de, Koordinat listesinde ve kartta M; Km yaz'ın Ölçü (M)'si ölçülü rotada km'yi M'den yazar; CBS'de Analiz › Ağ analizi ▾
  (ADR 0224);
  nesne ekleri: nesnenin fotoğrafları ve belgeleri (`attachments`: gömülü fotoğraf, bağlı dosya ya da adres, not; `.kcad` şema 40,
  `FORMATS_VERSION` 52; `cad.entities.set`'in `attachments`'ı), Öznitelikler'de Ekler ve Eylemler, fotoğraf görüntüleyici, üzerine gelme
  kartında ilk fotoğraf; Konumlu fotoğraflar (kendi EXIF okuyucumuz, WGS 84'ten Koordinat dönüştür'ün yoluyla, nokta ve eki); katmanın
  eylemleri ve bilgi kartı (şablon `{ifade}`, `{$enlem}`, `{$boylam}`, adreste yüzde kodlama; `cad.layers.info`), Bilgi aracı ve Bilgi
  kartı ve eylemler penceresi, çizimin sağ tıkında Eylemler ▸; masaüstünde fotoğraflar kabuğun wgpu boru hattında (ADR 0225);
  sembol katmanı ekleri: Geometri üretici (15 adım: merkezler, köşeler, sınır, paralel, uzat, tampon, sınırlayıcı kutu, dışbükey örtü,
  daire, daire dilimi, düzgün çokgen, öteleme, döndür, ölçekle; değerleri veriyle), sembol katmanlarının çizim efektleri (gölge, dış parıltı,
  bulanıklaştır, iç gölge, iç parıltı), katmanın Katman çizimi (saydamlık, 13 karışım kipi, etiketlerin ve işaretlerin maskesi; `.kcad` şema
  41, `cad.layers.compose`); çizim alanında üç çizim hattında aynı planla (paketli haritalar, bir grubun efektleri tek geçişte); Sembol
  tasarımcısında adımlar (Adım ekle ikonlu) ve Efektler, Katman stili'nde Katman çizimi (ADR 0226);
  grafikler ve alan istatistikleri: alt panelin İstatistik sekmesi (katmanın bütün alanlarının özeti, seçilen alanın istatistikleri ve en çok
  500 değeri, değere tık seçer; özetler karede 8 ms) ve Grafik sekmesi (Çubuk, Pasta, Histogram, Kutu, Saçılım, Çizgi; katmandan ya da
  İşlemler'in sonuç tablosundan, ROC, Kesit, Zaman serisi ve Histogram kendi grafikleriyle; basmak seçer, Shift ekler, Ctrl çıkarır,
  Saçılım'da kutu, çizimin seçimi vurgulanır; ipucu, Grafik | Tablo, Panoya kopyala, PNG ve SVG); `data.statistics`, `data.chart`,
  `chart.*`; Katmanlar'ın sağ tıkında; hesap ve yerleşim çekirdekte (`ops::charts`, `kentos.charts/1`), web'e `ChartWork` (ADR 0227);
  adres arama, yalnız masaüstünde (web'e ara, ADR 0228): projenin adres verisi (kapı noktaları, yol orta hatları, mahalle, ilçe ve il
  alanları katmanlarıyla ve alanlarıyla; en düşük puan, Konumdan adres uzaklığı; `.kcad` şema 42, `cad.addresses.define`); adresin
  parçaları (Mah., Cad., Sk., Blv., No:, 12/A, posta kodu), yazım hatasına dayanıklı benzerlik, kapı, kapılar arası aradeğer, yol ve
  mahalle adayları puanlarıyla, Eşleşti, Belirsiz, Eşleşmedi, adres kodu, konumdan adres (`ops::geocode`); çevrimiçi sağlayıcılar
  (Nominatim, Photon, ArcGIS World, Google, kurum sunucuları) kartlarıyla: koşullar onaylanmadan kullanılmaz, hız, toplu iş ve saklama
  kuralları kodda, önbellek ve anahtarlar cihazda (`kentos_services::geocode`); alt panelin Adres sekmesi (Kaynak, Ara, Konumdan adres;
  Git ve işaret, Nokta ekle, Taşı), Konumdan adres aracı, Adres verisi ve Adres sağlayıcıları pencereleri, İşlemler'in Adres kategorisi
  (Adresleri eşle, Konumlara adres yaz); şeritte CBS › Harita ve CAD › Ekle'de Adres; web dururken yeni komutların arayüz kaydı
  `catalog/v1/additions.json`; Python `kentos.address` (ADR 0229); adres verisini denetle: salt okunur `cad.addresses.check` (yinelenen kapı,
  adres kodu ve yolun farklı yazılışları kalıcı kimliklerle; ADR 0579);
  CAD işlem manifesti: arayüzün her komutunun, aracının ve yönteminin ürün işlemi düzeyi (semantic, geometry, processing, none,
  query, session, host) ve Python ile AI karşılığı `catalog/v1/operations.json`'da, üretilen `docs/cad/manifest.{json,md}` CAD
  tarafının boşluklarıyla; yeni komut eşlemesiyle gelir (ADR 0244; plan/cad.md, TODOS.md §16.5);
  ilişkili ölçü: nesnenin yerleri (uç, köşe, kenar ortası, merkez, düğüm) ve başvuru (`uid`, yer, kopma nedeni; `.kcad` şema 43,
  `FORMATS_VERSION` 55), çekirdeğin `ops::subref`'i (yapı aynıysa yer korunur, değiştiyse köşe bit bit aranır); Hizalı, Doğrusal,
  Koordinat, Semt ve Eğim'in kenetle bir yere oturan noktası bağlanır, belge kayıttan önce aynı adımda izler (`dimension_ties.rs`),
  kopan bağ nedeniyle kalır ve çizimde uyarı üçgeniyle görünür; Öznitelikler'de İlişki ve Serbest bırak, Açıklama › Ölçü'de Yeniden
  bağla, `cad.dimensions.reassociate`; Python ve MCP (ADR 0249);
  anlamsal Buda, Uzat, Köşe yuvarla ve Pah: `cad.entities.trim` ve `.extend` (seçimler ve açık sınırlar: bütün görünenler,
  dikdörtgen ya da verilenler; Çit'in sırası), `.fillet` ve `.chamfer` (yolun köşesi, iki çizgi ya da bütün köşeler; Kırp); komut
  bağlamı ev sahibinin geometri deposunu taşır (`ExecutionContext::with_store`), başsız sunucu kendisininkini kurar; araçlar
  önizlemeyi planla, yazmayı komutla yapar; `cad.entities.create` ilişkili ölçünün bağlı noktalarını yerlerinden yazar (ADR 0250);
  pafta kitabı projenin dosyasında: `.kcad` şema 44'ün `sheets`'i (pafta çekirdeğinin kitabı opak, resimlerin baytları SHA-256'larıyla;
  `FORMATS_VERSION` 56), belgede düzenleme ama geri alma adımı değil (`set_sheets`), pafta çekirdeğinin `project`'i; masaüstünde kitap
  çizimle açılır ve paftadaki her değişiklik çizime yazılır (`sheet_book.rs`), cihazda eski yerinde kalan kitap projeye alınır ve
  kaydedilince `pafta/kitaplar/tasindi/`'ye ayrılır, okunamayan kitap olduğu gibi kalır; veritabanı projesinde `project.sheets` (göç
  0017), `ProjectPatch.sheets`, kopya ve geri yükleme; bulut eşitlemesi onu meta olarak taşır (ADR 0251);
  ilk uçtan uca kabul çizimi: oda, düzeltme, ilişkili ölçü ve tarama, duvarlar, 1:50 ve 1:100 A3 paftalar, DXF ve PDF, başka ortamda
  açma tek senaryoda (`fixtures/acceptance/v1/room.json`, beklentiler `acceptance_room.py`'den) arayüzde, ajan bağlantısında, Python'da
  ve MCP'de aynı sonuçla, dosyalar bağımsız okuyucularla; `cad.sheets.edit` (pafta çekirdeğinin işlemleri, kâğıdın geçmişi `inverse`'le),
  `kentos-plot` (haritaların vektörleri, paftanın girdileri, yazı tipleri, başsız kitap PDF'i; haritaların yazıları henüz yalnız
  masaüstünde), başsız DXF (`kentos_headless::export`); Python `export_dxf`, `export_pdf`, `sheets`, MCP `drawing.exportDxf`,
  `drawing.exportPdf` (ADR 0252);
  komut protokolü: her yürütme belgenin tek adımı (`run_step`), nesne, blok, katman ağacı, ayar, stil ve pafta durum damgaları
  (kanonik JSON'un SHA-256'sı; sayılar en kısa, eşitlikte çift), çağrı zarfı ve cevabı (`requestId`, `expected`, `planId`), planın
  özeti ve parmak izi, idempotency defteri (`.kcad` şema 45, `FORMATS_VERSION` 57), kayıt `kentos.calls/1` ve tekrar oynatma (dosya
  komutlarının baytları karşılaştırılır), kataloğun `capabilities`'i ve kodların tek tablosu (`codes.rs`, `host` türü dahil),
  `CommandError`'ın `uids` ve `retryable`'ı, ortak değişiklik kümesi `cad.changeset.apply`, uzun işler (`start`, `job`, `cancel`;
  hesap, `staged`, kesinleşme), `file.export.dxf` ve `.pdf` atomik yazmayla; başsız sunucu, Python, MCP ve masaüstünün ajan bağlantısı
  aynı yoldan (ADR 0253);
  anlamsal Esnet, Kır, Birleştir, Böl, Alan oluştur ve Tarama: `cad.entities.stretch` (pencere, fark, seçim, Topoloji'nin komşuları),
  `.break`, `.join` (Zincir), `.divide`, `.boundary`, `.hatch` (nokta ya da Çoklu tara, desen adıyla); bölgeler ve komşular uygulama
  katmanında (`region.rs`, `RegionCache`, `topology_follow.rs`), alanın halkaları kanonik; araçlar önizlemeyi planla, yazmayı komutla
  yapar (ADR 0254); Ölçülendirme'nin on türü `cad.dimensions.create` ile: ölçülen nokta koordinat ya da yer, kenar nesne ve tıklanan
  yerle, yer `at` ya da yazılan uzaklık, stil adıyla; aracın önizlemesi `measured_in` (ADR 0255);
  birim değiştirme ve birimler: `cad.units.change` boyları (yalnız birim) ya da sayıları (çizim birimlerin oranıyla) korur, birim ve
  çizim tek adım (`Op::Units`), Proje ayarları sorar; ölçekleme kuralı sözleşmenin `units`'i (DXF'in birim çevirisi de: kutu, çerçeve,
  eğri artık ölçeklenir); komut şemalarındaki her sayının birimi `x-unit` (kataloğun `units`'i, birimsiz sayıyı test düşürür); ölçünün
  `basis`'i (sistem, `frame`, `method`: düzlem, elipsoit, zemin) ve toplu ölçü; özetin `conventions`'ı (eksen adları, kutupsal açı)
  (ADR 0256);
  DXF'te dosyanın birimi ve içe almanın komutu: birim tek kuralla (seçilen `fileUnit`, yerel projede bildirilen, sistemli projede
  metre, bildirmeyen dosyada projenin birimi; belirsiz dosya sessizce ölçeklenmez), `UnitDecision` ve uyarıları, `BLOCK_RECORD`'un
  70'i; DXF içe aktarma penceresinde “Dosyanın birimi” (kararın sözü, seçim dosyayı yeniden okur); `file.import.dxf` (pencere
  okuduğu dosyayla komutla yazar, büyük dosya kare kare komutun planı ve kaydıyla; Python `Document.import_dxf`, MCP); içe almanın
  kuralları uygulama katmanında (`import_apply`) (ADR 0257);
  kapalı ve genişlikli çoklu çizgi: çoklu çizginin `closed`'ı (kapalı çizgi, alan değil; gösterilen alanı `enclosed_area`) ve `widths`'i
  (köşe başına başlangıç ve bitiş genişliği, çizim uzunluğunda, kâğıt kalınlığından ayrı; `.kcad` şema 46, `FORMATS_VERSION` 59);
  çekirdekte yolun kapalılığı `is_closed_path`, şerit `geom::band` (`MIXED` kaydı, nesnenin renginde dolu), genişliğin taşınması
  `ops::widths` (`widths_lost`); `cad.polyline.create`'in `closed` ve `widths`'i, `invalid_polyline`; Çoklu çizgi'de Kapat (K) ve
  Genişlik (E), Öznitelikler'de Kapalı ve Genişlik; DXF'in 43, 40 ve 41'i ve `closedPolylines` (CAD projesinde kapalı çizgi) (ADR 0258);
  NURBS eğrisi: `nurbs` türü (derece, denetim noktaları, düğümler, ağırlıklar; kapalı, periyodik, geçiş noktaları, teğetler; `.kcad` şema 47,
  `FORMATS_VERSION` 60), çekirdek `geom::nurbs` ve `nurbs_build` (Piegl ve Tiller; geçiş noktalarından global interpolasyon), budama ve
  kırma `ops::nurbs_cuts`; `cad.nurbs.create`, NURBS eğrisi aracı (Yöntem, Derece, Kapat); DXF'in SPLINE'ı tam NURBS; daire, yay ve elips
  analitik, Catmull–Rom eğrisi ayrı tür; uzunluklar OpenCASCADE'le (`curves_occt.py`) (ADR 0259);
  nesnenin görünüşü: kendi çizgi tipi, ölçeği ve saydamlığı (`lineType`, `lineTypeScale`, `transparency`), bloğun nesnesinde katmana göre
  parçalar (`byLayer`), katmanın içinde çizim sırası (`order`; Öne getir, Arkaya gönder: `cad.entities.order`; stil motorunda sıra
  bantları), katmanın `plot`'u (Yazdır / Yazdırma, pafta atlar); `.kcad` şema 48, `FORMATS_VERSION` 61, sunucuda göç 0018
  (`feature.look`); `cad.entities.set`'in üç işlemi, oluşturan komutların `lineType`'ı (şeridin Çizgi tipi yeni nesnelere), DXF'in 6, 48,
  440 ve 290'ı (ADR 0260);
  kullanıcı koordinat sistemi (2B UCS): projenin `ucs`'i ve adlı UCS'leri (`namedUcs`; `.kcad` şema 49), `cad.ucs.set` (Dünya, Çerçeve,
  Orijin, Döndür, İki nokta, Nesneye hizala, Adlı, Kaydet, Sil; ayar, geri alma adımı değil, nesneler değişmez); çekirdek `tools::ucs`
  (`Frame`, `point_from_text_framed`, `constrain_cursor_in`, `object_frame`); yazılan nokta, Orto, kutupsal izleme, açı kilidi, ızgara,
  Karelaj, Doğrusal ölçü ve yeni taramanın açısı UCS'in eksenleriyle; durum çubuğu ve Koordinat oku UCS'te, eksen simgesi döner;
  Görünüm › Koordinatlar ve Önceki UCS (oturumun geçmişi); tel dünyadır: Python `Document.ucs`, `to_world`, `to_ucs`, MCP
  `drawing.toWorld`, `drawing.toUcs`, özetin `conventions.ucs`'i (ADR 0261);
  ayak ve inç: yerel projenin `in` ve `ft` birimleri (tam oran 254/10000 ve 3048/10000) ve `lengthStyle` (mimari `12'-6 1/2"`,
  mühendislik `12'-6.50"`, kesirli `150 1/2"`; `.kcad` şema 50), yazılan uzunlukta ayak ve inç işaretleri her projede (`5'6-1/2"`,
  çekirdeğin `tools::imperial`'i ve `point_text`'in `Len`'i), Proje ayarları ve sihirbazda İnç ve Ayak, `cad.settings.set`'in
  `lengthStyle`'ı (`style_needs_imperial`), `cad.units.change`'in oranı, DXF'in `$INSUNITS` 1 ve 2'si (ADR 0262);
  eğri üzerindeki yer ve blok içi alt nesne: başvurunun `along` yeri (kenar ya da eğri boyunca `t`; Yakın, Dik, Teğet ve Çeyrek
  kenetten), eğri aralığı (`SubRange`, `range_length`, `follow_range`), blok içi parçanın yolu (`path`; yerleştirme ya da tanım
  değişince belge izler, `follow_in`, `piece_gone`), `.kcad` şema 51, `FORMATS_VERSION` 62, Yeniden bağla ve ölçü komutları (ADR 0263);
  ileri CAD nesnelerinin kaynak modeli (dolu alan, bölge, wipeout, merkez işareti, eksen, çok kollu kılavuz; ADR 0264); modelin taşıma
  matrisi (bütün `.kcad` örnekleri kodekten, sütunlardan, bulutun kaynağından ve Python'dan; `tests/carriage.rs`, `test_carriage.py`) ve
  yeni şemanın altı adımlık kuralı (`kentos-kcad`'in `schemas` testi) (ADR 0265);
  yazılan uzunlukta metrik birim (`2.5m`, `250cm`) ve parantezli toplam (`(12.5+3*2)`), araçların uzunluk alanları tek yardımcıdan (ADR 0266);
  koordinat kilitleri: değer alanında doğu ve kuzey (projenin eksen adlarıyla X ve Y) ayrı kilitlenir, uzunlukla birlikte de (çemberle
  doğrunun imlece yakın kesişimi), kilidin çerçevesi UCS'tir ve çipte “(UCS)” der, kilitli koordinatın kesikli kılavuzu; `draft.lock.east`,
  `.north`, çekirdeğin `coordinate_point`'i (ADR 0267);
  çizim yardımlarının önceliği: koordinat kilidi, doğrultu kilidi, Dik açı, kenet ve izleme, Orto, kutupsal izleme (`Aid`,
  `Session::ruling_aid`); Orto ve kutupsal birbirini kapatmaz, Orto önce gelir; Shift ve F8/F10 önizlemeyi hemen yeniler, durum
  çubuğunda Orto'nun geçici hâli ve kutupsalın askısı; izlerin `hold` adımı ve `aid` beklentisi (ADR 0268);
  kenet türlerinin kapsam matrisi (`snap_coverage_cases.py`), Ekleme noktası türü (yazı, blok, tablo; `snap.insertion`), yayın ve yaylı
  kenarın çeyrekleri, tek seferlik Merkez'de eğrinin üstünden merkez (ADR 0269);
  nokta yapımları: salt okunur `cad.points.construct` (iki nokta ortası, uzantı, paralel, izleme, kutupsal, hat üzerinde), Python ve MCP'de;
  Geçici izleme noktası (`draft.trackPoint`), tek seferlik menüde Referans noktası ve İki nokta ortası (ADR 0270);
  kenet adayları arasında Tab ve Shift+Tab (çekirdeğin `snap_candidates`'i: ağırlıklı uzaklık, eşitlikte çizimin sırası), işarette
  nesnenin türü ve sıra (ADR 0271);
  tutamaç menüsü her tutamakta (Esnet, Taşı, Döndür, Ölçekle, Aynala; tutamacın noktası ilk nokta) ve köşe komutu
  `cad.entities.vertex` (köşe sil, köşe ekle, düz kenar, yaya dönüştür; topoloji komşularıyla); tutamaç menüsünün köşe işleri ve
  Köşe ekle/sil aracı onunla yazar (ADR 0279);
  tutamaç kipleri: Shift ile birden çok sıcak tutamaç, E Esnet, T Taşı, D Döndür, L Ölçekle, A Aynala, B taban noktası, R referans,
  K kopya (ADR 0280);
  yazılan adlar: seçenek adıyla ya da adının başıyla, komut satırında bağlama göre tam ad ve tekil kısaltma, belirsiz olan adaylarıyla
  söylenir; takma ad ve seçenek harfi çakışmaları testte (ADR 0281);
  AutoCAD takma adları: Uygulama ayarları › Görünüm › Komut satırı'nda kapalı başlayan tercih (`drafting.autocadAliases`); komut satırı
  AutoCAD'in acad.pgp adlarını da alır (RECTANG, LA, DT, U, UNDO, QSAVE; yöntemiyle JOG), profil veridir
  (`catalog/v1/aliases-autocad.json`), KentOS'un adları, yöntem adları ve tek tuş kısayolları önce gelir (B, CP, D, I, P, POLYGON, S, T
  eklenmez); öneri listesinde ve F1'de görünür, tuşlar ve seçenek harfleri değişmez (`alias_profile.rs`, ADR 0282);
  şeffaf komutlar: Kaydır, Pencere yakınlaştır, Mesafe ölç, Açı ölç ve Koordinat oku çalışan komutun ya da sıcak tutamacın üstünde
  çalışır (`Session::start_over`, `TRANSPARENT`), komut noktaları, seçenekleri, istemi, kilitleri, referansı ve Yapım kipiyle bekler,
  taslağı altta çizilir, Esc ya da Enter ile kaldığı yerden sürer; askıdakiler yığın (nokta hesaplayıcı şeffaf ölçmenin üstünde de);
  görünüm değişince ya da komut dönünce imleç yeniden verilir, önizleme yeni görünümde imlecin altında (`follow_view`); çizen araç
  şeffaf değildir (ADR 0283);
  komut geçmişinde çağrılar: Komut geçmişi çizimin kaydını (`kentos.calls/1`) okur ve her ürün komutunu Türkçe başlığı, düz Türkçe
  girdisi, yazdığı, uyarı ve hata kodlarıyla gösterir (aynı geri alma adımı birlikte, geri alınan soluk); yazmadan dönen çağrılar
  `call::recorded`'dan belgeye not edilir (`Document::note_refusal`, `refused_calls`; oturumun); her çağrının Python kodu
  (`kentos_native_application::python_call`) gösterilir ve kopyalanır, tekrar oynatma girdisi (`kentos.replay/1`) kopyalanır ya da
  kaydedilir; ortak komut durumlarının çağrıları `scripts/python/history_calls.py` ile SDK'ya karşı denetlenir (`history/`, ADR 0284);
  büyük seçimde Öznitelikler: karışık seçimde kilitsizler düzenlenir, kilitliler atlanır ve ilk satırda sayılır; Katman ortak ya da Çeşitli;
  Ortak öznitelikler (değer ya da “Çeşitli (k/n nesnede)”, bütün seçime tek adımda); 100 000 nesnede panel 48 ms (ADR 0286);
  çizim araçlarının yöntemleri: AutoCAD'le karşılaştırma ADR 0287'nin tablosunda; araçların yöntemleri arayüz kaydında
  (`catalog/v1/additions.json`'un `methods`'u, `AddedMethod`, `place_methods`); şeritte Yay'ın 11 yöntemi, Daire'nin Merkez, çap'ı, Elips
  ve Revizyon bulutu yöntemleri, Düzgün çokgen'in çember içinde, dışında, kenardan'ı; Dikdörtgen'in Alan'ı (A) ve Boyutlarla, Alanla
  yöntemleri, Yardımcı çizgi'nin Paralel'i (P, Noktadan), Revizyon bulutu'nun Nesne'si (N); hesap çekirdekte (`rect_side_for_area`,
  `xline_offset`, `cloud_around`), yazma araçların kendi komutlarıyla (ADR 0287);
  formlar (ADR 0400–0446; 0446 ajanın form işinin değerlendiricisi `kentos.forms_eval` ve `fixtures/forms-eval/v1`; 0445 `cad.forms.edit`'in `withdraw`'ı ve formun `withdrawn`'ı (`form_withdrawn`, `form_enforced`), bağ tablosu geometrisiz kayıt katmanı ve iki ilişkiyle; 0444 ilişkili kayıtlar öğesinin `min`, `max`'ı (`form_related_few`, `form_related_many`, gönderimin alt kayıtları sayılır), yayının `fields`'ı (`bound_fields_digest`) ve `cad.forms.validate`'in `fieldsChanged`'ı, Formla düzenle'de ⓘ, pencere ile komutun eşdeğerlik testi; 0443 `cad.schema.describe` (sayfa sayfa şema keşfi), alanın `redact`'ı ve komut geçmişinin gizleyen Python betiği, Python `cad.compatibility`, `cad.require`, `cad.FORM_PROFILE`, formun `translations`'ı (`form_translation_item`), grubun `across`'u, Sonraki sorun, kapanış sorusu, Kaydet'in bütün taslağı yazması; 0442 yayında senaryolar `form_tests_failed` ve kırılan kayıtlar `breaking`, `form_breaking`, sürüm farkının `areas`'ı ve `requiresAdded`, iş akışının `notes`'u ve `cad.forms.transition`'ın `notes`'u, eylemin `kind`'i (tool, recompute, step, show) ve `runs`'ı, `form_action_step`, `cad.attachments.inspect`, Formla düzenle'de Yeni sürümle aç ve Alanlara not; 0441 bölücü `divider`, eylem `action` (kayıtlı işlem aracı), ilişkili kayıtların `chart` görünümü, formun `comments`'i ve `cad.forms.edit`'in `comment`, `resolve`'u, `cad.forms.validate`'in `requires`'ı (`kentos_forms::requirements`), tasarımcıda paletler, cihazdaki bölüm kitaplığı ve kalan taslak (`forms/library.rs`); 0440 `cad.forms.submit`'in `children`'ı ve cevabın `changes`, `snapshot`, `undo`'su, silmenin `followed`'ı ve `relation_locked`, `relation_in_use`, `relation_cycle`, iş akışı satırının anlık görüntüsü, ilişkili kayıtların `cards` görünümü, iç içe form; 0439 başka katmandan listenin `sort`, `columns`, `limit`, `copy`, `stamp`, `duplicates`, `spatial`'ı, zincirin `fold`'u, `minSearch`, `cad.domains.export`, `cad.domains.edit`'in `import`'u, `cad.fields.migrate`, işlem araçlarında Yinelenen anahtar; 0438 kayıt kuralları `rules`, `cad.layers.rules`, `rule_broken`, kontrol zamanı `on`, ekranın yayını ve kaydı `release`, `opened`, tekil alan, hariç uç, birim, değerin günleri; 0437 yansıma, ağaç, adım adım, kopyala ve yapıştır; 0436 tarih ve saat aralığı; 0435 form paketi `cad.forms.export`, `cad.forms.import`; 0434 formun JSON Schema'sı ve `docs/specs/forms-v1.md`; 0433 formda konum `location`; 0432 taslak durumlar `drafts`; 0431 aranır liste ve etiketler `search`, `chips`; 0430 projenin değer listeleri `domains`, alanın `domain`'i, `cad.domains.edit`; 0429 iş akışının geçmişi `log`; 0428 kayıt iş akışı `workflow`, `cad.forms.transition`; 0427 sürümleri karşılaştır `cad.forms.compare`, `kentos_forms::compare`; 0426 sekmenin kuralı ve Alt türlerde göster; 0425 ilişkili kayıtların tablosu `view`, `columns`; 0424 büyük form ve katmanda süreler `forms_timing`; 0423 değer bileşimleri `contingents`, `cad.layers.contingents`, `invalid_combination`; 0422 alt türler `subtypes`, `cad.layers.subtypes`, `subtype_misfit`; 0421 Hesapları yenile `cad.forms.recompute`; 0420 form senaryoları `tests`, `cad.forms.test`, `kentos_forms::scenario`; 0419 alanın bölme ve birleştirme kuralı `split`, `merge`, `field_policy.rs`; 0418 kullanımdan kalkan liste değeri `retired`; 0417 değiştirilebilir hesap `overridable`; 0416 Öznitelik tablosunda form görünümü (Tablo | Form, `Entry::docked`); 0415 kaydı alt kayıtlarıyla çoğaltma `cad.records.duplicate`; 0414 alanın kullanıldığı yerler `cad.fields.usage`, yeniden adlandırmada başvuruların izlenmesi `kentos_forms::field_refs`; 0413 Öznitelik tablosunda koşullu biçim `tableStyles`, `cad.layers.table`; 0412 tarih ve saat, saat alan türleri; 0410 geometrisiz kayıt `record`, `cad.records.create`, Öznitelik tablosunda Kayıt ekle; 0411 `forms.design`, `forms.publish`, `forms.use`, Veri girişi rolü, sunucunun yazma bekçisi `form_guard.rs`): Tab ile alanlar arasında gezinme ve klavyeyle form öğeleri; katmanın veri giriş ekranı projede
  (`.kcad` şema 72 `forms`, alanın kararlı `id`'si), çekirdek `kentos-forms` (ön denetim, kuralların durumu, kaydın doğrulaması,
  yazma profili), `cad.forms.edit`, `.validate`, `.submit` ve `cad.entities.set`'te `form_rules`; Formlar penceresi (tasarımcı) ve
  Formla düzenle; tipli evet/hayır ve sayı alanları ifadelerde türüyle okunur; Python `kentos.cad.forms`, MCP; ifadeli varsayılan,
  hesaplanan alan, seçenek süzgeci (`@secenek`), tekil alan, son değeri kullan, uyarı denetimi, kapalı grup, gizlenenleri silme,
  şablonlu not; kurallarda nesnenin ölçüleri ve komutun `moment`, `user`'ı; belge formu uygulanan katmanın her yazmasını aynı
  adımda tamamlar (`kentos_domain`'in `form_follow.rs`'i); tasarımcıda geri alma, ağaçta sürükleme ve canlı önizleme, çizilen
  nesnenin formu (`drafting.formOnCreate`), toplu yazma `cad.forms.fill` ve Formla düzenle'nin toplu kipi; başka katmandan
  değer listesi (`lookup`: zincir, süzgeç, `$uid` ile nesneye bağ) ve ilişkili kayıtlar öğesi (`related`), `Document::form_world`;
  projenin ilişkileri (`relations`, `cad.relations.edit`; silme politikası belgede `relation_follow.rs`, `restrict`
  `cad.entities.delete`'te), İlişkiler penceresi, formda Bağla ve Bağı kopar; katmanın bütün kayıtları formla salt okunur
  `cad.forms.audit` ile (Kayıtları denetle), sayı alanında kaydırıcı (`slider`, aralık ister); formda ekler öğesi (kaydın
  fotoğrafları ve belgeleri, en az ve en çok), renk öğesi ve katmanın değerlerinden öneri (`suggest`); çok değerli alan
  (`LayerField.multiple`, kodlar `; ` ile) ve çoklu seçim öğesi, Formla düzenle'de kayıtlar arasında ◀ ▶; çoktan çoğa
  ilişki (çok değerli başvuru, silme politikası öbür bağları korur) ve ilişkili kayıtlarda Yeni;
  CBS planı ([plan/gis.md](../../plan/gis.md), ADR 0500–; 0744 önbelleğin kazancı ve zamanda geri dönüş (göç 0037 `kentos.job_cache_metrics`, `kentosd jobs`'un `önbellek` satırları, `CacheGain::saved_seconds`; `scripts/ops/pitr_drill.sh` geçici docker PostgreSQL'de WAL arşivi, `pg_basebackup` ve `recovery_target_time`: RPO `archive_timeout`, RTO ölçülür; nesne deposu döndürülen yedekten ve `backup check`); 0743 ajan gizli alanla süzmez, hesaplamaz ve onu dosyaya yazmaz (girdide gizli alanı anan anahtar, ad, ifade ya da şablon çalışmadan `hidden_field` ile reddedilir `Veil::named_in`, tırnaktaki metin ve yazılan içerik sayılmaz; ajanın dosyası gizli alanlar çıkarılmış okuma kopyasından `Document::reading_copy_without`, `file_commands::call_without`, iş olarak `Jobs::start_for_agent`; işlemin `sink`'i `hidden_field_sink`; tek giriş `dispatch::call_envelope_for_agent`); 0742 büyük çizimde tek düzenleme yalnız dokunduğunu yeniden kurar (katmanın kendi çizim sırası olan nesneleri parça parça `style::scene`'in `Layer.ordered`, Kırp'ın çerçeveleri belgenin günlüğüyle `clip_frames.rs`, Bloklar panelinin yerleştirme sayımı çizimin kuşağıyla; silme 250 000'de 62 → 30 ms, 500 000'de 118 → 53 ms; `perf::frame::edit_parts`, `docs/perf/edit-2026-10-29.md`; kaydırmanın GPU payı yoğun görünümde 498 / 1 044 ms, ADR 0597'nin sonucu yalnız arayüz iş parçacığıydı); 0741 İşlemler araçlarının açık seçenek hücreleri ve yazmayan adımın değişmezi (788 hücrenin 781'i bağımsız başvurulu durumla: raster işlemlerinin istatistikleri, yüzey rampaları ve ayları, Tablodan nokta ve Koordinat sistemine dönüştür'ün bütün sistemleri PROJ'un adlı EPSG yollarıyla 1 mm içinde `table_points_systems_cases.py`, `reproject_systems_cases.py`, “Çizime ekle” kapalı her raster aracı `raster_ops_processing_cases.quiet`; COPC'nin sınıfları ve kayıtları sıradan bağımsız `sortedRecords`; ortak durum koşucuları (Rust `Held`, Python `writes_nothing`) plan, doğrulama, sorgu ve retten sonra revizyonu, kirliliği, geri alma ve yinelemeyi denetler, çağrıların kaydı sayılmaz); 0740 okunamayan sayfa ve taşınan kaynak tek akıştan (`sources::trouble::Trouble::of`: reddin kodundan Yeniden dene (`source_unreachable`, `import_unreadable`), Yeniden bağla (`file_not_found`, `table_not_found`, …), ilk sayfadan (`source_changed`) ya da daralt; masaüstünde `source_trouble::{Refused, way_out}`: Kaynağın tümü'nde okunamayan sayfa okunanları tutar, “Sayfa n okunamadı” ve tek düğme, ilk sayfanın reddi “kayıt yok” demez, Durdur'dan sonra Sürdür; İstatistik'in Kaynağın tümü ve birleşik görünümün kaynak tablosu (`JoinRead.refused`) aynı düğmeyle; Yeniden bağla… aynı dosyanın diğer bağlı katmanlarını da bağlar ve okunamayanı yeniden sorar `source_back`; aynı soru kaldığı yerden sürer, imleç tablonundur, dosya önce aranır, damga sonra; ortak durum koşucularında `moveFile` adımı); 0739 sunucu işlerinde kullanıcının payı, sonuç önbelleği, dolu disk ve düzenli yedek (`KENTOS_JOB_USER_LIMIT` kurumun sınırı içinde, `kentos.job_room` kurumun kilidiyle, ikisi doluysa kullanıcının sözü; dışa aktarma, atlas ve dosya yazan araç aynı proje durumu (revizyon ve olay imleci), girdi, erişim damgası ve yazılımla önceki işin dosyasını alır, atlas ve araçta kullanıcı ve gün de anahtarda `kentos_application::job_cache`, her işin kendi sabit bağı `Blobs::share`, `fresh`, `cachedFrom`, göç 0036; dolu depo `storage_full` 507, `blobs::is_full`, parça ya bütün ya hiç `Blobs::append` (tokio'nun arka plan yazmasının hatası yalnız `flush`'ta döner: `sync_data`'dan önce `flush`, `/dev/full` birim testi), tatbikat gerçek dolu tmpfs'te `scripts/ops/disk_full_drill.sh`; `kentosd backup --into KÖK --keep 7` ve `backup list`: değişmeyen nesneler önceki yedekten bağlanır `backup::take_with`, döndürme `backup::rotation`, başarısız yedek kendi klasörünü siler); 0738 ajanın okuduğu, Veri girişinin sayfaları ve hiç erişilmemiş kaydın olayları (`kentos_headless::Veil`: formun `redact` ve katmanın `withheld` alanlarının değerleri MCP'nin ve masaüstünün ajan bağlantısının cevaplarında `‹gizli›`, öznitelik ve değer haritaları, birleşik görünüm, uzlaştırma, istatistik, retlerin tırnaklı değeri, kayıt; özet `hidden`; `rpc::call_for_agent`, konsolun Python'u bütün okur; sunucuda Veri girişinin sayfası dolu ve imleci kendi kaydı `projects::reached_page`; olay kaydında önceki kopyanın kapsamın okuduğu alanları `events::Earlier`, `Scopes::reads`, `Reach::reached`, hiç erişilmemiş kaydın olayı gitmez); 0737 Veri girişinin sunucuda birleştirilmesi ve taslağın kopyası (`kentos_contracts::merge_reading`: okunmayan alanlar taban ve kopya için saklı kopyadan; `edit` Veri girişinde de birleşir, birleşim formların bekçisinden geçer, `merged` okunduğu biçimde; uzlaştırmanın tabanı çok köşeli nesnede aynı kaynağın öbür katmanlarıyla paylaşılan köşeleri tutar; Kayıt çakışması'nda Kopyasını dosyaya yaz, `saving::Target::Copy`; CBS-37 tamam); 0736 adres kaydıyla karşılaştırma (salt okunur `cad.addresses.compare`: kayıt tablo dosyasından ya da katmandan, sütunları adlarıyla; kodu kayıtta olmayan, farklı yazılan, yerinden uzak kapılar, çizimde olmayan adresler, yinelenen kodlar `ops::geocode::register::compare`; Adres kaydıyla karşılaştır penceresi, karşılaştırma kendi iş parçacığında, Farkları kayıttan yaz tek değişiklik kümesi); 0735 kurumun profili ve eşlemesi projenin kendi dosyasında (ayarların `profileFiles`'ı: tür, ad, belgenin metni, `.kcad` şema 132; `profile_files::Shelf` ile aynı ad ve türde çizimin kopyası klasördekinden önce okunur; `cad.profiles.import`'un `store`'u `folder` ya da `drawing` (çizimin dosyası gerekmez, proje ayarıdır), `profile_exists`, `profile_shadowed`, `expectedRevision`; listede `source: drawing` ve `shadowed`; yeni `cad.profiles.remove` yalnız çizimdeki kopyayı çıkarır, `profile_still_named`; sunucu `check_profile_files`; Veri profili penceresinde Nereye ve sorulu kaldırma); 0655 servis belirtecinin yenilenmesi ve CBS kabul projesi masaüstünde (ArcGIS ve OAuth 2 karo servislerinin belirteci bitmeden ve sunucu reddedince yeniden alınır `kentos_services::auth::renew`, `Token::refused`, masaüstünde `net::tile_request` ve `refuse_token`; `parcels.json` masaüstünde ajan bağlantısıyla, yarıda durdurulan kayıt dosyaya dokunmaz `acceptance_gis_tests.rs`; OGC API'den alınan katmanın alanları queryables'tan tipli `kentos_services::queryables`; yalnız CQL2 JSON okuyan servise süzgeç Basic CQL2 metninden çevrilir `kentos_services::cql2::to_json`, `filter-lang=cql2-json`; kabul projesi PostGIS'e yazılıp geri okunur `the_parcels_scenario_goes_to_postgis`; CBS-43 ve CBS-44 sahibin kararıyla kapandı, kalanlar ADR'de); 0654 büyük çizimi kaydederken ve açarken sınırlı bellek (kodlayıcı nesneleri olduğu yerden okur `encode::Objs`, belgenin paylaşılan nesneleri `Document::to_snapshot_v2_shared`, `kentos_kcad::encode_verified_parts_watched`; doğrulama nesne nesne `decode::payload_checked` ve `columns::Same`, ikinci çizim kurulmaz; masaüstü `saving::write_shared`, geçici dosya parça parça geri okunur; açılışta liste boşaldıkça küçülür; 1 milyon parselde kaydın tepesi 4 240 → 1 018 MB); 0653 bir ajanın CBS işinin değerlendirilmesi (`kentos.gis_eval`: komut çağrıları çizimin kopyasında, valid, scope (soru çizimi değiştirmez, yazan komut çağırmaz), outcome (`cad.sources.query` ile katmandaki sayı, cevabın söylediği sayı), untrusted (öznitelikteki yönerge), hidden (`withheld` alanın değeri); `report` oranları, doğal dil başarısı kapsamdan ayrı; `python -m kentos.gis_eval`; `fixtures/gis-eval/v1`); 0652 her İşlemler aracının ortak durumu ve komutla çalışması (nokta bulutunun dokuz aracı `fixtures/processing/v1/pointcloud.json`'da, yazılan bulut okunup sayısı, noktaları, sınıfları ya da kayıtları başvurunun özetiyle `cloudOf`, raster `cloudRasterOf`; bellekteki ev sahibi bulut okur ve COPC yazar; COG olarak kaydet `raster-ops.json`'da; eşdeğerlik testi pointcloud ve multidim'i de oynatır; kapsam matrisinde durumsuz araç 0); 0651 nesnenin kaynağı ve bağın durumu arayüzde (`cad.sources.describe`'ın `uids`'i: bağlı katmanın nesnelerinin `objects`'i, bağ, tablo, anahtar ve değeri, yazılabilirlik, `matched`/`edited`/`added`/`unknown` tablonun okunmadan bağın tabanından `sources::objects_of`, `object_source`; bağın durumunun `filtered`'ı katman süzgecinden geçen sayı `layer_filter::passing`, `error`'ı sağlayıcının nedeni; Bilgi kartında “Kaynak: …” satırı çizimin sürümüyle önbellekte; Kaynak durumu'nda süzgeç, Neden:, veritabanında Yeniden dene ve bulut taslağı); 0650 sunucunun yedeği ve geri yükleme tatbikatı (`kentosd backup --dir`: salt okunur `repeatable read` işlemde `pg_export_snapshot()`, o anın satırlarının adladığı nesneler (dosya revizyonları, kontrol noktası görüntüleri, işlerin dosyaları, alınmış iş girdileri), aynı anla `pg_dump --snapshot`, nesneler SHA-256'larıyla kopyalanarak `nesneler/`'e, `yedek.json` `kentos.backup/1`; parola ortamda `kentos_postgres::env::split_password`; `kentosd backup check [--dir]` yedeğin klasörünü ya da çalışan sunucuyu denetler: eksik, farklı, adlanmamış; `kentos_application::backup`; tatbikat `TestDb::restored`, bağımsız denetim `scripts/ops/backup_check.py`); 0649 Veri girişinin satır kapsamı SQL'de ve havuzdaki bağlantı (`kentos_forms::scope::Scopes::sql_of` ve `Reach::scope_sql`: kapsamın veritabanının denetleyebildiği kısmı `ScopeSql` olarak, `Nothing` hiç okunmaz, `Judge` bütün katman okunup yargılanır, `Where` SQL'in tuttuğu satırlar okunup yeniden yargılanır; formlar `OR` ile, yalnız dilin metin okuduğu alanlar, `@` değerleri bağlanır, `pushdown::push_after` yer tutucuları kaydırır; sunucunun `reached_ids`'i kapsamlı katmanları tek sorguda okur; tek bağlantılı havuzda kurum A'nın onaylanan ve hatayla biten işleminden sonra bağlam boş, B yalnız kendini görür `a_pooled_connection_keeps_no_scope`); 0648 sunucuda alan alan birleştirme ve birlikte uzlaştırma (`project.changes`'in `edit` değişikliği: istemcinin bildiği kopya `base`, sürüm tutmayınca sunucu kilit altında `merge_entities` ile birleştirir, temiz birleşim yazılır ve cevabın `merged`'inde gelir, çatışan alan, silinen nesne ve Veri girişi çatışma kalır; `kentos_cloud` güncellemeyi `edit` gönderir ve `ProjectSync::merged_in` birleşimi çizime alır; `cad.sources.reconcile`'ın `with`'i tek PostGIS veritabanının katmanlarını da tek işlemde uzlaştırır; tabanın kaydı 128'den çok köşeli ve çok parçalı nesnede katmandaki ortak köşeleri tutar `base::shared_corners`, grup kurulur); 0647 adres yazılışları, eşlemeyle göç ve değer listesinin kimliği (`ops::geocode::check::name_groups`: `cad.addresses.check`'in mahalle ve ilçe yazılışları; `cad.addresses.normalize`: yol, mahalle ya da ilçe adı her grupta `prefer`'in ilk yazılanı ya da en çok yazılanla, tek geri alma adımı “Adres yazılışlarını düzelt”, Adres verisi penceresinde Yazılışları düzelt; göçün `renameDomain` adımı, bağlı alanlar izler, `domain_exists`; `cad.profiles.migrate`'in `mapping` ve `mappingName`'i `profiles::mapping_for` ile, adımlar hedef sürümdeki sınıfa ve alana götürülüp kurumun adlarına çevrilir, eşlemenin verdiği ad değişmez, cevabın `mapping`'i); 0646 değişmez duruma bağlı tarihsel sorgu, dışa aktarmada zaman ve sensör gözlemleri (`cad.time.query`'nin `revision`'ı geri alma geçmişindeki ya da dosyadaki durumu sorar `drawing_compare::other_state`, cevabın `state` ve `stamp`'i katmanın o durumdaki içerik özeti `sources::layer_digest`, `expectedStamp` tutmazsa `state_changed` çakışması; yedi dışa aktarmanın `time`'ı `QueryTime` ile, çizimin kopyasından `time_export::filtered`, zamansız katman bütünüyle, `time_left_out`; sensör katmanı zaman ayarlı kayıt katmanı: `key` varlık, `start` olay zamanı, `recordedStart` alım zamanı; `cad.sensors.ingest` satırlardan, SensorThings dosyasından ya da adresinden `@iot.nextLink`'le, yinelenen gözlemde `first`/`last`/`all` (kayıt bitişi tutan katmanda eski sürüm o anda biter), geç gelende `accept`/`reject` ve `lateField`, satır retleri `rejected`'da; salt okunur `cad.sensors.latest` her varlığın son gözlemi, yaşı ve `staleAfter` ile eskimişliği, `knownAt`, `expectedAssets`, `missing`; masaüstünde Sensör gözlemleri penceresi `time.sensors`, dışa aktarma pencerelerinde Zaman sürgüsünün zamanıyla); 0645 parçalı çok boyutlu biçimler, başsız adres, metin bulutu ve meshte alan ve hat (NetCDF-4 ve HDF5 `multidim::hdf5`, GRIB1 ve GRIB2 `multidim::grib` (CCSDS ve PNG paketleme), Zarr v2 ve v3 `multidim::zarr`, Blosc `multidim::blosc`: kendi okuyucularımız dosyayı sanal klasik CDF-5'e çevirir `multidim::chunked::VirtualFile`, parçalar istendikçe çözülür, global `_KentOSFormat` ve `CubeInfo.format`; ev sahipleri `kentos_native_application::multidim_source` ile (masaüstünün karoları ve pencereleri, başsız `LocalFiles`, `cad.meshes.*`); ağ `kentos_services::remote` ve `remote_host::register`: masaüstü ve MCP `kentos_cloud::range::HttpRemote`, Python `kentos._remote.Http` urllib ile; metin bulutu `pointclouds::TextPoints` ile alan, profil ve sorguda; `cad.meshes.region` yüzün merkeziyle, alanla ağırlıklı ortalama ve değer × alan, `cad.meshes.profile` istasyonlar, `MeshFace.neighbours`; Mesh sor'da Yer, Alan ve Hat; Raster ekle'nin adresi çok boyutlu veriyi pencereye götürür); 0644 STAC varlıklarına erişim, kalite maskesi, yeniden izdüşüm ve modelin kaydı (`s3://` adres kovanın bölgesindeki HTTPS adresiyle, saklama uzantısının v1 ve v2'si `region`, `requesterPays`; masaüstünün aralık okuyucusu S3'e kullanıcının AWS kimlik bilgisiyle Signature V4 imzası `kentos_services::stac::access::sign`, `kentos_cloud::access`, Planetary Computer'ın blobuna reddedilince kapsayıcının SAS belirteci, saklanan süreyle; `stac.access`, `stac.requester_pays`, `stac_requester_pays`; İşlemler'de Kalite maskesi uygula `remote.qaMask`, kural `values:`/`bits:` ya da kalite rasterinin `stac.qa_mask`'i, `kentos_raster::remote::mask`, gizlenen hücre değişimde sayılmaz; `cad.rasters.reproject`: dosya projenin sistemine, ızgara kenarların görüntüsünden, piksel orta pikselin kapladığından altı basamakla, en yakın ya da değersizleri dışarıda bırakan çift doğrusal, değerler korunur `warp::Job::keeping`, Raster ekle'de Projenin sistemine çevir ve ekle; `cad.stac.prepare`'in `local`, `reproject`, `drawing`'i, kayıtta `reprojected`, `stac_local_missing`, STAC penceresinde Projenin sistemine çevir indirerek; sınıflandırmanın rasterinde `model.*` kaydı, Doğruluk analizinde eğitimle örtüşen doğrulama nesnesi `leaked` ve uyarı); 0643 raster çözümlemesinde analiz ortamı (`kentos_raster::env::Environment`: alan, hücre boyu, hizalama rasteri, NoData; yalnız alanla tabanın dokunulan hücreleri, hücre boyu ya da hizalamayla yeni kafes; `cad.processing.run` ve `.batch`'in `environment`'ı `RasterEnvironment`, çözümü `kentos_processing::raster_env::resolve`, ortamı almayan araç `environment` yoluyla reddeder; bütün raster aileleri ortamla, kendi hücreleriyle çalışanlar alanı kayma olarak alır, Komşuluk istatistiğinin okuma bölgesi kaymayı atlıyordu, düzeldi; bütün rasteri tutan araçlar adresteki rasteri önce diske alır `Files::stage_raster`, `OpsSpec::whole`; her ailede kayıtta ızgaralar `grid_record`; Ağırlıklı toplam ve çakıştırmanın ağırlık duyarlılığı özette `Sensitivity`; İşlemler penceresinde Analiz ortamı bölümü, oturumun ortamı `processing/environment.rs`); 0642 raster kaynağının codec profili ve yaşam döngüsü (`raster::layout::TIFF_CODECS`: GDAL'ın yazdığı TIFF sıkıştırmaları ve okunup okunmadığı, ZSTD `compress::zstd` ve WebP `raster::webp` ile okunur, okunmayan `compression_refused` adıyla ve `gdal_translate` ile Deflate'e çevirme yoluyla; adresteki dosya değişince `range::is_changed`, karo işçisi kaynağı yeniden açar, karonun anahtarında kaynağın `epoch`'u, `Service::notices`; Range'siz sunucu ve adresteki PNG ve JPEG için Raster ekle'de İndir ve dosya olarak ekle `range::download`, `add::download_place`, çizimin `veri` klasörü, 8 GB; piramit klasörüne 8 GB bütçe `pyramid::prune`; çok gigabaytlı COG'da ilk görüntü ölçüldü `scripts/perf/big_cog.py`); 0641 ağın artımlı kurulması ve olayların yalnız dokunulanlarla yerleşmesi (`Graph::build_kept` kesimleri, düğümleri, kenarların özetlerini ve hücre dizinlerini saklar; `Graph::rebuild` özetlerle, `rebuild_known` ev sahibinin eşlemesiyle yalnız değişen noktaların toleransındaki kenarları yeniden keser, dağılan düğümleri yeniden birleştirir, öteki kenarların parçalarını ve tanımın maliyetlerini önceki graftan alır, tam kurmayla bit bit aynı; tam kurmada hücrelerin karması ve her çifte bir bakış; masaüstünde yalnız değişen nesneler okunur `networks::read`, ağın iş parçacığı son girdiyi saklar; olay bağı yalnız yazmanın yazdığı olayları ve yazmadan önce ya da sonra taşınan rota kimliklerinin olaylarını yerleştirir `Document::step_objects`, `event_follow::Reached`; şehirde yeniden kurma 26–34 ms, bölgede 0,40–0,44 s); 0640 yedi günden uzun yolculuk ve hizmet alanını kesenler (saatli kapanan kenarın saati kendi çerçevesinde: sorunun haftası ve kapanmanın değiştiği günler, her yanda bir hafta, 92 günlük ufuk; önce ve sonra ilk ve son haftası yinelenir, `Clock::framed`, `Frame`, `access::HORIZON_DAYS`; hizmet alanının `cut`'ı: son aralığın içinde kenarın nedeni, dönüş, kapalı düğüm, engel ve seviye, hiçbir tesise bağlanmayan kenarlar `apart`, `service_lines_cut`, `AreaCut`, parçalar kurulurken `Graph::find_parts`; seviye aralığı `Graph::level_gaps`, ulaşılamayanda `levels` kapatanı; bağlı katmanın dosyası değişince ağın sözü `sources::stale_layers`, masaüstünde bağın sürümüyle yeniden kurma; 1,2 milyon parçalık bölgede ölçüldü, dış yönlendirici gerekmiyor); 0639 kararlılık fpc'nin kuralıyla (bootstrap örneği her nesneyi bir kez alır, örnekte nesnesi olmayan küme γ = 0; `stability::resample`), R'ın fpc'siyle aynı yeniden örneklerde çapraz denetim, En yakın komşu'nun Donnelly kenar düzeltmesi (`nearest::Edge`, `nearest_edged`, aracın `edge`'i; P kutunun, ölçeklenmiş kutunun ya da çalışma alanının bütün halkalarının çevresi) ve DBSCAN taramasının k-uzaklık grafiği (`StatsRun.graph`, aracın `chart` çıktısı, `ToolDialog.chart`, `specs::for_table`'ın `stats.dbscanSweep`'i); 0638 atlas başka sistemde, nesneye kırpılmış ve kaynağından (`file.export.atlas`'ın `crs`'i kayıttaki metre birimli sistem: sayfalar o sistemin zemininde, haritalar `View.map` ile, yazılar grid yakınsamasıyla taşınır, `kentos_project::systems::paper_projection`; `clipToAtlasFeature` pafta çekirdeğinde `display::map::feature_clip`; `coverage: source` bağlı kapsam tablosunu bütünüyle, parçayla bağlı harita katmanlarını sayfa sayfa kaynaktan okur, cevabın `read`'i, `coverage_not_bound`; sayfalar R-ağacından `Store::reaching`, PDF yazıcısında çizim durumunun belleği, tek geçişte GeoPDF ve yan yana sıkıştırma: 100 sayfa 8,6 → 2,7 s), alanın kalıcı “dışarı verilmez” izni (`LayerField.withheld`, `.kcad` şema 107, `reach::withholding`); 0637 dört görünüme kadar (`view.splitAdd` Görünüm ekle, `MOST_VIEWS`, sabit yerler ve basınca değiş tokuş `activate_other`, × ile kapatma, iki ve üç yan yana, dört iki satırda; Bağla hepsini, Kaydırmalı karşılaştır yalnız iki görünümle; dar görünümün kısa şeridi), revizyon görünümünün kendi zamanı, açık çizimdeki seçim revizyonda kalıcı kimlikle aynı kayıt (`Revision::choose`), temanın gizlediği vurgulanmaz, revizyonun işaretleri günlükle artımlı (`drawing_compare::differs`; bütün fark 100 000 parselde 319 ms), yerleşmeyi bekleyen görünüm boyutunu hemen alır; 0636 parçanın bandı her yerde (PostGIS parçası, paftanın ölçekli kopyası `Spatial::ghosts_of`, başsız kitap ve atlas `kentos_plot::book::with_bands`; bant katmanın süzgecine ve zamanına uyar `Store::set_ghosts_timed`, `band_shown_key`; anahtarı boş ya da yinelenen satırın iğnesi geometriyle; bağın anahtarı ve alanı geri alma adımı `Op::Source`, `Document::record_source`; Kaynağa bağla'da ve Kaynak ▸'de Anahtar alanı, Etiketler'de Denetle ölçeği); 0635 stilin alanlara bağı ve stil alışverişinin genişlemesi (kaynakta yeniden adlandırılan sütunu `cad.sources.sync` izler `renamed`, `source_columns_renamed`; kitaplık simgesindeki eski alan adı `library_symbol_field`; etiketler SLD, QML ve MapLibre'de iki yönde `kentos_formats::style_labels`; QML'de ısı haritası, ters alan, kümeleme, yayma, MapLibre'de ısı haritası; KentOS ile QGIS ifadesi arasında 39 işlevli çeviri `style_expr`; QGIS'in EllipseMarker, FilledMarker, ShapeburstFill'i; MapLibre'de şekil işaretleri sprite ikonu, başsız sunucu sprite'ı çizer `kentos_headless::sprite`; QML `kentos-qml` ve MapLibre `kentos-maplibre` kendi crate'lerinde; alışverişin kodla denetlenen profili `catalog/v1/style-exchange.json` → `docs/style/exchange.md`; Katmanlar'ın Görünüş ▸'ünde Stil yükle ve Stili kaydet uzantıyla; yakınlaştırma eğrisi çizimin ölçeğindeki değeriyle `renderer_of_at`, tek alanlı `case` `match`'tir; vektör kaynağın kendi `maxzoom`'u 24'e kadar `VECTOR_MAX_ZOOM`; Sivas Kent Rehberi'nin stiliyle sınandı); 0631 geoit modeli ve hız ızgarası (`crs::geoid`: GTX ve GeoTIFF düğüm ızgarası, çift doğrusal, sarma; `transform_point` kaynağın geoidi, hız, datum, hedefin geoidi; kayıtta `vgridshift` ve `deformation`; ızgaralar cihazın kitaplığında `crs_models`, `KENTOS_GRID_DIR`; `geoid_missing`, `velocity_missing`; GNSS içe aktar ortometrik projede geoitle; dosyaların düşey sistemi `formats::vertical`, `DeclaredCrs.vertical`, bileşik `.prj` ve GeoPackage `COMPD_CS`, `vertical_crs`; sunucu ile masaüstü bit bit aynı); 0630 harita sisteminde rasterler, resimler ve nokta bulutları (raster karoları n × n ağla `MapProjection::grid_cells` ve `grid`, önbellekte; resim kırpılmış çerçevesinin üçgenleri yarılanarak `mesh_level`, `mesh`, parti `"warped":true` x, y, u, v raster boru hattında resmin dokusuyla; bulut düğümü yüklenirken hücre ağıyla çift doğrusal `on_map`; görünen kutu `box_back`; yerel CRS kapsamı `docs/crs/coverage.md`, PROJ bağdaştırıcısı yok; pafta projenin sisteminde); 0629 alanın kendi türleri (`bigint` 64 bitlik tam sayı, `instant` saat dilimli zaman damgası, `uuid`, `json`, `binary` base64 ile en çok 1 MiB; değer yine metin, tek biçimi `check_value`'nun; ifadede metin, tabloda `bigint` kesin sayı ve `instant` UTC'deki anıyla sıralanır; `.kcad` şema 104; okumada 2⁵³'ü aşan tam sayı, dilimli ya da kesirli tarih-saat, BLOB, PostGIS'in `timestamptz`, `uuid`, `jsonb`, `bytea`'sı ve GeoParquet'in UTC zaman damgası, UUID, JSON ve ikilisi kendi türüne kayıpsız gelir, `geopackage::Reading`; zaman damgası dosyaya UTC'de yazılır; GeoPackage'ta tablonun KentOS belgesi `https://kentos.app/fields/1`, GeoParquet'te `kentos` anahtarı türleri söyler; eşitleme hücreyi katmanın alanının türüyle karşılaştırır `same_cell_as`); 0627 GeoPackage'ın kalanı (JPEG ve WebP karolar: zune-jpeg ve `image-webp`, `raster::webp`; yükseklik karoları OGC 17-066r1 GDAL 3.12'nin okuma kuralıyla, yazma 16 bitlik PNG ya da 32 bitlik TIFF, kaba düzeyler ve karo istatistikleri; `cad.rasters.gpkgread`'in `link`'i: rasterin `tiles`'ı `.kcad` şema 103, karolar dosyadan okuyucunun `LevelFrom::Tiles`'ıyla, yalnız karosu olan zoom'lar düzey, masaüstünde iş parçacıklarında `Origin::Table`, İşlemler'in iki ev sahibi `raster_files::table_open`; `.gpkgwrite`'ın `format: webp`'i; `file.export.gpkg` projenin ilişkilerini OGC 18-000 `kentos_references` ve alanın sütununa bağlı KentOS belgesiyle (`https://kentos.app/relation/1`), görünüşleri QGIS'in `layer_styles`'ına QML ve SLD olarak yazar, `file.import.gpkg` ikisini ve QGIS'in çoktan çoğa ilişkilerini çok değerli başvuru alanı olarak alır, eşleme tabloları ve `layer_styles` katman olmaz; `gpkg_profile.rs`; bağlamada ağ paylaşımı `network_share`, `file_place.rs`); 0573 başsız mozaik ve JPEG rasterler (rasterin dosya işi `kentos_processing::raster_files`'ta, başsız `LocalFiles` JPEG resmi, JPEG sıkıştırmalı TIFF'i ve bağlı VRT mozaiğini açar; `test_headless_rasters.py`); 0572 nokta bulutu ve çok boyutlu araçlar başsız (bulutun ve NetCDF'in dosya işi kabuktan `kentos_processing::cloud_files`'a, başsız `LocalFiles` bulutu, COPC'yi, NetCDF küpünü ve veri setinin rasterini açar; `test_headless_clouds.py`); 0554 CBS'nin ilk kabul projesi (`fixtures/acceptance/v1/parcels.json`: tipli alanlı parsel ve binalar, geri alma, Alan hesapla, Konuma göre seç, kaydet ve aç, GeoPackage, FlatGeobuf, GML, Shapefile ve CSV, GeoPackage'ın yeni projede açılması; Python'da oynatılır, dosyalar OGR'la denetlenir); 0548 kapsam matrisi (`scripts/gis/coverage.py` → `docs/gis/coverage.{json,md}`: komutların ve İşlemler araçlarının işlem + seçenek + veri türü + ev sahibi hücreleri ortak durumlardan) ve pencereyle `cad.processing.run`'ın eşdeğerliği (`tests/cases/equivalence.rs`, 420 durum, raster dosyaları bayt bayt), açık çizimin dosyası `drawing` alanlı komutlara ev sahibinden (`kentos_headless::with_own_file`, `rpc::call_from`; `cad.processing.run` ve `.batch`'in `drawing`'i, `LocalFiles` rasteri `links::find_in` ile bulur); her yazma kayıtta komutun kendi satırı (`call::recorded`, ortak durum koşucusu denetler; `recorded` planın kuruluğunu adımın içine taşır); 0524 topoloji bulgularının kalıcı anahtarı (`incremental::finding_key`) ve artımlı denetim
  (`incremental::recheck`: değişenler, komşuları, en yakına uzaklık ölçen kuralların erimi, boşluk kuralının kümeleri; tam denetimle aynı),
  Topoloji sekmesinde Değişenleri denetle; 0523 bağlı görünümler ve tema (görünümün teması projenin bir katman durumu,
  ağacın kopyasında: `layer_states::themed`, `StyledCache::scene_themed`, `Spatial::set_theme`, `Store::labels_shown_in`; Görünümleri bağla, Kaydırmalı
  karşılaştır, çizim değişmez); 0576 bölünmüş görünümde görünüm başına zaman (sürgüyü izler ya da sürgünün ızgarasında kendi anı: `tools::view_link::own_window`, `Store::with_window`; seçme ve kenet etkin görünümün zamanında), Bağ üç kip (yok, merkez, merkez ve ölçek: `view_link::follow`) ve her görünümde ölçek çipi (1:N, `screen_scale`) ve ikinci görünümde başka revizyon, salt okunur (geri alma geçmişi `Document::past`, dosya, Geçmiş › İkinci görünümde `Download.side`; açık çizimle kalıcı kimlikle eşlenir, silinen ve değişen işaretli; zaman, bağ ve kaynak oturum durumu, `.kcad` değişmez) ve farkı salt okunur `cad.drawing.compare` v1 (`drawing_compare::diff`, `ops::compare` kuralı; Python, MCP, manifest); 0522 CSV tablolar (`file.export.csv` ve `file.import.csv`: GDAL'ın CSV'si, geometri ISO WKT
  ya da X ve Y, yaylar kesin, `.csvt` türleri ve OGC WKT1 `.prj`; okuma GeoPackage içe aktarmasının yolundan, `kentos_geopackage::wkt`; Veri › Dosya alışverişi'nde
  CSV içe ve dışa aktar); 0525 şebeke izlemede ilişkili kayıtlar (`Document::related_of`: borunun aboneleri, vananın haznesi)
  ve doğrulanmamış bağlantılar (`TraceAnswer.junctions`, `.unverified`; araçta sayılır ve seçilir, Python'da `related`); 0557 şebekede
  terminaller ve alt ağlar (ağın cihazları: düğüm katmanı, terminaller ve terminal yolları, kenar uçlarının bağlı olduğu terminal alanları;
  izleme ve Denetle yollardan geçer, terminalsiz uç bütün terminallere geçer ve doğrulanmamıştır; alt ağ: Kaynak düğümleri denetleyici, Akış aşağı
  ya da Bağlı, yetim, paylaşılan ve eski nesneler; `cad.subnetworks.find` ve `.update` (adı özniteliğe tek adımda yazar); `.kcad` şema 93;
  Ağlar'da Terminaller ve Alt ağlar, Analiz › Ağ analizi'nde Alt ağlar penceresi); 0562 terminaller her yerde ve alt ağ adları güncel (en kısa yol,
  hizmet alanı, en yakın tesis ve maliyet matrisi cihazdan yalnız terminal yollarıyla geçer: terminalli ağın aramaları (düğüm, terminal) durumlarıyla
  yürür, `Graph::turn` önce cihazın geçişini sorar, dönüş ve seviye kuralları yine uygulanır; cihazın terminalleri ve yolları noktasının özniteliğinden
  de gelir: `TerminalDevice.by` ve `configs`, değeri Türkçe büyük küçük harfsiz eşleşen yapılandırma, bilinmeyen değer cihazın kendisi ve söylenir;
  `NetworkSubnetworks.keepCurrent`: ağın katmanlarının nesnesine dokunan her yazma alt ağ özniteliğini aynı adımda yeniden yazar
  (`call::recorded`'ın `subnetwork_follow`'u, `cad.subnetworks.update`'in hesabıyla), yazılamayanı `subnetwork_dirty` uyarısıyla söyler; `.kcad`
  şema 94; Ağlar'da cihazın seçen alanı ve yapılandırmaları, Güncelleme); 0559 İşlemler araçları dayanıklı sunucu işi olarak
  (`project.processing`: `cad.processing.run`'ın girdisi, sonuç ya tek `project.changes` komutuyla işin bitirme işleminde projeye kesinleşir, proje aracın
  okuduğu revizyondaysa ve yetki yeniden sorulduktan sonra, geçmeyen kira commit'i de geri alır, ya GeoPackage dosyası olur; araç ayrı süreçte
  `kentos-jobrun`, CPU ve bellek bütçesiyle (`cpu_limit`, `memory_limit`, `memory_estimate`), işçi ilerleme ve sözcüklerini yazar `progress_text`;
  Nerede çalışır › KentOS sunucusunda, Sunucu işleri'nde işlem satırları; Python `processing_job`); 0561 dosya içe aktarma ve atlas PDF'i sunucu işi
  olarak ve MCP'de iş araçları (`project.import`: GeoPackage, FlatGeobuf, GML, CSV ya da DXF önce `kentos.job_input`'a hazırlanır, `POST/PUT …/job-inputs`, tek akışlı gövde,
  boy ve SHA-256 denetimi, `KENTOS_JOB_INPUT_MAX_MB`; çocuk süreç `file.import.*` komutunu çalıştırır, komut kodu işin kodudur, kayıplar sonuçtadır, sonuç blok tanımlarıyla tek
  commit ve işin kesinleşmesi 100 000 değişikliğe kadar; `project.atlas`: `file.export.atlas`, pin denetlenir, PDF artifact'ı Range ile iner; MCP `jobs.start`, `jobs.status`,
  `jobs.cancel`, `jobs.result`; Python `stage_file`, `import_job`, `atlas_job`; masaüstünde “Sunucuya gönder” penceresi; `job_run` düşüşünün kök nedeni kısa işte kullanım örneğinin
  alınamamasıydı); 0563 Shapefile, GeoJSON ve KML/KMZ içe aktarma ürün komutları (`file.import.shp`: `.shp` yanındaki dosyalarıyla ya da `.zip`
  arşivi (`entry`), `file.import.geojson`, `file.import.kml`: ortak okuyucular, katman yönlendirmesi DXF'inkiyle ortak (`layers`, `layer`, `skipLayers`, `group`), dosyanın sistemi girdinin sorusu (`srid`,
  `reproject`; `crs_unknown`, `crs_mismatch`, `crs_overridden`, `crs_undeclared`, `crs_unofficial`; yerel proje olduğu gibi alır), cevapta `moved` (çevirmenin yolu ve doğruluğu), tek geri alma adımı
  “Shapefile: ad.shp”, günlükte komutun adı; pencereler `execute_read` ile yazar; `project.import` işi `shp` (zip), `geojson`, `kml` biçimlerini alır; sunucu iki köşeli yaylı alanı (daire, mercek) masaüstü
  gibi kabul eder ve PostGIS izdüşümü geçerli çokgendir; Python `file.import_.shp`, `.geojson`, `.kml`); 0530 CAD kaynağının izlenebilir GIS türevi (`file.export.gis`: düz kirişler `kentos.chords/1`, daire ve bükümlü kenarda kesin sagitta, her satırda kaynağın kalıcı kimliği, türü, sapması ve özeti, Z/M, yazı ve blok tabloları, tablo başına türetim kaydı metadata uzantısında; `cad.gis.derivation`: eski, GIS'te değişmiş, kaynağı yok; türev tablo bağlanmaz ve geri yazılmaz `derived_dataset`; `fixtures/gis-derivation/v1`); 0634 türevde geri alma, bağ koparma, başka kaplar ve mimari türev (`cad.gis.takeback`: yalnız GIS'te değişmiş ve kaynağı değişmemiş satır, `row_stale`, `row_not_edited`, `row_source_gone`; `take: attributes` sütunları alanın kuralıyla, `take: geometry` düz geometriyi olduğu gibi, eğrinin kirişlerini alıp bağı koparır `link_broken`, tek geri alma adımı “GIS'ten geri al”; `cad.gis.detach`: türetim kaydı nedenli nota `https://kentos.app/derivation-detached/1`, kimlikler bırakılır, tek işlemde; `file.export.gis`'in `format`'ı: GeoPackage, Shapefile `.zip` (setler ve `kentos-turev.json`), GeoJSON (`kentos:table`), PostGIS (yorum ve `kentos_uids`); ortak okuyucu `gis_set::open_any`, halkalar yönden bağımsız, Shapefile'da kotsuz köşe 0; `arch` tabloları `mimari_duvar` … `mimari_aciklik` ve `kentos_dimensions`, hesaplanmış sütun `fields_left`, geometri `geometry_parametric`; masaüstünde Biçim, GIS'teki değişiklikleri al ve Bağı kopar); 0551 toplu çalıştırmada hata kipi ve kayıt (`cad.processing.batch`'in `onError`'ı `stop` ya da `continue`: duran satır kendi kayıt noktasına geri alınır, `Document::group_savepoint` ve `rollback_to`, öteki satırlar tek adımda, `failed` ve `batch_rows_failed`; durdurma çalışan aracın içinde `BatchFlow::stop_flag`, `Runner::with_stop`; `sink.tables` tablo çıktısını GeoPackage'a öznitelik tablosu; katman özeti iş parçacıklarında, anahtarı değişiklik işareti; `cad.processing.run` ve `.batch` protokolün kayıtlı adımı; kayıt `.kcad`'e yazılmaz; Toplu çalıştır'da Hata olursa); 0529 İşlemler'de toplu çalıştırma ve çalışma kaydı (`cad.processing.batch`: araç ya da `model:` kimliği, parametre tablosunun satırları, `cad.processing.run`'ın aynı çalıştırıcısı, tek geri alma adımı “Toplu: <araç>”, ya hep ya hiç, satır retleri `rows[i]` yoluyla, iş olarak; modeller `cad.processing.run`'da da, `tools`'un `models`'i; her cevapta `record` (`kentos.run-record` 1), `kentos_processing::record::rerun_kind`; kayıt `.kcad`'e yazılmaz; masaüstünde Toplu çalıştır, `batch_table` çekirdekte); 0528 STAC keşfi ve veri hazırlama (`cad.stac.search`: bbox, tarih, koleksiyon ve bulut oranıyla arama, sayfalama ve yalnız uç noktanın kendi sunucusuna `next`, imzalı adresin imzasız kalıcı adresi; `cad.stac.prepare`: varlıklar adresli, tek bantlı raster olarak kendi grubunda tek adımda, `kentos.stac-preparation` kaydı ve rasterlerin `stac.*` öznitelikleri; çekirdek `kentos_services::stac`, ağ ev sahibinin `stac_host` + `kentos_cloud::stac::HttpFetch`, test için `Recorded` ve `KENTOS_STAC_RECORDED`; masaüstünde “STAC'ta ara”; gerçek Earth Search cevapları `fixtures/services/v1/stac`); 0558 kurumun kendi profili (çizimin yanındaki `profiller` klasöründe profil ve eşleme dosyası; profilin `release`'i ve `migrations`'ı; `cad.profiles.import` biçim ve sürüm denetimiyle, `.list`, `.migrate`: katman ve alan adı, alan türü, değer yeniden kodlama ve bölme, kodu kullanımdan kaldırma, tek geri alma adımı “Profil göçü” (katman adları hariç), uymayan değerler değer değer; `apply` ve `check`'in `mapping`ı kurumun adlarıyla, hiçbir ad kendiliğinden değişmez; Veri profili penceresinde profil ve eşleme seçici, İçe aktar…, göç; Paketle `profiller/` dosyalarını bayt bayt pakete taşır, `paket.json`'da `profile` ve `mapping` olarak, okunamayan `unreadable`); 0541 veri profilleri (`catalog/v1/profiles/tucbs-temel.json`, `cad.profiles.apply` ve salt okunur `.check`: katman yolları, alanlar, kimlikler, kayıtlı ile geometrik alanın farkı, ilişkiler, adres verisi; kolon adı kodda yok; referans belediye projesi `fixtures/profiles/v1`; Veri profili penceresi; komut türleri ADR 0540'tan beri `kentos-commands`'ta); 0545 GML ve XYZ (`file.export.gml`: GML 3.2, her katman bir nesne türü, yanında `.xsd`, yaylar ArcString, Ring, CompositeCurve olarak kesin, Z `srsDimension`'la, srsName `urn:ogc:def:crs:EPSG::kod` ve EPSG eksen sırası; `file.import.gml`: GML 2, 3.1 ve 3.2, türler `.xsd`, `.gfs` ya da değerlerden, GeoPackage alma yolundan; `file.export.xyz`: noktalar ya da bütün köşeler; kodek `kentos_native_application::{gml_read, gml_write, gml_table, xyz_table}`; Veri › Dosya alışverişi'nde); 0549 işlem sonucunu var olan GeoPackage'a yazma (`sink.mode` `new`, `addTables`, `replaceTables`; öbür tablolara dokunulmaz, aynı ad `table_exists`; tek SQLite işlemi, 5000 satırlık gruplar, iptalde dosya eskisi; yanıtta `sinkReport`; 1.4 dosyaya 1.4 tetikleri); 0544 FlatGeobuf (`kentos_geopackage::fgb`: kendi FlatBuffers yazıcı ve okuyucumuz, paketli Hilbert R-ağacı, yaylar kesin; `file.export.fgb` ve `file.import.fgb` GeoPackage alma yolundan; Veri › Dosya alışverişi'nde FlatGeobuf içe ve dışa aktar); 0546 Öznitelik tablosunda dışarı çıkanlar ve kaynak satırı (Sütunlar ▾: gizlenen sütun tabloda, kopyada, dosyada ve Yapıştır'ın sütun sırasında yok; Kopyala: gösterilen satırlar panoya, başlık alan adları, Yapıştır'la geri; formda `redact` işaretli alan tabloda görünür ama kopyaya, Dışa aktar'a, İstatistik ve Grafik'e girmez ve söylenir; İncele ▾ ve İstatistik ile Grafik'in Göster ▾'inde Tablonun süzgeciyle, aynı `feature_rows`; bağlı katmanın tablosunda Kaynak satırı: Kaynak durumu'nun durumu ve tek işi `sources::layer_state`, `source_status::way_out`, yüklü, gösterilen, seçili ve toplam; çekirdek `feature_table::{visible_columns, outgoing_columns, copy_text, workbook_sheet_of}`); 0543 Kaynak durumu (`cad.sources.describe`'ın `objects`, `rows`, `total` ve durumun istediği iş `next`, `SourceNext::of`; masaüstünde Kaynak durumu penceresi, satırın düğmesi Kaynak ▸'nın komutlarıyla); 0547 sunucu işlerinin sınırları ve masaüstü işler görünümü (kurum başına sırada ve çalışan iş sınırı `KENTOS_JOB_LIMIT`, aşan istek `quota_exceeded` 429, sayma `kentos.job_slot` kurum kilidiyle; sonuç dosyası boy sınırı `KENTOS_JOB_ARTIFACT_MAX_MB` ve `too_large`, yazılamayan depo `storage` ve yarım nesne silinir; `KENTOS_JOB_RETENTION_DAYS` ile biten işin ve dosyasının silinmesi `job_upkeep::expire`, göç 0030; `kentosd jobs` sayıları; ikinci iş türü `project.checkpoint`, idempotency anahtarı iş kimliği; iş dosyasında `Range`; masaüstünde katalogda Sunucu işleri sekmesi: `kentos_cloud::server_jobs`, `Fetch::Job`, 2 sn yoklama, aynı istek kimliğiyle yeniden; Python `checkpoint_job`; `catalog_jobs.rs`); 0542 dayanıklı sunucu işleri (`kentos.server_job`, göç 0029: değişmez girdi, istek kimliğiyle tek iş, kira, yoklama ve çit, deneme ve süre sınırı, iptal; `project.export`'un staged sonucu, bitirmede yetki ve revizyon yeniden sorulur; `…/jobs` yolları, `KENTOS_WORKERS`, `kentosd worker`; Python `export_job`, `wait_job`); 0550 tek erişim çözücüsü (`kentos_forms::reach`: çağıran Veri girişi, okuyan ya da form tasarlayan; alan gizli, işaretli ya da açık; kayıt sorgusu, kopya kuralı, damga; `fixtures/reach/v1`; sunucuda Veri girişinin sayı ve sınırı, olayları, çakışması, liste değerleri ve geçmiş sayıları ondan, bütün formları kaldırılmış katman hiçbir şey vermez; `redact` alan projenin görüntüsünden ve dışa aktarma işinden kesilir, saklı kopya yalnız kesilmeden alabilene, işin kesinleşmesi damgayı yeniden sorar `reach_changed`; yerel çizim ve başsız sunucu tüm hak, çevrimdışı kopya geri alınamaz); 0539 Veri girişinin satır kapsamı (formun `scope`'u, `@kullanici_eposta`, `kentos_forms::scope`, `form_scope_invalid`; sunucuda okuma, sayım ve kimlikle okuma kapsamla süzülür, kapsam dışına yazma 404, kapsam dışına çıkaran 403; Formlar penceresinde Kapsam); 0553 alan alan birleştirme, grup ve çok katman (bağın tabanı alan kaydı tutar, `.kcad` şema 92: tür ve geometri, öznitelik ve köşe özetleri; iki yanın farklı alanlarını değiştirdiği nesne tabana karşı üç yönlü birleşir, yalnız aynı alanı farklı değiştirmek çatışır, `take`/`rest` çatışan yeri seçer; ortak köşeyi iki yanın da taşıdığı komşular ve silinen üst kayıtla eklenen alt kayıt bir grup, tek yanla çözülür `group_split`; `with` ile aynı GeoPackage dosyasının katmanları tek çağrıda ve tek işlemde; bulut istemcisi dışarıdan gelen değişiklikte ve sunucunun 409'unda farklı alanları birleştirir `kentos_contracts::merge_entities`); 0538 kaynakla uzlaştırma (projenin `sourceBases`'i, `.kcad` şema 91: bağın son eşleşmesinde nesne başına katmanın ve tablonun özeti, `sources/base.rs`; `cad.sources.reconcile`: tabana göre iki yanın değişiklikleri, uyuşanlar, çatışmalar farklı öznitelikleri ve geometrisiyle, `take`/`rest`, `unresolved_conflicts`, `no_source_base`, `not_a_conflict`; tablonun yanı katmana, katmanın yanı tabloya tek adım ve tek işlem; masaüstünde Kaynakla uzlaştır penceresi, reddedilen yazma onu açar; ortak durumların `copyFile` adımı); 0537 iki zamanlı katmanlar (`LayerTime`'ın `recordedStart`, `recordedEnd`'i, `.kcad` şema 90; `time::recorded_interval`, `known`; salt okunur `cad.time.query`: geçerlilik anı ya da aralığı ve `knownAt`, `notValid`, `notKnown`, `unrecorded`; mağazanın `set_recorded`, `set_known_at`'i; Zaman ayarları'nda kayıt alanları, Zaman sürgüsünde Bilinen gün); 0552 bulutta alan ve profil, mesh sorgusu (`cad.pointclouds.region` ve `.profile` kaynağın çözünürlüğünde, okunan noktalarla; `cad.meshes.describe` ve `.query`: yüzün kalıcı kimliği, değerler, an, seri; çok dosyalı bulutun özeti; Nokta bulutu alan istatistiği, Nokta bulutu profili, Mesh sor pencereleri; `pointcloud_areas.rs`, `meshes.rs`); 0536 nokta bulutunun kaynak bilgisi ve kalıcı nokta kimliği (`cad.pointclouds.describe`: başlık, yatay ve düşey sistem, oluşturulma, GPS zamanı, boyutlar ve ek baytlar, dönüşler, COPC düğümleri, `classes`; `cad.pointclouds.query`: en yakın nokta tam çözünürlükte, kimliği bulut + kaynak + sıradaki yer; uygulama katmanının `pointclouds.rs`'i dosyaları `links::find` ile bulur; Öznitelikler'de başlık satırları); 0535 raster analizinde alan ve ızgaralar (yüzey analizinin Kapsam'ı: `Spec.window`, `window_of`, `window_affine`; kutunun hücreleri kaynağın ızgarasında, komşuları kaynaktan, değerler bütün rasterinkiyle bit bit aynı; çalışma kaydının `grids`'i, `RunResult.grids`); 0532 bağlı katmanda form ve ilişki işlemi (`cad.sources.sync`'in `with`'i ve `cad.forms.submit`'in `commit`'i üst ve alt kayıtları tek SQLite işleminde, `GeoPackage::edit_many`, `sources/together.rs`; `cad.sources.relate` çok değerli başvuruyu GeoPackage ilişki profiline `kentos_references` eşler, tanınmayan temsil metne çevrilmez; `cad.records.page` sayfalı, süzgeçli ve kaynak sürümlü okuma; `ExecutionContext::dry()` protokolün planında dosya yazmayı keser); 0534 atlas PDF'i (`file.export.atlas`: paftanın atlas kurallarıyla seçilen sayfalar tek PDF'e, her sayfa kendi nesnesinin haritası, tablosu ve lejantıyla; çıktı çizimin parçalarının damgalarına, katmanların özetine ve bağlı kaynağın SHA-256'sına sabitlenir, `AtlasPin`, `pin_mismatch`, `source_changed`; `theme`, `time`, `crs` açık, kayıt PDF'in `Keywords`'ünde; `fields` ve formun `redact`'ı izinli öznitelikleri belirler, süzgeç dışı nesne hiçbir yerde yok: `vector_content`'in `view_shown`'ı; `PdfInputs.atlas`, `kentos-plot`'un `atlas`'ı ve `MapDrawer`'ı; Atlası PDF yap penceresi; ortak durumlar `fixtures/atlas/v1/cases.json`); 0533 etiket iğneleri ve bağlı katman (bağın anahtarı `SourceLink.key`, `.kcad` şema 74'te isteğe bağlı, iğneleri yeniden yazılan tabloda izler; `label_pins_lost`, `label_pins_moved`, `key_not_unique`; `Extent` ve `Key` işlemleri; motor önceki yerleşimi tercih eder `Prior`, yalnız ana görünümde; önbellek kaynağın sürümünü sayar; ekran ve PDF aynı `label_layout`; salt okunur `cad.labels.report` ve Etiketler'de Denetle); 0578 parçalı katmanda etiket iğneleri ve engeller (parçanın bandı: alan kısa kenarının %15'i kadar büyür, bandın satırları katmanın nesnesi olmayanlar aynı dosya sürümünden okunur ve etiketlere yalnız engel olur: `sources::band_of`, `BandJob`, `Store::set_ghosts`, masaüstünde `band_ghosts.rs`, Denetle okumadan çizimin bandını kullanır `labels_report::execute_with`; anahtarsız bağda iğneler yeniden yazılan tabloda aynı geometriye ya da 0,05 m içindeki en yakın aynı türden satıra gider, eşitte verilmez: `ops::pin_match`, `PinMove.by`, `PinLoss::Ambiguous`; alanın dışına çıkan nesnenin iğnesi bağda park edilir, satır alana girince geri verilir: `SourceLink.parked` şema 74'ün içinde, `label_pins_parked`, `label_pins_restored`; `cad.labels.report`'ta `band`, `neighbours`, `parked`, `band_unread`); 0582 PostGIS'te alan, view ve sorgu katmanı, satır düzeyinde çakışma (bağlama ve `extent` işlemi veritabanında da: okuma `&&` ile, alanın sürümü alanın satırlarından; view ve SQL sorgusu salt okunur, anahtar `fid`/`id`/`gid`/`ogc_fid`, sorgunun sütunları geri alınan geçici view'la, okuma `READ ONLY` işlemde, `no_row_key`, `row_key_not_unique`; yazma kilit altında yalnız katmanın değiştirdiği satırları tabana karşı denetler `RowCheck`, aynı satır `source_changed` ve `uids`, başka satırlar `source_others`; masaüstünde PostGIS'e bağlan'da view'lar, Sorgu katmanı ve Yalnız görünen alan; QGIS 4.2.3'le eşzamanlı düzenleme `postgis_qgis_cases.py`); 0583 katmanın kaynak sistemi ve harita sistemi (katman düğümünün `sourceCrs`'i: EPSG kodu, ad, WKT2 `kentos_project::wkt2`, eksen sırası, birim, kullanım alanı; içe aktarmalar `ImportPlan.source_srid` ve bağlı katmanlar yazar; `cad.layers.crs` (Sistem ata, nesneler değişmez) ve `cad.layers.reproject` (Yeniden izdüşür, tek adım, çevrilemeyen nesne `crs_unreached`); projenin `mapSrid`'i, `.kcad` şema 98: stil toplayıcısı ilkelleri sıklaştırarak harita sistemine çevirir `View.map`, `geometry_core::crs::map::MapProjection`, kamera `map` ve `map_ratio` ile imleci, tıklamayı, keneti projenin noktalarıyla verir, vurgu ve ızgara harita sisteminde, harita sisteminde rasterler, bulutlar ve resimler çizilmez ve söylenir; durum çubuğunda Harita hücresi, koordinat sistemi menüsünde Harita sistemi ▸, katmanın sağ tıkında Koordinat sistemi ▸); 0585 veritabanında alanın dışındaki komşular (PostGIS parçasının yazmasında `topology` komşuları değişen köşelerin kutusuyla sağlayıcıdan okur, `topology_edit` ile izletir, ADR 0584'ün tek işleminde yazar; okunan komşu kilitten sonra `neighbours_changed` ile satır satır denetlenir, arada değişen `source_changed` yol `topology`; görünüp değiştirilemeyen satır (yetki, satır güvenliği; `SELECT … FOR UPDATE`) yazmadan önce `rows_denied`, `ProviderError::RowsDenied`; komşunun tam geçerliliği Geçerliliği denetle'nin sorunlarıyla, yinelenen köşe hariç, önceden geçersiz komşu engel değil, GeoPackage'ta da; sağlayıcının `check`'i boş olmayan `revision`'la alanı da denetler); 0599 kaynak gezgini (Kaynaklar'da GeoPackage, FlatGeobuf, GML, raster ve nokta bulutu dosyaları; GeoPackage eklenmeden tablolarına (`cad.sources.describe`, `SourceTable.extent`: `gpkg_contents` ya da PostGIS'in `ST_EstimatedExtent`'i) ve tablo kapsamına ve `cad.sources.query` ile ilk beş kaydına açılır; Bağlı katman olarak aç ve Kopya olarak al pencerenin yolundan, bağlı tabloda “Bağlı: katman” ve Katmana git; Veritabanları ADR 0623'ün adlı bağlantıları, gezgin onlara arama ile sık ve son kullanılanları katar; Servisler hazır altlıklar ve Harita servisi ile eklenenler katmanlarıyla; arama `kentos_interaction::sources::search_*`; Son ve Sık kullanılanlar `kaynak-klasorleri.json` sürüm 2; sürükleyip bırakma ve sistemden dosya `sources::dropped`; Kapsamı göster `sources::extent_in_project` ve kaydedilmeyen çerçeve); 0598 bölmede ve birleştirmede soy bağı (projenin `lineage`'i, `.kcad` şema 101: nesne, ataları, `split` ya da `merge`, adımın adı; `cad.entities.edit`'in bölme ve birleştirmeleri (Kır, Birleştir, Böl, Parçalara ayır, Parçaları birleştir) kaydı aynı geri alma adımında yazar `Op::Lineage`, geri alma geri alır; bağlı katmanın yazması yazdığı satırların soyunu aynı işlemde `kentos_lineage` tablosuna yazar, GeoPackage'ta uzantısıyla, PostGIS'te şemada; kaynağa yazılmış adımın geri alınması çizimindir, sonraki yazma denetimli, arada değişen satır `source_changed`; masaüstü geri almada söyler `Document::undo_report`, `sources::undoing_written`; Python `Document.lineage`, `ancestors`, `descendants`); 0597 büyük veride bütçeler (1 milyon parselde kenet ve seçme p95 ≤0,17 ms, 500 binde bütün çizim ekrandayken kaydırma p95 51 → 0,65 ms ve tekerlek 49 → 4,8 ms: görünüm hareket ederken etiketler yeniden yerleşmez, 150 ms durulunca yerleşir, `viewport.moving`, `Spots`'un iki anahtarı; bağlı dosyanın SHA-256'sı damgasıyla tutulur, git'in yarışlı girdi kuralıyla `file_digest::sha256`; alanla bağlı katman yalnız kendi satırlarının kimliklerini okur `GeoPackage::uids_of`, dışarıdakiler tek akışla `each_uid`; 10 milyon satırda alanla bağlama 8,3 → 2,3 s; ölçüm `docs/perf/budgets-2026-10-23.md`); 0596 Paketle'de kaynakların yolu (`file.package`'ın `geopackages` ve `databases`'i: `copy`, `keep`, `embed`; veritabanı tablosu bağın okuduğu satırlarla (alanla bağlıda yalnız alanınkiler) projenin sisteminde, kalıcı kimlikleriyle `veri/<şema>-<tablo>.gpkg`'ye, bağ kopyaya döner, uzlaştırma tabanı düşer, son eşleşmeden beri değişen tablo `source_changed`; `embed` paketin projesinde bağı kaldırır; `leaveOut`: bağ (katmanın nesneleri paketten çıkar), nesnenin dosyaları, `xref:ad`; cevabın `embedded`'ı, `PackagedFile.table` ve `rows`, `excluded`, `kept`; `validate` veritabanını okumaz; `kentos_headless::package_sources`; masaüstünde Paketle penceresi); 0595 zaman süzgeci kaynakta (`cad.sources.query`'nin ve İşlemler'in `scope: source` girdisinin `time`'ı: `at` ya da [`from`, `to`), `knownAt`, alanlar katmanın zaman ayarından ya da `fields`; her kayıt ADR 0210 §4 ve ADR 0537'nin kuralıyla burada, `time_filter.rs` `cad.time.query` ile ortak; kaynağa kapsayan koşul: tarih sütununun ilk on karakteri bir gün genişletilmiş anlarla, ISO tarihiyle başlamayan değer tutulur, SQLite'ta GLOB, PostgreSQL'de sütunun metni üzerinde düzenli ifade; masaüstünde Zaman sürgüsü açıkken Öznitelik tablosunun ve İstatistik'in Kaynağın tümü soruları sürgünün zamanıyla, sürgü oynayınca yeniden `time_sources`); 0594 raster oturtmada hedeflerin sistemi tanımla (`cad.rasters.georef`'in `crs`'i: WKT 1, WKT 2 ya da PROJ dizesi `crs::text::read_text` ile; kayıttaki sistemin kendisi EPSG kodu olarak kaydedilir, projeninki hiç, başkası verildiği gibi `RasterGeoref.crs`'te, `.kcad` şema 100; hedefler projenin datum seçimleriyle çevrilir, sıfır `+towgs84` EPSG'nin boş TUREF dönüşümüyle; `crs_text_unreadable`; `.points`'in `#CRS:` satırı okunur ve yazılır (tanım, kodun ya da projenin WKT 2'si); Raster oturt'ta tanım satırı ve Kaldır; Hesap tablolarında sütun payı `Table::portion`); 0593 toplu çalıştırmada ve modellerde kaynaktan okuma ve sonucu dosyaya ya da veritabanına yazma (`cad.processing.batch`'in `sink`'i: satırlar çizimin kopyasında `cad.processing.run`'ın çalıştırıcısıyla, sonuç tek yazmada, katman başına tablo, aynı çıktı katmanı tek tablo, tablo çıktıları satırın adıyla `Özet tablosu (Satır 1)`; ortak `scope: source` girdisi bir kez `kaynak-<parametre>`'ye, satırınki `kaynak-<parametre>-<n>`'ye, kopyadaki kaynak katmanı “bina (kaynak)”; modeller aynı yoldan; kaydın `sink`'i `RecordSink` (yer, şema, tablolar `of`/`row`/`rows`, dosyanın boyu ve SHA-256'sı), `RunInput.path`, kayıt girdiyi verildiği gibi ve çizimin sürümünü tutar, `rerun_kind`'da kaynak dosyası SHA-256'yla `ChangedData`, veritabanı `UnknownSource`; `call::recorded_with` ile komut geçmişinde yazılan dosya `effects`; masaüstünde Toplu çalıştır'da Sonucun yeri ve Bütün tablo, `batch_sink.rs`); 0592 İşlemler'de girdiyi kaynağından okumak ve sonucu veritabanına yazmak (nesne girdisinin `scope: source`'u: bağlı katmanın bütün tablosu ya da `path` ve `table` ile bir GeoPackage'ın ya da PostGIS'in tablosu, isteğe bağlı `filter` ve `bbox`; kayıtlar `cad.sources.query`'nin okuduğu gibi parça parça çalıştırmanın kopyasına `sources_query::stage`, `kentos_processing::source_inputs`, en çok 1 000 000 `input_too_large`, yalnız `sink`'le; cevabın `inputs`'unda `table`, `rows`, dosyanın SHA-256'sı; parçalı bağlı katman `scope: layer` ile `partial_scope`; `sink.path` veritabanı adresiyse sonuç önce geçici GeoPackage'a, sonra `TableProvider::create_tables` ile tek işlemde yeni tablolara: şema `sink.schema` (yoksa açılır), `fid` anahtar, tipli geometri sütunu, 5000'lik gruplar, uzamsal dizin, `kentos_uids`, yorumu soy belgesi, `replaceTables` ya da `overwrite` aynı adı yeniden açar, plan yalnız adları sorar; `kentos_postgis::create`; masaüstünde İşlemler penceresinde Sonucun yeri (Çizime, GeoPackage dosyasına, PostGIS veritabanına) ve bağlı girdinin Bütün tablo (kaynaktan oku) anahtarı, çalıştırma iş parçacığında komutla `processing/sink.rs`); 0591 kaynakta sıralama ve sıralı sayfalar (`cad.sources.query`'nin `order`'ı: sayı, tarih ve evet/hayır sütunu kaynakta `ORDER BY … NULLS LAST, anahtar` ve anahtar kümesiyle (`gpkg::ScanOrder`, `ProviderScan.order`; PostgreSQL'de değer sütunun türüne `CAST`), metin sütunu ve çizimin katmanı burada öznitelik tablosunun kuralıyla, en çok 200 000 kayıt `order_too_large`; sıralı imleç `o:[yer,değer]`, imlecin kaydı silinse de yerinden sürer; `expectedStamp` dosya değişince `source_changed`; masaüstünde Öznitelik tablosunun Kaynağın tümü kipi: kaynağın satırları 500'lük sayfalarla, başlıkla sıralama, İfade süzgeci, Sonraki sayfa, salt okunur, `features/whole.rs`); 0590 PostGIS'te çoktan çoğa başvurunun bağlantı tablosu (`base_id`, `related_id`, `position`, iki yabancı anahtar `ON DELETE CASCADE`, yorumu `kentos_references base=… related=…`; `cad.sources.relate` veritabanında: check, map, unmap; başkasının bağlantı tablosu `foreign`; satırlar tabloların işleminde `TableProvider::edit_many_mapped` ve `MapWrite` ile, yeni kaydın satırı işlemin verdiği anahtarla; okuma `position` sırasıyla; sağlayıcının `relation`, `mapping_rows`, `map`, `unmap`'i, `kentos_postgis::junctions`; planda bağlantının sırası da değişikliktir; İlişkiler'in Veritabanında bölümü); 0589 kaynağa bağlı topoloji denetimi (salt okunur `cad.topology.check`: kuralların bulguları kalıcı anahtarlarıyla, bağlı parçanın bandındaki satırlar komşu (`outside`), yalnız dışarıdakilerin bulgusu bırakılır, banttan taşan nesne ya da okunamayan band denetimi eksik bırakır `topology_incomplete`, damga: çizimin sürümü, kuralların özeti, kaynakların sürümü ve bandı; düzeltmenin önizlemesi `preview`: ölçüler önce ve sonra, yazılacak tablo, dışarıdaki satırı değiştiren `fix_unavailable`; istisnanın geometri özeti `digest` (`.kcad` şema 99), nesneleri değişince düşer `exception_lapsed`; `cad.topology.exceptions` (mark, unmark; projenin ayarı); masaüstünde sekme aynı çekirdekle `topology_check::gather`, Düzelt ▾ önizlemeli); 0584 aynı kaynağa birlikte yazma ve yazmada Z ile M (`cad.sources.sync`'in `with`'i aynı PostGIS veritabanının katmanlarını tek işlemde yazar: tablolar adlarının sırasıyla kilitlenir, satırlar tabanlarına karşı denetlenir, biri reddederse hiçbiri, ret `layer`/`with` ve `uids`; `sources/database.rs`, `TableProvider::edit_many`, PostGIS'te `edit_in`; iki kaynak `transaction_split`; sütunun Z ve M bayrakları yazmadan önce `geopackage::dims_problem`: yasaklayana kotlu, isteyene kotsuz, hiçbirine kısmen kotlu nesne `geometry_mismatch`, nesne bekleyen değişiklik olarak sayılır; PostGIS'in tipsiz `geometry` sütunu isteğe bağlı; Kaynağa birlikte yaz aynı kaynağın değişen katmanlarını da toplar); 0581 kaynağı sorgula (`cad.sources.query`, salt okunur: GeoPackage tablosu, PostGIS tablosu ya da çizimin katmanı; kutu, yer, sütunlar ve süzgecin kaynağın denetleyebildiği kısmı kaynağa gider (`kentos_expression::pushdown`: kapsayan kural, SQLite'ta sayı sütunundaki metin de geçer ve tam sayılmaz), gelen her kayıt projenin sisteminde ifadeyle yeniden sınanır; anahtar sırasıyla sayfa ve imleç `k:`/`u:`, sayı, `kentos.statistics/1` istatistikleri, plan; `GeoPackage::scan`, sağlayıcının `describe` ve `scan`'i; parçalı bağda `partial_scope`; İstatistik'te Kapsam: Çalışma kopyası ya da Kaynağın tümü, iş parçacığında Durdur'la, `sources_query::ask_source`); 0531 alanın dışındaki komşularla yazma (`cad.sources.sync`'in `topology`'si `strict`/`loose`: alan sınırlı GeoPackage parçasının değişen ortak köşeleri, `Stored.outside`'daki komşular aynı dosya sürümünden kutuyla okunup `topology_edit::apply` ile izler, aynı SQLite işleminde; `followed`, `left`, `outside_neighbour_invalid`, `neighbours_left`, `topology_on_read`; masaüstünde Topoloji açıkken Kaynağa yaz ve Kaynağa yaz, komşular gevşek; `komsu.gpkg`); 0527 birleşik görünüm (projenin `joinViews`'u, `.kcad` şema 89; `cad.joins.edit`, `cad.joins.query`, `joins_query::read`: satırlar her okuyuşta çizimden, `sol/sağ` kimlikli, kaydı bir kez sayan toplamlar, kopuk görünüm; Öznitelik tablosunda ⋈ ve Birleşik görünümler penceresi); 0632 birleşik görünümde kaynak tablosu ve toplama sütunları (görünümün `source`'u: birleşen kayıtlar bir GeoPackage'ın ya da PostgreSQL'in tablosu, view'u ya da sorgu katmanı, her okuyuşta `sources_query::stage` ile bütünüyle, kopya yok, okunamayan kaynak görünümü kopuk yapar, kaynağın hücresi yazılmaz; `aggregates`: kayıt başına `count`, `sum`, `min`, `max`, `mean`, `kentos.statistics/1`, toplamlar kaydı bir kez sayar; `.kcad` şema 105; Dışa aktar ▾ `joins_query::sheet`; pencerede Birleşen: Katman | Kaynak tablosu ve Toplama sütunları); 0633 veri kümesi bilgisi ve soy (katmanın `dataset`'i: başlık, açıklama, sorumlu, lisans, kısıt, anahtar sözcükler, gün, ölçek, çözünürlük, doğruluk, tamlık, üretim; `cad.datasets.edit` tek adım; işlem aracının açtığı katmanın `derivation`'ı çalıştırıcının uygulama adımında, girdilerin özetleri yazmadan önce; salt okunur `cad.datasets.describe`: kapsam ve WGS 84 kutusu, sistem, sözlük, girdilerin durumu `current`/`changed`/`gone`, eski türev, `upstream`, `downstream`, OGC API Records kaydı; `.kcad` şema 106; GeoPackage'ta `https://kentos.app/dataset/1` ve `/layer-derivation/1`, içe aktarmada bilgi gelir; Katmanlar'da Veri kümesi bilgisi penceresi); 0526 olay kaymasını denetle (`linearRef.eventDrift`: olay rota kimliğiyle bağlı, rotanın bugünkü hâliyle yeniden yerleştirilir, `ops::lrs::line_drift` ve `point_drift`, Yerine taşı; arayüz kaydında ▾'e yalnız `overflow`); 0569 olaylar rotalarını kendiliğinden izler (projenin `linearEvents`'i, `.kcad` şema 97: bağ olay katmanını, rota katmanlarını ve olayın rota, ölçü ve öteleme özniteliklerini söyler; bağın katmanlarına dokunan her yazma, aynı adımda, olayları Olay kaymasını denetle'nin kuralıyla (`lrs_events`) rotanın verdiği yere koyar, koyamadığını olduğu yerde bırakıp `events_dirty` der; `cad.linearref.define`, Olay bağları penceresi, Python ve MCP); 0518 raster oturtmanın kaydı (rasterin `georef`'i: yöntem, kontrol ve denetim noktaları piksel
  köşeleriyle, hedeflerin sistemi, kaynak ve SHA-256, m0, denetim hataları, çıktı ızgarası; `.kcad` şema 88, `FORMATS_VERSION` 94;
  anlamsal `cad.rasters.georef`, `rasters_georef::prepare` ve `made`, `kentos_formats::raster::gcp`'nin `.points`'i; Raster oturt'ta
  Denetim, Python ve MCP); 0567 araç profilleri ve tarihli kapanmalar (ağın `profiles`'ı: kenar alanlarına sınırlar, yasaklanan değerler,
  en yüksek hız, iki yön; `closures`'ı: ilk ve son gün alanı; `.kcad` şema 95; graf bir kez kurulur, soru maliyetleri değiştirir
  `Graph::vary`, `NetworkSession::set_access`; Ağlar'da Araç profilleri ve Tarihli kapanma, araçlarda Profil (P) ve Tarih (T), İşlemler'de
  Araç profili ve Tarih, Python `Network.profile`, `.date`; bağımsız başvuru `network_access_cases.py`); 0568 kapanmanın haftanın günleri
  ve saatleri (`closures`'ın `days` ve `hours`'u, `.kcad` şema 96; saatli kapanma kalkış saatiyle: hız maliyetinin saati haftalık
  `Clock::week`, pencerede sıfır hız, öbür maliyetler sorunun anında; `NetworkSession::set_time`; Ağlar'da Gün ve saat; bağımsız başvuru
  `network_timed_closures_cases.py`); 0570 ulaşılamayanın nedeni (her kenarın açık olduğu en kısa yolu kapatan kenarlar nesneleri ve
  nedenleriyle, engeller, kopuk ağ; `ops::network::blocked`, `Refusal::Unreachable`'ın nedeni, `NetworkSession::blocked_between`;
  Maliyet matrisi ve En yakın tesis söyler; bağımsız başvuru `network_blocked_cases.py`); 0575 ulaşılamayanda en az kapatan (en az kapatanı olan, eşitlikte en kısa yol; yaylar üzerinde sözlük sıralı Dijkstra, kenar bir kez; söz “yolu açmak için en az kalkması gerekenler”); 0577 ulaşılamayanda düğüm, dönüş ve cihaz (arama düğüm geçişini de sayar: kapalı düğüm, yasak dönüş, cihaz; JSON'un `junctions`'ı ve `turns`'ü); 0521 ağın dönüşleri, seviyeleri ve zaman profili (`turns`: geri dönüş kuralı, kavşak gecikmeleri,
  dönüş katmanları; `levels`: köprü ve tünel; hızla maliyetin `periods`'ı, ilk giren ilk çıkar; `.kcad` şema 87,
  `Graph::build_with`, `NetworkSession.time`, `ops::network::{turns, clock}`; Ağlar penceresi, araçlarda Saat (Z), İşlemler'de
  Kalkış saati, Python `Network.time`); 0519 düşey sistem ve epok (projenin `heights`'i ve `coordinateEpoch`'u,
  `.kcad` şema 86, `FORMATS_VERSION` 93, `cad.crs.transform` kotlarıyla ve işlemin kaydıyla, Proje ayarları › Kotlar ve epok); 0517 sanal raster mozaiği (GDAL VRT'si `kentos_formats::raster::vrt`,
  `Reader::mosaic`, `cad.rasters.mosaic` gdalbuildvrt'nin kuralıyla, Sanal mozaik penceresi); 0520 permütasyon çıkarımı (`kentos.spatial-inference/1`, katlanmış sözde
  p, koşullu permütasyon), Gi*'ın FDR ve Bonferroni'si, Yerel Moran `stats.localMoran`, deney profili; 0566 En yakın komşu'nun Monte Carlo sınaması (`nearest_with`, `Simulation`) ve DBSCAN ile k-ortalamaların kararlılığı (Hennig'in clusterboot'u: `ops::spatial_stats::stability`, Bootstrap ve Alt örnek, kümenin Jaccard ortalaması; kopyalarda Kararlılık), esda, pointpats ve scikit-learn ile çapraz denetim; 0574 çalışma alanının alan nesnesinden verilmesi (En yakın komşu'da `region` ve `studyArea`, çekirdekte `nearest::nearest_in` ve `Window::Areas`: verilen alanların birleşimi, delikler çıkar, A net alan, bölge dışındaki nesneler alınmaz ve söylenir, Monte Carlo noktaları bölgede atılır) ve DBSCAN parametre taraması (`stats.dbscanSweep`, `sweep::dbscan_sweep`: en az nokta × yarıçap ızgarasında küme sayısı, gürültü ve kararlılık; k-uzaklık grafiğinin dirseğinden ve en az nokta 4'ten önerilen çift; R yok, fpc ile çapraz denetim yapılmadı); 0516 SLD alışverişi (`kentos_formats::sld`, SLD 1.1 / SE 1.1 iki yönde;
  `file.export.sld` ve `file.import.sld`, kayıplar söylenir; Katmanlar'ın sağ tıkında; QGIS 4.2'yle denetlenir); 0564 görünüş alanın adını izler (`field_refs` işleyicinin ifadelerini, kuralların süzgeçlerini ve simgelerin veriye bağlı
  değerlerini de yürür; `cad.fields.usage` sayar, yeniden adlandırma yazar); 0555 QML alışverişi (`kentos_formats::qml`, QGIS'in
  `renderer-v2`'si iki yönde: dört görünüş, dolgu, çizgi, işaret ve merkez dolgusu, birimleri ve saydamlığıyla; süzgeçler QGIS'in ifadesiyle;
  `file.export.qml` ve `file.import.qml`; Katmanlar'ın sağ tıkında; QGIS 4.2'nin `loadNamedStyle`'ıyla denetlenir); 0565 QML'de tarama (`LinePatternFill`), işaretli çizgi (`MarkerLine`) ve veriye bağlı kalınlık, boy ve dönüş (ifade alanlar, sayılar ve dört işlemse); 0571 stil alışverişinin kalanı (degrade: QML'in `GradientFill`'i, SLD ve MapLibre'de yok; nokta deseni: QML `PointPatternFill`, SLD `GraphicFill`; SVG işareti: çizim simgeye konur, QML `base64:`, SLD iki `ExternalGraphic`, okunan çizim projenin kitaplığına girer; yazı işareti: QML `FontMarker`, SLD `ttf://` + `MarkIndex`, MapLibre `symbol` katmanı; renklerin veriye bağlı değerleri: QML alan ve ifade, SLD `PropertyName`, `Recode` ve `Categorize`, MapLibre `to-color` + `get`; söylenemeyen adıyla söylenir, QGIS SLD'de alana bağlı rengi okumaz; `kentos_formats::style_extras`); 0560 MapLibre stili alışverişi (`kentos_formats::maplibre`, stil sürüm 8 iki yönde:
  kategorili `match`, aralıklı `step`, süzgeçli kurallar, ölçek aralığı zoom; fill, line, circle ve etiket symbol katmanları; mm 96 dpi'de piksel, metre
  zoom'la büyüyen genişlik; `file.export.maplibre` (görünen katmanlar tek stil, her katmanın WGS 84 GeoJSON'u yanında ya da `sourceUrl` ile vektör karo) ve
  `file.import.maplibre`; Katmanlar'ın sağ tıkında ve Veri › Dosya alışverişi'nde; MapLibre'nin resmî doğrulayıcısı ve ifade değerlendiricisiyle denetlenir); 0515 COG (`kentos_formats::raster::cog`: dizinler başta, küçük katlar
  önce, karolar bayt bayt; bütün raster sonuçları COG, `Sink::finish_cog`; COG olarak kaydet `raster.cog`; başsız raster dosyaları
  `local_files::LocalFiles`, raster araçları Python ve MCP'den); 0514 Shapefile dışa aktarma (`file.export.shp`: arşiv, klasör ya da tek
  `.shp`, bütün ya da hiç; katman ve tür başına, Z ve M, eğriler 0,1 mm kirişle, `.dbf`'in adları, türleri ve kod sayfası, ESRI
  `.prj`; kayıplar yazmadan önce; çekirdek `shp::write`, masaüstünde Shapefile dışa aktar…); 0513 servisten veri almanın bitişi (`feed::Ending`: complete, capped,
  short, pages; WFS 2.0 sunucunun kestiği sayfada sürer) ve OGC API'nin uygunluk profili (`profile::FeedProfile`; CQL2 metin
  süzgeci yalnız bildiren servise), Servisten veri al'da Uygunluk satırı; 0512 bağlı katmanda geniş değerler: eşitleme hücreleri sütunun okuma kuralıyla
  karşılaştırır (`geopackage::same_cell`; 2⁵³ üstü tam sayı, saat dilimi); 0511 işlem sonucunun soyu: girdilerin içerik özeti (`RunInput.digest`),
  `sink`'in tablolarında GeoPackage metadata uzantısıyla `kentos.lineage` 1 (`GeoPackage::add_metadata`); 0510 bağlı katmanın parçası: `cad.sources.edit`'in `extent`'i (alan
  sınırlı çalışma kopyası, `SourceLink.extent`; eşitleme alanın dışına dokunmaz, dışarı taşınan nesne kendi satırına),
  GeoPackage kutu sorgusu dizinsiz tabloda da süzer, `fids`; 0509 sağlayıcının ölçüm tabanı (`provider_timing`, 1 milyon parsel;
  `docs/perf/provider-2026-10-18.md`) ve bağlamada bellek (katmanın özeti ve dosyanın SHA-256'sı akışla, yeni katmana bağlamada
  satırlar bir kez); 0508 İşlemler'in araçları komut olarak: `cad.processing.tools` ve
  `cad.processing.run` (pencerenin çalıştırıcısı, açık değerler, nesneler kalıcı kimliklerle ya da girdinin `selection` ve
  `view`'u, plan kopyada; `sink` sonucu yeni bir GeoPackage'a yazar, çizim değişmez, soy `inputs`), ev sahipleri araçları
  kaydeder (`processing_host::register_tools`, `kentos_processing::command`);
  0507 toplu yazma: `cad.fields.write` (Alan hesapla: ifade, kapsam, boş
  sonuç; Yapıştır: satır başına değerler; değerler `cad.entities.set`'in kurallarıyla, tekil alan birlikte; bütün ya da hiç ya da
  uyanlar, plan önizleme ve sorunlar), Öznitelik tablosunda Alan hesapla… ve Yapıştır; 0506 taşınabilir proje: kaynağın göreli yolu (`SourceLink.relative`,
  kaydetmede yazılır, açılışta aynı içerikteki dosya bulunur, `source_places`), Kaynak ▸ Yeniden bağla, Paketle (`file.package`:
  proje ve `veri/` altında bağlı dosyaların kopyaları, bütün ya da hiç, SHA-256 denetimiyle; kalanlar `package_left`); 0505 PostGIS bağlı katmanı: `cad.sources.*` adresle, sağlayıcı
  `provider::TableProvider` ve kendi crate'i `kentos-postgis` (sqlx, TLS; masaüstü, Python ve MCP kaydeder), sürüm satırların
  SHA-256'sı, yazma kilit altında tek işlemde, `kentos_uids`, parolasız adres, masaüstünde PostGIS'e bağlan; 0504 kaynağın sistemi: bağlı katmanda `reproject` (tablo kendi sisteminde,
  yazma ona geri çevirerek), çekirdeğin `crs::transform_bounds`'u (sıklaştırma, antimeridyen); 0503 bağlı katman: GeoPackage tablosunun çalışma kopyası, projenin `sources`'ı
  (`.kcad` şema 74; dosyanın SHA-256'sı ve katmanın özeti), `cad.sources.describe`, `.edit`, `.sync` (tek SQLite işleminde yazma,
  `source_changed`, `pending_changes`), masaüstünde Bağlı katman olarak aç, rozet ve Kaynak ▸; 0502 GeoPackage: kendi crate'i `kentos-geopackage` (rusqlite, gömülü SQLite), `file.import.gpkg`,
  `file.export.gpkg`, masaüstünde içe aktarma penceresi; 0501 standart ilişkiler: kesin DE-9IM `ops::de9im`, on yüklem ve desen,
  `cad.entities.relate`; 0500 bindirmenin doğruluk korpusu `fixtures/overlay/v1`, GEO-01 kapandı);
  görünümler: adlı görünümler projenin ayarı (`.kcad` şema 52 `namedViews`, `cad.views.edit`: keep, rename, remove; Görünümler penceresi;
  Python, MCP), Görünümü döndür (iki nokta ya da açı, UCS'e göre, Kuzey yukarı; kamera dönüşü `crates/render/wgpu` kamerasında, dönmüş
  görünümde seçim kutusu çokgen sorgusu), Görünümü böl (yan yana iki model görünümü, etkin olan basmayla değişir) (ADR 0285);
  çoklu çizginin yay ikilileri ve kenar düzenlemeleri: Çoklu çizgi'de PLINE'ın Açı+Merkez/Yarıçap, Merkez+Açı/Uzunluk ve Yarıçap+Açı yayları,
  yay kipinde Genişlik (E) ve teğet yayla Kapat (K), kenar başına başlangıç ve bitiş genişliği; `cad.entities.vertex`'in Yaya dönüştür'ü
  noktadan (`through`) ya da yarıçapla (`radius`), Kenar genişliği (`width`; `cad.entities.edit`'in `EdgeWidth`'i); çekirdeğin `bulge_via`,
  `bulge_of_radius`'u; tutamaç menüsünde Noktadan ya da yarıçapla yay… ve Kenar genişliği… (Kenar düzenle) (ADR 0288);
  dikdörtgen ve düzgün çokgen komutları: `cad.rectangle.create` (iki köşe, boyut, alan ya da kenar ve en; dönüş; keskin, yuvarlak, pah)
  ve `cad.polygon.regular` (çember içinde, dışında, kenardan) halkayı çekirdekle hesaplar ve `cad.polygon.create`'in yolundan yazar;
  araçlar önizlemeyi komutun hesabından (`shapes::rectangle_ring`, `regular_ring`) alır, Çakışma ve Topoloji alanı değiştirince alanın
  yolundan yazar (`points::write_area_by`) (ADR 0290);
  eğri yapımı ve yöntemlerin doğrulanması: salt okunur `cad.curves.construct` (iki ve üç noktadan, TTY ve TTT daire; yayın on bir yöntemi ve
  Devam; eksen uçlarından ya da merkezden elips ve eliptik yay), Yay, Daire ve Elips eğriyi onun hesabından alır ve reddini söyler;
  bağımsız başvuru `curve_method_cases.py`, ortak durumlarda `within` (ADR 0289);
  dönüşüm kuralları: Taşı ve Kopyala'da Yer değiştirme (Y ya da temel noktada Enter; `Memory::last_displacement`), Kopyala'da Kip
  (`Memory::copy_single`) ve Dizi (adet, Sığdır; `cad.entities.array`'in `line`'ı), Döndür'de projenin açı düzeni
  (`Format::turn_from_typed`, `direction_from_typed`: CAD derece saat yönü tersine, CBS grad saat yönünde) ve Noktalar, Ölçekle'de
  Noktalar; her türün dönüşümü bağımsız başvurudan (`transform_kinds_cases.py`), dolu tarama deseniyle kalır (ADR 0291);
  aynada yazılar: `cad.entities.transform`'un aynasının `text`'i (readable varsayılan, mirrored; MIRRTEXT), çekirdeğin `TextPolicy`'si ve
  `text_frame`'i: okunur yazı aynadaki yerinde, baş aşağı olmadan, hizası değişerek; bloğun yazıları ve öznitelikleri her zaman okunur,
  değerleri değişmez; Aynala ve tutamaç kipinde Yazılar (Y, `Memory::mirror_text_mirrored`); web'in çağrısı eski kuralda (ADR 0292);
  Ötele'nin kesinliği ve raporları: delikli ve çok parçalı alanın delikleri öbür yana, fit-point eğrisi ve NURBS kirişleriyle (yaklaşık,
  `approximate_offset`), yarıçap ve alan genişliğinin çökmesi `radius_collapse` (tersine dönen halka yazılmaz), kendini kesen kopya
  `self_intersecting`; çekirdeğin `offset_noted`'ı, `offset_entity` web'in kayıtlarıyla aynı (ADR 0293);
  Buda ve Uzat'ta AutoCAD'in seçenekleri: Çit (`fence`) ve Kesişen (`crossing`) komutun girdisi, Uzantılar (`edgeMode`: çizgi doğrusuyla,
  yay çemberiyle, açık çoklu çizginin uç kenarları), hızlı kipte kesilmeyen nesnenin silinmesi (`eraseUncut`), kalan parçanın bütünüyle
  gitmesi; araçta K, boş yere tıklama, U ve Sil (R); tutamaç kipinin Döndür'ü projenin açı düzeniyle (ADR 0294);
  Köşe yuvarla ve Pah'ta iki nesne: çizgi ya da açık çoklu çizginin uç kenarı (`corner::side_of`, `lines_corner`), aynı yolun iki ucu
  yolu kapatır; Tüm köşeler uçları buluşan çizgilerde de (`corner::line_corners`), araçlar aynı işlevlerle (ADR 0295);
  Kır, Birleştir ve Yönü çevir'de taşınanlar: parçalar katmanı, görünüşü, öznitelikleri, kotları, ölçüleri, genişlikleri ve ekleri taşır,
  ilişkili ölçüler, bağlı yazılar ve taramalar izler ya da nedeniyle kopar; Yönü çevir `cad.entities.reverse` (ADR 0296);
  Esnet'te yalnız penceredeki köşeler: yayın köşeleri uçlarıdır (bir ucu taşınan yay kiriş yüksekliğini korur), bağlı olduğu nesneyi
  izleyen nesne (ilişkili tarama, bağlı yazı, ölçünün bağlı noktası) kendisi esnemez, nesnesini izler (`follows_object`); plan ve sonuç
  izleyenleri verir (`dependents`, belgenin `followers`'ı bir kopyada), Esnet önizlemesinde noktalı (ADR 0297);
  anlamsal Uzat-kısalt: `cad.entities.lengthen` (uç, Fark, Yüzde, Toplam ve Dinamik; yayda açıyla), elips yayı kendi elipsinde,
  uzayan ucun kotu ve ölçüsü uç kenar boyunca sürer; araç komutun planıyla önizler, Fark ve Toplam'da `<açı` (`Tool::takes_angle`) (ADR 0298);
  Böl ve Ölç'te blok: `cad.entities.divide`'ın `block`'u (adıyla, nesneye hizalı, ölçek ve dönüş), aralığın ilk uzaklığı `start`, açık
  nesnenin uçları `ends`; elips yayı gerçek yay uzunluğuyla (`division_places`); Böl'de Blok, Nokta, Hizala, Uçlar ve İlk (ADR 0299);
  ilişkili dizi: belgenin `arrays`'i (kaynak blok “Dizi n”, ilk yer, dizi komutunun yerleşimi, yer başına öğe: yerleştirmenin kimliği,
  elle silinince boş, elle değişince `kept`; `.kcad` şema 53, `FORMATS_VERSION` 63), `cad.entities.array`'in `associative`'i; adımın
  sonunda silinen öğe boşalır, değişen korunur, yolu değişen yol dizisi izler (`follow_arrays`); veritabanı projesinde meta veri (göç
  0019, `ProjectPatch.arrays`, görüntü ve açılış `arrays_kept`'le), başkasınınki `ExternalMeta.arrays` ile (ADR 0300);
  Diziyi düzenle: `cad.arrays.edit` (yerleşim: öğe numarasını, ızgarada satır ve sütununu korur, yeni yerlere yerleştirme, kalkanlar
  silinir; ilk yer; Değiştir; Sil; Sıfırla), `array_not_found`, `invalid_item`, `item_removed`; araç `tool.arrayEdit` öğeye tıklanınca
  dizinin seçenekleriyle, Taban'da plandan önizleme; durumlarda `$array:k` (ADR 0301);
  Dizi, Kutupsal dizi ve Yol boyunca dizi'de İlişkili (İ; kapalı başlar, `Memory::array_associative`), anlamsal Patlat
  `cad.entities.explode` (parçalar çekirdekten, bloğun nesneleri kendi katman ve görünüşleriyle, ilişkili dizinin öğesi bütün diziyi
  patlatır ve dizi kalkar, `not_explodable`); uygulama katmanı `project`'in `Format`'ını kullanır (ADR 0302);
  Eğriyi düzenle: `cad.curves.edit` (çoklu çizgide Kapat, Aç, Düzleştir, Eğriye çevir; geçiş noktalı eğride nokta ekle, taşı, sil,
  uç teğetleri; her eğride denetim noktası taşı, ağırlık, sil, düğüm ekle, kapat ve aç, toleransla çoklu çizgiye çevir; çekirdeğin
  `through_tangents`'ı), araç `tool.curveEdit` (ADR 0303);
  Özellik kopyala: `cad.entities.match` (kaynağın seçilen özellikleri: katman, renk, çizgi tipi ve ölçeği, kalınlık, saydamlık, sembol;
  aynı türe yazı, ölçü, tarama ve kılavuz özellikleri), istemde on anahtar (`Memory::match_off`) (ADR 0304);
  Çizimi temizle: `cad.entities.cleanup` (tolerans, yok sayılan özellikler; boş ve yinelenen nesneler, eş doğrultulu çizgiler, yolun
  gereksiz köşeleri; rapor kayıpla), çekirdek `ops::overkill`, aracın beş seçeneği (ADR 0305);
  Denetle: `cad.drawing.audit` (nesnesi olmayan bağlı yazı, ilişkili tarama ve ölçü bağı, projede olmayan stil; `fix` tek adımda onarır,
  kilitli katmandaki söylenir; kullanılmayan katman, grup, blok ve stiller raporda, `purge` stilleri temizler), kurallar
  `kentos_domain::audit`, araç `tool.audit` (ADR 0306);
  delikli ve çok parçalı nesnelerde düzenlemeler: envanter; Kır, Buda ve Uzat çok parçalı çizgide tıklanan parçada, parçalar aynı
  nesnede kalır (`ops::parts::part_near`, `polyline_of_pieces`; Buda'nın parça başına “kesildi” bayrağı) (ADR 0307);
  Patlat'ta eğriler: daire, yay, elips, eğri ve NURBS köşeleri üstünde, eşit uzunlukta bölümlü çoklu çizgi; `density` (count, step,
  deviation; varsayılan 0,01 m sapma), `invalid_density`; araç eğri seçilince açık kalır, Sayı (S), Aralık (A), Sapma (P), önizlemede
  köşeler; CAD ve CBS (ADR 0308);
  pano komutları: `cad.entities.copy` (salt okunur kopya: nesneler çizim sırasıyla, katman yolları, bloklar, adı geçen stiller,
  kitaplık, birim, taban noktası; aralarındaki bağlar kalır) ve `.paste` (katman yoluna, olmayan yol açılır; aynı adlı blok ve stil bu
  çizimin; kilitli katman bütün yapıştırmayı reddeder, gizli ve başka birim söylenir; `asBlock` tek blok “Pano”), Taban noktasıyla
  kopyala (`tool.copyBase`), Blok olarak yapıştır (`edit.pasteBlock`); `Document::to_snapshot_v2_of`, katman ve blok yazma
  `exchange_write.rs`'te (ADR 0309);
  çizim sırası ve maske: `cad.entities.order`'ın `above` ve `under`'ı (`reference`; başvurunun öbür yanı yer açmak için aynı adımla
  kayar, başka katman `not_same_layer`), Nesnenin üstüne ve altına araçları, Yazıları öne getir ve Taramaları arkaya gönder (seçim ve
  sıra); alanın maskesi `ObjectLook.wipeout` (`{ frame }`, yalnız alanda; `.kcad` şema 54, `FORMATS_VERSION` 64): zemin renginde opak,
  kendinden önce çizilenleri örter (maskeli katmanda her maske kendi bandı), yalnız sınırından seçilir (`Store::set_masks`); Maske aracı
  (yol aracının biçimi; Çerçeve, Nesne), `cad.polygon.create`'in ve `cad.entities.set`'in `wipeout`'u, Öznitelikler'de Maske; DXF'in
  WIPEOUT'u ertelendi (ADR 0310);
  katmanlar: dondurma (`frozen`, `.kcad` şema 55: çizilmez, seçilmez, kenetlenmez, kapsama ve paftaya girmez, Hepsini göster çözmez,
  etkin katman dondurulmaz; DXF'in 70 grubunun 1. biti), `cad.layers.set`'in `frozen`, `thawAll` ve `unisolate`'i (Yalıt'ın gizledikleri
  belgede, oturumun), durumların dondurma ve yazdırması, `export` ve `import`'u (`kentos.layer-states`, düğümler yollarıyla), salt okunur
  `cad.layers.find` (AutoCAD'in `wcmatch` örüntüsü ve özellikler) ve `cad.layers.translate` (LAYTRANS, plan önizleme); Katmanlar'da
  Dondur / Çöz, kar tanesi, örüntülü arama ve Süzgeç ▾, Katmanı dondur, Bütün katmanları çöz, Katmanları çevir penceresi (ADR 0311);
  çizim standardı: standart veri olarak (katmanlar adları, görünüşleri, Yazdır'ları ve topladıkları ad örüntüleriyle, ad örüntüleri,
  yazı tipleri, yazı ve ölçü stilleri, çizgi tipleri, katmana göre), yerleşik `mimari-ofis` (`catalog/v1/standards/`), salt okunur
  `cad.standards.check` ve ayrı, planlı `cad.standards.apply` (birleştir, ekle, düzelt, stiller), Çizim standardı penceresi (CAD Yönet,
  CBS Veri › Standart); görünüşün kaynağı tek kural (`resolve_look`: kendi, blokta katmana göre ya da yerleştirmeden BYBLOCK, çizimde
  katmandan), blok parçası çizgi tipini, ölçeğini ve saydamlığını taşır, DXF'te tanımın BYBLOCK ve BYLAYER çizgi tipi ve saydamlığı;
  kâğıdın renkleri tek tablo (`print_color`, `paper`), çıktılar temayı okumaz (ADR 0312);
  çizgi tipleri: projenin LIN desenli tipleri (`lineTypes`, öğeler çizgi, boşluk, nokta, yazı ve şekil; birim kâğıt ya da çizim;
  kaynak) ve çizgi tipi ölçeği (`lineTypeScale`, LTSCALE), katmanın ve nesnenin `linePattern`'ı, `lineType` en yakın yerleşik tip
  (`.kcad` şema 56, `FORMATS_VERSION` 65); LIN okuyucu ve yazıcı `kentos_formats::lin`, `cad.linetypes.edit` (load, add, remove, scale,
  export), DXF LTYPE desenleri, `$LTSCALE` ve PLINEGEN iki yönde; desen kâğıtta mm, çizim deseni çizim ölçeğiyle, × LTSCALE × nesnenin
  ölçeği, paftada kâğıtta aynı; faz yol boyunca sürer, en çok 8 sayı; yazı ve şekil saklanır, boşluk çizilir ve söylenir; Çizgi tipleri
  penceresi (Yönet ve Veri › Standart) (ADR 0313);
  haritanın katmanları, değişikliğin etkisi ve kitaplık: paftanın haritasında katman durumu ve katman başına geçersiz kılma
  (`layerOverrides`: dondur, yazdır, renk, çizgi tipi ya da desen, kalınlık; tek kural `kentos_plot::layers::map_layers`, model değişmez,
  `cad.sheets.edit` ile), değişikliğin etkisi tablosu `CHANGE_EFFECTS` (adım, adımın dışında ya da tercih; kirlilik; parçalar ve bulutta
  gidişleri; kataloğun `effects`'i, `change_effects.rs` komutlarla sınar), `cad.library.edit` (projenin kitaplığı: oluştur, güncelle,
  kopyala, ad, taşı, sil, `.kstil` al ve ver, sembol ver; kurallar `kentos-style-library`'de), `cad.blocks.edit`'in `duplicate`,
  `export` ve `import`'u (`kentos.blocks`, `.kblok`; kanonik), salt okunur `cad.styles.impact` (izleyenler, kendi değerini koruyanlar,
  kilitliler, blok parçaları ve yerleştirmeler, durumlar, haritalar, dış başvuru 0); Değişikliğin etkisi penceresi, Bloklar panelinde
  Kopyasını oluştur, Dışa aktar… ve Blokları al… (ADR 0314);
  blokta kararlı kimlik, katman kuralı, X/Y ölçeği ve birim: blok içi yol tanım değişince parçanın yerel kimliğiyle izlenir
  (`Blocks::path_in`, `follow_path`); 0'daki (`""`) parçanın `byLayer`'ı yerleştirmenin katmanından, kendi katmanındaki parça o
  katman görünürken görünür (`Piece::gates`, deponun `Item::veiled`'ı), Blok oluştur parçaları çizildiği gibi `byLayer`'la kopyalar,
  DXF'in BYLAYER'ı `byLayer` kalır; yerleştirmenin `yScale`'i (afin bükmeyle: daire ve yay kesin elips, yolun yayları 0,1 mm; Patlat
  eğilen iç bloğu reddeder; DXF 41/42) ve tanımın `units`'i (birimli gerçek boyutlu, birimsiz çizim birimiyle; `BLOCK_RECORD` 70);
  `.kcad` şema 57, `FORMATS_VERSION` 66; anlamsal `cad.blocks.insert` (Blok ekle, Y ölçeği), `cad.blocks.edit`'in `units`'i,
  Öznitelikler'de X/Y ölçeği, Bloklar panelinde Birim ▸ (ADR 0315);
  Blok düzenleyici (`block.edit`: tanımın nesneleri yerel kimlikleriyle ayrı çizimde, “0 (blok)” katmanı, Kaydet `cad.blocks.edit`'in
  `update`'iyle tek adım, Vazgeç), `file.export.block` (WBLOCK; taban başlangıçta, aynı blok aynı bayt) ve `file.import.block`
  (`drawing`: dosya tek blok, `blocks`: adlarıyla, `replace` yeniden tanımlar; çakışan kimlik yenilenir, resim ve tablo
  `objects_left_out`); Bloklar panelinde Dosyaya yaz… ve Başka çizimden yeniden tanımla…, Dosyadan blok ekle komutla; iç içe
  bloğun bir milyon parçalık sınırı `block_too_large` bütün okuyucularda ve sunucuda (ADR 0316);
  blok öznitelikleri ve kartlar: özniteliğin seçenekleri (görünmez, sabit, doğrula, ön ayarlı, taşınabilir, çok satırlı ve kutu
  genişliği, alan şablonu `{NO}/{TUR}`), yerleştirmenin `attributePlaces`'i, bloğun kartı (kategori yolu, etiketler, sürüm; `.kcad`
  şema 58, `FORMATS_VERSION` 67); `cad.blocks.edit`'in `sync` (ATTSYNC; `attributes` ve `update` de eşitler, rapor `AttributeSync`),
  `fields`, `place` ve `card`'ı, `cad.blocks.replace` (BLOCKREPLACE, `purge`) ve salt okunur `cad.blocks.query` (`count`, `instances`,
  `schedule`, `where`); Blok öznitelikleri'nde seçenekler, Öznitelik değerleri'nde doğrulama ve çok satır, Blok kartı, Blok çizelgesi
  (`block.schedule`), Bloklar panelinde Liste/Kartlar ve kategori; DXF ATTDEF'in 70'i (ADR 0317);
  dinamik bloklar: tanımın `dynamic`'i (doğrusal, kutupsal, açı, nokta, çevir, görünürlük, tablo parametreleri; taşı, esnet, ölçekle,
  dizi, döndür, aynala eylemleri; sayı kuralı ve `@ad` ifadesi; `.kcad` şema 59, `FORMATS_VERSION` 68), yerleştirmenin `parameters`'ı
  (yalnız varsayılandan farklılar), çekirdeğin varyantı (`block_dynamic::vary`, önbellek, iç içe), `cad.blocks.parameters` (get, set,
  reset), `cad.blocks.edit`'in `dynamic`'i, Blok ekle'nin değerleri; KentOS'un 16 bloklu kitaplığı (`catalog/v1/blocks/kentos-temel.kblok`,
  `import` `name: kentos-temel`, KentOS blok kitaplığı) (ADR 0318);
  dinamik bloğun tutamaçları ve değerleri: seçili dinamik yerleştirmenin parametre tutamaçları (doğrusal, kutupsal, açı, nokta, çevir;
  çekirdeğin `Dynamic::grips`, `value_at`, `Rule::nearest`; `Store::param_grips`, `param_value`, `varied_outlines`), çekilen değer
  kuralına uyar ve `cad.blocks.parameters` ile yazılır, açık çevirmeler tutamaçları aynalar (`kentos_interaction::param_grip`);
  Öznitelikler'de Blok parametreleri; gizli parça her yerde gizli; salt okunur `cad.blocks.proxy` (yerleştirmenin çizdiği düz
  nesneler, `vary_traced`) ve `cad.blocks.freeze` (Sabit bloğa çevir); DXF'e değerli yerleştirme çizdiğinin düz bloğu “Ad V1”
  olarak, raporda söylenerek (`block_variants::plain_for_export`, `DxfWriteInput.notes`); AutoCAD'in dinamik blok verisi taşınmaz
  ve gerçek korpusla sınanmadı (ADR 0319);
  her türün ilişkili ölçüsü: ölçünün başvurularının kuralı (`refs.rule`: `curve` yarıçap, çap, kırıklı yarıçap, yay uzunluğu ve
  yaydan açının merkezi eğrisinden, yarıçapın noktası yerinden dönülmüş; `edges` kenarlardan açının köşesi iki kenarın kesişimi,
  kollar boylarıyla; `.kcad` şema 60, `FORMATS_VERSION` 69), köşeden açının yerleri, Buda'da bağlar (parçalarda aranır, kesilen uç
  yeni uca), `cad.dimensions.create`'in `continue` ve `baseline`'ı (Zincir ve Baz araçları onunla, ilk noktası temelinkini izler),
  seçili ölçünün bağ vurgusu (`kentos_interaction::dimension_bonds`) (ADR 0320);
  ölçü aralığı, kırma ve değerin yazımı: ölçünün ve ölçü stilinin `options`'ı (sığdırma, yuvarlama, sıfırlar, ayak-inç, alternatif birim,
  tolerans, kırma genişliği), ölçünün `textShift` ve `breaks`'i (`.kcad` şema 61, `FORMATS_VERSION` 70), tek yazım kuralı
  `Format::dimension_value` (`<>` ölçülen değer; `<>`'siz yazı elle), `cad.dimensions.edit` (space, break, unbreak, moveText, text,
  options) ve salt okunur `cad.dimensions.query` (measured, shown, overridden, bond); şeritte Ölçü aralığı, Ölçüyü kır, Kırmayı kaldır,
  Öznitelikler'de Ölçülen değer ve seçenekler (ADR 0321);
  merkez işareti, eksen çizgisi ve kot: ölçünün üç yeni türü (`centerMark` eğrinin `curve` kuralıyla, `centerLine` iki düz kenarın yeni
  `pair` kuralıyla, `elevation` noktası ve köşesinin kotuyla; `.kcad` şema 62, `FORMATS_VERSION` 71), `cad.dimensions.create`'in
  `centerMark`, `centerLine` ve `elevation`'ı, araçlar `kentos_interaction::dimension_marks`; `cad.styles.edit`'in `export` ve `import`'u
  (`kentos.annotation-styles`, stil pencerelerinde Dışa aktar… ve Al…); yazının ve ölçünün stilden ayrılan alanları `overrides`
  (adımın elle değiştirdikleri, `kentos_domain::style_overrides`; stil değişince korunur; `.kcad` şema 63, `FORMATS_VERSION` 72),
  Öznitelikler'de Stilden ayrılan ve Stile döndür (ADR 0322);
  ölçekli açıklamalar: yazının, ölçünün ve kılavuzun `annotative`'i (kendi ölçeği `at`, diğer ölçekler ve her birinin ayrı yeri,
  doğrultusu, ölçünün uzaklığı ve değerinin yeri; `.kcad` şema 64, `FORMATS_VERSION` 73), temsil kuralı sözleşmede (`at_scale`,
  `rebase_annotative`), paftanın haritası ölçeğindeki kopyayı çizer (`Document::at_scale`, masaüstünün pafta kipi ve başsız PDF), çizim
  ölçeği değişince ölçekli nesne yeni ölçeğe taşınır (açık göç: eskiler değişmez); `cad.annotations.scales` (make, unmake, add, remove,
  set, reset), Öznitelikler'de Ölçekli, Kendi ölçeği, Diğer ölçekler, Ayrı değerler; CAD › Açıklama › Ölçek ve Ölçekteki yeri aracı
  (`kentos_interaction::annotative`); kâğıttaki ölçünün kuralı (ölçek bir kez) (ADR 0323);
  çok satırlı yazının düzeni ve yazı tipleri: yazının `layout`'u (girinti, sol girinti, sekme durakları yükseklik cinsinden, kutulu
  yazıda sütunlar; `.kcad` şema 65, `FORMATS_VERSION` 74) ve dilimin `stack` kesri (`/` çizgili, `^` çizgisiz, `#` yan yana); çekirdeğin
  yerleşimi sekmelerde parçalara ayrılır, asılı ilk satırın sol girintisi de duraktır; Öznitelikler'de Girinti, Sol girinti, Sekmeler,
  Sütunlar, düzenleyicide Kesir ve Liste ▾ (`list_lines`); MTEXT `\p`, `^I` ve `\S` iki yönde, sütunlar KENTOS verisinde; tek yazı tipi
  çözüm kuralı (`text::fonts::resolve`: kendisi, eşdeğeri, SHX, yok), salt okunur `cad.fonts.report` ve Yazı tipleri penceresi, stilin
  yerine çizilen yazı tipi `font_substituted`; paftanın PDF'i yazının genişlik çarpanını ve yatıklığını yazar (`MapText`) (ADR 0324);
  stili projeye gömülü `.shx` (ve Big Font) adlandıran yazı nesnesi fontun çizgileriyle çizilir (ekran, PDF, CPU;
  `shx_text`), çizilemeyen `text_shx_unsupported` hatası ve PDF reddi, Unicode → Big Font çevrilmez (kod sayfası yok),
  `cad.fonts.report`'ta `embedded`, DXF/DWG aktarımı gömülü yazı fontlarını yanına yazar; çizgi tipinin noktası çizgi
  kalınlığında yuvarlak basılır (ADR 1124, 1125);
  alanlar ve yazının komutları: yazının `fields`'ı (harflerden bir aralık; kaynağı nesnenin alanı, çevresi, uzunluğu, katmanı, etiketi,
  özniteliği, yazısı ya da yeri, ya da projenin adı, ölçeği, birimi, sistemi, değişkeni; biçimi; `.kcad` şema 66, `FORMATS_VERSION` 75),
  belge kayıttan önce yeniler (`kentos_domain::text_fields`: kaynağı değişen, elle düzenlenen, döngü ve kopuk `####`), `cad.fields.edit`
  (insert, remove, format, update), `cad.texts.edit` (replace, number, combine, split; Bul ve değiştir onunla), salt okunur
  `cad.texts.check`; Alan ekle, Alanları yenile, Numarala, Birleştir, Satırlara ayır, Yazı denetimi, Öznitelikler'in Alan satırları (ADR 0325);
  çok kollu kılavuz: kılavuzun `arms`'ı (ek kollar ortak kolun başına), `blocks`'u (blok içerik: kolun ucunda sıra hâlinde duran
  yerleştirmeler), uçların `tip` bağı ve `leaderStyle`'ı, projenin `leaderStyles`'ı (`.kcad` şema 67, `FORMATS_VERSION` 76); belge
  kayıttan önce uçları ve blokları izler (`kentos_domain::leader_ties`); `cad.leaders.edit` (addArm, removeArm, align, collect,
  content, blocks, look, style, bind), `cad.styles.edit`'in `leader` türü; Kol ekle, Kol sil, Hizala, Topla, Blok içerik, Kılavuz
  stilleri penceresi, Öznitelikler'de kollar ve uç bağları; DXF'in MULTILEADER kolları ve LEADER olarak kollar (ADR 0326);
  tablonun hücreleri, bağlı çizelge ve DXF biçimleri: tablonun `styles`'ı (aralıkların hizası, kalını, rengi, dolgusu, basamağı;
  sonraki öncekini ezer), `formulas`'ı (`=` ile; A1 başvuruları ve aralıklar, TOPLA … YUVARLA Türkçe ve İngilizce adlarıyla,
  `#DEĞER!` … `#DÖNGÜ!`, satır ve sütunla kayan başvurular; `kentos_contracts::table_formula`) ve `fields`'ı (ADR 0325'in kaynakları,
  Blok sayısı `blockCount`); blok çizelgesi (`TableSource::Blocks`); belge hesaplanan tabloları kayıttan önce yazar
  (`kentos_domain::table_cells`); dosya kaynağının `path`, `digest`, gömülü `copy` ve `overrides`'ı (keep, replace), Tabloyu
  güncelle'nin raporu, Dosyaya bağla; `.kcad` şema 68, `FORMATS_VERSION` 77, paketli kaydın 25. türü; `cad.tables.edit` (edit,
  refresh, link, unlink); Tabloyu düzenle'nin ikinci araç satırı ve biçimli ızgarası; DXF dışa aktar'da Ölçüler ve Kılavuzlar
  Yerel nesne ya da Blok görünüşü ve KentOS verisini yaz (`DxfWriteInput.annotations`; ACAD_TABLE ve MULTILEADER yazılmaz)
  (ADR 0327);
  taramanın kaynak sınırı: `boundary` (halkalar: nokta, büküm, elips yayı ve NURBS kenarları; `ring`/`holes` ondan çizilir,
  elle değişen halka sınırı düşürür; `.kcad` şema 69, `FORMATS_VERSION` 78), bölgeden kapalı eğriler geri bulunur
  (`geom::hatch_boundary`), dönüşümler, Esnet ve tutamaçlar taşır, DXF HATCH'in kenarlarıyla iki yönde; çizgilerle kapanan yüzün
  bağı (`bounds`; belge yüzü aynı adımda yeniden keser, bütünüyle taşınan yüz tohumunu götürür), Tarama'nın çizgi yolunda
  İlişkili (ADR 0328);
  taramanın bölgeleri ve düzenlenmesi: `cad.entities.hatch`'in `islandStyle`'ı (Dış, İç içe: adaların içindekiler sırayla ayrı
  taramalar, Yok say), `alsoAt` (çoklu bölge) ve `gap` (çizgi işinin boşta uçları en yakın çizgiye köprülenir, `gaps_closed` sayı,
  uzunluk ve alanla; bağda `gap`, `.kcad` şema 70, `FORMATS_VERSION` 79); `cad.hatches.edit` (Sınırı yeniden oluştur: alan ya da
  çoklu çizgi, bağlı; Yeniden seç; Ada ekle ve çıkar; İlişkiyi kopar), bağı değişen tarama yeniden kesilir
  (`Document::recut_hatch`); Taramayı düzenle aracı, Tarama'nın Boşluk (G) ve Adalar'ın iç içe değeri, Öznitelikler'de Sınır
  (ADR 0329);
  tarama desenleri, başlangıç ve ölçekli tarama: projenin desenleri (`hatchPatterns`: ad, birim mm, inç ya da çizim, PAT aileleri;
  kitaplığınkinden önce), PAT okuyucu ve yazıcı `kentos_formats::pat`, `cad.hatchpatterns.edit` (yükle, ekle, sil, dışa aktar;
  ayar), Tarama desenleri penceresi; desenin `origin`'i (çizgiler ona bağlanır, dönüşümler taşır), desen spec'inin `origin`'i
  (dünya, nokta, köşe, ortak kutu), `basis` (kâğıtta ya da çizimde) ve `annotative`'i; ölçekli tarama (`HatchEntity.annotative`,
  çizim ölçeğiyle taşınır); `cad.hatches.edit`'in Deseni değiştir ve Taramaları hizala'sı; Tarama'nın Ölçek tabanı (E),
  Başlangıç (S), Ölçekli (L), Özellik al (Z), Taramayı düzenle'nin Hizala (H)'sı; `.kcad` şema 71, `FORMATS_VERSION` 80
  (ADR 0331);
  taramanın saydamlığı, sırası, boş bıraktığı yazılar ve bütçesi: `cad.entities.hatch`'in `transparency`'si ve `drawOrder`'ı
  (none, back, front, behindBoundary, aboveBoundary; katmanın içinde, sınır başka katmandaysa `order_other_layer`), `NewObject`'in
  `transparency` ve `order`'ı, Tarama'nın Saydamlık (M) ve Sıra (O); yeniden kesilen taramanın bağlı yazıları aynı adımda turlarla,
  kendi etiketi boş bırakılmaz, `cutout_gone` ve `hatch_text_cycle` uyarıları, Denetle'de `missingCutout` ve `hatchTextCycle`; komutun
  toplam 200 000 çizgilik bütçesi ve işte iptal (`Stop::Cancelled`), ekranın tonu paftayı değiştirmez (ADR 0332);
  taramanın PDF'i ve DXF'te ilişkisi: degrade paftanın PDF'inde gölgelendiricinin kuralıyla vektör bantlar (doğrusal ve silindir
  şeritler, küre dışbükey dilimler; `kentos_plot::vectors::gradient`); DXF'in ilişkili taraması kaynaklarıyla (97, 330'lar),
  tohumuyla (98) gelir: okuyucu yer tutucu kimliklerle bağlar (`import_placeholder`), içe aktarma aynı adımda gerçek kimlikleri
  yazar (`tie_placeholders`); `DxfWriteInput.uids` ile `71 1`, yolların kaynakları ve nesnelerde `{ACAD_REACTORS` yazılır
  (`writer::hatch_ties`); belgenin tarama izleyicisi kapalı çoklu çizgiyi kapalı okur; bağımsız dosyalarla gidiş-dönüş
  `dxf_hatch_cases.py` (ADR 0333);
  kısıt modeli ve çözücü: belgenin `constraints`'i (`kentos_contracts::ConstraintSet`: kalıcı kimlikli on beş kural, parçalar
  `uid` ve yerle, parametreler ifadeleriyle, `constraint_expr`; `.kcad` şema 73, `Op::Constraints` tek adım), çekirdeğin
  `ops::constraints`'i (kendi çözücümüz: ikili sayılarla kesin Jakobiyen, gruplar, kısıtların sırasıyla seyrek LDLᵀ, en az
  değişikliğin Gauss–Newton'u; `fully`, `under`, `over`, `inconsistent`, `singular`, `nonConvergent`; fazla, çelişen, tekil, kopuk
  kısıtlar, serbestlik ve serbest nesneler; çelişkide hiçbir şey yazılmaz), bağlı 50 × 50 dikdörtgen 75 ms (ADR 0334);
  kısıt komutları ve aracı: `cad.constraints.edit` (add, remove, mode, flip, solve; yön çizimden, parametre adı d1/ang1/rad1/dia1,
  değer ifadeyle ya da ölçülen, başvuru ölçü; `hold`; çelişki, tekillik, fazlalık ve ters dönme `shape_flipped` retleri) ve salt
  okunur `cad.constraints.query`; uçta teğet; Kısıt aracı (`tool.constraint`, on yedi yöntem, önizleme komutun planı) CAD › Değiştir ve
  CBS › Düzenle'nin Kısıtlar panelinde; Python `kentos.cad.constraints`, `Document.constraints()`, MCP (ADR 0335);
  parametreler ve taşınma: `cad.parameters.edit` (add, set, rename, remove; ifadeyle, birimli, aralıklı; çizim yeni değerlerle aynı
  adımda çözülür), protokolün `constraints` damgası (`ProjectPart::Constraints`), veritabanı projesinin `constraints`'i (göç 0022,
  `ProjectPatch.constraints`, görüntüde `constraints_kept`), bulut eşitlemesinde meta olarak; Python `kentos.cad.parameters` (ADR 0336);
  düzenlemelerde kısıtlar: her yazan çağrının ardından aynı adımda çözüm (oynatılan parçalar tutulur, sabit eski yerinde, çelişkide
  hiçbir şey yazılmaz; kesen komutlar çözmez, bozduklarını kaldırır), gevşetme (`relaxConstraints`, `Document::set_relax`, Python
  `with doc.relaxed():`, Gevşeterek düzenle), silinen ve yapısı değişen nesnenin kısıtları `constraints_removed` ile, Kopyala ve
  Dizi'de aralarındaki kısıtlar yeni adlı ölçülerle (`constraints_not_copied`), çözüm işin durdurmasını dinler (ADR 0337);
  kısıt işaretleri ve otorite: çizimde her kısıtın işareti (türün ikonu, ölçünün adı ve değeri, tutmayan tehlike renginde; üzerine
  gelince bağlı parçalar; `kentos_interaction::constraint_marks`), Kısıtları göster, Parametreler penceresi (parametreler, kısıtlar,
  çelişenler işaretli; `cad.parameters.edit` ve `cad.constraints.edit` ile); başka ilişkinin taşıdığı nesneye kısıt konmaz
  (`constraint_governed`, `Document::governed`); DXF'te AutoCAD'in kısıt grupları okunmaz ve KentOS'unkiler yazılmaz, raporda
  söylenir (ADR 0338);
  duvar: `wall` türü (taban çizgisi, genişlik, hiza, öteleme, stil, bileşenler, uçlar, dolgu, yükseklik, kat ve tadilat durumu; `.kcad` şema
  75, `FORMATS_VERSION` 82), yüzler ve bileşen sınırları çekirdekten türer (`geom::wall`), depo onu yerleştirme gibi açar; projenin
  duvar stilleri (`settings.wallStyles`, KentOS'un beş stili kullanılınca projeye alınır, `cad.styles.edit`'in `wall` türü ve izleyen
  duvarlar) ve katları (`settings.arch`, `cad.settings.set`'in `arch`'ı); görünüş şematik, standart ya da ayrıntılı (ayardan ya da
  ölçekten; paftanın haritası kendi ölçeğiyle); `cad.walls.create`, `.convert` (Tek tek, Çiftler) ve `.edit`; Duvar ve Çizgiden duvar
  araçları, Duvar stilleri, Proje ayarları › Mimari, Öznitelikler'in duvar satırları, seçim süzgecinde Duvar; CAD'de Mimari sekmesi
  (ek kaydının `tabs`'ı); DXF'e seçilen ayrıntıda düz parçalarla; Python `kentos.cad.walls`, MCP (ADR 0339);
  duvar birleşimleri ve düzenlemeleri: birleşim saklanmaz, duvarlardan çözülür (`geom::wall_join`: köşe gönye ya da alın, düz devam, T
  bileşen önceliğiyle geçişli, X; birleşmeyen dört kodla ve onaran uç birleşimiyle), depo değişen duvarların çevresini çözer
  (`store/wall_joins.rs`); bileşenin `priority`'si, uçların `joins`'u ve `cleanup` (`.kcad` şema 76, `FORMATS_VERSION` 83);
  `cad.walls.edit`'in split, join, extend, trim, flip, replaceStyle, joins, resetJoins'i ve salt okunur `cad.walls.joins`; duvarın yüzüne
  bağlı ölçü ve odanın taraması kalınlığı aynı adımda izler (duvarın yerleri `Vertex{part: çizgi, index: yol köşesi}`); Patlat ve DXF
  birleşmiş çizgilerle ve kaybı söyleyerek; Mimari › Duvar düzenle'nin yedi aracı, Birleşimleri denetle, uyarı üçgenleri (ADR 0340);
  açıklık, kapı ve pencere: `opening` türü (duvarın kalıcı kimliği, taban çizgisi boyunca `at` ve `anchor`, genişlik, biçim, kanat,
  açılış, menteşe, yan, açı, kasa, denizlik, lento, tür, numara; pervazları `place` belgeden yazılır, `broken`: hostGone, offWall,
  overlap; `.kcad` şema 77, `FORMATS_VERSION` 84, sunucuda göç 0024), KentOS'un yedi açıklık türü; çekirdeğin `geom::opening`'i
  (pervazlar, simge, `read_back`) ve birleşim çözücüsünün boşlukları (`solve_with`), deponun `set_hosts`'u; belge açıklığı duvarıyla
  izler (`kentos_domain::opening_follow`: duvar taşınınca, kalınlaşınca yeniden yazılır, açıklığın kendisi taşınınca, döndürülünce,
  aynalanınca duvarına okunur, sessiz yeniden barındırma yok), duvarın Böl, Birleştir, Uzat, Buda ve Çevir'i açıklıklarını taşır;
  `cad.openings.create` (at, nokta, köşeden uzaklık, kenar ortası; tür) ve `.edit` (set, move, flip, rehost, replaceType, renumber,
  copy); Mimari › Açıklık: Kapı, Pencere, Boşluk, Açıklığı çevir, Açıklığı taşı, Numarala; Öznitelikler'in açıklık satırları, seçim
  süzgecinde Açıklık, duvarsız açıklığın uyarı üçgeni; Patlat ve DXF simgenin düz parçalarıyla; Python `kentos.cad.openings`, MCP
  (ADR 0341);
  mahal ve metraj: `space` türü (halkalar ve delikler `parts`, duvarlardan bulunanın tohumu `seed`, yöntem `method`: net, duvar ekseni,
  brüt; numara, ad, kullanım, kat ve tadilat durumu, yükseklik, kaplamalar, `broken`: lostEnclosure; `.kcad` şema 78, `FORMATS_VERSION` 85, sunucuda
  göç 0025), projenin yöntemi `settings.arch.spaceMethod`; çekirdeğin `geom::space`'i (duvarların birleşmiş yüzleri, tohumun yüzü, yönteme
  göre kenar kenar kaydırma, alan, çevre, kutup, tek çok satırlı etiket); belge duvarı değişen mahali aynı adımda yeniden bulur
  (`kentos_domain::space_follow`), kapanmayan nedeniyle kalır; `cad.spaces.create`, `.edit` (set, merge, split, rebound) ve salt okunur
  `.quantities` (mahaller, duvarlar, açıklıklar; kat, tadilat durumu, kullanım, katman; kurallarıyla); Mimari › Mahal: Mahal (Yöntem, Numara, Ad,
  Kullanım, Çiz), Metraj (pano, XLSX, ODS, CSV), Mahali böl, Mahalleri birleştir; Öznitelikler'in mahal satırları, seçim süzgecinde
  Mahal, kapanmayan mahalin uyarı üçgeni; Python `kentos.cad.spaces`, MCP (ADR 0342);
  aks, kolon, merdiven ve rampa: `grid` (iki nokta, büküm, etiket telde `name`, balonlar ve çapı), `column` (merkez, dikdörtgen ya da daire,
  en, boy, dönüş, tür, etiket, dolgu, iki aksı `grids`, konumu `place`, `broken`: gridGone), `stair` (2–4 noktalı yürüme çizgisi, genişlik,
  rıht sayısı ve yüksekliği, basamak, sahanlık, kesme, numaralar) ve `ramp` (orta çizgi, genişlik, yükseklik, eğim, başlangıç kotu,
  sahanlık) türleri (`.kcad` şema 79, `FORMATS_VERSION` 86, sunucuda göç 0026); çekirdeğin `geom::arch`'ı (aksın balonları, kolonun
  çevresi, şerit, sahanlıklar, basamaklar, ok, kotlar, denetimler, aks kesişimleri); belge akslara bağlı kolonu kesişimde tutar, konumu
  “B-3” (`kentos_domain::arch_follow`), aks silinince bağ nedeniyle kopar; `cad.grids.create` (seri), `cad.columns.create` (aks
  kesişimleri), `cad.stairs.create`, `cad.ramps.create` ve `cad.arch.edit` (set, renumber, attach, detach); merdivenin ve rampanın
  denetimleri uyarıdır (Blondel, yolun yetmesi, katın yüksekliği, eğim); Mimari › Taşıyıcı (Aks, Kolon, Aksları numarala) ve Merdiven
  (Merdiven, Rampa); Öznitelikler'in satırları ve Denetim'i, seçim süzgecinde dört tür, bağı kopan kolonun uyarı üçgeni; Python
  `kentos.cad.grids`, `.columns`, `.stairs`, `.ramps`, `.arch`, MCP (ADR 0343);
  etiketler ve çizelgeler: `tag` türü (nesnesi `of` kalıcı kimlikle, yeri, şablonu `{number}` … `{attr:Ad}`, çerçevesi, yazı yüksekliği,
  kolu; yazısı `text` ve bağ noktası `anchor` belgenin; `broken`: targetGone; `.kcad` şema 80, `FORMATS_VERSION` 87, sunucuda göç 0027),
  çekirdeğin `geom::tag`'ı (çerçeve yazının ölçülmüş genişliğiyle, satırlar, kol; yazısı gerçek boy kuralında), belge etiketi nesnesiyle
  yazar ve taşır (`kentos_domain::tag_follow`, değerler `arch_values`); tablonun `schedule` kaynağı (açıklık, mahal, kolon, duvar;
  sütunlar, süzgeç, sıralama, grup, toplam; satırların nesneleri `members`), belge yalnız okuduğu türler değişince yeniden yazar
  (`kentos_domain::schedules`); `cad.tags.create` (Hepsini etiketle `all`), `cad.tags.edit`, salt okunur `cad.tags.check` (yinelenen
  numara, türsüz, duvarı kopuk, nesnesiz, etiketsiz), `cad.schedules.create`, `cad.schedules.edit` (`cell` kaynağa yazar, hesaplanan
  `cell_computed`; `set`), `cad.tables.edit` çizelgenin hücresini reddeder (`schedule_cell`); Mimari › Etiket (Etiket, Hepsini
  etiketle, Etiketleri denetle) ve Çizelge (Kapı ve pencere çizelgesi, Mahal çizelgesi), Tabloyu düzenle'nin çizelge yolu,
  Öznitelikler'in etiket satırları, seçim süzgecinde Etiket; Python `kentos.cad.tags`, `.schedules`, MCP (ADR 0344);
  görünüm işaretleri ve mahalin kaplamaları: `viewmark` türü (kesit, görünüş, detay; noktalar, yarıçap, balonun yeri, bakış,
  numara, hedef: adlı görünüm ya da paftadaki harita çerçevesi; yazısı `words` ve `broken: targetGone` belgenin; `.kcad` şema 81,
  `FORMATS_VERSION` 88, sunucuda göç 0028), adlı görünümün kararlı kimliği (`id`, `view_key`; yeniden adlandırma korur), çekirdeğin
  `geom::viewmark`'ı (balon yazıdan, bakış oku, kalın uçlar, kesikli detay çemberi); belge işaretin yazısını hedefinden işaretin
  adımında ve adım dışında görünümler ya da pafta kitabı değişince, geri almada yerinde yazar (`kentos_domain::viewmark_follow`);
  `cad.viewmarks.create`, `.edit` (set, retarget, detach), salt okunur `.references` ve `cad.spaces.finish` (mahale bağlı döşeme deseni
  ya da tavan ızgarası, aynı yüzün eskisinin yerine; tarama izleyicisi mahali dış sınır alır); Mimari › Görünüm (Kesit, Görünüş, Detay,
  Başvurular), Mahal'de Döşeme deseni ve Tavan ızgarası, Donatı (KentOS blok kitaplığı, Blok ekle); Öznitelikler'in işaret satırları,
  seçim süzgecinde Görünüm işareti, kopuk işaretin uyarı üçgeni; Python `kentos.cad.viewmarks`, MCP (ADR 0345);
  mimari şablonu ve tadilat durumları: çalışma şablonu veri olarak (`catalog/v1/templates/mimari.json`: cm, 1:50, katlar, mimari ofis standardı,
  KentOS blok kitaplığı, duvar stilleri, A3 paftalar) ve `cad.templates.apply` (eksik parçaları, var olanı koruyarak; katmanlar,
  stiller ve bloklar tek adım); projenin `arch.phaseView`'u (Tümü, Mevcut durum, Yıkım planı, Yeni durum; `.kcad` şema 82, `FORMATS_VERSION`
  89), görünüş kuralı `PhaseView::look` (gizli, kesikli, soluk; açıklık duvarının, etiket nesnesinin tadilat durumuyla), çizimde ve paftada
  `kentos_native_style::phases`, deponun `set_phase_hidden`'ı; etiketin `phases` süzgeci; Mimari › Proje'de Mimari şablonu ve Tadilat ▾ (ADR 0346);
  pafta seti ve mimari DXF: salt okunur `cad.sheets.set` (paftalar adlarının numarasıyla serilerde: A-1 Planlar … Diğer; harita
  çerçevesinin zemini sabit görünümünden, `kentos_sheet::display::fixed_corners`; ona bakan işaretler; `uids` ile etkilenen
  çerçeveler: kutusu kesişen `shows`, işaretin hedefi `marked`, zemini bilinmeyen `unknown`), Pafta seti penceresi (Mimari › Görünüm,
  Paftaya git); mimari nesneler DXF'e parçalarının adsız bloğu ve nesneyi ve kalıcı kimliğini KENTOS verisinde taşıyan INSERT'le
  (`Writer::arch_whole`), geri okumada kendileri, aralarındaki bağlar yer tutucularla (`Emitter::tie_arch`, `tie_placeholders`);
  otomatik kesit ve görünüşün modeli yalnız tasarım (ADR 0347);
  dış başvuru: blok tanımının `xref`'i (mutlak ve göreli yol, Ekle ya da Bindir, SHA-256 özeti, boşaltılmış, katman grubu, getiren,
  kaynak nesnelerin kimlikleri; `.kcad` şema 83, `FORMATS_VERSION` 90), içerik çizimde son yüklenen hâliyle, kaynağın katmanları
  “Dış başvuru: ad” grubunda, Ekle türündeki iç içe dış başvurular getirenle, Bindir olanlar gelmez; `cad.xrefs.edit` (attach,
  reload, unload, repath, detach, bind; yeniden yüklemede nesne yerel kimliğini korur, bağlar onunla gider; döngü `xref_cycle`) ve
  salt okunur `cad.xrefs.list` (yüklü, boşaltılmış, bulunamadı, değişti); kayıtta göreli yol; CAD › Ekle › Dış başvuru (Dış başvuru
  ekle, Dış başvurular), açılışta değişen ve bulunamayan söylenir; kurallar `kentos_domain::xref`; Python ve MCP (ADR 0348);
  bağlı dosyalar ve arama yolları: projenin `xrefPaths`'i (`.kcad` şema 84, `FORMATS_VERSION` 91; `cad.xrefs.edit`'in `paths`'i), kaynak
  kayıtlı yolda, göreli yolda, sonra arama yollarında aranır, denenen yollar söylenir (`links::find`); salt okunur `cad.assets.check`
  (dış başvurular, bağlı resim, raster, nokta bulutu, tablo, ek ve kaynaklar: bulunduğu yer, `via`, `ok`/`changed`/`missing`/…,
  denenenler); `cad.xrefs.list`'in `affected`'i (etkilenen paftalar ve adlı görünümler); masaüstü yüklü dış başvuruların dosyalarına beş
  saniyede bir bakar, değişen kaynağı çizimin üstünde kartla söyler (Yeniden yükle, Dış başvurular), kendiliğinden yüklemez; Dış
  başvurular penceresinde Arama yolları ve Diğer bağlı dosyalar (ADR 0349);
  resim altlığı ve taşınabilir paket: `cad.images.edit` (Göm: bağlı resimlerin dosyası kitaplığa, PNG ve JPEG başlıktan; Yeniden bağla;
  kilitli katmandakiler sayılarak atlanır), resim ve rasterler arama yollarıyla aranır (`links::find_in`), bulunamayan resim çizimde
  kesikli çerçeve ve uyarı üçgeniyle, dosyası değişen resim izleyiciyle yeniden okunur (çizim hattında resmin dokusu pikselleri
  değişince yeniden yapılır); Paketle dış başvuruların kaynaklarını bayt bayt (özetleri tutar) ve `paket.json`'u (`kentos.package/1`:
  proje, kopyalar, kalanlar, yazı yüzleri; yerine başka yüzle çizilen yazı stili `left`'te) yazar (ADR 0350);
  belge sekmeleri, kırpma ve PDF altlığı: birden çok çizim açık, arkadaki sekmesinde geri alması, kamerası, seçimi ve görünümleriyle
  (`doc_tabs.rs`; Yeni sekmede aç, Sonraki çizim Ctrl+Tab, Çizimi kapat; bulut projesi sekmede beklemez), ajan bağlantısının
  çağrıları çizimini `document` ile adlandırır (MCP'nin `desktop.attach`'ı tutar), Yapıştır'ın `conflicts: rename`'i; `cad.inserts.clip`
  (Kırp: yerleştirmenin `clipping`'i tanımın koordinatlarında, Ters, Çerçeve gizli/ekranda/yazdır; çekirdeğin `block_clip`'i; `.kcad`
  şema 85, `FORMATS_VERSION` 92); PDF altlığı: kendi PDF okuyucumuz `kentos_formats::pdf` (xref tablosu ve akışı, nesne akışı, artımlı
  güncelleme, süzgeçler, formlar, yazılar ve ToUnicode, isteğe bağlı içerik grupları), sayfa `cad.xrefs.edit`'in `page`'iyle dış
  başvuru olur, görünüşü `underlay` (soldur, tek renk, yazdırma); dış başvuru tanımı `cad.blocks.edit`'te salt okunur
  (`xref_read_only`), Kaynağı aç onu yeni sekmede açar; altlığın çizgisine kenet ve bağlı ölçü `underlay_measure` der (ADR 0351);
  PDF içe aktarma: `file.import.pdf` (sayfa, bölge mm, ölçek 1:N, yer; çizgiler, dolgular, yazılar; eğriler toleransla çoklu çizgi ya da
  kesin üçüncü dereceden NURBS, okuyucu eğrileri `PdfPath.curves`'te saklar; isteğe bağlı içerik grupları katman, `skipLayers`, `layers`,
  `group`; beyaz dolgu kâğıt; `notes` PDF'in taşımadığını, `left` alınmayanı söyler; tek adım “PDF: ad.pdf”), masaüstünde CAD › Ekle ›
  İçe aktar ve CBS › Veri'de PDF içe aktar penceresi (planın canlı özeti, yer çizimden); Python ve MCP (ADR 0352);
  kenetin başvurusu: salt okunur `cad.snap.find` (tür, nokta, kimlik, yer, blok yolu, sürüm; cevabı ilişkili ölçüye olduğu gibi
  verilir), yazının, bloğun ve tablonun noktası `node` yeri, Koordinat oku kenetin nesnesini ve yerini yazar (ADR 0272);
  kenet basma anında kendi yerinde ve o anki çizimde alınır, Tab'ın adayları çizimin sürümüne bağlı (ADR 0273);
  blokların içine kenet: parçalar iç içe, dönük, ölçekli ve aynalı yerleştirmede kendi nesneleri gibi, kapalı parçanın ağırlık merkezi,
  ekleme noktası en dıştakinin, karar yerleştirmenin katmanının (gizli değil, kilitli olur, kendi türleri), dış başvuru ve mimari
  nesneler gelince aynı kuralla (ADR 0274);
  seçim tek açık sorgu: salt okunur `cad.entities.select` (pencere, kesişen, çit, çokgen, daire, içeren alan, tıklama, hepsi, verilenler,
  ters çevir, benzerleri; türler ve katmanlar, dışarıda kalanların sayısı), seçim araçları, Tümünü seç, Ters çevir ve Önceki seçim
  aynı sorguyla (`entities_select::select`, `selectable::query`), yalnız gösterilenler, kilitlininkiler seçilir; Python ve MCP (ADR 0275);
  alt nesne seçimi: tıklamanın alt nesnesi (en yakın nokta kenarından önce, kenarın `along` yeri, blokta parçanın yolu;
  `ops::subref::place_near`), salt okunur `cad.entities.pick` (adaylar en belirliden, kimlik, yer, yol), seçim aracında Ctrl/⌘ + tık
  (halka, günlük, çipte her adayın alt nesnesi), kısayol penceresinde Ctrl + tık ve Çizerken grubu (ADR 0276);
  fareyle çakışmalar: seçim kutusunun köşeleri çizimin noktaları (`SelectBox::worlds`; kaydırma ve yakınlaştırma kutuyu koparmaz,
  yönünü değiştirmez), izlerin ham fare adımı `mouse` ve `pointer-conflicts` izi (ADR 0277);
  kilitli, gizli katman ve seçim süzgeci için tek kural: gizli ve süzgeçten geçmeyen hiçbir seçimde yok, seçim süzgeci nesne seçen her
  adımda (kenar seçen araçlar da), kilitliler seçilir ama nesne kümesine iş yapan komutlar (`set` ve `order` dahil) onları sayıyla atlar
  (`checks::skip_locked`), ilişki kuran düzenlemeler bütün ya da hiç; tıklamada neden söylenir (`edge::pick_said`) (ADR 0278);
  yüzey analizi: İşlemler'in Yüzey analizi kategorisinde Eğim, Bakı, Gölgeli kabartma, Renkli kabartma, Eğrilik, Pürüzlülük (TRI, TPI,
  engebe), Güneşlenme ve Eş yükselti eğrileri; DEM şerit şerit okunur, sonuç önizleme katlı karolu GeoTIFF ve kaynağın hemen üstündeki
  yeni katmanda raster ya da kotlu çoklu çizgiler (Kot, Tür); masaüstünde İşlemler'in iş parçacığında ev sahibinin dosyalarıyla, web'de iş
  başına bir çözümleme işçisinde (`raster-wasm`; sonuç gömülür ya da indirilir); `analysis.slope` ve `map.contours` araçları açar; CBS
  şeridinde Raster sekmesi (rasterler ve raster çözümleme kategorileri); çekirdek `kentos-raster`, GIS-32–36'nın da altyapısı (ADR 0231);
  interpolasyon ve yoğunluk: İşlemler'in İnterpolasyon ve Yoğunluk kategorilerinde Ters uzaklık (IDW), Doğal komşu, Spline, Kriging (hata
  yüzeyiyle), TIN'den raster, Çekirdek yoğunluğu, Çizgi yoğunluğu; noktaların ve çizgi ve alanların kotlu köşelerinden ya da alan değerinden,
  çapraz doğrulama tablosu; sonuç girdinin katmanının hemen altında; geometri çekirdeğinde kesin Delaunay (`geom::delaunay`) (ADR 0232);
  raster işlemleri: İşlemler'in Raster işlemleri ve Raster istatistiği kategorilerinde Raster hesaplayıcı (harita cebiri: rasterler
  katmanlarının adlarıyla, bantlar `@`'la; yalnız andığı rasterler açılır, sonuç ilk andığının ızgarasında; ifade diline `ln`, `log10`,
  `log`, `üstel`, trigonometri, `derece`, `radyan`), Yeniden sınıflandır, Maskeyle kırp, Mozaik, Yeniden örnekle, Bölgesel istatistik
  (alana yazar, tablo), Histogram, Komşuluk istatistiği, Hücre istatistiği; sayıların kuralı `kentos.rasterstats/1` (çift-çift
  toplamlar); rasterlerin girdisinde ifadenin alanları bantların adları (ADR 0233);
  raster ve vektör: İşlemler'in Raster ve vektör kategorisinde Rasterleştir (alanlar, çizgiler, noktalar; sabit ya da alandan değer, altı
  çakışma kuralı, sonuç girdinin katmanının altında), Rasterden alan, Rasterden çizgi (Lü–Wang'lı inceltme), Rasterden nokta (adımla, her
  hücre, tepeler ve çukurlar); Taranmış harita kategorisinde Çizgi yakala (renge göre, tıklanan çizgi ve ona bağlılar, isteğe bağlı kot),
  Alan kapat ve Eğrilere kot ver (kesen çizginin sırasıyla); vektör sonuçlar rasterin hemen üstündeki yeni katmanda (ADR 0234);
  hidroloji: İşlemler'in Hidroloji kategorisinde Çukur doldur (taşma yüksekliğine ya da en küçük eğimle; derinlik; şeritlerde paralel
  doldurma), Akış yönü (D8, ESRI ya da TauDEM kodları, düzlükler Barnes'ın gradyanlarıyla), Akış birikimi (D8, D∞, çoklu yön; hücre,
  alan, özgül havza alanı), Topografik nemlilik indisi, Döküm noktası, Noktadan havza, Havzalar (ana, alt havzalar, güzergâhı kesen
  derelerin havzaları km'leriyle) ve Dere ağı (Strahler, Shreve, uzunluk, düşü, eğim); bütün DEM bellekte, 2²⁵ hücreye kadar; sonuçlar
  DEM'in hemen üstündeki yeni katmanda (ADR 0235);
  uzaklık ve maliyet: İşlemler'in Uzaklık ve maliyet kategorisinde Uzaklık yüzeyi (nesnelerden ya da rasterin değerli hücrelerinden;
  kesin uzaklık dönüşümü; uzaklık ya da en yakın kaynak), Birikimli maliyet (8 ya da at hamleleriyle 16 komşu; köşeden bağlı engel
  geçilmez; Yükseklik modeliyle yüzey uzunluğu ve en büyük boyuna eğim; maliyet ya da en ucuz kaynak), En düşük maliyetli yol (Yol,
  Kaynak, Maliyet, Uzunluk) ve Maliyet koridoru (yüzde ya da değer eşiği); bütün raster bellekte; nesnelerden uzaklık kaynakların
  katmanının, öbürleri maliyet rasterinin yanında (ADR 0236);
  çok ölçütlü uygunluk: İşlemler'in Uygunluk analizi kategorisinde Bulanık üyelik (Doğrusal, Üslü, Gauss, Büyük, Küçük, Yakın),
  Bulanık çakıştırma (Ve, Veya, Çarpım, Toplam, Gamma), Ağırlıklı toplam, Ağırlıklı çakıştırma (ortak ölçek, yüzde etkiler, sınıf
  tabloları ve kısıt; tam sayılarla kesin), İkili karşılaştırma (AHP: ağırlıklar ve tutarlılık oranı tablosu, isteğe bağlı ağırlıklı
  toplam) ve ROC ile doğrulama (AUC, eğrinin tablosu, en iyi eşik); çok rasterli araçlar girdilerin kesişiminde; İşlemler'in iki yeni
  parametre türü: rasterlere değer ve raster çiftleri, girdinin rasterlerinden tablolar; tablo veren çalıştırmadan sonra tablo görünür
  olur (ADR 0237);
  mekânsal istatistik: İşlemler'in Mekânsal istatistik kategorisinde Ortalama ve Ortanca merkez (ağırlıklı, gruplara göre), Standart
  uzaklık, Yön dağılımı (standart sapma elipsi), En yakın komşu (oran, z, p, desen), Moran I (sabit bant, ters uzaklık, k en yakın; satır
  standartlaştırma), Sıcak nokta (Gi*: kopyalar z, p ve güven sınıfıyla, katman sınıflara göre kategorili), DBSCAN ve k-ortalamalar
  (kopyalar küme numarasıyla, katman kümelere göre kategorili); nesnenin yeri `$merkez`, komşular k-d ağacıyla; sonuçlar girdinin
  katmanının üstünde; çekirdek `ops::spatial_stats` metinleri de yazar; CBS'de Analiz › Mekânsal istatistik (ADR 0238);
  uzaktan algılama: İşlemler'in Uzaktan algılama kategorisinde Bant birleştir (ortak ızgarada, en yakın ya da çift doğrusal), Bantlara
  ayır (bant başına GeoTIFF), Spektral indis (NDVI, GNDVI, SAVI, EVI, NDWI, MNDWI, NDBI, oran, normalize fark; ölçek ve öteleme ile
  yansıma), Denetimli sınıflandırma (eğitim alanlarının Sınıf metinleri doğal sırayla; en büyük olabilirlik ya da en yakın ortalama),
  Denetimsiz sınıflandırma (örnekten k-ortalamalar), Doğruluk analizi (karışıklık matrisi, üretici ve kullanıcı doğruluğu, kappa), Değişim
  tespiti (fark, oran, normalize fark; sınıf değişiminin “neden neye” matrisi) ve Görüntü birleştirme (ağırlıklı Brovey ya da basit
  ortalama, pankromatiğin ızgarasında); iki geçişli araçlar aynı işin geçişleri; tablolar, özetler ve uyarılar raster çekirdeğinin; raster
  okuyucusunda ExtraSamples'ın 0'ı veri, alfa değil; CBS'de Raster › Uzaktan algılama (ADR 0242);
  mesh ve çok boyutlu veri: kendi NetCDF klasik okuyucu ve yazıcımız (CDF-1, 2, 5; dosyanın gereken parçaları; `vsize` işaretsiz), CF
  düzenli ızgaranın değişkeni ve dilimi Raster ekle'den, UGRID 1.0 ve 2DM ile ASCII DAT (tek UGRID dosyasına çevrilir) Mesh ekle'den
  (`mesh.add`); mesh sanal ızgarada, karoları ağdan rasterleştirilir, ağ çizgileri (`edges`); rasterin `dataset`'i (`.kcad` şema 38),
  Zaman sürgüsünü izleyen rasterler, Raster stili'nde Veri seti, Öznitelikler'de boyutlar ve Ağ; İşlemler'in Çok boyutlu veri kategorisi
  (Kesit, Zaman serisi, Mesh hesaplayıcı), raster çözümleme araçları gösterilen dilimde; çekirdek `kentos_formats::multidim` ve
  `kentos_raster::multidim`; CBS'de Raster › Raster ▾ (ADR 0243);
  açıklamaların yükseklikleri ve ölçeği: yazı, kılavuz, ölçü, tablo, Koordinat yaz, Km yaz ve İşlemler'in yazılarının kâğıt yüksekliği
  projenin ayarı (`.kcad` şema 30; Proje ayarları › Ölçek ve yazılar), ölçek ya da genel yükseklik değişince genel yükseklikteki nesneler
  tek adımda izler (Yazı yüksekliklerini uydur), Ölçek yaz… ve türün ölçekleri, görünüş Kaybolmasın / Gerçek boy / Ekranda sabit
  (`graphics.annotationSize`), ölçünün çizgilerinin rengi, kalınlığı ve tipi (Ölçü stilleri'nin Çizgiler'i, Öznitelikler, DXF), kılavuzun
  AutoCAD gibi 14 ucu ve Ok boyu, DXF'te AutoCAD'in ok blokları (ADR 0205);
  paftada haritanın ölçeği: Çizim ölçeğini al, Görünüme sığdır, Görünümden al; koordinat listesinin kaynağı kimlikleriyle
  (Seçimi al), katman ya da canlı seçim, ne verdiği bir satırla, Çizimde göster, sütun başlıkları (ADR 0206);
  alan işlemleri: birleştir, kesiştir, çıkar, böl, alana ve çizgiye çevir, içine tıklayarak alan (ADR 0065);
  topolojik temizlik: uçlar ve köşeler var olan köşede birleşir, kısa uç uzar, taşan uç budanır, yazılan toleransla, önizlemeli tek adım (ADR 0148);
  topolojik düzenleme: durum çubuğundaki Topoloji açıkken tutamaç, tutamaç menüsü ve Esnet görünen ve kilitsiz komşuların ortak köşe ve kenarlarını da tek adımda değiştirir, kart ortak köşeyi sayar, Noktalar da seçeneğiyle (ADR 0160);
  çakışma denetimi: durum çubuğundaki Çakışma açıkken yeni alan (Kapalı alan, Parsel oluştur, Dikdörtgen, Düzgün çokgen, Daire dilimi, Alan olarak çiz) kendi katmanındaki ya da seçili katmanlardaki görünen alanlarla örtüşen kısmı çıkarılarak yazılır; Bitişik alan: yalnız yeni sınır çizilir, yolun görünümdeki komşu alanlarla kapattığı bölge imleçle dolar ve tek alan olarak yazılır, komşuların içindekiler delik; Topoloji açıkken yeni alan komşularıyla köşe köşe bağlanır, aynı adımda (ADR 0162);
  kenet ekleri: Ağırlık merkezi, Uzantı, Paralel ve Karelaj türleri (karelaj aralığı doğu ve kuzey), çizilmekte olan yola kenet, ölçek aralığında kenet; durum çubuğundaki Kenet hücresinin sağ tık menüsünde türler tek tek ve karelaj aralıkları; uçta durmak uzantısını, kenarda durmak doğrultusunu alır, yazılan mesafe uzantı ve paralel boyuncadır; Katmanlar'da katmanın kendi keneti (mıknatıs, Kenet ▸; `.kcad` şema 10) (ADR 0163);
  kayıtlı ölçüler: çizgi ve yayın Kayıtlı semt, Kayıtlı uzunluk, Kayıtlı yarıçap ve Kayıtlı yay uzunluğu öznitelikleri (metin, yazıldığı
  gibi), Çizgi'ye kutupsal yazılan uzunluk ve CBS'de grad iken semt kaydedilir; Kayıtlı ölçüleri denetle (toleranslar, Uyuyor, Farklı,
  Okunamadı; pano ve CSV) ve Kayıtlı ölçüleri çizimden yaz, tek adımda (ADR 0180);
  Genel bakış ve Büyüteç: çizim alanının üstünde kartlar; Genel bakış bütün çizimin çekirdekte çizilen resmi ve görünüm çerçevesi
  (basmak taşır, sürüklemek kaydırır, tekerlek, çift tık tümü), Büyüteç imlecin altını 2–16 kat çizim hattının ikinci kamerasıyla
  gösterir, imleç yaklaşınca öbür yana geçer (ADR 0181);
  sayısallaştırma kilitleri: değer kartında sayı ve Tab ile uzunluk, `<açı` ile doğrultu (CBS'de semt), Sapma, Nesneye paralel ve dik (kenar ya da yay seçilerek), Kalıcı, Esc önce kilitleri kaldırır; Dik açı ve kapalı alanlarda Dik kapat (D); Referans noktası ve Yapım kipi; çizimde kesikli kılavuzlar, seçilen kenar ve “R”, sağ tıkta Kilit ▸ (ADR 0166);
  ikinci koordinat sistemi: projenin ayarı (`.kcad` şema 12), durum çubuğunda, Koordinat oku'da, Mesafe ölç ve Alan hesapla'da ikinci sistemin değerleri doğruluklarıyla (EPSG'nin yolları, “resmî dönüşüm değil”), coğrafide DMS ya da DD; Koordinat dönüştür (tek nokta, Çizimden, liste, panoya ve CSV; `crs.transform`) (ADR 0167);
  saha verisi: Leica GSI, Sokkia SDR, Topcon GTS-7, Nikon RAW, Trimble JobXML ve CSV/TXT karne (alet dosyası içerikten tanınır, metin karnede sütun eşleme), Karne editörü (gözlemler, Kullan, nokta adı;
  iki durumun indirgenmesi, indeks hatası, yatay uzunluk ve kot farkı yer eğriliği ve refraksiyonla; toleransı aşan fark; poligonun
  istasyon zinciri ve iki yönden kenarları), Kutupsal alım'a ve Poligon hesabı'na aktar; Proje ayarları › Ölçme: k (0,13), iki durumun
  ve poligonun toleransları (`.kcad` şema 14, 15), Kutupsal alım da k'yı uygular, Poligon hesabı kapanmaları toleranslarla karşılaştırır;
  zemin, elipsoit ve düzlem: Proje ayarları › Ölçme'de ortalama elipsoit yüksekliği (`.kcad` şema 16), yazılıyken Mesafe ölç ve Alan
  hesapla'nın elipsoit üstündeki (jeodezikler, düzlemin ölçeği) ve zemindeki (yükseklik çarpanı) satırları; “Uzunlukları projeksiyona
  indir” açıkken Kutupsal alım ve Poligon hesabı ölçülen uzunlukları düzleme indirir, Aplikasyon zemin uzunluklarını da verir (ölçek ve
  yükseklik çarpanı raporda); çekirdekte noktanın ve çizginin ölçeği, GeographicLib'in libm'li kopyası (ADR 0171);
  köşe tablosu: alt panelin Koordinat listesi tek çizgi, çoklu çizgi ya da alanda düzenlenir: Halka, Y, X, Z, işaretli Yarıçap, Kenar,
  Semt; seçili satırların köşeleri çizimde halkalı, yerinde düzenleme (“Köşe düzenle”), Satır ekle, Sil ve Delete; Topoloji açıkken
  komşular da (ADR 0172);
  delikler: Delik ekle (halka ilk köşesinin içinde olduğu alana, seçiliyse ona; var olan deliğe değen halka onunla birleşir, sınırı aşan
  reddedilir), Deliği sil ve Deliği doldur (imlecin altındaki delik vurgulu; dolduran yeni alan deliğin kotlarıyla, delik kalır),
  şeritte Delikler paneli; Sürdür: çizgi ya da çoklu çizgi ucundan (seçiliyse imlece yakın ucundan) sürer, ilk yay uçtaki teğetle,
  eski köşeler kotlarıyla; Biçim değiştir: alanın ya da çizginin üstünden çizilen düz kenarlı hatla, alanda kesim büyük parçayı bırakır,
  dışarıdan dolanan hat cebi ekler, çizgide buluşmalar arası değişir; önizleme yeni boyutu ve değişimi gösterir (ADR 0173);
  GNSS içe aktar: GPX ve NMEA konumları WGS 84'ten projenin sistemine doğruluğu ve dayanağıyla, adlı noktalar olarak (türler, adsızların ön eki ve
  numarası, kot elipsoit yüksekliği; çözüm, uydu, HDOP, zaman ve yükseklikler öznitelik), sistemi olmayan projeye alınmaz; Cihaza gönder: seçili
  noktalar, Aplikasyon'un ve Nokta editörünün noktaları Leica GSI-16 ve GSI-8, Topcon GTS-7, Trimble JobXML, Nikon RAW ya da CSV olarak, taşınamayan
  nokta nedeniyle söylenir (SDR33 sahibin örneğini bekliyor) (ADR 0169);
  özel koordinat sistemi: projenin ya da ikinci sistemin tanımı (TM, coğrafi, taban sisteme bağlı yerel; kayıttaki ya da projenin datumu, WGS 84'e 7 parametre; `.kcad` şema 13), projenin datum dönüşümleri (7 parametre ya da cihazın NTv2 ızgarası), Izgaralar; Özel koordinat sistemi penceresi: WKT ve PROJ'dan al ve kopyala, Kayıttakini seç, Deneme noktası, Ortak noktalardan hesapla (ADR 0168);
  izleyerek çizim: yol aracının İzle (İ) düğmesi açıkken çizgiye yakın tık çizginin üstüne oturur, iki nokta arası görünen çizgiler boyunca kısa yoldan, köşeleri ve yaylarıyla; Birleştir'in Zincir (Z) seçeneği tıklanan çizginin bağlı zincirini tek çoklu çizgi yapar; yol aracının Akış (A) düğmesi açıkken imleç Adım boyu (B) kadar ilerledikçe köşe bırakır (ADR 0161);
  toplu alan: çizgilerin kapattığı bütün bölgeler tek adımda alan olur, içteki yazı ya da adlı nokta özniteliği; etiketsiz, çok etiketli bölgeler ve boşta uçlar söylenir (ADR 0151);
  etiketleri yazıya çevirme: katmanların etiketleri paftanın kuralıyla seçilen ölçekte yazı olur (Ölçek, Örtüşenler de, Zemin, Katman,
  standart yazı katmanı aynı adımda); Nesneye bağlı (B) açıkken yazı nesnesini bilir (`labelOf`, `labelScale`; `.kcad` şema 18), iki belge
  onu kayıttan önce nesnesiyle tutar (taşınınca yeniden yazılır, nesne silinince silinir, elle düzenlenince bağı kopar), bağlı yazısı olan
  nesnenin etiketi çizilmez; Öznitelikler'de Bağlı nesne ve Bağı kopar (ADR 0175);
  nesne şablonları: stil kitaplığının üçüncü öğe türü (araç, katman, renk, kalınlık, sembol, öznitelik, etiket; `.kstil` sürüm 2),
  Şablonla çiz: katmanı adıyla bulunur ya da yolundaki gruplarla açılır, renk ve kalınlık şablonun olur ve araç bitince döner, nesneler
  sembolünü, özniteliklerini ve etiketini alır, istemde şablonun adı, Son komutu yinele şablonu yineler; sağ dokta Şablonlar paneli
  (kategoriler, arama, son kullanılanlar, tıklayınca çizer); Şablon düzenleyici ve Seçili nesneden şablon; grup şablonu (üyeleri: aynı
  geometri, öteleme, köşelere nokta, ağırlık merkezinde yazı; ana nesneyle tek adımda); Şablonu uygula (seçili nesnelere, `template.apply`) (ADR 0176);
  katman yönetimi ekleri: Katmanı gizle, yalıt, kilitle, etkin yap ve Yalıtımı kaldır (nesneye tıklayarak), Katmanı eşle, Katmana
  kopyala, Kopyasını oluştur, Katmanları birleştir; Katman durumları (görünürlük, isteğe bağlı kilit ve stil; projenin ayarı, `.kcad`
  şema 19; Katmanlar panelinde Katman durumları ▾), Kullanılmayanları temizle (boş katmanlar ve gruplar, kullanılmayan bloklar,
  projenin kullanılmayan sembolleri ve varlıkları), Katman listesi (pano, CSV) (ADR 0177);
  veri karşılaştırma: iki katman, grup ya da çizim konumla ya da anahtar alanla eşlenir; eklenen, silinen, geometrisi ya da
  öznitelikleri değişen nesneler, rapor (pano, CSV) ve Karşılaştırma grubunda renkli fark katmanları (ADR 0179);
  ölçü noktası: Nokta'nın Ad, Kod ve Kot'u, ad her noktada artar, aynı yerde nokta varsa Düzelt, Ekle ya da Atla, `#ad` ile adlı noktanın yeri, Köşelere nokta (ADR 0152);
  veride arama: alt panelin Arama sekmesi (Ctrl+F, `data.search`): bütün katmanlarda nesnelerin etiketi, yazısı, blok adı ve öznitelik değerleri, alanlar
  düğmelerle seçilir, `*` kalıbı, Büyük küçük harf ve Tam sözcük, kapsam (bütün katmanlar, bir katman, yalnız seçim), satırdan yakınlaş ve seç, Hepsini seç;
  `Y,X` (CAD'de `X,Y`) yazılınca Git görünümü koordinata getirir ve çizimde kaydedilmeyen bir işaret koyar, İşareti kaldır (`data.unmark`) (ADR 0178);
  mesafe ölç, alan hesapla ve parsel oluştur (ADR 0067); seçili nesnelerin tutamaçları ve üzerine gelme kartı (ADR 0068);
  Hesap pencereleri: poligon hesabı, kutupsal alım, önden ve geriden kestirme, aplikasyon (ADR 0070, 0071);
  Vektör oturtma: kontrol noktalarından Helmert, afin ya da projektif dönüşüm, artıklar ve m0, Adla eşle, Kullan ile çift çıkarma,
  ya da Parametrelerle (taban noktası, Y ve X ölçeği, dönüklük, öteleme), ya da Kauçuk levha (bağlardan tam geçen ince plaka eğrisi,
  Sabit noktalar, Helmert'e göre yerel düzeltmeler; ADR 0158); seçili nesnelere, katmana ya da bütün çizime (kopya olarak da)
  `cad.entities.transform` ile tek adımda (ADR 0156);
  kenar eşleme: komşu paftaların kenarında buluşmayan çizgiler, çizginin devamı ölçütüyle (arama uzaklığı, açı toleransı, isteğe bağlı
  öznitelik ya da katman adı) eşlenir; komşunun ucunda, ortada ya da pafta sınırında buluşur; ucu taşı, parça ekle ya da köşeleri ayarla;
  bağlar tablosu (Kullan, Göster), `cad.entities.edit`'in `edgematch` işlemiyle tek adımda (ADR 0159);
  İşlemler: dört yerleşik araç ve Parsel ölçü yazıları modeli, tanımdan üretilen penceresiyle (ADR 0084);
  taşı, kopyala, döndür, ölçekle ve aynala (ADR 0037);
  ötele, buda, uzat, köşe yuvarla, pah, kır, birleştir, patlat, uzat-kısalt, köşe ekle/sil, esnet, dizi,
  kutupsal dizi ve hizala (ADR 0047);
  seçim, kenet ve silme (ADR 0029), pano (kes, kopyala, yapıştır, yerine yapıştır), kaydır, pencere
  ve seçime yakınlaştır ve son komutu yinele (ADR 0056); alt panel (komut geçmişi, koordinat listesi,
  uyarılar; saatli satırlar ve rozet, durum çubuğunda son ileti, ADR 0114), F4 ile paneller, tam ekran, komut arama ve sunucu denetimi (ADR 0058), çizim alanının
  sağ tık menüleri ve tek seferlik kenet vardır (ADR 0059); çizimin
  yazıları (yazı nesnesi, ölçü değeri, etiket; projenin yazı tipiyle), ölçü çizgileri ve yardımcı
  çizgiler çizilir (ADR 0055).
  Web özellikleri envanter üzerinden adım adım masaüstüne taşındı; ADR 0228'den beri masaüstü öndedir, web geri dönüşte yetişir. İki platformda da yalnız şerit
  arayüzü vardır; web'in klasik arayüzü (menü çubuğu, araç çubuğu, kayan araç kutusu) kaldırıldı
  (sahibin kararları, 27 Eylül ve 2 Ekim; ADR 0155).
- Web WebGPU/WebGL2 renderer'larını korur. Uygun WGSL kaynakları native ile
  paylaşılabilir; native Iced/application/wgpu runtime'ı web'e derlenmez.
- Server'ın ana görevi kişisel/kurumsal proje saklama, erişim, yetkilendirme,
  sürümleme, senkronizasyon ve paylaşımdır. Martin rolü şimdiki kapsamda yoktur;
  olası tile/harita servisi ayrı bir gelecek kararıdır.
- Hem CAD hem GIS projeleri PostGIS'te saklanıp çalışılabilecek. CAD tanımı
  sessizce GIS polyline'ına indirgenmeyecek; kaynak ve türev ayrımı korunacak.
- `.kcad` yalnız kayıt/yükleme için açık, sürümlü binary snapshot'tır (KCAD v2, ADR 0025,
  `docs/specs/kcad-v2.md`); DB, SQL, kalıcı sorgu indeksi veya tile dosyası değildir.
  Web ve masaüstü v2 yazar; eski v1 JSON okunur, göçü kimlikleri ve kaynağı kaydeder.
  Cloud dosyaları da aynı formatı kullanacak.
- Python embed/SDK ve AI yüzeyi sürümlü command sözleşmelerini kullanacak;
  yetki/transaction kurallarını atlayan ayrı mutasyon yolu kurulmayacak.
- Yeni mimariyi bir seferde baştan yazmayın; çalışan web'i koruyan dikey dilimler
  ve TODOS.md'deki kabul kapılarıyla ilerleyin.
