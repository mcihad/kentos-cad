# ADR 0737: Veri girişinin sunucuda birleştirilmesi, katmanlar arası ortak köşe ve taslağın kopyası

- **Durum:** kabul edildi (2026-10-29). `plan/gis.md` CBS-37'nin dördüncü dilimi. Kapsamı ben belirledim.
- **Kalan işler:** [ADR 0648](0648-server-field-merge-database-reconcile-and-shared-corners.md)'in "Sonraya kalan"ı:
  - Veri girişinin (`forms.use`) düzenlemesinin sunucuda birleştirilmesi (bugün çatışmadır);
  - katmanlar arası çok köşeli nesnenin bağı (taban katman başınadır);
  - planın tamamlanma ölçütündeki “kullanıcı kendi taslağını dışa alabilsin”: veritabanı projesinin çakışmasında taslağın
    yerel kopyası.
- **Bağlam belgesi:**
  - ADR 0411 (Veri girişi, gizli alanlar saklı kopyadan), 0550 (erişim çözücüsü);
  - ADR 0553 ve 0648 (alan alan birleştirme, gruplar, sunucuda `edit`);
  - ADR 0538 (kaynakla uzlaştırma), 0041 ve 0043 (masaüstünün bulut istemcisi, cihaz taslağı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. Tabanın köşe kaydının biçimi aynıdır; yalnız çok köşeli nesnede ne tuttuğu genişler (§2).
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - adlı çalışma sürümü ve review → publish akışı: planın dediği gibi ihtiyacı gerçek kullanımda kanıtlanınca;
  - web'in bulut istemcisinin `edit` göndermesi (web dururken).

## Bağlam

ADR 0648 sunucuda `project.changes`'in `edit`'ini alan alan birleştirdi; Veri girişini dışarıda bıraktı. Veri girişi, formunun
gizlediği alanları hiç okumaz ve yazdığı nesne o alanları saklı kopyadan alır (ADR 0411 §6). İki kural karışmasın diye
sürümü tutmayan bir Veri girişi düzenlemesi çatışma kalıyordu. Sahada formla veri giren biri, ofisteki düzenleyici aynı kaydın
şeklini ya da gizli notunu değiştirdi diye her seferinde çatışma alıyordu.

Kaynakla uzlaştırmanın tabanı, 128'den çok köşeli nesnede yalnız katmanın başka bir nesnesiyle paylaştığı köşeleri tutar
(ADR 0648 §4). `with` ile birlikte uzlaştırılan öbür katmandaki komşuyla paylaşılan köşe tutulmuyordu: çok köşeli parselle
üstündeki bina, köşelerini iki yan da taşısa bile ayrı ayrı çözülüyordu.

Planın tamamlanma ölçütü “kullanıcı kendi taslağını dışa alabilsin” der. Veritabanı projesinin çakışma penceresinde üç
seçenek vardı: Sonra, Benimkini kaydet, Sunucudakini al. Farklı kaydet çizimi yerel dosyaya yazar ama buluttan koparır.
Sunucudakini almadan önce kendi işini saklamak isteyen kullanıcının yolu yoktu.

## Karar

### §1 Veri girişinin `edit`'i sunucuda birleşir

`kentos_contracts::merge_reading(base, mine, theirs, reads)` saf kuraldır:

- `base` ve `mine` çağıranın okuduğu kopyalardır. Okumadığı öznitelikler (`reads` yanlış olanlar) ikisinden de çıkarılır,
  `theirs`'inkiler ikisine de konur.
- Sonra ADR 0553 §4'ün `merge_entities`'i çalışır.
- Okunmayan öznitelikler saklı kopyadaki gibi kalır. Değişiklikleri ne çatışmadır ne de `from_theirs`'te adı geçer: gizli
  bir alanın değiştiği Veri girişine söylenmez.

Sunucu (`changes::commit_in`):

- Sürümü tutmayan `edit` Veri girişinden de gelse birleştirilir. Okunabilirlik erişim çözücüsünün verdiğidir:
  `reach.level(katman, alan).readable()`.
- Birleşim, Veri girişinin yazması gibi formların bekçisinden geçer (`guard_forms`, `entry_write`). Saklı kopyaya göre
  yalnız formun alanları değişmiş olmalıdır.
  - Veri girişinin şekil değişikliği birleşime kendi tarafından girer ama bekçi onu yine reddeder (“nesnenin şekli …
    değişmez”). Birleşim, Veri girişine yazamadığını yazdırmaz.
  - Kapsam da birleşimle denetlenir.
- Cevabın `merged`'i birleşmiş nesneyi çağıranın okuduğu biçimde verir (`form_guard::projected`). Çatışmanın `current`'ı
  zaten öyleydi. Gizli alan cevapta da yoktur.
- Aynı istek yinelenince kaydedilmiş cevap gelir.
- Sürümsüz `update` (tabanı olmayan istemci) eskisi gibi çatışmadır.

### §2 Katmanlar arası ortak köşe

Tabanın kaydı alınırken (`sources::base::take`), çok köşeli nesnenin tuttuğu ortak köşeler yalnız kendi katmanında değil,
aynı kaynağa bağlı öbür katmanların nesnelerinde de sayılır:

- Aynı kaynak, aynı türde ve aynı yolda olmaktır (tek GeoPackage dosyası ya da tek veritabanı). `with` ile birlikte
  uzlaştırılabilenler bunlardır (ADR 0648 §3).
- Grup kuralı değişmez: tabanda paylaşılan ve iki yanın da taşıdığı köşe iki nesneyi birlikte çözdürür.
- Sınır aynıdır: ortak köşesi 128'den çok olan nesnenin köşeleri yine tutulmaz.

### §3 Taslağın kopyası: Kopyasını dosyaya yaz

Veritabanı projesinin Kayıt çakışması penceresinde yeni seçenek: **Kopyasını dosyaya yaz…**.

- Çizim, gönderilmemiş değişiklikleriyle, seçilen `.kcad` dosyasına kaydın yoluyla yazılır: kendi iş parçacığında,
  doğrulanarak (`saving::Target::Copy`).
- Farklı kaydet'ten farkı: çizim bulut projesi olarak kalır. Dosyası, kaynağı, kirliliği, cihaz taslağı ve çakışma sorusu
  değişmez.
- Kullanıcı kopyayı aldıktan sonra Sunucudakini al'ı seçebilir; kendi işi dosyada kalır.
- Önerilen ad “‹proje› (taslak).kcad”dır. Pencerenin açıklaması seçeneğin ne yaptığını söyler.

## Doğrulama

- **Birleştirme kuralı:** `scripts/fixtures/entity_merge_cases.py` 14 Veri girişi durumu daha yazar
  (`fixtures/cloud/v1/merge.json`, 39 durum). Durumların her biri gizli alanları `hidden` ile adlandırır:
  - görünen ve gizli alan;
  - şekil;
  - aynı alan farklı ve aynı;
  - silme;
  - kopyasında gizli alan gönderen istemci;
  - tabanında eski gizli değer;
  - bütün alanların gizli olması.

  `kentos-contracts`'ın `entity_merge::tests::the_shared_cases_merge_alike`'ı `merge_reading` ile oynatır.
- **Sunucu:** geçici PostGIS'te `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_forms`,
  `a_veri_girisi_edit_is_merged_over_the_fields_it_reads`:
  - düzenleyici şekli ve gizli notu değiştirir;
  - Veri girişinin sürümsüz `update`'i çatışmadır, `current`'ında not yoktur;
  - aynı değişiklik `edit` olarak birleşir: saklı kopyada Veri girişinin durumu, düzenleyicinin notu ve şekli;
  - cevabın `merged`'inde not yoktur, yinelenen istek aynı cevabı verir;
  - aynı alan farklı değiştirilince çatışma;
  - Veri girişinin şekil değişikliği birleşse de reddedilir.
- **Uzlaştırma:** `a_many_cornered_parcel_groups_with_a_building_of_another_layer`. 200 köşeli K2'nin üst kenarındaki köşe
  yalnız aynı dosyanın bina katmanındaki binayla paylaşılır. İki yan köşeyi taşıyınca `with` ile uzlaştırmada parsel ve bina
  tek gruptur (`sharedVertex`). Değişiklikten önce ayrıydılar.
- **Masaüstü:** `cloud::tests::the_conflict_writes_a_copy_of_the_draft_and_leaves_the_question`:
  - kopya dosyası yazılır ve taşınan noktayı taşır;
  - çizim bulut projesi kalır, dosyası olmaz;
  - durum Çakışma'dır, pencere açık kalır.
- **Resimler:** `cargo test -p kentos-desktop --bin kentos-cad cloud::cells_tests::screens -- --ignored --nocapture`
  (`.run/shots/bulut-hucre-cakisma-*`).

## Ölçümler

- Sunucuda birleştirme yalnız sürümü tutmayan `edit`'te çalışır. Veri girişinde öznitelik listesi iki kez süzülür: nesne
  başına öznitelik sayısıyla doğrusal. ADR 0648'in 0,15 ms'lik birleştirmesine ölçülür bir şey eklemez.
- Tabanın kaydında öbür katmanların köşeleri yalnız katmanda çok köşeli nesne varken, bir kez sayılır.
