# ADR 0115: Masaüstünde yerleşimin kalıcılığı ve web'in kabuk düzeni

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.4 (Yerleşim kapsamı); DESIGN.md §5.1; ADR 0058 (alt panel), 0114 (günlük).
- **Kaynak:** web'in `app/layoutPlan.ts`, `app/ribbon.ts`'in şerit parçaları, `app/state.ts` (`kentos.ui.v1`); ortak durumlar `fixtures/shell/v1/layout.json`.

## Bağlam

Web yerleşimi localStorage `kentos.ui.v1`'de tutar:

- sağ dokun görünüp görünmediği, genişliği;
- katman ağacının dok yüksekliğindeki payı;
- dokun önündeki sekme (Katmanlar ya da İşlemler);
- alt panelin açıklığı, yüksekliği ve sekmesi;
- İşlemler'in sekmesi ve katlanan kategorileri;
- şeridin sekmesi ve katlanması;
- klasik arayüzün araç kutusu, hızlı erişim çubuğu, bölünmüş düğmelerin son seçimi.

Boyutlar kullanıcının isteğidir: pencere daralınca daha az gösterilir, istek değişmez. Kurallar `layoutPlan.ts`'tedir.

Masaüstünde yerleşim yeniden açılışta hatırlanmıyordu. Varsayılanları da web'den ayrıydı:

- dok 320 birim (yazı boyuyla büyür, ~347 px);
- alt panel on iki satırlık geçmiş;
- alt panel ve komut satırı pencerenin bütün genişliğindeydi, sağ dokun altında da.

Web'de ve DESIGN.md §5.1'de ise yan paneller gövdenin bütün yüksekliğini kaplar; alt panel ve komut satırı yalnız çizim alanının altındadır.

## Karar

Kurallar `layout_plan.rs`'tedir. Bu, web'in `layoutPlan.ts`'inin ve şerit yardımcılarının (`quickAccessOf`, `startTab`, `splitCurrent`) karşılığıdır. İki platform `layout.json`'u oynatır (`layout_plan_tests.rs`).

Uygulamadaki yeri `layout.rs`'tir:

- **Dosya:** `~/.config/kentos-cad/yerlesim.json`, `ayarlar.json`'ın yanında. Web'in alanları, aynı adlarla tek JSON nesnesidir.
- **Okuma:** web'in kuralıyla. Türü ya da değeri tutmayan alan varsayılanını alır, bilinmeyen alan atılır. JSON nesnesi olmayan dosyanın her alanı varsayılandır. JavaScript'in sonsuz saydığı `1e999` gibi sayılar hiçbir alana alınmaz; serde_json bütün metni reddederdi, o yüzden okunmadan önce `null` yazılır.
- **Yazma:** son değişiklikten 250 ms sonra bütün alanlar birlikte, yanındaki geçici dosyadan adıyla değiştirilerek. Pencere kapanırken beklemeden yazılır. Yazılamayan dosya web'deki gibi sessizce geçilir.
- **Masaüstünde parçası olmayan alanlar** okunduğu gibi geri yazılır: araç kutusu, hızlı erişim, bölünmüş düğmeler, tema. Masaüstünün teması ayarlardadır (`appearance.theme`).

Yerleşimin durumları her iletiden sonra okunup tutulur:

- dok görünüyor mu (F4);
- önde hangi sekme;
- alt panel açık mı, sekmesi;
- İşlemler'in sekmesi ve katlananları;
- şeridin sekmesi ve katlanması.

Boyutlar ise yalnız sürüklendikleri yerde tutulur, ekrandakinden geri okunmaz. Pencere küçülünce gösterilen boyut küçülür, istek değişmez:

- **Dokun kenarı:** gösterilen boyuttan sürüklenir. Sonuç web'in kuralıyla (240–560 px ve pencerenin yarısı) gösterilir ve öyle tutulur.
- **Yığınlar arası tutamak:** katman payını 0,15–0,85 içinde tutar.
- **Alt panelin kenarı:** paneli 96 px ile pencere yüksekliğinin %60'ı arasında tutar. Sekme satırı panelin yüksekliğine sayılır, web'deki gibi.
- **Çift tıklama:** web'in ilk değerlerini geri getirir. KentOS UI yuvası kenara çift tıklamayı `Event::Reset` olarak, yığınlar arası tutamağa çift tıklamayı eşit paylar olarak bildirir. Dok 312 px'e, pay 0,5'e, alt panel 190 px'e döner.

Açılışta şeridin sekmesi, saklanan sekme varsa ve bağlamsal değilse odur, yoksa Giriş.

Kabuk web'in düzenine geçti: sağ dok gövdenin bütün yüksekliğini kaplar, alt panel ve komut satırı çizim alanının altındadır, durum çubuğu bütün genişliktedir.

## Sonuçlar

- Varsayılanlar web'inkilerdir: dok 312 px, alt panel 190 px, katman payı %50. Masaüstünde dok yazı boyuyla büyümeye devam eder. Saklanan genişlik, o anki yazı boyunda ekrandaki px'tir.
- Yuvanın yüzen pencereleri ve başka kenarlara taşınan paneller web'de yoktur; saklanmaz, oturumludur.
- Sonraki şerit dilimi, hızlı erişimi ve bölünmüş düğmelerin son seçimini bu dosyadaki alanlardan okuyacak (`quick_access_of`, `split_current` hazır).

## Doğrulama

- **`layout_plan_tests`:** `layout.json`'un bütün bölümleri:
  - alanlar, varsayılanlar, kurallar ve sınırlar;
  - okumalar (bozuk metin, yanlış tür, sınır dışı, sonsuz sayı, göç, bilinmeyen alan);
  - gösterilen boyutlar ve katman payı;
  - hızlı erişim, bölünmüş düğmeler, açılış sekmesi.
- **`layout_tests`:**
  - saklanan yerleşim açılışta gösterilir;
  - değişiklikler 250 ms sonra birlikte yazılır, bilinmeyen alan atılır, masaüstünün bilmediği alan kalır;
  - dar pencere daha azını gösterir, isteği değiştirmez;
  - sürüklenen kenar web'in kuralıyla tutulur, çift tıklama ilk değerleri getirir.
- **`bottom::tests`:** alt panelin sınırları.
- **Görüntüler:** `layout_tests::screens` (`.run/shots/yerlesim-*`), varsayılan ve saklanmış yerleşim; koyu ve açık tema, 1440×900 ve 1100×650; web'in `viewport-*` resimleriyle ve DESIGN.md §5.1 ile karşılaştırıldı.
