# ADR 0416: Öznitelik tablosunda form görünümü

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-W`.
- **Bağlam belgesi:** ADR 0199 §4 (Öznitelik tablosu), 0401 §10 (Formla düzenle), 0408 §3 (kayıtlar arasında gezinme).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

QGIS'in öznitelik tablosunun iki görünümü vardır: tablo ve form (“dual view”): solda kayıtların listesi, sağda seçilen kaydın formu.
Kayıtlar tek tek gözden geçirilip düzeltilirken pencere açıp kapamak gerekmez. KentOS'ta form yalnız Formla düzenle penceresiydi.

## Karar

- Tablo sekmesinin çubuğunda **Tablo | Form** seçimi (`Pane`). Form görünümünde solda tablonun gösterdiği kayıtlar (aynı arama,
  süzgeç ve sıra; ad: etiketi, yoksa ilk değeri; en çok 2000, fazlası söylenir), sağda seçilen kaydın formu.
- Sağdaki form Formla düzenle'nin kendisidir, pencere olmadan (`Entry::docked`): kurallar, hesaplar, listeler, ilişkili kayıtlar, ekler
  ve ◀ ▶ aynı; yazma `cad.forms.submit` ile tek adım. Kaydet ve Vazgeç formu kapatmaz: kayıt yazılır ya da değerler bırakılır, form
  aynı kayıtta kalır. Alt çubuk (sorunlar, Vazgeç, Kaydet) sayfaların altında sabittir.
- Kaydedilmemiş değer varken başka kayıt seçilmez (Formla düzenle'nin ◀ ▶ kuralı). Katman değişince ya da Tablo'ya dönülünce form
  bırakılır. Formu olmayan katmanda “Veri › Formlar'dan bir form oluşturun” denir.
- Form görünümüne geçince seçili nesne (tablonun katmanındaysa), yoksa ilk gösterilen kayıt açılır.

## Denetimler

- Masaüstü: `form_scenes`'in `the_form_view_writes_a_record_and_stays_on_it` (pencere açılmaz, kaydedilmemiş değerle başka kayda
  geçilmez, Kaydet yazar ve formu tutar, Tablo'ya dönünce form bırakılır); resim `form-gorunumu`.

## Sonuçlar

- Kayıtlar tablodan ayrılmadan formla tek tek düzenlenir.
- Açık: listenin adı için katmanın görüntü ifadesi (QGIS'in display expression), formun içinde düzenleme çubuğu kısayolları.
