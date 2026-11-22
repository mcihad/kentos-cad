# ADR 9130: İstatistik ve Grafik veri katmanının satırlarını kaynağından sayar

- **Durum:** kabul edildi (2026-11-22); adım VY-29. Plan: [plan/veri.md](../../plan/veri.md) VY-29.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  17. maddesi (P1):
  - bakım katmanının tablosu altı kaydı okuyordu;
  - İstatistik › bakim › Tümü “0 nesne / Katmanda nesne yok” dedi;
  - Grafik › durum › Sayı tablosunda başlıklar vardı ama veri satırı yoktu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- İstatistik ve Grafik bir katmanın verisini ortak okuyucudan (`charts::data::LayerData`) alır. Okuyucu katmanın
  çizimdeki nesnelerini sayıyordu.
- Veri katmanının satırları çizimde değil, kaynağındadır (ADR 9101). Yalnız düzenlemede alınanlar ve yeni çizilenler
  çizimdedir.
- Bağlı katmanlar için “Kaynağın tümü” kapsamı vardı (ADR 0581 §6). Veri katmanında hiçbir kapsam satırları
  görmüyordu.

## §2 Karar

1. **Satırlar kaynaktan** (`gis/data_rows.rs`):
   - bir sekme veri katmanının satırlarını isteyince satırlar kaynaktan bütün olarak, arayüzü bekletmeden okunur;
   - okunanlar çizimin oturumu ve kaynaklara yazım sayısıyla saklanır; katmanın değişiklikleri yazılınca yeniden
     okunur;
   - çizimin tuttuğu satırlar (düzenlemede alınanlar ve yeniler) her seferinde çizimdeki o anki hâlleriyle alınır.
2. **Ortak okuyucu.** `LayerData` veri katmanında bu satırları kullanır. İstatistik ve Grafik aynı veriyi okur.
   - Satırların çizimde yeri yoktur. Seçililer ve Görünümdekiler onları almaz; Alan ve Uzunluk sütunları çizimin
     ölçüsü olmadığından eklenmez.
   - Alanların değerleri, boşları, benzersizleri ve toplamları sayılır.
3. **Durumlar ayrı söylenir:**
   - okunurken “Kaynak okunuyor…”;
   - okunamazsa “Kaynak okunamadı: neden”;
   - kaynakta satır yoksa “Kaynakta satır yok.”;
   - okununca sayı satırı “6 satır, kaynaktan” der. Kaynağa yazılmamış değişiklik varsa “…; kaynağa yazılmamış
     değişiklikler bu cihazdaki hâliyle” diye eklenir.
4. **Katman ▾** İstatistik'te ve Grafik'te veri katmanını “(veri)” diye yazar. Satırlar kaynağındadır, çizimin nesne
   sayısı gösterilmez.

## §3 Kanıt

- **Masaüstü** (`gis/data_rows_tests.rs`), raporun bakım verisi (bedel 0, 100, 250, NULL, 50, 100; durum acil, planlı,
  tamam ikişer):
  - ilk sayımda sekme kaynağın okunduğunu söylüyor;
  - okununca bedel 5 dolu, 1 boş, en az 0, ortalama 100, toplam 500 çıkıyor;
  - durum 6 dolu, 3 farklı değer.
- İstatistik ve Grafik sınamaları geçiyor.

## §4 Kalan

- Veri katmanında Seçililer ve Görünümdekiler (tablonun seçimi ve görünüm kutusuyla kaynağa sormak).
- Grafik'in sütunlarından haritada seçim.
- Büyük tablolarda sağlayıcının kendi toplamı (`GROUP BY`) ile sayım. Bugün satırlar okunup sayılıyor.
- Web (TODOS.md §0.2).
