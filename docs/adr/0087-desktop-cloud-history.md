# ADR 0087: Masaüstünde bulut projesinin geçmişi: revizyonlar ve kontrol noktaları

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §21; TODOS.md `SYNC-11`, `CLOUD-07`; ADR 0034 (kontrol noktaları), 0038 (dosya projeleri), 0044 (olayları bekleyerek sorma), 0086 (masaüstünde katalog).
- **Kaynak:** web'in `app/cloud/history.ts`, `ui/cloud/catalogHistory.ts`, `historyPanel.ts`, `HistoryForms.ts` ve `downloads.ts`. Referans görüntüler web ajanının `cloud-shots.mjs`'indendir: `catalog-details-history-*` ve `history-*`.

## Bağlam

Katalogda seçili projenin ikinci sekmesi Geçmiş'tir (ADR 0086). Masaüstünde sekme görünüyor ama içi “masaüstüne henüz taşınmadı” diyordu.

Web'de bu sekmede:

- **İçerik:** dosya projesinin revizyonları ve her projenin adlandırılmış kontrol noktaları.
- **İşlemler:** indirme, yeni proje olarak geri yükleme, kontrol noktası oluşturma ve silme.
- **Kapalı işlem:** hesabın yapamadığı işlem görünür ve kapalı kalır; ipucu nedenini söyler.
- **Tazelenme:** projenin olayları dinlenir; `project.checkpoint` ya da `project.file` gelince liste yeniden sorulur.

## Karar

### Kurallar: `cloud/history.rs`

`history.ts`'in karşılığıdır:

- `kind_label`, `point_text`: bir kontrol noktasının ne olduğu ve neyi gösterdiği.
- `why_not_create`:
  - `feature.write` ister;
  - arşivde yapılamaz;
  - dosya projesinde kaydedilmiş bir revizyon ister.
- `why_not_delete`: önce `feature.write`; sonra ya oluşturan ya da `project.edit`; arşivde asla.
- `why_not_download`: revizyon için `project.download`.
- `why_not_take`: kontrol noktası indirmek ve geri yüklemek için `project.history` ve `project.download`.
- `changes_history`: hangi olayların geçmişi değiştirdiği.

Web'in iki kural testi aynı rollerle masaüstünde de koşar.

### Sekme: `catalog_history.rs`, `catalog_history_view.rs`

**Ne zaman sorulur:**

- Sekme bir projeyi gösterdiği sürece geçmiş sorulur: dosya projesinin revizyonları; `project.history` varsa kontrol noktaları. İki istek birlikte gider.
- Proje, sekme ya da liste değişince eski yanıt atılır.
- Web'deki gibi sekme seçim değişince korunur; önceki dilim onu Bilgiler'e döndürüyordu.

**Olaylar:** WebSocket yoktur. Seçili projenin olayları uzun bekleyişle izlenir:

1. İmleç projenin bilgisinden alınır (`GET …/projects/{id}`).
2. Sonra `GET …/events?wait=25` ile beklenir.
3. `project.checkpoint` ya da `project.file` gelirse liste 250 ms sonra sessizce yeniden sorulur: liste yanıta kadar yerinde kalır.
4. Hata olursa bekleyiş 5 s sonra yeniden başlar.

Sekme kapanınca, proje değişince ya da pencere kapanınca istekler bırakılır.

**Görünüş**, web'in `renderHistory`'si gibi:

- “Kontrol noktası oluştur…”;
- Kontrol noktaları ve sayısı. Her satırda ad, tür çipi, not, “veri revizyonu n · kim · ne zaman”, “1,6 KB · 6 nesne”; İndir, Yeni proje olarak geri yükle…, Sil…;
- Revizyonlar ve sayısı. Her satırda “Revizyon n”, en yenisinde “En yeni” çipi, kim · ne zaman, boyut ve nesne; İndir, Yeni proje olarak geri yükle…;
- Veritabanı projesinde revizyon dosyası olmadığını söyleyen not.
- Boş, yükleniyor, okunamadı (Yeniden dene) ve yetkisiz durumlarda web'in cümleleri.

**Formlar**, katalog penceresinin üstünde:

- **Kontrol noktası oluştur:**
  - ad (en çok 120), not (çok satırlı), dosya projesinde adlandırılacak revizyon (en yenisi seçili);
  - durum satırı: dosya projesinde “Oluşturuluyor…”, veritabanı projesinde “Projenin görüntüsü alınıyor…”;
  - açık veritabanı projesinde önce bekleyen iş gider (`Settle::Checkpoint`), böylece kontrol noktası onları da tutar.
- **Yeni proje olarak geri yükle:**
  - web'in dört maddesi; saklama biçimi noktadan gelir: dosya noktası dosya projesi, anlık görüntü veritabanı projesi olur;
  - isteğe bağlı ad; proje açılabilen çalışma alanları, projenin alanı önce (`creatable`, web'in `creatableWorkspaces`'i);
  - yeni proje Projelerim'de seçilir ve liste onu gösterince açılır (web'in `openMade`'i); göstermezse durum satırı bunu söyler.
- **Kontrol noktasını sil:** web'in `askRemove` sorusu. Adlandırılmış revizyonda revizyonun kaldığı, anlık görüntüde dosyanın da silindiği söylenir.
- **Esc** önce formu ya da soruyu kapatır. İsteği yoldaki form beklemesini sürdürür.

Başarı ve ret satırları web'inkidir. Formun reddi formun içinde söylenir, silmenin reddi günlükte.

### İndirme

Katalogdaki indirme (ADR 0086) genelleşti: projenin kendisi, bir revizyon ya da bir kontrol noktasının dosyası.

- **Adlar:** “Ada 101 (revizyon 3)” ve “Ada 101 - Belediyeye teslim”.
- **SHA-256:** listenin verdiği değer, sunucununkine ek olarak denetlenir (web'in `verifyDownload`'u). Tutmazsa dosya yazılmaz.
- **İstemci:** `kentos-cloud` `checkpoints` ve `checkpoint_file` çağrılarını aldı.

## Doğrulama

- **Birim testleri** (`cloud::catalog_history_tests`, `cloud::history`):
  - sorma, geç yanıtın atılması;
  - olay izleme: düzenleme olayı geçmişi tazelemez, kontrol noktası olayı sessizce tazeler;
  - sekmeden çıkınca bırakma;
  - kontrol noktası formu: adsız gitmez, ret formda, başarı satırı ve yeniden sorma;
  - geri yükleme formu: projenin alanı önce; yeni proje Projelerim'de açılır;
  - silme sorusu, Esc ve reddin adıyla satırı;
  - indirmelerin adı, listenin SHA-256'sı ve yetkisi.
- **Gerçek sunucu** (`apps/desktop/scripts/cloud-live.sh`, adım 16–22):
  - çöp kutusundan geri yükleme;
  - veritabanı projesinin bilgileri (13 nesne);
  - favori açık ve kapalı;
  - `.kcad` indirme (KCAD v2 olarak koklanır);
  - kontrol noktası oluşturma ve listede görünmesi;
  - geri yüklenen projenin açılması;
  - koşunun yaptıklarının çöpe taşınıp kalıcı silinmesi.
- **Görüntüler:** `cloud::catalog_tests::screens`:
  - veritabanı projesinin geçmişi;
  - dosya projesinin revizyonları ve adlandırılmış revizyonu;
  - kontrol noktası formu, geri yükleme formu, silme sorusu;
  - koyu ve açık tema, 1440×900 ve 1100×650.
