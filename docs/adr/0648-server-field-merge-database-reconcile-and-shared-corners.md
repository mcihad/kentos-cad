# ADR 0648: Sunucuda alan alan birleştirme, veritabanında birlikte uzlaştırma ve çok köşeli nesnenin bağı

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-37'nin üçüncü dilimi. Kapsamı ben belirledim.
- **Kalan işler:** [ADR 0553](0553-field-merge-groups-and-layers.md)'ün "Sonraya kalan"ı:
  - sunucuda `project.changes`'in istemcinin tabanıyla alan alan birleştirmesi;
  - PostGIS katmanlarının birlikte (`with`) uzlaştırılması;
  - çok köşeli (128'den fazla) ve çok parçalı nesnenin köşe bağı.
- **Bağlam belgesi:**
  - ADR 0026 (bulutta nesnenin sürümü), 0043 (masaüstünün bulut istemcisi);
  - ADR 0538 ve 0553 (kaynakla uzlaştırma, tabanın alan kaydı, gruplar);
  - ADR 0584 (aynı veritabanının katmanlarına birlikte yazma), 0582 (satır düzeyinde çakışma);
  - ADR 0411 ve 0550 (Veri girişi, erişim çözücüsü).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. Tabanın köşe kaydının biçimi aynıdır; yalnız çok köşeli nesnede ne tuttuğu değişir (§4).
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - adlı çalışma sürümü ve review → publish akışı: ihtiyacı gerçek kullanımda kanıtlanınca (plan böyle diyor);
  - Veri girişinin (`forms.use`) düzenlemesinin sunucuda birleştirilmesi (bugün çatışmadır, §1);
  - katmanlar arası çok köşeli nesnenin bağı (§4: taban katman başınadır);
  - web'in bulut istemcisinin `edit` göndermesi.

## Bağlam

ADR 0553 bulut projesinde iki editörün aynı nesnenin farklı alanlarına dokunmasını istemcide birleştirdi: sunucu 409 verir,
istemci sunucunun kopyasıyla birleştirip yeniden gönderir. Bu iki gidiş demektir ve birleştirmeyi yapan istemcinin taban
kopyası yalnız kendisindedir. Sunucu sürümü tuttuğu için her hızlı değişiklik zincirinde 409'lar birikir; ADR'de sunucu
tarafı tasarlandı ama yapılmadı.

Kaynakla uzlaştırma (ADR 0538, 0553) birden çok katmanı yalnız tek GeoPackage dosyasında birlikte uzlaştırıyordu. ADR 0584'ten
beri aynı PostGIS veritabanının katmanları tek işlemde yazılabiliyor; uzlaştırmanın `with`'i bunu reddediyordu.

Tabanın köşe kaydı nesne başına en çok 128 köşe tutar (kayıt boyu sınırı). Daha çok köşeli ya da çok parçalı nesnenin kaydı
yoktu; komşusuyla ortak köşesini iki yanın da taşıdığı durumda grup kurulmuyor, iki nesne ayrı ayrı çözülüp topolojisi bozuk bir
birleşim çıkabiliyordu.

## Karar

### §1 Sunucuda alan alan birleştirme: `edit`

`project.changes`'in değişikliklerine yeni tür: `{"op": "edit", "id", "entity", "base"}`. Bir güncellemedir;
`base` istemcinin o nesnenin son bildiği kopyasıdır (`expectedVersions[id]`'nin kopyası).

- Sürüm tutuyorsa `edit` bir `update` gibi yazılır.
- Sürüm tutmuyorsa sunucu, proje kilidi altında, saklı kopyayı (`theirs`), gelen nesneyi (`mine`) ve `base`'i
  `kentos_contracts::merge_entities` ile birleştirir (ADR 0553 §4'ün kuralı: üst düzey alanlar ve öznitelikler tek tek; yalnız
  bir yanın değiştirdiği alan o yandan, iki yanın farklı değiştirdiği alan çatışma).
  - Temiz birleşim yazılır, nesnenin yeni sürümü olur. Cevabın `merged`'i birleşmiş nesneleri sürümleriyle verir.
  - Birleşim yazılamazsa (alanlar çatıştı, katman kilitli, nesne kurulamadı) değişiklik bugünkü gibi `changed` çatışmasıdır,
    sunucunun kopyasıyla.
  - Nesne silinmişse `deleted` çatışmasıdır.
- Kilitli katman ve form denetimi birleşmiş nesneyle yapılır (`guard_forms` birleşimi görür).
- `base` yalan olsa da yazanın hakkını aşmaz: yetki ve formlar aynı denetlenir, `base` yalnız hangi alanların değiştirildiğini
  belirler.
- Veri girişi (`forms.use`, `feature.write` yok) için birleştirme yapılmaz; sürüm tutmayan `edit` çatışmadır. Onun yazdığı,
  formun gizlediği alanları saklı kopyadan alan ayrı bir kuralla birleşir (ADR 0411 §6); iki kuralı karıştırmamak için.
- Aynı istek kimliğiyle gelen ikinci istek, kaydedilmiş cevabı `merged`'iyle verir.
- `update` aynen kalır: tabanı olmayan istemci (eski sürüm, Python) eskisi gibi çatışma alır.

### §2 Masaüstünün bulut istemcisi

- `kentos_cloud` bildiği kopyası olan her güncellemeyi `edit` olarak, kopyasıyla gönderir (`Planned::Update.base`).
  Taslağa yazılmış komut okununca `edit` geri `base`'li güncelleme olur.
- Cevapta `merged` varsa (`ProjectSync::merged_in`):
  - gönderilenden sonra düzenlenmemiş nesne sunucunun birleşimini alır;
  - düzenlenmiş nesnede o düzenleme birleşimin üstüne alan alan konur (taban gönderilen kopyadır); çatışırsa düzenleme olduğu
    gibi kalır ve sıradaki komutla gider;
  - bilinen sürüm birleşimin sürümüdür.
- Masaüstü kullanıcıya “n nesneyi başka biri de değiştirmişti, ama farklı alanlarını: sunucu ikisini birleştirdi.” der.
- ADR 0553'ün istemci birleştirmesi (gelen değişiklik ve 409) yerinde kalır: `edit`'i bilmeyen sunucuyla ve çatışmada.

### §3 Veritabanında birlikte uzlaştırma

`cad.sources.reconcile`'ın `with`'i aynı kaynağın katmanlarını alır: tek GeoPackage dosyası ya da tek PostGIS veritabanı (aynı
adres). Yazma ADR 0584'ün tek işlemiyle olur. Her katmanın bağı kendi tablosunun sürümüne geçer (veritabanında tablolar ayrı
sürümlüdür). Başka kaynakların katmanı `transaction_split` olarak reddedilir; ileti “tek GeoPackage dosyasında ya da tek
veritabanında olmalı” der.

### §4 Çok köşeli ve çok parçalı nesnenin köşe bağı

Tabanın kaydı alınırken, köşesi 128'den çok olan nesne bütün köşelerini değil, katmandaki başka bir nesneyle paylaştığı köşeleri
tutar (en çok 128). Grup kuralı yalnız tabanda paylaşılan köşeye bakar, bu yüzden aynı sonucu verir. Uzlaştırmada iki yanın
köşeleri sınırsız sayılır (`vertex_set_all`). Parçalar ve delikler de köşedir.

Taban katman başına alındığından, başka katmandaki nesneyle paylaşılan köşe çok köşeli nesnede tutulmaz: o bağ yalnız 128 ve
daha az köşeli nesnelerde kurulur.

## Doğrulama

- Birleştirme kuralının 25 ortak durumu `scripts/fixtures/entity_merge_cases.py` ile ADR'lerden, KentOS kodu olmadan yazılır
  (`fixtures/cloud/v1/merge.json`). Durumlar: nokta, alan ve çizgi; taşıma, öznitelik, etiket, renk, katman, kot, delik;
  çatışma sırası. `kentos-contracts`'ın `entity_merge::tests::the_shared_cases_merge_alike`'ı oynatır.
- Sunucu, geçici PostGIS'te (`KENTOS_TEST_DB=required cargo test -p kentos-application --test changes`):
  `an_edit_from_a_known_copy_is_merged_with_a_newer_version`.
  - Düz `update` çatışır; `edit` birleşir, sürüm 3, saklı kopya iki değişikliği taşır, `merged` saklı kopyadır.
  - Aynı istek yinelenince aynı birleşim gelir.
  - Aynı öznitelik farklı değiştirilince çatışma, sunucunun kopyasıyla gelir; silinmiş nesne `deleted`'dır.
- İstemci (`cargo test -p kentos-cloud`):
  - güncelleme `edit` olarak tabanıyla gider;
  - sunucunun birleşimi çizime girer, gönderilecek bir şey kalmaz;
  - arada yapılan düzenleme birleşimin üstüne konur ve sıradaki komutla gider.
- Uzlaştırma:
  - `scripts/fixtures/reconcile_cases.py` (55 durum, yeni iletiyle);
  - `a_many_cornered_parcel_still_groups_with_its_neighbour`: 200 köşeli parsel komşusuyla gruplanır;
  - geçici PostGIS'te (`/usr/bin/python3 scripts/fixtures/postgis_cases.py --only layers_of_one_database`)
    `layers_of_one_database_are_reconciled_together`: iki tablo, katmanda taşınan alan, tabloda yeniden adlandırılan iki satır;
    çatışma yok, iki tablo ikisini de alır, ikinci uzlaştırma bir şey bulmaz.

## Ölçümler

Release, bu makinede (`cargo test --release -p kentos-contracts --lib entity_merge::tests::merge_timing -- --ignored
--nocapture`): 200 köşeli bir parselin 10 000 birleştirmesi (bir yanda köşe, öbür yanda öznitelik) 1,47 s, birleştirme başına
0,15 ms.

Sunucuda birleştirme yalnız sürümü tutmayan `edit`'lerde çalışır; tutan komutun yolu değişmedi. `edit` gövdeye tabanın
kopyasını ekler: güncellenen nesne başına bir kopya daha gider. Tabanın kaydında çok köşeli nesne için katmanın bütün
köşeleri bir kez sayılır, yalnız böyle bir nesne varsa.
