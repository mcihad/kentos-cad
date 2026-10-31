# ADR 0793: Model alanında kalem kalınlığı ve birlikte kopyalanan bağlar

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-07'nin dördüncü teslimi (ADV-07d) **[~]**.
- **Bağlam:** ADR 0776'nın açık noktası B8 (“model alanında kalem kalınlığı dünya biriminde; derin yakınlıkta 1 mm ayrıntı
  banda karışır”), ADR 0696 (yakınlaştırmaya göre duvar ayrıntısı, yoğun kıl çizgiler), ADR 0626 (birim, çizim ölçeği,
  gerçek boy yazılar), ADR 0157 (stilli çizimin karoları), ADR 0344 (etiket), ADR 0730 §4 (kopyada grup), ADR 0309 (pano),
  ADR 0249 §3 ve 0175 §4 (kopyanın kendi noktaları).
- **Web:** ertelendi (ADR 0228); satırı aşağıda.
- **Şema:** `.kcad` değişmez. WGSL düzen sözleşmesi `styled.layout.json` 8 → 9 (yapı aynı boyda; bir değerin anlamı).

## Araştırma (üreticilerin resmî belgeleri, 31 Ekim 2026'da okundu)

- **AutoCAD LT 2026, [Lineweight Settings Dialog Box](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-1BB43E62-DF93-494E-ACF2-55824ACD5130.htm):**
  model alanında kalınlık pikselle gösterilir ve yakınlaştırmayla değişmez; pafta alanında gerçek birimle, yakınlaştırmayla
  büyür; LWDISPLAY yalnız ekranı etkiler, çıktıyı değil.
- **Revit, [Line Weights](https://help.autodesk.com/cloudhelp/2015/ENU/Revit-Customize/files/GUID-35EF0EF0-9E1B-42E9-AB53-FC94F9AD8C97.htm)
  ve [Add View Scales for Model Line Weights](https://help.autodesk.com/cloudhelp/2022/ENU/Revit-Customize/files/GUID-402A7037-BE0C-4061-B4CF-598467BBF6D3.htm):**
  model çizgisinin kalemi görünüşün ölçeğine bağlıdır (1:100'de 0,5 mm, 1:50'de 0,7 mm); açıklama çizgileri ölçekten
  bağımsızdır. **[Thin Lines](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-DF7D7ED0-20F5-4CFD-A415-2023E732B09B.htm):**
  yakınlaşınca kalın çizgileri ince tutar, yazdırmayı etkilemez.

**KentOS kararı:** etiket nesnesini kalıcı kimlikle adlar (ADR 0344); nesnesiyle birlikte kopyalanan etiket kopyayı adlar,
tek başına kopyalanan etiket aynı nesnenin ikinci etiketidir. Kalem kâğıtta milimetredir ve çizim ölçeğinde gösterilir (Revit'in görünüş ölçeği gibi); ama ekran çizim
ölçeğinden derine yakınlaşınca kalem kâğıttaki genişliğini korur, dünyayla şişmez (AutoCAD'in model alanı gibi, bu sınırın
ötesinde). Uzaklaşınca dünyayla incelir, en az bir cihaz pikseli kalır (eski kural). Kalınlık kapalıyken (Kıl çizgi) her
çizgi bir pikseldir (aynen). Pafta, pafta haritası ve PDF zaten kâğıttır, değişmez.

## Karar

### 1. Model alanında kalem kalınlığı

Önce: kâğıtta w mm'lik kalem dünyada w × ölçek / 1000 birim genişliğinde çiziliyordu; 1:100'de 0,25 mm kalem 25 mm idi ve
0,1 mm/piksel yakınlıkta 250 piksel genişliğinde banttı; 1 mm aralıklı ayrıntılar ve duvarın katmanları banda karışıyordu.

- **Kural.** Bir çizginin (stroke) ve taramanın (hatch) dünya biriminde verilmiş genişliği model ekranında
  `min(pxPerM, tutulan × dpr)` cihaz pikseli/birimle çizilir; `tutulan = 1 / (METRES_PER_PX × çizim ölçeği)` mantıksal piksel
  /birim, yani ekran çizim ölçeğindeyken kalemin kâğıttaki genişliği. 1:100'de 0,25 mm 0,94 px, 0,5 mm 1,89 px, 1 mm 3,78 px
  kalır; 1:500 yakınlıkta beşte birine iner (en az 1 px).
- **Yalnız kâğıt kalemleri.** Sembolün yere ölçülen genişliği (birimi `m`: bir yolun 10 m'lik çizgisi) tutulmaz; dünyayla
  büyür. Stil çekirdeği (`kentos_style_core`) çizgi ve taramanın `ground`'unu bilir; stilin JSON'u değişmez (web'in donmuş
  fikstürleri aynı kalır), yere ölçülen genişlik kendi topluluğunda tutulur ve `Batches::ground` ile listelenir
  (`batch.rs`; `StrokeStyle::ground`, `FillPaint::ground`). Piksel birimli genişlik (Semboller › Ekranda sabit) ve sıfır
  genişlik (kıl çizgi) zaten tutulmaz.
- **Taşıma.** Yerli toplulukta `StyledBatch::weight_cap` (`decode`, `DecodeOptions::hold_weights`); stil bloğunun boş
  duran `origin.z`'sine yazılır (`uniform.rs`), WGSL'de `weightK()` (`common.wgsl`) çizgi ve taramanın yarı genişliğini onunla
  hesaplar (`stroke.wgsl`, `fill.wgsl`; taramanın uzak görünüşteki kâğıt örtüsü de). Kesik uzunlukları ve bulanıklık
  dünyada kalır. CPU ikizi (`cpu.rs`) satır satır aynı. Servis karolarının çerçeve başına yazılan `origin`'i `z`'yi korur.
- **Kim tutar.** `kentos_plot::layers::Look::hold_weights`: masaüstünün çizim alanı (`viewport.rs`) açık; paftanın
  haritası (`sheets.rs`), başsız pafta (`maps.rs`), önizlemeler ve resimler kapalı. Tutulan değer çizim ölçeğinden geldiği için
  ölçek değişince katmanlar zaten yeniden kurulur; yakınlaştırma yeniden kurmaz, kare başına tek `min`.
- **Sözleşme.** `styled.layout.json` sürüm 9: yapı ve bağlamalar aynı (çerçeve 48, stil bloğu 160 bayt); `origin.z`'nin
  anlamı ve `weightK` yazıldı. Web `origin.z = 0` yazar: davranışı eskisi gibidir.

### 2. Birlikte kopyalanan bağlar

Önce: Kopyala, Dizi ve Yapıştır etiketin `of`'unu özgün nesnede bırakıyordu; odanın kopyası etiketlerin kopyalarını
özgün odaya bağlıyor, oda adı özgünün üstünde iki kez çıkıyor, kopya oda etiketsiz kalıyordu.

- **Kural** (`kentos_domain::copy_links`, `Document::add_room_copies` içinde, ADR 0730 §4'ün mahal kaynakları ve açıklık
  konağı gibi): kopyalanan nesnenin, aynı kopyada (dizide aynı öğede) kopyalanan bir nesneye bağı o nesnenin kopyasına
  geçer; kopyalanmayan nesneye bağ olduğu gibi kalır.
  - **Etiket** (`of`): Kopyala, Dizi, Yapıştır.
  - **Ölçünün bağları** (`refs`), **koordinat, alan ve öznitelik tablosunun nesneleri**: Yapıştır (pano bunları taşır,
    ADR 0309). Kopyala ve Dizi'de ölçünün noktaları kendinindir (ADR 0249 §3), değişmez.
  - **Bağlı yazı ve ilişkili tarama:** Kopyala ve Dizi bağı düşürür (ADR 0175 §4), Yapıştır da düşürür (ADR 0309 §6);
    değişmedi. Kural yine de onları kopyaya bağlayabilir, bağ düşmemişse.
  - **Görünüm işareti:** hedefi nesne değil adlı görünüm ya da paftanın harita çerçevesidir (ADR 0345); kopyada aynı hedefi
    adlar, değişmedi.
- Komutların girdisi, çıktısı ve kataloğu değişmez; Python ve MCP aynı işleyicilerden geçer.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/copy_links_cases.py` (KentOS kodu olmadan; kopyalar sırayla sonraki yuvaları alır,
  dizi öğe öğe; bağ kuralı yukarıdaki gibi): `cad.entities.transform.links.json` 21, `cad.entities.array.links.json` 24,
  `cad.entities.paste.links.json` 24 durum; her biri yazar, alanları denetler, geri alınır. Masaüstünde
  `KENTOS_CASES_ONLY=links cargo test -p kentos-native-application --test all every_command_case` geçer; bir beklenen
  `of` bilerek bozulunca düşer (koşucu dosyaları okuyor). Komutların eski fikstürleri (transform, array, paste, copy, tags,
  arrays, dimensionsets, spaces, openings) aynen geçer.
- **Gerçek GPU** (`KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu paper_widths`, Apple M5/Metal): 1:100'ün 1 mm
  kalemi çizim ölçeğinde 3,78 px, 10 ve 265 kat derinde yine 3,78 px, 1:500'de 0,76 → 1 px; tutulmayan topluluk 10 kat
  derinde 37,8 px; çizgi ve tarama, 1× ve 4× MSAA, ±0,6 px. Eski `on_a_real_gpu` (kıl çizgiler, efektler) ve
  `styled_contract` (naga, sürüm 9) geçer.
- **Çekirdek ve çözücü:** `a_ground_width_keeps_a_batch_of_its_own_and_is_listed` (stil çekirdeği; JSON aynı, yalnız yerdeki
  listelenir), `weight_caps` (yerli; 1:1/20/100/500'de tutulan, kâğıtta 0, `m` ve `px` ve işaret 0). `kentos-style-core` ve
  `kentos-native-style`'ın bütün testleri (web'den donmuş `cases.json`, `batches.json` fikstürleri dahil) geçer.
- **Masaüstü:** `a_paper_width_is_held_where_the_millimetre_is_ten_pixels` (ADR 0776'nın 1 mm sahnesi: her kâğıt genişliği
  1:100'e tutulu, en genişi < 5 px, dünyada > 100 px olurdu), `a_copied_room_tags_its_own_copies` (etiketli oda Kopyala ile:
  her kopya etiket kendi nesnesinin kopyasını adlar, yazısı özgününkiyle aynı). `quality_scenes`, `style::scene`,
  `split_wall`, `sheets` filtreleri geçer.
- **Resimler** (`KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=kalite-ayrinti-1mm,kalite-ayrinti-1mm-ince,kalite-ayrinti-1-5,kalite-ayrinti-1-1,kalite-1-20,kalite-bilesen-1-20,kalite-etiket-kopya cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture`),
  1440×900 ve 1100×650, koyu ve açık: `.run/shots/arac-kalite-*`. 0,1 mm/piksel'de iki çizgi on piksel aralıkla ayrı; 1:5 ve
  1:1'de duvarın yüzleri ince; 1:20'de bileşenlerin taraması ve sınırları ayrı; kopya oda kendi etiketleriyle.
- **Koşulmayan:** Python'un ortak durum koşucusu (`pnpm py:test`, yerli modülü maturin'le yeniden derlemek gerekir; katalog ve
  SDK değişmedi, `sdk.py --check` ve `manifest.py --check` geçer); MCP ayrıca koşulmadı (aynı işleyici). Web'in WebGL2 ikizi
  donmuş. Süre ölçülmedi: kare başına bir `min`, kurulum değişmedi.

## Açık

- Kalem kalınlığının ekrandaki tavanı çizim ölçeğine bağlı; görünüş başına kalem seti (ADR 0783) model ekranında henüz bu
  tavanı görünüşün ölçeğinden almıyor.
- Web'in WebGL2 ve WebGPU yolu `origin.z`'yi yazmıyor (eski davranış).

## Belgelere eklenecek satırlar

**CLAUDE.md §0 (ADV-07 maddesinin altına ya da yeni madde):**

- **Model alanında kalem kalınlığı ve birlikte kopyalanan bağlar (31 Ekim 2026; [ADR 0793](docs/adr/0793-model-pen-widths-and-copied-links.md), ADV-07 [~]).**
  Çizim alanında kâğıt kalemi (çizgi ve tarama genişliği, mm) çizim ölçeğinden derine yakınlaşınca kâğıttaki genişliğini
  korur, dünyayla şişmez; uzaklaşınca incelir (en az 1 px): stil bloğunun `origin.z`'si, WGSL `weightK`, düzen sözleşmesi 9;
  `Look::hold_weights` yalnız çizim alanında; sembolün yere ölçülen genişliği (`m`) tutulmaz (`Batches::ground`, stilin
  JSON'u aynı). Kopyala, Dizi ve Yapıştır birlikte kopyalanan nesneye bağı kopyaya geçirir (`kentos_domain::copy_links`):
  etiket her üçünde, ölçünün bağları ve tablonun nesneleri Yapıştır'da. 21 + 24 + 24 ortak durum (`copy_links_cases.py`),
  gerçek GPU testi `paper_widths`, resimler `arac-kalite-*`, `arac-kalite-etiket-kopya`.

**CLAUDE.md §2 (komutlar):**

```bash
python3 scripts/fixtures/copy_links_cases.py --check; KENTOS_CASES_ONLY=links cargo test -p kentos-native-application --test all every_command_case; KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu paper_widths; cargo test -p kentos-desktop --lib quality_scenes   # model alanında kalem kalınlığının tavanı ve birlikte kopyalanan bağlar (ADR 0793)
```

**plan/architecture-advanced.md, ADV-07 durumu:** “Açık: model alanı kalem kalınlığı” kalkar; yerine “Model alanında kalem
kâğıttaki genişliğinde tutulur ve kopyalanan etiket kopyayı adlar ([ADR 0793](../docs/adr/0793-model-pen-widths-and-copied-links.md)).”
ADR 0776'nın B8'i kapandı.

**TODOS.md §0.2 (Web'e ertelenenler):** ADR 0793: web'in WebGPU ve WebGL2 stilli çizicisi stil bloğunun `origin.z`'sine
modelde `1 / (METRES_PER_PX × çizim ölçeği)` yazar ve `weightK`'yı (GLSL ikizinde de) uygular; web'in stil çekirdeği
çağrısı `Batches.ground`'u okur; `add_room_copies`'in bağ kuralı web'in kopya, dizi ve yapıştırmasına.
