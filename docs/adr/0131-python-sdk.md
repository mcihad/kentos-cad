# ADR 0131: Python SDK'sı (`kentos.cad`)

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0130 (başsız komut sunucusu), ADR 0013 ve 0022 (ürün komutları), ADR 0007 ve 0040 (kimlik, masaüstünün bulut istemcisi), ADR 0014 (kalıcı kimlik); TODOS.md §14 (PY-), §15 (AI-02), §22 F6.

## Bağlam

Sahibin isteği: "komut sistemi ile yapılan her şey için python sarmalayıcıları yazacaksın, buna tipler de dahil her şey için, ve kentos.cad namespace içinde olacak". F6'nın sırası çekirdek → Python → MCP'dir. Sahip PyO3 ve maturin'i 28 Eylül'de onayladı.

Katalogda 33 komut var:

- 12'si çizimin komutudur (`cad.*`). Web ve masaüstü çalıştırır; başsız sunucu (ADR 0130) da.
- 21'i sunucunun komutudur (`project.*`). Sunucu, hesabın yetkisini denetleyerek çalıştırır.

## Karar

**Paket `kentos`** (`python/`), maturin ile kurulur. İki katmanı vardır:

- Yerel modül `kentos._native` (`crates/native/python`, PyO3 0.29). CPython 3.10 ve sonrası için kararlı ABI'yla derlenir; platform başına tek tekerlek.
- Tipli katman `kentos.cad`. Saf Python'dur, yalnız standart kitaplığı kullanır.

**Yerel modül incedir.**

- Yalnız `kentos-headless`'in `Session`'ını ve `catalog()`'unu açar. Sınırı JSON metni geçer; kataloğun, web'in ve sunucunun tel biçimi budur.
- Rust çalışırken GIL bırakılır. Aynı çizime o sırada gelen ikinci çağrıyı PyO3'ün ödünç denetimi reddeder; çağrı kuyruğa alınmaz.
- Yerel sunucunun reddi `HostError(code, message)` olarak gelir.

**Tipler katalogdan üretilir** (`scripts/python/sdk.py`; `--check` farkı arar). `kentos.cad.types` bütün girdi, çıktı ve plan tiplerini ve parçalarını tutar: 24 enum, 6 etiketli birleşim, 92 yapı, 31 varyant sınıfı.

- **Enum.** `str` enum'dur; adlarının `Literal`'i de üretilir (`LineType | LineTypeName`). Teldeki adıyla yazılır (`str(ProjectRole.OWNER) == "owner"`). Tanınmayan bir ad (daha yeni bir sunucu) düz metin kalır.
- **Yapı.** `dataclass(kw_only=True, slots=True)`'dur; alanlar snake_case, JSON camelCase. Her biri `to_json` ile yazılır, `from_json` ile okunur. Okunup yeniden yazılan JSON aynıdır.
- **İsteğe bağlı alan.** Gönderilmeyen alan `UNSET`'tir. Tek üyeli bir enum'dur, böylece tip denetleyicisi `x is not UNSET` ile daraltır. Telde olmayan alan `UNSET` okunur, null `None` okunur.
  - Bu ayrım `cad.entities.set` için şarttır: orada null "kaldır" demektir (`color=None` nesnenin kendi rengini kaldırır).
- **Birleşim.** Bir taban sınıf ve her varyant için bir sınıf üretilir: `Transform` → `MoveTransform`, `RotateTransform` …; `Entity` → `PointEntity`, `PolylineEntity(PathEntity)`, `PolygonEntity(PathEntity)` …
  - Varyantın adı `kind` özelliğindedir.
  - Taban, bütün varyantların ortak alanlarını tip denetleyicisine bildirir. Bu yüzden herhangi bir `Entity`'de `entity.layer_id` okunur.
- **Nokta.** `Vec2` bir `NamedTuple`'dır. Sarmalayıcılar noktayı çift olarak da alır (`(x, y)`).
- **Plan şeması.** Katalog artık uygulamaların çalıştırdığı her komut için `plan` şemasını da taşır (`CommandDescriptor.plan`; AI-02). Sunucunun komutları planlamaz; onlarda bu alan yoktur. Böylece plan sonuçları da tiplidir.

**Her komut kendi adıyladır.**

- `cad.polygon.create` → `kentos.cad.polygon.create`; `project.checkpoint.create` → `kentos.cad.project.checkpoint.create`.
- `project.import` → `project.import_`, çünkü `import` Python'da anahtar sözcüktür.
- `COMMANDS` bütün sarmalayıcıları kimliğe göre tutar; `catalog()` kataloğun kendisini, şemaları ve örnekleriyle verir.
- **Çizim komutu.** `create(doc, /, *, layer_id=…, pts=…)` çalıştırır ve çıktı tipini döner; her çağrı tek geri alma adımıdır.
  - `.plan(…)` yazılacak olanı gösterir, `.validate(…)` uyarıları döner.
  - `.run(doc, girdi, op=…)` bütün cevabı (`Outcome`) reddi yükseltmeden verir.
- **Sunucu komutu.** Biçimi `rename(project, /, *, name=…, expected_versions=…, idempotency_key=…)`'dır.
  - Hedef `Connection.project(…)`'tir; `project.create` için `Connection.tenant(…)`.
- İlk parametre konum-yalnızdır; böylece girdinin alanlarıyla çakışmaz (`project.changes`'in bir `project` alanı vardır).

**Hatalar üç ailedir; hepsi `KentosError`'dur.**

| Aile | Sınıflar | Ne zaman |
|---|---|---|
| `CommandError` | `CommandFailed`, `NeedsInput`, `RevisionConflict`, `CommandCancelled` | Komutun kendi reddi; kod, ileti, alan yolu ve revizyonla. Hiçbir şey yazılmamıştır. |
| `HostError` | `InvalidInput`, `UnknownObject`, `FileError`, `NotRunHere`, `Busy`, `UnknownSystem` | Yerel sunucu isteneni yapamadı. |
| `ServerError` | `NotSignedIn` 401, `Forbidden` 403, `NotFound` 404, `ServerConflict` 409, `Gone` 410 | Sunucunun `ApiError`'u; `retryable` ve `retry_after` ile. |

- Ayrıca: `Unreachable` (sunucudan yanıt gelmedi), `DecodeError` (paket, cevabı verenden eski).
- Uyarılar `warnings` modülüne `CommandWarning` olarak gider ve çağıranın satırını gösterir.
- NaN ve sonsuz JSON'da taşınmaz. SDK onları komuta göndermeden `InvalidInput("not_finite")` ile reddeder.

**`Document`** (`kentos.cad.Document`) masaüstünün kendi belgesidir.

- **Açma:** `new(name, srid=…)`. Koordinat sistemi sorulur, tahmin edilmez (CLAUDE.md §5). `open` ve `from_bytes` da açar.
- **Kayıt:** `save` KCAD v2 yazar, baytları doğrulayarak; eski v1 dosyasının üzerine yazmaz. `to_bytes` baytları verir.
- **Okuma:** `info`; `layers`, `all_layers`, `layer`; `page` ve `entities` (sayfa sayfa); `entity`; `measure`.
- **Geçmiş:** `undo`, `redo`. `group(label)` bir betiği tek adım yapar; hata hepsini geri alır.
- **Genel yol:** `run(id, girdi)`. Kimliği, sürümü ve girdinin tipini yerel sunucu katalogla denetler (PY-11).

**`Connection`** (`kentos.cad.Connection`) standart kitaplığın `urllib`'iyle konuşur.

- **Oturum:** `login` yerel hesapla oturum açar. Oturum çerezi her istekle `x-kentos-client: python` başlığıyla gider. `logout` kapatır, `me` hesabı verir.
- **Proje:** `projects(view, …)` katalogdan bir sayfa verir. `project(…)` ve `tenant(…)` komutların hedefini kurar. `ProjectRef.snapshot()` veritabanı projesinin `.kcad`'ini indirir; `Document.from_bytes` onu açar.
- **Komut:** sunucunun zarfıyla gider. Her çağrı yeni bir idempotency anahtarı alır; çağıran kendi anahtarını da verebilir.
- **Sunucu tarafı:** sunucu `x-kentos-client` başlığında `python`'u da tanır (`CLIENTS`). Kural aynıdır: özel başlık siteler arası formu durdurur (ADR 0007).

**Sınır.** SDK yetki kararı vermez; her komutu sunucu denetler. Çizim komutları sunucuda çalışmaz (CLAUDE.md §21.3).

## Sonuçlar

- 33 komutun hepsi tiplidir (ADR 0144'ün blok komutlarıyla 35). Katalog değişip Python yeniden üretilmezse `sdk.py --check` ve kapsama testi düşer (PY-16).
- **Masaüstüne gömülü Python sonraki dilimdir** (PY-01..05): konsol ve betik paneli. Aynı `kentos.cad` o zaman açık çizimin `Document`'iyle çalışacak. MCP de aynı çekirdeği kullanacak.
- **Açık kalanlar:**
  - Web'de Python yok (Pyodide, PY-18..22).
  - Dosya projesine revizyon (`project.file.commit`) önce yükleme ister; SDK'da yükleme yardımcısı yok.
  - `queued` cevabın `Outcome.job_id`'si var, işi izleyen bir API yok.
  - Sayılar float'tır; kesin ondalık (CLAUDE.md §23) tipli özniteliklerle gelecek.
- **Sunucuda bir değişiklik var:** `CLIENTS`'e `python` eklendi.

## Doğrulama

**`pnpm py:test`** (`scripts/python/test.sh`) üreticiyi denetler, maturin'le derler ve 27 testi koşar:

- **Kabul akışı (TODOS.md §14):** yeni proje → plan → kapalı alan → ölçü (250 m², çevre 65 m) → katman değiştirme → kayıt → yeniden açma: aynı kimlik, aynı ölçü.
  - Ayrıca çakışan plan, grup, redler, gizli katmana uyarı, kilitli katman, eski dosya, sayfalar; her tür kendi sınıfıyla okunur.
- **Katalog:** her komutun sarmalayıcısı kendi adında, sürümü ve tipleriyle; kataloğun her tipi burada. Kataloğun bütün örnekleri tiplere okunur ve aynı JSON'a yazılır.
- **Ortak durumlar:** `fixtures/commands/v1`'in 328 durumu tipli katmandan geçer; web ve masaüstünün koştuğu aynı dosyalardır.
  - Girdi tipe okunur, sarmalayıcıyla gönderilir; cevap tiplere okunur, yeniden yazılır ve durumla karşılaştırılır.
  - NaN'lı 81 adımı SDK `not_finite` ile reddeder ve çizim değişmez.
- **Bağlantı:** yerine konan bir sunucuyla zarf, çerez, başlık, hatalar ve katalog sayfası sınanır.

**`scripts/python/live.py`** gerçek kentosd'yi geçici bir veritabanında çalıştırır; `kentos_cad`'e dokunulmaz.

- Python'da çizilen kapalı alan projeye dönüşür ve `project.changes` ile gider.
- Ad, favori, açıklama ve kontrol noktası yazılır.
- Sunucunun redleri beklenen kodlarla gelir: arşivde 409 `project_archived`, paylaşılmamış hesaba 404, görüntüleyiciye 403, erişim geri alınınca 404, çöpte 410 `project_deleted`.
- Projenin görüntüsü açılır: aynı kimlikle 250 m².

**Diğer denetimler:**

- `mypy --strict` (2.3.1, yerel ortamda; projenin bağımlılığı değil): paket ve testler temiz.
- `cargo clippy -p kentos-python --all-targets -- -D warnings`.
- `cargo test -p kentos-contracts`: plan şemaları; bir uygulamanın çalıştırdığı her komutun planı var, yalnız sunucunun çalıştırdığının yok.
- `cargo test -p kentos-api client_tests`.
- `node scripts/arch/deps.mjs`: `python` grubu, pyo3'ü kullanan tek native crate.
