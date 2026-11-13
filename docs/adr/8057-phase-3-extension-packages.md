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
  `sha256:3e1163602de302f2deebf2a90a21d661c9e446aa2de285445b37279fc90e2002`.
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
