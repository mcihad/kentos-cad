# ADR 0423: Değer bileşimleri

- **Durum:** kabul edildi (2026-10-17). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle getirene kadar
  sürdürmek. TODOS.md §16.6 `FORM-AE`; `plan/form.md` FORM-DATA-19.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0401 (formlar), 0414 (alanın kullanıldığı yerler), 0422 (alt türler).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bazı alanların değerleri tek tek geçerli olsa da birlikte geçersizdir: çelik bina kırma çatıyla, PE boru 300 mm çapla yapılmaz.
ArcGIS bunu “contingent attribute values” ile veride tutar: alan grupları ve birlikte geçerli değerlerin listesi, kısıtlayıcı ya
da yalnız uyarıcı. QGIS'te karşılığı formun süzgeçli değer ilişkisidir; kural veride değil formdadır, tabloda ve betikte uygulanmaz.

## Karar

### 1. Model

Katmanın `contingents`'ı (en çok 50 grup, `kentos_contracts::FieldGroup`): grubun adı (katmanda bir kez), alanları (2–8, değer
listeli ve tek değerli, anahtarlarıyla), kısıtlayıcı olup olmadığı (`restrictive`) ve bileşimleri (`rows`, 1–10 000): grubun her
alanı için bir hücre (`{"code": …}`, `"any"` herhangi bir değer — boş da —, `"empty"` boş) ve isteğe bağlı alt tür (ADR 0422:
bileşim yalnız o türün kayıtları için). Kuralları `contingents_problem`. Alanlar ya da alt türler değişince gruplar izler
(`follow_contingents`): alanı giden ya da listesiz kalan grup, listeden çıkan kodu ya da giden alt türü anan bileşim düşer.
Gruplar alanları anahtarla anar; yeniden adlandırmada anahtar kalır, Alanın kullanıldığı yerler onları sayar.

### 2. Kayıtlar

Bir kayıt, grubun alanlarından en az birini dolduruyorsa grubun (türüne uyan) bileşimlerinden birine uymalıdır.

- **Yazma** (`cad.entities.set`, çizen komutlar, `cad.entities.create`): grubun alanlarından birini değiştiren yazma (yeni kayıtta
  doldurduğu her alan) bileşime uymuyorsa kısıtlayıcı grupta `invalid_combination` (ilk değişen alanın yolunda), uyarıcı grupta
  `combination_unmet` uyarısı. Grubun alanlarına dokunmayan yazma sorulmaz: eski kayıtlar kendiliğinden geçersiz sayılmaz.
- **Form:** kısıtlayıcı grup alanın seçeneklerini, kaydın öbür değerleriyle bir bileşimin izin verdiği kodlara daraltır (kaydın
  tuttuğu değer kalır). Bileşim yoksa `form_combination` (kısıtlayıcıda hata, uyarıcıda uyarı), ilk değişen alanın öğesinde.
- Öznitelik tablosu ve Öznitelikler'in seçimleri de daralır; bileşimin izin vermediği tutulan değer “(bileşimde yok)” diye görünür.
- İleti değerleri listedeki adlarıyla söyler: “Malzeme ve çatı” değer grubunda bu bileşim yok: Malzeme = Çelik, Çatı = Kırma.

### 3. Komut

`cad.layers.contingents` v1 (“Değer bileşimleri”): `layer`, `groups` (boş liste kaldırır), `expectedRevision`; tek geri alma adımı.
Retler: `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `service_layer`, `invalid_contingents`
(`groups/<yer>`). Python `kentos.cad.layers.contingents`, MCP'de araç (yıkıcı listede).

### 4. Dosya

`.kcad` şema 60'ın katman düğümünde `contingents` (JSON'u olduğu gibi; alanlar ve alt türlerle denetlenir). Bağımsız okuyucu
`kcad.py` kuralı ayrıca yazar; örnek `forms.json`'da, kırık örnekler `contingents-in-schema-59`, `contingents-unfit`,
`contingents-empty-row-list`.

### 5. Masaüstü

Veri › Form'da **Değer bileşimleri** (`data.contingents`): katman; gruplar listesi ve Yeni grup; seçilen grupta ad, Kısıtlayıcı,
alanların çipleri (sırası sütunlardır) ve bileşimler tablosu (her hücre Herhangi, Boş ya da listenin değeri; alt türü olan katmanda
Alt tür sütunu); Bileşim ekle. Kaydet komutla yazar ve pencereyi kapatır.

## Denetimler

- `scripts/fixtures/contingents_cases.py --check`: `cad.layers.contingents` (24), `cad.entities.set.contingents` (21),
  `cad.polygon.create.contingents` (21) ve `cad.forms.validate.contingents` (21) durumları KentOS kodu olmadan; formunkiler form
  ikizinin alt türleri ve grupları bilen alt sınıfıyla.
- `kentos-contracts`'ın `contingents` testi; `kcad_v2_reference.py --check` ve `kentos-kcad`.
- Masaüstü: `the_contingents_window_writes_through_its_command_and_closes`,
  `a_value_group_narrows_the_choices_and_refuses_a_combination`; resimler `form-bilesimler`, `form-bilesim-sorunu`.

## Sonuçlar

- Birlikte geçerli değerler veride durur; form, tablo, Python ve ajan aynı kuralı uygular.
- Açık: aralık etki alanlı (sayı) alanlar gruba girmez; büyük bileşim tablosunun CSV'den alınması; ArcGIS'in contingent values
  verisinin içe ve dışa aktarılması.
