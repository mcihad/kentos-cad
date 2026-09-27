# ADR 0119: Masaüstünde dosya projesinin revizyonları: başkasının revizyonu, Kaydet'in ilk adımı, sorular

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** [docs/specs/file-revisions.md](../specs/file-revisions.md); ADR 0031 (dosya projeleri), 0038 (web'de dosya projeleri), 0043 (masaüstünün yerel kopyası), 0044 (olayların uzun sorguyla izlenmesi), 0113 (durum çubuğunun bulut hücreleri).
- **Kaynak:** web'in `app/cloud/fileRevisionsPlan.ts`, `fileProject.ts` ve `ui/cloud/FileConflict.ts`'i; ortak durumlar `fixtures/cloud/v1/file-revisions.json`.

## Bağlam

Masaüstü açık bir dosya projesinin olaylarını izlemiyordu. Başkası yeni bir revizyon kaydettiğinde bunu duymuyordu. Kayıt hücresi hiçbir zaman “Yeni revizyon” demiyordu; çakışma ancak Kaydet'te, dosya kodlanıp yüklendikten sonra ortaya çıkıyordu. Çakışma sorusunda web'in dört cevabından “Yerel dosyaya kaydet” yoktu; sözleri de ayrıydı (eşitlik denetimi, `docs/inventory/parity-audit.md`, 1. ve 2. madde, B1–B4).

Web'in ajanı 28 Eylül'de web'in davranışını plan olarak yazdı: saf kurallar (`fileRevisionsPlan.ts`), belgesi ve fixture'ı. Bu karar planın masaüstündeki karşılığıdır.

## Karar

- **Kurallar** `apps/desktop/src/cloud/revisions.rs`'tedir; web'in planının birebir karşılığıdır:
  - bilinenler (`RevisionState`) ve onları değiştiren girdiler (`step`);
  - hücrenin durumu (`cell_state`) ve Kaydet'in ilk adımı (`save_step`);
  - olaylar (`read_events`), en yeni revizyon (`newest_of`), yeniden eşitleme (`resync_step`);
  - sorular (`offer`, üç soru ve cevapları) ve sözler.

  `revisions_tests.rs` fixture'ın bütün bölümlerini oynatır: sözler, durumlar, girdiler, Kaydet'in adımları, olaylar, yeniden eşitleme, sorular, günlük satırları, ipuçları, hücreler, Geçmiş'in işaretleri ve on sıra.
- **Bilinenler masaüstünde tek yerde tutulmaz,** var olan parçalardan toplanır (`App::file_revisions`):
  - çizimin dayandığı revizyon kaynağından, kaydedilmemiş iş çizimden gelir;
  - Kaydet'in aşaması kayıt işçisinden, son başarısızlık ve çakışma eskisi gibi kendi alanlarından gelir;
  - yeni revizyon ve projenin bitişi olayları izleyenden (`FileFollow`) gelir.

  Bu cihazda tutulan kayıt (ADR 0043) kaydedilmemiş iş sayılır: henüz hiçbir revizyonda değildir. Bir girdi (`file_step`) planın `step`'iyle uygulanır; yeni revizyon ve çakışmanın revizyonu geri yazılır, ilk kez duyulan revizyon günlüğe bir kez yazılır.
- **İzleme** `apps/desktop/src/cloud/file_follow.rs`'tedir:
  - Açılan dosya projesi, okunduğu andaki olay imlecinden izlenir. Arşivlenmiş proje izlenmez: oraya bir şey kaydedilmez. Bu cihazdaki kopyadan açılan proje de izlenir; bağlantı dönünce kaçırılan olaylar gelir.
  - Olaylar veritabanı projesininki gibi uzun sorguyla beklenir (ADR 0044). Yanıt gelmezse sonraki deneme 1 sn sonra, iki katına çıkarak en çok 30 sn sonra yapılır.
  - Başkasının `project.file` olayı sunucuya en yeni revizyonu sordurur: kim ve ne zaman. Çizim hiçbir zaman kendiliğinden yeniden yüklenmez.
  - Bu pencerenin kendi kaydının olayı başkasınınki sayılmaz. Bunun için kaydın istek kimliği gönderilmeden önce saklanır; `kentos-cloud`'un yeni `saving::save_revision_sent`'i kimliği çağırandan alır. Cihazda tutulan kaydın gönderilmesi de böyledir.
  - `project.access` erişimi yeniden sordurur; rol değişince söylenir.
  - `project.deleted` ya da `project.archived` kaydı bitirir. Bitiş günlükte söylenir, kopyaya yazılır ve bildirim açılır. Arşivleyen bu pencereyse sessizdir.
  - Kaçırılan olaylar artık verilemiyorsa (`resync_required`) proje sorulur. Proje açılmaz, çizim değişmez. Cevap planın `resync_step`'iyle yorumlanır: yeni imleçten izleme (erişim ve en yeni revizyon da sorulur), bitiş ya da 30 sn sonra yeniden deneme. Yeniden deneme bir kez söylenir.
- **Kaydet** planın ilk adımıyla karar verir:
  - bitmiş ya da salt okunur projede bir şey yazılmaz;
  - çakışma dururken soru yeniden sorulur;
  - çizim dayandığı revizyonun kendisiyse “zaten kaydedilmiş” denir;
  - yeni bir revizyon biliniyorsa hiçbir şey kodlanmaz ve yüklenmez: durum reddedilmiş gibi olur, çakışma sorusu hemen gelir.

  Sunucunun reddi yalnız revizyonun numarasını söyler. Kim ve ne zaman ardından sorulur.
- **Sorular** tek pencerededir: `Dialog::Revision` (eskiden üç ayrı pencereydi). Sorunun sözleri ve cevapları planındır:
  - **Çakışma sorusu:** Son revizyonu aç (kırmızı çerçeveli, solda), Vazgeç, Yerel dosyaya kaydet, Ayrı kopya olarak kaydet (amber, Enter).
  - **Kaydedilmemiş değişiklikler sorusu:** Kaydetmeden aç (solda), Vazgeç.
  - **Son revizyonu aç sorusu:** Sonra, Son revizyonu aç.

  Esc, × ve arka plan hiçbir şeyi değiştirmeyen cevaptır. Cevapların yaptığı:
  - **Son revizyonu aç:** bilerek atılan iş kurtarma kopyasından da, cihazda tutulan kayıttan da silinir. Proje yalnız sunucudan açılır, bu cihazdaki kopyaya düşülmez: kopya en yeni revizyon değildir. Açılamazsa “… son revizyonu açılamadı: …” denir; çizim ve çakışma olduğu gibi kalır. Çakışma artık açılış başlarken değil, proje okununca temizlenir.
  - **Yerel dosyaya kaydet:** Farklı kaydet. Dosya yazılınca çizim projeden ayrılır, cihazda tutulan kayıt silinir ve günlüğe “Çizim yerel dosyaya kaydedildi ve … bulut projesinden ayrıldı; proje olduğu gibi duruyor.” yazılır.
  - **Ayrı kopya olarak kaydet:** yükleme penceresi, dosya saklamasıyla ve “… (kopya)” adıyla açılır.
- **Hücre ve ipucu:** hücrenin durumu planın `cell_state`'idir. Kaydedilmemiş iş üstünde yeni revizyon “Yeni revizyon: r5 · kaydedilmedi” der. İpucu yeni revizyonu kim ve ne zaman ile söyler (`newer_tip`). İzlenen projede “Canlı bağlantı” satırı yazılır.
- **Geçmiş:** revizyon satırı “En yeni” ile birlikte “Açık çizim” işaretini de taşır (`revision_marks`); bu işaret projenin bu pencerede açık olan çizimin dayandığı revizyondur.
- **KentOS UI'a eklenenler:** `Dialog::aside` (düğme çubuğunun soluna ayrılan cevap) ve `style::button::danger_outline` (web'in `.btn--danger`'ı: kenarlı, yazısı tehlike renginde).

## Sonuçlar

- Masaüstünün dosya projesi web'inkiyle aynı kurallarla davranır; kurallar değişirse fixture iki platformu birden düşürür.
- Veritabanı projesinin izlenmesi değişmedi.
- Aynı düzeltmede eşitlik denetiminin bulduğu bir hata da giderildi: kalıcı olarak reddedilen yüklemenin boş projesi çöpe gider, sonraki deneme yeni bir istek anahtarıyla yeni yüklemedir. Yanıt gelmeyen denemenin anahtarı aynı kalır.

## Doğrulama

- `cargo test -p kentos-desktop cloud::` bulutun bütün testlerini çalıştırır. Bunlar arasında fixture'ı oynatan testler (`revisions_tests`) ve izlemenin testleri (`file_follow_tests`) vardır. İzlemenin testleri şunları sınar:
  - başkasının revizyonu duyulur, bir kez söylenir ve sunulur;
  - kendi kaydının olayı sorulmaz, geç gelen eski cevap bilineni düşürmez;
  - bilinen yeni revizyonun üstüne Kaydet hiçbir şey yüklemez;
  - reddedilen Kaydet'in dört cevabı çalışır;
  - yeniden eşitleme çizimi açmaz;
  - silinen ya da arşivlenen projede kayıt biter.
- `apps/desktop/scripts/cloud-live.sh` aynı akışı gerçek `kentosd` ile koşar: başkasının kaydettiği revizyon duyulur, hücre söyler, Kaydet yüklemeden sorar.
- Resimler `cargo test -p kentos-desktop cloud::file_follow_tests::revision_screens -- --ignored --nocapture` ile `.run/shots/bulut-revizyon-*`'a çekilir.
