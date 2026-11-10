# A3 — Mimari plan maddelerinin gerçek durumu (kodla doğrulama)

İnceleme: 7 Kasım 2026, depo `/home/cihad/Projects/kentos-cad`, dal `arch-adv-ileri` = main `e40e6dae0`.
Yöntem: yalnız okuma (`grep`, `git log/show`, dosya okuma, JSON sayımı). Derleme ve test **çalıştırılmadı**;
"testler geçiyor" iddiaları bu incelemede doğrulanmadı, yalnız testlerin ve ortak durumların varlığı ve sayıları doğrulandı.

Kanıt düzeyleri (tabloda kısaltma):

- **K** — kod: komut katalogda (`apps/web/src/contracts/generated/commandCatalog.json`, 464 komut; mimari 63), işleyici
  (`crates/native/application/src/…`), araç (`crates/native/interaction/src/…`) ve masaüstü sahnesi var.
- **D** — bağımsız başvurulu ortak durum dosyası (`fixtures/commands/v1/*.json`) var ve durum sayısı ADR'nin iddiasıyla eşleşiyor.
- **G1** — gerçek pencere, gerçek fare/klavye kabulü ve depoda kayıt (`docs/baseline/2026-10-2x-architecture-*/README.md`).
  Mimari için 39 kayıt var; **sonuncusu 27 Ekim** (`2026-10-27-architecture-wall-dimensions`).
- **G2** — ekransız anlık görüntü (`apps/desktop/src/tools_screens.rs:1-10`: “Not a test of correctness … writes
  `.run/shots/arac-*`”; `kentos_ui::snapshot::Snapshot`, 1440×900 ve 1100×650, açık/koyu, **13 px; 18 px yok**).
  ADR 0699–0734 ve bütün ADV ADR'leri “gerçek GUI resimleri” derken bunu kastediyor. Bu makinenin `.run/shots`'unda
  27 Ekim sonrası mimari resimleri yok (`arac-mutfak-*`, `arac-doseme-*`, `arac-denetim-*`, `arac-gecici-*` = 0 dosya):
  resim kanıtı burada doğrulanamadı.
- **P** — ölçüm (perf testi ve ADR tablosu).

---

## 1. Madde tablosu

### 1.1 `plan/architecture.md` — 26 madde (18 `[x]`, 8 `[~]`, madde düzeyinde `[ ]` yok)

| Kimlik (satır) | Başlık | İşaret | Gerçek durum | Kanıt | Açık alt maddeler | Risk |
|---|---|---|---|---|---|---|
| ARCH-BASE-01 (171) | Güncel GUI tabanı | [~] | **Kısmen** | G1: ADR 0675/0676/0677/0683 baseline kayıtları (25–26 Eki); ADR 0699 `cad.styles.edit` 61 durum (D), G2 | Bütün araç/alan odağı klavye matrisi; §10 boyut/font/tema matrisi; diğer özellik ad/uzunluk alanları (satır 204-206). ADR 0776 B9: 17 mimari aracın panelinde 56 satır komut satırı isteminde yok (`crates/native/interaction/tests/all/prompt_settings.rs:31-32`, yok sayılan ölçüm testi) | **Yüksek:** 27 Ekim'den beri güncel ikiliyle gerçek pencere tabanı yenilenmedi; ADV planı da bunu “bekliyor” diyor (`architecture-advanced.md:66`) |
| ARCH-UX-01 (225) | Görünür araç ayarları | [x] | Bitti (ilk 5 araç için) | K: `apps/desktop/src/architecture_settings.rs`, `interaction/src/tool_settings.rs`; G1: ADR 0657/0661/0685/0686 (18 px dahil) | Kabul Duvar, Dikdörtgen duvar, Kapı/Pencere/Boşluk, Mahal, Ölçülü bölme'yi kapsıyor; sonradan eklenen ~20 araçta (merdiven, rampa, döşeme, mutfak, aks ağı, modül…) panel–istem eşitliği yok (B9) | Orta: “klavye akışı korunur” kabulü yeni araçlarda sağlanmıyor |
| ARCH-UX-02 (273) | Mimari şerit ve bağlam | [x] | Bitti (24 Eki hâli); **sonrası sınanmadı** | K: `catalog/v1/ribbon.json:1930` (`id: arch`); G1: ADR 0658 (46 komut) | Şerit 46 → ~75 girişe çıktı; 24'ü “Diğer” taşma menüsünde: Düzenle 11 (`tool.wallShift` dahil, :2092), Belgele 5 (`tool.roomElevation`, :2217), Proje 8 (Gezgin, Kapı/pencere tipleri, Malzemeler, Oda modülleri, Modül yerleştir, Tasarım seçenekleri, Pafta takımı, Görünüm kırp; :2260-2268). `ctx.wall` bağlam sekmesinde Duvarı kaydır yok | **Yüksek (bulunabilirlik):** 1440×900/1100×650 × 13/18 px kabulü yeni şeritle tekrarlanmadı |
| ARCH-UX-03 (290) | Güvenli başlangıç/şablon | [x] | Bitti | K: `templates_apply.rs`, `apps/desktop/src/architecture_template.rs`; D: `cad.templates.apply.review` 34; G1: ADR 0659 | Şablonun kendisi hâlâ **cm/1:50** (`catalog/v1/templates/mimari.json:7-8`), 3 A3 pafta; ofis standardı ince (`catalog/v1/standards/mimari-ofis.json`: 10 katman, 2 yazı, 1 ölçü stili; 15 Eki'den beri değişmedi; şablon 19 Eki) | Düşük-orta: fark güvenli önizlenir ama şablon m/1:100 başlangıçla uzlaştırılmadı |
| ARCH-UX-04 (312) | Etkin kat ve yeni nesnenin katı | [x] | Bitti | K: `architecture_level.rs`, `level_view.rs`; D: `cad.levels.assign` 32, `cad.sheets.levels` 31; KCAD 120; G2 | Gerçek pencere kabulü yok (ADR 0713–0720 yalnız G2) | Düşük |
| ARCH-PERF-01 (368) | Büyük belgede önizleme | [~] | **Kısmen** | K: `crates/native/domain/src/wall_candidates.rs`; P: ADR 0660 (100 bin duvar sıcak 2,379→0,156 ms), 0680 (metraj 8 647→84 ms, debug), 0782 (100 daire değişiklik 1 862→38,7 ms), 0786 (kare 351→163 ms, 200 çizelge); `apps/desktop/src/perf/{architecture,flat,corpus}.rs` | Gerçek pencere olaydan-sunuma ve saf GPU süresi (ölçümler başsız wgpu, “pencere sunumu yok”: ADR 0776 §4); stil/evre hız matrisi; 200 çizelgeli 100 dairede kare 163 ms (>100 ms hedef); etiket motorunun artımlı yerleşimi; uzun işte iptal/ilerleme | **Yüksek:** gerçek büyük projede donma riski ölçülmemiş |
| ARCH-DRAW-01 (403) | Dikdörtgen duvar | [x] | Bitti | K: `walls_rectangle.rs`, `wall_rectangle.rs`; D: `cad.walls.rectangle` 81; iz `wall-rectangle.json`; G1: 24 Eki | — (ADV-01'in `adopt`'u eklendi: 26 durum) | Düşük |
| ARCH-DRAW-02 (428) | Ölçülü bölme | [x] düz+yay | Bitti; alt madde açık | K: `walls_partition.rs`; D: `cad.walls.partition` 90 + `.arcs` 22; G1 (düz) / G2 (yay, ADR 0734) | `[ ]` kalıcı kısıt: bölme hâlâ sıradan duvar yazar (`walls_partition.rs:229-235` → `walls_create::execute`), sözleşmede bağ alanı yok (`crates/shared/commands/src/cad_walls_partition.rs:74-86`). ADV-01 bağı sonradan “Bu ölçüyü koru” ile kurulabilir ama yay kenarda bağ yok (`application/src/arch_bonds.rs:142`). Tek kaynak kenarı (ADR 0734 Sınırlar) | Orta: planın “ayrı veri modeli gerekir” notu eskidi (model ADR 0770'te var), kalan iş araca bağlamak |
| ARCH-DRAW-03 (453) | Konturdan duvar | [~] | Kısmen (işlev bitti) | K: `walls_from_lines.rs`, `wall_from_lines.rs`; D: `cad.walls.convert` 118 (93+25); G1: ADR 0666–0669 | Tam tema/font/pencere matrisi; native GUI/GPU gecikmesi; uçtan uca bellek/süre (satır 499-501). Satır 501'deki “etkin/görünür kat yönetimi tamamlanmış sayılmaz” notu ARCH-UX-04 ile kapandı (eski) | Orta |
| ARCH-OPEN-01 (505) | Kapı/pencereyi hassas yerleştirme | [~] | Kısmen (işlev bitti) | K: `opening_tool_placement.rs`, `select_opening.rs`, `cad_openings.rs:89` (`distance_reference`); D: `cad.openings.create` 112; G1: ADR 0662–0664 | `[ ]` 1440×900/1100×650 × 13/18 × açık/koyu gerçek pencere matrisi (satır 528); odaya göre numara yok (ADR 0714:46); genel çoklu mimari tutamaç düzenlemesi tamamlanmadı (ADR 0663) | Orta |
| ARCH-OPEN-02 (543) | Açıklık dağıt/çoğalt | [~] | Kısmen (işlev bitti) | K: `cad_openings.rs:512` (`Array`), `opening_array.rs`; D: `cad.openings.edit` 102; G1: ADR 0665 (koyu 18 dahil) | `[ ]` §10 matrisi ve birleşik zoom/pan/özellik kabulü (satır 564) | Düşük |
| ARCH-SPACE-01 (568) | Toplu mahal bul ve tanı | [~] | Kısmen (işlev bitti) | K: `spaces_discover.rs`, `space_discover.rs`; D: `cad.spaces.discover` 34, `.scope` 90, `.sources` 63, `.separators` 105, `.columns` 72; G1: ADR 0670–0679 | `[~]` ortak sınır dizini: release uçtan uca süre/bellek ve tam GUI matrisi (satır 661-668); blok içi kaynaklar kapsam dışı | Orta |
| ARCH-SPACE-02 (670) | Mahal kimliği, etiket, numara | [~] | Kısmen | K: `spaces_number.rs`, `tag_tools.rs`, `tag_settings.rs`; D: `cad.spaces.number` 68, `cad.tags.create` 164, `.check` 29, `.edit` 26; G1: ADR 0682–0693 | `[ ]` çoklu kat/görünüş/pafta oranı bağlamı (satır 773): etiketler paftanın ölçeğinde kâğıt boyunda çiziliyor (`crates/native/domain/src/annotative.rs:27-42`) ama ölçek başına ayrı yer/temsil yok (`crates/shared/contracts/src/annotative.rs:1-10` yalnız yazı, ölçü, kılavuz); plan 26 Eki'den beri güncellenmedi | Orta |
| ARCH-DIM-01 (778) | Otomatik ölçü dizileri | [x] | Bitti | K: `dimension_sets.rs`, `dimension_interior.rs`, `wall_dimensions.rs`, `interior_dims.rs`; D: `cad.dimensionsets.*` 7 dosya/226 durum, `cad.dimensions.interior` 47; KCAD 113/114/118/128/129; sunucu PG-23/24 (göç 0033); PDF `pdftotext`; G1 (0694/0695) + G2 | ADR sınırları: eğik/yay duvar, eğik aks, yay kenarlı mahal atomik ret (ADR 0729 Sınırlar); DXF'e tarif/üyelik gitmez; Archicad Flexible yazı kaydırma yok (ADR 0730 Sınırlar). ADR 0695'in durum satırı hâlâ “nihai özellik kabulü verilmedi” (0695:2) — kod `dd300119a` ile kapattı (`select_opening.rs:51-88` izleyenleri taşıyor) | Düşük (kod); belge tutarsız |
| ARCH-EDIT-01 (868) | Geçici ölçüyle düzenleme | [x] | Bitti | K: `temporary_refs.rs`, `wall_distance.rs`, `column_distance.rs`, `opening_distance.rs`; D: `cad.walls.distances` 33, `cad.openings.distances` 29, `cad.columns.distances` 25, `.length` 29, `arch.edit.distance` 30, `openings.edit.distance` 30; G2 | ADR 0728 Sınırlar: eğrisel/eğik aks ve iki aksa mesafe yok; T ve çok duvarlı uç kaymaz; kalıcı kilit yalnız ADV-01 bağıyla | Düşük |
| ARCH-LIB-01 (918) | Mobilya/vitrifiye galerisi | [x] | Bitti | K: `catalog/v1/blocks/kentos-temel.kblok`; D: `file.import.block.library` 30; G2 (`furniture_scenes.rs`, `block_library_scenes.rs`) | ADR 0726 Açık: kitaplıkta `.kblok`, son/sık kullanılan, bütün kitaplıklarda arama, DXF'te bloğun birim adı | Düşük |
| ARCH-LIB-02 (950) | Mutfak dizisi ve tezgâh | [x] | Bitti | K: `casework_run.rs`, `casework_edit.rs`, `casework.rs`; D: `cad.casework.run` 80, `.edit` 39, `.query` 22; KCAD 127; göç 0035 | ADR 0727 Sınırlar: köşe dolabı bloğu, uzun dolap, süpürgelik/tezgâh arkası, dik olmayan köşe ve eğri duvar, NKBA denetimi | Düşük |
| ARCH-ELEM-01 (980) | Döşeme, balkon, teras, şaft | [x] | Bitti | K: `slabs*.rs`, `slab_tools.rs`; D: `cad.slabs.create` 47, `.edit` 38, `.bound` 32; KCAD 116/123; göç 0034 | ADR 0722 Açık: katmanlı döşeme, kesit/3B, katlar arası tek şaft, eğimli döşeme | Düşük |
| ARCH-ELEM-02 (1010) | Niş, baca, tesisat boşluğu | [x] | Bitti | K: açıklığın `niche`/`flue` biçimi; KCAD 119; G2 (`recess_scenes.rs`) | ADR 0719 Sınırlar: kesme yüksekliği proje ayarı değil; kesit/3B yok | Düşük |
| ARCH-ELEM-03 (1029) | Aks ve kolon | [x] | Bitti | K: `grid_system.rs`; D: `cad.grids.create` 33, `.system` 36, `.radial` 41, `cad.columns.create.forms` 27; KCAD 126 | ADR 0725 Açık: eğrisel ağın ölçü zinciri, aks uç tutamacı, kolon kesit kitaplığı, katlar arası yinelenen aks | Düşük |
| ARCH-STAIR-01 (1057) | 2B merdiven | [x] | Bitti | D: `cad.stairs.create` 30, `.design` 33, `.curved` 26, `.levels` 34; `arch.audit.stairs` 33; KCAD 117/125; `catalog/v1/arch-profiles/` 4 profil | ADR 0724 Açık: kat yüksekliği değişince rıht sayısı yeniden hesaplanmaz; çok katlı merdiven; yangın kaçış; döner merdivende karışık kol; kesit/3B | Orta (çok katlı konut) |
| ARCH-STAIR-02 (1108) | Rampa ve korkuluk | [x] | Bitti | D: `cad.ramps.create` 25, `.design` 31, `.levels` 27; `arch.audit.accessibility` 24; KCAD 115/124 | ADR 0723 Açık: genişlik/küpeşte profili, döner rampa, kesit/3B, merdiven başlangıç kotu | Düşük |
| ARCH-DOC-01 (1138) | Semboller ve notlar | [x] | Bitti | D: `cad.north.create` 27, `cad.revisions.mark` 35, `cad.legends.materials` 20, viewmarks 37/28/27; KCAD 121/122; PDF `pdftotext/pdffonts` | ADR 0721 Açık: pafta başına revizyon çizelgesi (antet), proje revizyon tablosu, lejandın paftaya özel temsili | Orta (ARCH-DOC-04'e taşar) |
| ARCH-DOC-02 (1158) | Çizelge ve metraj | [x] | Bitti | K: `schedules.rs`; D: 8 çizelge dosyası (37/35/23 + 22/21 + 25/25/20); `file.export.table` (`capabilities.rs:7225`); KCAD 131 | ADR 0733 Açık: donatının en/boy sütunu, çok satırlı başlık, paftada ayrı çizelge görünümü | Düşük |
| ARCH-DOC-03 (1183) | Mimari denetim | [x] | Bitti | K: `arch_audit.rs`, `arch_exceptions.rs`; D: `cad.arch.audit` 80, `.exceptions` 25, `.audit.doors` 22+22, `.stairs` 33, `.accessibility` 24; KCAD 130 | ADR 0732 kapsam dışı: görünüşe göre istisna, yaylı örtüşme, 3B çakışma | Düşük |
| ARCH-DOC-04 (1211) | Pafta takımı ve ofis profili | [~] | **Kısmen** | ADR 0718 (`cad.sheets.levels` 31); ilgili: ADV-06 (tarif, şablon, kalem seti, kırpma), CAD-ADV-16 (`file.publish.sheets`, ADR 1090) | A3/A2/A1 plan+detay+çizelge hazır takımı ve fark önizlemesi; antet/revizyon (ADR 0721 açık); ofis profili (mimari-ofis.json ince; şablon cm/1:50); “10 m 1:50'de 200 mm” bağımsız kaydı ve DXF/DWG kayıp raporu mimari için ayrıca yok | **Yüksek (teslim belgesi)** |

### 1.2 `plan/architecture-advanced.md` — 7 paket (hepsi `[~]`)

Hepsi 31 Ekim–1 Kasım'da (iki günde 35 ADR: 0770–0799, 0807–0811) paralel dallarla yapıldı. 35 ADR'nin 32'si
“gerçek pencerede elle denenmedi / görüntüler ekransız” diyor; kalan 3'ü (0782, 0786, 0793) ölçüm/çizim dilimi.

| Kimlik (satır) | Başlık | İşaret | Gerçek durum | Kanıt | Açık alt maddeler (plan + ADR sınırları + kod) | Risk |
|---|---|---|---|---|---|---|
| ADV-01 (93) | Bağlantılı odalar, net ölçü | [~] | İşlev büyük ölçüde var | K: `geometry-core/src/ops/wall_adopt.rs`, `ops/arch_bonds.rs`, `application/src/arch_bonds.rs`, `interaction/src/wall_shift.rs`, `bond_refusal.rs`; D: `cad.walls.rectangle.adopt` 26, `cad.constraints.edit.arch` 31 (“Koridor bağı tutar …” kabul durumu), `cad.walls.edit.bonds` 22, `.bondsets` 23, `cad.constraints.query.arch` 22; KCAD 137; P: `tests/all/arch_bond_timing.rs` (200/50 2,4 ms; 400/100 6,9 ms) | Plan yalnız “gerçek pencerede elle kabul” diyor; ADR 0770:168-170 sınırları planda yok: **yay kenar, kolon, aks, döşeme bağı yok; duvar boyu/ucu bağlanmaz; döndürülen bağlı duvar çelişki; modül örnekleri arasında bağ yok; bağımsız bölüm kapsamı yok**. ADR 0776 B11 (yanlış neden: bağı suçlayan ret) açık. Ölçülü bölme oluştururken bağ seçeneği yok | Orta-yüksek |
| ADV-02 (133) | Yapı bileşenleri, bağlı sınırlar | [~] | İşlev var | D: `cad.walls.joins.nodes` 22, `cad.walls.edit.parts` 23, `cad.slabs.bound` 32 (“dış duvar 50 cm kayınca bağlı döşeme büyür, bağımsız balkon yerinde”), `cad.materials.edit` 34 + `.library` 30, `cad.styles.edit.materials` 21; KCAD 138/148; `catalog/v1/materials/kentos-temel.json` | Plan: IFC, kitaplık kaynağının izlenmesi, gerçek pencere. ADR 0771 §7 kalanları: köşe düğümünde bileşen kararı ve çok uçlu düğüm; kaplama nesnesi ve “çekirdeğe kadar” yüz kuralı; kalınlıkta sabit referans duvarın alanı değil (oturum tercihi); etkiler önizleme penceresinde değil durum satırında. IFC kodu yok (yalnız `formats/src/ifc_alignment.rs`, `application/src/road_ifc.rs`) | Orta |
| ADV-03 (171) | Kapı/pencere tipleri, net geçiş | [~] | İşlev var | K: `opening_types*.rs`, `opening_tool_doors.rs`; D: `cad.openingtypes.edit` 43 (+21+21; “Kabul: K01'in kasası değişir; 19 kapı izler …”), `cad.openings.create.special/multislide/slidemore/arcpocket`, `cad.arch.audit.doors` 22/22; KCAD 139/144/150/153 | Aynı tiptekileri seç'in bağımsız bölüm kapsamı yok (`commands/src/cad_opening_types.rs:331`: `OpeningsQuery` yalnız `level`); dört kanatta eşit olmayan kanat; XOX'ta dış kanatların farklı eni; kaldır-sür eşiği (ADR 0807:57); gerçek pencere | Düşük-orta |
| ADV-04 (206) | Bina programı, bağımsız bölüm | [~] | İşlev var | D: `cad.program.edit` 30, `.edit.buildings` 22, `cad.program.check` 26 (“İki daire, ortak koridor, açık mutfak, şaft, balkon ve dubleks”), `.check.shares` 20, `cad.schedules.*.units` 25/25/20, `cad.spaces.edit.decisions` 21; KCAD 140/147/151 | Resmî alan profili, arsa payı; kaçış mesafesi/en kısa yol/erişilebilir dönüş dairesi; brief'in ofis profillerinden alınması; performans bütçesi ölçülmedi (ADR 0773 §7); gerçek pencere. **Plan metni hatalı:** “Açık: bölüm çizelgesi (ADR 0795)” — ADR 0795 bunu yaptı (satır 212-213) | Orta (mevzuat) |
| ADV-05 (242) | Oda/daire modülleri, seçenekler | [~] | İşlev var, kapsam dar | D: `cad.modules.edit` 43, `.flats` 52, `.storeys` 38, `.matched` 23, `.single` 23; `cad.modules.query` 22/22/23/20; `cad.options.edit` 49, `.combos` 33; `cad.options.query` 23/21/26 (“Kabul: dört örnek iki katta, biri aynalı …”); KCAD 142/145/149/152/154/155 | Plan: seçenekler arası eşleme/birleştirme; gerçek pencere. **Kodda daha geniş açık:** modül yalnız düz duvar, açıklık, mahal, blok, ölçü, etiket, yazı alır (`contracts/src/arch_modules.rs:224-235`) — kolon, merdiven, rampa, döşeme, mahal ayırıcı, aks, mutfak dizisi kaydı yok; eğri duvar reddi (`application/src/modules_more.rs:365-371`); modülsüz kat tekrarı da bunları kopyalamaz (ADR 0796:149) | **Yüksek:** çok katlı konutta (merdiven çekirdeği, kolon, döşeme) “katları tekrarla” eksik kalır |
| ADV-06 (279) | Görünüşler, gezgin, yayın | [~] | İşlev kısmen | D: `cad.views.edit.recipe` 33, `.template` 29, `.area` 28; `cad.sheets.set.recipe` 22, `.crop` 22; `cad.navigator.read` 21; `cad.elevations.edit` 36, `.query` 22; KCAD 141/143/146 | Kanat çizimi (kapı/pencere açılımı), döşeme/tavan kesiti, sınırlı 2,5B kesit, yayın/revizyon birlikteliği, açıklama kırpması (ADR 0789), pafta kipinde etiket renklenmesi ve koşullu kalem (ADR 0783), gerçek pencere. Mimari 3B/kesit kodu yok (`crates/shared/scene3d` duvar bilmiyor) | Orta-yüksek (teslim çizimi) |
| ADV-07 (321) | Ölçülen kalite kabulü | [~] | Ölçüm ve korpus var | Kabul korpusu `fixtures/acceptance/v1/{room,flat,change,house,curved,renovation}.json`; izler `fixtures/interaction/v1/acceptance-*.json` (6); P: `perf/flat.rs`, `perf/corpus.rs`; `quality_scenes.rs`; düzelenler B1–B5, B6/B7/B8/B13, K1–K5 | Açık bulgular: **B9** (56 satır istemde yok), **B10** (bölme/dikdörtgenin geri alma adı “Duvar”), **B11**, **B12** (geçici ölçü etiketin üstünde), **K6** (Duvar/Ölçülü bölme/Mahal/Çizgiden duvar araçlarında Evre yok — `wall.rs`, `wall_partition.rs`, `space_tools.rs` vb. 0 eşleşme), **K7** (duvar genişliği araçlar arası paylaşılıyor), **K8** (`cad.settings.set`'in `arch`'ı bütün yazar), **K9** (etiket merdiven altında); etiket motorunun artımlı yerleşimi; acemi kullanıcı ve araç bulma süresi ölçülmedi; pafta GUI'de oynatılmadı; gerçek ofis DWG'si yok (tadilat GDAL'ın DXF'i); eklenti yolu yok (`.kek` ve PY-KEK-01–17 açık: `cad-advanced.md:3081-3082`) | **Yüksek:** sahibin “ciddi kullanım sorunları” bulgusuyla örtüşen tek ölçülü kaynak |

### 1.3 `plan/architecture-advanced.md` §4 uygulama sırası

| Sıra | Plan | Gerçekleşen |
|---|---|---|
| 0 | Güncel derlemeyle 2+1 akışını ölç; gerçek sorunları yeniden üret | ADR 0776 iz oynatıcısıyla ölçtü (167 adım, gerçek Iced olayları); “gerçek pencerede fare ve klavyeyle kabul yok” (0776 §5). §1'deki “Bu kapı şu an bekliyor” hâlâ geçerli |
| 1–5 | Sıralı, her biri bitiş kanıtıyla; “Dar modül kabul edilmeden bütün bina/alternatif sistemi açılmaz” | Hepsi 31 Eki–1 Kas'ta paralel. ADV-05: 0775 (dar modül) → 0779 (daire, kat tekrarı) → 0788 (tasarım seçenekleri) aynı gün; dar modülün gerçek pencere kabulü hiç yapılmadı. Sıra kapısı uygulanmadı |
| Her sıra | ADV-07 + plugins.md kapıları olmadan `[x]` yok | Tutarlı: hiçbir ADV `[x]` değil. plugins.md kapısı (eklenti) hiçbir dilimde sağlanamaz, çünkü `.kek` yok |

### 1.4 `plan/architecture.md` §9–12

| Bölüm | Durum |
|---|---|
| §9 Standart/profil | Mevzuat profilleri ayrı ve sürümlü: `catalog/v1/arch-profiles/` (4 İmar md. 31 merdiven, 1 TS 9111 rampa özeti; “mevzuat onayı değil”). Ofis profili `mimari-ofis.json` ince. ISO uygunluk iddiası yok (doğru). Resmî alan profili, yangın/kaçış yok |
| §10 Kabul kapıları | 1–4, 6–7 çoğu ADR'de karşılanmış (≥20 bağımsız durum, Python/MCP, KCAD). **Kapı 5 (gerçek pencere, fare/klavye, 13/18 px, açık/koyu, iki boyut) 27 Ekim'den sonra hiçbir ADR'de yok** (ADR 0699–0734: “gerçek pencere/fare/klavye” 0 eşleşme, “18 px” 0 eşleşme; G2 kullanıldı). Kapı 7'nin WASM'ı web durduğu için kısmi (ADR 0228; TODOS §0.2'de ~102 mimari web satırı) |
| §11 Örnekler | A (net 4×3 oda, 12 m²): G1 24 Eki. B (konut katı, A3 1:100 pafta): ADR 0776 `flat` — pafta yalnız anlamsal yolda. C (düzenleme): ADR 0798 `change`. D (kullanıcı DWG'si): `renovation` GDAL DXF'i ile; gerçek ofis DWG'si yok. Birim/uzak koordinat: ADR 0776 §3 |
| §12 Günlük | Satır 1281–1372; kronolojik değil (10-29 satırları 10-27/10-28 arasında); son mimari satır 29 Eki (ADV işleri ayrı planda) |

---

## 2. Tekrar eden açık temalar

1. **Gerçek pencerede elle kabul yok (27 Ekim'den beri).** ARCH-BASE-01 (`[~]`), ARCH-OPEN-01/02, ARCH-SPACE-01/02, ARCH-DRAW-03 ve
   bütün ADV paketleri. Kanıt: son G1 kaydı `docs/baseline/2026-10-27-architecture-wall-dimensions`; ADR 0699–0734'te gerçek
   pencere ifadesi yok; 35 ADV ADR'sinin 32'si açıkça “elle denenmedi” diyor. “Gerçek GUI resmi” = ekransız snapshot
   (`tools_screens.rs:1-10`).
2. **Tam boyut/font/tema matrisi** (1440×900/1100×650 × 13/18 px × açık/koyu): snapshot düzeni 18 px'i hiç çekmiyor.
3. **Uçtan uca performans:** ARCH-PERF-01, ARCH-DRAW-03, ARCH-SPACE-01, ADV-07 — gerçek pencere olaydan-sunuma ölçülmedi;
   200 çizelgeli 100 dairede kare 163 ms; etiket motorunun artımlı yerleşimi; komut UI iş parçacığında, iptal/ilerleme yok.
4. **Klavye/komut satırı eşitliği:** B9 (17 araç, 56 satır), TODOS `UX-01…UX-03`, `UX-06`, `UX-08` açık.
5. **Yay/eğik geometri eksikleri:** bağlar (yay kenar yok), ölçü dizileri (eğik/yay duvar ret), geçici ölçü (eğrisel aks yok),
   mutfak dizisi (eğri duvar yok), modüller (eğri duvar ret), ölçülü bölme (tek kenar).
6. **Kesit/görünüş/3B ve IFC:** yalnız oda iç görünüşü (ADR 0799); kesit işareti yalnız başvuru (A9); mimari IFC yok.
7. **Bulunabilirlik:** ADV'nin ana komutları şeridin taşma menülerinde (ARCH-UX-02 kabulü eski şeride ait).
8. **Mevzuat/kurum profili:** resmî alan, arsa payı, yangın/kaçış, kurumun kendi profil dosyası (ADV-04, STAIR-01/02).
9. **Gerçek ofis verisi:** gerçek ofis DWG'siyle tadilat (ADV-07, §11 D; TODOS `CADP-11` ertelendi).
10. **Veritabanı (bulut) projesi:** ölçü dizisi (PG-23/24, göç 0033), döşeme (0034), mutfak (0035) sunucuda sınandı; ADV
    alanlarına (`openingTypes`, `modules`, `optionCombos`, `typeVersion`, `startPass`, `materialId`, `auditExceptions`)
    `crates/server`'da hiç atıf yok — `settings` JSON'unda taşınıyor olabilir ama sunucu kuralı/kabulü yok (doğrulanmadı).
11. **Web:** her madde ertelendi (ADR 0228), TODOS §0.2'de ~102 mimari satırı.

## 3. `[x]` işaretli olup kabul koşulu eksik görünenler

1. **ARCH-UX-02** — kabul 24 Ekim'in 46 komutlu şeridine ait; şerit ~75 girişe çıktı, 24'ü taşma menüsünde, matris yeniden
   koşulmadı. ADV-07'nin “beş panelli şerit kalabalıklaştırılmasın” şartı taşma menüsüyle sağlanmış, bulunabilirlik bedeliyle.
2. **ARCH-UX-01** — “klavye akışı korunur” kabulü ilk beş araç için; B9 sonraki araçlarda panel–istem eşitliğinin olmadığını ölçtü.
3. **ARCH-DIM-01** — kod kapanmış, ama ADR 0695'in durum satırı (“son Python/API, clippy ve WASM kapıları beklenir. Nihai özellik
   kabulü verilmedi”) güncellenmemiş; plan `[x]` ile ADR çelişiyor.
4. **ARCH-UX-04, ARCH-EDIT-01, ARCH-LIB-01/02, ARCH-ELEM-01/02/03, ARCH-STAIR-01/02, ARCH-DOC-01/02/03** — §10 kapı 5'in istediği
   gerçek pencere, fare/klavye, 18 px kanıtı yok; GUI kanıtı yalnız G2 (ve bu makinede yok). İşlev, durum ve API kabulü sağlam (D).
5. **ARCH-UX-03** — kabul geçerli, ama şablon içeriği (cm/1:50, A3) ADR 0626'nın m/1:100 başlangıcıyla hâlâ uzlaşmamış.
6. **ARCH-DRAW-02** — madde `[x]`, ama başlığındaki `[ ]` kalıcı kısıt ADV-01'in bağ modeliyle çözülebilir hâle geldiği hâlde
   bağlanmadı; plan notu eskidi.
7. **ARCH-DOC-01 / ARCH-DOC-02** — kendi açıkları (pafta başına revizyon çizelgesi; donatının en/boy sütunu) ARCH-DOC-04'ün
   teslimini doğrudan etkiliyor.

## 4. En önemli 15 bitmemiş iş (öncelik sırasıyla)

1. **Güncel ikiliyle gerçek pencere kabul turu** (acemi + deneyimli 2+1 akışı, araç bulma süresi; 13/18 px, iki boyut, iki tema) —
   ARCH-BASE-01, ADV-07 §0. Hiçbir ADV dilimi gerçek kullanımda görülmedi.
2. **Komut satırı ↔ panel eşitliği ve odak** — B9 (17 araç/56 satır), TODOS UX-01–03/06; B10 geri alma adları.
3. **Tadilat ve hızlı çizim sürtünmeleri** — K6 araçlarda Evre, K7 duvar genişliğinin araç başına belleği, K8 `settings.set` arch yaması,
   B12/K9 yazı çakışmaları.
4. **Bulunabilirlik** — ADV komutlarını taşma menülerinden bağlama (ör. `ctx.wall`'a Duvarı kaydır, bağlar, malzeme), ARCH-UX-02 kabulünü
   yeni şeritle yeniden koşma.
5. **Ölçülü bölmede kalıcı bağ** (“Bu ölçüyü koru” oluştururken) ve bağların yay kenar/kolon/aks kapsamı — ARCH-DRAW-02 `[ ]`, ADR 0770 sınırları.
6. **Modül/kat tekrarı kapsamı** — kolon, merdiven, rampa, döşeme, ayırıcı, mutfak dizisi ve eğri duvar (`arch_modules.rs:224-235`).
7. **Büyük projede akıcılık** — kare 163 ms → <100 ms, etiket motorunun artımlı yerleşimi, gerçek pencere olaydan-sunuma, iptal/ilerleme (ARCH-PERF-01).
8. **Pafta takımı ve ofis profili** — A3/A2/A1 hazır takım + fark önizlemesi, antet/pafta revizyon çizelgesi, proje revizyon tablosu,
   yayın/revizyon birlikteliği, şablonun m/1:100'e uyumu (ARCH-DOC-04, ADR 0721 açık, ADV-06).
9. **Türetilmiş çizimler** — kapı/pencere açılımı, sınırlı 2,5B kesit, döşeme/tavan kesiti (ADV-06).
10. **Merdivenin çok katlı tasarımı** — kat yüksekliği değişince rıht sayısı, kesme rıhtı, çok katlı merdiven, karışık kol (ADR 0724 açık).
11. **Eğik/yay geometri** — ölçü dizilerinde eğik/yay duvar ve eğik aks, eğrisel aks ölçü zinciri, eğri duvar boyunca mutfak.
12. **Açıklık derinleşmesi** — odaya göre numara, Aynı tiptekileri seç'in bölüm kapsamı, çoklu mimari tutamaç, dört kanatta eşit olmayan kanat.
13. **Gerçek ofis DWG'siyle tadilat kabulü** — ADV-07 korpusu, §11 D, CADP-11.
14. **Mevzuat profilleri** — resmî alan profili, arsa payı, kaçış; profillerin kurum dosyası (ADV-04, STAIR-01/02).
15. **Belge hijyeni** — §7'deki tutarsızlıklar (yanlış KCAD numaraları, eski durum satırları, plan girişinin “uygulanmamış” dili).

## 5. Sonraki iş için sayılar

**ADR numaraları**

- Mimari aralıkları: **0656–0734** (ARCH; 79 numara; 0731 komut kataloğu gibi birkaç genel ADR içinde) ve **0770–0799 + 0807–0811**
  (ADV; 35). Son mimari ADR **0811** (1 Kas). Bütün depoda son ADR **1780** (MAP-12, 7 Kas).
- Numaralandırma düzenleri: Yol `plan/road.md` §25.1 adım 3 “son numaranın bir fazlası (şu an 1042)” der (yani 1042–1059 boşluğu
  Yol'un sıradaki numarasıdır); Harita (MAP) 20'lik bloklar kullanır (1200, 1230, 1260, …, 1760, 1780 → sıradaki 1800; alt dilimler
  1201, 1231, 1261-1262 gibi blok içinde); CAD-ADV 1000–1131 arasında bloklarla ilerledi (CAD-ADV-20 hâlâ `[~]`).
- Boş bloklar (78 dalın hiçbirinde ADR yok — `git ls-tree` ile denetlendi): **0766–0769** (4), **0883–0899** (17), 1042–1059 (18; Yol'un
  “son+1” işaretçisi), 1069–1089 (21), 1091–1099, 1101–1109, 1113–1119, 1132–1199 (68; CAD-ADV devamı riski), ≥1781 (MAP bloğu riski).
- **Öneri:** mimari ileri planın yeni maddeleri için **0883–0899** bloğunu ayırın; sıradaki boş ADR **0883**. 17'den fazlası gerekirse
  MADDE-TARIFI §14 gereği sahipten ayrı bir blok (çakışmasız, ör. 2000–2099) isteyin ve plana yazın. “Son numara + 1” (1781) MAP'in
  1780 bloğuna düşer; kullanmayın.

**`.kcad` şeması** (`crates/shared/kcad/src/lib.rs`)

- En yeni şema **204** (`SCHEMA_WITH_ROAD_APPROVAL_SOURCES`, lib.rs:850). `SCHEMAS: [u32; 202]` (lib.rs:855): 2, 3–155, 157–204;
  **156 hiç kullanılmadı** (155 `SCHEMA_WITH_INSTANCE_STOREYS` lib.rs:743, 157 `SCHEMA_WITH_ARC_EDGE_BONDS` lib.rs:745; eski gelecek-sürüm
  örneği `fixtures/kcad/v2/broken/schema-version-156.kcad` duruyor).
- Sıradaki şema **205** (gelecek-sürüm örneği `fixtures/kcad/v2/broken/schema-version-205.kcad` → 206'ya taşınacak); MADDE-TARIFI
  §14: dalda geçici numara, main'e birleşme sırasıyla kesinleşir.
- `FORMATS_VERSION` **107** (`crates/shared/contracts/src/formats.rs:234`).
- Mimari şemalar: 108–131 (ARCH: 108 mahal kolonu … 131 donatı çizelgesi), 137–155 (ADV: 137 bağlar … 155 tek katlı örnek).

**Ortak durumlar ve diğer sayılar**

- `fixtures/commands/v1`: 603 JSON (+5 alt klasör). Mimari çekirdek aileler (`cad.{arch,casework,columns,dimensionsets,elevations,
  grids,levels,legends,materials,modules,navigator,north,openings,openingtypes,options,program,ramps,revisions,slabs,spaces,stairs,
  tags,walls,viewmarks}.*`): **115 dosya / 4 208 durum**. Mimariye bitişik (çizelge, görünüş tarifi/şablon/alan, pafta kat/tarif/kırpma,
  stil malzeme/tablo, şablon uygula, blok kitaplığı, mimari kısıt, iç ölçü zinciri): 21 dosya / 581 durum. **Toplam 136 / 4 789.**
- Kabul korpusu: `fixtures/acceptance/v1` içinde 6 mimari senaryo (room, flat, change, house, curved, renovation).
- Etkileşim izleri: 295'in 11'i mimari (`wall-*` 5, `acceptance-*` 6); açıklık, mahal, etiket, merdiven vb. için araç başına iz yok
  (etkileşim crate testleri ve kabul izleriyle kapsanıyor).
- Masaüstü sahne dosyaları: 155'in 52'si mimari (`*_scenes.rs`). Katalog: 464 komutun 63'ü mimari. Mimari baseline kaydı: 39.

## 6. Çekirdek CAD yetenekleri (mimari çizimi doğrudan etkileyenler)

| Yetenek | Durum | Kanıt | Mimari için not |
|---|---|---|---|
| Nesne kenetleme ve izleme | **Var** | `interaction/src/object_tracking.rs` (606 satır), `cad.snap.find`; izler `snap-*`, `object-tracking`, `track-point`; `plan/cad.md:259-264` CAD-INPUT-05…10 `[x]` | Mimari alt yerler (pervaz, yüz) ADR 0694 SubPlace ile |
| Dinamik girdi | **Var / kısmen** | CAD-INPUT-01…03 `[x]` (`plan/cad.md:255-257`, ADR 0267), ADR 0813 | TODOS UX-08 (giriş kutusunun konumu) ve UX-04 açık; mimari araçlarda B9 |
| Tutamaçlar | **Var / kısmen** | `grip_edit.rs`, `param_grip.rs`, `grip_menu.rs`; CAD-INPUT-15/16 `[x]`; ADR 0663, 0684, 0824 | Genel çoklu mimari tutamaç ve aks uç tutamacı açık (ADR 0663, 0725) |
| Tarama | **Var** | CAD-HATCH 13/13 `[x]`; CAD-ADV-08 `[x]` (ADR 0814, 0819) | — |
| Ölçü stilleri | **Var** | CAD-ANNO `[x]`; CAD-ADV-09 `[x]`; ADR 1023 (DIMTXSTY) | Ölçü dizileri belge ölçü stilini kullanır |
| Çok satırlı yazı | **Var** | `paragraph.rs`; CAD-ADV-10 `[x]` (unicode-v1, şema 163), SHX ADR 1022 | — |
| Tablo | **Var** | `table.rs`; tablo stilleri (şema 185, ADR 1126–1128), veri çıkarımı ADR 1030; CAD-ADV-12/13 `[x]` | Mimari çizelgeler aynı tabloyu kullanır |
| Blok / dinamik blok / öznitelik | **Var** | CAD-BLOCK 20/20 `[x]`; CAD-ADV-06 `[x]` (ADR 0844–0864) | Oda modülü bloktan ayrı (ADR 0775) |
| Dış başvuru | **Var** | `xrefs.rs`, `xrefs_bind.rs`, `xrefs_review.rs`; CAD-REF 20/20 `[x]`; CAD-ADV-14 `[x]` | Altlıkta tadilat korpusu DXF ile |
| Pafta / görünüm penceresi | **Var** | `sheets_*.rs`; CAD-SHEET 30/30 `[x]`; CAD-ADV-16 `[x]`; görünüş tarifi ADR 0774 | Mimari hazır pafta takımı açık (ARCH-DOC-04) |
| Kalem tabloları (CTB/STB) | **Var** | `cad.plotstyles.import` (`plot_styles_import.rs`, ADR 1130); baskı profili ADR 0609; tarifte kalem seti ADR 0783 | Koşullu kalem açık (ADR 0783) |
| Yayın | **Var** | `file.publish.sheets`, `file.publish.status`, `file.preflight.sheets` (ADR 0615/0617/0618/1090) | Pafta başına revizyon çizelgesi açık (ADR 0721) |
| DWG | **Kısmen** | `crates/native/dwg` (OpenCADCodec, ADR 0586), `file.import/export.dwg` (51 durum) | Gerçek ofis DWG kabulü ertelendi (CADP-11) |
| IFC (mimari) | **Yok** | Yalnız `IfcAlignment` (yol); `IfcWall/IfcDoor/IfcSpace` yok | ADV-02 ve TODOS BIM-07 açık |
| Mimari 3B / kesit | **Yok** | `crates/shared/scene3d` mimari nesne bilmiyor | Yalnız oda iç görünüşü (ADR 0799) |
| Python / MCP | **Var** | MCP katalogdaki her komutu üretir (`crates/native/mcp/src/tools.rs:60` ve `:582-925`); CAD-ADV-20 `[~]` | Eklenti (`.kek`) ve AI sağlayıcı yok |

Genel: `plan/cad.md` §4–17'nin maddeleri neredeyse tamamen `[x]` (CAD-ARCH 44/45, CAD-IO 24/26). `plan/cad-advanced.md`
CAD-ADV-01…19 `[x]`, 20 `[~]`. §18–23 (Python, AI, CLI, yaşam döngüsü, performans, QA) `[ ]`.

## 7. Belge tutarsızlıkları (yeni plana geçmeden düzeltilmesi önerilir)

1. Eski KCAD numaraları: `plan/architecture.md:603, 614, 689, 1310, 1314, 1316, 1327` ve `docs/kilavuz/00-guncel-karar-ozeti.md:1125,
   1234, 1246` “KCAD 104/105/106/107” diyor; gerçek 109/110/111/112 (lib.rs:599-609; ADR'ler düzeltilmiş).
2. ADR 0695:2 durum satırı eski (§3/3).
3. `architecture-advanced.md:212-213` “Açık: bölüm çizelgesi (ADR 0795)” → yapıldı.
4. `architecture-advanced.md:11-14` giriş (“uygulanmış sayılmaz”) ve §1 kanıt tablosu `dc7b16d6c`'ye ait: A2, A3, A5, A6, A7, A8 artık
   ADR 0770–0775 ile karşılandı; A1 (bölme bağ yazmaz) ve A9 (kesit işareti kesit üretmez) hâlâ doğru.
5. `architecture.md:499-501` ARCH-DRAW-03'teki kat notu ARCH-UX-04 ile kapandı.
6. `architecture.md:450-451` ARCH-DRAW-02 kalıcı kısıtının “ayrı veri modeli gerekir” gerekçesi eskidi (ADR 0770/0780/0790).
7. `architecture.md:773` ARCH-SPACE-02 çoklu pafta maddesi kısmen karşılandı (§1.1), yeniden kapsamlanmalı.
8. `plan/road.md` §25.1 adım 3'ün “son numara + 1 (şu an 1042)” kuralı bugün 1781'i verir ve MAP bloğuyla çakışır.
9. ADV-01 durum paragrafı ADR 0770'in sınırlarını (yay, kolon, aks, döşeme) saymıyor.

## 8. Yöntemin sınırları

- Test, derleme ve bağımsız başvuru `--check`'leri çalıştırılmadı; durum sayıları JSON'dan sayıldı.
- GUI resimleri bu makinede yok; resim kanıtı görülemedi.
- Sunucunun ADV verisini veritabanı projesinde nasıl taşıdığı yalnız `grep` ile tarandı; davranış doğrulanmadı.
- Kaynak satır numaraları HEAD `e40e6dae0`'a aittir.
