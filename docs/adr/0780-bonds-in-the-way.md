# ADR 0780: Yoldaki bağlar — çelişkide bağların yerinde vurgusu, Bağı göster ve kaldır, 20 ms bütçesi

- **Durum:** kabul edildi (31 Ekim 2026). Kapsam: `plan/architecture-advanced.md` ADV-01'in ADR 0770'te açık kalanları
  (çelişen bağın çizimde yerinde vurgusu, “Bağı göster/kaldır” Python ve MCP'de, 20 ms bütçesinin ölçümü); madde tek
  parçada (ADV-01b).
- **Bağlam belgesi:** ADR 0770 (bağlantılı odalar ve kalıcı mimari bağlar), 0334–0338 (kısıtlar, komutları, düzenlemede
  kısıtlar, işaretleri), 0710 (duvarın geçici ölçüsü), 0253 (komut protokolü, `CommandError.uids`).
- **Web:** ertelendi (ADR 0228); satırı “Belgelere eklenecek satırlar”da.
- **Şema:** `.kcad` değişmez (yeni alan dosyaya yazılmaz). `FORMATS_VERSION` değişmez.

## Bağlam

ADR 0770'te bağlar bir düzenlemeyi izleyemezse komut `constraint_conflict` ile reddediyor, `uids`'te çelişen kısıtların
kimliklerini veriyor ve iletide “Çelişen bağı kaldırın ya da kısıtları gevşeterek düzenleyin” diyordu. Üç eksik vardı:

1. Hangi bağın hangi duvar çizgisini ya da pervazı tuttuğu, ne değeri koruduğu ve düzenlemenin onu kaça götürdüğü reddin
   kendisinde yoktu; Python ve MCP bunu ancak kimlikleri `cad.constraints.query` ile tek tek arayarak bulabiliyordu.
   Hangi bağın tek başına kaldırılınca düzenlemeyi geçireceği hiç söylenmiyordu.
2. Masaüstünde ret yalnız kartın kırmızı iletisiydi; bağlar çizimde işaretlenmiyordu (plan: “Çelişkide ilgili bağlar
   yerinde işaretlensin; hangi bağı kaldırabileceği açıklansın”).
3. Bütçe (100 bağlı, 400 duvarlı katta ≤ 20 ms) ölçülmemişti.

Araştırma sırasında bulunan hata: `cad.walls.edit`'in planı (önizlemesi) bağları hiç izlemiyordu. `effects` adımı
okuma kopyasında `transact` ile yazıyordu; kısıtların izleyicisi (`constraint_follow`) yalnız `run_step`'in açtığı
adımın değişikliklerini (`step_befores`) okuduğundan planda hiçbir şeyi değişmiş görmüyordu. Önizleme kartı bu yüzden
çelişkiyi Enter'dan önce söyleyemiyor, bağların kaydırdığı duvarları (`effects.bonded`) boş veriyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit — Removing Constraints](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-CBBA70BA-0FBA-4CC5-807E-517A2654075A.htm):
  kısıt ölçünün kilidine basılarak (açılarak), EQ işaretine basılarak ya da ölçü silinerek kaldırılır; ölçüyü silmek
  kısıtı silmez, uyarı penceresinde “Delete Element(s)” ile kısıt da kalkar.
- [Revit — Reveal Constraints](https://help.autodesk.com/cloudhelp/2015/ENU/Revit-Model/files/GUID-A36596DF-336B-4D84-9F91-B5D764A07281.htm):
  görünümün geçici kipi; bütün kısıtlar renkli, model yarım tonda çizilir, bir kısıt seçilince tuttuğu elemanlar
  vurgulanır, kısıt orada silinir ya da kilidi açılır.
- Revit API'nin hata tanımı [`DimensionFailures.RegenConstraintUnsatisfiedWarn`](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-API-MainReference/files/html/185612e4-1b2f-7ea1-4e1e-8106da425e2a.htm)
  (“Constraints are not satisfied”): Revit çelişen bir düzenlemeyi bu uyarıyla durdurur; kullanıcı kısıtı kaldırarak ya
  da düzenlemeden vazgeçerek çıkar (pencerenin düğme adları resmî yardımda yazılı bulunamadı).
- [Archicad 30 — Distance Guides](https://help.graphisoft.com/AC/30/INT/Topics/030_Interaction/030_Interaction-65.html)
  (ADR 0770'teki gibi): Archicad kalıcı ölçü bağı tutmaz; çelişki penceresi yoktur, bağlı duvarlar düzenlemeyle esner.

KentOS kararı: Revit'in iki adımını (çelişkide durmak, sonra bağı bulup kaldırmak) tek yere toplarız: ret, yoldaki her bağı
parçaları, tuttuğu değer ve düzenlemenin bıraktığı değerle verir ve **hangisinin tek başına kaldırılınca yazmayı geçirdiğini**
hesaplayıp söyler. Revit'in Reveal Constraints'i gibi bağlar yerinde vurgulanır, ama ayrı bir kip açmadan, yazı yazılırken
önizlemede. Kaldırma yeni bir komut değil: var olan `cad.constraints.edit` `remove` (masaüstünde kilit karesi).
Otomatik kaldırma, tolerans büyütme ya da sessiz koparma yok (plan); gevşeterek düzenleme (ADR 0337) seçenek olarak kalır.

## Karar

### 1. Reddin bağları (`CommandError.bonds`)

`CommandError`'ın yeni, isteğe bağlı `bonds` alanı (boşsa yazılmaz; bütün öbür retlerde boş): `constraint_conflict`'in
çelişen **mimari** bağları, kümenin sırasıyla (`uids` ile aynı sıra). Her biri `BondInTheWay`:

| Alan | Anlamı |
|---|---|
| `id` | kısıtın kimliği (`uids`'tekiyle aynı) |
| `label` | Türkçe adı (`Uzaklık`, `Sabit`, `Simetrik`, `Paralel`) |
| `parameter` | mesafenin parametresi (`d1`) |
| `parts` | andığı duvar çizgileri ve pervazlar, kuralın sırasıyla (`a`, `b`, `axis`) |
| `kept` | mesafenin tuttuğu değer, metre (başvuru bağında yok) |
| `measured` | yazmanın bıraktığı net mesafe, bağlar izlemeden önce (iki pervazda duvar boyunca, işaretli); parçaları artık uymuyorsa yok |
| `frees` | bu bağ tek başına çıkarılınca aynı yazma bağlarla çözülüyor |

`frees`: bağdaştırıcı (`arch_bonds::solve`) çelişki bulunca çelişen her kısıtı tek başına kümeden çıkarıp aynı yazmayı
aynı tutulanlarla yeniden dener. Çelişki bağların denklemlerinden geldiyse (sabitin oynaması, paralellik, pervazın açısı,
tutmayan satır) deneme denklemlerle sınırlıdır: tutulanlar o bağsız yeniden belirlenir (eşit aralığın uçları, yeni
değerde tutmayan mesafenin `a`'sı, oynayan sabit), satırlar yok edilir, en az değişiklik hesaplanmaz
(`ops::arch_bonds::conflict`). Çelişki kaydırmalar yazılırken geldiyse (Kaydır'ın reddi: kilitli katman, birleşim; yazılandan
sonra tutmayan bağ) deneme bütündür (ADR 0770 §4'ün bütün adımları). Çelişkisiz çıkan bağ `frees`'tir. İleti sonunda
onları adlandırır:

- birisi: “Şu bağ kaldırılınca düzenleme geçer: d1 (Uzaklık). Ya da kısıtları gevşeterek düzenleyin.”
- birkaçı: “Şu bağlardan biri kaldırılınca düzenleme geçer: d1 (Uzaklık), Sabit. …”
- hiçbiri (örn. aynı duvara iki sabit bağ): “Tek başına kaldırılınca düzenlemeyi geçiren bağ yok; çelişen bağları kaldırın
  ya da kısıtları gevşeterek düzenleyin.”

Düzenlemenin izleyicisinde (`constraint_follow`: Kaydır, Mesafe, Boy, açıklığı taşı …) ileti bağdaştırıcının nedeniyle
başlar (ADR 0770 §4). `cad.constraints.edit` ve `cad.parameters.edit`'in (`settle`) çelişkisinde bağ varsa ileti
“Kısıtlar birlikte tutamıyor; hiçbir şey yazılmadı. ” + aynı söz; hiçbiri geçirmiyorsa eski ileti. Genel (mimari olmayan)
kısıtların çelişkisi değişmez (`bonds` boş). Kod tablosunun (`codes.rs`) sözü `bonds`'u anar.

`cad.walls.edit`'in planı artık yürütmeyle aynı adımı (`run_step`) okuma kopyasında açar: bağlar izler, çelişki planda da
aynı retle (bağlarıyla) döner, `effects.bonded` bağların kaydırdığı duvarları verir.

### 2. Bağı göster ve kaldır (Python, MCP)

Yeni komut yok; var olan iki komut yeter:

- **Göster:** `cad.constraints.query` v1'in yeni isteğe bağlı girdileri `ids` (yalnız bu kısıtlar: reddin `bonds`'u ya da
  `uids`'i) ve `architectural` (yalnız duvar çizgisi ve pervaz bağları); `uids` ile birlikte kesişimleri. Liste satırının
  yeni `kept`'i: sürücü ölçü kısıtının parametresinin değeri (metre ya da radyan; başvuruda ve geometrik kurallarda yok).
  Mimari bağın `holds`'u artık kendi ölçümüyle (ADR 0770 §4'ün 1e-6 m'si): tutmayan **her** bağ `false` (önceden her
  nesneyi tutan çözücünün ilk çelişen satırı yalnız ilk bağı işaretliyordu). Yeni retler: `ids[i]` UUID değil
  `invalid_uid`, çizimde olmayan kısıt `constraint_not_found` (`path` `ids[i]`).
- **Kaldır:** `cad.constraints.edit` `{"kind": "remove", "ids": [...]}` (parametresi onu okuyan yoksa onunla gider).

Python: `kentos.cad.CommandError.bonds` (`BondInTheWay`, `from_json`/`to_json`), `cad.constraints.query(doc, ids=…,
architectural=True)`; MCP'de reddin `structuredContent`'i `bonds`'u olduğu gibi taşır, araçların şeması katalogdan.

### 3. Masaüstü: yerinde vurgu

Duvarın mesafesi (ADR 0710, 0770 §6): yazı kutusuna yazılan değerin planı reddedilirse kart tehlike renginde iletiyi ve
her yoldaki bağı bir satırda verir (önce geçirenler): “• d1 (Uzaklık): 2.800 m tutuyor, düzenleme 3.200 m yapar —
kaldırılınca geçer”. Önizleme aynı anda bağları yerinde çizer (`wall_distance::bonds_in_way`): tuttuğu her duvar çizgisi
3 px kalın, her pervaz halkayla, bir bağın parçaları arasında kesikli çizgi ve ortasında “× d1 — kaldırılınca geçer”,
hepsi tehlike renginde. Bağlı yanın sayısına yazılan yeni değer (`cad.parameters.edit`) de önce planlanır; reddi aynı
kartla ve vurguyla gelir. Enter hiçbir şey yazmaz; kilit karesi (Bağı kaldır, ADR 0770) bağı kaldırınca aynı değer geçer.

### 4. Bütçe (20 ms)

`arch_bond_timing` (release, `--ignored`): 50 kat duvarı (20 m, 3 m arayla), iki uçlarındaki duvarlar ve odaların bölmeleri
= 200 duvar; 49 mesafe bağı her duvarın üst yüzünü bir üsttekinin alt yüzüne zincirler, biri ilk bölmeyi batı duvarına
bağlar = 50 bağ. En alttaki duvarı kaydırmak 49 duvarı bağla sürükler (en kötü zincir). Aynısı ADR 0770 §8'in boyunda:
100 kat, 400 duvar, 100 bağ. Reddedilen önizlemede en üstteki duvar da sabittir: bütün bağlar yoldadır. Apple M5, release,
ortanca:

| Çizim | Önizleme (plan), bağlarla izleyen | Yazma (execute) | Tek bağlı bölmenin önizlemesi | Reddedilen önizleme (bütün bağlar yolda) |
|---|---|---|---|---|
| 200 duvar, 50 bağ | 2,38 ms (101 duvar izler) | 3,81 ms | 0,06 ms | 0,50 ms (50 bağ) |
| 400 duvar, 100 bağ | 6,85 ms (201 duvar izler) | 13,60 ms | 0,11 ms | 2,01 ms (100 bağ) |

Bütçe (≤ 20 ms) her durumda tutuyor. İlk ölçümde reddedilen önizleme 200 duvarda 43,3 ms, 400 duvarda 44,9 ms idi: `frees`
her bağ için kümeyi baştan okuyup kaydırmaları yazıyordu. Denklemlerin çelişkisinde küme bir kez okunur ve her bağsız
problem yalnız yok edilir (en az değişiklik ve yazma yok); 400 duvarda 36,8 → 2,0 ms.

## Sınırlar

- `frees` yalnız tek bir bağın çıkarılmasını dener; iki bağın birlikte çıkarılması gereken durumda (örn. aynı yüze iki
  sabit) “hiçbiri” denir, çelişen bağların hepsi `bonds`'ta olduğundan kullanıcı ya da program ikisini birden kaldırabilir.
- Genel (mimari olmayan) kısıtların çelişkisi için `bonds` yazılmaz; onlar ADR 0338'in işaretleriyle kalır.
- Vurgu yalnız Duvarın mesafesi'nin yazı kutusunda; tutamaçla sürüklemede ve Kaydır aracında ret yalnız iletiyle.
- Gerçek pencerede elle denenmedi.

## Uygulama

- Sözleşme: `BondInTheWay` (`constraints.rs`), `CommandError.bonds`, `ConstraintsQuery.ids`/`.architectural`,
  `ListedConstraint.kept`; yetenek `cad.constraints.query` → `constraint_not_found`.
- Uygulama: `arch_bonds::solve` (`solve_once` ve `frees`), `constraint_solve::Solved.arch_bonds` ve `bonds_words`,
  `constraint_follow` ve `constraints_edit`'in retleri, `constraints_query` (`ids`, `architectural`, `kept`, mimari
  `holds`), `walls_edit::effects` `run_step` ile.
- Çekirdek: `ops::arch_bonds::conflict` (satırlar yok edilir, en az değişiklik hesaplanmaz), `solve` onun üstünde.
- Masaüstü: `kentos_interaction::wall_distance` (`refusal_lines`, `bonds_in_way`, bağlı yanın planı); sahneler
  `wall_scenes.rs` (`duvar-bag-celiski`, `duvar-bag-kaldirildi`); Komut geçmişinin `Call::Refused`'u kutulu (büyüyen
  `CommandError` yüzünden).
- Python: `kentos.cad.errors.BondInTheWay`, `_runtime.outcome`; SDK katalogdan yeniden üretildi.

## Doğrulama

- Bağımsız başvuru (KentOS kodu olmadan): `scripts/fixtures/arch_bond_conflict_cases.py` → `cad.walls.edit.bonds.json` 22
  durum (koridor bağı ve sabit dış duvar, planı da, bağı kaldırıp yeniden deneme, sabitin oynaması, aynı duvara iki sabit:
  hiçbiri geçirmez, kilitli katman, üç bağlı zincir, yolda olmayan bağ adlandırılmaz, pervaz–duvar ve pervaz–pervaz,
  eşit aralık, Mesafe ve planı, aynı yüzlere iki değer, `cad.parameters.edit` ve `cad.constraints.edit` `add`'in reddi,
  paralel bağ, yolda olmayan duvarın kaydırılması) ve `cad.constraints.query.arch.json` 22 durum (bütün kısıtlar ve genel
  kısıt, `architectural`, `uids`, `ids`, ikisi birlikte, tutmayan bir ve iki bağ, başvuru, iki pervaz, pervaz–duvar, eşit
  aralık, sabit ve paralel, bağsız çizim, kaydırmadan ve kaldırmadan sonra, kilitli katman, `constraint_not_found` iki,
  `invalid_uid` iki, parametre değişince). Kuralları `arch_bond_cases.py`'nin (ADR 0770 §4) yeni `equations`,
  `in_the_way`, `bonds_words`'ü; o dosyanın beş çelişki durumu artık `bonds`'u ve iletinin yeni sonunu bekler
  (`cad.constraints.edit.arch.json`, 31). `constraint_command_cases.py` sorgu satırlarına `kept` yazar
  (`cad.constraints.query.json`).
- Rust: `KENTOS_CASES_ONLY=cad.constraints,cad.parameters,cad.walls,cad.openings cargo test -p kentos-native-application
  --test all every_command_case` (823 durum geçti); `--test all capabilities`, `--test all catalog`;
  `cargo test -p kentos-geometry-core --lib arch_bonds` (çekirdeğin `conflict`'i); `KENTOS_WRITE_CATALOG=1 cargo test
  -p kentos-contracts -p kentos-commands` (katalog ve TS türleri yeniden yazıldı, sonra aynı).
- Python: `python3 scripts/python/sdk.py --check`; yerli modül maturin ile, `KENTOS_CASES_ONLY=cad.constraints,cad.walls,
  cad.parameters python -m unittest python.tests.test_command_cases` (reddin `bonds`'u `BondInTheWay` ile gidip gelir),
  `test_protocol`, `test_catalog`. `python3 scripts/cad/manifest.py --check` (933 satır, yeni komut yok).
- Masaüstü: `cargo test -p kentos-desktop --lib wall_scenes::bond_tests` (3; yeni: çelişkide kart tehlike renginde, d1 ve
  Sabit satırları, üç kalın çizgi, iki “kaldırılınca geçer” yazısı, Enter hiçbir şey yazmaz; kilit karesiyle bağ kalkınca
  aynı değer yazılır). Resimler `KENTOS_SHOTS_ONLY=duvar-bag-celiski,duvar-bag-kaldirildi cargo test -p kentos-desktop
  --lib tools_screens::screens -- --ignored --nocapture`: `.run/shots/arac-duvar-bag-celiski-*`, `-bag-kaldirildi-*`
  (1440×900 ve 1100×650, koyu ve açık). 1100×650'de kart odanın üstüne düşer; vurgu 1440'ta okunur.
- Bütçe: `cargo test --release -p kentos-native-application --test all arch_bond_timing -- --ignored --nocapture` (§4).
- `cargo test -p kentos-interaction wall` (54 + 3), `cargo test -p kentos-desktop --lib history` (23); clippy `-D warnings`
  `--all-targets` kentos-contracts, -commands, -geometry-core, -native-application, -interaction, -desktop.
- Gerçek pencerede elle denenmedi.

## Belgelere eklenecek satırlar

**CLAUDE.md §0** (ADR 0770'in maddesinin hemen altına):

> - **Yoldaki bağlar ve Bağı göster/kaldır (31 Ekim 2026; [ADR 0780](docs/adr/0780-bonds-in-the-way.md), ADV-01 [~]).**
>   `constraint_conflict`'in yeni `bonds`'u (`CommandError.bonds`, `BondInTheWay`): çelişen her mimari bağın parçaları,
>   tuttuğu (`kept`) ve yazmanın bıraktığı (`measured`) değer, tek başına kaldırılınca yazmayı geçirip geçirmediği
>   (`frees`); ileti geçirenleri adlandırır. `cad.walls.edit`'in planı yürütmeyle aynı adımı (`run_step`) açar: bağlar
>   önizlemede izler ve reddeder (önceden izlemiyordu). `cad.constraints.query`'nin `ids`'i, `architectural`'ı ve satırın
>   `kept`'i; mimari bağın `holds`'u kendi ölçümüyle; kaldırma `cad.constraints.edit` `remove`. Duvarın mesafesi'nde ret
>   kartı yoldaki bağları sayar, bağlar yerinde tehlike renginde (kalın çizgi, kesikli bağ, “× d1 — kaldırılınca geçer”).
>   Bütçe: 200 duvar/50 bağ önizleme 2,4 ms, ret 0,5 ms; 400/100 6,9 ms ve 2,0 ms (`arch_bond_timing`). Şema değişmez.
>   22 + 22 ortak durum (`arch_bond_conflict_cases.py`). Gerçek pencerede elle denenmedi.

**TODOS.md §0.2** (web'e ertelenenler):

> - ADR 0780: web'in komut sonucu `CommandError.bonds`'u okumuyor; mesafe düzenlemesinde yoldaki bağların vurgusu ve ret
>   kartının bağ satırları yok; `cad.constraints.query`'nin `ids`, `architectural`, `kept`'i web'in arayüzünde kullanılmıyor.

**plan/architecture-advanced.md ADV-01** (Durum satırı):

> _Durum (31 Ekim 2026, ADR 0770, [ADR 0780](../docs/adr/0780-bonds-in-the-way.md)):_ … Çelişen bağ reddin `bonds`'unda
> (parçalar, değerler, `frees`) ve Duvarın mesafesi'nde yerinde vurgulu; Bağı göster/kaldır Python ve MCP'de
> (`cad.constraints.query` `ids`/`architectural`, `cad.constraints.edit` `remove`); 20 ms bütçesi ölçüldü (200/50: 2,4 ms,
> 400/100: 6,9 ms). Açık: tutamaçla sürüklemede ve Kaydır aracında yerinde vurgu, iki bağın birlikte kaldırılmasının
> önerisi, gerçek pencerede elle kabul.
