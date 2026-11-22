# ADR 9134: Birleşik görünüm veri katmanlarının satırlarını kaynaklarından okur

- **Durum:** kabul edildi (2026-11-22); adım VY-33. Plan: [plan/veri.md](../../plan/veri.md) VY-33.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  21. maddesi (P1):
  - Birleşik görünümler'de ana katman `mahalleler`, birleşen `bakim`, iki anahtar `mahalle`, “Yalnız eşleşen kayıtlar”
    kapalı;
  - görünümün sütunları oluştu ama durum “0 satır · 0 kayıt · 0 eşleşen” dedi;
  - dosya yeniden açılıp koşul temizlenince de sıfır kaldı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi.
- **Komut:** `cad.joins.query` veri katmanlarının satırlarını kaynaktan okur. Önceden boş görürdü.

## §1 Bağlam

- Görünüm (ADR 0527) iki tarafını çizimin nesnelerinden okuyordu (`by_layer_with_uids`). Veri katmanının satırları
  çizimde değil kaynağındadır (ADR 9101), bu yüzden iki taraf da boştu.
- Birleşenin bir kaynak tablosu olduğu yol (ADR 0632) yalnız sağ tarafı ve eski kaynak bağlantısını okuyordu. Projeye
  eklenmiş veri katmanı bu yoldan geçmiyordu.
- Veri katmanının satırları için iki ortak parça vardı:
  - satır kimliği `data_row_uid` (ADR 9124);
  - İstatistik'in arka planda okuduğu satırlar `data_rows` (ADR 9130).
- **Araştırma.** Başvuru alınan CBS uygulamasının “alan değeriyle öznitelik birleştir” işlemi bire-çok birleşimde
  eşleşen her kayıt için ayrı satır üretir, eşleşmeyenleri isteğe göre tutar ya da atar.
  Kaynak: [genel vektör işlemleri kılavuzu](https://documentation.qgis.org/3.44/en/docs/user_manual/processing_algs/qgis/vectorgeneral.html)
  (değerlendirmenin bağlantısı).

## §2 Karar

1. **Veri katmanı tarafı** (`joins_query::read_with`). Görünümün ana ya da birleşen katmanı veri katmanıysa kayıtları:
   - kaynağın çizimin tutmadığı bütün satırları, kaynak, tablo ve anahtardan türeyen kalıcı kimlikle (`data_row_uid`);
   - ardından çizimin düzenleme oturumunda tuttuğu satırlar, çizimdeki hâlleriyle.

   Eşleşme kuralı değişmedi: anahtarın boşlukları atılır, `007` ile `7` eşittir, eşleşen her kayıt bir satırdır.
2. **Okuyucu çağıranındır.**
   - `read_with` satırları çağıranın okuyucusundan alır.
   - Masaüstü onları arayüzü bekletmeden okunan `data_rows`'tan verir. Satırlar gelene kadar görünüm ““bakim” veri
     katmanının satırları kaynaktan okunuyor…” der, gelince yeniden okunur.
   - İki tarafın satırları birlikte istenir.
   - `cad.joins.query` (Python, MCP) satırları o an okur (`read`).
3. **Yalnız okunur.** Veri katmanının hücreleri görünümden yazılmaz: “veri katmanının satırı: görünümden yazılmaz;
   katmanın tablosunda, düzenleme oturumunda düzenleyin.”
4. **Ayrı sözler.**
   - Kaynak okunamazsa görünüm bozuk sayılır ve nedeni söylenir. Eski bir okuma gösterilmez.
   - Ana katmanda kayıt yoksa: ““mahalleler” katmanında kayıt yok; görünümde satır yok.”
   - Birleşende kayıt yoksa ve yalnız eşleşenler gösteriliyorsa bu söylenir.
   - Hiç eşleşme yoksa: “Hiçbir kayıt eşleşmedi: … anahtarlarında ortak değer yok.”
5. **Geometrisiz satır da satırdır.** `data_write::all_records` geometrisi olmayan satırı kayıt olarak tutar.
   - `data_rows` (İstatistik, Grafik, Katman süzgeci sayımı, İfadeyle seç önizlemesi, birleşik görünüm) bunu kullanır.
   - Topoloji `all_rows` ile geometrili satırları okumaya devam eder.

## §3 Kanıt

- **Masaüstü** (`gis/features/joins_data_tests.rs`), raporun verisiyle. B06'nın `mahalle`si NULL, B05'inki M9; ortak
  sınama verisi buna göre düzeltildi.
  - Birleşik görünümler penceresinde kurulan görünüm önce “okunuyor” diyor.
  - Sonra beş satır veriyor: M1/B01, M1/B02, M2/B03, M2/B04, M3 eşleşmesiz. Rapor 3 kayıt, 2 eşleşen, 1 eşleşmeyen;
    bakım 6 kayıttan 4'ü.
  - Yalnız eşleşenler dört satır.
  - Hücreler yalnız okunur, nedeni yazılı.
  - Eşleşme yok, boş katman ve okunamayan kaynak ayrı sözlerle söyleniyor.
- **Uygulama:** birleşim, topoloji, makbuz ve ortak komut durumu sınamaları (21) geçiyor.
- Öznitelik tablosu, işlemler, veri katmanı, istatistik, grafik, topoloji, izler, şerit, modül, katalog ve ayar
  sınamaları (371) geçiyor.
- Resimler: `.run/gui-evidence/kullanim-9134/` (Git'e girmez).

## §4 Kalan

- Birleşenin “Kaynak tablosu” seçeneği hâlâ eski kaynak bağlantısıyla okur. Projeye eklenmemiş bir veri tablosunu
  aynı veri sürücüsünden okumak ayrı iştir.
- Milyon satırlı iki tablonun birleşimi bellekte yapılır. Kaynakta birleştirme (SQL `JOIN`) ölçülmedi.
- Web (TODOS.md §0.2).
