# KentOS — Python eklentileri ve analitik ekosistem

İnceleme tarihi: **12 Ekim 2026**. Bu belge [CAD geliştirme planındaki](cad.md) ortak komut, Python ve AI tamlığı şartını Python/uv ortamlarına, KentOS arayüz eklentilerine, Processing araçlarına ve onaylı `.kek` dağıtımına genişletir.

**Form sistemi ayrı plan:** Ortak CBS form standardı, tasarımcı, domain/subtype/ilişki/join, merkezi yayın/izinler ve mobil saha akışı [form.md](form.md) içinde ayrıntılıdır. Bütün `FORM-*` görevlerinin tek kaynak planı o belgedir.

**Ürün sırası:** Form sistemi ilk eklenecek özellik olarak önceliklendirilebilir. Form planının core/central dilimleri Python/uv, `.kek` manager veya eklenti bulut kataloğunu beklemez. Bu belge form çekirdeğinin üzerine isteğe bağlı Python builder, UI/Processing action ve dağıtım entegrasyonunu ekler; form kullanıcılarını Python kurmaya zorlamaz.

**Kullanıcı kararı:** Eklenti arayüzleri **KentOS'un kendi arayüz bileşenlerini** kullanacak. Python eklentileri için ayrı bir Qt/PySide arayüz dünyası kurulması bu planın hedefi değildir.

**Bu teslim bir araştırma ve uygulama planıdır.** Uygulama kodu değiştirilmemiş, uv/Python/paket/eklenti kurulmamış ve herhangi bir bulut yayını yapılmamıştır. İnceleme kaynak okumaya ve resmi belgelere dayanır; hedeflenen davranışların bu oturumda test edildiği iddia edilmez. Örnek şemalar önerilen v1 standardıdır, bugün çalışan API değildir.

## 1. Kesin ürün şartları

- Python özelliği, uv kullanımı, interpreter indirme, analitik paket kurulumu ve eklenti etkinleştirme kullanıcıya bağlı ve seçilebilir olmalıdır.
- KentOS'un temel CAD/GIS ve hazır Rust araçları, Python veya uv kurulmadığı için kullanılamaz hale gelmemelidir.
- uv bulunamazsa işletim sistemine uygun bir kurulum akışı sunulmalı; kurulum yöntemi ve değişiklikler kullanıcı tarafından seçilmelidir.
- Kullanıcı NumPy gibi ihtiyaç duyduğu analitik paketi uygulama içinden seçip kendi ortamına kurabilmelidir; bütün analitik paketler varsayılan gelmemelidir.
- Eklenti geliştiricisi gereken Python paketlerini açıkça tanımlamalıdır. Başka kullanıcı kuruluma geçmeden doğrudan/geçişli bağımlılıkları, Python gereksinimini ve izinleri görmelidir.
- Gerekli bağımlılıklar kurulmadıysa eklenti “hazır” sayılmamalı; eksik gereksinim nedeniyle kullanılamayan iş açıkça açıklanmalıdır.
- Eklentiler KentOS UI ile form, dialog, dock/panel ve uygulama komutları oluşturabilmeli; Processing araçları da geliştirebilmelidir.
- Processing araçları parametre, kaynak katman/nesne türü, kapsam, çıktının türü ve hedefi, ilerleme/iptal ve doğrulama için ortak sözleşme kullanmalıdır.
- Hazır Processing araçları eklentiden kullanılabilmeli; aynı araç UI, komut, model, Python ve AI üzerinden çağrılabilmelidir.
- Eklenti yerel kalabilir veya isteğe bağlı olarak buluta gönderilebilir. **Bulutta yayınlanacak her sürüm, yayınlanmadan önce onaydan geçmelidir.**
- Taşıma biçimi `.kek` uzantılı sıkıştırılmış paket olmalıdır; kaynak, UI/form/tool tanımları, dependency metadata ve dağıtım bilgileri birlikte taşınmalıdır.
- Python/AI bütün eklenti, UI ve Processing lifecycle işlemlerine aynı komut yüzeyinden erişmelidir; kurulum, izin, doğrulama veya yayın onayını atlayan yol olmamalıdır.
- Eklenti portable CBS form kaynağı sunabilir; binding/rule/deployment/veri güvenliği [form standardına](form.md) uymalıdır. Form kullanımı ile Python kodu çalıştırma ayrı capability olmalıdır.
- Temel CBS formu Python/uv ve `.kek` olmadan çalışır. İsteğe bağlı Python-dependent action gereken executor/dependency/platform sınırını açık bildirir.

## 2. Mevcut uygulamada bulunan temel ve doğrulanan boşluklar

### 2.1 Kaynak kanıtları

| Kanıt | Kaynak / anlamı |
|---|---|
| E01 | [Python SDK](../python/kentos/cad/document.py), [paket tanımı](../python/pyproject.toml), [SDK üreticisi](../scripts/python/sdk.py): tipli `kentos.cad`, Document/Connection, katalogdan wrapper üretimi |
| E02 | [Python konsolu ADR](../docs/adr/0132-desktop-python-console.md), [masaüstü Python host](../apps/desktop/src/python/host.rs): ayrı süreçte çalışma, çıktı/hata, stop ve undo grubu |
| E03 | [Python UI ADR](../docs/adr/0150-python-surfaces.md), [AI Surface ADR](../docs/adr/0154-ai-surfaces.md): mevcut KentOS UI Python/AI bileşenleri |
| E04 | [Ürün kataloğu](../crates/shared/contracts/src/catalog.rs), [headless dispatcher](../crates/native/headless/src/dispatch.rs), [MCP](../crates/native/mcp/src/tools.rs): ortak otomasyon yolu |
| E05 | [Web Processing tipi](../apps/web/src/processing/types.ts), [registry](../apps/web/src/processing/registry.ts), [runner](../apps/web/src/processing/runner.ts): parametre, tool, model, sonuç ve kayıt yaşam döngüsü |
| E06 | [Native Processing](../crates/native/processing/src/lib.rs), [tool tipleri](../crates/native/processing/src/types.rs), [registry](../crates/native/processing/src/registry.rs), [runner](../crates/native/processing/src/runner.rs): salt okunur girdi → changeset → tek undo |
| E07 | [Processing parametreleri](../crates/native/processing/src/parameters.rs), [web parametreleri](../apps/web/src/processing/parameters.ts), [Processing ADR](../docs/adr/0084-desktop-processing.md): doğrulama/default/görünürlük ve platform uyumu |
| E08 | [Alan sözleşmesi](../crates/shared/contracts/src/fields.rs), [katman alanları ADR](../docs/adr/0199-layer-fields-and-feature-table.md): tür, aralık, seçim listesi, zorunluluk ve otomatik alan düzenleme |
| E09 | [Sorgu/join ADR](../docs/adr/0200-spatial-and-attribute-queries.md): mekânsal sorgu ve anahtar üzerinden değer aktarma; kalıcı 1:N/N:M ilişki formu ayrı gelecek kapsam |
| E10 | [KentOS UI Form](../crates/ui/src/widget/form.rs), [widget modülü](../crates/ui/src/widget/mod.rs), [web alan düzenleyici](../apps/web/src/ui/properties/fieldRows.ts): mevcut form yapı taşları |
| E11 | [Proje yetki ADR](../docs/adr/0015-project-ownership-and-access.md), [gerçek yetki değerlendirme](../crates/server/application/src/access.rs): roller, paylaşım, sunucu ve satır güvenliği temeli |
| E12 | [Proje snapshot](../crates/shared/contracts/src/document.rs), [kalıcı kimlik ADR](../docs/adr/0014-persistent-entity-identity.md): yerel/bulut kayıt ve UUID temeli |
| E13 | [Öznitelik ekleri ADR](../docs/adr/0225-object-attachments.md), [Attachment sözleşmesi](../crates/shared/contracts/src/attachment.rs): fotoğraf/belge ekleri için mevcut kaynak modeli |
| E14 | [İfade çekirdeği](../crates/shared/expression/src/lib.rs), [ifade ADR](../docs/adr/0100-expression-engine.md): ortak hesap/ifade altyapısı; form profiline sınırlandırılacak |
| E15 | [Yol haritası](../TODOS.md), [CAD planı](cad.md): Python ortam, AI kapsam, ilişkiler ve platform hedefleri |

### 2.2 Mevcut / kısmi / yeni iş ayrımı

| Alan | Gözlenen durum | Planın tamamlayacağı şey |
|---|---|---|
| Python SDK/REPL/editör | Mevcut [E01–E03] | Ortam seçimi, uv adapter, eklenti runtime ve tam ürün erişimi |
| Headless/MCP/komut | Mevcut temel [E04] | Plugin/form/tool lifecycle ve yüksek seviyeli işlemler |
| ProcessingTool | Mevcut TS ve Rust modelleri [E05–E07] | Sürümlü, dışarı taşınabilir descriptor + Python executor |
| Web tool registration | `register` ile ekleme/çıkarma yolu var [E05] | Gerçek Python eklenti manager ve unload/izin/runtime bağları |
| Native tool executor | `run/validate/preview` alanlarında Rust function pointer var [E06] | Python çağırabilen executor adapter; veri descriptor'ıyla kodun ayrılması |
| Processing scope | Yerel selection/visible/layer/IDs ve snapshot yolu var [E05–E06] | Dış API'de kalıcı UUID + snapshot; local slot ID sızdırmama |
| Parametre kuralları | Default/görünürlük/validation var [E07] | Callable callback yerine taşınabilir expression/action sözleşmesi |
| Processing çıktısı | Layer değişiklikleri, sayı/metin/tablo var [E05–E07] | Dosya/asset/job ve modeller arası typed output akışı; mevcut tablo/file sınırlamalarının kapanması |
| uv/.kek/eklenti bulut incelemesi | İncelenen ürün yollarında tam sistem bulunmadı | Yeni opt-in ürün akışı, paket standardı ve onaylı katalog |

**Kapsam sınırı:** Veri şeması, form tasarımı, domain/ilişki, merkezi ACL ve mobil runtime için ayrıntılı envanter ve QGIS/ArcGIS kıyası [form.md](form.md) içindedir. Yukarıdaki ortak kaynaklar plugin broker, binding ve form action entegrasyonunda da kullanılır; bu belge ikinci bir form uygulaması tanımlamaz.

## 3. Ortak mimari, form entegrasyonu ve değişmez kurallar

- **Form sözleşmesi:** [form.md](form.md) içindeki core/central ve ileri profiller authoritative form planıdır; portable form resources bağımsız çalışır, Python action ayrı capability'dir.
- **Veri tanımı:** DatasetSchema, DomainDefinition, SubtypeDefinition, RelationDefinition ve SourceDefinition.
- **Sunum tanımı:** FormDefinition ve KentOS UiDefinition; veri şeması veya ACL'nin yerine geçmez.
- **İş tanımı:** ProcessingToolDescriptor, ActionDescriptor, CommandDescriptor ve JobDescriptor.
- **Dağıtım tanımı:** ExtensionManifest, `.kek` içerik manifesti, dependency lock ve yayın inceleme kaydı.
- **Oturum/veri girişi:** RuntimeContext, FormSession, SubmissionEnvelope, snapshot ve changeset.
- **Otorite:** Yerelde mevcut native/web domain; bulutta mevcut yetkili application/service katmanı. Her mutation ortak komut/transaction üzerinden geçer.
- **UI:** KentOS desktop bileşenleri ve web/mobil karşılıkları aynı widget/action anlamını uygular; Python doğrudan Iced/Rust pointer'ı veya DOM elemanı almaz.
- **Python:** Ayrı plugin süreçleri + tipli IPC/broker; uzun hesap UI thread'de çalışmaz. Aynı süreçte bütün kullanıcı eklentilerini import etmek varsayılan olmamalıdır.
- **Taşınabilir kural:** Ortak expression profili/AST ile değerlendirilir. Zorunlu form doğruluğu yalnız masaüstünde çalışan Python callback'e bağlanmaz.
- **İzin:** Bir widget'ı gizlemek veya `editable=false` yapmak güvenlik sınırı değildir. Veri okuma/yazma ve schema/form tasarımı sunucuda ayrıca denetlenir.
- **Opt-in:** Proje açma, `.kek` dosyasını görme, cloud catalog'a bakma veya AI önerisi; interpreter/paket kurma, plugin import veya kod yürütme izni sayılmaz.
- **Tamlık:** UI, komut, Python ve AI aynı lifecycle/parameter/query/validation/result yüzeylerine erişir; ayrı API'de farklı iş kuralları oluşmaz.

## 4. PY-OPT — Kullanıcı seçimi ve kişisel Python deneyimi — P0/P1

- [ ] **PY-OPT-01:** Kullanıcı tercihlerine “Python ve eklentiler” alanı ekle; kapalı kullanıcıya interpreter/uv/package startup maliyeti yükleme.
- [ ] **PY-OPT-02:** Etkinleştirme sihirbazı; uv algılama, Python seçimi, çalışma ortamı ve paket kurulumu adımlarını ayrı sunsun.
- [ ] **PY-OPT-03:** “uv var / bulunamadı / sürümü uyumsuz / çalıştırılamıyor” durumlarını açık capability olarak bildir.
- [ ] **PY-OPT-04:** Kullanıcı ortamı kişisel olsun; aynı cihazdaki farklı hesapların paket/eklenti/secret alanları birbirine karışmasın.
- [ ] **PY-OPT-05:** Kullanıcı paket arama, sürüm seçme, ekleme/kaldırma/güncelleme, dependency tree ve cache boyutunu uygulama içinden yönetebilsin.
- [ ] **PY-OPT-06:** NumPy için örnek akış kur: ihtiyacı seç → kurulacak paketleri gör → kendi analitik ortamına kur → konsolda kullan → script/eklenti bağımlılığına ekle.
- [ ] **PY-OPT-07:** Paket seçimi yayımlanmış plugin ortamını değiştirmesin; kişisel analitik çalışma ortamı ile plugin lock ortamı ayrılmalı.
- [ ] **PY-OPT-08:** Kurulum ekranında gerekli/isteğe bağlı paketler ve hangi özelliğin hangi paketi istediği görünür olsun.
- [ ] **PY-OPT-09:** “Yalnız form görüntüle”, “Analizi çalıştır”, “Geliştirici modu” farklı gereksinimler taşıyabilsin; kullanılmayan optional extra zorunlu kurulmasın.
- [ ] **PY-OPT-10:** Reddedilen dependency kurulumu plugin'i pasif/blocked durumda bırakmalı; her açılışta otomatik indirme veya tekrar onay baskısı olmamalı.
- [ ] **PY-OPT-11:** Paket/izin/sürüm seçimlerini user scope'ta hatırla; bir kullanıcının kararı başkasının hesabına aktarılmasın.
- [ ] **PY-OPT-12:** Kurum politikası dış index veya cloud execution sınırı koyabilir; kullanıcı onayı kurum politikasını genişletemesin.
- [ ] **PY-OPT-13:** Ortam/eklenti kaldırma; dosya ve çalışma çıktısı silme kapsamını açık göstersin; kişisel scriptler varsayılan temizliğe dahil olmasın.
- [ ] **PY-OPT-14:** İlk kullanıcının hazır araçlarla uygulamaya devam edebilmesi bütün kurulum akışlarında korunsun.

**Kabul:** uv/Python istemeyen kullanıcı normal CAD/GIS işlerini yapar. NumPy isteyen kullanıcı kendi ortamına seçerek kurar; hiçbir başka kullanıcının/plugin'in ortamı değişmez.

## 5. PY-UV — uv entegrasyonu, kurulum ve environment manager — P0/P1

uv'nin resmi belgeleri Windows/macOS/Linux için kurulum seçenekleri, environment ve lock yönetimi sunuyor. Önemli ayrım: bazı uv komutları ortamı/lock'u otomatik senkronize eder ve gerekirse Python indirebilir. KentOS adapter'ı bunu kullanıcının seçimine bağlamalıdır. [uv kurulum](https://docs.astral.sh/uv/getting-started/installation/), [Python sürümleri](https://docs.astral.sh/uv/concepts/python-versions/), [locking/sync](https://docs.astral.sh/uv/concepts/projects/sync/)

### 5.1 Algılama ve kurulum

- [ ] **PY-UV-01:** PATH ve kullanıcı tarafından seçilen executable yolundan uv'yi keşfet; sürüm/platform/arch bilgisini okuyup destek matrisiyle karşılaştır.
- [ ] **PY-UV-02:** GUI uygulamasının shell PATH'inden farklı başlayabileceğini hesaba kat; manuel uv yolu ve “yeniden algıla” sun.
- [ ] **PY-UV-03:** uv yoksa işletim sistemine göre resmi kurulum yöntemlerini sun: macOS package manager/standalone, Windows package manager/standalone, Linux uygun resmi binary/installer.
- [ ] **PY-UV-04:** Kurulum yöntemi, hedef konum, indirme ve PATH/izin değişikliğini kullanıcı seçsin; uygulama onaysız sistem kurulumu başlatmasın.
- [ ] **PY-UV-05:** Kullanıcı installer'ı kendisi çalıştırabilir veya açıkça uygulamadan kurulum isteyebilir; komut ve kaynağı önceden inceleyebilsin.
- [ ] **PY-UV-06:** Yönetici izni gerekmeyen kullanıcı-local yolu tercih edilebilir olsun; sistem package manager yöntemi seçilirse onun kuralları uygulansın.
- [ ] **PY-UV-07:** Kurulum/güncelleme cancel, proxy, download failure, checksum, PATH görünmemesi ve yeniden deneme durumlarını düzgün sonlandırsın.
- [ ] **PY-UV-08:** Mevcut uv'yi otomatik değiştirme; desteklenmeyen sürümde neden ve seçilebilir upgrade/manual path yolu sun.
- [ ] **PY-UV-09:** Platform/architecture ve uv/Python minimum sürümlerini sabit bir compatibility manifestinden keşfet; uygulama koduna dağılmış versiyon kontrolü olmasın.

### 5.2 Ortam modeli ve dependency çözümleme

- [ ] **PY-UV-10:** Üç ortam türünü ayır: kişisel analitik workspace, plugin geliştirme ortamı, immutable release runtime ortamı.
- [ ] **PY-UV-11:** Varsayılan plugin runtime isolation anahtarı; user, plugin ID/version, lock digest, Python ABI, OS/arch ve seçilmiş extras olsun.
- [ ] **PY-UV-12:** Paket cache'i paylaşılabilir olsa da mutating venv paylaşılmasın; iki eklentinin çelişen NumPy sürümleri aynı interpreter namespace'ine girmesin.
- [ ] **PY-UV-13:** `pyproject.toml` dependency gereksiniminin kaynağı, `uv.lock` resolved sürümlerin kaynağı olsun; manifestte üçüncü çelişkili paket listesi elle tutulmasın.
- [ ] **PY-UV-14:** Dependency preview; direct/transitive, version, extra, Python/platform marker, index, license ve mümkünse wheel/download boyutunu göstersin.
- [ ] **PY-UV-15:** Runtime hazırlama ve çalıştırmayı ayır: install/resolve adımı kullanıcı kararıyla; execution hazır ortamı kullanarak indirmesiz başlasın.
- [ ] **PY-UV-16:** Python otomatik indirmeyi execution aşamasında kapat; gerekli interpreter ayrı kurulum kararı olarak sunulsun.
- [ ] **PY-UV-17:** Release runtime'da lock güncelleme yapılmasın; manifest/pyproject/lock uyumsuzsa yeniden çözmek yerine invalid-package hatası ver.
- [ ] **PY-UV-18:** Mevcut release venv'ini yerinde bozmak yerine staging ortam oluştur, sync/check/import smoke tamamlanınca atomik aktive et.
- [ ] **PY-UV-19:** Dev bağımlılıklar release kullanımına kurulmasın; editable kurulum yalnız geliştirici modunda olsun.
- [ ] **PY-UV-20:** Paket kurulumunun native wheel, source build ve işletim sistemi kütüphanesi ihtiyaçlarını ayır; başarısız build'i plugin bug gibi göstermesin.
- [ ] **PY-UV-21:** Source build hook'ları kod yürütür; gereken durumda açık kurulum kapsamı ve yönetilen build süreci olsun. Public release için wheel tabanlı dağıtım öncelikli olsun.
- [ ] **PY-UV-22:** `uv` argv ile çağrılsın; paket adı/path/index bilgisini shell string'e birleştirme. Timeout, çıktı limiti ve request/job ID kullan.
- [ ] **PY-UV-23:** System Python'a paket yazma, global `pip` ve `--system` gibi yollar normal install flow'unda kullanılmasın.
- [ ] **PY-UV-24:** Çalıştırılacak environment path, interpreter ve cwd açık seçilsin; çevredeki `.python-version`, uv config veya aktif venv yanlış hedefi belirlemesin.
- [ ] **PY-UV-25:** Custom index/kurum mirror, proxy ve credentials user/tenant kapsamıyla yapılandırılsın; sırlar `.kek`, lock veya script çıktısına yazılmasın.
- [ ] **PY-UV-26:** Index priority ve package-source pin kuralları açık olsun; özel paket adı başka public index'ten sessizce çözülmesin.
- [ ] **PY-UV-27:** Offline cache/wheelhouse desteği için platform-tag uyumu ve hash doğrulaması yap; eksik dosyalar açık raporlansın.
- [ ] **PY-UV-28:** Environment check; metadata, SDK import, gerekli paket version/import, native library ve plugin compatibility durumunu tanılasın.
- [ ] **PY-UV-29:** Secret içermeyen lock/environment/SBOM raporu export edilebilsin; analizin tekrar üretilebilir ortamı kaydedilsin.
- [ ] **PY-UV-30:** uv cache/venv/interpreter temizliğinde kullanıcı sahipliği ve başka aktif iş referansları denetlensin.
- [ ] **PY-UV-31:** Plugin broker SDK'sı host'un doğrulanmış/sürümü eşleşen wheel/facade artifact'inden sağlansın; plugin aynı SDK package adını farklı index'ten gölgeleyemesin. SDK artifact/digest dependency preview ve environment provenance'da görünsün.

**Kabul:** NumPy isteyen A plugin'i ve farklı NumPy gerektiren B plugin'i birlikte kurulabilir; bağımlılıklar karışmaz. İnternet kapalıyken hazır ortam çalışır. Plugin çalıştırma kendiliğinden package/interpreter indirmez.

## 6. PY-KEK — `.kek` paket standardı — P0/P1

### 6.1 Biçim ve kimlik

- `.kek`, **KentOS Eklenti** paketinin uzantısıdır ve ZIP tabanlı veri/kod dağıtım biçimi olacaktır. İşletim sistemi eklentisi değil, KentOS tarafından incelenen ve yönetilen bir uygulama paketidir.
- Dosya yalnız uzantıdan tanınmamalı; ZIP içeriği ve root manifestte `format: kentos.extension` doğrulanmalıdır.
- Source/UI/tool metadata **okumak**, plugin entrypoint'i import etmekten ayrılmalıdır. Kurulum önizlemesi bağımlılık kurmadan ve plugin kodu çalıştırmadan yapılmalıdır.
- Her release immutable plugin ID + sürüm + içerik digest'iyle tanınmalıdır. Aynı sürüm numarasına farklı byte içeriği yayınlanamamalıdır.
- Önerilen MIME adı `application/vnd.kentos.extension+zip` olarak ayrıca değerlendirilmeli; isim tescilli standartmış gibi sunulmamalıdır. İçerik tanıma manifest/format doğrulamasına dayanmalıdır.
- Portable form/template katkısı ile Python GUI/analiz katkısı ayrı seçilebilmelidir. Yalnız portable form import eden kullanıcıya NumPy veya interpreter kurdurulmamalıdır; formun Python action gereksinimi varsa bu ayrı görünmelidir.

Önerilen paket yapısı:

```text
urban-analysis.kek
├── manifest.json
├── pyproject.toml
├── uv.lock
├── src/urban_analysis/
├── tools/normalize.tool.json
├── ui/main.ui.json
├── forms/inspection.form.json
├── assets/
├── docs/
├── tests/
├── LICENSE
└── integrity.json
```

`integrity.json`, içerik dosyalarının hash/size listesini taşır; kendi hash'ini içermez. Bulut onay/signature kaydı nihai paket digest'ine dışarıdan bağlanır. Paket içindeki hash listesi tek başına yayıncı güveni veya inceleme onayı değildir.

### 6.2 Önerilen manifest v1

```json
{
  "format": "kentos.extension",
  "schemaVersion": 1,
  "id": "tr.example.urban-analysis",
  "version": "0.1.0",
  "name": "Kentsel analiz",
  "publisherId": "example-publisher",
  "licenseFile": "LICENSE",
  "compatibility": {
    "sdkProtocol": 1,
    "python": ">=3.10,<3.14",
    "runtimes": ["desktop-python", "portable-form"],
    "operatingSystems": ["windows", "macos", "linux"]
  },
  "dependencies": {
    "projectFile": "pyproject.toml",
    "lockFile": "uv.lock",
    "optionalFeatures": {
      "charts": { "extra": "charts", "label": "Analiz grafikleri" }
    }
  },
  "entrypoints": {
    "processing": [
      {
        "id": "normalize",
        "runtime": "desktop-python",
        "descriptor": "tools/normalize.tool.json",
        "handler": "urban_analysis.tools:normalize"
      }
    ],
    "ui": [{ "runtime": "desktop-python", "definition": "ui/main.ui.json" }],
    "forms": [{ "runtime": "portable-form", "definition": "forms/inspection.form.json" }]
  },
  "requestedCapabilities": [
    { "id": "datasets.read", "scope": "user-selected" },
    { "id": "datasets.create", "scope": "current-project" },
    { "id": "ui.mount", "scope": "plugin-owned" }
  ]
}
```

SDK/Python aralığı örnektir; gerçek release ortamı lock ve test matrisiyle belirlenir. Bu örneğin Python/OS koşulları `desktop-python` katkılarına uygulanır; `portable-form` kendi form standardı/host capability'siyle değerlendirilir. Katkı bazlı compatibility/dependency/capability çözümleme manifest standardında zorunlu olmalıdır. Manifest capability talebi yetki verilmiş olması değildir. Public katalogda `publisherId` ve namespace sahipliği sunucudan doğrulanır; paket kendi yayıncı kimliğini güvenilir ilan edemez.

### 6.3 Paketleme ve kurulum görevleri

- [ ] **PY-KEK-01:** Manifest/descriptor/UI/form schema'ları Rust contracts kaynağından JSON Schema/TS/Python tiplerine üret.
- [ ] **PY-KEK-02:** Plugin ID namespace, semantic version, compatibility, entrypoint, dependency refs ve kaynak dosyası gerekliliklerini doğrula.
- [ ] **PY-KEK-03:** SDK template/build/check/package komutları aynı validator'ı kullansın; eksik LICENSE/help/icon/lock gibi dağıtım bilgileri raporlansın.
- [ ] **PY-KEK-04:** Deterministik ZIP ordering, timestamp/metadata policy ve integrity listesiyle yeniden üretilebilir paket oluştur.
- [ ] **PY-KEK-05:** Paket `.venv`, interpreter, user secret, local absolute path ve gereksiz build cache taşımamalı.
- [ ] **PY-KEK-06:** Platforma özel runtime wheel/asset eklenirse platform marker, hash, lisans ve dependency preview'da görünür olsun; varsayılan source-only paket sade kalsın.
- [ ] **PY-KEK-07:** ZIP path traversal, absolute path, symlink/hardlink, duplicate entry, case/Unicode çakışması ve archive bomb sınırlarını uygula.
- [ ] **PY-KEK-08:** Dosya/adet/boyut/ratio/derinlik ve decompression bütçesi koy; doğrulama bitmeden live plugin klasörüne extraction yapma.
- [ ] **PY-KEK-09:** Manifest dışında kalan, hash'i tutmayan veya bilinmeyen executable girişleri invalid package say.
- [ ] **PY-KEK-10:** İçe alma akışı; incele → dependency/izin/compatibility göster → kullanıcı seçimi → staging install → hazırla → etkinleştir olsun.
- [ ] **PY-KEK-11:** Entry point'leri environment hazır ve kullanıcı izinli olmadan import etme; description/help okumak kod çalıştırmasın.
- [ ] **PY-KEK-12:** Yerel paket, kurum paketi ve public onaylı paket güven durumu ayrı görünsün; yerel geliştirme için cloud yayın zorunlu olmasın.
- [ ] **PY-KEK-13:** Release update; yeni dependency/izin farkını göster, doğrula ve atomik geç; çalışan iş eski release/env lease'ini bitirene kadar koru.
- [ ] **PY-KEK-14:** Update başarısızsa önceki plugin sürümü hazır kalsın; downgrade/rollback uyumluluğu ve veri migration'ı ayrı denetlensin.
- [ ] **PY-KEK-15:** `.kek` dosya ilişkilendirmesi yalnız KentOS içe alma/inceleme akışını açsın; dosyaya çift tıklama otomatik execution sayılmasın.
- [ ] **PY-KEK-16:** Plugin kaldırma source/env/UI/command kayıtlarını temizlesin; kullanıcı analiz çıktıları ve bağımsız form yayınları silinmesin.
- [ ] **PY-KEK-17:** Contribution bazlı kurulum; portable resource import, Python runtime hazırlama ve action izinlerini ayırsın. Seçilmemiş katkı için dependency/grant hazırlanmasın.

**Kabul:** Bir `.kek` başka cihazda kod import edilmeden önizlenir. Dependency listesi okunur, kullanıcı kabul ederse izole ortam kurulur; bozuk paket hiçbir live dosyayı değiştirmez.

## 7. PY-RUN — Eklenti çalışma modeli, broker ve yaşam döngüsü — P0/P1

- [ ] **PY-RUN-01:** ExtensionManager; package index, install receipt, enabled state, environment, permission grant ve çalışan oturumları kullanıcıya göre yönetsin.
- [ ] **PY-RUN-02:** State machine tanımla: discovered → inspected → dependency-blocked/permission-blocked → preparing → ready → enabled/running → stopped/failed/disabled.
- [ ] **PY-RUN-03:** Install, environment prepare, enable, run, disable, update ve uninstall ayrı işlemler olsun; birinin başarısı öbürünün tamamlandığı anlamına gelmesin.
- [ ] **PY-RUN-04:** Her plugin release için ayrı süreç; IPC protocol version, plugin/version/digest, request/run/session ID ve timeout taşısın.
- [ ] **PY-RUN-05:** Kod yürütme ayrı süreçte lazy başlasın; kodu yüklemeden descriptor ve UI/form tanımları gösterilebilsin.
- [ ] **PY-RUN-06:** Broker context; izinli drawing/dataset snapshot, seçilmiş kaynaklar, unit/CRS, actor ve output sink kapsamını versin.
- [ ] **PY-RUN-07:** Python'a canlı document reference, DB connection, internal slot, UI pointer veya host secret store verilmesin.
- [ ] **PY-RUN-08:** Mutation yalnız validated changeset/command ile uygulansın; plugin'in kendisini built-in veya başka publisher olarak tanıtması reddedilsin.
- [ ] **PY-RUN-09:** Plugin-owned command/tool/UI registration lease ile yönetilsin; disable/crash/uninstall bütün geçici kayıt ve abonelikleri bıraksın.
- [ ] **PY-RUN-10:** UI callbacks action ID üzerinden taşınsın; callback'in mount/run generation'ı eskidiyse mesaj uygulanmasın.
- [ ] **PY-RUN-11:** stdout/stderr, log/progress, exception/traceback ve artifact ayrı kanallarda, limit ve UTF-8 kurallarıyla taşınsın.
- [ ] **PY-RUN-12:** Stop; cooperative cancel ardından gerekiyorsa süreç sonlandırma yapsın. Commit edilmemiş changeset ve geç sonuçlar iptal olsun.
- [ ] **PY-RUN-13:** Plugin crash uygulamayı kapatmasın; kullanıcı çıktısı/taslağı korunsun ve yeniden başlatılabilsin.
- [ ] **PY-RUN-14:** Shared built-in analiz çağrıları parent job/actor/source scope'u devralsın; plugin nested call ile izin genişletemesin.
- [ ] **PY-RUN-15:** Bir plugin'in başka plugin'e çağrısı explicit dependency/capability taşısın; döngü, derinlik ve runtime bütçesi denetlensin.
- [ ] **PY-RUN-16:** Aynı veri üzerinde UI, Python plugin ve AI birlikte çalışırken source revision/expected versions doğrulansın; başka aktif belgeye yazılmasın.
- [ ] **PY-RUN-17:** Environment/runtime lease aktifken update veya uninstall dosyaları silmesin; yeni işler yeni sürüme, çalışan iş eski immutable sürüme bağlı kalsın.
- [ ] **PY-RUN-18:** Plugin persistent settings/data namespace'i kullanıcı/plugin sahibiyle ayrışsın; schema migration ve reset/export yolu olsun.
- [ ] **PY-RUN-19:** Güvenilir yerel Python modu ile OS düzeyinde korumalı çalışma modu açık ayrılmalı. **Ayrı süreç tek başına sandbox değildir.**
- [ ] **PY-RUN-20:** Capability broker yalnız broker erişimini sınırlar; trusted plugin'in doğrudan Python dosya/ağ/OS işlemlerini engellediği iddia edilmesin.
- [ ] **PY-RUN-21:** Korumalı mod seçilmişse platforma uygun filesystem/network/process sınırları gerçek OS izolasyonuyla uygulanmalı; destek yoksa bu mod açıkça kullanılamaz olsun.
- [ ] **PY-RUN-22:** Bulut Python işleri ayrı worker/container, kaynak limiti, network policy, scoped credential ve tenant izolasyonuyla yürüsün; normal desktop plugin süreci sunucu izolasyonu sayılmasın.
- [ ] **PY-RUN-23:** Preview/validate aşamasında broker mutation ve dış artifact commit izinlerini kapat; yetkili execution ile aynı işlem semantiğini doğrula.
- [ ] **PY-RUN-24:** Plugin kodunun güven durumu, public review durumu ve kullanıcı grant'leri ayrı gösterilsin; “onaylı” etiketi sınırsız OS yetkisi anlamına gelmesin.

**Kabul:** Hata veren/sonsuz çalışan eklenti durdurulur; uygulama açık kalır, commit edilmemiş belge değişmez. Disable edilen eklentiye ait tool/UI/action kaydı çalışmaya devam etmez.

## 8. PY-UI — Python'dan KentOS arayüzü üretme standardı — P0/P1

### 8.1 Tek tanım, host'a ait renderer

- `UiDefinition` bir widget ağacıdır; Python builder veya tasarımcı aynı tipli tanımı üretir.
- Masaüstü renderer `kentos-ui` bileşenlerini, web/mobil renderer kendi KentOS karşılıklarını kullanır. Bu, Rust widget'ının Python'a ham bağlanması değildir.
- Widget davranışı, veri binding'i, action ve validation taşınabilir olmalıdır. Piksel bazlı absolute layout normal form/analiz panelinin temel standardı olmamalıdır.
- CBS `FormDefinition`, [form.md](form.md) içindeki veri giriş profilidir; form çekirdeğinde kurulan widget/binding/rule sözleşmesi burada yeniden kullanılır. Serbest plugin paneli ve yetkili feature formu aynı canonical data command'a bağlı, ayrı UI/session profilleridir. Python broker form çekirdeğinin önkoşulu değildir.

| Bileşen ailesi | Desteklenen anlam |
|---|---|
| Layout | Stack/row/grid, tabs, group, accordion, scroll, splitter, stepper |
| Basit girdi | Text/textarea, integer/decimal, date/time, boolean, choice, multichoice |
| CBS girdi | Layer/feature/field picker, coordinate/map pick, CRS/unit, relation reference |
| Kaynak ve çıktı | File/asset picker, attachment, image preview, output destination |
| Veri görünüşü | Table/data grid, relation detail list, tree, card, summary/metric/chart |
| İş denetimi | Button/action, progress/log, validation summary, cancel/retry |
| Host yüzeyi | Dialog, dock/panel, command/ribbon/menu item, context action |

- [ ] **PY-UI-01:** Widget IDs, supported props, value types, bindings/events ve host capability için ortak schema oluştur.
- [ ] **PY-UI-02:** `kentos.ui` benzeri Python facade/builder'ı bu schema'dan üret; yeni isimler mevcut ürün SDK düzeniyle ADR'de kesinleştirilsin.
- [ ] **PY-UI-03:** Python'a UI oluştur/patch/close/focus/notify ve action registration sağlayan broker API sun.
- [ ] **PY-UI-04:** UI tree update; stable IDs ve sürümlü patch taşısın; kullanıcı odağı, yazdığı taslak ve scroll korunmalı.
- [ ] **PY-UI-05:** Widget value state ile kalıcı feature değerini ayır; her harf girişini otomatik document mutation yapma.
- [ ] **PY-UI-06:** Event payload; typed value, action ID, widget ID, session/mount generation ve event sequence içersin.
- [ ] **PY-UI-07:** Uzun callback job olarak çalışsın; UI thread beklemesin, progress/stop gerçek çalıştırıcıya bağlı olsun.
- [ ] **PY-UI-08:** Default/visible/enabled/required/calculated kuralları ortak expression profile'ı kullansın; taşınamayan Python closure gönderilmesin.
- [ ] **PY-UI-09:** Generic Python callback mümkün olabilir; buna bağlı yüzey runtime capability gerektirsin. Zorunlu saha formu sessizce eksik callback ile çalışmasın.
- [ ] **PY-UI-10:** File/map/camera/clipboard işlemleri host service üzerinden yapılsın; Python widget'ı dosya erişimini kendi başına genişletmesin.
- [ ] **PY-UI-11:** Form/table/chart/map seçim bağlantıları command/session IDs ile çalışsın; görsel selection örtük analiz input'u sayılmasın.
- [ ] **PY-UI-12:** Theme, font, spacing, validation, keyboard focus, accessibility ve localization KentOS bileşen kurallarını izlesin.
- [ ] **PY-UI-13:** Screen size sunum profili; grid tek kolona, tab accordion'a geçebilir. Widget/field/action kimliği ve validation anlamı aynı kalsın.
- [ ] **PY-UI-14:** Keyfi HTML/JavaScript/QML normal schema'da bulunmasın; özel rich content sanitization ve ayrı capability/review profili gerektirsin.
- [ ] **PY-UI-15:** Custom widget geliştirme; descriptor, value codec, validation, renderer platformları ve read-only fallback ile kayıtlı olsun.
- [ ] **PY-UI-16:** Eksik custom widget için formun çalışabilirliği önceden kontrol edilsin; mandatory input atlanarak başarılı submit yapılmasın.
- [ ] **PY-UI-17:** Plugin GUI'lerinin durdurma/unload sırası, multiple instance, modal scope ve kaydedilmemiş taslak davranışı tanımlansın.
- [ ] **PY-UI-18:** Aynı UI definition'ın desktop/web/mobile preview ve headless state testleri ortak fixture'larda çalışsın.

**Kabul:** Python eklentisi KentOS bileşenleriyle layer seçimi, numeric input ve sonuç tablosu içeren panel açar. Aynı definition web/mobilde uygun sunumla aynı değer/action/validation davranışını üretir.

## 9. PY-TOOL — ProcessingTool parametre ve çıktı standardı — P0/P1

QGIS, Processing provider üzerinden eklenti algoritmalarını normal araç kutusuna kaydeder. KentOS'ta mevcut registry/runner/model altyapısı bunun karşılığı için kullanılmalı; Python analizlerine ayrı ve sınırlı bir araç kutusu yapılmamalıdır. [QGIS Processing plugin](https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/processing.html)

### 9.1 Descriptor ve executor ayrımı

`ProcessingToolDescriptor` yalnız taşınabilir veridir. `ProcessingExecutor` Rust built-in, Python plugin, browser worker veya yetkili server job gibi yürütme adapter'ıdır. Mevcut Rust function pointer/TS callable alanları bu ayrımın arkasında kalabilir; dış pakette serialize edilecek sözleşme olamaz.

| Parametre türü | Zorunlu semantik bilgi |
|---|---|
| Feature/layer | Source ID, snapshot, geometry kinds, selection/filter, read/edit scope, allowEmpty |
| Field | Source param ref, stable field ID, allowed data types, multiple, read/write intent |
| Numeric | Integer/decimal, unit, min/max/step, precision, nullable/default |
| Choice/domain | Kod tipi, domain/version veya enum; dependent filter ve default |
| Geometry | Point/bbox/path, CRS/UCS/unit, drawing/source scope |
| Raster/cloud/network | Kaynak türü, band/attributes, range, reference system ve tool capability |
| File/table | User-selected asset/file handle, MIME/format, sheet/column mapping, size limits |
| Destination | Yeni layer / geçici sonuç / mevcut veri / yerel dosya / cloud asset seçenekleri |
| Rule/expression | Expression profile/schema refs, izinli fields, validation ve evaluation budget |

| Çıktı türü | Commit / sahiplik |
|---|---|
| Yeni vector layer | Geometry + schema + CRS + metadata/provenance; host atomik oluşturur |
| Source changeset | İzinli feature/field patch'leri; source revisions ve tek undo |
| Raster/point cloud | Staged file/asset; format/CRS/nodata/bands ve bağlama politikası |
| Table/number/text | Tipli sonuç; modelde sonraki adıma aktarılabilir |
| Report/chart | Data + artifact; UI gösterimi ve dosya sonucu ayrılır |
| File/cloud asset | Staged handle; kullanıcı destination seçimi ve yetkili final commit |
| Selection | Kalıcı feature refs; document mutation yerine session sonucu |

### 9.2 Önerilen araç descriptor örneği

```json
{
  "format": "kentos.processingTool",
  "schemaVersion": 1,
  "id": "tr.example.urban-analysis.normalize",
  "version": "0.1.0",
  "label": "Sayısal alanı normalleştir",
  "category": "analysis",
  "executor": { "kind": "extension", "entrypoint": "normalize" },
  "parameters": [
    {
      "id": "input",
      "type": "features",
      "required": true,
      "accepts": { "geometryKinds": ["point", "polyline", "polygon"] },
      "readIntent": "snapshot"
    },
    {
      "id": "valueField",
      "type": "field",
      "sourceParameter": "input",
      "valueTypes": ["integer", "decimal"],
      "required": true
    },
    {
      "id": "destination",
      "type": "destination",
      "allowedKinds": ["newLayer", "file"],
      "default": { "kind": "newLayer", "suggestedName": "Normalleştirilmiş" }
    }
  ],
  "outputs": [
    {
      "id": "result",
      "type": "vectorLayer",
      "sinkParameter": "destination",
      "schemaPolicy": "copy-input-with-added-fields"
    }
  ],
  "mutation": { "mode": "create-output", "undo": "single-group" },
  "feedback": { "progress": true, "cancel": true },
  "capabilities": { "runtimes": ["desktop-python"] }
}
```

Bu örnekte input layer/selection seçimi **çağrı anında** explicit source/snapshot/UUID girdisine çözülür. Dosya hedefi layer oluşturma gibi aynı fiziksel işlem değildir; output sink iki hedef için gerekli format/asset yeteneğini doğrulamalıdır.

Örnek algoritmanın kabul girdisi ayrıca tanımlanmalıdır: min–max normalizasyonunda `[2, 4, 6] → [0, 0.5, 1]`. Boş/null değer, bütün değerlerin eşit olması, exact decimal→NumPy dönüşümü ve output precision politikası descriptor/help/test korpusunda açık belirtilmelidir; “NumPy kullanıyor” algoritma tanımı değildir.

### 9.3 Araç geliştirme ve çalışma görevleri

- [ ] **PY-TOOL-01:** Descriptor schema; parametre/output ID benzersizliği, source references, görünürlük/default ifadeleri ve supported runtime'ı denetlesin.
- [ ] **PY-TOOL-02:** Python SDK; describe/validate/estimate/execute veya eşdeğer handler yaşam döngüsünü tipli tanımlasın; paket inspection için describe import gerektirmesin.
- [ ] **PY-TOOL-03:** Registry namespace/publisher/version çakışmasını reddetsin; built-in ID override edilmesin.
- [ ] **PY-TOOL-04:** Tool dialog descriptor'dan KentOS UI/form standardıyla oluşsun; parametre şeması command/model/Python/AI'ın da kaynağı olsun.
- [ ] **PY-TOOL-05:** Layer/field/feature seçimi UUID/snapshot/projection ile resolve edilsin; private local slot ID dışarıya verilmesin.
- [ ] **PY-TOOL-06:** Geometri/dataset/field type, required domain, CRS, scale/unit ve empty-input redleri hesap öncesi ortak doğrulansın.
- [ ] **PY-TOOL-07:** Read/edit/create-output intent descriptor'da bulunsun; gizli ve kilitli veri, source ACL ve tenant kapsamıyla çözümlensin.
- [ ] **PY-TOOL-08:** Girdi büyükse pagination/chunk/buffer aktar; milyon entity'nin JSON kopyası zorunlu yol olmasın.
- [ ] **PY-TOOL-09:** Girdi snapshot'ı stable source versions ile izlenmeli; hesap bitene kadar canlı belge lock'u tutulmamalı.
- [ ] **PY-TOOL-10:** Python hesabı raw result yerine typed changeset/LayerBuilder/TableBuilder/ArtifactBuilder üretsin; host tamamını doğrulasın.
- [ ] **PY-TOOL-11:** New layer naming/style/schema/CRS/lineage; mevcut free-name/runner yoluyla tek commit'te uygulansın.
- [ ] **PY-TOOL-12:** User destination seçimine göre temporary/newLayer/file/cloudAsset sink çalışsın; overwrite ve source mutation ayrı planlı seçenek olsun.
- [ ] **PY-TOOL-13:** File write staged path/asset üzerinden olsun; validation sonrası atomik finalize, fail/cancel sonrası temizleme yap.
- [ ] **PY-TOOL-14:** Output geometry/field/domain/row/attachment kuralları sunucuda veya yerel command host'ta yeniden doğrulansın.
- [ ] **PY-TOOL-15:** Progress/log/warnings/losses/metrics/result/job handle; typed ve stream edilebilir olsun; success yalnız gerçek commit sonucu gelsin.
- [ ] **PY-TOOL-16:** Cancel exception, process kill, timeout, source conflict ve out-of-memory tek lifecycle'da tanımlansın.
- [ ] **PY-TOOL-17:** Source edit veya multi-layer sonucu tek undo grubuna girsin; dış dosya yan etkisinin undo kapsamı ayrıca bildirilsin.
- [ ] **PY-TOOL-18:** Built-in araç çağrısı, plugin analizinden `ProcessingService` ile yapılabilsin; nested output ve source versions ana işte birleştirilsin.
- [ ] **PY-TOOL-19:** Modeler typed table/file/layer/result çıktılarını bir sonraki adıma aktarabilsin; mevcut table/file kısıtları kapanmalı.
- [ ] **PY-TOOL-20:** Model; plugin ID/version/digest, input/output types ve environment gereksinimlerini saklasın; eksik plugin model açılırken kod kurdurmasın.
- [ ] **PY-TOOL-21:** UI, command line, batch, Python, AI/MCP ve headless server aynı tool ID/schema/result sözleşmesini kullansın.
- [ ] **PY-TOOL-22:** Run provenance; tool/plugin/version, environment digest, input snapshot, params/unit/CRS/seed ve output IDs taşısın.
- [ ] **PY-TOOL-23:** Public tool metadata'da örnek veri ve correctness testleri bulunsun; doğru parametre listesi algoritma doğruluğu kabulü sayılmasın.

**Kabul:** NumPy kullanan araç sayısal alanı okuyup yeni katmana veya dosyaya çıktı üretir; hazır analizle modelde zincirlenir. UI/Python/AI aynı parametrelerden aynı değerleri ve provenance'ı alır. Hata/iptalde yarım katman oluşmaz.

## 10. PY-CLOUD — İsteğe bağlı bulut kataloğu ve yayın onayı — P0/P1

Önerilen state machine: `draft → submitted → checking → needs_changes / approved → published → suspended / revoked / archived`. Upload edilen taslak yalnız yazarı ve yetkili inceleyenler tarafından görülebilir; **upload, yayın değildir**.

- [ ] **PY-CLOUD-01:** Yerel kullanıma cloud hesabı zorunlu olmasın; buluta yayınlama açık kullanıcı eylemi olsun.
- [ ] **PY-CLOUD-02:** Personal/private, tenant catalogue ve public catalogue scope'larını tanımla; görünürlük ve çalıştırma izinlerini ayır.
- [ ] **PY-CLOUD-03:** Yayıncı namespace sahipliği, contact/support, license, source/build bilgisi ve release changelog kaydedilsin.
- [ ] **PY-CLOUD-04:** Gönderilen release package digest + metadata + lock + SBOM + requested capability kümesiyle immutable olsun.
- [ ] **PY-CLOUD-05:** Otomatik check; archive/schema/dependency/license/compatibility, test sonuçları ve gizli bilgi sızıntısını raporlasın.
- [ ] **PY-CLOUD-06:** Kod/bağımlılık smoke testleri izole inceleme worker'ında, gerçek kullanıcı secrets veya üretim verisi olmadan çalışsın.
- [ ] **PY-CLOUD-07:** Yetkili reviewer tarafından açık onay kaydı oluşmadan katalog yayını yapılamasın; yazarın public sürümünü kendi kendine onaylaması normal yol olmasın.
- [ ] **PY-CLOUD-08:** **Her sürüm yeniden onaydan geçsin**; kod/lock/permission/artifact değişikliği eski onayı geçersiz kılsın.
- [ ] **PY-CLOUD-09:** Approve ve publish izinleri ayrı tanımlansın; approved sürümü yayınlama da scope/actor denetiminden geçsin.
- [ ] **PY-CLOUD-10:** Publisher başka `.kek` içeriğini approved digest yerine koyamasın; download ve install bütünlük/onay kaydını doğrulasın.
- [ ] **PY-CLOUD-11:** Review refusal; reason codes, ilgili dosya/dependency ve yeniden gönderim yolu sunsun; keyfi sessiz ret olmasın.
- [ ] **PY-CLOUD-12:** Paket listesi release state, compatibility, Python/native bağımlılıkları ve gerekli izinlerle filtrelenebilsin.
- [ ] **PY-CLOUD-13:** Kurma ve çalıştırma tüketici kullanıcının seçimi olsun; katalogda approved olması kişinin dependency/izin kararını atlamasın.
- [ ] **PY-CLOUD-14:** Update; yeni dependency/capability/supported-platform farklarını göstersin; eski çalışan sürüm kontrollü korunsun.
- [ ] **PY-CLOUD-15:** Suspend/revoke; yeni install/run ve cloud job politikasını uygulasın, sebebi görünür olsun; yerelde indirilmiş bytes'ın geri alınabildiğini iddia etme.
- [ ] **PY-CLOUD-16:** Kötü release raporlama, audit, reviewer history, rollback ve yazar/sürüm ilişkisinin devri denetlenebilir olsun.
- [ ] **PY-CLOUD-17:** Plugin katalog işlemleri tenant/project secret veya private dataset bilgisi sızdırmasın; count/search/result ACL sonrasında hesaplanmalı.
- [ ] **PY-CLOUD-18:** Python/MCP/AI publish çağrısı da aynı state machine'e bağlı olsun; programlı çağrı review'u atlayamasın.

**Kabul:** Onaysız sürüm public/tenant yayın endpoint'inden görülemez ve kurulamaz. Approved paketin tek byte'ı veya lock'u değişirse yeni inceleme gerekir. Kullanıcı cloud'a yayınlamadan `.kek` ile yerel kullanım yapabilir.

## 11. PY-AI — Python, eklenti ve form entegrasyonunda AI — P1

CBS formu üretme/yayınlama/veri girişinin ayrıntıları [form.md](form.md) içindedir. Bu bölüm AI'nin plugin/resource/action üretimini o hazır standarda bağlar.

- [ ] **PY-AI-01:** Plugin/tool/form/environment discover, capabilities, schemas, dependencies, examples ve hata kodlarını ortak katalogda sun.
- [ ] **PY-AI-02:** AI kullanıcı isteğinden draft plugin skeleton, portable UI ve Processing descriptor oluşturabilsin; paket gereksinimlerini açık listeleyebilsin.
- [ ] **PY-AI-03:** AI'nin plugin içindeki portable form/domain/relation resources taslağı [form standardındaki](form.md) compiler/policy kurallarına uysun; mevcut schema'ya göre valid binding kullansın, kaynak yetkisini genişletmesin.
- [ ] **PY-AI-04:** Generated Python ayrı test run ile doğrulansın; “AI yazdı” package review veya dependency kurulum kararı yerine geçmesin.
- [ ] **PY-AI-05:** `inspect → dependency plan → prepare → run → result` aşamaları makinece okunabilir olsun; kullanıcı seçimini gerektiren aşamalar AI tarafından sessiz geçilmesin.
- [ ] **PY-AI-06:** Runtime/environment/package yüklü olup olmadığını capability sorgusundan öğren; kurulu paket veya izin uydurmasın.
- [ ] **PY-AI-07:** Tool çağrıları version/digest/source snapshot ile bağlansın; başka plugin sürümü veya aktif belge değişince stale sonuç algılansın.
- [ ] **PY-AI-08:** Doğal dil analizleri mevcut built-in ve plugin araçlarını aynı plan/model üzerinden zincirlesin.
- [ ] **PY-AI-09:** Plugin açıklaması, README, veri attribute'ları ve form yardım metni untrusted veri olsun; içindeki talimatlar endpoint/actor/grant/publication kapsamını değiştiremesin.
- [ ] **PY-AI-10:** Form field masking, row ACL ve sensitive projection AI context/results'ta da uygulansın.
- [ ] **PY-AI-11:** AI'nin kişisel trusted Python yürütmesi ile scoped processing tool çağrısı ayrı yetki sınıfları olsun.
- [ ] **PY-AI-12:** Generated plugin/form için Python örneği, execution plan, gerçek changeset, test raporu ve output artifacts kullanıcı tarafından incelenebilir olsun.
- [ ] **PY-AI-13:** Model/provider'a gönderilecek veri ve görüntü user/tenant policy ile sınırlandırılsın; bir dependency install isteği veri upload onayı sayılmasın.
- [ ] **PY-AI-14:** Code generation başarısı, semantic tool correctness ve kullanıcı niyetini doğru anlama ayrı eval olarak ölçülsün.

**Kabul:** Kullanıcı AI'a “Bu sayısal alanı analiz eden araç ve formu oluştur” dediğinde incelenebilir draft `.kek` üretilebilir; bağımlılık/izin/review akışları korunarak gerçek tool execution'a geçilir.

## 12. ECO-API — Tek komut/SDK/AI yüzeyi ve üretici araçları — P0/P1

Önerilen katalog aileleri aşağıdadır; bunlar bugün çalışan yeni API adları değildir. Mevcut `kentos.cad` korunmalı; `extensions/ui/processing` facade'larının public isimleri SDK uyumluluk ADR'sinde seçilmelidir. `datasets/domains/relations/forms/assignments/records` komutları ve form SDK tamlığı [form.md](form.md) içinde tanımlıdır; plugin'ler bu aynı komutlara bağlanır.

| Aile | İşlem kapsamı |
|---|---|
| `python.environments.*` | Discover/create/check/list/select/remove, dependency plan/install/update, Python/uv capability |
| `extensions.*` | Inspect/package/check/install/prepare/enable/disable/run/update/uninstall |
| `extensions.catalog.*` | Search/submit/review/approve/publish/suspend/version/download |
| `ui.sessions.*` | Mount/patch/focus/action/close ve host capability |
| `processing.tools.*` | Discover/describe/validate/plan/run/result ve dynamic provider |
| `processing.models.*` / `jobs.*` | Model create/run, source snapshots, progress/cancel/artifacts |

- [ ] **ECO-API-01:** Bütün descriptor/schema/capability/error/type kaynakları Rust contracts veya seçilmiş ortak authoritative kaynakta tutulup codegen ile üret.
- [ ] **ECO-API-02:** Platform adapter'ları lifecycle/state anlamını aynı shared fixture ile uygulasın; Python/AI için alternatif business rule olmasın.
- [ ] **ECO-API-03:** Read/validate/plan çağrıları no-write olsun; environment download/install gibi mutation ve external effect ayrı açık command olsun.
- [ ] **ECO-API-04:** Her command; immutable source/version/digest, actor/context, expected revision ve sonuç/uyarı/undo/job/artifact bilgisi taşısın.
- [ ] **ECO-API-05:** Generated Python stubs/wrappers/docs/help, CLI ve MCP schema drift CI'da hata olsun.
- [ ] **ECO-API-06:** Coverage manifest UI/command/Python/AI/web/desktop/mobile/headless ve seçenek düzeyinde tamlık ölçsün.
- [ ] **ECO-API-07:** Tool/schema/action discover query pagination ve projection kullansın; bütün ekosistem metadata'sı her AI prompt'una doldurulmasın.
- [ ] **ECO-API-08:** Generic execute route aynı authorization/validation/state machine'e tabi olsun; hidden field veya pending release için escape hatch olmasın.
- [ ] **ECO-API-09:** CLI package/create/check/test/export; aynı validator/executor kullansın, local developer test ile cloud publish farklı grant olsun.
- [ ] **ECO-API-10:** SDK template; GUI plugin, Processing plugin, portable form ve birleşik `.kek` örnekleri sunsun.
- [ ] **ECO-API-11:** SDK compatibility/ABI deprecation/migration ve plugin sandbox/runtime matrisi geliştiriciye keşfedilebilir olsun.
- [ ] **ECO-API-12:** AI form/query/submission da effective field/row policy'ye uysun; tasarım/yayın/kurulum/onay işlemlerini ayrı ve doğru actor'la kullansın.
- [ ] **ECO-API-13:** Semantic UI action recorder; izinli işlemi editable Python örneği/replay descriptor'a dönüştürsün; secrets ve hassas field'lar kayda girmesin.
- [ ] **ECO-API-14:** Debug/report aracı environment digest, source versions, descriptor schemas ve hata kodunu sır içermeden toparlasın.

**Kabul:** Bir eklenti UI/Processing aracı bütün desteklenen yüzeylerden keşfedilir ve aynı validation/permission/result üretir. Portable form veya form action içeren `.kek` aynı [form API/ACL standardına](form.md) bağlanır.

## 13. ECO-QA — Doğrulama ve uçtan uca kabul — P0/P1

- [ ] **ECO-QA-01:** JSON Schema/codegen/registry drift, duplicate IDs, source ref ve incompatible capability testleri oluştur.
- [ ] **ECO-QA-02:** Archive parser path/size/ratio/hash/encoding/case/platform edge case korpusu ile fuzz/negative test uygula.
- [ ] **ECO-QA-03:** Opt-in/uv/python/package install testlerini fake transport + temiz OS profiliyle doğrula; gerçek kullanıcı ortamını test için değiştirme.
- [ ] **ECO-QA-04:** Windows/macOS/Linux, Python ABI, architecture/native wheel ve offline cache destek matrisiyle package install/run smoke test yap.
- [ ] **ECO-QA-05:** Python süreç crash/stop/restart, late callback ve plugin unload/multi-instance fixture'ları çalışsın.
- [ ] **ECO-QA-06:** Processing plugin/built-in/model/UI/Python/MCP için aynı source snapshot ve typed result conformance testi kur.
- [ ] **ECO-QA-07:** Portable plugin UI/rules/value codecs/validation aynı host profile'ında shared fixtures ile eşdeğer sonuç versin; CBS form conformance korpusu [form.md](form.md) kapsamındadır.
- [ ] **ECO-QA-08:** Plugin UI görsel testleriyle birlikte state/focus/value preservation, unload, late callback ve validation navigation gerçek input testleri yapılsın.
- [ ] **ECO-QA-09:** ACL testleri ham HTTP/generic command/Python/plugin/AI/table/lookup/export ve event yollarını kapsasın.
- [ ] **ECO-QA-10:** Plugin Processing changeset, typed layer/table/file sink commit, duplicate retry ve staged artifact cleanup fault injection ile sınansın; form submission entegrasyonu form planının kapısını kullansın.
- [ ] **ECO-QA-11:** Draft/release approval digest, package tampering, scope escalation ve unpublished visibility negatif testleri çalışsın.
- [ ] **ECO-QA-12:** Locked plugin environment offline run, eksik wheel/cache, revoked broker grant ve changed source revision senaryoları test edilsin; mobile form offline sync ayrı form planındadır.
- [ ] **ECO-QA-13:** Runtime benchmark; discovery/startup, env hazırlama, callback round-trip, büyük UI/Processing payload, Python batch ve host process memory ölçülsün.
- [ ] **ECO-QA-14:** Performance eşikleri gerçek reference cihaz/korpus ölçümü sonrası sabitlensin; bu planda ölçülmüş sonuç gibi süre/FPS icat edilmesin.
- [ ] **ECO-QA-15:** Kendi generator çıktısını golden yapmayı tek doğruluk kanıtı sayma; bağımsız schema/analiz/veri policy beklentileri oluştur.
- [ ] **ECO-QA-16:** AI eval; dependency/permission seçimini koruma, doğru plugin/UI/tool descriptor üretme, source scope ve release review/publish lifecycle'ini ayrı ölçsün. Form kaynakları form compiler/eval kapısına da girsin.

### 13.1 Zorunlu kabul korpusu

| ID | Senaryo | Beklenen sonuç |
|---|---|---|
| P01 | uv/Python kapalı kullanıcı normal uygulamayı açar | CAD/GIS ve hazır Rust araçları çalışır; dependency download yok |
| P02 | uv bulunmaz; kullanıcı kurulum istemez | Uygulama çalışır; Python plugin blocked ve açık nedenli |
| P03 | OS'ye uygun uv yolunu seçer/kurar | Seçilen executable sürüm/arch doğrulanır; manuel algılama çalışır |
| P04 | Kullanıcı NumPy seçip kişisel ortamına kurar | Önizlenen direct/transitive deps kurulur; console analizi çalışır |
| P05 | İki plugin uyumsuz analitik paket sürümü ister | Ayrı runtime'larda çalışır; kişisel/başka plugin env'i bozulmaz |
| P06 | `.kek` preview; dependencies kurulumunu reddet | Plugin import edilmez; otomatik paket/Python download olmaz |
| P07 | Hazır plugin offline çalışır | Locked hazır ortam kullanılır; implicit sync/install gerçekleşmez |
| P08 | Bozuk/path traversal/hash mismatch `.kek` kur | Live plugin/app dosyası değişmez; structured refusal |
| P09 | Plugin dosya/SDK/import/native lib uyumsuzdur | Ready sayılmaz; dependency veya compatibility hatası doğru gösterilir |
| P10 | Python KentOS UI paneli açar, değer değiştirip analiz başlatır | UI bloklanmaz; state/focus korunur; gerçek progress/stop |
| P11 | Plugin aracı yeni layer ve file sink'e çalıştır | Aynı source analiz değerleri; typed output/provenance; doğru sink commit |
| P12 | Built-in ve Python tool modelde zincirlenir | Table/file/layer çıktıları doğru aktarılır; source scope/grant korunur |
| P13 | Plugin crash/timeout/cancel/unload | Yarım document sonucu yok; late UI/tool callbacks reddedilir |
| P14 | `.kek` onaysız veya aynı sürüm farklı içerikle yayınlanır | Katalog yayını/install reddedilir; approved digest değişmez |
| P15 | Yeni release dependency/capability ekler | Yeniden review; kullanıcı update/izin farkını görür |
| A01 | AI plugin/UI/tool taslağı ve dependency planı üretir | İncelenebilir schema/code; kurulum/yayın/onay aşaması atlanmaz |
| A02 | README/layer/form help içinde araç talimatı vardır | Data olarak işlenir; actor/grant/index/endpoint değişmez |
| A03 | Aynı plugin tool işi UI, command, Python ve MCP üzerinden çalışır | Uygun scope ve aynı semantic result/validation/audit |

CBS formunun I01–I08, F01–F14 ve form AI kabul senaryoları [form.md](form.md) içindedir. Plugin portable form/action sağlıyorsa ilgili form capability kapıları ayrıca uygulanır.

### 13.2 Tamamlandı kapısı

- [ ] Kaynak contract, ortak schema, codegen ve migration tamamlandı.
- [ ] UI/Python/command/AI ve desteklenen runtime adapter'ları aynı semantik testi geçti.
- [ ] Opt-in, izin, dependency, package ve review lifecycle'i uçtan uca doğrulandı.
- [ ] Plugin SDK/Processing query ve mutation grant'leri broker/server girişlerinde ve commit'te uygulandı; form action aynı form policy'sine bağlandı.
- [ ] No-write validate/plan, fail/cancel rollback, idempotency ve source conflict testleri geçti.
- [ ] `.kek` package/lock/resources ve local/cloud install/update/revoke taşınabilirliği doğrulandı.
- [ ] Teslim edilen host'larda plugin UI/executor accessibility ve unsupported runtime capability davranışı doğrulandı.
- [ ] Gerçek analitik/Processing ve KentOS UI plugin'i ile consumer kurulum/kullanım kabulü yapıldı; varsa portable form entegrasyonu form planından doğrulandı.
- [ ] Kalan platform/package/format sınırı capability ve dokümantasyonda açık; yanlış “hazır/tam” durumu yok.

## 14. Uygulama fazları, öncelikler ve bağımlılıklar

P0 ortak kaynak/doğruluk/yetki/opt-in temeli, P1 ürün iş akışları, P2 ileri çalışma/dağıtım kapsamıdır. Takvim veya kişi sayısı bilinmediği için sprint/süre tahmini verilmez.

**Önerilen ilk ürün:** [Form planının](form.md) FM0–FM2 core/central dilimleri ilk yeni özellik olarak çıkarılabilir. Python/uv eklenti altyapısı form kullanımının önkoşulu değildir. Aşağıdaki fazlar ayrı Python ekosisteminin teslimleridir; ileri domain/ilişki/mobile/offline form işlerini tekrar sıralamaz.

| Faz | Somut teslim | Bağımlılıklar / çıkış |
|---|---|---|
| PY0 — Envanter ve ortak contracts | Processing/SDK/ACL boşlukları; UI/tool/plugin standartları ve form profile/action bağlantısı | Mevcut widget/contracts ile form standardını yeniden kullan; örnek descriptor ve fixture korpusu |
| PY1 — Opt-in Python ve `.kek` | uv detect/manual install, kişisel analitik env, locked plugin env, inspect/install/enable/unload | P01–P09; kod import etmeden dependency preview |
| PY2 — Python UI ve Processing | KentOS UI broker, external executor, built-in/model chaining, typed sinks/jobs | P10–P13; UI/command/Python/AI aynı araç |
| PY3 — İsteğe bağlı form entegrasyonu | Portable form resources, registered actions/custom widgets, version/runtime/capability pin | Çalışan form contract/command/ACL'yi kullan; yalnız Python action için PY1–PY2 gerekir |
| PY4 — Onaylı eklenti bulutu | Private/tenant/public review catalogue, digest approval, update/revoke ve izole review worker | P14–P15; her published release onaylı |
| PY5 — Ekosistem kabulü | SDK templates/docs, host matrix, gerçek UI/Processing plugin, performans ve AI eval | Bütün P/A senaryoları ve ilgili integration gate'leri |

Python/AI/komut eşdeğerliği her fazın tamamlanma koşuludur. Form tasarım/veri giriş API'si form planının kendi fazlarında mevcut SDK/katalogdan açılır; bu tablodaki runtime fazını beklemez. Eklenti review ve server executor izolasyonu ilk contracts aşamasında netleştirilmelidir. Bir portable form resource, Python entrypoint import edilmeden form compiler/permission/source remapping ile kullanılabilir olmalıdır.

Bağımlılık omurgası:

1. `Form/ortak widget contracts → portable plugin UI/tool descriptor → Python dynamic executor/broker`.
2. `Opt-in uv → personal dev env → immutable release env → .kek lifecycle`.
3. `Çalışan form command/ACL → optional resource/action/custom widget entegrasyonu`.
4. `Immutable .kek digest + dependency/permission metadata → review → approved publication`.
5. `Aynı source/command/schema → generated Python/CLI/MCP → AI ekosistemi`.

**Döngü oluşturma:** Portable core form → isteğe bağlı plugin integration yönü korunur. Core formdan uv/plugin manager'a zorunlu bağımlılık veya ortak form/profile için ikinci bir plugin-only validator kurulmaz.

### 14.1 İlk 12 Python ekosistem işi

1. Form planındaki core/central önceliğini kabul et; mevcut widget, session, binding ve permission contracts ile plugin kesişimini kaydet.
2. Portable plugin UI, Processing descriptor ve ExtensionManifest v1 ADR/schema taslaklarını kesinleştir.
3. Processing callable/slot bağımlılıklarının external DTO/executor sınırına taşınmasını planla.
4. uv discovery ve opt-in personal environment akışını kur; NumPy seçme/kurma/kullanma senaryosunu çalıştır.
5. `.kek` inspect/package/check ve güvenilir extraction ilk dilimini çıkar.
6. İzole plugin environment, runtime process, broker ve lifecycle registration dikey dilimini tamamla.
7. KentOS UI panelini Python builder/broker ile aç; callback/state/stop/unload akışını doğrula.
8. Python Processing tool'u layer/table/file typed çıktılarıyla built-in/model zincirine bağla.
9. UI/komut/Python/MCP için aynı plugin tool sonuç ve permission korpusunu geçir.
10. Portable form resource ve optional action'ı [form.md](form.md) compiler/deployment/session kurallarına bağla; Python'suz form kullanımı korunsun.
11. Cloud plugin review/digest approval ve consumer install/update/revoke kabulünü uygula.
12. SDK şablonları/host matrisi/gerçek plugin ve AI eval ile ürün kabulünü tamamla; form planının sahip olduğu uygulama işlerini burada tekrar açma.

## 15. Karar kayıtları ve geliştiren ajana sınırlar

- [ ] **ECO-DEC-01:** Runtime OS izolasyon profilleri ve hangi desktop/server ortamında gerçekten uygulanacağı seçilsin.
- [ ] **ECO-DEC-02:** uv/Python supported version/ABI/native wheel matrisi testle sabitlensin; core interpreter ve external wheel build karışmasın.
- [ ] **ECO-DEC-03:** Plugin namespace/signature/review trust root ve içerik digest canonicalization standardı seçilsin.
- [ ] **ECO-DEC-04:** Personal/developer/runtime environment ownership, cache ve release lock/disk layout kararı yazılsın.
- [ ] **ECO-DEC-05:** Portable plugin UI ve form standardının widget/profile/version bağlantısı yazılsın; source/binding/ACL için [form.md](form.md) authoritative kalsın.
- [ ] **ECO-DEC-06:** Python tool executor DTO, typed source snapshot/batch API, output sink ve transaction/commit sınırı seçilsin.
- [ ] **ECO-DEC-07:** Python UI callback/job, modal/multiple-instance/unload ve host state ownership politikası seçilsin.
- [ ] **ECO-DEC-08:** `.kek` portable form/resource import'u ile executable entrypoint hazırlama/etkinleştirme bağımsız capability olarak seçilsin.
- [ ] **ECO-DEC-09:** Optional form/Processing action için form release, plugin digest, env/runtime ve broker grant pin politikası form planıyla ortak tanımlansın.
- [ ] **ECO-DEC-10:** Web/mobile Python executor ve package support matrisi ayrı doğrulansın; portable form kullanımı bu desteğe bağlanmasın.
- [ ] **ECO-DEC-11:** Private index/credentials/native build ve cloud worker network/filesystem/tenant izolasyon politikası belirlensin.
- [ ] **ECO-DEC-12:** Public/tenant plugin review rolü ile kurum form publication/data review rollerinin farklı kaynak lifecycle'ları olduğu sabitlensin.

Uygulama notları:

- Bu plan KentOS application geliştirmesidir; Codex uygulamasına eklenti/skill kurma talimatı değildir.
- Mevcut Rust/TS uygulama sınırları, KentOS UI widget'ları, processing runner, SDK/codegen ve ProjectAccess genişletilmeli; ikinci paralel sistem kurulmasın.
- Mevcut `run: fn(...)` veya TS closure'ı Python plugin desteği tamamlanmış sayma; external executor köprüsü gerekir.
- Form/data/schema/domain/relation tasarımı, ACL ve mobile/offline işleri [form.md](form.md) içinde sürdürülür. Plugin'in resource/action entegrasyonu aynı authoritative contracts/commands'a bağlanır.
- Temel form feature'ı Python/uv veya `.kek` kurulumu beklemez; Python-dependent action yalnız kendi declared capability'sini gerektirir.
- ZIP/integrity/review ve environment izolasyonu birbirinin yerine güvenlik kanıtı kullanılmasın.
- NumPy/analitik paketler isteğe bağlı Python capability'dir; Rust geometri veya portable form kural çekirdeğini bu paketlere bağımlı yapma.
- Review worker production credentials ile çalışmasın; plugin resource preview/import kod yürütme veya dependency kurulum onayı sayılmasın.
- Başka geliştiricinin devam eden uygulama dosyalarına bu plan çalışması adına müdahale edilmemeli; uygulama diliminde kaynak sahipliği ayrıca netleştirilmeli.
- Generated katalog/schema/SDK dosyaları elle düzeltilmemeli; kaynak ve üretici aynı commit'te güncellenmeli.
- `.kek` açık dependency/compatibility/permission raporu üretmeli; kişisel kurulum tercihi ve cloud release onayı farklı aşamalardır.
- Her teslim gerçekten çalıştırılan test, kalan capability ve atlanan kabulü ayrı raporlamalıdır.

## 16. Resmi araştırma kaynakları

Kaynaklar 12 Ekim 2026 araştırmasında incelendi. QGIS eklenti/Processing kıyası 3.44 resmi dokümanını esas alır. Ürün sürümleri ve platform desteği uygulama aşamasında yeniden doğrulanmalıdır. CBS formu/QGIS form designer/ArcGIS domain ve ilişkiler kaynakları [form.md](form.md) içinde tutulur.

| Kaynak | Kullanıldığı konu |
|---|---|
| [uv — Installation](https://docs.astral.sh/uv/getting-started/installation/) | OS kurulum yolları ve kullanıcı seçimi |
| [uv — Python versions](https://docs.astral.sh/uv/concepts/python-versions/) | Managed/system interpreter, automatic download ve sürüm seçimi |
| [uv — Projects](https://docs.astral.sh/uv/guides/projects/) | pyproject/lock/environment ve geliştirme akışı |
| [uv — Dependencies](https://docs.astral.sh/uv/concepts/dependencies/) | Direct/optional/dev dependency ayrımı |
| [uv — Locking and syncing](https://docs.astral.sh/uv/concepts/projects/sync/) | Lock/sync/auto-update ve install/run ayrımı |
| [uv — Package indexes](https://docs.astral.sh/uv/concepts/indexes/) | Private index, kaynak seçim/priority |
| [uv — Caching](https://docs.astral.sh/uv/concepts/cache/) | Cache ve offline hazırlama |
| [uv — Using environments](https://docs.astral.sh/uv/pip/environments/) | Isolated venv ve system environment sınırları |
| [uv — CLI reference](https://docs.astral.sh/uv/reference/cli/) | Adapter'ın seçilmiş sürümle kullanacağı flags/options |
| [QGIS 3.44 — Structuring Python Plugins](https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/plugins/plugins.html) | Metadata, lifecycle ve plugin dependency yapısı |
| [QGIS 3.44 — Writing a Processing plugin](https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/processing.html) | Provider ve normal araç kutusuna algoritma kaydı |
| [QGIS 3.44 — Releasing your plugin](https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/plugins/releasing.html) | Paket ve dağıtım iş akışları; KentOS review politikası kullanıcı şartıdır |

**Ürün kabulü:** Kullanıcı kendi seçtiği ortam/paketlerle KentOS UI ve Processing eklentisi geliştirir; `.kek` ile açık dependency/izin bilgisiyle dağıtır, cloud yayını her sürümde onaydan geçer. Portable CBS formu ve optional action'lar [form.md](form.md) standardına bağlanır; temel form kullanımı Python/uv eklenti altyapısından bağımsız kalır.
