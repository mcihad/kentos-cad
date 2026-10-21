# ADR 0577: Ulaşılamayanda düğüm, dönüş ve cihaz

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-26'nın kalanından: “Başka'nın ayrıntısı”.
- **Bağlam belgesi:** ADR 0521 (dönüşler), 0557 ve 0562 (cihazlar ve terminaller), 0570 (ulaşılamayanın nedeni), 0575 (en az kapatan).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - Hizmet alanında ulaşılamayan kenarların nedeni.
  - Seviyenin (köprü, tünel) yolu kesmesi; bugün “Başka” olarak kalır.
  - Cihazın terminal yolunun kapatması koddadır, ortak durumlarda yoktur.

## Bağlam

ADR 0575'in araması kenarları ve engelleri sayıyordu. Yolu bir düğüm kesiyorsa, yani kapalı bir düğüm (kapalı vana), yasak bir dönüş ya da
iki kenarı birleştirmeyen bir cihaz yolu kesiyorsa, söz yalnız “Başka” diyordu: “yolu bir dönüş yasağı, kapalı düğüm ya da cihazın
terminal yolları kesiyor”. Hangisi olduğu ve nerede olduğu söylenmiyordu.

## Karar

### 1. Düğüm de kapatandır

En az kapatanlı yolun araması (`blocked.rs`'in `fewest`'i) bir yaydan ötekine geçerken düğümü de sayar. Düğüm geçişi şu durumlarda bir
kapatandır:

- düğüm kapalıdır (kapalı ifadesi tutan bağlantı düğümü);
- grafın dönüşü yasaktır (`Graph::turn` yok der): dönüş katmanının çizgisi, kuralların izin vermediği geri dönüş ya da cihazın birleştirmediği
  iki terminal.

Sıralama ADR 0575'teki gibidir: en az kapatan, sonra en kısa. Bir durağın bulunduğu düğümden çıkış ya da varılan düğüme giriş dönüş
sayılmaz.

### 2. Söz ve JSON

Yolun sırasıyla, kenarların ve engellerin arasında söylenir:

| Kapatan | Söz | JSON |
|---|---|---|
| Kapalı düğüm | “düğüm 100 (kapalı)” | `junctions: [{id, why: "closed"}]` |
| Cihaz | “düğüm 7 (cihaz kenar 4 → kenar 5 geçirmiyor)” | `junctions: [{id, why: "device", from, to}]` |
| Yasak dönüş | “kenar 23 → kenar 25 dönüşü (yasak)” | `turns: [{from, to}]` |

- Kapalı düğüm için düğümün nesnesi söylenir.
- Cihaz için düğümün nesnesiyle birlikte iki kenar söylenir. Cihazı değil de dönüşü kapatan bir düğüm, dönüş olarak söylenir.
- `junctions` ve `turns` yalnız dolu olduklarında yazılır; ADR 0570'in cevapları değişmez.
- “Başka” (`other`) artık yolun kenarlarının, engellerinin ve düğümlerinin hepsi açıkken söylenir: “yolu bir seviye ya da ağın dışında kalan
  durak kesiyor”.

## Doğrulama

- **Bağımsız başvuru: `scripts/fixtures/network_blocked_cases.py --check`.**
  - **Yöntem:** KentOS kodu olmadan yazılmıştır. Arama (köşe, gelinen kenar) çiftleri üzerinde sözlük sıralı bir Dijkstra'dır; dönüşü gelinen
    kenara göre sorar. Ulaşılabilirlik de aynı aramadan gelir: kapatanı sıfır yol varsa yol vardır.
  - **Tanım:** ağa bağlantı düğümü katmanı (kapalı ifadesiyle) ve dönüş katmanı eklendi. Geri dönüş hiçbir yerde yoktur; yoksa T'nin çıkmaz
    ucunda geri dönülüp yasak dönüş dolanılırdı.
  - **Yeni sorular:**
    - `kapali-dugum`: açık iki kenarın ortasında kapalı düğüm; cevap düğüm 100.
    - `yasak-donus`: T kavşakta yasak dönüş; cevap kenar 23 → kenar 25.
    - `izinli-donus`: aynı T'nin öbür kolundan dönüş serbest; yol var.
  - **Eski sorular:** ADR 0570 ve 0575'in 15 sorusunun cevabı değişmedi.
- **Çekirdek:** `tests/all/network_access.rs` 18 sorunun JSON'unu başvurununkiyle karşılaştırır. Düğümleri ve dönüş çizgilerini
  fixture'dan kurar (`Graph::build_with`).
- **Python:** `test_network_blocked.py` çizimi düğüm ve dönüş katmanlarıyla kurar ve sözleri karşılaştırır.
