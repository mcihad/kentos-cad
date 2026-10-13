# ADR 0256: Birim değiştirme, şemada birimler, ölçünün dayanağı ve eksen düzeni

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §6 `CAD-DATA-04`, `05`, `06`, `08`. Kapsamı ben belirledim
  (sahibin sözü: “CAD data bölümüne geç ve orayı tamamla”).
- **Bağlam belgesi:** ADR 0165 §2 ve §4 (çizim birimi; eksen ve açı düzeni), 0245 (Python ve MCP'de birim), 0248 §5
  (`cad.settings.set`), 0253 (komut protokolü), 0171 (düzlem, elipsoit ve zemin), 0049 (proje ayarları).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

- Birimi değiştirmek bugün yalnız gösterimi değiştirir (`cad.settings.set`'in `drawingUnit`'i, Proje ayarları): 4000 mm'lik duvar 4 m
  okunur, boyu aynı kalır. Yanlış birimle gelmiş bir çizimi düzeltmenin (sayılar kalsın, boy değişsin) yolu yoktu. Ölçekle ×0,001 ile
  koordinatlar küçülür, ama kotlar, blokların içi, ana görünüm ve çapa olduğu gibi kalır; birim ayrı bir ayardır, geri alma ikisini
  birlikte geri vermez.
- Komut şemaları sayıların birimini çoğunlukla söylemiyordu. `Arc.a0` radyan, `Text.rotation` derece, `lineWeight` kâğıtta mm,
  `zs` metre (kot), `arrowSize` yüksekliğin katı, `x,y` düzlemin birimi; ajan bunları tahmin ediyordu.
- Ölçü sonucu (`drawing.measure`) yalnız `units` taşıyordu. Hangi sistemin düzleminde, hangi yöntemle hesaplandığını yazmıyordu;
  birden çok nesne tek tek ölçülüyordu.
- Eksen ve açı düzeni (ADR 0165 §4) arayüzündü. Python ve ajan şunları bilmiyordu:
  - telde x'in doğu, y'nin kuzey olduğunu;
  - CBS'nin arayüzünün bunlara Y ve X dediğini;
  - kutupsal açının nereden ve hangi yöne ölçüldüğünü.
- Yolda bulunan hata: DXF'in birim çevirisi (`formats::units`) bütün uzunlukları ölçeklemiyordu. Çok noktalı nesnenin parçaları,
  paragrafın kutu genişliği, eğri boyunca yazının eğrisi ve tablonun çerçeve kalınlığı dosyanın biriminde kalıyordu. Milimetrelik
  DXF'in MTEXT'i bin kat geniş kutuyla geliyordu.

## Karar

### 1. `cad.units.change` v1

Girdi: `from` (çizimin şimdiki birimi), `to`, `geometry` (`keep` | `scale`), `expectedRevision?`. Tek geri alma adımı: “Birimi
değiştir”.

- **`keep`: boylar korunur.** Yalnız okuma ve yazma birimi değişir: 4000 mm'lik duvar 4 m okunur. Hiçbir nesne değişmez.
- **`scale`: sayılar korunur.** Çizim iki birimin oranıyla 0,0'a göre ölçeklenir: 4000 mm 4000 m okunur (×1000). Çarpan tek tam
  sayıdır (birimler onun kuvvetleri kadar ayrı): mm'den m'ye ×1000, m'den mm'ye ÷1000, cm'den mm'ye ÷10. Tek float64 işlemiyle
  uygulanır; yazılan değerin metreye çevrilmesi ve DXF'in `$INSUNITS`'i de aynı yoldan geçer (`kentos_contracts::units::Scale`).
  - **Ölçeklenenler:**
    - her nesnenin koordinatları, uzunlukları, yarıçapları ve kotları (z);
    - yazı, ölçü, kılavuz ve tablo yükseklikleri; ölçünün uzaklığı;
    - tablonun satır ve sütun boyları ile çerçeve kalınlığı;
    - paragrafın kutu genişliği; eğri boyunca yazının eğrisi;
    - taramanın çizgi aralığı ve deseninin birim başına metresi;
    - resmin ve rasterin çerçevesi;
    - blok tanımları (taban noktası, nesneleri, öznitelik tanımları);
    - ana görünüm ve çizimin çapası.
  - **Kalanlar:**
    - açılar ve oranlar (büküm; ölçünün ve kılavuzun boyları, yüksekliklerinin katıdır);
    - yerleştirmenin kendi ölçeği (tanımı ölçeklenir);
    - kâğıt üstü boylar (kalınlıklar, kâğıttaki yazı yükseklikleri);
    - rotanın ölçüleri (M, rotanın kendi birimi).
  - **Ölçeklenmeyip uyarıyla söylenenler:**
    - nokta bulutlarının noktaları, çünkü dosyalarındadır (`point_clouds_kept`);
    - pafta kitabı, çünkü kâğıdındır (`sheets_kept`);
    - projenin toleransları (topoloji, ağlar), çünkü projenin kesinliğidir (`tolerances_kept`).
- **Birim ve geometri tek adımdır.** Proje ayarları geri alma adımı değildir (ADR 0049). Ama sayıları koruyan birim değişikliğinde
  birim, ölçeklenen çizimin anlamıdır: geri alma ikisini birlikte vermelidir. Bu yüzden komut birimi, ana görünümü ve çapayı adımın
  kendi işlemiyle yazar (`Op::Units`, `Document::set_unit_state`). `keep` da aynı işlemle yazılır, böylece komutun iki kipi de geri
  alınır. `cad.settings.set`'in `drawingUnit`'i olduğu gibi kalır: yalnız gösterim, adım değil.
- **Retler, bu sırayla:**
  1. `same_unit` (`from` `to`'ya eşit);
  2. `invalid_revision`, `revision_conflict`;
  3. `unit_needs_local` (koordinat sistemli projenin birimi sistemindir);
  4. `unit_mismatch` (`from` çizimin birimi değil; ileti çizimin birimini söyler);
  5. `layer_locked` (`scale`'de kilitli katmanda nesne var; ileti katmanları ve nesne sayılarını, `uids` ilk 100 nesneyi
     söyler). Birim bütün çizimin anlamıdır: hep ya da hiç.
- **Çıktı ve plan:** `{from, to, geometry, factor, objects, blocks, revision}`. `factor` `keep`'te 1'dir. `objects`, nokta bulutları
  dışındaki nesnelerin sayısıdır; `blocks`, tanımların sayısıdır.

### 2. Proje ayarları

- Kaydet'te çizim birimi değişmişse ve çizimde nesne ya da blok varsa sorar: “Çizim birimi mm'den m'ye değişiyor. Çizimdeki
  nesneler ne olsun?”. Üç seçenek:
  - “Boyları korunsun” (önerilen; 4000 mm'lik duvar 4 m okunur);
  - “Sayıları korunsun” (çizim ×1000: 4000 mm 4000 m okunur);
  - Vazgeç.
- Seçim `cad.units.change` ile yazılır; öbür ayarlar bugünkü gibi. Boş çizimde sorulmaz, boylar korunur.

### 3. Şemada birimler: `x-unit`

Kataloğun girdi, çıktı ve plan şemalarındaki her sayı birimini `x-unit` anahtarıyla söyler. Birim ya sayının kendisinde ya da onu
taşıyan alandadır: `Vec2` ve `Bounds` kendi birimini taşımaz, onları kullanan alan söyler (`p` için `length`, `dir` için `1`).

| `x-unit` | Anlamı |
|---|---|
| `length` | projenin düzleminde uzunluk ya da koordinat: metre, coğrafi sistemde derece (`units.length`) |
| `area` | projenin düzleminde alan: m², coğrafi sistemde derece² (`units.area`) |
| `elevation` | düşey metre: kot, z, yükseklik |
| `m` | her sistemde metre (izdüşümün parametreleri, ölçme doğrulukları) |
| `measure` | rotanın ölçüsü (M), rotanın kendi biriminde |
| `paper-mm` | kâğıtta milimetre: kalınlık, kâğıttaki yazı yüksekliği, pafta boyları |
| `px` | ekranda mantıksal piksel |
| `deg`, `rad` | derece, radyan; yön ise doğudan saat yönünün tersine (açıklaması başkasını söylemiyorsa) |
| `azimuth-deg` | kuzeyden saat yönünde derece |
| `1` | birimsiz sayı: çarpan, oran, büküm, saydamlık (0–1), ağırlık, öncelik |
| `percent` | yüzde (0–100) |
| `ppm` | milyonda bir |
| `pattern` | tarama deseninin tanımının birimi (taramanın ölçeğiyle çarpılır) |
| `count`, `index`, `id` | adet; 0'dan başlayan sıra; çizimdeki yuva numarası |
| `epsg` | koordinat sisteminin EPSG kodu (0: yok) |
| `scale` | ölçeğin paydası (1:N'nin N'si) |
| `zoom`, `bytes`, `version` | karo düzeyi; bayt; sürüm numarası |
| `value` | verinin kendi birimi (rasterin değeri, kuralın eşiği) |
| `km/h` | hız |
| `system` | adı verilen sistemin kendi birimi (karo ağının koordinatları) |

Katalog bu sözlüğü `units` alanında anlamlarıyla taşır. Sözleşmelerin testi her şemadaki her sayının sözlükte bir birimi olduğunu
denetler: birimsiz sayı katalogu düşürür. Python SDK'sının üretilen alan belgeleri birimi yazar. MCP'nin araç şemaları kataloğun
şemalarıdır; birim onlarla gider.

### 4. Ölçünün dayanağı

`drawing.measure` ve Python'un `Document.measure`'ı:

- **`basis`:**
  - `srid`, `system` (adı), `kind` (`local`, `projected`, `geographic`);
  - `frame` (`world`: koordinatlar ve kutu çizimin dünyasında; UCS geldiğinde de tel dünyadır);
  - `method` (`plane`, `ellipsoid`, `ground`) ve `ground`'da `height`.
- **`method` seçilebilir:**
  - `plane` (varsayılan): projenin düzleminde; yaylar, daireler ve elipsler kaynaklarından kesin.
  - `ellipsoid`: sistemin elipsoidi üstünde (Karney jeodezikleri, ADR 0171). Ancak sistemi olan projede; birimler m ve m².
  - `ground`: zeminde, projenin ortalama elipsoit yüksekliğinde. Ancak yüksekliği yazılmış projede.
  - Elipsoit ve zemin yalnız halkalara inen nesnelerde verilir: çizgi, çoklu çizgi, alan, daire, yay, tarama. Elips ve eğri 0,1 mm
    içindeki kirişleriyle (ADR 0149 §5.3) verilir. Öbürlerinde değer yoktur.
  - Sistemi olmayan projede ellipsoid ve ground reddedilir: `method_unavailable`.
- **Toplu ölçü:** Python'da `Document.measure_many`, MCP'de `drawing.measure`'ın `uids`'i (en çok 1000). Nesnelerin ölçüleri,
  toplamları (`count`, `area`, `length`) ve tek `basis` ile `units` döner.

### 5. Eksen ve açı düzeni

Çizimin özeti (`drawing.summary`, `Document.info()`, ajan ve konsol bağlantısı) `conventions` taşır:

```json
{ "workspace": "cad", "axes": { "x": "east", "y": "north" }, "labels": { "x": "X", "y": "Y" },
  "polar": { "zero": "east", "turn": "ccw" }, "angleUnit": "deg" }
```

- Telde x her zaman doğu (sağa), y kuzeydir (yukarı); sayılar iki türde de aynı sırayla yazılır.
- `labels`, arayüzün bu iki eksene verdiği addır: CAD'de X ve Y, CBS'de Y ve X.
- `polar`, kutupsal girişin ve okumanın düzenidir: CAD'de doğudan saat yönünün tersine, CBS'de kuzeyden saat yönünde (semt).
- `angleUnit`, projenin gösterdiği ve yazdığı açı birimidir. Telde her açı alanı kendi birimindedir (`x-unit`).
- Python'da `Document.conventions`; SDK'nın ve MCP'nin açıklamaları bunu söyler.

## Uygulama

- **Birim kuralı:** `crates/shared/contracts/src/units.rs` (`Scale`, `keeping_numbers`, `scale_entity`, `scale_block`,
  `scale_bounds`, `UNITS`, `is_unit`); `kentos_formats::units` onu yeniden dışa aktarır. Kural DXF'in birim çevirisiyle aynı
  yoldur. Eksikleri giderildi: noktanın parçaları, paragrafın kutusu, eğri boyunca yazının eğrisi, tablonun çerçevesi.
- **Sözleşme:** `cad_units.rs` (`UnitsChange`, `UnitGeometry`, `UnitsChanged`). Yeni kodlar `codes.rs`'te: `same_unit`,
  `unit_mismatch`, `point_clouds_kept`, `sheets_kept`, `tolerances_kept`; sunucu kodları `method_unavailable`, `out_of_system`.
  Katalog kaydı ve örnekleri, yetenekleri, kataloğun `units`'i (`CatalogUnit`); şemalardaki her sayının `x-unit`'i (alanlara
  `schemars(extend(..))`, `VariableValue`'da türe).
- **Belge:** `crates/native/domain/src/units.rs` (`UnitState`, `unit_state`, `set_unit_state`); geçmişin `Op::Units`'i; adımın
  raporunda `ProjectPart::Settings`.
- **İşleyici:** `crates/native/application/src/units_change.rs` (`validate`, `plan`, `plan_in`, `execute`); `DESKTOP_COMMANDS`,
  `registry.rs`.
- **Başsız sunucu:** `query.rs`:
  - `Method`, `Basis`, `basis`, `units_by`, `measure_by`, `measure_many`, `Measures`, `MeasureItem`, `Totals`;
  - elipsoit ve zemin `crs::ground::ground_measures`'tan (ADR 0171);
  - `Conventions`, `conventions`.
  `rpc.rs`'in özetinde `conventions`, `homeView`, `origin`; `measure`'ın `method`'u ve `measureMany`.
- **Python:**
  - `Document.measure(uid, method)`, `Document.measure_many`, `Basis`, `Measures`, `MeasureItem`, `Conventions`,
    `Document.conventions`, `DocumentInfo.conventions` ve `home_view`;
  - yerel modülün `measure`'ı ve `measure_many`'si; konsolun `measureMany`'si;
  - `kentos.cad.units.change` üretildi;
  - üretilen alan belgelerinde `Unit: …`, `kentos.cad.types.UNITS`.
- **MCP:**
  - `drawing.measure`'ın `uids`'i ve `method`'u;
  - `drawing.summary`'nin ve sunucu talimatının sözleri, komut araçlarının açıklamasında `x-unit`;
  - `cad.units.change` yıkıcı sayılır.
- **Masaüstü:** `apps/desktop/src/project/unit_change.rs` (soru ve görünüşü); `settings.rs`'in `change_unit`'i ve
  `UnitAnswer`'ı.
- **İşlem manifesti:** Proje ayarları `cad.settings.set` ve `cad.units.change` üzerinden.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/units_change_cases.py` (KentOS kodu olmadan, bu ADR'nin kuralından):
  - `fixtures/commands/v1/cad.units.change.json`'un 23 durumu;
  - her nesne türü uzunluk ve uzunluk olmayan alanlarıyla;
  - beş yönün çarpanları, metre sanılan milimetrelik oda, boş çizim, plan ve doğrulama, kilitli katman, retler, sürüm
    çakışması.
  Masaüstü ve Python'un tipli katmanı bit bit geçer. İki ölçekleme kuralı bozulunca 7 durum düşer.
- **Masaüstünün kendi testleri** (`tests/all/units.rs`):
  - pafta kitabı kalır, `sheets_kept` söylenir;
  - birim ve ölçeklenen çizim tek adım ve tek kayıt satırı;
  - boyları korumak da geri alınır.
- **Ölçü:**
  - `headless` testi `a_measure_says_its_basis_and_a_batch_sums`;
  - MCP testi `an_agent_changes_the_unit_and_measures_in_a_batch`;
  - Python `test_units.py`'nin `UnitChange` ve `MeasureBasis` sınıfları.
  TM33'te 100 × 100 m karenin elipsoit alanı 10 000 m²'ye 1 m² içinde (merkez meridyende k₀ = 1).
- **Şemada birimler:** `catalog::tests::every_number_names_its_unit` her komutun girdi, çıktı ve plan şemasındaki her sayının
  birimini ve birimin sözlükte olduğunu denetler (ilk koşuda 509 birimsiz sayı vardı).
- **DXF:** `fixtures/formats/v1/dxf-write/units`'e 45 mm kutulu çok satırlı yazı, kalın çerçeveli tablo ve yaylı eğri boyunca
  yazı eklendi. `dxf_write_reference.py` (bağımsız; ölçekleme kuralına kutu, çerçeve, satır ve sütun boyları ile eğri eklendi)
  milimetreyle yazıldıklarını denetler.
- **Masaüstü:** `project::unit_change::tests`:
  - Kaydet sorar, sorarken hiçbir şey yazılmaz;
  - Boyları korunsun yalnız birimi değiştirir, Sayıları korunsun 4 m'yi 4000 m yapar, ikisi de geri alınır;
  - Vazgeç pencereye döner, boş çizimde sorulmaz.
- **Resimler:** `cargo test -p kentos-desktop project::unit_change::tests::screens -- --ignored --nocapture`,
  `.run/shots/birim-sorusu-*` (iki tema, 1440 × 900 ve 1100 × 650).

## Sonuçlar

- Yanlış birimle gelmiş çizim tek komutla düzelir; birim ve çizim birlikte geri alınır.
- Python ve ajan her sayının birimini şemadan, ölçünün hangi sistemde ve nasıl alındığını sonucundan, eksen ve açı düzenini
  özetten okur.
- DXF'in birim çevirisi bütün uzunlukları taşır: milimetrelik dosyanın çok satırlı yazısının kutusu, tablosunun çerçevesi ve eğri
  boyunca yazısı doğru boyda gelir ve gider.
- **Açık kalanlar:**
  - DXF içe almada birimin açık seçimi ve blok birimi (`CAD-DATA-07`, ayrı dilim);
  - UCS (`CAD-DATA-09`–`11`): ölçünün `frame`'i o gelene dek hep `world`;
  - ayak ve inç (`CAD-DATA-12`).
