# Dosya projesinin revizyonları: web'in davranışı

Güncelleme: 28 Eylül 2026. Bu belge açık bir dosya projesinde ([ADR 0031](../adr/0031-cloud-file-projects.md), [0038](../adr/0038-web-file-projects-and-history.md)) şunları anlatır:

- web sunucudaki yeni bir revizyonu nasıl öğrenir;
- durum çubuğunun kayıt hücresi her durumda ne der;
- bir tıklama neyi sunar;
- her cevap çizime ve kaydedilmemiş işe ne yapar;
- Kaydet ve Geçmiş sekmesi bunlarla nasıl çalışır.

Amaç, web kodunu görmemiş birinin bunu başka bir platformda kurabilmesidir. Masaüstü olayları uzun sorguyla izler (`GET …/events?wait=`, [ADR 0044](../adr/0044-events-long-poll.md)); web WebSocket kullanır. Bu yüzden belge “bir olay, birinin revizyon kaydettiğini söyler” diliyle yazılmıştır; soketin ayrıntısı yoktur.

Kesin kurallar ve sözler `fixtures/cloud/v1/file-revisions.json`'dadır. Biçimi [fixtures/cloud/README.md](../../fixtures/cloud/README.md)'dedir. Dosyanın cevapları web'in kodundan ayrı olarak `scripts/fixtures/file_revisions_cases.py` ile bulunur. Web'de kaynaklar şunlardır:

| Dosya | İçinde |
|---|---|
| `apps/web/src/app/cloud/fileRevisionsPlan.ts` | Bilinenler, onları değiştiren girdiler (`step`), hücrenin durumu (`cellState`), Kaydet'in ilk adımı (`saveStep`), olaylar (`readEvents`), en yeni revizyon (`newestOf`), yeniden eşitleme (`resyncStep`), sorular (`offer`) ve sözler. Saftır. |
| `app/cloud/fileProject.ts` | Sunucuya sorar, yükler, kaydeder; bilinenleri planın `step`'iyle tutar. |
| `app/cloud/session.ts` | Canlı kanal; dosya projesinin yeniden eşitlemesi. |
| `app/cloud/fileSession.ts` | Açma; yeni revizyonun günlük satırı. |
| `ui/cloud/FileConflict.ts` | Soruları sorar, cevabı yapar. |
| `ui/statusbar/cellsPlan.ts`, `cloudCells.ts` | Kayıt hücresi ve ipucu. |
| `ui/cloud/catalogHistory.ts`, `historyPanel.ts` | Geçmiş sekmesinin işaretleri. |

Masaüstünde ([ADR 0119](../adr/0119-desktop-file-revisions.md)) kaynaklar şunlardır:

| Dosya | İçinde |
|---|---|
| `apps/desktop/src/cloud/revisions.rs` | Planın karşılığı: bilinenler, `step`, `cell_state`, `save_step`, `read_events`, `newest_of`, `resync_step`, `offer` ve sözler. Saftır; `revisions_tests.rs` bu dosyayı oynatır. |
| `cloud/file_follow.rs` | Olayların uzun sorguyla izlenmesi, en yeni revizyonun ve erişimin sorulması, yeniden eşitleme, bitiş; soruların sorulması ve cevapların yapılması. |
| `cloud/file.rs` | Kaydet'in ilk adımı, yükleme, kendi kaydının istek kimliği, ret ve yazılma. |
| `cloud/cells.rs`, `cloud/cells_plan.rs` | Kayıt hücresi ve ipucu. |
| `cloud/catalog_history_view.rs` | Geçmiş sekmesinin işaretleri. |

Masaüstünün resimleri `cargo test -p kentos-desktop cloud::file_follow_tests::revision_screens -- --ignored --nocapture` ile `.run/shots/bulut-revizyon-*`'a çekilir.

Resimler `node apps/web/scripts/e2e/cloud-shots.mjs --only revision-…` ile çekilir. Gerçek `kentosd` geçici bir veritabanında çalışır. Her sahne 1440×900 ve 1100×650'de, koyu ve açık temada `apps/web/scripts/e2e/out/shots/cloud/` altına yazılır:

| Sahne | Gösterdiği |
|---|---|
| `revision-current` | Güncel çizim: “Buluta kaydedildi · r3” ve ipucu |
| `revision-newer` | Başkası kaydetti: “Yeni revizyon: r4”, ipucunda kim ve ne zaman |
| `revision-newer-question` | Hücreye tıklama: “Son revizyonu aç” sorusu |
| `revision-newer-dirty` | Kaydedilmemiş iş varken: “Yeni revizyon: r4 · kaydedilmedi” ve ipucu |
| `revision-newer-dirty-question` | Aynı durumda tıklama: çakışma sorusu |
| `revision-conflict` | Kaydet'ten sonra Vazgeç: “Çakışma: r4 kaydedilmiş” ve ipucu |
| `revision-unsaved-question` | Yeni revizyon bilinmezken Son revizyonu aç…: kaydedilmemiş değişiklikler sorusu |
| `revision-history` | Geçmiş sekmesi: “En yeni” ve “Açık çizim” işaretleri |
| `revision-log` | Alt panelin günlüğünde yeni revizyonun satırı |
| `revision-offline` | Bağlantı yokken hücre ve ipucu (durum resim için kuruldu) |

## 1. Bilinenler

Açık dosya projesi şunları bilir (`RevisionState`):

| Alan | Anlamı |
|---|---|
| `base` | Ekrandaki çizimin dayandığı revizyon. Henüz revizyon yoksa `"0"`. |
| `newer` | Sunucuda `base`'ten yeni bir revizyon: numarası, kaydedenin adı (`by`, bilinmiyorsa boş) ve zamanı (`at`, RFC 3339; bilinmiyorsa `null`). |
| `conflict` | Reddedilen (ya da reddedileceği bilinen) Kaydet'in karşılaştığı revizyonlar: `expected` (çizimin `base`'i) ve `actual` (sunucunun en yenisi). Kullanıcı seçene kadar durur. |
| `dirty` | Çizimde hiçbir revizyonun tutmadığı değişiklik var. |
| `stage` | Kaydet'in aşaması: `idle`, `encoding`, `uploading`, `verifying`. |
| `failed` | Son Kaydet başka bir nedenle başarısız oldu. Sonraki Kaydet başlayınca unutulur. |
| `writable` | Hesap revizyon yazabilir (`feature.write`). |
| `ended` | Artık kaydedilmez: `deleted`, `revoked`, `archived`. |

Proje açılınca `newer`, `conflict` ve `failed` boştur; `stage` `idle`'dır. Arşivlenmiş proje açılır açılmaz `ended: archived` olur.

Bilinenleri yalnız şu girdiler değiştirir (`step`). Proje bittikten (`ended`) sonra yalnız çizimin kendi değişikliği sayılır.

| Girdi | Ne olur |
|---|---|
| `dirty` | Değişiklik geldi ya da gitti; örneğin kaydedilmiş hâle geri alındı. |
| `newest` | Sunucu en yeni revizyonunu söyledi. Kurallar §2.2'dedir. |
| `stage` | Kaydet bir aşamaya geldi. `encoding` yeni bir Kaydet'tir; önceki başarısızlık unutulur. |
| `committed` | Sunucu bu pencerenin Kaydet'ini `revision` olarak yazdı. `base` o olur. `newer` yalnız ondan yeniyse kalır. Kayıt sürerken yapılan değişiklik kaydedilmemiş kalır. |
| `refused` | Başkası `actual`'ı önce kaydetti. Ya sunucu `@file` çakışmasıyla reddetti ya da Kaydet bunu önceden biliyordu (§4). `conflict` `{ base, en yeni }` olur. `newer` aynı ya da daha yeni bir revizyonsa, kim ve ne zaman bilgisi korunur; değilse yalnız numara bilinir. |
| `failed` | Kaydet başka bir nedenle olmadı: yanıt gelmedi ya da çizimin düzeltemeyeceği bir ret geldi. |
| `unchanged` | Kaydet çizimi `base`'in kendisi buldu; yazacak bir şey yok. Önceki başarısızlık unutulur. |
| `access` | Hesabın yazma hakkı değişti. |
| `ended` | Proje silindi, arşivlendi ya da erişim kalktı. |

## 2. Yeni revizyonun öğrenilmesi

### 2.1 Olaylar

Projenin olayları sırayla okunur (`readEvents`):

- **`project.file`**: biri bir revizyon kaydetti. Olay revizyonun numarasını taşımaz; `dataRevision` projenin sayacıdır, dosyanınki değildir. Bu yüzden istemci sunucuya sorar: `GET …/files`. Cevabın `current`'ı en yeni revizyondur. Listedeki satırından kaydedenin adı (`createdByName`) ve zamanı (`createdAt`) alınır (`newestOf`). Satır listede yoksa yalnız numara bilinir. Bir olay dizisi için en çok bir kez sorulur.
- **Bu pencerenin kendi kaydı**: her Kaydet'in istek kimliği (`requestId`) gönderilmeden önce saklanır. O kimliği taşıyan `project.file` olayı başkasının değildir; sorulmaz. İstek kimliği olmayan olay başkasınındır.
- **`project.access`**: hesabın ne yapabildiği yeniden sorulur.
- **`project.deleted`**: proje biter (`deleted`). Dizide ondan sonra gelenlere bakılmaz.
- **`project.archived`**: proje biter (`archived`). Arşivleyen bu pencereyse sessiz biter; değilse söylenir.
- Öbür olaylar (ad, kontrol noktası…) burada bir şey değiştirmez.

### 2.2 Duyulan revizyon

Sunucunun söylediği en yeni revizyon (`newest` girdisi) şöyle işlenir:

1. Revizyon yoksa ya da `base`'ten yeni değilse hiçbir şey olmaz.
2. Bilinen `newer`'dan eskiyse hiçbir şey olmaz. Aynı anda iki soru gidebilir; geç gelen eski bir cevap bilineni düşürmez.
3. Bilinen `newer`'ın kendisiyse söylenmez; bilinmeyen kaydeden ya da zaman doldurulur. Reddedilen Kaydet yalnız numarayı söyler; kim ve ne zaman sonra gelir.
4. Daha yeniyse `newer` o olur ve **bir kez söylenir**: günlüğe uyarı satırı yazılır (§3.4). Duran bir çakışma varsa artık bu revizyonladır.

Revizyon hiçbir zaman kendiliğinden yüklenmez; çizim olduğu gibi kalır.

### 2.3 Bağlantı kopunca ve dönünce

- Bağlantı yokken hiçbir olay duyulmaz. Kayıt hücresinin sözü değişmez, çünkü çizimin durumu değişmemiştir. Bağlantının yokluğu sunucu hücresinde (“Sunucu: yok”) ve kayıt hücresinin ipucunda (“Canlı bağlantı: çevrimdışı.”) görünür.
- Bağlantı dönünce olaylar imleçten sonrası olarak gelir. Web kanalı yeniden abone olur; masaüstü uzun sorguyu imleçten sürdürür. Kaçırılan `project.file` olayları o zaman §2.1'deki gibi işlenir.
- Web'de bağlantı yokken Kaydet başarısız olur: `failed` olur, hücre “Kayıt hatası” der. İleti şudur: “Dosya kaydedilemedi: sunucuya ulaşılamadı (…). Çizim olduğu gibi duruyor; bağlantı dönünce yeniden Kaydet'e basın.”
- Masaüstü internetsiz çalışır ([ADR 0043](../adr/0043-desktop-offline-replica.md)). Kaydet cihazda tutulur (“Kaydedildi (bu cihazda) · gönderilecek”) ve bağlantı dönünce gider. Arada başkası kaydettiyse çakışma sorusu gelir; cihazda tutulan kayıt o seçim olmadan atılmaz.

### 2.4 Yeniden eşitleme

Sunucu imleçten devam edemeyebilir: imleçten sonraki olayları artık tutmuyordur ya da imleç sunucunun en yenisinin ötesindedir. O zaman web kanalında `resyncRequired`, uzun sorguda `410 resync_required` gelir. **Dosya projesi yeniden açılmaz; çizim değişmez.** Proje sorulur (`GET …/projects/{id}`; geçici hatalar önce yeniden denenir) ve cevaba göre davranılır (`resyncStep`):

| Cevap | Ne olur |
|---|---|
| Proje, `active` | `follow`: olay imleci projenin şimdiki `eventCursor`'ı olur. Erişim yeniden sorulur (`project.access` gibi). En yeni revizyon sorulur (`project.file` gibi). Sonra olaylar yeni imleçten izlenir. |
| Proje, `archived` | Biter: `archived` (söylenir). |
| Proje, `trashed`; ya da 410 | Biter: `deleted`. |
| 404 | Biter: `revoked`. |
| 403 | Biter: `revoked`; sunucunun nedeni söylenir. |
| Yanıt yok, ya da başka bir ret | `retry`: 30 saniye (`resyncRetryMs`) sonra yeniden abone olunur. Sunucu yine yeniden eşitleme ister ve proje yeniden sorulur. İlk başarısızlıkta bir uyarı yazılır (`resyncFailed`); cevap gelene kadar yinelenmez. |

Veritabanı projesinde yeniden eşitleme eskisi gibidir: proje sunucudan yeniden açılır, cihaz taslağı üstüne konur.

## 3. Kayıt hücresi

### 3.1 Durum

Hücrenin durumu bilinenlerden şu sırayla çıkar (`cellState`):

1. Proje bittiyse: `deleted`, `revoked` ya da `archived`.
2. Kaydet sürüyorsa aşaması: `encoding`, `uploading`, `verifying`.
3. Çakışma varsa: `conflict`.
4. Yeni revizyon varsa: `outdated`. Kaydedilmemiş iş varken de, salt okunur hesapta da böyledir; salt okunur hesap da son revizyonu açabilir.
5. Hesap yazamıyorsa: `readonly`.
6. Son Kaydet başarısızsa: `error`. Yeni revizyon bundan önce gelir, çünkü sonraki Kaydet zaten ona çarpar.
7. Kaydedilmemiş iş varsa: `pending`; yoksa `saved`.

### 3.2 Sözler, renk ve tıklama

Hücrede simge yoktur: bir lamba ve bir söz vardır. Lambanın rengi durumdan gelir.

| Durum | Söz | Renk | Tıklama |
|---|---|---|---|
| `saved` | “Buluta kaydedildi · r4”; revizyon yoksa “Henüz revizyon yok” | Yeşil dolu lamba | Kaydet (değişiklik yoksa “zaten kaydedilmiş” der) |
| `pending` | “Kaydedilmedi · r4 üstüne”; revizyon yoksa “Kaydedilmedi” | Olağan, boş lamba | Kaydet |
| `encoding` | “Dosya hazırlanıyor…” | Olağan | Yok |
| `uploading` | “Yükleniyor %42” (yarım yukarı yuvarlanır) | Olağan | Yok |
| `verifying` | “Sunucu doğruluyor…” | Olağan | Yok |
| `outdated` | “Yeni revizyon: r5”; kaydedilmemiş iş varsa “Yeni revizyon: r5 · kaydedilmedi” | Sarı yazı, boş lamba | Son revizyonu aç… (`cloud.openNewest`, §5) |
| `conflict` | “Çakışma: r5 kaydedilmiş” | Kırmızı yazı, dolu lamba | Kayıt çakışmalarını çöz… (`cloud.conflicts`, §5) |
| `error` | “Kayıt hatası” | Kırmızı, dolu | Kaydet |
| `readonly` | “Salt okunur” | Soluk | Yok |
| `deleted` | “Proje silindi” | Kırmızı, dolu | Kaydet: yerel dosya önerilir |
| `revoked` | “Erişim kaldırıldı” | Kırmızı, dolu | Kaydet: yerel dosya önerilir |
| `archived` | “Proje arşivde” | Soluk | Kaydet: yerel dosya önerilir |

Uygulama menüsünün başlığı da aynı sözü kullanır: “Bulut dosya projesi · Yeni revizyon: r5”.

### 3.3 İpucu

İpucunun başlığı “Bulut kaydı: dosya projesi”dir. Satırları sırasıyla şunlardır; boş olan yazılmaz:

1. “Büro › Kadastro paftası 2026, dosya olarak saklanıyor (KCAD revizyonları).”
2. “Çizimin dayandığı revizyon: 4.” Revizyon yoksa: “Projenin henüz revizyonu yok.” Sayı ekten ayrı yazılır.
3. “Kendiliğinden kaydedilmez: Kaydet (Ctrl+S) yeni bir revizyon yazar; arada başkası kaydettiyse üzerine yazılmaz.”
4. Bu pencere kaydettiyse: “Bu pencerenin son kaydı: revizyon 4, 2 dk önce.”
5. Yeni revizyon varsa (`newerTip`), kim ve ne zaman bilinenle birlikte:
   - Temiz çizimde: “Sunucuda daha yeni revizyon var: 5 (Mehmet Demir, 27.09.2026 14:32); açmak için tıklayın.”
   - Kaydedilmemiş iş varken: “…; kaydedilmemiş değişiklikleriniz Kaydet ile onun üzerine yazılamaz. Seçenekler için tıklayın.”
6. Son Kaydet'in hatası.
7. “Canlı bağlantı: canlı.” Bağlantı yoksa “çevrimdışı”, dönerken “yeniden bağlanıyor”.
8. Kaydedilmemiş iş varsa: “Kaydedilmemiş değişiklikler bu cihazda kurtarma kopyası olarak da saklanıyor.”

Kim ve ne zaman şöyle yazılır: “ (Mehmet Demir, 27.09.2026 14:32)”. Yalnız biri biliniyorsa yalnız o yazılır; ikisi de bilinmiyorsa hiçbiri yazılmaz. Zaman cihazın yerel saatiyle, gg.aa.yyyy ss:dd biçimindedir; Geçmiş sekmesi de aynı biçimi kullanır.

### 3.4 Günlük satırı

Yeni revizyon ilk duyulduğunda günlüğe bir uyarı yazılır (`newerLine`). Aynı revizyon için ikinci kez yazılmaz:

- Temiz çizimde: ““Kadastro paftası 2026” başka bir yerde kaydedildi: revizyon 5 (Mehmet Demir, 27.09.2026 14:32). Açık çizimin dayandığı revizyon: 4; yeni revizyonu açmak için durum çubuğundaki kayıt durumuna tıklayın. Kendiliğinden yeniden yüklenmez.”
- Kaydedilmemiş iş varken ortası şöyle olur: “…; kaydedilmemiş değişiklikleriniz Kaydet ile bu revizyonun üzerine yazılamaz. Ayrı kopya, yerel dosya ya da son revizyon için durum çubuğundaki kayıt durumuna tıklayın.”
- Çizimin henüz revizyonu yoksa “Açık çizimin dayandığı revizyon: 4” yerine “Açık çizimin henüz revizyonu yok” yazılır.

## 4. Kaydet

Kaydet çizimi `base`'in üstüne yazar. Beklenen sürüm `expectedVersions["@file"] = base`'tir; ilk kayıtta `"0"`dır. Kaydet'in ilk adımı şu sırayla seçilir (`saveStep`):

1. `ended`: proje bittiyse hiçbir şey yazılmaz. İleti yerel dosya önerir.
2. `readonly`: hesap yazamıyorsa hiçbir şey yazılmaz. İleti Farklı kaydet'i önerir.
3. `conflict`: bir çakışma duruyorsa soru yeniden sorulur; hiçbir şey yüklenmez.
4. `unchanged`: çizim `base`'in kendisiyse yazılacak bir şey yoktur (“zaten kaydedilmiş”); önceki başarısızlık unutulur. Revizyonu olmayan proje temiz olsa da ilk revizyonunu yazar.
5. `behind`: yeni bir revizyon biliniyorsa sunucu bu kaydı reddeder. Bu yüzden **hiçbir şey kodlanmaz ve yüklenmez**. Durum reddedilmiş gibi olur (`refused`); çakışma sorusu hemen gelir. Kullanıcı için reddedilen Kaydet'ten farkı yoktur; yalnız dosya boşuna gitmez.
6. `save`: aşamalar sırayla gelir: kodlama, yükleme, doğrulama. “Kaydedildi” ancak sunucu revizyonu yazınca denir.

Sunucu bir kaydı `@file` çakışmasıyla reddederse sırayla şunlar olur:

- Hiçbir şey yazılmaz; çizim olduğu gibi kalır.
- Durum `refused` olur ve çakışma sorusu gelir.
- Ret yalnız numarayı söyler. Kim ve ne zaman ardından sorulur; soru açıksa sonraki ipucu ve soru onları bilir.

Çakışma durduğu sürece her Kaydet soruyu yeniden sorar. Proje hiçbir zaman kendiliğinden kaydedilmez.

## 5. Sorular ve cevaplar

### 5.1 Hangi soru gelir

İki yoldan sorulur (`offer`):

- **`newest`**: hücrenin yeni revizyondaki tıklaması ya da Son revizyonu aç….
- **`conflict`**: reddedilen Kaydet ya da Kayıt çakışmalarını çöz….

| Yol | Durum | Gelen |
|---|---|---|
| `newest` | Kaydet sürüyor | Soru yok. Bilgi satırı: ““…” kaydediliyor; son revizyonu açmak için kaydın bitmesini bekleyin.” |
| `newest` | Proje silinmiş ya da erişim kalkmış | Soru yok. Uyarı satırı: “… son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin.” |
| `newest` | Çakışma var, ya da kaydedilmemiş iş üstünde yeni revizyon | Çakışma sorusu (§5.2) |
| `newest` | Kaydedilmemiş iş var, yeni revizyon bilinmiyor | Kaydedilmemiş değişiklikler sorusu (§5.3). En yenisi çizimin kendi `base`'i olabilir; onu açmak işi atar. |
| `newest` | Temiz çizim; arşivlenmiş proje de | Son revizyonu aç sorusu (§5.4) |
| `conflict` | Çakışma ya da yeni revizyon var | Çakışma sorusu |
| `conflict` | Hiçbiri yok | Hiçbir şey |

Esc, × ve arka plan her soruda hiçbir şeyi değiştirmeyen cevaptır.

### 5.2 Çakışma sorusu

- **Başlık:** “Dosya başka biri tarafından kaydedildi”.
- **Soru:** ““Kadastro paftası 2026” siz çalışırken başka biri tarafından kaydedildi: sunucuda revizyon 5 (Mehmet Demir, 27.09.2026 14:32) var; çiziminizin dayandığı revizyon 4. Hiçbir şey yazılmadı; iki dosya birleştirilmez.”
  - Kim ve ne zaman, yalnız o revizyon için biliniyorsa yazılır.
  - Çizimin revizyonu yoksa sonu şöyle olur: “çiziminiz henüz kaydedilmiş bir revizyona dayanmıyor”.
- **Maddeler:** her cevabın ne yaptığı. Temiz çizimde son madde “Son revizyonu aç: revizyon 5 açılır.” diye biter; atılacak iş yoktur.

Cevaplar düğme çubuğunun sırasıyladır:

| Cevap | Düğme | Yaptığı | Kaydedilmemiş iş |
|---|---|---|---|
| Son revizyonu aç | Kırmızı çerçeveli, solda | En yeni revizyonu açar | Bilerek atılır; kurtarma kopyası da silinir |
| Vazgeç | Olağan | Hiçbir şey | Durur; çakışma da durur |
| Yerel dosyaya kaydet | Olağan | Farklı kaydet; yazılınca çizim bulut projesinden ayrılır | Yerel dosyaya kaydedilir |
| Ayrı kopya olarak kaydet | Amber (Enter) | Yükleme penceresini dosya saklamasıyla ve “… (kopya)” adıyla açar | Yeni projenin 1. revizyonu olur; açık proje o olur |

Temiz çizimde “kaydedilmemiş iş” sütunu `none`'dır.

### 5.3 Kaydedilmemiş değişiklikler sorusu

- **Başlık:** “Kaydedilmemiş değişiklikler”.
- **Soru:** ““Kadastro paftası 2026” içinde kaydedilmemiş değişiklikler var. Sunucudaki en yeni revizyon açılırsa bu değişiklikler atılır (açık çizim revizyon 4). Saklamak için önce Kaydet ile kaydedin.” Revizyon yoksa parantezin içi “açık çizimin henüz revizyonu yok” olur.

| Cevap | Düğme | Yaptığı | Kaydedilmemiş iş |
|---|---|---|---|
| Kaydetmeden aç | Solda | En yeni revizyonu açar | Bilerek atılır; kurtarma kopyası da silinir |
| Vazgeç | Olağan (odakta) | Hiçbir şey | Durur |

### 5.4 Son revizyonu aç sorusu

- **Başlık:** “Son revizyonu aç”.
- **Yeni revizyon biliniyorsa:** ““…” başka bir yerde kaydedildi: revizyon 5 (Mehmet Demir, 27.09.2026 14:32). Açık çizim revizyon 4; kaydedilmemiş değişikliği yok.”
- **Bilinmiyorsa:** ““…” projesinin sunucudaki en yeni revizyonu açılsın mı? Açık çizim revizyon 4; kaydedilmemiş değişikliği yok.”

| Cevap | Düğme | Yaptığı |
|---|---|---|
| Sonra | Olağan | Hiçbir şey; hücre “Yeni revizyon” demeyi sürdürür |
| Son revizyonu aç | Amber (Enter) | En yeni revizyonu açar |

### 5.5 Cevapların yaptığı

- **En yeni revizyonu açmak** projeyi yeniden açar. Açılış aşamalıdır ve durdurulabilir; indirilen baytlar sunucunun özetiyle denetlenir (ADR 0038). Çizim yalnız bütün revizyon okununca değişir. Yeni `base` en yeni revizyondur; `newer` ve `conflict` boşalır, çizim temizdir. Açılamazsa ileti şudur: ““…” son revizyonu açılamadı: <neden>”; çizim olduğu gibi kalır.
- **Kurtarma kopyası:**
  - Web'de kaydedilmemiş iş yalnız çizimde ve kurtarma kopyasındadır (ADR 0030); dosya projesinin cihaz taslağı yoktur.
  - İş bilerek atılınca kopya da silinir.
  - Kaydedilince kopya kendiliğinden gider.
  - Vazgeç kopyayı olduğu gibi bırakır.
- **Masaüstü:** cihazda tutulan kayıt ve yerel kopya bu soruda “kaydedilmemiş iş” sayılır. Atılan iş onlardan da silinir; Vazgeç onlara dokunmaz (ADR 0043).
- **Yerel dosyaya kaydet:** Farklı kaydet vazgeçilirse hiçbir şey olmaz. Yazılırsa bir bilgi satırı eklenir: “Çizim yerel dosyaya kaydedildi ve “…” bulut projesinden ayrıldı; proje olduğu gibi duruyor.”

## 6. Geçmiş sekmesi

- **Yenilenme:** Geçmiş sekmesi projenin olaylarını dinler. `project.file` (bu pencerenin kendi kaydı da) ya da `project.checkpoint` olayı listeyi 250 ms sonra yeniden sorar. Yanıt gelene kadar liste olduğu gibi kalır. Kanal yeniden eşitlenirse de yeniden sorulur.
- **İşaretler (`revisionMarks`):** revizyon satırında sırasıyla iki işaret olabilir:
  - “En yeni”: sunucunun en yeni revizyonu.
  - “Açık çizim”: proje bu pencerede dosya projesi olarak açıksa, çizimin dayandığı revizyon.
- **İşaretin izlenmesi:** bu pencerenin Kaydet'i ya da son revizyonun açılması `base`'i değiştirince işaret yerini değiştirir.
- **Revizyon açma:** geçmişte hiçbir satır açık çizimin üstüne revizyon açmaz. Geri yükleme yeni bir projedir (ADR 0034); en yeni revizyon yalnız §5'teki yoldan açılır.

## 7. 28 Eylül'de web'de değişenler

Bu belge yazılırken web'de şunlar düzeltildi ya da eklendi:

1. **Yeniden eşitleme:** dosya projesi artık yeniden açılmıyor (§2.4). Önceden proje sunucudan yeniden açılıyor ve çizimi değiştiriyordu. Kaydedilmemiş iş kurtarma kopyasına gidiyordu ama bu, ADR 0038'in “başkasının revizyonu kendiliğinden yüklenmez” kuralına aykırıydı.
2. **Kaydedilmemiş iş üstünde yeni revizyon:** hücre bunu söylüyor: “Yeni revizyon: r5 · kaydedilmedi”. Tıklama çakışma sorusunu getiriyor. Önceden hücre “Kaydedilmedi · r4 üstüne” diyordu; yeni revizyon yalnız ipucunda ve günlükteydi.
3. **Kaydet yüklemiyor:** yeni revizyon biliniyorsa Kaydet hiçbir şey yüklemeden soruyu getiriyor (`behind`). Önceden dosya kodlanıp yükleniyor, sonra reddediliyordu.
4. **Geç gelen cevap:** eski bir cevap bilinen yeni revizyonu artık düşürmüyor.
5. **Kim ve ne zaman:** yeni revizyonun zamanı ipucunda, günlükte ve soruda yazılıyor. Reddedilen Kaydet'ten sonra kim ve ne zaman sunucuya soruluyor.
6. **Salt okunur hesap:** yeni revizyonu görüyor ve açabiliyor. Önceden hücre “Salt okunur” diyordu ve tıklama bir şey yapmıyordu.
7. **Kaydet sürerken:** Son revizyonu aç soru sormuyor, beklemeyi söylüyor. Silinmiş ya da erişimi kalkmış projede neden açılamayacağını söylüyor.
8. **Revizyonu olmayan proje:** sorular “revizyon 0” demiyor.
9. **Geçmiş:** açık çizimin revizyonu işaretleniyor.
10. **Kullanılmayan kod:** `FileProjectSave.settle` kaldırıldı. Uygulamanın hiçbir yolu onu çağırmıyordu: üç seçim de oturumu değiştiriyor.

## 8. Doğrulama

- **Dosya:** `python3 scripts/fixtures/file_revisions_cases.py --check` dosyanın bağımsız yazıcıyla aynı olduğunu denetler.
- **Web'in planı:** `apps/web/src/app/cloud/fileRevisionsPlan.test.ts` web'i dosyaya bağlar:
  - sözler;
  - durum ve girdiler;
  - hücre;
  - Kaydet'in ilk adımı;
  - olaylar ve yeniden eşitleme;
  - sorular;
  - satırlar ve ipucu;
  - işaretler;
  - on sıra (`traces`).
- **Sınıf ve oturum:** `fileProject.test.ts` ve `fileSession.test.ts` sahte sunucuyla şunları sınar: Kaydet'in yüklemediği durum, geç gelen cevap, yeniden eşitleme ve sonuçları, oturumun yeniden eşitlemede çizimi açmaması.
- **Gerçek sunucu:** `pnpm e2e:cloud` şunları denetler:
  - kaydedilmemiş iş üstünde yeni revizyonun hücresi;
  - yeni revizyon biliniyorken Kaydet'in hiçbir şey yüklememesi;
  - yeniden eşitlemenin çizimi değiştirmemesi ve olayları yeni imleçten sürdürmesi;
  - Geçmiş'teki “Açık çizim” işareti.
