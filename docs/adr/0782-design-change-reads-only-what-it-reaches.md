# ADR 0782: Tasarım değişikliği yüz dairede: adım yalnız ulaştığını okur

- **Durum:** kabul edildi (2026-10-31); ADR 0776'nın B6 ve B7 bulgularının dilimi; `plan/architecture-advanced.md`
  ADV-07 **[~]** sürer.
- **İş:** 100 dairede bir bölmenin Mesafe'si (Bu ölçüyü koru) 1,86 s sürüyor, bir öznitelik yazmak 0,79 s, değişiklik
  sırasında bellek ikiye katlanıyordu (ADR 0776 §4). Dairelerin sayısıyla karesel büyüyen iki yer bulundu ve
  kaldırıldı; adımın sonucu bit bit aynıdır.
- **Web:** web dururken (ADR 0228) yalnız masaüstü ve ortak çekirdek; web'e gerekenler aşağıda.
- **`.kcad`:** şema değişmez. `FORMATS_VERSION` değişmez. Komut, girdi ve cevap değişmez.
- **Derleme:** `main` a7cd4212d (ADV-04/05/06 ve ADV-07 birleşmiş). Apple M5, 32 GB, macOS; ölçümler release.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Bir adımın sonunda belge, adımın değiştirdiklerini izleyenleri aynı adımda yazar (`Document::follow_into`: açıklıklar,
mahaller, bağlı döşemeler, etiketler, ölçü dizileri, çizelgeler …). Revit de değişikliği işlem kapanırken yeniden
üretir ve bunun pahalı olduğunu, gerektiğinden sık yapılmaması gerektiğini söyler (Revit API Developer's Guide 2025,
“Getting Element Geometry and AnalyticalElement”: işlem kapanınca yeniden üretim ve kendiliğinden birleştirme olur,
yeniden üretim zaman alır;
<https://help.autodesk.com/cloudhelp/2025/ITA/Revit-API/files/Revit_API_Developers_Guide/Basic_Interaction_with_Revit_Elements/Transactions/Revit_API_Revit_API_Developers_Guide_Basic_Interaction_with_Revit_Elements_Transactions_Getting_Element_Geometry_and_AnalyticalModel_html.html>,
31 Ekim 2026'da okundu). KentOS kararı: yeniden üretim adımın sonunda kalır, ama yalnız adımın ulaştığını okur.

Ölçüm (ADR 0776'nın `perf::flat`'i ve aynı akışı penceresiz oynatan yeni `flat_timing`) ve macOS `sample` ile:
Mesafe'nin 1,92 s'sinin %60'ı bağlı döşemenin sınırının yeniden bulunmasında, %38'i çizelgelerin yeniden
yazılmasındaydı; komutun cevabı adımı bir kez okuma kopyasında (etkiler, ADR 0770 §3), bir kez gerçekten koştuğu için
ikisi iki kez ödeniyordu.

1. **Bağlı döşeme bütün katı okuyordu.** `slab_from_walls` döşemenin tohumu çevresindeki dış yüz çizgisini katın *bütün*
   duvarlarının birleşim ve yüz hesabıyla buluyordu (`wall_faces`, `FaceIndex::new`, duvar başına JSON çevirisi). Hangi
   döşemenin yeniden bulunacağı kutuyla sınırlıydı; ama o tek döşemenin hesabı 100 dairenin 900 duvarını okuyordu.
   Bir öznitelik yazmak da duvarın kutusunu “değişti” saydığından aynı hesabı tetikliyordu.
2. **Çizelgeler her adımda baştan yazılıyordu, her biri kendi başına.** ADR 0776'nın ölçümü daireyi iki çizelgesiyle
   kopyalıyordu: 100 dairede 200 çizelge, her biri binanın bütün kapılarını ya da mahallerini okur. Bir duvarın değişmesi
   açıklık ve mahal çizelgelerinin türlerine dokunduğundan 200 çizelgenin her biri satırlarını, sütun genişliklerini
   (çizelgenin JSON'a çevrilip ölçülmesiyle) ve görünüşünü yeniden hesaplıyordu: çizelge sayısı × satır sayısı.

## Karar

### 1. Bağlı döşeme yalnız kendi parçasını okur

`kentos_domain::wall_pieces::walls_round`: katın duvarları parçalara ayrılır; iki duvar, erim kutuları (noktaları, yayın
kirişten taşması, genişlik ve öteleme) birbirine `GAP` = 1 m'den yakınsa aynı parçadadır (x boyunca süpürme, birleşim
kümesi). Tohumu kutusunda tutan parçalar, kutuları onlarınkine değen parçalar (kapanana dek) ve katın **ilk duvarının**
parçası okunur; başka parçanın duvarı alınan her duvardan en az 1 m uzaktadır, ne birleşir, ne keser, ne tohumu tutar,
ne de onun yüzünün içinde delik olur. İlk duvarın parçası şunun için gerekir: bindirme işini ilk yüz çizgisinin başına
taşır (`overlay::origin_of`); aynı başlangıç sonucu katın bütün duvarlarıyla bulunanla **son bitine kadar** aynı yapar.
Bütün parçalar gerekiyorsa (bir çevre duvarı her daireyi sarıyorsa) eski yol aynen koşar
(`Document::slab_from_every_wall`).

### 2. Aynı çizelgeler bir kez çıkarılır

Bir adımın `follow_tables`'ı, yerleri dışında aynı olan çizelgeleri (`alike_but_placed`: kaynak, yazı boyu, satırlar,
sütunlar, hücreler, görünüş; nesnesi, yeri ve dönüşü sayılmaz) bir kez çıkarır: çizelgenin çıkarılması yerine bakmaz
(alanları ve formülleri olmayan çizelge), ikincisi ilkinin sonucunu kendi nesnesi ve yeriyle alır (`placed_as`). Değişmeyen
çizelge hiç kopyalanmaz. Alan bağlı tablolar ve blok çizelgeleri eskisi gibi tek tek çıkarılır.

### 3. Pencere dışına iş taşımak gerekmedi

§1 ve §2'den sonra 100 dairede komut p50 38,7 ms (binanın iki çizelgesiyle) ve 81,5 ms (her dairenin iki çizelgesiyle),
hedef 100 ms'nin altında. Komutu ADR 0756/0757'nin iş düzeniyle (ilerleme, Durdur) arka plana almak bu yüzden
yapılmadı. Her dairenin çizelgeleriyle karenin kalan 340 ms'si komutun değil, yazı katmanının bütün görünümü yeniden
kurmasıdır (§5, B13): komutu arka plana almak onu kısaltmaz.

### 4. Bellek

Binanın çizelgeleriyle 100 dairede değişikliğin tepesi çizilmiş hâlin %4,4 üstündedir (hedef %25). Her dairenin
çizelgeleriyle tepe hâlâ yüksektir; çekirdek tek başına (penceresiz) %29 büyür, gerisi masaüstünün yazı katmanının
400 000 çizelge hücresini her değişiklikte yeniden yerleştirmesidir (B13). Belgenin geri alma geçmişi ve okuma
kopyaları paylaşılır (`Arc`); yeni bir kopya bulunmadı.

### 5. Ölçümler

**Penceresiz** (`flat_timing`, adımın kendisi, ms; önce → sonra, medyan):

| Daire, çizelgeler | Mesafe | Planı | Kaydır | Öznitelik | Bellek: önce → tepe (sonra) |
|---|---|---|---|---|---|
| 20, binanın 2'si | 43,1 → 6,1 | 42,5 → 5,8 | 42,4 → 5,2 | 20,8 → 1,9 | |
| 100, binanın 2'si | 1 081 → 35,9 | 1 086 → 35,7 | 1 091 → 35,6 | 545 → 14,5 | 39 → 43 MB |
| 20, her dairenin 2'si (40) | 73,5 → 7,1 | 73,1 → 7,8 | 72,1 → 6,7 | 30,3 → 1,9 | |
| 100, her dairenin 2'si (200) | 1 920 → 79,4 | 1 982 → 98,2 | 1 999 → 79,4 | 844 → 16,9 | 318 → 410 MB |

**Masaüstü** (`perf::flat`, gerçek Iced olaylarıyla; komut p50 / karenin UI payı p95, ms; bellek MB, `ps` her 5 ms):

| Daire, çizelgeler | Önce: komut / kare | Sonra: komut / kare | Öznitelik, önce → sonra | Bellek sonra: çizildi / değişiklik öncesi / tepe |
|---|---|---|---|---|
| 1, 2 | 0,9 / 1,9 | 1,0 / 2,1 | 0,21 → 0,23 | 379 / 726 / 727 |
| 20, 2 | 45,6 / 49,1 | 6,2 / 10,4 | 21,1 → 2,0 | 723 / 738 / 744 |
| 100, 2 | 1 151 / 1 174 | 38,7 / 52,6 | 544 → 14,8 | 758 / 776 / 791 (+%4,4) |
| 100, 200 | 2 163 / 2 460 | 81,5 / 340 | 881 → 17,1 | 1 515 / 2 346 / 4 460 (önce 2 290 / 5 376) |

Kaydırma, Kapı önizlemesi ve seçme değişmedi (100 dairede UI p95 1,15–1,22 ms). Ham raporlar
`.run/perf-before*/`, `.run/perf-after*/` (sürüm denetiminde değil).

Bulgular (ADR 0776 §5'in sürdürülmesi):

| # | Önem | Bulgu | Durum |
|---|---|---|---|
| B6 | yüksek | Tasarım değişikliği 100 dairede 1,86 s, karesel | kapandı: 38,7 ms (binanın çizelgeleri), 81,5 ms (her dairenin) |
| B7 | yüksek | Değişiklik sırasında bellek ikiye katlanıyordu | binanın çizelgeleriyle kapandı (+%4,4); her dairenin çizelgeleriyle B13'e bağlı |
| B13 | orta | Yazı katmanı belge her değiştiğinde görünen bütün yazıları yeniden yerleştirir (`labels::build`, `Spatial::labels_shown_in`): 200 çizelgenin 400 000 hücresiyle kare 340 ms ve bellek +2 GB | açık: yazı katmanının değişen nesnelerle artımlı kurulması ayrı iş |

## Doğrulama

- `crates/native/headless/tests/headless/flat_pieces.rs`: 2+1 daire altı kez kopyalanır; 48 rastgele düzenleme (duvar
  kaydırma, düz, yaylı, iki daireyi bağlayan ve en sonda bütün daireleri saran duvar ekleme, duvar silme, ara sıra geri
  alma) ve her birinden sonra: her bağlı döşemenin parçalardan bulunan sınırı katın bütün duvarlarından bulunanla
  `==` (bit bit), 12 rastgele tohumda iki kip aynı, her tablo kendi çıkarılmasıyla aynı (`table_worked_out(t) == t`);
  parçaların gerçekten daha az duvar okuduğu sayılır (en az 60 okuma). Geçti.
- `wall_pieces_tests::an_arc_reaches_past_its_points`: yayın kutusu kirişten taşar.
- `cargo test -p kentos-domain` (17 + 85 + 22 + 5), `cargo test -p kentos-native-application --test all` (266, bütün
  ortak komut durumları), `cargo test -p kentos-headless --test headless` (68): geçti. `cargo clippy -p kentos-domain
  -p kentos-headless -p kentos-desktop --all-targets -- -D warnings` temiz; masaüstünün döşeme, tablo, çizelge, yapı
  bileşeni, 2+1 kabul ve kalite testleri (56) ve `acceptance-flat` izi üç çeşitte geçti; `bound_slab_cases.py --check`
  (32 durum) ve `acceptance_flat.py --check` güncel.
- Ölçümler: `cargo test --release -p kentos-headless --test headless flat_timing -- --ignored --nocapture`
  (`KENTOS_PERF_SCHEDULES=each` her dairenin çizelgeleri), `cargo test --release -p kentos-desktop --lib perf::flat
  -- --ignored --nocapture --test-threads=1` (aynı değişken, `KENTOS_PERF_PARTS=1`). “Önce” sütunları aynı makinede,
  bu ADR'nin değişiklikleri çıkarılmış hâliyle ölçüldü.

## Belgelere eklenecek satırlar

CLAUDE.md §0:

- **Tasarım değişikliği yalnız ulaştığını okur (31 Ekim 2026; [ADR 0782](docs/adr/0782-design-change-reads-only-what-it-reaches.md), ADV-07 [~]).**
  Bağlı döşemenin sınırı katın bütün duvarlarından değil, tohumu çevreleyen duvar parçalarından (erim kutuları 1 m'den
  yakın duvarlar bir parça; tohumu tutan, onlara değen ve ilk duvarın parçası) bulunur, bindirmenin başlangıcı aynı
  kaldığından sonuç bit bit aynı (`kentos_domain::wall_pieces`, `slab_from_every_wall`); yerleri dışında aynı çizelgeler
  adımda bir kez çıkarılır (`follow_tables`'ın `alike_but_placed`'i). 100 dairede Mesafe 1 151 → 38,7 ms, öznitelik
  544 → 14,8 ms, değişiklikte bellek +%4,4 (binanın iki çizelgesiyle); her dairenin çizelgeleriyle (200) komut 81,5 ms,
  karenin 340 ms'si ve belleği yazı katmanının bütün görünümü yeniden kurmasıdır (açık). Rastgele 48 düzenlemede parça
  ile bütün kat aynı (`flat_pieces`).

CLAUDE.md §2:

```
cargo test -p kentos-headless --test headless flat_pieces; cargo test --release -p kentos-headless --test headless flat_timing -- --ignored --nocapture   # tasarım değişikliğinin parçaları bütün katla bit bit aynı (rastgele 48 düzenleme) ve 1, 20, 100 dairede penceresiz süre ve bellek (KENTOS_PERF_SCHEDULES=each her dairenin çizelgeleri; ADR 0782)
```

TODOS.md §0.2:

- ADV-07b (ADR 0782): web'in belgesinde bağlı döşemenin parça okuması ve aynı çizelgelerin bir kez çıkarılması
  (`slab_follow`, `table_cells`'in karşılıkları).

`plan/architecture-advanced.md` ADV-07 satırına: “B6 ve binanın çizelgeleriyle B7 kapandı (ADR 0782); yazı katmanının
artımlı kurulması (B13) açık.”
