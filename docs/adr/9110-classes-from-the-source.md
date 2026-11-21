# ADR 9110: Sınıflar kaynaktan, çift tık, kapalı sınıfın etiketi, boş alan

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-13 (ikinci tur).
- **İstek:** sahibin sözü (21 Kasım 2026): “Hepsini düzelt eksikleri tamamla.” Kapatılan kalanlar:
  - ADR 9101 §10: stil penceresinin sınıflandırması sürücüden okunmuyor.
  - ADR 9105 §6: kaynakta gruplanmış sayım yok; sınıf satırında çift tık yok; çizim katmanının etiketleri kapalı
    sınıfa bakmıyor.
  - ADR 9103 §8: değeri boş alan etiket şablonunda yer tutucusuyla görünüyor.
- **Sözleşmeler:** komut ve `.kcad` değişmedi. Etiket anahtarı (`texts_key`) çiziciyi de içerir. `LayerTexts::Classes`
  katmanın alan adlarını taşır; `fill_template_with` onları alır. **Bağımlılık:** yok.

## §1 Karar: stil penceresi veri katmanının değerlerini kaynaktan okur

**Sorun:** stil penceresi değerleri çizimin nesnelerinden alıyordu; veri katmanının hiçbir satırı çizimde değil.
Bir milyon parsellik katmanda:

- Değerlerden sınıfla “Bu ifade nesnelerde değer vermiyor” diyordu. Bir PostGIS katmanı arayüzden sınıflandırılamıyordu.
- Nesne sütunu her sınıfta 0 gösteriyordu.
- Başlık “0 nesne: çizilecek nesne yok” yazıyordu.

**Masaüstü CBS:** QGIS'in Sınıflandır düğmesi ayrık değerleri sağlayıcıdan alır. Nesne sayısını ise nesneleri gezerek
kendisi sayar.

**Karar:**

- Veri katmanında Değerlerden sınıfla ve Sınıfla, alanın değerlerini kaynaktan okur (`Source::groups`, ADR 9108 §3).
  Okuma arayüzün iş parçacığında yapılmaz. Kategoriler, çizim katmanınınki gibi Türkçe ve sayı duyarlı sırayla
  dizilir; var olan renkler korunur.
- Sınıfla, aralıkları değerlerin satır sayılarıyla tartılmış dağılımından bulur. Alanın 200 000'den çok farklı değeri
  varsa sütunun sayıları baştan sona okunur.
- Pencere açılınca kategorili ya da aralıklı stilin alanı okunur. Nesne sütunu kaynağın satır sayılarını gösterir
  (`category_tally_grouped`, `class_tally_grouped`). Bir grup, aynı değerdeki o kadar nesne sayılır; bu, nesne
  başına sayımla aynı sonucu verir. Başlık “1.000.000 satır kaynakta” der.
- Sınıflandırma yalnız tablonun bir alanıyla yapılır. İfade, satırları bütün olarak getirmeyen kaynakta sorulamaz;
  pencere bunu söyler (“Veri katmanı tablosunun bir alanıyla sınıflandırılır…”).
- Veri katmanında varsayılan alanlar: kategoriler için ilk metin alanı, aralıklar için ilk sayı alanı (`$alan`
  yerine).

## §2 Karar: katman ağacının sınıf sayımı kaynakta

- **Kategorili stil bir alana göre çiziyorsa:** tek bir `GROUP BY` yapılır. Değer ilk kategorisine gider, kalanı
  “Diğer değerler”e. 100 000'den çok değer varsa satırlar okunur.
- **Aralıklı stil** (sınıfları üst üste binmiyorsa) ve **kurallar** (“değilse” kuralı yoksa): her sınıfın ifadesi
  kaynağa tam gidiyorsa (ADR 9109) her sınıf kendi koşuluyla sayılır.
- Öbür durumlarda satırlar eskisi gibi okunur ve burada sayılır. GeoPackage'ta sayı koşulları tam olmadığı için
  aralıklı stil ve kurallar okunarak sayılır; kategorili stil orada da gruplanır.

## §3 Karar: sınıf satırına çift tık sembolü açar

- QGIS'te katman ağacındaki sınıfa çift tık o sınıfın sembol düzenleyicisini açar.
- KentOS'ta sınıf satırının basışı kendi olayıdır (`Pressed`). Tek basış katmanın satırını seçer, katmanı etkin
  yapmaz. Aynı sınıfa ikinci basış Katman stili'ni açar ve sembol tasarımcısını o sınıfın yuvasında, katmanın
  geometrisine göre (nokta, çizgi, alan) açar. Uygula sembolü pencereye koyar, Tamam stili yazar (tek geri alma
  adımı).
- Katman satırına sonra sınıf satırına basmak çift tık sayılmaz.

## §4 Karar: çizim katmanında kapalı sınıfın nesnesi etiketlenmez

- Veri katmanında bu VY-07'den beri böyleydi. Çizim katmanında etiket motoru katmanın bütün nesnelerini alıyordu.
- Artık stilin çizdiği ama sınıfı kapalı (ya da sınıfsız) nesnenin etiketi yazılmaz.
- Etiket anahtarı çiziciyi de içerir: bir sınıf açılıp kapanınca etiketler yeniden yapılır.

## §5 Karar: değeri boş alan etikette boş

- Şablonda katmanın bir alanını anan `{alan}`, nesnede değer yoksa boş yazılır: “{ada}/{parsel}” parsel numarası
  olmayan parselde “101/” olur.
- Ne alanı ne özniteliği anan `{…}` yazıldığı gibi kalır: yanlış yazılmış alan görünsün diye.

## §6 Kanıt

**Ölçüm:** bir milyon PostGIS parseli, `cad.layers.classes` sayımı.

| Stil | Kaynakta | Okuyarak (gönderilemeyen eşdeğeri) | Sayılar |
|---|---|---|---|
| Kategorili `kullanim` (dört sınıf ve Diğer) | 564 ms | 8 149 ms | aynı (250 000'er) |
| Aralıklı `alan` (üç sınıf) | 116 ms | 7 987 ms | aynı (242 984, 333 204, 423 812) |

**Gerçek pencere:**

- Önce: Parseller'in stil penceresi “0 nesne: çizilecek nesne yok” diyordu; her sınıfın Nesne sayısı 0'dı.
- Sonra: Konut sınıfının satırına çift tık “Konut: alan sembolü” tasarımcısını açtı. Esc ile dönülen pencere
  “1.000.000 satır kaynakta” ve sınıf başına 250000 gösterdi. Değerlerden sınıfla kategorileri kaynaktan Türkçe
  sırayla yeniden kurdu (Eğitim, Konut, Park, Ticaret) ve renkleri korudu.
- Görüntüler `.run/gui-evidence/veri/vy13/{kesif,sonra}`.

**Sınamalar:**

- `kentos-native-style`: gruplanmış sayım, açılmış listeyle aynı.
- `layer_classes_source_tests`: aynı satırlar yalıtılmış PostGIS'te kaynakta, GeoPackage'ta okuyarak sayıldı;
  kategorili, aralıklı ve kurallı stil aynı sayıları verdi.
- `source_values_tests`: veri katmanında Sınıflandır ve Sınıfla kaynaktan; Nesne sayıları (21 Konut, 20 Park); alan
  olmayan ifade için pencerenin sözü.
- `layer_classes_tests`: çift tık tasarımcıyı sınıfın yuvasında açar; tek basış yalnız seçer; kapalı sınıfın
  nesnesi etiketsiz, açık ve Diğer etiketli.
- `label_texts`: bilinen alan boş, bilinmeyen ad görünür.
- `cad.layers.classes` durumları ve bütün ortak durumlarla yetenek denetimi geçti. Masaüstünün etiket, stil, veri ve
  katman testleri geçti (263).

## Uygulama

- Stil: `crates/native/style/src/tally.rs`.
- Masaüstü: `apps/desktop/src/cad/style/layer_style/source_values.rs` (yeni), `mod.rs`, `panels.rs`;
  `apps/desktop/src/gis/layer_classes.rs` (`Pressed`, `edit_class_symbol`); `apps/desktop/src/cad/layering.rs`
  (`row_chosen`).
- Uygulama: `crates/native/application/src/gis/layer_classes.rs` (`source_counts`, `overlapping`).
- Etiket: `crates/native/scene-query/src/spatial.rs`, `label_texts.rs`; `crates/shared/geometry-core/src/ops/label_text.rs`.
- Sürücü sözleşmesi: `crates/native/data/src/conformance.rs` (`new_features` açıldı).
