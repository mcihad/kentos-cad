# ADR 0249: Alt nesne başvuruları ve ilişkili ölçü

- **Durum:** kabul edildi (2026-10-12). [plan/cad.md](../../plan/cad.md) §24.1'in altıncı ve yedinci işleri birlikte (TODOS.md `CADP-06`,
  `CADP-07`; plan `CAD-DATA-13`, `14`, `DEC-03`, `CAD-ANNO-01`–`06`). Sahibin isteğiyle (12 Ekim: “Muhakkak çalıştığını doğrula hem
  arayüz hem de testler ile”) iki iş tek parçada yapılır: başvurunun kuralı arayüzde ilişkili ölçüyle doğrulanır. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0061 ve 0147 (ölçü türleri), ADR 0186 §6 (ilişkili tarama: kayıttan önce izleme), ADR 0175 §4 (bağlı yazı),
  ADR 0014 (kalıcı kimlik), ADR 0025 (KCAD v2), ADR 0244 (CAD işlem manifesti).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

`DimensionEntity` ölçtüğü noktaları (`a`, `b`, `c`) koordinat olarak taşır. Ölçülen duvar taşınınca ya da köşesi çekilince ölçü
yerinde kalır, değeri eskir. AutoCAD'in ilişkili ölçüsü geometriyi izler; kopan ilişki görülür ve yeniden bağlanır.

Bunun için önce bir nesnenin üstündeki bir **yer**i (köşe, uç, merkez, kenar ortası) kalıcı olarak anabilmek gerekir (plan
`CAD-DATA-13`, `14`). Bu başvuru köşe sırasına körü körüne bağlanamaz: köşe eklenip silindiğinde sıra kayar.

## Karar

### 1. Yer ve başvuru

Bir **yer** (`SubPlace`), `kind` etiketiyle şunlardan biridir:

| Yer | Nesne | Anlamı |
|---|---|---|
| `node { part }` | nokta | noktanın yeri; 0 kendisi, `k` çok noktalının `k`'ıncı ek noktası (ADR 0174) |
| `start`, `end` | çizgi, yay, elips yayı | baş ve son |
| `center` | daire, yay, elips | merkez |
| `vertex { part, ring, index }` | çoklu çizgi, alan | köşe: parça (0 kendisi), halka (0 dış, `h` `h`'ıncı delik), sıra |
| `mid { part, ring, index }` | çizgi (hepsi 0), çoklu çizgi, alan | kenarın ortası: `index`'ten sonrakine; bükümlü kenarda yayın ortası |

Bir **başvuru** (`SubRef`), nesnenin kalıcı kimliği (`uid`, ADR 0014) ile yeridir. Kopmuşsa `broken` nedenini taşır.

### 2. Başvurunun izlenmesi (`ops::subref::follow`)

Nesne değişince yerin nereye gittiği çekirdeğin tek kuralıyla bulunur:

1. Nesne silinmişse ya da yerini yeni nesneler almışsa (Patlat): kopar, `object_gone`.
2. Başka türe döndüyse (daire yaya kırıldı): kopar, `kind_changed`. Çoklu çizgi ile alan tek ailedir.
3. Önceki nesnede böyle bir yer yoksa: `no_such_place`.
4. **Yapı aynıysa** yer aynı kalır, noktası yeni nesneden okunur: nesne taşınsa, dönse, köşesi çekilse de izler. Yapı yalnız yolda
   (yerin parçasında ve halkasında köşe sayısı ve kapalılık) ve çok noktalıda (nokta sayısı) vardır. Çizginin ortasının, yayın başının
   yapısı yoktur.
5. **Yapı değiştiyse** (köşe eklendi ya da silindi, delik ya da parça değişti) köşe ve düğüm, eski koordinatlarıyla **bit bit** yeni
   nesnenin köşeleri arasında aranır. Düzenlemenin dokunmadığı köşe bitlerini korur. Bir kez bulunursa yer oraya bağlanır; hiç
   bulunmazsa `vertex_gone`, birden çok bulunursa `ambiguous`. Kenar ortası, iki ucu bir halkada ardışık bulunursa bağlanır.

Kesin karşılaştırma her platformda aynı sonucu verir. Aynı adımda hem yapısı değişen hem köşesi taşınan yer kopar. Kopan yer son
koordinatlarını korur (`CAD-ANNO-02`).

### 3. İlişkili ölçü

- `DimensionEntity.refs` (`DimensionRefs`): `a`, `b` ve `c` noktalarının isteğe bağlı başvuruları. Bağsız nokta serbesttir; ölçü
  kısmen ilişkili olabilir.
- Kapsam: nokta yöntemleri. Hizalı, Doğrusal, Koordinat, Semt ve Eğim. Nesneden alınanlar (Yarıçap, Çap, Yay uzunluğu, Kırıklı
  yarıçap, Açı) bu işte bağlanmaz; ilerisi için Sonuçlar'da.
- **Ölçü aracı:** nokta kenetle bir nesnenin uç, köşe, merkez, orta ya da düğüm yerine oturduysa (`place_at`: kenetin türü ve tam
  noktası nesnenin yerlerinden birine eşit), o nokta bağlanır. Kenetsiz ya da başka kenetle alınan nokta serbesttir.
- **İzleme (kayıttan önce):** bir adım bağlı ölçülerin nesnelerini değiştirdiyse, adım yazılmadan önce ölçünün bağlı noktaları §2'nin
  kuralıyla yeni yerlerine taşınır. Yeni yer başvuruya yazılır; kopan başvurunun nedeni `broken`'a yazılır, noktası olduğu gibi kalır.
  Bunlar aynı geri alma adımındadır (`CAD-ANNO-06`); masaüstünde `kentos_domain` (`dimension_ties.rs`), ilişkili taramanın izlemesiyle
  aynı yerde.
- **Kendi başına düzenlenen ölçü:** ölçünün bağlı noktası, nesnesi değişmediği bir adımda değiştiyse (ölçü tek başına taşındı, ucu
  tutamaçla çekildi) o noktanın bağı düşer, nokta serbest kalır. Nesnesiyle birlikte taşınan ölçü bağlı kalır.
- **Kopyası bağsızdır:** Kopyala, Dizi, Yapıştır, bloğun tanımı (`unlinked`).
- **Yeniden bağla:** `cad.dimensions.reassociate` v1. Girdi: ölçü, nokta (`a`, `b`, `c`) ve yeni başvuru (ya da `null`: serbest).
  Komut noktayı başvurunun yerinden kendisi hesaplar; plan ölçünün yeni değerini söyler. Tek adım, “Ölçüyü yeniden bağla”.
  Masaüstünde **Yeniden bağla** aracı: ölçüye, sonra yeni yere (kenetle) tıklanır; önizleme yeni ölçüyü gösterir.
- **Görünüş:** kopuk başvurusu olan ölçünün kopan noktasında çizimde küçük bir uyarı işareti. Öznitelikler'de **İlişki** satırları:
  “Bağlı (2/2)”, “Kısmen bağlı (1/2)”, “Kopuk: köşe silindi”, “Serbest”. Nedenlerin Türkçe adları:

  | Kod | Söz |
  |---|---|
  | `object_gone` | nesne silindi |
  | `kind_changed` | nesnenin türü değişti |
  | `no_such_place` | yer nesnede yok |
  | `vertex_gone` | köşe silindi ya da taşındı |
  | `ambiguous` | köşe birden çok yerde |

### 4. Kayıt

- `.kcad` şema 43: ölçünün `refs`'i. Yazıcı 43'ü yalnız bir ölçünün başvurusu varken yazar; eski şemada `refs` bilinmeyen alandır.
- `FORMATS_VERSION` 55. Bağımsız Python okuyucusu ve yazıcısı, spesifikasyon ve örnek dosya `dimension-refs.kcad` aynı değişiklikte.
- Bulut projesinde ölçünün tanımı başvurularıyla gider; sunucu sözleşmenin türünü denetler.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/subref.rs` (`SubPlace`, `SubRef`, `DimensionRefs`, `SUBREF_BROKEN`, `broken_words`);
  `DimensionEntity.refs` ve `EntityGeometry::Dimension.refs`; komut `cad_dimensions.rs` (`DimensionsReassociate`, `PointBinding`,
  `PlaceOn`, çıktı ve plan). Ret kodları `invalid_refs`, `not_a_dimension`, `no_such_place`.
- **Çekirdek:** `kentos_geometry_core::ops::subref` (`point`, `follow`, `place_at`). Yer yalnız yolda ve çok noktalıda yapı taşır;
  `place_at` kenetin türünü ve tam noktasını nesnenin yerlerine eşler (Uç nokta: baş, son ve köşeler; Orta nokta: ortalar; Merkez;
  Nokta: düğümler).
- **Belge (`kentos_domain`):** `dimension_ties.rs`'in `follow_dimensions`'ı `commit`'te taramaların izlemesinden sonra çalışır;
  deponun `ties` dizini canlı başvuruların nesnelerini de tutar, `broken` dizini kopuk başvurulu ölçüleri (çizimin işareti tarama
  yapmaz). Kendi başına düzenlenen ölçünün noktası, adım o noktaya yeni bir başvuru vermediyse bağını bırakır.
- **Uygulama:** `dimensions_reassociate.rs` (komut); `edit.rs`'in Update'i başvurusuz geometride ölçünün başvurularını korur
  (Öznitelikler'in yazmaları çekirdeğin şeklinden kurulur, şekil başvuru taşımaz); `unlinked` kopyanın başvurularını düşürür;
  oluşturma ve düzenleme başvuruları `invalid_refs` ile denetler. Başsız sunucu, Python (`kentos.cad.dimensions.reassociate`) ve MCP
  komutu katalogdan alır.
- **Etkileşim:** ölçü aracı (`dimension.rs`) noktayı aldığı kenetin yerini `places`'te tutar ve yazarken `refs`'e koyar; Yeniden bağla
  aracı `dimension_reassociate.rs` (ölçü, sonra her noktanın yeri; Enter olduğu gibi, S serbest; önizlemede yeni ölçü ve değeri);
  Öznitelikler'in Serbest bırak'ı `properties::free_dimensions` (tek adım).
- **Masaüstü:** Öznitelikler'de İlişki satırı (`rows/dimension.rs`'in `relation`'ı) ve Serbest bırak; çizimde kopan noktada uyarı üçgeni
  (`marks.rs`'in `broken`'ı, belgenin `broken_dimensions`'ından); şeritte CAD › Açıklama › Ölçü'de Yeniden bağla
  (`catalog/v1/additions.json`, ikon `dimensionReassociate`); işlem manifestinde `semantic`.
- **Kayıt:** `.kcad` şema 43 ve `FORMATS_VERSION` 55 (§4); tipli sütunlarda ölçünün `REFS` işareti (bit 28).
- **İz oynatıcısı:** nesne beklentisinde `refs` (`a`, `b:vertex_gone`).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/subref_cases.py` (KentOS kodu olmadan): 17 yer, 25 izleme durumu; çekirdek aynı sonucu verir
  (`geocode` gibi `tests/all/subref.rs`). Başvuru, çizginin ortasının taşınınca kopması hatasını yakaladı.
- Ortak komut durumları `fixtures/commands/v1/cad.dimensions.reassociate.json` (26), bağımsız başvuru
  `scripts/fixtures/dimension_command_cases.py` (yerler tanımlarından, değerler kesin); masaüstünde ve Python'da koşar.
- Belge: `crates/native/domain/tests/all/dimension_ties.rs` (köşe çekilince izler ve tek geri alma, eklenen köşede yeniden bağlanır,
  silinen köşede ve silinen nesnede kopar, kendi başına taşınan uç bağını bırakır).
- Uygulama: `crates/native/application/tests/all/reassociate.rs` (izler, kopar, yeniden bağlanır ve yine izler; kopya bağsız;
  Öznitelikler'in başvurusuz yazması bağları korur).
- Kayıt: `cargo test -p kentos-kcad` (`dimension_refs.rs`), bağımsız yazıcı ve okuyucu (`kcad_v2_reference.py`, `tools/kcad/kcad.py`),
  `dimension-refs.kcad` ve yedi bozuk dosya.
- İz: `fixtures/interaction/v1/dimension-refs.json` masaüstünde üç kipte (us, tr-q, hidpi); bir beklenti bilerek bozulunca düştüğü
  görüldü. Resimler `kentos-cad kullan dimension-refs`.

## Sonuçlar

- Nesneden alınan ölçüler (Yarıçap, Çap, Yay uzunluğu, Kırıklı yarıçap, Açı) ve Zincir, Baz ve Hızlı ölçü bu işte bağlanmaz.
- Blok içindeki yerler (iç içe yol) ve parçalanan ya da birleşen nesnelerde yerin eşlenmesi (`DEC-03`'ün kalanı) açık.
- Kesişim, teğet ve yakın kenetleri yer değildir; bunlarla alınan nokta serbesttir.

