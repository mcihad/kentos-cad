# ADR 0774: Görünüş tarifi: adlı görünüm üstünde kat, evre, katman durumu, gizlenen türler ve ölçek

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-06'nın ilk teslimi **[~]**.
- **Bağlam:** ADV-06 ("Mimari görünüşler, proje gezgini ve koordineli yayın"); dayanak A8–A9. Adlı görünüm ADR 0285, görünümün
  kararlı kimliği ADR 0345 §2, paftadaki canlı bağ ADR 0599 (`Source::ViewLink`), paftanın katı ADR 0716, evre görünümü
  ADR 0346, katman durumları ADR 0177 ve 0314, pafta seti ADR 0347.
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'de.
- **Şema:** `.kcad` 141 (opt-in), `FORMATS_VERSION` 101.

## Araştırma (üreticilerin resmî belgeleri)

- Archicad 30 yardımı: [View Map ve kayıtlı görünüm](https://help.graphisoft.com/AC/30/INT/Topics/050_ViewsVB/050_ViewsVB-90.html),
  [View Settings](https://help.graphisoft.com/AC/30/INT/Topics/050_ViewsVB/050_ViewsVB-101.html): bir görünüm kamerayla birlikte
  katman takımı, kalem seti, model görüntü seçenekleri, renovasyon süzgeci, ölçek ve yapı gösterimini saklar; yerleşim (layout)
  görünümü çizer, modeli değiştirmez. KentOS'ta tarif aynı yerde, adlı görünümün üstündedir: kamerayı korur, neyin çizildiğini
  söyler.
- Revit'in görünüş özellikleri ve görünüş şablonu (View Template: kat, Phase ve Phase Filter, View Scale, Visibility/Graphics'te
  kategori gizleme): paftadaki görünüş (viewport) kendi görünüşünün özellikleriyle çizer, kategoriyi gizlemek öbür görünüşü ya da
  modeli etkilemez. KentOS bunu ayrı bir “görünüş şablonu” nesnesi kurmadan, görünümün kendi tarifiyle yapar; şablon/kopya ileride.
- Sonuç: **tarif görünümündür**: model özelliği (nesnenin katı, evresi) değişmez; ekranın kat görünümü, teması ve katman paneli
  kullanıcınındır, tariften etkilenmez. Tarif yalnız ona bağlı paftanın haritasında uygulanır.

## Karar

### §1 Veri

- Adlı görünümün `recipe`'i (`ViewRecipe`): `version` (değiştikçe bir artar; girdide verilen yok sayılır), `level` (`id`,
  `below`: alt kat soluk), `phase` (`PhaseView`), `layerState` (projenin katman durumunun adı), `hidden` (gizlenen türler:
  `dimensions`, `tags`, `texts` (kılavuz da), `hatches`, `blocks`, `spaces`, `grids`, `viewmarks`; kendi sıralarında, birer kez),
  `scale` (1:N, 1–100 000). Sürümden başka en az bir alan; varsayılan değerinde alan yazılmaz (`recipe_problem`).
- `cad.views.edit`'in `recipe` işlemi: tarifi yazar ya da (`recipe` verilmezse) kaldırır; görünüm yoksa `unknown_view`, projede
  olmayan kat `level_not_found` (yol `edit.recipe.level.id`), olmayan durum `state_not_found`, kuralı kıran `invalid_recipe`.
  Aynı tarif değişiklik değildir (sürüm artmaz). Görünümün kaydı (Güncelle) tarifi korur. Ayardır, geri alma adımı değil (ADR 0285).

### §2 Arayüz

Görünümler penceresinde seçilen görünümün altında Görünüş tarifi: Hazır ▾ (Uygulama planı 1:50, Mobilya planı 1:100, Mahal planı
1:100, Yıkım planı 1:100, Yeni durum 1:100; kat korunur), Kat ve Alt kat soluk, Evre, Katman durumu, Ölçek, Gizlenen türler,
Tarifi uygula, Tarifi kaldır. Listede tarifli görünümün satırı tarifi kısaca söyler. Hazır tarif azdır; şeride yeni ikon eklenmedi.

### §3 Paftada

- Görünüme bağlı haritanın kamerası görünümün ortasıdır; tarifin ölçeği varsa haritanın ölçeği odur (`effective_fixed_view`'in
  oranı), yoksa görünümün kutusu çerçeveye sığdırılır (ADR 0599).
- Harita tarifin katını (yoksa kendi katını), katman durumunu (`MapLayers::Theme`), evresini ve gizlenen türlerini çizim anında
  okur (`linked_recipe`, `MapPrim.level/phase/hidden`); ekran, başsız kitap ve PDF aynı kuralla (`map_phasing_of`, etiket yerleşimi
  gizlenen türleri ve evrede gizlenenleri atlar).
- Tarif değişince bağlı her pafta hemen yeni tarifle çizer; model bir kez değişince (kapı genişliği) iki pafta da izler.

### §4 Pafta seti

`cad.sheets.set`'in çerçevesi `view` (bağlı görünümün kimliği), `missing` (görünüm silinmiş; zemin bilinmez), `level` (tarifin ya
da haritanın katı) ve `recipe` (tarifin sürümü) söyler. Verilen nesneler için `shows` artık haritanın **çizdiğini** sayar: kutusu
zemine değen, katı haritanın katı ya da katsız (ya da projede olmayan bir kat), türü tarifte gizlenmemiş ve evrede gizlenmeyen.

## Uygulama

- Sözleşme `cad_views.rs` (`ViewRecipe`, `RecipeLevel`, `ViewCategory`, `recipe_problem`, `ViewsOp::Recipe`), kod
  `invalid_recipe`; komut `views_edit.rs`; KCAD `decode/views.rs`, `encode/views.rs` (`SCHEMA_WITH_VIEW_RECIPES`).
- Pafta çekirdeği `viewport.rs` (`linked_recipe`, tarifin ölçeği), `display/map.rs`; çizim `kentos-plot` (`layers::map_phasing_of`,
  `maps.rs`, `labels.rs`, `book.rs`, `preflight.rs`); masaüstü `sheets.rs` (stilli harita önbelleğinin anahtarı tarifi içerir).
- Pafta seti `sheets_set.rs` (`drawn_by`), `cad_sheet_set.rs` (`SetFrame`).
- Masaüstü `views_recipe.rs`, `views.rs`; sahne ve testler `view_recipe_scenes.rs`.

## Doğrulama

- `scripts/fixtures/view_recipe_cases.py` (25 durum, `cad.views.edit.recipe.json`) ve `view_recipe_sheet_cases.py` (22 durum,
  `cad.sheets.set.recipe.json`; ADR 0347'nin çerçeve ve köşe kurallarını yeniden kullanır, KentOS kodu olmadan); ikisi de masaüstü
  işleyicileriyle geçer.
- KCAD: `view-recipes.json/kcad` ve yedi bozuk örnek (boş tarif, sürüm 0, iki kez tür, bilinmeyen tür, ölçek dışı, `below: false`,
  bilinmeyen alan) bağımsız yazıcı ve okuyucuyla (`kcad_v2_reference.py`, `tools/kcad/kcad.py`) ve Rust kodeğiyle aynı sonuç.
- Masaüstü: pencere tarifi yazar, sürümü artırır, kaldırır, modeli değiştirmez; bağlı iki harita 1:50 ve 1:100 çizer, mobilya
  haritasında etiket ve ölçü yok, tarif değişince ölçek hemen değişir. Resimler `gorunus-tarifi-pencere`, `-uygulama`,
  `-mobilya`.

## Açık (ADV-06'nın kalanı)

- Proje gezgini (bina/kat/görünüş/çizelge/pafta bağları ve bir nesnenin öbür görünüşlerdeki temsili).
- Kalem seti, duvar ayrıntısının tarife bağlanması, kırpma/çizim yönü; görünüş şablonu ve kopyası.
- Türetilmiş çizim (kapı-pencere açılımı, oda iç görünüşü, sınırlı 2,5B kesit) ve kesit işaretinden ayrımı.
- Yayın seti ve revizyon kaydının tarif sürümüyle birlikte sınanması; 1:20 detayın PDF'te incelenmesi; gerçek pencerede elle kabul.
