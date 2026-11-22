# ADR 9143: Katman stili uyan işleyicileri önce sunar; kategoriye girmeyen nesneleri söyler

- **Durum:** kabul edildi (2026-11-22); adım VY-42. Plan: [plan/veri.md](../../plan/veri.md) VY-42.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  10. maddesi (P1):
  - yol çizgi katmanında semboloji menüsü yoğunluk, kümeleme ve ters alan gibi seçenekleri de gösterdi;
  - kategorili seçilince sınıflandırmadan önce bir öğenin çizilmeyeceği yazıyordu ama Tamam etkindi;
  - kategoriler eksikse görünmeyen kayıt sayısı, “diğer değerler” sembolü ve yeniden sınıflandırma sunulmalı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Katman stili penceresi (ADR 0226) bütün işleyicileri katmanın geometrisine bakmadan aynı sırayla listeliyordu.
- Kategorili işleyicide “Diğer değerler” satırı vardı: işareti, sembolü ve kaç nesne olduğu. Kapalıyken bu nesneler
  çizilmiyordu ama pencere bunu ve çaresini ayrıca söylemiyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasının semboloji sayfası katmanın geometri türüne uyan işleyicileri listeler:
  ısı haritası ve nokta yerleşimi yalnız nokta katmanında, ters çokgen yalnız alan katmanında çıkar. Kategorili
  işleyicide “tüm diğer değerler” sınıfı ve Sınıflandır düğmesi vardır.
  Kaynak: [vektör katman özellikleri](https://documentation.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html).

## §2 Karar

1. **Uyanlar önce.** Pencere katmanın geometrisini bilir: katmanın türü, yoksa veri katmanının tablosunun geometrisi.
   Uyanlar listenin başında, eski sırasıyla gelir. Uymayanlar “Bu katmanın geometrisine uymayan” başlığı altında en
   sonda durur; seçilebilirler, katman karışıksa saklanmazlar.
   - Çizgi katmanına uymayan: ısı haritası, kümeleme, yayma, nokta yoğunluğu, grafik, ters alan.
   - Nokta katmanına uymayan: nokta yoğunluğu, ters alan.
   - Alan katmanına uymayan: ısı haritası, kümeleme, yayma.
   - Geometrisi bildirilmemiş ya da karışık katmana hepsi uyar.
2. **Kategoriye girmeyenler.** Hiçbir kategoriye girmeyen nesne varken Diğer değerler kapalıysa tablonun altında bir
   uyarı çıkar: “1 nesne hiçbir kategoriye girmiyor ve çizilmez: Diğer değerleri açın ya da değerlerden yeniden
   sınıflayın.” Yanında iki düğme vardır:
   - “Diğer değerleri aç”;
   - “Değerlerden sınıfla”: Değer satırındaki düğmenin aynı işi, aynı adla.

   Tamam etkin kalır; uygulanan, gösterilenin aynısıdır.

## §3 Kanıt

- **Masaüstü** (`cad/style/layer_style/tests.rs`):
  - çizgi katmanında uyanlar önce, uymayanlar sonra;
  - nokta ve alan katmanının uymayanları;
  - geometrisiz katmanda hepsi uyuyor.
- Katman stili sınamaları (22) geçiyor.
- Resim (`.run/gui-evidence/kullanim-9143/`, Git'e girmez): Kategorili, sınıflamadan önce uyarı ve iki düğme.

## §4 Kalan

- **Etiket önizlemesi.** Boş ve uzun adlar, yakın/uzak ölçek ve baskı örneği.
- **Kayıp bildirimi.** Stil yükleme, kaydetme ve QML alışverişinde kaybolan özelliklerin tek tek bildirilmesi.
- **Değer değişimi.** Yolun planlı → kontrol geçişinde harita, sınıf sayacı, tablo ve çıktının aynı politikayı izlediği
  uçtan uca sınama.
- Web (TODOS.md §0.2).
