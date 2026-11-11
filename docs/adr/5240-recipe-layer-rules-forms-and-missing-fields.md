# 5240 — Tarifin katmanının değer bileşimleri, kayıt kuralları, dizinleri ve formları; bulunan katmana eksik alanlar

Tarih: 2026-11-11. plan/map-cad.md MAP-20, onuncu tur. Durum: taslak. ADR 4860'ın “Sonra”sındaki iki iş: katmanın
formları, değer bileşimleri, kayıt kuralları ve dizinlerinin tarife girmesi; adıyla bulunan katmana tarifin eksik
alanlarını ekleme seçeneği. Bellek ve CPU bütçesi gerçek veriyle ölçüm ister; sahibin beklediği işlerdendir, açık kalır.

İlgili:
- [ADR 4860](4860-stop-within-seven-drawings-and-recipe-layer-fields.md): tarifin katmanının alanları, listeleri ve
  alt türleri;
- ADR 0401 (formlar), 0423 (değer bileşimleri), 0438 (kayıt kuralları), 0758 (dizinler).

## Dayanak

11 Kasım 2026'da okundu; not `.run/research/map-cad/uygulama/MAP-20.10.md`, resim `MAP-20.10-resim/`.

- **ArcGIS Pro, Copy** ([belge](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/copy.html)):
  - nesne sınıfına bağlı veriler sınıfla birlikte kopyalanır: ilişki sınıfı, nesneye bağlı açıklama, alan listeleri,
    alt türler ve dizinler;
  - katman kopyalanmaz: katman sınıfa bir başvurudur.
- **ArcGIS Pro, Export Attribute Rules**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/export-attribute-rules.html)):
  kayıt kuralları ayrı bir araçla CSV'ye yazılır, Import Attribute Rules ile başka veri kümesine alınır. Her kural adı,
  türü, alt türü, alanı, betiği ve hata iletisiyle yazılır; kural alanlara adla bağlıdır.
- **Netcad wiki** giriş istiyor; okunmadı (dokuzuncu turun notu: Tablo Düzenle, Lookup Tanımı).
- **Mevzuat:** tarifin katman tanımı için hüküm yok (dokuzuncu turun notu).
- **KentOS bugün** (ADR 4860): tarif katmanın alanlarını, listelerini ve alt türlerini taşır. Değer bileşimlerini,
  kayıt kurallarını, dizinlerini ve formlarını taşımaz. Adıyla bulunan katmanın eksik alanları yalnız söylenir.

## Karar

1. **Tarif katmanın dört tanımını daha taşır** (`road.project.graph`, `RoadRecipeLayer`; ek alanlar, komut sürümü
   değişmez):
   - `contingents`: değer bileşimleri;
   - `rules`: kayıt kuralları;
   - `indexes`: dizinler;
   - `forms`: projenin o katmanı düzenleyen formları, yalnız kimliği, adı ve **taslağı**. Yayımlar, özetleri, zorunluluk
     (`enforce`), yayından kaldırma ve yorumlar tarifin projesinin kararlarıdır; taşınmaz.

   Alanların kendisi gibi alanlara adla (form, alanın kalıcı anahtarıyla) bağlıdırlar.
2. **Tariften üret eklediği katmana bunları da verir** (`road.project.produce`):
   - değer bileşimleri, kayıt kuralları ve dizinler katmanın;
   - formlar projenin formlarına taslak olarak eklenir; katmanı eklenen katmandır. Kimliği bu projede doluysa ilk boş
     `<kimlik>-2`, `<kimlik>-3` … alınır. Yayımlamak bu projenindir;
   - cevapta eklenen katmanın `formsAdded`'i (adlarıyla);
   - `recipe_layer_created` uyarısı söyler: “… 1 değer bileşimi, 1 kayıt kuralı ve 1 dizin de tarifteki gibi. “Sınır
     formu” formu taslak olarak eklendi; yayımlamak bu projenindir.”;
   - **geri alma:** hiçbir çizim gitmediği için geri alınan katman, eklenen formlarıyla da gider.
3. **`addFields`** (yeni seçenek, öntanımı yok): adıyla bulunan ya da `layers` ile verilen katman, tarifin onda olmayan
   alanlarını (adla) kendi alanlarının sonuna alır.
   - Bağladıkları listeler eklenen katmanınkiler gibi gider: aynı adlı aynı değerli liste bağlanır; başka değerli
     liste değişmez, tarifinki `<ad>_1` adıyla eklenir; yoksa eklenir.
   - Katmanın kendi alanları (türü başka olanlar da), alt türleri, değer bileşimleri, kayıt kuralları, dizinleri ve
     formları değişmez. Sessiz birleştirme yok.
   - Cevapta katmanın `fieldsAdded`'i ve `domainsAdded`'i.
   - Yeni uyarı `recipe_layer_fields_added`: “Tarifin “Tür” ve “Zemin” alanları “Sıfır hattı” katmanının alanlarının
     sonuna eklendi; katmanın kendi alanları ve öbür tanımları değişmedi. …”. Türü başka alan `recipe_layer_fields`
     ile söylenmeye devam eder.
   - Eklenen katmanda seçeneğin etkisi yoktur.
4. **Retler** (`invalid_project_recipe`, `path`; çizimden önce):
   - katmanın alanlarına uymayan değer bileşimi, kayıt kuralı ya da dizin (her biri kendi kuralıyla);
   - geçersiz kimlikli ya da adlı form, gövdesi geçersiz form, katmanın alanlarında olmayan bir alana bağlı form;
   - kimliği aynı iki form.
5. **Masaüstü** (İş grafiği › Tariften üret…):
   - planın satırı eklenecek katmanın bileşimlerini, kurallarını, dizinlerini ve formunu söyler;
   - katmanlarında alan olan tarifte **Bulunan katman: tarifin eksik alanlarını ekle** kutusu;
   - işaretlenince plan uyarıyı gelecek zamanla söyler (“eklenecek”);
   - üretilince “… adıyla bulundu: Sıfır hattı. Tarifin eksik alanları eklendi: “Tür”, “Zemin”.”.
6. **Kapsam işareti:** ADR 3640 gibi **Harita + Yol**.

## Sınırlar

- **Bellek ve CPU bütçesi yok:** gerçek veriyle ölçüm ister.
- Formun yayımı taşınmaz. Zorunlu bir form yeni projede zorunlu değildir; yeniden yayımlanmalıdır.
- Bulunan katmana alanlardan başkası eklenmez: bileşim, kural, dizin ve formları birleştirmek kullanıcınındır.
- Alanın türü başkaysa değiştirilmez, söylenir.
- Form gövdesinin anlamı (ifadeler, bileşenler) KentOS'un form çekirdeğinin işidir; tarif yalnız yapıyı ve bağlı
  alanları denetler.
- Gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Kanıt

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/road_project_graph_cases.py`, 1 yeni durum (25): Sıfır hattı'nın listeli Zemin alanı, Tür ve zemin
    değer bileşimi, Derinlik artı kuralı, Ad dizini. Katmanın formu tarifte taslağıyla; başka katmanın formu girmez.
    Eski 24 durum aynı.
  - `scripts/fixtures/road_project_produce_cases.py`, 14 yeni durum (89); tarifler `fixtures/recipes/v1` altında elle:
    - eklenen katman dördünü de alır, form taslak; planı;
    - kimliği dolu form `-2` ile;
    - çizimi gitmeyen katman formuyla ve listesiyle geri alınır;
    - `addFields`: eksik alanlar ve listesi; planı; başka değerli liste `_1` ile; seçeneksiz söylenir; eklenen katmanda
      etkisiz;
    - beş ret.

    Eski 75 durum değişmedi. Bozma denemesi: formun `-2`'si `-3` ve `fieldsAdded`'den bir alan çıkarılınca koşucu tam o
    ikisini yakalar (89'da 2).
- **Python** (`python/tests/test_recipe_layer_rules.py`), testin kendi okumasıyla:
  - tarif dördünü taşır, formun yalnız taslağını;
  - B'de eklenen katman onları alır, form taslak, yayımsız ve zorunsuz;
  - C'de `addFields` eksik üç alanı ve listesini ekler, başka hiçbir şeyi eklemez; seçeneksiz katman değişmez.

  ADR 4860'ın testi ve Python ortak durumları (`road.project`) geçer.
- **MCP** (`crates/native/mcp/tests/recipe_layer_rules.rs`): eklenen form ve uyarının sözleri; grafiğin tarifleri
  dördünü taşır; `addFields`'in `fieldsAdded`'i; alanı olmayan forma bağlı tarifin reddi. ADR 4860'ın MCP testi geçer.
- **Masaüstü** (`recipe_layer_rules_tests`, 2 test ve resim):
  - plan dördünü söyler, Üret katmana ve formlara yazar;
  - kutu işaretlenince plan eksik alanları söyler, Üret bulunan katmana dört alanı verir, form eklenmez.

  Çapraz denetimler 150 test. Resimler `.run/gui-evidence/tarif-katman-tanimlari-5240/` (eklenen, eksik-alanlar; açık
  geniş, koyu dar).

## Sonra

- Bellek ve CPU bütçesi, gerçek veriyle ölçümle (sahibin beklediği).
- Bulunan katmana tarifin bileşimlerini, kurallarını, dizinlerini ve formlarını ekleme seçeneği (sorarak, çakışmada
  ayrı adla).
- Formun yayımının taşınması (bu projede yeniden yayım isteğiyle).
- Web (TODOS §0.2).
