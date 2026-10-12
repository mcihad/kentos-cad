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

Uygulama bölümü işin sonunda yazılır.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/subref_cases.py` (KentOS kodu olmadan): 17 yer, 25 izleme durumu; çekirdek aynı sonucu verir
  (`geocode` gibi `tests/all/subref.rs`). Başvuru, çizginin ortasının taşınınca kopması hatasını yakaladı.
