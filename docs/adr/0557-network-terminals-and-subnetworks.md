# ADR 0557: Şebekede terminaller ve alt ağlar

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-27'nin ikinci dilimi (“şebekeyi terminal/varlık ilişkisiyle
  genişlet”). Kapsamı ben belirledim: ArcGIS Utility Network'ün terminal yapılandırması ve alt ağ denetleyicisi, KentOS'un ağ
  modeline sığdığı kadarıyla.
- **Bağlam belgesi:** ADR 0209 (ağ analizi, Denetle), 0521 (dönüşler ve seviyeler), 0525 (ilişkili kayıtlar ve doğrulanmamış
  bağlantılar), 0199 (katman alanları), 0265 (`.kcad` şemasının altı adımı), 0508 ve 0529 (komutla çalışan işlemler).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan** (ilk üçü [ADR 0562](0562-network-terminals-everywhere-and-subnetworks-kept-current.md)'de yapıldı):
  - ~~En kısa yol, hizmet alanı ve maliyet matrisi terminal yollarını bilmez~~: bilir (ADR 0562 §1; en yakın tesis de).
  - ~~Alt ağ adının kendiliğinden yeniden yazılması~~: `keepCurrent` (ADR 0562 §3); alt ağ kapsamı değişince bildirim hâlâ yok.
  - ~~Cihazın terminallerinin cihazın özniteliğinden okunması~~: `by` ve `configs` (ADR 0562 §2); varlık türü tablosu hâlâ yok.
  - Artımlı graf kurulumu (ADR 0525'ten kalan; ADR 0562 §6'da neden bu dilimde yapılmadığı).

## Bağlam

ADR 0209'un ağında bir düğüm nesnesi (vana, trafo, pompa) bağlı olduğu bütün kenarları birbirine bağlar. Gerçek cihazlarda
bu yanlıştır: bir trafonun yüksek gerilim ucuna gelen hat alçak gerilim hattıyla bağlıdır ama aynı noktada biten, orta gerilim
ucuna bağlı hatla değil; bir üç yollu vananın iki çıkışı birbirine açılmaz. Ayrıca bir şebeke tek parça değildir: kaynak
başına besleme alanları (basınç bölgesi, trafo merkezinin beslediği hat) vardır ve “bu boru hangi merkezden beslenir?”
sorusunun cevabı nesnede durmalıdır.

## Karar

### 1. Terminaller

Ağın tanımında isteğe bağlı `terminals` (`NetworkTerminals`): kenarın başlangıcının ve bitişinin bağlı olduğu terminali
söyleyen iki kenar alanı (`start`, `end`) ve 1–16 cihaz (`TerminalDevice`). Cihaz bir düğüm katmanıdır (o katmanın her
noktası cihazdır): 2–8 terminali (`id`: küçük harf, rakam ve tire; `name`) ve 1–16 yolu (iki ya da daha çok terminalin
kimliği: cihazın içinden birbirine bağlı olanlar). Kurallar `kentos_contracts` içindedir (`network_problem`) ve her yerde
aynıdır: cihaz katmanı ağın düğüm katmanıdır, bir katman bir cihaz, iki alan farklı, yolda tanımsız terminal yok, bir
yol en az iki terminal.

İzleme (graf) durumu (düğüm, terminal) çiftidir. Cihazın düğümüne bir kenar bir terminalle gelirse ötesine yalnız o terminalle
aynı yolda olan terminallere bağlı kenarlar geçilir; terminalin hiçbir yolu yoksa cihaz o uçtan geçilmez. Terminali olmayan ya da
cihazın bilmediği bir terminali söyleyen ucu olan kenar, ArcGIS'in tersine hatayla reddedilmez: bütün terminallere geçer
(eski davranış) ve izleme doğrulanmamıştır (ADR 0525'in mekanizması; sorun türü `terminal`). Denetle böyle uçları, terminali
olmayan bir yolu ve cihazı olmayan ucu söyler. Çizgi ağında (cihaz tanımı yoksa) hiçbir şey değişmez.

### 2. Alt ağlar

Ağın tanımında isteğe bağlı `subnetworks` (`NetworkSubnetworks`): ağın Kaynak düğümleri alt ağ denetleyicisidir; alt ağ bir
denetleyicinin ağın kurallarıyla ulaştığı yerdir (`reach`: Akış aşağı ya da Bağlı; kapalı kenarlar ve kapalı düğümler,
cihazların terminal yolları, dönüşler, seviyeler aynen). Denetleyicinin adı `name` alanındaki değerdir (yoksa etiketi, yoksa `Ad`,
yoksa `Kaynak <no>`); aynı ada (Türkçe büyük-küçük harfsiz) sahip denetleyiciler tek alt ağdır (iki besleme noktalı bölge).
Gezinti başka alt ağın denetleyicisinin düğümünde durur. Sonuç:

- her alt ağın denetleyicileri, kenar ve düğüm sayısı, uzunluğu ve nesneleri;
- **yetim** nesneler (hiçbir denetleyicinin ulaşmadığı), **paylaşılan** nesneler (birden çok alt ağın ulaştığı; adları `; ` ile
  birleştirilir);
- **eski** nesneler (`stale`): `attribute`'taki değer hesaplananla farklı (eksik, değişmiş, artık).

### 3. Komutlar

- `cad.subnetworks.find` (salt okunur): ağ, isteğe bağlı sınırlar; yukarıdaki sonuç, nesne kimlikleriyle (kalıcı kimlikler).
  Ağın alt ağı yoksa `no_subnetworks`, kaynak yoksa `no_controllers`.
- `cad.subnetworks.update`: `attribute`'u ağın kenar ve düğüm katmanlarının nesnelerine yazar (değerler `cad.fields.write`'ın
  kurallarıyla: alanın türü, uzunluğu, değer listesi; tek adım “Alt ağları güncelle”), hiçbir alt ağda olmayanın değerini
  kaldırır; kilitli ya da alanı olmayan katman atlanır ve söylenir (`subnetwork_layer_skipped`). Cevapta güncel durum ve `written`.
- İkisi de `call::recorded`'la protokolün kayıtlı adımıdır; masaüstü, başsız sunucu, Python ve MCP aynı komutu çalıştırır.
  Graf İşlemler'in ev sahibi üzerinden kurulur (`ToolHost`; uygulama katmanı İşlemler'e bağımlı değildir).
- Ağ tanımı `cad.network.define`'ın yeni alanlarıyla yazılır (`terminals`, `subnetworks`).

### 4. Biçim

`.kcad` şema 93 (`SCHEMA_WITH_NETWORK_TERMINALS`; sütunlar değişmediği için `FORMATS_VERSION` aynı): ağın `terminals` ve `subnetworks`'ı. Altı
adım (ADR 0265): bağımsız Python okuyucu ve yazıcı (`tools/kcad/kcad.py`, `kcad_v2_reference.py`), örnek dosya
`fixtures/kcad/v2/network-terminals.{kcad,json}`, bozuk örnekler (`broken/network-terminals-*`, `network-subnetworks-*`: on altı
dosya, eski şemada bulunması dahil), `expected.json`, şartname, taşıma testi.

### 5. Arayüz

- **Ağlar penceresi:** Terminaller bölümü (cihaz katmanı, terminaller `kimlik: Ad; …` ve yollar `a, b; a, c` olarak yazılır,
  Kenar alanları) ve Alt ağlar bölümü (Ad alanı, Öznitelik, Ulaşım); form kuralları `kentos_interaction::network::form`
  (`DeviceRow`, `subnet_*`), hata ağın kendi kuralının iletisiyle.
- **Alt ağlar penceresi** (Analiz › Ağ analizi › Alt ağlar…, `network.subnetworks`; arayüz kaydı `catalog/v1/additions.json`):
  tablo alt ağları, yetimleri, paylaşılanları ve eski nesneleri sayar; satır nesnelerini seçer; Güncelle yazar.
- Şebeke izleme araç olarak aynı kalır; cihazlardan terminal yollarıyla geçer.
- Python: `kentos.cad.subnetworks.find` ve `.update`, `Network.check`'in `terminal` sorunları.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/network_terminals_cases.py` (KentOS kodu olmadan): `fixtures/network/v1/terminals.json`, 5
  sahne, 24 izleme, 15 alt ağ durumu; Rust tarafı `cargo test -p kentos-geometry-core --test all network_terminals`.
- `scripts/fixtures/subnetworks_command_cases.py`: `cad.subnetworks.find` (26) ve `.update` (23) ortak durumları;
  `network_rules_cases.py` ve `network_form_cases.py`'nin terminal ve alt ağ kuralları.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`; `python/tests/test_network_terminals.py`
  aynı sahneleri Python'dan.
- Masaüstü: `cargo test -p kentos-desktop networks::tests`; resimler
  `KENTOS_SHOTS_ONLY=terminaller,terminal-iz,alt-aglar,alt-aglar-guncel,alt-aglar-serit cargo test -p kentos-desktop
  networks::tests::screens -- --ignored --nocapture` (`.run/shots/aglar-terminaller-*`, `aglar-terminal-iz-*`,
  `aglar-alt-aglar-*`).
- Süre: terminalli izleme bütçesi 20 ms (ADR 0209 §12), terminalsiz ağ hızı değişmez.
