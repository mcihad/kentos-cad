# ADR 0117: Masaüstünde şeridin hızlı erişim çubuğu, sağ tık menüleri ve bölünmüş düğmenin seçimi

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** docs/specs/ribbon.md §2–§5; DESIGN.md §7.3; ADR 0017 (şerit), 0032 (bölünmüş düğmeler), 0115 (yerleşim).
- **Kaynak:** web'in `ui/ribbon/ribbonPlan.ts`, `controls.ts`, `Ribbon.ts` ve `app/ribbon.ts`'in `quickAccessOf`, `splitCurrent`'i; ortak durumlar `fixtures/shell/v1/ribbon.json`, `layout.json`.

## Bağlam

Masaüstünün şeridinde hızlı erişim çubuğu yalnız sabit üç komutu (Kaydet, Geri al, Yinele) gösteriyordu. Kullanıcı çubuğa komut ekleyemiyor, çıkaramıyordu. Şeritte sağ tıklamak hiçbir şey açmıyordu. Bölünmüş düğmenin üstünde hep ilk girdi duruyordu; web ise son seçileni gösterir ve saklar. Yerleşim (ADR 0115) `ribbonQuickAccess` ile `ribbonSplits`'i okuyup olduğu gibi geri yazıyordu, ama onlara bakan bir parça yoktu.

Web ajanı şeridin bu parçalarını, web kodunu görmemiş biri için `docs/specs/ribbon.md`'ye yazdı. Kurallarını DOM'suz bir plana ayırdı ve `fixtures/shell/v1/ribbon.json`'la sabitledi.

## Karar

- **Kurallar** `apps/desktop/src/ribbon_plan.rs`'tedir; web'in `ribbonPlan.ts`'i gibidir:
  - sözler;
  - çubuğun ▾ menüsü (`quick_access_menu`);
  - bir komutun çubuğa eklenmesi ya da çıkarılması (`with_quick_access`);
  - komut düğmesinin ve şeridin başka yerinin sağ tık menüsü (`command_menu`, `ribbon_menu`);
  - bölünmüş düğmenin listesi ve yüzü (`split_menu`, `split_face`).

  Testleri `ribbon.json`'un bu bölümlerini oynatır. Çubuk ve seçimin okunuşu `layout_plan.rs`'tedir (`quick_access_of`, `split_current`); `layout.json`'u oynar.
- **Hızlı erişim çubuğu:** sabit üç komut, ardından kullanıcının eklediklerinden uygulamada olanlar, eklendiği sırayla.
  - ▾ “Hızlı erişimi özelleştir” menüsü çubuktakileri ve önerilenleri işaretli ya da işaretsiz listeler; sabit üçü kapalıdır, yanında “sabit” yazar. En altta “Şeridi daralt” vardır.
  - Değişiklik yerleşimde (`ribbonQuickAccess`) saklanır.
- **Sağ tık:**
  - Bir komut düğmesinde (panelde, çubukta, bölünmüş düğmenin üstünde ya da okunda; kapalı olsa da) menü, çubuğa ekler ya da çıkarır. Sabit komutta “Hızlı erişimde (sabit)” kapalıdır. Altında “Şeridi daralt” vardır.
  - Şeridin başka bir yerinde yalnız “Şeridi daralt” çıkar.
  - Komut ara kutusunda şeridin menüsü açılmaz; kutu sağ tıkı kendisi alır.
- **Bölünmüş düğme:** üstünde son seçilen girdi durur (`ribbonSplits[anahtar] = "komut|seçenek"`). Anahtar web'inkidir: araç ailesinin adı ya da aracın kimliği. Envanter (`apps/web/scripts/inventory/collect.mjs`) onu, girdilerin başlıklarını ve açıklamalarını artık yazar.
  - Listeden seçmek seçimi saklar, üstü yeniler ve girdiyi çalıştırır: önce komut, sonra seçeneği aracın girdisine yazılmış gibi verilir. Araç seçeneği almazsa günlüğe ““Daire: 3 nokta” şu an başlatılamadı.” yazılır.
  - Üstü tıklamak seçimi değiştirmez.
  - Listenin satırında girdinin açıklaması ve kısayolu vardır.
- **KentOS UI'a eklenenler:**
  - `Button::context`: şerit düğmesinin sağ tık menüsü.
  - `Ribbon::quick_with_menu`: sağ tık menülü hızlı erişim düğmesi.
  - `Ribbon::quick_menu_tip`: ▾'in ipucu.
  - `Ribbon::context_menu`: şeridin başka bir yerinde sağ tık.
  - `Menu::hint`: satırın sağında soluk not (“sabit”, girdinin açıklaması).

Harf ipuçları (spesifikasyonun §1'i) bu ADR'nin kapsamında değildir; ayrı dilimdir.

## Sonuçlar

- Masaüstünde hızlı erişim çubuğu web'deki gibi değişir ve hatırlanır; bölünmüş düğme son seçimini gösterir.
- Envanterin bölünmüş düğmeleri anahtarlarını, girdilerin başlıklarını ve açıklamalarını taşır.

## Doğrulama

- **`cargo test -p kentos-desktop ribbon_plan`:** `ribbon.json`'un sözleri, sabit üçü, önerileri, çubuk menüleri, eklemeler ve çıkarmalar, sağ tık menüleri, bölünmüş düğme listeleri ve yüzleri.
- **`cargo test -p kentos-desktop ribbon_bar`:**
  - çubuğa ekleme ve çıkarma, yerleşimde saklanması;
  - sabit üçün değişmemesi ve envanterin yazdığıyla aynı olması;
  - seçimin anahtarıyla saklanması ve üstte görünmesi;
  - girdinin seçeneğiyle çalışması;
  - seçeneği almayan aracın uyarısı.
- **`cargo test -p kentos-desktop catalog`:** envanterden okunan bölünmüş düğmeler anahtarlarını, başlıklarını ve açıklamalarını taşır.
- **Resimler:** `cargo test -p kentos-desktop ribbon_bar::screens -- --ignored --nocapture` yedi sahneyi çizer: çubuğun ▾ menüsü; eklenecek, eklenmiş ve sabit komutta sağ tık; sekmede sağ tık; aracın yöntemleri; bir aile. Her biri 1440×900 ve 1100×650'de, koyu ve açık temada `.run/shots/serit-*`'a yazılır ve web'in `ribbon` sahneleriyle karşılaştırılmıştır.
- **Temiz denetimler:** `cargo clippy -p kentos-ui -p kentos-ui-showcase -p kentos-desktop --all-targets -- -D warnings`; `pnpm inventory:check`.
