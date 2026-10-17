# ADR 0331: Projenin tarama desenleri, desenin başlangıcı ve ölçekli tarama

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §12 `CAD-HATCH-06`, `07`, `08` (sahibin sözü: “Hatch
  bitene kadar durma”; 2–3 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0186 (tarama desenleri ve kitaplık), 0254 (`cad.entities.hatch`), 0313 (çizgi tipleri ve LIN: aynı
  biçimin PAT'ı), 0323 (ölçekli açıklamalar), 0329 (`cad.hatches.edit`), 0265 (yeni şemanın altı adımı).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Tarama yalnız kitaplığın 28 desenini kullanabiliyordu; bir ofisin `.pat` dosyası (tuğla örgüsü, çim, karo) projeye alınamıyor,
taramalar dışarı PAT olarak verilemiyordu. Desenin çizgileri her zaman dünyanın başlangıcına bağlıydı: iki komşu odanın taraması
kendi köşesinden başlatılamıyor, bir taramanın çizgileri ötekinin devamı yapılamıyordu (AutoCAD'in “Set origin” ve “Match
properties” seçenekleri). Desen ölçeği her zaman kâğıttaydı (çizim ölçeğiyle çarpılır): gerçek boyutlu bir karo (30 × 30 cm)
çizim biriminde verilemiyor, pafta ölçeği değişince deseni kendi kâğıt boyunda tutan ölçekli tarama yoktu.

## Karar

### 1. Projenin desenleri ve PAT (`CAD-HATCH-06`)

`ProjectSettings.hatchPatterns`: projenin adlı desenleri (`HatchPatternDef`: ad, açıklama, birim, çizgi aileleri, kaynak dosya).
Aile PAT'ın satırıdır: açı, başlangıç, öteleme, kesikler (`PatternLine`, kitaplıkla aynı tür). Birim (`HatchPatternUnit`) desenin
sayılarının ne olduğunu söyler: `mm` ya da `inch` kâğıtta (çizim ölçeğiyle çarpılır: 1:100'de 1 mm 0,1 m), `drawing` çizim
biriminde (ölçek çarpmaz). Ad büyük küçük harf ayırmaz (`same_hatch_pattern_name`); projenin deseni aynı adlı kitaplık deseninin
yerine geçer. Kurallar (`hatch_patterns_problem`): ad boş ya da yinelenen olmaz, en az bir aile, sayılar sonlu, ötelemenin dik
bileşeni sıfır olmaz, kesiklerin hepsi sıfır olmaz.

PAT okuyucu ve yazıcı `kentos_formats::pat` (`read_pat`, `write_pat`): `*AD, açıklama` başlığı, `açı, x, y, dx, dy, kesikler`
satırları; `;` yorumları, boş satırlar, BOM ve CRLF; okunamayan satır satır numarasıyla söylenir, dosyanın geri kalanı okunur.
Yazıcı en kısa ondalık yazımla aynı sayıları verir (okuma ve yazma bit bit gidip gelir).

Komut `cad.hatchpatterns.edit` v1 (ayar, geri alma adımı değil; çizgi tipleri gibi, ADR 0313):

- **Yükle** (`load`: `pat`, `source`, `unit`, `replace`, `names`): dosyanın desenleri eklenir; aynı adlı olan `replace` ile yerinde
  değişir, yoksa atlanır ve söylenir; `names` yalnız seçilenleri alır.
- **Ekle** (`add`: `definition`), **Sil** (`remove`: `names`), **Dışa aktar** (`export`: `names`; salt okunur, PAT metni döner).

Retler: `no_pat` (dosyada desen yok), `invalid_hatch_pattern` (kural; satırın sorunu yolla), `hatch_pattern_not_found`,
`no_definition`, `empty_name`. Uyarılar: okunamayan satırlar, atlanan aynı adlar.

### 2. Desenin başlangıcı, Özellik al ve Hizala (`CAD-HATCH-07`)

`HatchPattern.origin` (dünya noktası, isteğe bağlı): desenin çizgi aileleri bu noktaya bağlanır. Çizgilerde ve çaprazda ailenin
ötelemesi başlangıcın aileye dik bileşeniyle kayar; PAT ailelerinin başlangıçları başlangıç kadar taşınır. Yoksa dünyanın
başlangıcı (eski taramalar değişmez). Dönüşümler (taşı, döndür, ölçekle, aynala, oturt) başlangıcı da götürür; olmayan başlangıç
olmayan kalır.

`cad.entities.hatch`'in deseni (`HatchPatternSpec`) üç alan alır:

- `origin`: `world` (dünya), `point` (`at`), `corner` (`corner`: sol alt, sağ alt, sol üst, sağ üst, orta; her bölgenin kendi
  kutusundan, `across` ile bütün bölgelerin ortak kutusundan).
- `basis`: `paper` (Ölçek kâğıtta, varsayılan) ya da `world` (desenin tanımındaki bir mm çizimde bir mm).
- `annotative` (§3).

`cad.hatches.edit`'e iki işlem:

- **Deseni değiştir** (`pattern`: `uids`, `pattern`): taramaların deseni, ölçeği, açısı, başlangıcı; başlangıç verilmezse her
  taramanın kendisininki kalır. Adım “Deseni değiştir”.
- **Taramaları hizala** (`align`: `uids`, `reference`): taramalar başvurunun başlangıcını alır; aynı türde çizgileri varsa
  açısını, aralığını ve ölçeğini de: çizgileri başvurununkileri sürdürür. Dolu hedef atlanır; çizgisi olmayan başvuru
  `not_a_hatch` ile reddedilir. Adım “Taramaları hizala”.

Araçta (Tarama ve Çoklu tara): Desen (D) menüsünde kitaplığın desenlerinden sonra projenin desenleri “(proje)” ile; Ölçek tabanı
(E: kâğıtta, çizimde); Başlangıç (S: dünya, dört köşe, orta, nokta; Nokta tıklama ya da `Y,X` ister); Ölçekli (L); Özellik al
(Z: tıklanan taramanın deseni, kendi ölçeğindeki ölçeği, açısı, başlangıcı ve ölçekliliği araca geçer). Taramayı düzenle'ye
Hizala (H) kipi: başvuruya, sonra hizalanacak taramalara tıklanır. Önizleme komutun desen çözümüyle (`region::spec_pattern`,
`origin_of`) çizilir.

### 3. Ölçekli tarama ve çizimde ölçek (`CAD-HATCH-08`)

`HatchEntity.annotative` (ADR 0323'ün `Annotative`'i): ölçekli taramanın deseni kendi ölçeğinde (`at`) kaydedilir; başka bir
ölçekteki temsili ölçek oranıyla çarpılır (çizgilerde aralık, desende ölçek; açı ve başlangıç aynı). Çizim ölçeği değişince
ölçekli taramalar yeni ölçeğe taşınır (yazı, ölçü ve kılavuz gibi); paftanın haritası kendi ölçeğindeki temsili çizer.
`cad.annotations.scales` taramaları da alır; dolu ve degrade tarama ölçekli olmaz (`not_annotative`: “Yalnız yazı, ölçü,
kılavuz ve tarama ölçekli olur.”). Desenin `basis: world`'ü ölçekliyle birlikte verilemez (`not_annotative`, yol
`pattern.annotative`); dolu ya da degradede başlangıç `invalid_scale` (yol `pattern.origin`).

Öznitelikler'de taramanın Başlangıç satırı ve Ölçekli açıklama satırları (Ölçekli, Kendi ölçeği, Diğer ölçekler).

### 4. KCAD şema 71

`settings.hatchPatterns` (JSON), desenin `origin`'i (nokta; kanonik anahtar sırası: name, type, angle, lines, scale, origin,
spacing, gradient) ve taramanın `annotative`'i. Sütunlarda OPT[6]'nın JSON'unda. `FORMATS_VERSION` 80. Depoda başlangıçlı tarama
paket türü 26'dır (degradeden sonra x, y). Bağımsız Python okuyucusu ve yazıcısı aynı kuralla; örnek `fixtures/kcad/v2/hatch-extras.*`
ve beş bozuk dosya (şema 70'te ekler, dolu taramada başlangıç ve ölçeklilik, boş ve iki kez yazılmış desen).

### 5. Arayüz

- **Tarama desenleri** penceresi (`hatchPatterns.manage`; CAD'de Yönet › Standart, CBS'de Veri › Standart): projenin desenleri
  örnekleriyle, PAT dosyasının birimi (mm, inç, çizim birimi), PAT'tan yükle…, Dışa aktar…, Sil.
- İkonlar `hatchPatterns`, `hatchAlign` (`catalog/v1/additions.json`).
- Python `kentos.cad.hatchpatterns.edit`, MCP aracı (yıkıcı listede), manifestte `hatchPatterns.manage` `semantic`.

## Sonuçlar

- Bağımsız başvurular, KentOS kodu olmadan: `hatch_pattern_file_cases.py` (PAT: 24 okuma, 17 yazma, 9 ölçek durumu; komutun 30
  durumu), `hatch_pattern_cases.py` (başlangıçlı boyalar), `region_command_cases.py` (64 durum: başlangıç, taban, ölçekli,
  projenin desenleri, retler), `hatch_edit_cases.py` (65 durum: Deseni değiştir ve Hizala), `annotative_cases.py` (38 komut ve 41
  temsil durumu, taramalarla).
- Desen menüsünde projenin desenlerinin kendi örnek resmi yok (kitaplığınkiler hazır ikonlar); hepsi “Tarama desenleri” ikonuyla.
  Pencere örnekleri çizer.
- PAT'ın birimi dosyada yazmaz; kullanıcı seçer (acadiso.pat mm, acad.pat inç). Çok yoğun desen (çizim biriminde verilmiş küçük bir PAT
  büyük bir bölgede) çekirdeğin parça sınırıyla reddedilir (ADR 0062).

## Doğrulama

```bash
python3 scripts/fixtures/hatch_pattern_file_cases.py --check; python3 scripts/fixtures/hatch_pattern_cases.py --check
python3 scripts/fixtures/region_command_cases.py --check; python3 scripts/fixtures/hatch_edit_cases.py --check
python3 scripts/fixtures/annotative_cases.py --check; python3 scripts/fixtures/kcad_v2_reference.py --check
cargo test -p kentos-formats --test all pat; cargo test -p kentos-native-application --test all fixtures
KENTOS_SHOTS_ONLY=tarama-desenleri,tarama-hizala,tarama-baslangic,tarama-olcekli,tarama-desen-proje cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
```
