# ADR 0320: Her türün ilişkili ölçüsü, Buda'da bağlar, bağ vurgusu, Zincir ve Baz komutu

- **Durum:** kabul edildi (2026-10-16). [plan/cad.md](../../plan/cad.md) §11.1 `CAD-ANNO-03`, `04`, `07` (sahibin sözleri: “Pull yap
  anno bitene kadar durma”; CAD-ANNO 3–4 maddelik gruplarla). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0249 (alt nesne başvuruları ve ilişkili ölçü), 0255 (`cad.dimensions.create`), 0263 (`along` yeri, blok
  yolu), 0296 (Kır ve Birleştir'de bağların taşınması), 0140 (Zincir ve Baz ölçü).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0249 ve 0255'ten sonra yalnız nokta yöntemlerinin (Hizalı, Doğrusal, Koordinat, Semt, Eğim) noktaları bağlanıyordu. Nesneden
alınan ölçüler (Yarıçap, Çap, Kırıklı yarıçap, Yay uzunluğu, Açı) nesnesi değişince yerinde kalıyordu; Zincir ve Baz ölçü hazır
geometriyle `cad.entities.create`'ten yazıyordu. Buda kesilen nesnenin ikinci parçasındaki bağı koparıyordu (ADR 0296 Kalanlar). Seçili
ölçünün neye bağlı olduğu çizimde görünmüyordu.

## Karar

### 1. Ölçünün kuralı (`DimensionRefs.rule`)

Bir yerin noktası olmayan noktalar kuralla izler:

| Kural | Ölçüler | Bağlananlar | Kuraldan gelen |
|---|---|---|---|
| — | Hizalı, Doğrusal, Koordinat, Semt, Eğim, Açı (köşeden) | yere oturan noktalar | — |
| `curve` | Yarıçap, Çap, Kırıklı yarıçap | `b`: eğrinin üstünde, `b`'ye en yakın noktası (yayın ötesindeyse yakın ucu) | `a` eğrinin merkezi; `b` yerin noktası, yer orada değilse yerden o kadar dönülmüş nokta; kırıklının gösterilen merkezi merkezle kayar |
| `curve` | Yay uzunluğu, Açı (yaydan) | `a`, `b`: yayın iki ucu (Kısmi'nin uçları) | `c` yayın merkezi |
| `edges` | Açı (iki kenardan) | `a`, `b`: kolların durduğu kenarlar, tıklanan yere en yakın noktada | `c` iki kenarın doğrularının kesişimi; her kol boyunu ve köşenin hangi yanında olduğunu korur |

- Kenar ve eğri yeri `along`'dur (ADR 0263): çekirdeğin `ops::subref::edge_place_near` (düz kenar, uçlu yay ya da yuvarlak),
  merkez `curve_centre`, kenarın doğrusu `edge_line`.
- Paralel olan kenarlar kesişmez: ölçü yerinde kalır. Kopan yer kuralın noktalarını yerinde bırakır.
- Kural yalnız ilgili ölçü türünde olur (`rule_problem`): yarıçapta yalnız `b`, öbürlerinde `a` ve `b` bağlı, `c` bağsız; uymayan
  `invalid_refs`. Kendi başına düzenlenen noktası bağını bırakınca kuralı tutmayan ölçünün bütün bağları düşer.
- `cad.entities.create` kural altındaki noktaları yerlerinden yeniden okumaz (açının kolları kenarın yeri değildir).
- `.kcad` şema 60, `FORMATS_VERSION` 69: `refs`'in `rule`'ü; sütunlarda başvuru maskesinin 3. ve 4. bitleri.

### 2. Buda'da bağlar

Buda, kestiği nesnenin bağlı ölçü noktalarını Kır gibi (ADR 0296) yerlerinden arar: nesnenin kendisinde ve ondan yapılan
parçalarda, sırasıyla; bulunan yere ve parçaya bağlanır (`Document::relocate_ties_keeping_ends`). Kesilen kısımda kalan nokta
nesnenin başı, sonu ya da köşesiyse o yer nesnenin şimdiki hâlinde izlenir (kesilen uç yeni uca gider, AutoCAD gibi); başka yer
`vertex_gone` ile kopar. Uzat'ta uç, yerin kuralıyla yeni uca gider. Blok yeniden tanımı ADR 0263 §3'le izler; dış başvuru
(CAD-REF) gelince aynı kuralı kullanacak.

### 3. Zincir ve Baz komutu

`cad.dimensions.create`'in iki yeni ölçüsü: `continue { base, to }` ve `baseline { base, to }`. Temel hizalı ya da doğrusal ölçüdür
(değilse `not_measurable`); Zincir temelin ikinci noktasından `to`'ya, çizgisi temelinkinde; Baz temelin ilk noktasından, bir kat
(değer yüksekliğinin üç katı) dışarıda. İlk noktaları temelin o noktasının bağını izler, `to` yerse bağlanır; stil, görünüş ve yükseklik
temelindir (stil adı verilmedikçe). `at` okunmaz, `offset` `invalid_offset`'tir. Zincir ölçü ve Baz ölçü araçları bununla yazar,
her yazılan bir sonrakinin temelidir; önizleme komutun hesabıdır (`continue_dimension`, `baseline_dimension`). İşlem manifestinde
ikisi `semantic`.

### 4. Bağ vurgusu

Seçili ilişkili ölçülerin (en çok 64) canlı bağları çizimde gösterilir: bağlı nesnenin (blokta parçanın) kenarları vurgu renginde
kesikli, yeri dolu elmas (`kentos_interaction::dimension_bonds`, masaüstünde `marks.rs`'in `bonds`'u). Kopuk bağ uyarı üçgenidir
(ADR 0249).

## Sonuçlar

- Nesnesi taşınan, dönen, ölçeklenen, budanan ya da uzatılan her ölçü türü onu izler; aynı geri alma adımında.
- Python ve ajan Zincir ve Baz'ı temelin kimliği ve noktayla ister; `kentos.cad.dimensions.create`.
- Hızlı ölçü hâlâ `cad.entities.create`'in işlemiyle yazar; noktaları bağlanmaz.
- Kırıklı yarıçapın gösterilen merkezi bağlanmaz, merkezle birlikte kayar.

## Doğrulama

- Bağımsız başvurular (KentOS kodu olmadan): `scripts/fixtures/dimension_create_cases.py` (62 durum: türlerin bağları ve kuralları,
  köşeden açının yerleri, Zincir ve Baz, retleri) ve yeni `scripts/fixtures/dimension_follow_cases.py`
  (`fixtures/commands/v1/cad.dimensions.follow.json`, 20 durum: taşıma, ölçekleme, döndürme, silme, Yeniden bağla, Buda'nın ucu ve
  ortası, Uzat, Zincir ve Baz'ın izlemesi). Aşkın değerli adımlar `within` ile; çalıştırıcılar nesne beklentisinde de `within`'i okur.
- KCAD: `dimension-rules.kcad`, `broken/dimension-rule-in-schema-59.kcad`, `broken/dimension-rule-unknown.kcad`; bağımsız yazıcı ve okuyucu.
- Resimler: `KENTOS_SHOTS_ONLY=olcu-bag,olcu-izleme,olcu-zincir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
