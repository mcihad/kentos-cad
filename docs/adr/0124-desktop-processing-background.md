# ADR 0124: Masaüstünde işlem araçları arka planda çalışır

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0084 (İşlemler), ADR 0020 (belge), ADR 0120 (karenin maliyeti); docs/PROCESSING.md; TODOS.md PERF-08.

## Bağlam

Masaüstünde İşlemler araçları arayüz iş parçacığında çalışıyordu. Hesap sürerken pencere donuyordu. 50 000 parselde ölçülenler:

- Köşe noktalarını numarala: 629 ms;
- Kenar uzunluklarını yaz: 430 ms;
- Parsel ölçü yazıları modeli: 1,1 sn.

Web büyük işi Worker'da çalıştırır (`workerExecutor.ts`):

- **Otomatik kuralı:** girdisi 2 000 ya da daha çok nesne olan iş arka plana gider (`WORKER_THRESHOLD`). Pencerenin "Nerede çalışır" seçimi bunu değiştirir: Otomatik, Bu tarayıcıda ya da Arka planda.
- **İlerleme:** pencerenin alt çubuğunda ilerleme çubuğu ve aracın adımı görünür.
- **Durdur:** Kapat düğmesi iş sürerken Durdur olur. Worker'ı hemen kapatır; çizim değişmez.
- **Uygulama:** sonuç sayfada, tek geri alma adımında uygulanır. Bu arada silinen nesneler ve kilitlenen katmanlar atlanır, sayılır.

Masaüstü bunların hiçbirini yapmıyordu. "Nerede çalışır"da yalnız "Bu bilgisayarda" seçilebiliyordu, öbür yerler "yakında" diye duruyordu.

## Karar

- **Çalıştırıcı üç adıma bölündü** (`kentos-processing`, `runner.rs`). Web'in `RunJob` modelidir:
  - `prepare`: değerleri doğrular. Çizime bağlı olanları (girdilerin nesneleri, yazılacak katmanlar, seçim) sahipli bir işe (`Job`) çözer.
  - `compute`: aracın hesabıdır. Çizimi yalnız okur; herhangi bir iş parçacığında çalışabilir.
  - `finish`: değişiklikleri çizime tek geri alma adımında yazar, seçimi kurar, çalıştırmayı geçmişe kaydeder.
  - `run` üçünü art arda yapar (`complete`). Durdurulan ve hesabı kopan iş için `stop` ve `failed` geçmişe iptal ve hata kaydı yazar. Kayıt işin nerede çalıştığını da taşır (`Job::target`).
- **Belgenin okuma kopyası** (`kentos-domain`): `Document::reading_copy` çizimin şimdiki hâlini geri alma geçmişi olmadan verir. Nesneler kopyalanmaz, paylaşılır (`Arc`). Arka plandaki hesap bu kopyada çalışır; çizim kullanılmaya devam eder.
- **Arka plan** (`apps/desktop/src/processing/background.rs`): iş kendi iş parçacığında hesaplanır. İş parçacığı web'in Worker'ı gibi konuşur:
  - ilerleme en çok 50 ms'de bir gelir, son adım her zaman;
  - aracın iletileri geldikçe komut geçmişine yazılır;
  - sonuç gelince arayüz iş parçacığında `finish` ile uygulanır.

  Bağlantı iced'in `Task::run`'ıdır, ayrı bir zamanlayıcı yoktur. Hesap çökerse (panic) çalıştırma web'in sözleriyle hata olur: "“…” çalışırken hata: …".
- **Durdur:** işi hemen bitirir, web'in Worker'ı kapatması gibi. Araç durması için uyarılır (bakan araçlar erken durur). İş parçacığının sonradan söyledikleri dinlenmez. Çizim değişmez; kayıt "İşlem iptal edildi; çizim değişmedi."dir.
- **Çizim değişirse:** sonuç yalnız hesaplandığı çizime uygulanır (`session`). Başka bir çizim açıldıysa hiçbir şey uygulanmaz, çalıştırma iptal sayılır.
- **Pencere:** alt çubuk ve durum satırı web'in planıyla kurulur (`plan.rs`: `status_line`, `footer_of`):
  - iş sürerken ilerleme çubuğu (140 px, sönük) ve adım görünür;
  - Çalıştır "Çalışıyor…" olur ve beklenir; Varsayılanlar da bekler;
  - Kapat Durdur olur.

  Pencere Esc ya da × ile kapanabilir; iş sürer, sonucu komut geçmişine yazılır. Pencere bir alanı çizimden seçmek için kenara çekildiyse (Sahneden seç), sonucu döndüğünde görünür.
- **Nerede çalışır** web'in planıyla kurulur (`targets_view`, `effective_choice`):
  - önce Otomatik ve şimdi neyi seçtiği ("şimdi: arka planda"), altında kuralı;
  - sonra aracın adlandırdığı her yer;
  - bu programda olmayanlar (sunucu, PostGIS) "yakında"dır.

  Seçim araç başına `islemler.json`'da saklanır, web'deki gibi. Seçenek satırlarının notu satırın sonunda durur, kural Otomatik'in altındadır. Bunun için KentOS UI `RadioGroup`'a `notes_at_end`, `hint` ve `disabled_with` eklendi.
- **Sözcükler:** masaüstü web'in sayfasından değil kendinden söz eder: "Bu bilgisayarda", "Arka planda", "Arka planda çalışıyor; Durdur ile durdurabilirsiniz.", "…; program donmaz." Testler web'in sözcüklerini bunlarla değiştirerek aynı dosyayı oynar.

## Sonuçlar

- 2 000 ve daha çok nesneli bir aracın hesabı artık arayüzü dondurmaz. Pencere ilerlemeyi gösterir, Durdur hemen yanıt verir.
- Arayüz iş parçacığında kalanlar:
  - hazırlık (girdilerin çözülmesi) ve okuma kopyası (100 000 nesnede birkaç ms);
  - sonucun tek adımda uygulanması.
- **Modeller** (Parsel ölçü yazıları) adımlarını hâlâ burada, art arda çalıştırır. "Nerede çalışır"da yalnız "Bu bilgisayarda" vardır. Adımlar arka plana gidince model birden çok güncelleme boyunca açık bir geri alma grubu tutar. Bu arada kullanıcının yaptığı düzenleme modelin adımına katılmamalıdır. Bu ayrı bir dilimdir (TODOS.md PERF-08).
- İş sürerken yapılan düzenlemeler sonuçla çakışabilir. Web'deki gibi: silinen nesnenin değişikliği ve kilitlenen katmana yazılanlar atlanır ve sayılır.

## Doğrulama

- `cargo test -p kentos-processing`: `fixtures/processing/v1`'in araç durumlarının her biri iki yoldan oynanır:
  - `run` ile;
  - `prepare`, çizimin kopyasında `compute` ve çizimde `finish` ile.

  İkisi aynı sonucu verir. İş, belge ve sonuç `Send`'dir.
- `cargo test -p kentos-domain --test document`: okuma kopyası nesneleri, kimlikleri, sırayı, katmanları ve ayarları taşır. Geri alma taşımaz. Çizim değişince kopya değişmez.
- `cargo test -p kentos-desktop processing::plan`: `fixtures/processing/v1/dialog.json`'ın durum satırı ve alt çubuk durumları (16) ile "Nerede çalışır" durumları (10) web'in planını verir.
- `cargo test -p kentos-desktop processing::tests`:
  - arka plandaki çalıştırma buradakinin aynısını yapar: tek geri alma adımı, geçmişte "arka planda";
  - Durdur işi hemen bitirir, geç gelen sonuç uygulanmaz;
  - Otomatik 2 100 nesneyi arka plana gönderir ve buradakiyle aynı sonucu verir.
- `cargo test -p kentos-desktop processing::tests::screens -- --ignored --nocapture`: pencerenin resimleri (arka planda çalışırken, durdurulunca, Nerede çalışır), `.run/shots/islem-*`.
