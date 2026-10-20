# ADR 0265: Modelin bütün katmanlara taşınması ve şema değişikliğinin kuralı

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §6 `CAD-DATA-22` (nesne, alt nesne ve ilişki modelinin `.kcad`'e,
  tipli sütunlara, WASM'a, bulutun kaynak tanımına ve Python tiplerine **birlikte** taşınması) ve `CAD-DATA-23` (yeni şemada eski
  okuyucunun açık reddi, yeni okuyucunun eski dosyayı açması, bağımsız doğrulama ve geri dönüş yolu). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0025 ve `docs/specs/kcad-v2.md` (KCAD v2), 0030 (tipli sütunlar), 0006 ve CLAUDE.md §15 (bulutta CAD kaynağı),
  0131 (Python SDK'sı), 0263 (son eklenen alanlar).
- **Web:** ertelendi (ADR 0228); web'in sütun okuyucusu (`io/columns.ts`) ve KCAD okuyucusu (`io/kcad.ts`) `FORMATS_VERSION` 53'te durur;
  gerekenler TODOS.md §0.2'de (her maddenin satırı).

## Bağlam

Bir nesne ya da alan eklendiğinde altı yerde birlikte yaşamalıdır: sözleşme (`kentos_contracts`), `.kcad` kodeği ve bağımsız Python
okuyucusu, tipli sütunlar (web işçisinin sınırı), WASM bağlayıcıları, bulutta saklanan özellik (PostGIS geometrisi ya da CAD tanımı) ve
Python'un tipli katmanı. Şimdiye dek her ADR kendi alanını bu katmanlarda tek tek sınadı; **bütün** örneklerin her katmandan geçtiğini
bir arada sınayan bir kanıt yoktu. Bu ADR'yi yazarken yapılan denetim gerçekten bir delik buldu: sunucu bütün sayıları ±10⁹ ile
sınırlıyordu; çok boyutlu rasterin zaman boyutu (milisaniye) ve nokta bulutunun nokta sayısı buluta yazılamıyordu.

## Karar

### 1. Taşıma matrisi ve bekçileri (`CAD-DATA-22`)

| Katman | Bekçi test | Örnekler |
|---|---|---|
| `.kcad` kodeği | `kentos-kcad` `fixtures::every_fixture_reads_as_expected_json_says`, bağımsız `tools/kcad/kcad.py` (`kcad_v2_reference.py --check`) | `fixtures/kcad/v2`'nin bütün geçerli örnekleri bit bit |
| Tipli sütunlar | `kentos-kcad` `columns::every_valid_fixture_crosses_the_typed_boundary_to_the_codecs_own_bytes` | aynı örnekler |
| Bulutun kaynağı | **yeni** `kentos-application` `tests/carriage.rs`: her örnek nesne `to_stored` → `from_stored` ile aynı gelir; blok tanımları `to_stored_block` ile | aynı örnekler (veritabanı gerekmez) |
| Python'un tipli katmanı | **yeni** `python/tests/test_carriage.py`: her örnek dosya `Document.open` ile açılır, her nesne `Entity`'ye okunur ve geri yazılır, örnekle aynıdır, kalıcı kimliğiyle | aynı örnekler |
| Komutlar | `fixtures/commands/v1` (masaüstü ve Python, aynı durumlar) | komutların ortak durumları |
| WASM | bağlayıcılar aynı Rust'tan derlenir; web dururken çalıştırılmaz | — |

- **Kural:** yeni alan ya da tür, önce bir `.kcad` örneğine (elle yazılmış `*.json`, bağımsız yazıcının `*.kcad`'i) girer; matrisin her
  bekçisi o örneği kendiliğinden alır. Örneksiz alan eklenmez.
- **Sunucunun sayı sınırı** koordinat ve boylar içindir (çöp değeri reddetmek için); verinin sayıları (`dataset`, `count`) sınırsızdır
  (`DATA_KEYS`). ±10⁹'u aşan koordinat reddedilir, sessizce bozulmaz (kodeğin `f64::MAX` örneği).

### 2. Şema değişikliğinin kuralı (`CAD-DATA-23`)

Her yeni şema `n`:

1. **Sabiti ve listesi:** `SCHEMA_WITH_…: u32 = n`, `SCHEMAS`'a eklenir; yazıcı `n`'yi **yalnız** yeni özellik varken yazar (en eski şema
   kuralı); başka her çizim eskisiyle bayt bayt aynıdır.
2. **Örnek:** `n`'de yazılmış geçerli bir örnek (`<özellik>.json` ve `.kcad`), bağımsız Python yazıcısıyla.
3. **Eski okuyucunun açık reddi:** `n − 1`'de yazılmış özelliğin bozuk örneği (`broken/<özellik>-in-schema-<n−1>.kcad`) ve reddi
   (`unknown_field` ya da `bad_value`, yeri adıyla); ve `broken/schema-version-<n+1>.kcad` (`schema_version`).
4. **Yeni okuyucunun eski dosyayı açması:** önceki bütün örnekler okunmaya devam eder (`fixtures` testi).
5. **Bağımsız doğrulama:** `tools/kcad/kcad.py` aynı kuralı KentOS kodu olmadan okur; `kcad_v2_reference.py --check`.
6. **Taşıma:** §1'in bekçileri yeni örneği geçer; `FORMATS_VERSION` sütunlar değiştiyse artar.

Bekçisi: **yeni** `kentos-kcad` `schemas::every_schema_has_a_sample_and_the_one_before_refuses_its_feature`: `SCHEMAS`'taki her şemanın
bir geçerli örneği, her şemanın (ilki dışında) bir önceki şemada reddedilen örneği ve sonrakinin `schema_version` reddi yoksa düşer.
Bugün 2'den 51'e her şema bu kuralı karşılıyor.

### 3. Geri dönüş yolu

- **Dosyanın geri dönüşü:** yazıcı çizimin tuttuğu **en eski** şemayı yazar; yeni bir özelliği kullanmayan çizim eski KentOS'ta açılır.
  Yeni özelliği kullanan dosya eski okuyucuda **açılmaz** (sessiz kayıp yok); onu eski sürüme götürmenin yolu yeni KentOS'ta özelliği
  kaldırıp kaydetmektir: dosya kendiliğinden eski şemaya döner (örn. ölçünün `along` yeri kalkınca şema 50).
- **Değişikliğin geri alınması:** bir şema numarası bir kez kullanıldı mı **yeniden kullanılmaz**; hatalı bir özellik geri çekilirse
  numarası ayrılmış kalır, o numarayla yazılmış dosyaları okuyan kod okumaya devam eder (alanı yok sayan değil, okuyup yeni biçime
  çeviren kod). Kabın `min_reader`'ı (spesifikasyon §4) bozucu değişiklik için ayrılmıştır; bugüne dek hiç artmadı.

## Sonuçlar

- Bütün `.kcad` örnekleri dört katmandan (kodek, sütunlar, bulut, Python) birlikte geçer; bir alan bir katmanda unutulursa test düşer.
- Çok boyutlu raster ve büyük nokta bulutu artık buluta yazılır (sunucunun sayı sınırı düzeltildi).
- Şema eklemenin altı adımı bir testle zorlanır.

## Doğrulama

- `cargo test -p kentos-application --test carriage` (bütün geçerli örneklerin nesneleri ve blokları; bir çöp örnek reddedilir).
- `.run/py/bin/python -m unittest discover -s python/tests -p "test_carriage.py"`.
- `cargo test -p kentos-kcad --test all schemas`.

## Ek (21 Ekim 2026): bir proje özelliği tek başına taşınmıyordu

**Kusur.** Yazıcı (`schema_of`) çizimin tuttuğu en eski şemayı yazar; şema 72'nin listesi ayarların `domains`'ini, `relations`'ını ve
alanın `domain`'ini içermiyordu. Yalnız bunlardan birini taşıyan proje (değer listesine bağlı alanlı ya da ilişkili ama formsuz, alan
kimliksiz) şema 26 ya da daha eskisiyle yazılıyor, geri okuma denetimi “bilinmeyen alan” diye düşüyor, Kaydet yazamıyordu (örnek:
`fixtures/profiles/v1/belediye.json`). Biçim değişmedi: okuyucu, spesifikasyon ve bağımsız Python yazıcısı (`kcad_v2_reference.py`)
bu anahtarları hep şema 72'ye bağlıyordu; yalnız Rust yazıcısı unutmuştu. Bu yüzden şema numarası artmadı, `FORMATS_VERSION` değişmedi.

**Matris neden kaçırdı.** Bekçiler örnek dosyaları bütün olarak okuyor ve yazıcının baytlarını örneğin baytlarıyla karşılaştırıyordu;
ama hiçbir geçerli örnek bu üç anahtarı başka bir şema 72 özelliğinden (alan kimliği, form) ayrı taşımıyordu: `domains.json` alan
kimlikleri de taşıyordu, `forms.json` formları. `broken/domains-in-schema-71.kcad` yalnız okuyucunun reddini sınıyordu, yazıcının
72'yi seçtiğini değil.

**Düzeltme.** Yazıcı bunları 72'nin listesine alır; spesifikasyon (§6, şema 72) listeyi söyler; her biri için tek başına örnek
(`domains-alone`, `field-domain-alone`, `relations-alone`, `field-max-exclusive`, bağımsız yazıcıyla, şema 71'deki reddiyle) eklendi.

**Matris genişledi** (`kentos-kcad` `tests/all/carriage.rs`): ayarların her alanı, belgenin her parçası, katmanın her anahtarı ve
alanın her anahtarı, örneklerdeki ilk değeriyle **tek başına** en küçük çizime konur, yazılır, geri okunur ve aynı gelmesi beklenir;
sözleşmenin türleri alan alan adlandırıldığı için (`ProjectSettings`, `DocumentSnapshotV2`, `LayerField` açık ayrıştırılır) yeni bir
alan adlandırılmadan derlenmez, örneği olmayan düşer. Ayrıca bir proje değer listeleri, ilişkiler ve formlarla birlikte ve profil
projeleri Kaydet'in yoluyla (`encode_verified`) kaydedilip açılır; Python'da her geçerli örnek açılıp kaydedilip yeniden açılır
(`test_carriage.py`).
