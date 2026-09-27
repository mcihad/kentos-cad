# ADR 0074: Masaüstünde tutamaçlar ve yapıştırma ürün komutlarıyla; tutamaç menüsü

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.7; TODOS.md `CMD-07`, `UI-11`; ADR 0056 (pano), 0059 (çizim alanının menüleri), 0066 ve 0069 (ürün komutlarıyla yazma), 0068 (tutamaçlar), 0073 (bulut projesinin işlemleri)
- **Kaynak:** web ajanının üç commit'i ve web kodu:
  - `dd39864`: tutamaçlar `cad.entities.edit` ile yazar; sözleşmeye `grip`, `straightEdge` ve `arcEdge` işlemleri eklendi.
  - `7a5ca22`: Yapıştır `cad.entities.create` ve `cad.entities.set` ile yazar.
  - `f7616e4`: Buluta yükle, çizim yolda değişse de kapanır.
  - Web kodu: `ui/shell/viewportMenus.ts` (`gripItems`), `tools/SelectTool.ts` (`commitGrip`), `tools/editTools.ts` (`pasteEntities`).

## Bağlam

- Masaüstünde tutamaçla düzenleme ve yapıştırma belgeye doğrudan yazıyordu (`update_many`, `add_many`). Web ikisini de ürün komutlarına geçirdi. Python ve AI aynı komutları kullanacak (`CMD-07`).
- Web'de bir tutamacın üstünde sağ tıklayınca menü tutamacın öğeleriyle başlar. Masaüstünde bu yoktu (ADR 0059, "Bu dilimde olmayanlar").
- Web'in Buluta yükle penceresinde bir durum vardı: yükleme sürerken çizim değişirse proje oluşur ama ekrandaki çizime bağlanmaz. Pencere bu durumda açık ve düğmesi etkin kalıyordu; ikinci basış ikinci proje açıyordu. Masaüstünde yükleme penceresi kalıcıdır, başarıdan sonra hep kapanır. Yine de proje sunucudan hep açılıyordu. Açılış, yolda gelen değişikliğin yerine geçebilirdi.

## Karar

### Tutamaçla düzenleme (`crates/native/interaction/src/select.rs`)

- Tutamaç `cad.entities.edit` ile yazar: işlem `grip`, adım “Tutamaçla düzenle”.
- Nesne kalıcı kimliğiyle anılır; kimlik tutamaç alınırken okunur. Yeni geometrinin tamamı açıkça gönderilir.
- Tutamaç beklerken iki şey olabilir, ikisinde de hiçbir şey yazılmaz:
  - Katman Katmanlar panelinden kilitlenirse komutun reddi söylenir: “Çizim” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.
  - Nesne silinir ya da geri alınırsa şu söylenir: Tutamacın nesnesi artık çizimde yok (silinmiş ya da geri alınmış); tutamaç bırakıldı.
- Şekli bozan konumun iletisi aynı kalır.

### Tutamaç menüsü

Kod: `crates/native/interaction/src/grip_menu.rs` ve `apps/desktop/src/drawing_menus.rs`.

- **Nerede açılır:** komut yokken, seçili bir yolun (açık çoklu çizgi ya da kapalı alan) tutamacında sağ tıklanınca. Boştaki menü tutamacın öğeleriyle başlar, sonra bir ayırıcı gelir (web'in `gripItems`'i). Kısa tık ve basılı tutma aynı menüyü açar.
- **Köşe:** başlık “Köşe N”, tek öğe “Köşeyi sil”. Köşenin iki kenarı tek düz kenar olur.
- **Kenar ortası:** başlık “Kenar N”.
  - “Ortasına köşe ekle”: yay kenar çemberini koruyarak ikiye bölünür.
  - Yay kenarda “Düz kenar yap”.
  - Düz kenarda “Yaya dönüştür”. Yayın yüksekliği kirişin dörtte biridir (bulge 0,5). Halkada dışarı, açık yolda ilerleme yönünün sağına doğru bombelenir.
- **Menüsüz tutamaçlar:** deliğin köşeleri yalnız sürüklenir; deliğin şekli için Patlat gerekir. Çizgi ve daire gibi yol olmayan nesnelerin tutamaçlarında da menü yoktur.
- **Yazma:** her öğe `cad.entities.edit` ile tek adımdır. İşlem adımın adını verir: Köşe sil, Köşe ekle, Düz kenar yap, Yaya dönüştür.
  - Nesnenin geometrisinden yalnız öğenin değiştirdiği kısım değişir; delik yerinde kalır.
  - Hiç yay kenar kalmayınca bulge'lar düşer.
- **İletiler:**
  - Başarıda “<adım>: tamam.”
  - Çekirdeğin reddi söylenir. Örnek, üçgenin köşesi: “Kapalı alanda en az üç köşe kalmalı.”
  - Komutun reddi de söylenir (kilitli katman).
- İkonlar web'inkilerdir: `erase`, `vertex`, `line`, `arc`.

### Yapıştır (`crates/native/interaction/src/clipboard.rs`)

- Web'in `7a5ca22`'si gibi yazar: `cad.entities.create` ve semboller için `cad.entities.set` (işlem `symbol`), tek adımda, “Yapıştır”.
  - Aynı katmana giden ardışık nesneler tek create ile yazılır. Nesnelerin ve yuvalarının sırası panodaki sıradır.
  - Yeni nesne sembol taşımaz. Semboller bu yüzden ayrıca yazılır; aynı sembolü alanlar tek istekte gider.
  - Komutlardan biri reddederse bütün yapıştırma geri alınır ve komutun sözü söylenir. Örnek, boş yazı: “Yazının metni boş olamaz; yalnız boşluktan oluşan metin de boştur. Yazıya bir metin verin.”
- Davranış değişiklikleri (web'le aynı):
  - Katman kimliği çizimde bir grubu gösteren nesne etkin katmana gider. Önceden gruba yazılıyordu.
  - Gizli hedef katman bir kez söylenir: “X” katmanı gizli; yapıştırılan nesneler görünmeyecek.
  - Kalanı aynıdır: kilitli etkin katmanın reddi, “N nesne yapıştırıldı.” ve tek geri alma adımı.

### Buluta yükle: yükleme sürerken değişen çizim (`apps/desktop/src/cloud/upload.rs`)

- Yükleme bitince çizimin (oturum, revizyon) çifti gönderilenle karşılaştırılır. Fark varsa:
  - proje açılmaz ve pencere kapanır;
  - web'in sözü söylenir: “X” bulut projesi oluşturuldu ve çizim içe aktarıldı (N nesne), ama çizim yükleme sürerken değişti; ekrandaki çizim projeye bağlanmadı ve değişiklikleri yerinde duruyor. Projeyi Bulut projesi aç ile açın.
- Açık bir veritabanı projesine başkalarının değişiklikleri yükleme sürerken gelebilir; kural bunun için.
- Yükleme bir çakışmadan geldiyse (“Ayrı proje olarak kaydet”) cihazda tutulan kayıt da silinmez. Yeni proje en yeni işi tutmaz.

## Web'den ayrılanlar

- **Değişen çizim kuralının kapsamı:** web'de yalnız veritabanı yüklemesi değişen çizimi bağlamaz; dosya yüklemesi hep bağlar. Masaüstünde kural iki saklama biçiminde de geçerlidir. Masaüstü iki biçimde de projeyi sunucudan açarak bağlar ve açılış değişikliği ezerdi.

## Doğrulama

- `kentos-interaction`:
  - `grips::a_waiting_grip_meets_a_lock_or_a_deletion`;
  - `grips::the_grip_menu_edits_a_vertex_or_an_edge_through_the_command`: dört öğe, adımların adları ve geri alınmaları, üçgenin reddi, çizgide menü olmaması;
  - `clipboard::a_paste_goes_through_the_commands_in_one_step`: gizli katmanın sözü, grup kimliği, boş yazının reddinde hiçbir şey yazılmaması, tek adım.
- Masaüstü:
  - `drawing_menus::tests::a_right_click_on_a_grip_offers_its_actions`;
  - `cloud::tests::a_drawing_changed_while_it_went_up_stays_as_it_is`.
- `fixtures/interaction/v1` izleri (`grips`, `clipboard` dahil) iki platformda değişmeden geçer.
- Görüntüler: `cargo test -p kentos-desktop drawing_menus::screens -- --ignored --nocapture`. Dosyalar `.run/shots/sag-tik-tutamac-kose-*` ve `sag-tik-tutamac-kenar-*`: koyu ve açık, 1440×900 ve 1100×650.
- Geçenler:
  - `pnpm rust:test` ve `pnpm rust:test:desktop`;
  - `pnpm typecheck`, `pnpm test` ve `pnpm build`;
  - `pnpm e2e` ve `pnpm e2e:interaction`;
  - `pnpm inventory:check`;
  - `KENTOS_E2E_DB=scratch pnpm e2e:cloud`.
