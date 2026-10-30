# ADR 0757: Tablo bağlamak arayüzü kilitlemez

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözü (30 Ekim 2026): “veri tabanına bağlanıp katman eklediğim zaman diyelim ki 50.000 satırı olan bir
  tablom var Parseller tablosu bunu eklediğim zaman bu da 3 4 belki de 10 saniye kadar sistemi kilitliyor bunlar çok ciddi
  problemler mesela qgis içinde bu problemlerin hiçbiri yok ve harika çalışıyor”.
- **Web:** bağlama masaüstünündür (ADR 0503, 0505); web'e gerekenler TODOS.md §0.2'de. **`.kcad`:** değişmez. **Komut:**
  `cad.sources.edit`'in girdisi, çıktısı ve sonucu aynı; iş olarak koşarken adımını söyler ve durdurulabilir.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

PostGIS'e bağlan'ın Bağla'sı, GeoPackage'ın “Bağlı katman olarak aç”ı ve Kaynaklar'ın bağlaması `cad.sources.edit`'in
`link`'ini arayüzün iş parçacığında, çizimin kendisinde çalıştırıyordu: tablonun okunması, satırların nesneye çevrilmesi,
katmana yazılması, katmanın özeti ve uzlaştırmanın tabanı (ADR 0538) bitene dek pencere boyanmıyor, tıklama gitmiyordu.

Ölçüm (release, i5-11300H; geçici docker PostGIS 17, aynı makinede; `kentos-postgis`'in `a_whole_table_binds_in_time`'ı,
geçici zaman satırlarıyla):

| Tablo | Satır | Bağlama | Okuma | Satırlar → nesne | Kısıtlar | Kopya | Yazma | Özet | Taban | Kalan |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `kadastro.parseller` (8 köşe, 7 sütun) | 50 000 | 1,56 s | 188 ms | 245 ms | — | — | 95 ms | 116 ms | 762 ms | — |
| `kbs.parseller` (13 köşe, 28 sütun, 4 yabancı anahtar; sahibin tablosunun şeması) | 50 000 | 3,05 s | 405 ms | 661 ms | 23 ms | 157 ms | 95 ms | 222 ms | 1 324 ms | 143 ms |

Sahibin `kbs.parseller`'ı aynı 28 sütunlu şemadadır (yerel kopyasında satır yok; yalnız okunarak bakıldı). Ağ üzerindeki bir
sunucuda okuma payı büyür; “3 4 belki de 10 saniye” bu ölçümle tutarlıdır.

QGIS'in yolu (30 Ekim 2026'da okundu):

- Harita arka planda ayrı bir resme çizilir, görünen resim belli aralıklarla güncellenir (Kullanıcı kılavuzu, 5.1.6
  Rendering Settings: “Map update interval”, öntanımlı 250 ms; <https://docs.qgis.org/latest/en/docs/user_manual/introduction/qgis_configuration.html#rendering-settings>).
- PostgreSQL sağlayıcısı satırları imleçle parça parça okur (`FETCH FORWARD %1 FROM %2`), istekte kutu varsa sorguya
  `&& st_makeenvelope(…)` ekler (`src/providers/postgres/qgspostgresfeatureiterator.cpp`). Katman eklemek tabloyu okumaz.

KentOS'un bağlı katmanı ise tablonun çalışma kopyasıdır (ADR 0503, 0505): satırlar çizimin nesneleri olur; kenet, seçim,
düzenleme, uzlaştırma ve topoloji onlarla yapılır. Bu karar değişmez; değişen, kopyanın nerede kurulduğu ve ne kadar iş
olduğudur. Görünen alanla bağlamak (ADR 0582, “Yalnız görünen alan”) yine vardır.

## Karar

### 1. Bağlama iş olarak, çizimin okuma kopyasında

- Masaüstü `cad.sources.edit`'in bağlamasını iş olarak çalıştırır (`jobs::Job`, ADR 0253 §9): çizimin okuma kopyasında
  (nesneler paylaşılır, kopyalanmaz) kendi iş parçacığında; yazdığı (katman, nesneler, bağ, taban) bitince yama olarak
  çizime girer, tek geri alma adımıdır. Birden çok tablo sırayla, her biri kendi adımı (önceki gibi).
- Bağlamanın adımları (`sources::BIND_STEPS`): Tablo okunuyor (0), Satırlar nesneye çevriliyor (0,3), Nesneler katmana
  yazılıyor (0,55), Katmanın özeti çıkarılıyor (0,7), Kaynakla eşleşme kaydediliyor (0,8). Her adımın başında işin payı
  söylenir ve durma sorulur (`Steps::at`); durdurulan bağlama `cancelled` döner ve bir şey yazmaz. Bağlama kaynağa bir
  şey yazmaz (yalnız okur), durdurmak kaynağı değiştirmez.
- Aynı komut hemen çalışınca (betik, Python, MCP, plan) adım söylenmez, durdurulmaz; cevap aynıdır.

### 2. Masaüstünde pencere hemen kapanır, köşede panel

- Bağla'ya basınca pencere kapanır; sağ altta panel: “Bağlanıyor: kbs.parseller”, adım, yaklaşık satır sayısı (kataloğun
  tahmini), birden çok tabloda “tablo i / n”, çubuk ve Durdur. Çubuk her karede (`window::frames`) söylenene 0,12 s'lik
  üstel yumuşatmayla yaklaşır, hiç geri gitmez; bir adımın içinde beklenen süreyle ilerler (satır başına süre × adımın
  payı; beklenen sürede yolun %63'ü, en çok %95'i).
- Bir GeoPackage'ın her biri 2 000'den az satırlı tabloları eskisi gibi hemen bağlanır: iş parçacığı ve panelin ilk
  karesi bağlamadan uzun sürer. Veritabanının satır sayısı kataloğun tahminidir (`reltuples`; toplu yüklemeden sonra
  eski kalabilir: “1 satır” elli bin olabilir), veritabanı tabloları bu yüzden hep iş parçacığında bağlanır; panel
  bağlama 0,15 s'yi geçince görünür, küçük tablo panel yanıp sönmeden biter.
- Bağlama sürerken çizim düzenleme almaz: işin yazması çizim değişince reddedilirdi (katmanlara ve ayarlara dokunan yama).
  Görünüm kayar ve yakınlaşır; Esc ve Durdur bağlamayı bitirir, pencereyi kapatmak bağlamayı durdurur. İkinci bağlama ve
  içe aktarma o bitince yapılır (söylenir).
- Durdur hemen biter: işe durması söylenir, cevabı beklenmez; önceki tablolar bağlı kalır. Açık çizim değişirse (başka dosya)
  iş durdurulur.
- Bittiğinde eskisi gibi: görünüm gelen nesnelere, duyuru (veritabanında bağlantının adıyla, dosyada dosyanın adıyla),
  uyarılar gövdede.

### 3. Daha az iş

- **Taban** (`sources::base::take`): 2 048 ya da daha çok nesnede nesnelerin satırları ve kayıtları makinenin iş
  parçacıklarına (en çok 8) bölünerek hesaplanır; parçalar sırayla birleşir, baytlar tek iş parçacığınınkiyle aynı.
- **Satırlar nesneye** (`geopackage::table_of_rows`): 2 048 ya da daha çok satırda aynı bölmeyle; nesneler, atlanan
  satırlar ve ilk ret satır sırasıyla, kayıplar birleşir.
- **Katmana yazma:** nesneler bağın okuduğundan katmana taşınır, kopyalanmaz (`Stored::take_in_project`).
- **İşin özeti yok:** masaüstü işi komutun kendisiyle başlatır (`sources::edit_job`), protokolün zarfıyla değil. Zarflı iş
  (ADR 0253 §9) her eklenen nesnenin damgasını ve JSON değerini özete yazar: 50 000 parselde 1,6 s, sahibinkine benzeyen
  tabloda 3 s ek. Bağlamayı isteyen pencere özeti okumaz; komut aynıdır, kaydın satırı ve geri alma adımı aynıdır.

Ölçüm, aynı makinede ve aynı geçici PostGIS'te (`a_whole_table_binds_in_time`; adımlar işin söylediği anlardan):

| Tablo | Önce (arayüzde) | Hemen (komut) | İş olarak | Okuma | Satırlar → nesne | Yazma | Özet | Taban | Arayüzün payı |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `kadastro.parseller` | 1,56 s | 0,73 s | 0,74 s | 168 ms | 88 ms | 87 ms | 117 ms | 278 ms | 20 ms |
| `kbs.parseller` | 3,05 s | 1,37 s | 1,41 s | 385 ms | 268 ms | 100 ms | 222 ms | 434 ms | 22 ms |

“Arayüzün payı” işin yamasının çizime yazıldığı karedir (`Job::status`'un yazdığı bakış); gerisi iş parçacığındadır. Adımların
payları (`BIND_STEPS`) ve satır başına süre (28 µs) ikinci satırdan.

## Sonuçlar

- 50 000 satırlık bir tablo bağlanırken pencere boyanır, görünüm kayar ve yakınlaşır; arayüzün payı yamanın yazıldığı tek
  karedir (20 ms). Bağlama yarı sürede biter; Durdur onu hemen bitirir ve bir şey yazılmaz.
- Bağlamanın kendisi hâlâ tablonun bütün satırlarını okur ve çalışma kopyası kurar (ADR 0503): ağ üzerindeki yavaş bir
  sunucuda okuma payı büyür, ama arayüzü değil, paneli bekletir. Büyük tabloda “Yalnız görünen alan” (ADR 0582) okumayı
  alana indirir.
- Bağlama sürerken çizim düzenleme almaz (birkaç saniye); bu, işin yamasının reddedilmemesi içindir. Bulut projesinde
  başkasının değişikliği o arada gelirse yama reddedilir ve bağlama söylenen nedenle biter (yeniden bağlanır).
- Resimlerde iki kusur bulundu ve düzeldi. İşin yaması çizime nesne eklediğinde belgenin sıradaki yuvası ilerlemiyordu
  (`Document::apply_patch`, çizim iş sürerken değişmediğinde): görünüm gelen parsellere gitmiyordu (`zoom_to` yuvaları
  `ilk..sıradaki` aralığından alır), ondan sonra çizilen nesne işin eklediği ilk nesnenin yuvasını alır, onun yerine yazılırdı.
  Bu, iş olarak nesne ekleyen her komutun (ADR 0253 §9) kusuruydu; yama artık eklediği her nesnenin yuvasından sonrasına
  ilerletir. Katmanlar'ın Öğe sütunu 44 px sabitti, bağlı katmanın simgesi sayının yanına girince “50000” kesiliyordu:
  sütun, bir satırın en çok rozetini (süzgeç, dondurma, zaman, bağ; her biri 16 px) ve çizimin nesne sayısının en uzun
  yazısını sayar (`layer_count_width`).
- Kalan: GeoPackage bağlamasında tablo iki kez okunur (bağ için ve içe aktarma için); zarflı işin özeti (Python, MCP) büyük
  bağlamada yavaştır; nesnelerin alındığı kare 0,18 s'dir (50 000 nesnenin seçme ve kenet deposuna ve çizime girmesi,
  hemen bağlamada da aynı iş); üçü ayrı iştir (TODOS.md).

## Doğrulama

- Komutun cevabı aynı: `KENTOS_CASES_ONLY=cad.sources cargo test -p kentos-native-application --test all every_command_case`
  (bağın ortak durumları), `cargo test -p kentos-native-application --test all -- sources geopackage gpkg reconcile` (46 geçti),
  `--lib -- sources geopackage topology_check jobs` (adımların iki testi: durdurulan iş bağlamayı bitirir, adımın payı söylenir).
- Belge: `cargo test -p kentos-domain --test all patches` (işin eklediği nesnelerden sonra sıradaki yuva, sonra çizilen
  nesne yeni yuvada, geri alma ve yineleme), `cargo test -p kentos-domain` bütünüyle.
- Masaüstü: `cargo test -p kentos-desktop --lib -- exchange::binding exchange::postgis exchange::gpkg sources::` (48 geçti):
  iş parçacığında bağlanan tablo hemen bağlananla aynı nesneleri, bağı, tabanı, geri alma adımını, sıradaki yuvayı ve
  görünümü bırakır, geri alma ikisinde aynı; Durdur hemen biter ve bir şey yazılmaz, işin sonradan söylediği duyulmaz;
  bağlama sürerken düzenleme ve ikinci bağlama alınmaz, Esc durdurur; çubuk adımın içinde ilerler ve geri gitmez; veritabanı
  tablosu tahmini küçük olsa da iş parçacığında, paneli 0,15 s'den önce görünmez. PostGIS penceresinin testleri bağlamayı
  bekleyerek (`finish_binding`).
- Süreler: `KENTOS_PG_URL=… PGPASSWORD=… KENTOS_PG_BIND=kbs.parseller cargo test --release -p kentos-postgis --test all
  a_whole_table_binds_in_time -- --ignored --nocapture` (geçici docker PostGIS, `kadastro.parseller` ve sahibinkine benzeyen
  `kbs.parseller`; Karar'daki tablo).
- Resimler: `KENTOS_PG_URL=… PGPASSWORD=… KENTOS_PG_BIND=kbs.parseller KENTOS_PG_FIRST=kbs.adalar cargo test -p
  kentos-desktop --lib -- exchange::binding::tests::screens --ignored --nocapture --test-threads=1` (veritabanı yoksa
  GeoPackage'ın tablosuyla): `.run/shots/baglaniyor-{1-okunuyor,2-cevriliyor,3-iki-tablo,4-baglandi}-*`, iki boyut ve iki
  temada; okuma, satırların çevrilmesi, iki tablonun ikincisi ve bağlanmış 50 000 parsel.
- Kareler: `KENTOS_PG_URL=… PGPASSWORD=… KENTOS_PG_BIND=kbs.parseller cargo test --release -p kentos-desktop --lib
  perf::bind -- --ignored --nocapture --test-threads=1` (masaüstünün kendisi, 16 ms'lik karelerle): arayüzde hemen bağlama
  1,43 s süren tek karedir (güncelleme 1 376 ms); iş parçacığında bağlama 1,85 s'de 122 kare, en uzun kare 181 ms
  (güncelleme 142 ms, görünüm 38 ms, GPU 65 ms), 50 ms'yi aşan tek kare o: yamanın yazıldığı, 50 000 nesnenin seçme ve
  kenet deposuna ve çizime girdiği kare. Bağlamanın kendisi iş parçacığında 1,41 s; kalan, karelerin arasıdır.
