# KentOS CAD kılavuzu — §17 Harita yayını: olası gelecek kapsamı

> [CLAUDE.md](../../CLAUDE.md)'nin 17. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §17…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 17. Harita yayını: olası gelecek kapsamı

Martin entegrasyonu veya Martin eşdeğeri şu an görev değildir; cloud
saklama/açma/paylaşım için MVT/TileJSON servisi zorunlu tutulmaz.
Mevcut outbox/event güncelliği sync için korunur; gelecekte yayın için de
kullanılabilir. Ayrı yayın kararı gerekirse TODOS.md §12.5 izlenir.

### 17.1 Gelecek değerlendirmesi

Somut ihtiyaç olmadan tile crate'i, compatibility endpoint'i veya yayın fazı
başlatmayın. Olası yayın türevleri `.kcad` ya da CAD kaynak modelinin yerine geçmez.
