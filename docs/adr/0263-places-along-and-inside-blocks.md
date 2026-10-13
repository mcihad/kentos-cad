# ADR 0263: Eğri üzerindeki yer, eğri aralığı ve blok içi alt nesne başvurusu

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §6.1 `CAD-DATA-13`'ün kalanı: ADR 0249 köşe, kenar ortası, uç, merkez
  ve düğümü anıyordu; eğri aralığı ve blok içi alt nesne açık kalmıştı. Kapsamı ben belirledim (sahibin sözleri: “CAD data bölümüne geç ve
  orayı tamamla”, “durmadan devam et”).
- **Bağlam belgesi:** ADR 0249 (alt nesne başvuruları ve ilişkili ölçü), 0144 (blok), 0255 (`cad.dimensions.create`), 0261 (UCS).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

- ADR 0249'un yerleri nesnenin ayrık noktalarıdır: uç, köşe, kenar ortası, merkez, düğüm. Kenetin **Yakın**, **Dik**, **Teğet** ve
  **Çeyrek** türleri nesnenin üstünde ama bu noktalardan birinde olmayan bir yere oturur; ilişkili ölçü orada serbest kalıyordu.
- Bir yerleştirmenin (blok) içindeki çizgiye kenetlenen ölçü de serbest kalıyordu: başvuru yalnız çizimin nesnesini anıyordu,
  yerleştirmenin parçasını anamıyordu.
- Yay uzunluğu ölçüsü ve ileride kesik ölçüler, tarama sınırları gibi işler bir nesnenin **iki yeri arasındaki parçasını** anmak ister.

## Karar

### 1. Eğri üzerindeki yer: `along`

Yeni yer `along { part, ring, index, t }`: nesnenin bir kenarının ya da eğrisinin parametresiyle bir noktası, `t` 0 ile 1 arasında.

| Nesne | `part`, `ring`, `index` | `t`'nin anlamı |
|---|---|---|
| çizgi | 0, 0, 0 | `a + t·(b − a)` |
| çoklu çizgi, alan | kenarın parçası, halkası, sırası (ADR 0249 §1) | düz kenarda `a + t·(b − a)`; bükümlü kenarda yayın açısı `a0 + t·süpürme` |
| yay | 0, 0, 0 | açı `a0 + t·süpürme(a0, a1)` (saat yönünün tersine) |
| daire | 0, 0, 0 | açı `t·2π`, doğudan |
| elips ve elips yayı | 0, 0, 0 | parametre `t0 + t·süpürme(t0, t1)` |

- `t` sonlu ve `[0, 1]` içinde olmalıdır; değilse başvuru reddedilir.
- **Kenetten yer (`place_at`):** Yakın, Dik, Teğet ve Çeyrek'in noktası nesnenin sırasıyla ilk kenarında ya da eğrisinde aranır; nokta
  kenara (eğriye) `max(1e-9, 1e-14·|koordinat|)` içinde yakınsa `t` izdüşümden ya da açıdan hesaplanır, `[0, 1]`'e kırpılır. Kesişim
  iki nesnenin üstündedir, hangisini andığı belirsizdir: yer değildir. Uç, Orta nokta, Merkez ve Nokta önce ADR 0249'un ayrık yerlerini
  arar.
- **İzleme:** yerin yapısı ADR 0249 §2'deki gibidir: çizginin, yayın, dairenin ve elipsin yapısı yoktur, yer aynı `t`'de kalır (budanan
  yayda yer süpürmenin aynı oranındadır, AutoCAD'in parametreli ilişkisi gibi). Çoklu çizgi ve alanda halkanın köşe sayısı ve kapalılığı
  aynıysa aynı yer; değilse kenar iki ucuyla, kenar ortası gibi bit bit aranır ve bulunursa yer o kenara aynı `t` ile bağlanır.

### 2. Eğri aralığı: `SubRange`

`SubRange { uid, path, from, to }`: aynı nesnenin iki `along` yeri arasındaki parçası, nesnenin yönünde `from`'dan `to`'ya. İki yer aynı
parçada ve halkada olmalıdır. Kapalı halkada, dairede ve tam elipste `to` `from`'dan önceyse aralık başlangıcın üstünden dolanır; açık
yolda, çizgide ve yayda `to` `from`'dan önce olamaz.

- **Uzunluk (`ops::subref::range_length`):** çizgide ve düz kenarda kirişin oranı, yayda ve bükümlü kenarda `r·|süpürme|·Δt`, yolda
  aradaki kenarlar tam, uçlardakiler kısmen; elipste parçanın elips yayının uzunluğu (çekirdeğin `ellipse_length`'i).
- **İzleme:** iki uç ayrı ayrı izlenir; biri koparsa aralık kopar; ikisi farklı halkaya düşerse `vertex_gone`.
- Bugün aralığı saklayan nesne yoktur; tür, kuralı ve çekirdekteki işlevi sözleşmededir. İlk tüketicisi yay uzunluğu ölçüsünün ilişkisi
  (`CAD-ANNO`) olacak.

### 3. Blok içi alt nesne: `path`

`SubRef`'in yeni alanı `path` (boşsa yazılmaz): `uid` bir yerleştirmeyse, tanımın içindeki parçanın yolu. Yolun her sayısı o düzeydeki
tanımın `entities` dizisindeki sıradır; iç içe yerleştirmede bir sonraki sayı içteki tanımın sırasıdır. Son sayı yerleştirme olmayan bir
nesneyi anar; yer o nesnenin, yerleştirmenin dönüşümüyle çizime konmuş hâlindeki yeridir.

- Yol en çok 16 düzeydir (blok çekirdeğinin `MAX_DEPTH`'i). Blok özniteliklerinin yazıları anılmaz.
- **Kenetten yol:** yerleştirmeye kenetlenen noktanın yeri yerleştirmenin parçalarında, çekirdeğin düzleştirme sırasıyla aranır; ilk
  bulunan parçanın yolu yazılır.
- **İzleme:** yerleştirme değişince (taşındı, döndü, ölçeklendi) önceki ve sonraki yerleştirmenin aynı yoldaki parçası ADR 0249 §2'nin
  kuralıyla izlenir. Bloğun tanımı değişince (yeniden tanımla, taban noktası) o adımın önceki ve sonraki tanımlarıyla aynı izleme
  yapılır: yoldaki parça hâlâ varsa yer onda aranır; yol yoksa ya da sonda yerleştirme olmayan nesne yoksa başvuru kopar, yeni neden
  `piece_gone` (“blok parçası yok”). Yerleştirme silinirse ya da patlatılırsa `object_gone`.

### 4. Dosyalar ve komutlar

- **KCAD v2 şema 51:** yerin `along` türü (`part`, `ring`, `index` sayı, `t` ondalık) ve başvurunun `path`'i (sayı dizisi). Yazıcı 51'i
  yalnız biri varken yazar; eski okuyucu ikisini de `unknown_field` ile reddeder (kanonik sırada `t`, `kind`'dan önce gelir).
- `cad.dimensions.create` ve `cad.dimensions.reassociate` `along` yerini ve `path`'i alır; yer yoksa `no_such_place`, yol yoksa yeni
  `piece_gone` reddi. Python ve MCP katalogdan.
- Masaüstünde ilişkili ölçünün kenetlenen noktası Yakın, Dik, Teğet ve Çeyrek'te de, yerleştirmenin parçasında da bağlanır;
  Öznitelikler'in İlişki satırı yeri “kenar üstünde %40” ve “blokta” diye söyler.

## Sonuçlar

- Ölçü nesnenin üstündeki herhangi bir yere ve bloğun içindeki çizgiye bağlanır, onlarla birlikte izler.
- Eğri aralığı anılabilir; uzunluğu çekirdekten gelir.
- Eski çizimler değişmez: şema 50 ve öncesi aynı bayttır.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/subref_cases.py --check`: `along`'un noktaları (çizgi, düz ve bükümlü kenar, yay, daire, elips),
  izlemesi (taşıma, yapı değişimi, budanan yay, silinen kenar), kenetten yeri (Yakın, Dik, Çeyrek, Kesişim değil), aralığın uzunlukları ve
  izlemesi, blok yolunun parçaları (iç içe, döndürülmüş ve aynalı yerleştirme) ve izlemesi (taşınan yerleştirme, yeniden tanımlanan
  blok, silinen parça); KentOS kodu olmadan. Çekirdekte `tests/all/subref.rs`.
- Komutlar: `cad.dimensions.create` ve `cad.dimensions.reassociate`'in yeni ortak durumları (`dimension_create_cases.py`,
  `dimension_command_cases.py`).
- KCAD: `subref-extras.kcad` ve iki bozuk dosya, bağımsız Python okuyucu ve yazıcısı.
- Belge: `kentos-domain`'in `dimension_ties.rs`'inde yerleştirmenin taşınması ve bloğun yeniden tanımlanması.
- Masaüstü resimleri: `KENTOS_SHOTS_ONLY=yer-yakin,yer-blok cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-yer-*`).
