# ADR 0329: Taramanın bölgeleri, boşluk toleransı ve taramayı düzenleme

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §12 `CAD-HATCH-03`, `04`, `05` (sahibin sözü: “Hatch
  bitene kadar durma”; 2–3 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0062 ve 0186 (tarama, ilişkili tarama), 0254 (`cad.entities.hatch`), 0328 (kaynak sınır, yüzün bağı),
  0253 (komut protokolü), 0265 (yeni şemanın altı adımı), 0244 (işlem manifesti).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

`cad.entities.hatch` bölgeyi tek bir noktadan buluyordu; adalar ya açık kalıyor ya taranıyordu (`islands`), adanın içindeki
kapalı nesne bir daha taranmıyordu (AutoCAD'in “Normal” ada algılaması yoktu). Çizgilerin kapattığı yüz, duvarlardan biri birkaç
santim kısa kalınca bulunmuyordu: kullanıcı ucu elle uzatmak zorundaydı, araç da yalnız “bölge yok” diyordu. Tarama yazıldıktan
sonra sınırını nesne olarak geri almanın, bölgeyi yeniden seçmenin, ada eklemenin ve çıkarmanın, ilişkiyi koparmanın anlamsal
yolu yoktu; Öznitelikler'deki “İlişkiyi kopar” dışında bunlar Python ve ajan için hiç yoktu.

## Karar

### 1. Ada algılaması (`CAD-HATCH-03`)

`HatchRegion`'ın `point` ve `objects` biçimlerine `islandStyle` gelir: `outer` (Dış: adalar açık, içlerindeki her şeyle),
`normal` (İç içe: adalar açık, içlerindeki kapalı nesneler yeniden taranır, onların içindekiler yine açık; sırayla, en çok 16 kat)
ve `ignore` (Yok say: adaların üstü taranır). Verilince `islands`'ın yerine geçer; verilmezse `islands` eskisi gibidir (açık
`outer`, kapalı `ignore`; `HatchRegion::islands_taken`).

İç içe'de her kat ayrı taramadır, aynı adımda: adanın içindeki kapalı nesnelerden **doğrudan** olanlar (bütünüyle adanın
içinde, başka bir iç nesnenin içinde değil; yuvalarının sırasıyla) taranır, kendi adalarıyla. Kapalı nesne yolunda ve Çoklu
tara'da böyledir; çizgilerin yüzünde iç içe yoktur: bölge Dış gibi taranır ve `islands_not_nested` uyarısı söylenir. İlişkili
iç taramalar kendi nesnelerine bağlanır.

### 2. Çoklu bölge

`point` biçimine `alsoAt` gelir: başka noktalar, her biri `at` gibi kendi bölgesini bulur (en çok 1000). İki noktanın bulduğu
aynı bölge bir kez taranır; bir noktanın bölgesi yoksa bütün komut `not_enclosed` ile reddedilir. Sonlu olmayan nokta
`not_finite`, 1000'i aşan `invalid_region`.

### 3. Boşluk toleransı (`CAD-HATCH-04`)

Çizgiler yolunda `gap` (m, 0 ile 1000 arası; başka değer `invalid_gap`): çizgi işinin boşta kalan her ucu (başka hiçbir
çizgiye değmeyen uç) `gap` içindeki en yakın başka çizgi işine bir köprüyle bağlanır (`ops::hatch_region::bridges`: uçtan en
yakın noktaya; aynı nesnenin kendi gövdesi sayılmaz, iki uç birbirine bağlanınca köprü bir kez). Yüzler köprülerle birlikte
bulunur; köprü yalnız bölgeyi bulmak içindir, nesne yazılmaz.

Kapanan boşluk planda ve sonuçta `gaps_closed` uyarısıdır (yol `region.gap`): “n boşluk kapatıldı (toplam L, en uzunu M);
taranan alan A.” — uzunluk ve alan projenin biriminde (`Format`). Köprüsü bölgenin halkasında olmayan boşluk sayılmaz.
İlişkili yüz taraması `gap`'i bağında tutar (`HatchAssoc.gap`; `.kcad` şema 70, `FORMATS_VERSION` 79): duvar taşınınca bölge
aynı toleransla yeniden kesilir. Dış halkası olan bağda boşluk yoktur (şemanın kuralı).

Araçta Tarama'nın Boşluk (G) seçeneği (yalnız çizgiler yolunda anlamlı; `Memory::hatch_gap`) ve Adalar (A) seçeneğinin üç
değeri: taranmaz, taranır, iç içe (`Memory::hatch_nested`).

### 4. Taramayı düzenle: `cad.hatches.edit` v1 (`CAD-HATCH-05`)

Her işlem tek geri alma adımı, adı işlemin adıdır:

- **Sınırı yeniden oluştur** (`recreate`: `form` `area` ya da `polyline`, `layerId`, `associative`): taramanın kaynak sınırı
  (ADR 0328; yoksa çizilen halkası) yeni nesneler olarak yazılır. Alan: delikleriyle tek alan, yaylar kesin, elips ve eğri
  0,1 mm'lik kirişleriyle. Çoklu çizgi: her halka bir nesne; düz ve yaylı halka kapalı çoklu çizgi, bütün elips elips, bütün
  kapalı eğri NURBS, karışık halka kirişleriyle. `associative` (varsayılan açık) taramayı yeni nesnelere bağlar: alanda dış
  nesne, çoklu çizgide ilk halka sınır ötekiler ada.
- **Sınırı yeniden seç** (`reselect`: `region`, bir nokta): bölge Tarama'nın kuralıyla yeniden bulunur; desen, katman ve
  görünüş kalır, bağ ve sınır yeni bölgenin olur.
- **Ada ekle** (`addIslands`: `uids`, kapalı nesneler): bağlı tarama onları adalarına ekler ve nesnelerinden yeniden kesilir
  (zaten adası olan bir şey değiştirmez: yazma yok); bağsız taramanın bölgesinden alanları çıkarılır (en büyük parça kalır).
- **Ada çıkar** (`removeIslands`): bağlı taramada nesneleriyle (`uids`, adalarından olmalı: `island_not_found`), her taramada
  deliklerinin sırasıyla (`holes`); delik taranır.
- **İlişkiyi kopar** (`untie`: `uids`): taramalar bir şey izlemez, yerinde kalır.

Belge kuralı (`hatch_ties`): adımın bağını değiştirdiği tarama artık bağdan düşmez; yeni bağından yeniden kesilir
(`Document::recut_hatch`). Böylece komutun yazdığı bağ ve halka belgenin kendi kuralıyla aynıdır.

Retler: `invalid_uid`, `entity_not_found`, `not_a_hatch`, `layer_locked`, `layer_not_found`, `no_entities`, `not_closed`,
`not_enclosed`, `island_not_found`, `invalid_region`, `not_finite`, `invalid_gap`, `invalid_revision`, `revision_conflict`.
Uyarılar: `layer_hidden`, `gaps_closed`, `islands_not_nested`, `curves_approximated` (sınır kirişlerle yazıldı).

### 5. Arayüz

- **Taramayı düzenle** aracı (`tool.hatchEdit`; CAD'de Açıklama › Tarama, CBS'de Düzenle › Nesne): taramaya tıkla, kip
  (Ada ekle E, Ada çıkar Ç, Yeniden seç S, Yeniden oluştur O, İlişkiyi kopar K; Yeniden oluştur'da Biçim B) ve tıklama. Tıklanan
  yerin adayları arasından tarama seçilir (kenarı çoğu zaman bir alanın kenarıyla ortaktır). Seçilen taramanın halkası ve
  delikleri ayrı ayrı vurgulanır. Kip oturumun belleğindedir (`Memory::hatch_edit_mode`).
- **Öznitelikler**'de taramanın Sınır satırı: kaynak sınırı olup olmadığı ve Yeniden oluştur ▾ (alan, çoklu çizgi).
- İkonlar: `hatchEdit`, `hatchIslandAdd`, `hatchIslandRemove`, `hatchReselect`, `hatchRecreate`, `hatchUntie`
  (`catalog/v1/additions.json`).
- Python `kentos.cad.hatches.edit`, MCP aracı (yıkıcı listede), manifestte `tool.hatchEdit` `semantic`.

## Sonuçlar

- `fixtures/commands/v1/cad.entities.hatch.json`'a iç içe, çoklu bölge ve boşluk durumları (bağımsız başvuru
  `region_command_cases.py`, 46 durum); `cad.hatches.edit.json` 40 durum (`hatch_edit_cases.py`), KentOS kodu olmadan,
  kesirlerle.
- KCAD şema 70: `fixtures/kcad/v2/hatch-gaps.*` ve dört bozuk dosya (şema 69'da boşluk, sıfır boşluk, dış halkalı bağda boşluk,
  şema 71); bağımsız Python okuyucusu ve yazıcısı.
- Köprü en yakın noktaya düz çizgidir; AutoCAD gibi boşluğu yayla ya da uçları uzatarak kapatmaz. Köprü çizgi işinin kendisi
  değildir: duvarı uzatmak isteyen Uzat'ı kullanır.

## Doğrulama

```bash
python3 scripts/fixtures/region_command_cases.py --check; python3 scripts/fixtures/hatch_edit_cases.py --check
python3 scripts/fixtures/kcad_v2_reference.py --check
cargo test -p kentos-native-application --test all fixtures
KENTOS_SHOTS_ONLY=tarama-ic-ice,tarama-bosluk,tarama-duzenle cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
```
