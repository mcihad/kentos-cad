# ADR 0783: Görünüş şablonu ve kalem seti

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-06'nın dördüncü teslimi **[~]**.
- **Bağlam:** ADV-06 (“Mimari görünüşler, proje gezgini ve koordineli yayın”), dayanak A8–A9. Görünüş tarifi ADR 0774, duvar
  ayrıntısı ve kırpma ADR 0781, proje gezgini ADR 0778, haritanın evre ve gizlenen türleri `kentos_plot::layers::map_phasing_of`.
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'de.
- **Şema:** `.kcad` şema 146 (opt-in): görünüş şablonu olan ya da tarifinde kalem, siyah beyaz veya şablon bulunan çizim 146 yazar.
  `FORMATS_VERSION` değişmez (yeni alanlar proje ayarlarının JSON'undadır).

## Araştırma (üreticilerin resmî belgeleri)

- Revit, [Apply a View Template](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Customize/files/GUID-DA062846-32C9-4FFF-9BC1-BBB548868ACA.htm):
  şablonun özellikleri görünüşe bir kez uygulanabilir ya da şablon görünüşe atanır; atanmış şablonun sonraki değişikliği bağlı
  görünüşlere kendiliğinden geçer. [View Template Properties](https://help.autodesk.com/cloudhelp/2020/ENU/RevitLT-Customize/files/GUID-66D789A8-AB92-4AF0-B884-DF3A3302E117.htm):
  şablon ölçek, ayrıntı, görünürlük/grafik geçersiz kılmaları, evre süzgeci gibi alanları taşır; görünüşün kendi alanları
  (kırpma bölgesi, kat) şablonda değildir.
- Archicad, [Edit and manage Pen Sets](https://help.graphisoft.com/AC/28/INT/_AC28_Help/025_Attributes/025_Attributes-32.htm):
  kalem seti kalemlerin kalınlığını ve rengini topluca verir; kayıtlı görünüm kendi kalem setiyle çizer. Graphic Override
  kuralları öğeleri ölçütleriyle seçip görünüşlerini o görünümde değiştirir.
- Sonuç: KentOS şablonu adlı bir tarif olarak projede tutar; görünüm ona bağlanır ve şablon değişince bağlı her harita yeniden
  çizer (Revit'in atanmış şablonu). Kat ve kırpma görünümündür. Kalem seti Archicad'in kalem seti ile Revit'in kategori
  geçersiz kılmasının sadeleşmişidir: nesne türü kategorileri, kâğıt kalınlığı ve renk; siyah beyaz çıktı için tek anahtar.

## Karar

### §1 Kalemler ve siyah beyaz

- `ViewRecipe.pens`: kategori başına bir kalem (`of`: walls, openings, columns, stairs, slabs, spaces, dimensions, tags, texts,
  hatches, blocks, grids, viewmarks, others), `weight` (kâğıtta 0–5 mm) ya da `color` (`#rrggbb`) ya da ikisi. Kategori nesne
  türünden (`PenCategory::of_kind`: merdiven ve rampa stairs, yazı ve kılavuz texts, yerleştirme blocks …).
- `ViewRecipe.mono`: haritanın bütün çizgileri siyah, taramaları gri (#808080); kalemin rengi siyah beyazdan önce gelir.
- Kalem nesnenin kendi kalınlığının ve renginin yerine geçer; neyin gösterildiğini değiştirmez (`cad.sheets.set`'in `shows`'u aynı).
- Kalem ve siyah beyaz `map_phasing_of`'un `Phasing`'ine kimlik başına görünüş olarak eklenir; ekran, başsız kitap ve PDF aynı
  yoldan çizer. Model ve ekranın kendi görünüşü değişmez.

### §2 Şablon

- Projenin `settings.viewTemplates`'i (en çok 50): kimlik (`t-` ve adın FNV-1a'sı, alınmışsa `-2`, `-3` …), ad, tarif (sürümlü).
  Şablonun tarifi kat, kırpma ya da şablon taşımaz (`invalid_recipe`).
- `cad.views.edit`'in `template` işlemi şablonu kaydeder ya da değiştirir (sürüm bir artar; aynı tarif değişiklik değildir);
  tarifsiz işlem kaldırır, ona bağlı görünümler şablonun alanlarını kendi tarifleri olarak tutar (sürümleri bir artar)
  ve `template_detached` uyarısı. `templateRename` kimliği koruyarak adlandırır (`unknown_template`, `template_exists`).
  Elli birinci şablon `too_many_templates`.
- Görünümün tarifi `template`'le bağlanır (kimlik ya da ad; kimlik olarak tutulur, yoksa `unknown_template`); bağlı tarifte
  kat ve kırpma dışında alan yoktur (`invalid_recipe`).
- Çözüm tek yerde: `resolved_named_views(settings)` bağlı görünümün tarifini şablonun alanları, görünümün katı ve kırpmasıyla
  verir. Pafta bağlamı, başsız kitap, `cad.sheets.set` ve proje gezgini bundan okur: şablon değişince bağlı her harita aynı
  adımda yeniden çizer.

### §3 Masaüstü

- Görünümler penceresinin Görünüş tarifi bölümünde **Şablon** (Yok ya da projenin şablonları). Şablon seçilince evre, katman
  durumu, ölçek, ayrıntı, gizlenenler ve kalemler şablondan gelir, pencere onları “Şablondan” satırında söyler; kat ve kırpma
  görünümün kalır. Yok'a dönmek şablonun alanlarını görünümün kendi tarifi yapar (Revit'in şablonu kaldırınca görünüşün
  değerlerini tutması gibi).
- **Siyah beyaz** ve 14 kategorinin kâğıtta kalem kalınlıkları (Kendi, 0,13 … 1,0 mm; ISO 128'in dizisi); rengi olan kalemin
  rengi kalınlık değişince korunur.
- **Şablon olarak kaydet**: ad ve düğme; taslağın çizdiği (şablona bağlıysa o şablonun alanları), kat ve kırpma dışında,
  `cad.views.edit`'in `template`'iyle kaydedilir.

### §4 Dosya

- `.kcad` şema 146: `settings.viewTemplates` ve tarifin `pens`, `mono`, `template`'i. Daha eski şemada bunlardan biri reddedilir.
  Bağımsız Python okuyucusu ve yazıcısı aynı kuralla.

## Doğrulama

- `python3 scripts/fixtures/view_template_cases.py --check`: `cad.views.edit`'in 29 şablon ve kalem durumu
  (`fixtures/commands/v1/cad.views.edit.template.json`), `cad.sheets.set`'in şablonla çizen 9 durumu
  (`cad.sheets.set.template.json`); kural ADR'den, KentOS kodu olmadan. `view_recipe_cases.py --check` (33) değişmeden geçer.
- `KENTOS_CASES_ONLY=cad.views,cad.sheets,cad.navigator cargo test -p kentos-native-application --test all every_command_case`;
  `kentos-plot` (haritanın kalemleri ve siyah beyazı `map_phasing_of`'ta), `kentos-native-style` `phases`.
- `.kcad`: `view-templates.kcad`, `broken/view-templates-in-schema-145.kcad` ve beş bozuk örnek
  (`pen-twice`, `pen-empty`, `with-level`, `missing`, `own-scale`) bağımsız Python yazıcısıyla; `tools/kcad/kcad.py` aynı kuralı
  okur (`kcad_v2_reference.py --check`, 1432 dosya); `cargo test -p kentos-kcad --test all`.
- Masaüstü: `cargo test -p kentos-desktop --lib view_template` (iki bağlı harita tek şablon değişikliğiyle birlikte siyah
  beyaz olur; pencere şablona bağlanır, bırakır, kalem ve siyah beyaz yazar, taslağı şablon olarak kaydeder). Resimler
  `KENTOS_SHOTS_ONLY=gorunus-sablon-pencere,gorunus-sablon-kalem,gorunus-sablon-pafta cargo test -p kentos-desktop --lib
  tools_screens::screens -- --ignored --nocapture` (`.run/shots/arac-gorunus-sablon-*`, 1440×900 ve 1100×650, koyu ve açık).

## Sınırlar

- Pafta kipinde ekrandaki etiketler kalemle ve siyah beyazla yeniden renklenmez (çizgiler ve taramalar renklenir; PDF ve başsız
  kitap da aynı); etiket katmanının görünüşü ayrı iştir.
- Kalem nesne türüne göredir; katman ya da öznitelik koşuluyla geçersiz kılma (Archicad'in Graphic Override kuralları) yok.
- Şablon değişikliğinin hangi paftaları etkileyeceği önizlenmez; `cad.sheets.set`'in `shows`'u yeniden okunarak görülür.
- Gerçek pencerede elle denenmedi.
