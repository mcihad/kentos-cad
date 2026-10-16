# ADR 0422: Alt türler

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AD`; `plan/form.md` FORM-DATA-17, FORM-DATA-18.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0401 (formlar), 0414 (alanın kullanıldığı yerler), 0418 (kullanımdan kalkan değer).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir katmandaki kayıtlar çoğu zaman türlere ayrılır (bina: konut, sanayi, depo; boru: içme suyu, atık su) ve bir alanın anlamlı
değerleri türe göre değişir: sanayi binası ahşap olmaz, atık su borusunun basınç sınıfı yoktur. ArcGIS bunu alt türle (subtype)
çözer: bir tamsayı alan kayıtları türlere ayırır, her tür alanların etki alanını ve varsayılanını değiştirir. QGIS'te karşılığı
yoktur; ancak formun “süzgeçli değer ilişkisi” ve her alana ayrı ifadeyle yaklaşılır, kural veride değil formda kalır.

## Karar

### 1. Model

Katmanın `subtypes`'ı (`kentos_contracts::LayerSubtypes`): alt tür alanı (`field`, alanın anahtarı; değer listeli, tek değerli bir
alan) ve alt türler (`types`, en çok 256): her biri listenin bir kodu (`code`), adı (`label`; boşsa listedeki adı) ve alan kuralları
(`fields`): alanın anahtarı, sunulan değerler (`values`, listeli alanda; yoksa hepsi), yeni kaydın varsayılanı (`default`) ve
zorunluluk (`required`). Kuralları `subtypes_problem`: alt tür alanı ve kodları, adlar, kuralın alanı (alt tür alanının kendisi
olmaz, bir kez), değerler listenin değerleri ve boş olmaz, varsayılan alanın yazımıyla ve alt türün sunduğu değerlerden.

Alanlar değişince alt türler izler (`follow_subtypes`): giden alanın kuralları, listeden çıkan kodlar, artık uymayan varsayılan
düşer; alt tür alanı giderse alt türler kalkar. Alt türler alanları anahtarla anar: yeniden adlandırılan alanın anahtarı kalır
(ADR 0414 §2), Alanın kullanıldığı yerler onları da sayar.

### 2. Kayıtlar

Bir kaydın alt türü alt tür alanındaki değerdir; alanı o türün gözüyle görmek (`LayerSubtypes::field_for`) listeyi daraltır,
varsayılanı ve zorunluluğu değiştirir.

- **Yazma** (`cad.entities.set`, çizen komutlar, `cad.entities.create`): yeniden yazılan (öncekinden farklı) bir değeri alt tür
  sunmuyorsa `invalid_attribute`, alt türün zorunlu tuttuğu alan boşaltılıyorsa `attribute_required`. Tür değişince ya da başka
  alan yazılınca kalan uyumsuz değerler silinmez: `subtype_misfit` uyarısıyla söylenir (FORM-DATA-18: geçerli değer gereksiz
  silinmez, karar kullanıcınındır).
- **Yeni kayıt:** önce alt tür alanının kendi varsayılanı, sonra her alana alt türünün varsayılanı, yoksa alanınki — alt tür onu
  sunuyorsa.
- **Form:** alt türün daralttığı alanın seçenekleri alt türün değerleridir (kaydın tuttuğu değer kalır), alt türün zorunlu tuttuğu
  alan zorunludur, yeni kayıtta formun kendi varsayılanı olmayan alan alt türünkini alır. Yeniden yazılan uyumsuz değer, ya da tür
  değiştiğinde kalan her uyumsuz değer `form_value_not_in_subtype` sorunudur. Formla düzenle'de “Kaydın alt türü şu değerleri
  sunmuyor” şeridi ve **Uymayanları boşalt**.
- Öznitelik tablosu ve Öznitelikler'in seçimleri de kaydın türüne göre daralır; türün sunmadığı tutulan değer “(alt türde yok)”
  diye gösterilir.

### 3. Komut

`cad.layers.subtypes` v1 (“Alt türler”): `layer`, `subtypes` (yoksa kaldırır), `expectedRevision`; tek geri alma adımı, aynıysa
yazılmaz. Retler: `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `service_layer`, `invalid_subtypes`
(`subtypes/<yer>`). Python `kentos.cad.layers.subtypes`, MCP'de araç (yıkıcı listede).

### 4. Dosya

`.kcad` şema 60'ın katman düğümünde `subtypes` (JSON'u olduğu gibi, alanlarla birlikte denetlenir). Bağımsız okuyucu `kcad.py`
kuralı ayrıca yazar; örnek `forms.json`'da, kırık örnekler `subtypes-in-schema-59`, `subtypes-unfit`, `subtypes-sorter-many`.

### 5. Masaüstü

Veri › Form'da **Alt türler** (`data.subtypes`): katman ve alt tür alanı; listenin değerleri işaretlenerek alt tür olur; seçilen
türde ad ve her alanın satırı: Zorunlu, Varsayılan ve sunulan değerlerin çipleri, Tümü. Kaydet komutla yazar ve pencereyi kapatır.

## Denetimler

- `scripts/fixtures/subtypes_cases.py --check`: `cad.layers.subtypes` (30), `cad.entities.set.subtypes` (21),
  `cad.polygon.create.subtypes` (21) ve `cad.forms.validate.subtypes` (21) durumları KentOS kodu olmadan; formunkiler
  `forms_cases.py`'nin form ikizinin alt türleri bilen alt sınıfıyla.
- `kentos-contracts`'ın `subtypes` testi; `kcad_v2_reference.py --check` ve `kentos-kcad`.
- Masaüstü: `the_subtypes_window_writes_through_its_command_and_closes`, `a_changed_kind_keeps_its_values_until_they_are_emptied`;
  resimler `form-alt-turler`, `form-alt-tur-degisti`.

## Sonuçlar

- Türe bağlı değer kuralları veride durur: form, tablo, Python ve ajan aynı kuralı uygular.
- Açık: alt türe göre katman stili ve etiket (FORM-DATA-17'nin “style” kısmı), koşullu alan grupları (FORM-DATA-19), alt tür
  alanı tamsayı kodlu ArcGIS verisinin içe ve dışa aktarılması.
