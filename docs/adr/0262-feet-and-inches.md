# ADR 0262: Ayak, inç ve mimari kesirli uzunluk

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §6.1 `CAD-DATA-12`. Kapsamı ben belirledim (sahibin sözleri:
  “CAD data bölümüne geç ve orayı tamamla”, “durmadan devam et”; planın “uluslararası uyumluluk için ayrı genişletme; metrik ilk kabulü
  geciktirmesin” notu).
- **Bağlam belgesi:** ADR 0165 §2 (yerel projenin çizim birimi), 0245 (Python ve MCP'de birim), 0256 (birim değiştirme, şemalarda
  birim), 0257 (DXF'in birimi), 0149 ve `docs/specs/numeric-policy.md` (gösterim yuvarlaması), 0025 ve `docs/specs/kcad-v2.md`.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

- Yerel projenin çizim birimi mm, cm ya da m'ydi (ADR 0165 §2); geometri her zaman metrededir, birim yalnız okuma ve yazmadadır.
- Ayak ve inçle çalışan bir mimarın ya da Amerikan ve İngiliz DXF'lerinin birimi (`$INSUNITS` 1, 2) karşılıksızdı: DXF içe aktarmada
  “birimi bildirmeyen” gibi davranılıyor ya da reddediliyordu; uzunluklar `12'-6 1/2"` gibi yazılamıyor, okunamıyordu.

## Karar

### 1. İki yeni çizim birimi

- `DrawingUnit`'e `in` (inç) ve `ft` (ayak) eklenir. Yalnız koordinat sistemi olmayan yerel projede seçilir, mm ve cm gibi
  (`unit_needs_local`). 1 inç tam 0,0254 m, 1 ayak 0,3048 m'dir (uluslararası ayak; ABD'nin ölçme ayağı yoktur).
- Geometri metrede kalır; birim yalnız okuma ve yazmadadır. `cad.units.change` in ve ft'yi de alır; ölçeklemenin çarpanı birimlerin
  metre karşılıklarının oranıdır.
- **DXF:** `$INSUNITS` 1 (inç) ve 2 (ayak) bu birimlerdir: okumada çevirinin çarpanı, yazmada projenin birimi yazılır (ADR 0257'nin
  kuralı aynıdır: bildirilen birim sessizce yok sayılmaz).

### 2. Uzunluk biçimi

Projenin yeni ayarı `lengthStyle` (yoksa `decimal`):

| Biçim | Örnek (150,5 inç) | Anlamı |
|---|---|---|
| `decimal` | `150.5000` (inçte), `12.5417` (ayakta) | çizim biriminde ondalık; bugünkü |
| `architectural` | `12'-6 1/2"` | ayak, inç ve inçin kesri |
| `engineering` | `12'-6.50"` | ayak ve ondalık inç |
| `fractional` | `150 1/2"` | inç ve kesri |

- Ondalık olmayan biçimler yalnız in ve ft birimlerinde seçilir (`style_needs_imperial`).
- **Duyarlık:** `lengthDecimals` ondalık biçimlerde basamak sayısı, kesirli biçimlerde paydanın üssüdür: 0 tam inç, 1 yarım, 2 çeyrek,
  3 sekizde bir, 4 on altıda bir (AutoCAD'in LUPREC'i gibi). Kesir sadeleşir (`8/16` `1/2` olur); yuvarlama yarımı yukarı alır
  (`Math.round`), gösterim kuralının dışında değildir: yalnız gösterimdir, kaynak sayıyı değiştirmez (ADR 0149).
- Koordinatlar, uzunluklar ve ölçü değerlerinin varsayılan yazımı bu biçimdedir; alanlar biriminin karesinde ondalıktır (ft², in²).
  Ölçü stilinin kendi birimi (ADR 0183) ayrıdır.

### 3. Yazılan uzunluk

- Ayak (`'`) ya da inç (`"`) işaretli bir uzunluk her projede okunur ve çizim biriminden bağımsızdır: `5'6-1/2"` 66,5 inçtir, mm
  projesinde de 1689,1 mm'dir. İşaretsiz sayı bugünkü gibi çizim birimindedir (inç projesinde inç, ayak projesinde ayak).
- Dilbilgisi AutoCAD'in mimari girişidir: `[-+] [AYAK '] [-] [İNÇ] [[-] PAY/PAYDA] ["]`; en az bir işaret; içinde boşluk yok (boşluk iki
  koordinatı ayırır). `5'`, `6"`, `5'6"`, `5'-6"`, `5'6-1/2"`, `6-1/2"`, `1/2"`, `5'6.5"`.
- Mutlak, göreli ve kutupsal noktalarda, mesafede ve araçların uzunluk girişlerinde (`typed_length`) geçerlidir; açı işaretsizdir.
  Ortak durumlar `fixtures/point-input/v1`'dedir.

### 4. Dosyalar

- **KCAD v2 şema 50:** `drawingUnit`'in `in` ve `ft` değerleri ve ayarların `lengthStyle`'ı. Yazıcı 50'yi yalnız bunlardan biri varken
  yazar; eski okuyucu bilinmeyen birimi `bad_value` ile, `lengthStyle`'ı `unknown_field` ile reddeder.
- **Sunucu:** proje ayarları JSON'dur; ayrı sütun yoktur.

### 5. Arayüz, Python ve ajan

- Proje ayarları'nda Çizim birimi listesine İnç ve Ayak, yanında Uzunluk biçimi; yeni proje sihirbazında birimler.
- `cad.settings.set`'in `lengthStyle`'ı; `Document.new(drawing_unit="in")`; özetin `units`'i `in` ve `ft`'yi söyler. Tel metredir
  (ADR 0245).

### 6. Sonraya kalanlar

- ABD ölçme ayağı (1200/3937 m) ve yarda, mil; açıların derece-dakika-saniye ve gezgin yazımı (N 45°E).
- Ölçü stilinin kendi biriminde mimari biçim (ölçü değeri bugün ondalık stil birimiyle yazılır).
- Tablo çizelgelerinin (koordinat ve alan çizelgesi) ayak-inç biçimi: bugün inç ve ayakta ondalıktır.

## Sonuçlar

- Ayak ve inçle çalışan proje kurulur, uzunluklar `12'-6 1/2"` gibi yazılır ve okunur; DXF'in inç ve ayağı kayıpsız gelir ve gider.
- Metrik projeler değişmez: varsayılan biçim ondalıktır, şema 49 ve öncesi aynı bayttır.

## Doğrulama

- Çekirdek: `tools::imperial`'in testleri (okuma ve dört biçim); `fixtures/point-input/v1`'in dokuz ayak ve inç durumu (elle
  hesaplanmış); `Format`'ın `an_inch_project_writes_feet_and_inches`'ı.
- Komutlar: `cad.settings.set`'in altı ayak ve inç durumu (`settings_set_command_cases.py`) ve `cad.units.change`'in dört in ve ft
  durumu (`units_change_cases.py`, oranlar en küçük terimleriyle); `Scale`'in `inches_and_feet_keep_the_numbers_by_their_ratio`'su.
- KCAD: `imperial.kcad` ve dört bozuk dosya, bağımsız Python okuyucu ve yazıcısı (`kcad_v2_reference.py --check`), `kentos-kcad`'in
  `imperial.rs`'i.
- DXF: `an_inch_project_is_written_in_inches_and_read_back_in_metres` (`$INSUNITS` 1, inçte koordinat, geri okununca metre).
- Python: `test_units.py`'nin `FeetAndInches`'ı.
- Masaüstü resimleri: `KENTOS_SHOTS_ONLY=inc-ayarlar,inc-duvar,inc-yazilan cargo test -p kentos-desktop tools_screens -- --ignored
  --nocapture` (`.run/shots/arac-inc-*`).
