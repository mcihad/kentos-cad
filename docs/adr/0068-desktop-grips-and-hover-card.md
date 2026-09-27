# ADR 0068: Masaüstünde tutamaçlar ve üzerine gelme kartı

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7, §4.9; DESIGN.md §7.5; TODOS.md `UX-01`, `UI-11`; ADR 0029 (seçim ve kenet), 0059 (çizim alanının menüleri)
- **Sahibin yönü (26 Eylül):** web'deki araçlar ve davranışlar masaüstüne birebir taşınır.
- **Web'in kaynağı:**
  - `tools/SelectTool.ts` (tutamaç düzenleme);
  - `viewport/overlay.ts` `drawGrips`, `ViewportController.gripAt`;
  - `ui/shell/HoverCard.ts`.
  Koddan okundu.

## Bağlam

- Web'de komut yokken seçili nesnelerin tutamaçları görünür ve sürüklenerek düzenlenir. Nesnenin üzerinde durunca bir kart ne olduğunu söyler.
- Masaüstünde ikisi de yoktu. CLAUDE.md "çizim alanında tutamaçlar"ı gelecek iş sayıyordu.
- Tutamaçların hesabı ortak çekirdekteydi (`ops/grips.rs`: `entity_grips`, `move_grip`; deponun `grips`'i).

## Karar

### Tutamaçlar (`crates/native/interaction/src/select.rs`)

- **Görünüm** (`apps/desktop/src/marks.rs`, web'in `drawGrips`'i). Çizimin üstündeki işaret katmanında (Iced canvas):
  - En çok 150 seçili nesnenin tutamaçları çizilir; bir komut çalışırken de.
  - Tutamaç 6 px vurgu renginde karedir; 7 px çerçevesi çizim zemininin rengindedir. Ekranda 9 px'ten yakın tutamaçlardan yalnız biri çizilir.
  - Yolun kenar ortaları küçük, içi boş eşkenar dörtgendir; kenar ekranda 28 px'ten kısaysa gösterilmez ve alınmaz.
  - Taşınan tutamaç 8 px, çizimin mürekkep renginde, 1,5 px vurgu çerçevesiyle çizilir.
- **Alma:**
  - Komut yokken, seçili ve kilitli katmanda olmayan bir nesnenin tutamacına 6 px içinde basmak onu alır; en yakın tutamaç kazanır.
  - Kilitli katmandaki nesnenin tutamacı görünür ama alınmaz: basmak seçim kutusu başlatır.
- **Taşıma:**
  - Sürüklenen tutamaç düğmenin bırakıldığı yere gider.
  - Sürüklenmeden tıklanan tutamaç "sıcak" kalır; sonraki tık onu yerleştirir.
  - Yazılan nokta da yerleştirir, tutamacın eski yerine göredir (`@0,4`).
  - Enter, Boşluk ya da kısa sağ tık imlecin olduğu yere koyar. Esc tutamacı bırakır; nesne ve seçim yerinde kalır.
  - Kenet (tutamacın eski yerinden dik ve teğet dahil), orto ve kutupsal izleme uygulanır. Seçim aracı yalnız tutamaç taşırken kenetler.
- **Yazma:**
  - Tek adım: "Tutamaçla düzenle". Nesnenin yuvası ve kalıcı kimliği kalır.
  - Yerinde kalan tutamaç adım yazmaz.
  - Şekli bozan konum yazılmaz: "Bu konum geçersiz bir şekil oluşturuyor; tutamaç yerinde bırakıldı."
  - Web gibi belgeye doğrudan yazılır; ürün komutu yoktur.
- **Kenar ortası:** düz kenarda yeni köşe olur, yaylı kenarda yayı yeni noktadan geçirir. Deliklerin köşeleri de tutamaçtır.
- **İstem:** "Tutamaç: yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)". Komut satırı onu komutun istemi gibi gösterir.
- **Önizleme:**
  - nesnenin olacağı hâli 1,5 px kesikli (4/3);
  - eski yerden kesikli (2/3) çizgi;
  - imlecin yanında uzaklık;
  - kutupsal izleme çizgisi.
- **Oturum** (`session.rs`): komut yokken yazılan değer, Enter, Esc, kenet, istem ve önizleme tutamaç taşınırken seçim aracınındır (`Session::grip_active`).
- **Çift tık:** tutamacı alan ya da yerleştiren bir basış, yazıyı yerinde düzenlemeyi açmaz.

### Üzerine gelme kartı (`apps/desktop/src/hover_card.rs`)

- **Ne zaman:** komut yokken imleç aynı nesnenin üzerinde yarım saniye durunca açılır.
  - İmleci 18 px sağdan, 20 px aşağıdan izler.
  - Nesneden ya da çizimden çıkınca, bir komut başlayınca ya da tutamaç alınınca kapanır.
  - `drafting.hoverInfo` kapalıyken hiç açılmaz.
- **Başlık:**
  - Nesnenin Parsel'i varsa "Parsel 7";
  - yoksa türü ve etiketi ("Daire", "Kapalı alan 101/7").
  - Yanında katmanın renk örneği ve adı.
- **Satırlar, web'in sırasıyla:**
  - Ada, Mahalle, Nitelik;
  - tapu alanı ("Tapu alanı (m²)" özniteliği) ve yanında "Hesaplanan alan"; tapu alanı yoksa "Alan". Tapu alanı yazıldığı gibi gösterilir: ayrıştırılmaz, yuvarlanmaz, birimi değiştirilmez (CLAUDE.md §7, §23.1). Düz bir ondalık sayıysa (nokta ya da virgülle) sonuna "m²" eklenir. Web'de `7b5d07f`, masaüstünde aynı birleştirmede;
  - "Ada (delik)";
  - Çevre (kapalı alan, daire) ya da Uzunluk;
  - Yarıçap;
  - yazının Metin'i;
  - noktanın Kot'u.
  Sayılar projenin birimi ve basamağıyla, eş aralıklı yazılır.
- **Görünüm:**
  - açılır menülerin kutusu (KentOS UI `popover`);
  - başlığın altında ince çizgi;
  - adlar soluk, değerler sağa hizalı;
  - en az 160 px genişlik.
- Bekleme, kurtarma kopyasının zamanlayıcısı gibi kendi iş parçacığında sayılır. Seçimin üzerine gelme sayacı (`hover_version`) eskiyen beklemeleri ayırır.

### Ortak iz

`fixtures/interaction/v1/grips.json`, `objects.kcad` üstünde. İki platformda şunları oynatır:
- çizginin ucunu sürükleme ve geri alma;
- sıcak tutamacı tıkla yerleştirme;
- Esc ile bırakma;
- Enter ile imlecin yerine koyma;
- kapalı alanın kenar ortasından yeni köşe;
- kilitli katmandaki nesnenin alınmayan tutamacı.

## Web'den ayrılanlar

- **Yazılan nokta:** web'de tutamaç sürerken yazılan koordinat seçim aracına ulaşmıyor, istem ise "koordinat yazın" diyor. Komut satırı ve imleç yanındaki alan, çalışan araç `select` iken yazılanı araca vermiyor. Masaüstü istemin dediğini yapar. Web düzeltmesi web ajanının 14b görevidir; gelince iz yazılan adımı da oynatacak.
- **Kartın yerleşimi:**
  - Web'de katmanın adı başlık satırının sağ ucundadır; masaüstünde başlığın hemen sağındadır.
  - Değerler web'de kartın sağ kenarına, masaüstünde kendi sütunlarının sağına hizalanır.
  - Sebep: Iced, içeriğe göre daralan kutuda dolduran satırları sıfıra indiriyor.

## Doğrulama

- `crates/native/interaction/tests/grips.rs` (8 test):
  - sürükleme ve adımın adı;
  - sıcak tutamaç ve önizlemesi;
  - yazılan nokta;
  - Enter ve Esc;
  - kenar ortası ve kısa kenarın gizli tutamacı;
  - kilitli katman;
  - şekli bozan konum;
  - kenet.
- `apps/desktop/src/hover_card.rs`:
  - satırların web'in sırasıyla oluşu, tapu alanının yazıldığı gibi gösterilişi;
  - bekleme, eskiyen bekleme, ayar ve çalışan komut.
- İz `grips`: web `pnpm e2e:interaction` ve masaüstü `cargo test -p kentos-desktop traces`.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm inventory:check`.
- Görüntüler (`hover_card::screens`, `.run/shots/tutamac-*`, `uzerine-gelme-*`), koyu ve açık, 1440×900 ve 1100×650:
  - seçili parselin tutamaçları, taşınan köşe, kesikli önizleme ve uzaklık;
  - parselin üzerindeki kart.
