# 3012 — Kesme yüksekliği ve üstteki çizgiler

Tarih: 2026-11-08. Durum: kabul (M13, ADV-15; plan S4). Üç adımda yapıldı.

## Sorun

- **Pencere her zaman kesilmiş çiziliyordu.** Denizliği 1,60 m olan bir banyo vasistası da planda duvarı kesiyor ve düz
  çizgiyle basılıyordu.
  - Planın kesme yüksekliği 1,10 m'dir (ADR 0719). Boşluk, niş ve baca yalnız bu yüksekliği geçiyorsa kesiliyordu;
    pencere ve kapı her durumda kesiliyordu.
  - Ölçüm (1:50 PDF, `headless/scales.rs`): denizliği 0,9 ve 1,6 olan iki pencere duvarı iki yerden kesiyordu. Duvar
    çizgisi (0,5 mm) 9 yol, pencere çizgileri 0,25 mm'de 10 yol, kesikli yol hiç yok.
- **Kesme yüksekliği sabitti** (1,10 m, `opening.rs`): ofisin ya da projenin kendi değeri verilemiyordu.
- **Üstteki çizgiler yoktu.** Üst katın çıkması, kiriş izi ve asma tavan alt katın planında kesikli görünmüyordu.

## Araştırma (8 Kasım 2026)

- **İTÜ, Ecem Edis, *Teknik Çizim İlkeleri*** (2013, s. 1): "Bir binada herhangi bir katın plan çiziminde, çizimin kat
  döşemesinin yaklaşık 1,20-1,50 m üzerinden yapıldığı kabul edilir. Bu kabul, kattaki pencere boşluklarının
  yerlerinin gösterilmesine olanak tanımaktadır." (`.run/gui-evidence/mimari-m13/referans/`)
- **TMMOB Mimarlar Odası İstanbul Büyükkent Şubesi, *Mimari Proje Çizim ve Sunuş Standartları*** §14.5.3, uygulama
  projesi planları:
  - o) "Tavandaki kirişlerin sarkıntıları, nervür ve kasetler nokta nokta (ifade edecek kadar) gösterilir."
  - u) "Planın geçtiği düzlem ile tavan arasında kalan imalat nokta nokta işlenir. (Saçak ara kat çıkma vb.)"
- **Archicad 17 yardımı, *Show On Stories*:** döşeme kendi katında, bir üst ve bir alt katta ya da bütün katlarda
  gösterilebilir ("Home & One Story Down"). "The element's outline on remote stories will be displayed using the line
  type chosen in Options > Project Preferences > Construction Elements"; o ayarın "Below Home Story" değeri kesikli
  çizgidir (`referans/archicad17-ShowOnSlab.png`, `archicad17-ShowLineType.png`).
- **Revit 2023 yardımı, *About the View Range*:**
  - "The horizontal planes that define the view range are Top, Cut Plane, and Bottom."
  - Kesme düzleminin üstündeki öğeler için: "These elements are not displayed in a plan view unless their categories
    are window, casework, or generic model."
  - Pencere kesme düzleminin üstündeyse kesilmez, görünen çizgiyle çizilir (`referans/revit-GUID-8B8DB6E2…png`,
    `revit-GUID-68FEDE8B…png`).
  - Metrik şablonlarda kesme düzlemi 1200 mm'dir (Autodesk Community; resmî değer değil).
- **2026 Dijital Hazırlama Yönetmeliği** EK-4 4.22 → TS EN ISO 128-2; ISO 128-23 Tablo 1: kesikli ince çizgi (02.1)
  gizli ve görünmeyen çizgiler içindir.

## Karar

### Adım 1: kesme düzleminin üstündeki pencere

- **Pencere de yüksekliğine göre kesilir** (`OpeningForm::cut_by_height`, `geom::opening::cut_in_plan`):
  - denizliği kesme yüksekliğinin altında, lentosu üstünde olan pencere kesilir ve düz çizilir;
  - bütünüyle üstte kalan pencere (vasistas, yüksek pencere) duvarı kesmez; bütün parçaları kesikli çizilir.
  - Kapı yerden başladığı için her zaman kesilir.
  - Yüksekliği verilmemiş pencere yerden tavana sayılır ve eskisi gibi kesilir. Hazır pencere tiplerinin denizliği
    0,9 m'dir; onlar değişmedi.
- **Panel satırları ayrı bir yoklamaya bağlandı** (`OpeningForm::hole_or_recess`): açıklık aracının Yüz, Alt ve Üst
  satırları yalnız boşluk, niş ve bacada çıkar. `cut_by_height` pencereyi kapsayınca pencere panelinde nişin "Yüz"
  satırı belirecekti.
- **Bilgi satırları pencereyi de söyler:** aracın yerleştirme bilgisi ve Öznitelikler'in "Planda" satırı ("Kesilir" /
  "Kesik çizgi") artık pencerede de görünür.
- **Çakışma kuralı** (ADR 0719 §3) aynıdır ama sonucu değişir. Kesmenin üstündeki pencerenin altına, kesmeyi geçen bir
  niş konabilir: yükseklikleri çakışmaz ve plan yalnız nişi keser.
  - Başvuru betiği (`scripts/fixtures/opening_command_cases.py`) aynı kurala geçti.
  - Bir durum yeniden üretildi: "yüksek pencerenin altında kesmeyi kesen niş" artık eklenir (önce reddediliyordu).

### Adım 2: kesme yüksekliği projenin ayarı

- **Mimari ayarına `cutHeight` eklendi** (`ArchSettings::cut_height`, katın döşemesinden metre; `.kcad` şema 217).
  - Yoksa plan eskisi gibi 1,10 m'den kesilir (ADR 0719): eski çizimler değişmez.
  - Yeni Mimari projesi 1,20 m yazar (`NEW_PROJECT_CUT_HEIGHT`). İTÜ'nün 1,20–1,50 m aralığının alt ucudur; Revit'in
    metrik şablonu da 1200 mm'dir. Alt uçta denizliği 0,9–1,1 m olan olağan pencereler kesilir, denizliği 1,50 m ve
    üstündeki vasistaslar kesikli kalır.
  - Aralık 0,30–3,00 m (`CUT_HEIGHT_RANGE`): kesme düzlemi katın içinde kalır (olağan kat yüksekliği 3 m). Metre
    yerine santimetre yazılırsa (120) ayar reddedilir.
- **Tek kaynak `ProjectSettings::plan_cut()`.** Kesme yüksekliğini okuyan her yer onu sorar:
  - duvarın delikleri (`opening::holes_at`) ve açıklığın çizimi (`wall::View::cut`, `opening::pieces_at`);
  - DXF yazıcısı (`DxfWriteInput::cut_height`);
  - açıklık aracının yerleştirme bilgisi, Öznitelikler'in "Planda" satırı ve açıklığın duvarı izlemesi;
  - Merdiven aracı: K seçeneği verilmezse merdivenin kesme çizgisi projenin yüksekliğinden geçer.
- **Değişince bütün duvarlar yeniden kesilir.** Depo görünüşün kesme yüksekliği değişince bütün duvarları kirli sayar
  (`Store::set_wall_view`); ayrıntı düzeyi değiştiğindeki gibi.
- **Proje ayarları › Mimari'de "Planın kesme yüksekliği" alanı.** Virgül de nokta da kabul edilir. Boş bırakılırsa
  1,10 m'dir; aralık dışı ya da sayı olmayan değerde Kaydet beklemede kalır. Açıklama hangi nesnelerin kesildiğini ve
  İTÜ'nün aralığını söyler.
- **Betik ve ajan:** `cad.settings.set`'in `arch` girdisi `cutHeight`'ı alır; komutun özeti onu anar.

### Adım 3: üst katın döşemesi alt katın planında kesikli

- **Bir üst katın döşemeleri etkin katın planında kesikli çizilir** (`level_view::shows_below`, `LevelLooks::overhead`):
  - "Yalnız etkin kat" ve "Etkin kat ve alt kat" görünümlerinde; "Bütün katlar"da her döşeme kendi planındaki gibidir.
  - Yalnız hemen üstteki kat (`level_view::above`): Archicad'in "Home & One Story Down"ı. En üst katın üstü yoktur.
  - Yalnız döşeme (`Entity::Slab`: döşeme, balkon, teras). Kiriş ve asma tavan nesnesi henüz yok.
- **Görünüşü:** döşemenin halkaları (dış çizgisi ve delikleri) kesikli ve ince, nesnenin kaleminin yarısı
  (`slab::OVERHEAD_PEN`, ISO 128-23'ün kesikli ince çizgisi 02.1). Korkuluk ve parapet çizilmez: onlar üst katın
  planına aittir.
- **Bütün döşeme çizilir, yalnız çıkma değil.** Alt katın duvarlarının içinde kalan kısım duvarın çizgisiyle çakışır ya da
  tavanın sınırını gösterir; çıkma (balkon, konsol) duvarın dışında kesikli okunur. Çıkmayı alt katın duvarlarına göre
  kesip ayırmak her duvar değişiminde yeniden hesap ister; Archicad da döşemenin bütün dış çizgisini çizer.
- **Seçilmez, kenetlenir:** alttaki katın merdiveni gibi (ADR 0724 §4). Tıklama etkin katın nesnesini alır.
- **Pafta:** kat paftası (`cad.sheets.levels`) aynı kuralla basılır; harita kendi kat görünümüyle kendi deposunu kurar
  (`layers::shows_from_above`).
- **Durum çubuğu:** Kat hücresinin ipucu "üst katın döşemeleri kesikli" der.
- **Ekran önbelleği:** çizim alanının biçemli sahnesi belge ya da görünüş (gizli, soluk) değişince yeniden kuruluyordu.
  "Bütün katlar"a geçiş zemin katta hiçbir gizli ya da soluk nesneyi değiştirmediği için balkon ekranda kesikli
  kalıyordu (gerçek pencerede görüldü; depo doğruydu). Başka kattan görünen nesneler artık görünüşe not edilir
  (`Phasing::drawn_across`): yalnız onlar değişse de sahne yeniden kurulur. Aynı açık alttaki katın merdiveninde de
  vardı; orada başka nesneler de gizlendiği için örtülüyordu.

## Ölçüm

- **Adım 1, 1:50 PDF** (`headless/scales.rs::a_window_over_the_cut_leaves_its_wall_whole_on_paper`):
  - Aynı duvarda denizliği 0,9 ve 1,6 olan iki pencere çizildi, pafta PDF'e basılıp geri okundu.

    | | Duvar 0,5 mm | Pencere 0,25 mm düz / kesikli | 0,125 mm düz / kesikli |
    |---|---|---|---|
    | Önce | 9 | 10 / 0 | 2 / 0 |
    | Sonra | 5 | 6 / 4 | 1 / 1 |

  - Duvar yalnız alçak pencerede kesilir. Yüksek pencerenin kasa kesitleri ve camı kesikli çizilir; yüzleri duvarın
    yüzleriyle çakışır.
  - Resim: `.run/gui-evidence/mimari-m13/m13-pencere.png` (solda önce, sağda sonra).
- **Testler:** `opening_builds::a_window_over_the_cut_leaves_its_wall_whole_and_is_drawn_dashed`; başvuru
  fikstürleri ve `cad.openings.create` durumları.
- **Adım 2, 1:50 PDF** (aynı test, ardından `cad.settings.set` ile kesme 1,80):

  | Kesme yüksekliği | Duvar 0,5 mm | Pencere 0,25 mm düz / kesikli |
  |---|---|---|
  | 1,20 (yeni proje) | 5 | 6 / 4 |
  | 1,80 | 9 | 10 / 0 |

  - Kesme 1,80'e çıkınca denizliği 1,60 olan pencere de kesilir: duvar iki yerden açılır, kesikli çizgi kalmaz.
- **Adım 2, gerçek pencere** (`fixtures/interaction/real/kesme-yuksekligi.txt`, gerçek fare ve klavye): aynı çizim
  dosyayla açıldı. 1,20'de yüksek pencere kesikli; Proje ayarları › Mimari'de alana 1,8 yazılıp Kaydet'e basılınca
  ikisi de kesik. M13'ten önceki derlemede alan yok ve iki pencere de kesik. Resim:
  `.run/gui-evidence/mimari-m13/m13-gercek-kesme.png`.
- **Adım 2 testleri:**
  - `contracts::arch::plan_cut_tests` (aralık, JSON);
  - `store/walls.rs::a_higher_cut_cuts_a_high_window` (kesme değişince duvar yeniden kesilir);
  - `kcad/tests/all/plan_cut.rs`;
  - Proje ayarları alanı: `the_cut_height_is_read_as_typed_and_held_to_its_range`;
  - yeni proje: `new_project.rs`.
  - Başvuru fikstürleri: `plan-cut.kcad`, şema 216'da `broken/plan-cut-in-schema-216.kcad`, aralık dışı
    `broken/plan-cut-out-of-range.kcad` (`tools/kcad/kcad.py` bağımsız okuyucusu da aynı sonucu verir).

- **Adım 3, 1:50 PDF** (`headless/scales.rs::an_upper_slab_shows_dashed_on_the_plan_below_it`): zemin katta 8 × 6 m
  duvar halkası, 1. katta 3 × 1,5 m korkuluklu balkon (kalem 0,35 mm), `cad.sheets.levels` ile iki pafta.

  | | Zemin kat: kesikli 0,175 mm | 1. kat: düz 0,35 mm |
  |---|---|---|
  | Önce | 3 (paftanın kendi çizgileri) | 8 |
  | Sonra | 4: balkonun halkası | 8 |

  - Resim: `.run/gui-evidence/mimari-m13/m13-cikma-pdf.png`.
- **Adım 3, gerçek pencere** (`fixtures/interaction/real/ust-kat-doseme.txt`): çizim dosyayla zemin katta, "Yalnız etkin
  kat" ile açıldı. Önce balkon görünmüyordu; sonra ince kesikli. Kesikli kenara tıklayınca hiçbir şey seçilmez; durum
  çubuğunun Kat hücresinden "Bütün katlar"da balkon düz ve korkuluklu, aynı tık onu seçer; 1. katta kendi planındaki
  gibi. Resim: `.run/gui-evidence/mimari-m13/m13-gercek-cikma.png`.
- **Adım 3 testleri:** `interaction/tests/all/overhead_slab.rs` (görünümler, seçilmez/kenetlenir, düzenlemeden sonra),
  `arch_levels::a_slab_edge_is_its_outer_line_at_1_200` (üstten görünüş yalnız halka), masaüstünde
  `a_slab_of_the_level_above_is_dashed_until_every_level_shows`, stil
  `a_level_view_that_only_draws_across_still_changes_the_look`.

## Sonraki adımlar

- Kiriş ve asma tavan nesnesi geldiğinde onlar da kesikli çizilir (§14.5.3 o ve t).
- Web: `io/kcad.ts` şema 217, Proje ayarları › Mimari'de kesme yüksekliği ve üst katın döşemesinin kesikli çizimi
  (TODOS.md).
