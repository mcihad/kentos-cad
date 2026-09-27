# ADR 0091: Masaüstünde Katman stili

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** docs/STYLE.md §4; CLAUDE.md §4.4, §4.8, §4.10; DESIGN.md; ADR 0090 (masaüstünde stilli çizim), 0100 (ifade dili)
- **Sahibin yönü (27 Eylül):**
  - web'in stil pencereleri masaüstüne en az bire bir gelir, olabildiğince iyileşerek;
  - iyileşmeler web'e de gelir;
  - görsellik en önemli işlerdendir.

## Bağlam

- **Web:** katmanın işleyicisini `LayerStyleDialog` düzenler (`ui/style/`):
  - Basit, Tek sembol, Kategorili, Aralıklı ve Kurallar;
  - sınıflar veriden yapılır, sonra düzenlenir (`style/classify.ts`, `fixtures/style/v1/classify.json`);
  - Uygula ve Tamam katmanın stilini tek geri alma adımında yazar.
- **Masaüstü:** Katmanlar menüsündeki “Katman stili…” sönüktü; `style.layerStyle` web'e yönlendiriyordu.
- **Web penceresinin kusurları:**
  - alan önerileri `"Tapu alanı"` diye yazılıyordu; dil bunu metin okur, alan `[Tapu alanı]`'dır. Kuralların örneği (`"Nitelik" = 'Arsa'`) ve Aralıklı'nın yer tutucusu (`$alan, "Kat"`) de öyleydi;
  - ifade hataları yerini söylemiyordu;
  - Nesne sütunu çizilecek olanı söylemiyordu:
    - aynı değerli iki kategori ikisi de sayılıyordu (çekirdek yalnız ilkini çizer);
    - yazılar da sayılıyordu (stil motoru yazıyı çizmez);
    - alt kural bütün katmanda sayılıyordu, üstünün aldıkları arasında değil;
    - değilse kuralı hiç sayılmıyordu;
  - boş sembol yuvası boş kalıyordu, oysa sınıf katmanın düz görünüşüyle çizilir.

## Karar

### Hesap: `kentos-native-style`

- **`classify`**: `style/classify.ts`'in karşılığı. Kapsadıkları:
  - değerler ve farklı değerler (sayılar değerine, yazılar Türkçe sırasına göre, `localeCompare('tr', { numeric: true })`);
  - eşit aralık, eşit sayı;
  - rampalar, düz semboller;
  - etiketler (`Math.round`);
  - sınıf sayısı alanı (`Number()`).

  `classify.json`'u geçer (`tests/classify.rs`).
- **`renderer`**: işleyicinin JSON'u tipli yapılar olarak.
  - Bu sürümün bilmediği alanlar yerinde kalır.
  - Okunamayan bir işleyici (bilinmeyen tür) olduğu gibi korunur; pencere bunu söyler, başka tür seçilip uygulanmadıkça değişmez.
- **`preview`**: bir sembolün örnek nesnesi, `symbolPreview.ts`'in örneği. Kâğıtta 36 × 22 mm; nokta sembolü erişimine göre sığdırılır.
- **`tally`**: her kategorinin, sınıfın ve kuralın çizimde alacağı nesne, çekirdeğin çizdiği gibi (`resolve.rs`):
  - yalnız stil motorunun çizdiği nesneler sayılır;
  - nesne değerinin ilk kategorisine gider; değeri olmayanın değeri boş metindir;
  - sayı onu alan ilk sınıfa gider;
  - kural üstünün aldıkları arasından sayar; değilse kuralı açık ve koşulu okunan hiçbir kardeşin almadıklarını alır;
  - kapalı kural sayısını yine söyler;
  - okunamayan koşul yerini söyleyen hatayla döner.

  Kurallar elle yazılan `fixtures/style/v1/tally.json`'dadır; iki platform onu denetler (`tests/tally.rs`, `ui/style/tally.test.ts`).

### Pencere: `apps/desktop/src/style/layer_style/`

- Web'in penceresi, aynı sözler ve aynı sırayla (`mod.rs` durum ve düzenlemeler, `panels.rs`, `rules.rs`, `widgets.rs`):
  - her türün kendi taslağı vardır; türler arasında gidip gelmek bir şey kaybettirmez;
  - Uygula ve Tamam “Katman stili” ya da “Basit katman stili” adımını yazar; değişmemiş stil adım yazmaz;
  - pencere en çok 980 × 760'tır; gövde kayar, alt satır hep görünür.
- **Açılış:**
  - `style.layerStyle` etkin katmanı açar; grup etkinse komut kapalıdır;
  - Katmanlar menüsü o katmanı açar;
  - Esc, Vazgeç ve arka plan kapatır.
- **Sembol resimleri** (`style/thumbs.rs`): web Canvas2D ile çiziyor. Masaüstünde resmi çizimin stilli boru hatları ve atlası çizer:
  - her resim kendi görünümüdür;
  - dört örnekli çizilir;
  - aynı sembol, örnek, boy, palet ve kitaplık sürümü için bir kez kurulur;
  - ekrandan çıkan resmin GPU arabellekleri iki kare sonra bırakılır.

  Böylece resim haritanın çizdiğinin kendisidir.
- **Henüz olmayanlar:** yuva menüsünün “Kitaplıktan seç…”, “Düzenle…” ve “Kitaplığıma kaydet” öğeleri sönüktür ve “Web'de var; masaüstüne henüz taşınmadı” der. Stil yöneticisi ve sembol tasarımcısı gelince açılırlar.

### İyileşmeler (iki platformda)

- **İfade alanı:** Alanlar (her alanın kaç nesnede olduğuyla), Değişkenler ve İşlevler menüleri. Alan `fieldToken`'la yazılır (tek parça değilse `[…]`). Kuralların koşulunda üçü tek bir ƒ menüsündedir.
- **Hatalar** yeriyle yazılır: “8. karakterde: …”.
- **Nesne sütunu** `tally`'nin söylediğidir:
  - aynı değerin sonraki kategorisi uyarıyla işaretlenir: “yalnız ilki çizer”;
  - Aralıklı'da hiçbir sınıfın almadıkları ayrı satırda sayılır: “çizilmez”;
  - değilse kuralı da sayısını söyler.
- **Düz görünüş:** boş yuvanın resmi katmanın o sınıftaki düz görünüşüdür. Basit, katmanın şimdiki görünüşünü resimleriyle gösterir.
- **Metinler:** kuralların örneği ve yer tutucular dilin yazılışıyla: `Nitelik = 'Arsa' ve $alan > 500`, `$alan, Kat`.
- **Kapatırken soru:** uygulanmamış değişiklikle Esc, × ya da Vazgeç sembol tasarımcısındaki gibi sorar (`askUnsaved`, DESIGN.md §7.9.1): “Uygulamadan kapat”, Vazgeç, “Uygula ve kapat”. Soru açıkken Esc “kal” demektir.
- **Sınır alanları:** sınıfın alt ve üst sınırı etiketler gibi iki basamağa yuvarlanmış gösterilir; sınıf değeri yazılana dek tam tutar.
- **Klavye:** bir alan klavyeyi tutmuyorken ← ve → işleyiciyi değiştirir (web'in parçalı seçimi gibi); soru açıkken değiştirmez.
- **Yuva adı:** “Basit görünüş” resmin biraz dışına taşarak tam yazılır (en çok 72 px).
- **Masaüstünde alanlar** her tuşta etkilidir. Sayılar ve hata yazarken güncellenir; web'de Enter ya da odak çıkınca. Değerler ifadeye göre önbellekte olduğundan yazmak yavaşlamaz.

### Çekirdekte kapalı kategori (web ajanıyla anlaşıldı)

- Kapalı kategorinin nesnesi “Diğer değerler”e düşüyordu: `find(|c| c.enabled && c.value == v)`. Pencerenin sayısı ise onu kategoriye veriyordu.
- QGIS'te kapalı kategori çizilmez. Yeni kural (`style-core` `resolve.rs`):
  - değerin ilk kategorisi alınır;
  - kategori kapalıysa nesne çizilmez;
  - `other` yalnız hiçbir kategorinin tutmadığı değere gider.
- Sabitleyenler:
  - çekirdeğin birim sınaması (`style/tests.rs`);
  - `batches.json`'ın yeni durumu `categorized-off`: Arsa kapalı, parseli çizilmez, diğer değerlere de düşmez. Web ve masaüstü geçer.
- `cases.json`'un rastgele katmanlarında kapalı kategoriye düşen nesne yoktu: yanıtları değişmedi, yeniden kaydedilmedi.
- Lejant kapalı kategoriyi zaten yazmıyordu (`legendOf`); `legend.json` değişmedi.

## Sonuçlar

- **Denetimler:**
  - `classify.json`, `tally.json`, `batches.json` iki platformda geçer;
  - masaüstü davranışı `layer_style/tests.rs`'te sınanır (10 durum): açılış, taslaklar, sınıflar, kurallar, geri alma, bilinmeyen işleyici, komut, kapatırken soru, oklar.
- **Resimler:** `cargo test -p kentos-desktop style::screens::layer_style_screens -- --ignored` masaüstünü, `node apps/web/scripts/style/layer-style-shots.mjs` web'i çeker. Masaüstünde altı durum (kapatırken soru dahil), iki boy, iki tema, büyük yazıyla bir boy daha; web'de beş durum. Web ajanının 18 sahnelik başvuru resimleri ve pencerenin tanımı (tuşlar, durum, geri alma, bilinen kusurlar) karşılaştırmada kullanıldı.
- **Başarım:** değerler, sayılar ve kural sayıları belgenin sürümüyle önbellektedir. Resimler bir kez kurulur; çizimleri her karede GPU'da birkaç ek geçiştir.
