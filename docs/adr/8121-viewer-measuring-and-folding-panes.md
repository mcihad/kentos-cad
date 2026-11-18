# ADR 8121: Görüntüleyicide ölçme ve açılıp kapanan bölmeler

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12 (görüntüleyici),
  [bim-plan.md](../../plan/bim-plan.md) BIM-06. Sahibin isteği: ölçme özellikleri eklensin. Sahne fazla
  daraltılmasın, estetik ve şık olsun. Ölçme araçları ve paneller açılıp kapanabilsin.
- **Bağlam:** ADR [8073](8073-ifc-viewer-first-slice.md), [8118](8118-ifc-viewer-layout-refinement.md),
  [8119](8119-building-3d-in-the-model-viewer.md), [8120](8120-3d-render-quality.md); kılavuz §5 (eksen adları), §23
  (ölçü, gösterim ve tessellation); DESIGN.md §7.21.
- **Web, `.kcad`, komut:** değişmedi. Ölçme bir görünümdür; çizimi değiştirmez, ürün komutu değildir.

## §1. Sorun

Görüntüleyicide bir uzunluğu, açıyı ya da alanı görmek için Özellikler'deki miktar setine bakmak gerekiyordu. İki nesne
arasındaki uzaklık, bir köşenin açısı, bir duvar yüzünün alanı okunamıyordu. İki yan bölme (600 px) sahneyi
daraltıyordu; dar pencerede ya da ölçerken bölmeler kapatılamıyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Autodesk Viewer, [Measure models](https://autodesk.com/learn/ondemand/tutorial/measure-models); APS [MeasureExtension](https://aps.autodesk.com/en/docs/viewer/v7/reference/Extensions/MeasureExtension) | Uzaklık (noktadan noktaya), açı (iki çizgi arası), alan; ölçerken noktalar yakalama seçeneklerine göre yakalanır | Uzaklık, Açı, Alan; köşe, kenar ortası, kenar, yüz yakalaması |
| xeokit, [DistanceMeasurementsPlugin](https://xeokit.github.io/xeokit-sdk/docs/class/src/plugins/DistanceMeasurementsPlugin/DistanceMeasurementsPlugin.js~DistanceMeasurementsPlugin.html) ve [yakalama](https://xeokit.io/blog/accurate-measurements-with-snapping) | Her ölçü doğrudan bir tel ve X, Y, Z eksenleri boyunca teller çizer; imleç en yakın köşeye ya da kenara yakalanır | Uzunluğun yanında eksen bileşenleri (ΔY, ΔX, ΔZ); eğik çizgide bileşenler kesikli |
| Bonsai, [Exploring an IFC model](https://docs.bonsaibim.org/studio/quickstart/explore_model.html) | Proje ağacı ve nesne bilgisi panelleri; arama; kata göre gizleme | Bölmeler sahnenin yanında kalır, istenince bir kenar şeridine katlanır |
| KentOS kılavuzu §5, §23.3 | Kodda x doğu, y kuzey; arayüzde Y sağa, X yukarı. Tessellation nihai ölçü değildir; gösterim kaynağı değiştirmez | Bileşenler projenin eksen adlarıyla; ölçü kaynağın f64 köşelerinden; kartta “gösterilen gövdeden, eğri yüzde üçgenlerden” |

## §3. Karar

- **K258 — Ölç:** şeridin sonunda (Kesit ▾'nin ardından) “Ölç” anahtarı (cetvel simgesi). Açıkken sahnedeki tıklama
  nesne seçmez, nokta verir; kapanınca tıklama yeniden seçer, ölçüler sahnede kalır.
- **K259 — Araçlar:** Uzaklık iki noktayla ölçülür: uzunluk, eksen bileşenleri (Δ doğu, Δ kuzey, ΔZ; projenin eksen
  adlarıyla) ve yükselip alçalan çizgide yatay uzunluk. Açı üç noktayla ölçülür; değer ortadakindedir (0–180°, atan2
  ile). Alan bir tıklamayla ölçülür: tıklanan üçgenin düzlemindeki, kenarlarla ona bağlı üçgenlerin bütünü (normali
  0,1°, köşeleri 1 mm içinde). Alanı f64 üçgen alanlarının toplamıdır; dış çizgisi çizilir.
- **K260 — Yakalama:** imlecin üstündeki gövdede önce 10 px içindeki köşe gelir, sonra kenar ortası, sonra kenarın
  ışına en yakın noktası, yoksa yüzdeki nokta. Yalnız gövdenin kenarları sayılır (iki yüzün en az 25° döndüğü yer,
  çizicinin kenar çizgisiyle aynı kural): bir yüzün üçgen köşegenine yakalanılmaz. İmleç gezinirken yakalanan yer
  simgesiyle (köşe kare, kenar ortası üçgen, kenar eşkenar dörtgen, yüz halka) ve adıyla gösterilir; son noktadan
  imlece kesikli bir çizgi ve uzaklıkta anlık uzunluk çizilir. Kesitin kestiği yere nokta düşmez.
- **K261 — Kesinlik:** noktalar kaynağın f64 koordinatlarıdır; GPU'nun float32'si değildir. Köşeye yakalanan nokta
  dosyadaki köşenin kendisidir. Eğri yüz üçgenleriyle ölçülür (çember yay değil kirişlerdir). Kart bunu söyler:
  “Gösterilen gövdeden; eğri yüzde üçgenlerden.” Dosyanın miktarları Özellikler'de kalır. Gösterim projenin basamaklarıyla
  yazılır (`Host::measure`), açı iki basamakla.
- **K262 — Sahnede:** ölçü çizgisi vurgu renginde ve beyaz kenarlıdır. Bileşenler, dış çizgiler ve imlece giden çizgi
  ince ve kesiklidir. Noktalar beyaz halkalı noktadır. Değerler alanın renginde plakadadır (uzunluk çizginin ortasında,
  açı açının içinde, alan yüzün ortasında). Plakalar sahnenin içinde tutulur.
- **K263 — Kart:** ölçerken ya da ölçü varken sahnenin sağ altında, kesit kartının ve lejantın çerçevesinde durur (330 px).
  Başlıkta “Ölçü” ve sayısı, araçların parçalı seçicisi (Uzaklık | Açı | Alan) ve katlama düğmesi vardır. Altında
  sıradaki tıklamanın ne yaptığı ve yarım ölçüde “Vazgeç” yer alır. Ölçüler listelenir: türü, değeri eş aralıklı, alt
  satırında bileşenleri ya da alanın gövdesi, × ile silinir. Liste 220 px'ten sonra kayar. En altta not ve “Temizle”
  vardır. Katlanınca yalnız başlık kalır, ölçüler sahnede kalır.
- **K264 — Açılıp kapanan bölmeler:** ağacın sekmelerinin yanında “‹”, Özellikler başlığının yanında “›” düğmesi bölmeyi
  34 px'lik bir kenar şeridine katlar; şeritteki düğme geri açar. Sahne genişler, kamera yerinde kalır. Katlı bölmenin
  tutamağı yoktur. Bölmelerin durumu görüntüleyicinindir (pencere kapanınca açık başlar).
- **K265 — Yeni renk ve yazı yok:** vurgu, alan, kenarlık ve metin jetonları; kitin parçalı seçicisi, düğmeleri ve
  lejant çerçevesi (kılavuz §8).

## §4. Uygulama

- `crates/shared/model-view/src/measure.rs` (saf, wasm32): `edges`, `snap`, `nearest_on_segment`, `Distance`,
  `angle`, `face`.
- `crates/model-view-ui`: `measuring.rs` (`Tool`, `Measured`, `Measuring`, `Viewer::{snapped, measure_click,
  measure_hover, marks}`); `viewer.rs` (iletiler `Measure`, `Tool`, `MeasureFold`, `MeasureCancel`, `MeasureRemove`,
  `MeasureClear`, `Pane(Side)`; ölçerken tıklama ölçer); `viewer_view.rs` (Ölç, kart, kenar şeritleri); `scene.rs`
  (`Gesture::Hover`, `Mark`, `Picked::{mesh, triangle}`, işaretlerin çizimi); `look::framed`'a sağ alt köşe.
- Masaüstü: izlerin denetimleri (`viewer_control`: Ölç, Uzaklık, Açı, Alan, Vazgeç, Temizle, bölmeler); 3B
  görünümün binası ve IFC modeli aynı bileşendir.

## §5. Sınamalar ve resimler

- Çekirdek (`measure`): kutunun 3 × 2 m yüzü 6 m², dört kenar, köşegeni dış çizgi değil; 0,25 × 2 m yan yüzü 0,5 m².
  Kutunun 12 kenarı var. Bileşenler (3, 4, 12) → yatay 5, uzunluk 13. Açı 90°, 180°'ye yakın açı atan2 ile, uçsuz açı
  yok. Işının parçaya en yakın noktası. Yakalama sırası: köşe, kenar ortası, kenar, yüz.
- Bileşen (`viewer`): üst yüzün köşelerine 3 cm uzaktan iki tıklama köşelere yakalanır ve tam 1 m ölçer, nesne seçmez.
  İmleç orada köşeyi gösterir. Alan üst yüzün bütünüdür (1 m², 4 kenar). Ölç kapanınca tıklama seçer, ölçüler kalır.
  Silme ve Temizle çalışır, bölmeler katlanıp açılır.
- Resimler: `.run/gui-evidence/model-view-ui/kesit-binasi-olcum-{acik,koyu}.png`, `kesit-binasi-paneller-kapali.png`;
  uygulamada `.run/gui-evidence/3b-kalite-8120/bina-olcum-acik.png`, `bina-paneller-kapali-acik.png`. İlk resimde sahne
  kaybolmuştu: vektörden kurulan satır çocuklarının genişliğini almıyordu (`Row::from_vec`), satıra genişlik verildi.

## §6. Sıradaki adımlar

- Çokgenle alan (yüzden bağımsız noktalar), iki yüz arasındaki dik uzaklık, kotun (Z) tek noktada okunması.
- Ölçülerin pencere kapanınca saklanması, BCF ya da paftaya aktarılması.
- Esc ile yarım ölçüden vazgeçmek (bugün kartın Vazgeç'i): pencerenin Esc'i kapatmaya bağlı.
- Gerçek pencerede kabul (`scripts/usage/real/`).
