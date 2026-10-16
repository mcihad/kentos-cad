# ADR 0322: Merkez işareti, eksen çizgisi ve kot; stil dosyaları; stilden ayrılan alanlar

- **Durum:** kabul edildi (2026-10-16). [plan/cad.md](../../plan/cad.md) §11 `CAD-ANNO-11`, `16`, `17` (sahibin sözü: “Pull yap anno
  bitene kadar durma”; 3–4 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0264 §4 (merkez işareti ve eksen çizgisinin kaynak modeli), 0320 (her türün ilişkili ölçüsü), 0321 (değerin
  yazımı), 0183 (yazı ve ölçü stilleri), 0248 §4 (`cad.styles.edit`), 0142 (köşe kotu), 0296 (Yönü çevir'de bağlar).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0264 §4 merkez işaretini ve eksen çizgisini, kaynak nesneden geometrisini türeten ve onu izleyen iki yeni açıklama türü olarak
tanımlamıştı. ADR 0320'den beri ölçü, bağlı noktaları ve **kuralı** (`curve`, `edges`) ile tam bu işi yapıyor: yeri, izlemesi, kopan bağı,
bağ vurgusu, Öznitelikler'i, DXF'i, Python'u ve MCP'si var. Kot işareti (planda bir noktanın kotu) de bir noktaya bağlı açıklamadır.

Yazı ve ölçü stilleri projede tanımlanıp değiştiriliyordu ama başka çizime taşınamıyordu (yalnız Başka çizimden al, ADR 0193). Stil
değişince nesnenin hangi alanının stili izleyeceği **değer eşitliğinden** tahmin ediliyordu: elle değiştirilmiş bir alan stilin eski
değerine denk gelince (stil önce o değere, sonra başka değere geçince) elle verilen değer kayboluyordu.

## Karar

### 1. Üç yeni ölçü türü (yeni nesne türü değil)

ADR 0264 §4'ün “yeni açıklama türü” yerine `DimensionStyle`'ın üç yeni türü: izleme, bağ, Öznitelikler, DXF, Python ve MCP ölçününkiyle
gelir; ikinci bir bağ mekanizması yazılmaz.

| Tür | Noktalar | Kural | Çizim | Değer |
|---|---|---|---|---|
| `centerMark` (Merkez işareti) | `a` merkez, `b` eğrinin üstünde | `curve` (yarıçap gibi: yalnız `b` bağlı) | artı `height` boyunda, `a`→`b`'den `angle` derece dönük; dört eksen çizgisi artıdan `height/4` sonra başlar, eğriyi `offset` kadar aşar (uzatma çizgileri) | yok |
| `centerLine` (Eksen çizgisi) | `a`, `b` eksenin uçları | yeni `pair`: `a` ve `b` iki düz kenarın yerleri | `a`'dan `b`'ye, iki uçta `offset` kadar uzayan tek çizgi (uzatma çizgisi) | yok |
| `elevation` (Kot) | `a` nokta, `b` değerin omzu | yok; `a` yerine bağlanır | tepesi `a`'da, kenarı `height` üçgen; `b` uzaktaysa üçgenin tepesinden `b`'ye kol; `b`'den değerin boyunca omuz | `za`, işaretli: `+3.20`, `-1.50`, `±0.00` |

- **`pair` kuralı:** eksen birinci kenarın başlarının ortasından sonlarının ortasına gider; ikinci kenar birinciye ters yöndeyse çevrilir
  (AutoCAD'in CENTERLINE'ı gibi). Kenarlar taşınınca, döndürülünce, ters çevrilince eksen yeniden hesaplanır.
- **Kot:** `za` yazılır ya da noktanın köşesinin kotundan alınır (`missing_elevation`). Bağlı noktanın köşesi kot alınca, değişince
  işaret onu alır; köşenin kotu kalkarsa işaretinki kalır. Değerin yazımı `Format::dimension_value`'nun `level` birimi: metre, artış,
  ölçünün basamağı (yoksa 2), işaret; merkez işareti ve eksen (`none`) yazı yazmaz, etiket kaydı da yoktur.
- `cad.dimensions.create`'in üç yeni ölçüsü: `centerMark { of }`, `centerLine { first, second }`, `elevation { a, z? }`. Merkez işaretiyle
  eksenin `offset`'i uzantıdır (yoksa değer yüksekliği; eksi `invalid_offset`), uzatma çizgilerinin tipi görünüş vermezse kesik noktalı.
  Kot uzaklık almaz. Aynı kenar iki kez `no_dimension`. `cad.dimensions.query` türleri adlarıyla verir.
- **Araçlar:** Merkez işareti (eğriye tıkla, sürer), Eksen çizgisi (iki düz kenar), Kot (nokta; kotu yoksa sorulur; değerin yeri); üçü de
  komutla yazar, önizleme komutun hesabıdır (`kentos_interaction::dimension_marks`). CAD › Açıklama › Ölçü panelinde.
- **Datum:** Koordinat ölçüsü (ADR 0147) bağlı noktasını zaten izler; izleme durumlarıyla sınanır.
- Kır, Birleştir ve Yönü çevir'de bağ, ölçünün noktasıyla değil yerin adımdan önceki noktasıyla aranır (`Document::relocate_ties`):
  kural altındaki noktalar (eksenin ucu, açının kolu) yerin üstünde değildir. Yönü çevrilen kenara bağlı eksen artık kopmaz.
- DXF'e hizalı DIMENSION olarak, kendi çizgileriyle ve KENTOS verisiyle gider, geri okunur (bağları okunmaz); değer yazısı `" "`.
- `.kcad` şema 62, `FORMATS_VERSION` 71: türler ve `pair`; sütunlarda türler 10–12, kural maskesinin iki biti birlikte.

### 2. Stil komutları ve dosyası (`CAD-ANNO-16`)

`cad.styles.edit`'e `export` ve `import`:

- `export`: türün stilleri (ya da `styles`'ın adlandırdıkları, tablonun sırasıyla) `kentos.annotation-styles` dosyasının kanonik metni olarak
  `library`'de: `{"format", "version": 1, "kind", "styles": [tanımlar]}`. Yazmaz.
- `import`: `library`'deki aynı türün stilleri adlarıyla (kırpılmış, küçük harfe çevrilmiş): yeni ad eklenir (kimliği boşsa yeni),
  tablodaki ad olduğu gibi kalır (`skipped`) ya da `replace` ile yerinde, kimliği ve adıyla güncellenir ve nesneleri izler (`replaced`).
  Kodlar `no_library`, `invalid_library`; tablo kurala uymazsa `invalid_style`.
- Yazı stilleri ve Ölçü stilleri pencerelerinde **Dışa aktar…** ve **Al…** (kaydedilmemiş taslak varken Al… kapalı).
- Kılavuz ve tablo stilleri yoktur: kılavuzun adlı stili `CAD-ANNO-25`, tablonun hücre stili `CAD-ANNO-28` ile gelir; komut aynı
  `kind` alanıyla genişler. Python `kentos.cad.styles.edit`, MCP aracı katalogdan.

### 3. Stilden ayrılan alanlar (`CAD-ANNO-17`)

Değerlendirme: değer eşitliğinden tahmin, elle verilen değerin stilin eski değerine denk geldiği her durumda yanılır; açık bayrak doğru
olandır. Karar:

- Yazının ve ölçünün yeni `overrides` alanı: stilinden elle ayrılan alanların adları (`TEXT_OVERRIDES`: yazı tipi, kalın, italik, eğiklik,
  genişlik, yükseklik; `DIMENSION_OVERRIDES`: görünüşün alanları ve yükseklik). Boş liste yazılmaz.
- **İzleme:** stil değişince listedeki alan kendi değerini korur; listede olmayan alan eski kuralla, stilin eski değerini taşıyorsa yeniyi
  alır (`follow_text_style`, `follow_dimension_style`).
- **Yazılması:** belge her adımın sonunda, adımın değiştirdiği adlı stilli yazı ve ölçülerin listesini adımdan yazar
  (`kentos_domain::style_overrides`): adımın değiştirdiği alan stilin değerinden farklıysa listeye girer, aynıysa çıkar; değişmeyen alan
  adımın bıraktığı listedeki gibidir. Başka stile geçen nesnenin listesi boşalır; stili kalkan ya da projede olmayan nesneninki de.
  Yeni nesne yapıldığı listeyle kalır (yapıştırılan kopyanınki). Bu yüzden stil komutunun izlemesi listeyi bozmaz, geri alma onu da geri alır.
- **Eski dosyalar:** listesi olmayan nesne eskisi gibi izler; göç yoktur, dosya bir adım onu değiştirene dek aynı kalır.
- Öznitelikler'de **Stilden ayrılan** satırı (Türkçe adlarla) ve **Stile döndür** (stilin değerleri, liste temizlenir).
- `.kcad` şema 63, `FORMATS_VERSION` 72: yazının ve ölçünün `overrides`'ı; sütunlarda yazının 14. seçeneği, ölçünün “more” JSON'u.

## Sonuçlar

- Merkez işareti, eksen ve kot nesnelerini izler; Python ve ajan onları nesnenin kimliği ve tıklanan yerle ister.
- Stiller dosyaya verilir ve alınır.
- Elle verilen stil alanı stil değişikliklerinde kaybolmaz; eski çizimler eskisi gibi davranır.
- Kotun omzu yazının genişliğini tahminle (harf başına 0,6 yükseklik) bulur.
- Eksen yalnız iki düz kenar arasındadır; iki yay arası eksen (ADR 0264'ün sözü) ileri iştir.

## Doğrulama

- Bağımsız başvurular (KentOS kodu olmadan): `scripts/fixtures/dimension_create_cases.py` (83 durum: işaret, eksen, kot ve retleri),
  `dimension_follow_cases.py` (28: dairenin taşınması, büyümesi, yayın dönmesi; kenarın kayması, ters çevrilmesi, silinmesi; köşenin kotu),
  `dimension_value_cases.py` (34: kotun yazımı), `style_edit_command_cases.py` (47: dışa ve içe aktarma, geçersiz kılmalar),
  `annotation_style_cases.py` (geçersiz kılınanla izleme).
- KCAD: `dimension-marks.kcad`, `overrides.kcad`, sekiz bozuk dosya; bağımsız yazıcı ve okuyucu.
- Araç testleri `kentos-interaction`'ın `dimension_marks`'ı.
- Resimler: `KENTOS_SHOTS_ONLY=olcu-isaretler,olcu-isaret-onizleme,olcu-isaret-izleme,stil-dosya,stil-gecersiz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
