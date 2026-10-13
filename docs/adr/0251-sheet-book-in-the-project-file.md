# ADR 0251: Pafta kitabı projenin dosyasında; cihazdaki kitaplardan göç

- **Durum:** kabul edildi (2026-10-13). [plan/cad.md](../../plan/cad.md) §24.1'in dokuzuncu işi (TODOS.md `CADP-09`; plan
  `CAD-SHEET-01`–`04`, `05`'in sahiplik ve geri alma kısmı). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0164 (pafta düzeni), [docs/sheet/integration.md](../sheet/integration.md) §6.1–6.2, ADR 0025
  (KCAD v2), ADR 0031 ve 0038 (dosya projeleri), ADR 0043 (masaüstünün bulut eşitlemesi), ADR 0092 (projenin stil kitaplığı:
  düzenleme, geri alma adımı değil).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Pafta kitabı (paftalar, ana sayfalar, öğeler, değişkenler, resimlerin bilgileri) ADR 0164'ten beri cihazda duruyordu:

- web'de IndexedDB `kentos.sheets.v1`'de,
- masaüstünde `$XDG_DATA_HOME/kentos-cad/pafta/kitaplar/<anahtar>.json`'da,
- resimlerin baytları ayrıca `resimler/<sha256>`'da.

Anahtar projeyi adlandırıyordu: `bulut/<kurum>/<proje>`, `proje/<kimlik>`, `dosya/<ad>` ya da `oturum/…`. Bunun sonuçları:

- Dosya başka bilgisayara giderse paftaları gitmez.
- Aynı adlı iki dosya (`dosya/ada.kcad`) aynı kitabı paylaşır.
- Bulut projesinin paftaları başka kullanıcıya gitmez; kontrol noktası ve kopya onları taşımaz.

`.kcad` taşınabilir proje olmalıdır: kaydedilen her şey dosyanın içindedir.

## Karar

### 1. `.kcad` belge şeması 44: `sheets`

- Belgenin `sheets`'i (spesifikasyon §6.12) iki alandır:
  - `book`: pafta çekirdeğinin kitabı (`kentos.sheet/1`), opak harita. Dosya biçimi onu yorumlamaz, olduğu gibi taşır; proje
    stillerinin öğeleri gibi (§6.7).
  - `pictures`: kitabın resimlerinin baytları, SHA-256'larıyla.
- Dosya biçiminin kuralları:
  - Resim baytları bayt dizgisidir. Özet 32 bayttır ve baytların SHA-256'sı olmalıdır.
  - Aynı resim iki kez verilemez; boş liste yazılmaz.
  - Kitap haritadır.
- Yazıcı 44'ü yalnız projenin kitabı varken yazar. Başka her çizim eskisiyle bayt bayt aynıdır. Eski okuyucu şema 44'ü açmaz
  (`schema_version`): paftaları bir sonraki kayıtta sessizce düşürmez.
- Sözleşmenin JSON'unda (`DocumentSnapshotV2.sheets`, `ProjectSheets`) baytlar base64 metindir. Base64 ve SHA-256'nın tek kopyası
  sözleşmededir (`kentos_contracts::encoding`); pafta çekirdeği onları yeniden dışa aktarır.
- `FORMATS_VERSION` 56; web'in kopyası 53'te kalır.

### 2. Kitabın kuralları pafta çekirdeğinindir

`kentos_sheet::project` kitabı projeye yazar ve projeden okur, her ev sahibi için tek kuralla:

- **Yazma:**
  - Kitabın andığı her resmin ev sahibinde olan baytları kitabın sırasıyla yazılır.
  - Ev sahibinde olmayan resim yazılmaz; paftanın ön denetimi söyler.
  - Boş kitap (pafta, ana sayfa, resim ve değişken yok) kitap değildir: proje kitapsız kalır.
- **Okuma:**
  - Kitap bir `.kpafta`'nınki gibi denetlenir (`read_book`).
  - Kitabın andığı her resim bilgisine (özet, boy) tutulur; tutmayan kitabı okunmaz yapar, yarım kitap olmaz.
  - Kitabın anmadığı resim kitabın değildir; bırakılır.

### 3. Belgede kitap: düzenleme, geri alma adımı değil

- Masaüstünün belgesi (`kentos_domain::Document`) kitabı paylaşılan olarak tutar (`sheets`, `sheets_shared`); okuma kopyası
  resimleri kopyalamaz.
- `set_sheets` bir düzenlemedir, ama çizimin geri alma adımı değildir:
  - Kitap kâğıdındır; pafta tasarımcısının kendi geri alması vardır. Çizimin Geri al'ı paftayı geri almaz, paftanın Geri al'ı
    çizimi geri almaz.
  - Proje stil kitaplığının kuralı budur (ADR 0092).
  - Model ile kâğıdın ayrı geçmişleri `CAD-SHEET-05`'in ilk cevabıdır. Betiğin ikisini birlikte yazan tek işlemi sonraki iştir.
- Dosya biçiminin kurallarına uymayan kitap reddedilir (`sheets_problem`); açılışta da.

### 4. Masaüstünde kitap çizimle gider

`apps/desktop/src/sheet_book.rs`:

- **Çizim açılınca** ya da kitabı başka bir şey (bulut eşitlemesi, betik) değiştirince tasarımcı çizimin kitabını gösterir.
  Belge kitabı göstericiyle karşılaştırır (`Arc::ptr_eq`).
- **Paftada yapılan her değişiklik** o anda çizime yazılır (tasarımcının `book_revision`'ı ilerleyince). Kaydedilmemiş
  noktası ve kayıt soruları çizimle birdir.
- **Okunamayan kitap** (bilinmeyen şema, bozuk resim) bir kez söylenir, çizim açılır. Kitap çizimde olduğu gibi kalır;
  paftalarda yapılan değişiklik onun üstüne yazılmaz.
- Tasarımcı kitap tutmaz; cihazın deposu yalnız şablonların ve şablon resimlerinin yeridir. Projenin resimleri artık
  `resimler/`'e yazılmaz.

### 5. Cihazdaki kitaplardan göç

Göçün dayanağı projenin anahtarıdır (`kentos_sheet_ui::project_key`, eski yazıcının kendi kuralı).

- **Kitabı olmayan proje açılınca:**
  - Cihazda o anahtarda dolu bir kitap varsa kitap ve cihazdaki resimleri projeye alınır.
  - Çizim değişir (kaydedilmemiş). Günlükte bir satır söylenir; cihazda baytı olmayan resimler de sayılır.
- **Proje o kitapla kaydedilince** (belge kirli değil, kitap içinde) cihazdaki kopya kenara alınır:
  `kitaplar/tasindi/<ad>-<zaman>.json`.
  - Silinmez; kaydedilmeden kapatılan proje kitabı yine cihazda bulur.
- **Kitabı olan proje açılınca** cihazdaki kopya da kenara alınır: dosyadaki projenindir. İkisi farklıysa günlükte cihazdakinin
  yeri söylenir. Karşılaştırma içerikle yapılır (okunan kitapların eşitliği; özetten güçlü).
- **Hiçbir zaman:** bulunamayan kitap boş kitapla ezilmez, cihazdaki kitap silinmez.
- `oturum/…` anahtarlı kitaplar (kaydedilmemiş çizimlerin) zaten yalnız o oturumundu; göçü yoktur.

### 6. Bulut

- **Dosya projeleri** (ADR 0031) `.kcad` revizyonlarıdır. Kitap revizyonla, kontrol noktasıyla, kopyayla, geri yüklemeyle ve
  indirmeyle kendiliğinden gider.
- **Veritabanı projeleri** kitabı projenin satırında tutar:
  - Göç 0017, `kentos.project.sheets jsonb`: sözleşmenin JSON'u, şekli denetlenir; kuralları uygulamanındır.
  - Projenin bilgisinde (`ProjectInfo.sheets`) ve `.kcad` görüntüsünde gelir.
  - Oluşturmada verilir (`ProjectCreate.sheets`).
  - `project.changes`'in `ProjectPatch.sheets`'i bütün kitabı değiştirir; `null` kaldırır. Bu bir meta değişikliğidir:
    projenin meta sürümüyle korunur, çakışması meta çakışmasıdır.
  - Kuralları sunucuda denetlenir (`check_sheets`, yer `project.sheets.pictures[i].sha256`).
  - İçe aktarma, dosya projesinden çevirme, kontrol noktasından geri yükleme ve kopya (`kentos.duplicate_project`) kitabı
    taşır.
- **Masaüstünün eşitlemesi** (`kentos-cloud`):
  - Kitap projenin meta bilgisidir: değişince bütünü gider, kaldırılınca `null`.
  - Başkasının kitabı bilgiyle gelir.
  - Cihaz taslağı ve yerel kopya onu tutar (`BaseMeta.sheets`; eski kopyada yoktur).
- Web'in istemcisi kitabı göndermez; `ProjectPatch`'te alanı olmadığı için sunucudakini korur (§0.2).

### 7. `.kpafta` kalır

`.kpafta` şablon ve alışveriş dosyası olarak kalır (dışa ve içe aktarma). Projenin kaydı için ek dosya gerekmez (`CAD-SHEET-04`).

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/sheets.rs` (`ProjectSheets`, `SheetPicture`, `sheets_problem`, `SheetsFault`),
  `encoding.rs`, `document.rs`'in `sheets`'i, `cloud.rs`'in `ProjectCreate`, `ProjectInfo` ve `ProjectPatch` alanları; üretilen TS
  tipleri.
- **Kodek:** `crates/shared/kcad/src/encode/sheets.rs`, `decode/sheets.rs`, `SCHEMA_WITH_SHEETS`; `docs/specs/kcad-v2.md` §6.1, §6.2,
  §6.11, §6.12; `tools/kcad/kcad.py`; `scripts/fixtures/kcad_v2_reference.py`.
- **Pafta çekirdeği:** `crates/shared/sheet/src/project.rs`.
- **Belge:** `crates/native/domain`'in `sheets`'i, `set_sheets`, `ExternalMeta.sheets`.
- **Tasarımcı:** `crates/sheet-ui/src/designer.rs` (`attach(store)`, `open_project`, `book_revision`, `project_sheets`), `store.rs`
  (`retire_book`; `KeyChange` kalktı).
- **Masaüstü:** `apps/desktop/src/sheet_book.rs`; `sheets.rs`'in `follow_sheets`'i.
- **Sunucu:** `crates/server/postgres/migrations/0017_project_sheets.sql`, `projects.rs` (`check_sheets`, bilgi, oluşturma),
  `changes.rs`, `snapshot.rs` (`DrawingMeta.sheets`), `importing.rs`.
- **Bulut istemcisi:** `crates/native/cloud/src/sync.rs` (`Meta.sheets`), `sync/draft.rs`, `sync/remote.rs`, `sync/base.rs`,
  `replica.rs`, `open.rs`, `saving.rs`.

## Doğrulama

- **Dosya biçimi, bağımsız başvuruyla:**
  - `scripts/fixtures/kcad_v2_reference.py --check` (624 dosya) örneği ve dokuz bozuk dosyayı KentOS kodu olmadan yazar.
    `tools/kcad/kcad.py` her birini `expected.json`'un dediği gibi okur:
    - şema 43'te `sheets` `unknown_field`,
    - harita olmayan kitap `wrong_type`,
    - eksik kitap `missing_field`,
    - bilinmeyen alan,
    - boş resim listesi, uymayan özet, iki kez verilen resim ve 31 baytlık özet `bad_value`,
    - `schema-version-45.kcad` `schema_version`.
  - Örneğin kitabı pafta çekirdeğinin kendi örneğidir (`fixtures/sheet/v1/kpafta/valid/buro-paftasi.kpafta`). Çekirdeğin
    `project` testi ikisini karşılaştırır.
- **Rust kodeği** (`crates/shared/kcad/tests/all/sheets.rs`): yalnız kitaplı proje şema 44, bayt bayt geri yazma; 300 000 baytlık
  resim; opak kısımda tam sayı ile float ayrı; yazıcının retleri ve okuyucunun hata yerleri. `fixtures.rs` 50 geçerli dosyayı geçer.
- **Belge:** `set_sheets` düzenlemedir, geri alma adımı değildir; çizimin Geri al'ı kitabı geri almaz; geçersiz kitap reddedilir;
  görüntü ve okuma kopyası taşır (`tests/document.rs`).
- **Masaüstü** (`sheet_book_tests.rs`, dört test):
  - dosyadan açma, paftanın adının değişmesiyle çizimin kirlenmesi (geri alma adımı olmadan), tasarımcının Geri al ve Yinele'si,
    kaydedip başka pencerede açma, kitapsız çizimde yeni paftanın kitabı açması;
  - cihazdaki kitabın göçü: çizim kirli, satır söylenir, kaydedilmeden cihazdaki kalır, kaydedilince `tasindi/`'ye ayrılır, yeniden
    açılınca bir şey söylenmez;
  - kitaplı projede cihazdaki farklı kopyanın ayrılması ve söylenmesi;
  - okunamayan kitabın olduğu gibi kalması.
- **Bulut eşitlemesi** (`kentos-cloud`, `the_sheet_book_goes_and_comes_as_the_projects_metadata`): kitap meta sürümüyle bütün gider
  (tel üzerinde örneğin JSON'u), kaldırılınca `null` gider, başkasınınki bilgiyle gelir ve geri gönderilmez.
- **Sunucu, gerçek PostGIS'te** (`tests/project_sheets.rs`, `KENTOS_TEST_DB=required`):
  - oluşturma, bilgi ve `.kcad` görüntüsü;
  - kitabı korumayan değişiklik, değiştirme ve kaldırma; meta sürümü;
  - ret ve yeri;
  - kopya ve kontrol noktasından geri yükleme.
  - `kentos-application`, `kentos-postgres` ve `kentos-api`'nin bütün veritabanı testleri (göç 0017 ve yeni kopya işleviyle) geçici
    bir PostGIS 18 sunucusunda geçti, hiçbiri atlanmadı.
- **Python** (`test_document.py`): `kentos.cad.Document` örneği açar, nokta ekler, kaydeder; bağımsız okuyucu kitabı bit bit aynı
  okur.
- **Takımlar** (13 Ekim, Apple M5):
  - `pnpm rust:test` 3 404 test, clippy ve bağımlılık yönü;
  - `pnpm rust:test:desktop` 1 432;
  - `pnpm py:test` 70.
- **Resimler:** `cargo test -p kentos-desktop sheet_book::tests::screens -- --ignored --nocapture`, `.run/shots/pafta-kitabi-*.png`:
  - `dosyadan`: dosyadan açılan pafta,
  - `goc`: cihazdan alınan kitabın satırı ve kaydedilmemiş işaret,
  - `okunamayan`: daha yeni sürümün kitabının uyarısı.

## Sonuçlar

- Paftalar projenin dosyasıyla gider: başka bilgisayar, başka kullanıcı, kontrol noktası ve kopya onları görür.
- Python ve MCP'nin başsız belgesi kitabı açar ve kaydeder; okuyan ve yazan komutlar `CAD-SHEET-06`'dadır.
- Açık kalanlar:
  - Pafta işlemlerinin ürün kataloğu, başsız sunucu ve Python yüzü (`CAD-SHEET-06`, `07`).
  - Modeli ve kâğıdı birlikte yazan betik işlemi (`CAD-SHEET-05`'in kalanı).
  - Veritabanı projesinde kitabın parça parça (resimleri ayrı) gönderilmesi: şimdi bütün kitap gider.
  - Web'in kitabı (TODOS.md §0.2).
