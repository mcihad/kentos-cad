# 1102 — STAC aramasının boş tablosu ne olduğunu söyler

Tarih: 2026-11-20. gis-usage-report.md GIS-045. Durum: kabul edildi.

İlgili: ADR 0644 (STAC'ta ara).

## Sorun

Katalog geçerli ama boş bir FeatureCollection döndürünce tablo ilk açılıştaki “Öğe yok: Ara ile arayın” metnini, alt
satır “Bir öğe seçin” yönlendirmesini korudu; kullanıcı aramanın çalışıp çalışmadığını ve neyi genişleteceğini
anlayamadı.

## Karar

1. Pencere son aramanın nasıl bittiğini tutar (`answered`: yok, yanıtlandı, başarısız). Tablonun boş metni dört
   durumu ayırır: arama yokken “Öğe yok: Ara ile arayın.”, sürerken “Aranıyor…”, yanıtta öğe yoksa “Eşleşen öğe
   yok.”, hata varsa “Arama yapılamadı: nedeni aşağıda.” (neden alt satırda, sunucunun sözüyle).
2. Öğesiz yanıtta tablonun altında sorulan yazılır: koleksiyon, kutu (derece), tarih aralığı ve bulut sınırı; yanında
   “Kutuyu kaldır” (kutu varsa) ve “Tarihleri kaldır” (tarih varsa).
3. Seçilecek öğe yokken “Bir öğe seçin” yönlendirmesi ve boş özet kutusu gösterilmez.

## Kanıt

`apps/desktop/src/gis/rasters/stac.rs` `an_answer_with_no_item_says_what_was_asked`: kayıtlı aramanın öğeleri
boşaltılmış yanıtında “Eşleşen öğe yok.”, sorulanın satırı ve Tarihleri kaldır ekranda; “Bir öğe seçin” yok;
Tarihleri kaldır alanları boşaltır; hata “başarısız” olarak ayrılır. Resim
`.run/gui-evidence/cbs-kullanim/gis-045-stac-bos.png`.
