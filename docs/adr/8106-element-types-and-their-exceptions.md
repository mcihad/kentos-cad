# ADR 8106: Eleman tipleri — kolon, kiriş, döşeme, çatı, korkuluk ve merdiven tipi; örneğin istisnası ve değerin kaynağı

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-12 ("Tipler ve katmanlı
  yapı"), B21; [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-04'ün ikinci parçası. BIM-12'nin ilk kabulü sağlandı (§4);
  giydirme cephe tipi, profil takımı, ofis kitaplığı ve eski sürüm denetimi §6'da.
- **Bağlam:** ADR [0772](0772-door-window-types-instances-clear-width.md) (kapı ve pencere tipleri: tip, sürüm, istisna,
  takip raporu; bu ADR aynı düzeni öbür elemanlara genişletir), [8105](8105-wall-style-types-and-layer-sets.md) (duvar
  stili IFC'de tip, `TypeObject`), [0343](0343-grids-columns-stairs-ramps.md) ve [8041](8041-beams-lintels-column-wall-joins.md)
  (kolon ve kiriş), [0722](0722-slabs.md) (döşeme), [8044](8044-roof-and-roof-plan.md) (çatı), [8045](8045-balanced-stairs-and-special-elements.md)
  (korkuluk ve merdiven), [8070](8070-missing-building-elements.md) §8, §10 (dikme, kayıt, kaplama).
- **Web:** dokunulmadı (ADR 0228); TS türleri üretildi (`ElementType`, `TypeLink`, `ElementTypesEdit` …); pencere TODOS.md
  §0.2'de. **`.kcad`:** Mimari verisi 19 (§2, K187). **Komut:** `cad.elementtypes.edit` v1. **Bağımlılık:** yok.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Revit, [About Element Properties](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-GetStarted/files/GUID-1B5B5C2E-072F-4349-9C70-D88204F9145D.htm) | Tip özellikleri o tipin projedeki bütün örneklerine uygulanır, bir tipin bütün örneklerinde aynı değerdedir; örnek özellikleri elemanın yerine göre değişebilir; Edit Type tip penceresini açar | Tip değişince izleyen elemanlar alır (K184); yer, kat, dönüş, etiket elemanındır (K182). Revit'ten ayrı olarak örnek bir tip alanında kendi değerini tutabilir: plan "örnekte ayrılan değer korunur" diyor (K183) |
| Vectorworks, [Additional plug-in object style and instance options](https://app-help.vectorworks.net/2021/eng/VW2021_Guide/Symbols/Additional_plug-in_object_style_and_instance_options.htm), [Changing plug-in object styles from the Object Info palette](https://app-help.vectorworks.net/2020/eng/VW2020_Guide/Symbols/Changing_plug-in_object_styles_from_the_Object_Info_palette.htm) (arama sonucunun özetinden; sayfa doğrudan açılmadı, 403) | Her parametre stilde "By Style" ya da "By Instance"tır; stil düzenlenince by-style değerler stili kullanan bütün örneklere gider; stilsize çevrilen nesne değerlerini korur, hepsi örneğin olur | Ayrım stilde değil örnekte: bir alan elemanın kendi değeriyse istisnadır (K183); Tipten ayır değerleri korur (K184) |
| ADR 0772 (KentOS'un kapı ve pencere tipleri) | Tip, sürüm, istisna listesi; takip raporu: izledi, istisna korundu, atlandı (kilitli), reddedildi; Tipten ayır, Tipe uygula, Tipe döndür, Tipi yeniden uygula | Aynı kavramlar ve adlar, bütün türler için tek komutta (K184) |
| IFC 4.3 [`IfcColumnType`](https://raw.githubusercontent.com/buildingSMART/IFC4.3.x-development/master/docs/schemas/shared/IfcSharedBldgElements/Entities/IfcColumnType.md), [`IfcColumnTypeEnum`](https://raw.githubusercontent.com/buildingSMART/IFC4.3.x-development/master/docs/schemas/shared/IfcSharedBldgElements/Types/IfcColumnTypeEnum.md) | Kolon tipi kolonların ortak bilgisidir; tipin malzemesi `IfcMaterialProfileSet` (ya da yedek olarak `IfcMaterial`), profil takımı varsa her kolonun `IfcMaterialProfileSetUsage`'ı olmalı; PredefinedType COLUMN, PILASTER … USERDEFINED (o zaman ElementType yazılır), NOTDEFINED | Her eleman tipi sınıfının IFC tipi; elemanlarının ortak PredefinedType'ı, değilse NOTDEFINED; USERDEFINED tipte NOTDEFINED (K185). Profil takımı ve kullanımı sonraki iş (§6) |

## §2. Karar

- **K182 — Tip:** projenin tipleri `settings.arch.elementTypes`'tadır (`ElementType`: kimlik, ad, tür, sürüm, değerler).
  Türler: kolon, kiriş, döşeme (kaplama da), çatı, korkuluk, merdiven. Değerler türün **tip alanlarıdır**, elemanın kendi
  alan adlarıyla (`width`, `riserHeight` değil `tread` …); verilmeyen alan o alanın yokluğudur (varsayılanı). Tip alanları:
  kolon `member`, `shape`, `width` (zorunlu), `depth`, `legs`, `height`, `fill`, `material` ve adı (`type`, tipin adı);
  kiriş `role`, `member`, `width`, `height` (zorunlu), `material`; döşeme `role`, `covering`, `thickness` (zorunlu),
  `materialId`, `material`, `edge`, `layers`; çatı `thickness`, `material`, `layers`, `gutter`, `snowGuard`; korkuluk
  `height`, `width` (zorunlu), `infill`, `gap`; merdiven `width`, `tread` (zorunlu), `landing`, `winders`, `post`,
  `railings`, `numbering`. Elemanın yeri, katı, dönüşü, eğimleri, akslara bağı, etiketi kendisinindir; merdivenin rıhtı
  katın yüksekliğinden gelebildiği için tip alanı değildir. Değerler elemanın yazdığı biçimde saklanır (varsayılan değer
  yazılmaz, `1` `1.0` olur): böylece tipini izleyen eleman yazımın farkı yüzünden istisna sayılmaz. Tip, türünün sade bir
  elemanına uygulanıp türün kendi denetiminden geçmeden kabul edilmez. Ad türünün tipleri içinde tektir (büyük küçük harf
  ayrılmaz); kiriş tipi ile kolon tipi aynı adı taşıyabilir.
- **K183 — Bağ ve istisna:** tipi izleyen eleman `arch.elementType` taşır: tipin kimliği, aldığı sürüm, kendi değerini
  tuttuğu alanlar (`overrides`, türün sırasıyla). Belge her düzenlemede (Öznitelikler, tutamak, betik, hangi komut olursa
  olsun) istisnaları doğru tutar: tipin sürümündeki elemanda istisnalar tipinkinden farklı olan alanlardır; tipin değerine
  geri getirilen alan yeniden tipi izler. Eski sürümdeki elemanda tuttukları ve düzenlemenin tipin son değerinden başka
  bir değere getirdiği alanlar.
- **K184 — Komut** (`cad.elementtypes.edit` v1, tek geri alma adımı "Eleman tipi"): define (değerlerle; `copyOf` bir tipin
  kopyası; `from` çizilmiş bir eleman — **Tipe çevir**: önce çiz, beğenince tipe çevir; eleman aynı adımda tipi izler),
  update (verilen değerler, null kaldırır; sürüm bir artar, izleyenler alır), rename (kolonların tür yazısı yeni ad),
  remove (izleyeni varsa `detach` olmadan ret), assign (Tipe bağla: bütün değerler, istisnasız), detach (Tipten ayır:
  değerler kalır), reset (Tipe döndür), push (Tipe uygula: elemanın kendi değerleri tipin olur, öbürleri izler), refresh
  (Tipi yeniden uygula: eski sürümdekiler son sürümü istisnalarıyla alır). Takip raporu ADR 0772'ninki: izledi
  (`applied`), istisna korundu (`kept`, iki listede de), atlandı (`skipped`, kilitli katman), reddedildi (`refused`:
  türün kendi kuralı — kendi kolları kolondan kalın kalan L kolon gibi; `onRefused: stop` hepsini reddeder, `skip` onu eski
  sürümde bırakır). Plan önizlemedir: yazmadan tabloyu ve her elemanın sonucunu verir.
- **K185 — IFC:** her eleman tipi değerlerinin yaptığı sınıfın IFC tipidir (`IfcColumnType`, dikmede `IfcMemberType`,
  `IfcBeamType`, kayıtta `IfcMemberType`, `IfcSlabType`, kaplamada `IfcCoveringType`, `IfcRoofType`, `IfcRailingType`,
  `IfcStairType`); o sınıftan elemanları `IfcRelDefinesByType` ile ona bağlıdır, istisnası başka sınıf yapmış eleman
  bağlanmaz. PredefinedType elemanlarının ortak değeridir, değilse NOTDEFINED. Elemanları aynı katman takımını taşıyorsa
  tipin takımı odur (ADR 8105'in paylaşımıyla). Tipler projede `IfcRelDeclares` ile bildirilir; GlobalId tipin kimliğinden
  (bina dosyasında binanın kimliğiyle) kurallıdır.
- **K186 — Arayüz:** Eleman tipleri penceresi; Öznitelikler'de değerin kaynağı ve Tip bölümü (§5).
- **K187 — `.kcad`:** tipler ve elemanın bağı Mimari verisi 19'dur; belge şeması değişmez. Daha eski okuyucu dosyayı
  "eleman tipleri mimari verisi sürüm 19'dan önce yok" diye reddeder.

## §3. Uygulama

- **Sözleşme:** `crates/shared/contracts/src/arch/element_types.rs` (`ElementTypeKind`, `TypeField`, `ElementType`,
  `TypeLink`; `follow_type`, `fields_off_type`, `exceptions_after`, `element_type_values`, `canonical_values`,
  `element_types_problem`); `ArchInfo.element_type`, `ArchSettings.element_types`; kodlar `element_type_*`.
- **Belge:** `crates/native/domain/src/element_types.rs` (tipler geri alma geçmişinde `ArchOp::ElementTypes`; `update_many`
  her güncellemede `keep_exceptions` çağırır).
- **Komut:** `crates/shared/commands/src/arch/cad_element_types.rs`, katalog ve yetenek tablosu;
  `crates/native/application/src/arch/element_types_edit.rs`.
- **IFC:** `crates/shared/ifc/src/arch.rs` (eleman tiplerinin `TypeObject`'leri).
- **`.kcad`:** `ARCH_WITH_ELEMENT_TYPES`, `uses_element_types`; okuyucunun reddi.
- **Masaüstü:** `apps/desktop/src/arch/element_types_window.rs`, `cad/properties/rows/element_types.rs`, satırın `source`'u
  (`rows.rs`, `panel.rs`); komutlar `arch.elementTypes`, `arch.elementTypeFrom`, `arch.elementSameType`.
- **Python:** `python/kentos/cad/elementtypes.py` (`scripts/python/sdk.py` ile üretildi).

## §4. Sınamalar

- **İlk kabul** (`crates/native/application/tests/all/element_types.rs`): on yedi kolon; birincisi Tipe çevir ile S1
  (değerleri 30/50, dolu), öbürleri ona bağlanır; on yedincinin eni Öznitelikler'in komutuyla (`cad.arch.edit`) 0,45 olur
  ve istisna sayılır (`Örnekten`). S1'in eni 0,35 olunca **16 kolon izler, kendi eni olan izlemez**; hepsi sürüm 2;
  bir geri alma tipi ve kolonları geri verir.
- **Bağımsız başvuru** (`scripts/fixtures/element_types_cases.py`, KentOS kodu olmadan): `cad.elementtypes.edit`'in 45
  ortak durumu: kabulün senaryosu (kilitli katmandaki eski sürümde kalır), değer kaldırma, kiriş tipi, L kolonun reddi ve
  atlanması, beş türde Tipe çevir, tipin yazım biçimi, ad, silme, bağlama, ayırma, döndürme, uygulama, yeniden uygulama,
  başka komutun düzenlemesinin istisna yapması ve tipin değerine dönünce yine izlemesi, retler.
- **IFC** (`element_types.rs`, ikinci sınama; `scripts/fixtures/ifc_types_check.py`, IfcOpenShell 0.9.0): kesit binasının
  elemanları kendi değerlerine göre tiplenir; IfcOpenShell her tipi sınıfıyla okur: "Kolon 1" `IfcColumnType` COLUMN, 16
  kolon; "Kiriş 1" `IfcBeamType` BEAM, 8 kiriş; üç `IfcSlabType` FLOOR; "Merdiven 1" `IfcStairType` HALF_TURN_STAIR; her
  tip projede bildirilmiş: "uyuyor". Dosya `fixtures/arch/v1/ifc/kesit-binasi-eleman-tipli.ifc`, notu yanında.
- **`.kcad`:** tip ve izleyen kolon (istisnasıyla) Mimari verisi 19 ile yazılıp aynı okunur.
- **Masaüstü** (`apps/desktop/src/tests/element_types_tests.rs`): kesit binasının 16 kolonu; Tipe çevir, Öznitelikler'in
  Tip listesiyle bağlama, birine kendi eni; pencerede tipin eni yazılınca plan "15 eleman yeni değerleri alır; 1 eleman
  değişen alanda kendi değerini korur" der, çizim değişmez; Uygula'dan sonra 15 kolon izler, biri korur; bir geri alma.
  Öznitelikler'de En "Örnekten", Boy "Tipten", İstisnalar "En"; Tipe döndür ve Tipten ayır; Aynı tiptekileri seç 15
  kolon. Kesişen süzgeçler (şerit, katalog, geçmiş yazısı, ayarlar, Öznitelikler, kapı tipleri) geçti (240).
- **Gerilemeler:** bütün komutların ortak durumları istisna kuralıyla geçti; yetenek tablosu her kodu bir durumla gösterir.
- **Resimler:** `.run/gui-evidence/eleman-tipleri-8106/` (Öznitelikler Tipten ve Örnekten, Tip bölümü, pencere
  önizleme ve uygulandı, bağlamsal sekmenin Tip paneli, Proje ▾'deki yeri; açık ve koyu); simgeler
  `.run/gui-evidence/ikon-eleman-tipleri/` (A).

## §5. Arayüz

- **Eleman tipleri** (Mimari › Proje ▾, ve kolon, merdiven ya da döşeme seçiliyken bağlamsal sekmenin Tip paneli): solda
  tipler (Tip, Tür, Eleman, İstisna, Eski); sağda seçili tipin alanları projenin biriminde (uzunluklar), seçmeli
  alanlar (Biçim, Tür, Dolgu, Korkuluk …) ve yalnız gösterilenler (katmanlar, kollar, kenar: bir elemanda değiştirilip
  Tipe uygula ile tipe verilir); Kullanım; Uygula'dan önce planın cümlesi. Yeni tip ▾ (türler ve Seçili elemandan), Sil
  (elemanlar değerleriyle kalır), Aynı tiptekileri seç, Uygula, Kapat.
- **Öznitelikler:** tip alanının satırı değerin kaynağını yazar: "Tipten" (sessiz) ya da "Örnekten" (vurgu rengi). Tip
  bölümü: Tip listesi (türün tipleri, Tipten ayır, Tipe çevir…, Eleman tipleri…), Tip sürümü (güncel ya da eski),
  İstisnalar (Aynı tiptekileri seç, Tipe uygula, Tipe döndür, Tipi yeniden uygula).
- **Simgeler:** `elementTypes`, `elementTypeFrom`, `elementSameType`, `elementDetach`, `elementPush`, `elementReset`.

## §6. Sıradaki adımlar

- **Giydirme cephe tipi:** duvar stilinin ızgarası (`curtain`) olarak, ve rampa.
- **Profil takımı:** kolon ve kiriş tipinin `IfcMaterialProfileSet`'i, elemanlarının `IfcMaterialProfileSetUsage`'ı.
- **Eski sürüm denetimi:** Mimari denetimde eski sürümdeki eleman (kapı tiplerinin `arch_opening_type_stale`'i gibi), Düzelt
  Tipi yeniden uygula.
- **Ofis kitaplığı:** tiplerin dosyayla taşınması (kapı tiplerininki gibi).
- **Pencerede katmanlar, kollar ve kenarın düzenlenmesi;** şimdilik bir elemanda değiştirilip Tipe uygula ile.
- **Gerçek pencerede kabul** (ADR 8000) ve Python'un yerel modülüyle sınama; web (TODOS.md §0.2).
