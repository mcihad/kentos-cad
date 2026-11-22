# ADR 9127: CBS projesi binanın komutlarını sunmaz; CBS şeridinden CAD'e özgü öğeler çıktı

- **Durum:** kabul edildi (2026-11-22); adım VY-26. Plan: [plan/veri.md](../../plan/veri.md) VY-26.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  12. maddesi (P1), “komut aramasında doğrulanmış sızıntı” ile:
  - harita projesinde genel aramaya `duvar` yazıldı, dokuz Mimari sonucu çıktı;
  - ilki seçilince Yol ekseni katmanında “Duvar · ilk noktayı belirtin” başladı;
  - CBS'nin Dosya › Dışa aktar listesinde iki bina IFC komutu, Görünüm'de UCS komutları, Analiz'de ilgili veri yokken
    nokta bulutu araçları vardı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- **Modül programları** (ADR 0802): bir modülün programı yalnız kendi komutlarını sunar (`App::offers`):
  - Menülerde ara, komut satırı, kısayollar, hızlı erişim, F1 ve çizimin menüleri yalnız onları bulur;
  - `App::run` öbürlerini reddeder.
- **Bütün modüllü program** (kentos-cad) her komutu her projede sunuyordu. Arama sonucunu süzen ve komutu çalıştıran
  yol aynı karara (`offers`) bakıyordu; o da her şeye evet diyordu.
- **İlk deneme:** bütün modüllü programda CBS projesinin KentOS CBS programı gibi davranması (komutlar, paneller,
  hücreler, araç seçenekleri). Bütün masaüstü sınamaları bunun fazla geniş olduğunu gösterdi:
  - kentos-cad'de CBS projeleri Harita modülünün komutlarını da kullanıyor: Yatay ağ dengelemesi, CBS türetme ve
    denetimi;
  - çoklu çizginin Genişlik seçeneği de kullanılıyor.

  Bu yüzden kural, raporun söylediği yere daraltıldı: bina.

## §2 Karar

1. **Yalnız binanın komutu.** Bütün modüllü programda bir CBS projesi açıkken bir komut yalnız KentOS Mimari'nin
   şeridindeyse sunulmaz (`App::building_only_here`). Başka hiçbir modülün (Harita, CBS, Yol) şeridi onu tutmaz.
   - Harita'nın, CBS'nin ve yolun komutları yerinde kalır; kentos-cad'in yüzeyleri (paneller, hücreler, araç
     seçenekleri) değişmedi.
   - Karar `offers`'tadır. Menülerde ara, komut satırı, kısayol, hızlı erişim, F1 ve menüler aynı kararı kullanır;
     `run` aynı kararla reddeder.
   - Ret iletisi yerini söyler: ““Duvar” bu CBS projesinde yok; KentOS Mimari'de var.”
2. **CBS şeridi** (`catalog/v1/ribbon.json`, `tabs.gis`):
   - Dosya › Dışa aktar'ın “Bina modeli” bloğu (IFC ve kesitli IFC) CBS'den çıktı. Blok yalnız mimari işin CAD
     projesindedir.
   - Görünüm'ün “Koordinatlar” paneli (Dünya UCS, Önceki UCS, nesneye, iki noktayla, orijinle, döndürerek, adlı UCS,
     UCS'e döndür) CBS'den çıktı. Kullanıcı koordinat sistemi CAD'in kavramıdır.
   - Analiz › Arazi'de nokta bulutu araçları (bölge, örtüşme, kayıt, ağ, yerleştir, profil) yoktur. Bir nokta bulutu
     seçilince açılan Nokta bulutu bağlamsal sekmesindedirler, Menülerde ara onları bulmaya devam eder. Arazi'de Kot
     noktası ve Yüzey sorgusu kalır.

## §3 Kanıt

- **Masaüstü** (`tests/gis_surface_tests.rs`):
  - CBS projesinde `duvar` aranınca Duvar ve hiçbir Mimari komutu çıkmıyor;
  - Duvar yine de çalıştırılınca reddediliyor ve ileti KentOS Mimari'yi söylüyor;
  - CBS'nin Dosya, Görünüm ve Analiz sekmelerinde IFC, UCS ve nokta bulutu araçları yok. Nokta bulutu profili sunulmaya
    devam ediyor, aramada bulunuyor; IFC sunulmuyor;
  - CAD projesinde `duvar` Duvar'ı buluyor.
- **Bütün masaüstü kitaplığı:** 2918 geçti, 2 düştü.
  - `python::tests::every_program_opens_a_link_of_its_own` bu makinenin ortamından düşüyor: sınamanın soket yolu 108
    bayt, macOS'un 104 baytlık sınırını aşıyor. Bu değişiklikle ilgisi yok.
  - `traces::tests::every_trace_passes_in_every_variant` tek başına geçiyor.

## §4 Kalan

Raporun 12. maddesinin kalanı:

- proje türü, yetki, etkin katmanın geometrisi, seçim ve sağlayıcı yeteneği için görünürlük matrisi;
- CBS'nin alt şeridindeki CAD terimleri (Kalınlık, Model sekmesi) ve dar pencerede taşan durum çubuğu (ADR 9123 §4);
- raster ve ağ işleme için bağlamsal araçlar;
- web (TODOS.md §0.2).
