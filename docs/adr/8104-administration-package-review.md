# ADR 8104: Gelen paketi incele — idarenin inceleyeni paketi müellifin çizimi olmadan katman katman denetler

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-18 ("İdare inceleme
  çalışma alanı"), B20. İlk kabul sağlandı (§4); imar denetimi, e-PYS aktarımı ve erişim kaydı §6'da.
- **Bağlam:** ADR [8075](8075-bim-permit-package.md)–[8077](8077-ek9-author-answers-in-the-drawing.md) (BIM ruhsat paketi,
  `yayin.json`, kontrol formunun kaydı), [8074](8074-ek9-from-the-written-file.md) (formun 21 sorusu yazılmış dosyadan,
  `kentos_ifc::read::file_facts`), [8037](8037-turkish-title-block-sheet-sets-pdfa.md) (PDF/A-2 denetimi), [8099](8099-delivery-record.md) (teslim ve iade
  kaydı), [8103](8103-ifc-revisions-and-bcf.md) (BCF), [8073](8073-ifc-viewer-first-slice.md) (IFC görüntüleyici), plan K10
  (denetim kuralları Python'da, olgular Rust'ta), araştırma notu §1.2 (e-PYS).
- **Web:** dokunulmadı (ADR 0228); TS türleri üretildi (`ReviewPackage`, `PackageReviewed`, `Review*`); pencere TODOS.md
  §0.2'de. **`.kcad`:** değişmedi. **Komut:** `file.review.package` v1 (başsız; dosya okur, yazmaz). **Kural:** dijital-proje
  0.10.0 (`dijital_proje.inceleme`). **Bağımlılık:** yeni yok (`kentos-headless` mimari özelliğiyle `kentos-ifc`'yi kullanır).

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| e-PYS Yönetmeliği (RG 5.8.2026/33331; [tam metin](https://www.alomaliye.com/2026/08/05/mimarlik-ve-muhendislik-projelerinin-elektronik-ortamda-teslimi-ve-yonetilmesi-hakkinda-yonetmelik/); araştırma notu §1.2) Md. 4/3, 7/7 | İdarelerin yazılım platformları ile e-PYS arasında dosya ve bilgi paylaşılır; platformda yapılan işlem e-PYS'ye aktarılır ve kayda alınır | İnceleme idarenin yazılımında yapılır; aktarım arayüzü yayımlanınca (4/4) bağlanır (§6) |
| aynı, Md. 4/5 | İdare veriyi üçüncü kişiye veremez, müellifin izni olmadan çoğaltamaz, değiştiremez | İnceleme dosyaları yalnız okur; hiçbir şey yazmaz, kopyalamaz (K175) |
| aynı, Md. 5/1–3 | Müellif kaydeder ve doğruluğundan sorumludur; dosyalar Hazırlama Yönetmeliğine göre incelenir, eksik ya da hatalı dosya yeniden yüklenir; idare eksikleri gerekçesiyle bildirir | Her bulgu dayanağıyla, ölçülen ve istenen değerle, dosyası ve nesnesiyle; iade notu bunlardan (K178) |
| aynı, Md. 6/4 | Taranmış PDF kullanılamaz | Yalnız görüntüden oluşan sayfa pafta katmanının bulgusudur (K176) |
| Dijital Proje Yön. Md. 4/2, 9/1, 13, 15; EK-2, EK-6, EK-9 | Paftalar PDF/A, model IFC 4.3, ikisi aynı modelden ve uyumlu; proje bilgileri ve koordinat sistemi; her teslimde kalite kontrol formu | Beş katman: paket, pafta, model, konum, form (K176) |
| ADR 8074, 8075 (KentOS'un yayın tarafı) | Formun 21 sorusu yazılmış dosyanın olgularından; paket `yayin.json` ile dosyalarını özetleriyle listeler; paftalar kaynaklarını (`kentos.sheet-source/1`: çizimin revizyonu ve özeti) içlerinde taşır | İnceleme aynı olguları müellifin çizimi olmadan, gelen dosyalardan ölçer; karar aynı paketin kuralında (K177) |

## §2. Karar

- **K175 — Yalnız okur:** `file.review.package` paketi (yayın klasörü ya da tek bir `.ifc`) geldiği gibi okur; müellifin
  çizimini istemez, dosya yazmaz. Bir dosyanın okunamaması bulgudur, ret değil; ret yalnız yolun kendisi ve kuralın
  çalışmaması (`review_rule_unavailable`: Python yok).
- **K176 — Katmanlar ve olgular** (Rust ölçer, `crates/native/headless/src/package_review.rs`):
  - **paket:** `yayin.json` (biçimi `kentos.publish/1`), her dosya listede mi, klasörde mi, baytı ve SHA-256'sı listedeki
    mi; listede olmayan dosya (uyarı); listenin modeli, paftaları ve formu var mı. Tek `.ifc`: "yalnız model" bulgusu.
  - **pafta:** her PDF'in PDF/A-2 raporu (kimliği ve ISO 19005-2 kuralları, ADR 8037), yalnız görüntüden oluşan sayfaları
    (görüntü var, çizgi ve yazı yok: taranmış) ve içindeki kaynağın çizim revizyonu ve özeti; kaynak paketinkinden başkaysa
    "farklı revizyonlu pafta".
  - **model:** dosyanın olguları (`kentos_ifc::read::file_facts`) ya da neden okunmadığı; şema hataları; formun dosyadan
    cevapladığı sorularda kalanların bulguları nesneleriyle (GlobalId).
  - **konum:** modelin koordinat sistemi (IfcProjectedCRS) idarenin verdiği sistem mi (EPSG kodu, dilim); verilmezse
    değerlendirilmez. Formun 6. sorusunun (koordinat sistemi) bulguları da bu katmandadır.
  - **form:** müellifin kaydındaki cevap dosyadan ölçülenle aynı mı (inceleme soruları dışında); form teslim edilen
    modelin mi (özetiyle).
- **K177 — Karar Python'da** (`dijital_proje.inceleme.denetle`, plan K10): bulgular katmanların sırasıyla, katmanlar
  (tutuyor, tutmuyor, değerlendirilmedi) ve formun 21 satırı yan yana (müellif, KentOS, uyuşuyor mu).
- **K178 — İade notu:** hata bulguları BCF olarak (`cad.bcf.write`: her konu bulgunun nesnelerini seçer, paketin modeline
  bağlanır; etiketi katmanı) ve yanında yazı olarak (`.txt`: katman, bulgu, onarım, dayanak, dosya, nesneler). Modelde göster,
  paketin modelini IFC görüntüleyicide açar, bulgunun nesnelerini seçer ve boyar.

## §3. Uygulama

- **Sözleşme:** `crates/shared/contracts/src/arch/package_review.rs` (`ReviewPackage`, `PackageReviewed`, `ReviewLayer`,
  `ReviewFinding`, `ReviewFormRow`, `ReviewFile`); `FILE_COMMANDS`'ta; kod `review_rule_unavailable`.
- **Başsız:** `package_review.rs` (olgular; PDF bilgi sözlüğünden kaynak; `kentos_formats::pdf`), `file_commands.rs` (mimari
  özelliğiyle).
- **Kural:** `catalog/standards/dijital-proje/src/dijital_proje/inceleme.py`, paket 0.10.0; birim testleri
  `catalog/standards/tests/dijital-proje/test_inceleme.py` (10).
- **Masaüstü:** `apps/desktop/src/arch/package_review_window.rs` (`arch.reviewPackage`, Mimari › Proje ▾); IFC
  görüntüleyiciye `open_ifc_model_showing` (açılınca nesneleri seç ve boya).

## §4. Sınamalar ve ölçümler

- **İlk kabul** (`crates/native/headless/tests/headless/package_review.rs`):
  - KentOS'un kendi paketi (`fixtures/bim/v1/paket/kentos`: beyanlı kesit binası BIM bölümüyle yayımlandı, TUREF / TM36)
    idarenin EPSG:5256'sında beş katmanda da tutar, formun 21 satırının hiçbiri ayrışmaz.
  - Başka bir yazılımın açık örnek modeli (buildingSMART'ın IFC 4.3 mimari örneği) tek başına açılır: paket "yalnız model",
    konum "başka dilim" (EPSG:32760), pafta ve form değerlendirilmedi.
  - Bilerek bozulan paket (`scripts/fixtures/review_packages.py`, IfcOpenShell: model TM33'te; doğu duvarı
    `01alMXGa9m080000000002` yangın sınıfsız, form hâlâ Evet; UP-03 paftası çizimin 8. revizyonundan): konum
    `review_place_zone`, pafta `review_sheet_revision` (yalnız UP-03'te), model yangın dayanımı bulgusu duvarın GlobalId'siyle,
    form `review_form_disagrees`; paket katmanında bulgu yok (listesi dosyalarla tutarlı).
  - Öbür bozuk paketler kendi bulgularını verir: eksik, değişmiş ve listede olmayan dosya, PDF/A-3 beyanı (`dosyalar`),
    taranmış pafta (`taranmis`), okunmayan model (`okunmayan`), listesiz klasör (`listesiz`), formsuz paket (`formsuz`);
    yol yoksa, göreli ya da boşsa ret.
- **Masaüstü** (`package_review_tests`): bozuk paket incelenir, başarısız katmanlar pafta, model, konum, form; yangın
  bulgusu seçilip Modelde göster'e basılınca IFC görüntüleyicide seçili nesne o duvardır; iade notu BCF'ye ve yazıya
  yazılır, BCF'nin bir konusu duvarı seçer. KentOS'un paketi her katmanda tutar, iade gerekçesi yazılmaz. Kesişen
  süzgeçler geçti (159).
- **Kural:** 10 birim testi; dijital-proje'nin öbür testleri (bim, form, paket) geçti.
- **Resimler:** `.run/gui-evidence/paket-inceleme-8104/` (bozuk paket, form, duvar modelde, KentOS paketi, şeritteki yeri;
  açık ve koyu); simge `.run/gui-evidence/ikon-paket-incele/` (A).
- **Yapılmadı:** paylaşılan komut durumları (≥ 20): bozuk paketlerin notu (`review_packages.json`) bağımsız beklentiyi
  taşıyor ve başsız sınamada oynatılıyor, ama `fixtures/commands/v1/` biçiminde bir durum dosyası yok (yayın komutu gibi
  başsız komutların ortak koşucusu yok, ADR 8075 §8).

## §5. Arayüz

- **Gelen paketi incele:** Paket (Klasör…, IFC…), İdarenin sistemi (EPSG; boşsa konum değerlendirilmez), İncele (iş
  parçacığında); beş katmanın işareti (tutuyor, tutmuyor, değerlendirilmedi); dosya, hata ve uyarı sayısı; Bulgular | Form
  sekmeleri; bulgu seçilince bulgu, onarım, dayanak, dosya ve nesneler, Modelde göster; Yazar ve İade notu yaz….

## §6. Sıradaki adımlar

- **İmar denetimi:** parsel, çekme, TAKS, emsal, kat adedi ve yükseklik; belediyenin kendi plan ve parsel verisiyle, kaynağı
  ve onay tarihi görünerek (BIM-18'in maddesi).
- **e-PYS:** aktarım arayüzü yayımlanınca (Md. 4/3, 4/4, 7/7) iade notunun ve kararın aktarımı.
- **Erişim kaydı ve gizlilik:** incelemenin kim tarafından, ne zaman yapıldığının kaydı; paketin çoğaltılmaması (Md. 4/5).
- **Görüntüleme:** kat kat plan, mahal ve emsal süzgeçleri, pafta ile model yan yana.
- **Paylaşılan komut durumları** ve gerçek pencerede kabul (ADR 8000).
