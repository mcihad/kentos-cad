# ADR 9167: Bilgi aynı tıklamadaki nesneleri ve veri satırlarını birlikte sayar

- **Durum:** kabul edildi (2026-11-23); adım VY-66. Plan: [plan/veri.md](../../plan/veri.md) VY-66.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 13. maddesinin
  kalanı (ADR 9145 §4): “Örtüşmelerde sorgu kapsamına göre kaç kayıt bulunduğu ve diğer isabete geçiş görülsün.”
  Tıklamanın altındaki çizim nesneleri sayılıyordu; aynı yerdeki veri katmanı satırları sayılmıyordu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Bilgi bir nesneye tıklanınca o nesnenin kartını açıyor, alttaki öbür nesneleri “Bu noktada 3 nesne · 1/3” diye
  sayıyor ve Sonraki ile geziyordu (ADR 9145).
- Veri katmanlarının satırlarına yalnız tıklama hiçbir nesneye değmediğinde soruluyordu (ADR 9101 §6). Bir yolun
  çizim nesnesi ile aynı yerdeki veri katmanı kaydı birlikte görülemiyordu.
- **Araştırma.** Masaüstü CBS'lerin Bilgi sonuç paneli tıklamanın altındaki kayıtları katman katman sıralar: önce
  katmanın adı, altında bulunan kayıtlar. Kaynak:
  [QGIS “Working with the Attribute Table”, Exploring features attributes through the Identify Tool](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/attribute_table.html).

## §2 Karar

1. **Birlikte sorulur.** Bilgi aracı nesne bulduğunda isabetlerle birlikte tıklamanın erişimini de verir
   (`ViewChange::InfoHits(hits, reach)`). Pencere nesnenin yanında iki şeyi birlikte sorar, arayüzün iş parçacığı
   dışında:
   - rasterleri (ADR 9137);
   - gösterilen veri katmanlarının o noktadaki satırlarını, her katmandan birini (`data_info::ask_all`), üstteki katman
     önce.

   Önceki tıklamanın yanıtı bayattır.
2. **Birlikte sayılır.**
   - Kart şöyle der: “Bu noktada 1 nesne, 1 veri satırı · 1/2”. Yalnız nesne varsa söz eskisi gibidir.
   - Sonraki önce nesneleri, sonra satırları gezer; sonra başa döner.
   - Satıra gelince kart satırın kartıdır (katman, dosya, tablo ve anahtar) ve satır halkalanır.
3. **Kapanış.** Kart kapanınca ya da tıklama boşa gelince nesneler ve satırlar unutulur.

## §3 Kanıt

- **Masaüstü** (`cad/info_card_tests.rs`): örnek çizim ve parsellerin GeoPackage tablosu kullanılıyor. Tıklama hem
  çizimin parselinin hem tablonun ilk satırının içinde.
  - İsabetler parsel nesnesi ve 1 anahtarlı satır; yer 1/2.
  - Sonraki satırın kartını açıyor (yer 2/2), sonraki yine nesneye dönüyor.
  - Kapanınca ikisi de boşalıyor.
- Raster sınaması nesnenin yanında rasteri yine sunuyor. Etkileşim (84), kart, veri, iz ve ortak masaüstü
  sınamaları geçiyor.
- 2+1 daire izinin saveAndReopen adımı paralel koşuda düştü, tek başına geçti. Bu bilinen bir kararsızlıktır.
- **Resimler:** `.run/gui-evidence/kullanim-9167/`.
  - `01-nesne-ve-satir`: parselin kartında “Bu noktada 1 nesne, 1 veri satırı · 1/2”.
  - `02-satirin-karti`: satırın kartı, dosya ve “fid 1”, yer 2/2, satır halkalı.

## §4 Kalan

- ADR 9145 §4'ün öbür kalanları: etiket önizlemesinin kendi biçimiyle çizimi, veri katmanında etiket.
- Bir veri katmanından birden çok satırın sayılması; bugün her katmandan en yakın satır alınıyor.
- Web (TODOS.md §0.2).
