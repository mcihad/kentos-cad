# ADR 0562: Terminaller her yerde ve alt ağ adları güncel

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-27'nin üçüncü dilimi; [ADR 0557](0557-network-terminals-and-subnetworks.md)'nin
  “sonraya kalan” listesinden üç madde.
- **Bağlam belgesi:** ADR 0209 (ağ analizi, aramalar), 0521 (dönüşler ve seviyeler; arama durumları), 0525 (doğrulanmamış
  bağlantılar), 0557 (terminaller, alt ağlar), 0199 (katman alanları), 0265 (`.kcad` şemasının altı adımı), 0337 (belgenin
  izleyicileri: yazmanın ardından aynı adımda).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - Artımlı graf kurulumu (yalnız dokunulan parçaların yeniden kurulması): §6; bu dilimde yapılmadı, nedeni orada.
  - `cad.network.define` ağın tanımını değiştirince (alt ağ ulaşımı, yapılandırma, katman listesi) alt ağ özniteliğinin
    kendiliğinden yeniden yazılması; izleyici nesnelere dokunan yazmayı izler, ayarı değil.
  - Yapılandırmanın cihaz katmanının yanında nesnenin türünden (bir varlık türü tablosundan) okunması.
  - Güncel tutmanın uzun sunucu işlerinde (ADR 0559) kopyada çalışması: iş kopyada `recorded`'dan geçer ve izleyici orada da
    çalışır, ama işin sonunda kaynak belgeye yazılan değişiklik kümesi izleyicinin yazdıklarını ayrıca sınamadı.

## Bağlam

ADR 0557 cihazın terminallerini yalnız izlemede (Şebeke izleme: Bağlı, Akış yukarı, Akış aşağı, Yalıtım) ve Denetle'de uyguladı.
Aynı ağda En kısa yol, Hizmet alanı, En yakın tesis ve Maliyet matrisi cihazı bir nokta sayıp bütün kenarlara geçiyordu: üç yollu
bir vananın kapalı koluna giden “en kısa yol” vanadan geçiyor, trafonun yüksek gerilim hattından alçak gerilim hattına geçmeyen
bir hizmet alanı karşı yana taşıyordu. İki sonuç aynı ağda iki ayrı doğru veriyordu.

İki başka eksik: cihazın terminalleri cihaz katmanının her noktası için aynıydı (bir “Ayırıcı” katmanında hem iki hem üç
terminalli cihaz olamazdı), ve alt ağ adları yalnız Alt ağları güncelle'yle yazılıyordu: boru eklenince ya da vana kapanınca
öznitelik, biri yeniden çalıştırana dek eski kalıyordu.

## Karar

### 1. Aramalar cihazdan yalnız terminal yollarıyla geçer

Çekirdekte (`ops::network`) aramaların hepsi tek arayıcıdan (`Searcher`) geçer: En kısa yol, Hizmet alanı, En yakın tesis, Maliyet
matrisi, ters sorular (Tesise, Akış yukarı) ve ağacı kurup imlece yol bulan araçlar. Kural bu arayıcıda uygulanır, her araçta ayrı
yazılmaz:

- Cihazlı ağın (`Graph.terms` var) aramaları dönüşlü ağınki gibi **ok durumlarıyla** yürür (`by_arcs`: dönüş ya da terminal
  tanımı varsa): durum bir düğüm değil, düğüme hangi okla (kenar parçası, yön) gelindiğidir; ok kendi ucundaki terminali
  bilir (`Terminals.arc_terminal`: ok, kuyruğunda cihaz varsa kenar ucunun söylediği terminal, yoksa ya da söylemiyorsa
  `NO_TERMINAL`).
- `Graph::turn(düğüm, gelen_ok, giden_ok)` önce cihazın geçişini sorar (`crosses`): gelen okun terminali ile giden okun terminali
  aynı ya da aynı yolda değilse geçiş yoktur; terminali olmayan uç (doğrulanmamış, ADR 0557 §1) her yöne geçer; cihaz olmayan
  düğüm (nokta) her yöne geçer. Geçiş varsa dönüşün gecikmesi, yasağı ve geri dönüş kuralı (ADR 0521) **olduğu gibi** uygulanır:
  terminal yolu dönüşü açmaz, dönüş yasağı terminal yolunu kapatır.
- Seviyeler, kapalı kenarlar, kapalı düğümler, tek yön, zaman profili aynen; arama başlangıcı bir düğümse ya da kenarın
  ortasındaysa terminal yoktur (cihazın kendisinden başlayan her yöne gider).
- Cihazsız ağın arama yolu değişmez: düğüm durumları (hız), dönüşlüde ok durumları.

Doğrulama: `scripts/fixtures/network_terminals_cases.py` ADR'den, KentOS kodu olmadan, (düğüm, terminal) çiftleri üstünde düz bir
Dijkstra yazar; ters sorular kenarları ters çevrilmiş sahnenin ileri sorusudur. Yedi sahne: trafo, üç terminalli ayırıcı, iki
kaynaklı alt ağ sahnesi, terminalsiz uç, kısa yolun cihazdan geçemediği halka ve cihazın noktalarındaki alana göre
yapılandırmaları. Sorular: **38 rota** (duraklar arasında maliyet ve kenarların sırası; ulaşılamayan durak), **7 matris**
(her iki yönde, en yakın k ve en çok maliyetle), **20 hizmet alanı** (kenar başına ulaşılan uzunluk, kapalı düğümden öte
geçmez; Tesise dahil). Rust tarafı `cargo test -p kentos-geometry-core --test all network_terminals`.

Maliyet: cihazlı ağın aramaları ok durumlarıyla yürüdüğü için dönüşlü ağın çalışma biçimindedir. Ölçüm §Doğrulama'da.

### 2. Cihazın terminalleri noktasının özniteliğinden

`TerminalDevice` iki isteğe bağlı alan kazanır: `by` (cihaz katmanının noktalarında yapılandırmayı söyleyen öznitelik) ve
`configs` (1–16 `TerminalConfig`: `value`, `terminals`, `paths`; terminal ve yol kuralları cihazınkinin aynısı). Cihazın kendi
`terminals` ve `paths`'ı **varsayılan** yapılandırmadır:

- Noktanın `by` özniteliğinin değeri (kırpılmış, Türkçe büyük küçük harfsiz) bir yapılandırmanın `value`'suna eşitse nokta o
  yapılandırmanın terminallerini ve yollarını taşır; kenar ucunun söylediği terminal o yapılandırmaya göre çözülür.
- Öznitelik yoksa ya da boşsa nokta cihazın kendi terminallerini taşır. Öznitelik bir değer taşıyor ama hiçbir yapılandırma
  onu bilmiyorsa nokta yine cihazın kendi terminallerini taşır **ve bu söylenir**: ağın kurulumunun sorunlarında (“… cihaz
  yapılandırması “x” için tanımlı değil; bu noktalar cihazın kendi terminallerini aldı”), kenar uçlarının terminal sorunu ise
  zaten kendi başına Denetle'de.
- `by` ağın okuduğu bir alandır: alt ağ özniteliği onun üstüne yazamaz (`subnetworks_problem`). `by` olmadan `configs`,
  `configs` olmadan `by` olmaz; değerler cihazda bir kez (Türkçe büyük küçük harfsiz).

Çekirdekte kurallar düzleştirilir: `Rules.terminals.devices` önce her tanımdaki cihazın kendi kuralı (ADR 0557'deki sıra), sonra
bütün yapılandırmaların kuralları; cihazın kuralı kendi yapılandırmalarının değerlerini ve sıra numaralarını taşır
(`Rules::device_for(katman, değer) -> (sıra, bilinmeyen)`). Graf yalnız “düğümün cihazı”nı bilir; hangi yapılandırma olduğunu
ev sahibi (`kentos-processing::network::network_input`) noktanın özniteliğinden seçer. Düğümün hesabı ve aramalar değişmez:
bir cihaz kuralı, terminalleri ve yollarıyla, kendi yapılandırmasının kuralıdır.

Kural sözleşmededir (`terminal_set_problem`, `configs_problem`) ve her yerde aynıdır; mesajlar yapılandırmanın değerini söyler
(“1. cihazın “üç yollu” yapılandırmasının terminalleri 2–8 olmalı”).

### 3. Alt ağ adlarını güncel tutmak

`NetworkSubnetworks`'a `keepCurrent` (yalnız `true` yazılır; yok: eskisi gibi, adı Alt ağları güncelle yazar). Açıksa ağın
katmanlarının (kenar, düğüm ve dönüş katmanları: `NetworkDef::layers`) bir nesnesine dokunan her yazma, **aynı adımda**,
ağın alt ağlarını bulur ve `cad.subnetworks.update`'in yazdığı gibi yazar:

- İzleyici `call::recorded`'ın içinde, komutun yazmasından ve kısıtların izlemesinden sonra, aynı `run_step`'in içinde çalışır
  (`subnetwork_follow::follow`; belgenin öteki izleyicileri gibi). Komutun kaydı, geri alma adımı, `expectedRevision` ve planı
  tek adımdır; geri alma hem yazmayı hem adları geri alır.
- “Dokunulan katman” belgenin adım kaydından gelir (`Document::step_layers`: eklenen, silinen, değişen nesnenin, değişirse
  önceki ve sonraki katmanı). Ayar (ağın tanımı, katman özellikleri) nesne sayılmaz.
- Ağ yeniden kurulur ve bulunan, güncelleme komutunun planıyla (`update_in`) karşılaştırılır; yalnız farklı nesneler yazılır.
  `cad.subnetworks.update`'in kendisi izlenmez (zaten odur).
- **Bir yazma hiçbir zaman bunun yüzünden reddedilmez.** Öznitelik güncellenemezse (ağ kurulamıyor, çizimde Kaynak yok,
  kilitli ya da özniteliğin alanı olmayan katman, alanın kuralına uymayan değer) yazma yapılır ve cevaba
  `subnetwork_dirty` uyarısı girer (“… ağı alt ağ adlarını güncel tutuyor ama bu yazma onları güncelleyemedi: <neden>. Alt
  ağları güncelle ile yenileyin.”); güncelleme komutunun kendi uyarıları (`subnetwork_layer_skipped`, `values_skipped`) da
  söylenir. Kod her yazan komutun olası uyarısıdır (`PROTOCOL_CODES`).
- Maliyet: izleyici ağın tamamını yeniden kurar (100 800 parçada çizimden kurma 120 ms; bir milyon parçada her yazma saniyeler
  alır, ölçülmedi). Seçenek bu yüzden kapalı başlar ve ağ başına açılır; ölçüm §Doğrulama'da, artımlı kurulum §6'da.

### 4. Biçim

`.kcad` şema 94 (`SCHEMA_WITH_NETWORK_CONFIGS`; sütunlar değişmediği için `FORMATS_VERSION` aynı): cihazın `by` ve `configs`'i ve alt
ağların `keepCurrent`'ı (yalnız `true`). Altı adım (ADR 0265): bağımsız Python okuyucu ve yazıcı (`tools/kcad/kcad.py`,
`kcad_v2_reference.py`), örnek dosya `fixtures/kcad/v2/network-configs.{kcad,json}`, bozuk örnekler (`broken/network-configs-*`,
`broken/network-keep-current-*`: on iki dosya, şema 93'te bulunması dahil), `expected.json`, şartname §6.4.8 ve şema listesi,
taşıma testi (`kentos-kcad`'in `network_terminals` testi, `carriage`). `broken/schema-version-94.kcad` 95 oldu.

### 5. Arayüz

- **Ağlar penceresi:** her cihaz satırının altında ikinci satır: **seçen alan** (`by`) ve **yapılandırmalar**
  `üç yollu = a: A; b: B; c: C | a, b, c || trafo = yg; ag | yg, ag` olarak yazılır (`kentos_interaction::network::form`'un
  `configs_of` ve `configs_text`'i; yapılandırmalar `||`'la ayrılır, değer `=`'den önce, terminaller ve yollar `|`'la ayrılır);
  Alt ağlar bölümünde **Güncelleme** (Elle ya da Güncel tut). Hatayı ağın kendi kuralının iletisi söyler.
- Python: `NetworkDef`, `TerminalDevice` ve `NetworkSubnetworks` tipleri katalogdan üretilir; `cad.network.define` yeni alanları
  yazar; güncel tutan ağın yazması cevabında `subnetwork_dirty` uyarısı gelir.
- MCP: komutlar katalogdan; yeni komut yok.

### 6. Artımlı graf neden bu dilimde yok

Artımlı kurulum (yalnız dokunulan parçaların yeniden kurulması) ADR 0525'ten beri açık. Bu dilimde yapılmadı, nedenleri:

- Grafın parçaları kenarların kesişimlerinden doğar (bir kenarın eklenmesi komşu kenarı böler, silinmesi iki parçayı birleştirir);
  bir tek nesnenin dokunduğu bölgeyi güvenle sınırlamak parça kimliklerinin, düğüm numaralarının, ok numaralarının, terminal ve dönüş
  tablolarının, bellekteki R-ağacının ve `Terminals.slot`'un birlikte yeniden numaralanmasını ister; ADR 0212'deki etiket ve ADR
  0524'teki topoloji denetimi gibi “tam kurmayla aynı” ispatı (rastgele düzenlemelerde karşılaştırma) gerekir.
- Kurulum süresi ölçülmüş bütçede (150 ms'de 100 800 parça) ve her yazmanın bunu ödediği tek yer `keepCurrent` (§3), açılması
  bilinçli bir seçim olan bir seçenek.
- Doğru adım önce tam kurmayla aynı olduğu kanıtlı bir `Graph::rebuild_part` ve ondan sonra izleyicinin ona geçmesidir; ayrı bir ADR.

## Doğrulama

- `scripts/fixtures/network_terminals_cases.py` (bağımsız, KentOS kodu olmadan; `fixtures/network/v1/terminals.json`): 7 sahne,
  37 izleme, 19 alt ağ sorusu, **38 rota, 7 matris, 20 hizmet alanı**; Rust tarafı
  `cargo test -p kentos-geometry-core --test all network_terminals`. `Rules::device_for` ve düzleştirme birim testle.
- `network_rules_cases.py` (156 durum, 26'sı yeni: yapılandırma ve güncel tutma) ve `network_form_cases.py` (31 durum, 4'ü yeni);
  `kentos-contracts`'in `network_rules` ve `network::tests` testleri.
- Komut durumları, bağımsız başvuruyla (KentOS kodu olmadan): `cad.network.define` (35; yeni 14), `cad.subnetworks.find` (37; yeni 11),
  `cad.subnetworks.update` (27; yeni 4), `cad.subnetworks.follow.json` (28: başka komutların yazması: öznitelik, çizgi, silme, taşıma,
  katman; vana kapanır, boru eklenir, kaynak silinir; kilitli, alansız, kısa alan ve kaynaksız ağda `subnetwork_dirty`; güncel
  tutmayan ağ; ağın katmanı olmayan katman; geri alma ve yineleme tek adım; güncellemenin kendisi izlenmez). Masaüstü
  `cargo test -p kentos-native-application --test all fixtures`, Python `python/tests/test_command_cases.py`.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Masaüstü: `cargo test -p kentos-desktop networks::terminal_tests` (iki yeni test: yapılandırmalar ve Güncelleme formdan yazılır ve
  gösterilir; güncel tutan ağda vanaya çizilen boru kendi adımında adlarını alır, tek geri almayla boru ve adlar gider);
  resimler `KENTOS_SHOTS_ONLY=yapilandirma,guncel-tut,terminaller cargo test -p kentos-desktop networks::tests::screens --
  --ignored --nocapture` (`.run/shots/aglar-yapilandirma-*`, `aglar-guncel-tut-*`; koyu ve açık, 1440×900 ve 1100×650).
- Süre (100 800 parçalı şehir; `cargo test --release -p kentos-geometry-core --test all network_terminals::timing -- --ignored --nocapture`, `network::timing` ve `kentos-processing`'in `network_timing`'i):

Ölçümler 21 Ekim 2026, bu yapıda, release, M5 Mac (aynı anda başka derlemeler de koşuyordu; her süre beş koşunun en küçüğü):

| Soru (100 800 parça) | cihazsız | her onuncu kavşakta cihaz (düz geçişli vana) | her kavşakta cihaz (4 terminal bir yolda) | bütçe |
|---|---|---|---|---|
| kurma | 54,7 ms | 50,2 ms | 64,5 ms | 150 (cihazlıda 300) |
| bütün ağda Dijkstra | 3,2 ms | 14,3 ms | 15,0 ms | 30 |
| köşeden köşeye rota | 3,1 ms | 14,0 ms | 14,4 ms | 30 |
| 10 dakikalık hizmet alanının çizgileri | 12,4 ms | 25,6 ms | 26,8 ms | 30 |

Hepsi bütçe içinde; ama cihazlı ağın aramaları cihazsızınkinin yaklaşık 4–5 katı (rota ve Dijkstra) ve 2 katı (hizmet alanı) sürer:
ok durumları düğüm durumlarının iki katı bellek yazıp okur ve her adımda dönüş tablosuna ve terminal tablosuna bakar. Cihazsız ağ
aynı koddan düğüm durumlarıyla geçer, hızı değişmemiştir (`network::timing`: kurma 55,8 ms, Dijkstra 4,1 ms, rota 3,3 ms, hizmet
alanı 12,6 ms). `network::timing`'de iki satır bütçeyi **aşıyor** ve bu dilimin değişmediği yollardadır: “izleme, bağlı” 72,7 ms
(bütçe 20; terminalsiz ağın izlemesi) ve “aynısının alanları, pay 25 m” 354,8 ms (bütçe 300; alan birleşimi). Bu iki satırın
ADR 0557'nin izleme değişikliğinden mi, makinenin o anki yükünden mi geldiğini ölçmedim; ikisi de iki koşuda aynı çıktı, bu
dilimden önceki bir yapıyla karşılaştırılmadı. Çizimden bütün kurma (`kentos-processing`'in `network_timing`'i: depo, okuma,
graf; 100 800 parça) 119,8 ms: `keepCurrent` açıkken her yazmanın ağ başına ödediği tutar bu ve alt ağların aranması (ağın
denetleyicilerinden iki arama) ve `update`'in planı üstüne gelir.
