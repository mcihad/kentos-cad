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
| Uyarılar | `manifest_link_insecure`, `manifest_capability_unused`, `manifest_panel_unopened` (16 Kasım: komutu olmayan eklentide hiçbir şerit ya da sağ tık öğesinin açmadığı panel; Asistan'ın yazdığı Python'suz yardımcı eklentinin paneli açılamıyordu), `manifest_python_unused`, `catalog_changelog`, `form_preflight_warning`, `text_user_path` (düzyazıda), `python_lock_version`, `python_vcs_source`, `python_registry` |

### Altın örnekler

- `fixtures/extensions/v1/mebis/`: dikey eklenti; komut, harita aracı, İşlemler aracı, denetim, ifade, panel, iki form,
  Python'suz veri kaynağı, şema, çeviri, ikon; `pyproject.toml` ve **gerçek `uv lock` çıktısı** (uv 0.11.26; `shapely`,
  isteğe bağlı `pygal`; üç sistem `environments` ve `required-environments`'ta). Paket 14 382 bayt, 16 dosya, içerik özeti
  `sha256:e3816e901e3241d238547ddd25a9d31731d0e6931e96752ee8ff8507030a3b1b`. 14 Kasım: araç tanımı ADIM 19'da
  kesinleşen v1'e geçti (taslağın `id`/`accepts`/`destination` biçimi kurulumdaki tam denetimden geçmiyordu); özet
  `sha256:ec9730da3c4841b7ec3b5ff82103e90db52766eb7d3e3c3da47ebec40ac46501` (Python'da ayrıca hesaplandı). 14 Kasım
  (§23): şema `kentos.layerSchema` v1'in sözcüklerine geçti (`area`, ilişkinin `id`/`parent`/`child`'ı, grup); özet
  `sha256:1a6af9c39da91b1fd7552a85204bf0ffea6c2e650c64bb06f936302db3bd317a` (aynı Python hesabı eskisini de birebir
  verdi); katmanlarına formları bağlanınca `sha256:05585d1c96e53cb10e90ca819f9fe121db6c359830437ea1a8aaa2266b09a05f`; arama paneli
  v1'e geçip `search` komutunu alınca (§24) `sha256:a0bf2174415400e8cdd44b8942f11cad47c67ff15bb3aee39b9a00446f727a97`;
  denetimin düzeltmesi `operation`'ını, doluluk aracı Durdur'u SDK'nın yoluyla alınca `sha256:6ef13bdd8590e97126909bd494480d1e3345183599b095dc03d6dd18fa05a9bb`; arama nesnenin özniteliklerini okuyup satırlarına `uid`, okunur başlıklar ve durumun adını verince (§24 kabulü) `sha256:750a026ca3d9a75da06e660da0d068931c63ca9c151f6e6078b82cbf1ceb4054`; ikinci İşlemler aracı (Boş mezar
  yerleri: boşları yeni katmana yazar, sayısını verir) eklenince `sha256:4761924ff6428198c016ec3cb78d1c2929e1bf70b0a7dbabc09fc650e78bbe5b`.
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
   yetenekler; eklenti başına en düşük sürüm; dış yapay zekâya veri izni (uygulama içi ajanı da bağlar, izinli yapay
   zekâ adresleriyle: ADR 8056 §7). Host sistem klasöründen okur, kullanıcı gevşetemez; kurulu bir eklentiyi sonradan
   yasaklarsa eklenti durur, silinmez.
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
- **`snapLayers` (15 Kasım):** manifestte doğrulanıyordu ama hiç kullanılmıyordu: MEBİS'in mezar aracı her katmana
  kenetleniyordu. Artık aracın noktaları yalnız bildirdiği katmanlara kenetlenir: kenetleme sorusu (`SnapExtras`)
  katman süzgeci taşır (`layers`; çizimde olmayan katman hiçbir şey vermez, süzgeçsiz araç eskisi gibi), etkileşim
  aracı bildirir (`Tool::snap_layers`), yakalama aracı manifestten alır (`Capture::snapping_to`). Gerçek pencerede aynı
  parsel köşesinde Tekli mezar kenetlenmedi (parsel `snapLayers`'ta yok), Çizgi "Uç nokta · kapalı alan" dedi. Sınamalar:
  çekirdekte katman süzgeci, oturumda yakalama aracının süzgeci (adı verilen, başka ve boş liste).

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
- **Köken** (15 Kasım; ADIM 20'nin "köken" maddesinin eklenti payı): her denetim çalışması kuralın geldiği eklentinin
  sürümünü, paket özetini ve okuduğu çizim revizyonunu taşır; Eklenti denetimi sekmesi yazar ("Köken: Eklenti Vitrini
  1.0.19 · yapi-no · paket cbfbe7b8845d · çizim revizyonu 0"). Sonradan değişen söylenir: eklentinin paketi
  değiştiyse "Eklenti denetimden sonra değişti (1.0.19 → 1.0.20): bulgular eski kural sürümüyle üretildi; yeniden
  denetleyin" (aynı sürümde paket değiştiyse onu söyler), çizim değiştiyse "Çizim denetimden sonra değişti
  (düzeltmeler dahil): yeniden denetlemek bulguları tazeler". Planın dersi (QEP 425 §2.4.1): sürüm değişince eski
  raporun hangi kural sürümüyle üretildiği okunur. Gerçek pencere: Yapı numarası 5 uyarı, köken satırı; Düzelt: Y-01
  yaz'dan sonra çizim notu. Bulutla ortak rapor biçimi ve betik özeti ADIM 20'nin kalanı.

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

## §22. Ajanların eklenti atölyesi: yaz, doğrula, incelemeye aç (`ADIM 21`, akış ve yüzeyler)

**14 Kasım.** Kod: `crates/native/headless/src/agent_tools.rs` (`extension.workshop.write`, `.read`, `.review`;
`Host::workshop`, `Host::review_package`), `crates/native/extension-host/src/places.rs` (`workshop_root`,
`workshop_for`, `workshop_beside`), masaüstü `assistant/host.rs`, MCP `tools.rs` (`WithWorkshop`).

- **Bulgu:** masaüstü Asistan'ı `extension.sdk` ve `extension.blueprint` ile eklentinin dosyalarını alıyor ama diske
  yazacak aracı olmadığı için akış yarıda kalıyordu; dış ajanın kendi dosya araçları var, Asistan'ın yok.
- **Atölye:** `…/kentos-cad/eklenti-atolyesi/<kimlik>/` (eklenti klasörünün yanında; `KENTOS_EXTENSIONS_WORKSHOP`
  başka yer). Masaüstünde uygulamanın kendi kayıt defteri klasörünün yanında: sınama ve gerçek pencere oturumu
  kullanıcının klasörüne yazmaz. Atölyesi olmayan bağlantıya araçlar hiç sunulmaz.
- **`write`:** dosya yolu → metin (`null` siler). Her yol paketin yol kuralıyla (`kentos_extensions::path_problem`:
  `..`, mutlak yol, ayrılmış ad), dosya başına 512 KB, çağrı başına 4 MB; hepsi denetlenmeden hiçbir şey yazılmaz
  (kötü yollu çağrının iyi dosyası da yazılmadı, sınandı). Atölye dışına yazmaz, hiçbir şey çalıştırmaz.
- **`read`:** atölyedeki eklentiler (ad, sürüm), bir eklentinin dosyaları, bir dosyanın içeriği. **`verify`** artık
  `id` ile de çalışır.
- **`review`:** doğrular; geçmeyen eklenti paketlenmez ve sorunlarını döndürür. Geçeni klasörün YANINA paketler
  (`<kimlik>-<sürüm>.kek`; içine yazsaydı bir sonraki paket öncekini taşırdı) ve host'a incelemeye açtırır: masaüstünde
  kullanıcının `.kek` açtığındaki Gözden geçir ve kur penceresi. Kurmak ve yetki vermek kullanıcınındır (PY-AI-05);
  açmak hiçbir şey kurmaz (sınandı). MCP sunucusu paketi açamaz, Paket yükle… ile açılacağını söyler.
- **Onay:** atölyeye yazmak Asistan'da onay beklemez (kendi klasörü, panelde her çağrı görünür); kurulum zaten
  kullanıcının penceresi. Asistan'ın yönergesine akış yazıldı: sdk → blueprint → workshop.write → verify →
  workshop.review.
- **Bağlantı yok:** atölyede önceden duran bir sembolik bağlantı (`src` → `/etc`) yazmayı, okumayı ya da paketi
  atölyenin dışına taşıyabilirdi. Yazma ve okuma yolu bağlantıdan geçmez, klasör okuyucusu (doğrulama ve paket)
  bağlantıyı komut satırı paketleyicisi gibi reddeder (sınandı: dışarıya hiçbir şey yazılmadı, paket açılmadı).
- **Bulgu (gözden geçirme):** `extension.blueprint`'in ve SDK'nın "sıradaki adım" metinleri dosya araçları olan dış
  ajana göre yazılmıştı ("dosyaları boş bir klasöre yazın", "`kentos kek pack`"); uygulama içindeki ajan için bu
  "kullanıcıya bırak" demekti. Artık atölyesi olan ajana atölyenin adımları söylenir, olmayana klasörün; SDK'nın akışı
  ikisini ayrı yazar ve atölyede uv çalışmadığını (kütüphanesiz yazılacağını) söyler.
- **Gerçek pencere (DeepSeek `deepseek-v4-pro`, test anahtarı yalnız sürecin ortamında, sentetik istek ve Vitrin
  çizimi):** "Mezarlık için bir eklenti yaz ve kurulum için incelemeye aç…" isteğiyle Asistan SDK'yı ve iskeleti okudu,
  harita aracını (dikdörtgen, `mezar` katmanına ad/ada/parsel) ve adsız mezar denetimini yazdı, doğruladı; yaklaşık 2
  dakikada Gözden geçir ve kur penceresi üç yeteneği gerekçeleriyle gösterdi, Kur kullanıcıda kaldı. Üretilen paket
  `kentos kek verify` ve `inspect`'ten 0 uyarıyla geçti. Ajan, `mezar` katmanının çizimde bulunması gerektiğini kendisi
  söyledi (katman şeması kurulumu ADIM 22). Oturum betiği sahibin ajan dosyasını kopyalamaz; yalnız oturuma verilen
  ayar klasöründekini.
- **Uygularken verilen kararlar:** paket atölyede kalır (silinmez, kullanıcı yeniden açabilir); kaynak türü "dosya"
  olarak görünür; MCP'nin masaüstü bağlantısında incelemeye açma §30'da.
- **Canlı yükleme ve görünüş** (15 Kasım; planın `hot_reload`/`preview_render`'ı, `extension.workshop.reload` ve
  `.preview`; masaüstü `extensions/workshop.rs`): atölyeden incelemeye açılan paket artık **geliştirici klasörü**
  kaynağıyla kurulur (kayıt defterinin D2 yolu: her yüklemede klasör yeniden paketlenir, aynı sürüm yeni kira ile
  üstüne kurulur, eskisinden bir şey kalmaz, PY-RUN-09). Kaynak türü artık "geliştirici klasörü" görünür.
  - **Kullanıcı bir kez karar verir, ajan yineler:** ilk kurulum kullanıcının Gözden geçir ve kur penceresi. Sonra
    `reload` klasörü doğrular (review'un denetimi) ve yükler; yalnız atölyeden kurulana (dosyadan, katalogdan ya da
    başka klasörden kurulan "atölyeden kurulmadı" der), yalnız kullanıcının verdiği yeteneklerle: yeni yetenek isteyen
    eklenti yüklenmez, "yetenekleri kullanıcı verir: review ile açın" der. Her yükleme kullanıcıya bildirim ve
    eklentinin günlüğüne kayıt; eklentinin süreci bırakılır (bir sürümün süreci çalıştırmalar arasında tutuluyordu,
    eski kodu çalıştırırdı); çalışması sürerken yüklenmez. Değişiklik yoksa "kurulu hâli aynı" der.
  - **`preview`:** kurulu eklentinin kullanıcının gördüğü hâli, uygulamanın kendi modelinden: şerit sekmesi (gruplar,
    düğmeler, ne açtıkları, `when` ya da Python hazır değilse kapalı olma nedeni), paneller (başlık, yer, bileşenler,
    düğmelerin komutları), kayıt defterinin katkıları (İşlemler araçları, denetimler…), sağ tık öğeleri, durum ve
    Python ortamı. Görüntü yerine yazı: ajanın okuyacağı kanıt; ekran görüntüsü ayrı iş.
  - **Kullanıcı bilerek karar verir:** inceleme penceresi atölye paketinde kaynağı klasörüyle yazar (".kek" yolu değil:
    yüklemeler oradan gelir) ve kurulmadan önce söyler: "Atölyeden kuruluyor: kurulduktan sonra Asistan değişiklikleri
    bu klasörden uygulama kapanmadan yükleyebilir; her yüklemede size bildirilir. Yeni bir yetenek isterse yeniden
    sizin incelemenize gelir."
  - **Gerçek pencere (Asistan, DeepSeek test anahtarı yalnız sürecin ortamında, sentetik istek, Vitrin çizimi):** ajan
    "Canlı Deneme"yi (katmanları nesne sayılarıyla raporlayan komut, kendi sekmesi) yazdı ve incelemeye açtı; Kur
    kullanıcıda: "Canlı Deneme 0.1.0 kuruldu". İkinci istekte ("düğmenin adını Nesne sayımı yap, canlı yükle, kullanıcının
    ne gördüğünü kontrol et") manifesti değiştirip `reload` ve `preview` çağırdı; günlükte "0.1.0 yeniden kuruldu
    (geliştirici klasörü)" ve "Atölyeden yeniden yüklendi (Asistan)", şeritte yeni ad. Ajanın yanıtı görünüşten okundu:
    "Canlı Deneme sekmesi → Canlı Deneme grubu → büyük Nesne sayımı yap düğmesi; düğme şu an kapalı, sebebi Python
    ortamının hazırlanmamış olması: Eklentiler › Canlı Deneme › Python ortamı". (Ajan istenen adın sonuna "yap" ekledi:
    ajanın tercihi.)
  - MCP'nin masaüstü bağlantısında da var (§30, 15 Kasım); MCP'nin kendi çizimlerinde yok: araçlar ajanı masaüstüne
    bağlamayı ya da Asistan'ı söyler.
  - Test: başsız (doğrulanmayan klasör yüklenmez, host'a gitmez; geçen yüklenir; preview geçer) ve masaüstü (Vitrin
    atölyeden kuruldu, panel adı değişti, yeniden yüklendi: şeritte yeni ad, aynı sürüm; değişiklik yoksa söyler;
    preview sekmeyi, panelin düğmesini ve TAKS aracını yazar; `files.userSelected` isteyen yükleme reddedildi, şerit
    eski hâliyle kaldı).

## §23. Eklentinin katman şeması: `schema.json` ve `apply_declarative_schema` (`ADIM 22`, katman şeması kurulumu)

**14 Kasım.** Kod: `crates/shared/extensions/src/layer_schema.rs` (Kapı 1), `python/kentos/ext.py`
(`CommandContext.apply_declarative_schema`, `_install_schema`), `cad.layers.edit`'in `id`'si
(`crates/shared/commands/src/core/cad_layer_tree.rs`, `crates/native/application/src/gis/layers_edit.rs`), iskeletin
`schema` katkısı, Vitrin 1.0.5.

- **Bulgu (§22'nin gerçek pencere denemesi):** ajanın yazdığı harita aracı `mezar` katmanına yazıyordu ama eklentinin
  katman kurma yolu yoktu (`apply_declarative_schema` `NotAvailable` idi); ajan kullanıcıya "önce katmanı oluşturun"
  demek zorunda kaldı.
- **Biçim `kentos.layerSchema` 1, KentOS'un kendi sözcükleriyle:** `layers` (`name`: çizimdeki kimliği; `title`;
  `geometry`: point, line, area, annotation, mixed; `fields`: KentOS katman alanının alt kümesi `name, kind, alias,
  required, default, length, min, max, minExclusive, maxExclusive, domain, multiple`), isteğe bağlı `group` (`id`,
  `name`), `domains` (`cad.domains.edit`'in listesi), `relations` (`cad.relations.edit`'in ilişkisi; `parent`/`child`
  katman kimlikleri). `format` ve `version` isteğe bağlı; daha yeni sürüm "daha yeni bir KentOS gerekir" der. Taslağın
  `polygon`/`none`/`from`-`to` sözcükleri bırakıldı (MEBİS ve Kent fikstürleri geçirildi; MEBİS özeti yeniden
  hesaplandı, §1).
- **Katmanın kimliği şema adıdır:** `cad.layers.edit` `add` isteğe bağlı `id` aldı (geriye uyumlu; harf, rakam, `_`,
  `-`, `.`, harfle başlar, 1–64; ağaçta olan, biçimsiz ya da `add` dışında verilen kimlik `invalid_layer_id`, hiçbir
  şey yazılmaz). Alan modeli bunu zaten taşıyordu (`NewLayer.id`); şablonun katmanları da kimlikleriyle bulunuyor
  (`parsel`). Eklentinin harita aracı ve denetimi katmanı bu kimlikle bulur.
- **Kurulum Python'da, mevcut komutlarla:** grup → değer listeleri → katmanlar ve alanları → ilişkiler (alan listeyi,
  ilişki katmanları gösterdiği için bu sırayla), eklentinin sürecinde aracının üzerinden; çalıştırmanın tek geri alma
  adımı. Yeni bir Rust host komutu gerekmedi. **Var olana dokunmaz, yalnız eksiği ekler:** katmanın eksik alanları
  kullanıcının kendi alanlarının sonuna eklenir; ikinci çalıştırma revizyonu değiştirmez (sınandı). Sonuç satır satır
  rapor (Tür, Kimlik, Ad, Sonuç) ve tek cümle ("Şema kuruldu: 1 grup, 1 değer listesi, 2 katman, 1 ilişki.").
- **Kapı 1:** paket incelemesi şemayı denetler, her sorun JSON işaretçisiyle (`layer_schema`): bilinmeyen alan,
  biçim/sürüm, ad kuralı ve tekliği, katman türü, alan türü, şemada olmayan listeye bağlanan alan, tekrarlanan kod,
  ilişkinin olmayan alanı. Şemanın kurmadığı katmanı adlandıran ilişki uyarıdır (`layer_schema_outside`: projede
  bulunmalı, ör. şablonun `parsel`'i).
- **Bulgu (düzeltildi):** Python'un `"İlişki".lower()`'ı "i̇lişki" (i + birleşik nokta) üretiyordu; ileti sözcükleri
  açık bir eşlemeyle yazıldı.
- **İskelet ve SDK:** `extension.blueprint`'te `schema` katkısı (`schema.json` + "Katmanları kur" komutu, yerleşik
  `layer-add` ikonuyla; harita aracı da seçilirse dikdörtgen çizip şemanın katmanına yazar); SDK rehberinde bölüm,
  katkı tablosunda satır, stub'da yeni imza.
- **Gerçek pencere (`13-sema`, Vitrin 1.0.5):** 1.0.4 → 1.0.5 güncellendi, ortam hazırlandı; "Katmanları kur" Vitrin
  grubunu, Örnek tesis (nokta) ve Bakım kaydı katmanlarını, Tesis türü listesini ve ilişkiyi kurdu; ikinci basış
  "Şema zaten kurulu: değişiklik yok." dedi.
- **Gerçek pencere, uçtan uca (Asistan, DeepSeek test anahtarı, sentetik istek):** "Kendi katmanını kursun…" diye
  istenen Mezarlık Bilgi'yi ajan bu kez `schema.json`'uyla yazdı (grup `mezarlik`, `mezar_yeri` alan katmanı: ada,
  parsel, ad; harita aracı dikdörtgeni `mezar_yeri`'ne, denetim aynı katmana bakıyor), paket 0 uyarıyla doğrulandı.
  Kullanıcı gibi: kurum kataloğundan kuruldu, ortam hazırlandı ("kurulacak paket yok"), sekmede Katmanları kur grubu
  ve katmanı kurdu, Mezar yeri çiz dikdörtgeni yazdı (nesne 9 → 10), Adı boş mezarlar 1,4 sn'de 1 uyarı verdi, Göster
  seçip yakınlaştırdı, "Meçhul olarak işaretle" düzeltmesinden sonra yeniden denetim "bulgu yok" dedi.
- **Formlar şemayla birlikte:** katman isteğe bağlı `form` alır (paketteki `.kform`; manifestin `entrypoints.forms`'unda
  bildirilmiş olmalı ki inceleme onu form paketi olarak okusun, yoksa `layer_schema`). Kurulum formu
  `cad.forms.import_` ile katmana katmanın adıyla (form kimliği) ve eksik alanları ekleyerek bağlar; tekrarda "vardı".
  MEBİS'in iki katmanı formlarına bağlandı (özet yeniden hesaplandı), Vitrin 1.0.6 Örnek tesis formunu taşır.
  Gerçek pencere: "Şema kuruldu: 1 grup, 1 değer listesi, 2 katman, 1 form, 1 ilişki."
- **Alt türler ve değer bileşimleri** (15 Kasım; Vitrin 1.0.16, `23-alt-tur`): katman isteğe bağlı `subtypes`
  (KentOS'un `LayerSubtypes`'ı, ADR 0422: değer listeli tek değerli alan ve türleri; her tür öbür alanların kodlarını,
  varsayılanını, zorunluluğunu daraltır) ve `contingents` (değer grupları, ADR 0423: 2–8 değer listeli alan, kısıtlayıcı
  ya da uyarıcı, satır başına `code`/`any`/`empty` hücreleri, isteğe bağlı alt türü) alır; sözcükler KentOS'un, yeni
  bir host komutu gerekmedi: kurulum alanlardan ve formlardan sonra `cad.layers.subtypes` ve `cad.layers.contingents`
  ile, bunları taşıyan katmana dokunmadan. Kapı 1: anahtarlar, alanın katmanda ve değer listeli tek değerli olması,
  kodların şemadaki listede olması, tekrarlanan kod/ad, hücre sayısı ve türü, satırın alt türü, KentOS'un sınırları
  (256 tür, 50 grup, 8 alan, 10 000 satır). Rapor "1 katmana alt tür, 1 katmana değer bileşimi" der.
  - **Gerçek pencere:** Katmanları kur "Şema kuruldu: 1 grup, 3 değer listesi, 2 katman, 1 form, 1 katmana alt tür, 1
    katmana değer bileşimi, 1 ilişki"; Örnek tesis'e konan noktada Tür = Okul: Kademe yalnız İlkokul, Ortaokul, Lise;
    Lise seçilince İşletme yalnız Bakanlık ve Özel. KentOS kuralları yazmada da uyguladı (Python, gerçek belge): Lise +
    Belediye "değer grubunda bu bileşim yok", Okul'a Mahalle parkı "bu alt tür bu değeri sunmaz" diye reddedildi.
  - **Bulgu (Kapı 1 uyarısı eklendi): her satırı bütün alanları dolduran kısıtlayıcı grup alan alan doldurulamaz.**
    Vitrin'in ilk şemasında Kademe yazılınca İşletme boş olduğu için reddediliyordu (KentOS'un kuralı: grubun bir alanı
    doluysa her hücre bir satırca karşılanmalı); öznitelik tablosu bir alanı yazar, kayıt hiç doldurulamıyordu. Kapı 1
    artık hiçbir alanın ilk yazılamadığı kısıtlayıcı grubu uyarır (`layer_schema_fill`); Vitrin'e `[any, empty]` ve
    `[empty, any]` satırları eklendi, rehber bunu söyler.
  - **Bulgu (düzeltildi, Öznitelikler): alt türün zorunlu kıldığı alan işaretsizdi.** Panel seçenekleri alt türe ve
    gruplara göre daraltıyordu ama zorunluluğu katmanın alanından okuyordu: Okul'da Kapasite'nin yıldızı ve uyarısı
    yoktu, boşaltan yazma ise reddediliyordu. Artık işaret ve uyarı kaydın alt türünü okur, sözler yazmayı reddeden
    denetiminkiyle aynı ("“Kapasite” alanı “Okul” alt türünde zorunlu; boş bırakılamaz.").
- **Kalan:** geometrisiz tablo katmanı KentOS'ta henüz yok (kayıtlar `mixed` katmana yazılır).

## §24. Eklentinin paneli: `kentos.ui.panel` 1, düğmesi bir komut (`ADIM 22`, panel köprüsü, ilk dilim)

**14 Kasım.** Kod: `crates/shared/extensions/src/ui_panel.rs` (tipler, Kapı 1), masaüstü
`extensions/panels.rs` (durum, olaylar, düğme; görünüm 15 Kasım'da `panel_view.rs`'e ayrıldı: dosya 1855 satıra
ulaşmıştı, sınır 2000), `runner.rs` (`PanelCall`, `ExtRun.origin`), `app.rs`
(`Panel::Extension`), Vitrin 1.0.7, MEBİS'in arama paneli.

- **Karar: panel veridir, düğmesi bir komuttur.** `kentos.ui.panel` 1'in yedi bileşeni KentOS'un kendi
  bileşenleriyle çizilir: `text`, `textField`, `numberField` (birimi, sınırları), `select`, `checkbox`, `button`,
  `table`. Eklentinin kodu bir arayüz nesnesi ya da araç takımı tipi almaz (QGIS'te Qt değişince paketlerin %58'i
  geride kaldı, D7). Düğme `entrypoints.commands`'tan bir komutu panelin değerleriyle çalıştırır: değerler
  `ctx.params`'tadır (bileşen kimlikleriyle; metin kırpılır, sayı virgül ya da noktayla okunur ve boşsa None, seçim
  değeri, onay true/false), komutun satırları panelin tablosunu, iletisi altını doldurur. Komut yürütücüsü olduğu gibi
  kullanılır (süreç, aracı, tek geri alma adımı, Durdur, günlük): **Python API'sine hiçbir şey eklenmedi.**
- **Panel Python'suz açılır:** şerit düğmesi her zaman açar; ortam hazır değilse panelin düğmeleri kapalıdır ve
  nedeni panelde yazar (şeridin sözleriyle). Yazılan değerler yazıldığı gibi tutulur (yarım `12,` kaybolmaz), basınca
  okunur; sayı olmayan ya da sınır dışı değer adıyla söylenir, hiçbir şey çalışmaz. Aynı panel yeniden açılınca
  değerleri durur. Geç gelen yanıt yalnız başladığı açık panele yazılır (kapandıysa hiçbir yere).
- **Yer:** dock'un "Eklenti" sekmesi, her programda. İlk açılış Katmanlar'ın yığınına: `show` yeni paneli kenarın son
  yığınına (Öznitelikler'inkine) koyuyordu, form ve tablo yüksekliğin üçte birine sıkışıyordu (gerçek pencerede
  ölçüldü); metin bileşeni form sütununda birkaç sözcükte bir kırılıyordu, tam genişliğe alındı. Boş sonuçta sütun
  olmadığından boş başlık şeridi yerine "Satır yok." yazar.
- **Kapı 1** (`ui_panel`, JSON işaretçili): biçim/sürüm, başlık, 1–100 bileşen, bilinmeyen tür ve alan, kimlik kuralı
  (küçük harf, rakam, `_`, `-`; ctx.params anahtarı) ve tekliği, zorunlu ad, sayı alanının sınırları, seçimin
  seçenekleri ve başlangıç değeri, **düğmenin komutu manifestte olmalı**, düğmenin tablosu panelde olmalı.
- **MEBİS:** taslağın düğmesi olmayan bir `search` eylemine bağlıydı; MEBİS'e gerçek `search` komutu eklendi
  (`mezar_yeri`'nde arar, `ctx.params["query"]`), panel v1'e geçti (özet yeniden hesaplandı).
- **Gerçek pencere (`14-panel`, Vitrin 1.0.7):** Yapı ara açıldı; Ara "5 yapı bulundu" ve 5 satırlık tablo (no, alan
  m²); Y-9 ve "Yalnız numarasız" ile "0 yapı bulundu".
- **Satır bir nesneyse:** komutun bir satırında `uid` varsa satıra tıklamak nesneyi seçip yakınlaştırır (denetimin
  Göster'iyle ortak `ext_show_object`); `uid` sütun olarak gösterilmez. Vitrin 1.0.8'in Yapı ara satırları yapının
  kimliğini taşır. İskelette `panel` katkısı: özniteliklerinde arama yapan `nesne-ara` komutu, onu çalıştıran panel ve
  şerit düğmesi (yerleşik `search` ikonu); bütün birleşimler doğrulamadan uyarısız geçer.
- **Katman seçici** (`layerPicker`: `id`, `label`, isteğe bağlı `geometry`): açık çizimin katmanlarını grup yollarıyla
  ("Kadastro › Parsel sınırı") listeler, değeri katmanın kimliği, seçilmezse None. Tür süzgecine **türsüz katmanlar da
  girer**: türsüz katman her türü tutabilir; CBS şablonunun katmanları türsüz olduğundan "alan" süzgeci orada neredeyse
  bütün katmanları gösterir (gerçek pencerede görüldü, bilinçli). Vitrin 1.0.9'da Yapı ara'nın katmanı (boşsa `yapi`):
  Parsel sınırı seçilince 4 parsel geldi (`15-katman-secici`). İskeletin panelinde de var (boşsa bütün katmanlar).
- **Bulgu (düzeltildi): şerit öğelerinin `when` koşulu hiç değerlendirilmiyordu.** `documentOpen`, `selection`,
  `layerSchema`, `capabilityGranted` manifestte doğrulanıp öğede saklanıyor ama masaüstü onlara bakmıyordu: düğme
  koşuldan bağımsız hep basılabiliyordu. Artık pencere çizilirken denetlenir (`item_when_why`): tutmayan koşulda düğme
  kapalı ve ipucu nedenini söyler ("“mezar_parsel” katmanı projede yok: önce eklentinin katmanlarını kurun"); komut
  aramasından basılınca da aynı neden yazılır. `layerSchema` katmanın kimliğiyle karşılaştırılır (§23: kimlik şema
  adıdır). Sınandı: MEBİS'in Tekli mezar aracı katmanı yokken kapalı, `mezar_parsel` kimlikli katman eklenince açık.
- **MEBİS (düzeltildi):** denetimin düzeltmesi `cad.entities.set`'e `operation: "attributes"` vermiyordu (düzeltme
  çalışmazdı); doluluk aracı durdurulunca sözlük yerine `ctx.cancelled()` döndürüyordu (`read_features` Durdur'u zaten
  `Stopped`la söyler). MEBİS'in `tr.ornek.kent-temel` bağımlılığı fikstürlerde yok: kurulunca bağımlılık bekler.
- **Referans eklentinin gerçek pencere kabulü (MEBİS, kurum kataloğundan; `kek pack` + `kek index`, bağımlılığı
  çıkarılmış kopya):** Eklentiler → Bulunabilir → Gözden geçir ve kur → Ortamı hazırla (Python 3.12, 6 paket).
  Mezarlık BS sekmesinde Tekli mezar katmanlar kurulmadan kapalı (`when`); "MEBİS katmanlarını kur" → "Şema kuruldu:
  1 grup, 1 değer listesi, 4 katman, 2 form, 1 ilişki"; Tekli mezar 1,10 × 2,20 m mezarı yerleştirdi (nesne 9 → 10);
  Ada bazlı doluluk boş ada katmanını reddetti, Parsel sınırıyla `BELIRSIZ 1 0`; denetim "Bulgu yok (1,4 sn)"; Mezar
  sorgulama "BOS" ile 1 mezar, satırına tıklanınca mezar seçilip yakınlaştı (Vitrin çizimi üzerinde). İki bulgu düzeltildi:
  - **Arama hiçbir mezarı bulmuyordu:** öznitelikleri kayıttan okuyordu (`record.attrs` yok; nesnenin öznitelikleri
    `record.entity.attrs`'ta), "0 mezar yeri bulundu" diyordu. Artık katman yoksa "önce MEBİS katmanlarını kur" der;
    satırları `uid`, okunur başlık (Mezar no, Ada no, Durum) ve durumun adını taşır ("Boş": "bos" da "boş" da bulur).
    MEBİS'i kendi klasöründen gerçek eklenti süreciyle koşan ilk Python testi (`ReferenceExtensionTest`) eski kodda
    kırmızı.
  - **İşlemler aracının şerit düğmesi kimliğini yazıyordu** ("occupancy-by-island"): manifest yalnız tanım dosyasını
    adlandırır; ad artık tanımın `label`'ından ("Ada bazlı doluluk"), komut aramasında da.
- **Alan ve nesne seçici** (Vitrin 1.0.10, Değer say; `16-secici`):
  - `fieldPicker` (`id`, `label`, `layer`, isteğe bağlı `kinds`): `layer` bu paneldeki bir `layerPicker`'ın kimliğidir;
    liste orada seçilen katmanın alanlarıdır (takma adıyla, "Takma ad (ad)"), `kinds` katman şemasının alan türleriyle
    süzer (`["integer", "decimal"]`). Değeri alanın adı, seçilmezse None. Katman seçilmeden yalnız "—".
  - `featurePicker` (`id`, `label`, isteğe bağlı `layer`, `geometry`, `multiple`): **seçimi kullanıcı verir:** "Seçimi
    al" çizimdeki seçimden o katmanın (bağlı katman seçicide seçilen) ve türün nesnelerini alır; değeri kalıcı
    kimliklerin listesi (`multiple: false`: ilk kimlik ya da None). Haritada tıklatan bir seçim kipi yerine bu yol
    seçildi: KentOS'un seçimi zaten bütün araçların girdisi (pencere, çit, süzgeç), panel ikinci bir seçim aracı
    kurmaz. Uygun nesne yoksa nedenini adıyla söyler ("seçili 1 nesneden hiçbiri “Parsel sınırı” katmanından değil")
    ve önceki seçimi tutar; düğmeye basılınca o arada silinen nesneler verilmez.
  - **Bağlı katman değişince** alan seçimi ve o katmandan alınan nesneler temizlenir; aynı katmanı yeniden seçmek bir
    şey silmez. Kapı 1: `layer` paneldeki bir katman seçiciyi göstermeli (alan seçicide zorunlu), `kinds` bilinen
    türler, `multiple` true/false.
  - **Gerçek pencere:** Parsel sınırı + ada → Say "katmanın 4 nesnesinde 2 farklı “ada” değeri" (101 → 3, 102 → 1);
    pencereyle iki parsel + Seçimi al → "2 nesne", Say → 101 → 2; Yapı seçilince Alan "—", Nesneler "Seçilmedi".
    **Bulgu (düzeltildi):** sayı ile iki düğme aynı satırdayken dar dock'ta sayı görünmüyor, Temizle kenardan
    taşıyordu; satır ikiye bölündü (üstte "2 nesne · Temizle", altta Seçimi al).
  - İskeletin panelinde katman seçiciye bağlı alan seçici var (`nesne-ara` istenirse yalnız o alanda arar; gerçek
    eklenti süreciyle sınandı: alanla 1, alansız 2 nesne).
- **İlerleme** (Vitrin 1.0.11, Yapı ara › Yapıları numarala; `17-ilerleme`): ayrı bir bileşen **gerekmez**: düğmenin
  komutu `ctx.progress(done, total, text)` bildirdikçe panel formun altında ilerleme çubuğunu, sözlerini ("%40 · 3.
  yapı numaralanıyor"; toplamsız: kodun sözleri ya da adım sayısı; ilk bildirime dek kayan çubuk, "Çalışıyor…") ve
  Durdur'u gösterir. Bir bileşenin bildirdiği yer, eklentinin unutabileceği bir şey olurdu; çalışan her komut
  ilerlemesini aynı yerde söyler.
  - **Bulgu (düzeltildi): komutun ilerlemesi hiç çizilmiyordu.** `ctx.progress` sürece ulaşıyor, çalışmada
    saklanıyor ama hiçbir yerde gösterilmiyordu: Vitrin'in Yapıları numarala'sı "ilerleme çubuğunu göstermek için
    bekler" diyordu, çubuk yoktu. Artık çalışma bildirimi (Durdur'lu, kalıcı) çubuk ve sözleri taşır; ortak bileşende
    (`kentos_ui::widget::toast`: `Toast::progress`, yerinde güncelleme `Toasts::edit`: sırası, sayısı, süresi değişmez)
    düzeltildi, bütün kalıcı iş bildirimleri kullanabilir.
  - İleti veren (satır vermeyen) komut panelin tablosunu boşaltmaz: Yapıları numarala sonrası arama sonuçları durur,
    Ara yeni numaraları listeler (Y-01 … Y-05).
  - **Durdur kullanıcının seçimidir, hata değil:** panelde kırmızı yazılıyordu (bildirim bilgi); artık panelin sözleri
    üç işaretli (bitti, durduruldu, çalışmadı) ve durdurma bilgi işaretiyle yazılır. Gerçek pencerede %40'ta çubuk,
    panelin Durdur'uyla "Durduruldu; çizime yazdıkları geri alındı" ve bildirimde "Çizimde bir şey değişmedi".
- **Yer** (`entrypoints.panels[].side`; Vitrin 1.0.12, `18-kenar`): `right` Katmanlar'ın yığınında (F4'le gizlenen
  yan paneller açılır), `left` haritanın solunda (Katmanlar görünür kalır: panelle katman seçerken listeye bakılır),
  `bottom` haritanın altında. Tek Eklenti sekmesi var: KentOS'un yerleştirdiği yerdeyken başka kenar isteyen panel
  açılırsa sekme oraya taşınır; **kullanıcının taşıdığı sekme yerinde kalır** (KentOS son yerleştirdiği kenarı
  bilir, `panel_placed`). Gerçek pencerede Değer say solda, ardından Yapı ara Katmanlar'ın yığınında.
- **Kapatılan eklenti iz bırakmaz** (referans eklentinin kabul maddesi; gerçek pencerede MEBİS, paneli açıkken
  Eklentiler › Devre dışı bırak): Mezarlık BS sekmesi, panel ve Eklenti dock sekmesi gider, komut araması "mezar" ve
  "doluluk" için "menülerde yok" der; yeniden etkinleştirilince sekme döner, "doluluk" Ada bazlı doluluk'u bulur. İki
  bulgu düzeltildi: (1) **açık panel kalıyordu**: düğmeleri basılabilir, eklentisi kapalıydı; artık eklenti durunca
  paneli, düğmesinin bıraktığı çağrı ve dock sekmesi kapanır (`extensions_changed`). (2) **Denetim bulgusunun Düzelt'i
  kapalı eklentinin düzeltmesini yazıyordu**; bulgular okunmak için durur, düzeltme "… kapalı: düzeltmesi uygulanmaz;
  Eklentiler'den etkinleştirin" der.
- **Aynı kimlik, aynı sözler** (referans eklentinin kabul maddesi: "şerit, Eklentiler, İşlemler, Denetim, komut arama
  ve MCP aynı kimliğe gider"): kimlikler hep aynıydı (`ext.tr.ornek.mebis.occupancy-by-island`; hepsi barındırıcının
  `Registered.id`'si), ama manifestin yalnız dosyasını adlandırdığı katkıların başlığı kimlikti: `kek dump` ve MCP'nin
  `kentos://extensions/installed`'ı İşlemler aracını "occupancy-by-island", formları "mezar-yeri", adsız sağ tık
  öğesini boş gösteriyordu. Barındırıcı artık kayıtta okur (`lease::file_titles`: tanımın `label`'ı, form paketinin
  `name`'i; okunmayan dosyada kimlik kalır), adsız menü öğesi gösterdiği komutun ya da aracın adını alır: "Ada bazlı
  doluluk", "Mezar yeri", "MEBİS katmanlarını kur". Şerit aynı başlığı kullanır (tanımı ikinci kez okumaz).
- **İkinci İşlemler aracı** (referans eklentinin "iki İşlemler aracı"): MEBİS'in Boş mezar yerleri'si durumu BOS olan
  mezarları yeni bir katmana (mezar ve ada numarasıyla; yeşil) yazar ve sayısını verir: ilk araç tablo, bu araç katman
  ve sayı üretir. Gerçek eklenti süreciyle sınandı (`ReferenceExtensionTest`: 3 mezardan 2 boş); gerçek pencerede
  bir Tekli mezar yerleştirildikten sonra "1 boş mezar yeri bulundu", nesne 10 → 11, çıktı katmanı Boş mezar
  yerleri; şeritte Ada bazlı doluluk'un altında, adı tanımından.
- **Ajan referans biçimini üretti** (referans eklentinin son maddesi; Asistan, DeepSeek test anahtarı yalnız sürecin
  ortamında, sentetik istek, Vitrin çizimi): kendi şerit sekmesi, `schema.json`'la ada ve mezar yeri katmanları, katman
  kuran komut, harita aracı, arama paneli, iki İşlemler aracı (ada doluluk tablosu; boşları yeni katmana yazan) ve
  denetim kuralı istendi; ajan iki-üç dakikada yazıp doğruladı, Gözden geçir ve kur penceresine altı yetenekle açtı.
  Kurum kataloğundan kuruldu, ortam hazırlandı ("0 paket"), Katmanları kur, Mezar çiz (nesne 9 → 10), denetim "1 uyarı:
  Durumu BOŞ ama mezar numarası boş", Ada doluluk tablosu ("1 ada, 1 mezar"), Boş mezarlar "1 boş mezar yeri yazıldı".
  Dört bulgu:
  1. **Ajanın doğrulaması kurulumdan gevşekti.** İlk turun paketi `extension.verify`'dan 0 sorunla geçti, inceleme
     reddetti: yeni nesne veren çıktının `sinkParameter`'ı yoktu. Paylaşılan denetim araç tanımında yalnız biçime
     bakıyor; tam denetim (`kentos-processing`'in `from_descriptor`'ı) yalnız masaüstü kurulumundaydı. Artık
     `extension.verify` (klasör ve paket) ve atölyenin incelemeye açması aynı denetimi aynı kod ve sözlerle yapar (mod-gis;
     MCP ve Asistan); komut satırındaki `kek verify` mimari gereği (`kentos-processing`'i taşımaz) yalnız biçime bakar
     ve bunu söyler. İkinci turda ajan doğru yazdı.
  2. **Çıktı katmanını imzasına yazan işlev çöktü** ("TypeError: … missing 1 required positional argument: 'hedef'"):
     KentOS katman parametresini işleve hiç geçirmiyordu. Artık katman parametreleri ayrı gelir (`layers`:
     `{"layerId"}` ya da `{"newName"}`) ve yalnız o adı alan işleve verilir: TAKS gibi almayan bozulmaz.
  3. **Araçlar etkin katmanla (Taslak) açıldı:** `features` parametresinde varsayılan katman yoktu. Rehber
     `default: {scope: "layer", layerId: …}`'ı söyler; şemalı iskeletin aracı şema katmanında açılır.
  4. **Panelin araması katmanı bulamadı:** şema katmanını görünen adıyla aradı (`name == "mezar_yeri"`); şemanın `name`'i
     katmanın kimliği, `title`'ı adıdır. Rehber bunu, `ctx.doc.layer(...)`'ı ve özniteliklerin `kayit.entity.attrs`'ta
     olduğunu söyler.

  **Üçüncü tur, rehber düzeltmeleriyle:** ajanın paketi (Mezarlık Hizmet) kutudan çıktığı gibi çalıştı: araçlar
  varsayılan katmanlarıyla açıldı (`layerId: mezar_yeri`), Boş mezarlar "1 boş mezar yeri yazıldı" (10 → 11), denetim
  1 uyarı, panelin araması katmanı kimliğiyle okuyup "1 mezar yeri bulundu" dedi, satıra tıklayınca mezar seçilip
  yakınlaştı; Ada doluluk ada çizilmediği için haklı olarak "uygun nesne yok" dedi.
- **Değer değişince** (`onChange`; Vitrin 1.0.13, Değer say; `19-degisince`): `select`, `layerPicker`, `fieldPicker`,
  `featurePicker` ve `checkbox` bir komut adlandırabilir; kullanıcı değeri değiştirince (nesne seçicide "Seçimi al" ya
  da Temizle) komut panelin değerleriyle çalışır, satırları panelin ilk tablosuna gelir. Metin ve sayı alanında yok:
  her tuşta bir süreç çağrısı olurdu, onlar için düğme var. Komut çalışırken gelen değişiklik bekler, bitince **bir
  kez** o anki değerlerle yeniden çalışır (art arda üç seçim üç çalıştırma değil). Kapı 1: `onChange` eklentinin bir
  komutu olmalı. Vitrin'de alan seçilince ya da nesneler alınınca sayım kendiliğinden yenilenir; Say düğmesi durur.
- **Koyu tema, büyük yazı, dar pencere, klavye** (ADIM 22 kabulü; 1100×650, 16 px yazı, koyu tema, gerçek fare ve
  klavye; sahibin ayarlarının yalıtılmış bir kopyası): "Yapı ara" Alt+Q ile komut aramasından açıldı. İki bulgu:
  1. **Dar yerde ortak `Form` etiket sütununu koruyordu:** alanlar yan panelin genişliğinin üçte birine iniyor, onay
     kutusunun yazısı ("Yalnız numarasız yapılar") sağ kenardan kesiliyordu. Düzeltme ortak bileşende
     (`kentos_ui::widget::form`, `Pair`): etiketin yanında alana en az 120 px (yazıyla ölçekli) kalmıyorsa etiket alanın
     üstüne geçer ve alan bütün genişliği alır, adsız satır girintisiz durur. Geniş yerde görünüm aynı: 1440×900,
     13 px'te Yapı ara değişmedi (aralık eskisi gibi sabit 12 px). Ekransız düzen testi iki genişliği ölçer.
  2. **Enter:** panelin metin ve sayı alanında Enter birincil düğmeye (yoksa ilk düğmeye) basar; Yapı no'da Enter
     aramayı çalıştırdı (5 satır).
  Kalan: kısa pencerede Eklenti yığını yüksekliğin küçük bir payını alıyor (Öznitelikler altında), panel kaydırılarak
  kullanılıyor (aşağıda *Yığın payı*'yla azaldı).
  **Düzeltme (15 Kasım, gerçek pencere):** burada "alanlar arası Tab yok" yazıyordu, yanlıştı. Panelin alanları ADR
  0400'ün ortak odak geçişini kullanır: Yapı no'da Tab → Katman → onay kutusu (Boşluk işaretler), Shift+Tab geri,
  Enter metin alanında Ara'ya basar ("0 yapı bulundu"). Düğmeler odak almaz (Enter onların yerine); son alandan sonra
  Tab pencerenin sıradaki kutusuna (Öznitelikler'in seçimi) geçer: dock bir odak kapsamı değildir, iletişim kutusu
  gibi kendi içinde döndürmez. Esc alandan çıkar, yazılan kalır ("Y-0"); sonraki tuş komut satırına gider (hiçbir kutu
  odakta değilken yazılan komut satırınındır, uygulamanın genel davranışı).
- **Ağaç** (`tree`: `id`, `label`, `group`; Vitrin 1.0.14, Değer say; `20-agac`): düğmenin (ya da `onChange`'in) komutu
  düz satır verir, KentOS onları `group`'un 1–3 sütununa göre iç içe gruplara dizer (KentOS'un ağaç görünümü): her grup
  ilk satırının sırasıyla, yanında sayısı, kullanıcı kapatmadıkça açık; yaprağın adı satırın gruplanmayan ilk sütunu,
  diğer sütunları yanında, `uid`'li yaprak tıklanınca nesneyi gösterir. **Python API'sine bir şey eklenmedi:** ağaç
  satırları kendisi gruplar, komut bir tabloya yazar gibi yazar. Değersiz satır "—" grubuna düşer. Kapı 1: `group` 1–3
  farklı sütun adı; düğmenin `table`'ı bir tabloyu ya da ağacı gösterir. Vitrin'in Değer say'ı artık nesne başına satır
  verir; ağaç dağılımı başlık sayılarıyla gösterir, nesneler altında.
- **Tarih alanı** (`dateField`: `id`, `label`, `value`, `min`, `max`, `onChange`; Vitrin 1.0.17, Parsel kartı'nın Kart
  tarihi; `24-tarih`): kullanıcı GG.AA.YYYY ya da YYYY-AA-GG yazar (tek haneli gün ve ay da) ya da KentOS'un takvimini
  (formların tarih seçicisi) açar; değeri `ctx.params`'ta ISO tarih metni (`2026-11-20`), boşsa None. Yazılan metin
  basınca okunur: gün olmayan ("30.02.2026") ve sınır dışı tarih adıyla söylenir, hiçbir şey çalışmaz. Takvimden seçilen
  gün bir değişikliktir (`onChange`); yazmak değildir (her tuşta süreç çağrısı olurdu), ama **Enter yazılan tarihi
  uygular** `onChange`'i varsa: Parsel kartı'nın düğmesi yok, kartlar yalnız takvimden yenilenebiliyordu (gerçek
  pencere); `onChange`'siz alanda Enter eskisi gibi birincil düğmeye basar. Kapı 1: `value`/`min`/`max` takvimde var
  olan gün, YYYY-AA-GG; min ≤ max. Gerçek pencere: takvimden 20 Kasım seçilince ve "20.11.2026" yazılıp Enter'a
  basılınca kartlar "tarih 20.11.2026" ile yenilendi.
- **Çoklu seçim** (`multiSelect`: `id`, `label`, `options`, `value`: başta seçili değerler, `onChange`; Vitrin 1.0.18,
  Parsel kartı'nın Bilgiler'i; `25-coklu`): her seçenek KentOS'un seçilebilir bir çipi (ortak bileşene
  `chips::toggles` olarak eklendi; kütüphanenin kategori çipleri aynı çipi kullanır), basınca seçilir ya da bırakılır
  (`onChange`); değeri seçilenlerin listesi, seçeneklerin sırasıyla, hiçbiri seçilmezse boş liste. Kapı 1: seçenekler
  seçimdeki gibi, `value` seçeneklerin değerlerinden her biri bir kez. Gerçek pencere: Alan ve Yapı sayısı bırakılınca
  kartlarda taban alanı, TAKS ve tarih kaldı; üç çalıştırma tek bildirimde "×3" diye birleşti.
- **Gösterge kartları** (`stats`: `id`, `label`, `title`, `value`, `unit`, `note`: sütun adları; Vitrin 1.0.19, Yapı
  ara'nın Özetle düğmesi; `26-ozet`): komutun her satırı KentOS'un gösterge kartı (`StatCard`): `title` sütunu adı
  (yoksa ilk sütun), `value` sütunu sayısı (yoksa ikinci), `unit` birimi, `note` altındaki notu; satırın vermediği birim
  ya da not "—" diye yazılmaz. Kartlar yan yana sığdıkça dizilir; `uid`'li kart tıklanınca nesneyi gösterir. Kapı 1:
  sütun adları boş olmayan metin; düğmenin `table`'ı göstergeyi de gösterebilir. Gerçek pencere: Yapı 5 adet, Toplam
  taban alanı 1119 m², Numarasız 5 adet, En büyük yapı 400 m² "numarasız"; kartına tıklanınca yapı seçilip yakınlaştı.
  **Bulgu (düzeltildi):** 132 px'lik kartlar 300 px'lik yan panelde birer satıra düşüyordu; 124 px'te ikişer.
- **Birden çok açık panel** (15 Kasım; Vitrin'in iki paneli, `21-paneller`): tek Eklenti dock sekmesi kalır, açık her
  panel onun üstünde bir sekmedir (KentOS'un belge sekmeleri: `kentos_ui::widget::tabs`; sürüklenerek sıralanır, ×'i
  ya da orta tık kapatır). Her eklenti panelini ayrı dock sekmesi yapmak dock'un sabit panel listesini (yerleşimin
  kaydı) dinamik kılmayı gerektirirdi; alt sekmeler tek yerde kalır ve dar dock'ta kayar.
  - Her panelin değerleri, tabloları ve ağacının kapalı grupları kendinde durur; şeritten yeniden açılan panel
    gösterilir, ikinci kez açılmaz. Yeni panel sona eklenir ve gösterilir; gösterilen kapanınca sağındaki, yoksa
    solundaki gösterilir; tek panel kalınca sekme şeridi gider.
  - **Çalışma başladığı panele yanıt verir, gösterilmese de:** komutu sürerken panelin sekmesinde nokta durur, başka
    panel gösterilirken yanıt gelince nokta panel gösterilene dek kalır (`unseen`). Değer say gösterilirken Yapıları
    numarala sürdü: nokta, bitince de nokta, Yapı ara açılınca "5 yapı numaralandı" ve nokta gitti.
  - **Yer:** sekme, paneli **en son açılanın** `side`'ına gider (KentOS yerleştirdiyse); sekmeler arasında geçmek
    dock'u taşımaz: alt sekmeye tıklayınca bütün dock'un pencerenin öbür yanına atlaması kullanıcıyı yerinden ederdi.
    Değer say solda açıldı, Yapı ara açılınca ikisi sağda.
  - **Çağrılar sıralı liste** (`panel_calls`): tek çağrı yuvası iki panelin aynı anda basılan düğmelerinde (biri
    ortamı aranırken) birinin çağrısını ezerdi: o komut panel değerleri olmadan koşar, panelin düğmesi "…"da kalırdı.
    Çağrı değerlerini ve panelini taşıdığından, aynı komutu çağıran iki panelin çağrısını hangi başlatma alırsa alsın
    yanıt doğru panele gider.
  - **Bulgu (düzeltildi): ikinci başlatma çağrıyı askıda bırakıyordu.** Aynı eklentinin iki paneli art arda basınca
    ikinci başlatma eklentinin çalışmasını görüp sessizce dönüyordu (çağrı listede, düğme basılmış kalıyordu);
    süreç başlatılamadığında da aynısı. Artık ikisi de paneline nedenini söyler ("“Yapıları numarala” sürüyor:
    bitmesini bekleyin ya da Durdur'a basın"), düğme yeniden basılabilir. Tek panelde de geçerli bir açıktı.
  - **Bulgu (düzeltildi): çalışan düğmenin adına "…" eklenince** dar dock'ta "Yapıları numarala…" iki satıra
    kırılıyor, form çalışma başlarken aşağı zıplıyordu. Düğmenin adı artık değişmez; formun altındaki ilerleme satırı
    düğmeyi adlandırır ("“Yapıları numarala” çalışıyor…", ilerleme gelince "Yapıları numarala · %40 · 3. yapı
    numaralanıyor"); değişikliğin komutunda düğme yoktur, satır eskisi gibi.
  - Eklenti kapatılınca yalnız onun panelleri gider; diğer eklentinin paneli ve dock sekmesi kalır, son panelle sekme
    de gider. Test: üç panel (Vitrin'in ikisi, MEBİS'in Mezar sorgulama'sı) ekransız.
- **Kart** (`card`: `id`, `label`, `title`, `tag`; Vitrin 1.0.15, Parsel kartı; `22-kart`): düğmenin (ya da
  `onChange`'in) komutunun her satırı bir kart: `title` sütunu başlığında (verilmezse ilk sütun), `tag` sütunu
  yanında, diğer sütunlar altında adlarıyla; sayı rakam yazısıyla sağa dayalı. KentOS'un bilgi kartıdır
  (`kentos_ui::widget::InfoCard` + `Pairs`, çizimdeki nesnenin üzerine gelince açılan kartla aynı), kart yüzeyinde
  (`style::container::bordered`); `uid`'li satırın kartı tıklanır, nesneyi seçip yakınlaştırır (tablonun satırı
  gibi), üzerine gelince kenarı belirginleşir. Tıklanan kartın stili gösterge kartından ortak stile taşındı
  (`style::button::card`). **Neden:** dar dock'ta çok sütunlu tablo okunmuyor; kart sütunları alt alta yazar.
  **Python API'sine bir şey eklenmedi:** kart, tablonun aldığı satırları alır. İlk 100 kart, gerisi sayılır. Kapı 1:
  `title`/`tag` boş olmayan sütun adları ve birbirinden farklı; düğmenin `table`'ı bir kartı da gösterebilir.
  - **Gerçek pencere:** iki parsel pencereyle seçilip Seçimi al'a basılınca 101/1 "TAKS aşıyor" (900 m², 2 yapı,
    450 m², 0,5) ve 101/2 "uygun" (244 m², 0,27); 101/2'nin kartına tıklanınca parsel seçilip yakınlaştı. Koyu tema,
    16 px yazı, 1100 × 650'de: dar panelde etiketler alanın üstünde, kart okunur; Eklenti sekmesi "Paneli yüzdür" ile
    ayrı pencereye alınınca kartlar orada da (planın "kalıcı pencere"si: panel tanımı değil, kullanıcının dock'u).
  - **Bulgu (düzeltildi):** düğmesi olmayan panelin boş sonuç alanı "Düğmeye basınca komutun satırları burada …"
    diyordu (Parsel kartı'nı nesne seçicinin `onChange`'i doldurur). Tablo, ağaç ve kartın bekleme sözü artık onu
    dolduran şeye göre: bir düğme ise "Düğmeye basınca", değilse "Değer değişince".
  - Dar pencerede sağ yığın kısa ve Katmanlar sekmesi şeritten taşmış: etkin katmanı seçmek için yığının ⋯ menüsünden
    Katmanlar açıldı (yukarıdaki kısa pencere bulgusunun parçası, ayrı iş).
- **Yığın payı:** panel açılınca onu gösteren yığın kenarının açık yığınlarının en az beşte üçünü alır (`PANEL_SHARE`;
  yalnız artırır: kullanıcının büyüttüğü yığın payını korur, sekmeler arasında geçmek dokunmaz). Ölçüm: 1440 × 900'de
  Eklenti yığını ~365 → ~435 px (Öznitelikler'in tutamağı y 508 → 579), 1100 × 650 / 16 px'te ~220 → ~260 px. Kısa
  pencerede uzun açıklamalı panel yine kaydırılır. Vitrin senaryolarının tutamak sürüklemesi yeni yere alındı; 14-panel
  ayrıca eskimişti (form sonradan büyüyünce Ara tıklaması onay kutusuna düşüyordu), güncel yerleşime göre düzeltildi.
- **Bulgu (düzeltildi, SDK): hiçbir şey bulmayan arama ileti olarak gidiyordu.** `ctx.report([])` satır yokken
  `{"kind": "message"}` gönderiyordu; panel bunu "satır vermeyen ileti" sayıp önceki aramanın satırlarını tutuyordu:
  Yapı ara'da Y-9 ile "0 yapı bulundu" yazarken eski 5 satır duruyordu (gerçek pencere). Artık `CommandResult.rows`
  None ise ileti, liste ise (boş da olsa) rapor: tablo boşalır ("Satır yok."), kart "Kart yok." der; Parsel kartı'nda
  Temizle kartları kaldırır. SDK masaüstüyle gelir (ortamın kitaplığı her başlatmada tazelenir): bütün eklentiler
  güncellemesiz düzelir. Test: MEBİS'in gerçek süreçteki araması "yok" için `("report", [])`.
- **Giriş sırasında yanıt** (PY-UI-04'ün panel payı; gerçek pencere): Yapıları numarala sürerken Yapı no'ya "Y-"
  yazıldı; yanıt gelince (ilerleme gitti, "5 yapı numaralandı" geldi) yazmak aynı alanda sürdü ("Y-0"): odak ve
  taslak korunur, çünkü yanıt yalnız formun altını değiştirir, yazılanlar panelin durumundadır.
- **Bulgu (düzeltildi): kaydırınca panellerin sekmeleri de kayıp gidiyordu.** Dock, Eklenti panelini kendi kaydırıcısına
  sarıyordu (bütün panellerin varsayılanı) ve panel de kendi içinde kaydırıyordu: tekerlek dış kaydırıcıyı çevirip
  sekme şeridini görünmez yapıyordu, sekmeye tıklanamıyordu. Artık panel kendi kaydırır (Asistan gibi), sekmeler
  üstte durur. Her panel kaydırıldığı yeri tutar: Yapı ara aşağı kaydırılıp Değer say'a geçilince o en üstten,
  Yapı ara'ya dönünce bırakıldığı yerden gösterilir (tek kaydırıcı, `scroll_to` ile geri verilir).
- **Kalan (ADIM 22):** panel köprüsü tamamlandı; bileşen envanteri ve tanım/oturum maddeleri (PY-UI §8.1, PY-UI-01…18)
  ayrı maddeler.

## §26. Kapı 3'ün kod çalıştırmayan payı: kuru bağlama (`ADIM 21`)

**15 Kasım.** Kod: `crates/native/extension-host/src/dry_run.rs` (`dry_run`, `DryRun`), `apps/cli/src/main.rs`
(`kek verify`), `crates/native/headless/src/agent_tools.rs` (`extension.verify`'ın `dryRun`'ı, `Host::dry_run`),
masaüstü `assistant/host.rs`, MCP `tools.rs`.

- **Karar: Kapı 3 ikiye ayrıldı.** Planın kuru koşusu (bağlama, şerit ve panelin kurulması, algoritmanın ve denetimin
  örnek girdiyle sonucu, kaldırmada sıfır kalıntı) iki ayrı şeydir: **kod çalıştırmayan pay** kurulumdan önce
  güvenlidir; **kodu çalıştıran pay** ise kullanıcının onayını (kurulum: "kurulmadan önce eklenti çalışmaz") ve
  eklentinin Python ortamını ister. Eklentinin süreci kullanıcının hesabıyla çalışır: aracı KentOS'a erişimi sınırlar,
  işletim sistemine erişimi sınırlamaz; incelenmemiş kodu kuruluma karar verilmeden çalıştırmak bu ilkeyi delerdi.
  Çalıştıran pay kurulmuş atölye eklentisi için ayrı bir dilimdir.
- **Kuru bağlama:** paket geçici klasördeki boş bir kayıt defterine, istediği her yetenek verilerek (Python katkıları
  da: etkinleştirme süreç başlatmaz, D6) kurulur ve etkinleştirilir; eklediği katkılar (kimlik, tür, ad), atlananlar
  nedenleriyle, kurulamadıysa sorunları, etkinleşmediyse nedeni toplanır; sonra verisiyle kaldırılır ve paket ya da
  veri klasörü, kayıt defteri girdisi, kirasıyla kalan katkı ya da hazırlık klasöründe dosya kalırsa **kalıntı** sayılır.
  Kullanıcının eklentilerine dokunulmaz; klasör koşudan sonra silinir. Masaüstünde kurum ilkesi ve KentOS'un İşlemler
  araç denetimiyle, MCP'de ve komut satırında ilkeyle.
- **Yüzeyler:** `kentos kek verify` doğrulanan klasörün paketini bağlar ("Kuru koşu: 18 katkı bağlandı (…). Kaldırınca
  kalıntı yok."; `--json`'da `dryRun`), ajanın `extension.verify`'ı aynısını `dryRun` olarak verir (masaüstü ve MCP
  host'u). Bağımlılığı olan pakette boş kayıt defteri bağımlılığı bulamaz: etkinleşmediği söylenir ve not düşülür
  ("Kuru koşunun kayıt defteri boş: bağımlı olduğu eklentiler (tr.ornek.kent-temel) orada kurulu değil"; MEBİS).
- **Sınamalar:** extension-host 2 (kaynaklı küçük paket temiz bağlanır, eksik bağımlılık söylenir ve kalıntı kalmaz,
  bozuk paket sorunlarıyla; Vitrin bütün katkılarıyla, hiçbiri atlanmadan), başsız ajan aracı (doğrulanan klasörün paketi
  host'a gider), Asistan (iskelet eklenti masaüstünün ilke ve araç denetimiyle temiz bağlanır, harita aracı kayıtlı).
- **Kalan (Kapı 3'ün çalıştıran payı):** kullanıcının kurduğu atölye eklentisinin denetim kurallarını ve komutlarını
  sentetik bir çizimin kopyasında çalıştırıp bulguları ve değişiklik kümesini ajana vermek.

## §25. Panel bileşenlerinin kapsamı: PY-UI §8.1'in envanteri (`ADIM 22`, bileşen envanteri, ilk dilim)

**15 Kasım.** Planın bileşen listesi (PY-UI §8.1) madde madde: panelde var mı, KentOS'ta bileşeni olup panele
bağlanmamış mı, yoksa hiç mi yok. Plan "birkaç basit bileşen bağlayıp tam UI API denmez" diyor; bu tablo neyin
bağlandığını ve neyin bağlanmadığını açıkça söyler. Yeni bir panel bileşeni eklenince bu bölüm güncellenir: Kapı 1'in
bildiği her tür (`kentos_extensions::ui_panel` `TYPES`) burada adıyla geçmezse test kırmızıdır
(`ui_panel::tests::every_component_type_is_in_the_coverage_report`).

**Durum:** *var*: `kentos.ui.panel` 1'de, Vitrin'de gösterilir · *KentOS'ta*: uygulamanın kendi bileşeni var, panele
bağlanmadı · *yok*: ikisi de yok · *host*: panel bileşeni değil, manifestin başka bir yüzeyi.

| Grup | Bileşen | Durum | Panelde / KentOS'ta | Not |
|---|---|---|---|---|
| Düzen | yığın | var | gövde tek sütun (ortak `Form`) | dar yerde ad alanın üstüne geçer |
| | satır, ızgara | var | bileşenin `row` anahtarı, ortak formun hücreleri (§55) | yer yoksa alt alta (PY-UI-13); aynı sayıda hücreli satırlar sütun gibi hizalanır |
| | sekme | var | bileşenin `tab` anahtarı, KentOS'un sekme şeridi (§43) | sekmesiz bileşen her sayfada; yanlış alan sayfasını açar |
| | grup, akordeon | var | `section` (katlanır başlık; sonraki bileşenleri bir sonraki bölüme dek toplar, §38) | katlıyken değerler komuta gider; yanlış bırakılan alan bölümünü açar |
| | kaydırma | var | panel kendi kaydırır, yerini tutar | sekmeler üstte kalır |
| | çizgi | var | `divider` (formun genişliğinde ince çizgi, §59) | |
| | bölücü | var | panelin `split`'i: form üstte, çıktılar altta, arada KentOS'un tutamağı (`Sash`), §69 | sürüklenen yükseklik panelde kalır |
| | adımlayıcı | var | KentOS'un `wizard`'ı, eklentinin penceresinde: `ctx.dialog(…, steps=True)`, alanların sekmeleri adımlar (§60) | dock'taki panelde sekme |
| Basit girdi | metin | var | `textField` | Enter birincil düğme |
| | sayı | var | `numberField` (birim, sınır; `step` ile − ve + düğmeleri, §59) | yazıldığı gibi tutulur, basınca okunur |
| | tarih | var | `dateField` (KentOS'un takvimi; GG.AA.YYYY ya da YYYY-AA-GG yazılır, değeri ISO tarih) | |
| | saat | var | `timeField` (KentOS'un saat seçicisi, `TimePicker`; 14:30, 14.30, 9:05 yazılır, değeri SS:DD) | 24 saat |
| | an | var | `dateTimeField` (takvim ve saat yan yana, `DatePicker::date_time`; değeri `2026-11-15T14:30`) | yerel saat, saat dilimi yok |
| | mantıksal | var | `checkbox` | `widget::switch` bağlanmadı |
| | seçim | var | `select` | |
| | çoklu seçim | var | `multiSelect` (KentOS'un seçilebilir çipleri, `chips::toggles`) | değeri seçilenlerin listesi |
| CBS girdisi | katman seçici | var | `layerPicker` (tür süzgeci) | |
| | nesne seçici | var | `featurePicker` (çizimdeki seçimden) | haritada tıklatan kip bilinçli olarak yok (§24) |
| | alan seçici | var | `fieldPicker` (bağlı katmanın alanları) | |
| | koordinat/harita seçimi | var | `pointPicker` (`x, y` yazılır ya da Haritadan seç ile haritada gösterilir, kenetlenir; değeri `[x, y]`, §57); çizgi ve alan `shapePicker` (haritada çizilir, değeri GeoJSON, §62); harita aracı `entrypoints.mapTools` (yakalama türleri) | pencerede (`ctx.dialog`) yok |
| | CRS/birim | var | `crsPicker` (KentOS'un sistemleri, değeri `EPSG:<kod>`, verilmezse çizimin sistemi; §45); birim `numberField`'ın `units`'i (KentOS'un sayı alanları gibi birimiyle yazılır: 0,03 km; kod alanın biriminde alır, §64) | |
| | ilişki başvurusu | var | `relationPicker` (projenin bir ilişkisinin üst katmanının kayıtlarından adıyla seçilir; değeri kaydın kalıcı kimliği, ilişkinin alt alanının tuttuğu; §67) | |
| Kaynak ve çıktı | dosya/eser seçici, ek | var | ek `attachments` (seçicinin tuttuğu kaydın fotoğrafları, belgeleri ve adresleri; Aç, KentOS'un Dosya ekle… ve Adres ekle…'si; §68); `filePicker` (okunacak dosya: yolu yazılır ya da bilgisayarın dosya penceresinden seçilir, §41; `save` ile yazılacak dosyanın yeri, §44) | `files.userSelected` ister, kaydetme yeri `files.userSelectedWrite` |
| | görsel önizleme | var | `image` (kodun verdiği PNG ya da JPEG, `ctx.panel.set(id, image=…)`; oranı korunur, §40, §115) | en çok 4096 piksel kenar, boyutu başlığından okunur |
| | çıktı hedefi | host | İşlemler aracının çıktısı (`sinkParameter`) | panel komutu çizime yazar |
| Veri görünüşü | tablo | var | `table` (500 satır) | `uid`'li satır nesneyi gösterir |
| | ağaç | var | `tree` (1–3 sütunla gruplar) | |
| | kart | var | `card` (KentOS bilgi kartı) | dar yerde tablonun yerine |
| | ilişki listesi | var | `relationList` (bir seçicinin tuttuğu kaydın ilişkili kayıtları, çizimden canlı, Python'suz; satıra tıklamak kaydı gösterir; §67) | ekleme, bağlama ve düzenleme kodda (`cad.records.create`) |
| | özet/ölçü | var | `stats` (KentOS'un gösterge kartı, `StatCard`) | satır başına bir kart, yan yana |
| | grafik | var | `chart` (KentOS'un grafik motoru, Grafik sekmesinin çizimi: çubuk, pasta, histogram, çizgi) | satırları düğmenin `table`'ından ya da `of` ile başka bir bileşenden |
| İş denetimi | düğme | var | `button` (birincil) | bir komut çalıştırır |
| | değer değişince | var | `onChange` (seçim, seçiciler, onay) | çalışırken gelen bir kez yeniden |
| | ilerleme/günlük | var | `ctx.progress` panelde çubuk; `ctx.log` eklentinin günlüğünde | |
| | iptal/yeniden dene | var | Durdur (çizime yazdıkları geri alınır); düğmeye yeniden basmak | |
| | doğrulama özeti | var | `required` (alan başına zorunlu), sayı ve sınır hatası: sözler alanın altında (ortak `Form`'un hatası), adının yanında yıldız; tanım Kapı 1'de | koşullu kurallar (`requiredWhen`, `visibleWhen`, `enabledWhen`, §33) ve hesaplı değerler (`calculate`, `defaultExpr`, §35) formun ifade diliyle |
| Host yüzeyi | dock/panel | host | `entrypoints.panels` (`side`), birden çok açık panel | |
| | kalıcı pencere | host | Eklenti sekmesi "Paneli yüzdür" | |
| | komut/şerit/menü, bağlam eylemi | host | `contributions.ribbonTabs`, sağ tık menüleri | |
| | iletişim kutusu | var | `ctx.dialog`, `ctx.confirm` (KentOS'un penceresi, bir panelin alanları; §29) | çalışma yanıtı bekler |

**Bileşen türleri** (Kapı 1, `type`): `text`, `textField`, `numberField`, `dateField`, `timeField`, `dateTimeField`,
`select`, `multiSelect`, `layerPicker`, `fieldPicker`, `featurePicker`, `checkbox`, `button`, `table`, `tree`, `card`,
`stats`, `chart`.

**JSON Schema** (15 Kasım; planın "kod üretimi" maddesinin panel payı): `kentos.ui.panel` 1'in şeması panelin Rust
tiplerinden üretilir (`ui_panel::json_schema`, `schemars`; `format` ve `schemaVersion` sabitleriyle) ve SDK'nın yanında
durur (`crates/shared/extensions/sdk/kentos-ui-panel.schema.json`, 15,8 KB); tipler değişip dosya güncellenmezse test
kırmızıdır (`sdk_pack::tests::the_panel_schema_is_the_panel_types`, `KENTOS_WRITE_SDK=1` yazar). Ajan ve komut satırı
ayrı bölüm olarak alır (`extension.sdk part=panelSchema`, `kentos kek sdk --part panelSchema`); pakette yalnız nasıl
alınacağı yazar. Kesin denetim Kapı 1'dir: düğmenin komutu, seçicinin katman seçicisi, kimliklerin tekliği şemayla
söylenemez. **Paket boyutu:** SDK paketi sıkıştırılmış 46,1 KB, Asistan'ın araç sonucu sınırı 48 KB (bekçi testi var);
en büyükleri örnek 16,9, rehber 11,3, imzalar 9,9 KB. Sonraki eklemelerde örnek ayrı bölüme alınmalı.

**Sınır: özel çizim ve düzenleyici bileşeni yok.** Eklentinin kodu bir arayüz nesnesi ya da tuval almaz (QGIS'in Qt
dersi, D7): haritaya çizim harita aracının yakalamasıyla (KentOS çizer), panelde çizim hiç yok. Kayıtlı özel bileşen
(tanım, değer kodlayıcı, platform çizicisi ve salt okunur yedek, PY-UI-15/16) ayrı bir maddedir; o gelene dek panel
yalnız bu türleri çizer, bilinmeyen tür kurulumda reddedilir.

15 Kasım'da bağlananlar: tarih (`dateField`), çoklu seçim (`multiSelect`), özet/ölçü (`stats`), saat (`timeField`), an
(`dateTimeField`) ve grafik (`chart`). Planın listesinde bağlanmamış kalanlar yukarıda "KentOS'ta" ve "yok" diye yazılı.

**Zorunlu alan** (`required`, 15 Kasım): metin, sayı, tarih, saat, an, çoklu seçim, katman/alan/nesne seçicisi
`required: true` alır. Boş bırakılan komut çalışmadan adıyla söylenir ("“Önek” boş bırakılamaz", seçicide "seçilmedi",
nesne seçicide "nesne alınmadı", çoklu seçimde "en az biri seçilmeli"); yanlış sayı ve sınır dışı değer gibi bu söz de
artık **alanın altında** (`Form::error`, değerlerin denetimi hangi alanın yanlış olduğunu da verir), alan değişince
kalkar; zorunlu alanın adının yanında yıldız (`Form::required`). Panelde söz ayrıca alttaki durum satırında da durur
(uzun panelde alan görünmeyebilir), pencerede yalnız alanın altında. Vitrin 1.0.23: Yapıları numarala'nın İlk
numarası zorunlu (`30-zorunlu`, gerçek pencere).

**Grafik** (`chart`, 15 Kasım): KentOS'un grafik motoru (`geometry-core` `ops::charts`, Grafik sekmesinin) çizer; ayrı
bir çizici yazılmadı. Satırlar bir İşlemler tablosu gibi okunur (`table_columns`: sütunun türü hücrelerinden), model
satırlar gelince bir kez kurulur, yalnız yerleşim panelin genişliğini izler; Grafik sekmesinin boyaması
(`charts::canvas::Still`, şekiller tek resim, yazılar arayüzün yazı tipiyle) paylaşıldı, panel grafiği seçim yapmaz.
Türler çubuk (varsayılan), pasta, histogram, çizgi; çubuk ve pasta `category` ister (sayım; `value` verilirse toplam,
`aggregate` başkası), histogram `value`, çizgi ikisini (Kapı 1). Satırlar bir düğmenin `table`'ından ya da `of` ile
başka bir bileşenden (tablo, ağaç, kart, gösterge) gelir: aynı satırlar iki biçimde. Satırlarda olmayan bir sütun adıyla
söylenir ("“Mahalle” sütunu komutun satırlarında yok"; motorun "seçilmedi" iletisi Grafik sekmesinin sözüydü). Önbellek
(boyama önbelleği kopyalanamaz) açık panellerin yanında, eklenti/panel/grafik kimliğiyle; panel kapanınca gider. Yükseklik
220 px. Vitrin 1.0.21: Değer say'ın ağacının altında "Dağılım" (101 boyu 3, 102 boyu 1; `28-grafik`).

## §27. Ajanın eklenti denetimi: `audit.run`, `audit.report`, `audit.apply_fix` (`ADIM 20`, AI/MCP `audit.run`)

**15 Kasım.** Kod: `crates/native/headless/src/agent_tools.rs` ve `agent_tools/extension_audits.rs` (`audit_outcome`,
`Host::audit_wait`), masaüstü `extensions/audit_agents.rs` (istekler: `audits`, `audit`, `audit_report`,
`audit_fixed`), `extensions/audits.rs` (`ext_audit_start`: şeridin ve ajanın ortak başlatması), `python/agents.rs`
(bağlantının istekleri), `assistant/mod.rs` (`Later`, `assistant_audit_ended`), `assistant/host.rs`, MCP `desktop.rs`.

- **Karar: ajanın çalıştırması şeridin çalıştırmasıdır.** Planın "AI/MCP `audit.run` aynı yol" maddesi: ajan ikinci
  bir denetim yolu açmaz. Kural şeritteki düğmesi gibi eklentinin sürecinde (havuz) arka planda, öndeki çizimde çalışır;
  alt panelin Eklenti denetimi sekmesi açılır ve kullanıcı izler. Aynı anda tek denetim: biri sürerken ikincisi "Bir
  denetim sürüyor (Eklenti Vitrini › Yapı numarası): bitince yeniden deneyin" ile geri çevrilir (şeritte bilgi notu,
  ajana `busy`).
- **Listede:** `audit.list_providers` yerleşik denetimlerin ardından kurulu, etkin ve `audit.provider` yeteneği
  verilmiş kuralları verir: `ext.<eklenti>.<kural>`, başlık, eklenti (ad, sürüm), program, Python'u şimdi çalışıyor mu,
  çalışmıyorsa şeridin söylediği neden. MCP'nin kendi çizimleri eklenti barındırmaz: kural listelenmez, `audit.run` ve
  `audit.report` "Eklenti denetimleri KentOS masaüstünde çalışır: Asistan'da ya da masaüstünün ajan bağlantısında" der.
- **Bekleme iki türlü.** Asistan çizimin bulunduğu arayüz iş parçacığındadır: orada beklemek pencereyi dondurur,
  denetimin bitişi de duyulamazdı. Çağrı denetim başlayınca bekletilir (`Later`), bitince raporla cevaplanır; tur
  bütün çağrıların cevabını zaten bekliyor (onay bekleyen çağrının yolu). Bağlantıdaki ajan (MCP) kendi iş
  parçacığındadır: `audit_report`'u 250 ms'de bir, en çok 130 sn (masaüstünün 120 sn sınırı ve pay) sorar. Bulgu
  vermeyen çalışma (kural düştü, durduruldu) çağrının hatasıdır, nedeniyle (`audit_failed: Denetim bulgu vermedi:
  TypeError: …`). Sohbette Durdur ajanın beklediği denetimi de durdurur.
- **Görev teslimi:** başlatma arka plan işini bir `Task` olarak verir; ajanın çağrısı görev döndüremez. Görev
  `Extensions::audit_job`'da bekler, çağrıdan sonra çalışma zamanına verilir (Asistan'da `assistant_run`, bağlantıda
  `Event::Agent`). Görev bırakılsaydı iş hiç başlamazdı: `Task::perform` iş parçacığını ancak çalıştırılınca açar.
- **Rapor** (`audit.report`; çalıştırmanın cevabı da bu): durum (`running`, `done`, `failed`, `stopped`, `none`),
  kural, eklenti (kimlik, ad, sürüm, paket özeti), okunan çizim revizyonu, sonradan değişen (§17'nin köken notu), süre,
  önem sayıları, bulgular 100'er (`offset` ile devamı, kalanı `more`): numarası (sekmedeki sırası, 1'den), önemi,
  iletisi, nesnesi, uygulanabilen düzeltmesi ya da neden uygulanamadığı (`fixRefused`). Numara 0'dan başlarken model
  kullanıcıya "İlk bulgu (0)" diyordu (gerçek pencere); sohbetin araç satırı da "eklenti bulgusu 1" diyor. Kullanıcının şeritten çalıştırdığı denetim de okunur: "bu
  bulguları düzelt" isteği önce rapordan başlar.
- **Bulgular veridir.** İleti ve düzeltme eklentinin sözüdür: araçların açıklaması, her rapor ve Asistan'ın kuralları
  bunları talimat olarak okumamayı söyler. Düzeltme ancak kullanıcının Düzelt'i şimdi uygulayabilecekse verilir (§17:
  `document.write`, eklenti etkin, denetlenen çizim önde).
- **Düzeltme sahibin koruyucusuyla.** `audit.apply_fix` `rule` + `finding` (numarası) alır; düzeltmenin komutu
  katalogda ve programda sunulmalı. Önce `mode: plan` (planId), sonra aynı planla `execute`; Asistan'da `audit.apply_fix`'in her
  execute'u kullanıcının Onayla'sını bekler. Yazılınca sekmede bulgu "✓" olur, eklentinin günlüğüne "ajan uyguladı"
  düşer (`audit_fixed`); çizim denetimden beri değişmediyse işaretlenmez.
- **Sınamalar:** başsız 2 (masaüstü yerine geçen host: listede kural; `audit.run` raporu bitene dek sorar; uygulanamayan
  düzeltme nedeniyle, başka kuralın raporu, plansız execute reddedilir; planlı düzeltme yazılır ve işaretlenir; düşen
  kural hatadır; sunucunun çizimi eklenti denetimi olmadığını söyler), masaüstü 1 (iskelet eklenti atölyeden kurulur;
  Asistan'ın `audit.run` çağrısı denetim bitince raporla cevaplanır, sekme açılır, rapor sürüm ve paket özeti taşır;
  düzeltme koruyucudan yazılır, öznitelik yazıldı, bulgu ✓, çizim notu; bağlantıdaki ajanın başlattığı denetim de
  çalışma zamanına gider).
- **Tanımı:** ajanlar komutu çalıştırmadan önce `catalog.describe` ile okuyor (Asistan'ın kuralı); eklenti kuralı
  katalogda olmadığı için bu çağrı başarısız oluyordu (gerçek pencere). `catalog.describe` artık eklenti kuralını
  tanır: başlığı, eklentisi, girdisi olmadığı, `audit.run` ile çalıştığı, Python'unun durumu.
- **Gerçek pencere (Asistan, DeepSeek deneme anahtarı yalnız sürecin ortamında, sentetik Vitrin çizimi):** "Vitrin
  eklentisinin Yapı numarası denetimini çalıştır" → Denetimler, Denetle · eklenti denetimi yapi-no (2,0 sn, çağrı
  denetim bitince cevaplandı), sekme açıldı ("5 uyarı (1,9 sn)", köken satırı), cevap "5 bulgu çıktı (5 uyarı) … 5'inde
  de düzeltme var"; 662 jeton, 12,4 sn. "…sonra ilk bulgunun düzeltmesini uygula" → plan, onay kartı ("Bulguları onar ·
  eklenti bulgusu 1 · yazma — Onay gerekiyor"), Onayla → yapıya Y-01 yazıldı, sekmede "Y-01 yaz ✓" ve çizim notu, geri
  alma etkin; 1276 jeton, 26 sn.
- **Ajanın eklenti aracı arayüz iş parçacığında hesaplanmaz (15 Kasım).** Önceki sınır: ajanın `processing.run`'ı bir
  eklenti aracını (`ext.…`) masaüstünde rpc'nin `call`'u üzerinden eşzamanlı çalıştırıyordu (`extensions/tools.rs`
  `Bridge::run` havuzu bekler; Asistan'da ve bağlantıda bu arayüz iş parçacığı): Python aracı sürerken pencere aracın süre
  sınırı kadar (varsayılan 600 sn) bekliyordu; koruyucunun **plan** adımı da aracı hesapladığı için (`command.rs`
  `run_on`; yürütmede plan kimliği yazılan değişiklik kümesinden hesaplanır, `core/call.rs`) plan da bekletiyordu.
  Çözüm dört parça:
  1. **İşler planı da taşır** (`core/jobs.rs` `Job::start`): plan okuma kopyasında hesaplanır, yazılacak bir şey bırakmaz,
     cevabı plandır (`protocol::a_plan_as_a_job_writes_nothing_and_its_execution_matches_it`).
  2. **Koruyucu uzun komutları iş olarak gönderir** (`agent_tools/long_calls.rs`): `cad.processing.run|batch` ve aracı
     `ext.` ile başlıyorsa plan da yürütme de `start` → `job`. Plan kimliği iş bitince, çağrının yapıldığı revizyonla
     hatırlanır (`finish`); yürütme `expected`'ı (plan + revizyon) taşır, çizim bu arada değiştiyse plan geçersizdir.
  3. **Bekleyen, çağrının sahibidir.** MCP kendi iş parçacığında yoklar (`Host::job_wait` öntanımı: 250 ms'de bir
     `job`, en çok `JOB_WAIT` = 610 sn; istekleri tutamacın çizimini taşır). Asistan arayüz iş parçacığında bekleyemez:
     `AppHost::job_wait` işi `Pending::Job` olarak saklar ve "çalışıyor" der, `Event::JobLook` 250 ms'de bir sorar, iş
     bitince çağrı işin cevabıyla cevaplanır (`assistant_job_look` → `finish_long`); masaüstü meşgulse (konsol kodu, çizim
     açılışı) yeniden sorar. Durdur ve sahipsiz kalan çağrı işi durdurur (`cancel`; yazılmış olan kalır, Geri al alır).
  4. **İş başladığı çizimde sorulur.** Masaüstünün işleri (`python.jobs`) bir çizimin değil uygulamanındır ve işin
     sonucu `job` hangi çizimde sorulursa oraya yazılır; revizyon ve kuşak sayaçları her çizimde ayrı saydığı için başka
     bir çizimde aynı tabana denk gelebilir. Asistan işi başlattığı çizimin oturum numarasını saklar ve `job`/`cancel`'ı
     onunla sorar: kullanıcı bu arada başka sekmeye geçse de sonuç kendi çizimine yazılır, çizim kapandıysa
     `document_closed` çağrının hatasıdır.
  Bağlantının günlük satırı da yazmaya bağlandı: `start` artık "yazdı" sayılmaz, `job` çizimin revizyonu değiştiyse
  "Ajan: <komut>" der (komutu işin durumundan okur). Ölçüm: Asistan sınamasında İmar'ın TAKS planı iş olarak gitti,
  eklenti köprüsüne işin iş parçacığında ulaştı (sınamada KentOS Python kitaplığı yok: `tool_failed` "“Parsel TAKS
  hesabı” çalıştırılamadı"), çağrı bir yoklamada (251 ms) işin cevabıyla cevaplandı, çizimin revizyonu değişmedi
  (`assistant::tests::an_extensions_python_tool_runs_as_a_job_and_its_call_waits_for_it`); bağlantıda plan işi hiçbir
  şey yazmadı ve söylemedi, yürütme işi yazınca satır bir kez düştü
  (`python::tests::an_agents_job_is_said_in_the_log_when_it_writes`). MCP tarafı uçtan uca: gerçek `kentos-mcp`
  masaüstüne bağlanıp İmar'ın TAKS planını istedi, iş bağlantı üzerinden kendi iş parçacığında yoklandı, işin cevabı
  ajana döndü, masaüstünde `is-1` işi kaldı (`a_real_agents_extension_tool_runs_as_a_job_through_mcp`, yok sayılan;
  `cargo build -p kentos-mcp` sonra `--ignored`, 3,3 sn). Aynı sırada eski "gerçek ajan çizer" sınamasının hiç
  koşmadığı ortaya çıktı: ikiliyi yalnız üç üst klasörde arıyordu (cargo'nun derleme dizininde sınama ikilisi
  `target/<profil>/build/<crate>/<özet>/out/` altında) ve bulamayınca hemen düşüyordu; arama genişleyince
  `target/debug/build/kentos-mcp` klasörünü ikili sandı ("Permission denied"), ajan bağlanmadan öldü, sınama sonsuza dek
  bağlantının kapanmasını bekledi. Artık dosya aranıyor (`is_file`) ve sınama MCP v2'nin araçlarıyla yazıyor (`cad.run`
  plan → yürütme, `scene.measure`; `cad.polygon.create` ve `drawing.measure` artık araç değil). Sınamada çıkan tuzak: `cad.processing.run`'ın
  girdisi `parameters`'tır; `ProcessingRun` bilinmeyen anahtarı reddetmediği için `values` yazan çağrı sessizce
  varsayılanlarla çalışıyordu (ilk sınama "Yapılar: seçilen katman artık yok" ile düştü).
- **İşin ilerlemesi ajana ulaşır (15 Kasım).** Tek bir İşlemler çalıştırmasında aracın kendi ilerlemesi (eklentinin
  `ctx.progress`'i, yerleşik aracın adımları) Runner'ın geri bildiriminde düşüyordu (`Collect::progress` boştu): iş
  çalışırken `job` hiç ilerleme söylemiyordu; yalnız toplu işin satırları ve dosyaya yazma söylüyordu. Artık işin sayaçları
  başka iş parçacığından yazılabilen bir alıcı verir (`Progress::sink`, aşama payıyla), `BatchFlow::progress_sink` onu
  tek çalıştırmaya geçirir, Runner aracın payını binde olarak ona iletir (`with_progress`); toplu işin satırları
  eskisi gibi satır arasında söyler. Asistan'ın kartı "Çalışıyor · %40" der (`ToolCall.progress`, yüzde değişince
  yeniden çizilir); MCP ajanı `job`'un `progress`'ini okur. Bekçiler: `jobs::a_progress_sink_reports_from_another_thread_through_its_span`,
  `an_extensions_tools_progress_reaches_the_job_running_it` (sağlayıcının 0,5'i → 500/1000),
  `assistant::tests::a_long_jobs_progress_shows_on_its_calls_card`.
- **Yeni katman açan çalıştırma planlandığı gibi yürütülemiyordu (15 Kasım, gerçek pencerede bulundu).** Asistan
  Vitrin'in TAKS aracını planladı (plan işi arka planda, "İşlem aracını çalıştır · plan — Çalışıyor"), onay kartı geldi,
  Onayla → `plan_changed` "Yazılacak olan onaylanan plana eşit değil"; ajan yeniden planladı, yine farklı bir plan
  kimliği. Sebep iş değil, eskiden beri: aracın açtığı katmanın türetimi (`LayerDerivation.at`) aracın çalıştığı anı
  milisaniyesiyle yazar, katman damgası bütün düğümü (türetimi dahil) özetler ve plan kimliği sonucun katman damgasını
  taşıyordu; plan ile yürütme iki ayrı anda hesaplandığı için kimlik hiç tutmadı. Yerleşik araçta da aynı:
  `annotation.edgeLengths` eşzamanlı yoldan planlanıp yürütülünce `plan_changed`
  (`agent_tools::a_processing_run_that_makes_a_layer_executes_as_planned`, düzeltmeden önce kırmızı). Düzeltme: plan
  kimliği katman ağacını türetimin anı olmadan özetler (`Document::layers_plan_stamp`, docs/adr/0253 §5); yazılan
  katman anını korur, sonucun damgası (`stamps.layers`) da değişmedi. Türetimi olmayan ağaçta kimlik eskisiyle aynı.
- **Ajanın çalıştırdığı aracın seçimi ekranda (15 Kasım).** Aynı koşuda ajan "Sınırı aşan 1 parsel (101/1) seçildi"
  dedi, pencere "Seçili nesne yok" gösteriyordu: aracın seçimi (`ctx.select`) çalıştırmanın cevabında
  (`output.selected`) duruyor ama yalnız İşlemler penceresi onu seçime çeviriyordu. Masaüstü bağlantısı artık yürütülmüş
  bir `cad.processing.run|batch`'in seçtiklerini öndeki çizimde seçer: `call`'da cevaptan, işte durumu `done` olunca
  (yalnız seçen, hiçbir şey yazmayan araç da). İşin durumu çağrının op'unu söylemediği için bağlantı `start` anında
  yürütme olarak başlayan işleri bitene dek hatırlar (`python.executing`); planın seçimi uygulanmaz. Bekçi:
  `python::tests::an_agents_processing_run_chooses_its_tools_objects` (`selection.byExpression`: plan hiçbir şey seçmez,
  `call` ve iş yürütmesi seçer, biten iş unutulur). Gerçek pencerede: Onayla → TAKS katmanı, 101/1 seçili ("1 seçili",
  Alan sekmesi, Öznitelikler "Kapalı alan (1)"); 2242 jeton, 68,3 sn.
- **Kalan:** Kapı 3'ün çalıştıran payı (kuralı sentetik bir çizimin kopyasında koşturmak), bulutla ortak rapor biçimi;
  `ProcessingRun`'ın bilinmeyen anahtarı (`values`) sessizce yok sayması (reddetmek bütün çağıranların sözleşmesini
  değiştirir, ayrı karar).

## §28. Manifestin, katman şemasının ve araç tanımının JSON Schema'ları (`ADIM 21`, kod üretimi)

**15 Kasım.** Kod: `crates/shared/extensions/src/manifest.rs` (`json_schema`; manifestin 38 tipi ve `compat::Os`
şemayı türetir, `schema` özelliğiyle), `sdk/kentos-extension-manifest.schema.json` (24,5 KB), `sdk_pack.rs`
(`MANIFEST_SCHEMA`, `manifestSchema` bölümü), `schema_check.rs` (yalnız sınamada).

- **Karar: panel şemasının yolu (§25).** Şema KentOS'un manifesti okuduğu tiplerden üretilir (schemars), depoda SDK'nın
  yanında durur; tipler değişip şema yazılmazsa test kırmızıdır (`KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions
  --lib -- sdk_pack` yazar). Elle yazılmış bir şema okuyucuyla ayrışırdı: okuyucu bilinmeyen anahtarı reddediyor
  (`deny_unknown_fields`), şema da `additionalProperties: false` diyor; kapalı sözlükler (15 yetenek, 3 kapsam, 4
  program, katkı türleri, menü hedefleri, veri kaynağı türleri) enum olarak, `format` ve `schemaVersion` sabit.
- **Yüzeyler:** `extension.sdk part=manifestSchema`, `kentos kek sdk --part manifestSchema`; paket yalnız nasıl
  alınacağını söyler (Asistan'ın araç sonucu sınırının altında kalır). Şemanın söyleyemediği (yeteneğin kendi kapsamları,
  paketteki dosyalar, kimliklerin tekliği, işleyicinin koddaki işlevi) Kapı 1'indir: `extension.verify` yargıç kalır.
- **Şema yalan söylemesin diye:** sınamalarda schemars'ın kullandığı alt kümeyi denetleyen küçük bir denetleyici
  (`$ref`, `type`, `properties`, `required`, `additionalProperties`, `items`, `enum`, `const`, `oneOf`/`anyOf`/`allOf`,
  `minimum`/`maximum`). KentOS'un gönderdiği beş manifest (fixture'lar: ornek-altlik, ornek-imar, ornek-kent, MEBİS;
  Vitrin) ve bütün katkılarıyla bir iskelet şemaya uyar; uydurma bir anahtar ("hanlder") ve sözlükte olmayan bir yetenek
  reddedilir. Aynı denetleyiciyle Vitrin'in üç paneli panel şemasına uyar, çizilmeyen bir bileşen türü ("saat")
  reddedilir. Depoda JSON Schema doğrulayıcısı yok; bağımlılık eklenmedi.
- **Katman şeması (`schema.json`, `kentos.layerSchema` 1):** serde tipleri yok, denetleyici değeri sabit anahtar
  listeleriyle okuyor (`TOP`, `LAYER`, `FIELD`, `DOMAIN`, `CHOICE`, `RELATION`). Şema elle ama **aynı listelerden**
  kurulur (`layer_schema::json_schema`): bir listeye eklenen anahtar şemanın da anahtarıdır, türü verilmemişse sınama
  düşer; sözlükler (`GEOMETRIES`, `FIELD_KINDS`, `ON_DELETE`) ve sınırlar (256 alt tür, 50 değer grubu, 2–8 alan,
  10 000 bileşim) da oradan. `sdk/kentos-layer-schema.schema.json` (12,2 KB), `extension.sdk part=layerSchema`.
  Vitrin, ornek-kent, MEBİS ve iskeletin katman şemaları şemaya uyar ve denetleyiciden geçer; uydurma anahtar, KentOS'ta
  olmayan alan türü ("string") ve boş katman listesi reddedilir. Denetleyici `minItems`/`maxItems`'ı da okur.
- **Araç tanımı (`tools/<id>.tool.json`, `kentos.processingTool` 1):** doğru okuyucunun yanında, yerel `processing`
  crate'inde (`external::descriptor_schema`): saf `kentos-extensions` ona bağımlı olamaz. Şema okuyucunun sözlüklerinden
  kurulur (eklenti aracının aldığı 8 parametre türü, 4 çıktı türü; çıktı türlerinin listesi `OUTPUT_KINDS` olarak
  okuyucunun hata iletisiyle ortak oldu), parametreler türlerine göre `oneOf`: her türün kendi anahtarları ve ortaklar.
  Okuyucu kullanmadığı anahtarı geçer; şema kapalıdır ki yanlış yazılan anahtar ("lable") yazıldığı yerde söylensin.
  `processing`'in sınaması dosyayı SDK'nın yanına yazar ve denetler (`KENTOS_WRITE_SDK=1 cargo test -p
  kentos-processing --lib -- descriptor_schema`), sözlüklerin okuyucununki olduğunu da sınar: 8 × 4 birleşimin her biri
  okunur. `sdk/kentos-processing-tool.schema.json` (20,4 KB), `extension.sdk part=toolSchema`. Vitrin, ornek-imar,
  MEBİS'in iki aracı ve iskeletin aracı şemaya uyar; yanlış yazılan anahtar, eklenti aracının almadığı tür
  (`rasterValues`) ve katman adlandırmayan yeni nesne çıktısı reddedilir.
- **Kalan:** `ProjectAuditFacts`'in şeması (olgular henüz yok: ADIM 20).

## §29. Eklentinin iletişim kutusu: `ctx.dialog`, `ctx.confirm` (`ADIM 22`, kabulün kalanı)

**15 Kasım.** Kod: `python/kentos/ext.py` (`CommandContext.dialog`, `.confirm`, `_Link.ask`), `ext_host.py`
(bağlamın `channel.call`'u), masaüstü `extensions/dialogs.rs` (açma, alanlar, Tamam/İptal, kapatma), `runner.rs`
(`dialog` çağrısı bekletilir; Durdur ve çalışmanın sonu pencereyi kapatır), `panel_view.rs` (`panel_form`: panelin
bileşen çizimi panel ve pencere için ortak), `shell/view.rs` (pencere katmanı).

- **Karar: pencere bir panelin alanlarıdır.** Eklentinin kodu çalışırken kullanıcıya sorar: `ctx.dialog(başlık,
  alanlar, ok=…, cancel=…)`; alanlar `kentos.ui.panel` girdileri (metin/sayı/tarih/saat/an alanı, seçim, çoklu seçim,
  onay, katman ve alan seçici, yazı). Aynı Kapı 1 denetiminden geçer, aynı kodla çizilir (`panel_form`); ikinci bir
  arayüz dili icat edilmedi (PY-UI-12). Düğme, tablo, grafik ve `onChange` pencerede yok: pencere komut çalıştırmaz,
  onu açan koda cevap verir. `ctx.confirm(metin)` yazılı bir penceredir, evet `True`.
- **Çağrı bekletilir.** Eklenti süreci `dialog` çağrısını mevcut kanaldan gönderir (`Channel.call` cevabı zaten
  beklerken başka iletileri okuyor); masaüstü cevabı hemen değil kullanıcı yanıtlayınca yazar. **Tamam** değerleri
  kimlikleriyle verir, bir panelin komutunun aldığı gibi (yanlış sayı adıyla söylenir, pencere açık kalır); **İptal**,
  Esc ve ✕ `None`. Çalışmanın **Durdur**u ve sonu pencereyi kapatır, kod `None` alır ve ardından Durdur'u duyar
  (`Stopped`); kapanmayan pencere ölü bir çalışmayı beklemez.
- **Hatalı pencere kullanıcıya gösterilmez.** Düğmeli, komutlu ya da bozuk alanlı pencere çağrının hatasıdır
  ("body[0]: “button” pencerede olamaz; olanlar: …"), eklentinin koduna ve günlüğüne gider. Aynı anda tek pencere;
  ikincisi "Bir eklenti penceresi zaten açık" hatası alır.
- **Yetenek istemez:** kullanıcının başlattığı bir komutun (ve harita aracının) penceresidir; havuzda çalışan İşlemler
  aracı, ifade işlevi ve denetim kuralı soramaz (havuz çağrıları geri çevirir, bağlamlarında `dialog` yok).
- **Paket boyutu:** rehbere `ctx.dialog` eklenince SDK paketi Asistan'ın araç sonucu sınırını (48 KB) aştı (49 042
  bayt); §25'teki not gereği çalışan örnek ayrı bölüme alındı (`extension.sdk part=example`, `kentos kek sdk --part
  example`), paket dosya adlarını ve nasıl alınacağını söyler.
- **Vitrin 1.0.22:** "Yapıları numarala" şeritten çalışınca önek ve ilk numarayı sorar; "Yapı ara" panelinin
  düğmesinden çalışınca Y-01'den başlar (panelin değerleri `ctx.params`'ta). Gerçek pencere (`29-pencere`): pencere
  ("Eklenti Vitrini soruyor", Önek Y-, İlk numara 1, İptal/Numarala), B- ve 10 yazıldı, Numarala: "5 yapı numaralandı
  (B-10 … B-14)"; `03-komutlar` pencerede Numarala'ya basıp Durdur'la geri aldı (bildirim başlığı iki satıra kayınca
  Durdur 12 px yukarı çıkmıştı: koordinat ölçülüp düzeltildi).
- **Klavye** (gerçek pencere, koyu tema, 16 px, 1100×650): Tab alandan alana geçmiyor, Esc pencereyi kapatmıyordu.
  Neden: yazan bir eklenti çalışırken çizim korunuyor ve tuş iletileri sessizce tutuluyor; pencere de o çalışmanın
  içinde açık. Pencere açıkken tuşlar artık ona gider: Tab ve Shift+Tab alanlarında döner (`kentos_ui::focus`, pencere
  kapsam), Esc İptal, Enter Tamam (metin alanının Enter'ı kendisinin); arkadaki çizime bir tuş ulaşmaz. Ölçüm: C- ve
  Tab, 20 ve Enter → "5 yapı numaralandı (C-20 … C-24)"; Esc → "Vazgeçildi: numara yazılmadı".
- **Sınamalar:** Python 1 (sahte KentOS: pencerenin başlığı, düğmeleri, alanları; Tamam'ın değerleri, İptal'in `None`,
  onayın evet/hayırı), masaüstü 1 (girdisini dosyaya yazan sahte süreç: pencere açılır ve çizilir, yanlış sayı adıyla
  söylenir, Tamam'ın değerleri ve İptal'in boşu sürece yazılır, Esc yazan bir çalışma sürerken de İptal'dir, düğmeli
  pencere hemen hata, Durdur pencereyi kapatıp boş yanıt verir).

## §30. Ajanın atölyesi masaüstü bağlantısında: incelemeye açma, canlı yükleme, görünüş (`ADIM 21`)

**15 Kasım.** Kod: masaüstü `python/agents.rs` (bağlantının istekleri: `review_workshop`, `reload_extension`,
`preview_extension`), `extensions/workshop.rs` (`agent_review_workshop`, `ext_workshop_reload_from`: klasör ve "kim"
parametreli), `extensions/view.rs` (inceleme bandı kimin atölyesi olduğunu söyler), başsız
`agent_tools/link_workshop.rs` (`link_review_workshop`, `link_reload`, `link_preview`), MCP `tools.rs` (`WithWorkshop`).

- **Karar: dış ajan Asistan'ın yolundan.** Masaüstüne bağlı MCP ajanı (desktop.attach) kendi atölyesinde yazdığı
  eklentiyi `extension.workshop.review` ile kullanıcının masaüstündeki Gözden geçir ve kur penceresine açar; kullanıcı
  o klasörden kurduktan sonra `reload` değişiklikleri uygulama kapanmadan yükler, `preview` kullanıcının gördüğünü
  yazıyla verir. Önce yalnız Asistan yapabiliyordu; MCP ajanı paketi kullanıcıya "Paket yükle… ile açın" diye bırakıyordu.
  Kur ve yetenekler yine kullanıcının; yeni yetenek isteyen yükleme yine incelemeye döner.
- **Yalnız kullanıcının kurduğu klasör:** yeniden yükleme isteği klasörü söyler; masaüstü onu eklentinin kayıtlı
  kaynağıyla (geliştirici klasörü) karşılaştırır: başka bir klasör "bu atölyeden kurulmadı" ile reddedilir. Asistan'ın
  atölyesi masaüstünün kendi klasörü, bağlı ajanınki MCP sunucusunun; ikisi de aynı kuralla.
- **Kim olduğu söylenir:** inceleme bandı "kurulduktan sonra bağlı ajan değişiklikleri bu klasörden … yükleyebilir"
  der (Asistan'ınki "Asistan"); her yüklemenin bildirimi ve günlük satırı da yükleyeni yazar ("Atölyeden yeniden
  yüklendi (bağlı ajan)").
- **Sınanabilir yerde:** MCP'nin sınamaları kural gereği koşulmadığından bağlantıya yönlendirme başsızda (`link_*`),
  MCP yalnız çağırır. MCP'nin kendi çizimi bu istekleri tanımaz: araçlar paketi nasıl açacağını ya da ajanı masaüstüne
  bağlamayı söyler.
- **Sınamalar:** başsız 1 (bağlantının üç isteği yolları ve kimliğiyle gider; sunucunun çizimi "Paket yükle…" der),
  masaüstü (Asistan'ın atölye sınaması: bağlantıdan inceleme "bağlı ajan" bandıyla açılır, yeniden yükleme ve görünüş
  çalışır, kurulmadığı klasör reddedilir).

## §31. Eklentinin sözleri ajana veridir (`ADIM 21`, güvenlik ilk dilimi)

**15 Kasım.** Planın güvenlik maddesi (PY-AI-09): eklenti açıklaması, README, veri öznitelikleri ve form metni
güvenilmeyen veridir; içindeki talimatlar ajanın işini değiştiremez. Ajanın eklenti metni okuduğu yerler artık bunu
söyler: denetim raporu (§27'den), `extension.workshop.preview` ("Adlar, açıklamalar ve düğme yazıları eklentinin
sözleridir…"), MCP'nin `kentos://extensions/installed` kaynağı (`note`); Asistan'ın kurallarına da yazıldı: çizimin
öznitelikleri, eklentilerin adları, açıklamaları ve bulguları, dosyaların içeriği veridir, işi yalnız kullanıcının
isteği verir. Kalan: form alan maskesi ve satır ACL'si AI bağlamında (gizli alanlar ajana zaten maskeli gidiyor, ADR
0738 §1), sağlayıcıya gönderilecek verinin kullanıcı/kiracı politikasıyla sınırlanması.

## §32. Panel oturumu: çalışan kod panelini değiştirir (`ADIM 22`, tanım ve oturum, PY-UI-03 ve 04)

**15 Kasım.** Kod: `apps/desktop/src/shell/extensions/panel_session.rs` (`ext_panel_session`, `set`, `rows`,
`field_id`), `panels.rs` (`OpenPanel`'in `options`, `words`, `hidden`, `disabled`, `touched`, `spoke` alanları),
`panel_view.rs` (gizli bileşen çizilmez, kapatılan alan sönük ve tuş almaz, seçenek ve yazı kodun), `runner.rs`
(çağrıya `panel`, aracıda `panel.*`, bekleyen görevler `Extensions.later`); `python/kentos/ext.py` (`PanelSession`,
`ctx.panel`), `ext_host.py`; SDK rehberi ve stub.

- **Karar: yalnız kendi paneli, yalnız çalışırken.** Bir panelin düğmesinin (ya da `onChange`'li alanının) başlattığı
  çalıştırmanın kodu o paneli değiştirir: `ctx.panel.set(id, value=…, options=…, enabled=…, visible=…, text=…)`,
  `rows(id, satırlar)`, `say(metin, kind)`, `focus(id)`, `close()`. Çağrı panele değil çalıştırmanın kökenine
  (`PanelOrigin`) gider; şeritten çalışan komutun `ctx.panel`'i None, masaüstüne gelen `panel.*` isteği `no_panel`.
- **Oluştur: `ctx.open_panel(id)`.** Komut eklentinin bildirdiği bir panelini açar (açıksa öne getirir) ve onu
  değiştirmek için alır: şeritten çalışan komut yaptığını panelinde gösterir. İstek panelin kimliğini taşır (`panel`;
  oturumun her isteği taşır), masaüstü onu eklentinin kurulu manifestinde arar: bildirilmemiş panel `unknown_panel`
  (panelleri sayar), yalnız kendi panelleri. Açılan panel çalışma bitene dek tutulur (`held`): kullanıcının o sırada
  değiştirdiği girdi onun kalır; bitince (ya da süreç kapanınca) bırakılır (`ext_panels_released`).
- **Kullanıcının yazdığı kazanır (PY-UI-04):** komut çalışırken kullanıcının değiştirdiği girdiler (`touched`, her
  çalıştırmanın başında boşalır) kodun değerini almaz: `set` False döner, çağrının diğer özellikleri (seçenek, etkinlik,
  görünürlük) uygulanır. Odak, metnin imleci ve panelin kaydırması yamadan etkilenmez: form panelin durumundan çizilir,
  bileşenler kendi durumlarını tutar. Kodun koyduğu değer `onChange`'i çalıştırmaz (döngü olmaz).
- **Tanım gibi denetlenir:** olmayan kimlik (`unknown_component`, panelin kimlikli bileşenlerini sayar), bileşenin
  almadığı özellik ("“liste” (table) enabled almaz: düğmeler ile metin, sayı, tarih ve saat alanları kapatılır"),
  yanlış türde ya da sınırı dışında değer ("“adet” (numberField): 0 ile 10 arasında olmalı"), seçeneklerde olmayan seçim,
  Kapı 1'in seçenek kuralı (boş olmayan değer ve ad, tekrarsız) kodun hatasıdır: `kentos.HostError`, çağrının hiçbir
  parçası uygulanmaz; eklentinin günlüğüne de uyarı olarak yazılır (`panel_refused`, bölüm `panel:<kimlik>`): kod hatayı
  yakalayıp sürebilir, yazarı izi günlükte bulur. Kapanmış panel hata değildir: `applied: false, closed: true`
  (kullanıcı kapattı).
- **Ne nasıl değişir:** değer alanın kullanıcı yazımıyla saklanır (sayı "4,5", tarih "15.11.2026"; kod ISO verir ya da
  `datetime.date`); yeni seçeneklerde olmayan seçim bırakılır (seçimde ilk seçenek); gizlenen zorunlu alan kullanıcıya
  sorulmaz ama değeri komuta gider; `say`'in sözü çalıştırma kendi sözünü söylemeden biterse kalır (`spoke`), satır
  sayısıyla ezilmez; `rows` grafiğini yeniden hesaplatır; başka sekmedeki panelin değişmesi sekmesinde nokta
  (`unseen`). `focus` yalnız gösterilen panelde, görünür ve açık yazı alanında: kod kullanıcının baktığı sekmeyi
  değiştirmez (False).
- **Katman menüsünün komutu katmanı alır.** Eklentinin `contextMenus` öğesi (`target: "layer"`) şerit düğmesi gibi
  çalışıyordu: komut sağ tıklanan katmanı bilmiyordu (QGIS'in katman eylemleri katmanı alır). Artık menü katmanın
  kimliğini verir (`extension_menu`'nün `on`'u, `Event::RunOn`), çalıştırmanın çağrısında `params: {"layer": …}` olur
  (`ctx.params["layer"]`); grubun menüsü katman vermez. Bekleyen hedef çalıştırmanın başlatılmasında alınır
  (`target_params`, harita aracının şekli gibi), başlamayan çalıştırmayla gider: sonra şeritten çalışan aynı komut onu
  almaz. Bekçi: `a_layer_menus_command_gets_the_layer_it_was_run_on` (sürece giden çağrının parametreleri). Vitrin
  1.0.27: katmanın menüsünde "Değerlerini say", Değer say'ı o katman seçili açar (`ctx.open_panel` + `set`). Gerçek
  pencerede (`33-katman-menusu`): etkin katman Taslak iken Parsel sınırı'nın menüsünde eklentinin iki öğesi (Katman
  özeti, Değerlerini say); tıklanınca Değer say "Kadastro › Parsel sınırı" seçili açıldı (etkin katman değil, tıklanan),
  altında kodun satırı; Alan › ada ile sayım "En sık değer: “101” (3 nesne)."
- **Büyük veri: sayfalar (15 Kasım).** Panel tablosu ilk 500 satırı çizip "… N satır daha" diyordu: kalanlara
  ulaşılamıyordu; ağaç da yalnız ilk 500 satırı gruplayıp grup sayılarını eksik söylüyordu (501. satırdan sonraki
  değerler sayılmıyordu). Artık tablo sayfa sayfa (200 satır), kartlar 100'er; altında "‹ Önceki · 201–400 / 12.340 ·
  Sonraki ›" (Türkçe binlik ayraç), sayfa bileşen başına tutulur ve komutun ya da `rows`'un yeni satırlarıyla başa döner.
  Ağaç bütün satırları sayar, ilk 300 grubu ve grup başına 100 yaprağı çizer ("… N daha", "… N grup daha"); gruplama
  her karede çalıştığı için gruplar haritayla bulunur (önce satır × grup aramaydı). Bekçiler:
  `panel_view::tests::pages_go_through_every_row_and_counts_read_as_turkish`, `a_trees_counts_are_all_its_rows` (700
  satır, 7 değer, her biri 100), `a_tables_pager_moves_and_new_rows_start_at_its_first_page`.
- **Vitrin 1.0.26:** "Yapıları numarala" şeritten çalışınca bitince Yapı ara panelini açar: Yapı no önekle dolar,
  numaralananlar tabloda (Ara'nın satır biçimiyle: tıklanan satır yapıyı gösterir), altında özet (`32-panel-ac`).
  Gerçek pencerede: B- ve 10 ile Numarala → Eklenti sekmesi Asistan'ın yanında açıldı, Yapı no "B-", altında "5 yapı
  numaralandı (B-10 … B-14).", tabloda B-10 400 … B-14 200; satıra tıklamak yapıya gitti ("1 seçili"). **Bulgu
  (düzeltildi):** `rows` sütunları anahtarlardan çıkarıyordu ve JSON nesnesi sırayı taşımadığı için tablo "alan (m²) |
  no" diziliyordu; SDK artık sütunları satırların verdiği sırayla gönderiyor (raporun yaptığı gibi). Senaryo notu:
  sürecin ilk açılışı (SDK yenilenince) 3 sn'yi geçti, pencere gelmeden yazılanlar komut satırına gitti; bekleme 5 sn.
- **Vitrin 1.0.24–25:** "Değer say" sayınca bulduğu değerleri yeni Seç kutusuna koyar (seçenekler veriden gelir:
  panelin JSON'u onları bilemez), sıklıklarıyla ("101 (3)"), en sık değeri formun altındaki satıra yazar; Seç'te bir
  değer o değerdeki nesneleri çizimde seçer (`deger-sec`, `onChange`). Gerçek pencerede (`31-oturum`): Parsel sınırı ve
  ada seçilince satır "En sık değer: “101” (3 nesne).", Seç "Seçmeyin / 101 (3) / 102 (1)"; 101 (3) seçilince üç parsel
  seçili ("3 seçili", Öznitelikler "Kapalı alan (3)", panelde "“ada” = “101”: 3 nesne seçildi."). 1.0.24'te
  açıklamayı uzatmak Katman ve Alan kutularını 30 px kaydırıp eski senaryoların tıklamalarını boşa düşürdü; 1.0.25
  açıklamayı eski boyuna döndürdü, Seç'in yönergesi sayımdan önce altındaki satırda. Yeni iki form satırı ağacı 89 px
  indirdiği için `20-agac`'ın tıklamaları yeni yere alındı (gerçek pencerede yeniden koşuldu).
- **Sınamalar:** masaüstü 4 (değerler, seçenekler, yazı, etkinlik ve görünürlük tanımın izin verdiği gibi, yanlış parça
  hiçbir şey uygulamaz; kullanıcının yazdığı kazanır; satırlar tabloya, düğmeye değil; Vitrin'in Yapı ara paneli
  aracıdan uçtan uca: değer, söz, imleç, çalışma sırasında yazılan alan korunur, yanıtın sözsüz satırları tabloya,
  kodun sözü kalır, panelsiz çalıştırma `no_panel`), Python 1 (`ctx.panel`'in istekleri, tarih ISO metne, panelsiz
  None). Ayrıca SDK stub'ı özel nöbetçi varsayılanı (`_KEEP`) `...` yazıyor: stub'da tanımsız ad kalmıyor.

## §33. Bileşen kuralları: `visibleWhen`, `requiredWhen`, `enabledWhen` (`ADIM 22`, PY-UI-08)

**15 Kasım.** Kod: `crates/shared/extensions/src/ui_panel/rules.rs` (`Rules`, `inputs_of`, `compile`, `holds`),
`ui_panel.rs` (`read` kuralları bileşenin anahtarlarından okur, Kapı 1'de `takes_rule` ve derleme, JSON Schema'da
anahtarlar), masaüstü `panels.rs` (`CompiledRules`, `shown`, `usable`, `asked`, `rule_value`), `panel_view.rs`.

- **Karar: formun ifade dili, Python'suz.** Bileşen kuralı KentOS'un ifade diliyle panelin girdilerini kimlikleriyle
  okur; formun kurallarıyla aynı sözcükler (`visibleWhen`, `requiredWhen`) ve aynı doğruluk (`truthy`: boş doğru
  değildir). Masaüstü kuralı kullanıcı değer değiştirdikçe hesaplar: süreç başlamaz, ortamı hazırlanmamış eklentinin
  paneli de kurallarıyla çalışır. `enabledWhen` düğmede ve metin, sayı, tarih, saat, an alanlarında; `requiredWhen`
  zorunlu olabilen girdide; `visibleWhen` her bileşende.
- **Girdiler türleriyle:** sayı alanı sayı (`2,5` yazılsa da `adet > 0` doğru), onay evet/hayır (`tamam`), tarih ve an
  ISO (sırayla karşılaştırılır), çoklu seçim seçilenler birleşik metin (`içerir(bilgiler, 'tarih')`), gerisi yazıldığı
  gibi. Tireli kimlik köşeli parantezle yazılır (`[en-sik]`; çıplak yazılınca çıkarma okunur).
- **Kapı 1:** kural, bileşenin alamadığı anahtar ("“requiredWhen” table bileşeninde bilinmeyen bir alan; olanlar: …,
  visibleWhen"), girdi kimliği olmayan ad ("“mahalle” paneldeki bir girdinin kimliği değil; girdiler: …"), `@` değeri,
  derlenmeyen ifade ve metin olmayan değer yerinde söylenir. Panelin JSON Schema'sı kuralları bileşen başına tanır
  (editör tamamlaması); karar Kapı 1'indir.
- **Oturumla birlikte (§32):** bileşen kural tutuyorsa ve kod gizlemediyse görünür, kural tutuyorsa ve kod kapatmadıysa
  kullanılır; zorunluluk `required` ya da `requiredWhen`. Gizli girdi (kuralla ya da kodla) sorulmaz ama değeri komuta
  gider; yıldız `requiredWhen` tutunca çıkar.
- **Vitrin 1.0.28:** Parsel kartı'nın Kart tarihi ve Saat alanları yalnız Bilgiler'de Tarih seçiliyken görünür.
  Gerçek pencerede (`34-kural`): Tarih çipi bırakılınca iki satır kalktı, Geçerlilik sonu ve çipler yukarı kaydı (çipin
  `onChange` komutu da ayrıca çalıştı); yeniden seçilince satırlar döndü.
- **Sınamalar:** paylaşılan 3 (değerleri türleriyle okur; girdi olmayan ad, `@`, derlenmeyen ifade, köşeli parantez;
  Kapı 1'in yerleri), masaüstü 1 (onay kapalıyken alan gizli ve sorulmaz, açılınca görünür ve zorunlu, `2,5 > 0` ile düğme
  kullanılır, kodun kapatması kuralı geçer).

## §34. Panel bileşenlerinin Python işlevleri: `kentos.ui_panel` (`ADIM 22`, PY-UI-02)

**15 Kasım.** Kod: `crates/shared/extensions/src/ui_panel/python.rs` (`python_builders`), üretilen
`python/kentos/ui_panel.py`, SDK bölümü `panelBuilders` (`sdk_pack.rs`).

- **Karar: şemadan üretilir, elle yazılmaz.** Bileşen başına bir işlev (`text_field`, `number_field`, `select`,
  `feature_picker`, `chart`, …), panelin JSON Schema'sından (§25, `json_schema`) üretilir: gerekli anahtarlar sırayla
  konumsal, kalanı yalnız adıyla (`on_change`, `visible_when`), dönen değer bileşenin JSON'u. Verilmeyen anahtar
  yazılmaz; evet/hayır anahtarı yalnız varsayılanından farklıysa yazılır (`multiple=False`, `required=True`).
  Seçenekler yalnız değerle verilebilir (`["A", "B"]`), `choice(value, label)` bir seçenek, `panel(title, body)` bütün
  panel. Türler şemadan: sayı `float`, seçenek listesi `Sequence[Choice | str]`, grafiğin türü ve toplamı `Literal`.
  Şema değişince (yeni bileşen, anahtar, kural) modül yeniden üretilir; sınama ikisi ayrışınca kırılır
  (`KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- python_builders`).
- **Neden:** `ctx.dialog`'un alanları, `set(options=…)` ve Python'dan yazılan paneller elle JSON'du: yanlış yazılan bir
  anahtar (`"requred"`) Kapı 1'e ya da çalıştırmaya kadar görünmüyordu. İşlevde yanlış anahtar Python'da `TypeError`;
  editör ve ajan imzayı görür. Karar yine Kapı 1'in ve oturumun: düğmenin komutu, kimliklerin tekliği, kuralın
  adları şemada söylenemez.
- **Ajanlar:** SDK kılavuzunda bir paragraf (paket 48 KB'nin altında kalır); kaynağın tamamı ayrı bölüm
  (`extension.sdk part=panelBuilders`, `kentos kek sdk --part panelBuilders`, 16 KB), her istekte şemadan üretilir.
- **Vitrin 1.0.29:** "Yapıları numarala"nın penceresi (`ui.text`, `ui.text_field`, `ui.number_field`) ve "Değer
  say"ın Seç seçenekleri (`ui.choice`) işlevlerle; davranış aynı. Gerçek pencerede: güncelleme "1.0.28 → 1.0.29
  güncellendi", ortam yeni KentOS kitaplığıyla kuruldu; `29-pencere` aynı pencere (açıklama, Önek, zorunlu İlk numara),
  B- ve 10 ile "5 yapı numaralandı (B-10 … B-14)."; `31-oturum` Seç "Seçmeyin / 101 (3) / 102 (1)".
- **Sınamalar:** paylaşılan 1 (modül şemanınki; imzalar; satır en çok 120 karakter), Python 5 (yalnız verilen yazılır;
  seçenekler yalnız değerle; yanlış anahtar `TypeError`; örnek panellerin her bileşeni işleviyle aynı yazılır;
  parametreler bileşenin anahtarları, gerekliler konumsal).

## §35. Hesaplı değerler: `calculate` ve `defaultExpr` (`ADIM 22`, PY-UI-08'in kalanı)

**15 Kasım.** Kod: `crates/shared/extensions/src/ui_panel/values.rs` (`VALUE_KEYS`, `takes_value`, `Moment`,
`compile`, `value_of`, `calc_order`), `ui_panel.rs` (Kapı 1 ve JSON Schema), masaüstü `panel_values.rs`
(`moment_now`, `apply_defaults`, `recalculate`), `panels.rs` (`CompiledRules::calculate`, `calc_order`,
`calculated`, `usable`), `panel_session.rs` (ret), `dialogs.rs`, `panel_view.rs`.

- **Karar: formun sözcükleri, Python'suz.** `calculate` bileşenin değerini panelin girdilerinden hesaplar ve okuduğu bir
  değer (kullanıcının, kodun ya da başka bir hesabın) değiştikçe yeniden hesaplar; `defaultExpr` alanın ilk değerini panel
  açılırken bir kez hesaplar (üstündeki alanlar değerlerini aldıktan sonra), sonra değer kullanıcınındır. İkisi formun
  ifade diliyle ve girdileri kural gibi okur (§33); ayrıca `bugün()` ve `şimdi()` (`@tarih`, `@simdi`): panelin
  açıldığı an, saatle ilerlemez. Başka `@` değeri yoktur.
- **Kim alır:** `calculate` metin ile metin, sayı, tarih, saat ve an alanları; `defaultExpr` bu alanlar. Metinde hesap
  yazıyı verir, `text` hesap boşken görünür (`eğer(boş(ilk), boş, …)`).
- **Hesaplanan bileşen yazılmaz, sorulmaz, kodla değişmez:** alan soluk ve tuş almaz (`usable`), değeri komuta her
  girdi gibi gider; `ctx.panel.set` değerini ya da yazısını reddeder (`invalid_input`, "hesaplanan bir bileşen:
  değerini kendi hesabı (calculate) yazar"); kod okuduğu bir girdiyi değiştirirse hesap onu izler.
- **Sonuç alanın sözleriyle:** sayı alanın yazdığı gibi (`2,5`), tarih `GG.AA.YYYY`, saat `SS:DD`, an
  `GG.AA.YYYY SS:DD`; o tür değilse boş. `şimdi()` tarih alanına günü, saat alanına saati verir.
- **Kapı 1:** derlenmeyen ifade, girdi olmayan ad, başka `@` değeri yerinde söylenir; `calculate` ile `value`,
  `defaultExpr`, `required`, `requiredWhen`, `enabledWhen` ve `onChange` birlikte olmaz (her biri kendi işaretçisinde,
  nedeniyle); `defaultExpr` ile `value` birlikte olmaz. Hesaplar okuduklarından sonra hesaplanır (`calc_order`):
  kendini ya da birbirini döngüyle okuyan hesap reddedilir ("Hesaplar birbirini okuyor: b → c → b"). İletişim
  kutusunun alanları aynı denetimden geçer.
- **Python cephesi:** üretilen işlevler iki anahtarı da alır (`ui.text(…, calculate=…)`, `default_expr=`); JSON
  Schema bileşen başına tanır.
- **Vitrin 1.0.30:** "Yapıları numarala" penceresinin açıklaması hesaplı (yerleşim kaymasın diye yeni satır değil, var
  olan açıklama); Parsel kartı'nın Kart tarihi `defaultExpr: "bugün()"`. Gerçek pencerede (`35-hesap`): açılınca
  "“yapi” katmanındaki 5 yapıya Y-01 … Y-05 yazılır.", B- ve 10 yazılınca "… B-10 … B-14 yazılır.", İptal "Vazgeçildi";
  Parsel kartı Kart tarihi "15.11.2026" ile açıldı. `24-tarih` artık yazmadan önce alanı seçer (ctrl+a), kartlar
  20.11.2026 ile yenilendi.
- **Sınamalar:** paylaşılan 4 (değer girdilerden ve andan; Kapı 1'in sözleri; bütün panelde yerler ve döngü; sıra ve
  döngü), masaüstü 2 (açılışta varsayılan, hesap değerleri izler, alan yazılmaz, değerler komuta; kod hesaplanana
  yazamaz, okuduğunu değiştirince hesap izler).

## §36. Rust'ın ölçtüğü denetim olguları: `ProjectAuditFacts` (`ADIM 20`, olguların ilk dilimi)

**15 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/audit_facts.rs` (`ProjectAuditFacts`, `LayerFacts`,
`ValidityProblem`, `MOST_ITEMS`, `json_schema`) ve şeması `sdk/kentos-audit-facts.schema.json` (SDK'nın `factsSchema`
bölümü), İşlemler `crates/native/processing/src/audit_facts.rs` (`project_facts`, `MOST_OBJECTS`), masaüstü
`extensions/audits.rs`, `python/kentos/ext.py` (`GisAuditFacts.project`, `ProjectAuditFacts`, `LayerFacts`,
`InvalidGeometry`, `Overlap`, `Gap`); örnek çizim ve olguları `fixtures/audit-facts/v1/denetim.kcad`, `denetim.json`.

- **Karar: geometriyi Rust çıkarır, kararı Python verir.** Kural katmanları nesne nesne okumayı sürdürür (§17);
  yanında `facts.project`, KentOS'un çizimde bir kez ve geometri çekirdeğiyle ölçtüğü özettir: her katmanın nesneleri
  geometriye göre (nokta, çizgi, alan, diğer), alanlarının toplam alanı ve çizgilerinin toplam uzunluğu (yaylar
  kesin, deliklerin alanı düşülmüş), ve Topoloji sekmesinin üç kuralının bulduğu: geçersiz geometriler (nesne, sorun,
  yer), aynı katmandaki alanların çakışmaları (iki kalıcı kimlik, ortak alan, en büyük parçanın ağırlık merkezi),
  alanların birleşiminin kapattığı boşluklar (parseller arası boşluk ya da alanın deliği: alanı, yeri, çevresindeki
  alanlar, en uzun sınırı paylaşan önce). Projenin topoloji toleransıyla ve istisnaları uygulanmadan: olgular
  çizimindir, ne anlama geldiklerini kural söyler. Kural shapely istemez, geometriyi yeniden hesaplamaz.
- **Neden ayrı bir özet:** `GisAuditFacts`'in GeoJSON'u yayları parçalanmış verir (alan biraz eksik çıkar); çakışma ve
  boşluk Python'da paket ister ve yavaştır; aynı ölçü masaüstü ile bulut işinde başka kütüphanelerle başka çıkabilirdi.
  Özet iki yerde aynı Rust kodudur.
- **Aynı baytlar:** sıra, yuvarlama ve sınır paylaşılan modülde yazılıdır ve her üretici olguları
  `ProjectAuditFacts::new` ile kurar. Katmanlar ağacın sırasıyla; geçersizler nesne, sorun, yer; çakışmalar kimlik
  çifti (küçüğü önce), yer; boşluklar kuzeyden güneye, batıdan doğuya, alan. Alan 0,0001 m², uzunluk ve yer 1 mm'ye
  yuvarlanır (değer × 10ⁿ sıfırdan uzağa, geri bölünür); −0 ve sayı olmayan değer 0 yazılır; tolerans projenindir.
  Nesneler çekirdeğe çizimin sırasıyla değil kalıcı kimlik sırasıyla gider: aynı nesneler hangi sırayla çizilmiş ya da
  okunmuş olursa olsun aynı baytları verir. Harita sırası, saat ya da iş parçacığı sayısı girmez.
- **Sınırlar:** bir liste en çok 10 000 öğe tutar (sırasındaki ilkleri), `totals` hepsini sayar; en çok 200 000 nesne
  (kalıcı kimlik sırasıyla ilkleri) ölçülür, ötesi kalınca `cut`. Masaüstü ile bulut aynı sınırı (`MOST_OBJECTS`)
  verir. Halkanın kapanmaması yazılamaz (alanın halkası kendiliğinden kapanır): hiçbir alana varmayan halka `zeroArea`,
  alan katmanındaki açık çizgi `lines` sayısında görünür.
- **Masaüstü:** olgular kuralın arka plan görevinde, denetimin başladığı anki çizimin okuma kopyasında ölçülür; arayüz
  beklemez, Durdur ölçümü de keser. Kuralın sürecine `facts` içinde `project` olarak gider; ajanın `audit.run`'ı aynı
  yoldur (§27). Hiçbir öznitelik okunmaz: gizli alan olgulara girmez.
- **Python cephesi:** `facts.project.layer("parsel")` → `LayerFacts` (`points`, `lines`, `areas`, `other`, `area`,
  `length`, `invalid`, `overlaps`, `gaps`, `totals`); öğeler değiştirilemez veri sınıfları (`InvalidGeometry`,
  `Overlap`, `Gap`), yerler `(x, y)`; nesnesi olmayan katman boş olgular verir. Olguları vermeyen KentOS'ta
  `facts.project` `NotAvailable` atar: yetenek eksiktir, kural bozuk değil. Stub ve rehber güncellendi; şema
  türlerden üretilir, bekçisi kayıtlı şemanın üretilenle aynı olduğunu ve KentOS'un yazdığı olguları aldığını sınar.
- **Kabulün payı:** sabit örnek çizimin olguları baytı baytına `denetim.json`'dur ve Python sınamaları aynı dosyayı
  okur; bulut işi aynı çizimi aynı kodla ölçünce aynı dosyayı vermelidir.
- **Sonraki dilim:** mimari olgular (`paiy_areas`: bağımsız bölümlerin brüt ve net alanları; `kentos_domain::escape`:
  kaçış yolları ve uzunlukları) aynı belgede ayrı bir bölüm olarak; ikisi de `Document` üzerinde hazır ve okuma
  kopyasında hesaplanabilir, ama biçimleri ve sınamaları ayrı bir iş. Bulutta `RunRequest::Audit` ve ortak köken
  (olguların özeti rapora). Denemede (§37) kurallar olguları henüz almaz.
- **Sınamalar:** paylaşılan 4 (yuvarlama ve sıra; liste sınırı ve toplam; JSON biçimi ve geri okuma; şemanın bekçisi),
  İşlemler 8 (katmanların olguları; örneğin baytları; her sırada aynı baytlar; sınır kalıcı kimlikle; Durdur; tolerans
  projenin; alanın deliği boşluk; sorun adları çekirdeğin), Python 5 ve eklenti süreci 1 (Rust'ın olgularını tipli
  okur; kural yalnız olgulardan bulgu verir; olgusuz KentOS'ta yetenek eksik).
- **Vitrin 1.0.32:** şeridin yeni Denetim grubunda "Parsel çakışması": çakışmayı ve boşluğu KentOS ölçer, kural
  bulguyu parselin ada/parsel adıyla yazar (çakışma hata, boşluk uyarı; düzeltme önermez). Gerçek pencerede
  (`37-olgular`) örnek çizimin parselleri temiz: "Bulgu yok" (1,6 sn), köken 1.0.32.

## §37. Kapı 3'ün kodu çalıştıran payı: sentetik çizimde deneme (`ADIM 21`)

**15 Kasım.** Kod: başsız `agent_tools/workshop_trial.rs` (`extension.workshop.trial`, `trial_outcome`,
`Host::trial_wait`), masaüstü `extensions/trial.rs` (`ext_trial_start`, `ext_trial_report`, `run`) ve
`trial/drawing.rs` (`synthetic`, `written`),
`python/agents.rs` (`trial`, `trial_report` istekleri), Asistan `host.rs` ve `mod.rs` (`Pending::Trial`),
`kentos_processing::external::shape` (GeoJSON'dan nesne, artık açık).

- **Karar: kullanıcının kurduğu eklenti, ajanın verdiği sentetik çizimde.** §26'nın ayırdığı pay: eklentinin kodu
  ancak kullanıcı kurduktan sonra ve ortamı hazırken çalışır; ajan ona kullanıcının çizimini değil, kendi verdiği örnek
  nesnelerden kurulan bir çizimi verir (yeni CBS projesi, TUREF / TM36; her nesne katman adı, GeoJSON şekli ve
  öznitelikleriyle; basit ad katmanın kimliği de olur, projenin var olan katmanı kullanılır). Çizim arka plan görevinde
  yaşar, deneme bitince gider; kullanıcının çizimine, geri alma geçmişine ve dosyalarına dokunulmaz.
- **Ne çalışır:** eklentinin denetim kuralları (verilenler, yoksa hepsi) çizimin olgularını okur, masaüstündeki
  denetim gibi (`audit_facts`, aynı süre sınırı); istenen İşlemler araçları (`cad.processing.run` girdisiyle,
  yalnız bu eklentininkiler) çizimde yürütülür. Adımlar sırayla, eklentinin kendi sürecinde; Durdur (ajanın turu
  durunca) adımlar arasında ve çalışan kuralın sürecinde durdurur. Aynı anda bir deneme.
- **Rapor:** her kuralın bulguları nesnenin ajanın listesindeki sırasıyla (`feature`), önemi, sözleri ve düzeltmesinin
  başlığı ve komutu; hata veren kural istisnası ve iz kaydıyla (denemenin bulgusu, çağrının hatası değil); her aracın
  özeti, uyarıları ve yazdıkları (katman başına sayı ve ilk 5 nesne GeoJSON ve öznitelikleriyle); bir satırlık özet.
  Bulgular eklentinin sözleridir: ajan veri olarak okur (§31).
- **Olgular:** denemenin kuralları KentOS'un ölçtüklerini de okur (`facts.project`, §36), çizimin denetimindeki gibi;
  ölçüm kırılırsa kurallar katmanlarla çalışır. Gerçek pencerede Vitrin 1.0.32'nin Parsel çakışması, 2 m iç içe iki
  20×20 m sentetik parselle: "101/1 ile 101/2 40.00 m² çakışıyor." (hata).
- **Yüzeyler:** Asistan çağrıyı deneme bitince cevaplar (arayüz iş parçacığında beklenmez, denetimin yolu); masaüstüne
  bağlı MCP ajanı `trial_report`'u sorar. Masaüstü olmayan bağlantı nerede çalıştığını söyler (`not_offered`).
  SDK'nın iş akışı aracı anar.
- **Gerçek pencerede** (Asistan, DeepSeek, yalnız sentetik veri): "Vitrin'in Yapı numarası denetimini sentetik bir
  çizimde dene: yapi katmanında iki kare yapı, birinin no'su Y-01, diğerinin yok" → ajan `extension.workshop.trial`
  çağırdı; 1 uyarı (numarasız yapı, sıradaki numara Y-02, düzeltmesi `cad.entities.set`), Y-01'e bulgu yok. TAKS aracı
  20×20 m parsel ve içinde 10×10 m yapıyla: taban alanı 100 m², TAKS 0,25, uygun; kural verilmediği için eklentinin
  denetimi de koştu (numarasız yapı, Y-01). İki koşuda da kullanıcının çizimi aynı kaldı.
- **Sınırlar:** komutlar (`@ext.command`) ve harita araçları denenmez: aracı üzerinden canlı çizimle konuşurlar ve
  kullanıcıya sorabilirler (bir sonraki dilim; §52'de denenir); şerit ve panelin kurulması ile kaldırmada kalıntı §26'nın
  kuru bağlamasında.
- **Sınamalar:** başsız 2 (çağrı masaüstüne olduğu gibi gider, raporla cevaplanır; nesnesiz ve masaüstüsüz çağrı
  söylenir; atölye araçlarının listesi), masaüstü 2 (sentetik çizim nesnelerden kurulur, katman adları ve öznitelikler,
  yanlış şekil yeriyle; bulgular nesnenin sırasıyla, hata veren kural nedeni ve iz kaydıyla).

## §38. Katlanır bölüm: `section` (`ADIM 22`, bileşen envanteri, düzen)

**15 Kasım.** Kod: `ui_panel.rs` (`Component::Section`, Kapı 1), ortak `kentos_ui::widget::Form::foldable_section`,
masaüstü `panel_sections.rs` (`drawn`, `section_of`, `reveal`), `panels.rs` (`shown`, `closed`, `PanelEvent::Section`),
`panel_view.rs`, `dialogs.rs`.

- **Karar: düz gövde, başlık bileşeni.** Panelin gövdesi tek liste kaldı: `section` bir başlıktır, ardından gelen
  bileşenleri bir sonraki bölüme dek toplar. İç içe bileşen listesi (grup içinde gövde) her denetimi, kuralı, oturumu ve
  çizimi iki katmanlı yapardı; başlık modeli hepsini olduğu gibi bırakıyor. Anahtarlar: `label` (zorunlu), `id`
  (isteğe bağlı; kod `set(id, visible=…)` ile gizler), `collapsed` (açılışta katlı), her bileşen gibi `visibleWhen`.
- **Katlamak bir görünüştür, kural değil:** katlı bölümün bileşenleri çizilmez ama gösterilmiş sayılır: değerleri komuta
  gider, zorunlu olanı yine sorulur ve yanlış bırakılan alan bölümünü açar (sözler alanın altında görülsün). Kuralı ya
  da kodu bölümü gizlerse topladıkları da gizlenir (sorulmaz). Satırların görünüşleri (tablo, kart…) formun altında
  çizilir; bölüm onları da katlar.
- **Ortak bileşende:** katlanır başlık `Form`'un kendi satırı (adı, açıkken aşağı katlıyken sağa bakan ok, satır sonuna
  çizgi; satırın tamamı basılır), panele özel çizim değil; başka formlar da kullanır. İletişim kutusunun alanları da
  alır.
- **Vitrin 1.0.31:** Yapı ara'nın sonunda kapalı "İpuçları". Gerçek pencerede (`36-bolum`): kapalı açıldı, başlığa
  basınca kısayollar ve geri alma notu göründü, yeniden basınca kapandı; bölüm sonda olduğu için başka senaryonun
  koordinatı kaymadı.
- **Sınamalar:** paylaşılan 1 (başlık, isteğe bağlı kimlik, kural; adsız ve `collapsed`'ı yanlış olanın yerleri),
  masaüstü 1 (katlıyken gösterilir ama çizilmez, zorunlu alan sorulur ve bölümünü açar, gizli bölüm topladıklarını
  gizler, kullanıcı katlar); Python cephesi `ui.section(label, id=…, collapsed=…)`.

## §39. Mimari olgular: katlar, mahaller, PAİY alanları ve kaçış yolları (`ADIM 20`, olguların ikinci dilimi)

**15 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/audit_facts/architecture.rs` (`ArchitectureFacts`,
`Storey`, `StoreyExits`, `RoomFacts`, `RoomEscape`, `EscapeWalk`, `UnitFacts`, `GroupKind`, `UnitLevel`,
`DoorEscape`; `ProjectAuditFacts::with_architecture`), İşlemler
`crates/native/processing/src/audit_facts/architecture.rs` (`measure`), `python/kentos/ext.py`
(`ProjectAuditFacts.architecture` ve sınıfları); örnek bina
`fixtures/audit-facts/v1/mimari.kcad` (KentOS'un komutlarıyla `scripts/fixtures/audit_facts_mimari.py` çizer) ve
olguları `mimari.json`.

- **Karar: sürüm 1'in isteğe bağlı bölümü, sürüm 2 değil.** §36 yazılı: okuyucu bilmediği alanı yok sayar, yeni olgu
  aynı sürümde gelir, anlam değişirse sürüm değişir. Burada anlam değişmedi, olgu eklendi: `architecture` bölümü mahali
  olan çizimde yazılır, olmayanda hiç yazılmaz. Bu yüzden §36'nın `denetim.json`'u baytı baytına aynı kaldı (sınaması
  değişmeden geçiyor) ve sürüm 1 okuyucusu (masaüstü, eski Python) bölümü görmeden çalışır.
- **Ne var:**
  - `levels`: projenin katları (kimlik, ad, kot, yükseklik), alttan üste; `exits`: mahali olan her katın kat çıkışı
    sayısı (BYKHY 4/u: kaçış merdiveninin mahali, açık merdivenin ayağı ya da başı, ±0.00 ve altındaki katta dışarı
    açılan kapı).
  - `rooms`: her mahal (boşluk ve dış mahal da): kalıcı kimlik, numara, ad, kullanım, kat, grup; `void`, `outdoor`
    (kullanımının programdaki tarifi dış mahal), `broken` (sınırı tohumundan bulunamadı), `stair` (kaçış merdiveni
    mahali); `net` (duvarların iç yüzleri arası, PAİY 4/ğ-3'ün ölçtüğü), `axis` (duvar eksenlerine göre çevre: Ek-5/A
    kişi yükünün çoğu kullanımda okuduğu); `escape`: en uzak noktasından (duvarlardan 40 cm, 32/6) en yakın kat
    çıkışına yürüyüş (uzunluk, başka çıkışa giden yolla ortak kalan tek yön payı, ulaşılan çıkış sayısı, kırılma
    noktaları; 32/4, 4/i), konut bölümünün mahalinde daire kapısına yürüyüş (48/2), katında çıkışa ulaşıp ulaşmadığı
    (33/6-b), kapı ve eşiksiz geçit sayısı (39/2).
  - `units`: programın mahali olan grupları PAİY 4/ğ alanlarıyla (BB brüt, net, eklentilerin brüt ve neti, toplam
    brüt, ortak alan payı, genel brüt, kat kat brüt ve net), türü, numarası, niteliği, eklentinin bölümü, konut olup
    olmadığı.
  - `doors`: her konut bağımsız bölümünün kapısından en yakın kat çıkışına yürüyüş (48/4).
- **Brüt alan mahal başına verilmez:** BB brüt alanı bölümün dış çevresidir, başka bölümle paylaşılan duvar ekseninden
  kesilir (4/ğ-1); mahal başına brüt paylaşılan duvarı iki kez sayardı. Mahal başına net ve eksen alanı, grup ve kat
  başına brüt ve net. Hesaplar KentOS Mimari'nin (`Document::paiy_areas`, `kentos_domain::escape`): metrekare cetveli
  ve Mimari denetim'in yangın kuralları neyi ölçüyorsa kural da onu okur.
- **Aynı baytlar her sırada:** kaçış yolları eşit yürüyüşlerden birini, PAİY toplamları da toplananları çizimin
  sırasıyla seçiyor. Ölçüldü: nesnelerin sırası ters çevrilince bir daire kapısının yolu aynı uzunlukta ama yinelenen
  bir noktayla çıktı. Ölçü bu yüzden nesneleri kalıcı kimlik sırasına dizilmiş bir kopyada okur (yalnız mahali olan
  çizimde); aynı nesneler ters ve karışık sırada aynı baytları verir (sınaması kopya kaldırılınca kırılıyor).
- **Sıra ve yuvarlama:** katlar kota, sonra kimliğe göre; kat çıkışları ve grubun katları o sırayla (katsız mahaller
  önce, projenin adlandırmadığı kat sonda); mahaller ve kapılar kalıcı kimliğe göre; gruplar programın sırasıyla;
  yürüyüşün noktaları yüründüğü gibi. Alan 0,0001 m², uzunluk, yer, kot ve yükseklik 1 mm. Liste başına en çok 10 000
  mahal ya da kapı; aşınca olgular `cut` der.
- **Yalnız mimarisi olan çizimde:** mahal yoksa bölüm yok; Python'da `facts.project.architecture` boş olgular verir
  (katlar, mahaller, gruplar, kapılar boş; `exits(kat)` 0). Bölümün ölçüsü bir planda kırılırsa bölüm yazılmaz,
  katmanların olguları kalır.
- **Maliyet (ölçüldü):** orta boy plan (`audit_facts_mimari.py --mid`: 8 kat, 152 mahal, 192 duvar, 144 kapı, 7
  merdiven; olgular 77,6 KB) release derlemede 76 ms (bütün olgular 78 ms), debug derlemede 482 ms; mahal başına
  yaklaşık 0,5 ms. **Karar: alana bağlanmadı**, mahali olan her çizimde ölçülür: kuralın süreci ve Python'u bundan
  uzun sürüyor, ölçü masaüstünde arka planda ve okuma kopyasında. Durdur adımlar arasında sorulur (mahaller, PAİY
  alanları, kaçış yolları); bir adımın içi durmaz. Binlerce mahalli bir planda (1 000 mahalde yarım saniye civarı
  beklenir, ölçülmedi) yalnız `domain: "mimari"` denetimlerde ölçmek bir bayrakla eklenir.
- **Python cephesi:** `facts.project.architecture` → `ext.ArchitectureFacts` (`levels`, `rooms`, `units`, `doors`;
  `exits(kat)`, `room(uid)`, `unit(kimlik_ya_da_numara)`); öğeler değiştirilemez veri sınıfları (`Storey`,
  `RoomFacts`, `RoomEscape`, `EscapeWalk`, `UnitFacts`, `UnitLevel`, `DoorEscape`). Stub, rehber ve `factsSchema`
  güncellendi. Örnek: yalnız olgularla yazılmış bir BYKHY 32/4 ve 4/i kuralı (konut olmayan mahalde tek yön payı 15
  m'yi aşarsa) örnek binada Mimari denetim'in yangın kurallarının bulduğu ikisini bulur: Arşiv 22,52 m, 1. katın
  koridoru 21,82 m çıkmaz.
- **Gösterim önerisi (Vitrin değil, mimari tarafta):** örnek bina gerçek pencerede açılır; bir mimari denetim
  eklentisinin iki kuralı şeritten çalışır: tek yön payı (yukarıdaki kural) ve PAİY metrekare cetveli raporu
  (`units`: her bölümün brüt, net, toplam ve genel brüt alanı). Göster bulgunun mahalini seçer; sonuçlar Mimari
  denetim'in kendi bulgularıyla yan yana karşılaştırılır.
- **Sınamalar:** paylaşılan 4 (yuvarlama ve sıra, katların sırası; sınır ve `cut`; bölümsüz olguların baytları ve
  geri okuma; şema), İşlemler 5 (örnek binanın katları, mahalleri, grupları ve yürüyüşleri; örneğin baytları; her
  sırada aynı baytlar; mahalsiz çizimde bölüm yok; Durdur), Python 6 (katlar ve çıkışlar, mahaller, gruplar, kapılar,
  olgularla yazılan kural, mahalsiz çizimde boş bölüm).

## §40. Görsel: `image` (`ADIM 22`, bileşen envanteri, görsel önizleme)

**15 Kasım.** Kod: `ui_panel.rs` (`Component::Image`, Kapı 1), masaüstü `panel_image.rs` (`decode`, `panel_image`),
`panel_session.rs` (`set(id, image=…)`), `panel_view.rs`; Python `PanelSession.set(..., image=…)`.

- **Karar: görseli kod verir, KentOS denetleyip çizer.** `image` bileşeni bir yer tutar (`label`, `alt`, `height`
  80–600 px, yoksa 200); görsel eklentinin kodundan gelir: `ctx.panel.set(id, image=…)` PNG baytlarını ya da bir PNG
  dosyasının yolunu alır, aracıdan base64 geçer; `None` görseli kaldırır. Python'da grafik kitaplığı serbest:
  matplotlib'in `savefig(format="png")` baytları da, NumPy dizisinden yazılan PNG de olur.
- **Yalnız PNG, okumadan önce denetlenir:** grafik ve çizim kayıpsız ister; boyutu başlığından (IHDR) okunur, kenarı
  4096 pikseli ya da toplamı 8 milyon pikseli aşan görsel çözülmeden reddedilir (düşmanca bir dosya sınırsız bellek
  isteyemez); metin en çok yaklaşık 6 MB. Yanlışı kodun sözleriyle (`invalid_input`) ve hiçbir şey değişmeden söylenir;
  önceki görsel kalır. Fotoğraf (JPEG) sonraki dilim.
- **Oranı korunur:** panel genişliğinde, oranının verdiği yükseklikte, en çok `height` kadar; üstünde ve altında bant
  kalmaz. Verilene dek `alt` görselin yerinde yazar. Tablo ve kartlar gibi formun altında çizilir.
- **Vitrin 1.0.33:** Yapı ara'nın Özetle'si gösterge kartlarının altında taban alanlarını büyükten küçüğe çubuk grafik
  olarak çizer: NumPy dizisine çizilip `zlib` ve `struct` ile PNG'ye çevrilir, ek kitaplık yok. Gerçek pencerede
  (`38-gorsel`): beş yapının çubukları (400 m² en büyüğü) "Taban alanları"nın altında.
- **Sınamalar:** paylaşılan 1 (alanlar, `alt` ve `height` sınırı yerinde), masaüstü 2 (PNG çizilir; başka biçim,
  base64 olmayan metin, sınırı aşan başlık çözülmeden ve bozuk PNG reddedilir; kod görseli verir, yanlışı öncekini
  bırakır, `None` kaldırır, başka bileşen almaz), Python 1 (bayt ve dosya base64 gider, `None` temizler).

## §41. Dosya seçici: `filePicker` (`ADIM 22`, bileşen envanteri, kaynak)

**15 Kasım.** Kod: `ui_panel.rs` (`Component::FilePicker`, `FileFilter`, Kapı 1), `archive.rs` (yetenek denetimi),
masaüstü `panel_files.rs` (`file_value`, `pick`, `picker_of`), `panels.rs`, `panel_view.rs`, `dialogs.rs`,
`panel_session.rs`; Python `ui.file_picker(...)`, `ui.file_filter(...)`.

- **Karar: okunacak dosya, kullanıcının verdiği.** `filePicker` bir panelde ya da iletişim kutusunda kullanıcıya
  dosya sorar: yolu yazılır ya da "Seç…" ile bilgisayarın dosya penceresinden seçilir (`filters`'ın türleriyle).
  Değeri dosyanın mutlak yoludur, verilmeyince None; komut çalışmadan önce dosyanın var olduğu ve sunulan türlerden
  olduğu denetlenir, yanlışı alanın adıyla söylenir. Okumak eklentinin işidir: manifest `files.userSelected` ister
  ("Sizin seçtiğiniz dosyaları okur"). Yazılacak dosya (kaydet penceresi) ve ek, yeteneğin sözü okumak olduğu için
  sonraki dilim.
- **Yetenek iki yerde:** paketin paneli dosya seçici taşıyıp manifest `files.userSelected`'ı istemiyorsa Kapı 1 reddeder
  (`ui_panel`); iletişim kutusunda kullanıcının kurarken vermediği yetenekle dosya seçici açılmaz.
- **Pencere kapanırsa bir şey olmaz:** "Seç…"in dosya penceresi kapatılınca hiçbir mesaj gelmez (`Task::and_then`);
  seçilen yol yazılmış gibi alana gelir (`onChange` çalışır).
- **Alanın sözü onunla gider:** düğmeye basınca yanlış alanın sözü hem altında hem formun altında yazar; alan
  düzeltilince formun altındaki aynı söz de kalkar (önce kalıyordu: yol yazıldığı hâlde "dosya verilmedi" duruyor ve
  pencerenin düğmesini kaydırıyordu; gerçek pencere).
- **Vitrin 1.0.34:** katmanın sağ tık menüsünün sonunda "CSV'den nokta ekle": pencere CSV dosyasını sorar, x ve y
  sütunlarındaki noktalar o katmana öznitelikleriyle eklenir (tek geri alma adımı). Gerçek pencerede (`39-dosya`):
  boşken Ekle "“CSV dosyası”: dosya verilmedi (yolunu yazın ya da Seç…'e basın).", `ornek-noktalar.csv` yazılınca
  "4 nokta eklendi.", Taslak katmanında 4 nesne. Yeni yetenek güncellemenin incelemesinden geçti.
- **Sınamalar:** paylaşılan (paket: dosya seçicili panel yeteneksiz reddedilir), masaüstü 2 (boş None, doğru tür
  mutlak yol, olmayan dosya ve başka tür adıyla söylenir, zorunlu sorulur ve yol komuta gider; formun altındaki söz
  alanla gider), Vitrin sınamalarının katkı listesi ve yeniden yükleme yeteneği güncellendi.

## §42. Bulut denetim işi: iş çalıştırıcısında aynı olgular, aynı kural, aynı rapor (`ADIM 20`, bulut işinin ilk dilimi)

**15 Kasım.** Kod: paylaşılan `crates/shared/contracts/src/gis/audit_job.rs` (`AuditRun`, `check_audit_run`,
`AuditReport`, `RuleRef`, `Finding`, `Fix`, `findings_of`, `counts`, `digest_of`; `RunRequest::Audit`), İşlemler
`crates/native/processing/src/audit_job.rs` (`facts_of`, `message`, `drawing_stamp`, `MOST_FEATURES`), iş
çalıştırıcısı `apps/jobrun/src/audit.rs` (`run_audit`, `RuleRunner`, `ExtHost`); sunucunun iki eşleşmesine yeni kolun
karşılığı (`jobs.rs`, `jobs_tests.rs`); `scripts/arch/deps.mjs` (`jobrun` → `python-env`).

- **Karar: en küçük dürüst dilim iş çalıştırıcısında biter.** Sunucunun iş kuyruğu, eklenti kataloğunun yayımlanmış
  sürümü ve sunucuda hazırlanan Python ortamı veritabanı ve kurulum işi; bu dilim onları tasarlar (aşağıda), çalışan
  payı `kentos-jobrun`'dadır: klasöre `request.json` (`{"kind": "audit", "input": AuditRun}`), `snapshot.kcad` ve
  `extension.kek` konunca kuralı uçtan uca çalıştırıp `answer.json`'a raporu yazar.
- **İstek (`AuditRun`):** eklentinin kimliği, sürümü ve paketin içerik özeti (`sha256:…`), kuralın yerel kimliği,
  sunucunun o sürüm için hazırladığı Python'un yorumlayıcısı (mutlak yol; yoksa kural çalışmaz), süre (varsayılan
  masaüstününki gibi 120 sn, en çok 3600). İstekte kullanıcının yazdığı bir yol yoktur: istek sunucunundur, kutu yine
  de her alanı denetler (`check_audit_run`).
- **Kutunun yaptıkları:** paketi paket denetleyicisiyle açar; içerik özeti, kimliği ya da sürümü istenen değilse hiçbir
  şey çalışmaz. Kuralı manifestin denetimlerinde bulur; `network` yeteneğini isteyen eklentinin kuralını reddeder
  (kutu ağ vermez); Python yoksa `tool_unavailable` der. Paketi klasöre açar, çizimi başsız oturumla açıp
  **masaüstünün koduyla** ölçer (`facts_of`: katmanların nesneleri en çok 50 000, ölçülmüş olgular en çok 200 000;
  ölçü kırılırsa kural katmanlarla çalışır, masaüstünde olduğu gibi), kuralı **masaüstünün protokolüyle**
  çalıştırır (`kentos.ext_host`, `kentos_python_env::process::Pool`; yorumlayıcı `-I` ile yalıtık, bayt kodu klasörde,
  süreç sunucunun boşalttığı ortamla) ve cevabı **masaüstünün okuduğu gibi** okur (`findings_of`: bilinen önem, söz
  2000, düzeltme başlığı 120 karakter, en çok 10 000 bulgu). Kural hata verirse iş `tool_failed` ile, kuralın sözü ve
  iz kaydının son 4000 karakteriyle biter.
- **Rapor ve köken (`AuditReport`):** kural (eklenti, sürüm, paket özeti, yerel kimlik, işleyici, başlık), çizimin
  içerik damgası (`drawing_stamp`: nesnelerin damgaları kalıcı kimlik sırasında, blok tanımları, katman ağacı,
  ayarlar, stiller, kısıtlar, ölçü takımları ve pafta kitabı; 16 hane, ADR 0253 §2'nin damgaları), kurala verilen
  olguların özeti (`sha256:` ve kanonik JSON'unun SHA-256'sı), olguları ölçen KentOS'un sürümü, bulgular, sayılmayan
  bulgular, olgular kesildi mi. Saat ve yer yok: aynı girdi aynı baytları verir. Planın `snapshot_etag`'i içerik
  damgasıdır: sunucunun revizyon numarası masaüstünde yok, içerik damgası her ikisinde aynı.
- **Kabul (ölçüldü):** `fixtures/audit-facts/v1/denetim.kcad` ve aynı kural (paketlenmiş bir eklentinin `@ext.audit`'i:
  parsellerin ölçülmüş çakışması ve boşluğu, ada numarası olmayan nesneler, düzeltmesiyle) gerçek Python'da:
  masaüstünün yolu (anlık görüntüden açılan belge, kurulu paketin klasörü, aynı protokol) ile işin yolu (sunucunun
  `.kcad`'i, isteğin yanındaki paket) baytı baytına aynı raporu verdi (1 hata, 1 uyarı, 9 bilgi; çakışma "50.00 m²"
  ölçülmüş olgulardan); iş iki kez aynı. Paketi değiştirilmiş kural `KeyError` ile ve iz kaydıyla düştü.
- **Kabulün eksik payı:** "bulut kutusu ağa ve yerel dosyalara erişemez": KentOS tarafında ağ isteyen kural
  reddedilir, kutu kurala dosya vermez, süreç boş ortamla ve yalıtık başlar; ama Python bir işletim sistemi süreci,
  ağı ve dosyaları işletim sistemi düzeyinde kapatılmadı (ağ ad alanı, salt okunur kök, sistem çağrısı süzgeci): bu
  sunucu dağıtımının işi ve sonraki dilim. Python süreci `kentos-jobrun`'ın çocuğu: sunucunun işlemci ve bellek bütçesi
  yalnız `kentos-jobrun`'ı ölçüyor; bütçe süreç grubunu (cgroup) ölçmeli.
- **Sunucunun payı (tasarım, sonraki dilim):** `project.audit` iş türü: istek eklentinin kimliği, kuralı ve isteğe
  bağlı sürümü; sunucu yayımlanmış sürümü katalogdan alır (ADIM 22A, şimdilik bellek deposu), paketi nesne deposundan
  klasöre koyar, sürümün hazırlanmış ortamını bulur (yayımlanınca kilitten bir kez, ağlı bir adımda kurulur; denetim
  kutusu onu ağsız kullanır) ve `RunRequest::Audit` yazar. Sonuç projeye yazılmaz, işin sonucu olarak saklanır
  (`AuditReport`); önbellek anahtarı kural özeti, çizim damgası ve KentOS sürümü. Kayıt, onay noktası ya da istemci
  isteğinde çalıştırma ve "geçen/kalan kurallar", "delta" bu iş türünün üstüne gelir.
- **Masaüstü aynı işlevlerde (bu dilimde):** Eklenti denetimi ve deneme koşusu (§37) olguları `audit_job::facts_of`
  ile kurar ve `message` ile gönderir, kuralın cevabını `findings_of` ile okur, özeti `counts` ile söyler; önem
  paylaşılan `Severity`'dir (`code`: kuralın ve ajanın yazdığı, `word`: kullanıcının okuduğu). Katmanların olguları
  artık arayüz iş parçacığında değil, ölçülmüş olgularla birlikte okuma kopyasında koşunun iş parçacığında kurulur.
  `audit.report` `drawing` (içerik damgası) ve `facts` (olguların özeti) taşır: aynı çizim ve kural için masaüstünün
  raporu ile işin raporu aynı damgaları verir; masaüstünün yolu bu sınamadaki yolun ta kendisidir.
- **Kökenin bedeli (ölçüldü, hata ayıklama derlemesi, 50 000 parsel, `provenance_cost`):** olgular 4,96 sn, kurala
  giden 18 MB'ın yazılması 1,35 sn; içerik damgası 1,12 sn, olguların özeti 0,83 sn. Köken, kural başlamadan önceki
  süreye yaklaşık %30 ekler; ikisi de koşunun iş parçacığında, arayüz beklemez. Yayın derlemesi bunların hepsini
  birkaç kat kısaltır; büyük çizimde gerekirse damga ve özet kuralla eşzamanlı hesaplanır (kuralın süresi altında
  kalır).
- **Sınamalar:** paylaşılan 5 (istek biçimi, alan alan denetim, cevabın masaüstündeki gibi okunması, sınır, özetin
  kanonikliği, önemin kodu ve sözü), İşlemler 2 ve elle çalıştırılan ölçüm (olgular masaüstününkiler, çizim damgası
  içerikten), iş çalıştırıcısı 4 (kural Rust yerine geçenle uçtan uca, reddedilenler, masaüstü ve iş aynı rapor gerçek
  Python'da, düşen kural iz kaydıyla), masaüstü (Vitrin'in denetimi: raporun `drawing` ve `facts`'i ortak işlevlerin
  verdiği). Python yoksa (`KENTOS_TEST_PYTHON` ya da `.run/py`) iki sınama atlandığını söyler.

## §43. Panel içi sekme: `tab` (`ADIM 22`, bileşen envanteri, düzen)

**15 Kasım.** Kod: paylaşılan `ui_panel/tabs.rs` (`TAB_KEY`, `MOST_TABS`, `tab_of`, Kapı 1), `Rules::tab`, panelin
JSON Schema'sı ve `kentos.ui_panel`'in işlevleri (`tab=`); masaüstü `panel_tabs.rs` (`tabs`, `active_tab`, `on_page`,
`show_tab`, `tab_bar`), `panel_view.rs` (şerit), `panel_sections.rs` (bölüm ve yanlış alan), `panels.rs`/`dialogs.rs`
(`PanelEvent::Tab`).

- **Karar: sekme bir bileşen değil, bileşenin anahtarı.** Panelin gövdesi düz bir liste; kurallar, hesaplar, oturum ve
  doğrulama kimlikle ve sırayla çalışır. İç içe sayfalar bunların hepsini değiştirirdi. Her bileşen isteğe bağlı
  `"tab": "Seçenekler"` taşır: aynı sözü taşıyanlar aynı sayfadadır, şerit sekmeleri gövdede ilk göründükleri sırayla
  dizer ve ilk sekmeli bileşenin yerinde durur. **Sekmesiz bileşen her sayfada**, kendi yerinde çizilir: gövdenin
  sonundaki düğme sayfaların altında kalır, bitiş işareti yazılmaz. Panelde de, `ctx.dialog`'un penceresinde de aynı.
- **Görünüş, kural değil:** başka sayfadaki alan çizilmez ama gösterilir: değeri komuta gider, zorunluysa sorulur,
  kurallar okur (katlanır bölümün yolu, §38). Yanlış bırakılan alan kendi sayfasını açar (bölümünü de); aynı yol
  `ctx.panel.focus`'ta. Bütün bileşenleri gizli sekme (kuralı ya da kodu) şeritte yoktur; seçili sekme böyle gizlenirse
  ilk sekme açılır. Seçim panel açık kaldıkça kullanıcınındır.
- **Bölümle birlikte:** bölüm sonraki bileşenleri bir sonraki bölüme ya da **başka sekmedeki ilk bileşene** dek toplar.
  Sekmeli bölümün ardından gelen sekmesiz düğme her sayfada kalır, bölümle katlanmaz; ayrı bir kural yazılmadı.
- **Kapı 1:** sekme boş olmayan, tek satır, en çok 40 karakterlik bir söz; panelde en çok 8 sekme. Her bileşen alır.
- **Şerit KentOS'un sekme şeridi** (`widget::tabs`): sığmayınca kayar ve bütün sekmelerin listesi açılır; panelde
  sekmeler kapatılmaz, sürüklenmez.
- **Vitrin 1.0.35:** "CSV'den nokta ekle" penceresi Dosya (dosya seçici) ve Seçenekler (ayırıcı: virgül, noktalı virgül,
  sekme; diğer sütunlar öznitelik olsun mu) sekmeleriyle; açıklama sekmesiz (`40-sekme`).
- **Sınamalar:** paylaşılan 1 (sekme kırpılarak okunur; söz olmayan ve dokuzuncu sekme yerinde söylenir), Python
  işlevlerinin imzası (`tab`), masaüstü 1 (sıra, gizli sekme listede yok, ilk sekme, sekmesiz bileşen her sayfada,
  başka sayfadaki zorunlu alan sorulur ve sayfası açılır, bölüm başka sekmede durur, gizlenen seçili sekmenin yerine
  ilki).

## §44. Kaydetme yeri: `filePicker`'ın `save` kipi ve `files.userSelectedWrite` (`ADIM 22`, kaynak ve çıktı)

**15 Kasım.** Kod: paylaşılan `ui_panel.rs` (`FilePicker::save`, `default_name`), `ui_panel/files.rs` (dosya
seçicinin Kapı 1 denetimi, `ui_panel.rs`'ten taşındı), `manifest.rs` (`Capability::FilesUserSelectedWrite`),
`archive.rs` (paketteki panelin yetenek denetimi); masaüstü `panel_files.rs` (`Picker`, `file_value`'nun kaydetme
kolu, `pick` kaydetme penceresiyle), `dialogs.rs` (`ctx.dialog`'da verilen izin), `panel_view.rs` (ipucu).

- **Karar: okumak ile yazmak ayrı yetenek.** `files.userSelected` kurulum penceresinde "Sizin seçtiğiniz dosyaları
  okur" der; anlamını yazmaya genişletmek, okumaya izin vermiş kullanıcıya sormadan yazma izni vermek olurdu. Yeni
  yetenek `files.userSelectedWrite`: "Sizin seçtiğiniz yere dosya yazar", yalnız `user-selected` kapsamı. Kaydetme
  kipindeki seçici onu ister: paketin panelinde Kapı 1, `ctx.dialog`'un penceresinde masaüstü kullanıcının verdiği
  izne bakar. Var olan kurulum güncellenirken yeni yetenek kurulum incelemesinde "eklendi" diye sorulur.
- **Değer:** yazılacak dosyanın mutlak yolu. Dosyanın var olması gerekmez, klasörünün gerekir. Uzantısız yazılan ada
  süzgecin ilk uzantısı eklenir (bilgisayarın kaydetme penceresi her sistemde eklemiyor); başka uzantı süzgecin
  sözüyle, klasör yolu "dosyanın adı da yazılmalı" diye söylenir. Var olan dosyanın üzerine yazmayı kaydetme penceresi
  sorar; elle yazılan yol kullanıcının kararıdır.
- **`defaultName`:** kaydetme penceresinin önerdiği ad (ör. `parseller.csv`); yalnız `save: true` ile, klasörsüz,
  en çok 120 karakter (Kapı 1). Okunacak dosyanın önerilen adı olmaz.
- **Yazmak eklentinin:** KentOS dosyayı açmaz, yazmaz; yolu kodun parametrelerine verir. Aracı dosya erişimini
  denetlemez (ayrı süreç korumalı alan değildir, PY-RUN-19): yetenek kullanıcıya söylenen sözdür.
- **Vitrin 1.0.36:** katmanın sağ tık menüsünde "Katmanı CSV'ye yaz": önerilen ad katmanın adı; her nesne bir satır
  (x, y: noktanın kendisi, alanın ağırlık merkezi, çizginin ilk noktası; öznitelikler sütun). `41-kaydet`.
- **Sınamalar:** paylaşılan (Kapı 1: önerilen ad dosya adı ve yalnız kaydetmede, `save` evet/hayır, türün uzantısı;
  paket: kaydetme yeri yazma yeteneği ister), masaüstü 1 (uzantısız ada ilk uzantı, dosya olmasa da olur, klasörü
  yoksa, başka tür ve klasör söylenir).

## §45. Koordinat sistemi seçici: `crsPicker` (`ADIM 22`, bileşen envanteri, CBS girdisi)

**15 Kasım.** Kod: paylaşılan `ui_panel.rs` (`Component::CrsPicker`), `ui_panel/crs.rs` (`codes`, `code_of`, Kapı 1;
kayıt `fixtures/crs/v1/registry.json`), `rules.rs` (metin girdisi); masaüstü `panel_crs.rs` (`default_crs`,
`crs_select`), `panels.rs`, `dialogs.rs`, `panel_view.rs`, `panel_session.rs` (`ctx.panel.set(id, value=…)`).

- **Değer `EPSG:<kod>`:** eklentinin kodu onu olduğu gibi okur (`pyproj.CRS(value)` de okur); KentOS'un
  dönüşümüne kodu verir (`cad.crs.transform(..., from_=cad.CrsSide(srid=kod))`). Seçenekler KentOS'un kayıt
  defterindeki sistemlerdir (web ve masaüstü aynı listeyi okur), datumlarına göre sıralı, her biri datumuyla; **yerel
  sistem (0) seçenek değildir**: seçici bir sistem sorar, sistemsizliği değil.
- **Varsayılan çizimin sistemi:** `value` verilmezse seçici açık çizimin kendi sistemiyle başlar (panelde de
  `ctx.dialog`'da da); yerel sistemli çizimde boş kalır, zorunluysa "seçilmedi" diye sorulur. Eklentilerin çoğu "çizimin
  sisteminde mi, başka birinde mi?" diye sorar.
- **Kapı 1:** `value` KentOS'un tanıdığı bir sistemin `EPSG:<kod>`'u olmalı; kod kayıt defterinden okunur (paylaşılan
  crate kayıt dosyasını içine alır, yerel çekirdeğe bağlanmaz). `ctx.panel.set(id, value=…)` aynı denetimden geçer.
- **Pencerede de:** `ctx.dialog`'un alabildiği alanlara eklendi (14 tür). Gerçek pencere sınaması yakaladı: liste
  eksikken Vitrin'in penceresi "“crsPicker” pencerede olamaz" diye reddediliyordu; SDK rehberinin pencere alanları da
  eskimişti (dosya seçici ve bölüm yoktu), düzeltildi.
- **Sınır:** birim seçici yok; sistem listesi süzülmez (bir eklentinin yalnız TM dilimlerini sunması sonraki dilim).
- **Vitrin 1.0.37:** "CSV'den nokta ekle"nin Seçenekler sekmesinde dosyanın koordinat sistemi; seçilen sistemden
  çizimin sistemine `cad.crs.transform`. `ornek-noktalar-turef.csv` aynı dört noktayı TUREF coğrafi koordinatlarla
  (EPSG:5252, boylam ve enlem; Krüger serisiyle hesaplandı, ileri-geri farkı 1 nm altı) taşır: TUREF seçilince noktalar
  TM36 dosyasındakilerin tam üstüne düşer (`42-crs`).
- **Sınamalar:** paylaşılan 1 (kayıt yerel sistemsiz, değer `EPSG:<kod>`, bilinmeyen kod ve kodsuz yazım reddedilir),
  masaüstü 1 (değersiz seçici çizimin sistemiyle başlar, kendi değeri kalır, yerel çizimde zorunlu sorulur, değer
  komuta yazıldığı gibi gider).

## §46. Eklentinin komutundan KentOS'un yerleşik aracı: `ctx.run_tool` (`ADIM 19`, PY-RUN-14, PY-TOOL-18'in komut payı)

**15 Kasım.** Kod: Python `kentos/ext.py` (`CommandContext.run_tool`, `layer_input`, `new_layer`; stub yeniden
üretildi), SDK rehberi; masaüstü sınaması `tests/demo.rs`; Vitrin 1.0.38 "Yapı tamponu".

- **Bulgu: yol vardı, adı yoktu.** Eklentinin komutu aracı üzerinden her KentOS komutunu çalıştırabiliyordu;
  `cad.processing.run` da onlardan biri. Yani yerleşik İşlemler aracı (Tampon, Kesiştir, alan hesabı …) eklentiden
  zaten çağrılabiliyordu, ama SDK söylemiyor, Vitrin göstermiyordu; plan da bu yüzden onu "kalan" sanıyordu.
- **Karar: ayrı bir mekanizma değil, adlı bir yardımcı.** `ctx.run_tool(araç, **parametreler)` `cad.processing.run`'ı
  komutun çizimiyle çağırır; `ext.layer_input("yapi")` katmanın nesnelerini (`{"scope": "layer", "layerId": …}`,
  isteğe bağlı türler), `ext.new_layer("Ad")` yeni katmanı (`{"newName": …}`) yazar. Cevap aracın cevabıdır (`summary`,
  `outputs`, `record`); araçların listesi ve parametreleri `cad.processing.tools(ctx.doc)`.
- **Kapsamın devri (PY-RUN-14):** araç komutun hakkıyla çalışır: yazan araç `document.write` ister, yoksa aracı
  `capability` ile reddeder ve hiçbir şey yazılmaz; yazdıkları komutun tek geri alma adımına girer (adımı komutun
  adını taşır); yalnız komutun başladığı çizime. Başka bir eklentinin aracı yalnız bağımlılıkla (§16). İç içe çağrı
  izni genişletemez: araç komutun yapamadığını yapamaz.
- **Kalan (PY-TOOL-18'in araç payı):** eklentinin İşlemler aracı (ayrı süreçte, aracıya istek yapamayan) yerleşik bir
  aracı çağıramaz; onun için çizime dokunmayan, verilen nesnelerle çalışan bir yol gerekir (sonraki dilim).
- **Vitrin 1.0.38:** Vitrin › Analiz › Yapı tamponu: uzaklığı sorar (3 m), KentOS'un Tampon aracı beş yapının
  tamponunu "Yapı tamponu 3 m" katmanına çizer (`43-arac`).
- **Sınamalar:** masaüstü 1 (yazabilen koşuda aracın tamponları yeni katmanda, beş yapıya beş; yalnız okuyabilen koşuda
  araç `document.write` sözüyle reddedilir, katman yok).

## §47. Eklentinin İşlemler aracından KentOS'un yerleşik aracı: kendi verisiyle (`ADIM 19`, PY-TOOL-18)

**15 Kasım.** Kod: süreç havuzu `kentos_python_env::process` (`Pool::run_answering`, `Answer`, `refuse_calls`; `run`
eskisi gibi reddeder), İşlemler `external.rs` (aracın girdisinde çizimin `srid`'i), masaüstü `tool_calls.rs`
(`tool_call`, `made_objects`), `tools.rs` (aracın koşusu çağrıları yanıtlar), `trial/drawing.rs` (`synthetic_on`);
Python `ProcessingContext.run_tool`, `ToolAnswer`, `_tool_value`; Vitrin 1.0.39 "Çekme mesafesi denetimi".

- **Sorun:** eklentinin İşlemler aracı ayrı süreçte, girdileri veri olarak alarak çalışır (PY-TOOL-09) ve aracıya
  istek yapamaz; havuz her çağrısını "çizime erişemez" diye reddediyordu. Bu yüzden bir araç KentOS'un Tampon'unu,
  Kesiştir'ini kendi hesabının içinde kullanamıyordu; algoritmaların birbirini çağırması eklenti ekosistemlerinin
  temel kalıbı.
- **Karar: veri girer, veri çıkar.** Araç `ctx.run_tool("geometry.buffer", input=<elindeki nesneler>, distance=3)`
  der; masaüstü bu nesnelerden kullanıcının çizimine dokunmayan bir çizim kurar (deneme koşusunun sentetik çizimi,
  §37; kullanıcının çiziminin koordinat sisteminde), yerleşik aracı onda çalıştırır ve yaptığı nesneleri aracın
  girdisinin biçiminde (`uid`, `layer`, `entity`, `geometry`) geri verir. Araç yalnız elindekini verebilir: verilmeyen
  bir katmana ya da gizli bir alana bu yoldan ulaşamaz. Sonucu araç kendi çıktısında döndürür (`NewFeature`).
- **Yalnız yerleşik araçlar:** başka bir eklentinin aracı bir aracın içinden çalışmaz (süreç zinciri yok, bağımlılık
  denetimi delinmez); aracın kendi kodu kendi işlevlerini çağırır. Başka bir yöntem `unknown_method`.
- **Sınır:** bir çağrıda en çok 50 000 nesne girer ve çıkar; boş girdi aracı çalıştırmaz ("Girdi boş"). Çağrı
  sürerken aracın Durdur'u ve süresi bir sonraki iletide sorulur (çağrı uzunsa onun bitişini bekler).
- **Havuzun yolu:** `Pool::run_answering` koşunun çağrılarını sahibinin `Answer`'ına verir; denetim kuralları ve
  bulut işi eskisi gibi `run` ile, çağrıları reddederek çalışır.
- **Vitrin 1.0.39:** "Çekme mesafesi denetimi" (Vitrin › Analiz): her yapının tamponu KentOS'un Tampon aracıyla, yapı
  tamponuna sırasıyla bağlanır (öznitelik "sira", araç öznitelikleri taşır); tamponun yapının parselinden taşan alanı
  shapely ile; taşanlar seçili, tamponlar yeni katmanda (`44-cekme`).
- **Sınamalar:** süreç havuzu 1 (sahibi yanıtlayan koşuda çağrı yöntemi ve parametreleriyle ulaşır, cevabı koda
  döner; yanıtlamayan koşu eskisi gibi reddeder), masaüstü 1 (yerleşik Tampon verilen iki karenin iki tamponunu
  yeni katmanda verir; eklenti aracı, başka yöntem ve boş girdi söylenir).

## §48. Aracın örnekleri: tanımda sentetik veri ve doğruluk sınaması (`ADIM 19`, PY-TOOL-23; `ADIM 21`, Kapı 3)

**15 Kasım.** Kod: İşlemler `external/examples.rs` (`examples_of`, `MOST_EXAMPLES`, `MOST_EXAMPLE_FEATURES`),
`ExternalTool::examples`, araç tanımının JSON Schema'sı; masaüstü `examples.rs` (`with_examples`, `ext_examples_start`,
`run`, `one`, `check`, `ext_examples_said`), `trial.rs` (`TrialState::by_user`, `examples` kipi), Eklentiler penceresi
(`Event::RunExamples`, "Örnekleri çalıştır"); başsız `extension.workshop.trial` (`examples`); Vitrin 1.0.40.

- **Karar: örnek aracın tanımında, beklenenle birlikte.** `examples` (en çok 10): her biri bir ad, sentetik nesneler
  (katman adı, GeoJSON şekli, öznitelikler; en çok 500), aracın parametreleri (nesne parametresi örneğin katmanını
  adıyla anar; verilmeyen parametre aracın varsayılanıyla) ve beklenen (`expect`): sayı ya da metin çıktısının değeri,
  nesne çıktısının `count`'u ve `has` (en az bir yazılan nesnenin taşıması gereken öznitelik kümeleri), özette geçmesi
  gereken söz (`summary`). Biçimi araç tanımı okunurken denetlenir (yeri söylenir), JSON Schema'da kapalı.
- **Koşu deneme koşusunun yolu (§37):** her örnek kendi sentetik çiziminde, kullanıcının çizimine dokunmadan; araç
  İşlemler'in çalıştırdığı gibi (eklentinin süreci, tipli sonuç). Karşılaştırma: sayılar sayı olarak (metin ve ondalık
  virgül de; milyarda bir pay), metin yazıldığı gibi; nesne çıktısının katmanı çıkış parametresinden (örneğin verdiği ya
  da aracın varsayılanı: yeni katmanın adı ya da katmanın kimliği). Durdur örnekler arasında.
- **İki yüz:** kullanıcı Eklentiler › eklenti › "Örnekleri çalıştır" (araçlarından biri örnek taşıyorsa; koşarken
  "Örnekler çalışıyor…"): sonuç bildirimde ("2 örneğin hepsi geçti", "3 örnekten 2 tanesi geçti, 1 tanesi kaldı";
  sayıdan sonra ek gerektirmeyen söz, "2'si", "3'ü" sayıya göre değişir), kalan her örnek eklentinin günlüğünde
  nedeniyle ve eklenti sorunu olarak; hepsi geçince sorun kalkar. Ajan `extension.workshop.trial` ile `examples: true` (nesne
  gerekmez): rapor örnek başına araç, ad, geçti/kaldı, nedenler, özet ve süre. Aynı anda bir deneme ya da örnek koşusu.
- **Neden:** yapay zekânın ürettiği eklentiyi doğrulamanın en ucuz yolu aracın kendi söylediği örnekler; inceleyici ve
  kullanıcı da aynı örneği görür (PY-AI-04: üretilen Python ayrı bir test koşusuyla doğrulanır).
- **Vitrin 1.0.40:** TAKS'ta biri aşan iki parsel (aşan 1; 0,6 aşıyor, 0,08 uygun; özet "1 parsel"), Çekme
  mesafesinde kenara yakın ve ortadaki yapı (taşan 1; Y-01 taşıyor, Y-02 uygun). `45-ornekler`.
- **Derin yığın:** örnek ve deneme koşuları 64 MB yığınlı bir iş parçacığında (`in_background_deep`). Örnek aracı
  komut zarfıyla çalıştırır, araç da içinden yerleşik aracı aynı yolla çağırır (§47): iki işleme koşusu tek iş
  parçacığında iç içe; varsayılan 2 MB hata ayıklama derlemesinde taştı (gerçek pencere sınaması yakaladı: "stack
  overflow"; İşlemler penceresinden aynı araç sorunsuz çalışıyordu).
- **Sınamalar:** İşlemler 1 (örnek okunur; adsız, şekilsiz ve on birinci örnek yeriyle söylenir) ve şemanın sapması,
  masaüstü 1 (sayı ve metin karşılaştırması, öznitelik kümesi, sonucun sözü), başsız 1 (örnek kipi nesnesiz
  masaüstüne gider); gerçek pencerede Vitrin'in iki örneği geçti.

## §49. Aracın kendi girdisini yerinde değiştirmesi: `writes: true` ve `ctx.update` (`ADIM 19`, PY-TOOL-12)

**15 Kasım.** Kod: İşlemler `external/updates.rs` (`updates_of`), `external.rs` (`result_of` değişikliği ekler);
Python `ProcessingContext.update`, `ext_host.outputs_of` (`update`); Vitrin 1.0.41.

- **Sorun:** eklentinin aracı yalnız yeni nesne (çıktı katmanına), tablo ve sayı verebiliyordu; girdisindeki nesneye
  bir sonuç yazmak (parselin TAKS'ı, yapının çekme durumu) için kopyasını yeni katmana yazmak gerekiyordu.
- **Karar: kaynağı değiştirmek aracın tanımında söylenir.** Yalnız tanımında `"writes": true` bildiren nesne
  parametresinin nesneleri değiştirilebilir (parametre penceresi ve ajan aracın girdisini değiştirdiğini tanımdan
  görür); kod `ctx.update(nesne, öznitelik=değer)` der, `None` özniteliği kaldırır, sayı ve evet/hayır metin olarak
  saklanır. Yanıtta `update: [{uid, attrs}]`.
- **Denetim yeni nesneninkiyle aynı:** değerler katmanın alanlarına göre denetlenir; araca verilmemiş nesne ("araca
  verilen nesnelerden biri değil"), yazmayan girdiden gelen nesne ("verildiği girdinin tanımında writes: true yok"),
  aynı nesnenin iki kez değişmesi ve sözlük olmayan öznitelikler reddedilir; bir değişiklik reddedilirse hiçbir şey
  yazılmaz. Değişiklikler yeni nesnelerle aynı, tek geri alma adımında.
- **Dosya hedefi zaten atomikti (PY-TOOL-13):** yeni dosya geçici dosyaya yazılıp doğrulandıktan sonra yeniden
  adlandırılıyor, iptal ve hatada siliniyor; var olan dosyaya yazma tek işlemde (`command.rs` `write_sink`). Plan
  notu bunu "kalan" sanıyordu; düzeltildi.
- **Vitrin 1.0.41:** Çekme mesafesi denetimi yapıların girdisini `writes: true` bildirir ve her yapının `cekme`
  özniteliğine "uygun" ya da "taşıyor" yazar (`46-yerinde`).
- **Sınamalar:** İşlemler 1 (yazan girdide öznitelik yazılır, `None` kaldırır, sayı metin olur, tek geri alma
  adımı; yazmayan girdide aynı değişiklik reddedilir, hiçbir şey yazılmaz) ve yanıtın denetimine iki durum (verilmeyen
  nesne, liste olmayan `update`).

## §50. Sunucunun denetim işi: `project.audit` (`ADIM 20`, bulut işinin ikinci dilimi)

**15 Kasım.** Kod: paylaşılan `crates/shared/contracts/src/gis/project_audit.rs` (`ProjectAuditInput`,
`check_project_audit_input`, `ProjectAuditResult`) ve `core/server_jobs.rs` (`PROJECT_AUDIT_JOB`, `SERVER_JOB_KINDS`);
sunucu uygulaması `crates/server/application/src/audit_catalog.rs` (`AuditSource`, `AuditPackage`, `AuditPythons`),
`jobs/audit.rs` (`checked`, `external`, `before_run`, `result`), `job_cache.rs` (`audit_key`), `processing_job.rs`
(`ProcessingTask.package`, `Processor::audits`) ve `jobs.rs` (yeni türün yolları); `kentosd`
`apps/api/src/extension_catalog/audits.rs` (`Catalog::runnable`, `CatalogAudits`) ve `processing_job.rs` (paket
klasöre, `with_audits`); göç `0041_server_job_audit.sql` (türün kısıtı); sınamaların ortak payı
`apps/api/tests/support/mod.rs` (`child_exe`, aşağıda).

- **Karar: denetim yeni bir iş türüdür, yeni bir yol değil.** İstek var olan iş yolundan gelir (`POST
  /v1/tenants/{t}/projects/{p}/jobs`, `{"kind": "project.audit", "input": …, "requestId": …}`). Kuyruk, kurum ve
  kullanıcı sınırları, iptal, süre, yeniden deneme, ilerleme, bitişte erişimin ve görünürlüğün yeniden sorulması ve
  önbellek o işlerinkidir; yeni HTTP yolu yok. İşi yalnız işlemcisi olan işçi alır (araç işleri gibi).
- **Girdi (`ProjectAuditInput`):** `extension`, `rule`, isteğe bağlı `version` (yoksa en yeni yayımlanmış sürüm) ve
  `expectedRevision` (proje ilerlemişse `revision_conflict`). Bilinmeyen alan reddedilir: yorumlayıcı isteğin değil
  sunucunun ayarıdır, istek bir yol taşıyamaz. Dosya olarak saklanan projede iş açılmaz: sunucuda çizim yoktur,
  kullanıcıya denetimi masaüstünde çalıştırması söylenir.
- **Sürüm katalogdan (`AuditSource` → `CatalogAudits`):** önce kurumun kataloğu (`tenant:{kısa ad}`), sonra genel
  katalog. Verilen sürüm çalışabilir olmalı (`allows_run`: yayımlanmış ya da yayımlandıktan sonra emekliye ayrılmış);
  sürüm verilmezse en yeni yayımlanmış sürüm alınır, askıya alınan atlanır ve ondan önceki yayımlanmış sürüm çalışır.
  Taslak, incelemedeki ve askıdaki sürüm çalışmaz; sebebi işin hatasıdır (`invalid`). Paketin baytları depodan okunur
  ve kurulumdaki gibi onaylı içerikle karşılaştırılır: tutmazsa iş çalışmaz.
- **Karar: eklentiyi tanıyan kurum kataloğu tek başına karar verir.** Katalog bir eklentiyi, onun herkesin görebildiği
  bir sürümü varsa tanır (yayımlanmış, askıya alınmış, geri çekilmiş ya da yayımlandıktan sonra emekliye ayrılmış;
  rolü ve yayımcısı olmayan hesabın gördüğü, `visible_to`). Kurumun kataloğu eklentiyi tanıyorsa ama çalışabilir sürümü
  yoksa iş düşer, genel kataloğa geçilmez ("… genel katalogdaki aynı kimliğe bakılmadı"): ad alanı doğrulanana dek
  (ADR 8059 D13) iki katalogdaki aynı kimlik iki ayrı yayımcının olabilir ve kurumun askıya aldığı sürümün yerine
  sessizce başka bir yayımcının paketi çalışmamalı. Kurum kataloğundaki taslak hiçbir şeyi sahiplenmez ve varlığı da
  söylenmez: yayımlanana dek genel katalogdaki sürüm çalışır.
- **Yorumlayıcı (`AuditPythons`):** sunucu ayarı, `{"eklenti@sürüm": "/mutlak/yol/bin/python"}` biçiminde bir JSON
  nesnesi; anahtar `eklenti@sürüm` değilse ya da yol mutlak değilse ayar reddedilir. Sürümün hazırlanmış ortamı yoksa
  iş, kural başlamadan `tool_unavailable` ile biter ("… için hazırlanmış bir Python ortamı yok"); kural başka bir
  Python'la çalıştırılmaz.
- **Çalışma:** işçi paketi işin klasörüne isteğin yanına koyar (`extension.kek`) ve `RunRequest::Audit` yazar; gerisi
  §42'nin kutusudur (içerik özeti, kimlik ve sürüm denetimi, `network` isteyenin reddi, masaüstünün olguları ve
  protokolü). Bütçe araç işlerininki (sunucunun varsayılan işlemci ve bellek sınırları); projeye bir şey yazılmaz,
  bu yüzden görünürlükle kesilmiş kopya da denetlenebilir (kural kullanıcının görebildiğini okur).
- **Sonuç (`ProjectAuditResult`):** sürümün geldiği katalog, okunan veri revizyonu, özet ("Adsız nesneler: 1 hata, 9
  bilgi"), kutunun raporu (`AuditReport`; köken §42'deki gibi: kural ve paket özeti, çizimin içerik damgası, olguların
  özeti, KentOS sürümü) ve kutunun harcadığı. Eser (artifact) yok. Kutunun raporu istenen sürümün (paket özeti) ya da
  kuralın değilse saklanmaz, iş düşer.
- **Önbellek (tasarımdan ayrılış):** §42 anahtarı "kural özeti, çizim damgası ve KentOS sürümü" diye tasarlamıştı;
  çizim damgası ancak kutu çizimi açınca bilinir, anahtar ise kutu başlamadan gerekir. Anahtar (`audit_key`): eklenti,
  sürüm, paket özeti ve kural; projenin veri revizyonu ve olay imleci (her yazım imleci ilerletir: aynı durum aynı
  anahtar), görünürlük damgası ve sunucunun yazılım sürümü. Kullanıcı ve gün anahtarda değil: kural çizimin olgularını
  okur, ikisini de görmez; yeni sürüm yeni paket özeti demektir. Aynı anahtarla daha önce başarılmış bir iş varsa yeni
  iş kutuyu çalıştırmadan onun raporunu alır (`cachedFrom`); `fresh` istenen iş yeniden çalışır. Çizim damgası
  raporda köken olarak kalır. Önbelleğe bakış ve yorumlayıcı denetimi proje okunduktan hemen sonra, bellek tahmininden
  önce yapılır: önbellekten dönecek iş bütçeye takılmaz.
- **Kabul (ölçüldü):** `kentosd`'nin gerçek işlemcisi ve `kentos-jobrun` ile, paketlenmiş bir eklentinin kuralı (ada
  numarası olmayan nesneler) sunucunun parsel projesinde gerçek Python'da çalıştı; rapor istenen sürümün ve kuralın,
  çizim damgası 16 hane, iki koşu baytı baytına aynı (hata ayıklama derlemesi: iki koşu 4,1 sn; her biri çocuk süreç,
  çizimin açılması, olgular, yalıtık Python'un açılışı ve kural). Başka içerikte paket, paketsiz istek ve yorumlayıcısız
  sürüm hiçbir kural başlamadan reddedildi. Katalog: iki sürüm yayımlanınca en yenisi, istenince eskisi; en yenisi
  askıya alınınca öncekisi; askıdaki ve taslak sürüm istenince ret; paketin baytları onaylı içerik.
- **Eksik kalanlar:**
  - **Sunucu kataloğu henüz kurmuyor.** `kentosd` üretimde kataloğu kurmuyor (`extension_catalog: None`; ADIM 22A'nın
    deposu şimdilik bellekte) ve işçinin işlemcisine denetim kaynağı verilmiyor (`ChildProcessor::with_audits` hazır,
    bağlanmadı). Bugün üretimde her `project.audit` işi "Bu sunucuda eklenti kataloğu açık değil" diye
    `tool_unavailable` ile biter. Kalıcı katalog deposu gelince `main.rs` işçiye `CatalogAudits::new(katalog,
    yorumlayıcılar)` verir; yorumlayıcı eşlemesinin ortam değişkeni o adımda okunur.
  - **Hazırlanmış ortamlar:** sürüm yayımlanınca kilit dosyasından bir kez, ağlı bir adımda kurulup eşlemeye
    eklenmeli; bugün eşleme elle yazılan bir ayar ve ortamı kuran adım yok.
  - **İşletim sistemi yalıtımı** (ağ ad alanı, salt okunur kök, sistem çağrısı süzgeci) ve **Python'u da ölçen bütçe**
    (süreç grubu): §42'deki eksikler aynen; sunucu bütçesi yalnız `kentos-jobrun`'ı ölçüyor, Python onun çocuğu.
  - **Önbellek kuralın belirlenimciliğine dayanır:** saati ya da rastgeleliği okuyan bir kural aynı anahtarla eski
    raporu alır. Kutu bunu engellemiyor; kural yazarının sözleşmesi olarak SDK belgesine yazılmalı.
  - **Önbellekten dönen iş de projeyi okur:** revizyon, imleç ve görünürlük damgası anlık görüntüyle birlikte
    okunuyor; büyük projede bunlar anlık görüntüden önce okunarak önbellekten dönüş ucuzlatılabilir.
  - **Yorumlayıcının yolu anahtarda değil:** ortam sürümün kilidinden kurulur, sürümün parçası sayıldı. Aynı sürümün
    ortamı başka bir Python'la yeniden kurulursa eski raporlar önbellekten dönmeye devam eder; ortamı kuran adım
    gelince ortamın özeti anahtara girmeli.
  - Göç numarası `0041`: koşut bir işte aynı numara alındıysa yeniden numaralanmalı (`sqlx::migrate!` sırayla
    uygular).
- **Sınamalar:** paylaşılan 1 (girdi alan alan, yorumlayıcı girdide reddedilir); sunucu uygulaması 4 (iş açılırken
  girdi denetimi, sonucun rapordan kurulması ve başka sürümün raporunun reddi, önbellek anahtarının içeriği, yorumlayıcı
  eşlemesi); `kentosd` katalog 3 (en yeni yayımlanmış ya da istenen sürüm, askı ve taslak; kurumun kataloğu önce, sonra
  genel katalog ve hazırlanan yorumlayıcı; eklentiyi tanıyan kurum kataloğu tek başına karar verir, taslak
  sahiplenmez); iş çalıştırıcısı üzerinden uçtan uca 2 (kural başlamadan reddedilenler;
  gerçek Python'da rapor iki kez aynı); veritabanlı HTTP 1 (iş raporu saklar, ikinci istek kutuyu çalıştırmadan
  önbellekten, yorumlayıcısız sürüm `tool_unavailable`; veritabanı yoksa atlanır). Python yoksa (`KENTOS_TEST_PYTHON`
  ya da `.run/py`) uçtan uca sınamalardan biri atlandığını söyler.
- **Sınamaların ortak payı düzeltildi:** cargo 1.100'ün yeni derleme klasörü düzeninde sınama programı
  `target/<profil>/deps/` yerine `target/<profil>/build/<paket>/<özet>/out/` altında; `child_exe` iki üst klasörü
  `target/<profil>` sanıyordu ve `KENTOS_JOBRUN` verilmeden bütün `job_run` sınamaları "kentos-jobrun yok" ile
  düşüyordu. Artık sınama programının üstündeki klasörlerde `kentos-jobrun` dosyasını arıyor (aynı adlı paket derleme
  klasörü dosya değil, atlanır).

## §51. Türkiye mevzuatının ilk kuralları: `tr.kentos.mevzuat` (`ADIM 20`, ulusal kuralların ilk dilimi)

**16 Kasım.** Kod: birinci taraf paket `catalog/standards/tr-mevzuat/` (`tr.kentos.mevzuat` 0.1.0; `manifest.json`,
`src/tr_mevzuat/paiy.py`, `bykhy.py`, `bina.py`); sınamalar `apps/jobrun/src/audit/mevzuat_tests.rs`; araştırma notu
`docs/research/2026-11-16-tr-mevzuat-denetim-kurallari.md` (hükümlerin aynen alıntısı, indirilen resmî metinlerin
adresi ve SHA-256'sı).

- **Karar: ulusal kurallar sıradan bir `.kek`'tir.** Plan kuralların yerini `catalog/standards/` diye yazıyor (ADIM 20:
  "`catalog/standards/*.py` ve `.kek` içindeki `@ext.audit`"); paket orada kaynak klasör olarak durur. Paket olduğu için
  masaüstünün Eklenti denetimi, ajanın `audit.run`'ı ve bulut işi (§42, §50) onu aynı yoldan, kökeniyle (paket özeti,
  çizim damgası, olgu özeti) çalıştırır; ayrı bir "yerleşik kural" yolu açılmadı. Yayımcı `kentos`, MIT, `utility`
  (sekme açmaz), `runtimes` masaüstü ve bulut; yalnız `audit.provider` ister: ağ yok, çizime yazma yok, Python
  bağımlılığı yok. `kentos kek verify` 0 uyarı; paket 10 983 bayt, 11 dosya.
- **Karar: kural yalnız olguların kesin söylediğine karar verir.** Olgular (§39) mahalin türünü (oturma odası, yatak
  odası), dar kenarını, binada yağmurlama sistemi olup olmadığını ve konut olmayan bölümün yangın yönetmeliğindeki
  kullanım sınıfını vermiyor; mahalin ve bölümün adı serbest yazıdır ve addan tür çıkarılmadı. Bu yüzden kurallar ya
  türden bağımsız kesin bir sonuca bakar (12 m²'lik piyesi olmayan konut), ya olguların verdiği türle çalışır (konut:
  `dwelling`), ya da bilinmeyeni bulguda söyler (yağmurlama: iki eşik).
- **Kaynaklar (16 Kasım'da mevzuat.gov.tr'den indirilen resmî metin, tablolar sayfa görüntüsüyle karşılaştırıldı):**
  Planlı Alanlar İmar Yönetmeliği (PAİY, RG 3.7.2017/30113; RG 14.1.2026/33137 değişikliği işlenmiş) ve Binaların
  Yangından Korunması Hakkında Yönetmelik (BYKHY, BKK 2007/12937, RG 19.12.2007/26735; son değişiklik RG
  1.7.2025/32943). Değerler KentOS Mimari'nin yerel denetimiyle (`fire_audit.rs`, `regulation_audit.rs`) aynı.
- **Kurallar:**
  - `oturma-odasi` — **PAİY 29/1** ("Her müstakil konutta en az aşağıdaki piyesler bulunur": oturma odası dar kenarı
    3.00 metre, net alanı 12.0 m²) ve **5/25**: konut bölümünde net alanı 12.0 m²'ye ulaşan piyes yoksa oturma odası
    hangi mahal olursa olsun sağlanamaz → hata, nesnesi en büyük piyes. 5/25'in yatak nişi istisnası (RG 13.8.2025)
    oturma odasını kaldırmaz; 29/4'e göre mutfak nişiyle birleşen oda daha da büyük olur. Sınırı bulunamayan mahali
    olan bölümde karar verilmez (bilgi).
  - `daire-ici` — **BYKHY 48/2** ("konut içindeki herhangi bir noktadan konut çıkış kapısına kadar olan uzaklığın 20
    m’yi, yağmurlama sistemi olan konutlarda 30 m’yi geçmemesi gerekir"): konutun her katında mahallerin en uzak
    noktasından (32/6: duvarlardan 40 cm önde) kapıya en uzun yürüyüş 20 m'yi aşarsa uyarı, 30 m'yi de aşarsa hata;
    kapısı o katta olan konutun kapıya yol bulamayan mahali hata.
  - `daire-kapisi` — **BYKHY 48/4** ("Kaçış uzaklığı, apartman dairelerinin kapısından başlanarak ölçülür"), **32/3**
    ve **Ek-5/B apartmanlar** (tek yön 15 m, yağmurlamalı 30 m; iki yön 30 m, yağmurlamalı 75 m): tek çıkışta bütün
    yol tek yöndür, iki çıkışta önce yolun tek yönlü kısmı (4/kk) tek yön sınırına, o aşılmıyorsa bütün yol iki yön
    sınırına; yağmurlamasız sınırı aşan uyarı, yağmurlamalıyı da aşan hata.
  - `cikis-erisimi` — **BYKHY 31/3** ("Her katta … çıkış imkânları sağlanır") ve **33/6-b** (her kullanıcı başka
    kullanıcıların odalarından geçmeden bir çıkışa doğrudan erişir): mahali olup kat çıkışı olmayan kat hata (kat başına
    bir), çıkışı olan katta kendi bölümünün mahallerinden ve ortak alanlardan hiçbir kat çıkışına ulaşmayan mahal hata.
- **48/1'in muafiyeti** ("Bodrum katlar dâhil 4 katı geçmeyen konutlar ile tek evler … kaçış mesafesi aranmaksızın"):
  `daire-ici` ve `daire-kapisi` bu binalarda bir şey söylemez. Kat sayımı KentOS Mimari'ninki: ±0.00 ve üstündeki
  katlar ile mahali olan alt katlar; tek konut bölümlü bina tek ev.
- **Bulgunun sözü maddeyi söyler:** "— PAİY md. 29/1 (RG 3.7.2017/30113)", "— BYKHY md. 48/2 (2009/15316 sayılı BKK
  ile değişik)", "— BYKHY md. 32/3, 48/4 ve Ek-5/B (apartmanlar)", "— BYKHY md. 31/3", "— BYKHY md. 33/6-b"; ölçü
  iki basamakla ("24.00 m", "11.50 m²"); yağmurlamaya bağlı bulgu "binada yağmurlama sistemi yoksa sınır aşılıyor"
  der. Nesne mahal ya da konut kapısıdır (Göster seçer); katın bulgusunun nesnesi yoktur.
- **Dışarıda kalanlar (neden):** PAİY 29/1'in öbür satırları ve bütün dar kenarlar (mahal türü ve dar kenar olgu
  değil; ikinci 9 m²'lik piyesi aramak da karar veremez: 5/25'in yatak nişi istisnası parseldeki bölümlerin %20'sine
  kadar ayrı yatak odasını kaldırıyor); PAİY 29/3 (genişlik yok); konut olmayan bölümlerin Ek-5/B uzaklıkları
  (kullanım sınıfı yok; sınıftan bağımsız eşik de dürüst değil: yağmurlamasız tek yön yüksek tehlikeli yerlerde 10 m,
  Ek-5/B (1) dipnotuyla bazı endüstri yapılarında 22.5 m); 4/i çıkmaz koridor (koridor türü yok); 48/2'nin ara katlı
  daire cümleleri (sonraki aday); kapısı yürüyüşsüz konut kapısı (merdivene mi açılıyor, çıkışa mı ulaşmıyor, olgu
  ayırmıyor); 33/1-2, 39/2, 48/5, Ek-5/A (genişlik, sınıf, korunumluluk yok).
- **SDK'ya öneriler:** `ArchitectureFacts.sprinklers` (projenin mimari ayarı, KCAD 226; uyarılar karara döner);
  bağımsız bölümün Ek-5/B kullanım sınıfı (sabit kod listesi); mahalin türü ve dar kenarı (KentOS Mimari `width_of`
  ile zaten ölçüyor; PAİY 29/1'in tamamı ve 29/3); `DoorEscape`'te "merdivene açılıyor" ile "çıkışa ulaşmıyor" ayrımı.
- **Sınamalar (gerçek Python, `KENTOS_TEST_PYTHON` ya da `.run/py`; yoksa atlandığını söyler):** paket klasöründen
  `kentos kek pack`'in okuduğu gibi paketlenir; kurallar `kentos.ext_host` üzerinden masaüstünün yoluyla çalışır. Her
  kuralın uygun ve aykırı hâli, örnek binanın (`fixtures/audit-facts/v1/mimari.json`, KentOS'un `mimari.kcad`'de
  ölçtüğü; iki katlı, iki daire, dükkân, büro) olgularında tek ölçü değiştirilerek: oturma odası (uygun; 11.50 m² →
  hata), daire içi (iki katta muaf; beş katta uygun; 24 m uyarı, 31 m hata, kapıya yol yok hata), daire kapısı (uygun;
  tek çıkışta 18 m uyarı, 32 m hata; iki çıkışta 40 m uyarı), erişim (uygun; çıkışa ulaşmayan arşiv hata; çıkışsız kat
  tek hata). Dört kural ayrıca çizimin kendisinde bulut işinin yoluyla (`run_audit`) çalışır: raporlar paketin
  sürümünün, örnek bina hepsine uygun. 5 sınama, 7,2 sn (hata ayıklama derlemesi, 18 Python süreci).

## §52. Denemede komutlar ve harita araçları (`ADIM 21`, Kapı 3'ün kodu çalıştıran payının ikinci dilimi)

**16 Kasım.** Kod: masaüstü `apps/desktop/src/shell/extensions/trial/steps.rs` (adımların okunması ve kurulu
manifestle denetimi), `trial/broker.rs` (kodun istekleri sentetik çizimde, adımın raporu), `trial.rs` (sıra,
`auditAfter`), `tools.rs` (`PoolJob::run_step`: denemenin süreci), `dialogs.rs` (`dialog_panel`: pencerenin denetimi
pencereyle ortak), `panel_session.rs` (`set`, `rows`, `say`, `unknown_panel`, `no_panel` ortak), `panels.rs`
(`OpenPanel::take_answer`: düğmenin cevabı panelle ortak), `runner.rs` (aracının ret sözleri sabit, ortak); ajan aracı
`crates/native/headless/src/agent_tools/workshop_trial.rs` (şema ve açıklama); SDK rehberi.

- **Karar: komut ve harita aracı kullanıcının başlattığı gibi denenir.** `commands[]`: ya `command` (şeritten: `params`
  kodun `ctx.params`'ı, verilmezse boş) ya `panel` ile `button` (panelin düğmesi ya da onChange'i olan alanı: panel
  açılır, ajanın değerleri panele kodun `panel.set`'iyle konur ve düğmenin denetimiyle sınanır, kod düğmenin
  komutunu panelin değerleriyle alır; cevabın satırları düğmenin tablosuna, sözü formun altına gider); `selection`
  (features'taki sıralar: kod çalışırken seçili olanlar). `mapTools[]`: `mapTool` ve `shape`, manifestin istediği
  şekil (nokta: Point; çizgi: LineString; alan: Polygon; dikdörtgen ve yönlü dikdörtgen: dört köşeli Polygon; daire:
  `center`, `radius`), kod onu çizimin çizilen şekli verdiği biçimde alır (runner.rs `shape_json`). Yanlış kimlik,
  olmayan panel ya da düğme, şekle uymayan şekil, features dışında sıra: çağrının hatası, hiçbir şey çalışmadan.
- **Karar: aracının kuralları aynen.** Okuma `document.read`, yazma `document.write` (kullanıcının verdiği); ilk yazma
  adımın geri alma adımını açar (kodun koyduğu adla, yoksa adımın adıyla); kod hata verir, Durdur'la durur ya da bir
  grubu başarısız olursa yazdıkları geri alınır ve sonraki adım onları görmez; başka eklentinin aracı yalnız
  bildirilmiş bağımlılıkla; geri alma, yineleme ve dosya işlemleri hiç. Ret sözleri runner.rs'deki sabitlerdir:
  kullanıcının çiziminde ve denemede aynı söz.
- **Karar: pencereleri ajan cevaplar.** `dialogs[]` sorulduğu sırayla: alan kimliğine göre değerler (`{}` varsayılanlarla
  Tamam), `null` İptal. Pencere gerçek pencerenin denetiminden geçer (bozuk tanım kodun hatasıdır); değerler kodun
  `panel.set`'iyle konup Tamam'ın denetimiyle sınanır. Uymayan ya da verilmeyen cevap İptal sayılır ve raporda nedeniyle
  söylenir (kod Vazgeç yolundan gider). **Dosya seçicili pencere İptal'dir:** kullanıcının dosyasını okutmak ya da
  üstüne yazdırmak ajanın kararı olamaz (§41, §44).
- **Karar: panelleri deneme tutar.** `ctx.open_panel` ve `ctx.panel.set/rows/say/close` panel_session.rs'nin
  denetimleriyle denemenin panellerinde çalışır; paneller adımlar arasında kalır (sonraki düğme önceki adımın
  doldurduğu seçeneklerle sınanır); `focus` bir şey yapmaz (pencere yok). Şeritten başlayan adımın `ctx.panel`'i yoktur
  (`no_panel`, gerçekteki söz).
- **Karar: denemenin kendi süreci.** Komut, havuzun sürüm sürecinde değil `<eklenti>@<sürüm>#deneme` anahtarlı ayrı bir
  süreçte çalışır. Havuzun süreci komutla meşgulken komut kendi eklentisinin aracını çağırırsa
  (`ctx.run_tool("ext.<kendi>.<araç>")`, §46) araç o sürecin sırasını, komut da aracı bekler ve ikisi 120 sn sonra
  sonlandırılırdı. Kullanıcının çiziminde komutlar zaten ayrı süreçte (runner.rs). Denemenin süreci 10 dk boşta kalınca
  kapanır; sürüm değişince ya da eklenti kaldırılınca sürümünkiyle birlikte kapanır.
- **Sıra ve `auditAfter`:** kurallar, araçlar, komutlar, harita araçları; her adım öncekilerin yazdığını görür.
  `auditAfter: true` kuralları en sonda son çizimde yeniden çalıştırır (`auditsAfter`): bir düzeltme komutunun bulguyu
  kapattığı böyle görülür. Komut ya da harita aracı denerken `features` boş olabilir (katmanları kuran komut).
- **Adımın raporu:** durum (`done`; `failed` istisnası ve iz kaydıyla; `refused`; `unavailable`; `stopped`;
  `invalid`: panelin değerleri uymadı, kod çalışmadı), sonucun sözü, başlığı ve satırları (sayısı ve ilk 10'u), yazılan
  nesneler katmana göre, değişen nesneler (features'taki sırasıyla, ilk 5'inin öznitelik ve şekli) ve silinenler,
  `undone`, kodun seçtikleri, sorduğu pencereler ve verilen cevaplar, KentOS'un geri çevirdiği istekler (en çok 20),
  dokunduğu paneller bıraktığı hâliyle (değerler, söz, tabloların satır sayısı ve ilk satırları), yazdırdıkları (en çok
  8 KB), süre. Özet satırı komut ve harita aracı sayısını ve yeniden koşan kuralların bulgusunu da söyler.
- **Sınırlar:** en çok 10 komut ve 10 harita aracı yerleştirmesi (`TRIAL_MOST_COMMANDS`), adım başına 10 pencere
  cevabı; adımın süresi kuralınki (120 sn).
- **Bilerek dışarıda:** dosya seçen pencere (yukarıda); harita aracının kenetlenmesi (ajan koordinatı kendisi verir) ve
  yönlü dikdörtgenin şablon ölçüsü (verilen dört köşe olduğu gibi gider). Komut kullanıcının verdiği haklarla gerçekten
  çalışır, deneme yalnız çizimi sentetik yapar: eklentinin ağa ya da diske doğrudan yaptığı bir şey varsa o da olur
  (PY-RUN-19, "ayrı süreç korumalı alan değildir"); kurallarda ve araçlarda olduğu gibi.
- **Sınamalar:** masaüstü 8 yeni: adımların okunması 3 (harita aracının istediği şekil ve yanlışları, seçim ve
  cevaplar, panel düğmesinin ve değişen alanın komutu ve yanlışları), aracı 4 (sahte süreç, sentetik çizimde gerçek
  KentOS komutlarıyla: okuma, yazma, seçim, tek geri alma adımı ve hata verenin yazdıklarının geri alınması; izinsiz
  yazma, bağımlılıksız başka eklenti aracı ve okuma izni olmayana çizimin verilmemesi; pencerelerin cevabı, uymayan
  ve verilmeyen cevap, dosya seçici, bozuk tanım; panel düğmesi: eksik değer `invalid`, kodun panel istekleri,
  satırların düğmenin tablosuna gitmesi, şeritten `no_panel`); gerçek Python 1 (testin kendi küçük paketi
  `kentos.ext_host` üzerinden: pencere cevabıyla iki yapıya B-05 ve B-06, panelin tablosu ve sözü, yazdırılan satır;
  cevapsız pencere İptal, bir şey yazılmaz; yazıp hata veren komutun yazdığı iz kaydıyla geri alınır; harita aracı
  verilen dikdörtgeni yazar; 1,07 sn; Python yoksa atlandığını söyler); ajan aracı 1 (komutla boş çizim kabul edilir).
- **Gerçek pencerede** (Asistan, DeepSeek deneme anahtarı, sentetik istem, Vitrin 1.0.41): tek çağrıda Yapıları
  numarala (pencerenin cevabı önek B-, ilk numara 5), Yapı yerleştir ((100, 100) köşeli 10 × 12 m dikdörtgen) ve
  `auditAfter`. Komut B-05 ve B-06 yazdı, Yapı ara panelinin tablosunda 2 satır; Yapı numarası denetimi önce 2 bulgu,
  sonra 1 (harita aracının eklediği numarasız yapı); deneme 5,2 sn (ilk koşuda süreç açılışıyla 4,2 sn). Bir koşuda
  ajanın bir deneme çağrısı girdi denetiminde geri çevrildi (1 ms, hiçbir şey çalışmadan); ajan düzeltip yeniden
  denedi. Kullanıcının çizimi aynı kaldı.

## §53. Mimari olgular, üçüncü dilim: mahalin türü ve dar kenarı, yağmurlama, merdivene açılan kapı (`ADIM 20`)

**16 Kasım.** Kod: `kentos_domain::room_width` (dar kenar ölçüsü ve net dış hat; Mimari denetimden taşındı,
`program_check::width_of` ve `regulation_audit::net_parts` artık onu dışa açar), `kentos_domain::escape`
(`DoorEscape::into_stair`), sözleşme `kentos_extensions::audit_facts::architecture` (`RoomKind`, `RoomFacts.kind`,
`RoomFacts.narrow`, `ArchitectureFacts.sprinklers`, `DoorEscape.into_stair`), üretici
`kentos_processing::audit_facts::architecture`, SDK `python/kentos/ext.py`, şema `sdk/kentos-audit-facts.schema.json`,
fikstür `fixtures/audit-facts/v1/mimari.json`; stub üreticisi ve paket `crates/shared/extensions/src/sdk_pack.rs`.

- **Neden:** §51'in kuralları dört şeyi bilmediği için karar veremiyordu: mahalin türü ve dar kenarı (PAİY 29/1'in
  yatak odası, mutfak, banyo ve tuvalet satırları ile bütün dar kenarlar; 29/3 hol ve koridor 1,20 m), binada yağmurlama
  sistemi olup olmadığı (BYKHY 48/2 ve Ek-5/B'nin iki eşiği uyarı ve hata diye ikiye bölünüyordu) ve yürüyüşü olmayan
  konut kapısının merdiven mahaline mi açıldığı, hiçbir çıkışa mı ulaşmadığı.
- **Karar: Mimari denetimin okuması, aynı kodla.** Tür `kentos_domain::room_names::kind_of`: ad ya da kullanım; Salon
  ve Oturma `living`, Yatak, Ebeveyn, Çocuk ve Oda `bedroom`, Mutfak `kitchen`, Banyo, Duş ve Yıkanma `bath`, WC ve
  Tuvalet `toilet`, Hol, Koridor ve Antre `corridor`; başka bir mahalin türü yok (depo, çalışma odası, balkon). Dar
  kenar net dış hattın içindeki en büyük dairenin çapıdır (`width_of`, 0,1 mm duyarlıkla); net dış hat tohumdan yeniden
  bulunan iç yüzler, yoksa çizilen halkalar (`net_parts`). İkisi Mimari denetimden `kentos_domain`'e taşındı: denetim
  aynı işlevi kullanmaya devam ediyor ve sonuçları değişmedi (Mevzuat, yangın, asansör, korkuluk ve otopark
  sınamaları).
- **Karar: sürüm 1'in isteğe bağlı alanları.** Yeni alanlar okunurken varsayılanlı: eski bir KentOS'un olguları türsüz,
  dar kenarsız, yağmurlamasız ve `intoStair: false` okunur (Python sınıflarında aynı varsayılanlar). Olguların sürümü
  değişmedi; mimarisi olmayan çizimin olguları bayt bayt aynı kaldı (`denetim.json` değişmedi), `mimari.json` yeni
  alanlarla yeniden yazıldı. Sınırı bulunamayan mahalin dar kenarı yok: çizilen halka yanıltır.
- **`sprinklers`** projenin mimari ayarından (`.kcad` 226, docs/adr/8035), binanın tamamı için.
- **`intoStair`:** konut kapısı kaçış merdiveni mahaline açılıyor; kapı kat çıkışındadır ve yürüyüşü yoktur. Yürüyüşü
  olmayan ve `intoStair` yanlış olan kapı hiçbir çıkışa ulaşmıyor demektir.
- **SDK paketi sınırda:** yeni alanlarla stub büyüdü ve ajan paketi 48 000 baytlık sınırı (Asistan'ın araç sonucu
  sınırı: "tokens are the user's money") 87 baytla aştı. Sınır yükseltilmedi; stub üreticisi açıklamalardan KentOS'un
  iç tasarım belgelerine göndermeleri (`docs/adr/… §…`, `plan/eklenti.md ADIM …`) atar, paket stub'ın başındaki
  bakımcı notunu (nasıl üretildiği, elle düzenlenmeyeceği) almaz. Eklenti yazarı API'yi okur, kayıtları değil.
- **Dışarıda kalan:** bağımsız bölümün Ek-5/B kullanım sınıfı. Modelde yok (nitelik tapudaki serbest yazıdır); programa
  bir sınıf alanı girince olgulara da girer.
- **Sınamalar:** alan 1 (dikdörtgenin dar kenarı ve merkezi; halkasız ölçü), olgu sözleşmesi 3 (dar kenar 1 mm'ye,
  `kind`, `narrow`, `intoStair` ve `sprinklers` anahtarları, okunup aynı geri gelmesi, şemaya uyması), üretici
  (`mimari.json` bayt bayt: Salon `living` 3,85 m, Koridor `corridor` 1,40 m, Arşiv türsüz 3,35 m, kapılar merdivene
  açılmıyor, yağmurlama yok), Python 1 (türler, dar kenarlar, yağmurlama ve eski olguların varsayılanları), SDK paketi 1
  (iç belge göndermelerinin stub'dan atılması: yalnız, başka sözle birlikte, noktalı virgülden sonra).

## §54. Türkiye mevzuatı 0.2.0: piyesler, hol ve koridor, yağmurlama ayarı, merdivene açılan kapı (`ADIM 20`, ulusal kuralların ikinci dilimi)

**16 Kasım.** Kod: `catalog/standards/tr-mevzuat/` (`tr.kentos.mevzuat` 0.2.0; `src/tr_mevzuat/paiy.py`,
`piyes_eksik.py`, `bykhy.py`, `bina.py`, `manifest.json`, `CHANGELOG.md`); sınamalar
`apps/jobrun/src/audit/mevzuat_tests.rs` ile `mevzuat_tests/paiy.rs`, `mevzuat_tests/bykhy.rs`; araştırma notu
`docs/research/2026-11-16-tr-mevzuat-denetim-kurallari.md` (29/2, 29/3, 29/4 ve BYKHY 48/3 aynen; resmî metinler 16
Kasım'da yeniden indirildi, SHA-256'ları §51'inkiyle aynı).

- **Karar: §53'ün olgularıyla karar, eksik bilgide "karar verilemedi".** Mahalin türü ve dar kenarı KentOS Mimari'nin
  okuyuşudur; kural addan tür çıkarmaz. Yönetmeliğin istisnası olguların göremediği bir şeye bağlıysa (bir odanın içindeki
  niş, mahalin donanımı, parseldeki pay) bulgu bilgidir ve karar verilemediğini söyler; istisna mümkün değilse hatadır.
- **Karar: PAİY 29/1 tek kural (`piyesler`, 0.1.0'ın `oturma-odasi`'nın yerine), 29/3 ayrı (`koridor`).** Eksik piyesin
  kararı konutun bütün piyeslerine bakar (29/2 oda sayar, 29/4 nişi taşıyacak odanın alanını toplamla karşılaştırır,
  türü okunamayan konutta 12 m²'lik denetim tür denetiminin yerini alır); ayrı kurallar aynı konutu ayrı ayrı çözer,
  çelişen ya da yinelenen bulgu verirdi. 29/3 başka bir fıkra ve başka bir alan (ortak alanın holleri de).
- **`piyesler` (PAİY 29/1, 29/2, 29/4, 5/25):**
  - Türü bilinen her piyes tablonun en azına: oturma odası 3.00 m / 12.0 m², yatak odası 2.50 m / 9.00 m², mutfak 1.50
    m / 3.30 m², banyo 1.50 m / 3.00 m², tuvalet 1.00 m / 1.20 m². Biri yetmiyorsa hata, bulgu iki ölçüyü de sınırıyla
    söyler ("net alanı 8.50 m² (en az 9.00 m²), dar kenarı 3.85 m (en az 2.50 m); alanı yetmiyor"). Tablo her piyese
    uygulanır (ikinci yatak odası da); KentOS Mimari'nin yerel denetimi de böyle okur.
  - Eksik piyes: oturma odasının istisnası yok (hata). Yatak odası: oturma odası 5/25'in yatak nişini (2.50 m, 9 m²)
    taşıyabiliyorsa karar verilemez, taşıyamıyorsa ya da oturma odası yoksa hata. Mutfak: 29/4'e göre nişi taşıyan oda
    kendi en azı ile 3.30 m²'nin toplamı kadardır (oturma odasında 15.30, yatak odasında 12.30 m²); böyle oda varsa karar
    verilemez, yoksa hata. Tuvalet: 3 veya daha az odalı (oturma ve yatak odaları) konutta banyo tuvaleti de
    karşılayabilir (29/2) ve o zaman en az 4.20 m² olur (29/4): böyle banyo varsa bulgu yok, banyolar küçükse hata; 4 ve
    daha çok odada tuvalet ayrıdır, yoksa hata. Banyo: 3 veya daha az odalı konutta 4.20 m²'lik bir tuvalet ikisini
    birlikte karşılıyor olabilir: karar verilemez, değilse hata. Adı türünü söylemeyen bir mahal eksik türün en az
    ölçüsünü karşılıyorsa o tür olabilir: karar verilemez, bulgu adının düzeltilmesini söyler.
  - Piyeslerinin hiçbirinin türü okunamayan konutta (adlar türü söylemiyor ya da olgular eski bir KentOS'tan) yalnız
    0.1.0'ın kesin sonucu: 12.0 m²'ye ulaşan piyes yoksa hata.
- **`koridor` (PAİY 29/3):** türü `corridor` olan mahalin dar kenarı 1.20 m'den azsa hata; bulgu fıkrayı aynen söyler
  (“Hol ve koridor genişlikleri 1.20 metreden az olamaz.”). Alan KentOS Mimari'ninki: konutun ve ortak alanın holleri
  ile bir bölüme ait olmayanlar; dükkânın, büronun, eklentinin kendi holü okunmaz.
- **Yağmurlama ayarı (BYKHY 48/2, Ek-5/B):** sınır projenin ayarınındır: işaretliyse yağmurlamalı sütun (48/2'de 30
  m; apartmanlarda tek yön 30, iki yön 75 m), değilse yağmurlamasız sütun (20 m; 15 ve 30 m); aşan ölçü hatadır ve
  bulgu hangi sınırın, hangi ayarla okunduğunu söyler ("projenin ayarında yağmurlama sistemi işaretli değil, sınır
  20.00 m"). Ayarın varsayılanı "yok"tur (`.kcad` 226: "Absent: none") ve KentOS Mimari'nin yangın denetimi de
  ayarsız projeyi yağmurlamasız okur (`fire_audit.rs`: `arch.sprinklers`, ayar yoksa `false`): kural da öyle okur.
  **Eski olgular:** ayarı vermeyen bir KentOS'un olgularında `sprinklers` okununca `False` gelir ve "yok" ile "ayar
  verilmedi" ayırt edilmez. Paket olguların yeni olduğunu sınırı bulunan bir mahalin dar kenarından tanır (§53'ün
  KentOS'u onu her zaman yazar): yeni olgularda ayar kesindir; eski olgularda 0.1.0'ın iki eşiği sürer (yağmurlamasız
  sınırı aşan uyarı, ikisini de aşan hata) ve bulgu olguların eski olduğunu söyler.
- **Konut kapısı (BYKHY 48/3, 48/4, 33/6-b):** `into_stair` olan kapı kat çıkışındadır, uygundur. Yürüyüşü olmayan ve
  merdivene de açılmayan kapı hatadır: dayanağı 48/3 (“Konut birimlerinden bütün çıkışların, kaçış merdivenlerine veya
  güvenli bir açık alana doğrudan erişim imkânı sağlayacak şekilde olması gerekir.”) ve 33/6-b. 48/4 yalnız uzaklığın
  kapıdan ölçüleceğini söylediği için ulaşılmayan çıkışın dayanağı 48/3 seçildi; uzaklık bulgusu 32/3, 48/4 ve Ek-5/B
  der. 48/1 uzaklığı muaf tutar, çıkışı değil: bu hata her binada verilir. Eski olgularda iki durum ayırt edilmez,
  karar verilmez.
- **Karar verilemeyenler:** 5/25'in yatak nişi ve %20 sınırı (niş ve parseldeki bölüm sayısı olgu değil); 29/4'ün
  mutfak nişi (niş görünmez); 29/2'nin birleşik banyo-tuvaleti tuvalet adıyla çizildiğinde (donanım görünmez). Konut
  olmayan bölümün Ek-5/B kullanım sınıfı modelde yok: o bölümlerin kaçış uzaklığı bu sürümde de yok. Çıkmaz koridor
  (4/i) ve 48/2'nin dubleks cümleleri sonraki adaylar.
- **Örnek binada (mimari.kcad, bulut işinin yoluyla):** iki dairede oturma ve yatak odaları var, mutfak, banyo, tuvalet
  yok: `piyesler` her daire için mutfakta "karar verilemedi" (salon nişe yetiyor), banyo ve tuvalette hata verir (4 hata,
  2 bilgi); öbür dört kural uygun.
- **Sınamalar (gerçek Python; yoksa atlandığını söyler):** `cargo test -p kentos-jobrun --bin kentos-jobrun --
  audit` 13/13 (4 eski, 9 mevzuat), 11.2 sn; her durum örnek binanın olgularında birkaç ölçü değiştirilerek. Piyesler:
  tam daireler uygun; 8.50 m²'lik yatak odası ve 2.80 m dar kenarlı salon hata, bulgu iki ölçüyü söyler; çizildiği
  gibi 4 hata 2 bilgi; nişe yer yokken mutfak hata; 2 odada 4.50 m²'lik banyo tuvaleti karşılar, 3.50 m²'lik hata;
  4 odada tuvalet hata; yatak odası yokken yatak nişi "karar verilemedi"; oturma odası yok hata; adı türsüz ama mutfak
  boyunda mahal "karar verilemedi"; türü okunamayan ve eski olgularda 12 m² denetimi. Koridor: 1.10 m hata, fıkra
  aynen; büronun kendi holü okunmaz. Daire içi: iki katta muaf; beş katta ayarsız 24 m hata ("sınır 20.00 m"), ayarla
  uygun, 31 m ayarla hata ("sınır 30.00 m"); eski olgularda uyarı; kapıya yolu olmayan mahal hata. Daire kapısı: ayarsız
  tek yönde 18 m hata, ayarla uygun, 32 m ayarla hata; iki yönde 40 m ayarsız hata, ayarla uygun; eski olgularda uyarı;
  merdivene açılan kapı uygun, çıkışa ulaşmayan kapı iki katlı binada da hata (48/3, 33/6-b), eski olgularda karar
  yok. Çıkış erişimi değişmedi. Beş kural ayrıca örnek çizimde bulut işinin yoluyla (`run_audit`). `kentos kek verify`
  0 uyarı (yeniden derlenmiş komut satırıyla); paket 16 999 bayt, 12 dosya.

## §55. Panelde satır: `row` ve ortak formun hücreleri (`ADIM 22`, bileşen envanterinin düzen payı)

**16 Kasım.** Kod: ortak form `crates/ui/src/widget/form.rs` (`Form::cells`, `Form::line_count`, `Line::Cells`,
`Cells`), paylaşılan `crates/shared/extensions/src/ui_panel/rows.rs` (`row` anahtarı, Kapı 1), şema
`sdk/kentos-ui-panel.schema.json`, Python kurucuları `python/kentos/ui_panel.py` (`row=`), masaüstü
`apps/desktop/src/shell/extensions/panel_rows.rs` ve `panel_view.rs`; Vitrin 1.0.42.

- **Karar: satır bileşenin anahtarıdır, kap bileşeni değil.** `"row": "Konum"` taşıyan komşu bileşenler bir satırın
  hücreleridir (Ada | Parsel, Önek | İlk numara, Ara | Temizle). Sekmenin yolu (§43): gövde düz liste kalır, var olan
  bileşenler değişmez, Python'da tek parametredir; iç içe gövdeli bir kap bileşeni tanım biçimini, denetimi ve kural
  okumasını değiştirirdi.
- **Karar: düzeni KentOS kurar, ortak formda.** Geniş yerde hücreler eşit genişlikte yan yana, her hücrede alanın adı
  üstünde; her hücreye en az 140 piksel (gövde yazısıyla büyür) kalmıyorsa hücreler formun öbür alanları gibi alt alta
  iner (PY-UI-13: ekran boyu sunum profili, kimlik ve doğrulama aynı). Dock'taki panelde ve `ctx.dialog`'un penceresinde
  aynı bileşen çizer. Aynı sayıda hücreli ardışık satırlar sütun gibi hizalanır: ızgara ayrı bir bileşen istemiyor.
- **Görünüş, kural değil:** hücredeki alan sorulur, denetlenir, değeri komuta gider; zorunluluk yıldızı ve yanlışın
  sözü hücrenin içinde. Gizlenen bileşen satırını öbürlerine bırakır; satır başka satırın ya da satırsız çizilen ilk
  bileşende ve sekme çubuğundan önce biter. Tek hücresi çizilen satır sıradan bir alandır.
- **Kapı 1:** satırın adı tek satır, en çok 40 karakter; satırın bileşenleri gövdede art arda, aynı sekmede, en çok 4;
  yalnız küçük türler (metin, metin/sayı/tarih/saat/an alanları, seçim, çoklu seçim, onay kutusu, katman/alan/koordinat
  sistemi seçicisi, dosya seçici, düğme). Tablo, ağaç, kart, özet, grafik, görsel ve bölüm panelin genişliğini alır,
  satıra girmez. Her yanlış yeriyle söylenir (`/body/2/row`).
- **Vitrin 1.0.42:** Yapıları numarala'nın penceresinde Önek ve İlk numara bir satırda; hesaplı açıklama, zorunluluk ve
  değerler eskisi gibi (gerçek pencere: B- ve 10 yazıldı, 5 yapı B-10 … B-14 oldu). Alanların yeri değiştiği için
  senaryolar 29, 30 ve 35'in tıklama noktaları güncellendi.
- **Bu dilimde yok:** bölücü (düz çizgi; başlıklı bölüm `section` var), adımlayıcı, birim seçici ve envanterin kalanı.
- **Sınamalar:** ortak form 1 (400 pikselde iki hücre yan yana, 240 pikselde alt alta), paylaşılan 1 (adın kırpılması;
  art arda olmayan, satıra girmeyen türde, yanlış adlı, başka sekmede ve beşinci hücre yerleriyle), Python
  kurucularının imzası (`row`), masaüstü 1 (satırın hücre sınırları: başka satırda, satırsızda ve formun sonunda biter).

## §56. Türkiye mevzuatı 0.3.0: çıkmaz koridor ve ara katlı daire (`ADIM 20`, ulusal kuralların üçüncü dilimi)

**16 Kasım.** Kod: `catalog/standards/tr-mevzuat/` (`tr.kentos.mevzuat` 0.3.0; yeni `src/tr_mevzuat/cikmaz.py`,
`ara_kat.py`; `bina.py`'ye taşınan `karar`; `manifest.json`, `CHANGELOG.md`); sınamalar
`apps/jobrun/src/audit/mevzuat_tests/cikmaz.rs`, `ara_kat.rs` ve `mevzuat_tests.rs` (bulut işinin yolu); araştırma
notu `docs/research/2026-11-16-tr-mevzuat-denetim-kurallari.md` (BYKHY 4/i, 30/5'in çıkmaz alan cümlesi, 48/2'nin
tamamı, 43/1 ve Ek-5/B'nin çıkmaz koridor sütunu aynen; 22 alıntının hepsi resmî metinde aynen geçiyor).

- **Kaynak (aynen):** 4/i (2009/15316 BKK ile değişik): “Çıkmaz koridor mesafesi: Mekân içerisinden mekânın koridora
  bağlanan kapısına kadar olan mesafe göz önüne alınmaksızın, kaçışta, mekânların bağlı olduğu koridorun en uzak
  noktasından koridor boyunca bir çıkışa veya iki yönde kaçış imkânına sahip olunan noktaya kadar olan mesafeyi”.
  Ek-5/B'nin "Çıkmaz koridor en çok uzaklık" sütunu (yağmurlama sistemi yok / var): yüksek tehlikeli yerler 10/20,
  öbür bütün satırlar, apartmanlar dahil, 15/20 m; dipnot (1) yalnız tek ve iki yönlü uzaklığı artırır. Çıkmaz alandan
  söz eden öbür hüküm 30/5 sayısal sınır vermez.
- **`cikmaz-koridor` (BYKHY 4/i, Ek-5/B):** türü `corridor` olan mahalin kendi yürüyüşünün tek yön payı (en uzak
  noktasından bir kat çıkışına ya da iki yönün ayrıldığı noktaya; tek çıkışta bütün yol) projenin yağmurlama ayarının
  sınırına: 15 m, yağmurlama sistemiyle 20 m; aşarsa hata. Bulgu ölçüyü, sınırı, ayarı ve maddeyi söyler ("çıkmaz
  koridor 21.82 m …; projenin ayarında yağmurlama sistemi işaretli değil, sınır 15.00 m — BYKHY md. 4/i ve Ek-5/B").
  Ayarı vermeyen eski bir KentOS'un olgularında koridor türü de yok: kural bir şey söylemez (yağmurlama ayarı 0.2.0'daki
  `bina.yagmurlama`/`karar` ile, iki eşik dahil, aynı yoldan okunur).
- **Karar: ölçü KentOS Mimari'nin.** Yerel yangın denetimi (`fire_audit.rs`) çıkmaz koridoru koridorun kendi
  yürüyüşünün tek yön payı sayar ve `Use::dead_end` ile 15/20 m'ye (yüksek tehlikeli yerde 10/20) bakar; olgularda aynı
  yürüyüş var. Örnek binanın 1. kat koridoru (21.82 m) iki denetimde de hata.
- **Karar: konut olmayan sınıflar da okunur, çünkü sütun sınıftan bağımsız.** Görevde konut olmayan kullanım
  sınıflarının (olgularda yok) atlanması istenmişti; ama çıkmaz koridor sütunu yüksek tehlikeli yerler dışında her
  satırda 15/20 m. Konut olmayan bölümün ya da karma binanın koridorunda bu sınırı aşan çıkmaz her sınıf için
  aykırıdır: karar verilir ve bulgu "kullanım sınıfı olgularda yok; yüksek tehlikeli yerler dışında her sınıfın sınırı"
  der. Sınıfa bağlı tek değer yüksek tehlikeli yerin yağmurlamasız 10 m'si; yüksek tehlikeli yer olgularda tanınmaz,
  10-15 m arası söylenmez.
- **Yalnız konut bölümleri olan bina:** ortak koridoru Ek-5/B'nin apartmanlar satırıyla (15/20 m) okunur ve 48/1'in
  muafiyeti uygulanır (çıkmaz koridor mesafesi bir kaçış mesafesidir). **Yerel denetimden ayrılış:** KentOS Mimari bu
  koridoru hiç okumuyor (kullanımı `None` kalıyor, kural geçiyor). Karma binada 48/1 uygulanmaz: ortak koridor konut
  olmayan bölümlere de hizmet eder (31/5, en sıkı kullanım). Konutun ve konutun eklentisinin kendi holü 48/2'nindir,
  okunmaz (yerel denetim de konutun mahallerini bu kurala sokmaz).
- **`ara-katli-daire` (BYKHY 48/2'nin ikinci ve üçüncü cümleleri, karar verilebildi):** “İkiden çok ara kat bulunmayan
  apartman dairelerinde tek kapı bulunması hâlinde, bu kapı üst katta düzenlenemez. Üstteki katın döşeme alanı, bu kat
  için ayrı bir çıkış sağlanmadıkça 70 m2’yi aşamaz.” Olgularla karar verilen hâl iki katlı konut bölümüdür
  (`UnitFacts.levels`'ta iki kat): "ara kat" yönetmelikte tanımlı değil ve 43/1 onu kattan ayrı sayar, ama iki katlı
  daire her okuyuşta bu cümlelerin içindedir ve "üst kat" yüksek olandır. Dairenin tek kapısı (olguların konut
  kapıları, katlarıyla) üst kattaysa hata. Üst katta kapı yoksa üst katın döşeme alanı 70 m²'yi aşamaz: "döşeme alanı"
  net mi brüt mü yazılı değil, net alan aşıyorsa hata, yalnız brüt aşıyorsa "karar verilemedi" (bilgi). Üst katta kapı
  varsa o katın ayrı çıkışıdır. Fıkra "Birinci fıkrada belirtilenler dışındaki konutlarda" diye başladığı için 48/1'in
  muaf tuttuğu binalarda okunmaz. **Karar verilmeyen:** üç ve daha çok katlı daire (kaç ara katı olduğu ve hangisinin
  "üstteki kat" olduğu belli değil). Yerel denetimde bu cümlelerin karşılığı yok.
- **Örnek binada (mimari.kcad, bulut işinin yoluyla):** `cikmaz-koridor` 1. kat koridorunda 21.82 m (hata);
  `ara-katli-daire` iki katlı binada (48/1) bir şey söylemez; öbür kurallar §54'teki gibi.
- **Sınamalar (gerçek Python; yoksa atlandığını söyler):** `cargo test -p kentos-jobrun --bin kentos-jobrun --
  audit` 16/16 (4 eski, 12 mevzuat), 14.2 sn; her durum örnek binanın olgularında birkaç ölçü değiştirilerek. Çıkmaz
  koridor: çizildiği gibi 21.82 m ayarsız hata ("sınır 15.00 m", sınıfsız söz), 12 m uygun; ayarla 19 m uygun, 21.82 m
  hata ("sınır 20.00 m"); yalnız konutlu binada iki katta muaf (48/1), beş katta apartmanlar satırıyla hata; konutun
  kendi holü okunmaz, büronun koridoru (22.52 m) hata; eski olgularda bir şey söylenmez. Ara katlı daire: iki katlı
  binada muaf; beş katta 60/68 m²'lik kapısız üst kat uygun, 75/80 m² hata, 66/74 m² "karar verilemedi"; tek kapı üst
  katta hata; her katta kapı varsa 90 m²'lik üst kat uygun; üç katlı dairede karar yok. Yedi kural ayrıca örnek çizimde
  bulut işinin yoluyla (`run_audit`): `piyesler` 4 hata 2 bilgi, `cikmaz-koridor` 1 hata, öbürleri uygun. `kentos kek
  verify` 0 uyarı (yeniden derlenmiş komut satırıyla); paket 21 257 bayt, 14 dosya.

## §57. Panelden harita seçimi: `pointPicker` (`ADIM 22`, bileşen envanteri, CBS girdisi)

**16 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/ui_panel.rs` (`Component::PointPicker`, `point_value`,
`point_words`, `typed_point`, Kapı 1), şema ve Python kurucusu (`ui.point_picker`, `value: Sequence[float]`), masaüstü
`apps/desktop/src/shell/extensions/panel_points.rs` (alan, seçim, komut), `panels.rs` (değer, zorunluluk),
`panel_session.rs` (`ctx.panel.set(id, value=[x, y])`), `panel_view.rs`, `shell/input.rs` (seçilen noktanın zinciri),
`shell/app/update.rs` (`ext_point_tasks`); Vitrin 1.0.43.

- **Karar: değer `[x, y]`, çizimin düzleminde, metre.** Alanda `x, y` milimetreye yazılı durur; kullanıcı yazabilir
  (`x, y`, `x y` ondalık noktayla; ondalık virgülle `x; y`) ya da Haritadan seç'e basar. Kod `ctx.params["nokta"]`'yı
  `[x, y]` (boşsa None) alır; `ctx.panel.set(id, value=[x, y])` yazar. Nokta olmayan yazım "“Nokta” bir nokta olmalı"
  diye alanın altında söylenir, komut çalışmaz.
- **Karar: İşlemler penceresinin seçimi.** Haritadan seç, İşlemler aracının nokta parametresinin `PickPoint`'idir:
  kenetlenme uygulanır, komut satırı hangi panelin hangi alanı için seçildiğini söyler, Esc alanı olduğu gibi bırakır.
  Dock'taki panel haritanın yanında durduğu için (İşlemler penceresi gibi) kenara çekilmez. Düğme hedef ikonlu ve
  yazısız, adı ve ne yaptığı üzerine gelince: yazılı "Haritadan seç" düğmesi 300 piksellik dock'ta alanı birkaç piksele
  indiriyordu (gerçek pencere).
- **Seçilen nokta bir değişikliktir:** alanın `onChange` komutu çalışır (takvimden seçilen günün yolu). Komut
  güncellemenin sonunda başlar (`ext_point_tasks`, harita aracının şeklinin yolu): ilk sürümde görev panel oturumunun
  `later` kuyruğuna konmuştu, kuyruk yalnız eklentinin süreci konuşunca boşaldığı için komut hiç başlamıyor, düğme
  basılı kalıyordu (gerçek pencere).
- **Pencerede yok:** `ctx.dialog`'un penceresi cevaplanana dek pencereyi tutar; noktayı oradan seçmek pencereyi kapatıp
  açmayı ister. Pencerenin alanları (14 tür) değişmedi.
- **Kapı 1:** `pointPicker` `id`, `label`, `value`, `onChange`, `required` alır; `value` iki sonlu sayı. Satıra girer
  (`row`, §55). Kural dili alanı metin olarak okur (`visibleWhen: "boş(nokta)"`).
- **Vitrin 1.0.43:** "Yakın yapılar" paneli (Vitrin › Panel): Nokta (`pointPicker`, `onChange`) ve Uzaklık bir satırda,
  Bul, tablo; `yakin-yapilar` komutu noktaya uzaklıktaki yapıları shapely ile ölçüp yakından uzağa listeler, satıra
  tıklamak yapıyı gösterir. Gerçek pencerede (47-nokta): yapıların arasına tıklandı, nokta alana yazıldı, komut çalıştı:
  "25 m içinde 3 yapı" (6,4, 6,6, 20,6 m). Dar dock'ta Nokta ile Uzaklık alt alta.
- **Sınamalar:** paylaşılan 1 (yazımın üç biçimi ve yanlışları, JSON değeri, milimetre yazımı), rehberin ve ADR §25'in
  türü anması (var olan sınamalar), Python kurucusu; masaüstü 2 (değer `[x, y]`, boş, zorunlu, nokta olmayan yazım,
  kodun `set`'i ve yanlışı; seçimin alana yazılması, Esc, soran alan yokken seçimin başkasına kalması, `onChange`'in
  güncellemeden sonra istenmesi); Vitrin'in sayıları (14 komut, 4 panel) ve şerit etiketleri.


## §58. İşin bütçesi kuralın Python'unu da ölçer (`ADIM 20`, "bütçenin Python'u da ölçmesi")

**16 Kasım.** Kod: `kentosd` `apps/api/src/processing_job.rs` ve yeni `apps/api/src/processing_job/tree.rs` (`measure`,
`own_group`, `end`, `ended`, `parse_proc_line`, `parse_ps_rows`, `tree_taken`); iş çalıştırıcısı `apps/jobrun/src/main.rs`
(`own_usage`) ve `audit.rs` (`rule_peak_kb`, `note_peaks`); bağımlılık `libc` (yalnız Unix, `kentosd`'de); sınamalar
`apps/api/tests/job_run/audit_budget.rs` ve `apps/api/src/processing_job.rs`'in birim sınamaları.

- **Bugünkü uygulama (okundu):** bütçe `ChildProcessor`'ın gözcüsüdür. Örnek başına (ilk 2 sn 25 ms'de, sonra 100 ms'de
  bir) `kentos-jobrun`'ın kendi `/proc/<pid>/stat`'ının utime+stime'ını ve `VmRSS`'ini okuyordu, bütçeyi aşınca yalnız o
  süreci `kill` ediyordu; rlimit, getrusage ya da süreç grubu yoktu. Kuralın Python'u iş çalıştırıcısının çocuğudur:
  işlemci süresi ve belleği bütçenin dışında kalıyor, iş çalıştırıcısı öldürülünce de Python sahipsiz yaşamaya devam
  ediyordu. Döngüye giren bir kuralı yalnız kuralın kendi zamanı (120 sn) durduruyordu.
- **Karar: aynı gözcü sürecin ağacını ölçer ve ağacı birlikte bitirir.** En küçük dürüst değişiklik: yeni bir sınır, yeni
  bir kod ya da yeni bir söz yok. Bütçeyi aşan iş yerleşik bir aracın aldığı sonucu alır: `cpu_limit` ve `memory_limit`,
  `over_budget`'ın aynı sözleriyle ("… sn işlemci süresi kullandı; bu işin işlemci bütçesi … sn. İş durduruldu, hiçbir
  şey yazılmadı …").
- **Ölçü (Linux, `/proc`):** ağaç her görevin `children` dosyasından (çekirdek tutmuyorsa her sürecin `stat`'ındaki
  ebeveynden) kurulur. İşlemci = ağaçtaki her sürecin utime + stime + cutime + cstime'ı: bitip beklenen çocuğun süresi
  bekleyeninkine geçer (iş çalıştırıcısı kuralın Python'unu kapatıp beklediğinde onun süresi cutime'dadır), iki kez
  sayılmaz. Bellek = ağaçtaki süreçlerin `VmRSS` toplamı; tepe değeri örnekler arasında en büyük toplamdır. Öbür Unix'lerde
  aynısı örnek başına tek bir `ps -A -S -o pid=,ppid=,rss=,time=` ile (önceden de örnek başına bir `ps` vardı).
- **Bitirme:** iş çalıştırıcısı kendi süreç grubunda başlatılır (`process_group(0)`). Bütçe aşılınca ya da Durdur'da grup
  `SIGKILL` alır, sonra çocuk beklenir: grup kimliği çocuk beklenmeden kimsenin olamaz, başka bir sürece dokunulmaz.
  Linux'ta çocuk kendi bittiğinde de beklenmeden önce grup bitirilir (`waitid` + `WNOWAIT`): geride kalan bir Python
  ondan uzun yaşamaz.
- **Rapor:** işin `usage`'ı (`ProjectAuditResult.usage`, `Processed.usage`) artık iş çalıştırıcısı ile kuralın Python'unu
  birlikte söyler: gözcünün ağaç örnekleri ve iş çalıştırıcısının sonda kendisinin söylediğinin büyüğü. İş çalıştırıcısı
  kendi işlemci süresine beklediği çocuklarınkini katar (cutime, cstime) ve tepe belleğine kuralın sürecinin tepesini
  ekler (havuz onu kapatmadan hemen önce `/proc/<pid>/status`'tan `VmHWM` okunur). İki tepe aynı anda olmayabileceği için
  bu toplam bir üst sınırdır.
- **Ölçümler (hata ayıklama derlemesi, 16 Kasım, sınamalar tek başına, iki koşu):** olağan bir kural (parsellerde ada
  numarası olmayanlar) işin tamamında 1.89-1.90 sn işlemci, en çok 506.5-506.8 MB (iş çalıştırıcısının ve Python'un
  tepeleri toplamı), 1.91-1.96 sn; kuralın Python süreci tek başına en çok 412.4 MB, çoğu SDK'nın hata ayıklama
  derlemesindeki yerel modülünün (789 MB'lık `_native`) sayfaları: yayın derlemesinde çok daha küçük. İşlemciyi yakan
  kural 3 sn'lik işlemci bütçesinde 3.07 sn'de durdu (iş çalıştırıcısının açılışı ve Python'un yüklenmesi dahil; aşıldıktan
  sonra en geç bir örnek aralığı, 25-100 ms). Belleği 16 MB'lık parçalarla dolduran kural, olağan kuralın tepesi + 300 MB
  olan 807 MB'lık bütçede 2.06-2.07 sn'de durdu; durduğu örnekte 902-962 MB ölçülmüştü (bir örnek aralığında 95-155 MB
  aşım). İkisinde de işin hiçbir süreci (iş çalıştırıcısı ya da Python) geride kalmadı. Değişiklikten önce (kod okunarak, koşulmadı)
  yakan kural bütçeye takılmazdı: bütçe yalnız iş çalıştırıcısının kendi işlemcisini görüyordu; kuralın 120 sn'lik
  zamanı onu durdururdu.
- **Neden rlimit değil:** `RLIMIT_AS` sanal belleği sayar (Python ve SDK'nın yerel modülü çok sanal alan ayırır, yanlış
  bulgu verir), `RLIMIT_CPU` süreç başınadır (iş çalıştırıcısı ile Python'un toplamı olan işin bütçesine eklenmez) ve
  sınırı aşan süreç sinyal ya da `MemoryError` ile biter, yerleşik araçların sonucu ve sözüyle değil. Gözcü zaten vardı ve
  bütçenin tek sahibi o.
- **Windows:** `/proc` ve `ps` yok: bugün de hiçbir şey ölçülmüyor (değişiklikten önce de öyleydi, yerleşik araçlar
  için de) ve yalnız iş çalıştırıcısı bitiriliyor. Doğru karşılığı iş nesnesidir (job object: çocuklar kendiliğinden
  girer, toplam işlemci süresi ve tepe bellek sorulur, kapanınca hepsi biter); ayrı iş.
- **Sınırlar ve kalan:** (1) Gruptan ve ağaçtan çıkan bir süreç (oturum açıp iki kez çatallanan) ölçüden de bitirmeden de
  kaçar; tam karşılığı Linux'ta işin cgroup'u, Windows'ta iş nesnesidir ve işletim sistemi yalıtımı işiyle gelir. (2)
  Sunucunun kendisi ölürse iş çalıştırıcısı standart girişinden bunu anlayıp çıkar; o an çalışan kuralın Python'u sahipsiz
  kalır ve kuralın zamanını da artık kimse saymaz. İş çalıştırıcısı güvensiz kod içermez (`forbid(unsafe_code)`), grubunu
  bitirmek için bir sistem çağrısı gerekir: ayrı iş. (3) Paylaşılan sayfalar (kütüphaneler) iki süreçte iki kez sayılır.
  (4) Hızlı bir ayırma bir örnek aralığında (25-100 ms) ne alabiliyorsa bütçeyi o kadar aşabilir. (5) macOS ve öbür
  Unix'lerde kendiliğinden biten çocuğun geride bıraktığı süreç bitirilmez (yalnız bütçede ve Durdur'da).
- **Sınamalar:** `kentosd`'nin birim sınamaları 8/8 (`/proc/<pid>/stat`'ın cutime/cstime'ıyla okunması, `ps`
  satırlarından ağacın toplamı, bu sürecin ölçülmesi, bütçe sözleri); işlemcinin gerçek iş çalıştırıcısıyla sınamaları
  26/26 (yerleşik araçların bütçe, Durdur ve çökme sınamaları değişmeden; yeni ikisi gerçek Python'la: yakan kural
  `cpu_limit` ile, belleği dolduran kural `memory_limit` ile duruyor ve `/proc`'ta işin klasörünü anan süreç kalmıyor;
  olağan kural aynı bellek bütçesine sığıyor); iş çalıştırıcısının denetim sınamaları 16/16 (kuralın Python sürecinin
  tepesi kapanmadan önce okunuyor). Python yoksa iki yeni sınama atlandığını söyler. Clippy temiz, `scripts/arch/deps.mjs`
  temiz (`libc` yalnız `kentosd`'de, Unix'te).

## §59. Çizgi ve sayı adımı: `divider`, `numberField`'ın `step`'i (`ADIM 22`, bileşen envanteri)

**16 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/ui_panel.rs` (`Component::Divider`, `NumberField::step`,
Kapı 1), şema ve Python kurucuları (`ui.divider()`, `ui.number_field(…, step=)`), masaüstü `panel_view.rs` (çizim),
`panels.rs` (`OpenPanel::step`, `PanelEvent::Step`), `dialogs.rs` (pencerenin alanları 15 tür); Vitrin 1.0.44.

- **Envanterin "bölücü"sü ve "adımlayıcı"sı bunlar değil:** kapsam raporunda bölücü sürüklenen ayraç (`sash`),
  adımlayıcı sihirbazdır (`wizard`); ikisi de hâlâ "KentOS'ta, bağlanmadı". Bu dilimin ikisi küçük ve ayrı: rapora
  "çizgi" satırı eklendi, sayı satırı `step`'i anıyor.
- **`divider`:** formun genişliğinde ince çizgi (bölüm başlığının çizgisinin stili); adı, değeri yok, isteğe bağlı
  `id` (kodun `visible` ile gizleyebilmesi için), `visibleWhen`, `tab`; satıra girmez. Pencerede de olur.
- **`step`:** sayı alanının yanında − ve +; değer (yoksa ilk değeri, o da yoksa alt sınırı, o da yoksa 0) adım kadar
  değişir, sınırların içinde kalır ve adımın ondalığına yuvarlanır (0,1'lik adımlar 0,30000000000000004'e kaymaz).
  Yazmak gibi bir değişikliktir: sayı alanının `onChange`'i yok, komut çalışmaz. Kapı 1: sıfırdan büyük sayı.
- **Vitrin 1.0.44:** Yakın yapılar'da açıklamanın altında çizgi, Uzaklık 5 m'lik adımlarla (1–1000 m). Gerçek pencere
  (47-nokta): + ile 25 → 30, Bul: "30 m içinde 3 yapı"; nokta seçicinin alanı yazısız hedef düğmesiyle genişledi.
- **SDK paketi:** rehber `divider`'ı ve `step`'i anar (her tür anılmalı); paket 48 000 baytın 21 bayt üstüne çıktı,
  rehberin yeni sözleri kısaltılarak sınıra döndü.
- **Sınamalar:** paylaşılan 1 (adsız ve kimliksiz çizgi; adımın sıfır olması yeriyle), masaüstü 1 (üst sınırda durma,
  ondalığın kaymaması, boştan alt sınırla başlama); rehberin ve kapsam raporunun türleri anması (var olan sınamalar).

## §60. Eklenti penceresinde sihirbaz: `ctx.dialog(…, steps=True)` (`ADIM 22`, envanterin "adımlayıcı"sı)

**16 Kasım.** Kod: `python/kentos/ext.py` (`CommandContext.dialog`'un `steps`'i), masaüstü
`apps/desktop/src/shell/extensions/dialogs.rs` (`dialog_panel`'in denetimi, `ext_dialog_step`, `ext_wizard_view`),
`panels.rs` (`OpenPanel::steps`, `OpenPanel::check_page`), `panel_view.rs` (sihirbazda sekme şeridi yok); stub ve SDK
rehberi; Vitrin 1.0.45.

- **Karar: KentOS'un `Wizard`'ı, eklentinin penceresinde.** Kapsam raporunun "adımlayıcı"sı KentOS'ta duran ama
  bağlanmamış `wizard` idi. Sihirbaz bir iletişim kutusudur (başlık, adımlar, Geri/İleri, bitirme düğmesi): eklentinin
  sorduğu pencereye (`ctx.dialog`) oturur. Dock'taki panelde sekmeler sürer; orada adım adım ilerlemek istenirse ayrı karar.
- **Karar: adımlar alanların sekmeleri.** `steps=True` ile alanların `tab`'ları (§43) ilk göründükleri sırayla adımlardır;
  sekmesiz alan her adımda görünür (açıklama gibi). Yeni bir tanım biçimi yok: aynı gövde `steps` olmadan sekmeli bir
  pencere olarak açılır. En az iki adım; tek sekme ya da sekmesiz gövde Kapı 1'in sözüyle reddedilir (kodun hatası).
- **İleri adımın alanlarını denetler** (`check_page`: zorunlu, sayı, sınır, dosya, nokta; Tamam'ın denetiminin o adıma
  düşen payı): yanlış alanın sözü altında, adım kalır. **Geri** denetlemez. Son adımda İleri'nin yerini pencerenin kendi
  düğmesi (`ok`, ör. "Yaz") alır ve bütün alanları Tamam gibi denetler; Vazgeç ve Esc İptal'dir (kod None alır).
- **Denemede de aynı:** ajanın denemesindeki pencere cevaplayıcısı (§52) `dialog_panel`'i kullandığı için sihirbazın
  kuralı orada da geçerli; cevap bütün alanların değerleridir.
- **Vitrin 1.0.45:** "Katmanı CSV'ye yaz" iki adımlı: Sütunlar (yazılacak öznitelikler, hepsi seçili gelir; ayırıcı) ve
  Dosya (kaydetme yeri). Gerçek pencere (41-kaydet): "Adım 1 / 2", parsel sütunu çıkarıldı, İleri; yol yokken Yaz
  zorunluluğu alanın altında söyledi, adım kaldı; yol yazılıp Yaz: dosyanın başlığı `x,y,ada`, dört parsel.
- **SDK paketi:** `ctx.dialog`'un imzasına `steps` girdi, rehber onu anar; yer açmak için rehberden bir örnek
  (`ui.choice`) ve bir yan not ("çalışırken gelen değişiklik…") çıktı. Paket 48 000 baytın altında.
- **Sınamalar:** masaüstü 1 (boş zorunlu alanla İleri adımı tutar ve alanı söyler, dolunca sonraki adım, Geri, sihirbaz
  çizimi, tek sekmenin reddi), Python 1 (`steps` pencereyle gider, verilmezse gönderilmez), gerçek pencere 41-kaydet.

## §61. Bulut denetim kutusu işletim sistemi düzeyinde: Landlock; UDP açık kalır (`ADIM 20`, kabulün "bulut kutusu")

**16 Kasım.** Kod: iş çalıştırıcısı yeni `apps/jobrun/src/kutu.rs` (`ARG`, `POLICY_ENV`, `Policy`, `allowed`, `wrap`,
`level`, `enter`; Linux'ta `ruleset`, `restrict`), `apps/jobrun/src/main.rs` (kutu kipi ilk iş), `audit.rs`
(`Confinement`, `run_audit_in`: kuralın süreci kutudan başlar; ilke, özet ve söz); `kentosd`
`apps/api/src/processing_job.rs` (`AUDIT_CONFINE_ENV` çocuğa geçer); bağımlılık `landlock = "=0.4.7"` (yalnız iş
çalıştırıcısında, yalnız Linux'ta); sınamalar `apps/jobrun/src/kutu/tests.rs`, `apps/jobrun/src/audit/kutu_tests.rs`.

- **Bugünkü uygulama (okundu):** kuralın Python'u iş çalıştırıcısının çocuğuydu: ortamı boş, `-I` ile yalıtılmış, ağ
  yeteneği isteyen paket reddediliyordu (§42, §50). Süreç ise sunucunun kullanıcısının bütün haklarıyla çalışıyordu:
  işin klasöründeki çizimi ve isteği, sunucunun ve kullanıcının dosyalarını okuyabilir, yazabilir, TCP ve UDP ile
  dışarı çıkabilir, sunucuya sinyal gönderebilirdi. "Ağa ve yerel dosyalara erişemez" yalnız manifestin sözüydü.
- **Karar: kuralın süreci Landlock alanında, iş çalıştırıcısının kutu kipiyle.** `kentos-jobrun --kutu --oku <yol>…
  --yaz <yol>… --tmp <klasör> -- <python> <bağımsız değişkenler>` yeni ve tek iş parçacıklı bir süreçtir: kural
  kümesini kurar, kendini kısıtlar (`restrict_self`; `no_new_privs` ile, setuid programlar yetki kazanamaz) ve
  yorumlayıcıyı çalıştırır (`exec`, ortam boş, yalnız `TMPDIR`). Alan `exec`'ten geçer, kuralın başlattığı her sürece
  kalıtılır ve kaldırılamaz. İş çalıştırıcısının kendisi dışarıda kalır: bütçe ve kullanım için `/proc`'u okur (§58),
  işin cevabını yazar. Kendini kısıtlasaydı ya bunları kaybederdi ya da kurala `/proc`'u açardı. İş çalıştırıcısı
  güvensiz kod içermez (`forbid(unsafe_code)`): çatallanma ile `exec` arasına kanca (`pre_exec`) yerine kendi
  programının ayrı bir kipi seçildi; ne güvensiz kod ne de çatallanmış süreçte bellek ayırma gerekir. Kutu kipi
  `exec` ettiği için kuralın Python'u aynı süreçtir: bütçenin ağacı (§58) onu eskisi gibi görür ve bitirir.
- **Bağımlılık:** çalışma alanında da std'de de Landlock'a erişen bir şey yok. `landlock` Landlock projesinin Rust
  bağlayıcısıdır (MIT ya da Apache-2.0), güvenli bir arayüz verir ve ABI 9'a (Linux 7.1) kadar bilir; çektiği
  `enumflags2`, `thiserror` ve `libc` kilitte zaten vardı (kilide yalnız `landlock` eklendi). Çalışma alanının
  kuralıyla tam sürüme sabitlendi; yalnız `apps/jobrun`'da, yalnız `target_os = "linux"` için. `scripts/arch/deps.mjs`
  temiz.
- **Dosyalar (`allowed`):** okuma ve çalıştırma yalnız şunların altında: paketin açıldığı klasör (`extension/`);
  hazırlanmış ortam (klasörü; `pyvenv.cfg`'nin `home`'unun gösterdiği temel kurulumun `lib`'leri; ortamın bir `.pth`
  dosyasının eklediği klasör, yalnız içinde KentOS'un kütüphanesi (`kentos/__init__.py`) varsa: geliştirmede
  düzenlenebilir kurulmuş SDK); yorumlayıcının çözülmüş dosyası ve kurulumunun `lib`'leri; sistemin kütüphane klasörleri
  (`/usr/lib`, `/usr/lib64`, `/lib`, `/lib64`, `/usr/local/lib`); `/etc/ld.so.cache`, `/dev/urandom`. Yazma yalnız işin
  kendi karalamasında: `pyc` (bayt kodu) ve `tmp` (kuralın `TMPDIR`'i), bir de `/dev/null`. İşin klasöründeki çizim
  (`snapshot.kcad`), istek ve paket dosyası okunamaz; `/proc`, `/sys`, `/etc`'nin geri kalanı, kullanıcının klasörleri
  okunamaz. Bir programı da yalnız bu klasörlerden çalıştırabilir ve çalıştırdığı her program kutuyu kalıtır.
  **`.pth` süzgeci bilerek:** ortam yazarın kilidindeki bağımlılıklarla kurulur ve bir bağımlılığın `.pth` dosyası
  istediği mutlak yolu yazabilir (`/home`, `/etc`); her `.pth` satırını açmak kutuyu yazarın eline verirdi. Yayında SDK
  ortamın içinde, kurulumun tekerleğinden gelir (`.pth` gerekmez); göreli `.pth` satırları zaten ortamın içindedir.
- **Ağ ve süreçler:** TCP'de bağlanma ve dinleme yasak (ABI 4, hiçbir bağlantı noktasına izin yok). Sinyaller ve soyut
  UNIX soketleri alana kapsanır (ABI 6): kural sunucuyu ya da alan dışındaki bir süreci sinyalleyemez, alan dışındaki
  soyut sokete bağlanamaz; alan dışındaki süreçleri izleyemez de (ptrace, Landlock'un kendisi). Yol adlı UNIX soketine
  bağlanma çekirdek ABI 9'u biliyorsa yasak (karalamadaki kendi soketleri dışında).
- **Kapatılamayan yerde (Landlock yok, ABI 4'ün altı, Linux değil) ilke sunucunun ayarıdır:** `KENTOS_AUDIT_CONFINE`.
  `require` (varsayılan; ayarsız ve tanınmayan değer de böyle okunur): kural çalışmaz; iş, paket açılmadan ve çizim
  ölçülmeden `tool_unavailable` ile "Bulut denetim kutusu kapatılamadı: …. Sunucu kuralı kutusuz çalıştırmaz; yönetici
  KENTOS_AUDIT_CONFINE ayarını report yaparsa …" der. `report`: kural kutusuz çalışır, işin özeti "(kutu kapatılamadı:
  …)" ile biter (iş çalıştırıcısı hata çıktısına da yazar). Varsayılan `require`, çünkü planın kabulü ("bulut kutusu ağa
  ve yerel dosyalara erişemez") ya tutar ya da hiçbir kural çalışmaz; daha zayıfını yönetici bilerek seçer. `kentosd`
  ayarı çocuğa geçirir (çocuğun ortamı başka türlü boştur). Kutu kipi kutuyu kapatamazsa (beklenmez: önce yoklanır) ya
  da yorumlayıcıyı çalıştıramazsa `KENTOS-KUTU` imiyle söyler ve iş, kuralın hatası (`tool_failed`) değil, aracın
  yokluğu (`tool_unavailable`) olarak biter.
- **Söz:** kutu kapandığında iş çalıştırıcısı ne kapandığını hata çıktısına yazar; bu makinede (Linux 7.0, ABI 8):
  "dosyalar ve TCP kapalı; sinyaller ve soyut UNIX soketleri kutuda; yol adlı UNIX soketleri açık (ABI 9 yok); UDP ve
  öteki soket türleri açık (Landlock kapsamaz)". Sınama bu sözü kutudaki kuralın gördüğüyle karşılaştırır: açık denen
  açık, kapalı denen kapalı çıkmalı. Kapanan kutu işin özetine bir şey eklemez, rapora da (masaüstünün raporuyla bayt
  bayt aynı kalır); söylenmeyen bir kapalılık iddia edilmez. `kentosd` iş çalıştırıcısının hata çıktısını yalnız çöken
  işin sözünde gösterir: kutunun düzeyini açılışta kendi günlüğüne yazması ayrı iş.
- **Kapanmayanlar (bu makinede ölçüldü):** (1) **UDP** ve TCP dışındaki soket türleri: Landlock'un hiçbir ABI'si
  kapsamıyor. Kutudaki kural 127.0.0.1'deki bir UDP soketine veri gönderdi, sınama aldı; bir DNS sorgusu da dışarı veri
  taşıyabilir. (2) **Yol adlı UNIX soketleri ABI 9'un altında** (bu makine ABI 8): kutudaki kural işin klasöründeki,
  okuyamadığı bir sokete bağlandı. Sunucunun kullanıcısının ulaşabildiği her yerel soket kurala da açıktır: ör.
  veritabanının yerel soketi (peer kimlik doğrulamasında sunucunun kullanıcısıyla), kapsayıcı motorunun soketi. ABI 9'a
  kadar önlem işletim sistemi yönetiminindir: `kentosd`'yi yerel soketlerin arkasında yetkisi olmayan bir kullanıcıyla
  çalıştırmak. (3) Herhangi bir yolun **varlığı ve üst verisi** (`stat`): Landlock içeriği ve değişikliği kısıtlar, yol
  yürüyüşünü değil. (4) İşlemci ve bellek kutunun değil bütçenin işidir (§58). (5) Masaüstü kapsam dışı: eklentiler
  orada kullanıcının haklarıyla çalışır (PY-RUN-19/20). (1) ve (2)'nin kalıcı çözümü bir seccomp süzgecidir:
  `socket()`'i reddetmek (kuralın hiçbir sokete ihtiyacı yok; yalnız `socketpair` asyncio için kalır). İkinci bir
  bağımlılık ister (güvensiz kod yok kuralıyla): ayrı iş.
- **Ölçüm (hata ayıklama derlemesi, 16 Kasım):** kutu kipinin kendi maliyeti (`/usr/bin/true`'nun çevresinde, 30 koşunun
  ortancası) 6.8 ms: 6.1 ms'si iş çalıştırıcısının hata ayıklama derlemesinin (770 MB) açılışı, Landlock'un kurulması
  ile `exec` 0.7 ms. Ortamın Python'unun SDK'nın konağını yüklemesi (`import kentos.ext_host`, sırayla 10'ar koşu,
  ortanca): kutusuz 988 ms duvar / 987 ms işlemci, kutuda 991 / 991 ms (+3 / +4 ms). Olağan kuralın işi baştan sona
  (§58'in ölçümü, sınama tek başına, iki koşu): 1.91-1.92 sn işlemci (kutusuz, §58: 1.89-1.90), en çok 500.3-500.9 MB
  (§58: 506.5-506.8), 1.96 sn; kuralın Python süreci en çok 412.2 MB (§58: 412.4). Bütçe kutuyla da tutuyor: işlemciyi
  yakan kural 3 sn'lik bütçede 3.07 sn'de, belleği dolduran 801-805 MB'lık bütçede 2.06-2.16 sn'de durdu (§58: 3.07;
  2.06-2.07).
- **Sınamalar:** iş çalıştırıcısının `audit` ve `kutu` sınamaları 22/22. Eski 16'sı (örnek kural, ulusal kurallar)
  artık kutuda koşuyor ve aynı bulguları veriyor; masaüstünün kutusuz raporu ile işin kutudaki raporu bayt bayt aynı
  (`the_desktop_and_the_job_give_the_same_report`: planın kabulü kutuyla da tutuyor). Yerine konan kural sınaması
  (`a_job_runs_the_package_s_rule_on_the_measured_facts`) kutunun komut satırını da sınar: paket okunur, işin klasörü
  okunmaz, karalama yazılır. Yeni altısı: izinli yollar (bir bağımlılığın `.pth`'si hiçbir şey açmaz) ve kutu kipinin
  komut satırı; anlamadığı sözü reddetmesi; gerçek Python'la kutudaki kural (TCP, `/etc/hostname`, kullanıcının bir
  dosyası ve işin çizimi `PermissionError`, işin klasörüne yazmak `PermissionError`, karalamaya yazmak açık; UDP
  gönderiliyor ve sınama alıyor, yol adlı UNIX soketine bağlanılıyor: kutunun sözüyle aynı; aynı denemeler sınamanın
  kendi sürecinden başarılı); kutunun açılış maliyeti; Python'suz ve her sistemde koşan ikisi: kutu kapanmayınca
  `require` paketi açmadan reddeder, hiçbir kural koşmaz, `report` yorumlayıcıyı kendisi başlatır ve özet "(kutu
  kapatılamadı: …)" der; yorumlayıcısını bulamayan kutu kipi imiyle söyler ve iş `tool_unavailable` olur. Gerçek
  Python'lu ikisi Python yoksa ya da kutu kapanmıyorsa atlandığını söyler. `kentosd`'nin işlemci sınamaları 26/26
  (denetim işleri ve §58'in bütçe sınamaları kutudan geçiyor), birim sınamaları 8/8. Clippy (`--tests --no-deps`),
  rustfmt ve `scripts/arch/deps.mjs` temiz.
- **Sınama makinesinde kutu kapanmıyorsa:** gerçek Python'la koşan denetim sınamaları (iş çalıştırıcısının ve
  `kentosd`'nin) `require` altında reddedilir; o makinede `KENTOS_AUDIT_CONFINE=report` ile koşulur, kutunun kendi
  sınamaları atlandığını söyler. İlkeyi sınayan ikisi her sistemde ve Python'suz koşar.
- **Kalan:** (1) UDP ve ABI 9'un altında yol adlı UNIX soketleri (seccomp ile `socket()` reddi, yukarıda). (2)
  `kentosd`'nin açılışta kutunun düzeyini günlüğüne yazması. (3) Windows'ta ve macOS'ta kutu: bugün `require` orada
  kuralı çalıştırmaz. (4) Yayın derlemesinde ölçüm (iş çalıştırıcısının açılışı, buradaki 6.1 ms'nin çoğu, küçülür).

## §62. Panelden çizgi ve alan: `shapePicker` (`ADIM 22`, envanterin "panelden çizgi/alan seçimi")

**16 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/ui_panel.rs` (`Component::ShapePicker`, `PickShape`,
Kapı 1), `ui_panel/rules.rs`, şema ve Python kurucusu (`ui.shape_picker`), SDK rehberi; masaüstü
`apps/desktop/src/shell/extensions/panel_points.rs` (`shape_field`, `shape_words`, `ext_panel_draw_shape`,
`ext_panel_shape_drawn`), yeni `panel_marks.rs` (haritadaki işaret), `panels.rs` (değer, zorunluluk,
`waits_for_value`), `panel_session.rs` (`ctx.panel.set`), `panel_view.rs`, `runner.rs` (yakalamanın zinciri),
`cad/marks.rs` (`picked`); Vitrin 1.0.46.

- **Karar: harita aracının yakalaması, panelin alanında.** `shape`: `line`, `polygon`, `rectangle`, `circle`. Haritada
  çiz (hedef düğmesi) KentOS'un yakalamasını (`Capture`, harita araçlarınınki: kenetlenme; Enter ya da çift tık bitirir,
  Esc vazgeçer) o türle başlatır, komut satırı panelin ve alanın adını söyler; ✕ alanı bırakır. Yazılarak verilmez:
  bir alanın köşelerini panelde yazmak işe yaramaz, noktada (§57) yazım vardı.
- **Karar: değer GeoJSON, çizimin düzleminde.** Kod `ctx.params["alan"]`'ı `{"type": "Polygon", "coordinates":
  [[[x, y], …]]}` (halka kapalı) ya da `LineString` alır, boşsa None; shapely'nin `shape()`'i doğrudan okur. Dikdörtgen
  dört köşeli çokgendir; daire 64 köşeli çokgendir ve ayrıca `circle: {center, radius}` taşır. Kod `ctx.panel.set(id,
  value=…)` ile alanın türünden bir şekil yazar ya da None ile bırakır; başka tür (alan isteyen alana çizgi) "value bir
  GeoJSON Polygon olmalı" diye reddedilir.
- **Alanın sözü:** "4 köşe, 673,3 m²", "2 nokta, 5,0 m", "yarıçap 5,0 m, 78,5 m²". İlk sürüm "Alan: 4 köşe, …" diyordu:
  etiket neyin çizildiğini zaten söylüyor. Sayı birimine bölünmez boşlukla bağlı: dar dock'ta satır virgülden sonra
  kırılır, "m²" alt satıra tek başına düşmez (gerçek pencere: üç satırdı, iki oldu).
- **Karar: çizilen alan, panel gösterildikçe haritada.** Gösterilen panelin şekil seçicilerindeki şekil kesikli ve vurgu
  renginde, alan hafif dolgulu; nokta seçicinin noktası halkayla (§57'ninki de). İlk sürümde alan çizim bitince
  kayboluyordu: tablo bir alanı anlatıyor, alan görünmüyordu (gerçek pencere). Yalnız gösterilen panelinki (iki panelin
  alanı karışmaz); gizlenen bileşen (kuralı ya da kodu) bir şey göstermez; çizime bir şey yazılmaz.
- **Karar: değişiklik eksik değerle çalışmaz ve bir şey demez** (bütün paneller için). `onChange`'li bir değişiklik,
  panelin sorduğu bir alan boşken (✕ ile bırakılan alan, alan çizilmeden işaretlenen kutu) komutu başlatmaz, alanı
  kırmızıya döndürmez; eski değerlere verilen cevap (komutun tablosu ve sözü) gider. İlk sürümde ✕'e basınca "“Alan”:
  çizilmedi" iki yerde kırmızıyla söyleniyor, eski alanın satırları duruyordu (gerçek pencere). Eksiği düğme söyler,
  eskisi gibi. Vitrin'in öbür panellerinde zorunlu ve `onChange`'li tek alan nokta seçicisidir: davranışları değişmedi.
- **Pencerede yok** (§57'nin gerekçesi: pencere cevaplanana dek pencereyi tutar). **Satıra girmez:** çizilenin sözü dar
  dock'ta iki satıra yayılır, yanında başka hücreye yer kalmaz.
- **Kapı 1:** `shapePicker` `id`, `label`, `shape` (zorunlu), `onChange`, `required` alır; tanınmayan `shape` yerinde
  söylenir (`/body/<n>/shape`). Kural dili alanı metin olarak okur, boşsa boş.
- **Vitrin 1.0.46:** "Alandaki yapılar" paneli (Vitrin › Panel): Alan (`shapePicker`, `polygon`, `onChange`), "Yalnızca
  tamamı içinde kalanlar" (`checkbox`, `onChange`), tablo; `alandaki-yapilar` komutu alanı `shape()` ile okur, alana
  giren yapıları içinde kalan taban alanı ve payıyla büyükten küçüğe listeler. Gerçek pencerede (48-alan): dört köşe
  tıklanıp Enter: "4 köşe, 673,3 m²", "673 m²'lik alanda 3 yapı" (150 m² %100, 92,6 m² %64, 71,9 m² %18), alan haritada;
  kutu işaretlenince 1 yapı; ✕: alan haritadan ve tablodan gitti, söz yok.
- **Sınamalar:** paylaşılan 1 (türün yerinde söylenmesi, GeoJSON türleri), rehberin ve §25'in türü anması (var olan
  sınamalar); masaüstü 3 (çizilenin değeri ve sözü, zorunluluk, kodun `set`'i ve yanlışı, ✕; gösterilen panelin alanı ve
  noktası işaretli, çizgi açık, gizlenen ve yanlış yazılan işaretsiz; değişikliğin eksik alanla sessiz beklemesi, eski
  satırların gitmesi, düğmenin eksiği söylemesi); Vitrin'in sayıları (15 komut, 5 panel) ve şerit etiketleri; gerçek
  pencere 48-alan.

## §63. Bulut denetim kutusunda soket yok: seccomp süzgeci (`ADIM 20`, §61'in açık bıraktığı ağ)

**16 Kasım.** Kod: iş çalıştırıcısı `apps/jobrun/src/kutu.rs` (`NO_FILTER`, `Landlock`, `Plan`, `plan`, `here`,
`landlock`, `filter`; `wrap` süzgeci söyler, `enter` kurar), yeni `kutu/seccomp.rs` (`program`, `available`,
`install`), `kutu/linux.rs` (§61'in Landlock'u kendi dosyasında; `level` yerine `landlock`), `audit.rs` (`run_audit_in`
kutunun planını alır); bağımlılık `seccompiler = "=0.5.0"` ve çağrı numaraları için `libc` (yalnız iş
çalıştırıcısında, yalnız Linux'ta, yalnız x86_64 ve aarch64 için); sınamalar `kutu/tests.rs`, `audit/kutu_tests.rs`,
`audit/tests.rs`.

- **Bugünkü uygulama (§61, ölçüldü):** Landlock TCP'yi kapatıyor; UDP ve öteki soket türleri açıktı (kutudaki kuralın
  UDP'si sınamaya ulaştı), yol adlı UNIX soketleri ABI 9'un altında açıktı (bu makine ABI 8: kural, okuyamadığı bir
  klasördeki sokete bağlandı). io_uring de açıktı: bu makinede `kernel.io_uring_disabled = 0` ve bir halka soketi
  kendisi açabilir (`IORING_OP_SOCKET`), `socket(2)` çağrılmadan.
- **Karar: kutu kipi Landlock'tan sonra, `exec`'ten önce bir seccomp süzgeci kurar.** Reddedilen çağrı `EACCES` ile
  döner, Landlock'un reddi gibi (Python'da `PermissionError`):
  - `socket(2)`: her aile için.
  - `socketpair(2)`: yalnız UNIX akış ya da sıralı paket çifti olarak açık. asyncio'nun kendi borusu (self-pipe) bu
    çifttir ve iki ucu da kutunun içindedir. UNIX datagram çifti reddedilir: bağlı olsa bile adıyla herhangi bir yol
    adlı datagram soketine gönderebilir (ör. sistem günlüğünün `/dev/log`'u); UNIX dışı aileler de reddedilir. Süzgeç
    türe bayraklarından ayırarak bakar (`SOCK_NONBLOCK`, `SOCK_CLOEXEC`: `type & 0xf`).
  - `io_uring_setup(2)`: halka kurulamaz, soketi o da açamaz.
  - x86_64'te aynı üç çağrı x32 numaralarıyla da (`0x40000000 |`): x32 ABI'siyle derlenmiş bir çekirdek onlara
    cevap verir (bu makinenin çekirdeğinde x32 yok; süzgeç çekirdeğe bakmadan kapatır).
  - Başka bir mimarinin çağrısı (32 bit uyumluluk) süreci bitirir: süzgecin ilk sınaması.
  - Gerisi açık: süzgeç bir ret listesidir, izin listesi değil. Python'un ve kütüphanelerin çağrılarını tek tek saymak
    her sürümde kırılırdı; kutunun dosya ve süreç tarafı Landlock'ta.

  Süzgeç `exec`'ten geçer, kuralın başlattığı her sürece kalıtılır ve kaldırılamaz (`no_new_privs`: Landlock zaten
  açmıştı). TCP artık iki kilitle kapalı: soket açılamaz, açılsa Landlock bağlatmazdı. Sonucu kural yazarı da görür:
  asyncio çalışır; soket açan her şey (`multiprocessing`'in yöneticisi, sistem günlüğüne yazan işleyici)
  `PermissionError` alır, ad çözümü de çalışmaz. Bulut kuralının bunlara ihtiyacı yok; masaüstünde değişen bir şey yok.
- **Bağımlılık:** çalışma alanında da std'de de seccomp süzgeci kuran bir şey yok; elle yazılmış BPF ve `prctl` ise
  güvensiz kod isterdi (iş çalıştırıcısı `forbid(unsafe_code)`). `seccompiler` (Apache-2.0 ya da BSD-3-Clause) saf
  Rust'tır, sistem kütüphanesi istemez, tek bağımlılığı `libc`'dir (JSON özelliği kapalı); süzgeci kurallardan derler
  (`SeccompFilter` → `BpfProgram`) ve güvenli bir arayüzle kurar (`apply_filter`). Çalışma alanının kuralıyla tam
  sürüme sabitlendi; yalnız `apps/jobrun`'da, yalnız `target_os = "linux"` ve x86_64 ya da aarch64 için. Çağrı ve
  sabit numaraları hedefin kendi `libc`'sinden (`SYS_socket`, `AF_UNIX`, `EACCES`; yalnız sabitler), sürümü çalışma
  alanınınki. Kilide yalnız `seccompiler` eklendi. `scripts/arch/deps.mjs` temiz.
- **Mimariler:** x86_64'te 41, 53, 425; aarch64'te 198, 199, 425. Başka bir mimaride ya da Linux dışında süzgeç kurulmuş
  sayılmaz ve bu açıkça söylenir: "seccomp süzgeci yalnız Linux'ta x86_64 ve aarch64 için var; bu sistem linux
  riscv64".
- **Seçim: süzgeç kurulamayan kutu "kapatılamayan kutu" sayılır,** Landlock'un yokluğu gibi. Süzgeç yoksa UDP ve
  öteki soket türleri açık kalır; kutunun sözü ise ağ yok. `require` (varsayılan) kuralı çalıştırmaz, `tool_unavailable`
  ile neden çalışmadığını söyler. `report` kuralı kutunun kapanan kısmıyla çalıştırır: Landlock kapanıyorsa yalnız
  Landlock'la (kutu kipi `--suzgecsiz`: §61'in kutusu, oradan geriye gidilmez), Landlock da yoksa kutusuz (§61 gibi).
  İşin özeti neyin açık kaldığını söyler: "(kutu tam kapatılamadı: seccomp süzgeci …; dosyalar ve TCP kapalı; …; UDP ve
  öteki soket türleri açık)" ya da "(kutu kapatılamadı: …)". Karar tek yerde, sınanabilir: `kutu::plan(landlock,
  filter, policy)`. Süzgecin kurulabileceği, sunucu kendine kurmadan sorulur: mimari, çekirdeğin
  `/proc/sys/kernel/seccomp/actions_avail`'i (`allow`, `errno`, `kill_process`; Linux 4.14'ten beri) ve süzgecin
  derlenmesi. Kutu kipi yine de kuramazsa `KENTOS-KUTU` imiyle söyler ve iş `tool_unavailable` olur (§61).
- **Söz:** bütün kutu iş çalıştırıcısının hata çıktısına şunu yazar: "dosyalar kapalı; ağ kapalı (soket açılamaz):
  TCP, UDP, yol adlı ve soyut UNIX soketleri, öteki türler, io_uring; yalnız kutunun içinde kalan UNIX akış çifti
  açılır; sinyaller kutuda". Landlock ABI 6'nın altındaysa son kısım "sinyaller açık (Landlock ABI 6 yok)" olur: soyut
  UNIX soketleri artık süzgeçle kapalı, sinyaller değil. Sınama bu sözü kuralın gördüğüyle karşılaştırır.
- **Kapanmayanlar:** (1) Landlock ABI 6'nın altında sinyaller: kural sunucunun kullanıcısının süreçlerini
  sinyalleyebilir (`kill(-1, …)`). (2) Yolların varlığı ve üst verisi (`stat`, §61). (3) Kutunun içi: kuralın kendi
  süreçleri kendi akış çiftleri üzerinden konuşabilir. (4) Windows ve macOS'ta kutu yok (`require` orada kuralı
  çalıştırmaz).
- **Ölçüm (hata ayıklama derlemesi, 16 Kasım):** kutu kipi `/usr/bin/true`'nun çevresinde (30 koşunun ortancası):
  yalnız Landlock'la 6.91 ms, bütün kutuyla 7.30 ms; süzgecin payı (derlenmesi ve kurulması) 0.38 ms. Kutunun kendi
  maliyeti 6.76 ms, çoğu iş çalıştırıcısının hata ayıklama derlemesinin açılışı (6.25 ms). Ortamın Python'unun
  SDK'nın konağını yüklemesi (`import kentos.ext_host`, sırayla 10'ar koşu, ortanca, duvar = işlemci): kutusuz 977 ms,
  Landlock'ta 989 ms, bütün kutuda 986 ms; süzgecin payı ölçülemiyor (-3 ms, gürültü), her çağrıda koşan süzgecin
  maliyeti görünmüyor. Olağan kuralın işi baştan sona (sınama tek başına, iki koşu; §58 ve §61'in ölçümü): 1.90-1.93 sn
  işlemci (§61: 1.91-1.92; kutusuz §58: 1.89-1.90), en çok 497.9-503.2 MB, 1.96 sn. Bütçe bütün kutuyla da tutuyor:
  işlemciyi yakan kural 3 sn'lik bütçede 3.07 sn'de, belleği dolduran 803 MB'lık bütçede 2.06 sn'de durdu.
- **Sınamalar:** iş çalıştırıcısının `audit` ve `kutu` sınamaları 23/23. Gerçek Python'la
  `the_whole_box_opens_no_socket_and_no_file_beyond_the_rule_s_own`: bütün kutudaki kuralda TCP, UDP, yol adlı UNIX
  soketi, UNIX datagram çifti ve io_uring halkası `PermissionError`; asyncio'nun döngüsü (kendi akış çifti) çalışıyor;
  kutunun dışındaki dosyalar ve işin kendi dosyaları `PermissionError`, karalama yazılıyor; sınamanın TCP, UDP ve UNIX
  dinleyicilerine hiçbir şey ulaşmadı. Aynı kural kutunun dışında hepsini açıyor (bu makinede io_uring de) ve üçü de
  dinleyicilere ulaşıyor: farkı kutu yapıyor. Sözün UDP'yi ve yol adlı UNIX soketlerini kapalı saydığı da sınanıyor.
  Python'suz ve her sistemde: planın bütün tablosu
  (`a_box_that_is_not_whole_runs_only_under_report_and_says_what_is_open`: Landlock ve süzgeç → bütün kutu; süzgeç
  yok → `require` reddeder, `report` Landlock'la ve "UDP ve öteki soket türleri açık" der; Landlock yok → kutusuz;
  ikisi de yok → iki neden birlikte); işin üç yolu
  (`where_the_box_cannot_be_whole_…`: `require` paketi açmadan reddeder, Landlock'suz `report` yorumlayıcıyı kendisi
  başlatır ve "(kutu kapatılamadı: …)", süzgeçsiz `report` kutu kipini `--suzgecsiz` ile başlatır ve "(kutu tam
  kapatılamadı: …)" der); kutu kipinin komut satırında `--suzgecsiz` yalnız istenince var. Eski sınamalar bütün kutuda
  da geçiyor: masaüstünün kutusuz raporu ile işin bütün kutudaki raporu bayt bayt aynı, ulusal kurallar aynı bulguları
  veriyor. `kentosd`'nin işlemci sınamaları 26/26 (denetim işleri ve §58'in bütçe sınamaları bütün kutudan geçiyor).
  Clippy (`--tests --no-deps`), rustfmt ve `scripts/arch/deps.mjs` temiz.
- **Kalan ve öneri:** (1) Süzgeçle ağ için Landlock'un ABI 4'üne gerek kalmadı. Dosyalar için ABI 3 yeter (`truncate`
  ABI 3'te geldi; altında yol adıyla `truncate(2)` Landlock'un dışındadır); süzgeç yol adıyla `truncate(2)`'yi de
  reddederse ABI 1 yeter ve kutu 5.15 ile 6.1 LTS çekirdeklerinde de bütün kapanır. Bugün asgari ABI 4 (değişmedi; ayrı
  karar). (2) ABI 6'nın altında sinyaller (süzgeçle `kill` ailesini kendi süreciyle sınırlamak). (3) `kentosd`'nin
  açılışta kutunun düzeyini günlüğüne yazması (§61).

## §64. Sayı alanında birim: `numberField`'ın `units`'i (`ADIM 22`, envanterin "birim seçici"si)

**16 Kasım.** Kod: paylaşılan yeni `crates/shared/extensions/src/ui_panel/units.rs` (`UNITS`, `unit`, `quantity_of`,
`tidy`, Kapı 1'in `check`'i), `ui_panel.rs` (`NumberField::units`), şema ve Python kurucusu (`ui.number_field(…,
units=)`), SDK rehberi; masaüstü yeni `apps/desktop/src/shell/extensions/panel_units.rs` (`typed_number`,
`units_hint`, `units_words`, `show_in_own_units`), `panels.rs` (değer, kural değeri, adım, sınırın sözü),
`panel_view.rs` (boş alanın ipucu); ortak bileşen `crates/ui/src/widget/number.rs` (birimin üssü); Vitrin 1.0.47.

- **Karar: birim seçilmez, yazılır: KentOS'un sayı alanlarının dili.** `units` alanın kendi birimi (`unit`) dahil 2-4
  birimdir; kullanıcı sayıyı bunlardan biriyle yazar ("0,03 km", "2 dönüm", "1 km + 20 m"), birimsiz sayı alanın
  kendi birimindedir. Okuyan KentOS UI'nin sayı girişinin çözümleyicisidir (`kentos_ui::widget::number::evaluate`):
  KentOS'un her sayı alanı ("1 m + 20 cm", Türkçe sayılar, ifadeler) böyle yazılır, eklentinin paneli ayrı bir dil
  konuşmaz. İlk sürüm sayının yanına bir birim seçimi koymuştu: 300 piksellik dock'ta seçim ve − + düğmeleri sayının
  yerini aldı, sayı kutusu sıfır genişliğe indi, + kenardan kesildi (gerçek pencere).
- **Karar: kod alanın kendi biriminde alır.** `value`, `min`, `max`, `step` ve kodun `ctx.panel.set`'i de onunla
  yazılır; kod hiç çevirmez. Çeviri on iki anlamlı haneye yuvarlanır (1,1 km 1100 m, 1100,0000000000002 değil). Komut
  başlarken alan değeri kendi biriminde yazar ("0,03 km" "30" olur, yanında "m"): KentOS'un sayı alanı da girilen
  değeri kendi biriminde gösterir; yazılan birim alanın biriminin yanında iki kez okunuyordu ("0,03 km m").
- **Sözler:** sınırı aşan değer alanın birimiyle söylenir ("“Uzaklık” 1 m ile 1000 m arasında olmalı."; km ile yazılan
  sayı metreyle denetlenir), okunamayan değer alanın birimlerini sayar ("bir sayı ya da birimiyle yazılmış bir sayı
  olmalı (m, km)"). Boş alan aldığı birimleri söyler ("m ya da km"). `units`'siz alan eskisi gibi düz sayı okur;
  sözleri değişmedi.
- **Birimler KentOS'unkiler:** uzunluk mm, cm, m, km; alan m², dönüm (1 000 m²), ha (10 000 m²); açı ° ve grad (400
  bir tur): projenin ayarları ve raporlarındakiler. Kapı 1: hepsi bilinen, aynı şeyi ölçen, her biri bir kez, `unit`
  aralarında; yanlış olan yerinde söylenir (`/body/<n>/units/<i>`, `/body/<n>/unit`).
- **Ortak bileşen düzeltildi:** çözümleyici `m²`'yi okuyamıyordu ("²" harf sayılmadığı için birim "m" ile "²" diye
  bölünüyordu); "m2" de yazılmıyordu. Artık birimin üssü yazımın parçasıdır ve düz rakamla da yazılır (m2, m3; yalnız
  takımda öyle bir birim varsa, hemen ardından tek rakam). KentOS'un kendi sayı alanlarının takımlarında alan birimi
  yoktu: davranışları değişmedi ("5 m2" uzunluk takımında eskisi gibi yazım hatası).
- **SDK paketi:** rehber `units`'i anar; yer açmak için saat alanının sözü kısaldı. Paket 47 986 bayt (sınır 48 000).
- **Vitrin 1.0.47:** Yakın yapılar'ın Uzaklık'ı `unit: "m"`, `units: ["m", "km"]`; açıklama "Uzaklık km ile de
  yazılır: 0,03 km" der. Gerçek pencerede (49-birim): nokta seçildi (25 m: 3 yapı), "0,03 km" yazılıp Bul: "30 m içinde
  3 yapı" (6,4, 6,6, 20,6 m), alan "30 m"; "2 km": "“Uzaklık” 1 m ile 1000 m arasında olmalı.", komut çalışmadı.
- **Sınamalar:** paylaşılan 2 (birimlerin büyüklüğü ve yuvarlama; Kapı 1'in dokuz durumu), ortak bileşen 1 (m², m2,
  dönüm, ha, büyük harf, takımda olmayan birim, uzunluk takımında m2), masaüstü 1 (km ile yazılanın metre gitmesi,
  toplama, takımda olmayan cm, alan birimleri, ipucu, kural değeri, sınırın ve okunamayanın sözü, metre adımı,
  komutun aldığı değerin metreyle yazılması, birimsiz alanın değişmemesi); gerçek pencere 49-birim.

## §65. Bulut denetim kutusunun Landlock tabanı ABI 1: eski çekirdeğin bıraktığını süzgeç kapatır (`ADIM 20`)

**16 Kasım.** Kod: iş çalıştırıcısı `apps/jobrun/src/kutu/linux.rs` (`ruleset(cap)`: ABI 1 zorunlu, gerisi çekirdekte
varsa; `landlock`: çekirdeğin ABI'si özellik özellik; `restrict(allowed, cap)`), `kutu/seccomp.rs` (yol adıyla kısaltma,
dosyanın üst verisi, öteki süreçler, ABI 6'nın altında sinyaller; x32'nin tümü: `refuse_x32`), yeni `kutu/plan.rs`
(`kutu.rs`'ten ayrıldı: `Landlock { abi }`, `Plan::Whole { words, abi }`, ABI'ye göre sözler), `kutu.rs` (`ABI_WORD`:
yalnız sınamaların `--abi`'si; kutu kipi ABI 6'nın altında sinyal kurallarını kurar), `audit.rs`; sınamalar
`kutu/tests.rs`, `kutu/seccomp.rs`'in kendi sınamaları, yeni `audit/kutu_tests/probe.rs`.

- **Bugünkü uygulama (§61, §63; ölçüldü):** Landlock'tan ABI 4 isteniyordu (dosyalar ve TCP); süzgeçten beri TCP'yi
  `socket(2)`'nin reddi de kapatıyor. 6.7'den eski çekirdekte (5.15, 6.1 ve 6.6 LTS) kutu bütün kapanmıyor, `require`
  kuralı hiç çalıştırmıyordu. Ayrıca bu makinede (ABI 8 + süzgeç) kutudaki kural, kutunun dışındaki bir dosyanın
  iznini, sahibini, zamanını ve xattr'ını değiştirebiliyor; sunucunun kullanıcısının bir sürecinin kaynak sınırını
  okuyabiliyor (okuyabilen değiştirebilir: çekirdek ikisine aynı izni sorar), önceliğini ve işlemcilerini
  okuyabiliyordu. Landlock bunların hiçbirini hiçbir ABI'de denetlemiyor: §61'den beri söylenen "dosyalar kapalı", üst
  veri için doğru değildi.
- **Karar: Landlock'tan yalnız ABI 1 (Linux 5.13) istenir.** ABI 1'in dosya hakları zorunlu; ABI 2–9'un getirdikleri
  çekirdekte varsa kullanılır, yoksa süzgeç kapatır. Kutu, Landlock'un açık olduğu (önyüklemede LSM listesinde) her
  5.13 ve sonrası çekirdekte, x86_64 ve aarch64'te bütün kapanır. ABI'lerin her biri için (Landlock'un belgesi ve
  ölçüm):
  - ABI 2 (`REFER`) yoksa klasörler arası yeniden adlandırma ve bağ hiç yapılamaz (`EXDEV`): daha sıkı. Python'un bayt
    kodu dosyayı aynı klasörde yerine koyar, etkilenmez. Açık bir şey yok.
  - ABI 3 (`TRUNCATE`) yoksa yol adıyla `truncate(2)` ve okumak için `O_TRUNC` ile açılış dosyayı keser: ikincisinde
    Landlock yalnız okuma hakkını sorar, yani kuralın okuyabildiği her dosya (ortamı, paketi). Süzgeç ikisini de
    reddeder; bayraklarını göremediği `openat2(2)`'yi tümüyle. `ftruncate(2)` yazmak için açılmış bir tanıtıcı ister,
    onu ABI 1 denetler.
  - ABI 4 (TCP) yoksa: süzgeç `socket(2)`'yi her aile için reddediyor (§63), açık bir şey yok.
  - ABI 5 (`IOCTL_DEV`) yoksa: kutuda açılabilen aygıtlar yalnız `/dev/null` ve `/dev/urandom`, miras alınan
    tanıtıcılar boru; açık bir şey yok.
  - ABI 6 (kapsamlar) yoksa soyut UNIX soketleri süzgeçle kapalı (soket açılamaz); sinyaller de süzgeçle (aşağıda).
  - ABI 9 (`RESOLVE_UNIX`) yoksa yol adlı UNIX soketleri süzgeçle kapalı (§63).
- **Sinyaller ABI 6'nın altında süzgeçle: temiz, kuruldu.** Kutu kipi kendi süreç numarasını (exec'ten geçer)
  süzgece yazar: `kill` yalnız kendine ya da işin süreç grubuna (0); `tkill`, `tgkill`, `rt_sigqueueinfo`,
  `rt_tgsigqueueinfo` yalnız kendi içinde (iş parçacıkları dahil); `pidfd_open` ve `pidfd_send_signal` yok. SDK'nın
  konağı yalnız iş parçacığı kullanır, sinyal göndermez; asyncio'nun döngüsü sinyal göndermez, alt süreç izleyicisi
  `pidfd_open` reddedilince iş parçacığına düşer. Sınama ABI 1'de bunu gösterir: asyncio, kuralın kendine, grubuna ve
  iş parçacığına sinyali açık; başka sürece kapalı. Bedeli: ABI 6'nın altında kuralın çatalladığı bir süreç kendini
  sinyalleyemez (`raise`), kural çocuğunu tek tek sinyalleyemez. ABI 6 ve üstünde bu kurallar kurulmaz: orada Landlock
  sinyali kutunun içinde tutar ve içindekilere izin verir.
- **Landlock'un hiç denetlemedikleri de süzgeçte (her ABI'de):**
  - Dosyanın üst verisi, yol ve tanıtıcıyla: `chmod`, `fchmod`, `fchmodat`, `fchmodat2`, `chown`, `fchown`, `lchown`,
    `fchownat`, `utime`, `utimes`, `futimesat`, `utimensat`, `setxattr`, `lsetxattr`, `fsetxattr`, `setxattrat`,
    `removexattr`, `lremovexattr`, `fremovexattr`, `removexattrat`. Kural sunucunun kullanıcısının herhangi bir
    dosyasını okunmaz ya da herkese okunur yapabiliyordu.
  - Öteki süreçlerin kaynak sınırı, önceliği, zamanlaması ve G/Ç önceliği yalnız kendine (0 ya da kendi numarası):
    `prlimit64`, `setpriority` (yalnız tek süreç), `sched_setaffinity`, `sched_setscheduler`, `sched_setparam`,
    `sched_setattr`, `ioprio_set` (yalnız tek süreç). Kural sunucunun dosya sınırını sıfıra indirip onu
    durdurabilirdi.
  - Bedeli: kural karalamasındaki dosyanın da iznini ya da zamanını değiştiremez (`shutil.copy`/`copy2` karalamaya
    kopyalayamaz; okuyup yazarak kopyalanır), kendi iş parçacıklarına ayrı işlemci ya da zamanlama veremez (yerel
    kütüphaneler bu ret karşısında devam eder). Python'un açılışı ve SDK'nın konağı bunların hiçbirini kullanmaz.
- **x32:** x86_64'te x32 numaralı (`0x40000000`'lı) bütün çağrılar reddedilir: mimari sınamasının hemen ardından üç BPF
  komutu, seccompiler'in kendi kurallarından önce (göreli atlamaları kaymaz; yerini bir sınama denetler). §63'teki tek
  tek x32 numaralarının yerine: x32'nin kendine özgü numaralarını (ör. `rt_sigqueueinfo` 524) da kapatır.
- **Söz (her ABI'de doğru):** bütün kutu "dosyalar kapalı: içerik, yol adıyla kısaltma, izin, sahip, zaman, xattr; ağ
  kapalı (soket açılamaz): TCP, UDP, yol adlı ve soyut UNIX soketleri, öteki türler, io_uring; yalnız kutunun içinde
  kalan UNIX akış çifti açılır; öteki süreçler kapalı: sinyaller Landlock'la, kaynak sınırı, öncelik ve zamanlama
  süzgeçle" der; ABI 6'nın altında "sinyaller süzgeçle (Landlock ABI 6 yok)". Süzgeçsiz kutu (`report`) ABI'sine göre
  söyler: "kapalı: dosyaların içeriği, …; açık: yol adıyla kısaltma (ABI 3 yok), TCP (ABI 4 yok), …, dosyaların izni,
  sahibi, zamanı ve xattr'ı, öteki süreçlerin kaynak sınırı, önceliği ve zamanlaması". Çekirdeğin ABI'si kısıtlamadan,
  özellik özellik sorulur (9, 6, 5, 4, 3, 2, 1): ABI 7 ve 8 kutunun kullandığı bir şey eklemez, bu makine (ABI 8) "6"
  sayılır.
- **Sınamada düşük ABI:** kutu kipinin gizli sözü `--abi <n>` kural kümesini n'den yeni hakları istemeden kurar;
  çekirdek kural kümesine o ABI'nin anlamını uygular (`REFER` istenmeyince klasörler arası taşıma reddi, `TRUNCATE`
  istenmeyince kısaltma serbest, kapsam istenmeyince sinyaller serbest), yani yalnız ABI 1'i bilen bir çekirdeğin
  kutusu. Sözü iş çalıştırıcısı kendi kurduğu komut satırında yalnız `Plan::Whole`'un `abi`'si doluysa yazar; işte
  `None`'dır, kural komut satırına erişemez. Geçersiz değer (0, 10, sayı olmayan) kutu kipini hiçbir şey kısıtlamadan
  `KENTOS-KUTU` ile durdurur.
- **Kapanmayanlar:** (1) Yolların varlığı ve üst verisinin okunması (`stat`), öteki süreçlerin varlığı, önceliği ve
  işlemcilerinin okunması (`getpriority`, `sched_getaffinity`; değiştirmek kapalı). (2) Kuralın açabildiği dosyalarda
  `ioctl` ile düğüm bayrakları (`FS_IOC_SETFLAGS`; değiştirilemez ve eklenir bayrakları yetki ister) ve dosya sistemi
  destekliyorsa fs-verity: yalnız kuralın okuyabildiği ve sunucunun kullanıcısına ait dosyalarda (ortamı, paketi). (3)
  Windows ve macOS'ta kutu yok.
- **Ölçüm (hata ayıklama derlemesi, 16 Kasım):** kutu kipi `/usr/bin/true`'nun çevresinde (30 koşunun ortancası): iş
  çalıştırıcısının kendi açılışı 7.29 ms, yalnız Landlock'la 7.29 ms, bütün kutuyla 7.98 ms, bütün kutuyla ABI 1'de
  7.94 ms; kutunun kendi maliyeti 7.41 ms, süzgecin payı 0.69 ms (büyüyen süzgecin derlenmesi ve kurulması; §63: 0.38
  ms), Landlock'un özellik özellik sorulması gürültünün içinde. Ortamın Python'unun SDK'nın konağını yüklemesi (`import
  kentos.ext_host`, sırayla 10'ar koşu, ortanca, duvar / işlemci): kutusuz 983 / 982 ms, Landlock'ta 987 / 986 ms, bütün
  kutuda 990 / 989 ms, bütün kutuda ABI 1'de 991 / 990 ms: süzgecin her çağrıdaki payı ölçülemiyor (+3 ms). Olağan
  kuralın işi baştan sona (sınama tek başına, iki koşu; §58, §61 ve §63'ün ölçümü): 1.93 sn işlemci (§63: 1.90-1.93),
  en çok 505.3-506.2 MB, 2.01-2.11 sn.
- **Sınamalar:** iş çalıştırıcısının sınamalarının tümü 25/25.
  - Gerçek Python'la `the_whole_box_closes_the_same_at_the_kernel_s_landlock_abi_and_at_abi_1` aynı kuralı çekirdeğin
    ABI'siyle (bu makinede "6") ve `--abi 1` ile koşturur; 25 denemenin sonucu hata numarasıyla karşılaştırılır. İki
    ABI'de de TCP, UDP, yol adlı UNIX soketi, UNIX datagram çifti, io_uring halkası, kutunun dışındaki ve işin kendi
    dosyaları, yol adıyla kısaltma, kuralın kendi modülünü `O_RDONLY | O_TRUNC` ile açmak, kutunun dışındaki bir
    dosyanın izni, sahibi, zamanı ve xattr'ı, sınama sürecinin dosya sınırı, önceliği ve işlemcileri `PermissionError`
    (13); sınama sürecine sinyal çekirdeğin ABI'sinde Landlock'la (`EPERM`, 1), ABI 1'de süzgeçle (`EACCES`, 13)
    kapalı. asyncio, kuralın kendine, grubuna ve iş parçacığına sinyali, karalaması açık. Karalamada klasörler arası
    taşıma çekirdeğin ABI'sinde açık, ABI 1'de `EXDEV` (18): kural kümesinin gerçekten ABI 1'de kurulduğunu bu gösterir.
    Dinleyicilere hiçbir şey ulaşmadı, kesilmeye çalışılan dosya bütün kaldı. Kutusuz aynı kural 25 denemenin hepsini
    açıyor (bu makinede io_uring ve xattr da), üç dinleyici de alıyor.
  - Süzgecin kendi sınamaları: x32 bekçisi mimari sınamasının hemen ardında; sinyal kuralları yalnız istenince.
  - Planın tablosu her ABI'de (1, 2, 3, 4, 5, 6, 9) ve iki ilkede bütün kutuyu verir, sözü yalnız sinyallerin kimde
    olduğuyla değişir; süzgeçsiz Landlock'un sözü ABI 6'da ve ABI 1'de neyin açık kaldığını sayar. Kutu kipi geçersiz
    `--abi`'yi (0, 10, `bir`) hiçbir şey kısıtlamadan reddeder; komut satırında `--abi` yalnız istenince var.
  - `kentosd`'nin işlemci sınamaları 26/26 (iki koşu). Ondan önceki bir koşuda
    `a_memory_budget_is_kept_by_killing_the_child` `tool_unavailable` ile düştü (çocuk başlatılamadı). Kutuyla ilgisiz:
    o sınamanın çocuğu bir kabuk betiği, kutu kipi değil. Olası neden: sınamaların ortak yardımcısı
    `support::shell_program` iki sınamaya ("sleep 30" ile) aynı dosyayı (`program-8.sh`) yazdırıyor; biri yazarken öteki
    çalıştırırsa çekirdek başlatmayı reddeder (`ETXTBSY`; hata sözü kaydedilmedi). Düzeltildi: yardımcı her çağrıya
    ayrı ad verir (`program-<n>.sh`); ardından iki koşu 26/26.
  - Clippy (sınamalı ve sınamasız derleme, `--no-deps`), rustfmt ve `scripts/arch/deps.mjs` temiz.

## §66. Sunucu denetim sürümünün Python ortamını hazırlar: kilidin tekerlekleri, özetleriyle (`ADIM 20`)

**16 Kasım.** Kod: `kentos_python_env` `crates/native/python-env/src/extension.rs` (`Key::digest`, `make`'in
`start`/`sync`/`finish`'e ayrılması, `remove_version`) ve yeni `extension/wheels.rs` (`Wheels`, `prepare_wheels`,
`wheel_words`); `kentos-application` `audit_catalog.rs` (`AuditPythons::overlaid`, `insert`, `remove`, `entries`,
`to_json`); `kentosd` yeni `apps/api/src/audit_env.rs` (`EnvSettings`, `prepare_release`, `remove_release`,
`listing`, `release_of`), `audit_env/map.rs` (`AuditMap`), `audit_env/python.rs` (KentOS'un Python'u, `in_range`,
hata sözleri), `config.rs` (`audit_envs`), `cli.rs` (`audit-env prepare`,
`remove`, `list`), `extension_catalog/audits.rs` (`PythonSource`); bağımlılık yönü `scripts/arch/deps.mjs` (`api` →
`python-env`); sınamalar `extension/wheels/tests.rs`, `audit_env/tests.rs`, `config.rs`'in, `audit_catalog.rs`'in ve
yeni `apps/api/tests/job_run/audit_env.rs`.

- **Bugünkü uygulama (okundu):** §50'de yorumlayıcı eşlemesi elle yazılacak bir ayardı ve hiçbir yerde okunmuyordu
  (`AuditPythons::parse` yalnız sınamalarda); ortamı kuran bir adım yoktu.
- **Karar: masaüstünün makinesi, sunucunun yeri.** Ortamın anahtarı, yeri, kurulumun kendi tekerleğinden KentOS
  kitaplığı, `uv pip check`, bayt kodu derlemesi ve kitaplığın açılması, paket listesi ve en son yazılan kayıt
  masaüstünün kodudur (`extension::finish`). Masaüstünün yolu değişmedi: `make` yalnız üç parçaya ayrıldı (`start`,
  `sync`, `finish`), argümanlar, ortam, sıra ve hata sözleri aynı; crate'in sahte uv'li sınaması
  (`an_extensions_environment_is_its_own_previewed_made_and_used_offline`, uv'nin aldığı her söz) ve masaüstünün ortam
  sınamaları geçti. Anahtara sunucuya özgü bir alan eklendi (`digest`, paketin içerik özeti): masaüstünde boş, JSON'a
  yazılmaz ve klasör adının özetine girmez; masaüstünün anahtarı ve klasörü baytı baytına aynı (sınama eski biçimin
  özetini hesaplayıp karşılaştırır).
- **Neden `uv sync` değil (ölçüldü, uv 0.11.26):** kilit, yazarın dizinine bağlı. Bir dizinden kilitlenmiş projeye
  `uv sync --locked` ile `--find-links` ya da `--no-index` verilince uv "The lockfile at `uv.lock` needs to be updated,
  but `--locked` was provided" der; `--frozen --offline --find-links` kilitteki adresi önbellekte arar, klasöre hiç
  bakmaz. Sunucunun yolu (`prepare_wheels`): (1) `uv export --locked --offline` kilidin listesini, her dosyanın
  SHA-256'sı ve ortam işaretleriyle verir (kilidin projeyle uyuşması denetlenir, ağ istenmez; bayraksız ve çevrimdışı
  ölçüldü); (2) `uv venv` KentOS'un Python'undan, indirmesiz; (3) `uv pip install --require-hashes --no-build --no-deps
  -r liste`.
- **Yalnız tekerlek, özet zorunlu:** `--no-build` hiçbir kaynak paketinin derleme arka ucunu sunucuda çalıştırmaz;
  `--require-hashes` kilitteki özeti olmayan isteği ve özeti tutmayan dosyayı reddeder; `--no-deps` listenin dışında
  hiçbir şey kurmaz (liste zaten kilidin tamamı). Bir dizinden değil yerel klasörden kilitlenmiş paketin kilidinde özet
  yoktur (uv yerel tekerleğe özet yazmıyor, ölçüldü): sunucu onu da reddeder. uv'nin sözleri Türkçeye, paketin adıyla
  çevrilir (`wheel_words`): tekerleği olmayan (yalnız kaynak paketi ya da başka bir sistemin tekerleği), klasörde
  olmayan, dosyası kilittekinin olmayan, kilidi özetsiz bağımlılık; evrensel kilidin isteğindeki ortam işaretleri
  (`tasli{…}==1.0`) addan ayıklanır. Ör.: "“tasli” 1.0: bu sunucunun sistemi (linux-x86_64) için tekerleği yok ve sunucu
  kaynak paketinden derlemez. …". KentOS'un kitaplığı kurulumun kendi tekerleğinden, masaüstündeki gibi.
- **Ağ yalnız bu adımda ve yöneticinin izniyle:** varsayılan çevrimdışı (`--offline --no-index --find-links`):
  yöneticinin doldurduğu tekerlek klasöründen (`KENTOS_AUDIT_WHEELS`, yoksa ortamların klasöründe `tekerlekler`). İzin
  verilen sunucuda `KENTOS_AUDIT_INDEX_URL` bir paket dizinidir (http/https): yalnız `pip install` adımı ona çıkar,
  klasör de bakılır. Dışa aktarma, venv, kitaplık, denetim ve derleme her zaman çevrimdışı. İşler eskisi gibi ağsız
  (kutu).
- **Yer ve anahtar:** `KENTOS_AUDIT_ENV_DIR/ortamlar/<16 hane>`; anahtar eklenti, sürüm, içerik özeti, kilidin
  SHA-256'sı, KentOS'un Python sürümü ve sistem. Kayıt en son yazılır: yarım ortam eşlenmez, hata yarım klasörü siler.
  Hazır ortam için yeniden hazırlama uv'yi hiç çalıştırmaz (anahtar Python'un sürümüyle kilidin özetinden kurulur,
  kitaplığın tekerleği ancak gerekince özetlenir), eşlemeyi yeniden yazar. Aynı sunucuda hazırlama ve silme tek tek
  yürür (klasörün kilidi, `hazirlama.kilit`): yönetici komutu ve yayımlama aynı anda aynı ortamı ya da eşlemeyi yazmaz.
- **KentOS'un Python'u:** `KENTOS_AUDIT_BASE_PYTHON` (mutlak yol); sürümü sorulur ve KentOS kitaplığının aralığında
  (`>=3.10,<3.15`, compat.json) olmalı. Ayar yoksa uv'nin ortamların klasörüne kurduğu KentOS Python'u aranır; o da
  yoksa ret ve ne yapılacağı.
- **Eşleme dosyası (`AuditMap`):** ortamların klasöründe `yorumlayicilar.json`, `{"eklenti@sürüm": "/…/bin/python"}`;
  yanına geçici bir dosya yazılıp üstüne taşınır (yarım dosya okunmaz). `KENTOS_AUDIT_PYTHONS` ayarı (§50'nin JSON'u,
  artık okunuyor) altındadır, aynı sürümde dosya üstün; dosya okunamazsa yalnız ayar geçerli. Kaynak (`CatalogAudits`,
  `PythonSource`) eşlemeyi her işte yeniden okur: sunucu çalışırken hazırlanan sürüm yeniden başlatmadan bulunur. Bir
  sürümün ortamı silinince (`remove_release`) eşlemedeki yeri de gider.
- **Yönetici komutu:** `kentosd audit-env prepare --release KİMLİK@SÜRÜM --package PAKET.kek [--digest sha256:…]`
  paketi işin denetlediği gibi denetler (açılış, verilirse içerik özeti, kimlik ve sürüm), `network` isteyeni, denetimi
  ya da Python projesi olmayanı reddeder; `audit-env remove --release …` ve `audit-env list`. Komut satırı ve
  ayarlar `kentosd`'nin öteki komutları ve ayarları gibi `cli.rs`'in `USAGE`'ında ve `config.rs`'te belgeli.
- **Yayımlama kancası:** `prepare_release(ayarlar, eklenti, sürüm, özet, baytlar)` katalogun `AuditPackage`'ıyla
  çağrılacak biçimde; kalıcı katalog deposu (ADIM 22A) gelince yayımlama onu çağırır ve `main.rs` işçiye
  `CatalogAudits::new(katalog, config.audit_envs.map())` verir. Bugün üretimde katalog kurulmadığı için kanca bağlı
  değil (§50).
- **Bağımlılık yönü:** `kentosd` (`apps/api`) artık `python-env`'i kullanır (`scripts/arch/deps.mjs`'in `api` öbeği);
  `python-env`'in yeni bağımlılığı yok, uv bir program olarak çalışır.
- **Ölçüm (hata ayıklama derlemesi, 16 Kasım; uv 0.11.26, Python 3.14.4):** bir tekerlekli kilitten ortamı çevrimdışı
  hazırlamak 3,73-5,77 sn (dört koşu): dışa aktarma, venv, tekerlek, KentOS kitaplığının 91 MB'lık hata ayıklama
  tekerleği, `uv pip check`, bayt kodunun derlenmesi ve kitaplığın (789 MB'lık yerel modülüyle) açılması, liste. Hazır
  ortamı yeniden hazırlamak 0,024-0,079 sn (uv hiç çalışmıyor; ilk denemede uv'yi bulmak ve kitaplığın tekerleğini
  özetlemek hazır denetiminden önceydi: 0,71 sn, sıra düzeltildi). Sınamanın klasörü (ortam, uv'nin
  önbelleği, tekerlekler) kitaplığın hata ayıklama derlemesiyle 342 MB tuttu; bu yüzden geçici bellek klasöründe değil,
  derlemenin yanında açılır.
- **Sınamalar:**
  - `kentos-python-env` 27/27: masaüstünün sahte uv'li ortam sınaması değişmeden geçiyor; yeni ikisi sunucunun yolunu
    (uv'ye giden her söz, hazırken hiçbir şey çalışmaması, dizinle çevrimiçi sözler, kaynak paketi yalnız bağımlılıkta
    ret ve yarım klasörün gitmesi, masaüstü anahtarının eski özeti) ve uv'nin 0.11 sözlerinin Türkçesini (ortam
    işaretli evrensel kilit isteği dahil) sınıyor.
  - `kentos-application` eşleme 2/2; `kentosd` birim sınamaları (`audit_env`, `config`, katalog) 24/24: dosyanın ayarın
    üstünde olması, bütün yazılması ve yanında bir şey kalmaması, okunamayan dosyada ayara dönüş; içerik özeti, kimlik,
    sürüm, `network`, denetimsiz paket ve açılmayan paketin hiçbir şey çalışmadan reddi; ayarların okunması ve
    yanlışının kendini adlandırması.
  - Gerçek uçtan uca (`job_run`, 30/30): sınama bir tekerlek yazar, uv onu klasörden kilitler, kilit dizinin yazdığı
    biçime (adres ve özet) çevrilir, paket onunla paketlenir; ortam çevrimdışı hazırlanır, eşleme yorumlayıcıyı verir,
    ikinci hazırlama hiçbir şey çalıştırmaz; `kentosd`'nin gerçek işlemcisi ve `kentos-jobrun` ile bütün kutuda
    (Landlock ve süzgeç) gerçek bir `project.audit` işi o Python'da koşar ve kural tekerleğin modülünü içe aktarır
    (bulguların hepsi "kucuk 1.0: …"); başka içerikte paket reddedilir; yalnız kaynak paketi olan bağımlılık “tasli” 1.0
    adıyla reddedilir, eşlenmez, yarım ortam kalmaz; sürüm silinince ortamı ve eşlemedeki yeri gider. Ağ hiç açılmaz:
    kilitteki dizin adresi hiç sorulmaz. uv, aralıkta bir Python ya da kitaplığın tekerleği yoksa sınama atlandığını
    söyler.
  - Masaüstünün ortam ve süreç sınamaları (`extensions::tests::python`, `extensions::tools::tests`) 12/12.
  - Clippy (sınamalı ve sınamasız, `--no-deps`; kalan iki not `kentos-application`'ın bu işin dokunmadığı
    `jobs.rs`'inde), rustfmt ve `scripts/arch/deps.mjs` temiz.
- **Kalan:** (1) Yayımlamaya bağlanması (ADIM 22A). (2) İş, ortamın derlenmiş bayt kodunu okumuyor: kutu yalnız
  `.venv`'i açıyor, işin bayt kodu her işte yeniden derleniyor (§58'deki gibi); ortamın `pyc`'si kutuya salt okunur
  açılabilir. (3) İsteğe bağlı özellikler (extras) sunucuda seçilmiyor. (4) Sunucunun KentOS Python'unu kuran bir komut
  yok: yönetici bir CPython gösterir. (5) Windows ve macOS sunucuda kutu yok (§61), ortam hazırlanır ama kural `require`
  altında çalışmaz.

## §67. İlişki seçicisi ve listesi: `relationPicker`, `relationList` (`ADIM 22`, "ilişki başvurusu ve listesi")

**16 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/ui_panel.rs` (`Component::RelationPicker`,
`Component::RelationList`), yeni `ui_panel/relations.rs` (Kapı 1), `ui_panel/rules.rs`, `ui_panel/rows.rs`, şema ve
Python kurucuları (`ui.relation_picker`, `ui.relation_list`), SDK rehberi; masaüstü yeni
`apps/desktop/src/shell/extensions/panel_relations.rs` (`relation_choices`, `relation_rows`, `parent_of`,
`relation_select`, `relation_table`), `panels.rs` (değer, zorunluluk, kural değeri, silinmiş kaydın bırakılması),
`panel_session.rs` (`ctx.panel.set`), `panel_view.rs` (`rows_table`: komutun tablosuyla ortak çizim); Vitrin 1.0.48.

- **Karar: projenin ilişkileri, formun ilişkili kayıt kutusunun okuduğu gibi.** İlişki kimliğiyle anılır (`relation`):
  eklentinin `schema.json`'unun kurduğu ya da kullanıcının projesindeki (§23; üst katman, alt katman, alt katmanın üst
  kaydın kalıcı kimliğini tutan alanı, ADR 0405). Okuma çizimin modelinden, formun ilişkili kayıt kutusununkiyle aynı
  yoldan (`gis/forms/entry/related.rs`): komut yok, Python yok, çizildiği an çizimden.
- **`relationPicker`:** ilişkinin üst katmanının kayıtları, en çok 500, katmanın sırasıyla; kaydın sözü `display` alanı,
  yoksa etiketi, yoksa ilk dolu alanı, yoksa "Kayıt <kimliğin başı>" (formun kutusunun adlandırması). Aynı sözlü iki
  kayıt kimliğinin başıyla ayrılır. Değeri seçilen kaydın kalıcı kimliği (alt alanın tuttuğu), boşsa None; zorunlu
  olabilir, `onChange` alır, satıra girer, kural dili kimliği okur. Kod `ctx.panel.set(id, value=<kimlik>)` ile seçer
  (kimlik olmayan söz reddedilir), None bırakır. Komut başlarken seçilmiş ama o arada silinmiş kayıt bırakılır
  (nesne seçicinin seçimleri gibi): kod bulamayacağı bir kimlik almaz.
- **`relationList`:** `of` bileşeninin (bir nesne seçicinin ilk nesnesi ya da bir ilişki seçicisinin kaydı) tuttuğu
  kaydın alt kayıtları: alt katmanda bağ alanı onu adlandıranlar (çok değerli alanda `;` ile ayrılmışların biri), en
  çok 500. Sütunlar `columns` (en çok 6), verilmezse alt katmanın bağ alanı dışındaki ilk dört alanı; başlıkları alanın
  adı (takma adı varsa o), değerleri katmanın tablosunun yazdığı gibi (`display_value`: tarih "16.11.2026"; ilk sürüm
  ham "2026-11-16" yazıyordu, gerçek pencere). Komutun tablosuyla aynı çizim (`rows_table`): sayfalar, satıra tıklamak
  kaydı gösterir. Kayıt seçilmeden "Önce bir kayıt seçin.", alt kaydı yoksa "Bu kayda bağlı kayıt yok.", ilişki
  projede yoksa "“…” ilişkisi bu projede yok.". Değeri yok (bir çıktı): öbür çıktılar gibi formun altında durur.
- **Ekleme, bağlama, düzenleme kodda:** formun kutusunun eylemleri (Yeni, Bağla, Bağı kopar, Düzenle; ADR 0762) panele
  taşınmadı. Kod `cad.records.create` ile geometrisiz kayıt yazar; liste Python'suz kendiliğinden yenilenir.
- **Kapı 1** (`ui_panel/relations.rs`): `relation` bir ilişki kimliği (harf, rakam, `-`, `_`, `.`; 1–64), seçicinin
  `display`'i bir alan adı, listenin `of`'u bu paneldeki bir nesne ya da ilişki seçicisi, `columns` en çok altı alan
  adı; her yanlış yerinde. İlişkinin projede olup olmadığı kurulumda bilinemez: panelde söylenir.
- **SDK paketi:** rehber iki türü anar; yer açmak için kart ve gösterge açıklamalarındaki tekrarlar ve bir örnek çıktı,
  `required` ve `onChange` alan listeleri "seçiciler" diye genelleşti (yeni seçicileri de kapsar). Paket 47 926 bayt
  (sınır 48 000; önce 47 986).
- **Vitrin 1.0.48:** "Bakım kayıtları" paneli (Vitrin › Panel): Tesis (`relationPicker`, `bakim-tesis`, `display:
  "ad"`), Yeni bakım bölümünde Tarih (`defaultExpr: bugün()`), Açıklama ve Ekle (`bakim-ekle`: `cad.records.create`),
  altta Bakım kayıtları (`relationList`, `of: "tesis"`, Tarih ve Açıklama). Gerçek pencerede (50-iliski): Katmanları
  kur, Örnek tesis'e Nokta aracının Ad'ıyla iki nokta (Merkez Okulu, Park Kreşi); Tesis iki tesisi adlarıyla sundu
  (`ad` alanı boş, etiketleri okundu); Merkez Okulu'na iki kayıt eklendi, liste her Ekle'de kendiliğinden yenilendi
  ("16.11.2026 · Yangın tüpleri yenilendi", "16.11.2026 · Çatı oluğu temizlendi"); Park Kreşi seçilince "Bu kayda bağlı
  kayıt yok.".
- **Sınamalar:** paylaşılan 1 (Kapı 1'in sekiz durumu); masaüstü 1 (kayıtların fikstürüne ilişki: seçicinin sözü,
  listenin satırları ve kimlikleri, varsayılan sütunlar, tarihin günün sözüyle yazılması, başka kaydın boş listesi,
  projede olmayan ilişki, zorunluluk, değer ve kural değeri, kodun `set`'i ve reddi); rehberin ve §25'in türleri anması;
  Vitrin'in sayıları (16 komut, 6 panel) ve şerit etiketleri; gerçek pencere 50-iliski.

## §68. Panelde ek listesi: `attachments` (`ADIM 22`, envanterin "ek"i)

**16 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/ui_panel.rs` (`Component::Attachments`, Kapı 1), şema ve
Python kurucusu (`ui.attachments`), SDK rehberi; masaüstü yeni `apps/desktop/src/shell/extensions/panel_attachments.rs`
(`attachments_view`, `panel_attachment_slots`), `shell/attachments.rs` (`wanted_photos`: gösterilen panelin ekleri),
`panel_view.rs`, `panel_session.rs`; Vitrin 1.0.49.

- **Karar: KentOS'un ekleri, Öznitelikler'in gösterdiği gibi.** `of` bileşeninin (nesne seçicinin ilk nesnesi ya da
  ilişki seçicisinin kaydı) tuttuğu kaydın fotoğrafları, belgeleri ve adresleri (ADR 0225), her biri bir satır:
  fotoğrafın küçük resmi çözülünce, yoksa türünün simgesi; adı (boşsa türü), nerede olduğu ("Bağlı dosya: …", adres);
  Aç (fotoğraf görüntüleyicisi, belge ve adres için sistemin açıcısı). Fotoğraflar kabuğun öbür fotoğraflarıyla aynı
  kuyrukta, iş parçacığı dışında çözülür (`wanted_photos`'a gösterilen panelin kayıtları eklendi). Kayıt seçilmeden
  "Önce bir kayıt seçin.", eki yoksa "Bu kaydın eki yok.".
- **Karar: eklemek KentOS'un, kullanıcının düzenlemesi.** Katmanı kilitli değilse altta KentOS'un kendi Dosya ekle… ve
  Adres ekle…'si (Öznitelikler'inkiyle aynı pencere ve yazım, tek geri alma adımı "Ekler"). Eklentinin kodu bir şey
  yazmaz, yeni bir yetenek istenmez: ekleyen kullanıcıdır. Kodun eki okuması ya da yazması `ctx.doc` ve
  `cad.entities.set` ile, eskisi gibi.
- **Değeri yok** (bir çıktı): öbür çıktılar gibi formun altında durur. Kapı 1: `of` bu paneldeki bir nesne ya da ilişki
  seçicisi.
- **SDK paketi:** rehber türü anar; yer için görselin örnek cümlesi çıktı. Paket 47 981 bayt.
- **Vitrin 1.0.49:** Bakım kayıtları'nın altında "Tesisin ekleri" (`of: "tesis"`). Gerçek pencerede (51-ek): Merkez
  Okulu seçildi, "Bu kaydın eki yok."; Adres ekle… KentOS'un penceresini açtı, `https://okul.ornek.bel.tr` ve
  "Okulun sayfası" yazılıp Kaydet: liste eki bağlantı simgesi, adı ve adresiyle, Aç'la gösterdi.
- **Sınamalar:** masaüstü 1 (seçilmeden kayıt yok; nesne seçicinin binası seçilince eki listede, katmanı eklemeye açık,
  görünüm kuruluyor); rehberin ve §25'in türü anması; gerçek pencere 51-ek.

## §69. Bölünmüş panel: `split`, form ile çıktılar arasında sürüklenen çizgi (`ADIM 22`, envanterin "bölücü"sü)

**16 Kasım.** Kod: paylaşılan `crates/shared/extensions/src/ui_panel.rs` (`PanelDefinition::split`, Kapı 1), şema ve
Python kurucusu (`ui.panel(…, split=True)`), SDK rehberi; masaüstü `apps/desktop/src/shell/extensions/panel_view.rs`
(`panel_content`: bölünmüş panel `responsive` içinde eldeki yüksekliği bilerek kurulur), `panels.rs`
(`OpenPanel::split`, `out_scroll`, `PanelEvent::{Split, SplitReset, ScrolledOutputs}`, iki kaydırıcının geri verilmesi);
Vitrin 1.0.50.

- **Karar: panelin anahtarı, bileşen değil.** Çıktılar (tablo, ağaç, kart, gösterge, grafik, görsel, ilişki ve ek
  listesi) zaten formun altında çiziliyor; gövdede bir bölücü bileşeninin yeri görünür sırayla örtüşmezdi. `"split":
  true` ile başlık, form, çalışmanın çubuğu üstte bir kaydırıcıda; çalışmanın sözü ve çıktılar altta kendi
  kaydırıcısında; arada KentOS'un tutamağı (`Sash::horizontal`, dock'larınki). Uzun sonuçlar kayarken form görünür
  kalır.
- **Yükseklik:** form ilk açılışta eldeki yüksekliğin beşte üçü (önce formdan yana: dar dock'ta alanlar kesilmesin),
  kullanıcı sürükleyince o yükseklik paneliyle kalır (96–1600 piksel), çift tık ilk hâline döndürür. Çıktıların kaydırma
  yeri de panel başına tutulur, panel yeniden gösterilince ikisi de geri verilir. Çıktısı olmayan panel bölünmez.
- **Kapı 1:** `split` true ya da false; bilinmeyen üst alanların listesi onu anar. Pencerede (`ctx.dialog`) yok.
- **SDK paketi:** rehber `split`'i anar; yer için tarih-saat alanının yazım cümlesi kısaldı. Paket 47 980 bayt.
- **Vitrin 1.0.50:** Bakım kayıtları bölünmüş. Gerçek pencerede (52-bolucu): Öznitelikler kapatılınca form üstte
  eksiksiz, iki kayıtlı liste ve ekler altta; çizgi y=595'ten 470'e sürüklendi, çıktılar büyüdü, form kendi
  kaydırmasına geçti; çift tık çizgiyi geri getirdi. 50-iliski ve 51-ek bölünmüş panelle yeniden koşuldu. **Bulgu:**
  çizginin tutma alanı birkaç piksel; ilk sürükleme 2 piksel ıskalayınca hiçbir şey olmadı (üzerine gelince imleç
  boyutlandırma okuna döner).
- **Sınamalar:** paylaşılan 1 (`split` okunur, yanlış türü yerinde söylenir, verilmezse kapalı); masaüstü 1 (bölünmüş
  görünüm kurulur, sürükleme yüksekliği ve çıktıların kaydırması tutulur, çift tık bırakır); Python kurucusunun çıktısı;
  gerçek pencere 52-bolucu.

## §70. Eklenti araçları başsız konaklarda: MCP ve Python, aynı araç aynı süreçte (`ADIM 19`, "Derslerden")

**16 Kasım.** Kod: yeni `crates/native/extension-tools` (`kentos-extension-tools`: `plan.rs`, `run.rs`, `calls.rs`,
`synthetic.rs`, `headless.rs`); İşlemler `external.rs` (`DESKTOP_RUNTIME`, `HEADLESS_RUNTIME`,
`ExternalTool::headless`, `register_provider_if_none`, `is_provider`, `set_unavailable`/`unavailable`),
`command.rs`, `model.rs`, `external/descriptor_schema.rs`; `kentos-extension-host` (`verify_installed`);
`kentos-python-env` (`process::interpreter_args`); masaüstü `tools.rs` (ince köprü), `tool_calls.rs` paylaşılan
crate'e taşındı, `trial/drawing.rs`; MCP `server.rs`, `main.rs`, `extensions.rs` (`said`); ajan araçları
`agent_tools.rs` (`processing.describe` nedeni söyler); Python bağı `lib.rs` (`extension_tools`);
`python/kentos/ext_host.py`; SDK: `blueprint.rs`, `sdk_pack.rs`, araç tanımının şeması.

- **Karar: çekirdek küçük, yeni bir crate'te.** Masaüstünün köprüsündeki iş paylaşılan koda indi: kayıt defterinden
  araç listesi ve planlar (`read_tools`, `Plan::of`), ortamın bulunup sürecin başlatılması (`start_of`), koşu
  (`run_tool`: girdiler veri olarak, ilerleme, günlük, yazdırılan satırlar, Durdur, süreç belleği, nasıl bittiğinin
  İşlemler'in yanıtına ve bir `Note`'a okunması) ve kodun istediği yerleşik araç (§47: `calls::tool_call`, sentetik
  çizim `synthetic_on`). Masaüstünün `Bridge`'i kendi `Shared`'ını okur, `run_tool`'u çağırır, notlarını pencerenin
  iş parçacığına bırakır (günlük, bildirim, bekçi); iletiler ve davranış aynı. Neden `kentos-headless` değil: başsız
  konak saf kalır (süreç başlatmak bir konağın işi) ve masaüstü de onu kullanıyor; `kentos-processing` değil:
  sözleşme süreç başlatmaz (§7); `kentos-python-env` değil: sunucunun API'si onu kullanıyor, İşlemler'i taşımamalı.
  Yön tek: `extension-tools` → `headless` (`mod-gis`) → `processing`; `scripts/arch/deps.mjs`'te kendi grubu, MCP,
  Python ve masaüstü ona bağlanabilir. MCP'de `mod-gis` özelliğine bağlı (İşlemler CBS'nindir).
- **Karar: kayıt kuralı.** `headless::offer` yalnız konakların giriş noktasından çağrılır: MCP programının başı
  (`main.rs` → `Server::offer_extension_tools`; sınamaların kurduğu sunucu istemedikçe sunmaz) ve Python kitaplığının
  çizim açması (`Session.new_project/open/from_bytes`). Paylaşılan oturum kodu (`Session::of`) çağırmaz: masaüstü de
  başsız parçaları barındırıyor (konsol, deneme, §47) ve kayıt orada olsaydı masaüstünün köprüsünün yerine geçerdi.
  İkinci kat: `register_provider_if_none`, bir sağlayıcı kayıtlıysa yerine geçmez; listeler yalnız kendi sağlayıcısı
  kayıtlıyken yazılır (`is_provider`). Masaüstü eskisi gibi `register_provider` ile kaydeder ve her zaman kazanır.
  Eklentinin kendi süreci de sunmaz: `kentos.ext_host` `KENTOS_EXTENSION_PROCESS` koyar; kodun açtığı yerel bir çizim
  başka bir eklentinin aracını bağımlılığını bildirmeden, süreç zinciriyle çalıştıramaz (§47, PY-RUN-15).
- **Karar: çalışma ortamı.** `capabilities.runtimes`: `desktop-python` her aracındır (araç masaüstünde her zaman
  çalışır; liste verilirse içermeli), `headless-python` başsız konakları ekler. Bilinmeyen ad, masaüstünü içermeyen
  liste ve liste olmayan değer yeriyle reddedilir: önce bilinmeyen ad sessizce yalnız masaüstü sayılırdı
  ("headles-python" aracı başsızdan habersizce çıkarırdı). Şema değerleri sayar (`enum`, `contains`, `uniqueItems`);
  şablon (`extension.blueprint`) yeni araca ikisini yazar; SDK paketi alanın yanında söyler (yer için iki cümle
  kısaldı; paket 47 989 bayt). Bildirim isteğe bağlı: araç kodu masaüstündekiyle aynı yoldan çalışsa da başsız konağa
  yazarı açar.
- **Dürüst liste:** başsız konak yalnız `headless-python` bildirenleri listeler (`cad.processing.tools`); kurulu ve açık
  olup bildirmeyeni çalıştırmak hiçbir süreç başlamadan reddedilir, kod `unknown_tool` (listede yok), ileti nedeni ve
  çareyi söyler: "“Parsel TAKS hesabı” (ext.tr.ornek.vitrin.taks) yalnız KentOS masaüstünde çalışır: araç tanımı
  başsız çalışmayı bildirmiyor. MCP'de ve Python'da çalışması için eklentinin tools/taks.tool.json dosyasında
  capabilities.runtimes “desktop-python” ile birlikte “headless-python” içermeli." Toplu çalıştırma, modelin
  denetimi ve ajanların `processing.describe`'ı aynı sözü söyler. Masaüstü bu listeyi hiç yazmaz.
- **Salt okunur:** kayıt defteri `kentos kek dump` gibi okunur (hazır eklentilerin katkıları, `planned_contributions`;
  hiçbir şey etkinleştirilmez, yazılmaz) ve dosyaları değişince yeniden okunur: kayıt, geri çekilenler, sorunlu eklenti
  turu, kurum ilkesi ve Python ayarları özetlenir, MCP her araç çağrısından, Python her işlem komutundan önce sorar
  (birkaç küçük dosya). Listeden çıkan sürümün süreci kapanır. Ortam yalnız masaüstünde hazırlanır. Süreç başlamadan
  paketin dosyaları içerik özetine karşı denetlenir (`verify_installed`: etkinleştirmenin denetimi; başsız konak
  etkinleştirmez): kurulduktan sonra değişen dosya "Kurulu dosyalar kurulduğu hâlinden farklı: src/… değişmiş.
  Eklentiyi yeniden kurun ya da önceki sürüme dönün." ile durdurur.
- **Python ilk koşuda bulunur:** kullanıcının seçimi (`ayarlar.json`), uv ve KentOS'un yorumlayıcısı masaüstünün
  bulduğu gibi (`find_python`); liste hiçbir şey başlatmaz. Bulununca araçlar kodun nerede çalıştığını söyler
  (yazdıkları katmanın türetim kaydı, §15). Kitaplık: kurulumun tekerleği programın yanında bulunursa ortama konur
  (masaüstünün yolu, §6), bulunmazsa ortam hazırlandığı kitaplıkla çalışır (`Library::IfFound`: Python kitaplığının
  yorumlayıcısının yanında tekerlek yok); masaüstü tekerleği şart koşar (`Required`), değişmedi. uv de masaüstü gibi
  aranır (seçilen yol, programın yanındaki kurulum, PATH, kullanıcı klasörleri): Python kitaplığında yorumlayıcının
  yanında kurulum olmadığından uv PATH'te ya da ayarda seçili olmalı, yoksa koşu uv'nin sözüyle durur (gerçek koşuda
  `~/.local/bin`'deydi).
- **Söylenen:** MCP koşunun notlarını standart hataya yazar (protokol standart çıktıda): "eklenti tr.ornek.vitrin
  processing:cekme: Bitti (1,5 sn; süreç belleği 126 MB)."; Python kitaplığı sessizdir, yanıt nasıl bittiğini söyler.
  Başsız konak eklentinin günlüğüne yazmaz ve çökmeyi bekçiye saymaz (kayıt defterine yazmaz). *(§73'te değişti:
  başsız koşu günlüğe yazar, Python kitaplığı Python'un logging'iyle söyler; bekçi masaüstünün kaldı.)*
- **Gerçek koşu (16 Kasım, MCP stdio'dan elle JSON-RPC, yalıtılmış klasörler):** Vitrin 1.0.50'nin kopyası, "Çekme
  mesafesi denetimi" tanımı iki ortamı bildirir, TAKS yalnız masaüstü; Python ortamı masaüstünün hazırladığının kopyası
  (anahtar aynı: sürüm, kilit, yorumlayıcı). `cad.processing.tools` (`category: extensions`) yalnız
  `ext.tr.ornek.vitrin.cekme`'yi, sürümü ve paket özetiyle verdi. Çekme `mesafe: 3` ile: "5 yapının 3 m çekme mesafesi
  denetlendi; 3 yapı parsel sınırına daha yakın.", `tasan: 3`, yeni katman; Tampon KentOS'un aracıyla (§47),
  karşılaştırma ortamın shapely'siyle. Soğuk 1,61 sn ve süreç belleği 126 MB (bayt kodu hazır), ılık 0,03 sn. Bayt kodu
  başka yolda derlenmiş kopyada ilk koşu 2,73 sn ve 365 MB'tı (bayt kodu kaynağın tam yolunun altında tutulur, §7'nin
  ölçümü). TAKS 0,00 sn'de yukarıdaki sözle reddedildi. Python bağı (`kentos._native.Session`, aynı klasörler) aynı
  listeyi, aynı özeti ve `tasan: 3`'ü verdi (soğuk 1,07 sn, ılık 0,03 sn), TAKS'ı aynı sözle reddetti;
  `KENTOS_EXTENSION_PROCESS` konunca liste boş, araç "bir işlem aracı değil" (eklentinin kendi süreci).
- **Ajanların yolu ve bulgu:** MCP'nin ajan profili (kompakt, varsayılan) aynı aracı `processing.list`'te verdi,
  `processing.describe` TAKS için aynı nedeni söyledi; `processing.run` önce plan (eklentinin aracı iş olarak, §27),
  sonra aynı girdiyle yazma. **Bulgu:** hata ayıklama derlemesinde plan MCP'yi düşürdü: "thread 'kentos-job-is-1' has
  overflowed its stack" (işin iş parçacığı varsayılan 2 MiB; aracın içinden istenen Tampon, aracın komutunun içinde
  bütün bir komut koşusu; `RUST_MIN_STACK=64 MiB` ile düşmüyordu, yani özyineleme değil yığın boyu). Çağrı artık kendi
  64 MiB'lık iş parçacığında (`calls::STACK`; masaüstünde de). Sonra varsayılan yığınla plan 1,75 sn, yazma 0,25 sn: 5
  tampon eklendi, 5 yapının `cekme` özniteliği değişti (§49).
- **Sınamalar:** `kentos-extension-tools` 5 (SDK'nın şablonu sınamaların kayıt defterine kurulur, ortamın Python'u
  `kentos.ext_host`'un satırlarını konuşan bir betik: yalnız bildiren listelenir, ilerleme, günlük ve yazdırılan satır
  İşlemler'e ulaşır, kodun istediği Tampon kendi nesnelerinde çalışır, çıktılar tipli döner; `cad.processing.run`
  üzerinden masaüstündeki gibi; yalnız masaüstü aracı süreç başlamadan reddedilir; eklenti kapatılınca listeden çıkar,
  süreci kapanır; kurulduktan sonra değişen paket çalışmaz; Python kapalıyken söylenir; koşunun bitişinin okunması,
  masaüstünden taşındı; §47'nin çağrısı, masaüstünden taşındı), İşlemler 2 yeni (çalışma ortamının okunması; konağın
  çalıştırmadığı araç koşuda, toplu çalıştırmada ve modelde aynı sözle, sağlayıcıya sorulmadan; önce kayıtlı sağlayıcı
  yerinde kalır), masaüstü eklenti araçları, deneme ve bayt kodu sınamaları (16) geçti. MCP'nin sınamaları diski
  doldurduğu için koşulmadı (derlendi, elle sürüldü).
- **Kalan:** sunucu/bulut koşusu kentosd'nin iş çalıştırıcısının ayrı yolu (denetim kuralları §42, §50, §61–§66);
  İşlemler araçlarının sunucuda çalışması bu dilimin dışında. Başsız koşunun eklenti günlüğüne ve çökme bekçisine
  yazılması, başsız konaklarda ifade işlevleri, Vitrin'in araçlarının bildirimi (Vitrin'in kendi sürüm yolu) sonraya.
- **Uygularken verilen kararlar (planda yazılı değildi):** yeni crate; kayıt giriş noktasından ve yalnız sağlayıcı
  yokken; `desktop-python` zorunlu, bilinmeyen ad ret; reddin kodu yeni bir kod değil `unknown_tool`; başsız konakta
  paket özeti süreç başlarken denetlenir; Python ilk koşuda aranır; tekerlek başsızda isteğe bağlı; eklentinin süreci
  sunmaz; notlar MCP'de standart hataya, Python'da yok; aracın içinden istenen yerleşik araç kendi yığınında.

## §71. Bileşen davranış tablosu: değer, olay, eylem, klavye (`ADIM 22`, "özellik/olay/eylem/erişilebilirlik şeması")

**16 Kasım.** Bileşen envanterinin (§25) kalanı, bileşen başına davranış, tek tabloda:
`kentos_extensions::ui_panel::behaviour::TABLE`, Kapı 1'in 28 türünün her biri bir satır, Kapı 1'in sırasıyla (test).
Davranış önceden masaüstünün panel oturumunda tek tek `match`'lerdeydi (`panel_session.rs`); hangi bileşenin neyi
aldığı yalnız ret iletilerinden öğreniliyordu.

| Sütun | Ne söyler | Tabloda |
|---|---|---|
| `value`, `example` | komutun `ctx.params`'ta aldığı değer ve biçimi (boşsa None); `ctx.panel.set(id, value=…)` için bir örnek | 16 tür |
| `events` | eklentinin kodunu başlatan: `onChange` (kullanıcı değiştirdi), `command` (basıldı) | 14 tür `onChange`, düğme `command` |
| `sets` | `ctx.panel.set`'in değiştirdikleri (`visible` her bileşende) | `value` 16, `options` 2, `text` 1, `image` 1, `enabled` 8 |
| `rows`, `focus` | `ctx.panel.rows` doldurur; `ctx.panel.focus` imleci koyar | 5 ve 7 tür |
| `keys` | klavye: `type` (Tab ulaşır, yazılır, Enter birincil düğme), `choose` (Tab; Boşluk/Enter/F4 açar, ↑↓ adımlar), `toggle` (Tab; Boşluk/Enter çevirir), `press` (Tab düğmelerine; Boşluk/Enter basar), `rows` (Tab satırlara tek durakla; ↑↓ imleç, Enter nesneyi gösterir), `pointer` (yalnız fare), `shown` (yalnız gösterilir) | 7 · 5 · 3 · 4 · 3 · 2 · 4 |
| `namedBy`, `note` | adını veren özellik (`label`, metinde `text`); KentOS'un ayrıca yaptığı (uid'li satıra tıklamak nesneyi gösterir…) | |

- **Tek kaynak:** panel oturumu `set`, `rows` ve `focus`'u tablodan reddeder; ret iletisi alanları tablodan sayar
  ("“liste” (table) enabled almaz: yalnız textField, numberField, …, button alır."). Bir özellik ve bir değer None
  verilince (`image`, `value`) kaldırılır; öbürlerinin None'ı dokunmaz (eski davranış).
- **Klavye sütunu ölçülür, yazılmaz:** masaüstü testi her türü bağlam bileşenleriyle (katman ve nesne seçici, tablo;
  nesne seçili, ilişki ve ek dolu, satırlar dolu) KentOS'un çizdiği panelde çizer ve iced işlemiyle odaklanabilir
  öğeleri ve metin girişlerini sayar (Tab'ın saydığı gibi, görünenler): `type` +1/+1, `choose`/`toggle` +1/+0,
  `pointer`/`shown` 0 (`panel_session::behaviour_tests::the_keyboard_column_is_the_drawn_panels`). Olay sütunu
  tanımdan (`onChange` okunur ya da reddedilir), `sets`/`rows` sütunları oturumdan (her tür × her özellik, örnek
  değerle) sınanır.
- **Ölçüm ve düzeltme (16 Kasım):** ilk ölçümde Tab 13 türe ulaşıyordu (7 yazılan, 5 açılır liste, onay kutusu);
  düğme, çoklu seçimin çipleri, Seçimi al, Haritada çiz, bölüm başlığı, ek listesinin düğmeleri ve satırlar yalnız
  fareyleydi: birincil olmayan düğme klavyeyle hiç basılamıyordu. Aynı gün kapatıldı, Tab artık **19 türe** ulaşıyor:
  - kitte, onay kutusunun sınıfı olarak: çoklu seçimin her çipi (`chips::toggles`) ve katlanır bölüm başlığı
    (`Form::foldable_section`) `key_focus` ile Tab alır, Boşluk/Enter çevirir (ikisini de yalnız eklenti paneli
    kullanıyor);
  - panelde (`panel_view::keyed`, kitin `key_focus`'u): düğme, Seçimi al ve Temizle, Haritada çiz ve ✕, Haritadan seç,
    Seç…, ek listesinin Aç, Dosya ekle… ve Adres ekle…'si. KentOS'un öbür pencerelerindeki düğmeler Tab turunda değil
    (`words::primary/secondary/ghost`, 947 kullanım): uygulama geneline yaymak ayrı karar.
  - **İşi süren düğme odağı tutar** (`KeyFocus::hold`): gerçek pencerede Boşluk'la basılan Özetle, komutu sürerken
    devre dışı kalınca odağı bırakıyordu ve sonraki Tab pencerenin başından başlıyordu (bölüm başlığına ulaşılamadı).
    Kit testi: tutulan öğeden Tab sonrakine, tutmayandan pencerenin ilkine (`a_held_item_keeps_the_keyboard_while_disabled`).
  - **Satırlar** (aynı gün): `table`, `card` ve `relationList` Tab'ın turunda **tek durak** (`panel_view::keyed_rows`,
    kitin `key_focus`'u); ↑ ve ↓ satır imlecini taşır (ilk ↓ ilk satırı, ilk ↑ sonuncuyu alır, uçlarda durur), imlecin
    sayfası gösterilir, Enter ya da Boşluk imleçteki satırın nesnesini gösterir. Satırı ayrı durak yapmak Tab'ı 500 satırda
    yürütürdü. İmleç seçili satır vurgusuyla çizilir, komutun yeni satırlarıyla düşer. Gerçek pencerede: Yapı ara'da Ara,
    yedi Tab'la tablo (panel kaydı), ↓↓ ikinci yapı (150 m²), Enter haritayı ona götürüp seçti. `tree` ve `stats` fareyle
    kaldı (ağaçta grup satırları, göstergede tek sıra kart).
  - **Ağaç ve göstergeler** (aynı gün, sonra): `tree` ve `stats` de tek durak. Ağaçta ↑ ve ↓ satırlar arasında çizildikleri
    sırayla (grup, altındaki yapraklar, "… daha"), → kapalı grubu açar, ← açık grubu kapatır, Enter yaprağın nesnesini
    gösterir ya da grubu açıp kapatır; imleç satırın seçili vurgusuyla. Bunun için kitin `KeyFocus`'u sol ve sağı ayrı
    alabiliyor (`on_side`; verilmezse dört ok eskisi gibi adımdır): ağacın düzeltmesi bileşende. Göstergelerde oklar
    kartlar arasında, Enter kartın nesnesini gösterir, imleçteki kart seçili çizilir. Kit sınaması: `on_side`'lı öğede
    sol ve sağ ona, yukarı ve aşağı adıma; `on_side`'sız dördü de adım (`the_sides_go_apart_when_asked`). Davranış
    tablosunun ölçümü iki bileşende de tek odak buldu. Gerçek pencerede (senaryo 53, 54): Değer say'da Alan'dan dört Tab
    ağaca, ↓ "101" grubunu imledi, ← kapattı, → açtı, ↓ "parsel 1", Enter parseli seçip gösterdi, odak ağaçta kaldı;
    Yapı ara'da Özetle'den iki Tab kartlara (aradaki İpuçları başlığı), → dört kez "En büyük yapı 400 m²", Enter o
    yapıyı seçti. Boş tablo ("Bulunan yapılar", satır yok) Tab'ın turunda değil. Senaryo 20 ve 26'nın şerit
    koordinatları şeride yeni düğmeler girince eskimişti (Değer say 501 → 622, Yapı ara 417 → 546): düzeltildi.
  - Gerçek pencere (Vitrin, Yapı ara ve Parsel kartı): Yapı no'dan Tab → Katman → onay → Ara → Yapıları numarala →
    Özetle; Boşluk Özetle'yi çalıştırdı ("5 yapının özeti"), odak düğmede kaldı; Tab İpuçları başlığına, Boşluk açtı;
    Shift+Tab geri. Shift+Tab Kart tarihi'nden Seçimi al'a; Tab çiplerde, Boşluk çipi kaldırdı, komut sürerken basılan
    Tab sonraki çipe geçti.
  - **Odak görünür kalır** (kitte, `focus.rs`; WCAG 2.4.7, 2.4.11): açık temada, 16 px yazıda, 1100×650 pencerede dar
    panelde Tab görünür yerin altındaki Ara'ya, Yapıları numarala'ya, Özetle'ye geçiyor ama panel kaymıyordu: odak
    görünmüyordu. Tab'ın sayımı her öğenin içinde durduğu kaydırılabilir alanları ata zinciriyle izler (iced'in
    `scrollable`'ından hemen sonraki dolaşma onun içeriğidir); uygulama geçişi en içtekini öğe 8 px payla görünene dek
    kaydırır. Aynı yerde eski bir hata: iletişim kutusunun kapsamı öğenin kendi yerine bakıyordu, kaydırılan kutuda
    kenarın altında kalan alanlar Tab'ın adayı bile değildi; artık en dıştaki kaydırılabilir alanın görünür penceresine
    bakar. Kit sınaması: 100 px'lik kaydırılan sütunda 12 alan, her Tab'da odaktaki alan görünür (kaydırma kapatılınca
    3. Tab'da kırmızı). Gerçek pencerede panel her düğmeye kaydı.
- **Erişilebilirlik sınırı:** iced'de ekran okuyucu ağacı yok; tablonun "erişilebilirlik" payı klavye ve ad. Ad, KentOS'un
  bileşeni andığı yerdeki adıdır (hata iletisi, alanın yanındaki yazı).
- **SDK bölümü** (16 Kasım): tablo `extension.sdk part=panelBehaviour` ya da `kentos kek sdk --part panelBehaviour`
  ile JSON verilir (28 satır: `type`, `value`, `example`, `events`, `sets`, `rows`, `focus`, `keys`, `namedBy`, `note`).
  SDK paketi Asistan'ın araç sonucu sınırındaydı (47.998 / 48.000 bayt): yer açmak için `verifyCodes` da kendi bölümüne
  alındı (1,3 KB; pakette yalnız nasıl alınacağı kalır, her bulgu zaten iletisi ve yeriyle gelir). Paket 47.153 bayt.

## §72. Python'un yapmaması gerekenler kurulumda da: çalışırken paket kurma, `sys.path`, beyansız ağ (`ADIM 21`, "AST kuralları")

**16 Kasım.** Planın ADIM 21 dersi: "çalışma anında paket kurma/indirme, kendi kendini güncelleme, `sys.path` oynama
yasak; beyan edilmemiş yetenek kullanımı (ağ, dosya, alt süreç, pano) ret; görünmez Unicode ret". Durum ve karar:

| Kural | Kod | Önce | Şimdi |
|---|---|---|---|
| Görünmez Unicode (sıfır genişlik, yön denetimi, değişken seçici, etiket…) | `text_invisible` | Kapı 1, ret (`text.rs`, GlassWorm) | aynı |
| Çalışırken paket kurma: `"pip"`/`-m pip`, `pip.main(`, `pip._internal`, `pip install`, `ensurepip`, `easy_install`, `import pip` | `python_runtime_install` | yok | **ret** |
| Modül yolunu değiştirme: `sys.path` ekleme/silme/atama, `site.addsitedir(`, `sys.meta_path`, `PYTHONPATH` ataması | `python_sys_path` | yok | **ret** |
| `network` istemeden ağ kitaplığı (socket, requests, urllib, httpx, aiohttp, http) | `python_network_undeclared` | Kapı 2, uyarı | **ret** |
| İşletim sistemi komutu (subprocess, `os.system`, `os.popen`) | `python_os_command` | Kapı 2, uyarı | uyarı (Kapı 1) |
| `eval`/`exec` | `python_eval` | Kapı 2, uyarı | uyarı (Kapı 1) |

- **Kapı 2'den Kapı 1'e:** kurallar `python_rules.rs`'te ve arşiv denetiminin (`check_tree`) içinde: paketlemede de,
  `.kek` kurulurken de işler. Önce yalnız klasör doğrulamasındaydı (`kek verify`, `extension.verify`); `verify`'dan
  geçmeden kurulan bir `.kek` onlarla hiç karşılaşmıyordu. Kapı 2 artık yalnız işleyicilerin işaretlerini denetler.
- **Neden ret:** paketin bağımlılıkları `pyproject.toml` ve `uv.lock`'tur; KentOS ortamı eklenti açılmadan hazırlar,
  kullanıcı neyin kurulduğunu görür ve onaylar (§5). Çalışırken kurulan paket bu onayın ve kilitli özetlerin dışında.
  `sys.path` incelenen paketin dışından kod yükler. Beyansız ağda kullanıcı, eklentinin yaptığından azına onay vermiştir.
- **Alt süreç ve pano uyarı kaldı:** manifestte onları söyleyen yetenek yok (`Capability::ALL`, 16). Rete çevirmek önce
  `process.spawn` ve pano yetenekleri ister (onay metni, katalog incelemesi): sahibin kararı.
- **Kendi kendini güncelleme:** kurulu dosyalar her etkinleştirmeden önce içerik özetine karşı denetleniyor
  (`extension-host` `verify_installed`); kendi dosyasını değiştiren paket bir sonraki açılışta "değişmiş/sonradan
  eklenmiş" diye durur. İndirdiği metni çalıştırmak `eval`/`exec` uyarısı ve ağ yeteneğiyle görünür.
- **Metin kuralı, sınırı yazılı:** Python içe aktarılmaz, çalıştırılmaz (paylaşılan crate bağımlılıksız kalır).
  Belge dizgileri ve yorumlar kod sayılmaz ("Kurulum: pip install numpy" bir nottur); kararlı bir yazar dolanabilir
  (`getattr(__import__(…))`). Dürüst hatayı ve ajanın betiklerden getirdiği alışkanlığı
  (`subprocess.check_call([sys.executable, "-m", "pip", "install", …])`) yakalar; katalog incelemesi ve işletim sistemi
  yalıtımı ayrı katmanlar.
- **Sınamalar:** `python_rules` 2 (belge dizgisi/yorum ayrımı; her kural satırıyla, `pipeline`, `retrieval(`,
  `pip_tools_like` kural sayılmaz), `verify` 2 (beyansız ağ ret, komut uyarı ve paketlenir; çalışırken kurma ve
  `sys.path` satırlarıyla ret, belge dizgisindeki not değil). Vitrin ve MEBİS kurallara takılmıyor.

## §73. Başsız koşuda işletim sistemi penceresi gerekmedi; susturulan söylenir (`ADIM 19`, §70'in gözden geçirilmesi)

**16 Kasım.** Sahip: "gerekli görülen yerlerde işletim sistemi penceresi açılabilir … öyle bir kısıtımız yoktur" (alt
süreçlerin Windows'ta konsol penceresi açmaması ayrı kural, sürüyor). §70'in başsız yolunda "pencere yok" diye bir şeyi
reddeden, düşüren ya da susturan her yer tek tek gözden geçirildi; her biri için pencerenin doğru cevap olup olmadığına
karar verildi. Kod: `crates/native/extension-tools` (yeni `note.rs`: `Note::record`, `said`, `failed`; `headless.rs`,
`calls.rs` `NO_QUESTIONS`), MCP `server.rs` ve `extensions.rs`, Python bağı `lib.rs` (`say_notes`),
`python/kentos/ext_host.py` (`failure_words`), masaüstü `tools.rs`.

| Yer | §70'te | Karar |
|---|---|---|
| Aracın kodunda `ctx.dialog`, `ctx.confirm`, `ctx.open_panel` | başsızda ve **masaüstünde de** ret (`unknown_method`: "yalnız processing.run") | Pencere değil. Araç çalışırken sormaz, her konakta: sorulacak olan aracın parametresidir; araç penceresi, toplu çalıştırma, model, ajan ve sunucu işi onu çalıştırmadan önce verir, çalıştırma kaydı (türetim, PY-TOOL-22) onu tutar. Yarıda verilen cevap kayda girmezdi, toplu iş ve sunucu işi kimseyi bekleyemez. Ret artık nedeni ve çaresiyle (`capability`, `calls::NO_QUESTIONS`); kullanıcıyla konuşan iş komuttur (`@ext.command`). |
| Hata bildirimi ve Ayrıntı | MCP standart hataya, Python hiçbir yere | Pencere değil: cevabı çağıran (ajan, betik) alır, araya giren pencere otomasyonu durdururdu. Başsız koşu eklentinin günlüğüne masaüstünün yazdığı kaydı yazar (başladı, bitti, hata iz kaydı ve yazdırdıklarıyla; `program` hangi konak): masaüstünün Ayrıntı'sı ve `kentos://extensions/installed` başsız koşuyu da gösterir. |
| "Python kitaplığı sessiz" | notlar düşüyordu | Python'un logging'i (`kentos.ext`): hata uyarı olarak, iz kaydıyla (betik bir şey ayarlamadıysa standart hatada görünür); başladı ve bitti bilgi olarak. Not, çalıştırma yorumlayıcının kilidi bırakılmışken gelir; çağrı dönünce söylenir (en çok 200 bekler). |
| Bekçinin çökme sayacı | yazılmıyordu | Masaüstünün kalır: sayaç etkinleştirmenin kirasına bağlı, başsız konak etkinleştirmez; iki sürecin ardışık çökme sayımını birlikte tutması ayrı bir iş. Çökme günlükte hata olarak durur (`runner_crashed`, iz kaydıyla). |
| Python kapalı, kurulu değil, ortam hazırlanmadı | ret, yeri söylenir | Pencere değil: Python kurmak ve ortam hazırlamak onaylı, ilerlemeli masaüstü akışı (indirilecekler, paket önizlemesi); başsız konak indirmez. Ret yeri söylemeye devam eder. |
| Yalnız masaüstü aracı | ret | Pencere değil: tanımın bildirimi (§70). |

- **Sonuç: bu yolda işletim sistemi penceresi açılan bir yer çıkmadı**, ekran yokken (CI, sunucu, `DISPLAY` yok, Windows
  hizmeti) ret de değişmedi. Araçların çalışırken sorması istenirse bu bütün konaklarda bir sözleşme değişikliği olur:
  masaüstünde çizimin üstünde, başsızda masaüstü programının tek pencerelik bir soru kipiyle (ADR 8060'ın
  `Dialog::window`'u, eklenti penceresinin bileşenleri), gözetimsiz koşularda ret ve cevabın çalıştırma kaydına
  yazılmasıyla; ayrı karar.
- **Uzak konağın reddi okunur:** kodun yakalamadığı `HostError(code, message)` başarısızlıkta demet olarak yazılıyordu
  ("HostError: ('capability', 'Bir …')"); artık sözü ve kodu ("HostError: Bir İşlemler aracı … (capability)").
- **Tek kayıt biçimi:** başladı kaydını masaüstü de `Note::record`'dan yazar; MCP'nin kendi satır biçimi kalktı
  (`Note::said`).
- **Gerçek koşu (yalıtılmış klasörler; Vitrin kopyasının Çekme mesafesi aracına yalnız bu denemede 19 m'de soran, 20
  m'de hata veren iki satır):** MCP: 3 m soğuk 2,98 sn (kurulumun tekerleği değişmişti, ilk koşu kitaplığı ortama koydu:
  6,32 sn), ılık 0,04 sn (§70'te 0,03); 19 m 0,01 sn'de "HostError: Bir İşlemler aracı çalışırken kullanıcıya sormaz …
  (capability)"; 20 m "ValueError: deneme hatası: 20 m". `kentos://extensions/installed` Vitrin'in son kayıtlarını
  `kentos-mcp`'nin yazdığı iz kayıtlarıyla verdi, `problem: run_failed`. Python bağı: yapılandırmasız betikte iki uyarı
  iz kaydıyla standart hatada, başarılı koşu sessiz; `logging.basicConfig(level=INFO)` ile "INFO kentos.ext: … Bitti
  (0,9 sn; süreç belleği 128 MB)." satırları. Dört oturumda (ikisi MCP, ikisi Python) günlüğe 28 kayıt eklendi
  (`kentos-mcp` 16, `python3` 12), kayıt başına ortalama 703 bayt (hata kayıtları iz kaydıyla büyük).
- **Sınamalar:** extension-tools 6 (yeni: notun kaydı ve satırı; başsız koşunun günlüğe yazdığı başladı ve bitti, iz
  kayıtlı hata ve yazdırdığı; soru ve panel isteğinin nedeniyle reddi), `python/tests/test_ext_host.py` 28 (2 yeni:
  reddin sözü, eklenti sürecinin işareti), masaüstü eklenti araçları, deneme ve Python sınamaları geçti.
- **Kalan:** bekçi sayacının başsız koşuyu da sayması; ifade işlevleri başsızda; sunucu ve bulut koşusu kentosd'nin
  yolu.

## §74. `kentos kek test`: eklentinin klasörü pencere açmadan, sürekli tümleştirmenin sınadığı gibi (`ADIM 21`, "başsız test koşucusu")

**16 Kasım.** Kod: komut satırı `apps/cli/src/kek_test.rs` (yeni), `main.rs` (`verified`, `print_placed`);
`kentos-extension-tools` `examples.rs` (yeni, masaüstünden taşındı: `of`, `run`, `said`), `headless.rs`
(`Headless::every_runtime`), `synthetic.rs` (`TRIAL_SRID`); `kentos-python-env` `extension::prepare_words` (masaüstünden
taşındı); `kentos-headless` `agent_tools::with_tool_problems` (artık genel); masaüstü `examples.rs` (yalnız pencerenin
bağı kaldı); `scripts/arch/deps.mjs` (`cli` grubu).

- **Karar: bir komut, dört adım.** Örnek ekosistemin başsız sınama imgesinin deseni (geçici profil, klasörden kurulum,
  engelleyici pencere yok, etkinleştirme, başsız koşucu [K16]; araştırma notunun "Al" satırı):
  1. **Doğrulama** `kek verify` gibi; ret sınamayı bitirir.
  2. **Kendi kayıt defteri:** geçici klasörde, paketin istediği her yetenek verilmiş (sınama yazarın kendi işi), kurum
     ilkesi bilgisayarınki; sonda silinir. Kullanıcının eklentilerine dokunulmaz.
  3. **Ortam kilidinden,** KentOS'un Python klasöründe, masaüstünün anahtarıyla (§6): aynı kilidin ortamını sınama ve
     masaüstü paylaşır, ikinci kez indirilmez. KentOS'un Python'u yoksa ret, nedeni ve çaresiyle; `--install-python` uv
     ile kurar (çağıranın açıkça istediği indirme). Masaüstünün "Python açık" ayarı aranmaz: komut çağıranın kendi
     işidir, kullanıcının masaüstünde eklenti çalıştırma seçimi değildir.
  4. **Örnekler** (§48): her aracın, yalnız masaüstünü bildirenler de (kod iki konakta aynı yoldan çalışır;
     `Headless::every_runtime`, kullanıcının eklentilerini sunan konak bunu hiç yapmaz), sürümün sürecinde, sentetik
     çizimlerde, her biri beklenene karşı.
- **Çıkış kodları:** 0 geçti (ya da çalıştırılacak örnek yok), 1 eklenti kaldı (doğrulanmadı, kilidi kullanılamadı, bir
  örnek kaldı), 2 sınanamadı (yanlış komut, okunamayan klasör, uv yok, Python yok, ağ, disk). Sürekli tümleştirmede
  1 ile 2'nin ayrı olması: biri yazarın düzelteceği, öbürü makinenin.
- **Rapor:** metinde her örnek ✓/✗, süresi ve nedenleri; kodun hatası "Eklentinin söyledikleri" altında iz kaydıyla.
  İz kaydındaki yol yazarın klasörüne çevrilir: kurulu geçici kopyayı değil, düzenlediği dosyayı ve satırı gösterir.
  `--json`: `status` (`passed`, `failed`, `refused`, `not_run`), `verify`, `extension`, `environment` (`python`,
  `packages`, `made`, `ms`), `examples` (§48'in raporu), `notes`, `summary`.
- **Tek örnek koşucusu:** masaüstünün Örnekleri çalıştır'ı, ajanın denemesi (`extension.workshop.trial`) ve
  `kek test` aynı `examples::run`'ı çağırır.
- **`kek verify` de tanımı tam okur:** komut satırı İşlemler'i artık taşıdığından araç tanımları KentOS'un okuduğu gibi
  okunur (`with_tool_problems`, ajanların `extension.verify`'ının kodu); "yalnız biçimiyle denetlendi" notu kalktı.
  Örnek: `sinkParameter`'sız bir nesne çıktısı (`features`) önce yalnız Kur'da ve `extension.verify`'da reddediliyordu,
  `kek verify` geçiriyordu; artık o da "outputs[2]: yeni nesneler bir katman parametresine yazılır; sinkParameter onu
  adlandırmalı." diyor.
- **Bedel:** komut satırı başsız konağı taşıyor (hata ayıklama derlemesi 646 MB); `cli` grubunun izinleri İşlemler'e
  ve başsız konağa genişledi, sunucu, pencere ve tarayıcı yine yasak.
- **Gerçek koşu (16 Kasım, yalıtılmış Python klasörü):** Vitrin 1.0.51 hazır ortamla 2,56 sn (ortam 0,65 sn; TAKS örneği
  1,63 sn, Çekme 0,04 sn), "2 örneğin hepsi geçti.", çıkış 0. Boş Python klasöründe `--install-python`: uv Python
  3.12.13'ü kurdu, ortam kilitten (numpy 2.5.4, shapely 2.0.7) hazırlandı, iki örnek geçti; ortam dahil 11,87 sn, toplam
  16,5 sn. Kopyada bozulan beklenti (`asan: 2`) ve hata atan araç: "“asan”: 2 beklendi, 1 geldi", "ValueError: sınama
  hatası: 3 m" ve iz kaydı yazarın `src/vitrin/logic.py`'sinin 361. satırıyla, çıkış 1. Python'suz klasör: nedeni ve
  çaresi, çıkış 2. Geçersiz sürüm: "Doğrulanmadı: 1 sorun.", çıkış 1.
- **Sınamalar:** extension-tools (her ortamın araçlarını listeleyen sınama konağı; örnek koşucusunun geçen ve kalan
  örneği, kalanın nedeni), komut satırı (araç tanımının tam okunması), masaüstünün örnek ve deneme sınamaları.
- **Kalan:** sürekli tümleştirme şablonu (komut satırının kitaplık tekerleğiyle indirilebilir bir dağıtımı yok; şablon
  onu bekler); denetim kurallarının ve ifade işlevlerinin örnekleri (bugün yalnız İşlemler araçlarının örneği var);
  paketin kendi sınamaları (`tests/`, pytest) eklentinin ortamında; `kek serve`.
- **Uygularken verilen kararlar (planda yazılı değildi):** sınamanın kayıt defteri geçici, ortamı kalıcı ve
  masaüstününkiyle ortak; masaüstü ayarı aranmaz, Python yalnız bayrakla indirilir; yalnız masaüstü araçlarının
  örnekleri de çalışır; 1 ve 2 ayrı çıkış; iz kaydının yolu yazarın klasörüne çevrilir; `kek verify` araç tanımını tam
  okur.

## §75. Denetim kuralının örnekleri: olgular ve beklenen bulgular (`ADIM 20`–`21`, §48'in kurallara uzanması)

**16 Kasım.** Kod: paylaşılan `kentos_extensions::audit_examples` (yeni: `AuditExamples`, `read`, `json_schema`),
`manifest.rs` (`AuditEntry::examples`, `FileRole::AuditExamples`), `archive.rs` (Kapı 1), `sdk_pack.rs`
(`auditExamplesSchema` bölümü, katkı listesinde `examples`), `sdk/kentos-audit-examples.schema.json`;
`kentos-extension-tools` `examples.rs` (`Rule`, `rule_examples`, `one_rule`), `run.rs` (`run_in_pool`, `answered`);
komut satırı `kek_test.rs` (`rules_of`); masaüstü `examples.rs` (`ext_example_rules`), `view.rs`, `tools.rs`
(`answered`); ajan aracı `extension.workshop.trial`'ın sözü; Vitrin 1.0.52; `tr.kentos.mevzuat` 0.3.1.

- **Karar: örnek kuralın yanında, kendi dosyasında.** Kuralın tanım dosyası yok (araç örneği tanımındadır, §48);
  manifestin denetim girişi örnek dosyasını adlandırır: `"examples": "audits/<kimlik>.examples.json"`
  (`kentos.auditExamples` 1, en çok 10 örnek). Manifeste satır içi yazılmadı: mevzuat örneklerinin mimari olguları
  kilobaytlarca.
- **Karar: kural olgu okur, örnek olgu verir.** İki kaynak, birlikte ya da ayrı: `features` (katmanı adıyla, GeoJSON
  şekli, öznitelikler; en çok 500) sentetik çizim olur ve her çizim gibi ölçülür (`facts.layers`, `facts.project`:
  çakışmalar, boşluklar); `architecture` mimari çizimde ölçülenin kendisidir (`facts.project.architecture`: katlar,
  mahaller, bağımsız bölümler, kapılar), duvar ve mahal GeoJSON'la yazılamadığı için veri olarak verilir ve olguların
  türleriyle okunur (eksik zorunlu alan Kapı 1'de ret). Böylece mevzuat kuralı mimari çizim kurmadan sınanır.
- **Beklenen:** `count` (tam bulgu sayısı; 0: kural bir şey bulmaz) ve `has` (bulunması gereken bulgular: önemi,
  sözünden bir parça, adlandırdığı nesne `features`'taki sırasıyla, düzeltmesinin komutu; verilen her anahtar
  uymalı). Kalan örnekte rapor eksik bulguyu tarif eder ve gelenlerin ilk üçünü söyler; kuralın kodu hata verirse iz
  kaydı (`detail`) yazarın dosyasını gösterir.
- **Kapı 1 dosyayı kod çalıştırmadan okur:** biçim, örnek sayısı, ad, nesnenin katmanı ve şekli, boş beklenti, sayısı
  bulunacaklardan az olan beklenti, olmayan nesneye başvuru, bilinmeyen anahtar (yazım hatası satırıyla); her biri
  dosyası ve JSON yeriyle (`audit_examples`). Örnek: "audits/yapi-no.examples.json /examples/0/expect/has/0/feature:
  sırası 5 olan nesne yok: features'ın sırası 0'dan başlar (2 nesne var)."
- **Çalıştıranlar aynı kod:** Eklentiler › Örnekleri çalıştır (düğme kuralın örneği varsa da görünür), ajanın
  `extension.workshop.trial`'ı (`examples: true`) ve `kentos kek test`. Kural, konağın onu çalıştırdığı yoldan
  koşar: masaüstünde Eklenti denetiminin havuz işi, başsızda sürümün havuzdaki süreci; ikisinin sonucu aynı okunur
  (`answered`, masaüstündeki eş kod paylaşılana indi).
- **Vitrin 1.0.52:** "Yapı numarası" (numarasız yapı sıradaki numarayı ve düzeltmesini alır; numaralı yapıda bulgu
  yok), "Parsel çakışması" (üst üste binen iki parsel 20.00 m² çakışma hatası; komşu iki parselde bulgu yok).
- **İskelet örnekleriyle gelir:** `kek new` / `extension.blueprint`'in denetim kuralı örnek dosyasıyla (adsız nesne
  bulunur, düzeltmesi ad yazar), İşlemler aracı tanımında bir örnekle (iki kare alan, toplam 500 m²): yeni bir
  eklenti `kek test`'ten kutudan çıktığı gibi geçer ve yazan ajan örnek yazmayı iskeletten görür.
- **`tr.kentos.mevzuat` 0.3.1, yedi kuralın 20 örneği:** her kural en az bir hata ve bir bulgusuz örnekle; sınırlar
  yönetmeliğin sayılarıyla: PAİY 29/1 (8 m² yatak odası: "net alanı 8.00 m² (en az 9.00 m²)"), 29/3 (1.00 m hol; 1.20
  m sınırda bulgusuz), BYKHY 4/i ve Ek-5/B (18 m çıkmaz koridor yağmurlamasız hata, yağmurlamayla 20 m'ye kadar ve 12
  m'de bulgusuz), 48/2 (kapıya 24 m hata, 18 m bulgusuz, kapıya yolu olmayan mahal, 4 katlı konut binasında 48/1
  muafiyeti; iki katlı dairenin üst kattaki tek kapısı, ayrı çıkışsız üst katın net 72 m²'si), 48/3-4 (kapıdan tek
  yönde 22 m yağmurlamasız hata, yağmurlamayla 30 m'ye kadar bulgusuz, çıkışa ulaşmayan kapı), 31/3 ve 33/6-b (kat
  çıkışı olmayan kat, çıkışa ulaşmayan mahal). Örnekler kaçış yürüyüşlerini (`escape`, kapıların `exit`'i), katları ve
  yağmurlama ayarını olgu olarak verir; konut örnekleri 48/1 muafiyetinin dışında kalmak için beş katlı. Kuralların
  kodu değişmedi; 20 örneğin hepsi ilk koşuda geçti.
- **Gerçek koşu (16 Kasım):** `kek test` Vitrin: 6 örneğin hepsi geçti (ortam 1.0.52 için kilitten 3,54 sn, toplam
  6,9 sn; ilk kural örneği süreci başlatırken 1,49 sn, sonrakiler 0,00 sn). Mevzuat: 20 örneğin hepsi geçti (hazır
  ortamla toplam 2,5 sn). Bozulan beklenti: "(warning, “sıradaki numara Y-03”, features[1], düzeltme cad.entities.set) bulgusu
  gelmedi" ve "gelenler: warning “Yapının numarası yok; sıradaki numara Y-02.”"; hata atan kural: "kural hata verdi:
  KeyError: 'alan'" ve yazarın `src/vitrin/logic.py`'sinin satırıyla iz kaydı, çıkış 1.
- **Sınamalar:** paylaşılan crate 3 (dosyanın tamamı okunur; her yanlış yeriyle; şemanın türlerden üretilmesi),
  extension-tools 1 (stand-in kuralla: sayı, önem, söz, nesne, düzeltme; eksik bulguda gelenler; kodun hatası iz
  kaydıyla; verilen mimari olguların kurala ulaşması), komut satırı ve masaüstü sınamaları.
- **Kalan:** ifade işlevlerinin örnekleri; bir bulgunun mimari olgudaki mahali kimliğiyle beklenmesi (bugün yalnız
  sözüyle).
- **Uygularken verilen kararlar (planda yazılı değildi):** örnek ayrı dosyada ve manifestte adıyla; mimari olgu veri
  olarak; `has`'ın nesnesi `features`'taki sırayla; eksik bulguda gelenlerin ilk üçü; dosyanın biçimi `format` ve
  `schemaVersion` taşır (öbür biçimler gibi); mevzuat 0.3.1 yalnız örnek ekler.

## §76. Yapay zekânın yazdığı eklentinin kökeni: `generated`, KentOS'un damgası (`ADIM 21`, "Üretim kökeni"; D17)

**16 Kasım.** Kod: paylaşılan `kentos_extensions::provenance` (yeni: `Generated`, `stamp`, `words`), `manifest.rs`
(`Manifest::generated`, denetimi), `catalog.rs` (`CatalogEntry::generated`), manifestin JSON Schema'sı; başsız
`agent_tools.rs` (`Host::author`, `Author`, `stamp_author`; `workshop.review` ve `.reload`); masaüstü
`assistant/host.rs` (Asistan'ın profili ve modeli), `extensions/view.rs` (inceleme, kurulu eklenti, katalog girdisi);
MCP `server.rs` (`clientInfo`, `client_of`), `tools.rs` (`WithWorkshop::author`); komut satırı (`kek inspect`,
`kek verify` "Yazan:" satırı).

- **Karar: köken KentOS'un sözü, ajanın değil.** Atölyede paketlenen her şeyi bir ajan yazmıştır; KentOS atölyeyi
  paketlerken (`extension.workshop.review`, `.reload`) klasörün manifestine `generated` yazar: ajan (KentOS Asistan ya
  da MCP istemcisi, `initialize`'da kendini tanıttığı adıyla), konak biliyorsa modeli (Asistan'ın profili: "DeepSeek ·
  deepseek-chat"; MCP modeli söylemez) ve atölyenin KentOS'u (ajanın okuduğu SDK ve iskelet onunkidir; planın
  "blueprint sürümü"). Ajanın kendi yazdığı değer değiştirilir. Başka yerden gelen paketteki değer yazarlar listesi gibi
  üreticisinin beyanıdır.
- **Zaman yazılmaz:** aynı klasör yeniden paketlenince aynı paket (içerik özeti kararlı); zamanı kayıt defterinin kurulum
  anı ve kataloğun yükleme kaydı tutar.
- **Kullanıcı karar verirken görür:** kurulum incelemesinde "Yazan: Yapay zekâ: KentOS Asistan (DeepSeek ·
  deepseek-chat), KentOS 0.1.0 atölyesinde" ve "Bu eklentiyi bir yapay zekâ ajanı yazdı. İstediği yetenekleri okuyup
  karar verin: kurulursa kodu sizin haklarınızla çalışır." (yalnız bilinen: yapay zekânın yazdığı bir paket bir katalogun
  incelemesinden geçmiş olabilir; atölyeden kurulurken atölyenin kendi şeridi nereden geldiğini söyler);
  kurulu eklentinin ayrıntısında ve kataloğun girdisinde aynı "Yazan" satırı. Yıldız, puan yok: düz bir olgu (D17).
- **Katalog kaydında:** manifestle birlikte (`ReleaseFacts.manifest`) ve dizinin girdisinde (`CatalogEntry`); listede
  süzgeç ve karnede gösterim ADIM 22A'nın katalog yüzeyinin.
- **Kapı 1:** ajanın ve modelin adı 1–120 karakter, `kentos` bir sürüm (`manifest_generated`, yeriyle).
- **Sınamalar:** paylaşılan crate 1 (damga ajanın değerini değiştirir, kalanı korur, ikinci damga aynı bayt; sözü;
  kuralları yerleriyle), başsız atölye (paketin ve klasörün manifestinde konağın ajanı, modeli ve KentOS'u), masaüstü
  Asistan (kullanıcının kurduğu eklentinin kaydında "KentOS Asistan").
- **Kalan:** kataloğun listesinde "yapay zekâ üretimi" süzgeci ve karnede gösterim (22A); MCP istemcisinin modeli
  (protokol söylemiyor).
- **Uygularken verilen kararlar (planda yazılı değildi):** köken manifestte, atölye paketlerken klasöre yazar; zaman
  yok; MCP istemcisinin kendi verdiği ad; kurulum incelemesinde bilgi şeridi.

## §77. SDK'nın düz metni: `llms.txt` (`ADIM 21`, "düz metin SDK belgesi")

**16 Kasım.** Kod: paylaşılan `kentos_extensions::sdk_text` (yeni: `llms_txt`), depoda `sdk/llms.txt`; komut satırı
`kentos kek sdk --text`; MCP `kentos://sdk/llms.txt` (`text/markdown`).

- **Karar: paketten üretilir, elle yazılmaz.** `kentos://sdk/ai-kek-pack` JSON'dur; editördeki bir kodlama ajanı, bir
  modelin bağlamı ya da belgelerde arama düz metin okur. Metin paketin kendisinden kurulur: başlık, bir paragraflık
  alıntı (önerilen `llms.txt` biçimi), rehber (başlıkları bir düzey aşağıda; kod bloğunun `#` satırları olduğu gibi),
  adımlar, yetenekler ve kapsamları, katkılar (manifest anahtarı, işareti, dosyası, istediği yetenek), İşlemler aracı
  tanımının sözlüğü, her bölümün nasıl alınacağı ve `kentos.ext` imzaları. KentOS'un bağlantı verecek bir sitesi yok:
  bölümler metnin kendisini taşır; şemalar ve ikon listesi kendi bölümünde kalır (alıntı ve "Bölümler" söyler).
- **Depoda durur, sapınca kırmızı:** `sdk/llms.txt` öbür SDK dosyaları gibi `KENTOS_WRITE_SDK=1` ile yazılır; sınaması
  her yeteneğin, katkının ve bölümün metinde olduğunu ve biçimi (başlık, boş satır, alıntı) denetler.
- **Ölçü:** 46.434 bayt (rehber 16,5 KB, imzalar 21 KB). Asistan'ın araç sonucu sınırı (48 KB) bu metnin işi değil:
  Asistan JSON paketi okur.
- **Gerçek okuma:** MCP'nin `resources/list`'i iki SDK kaynağını verdi (`application/json`, `text/markdown`),
  `resources/read` metni 45.047 karakter olarak.
- **Kalan:** sürekli tümleştirme şablonu, `kek serve`.

## §78. İfade işlevinin örnekleri: çağrı ve değeri (`ADIM 19`–`21`, §48 ve §75'in ifadeye uzanması)

**16 Kasım.** Kod: paylaşılan `manifest.rs` (`ExpressionEntry::examples`, `ExpressionExample`, denetimi), manifestin
JSON Schema'sı, `blueprint.rs`; `kentos-extension-tools` `examples.rs` (`Examples`: araçlar, kurallar, ifadeler; tek
`run`), `examples/expressions.rs` (yeni: `Expression`, `expression_examples`, `one_expression`); masaüstü
`examples.rs` (`ext_examples_of`), `view.rs`; komut satırı `kek_test.rs` (`coded_of`); Vitrin 1.0.53.

- **Karar: örnek manifestte, satır içi.** İfadenin örneği küçüktür: bir çağrının argümanları (metin, sayı, true/false
  ya da null; en çok 10) ve vermesi gereken değer: `"examples": [{"args": ["101", "1"], "expect": "101/1"}]`, en çok
  10 örnek, adı (`title`) isteğe bağlı: yoksa çağrı ifadenin yazdığı gibi okunur (`ext.tr.ornek.vitrin.ada-parsel("101",
  "1")`). Kuralın örneği büyük olduğu için ayrı dosyadaydı (§75); bunun için dosya gerekmedi.
- **Koşu etiketin çağırdığı gibi:** sürümün sürecinde `kind: expression`, bir satır argüman; değer §48'in
  karşılaştırmasıyla (sayı milyarda bir payla, kalanı yazıldığı gibi; null null'a eşit). Başka değer verirse ikisi de
  söylenir ("\"9/9\" beklendi, \"1/2\" geldi"), kodu hata verirse nedeni ("işlev hata verdi: ValueError: …").
- **Tek koşucu:** araçların, kuralların ve ifadelerin örnekleri `Examples` yapısında birlikte, aynı `run`'la; masaüstünün
  Örnekleri çalıştır'ı (düğme ifadenin örneği varsa da görünür), ajanın denemesi ve `kek test`.
- **Kapı 1:** örnek sayısı, argüman sayısı, argümanın ve beklenenin düz değer olması, adın uzunluğu, yeriyle
  (`manifest_expression_example`).
- **Vitrin 1.0.53:** "Ada/parsel yazımı" üç örnekle: `("101", "1")` → `"101/1"`, sayı olarak gelen ada (`101.0`) →
  `"101/4"`, boş parsel → değer yok. İskeletin "Türkçe büyük harf"i: `"istanbul"` → `"İSTANBUL"`, `"ılık"` → `"ILIK"`,
  `null` → `null`.
- **Gerçek koşu:** `kek test` Vitrin "9 örneğin hepsi geçti." (iki araç, iki kuralın dört, ifadenin üç örneği); iskelet
  3/3.
- **Sınamalar:** extension-tools 1 (stand-in işlevle: değer, null, başka değer, kodun hatası, çağrının adı), paylaşılan
  crate'in manifest sınamaları, masaüstü örnek ve deneme sınamaları.

## §79. Bölümün kullanım sınıfı olgularda; konut olmayan bölümlerin kaçış uzaklığı (`ADIM 20`, ulusal kuralların dördüncü dilimi)

**16 Kasım.** Kod: `kentos_domain::fire_use` (yeni: `Use`, `use_of`, Ek-5/B'nin tablosu; yerel yangın denetiminden
taşındı, `fire_audit` onu kullanır), paylaşılan olgular (`Occupancy`, `UnitFacts::occupancy`), İşlemler'in olgu üreticisi
(`occupancy_of`), `python/kentos/ext.py` (`UnitFacts.occupancy`), olgu fikstürü, JSON Schema'lar, `kentos_ext.pyi`;
`tr.kentos.mevzuat` 0.4.0 (`kacis.py`, `kacis-uzakligi`, yedi örnek); jobrun'ın mevzuat sınaması.
Araştırma: `docs/research/2026-11-16-tr-mevzuat-denetim-kurallari.md` (Ek-5/B'nin bütün satırları, resmî sayfanın
görüntüsüyle karşılaştırılmış).

- **Karar: tek okuma.** Mimari denetimin yangın kuralları bölümün Ek-5/B satırını niteliğinden (Büro, Dükkân, Otel …)
  ve mahalin adından okuyordu; olgular okumuyordu, araştırma notu bunu "modelde yok" diye dışarıda bırakmıştı. Okuma
  `kentos_domain`'e taşındı; olgu üreticisi ve yerel denetim aynı kodu çağırır. Olgu okumanın kendisidir: niteliği
  bir sınıf söylemeyen bölümde `occupancy` yoktur (politika kuralın: yerel denetim gibi büro sayar ve söyler).
  Ek bölüm sahibinin niteliğiyle okunur; ortak ve dış alanın sınıfı yoktur. Olguların sürümü değişmedi (isteğe bağlı
  yeni alan; eski olgular alansız okunur).
- **Kural `kacis-uzakligi`** (BYKHY 32/3-4, Ek-5/B): konut olmayan bölümün her mahalinden en yakın kat çıkışına
  yürüyüş; tek çıkışta bütün yol tek yön sınırına, iki çıkışta önce tek yönlü kısım tek yön, sonra bütün yol iki yön
  sınırına; projenin yağmurlama ayarının sütunu (bilinmiyorsa yağmurlamalıyı da aşan hata, yalnız yağmurlamasızı aşan
  uyarı). Sınıfı okunamayan bölüm binanın bilinen en ağır sınıfıyla (kısa iki yön, sonra kısa tek yön), o da yoksa büro
  sayılır ve bulgu bunu söyler. Konut olmayan bölümü olan binanın ortak mahalleri binanın sınıfıyla ölçülür; konutun ve
  yalnız konut olan binanın mahalleri okunmaz (48'in kuralları). Endüstri yapısında dipnot (1)'in ½ artışıyla uygun olan
  uyarıdır ("karar verilemedi": malzemenin kolay alevlenici olup olmadığı olgularda yok).
- **Bütün çizimde** (fikstür, bulut işinin yolu): 1. katta ortak koridor tek çıkışlı katta 21.82 m (binanın sınıfı
  mağaza: eşit ağırlıkta ilk bölüm, yerel denetimin `stricter`'ı gibi) ve büronun Arşiv'i 22.52 m: iki hata.
- **Örnekler (7):** büro tek çıkış 18 m hata, yağmurlamayla bulgusuz; dükkân iki çıkış 50 m hata, 40 m bulgusuz;
  niteliği sınıf söylemeyen bölüm (Kuaför) büro sayılır; endüstri yapısı 20 m "karar verilemedi"; yalnız konut bina
  bulgusuz. `kek test`: mevzuatın 27 örneğinin hepsi geçti.
- **SDK paketi:** `UnitFacts`'in yeni alanı ve belge cümlesi pakete 102 bayt ekledi (47.992 / 48.000); sınır
  yükseltilmeden belge cümlesi ve rehberin `kek test` cümlesi kısaltıldı: 47.890.
- **Sınamalar:** olgu fikstürü (beş bölümün sınıfı), jobrun 12 (sekiz kural, yeni kuralın iki bulgusu), Python 7
  (`occupancy`'nin okunması), paylaşılan crate 149.
- **Gerçek pencere (KentOS Mimari, yalıtılmış profil):** 0.4.0 katalogdan kuruldu; Örnekleri çalıştır "Türkiye
  mevzuatı: örnekler — 27 örneğin hepsi geçti.". İnceleme penceresi paketin eskimiş açıklamasını gösterdi ("o
  bölümlerin kaçış uzaklığına karar verilmez"): açıklama 0.4.0'ın davranışına göre düzeltildi.
- **Kalan:** mahalin adından sınıf (yerel denetim okuyor, olgular bölümünkini taşıyor); yüksek tehlikeli yer
  (okunmuyor); bölüm sınıfının programda açıkça seçilmesi (bugün nitelikten okunur).

## §80. İfade işlevleri başsız konaklarda (`ADIM 19`, §70'in ifadeye uzanması)

**16 Kasım.** Kod: `kentos-extension-tools` `expressions.rs` (yeni: `Offered`, `read_functions`, `row_json`,
`values_of`), `headless.rs` (`functions`, `answer`, `publish`'in iki kaydı, `OfferedCalls`); paylaşılan ifade motoru
(`ExternalDef::refused`, derleyicinin reddi); masaüstü `tools.rs` (işlevler aynı okuyucudan); paylaşılan `blueprint.rs`
(`runtimes`), `manifest.rs` (`runtimes`'ın belgesi), SDK rehberi.

- **Sorun:** MCP ve Python kitaplığı eklentinin İşlemler araçlarını çalıştırıyordu (§70), ifade işlevlerini
  çalıştırmıyordu: `ext.tr.ornek.vitrin.ada-parsel(ada, parsel)` çağıran etiket, stil kuralı, form hesabı ya da
  İşlemler ifadesi (alan hesaplayıcı, ifadeyle seç) orada "Bilinmeyen işlev" diye reddediliyordu. Ölçüldü: değişiklikten
  önceki Python kitaplığında `attributes.calculate` bu iletiyle düştü.
- **Karar: eklenti söyler.** İşlevin tanım dosyası yok; nerede çalıştığını manifestin zaten var olan `runtimes` alanı
  söyler: `headless` varsa MCP ve Python kitaplığı işlevi sunar ve çalıştırır (araçta tanımın `headless-python`'ı gibi,
  §70). Yoksa işlev ifadelere adıyla bildirilir ve derlenirken nedeniyle reddedilir: "ext.….ada-parsel() burada
  çalışmaz: “Vitrin” eklentisi başsız çalıştığını söylemiyor (manifestinin `runtimes` listesinde `headless` yok);
  işlevlerini yalnız KentOS masaüstü çalıştırır." Sessizce boş kalan bir etiket ya da "bilinmeyen işlev" bir ajana
  neyin eksik olduğunu söylemezdi. Sınama koşucusunun konağı (`kek test`) ve masaüstü hepsini çalıştırır.
- **Bekler, masaüstünün aksine.** Masaüstü çizerken beklemez (yanıt gelince yeniden çizer, §8.1); başsız konağın
  koruyacağı bir karesi yok: yığının satırları sürümün sürecinde tek çağrıda çalışır (`kind: expression`) ve cevap
  beklenir (en çok 60 sn). Bir satırın hatası o satırı boşaltır; yığının hatası bir kez not edilir: eklentinin
  günlüğüne `expression_failed`, MCP'de standart hataya, Python'da logging'e.
- **İki kayıt, tek kaynak.** Başsız konak işlevleri ifade motorunun konağına (etiket, stil kuralı, katman süzgeci, form
  hesabı) ve İşlemler'in kendi listesine (alan hesaplayıcı, ifadeyle seç) birlikte yazar. Masaüstü de işlevleri artık
  aynı okuyucudan alır (`read_functions`, hepsi): işlevin satırı iki yerde aynı.
- **Süreç aracınkiyle aynı:** aynı sürümün aracı ve işlevi havuzda tek süreçtir; dosyaları süreç başlamadan özetle
  denetlenir; kaldırılan ya da kapatılan sürümün süreci kapanır (anahtarlara işlevlerin planları da girer).
- **İskelet:** İşlemler aracı ya da ifade işlevi olan yeni eklentinin manifesti `["desktop", "headless"]` der;
  komut, harita aracı ve panel masaüstüne bağlı kalır. Vitrin 1.0.51'den beri `headless` diyordu: paketi değişmedi.
- **SDK:** rehberin ifade satırına "MCP ve Python'da `runtimes`: `headless` ister." (paket 47.890 → 47.937 / 48.000);
  manifest şemasında `runtimes`'ın açıklaması.
- **Gerçek koşu:** `kentos-mcp` (stdio; Vitrin kurulu yalıtılmış kayıt; Vitrin çizimi): `processing.run`
  `attributes.calculate`, değer `ext.tr.ornek.vitrin.ada-parsel(ada, parsel)` → dört parselin `etiket`'i `101/1`,
  `101/2`, `101/3`, `102/1`; bilinmeyen `ext.tr.ornek.yok.islev` parametre denetiminde reddedildi. Python kitaplığında
  (yerel modül yeniden derlendi) aynı çağrı 963 ms'de (eklentinin sürecinin açılışı dahil) aynı dört değeri yazdı.
- **Sınamalar:** extension-tools 1 (yerine geçen Python süreciyle: `headless` diyen ve demeyen eklenti, sınama
  koşucusunun konağı, Türkçe büyük harf, hatalı satır ve notu, günlük kaydı), ifade motoru 1 (reddin sözü ve yeri),
  paylaşılan crate 149, masaüstünün eklenti sınamaları 98, İşlemler'in eklenti işlevi sınaması.
- **Python kitaplığında okuma (aynı gün):** kayıt defteri çizim açılışında, İşlemler komutundan önce her zaman, öteki
  komutlardan önce en çok iki saniyede bir okunur (`refresh_now_and_then`): çizim açıkken kurulan eklentinin işlevi en
  geç iki saniyede tanınır, bir betiğin binlerce çağrısı dosyaları her seferinde okumaz. MCP her araç çağrısından önce
  okur. Sınama: ilk bakış, 1,999 sn'deki bakılmaz, 2 sn'deki bakılır.

## §81. Kurulum incelemesinde Python paketleri; tanınmış bir paketin adına bir harf uzak ad (`ADIM 22A`, PY-CLOUD-14 ve bağımlılık kapısı)

**16 Kasım.** Kod: paylaşılan `python_names.rs` (yeni: 188 tanınmış ad, `resembles_known`), `python.rs` (kilidin
denetiminde `python_name_resembles` uyarısı), `release/facts.rs` (`locked_packages`, `LockedPackage::from_the_index`),
SDK'nın doğrulama kodları; kayıt defteri `packages.rs` (yeni: `PackageLine`, `lines`, `installed`),
`Review::python_packages`; masaüstünün kurulum incelemesi.

- **Sorun.** Kurulum incelemesi eklentinin yeteneklerini ve eklenti bağımlılıklarını gösteriyordu, kuracağı Python
  paketlerini göstermiyordu: paketler kullanıcının haklarıyla eklentinin ortamında çalışır ama karar verilirken adları
  bile görünmüyordu; güncellemenin getirdiği yeni paket de söylenmiyordu (PY-CLOUD-14). Paket deposu her boş adı
  verir: `uv add reqeusts` ya da bir yapay zekâ modelinin uydurduğu bir ad başkasının kodunu kurdurur.
- **Karar: paketler yeteneklerin yanında.** İnceleme paketin kilidindeki paketleri (eklentinin kendi projesi hariç)
  gösterir: ilk kurulumda tek satır ("Kuracağı Python paketleri (2): numpy 2.5.4, shapely 2.0.7."); güncellemede kurulu
  sürümün kilidine göre yalnız değişenler satır satır (YENİ; eski → yeni DEĞİŞTİ; "aynı sürüm, başka dosyalar";
  kaynağı değişen; "artık kurmuyor") ve değişmeyenlerin sayısı. Aynı sürümün başka dosyaları ayrıca söylenir: aynı adla
  başka bir paket demektir. Paket deposu dışından gelen paket kaynağıyla yazılır. Okuma katalog incelemesinin (D12)
  kilit okuyucusuyla aynıdır (`locked_packages`).
- **Karar: tanınmış ada bir harf uzak ad uyarıdır.** Paket deposunun en çok kurulan paketleri ile eklentilerin
  kullandığı coğrafi ve veri paketlerinin adları (PEP 503 ile normalleştirilmiş 188 ad) veridir. Kilitteki bir paketin
  adı bunlardan birine bir harf (ya da yer değiştirmiş iki harf) uzaksa ve kendisi listede değilse uyarıdır:
  `kentos kek verify` (`python_name_resembles`), kurulum incelemesinin en üstü (taklit uyarısının yanında; "Uyarılar"da
  ikinci kez yazılmaz) ve katalogun paket kapısı aynı denetimden okur. Engel değil: dürüst küçük bir paket ünlü birinin
  bir harf yanında durabilir (`pyaml` ile `pyyaml`). 5 harften kısa ad karşılaştırılmaz, listede olmayan ad hiç
  yargılanmaz. Depoda hiç olmayan ad (uydurulmuş ve henüz kimsenin almadığı) ağ ister: tarayıcınındır.
- **Gerçek pencere (KentOS GIS, yalıtılmış profil, Vitrin 1.0.53 kurulu):** kilidinde shapely 2.0.7 → 2.1.0 yapılmış ve
  `reqeusts` eklenmiş deneme 1.0.54'ün (depoya girmedi) güncelleme incelemesi: en üstte uyarı; Python bölümünde
  "reqeusts 2.32.3 · YENİ", "shapely 2.0.7 → 2.1.0 · DEĞİŞTİ", "1 Python paketi değişmedi.". `kek verify` aynı klasörü
  "1 uyarı" ile doğruladı (önce uyarısız: eski ikiliyle koşmuştu).
- **Sınamalar:** paylaşılan 156 (adlar, kilidin uyarısı), kayıt defteri 32 (ilk kurulum listesi; güncelleme farkı:
  başka sürüm, aynı sürüm başka dosyalar, yeni ve benzer adlı, değişmeyen, kalkan), masaüstünün eklenti sınamaları 99.
- **Kalan:** platform farkı (yeni bir işletim sistemi ya da işlemci tekerleği); listenin dönemsel güncellenmesi;
  paket deposunda olmayan adın tarayıcıda denetimi.

## §82. Mahalin adından kullanım sınıfı; bölümsüz mahal kendi sınıfıyla (`ADIM 20`, §79'un kalanı)

**16 Kasım.** Kod: paylaşılan olgular (`RoomFacts::occupancy`), İşlemler'in olgu üreticisi (`row_of`, mahalin adı ve
kullanımı `kentos_domain::fire_use::use_of` ile), `python/kentos/ext.py` (`RoomFacts.occupancy`), olgu fikstürü, JSON
Schema'lar, `kentos_ext.pyi`; `tr.kentos.mevzuat` 0.4.1 (`kacis.py`, sekizinci örnek).

- **Sorun.** Mimari denetimin yangın kuralı bir mahalin sınıfını önce bölümünden, bölümü yoksa kendi adından (Otopark,
  Depo, Lokanta …), o da yoksa binadan okuyor; olgular yalnız bölümün sınıfını taşıyordu. Kural bölümsüz mahali
  binanın sınıfıyla ölçüyor, yalnız konut olan binada hiç ölçmüyordu: konut binasının bodrumundaki otoparkın 27 m'lik
  tek yönlü yürüyüşü (Ek-5/B'de otoparklar ve depolar için 15 m) bulgusuz geçiyordu.
- **Karar: aynı okuma, mahal için de.** Olgu mahalin adının ve kullanımının söylediği Ek-5/B satırıdır (konutunki
  yazılmaz: konutun mahalleri 48'in kurallarınındır); yerel denetimin okumasıyla aynı işlevden. Kural sırası yerel
  denetiminki: bölümün sınıfı, bölüm yoksa mahalin adının sınıfı, o da yoksa binanın bilinen en ağır sınıfı; bulgu
  sınıfın mahalin adından okunduğunu söyler. Bölümdeki mahalin adı bölümün sınıfını değiştirmez (büro bölümündeki
  Depo büro sayılır), yerel denetim gibi. Olguların sürümü değişmedi (isteğe bağlı alan).
- **Bütün çizimde:** fikstürde üç mahal sınıf aldı (dükkân, otopark, büro), üçü de bir bölümde; kuralın bulguları
  değişmedi (iş çalıştırıcısının sınamaları 25/25).
- **SDK paketi:** yeni alan ve belge cümlesi pakete 40 bayt ekledi: 47.977 / 48.000 (sınır yükseltilmedi; bir sonraki
  alan için önce bir yerden kısaltmak gerekecek).
- **Sınamalar:** `kek test` mevzuatın 28 örneğinin hepsi (yeni örnek: konut binasının bölümsüz bodrum otoparkı, tek
  yönde 27 m, hata), Python 43, paylaşılan crate 156, İşlemler'in olgu sınamaları 13.
- **Kalan:** yüksek tehlikeli yer (adlardan okunmuyor; BYKHY'nin tanımı tehlikeli maddenin türü ve miktarına bakar,
  çizimde yok); bölüm sınıfının programda açıkça seçilmesi.

## §83. Ajanın kilidine soğuma süresi (`exclude-newer`), iskelette varsayılan (`ADIM 18A`, D18)

**16 Kasım.** Kod: paylaşılan `blueprint.rs` (`pyproject.toml`'da `[tool.uv] exclude-newer = "7 days"`, kilitte
`[options]`), `python.rs` (`python_lock_no_cooling` uyarısı), SDK'nın doğrulama kodları.

- **Sorun.** Paket deposu yeni bir sürümü ya da yeni bir adı anında yayımlar; kötü niyetli sürümler çoğu zaman
  günler içinde yakalanıp kaldırılır. Bir yapay zekâ ajanının uydurduğu paket adı (ya da tanınmış bir adın bir harf
  yanındaki taklidi, §81) birinin onu hemen kaydetmesiyle bir sonraki `uv add`'de kilide girerdi.
- **Karar: bir hafta.** İskeletin `pyproject.toml`'u `[tool.uv] exclude-newer = "7 days"` der: uv bir haftadan yeni
  sürümü çözüme almaz, kilit bunu `exclude-newer-span = "P7D"` ile kaydeder (uv 0.11.26 göreli süreyi kabul ediyor;
  ölçüldü). Yeni bir eklentinin `uv add`'i böylece kendiliğinden soğutulur; bir güvenlik düzeltmesi beklemezse yazar
  tek paketi `exclude-newer-package` ile açar.
- **Uyarı yalnız ajanın kilidine.** KentOS'un yapay zekâ damgasını (`generated`, §76) taşıyan pakette üçüncü taraf
  paketi olup soğuma süresi (ya da sabit bir tarih) olmayan kilit uyarıdır (`python_lock_no_cooling`); insanın
  paketinde karar yazarınındır, engel değil (D11). Depodaki fikstürler ve Vitrin değişmedi: sabit özetleri ve kilitleri
  korunur.
- **Ölçüm:** iskeletin yeni kilidi uv'nin `uv lock` ile yazdığıyla bayt bayt aynı, `uv lock --check --offline` temiz,
  `kek verify` 0 uyarı. Vitrin'in kilidi `exclude-newer` eklenince çevrimdışı yeniden kilitlenebildi (yalnız `[options]`
  bloğu eklendi); KentOS'un TOML okuyucusu satır sonu yorumunu okudu.
- **Sınamalar:** paylaşılan 157 (ajanın soğumasız kilidi uyarı; soğumalı ve sabit tarihli kilit ile insanın kilidi
  uyarısız), headless ajan araçları 20 (iskelet doğrulanır), extension-tools 11, masaüstünün eklenti sınamaları 99.
- **Gerileme ve düzeltme (aynı gün):** soğuma ayarlı iskeletten yeni bir eklentinin ortamı hazırlanamadı ("Eklentinin
  ortamı hazırlanamadı. uv: “”"): KentOS uv'yi kullanıcının ayarlarından yalıtmak için `--no-config` ile çalıştırıyor,
  bu da `pyproject.toml`'un `[tool.uv] exclude-newer`'ını yok sayıyor; kilit soğuma süresini taşıdığından uv onu eskimiş
  sayıp `--locked`'ta reddediyordu ("needs to be updated, but --locked was provided"). Yukarıdaki "`uv lock --check`
  temiz" ölçümü `--no-config`'suzdu ve bunu göstermedi. Artık ortam hazırlama (önizleme, kurma) ve sunucunun tekerlek
  deposu kilidin `[options]`'ındaki süreyi uv'ye bayrakla geri verir (`cooling_args`: `--exclude-newer P7D`, sabit tarih,
  `--exclude-newer-package`). Ölçüldü: iskelet 4 örnek ve 2 sınamayla geçti; numpy ve shapely'li soğumalı kilit
  ortamını 3,6 saniyede hazırladı, 9 örneği geçti. Sınama: `python-env` (bayrakların okunması).
- **Kalan:** kurum ilkesinde süre (D14, §85'te); katalogun bağımlılık kapısının bunu kendisi denetlemesi.

## §84. Kurum ilkesinde Python paketleri: engelli, izinli liste ve izinli paket depoları (`ADIM 17`, D14)

**16 Kasım.** Kod: kayıt defteri `policy.rs` (`blockedPythonPackages`, `allowedPythonPackages`, `allowedPackageIndexes`,
`Policy::package_refusal`), `registry.rs` (kurulu sürümün kilit paketleri bildirgesiyle bir kez okunur, `locked`;
inceleme, kurulum ve çalıştırma aynı kurala bakar); paylaşılan `python_names::normalized` (PEP 503).

- **Sorun.** Kurum ilkesi kaynağı, yayımcıyı, eklentiyi ve yeteneği sınırlıyordu; eklentinin kendi ortamına kuracağı
  Python paketlerine hiçbir kural yoktu. `kek verify` PyPI dışı bir depodan gelen paket için "kurum ilkesi bu depoya
  izin vermeli" diyordu ama ilkede böyle bir alan yoktu. Belediyenin bilgi işlemi kendi paket aynasını ya da bilinen
  sorunlu bir paketi yasaklayamıyordu.
- **Karar: üç alan, eklentininkilerle aynı anlam.** `blockedPythonPackages` adlarıyla (PEP 503 ile normalleştirilerek
  karşılaştırılır: `NumPy` = `numpy`); `allowedPythonPackages` yazılırsa yalnız onlar; `allowedPackageIndexes` yazılırsa
  paketler yalnız o depolardan (sondaki `/` önemsiz), depo dışı kaynak (git, adres) da reddedilir. Örnek:
  `{"format": "kentos.extension-policy", "schemaVersion": 1,`
  `"allowedPackageIndexes": ["https://pypi.ornek.bel.tr/simple"]}`.
  Okunamayan ilke paketleri de reddeder (kapalı kalır).
- **Üç yerde aynı kural:** inceleme Kur'dan önce söyler ve Kur'u kapatır (öbür ilke retleri gibi); kurulum reddeder
  (`registry_policy`); ilke sonradan değişirse kurulu eklenti nedeniyle durur, silinmez. Kurulu sürümün paketleri
  bildirgesiyle birlikte bir kez okunur: liste her çizildiğinde kilit dosyasını okumak gereksizdi; güncellemenin paket
  farkı da (§81) artık bunu kullanır.
- **Sınamalar:** kayıt defteri 33 (yeni: engelli paket, izinli liste, izinli depo; incelemenin sözü ve Kur'un reddi
  aynı cümle; sondaki eğik çizgiyle izinli depo; sonradan engellenen paketle kurulu eklentinin durması).
- **Gerçek pencere (KentOS GIS, yalıtılmış profil):** `blockedPythonPackages: ["reqeusts"]` ilkesiyle deneme Vitrin
  1.0.54'ün güncelleme incelemesi "Kurum ilkesi “reqeusts” Python paketini engelliyor." dedi, Güncelle kapalı kaldı,
  kurulu 1.0.53 çalışmayı sürdürdü. İlk koşu eski ikiliyle yapıldı ve bir şey gösterdi: daha yeni bir KentOS için
  yazılmış ilke (tanınmayan alan) her şeyi doğru biçimde kapalı tuttu ama nedeni okuyucunun İngilizcesiyle yazdı
  ("unknown field … expected one of …"). Artık Türkçe: "bu KentOS'un tanımadığı “…” alanı var; ilke daha yeni bir
  KentOS sürümü için yazılmış olabilir"; bozuk JSON satır ve sütunuyla. Kurumda ilke dosyası KentOS'tan önce
  güncellenebileceği için bu gerçek bir durum.
- **Sürüm düzeyinde engel (aynı gün):** `blockedPythonPackages` bir sürümü de adlandırabilir (`"ultralytics==8.3.41"`;
  2024'te bu paketin iki sürümü ele geçirilip kripto madencisi dağıtmıştı): yalnız o sürüm reddedilir ("… paketinin
  8.3.41 sürümünü engelliyor; eklentinin yazarı başka bir sürüme kilitlemeli."), öbürleri geçer; `==` çevresindeki
  boşluk önemsiz. Sınama: `numpy==2.5.3` kilitteki 2.5.4'ü geçirir, `shapely == 2.2.0` kilitteki 2.2.0'ı engeller.
  İlkede soğuma süresi aynı gün §85'te.

## §85. Kurum ilkesinde en düşük paket yaşı (`minimumPackageAgeDays`; `ADIM 17`, D14 ve D18)

**16 Kasım.** Kod: paylaşılan `release/facts.rs` (`LockedPackage::uploaded`: paketin en yeni dosyasının
`upload-time`'ı; `seconds_of`), kayıt defteri `policy.rs` (`minimumPackageAgeDays`, `package_refusal`'ın kurma anı).

- **Karar: yaş kilitten ölçülür, ayardan değil.** §83'ün soğuma süresi kilidin ayarıdır ve yazara bağlıdır; kurum
  bunu doğrudan isteyebilmeli. `uv.lock` her dağıtım dosyasının depoya konduğu anı yazar: paketin yaşı en yeni
  dosyasınınkidir. İlke `minimumPackageAgeDays: N` derse inceleme ve kurulum o anın saatiyle N günden yeni paketi
  reddeder ("… “numpy” 2.5.4 3 gün önce yayımlandı. Eklentinin yazarı daha eski bir sürüme kilitleyebilir ya da
  beklenir."); zamanı kilitte yazmayan paket (bir depodan değil, git'ten ya da adresten gelen) denetlenemez ve
  reddedilir (kapalı kalır). Kurulu eklenti paketlerinin yaşı yüzünden durmaz: yaş kurma anının sorusudur.
- **Anlar metin olarak değil an olarak karşılaştırılır:** kilit saniyenin kesirlerini değişen sayıda haneyle yazıyor
  (`43.45Z`, `43.4Z`), metin sırası yanlış olurdu.
- **Sınamalar:** paylaşılan akış 28 (RFC 3339 okuması, kesirli üç dosyadan en yenisi), kayıt defteri 33 (100 000 gün
  incelemede ve kurulumda reddedilir, 0 gün geçer).

## §86. Katalog girdisinin karnesi: dürüst sinyaller, yıldız yok (`ADIM 22A`, D17)

**16 Kasım.** Kod: paylaşılan `catalog.rs` (`CatalogEntry::python_packages`, `examples`, `card_of`; girdi tanımadığı
alanı reddetmeden okunur), masaüstü `view.rs` (`report_card`).

- **Sorun.** Katalogdaki bir eklentiyi seçerken kişi yayımcıyı, lisansı ve yetenekleri görüyordu; çalışıp
  çalışmayacağını, ortamına ne kuracağını, internete çıkıp çıkmadığını, sınanabilir olup olmadığını ancak incelemeyi
  açınca ya da hiç göremiyordu. D17: karne geçen kapıları, yetenekleri, bakım ve uyumluluğu söyler; yıldız oyu yok.
- **Karar: ayrıntının "Karne" bölümü.** Bu KentOS'ta çalışır mı (çalışmıyorsa neden), kurulu bir eklentiyi taklit
  ediyor mu ya da ona benziyor mu (ad ve kimlik; ADR 8059 §14, inceleme açılmadan, göz atarken), kuracağı Python
  paketleri (`numpy 2.5.4, shapely 2.0.7`), internete bağlanır mı ve nereye, verisini dış bir yapay zekâya gönderir mi,
  kaç örnek verir (Örnekleri çalıştır ve `kek test` koşturur). Yazanı (yapay zekâ ise, §76) bölümün üstünde zaten yazılı.
- **Dizin:** `kek index` paketin kilidinden paket listesini, araç tanımlarından, kural örnek dosyalarından ve ifade
  işlevlerinden örnek sayısını yazar; ikisi de boşsa hiç yazılmaz.
- **İleri uyum:** katalog girdisi tanımadığı alanı artık reddetmez: daha yeni bir `kek index`'in yazdığı dizin eski
  KentOS'ta da listelenir (her paket kurulmadan önce baytlarından yine denetlenir). Önce reddediyordu: karneye eklenen
  her alan eski masaüstlerine kataloğu bütünüyle kapatırdı.
- **Gerçek pencere (KentOS GIS, yalıtılmış profil, Vitrin kurulu):** taklit kopyanın (`tr.ornek.vltrin`) katalog
  ayrıntısı: "Bu KentOS'ta: Çalışır", taklit uyarısı, "Python: 2 paket: numpy 2.5.4, shapely 2.0.7", "İnternet:
  Bağlanır: tiles.ornek.bel.tr", "Örnekler: 9 örnek".
- **Sınamalar:** paylaşılan katalog sınamaları 4 (paketler, örneksiz ve Python'suz pakette alanların yazılmaması,
  tanınmayan alanla okuma).
- **Kalan:** sunucu kataloğunun karnesi: geçen kapılar ve inceleme derinliği (ADR 8059; masaüstüne bağlandığında).

## §87. Paketin kendi sınamaları: `kek test` `tests/`'i `unittest` ile eklentinin ortamında (`ADIM 21`, §74'ün kalanı)

**16 Kasım.** Kod: komut satırı `kek_test/own.rs` (yeni: `run`, `read_result`), `kek_test.rs` (rapor ve çıkış kodu),
`main.rs` (yardım); Vitrin 1.0.54 (`tests/test_mantik.py`).

- **Sorun.** `kek test` örnekleri (§48, §75, §78) koşturuyordu; yazarın kendi sınamaları (yardımcı işlevler, hesap
  ayrıntıları) için bir yer yoktu. Sürekli tümleştirmede ikinci bir komut, ikinci bir ortam gerekirdi.
- **Karar: `tests/test_*.py`, Python'un `unittest`'i, eklentinin ortamı.** Örneklerden sonra, klasörde `tests/test_*.py`
  varsa `python -B -m unittest discover -s tests -v` eklentinin hazırlanmış ortamında, yazarın klasöründe çalışır; kod
  `src`'den (`PYTHONPATH`), KentOS'un kitaplığı ortamdan gelir. pytest değil: kilidin getirmesi gereken bir paket
  olurdu, atölyedeki ajan uv çalıştıramaz; KentOS'un kendi Python sınamaları da `unittest`'le. Başlangıç klasörü kök
  sayılır: `tests/__init__.py` gerekmez (ilk denemede `-t .` onu istedi: "Start directory is not importable").
- **Rapor:** "Sınamalar (tests/, unittest): 5 sınamanın hepsi geçti."; kalanda unittest'in çıktısı (yazarın dosyasının
  satırıyla) ve "3 sınamadan 1 tanesi kaldı (1 başarısız, 0 hata)"; pytest biçimli düz işlevler ("NO TESTS RAN") ne
  yapılacağıyla söylenir; 300 saniyede bitmeyen durdurulur. `--json`'da `tests` alanı (`ran`, `failures`, `errors`,
  çıktının sonu). Kalan sınama çıkışı 1 yapar; sonuç cümlesi örneklerinkiyle birlikte.
- **Gerçek koşu:** mevzuatın kopyasında bilerek kalan bir sınama: çıkış 1, iz kaydı yazarın `tests/test_kacis.py`'sinin
  18. satırı; düzeltince çıkış 0. Vitrin'de ilk koşu göreli klasörle (`kek test samples/…`) "No module named 'vitrin'"
  verdi: alt süreç klasörde başladığı için göreli `PYTHONPATH` başka yeri gösteriyordu; klasör mutlak yapıldı. Vitrin
  1.0.54: 9 örnek ve 5 sınama geçti; deneme profilinde 1.0.53 → 1.0.54 gerçek pencerede güncellendi.
- **Tek koşucu, üç yer (aynı gün):** koşucu `kentos_extension_tools::examples::package_tests`'e taşındı; `Examples`
  isteğe bağlı `tests` taşır ve örneklerden sonra onları koşturur (rapora `tests`, özete "Sınamalar: …"). Komut satırı
  yazarın klasöründe, masaüstünün Örnekleri çalıştır'ı ve ajanın denemesi (`extension.workshop.trial` `examples` ile)
  kurulu paketin klasöründe, sürümün ortamının Python'uyla (`environment_python`) koşturur; kalan sınama eklentinin
  günlüğüne unittest'in çıktısıyla düşer (`tests_failed`). Gerçek pencere: Vitrin 1.0.54'ün Örnekleri çalıştır'ı "9
  örneğin hepsi geçti. Sınamalar: 5 sınamanın hepsi geçti." dedi (bildirim ve günlük).
- **Sınamalar:** extension-tools 12 (unittest'in sözleri: geçen, kalan, hata, hiç sınama yok, süre aşımı), komut
  satırı, masaüstünün eklenti sınamaları 99.
- **İskelet öğretir:** yeni bir eklentinin klasöründe `tests/test_logic.py` (kod yüklenir; ifade işlevi istendiyse
  `buyuk_harf`'ın cevapları): SDK paketi 48 000 bayt sınırına 23 bayt uzakken rehbere cümle eklemek yerine ajan
  yapıyı örnekten görür. Taze iskelet `kek test`'te "4 örneğin hepsi geçti. Sınamalar: 2 sınamanın hepsi geçti."
- **Kalan:** sürekli tümleştirme şablonu (komut satırının indirilebilir dağıtımı yok).

## §88. Listeyi karnenin sinyalleriyle süzmek (`ADIM 22A`, PY-CLOUD-12)

**16 Kasım.** Kod: masaüstü `browse.rs` (`Signal`, `SIGNALS`, `passes`), `view.rs` (süzgeç kutusu), `mod.rs`
(`SignalFilter`).

- **Karar:** Eklentiler'in listeleri (kurulu, bulunabilir, güncellenebilir, yeni) program ve sözcükten başka karnenin
  sinyalleriyle de süzülür: bu KentOS'ta çalışanlar, internete bağlanmayanlar, Python paketi kurmayanlar, yapay
  zekânın ve insanın yazdıkları (§76, §86). Kurulu eklentide bildirgeden, katalogdakinde girdiden okunur.
- **Yerleşim (sahibinin sözüyle düzeltildi):** süzgeç önce ikinci bir satıra kondu; sahibi "alt satıra kayması çirkin
  oldu, simetri bozuldu" dedi. Tek satırda üç kutu (arama, program, süzgeç) 170 pikselken sekmeler sıkıştı ("Kurulu
  (1)" iki satıra kırıldı, "Güncellenebilir" kısaldı); 150'de iletişim kutusunun 1 025 pikselinden sekmelere ~510 kalır
  (doğal genişlikleri ~450): üçü eşit genişlikte ve hizalı, kutunun yüksekliği eskisi gibi (kayıtlı senaryoların
  koordinatları değişmedi). Kapalı kutu kısa adı gösterir ("İnternetsiz", "Yapay zekâ yazdı"), liste kutudan geniş
  açılır ve tam adlarla; arama kutusunun yazısı "Ara: ad, açıklama…".
- **Sınama:** masaüstü 1 (iki girdili katalog: yapay zekânın ve insanın paketi, bu KentOS'ta çalışmayan, internetsiz
  yok, paketsiz ikisi); eklenti sınamaları 100.

## §89. Yüksek tehlikeli yerler: adından okunan sınıf, çıkmaz koridorun 10 m'si, 25 kişide iki çıkış (`ADIM 20`, ulusal kuralların beşinci dilimi)

**16 Kasım.** Kod: `crates/native/domain/src/fire_use.rs` (`high_hazard`; `use_of` önce ona bakar),
`crates/native/application/src/arch/fire_audit.rs` (`two_exits_above`), `catalog/standards/tr-mevzuat` 0.5.0
(`cikmaz.py`: `_sinif`, `CIKMAZ_YUKSEK`, `MADDE_YUKSEK`; `kacis.py`: `agir` paylaşıldı), iki yeni örnek, sınamalar;
kaynaklar `docs/research/2026-11-16-tr-mevzuat-denetim-kurallari.md` › *Yüksek tehlikeli yerler*.

- **Durum.** Ek-5/B'nin "yüksek tehlikeli yerler" satırı yerel denetimde (`Use::HighHazard`) ve olgularda
  (`Occupancy::HighHazard`) vardı, ama hiçbir ad onu okumuyordu: `use_of` onu hiç döndürmüyordu. Tüp bayiinin satış
  alanı mağaza diye ölçülüyor, çıkmaz koridoru 15 m'ye bakıyordu.
- **Kaynak (aynen, notta).** BYKHY 8/1-ğ (kullanım sınıfı), 17 (tanım: parlayıcı ve patlayıcı maddeler ile
  akaryakıtın imali, depolanması, doldurma-boşaltma ve satışı; LPG ve doğalgaz; mermi, barut, dinamit; yanıcı sıvılar),
  39/2 (25 kişiyi aşan yüksek tehlikeli mekânda en az 2 çıkış). 19/2-c'nin "yüksek tehlikeli yerler"i söndürme
  tasarımının tehlike sınıfıdır (Ek-1/C), satır değil.
- **Karar: yalnız açık sözler.** LPG; tüp ile depo, bayi, dolum, satış ya da "tüpçü"; akaryakıt ile istasyon, dolum,
  satış ya da depolama; doğalgaz ("doğal gaz", CNG) ile depo, dolum, istasyon, satış; parlayıcı, patlayıcı, barut,
  dinamit; mermi ile depo, imal, satış; yanıcı sıvı. Öbür satırlardan önce okunur (LPG tüp deposu depolar satırı
  değildir). **Tanınmayan:** "Yakıt deposu" ve "Akaryakıt deposu" (56'nın deposu da böyle adlandırılıyor, 56/2;
  eskisi gibi depolar satırı), "Doğalgaz sayaç odası", tek başına "Tüp". Belirsiz ad sınıf söylemez: yanlış bir yüksek
  tehlike konut binasının kazan dairesi deposunu 10 m'lik sınıra sokardı.
- **Sonuçlar.** Yerel denetim aynı okumayla: kaçış uzaklığı satırın 10/20 ve 20/35 m'si, çıkmaz koridor 10/20 m, iki
  çıkış 25 kişiyi aşınca (mahalde ve katta; önce her yerde 50). Olgular `occupancy: highHazard` taşır; yeni alan yok,
  sürüm 1. `kacis-uzakligi` satırı zaten okuyordu. `cikmaz-koridor` koridorun sınıfını kaçış uzaklığındaki gibi okur
  (bölümünün; bölümü yoksa kendi adının, o da yoksa binanın en ağır sınıfı) ve yüksek tehlikede yağmurlamasız 10 m'ye
  bakar; bulgu satırı, sınıfın nereden okunduğunu ve 17'yi söyler ("… en çok 10.00 m, yağmurlama sistemiyle 20.00 m
  (yüksek tehlikeli yerler satırı, binanın en ağır sınıfı) … — BYKHY md. 4/i, 17 ve Ek-5/B"). Karma binada ortak
  mahal en ağır sınıfla okunur (18, 31/5); yangın bölmesi olgularda yok, bölmeyle ayrılmış bölümün ayrı korunması
  okunamaz.
- **Ölçüm.** 15 × 10 m tek katlı satış alanı (eksen dış hattı 150 m², kişi başı 5 m², 30 kişi, tek kapı): bölüm
  "Tüp bayii" iken "Satış alanı kişi yükü 30: en az 2 çıkış gerekir, 1 var — BYKHY 39/2", katta da aynısı ve en uzak
  noktadan "> 10.00 m (yüksek tehlikeli yer, tek yön)"; aynı oda "Dükkân" iken üçü de yok.
- **Sınamalar.** `kentos-domain` `fire_use` (13 ad tanınır, 7'si tanınmaz), masaüstü `fire_hazard_tests` (yukarıdaki
  ölçüm), bulut işi `mevzuat_tests::cikmaz` (+1; 13'ün hepsi geçti), `kek test` 30 örneğin hepsi (iki yeni: tüp
  bayiinin 12 m'lik çıkmaz koridoru yağmurlamasız hata, yağmurlamayla bulgusuz), `kek verify` 0 uyarı.
- **Kalan.** 39/2 mevzuat paketinde kural değil (olgularda kişi yükü yok; yerel denetim söylüyor); kurp/görüş
  hesapları; belediye plan notları.

## §90. Güncelleme incelemesi çalıştığı yerlerin farkını söyler (`ADIM 22A`, PY-CLOUD-14)

**16 Kasım.** Kod: kayıt defteri `view.rs` (`platform_lines`, `Review::platforms`), `registry.rs` (`review_beside`);
masaüstünün kurulum incelemesi (`Çalıştığı yerler` bölümü).

- **Sorun.** Plan: "güncelleme yeni bağımlılık/yetenek/platform farkını gösterir". Yetenek (§3) ve Python paketi (§81)
  farkı vardı; eklentinin bildirdiği işletim sistemleri (`compatibility.operatingSystems`) ve çalışma yerleri
  (`runtimes`: masaüstü, başsız, bulut) güncellemede sessizce değişebiliyordu: başsız çalışmayı bırakan bir sürüm
  MCP ajanlarının ve Python kitaplığının kullandığı araçlarını götürür, kullanıcı bunu kurduktan sonra öğrenirdi.
- **Karar.** Güncellemenin incelemesi kurulu sürümle farkı cümleyle söyler, önce bıraktıklarını: "Artık şu sistemlerde
  çalışmıyor: Windows.", "Artık MCP ajanlarında ve KentOS'un Python kitaplığında çalışmıyor.", sonra eklediklerini
  ("Artık bulutta (sunucunun işlerinde) da çalışıyor."). Hiç sistem yazılmamışsa her sistemdir; ilk kurulum ve aynı
  yerler bir şey söylemez. Bu bilgisayarın sistemini bırakan sürüm zaten kurulamaz (uyumluluk: `incompatible`); cümle
  öbür sistemleri kullananlar için de doğruyu söyler (kurum aynı paketi Windows'ta da dağıtıyorsa).
- **Sınamalar:** kayıt defteri `view::tests` 1 (bırakma ve ekleme, ilk kurulum, aynı yerler), kayıt defterinin hepsi
  (36), masaüstünün eklenti sınamaları (102).
- **Söz (aynı gün):** cümle önce "başsız (MCP ajanları ve KentOS'un Python kitaplığı)" diyordu; "başsız" arayüzde
  başka hiçbir yerde geçmeyen bir geliştirici sözcüğü: yerine kullanıcının bildiği yerler yazıldı ("MCP ajanlarında ve
  KentOS'un Python kitaplığında").

## §91. Paketin SBOM'u, Python bağımlılıklarının listesi: `kentos kek sbom` (`ADIM 18A`, PY-UV-29)

**16 Kasım.** Kod: paylaşılan `sbom.rs` (yeni: `cyclonedx`, `without_credentials`, `utc_text`); komut satırı
`kentos kek sbom <klasör|paket.kek> [-o <dosya>]`.

- **Sorun.** Plan: "Sır içermeyen kilit/ortam/SBOM raporu dışa aktarılır". Kurumun bilgi işlemi kurduğu her eklenti için
  hangi Python paketlerinin, hangi sürümle, nereden ve hangi dosyalarla geldiğini standart bir biçimde saklamak ister
  (tedarik zinciri denetimi, açık bildirimleriyle karşılaştırma); bunun bir yolu yoktu.
- **Karar: kilitten CycloneDX 1.6 JSON.** Ortam kilitten kurulur ve her dosya kilitteki özetine karşı denetlenir (§5),
  yani kilit ortamın listesidir; Python çalıştırılmaz, katalog da aynı listeyi üretebilir. Her paket bir bileşen: purl
  (`pkg:pypi/numpy@2.5.4`; başka dizinde `?repository_url=`, depodan `?vcs_url=`), dağıtım dosyalarının adresi ve
  SHA-256'sı (`externalReferences`), kaynağı (`kentos:source`); kilidin bağımlılık ağacı `dependencies`'te; eklentinin
  kendisi `metadata.component` (kimlik, sürüm, yayımcı, lisans ifadesi). Projenin kendisi bileşen değildir.
- **Sır yok:** adreslerdeki kullanıcı adı ve parola (`https://kullanici:parola@ayna/simple`) her yerden atılır.
- **Aynı paket, aynı rapor:** seri numarası (`urn:uuid:`, RFC 4122 sürüm 5 bitleri) paketin içeriğinden; yalnız zaman
  damgası değişir.
- **Python'suz paket** listelenecek bir şey olmadığını söyler, dosya yazılmaz (çıkış 1).
- **Ölçüm:** Vitrin 1.0.54: numpy 2.5.4 (11 dosya), shapely 2.0.7 (7 dosya; numpy'a bağlı); mevzuat 0.5.0: bağımlılık
  yok (boş liste).
- **Sınamalar:** paylaşılan 3 (bileşenler, purl'ler, dosya özetleri, ağaç, parolasız adres, aynı seri numarası;
  parolanın her biçimde atılması; zaman), komut satırı 1 (Vitrin klasöründen iki paket; Python'suz pakette dosya yok).
- **Katalogda (aynı gün):** `GET …/releases/{eklenti}/{sürüm}/sbom` onaylı sürümün baytlarından aynı listeyi verir
  (`application/vnd.cyclonedx+json`, `{kimlik}-{sürüm}.cdx.json`); sürümü kurabilen okur; Python'suz pakette
  `sbom_empty` (422) nedeniyle. Katalog API sınaması 1 (yayımlanmış sürümde numpy ve shapely; Python'suzda 422).
- **Masaüstünde (aynı gün):** Eklentiler › kurulu eklentinin ayrıntısı › Python ortamı: "SBOM'u kaydet…"
  (`bom.rs`): kurulu paketin kilidinden aynı liste, kaydetme penceresinde önerilen ad `{kimlik}-{sürüm}.cdx.json`;
  ortam hazırlanmamış olsa da (kilit kurulacak olanı söyler). Düğme eylem satırına değil Python bölümüne kondu: eylem
  satırı zaten dar ("Kaldır" ikinci satıra iniyordu; bu değişiklikten önce de). Sınama: masaüstü 1 (Vitrin kurulunca
  numpy ve shapely; kurulu olmayan eklentide neden). Gerçek pencerede düğme ve açıklaması görüldü; kaydetme penceresi
  oturumda açılmaz (oturum D-Bus'sız).
- **Lisanslar (aynı gün):** kilit lisans söylemez; masaüstü eklentinin ortamı hazırsa ortamdaki her dağıtımın
  `*.dist-info/METADATA`'sından okur (`sbom::license_of`, `add_licenses`; masaüstü `bom::licences_in`): önce
  bildirilen SPDX ifadesi (`License-Expression`, PEP 639: numpy "BSD-3-Clause AND 0BSD AND MIT AND Zlib AND
  CC0-1.0"), yoksa kısa `License` alanı (shapely "BSD 3-Clause"), yoksa OSI sınıflandırıcısının adı; lisansın bütün
  metni ve `UNKNOWN` söylenmez. Ortam hazır değilse liste lisanssız çıkar. Sınamalar: paylaşılan 1 (üç kaynağın sırası,
  bütün metin, UNKNOWN, bileşenlere işlenmesi), masaüstü 1 (sahte ortamın iki dağıtımı).
- **Ad (sahibinin düzeltmesi, aynı gün):** düğme önce "Malzeme listesini kaydet…" diyordu; sahibi: "çok saçma bir
  ifade olmuş, terminolojiye uygun değil". KentOS'ta Malzeme Listesi (MLZ) bir mimari pafta türü ve yolun kesit
  malzemesi; "bill of materials"ın düz çevirisi burada yapı malzemesi gibi okunuyordu. Yazılımdaki yerleşik terim
  kullanıldı: "SBOM'u kaydet…", açıklaması "Python bağımlılıklarının listesi: sürümler, kaynaklar ve dosya özetleri
  (CycloneDX)."; iletiler "SBOM kaydedildi", "SBOM'da listelenecek bağımlılık yok".
- **Kalan:** komut satırında ve katalogda lisanslar (ikisinde de hazırlanmış ortam yok).

## §92. İmar durumu olgularda; emsal ve TAKS kuralı (`ADIM 20`, ulusal kuralların altıncı dilimi, plan notlarının temeli)

**16 Kasım.** Kod: paylaşılan `audit_facts/architecture/zoning.rs` (`ZoningFacts`), `ArchitectureFacts.zoning`; üretici
`crates/native/processing/src/audit_facts/architecture.rs` (`Document::zoning_report`'tan); Python `ext.ZoningFacts`,
`ArchitectureFacts.zoning`; şemalar, stub, `llms.txt`; `tr.kentos.mevzuat` 0.6.0 (`src/tr_mevzuat/imar.py`, 7 örnek).
Kaynaklar `docs/research/2026-11-16-tr-mevzuat-denetim-kurallari.md` › *İmar durumu*.

- **Durum.** KentOS Mimari emsali ve TAKS'ı durum çubuğundaki rozette ve Emsal ve TAKS tablosunda hesaplıyordu (ADR
  8030), ama denetimde bulgusu yoktu, bulut işi göremiyordu ve olgularda imar durumu olmadığı için bir belediyenin plan
  notu kuralı yazılamıyordu.
- **Olgular.** `zoning` yalnız projenin İmar durumu varken: net parsel, planın emsali ve TAKS'ı (verilmeyen yok), katlar
  alanı (%30'u aşan emsal harici eklenmiş), emsale dahil, emsal harici (%30 içinde), %30'a sayılmayan, %30 sınırı ve
  aşan, taban alanı. Mimari'nin tablosuyla aynı işlevden ve aynı sayılar (sınama); İmar durumu olmayan projenin
  olguları bayt bayt eskisi gibi (alan yazılmaz). Sürüm 1'in isteğe bağlı alanı.
- **Kural `imar-durumu`** (PAİY 4/1-tt, 4/1-cccc, 5/5): emsal = katlar alanı / net parsel, TAKS = taban alanı / net
  parsel; planın verdiği değeri aşan hata ("Emsal 1.62 > 1.50: katlar alanı 486.00 m², imar durumuna göre en çok 450.00
  m² (net parsel 300.00 m²) — PAİY md. 4/1-tt ve 5/5; imar durumu"). TAKS verilmemişse 5/6 (RG 1.7.2026/33297): %60'ı
  aşan hata; %40 ile %60 arası uyarı, çünkü ayrık ya da blok nizamda plan belirlemediyse %40'tır ve yapı nizamı olgularda
  yok. Emsal harici alanların %30'u aşan kısmı katlar alanına eklendi diye bilgi (5/8). Bulgu nesneye değil binaya
  aittir (`feature_id` yok).
- **SDK paketi sınırda kaldı.** Stub'a yeni sınıf girince paket 48 830 bayta çıktı (sınır 48 000, yükseltilmez). Olgu
  sınıflarının (`ArchitectureFacts`, `RoomFacts`, `UnitFacts`, `DoorEscape`, `EscapeWalk`, `RoomEscape`, yeni
  `ZoningFacts`) belge dizgileri birer cümleye indi: alanların anlamı zaten ayrı `factsSchema` parçasında alan alan
  yazılı. Paket 47 565 bayt.
- **Belediye plan notları için temel.** Bir belediyenin kendi eklentisi aynı olgularla kendi plan notunu yazabilir
  (ör. planın emsale eklediği bir artış, belirli bir kullanımda daha düşük TAKS). Planın öbür değerleri (kat adedi,
  Hmax, yapı nizamı, çekme mesafeleri) KentOS'un İmar durumunda henüz yok: onlar girince 21/4 (emsal verilmeyen parselde
  TAKS × kat adedi) ve 5/6'nın %40'ı da karar verilebilir olur.
- **Sınamalar:** paylaşılan 1 (yuvarlama, İmar durumu olmayanda alan yok), üretici 1 (Mimari'nin raporuyla aynı
  sayılar; fikstür değişmedi), `kek test` 37 örneğin hepsi (7 yeni), bulut işi `mevzuat_tests::imar` (emsal ve TAKS
  aşımı, %60 hata, %40-%60 uyarı, İmar durumu yokken sessiz) ve paketin bütün kuralları (14), SDK sınamaları (165).
- **Kalan:** İmar durumunda kat adedi, Hmax, yapı nizamı ve çekme mesafeleri; kurp/görüş hesapları.

## §93. Eklenti denetiminde kural seçici: araç eklentisinin kuralları da masaüstünde çalışır (`ADIM 20`, `ADIM 22`)

**16 Kasım.** Kod: masaüstü `shell/extensions/audit_picker.rs` (yeni: `RuleChoice`, `ext_audit_choices`,
`ext_audit_picker`, `ext_audit_pick`), `audits.rs` (`ext_audit_tab`, `seconds_words`), `runner.rs` (günlükte süre).

- **Sorun.** Eklenti denetimi sekmesi yalnız son kuralın bulgularını gösteriyordu; kural eklentinin şeritteki
  düğmesinden, bir ajandan (`audit.run`) ya da bulut işinden başlıyordu. Türkiye mevzuatı gibi bir araç eklentisi sekme
  açmaz, şeride düğme koymaz: kullanıcı ulusal kuralları önündeki çizimde çalıştıramıyordu, §92'nin İmar durumu kuralı
  masaüstünde yalnız Asistan'dan çalışıyordu.
- **Karar.** Sekmenin üstünde "Kural [seçim] Denetle": bu programın denetimine katılan (`entrypoints.audits[].domain`
  programın kimliği) etkin her eklentinin kuralı, kayıt defterinin sırasıyla, "Eklenti › Kuralın başlığı" diye. Liste
  ajanın `audits` listesiyle aynı yerden gelir: kayıtlı katkılar (eklenti çalışıyor, `audit.provider` verilmiş). Başka
  programın kuralı listelenmez (Mimari kuralının CBS'de olguları yok). Seçim bir sonraki çalıştırmaya dek tutulur;
  nereden başlarsa başlasın (şerit, seçici, ajan) yeni çalıştırma seçiciyi gösterilen kurala getirir.
- **Bir iş, bir düğme.** Seçici gösterilen kuralı sunuyorsa başlıktaki "Yeniden denetle" kalkar: gerçek pencerede
  seçicinin Denetle'siyle alt alta iki düğme aynı işi yapıyordu. Seçicinin sunmadığı kuralın bulgularında düğme kalır;
  çalışırken başlıkta Durdur vardır, Denetle kapalıdır.
- **Kural yokken** sekme ne yapılacağını söyler: "Bu programda denetim kuralı olan etkin bir eklenti yok. Eklentiler'den
  bir denetim eklentisi kurun (ör. Türkiye mevzuatı)."
- **Süre (aynı gün):** "2 hata (2.3 sn)" → "2 hata (2,3 sn)", eklenti günlüğündeki çalıştırma satırı da. Süre ölçü
  değildir, ADR 0149'un ondalık noktası onu kapsamaz (§17'nin örneği de "1,6 sn" diyordu). Bulgu metnindeki ölçüler
  ADR 0149'a göre noktalı kalır.
- **Gerçek pencere (İmar durumu çizimi, mevzuat 0.6.0 Eklentiler'den kurulmuş):** Mimari'de seçicide 9 kural; İmar
  durumu seçilip Denetle → 2 hata (Emsal 2.92 > 1.00, TAKS 1.44 > 0.40), köken "Türkiye mevzuatı 0.6.0 · imar-durumu ·
  paket 50e5a757e12e · çizim revizyonu 0".
- **Sınamalar:** masaüstü 1 (`audit_picker`: Mimari'de mevzuatın kuralları; seçim tutulur; başka yerden başlayan
  çalıştırma seçiciyi gösterilen kurala getirir; CBS'de hiçbiri), masaüstünün eklenti sınamaları (103), Asistan (17).

## §94. Kaynaktan derlenecek paket: `kek verify`, kurulum incelemesi ve genel katalog (`ADIM 18A`, PY-UV-20, 21; D3)

**16 Kasım.** Kod: paylaşılan `python/builds.rs` (yeni, `kentos_extensions::python_builds`: `Target`, `Machine`,
`unwheeled`, `fits`, `said`, `of_package`, `on_computer`; PEP 508 marker okuyucusu), `python.rs` (lint'in uyarısı);
python-env `python.rs` (`Cause::Build`, `build_words`), `extension.rs`, `personal.rs`; katalog `acting.rs`
(`build_gate`); masaüstü kurulum incelemesi (`view.rs` `on_this_computer`, `Pending.here`), `python_personal.rs`.

- **Sorun.** Bir paketin bu bilgisayara uyan hazır kurulum dosyası (wheel) yoksa uv onu kaynak dağıtımından
  kullanıcının bilgisayarında derler: paketin derleme betiği kullanıcının yetkileriyle çalışır, kilitte olmayan derleme
  araçlarını indirir, C eklentisi için derleyici ister. Bunu kimse söylemiyordu: `required-environments` yalnız kaynak
  dağıtımı olmayan paketlerden wheel ister (uv'nin kuralı), D3 denetimi de yalnız sistemin adına bakıyordu. Sunucu hiç
  derlemez (§66), masaüstü sessizce derler. Derleme düşünce derleme betiğinin kendi `error:` satırı uv'nin sözü sanılıp
  "Eklentinin ortamı hazırlanamadı" diye gösteriliyordu (uv 0.11.26 ile ölçüldü: ağsız, kendi derleme betiği bilerek
  düşen bir kaynak paketi).
- **Karar: kilitten okunur, hiçbir şey çalıştırılmaz.** Her bilgisayar için kurulan paketler projenin bağımlılıklarından
  marker'ları o bilgisayar için okunarak izlenir (her isteğe bağlı özellik dahil, geliştirme grupları değil; bir
  bağımlılığın istediği extra'lar ve çatallanan sürümler de). Paketin wheel etiketlerinden (PEP 425; platform etiketi
  `packaging`'in okuduğu gibi) biri bilgisayara ve Python'a uymuyorsa kaynağı varsa (depodan gelenin her zaman)
  **kaynağından derlenir**, yoksa **kurulamaz**. Kilidin desteklemediği bilgisayar (`supported-markers`,
  `requires-python`) yargılanmaz. Bilgisayarlar KentOS'un çalıştığı altısı (`compat.json` ile aynı olduğu sınanır),
  Python KentOS'un kurduğu 3.12; kilit almıyorsa KentOS'un çalıştırdıklarının en eskisi. Bilerek yalın: glibc'nin ve
  macOS'un en düşük sürümü karşılaştırılmaz, KentOS'un Linux'u glibc'dir (musl wheel'i sayılmaz), okunamayan marker
  tutar sayılır (paket atlanmaktansa yargılansın).
- **Üç yerde.** `kentos kek verify` ve ajanın `extension.verify`'ı: paket başına uyarı, bilgisayarlarıyla
  (`python_source_build`, `python_no_wheel`). Kurulum incelemesi: Python bölümünde yalnız bu bilgisayar için, KentOS'un
  buradaki Python'uyla (hazırsa onun sürümü): derlenecek paketler, kurulamayacaklar, kilidin bu bilgisayarı
  desteklemediği; yazara dönük cümleler Uyarılar'da yinelenmez. Genel katalog: gönderilen sürümün bulguları Bağımlılık
  denetimini engeller ve sürüm yazarına döner; yazar notla yeniden gönderir, inceleyici gerekçeyle kaldırır (planın
  "gerekçeli istisna"sı; kapının var olan gerekçeli kaldırma yolu). Kurum ve kişisel katalog engellemez: o
  masaüstlerinin incelemesi derlemeyi söyler.
- **Düşen derleme paketin hatasıdır (PY-UV-20).** `Cause::Build`: uv'nin "Failed to build `paket==sürüm`" satırı (uzun
  ad `│` altında sarılır) ve `[stderr]` altındaki derlemenin son sözü okunur: "“tasli” 0.3 kaynağından derlenemedi: bu
  bilgisayar için hazır kurulum dosyası (wheel) yok ve derlemesi başarısız oldu. Bu, eklentinin kendi hatası değil;
  çoğu zaman derleyici ya da paketin istediği bir sistem kütüphanesi eksiktir. …" Eklentinin ortamında ve kişisel
  ortamda aynı cümle; `kek test` bunu "çalıştırılamadı" sayar, eklentinin başarısızlığı değil.
- **Söz:** arayüzde "tekerlek" farenin tekerleğidir ("tekerlek: yaklaş"); Python'un wheel'i için aynı sözcük çakışırdı.
  Kullanıcıya görünen metin "hazır kurulum dosyası (wheel)" der.
- **Ölçüm:** Vitrin'in kilidinden Linux wheel'leri çıkarılınca `kek verify` iki uyarı verir (numpy 2.5.4, shapely
  2.0.7: Linux (x86_64), Python 3.12). Gerçek pencerede kurulum incelemesi: "Bu bilgisayar (Linux x86_64, Python 3.12)
  için hazır kurulum dosyası (wheel) olmayan paketler kaynağından derlenecek: numpy 2.5.4, shapely 2.0.7. …". Vitrin'in,
  mevzuatın ve fikstürlerin kendi kilitlerinde uyarı yok.
- **Sınamalar:** paylaşılan 7 (`python::builds`: etiketler, marker'lar, gezinme: marker, extra, çatal, isteğe bağlı
  özellik, depo; desteklenmeyen bilgisayar; bilgisayarlar ve sözleri; `compat.json` ile aynılık; Vitrin'in kilidi) ve 1
  (lint'in uyarıları), python-env 1 (uv 0.11'in sözü, sarılmış ad, derlemenin son sözü), katalog API 1 (iki depoda:
  engel, notla yeniden gönderim, gerekçeli kaldırma, wheel'li kilit geçer, kişisel katalog engellemez), masaüstü 1 (bu
  bilgisayarda derlenecekler, Uyarılar'da yinelenmez, pencere çizilir).
- **Kalan:** derlemeyi yalnız incelemede listelenen paketlere açmak (uv'de izin listesi yok; `--no-build-package`
  tersidir); kilidin desteklemediği bilgisayarı (ajan iskeletinin üç bilgisayarlık kilidi Linux aarch64'te) kurulumdan
  önce uyumsuzluk saymak; glibc ve macOS'un en düşük sürümü.

## §95. Kişisel ortamdaki paket, geliştirilen eklentinin bağımlılığına (`ADIM 18A`, PY-OPT-06)

**16 Kasım.** Kod: python-env `extension/dependency.rs` (yeni: `requirement_for`, `add_dependency`, `add_words`);
masaüstü `python_personal/add_to.rs` (yeni: `AddTo`, `Developed`, `developed_extensions`, `add_to_part`,
`add_to_start`, `add_to_done`), `python_personal.rs` (satırdaki "Eklentiye ekle…"), `extensions/workshop.rs`
(`ext_reload_folder`: atölyenin yeniden yüklemesinin bildirimsiz çekirdeği).

- **Sorun.** Planın NumPy örneği "ihtiyacı seç → kurulacakları gör → kişisel ortama kur → konsolda kullan →
  betik/eklenti bağımlılığına ekle" diyor; son adım yoktu. Konsolda denenen paketi eklentiye almak için geliştirici
  `pyproject.toml`'u elle düzenleyip `uv lock` çalıştırmalıydı. Betikler kişisel ortamda çalışır: onların bağımlılığı
  ortamın kendisidir.
- **Karar.** Kişisel ortamın kurulu paket satırında "Eklentiye ekle…": geliştirilen bir eklenti seçilir (geliştirici
  klasöründen kurulmuş, Asistan'ın atölyesi ya da bağlantıdaki bir ajanınki, Python projesi olan; paketlenmiş `.kek`
  ya da katalog sürümü yerinde düzenlenmez) ve Ekle. Yazılan istek denenen sürüm ve serisidir (`openpyxl>=3.1.5,<4`;
  0.x serisinde küçük sürümü: `fiona>=0.13.1,<0.14`).
- **uv'nin kendi yolu, eklentinin kendi ayarları.** `uv add --no-sync` proje klasörünün KentOS'un klasöründeki
  kopyasında çalışır; yalnız çözülen kilit `pyproject.toml` ve `uv.lock`'un yerine geçer (yanına yazılıp üzerine
  taşınır), çözülmezse eklentinin dosyaları değişmez. `--no-config` verilmez: uv 0.11.26 ile ölçüldü, `--no-config`
  `[tool.uv] environments`'ı tutar ama `exclude-newer`'ı düşürür ve ajanın kilidinin soğuma süresi (§83) kaybolurdu.
  Projenin `uv.toml`'u da kopyalanır.
- **Zaten bağımlıysa** söylenir, değiştirilmez: "“numpy” zaten “Eklenti Vitrini” eklentisinin bağımlılığı (kilitte
  2.5.4); değiştirilmedi."
- **Sonra** eklenti klasöründen yeniden yüklenir (atölyeninkiyle aynı çekirdek: bir çalıştırması sürerken ya da
  verilmemiş yetenek isterse yüklenmez), kurulu eklenti ile klasörü ayrışmaz; ortamı yeni kilitle, önizlemesiyle
  Eklentiler'den hazırlanır. Bildirim: "Eklenti Vitrini: bağımlılık eklendi — pyproject.toml'a “openpyxl>=3.1.5,<4”
  yazıldı. Kilit yenilendi; yeni: et-xmlfile 2.0.0, openpyxl 3.1.5. Eklenti klasöründen yeniden yüklendi; Python
  ortamını Eklentiler'den yeni kilitle hazırlayın." Eklentinin günlüğüne `dependency_added` düşer.
- **Gerçek pencere (gerçek uv ve PyPI):** Vitrin'in kopyası bağlantıdaki bir ajanın atölyesinden incelemeye açılıp
  kuruldu (`review_workshop`), kişisel ortam oluşturuldu, openpyxl 3.1.5 önizlenip kuruldu; satırdan Eklentiye ekle →
  Ekle: uv klasöre `openpyxl>=3.1.5,<4` yazdı, kilide openpyxl 3.1.5 ve et-xmlfile 2.0.0 girdi, bildirim 2,5 sn
  içinde; değişen klasör `kek verify`'dan 0 uyarıyla geçti.
- **Sınamalar:** python-env 1 (sahte uv: istek ve serisi, `--no-config` yok, `uv.toml` kopyalanır; başarıda dosyalar
  değişir ve kopya silinir; çözülmezse dosyalar aynı kalır ve neden söylenir; projesiz klasör), masaüstü 1 (atölyeden
  kurulu Vitrin: zaten bağımlı numpy söylenir ve uv çalışmaz; yeni paket eklenir, kilit yenilenir, eklenti yeniden
  yüklenir, bildirim), masaüstünün eklenti, Asistan ve Python sınamaları (159).
- **Kalan:** kurulu olmayan bir klasöre (geliştiricinin kendi projesi) eklemek; isteğe bağlı bir özelliğe (extra)
  eklemek.

## §96. Kurum ilkesi eklenti kodunu sunucudan uzak tutabilir (`ADIM 18A`, PY-OPT-12)

**16 Kasım.** Kod: kayıt defteri `policy.rs` (`allowExtensionsOnServer`, `Policy::server_refusal`), `registry.rs`
(`Registry::policy`, `set_policy`); masaüstü `gis/processing/server.rs` (`server_place_for`, `processing_server_place`;
Çalıştır'da ikinci denetim), `window.rs`, `mod.rs`; `kentos_ui::widget::radio` (uzun not adın altında).

- **Sorun.** PY-OPT-12: "kurum politikası dış dizin ya da bulut yürütme sınırı koyabilir, kullanıcı onayı politikayı
  genişletemez." Dış dizin §84'te yapılmıştı (izinli paket depoları); bulut yürütme yoktu. Masaüstünden eklenti
  kodunun sunucuya gidebildiği yol İşlemler penceresidir: açık bulut veritabanı projesinde her aracın "Nerede
  çalışır"ında "KentOS sunucusunda" vardır (ADR 0559 §5), eklentinin aracı (`ext.…`) dahil. Asistan ve bağlantıdaki
  ajanlar sunucu işi başlatmaz, modeller sunucuya gitmez.
- **Karar.** İlkede `allowExtensionsOnServer: false`: eklentinin aracında sunucunun satırı kapalı ve nedeniyle ("kurum
  ilkesi eklenti kodunun sunucuda çalışmasına izin vermiyor"); pencerenin hatırladığı seçim bunu aşamaz (kapalı yer
  seçilemez) ve Çalıştır göndermeden önce bir kez daha sorar. KentOS'un kendi araçları etkilenmez; yazılmamışsa evet.
  Okunamayan ilke eklenti kodunu burada tutar (kapalı başarısızlık), KentOS'un araçlarını değil.
- **Bileşen düzeltmesi (aynı iş).** "Nerede çalışır"ın ortak `RadioGroup::notes_at_end` satırı uzun notu satır sonuna
  yazınca adı sıkıştırıyor, ad ile not üst üste çiziliyordu. İlkenin sözüyle görüldü; var olan "yetkiniz yok
  (project.jobs.run)" notu da 1100×650'de aynı çakışmayı yapıyordu. Kural bileşende: ad ve not birlikte 34 harften
  uzunsa not adın altına, küçük ve sönük iner; kısalar ("Otomatik" · "şimdi: bu bilgisayarda", 30) satır sonunda kalır.
- **Resim:** İşlemler'in ekran sınamasına "ilke" görünümü eklendi (Vitrin kurulu, bulut projesi, ilke kapalı, yan
  panel Nerede çalışır'a kaydırılmış): `.run/shots/islem-sunucu-ilke-1440x900*.png`. Gerçek pencere çalışan bir KentOS
  sunucusu ve oturum ister.
- **Sınamalar:** kayıt defteri 1 (kapalı, yazılmamış, açık, okunamayan; KentOS'un aracı), masaüstü 1 (yer: kapalı
  ilkede eklentinin aracı kapalı ve sözüyle, KentOS'un aracı açık; yazılmamış ve ilkesiz açık; okunamayan; yerel
  çizimde yok; Vitrin'in TAKS penceresinde satır kapalı ve notuyla), bileşen kitaplığının sınamaları (366),
  masaüstünün İşlemler ve eklenti sınamaları (217).

## §97. Geliştirme ortamı: çalışma klasöründen kurulan eklentinin kendi türü (`ADIM 18A`, PY-UV-10)

**16 Kasım.** Kod: python-env `extension.rs` (`Key::development`, `Key::for_development`, `find`'ın `development`'ı,
`sync_args`); kayıt defteri `registry.rs` (`Registry::developed`); eklenti araçları `plan.rs` (`Plan::development`);
masaüstü `extensions/environment.rs`, `runner.rs`.

- **Sorun.** PY-UV-10 üç ortam türünü ayırır: kişisel çalışma alanı, eklenti geliştirme ortamı, değişmez yayın ortamı.
  Kişisel ortam (§5) ve sürüm başına yayın ortamı vardı; çalışma klasöründen kurulan eklenti (Asistan'ın atölyesi,
  bağlantıdaki bir ajanınki, §22, §30) de yayın ortamını kullanıyordu: projenin geliştirme grubu
  (`[dependency-groups] dev`) hiç kurulmuyor, geliştirilen ile yayımlanan aynı türden bir ortamı paylaşıyordu.
- **Karar.** Ortam anahtarında `development`: kayıt defteri eklentinin kaynağı geliştirici klasörüyse (`developed`)
  masaüstü ve başsız ana makine onun ortamını bu işaretle hazırlar ve arar. Geliştirme ortamı `uv sync`'i `--no-dev`
  olmadan çalıştırır (projenin geliştirme grubu girer), başka bir klasördedir ve bir yayın sürümünün ortamı olarak hiç
  bulunmaz (`find` işareti ayırır). Yayın ortamlarının anahtarı, özeti ve kaydı bayt bayt aynı kalır: işaret yalnız
  doğruyken özete ve kayda yazılır; var olan klasörler yeniden hazırlanmaz. Kurulum ve ortam kuralları (önizleme,
  hazırlanmadan çalışmama, internetsiz çalışma) aynıdır.
- **`kentos kek test` yayın ortamında kalır:** sınama, paketin yayımlandığı hâliyle çalışıp çalışmadığını söyler;
  geliştirme grubunda olup yayında olmayan bir içe aktarma orada düşmelidir.
- **Ayrıntıda söylenir:** "Python ortamı" bölümünde "Geliştirme ortamı: eklenti çalışma klasöründen kurulu; projenin
  geliştirme bağımlılıkları da kurulur ([dependency-groups] dev). Yayımlanan sürümün ortamı onları almaz."
- **Gerçek pencere:** Vitrin'in kopyası bağlantıdaki bir ajanın atölyesinden kuruldu, Ortamı hazırla → önizleme (numpy
  2.5.4, shapely 2.0.7) → Hazırla: "Ortam hazır · Python 3.12.13 · 2 paket · internetsiz çalışır"; diskteki kaydında
  `"development": true`.
- **Sınamalar:** python-env 1 (aynı sınamanın içinde: geliştirme anahtarı başka klasör, yayın anahtarının kaydında
  işaret yok; geliştirme eşitlemesi `--no-dev`'siz; `find` iki türü ayırır), kayıt defteri 1 (klasörden kurulan
  geliştirilir, dosyadan kurulan değil), eklenti araçlarının sınamaları (12; atölye kurulumu artık geliştirme
  ortamıyla), masaüstünün eklenti, Asistan, Python ve İşlemler sınamaları (271).

## §98. Derleme yalnız incelemede listelenen paketlere açık (`ADIM 18A`, PY-UV-21; §94'ün kalanı)

**16 Kasım.** Kod: paylaşılan `python/builds.rs` (`wheeled_on`; `project_of` artık Python projesini alır); python-env
`extension.rs` (`prepare`, `preview` ve eşitleme `no_build` alır, `sync_args`), `python.rs` (`no_build_words`);
masaüstü `extensions/environment.rs` (`Context::not_to_build`); komut satırı `kek_test/setup.rs`.

- **Sorun.** §94 kaynaktan derlenecek paketi kurulumdan önce söylüyordu, ama ortam hazırlanırken uv tekerleği uymayan
  başka bir paketi de sessizce derleyebilirdi (uv'nin platform eşlemesi bizimkinden ayrılırsa, ya da kilit
  incelemeden sonra değişirse). PY-UV-21: "derleme kancaları kod yürütür, açık kapsamla yönetilir". §94'te "uv'de izin
  listesi yok" diye bırakılmıştı; uv'nin paket başına `--no-build-package`'ı ters yönden aynı işi görür.
- **Karar.** Ortam hazırlanırken ve önizlenirken bu bilgisayarda tekerleği uyan her kilitli paket (`wheeled_on`:
  kilidin bütün paketleri, incelemenin burada derlenecek dediği paketler hariç) uv'ye `--no-build-package` ile verilir.
  Yalnız incelemede listelenen paketler derlenebilir; uv bir başkasını derlemek isterse reddeder ve söz paketin adıyla
  Türkçe söylenir: "“numpy” 2.5.4 bu bilgisayarda kaynağından derlenmek zorunda, ama kurulum incelemesinde burada
  derlenecekler arasında değildi: KentOS onu derlemez. …" Önizleme de aynı bayraklarla koşar: ret indirmeden önce
  görülür. Masaüstü ve `kentos kek test` listeyi KentOS'un buradaki Python'uyla aynı yoldan hesaplar; sunucu zaten
  hiç derlemez (§66).
- **Ölçüm (gerçek uv 0.11.26, ağsız, KentOS'un önbelleği):** Vitrin'in kilidi `--no-build-package numpy
  --no-build-package shapely` ile eşitlenir (iki paket kurulacak). Linux tekerlekleri çıkarılınca aynı bayraklarla:
  "error: Distribution `numpy==2.5.4 @ registry+https://pypi.org/simple` can't be installed because it is marked as
  `--no-build` but has no binary distribution" (sözün ayrıştırıcısı bu metinle sınanır).
- **Sınamalar:** paylaşılan 1 (`wheeled_on`: tekerlekli, işaretiyle bu bilgisayarda kurulmayan ve depodan gelen
  paketler; projenin kendisi yok; okunamayan kilit), python-env 2 (önizlemede bayraklar; uv'nin reddinin sözü),
  komut satırı, eklenti araçları ve masaüstünün sınamaları.

## §99. SBOM'da lisanslar komut satırında ve katalogda da (`ADIM 18A`, PY-UV-29; §91'in kalanı)

**16 Kasım.** Kod: paylaşılan `sbom.rs` (`licenses_in_venv`: masaüstünün `bom::licences_in`'i taşındı;
`licenses_in_wheels`, `metadata_of`); python-env `extension.rs` (`find_published`); komut satırı `kek sbom`; katalog
`extension_catalog.rs` (`Catalog::wheels`, `wheels_dir`), `http/extension_catalog.rs` (SBOM ucu), `main.rs`.

- **Sorun.** Kilit lisans söylemez; §91'de lisansı yalnız masaüstü, eklentinin hazır ortamındaki `METADATA`'dan
  okuyordu. Komut satırının ve kataloğun SBOM'u lisanssızdı: kurumun bilgi işlemi bağımlılıkların lisansını
  karşılaştıramıyordu.
- **Komut satırı.** `kentos kek sbom`, KentOS'un bu sürüm için hazırladığı bir yayın ortamı varsa (`find_published`:
  aynı kimlik, sürüm ve kilit özeti, bu sistem; geliştirme ortamı ve sunucunun ortamları sayılmaz; Python'u ne olursa
  olsun) lisansları onun `site-packages`'ından okur ve söyler ("lisanslar ortamdan: 2 paket"); yoksa lisanssız yazar ve
  neden: "bu sürümün ortamı hazırlanmadı (kentos kek test hazırlar)".
- **Katalog.** Sunucunun tekerlek klasörü (`KENTOS_AUDIT_WHEELS`, denetim ortamlarının) SBOM ucuna verilir; kilitteki
  her paketin orada bulunan ilk tekerleğinin `*.dist-info/METADATA`'sı açılır (zip okuyucu yalnız o girdiyi açar,
  merkez dizin 200 000 girdiye dek okunur: büyük bir tekerlek binlerce dosya listeler). Dosya adı kilitten gelir, eğik çizgi
  taşıyan ad okunmaz: klasör dışına çıkılamaz. Tekerleği orada olmayan paket lisanssız kalır.
- **Ölçüm (gerçek uv ve Vitrin):** `kek test` yayın ortamını §98'in bayraklarıyla hazırladı (9 örnek, 5 sınama
  geçti), ardından `kek sbom`: numpy "BSD-3-Clause AND 0BSD AND MIT AND Zlib AND CC0-1.0", shapely "BSD 3-Clause".
- **Sınamalar:** paylaşılan 1 (tekerlekten lisans: ilk bulunan tekerlek, eksik ve bozuk tekerlek, okunmayan kilit),
  python-env 1 (aynı ortam sınamasında `find_published`), katalog API 1 (iki depoda: tekerlek klasöründeki numpy'nin
  lisansı listede, tekerleği olmayan shapely lisanssız), masaüstünün SBOM sınamaları (2; taşınan okuyucuyla).
- **Kalan:** tekerlek klasörü olmayan sunucuda lisans (paket dizininden tekerlek indirmek bir ağ kararı).

## §100. Sır taraması genişledi: yeni sağlayıcılar ve bağlantı adresindeki parola (`ADIM 22A`, PY-CLOUD-05; D11)

**16 Kasım.** Kod: paylaşılan `secrets.rs` (`FORMS`'a on biçim; `SCHEMES`, `stands_in`, `passwords_in_addresses`).

- **Sorun.** ADR 8059 §15'in taraması sağlayıcıların anahtar biçimlerini ve özel anahtarı tanıyordu. İki boşluk vardı:
  eklentilerin kodunun çağırdığı yeni yapay zekâ sağlayıcılarının ve kurumların ulaştığı birkaç hizmetin biçimleri,
  bir de en olası sızıntı: belediyenin PostGIS'ine bağlanan eklentinin koduna ajanın yazdığı
  `postgresql://kullanici:parola@sunucu/veritabani`. Hiçbir sağlayıcı biçimine uymaz, paylaşılan paketle gider.
- **Yeni biçimler:** OpenRouter (`sk-or-v1-`), Groq (`gsk_`), xAI (`xai-`), Perplexity (`pplx-`), Replicate (`r8_`),
  NVIDIA (`nvapi-`), Google OAuth istemci sırrı (`GOCSPX-`), Mapbox gizli jetonu (`sk.eyJ…`; açık `pk.` jetonu sır
  değil), Docker Hub (`dckr_pat_`), Shopify (`shpat_`, `shpss_`). Her biri öneki, harf kümesi ve uzunluğuyla; kural
  az ve kesin kalır (D11).
- **Bağlantı adresindeki parola:** bilinen şemalar (PostgreSQL/PostGIS, MySQL/MariaDB, SQL Server, Oracle, MongoDB,
  Redis, AMQP, FTP/SFTP, SMTP, HTTP/HTTPS: paket deposu ya da parolalı WMS) ve `kullanici:parola@` biçimi. Parolanın
  yerini tutan değer sayılmaz: değişken (`${PGPASSWORD}`, `%s`, `{sifre}`, `<parola>`), yalnız onu adlandıran sözcük
  ("password", "parola", "şifre"…; sözcüğün tamamı, gerçek parola "Compass2024" gibi bir parçayı taşıyabilir),
  yıldızlar, örnek değer. Söz değeri hiç söylemez: "bağlantı adresindeki parola", satırıyla. Paketlemede ve kurulumda
  uyarı (`secret_in_text`), katalogda sır kapısını engeller (`secret_found`), atölye ve `kek verify` de aynı sözü alır.
- **Ölçüm:** depodaki paketlerde (Vitrin, Türkiye mevzuatı, MEBİS, örnek imar, örnek altlık) yeni uyarı yok.
- **Sınamalar:** paylaşılan 2 (yeni biçimler çalışma anında kurulan değerlerle: depo anahtar biçiminde bir değer
  taşımaz; bağlantı adresindeki parola, port ve sorgu dizesiyle, kapının sözünde değer yok; değişken, sözcük, yıldız,
  boş parola, parolasız adres ve bilinmeyen şema sayılmaz), ortak kitaplığın bütün sınamaları (176), katalog API (58).
- **Kalan:** yalıtılmış duman testi işçisi (PY-CLOUD-06; sunucunun ağ, dosya ve kiracı yalıtımı ECO-DEC-11, sahibin
  kararı).

## §101. Çevrimdışı, sürümlü SDK belge paketi: `kentos kek docs` (`ADIM 21`, EKL-06)

**16 Kasım.** Kod: komut satırı `docs.rs` (yeni), `main.rs` (komut ve yardım metni).

- **Sorun.** EKL-06 "sürümlü Python API referansı … çevrimdışı belge paketi" ister. SDK'nın her parçası vardı (`kek sdk`,
  `--part`, MCP'nin SDK aracı), ama ağsız bir makinede okunacak, bir yapay zekâ ajanına klasör olarak verilecek ve
  hangi KentOS sürümünün olduğunu söyleyen bir paket yoktu.
- **Karar.** `kentos kek docs <klasör>` aynı paketin parçalarını dosya dosya yazar: `llms.txt` (rehber ve `kentos.ext`
  imzaları, `kek sdk --text`'in aynısı), `kentos_ext.pyi`, `sdk.json` (paketin tamamı), `semalar/` altında altı şema
  (manifest, panel, katman şeması, İşlemler aracı, olgular, denetim örnekleri), yetenekler, programlar, katkılar,
  doğrulama kodları, panel davranışı ve kurucuları, numaralı iş akışı, ikon adları, `ornek-eklenti/` (her katkı
  türünün çalışan örneği), `surum.json` (`kentos.sdk-docs`, KentOS sürümü, SDK protokolü) ve Türkçe `BENIOKU.md`
  (dizin; ajana önce `llms.txt`'nin verileceğini söyler). Aynı koddan üretildiği için paket programından ayrışamaz.
- **Güvenli yazım:** boş olmayan ve belge paketi olmayan klasör reddedilir (`surum.json`'u olan paket yeniden yazılır);
  örnek eklentinin yolları klasör dışına çıkamaz (`..`, kök, sürücü harfi).
- **Söz:** dizin sayfası sürüme ek bağlamaz ("KentOS 0.1.0 sürümünün"): sayıya gelen ek okunuşuna göre değişirdi
  ("0.1.0'ın", "0.2.1'in").
- **Ölçüm:** 33 dosya, 444 KB; içindeki örnek eklenti `kentos kek verify`'dan geçer (kuru koşu 9 katkı, kalıntı yok).
- **Sınamalar:** komut satırı 1 (her parça kendi dosyasında, JSON'lar okunur, sürüm ve dizin, örnek eklenti doğrulamadan
  geçer, paket yeniden yazılır, yabancı klasör reddedilir, güvenli yol).

## §102. Çalışan işler: arka plandaki işler tek listede, kimin başlattığıyla (`ADIM 21`, EKL-06)

**16 Kasım.** Kod: masaüstü `works/` (yeni: `mod.rs`, `sources.rs`, `view.rs`, `tests.rs`), durum çubuğu
(`view/status.rs`), alt panel (`bottom.rs`, `module_surfaces.rs`, `layout.rs`), ajan bağlantısı (`python/agents.rs`,
`python/mod.rs`), İşlemler (`processing/background.rs`, `mod.rs`), Asistan (`assistant/mod.rs`), eklenti denetimi ve
deneme koşusu (`audits.rs`, `trial.rs`, `examples.rs`); headless `rpc.rs` (`Jobs::peek`, `Jobs::stop`), `lib.rs`
(`command_title`).

- **Sorun.** EKL-06 "çalışan işler … kullanıcıya anlaşılır gösterilir" ister. Arka plandaki işler dağınıktı: İşlemler
  koşusu penceresinde, eklenti komutu bir bildirimde, denetim kendi sekmesinde, Python kurulumları ayarlarda, Asistan'ın
  turu panelinde. Bağlı bir ajanın (MCP) başlattığı iş ise hiçbir yerde görünmüyordu; hesaplanıp ajanın sormasını
  bekleyen bir yazma işi de. Kullanıcı çiziminde kimin ne yaptığını göremiyor, durduramıyordu.
- **Karar: tek liste ve durum çubuğunda bir hücre.** Hücre yalnız iş varken görünür (tek işte adı ve yüzdesi, birden
  çokta sayısı, dar çubukta yalnız sayı); tıklanınca her programın alt panelindeki "Çalışan işler" sekmesi açılır. Her
  iş bir satır: adı, türü, **başlatan** (Siz, Asistan, Bağlı ajan), başlangıç saati, ilerleme çubuğu ve adımı, Göster
  (işin kendi sekmesi ya da penceresi) ve Durdur. Sıra: önce kullanıcının uzun işleri, sonra Asistan'ınkiler ve
  ajanlarınkiler.
- **Kaynak: işin kendi durumu, ayrı kayıt yok.** Liste her gösterilişinde işlerin kendi durumlarından okunur (İşlemler
  koşuları, eklenti komutları, denetim, deneme koşusu, ajan işleri, Asistan'ın turu, konsoldaki kod, KentOS'un Python
  kurulumu, kişisel ortam, eklenti ortamları). Kayıt defteri tutulmadığı için biten iş listede kalamaz; yeni bir arka
  plan işi `App::works`'e bir okuyucu ekleyerek girer.
- **Başlatan:** ajan işi, denetim ve deneme koşusu kimin isteğiyle başladığını taşır (bağlantının ve Asistan'ın
  isteklerini ayıran `who`); geri kalanları yalnız kullanıcı başlatır. Asistan'ın turunu kullanıcı başlatır; turun
  başlattığı iş "Asistan" yazar.
- **Durdur işin kendi durdurmasıdır:** kendi penceresindeki, bildirimindeki, sekmesindeki düğmeyle aynı işleyici ve
  aynı sonuç (yazdığı geri alınır, günlüğü ve bildirimi). Ajan işinde yeni yol: iş önce durdurulur, sonra kendi
  çiziminde sorulur; hesaplanmış ama yazılmamış sonuç **yazılmadan** düşer (`Jobs::cancel` önce sorduğu için
  hesaplanmışı yazardı). Ajan bir sonraki sorusunda `cancelled` duyar, Asistan'ın bekleyen çağrısı da.
- **Bekleyen iş dürüst söylenir:** hesaplanıp ajanı bekleyen yazma "Hesaplandı; yazmak için ajanı bekliyor", plan "Plan
  hazır; ajanı bekliyor"; çubuğu dolu, hareketli değil. Ajan hiç sormazsa iş listede kalır: sonucu gerçekten bekliyor ve
  kullanıcı durdurabilir.
- **Adlar Türkçe, arayüz beklemeden:** ajan işi komutun katalogdaki adıyla görünür ("Kapalı alan oluştur"). Katalog bir
  kez, kendi iş parçacığında okunur (bağlantı açılırken ve Asistan'ın turu başlarken); okunana dek komutun kimliği
  yazılır. **Ölçüm:** ilk sürüm kataloğu arayüz iş parçacığında okuyordu; debug derlemesinde ilk ajan işi 2 saniye geç
  yanıtlandı (art arda gönderilen iki işin başlangıcı 21:47:05 ve 21:47:07). Şimdi ikisi aynı saniyede.
- **Kapsam dışı (bilerek):** kendi penceresi açıkken çalışan işler (Topoloji, Dizi, CAD denetimi …: penceresi gösterir,
  Durdur'u oradadır) ve kataloğun sunucu işleri (sunucuda çalışır, Katalog bölmesinde listelenir). Sekmenin altındaki
  not bunu söyler.
- **Gerçek pencere (Xvfb, KentOS GIS, Vitrin'in çizimi):** bağlı ajan iki iş (yazma ve plan) ve Vitrin'in deneme
  koşusunu başlattı: hücre "▷ 3", sekmede üç satır, ipucu üçünü sayar; deneme koşusu yaklaşık 5 saniyede bitip listeden
  düştü. Yazma işinde Durdur: bildirim, çizime bir şey yazılmadı, ajan sorunca `"phase": "cancelled"`. Plan işi ajan
  sorunca bitti, liste boşaldı. İlk koşuda üç satırla kaydırma çubuğu Durdur düğmelerini örtüyordu ve düğmesi az satırın
  çubuğu kayıyordu: çubuk satırların yanına alındı, düğme alanı sabit genişlikte.
- **Sınamalar:** masaüstü 4 (ajan işi: başlatan, bekleme sözü, Durdur'da yazılmaması ve `cancelled`; Asistan'ın ve
  ajanın ayrımı, hücrenin sayımı, sorulan işin listeden çıkması, Göster; Python kurulumunun, kişisel ortamın ve eklenti
  ortamının kendi Durdur'u, Durdur'u olmayan işte düğme olmaması; hücre ve sekme); her programın sekmelerinde "Çalışan
  işler" (`module_surface_tests`).

## §103. Öznitelik değerleri yapay zekâya gitmez: ajan başına seçenek ve kurum ilkesi (`ADIM 21`, [PY-AI-13])

**16 Kasım.** Kod: headless `veil.rs`, `veil/every.rs` (yeni), `veil/tests.rs` (`veil.rs`'den taşındı), `rpc.rs`
(`call_for_agent_with`, `Jobs::start_veiled`), `dispatch.rs` (`call_envelope_veiled`); ajan `profiles.rs`
(`withholdValues`); eklenti ilkesi `policy.rs` (`allowAttributeValuesToAi`); masaüstü `assistant/` (`withholds_values`,
ajan formu, panel notu, yönergeler, `values_tests.rs`), `python/agents.rs`.

- **Sorun.** [PY-AI-13] "sağlayıcıya gönderilecek veri kullanıcı/kiracı politikasıyla sınırlanır" der. ADR 8056 §7
  kuruma çizimi dışarıdaki yapay zekâya hiç göndermeme yolunu verdi; izin verilen sağlayıcıya ise çizim bütünüyle
  gidiyordu. Belediye çiziminin öznitelikleri kişisel veri taşır (malik adı, kimlik numarası, adres) ve ne kurum ne
  kullanıcı "çizimle çalışsın ama değerleri görmesin" diyebiliyordu. ADR 8056 §7 bu adımı tarif etmişti: bütün alanlara
  çizimin gizli alan perdesi.
- **Karar: bütün alanlara perde.** Seçenek açıkken ajan çizimi her alanı gizliymiş gibi okur (`Veil::everything`):
  nesnelerin özniteliklerinde geçen bütün alan adları, formların ve katmanların gizledikleriyle. Değerler `‹gizli›`
  yazılır (öznitelik haritaları, birleştirme hücreleri, istatistikler, uzlaştırma); bir alanı anan sorgu, süzgeç, ifade
  ve hesap reddedilir, ret nedenini söyler. Geometri, katman ve alan adları, yazılar gider.
- **Serbest metin:** eklentinin bulgu sözü gibi metinlerde değer tırnaksız da sözcük olarak gizlenir (en az üç harf, en
  çok dört sözcük, en uzunu önce): bir eklenti `Malik Ayşe Yılmaz` diye yazabilir. Yanıtın parçalarını adlandıran
  yerlere (`id`, `layerId`, `kind`, `command` …) dokunulmaz; üç harften kısa değer yalnız öznitelik haritasında gizlenir
  (`1`, `A` her sayıyı keserdi). Gizlilikte fazla kesmek eksik kesmekten iyidir.
- **Yazmak serbest:** ajan bir alana değer yazabilir, dışarı veri gitmez. Çizimin gizli alanlarında yazmak eskisi gibi
  sahibinindir.
- **Kim karar verir:** ajan formunda "Gönderilen veri › Öznitelik değerleri › Gönderilmesin" (ajan başına; bu
  bilgisayardaki bir sunucuda gerekmez) ve kurum ilkesinde yeni `allowAttributeValuesToAi: false` (bu bilgisayarın
  dışındaki ajanlar için; form kutuyu işaretli ve kilitli gösterir, nedenini yazar). Eski KentOS bilinmeyen alanı
  görünce ilkeyi kapalı okur (§84'teki yol).
- **Ajana neden söylenir:** özet `valuesWithheld` taşır; yönergeler `‹gizli›`nin ne demek olduğunu söyler. **Ölçüm
  (gerçek pencere, DeepSeek):** neden söylenmezken ajan alanların çizimde gizli işaretli olduğunu sandı ve kullanıcıya
  alanların gizliliğini açmasını önerdi; `valuesWithheld` ile değerleri kullanıcının yazmasını ya da ayardan izin
  vermesini önerdi.
- **Panel:** Asistan panelinde sohbetin üstünde "Öznitelik değerleri bu ajana gönderilmiyor; yanıtlarında ‹gizli›
  yazar."
- **Eklenti denetim raporu da perdeden geçer:** `audit_report` bulguları eklentinin çizim hakkındaki sözleridir ve
  perdesiz gidiyordu, çizimin gizli alanlarında da (eski bir açık). Artık ajanın perdesi uygulanır (gizli alanlar ya da
  bütün alanlar).
- **Kapsam dışı:** bağlantıdaki ajanlar (MCP). KentOS verinin nereye gittiğini bilemez ve bağlantı ayrı bir izindir (ADR
  8056 §7); onlar çizimi eskisi gibi okur (sınamada denetlenir).
- **Sınamalar:** headless perde 1 (bütün değerler, serbest metin, yapısal anahtarlar, ret ve nedeni, yazmak serbest,
  gizli alanın kuralı aynı, özetin notu); ilke 1 (dışarıdakinde değer yok, bu bilgisayardakinde var, okunamayan ilke
  kapalı, ajan yine bağlanır); masaüstü 2 (sahte yerel sağlayıcı: ayar kapalıyken sağlayıcı değerleri alır; formdan
  "Gönderilmesin" kaydedilince aynı yanıtta alan adları ve ‹gizli› gider, değer gitmez; bağlantıdaki ajan değerleri
  okur; ilke dışarıdaki ajanda değerleri tutar, bu bilgisayardakinde tutmaz).
- **Gerçek pencere:** Vitrin çizimi (sentetik), DeepSeek deneme ajanı "Gönderilmesin" ile "parsellerin ada ve parsel
  numaralarını listele" isteğini aldı: değerleri göremediğini söyledi, listeyi uydurmadı (33.468 token).

## §104. Ajanın kendi Python'u ayrı bir yetki sınıfı (`ADIM 21`, [PY-AI-11])

**16 Kasım.** Kod: ajan `profiles.rs` (`PythonRuns`, `pythonRuns`); eklenti ilkesi `policy.rs` (`allowAgentPython`);
headless `agent_tools.rs` (`PYTHON_RUNS`, `Host::python_refusal`); masaüstü `assistant/` (`python_runs`,
`python_refusal`, onay, ajan formu, yönergeler, `python_tests.rs`), `assistant/host.rs`.

- **Sorun.** [PY-AI-11] "ajanın kişisel güvenilir Python yürütmesi ile kapsamlı araç çağrısı ayrı yetki sınıflarıdır"
  der. Asistan'ın araçlarından ikisi ajanın yazdığı kodu bu bilgisayarda kullanıcının yetkisiyle çalıştırır:
  `extension.workshop.trial` atölyedeki eklentiyi sentetik bir çizimde dener, `extension.workshop.reload`
  değişikliklerini kurulu eklentinin üstüne yükler (incelenmemiş yeni kod bir sonraki çağrıda çalışır). İkisi çizime
  yazmadığı için onay beklemiyordu: çizimi okuma ve komutla yazma izniyle aynı sınıftaydı. Ayrı süreç bir kum havuzu
  değildir (PY-RUN-19, 20).
- **Karar: üç seçenek, ajan başına.** Ajan formunda "Kod çalıştırma › Atölyedeki kodu": **Her seferinde sorsun**
  (varsayılan; çağrı onay bekler), **Sormadan çalıştırsın**, **Çalıştırmasın** (araçlar ajana sunulmaz, çağrılırsa
  reddedilir). Okuma, doğrulama (`extension.verify`), görünüşü okuma (`preview`) ve yazma araçları bu sınıfa girmez: kod
  çalıştırmazlar.
- **Ret araç katmanında:** `agent_tools::Host::python_refusal` (varsayılan: yok) kapalıysa iki araç `offered_tools`'tan
  düşer ve `call` onları `python_refused` ile reddeder; doğrudan çağrı da, modelin uydurduğu çağrı da. Onay ise
  masaüstünün işi (`needs_approval`): kart, kodun kullanıcının yetkisiyle çalışacağını ve seçeneğin nerede olduğunu
  söyler.
- **Kurum ilkesi:** yeni `allowAgentPython: false` her ajanın kendi kodunu çalıştırmasını kapatır, modeli bu
  bilgisayarda çalışan ajanınkini de (kod her durumda burada çalışır); form "Çalıştırmasın"ı seçili, diğerlerini sönük
  gösterir ve ilkenin cümlesini yazar. Okunamayan ilke kapatır.
- **Bağlantıdaki ajanlar (MCP) değişmedi:** onlar zaten kullanıcının yetkisiyle çalışan yerel programlardır; atölye
  denemesini reddetmek bir şey korumaz.
- **Yönergeler:** iki aracın ayrı bir izin olduğu, sunulmuyorsa denemeyi kullanıcıya bırakması söylenir.
- **[PY-AI-13]'ün kalan cümlesi:** "bağımlılık kurma isteği veri gönderme onayı sayılmaz": eklenti ortamının
  hazırlanması (kurulum incelemesi, ön izleme) ile yapay zekâya veri izni (§103, ADR 8056 §7) ayrı kararlardır; hiçbir
  kurma adımı veri izni vermez.
- **Sınamalar:** ilke 1 (kapalı, okunamayan, yazılmamış); masaüstü 3 (sahte yerel sağlayıcı: varsayılanda deneme çağrısı
  onay bekler ve kart nedenini söyler, reddedilince çalışmaz; "Sormadan" ile hemen çalışır; "Çalıştırmasın" ile iki araç
  sağlayıcıya sunulan listeden düşer, atölyenin diğer araçları kalır, doğrudan çağrı `python_refused`; ilke her ajanda
  kapatır); onay sınaması iki araçla genişledi.
- **Gerçek pencere:** yerel sahte sağlayıcı (127.0.0.1, gerçek bir yapay zekâya bir şey gitmez) ajanın Vitrin'i
  denemesini istedi: kart "Onay gerekiyor" ve kodun kimin yetkisiyle çalışacağını yazdı; Reddet'le "Reddedildi", kod
  çalışmadı. Ajan formunda "Kod çalıştırma" bölümü.

## §105. İskelet önerilen paketleri verir: ajan paket adı uydurmaz (`ADIM 21`, D18)

**16 Kasım.** Kod: `python_names.rs` (`SUGGESTED`, `suggested`); headless `agent_tools.rs` (`Host::allowed_packages`,
`extension.blueprint`'in yanıtı); masaüstü `assistant/host.rs`; komut satırı `main.rs` (`kek new`).

- **Sorun.** Plan (ADIM 21 › Derslerden › Bağımlılık, D18): "`blueprint` izinli ve yaygın paketleri önerir; `verify`
  dizinde olmayan, çok yeni ya da adı popüler pakete benzeyen bağımlılığı işaretler". Denetim yarısı vardı: kilitte
  olmayan bağımlılık `python_lock_stale` (uydurulmuş ad kilitlenemez: uv "dizinde bulunamadı" der), tanınmış bir pakete
  bir harf uzak ad `python_name_resembles` (§81), yeni paket ajanın kilidindeki soğuma süresi `python_lock_no_cooling`
  (§83) ve ilkenin en düşük yaşı (§85). Öneri yarısı yoktu: iskelet ajana hangi paketin adını yazabileceğini
  söylemiyordu; modeller paket adını yanıtlarının en az %5,2'sinde uyduruyor (D18).
- **Karar.** `extension.blueprint` yanıtı `packages` taşır: sık kullanılan 13 paket ve her birinin ne işe yaradığı
  (shapely, pyproj, numpy, pandas, scipy, networkx, pyogrio, rasterio, openpyxl, python-docx, reportlab, matplotlib,
  httpx; hepsi tanınmış adlar listesinde, sınamayla) ve bir not: listede olmayan paketin adını uydurmamak, kullanıcıya
  sormak ya da `uv add` ile eklemek, kilit çözülmezse adın yanlış olduğu.
- **Kurum ilkesi önce gelir:** ilke `allowedPythonPackages` yazmışsa (yalnız bu paketler kurulabilir) iskelet yalnız
  onları önerir: listedeki öneriler önce, ilkenin adlandırdığı öbürleri sonra; not "başkasını eklemeyin" der. İlkenin
  girdisi bir gereksinim olabilir (`numpy>=2`): adı alınır. Masaüstünde ilke kayıt defterinden okunur
  (`Host::allowed_packages`); başka bir host (MCP) yazmadıkça ilke yoktur.
- **İnsan için de:** `kentos kek new` iskeleti yazınca sık kullanılan paketlerin adlarını söyler.
- **Sınamalar:** `python_names` 1 (öneriler tanınmış adlar, her biri bir kez ve amacıyla; ilke listesi süzer ve ekler,
  gereksinimden ad); headless 2 (iskelet yanıtında paketler ve not; ilkeli host yalnız izinlileri önerir).

## §106. Eklentinin çevirisi manifeste, panellere ve formlara bağlandı (`ADIM 22`, "Derslerden")

**16 Kasım.** Kod: `l10n.rs` (yeni), `archive.rs` (Kapı 1), `sdk_pack.rs` (rehber, kodlar), `sdk/llms.txt`; kayıt
defteri `registry.rs` (`read_manifest`, `texts`, `localized_file`); araçlar `plan.rs`; masaüstü `extensions/` (kurulum
incelemesi, panel, form, atölye ön izlemesi).

- **Sorun.** Plan: "`l10n` anahtarları panel ve form metinlerine bağlanır"; paket düzeni `l10n/<dil>.json`'u "manifest,
  şerit, panel ve form metinlerinin çevirileri" diye tanımlar. Manifestin `l10n` alanı (varsayılan dil, dil → dosya)
  denetleniyordu ama dosyalar hiçbir metne uygulanmıyordu: İngilizce yazılıp Türkçeye çevrilmiş bir eklenti KentOS'ta
  İngilizce görünürdü.
- **Biçim MEBİS'in dosyasıdır.** Referans paketin `l10n/en.json`'u plan aşamasında yazılmıştı: düz bir nesne, anahtar
  metnin yeri (`"entrypoints.commands.init-schema.title"`, `"contributions.ribbonTabs.mebis.label"`), değer çevirisi.
  Karar onu izler; önce düşündüğüm "kaynak metin anahtardır" biçimi bırakıldı: aynı metin iki yerde iki çeviri
  isteyebilir, metin değişince çeviri sessizce düşerdi. Yer: nesnenin anahtarı; listenin öğesi `id`'siyle, yoksa `name`,
  yoksa `value` (seçeneğin), yoksa sırasıyla. Manifestin yerleri olduğu gibi; panelinkiler `ui.<panel>`, İşlemler
  aracınınkiler `tools.<araç>`, formunkiler `forms.<form>`, katman şemasınınkiler `schema` ile başlar, yerel
  kimliklerle.
- **Metinler** kullanıcının okuduklarıdır: dosyanın kökündeki `name` (manifestin, form paketinin) ve nerede olursa olsun
  `label`, `title`, `summary`, `description`, `help`, `text`, `reason`, `alias`, `placeholder`, `tooltip`. Kimlik,
  komut, değer, ifade hiçbiri değildir (kökün altındaki `name` şema alanının kimliğidir).
- **Ne zaman:** KentOS'un arayüzü Türkçe. Başka dilde yazılıp Türkçeye çevrilmiş eklenti Türkçe görünür; Türkçe yazılmış
  eklenti yazıldığı gibi görünür, başka dillere çevirisi arayüz o dili konuştuğunda kullanılır.
- **Nerede:** kayıt defteri manifesti çevirerek okur (ad, özet, şerit, menüler, yetenek gerekçeleri: hepsi aynı
  manifestten); kurulum incelemesi paketin dosyasından çevirir (ad, özet, açıklama, inceleme satırlarındaki yetenek
  gerekçeleri); panel, form ve İşlemler aracı tanımı açılırken `Registry::localized_file` ve `texts` ile çevrilir;
  atölyenin ön izlemesi (`extension.workshop.preview`) kullanıcının gördüğünü söyler.
- **Kapı 1:** biçimi bozuk çeviri ret (`l10n_file`); yerinde metin olmayan çeviri uyarı (`l10n_stale`: metin yer
  değiştirmiş, çeviri geride kalmış). Çevrilmemiş metin hata değildir: yazıldığı gibi gösterilir ve çeviri adım adım
  büyür (MEBİS'in İngilizcesi 57 metnin 4'ünü çevirir, dördünün yeri de yerinde: sınamayla).
- **SDK:** rehberde biçim tek madde; doğrulama kodlarında iki kod.
- **Sınır:** katman şemasını eklentinin Python'u kurar (`ctx.apply_declarative_schema`, §23); şemadaki alan adlarının
  (`alias`) çevirisi orada uygulanmadı.
- **Gerçek pencere:** İngilizce yazılmış örnek eklenti (`Grave lookup`, `l10n/tr.json`), bağlı ajanın açtığı kurulum
  incelemesinde "Mezar arama", Türkçe özet, açıklama ve yetenek gerekçeleriyle; kurulunca şeritte "Mezarlar" sekmesi,
  "Arama" grubu, "Katmanları listele"; panelde "Mezar arama", "Mezar no", "Durum: Dolu/Boş", "Yalnız bu ada"; Python
  uyarısı eklentinin çevrilmiş adıyla. `kek verify` 0 uyarı; kuru koşu katkıları Türkçe adlarıyla söyler.
- **Sınamalar:** `l10n` 4 (yerler; kimlik, komut, değer ve ifade çevrilmez; boş çeviri yok sayılır; form paketinin adı,
  şema alanı; dosya biçimi; Kapı 1'in uyarısı; MEBİS'in İngilizcesinin her yeri yerinde); kayıt defteri 1 (İngilizce
  yazılmış paketin manifesti ve paneli Türkçe, başka panelin yerleri karışmaz; Türkçe yazılmış paket yazıldığı gibi);
  masaüstü 1 (inceleme: ad, özet, gerekçe).

## §107. Kataloğa gönder: geliştirilen eklenti klasöründen kataloğa, incelemeye (`ADIM 21`, Akış, EKL-06)

**16–17 Kasım.** Kod: bulut istemcisi `api/extension_catalog.rs` (yeni: `upload_release`, `submit_release`),
`tests/extension_catalog.rs`; masaüstü `extensions/submit.rs` (yeni), `view.rs` (düğme), `mod.rs`, `shell/view.rs`,
`tests/submit.rs`; ajan araçları `agent_tools.rs` (`extension.workshop.submit`, `Host::submit_extension`),
`link_workshop.rs` (`link_submit`); MCP `tools.rs`; masaüstü bağlantısı `python/agents.rs`, Asistan `host.rs`.

- **Sorun.** Akışın son adımı "istenirse yayına gönderme" yoktu: sunucunun katalog uçları (ADR 8059 §10, §13) yükleme ve
  incelemeye göndermeyi programlara açıyordu, ama masaüstünde onları kullanan bir istemci yoktu. Eklentiyi atölyede
  geliştiren kişi ya da ajan, kataloğa ancak web'den elle yükleyerek ulaşabiliyordu.
- **Karar.** Çalışma klasöründen kurulmuş eklentinin ayrıntısında "Kataloğa gönder…": katalog (hesabın kişisel kataloğu,
  kurumlarının katalogları, genel katalog) ve inceleyiciye isteğe bağlı bir not; Gönder klasörü `kentos kek pack` gibi
  paketler (kapılardan geçmeyen gönderilmez, ilk sorunu söylenir), kataloğa taslak olarak yükler (`POST
  /v1/extension-catalogs/{katalog}/releases`, baytlar olduğu gibi) ve incelemeye gönderir (`…/moves`, `{"action":
  "submit", "note": …}`). Sonuç bildirimle ve eklentinin günlüğünde (`release_submitted`: ne, nereye, hangi durumda).
- **Masaüstü yalnız gönderir:** istekler program kanalından gider (`x-kentos-client: desktop`); onay, değişiklik isteme
  ve yayımlama kişinin web penceresindeki kararıdır (ADR 8059, `decision_by_program`). Hangi kataloğa gönderebileceğini
  sunucu söyler: yetkisizse gönderim sunucunun sözüyle reddedilir.
- **Oturum yoksa** pencere söyler ve "Oturum aç…" düğmesini verir; Gönder kapalıdır.
- **Ajan:** `extension.workshop.submit` kullanıcının atölyeden kurduğu eklenti için pencereyi açar ("{ajan} bu eklentiyi
  kataloğa göndermek istiyor. Göndermek sizin kararınız."); kataloğu kullanıcı seçer ve gönderir. Ajan hiçbir şey
  göndermez (PY-AI-05). MCP'nin bağlı ajanı da masaüstünün `submit_extension` isteğiyle aynı pencereyi açar; masaüstüne
  bağlı olmayan bir sunucu çizimi `desktop.attach`'i söyler.
- **Sınır:** dosyadan kurulmuş eklenti buradan gönderilmez (geliştirildiği klasörü yoktur); söylenir.
- **Gerçek pencere:** atölyeden kurulan örnek eklentinin ayrıntısında "Kataloğa gönder…"; pencere oturum yokken uyarı ve
  "Oturum aç…" ile. Oturumlu akış sahte katalog sunucusuyla sınandı (yerelde çalışan bir KentOS sunucusu ve hesabı
  kurulmadı).
- **Sınamalar:** bulut 2 (yükleme yolu ve baytları, program başlığı, gönderme hareketi ve notu; sunucunun reddi
  sözüyle); masaüstü 2 (oturumsuz uyarı; kurum kataloğuna notla gönderim: paketin kimliği, iki istek, günlük kaydı,
  pencere kapanır; dosyadan kurulmuş eklentide ajanın isteği nedeniyle reddedilir); headless 1 (araç atölyeyle sunulur,
  pencere açamayan host yerini söyler).

## §108. Ajan isteği bir modelle zincirler: satır içi model, kısa araç dizini, çıktı adları (Üretim, PY-AI-08)

**17 Kasım.** Kod: komut `gis/cad_processing.rs` (`ProcessingRun.model`, yeni `ProcessingToolOutput`,
`ProcessingTool.outputs`); İşlemler `command.rs` (`inline_model`, `described` çıktılarla), `tests/inline_model.rs`
(yeni); çekirdek `core/call.rs` (`fresh_layers`), alan `etag.rs` (`layers_plan_stamp(fresh)`, `plan_plain`); headless
`agent_tools.rs` (`processing.list` girdisi, `processing.run` açıklaması, çağrı sözleri),
`agent_tools/processing_list.rs` (yeni), `veil.rs` (`NAMING`, modelin verilen değeri, `refusal_at`, `read_by`, `among`),
`veil/reads.rs` (yeni), `dispatch.rs`, `tests/headless/processing_model.rs` (yeni); masaüstü Asistan yönergesi,
`assistant/model_tests.rs` (yeni); TS bağları, komut kataloğu, Python SDK katmanı ve `cad.processing.tools` durumları
(`scripts/fixtures/processing_command_cases.py`, araçlar çıktılarıyla) yeniden üretildi.

- **Sorun.** "Doğal dil analizleri yerleşik ve eklenti araçlarını aynı plan/model üzerinden zincirler" maddesinde ajanın
  elinde yalnız tek araç çalıştırmak vardı: "parselleri tamponla, sonra birleştir" iki ayrı çalıştırma, iki onay, iki
  geri alma adımıydı. Kayıtlı bir modeli ajan yazamıyordu (modeller kitaplıktadır, kaydetmek kullanıcınındır).
- **Karar: satır içi model.** `cad.processing.run`'a `model` alanı: kitaplığın modeliyle aynı biçim (`id`, `label`,
  `inputs`, `steps[{id, tool, values}]`, `outputs`; değerler `{kind: input|output|value}`), kaydedilmeden kayıtlı model
  gibi çalışır: tek plan, tek onay, tek geri alma adımı (adı modelin adı). Okunmayan model `invalid_parameters`
  (`model`), bu ortamda olmayan araç `unknown_tool` (`model.steps[i].tool`), tutmayan bağlantı `invalid_parameters`
  (`model.steps[i]`, "adım “…”"); hiçbir şey yazılmadan. `tool` okunmaz (`"model:"` yazılır). Boş liste yerine boş nesne
  (`"inputs": {}`) liste okunur: ajan öyle yazdı ve zinciri bırakıp iki ayrı çalıştırmaya döndü.
- **Gerçek pencerede bulunan dört engel** (DeepSeek, sentetik Vitrin çizimi; istek "Parsel sınırı katmanındaki
  parsellerin 3 metrelik tamponlarını çiz, sonra tamponları birleştirip tek alan yap"):
  1. **Liste bütçeyi yiyordu.** `processing.list` her aracı bütün parametreleriyle veriyordu: 157 araç, **274 957
     bayt**; ajan listeyi okuyup iki aracı tanımladıktan sonra 80 000 tokenlık sohbet sınırında (90 900) hiçbir şey
     çalıştıramadan durdu. Liste artık dizin: kimlik, ad, kategori, açıklamanın ilk cümlesi, eklenti aracının modele
     sözü ve eklentisi; **35 102 bayt** (7,8 kat küçük). `query`/`category`/`models` süzer; birden çok sözcük tek araçta
     buluşmazsa ("tampon birleştir") her sözcüğün araçları birleşik verilir. Parametreleri `processing.describe` verir.
  2. **Çıktı adları yoktu.** `processing.describe` "parametreleri ve çıktıları" diyordu ama çıktı vermiyordu; ajan
     bağlantı adını tahmin etti ("buffer çıktısının türü yüzünden tutmadı") ve iki adıma döndü. `ProcessingTool.outputs`
     (`name`, `label`, `kind`: features/number/string/table) eklendi.
  3. **Plan hiç yürütülmüyordu (`plan_changed`).** İki ayrı eski hata, ikisi de modelden bağımsız: (a) aracın yeni
     katmanı bir grubun içindeyse (Vitrin'de "Kadastro") plan damgası yalnız ağacın ilk düzeyini temizliyordu,
     türetmenin çalışma anı (`at`) plan kimliğine giriyordu: **gruplu bir çizimde tek bir tampon bile planlandığı gibi
     yazılmıyordu** (sınamada özyinelemesiz sürüm bu yüzden kırmızı); (b) zincirde sonraki adımın katmanı önceki adımın
     yeni nesnelerinin özetini (digest) taşıyor, nesneler her an yeni kimlikle doğduğu için özet değişiyordu. Damga
     artık grupların içine iner; bu yazmanın nesne eklediği ya da eklediği katmanı okuyan türetme girdisinin özeti
     damgaya girmez (o nesneler planın sonucunda sırayla durur, kaynakları planın revizyonudur).
  4. **Gizli alan denetimi kimlikleri alan sanıyordu.** Ajan değerleri göndermeyen ajanda (§103) her öznitelik adı
     gizlidir; model kimliği `parsel-tampon-birlesik` ifade olarak "parsel eksi tampon eksi birlesik" okunup `parsel`
     alanını andığı için reddedildi; aynı yüzden `"layerId": "parsel"` (katman kimliği alan adıyla aynı) her çağrıda
     reddedilirdi. Bir şeyi adlandıran anahtarların (`id`, `layerId`, `layer`, `kind`, `type`, `scope`, `command`,
     `tool`, `step`, `output`, `code`, `rule`, `version`, `revision`, `planId`, `newName`: `NAMING`) metni alan okuması
     sayılmaz; anahtarın kendisi yine denetlenir.
- **Modelden gizli alan kaçmaz.** Modelin adım değerleri `values` altında durur ve her değer `{kind: value, value: …}`
  biçimindedir; ikisi de "yazılan değer" (`WRITTEN`) sayıldığı için zincir gizli alan denetiminin dışında kalırdı
  (`group: "parsel"` ile gruplamak, süzgeçle okumak). Bir adımın değerleri artık parametresinin doğrudan verilmiş hâli
  gibi denetlenir. Ayrıca İşlemler araçlarının **ifade ve alan parametreleri türlerine göre** okunur (`veil/reads.rs`):
  Hesapla'nın ifadesi `value` adıyla durduğu için doğrudan çağrıda da denetlenmiyordu (`value: "upper(Malik)"` gizli
  alanı başka bir alana kopyalayabilirdi); toplanacak alanlar virgüllü yazılınca (`"alan, Malik"`) eşitlik denetimi
  kaçırıyordu. Çalıştırma, toplu çalıştırmanın satırları ve yazılan modelin adımları aynı kuralla reddedilir. Kayıtlı
  bir modelin adımları sahibinindir, ajanın değil; denetlenmez.
- **Ajana söylenen:** `processing.run` açıklaması modelin biçimini (listeler `[]`), çıktı adlarının yerini ve `"model:"`
  yazılacağını söyler; Asistan yönergesine "Birkaç aracı zincirlemek için adımlarını input.model'e yazdığın bir model
  ver" eklendi. Onay kartı modeli adıyla ve adım sayısıyla yazar ("İşlem aracını çalıştır · model “Tampon ve birleştir”,
  2 adım · plan").
- **Gerçek pencere (son koşu):** ajan dizinden iki aracı buldu, tanımlarından çıktı adını okudu, modeli yazıp planladı;
  tek onayla 4 tampon ve birleşik alan tek geri alma adımında yazıldı ("Parsel tamponu ve birleştir"). Sohbet 164 832
  token. Aynı istek bir koşuda zincir yerine iki ayrı çalıştırmayla da bitti (sonuç 4 343,38 m²): ajanın seçimi.
- **Kalan:** ajan sonuç katmanlarını kimlikleriyle anıyor (`islem-tampon`; çalıştırmanın çıktısı katman adı taşımıyor);
  her istekte gönderilen araç tanımları sohbetin tokenını hızla büyütüyor (ölçülmedi). Yeni katman kimliği kurucusu
  sonda tire bırakabiliyor (`islem-parsel-tamponu-birlesik-`); web'le eşdeğerlik gereği dokunulmadı.
- **Sınamalar:** İşlemler 2 (zincir tek geri alma adımında; reddedilen modeller yerleriyle, hiçbir şey yazılmadan; boş
  nesne liste okunur); headless 4 (tek araç ve model plan → yürütme; gruplu Vitrin çiziminde tek araç ve ajanın yazdığı
  model; dizin < 48 000 bayt, `query`, iki sözcüğün araçları, eşleşmeyenin notu; tanım çıktıları); örtü 2 (kimlikler
  alan sayılmaz, süzgeç ve adım değeri sayılır; ifade ve alan parametreleri, satırlar, adımlar); masaüstü 1 (Asistan'ın
  kapısından plan → yürütme); çağrı sözleri.

## §109. Ortak keşif kataloğu: komut, araç, eklenti ve yanıt kodu tek aramada (Üretim, PY-AI-01)

**17 Kasım.** Kod: headless `agent_tools.rs` (`catalog.search`'ün `kinds`'ı ve `others`'ı, `catalog.describe`'ın
`extension` ve `code`'u, çağrı sözleri), `agent_tools/discover.rs` (yeni); masaüstü `extensions/catalog_agents.rs`
(yeni: bağlantının `extensions` isteği), `python/agents.rs`, Asistan yönergesi;
`kentos_extensions::declared_requirements` (`python.rs`); sınamalar `tests/headless/agent_tools.rs`,
`extensions/tests/catalog.rs` (yeni), `python/tests.rs`.

- **Sorun.** PY-AI-01 "eklenti/araç/form/ortam keşfi, yetenekler, şemalar, bağımlılıklar, örnekler ve hata kodları ortak
  katalogda" der. Ajanın keşfi dağınıktı: komutlar `catalog.search`'te, İşlemler araçları `processing.list`'te, eklenti
  denetimleri `audit.list_providers`'ta; kurulu bir eklentinin ne istediği, hangi Python paketlerine dayandığı,
  izinlerinin verilip verilmediği ve parçalarının nasıl kullanılacağı hiçbir yerde yoktu. Yanıt kodlarının anlamı (1 547
  kod, `codes::CODES`) ajana açık değildi: `plan_changed` aldığında yeniden denenip denenmeyeceğini iletiden tahmin
  ediyordu.
- **Karar: `catalog.search` ortak katalogdur.** Komutların yanında `others`: İşlemler araçları (yerleşik ve
  eklentilerin, sözcük başına `cad.processing.tools`), kurulu eklentiler ve parçaları (denetim kuralı, panel, form,
  ifade işlevi, harita aracı, veri kaynağı, eklenti komutu) ve yanıt kodları; her sonucun `kind`'ı, kısa sözü ve
  `next`'i (sıradaki çağrı: `processing.describe`, `audit.run`, `catalog.describe (extension)` …). En çok sözcüğü tutan
  önce; bir eklentinin aracı bir kez, araç olarak. `kinds` aramayı daraltır (`command`, `tool`, `extension`, `code`);
  verilmezse komut, araç ve eklenti; yanıt kodları kod gibi yazılan sözcükle (`plan_changed`) ya da adıyla istenince
  aranır (bin beş yüz kodun anlamında "plan" geçenler her aramaya karışmasın). Yanıtın eski alanları (`commands`,
  `total`) aynı.
- **`catalog.describe`** bir eklentiyi (`extension`) ve bir kodu (`code`) da tanımlar. Eklenti: ad, sürüm, özet,
  yayımcı, anahtar sözcükler, durum, güven, kaynak; istenen yetenekler kurulum penceresinin Türkçe başlığı, gerekçesi ve
  verilip verilmediğiyle; Python: projenin bildirdiği gereksinimler (PEP 508, yazıldığı gibi), Python'a izin, ortamın
  hazır olup olmadığı ve nedeni; parçalar türü, kimliği, adı, Python isteyip istemediği ve ajanın nasıl kullanacağıyla
  (şeridin sekmesi ve sağ tık öğesi arayüzün kendisidir, listelenmez); son sorunu. Kod: anlamı, nasıl döndüğü (ret,
  çakışma, uyarı …), yeniden denenebilirliği ve ne yapılacağı.
- **Eklentiler masaüstündedir:** liste masaüstünün `extensions` isteğinden gelir (Asistan'ın ve masaüstü bağlantısındaki
  ajanın kapısı aynı). Sunucunun kendi çizimindeki ajan eklentiyi adıyla ararsa yanıt bunu söyler (`notes`);
  tanımlatırsa `not_offered`.
- **Gerçek pencere** (DeepSeek, sentetik Vitrin): "Kurulu Vitrin eklentisi neler sunuyor, hangi Python paketlerine
  ihtiyaç duyuyor ve izinleri verilmiş mi?" iki çağrıyla (`catalog.search` kinds extension, `catalog.describe`
  extension) doğru yanıtlandı: parçaları türleriyle, `shapely>=2.0,<2.1` ve `numpy>=2.0,<3`, 12 yeteneğin hepsi
  verilmiş, ortam hazır; sohbet 37 595 token. İlk koşuda ajan yeteneğin kimliğinden `audit.provider`'ı "eğitim" diye
  çevirdi: yeteneklere kurulum penceresinin başlığı ("Denetim ekler") eklendi, ikinci koşuda doğru yazdı.
- **Sınamalar:** headless 1 (araç `next`'iyle bulunur; kod kod gibi yazılınca ilk sırada ve tanımı: tür, yeniden
  denenebilirlik, anlam; bilinmeyen kod; yalnız kodlar; eklentiler bu ortamda yok diye söylenir, tanımı `not_offered`;
  boş tanım isteği; çağrı sözü); masaüstü 1 (Vitrin kurulu: gereksinimler, yetenekler başlık ve izinle, parçalar ve
  kullanımları, arayüz parçaları dışarıda; Asistan'ın kapısından arama ve tanım, bilinmeyen eklenti); eklenti paketi 1
  (gereksinimler yazıldığı gibi, boş ve okunmayan proje).

## §110. Hata ayıklama raporu: ortam, kaynak sürümleri, şemalar, kodun anlamı; anahtarsız (API ailesi, ECO-API-14)

**17 Kasım.** Kod: masaüstü `extensions/report.rs` (yeni: rapor `problems.rs`'ten taşındı ve genişledi),
`python/agents.rs` (bağlantının `extension_report` isteği); headless `agent_tools.rs` (`extension.report`),
`agent_tools/discover.rs` (`report`); eklenti paketi `secrets.rs` (`masked`, sırların yerleri); sınamalar
`extensions/tests/report.rs` (yeni), `secrets.rs`'in sınaması.

- **Sorun.** ECO-API-14 "hata ayıklama raporu sır içermeden ortam özeti, kaynak sürümleri, şemalar ve hata kodunu
  toplar" der. Eklentiler'in Raporu kopyala'sı kimliği, durumu, kaynağı, programın sürümünü ve son kayıtları veriyordu
  ama Python'u, uv'yi, eklentinin ortamını ve gereksinimlerini, paket özetini, yazıldığı şemayı ve protokolü, kodun
  anlamını vermiyordu; eklentinin iletisi ya da Python'un izi bir anahtar ve kullanıcının ev klasörü taşıyabiliyordu ve
  rapor bunları olduğu gibi kopyalıyordu. Ajan raporu okuyamıyordu: kullanıcının kopyalayıp yapıştırması gerekiyordu.
- **Karar: rapor dört bölüm.** Kimlik: ad, durum, yayımcı, kaynak, güven, bu bilgisayardaki sürümler, **paket özeti**
  (`sha256:…`), **manifest** (`kentos.extension` şeması, eklenti protokolü aralığı ve bu KentOS'unki, KentOS sürüm
  aralığı), katman şeması dosyası, günlük dosyası. **Ortam:** program ve sürümü, sistem ve mimari, KentOS'un Python'u
  (sürümü ya da neden hazır olmadığı), uv (sürümü, KentOS'la gelip gelmediği ya da neden yok), eklentinin Python
  gereksinimleri (projesinin yazdığı gibi) ve ortamının hazır olup olmadığı, yetenekler (kaç istendi, kaç verildi,
  verilmeyenler kurulum penceresinin sözleriyle). **Sorun:** duran hata, kodu ve kodun anlamı (yanıt kodları
  tablosundan), Python'un izi. **Son kayıtlar.**
- **Sır yok:** raporun tamamı sır tarayıcısından geçer (`secrets::masked`, paketlemenin ve kataloğun aynı biçim
  kuralları): sağlayıcı anahtarı, adresteki parola, özel anahtar zırhı bütün hâlinde `‹… gizlendi›` olur; ev klasörü `~`
  yazılır. Tarayıcı artık yalnız satırı değil gizlenecek aralığı da bilir (adreste yalnız parola, zırhta END satırına
  dek).
- **Ajan:** `extension.report` (yalnız okur) aynı raporu verir; Asistan'ın ve masaüstü bağlantısındaki ajanın kapısı
  aynı (`extension_report`). Rapordaki sözler eklentinindir: ajanın örtüsünden geçer (eklentinin denetim bulguları gibi,
  §103). Sunucunun kendi çiziminde `not_offered`.
- **Gerçek pencere** (DeepSeek, sentetik Vitrin): "Vitrin eklentisinin raporunu oku, ortamı hazır mı ve duran bir hatası
  var mı?" `catalog.search` ve `extension.report` ile doğru yanıtlandı (Python 3.12.13, ortam hazır, gereksinimler, 12
  yeteneğin 12'si verilmiş, duran hata yok; 37 133 token). Sınama sürücüsü `:` karakterini sohbete yazamıyor (tuş olayı
  yazılmadan genel kısayola düşüyor, Birleşim penceresi açıldı); istekler `:` içermeden yazıldı.
- **Sınamalar:** masaüstü 1 (Vitrin kurulu, iletisinde anahtar ve izinde ev klasörü olan bir hata: bölümler, paket
  özeti, manifest şeması, gereksinimler, yetenekler, kodun anlamı; anahtar ve ev klasörü raporda yok; Asistan'ın
  kapısından aynı rapor, bilinmeyen eklenti); eklenti paketi 1 (anahtar, adresteki parola, özel anahtar zırhı gizlenir,
  düz metin olduğu gibi kalır).

## §111. Komut geçmişinin Python'u ve yeniden oynatması anahtar taşımaz (API ailesi, ECO-API-13)

**17 Kasım.** Kod: masaüstü `cad/history/mod.rs` (`without_secrets`; `python_of`, `python_script_of_history`,
`replay_of`), `cad/history/tests.rs`.

- **Durum.** ECO-API-13 "anlamsal eylem kaydedici izinli işlemi düzenlenebilir Python örneğine çevirir, sır ve hassas
  alan kayda girmez" der. Kaydedici vardı (ADR 0284: Komut geçmişi her komut çağrısını SDK'nın aynı çağrısıyla Python'a
  yazar, bütün geçmiş tek betik, tek çağrı yeniden oynatma girdisi) ve hassas alan girmiyordu (ADR 0443 §2: formda
  gizlenen ve katmanda saklanan alanların değerleri `‹gizli›`). Sırlar tasarım gereği komut girdisinde durmaz
  (bağlantı sırları cihazda, PostGIS adresi parolasız), ama bir çağrının girdisi bir anahtarı yine de taşıyabilir: bir
  servis adresi sorgusunda anahtarıyla yazılmış (`?k=…`), bir nota yapıştırılmış.
- **Karar:** üç çıktı da (çağrının Python'u, geçmişin betiği, yeniden oynatma) girdinin her metnini sır tarayıcısından
  geçirir (`secrets::masked`, §110): sağlayıcı anahtarı, adresteki parola, özel anahtar zırhı `‹… gizlendi›` olur. Her
  metin kendi başına taranır: gizlenen anahtar etrafındaki tırnağı ve JSON'u almaz, yeniden oynatma JSON olarak okunur.
- **Sınama:** masaüstü 1 (girdisine anahtarlı bir servis adresi eklenmiş çağrının Python'unda ve betikte anahtar yok,
  gizlendiği yazılı; iç içe bir girdide anahtar ve zırh kendi metinlerinde gizlenir, yapı aynı).

## §112. İskeletin taşınabilir form şablonu (API ailesi, ECO-API-10)

**17 Kasım.** Kod: eklenti paketi `blueprint.rs` (`Contribution::Form`, `form_package`, `FORM_FILE`); headless
`agent_tools.rs` (`extension.blueprint`'in katkıları), CLI `kentos kek new --with … form`; sınamalar `blueprint.rs`'in
sınaması (her katkı, tek başına ve hepsi birden), masaüstü `extensions/tests/demo.rs`.

- **Sorun.** ECO-API-10 "SDK şablonları (GUI, İşlemler, taşınabilir form, birleşik paket)" der. İskelet panel (GUI),
  İşlemler aracı, denetim, ifade işlevi, harita aracı, katman şeması ve komut veriyordu, hepsi bir arada da (birleşik
  paket); taşınabilir formun şablonu yoktu: formlu bir eklenti yazan ajan `.kform`'u, şemanın katmanına bağlanmasını ve
  manifestteki bildirimini sıfırdan kurmak zorundaydı.
- **Karar:** `form` katkısı `forms/alan.kform` yazar (`kentos.form-package` 1): şemanın katmanının iki alanı, ad
  zorunlu, durum şemanın değer listesinden; şemanın katmanı formu `form` ile anar, manifest `entrypoints.forms`'ta
  bildirir; kurulum formu katmana adıyla bağlar. Form, nesnelerini açacağı katmanı olmadan anlamsızdır: tek başına
  istenince şemayı da getirir. İskeletin sözü değişmedi: her katkı tek başına ve hepsi birden doğrulamadan sorunsuz ve
  uyarısız geçer (formun ön denetimi dahil).
- **Gerçek kullanım:** `kentos kek new … --with form` 11 dosya yazdı; `kentos kek verify` 0 uyarıyla geçti, kuru koşu 3
  katkıyı bağladı (form, sekme, Katmanları kur). Masaüstünde her katkılı iskelet kuruldu; formu ajanların ortak
  kataloğunda `form` olarak görünür (`ext.<eklenti>.alan`).

## §113. Uyumluluk matrisi keşfedilebilir: SDK'da bir bölüm, ortak katalogda eklenti başına (API ailesi, ECO-API-11)

**17 Kasım.** Kod: eklenti paketi `sdk_pack.rs` (`compatibility` bölümü, ana pakette yeri), `python/builds.rs`
(`PYTHON_MINORS`, `Machine::platform_machine` açıldı); masaüstü `extensions/catalog_agents.rs` (eklentinin
`compatibility`'si); SDK paketi (`sdk/llms.txt`) yeniden üretildi; sınamalar `sdk_pack.rs`,
`extensions/tests/catalog.rs`.

- **Sorun.** ECO-API-11 "uyumluluk/ABI/eskime ve çalışma zamanı matrisi keşfedilebilir" der. Bilgi vardı ama dağınıktı
  ve okunamıyordu: eklenti protokolü `compat.rs`'te, Python aralığı, kurulan Python ve bilgisayarlar
  `python-env/compat.json`'da ve `python/builds.rs`'in sabitlerinde, eskiyen ve deneysel adlar `sdk_surface.rs`'te,
  çalışma zamanları manifestte ve araç tanımında. Ajan ya da yazar "bu eklenti hangi KentOS'ta, hangi bilgisayarda,
  hangi Python'la, nerede çalışır" sorusunu kapıdan geri çevrilerek öğreniyordu.
- **Karar: SDK'nın `compatibility` bölümü** (`kentos kek sdk --part compatibility`, `extension.sdk part=compatibility`):
  bu KentOS'un eklenti protokolü, manifestin uyumluluk alanlarının anlamı ve kararlılık kuralı (ADR 8061); Python
  aralığı (`>=3.10,<3.15`) ve kurulan sürüm (3.12); altı bilgisayar ve kilidin onları andığı işaretler (`sys_platform ==
  'win32' and platform_machine == 'AMD64'` …, tekerleklerin ABI'si); çalışma zamanları (masaüstü her katkıyı, başsız
  yalnız `headless-python` bildiren İşlemler araçlarını çalıştırır; bulut bildirilebilir, eklenti kodu orada henüz
  çalışmaz) ve eskiyen/deneysel adlar (bugün boş). Kapıların kullandığı sabitlerden okunur: kapılardan başka bir şey
  söyleyemez; sınama Python tarafıyla (`compat.json`) aynı olduğunu da denetler. Ana pakette yalnız yeri yazılı (48 000
  baytlık bütçe).
- **Eklenti başına:** ortak katalogdaki tanım (`catalog.describe` extension, §109) `compatibility`'yi bu bilgisayara
  göre verir: protokol aralığı ve buradaki, KentOS aralığı ve buradaki sürüm, sistemler, çalışma zamanları, burada
  çalışıp çalışmadığı ve nedeni, eskime notu ve yerine geçen.
- **Sınamalar:** eklenti paketi 1 (bölüm protokolü, Python'u ve bilgisayarları `compat.json`'la aynı söyler; işaret
  biçimi; çalışma zamanları; boş eskime listesi; bölüm listede); masaüstü (Vitrin burada çalışır, protokolü, masaüstü
  çalışma zamanı).

## §114. Keşif sayfalanır (API ailesi, ECO-API-07)

**17 Kasım.** Kod: headless `agent_tools.rs` (`catalog.search`'ün `offset`'i ve `next`'i), `agent_tools/discover.rs`
(`others`'ın penceresi), `agent_tools/processing_list.rs` (`limit`, `offset`, `total`, `next`); sınamalar
`tests/headless/agent_tools.rs`.

- **Sorun.** ECO-API-07 "keşif sayfalama ve izdüşüm kullanır, bütün ekosistem her isteme doldurulmaz" der. İzdüşüm
  vardı (§108: araç dizini parametresiz, §109: ortak katalog satır başına kısa söz ve `next`), sayfalama yoktu:
  `catalog.search` `limit`'ten sonrasını hiç vermiyor (toplamı söyleyip ötesine geçirmiyordu), `processing.list` bütün
  araçları tek yanıtta döküyordu (157 araç, 35 102 bayt).
- **Karar:** `catalog.search` `offset` alır; komutlar ve öbürleri (`others`) aynı pencereyle sayfalanır, ikisinden
  birinde ötesi varsa yanıt `next: {offset}` taşır. `processing.list` sayfa sayfadır: varsayılan 50, en çok 200 araç;
  `total` ve `next.offset`. Bir sayfa 16 000 baytın altında; sayfalar sonuna dek okunduğunda her araç bir kez gelir.
- **Sınamalar:** headless (dizin 50 araçlık sayfa, 16 000 baytın altında; `next.offset` ile sonuna dek her araç bir
  kez, sayısı `total`; aramanın iki sayfası farklı komutlarla, aynı toplamla).

## §115. Panelin görseli fotoğrafı da gösterir: JPEG (`ADIM 22`, bileşen envanteri)

**17 Kasım.** Kod: masaüstü `extensions/panel_image.rs` (`jpeg`, `within`); SDK rehberi (`sdk_pack.rs`), panelin şeması
(`ui_panel.rs`, `sdk/kentos-ui-panel.schema.json`), Python `kentos/ext.py` ve `kentos/ui_panel.py` (belgeleri); SDK
paketi yeniden üretildi.

- **Sorun.** Bileşen envanterinin (§25) tek açık satırı görsel önizlemeydi: `image` yalnız PNG alıyordu ("JPEG sonraki
  dilim"). Kaydın fotoğrafını panelde göstermek isteyen eklenti onu PNG'ye çevirmek zorundaydı; çoğu fotoğrafta bu
  dosyayı katlarca büyütür.
- **Karar:** `image` PNG'nin yanında JPEG de alır; KentOS'un kendi çözücüsüyle (zune-jpeg, sahibin onayıyla ADR 0092),
  yeni bağımlılık yok. PNG gibi JPEG'in de boyutu çözülmeden önce çerçeve başlığından okunur: kenarı 4096'yı ya da
  toplamı 8 milyon pikseli aşan görsel okunmadan reddedilir. Başka biçim "PNG ya da JPEG olmalı" diye, bozuk JPEG
  "JPEG okunamadı" diye kodun sözlerine düşer.
- **Sınamalar:** masaüstü 1 (8 × 4 fotoğraf çizilir; başlığı 5000 × 2 diyen JPEG çözülmeden reddedilir; kırpılmış JPEG
  okunamadı der); var olan PNG sınaması başka biçimi GIF ile sınar.


## §116. İşlemler'in her kapısı aynı sonucu verir: uyum sınaması (§5.2, ECO-QA-06)

**17 Kasım.** Kod: headless `tests/headless/processing_doors.rs` (yeni).

- **İstenen.** ECO-QA-06: İşlemler'in eklenti/yerleşik/model/arayüz/Python/MCP kapıları için aynı anlık görüntüde aynı
  tipli sonucu veren uyum sınaması.
- **Yeni sınama:** aynı çizimde (`fixtures/processing/v1/parcels.kcad`) aynı tampon dört kapıdan yürütülür: programın
  çağırdığı ürün komutu (`cad.processing.run`), ajanın `processing.run`'ı (planlanır, planın kimliğiyle yürütülür), tek
  adımlı satır içi model (§108) ve tek satırlı toplu çalıştırma (`cad.processing.batch`). Her biri aynı nesneleri
  (biçim, öznitelik, katman; kimlikler dışında) yazar, tipli sayılar (`outputs.count` 3) aynıdır. İlk koşuda yeşil.
- **Öbür kapılar aynı yollardan geçer, bağımsız beklentilerle bağlı:** Python'un `kentos.cad` çağrısı başsız oturumun
  `call`'ıdır (ürün komutu kapısı), MCP'nin araçları ajan araçlarıdır (ajan kapısı); masaüstünün Asistan kapısı
  `assistant/model_tests.rs` (§108); pencere (İşlemler penceresi) ortak durum dosyalarını çalıştırır
  (`every_shared_case_runs_from_the_window_as_it_says`; aynı dosyalar ürün komutunun durumlarıdır,
  `every_command_case_matches_the_desktop_handlers`); eklentinin aracı masaüstünde (Örnekleri çalıştır), ajanın
  denemesinde ve `kentos kek test`'te tek örnek koşucusundan (`examples::run`) aynı beklentilerle geçer (§48, §74).
  Üreticinin kendi çıktısı kanıt sayılmaz (ECO-QA-15): beklentiler durum dosyalarında ve örneklerde ayrıca yazılı.

## §117. Kapanan panel geç cevabı almaz: panelin kendi numarası (`ADIM 22`, Tanım ve oturum, PY-UI-17)

**17 Kasım.** Kod: masaüstü `extensions/panels.rs` (`OpenPanel::instance`, `PanelOrigin::instance`, `answered`),
`extensions/panel_session.rs` (çalışmanın kendi paneli numarasıyla); sınamalar `extensions/tests/late.rs` (yeni), var
olan panel sınamalarının çalışma kökenleri.

- **Sorun.** PY-UI-17 "pencere kapanması, kaldırma ve belge değişimi geç geri çağırmayı etkisiz kılar" der. Bir
  çalıştırmanın cevabı ve çalışırken yaptığı `ctx.panel` çağrıları panele yalnız eklentinin ve panelin kimliğiyle
  bağlanıyordu. Kullanıcı komutu çalışan paneli kapatıp aynı paneli yeniden açınca eski çalıştırmanın satırları yeni
  açılan panelin tablosuna yazılıyor, kodun `ctx.panel.set`'i kullanıcının yeni panelde bir şey yazmadığı alanlarına
  eski değerleri koyuyordu.
- **Karar:** her açılan panelin kendi numarası olur (`instance`, açıldıkça artar); çalıştırmanın kökeni o numarayı
  taşır. Cevap ve durdurma yalnız aynı numaralı panele gider; kodun kendi paneline yaptığı çağrı, o panel kapandıysa
  "kapandı" cevabını alır (`{"applied": false, "closed": true}`, kapanmış panelin bugünkü cevabı). Kodun
  `ctx.open_panel` ile açtığı ya da adıyla andığı panel adıyla bulunur (o çalıştırmanın içinde açılır).
- **Var olan sınırlar aynı:** kaldırılan eklentinin panelleri kapanır, çağrıları reddedilir; çalıştırmanın kimliği
  tutmayan geç "bitti" iletisi yok sayılır (`runner.rs`); silinmiş ya da başka çizimdeki nesneler komuta verilmez
  (§67'nin seçici denetimi).
- **Sınamalar:** masaüstü 2 (kapanıp yeniden açılan panel eski çalıştırmanın satırlarını almaz, açık kalan alır;
  çalışan kodun kendi paneline çağrısı açıkken uygulanır, kapanıp yeniden açılınca "kapandı" cevabını alır ve yeni
  panelin değerleri değişmez).

## §118. Kurumun kendi paketleri yalnız kendi deposundan: bağımlılık karışıklığı (`ADIM 18A`, PY-UV-26)

**17 Kasım.** Kod: kayıt defteri `policy.rs` (`pinnedPackageIndexes`, `package_refusal`); sınama `tests.rs`
(`the_policy_judges_the_python_packages_a_package_brings`).

- **Sorun.** PY-UV-26 "dizin önceliği ve paket kaynağı sabitlenir; özel paket adı başka genel dizinden sessizce
  çözülmez" der. Kurum ilkesinin izinli depoları (`allowedPackageIndexes`, §84) kurumun deposunu ve genel depoyu
  birlikte izinli yazınca kurumun kendi paketinin adı (`belediye-cbs`) genel depoda da varsa kilidin onu hangisinden
  aldığını ayırt etmiyordu: aynı adla genel depoya konan paket, bağımlılık karışıklığı saldırısının yoludur.
- **Karar:** ilkenin `pinnedPackageIndexes`'i kurumun paketlerini ve her birinin tek deposunu yazar: ad ya da `*` ile
  biten önek (`"belediye-*"`) → depo. Kilit böyle bir adı başka bir yerden alıyorsa paket reddedilir; nedeni ve yazarın
  yapacağı söylenir ("paketi o depoya bağlamalı ([tool.uv.sources])"). Ad PEP 503'e göre karşılaştırılır (`Shapely` =
  `shapely`), önekin ayracı ayraç kalır (`num-*` `numpy`'yi tutmaz), deponun sonundaki `/` önemsizdir.
- **Aynı kural her yerde:** inceleme, kurulum, ortam hazırlama ve kurulu eklentinin çalışması `package_refusal`'a bakar
  (§84): sonradan yazılan bir sabitleme kurulu eklentiyi nedeniyle durdurur, kaldırmaz.
- **Kalan (ADIM 22A):** kurum aynası ve vekil sunucunun kimlik bilgilerinin kullanıcı/kiracı kapsamı (PY-UV-25),
  çevrimdışı tekerlek deposunun platform etiketi ve özetle doğrulanması (PY-UV-27).
- **Sınama:** eklenti paketi kayıt defteri 2 durum (adla ve önekle sabitlenen paketin genel depodan gelişi reddedilir,
  inceleme ve kurulum aynı sözle) ve 1 izin (gerçek deposuna sabitlenen ve ayracı tutmayan önek: ret yok).

## §119. Emekli eklenti söylenir, yerine geçene götürür (`ADIM 22A`, Emeklilik ve devir, D9)

**17 Kasım.** Kod: eklenti paketi `catalog.rs` (`CatalogEntry::deprecation`); masaüstü `extensions/view.rs`
(`retired_words`, `retired_banner`; kurulu ayrıntıda ve katalog girdisinde), `extensions/mod.rs` (`Event::Find`);
sınamalar `catalog.rs`, `extensions/tests/retired.rs` (yeni).

- **Sorun.** Manifestin `deprecation`'ı (yayımcının sözü ve `replacedBy`) şemada vardı ama hiçbir yerde
  gösterilmiyordu: emekliye ayrılmış bir eklenti kurulu listede ve katalogda sıradan bir eklenti gibi duruyordu, yerine
  geçen eklentiye giden yol yoktu (plan: "Emeklilik ve devir: yerine geçen eklenti").
- **Karar:** kurulu eklentinin ayrıntısında ve katalog girdisinde, ne olduğundan önce uyarı: "Bu eklenti yayımcısınca
  emekliye ayrıldı: {söz}." Yerine geçen kuruluysa adıyla ("Yerine geçen “Kent Rehberi” kurulu."), değilse kimliğiyle
  ve "Yerine geçeni bul" düğmesiyle: Bulunabilir listesi o kimlikle açılır. Katalog dizini (`kentos kek index`)
  emekliliği girdiye yazar; emekli olmayanın girdisinde alan yazılmaz, eski KentOS yeni alanı yok sayar (§86'nın
  kuralı). Ajanların ortak kataloğu da söyler (§113'ün `compatibility.deprecated`'ı).
- **Kurmadan önce de:** kurulum incelemesi emekliliği paketin bilgisinden önce uyarır; eskimiş bir eklentiyi kuran
  kişi bunu kurduktan sonra öğrenmez (gerçek pencere: inceleme ve kurulu ayrıntı, Yerine geçeni bul düğmesiyle).
- **Kalan (katalog sunucusu):** bakıcı devri ve terk edilmiş paketin işaretlenmesi.
- **Sınamalar:** eklenti paketi 1 (emekli paketin girdisi yerine geçeni adlandırır, gidiş-dönüşte aynı; emekli
  olmayanda alan yok); masaüstü 1 (sözler: yerine geçen yokken, kuruluyken, adıyla; emekli paket kurulur, manifesti
  emekliliği taşır; Yerine geçeni bul Bulunabilir'i kimlikle açar).

## §120. İşlemler'in sonuç dosyası hata enjeksiyonuyla: parça kalmaz, istek bir kez yazar (ECO-QA-10)

**17 Kasım.** Kod: İşlemler `staged.rs` (yeni: `create`, `put_in_place`, `piece_of`), `files.rs` (`cog_file`),
`cloud_files.rs` (`copc_file`), `local_files.rs` ve masaüstü `pointclouds/files.rs` (ikisi de ortak yazıcıyı
kullanır), `command.rs` (`Said`); sınamalar İşlemler `tests/staged.rs` (yeni), uygulama `processing_sink_faults.rs`
(yeni).

- **Sorun 1: yerine konamayan dosyanın parçası kalıyordu.** Raster ve bulut araçlarının sonucu yanına
  (`{ad}.yaziliyor`) yazılıp bitince yerine taşınır. Taşıma başarısız olunca (o adda bir klasör var; Windows'ta dosya
  başka bir programda açık) parça klasörde kalıyordu: başsız sunucunun yazıcısında, masaüstünün kendi kopyasında, COG
  düzenlemesinde (`.cog-yaziliyor`) ve COPC dizininde. Kod yorumu "yarıda kalan parça silinir" diyordu; yalnız yazarken
  bırakılan sink için doğruydu.
- **Karar:** iki ana bilgisayar tek yazıcıyı (`staged::create`) kullanır. Yerine konamayan, sonunda yazılamayan ya da
  bitirilmeden bırakılan dosyanın parçası silinir; yerindeki klasöre ve içindekine dokunulmaz. Dosya tutamağı
  silmeden önce kapanır: Windows açık dosyayı silmez. Süreç öldürülürse parça kalır; aynı adın sonraki yazımı onu
  baştan yazar.
- **Sorun 2: ret iletisi ara dosyayı adlandırıyordu.** Yazma izni olmayan klasöre yeni GeoPackage yazılınca ileti
  kullanıcıya `yeni.gpkg.kentos-525807-0` adını gösteriyor, olmayan bir dosya için "Eski dosya olduğu gibi kaldı"
  diyordu. Var olan dosyanın planı da (kopyası üzerinde yapılır) tablo çakışmasını kopyanın adıyla söylüyordu.
- **Karar:** ret kullanıcının verdiği yolu adlandırır. Son cümle durumu söyler: yerinde yazımda ya da üzerine yazılacak
  dosya varken "Eski dosya olduğu gibi kaldı.", yoksa "Dosya yazılmadı."; veritabanı hedefinde "Veritabanına
  dokunulmadı."
- **Yinelenen deneme (yeni sınamayla kilitlendi, davranış değişmedi):**
  - Cevabı yolda kaybolan istek aynı `requestId` ile gelirse önceki cevap döner (`replayed`) ve hiçbir şey yazılmaz:
    tablo "zaten var" diye reddedilmez, günlüğe ikinci satır düşmez.
  - Dosya o arada kullanıcıca değiştiyse, `overwrite: true` olsa bile üzerine yazılmaz.
  - Aynı çalıştırma başka bir istek kimliğiyle gelirse yeni bir yazımdır: tablosu varsa `table_exists`, dosya bayt bayt
    aynı.
  - Kimlik başka bir çağrıyla gelirse `request_reused`.
  - Ajanın planı tek kullanımlıktır (`plan_required`; `a_write_is_planned_first_and_executed_only_as_planned`).
- **Daha önce sınanmış olanlar (gözden geçirildi):**
  - Her noktada durdurma, dosyayı ve çizimi olduğu gibi bırakır (`a_run_stopped_at_any_point_…`).
  - GeoPackage olmayan dosya reddedilir, dokunulmaz.
  - Modelin ve toplu işin yarıda durması katman bırakmaz.
  - PostGIS'te ikinci yazım `table_exists` verir.
- **Sınamalar:** İşlemler 6, gerçek dosyalarla:
  - bütün dosya yerine konur, parça kalmaz; bitirilmeden bırakılan sink hiçbir şey bırakmaz;
  - yerinde klasör varken düz dosya, COG ve COPC parça ya da kutu bırakmaz;
  - yazma izni olmayan klasör nedeniyle söylenir.

  Uygulama 4:
  - istek yeniden gönderilir; ikinci yazım ve planın reddi kullanıcının dosyasını adlandırır;
  - yeniden gönderilen istek sonradan değişen dosyanın üzerine yazmaz;
  - salt okunur dosyada `addTables` ve `replaceTables` reddedilir, dosya bayt bayt aynı kalır;
  - salt okunur klasöre yeni dosya yazılamaz, ara dosya adı iletide görünmez.

  Düzeltme geri alınınca 6 sınamadan 4'ü kırmızı oldu.
