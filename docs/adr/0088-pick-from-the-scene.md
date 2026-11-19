# ADR 0088: Sahneden seç: pencerelerin alanları çizimden seçilir

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7; ADR 0070 (çizimden nokta alma), 0084 (masaüstünde İşlemler).
- **Kaynak:** KentOS UI'ın nesne inceleyicisindeki nesne başvurusu alanı (`widget/inspector.rs`): hedef ikonlu “Haritadan seç”, seçim sürerken “Haritada seçin”. Web'in `PickPointTool`'u ve İşlemler penceresinin nokta alanı (“Haritadan göster”).

## Bağlam

Sahip, 27 Eylül'de şöyle yazdı:

> numaralandırma falan yaparken ilk numara sahneden seçici ile nokta seçilebilmeli, sahneden obje seçici bileşenimiz var

ve

> sahneden seçim çok çok önemli ve bir çok yerde kullanılmalı, nokta çizgi poligon ve diğer objeler seçilebilmeli arayüzden böylece iş kolaylaşır

**İşlemler penceresinde bugün:**

- **Başlangıç noktası:** yalnız “Başlangıç köşesi” açılır listesinde “Seçilen noktaya en yakın” seçilince görünen bir nokta alanı ve onun “Haritadan göster” düğmesi vardı. Numaralamanın başlangıcını çizimden vermek iki adımdı ve kolay bulunmuyordu.
- **Girdi nesneleri:** yalnız bir kapsamla verilebiliyordu: Seçili, Görünen, Tümü, Katman. Pencereyi kapatıp çizimde seçmek, sonra aracı yeniden açmak gerekiyordu.

## Karar

### Nokta alanları

Düğme KentOS UI'ın seçme düğmesidir: hedef ikonu, adı “Sahneden seç”. Nokta verilince adı “Yeniden seç” olur. Nokta yokken alan “Henüz seçilmedi” der.

Seçim ADR 0070'in `PickPoint`'udur:

- pencere kenara çekilir;
- kenet, orto ve kutupsal izleme her komuttaki gibi çalışır;
- nokta tıklanabilir ya da Y,X yazılabilir;
- Esc, Enter ve hızlı sağ tık vazgeçer.

### Bir seçeneği çizimdeki nokta olan seçimler

- **Tanım:** `ParamDef::picks_point(option, point)`, `kentos-processing`'te. Numaralamanın “Başlangıç köşesi” bununla tanımlanır: `picks_point("point", "startPoint")`.
- **Düğme:** pencere seçimin yanına hedef ikonlu bir düğme koyar.
  - Seçilen nokta nokta alanına yazılır ve seçim o seçeneğe geçer.
  - Böylece başlangıç köşesi tek tıkla çizimden verilir.
  - Seçenek seçiliyken ikon vurgu rengindedir.
  - Esc hiçbir şeyi değiştirmez.

### Girdi nesneleri: `kentos_interaction::pick_objects::PickObjects`

Nesne alanının kapsam seçiminin yanında hedef ikonlu “Sahneden seç” düğmesi durur.

- **Başlarken:**
  - Pencere kenara çekilir; o anki seçim saklanır ve seçim boşalır.
  - Satır “Alanlar: nesneleri tıklayın ya da pencereyle seçin (0 seçili) [Bitti (Enter) / Vazgeç (Esc)]” der.
- **Tıklama:**
  - İmlecin altındaki nesneyi seçime ekler ya da çıkarır.
  - Yalnız alanın türleri alınır: aracın `kinds`'i, kullanıcı tür çipleriyle daralttıysa onlar.
  - İmlecin altında başka türden bir nesne varsa, alınan türlerden kenarı en yakın olan seçilir: noktanın üstünde tıklamak alanı değil, yakındaki parseli alır.
  - İmlecin altında alınan türden kenar da yoksa, içinde tıklanan en küçük alan alınır.
  - Kenet yoktur.
- **Sürükleme:** 4 pikselden sonra kutu çizer; seçim aracının kuralıyla soldan sağa pencere, sağdan sola kesişim. Kutudakilerden alanın türleri eklenir.
- **Enter, Boşluk ya da hızlı sağ tık:**
  - Pencere geri gelir; alanın kapsamı Seçili olur, tür seçimi korunur.
  - Satıra “n nesne seçildi.” yazılır.
- **Esc:** pencere olduğu gibi geri gelir, önceki seçim yerine konur.
- **Katalog:** araç katalogda değildir ve son komut sayılmaz (`Session::run`).
- **Ev sahibine bildirim:** `ViewChange::PickedObjects(bool)`.

### Web

Web ajanı aynısını yapacak:

- sözcükler: “Sahneden seç”, “Yeniden seç”, “Henüz seçilmedi”; web'in `dialogTexts`'i ve `fixtures/processing/v1/dialog.json` onlarla değişecek;
- aracın tanımında `picks` bilgisi;
- nesne seçme aracı.

### Sıradaki yerler

Aynı düğme ve araçlar başka pencerelerde de kullanılacak:

- Hesap pencereleri noktayı zaten çizimden alıyor (ADR 0070);
- ifade oluşturucunun önizleme nesnesi;
- stil pencerelerinin örnek nesnesi;
- öznitelik ve bağlantı alanları.

## Doğrulama

- **`processing::tests::the_start_vertex_is_picked_on_the_drawing_from_beside_its_choice`:**
  - başlangıç köşesinin yanındaki düğme noktayı ister;
  - yazılan nokta alana gelir, seçim “point” olur;
  - Esc'te seçim değişmez.
- **`processing::tests::input_objects_are_picked_on_the_drawing_of_the_fields_kinds`:**
  - numaralamada nokta ne tıklamayla ne kutuyla alınır;
  - parsel içine tıklanınca alınır, satır sayıyı gösterir;
  - Enter'da alan Seçili olur;
  - Esc'te önceki seçim geri gelir.
- **Görüntüler:** `processing::tests::screens`, koyu ve açık tema, 1440×900 ve 1100×650.

## Güncelleme (19 Kasım 2026): alanın nesneleri her görünen katmandan; alınmayan tıklama nedenini söyler

**Sahibin bildirdiği:** “sahneden araç seçip processing tool ile kenar uzunluklarını yazdırma aracı çalışmıyor,
sahnedeki poligonu seçemiyoruz, seçmeye uygun”.

**Gerçek pencerede yeniden üretildi (macOS, gerçek fare ve klavye):**

- CBS projesi açıldı. Otomatik seçim projenin kendi ayarıyla kapalı, etkin katman Taslak.
- Düzenle › Kapalı alan ile bir alan çizildi ve Kadastro / Parsel sınırı katmanına taşındı.
- Kenar uzunluklarını yaz › Sahneden seç: alanın içine de kenarına da tıklamak hiçbir şey almadı. Satır “(0 seçili)” kaldı,
  hiçbir ileti çıkmadı, üzerine gelince vurgu da yoktu.
- Nedeni: `PickObjects::takes`, Otomatik seçim kapalıyken seçen adımlar gibi yalnız etkin katmanın nesnelerini alıyordu
  (ADR 0622 §5).

**Karar:**

- **Alanın nesneleri okunur, düzenlemek için seçilmez.** Sahneden seç, alanın türlerini Otomatik seçim açık da kapalı da her
  görünen katmandan alır: tıklama, üzerine gelme ve kutu (`selectable::in_box_read`). ADR 0622 §5'in nesneden okuyan
  adımları (Buda'nın sınırları gibi kenar seçen adımlar) da böyledir. Alanın kendi kapsamları (Görünen, Tümü, Katman) zaten her
  katmandan alır.
- **Seçim süzgeci geçerli kalır** (ADR 0278 §2).
- **Pencerenin üzerinde çalışacağı seçim bir seçimdir.** Bu, Çizimden seç'tir (ADR 8006: Oda modülleri, Tasarım seçenekleri;
  `PickObjects::selection`). Otomatik seçim kapalıyken etkin katmanla sınırlı kalır.
- **Hiçbir şey almayan tıklama, imlecin altında ya da çevresinde bir nesne varsa nedenini söyler:**
  - formun listesi başka katmandan: “Parsel: yalnız “Çizim” katmanının nesneleri alınır; tıklanan nesne (Kapalı alan) başka
    katmanda.”;
  - seçim süzgeci: “Seçim süzgeci tıklanan nesneyi (Kapalı alan) dışarıda bıraktı.”;
  - Çizimden seç'te Otomatik seçim'in katmanı: seçim aracının iletisi;
  - alanın almadığı tür: “Alanlar: tıklanan nesne (Nokta) bu alana alınmaz.”

  Boş yere tıklamak bir şey söylemez.

**Doğrulama:**

- `kentos-interaction` testleri:
  - `pick_objects::a_fields_pick_takes_another_layers_area_with_otomatik_secim_off` (içine, kenarına, pencereyle);
  - `pick_objects::a_windows_selection_pick_keeps_to_the_active_layer_and_says_why`;
  - `pick_objects::a_click_that_takes_nothing_says_why`.

  Bütün `--test all` geçer: 994 test.
- Masaüstü testleri:
  - `processing::tests::input_objects_are_picked_from_another_layer_with_otomatik_secim_off`: düz tıklama almaz,
    Sahneden seç alır, alan “1 kapalı alan” der;
  - Oda modülleri testleri değişmeden geçer.
- Gerçek pencerede: Parsel sınırı'ndaki alan üzerine gelince vurgulandı ve tıklayınca alındı (“1 seçili”). Pencere “1
  kapalı alan; seçili nesneler” dedi. Çalıştır “1 nesneye 4 kenar uzunluğu yazıldı.” dedi; üst kenarda “306.505” okundu.
- **Web:** ertelendi (ADR 0228; TODOS.md §0.2).
