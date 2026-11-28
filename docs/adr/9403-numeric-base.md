# ADR 9403: Sayısal temel: JavaScript'in sayı davranışından CAD'in kendi kurallarına

- **Durum:** yürüyor (2026-11-28). [plan/bakim.md](../../plan/bakim.md) BK-88. Yapılan: §6. §3'teki dış bağımlılık
  sahibin onayını bekler; onaydan bağımsız adımlar (§5, sıra 1–4) yürür.
- **Kural:** CLAUDE.md §14 (tek hesaplama kaynağı), §23 (sayısal ve kadastral doğruluk), §3 (yeni bağımlılık gerekçe
  ister).
- **İlgili:** [ADR 0008](0008-shared-core-boundary.md) (ortak çekirdek; `jsmath`'in çıkış nedeni),
  [ADR 0149](0149-measurement-accuracy-and-display-rounding.md) (gösterim yuvarlaması),
  [ADR 9303](9303-generator-environment.md) (üreteçlerin ortamı; `crmath.py`),
  [ADR 9401](9401-correctness-fixes-from-the-review.md) §9 ve §11 (halka hesapları, platformun matematik kitaplığı),
  [ADR 9402](9402-web-and-wasm-removed.md) §7 (sahibin kararı), [docs/specs/numeric-policy.md](../specs/numeric-policy.md).
- **Ürün:** hesap sonuçlarının son basamağı değişir (§5). **`.kcad`:** değişmez. **Sözleşme:** değişmez.

## Bağlam

Geometri çekirdeği web'in TypeScript çekirdeğinin birebir taşınmasıyla doğdu (ADR 0008). `jsmath` o taşımanın
aracıydı: tarayıcıyla aynı biti vermek için JavaScript'in sayı davranışını Rust'ta yeniden kurar (`Math.round`,
`Math.min`, V8'in `Math.hypot`'u, `Number()`…). Web uygulaması kaldırıldı (ADR 9402); geriye, amacı artık olmayan
bir uyum katmanı kaldı. Sahibin kararı (28 Kasım 2026): “JS Math yerine daha ciddi ve CAD için doğru sonuç veren bir
yapıya geçiyoruz.”

## Ölçüm

### Kodda ne var

| Yardımcı | Çağrı | Nerede tanımlı | JavaScript'ten taşıdığı |
|---|---|---|---|
| `js_max`, `js_min` (+ `_all`) | 882, 494 | `geometry-core/src/jsmath.rs` | NaN'ı taşır; −0 < +0 |
| `js_hypot` | 702 | aynı | V8'in ölçekli, telafili toplamı |
| `js_trim` | 575 | 4 kopya (`processing`, `contracts`, `geometry-core` × 2) | JavaScript'in boşluk kümesi |
| `js_number` | 178 | 5 kopya (sayı → metin 3, metin → sayı 2) | `Number()` ve `String(sayı)` |
| `js_round` | 159 | `jsmath.rs` (+ `style/classify.rs`) | yarımı +∞'a yuvarlar: −2,5 → −2 |
| `parse_number`, `parse_float` | 77 | `point_text.rs`, `svg-core`, `numbering.rs` | `parseFloat`: “12abc” → 12 |
| `truthy`, `or` | 58 | `jsmath.rs` | `x \|\| öntanımlı`: 0, −0 ve NaN “yok” sayılır |
| `js_sign`, `js_floor`, `js_cmp` | 39, 36, 34 | aynı | `Math.sign`; NaN'lı karşılaştırma “eşit” |
| `to_fixed` | 9 | `processing/text.rs`, `expression/js/number.rs` | `toFixed` |
| aşkın işlevler | ~400 nitelikli çağrı | `jsmath`'in `libm`'den yeniden dışa aktardıkları | yok; `libm` |

696 dosya bu yardımcılardan en az birini kullanıyor (yarısı geometri çekirdeğinde).

“Her platformda aynı bit” kuralı crate başına clippy.toml ile tutuluyor ve sekiz crate'te var (geometri, stil, SVG,
ifade, biçimler, NCZ, raster, IFC). Öbür hesap crate'lerinde (Yol, nokta bulutu, servisler, domain, uygulama,
etkileşim, işlem…) platformun kitaplığına giden çağrı serbest. IFC crate'inde bir BCF kamerası bu yüzden sistemden
sisteme 1 ulp değişiyordu (ADR 9401 §11).

### Kitaplıklar doğru yuvarlıyor mu

Bağımsız başvuru: mpmath 256 bit, en yakın çift duyarlıklı sayıya bir kez yuvarlanır (üreteçlerin `crmath.py`'si,
ADR 9303). Girdiler çizimin girdileri gibi: ondalık derece ve gradın radyanı, birkaç turluk semtler, çizim ve TM
ölçeğinde koordinat farkları; ayrıca zor açılar (π/2'nin katları ve komşuları, 2⁻³⁰⁰…2¹⁰⁰⁰). İşlev başına 20 000 girdi,
trigonometri için 12 604 zor girdi daha. Ölçüm programı depo dışındadır (kitaplıkların kendi kaynağıyla derlenir).

| İşlev | `libm` 0.2.16 yanlış | bu makinenin kitaplığı yanlış (en çok) | `pxfm` 0.1.30 yanlış |
|---|---|---|---|
| sin, cos | %3,1, %3,1 | %3,9, %4,4 (1 ulp) | 0 |
| tan | %3,8 | %42 (3 ulp) | 0 |
| asin, acos | %6,0, %7,8 | %5,2, %14,7 | 0 |
| atan, atan2 | %1,0, %17,9 | %1,4, %1,6 | 0 |
| exp, pow | %9,9, %7,8 | %0,1, %0,1 | 0 |
| log, log10, log2 | %0,3, %0,1, %0,2 | 0, %0,01, %0,005 | 0 |
| cbrt | 0 | %7,9 | 0 |
| **hypot** | **%13,7** | **%15,7** | **%16,7** |

- `libm` her platformda aynı sonucu verir ama sonuçların yüzde üçü ile on sekizi arası en yakın sayı değildir (hep
  1 ulp). Bağımsız başvurular doğru yuvarlanmış değeri yazdığı için ürün ile başvuru bugün ancak toleransla
  karşılaştırılabiliyor.
- `pxfm` on üç işlevde 297 812 girdinin hepsinde doğru yuvarladı. Kaynağında donanımın birleşik çarp-topla (FMA)
  işlemini kullanan ve kullanmayan iki yol var; ikisi de sınandı (FMA'sız kopya derlenerek) ve aynı sonucu verdi.
- **`hypot` hiçbirinde doğru yuvarlanmıyor**, `pxfm`'in belgesi “en çok 0,5 ulp” dese de. Örnek: (896,809 549 769 400 7;
  −294,254 883 547 293 4) için doğru değer 943,850 255 627 992 3; üç kitaplık da …924 veriyor (0,58 ulp). Karar
  kesirlerle (karelerin tam karşılaştırması) doğrulandı.

### Uzunluk: bugünkü `js_hypot`

Çizimde en çok hesaplanan değer uzunluktur (702 çağrı). Dört milyon çift (yarısı çizim ölçeğinde rastgele, yarısı
milimetreye yuvarlanmış TM koordinatlarının farkı), aynı makine, aynı derleme:

| | En yakın sayı değil | Süre (çağrı başına) |
|---|---|---|
| `sqrt(x·x + y·y)` | %16,9 | 0,7 ns |
| **`js_hypot` (bugünkü)** | **%35,4** | 1,4 ns |
| `libm::hypot` | %13,7 | 2,9 ns |
| `pxfm::f_hypot` | %16,8 | 2,3 ns |
| platformun `hypot`'u | %16,0 | 1,1 ns |
| **`kentos_num::hypot`** (§6) | **0** | 2,1 ns (birleşik çarpımı olmayan işlemcide bu makinenin ölçüsüyle ~4 ns) |

Bugünkü işlev (en büyüğe bölüp karelerini telafili toplayan biçim) taşmayı önler ama bölme ve geri çarpma iki
yuvarlama daha ekler: her üç uzunluktan biri en yakın sayı değildir.

### Seçenekler

| | Saf Rust | Doğru yuvarlama | Derleme | Not |
|---|---|---|---|---|
| `libm` (bugünkü) | evet | hayır (≤ 1 ulp) | bağımlılık zaten var | belirli ama doğru değil |
| `pxfm` | evet | 13 işlevde ölçüldü; `hypot` hariç | **kilit dosyasında zaten var** (`iced` → `image` → `moxcms`, masaüstü programlarında) | BSD-3 ya da Apache-2.0; 0.1 sürümü; kendi bağımlılığı yok |
| CORE-MATH (`core-math` crate'i) | hayır (C) | kanıtlı | C derleyicisi (bugün de gerekiyor: gömülü SQLite) | MIT; Rust sarmalayıcısı küçük bir proje |
| platformun kitaplığı | — | hayır | — | sistemden sisteme değişir; yasak |

Kaynaklar: CORE-MATH projesi (Inria; doğru yuvarlanan işlevler ve kanıtları,
<https://core-math.gitlabpages.inria.fr/>), IEEE 754-2019 (§9.2 önerilen işlevler ve doğru yuvarlama; §9.6
`minimum`/`maximum`),
`pxfm` (<https://github.com/awxkee/pxfm>; 0.1.30, 6 Temmuz 2026), `libm` (musl'un Rust'a taşınması,
<https://github.com/rust-lang/compiler-builtins>; 0.2.16), `core-math` (<https://github.com/jdh8/core-math>; 1.4.0,
27 Eylül 2026). Hatasız toplam ve çarpım: Knuth'un iki sayı toplamı, Dekker'in çarpımı ve Veltkamp'ın bölmesi
(Muller ve ark., *Handbook of Floating-Point Arithmetic*, 2. baskı, bölüm 4). `hypot`'un kitaplıklardaki durumu:
Borges, “An Improved Algorithm for hypot(a,b)” (2019).

## Karar (öneri)

Sayısal temel dört katmandır. Her biri ayrı ayrı, kendi başvurusuyla gelir.

### 1. Anlam: JavaScript'in tuhaflığı yerine yazılı kural

| Bugün | Olacak | Davranış değişir mi |
|---|---|---|
| `js_min`, `js_max` | `num::minimum`, `num::maximum` (IEEE 754-2019): NaN taşınır, −0 < +0 | hayır; yalnız ad |
| `js_sign` | `num::sign`: sıfır sıfır kalır, NaN taşınır | hayır; yalnız ad |
| `js_floor` | `f64::floor` | hayır |
| `js_round` | kullanım yerine göre adı konmuş yuvarlama: `round_half_away` (değer), `round_half_even` (politika isterse), `floor(x + 0,5)` kalacaksa gerekçesiyle | **evet:** eksi yarımlarda (−2,5 → −3). 159 yer tek tek sınıflanır |
| `js_cmp` | `f64::total_cmp`; NaN beklenmeyen yerde denetimli hata | yalnız NaN'da |
| `truthy`, `or` | `Option` ya da açık koşul | 58 yer tek tek |
| `stable_sort` | `slice::sort_by` (kararlı) | hayır (karşılaştırma tutarlıysa) |
| `js_trim` (4 kopya) | tek `text::trim`: Unicode boşlukları ve BOM | uç karakterlerde |
| `js_number`, `parse_float` (metin → sayı) | tek, katı ondalık okuyucu: boş metin sayı değildir, sondaki çöp reddedilir, `Option` döner | **evet:** `""` artık 0 değil, `"12abc"` artık 12 değil |
| `js_number`, `to_fixed` (sayı → metin) | `display::fixed` (gösterim) ya da en kısa geri dönüşlü yazım (`{}`) | uçlarda (1e21 üstü) |

Davranışı değişen her aile için: çağıran yerlerin sınıflandırması ADR'nin ekine yazılır, kural bağımsız başvuruya
(Python, `fractions`/`decimal`) bağlanır, ortak durumlar başvurudan yeniden üretilir.

### 2. Aşkın işlevler: doğru yuvarlanmış, her platformda aynı

- Hedef: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `log`, `log10`, `log2`, `pow`, `cbrt`, `hypot`
  her girdide en yakın çift duyarlıklı sayıyı verir. O zaman sonuç tanım gereği her platformda aynıdır ve bağımsız
  başvuruyla (aynı tanım) bit bit karşılaştırılır; tolerans yalnız gerçekten yaklaşık olan yerde kalır.
- **`hypot` kendi kodumuzdur** (hiçbir aday doğru yuvarlamıyor; §6): hızlı yol hatasız toplam ve çarpımla hesaplar,
  kendi hatasını sınırlar ve yalnız kanıtladığını yanıtlar; yuvarlama sınırına yakın girdide ve subnormallerde tam
  sayı aritmetiğiyle kesin karar verilir. Başvurusu kesirlerledir.
- **Öbür on üç işlev için öneri `pxfm`'dir:** saf Rust, ek bir derleme adımı istemez, kilit dosyamızda zaten vardır,
  ölçümde hatasızdır. Güvence kitaplığın belgesi değil kendi sınamamızdır: işlev başına zor girdiler ve rastgele
  girdiler depoda durur (`fixtures/numeric/v1/elementary.json`, üreteci `crmath`), her derlemede bit bit denetlenir;
  kitaplığın sürümü sabitlenir, yükseltme o sınamadan geçer.
- CORE-MATH kanıtlıdır ama C'dir: paketleme zincirine (üç işletim sistemi) bir C kitaplığı daha ekler. `pxfm`
  sınamada bir işlevde düşerse o işlev için ona dönülür.

### 3. Sahibin onayına bağlı olan

`pxfm`'in doğrudan bağımlılık olması. Gerekçe (CLAUDE.md §3): bakım (etkin; son sürüm 6 Temmuz 2026, ilk sürüm
Haziran 2025), lisans (BSD-3 ya da Apache-2.0), platform (saf Rust, kendi bağımlılığı yok), ölçüm (yukarıdaki tablo).
Onay gelene kadar §1 ve `hypot` yürür; aşkın işlevler `libm`'de kalır.

### 4. Denetim

- Tek yer: `kentos-num` crate'i (`crates/shared/num`), bütün crate'lerin altında, bağımlılıksız. Geometri
  çekirdeğinin bir modülü olamazdı: sözleşmeler crate'i çekirdeği bilmez ve o da hesap yapar (`libm`'i doğrudan
  çağırıyor); `maplibre` ve `qml` de öyle. Kendi işi olan kendi crate'inde durur.
- Platformun kitaplığına giden yöntemlerin yasağı (clippy.toml) bütün hesap crate'lerine yayılır; liste tek yerde
  durur, crate'ler onu paylaşır.
- `jsmath` adı ve `js_` öneki bekçiyle izlenir: sayı yalnız azalır, sıfıra inince modül silinir.

### 5. Sonuçlar ve geçiş

- Sonuçların son basamağı değişir: `libm` ile doğru yuvarlanmış değer girdilerin yüzde üç ile on sekizinde 1 ulp
  ayrıdır. Altın dosyalar ikiye ayrılır: bağımsız başvurudan gelenler (başvuru zaten doğru değeri yazıyorsa ürün ona
  yaklaşır, tolerans daraltılabilir) ve ürünün kendi çıktısının saklandığı dosyalar (yeniden yazılır; fark 1 ulp
  sınırıyla denetlenir ve sayısı raporlanır).
- `.kcad` sayıları bit bit taşır; dosya biçimi değişmez. Eski dosyadaki koordinatlar aynen okunur; yalnız onlardan
  yeniden hesaplanan değerler (uzunluk, açı) son basamakta değişebilir.
- Sıra, her adım kendi commit'i ve kendi başvurusuyla:
  1. Doğru yuvarlanan `hypot` (kendi kodumuz) ve başvurusu (yapıldı, §6); `js_hypot` ona bağlanır.
  2. Davranışı değişmeyen adlar: `minimum`, `maximum`, `sign`, `floor`, sıralama (mekanik; altın dosyalar aynı).
  3. Yuvarlama: 159 yerin sınıflandırması, sonra değişiklik.
  4. Metin: tek `trim`, tek katı sayı okuyucu, sayı yazımı (BK-50 ile birlikte).
  5. Aşkın işlevler (onaydan sonra): `kentos-num` üzerinden `pxfm`; altın dosyaların ayrıştırılması.
  6. Yasağın bütün hesap crate'lerine yayılması; `jsmath`'in silinmesi.

### 6. Yapılan

**`kentos-num` ve `hypot` (28 Kasım 2026).**

- `crates/shared/num`: bağımlılığı yok; `clippy.toml`'u platformun kitaplığına giden yöntemleri ve birleşik
  çarp-toplayı yasaklar.
- `kentos_num::hypot(x, y)`: √(x² + y²)'nin en yakın çift duyarlıklı sayısı, eşit uzaklıkta çift olan (IEEE 754
  `roundTiesToEven`); sonsuz NaN'ı yener.
  - **Hızlı yol:** kareler ve toplamları hatasız dönüşümlerle çift-çift olarak tutulur, kök bir Newton adımıyla
    düzeltilir. Yolun kendi hatası 2⁻¹⁰¹·r'nin altındadır (kanıt işlevin başında); düzeltilmiş kök, 2⁻⁹⁷·r'lik sınırın
    iki ucunda aynı sayıya yuvarlanıyorsa o sayı kesin yanıttır.
  - **Kesin yol:** sınırın iki ucu ayrı sayılara düşerse (tam orta nokta ya da ona 2⁻⁴⁴ son basamak kadar yakın bir
    değer) ve subnormal girdilerde kareler 256 bitlik tam sayılarla toplanır, kök tam sayı köküyle bulunur, yuvarlama
    orta noktanın karesiyle karşılaştırılarak verilir.
  - Hızlı yol yalnız kanıtladığını yanıtladığı için sonuç hangi yoldan geldiğine bağlı değildir.
  - **Birleşik çarp-topla:** bir çarpımın yuvarlama hatası bir çift duyarlıklı sayıdır; onu birleşik işlem de,
    çarpanları bölerek yapılan hesap da aynen verir. İşlemcide birleşik işlem varsa (ARM64; `fma` ile derlenen x86)
    o kullanılır: hız değişir, bit değişmez. İki yolun eşitliği sınamadadır. Yasak kalan, yuvarlamayı değiştiren
    kullanımdır.
- **Bağımsız başvuru:** `scripts/fixtures/numeric_hypot.py` (yalnız Python'un `fractions`'ı; hiçbir kitaplığın
  `hypot`'u sorulmaz) `fixtures/numeric/v1/hypot.json`'ı yazar: 5 729 durum (çizim ve harita ölçeği, her üs, üsleri
  24–31 ayrı çiftler, subnormaller, en büyük sayıya yakınlar, tam üçgenler, 300 tam orta nokta). Dosya ikinci bir
  yoldan da denetlendi (mpmath 400 bit, bir kez yuvarlama): fark yok.
- **Sınamalar:** her durum dört sırayla ve işaretle bit bit (`crates/shared/num/tests/all/hypot.rs`); hızlı yol ile
  kesin yol sabit bir milyon çiftte aynı sayıyı verir; çarpım hatasının iki hesabı 200 000 çiftte aynıdır.
- **Bedeli:** çağrı başına 2,1 ns (bugünkü 1,4 ns; yukarıdaki tablo).

**`js_hypot` → `kentos_num::hypot` (28 Kasım 2026).** Çekirdeğin uzunluğu artık sayısal temelindir (702 çağrı; ad
bir sonraki adımda değişir). Bağlamadan önce ve sonra koşulanlar: paylaşılan crate'lerin tamamı, yerel hesap
crate'leri (uygulama katmanının 22 682 ortak durumu dahil), masaüstü çekirdeği (2 616), dört modül paketi, birleşik
program, başsız konak, MCP (313), Python kanalında raster oturt durumları.

- **Bağımsız başvuruya karşı:** 22 682 ortak durumdan ikisi oynadı; ikisi de aynı kusuru gösterdi ve kaynağında
  düzeltildi (§7). Geometri çekirdeğinin başvuruya ve kaydedilmiş çağrılara karşı sınamalarında fark yok.
- **Saklanan çıktılar** (ürünün kendi yanıtının dondurulduğu dosyalar) son basamaklarında oynadı. Her dosya,
  farkı `scripts/fixtures/ulp_diff.py` ile okunduktan sonra yazıldı: sayı dışında hiçbir şey değişmedi.

  | Dosya | Sayı | Değişen | En uzak | Not |
  |---|---|---|---|---|
  | `fixtures/geodesy/v1/ground-answers.json` | 460 | 2 | 1 ulp | zemin uzunluğu |
  | `fixtures/svg/v1/cases.json` | 8 050 (yamalanan 69 durumda) | 1 516 | 7 590 ulp; mutlak 2,7·10⁻¹¹ | eğri uydurma, dış hat ve ofset zincirleri; en uzağı `strokeToPath` |
  | `fixtures/style/v1/cases.json` | 2 496 (14 durumda) | 235 | mutlak 9,3·10⁻¹⁰ | ülke koordinatındaki eğrinin örnekleme noktası 0,5 nm kayar; işaretçi açısı 7·10⁻¹⁰ rad |
  | `fixtures/style/v1/svgedit.json` | 11 (1 durumda) | 2 | 2 ulp | çokgen aracının köşesi |
  | DXF yazımının parmak izi (`dxf_write.rs`) | — | 1 sayı, 6 yerde | 1 ulp | eğrinin son düğümü (kiriş boyu); eski ve yeni dosya satır satır karşılaştırıldı |

- **Araç:** `ulp_diff.py` iki sürümü sayı sayı karşılaştırır (ulp, bağıl ve mutlak sınır) ve sayı dışı her farkı
  reddeder. Kaydedicisi kalmamış dosyalar için (web'in kaydettikleri) dondurulmuş sınamalar
  `KENTOS_WRITE_FROZEN=<klasör>` ile şimdi hesapladıklarını yama listesi olarak yazar; `ulp_diff.py --apply` farkı okur
  ve yalnız oynayan sayıları dosyanın metninde yerinde değiştirir (başka hiçbir bayt değişmez).
- **Bu işten bağımsız, önceden var olan kırmızılar** (aynı koşularda görüldü, dokunulmadı): DWG codec'inin
  `layers_blocks_annotations_and_curves_cross_the_codec_boundary` sınaması (17 ≠ 16 nesne; bağlama olmadan da
  kırmızı); iş koşucusunun üç denetim sınaması macOS'ta (kutulama yalnız Linux'ta); Python kanalının tam koşusunda
  `cad.arch.performance` (26) ve `cad.arch.ifc` (14) durumları Python koşucusunun belge kurucusunda takılıyor (eğimsiz
  kalkan çatı, duvarın `curtain` alanı, `compartments` sayısı); `fixtures/raster/v1/georef.json` üretecinin denetimi
  (ortamın GDAL sürümü).

### 7. Geçişin ortaya çıkardıkları

Bir işlevin son basamağı değişince sonucu milimetre oynayan hesap, o basamağa güvenmemesi gereken bir hesaptır. Böyle
her yer kaynağında düzeltilir; tolerans büyütülmez.

**Raster oturt'un öntanımlı pikseli (28 Kasım 2026).** `js_hypot` bağlanınca 22 682 ortak durumdan ikisi oynadı:
ince plaka ile oturtulan rasterin çıktı ızgarasının köşesi 1,4 ve 2,3 mm kaydı.

- **Neden:** ızgaranın köşesi piksel boyunun katına oturtulur (ADR 0204 §6) ve köşe orijinden sekiz milyon piksel
  uzaktadır. Öntanımlı piksel boyu, dönüşümün komşu piksellerdeki değerlerinin farkından hesaplanıyordu; o değerler
  milyonlarca metrelik koordinatlardır, son basamakları bir nanometredir, yarım metrelik pikselin farkı milyarda bire
  kadar doğrudur. Milyarda bir, köşede milimetredir. İnce plakanın denklemleri de yer değiştirmeyi bütün olarak
  (`hedef − piksel`, milyonlarca metre) çözüyordu.
- **Üretecin notu** bunu biliyordu (“son basamaklar köşeyi milimetrenin kesri kadar oynatır”) ve bu durumlara 10⁻³ m
  tolerans vermişti; gerçek gürültü birkaç milimetreydi, durumlar şans eseri sınırın içindeydi.
- **Düzeltme:**
  - `Georef::forward_offset`: dönüşümün sabiti eklenmeden önceki değeri (raster boyutunda bir sayı). Öntanımlı piksel
    onun farklarından hesaplanır (`warp::default_pixel`).
  - `Sheet::solve` denklemleri iki merkezin birbirine taşınması çevresindeki yer değiştirmeler için çözer; taşımayı a₀
    taşır. Lastik levhanın çizimdeki kullanımında (yer değiştirme küçük) sonuç değişmez.
  - Üreteçler aynı tanımı izler (`fwd.local`).
- **Sonuç (ölçüldü):** başvurunun kendi köşe değerleri, gürültüsü giderilince 0,3–0,8 mm oynadı (dört durum). Ürün ile
  bağımsız başvuru arasındaki köşe farkı şimdi en çok 3·10⁻⁸ m (ince plaka), 10⁻⁸ m (projektif, polinom 3); bu
  durumların toleransı 10⁻³'ten öbürleriyle aynı 10⁻⁶'ya çekildi. Hedefleri başka sistemden taşınan tek durumda 10⁻³ kaldı:
  orada PROJ ile KentOS'un izdüşümü hedeflerde nanometre ayrışır ve köşe bunu aynı oranda büyütür (tasarımın kendi
  duyarlılığı; kayıt pikseli saklar, aynı piksel verilince ızgara aynıdır).
- Yeniden projeksiyonun öntanımlı pikseli altı anlamlı basamağa yuvarlandığı için (`six_figures`) etkilenmez.
- Açık: `fixtures/raster/v1/georef.json` üretecinin denetimi bu değişiklikten önce de farklıydı (ortamın GDAL sürümü);
  dokunulmadı.

## Modül ve kanal tablosu

| İş | Mimari | Harita | Yol | CBS | MCP | Python | Eklenti | Yapay zekâ ajanı |
|---|---|---|---|---|---|---|---|---|
| Sayısal temel | var | var | var | var | var: aynı çekirdek | var: aynı çekirdek | var: aynı çekirdek | var: aynı çekirdek |

Hesap tek çekirdektedir; her modül ve kanal aynı işlevi çağırır.
