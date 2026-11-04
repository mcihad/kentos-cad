# KentOS CAD kılavuzu — §21 Bağlantı, proje açılışı ve autosave

> [CLAUDE.md](../../CLAUDE.md)'nin 21. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §21…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 21. Bağlantı, proje açılışı ve autosave

### 21.1 Sürekli bağlantı

Mevcut WS heartbeat/reconnect/replay/resync akışını koruyun (web). Masaüstü olayları
uzun sorguyla izler (`GET …/events?wait=`, ADR 0044). Auth, offline,
uyumsuz protokol ve server hatası ayrı durumlardır; socket açık diye veri güncel değildir.

### 21.2 Asenkron açılış

Yerel/cloud açılış iptal edilebilir ve generation/revision kontrollüdür.
Geç dönen yükleme kullanıcının yeni belgesini ezmez; doğrulanmamış aday belge
mevcut çalışmanın yerine geçirilmez. Büyük veride bounded bellek kullanın.

### 21.3 Otomatik kayıt

Masaüstü internetsiz çalışır, web çevrimiçi kalır (sahibin kararı, ADR 0043). Masaüstünde bulut
projesi yerel kopyadan (sunucunun son bilinen hâli) açılır, iş cihaz taslağında bekler, bağlantı
dönünce kendiliğinden eşitlenir; kopyaya sunucunun hâlindeki değişiklik taslaktan önce yazılır.
Çizim komutları sunucuda çalışmaz.

Cloud taslağı kalıcı kuyruğa alınmadan güvende denmez. Expected revision,
idempotency ve conflict davranışı korunur; başkasının güncel işi sessizce ezilmez.
Komut ACK, binary snapshot finalize ve cihazda kayıt ayrı durumlardır.
Yavaş kaydetme sırasında değişen belge eski revision adına temizlenmez.
Dosya projesi kendiliğinden kaydedilmez; `FileCommitted`'dan önce kaydedildi denmez;
başkasının revizyonu kendiliğinden yüklenmez (ADR 0038).

### 21.4 Doğrulama

Offline/reconnect, tekrar istek, ACK kaybı, mixed-client, quota, yetki iptali,
upload/finalize crash ve restore senaryolarını native/web/server sınırında sınayın.
