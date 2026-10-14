# ADR 0272: Kenetin başvurusu: komut olarak kenet ve ilişkili ölçüye bağlama

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-08`: “Snap sonucu; nesne/alt nesne referansı,
  kaynak noktası, türü ve revision bilgisiyle okunabilsin; ilişkili ölçü oluşturmak için referans kullanılabilsin.” Kapsamı ben
  belirledim.
- **Bağlam belgesi:** ADR 0249 ve 0263 (nesnenin yerleri, ilişkili ölçü), 0253 (komut protokolü, sürüm), 0269 (Ekleme noktası), 0271
  (adayların sırası), 0270 (salt okunur nokta komutları).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Ölçü aracı kenetin noktasını nesnenin yerine bağlıyordu (`snapped_place`, ADR 0249 §3), ama kenetin sonucu yalnız araçların içinde
yaşıyordu: Python ve ajan bir tıklamanın neye kenetleneceğini, hangi nesnenin hangi yerinde olduğunu soramıyordu; arayüzde de kenetin
hangi nesnenin hangi yeri olduğu bir yerde yazmıyordu. Ekleme noktası (ADR 0269) bir yere bağlanamıyordu: yazının, bloğun ve tablonun
noktası yer sözlüğünde yoktu.

## Karar

### 1. Ekleme noktası bir yerdir

Yazının, blok yerleştirmesinin ve tablonun noktası `node` yeridir (`part` 0): `ops::subref::point` onu verir, `place_at` Ekleme noktası
kenetini ona çevirir; ilişkili ölçü yazının noktasına bağlanır ve yazı taşınınca izler (yapı aynı: yer korunur, ADR 0249 §2). Sözleşme ve
`.kcad` değişmez (`node` yeri vardı).

### 2. `cad.snap.find` v1

Salt okunur ürün komutu (`effect: query`): `at`'taki kenet, `aperture` metre içinde; `kinds` (yoksa ayarların ilk açık türleri: Yakın ve
Ağırlık merkezi dışında hepsi), `from` (Dik ve Teğet için son nokta), `all` (bütün adaylar en iyiden, ADR 0271). Cevap her kenet için:

| Alan | Anlamı |
|---|---|
| `kind` | kenetin türü |
| `point` | kaynak noktası (metre) |
| `uid` | nesnenin kalıcı kimliği (kesişimde iki nesnenin çizimdeki ilki) |
| `place` | nesnedeki yeri: uç, köşe, kenar ortası, merkez, düğüm, kenar üstü (`along`); kesişimde ve ağırlık merkezinde yok |
| `path` | blok içindeyse parçanın yolu |

ve `revision` (okunduğu sürüm). Kenet deponun kuralıdır (`snap_ex`, `snap_candidates`): araçların kenetlediği nokta; gizli katman
kenetlenmez. Yer çekirdeğin `place_at`/`place_in`'idir: ölçü aracının bağladığı yer.

- **İlişkili ölçü:** cevabın `uid`, `place` ve `path`'i `cad.dimensions.create`'in `place` noktası olarak olduğu gibi verilir; ölçü o
  yere bağlanır (Python'un testi bunu yapar).
- Retler: `not_finite` (`at`, açıklık, `from`), `invalid_aperture` (sıfır ya da eksi), `no_snap_kinds` (boş liste). Python
  (`kentos.cad.snap.find`) ve MCP'de (salt okunur araç).
- Uzantı, Paralel ve Karelaj çalışan komutun oturumuna aittir (izleme, son nokta, ızgara); bu komutta istenmez.

### 3. Arayüzde okunur

- Kenet işareti nesnenin türünü söyler (ADR 0271 §3).
- **Koordinat oku**, kenetlenen tıklamada kenetin türünü, nesnenin türünü ve kimliğinin başını, nesnedeki yerini yazar:
  “Kenet: Uç nokta · Kapalı alan f08a1eed · 3. köşe”, koordinatlardan önceki satırda.

## Sonuçlar

- Python ve ajan bir tıklamanın kenetini, nesnesini ve yerini sürümüyle alır ve ölçüyü o yere bağlar; geometriyi kendileri hesaplamaz.
- Yazının, bloğun ve tablonun noktası ilişkili ölçüye bağlanabilir.

## Doğrulama

- Bağımsız başvuru `python3 scripts/fixtures/snap_find_cases.py --check` (`fixtures/commands/v1/cad.snap.find.json`, 21 durum; yerler
  ADR 0263'ün bağımsız betiğinden), `python3 scripts/fixtures/subref_cases.py --check` (Ekleme noktasının yeri);
  `cargo test -p kentos-native-application`, `cargo test -p kentos-geometry-core --test all subref`.
- `pnpm py:test` (`python/tests/test_snap.py`: kenet, yer ve ölçünün bağlanması), `cargo test -p kentos-mcp`.
- `cargo test -p kentos-interaction --test all snap_reference` (Koordinat oku'nun satırı).
- Resim: `KENTOS_SHOTS_ONLY=kenet-oku cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
