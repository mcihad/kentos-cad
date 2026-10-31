# ADR 0781: Görünüş tarifinde duvar ayrıntısı ve görünümün alanına kırpma

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-06'nın üçüncü teslimi **[~]**.
- **Bağlam:** ADV-06 (“Mimari görünüşler, proje gezgini ve koordineli yayın”), dayanak A8–A9. Görünüş tarifi ADR 0774, proje
  gezgini ADR 0778, duvarın ölçeğe göre ayrıntısı ADR 0339 §4 ve ADR 0696 (eki dahil), atlas nesnesine kırpma ADR 0638 §2, pafta
  seti ADR 0347.
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'de.
- **Şema:** `.kcad` şema 143 (opt-in): tarifin `detail`'i ya da `crop`'u olan çizim 143 yazar, öbürleri eskisi gibi.
  `FORMATS_VERSION` değişmez (tipli sütunlar değişmedi).

## Araştırma (üreticilerin resmî belgeleri)

- Revit, [View Properties](https://help.autodesk.com/view/RVT/2024/ENU/?contextId=HID_VIEW_PROPERTIES): görünüşün Detail Level'ı
  (Coarse, Medium, Fine) ölçeğin kendiliğinden ayrıntısının yerine geçer; duvar, döşeme ve çatının katmanları Medium ve
  Fine'da görünür. [Crop a View](https://help.autodesk.com/view/RVT/2024/ENU/?guid=GUID-DDD8E523-E1FC-42C5-A89E-488F6DDC5AA4):
  Crop View görünüşü sınırıyla kırpar, sınır gizlense de kırpma sürer; paftadaki görünüş penceresi kırpılmış çizer.
- Archicad 30: kayıtlı görünümün (View Map) ayarlarında Model View Options ve Partial Structure Display; yerleşimdeki çizim
  kaynağının kendi kırpma çerçevesi vardır.
- Sonuç: KentOS iki alanı adlı görünümün tarifine ekler. Duvar ayrıntısı Revit'in Detail Level'ı gibi ölçeğin ve projenin
  ayarının üstündedir; kırpma, görünüme bağlı haritaları görünümün alanıyla sınırlar. İkisi de yalnız bağlı paftalarda geçer;
  model ve ekran değişmez (ADR 0774 §3'ün kuralı).

## Karar

### §1 Duvar ayrıntısı (`detail`)

- `ViewRecipe.detail`: `schematic`, `standard` ya da `detailed`; yok: ölçeğe göre (proje ayarının ayrıntısı, yoksa ölçeğin;
  `walls::view_at`). Varken haritanın ölçeğinden ve projeninkinden önce gelir: `walls::view_with(settings, scale, detail)`.
- Pafta çekirdeği bağlı haritanın `MapPrim.wall_detail`'ine tarifin ayrıntısını yazar. Masaüstünün pafta haritası
  (`SheetMaps::scaled`) ve başsız kitap ile atlas (`kentos_plot::book`) haritanın deposunu bu ayrıntıyla kurar
  (`Spatial::set_wall_detail`, `walls_differ_with`); kopyanın anahtarında ayrıntı da vardır, aynı ölçekteki iki harita
  ayrı çizer. Stilli katmanların önbelleği deponun duvar ayrıntısını sayar.
- Ayrıntı yalnız çizimi değiştirir: haritanın gösterdiği nesneler (`cad.sheets.set`'in `shows`'u, gezginin bağları) aynıdır.

### §2 Kırpma (`crop`)

- `ViewRecipe.crop` (öntanımlı `false`, yazılmaz): görünüme bağlı her harita içeriğini görünümün alanına kırpar: alanın dört
  köşesi görünümün dönüşüyle (`viewport::view_ring`), haritanın zeminine çevrilip çerçeveye kesilerek, atlas nesnesine
  kırpmanın kuralıyla (`display::map::feature_clip`, en çok 256 köşe). Atlas nesnesine kırpma önce gelir.
- `cad.sheets.set`'in `shows`'u: kırpan tarifte nesnenin kutusu görünümün alanının kutusuyla da kesişmelidir (dönük görünümde
  kutu geniştir: fazladan listelenebilir, eksik listelenmez). Proje gezgini haritaları buradan okur.
- Görünümün kendisi (ekrandaki adlı görünüm) değişmez.

### §3 Masaüstü

- Görünümler penceresinin Görünüş tarifi bölümünde **Duvar ayrıntısı** (Ölçeğe göre, Şematik, Uygulama, Ayrıntılı) ve
  **Bağlı haritaları görünümün alanına kırp**. Hazır tariflerden Uygulama planı 1:50 Ayrıntılı'dır. Tarifin yazısı “duvar
  ayrıntılı”, “kırpılmış” der.

### §4 Dosya

- `.kcad` şema 143: tarifin `detail`'i (`schematic`/`standard`/`detailed`) ve `crop`'u (yalnız `true` yazılır). Daha eski
  şemada bunlardan biri, bilinmeyen ayrıntı ya da yazılmış `crop: false` reddedilir. Bağımsız Python okuyucusu ve yazıcısı
  (`tools/kcad/kcad.py`, `kcad_v2_reference.py`) aynı kuralla; örnek `view-recipe-details.kcad` ve üç bozuk dosya.

## Doğrulama

- `python3 scripts/fixtures/view_recipe_cases.py --check`: `cad.views.edit`'in tarif durumları 33 (ayrıntı ve kırpma dahil).
- `python3 scripts/fixtures/view_recipe_crop_cases.py --check`: 22 ortak durum (`fixtures/commands/v1/cad.sheets.set.crop.json`),
  KentOS kodu olmadan, ADR 0347 ve 0774'ün başvurularının üstüne: dar görünümün alanı, alana değen ve dışındaki nesneler,
  kırpmanın kalkması ve açıkça kapanması, katsız kırpan tarif, ayrıntının gösterileni değiştirmemesi, geniş görünümün kırpması,
  silinen görünüm. `view_recipe_sheet_cases.py --check` eskisi gibi.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad --test all`: şema 143, örnekler.
- `KENTOS_CASES_ONLY=cad.sheets.set,cad.views,cad.navigator cargo test -p kentos-native-application --test all -- every_command_case capabilit`.
- `cargo test -p kentos-desktop --lib -- view_detail view_recipe navigator`: bağlı haritanın ayrıntısı ve kırpması, Ayrıntılı'nın
  Şematik'ten çok vektör yazması, kırpma kalkınca çerçevenin kendi biçimi; pencerenin ayrıntı ve kırpma seçimleri.
- Resimler: `KENTOS_SHOTS_ONLY=gorunus-ayrinti-pencere,gorunus-ayrinti-pafta cargo test -p kentos-desktop --lib
  tools_screens::screens -- --ignored --nocapture` (`.run/shots/arac-gorunus-ayrinti-*`, 1440×900 ve 1100×650, açık ve koyu).

## Açık

- Kalem seti, görünüş şablonu, türetilmiş çizim (ADV-06'nın kalanı); kırpma çerçevesinin ekranda gösterilip tutamaçla
  değiştirilmesi; kırpmanın dönük görünümde kutu yerine kesin çokgenle sayılması. Gerçek pencerede elle kabul yapılmadı.
