# ADR 0294: Buda ve Uzat'ta AutoCAD'in seçenekleri: Uzantılar, hızlı kipin silmesi, Çit ve Kesişen komutta

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-08`: “Trim/extend; seçili sınırlarla ve hızlı
  tüm-sınırlar moduyla, fence/crossing ve edge extension seçenekleriyle çalışsın.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0047 (değiştirme araçları), 0140 (Çit), 0250 (anlamsal Buda ve Uzat), 0278 (kilitli katman), 0291 (projenin
  açı düzeni).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

`cad.entities.trim` ve `.extend` (ADR 0250) tıklamaları ve sınırları alıyordu; Çit aracın kendi hesabıydı (kesişimleri araç bulup
seçim olarak veriyordu), AutoCAD'in öbür seçenekleri yoktu. AutoCAD'in TRIM ve EXTEND'iyle karşılaştırma:

| AutoCAD | Önce (ADR 0250) | Şimdi |
|---|---|---|
| Standart kip: seçilen sınırlar | Sınır seç (S), `boundaries.objects` | aynı |
| Hızlı kip: bütün görünen kenarlar | varsayılan; görünümün dikdörtgeni `boundaries.area` | aynı |
| Hızlı kipte sınırı olmayan nesne silinir | uyarı, nesne kalır | `eraseUncut`; araç hızlı kipte verir, standart kipte uyarır |
| Edge: Extend (EDGEMODE 1) | yok | `edgeMode: "extend"`; araçta Uzantılar (U), oturumda kalır, iki araç paylaşır |
| Fence | araçta (C), kesişimleri araç bulur | `fence` komutun girdisi; araç çitini komuta verir |
| Crossing | yok | `crossing` (Kesişen): araçta K ya da boş yere tıklama, iki köşe |
| Shift: öbür işlem | var | aynı; önizleme de öbür işlemin planı |
| eRase (TRIM) | yok | Sil (R): tıklanan nesneler `cad.entities.delete` ile silinir, R ya da Esc döner |
| Undo | Ctrl+Z araçta kalarak son adımı geri alır | aynı (Çit çizilirken son noktayı alır) |
| Çit'in ikinci kesişimi aynı parçada | kalan parçanın ucu kesimse “kesen kenar yok” uyarısı, parça kalırdı | parça bütünüyle gider (AutoCAD gibi) |
| Yay, daire, elips, çoklu çizginin yayları | var | aynı; Kesişen'de de |
| Eğri (Catmull–Rom) budama, NURBS uzatma | yok (Patlat'a yönlendirir; NURBS budanır) | aynı (aşağıda “Kalanlar”) |
| Delikli alan, çok parçalı nesne | reddedilir (`not_trimmable`, `trim_failed`): delikler ve parçalar kaybolmaz | aynı; Çit ve Kesişen'de uyarıyla atlanır |

## Karar

### 1. Girdi

`EntitiesTrim`'e dört isteğe bağlı alan (iki komut aynı türü alır):

- `fence`: açık yol, en az iki nokta (`too_few_points`). Geçtiği her görünen nesne geçtiği yerden seçilir; kesişimler çitin sırasıyla,
  nesneler çizimin sırasıyla (`ops::trim::fence_crossings`).
- `crossing {min, max}`: pencere. Buda, pencereye değen her nesnenin sınırlar arasındaki **pencereye artı uzunlukla değen** her parçasını
  kaldırır (yalnız köşesiyle değen parça kalır). Uzat, ucu pencerede olan her açık nesnenin o ucunu uzatır.
- `edgeMode`: `none` (varsayılan) ya da `extend`.
- `eraseUncut`: Buda'da hiçbir sınırın kesmediği nesne seçildiği yerden bütünüyle silinir (`removed`). Uzat yok sayar.

`picks` artık boş olabilir; seçim, çit ve pencere üçü de yoksa `no_entities`. Sıra: seçimler, sonra çit, sonra pencere; aynı nesnenin
hepsi aynı parçalar üstünde işlenir.

### 2. Uzantılar (EDGEMODE 1)

Sınır, doğal yolu boyunca uçlarının ötesinde de sınırdır (`ops::trim::extended_edges`):

| Sınır | Uzantı |
|---|---|
| çizgi | bütün doğrusu |
| yay | bütün çemberi |
| açık çoklu çizgi | ilk kenarı başlangıcının, son kenarı ucunun ötesine (düz kenar yarı doğrusuyla, yay kenar çemberiyle); aradakiler olduğu gibi |
| kapalı nesneler, elips yayı, eğriler, yazı ve öbürleri | olduğu gibi (kapalı nesnenin ucu yok; elips yayının ve eğrinin uzantısında AutoCAD ile KentOS anlaşmıyor) |
| yardımcı çizgi ve ışın | zaten sonsuz |

Doğru, yapım çizgisi gibi uçlarının 10⁶ m ötesine kadar giden bir kenardır (`CONSTRUCTION_REACH`).

### 3. Kalan parça

Bir nesnenin ilk kesiminden sonra her parçanın uçları ya önceki kesimlerdir ya nesnenin uçlarıdır. Bu yüzden sonraki bir seçim parçada
iki yanında kesim bulamazsa parça bütünüyle kalkar. Çit'in aynı nesneyi iki yerden geçtiği hâlde ikinci kesişimin uyarı vermesi
düzeldi (ADR 0250'nin ortak durumu buna göre değişti).

### 4. Kesişen'in yürüyüşü

Pencereye değen ilk parçanın pencere içindeki ilk kısmının ortası seçilir ve parça orada budanır; pencereye değen parça kalmayana dek
yinelenir (`ops::trim::window_points`, `Run::window`). Nesnenin ilk seçiminde orta nokta tek kesimin üstüne düşerse (kesim iki yanında
aranmaz) kısmın üçte biri ve üçte ikisi denenir; ancak hiçbiri kesmiyorsa nesne kesilmemiş sayılır.

### 5. Kilitli ve gizli

Çit ve pencere yalnız görünen nesneleri alır. Kilitli katmandakiler, çit onları geçiyor ya da pencere onlarda iş görüyorsa sayılarak
atlanır (`layer_locked` uyarısı, ADR 0278); yalnız onlar varsa `layer_locked` reddi. Açıkça seçilen kilitli nesnenin reddi değişmedi.

### 6. Araç

Buda ve Uzat her tıklamada, çitte ve pencerede komutu çalıştırır, önizleme komutun planıdır (pencereninki de: kalkacak parçalar kırmızı
kesikli). Seçenekler: Sınır seç (S), Tüm kenarlar (T), Kesişen (K), Uzantılar (U, açık/kapalı), Sil (R, yalnız Buda), Çit (C, şeritten).
Boş yere tıklama pencerenin ilk köşesidir (AutoCAD'in örtük penceresi). Hızlı kipte Buda kesilmeyen nesneyi siler (“Silindi: onu kesen
bir sınır yoktu (hızlı kip).”); Sınır seç ile seçilen sınırlarda uyarır. Kısayol penceresinde Buda ve Uzat'ın tuşları.

### 7. Tutamaç kipinde Döndür'ün açısı

Tutamaç kipinin Döndür'ü (ADR 0280) yazılan açıyı hep derece ve saat yönü tersine okuyordu; şimdi Döndür aracı gibi projenin düzeniyle
(ADR 0291): CAD'de derece saat yönü tersine, CBS'de grad saat yönünde; Referans'la iki değer de doğrultudur.

## Uygulama

- **Sözleşme:** `cad_trim.rs`'in `TrimEdgeMode`, `TrimWindow` ve `EntitiesTrim`'in `fence`, `crossing`, `edgeMode`, `eraseUncut`'ı;
  kataloğun özeti ve örnekleri; yetenekler (`too_few_points`, `layer_locked` uyarısı).
- **Çekirdek:** `ops::trim`'in `extended_edges`, `trimmable`, `window_point`, `window_points`'i; deponun `trim_preview_in` ve
  `extend_preview_in`'i (`extended` ile; eskiler aynı kalır, web'in WASM'ı değişmez).
- **Uygulama:** `crates/native/application/src/trim.rs` (`Run`, `Work`, `Met`).
- **Araç:** `crates/native/interaction/src/trim.rs`; `Memory::trim_edge_mode`; `grip_edit.rs`'in `typed`'ı projenin biçimiyle.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/trim_modes_cases.py` (KentOS kodu olmadan, bu ADR'nin tanımlarından; parçalar özgün yolun
  uzunluk aralıklarıyla, kesimler doğru, yarı doğru, kesit, yay ve çember kesişimleriyle): 52 durum,
  `fixtures/commands/v1/cad.entities.trim.modes.json` (29) ve `.extend.modes.json` (23). Plan ve yazma ayrı adımlarda; geometri 1e-9
  içinde (`within`; çember kesişimleri ve yay açıları). Masaüstü ve Python bit bit değil, 1e-9 içinde geçer.
- `edge_command_cases.py`: kalan parçanın kuralı ve çit dışı bir uyarı durumu.
- **İzler:** `edge-tools.json`, `edge-commands.json` seçenekleri yeni hâliyle; yeni `trim-modes.json` (Uzantılar, Kesişen, boş yere
  tıklama, hızlı kipte silme, Sil).
- **Resimler:** `KENTOS_SHOTS_ONLY=buda-uzanti,buda-kesisen,buda-sil cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.

## Kalanlar

- Catmull–Rom eğrisini budamak ve NURBS'ü uzatmak (AutoCAD ikisini de yapar); elips yayının ve eğrinin uzantısı Uzantılar'da.
- Çok parçalı çoklu çizginin yalnız tıklanan parçasını budamak (bugün reddedilir; parçalar kaybolmaz).
- AutoCAD'in hızlı kipteki serbest çizgi (sürükleyerek çit) seçimi; KentOS'ta Çit tıklamalarla çizilir.
