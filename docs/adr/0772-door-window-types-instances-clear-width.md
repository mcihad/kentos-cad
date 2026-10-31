# ADR 0772: Kapı ve pencere tipleri, örneğin istisnası ve net geçiş

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md §3 `ADV-03` (“Gerçek kapı/pencere tipleri, örnek kontrolü ve net kullanım”).
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 137 (opt-in; §7). `FORMATS_VERSION` değişmez (99).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0341'in açıklık türleri (`kentos-kapi-90` …) KentOS'un kendi tablosudur ve yerleştirirken örneğe kopyalanır: örnek
türünü izlemez, ofisin kendi tipi yoktur, türün değişmesi yerleşmiş kapılara gitmez. `width` tek bir sayıdır ve neyin
eni olduğu söylenmez: duvardaki delik mi, kasanın dış eni mi, kanat mı. Kapının bıraktığı net geçiş hiç hesaplanmaz.
Kanadın açılma alanı yalnız çizilir; mutfak dizisi ya da kolon onun içine girse kimse söylemez.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Vectorworks, Door settings: Configuration/General pane.** Üç ölçü ayrı ayrı verilebilir: “Leaf Size”, “Unit Size”,
  “Rough Opening”; aralarını “Shim gap” ve kasa doldurur. Kanatlar eşit olmayabilir (“unequal leaf”), kasanın derinliği ve
  duvardaki yeri (“Frame depth”, “insert relative to”), eşik (“threshold”), pervaz (“trim”) birer ayardır.
  <https://app-help.vectorworks.net/2025/eng/VW2025_Guide/Doors/Door_settings_General_pane.htm>
- **Vectorworks, Object styles:** bir stilin her ayarı “By Style” ya da “By Instance”tır; stil değişince “by style”
  değerler bütün örneklere gider, örneğin kendi değeri kalır.
  <https://app-help.vectorworks.net/2025/eng/VW2025_Guide/ObjectStyles/Concept_Plug-in_object_styles.htm>
- **Graphisoft Archicad, Nominal sizes:** kapı ve pencerenin ölçüsü “Wallhole”, “Unit”, “Reveal”, “Egress” ya da “Leaf”
  ile verilir; aralarındaki fark tolerans ve kasa eni (`gs_frame_width`) ile hesaplanır.
  <https://help.graphisoft.com/AC/27/INT/_AC27_Help/050_Elements/050_Elements-75.htm>
- **2010 ADA Standards for Accessible Design, 404.2.3 Clear Width:** “Door openings shall provide a clear width of 32
  inches (815 mm) minimum. Clear openings of doorways with swinging doors shall be measured between the face of the door
  and the stop, with the door open 90 degrees.” <https://www.access-board.gov/ada/#ada-404>

Yönetmelik değeri bu ADR'nin konusu değildir: net geçişin en az değeri bir proje kuralıdır, ofis seçer (§6).

## Karar

### 1. Projenin tipleri ve örnek

Projenin kapı ve pencere tipleri `settings.arch.openingTypes`'tadır (`OpeningType`: kimlik, ad, sürüm, kaynak, biçim,
kaba boşluk, kanat, açılış, açı, kasa, denizlik, lento, derinlik, yapı). Bir açıklık tipi izlerken `style` tipin
kimliği, `typeVersion` aldığı sürüm, `overrides` kendi değerini tuttuğu alanlardır (alanların bildirim sırasıyla, birer
kez). KentOS'un kendi türleri (`kentos-*`) yalnız kopyalanır, izlenmez; ofis tipi yeniden `kentos-` ile başlayamaz.

- `cad.openingtypes.edit` v1: Yeni tip (kopyadan ya da biçimin ilk KentOS türünden), Tipi değiştir (sürüm bir artar, tipi
  izleyenler aynı adımda alır), Yeniden adlandır, Sil (`detach` olmadan tipi kullanan varsa ret), Kitaplıktan al
  (`kentos.opening-types` 1; ad çakışmasında Ret, Atla, Değiştir ya da Yeniden adlandır) ve Kitaplığa yaz (çizim
  değişmez, dosya ev sahibinin). Tek geri alma adımı “Açıklık tipi” (`Op::OpeningTypes`).
- `cad.openings.edit`'in yeni işlemleri: Tipten ayır (değerler kalır, tip izlenmez), Tipe uygula (tek açıklığın kendi
  değerleri tipin olur, öbürleri izler), Tipe döndür (kendi değerleri bırakılır, son sürüm alınır), Tipi yeniden uygula
  (eski sürümdeki açıklık son sürümü alır, istisnaları korunur). Tür değiştir projenin tipine bağlar; `level` ile yalnız
  bir katta.
- Tipi izleyen açıklığın bir alanını elle değiştirmek o alanı istisna yapar (`opening_type_override` uyarısı); biçimi
  değiştirmek reddedilir (`opening_type_form`).

### 2. Dört en

`width` bugün olduğu gibi **kaba boşluktur** (duvardaki delik); anlamı sessizce değişmez. Yanında hesaplanan üç en vardır
(`opening_sizes`):

- Kasa dış eni = kaba boşluk − 2 · montaj boşluğu (`fit`).
- Kanat eni (ışık) = kasa dış eni − 2 · kasa.
- Net geçiş (90°): açılan kapıda kanat eni − Σ(kanat · cos θ + kalınlık · sin θ), θ = 90°'de kanadın kalınlığı
  (öntanımlı 0,04 m); iki kanatlıda iki kanat birlikte açık; sürmede bir kanatlı kanat eni, iki kanatlı kanat eni − bir
  kanat; sabit kapıda yok; boşlukta ışık; pencere, niş, bacada yok.

Tipin ölçüsü yapının `sizeBy`'ıyla adlanır (kaba boşluk, kasa dış eni ya da kanat eni; öntanımlı kaba boşluk). Kanat
eniyle ölçülen tipte kasa ya da montaj boşluğu değişince kanat aynı kalır, kaba boşluk büyür: “90'lık kapı” kanadı 90 cm
olan kapıdır ve kaba boşluğu 90 cm değildir. Çizelgenin yeni sütunları `roughWidth` (“Kaba boşluk”), `unitWidth` (“Kasa
dış eni”), `leafWidth` (“Kanat eni”), `clearWidth` (“Net geçiş”); eski `width` (“Genişlik”) kaba boşluktur. Etiketin
yerleri aynı adlarla ve `typeVersion`'la.

### 3. Yapı

Açıklığın isteğe bağlı `build`'i (`OpeningBuild`): ölçü referansı, montaj boşluğu, kanat kalınlığı, iki kanadın payı
(eşit olmayan kanatlar), kasa derinliği ve yüzden uzaklığı, eşik, pervaz, denizlik çıkıntısı, dikey ve yatay kayıt,
kullanım alanı. Plan simgesi (`geom::opening::symbol`) onları çizer: kasa montaj boşluğuyla ve derinliğinde, kalın kanat
bant, eşik, iki yanda pervaz (yüzden 1,5 cm), pencerede kayıtlar ve denizlik. Boş yapı yazılmaz; yapısız açıklık bayt
bayt eskisi gibi çizilir (ADR 0341'in bağımsız başvuru durumları değişmeden geçer). Yatay kayıt planda görünmez, yalnız
veridir.

### 4. Takip raporu

Tip değişince her izleyen açıklık dört sonuçtan birini alır (`OpeningTypeReport`):

- **izledi** (`applied`): yeni sürümü aldı ve yeni değerlerle yeniden yazıldı;
- **istisna korundu** (`kept`): değişen alanlardan kendi değerini tuttukları; açıklık yine yeni sürümü alır (öbür
  alanları izler) ve iki listede de görünür, alanları sayılır (`opening_type_kept`). K01'in kasası değişince 20 kapı
  yeni sürümü alır, 19'unun kasası değişir, istisnanın kasası 7 cm kalır;
- **atlandı** (`skipped`): kilitli katmanda, eski sürümde kalır (`opening_type_skipped`), denetimde
  `arch_opening_type_stale`, Düzelt Tipi yeniden uygula;
- **reddedildi** (`refused`): duvarından taşar, öbür açıklıkla çakışır ya da geçersiz olur. `onRefused: stop`
  (öntanımlı) bütün yazmayı reddeder (`opening_type_refused`, ilk sığmayanın kimliğiyle); `skip` sığmayanı eski
  sürümünde bırakır.

Önizleme komutun planıdır: pencere Uygula'dan önce sayıları söyler.

Aynı tiptekileri seç: `cad.openings.query` (salt okunur; kimlik, tip, kat ve biçimle süzülür, her satırda dört en, tip
başına sayılar). Masaüstü etkin katta ya da çizimde arar; seçimin kapsamı sorgunun `uids`'idir. Bağımsız bölüm
kapsamı ADV-04'ündür (sınır).

### 5. Açılma ve kullanım alanı

`swing_zones`: açılan kapının her kanadının süpürdüğü daire dilimi (menteşe ve 17 nokta). `use_zone`: yapının
`useZone`'u verilmişse ışık eninde, açıldığı yüzden o derinlikte dikdörtgen; planda kesik çizgiyle çizilir (isteğe
bağlı, görünür). Mutfak dizisi kolunun izi başka duvardaki kapının dilimine girerse `casework_clash` söyler.

### 6. Denetim

`cad.arch.audit`'in yeni grubu **Kapılar** (`doors`, yalnız adıyla sorulur; masaüstü bütün grupları sorar). Bulgu üç
türden birini taşır (`nature`):

- **fiziksel çakışma** (`physical`): kanadın dilimi başka duvara, kolona, donatıya ya da öbür kapının dilimine girer
  (`arch_door_swing_clash`). İleti neye çarptığını söyler; bulgunun nesneleri çizimde seçilir.
- **net geçiş** (`passage`): kapının kullanım alanına bir duvar, kolon ya da donatı girer (`arch_door_front_blocked`).
- **proje kuralı** (`rule`): `minClearWidth` verilince net geçişi daha dar olan kapı (`arch_door_clear_narrow`).

Eski sürümdeki tip (`arch_opening_type_stale`) Açıklıklar grubundadır; Düzelt Tipi yeniden uygula'dır.

Yüzeyde 1 mm'lik pay: değen yüz çakışma sayılmaz. İkinci bir denetim sistemi kurulmaz.

### 7. `.kcad` şema 137

Projenin tipleri, açıklığın `typeVersion`'u, `overrides`'ı, `build`'i ve çizelgenin dört genişlik sütunu şema 137'yi
seçer; hiçbiri yoksa dosya eski şemasıyla bayt bayt aynı yazılır. Şema 136 dosyasında bu alanlar `unknown_field`'dır.
Bağımsız Python okuyucusu ve yazıcısı (`tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`) aynı kuralla.

### 8. Masaüstü

- **Kapı ve pencere tipleri** penceresi (Mimari › Proje ▾ ve Açıklık bağlamsal sekmesinin Tip paneli): tipler sayılarıyla
  (açıklık, istisna, eski), seçili tipin değerleri projenin biriminde, ölçü referansı, Açılış; Uygula'dan önce planın
  sonucu (“Şimdi … Değişince …”, kaç açıklık izler, hangi istisna korunur); Yeni tip, Sil, Kitaplıktan al…, Kitaplığa
  yaz…. Kapat yazmaz.
- **Öznitelikler:** Tür listesinde önce projenin tipleri; Tip bölümünde sürüm (eski/güncel), istisnalar ve tip işlemleri,
  dört en.
- **Denetim:** Kapılar grubu ve Eşikler ▾'de “Net geçiş <” (Yok, 0,80, 0,85, 0,90 m).
- İkonlar: `openingTypes`, `openingSameType`, `openingDetach`, `openingPush`, `openingReset`
  (`.run/shots/adv03-ikon-secenekleri.png`'deki seçeneklerden).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/opening_types_cases.py` (KentOS kodu olmadan): `cad.openingtypes.edit` 43,
  `cad.openings.query` 23, `cad.openings.edit` tip işlemleri 26, `cad.arch.audit` Kapılar 22 durum. Kabulün senaryosu
  durumların içinde: K01'in 20 kapısı, kasası değişince 19'u izler, istisna korunur ve sayılır; kanadı 90 cm olan kapının
  kaba boşluğu 1,02 m (montaj 1 cm, kasa 5 cm).
- Geometri: `tests/all/opening_builds.rs` elle hesaplanmış ölçülerle (kanat 0,88 m, dilimin yarıçapı, kullanım alanının
  alanı, boş yapı eskisi gibi).
- Masaüstü: `opening_types_tests.rs` (pencerenin sayıları ve önizlemesi yazmadan, Uygula tek geri alma adımı, Kapat yazmaz,
  okunamayan değer söylenir, Aynı tiptekileri seç 20 kapı, Tipe döndür ve Tipten ayır seçimde); resimler
  `opening_type_scenes.rs` (iki tema, 1440×900 ve 1100×650).
- KCAD şema 137: örnekler ve beş bozuk dosya, bağımsız okuyucuyla.
- Etiket ve çizelge: `tag_command_cases.py` yeni yerlerle yeniden üretildi.
- Süreler: §Süreler.

### Süreler

Release, Apple M5, `cargo test --release -p kentos-native-application --test all opening_types_timing -- --ignored
--nocapture`: 1 000 kapılık koridor (K01'i izleyen 1 020 kapı, her onuncunun yanında kolon).

| İş | Süre | Bütçe |
|---|---|---|
| Tipi değiştir'in planı (önizleme) | 4,6 ms | 50 ms |
| Tipi değiştir'in yazması (çizimin kopyası dahil) | 16,8 ms | 100 ms |
| Aynı tiptekiler (`cad.openings.query`) | 0,1 ms | 20 ms |
| Kapılar denetimi | 9,4 ms | 100 ms |

Kapılar denetimi ilk ölçümde 365 ms'ydi (her kapı çifti kesin testle); kutuların önce karşılaştırılmasıyla 9,4 ms.

## Sınırlar

- Döner (pivot), cebe giren (pocket) ve katlanır kapı yok: sahte simge çizilmez.
- Bağımsız bölüm kapsamı (ADV-04) açık; masaüstünde Aynı tiptekileri seç etkin katta ya da çizimde arar.
- Yatay kayıt yalnız veri; kesit ve görünüş ADV'nin başka işi.
- PDF: etiket ve çizelge değerleri ortak hesaptan gelir; PDF'in metni bu dilimde ayrıca okunmadı.
- Ofis kitaplığı dosya olarak taşınır (ev sahibi okur ve yazar); kitaplık klasörü ve sunucu yok.
- Gerçek pencerede elle denenmedi; resimler pencere açmadan.
