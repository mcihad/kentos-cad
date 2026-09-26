# ADR 0043: Masaüstünde çevrimdışı çalışma: bulut projesinin yerel kopyası

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §13, §21; ADR 0026 (kalıcı kimlikle eşitleme), 0031 (dosya projeleri), 0040 (masaüstü bulut istemcisi)
- **Sahibin kararı (26 Eylül):**
  - Kararın sözü: "İnternetsiz de çalışacak, sadece cloud değil yerel proje de olacak, sunucuda çizim komutları çalışmayacak ama proje kusursuz ve güçlü şekilde devamlı senkronize edilecek, bağlantısız da çalışabilecek."
  - Web için sahip ayrıca şunu söyledi: "web tarayıcı internetsiz olmaz."
  - Sonuç: çevrimdışı çalışma masaüstünündür. Web çevrimiçi kalır; IndexedDB taslakları gönderilmemiş düzenlemeleri korur. Çizim komutları sunucuda çalışmaz; sunucu saklar, yetkilendirir, sürümler, eşitler.

## Bağlam

- ADR 0040 ile masaüstü bulut projesini açar, düzenler, gönderir. Başkalarının değişikliklerini sorarak alır. Gönderilmemiş işi ve yoldaki komutu cihaz taslağında tutar.
- Açılış hâlâ sunucuya bağlıydı. Bağlantı yokken proje açılamıyordu. Başkalarının değişikliklerini almak ve taslağı geri koymak için de önce sunucudan tam açılış gerekiyordu.

## Karar

### Yerel kopya (`ReplicaStore`, `Replica`)

- Klasör düzeni: her sunucu, hesap ve proje için bir klasör. Masaüstünde `$XDG_DATA_HOME/kentos-cad/bulut-kopya` önerilir.
- Açık olduğu sürece bir programın kilidindedir (`kilit`, std'nin dosya kilidi). İkinci KentOS `InUse` alır ve "başka bir pencerede açık" der.
- Kopya sunucunun son bilinen hâlidir, bu cihazın işi değildir. Gönderilmemiş iş cihaz taslağında ayrı durur (ADR 0040).
- Dosyalar:
  - `bilgi.json`: projenin son listelenişi (ad, çalışma alanı, rol, durum, saklama biçimi). Çevrimdışı katalog yalnız bunları okur (`ReplicaStore::list`).
  - `taban-<n>.kcad` ve `taban-<n>.json`: sunucunun çizimi bir anda (KCAD v2, her nesne kalıcı kimliğiyle), yanında her nesnenin sürümü, üst veri sürümü, olay imleci. Dosya projesinde ayrıca kopyanın hangi revizyon olduğu yazar.
  - `taban-<n>.log` (veritabanı projesi): o andan sonra öğrenilenler, satır başına bir `BaseStep`. Her satır eklendiği anda diske işlenir.
  - `guncel`: geçerli nesil `n`. Toplama (`compact`), `n + 1`'i bütünüyle yazıp diske işler, sonra `guncel`'i değiştirir. Çökme ya eskiyi ya yeniyi bırakır, karışığını değil.
  - `bekleyen.kcad` ve `bekleyen.json` (dosya projesi): bağlantısız yapılmış kayıt ve dayandığı revizyon, gönderilmeyi bekler.
- Günlükte yalnız son satır yarım kalabilir (çökme). Okurken o satır atılır. Ortadaki bozuk bir satır kopyayı okunamaz yapar; proje sunucudan yeniden açılır, taslak onun üstüne konur.
- Kopyada oturum, parola ya da adres yoktur. Klasörü yalnız sunucunun adını taşır (TODOS.md `SYNC-13`).

### Sunucunun hâli bu cihazda (`BaseStep`, `ProjectSync::take_base_step`)

- Eşitlemenin sunucu bilgisi dört yoldan değişir:
  - onaylanan komut;
  - alınan başkası-değişikliği;
  - çakışmada benimki ya da sunucudaki seçimi;
  - yeni olay imleci.

  Her değişiklik bir adımda toplanır. Masaüstü adımı alır, kopyanın günlüğüne ekler, **sonra** yeni taslağı yazar.
- Diskteki kopya ile taslak böylece hep uyuşur:
  - çökme ya ikisini de adımdan önce bırakır;
  - ya da yeni kopyayı eski taslakla bırakır; eski taslaktaki yoldaki komutu sunucu kaydından yeniden yanıtlar.
- `ProjectSync::base` bütün tabanı verir (nesneler kimlik sırasında, sürümleri, üst veri, imleç). Kopya ondan toplanır: günlük uzayınca ve proje kapanırken.

### Çevrimdışı açma ve yeniden bağlanma

- `Replica::load`, çevrimiçi açılışla aynı yapıyı (`Opened`) verir. Veritabanı projesinde nesneler sunucunun kimlik sırasındadır.
- Bundan sonra eşitleme, taslağın geri konması, olay izleme ve gönderme aynen çalışır; bağlantısızken gönderim `Offline` olur ve bekler.
- Bağlantı dönünce olaylar kopyanın imlecinden alınır:
  - **Bu cihazın bildiği sürüm atlanır.** Yeniden başlatmadan önce yazılmış kendi komutlarının olayları artık kendi istek kimliğiyle tanınmaz, ama sürümü tabanda vardır. Bunlar başkası-değişikliği ya da çakışma sayılmaz. Tabanda olmayan ve çizimde de olmayan bir silme de bir şey yapmaz.
  - Saklanmayan imleç `resync_required` alır: proje sunucudan açılır, kopya ondan yeniden kurulur (`reset`), taslak üstüne konur, çakışmalar gösterilir.

### Bu dilimde bulunan hata

- Taslak geri konurken, yoldaki komutun taşıdığı değişiklikler "komut yanıtlanınca yerine oturur" diye çizime konmuyordu. Web onları komutun yanıtından sonra sunucudan getirir (`takeServerCopies`).
- Sonuç: bağlantısız açılışta çevrimdışı eklenen nokta çizimde görünmüyordu. Komut yanıtlanınca sunucu doğru hâle geliyor, bu cihazın çizimi eski kalıyordu.
- Çevrimdışı senaryo testi bunu yakaladı. Düzeltme: bu değişiklikler hemen çizime konur ve komut yanıtlanana kadar gönderilmemiş iş sayılır. Komut kalıcı olarak reddedilirse de çizimde kalır ve bekler.

## Bu dilimde olmayanlar

- **Masaüstü arayüzü:**
  - çevrimdışı katalog ("Bu cihazdaki projeler");
  - çevrimiçi/çevrimdışı göstergesi;
  - kopyanın ne zaman yazılacağı ve toplanacağı;
  - dosya projesinde bekleyen kayıt.

  Bunlar masaüstü ajanına gider; bulut arayüzü dilimiyle birlikte (ADR 0041).
- **Masaüstünde WebSocket:** olaylar bugün birkaç saniyede bir sorulur.
- **Uzun çevrimdışı dönemin yetki sınırı:**
  - erişimi o arada kaldırılmış ya da çöpe atılmış projede iş taslakta kalır ve söylenir (ADR 0040);
  - kopyanın ne kadar süre güvenilir sayılacağı (kurum politikası) ayrı karardır.
- **Kopyanın şifrelenmesi:** cihaz diski işletim sisteminindir; kurum isterse ayrı karar.

## Doğrulama (26 Eylül 2026, Linux)

- **`kentos-cloud` birim testleri (43):**
  - kopya çevrimiçi açılışla aynı açılır ve çevrimdışı katalogda listelenir;
  - adımlar sırayla işlenir (sürüm, silme, üst veri, imleç);
  - yarım son satır atılır, ortadaki bozuk satır kopyayı reddeder;
  - toplama eşitlemenin tabanından aynı çizimi verir ve tek nesil bırakır;
  - ikinci program `InUse` alır;
  - dosya projesi revizyonuyla ve bağlantısız kaydıyla açılır;
  - her sunucu-hâli değişikliği bir taban adımıdır;
  - bilinen sürüm ne yeniden alınır ne çakışma olur, bilinen silme bir şey yapmaz;
  - yoldaki komutun işi taslak geri konunca hemen çizimdedir, yanıtlanınca biter, reddedilirse bekler.
- **Gerçek sunucu, `a_project_goes_on_offline_and_catches_up_when_the_connection_returns`:**
  1. Ayşe çevrimiçi açar, kopya tutulur, bir değişiklik gider ve tabana eklenir.
  2. Dilek çevrimiçi olarak çizgiyi etiketler ve bir nokta ekler.
  3. Ayşe bağlantısız açar: çizim kopyadan, kendi değişikliğiyle ve Dilek'inkiler olmadan gelir. Nokta ekler, çoklu çizgiyi siler; gönderim ulaşılamayan sunucuda geçici hata alır (`Offline`); iş taslakta kalır.
  4. Bağlantı dönünce kopya açılır, taslak geri konur (`resends`), olaylar kopyanın imlecinden alınır. Ayşe'nin ilk oturumdaki komutu bilinen sürüm olarak atlanır, çakışma çıkmaz. Dilek'in işi gelir, Ayşe'nin çevrimdışı işi gider.
  5. Sunucu, Ayşe'nin çizimi ve kopya nesne nesne aynıdır; toplama sonrası da.
- Bu test yukarıdaki hatayı buldu; düzeltmeden önce düşüyordu.
- **Gerçek sunucu, `a_long_offline_spell_past_the_kept_events_reopens_and_loses_nothing`:**
  1. Ayşe'nin kopyası tutulur, program kapanır.
  2. Dilek çizgiyi etiketler ve bir nokta ekler; olaylar 8 günlük yapılıp temizlenir. Sunucu Ayşe'nin imlecinden sonrasını artık saklamaz.
  3. Ayşe bağlantısız olarak noktayı ve aynı çizgiyi değiştirir; iş taslakta kalır.
  4. Bağlanınca imleç `resync_required` alır. Proje sunucudan açılır, kopya ondan yeniden kurulur, taslak üstüne konur. Aynı çizgi tek çakışmadır, çizimde Ayşe'nin kopyası görünür. Dilek'in noktası gelir.
  5. "Benimkini koru" ile her şey gider. Sunucu, çizim ve kopya aynıdır; hiçbir iş kaybolmaz.
- **Gerçek sunucu, `a_file_project_saved_offline_goes_out_when_connected_and_a_clash_keeps_both`:**
  1. Dosya projesi kopyadan 1. revizyonla açılır. Bağlantısız kayıt bekler; bağlanınca 2. revizyon olur, bekleyen kayıt temizlenir.
  2. Yeniden bağlantısızken bekleyen kayıt, arada Dilek'in kaydettiği 3. revizyonla karşılaşır (`conflicting_revision` 3).
  3. İki iş de korunur: Dilek'inki projenin en yenisi kalır, bu cihazın işi ayrı dosya projesi olarak yüklenir. Nesne nesne ikisi de doğrudur.
- **Ölçüm** (`docs/perf/replica-desktop-2026-09-26.md`; i5-11300H, ext4 NVMe, sürüm derlemesi):
  - 100 bin parselin kopyası: kurulumu 584 ms, bağlantısız açılışı 392 ms, toplanması 609 ms, boyu 52,6 MB;
  - bir adımın diske işlenmesi yaklaşık 1,4 ms, projenin büyüklüğünden bağımsız.
