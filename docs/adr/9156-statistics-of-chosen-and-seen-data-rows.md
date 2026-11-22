# ADR 9156: İstatistik ve Grafik veri katmanının seçili ve görünümdeki satırlarını sayar

- **Durum:** kabul edildi (2026-11-23); adım VY-55. Plan: [plan/veri.md](../../plan/veri.md) VY-55.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”.
  - Değerlendirmenin 17. maddesinin kalanı (ADR 9130): veri katmanında Seçililer ve Görünümdekiler.
  - Ek kabul tablosunun “Adres, arama, istatistik ve karar desteği” satırı: seçili mahalleye göre bakım durumlarının
    sayılması.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- İstatistik ve Grafik veri katmanının satırlarını kaynağından sayar (ADR 9130). Kaynağın satırlarının çizimde yuvası
  olmadığı için Seçililer ve Görünümdekiler onları dışarıda bırakıyordu: tablodan seçilen satırlar ve görünümdeki
  satırlar sayılmıyordu.
- Veri katmanı tablosunun seçimi katmanıyla ve satır anahtarlarıyla tutulur. Haritadaki Seç ve İfadeyle seç de
  satırları bu seçime yazar (ADR 9103 §5, 9133).

## §2 Karar

1. **Anahtar ve kutu.** Veri katmanının satırları sayılırken her satırın anahtarı ve kutusu da tutulur
   (`data_layer_rows_keyed`). Çizimin nesneleri (alınmış ve yeni satırlar) kendi yuvalarıyla gelir.
2. **Seçililer.** Tablonun o katmandaki seçimi anahtarlarla, çizimde seçili nesneler yuvalarıyla.
3. **Görünümdekiler.** Kutusu görünümün kutusuna değen satırlar; kutular projenin sistemindedir (kaynağın sistemi
   izdüşürülmüş).

## §3 Kanıt

- **Masaüstü** (`gis/data_rows_tests.rs`), raporun altı bakım noktası (bedel 0, 100, 250, NULL, 50, 100):
  - bütün satırlar: 5 dolu, toplam 500 (eskisi gibi);
  - tablodan B02 ile B03 seçilince Seçililer: 2 satır, toplam 350;
  - görünüm ilk üç noktaya getirilince Görünümdekiler: 3 satır, toplam 350.
- Grafik, istatistik, veri satırları, şerit ve modül sınamaları (54) geçiyor.

## §4 Kalan

- **Tek süzgeç uçtan uca.** Tablo, grafik ve dışa aktarımın aynı süzgeç ve NULL politikasıyla aynı toplamı verdiğinin
  tek sınaması.
- **Alan ve uzunluk.** Kaynağın satırlarının ölçüleri (Alan, Uzunluk) çizimin ölçü deposundan gelmiyor; sayılmıyor.
- Web (TODOS.md §0.2).
