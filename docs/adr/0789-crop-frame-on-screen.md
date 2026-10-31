# ADR 0789: Kırpma çerçevesinin ekranda düzenlenmesi ve 1:20 PDF incelemesi

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-06'nın beşinci teslimi (ADV-06e) **[~]**.
- **Bağlam:** ADV-06 (“Mimari görünüşler, proje gezgini ve koordineli yayın”), dayanak A8–A9. Adlı görünüm ADR 0285, görünüş
  tarifi ADR 0774, proje gezgini ADR 0778, tarifin duvar ayrıntısı ve kırpması ADR 0781, görünüş şablonu ve kalem seti
  ADR 0783, revizyon işareti ve listesi ADR 0721, araç = komut CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); satırı aşağıda.
- **Şema:** değişmez. Görünümün alanı (orta, en, boy, dönüş) ADR 0285'ten beri `.kcad`'de; `FORMATS_VERSION` aynı.

## Araştırma (üreticilerin resmî belgeleri)

- Revit, [Crop a View](https://help.autodesk.com/cloudhelp/2024/ENU/RevitLT-GetStarted/files/GUID-F0E479F9-B646-46AC-8DE3-40EE6D7DB4DC.htm)
  ve [Crop Region Size](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-1B290480-EBE4-441D-8CB4-1746F2B7336A.htm):
  görünüşün kırpma bölgesi seçilince kenarlarında tutamaçlar çıkar, sürüklenen tutamaç bölgeyi değiştirir; Size Crop penceresi
  eni ve boyu sayıyla verir (Field of View kipi bölgeyi o boya gerer); bölgeyi değiştirmek paftadaki görünüş penceresini de
  değiştirir, basılı boyu değiştirmek için ölçek değişir.
- Archicad, [Drawing Frame](https://help.graphisoft.com/AC/25/INT/_AC25_Help/070_Documentation/070_Documentation-97.htm):
  yerleşimdeki çizimin çerçevesi kenarından sürüklenince “Manually Resized Frame” olur, “Crop Drawing to Frame” açıkken içerik
  çerçeveyle kırpılır; kırpılmış çerçeve noktalı çizilir.
- Revizyonlar: Revit'in [Revision Schedule](https://help.autodesk.com/cloudhelp/2018/ENU/Revit-DocumentPresent/files/GUID-9A72F51F-6DE4-4353-93E3-1D62B036AAE6.htm)
  paftanın bulutlarındaki revizyonları listeler; “Issued” işareti revizyonu kilitler. Ne Revit ne Archicad paftadaki görünüşün
  son yayından bu yana değişip değişmediğini kendiliğinden söyler (bulut elle konur).
- Sonuç: KentOS kırpma bölgesini modelde, adlı görünümün kendi alanı olarak düzenler (Revit gibi görünüşün özelliği; Archicad'in
  yerleşimdeki çerçevesi gibi ayrı bir pafta özelliği değil): bağlı haritalar ADR 0781'in kuralıyla izler. Sayıyla boy Revit'in
  Size Crop'u gibi ortası çevresinde. Yayın/revizyon birlikteliği bu teslimde yapılmadı (§4).

## Karar

### §1 Komut: `cad.views.edit`'in `area`'sı

- `ViewsOp::Area { name, center, width, height, twist? }` (Alanı değiştir): adı verilen görünümün ortası, eni ve boyu ve verilirse
  dönüşü ([0, 2π)'ye indirgenir; verilmezse görünümün dönüşü kalır). Kimliği, adı, listedeki yeri ve tarifi (sürümüyle) değişmez.
- Denetimler ve sırası Kaydet'inki: `not_finite`, `far_point`, `invalid_view_size`, `invalid_view_name`; sonra sürüm
  (`invalid_revision`, `revision_conflict`), sonra `unknown_view`. Aynı alan bir şey değiştirmez.
- Alan görünümün kendisinindir: görünüş şablonunu izleyen görünümde de (ADR 0783 §2) şablon alanı vermez, değişmez.
- Alan değişikliği **tek geri alma adımıdır**, “Görünümün alanı” (`views_edit::AREA_STEP`); öbür `cad.views.edit` işlemleri
  ADR 0285'teki gibi ayar düzenlemesi olarak kalır. Belgenin yeni işlemi `Op::ViewArea { key, before, after }`
  (`kentos_domain::view_area`, `Document::set_view_area`): geri alma ve yineleme alanı görünümün kalıcı anahtarına
  (`view_key`: kimliği, yoksa adının verdiği) yazar, o görünüm hâlâ varsa; arada değişen adı, tarifi ve öbür görünümler kalır,
  silinmiş görünüme bir şey yazılmaz.
- Bağlı haritalar ayrı yazılmaz: harita kamerasını her çizimde görünümden alır (`effective_fixed_view`; ortası görünümün ortası,
  tarifte ölçek yoksa ölçek alanın kutusuna sığar, dönüşü görünümün), kırpan tarif yeni alana kırpar (ADR 0781 §2). Bu yüzden
  harita, `cad.sheets.set`'in `shows`'u ve proje gezgini alanla aynı geri alma adımında izler.

### §2 Araç: Kırpma çerçevesi (`tool.viewCrop`)

- Adlı görünümlerin alanları modelde çerçeve olarak (seçilmeyenler kesikli, adlarıyla); seçilenin dokuz tutamacı: dört köşe,
  dört kenar ortası (kare) ve orta (halka). Çerçevenin içine tıklamak o görünümü seçer, iç içe çerçevelerde küçüğünü.
- Tutamaç sürüklenip bırakılınca (ya da basılıp yerinde bırakılıp ikinci tıklamayla) alan yazılır: köşe iki kenarı, kenar
  ortası bir kenarı taşır, karşı kenarlar yerinde kalır; orta bütün çerçeveyi taşır; dönük görünümde kenarlar görünümün
  eksenlerindedir; en ve boy 1 mm'den küçük olmaz. Tutamaç tutulurken kenet açıktır (kenar bir duvar köşesine oturabilir) ve
  yazılan nokta tutamacı oraya koyar.
- `12x8` (ya da `12×8`, `12*8`) eni ve boyu ortası çevresinde verir; Öznitelikler'de Görünüm, Genişlik ve Yükseklik satırları
  (araç ayarları, ADR 0657) aynı komutu çağırır.
- Esc önce tutulan tutamacı bırakır (çerçeve eski yerinde, bir şey yazılmaz), sonra görünümü, sonra aracı bitirir. Ctrl+Z
  tutamaç tutulurken onu bırakır; tutulmuyorken çizimin geri almasıdır (son alan geri gelir).
- Önizleme: sürüklenen çerçeve kalın vurgu renginde, eski yeri ince kesikli; etiket “Ad: en × boy”.
- Görünümler penceresinde **Alanı düzenle**: pencere kapanır, araç seçili görünümle başlar (`Memory::view_crop`).
- Şeritte Mimari › Proje ▾ (Gezgin'in yanında); takma adlar KIRPMACERCEVESI, CROPREGION. Kısayol listesinin Fare grubunda
  “Çerçeve tutamacını sürükle”. Manifestte `semantic`, `via` `cad.views.edit`.

### §3 1:20 PDF incelemesi

- Banyo detay (ADR 0781'in görünümü) 1:20, Ayrıntılı ve kırpılmış tarifle, bağlı paftası pafta kipinin PDF'iyle yazıldı;
  poppler'ın `pdftotext -layout`'u ve `pdftoppm -r 150`'si ile okundu (`view_crop_scenes::tests::a_cropped_detail_at_1_20_on_paper`;
  `.run/shots/kirpma-1-20-pdf-1.png`, `kirpma-1-20.txt`). Bulgular:
  - Duvarların katmanları ve taramaları (çapraz tuğla, iç yalıtımın çift taraması) 1:20'de ayrıntılı çizilir; kapı açıklığı,
    kasa ve kanat yayı kesilmeden yerinde. Banyonun etiketi kâğıtta okunur boyda (italik, ≈2 mm), alanı “12.04 m²”;
    antetin ölçeği “1/20”.
  - **Düzeltildi:** kırpılan haritada alanın dışında kalan yazılar görünmüyordu ama PDF'in içindeydi: `pdftotext` yandaki
    odanın etiketini (“201 · 2 Daire 2”, 14.62 m²) okuyordu. PDF yazıcısının yazı kuralı yalnız içerik kutusuna bakıyordu
    (`kentos_sheet::pdf::maps::Frame::holds`); artık kırpma şeklinin (`Clip::contains`) içinde de olmalıdır. Kesilen yazı
    dosyada yok; test kırpılan dikdörtgenin dışındaki yazıyı yazmaz (`a_text_cut_away_is_not_written`) ve 1:20 paftanın
    sözcüklerinde 201 yoktur.
  - **Açık:** görünümün alanı 1:20'de 3.8 × 5.3 m = 190 × 265 mm; A3 haritanın içerik yüksekliğinden büyük olduğu için alt ve
    üst duvarlar haritanın kendi çerçevesiyle kesilir. Tarifin ölçeği ve kırpma birlikteyken çerçevenin alana yetmediğini
    ön denetimin söylemesi ya da çerçevenin alana göre boylanması ayrı iştir.

### §4 Yayın/revizyon birlikteliği: açık

Yapılmadı. KentOS'ta revizyon ADR 0721'in modeldeki “Revizyon” üçgenleridir (numara, tarih, açıklama); paftanın kendi yayın
kaydı yoktur (pafta başına revizyon çizelgesi ADR 0721'de açık). “Son yayından bu yana değişen görünüm” için önce paftanın yayın
kaydı (hangi revizyonla, hangi gün, o anki görünüm damgaları: tarif/şablon sürümü ve alan) ve Revit'in “Issued” kilidinin
KentOS'taki anlamı sahibin kararını ister; damgasız bir “değişti” sözü yanlış güven verirdi. Görünümün tarif sürümü
`cad.sheets.set`'in çerçevesinde (`recipe`) ADR 0774'ten beri vardır; alanın sürümü yoktur, yayın kaydıyla birlikte gelir.

## Doğrulama

- `python3 scripts/fixtures/view_area_cases.py --check`: `cad.views.edit.area.json` 28 durum (alan, dönüş, aynı alan, geri
  alma ve yineleme, arada adlandırılan/tarifi değişen/silinen görünüm, plan, doğrulama, retler, sürüm) ve
  `cad.sheets.set.area.json` 20 durum (alan genişleyince, daralınca, uzaklaşınca, kayınca, dönünce, geri alınıp
  yinelenince, plan ve aynı alanda, kırpma kalkınca dar haritanın gösterdikleri; ADR 0774 ve 0781'in başvurularının
  üstüne, dönük harita çerçevenin dönüşüyle), KentOS kodu olmadan. Ortak durum koşucusu `area`'nın sayılarına da
  `nonFinite` verir.
- `KENTOS_CASES_ONLY=cad.views,cad.sheets.set cargo test -p kentos-native-application --test all every_command_case`.
- `KENTOS_TRACES_ONLY=view-crop cargo test -p kentos-desktop --lib every_trace_passes`: `view-crop.json` gerçek fare ve klavye
  olaylarıyla (sürükleme, Ctrl+Z, yazılan en×boy, iki tıklamayla taşıma, Esc); izin yeni `namedViews` beklentisi.
- `cargo test -p kentos-desktop --lib view_crop_scenes -- --include-ignored`: kenar sürüklemenin tek adımı, geri alma ve
  yineleme, bağlı haritanın kırpması; Esc'in sırası; yazılan en×boy ve pencerenin Alanı düzenle'si; 1:20 PDF (poppler).
- `cargo test -p kentos-interaction --lib view_crop`: kenar, köşe ve orta tutamacı, dönük çerçeve, ters dönmeyen çerçeve.
- `cargo test -p kentos-sheet --lib pdf::maps`, `--test all clip`: kırpılan haritanın yazı kuralı.
- Resimler: `KENTOS_SHOTS_ONLY=kirpma-pencere,kirpma-cerceve,kirpma-surukle,kirpma-pafta cargo test -p kentos-desktop --lib
  tools_screens::screens -- --ignored --nocapture` (`.run/shots/arac-kirpma-*`, 1440×900 ve 1100×650, açık ve koyu).

## Açık

- Yayın/revizyon birlikteliği (§4); kırpmanın dönük görünümde kutu yerine kesin çokgenle sayılması (ADR 0781'den);
  Revit'in açıklama kırpması (annotation crop); türetilmiş çizim; gerçek pencerede elle kabul.

## Belgelere eklenecek satırlar

- CLAUDE.md §0, ADR 0781 maddesinden sonra:
  “- **Kırpma çerçevesi (31 Ekim 2026; [ADR 0789](docs/adr/0789-crop-frame-on-screen.md), ADV-06 [~]).** `cad.views.edit`'in
  `area`'sı adlı görünümün ortasını, enini, boyunu ve isteğe bağlı dönüşünü yazar; kimlik, ad, yer ve tarif kalır; tek geri alma
  adımı “Görünümün alanı” (`Op::ViewArea`, görünümün kalıcı anahtarıyla), öbür işlemler ayar düzenlemesi. Bağlı haritalar ve
  kırpan tarif aynı adımda izler. Kırpma çerçevesi aracı (`tool.viewCrop`): modelde görünümlerin çerçeveleri, seçilenin dokuz
  tutamacı (köşe, kenar ortası, orta), sürükle-bırak ya da iki tıklama, `12x8`, Öznitelikler'de Genişlik/Yükseklik; Esc tutamacı,
  görünümü, aracı bırakır, Ctrl+Z son alanı geri alır; Görünümler'de Alanı düzenle. İzin `namedViews` beklentisi. 28 + 20 ortak
  durum (`view_area_cases.py`), iz `view-crop.json`, 1:20 PDF poppler ile; kırpılan haritanın dışındaki yazı PDF'e yazılmaz. Yayın/revizyon birlikteliği açık (paftanın yayın
  kaydı yok).”
- CLAUDE.md §2 komut listesine: `python3 scripts/fixtures/view_area_cases.py --check; KENTOS_CASES_ONLY=cad.views,cad.sheets.set
  cargo test -p kentos-native-application --test all every_command_case; KENTOS_TRACES_ONLY=view-crop cargo test -p
  kentos-desktop --lib every_trace_passes; cargo test -p kentos-desktop --lib view_crop_scenes   # kırpma çerçevesi: 28 + 20
  ortak durum KentOS kodu olmadan, iz ve masaüstü; resimler KENTOS_SHOTS_ONLY=kirpma-pencere,kirpma-cerceve,kirpma-surukle,kirpma-pafta
  …tools_screens::screens -- --ignored --nocapture; 1:20 PDF cargo test -p kentos-desktop --lib
  view_crop_scenes::tests::a_cropped_detail_at_1_20_on_paper -- --ignored --nocapture (ADR 0789)`
- plan/architecture-advanced.md ADV-06 Durum'una: “Kırpma çerçevesinin ekranda düzenlenmesi ve 1:20 PDF incelemesi
  ([ADR 0789](../docs/adr/0789-crop-frame-on-screen.md)).” Açık listesinden “kırpma çerçevesinin ekranda düzenlenmesi” ve
  “1:20 PDF incelemesi” çıkar; “yayın/revizyon birlikteliği (paftanın yayın kaydı ister)” kalır.
- TODOS.md §0.2 (Web'e ertelenenler): “ADR 0789: `cad.views.edit`'in `area`'sı ve geri alma adımı web belgesinde; Kırpma
  çerçevesi aracı ve Görünümler'in Alanı düzenle'si; izlerin `namedViews` beklentisi web oynatıcısında.”
