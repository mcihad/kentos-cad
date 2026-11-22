# ADR 9154: Adres aramasında eşit adaylar mahalleleriyle söylenir

- **Durum:** kabul edildi (2026-11-23); adım VY-53. Plan: [plan/veri.md](../../plan/veri.md) VY-53.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin ek kabul tablosunun
  “Adres, arama, istatistik ve karar desteği” satırı (P1):
  - aynı adlı iki sokak mahalleyle ayrılmalı;
  - mükerrer adres ile sıfır sonuç ayrılmalı;
  - seçili mahalleye göre bakım durumları sayılmalı; tablo, grafik ve dışa aktarım aynı süzgece ve NULL politikasına
    uymalı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Adres bulucusu (ADR 0229) adayları puanlar ve en iyisinin durumunu verir:
  - Eşleşti;
  - Belirsiz: tepede eşit puanlı birden çok aday;
  - Eşleşmedi.

  Paylaşılan durumlar (`fixtures/geocode/v1/cases.json`):
  - “Atatürk Bulvarı No:2” Kızılay ve Cumhuriyet'te Belirsizdir;
  - “Kızılay Atatürk Bulvarı No:4” Eşleşir;
  - “1203/2 Sokak No:1” aday vermez.
- Masaüstünün satırı Belirsiz'i “2 aday · En iyisi: Belirsiz (100)” diye söylüyordu: hangi mahallelerde olduğu ve nasıl
  ayrılacağı yoktu. Aday olmayınca “Aday yok: yazımı, kaynağı ve adres verisinin alanlarını denetleyin.” deniyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasının adres eşleştirme sonucunda (23 Kasım 2026'da okundu) durum alanı üç
  değer alır: eşleşti, eşleşmedi ve bağlı (tied). Bağlı, adresin aynı en iyi puanlı ama başka yerlerde birden çok adayı
  olduğu durumdur; eşleşmemekten ayrıdır.

  Kaynak: [eşleştirme sonucunun alanları](https://doc.esri.com/en/arcgis-pro/latest/help/data/geocoding/what-is-included-in-the-geocoded-results-.html).

## §2 Karar

1. **Eşit adaylar.** En iyisi Belirsiz olduğunda satır, tepedeki eşit adayların sayısını, puanını ve onları ayıran
   mahalleleri yazar: “2 aday · 2 aday aynı puanda (100): Cumhuriyet, Kızılay mahallelerinde; mahalleyi ekleyerek
   ayırın”. Mahalleleri aynıysa ilçeyi ya da posta kodunu eklemeyi söyler.
2. **Aday yok.** Hiç aday yoksa var olan söz kalır; eşitlikle karışmaz.

## §3 Kanıt

- **Masaüstü** (`gis/address/tests.rs`, `addresses.kcad`):
  - “Atatürk Bulvarı No:2” Cumhuriyet ve Kızılay'ı anıyor;
  - “Kızılay Atatürk Bulvarı No:4” Eşleşti (100);
  - “1203/2 Sokak No:1” “Aday yok …” diyor.
- Adres sınamaları (10) geçiyor.

## §4 Kalan

- **Seçili mahalleye göre sayım.** Veri katmanında seçili ya da görünümdeki satırlarla İstatistik ve Grafik
  (ADR 9130'un kalanı). Tablo, grafik ve dışa aktarımın aynı süzgeç ve NULL politikasıyla aynı toplamı verdiğinin uçtan
  uca sınaması.
- Web (TODOS.md §0.2).
