# ADR 0093: Masaüstünde Lejant

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** docs/STYLE.md §7; ADR 0090 (stilli çizim), 0091 (Katman stili), 0092 (Stil yöneticisi)
- **Sahibin yönü (27 Eylül):**
  - web'in stil pencereleri masaüstüne en az bire bir gelir, olabildiğince iyileşerek;
  - iyileşmeler web'e de gelir;
  - görsellik en önemli işlerdendir.

## Bağlam

- **Web:** `LegendDialog` (`ui/style/LegendDialog.ts`) çizimin sembollerinin anlamını katman katman listeler. Satırları `style/legend.ts` verir:
  - katmanın kendi görünüşü ya da işleyicisinin sınıfları;
  - yalnız katmanda olan geometriler;
  - nesnelerin kendi sembolleri.

  Katmanlar lejanttan bırakılabilir. “PNG olarak kaydet” lejantı temadan bağımsız, beyaz kâğıda siyah mürekkeple, iki kat boyda çizer.
- **Ortak durum:** `fixtures/style/v1/legend.json` (web ajanı kaydetti). Sabitledikleri:
  - okunan katmanlar;
  - her katmanın satırları;
  - resmin yerleşimi (`legendLayout`: mantıksal piksel, 2× çizilir);
  - pencerenin sözleri.
- **Web ajanının yaması** pencereyi `legendLayout`, `legendLayers` ve `LEGEND_TEXTS` ile yazdı. Böylece resmin yerleşimini de durum dosyası sabitler.
- **Masaüstü:** `style.legend` web'e yönlendiriyordu.
- **Web penceresinin kusurları:**
  - açılışta her satırın resmi hemen çiziliyordu; örnek çizimde bu 862 tuvaldir;
  - 543 satırdan uzun lejant tarayıcının tuval sınırını (2×'te 32 767 piksel) aşıyordu. Resim boş kalıyor, hiçbir şey kaydedilmiyor, pencere bir şey demiyordu;
  - resimdeki semboller ekranın piksel oranıyla çiziliyordu. Oran 1 olan ekranda 56 × 24 piksellik resim iki kat büyütülüp bulanıklaşıyordu. Bu `render/symbolPreview.ts`'tedir; web ajanına önerildi (`pixelRatio`).

## Karar

### Hesap: `kentos-native-style` `legend`

- `legend.ts`'in karşılığı: `legend_of`, `legend_layers`, `legend_layout`, sözler (`texts`) ve kâğıdın renkleri (`PAPER`).
- Katmanın satırları:
  - işleyicisi yoksa kendi görünüşü (`symbols_of_layer_style`);
  - tek sembol;
  - kategoriler: kapalı kategori girmez, boş etiketin yerine değer yazılır, Diğer değerler;
  - aralıklar;
  - kurallar: “üst › alt”, kapalı kural altlarıyla girmez;
  - nesnelerin kendi sembolleri: her sembol bir kez, kitaplıktaki adıyla, adı yoksa kimliğiyle.
- Yalnız katmanda olan geometriler yazılır; birden çoksa satır adı “(alan)”, “(çizgi)”, “(nokta)” alır. Satırı olmayan katman girmez.
- Bu sürümün okuyamadığı işleyici kendi satırı olmadan geçer (web'de hata verirdi); nesnelerin kendi sembolleri yine yazılır.
- `legend.json`'u geçer (`tests/legend.rs`); satırın adı değiştirilince sınama düşer.

### Pencere: `apps/desktop/src/style/legend/`

- Web'in penceresi, aynı sözlerle (`mod.rs`, `view.rs`):
  - üstte iki seçim;
  - her katman işaret kutusu, adı ve satır sayısıyla; satırları 56 × 32 resimleriyle;
  - bırakılan katmanın satırları soluk görünür (web'deki gibi);
  - altta satır sayısı ya da son söz, Kapat ve “PNG olarak kaydet”.
- Liste yalnız görünen satırları kurar (862 satırlık örnek çizimde bile).
- Pencere web'in boyundadır: 760 × 760, en çok uygulama penceresinin %88'i.
- Gruplar çizimin sürümü, kitaplığın sürümü ve “Yalnızca görünen katmanlar”la önbellektedir.
- `style.legend` açık çizimle açılır; Esc, Kapat ve arka plan kapatır.

### Resim: `sheet.rs`

- Yerleşim `legend_layout`'unkidir: başlık, çizimin adı, satırlar, kâğıdın renkleri. Metinler Arial'in ölçü ikizi Arimo ile yazılır (KentOS'la gelir).
- Semboller çizimin stilli boru hatlarıyla, kâğıdın renkleriyle çizilir; resim haritanın çizdiğinin kendisidir. Çerçeve web'deki gibi resmin üstüne çizilir.
- Resim pencerenin iş parçacığında çizilmez: KentOS UI'ın ekransız çizicisi kendi aygıtında çizer.
  - Uzun lejant 60 satırlık şeritlerle çizilir (her GPU'nun doku sınırında kalır); şeritler tek PNG'de birleşir.
  - Yer önce sorulur (`lejant.png` önerilir); PNG yazılınca pencere web'in sözünü söyler.
- Web'le aynı sınır: 2× boy 32 767 pikseli aşamaz (543 satır). Aşan lejant kaydedilmez; pencere satır sayısını ve sınırı söyler, katman bırakmayı ya da başlıkları kapatmayı önerir.

### İyileşmeler

- **İki platformda:**
  - PNG'ye sığmayacak lejant nedenini söyler;
  - satırların resimleri görünür oldukça çizilir (web `Thumbs`; masaüstünde sanal liste).
- **Masaüstünde:**
  - resimdeki semboller iki kat çözünürlükte çizilir, büyütülmez;
  - resim pencereyi dondurmaz.

## Sonuçlar

- **Denetimler:**
  - `legend.json` iki platformda geçer;
  - pencere kullanıcının sürdüğü gibi `legend/tests.rs`'te sınanır: açık çizim yokken, katman bırakma ve sayı, gizli katman, başlıklar, Esc, satırların sırası;
  - PNG'nin boyu ve kâğıdı sınanır (yazılım çizicisiyle);
  - PNG sınırı sınanır.
- **Resimler:** `KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-desktop style::screens::legend_screens -- --ignored --nocapture` çeker (`.run/shots/lejant-*.png`):
  - dört durum; iki boy, iki tema ve büyük yazıyla bir boy daha;
  - kaydedilen resim, başlıklı ve başlıksız (`lejant-resim*.png`).
