# ADR 0055: Masaüstünde çizimin yazıları, ölçüler ve yardımcı çizgiler

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.9, §5; DESIGN.md §8; TODOS.md `REN-12`, `UI-11`; ADR 0019 (wgpu çizim hattı), 0029 (seçim ve kenet deposu)
- **Sahibin yönü (26 Eylül):** masaüstü web ile aynı düzeye getirilir; görsellik en önemli şeylerden biridir.

## Bağlam

- Masaüstünün çizim alanı yazı çizmiyordu. Yazı nesneleri, ölçü değerleri, parsel numaraları ve nokta adları görünmüyordu.
- Ölçülerin çizgileri, sonsuz doğrular ve ışınlar da çizilmiyordu. Sahne onları "çizilmeyen" diye sayıyordu.
- Web bunları şöyle çizer:
  - yazıyı GPU sahnesinde değil, çizim alanının üstündeki Canvas2D katmanında çizer (`drawLabels`, `viewport/overlay.ts`);
  - hangi yazının nerede çizileceğini ortak Rust deposu söyler (`Store::labels`): görünür katman, görünümdeki kutu, ekrandaki boy, etiket stilinin ölçek aralığı ve en küçük nesne boyu;
  - yazılar projenin yazı tipiyle (`DrawingFont`) ve alan renginde bir haleyle çizilir; etiketler 8 piksellik hücrelerde birbirini örtmeyecek şekilde seyreltilir;
  - ölçünün çizgilerini ve görünüme kırpılmış yardımcı çizgileri sahnede, deponun `drawn` çıktısından çizer.
- Masaüstü aynı depoyu kullanır (ADR 0029), ama web'in varsayılan etiket kurallarını (`DEFAULT_LABELS`) depoya vermiyordu.
- Web'in yedi çizim yazı tipi WOFF2'dir (latin ve latin-ext alt kümeleri; üçü değişken). Masaüstünün yazı sistemi (cosmic-text) WOFF2 okumaz.

## Karar

### Çizim yazı tipleri

- `scripts/fonts/drawing_fonts.py`, web'in kendi WOFF2 dosyalarından TrueType yüzleri üretir (`apps/desktop/assets/fonts/drawing`):
  - her yüzün iki alt kümesi birleştirilir;
  - değişken yazı tipleri (Arimo, Overpass, Quicksand) çizimin istediği ağırlıklarda (400, 500, 600, 700) durağan örneklere çevrilir;
  - glifler ve genişlikleri web'inkilerdir; başka yerden bir şey alınmaz.
- 7 aile, 22 yüz, 1,9 MB; OFL lisansları yanlarında. Her yüz Türkçe harfleri ve `° Ø ² ³` işaretlerini kapsar.
- `--check` yüzleri bayt bayt karşılaştırmaz, çünkü fontTools sürümleri tabloları farklı dizer. Her yüzün adlarını, ağırlığını, biçemini, eşlediği karakterleri ve karakter başına genişliğini, web'in dosyalarından yeniden üretilenle karşılaştırır.
- Masaüstü yüzleri açılışta bir kez yükler (`drawing_fonts::load`, `App::start_with`).
- Eğik yüzü olmayan ailelerde tarayıcı eğiği kendisi üretir. Masaüstü bu ailelerde yazı nesnesini dik çizer.

### Yazılar (`apps/desktop/src/labels.rs`)

- Çizim alanının üstünde, işaretlerin altında bir Iced tuvali çizer.
- Yazıların nerede çizileceğini deponun tipli sorgusu verir (`Spatial::labels` → `LabelSpot`): ölçü değeri, yazı nesnesi, merkez, köşe, yan, boyunca.
- Çizim web'in `drawLabels`'ıdır:
  - ölçü değeri 500 ağırlıkla, taban çizgisinde ortalanır. Rengi nesnenin ya da katmanın rengidir; tema mürekkebiyse etiket rengidir;
  - yazı nesnesi eğik 400 ağırlıkla, taban çizgisinde soldan çizilir;
  - etiketler katmanın etiket stiliyle çizilir: boy, büyüme, en çok boy, ağırlık, şablon, mürekkep. Stili olmayan katmanda web'in `DEFAULT_LABELS`'ı geçerlidir. Masaüstünün kopyasını bir test web'in kaynağından okuyarak karşılaştırır;
  - hale 3 px'tir, alanın renginde.
- Taban çizgisi yazı sisteminin kendi ölçüsünden gelir: cosmic-text'in `line_y`'ı, 100 px'te ölçülüp boyla ölçeklenir.
- Dik yazı Iced'in glif önbelleğinden çizilir; hale dört yöne ve çaprazlara 1,5 px kaydırılmış sekiz kopyadır. Döndürülmüş yazı glif ana hatlarıdır: önce hale çizgisi, sonra dolgu.
- Etiketlerin yer tutması harf genişliklerinin toplamıyla (kerning olmadan) denetlenir. Görünümün sunduğu her etiketi şekillendirmek, sığanları çizmekten pahalıdır. Çizilen ve ortalanan yazı kesin ölçülür.
- Yazı nesneleri ve ölçü değerleri her zaman çizilir; seyreltilen yalnız etiketlerdir (web gibi).
- Masaüstünün depo kopyası web'in varsayılan etiket kurallarını da alır (`Store::set_label_defaults`, `Spatial::reload`).

### Sahne (`crates/render/wgpu`)

- Ölçü, çekirdeğin yerleşimindeki çizgilerdir: yardımcı çizgiler, ölçü çizgisi ve uçları (`layout_dimension`). Yerleşemeyen ölçü sayılır.
- Sonsuz doğru ve ışın beşinci bir parçadadır (`build_construction`). Web gibi kırpılır: görünüm ve çevresinde görünümün üç katı genişlikte bir kutuya (`clip_line`).
  - Görünüm kutudan çıkınca ya da çok yaklaşınca (kutu görünümün 20 katından büyükse) yalnız bu parça yeniden kurulur.
  - Seçim ve üzerine gelme vurgusu ölçüyü ve yardımcı çizgileri de gösterir; yardımcı çizgi aynı kutuya kırpılır.
- Yazı nesneleri sahnede değil, yazı katmanındadır.

## Bu dilimde olmayanlar

- Yazı aracı (`tool.text`) ve ölçülendirme aracı (`tool.dimension`); yazıyı çift tıkla düzenleme.
- Eğik yüzü olmayan ailelerde eğik yazı (tarayıcının ürettiği eğik).
- Yazının katman sırasına girmesi: yazılar her katmanın üstündedir. Web de öyledir.
- Etiketlerin kaydırmada yeniden yerleştirilmeden ötelenmesi: kare başına yeniden yerleştirilirler, web gibi.

## Ölçüm (26 Eylül 2026, Linux, `labels::perf`, release)

- Senaryo aşırıdır: 1 m arayla 50 176 adlı nokta, 4 px/m'de. Görünüm 34 721 etiket sunar, yaklaşık 2 500'ü sığar.
- Deponun sorgusu 1,9 ms, yerleştirme 14,1 ms, kare (çizim dahil) 40,8 ms.
- İlk yazımda kare 117 ms idi: ölçü önbelleğinin sınırı (20 000, web'deki gibi) görünümün sunduğundan küçüktü ve her çağrıda boşalıyordu. Sınır 200 000'e çıktı; yer tutma harf genişlikleriyle denetleniyor.
- Tipik paftada birkaç yüz etiket vardır.

## Doğrulama

- `labels::tests`:
  - etiketler birbirini örtmez; ekran dışındakiler yer tutmaz;
  - varsayılan etiket stilleri web'in `DEFAULT_LABELS`'ının aynısıdır (web kaynağından okunur);
  - örnek çizim yazılarını, ölçüsünü ve etiketlerini verir.
- `drawing_fonts::tests`: yedi aile yüklüdür.
- `crates/render/wgpu/tests/scene.rs`: gizli katman açılınca ölçü çizgileriyle çizilir ve "çizilmeyen" kalmaz; yardımcı çizgiler kutuya kırpılır.
- `python3 scripts/fonts/drawing_fonts.py --check`.
- Görüntüler (`labels::screens`, `.run/shots/yazi-*`): örnek çizim "Çizim" katmanı açık, koyu ve açık, 1440×900 ve 1100×650. Görünenler:
  - döndürülmüş "Çınar sokağı" (Barlow eğik);
  - "101/7" parsel numarası, "P1" nokta adı;
  - "23.417" ölçü değeri, ölçü çizgisi ve uçları;
  - görünüme kırpılmış sonsuz doğru ve ışın; yazının halesi üstünden geçen çizgiyi keser.
- `pnpm rust:test:desktop` (454), render ve etkileşim testleri, clippy geçti.
