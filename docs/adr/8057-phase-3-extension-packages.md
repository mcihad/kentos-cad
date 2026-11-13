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
  `sha256:e3816e901e3241d238547ddd25a9d31731d0e6931e96752ee8ff8507030a3b1b`.
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
- **Sonraya kalan (ADIM 19'un kalanı):** İşlemler araçları (tanım → parametre penceresi, NumPy aracı, tipli sonuç →
  yeni katman), ifade işlevleri, denetimler, harita aracı, form eylemleri, MCP eşdeğerliği ve sürümlü komut kaydı,
  boştaki sürecin kapanması ve süreç başına bellek ölçümü.
