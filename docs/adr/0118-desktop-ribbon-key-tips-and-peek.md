# ADR 0118: Masaüstünde şeridin harf ipuçları ve daraltılmış şeridin çizimin üstünde açılması

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** docs/specs/ribbon.md §1 ve §6; ADR 0117 (hızlı erişim, sağ tık, bölünmüş düğme).
- **Kaynak:** web'in `ui/ribbon/keytips.ts` ve `Ribbon.ts`'i; ortak durumlar `fixtures/shell/v1/ribbon.json`'un `keyTips` ve `tabs` bölümleri.

## Bağlam

Web'in şeridinde harf ipuçları vardır. F6 ya da tek başına basılıp bırakılan Alt, hızlı erişim düğmelerine rakam, sekmelere harf koyar. Bir sekmenin harfi o sekmeyi açar; düğmeleri de harf alır. Masaüstünde bu yoktu, `view.keyTips` taşınmamıştı. Daraltılmış şeritte bir sekmeye tıklamak da web'de sekmeyi çizimin üstünde açar; masaüstünde yalnız sekme seçiliyordu, paneli görünmüyordu.

## Karar

- **Kurallar** `apps/desktop/src/keytips.rs`'tedir, web'in `keytips.ts`'i gibi:
  - adın harfleri (`letters_of`): Türkçe harfler katlanır, büyük harfe çevrilir;
  - ipuçlarının dağıtımı (`assign_key_tips`): önce tek harf, sonra iki; hiçbir ipucu bir başkasıyla başlamaz;
  - birinci düzey (`first_level_tips`): çubuğa rakam, sekmelere harf;
  - tuşların anlamı (`key_tip_step`).

  Testleri `ribbon.json`'un `keyTips` bölümünü oynatır. Sekmeler üç çalışma modunda da web'inkilerdir (`tabs`).
- **Uygulama** `apps/desktop/src/ribbon_keys.rs`'tedir:
  - F6, `view.keyTips` ya da tek başına basılıp bırakılan Alt açar; yeniden çağırmak kapatır. Açılınca klavye metin kutularından alınır, harfler ipuçlarına gelir.
  - Birinci düzeyde çubuğun kullanılabilen ilk dokuz düğmesi 1…9'u alır, görünen sekmeler harflerini alır; bağlamsal Seçim sekmesi yalnız görünürken harf alır.
  - İkinci düzeyde açık sekmenin kullanılabilen denetimleri okuma sırasıyla harf alır: paneller soldan sağa; her panelde önce düğmeleri, sonra ▾'i ve pencere açıcısı. Bölünmüş düğmenin üstü ve oku ayrı denetimdir; tek düğmeye katlanmış panel tek denetimdir. Hangi denetimin görüneceği, şeridin kendi sığdırma kuralıyla pencerenin genişliğinden bulunur (`kentos_ui::widget::ribbon::fit`).
  - Tamamı yazılan ipucu denetimi çalıştırır: düğme çalışır; menü (bölünmüş düğmenin oku, açılır düğme, panelin ▾'i, katlanmış panel) klavyeden açılır, ilk satırı vurgulu.
  - Esc önce yazılanı, sonra düzeyi geri alır; Backspace bir harf siler. Fare basışı, pencerenin odağı kaybetmesi ve boyutunun değişmesi ipuçlarını kapatır.
- **Daraltılmış şerit:** bir sekmeye tıklamak ya da harf ipucuyla açmak sekmeyi çizimin üstünde açar; çizim yerinden oynamaz. Aynı sekmeye yeniden tıklamak, bir komutun çalışması, Esc ya da şeridin dışına basmak kapatır. Daraltma saklandığı gibi kalır.
- **KentOS UI'a eklenenler:**
  - `widget::key_tip`: öğenin üstündeki katmanda çizilen, öğeden taşabilen ipucu kutusu. Yeri web'in kuralıdır: sekmenin altında, büyük düğmenin alt kenarında, küçük düğmenin ikonunda. Yazılanla başlamayan ipucu soluk çizilir.
  - Şeridin `Button::key_tips` ve `menu_id`'si; `Group::key_tips` ve `menu_ids`'i; `Ribbon::key_tips` ve `peek`'i.
  - `context_menu::open_menu`: menü düğmesini kimliğiyle klavyeden açar.

## Sonuçlar

- Masaüstünde şerit web'inki gibi klavyeyle kullanılır. Envanterde `view.keyTips` taşındı; şerit penceresi tamamlandı.
- **Farklar:**
  - Şeridin kendi panellerinin (Katmanlar, Özellikler, Seçim özeti) ve Görünüm'ün masaüstüne özgü gruplarının denetimleri harf almaz; web'de düğmeleri alır.
  - Web'de tek düğmeye katlanmış panel açılınca içindeki düğmeler yeniden harf alır. Masaüstünde katlanmış panel bir menüdür; menü klavyeyle gezilir.

## Doğrulama

- **`cargo test -p kentos-desktop keytips`:** `ribbon.json`'un harfleri, dağıtımları, birinci düzeyleri ve tuş adımları.
- **`cargo test -p kentos-desktop ribbon_keys`:**
  - sekmeler üç modda web'in;
  - F6 ile rakamlar ve harfler;
  - sekme harfiyle ikinci düzey ve düğmenin çalışması;
  - Esc, Backspace ve fare basışı;
  - tek başına Alt ile açılma, başka tuşla açılmama;
  - daraltılmış şeridin çizimin üstünde açılıp kapanması.
- **Resimler:** `cargo test -p kentos-desktop ribbon_keys::screens -- --ignored --nocapture`. Beş sahne çizilir: sekmelerde ve düğmelerde ipuçları; yazılan harfle soluklaşanlar; daraltılmış şeridin açılışı, ipuçlarıyla ve ipuçsuz. Her biri 1440×900 ve 1100×650'de, koyu ve açık temada `.run/shots/serit-*`'a yazılır.
