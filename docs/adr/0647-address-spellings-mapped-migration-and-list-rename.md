# ADR 0647: Adres yazılışlarını düzeltmek, eşlemeyle göç ve değer listesinin kimliğini değiştirmek

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-34'ün dördüncü dilimi. Kapsamı ben belirledim.
- **Kalan işler:**
  - ADR 0579'dan: yazılışları düzelten komut, mahalle ve ilçe adlarının farklı yazılışları;
  - ADR 0558'den: göçün eşlemeyle çalışması, değer listesinin kimliğini değiştirmek.
- **Bağlam belgesi:**
  - ADR 0229 (adres verisi, dizin, Adres verisi penceresi);
  - ADR 0579 (`cad.addresses.check`);
  - ADR 0541 ve 0558 (veri profili, kurumun profili ve eşlemesi, `cad.profiles.migrate`);
  - ADR 0430 (projenin değer listeleri).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - profil dosyalarının `.kcad`'in içinde taşınması (bugün çizimin yanındaki `profiller` klasöründe; Paketle onları
    kopyalar);
  - adres kodunun UAVT gibi bir dış kayıtla karşılaştırılması;
  - ifraz, tevhit, hisse ve PlanGML (mevzuat işi, sahibin kararıyla sona kalır);
  - Adres verisi penceresinde tercih edilen yazılışı bulgu satırından seçmek;
- gerçek kullanıcılarla günlük kullanım kabulü.

## Bağlam

`cad.addresses.check` (ADR 0579) bir yolun kapılarının onun adını farklı yazdığını söyler (“Atatürk Bulvarı”,
“ATATÜRK BULVARI”, “Atatürk Blv.”). Ama düzeltmeyi kullanıcıya bırakır: kapıları tek tek açıp yazmak gerekir. Mahalle ve
ilçe adlarının farklı yazılışlarını hiç söylemez.

Kurumun profili ve eşlemesi (ADR 0558) kurumun katman, alan ve kod adlarıyla `apply` ve `check` yapar. Ama göç profilin
adlarıyla çalışır: kurumun adlarıyla yazılmış veride göç adımları katmanı ya da alanı bulamaz. Eşleme adımı kendi adına
çevirmez. Değer listesinin kimliğini değiştirmek de bir göç adımı değildir: liste bölünüp birleştirilebilir
(`mapValues`), ama listenin kimliği değişemez.

## Karar

### §1 Mahalle ve ilçe yazılışları

`ops::geocode::check` iki liste daha verir. Kural yolunkiyle aynıdır (ADR 0579 §1):

- **Mahalle:** kapılar mahallenin anahtarıyla (`area_value`: MAHALLE ve KÖY etiketleri olmadan, katlanmış sözcükler) ve
  ilçenin anahtarıyla (`words_key`) gruplanır. Kapıların yazdığı mahalle adı birden çok biçimdeyse bulgudur. Boş ad
  sayılmaz.
- **İlçe:** kapılar ilçenin anahtarıyla gruplanır, ilçe adının yazılışları sayılır.

Her yazılış kırpılmış metnidir ve onu yazan kapı sayısıyla söylenir. En çok yazılan önce gelir; eşitlikte önce yazılan.
Grupların sırası ilk kapılarınındır. Her listenin sayısı bütün listedir; liste ilk 100 gruptur.

`cad.addresses.check`'in cevabı `neighbourhoodsSpelled`, `neighbourhoodSpellings`, `districtsSpelled` ve
`districtSpellings` alır. Her öğe `within` (mahallede ilçenin ilk yazılışı, ilçede boş), `written` ve kapıların kalıcı
kimlikleridir (`uids`).

### §2 `cad.addresses.normalize`

Bir adres verisinin kapılarının yol, mahalle ya da ilçe adını her grupta tek yazılışa getirir. Tek geri alma adımıdır
(“Adres yazılışlarını düzelt”).

**Girdi:**

- `locator` (yoksa projenin ilk adres verisi);
- `what`: `street` (varsayılan), `neighbourhood` ya da `district`;
- `prefer`: tercih edilen yazılışlar. Bir grup bunlardan birini yazıyorsa ilk geçen seçilir; yoksa en çok yazılan
  (eşitlikte önce yazılan);
- `expectedRevision`.

**Gruplar** §1'in ve ADR 0579'un gruplarıdır: birden çok yazılışı olanlar. Grubun her kapısı, kırpılmış yazılışı
seçilenden başkaysa, kapı katmanının ilgili alanına seçilen yazılışı alır. Baştaki ve sondaki boşluk farkı yazılmaz.

**Cevap:**

- `groups`: her grup için seçilen yazılış (`to`), yerine geçenler (`from`) ve yazılan kapı sayısı;
- `written`: yazılan kapılar;
- `uids`: yazılan kapıların kalıcı kimlikleri, çizimin sırasıyla;
- sürüm.

Yazılacak kapı yoksa adım açılmaz. `plan` aynı cevabı yazmadan verir.

**Retler:**

- `invalid_revision`, `revision_conflict`;
- `no_address_data`, `unknown_locator`;
- `invalid_input`: adres verisinin kapılarında istenen alan yok (mahalle ya da ilçe alanı tanımlanmamış), ya da
  `prefer`'de boş yazılış;
- `layer_locked`: kapı katmanı kilitli.

Değerler alanın kuralından geçer (`checks::object_attrs`'in denetimi). Uymayan değer bütün komutu reddeder: seçilen
yazılış zaten kapılardan birinin yazdığıdır, uymaması yalnız alanın kuralı sonradan değiştiyse olur.

Masaüstünde Adres verisi penceresinin Denetle'si mahalle ve ilçe yazılışlarını da söyler. Pencerenin altında
**Yazılışları düzelt** vardır: Denetle farklı yazılış bulduğunda ve seçili tanım kaydedildiği gibi durduğunda açıktır
(komut projenin adres verisiyle çalışır). Basınca yol, mahalle ve ilçe için, bulgusu olanlar, en çok yazılan yazılışla
düzeltilir; hepsi tek geri alma adımıdır, ardından Denetle yeniden çalışır. Tercih edilen yazılış (`prefer`) yalnız
komuttadır: pencerede yazılışa basıp seçmek sonraya kaldı.

### §3 Değer listesinin kimliğini değiştirmek

Göçün yeni adımı `renameDomain { domain, to }`:

- Projenin `domain` kimlikli değer listesinin kimliği `to` olur.
- Ona bağlı bütün alanlar (`LayerField.domain`) yeni kimliğe bağlanır. Kodlar ve veriler değişmez.
- Liste projede yoksa adım atlanır ve söylenir (`migration_step_skipped`).
- Projede `to` kimlikli başka liste varsa göç reddedilir (`domain_exists`).
- Profilin denetimi: eski kimlik dolu, `to` küçük harf, rakam ve `-`'dir ve eskisinden farklıdır (`invalid_profile`).

### §4 Eşlemeyle göç

`cad.profiles.migrate` `mapping` ya da `mappingName` alır (`apply` ve `check` gibi). Eşleme sınıfın katmanını, alanlarını ve
kodlarını kurumun adlarıyla söyler. Göç adımları ise profilin adlarıyla yazılır.

**Adımın sınıfı.** Adım profilin bir katman yoluna bakar. Yol, göçte ondan sonra gelen `renameLayer` adımlarıyla hedef
sürümdeki adına götürülür. Hedef sürümün sınıflarından katmanı o yol olan sınıf, adımın sınıfıdır. Alan adı da sonraki
`renameField` adımlarıyla hedef sürümdeki adına götürülür.

**Adımlar:**

- `renameLayer`: eşleme sınıfın katmanını veriyorsa atlanır (kurumun adı değişmez), notunda söylenir. Vermiyorsa profilin
  adlarıyla çalışır.
- `renameField`: eşleme alanın adını veriyorsa atlanır. Vermiyorsa katman yolu kurumunkine çevrilerek çalışır.
- `changeFieldType`: katman ve alan kurumun adlarıyla; alanın yeni tanımı eşlemeyle okunmuş profilinkidir (`through`).
- `mapValues`: katman ve alan kurumun adlarıyla. Eşleme alanın kodlarını veriyorsa her iki yandaki kod kurumunkine
  çevrilir; eşlemenin bilmediği kod olduğu gibi kalır.
- `retireCode` ve `renameDomain`: projenin değer listesine bakar, olduğu gibi çalışır. Eşlenen kodlu alan listeye bağlı
  değildir (`through` onu ayırır), değerleri değişmez.

Sınıfı bulunamayan adım profilin adlarıyla çalışır. Cevabın `mapping`'i kullanılan eşlemenin adıdır.

## Doğrulama

- `scripts/fixtures/address_check_command_cases.py` (22 durum; mahalle ve ilçe yazılışları kapı 18 ve 19 ile),
  `scripts/fixtures/address_normalize_cases.py` (27 durum: üç ad, `prefer`, kırpılan boşluk, plan, geri alma, kilitli katman,
  alanı olmayan adres verisi, alanın kuralına uymayan yazılış, retler) ve `scripts/fixtures/profile_files_cases.py`'nin göç
  durumları (50; `renameDomain` beş, eşlemeyle beş) KentOS kodu olmadan yazılır.
  `KENTOS_CASES_ONLY=cad.addresses,cad.profiles cargo test -p kentos-native-application --test all every_command_case` onları
  oynatır; yetenek testi yeni kodları (`domain_exists`, `unknown_mapping`, `invalid_profile_mapping`) durumlarda bulur.
- Masaüstü: `address::tests::yazilislari_duzelt_sets_the_names_right_in_one_step` (Denetle, Yazılışları düzelt, tek geri alma
  adımı, Denetle'nin yeniden bulguyu söylememesi); Veri profili penceresinin göçü seçili eşlemeyle çalışır.
- Python SDK yeniden üretildi; `pnpm py:test` 270 testle geçti. MCP'de `cad.addresses.normalize` yıkıcı araçtır.
- Resimler: `KENTOS_SHOTS_ONLY=adres-yazilislar,adres-yazilislar-duzeltildi cargo test -p kentos-desktop tools_screens --
  --ignored --nocapture` (`.run/shots/arac-adres-yazilislar-*`), 1440×900 ve 1100×650, iki temada.

## Ölçümler

Release, bu makinede (`cargo test --release -p kentos-native-application --test all address_spellings -- --ignored
--nocapture` ve `profile_files::mapped_migrate_timing`, `profile_files::migrate_timing`):

| İş | Plan | Yazma |
|---|---|---|
| `cad.addresses.normalize`, 100 000 kapı, yol (1 000 grup, 28 572 kapı) | 202 ms | 263 ms |
| aynı, mahalle (1 grup, 9 091 kapı) | 218 ms | 242 ms |
| aynı, ilçe (1 grup, 7 693 kapı) | 226 ms | 240 ms |
| `cad.profiles.migrate` eşlemeyle, 100 000 ağaç, bütün zincir | 385 ms | 378 ms |
| aynı, eşlemesiz (karşılaştırma) | 975 ms | 903 ms |

Süreyi adres dizininin kurulması belirler (planda ve yazmada birer kez). Eşlemeyle göç daha kısadır: eşlemenin verdiği adları taşıyan
adımlar atlanır.
