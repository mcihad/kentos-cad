# ADR 0321: Ölçü aralığı, kırma, değerin yazımı ve elle yazılan değer

- **Durum:** kabul edildi (2026-10-16). [plan/cad.md](../../plan/cad.md) §11.1 `CAD-ANNO-08`, `09`, `10` (sahibin sözü: “anno bitene kadar
  durma”; 3–4 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0183 (ölçü stilleri ve görünüşü), 0205 (ölçünün çizgileri), 0262 (ayak ve inç), 0149 (gösterim kuralı), 0255
  (`cad.dimensions.create`), 0320 (her türün ilişkili ölçüsü).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Ölçünün değeri projenin birimi ve basamağıyla, öneki ve sonekiyle yazılıyordu. AutoCAD'in DIMSPACE'i (paralel ölçüleri eşit aralıkla
dizmek), DIMBREAK'i (kesen nesnelerde çizgileri kırmak), taşınan değer ve sığmayan değerin dışarı alınması yoktu; yuvarlama artışı, sıfır
bastırma, alternatif birim, tolerans ve ölçünün kendi ayak-inç biçimi de yoktu. Elle yazılan değer ölçülen değerin yerine geçiyor, ikisi
ayırt edilmiyordu: elle yazılmış “2.600”, geometrisi 2,5 olan ölçüyü okuyan bir programa 2,6 diye görünebilirdi.

## Karar

### 1. Ölçünün seçenekleri, taşınan değeri ve kırmaları

- Ölçünün görünüşünde ve ölçü stilinde yeni `options` (`DimensionOptions`, hepsi isteğe bağlı): `fit` (`auto`, `outside`), `round`
  (metre), `zeros` (`leading`, `trailing`, `both`), `lengthStyle` (ADR 0262'nin biçimleri), `alt` (birim ve basamak), `tolerance`
  (`symmetric`, `deviation`, `limits`; `upper`, `lower` metre; basamak), `breakSize` (değer yüksekliğinin katı, yoksa 1). Stil değişince
  stilin eski seçeneklerini taşıyan ölçü yenisini alır (ADR 0183'ün kuralı).
- Ölçünün kendi `textShift`'i (`along`, `across`, metre: değerin kendi doğrultusunda ve ona dik) ve `breaks`'i (`line`: `dimension`,
  `first`, `second`; `t` 0–1, en çok 64). Kırmanın yeri çizginin kesrindedir: ölçü taşınsa, dönse de kırma yerinde kalır.
- **Yerleşim** (çekirdeğin `geom::dimension`'ı): hizalı ve doğrusal ölçüde `outside` ya da sığmayan `auto` değeri ikinci uzatma
  çizgisinin ötesine alır, okları dışa çevirir; değerin genişliği yaklaşıktır (harf başına 0,6 değer yüksekliği, ölçünün biriminde ve
  basamağında). Taşınan değer her türde değerin doğrultusunda kayar. Kırmalar hizalı ve doğrusal ölçünün çizgilerini kırmanın
  çevresinde `breakSize × yükseklik` kadar keser; DXF'in ölçü bloğu da aynı yerleşimden çizilir.

### 2. Değerin yazımı

`Format::dimension_value` (tek kural; çizim, DXF, başsız sunucu, Python ve sorgu aynı): uzunluk ve koordinat önce artışa yuvarlanır (yarım
sıfırdan uzağa), sonra ölçünün biriminde (yoksa projeninkinde): inç ya da ayakta ve biçim ondalık değilse ayak-inç (ölçünün `lengthStyle`'ı,
yoksa ölçü kendi birimini anmıyorsa projeninki), değilse basamağıyla ve sıfırları bastırılarak. Tolerans aynı birimde: simetrik ` ±üst`,
sapma ` +üst -alt`, sınırlar sayının yerine `en büyük/en küçük`. Sıra: ön ek, türün öneki (R, Ø, Y=, X=, t=, %), sayı, tolerans, son ek, ` [alternatif]`
(yuvarlanmamış değer). Açı ve eğim bugünkü gibidir. Ortak durumlar `fixtures/text/v1/dimension-values.json` (26).

### 3. Ölçülen değer ile elle yazılan değer

- Ölçünün `text`'i değerin yerine yazılandır: içindeki `<>` yazılan değerdir (AutoCAD'in kuralı); `<>` yoksa ölçülen değer gösterilmez,
  bu **elle yazılmış** değerdir.
- Salt okunur `cad.dimensions.query` her ölçü için türü, geometrinin ölçtüğü değeri (`measured`, `unit`), gösterileni (`shown`),
  elle yazılmış olup olmadığını (`overridden`) ve bağını (`bond`: free, partial, bound, broken) verir. Çizimin boyutlarını okuyan program
  `measured`'ı okur. Öznitelikler'de “Ölçülen değer” satırı; elle yazılmışsa “Ölçülen değer (elle yazılmış)”.
- DXF'e değer, `<>`'si yazılmış hâliyle ve seçenekleriyle ölçünün yazısı olarak gider (AutoCAD aynı yazıyı gösterir).

### 4. `cad.dimensions.edit`

Tek düzenleme, tek geri alma adımı, adı düzenlemenin: `space` (Ölçü aralığı: temele paralel hizalı ve doğrusal ölçülerin çizgileri
temelinkinden aralıkla, her yanda uzaklık sırasıyla; aralık yoksa temel değer yüksekliğinin üç katı, 0 temelin çizgisine), `break` (Ölçüyü
kır: hizalı ve doğrusal ölçünün üç çizgisinde verilen ya da görünen, ölçü ve yazı olmayan nesnelerin kenarlarının kestiği yerler; uzatma
çizgisinin ölçülen noktaya boşluğundan yakın kesişim sayılmaz; kesen yoksa `no_crossing` uyarısı), `unbreak`, `moveText` (0 ve 0 yerine),
`text` (null ölçülen değer; boş yazı `empty_text`), `options` (null kaldırır; kurala uymayan `invalid_style`). Yeni kodlar `not_straight`
(Zincir ve Baz'ın temeli de, ADR 0320), `not_parallel`, `no_crossing`. Masaüstünde şeridin CAD › Açıklama › Ölçü panelinde Ölçü aralığı
(seçimin ilk ölçüsü temel), Ölçüyü kır, Kırmayı kaldır; Öznitelikler'de Sığdırma, Yuvarlama, Sıfırlar, Alternatif birim, Tolerans ve
sapmaları, Değerin yeri (Yerine döndür), Kırmalar (Kırmayı kaldır).

### 5. Kayıt

`.kcad` şema 61, `FORMATS_VERSION` 70: ölçünün ve ölçü stilinin `options`'ı, ölçünün `textShift`'i ve `breaks`'i sözleşmenin JSON'u gibi
(varsayılanlar yazılmadan); sütunlarda ölçünün 29. biti ve bir JSON metni. Bağımsız Python yazıcısı ve okuyucusu, örnek `dimension-options.kcad`
ve yedi bozuk dosya.

## Sonuçlar

- Paralel ölçüler bir komutla dizilir; kesen nesnelerde çizgiler kırılır; değer taşınır ya da sığmayınca dışarı alınır.
- Değer yuvarlama, sıfırlar, ayak-inç, alternatif birim ve toleransla yazılır; aynı kural her çıktıda.
- Elle yazılan değer ölçülen değerden ayrılır; Python ve ajan ikisini ayrı okur.
- Kırmalar nesneler değişince kendiliğinden yenilenmez: Ölçüyü kır yeniden çalıştırılır (AutoCAD'in kendiliğinden kırması ileri iş).
- Sığdırmada değerin genişliği yaklaşıktır; yazı tipinin ölçülen ilerlemeleri ileri iştir.

## Doğrulama

- Bağımsız başvurular (KentOS kodu olmadan): `scripts/fixtures/dimension_value_cases.py` (26 durum), `scripts/fixtures/dimension_edit_cases.py`
  (`cad.dimensions.edit` 32, `cad.dimensions.query` 20 durum); masaüstünde ve Python'da koşar; yetenekler her kodu bir durumda görür.
- KCAD: `kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Resimler: `KENTOS_SHOTS_ONLY=olcu-secenekler,olcu-aralik,olcu-kirma cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
