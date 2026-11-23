# ADR 9188: Hiç yazılmamış değer, boş metin ve kodlu değer ayrı görünür; veri katmanında rolün alan süzgeci

- **Durum:** kabul edildi (2026-11-23); adım VY-87. Plan: [plan/veri.md](../../plan/veri.md) VY-87.
- **İstek:** sahibin sözü (23 Kasım 2026): “Yarım kalan maddeleri tamamla”. Değerlendirmenin 9. ve 29. maddelerinin
  kalanı:
  - 9 (ADR 9142, 9166, 9170): kodlu değer örneği; NULL ile boş metnin tabloda ayrı işareti;
  - 29 (ADR 9150): NULL ile boş metnin ayrı işareti; veri katmanı tablosunda rolün alan süzgeci.

  Kabul: “Boş metin, NULL, 0, kodlu değer, Türkçe arama ve sıralama ayrı örneklerle sınanmalı”; “Gerçek NULL, boş metin
  ve rol nedeniyle gönderilmeyen değer ayırt edilebilmeli. Tablo, bilgi kartı ve dışa aktarım aynı okunabilir alan
  kümesini kullanmalı.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- İfade dili NULL ile boş metni tek boş sayar (ADR 0581 §3, 9109). Değerler ise ayrıdır:
  - çizimin nesnesinde hiç yazılmamış alanın anahtarı yoktur, boş metin `""` olarak durur;
  - veri katmanının sayfasında NULL'un anahtarı yoktur, boş metin `""`'dir.
- Tablo ikisini de boş hücre gösteriyordu. Kart, katmanın alanlarından değeri olmayanı boş, ya da (şablonsuz kartta)
  hiç göstermiyordu. Kartta boş metin “—” idi.
- Çizim katmanının tablosu kodlu alanı etiketiyle gösteriyordu (`display_value`). Veri katmanının tablosu ve kartı
  kaynağın kodunu gösteriyordu.
- Rolün okumadığı alanlar (ADR 9150) çizim katmanının tablosundan ve kartından çıkıyordu, veri katmanınınkinden
  çıkmıyordu.

## §2 Karar

1. **NULL'un işareti.** Hiç yazılmamış değer tabloda soluk bir “NULL” olarak çizilir (`features::null_cell`). Boş
   metin boş görünür. “NULL” yazan bir metin soluk değildir, ondan ayrılır.
   - Çizim katmanının tablosunda değer, nesnede o alanın anahtarı yokken NULL'dur.
   - Veri katmanının tablosunda değer, satırın sayfasında anahtar yokken NULL'dur (`pending_rows::cell_raw`).
     Oturumda değişen değer de aynı kuralla okunur.
2. **Kart.** Değeri hiç yazılmamış alan “NULL” der, soluk; boş metin “—”.
   - Şablonsuz çizim kartı katmanın bildirdiği alanları sırasıyla, ardından nesnenin öbür anahtarlarını listeler.
     Değeri olmayan bildirilmiş alan artık atlanmaz, “NULL” der.
   - Veri satırının kartı da aynıdır.
3. **Kodlu değer.** Veri katmanının değer listeli alanı tabloda ve kartta etiketiyle görünür (“Park alanı”), çizim
   katmanınınki gibi (`pending_rows::shown_value`). Kaynakta kod durur, süzgeç ve sıra kaynakta koda göre işler.
4. **Rolün alan süzgeci veri katmanında.** Formlar üzerinden çalışan çağıranın (Veri girişi) okumadığı alanlar veri
   katmanının tablosunda sütun değildir. Tablonun altı onları adıyla söyler (“Rolünüz bu alanları okumuyor, gösterilmiyor:
   aciklama, sayi”). Kart da onları göstermez, adlarını söyler. Kural ADR 9150'nin ortak çözümleyicisidir
   (`withheld_fields`). Dışa aktarma tablonun gösterilen alanlarından gider.

## §3 Kanıt

- **Masaüstü** (`gis/features/null_tests.rs`):
  - veri katmanı (iki nokta: K'nin açıklaması NULL ve sayısı 0; P'nin açıklaması boş metin ve sayısı NULL; `kod` değer
    listeli): tabloda iki soluk “NULL”, etiketler “Konut alanı” ve “Park alanı”; kod çizilmiyor;
  - P'nin kartı: kod “Park alanı”, açıklama boş metin, sayı “NULL”;
  - çizim katmanında hiç yazılmamış alan tabloda “NULL”, kartta “NULL”;
  - Veri girişi rolü: tablo yalnız formun alanını gösteriyor, alt satır ötekileri adıyla söylüyor; kart onları
    göstermeyip adlarını söylüyor.
- `chain_tests` kartın NULL değerini “NULL” bekleyecek biçimde güncellendi. 0, Türkçe arama ve sıra sınamaları
  değişmeden geçiyor.
- Öznitelik tablosu, veri tablosu, Bilgi kartı, roller ve ortak masaüstü sınamaları (174) geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9188` (Git'e girmez).

## §4 Kalan

- 29. maddenin bulutta üç hesapla kabulü (25. maddeyle birlikte).
- Web (TODOS.md §0.2).
