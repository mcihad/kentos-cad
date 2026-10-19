# ADR 0601: İç içe ifadeler — `dizi_her`, `dizi_süz`, `değişkenle`, `dene`

- **Durum:** kabul edildi (2026-10-19). ADR 0600'ün ikinci dilimi; sahibin isteği: “qgis expression engine içerisine gerçekten çok
  iyi fonksiyonlar var mesela aggregate falan gibi bir sürü iç içe kullanılabilecek şeyler var”, “ilk önceliğimiz performans”.
- **Bağlam belgesi:** ADR 0100 (sütun motoru), 0214 (diziler ve eşlemeler, `@` değerleri), 0600 (plan).
- **Web:** ertelendi (ADR 0228); çekirdek ortaktır (TODOS.md §0.2 `EXPR-0600`).

## Bağlam

QGIS'te bir ifadenin içinde başka bir ifade her dizi öğesinde ya da bir değişkenle değerlendirilebilir: `array_foreach(arr,
@element * 2)`, `array_filter(arr, @element > 0)`, `with_variable('k', …, @k * @k)`, `try(…, yedek)`. KentOS'un dilinde bir
işlevin bütün bağımsız değişkenleri çağrının yerinde bir kez değerlendiriliyordu; bu işler yazılamıyordu.

QGIS bunları ağaç yürüterek ve her öğe için bağlama bir kapsam ekleyerek değerlendirir. KentOS'un dili 256 nesnelik partilerle
sütun sütun değerlendirilir (ADR 0100); iç içe ifade bu düzeni bozmamalı.

## Karar

### 1. İşlevler

| İşlev | QGIS adı | Ne yapar |
|---|---|---|
| `dizi_her(dizi, ifade)` | `array_foreach` | ifadeyi her öğede değerlendirir, sonuçların dizisi; boş dizi değeri boş dizi |
| `dizi_süz(dizi, koşul, sınır)` | `array_filter` | koşulu doğru olan öğeler; sınır verilirse en çok o kadar (0 sınırsız) |
| `değişkenle(ad, değer, ifade)` | `with_variable` | değeri bir kez hesaplar, ifadede `@ad` |
| `dene(ifade, yedek)` | `try` | ifadenin değeri; değer vermezse yedek ya da boş |

- Gövdede `@öğe` (`@element`) öğe, `@sayaç` (`@counter`) öğenin yeri (0'dan).
- İç içe çağrıda içteki ad dıştakini örter (QGIS'teki gibi). Dıştakinin değeri gerekirse `değişkenle` ile adlandırılır.
- `@` adları çözülürken önce çevreleyen iç içe ifadelere, sonra projenin değişkenlerine bakılır.

### 2. Derleme: gövdenin nesneye bağlı parçaları dışarı alınır

Çözümleyici gövdeyi kendi çerçevesiyle çözer: `@öğe` ve `@sayaç` (ya da `değişkenle`'nin adı) çerçevenin yuvalarıdır
(`Node::Local`). Ardından gövdeden çerçeveye bakmayan en büyük parçalar dışarı alınır (`lambda::hoist`):

- bir alan, bir `$` değeri, nesnenin sırası, başka bir çerçevenin değeri;
- bunlardan kurulan her ifade.

Dışarı alınan parçalar çağrının yerinde nesne başına bir kez, sütun motorunda öteki işlenenler gibi değerlendirilir. Gövdeye
bunların yerine çerçevenin sonraki yuvaları yazılır. Gövdede yalnız çerçeve ve sabitler kalır; nesneyi bilmez.

Bundan çıkan kurallar:

- Başka katmana bakan bir işlev (ADR 0214 §3) gövdede kalırsa, yani kendi bağımsız değişkeni öğeye bağlıysa, ifade derlenmez:
  “onu dışarıda hesaplayın”. Öğeye bağlı değilse dışarı alınır ve nesne başına bir kez çalışır.
- `değişkenle`'nin adı tırnak içinde metin olmalıdır.
- İç içe ifade en çok 32 kat olur.
- Tohumsuz `rastgele` (ADR 0600 §3) gövdede öğenin yerini de tohuma katar: her öğe yeni sayı çeker.

### 3. Değerlendirme: gövde de sütun motorunda

- Gövde kendi küçük programına derlenir (`Code::program`): çerçevenin yuvası, programda alan olarak okunur.
- `dizi_her` ve `dizi_süz`: bir partideki bütün nesnelerin bütün öğeleri sırayla 256'lık parçalara toplanır. Gövdenin programı
  her parçada bir kez koşar; sonuçlar nesne nesne diziye yazılır.
- Sabit bir dizi (`dizi(1, 2, 3)`) bir kez okunur.
- `dene` ve büyük değerli `değişkenle`: gövdenin programı nesneler üzerinde, nesne başına bir kez koşar.
- Küçük değerli `değişkenle` (değerin boyu × okunuşu ≤ 256 düğüm) çözülürken gövdeye yazılır. Program aynı işi bir yazmaçta
  yaptığından değer bir kez hesaplanır; maliyeti elle yazılan eşdeğeriyle aynıdır.
- Tek nesne yolu (`walk`) aynı çerçeve kuralıyla gövdeyi ağaç olarak yürür. İki yolun aynı değeri verdiği 700 nesnelik bir
  karşılaştırmayla sınanır: boş, kısa ve parçadan uzun diziler, sınır, iç içe çağrı, `dene`, iki tür `değişkenle`.

### 4. QGIS'ten ayrılanlar

- `dizi_süz`'de de `@sayaç` vardır (QGIS vermez).
- `dene` ifade boş verdiğinde de yedeğe düşer. QGIS yalnız hatada düşer, ama KentOS'un işlevleri hatada boş verir (`tamsayı('a')`
  boş); ikisini ayırmak kullanıcıya bir şey kazandırmaz.

### 5. Performans

100 000 nesnede, `measures_docs_adr_0601` (sürüm derlemesi, p50):

| İfade | Süre |
|---|---|
| `değişkenle('a', $alan / 10, @a * @a + @a)` | 0,66 ms; elle yazılan `$alan / 10 * ($alan / 10) + $alan / 10` 0,65 ms |
| `dene(tamsayı(Parsel), -1)` | 4,2 ms |
| `dizi_topla(dizi_her(dizi(1, 2, 3), @öğe * $alan))` | 94 ms; aynı dizi doğrudan, `dizi_topla(dizi($alan, 2 * $alan, 3 * $alan))`, 93 ms |
| `dizi_metin(dizi_her(metin_dizi('a,b,c'), büyük(@öğe) \|\| Parsel), '-')` | 41 ms |
| `dizi_her(dizi(1, 2), rastgele(1, 100))` | 15 ms |

- İlk yazımda gövde her öğe için ağaç olarak yürünüyordu: `dizi_her` 115 ms, `değişkenle` 6,3 ms, `dene` 6,7 ms.
- Gövde sütun motoruna geçince iç içe ifadenin kendi maliyeti sıfırlandı: `dizi_her` aynı diziyi doğrudan kurmakla aynı sürede.
- Kalan süre dizilerin işaretli JSON metni olarak taşınmasıdır (ADR 0214 §2.4): her sayının yazılıp yeniden okunması. Bunu
  dilin bütün dizi işlevleri öder. Yazmaçta tipli dizi ayrı bir iştir.

## Doğrulama

- `fixtures/expression/v2/qgis.json`: 23 yeni durum, 20'si QGIS 4.2'nin kendi değeriyle
  (`scripts/fixtures/expression_qgis_cases.py`). Ayrılan üçü: `@sayaç` `dizi_süz`'de, `dene`'nin boş değeri, Türkçe ad.
- `tests/qgis.rs`:
  - her durum iki yoldan (ağaç ve sütun motoru);
  - 700 nesnede iki yolun satır satır eşitliği;
  - gövdedeki tohumsuz çekilişin öğe başına ayrı olması;
  - retler: `değişkenle`'nin adı, 32 kat, öğeye bağlı başka katman çağrısı, bilinmeyen `@` adı.
- İfade oluşturucu: gövdede `@öğe`, `@sayaç` ve `değişkenle`'nin adı tamamlanır (`editor::complete`'in `frame_items`'i).
  İşlevlerin yardım örnekleri derlenip denetlenir.
- `cargo test --release -p kentos-expression --test perf measures_docs_adr_0601 -- --ignored --nocapture`.
