# ADR 0255: Anlamsal ölçü oluşturma (`cad.dimensions.create`)

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §5 `CAD-CMD-05`, `06`. Kalıp
  [ADR 0254](0254-semantic-stretch-break-join-divide-boundary-hatch.md)'ünkidir; çağrıların ortak protokolü
  [ADR 0253](0253-command-protocol.md). Kapsamı ben belirledim (sahibin sözü: “CAD-CMD adımlarını tamamla”, “hepsini de
  bitirene kadar durma”).
- **Bağlam belgesi:** ADR 0061 (Ölçülendirme), 0147 (yeni ölçü türleri), 0183 (ölçü stilleri), 0205 (açıklamaların
  yükseklikleri), 0249 (alt nesne yerleri ve ilişkili ölçü), 0250 §5 (yerleriyle ilişkili ölçü), 0165 §2 (tel üzerinde metre).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Ölçülendirme ölçüyü kendisi kurup hazır geometriyle `cad.entities.create`'e yazıyordu. ADR 0250 §5'ten beri Hizalı,
Doğrusal, Koordinat, Semt ve Eğim'in noktaları nesnenin yeriyle verilebiliyordu; ama Yarıçap, Çap, Açı, Yay uzunluğu ve
Kırıklı yarıçap için Python ve ajan dairenin üstündeki noktayı, açının kollarını, yayın uçlarını kendileri hesaplamak
zorundaydı. Ölçünün yüksekliği, stilin görünüşü ve kayıtlı offset de çağıranındı. `CAD-CMD-05` ölçü oluşturmayı da semantik
girdiyle ister.

## Karar

### 1. Komut

`cad.dimensions.create` v1, adım “Ekle” (aracın adımı değişmedi). Girdi:

| Alan | Anlamı |
|---|---|
| `measure` | Ne ölçülür (§2) |
| `at` | Ölçünün yeri: çizgisi, yayı ya da kılavuzu oradan geçer |
| `offset?` | `at`'nin uzaklığı yerine yazılan uzaklık (§3) |
| `layerId` | Katman; `cad.entities.create`'in kurallarıyla (yok, grup, kilitli; gizliye uyarı) |
| `dimensionStyle?` | Projenin ölçü stilinin adı (büyük küçük harf ayırmaz); yoksa ya da “Standart”: Standart |
| `height?` | Değerin yüksekliği, metre; yoksa stilinki projenin ölçeğinde, Standart'ta projenin Ölçü yüksekliği (ADR 0205) |
| `mask?` | Zemin (ADR 0147) |
| `color?`, `lineWeight?`, `attrs?`, `label?`, `symbol?` | Görünüş ve taşıdıkları (`NewLook`) |
| `expectedRevision?` | Beklenen sürüm |

Çıktı ve plan `cad.entities.create`'inkidir (`EntitiesCreated`, `EntitiesCreatePlan`).

### 2. Ölçülenler

Ölçülen nokta (`MeasuredPoint`) koordinattır (`{kind: "at", at}`) ya da nesnenin yeridir (`{kind: "place", uid, place}`;
yerler ADR 0249'unkiler: `node`, `start`, `end`, `center`, `vertex`, `mid`). Yer, nesnenin şimdiki noktasıdır.

Kenar (`EdgeNear {uid, near}`) tıklamanın seçtiği gibidir: nesnenin ölçünün aradığı türdeki kenarlarından `near`'a en yakını
(düz kenar, uçları olan yay ya da daire).

| `measure.kind` | Araçtaki adı | Girdi |
|---|---|---|
| `aligned` | Hizalı | `a`, `b` |
| `linear` | Doğrusal | `a`, `b`, `angle?` (0 ΔY, 90 ΔX; yoksa `at`'nin konumuna göre, AutoCAD'in kuralı) |
| `ordinate` | Koordinat | `a`, `angle?` (0 Y, 90 X; yoksa `at`'nin konumuna göre); kılavuzu `at`'de biter |
| `azimuth` | Semt | `a`, `b` |
| `slope` | Eğim | `a`, `b`, `za?`, `zb?` (yoksa yerin köşesinin kotu, ADR 0142) |
| `angle` | Açı (kenarlardan) | `first`, `second`: iki düz kenar |
| `angleVertex` | Açı (köşeden; daireden) | `c`, `a`, `b` |
| `angleArc` | Açı (yaydan) | `of`: yay |
| `radius`, `diameter` | Yarıçap, Çap | `of`: daire, yay ya da yolun yay parçası |
| `arcLength` | Yay uzunluğu | `of`: uçları olan yay; `between?`: Kısmi'nin iki noktası |
| `jogged` | Kırıklı yarıçap | `of`, `shownCentre` (gösterilen merkez), `on` (yaydaki nokta) |

Ölçü ortak çekirdeğindir (aracın kullandığı aynı işlevler): `signed_offset`, `linear_angle_for`, `dimension_offset_at`,
`ordinate_axis_for`, `vertex_arms`, `edge_arms`, `radial_dimension`, `arc_length_ends`. Ölçü yerleşmiyorsa
(`layout_dimension` yok diyorsa) ret: `no_dimension`, ileti aracınkiyle aynı.

### 3. Yazılan uzaklık

`offset` aracın yazılan sayısıdır: Hizalı'nın, Semt'in ve Eğim'in işaretli uzaklığı (a→b'nin solu artı), Doğrusal'ın doğrultunun
normali boyunca, Açı'nın yay yarıçapı (işareti alınmaz), Yay uzunluğu'nun yaydan dışarı, Kırıklı yarıçap'ın gösterilen merkezden
yarıçap boyunca kırık. Yarıçap, Çap ve Koordinat uzaklık almaz: verilirse ret `invalid_offset`.

### 4. Bağlar

Hizalı, Doğrusal, Semt ve Eğim'in `a` ve `b`'si, Koordinat'ın `a`'sı yer olarak verilmişse bağlanır (`refs`, ADR 0249): nesnesi
değişince ölçü izler. Öbür türlerin yerleri yalnız okunur (ADR 0249'un kuralı: dairenin üstündeki nokta bir yer değildir).

### 5. Retler, bu sırayla

`not_finite` (`at`, `offset`, ölçülenlerin sayıları sırasıyla), `invalid_height`, `invalid_uid`, `invalid_offset`,
`invalid_revision`, `revision_conflict`, `entity_not_found`, `no_such_place`, `not_measurable` (nesnede aranan türde kenar
yok), `missing_elevation` (Eğim'in kotu yok), `unknown_style`, `no_dimension`; sonra `cad.entities.create`'in katman ve görünüş
retleri. Yeni kodlar: `not_measurable`, `missing_elevation`, `invalid_offset`, `no_dimension`.

### 6. Araç

Ölçülendirme'nin seçimleri komutun girdisidir: noktalar (kenetle oturduğu yer varsa yer), tıklanan kenarlar (nesne ve tıklanan
yer), yer ve yazılan sayı; etkin katman, CAD projesinde seçilen stil, Zemin, şeridin rengi ve kalınlığı, nesne şablonunun
taşıdıkları. Önizleme komutun ölçüsüdür (`measured_in`), yazma komutun kendisidir. Görünen davranış, istemler ve iletiler
değişmedi. Kenarların bulunuşu (`straight_edge_at`, `arc_edge_at`, `circle_at`) uygulama katmanına taşındı.

## Uygulama

- **Sözleşme:** `cad_dimensions.rs` (`DimensionsCreate`, `DimensionMeasure`, `MeasuredPoint`, `EdgeNear`); katalog kaydı,
  örnekleri ve yetenekleri (`radius` ve `arcLength` hedefleri); yeni kodlar `codes.rs`'te.
- **İşleyici:** `crates/native/application/src/dimension_create.rs` (`measured_in`, `dimensions_create`); `registry.rs`,
  `DESKTOP_COMMANDS`; `creating.rs`'in `input_as`'ı (adım “Ekle”).
- **Araç:** `crates/native/interaction/src/dimension.rs` (`measure`, `input_for`, `request`, `refresh`, `commit`).
- **Başsız sunucu, MCP, Python:** kayıt yoluyla; `python/kentos/cad/dimensions.py` üretildi.
- **İşlem manifesti:** `dimension` ve on yöntemi `semantic`, `cad.dimensions.create` üzerinden.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/dimension_create_cases.py` (KentOS kodu olmadan, ADR 0061, 0147, 0183, 0205, 0249'un
  tanımından; mesafeler 3-4-5 üçgenleriyle tam, aşkın değerler 60 basamakta): `fixtures/commands/v1/cad.dimensions.create.json`'un
  49 durumu; masaüstü ve Python bit bit geçer.
- **Yetenekler:** her kod bir durumda görülür; Yarıçap ve Yay uzunluğu her nesne türüyle denenir.
- **İzler:** Ölçülendirme'nin bütün izleri (`dimensions`, `dimension-kinds`, `dimension-more-kinds`, `dimension-styles`,
  `dimension-refs`, kabul izi) değişmeden geçer.
- **Resimler:** `usage-semantic-tools`'un yarıçap adımı (ADR 0254 Doğrulama).

## Sonuçlar

- Python ve ajan her ölçü türünü nesnenin kimliği, yeri ve ölçünün yeriyle ister; noktaları hesaplamaz.
- Açık kalanlar: Zincir ölçü, Baz ölçü ve Hızlı ölçü'nün kendi komutları (bugün `cad.entities.create`'in işlemleriyle yazarlar);
  web'in karşılığı (TODOS.md §0.2).
