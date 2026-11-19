# Claude test kaydı

KentOS'u gerçek pencerede (macOS, gerçek fare ve klavye) kullanırken bulunan ve düzeltilen hatalar. Her kayıt şunları
yazar:

- ne görüldü;
- nedeni;
- ne değişti;
- nasıl doğrulandı.

## 1. Sahneden seç, başka katmandaki alanı almıyordu (19 Kasım 2026)

**Bildiren:** sahip. Sözü: “sahneden araç seçip processing tool ile kenar uzunluklarını yazdırma aracı çalışmıyor, sahnedeki
poligonu seçemiyoruz, seçmeye uygun”.

**Gerçek pencerede yeniden üretildi:**

1. KentOS CAD › Yeni proje › CBS (Ankara, TUREF / TM33).
2. Düzenle › Kapalı alan ile dört köşeli bir alan çizildi.
3. Öznitelikler'de alanın katmanı Kadastro / Parsel sınırı yapıldı. Etkin katman Taslak kaldı. Oto. seçim kapalıydı; bu,
   CBS projesinin kendi ayarı.
4. İşlemler › Kenar uzunluklarını yaz › Sahneden seç ile alanın içine ve kenarına tıklandı:
   - hiçbir şey alınmadı, satır “(0 seçili)” kaldı;
   - hiçbir ileti çıkmadı;
   - imleç alanın üstüne gelince vurgu da olmadı.

**Neden:** `PickObjects::takes`, Otomatik seçim kapalıyken seçen adımlar gibi yalnız etkin katmanın nesnelerini alıyordu
(ADR 0622 §5). Oysa pencerenin alanı nesneyi düzenlemez, okur. Alanın kendi kapsamları (Görünen, Tümü, Katman) da her
katmandan alır.

**Düzeltme:**

- `crates/native/interaction/src/viewer_tools/pick_objects.rs`:
  - Alanın nesneleri her görünen katmandan alınır; seçim süzgeci geçerli kalır.
  - Pencerenin üzerinde çalışacağı seçim için ayrı bir kip var: `PickObjects::selection` (Çizimden seç). Bu kip etkin
    katmanla sınırlı kalır.
  - Hiçbir şey almayan tıklama, imlecin altında bir nesne varsa nedenini söyler. Nedenler: seçim süzgeci, alanın almadığı
    tür, formun katmanı, Otomatik seçim'in katmanı.
- `crates/native/interaction/src/viewer_tools/selectable.rs`:
  - kutu için `in_box_read`;
  - başka katman iletisi tek yerde (`other_layer_words`).
- `apps/desktop/src/shell/window_pick.rs`: Çizimden seç `PickObjects::selection` kullanır.
- Belgeler:
  - ADR 0088 güncellemesi;
  - ADR 0622 §5'teki okuyan adımlar listesi;
  - kılavuz §0;
  - TODOS.md §0.2 (web).

**Doğrulama:**

- `cargo test -p kentos-interaction --test all`: 994 test geçti, 3'ü yeni.
- `cargo test -p kentos-desktop --lib -- input_objects room_modules the_start_vertex_is_picked`: 13 test geçti, 1'i
  yeni.
- Gerçek pencerede:
  - Parsel sınırı'ndaki alan, imleç üstüne gelince vurgulandı; tıklayınca alındı (“1 seçili”).
  - Pencere “1 kapalı alan; seçili nesneler” dedi.
  - Çalıştır “1 nesneye 4 kenar uzunluğu yazıldı.” dedi; üst kenarda “306.505” okundu.
- Resimler `.run/gui-evidence/sahneden-sec-katman/` altında:
  - `once-0-secili.png`: düzeltmeden önce;
  - `sonra-vurgu.png`, `sonra-1-secili.png`, `sonra-calistir.png`, `sonra-yazi.png`: düzeltmeden sonra.

## 2. Türkçe Mac klavyesinde “@” yazılamıyordu; göreli koordinat girilemiyordu (19 Kasım 2026)

**Görülen:**

- Türkçe Q (PC) düzeninde “@” Option+Q ile yazılır. KentOS bunu Alt+Q, yani Menülerde ara sayıyordu.
- Mimari projede Duvar ikinci noktayı beklerken “@12,0” yazıldı. Menü araması açıldı, “12,0” arama kutusuna gitti.

**Neden:** Üçüncü düzeyin simgesi yalnız Windows'un AltGr'si (Ctrl+Alt) için yazı sayılıyordu. macOS'ta aynı işi Option
yapar ve uygulamaya Alt diye gelir.

**Düzeltme:** `apps/desktop/src/shell/keys.rs`. macOS'ta Option'la yazılan, harf ya da rakam olmayan simge yazıdır.
Option'la yazılan harf (ABD düzeninde ⌥Q'nun œ'si) ve yazı üretmeyen tuşlar (Alt+Up) kısayol kalır. ADR 0018 güncellemesi.

**Doğrulama:**

- Testler:
  - `keys::tests::option_symbols_are_text_on_macos`;
  - `option_contract_tests::option_q_types_the_at_of_a_relative_point_on_a_mac`.
- Gerçek pencerede: ⌥Q ile “@12,0” değer alanına yazıldı. Duvar “@0,9”, “@-12,0”, “@0,-9” ile kapandı (42,000 m).

## 3. Kenar uzunluklarını yaz, sahibin kendi oturumunda hâlâ poligonu almıyordu (19 Kasım 2026)

**Bildiren:** sahip, 1. düzeltmeden sonra. Sözü: “Dostum Kenar uzunluklarını yaz ProcessingTool halen çalışmıyor neden?”

**Bulunan:**

- Sahibin `~/.local/state/kentos-cad/islemler.json`'ı aracın son değerlerinde `kinds: ["line"]` tutuyordu. Eski bir
  oturumda tür çiplerinden yalnız Çizgi bırakılmıştı.
- Bu daraltma her açılışta geri geliyordu. Kapsamda nesne yokken çipler gizlendiği için görünmüyordu.
- Sonuç:
  - Sahneden seç poligonu almadı (yalnız çizgiyi alıyordu);
  - Katman kapsamı “Çizim katmanında uygun nesne yok” dedi;
  - pencere aynı anda “Uygun nesneler: kapalı alan, çoklu çizgi, çizgi” yazıyordu.

  Bir kez çip tıklanınca araç poligon için kalıcı olarak bozuluyordu.

**Düzeltme (`apps/desktop/src/gis/processing/`):**

- Son değerler geri gelir, tür daraltması gelmez (`without_kind_narrowing`).
- Sahneden seç aracın kendi türlerini alır; seçilen nesneler alanındır, daraltma kalkar.
- Daraltma çipleri her zaman görünür (kapsamda olmayan tür 0 sayısıyla).
- ADR 0088 güncellemesi 2.

**Doğrulama:**

- `processing::tests::a_remembered_kind_narrowing_does_not_come_back` ve
  `processing::tests::a_remembered_kind_narrowing_does_not_refuse_a_picked_area`.
- Gerçek pencerede, sahibin `islemler.json`'ının ve `Harita-CAD-Denemesi.kcad`'inin kopyasıyla, yalıtılmış ev dizininde:
  - Katman kapsamı “1 kapalı alan; “Çizim” katmanında” dedi;
  - Çalıştır “1 nesneye 4 kenar uzunluğu yazıldı.” dedi (3.22, 1.59, 2.20, 1.45);
  - Sahneden seç alanı aldı (“1 kapalı alan; seçili nesneler”).

**Not:** sahibin açık penceresi (`cargo run`, 22:25) bu düzeltmeden önce derlendi; yeniden başlatılmalı.

## 4. Otomatik seçim kapanınca hiçbir şey seçilemiyordu ve nedeni görünmüyordu (19 Kasım 2026)

**Bildiren:** sahip. Sözü: “bir kez bozulunca hiç bir şekilde seçilemiyor”.

**Görülen:**

- Mimari çizimde her nesne kendi aracının katmanındadır (MM-DUVA-TUGLA…). Durum çubuğundaki “Oto. seçim” bir tıkla kapanır.
- Kapalıyken duvara tıklamak hiçbir şey seçmedi. Neden yalnız kesilmiş durum satırındaydı (“Tıklanan nesne başka katmanda.
  Otomati…”).
- Ayar projeyle kaydedildiği için sonraki açılışta da sürüyordu.

**Düzeltme:**

- Otomatik seçim kapanınca “Otomatik seçim kapandı” bildirimi çıkar. Bildirim neyin seçilebileceğini sayıyla söyler ve
  **Geri aç** düğmesini taşır.
- Bir tık ya da kutu başka katmanın nesnesini dışarıda bırakırsa “Başka katmandaki nesne seçilmedi” bildirimi çıkar ve
  **Otomatik seçimi aç** düğmesini taşır.
- Düğmeli bildirim 8 saniye kalır. Gerçek pencerede 5 saniyelik bildirim, düğmesine varılmadan kapandı.
- Değişen dosyalar: `ViewChange::LayerLeftOut` (`kentos-interaction`), `apps/desktop/src/gis/layer_types.rs`.
- ADR 0622 §5.

**Doğrulama:**

- `selecting::tests::otomatik_secim_off_says_why_and_its_notice_turns_it_on` (iki bildirim, iki düğme, ardından seçim).
- `pick_objects::a_windows_selection_pick_keeps_to_the_active_layer_and_says_why`.
- Gerçek pencerede:
  - komut satırından kapatılınca bildirim çıktı; Geri aç tek tıkla açtı;
  - başka katmandaki ölçü yazısına tıklayınca “Başka katmandaki nesne seçilmedi” bildirimi çıktı.

  İkinci bildirimin düğmesi gerçek pencerede tıklanmadı; aynı düzenek Geri aç'ta denendi, ayrıca birim testinde var.

## 5. Otomatik seçim'in ikonu (19 Kasım 2026)

**İstek (sahip):** “otomatik seçim için ikon da belirle”.

**Görülen:** eski ikon katman yığınının köşesinde küçük bir oktu. 16 pikselde ok seçilmiyordu.

**Düzeltme:**

- Dört seçenek çizildi (`.run/gui-evidence/ikon-otomatik-secim/secenekler.png`).
- A seçildi: katmanların önünde bir ok.
- `catalog/v1/additions.json`'ın `autoSelect` ikonu değişti.

**Doğrulama:** Harita › Seçim'de 16 piksel boyunda okunuyor; açık ve koyu temada çizildi.

## Bulunan, henüz düzeltilmeyen

- **macOS'ta menülerdeki kısayollar Ctrl ile yazılıyor.** Değiştir › Seçim ▾ menüsünde “Ctrl+Shift+J” görünüyor, ⇧⌘J
  olmalı. Durum satırı ve bildirimler `keys::shown`'dan geçiyor, menülerin kısayol sütunu geçmiyor.
- **Çipler ve tutarlılık:** bir kapsamın tür çipleri yalnız kapsamda iki ya da daha çok tür varken görünür.

  3. kayıttaki düzeltme daraltmayı her zaman gösterir. Daraltma yokken tek türlü kapsamda çip yoktur; bu bilinçli
  bırakıldı.
