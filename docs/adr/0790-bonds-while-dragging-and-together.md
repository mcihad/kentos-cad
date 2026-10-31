# ADR 0790: Sürüklerken yoldaki bağlar, Duvarı kaydır ve birlikte kaldırılınca geçen bağlar

- **Durum:** kabul edildi (31 Ekim 2026). Kapsam: `plan/architecture-advanced.md` ADV-01'in ADR 0780'de açık kalanları
  (tutamaçla sürüklemede ve Kaydır'da yerinde vurgu, iki bağın birlikte kaldırılmasının önerisi); madde tek parçada
  (ADV-01c).
- **Bağlam belgesi:** ADR 0780 (yoldaki bağlar, `CommandError.bonds`, `frees`), 0770 (mimari bağlar, Kaydır, Mesafe,
  Boy), 0710 (duvarın geçici ölçüsü, `cad.walls.edit` `shift`), 0701 (açıklığın geçici ölçüsü), 0068 ve 0280
  (tutamaçlar, Esnet), 0341 §5 (Açıklığı taşı).
- **Web:** ertelendi (ADR 0228); satırı “Belgelere eklenecek satırlar”da.
- **Şema:** `.kcad` değişmez. `FORMATS_VERSION` değişmez.

## Bağlam

ADR 0780'den sonra iki eksik kalmıştı:

1. Yoldaki bağlar yalnız Duvarın mesafesi'nin yazı kutusunda yerinde işaretleniyordu. Bir duvarın tutamacı sürüklenirken,
   açıklık taşınırken, Boy ya da açıklığın mesafesi yazılırken bağların reddi ancak bırakınca, kırmızı bir iletiyle
   geliyordu; önizleme (`cad.entities.edit` ve `cad.openings.edit`'in planı) bağları hiç izlemiyordu. Kaydır'ın
   (`cad.walls.edit` `shift`) masaüstünde aracı yoktu: duvarı imleçle kendine dik sürüklemenin yolu yalnız sayı yazmaktı.
2. `frees` yalnız tek bir bağın çıkarılmasını deniyordu. Aynı duvara iki sabit bağ, ya da sabit bir duvar ile onu iten bir
   uzaklık bağı gibi durumlarda ileti “Tek başına kaldırılınca düzenlemeyi geçiren bağ yok” deyip duruyordu; kullanıcı
   hangi ikisini kaldıracağını kendisi bulmalıydı.

Araştırma (üretici belgeleri ve forumları; yerelde çalıştırılmadı):

- Revit'in “Constraints are not satisfied” uyarısı düzenlemeyi durdurur; kullanıcılar pencerenin “kısıtı kaldır” ya da
  “değişiklikten vazgeç” seçeneklerini ve Expand ile ihlal edilen kısıtların listelendiğini, birinin seçilince görünümde
  vurgulandığını anlatır ([Autodesk forumu](https://forums.autodesk.com/t5/revit-architecture-forum/revit-error-constraints-are-not-satisfied/m-p/8247921),
  [AUGI](https://forums.augi.com/archive/index.php/t-85945.html); resmî yardımda düğme adları yazılı bulunamadı). Revit
  kaldırırken **ihlal edilen bütün kısıtları** kaldırır; en küçük kümeyi önermez. Kısıtı bulmak için Reveal Constraints
  önerilir (ADR 0780'deki kaynak).
- Revit'te sürükleme sırasında kısıtlı eleman kısıtın izin verdiği kadar gider; çelişki bırakınca uyarıyla söylenir
  ([Autodesk forumu](https://forums.autodesk.com/t5/revit-api-forum/how-to-catch-and-close-the-warning-constraints-are-not-satisfied/td-p/9260756)).
- Archicad kalıcı ölçü bağı tutmaz (ADR 0770); sürüklemede çelişki yoktur.

KentOS kararı: Revit'in iki adımını yine tek yere toplarız ve bir adım öne alırız: çelişki **sürükleme sürerken**
önizlemede, bağlar yerinde işaretlenerek söylenir (bırakmayı beklemeden). Kaldırma Revit gibi hepsini değil, yazmayı
geçiren **en küçük kümeyi** önerir: önce tek bağ (ADR 0780), yoksa iki, yoksa üç bağ. Öneri bir düğmedir ve var olan
`cad.constraints.edit` `remove`'u tek geri alma adımıyla yazar. Otomatik kaldırma, tolerans büyütme, sessiz koparma yok.

## Karar

### 1. Planlar bağların reddini söyler; sürüklemede yerinde vurgu

- `cad.entities.edit` ve `cad.openings.edit`'in planı (ve masaüstünün önizlemelerinin okuduğu `plan_in`), değiştirdiği
  nesneleri bir mimari bağ anıyorsa komutu çizimin okuma kopyasında bütünüyle yürütür (`constraint_follow::bonds_refusal`:
  `call::recorded`'ın adımı, kısıtlar onun içinde izler). Kopya `constraint_conflict`'i bağlarıyla reddederse plan aynı
  retle döner; başka bir ret ya da başarı planın eski cevabını değiştirmez. Bağı olmayan çizimde ve bağın anmadığı
  nesnelerde ek iş yoktur. `cad.walls.edit`'in planı Kaydır, Mesafe ve Boy'da bunu ADR 0780'den beri yapıyordu.
- Masaüstünün ortak parçası `kentos_interaction::bond_refusal::Refused`: reddin kartı (`wall_distance::refusal_lines`) ve
  bağların yerinde işareti (`wall_distance::bonds_in_way`: 3 px kalın duvar çizgisi, halkalı pervaz, parçalar arası kesikli
  çizgi, “× d1 — kaldırılınca geçer”, tehlike renginde). Onu kullananlar:
  - **Tutamaçla sürükleme** (Esnet, tek ya da Shift ile birden çok sıcak tutamaç): her imleç hareketinde yazılacak
    `cad.entities.edit` `grip` (`select::grip_input`, bırakınca yazılanla aynı) planlanır; açıklığın merkez tutamacı
    `cad.openings.edit` `rehost`'u planlar. Ret kartı imlecin yanında, bağlar yerinde.
  - **Duvarı kaydır** (yeni araç, `tool.wallShift`, Mimari › Düzenle ▾): duvarın düz kenarına tıklanır, kenar imleçle
    kendine dik kayar; önizleme `cad.walls.edit` `shift { edge, by }`'ın planıdır (etki kartı: iki yanın net mesafesi,
    mahallerin alanı, izleyen duvarlar; ya da bağların reddi yerinde). İkinci tık ya da yazılan uzaklık (kenarın soluna
    artı) tek adımda yazar; araç sonraki kenarı bekler. Esc kenarı bırakır, kenarsız Esc çıkar.
  - **Boy** (duvarın kenar boyu yazı kutusu) ve **açıklığın mesafesi** (ADR 0701'in yazı kutusu): yazıldıkça planlanır,
    ret kartı alanın altında, bağlar yerinde. Boy'un kartı başarıda etkiyi de söyler.
  - **Açıklığı taşı**: önizlemenin reddi bağlıysa aynı kart ve vurgu.

### 2. Birlikte kaldırılınca geçen bağlar (`freesWith`)

`BondInTheWay`'in yeni, isteğe bağlı alanı `freesWith` (boşsa yazılmaz): hiçbir bağ tek başına yazmayı geçirmiyorsa, bu
bağla birlikte kaldırılınca geçiren en küçük kümenin **öbür** bağlarının kimlikleri. Kümenin her üyesi öbürlerini sayar.

Arama (`arch_bonds::smallest`, en çok `SET_TRIES` = 48 küme):

1. ADR 0780'deki gibi çelişen her bağ tek başına çıkarılıp yeniden çözülür; hiçbiri geçirmiyorsa her birinin
   **yerine çıkan çelişkisi** saklanır (çıkarılınca hâlâ çelişenler).
2. İkililer: çelişen her `a` için (reddin sırasıyla), onsuz hâlâ çelişen her `b` ile `{a, b}` denenir (aynı küme bir kez).
   İlk geçiren seçilir.
3. Üçlüler: denenen her ikili için (denendiği sırayla), onsuz hâlâ çelişen her `c` ile `{a, b, c}` denenir. İlk geçiren
   seçilir.
4. 48 küme denendiyse ya da üçlü de yoksa öneri yoktur (ileti ADR 0780'deki “hiçbiri”).

Çelişki bağların denklemlerinden geldiyse denemeler denklemlerle sınırlıdır, kaydırmalar yazılırken geldiyse bütündür
(ADR 0780 §1'in kuralı). Küme, ilk çelişkinin dışında bağ içerebilir (sabit koridor duvarını kaydırmak önce yalnız onun
sabitiyle çelişir; sabit kalkınca koridor bağı sabit dış duvarı iter): reddin `bonds`'u önce çelişenleri, sonra kümenin
dışarıdaki üyelerini verir; `uids` yalnız çelişenlerdir (ADR 0253 §7'nin anlamı değişmez).

İleti:

- “Tek başına kaldırılınca düzenlemeyi geçiren bağ yok; şu iki bağ birlikte kaldırılınca geçer: Sabit, d1 (Uzaklık). Ya da
  kısıtları gevşeterek düzenleyin.” (üçlüde “şu üç bağ”). Tek bağ geçiriyorsa ADR 0780'in iletileri aynıdır.
- `cad.constraints.edit` ve `cad.parameters.edit`'in çelişkisi bir bağ ya da küme geçiriyorsa ADR 0780'in sözünü kullanır
  (`constraint_solve::bonds_free`).

Kod tablosunun `constraint_conflict` sözü `freesWith`'i anar. Python: `BondInTheWay.frees_with` (`from_json`/`to_json`,
`freesWith`); kaldırmak `ConstraintEditRemove(ids=[bond.id, *bond.frees_with])`. MCP reddin `structuredContent`'inde
olduğu gibi taşır.

### 3. Masaüstü: reddin kartı ve “Bu iki bağı kaldır”

Bir yazma bağlarla reddedilince (`points::written`, araçların ortak yazma yolu) ret `Context.refused`'a, oradan
uygulamanın `bonds_refused`'ına geçer ve çizimin üstünde bir kart açılır (`bond_notice.rs`): ilk satırı ileti, sonra her
bağ (“• Sabit (Sabit) — d1 ile birlikte kaldırılınca geçer”, “• d1 (Uzaklık): 1.200 m tutuyor, düzenleme 1.500 m yapar —
kaldırılınca geçer”) ve düğme: tek bağ geçiriyorsa “Bağı kaldır: d1”, küme geçiriyorsa “Bu iki bağı kaldır” / “Bu üç bağı
kaldır”, hiçbiri yoksa düğme yok; Kapat. Düğme tek `cad.constraints.edit` `remove` yazar (parametreleriyle; tek geri alma
adımı) ve “2 bağ birlikte kaldırıldı; düzenlemeyi yeniden deneyin.” der. Kart açıkken bağlar yerinde işaretli kalır
(etkin aracın önizlemesi kendi işaretlerini çizmiyorsa ona eklenir). Geçen bir yazma ya da Kapat kartı kapatır.

### 4. Bütçe (20 ms)

`arch_bond_timing` (release, `--ignored`) ADR 0780 §4'ün çizimlerine ekler: en alttaki duvarın iki ucu birlikte
sürüklenince (Esnet) `cad.entities.edit`'in planı — bağlarla izleyen ve en üst duvar sabitken reddedilen — ve en alttaki
duvar iki kez sabitken (ekseni ve sol yüzü) kaydırmanın reddi (ikililer denenir). Apple M5, release, ortanca:

| Çizim | Tutamaçla sürükleme önizlemesi | Reddedilen sürükleme | Hiçbiri tek başına geçirmeyen ret (ikili) |
|---|---|---|---|
| 200 duvar, 50 bağ | 1,67 ms | 0,45 ms (50 bağ yolda) | 0,19 ms (2 bağ birlikte) |
| 400 duvar, 100 bağ | 6,11 ms | 1,92 ms (100 bağ yolda) | 0,38 ms (2 bağ birlikte) |

Aynı koşuda ADR 0780'in ölçüleri: kaydırmanın önizlemesi 2,26 / 6,41 ms, yazma 3,56 / 12,64 ms, reddedilen önizleme
0,52 / 2,02 ms. Bütçe her durumda tutuyor; masaüstünde her imleç hareketinde bir plan yürür (sürükleme ve Duvarı kaydır),
400 duvarda en çok ~6 ms. Kümelerin aranması 48 denemeyle sınırlıdır: denklem çelişkisinde her deneme bir yok etmedir
(400 duvarda bağ başına ~0,02 ms; 48 deneme ~1 ms), kaydırmalar yazılırken gelen çelişkide (kilitli katman, birleşim)
her deneme bütün bir çözümdü; ADR 0797 ölçtü (400 duvarda 366 ms) ve denklemlerle süzerek 22 ms'ye indirdi.

## Sınırlar

- Arama en çok üç bağlı kümeye ve 48 denemeye bakar; daha büyük kümede “hiçbiri” denir, çelişen bağlar yine `bonds`'tadır.
  Kaydırmalar yazılırken gelen (geç) çelişkide denemeler bütün çözümdür; 400 duvarda en kötü 48 deneme bütçeyi aşabilir,
  ölçülmedi.
- En küçük küme ilk bulunandır (reddin sırasıyla); eşit büyüklükteki öbür kümeler söylenmez.
- Planın bağ reddi yalnız bağın andığı nesneleri değiştiren yazmalarda sorulur; bağın anmadığı bir nesnenin yazması bir
  bağı dolaylı olarak etkilerse (yoksa da) execute'un cevabı geçerlidir.
- Özellikler panelinden yazılan değerlerin reddi kartı açmaz (yalnız ileti); kart araçların yazmasında açılır.
- Gerçek pencerede elle denenmedi.

## Uygulama

- Sözleşme: `BondInTheWay.frees_with` (`constraints.rs`), kod tablosunun sözü.
- Uygulama: `arch_bonds::solve` (`smallest`, `SET_TRIES`, `Solved.frees_with`), `constraint_solve` (kümenin dışarıdaki
  üyeleri `bonds`'a, `bonds_words`, `bonds_free`), `constraints_edit`, `constraint_follow::bonds_refusal`, `edit::plan_in`,
  `openings_edit::plan_in`.
- Etkileşim: `bond_refusal` (`Refused`, `way_through`, `remove`), `wall_distance::refusal_lines`/`bonds_in_way`
  (`freesWith`'i adlandırır; `bonds_in_way` belgeyi alır), `wall_shift` (Duvarı kaydır), `select` (`grip_input`, sürükleme
  reddi), `select_opening::preview`, `opening_tools` (Açıklığı taşı), `opening_distance` ve `WallLength`'in
  `text_draft`'ı, `Context.refused`, `points::written`.
- Masaüstü: `bond_notice.rs` (kart, `drawing_preview`), `traces` (`bondsInWay`, `bondNotice` beklentileri; `bondNotice`
  adımı), `shortcuts.rs` (iki fare, bir tuş satırı), sahneler `bond_drag_scenes.rs`.
- Katalog: `catalog/v1/additions.json` (`tool.wallShift`), `ribbon.json` (Mimari › Düzenle ▾), `operations.json`
  (`semantic`, `cad.walls.edit`).
- Python: `kentos.cad.errors.BondInTheWay.frees_with`.

## Doğrulama

- Bağımsız başvuru (KentOS kodu olmadan): `scripts/fixtures/arch_bond_conflict_cases.py` → yeni
  `cad.walls.edit.bondsets.json` 23 durum (aynı duvara iki ve üç sabit: birlikte, sonra birlikte kaldırılınca kayar; sabit
  koridor duvarı + koridor bağı + sabit dış duvar: küme ilk çelişkinin dışından üye alır, planı da, Mesafe'si de, iki bağ
  kaldırılınca kayar; sabit ikinci kapı: açıklığı taşımak ve planı; açıklığı taşımanın planı bağlarla reddedilir; tutamaçla
  koridor duvarını kaydırmak ve planı; tutamaçla ucu çekmek duvarı döndürür; tutamaçla sabit duvar; bağsız duvarda ve
  duvar boyunca uzatmada plan geçer; bağ kaldırılınca tutamaç planı geçer; iki sabitli zincirin ortası; parametrenin ve
  yeni bağın reddi tek bağla; kilitli katmanda başvuru yolda değil; eşit aralık + iki sabit) ve `cad.walls.edit.bonds.json`'un
  “aynı duvara iki sabit” durumu artık `freesWith`'i ve yeni sözü bekler. Kural `arch_bond_cases.py`'nin yeni `without`,
  `smallest`, `in_the_way`, `bonds_words`'ü (SET_TRIES = 48). `python3 scripts/fixtures/arch_bond_cases.py --check` (31,
  değişmedi). Sahne `scripts/fixtures/bond_drag_scene.py` (bağlarıyla KCAD v2, `kcad_v2_reference.py`'nin bağımsız
  yazıcısıyla; `bonds-drag.kcad`, `tools/kcad/kcad.py validate` geçerli).
- Rust: `KENTOS_CASES_ONLY=cad.constraints,cad.parameters,cad.walls,cad.openings,cad.entities.edit cargo test -p
  kentos-native-application --test all every_command_case` ve bütün ortak durumlar `--test all -- every_command_case
  capabilities catalog` (geçti); `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts -p kentos-commands` (katalog ve TS
  türleri yeniden yazıldı: `BondInTheWay.ts`, `commandCatalog.json`); `cargo test -p kentos-interaction` (bütün takım).
- Masaüstü: `cargo test -p kentos-desktop --lib -- bond every_trace_passes shortcuts history opening_distance
  wall_distance` (36; bütün izler üç çeşitte, yeni `wall-bond-drag.json` dahil: Duvarı kaydır'da imleç hareket ederken
  `bondsInWay`, reddedilen tıkta `bondNotice`, tutamaç basılıyken sürüklemede yerinde vurgu, bırakınca kart, Kapat, “Bu iki
  bağı kaldır”, sonra kaydırma geçer ve tek geri alma; `bond_drag_scenes::tests` 3). İzler artık KCAD v2 belgeyi de açar.
- Resimler `KENTOS_SHOTS_ONLY=bag-kaydir-onizleme,bag-tutamac-surukle,bag-birlikte,bag-birlikte-kart,bag-birlikte-kaldirildi
  cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture`: `.run/shots/arac-bag-*`
  (1440×900 ve 1100×650, koyu ve açık). 1100×650'de ret kartı odanın üstüne düşer; vurgu 1440'ta okunur.
- Bütçe: `cargo test --release -p kentos-native-application --test all arch_bond_timing -- --ignored --nocapture` (§4).
- Python: worktree'nin `.run/py`'sine maturin ile yerli modül; `KENTOS_CASES_ONLY=cad.constraints,cad.walls,cad.parameters,
  cad.openings,cad.entities.edit python -m unittest python.tests.test_command_cases python.tests.test_protocol
  python.tests.test_catalog` (18, `BondInTheWay.frees_with` gidip gelir). `python3 scripts/python/sdk.py --check`,
  `python3 scripts/cad/manifest.py --check` (938 satır, `tool.wallShift` semantic), `python3 scripts/ui/ribbon.py --check`.
- Clippy `-D warnings --all-targets`: kentos-contracts, kentos-interaction, kentos-desktop; kentos-native-application'da bu
  dilimin dosyaları temiz (aynı crate'te `modules_more.rs`'in iki uyarısı ana daldan, dokunulmadı).
- Gerçek pencerede elle denenmedi; MCP'nin testleri koşulmadı (MCP reddi olduğu gibi taşır, kodu değişmedi).

## Belgelere eklenecek satırlar

**CLAUDE.md §0** (ADR 0780'in maddesinin hemen altına):

> - **Sürüklerken yoldaki bağlar ve birlikte kaldırılınca geçen bağlar (31 Ekim 2026; [ADR 0790](docs/adr/0790-bonds-while-dragging-and-together.md), ADV-01 [~]).**
>   `cad.entities.edit` ve `cad.openings.edit`'in planı, bağın andığı nesneyi değiştirdiğinde komutu okuma kopyasında yürütür
>   ve bağların reddini execute gibi verir (`constraint_follow::bonds_refusal`). Tutamaçla sürüklemede, yeni Duvarı kaydır
>   aracında (`tool.wallShift`, `cad.walls.edit` `shift`; Mimari › Düzenle ▾), Boy'da, açıklığın mesafesinde ve Açıklığı
>   taşı'da bağlar imleç hareket ederken yerinde işaretlenir (`bond_refusal::Refused`). Hiçbir bağ tek başına geçirmiyorsa
>   `BondInTheWay.freesWith` en küçük kümeyi (iki, yoksa üç bağ; en çok 48 deneme) verir, küme ilk çelişkinin dışından üye
>   alabilir (`bonds`'ta sonda, `uids` değişmez); ileti “şu iki bağ birlikte kaldırılınca geçer”. Reddedilen yazmadan sonra
>   çizimin üstünde kart: “Bağı kaldır: d1” ya da “Bu iki bağı kaldır” tek `cad.constraints.edit` `remove` yazar.
>   Sürüklemenin önizlemesi 200/50'de 1,7 ms, 400/100'de 6,1 ms. Şema değişmez. 23 ortak durum
>   (`arch_bond_conflict_cases.py`), iz `wall-bond-drag.json`. Gerçek pencerede elle denenmedi.

**TODOS.md §0.2** (web'e ertelenenler):

> - ADR 0790: web'in sonucu `BondInTheWay.freesWith`'i okumuyor; sürüklemede ve kaydırmada yoldaki bağların vurgusu, reddin
>   kartı (“Bu iki bağı kaldır”) ve Duvarı kaydır aracı yok.

**plan/architecture-advanced.md ADV-01** (Durum satırının “Açık:”ı):

> _Durum (31 Ekim 2026, ADR 0770, 0780, [ADR 0790](../docs/adr/0790-bonds-while-dragging-and-together.md)):_ … Tutamaçla
> sürüklemede, Duvarı kaydır'da, Boy'da ve açıklığın taşınmasında yoldaki bağlar yerinde işaretli; hiçbir bağ tek başına
> geçirmiyorsa birlikte kaldırılınca geçen en küçük küme (`freesWith`) önerilir ve kartın düğmesiyle tek adımda kaldırılır.
> Açık: gerçek pencerede elle kabul, geç çelişkide 48 denemenin en kötü süresi, özellikler panelinden yazılan değerin kartı.
