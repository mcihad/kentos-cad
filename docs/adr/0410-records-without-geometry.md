# ADR 0410: Geometrisiz kayıt: tablo satırı, `cad.records.create`, form ve ilişkilerle

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-N`; `plan/form.md` FORM-REL-08.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0404 (başka katmandan liste), 0405 (ilişkiler), 0409 (çoktan çoğa ve
  yeni alt kayıt), 0265 (taşıma matrisi ve şema kuralı).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Malikler, sayaç okumaları, denetim kayıtları ve hisseler bir yerde durmaz: satırdırlar. QGIS bunu geometrisiz katmanla
(“No geometry” vektör katmanı, ya da CSV, PostgreSQL tablosu) çözer; ilişkinin alt katmanı çoğu zaman böyledir. ArcGIS'te
tablo (`table`) özellik sınıfının yanında durur. KentOS'ta her nesnenin bir şekli vardı: ADR 0409'un “Yeni” düğmesi alt kaydı
üst kaydın yerinde bir nokta olarak çiziyordu, yeri olmayan kayıt için “elle çizilmeli” diyordu.

## Karar

### 1. Nesne türü `record`

- Sözleşmede `Entity::Record(RecordEntity { base })`: kalıcı kimliği, katmanı, öznitelikleri, etiketi ve ekleri vardır; kendi
  alanı yoktur. `Entity::has_geometry` kayıtta yanlıştır.
- Geometri çekirdeğinde `Shape::Record {}`: köşesi, kenarı, kutusu, tutamacı, kenet noktası, etiket yeri yoktur; dönüşüm onu
  değiştirmez; depo onu tutar ama hiçbir yer sorgusu görmez (`Store::shown`: tıklama, pencere, kesişen, çit, çokgen, daire,
  kenet); kapsamı yoktur; paketlenmiş türü 24'tür, çizim kaydı boştur.
- Çizilmez, seçimle çizimde bulunmaz; Öznitelik tablosunda, formlarda, ilişkilerde, Veride ara'da ve ifadelerde (geometri
  değerleri boş) vardır.

### 2. Dosya ve bulut

- `.kcad` şema 60 (formlarla aynı şema: formlar ve kayıt birlikte gelir, bu dal birleşmeden ayrı numara açılmadı):
  `{"record": {…ortak alanlar}}`, kendi alanı yok; şema 59 okuyucusu `unknown_kind` ile reddeder; blok tanımında
  `bad_value`; kendi alanı `unknown_field`. Örnek `fixtures/kcad/v2/records.{json,kcad}`, bozuklar
  `broken/record-{in-schema-59,in-block,with-point}.kcad`; bağımsız `tools/kcad/kcad.py` ve `kcad_v2_reference.py` aynı
  kuralla.
- Tipli sütunlarda tür 20, kendi değeri yok; `FORMATS_VERSION` 69.
- Sunucu: `geom` boş (PostGIS'te `NULL`), kaynak `cad`, tanımı türüyle; göç 0020 nesne türü denetimine `record`'u ve
  ADR 0259'dan beri yazılıp denetimin reddettiği `nurbs`'ü ekler. Blok tanımında kayıt reddedilir.

### 3. Komut: `cad.records.create`

`{layerId, rows: [{attrs?, label?}], expectedRevision?}`: 1–10 000 satır, sırayla, tek geri alma adımı (“Kayıt ekle”). Her
satırın öznitelikleri katmanın alanlarıyla yazılır (ADR 0199 §2, çizim komutlarının kuralı `checks::object_attrs`): tek biçim,
kuralı bozan reddedilir (`invalid_attribute`, `attribute_required`, yeri `rows.<i>.attrs.<ad>`), verilmeyen alana varsayılan;
katmana uygulanan form aynı adımda tamamlar (ADR 0401). Retler sırasıyla `invalid_count`, sürüm, katman, satırların değerleri;
gizli katmanda `layer_hidden` uyarısı. Çıktı `{uids, ids, revision}`; plan yazılacak kayıtları gösterir. Düzenleme
`cad.entities.set`, silme `cad.entities.delete` ile (ilişkilerin silme politikası dahil); Python `kentos.cad.records.create`,
MCP aracı katalogdan.

### 4. Masaüstü

- Öznitelik tablosunda **Kayıt ekle** (`tableRowBelow`): tablonun katmanına boş bir kayıt, seçilir; katmanın formu varsa
  Formla düzenle onunla açılır.
- Formun ilişkili kayıtlar öğesinde **Yeni** (ADR 0409 §4): alt katman kayıt tutuyorsa (ve çizilen nesnesi yoksa) ya da üst
  kaydın yeri yoksa alt kayıt geometrisiz kayıttır; öbür durumda üst kaydın yerinde nokta.
- Öznitelikler'de Tür “Kayıt”; Renk, Saydamlık, Sembol ve Geometri bölümü gösterilmez. Seçime yakınlaş yalnız kayıt seçiliyken
  bunu söyler. Tablonun hücreleri tek satırdır (uzun değer hücrenin kenarında kesilir).

### 5. Dışa aktarma

GeoJSON kaydı `"geometry": null` ile yazar (RFC 7946 §3.2) ve raporda söyler; DXF, KML ve GPX kaydı yazmaz, raporda söyler.

## Denetimler

- `scripts/fixtures/records_command_cases.py --check`: `cad.records.create`'in 25 durumu (`fixtures/commands/v1/
  cad.records.create.json`) KentOS kodu olmadan; masaüstü ve Python koşucuları aynı durumları geçer (`uids` dizisinde `$uid`).
- `kcad_v2_reference.py --check`, kodeğin örnek testi, taşıma matrisi (`tests/carriage.rs`), çekirdeğin `records` testi
  (depoda tutulur, hiçbir yer sorgusu görmez).
- Masaüstü: `form_scenes`'in `a_record_is_added_in_the_table_and_opened_by_its_form` ve
  `many_to_many_owners_are_linked_and_made` (malikler kayıt); resimler `form-kayit-tablosu`, `form-kayit-ekle`,
  `form-bina-malikler`. Python `test_records_without_a_geometry`.

## Sonuçlar

- Tablo satırı kendi türüyle gelir; ilişkinin alt katmanı geometrisiz olabilir, “elle çizilmeli” kalkar.
- Bu dalın şema 60'ı ve `FORMATS_VERSION` 69'u ana daldaki 60–61 ve 70 ile çakışır; birleşmede formlar ve kayıt bir sonraki
  boş şemaya ve sürüme taşınır.
- Açık: tabloda başka katmandan listenin (ADR 0404) etiketiyle gösterim; CSV ve tablo dosyasından kayıtlar (İşlemler).
