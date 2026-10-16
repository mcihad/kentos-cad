# ADR 0418: Kullanımdan kalkan liste değeri

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-Z`; `plan/form.md` FORM-DATA-15.
- **Bağlam belgesi:** ADR 0199 (katman alanları ve değer listesi), 0401 (formlar), 0408 (çok değerli alan), 0411 §4 (bekçi).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir sınıflama değiştiğinde eski kodu listeden silmek, onu tutan kayıtları kuralına uymaz hâle getirir; bırakmak ise yeni kayıtlarda
seçilmeye devam etmesi demektir. QGIS'in değer haritasında ve ArcGIS'in kodlu alan adında “kullanımdan kalktı” yoktur: değer ya
vardır ya yoktur.

## Karar

### 1. Değer

Değer listesinin öğesinin `retired`'ı (yalnız `true` yazılır):

- değer geçerlidir: onu tutan kayıt kuralına uyar, tablo ve formda etiketiyle görünür;
- yeni seçimde sunulmaz: formun listeleri, Öznitelik tablosunun ve Öznitelikler'in seçicileri onu yalnız tutan kayda “(kullanımdan
  kalktı)” ekiyle gösterir (`LayerField::offered_values`); seçenek süzgeci de böyle (tutan kayda sunulur, süzgeç kuralı bozulmaz);
- formla yeniden yazılamaz: kaydın tutmadığı kalkmış değer formun yazmasında, Kuralları uygula'da ve sunucunun bekçisinde
  `form_value_retired` ile reddedilir (çok değerli alanda her yeni parça); kaydın tuttuğu değer kalır;
- alan kuralları: bütün değerleri kalkmış liste ve kalkmış varsayılan (çok değerlinin parçası da) alanın sorunudur.

### 2. Dosya

`retired` `.kcad` şema 60'tadır; şema 59 okuyucusu `unknown_field` der (`broken/value-retired-in-schema-59.kcad`). Örnek
`forms.json`'un Durum alanında kalkmış bir değer.

### 3. Masaüstünde

Alanlar › Değer listesi'nde **Kalktı** sütunu (ipucunda anlamı).

## Denetimler

- `scripts/fixtures/layer_field_cases.py --check`: dört alan durumu; `scripts/fixtures/forms_cases.py --check`: üç yazma ve iki kayıt
  durumu (sunulan değerler, `form_value_retired`), bağımsız başvurulardan; çekirdekler aynı dosyaları geçer.
- `kentos-kcad` ve `kcad_v2_reference.py --check`.
- Masaüstü: `form_scenes`'in `a_retired_value_stays_where_it_is_and_is_not_written_anew`; resim `form-kalkmis-deger`.

## Sonuçlar

- Sınıflama değişince eski kayıtlar bozulmaz, yeni kayıtlar eski kodu almaz.
- Açık: kalkmış değerin geçerlilik tarihi, kalkmış değeri tutan kayıtları listeleyip toplu değiştirmek.
