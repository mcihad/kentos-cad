# ADR 0731: Komut kataloğunda paylaşılan tanımlar

- **Durum:** kabul edildi (2026-10-28). Sahibin görevi: GitHub, `commandCatalog.json` için gönderimde 50 MB uyarısı verdi;
  dosya günde yaklaşık 2,4 MB büyüyordu ve 100 MB'ta gönderim reddedilecekti.
- **Bağlam belgesi:** ADR 0013 (ürün komutları ve katalog), 0131 (Python SDK'sı), 0133 (MCP), 0253 (komut protokolü),
  0256 (birimler, `x-unit`), 0540 (`kentos-commands`).
- **Web:** dururken (ADR 0228) yalnız üretilen tür (`CommandCatalog.ts`) değişti; web'in kataloğu okuyan testi (`registry.test.ts`)
  yalnız komutların kimliğini, sürümünü ve ev sahibini okur. Web dönerken şemayı tek başına isteyen bir okuyucu olursa §3'ün
  kuralını uygular (TODOS.md §0.2).

## Bağlam

`apps/web/src/contracts/generated/commandCatalog.json` her komutun girdi, çıktı ve plan şemasını JSON Schema olarak taşır. Şemalar
`schemars`'tan gelir: her şema, adıyla andığı bütün türleri kendi `$defs`'inde tekrar yazar. `Entity` gibi büyük türler 300 komutun
çoğunun üç şemasında da vardı:

| Ölçü | Değer |
|---|---|
| Dosya (girintili) | 56,6 MB (20 Ekim'de 37,1 MB) |
| Girintisiz içerik | 28,8 MB |
| Bunun `$defs`'teki kısmı | 27,8 MB (678 şemada) |
| Tekil tanımlar (ad ve gövde) | 1 174 tanım, 1,05 MB |
| Aynı adla farklı gövde | 0 |

Her yeni komut, andığı bütün türleri bir kez daha yazıyordu; dosya komut sayısıyla doğrusal büyüyordu.

## Karar

1. **Dosya kataloğun paylaşılan biçimidir (sürüm 2).** Komutların şemalarındaki `$defs` kataloğun üst düzeyindeki tek `$defs`'e
   taşınır; her tanım bir kez yazılır. Şemalardaki başvurular aynı kalır: `#/$defs/Ad`. JSON Schema, kendi `$id`'si olmayan, bir
   belgeye gömülü şemadaki `#` başvurusunu belgenin köküne göre okur; kataloğun kendisi bir belge olarak okunduğunda başvurular
   doğru yere çıkar.
2. **Bellekteki katalog değişmez (sürüm 1).** `kentos_commands::catalog()` her şemayı yine kendi `$defs`'iyle, tek başına
   kullanılabilir verir. Onu okuyanlar olduğu gibi çalışır:
   - MCP'nin araç şemaları (`inputSchema` kendi `$defs`'iyle);
   - başsız sunucu (`kentos_headless::catalog()`) ve Python'un yerli modülü (`kentos.cad.catalog()`'un `input_schema`'sı);
   - kataloğun kendi testleri (yetenekler, `x-unit`).
3. **İki yön tek kuralla, sözleşmede.**
   - `CommandCatalog::shared()` tanımları toplar. Aynı ada iki farklı gövde düşerse hata verir: dosya ikisini birden söyleyemez,
     türlerden birine başka ad verilmelidir.
   - `CommandCatalog::standalone(şema)` bir şemayı dosyadan yeniden tek başına kurar: eriştiği tanımları, onların eriştiklerini de,
     kendi `$defs`'ine koyar.
   - `standalone_all()` bütün kataloğu sürüm 1'e döndürür.
   - `CommandCatalog.version` 1 ya da 2'dir. Sürüm 2'de `$defs` alanı doludur; sürüm 1'de yazılmaz.
4. **Dosyayı okuyan betikler** tanımları kataloğun `$defs`'inden alır; sürüm 1 dosyası da okunmaya devam eder:
   - `scripts/python/sdk.py` (Python türleri);
   - `scripts/gis/coverage.py` (seçenek noktaları);
   - `scripts/python/history_calls.py` (komut geçmişinin Python kodu).

   `scripts/cad/manifest.py`, masaüstünün başlık testi ve web'in testi yalnız komut alanlarını okur; değişmediler.

## Seçilmeyenler

- **Dosyayı bölmek** (komut başına bir dosya): her dosya yine andığı türleri taşırdı; toplam boyut ve büyüme aynı kalırdı.
  Okuyucuların hepsi tek dosyayı bekliyor.
- **Sıkıştırılmış dosya ya da Git LFS:** büyüme sürerdi, fark okunmaz olurdu. LFS depoya yeni bir bağımlılık eklerdi.
- **Girintisiz yazmak:** yalnız yarıya indirirdi ve farkları okunmaz yapardı. Dosya girintili kalır.
- **Bellekteki kataloğu da paylaşılan biçime geçirmek:** MCP, Python ve başsız sunucu her şemayı yeniden kurmak zorunda kalırdı;
  hata yüzeyi büyürdü, kazanç olmazdı.

## Sonuçlar

- Dosya 56,6 MB'tan 4,25 MB'a indi (girintisiz 2,73 MB). Bir komut artık yalnız kendi kök şemalarını (ortalama ~1 KB) ve yeni
  türlerini ekler; büyüme komut sayısıyla değil, tür sayısıyla olur.
- Dosyadaki bir şemayı tek başına kullanmak isteyen okuyucu (bir doğrulayıcı, bir ajan) §3'ün kuralını uygular. Python'da
  `{**şema, "$defs": katalog["$defs"]}` yeterlidir.
- Komutların türleri ve şemaları değişmedi: üretilen Python, TypeScript ve MCP şemaları aynıdır.

## Doğrulama

- `cargo test -p kentos-commands`: 928 test. `the_shared_catalog_gives_back_every_schema` her komutun her şemasını paylaşılan
  dosyadan yeniden kurar ve bellekteki şemayla bit bit karşılaştırır. Dosyada hiçbir şemanın anmadığı tanım yoktur.
- `python3 scripts/python/sdk.py --check`: üretilen Python aynı.
- `python3 scripts/cad/manifest.py --check`: tutarlı (919 satır).
- `scripts/gis/coverage.py`'nin satırları iki sürümden aynı çıkar (299 komut). Kayıtlı matris bu değişiklikten önce de eskiydi
  (275 komut); ayrı iştir.
- `cargo test -p kentos-mcp`: 21 ikilide 81 test; araçların `inputSchema`'sı yine kendi `$defs`'iyle.
- `cargo test -p kentos-native-application --test all every_command_case`: bütün ortak komut durumları.
- `cargo test -p kentos-headless --test headless sheet_`: dosya komutlarının durumları (8 test).
- `cargo test -p kentos-desktop --bin kentos-cad history::text`: masaüstünün başlıkları dosyadakilerle aynı (3 test).
