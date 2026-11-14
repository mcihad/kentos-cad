# ADR 8057: Faz 3 — `.kek` eklenti paketleri

- **Durum:** uygulanıyor (13 Kasım 2026). Kaynak: `plan/eklenti.md` (eklenti sisteminin tek planı; `argon-tavsiye.md`
  FAZ 3'ün `ADIM 16` – `ADIM 22A`'sı).
- **Sahibin sözü (13 Kasım):** "Bu konuyu çok iyi çalışman gerekiyor çünkü bu ekosistemi büyütecek en iyi şey QGIS
  eklenti sistemi kadar güçlü ve sağlıklı çalışmalı, örneğimiz orası."
- **Bağlam belgesi:** `docs/research/2026-11-13-qgis-eklenti-sistemi.md`, `docs/research/2026-11-13-eklenti-ekosistemleri.md`
  (planın §1.5'teki D1–D18 dersleri), ADR 0053 (ZIP okuyucu), ADR 0401 ve 0435 (form ön denetimi ve form paketi),
  ADR 8056 (Faz 2).

## §1. Paket standardı, denetleyici ve paketleyici (`ADIM 16`)

**Ne yapıldı.** `crates/shared/extensions` (`kentos-extensions`): manifest v1'in tipleri ve kuralları, `integrity.json`,
paketi **hiçbir kodunu çalıştırmadan** açan denetleyici (`inspect`, `open`) ve deterministik paketleyici (`pack`). Saf
crate: dosya sistemi ve ağ yok; masaüstü, başsız host, MCP, bulut kataloğu ve tarayıcı aynı kodla denetler
(`wasm32-unknown-unknown` için derlendi). Yeni dış bağımlılık yok: `roxmltree`, `sha2`, `serde` zaten çalışma alanındaydı;
ZIP `kentos-zip`'in (ADIM 16'nın ön adımı), formlar `kentos-commands` ve `kentos-forms`'un. Yerel komut `apps/cli`
(`kentos` ikilisi): `kentos kek inspect <paket.kek> [--json]` ve `kentos kek pack <klasör> [-o <paket.kek>] [--json]`;
çıkış kodu 0 tamam, 1 ret, 2 kullanım ya da dosya hatası.

**Denetimin sırası** (bir aşama düşerse sonraki çalışmaz; kötü bir arşiv hiç açılmaz):

1. Arşiv: boyut, ilk bayttan ZIP, dizin, iki okuyucunun aynı gördüğü yerleşim.
2. Her girdi: yol kuralları, derinlik, yinelenen ya da harf büyüklüğüyle ayrılan ikiz ad, bağ, özel dosya, şifre,
   bayt taşıyan klasör girdisi, dosya başına boyut ve sıkıştırma oranı, toplam.
3. Pakete girmeyecekler (ada ve ilk baytlara göre).
4. İçerik: `integrity.json`, `manifest.json` ve kuralları, başvurduğu her dosya (ne olduğuyla), işlevlerin modülleri,
   Python projesi ile kilidi, her metin dosyası.

Her sorun bir `Problem`'dir: kalıcı kod, dosya, yer (JSON işaretçisi ya da `satır N`) ve Türkçe çözüm. Ajan tek adımda
düzeltebilsin diye serde'nin İngilizce iletisi Türkçeye çevrilir ("Bilinmeyen alan “chanel”. Buradaki alanlar: format,
schemaVersion, …") ve JSON işaretçisi serde_json'un satır/sütunundan metin bir kez yürünerek çıkarılır
(`pointer.rs`): `serde_path_to_error` bunu verirdi ama yeni bağımlılık sahibin kararı (CLAUDE.md §3), küçük tarayıcı
yeterli.

### Uygularken verilen kararlar (planda yazılı değildi)

1. **Yetenek kapsamları sabit:** `document.read|write` → `current-project`, `user-selected`; `datasets.*` → üçü de;
   `files.userSelected` → yalnız `user-selected`; arayüz ve sağlayıcı yetenekleri ile `network` → yalnız `plugin-owned`.
   Bir yetenek bir kez istenir.
2. **Katkı yeteneğini yazar:** şerit sekmesi `ui.ribbonTab`, sağ tık `ui.contextMenu`, panel `ui.panel`, İşlemler
   `processing.provider`, denetim `audit.provider`, ifade `expression.provider`, harita aracı `mapTool`, veri kaynağı
   `dataSource`, adresli veri kaynağı `network` ister; eksiği ret (kullanıcı kurmadan önce hepsini görmeli), kullanılmayan
   arayüz yeteneği uyarı. Planın §3.2 örneğine eksik beş yetenek eklendi. Formlar yetenek istemez: formu katmana
   kullanıcı bağlar.
3. **Sağ tık hedefleri:** `drawing` (çizim/harita alanının menüsü) ve `layer` (Katmanlar'daki katman); seçim durumu
   `when.selection`'dadır.
4. **`builtin` düğmesi** adını (`label`) yazar ve `ext.` ile başlayan komutu gösteremez (bir katkı yalnız ekler).
5. **Uyumluluk:** `sdkProtocol`'ün `min` ve `max`'ı zorunlu (protokol kırılmasının anlamı örtük olmasın); KentOS
   aralığının uçları isteğe bağlı, boş aralık ret; sistem listesi yazılmadıysa her sistem.
6. **Sürümde yapı eki (`+…`) ret:** karşılaştırmada yok sayıldığı için iki farklı paket aynı sürüm görünürdü.
7. **Sunucular:** `hosts` küçük harfli ASCII alan adı ya da IPv4; şema, yol, joker yok. Veri kaynağı adresinde kullanıcı
   adı/parola ret; `http` kabul, uyarıyla (belediye servislerinin bir kısmı şifresiz).
8. **Yollar:** `A–Z a–z 0–9 . _ -` ve `/`, en çok 240 bayt, derinlik 12; Windows aygıt adları (`con`, `aux.txt`,
   `COM1`) ve noktayla biten ad ret (Windows'ta açılınca değişirler).
9. **Sınırlar** (`Limits::default`): paket 128 MiB, 2000 girdi, açılınca 256 MiB, dosya başına 64 MiB, oran 200.
10. **İki okuyucu aynı görmeli** (`kentos_zip::layout_problem`): arşiv ilk girdinin yerel başlığıyla başlar, her yerel
    başlığın adı dizindekiyle aynıdır, iki girdi aynı baytları paylaşmaz. İnceleyenin aracı ile KentOS'un farklı dosya
    gördüğü arşiv (başına program eklenmiş, yerel adı farklı) böylece reddedilir.
11. **Klasör girdisi** boşsa kabul (zip araçları yazıyor), bayt taşıyorsa ret.
12. **Girmeyecekler:** Python ortamı, derleme önbelleği, sürüm denetimi, gizli bilgi (`.env`, anahtar, `.netrc` …), AI
    aracı ve düzenleyici ayarları (`mcp.json`, `.cursorrules`, `.claude/`, `.vscode/` …), derlenmiş kod (adına ve ELF,
    PE, Mach-O, WebAssembly ilk baytlarına göre), iç içe arşiv, işletim sisteminin ek dosyaları. Paketleyici çalışma
    klasörünün olağan artıklarını (ortam, önbellek, sürüm denetimi, gizli bilgi, AI ayarları, eski paketler, `build/`,
    `dist/`) **dışarıda bırakır ve listeler**; derlenmiş kodu ve arşivi reddeder.
13. **Metin:** kod ve verilerde (`.py`, JSON, TOML, kilit, SVG …) görünmez biçim karakteri ret; düzyazıda (`.md`, `.txt`,
    `LICENSE`) yalnız emojinin birleştiricisi (U+200D) ve sunum seçicisi (U+FE0F) serbest. Denetim karakterleri (uçbirimi
    değiştirebilir) ret; veri dosyasında BOM ret. Kullanıcının ev klasörüne mutlak yol kodda ret, düzyazıda uyarı.
14. **İkon:** masaüstü çizicisinin okuduğu alt küme (`kentos_ui::icon::svg`). Çizici işaretlemeyi elle okuduğu için onun
    okuyuşunun XML'den ayrılabileceği her şey de ret: yorum, CDATA, işleme talimatı (XML bildirimi hariç), varlık
    başvurusu, tek tırnak, `=` çevresinde boşluk. Renk ret (`currentColor` ya da `none`): ikon tema rengiyle çizilir.
15. **İçerik özeti (ECO-DEC-03'ün özet kısmı):** `kentos.extension-integrity` v1'in satırları yol sırasıyla
    `<sha256> <boy> <yol>\n`; özet bunun SHA-256'sı, `sha256:` önekiyle. ZIP'in sıkıştırmasından ve tarihinden bağımsız
    (testle: aynı dosyalar sıkıştırmasız ve ters sırayla yeniden ZIP'lenince özet aynı). Altın örneğin özeti bu crate'ten
    ayrı Python ile hesaplanıp teste sabitlendi; `fixtures/extensions/.gitattributes` satır sonu dönüşümünü kapatır.
    Güven kökü (imza, onay) ADIM 22A'da ve sahibin kararı.
16. **Deterministik paketleme:** önce `manifest.json`, sonra `integrity.json`, sonra yol sırası; 1 Ocak 1980 tarihi;
    deflate ya da daha küçükse sıkıştırmasız. Paketleyici çıktısını kendisi bir kez daha inceler.
17. **Python, hafif okuma:** bağımlılık eklememek için küçük bir TOML okuyucusu (`pyproject.toml` ve uv'nin yazdığı alt
    küme); güvenlik sınırı değil, kurulumda uv yeniden okur. Denetimler: `requires-python` iki dosyada aynı aralık
    (anlamca: uv `~=3.12`'yi `>=3.12, <4` yazıyor; Python 2.0–4.30.5 üzerinde karşılaştırılır), kökün `requires-dist`'i
    ile bağımlılıklar (ad + sürüm koşulu, ek grup başına), `required-markers` ile `required-environments`,
    `supported-markers` ile `environments`, manifestin her sistemine ikisinde de karşılık, her bağımlılığın kilitte
    çözülmüş olması. Yerel klasör kaynağı ret; git/adres kaynağı ve PyPI dışı depo uyarı; bilinmeyen kilit sürümü
    uyarı.
18. **Formlar:** önce `FormPackage::problem` (`cad.forms.import` ile aynı kurallar ve sözler), sonra form çekirdeğinin ön
    denetimi (`kentos_forms::preflight`) paketin kendi alanlarıyla. Kurulacağı projeyi gerektirenler (başka katmandan
    liste, ilişki: `form_lookup_*`, `form_relation_unknown`) ve ön denetimin uyarıları **uyarıdır**; kurulumda proje
    bağlamıyla yeniden koşar.
19. **Henüz yalnız biçim:** araç tanımında `format`, `schemaVersion` ve kimliğin manifestle aynılığı (tam
    `kentos.processingTool` şeması ADIM 19); panel tanımı ve `schema.json` için JSON nesnesi olmaları (ADIM 22).
20. **İşlevin modülü** `pyproject.toml`'un klasörüne göre `src/<modül>.py`, `src/<modül>/__init__.py` ya da düz yerleşimde
    aranır; fonksiyonun varlığı ve dekoratörü AST ile `extension.verify`'ındır (ADIM 21).
21. **Bağımlılık döngüsü:** manifest kendisine bağımlılığı reddeder; paketler arası döngüyü `dependency_cycle` bulur
    (kurucu kullanır, ADIM 17).
22. **`kentos kek pack`** klasördeki sembolik bağı reddeder (klasörün dışını gösterebilir); `.venv` gibi ağır klasörleri
    hiç okumaz, yalnız adını "pakete girmeyenler"e yazar.

### Sorun kodları

| Aşama | Kodlar |
|---|---|
| Arşiv | `archive_too_large`, `archive_not_zip`, `archive_broken`, `archive_layout`, `archive_encrypted`, `archive_link`, `archive_special`, `archive_folder`, `archive_file_too_large`, `archive_ratio`, `archive_total`, `archive_entries`, `archive_write` |
| Yol | `path_unsafe`, `path_depth`, `path_duplicate`, `path_case` |
| İçerik | `content_junk`, `content_environment`, `content_cache`, `content_vcs`, `content_secret`, `content_ai_config`, `content_binary`, `content_archive` |
| Bütünlük | `integrity_missing_file`, `integrity_json`, `integrity_format`, `integrity_self`, `integrity_duplicate`, `integrity_unlisted`, `integrity_missing`, `integrity_mismatch` |
| Manifest | `manifest_missing`, `manifest_json`, `manifest_format`, `manifest_newer`, `manifest_schema_version`, `manifest_unknown_field`, `manifest_missing_field`, `manifest_duplicate_field`, `manifest_bad_type`, `manifest_bad_value`, `manifest_id`, `manifest_version`, `manifest_text`, `manifest_html`, `manifest_authors`, `manifest_publisher`, `manifest_license`, `manifest_path`, `manifest_link`, `manifest_keyword`, `manifest_modules`, `manifest_runtimes`, `manifest_compat`, `manifest_dependency`, `manifest_dependency_cycle`, `manifest_python_missing`, `manifest_python`, `manifest_l10n`, `manifest_deprecation`, `manifest_local_id`, `manifest_duplicate_id`, `manifest_handler`, `manifest_icon`, `manifest_map_tool`, `manifest_utility_tab`, `manifest_tab_module`, `manifest_tab_empty`, `manifest_item_target`, `manifest_item_unknown`, `manifest_item_label`, `manifest_builtin`, `manifest_when`, `manifest_data_source`, `manifest_data_source_host`, `manifest_capability_reason`, `manifest_capability_scope`, `manifest_capability_duplicate`, `manifest_capability_network`, `manifest_capability_missing`, `manifest_empty` |
| Başvurulan dosya | `reference_missing`, `reference_empty`, `reference_json`, `icon_unsafe`, `tool_descriptor`, `form_invalid`, `form_preflight`, `geopackage_invalid`, `handler_missing` |
| Metin | `text_encoding`, `text_bom`, `text_control`, `text_invisible`, `text_user_path` |
| Python | `python_toml`, `python_project`, `python_extra_missing`, `python_environments`, `python_lock_stale`, `python_local_source` |
| Uyumluluk (`compatible`) | `compat_sdk_protocol`, `compat_kentos_min`, `compat_kentos_max`, `compat_os` |
| Uyarılar | `manifest_link_insecure`, `manifest_capability_unused`, `manifest_python_unused`, `catalog_changelog`, `form_preflight_warning`, `text_user_path` (düzyazıda), `python_lock_version`, `python_vcs_source`, `python_registry` |

### Altın örnekler

- `fixtures/extensions/v1/mebis/`: dikey eklenti; komut, harita aracı, İşlemler aracı, denetim, ifade, panel, iki form,
  Python'suz veri kaynağı, şema, çeviri, ikon; `pyproject.toml` ve **gerçek `uv lock` çıktısı** (uv 0.11.26; `shapely`,
  isteğe bağlı `pygal`; üç sistem `environments` ve `required-environments`'ta). Paket 14 382 bayt, 16 dosya, içerik özeti
  `sha256:e3816e901e3241d238547ddd25a9d31731d0e6931e96752ee8ff8507030a3b1b`. 14 Kasım: araç tanımı ADIM 19'da
  kesinleşen v1'e geçti (taslağın `id`/`accepts`/`destination` biçimi kurulumdaki tam denetimden geçmiyordu); özet
  `sha256:ec9730da3c4841b7ec3b5ff82103e90db52766eb7d3e3c3da47ebec40ac46501` (Python'da ayrıca hesaplandı).
- `fixtures/extensions/v1/ornek-altlik/`: en küçük paket (D2): `manifest.json`, `LICENSE` ve iki servis veri kaynağı;
  Python yok. Paket 1 400 bayt; tek uyarı değişiklik notunun yokluğu.

### Doğrulama

- `cargo test -p kentos-extensions`: 49 test; `cargo test -p kentos-zip`: 8 test (yerleşim denetimi dahil). Plandaki
  olumsuz korpusun her maddesi bir testte yapısal Türkçe ret: zip-slip, mutlak yol, sembolik bağ, yinelenen ve harf
  çakışmalı ad, oran, şifreli girdi, listede olmayan dosya, özet uyuşmazlığı, `.venv`/`.pyc`/ikili, eksik manifest,
  yanlış biçim, bilinmeyen alan, kötü kimlik/sürüm, `utility`'nin sekmesi, bilinmeyen modül ve yetenek, gerekçesiz
  yetenek, `hosts`'ta olmayan sunucu, kayıp başvuru, bozuk form, betikli ve desteklenmeyen öğeli ikon, görünmez Unicode,
  `mcp.json`, eskimiş kilit, platform kapsamı eksik kilit, döngülü bağımlılık ve uyumluluk aralığının uçları.
- Panik yok: geçerli paketin her ~96. baytında kesilmiş, bir baytı ve sekiz baytı bozulmuş ~450 paket incelendi (hata
  ayıklama derlemesinde testlerin tamamı 0,27 s).
- `kentos kek pack` iki altın örneği paketledi, `kentos kek inspect` geçerli dedi; Python'un `zipfile`'ı paketi CRC
  hatasız okudu, `unzip -l` tarihleri 1980-01-01.
- Clippy (değişen crate'ler) uyarısız; `node scripts/arch/deps.mjs` temiz (yeni `cli` grubu: yalnız ortak
  kütüphaneler); `cargo check --target wasm32-unknown-unknown -p kentos-extensions` temiz.

**Sonraki:** ADIM 17 (kayıt defteri, kiralama, yerel yaşam döngüsü) bu crate'in `open`'ını ve `dependency_cycle`'ını
kullanır; yayımcı alan adı ve lisans listesi gibi katalog denetimleri ADIM 22A'nındır.

## §2. Kayıt defteri, kiralama ve yerel yaşam döngüsü (`ADIM 17`)

**Ne yapıldı.** `crates/native/extension-host` (`kentos-extension-host`): bir kullanıcının kurduğu paketler, durumları,
kiralamaları, güncelleme ve geri dönüşleri, bekçi, kurum ilkesi ve geri çekme listesi. Planda yeri
`crates/native/application` idi; **ayrı crate'e alındı**: kayıt defteri eklenti sistemi büyüdükçe sık değişecek ve o
crate'in yeniden derlenmesi dakikalar sürüyor (ADR 8054); masaüstü, başsız host, MCP ve Python host'u aynı kodu
kullanacak. Bağımlılık yönünde kendi grubu (`extension-host`): yalnız ortak kütüphaneler; kullananlar ADIM 18'den
başlayarak `uses`'a adını yazar. Eklenti kodu çalıştırmaz.

**Diskteki yeri** (masaüstünde `<veri>/kentos-cad/eklentiler`, ADIM 18): `registry.json` (`kentos.extension-registry`
v1: kaynak, kurulu sürümler ve özetleri, kullanılan sürüm, etkinlik, izinler, kanal, sabitlenen sürüm, hata, art arda
çökme sayısı, ölçüm, son 50 satırlık günlük), `revocations.json` (son uygulanan geri çekme listesi, çevrimdışı da
geçerli), `packages/<id>/<sürüm>/` (salt okunur dosyalar), `staging/` (hazırlanan sürüm; açılışta artığı silinir),
`data/<id>/` (eklentinin kendi verisi; kaldırınca kalır), `.lock`.

**Kararlar.**

1. **Durumlar** diskten ve çalışandan hesaplanır, kaydedilmez: Etkin, Hazır, Devre dışı, Bağımlılık bekliyor,
   Engellendi, Uyumsuz, Hata; her birinin Türkçe adı ve "ne yapmalı" metni var. Planın "keşfedildi, incelendi,
   hazırlanıyor" durumları `Review` ve süren bir çağrıdır (yarıda kalan çağrı önceki durumu bırakır); "güncelleme
   bekliyor" katalogla gelir (ADIM 22A).
2. **Kiralama:** her etkinleştirme yeni bir kuşakla `ExtensionLease` alır; katkılar (veri kaynağı, form, şerit sekmesi,
   sağ tık, panel, komut, harita aracı, İşlemler, denetim, ifade) `ext.<eklenti>.<yerel>` adıyla onun altında kaydolur;
   `revoke_lease` hepsini birden siler; `is_current` eski kuşağın geç çağrısını ayırır.
3. **Kurulum** baytları yeniden inceler (önceki `Review`'a güvenmez), uyumluluk, kurum ilkesi, geri çekme, bağımlılık
   döngüsü ve verilen izinlerin paketin istediklerinden olmasını denetler; dosyaları `staging`'e yazıp geri okuyarak
   özeti doğrular, yerine taşır ve kaydı **tek yazımla** değiştirir. Bundan önceki her hata kullanılan sürüme
   dokunmaz. Diskte iki sürüm tutulur: kullanılan ve bir önceki (`switch_version` ile geri dönüş).
4. **Aynı sürüm, farklı içerik reddedilir** (yalnız geliştirici klasörü aynı sürümü yeniden yükler). Aynı paket başka
   bir seçimle yeniden kurulursa izinler değişir ve eklenti yeni kiralamayla yeniden başlar; dosyaları bozulmuş bir
   sürüm aynı paketle yeniden kurulunca onarılır.
5. **Bütünlük her etkinleştirmeden önce:** kurulu dosyalar yeniden özetlenir; tutmazsa kalıcı "Hata" ve güven
   "Bütünlüğü bozuk"; dosyalar salt okunur yazılır.
6. **Bağımlılıklar:** eksik, kapalı ya da aralık dışı bağımlılık "Bağımlılık bekliyor"; açılış sırası bağımlılıktan
   çıkar; bağımlı olunan durunca bağımlıları da durur, gelince yeniden başlar; döngü kurulumda reddedilir.
7. **Bekçi:** art arda **3** çökme (`CRASHES_TO_FAIL`) kalıcı "Hata"; iyi bir çalışma sayacı sıfırlar; durdurulmuş
   kiralamanın raporu yok sayılır; "Yeniden dene" kullanıcının. Eklentisiz başlatma (`safe_mode`) hiçbir şeyi
   etkinleştirmez. "Sorunlu eklentiyi bul" (`Bisection`): önce eklentisiz bir açılış (sorun eklentiden mi?), sonra
   yarılar; en çok 1 + ⌈log₂ n⌉ yeniden başlatma.
8. **Kaynak ve güven:** aynı kimlik başka kaynaktan (ya da başka bir geliştirici klasöründen) ancak kullanıcının açık
   seçimiyle değişir. Güven: Yerel (dosya, geliştirici klasörü), Kurum, Onaylı (genel katalog; imzası ADIM 22A'nın),
   Bütünlüğü bozuk, Engelli.
9. **Kurum ilkesi** (`kentos.extension-policy` v1): izinli kaynaklar, yayımcılar, eklentiler; engelli eklentiler ve
   yetenekler; eklenti başına en düşük sürüm; dış yapay zekâya veri izni. Host sistem klasöründen okur, kullanıcı
   gevşetemez; kurulu bir eklentiyi sonradan yasaklarsa eklenti durur, silinmez.
10. **Geri çekme listesi** (`kentos.extension-revocations` v1): kimlik, sürüm aralığı, gerekçe; aralıktaki önizleme de
    geri çekilmiş sayılır (`VersionReq::contains`). Kullanılan sürüm durur, verisi kalır, o sürüm yeniden kurulamaz;
    düzeltilmiş sürüm kurulur. İmza denetimi kataloğun güven köküyle (ADIM 22A).
11. **Birden çok süreç:** her değişiklik kilidi alır, kaydı yeniden okur, değiştirir, geçici dosyaya yazıp yerine
    taşır; iki dakikadan eski kilit çökmüş bir sürecindir ve devralınır.
12. **Proje:** `project_needs` bir projenin istediği eklentilerin durumunu verir (Hazır, Çalışmıyor ve nedeni, Yok);
    proje açmak kod çalıştırmaz, eksik eklentiyi kurdurmaz; `.kcad`'in `modules.ext`'ini okumak ve kısıtlı kip ADIM 18.
13. **Ölçüm:** etkinleştirme süresi (mikrosaniye) her etkinleştirmede kaydedilir; süreç belleği ve sayısı Python
    süreciyle gelir (`record_metrics`, ADIM 19); eşik ölçümden sonra.

**Sonraki adımlara kalanlar:** Python ortamının hazırlanması (ADIM 18A) ve süreç/aracı (ADIM 19); çökmede taslağın
korunması süreç düzeyidir (ADIM 19); gecikmeli otomatik güncelleme, uyumlu sürüm önerisi ve imzalar katalogla (ADIM 22A);
güncellemede veri göçü, eklentinin şemalı verisi ile (ADIM 22); arayüz, ilke dosyasının sistem yolu ve `.kcad` bağlantısı
ADIM 18.

**Doğrulama:** `cargo test -p kentos-extension-host`: 19 test (0,08 s). Plandaki kabul: Python'suz paket kur →
etkinleştir → devre dışı → yeniden etkinleştir → güncelle → geri dön → kaldır, her adımdan sonra eski kiralamadan hiçbir
katkı kalmıyor ve yeniden açılan kayıt aynı; yarım güncelleme (bozuk paket, `staging` artığı) çalışan sürüme dokunmuyor;
art arda üç kez çökertilen eklenti kalıcı "Hata"ya geçiyor ve sonraki açılışta denenmiyor; geri çekilen sürüm duruyor,
verisi kalıyor; bağımlılık döngüsü reddediliyor. Ayrıca: aynı sürüm farklı içerik, kaynak çakışması, kurum ilkesi,
verilmeyen izin, geliştirici klasörünün yeniden yüklenmesi, diskte değiştirilmiş dosya, yeni SDK protokolü, inceleme
farkı, proje ihtiyaçları, güvenli kip ve iki süreç. Clippy uyarısız; bağımlılık yönü temiz (60 crate).

## §3. Masaüstü, katalog ve döküm (`ADIM 18`)

**Katalog dizini** (`kentos_extensions::catalog`, `kentos.extension-catalog` v1). Kurum paylaşımındaki ya da çevrimdışı
bir diskteki `.kek` dosyalarının yanında duran statik `index.json`; `kentos kek index <klasör> [--name <ad>]` onu
paketlerin kendisinden yazar, yani dizinde yazan, denetleyicinin okuduğudur. Her paket için kurmadan önce gösterilecek
her şey içindedir: ad, özet, açıklama, sınıf, modüller, anahtar sözcükler, yazarlar, yayımcı, lisans, bağlantılar,
uyumluluk, bağımlılıklar, gerekçeli yetenekler, değişiklik notu (16 000 karaktere kadar) ve ikonun SVG metni; ayrıca
dosyanın adı, boyu, SHA-256'sı ve içerik özeti. Kurulmayacak bir paket dizine girmez (dizin yazılmaz, sorunlar dosya
adıyla söylenir); aynı kimlik ve sürüm iki dosyada olamaz. İndirilen paket kurulmadan önce boyu, SHA-256'sı ve içerik
özetiyle dizindekine karşı doğrulanır (`catalog_mismatch`). `latest` her paketin en yeni sürümünü verir; önizleme
(önsürüm) yalnız istenince. Dizine güven klasörü kadardır; genel kataloğun imzası ADIM 22A'da.

**Yerler** (`kentos_extension_host::places`). Kayıt defteri programın veri klasörünün `eklentiler`'idir
(`$XDG_DATA_HOME/kentos-cad/eklentiler`, Windows'ta `%APPDATA%\kentos-cad\eklentiler`, macOS'ta `~/Library/Application
Support/kentos-cad/eklentiler`); kurum ilkesi kullanıcının yazamadığı sistem klasöründedir (`/etc/kentos-cad`,
`%ProgramData%\kentos-cad`, `/Library/Application Support/kentos-cad`; dosya `eklenti-ilkesi.json`).
`KENTOS_EXTENSIONS_DIR` ve `KENTOS_EXTENSIONS_POLICY` sınamaların ve gerçek pencere oturumlarının kullanıcının kendi
klasörlerine dokunmamasını sağlar. **Okunamayan ilke dosyası her eklentiyi reddeder** ve nedenini söyler (yokluğu
"ilke yok", bozukluğu "ilke yok" değildir).

**Döküm.** `Registry::planned_contributions` çalışan ya da hazır her eklentinin eklediğini, hiçbir şeyi etkinleştirmeden
ve kayıt defterine yazmadan hesaplar (hazır olanınki kuşak 0'lık bir kiralamayla), bırakılanları nedeniyle verir;
`kentos kek dump [--root <klasör>]` kurulu her eklentinin durumuyla birlikte JSON olarak döker: "sekmem nereye düştü?"
sorusunun kanıtı (Ö·38). Kayıt defterini açmak artık yalnız bir saatten eski hazırlık artıklarını siler: masaüstü kurulum
yaparken MCP ya da CLI kayıt defterini açarsa süren kurulumun hazırladığı dosyalar silinirdi (sınamada yakalandı).

**Masaüstü (13 Kasım, gerçek pencerede sınandı).**

- **Eklentiler penceresi** (`extensions.manage`, `apps/desktop/src/shell/extensions/`): solda liste, sağda ayrıntı;
  Kurulu ve Katalog bölümleri. Eylemler (Etkinleştir / Devre dışı bırak / Yeniden dene, önceki sürüme dön, verisini
  sıfırla, kaldır) ayrıntının **başında**: ilk sürümde katalogdaki "Gözden geçir ve kur…" düğmesi uzun ayrıntının
  altında kalıp kesiliyordu (ekran görüntüsünde görüldü). Değişiklik notu Markdown'dan okunur yazıya çevrilir (ilk
  sürüm `#` ve `##` işaretlerini olduğu gibi gösteriyordu). Durum satırı eklentinin kimliğini değil adını söyler.
- **Gözden geçirme penceresi**, yöneticinin üstünde (Esc önce onu kapatır): tür, programlar, yayımcı, lisans, kaynak,
  kurulu sürüm; uyumsuzluk, kurum ilkesi, geri çekme ve döngü uyarıları; kaynak çakışmasında açık seçim kutusu;
  bağımlılıklar; istenen her yetenek **işaret kutusuyla** (varsayılan işaretli), gerekçesi, sunucuları, dış yapay zekâ
  bilgisi ve kurulu sürüme göre `YENİ` / `DEĞİŞTİ` / "artık istemiyor"; Python katkısı varsa ayrı kutu; uyarılar;
  değişiklik notu. Geçersiz paket yalnız sorunlarıyla açılır, kurulamaz.
- **Kurum kataloğu klasörü:** pencereden seçilir (`Katalog klasörü…`) ve bir sonraki açılış için hatırlanır
  (`<kayıt>/katalog-klasoru`); `KENTOS_EXTENSIONS_CATALOG` kurulumun ya da gerçek pencere oturumunun klasörünü verir.
  İndirilen paket dizine karşı doğrulanır; dizinden sonra değişmiş dosya reddedilir.
- **Eklentisiz başlatma:** `--eklentisiz`, `KENTOS_EKLENTISIZ` ya da pencerenin "Bir sonraki açılışta eklentisiz
  başlat" kutusu (tek seferlik bayrak dosyası; o açılış siler).
- **Şerit:** her iki kipte kalıcı bir **Eklentiler** sekmesi (CAD'de Yönet'in, CBS'de Görünüm'ün yanında; Eklentiler,
  Paket yükle). Dikey bir eklentinin sekmesi (`ui.ribbonTab` verildiyse) yalnız **kendi programında** eklenir
  (kentos-cad'de programının çalışma kipinde); sekmeler kayıt defteri her değiştiğinde ondan yeniden kurulur, bu yüzden
  devre dışı bırakılan ya da kaldırılan eklenti sekme ve düğme bırakmaz; seçili sekme eklentinin sekmesiyse programın
  kendi sekmesine düşer. Düğmeler eklenti kodu gerektirmeyeni yapar: yerleşik komut (`builtin`; programın sunmadığı
  komut basılamaz), veri kaynağı, form. Python isteyen düğme görünür ama basılamaz; ipucu nedenini söyler (ADIM 18A/19).
- **Sağ tık:** `ui.contextMenu` verilmiş öğeler Katmanlar'daki katman menüsüne (`layer`) ve çizimin menüsüne
  (`drawing`) Sil'den önce, kendi çizgisiyle eklenir; eklentinin programı dışında görünmez.
- **Veri kaynağı:** WMS, WMTS, XYZ ve vektör karo Kaynaklar'ın Harita servisi penceresinde adresi, türü ve ilk katmanı
  dolu açılır; GeoPackage paketin klasöründen **kopya** olarak alınır (bağlı katman, eklenti kaldırılınca kırılırdı);
  WFS ve OGC API Features bu sürümde açılmaz, adresini söyler.
- **Form:** `Formu içe aktar` yolundan (seçili formun ya da etkin katmanın katmanına, eksik alanlar eklenerek).
- **Kaynaklar:** Servisler'in altında açık başlayan **Eklentilerden** dalı: çalışan eklentilerin (bu programın)
  veri kaynakları başlıklarıyla; tık ya da "+" haritaya ekler, sağ tık "Haritaya ekle" ve hangi eklentiden, hangi
  türde olduğu. Satırda yalnız başlık var: ilk sürüm "başlık · tür · eklenti" yazıyordu, dar panelde yazı "+" düğmesinin
  altına giriyordu (ekran görüntüsünde görüldü). Arama eklentinin adını ve türü de bulur.
- **Kabul örneği** `fixtures/extensions/v1/ornek-kent/` (Kent Rehberi): Python'suz dikey eklenti; form, şema, veri
  kaynağı ve CBS şerit sekmesi.
- **Yan düzeltme (ADIM 15A'nın açığı):** CAD Görünüm › Paneller'e eklenen Asistan grubu üç sütuna taşırıp sekmeyi
  1440 pikselde yalnız simgeye indiriyordu (`every_tab_fits_1100_pixels_without_scrolling`); Asistan'sız koşuda test
  geçti, yani neden oydu. Tam ekran (pencerenin çubuğunda da var) grubun ▾ menüsüne alındı. Ortak şerit fikstürü
  (`fixtures/shell/v1/ribbon.json`) masaüstünü izler (ADR 0622 §3); Eklentiler sekmesi oraya eklendi. Webin
  `ribbonPlan.test.ts`'i bu değişiklikten önce de fikstürden ayrı düşüyordu (web masaüstünün gerisinde).

**Ölçüm ve doğrulama:** gerçek pencerede (Xvfb, `scripts/usage/real/session.sh`, yalıtılmış kayıt defteri) KentOS
GIS'te kurum kataloğundan kur → sekme görünür → veri kaynağı ve form düğmeleri çalışır → KentOS Mimari'de sekme yok →
1.1.0'a güncelle (yeni yetenek `YENİ` işaretli) → katman menüsünde eklentinin öğesi → devre dışı bırakınca sekme ve
öğe kalkar; Kent Rehberi'nin etkinleşmesi 0,2–0,3 ms. Masaüstü sahne testleri 5 (yönetici, sekme ve program kapısı,
hayalet bırakmama, güncelleme farkı ve verilmeyen yetenek, bozuk paket, katalog), kayıt defteri 21; ilgili masaüstü
testleri (şerit, katalog, menü, komut, tuş ipuçları) 335 geçti.

**MCP: kurulu eklentiler ve değişiklik bildirimi.**

- `kentos://extensions/installed`: `kentos kek dump`'ın görünümünün **aynısı** (`Registry::dump`; CLI ile sunucu aynı
  işlevi çağırır, ikisi ayrışamaz): kurulu her eklentinin durumu ve nedeni, sürümleri, kaynağı, güveni; çalışan ya da
  hazır olanların katkıları ve bırakılanlar. `places::open_to_read` kayıt defterini kurum ilkesiyle, hiçbir şey
  etkinleştirmeden açar. Oturum özetinin `extensions` alanı artık boş değil: kimlik, ad, sürüm, durum.
- **Bildirim `notifications/resources/updated`, `tools/list_changed` değil.** Plan "eklenti etkinleşince
  `tools/list_changed`" diyordu; ama bu adımdaki eklentiler sunucuya **araç eklemiyor** (komutları Python'un, ADIM 19).
  Araç listesi değişmeden "araç listesi değişti" demek istemciyi boşuna listeyi yeniden çekmeye yollar; değişen şey
  kurulu eklentiler kaynağıdır. `tools.listChanged` eklenti komutları araç olduğunda açılacak; abonelik düzeni hazır.
- **2026-07-28:** bildirim yalnız `subscriptions/listen` akışını açmış istemciye gider; ilk mesaj
  `notifications/subscriptions/acknowledged`'dır ve istenenin **karşılanan** kısmını söyler (`resourceSubscriptions`
  içinde yalnız `kentos://extensions/installed`; `toolsListChanged` alınmaz). Her bildirim
  `_meta.io.modelcontextprotocol/subscriptionId` taşır; `notifications/cancelled` akışı kapatır. Bu sürümde
  `resources/subscribe` yoktur (-32601).
- **2025-11-25 ve öncesi:** `resources/subscribe` / `resources/unsubscribe`; başka bir kaynağa abonelik hangi kaynağın
  izlenebildiğini söyleyerek reddedilir (-32602); bildirim `notifications/initialized`'dan önce gitmez, `_meta`
  taşımaz.
- **İzleme:** sunucu iki mesaj arasında 2 saniyede bir (yalnız dinleyen varken) `registry.json`, `revocations.json` ve
  kurum ilkesinin baytlarını özetler; özet değişince görünümü yeniden okur ve **görünüm de değiştiyse** söyler: aynı
  içerikle yeniden yazılan dosya bildirim üretmez. Standart girdi bunun için ayrı bir iş parçacığında okunur
  (`main.rs`); sunucunun kendisi tek iş parçacığında kalır.
- **Önbellek ipuçları (planda yoktu):** 2026-07-28 `server/discover`, `tools/list`, `resources/list` ve `resources/read`
  sonuçlarında `ttlMs` ve `cacheScope` ister; sunucu hiç vermiyordu. Yapıya ait olan (yetenekler, araçlar, katalog) bir
  saat taze ve `public`; kullanıcının çizimini ya da eklentilerini taşıyan `0` ve `private`. Liste tek sayfadır,
  `nextCursor` yoktur.
- **Ölçüm:** gerçek ikili stdio üzerinden, karalama kayıt defteriyle sürüldü: abonelik onayı yalnız karşılananı söyledi;
  eklenti devre dışı bırakılınca bildirim **1,0 saniyede** geldi; aynı içerikle yeniden yazılan kayıt bildirim
  üretmedi; iptalden sonra sessiz; eski protokolde el sıkışmadan önce sessiz, sonra `_meta`'sız bildirim; kaynak
  2 414 bayt. Bekçi: `tests/mcp.rs` `an_agent_is_told_when_the_installed_extensions_change` (MCP testleri kural gereği
  burada koşulmaz, derlemesi denetlendi).

**Komut ara ve tuş ipuçları (13 Kasım).**

- Komut ara (`Menülerde ara…`, Alt+Q) bu programın eklenti sekmelerindeki düğmeleri bulur (`extensions/search.rs`):
  veri kaynağı, form ve Python'lu düğmeler; etiketiyle (eşit, başlayan, içeren) ya da sekmenin ve eklentinin adıyla
  (daha alt sırada). Satır yerini ve sahibini söyler ("Kent Rehberi › Altlık · Kent Rehberi eklentisi"), Python'lu
  düğme sönük ve nedeniyle. Enter düğmenin işini yapar, Alt+Enter eklentinin sekmesini açıp düğmeyi 1,6 saniye çerçeveler.
  **Yerleşik bir komuta bağlı düğme ikinci kez listelenmez**: komutun kendisi zaten bulunur ("Öznitelik tablosu" iki
  satır olurdu). Takma adı tam tutan KentOS komutu yine en üstte ("ortofoto" önce Raster ekle…'yi bulur, ORTOFOTO
  onun takma adı). Önce "“kent rehberi” menülerde yok" diyordu, sekme şeritte dururken bile.
- **Tuş ipuçları:** eklenti sekmesi harfini KentOS'un sekmelerinden **sonra**, kalanlardan alır; KentOS'un harfleri
  değişmez (ortak fikstürün ve web'in harfleri, kullanıcının alışkanlığı). Kent Rehberi "K", düğmeleri O2, B, OT.
  `assign_key_tips` için iki kural eklendi, ikisi de var olan durumları değiştirmez (ortak fikstürün 7 durumu aynı
  sonucu verir): ayrılmış tek **harf** yeni ipucunun başı olamaz (rakam olabilir: fikstürdeki `1A`), yeni tek harf
  ayrılmış bir ipucunun başı olamaz; yoksa yazılan harf belirsiz kalırdı.
- Plan "kısayollar `ext.<id>.*` kimliklerini bulur" diyordu: masaüstünde kullanıcının tuş ataması yok (Kısayollar
  penceresi kataloğun sabit tuşlarını listeler); eklentinin klavye yolu tuş ipuçları ve Komut ara.

**Paketin kendi ikonları (13 Kasım).** Manifestin ikon alanları ya yerleşik bir ikonun adı ya da paketteki 20×20 SVG
dosyasıdır; dosya olanlar kurulumda denetlenir (`kentos_extensions::icon`: izinli öğe ve öznitelikler, betik, bağlantı,
renk yok), paketin dosyaları her etkinleşmede içerik özetine karşı doğrulanır. Bu yüzden masaüstü onları doğrudan
çizer (`Icon::Svg`, temanın rengiyle): şerit düğmesi, sağ tık öğesi, komut ve harita aracı ikonu; yöneticinin kurulu ve
katalog listelerinde adın yanında paketin ikonu (ikonsuz pakete yapboz). Önce paket dosyası olan ikon yerine genel bir
simge çiziliyordu. Paket ikonları kayıt defteri değişince bir kez okunur (`Extensions::icons`), her çizimde değil.
**Katalog dizinindeki ikon okunurken de denetlenir** (`Catalog::read`): dizin paylaşımlı klasörde durur ve elle
değiştirilebilir; kurulumun reddedeceği ikon çizilmez (sınamada `<script>`'li ikon). Kent Rehberi örneğine paket
ikonu ve form düğmesinin ikonu eklendi. Yan düzeltme: liste sütunları ikonla daraldı ("Kent Rehberi" "Ken / Reh" diye
kesiliyordu, gerçek pencerede görüldü); liste 300 → 380 piksel.

**`.kcad` bağlantısı ADIM 22'ye (planda ADIM 18'deydi).** Dosya biçiminde eklentinin yeri `modules.ext`: **eklentinin
belgeye yazdığı verinin sürümü**. Bu adımdaki Python'suz katkılar belgeye kendine ait veri yazmaz: alınan form ve
eklenen servis katmanı sıradan belge içeriği olur, eklenti kaldırılsa da açılır. Eklentiye ait belge verisi ilk kez
ADIM 22'nin katman şemasıyla gelir; `project_needs` ve kısıtlı açılış (D16) o veriye bağlanır, manifestin veri
sürümü de orada tanımlanır. Bugün yapılan: `modules.ext` taşıyan dosyanın ret iletisi "Eklentiyi kurup … yeniden
açın" diyordu, oysa hiçbir KentOS eklenti verisini henüz okumuyor; ileti "bu KentOS eklenti verisi taşıyan dosyaları
henüz açamıyor; KentOS'u güncelleyin" oldu.

**Görünümler, arama ve program süzgeci (D1, 13 Kasım).** Yöneticinin bölümleri **Kurulu · Bulunabilir · Güncellenebilir
· Yeni · Sorunlu**; sayısı olan "Güncellenebilir (1)" diye yazılır (`extensions/browse.rs`). Arama ad, özet, açıklama,
anahtar sözcük, kimlik ve yayımcıda (Türkçe harfler katlanır); program süzgeci "Tüm programlar" ile başlar (bu programla
başlasaydı başka programın kurulu eklentisi "kurulu listede yok" görünürdü). **Yeni**, kataloğun bir önceki açılışından
beri gelendir: görülenler `<kayıt>/katalog-gorulen.json`'da, oturum boyunca liste değişmez (ilk açılışta hepsi yeni).
Sorunlu, durum çubuğu hücresinin listesidir. Katalog klasörü açılınca güncelleme varsa Güncellenebilir, yoksa
Bulunabilir açılır; boş liste neden boş olduğunu söyler ("Süzgeçle eşleşen eklenti yok.", "Güncellenecek eklenti yok."
…). Sınama: sayılar, arama, iki program süzgeci, katalogda yeni sürüm (Güncellenebilir 1, Yeni yalnız o), Sorunlu.

**`.kek` dosya ilişkilendirmesi (PY-KEK-15).** `kentos-cad paket.kek` (çift tık) paketi çizim gibi açmaz: açılışta
Eklentiler'in üstünde **inceleme penceresi** gelir; açmak hiçbir şey kurmaz, çalıştırmaz, Kur kurar. Linux için
`apps/desktop/packaging/linux/kentos-mime.xml` (`application/x-kentos-kek`, `*.kek`; depoda hiç olmayan
`application/x-kentos-kcad` tanımı da) ve `kentos-eklenti.desktop` (`NoDisplay`). Windows ve macOS paketlemesi depoda
yok; dosya ilişkisi orada kurulumla gelir.

**Yan düzeltme: `--eklentisiz` bir dosya sanılıyordu.** `kentos-launch`'ın komut satırı onu tanımıyordu: `kentos-cad
--modul cbs --eklentisiz` eklentisiz başlıyordu ama "--eklentisiz" adlı dosyayı da açmaya kalkıyordu. Bayrak artık komut
satırının parçası (`Launch.safe`, `SAFE_FLAG`). Gerçek pencerede `--modul cbs --eklentisiz <paket.kek>`: eklentisiz
açılış ve paketin incelemesi, dosya hatası yok. Aynı crate'teki `refusals_say_why` bu değişiklikten önce de kırıktı ("yol"
artık bir modül); bilinmeyen bir adla düzeltildi.

**ADIM 18 bitti.** ADIM 22'ye taşınanlar: katman şemasının projeye kurulması (biçimi orada sabitlenir) ve `.kcad`
bağlantısı (yukarıda). Hata, günlük ve "Sorunlu eklentiyi bul": §4.

## §4. Hata ve günlük (`ADIM 18`, sahibin isteği)

**Bağlam.** Sahip (13 Kasım): "Hata işleme ve log mekanizması da düzgün olsun eklenti sisteminde", "Eklenti hata
verdiğinde açıkça bilelim sorunu". Ölçülen durum: kayıt defterinin günlüğü `registry.json` içinde en çok 50 satırlık,
seviyesiz, kodsuz bir metin listesiydi; masaüstünde eklentinin veri kaynağı ya da formu açılamayınca **hiçbir şey olmuyordu**
(beş yerde sessiz `return`), olan hata eklentiye bağlanmıyordu; açılışta çalışmayan eklenti durum çubuğunda birkaç saniye
görünüp kayboluyordu ve kimliğiyle ("“tr.ornek…” etkinleştirilemedi") söyleniyordu; bütünlük hatası hangi dosyanın
değiştiğini söylemiyordu.

**Günlük** (`kentos_extension_host::journal`).

- Kayıt: zaman (ms, UTC), seviye (Bilgi / Uyarı / Hata), **sabit kod**, eklenti, o anki sürüm, parça (`etkinleştirme`,
  `veri kaynağı “Ortofoto (2026)”`, `form “bina-denetimi”`, `kurulum`, `katalog`, `başlangıç`), kullanıcının okuyacağı
  cümle, hatanın kendi metni (`detail`: G/Ç hatası, sunucunun yanıtı; ADIM 19'da Python izi) ve yazan program
  (`kentos-cad`, `kentos`, `kentos-mcp`).
- Dosya: `<kayıt>/logs/<kimlik>.jsonl`, eklentiye ait olmayan için `registry.jsonl`; JSON Lines, satır başına **tek
  yazma** (masaüstü, CLI ve MCP yan yana yazar). 256 KiB'da `.1.jsonl`'e devrilir: eklenti başına en çok ~512 KiB.
  Kaldırılan eklentinin günlüğü kalır; yeniden kurulursa geçmişi sürer.
- **Gizli değer günlüğe girmez:** URL'deki `token`/`key`/`apikey`/`signature`…, `kullanıcı:parola@sunucu`'daki parola,
  bağlantı dizesindeki `password=`, `Authorization` başlığı (şeması kalır: `Bearer ***`), JSON'daki `"apiKey": "…"`
  yazılmadan önce `***` olur. Düzenli ifade kitaplığı eklenmedi (CLAUDE.md §3), tarayıcı elle yazıldı.
- Günlük yazılamazsa işlem düşmez; standart hataya bir satır düşer.
- **Duran hata** (`problem`): en yeni hata, ardından yeni dosya (kurulum, güncelleme, yeniden kurulum, sürüm değiştirme)
  ya da Yeniden dene gelmediyse ve **aynı parça sonradan çalışmadıysa** (kaynağa bağlanıldı, form alındı, etkinleşme
  oldu). İlk sürümde yalnız yeni dosya ve yeniden deneme temizliyordu: geçici bir bağlantı hatası, kaynak sonra
  bağlansa da sonsuza dek "duran hata" kalırdı (gerçek pencerede görüldü).
- Açılışta etkinleşemeyen eklentinin durumu (`not_started_*`) **değişene kadar bir kez** yazılır, her açılışta değil.
- `registry.json`'ın eski kısa geçmişi açılışta (ve her yazmada) günlüğe `earlier` koduyla taşınır, dosyaya bir daha
  yazılmaz; günlüğü olan eklentide iki kez taşınmaz.
- Pencere her çizimde dosya okumasın diye kayıt defteri her eklentinin son **50** kaydını açılışta okur, yazdıkça
  bellekte de günceller (`Registry::log_of`).

| Kod | Seviye | Ne |
|---|---|---|
| `installed` / `updated` / `reinstalled` / `switched` / `retried` | Bilgi | Yeni dosya ya da yeniden deneme: eski hata kapanır |
| `activated` | Bilgi | Etkinleşti, süresiyle |
| `enabled` / `disabled` / `grants_changed` / `data_reset` / `removed` | Bilgi | Kullanıcının eylemi |
| `integrity_mismatch` | Hata | Kurulu dosyalar değişmiş; **hangi dosya**, nasıl (değişmiş / eksik / sonradan eklenmiş) |
| `manifest_unreadable` | Hata | Kurulu sürümün manifesti okunamadı |
| `crashed` / `failed` | Hata | Çöktü; art arda 3'te durduruldu (bekçi) |
| `not_started_failed` / `_blocked` / `_incompatible` / `_waiting` | Uyarı | Etkin ama açılışta çalıştırılmadı, nedeni |
| `install_refused` | Uyarı | Paket ya da güncelleme kurulmadı (çalışan sürüm etkilenmez) |
| `catalog_mismatch` | Uyarı | Katalog klasöründeki dosya dizinle tutmuyor |
| `revoked` / `stopped_dependency` | Uyarı | Sürüm geri çekildi; bağlı olduğu eklenti durdu |
| `source_unreachable` / `source_add_failed` / `source_missing` / `source_file_missing` / `source_incomplete` | Hata | Veri kaynağı |
| `form_missing` / `form_file_missing` / `form_unreadable` / `form_invalid` | Hata | Form |
| `source_connected` / `form_imported` | Bilgi | Parça yeniden çalıştı: o parçanın hatası kapanır |
| `registry_corrupt` / `registry_newer` | Hata | Kayıt defteri okunamadı (`registry.jsonl`, değişene kadar bir kez) |

**Masaüstü: hata açıkça söylenir.**

- **Anında:** `App::extension_failed` hatayı Uyarılar'a ve durum çubuğuna **eklentinin adıyla, parçasıyla, nedeniyle ve
  nereye bakılacağıyla** yazar: "Kent Rehberi eklentisi hata verdi — veri kaynağı “Ortofoto (2026)”: Servise
  bağlanılamadı… [harita.ornek.bel.tr sunucusuna bağlanılamadı…] Ayrıntı ve rapor: Eklentiler › Kent Rehberi." Yönetici
  bir sonraki açılışta o eklentide açılır.
- **Kalıcı:** durum çubuğunda kırmızı **"Eklenti hatası: Kent Rehberi"** hücresi (Eski kesit rozetinin kalıbı); duran
  bir hata ya da etkin ama çalışmayan bir eklenti oldukça kalır, ipucunda her birinin nedeni, tıklanınca yönetici o
  eklentide açılır. Eklentisiz açılışta görünmez (bilerek hiçbir şey çalışmıyor). Uyarılar rozeti yalnız alt panel
  açıkken görünüyordu: açılış hatası bu hücre olmadan kaçırılabiliyordu (gerçek pencerede görüldü).
- **Açılış:** çalıştırılamayan eklenti **Hata** seviyesinde, adıyla ve nedeniyle ("Kent Rehberi eklentisi çalıştırılmadı:
  Kurulu dosyalar kurulduğu hâlinden farklı: forms/bina_denetimi.kform değişmiş. Eklentiyi yeniden kurun ya da önceki
  sürüme dönün."); önceki bir açılıştan kalan durumu olan her açılışta tek satır **Uyarı**: "Çalışmayan eklenti: Kent
  Rehberi (Hata)."
- **Yönetici:** duran hata ayrıntının **en üstünde** kırmızı kutuda (parça, cümle, "Raporu kopyala"), altında zaman ·
  seviye · parça · kod · sürüm ve hatanın kendisi; listede satır kırmızı (hata) ya da sarı (engellendi, uyumsuz,
  bekliyor) işaretli; Günlük bölümü seviye simgeli son 8 kayıt, "Tüm günlük", günlük dosyasının yolu.
- **Rapor** (Markdown, panoya): eklenti, kimlik, durum, yayımcı, kaynak, güven, sürümler, program ve işletim sistemi,
  günlük dosyası, duran hata (hatanın kendisi kod bloğunda) ve son 30 kayıt: yayımcıya ya da bir ajana yapıştırılır.
- **Servis penceresi** eklentinin kaynağını açınca hangi eklentiden geldiğini söyler; bağlanamama ve katmanı
  ekleyememe eklentinin hatası olarak yazılır, bağlanma hatayı kapatır.
- Eklentinin hatası olmayan durum ayrı tutulur: WFS / OGC API Features bu sürümde açılmıyor (Bilgi, `source_not_supported`);
  çizim açık değilken form düğmesi "önce bir çizim açın" der.
- Durum yardımlarındaki "yazarına bildirin" "yayımcısına bildirin" oldu: pencerede alanın adı "Yayımcı".

**CLI:** `kentos kek log [<kimlik>] [--son <n>] [--root <klasör>] [--json]`: kayıtlar eskiden yeniye, zaman UTC
(`2026-11-13 15:54:55Z`), hatanın kendisi girintili, sonda "Duran hata"; kimliksiz kayıt defterinin günlüğü; `--json`
JSON Lines.

**MCP:** `kentos://extensions/installed` her eklentinin `problem`'ini ve son kayıtlarını taşır (aynı `Registry::dump`);
izleme `logs/` dosyalarının boy ve zamanını da özetler, yeni bir hata da izleyene `notifications/resources/updated` olur.

**Ölçüm ve doğrulama (13 Kasım).**

- Gerçek pencere (Xvfb, yalıtılmış kayıt defteri, KentOS GIS): Kent Rehberi'nin Ortofoto kaynağı → durum çubuğu ve
  Uyarılar'da adıyla hata; servis penceresinde "Kent Rehberi eklentisinin veri kaynağı: Ortofoto (2026)."; yöneticide
  duran hata kutusu. Paketin form dosyası değiştirildi → açılışta eklenti çalışmadı, durum çubuğunda kırmızı hücre
  kaldı, tıklanınca yönetici Kent Rehberi'nde, kutu "forms/bina_denetimi.kform değişmiş" diyor.
- MCP aynı kayıt defterinde: kaynak 5 470 bayt, `problem` `integrity_mismatch`, 19 kayıt.
- Sınamalar: günlük 3 (maskeleme 8 durum, devir ve bozuk satır, duran hata kuralı), kayıt defteri 27 (+3: hata
  zinciri, bozuk kayıt defteri bir kez, eski geçmişin taşınması), masaüstü eklenti sahneleri 6 (+1: adıyla bildirim,
  hücre, yönetici, rapor, düzelince kapanma, açılış hatası, eklentisiz açılış), ilgili masaüstü 343, CLI 1, eklenti
  paketi 52 geçti.

**Sorunlu eklentiyi bul (D6).** Bir şey ters gidip hangi eklentiden olduğu belli değilken: yöneticinin altındaki
"Sorunlu eklentiyi bul…" (`extensions/bisect.rs`). Çalışan eklentiler şüphelidir; ilk tur **hiçbirini** açmaz (sorun
sürerse eklentilerden değildir), sonra şüphelilerin yarısı (`watchdog::Bisection`, en çok 1 + ⌈log₂ n⌉ tur).

- **Kullanıcının seçimi değişmez:** turun kapattığı eklenti yeni `State::Held`'dir ("Sorun bulmada kapalı");
  `registry.json`'daki `enabled` dokunulmaz. Bitince ya da "Bırak"ta hepsi eskisi gibi açılır (sınamada dosyadan
  okunarak denetlendi).
- **Yeniden başlatmaya dayanır:** tur `<kayıt>/sorun-bulma.json`'da durur; sorun açılışta görülüyorsa kullanıcı
  programı kapatıp açar, açılış turun kapattıklarını çalıştırmaz ve soruyu Eklentiler penceresinde yeniden sorar
  (gerçek pencerede: yeniden açılış → "Tur 1" sorusu kendiliğinden).
- Pencere: tanıtım (çalışan eklentiler, en çok kaç tur, seçimlerin değişmediği), tur ("Tur 2 — en çok 1 tur kaldı",
  bu turda açık ve kapalı olanlar, "Sorun hâlâ görülüyor mu?": Evet, sürüyor / Hayır, geçti / Bırak), sonuç ("Sorun büyük
  olasılıkla “Kent Rehberi” eklentisinden… (2 tur)": Raporu kopyala / Açık kalsın / Devre dışı bırak; ya da "eklentilerden
  değil"). Pencere kapatılırsa sorun bulma sürer; yöneticinin üstünde "Sorunlu eklentiyi bulma sürüyor (tur N)… Soruya
  dön" satırı kalır. Esc önce bu pencereyi kapatır.
- Bulunan eklenti günlüğüne Hata olarak yazılır (`bisect_found`, parça `sorun bulma`): durum çubuğu hücresi ve
  yöneticinin hata kutusu onu söyler. Kayıt defterinin günlüğünde `bisect_started` / `bisect_not_extension` /
  `bisect_stopped`.
- Durum çubuğu hücresi artık yalnız **etkin** eklentileri sayar: kullanıcının kendisinin kapattığı bir eklentinin eski
  hatası hücreyi sonsuza dek açık tutardı. "Sorun bulmada kapalı" bir sorun sayılmaz.
- MCP izlemesi `sorun-bulma.json`'ı da özetler: turun kapattıkları ajana da bildirilir.
- Ölçüm: gerçek pencerede iki eklentiyle (Örnek Belediyesi altlıkları, Kent Rehberi) tanıtım → Başla → tur 1 (ikisi de
  kapalı, Kent Rehberi sekmesi şeritten kalktı) → programı kapatıp açma → soru yeniden → Hayır → tur 2 (yalnız Örnek
  altlıklar açık) → Hayır → "Kent Rehberi" (2 tur); ikisi de yeniden etkin, sekme geri geldi, `sorun-bulma.json` silindi.
  Sınamalar: kayıt defteri 28 (+1: üç eklentide suçlu, yeniden açılışta süreklilik, seçimlerin dosyada aynı kalması,
  eklentilerden değil, bırakma, çalışan yokken ret), eklenti sahneleri 9 (+1).

**Uygularken verilen kararlar (planda yazılı değildi):** günlük `registry.json`'dan ayrı dosyalarda (kayıt defteri
yazmak kilit ister ve her satırda bütün dosyayı yeniden yazardı); durum çubuğu hücresi; "aynı parça çalıştı" kuralı;
açılışta çalışmayanlar için her açılışta tek satır uyarı; kurulmayan paket eklentinin günlüğüne Uyarı olarak (çalışan
sürüm iyi); bellekte 50 kayıt; eski kısa geçmişin taşınması; CLI'de zaman UTC; "yazarına" → "yayımcısına".

## §5. Python ve uv: kullanıcının seçtiği isteğe bağlı ortam (`ADIM 18A`)

**İlk dilim (13 Kasım): seçim, uv'nin bulunması, Windows.** Yeni crate `crates/native/python-env`
(`kentos-python-env`; `deps.mjs` grubu `python-env`, bağımlılığı yalnız serde): uyumluluk manifesti, uv'nin algılanması ve
çalıştırılması, kullanıcının seçimi.

- **Varsayılan kapalı (P01, P02).** Uygulama ayarları › **Python ve eklentiler**: "Python ve Python'lu eklentileri
  kullan". Kapalıyken uv ve Python **aranmaz**, hiçbir şey indirilmez; CAD, CBS ve KentOS'un araçları çalışır.
  Eklentinin Python düğmesi görünür, basılamaz ve nedenini söyler: "Python kapalı: Uygulama ayarları › Python ve
  eklentiler'den açabilirsiniz." / "uv hazır değil…" / "Python hazır; yürütücü sonraki sürümde" (ADIM 19). Seçim
  kullanıcınındır (`<python>/ayarlar.json`), Yapay zekâ'nın ajanları gibi taslaktan ayrı, hemen yazılır; okunamayan
  dosya "kapalı" sayılır.
- **Tek uyumluluk manifesti** (`crates/native/python-env/compat.json`, PY-UV-09): uv en az 0.9.0, Python `>=3.10,<3.15`,
  desteklenen platformlar, sistem başına uv'nin resmî kurulum yolları. Okunamayan gömülü manifest her şeyi reddeder.
- **uv bulunur (PY-UV-01, 02):** seçilen yol (verildiyse yalnız o), `PATH`, sonra menüden başlayan programın `PATH`'inde
  çoğu zaman olmayan kullanıcı klasörleri. Sürüm sorulur (`uv --version`), manifestle karşılaştırılır: bulundu / çok eski
  ("`uv self update`") / çalıştırılamıyor / bulunamadı / bu sistem desteklenmiyor; her biri ne yapılacağını söyler.
  **Algılama arka planda** (ayrı iş parçacığı): pencere uv'yi beklemez. uv kendiliğinden değiştirilmez.
- **KentOS uv kurmaz (PY-UV-03…06):** bulunamazsa sistemin resmî komutları her biri kendi "Kopyala" düğmesiyle
  gösterilir; kullanıcı kendisi çalıştırır, sonra Yeniden algıla.
- **uv'yi çalıştırmak (PY-UV-22, 24):** argv ile, kabuk yok (sınamada `a b; rm -rf /` tek bağımsız değişken kaldı);
  süre sınırı (aşınca durdurulur), akış başına 256 KiB çıktı; ortamdaki `UV_*`, `VIRTUAL_ENV`, `PYTHONPATH`,
  `PYTHONHOME`, `CONDA_PREFIX`, `PYTHONSTARTUP` çıkarılır: etkin bir venv ya da kullanıcının uv ayarı nereye kurulacağına
  karar vermez. Durdurulan uv'nin alt süreci boruyu açık tutarsa çıktı en çok 2 sn beklenir (yoksa iş parçacığı
  sonsuza dek beklerdi).
- **Windows (sahibin notu, 13 Kasım: "Windows olabilir sistem, ona göre yap"):**
  - uv `CREATE_NO_WINDOW` ile başlar: pencereli bir programdan başlayan konsol programı her çağrıda siyah bir pencere
    açıp kapatırdı.
  - Aranan yerler `uv.exe` adıyla: resmî kurulumcunun `%USERPROFILE%\.local\bin`'i, `.cargo\bin`, WinGet'in
    `%LOCALAPPDATA%\Microsoft\WinGet\Links`'i, Scoop'un `%USERPROFILE%\scoop\shims`'i. Açık bir Windows programı,
    kurulumcunun sonradan değiştirdiği `PATH`'i görmez; bu klasörler sayesinde yeniden başlatmadan bulunur.
  - Kurulum yolu iki: PowerShell kurulumcusu ve **WinGet** (`winget install --id=astral-sh.uv -e`): kurum
    bilgisayarlarında BT `-ExecutionPolicy ByPass`'ı çoğu zaman kapatır.
  - Ortamlar **`%LOCALAPPDATA%`**'da (`%APPDATA%` değil): gigabaytlarca tutarlar ve o bilgisayarındır; kurum etki
    alanının gezici profili onları her oturum açışta kopyalardı. (Eklenti kayıt defteri `%APPDATA%`'da kalır: paketler
    küçük, kullanıcının eklentileri onunla dolaşır; Python ortamı yeni bilgisayarda yeniden hazırlanır.)
  - Windows hedefi (`x86_64-pc-windows-msvc`) geliştirme makinesine eklendi: `kentos-python-env`, `-extension-host`,
    `-extensions`, `-launch` sınamalarıyla Windows için derleme denetiminden geçiyor (depoda CI yok; masaüstünün tamamı
    Windows için ayrıca denetlenmedi).
- **Ölçüm:** gerçek pencerede kapalı → aç → "uv 0.11.26 bulundu" (arka planda, pencere beklemedi); yanlış yol →
  "uv bulunamadı (1 yere bakıldı)", resmî komut ve Kopyala. Sınamalar: python-env 5 (manifest, Windows ve
  masaüstü oturumu arama yerleri, sürüm denetimi sahte uv'yle, kabuksuz/süreli/sınırlı çalıştırma, seçim), masaüstü 1
  (düğmenin nedeni seçime göre, hemen yazılan seçim, bölümün çizimi).

**İkinci dilim (13 Kasım): kurulumdaki uv.** Sahip "Python gömülü mü yoksa kuruluyor mu" diye sordu; üç yol önerildi
(uv kurulumda · uv ve Python kurulumda · uv'yi kullanıcı/BT kurar) ve karar: **"UV kurulum içine eklensin senin önerdiğin
yolu seçtim."** Gerekçe: kurum bilgisayarlarında kullanıcı komut çalıştıramayabilir, BT tek bir kurulumu onaylar. Python
kuruluma da KentOS'un sürecine de girmez: kullanıcı onaylayınca uv onu kullanıcının klasörüne indirir (sonraki dilim).

- **Arama sırası:** elle verilen yol (yalnız o) › **kurulumun kendi uv'si** › `PATH` › kullanıcı klasörleri. Kurulumun
  uv'si `tools/uv[.exe]`, programın yanında; macOS uygulama paketinde `Contents/MacOS/uv`, programla **aynı düzeyde**.
  Apple'ın "Placing content in a bundle" belgesi yardımcı aracı `Contents/MacOS/`'a koyar; `Resources` kod için değil, iç
  içe klasördeki kod imzalamayı bozar. (İlk taslakta `Resources/tools` vardı; belgeyle düzeltildi.) Programın kendi
  klasörü yalnız uygulama paketinin içindeyse "KentOS'un" sayılır: `/usr/bin`'deki bir programın yanındaki `/usr/bin/uv`
  kullanıcınındır (`bundled_dirs_for`).
- **Pencere ne bulduğunu söyler:** "KentOS'la gelen uv 0.11.26 hazır (…/tools/uv)." Bulunamazsa ileti gerçekten bakılan
  yerlere göre kurulur: kendi klasörü de bakılanlar arasındaysa "KentOS'la gelen uv silinmiş olabilir: KentOS'u yeniden
  kurun ya da resmî komutu kendiniz çalıştırın"; yalnız elle verilen yola bakıldıysa "uv verilen yolda yok: … Yolu
  düzeltin ya da silip Uygula'ya basın: boş bırakılınca KentOS önce kendi uv'sini kullanır." İkincisinde kurulum
  komutları **gösterilmez** (`UvStatus::offers_install`): çıkış yolu uv kurmak değil, yolu silmek.
- **Manifest** (`bundledUv`): sürüm 0.11.26, resmî sürüm klasörü, 6 hedefin dosyası ve SHA-256'sı (sürümün
  `dist-manifest.json`'ından), iki lisansın etiketli adresi ve SHA-256'sı. Sürüm arşivlerinde lisans dosyası **yok**
  (ölçüldü: tar'da `uv`, `uvx`; zip'te `uv.exe`, `uvx.exe`, `uvw.exe`); lisanslar `0.11.26` etiketinden ayrı alınır.
  Sınama: her desteklenen platformun bir dosyası var, özetler 64 küçük onaltılık, gömülü sürüm en düşük sürümden yeni
  (yoksa kurulumun uv'si kendi kendini reddederdi).
- **Paketleme** `scripts/desktop/bundle_uv.py HEDEF ÇIKTI [--cache KLASÖR] [--offline] [--list]`: manifesti okur,
  arşivi indirir, **açmadan önce** SHA-256'yı denetler; arşivden yalnız `uv`/`uv.exe` üyesini okur (toplu çıkarma yok),
  atomik yazar, lisansları `uv-LICENSE-MIT` / `uv-LICENSE-APACHE` adıyla yanına koyar; hedef bu bilgisayarsa
  `uv --version` çalıştırıp sürümü doğrular. `--offline` internetsiz derleme makinesi içindir: önbellekte olmayan ya da
  özeti tutmayan dosya kullanılmaz. macOS'ta `bundle_macos.py`'den sonra çalışır, uv paketten önce imzalanır.
- **Boyut (ölçüldü):** Linux x86_64 arşivi 25,6 MB → uv 63,9 MB; Windows x86_64 zip'i 24,9 MB → `uv.exe` 74,0 MB; macOS
  aarch64 arşivi 22,0 MB → uv 50,7 MB. Kurulum açılmış hâlde 51–74 MB büyür (sıkıştırılmış kurulum dosyasında ~22–25 MB).
  Sahibe ilk söylenen "yaklaşık 40 MB" eksikti.
- **Ölçüm:** çevrimiçi koşu indirdi, özeti denetledi, yerleştirdi, `uv 0.11.26 (x86_64-unknown-linux-gnu)` çalıştı;
  aynısı önbellekten `--offline`; önbellekte zip yokken `--offline` adresini söyleyerek reddetti; sonuna bir bayt eklenen
  arşiv "does not match its SHA-256" ile reddedildi, hiçbir şey yazılmadı; Windows ve macOS hedefleri yerleşti (bu
  makinede çalıştırılmadı). Gerçek pencere: `target/debug/tools/uv` ile "KentOS'la gelen uv 0.11.26 hazır"; yanlış elle
  yol yeni cümleyle, kurulum komutu olmadan. Sınamalar: python-env 5 (arama sırası, uygulama paketi kuralı, iki
  "bulunamadı" iletisi, `bundled` işareti, manifestin hedefleri), masaüstü eklenti sahneleri 13.
- **Uygularken verilen kararlar:** elle verilen yol kurulumun uv'sinden önce gelir (BT belirli bir uv'yi gösterebilir;
  alan boşken kurulumun uv'si ilk); kurulumun uv'si eski çıkamaz, sınama gömülü sürümü en düşük sürüme karşı denetler;
  `uvx`/`uvw` kuruluma konmaz; lisans adresleri de özetle sabitlendi.
- **Yapılmadı:** depoda Windows kurulum paketi (MSI/NSIS) henüz yok; betik, o paketin çağıracağı adım. uv'nin kendi
  bağımlılıklarının bildirimleri Astral'ın arşivinde de yok; KentOS'un toplu üçüncü taraf bildirimi ayrı iş.

**Üçüncü dilim (13 Kasım): KentOS'un Python'u — onayla kurmak, ilerleme, kaldırmak.** Sahibin bu dilimdeki istekleri:
**"Aynı yerden kaldırılabilmesi Python ve hoş bir progress ve toast ile geri bildirim"**, ardından
**"Kaldırılabilmelidir"**.

- **Onay düğmedir (P03).** uv hazırken KentOS'un Python'u kullanıcının klasöründe aranır (`<python>/yorumlayici`). Yoksa
  bölüm ne indirileceğini (Astral'ın python-build-standalone derlemesi, GitHub), boyunu ("yaklaşık 35 MB; kurulunca
  100 MB kadar"), nereye kurulacağını ve neye dokunulmayacağını söyler; "Python 3.12'yi indir ve kur"a basılmadıkça
  hiçbir şey inmez. 3.12: CBS paketlerinin tekerlekleri en geniş onda, güvenlik desteği Ekim 2028'e dek (manifestte
  `python.install`).
- **Sistemde iz yok:** `uv python install 3.12 --no-bin --no-registry --no-config --no-progress --system-certs`;
  `UV_PYTHON_INSTALL_DIR` ve `UV_CACHE_DIR` KentOS'un klasörleri. `--no-bin` olmasa uv 0.8'den beri `python3.12`'yi
  `~/.local/bin`'e koyar, `--no-registry` olmasa Windows kayıt defterine yazar (PEP 514). Ölçüldü: `~/.local/bin`'e
  hiçbir şey eklenmedi. `--system-certs`: TLS denetleyen kurum ağında kurumun sertifikası sistemin deposundadır, uv'nin
  kendi listesinde değil.
- **Bulmak** `uv python list --only-installed --managed-python --output-format json`, KentOS'un klasöründe çalışarak:
  yanındaki bir `.venv` ya da kullanıcının kendi uv Python'ları sayılmaz (ölçüldü). **Ölçülen tuzak:** uv, çalıştığı
  klasörün altındaki yolu **göreli** yazıyor (`yorumlayici/cpython-3.12-…/bin/python3.12`); klasöre göre çözülür.
- **İlerleme çubuğu** (sahibin isteği): uv başlarken arşivin boyunu yazar (`Downloading cpython-… (32.6MiB)`) ve
  `UV_PYTHON_CACHE_DIR` verilince arşiv orada **tek dosya olarak büyür** (ölçüldü: 0 → 34.159.222 bayt, sonra
  `Extracting`). İkisi indirilen payı verir: "İndiriliyor: 13,2 MB / 32,6 MB · %40". Açma aşamasının ölçüsü yok, çubuk
  kayar ("Açılıyor ve yerine konuyor…"). Bunun için uv çağrısı izlenebilir oldu (`uv::run_watched`, `Watch`: Durdur
  bayrağı, akarken okunan hata akışı, 150 ms'de bir çağrılan izleyici). Her kurulumun indirme klasörü kendine özel
  (`indirme/<süreç>-<an>`) ve nasıl biterse bitsin silinir: arşiv saklanmaz, iki KentOS penceresi aynı anda kurarsa
  birbirinin dosyasını silmez.
- **Durdur** uv'yi hemen sonlandırır. **Hata** cümlesiyle söylenir (internet yok · sertifika · disk dolu · klasöre
  yazılamıyor · bu sisteme derleme yok · 20 dakika sınırı), yanında uv'nin kendi sözü (ilk `error:` ve son
  `Caused by:`). Sertifika önce denetlenir: uv sertifika hatasını da "indirilemedi" diye yazıyor.
- **Kaldır** (sahibin isteği) önce sorar, açılacak yeri söyleyerek ("Python 3.12.13 kaldırılsın mı? 93 MB yer açılır…",
  kırmızı "Evet, kaldır" ve Vazgeç). Sonra `uv python uninstall --all` ve klasör uv'nin bıraktıklarıyla (`.lock`,
  `.temp`) birlikte silinir; kullanıcının seçimi (`ayarlar.json`) kalır. Windows'ta çalışan bir Python'un dosyaları
  silinemez: "dosyaları kullanılıyor; Python konsolunu ve Python'lu eklentileri kapatıp yeniden deneyin".
- **Toast** (sahibin isteği): "Python 3.12 kuruldu" · "Python kurulumu durduruldu" · "Python kurulamadı" (kendiliğinden
  kapanmaz) · "Python kaldırıldı — 93 MB yer açıldı" · "Python kaldırılamadı". Pencerenin mevcut bildirim yığını
  (`Toaster`); ayarlar kapalıyken de görünür. Hata ayrıca bölümde kalır.
- **Ölçüm:** gerçek pencerede, programa yalnız onun için verilen ve hızı 1,5 MB/sn'ye düşüren yerel bir vekille:
  2,5 sn'de %12, 8,5 sn'de %40; ~26 sn'de "kuruldu" bildirimi ve "Python 3.12.13 hazır · 93 MB"; Kaldır → onay →
  "93 MB yer açıldı" ve kurulmamış duruma dönüş. Durdur 3,5 sn'de: bildirim, indirme klasörü silinmiş. Ulaşılamayan
  vekille: "internete ulaşılamadı … Connection refused (os error 111)". Hızlı bağlantıda kurulum 2 sn (32,6 MiB
  indirme, kurulu 93 MB).
- **Ölçülen yan bulgu — `ETXTBSY`:** sınamalar sahte uv'yi yazıp hemen başlatıyor; başka bir iş parçacığının o an
  başlattığı süreç dosyanın yazma tanıtıcısını exec'ine dek miras alıyor ve başlatma "Text file busy (os error 26)" ile
  düşüyordu: sekiz iş parçacığıyla 25 koşuda 8 hata. `uv::spawn` bu hatada 25 ms arayla 8 kez yeniden dener; sonra 25
  koşuda 0.
- **Sınamalar:** python-env 10 (bulma, kurma bayrakları, ilerleme okuma, Durdur, hatanın cümlesi, kaldırma ve artıkları,
  bağın iki kez sayılmaması), masaüstü eklenti sahneleri 13 (onay, ilerleme yazısı, bildirimler, onaylı kaldırma,
  kaldırırken kurulamaması).

**Dördüncü dilim (13 Kasım): kişisel ortam ve paketler (P04), Python konsolu.** Sahibin istekleri:
**"Paketler de kaldırılabilmeli listelenebilmeli aranabilmeli listeden seçilerek kurulabilmeli UV harika bir araç ve arayüz
ile kolaylaştır"**; sorusu **"Sadece bu listedeki paketlerle sınırlamıyoruz değil mi"**: hayır, önerilen liste bir başlangıç,
paket dizinindeki her paket adıyla istenir.

- **KentOS'un Python kitaplığı kurulumda** (PY-UV-31): konsol `python -m kentos.host` çalıştırıyor, yani ortamda `kentos`
  paketi olmalı. `scripts/desktop/bundle_sdk.py` tekerleği maturin ile derler (ya da `--wheel` ile hazırını alır),
  programın yanına `tools/` (macOS'ta `Contents/Resources/python/`: tekerlek kod değil, programın okuduğu dosya) koyar ve
  `kentos-sdk.json` yazar (dosya, sürüm, SHA-256). Program tekerleği yalnız bu dosyadan, **çevrimdışı ve bağımlılıksız**
  kurar; sürümü programınkiyle aynı değilse ya da özeti tutmazsa reddeder. Hiçbir zaman paket dizininden değil: PyPI'deki
  ilgisiz bir `kentos` yerine geçemez; `kentos` adı paket olarak istenince reddedilir. Ölçüm: hata ayıklama + strip
  tekerleği 137 sn'de 92,8 MB (yayın derlemesi yapılmadı; sahibin "az yayın derlemesi" kararı).
- **Kişisel ortam küçük bir uv projesi** (`<python>/kisisel`: `pyproject.toml`, `uv.lock`, `.venv`), KentOS'un Python'una
  (`requires-python = "==3.12.*"`) ve bu sisteme (`environments`) **sabit**. Ölçüldü: sabitsiz proje uv'ye kilidi bütün
  Python'lar için çözdürdü ve önizleme iki pandas okudu (3.10 için 2.3.3, 3.11+ için 3.1.1). Oluşturmak çevrimdışı: boş
  kilit 31 ms, eşitleme, kitaplık 444 ms.
- **Her değişiklik önce önizlenir:** proje bir kenara kopyalanır (`kisisel-onizleme`, projenin içinde değil: uv onu
  çalışma alanı üyesi sanardı), uv orada ekler/çıkarır/yükseltir, iki kilidin paketleri karşılaştırılır: kurulacak
  (istenen), bağımlılık olarak gelecek, sürümü değişecek, kaldırılacak. Kullanıcı onaylayınca kopya projenin yerine geçer
  ve ortam ona **tam** eşitlenir; tam eşitleme kilitte olmayanı sildiği için (ölçüldü: `kentos` silindi) kitaplık hemen
  geri kurulur. Eşitleme başarısız olursa önceki proje geri gelir ve ortam ona çevrimdışı yeniden eşitlenir.
- **Paket arayüzü** (sahibin isteği): tek arama alanı kurulu paketleri ve **önerilen paketleri** süzer (`paketler.json`:
  7 grupta 29 paket, Türkçe açıklamalı; arama Türkçe harfsiz de bulur: `sayisal` → "Sayısal"). Önerilenler **işaretlenip
  birlikte** önizlenir. Listede olmayan bir ad paket dizininde aranır; ama yalnız ad gibi okunduğunda teklif edilir
  (sürüm yazılmışsa ya da arama hiçbir şey bulmadıysa). Ölçüldü: önce "koordinat" araması pyproj'u bulurken bir de
  "“koordinat” paketini önizle" diyordu. Kurulu paketler satırında Güncelle ve Kaldır; bağımlılıklar altta.
  **Güncellemeleri denetle** `uv pip list --outdated --exclude kentos` ile sorar. İstenen paket `uv add "pandas>=3.1.1"`
  ile yükseltilir (ölçüldü: yalın adı yeniden eklemek kullanıcının yazdığı `==2.3.3` sabitini koruyor), bağımlılık
  `uv lock --upgrade-package` ile. **Bağımlılık ağacı** `uv tree --frozen`.
- **Paket ilerleme çubuğu:** uv büyük indirmeleri boyuyla yazıyor ama bitmeden bir şey demiyor. İlk sürümde çubuk 4. ve
  12. saniyede "0,0 MB / 27,5 MB · %0"da kaldı (ölçüldü). Şimdi önbelleğin üstündeki `.tmp…` klasörlerinde açılan bayt
  3,5'e bölünerek inen bayta çevriliyor (ölçüm: 27,5 MiB indirme → 100,7 MB açılmış, ×3,66). uv bitti demeden çubuk dolmaz
  (%99). Gerçek pencerede: 2,5 sn'de "3,0 / 11,8 MB · %25", 6 sn'de "9,0 / 11,8 MB · %76".
- **Yer:** uv paketleri önbellekten ortama sabit bağla koyuyor (ölçüldü: önbellek sayıldıktan sonra `du` `.venv`'i 392 KB
  gösterdi, gerçek boy 104 MB). Ortamı silmek o yeri açmaz; bu yüzden kaldırma bildirimi yanlış bir "yer açıldı" yazmaz,
  **paket önbelleği** boyuyla ayrı gösterilir ve ayrı temizlenir (`uv cache clean`; ortamlar çalışmaya devam eder).
- **Konsol:** `KENTOS_PYTHON`, sonra Python açıkken ve hazırsa **kişisel ortam** (Windows'ta `.venv\Scripts\python.exe`),
  sonra geliştirme kopyasının `.run/py`'si, en son `python3`. Ölçüm: gerçek pencerede konsol "Hazır" açıldı ve
  `import pandas, numpy` "pandas 3.1.1 · numpy 2.5.4" yazdı (P04 kabulü). KentOS'un Python'u kaldırılınca kişisel ortam
  da silinir (onun Python'u buna bağ), onay cümlesi bunu söyler.
- **Uygularken verilen kararlar:** proje uv projesi (kilit, ağaç ve "betiğe/eklentiye ekle" için) ama kitaplık kilide
  girmez; önizleme klasörü projenin dışında; arama gruba da bakar ("cbs" bütün CBS paketlerini listeler); işaretliyken
  aramada Enter seçilenleri önizler; paket önbelleği ortamla birlikte silinmez.
- **Yapılmadı:** PyPI'nin arama API'si yok (2021'de kapatıldı); bu yüzden adla arama paket dizininde uv'nin çözümüyle
  yapılıyor, sürüm listesinden seçme yok (sürüm yazılabilir: `pandas==2.3.3`). Kurum aynası/dizin ayarı, Windows'ta gerçek
  pencere sınaması (bu makine Linux) ve yayın SDK tekerleğinin boyu sonraki işler.
- **Sınamalar:** python-env 17 (tekerlek denetimi, ad denetimi, sabit proje, ilerleme okuma, önizleme/uygulama/geri alma,
  güncelleme ve ağaç, öneri listesi ve Türkçe arama), masaüstü eklenti sahneleri 14 (oluşturma, önizleme, Durdur,
  çoklu seçim, güncelleme, ağaç, kaldırma, bildirimler).

**Beşinci dilim (14 Kasım): eklenti başına ortam (P05) ve çevrimdışı çalışma (P07).** Sahip geceye girerken "devam et
ve durma, sonraki maddelere kendin geç" dedi.

- **Her eklenti sürümünün kendi ortamı** (`kentos-python-env::extension`): anahtar eklenti kimliği + sürüm + kilidin
  SHA-256'sı + KentOS'un Python'u + sistem + seçilen isteğe bağlı özellikler; anahtarın 16 haneli özeti klasörün adı
  (`<python>/ortamlar/<özet>`; Windows'un yol uzunluğu için kısa). İndirme önbelleği ortak, venv asla (PY-UV-11, 12).
- **Yalnız paketin kilidinden:** `uv sync --locked --no-dev --no-install-project`: yazarın kilitlediği, yeni bir çözüm
  değil; geliştirme bağımlılıkları kurulmaz (PY-UV-17, 19). Projenin iki dosyası ortamın klasörüne kopyalanır, kurulu
  paket salt okunur kalır.
- **Önce önizleme** (PY-UV-14): `uv sync --dry-run` hiçbir şey indirmeden ve yazmadan bu bilgisayarda kurulacakları
  söyler. İsteğe bağlı özellikler (manifestin `optionalFeatures`'ı) işaretlenince listeye girer: MEBİS için 2 paket,
  "Doluluk grafikleri" ile 6.
- **Kur, denetle, en son "hazır" yaz** (PY-UV-18, 28, 31): eşitleme (ilerleme çubuğu ve Durdur, kişisel ortamdakiyle
  aynı ölçü), KentOS kitaplığı kurulumdaki tekerlekten çevrimdışı (kilitte `kentos` adlı paket varsa reddedilir),
  `uv pip check`, ortamın kendi Python'uyla `import kentos`. Kayıt (`ortam.json`) ancak bunlardan sonra yazılır; kaydı
  olmayan klasör yarım ortamdır, kullanılmaz ve yeniden yapılırken silinir. Kullanımdaki ortam yerinde değişmez:
  başka anahtar, başka klasör.
- **Hazır olmak diskteki dosyalardır** (P07): `ready`/`find` uv çağırmaz. Ölçüldü: hazır bir ortamı yeniden eşitlemek
  KentOS kitaplığını siliyor (tam eşitleme kilitte olmayanı kaldırır); bu yüzden çalışırken hiçbir şey eşitlemez ya da
  indirmez, yürütücü (ADIM 19) ortamın kendi Python'unu kullanacak.
- **Masaüstü:** eklentinin ayrıntısında "Python ortamı" bölümü (kurulumda Python katkıları seçildiyse): durum,
  isteğe bağlı özellikler, "Ortamı hazırla" → kurulacaklar → Hazırla → ilerleme/Durdur → "Ortam hazır · Python 3.12.13 ·
  N paket · internetsiz çalışır", paketler ve klasör, "Ortamı kaldır"; her sonuç bildirimle. Şerit düğmesinin nedeni
  artık eklentinin kendisi: "Bu eklentinin Python ortamı hazırlanmadı: Eklentiler › … › Python ortamı" ya da "Python
  ortamı hazır; yürütücü KentOS'un sonraki sürümünde" (ADIM 19'un birinci diliminden sonra komutlar çalışır, §6). KentOS'un Python'u kurulurken ya da kaldırılırken genel durum önce
  gelir. Eklenti kaldırılınca ortamları da gider (PY-UV-30); başka özelliklerle yeniden hazırlanınca eskisi silinir.
- **Yeni örnek** `fixtures/extensions/v1/ornek-imar` ("İmar Uygunluk Denetimi"): shapely `>=2.0,<2.1`, kilidi gerçek
  `uv lock` çıktısı (shapely 2.0.7). MEBİS shapely 2.2.0 kilitliyor.
- **Ölçüm (gerçek pencere, yalıtılmış eklenti ve Python klasörleriyle):** iki paket katalog klasöründen kuruldu; MEBİS
  "Doluluk grafikleri"yle önizlendi (6 paket) ve 1,5 MB/sn'lik vekil üzerinden hazırlandı (çubuk %18 → %54 → hazır);
  İmar önizlendi (numpy 2.5.4, shapely 2.0.7) ve hazırlandı (numpy önbellekten). İki ortam yan yana: ağ kapalıyken
  (ulaşılamayan vekil) MEBİS'in Python'u "shapely 2.2.0", İmar'ınki "shapely 2.0.7" açtı (P05, P07). Şerit ipucu: "Python
  ortamı hazır; …".
- **Uygularken verilen kararlar:** ortam klasörü yeniden adlandırılmaz (venv'ler taşınamaz: betiklerin yolları mutlak),
  "hazır" kaydı en son yazılır; özelliklerin seçimi ayrı bir ayar dosyası yerine son yapılan ortamın anahtarından okunur;
  ortam, eklenti etkin olmasa da hazırlanabilir (ortam sürüme aittir); yeniden eşitleme yerine kayıt.
- **Sınamalar:** python-env 18 (anahtar ve ayrı klasörler, önizleme, hazırlama ve kayıt, yarım ortamın silinmesi,
  `kentos` reddi, uv'siz yeniden kullanım, temizlik), masaüstü eklenti sahneleri 15 (bölüm, önizleme, özellik,
  hazırla/Durdur, şerit nedeni diskteki kayıtla, hata, eklentiyle kaldırma), eklenti crate'leri 28 + 52.

**ADIM 18A'nın kabulü (P01–P07):** P01–P04 önceki dilimlerde, P05 ve P07 bu dilimde, P06 ADIM 17–18'de (inceleme
penceresinden vazgeçen hiçbir şey kurmaz ve indirmez). **Sonraya kalan:** kurum dizini/aynası ve kimlik bilgileri
(PY-UV-25, 26), çevrimdışı tekerlek deposu (PY-UV-27, ADIM 22A), SBOM raporu (PY-UV-29), kişisel paketin betik ya da
eklenti bağımlılığına eklenmesi (PY-OPT-06'nın son adımı), PyPI sürüm listesinden seçme, Windows'ta gerçek pencere.


## §6. Yürütücü ve aracı: eklentinin Python'u kendi sürecinde (`ADIM 19`)

**Birinci dilim (14 Kasım): komutlar.** Sahip geceye girerken "devam et ve durma, sonraki maddelere kendin geç" dedi.
Kod: `python/kentos/ext.py` (eklentinin gördüğü), `python/kentos/ext_host.py` (sürecin kendisi),
`apps/desktop/src/shell/extensions/runner.rs` (yürütücü ve aracı).

- **Süreç:** her eklenti sürümüne bir süreç, eklentinin kendi ortamında:
  `<ortam>/.venv/bin/python -I -X pycache_prefix=<ortam>/pyc -m kentos.ext_host`. İlk çağrıda başlar (PY-RUN-05),
  çalıştırmalar arasında kalır, sürüm değişince bırakılır; Windows'ta konsol penceresi açmaz (`CREATE_NO_WINDOW`).
  Ölçüm (gerçek pencere, İmar, 4 yapı ve 3 parsel): ilk çalıştırma 1,8–1,9 sn (süreç, shapely ve SDK içe aktarımı),
  sonraki 0,1 sn.
- **Protokol 1** (JSON satırları, konsolunki gibi; ADR 0132): KentOS → `hello` (eklenti, sürüm, paketin içerik özeti,
  paket klasörü ve kod klasörleri), `invoke`, `cancel`, `reply`, `quit`; süreç → `ready`, `refused`, `out`, `progress`
  (en çok saniyede 10), `log`, `call`, `done`. `done`: `ok` + sonuç (`kind`, `title`, `message`, `rows`, `columns`) ya da
  `stopped`, `refused`, `unavailable`, `exception` (+ iz kaydı, en çok 20 000 karakter). Bir çalıştırmanın çıktısı en çok
  1 MB; fazlası kesilir ve kesildiği yazılır (PY-RUN-11).
- **Kod yalnız çağrılınca içe aktarılır.** Manifestin gösterdiği işlev `@ext.<tür>("<yerel kimlik>")` ile işaretli
  değilse ya da başka bir kimlikle işaretliyse çalıştırılmaz (PY-RUN-08). Paketin klasörleri yola ortamınkilerden
  **sonra** eklenir: bir paket kurulu bir kitaplığı gölgeleyemez; `kentos` adlı modül taşıyan paket hiç başlatılmaz
  (PY-UV-31).
- **Aracı:** her istek eklentinin izniyle cevaplanır: okuma `document.read`, yazma (`run`/`call`'ın `execute`'u ve
  `replay`) `document.write`; geri alma, yineleme ve dosya işleri kullanıcınındır (`not_for_extensions`); yalnız
  başladığı çizime (PY-RUN-16). **Eklenti çizimi ajan gibi okur:** çizimin gizlediği alanların değerleri kesilir,
  onları anan çağrı reddedilir (ADR 0738, 0743). Konsolun kodu çizimin sahibinindir, bütün okur; eklenti üçüncü bir
  tarafın (çoğu zaman bir yapay zekânın yazdığı) kodudur. Seçim (`ctx.doc.select`) oturum sonucudur, belge değişikliği
  değil.
- **Bir çalıştırma tek geri alma adımıdır**, adı komutun adı ("Izgara çiz (yavaş)"). Hata, Durdur, sürecin ölümü ya da
  başarısız bir grup yazdıklarını geri alır. Ölçüm (gerçek pencere): 36 kare ve katmanı tek Ctrl+Z ile gitti (43 → 7
  nesne); Durdur'da çizim 7 nesneye döndü; yazdıktan sonra `KeyError` veren komutun dikdörtgeni geri alındı.
- **Tek yazan:** yazma izni olan bir çalıştırma sürerken çizim başka düzenleme almaz (konsolun kuralı, `python::held`)
  ve bu bir kez söylenir; konsol, ajan ve yazan başka bir eklenti bekler. Görüntüleme, kaydırma, yakınlaştırma ve Durdur
  çalışır (EKL-04). Yalnız okuyan eklenti (İmar) çalışırken çizim düzenlenebilir. Ölçüm: kuraldan önce, yazma sürerken
  komut satırına yazılan `CIZGI` Çizgi'yi başlatıyordu; şimdi yazı satırda kalır, komut başlamaz.
- **Durdur önce koda sorulur:** okuma iş parçacığı iptali hesap sürerken duyar, `ctx.check()` `Stopped` atar; 5 sn'de
  yanıt yoksa süreç sonlandırılır ve bu ayrı yazılır (PY-RUN-12). Ölçüm: `ctx.check()` çağıran bir Python döngüsü iptalden
  5,4–5,6 ms sonra durdu (5 ölçüm; planın hedefi 1 sn); gerçek pencerede Durdur'dan 1,5 sn sonraki görüntüde "durduruldu —
  Çizime yazdıkları geri alındı" bildirimi ve 7 nesneye dönmüş çizim vardı.
- **Paket kurulduğu gibi kalır.** Python içe aktardığının bayt kodunu kaynağın yanına (`__pycache__`) yazıyordu;
  bütünlük denetimi eklenen dosyayı haklı olarak reddediyor (oradaki bir `.pyc` kaynağın yerine çalışabilir) ve eklenti
  **ilk çalıştırmasından sonraki açılışta duruyordu** ("sonradan eklenmiş: `__pycache__/logic.cpython-312.pyc`";
  gerçek pencerede bulundu). Bayt kodu artık ortamın `pyc` klasöründe (`-X pycache_prefix`; `-I` ortam değişkenlerini
  yok saydığı için `PYTHONPYCACHEPREFIX` işe yaramazdı) ve ortamla birlikte silinir. Bekçiler iki tarafta: Python'da
  paket klasörüne hiçbir şey yazılmadığı, masaüstünde yorumlayıcının argümanları.
- **KentOS kitaplığı ortamda tazelenir:** ortamın kaydı kurulduğu SDK tekerleğinin SHA-256'sını tutar; KentOS'un
  tekerleği değişmişse çalıştırmadan önce çevrimdışı yeniden kurulur (`--no-deps --offline --reinstall-package kentos`).
  Kilitli paketlere dokunulmaz.
- **Sonuç:** rapor satırları Komut geçmişi'ne, eklentinin verdiği sütun sırasıyla yazılır (`columns`: JSON nesnesinin
  anahtarları Rust'ta sıralı geliyordu, "mesafe_m · sınır_m · yapı" diye); 500 satırdan sonrası sayıyla. Başlık ve özet
  bildirimde.
- **Açıkça söylenir** (sahibin "eklenti hata verdiğinde açıkça bilelim" isteği): hata → günlüğe iz kaydıyla, bildirim
  "… hata verdi" ve **Ayrıntı** (eklentinin sayfası; Python ortamı da aranır, "aranıyor"da kalmaz), Komut geçmişi'nde
  hatanın yeri (`logic.py:32, write_then_fail`; iz kaydının ilk satırı değil); kendiliğinden kapanan süreç çökme
  sayacına (D6); `ext.NotAvailable` → "bu sürümde çalışmaz" uyarısı, iz kaydı yok, eklentinin hatası sayılmaz. Günlük
  maskesi `KeyError: 'parsel'`i gizli sanıp `KeyError: '***'` yazıyordu ("key" geçen her ad gizli sayılıyordu);
  `…Error:`, `…Exception:`, `…Warning:` artık istisna adı.
- **`kentos.ext` adları** planın §3.4 örneğiyle (ADIM 19'un kabul hedefi): `command`, `processing`, `audit`,
  `map_tool`, `expr`; `CommandContext` (`doc`, `settings`, `params`, `check`, `is_stopped`, `progress`, `log`, `report`,
  `message`, `cancelled`, `apply_declarative_schema`), `AuditFinding(message, severity, feature_id, fix)`,
  `SemanticFix(title, command, args)`, `Stopped`, `NotAvailable`. **`apply_declarative_schema` bu sürümde
  `NotAvailable`:** `schema.json`'un biçimi ve onu kuran host komutu ADIM 22'nin işi (planın sırası); burada
  kurulsaydı katman şeması kimliği, ad/başlık ve geometri eşlemesi o adımdan önce kararlaşmış olurdu. MEBİS'in "MEBİS
  katmanlarını kur" düğmesi bu yüzden "bu sürümde çalışmaz" der.
- **İmar örneği** seçili yapı varsa yalnız onları denetler, bulduklarını çizimde seçer ve satırları sıra numarasıyla
  yazar (kalıcı kimlik kullanıcıya bir şey söylemiyordu).
- **Uygularken verilen kararlar (planda yazılı değildi):** paket klasörleri yola sona eklenir; süreç
  `os._exit` ile biter (okuma iş parçacığı standart girdiyi tutarken yorumlayıcının kapanışı "could not acquire lock …
  at interpreter shutdown" ile düşüyordu, ölçüldü); SDK tekerleğinden `__pycache__` çıkarıldı (178 `.pyc` vardı);
  eklenti aracıda ajan kuralıyla okur; yazan çalıştırma çizimi bekletir; bayt kodu ortam klasöründe; hata bildirimi
  Ayrıntı'yla; `ext.NotAvailable`; bu dilimde yalnız komutlar (İşlemler aracı, harita aracı, denetim ve panel
  şeritte "sonraki sürümde" der).
- **Sınamalar:** Python 9 (`test_ext_host`: tembel içe aktarım, işaret reddi, iz kaydı, Durdur, protokol, `kentos`
  reddi, sütun sırası, `NotAvailable`, paketin değişmemesi), masaüstü eklenti sahneleri 18 (yürütücü akışı, aracının
  izinleri, tek geri alma adımı, bekletme, konsol/ajan/ikinci yazanın beklemesi, Durdur ve süreç sonlandırma, çökme,
  `NotAvailable`, hata bildirimi ve Ayrıntı, yorumlayıcı argümanları), konsol 30, extension-host 28 (maske), python-env
  18. Windows hedefi (`x86_64-pc-windows-msvc`) python-env ve extension-host için derlendi.
- **Gerçek pencere (yalıtılmış eklenti ve Python klasörleriyle):** İmar ve yalnız sınama için yazılan, depoya girmeyen
  "Deneme Yazıcı" katalog klasöründen kuruldu; Deneme Yazıcı'nın ortamı arayüzden hazırlandı ("0 paket · internetsiz
  çalışır"); bayt kodu hatası bulundu, düzeltildi ve bozulan eklenti Sorunlu › Yeniden dene ile açıldı.
- **Sonraya kalan:** İşlemler araçları, süreç başına bellek ve boştaki sürecin kapanması ikinci dilimde (§7); ifade
  işlevleri, denetimler, harita aracı, form eylemleri §7'nin sonunda.

## §7. İşlemler araçları: tanımdan pencere, ayrı süreçte iş, tipli sonuç (`ADIM 19`, ikinci dilim)

**14 Kasım.** Sahip: "durma demiştim"; dilim ortasında: "Biz sadece numpy ile çalışmayacağız, numpy örnek olsun diye
söyledim." Altyapı kütüphaneden bağımsızdır: her eklenti kendi paketlerini kendi kilidiyle getirir, KentOS yalnız giden ve
dönen veriyi görür. Kod: `python/kentos/ext.py`, `python/kentos/ext_host.py`, `crates/native/processing/src/external.rs`,
`crates/native/python-env/src/process.rs`, `apps/desktop/src/shell/extensions/tools.rs`; örnek: İmar 1.1.0.

- **Araç tanımı (`kentos.processingTool` v1, `tools/<id>.tool.json`):** `format`, `schemaVersion`, `id` (manifestteki
  yerel kimlik), `label`, `description`, `help`, `keywords`, `parameters`, `outputs`, `feedback`,
  `capabilities.runtimes` (verilirse `desktop-python` içermeli), `timeoutSeconds` (varsayılan 600, en çok 3600).
  **Parametreler İşlemler'in kendi JSON'uyla** (`web_param::param_from_json`, web'in `ParamDef`'i; modeller ve ortak
  vakalar da böyle tutar): planın §3.3 taslağındaki `id`/`required`/`accepts` biçimi alınmadı; web araçları, modeller,
  ortak vakalar ve eklentiler tek parametre sözleşmesini paylaşır (PY-TOOL-04). Desteklenen türler: `features`, `number`,
  `string`, `boolean`, `enum`, `field`, `point`, `layer`; başkası yeriyle reddedilir. Adlar Python adı olmalı (işleve
  anahtar sözcük argümanı olarak gider; `ctx` ve Python'un sözcükleri olmaz), tekrarsız; `field.of` ve
  `visibleWhen.param` var olan bir parametreyi göstermeli. Çıktılar: `features` (`sinkParameter` bir katman
  parametresini adlandırır), `table`, `number`, `string`. Tanım kod yüklenmeden okunur (PY-TOOL-02): masaüstü kayıt
  defteri değişince okur; uymayan tanım oturumda bir kez eklentinin günlüğüne yazılır, aracı listelenmez. Kurulumdaki
  paket denetimi yalnız biçimi ve kimliği bakar (paylaşılan crate İşlemler'i tanımaz); tam denetim listelemede.
- **Tek araç, her yerden (PY-TOOL-21):** kimlik `ext.<eklenti>.<yerel>`; `Registry::current()` yerleşiklerle eklentilerin
  araçlarını birlikte verir: pencere (araç kutusunda **Eklentiler** dalı), komut, model, toplu iş, `cad.processing.tools`
  ve `.run` (Python, MCP). Sınama: aynı araç pencere yolundan (çalıştırıcı) ve komut yolundan (`ToolHost`) aynı nesneleri
  aynı özniteliklerle yazar. İmar'ın şeritteki TAKS düğmesi aracın penceresini açar. Eklenti kapatılınca araç çıkar;
  liste değişince `external::generation()` artar (sürümlü kayıt).
- **Ayrı süreç, her iş parçacığından:** `kentos_python_env::process::Pool`: eklenti sürümüne bir süreç, çalıştırmalar
  arasında kalır, bir süreçte bir çalıştırma (ikincisi sırasını bekler, Durdur'u duyarak); Durdur → `cancel`, 5 sn'de
  yanıt yoksa süreç sonlandırılır; zaman aşımı aynı yoldan; kendiliğinden kapanan süreç hata çıktısının son satırlarıyla
  çökme sayılır (bekçiye gider); 10 dk kullanılmayan süreç kapanır (D6, Ö·9); belleği Linux'un `/proc`'undan okunur.
  Araçlar yalnız `Worker` hedefli: pencere hiç beklemez. İşlemler süreçleri komutlarınkinden (runner.rs) ayrıdır: komut
  çizime aracıyla ana iş parçacığında erişir, araç verisini taşır; birleştirme sonraya.
- **Girdiler veri olarak (PY-TOOL-05, 07, 09):** her nesne kalıcı kimliği (UUID), katmanının adı, GeoJSON şekli ve
  nesnesiyle (iç yuva kimliği sıfırlanır) gider; çizimin üçüncü taraflardan gizlediği alanların değerleri `‹gizli›`
  yazılır, gizli alanı seçen alan parametresi reddedilir (ADR 0738, 0743). Katman parametresi host'undur (çıktının
  yeri), işleve gitmez. Aracın çizimi yoktur: sürecin `call`'u `capability` ile reddedilir; çizim hesap boyunca
  tutulmaz.
- **Kütüphaneden bağımsız:** şekiller iki yönde GeoJSON (RFC 7946) ve Python coğrafi kütüphanelerinin ortak
  `__geo_interface__`'i: `ext.Feature` bir GeoJSON Feature'ı, `ext.FeatureInput` bir FeatureCollection'dır
  (`shapely.geometry.shape(f)`, `GeoDataFrame.from_features(...)`); çıktıda `ext.NewFeature` (GeoJSON ya da
  `__geo_interface__`'li bir şekil, ya da bir girdinin şekli: `source`, öznitelikleriyle), GeoJSON Feature sözlükleri ya
  da bütün bir FeatureCollection (GeoDataFrame). Tablo: satır sözlükleri, bir DataFrame (sütun sırasıyla) ya da
  `{columns, rows}`. Sayılar: `tolist()`/`item()` veren her dizi ve sayı (NumPy, pandas …), Decimal, ISO tarih; NaN ve
  sonsuz → boş. Yaylar GeoJSON'a 1 mm'lik kirişlerle iner (`ops::derive::path_chords`): kirişi 30 m, yay değeri 0,3
  olan parsel 69 kirişe bölünür, alanı yayınkinden 0,02 m² eksik (991,58; kesin 991,60 m²); çizimin kendi 5°'lik bölmesi
  0,50 m² eksik veriyordu (gerçek pencerede 991,10 m² görüldü, düzeltildi).
- **Tipli sonuç, bütün denetlenir (PY-TOOL-10):** tanımda olmayan çıktı, türü tutmayan değer, girdiler arasında olmayan
  `source`/`select` kimliği, 3 köşeden az halka, sonlu olmayan koordinat, liste ya da sözlük öznitelik, 500 000 nesneden
  ya da 100 000 satırdan fazlası, 10 000 karakterden uzun metin: "“…” tipli sonucu geçersiz: …" ile hiçbir şey yazılmaz,
  geri alma adımı açılmaz. Katman alanları değerleri yerleşik araçlarınki gibi denetler (`writes::check`). Yeni nesnenin
  görünüşü katmanınındır (kopyada renk, sembol, çizgi kalınlığı, etiket iğneleri, ekler ve görünüş sıfırlanır); çok
  parçalı şekiller ve delikler korunur. Tablo `{columns, rows}` olarak gelir: Rust'ın JSON nesnesi anahtarları sıralıyor,
  sütun sırası kayboluyordu.
- **Tek geri alma adımı (PY-TOOL-11, 17):** yeni katman ve nesneleri çalıştırıcının tek adımında, adı aracın adı.
  Gerçek pencerede tek Geri al: "Geri alındı: Parsel TAKS hesabı", TAKS katmanı ve dört nesnesi gitti.
- **Açıkça söylenir:** köprünün iş parçacığı notlar bırakır, pencerenin iş parçacığı yazar: `run_started`,
  `run_finished` (süre ve süreç belleği), `run_stopped` ("Durdur'dan 9 ms sonra durdu"), `run_killed`, `run_timed_out`,
  `run_failed` (iz kaydı ve yazdırılanlarla), `runner_crashed` (bekçiye), `runner_not_started`, `handler_refused`,
  `run_unavailable`; hata bildirimi "… hata verdi" ve **Ayrıntı**. Pencere sonucu ya da hatayı kendi satırında da söyler.
- **Bayt kodu önceden derlenir:** gerçek pencerede ilk TAKS süreci 364 MB tutuyordu. Neden ölçüldü: süreç KentOS
  kitaplığının bayt kodunu kendisi derliyordu (ortamın `pyc` klasörü, 17 MB) ve Python derleyicinin belleğini işletim
  sistemine geri vermiyor. Ortam hazırlanırken ve kitaplık tazelenirken bayt kodu ayrı, kısa ömürlü bir süreçte
  sürecin okuyacağı yere derlenir (`extension::precompile`, 2,2 sn, 15 MB; kitaplığın açıldığı denetimi de odur): ilk
  süreç hazır olduğunda 113 MB (1,8 yerine 0,85 sn), TAKS'tan sonra 129 MB; gerçek pencerede "Bitti (1,6 sn; süreç
  belleği 129 MB)" (önce "Bitti (2,9 sn; süreç belleği 364 MB)").
- **Süreç başına bellek (hata ayıklama kitaplığıyla, Linux):** yalın Python 10 MB; NumPy ve shapely 26 MB; `import kentos`
  111 MB (hata ayıklama derlemesinin `_native.abi3.so`'su 313 MB); TAKS'tan sonra 129 MB; paketsiz Deneme Yazıcı
  111 MB. Yayın derlemesinin kitaplığıyla ölçülmedi (daha küçük beklenir); işlem aracının yerel modüle ihtiyacı yoktur,
  tembel yükleme ayrı bir karar.
- **Durdur (gerçek pencere):** Durdur'u dinleyen araç Durdur'dan 29 ms ve 9 ms sonra durdu (planın hedefi: iptal 100 ms'de
  görünür, hesap 1 sn'de durur); dinlemeyen araç ("İnatçı") 5 sn sonra sonlandırıldı, çizim değişmedi; pencere Durdur'da
  hemen "İşlem iptal edildi; çizim değişmedi." der.
- **İmar 1.1.0: Parsel TAKS hesabı** (Planlı Alanlar İmar Yönetmeliği md. 4: taban alanının imar parseli alanına oranı):
  shapely 2.0'ın `STRtree` ve vektörel kesişimi, NumPy'nin `bincount`'u; parseller seçilen katmana `parsel_alani_m2`,
  `taban_alani_m2`, `taks`, `taks_siniri`, `yapi_sayisi`, `durum` ile kopyalanır, tablo yazılır, sınırı aşanlar seçilir.
  Varsayılan girdiler KentOS CBS şablonunun standart katmanları (`parsel`: "Parsel sınırı", `yapi`: "Yapı"); yaklaşma
  komutu da katmanları kimlikle bulur. NumPy açık bağımlılık (kilit en az değişiklikle: sürümler aynı). Gerçek pencerede
  (sentetik çizim, sınır 0,40): 101/1 450/900 = 0,5 aşıyor (iki parsele taşan yapının 50 m²'si dahil); 101/2 0,271;
  101/3 yaylı, 225/991,58 = 0,227; 102/1 200/500 = 0,4, sınırda, aşmaz.
- **Uygularken verilen kararlar (planda yazılı değildi):** parametreler İşlemler'in JSON'uyla; tek şekil biçimi GeoJSON;
  yay kirişleri 1 mm; çıktılar adlarıyla bir sözlükte; tablo `{columns, rows}`; işlem süreçleri komut süreçlerinden ayrı;
  araçlar yalnız arka planda; katman parametresi host'un; zaman aşımı 600 sn; boşta 10 dk; uymayan tanım bir kez
  yazılır, aracı listelenmez; şablon katman kimlikleri varsayılan; NumPy açık bağımlılık; bayt kodu önceden derlenir;
  araç kutusunda "Eklentiler" dalı; aracın yardımı "ayrı süreç bir korumalı alan değildir" der (PY-RUN-19); süre,
  bellek ve Durdur gecikmesi günlükte; "Deneme Yazıcı" 0.2.0'a dinleyen ve dinlemeyen iki deneme aracı (depoda değil).
- **Sınamalar:** Python 14 (`test_ext_host`: girdiler veri olarak, tipli çıktılar, kütüphane biçimli değerler, Durdur,
  yanlış dönüş), İşlemler 7 (`tests/external.rs`: tanım, pencere ve komut eşdeğerliği, kalıcı kimlik ve gizli alan,
  geçersiz sonucun hiçbir şey yazmaması, GeoJSON şekilleri, yayların kirişleri ve alanı), python-env 24 (süreç havuzu 6:
  süreç korunur, Durdur önce sorulur sonra sonlandırır, zaman aşımı, çökme, çizim isteği reddi, boşta kapanma, bellek),
  masaüstü eklenti sahneleri 21 (listeleme, şerit düğmesi ve pencere, kapatınca çıkma, uymayan tanım, sonuç okuma).
- **Modelde zincirleme (P12):** eklenti aracının yeni nesneleri aynı modelde yerleşik Tampon'un girdisi olur; model bütün
  olarak tek geri alma adımıdır (`tests/external.rs`).
- **Komut süreçleri de boşta kapanır:** bir komutun süreci 10 dk kullanılmazsa kapatılır ve günlüğe `runner_idle`
  yazılır; sonraki çağrı yeni süreç başlatır (D6, Ö·9).
- **Dosya hedefi (P11, gerçek pencere):** "Sonucun yeri: GeoPackage dosyasına" seçilince araç çizimin bir kopyasında
  çalışır, çizim değişmez: `TAKS` tablosu (CURVEPOLYGON, SRS 5256; yaylı parselin yayı eğri olarak kalır) dört satırı
  pencerenin değerleriyle yazıldı ("Sonuç taks-sonuc.gpkg dosyasına yazıldı: TAKS (4 satır)"). Bulgu: İmar'ın özeti
  "çizimde seçildi" diyordu, oysa dosya hedefinde seçim kopyada kalır; araç sonucunun nereye gittiğini bilmez, özeti
  yalnız hesabı söyler (`ctx.summary`'nin belgesi de öyle der).
- **MCP:** eklenti araçları MCP'ye `cad.processing.tools` ve `cad.processing.run` üzerinden, pencereyle aynı kayıttan
  (`Registry::current()`) ulaşır; liste her çağrıda o anki hâlidir. MCP'nin kendi araç listesi eklentilerle değişmediği
  için `tools/list_changed` gerekmez; eklenti komutları kendi MCP aracı olduğunda (sonraki adım) `external::generation()`
  o bildirimin ölçüsüdür.
- **Sonraya kalan (ADIM 19'un kalanı):** denetimler, harita aracı, form eylemleri; büyük girdinin sayfalı/ikili aktarımı
  (PY-TOOL-08: girdi şimdi tek mesajda); tanımın kurulumda tam denetimi (ADIM 21'in `extension.verify`'ı).

## §8. İfade işlevleri: `ext.<eklenti>.<işlev>(…)` (`ADIM 19`, üçüncü dilim)

**14 Kasım.** Kod: `crates/shared/expression/src/external.rs` (dil), `crates/native/processing/src/external.rs` (İşlemler'in
kaydı), `apps/desktop/src/shell/extensions/tools.rs` (`Answers`), `python/kentos/ext_host.py` (`expression`).

- **Sözdizimi:** planın yazdığı gibi `ext.tr.ornek.imar-denetim.ada-parsel(Ada, Parsel)`. Eklenti kimliklerinde ve yerel
  kimliklerde nokta ve tire var; sözcük çözümleyici yalnız `ext` sözcüğünden hemen sonra gelen `.` ile başlayan parçaları
  (`[\p{L}\p{N}_-]+`) adın içine alır: başka her yerde tire eksidir, `ext` tek başına bir alan adıdır (sınandı). Ad
  bütün yazılır; kısaltma yok.
- **Derlemede adıyla bilinir, kod yüklenmeden (PY-TOOL-02):** şemanın `externals` listesi (`ExternalDef`: ad, bir satır
  açıklama) masaüstünde kayıt defterindeki `expression.provider` izinli eklentilerin `entrypoints.expressions`
  girdilerinden kurulur. Listede olmayan bir `ext.…` çağrısı ilk geçtiği yerde reddedilir ("Bilinmeyen işlev: … burada
  kurulu ve açık bir eklentinin ifade işlevi değil"). Bağımsız değişken sayısı manifestte yok: eklentinin kodu söyler,
  yanlış sayı o satırın hatasıdır.
- **Yığın başına bir çağrı:** sütun motoru bir yığının (256 nesne) argümanlarını sütunlardan okur ve host'a bir kez sorar
  (`ExternalCalls::call(ad, satırlar)`); tek nesne yolu (`walk`) bir satır sorar. Argümanı hata veren nesne hatalı
  (THROWN) kalır; host yoksa (önizleme, eklentisiz host) yanıt boştur. `Expr::externals()` bir ifadenin çağırdığı
  işlevleri verir.
- **Masaüstünün yanıtları (`Answers`):** bir yığının yalnız farklı argüman satırları eklenti sürümünün sürecine tek
  mesajla gider (`kind: "expression"`, `rows`), yanıtlar argümanlarıyla saklanır (100 000'i geçince baştan); aynı ifade
  yeniden çalışınca ya da aynı değerler birçok nesnede olunca soru gitmez. Süreç İşlemler araçlarınınkiyle aynı havuzdadır
  (§7). Bir satırın hatası yalnız o satırı boş bırakır; kaç satırın hata verdiği ve ilki eklentinin günlüğüne bir kez
  yazılır (`expression_failed`); sürecin cevap vermemesi de.
- **Nerede:** bu dilimde İşlemler'in ifadeleri (Öznitelik hesapla, İfadeyle seç …; araç arka planda çalışır, beklemek
  pencereyi dondurmaz) ve pencerenin ifade denetimi (adı tanır). Oluşturucunun ve pencerenin önizlemesi sürece sormaz,
  eklenti işlevinin değeri önizlemede boştur: pencere beklemez. Etiketler, tematik stil kuralları, sanal alanlar,
  süzgeçler ve form hesapları çizim sırasında değerlendirilir; orada beklemek çizimi dondururdu: önbellekten okuyup
  eksikleri arka planda dolduran yol sonraki dilim. Form şeması bu yüzden şimdilik eklenti işlevi sunmaz.
- **İmar 1.1.0:** `ada-parsel(ada, parsel)` → "101/1" (ikisinden biri boşsa boş); `expression.provider` izni.
- **Sınamalar:** ifade motoru 4 (adın okunması, bilinmeyenin yeriyle reddi, yığına bir çağrı, tek nesne) ve bütün dil
  sınamaları (86 + bütünleşik); İşlemler 1 (Öznitelik hesapla üç parsele "101/1, 101/2, 101/3" yazdı, eklentiye tek
  yığınla soruldu; bilinmeyen işlev çalıştırmadan önce reddedildi); Python 1 (satır başına hata); masaüstü 1 (İmar'ın
  işlevi adıyla bilinir, pencerenin denetimi geçer).
- **Gerçek pencere (14 Kasım):** İmar 1.1.2 (deneme sürümü) güncellemesinde "İfadelere işlev ekler · YENİ" yeteneği
  gösterildi; Öznitelik hesapla'da `ext.tr.ornek.imar-denetim.ada-parsel(ada, parsel)` pencerenin denetiminden geçti,
  "4 nesnede “etiket” yazıldı". Bulgu: önizleme sürece sormadığı için "4 nesnede sonuç boş" diyordu; artık "Önizleme
  yapılmadı: ifade bir eklenti işlevi çağırıyor (…); değeri çalıştırınca eklentinin kendi sürecinde hesaplanır" der.
  Bulgu: "Otomatik" 2000 nesnenin altında aracı pencerenin iş parçacığında çalıştırıyordu, eklentinin süreci beklenirdi;
  `ext.` çağıran ifadesi olan çalıştırma artık her zaman arka planda; pencerenin "Nerede çalışır" satırı da "Otomatik —
  şimdi: arka planda" der (kullanıcı "Bu bilgisayarda"yı seçse de arka planda çalışır: eklentinin süreci beklenmez).
- **Uygularken verilen kararlar (planda yazılı değildi):** `ext` sonrası nokta ve tire adın içinde; bağımsız değişken
  sayısı denetlenmez; yanıt önbelleği argümanlarla, 100 000 yanıt; önizleme sürece sormaz ve bunu söyler; eklenti işlevi
  çağıran çalıştırma arka planda; etiket/stil/form sonraki dilimde; İmar'ın işlevi biçimlendirme (mevzuat iddiası yok).

### §8.1. Dördüncü dilim: çizimdeki ifadelerde eklenti işlevi (14 Kasım)

**Kural: çizim hiç beklemez.** Etiket metni ve koşulu, stil kuralı (koşul, kategori ve sınıf değeri, veriye bağlı
sembol), katman süzgeci ve form hesabı çizerken değerlendirilir. Host'un yanıtlayıcısı elindeki yanıtı hemen verir;
olmayan satır boş döner ve arka plandaki işçiye sıraya girer (`LabelCalls`, `LabelAnswers`); yanıt gelince sayaç
ilerler ve yalnız işlev çağıranlar yeniden kurulur. Hata veren ya da yanıtlanmayan satır boş kalır, yeniden sorulmaz.

- **Tek kayıt** (`a7da05404`): işlev listesi ve yanıtlayıcı ifade motorunda (`kentos_expression::external::set_host`,
  `host_defs`, `host_calls`, `host_answered`, `host_answers`). Etiket, stil, süzgeç ve form aynı kaydı okur; masaüstü
  bir kez kaydeder. Host yoksa (web, başsız Python, MCP'nin kendi açtığı çizim) işlev yoktur ve böyle bir çağrı
  derlenmez, etiketteki gibi.
- **Etiket** (`4f56a681e`, `eb63df1a2`; ADR 8058 §1): metin sürümü etiket çiziminin anahtarında.
- **Stil** (`17c4e8f15`): `style-core::style::externals`; derleme yerleri (çizimin `Exprs`'i, oluşturucunun denetimi,
  Katman stili'nin sınıf ve sayaçları) işlevleri tanır, değerlendirme `Calling` kapsamıyla. Stil önbelleği yanıt
  sayacını tutar; ilerleyince yalnız oluşturucusu `ext.` geçen katmanlar yeniden kurulur. Pencerenin sayaçları da
  (yanıttan önce 0 kalıyordu).
- **Süzgeç** (`9a1e02e9d`): sütun motoru host'un yanıtlarıyla (`evaluate_rows_on_with`); yanıt gelmeden nesne dışarıda.
  Yanıt gelince `Spatial` işlev çağıran süzgeçleri yeniden çalıştırır (`refilter_external`) ve `filters_version`'ı
  ilerletir; stil önbelleği o katmanları yeniden kurar (yeni işaretlerle). Pencerenin "N / M nesne geçiyor"u da tazelenir.
- **Form hesabı** (`a7da05404`): yalnız alanın `calculate`'i işlevleri tanır (`calc_schema`); koşul, denetim ve
  varsayılan tanımaz (yanıt gelmeden yargılanırlardı); ön denetim de aynı ayrımla. Yazarken bekleyen yanıtlayıcı, açık
  form yanıt gelince yeniden hesaplanır. **Kaydederken bekler:** masaüstü Kaydet'i, Hesapları yenile'yi ve Formla
  toplu yaz'ı `kentos_forms::answering_with` altında İşlemler'in bekleyen yanıtlayıcısıyla çalıştırır; değer çizime boş
  girmez.
- **"Sanal alan"** bu kod tabanında ayrı bir özellik değil (katman alanında ifade yok); planın satırı karşılıksız.
- **Gerçek pencere:** Kurallar koşulu `ext.tr.ornek.imar-denetim.ada-parsel(ada, parsel) = '101/2'` ile yalnız 101/2
  çizildi, kural sayacı yanıt gelince 0 → 1; aynı süzgeçle Katmanlar "1 / 4" saydı, pencere "1 / 4 nesne geçiyor"
  dedi. Form hesabı birim sınamasıyla (yazarken boş, kaydederken "101/2"); tasarımcıda gerçek pencerede yürünmedi.
- **Uygularken verilen kararlar:** tek kayıt ifade motorunda; formda yalnız hesap işlev tanır; kaydetme, yenileme ve
  toplu yazma bekler; yazdırma (pafta, PDF) çizimin önbelleğini okur, beklemez (ilk baskıda yanıt yoksa boş).

## §9. Harita aracı: Rust şekli çizer, eklentinin kodu yerleştirir (`ADIM 19`, beşinci dilim; `cdce4c02e`)

**14 Kasım.** Kod: `crates/native/interaction/src/viewer_tools/capture.rs` (`Capture`, `Kind`, `Shape`,
`template_rectangle`), `apps/desktop/src/shell/extensions/runner.rs` (`ext_capture_start`/`next`/`tasks`, `Capturing`,
`shape_json`), `python/kentos/ext.py` (`Shape`, `MapToolContext.add_feature`), `python/kentos/ext_host.py` (`mapTool`).
Planda harita aracı ADIM 22'deydi (`kentos.ui` panelleriyle); Vitrin'in her katkıyı göstermesi için öne alındı. ADIM
22'de panel ve katman şeması kurulumu kalır.

- **Şekli KentOS çizer** (plan §1.4: "kenetleme ve önizlemeyi Rust yapar; Python onaylanan noktaları işler"):
  manifestin `capture`'ı nokta, çizgi ve alan (Enter bitirir), iki köşeli dikdörtgen, yönlü dikdörtgen ve daire ister.
  Yönlü dikdörtgen şablonla (`templateWidthM`/`templateLengthM`) ortası ve uzun kenarın yönüyle çizilir (Enter kuzeye
  bakar; MEBİS'in 1,10 × 2,20 m mezarı, Vitrin'in 10 × 12 m yapısı), şablonsuz bir kenar ve genişlikle. Kenetleme ve
  yazılan koordinat her komuttaki gibi; Esc şekil vermez (`ViewChange::Captured`).
- **Eklentinin kodu yalnız sonucu alır:** masaüstü aracı şeritten başlatır (eklentinin Python'u çalışamıyorsa önce nedeni
  söylenir), şekli GeoJSON olarak verir (`params.shape`; daire 64 kirişli halka, merkezi ve yarıçapı yanında) ve kodu
  eklentinin sürecinde komut bağlamıyla çalıştırır. Yerleştirme yazıldıysa araç yeniden başlar; hata, Durdur, Esc ya da
  bu arada başlatılan başka bir araç onu bitirir. Her yerleştirme bir komut çalıştırması gibi tek geri alma adımıdır.
- **Python:** `ext.Shape` (GeoJSON, `__geo_interface__`, `points`, `circle`) ve `MapToolContext.add_feature(katman, şekil,
  öznitelikler)`: alanı, çizgiyi ya da noktayı katmana (kimlikle, ya da adı tek katmandaysa adıyla) KentOS'un
  komutlarıyla yazar ve ne yazdığını söyler.
- **Gerçek pencere (Vitrin, `02-harita-araci`):** Yapı yerleştir iki yapı yazdı (Yapı katmanı 5 → 7), biri
  doğu-kuzeydoğuya, biri kuzeye dönük; Esc'ten sonra araç bitti.
- **Uygularken verilen kararlar (planda yazılı değildi):** şekil GeoJSON; daire 64 kiriş; yönlü dikdörtgen ortası ve
  yönüyle, Enter kuzey; araç Esc'e kadar sürer; yerleştirme başına bir geri alma adımı.
- **Sınamalar:** yakalamanın türleri ve şablonun dönüşü (2), Python host'unun çizilen alanı öznitelikleriyle aracıdan
  yazması, masaüstü eklenti sahneleri.

## §10. İşlemler aracının tanımı kurulumdan önce denetlenir (`5af293f9a`)

Paketin kendi denetimi bir araç tanımının biçimini, sürümünü ve kimliğini okuyordu; masaüstünün kuramayacağı bir tanım
(`kentos_processing::external::from_descriptor`'ın reddettiği) "kuruldu" diye kuruluyor, araç ilk listelendiğinde bir
günlük satırıyla anlaşılıyordu. Artık kayıt defteri host'un denetimini alır (`Options::tools`, `ToolCheck`) ve gözden
geçirmede (sorun paketin altında Kur'dan önce görünür, Kur önerilmez) ve kurarken (başka yoldan gelen kurulum da aynı
reddedilir) sorar; her ret tanımın dosyasında bir `tool_descriptor` sorunudur ("“taks” aracı: …"). MEBİS'in örnek tanımı
planın ADIM 19 öncesi taslağıydı, v1'e yazıldı (içerik özeti `sha256:ec9730da…46501`, Python'da ayrıca hesaplandı).
**Yapılmadı:** `kentos kek inspect` paketin denetimiyle kalır (komut satırı İşlemler'i içermez).

## §11. Eklenti Vitrini: her gösterimin eklentisi (`samples/eklenti-vitrini`; `bfba24a2e`, `a863d6892`, `da300f5b7`)

Sahibin kararı (14 Kasım): "eklenti sisteminin bütün gücünü gösterecek demo bir eklentiyi … repoya dahil et ve bütün
demoları o eklentiyle yap … eklenti sistemini geliştirirken bu eklentiyi de geliştir"; "çok fazla büyütme ama rafine,
temiz ve kaliteli"; "içinde birçok durum için örnekler olsun, sadece eklenti de değil".

- **İçerik:** `eklenti/` (`tr.ornek.vitrin` 1.0.1, MIT, yapay veri): okuyan (Katman özeti; katmanın sağ tık menüsünde
  de), ilerleme ve Durdur'la yazan (Yapıları numarala) ve bilerek hata veren (Hata örneği) komutlar; shapely 2.0.7 ve
  NumPy 2.5.4 kullanan TAKS İşlemler aracı (gerçek `uv lock`); `ext.tr.ornek.vitrin.ada-parsel` ifade işlevi (etiket,
  kural, süzgeç, İşlemler); Yapı yerleştir harita aracı; Python'suz XYZ altlık. `vitrin_cizim.py` → `vitrin.kcad` (dört
  parsel, biri yaylı; beş yapı; TM36), `hazirla.sh` (katalog), `senaryolar/00…07` (gerçek pencere, 1440 × 900): kurulum,
  Python ortamı, harita aracı, komutlar, TAKS, etiket, kural, süzgeç. README her durumu, yerini ve nasıl deneneceğini;
  ajanlar için kimlikleri, `drawing.view`'u ve eklentinin günlüğünü yazar.
- **Kural:** eklenti sistemine gelen her yetenek aynı değişiklikte Vitrin'e de girer; gösterimler ve ekran görüntüleri
  onunla yapılır, deneme paketleriyle değil. Henüz çalıştırılamayan katkılar (denetim, panel) README'de böyle yazılı.
- **Bekçiler:** paket uyarısız paketlenir ve bütün katkılarıyla paketin denetiminden geçer (`extensions`); masaüstünün
  gözden geçirmesi kendi araç denetiminden geçer, sekmede beş düğme görünür (masaüstü).
- **Yaparken bulunanlar (düzeltildi):** eklentinin ortamındaki SDK tekerleği en son paketlenendir (eskisi "module
  'kentos.ext' has no attribute 'Shape'" dedi, eklentinin hatası olarak açıkça gösterildi; çare
  `bundle_sdk.py`); büyütülmüş Eklentiler penceresi bölmelerin 470 px'ini koruyup alt yarıyı boş bırakıyordu
  (`ListDetails::grow`: elle boyutlandırılmış ya da tam ekran pencerede yükseklik pencerenin); pencere açılınca seçilen
  eklentinin ortamı yalnız tıklanınca aranıyor, bölüm "Ortam aranıyor…"da kalıyordu (artık güncellemeden sonra aranır);
  İşlemler'in araç ve işlev listeleri, yürütücüsü ve ifade host'u her pencere kurulurken sürece yazılıyordu: sınamalarda
  bir pencere ötekinin listesini ezip "Bilinmeyen işlev" düşürüyordu. Artık araç ya da işlev sunan pencere yazar,
  sınamada o sınama bitene dek tutar (`hold_process_lists`); dört ardışık koşu 25/25.

## §12. Form eylemi eklenti aracını açar; eklenti sabitlenir (`ADIM 19`, FORM-API-13, ECO-DEC-09)

**14 Kasım.** Kod: `crates/shared/contracts/src/gis/forms.rs` (`ExtensionPin`, önce `ActionPin`; `action_tool_ok`,
`action_tool_extension`, `extension_pin_ok`), `apps/desktop/src/shell/extensions/tool_pins.rs` (`PinnedTool`), formun düğmesi
(`gis/forms/entry/items.rs`, `steps.rs`) ve tasarımcı (`designer/step.rs`, `designer.rs`, `designer_view/inspector.rs`).

- **Kimlik:** form eylemi (ADR 0441 §6) bir eklenti aracını `ext.<eklenti>.<araç>` diye anar. Önceki kural yalnız
  harf ve rakam kabul ediyordu; tireli eklenti kimlikleri (`tr.ornek.imar-denetim`) reddediliyordu. Şimdi `ext.`
  sonrası manifestin kuralıyla (küçük harf, rakam, tire; en az iki parça kimlik ve bir araç; en çok 196 karakter);
  yerleşik araçlar eskisi gibi. Şemanın deseni (`ACTION_TOOL_TEXT`) ikisini ayrı dallarda yazar.
- **Sabitleme (`pin`):** tasarımcıda eklenti aracı seçilince eylem `{extension, version, digest}` taşır: eklentinin
  kimliği, kurulu sürümü ve paketinin içerik özeti. Özet aracın tanımını, kodunu, ortamın kilidini ve istenen
  yetenekleri birlikte kapsar; FORM-API-13'ün "tanım/yayın/yürütücü/izin özeti" tek özettir. Yalnız eklenti aracında
  ve o aracın eklentisiyle yazılır; aksi `invalid_form` (çekirdek, Python başvuru denetleyicisi `kcad.py` ve şema).
  Sabitlemesiz eylem geçerlidir (ajanın yazdığı form): tasarımcı uyarır.
- **Masaüstünde üç durum:** *hazır* (eklenti etkin, aracı listede, Python'u çalışır: düğme aracı kayıtla açar);
  *bekliyor* (eklenti burada, kodu henüz çalışamaz: Python kapalı ya da ortamı hazırlanmadı; düğme kapalı, altında
  "Bekliyor: …" ve ne yapılacağı; hiçbir şey başlatılıp düşürülmez); *kullanılamaz* (eklenti yok, kapalı, engelli ya da
  kurulu sürümünde bu araç yok; "Kullanılamaz: …", sabitleme varsa formun beklediği sürümle). Nedenler şeridin
  Python nedenleriyle aynı yerden (`python_reasons`, şerit tazelenirken bir kez).
- **Tasarımcı:** aracı seçmek (yeniden seçmek de) sabitler; denetçi "Eklenti Vitrini 1.0.1 ile sabitlendi." der.
  Ön denetim: kullanılamayan araç `form_action_unknown` (nedeniyle), kurulu sürümden ya da yapıdan farklı ya da hiç
  sabitlenmemiş eylem `form_action_pin` (uyarı). **Sürüm farkı düğmeyi kapatmaz:** düğme aracın penceresini açar,
  kullanıcı parametreleri görüp Çalıştır'a kendisi basar; her güncellemede formların düğmesi kapanmasın.
- **Vitrin:** `vitrin.kcad`'a Parsel formu (ada, parsel, "TAKS hesapla"); düğmenin özeti `vitrin_cizim.py`'de KentOS'un
  paketleyicisiyle çıkar (`kentos kek pack --json`). Bekçi: paket değişip çizim yeniden yazılmazsa masaüstü sınaması
  düşer.
- **Kayıt aracın girdisidir (bulgu, düzeltildi):** düğme kaydı seçiyordu ama aracın nesne girdisi tanımın
  varsayılanında ("Katman") kalıyordu; TAKS formun parselini değil bütün parselleri hesaplıyordu. Artık kaydın
  katmanını okuyan girdi (yoksa seçimi okuyabilen ilk girdi) "Seçili"ye geçer, türleri korunur, öteki girdiler olduğu
  gibi kalır (ADR 0441 §6'nın "kayıt seçili olarak" sözü). Yerleşik araçlarda da (Öznitelik hesapla).
- **Gerçek pencere (`08-form-eylemi`):** 101/2 Formla düzenle'de açıldı; "TAKS hesapla" aracı "İmar parselleri: Seçili —
  1 kapalı alan" ile açtı; Çalıştır "1 parselin TAKS'ı hesaplandı; 0 parsel plan notundaki 0,4 sınırını aşıyor"
  dedi (244 / 900 = 0,271). Formlar'da araç "Eklenti Vitrini 1.0.1 ile sabitlendi.", ön denetim sorun yok. Boş bir
  Python klasörüyle düğme kapalı: "Bekliyor: Python kapalı: Uygulama ayarları › Python ve eklentiler'den
  açabilirsiniz." Bulgu: Otomatik seçim kapalıyken yalnız etkin katmanın nesnesi seçilir; senaryo önce Parsel
  sınırı'nı etkin yapar.
- **Sınamalar:** sözleşme 1 (kimlik, sabitleme, gövde, JSON), komut fikstürleri (iki ret, bir okuma; Rust ve Python),
  şema (1 geçerli, 3 bozuk), `kcad.py` aynı kararlar, masaüstü 2 (üç durum, düğme, tasarımcı, kaydın girdi oluşu;
  Vitrin çiziminin sabitlemesi paketin bugünkü özetine uyar), form sahnesi (yerleşik araç da kaydı okur).
- **Uygularken verilen kararlar (planda yazılı değildi):** tek içerik özeti dört özetin yerine; sürüm farkı uyarı,
  ret değil; sabitlemesiz eylem geçerli; bekleyen durumda düğme kapalı (aracın penceresi açılıp çalıştırınca düşmek
  yerine); kaydın girdisi kaydın katmanından bulunur; eklenti formları (manifestin `forms`'u) bu dilimde değil,
  ADIM 22.

## §13. Model adımı eklenti aracını sabitler (`ADIM 19`, PY-TOOL-20)

**14 Kasım.** Kod: `crates/native/processing/src/external.rs` (`ExternalTool::version`/`digest`, `pin_of`),
`model.rs` (`ModelStep::pin`, `check_model`), `model_edit.rs` (`add_step`), `apps/web/src/processing/model.ts`,
`apps/desktop/src/shell/extensions/tool_pins.rs` (`step_pin_note`), model tasarımcısının adım ayarları.

- **Tek tür:** formun `ActionPin`'i `ExtensionPin` oldu (sözleşme, TS, Python, şema); form eylemi ve model adımı aynı
  `{extension, version, digest}`'i taşır, JSON adı ikisinde de `pin`.
- **Adım eklenirken sabitlenir:** `add_step` eklenti aracının kurulu sürümünü ve paket özetini yazar (`pin_of`; özet
  masaüstünün kayıt defterinden `ExternalTool::digest`'e yazılır). Bütün yollar aynı düzenlemeden geçtiği için
  tasarımcı, Python ve MCP'nin model düzenlemesi aynı sabitlemeyi yazar. Seçimden kurulan tarif modeli de.
- **Eklentisiz açılış (PY-TOOL-20'nin "kod kurdurmaz"ı):** model açılır, hiçbir şey kurulmaz; adımın sorunu
  "“ext.tr.ornek.vitrin.taks” aracının eklentisi bu bilgisayarda yok: model tr.ornek.vitrin 1.0.1 ile yapıldı;
  Eklentiler'den kurun." der (Rust ve web aynı sözle). Sabitlemesiz eski adım "Bilinmeyen işlem aracı" der.
- **Adımın satırı (masaüstü):** "Eklenti Vitrini 1.0.1 ile eklendi."; başka sürüm ya da yapı kuruluysa ikisini
  söyler; eklentinin kodu çalışamıyorsa "Bekliyor: …", eklenti kapalıysa "Kullanılamaz: …". Sürüm farkı modeli
  durdurmaz (adım aracın kurulu hâliyle çalışır).
- **Gerçek pencere (`09-model`):** Yeni model › Araç ara "TAKS" › Eklentiler › Parsel TAKS hesabı: adım eklendi,
  ayarları "Eklenti Vitrini 1.0.1 ile eklendi." dedi.
- **Sınamalar:** İşlemler 1 (adım sabitlenir, yerleşik araç sabitlenmez, JSON geri okunur, eklentisiz açılışın sözü),
  web 1 (aynı söz, JSON'da kalır), masaüstü 1 (tasarımcıda ekleme, kurulu paketin özeti, satırın dört hâli).
- **Uygularken verilen kararlar:** `ActionPin` → `ExtensionPin` (bir tür); özet host'tan `ExternalTool`'a; sürüm
  farkı uyarı, ret değil; çalışma kökeni (PY-TOOL-22: girdi anlık görüntüsü, parametreler, çıktı kimlikleri) ayrı
  dilim.

## §14. Araç tanımında yapay zekâ açıklaması: `modelDescription` (`ADIM 19`, D18, Ö·39)

**14 Kasım.** Kod: `crates/native/processing/src/external.rs` (`ExternalTool::model_description`,
`MOST_MODEL_DESCRIPTION`), `command.rs` (`described`), `crates/shared/commands/src/gis/cad_processing.rs`
(`ProcessingTool::model_description`, `extension`).

- **Tanım:** `kentos.processingTool` v1'e isteğe bağlı `modelDescription` (en çok 2000 karakter): aracın bir yapay zekâ
  modeli için ne zaman ve nasıl kullanılacağı, kullanıcının `description`'ından ayrı. Eklemeli bir alan; v1 tanımları
  bilinmeyen anahtarı zaten reddetmiyordu. Uzun ya da metin olmayan değer tanımın sorunudur (Kur'dan önce, §10).
- **Liste:** `cad.processing.tools` eklenti aracını `modelDescription` ve `extension` (kimlik, kurulu sürüm, paket
  özeti; §13'ün `ExtensionPin`'i) ile listeler; ikisi de kod yüklenmeden tanımdan ve kayıt defterinden gelir.
  Yerleşik araçlarda yoklar (JSON'da yazılmaz). MCP, Python ve uygulama içi ajan aynı komutu okur: MCP'nin kendi araç
  listesi değişmez, eklenti araçları bu listeyle bulunur (§7).
- **Vitrin 1.0.2:** TAKS tanımına Türkçe `modelDescription` (girdiler, yazılan öznitelikler, dönenler, GeoPackage
  hedefi); `vitrin.kcad`'ın düğmesi yeni özete sabitlendi (bekçi sınaması). Sınamalar yeni sürümü manifestten okur.
- **Sınamalar:** İşlemler 1 (listede açıklama ve eklenti, yerleşik araçta yok, JSON adı, uzun açıklamanın reddi);
  masaüstü ve uygulama sınamaları değişmeden geçti.
- **Uygularken verilen kararlar:** açıklama tanımda, manifestte değil (araç başına); en çok 2000 karakter; listede
  `extension` da verilir (ajan hangi kodun çalışacağını bilsin); `llms.txt` ve kapalı sözlükler ADIM 21'de kalır.

## §15. Çalışma kökeni: katman onu yapan eklentiyi ve ortamı söyler (`ADIM 19`, PY-TOOL-22)

**14 Kasım.** Kod: `crates/shared/contracts/src/gis/dataset.rs` (`LayerDerivation::extension`/`environment`,
`derivation_problem`), `crates/native/processing/src/runner.rs` (`derivation_of`), `external.rs`
(`ExternalTool::environment`), `crates/shared/kcad` (`GIS_WITH_EXTENSION_DERIVATIONS`, `modules_of`, okuyucunun kapısı),
`tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`, `apps/desktop/src/gis/dataset_window.rs` (Soy).

- **Köken zaten vardı (ADR 0633 §2):** bir işlem aracının yazdığı katmanın türetim kaydı aracı, sürümünü, değerleri,
  okunan katmanları içerik özetleriyle (girdinin anlık görüntüsü), çizimin sürümünü, motoru ve zamanı tutuyordu.
  PY-TOOL-22'nin eksiği eklentiydi: aynı araç kimliği iki sürümde başka kod çalıştırabilir.
- **Eklenen:** `extension` (§13'ün `ExtensionPin`'i: kimlik, sürüm, paket özeti; aracın eklentisi olmalı) ve
  `environment` ("Python 3.12.13 · linux-x86_64": yorumlayıcı ve platform; paketlerin ikili dosyaları ikisine
  bağlı). Masaüstü ortamı Python hazır olunca `ExternalTool::environment`'a yazar; işleyici kaydı kurarken ikisini
  araçtan alır. Yerleşik araçlarda ikisi de yok, JSON'da yazılmaz. Çıktı kimlikleri ayrıca yazılmadı: kaydı taşıyan
  katmanın kendisi çıktıdır.
- **Dosyada: CBS verisi sürüm 1 (ADR 8054 §10'un ilk kullanımı).** Yeni alanlar belge şemasını yükseltmez: yazıcı bu
  alanlardan birini taşıyan türetim varken kökün `modules`'üne `gis: 1` yazar (şema en az 254); okuyucu `gis < 1` iken
  iki alanı bilinmeyen alan sayar (`unknown_field`), eski KentOS dosyayı "CBS verisi daha yeni bir sürümle yazılmış"
  diye açmaz. Bağımsız okuyucu (`kcad.py`) ve başvuru yazıcısı aynı kuralla; şema 254'ün ilk geçerli örneği
  `extension-derivation.kcad`, bozukları `derivation-extension-without-gis.kcad` ve `-other-tool.kcad`.
- **Gösterim:** Veri kümesi bilgisi › Soy: "Eklenti: Eklenti Vitrini 1.0.2 (sha256:dc228a05b377…) · Ortam: Python
  3.12.13 · linux-x86_64" (eklenti adı kurulu hâlinden, yoksa kimliği).
- **Gerçek pencere (`10-koken`):** Vitrin 1.0.1 → 1.0.2 katalogdan güncellendi, ortamı hazırlandı; TAKS'ın TAKS
  katmanı › Veri kümesi bilgisi › Soy eklentiyi, özeti ve ortamı gösterdi.
- **Sınamalar:** sözleşme 1 (sabitleme araca uyar, ortam tek satır, JSON), İşlemler 1 (eklenti aracının katmanı
  eklentiyi ve ortamı taşır), KCAD 244 (geçerli dosya sayısı 285 → 286; Rust yazıcısı başvuru yazıcısıyla bayt bayt
  aynı), bağımsız okuyucu 1861 dosyada tutarlı, wasm derlemesi.
- **Uygularken verilen kararlar:** köken yeni bir kayıt değil, türetim kaydının iki alanı; ortam "Python sürümü ·
  işletim sistemi-mimari"; yeni alanlar belge şemasını değil CBS verisinin sürümünü yükseltir.

## §16. Eklentiden eklentiye çağrı ve önizlemede kod yok (`ADIM 19`, PY-RUN-15, PY-RUN-23)

**14 Kasım.** Kod: `apps/desktop/src/shell/extensions/runner.rs` (`others_tool`, `calls_refused`, aracının ilk denetimi).

- **Eklentiden eklentiye (PY-RUN-15):** bir eklentinin kodu aracı üzerinden yalnız kendi araçlarını ve manifestinin
  `dependencies.extensions`'ında bildirdiği eklentilerin araçlarını çalıştırabilir (`cad.processing.run`/`batch`'in
  `ext.<eklenti>.<araç>`'ı); başkasınınki `capability` ile reddedilir ve ne yazılacağını söyler. Bağımlılığın kurulu,
  açık ve uygun sürümde olmasını kayıt defteri zaten sağlıyor (yoksa çağıran "Bağımlılık bekliyor"dur).
- **Döngü, derinlik, bütçe:** çağrılan araç kendi sürecinde çalışır ve aracıya istek yapamaz (havuz her isteği
  "Bu çalıştırma çizime erişemez" diye geri çevirir, §7): zincir bir basamaktır, geri dönemez. Bütçe aracın kendi zaman
  aşımıdır (tanımın `timeoutSeconds`'ı, en çok 3600 sn); çağıran o süre bekler, Durdur ikisini de durdurur.
- **Önizleme ve doğrulama (PY-RUN-23):** eklenti kodu hiçbir önizleme ya da doğrulama yolunda yazma izniyle çalışmaz.
  İşlemler aracının kendi doğrulaması ve önizlemesi yok (pencere KentOS'un kurallarıyla denetler; sınandı); ifade
  işlevinin değeri oluşturucunun ve pencerenin önizlemesinde sorulmaz (§8); çizim sırasında sorulan değerler (etiket,
  stil, süzgeç, form hesabı) havuzdan gelir ve havuz çizime hiçbir isteği geçirmez. Yazma izni yalnız komutun ve harita
  aracının çalıştırmasında, yeteneği verilmişse vardır.
- **Sınamalar:** masaüstü 1 (bağımlılıksız ret, kendi aracı, bağımlılıkla izin, toplu çalıştırma), İşlemler 1 (eklenti
  aracının doğrulaması ve önizlemesi yok); havuzun istek reddi python-env'de sınanıyordu.
- **Uygularken verilen kararlar:** kural aracıda (komutun yolunda), kayıt defterinde değil; ifade işlevleri üzerinden
  dolaylı çağrı (bir eklentinin çalıştırdığı yerleşik aracın ifadesinde başkasının işlevi) sınırlanmadı: değer yalnız
  okunur, yazma yolu yoktur.

## §17. Eklentinin denetim kuralı: bulgu, odak, düzeltme (`ADIM 20`, birinci dilim)

**14 Kasım.** Kod: `crates/native/processing/src/external.rs` (`audit_facts`), `python/kentos/ext.py` (`GisAuditFacts`,
`AuditFinding.to_json`, `Feature.id`), `python/kentos/ext_host.py` (`audit` türü, `findings_of`),
`apps/desktop/src/shell/extensions/audits.rs` (çalıştırma, bulgular, Göster, Düzelt, sekme), `tools.rs` (`PoolJob`),
alt panelin **Eklenti denetimi** sekmesi (`bottom.rs`, üç programda).

- **Olgular:** kural, eklenti sürümünün sürecinde (havuz) çizimin nesne taşıyan katmanlarını veri olarak okur: her
  nesne bir aracın girdisindeki gibi `ext.Feature` (kalıcı kimlik, tür, katman adı, öznitelikler, GeoJSON şekil, tam
  nesne); üçüncü taraftan gizli alan maskelidir. En çok 50 000 nesne: fazlasında `facts.cut` doğru olur ve kural bunu
  söyleyebilir (büyük girdinin parça parça aktarımı PY-TOOL-08'de). Kuralın çizimi yoktur: havuz isteklerini geri
  çevirir, kural yazamaz.
- **Bulgular:** `ext.AuditFinding` (önem: error, warning, info; ileti; nesne kimliği; isteğe bağlı `SemanticFix`:
  başlık, KentOS komutu, girdisi). Başka bir şey döndüren kural `TypeError` ile, yeri söylenerek düşer; bir çalıştırmada
  en çok 10 000 bulgu tutulur, sekme ilk 500'ü listeler, gerisini sayar. Hata eklentinin adıyla söylenir ve günlüğüne
  iz kaydıyla yazılır (§4).
- **Sekme:** şeritteki düğme kuralı arka planda çalıştırır, alt panel Eklenti denetimi sekmesinde açılır: "Eklenti
  Vitrini › Yapı numarası · 5 uyarı (1,6 sn)", Durdur / Yeniden denetle. **Göster** nesneyi seçer ve yakınlaştırır.
  **Düzelt** kuralın komutunu tek geri alma adımı olarak (adı düzeltmenin başlığı) yürütür; yalnız yazma izni
  (`document.write`) verilmiş eklentinin düzeltmesi ve yalnız denetlenen çizimde. Düzeltme eklentinin sözü, basış
  kullanıcınındır; komut KentOS'un sözleşmesiyle denetlenir, reddedilirse adım geri alınır ve nedeni söylenir.
- **Vitrin 1.0.3:** "Yapı numarası" (`yapi-no`): `no`su olmayan her yapı bir uyarı, düzeltmesi sıradaki numarayı yazar
  (`cad.entities.set`). `audit.provider` yeteneği güncellemede "YENİ" diye gösterildi.
- **Gerçek pencere (`11-denetim`):** 1.0.2 → 1.0.3 güncellendi, ortam hazırlandı; düğme 5 uyarı verdi (1,6 sn), Göster
  yapıyı seçip yakınlaştırdı, Düzelt "Y-01 yaz ✓" oldu, geri alma etkinleşti. Bulgu: satırın düğmeleri kaydırma
  çubuğunun altında kalıyordu; listeye çubuk payı verildi.
- **Sınamalar:** Python 1 (olgular veri olarak, bulgular ve düzeltme JSON'u, kesilen olgu, bulgu olmayanın reddi),
  masaüstü 1 (cevabın okunması, bilinmeyen önemin elenmesi, hata metni, Göster, Düzelt tek adım ve geri alma, yazma
  izni olmayanın reddi), programların sekme listeleri.
- **Uygularken verilen kararlar:** olgular bütün katmanlar (manifest katman adı istemiyor); sınır 50 000 nesne, 10 000
  bulgu, 500 satır, 120 sn; düzeltme `document.write` ister; sekme alt panelde ve her programda; ADIM 20'nin ortak
  kural motoru (`ProjectAuditFacts`, bulut işi, PAİY kuralları Python'da) sonraki dilimlerde.

## §18. `kek verify` ve `extension.verify`: pencere açmadan doğrulama (`ADIM 21`, birinci dilim; EKL-06)

**14 Kasım.** Kod: `crates/shared/extensions/src/verify.rs` (`verify`, `Verification`), `apps/cli/src/main.rs`
(`kentos kek verify`), `crates/native/headless/src/agent_tools.rs` (`extension.verify`: MCP ve Asistan).

- **Kapı 1, paket:** paketleyicinin bütün denetimi (yollar, manifest ve başvuruları, yeteneklerin kapsamı, bütünlük
  listesi); işleyicinin modülü pakette yoksa `handler_missing` burada.
- **Kapı 2, Python:** manifestin her giriş noktası (komut, İşlemler aracı, denetim, ifade işlevi, harita aracı) için
  işleyicinin işlevi `src/` altındaki modülde bulunur ve üstünde türünün ve yerel kimliğinin işareti tek başına durur
  (`@ext.audit("yapi-no")`); eksik işaret, yanlış tür, yanlış kimlik, iki işaret ve iki işleve verilmiş aynı kimlik
  sorundur. Uyarılar: `network` yeteneği istemeden ağ kitaplığı (socket, requests, urllib, httpx, aiohttp, http),
  işletim sistemi komutu (subprocess, os.system, os.popen), `eval`/`exec`. Kaynak metin olarak okunur; içe aktarılmaz,
  çalıştırılmaz (paylaşılan crate, bağımlılıksız, wasm'da da derlenir).
- **Kapı 4, rapor:** her bulgu kodu, dosyası, satırı ya da JSON işaretçisi ve ne yazılacağıyla: "src/vitrin/logic.py
  satır 155: “yapi_no” işaretsiz: üstüne @ext.audit("yapi-no") yazın."; yanlış yazılmış işleyiciye en yakın işlev
  önerilir ("En yakını: “taks”", düzenleme uzaklığı ya da en az dört harflik ortak baş).
- **Yüzeyler:** `kentos kek verify KLASÖR [--json]` (doğrulanınca içerik özeti; sorunda çıkış 1); ajan aracı
  `extension.verify {path}` klasör ya da `.kek` alır, yalnız okur. ADR 8056 §2'nin "`extension.*` Faz 3'e kaldı"
  notunun ilk karşılığı.
- **Kapı 3** (sentetik çizimde kuru koşu) KentOS'un Python'unu ister: masaüstüyle sonraki dilim.
- **Sınamalar:** extensions 3 (Vitrin her kapıdan geçer; işaretsiz, yanlış tür, yanlış kimlik, eksik işlev ve önerisi,
  pakette olmayan modül, iki kez kimlik; ağ ve işletim sistemi uyarıları, ad içindeki sözcük çağrı sayılmaz), headless
  ajan araçları 1 (Vitrin klasörü geçerli; işaretini yitiren kopya satırıyla reddedilir).

## §19. `extension.blueprint` ve `kek new`: doğrulamadan geçen iskelet (`ADIM 21`, ikinci dilim)

**14 Kasım.** Kod: `crates/shared/extensions/src/blueprint.rs` (`Ask`, `Contribution`, `blueprint`),
`apps/cli/src/main.rs` (`kentos kek new`), `crates/native/headless/src/agent_tools.rs` (`extension.blueprint`).

- **Ne verir:** manifest (kimlik, ad, özet, tür, program, uyumluluk, Python projesi, giriş noktaları, katkıların
  istediği yetenekler ve nedenleri; dikey eklentide şerit sekmesi), `pyproject.toml` (bağımlılıksız, üç sistem),
  `uv.lock` (uv 0.11'in bağımlılıksız proje için yazdığı kilit; `uv lock --check` geçer), ikon, MIT lisansı,
  değişiklik notu ve istenen her katkının küçük, çalışan örneği: katmanları okuyan komut, tanımlı İşlemler aracı
  (alan özeti, `modelDescription`'ıyla), düzeltmeli denetim (adsız nesneler), ifade işlevi (Türkçe büyük harf),
  harita aracı (işaret koy). Komut yalnız okuyorsa `document.read`, harita aracı varsa `document.write`.
- **Doğruluk inşa gereği:** on iki birleşimin (beş katkı tek tek, hepsi; dikey ve yardımcı) hepsi `verify`'dan sorunsuz
  ve uyarısız geçer; Python ayrıştırıcısı örnekleri ayrıştırır; masaüstü iskeleti paketleyip incelemesinden (kendi araç
  denetimi dahil) geçirerek kurar, sekmesinde dört örnek, İşlemler'de aracı görünür.
- **Yüzeyler:** `kentos kek new KLASÖR --id … --name … [--summary …] [--module …] [--utility] [--with …]` yalnız boş ya
  da yeni klasöre yazar; ajan aracı `extension.blueprint` dosyaları içerik olarak verir, hiçbir şey yazmaz (yazma ve
  kurma kullanıcının ve ajanın açık adımı, PY-AI-05).
- **Bulgu (düzeltildi):** Python şablonları önce Rust dizesinde satır devamlarıyla yazılmıştı; devam satırının baştaki
  boşlukları silindiği için iç içe bir bloğun girintisi bozuluyordu. Şablonlar ham çok satırlı dizelere taşındı ve
  sınama Python'un kendi ayrıştırıcısıyla denetliyor.
- **Uygularken verilen kararlar:** Python paket adı kimliğin son parçasından (`mezarlik-bilgi` → `mezarlik_bilgi`);
  sürüm 0.1.0; harita aracı `taslak` katmanına yazar (CBS şablonunda var); `kentos://sdk/ai-kek-pack` (SDK başvurusu,
  ikon listesi, şemalar tek okumada) ve Kapı 3 sonraki dilimlerde.

## §20. `kentos://sdk/ai-kek-pack`, `extension.sdk` ve `kek sdk`: SDK tek okumada (`ADIM 21`, üçüncü dilim)

**14 Kasım.** Kod: `crates/shared/extensions/src/sdk_pack.rs` (`GUIDE`, `EXT_STUB`, `PARTS`, `sdk_pack`, `sdk_part`),
`crates/shared/extensions/sdk/kentos_ext.pyi` (üretilmiş), `crates/native/mcp/src/tools.rs` (kaynak),
`crates/native/headless/src/agent_tools.rs` (`extension.sdk`), `apps/cli/src/main.rs` (`kentos kek sdk`).

- **Ne verir (33 KB, dokuz bölüm):** `guide` (Türkçe rehber: paket, beş işaret ve imzaları, bağlamın ortak yöntemleri,
  kurallar), `api` (`kentos.ext`'in Python imzaları, 9 KB'lık stub), `capabilities` (15 yetenek: kimlik, kurulumda
  görünen başlık, kapsamlar), `modules`, `contributions` (sekiz katkı: işareti, manifest anahtarı, istediği yetenek),
  `toolDescriptor` (İşlemler tanımının 8 parametre, 4 çıktı türü ve alanları), `verifyCodes` (doğrulama kodlarının
  anlamı), `workflow` (istekten pakete beş adım; kurma, yetki ve yayın kullanıcının) ve `example` (beş katkının hepsiyle
  dikey bir iskelet: `verify`'dan sorunsuz, uyarısız geçer).
- **Kayma yok:** yetenekler `Capability::ALL`'dan, örnek `blueprint`'ten, stub `python/kentos/ext.py`'den üretilir.
  Stub depoda durur (`include_str!`, paylaşılan crate dosya sistemi okumaz, wasm'da derlenir); sınama kaynaktan yeniden
  üretip karşılaştırır, fark varsa `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack` der.
  Rehberin her iddiası Python SDK'sıyla tek tek karşılaştırıldı (bağlam yöntemleri, `Feature` alanları, `NewFeature`'ın
  `source`/`geometry` yolu, `GisAuditFacts`, `AuditFinding`, `SemanticFix`, `MapToolContext.add_feature`).
- **Yüzeyler:** MCP kaynağı `kentos://sdk/ai-kek-pack` (yapının malı: bir saat, herkese aynı önbellek ipucu); ajan
  aracı `extension.sdk` (`part` ile tek bölüm; bilinmeyen bölüm bölümlerin listesiyle reddedilir; masaüstü Asistan'ın
  48 KB sonuç sınırının altında); `kentos kek sdk` rehberi ve imzaları okunur biçimde, `--json` paketi, `--part`
  bölümü verir.
- **Uygularken verilen kararlar:** stub `python/kentos/ext.pyi` olarak YAZILMADI: aynı klasördeki `.pyi`, tür
  denetleyicilerinde `ext.py`'nin tam belgelerini gölgelerdi. `to_json` yöntemleri (KentOS'un sonucu okuması) ve `_`
  ile başlayanlar stub'da yok; `__geo_interface__` var. Belge ilk cümleyle ve 400 karakterle sınırlı. MCP sunucusu
  `kentos-extensions`'ı artık normal bağımlılık olarak alıyor (önceden yalnız sınamalarda).
- **Kalan (ADIM 21):** geçerli yerleşik ikon adlarının listesi (masaüstünün ikon kümesinden üretilmeli; paylaşılan
  crate onu bilmiyor), `kentos.ui` bileşen ve `ProjectAuditFacts` şemaları (ilgili adımlarla), Kapı 3 kuru koşu,
  `hot_reload`/`preview_render`.

## §21. Yerleşik ikonlar tire biçiminde: 1346 adın hepsi yazılabilir (`ADIM 21`, kod üretimi)

**14 Kasım.** Kod: `crates/shared/extensions/src/icon_names.rs` (`LIST`, `is_builtin`, `web_name`, `nearest`),
`crates/shared/extensions/sdk/icons.txt` (üretilmiş), `manifest.rs` (`manifest_icon_unknown`), masaüstü
`extensions/ribbon.rs` (`icon_of`).

- **Bulgu (ölçüldü):** masaüstünün ikon kümesi (`docs/inventory/web.json` 875 + `catalog/v1/additions.json` 471 = 1346
  ad) camelCase yazıyor (`zoomExtents`), manifest ise yerleşik ikon adını küçük harf ve tireyle istiyor. Yalnız 213 ad
  zaten küçük harfti; geri kalan **1133 ikonu bir eklenti hiç adlandıramıyordu** (manifest reddediyordu).
- **Karar:** manifest tire biçiminde kalır (kimliklerle aynı yazım), masaüstü `zoom-extents`'i `zoomExtents`'e çevirir.
  Dönüşüm 1346 adın hepsinde kayıpsız ve çakışmasız (sınama her adı gidip geri getirerek denetler); tiresiz bir ad
  kendisidir, eski manifestler değişmeden çalışır.
- **Liste:** iki JSON'dan sınamada üretilir, depoda durur (`include_str!`); küme değişince sınama kırılır ve
  `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- icon_names` der. SDK paketinde `icons` bölümü (tamamı;
  pakette yalnız sayısı ve nasıl alınacağı, çünkü 19 KB'lık liste paketi Asistan'ın 48 KB sonuç sınırının üstüne
  çıkarırdı; paket 33 KB'ta kalır ve sınama bunu tutar), `kentos kek sdk --part icons` her satıra bir ad yazar.
- **Bilinmeyen ad ret değil uyarıdır** (`manifest_icon_unknown`): sonraki bir KentOS için yapılmış paket bu KentOS'ta
  olmayan bir ikon adlandırabilir; düğmesi düz simgeyle çizilir. Uyarı en yakın adları önerir: bir iki harflik yazım
  hatası (`space-numbr` → `space-number`), onu içeren ya da onda geçen ad (`number` → `…-number`), harflerinin üçte
  biri kadar uzak ad; hiçbiri yoksa "yakın bir yerleşik ad yok" (rastgele harf ortaklığı önerilmez: `mezar`).
- **Bulgu (düzeltildi):** `verify` paketleyicinin ve inceleyicinin aynı manifest uyarısını ayrı ayrı topladığı için her
  manifest uyarısını iki kez yazıyordu (güvensiz bağlantı uyarısı dahil); artık bir kez.
- **Vitrin 1.0.4:** "Yapıları numarala" `space-number`, katmanın sağ tık "Katman özeti" öğesi `layer-fields` (ikisi de
  camelCase adlı). Gerçek pencerede (`12-ikonlar`): 1.0.3 → 1.0.4 güncellendi; sekmede ve sağ tık menüsünde ikonlar
  web'deki çizimleriyle göründü. Örnek çizim yeniden üretildi (formun ve modelin sabitlediği paket özeti değişti).
