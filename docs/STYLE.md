# Stil motoru

KentOS'un semboloji sistemidir: noktaların, çizgilerin ve alanların (ve alan
kenarlarının) nasıl çizileceğini tanımlar, saklar, paylaşır ve GPU'da çizer.
Hedef, **Mekânsal Planlar Yapım Yönetmeliği'nin** (MPYY, 2026 değişikliğiyle)
EK-1a, EK-1c, EK-1ç ve EK-1d gösterimlerinin tamamını birebir üretebilmektir.
Esin kaynakları QGIS sembol motoru (sembol katmanları, işleyiciler, stil
yöneticisi) ve MapLibre stil belirtimidir (ifadeli özellikler, desen
dokuları, GPU dostu yapı). Kod ile bu belge çelişirse önce kodu doğrulayın,
sonra belgeyi güncelleyin.

---

## 1. Araştırma özeti

### 1.1 MPYY gösterimleri ne istiyor

Kaynak: Çevre, Şehircilik ve İklim Değişikliği Bakanlığı, Mekânsal Planlama
Genel Müdürlüğü, "Mekânsal Planlar Yapım Yönetmeliği Değişikliği (22 Ocak
2026)" ekleri: EK-1a Ortak Gösterimler, EK-1c Çevre Düzeni Planı, EK-1ç Nazım
İmar Planı, EK-1d Uygulama İmar Planı, EK-1e Detay Katalogları. Tablolarda
her satır bir gösterimdir ve dört sütunu vardır: **çizgi tipi (sınır)**,
**sembol**, **tarama** ve **alan renk kodu (RGB)**. Bir gösterim birden çok
sütunu birlikte kullanabilir (ör. renkli alan + tarama + alan içi harf).

Gösterimlerden çıkan gereksinimler:

| Gösterim türü | Örnek (EK-1d) | Motorun karşılığı |
|---|---|---|
| Kesik ve nokta gruplu sınırlar | Köy sınırı: uzun çizgi, üç nokta, uzun çizgi; mahalle sınırı: dört nokta | Kesik deseni (`dash`) ve çizgi üzerinde işaret (`markerLine`, aralıklı nokta grupları) |
| Çizgi üzerinde dönüşümlü işaretler | Kentsel tasarım projesi sınırı: çizgi üzerinde sırayla içi boş ve dolu daireler | İki `markerLine`: aynı aralık, biri yarım aralık kaydırılmış |
| Çizgisiz işaret dizileri | Gecekondu önleme bölgesi: yalnızca artılar; toplu konut: ⊕ dizisi | `markerLine` tek başına (alttaki çizgi yok) |
| İç içe şekilli işaretler | İmar hakkı aktarım alanı: çift daire; ⊕ daire içinde artı | Birden çok katmanlı işaret sembolü |
| Çizgiyle dönen işaretler | Sahil şeridi: çizgi üzerinde eşkenar dörtgenler; ok uçları, tırnaklar | `rotate: true`, çizgiye göre açı |
| Alan rengi | Konut, ticaret, sanayi … her biri RGB kodlu | `simpleFill` |
| Tarama | Açılı paralel çizgiler, çapraz, noktalı, kesikli tarama | `hatchFill` (açı, aralık, kalınlık, kesik), `patternFill` |
| Alan içi harf ve kısaltma | T, MİA, EGB, "E =" | Alanın iç noktasında `text` işareti |
| Değişken metinli işaretler | Yapı düzeni dairesi: merkezde A-/B-/BL-, dört yanda kat adedi ve bahçe mesafeleri; TAKS/KAKS kesri; "Yençok = …m" | Metni **ifadeyle** öznitelikten gelen `text` işaret katmanları, konumları `offset` ile |
| Alan kenarı gösterimleri | Sınırın içine doğru tırnaklı ya da taralı bantlı alanlar | Alan sembolünde çizgi katmanları, içe/dışa kaydırma (`offset`) ve kenara dik işaretler |
| Özel sembol çizimleri | İbadet yeri, sağlık tesisi, trafo … | SVG işaretleri (kendi SVG editörümüzle çizilir), gerektiğinde raster |

Ölçü birimi kâğıt milimetresidir: gösterimler belirli bir plan ölçeğinde (1/1000,
1/5000 …) çizilmek üzere tasarlanmıştır.

### 1.2 QGIS'ten alınanlar

- **Sembol = sembol katmanları yığını.** İşaret (marker), çizgi (line) ve alan (fill) sembolleri; her biri alttan üste çizilen katmanlardan oluşur. Alan sembolü kenar için çizgi katmanları da taşır.
- **Katman türleri:** basit işaret, SVG işaret, yazı (font) işareti, raster işaret; basit çizgi, işaretli çizgi (aralık, köşe, uç, orta; çizgiye göre dönme), taralı çizgi; basit dolgu, çizgi deseni dolgusu, nokta deseni dolgusu, SVG ve raster dolgu, merkez noktası işareti.
- **Birimler:** milimetre, piksel, harita birimi.
- **Veriye bağlı özellikler:** her özellik bir ifadeyle değişebilir (boyut, açı, renk, metin).
- **İşleyiciler (renderer):** tek sembol, kategorili, aralıklı (derecelendirilmiş), kural tabanlı (iç içe kurallar, ölçek aralığı, "diğerleri").
- **Stil yöneticisi:** etiketli, gruplu kitaplık; içe/dışa aktarma (XML).

### 1.3 MapLibre'den alınanlar

- **İfadeli özellikler** ve ölçeğe göre değişim (`interpolate`, `step`); bizde ifade dili (`model/expression`) ve kurallardaki ölçek aralığı.
- **Desenler doku atlasından** (`fill-pattern`, `line-pattern`, sprite): SVG ve raster desenler bir kez rasterlanıp atlasa konur, GPU örnekler.
- **GPU dostu yapı:** her şey veri; çizim tarafında kaynaklar değişmedikçe yeniden kurulmaz; ölçekle değişen görünürlük kare başına tamponlar yeniden kurulmadan uygulanır.
- **Sürümlü, JSON stil belgesi:** içe/dışa aktarma ve paylaşmaya uygun.

KentOS'un farkı: bir CAD katmanı karışık geometri taşır (aynı katmanda nokta,
çizgi ve alan olabilir). Bu yüzden katman stili her geometri sınıfı için ayrı
bir sembol bağlar (**sembol takımı**: nokta, çizgi, alan).

---

## 2. Kavramlar

```
Kitaplık (StyleLibrary)
 ├─ Kategori ağacı: MPYY › Uygulama İmar Planı › Sınırlar › …
 ├─ Sembol (Symbol): tür (marker | line | fill) + sembol katmanları
 └─ Varlık (Asset): SVG çizimi ya da raster görüntü (işaret ve desenlerde kullanılır)

Katman stili (LayerRenderer) ─ nesneye hangi sembol takımının uygulanacağı
 ├─ Tek sembol
 ├─ Kategorili (bir ifadenin değerine göre)
 ├─ Aralıklı (sayısal aralıklara göre)
 └─ Kurallı (ifade süzgeci + ölçek aralığı, iç içe, "diğerleri")

Sembol takımı (SymbolSet) = { marker?: sembol, line?: sembol, fill?: sembol }
```

- **Öncelik:** nesnenin kendi sembolü (`entity.symbol`) → katmanın stili (işleyici) → katmanın basit görünümü (renk, çizgi tipi, kalınlık, dolgu). Basit görünüm, motorun ürettiği basit sembollere çevrilir; böylece her şey aynı çizim yolundan geçer.
- **Geometri sınıfları:** alan (kapalı alan, tarama), çizgi (çizgi, çoklu çizgi, yay, daire, elips, eğri, yardımcı çizgiler; CAD'deki gibi daire dolmaz), nokta. Yazı ve ölçü kendi yollarında kalır (etiket motoru ayrı bir adımdır).
- **Alan kenarı:** alan sembolündeki çizgi katmanları halkalara uygulanır; `offset > 0` kenarı alanın **içine**, `< 0` dışına kaydırır. `rings` ile yalnızca dış halka ya da delikler seçilebilir.

## 3. Sembol katmanları

Her katmanın ortak alanları: `enabled` (ifadeyle de olabilir), `opacity`, `unit`
(boyut birimi, aşağıda).

### 3.1 İşaret (marker) katmanları

| Tür | Özellikler |
|---|---|
| `shape` | `shape`: circle, square, rectangle, diamond, triangle (eşkenar, ağırlık merkezinde), pentagon, hexagon, octagon, star, cross (+), x, line (—), arrow, arrowhead, chevron (açık V), semicircle, quartercircle, ring; dışbükey şekillerin kalın çerçevesi sivri köşelidir; `size`, `width`/`height` (dikdörtgen), `fill`, `stroke`, `strokeWidth`, `rotation`, `offset`, `anchor` |
| `svg` | `asset` (kitaplıktaki SVG), `size`, `rotation`, `offset`, `anchor`; SVG içindeki `param(fill)`/`param(stroke)` renkleri sembolden verilir (tek SVG farklı renklerle kullanılır) |
| `text` | `text` (sabit ya da ifade: `Parsel`, `'E=' \|\| Emsal`), `font` (ui, serif, mono), `weight`, `italic`, `size`, `color`, `halo`, `rotation`, `offset`, `anchor` |
| `raster` | `asset` (PNG/JPEG), `size`, `rotation`, `offset`, `opacity` |

### 3.2 Çizgi katmanları

| Tür | Özellikler |
|---|---|
| `simpleLine` | `color`, `width`, `dash` (açık/kapalı uzunluklar dizisi), `dashOffset`, `cap` (butt, round, square), `join` (miter, round, bevel), `offset` (paralel kaydırma; çizgide sol artı, alan kenarında içe artı; ifadeyle de verilir: yol kenarı `varsayılan([Genişlik], 16) / 2 * 1000 / $ölçek` mm) |
| `markerLine` | `marker` (işaret sembolü), `placement` (interval, vertex, innerVertex, first, last, center, segmentCenter), `interval`, `offsetAlong` (ilk işaretin başa uzaklığı), `offset` (dik kaydırma, ifadeyle de), `rotate` (çizgiyle dönsün mü; dönen yazı ters okunacağı yerde yarım tur çevrilir, kutusu aynı yanda kalır) |

Tırnaklı (hashed) çizgi, `line` şekilli ve dönen bir `markerLine`'dır; ayrı tür gerekmez.

### 3.3 Alan (fill) katmanları

| Tür | Özellikler |
|---|---|
| `simpleFill` | `color` |
| `hatchFill` | `angle`, `spacing`, `width`, `color`, `offset` (desen kaydırma), `dash` (kesikli tarama). Çapraz tarama iki katmandır |
| `patternFill` | `marker` (işaret sembolü) ızgarada: `spacingX`, `spacingY`, `stagger` (kaydırmalı satırlar), `angle`, `offset` |
| `imageFill` | `asset` (SVG ya da raster), `tileSize`, `angle`, `opacity` |
| `centroidMarker` | `marker` alanın iç noktasına (`pointOnSurface`, varsayılan: ağırlık merkezi alanın içindeyse o, değilse yatay tarama çizgilerindeki en geniş iç aralığın ortası) ya da ağırlık merkezine konur |
| `simpleLine`, `markerLine` | Alanın kenarlarına uygulanır (§2). `rings: 'all' \| 'exterior' \| 'interior'` |

### 3.4 Birimler

| Birim | Anlamı | Zoomla |
|---|---|---|
| `mm` (varsayılan) | Kâğıt milimetresi, projenin çizim ölçeğinde: 1 mm = ölçek/1000 m (1:1000'de 1 m) | Kâğıttaki gibi büyür küçülür; çıktı ne ise ekranda o |
| `px` | Ekran pikseli | Sabit kalır (GIS görünümü) |
| `m` | Harita birimi (metre) | Kâğıttaki gibi |

Kurallar: geometriyi değiştiren uzunluklar (kaydırma, işaret aralığı, desen aralığı) `mm` ya da `m` olur; genişlik, işaret boyu ve kesik uzunlukları `px` de olabilir (gölgelendiricide ölçeklenir). Hiçbir çizgi 1 aygıt pikselinden ince çizilmez (kıl çizgi).

Semboller varsayılan olarak çizim ölçeğinde (kâğıt mm) durur, bu yüzden harita uzaklaştıkça küçülür, basılı pafta gibi. **Görünüm → Sembol boyutu → Ekranda sabit** (uygulama ayarı `prefs.symbolSize`) seçilirse mm ekran milimetresi olur ve semboller her yakınlıkta aynı boyda kalır. Çizilen boylar (çizgi kalınlığı, kesik uzunlukları, işaret ve tarama boyları) çekirdekte doğrudan ekran pikseline çevrilir (`Env.screen`, 96 px/inç), böylece yakınlaştırma sürerken de sabit kalırlar. Geometriyi yerleştiren uzunluklar (çizgi boyunca işaret aralığı, kaydırma) görünüm ölçeğiyle derlenir: ölçek dörtte bir oktavlık adımlara yuvarlanır ve yakınlaştırma durunca (150 ms) yeniden kurulur; o arada yalnız bu aralıklar görünümle ölçeklenir. Uzak görünümde gürültü olmasın diye şu kurallar vardır:
- Aralığı 3 pikselin altına inen tarama düz bir renk tonuna döner. Bu ton kıl çizgilerin görünüşünden kâğıttaki gerçek kaplama oranına (çizgi kalınlığı ÷ aralık) geçer, böylece ince tarama alanı kapkara yapmaz.
- Hücresi 4 pikselin altına inen desen ve deseni 4 pikselin altına inen kesik çizgi de ortalama bir tona döner.
- Ekranda 3 pikselden küçük kalan yazı çizilmez (`batchLegible`).

### 3.5 Renkler ve ifadeler

- Renk: `#RRGGBB`, `#RRGGBBAA` ya da tema jetonu (`ink` = CAD renk 7, `fg`, `fg-dim`).
- **Veriye bağlı özellik:** sayı, renk ve metin özellikleri sabit ya da `{ expr: '…', fallback }` olabilir. İfadeler `model/expression` dilidir ($alan, alanlar, işlevler); sembol başına bir kez derlenir, nesne başına değerlendirilir.

## 4. Katman stili (işleyiciler)

```ts
type LayerRenderer =
  | { type: 'single'; symbols: SymbolSet }
  | { type: 'categorized'; expr; categories: { value, label, symbols, enabled }[]; other?: SymbolSet }
  | { type: 'graduated'; expr; classes: { min, max, label, symbols }[] }
  | { type: 'rules'; rules: Rule[] }          // Rule: filter?, minScale?, maxScale?, symbols?, children?, isElse?
```

- Kategorili ve aralıklı işleyiciler tasarımcıda veriden üretilir (benzersiz değerler, eşit aralık ya da doğal kırılmalar); saklanırken yine veridir.
- **Ölçek aralığı** (1:N paydası) kuralın çizilip çizilmeyeceğini belirler; GPU toplulukları bu aralığı taşır, çizici her karede seçer, tampon yeniden kurulmaz.

## 5. Kitaplık

| Kaynak | Nerede | Kim değiştirir |
|---|---|---|
| **Sistem** | Uygulamayla gelir (`style/system/`) | Kimse; kopyalanabilir, silinemez, değiştirilemez |
| **Kullanıcı** | Bu kullanıcının kitaplığı (şimdilik tarayıcı, sonra bulut) | Kullanıcı |
| **Proje** | Proje dosyasında (`.kcad`), projeyi açan herkes görür | Projeyi düzenleyen |

- **Ağaç:** her öğenin bir kategori yolu vardır (`['MPYY', 'Uygulama İmar Planı', 'Sınırlar']`); kategori derinliği sınırsızdır. Boş kategoriler ve sıralama için kategori kayıtları ayrıca tutulur.
- **Kimlikler** kalıcıdır; semboller birbirine ve varlıklara kimlikle bağlanır. Sistem öğesi kopyalanınca yeni kimlik alır ("… (kopya)").
- **Dışa/içe aktarma:** `.kstil` dosyası: sürümlü JSON (`{ format: 'kentos-style', version, items, assets }`), kullandığı SVG/raster varlıkları gömülü. İçe alırken şema denetlenir, kimlik çakışmasında "üzerine yaz / kopya olarak al" sorulur.
- **Paylaşma:** dosya ve pano (JSON) bugün; sunucu gelince bağlantıyla paylaşma ve kurum kitaplıkları.

## 6. Çizim hattı

```
render/styledLayer.ts: katmanın programı (semboller, işleyici, kümeler, renkler) + nesne başına kip + ifade tablosu
   ─► style-core (Rust, geometri deposunun yanında tek çağrı): resolve (hangi sembol takımı)
        ─► compile (sembol × geometri → çizim ilkelleri) ─► batch (GPU toplulukları, katman başına)
   ─► render/styledBatches.ts (renkler, atlas görüntüleri) ─► RenderBackend (WebGL2 / WebGPU)
style/compile.ts compileSymbol (tek sembol, aynı çekirdek) ─► önizleme ve lejant (Canvas2D, aynı ilkeller)
```

Hesabın hepsi `crates/shared/style-core/src/style/`'dadır (ADR 0008 “Stil derleyicisi”): `resolve.rs`, `compile.rs`, `place.rs` (yerleşim), `batch.rs`, `build.rs` (katmanın çağrısı). TypeScript katmanın programını kurar ve çekirdeğin yanıtını gösterir.

- **Sembol seçimi (`render/styledLayer.ts` sayfanın kipini verir, çekirdek çözer):** nesnenin kendi sembolü (`entity.symbol`), yoksa katmanın işleyicisi (`style.renderer`), yoksa katmanın basit görünüşü (`symbolsOfLayerStyle`: renk, çizgi tipi, kalınlık, dolgu, nokta simgesi). Kurallar:
  - Hiçbir kurala ya da kategoriye uymayan nesne çizilmez (QGIS gibi).
  - Uyan takımda nesnenin geometri türüne sembol yoksa ya da başvurulan sembol kitaplıkta yoksa basit görünüş çizilir; nesne sessizce kaybolmaz.
  - Dolgu sembolü olmayan alan, takımın çizgi sembolüyle kenarından çizilir.
  - Tarama nesneleri kendi desenini (`hatchSymbolOf`), ölçüler ince çizgilerini korur; yazılar üst katmandadır.
- **Derleme (`style-core` `compile.rs`, `place.rs`)** saf ve testlidir: kesik ve kaydırma, işaretlerin çizgi boyunca yerleşimi, alanın iç noktası, halka yönleri float64 ile hesaplanır. Çıktı arka uçtan bağımsız ilkellerdir: vuruş (stroke), dolgu (düz, tarama, döşeme), işaret (şekil, SVG, yazı, raster). İfadeler katman kurulumu başına bir kez derlenir (program); sembol ifadeleri çizim ölçeğinin paydasını `$ölçek` ile okur. Bir çizgi ya da alan katmanının içindeki işaret sembolünün katmanları kendi alt seviyelerini alır (seviye + j/256): çizici aynı görünüşlü işaretleri bir CAD katmanındaki bütün nesnelerde tek topluda birleştirdiği için, bir nesnenin kâğıt dolgulu çerçevesi başka bir nesnenin çerçevesinin üstündeki işareti örtmez.
- **GPU toplulukları (`style-core` `batch.rs`; renk ve görüntüler `render/styledBatches.ts`):** ilkeller stil ve ölçek aralığı anahtarıyla toplanır; sembol düzeyine, sonra türe (dolgu < vuruş < işaret) göre sıralanır. Geometri orijine göre float32'dir; dolgular katman bitince tek çağrıda üçgenlenir.
  - **Vuruş:** her parça bir örnek (`ax, ay, bx, by, dist, uç bayrakları`); kapsül SDF'si, yuvarlak birleşim, uçta düz/yuvarlak/kare kapak. Kesik deseni (en çok 8 değer) gölgelendiricide yol boyu mesafeden hesaplanır; desen dönemi 4 px'in altına inince çizgi, desenin dolu oranıyla soluklaşan düz çizgiye döner (titreşim yok).
  - **Tarama:** üçgen başına ek geometri yok; çizgiler gölgelendiricide yerel orijine göre dünya koordinatından (ya da `px` biriminde ekrandan) üretilir, her yakınlıkta tam. Aralık 3 px'in altına inince taramanın ortalama rengine döner.
  - **Döşeme (desen ve görüntü dolgusu):** atlas hücresi `textureGrad` ile tekrarlanır, dünya ızgarasına hizalıdır.
  - **İşaret:** her işaret bir örnek (`x, y, açı, genişlik, yükseklik`); geometrik şekiller SDF ile, SVG, yazı ve raster işaretler atlastan çizilir. Doku ve işaretlerde önceden çarpılmış alfa karışımı kullanılır.
  - **Ölçek aralıkları** her karede topluluk düzeyinde denetlenir (`FrameState.scaleDenominator`, 96 dpi).
- **Doku atlası (`render/atlas.ts`):** iki arka ucun paylaştığı 2048² Canvas2D sayfası. Girdiler içerikle anahtarlanır (aynı SVG ve renk bir kez çizilir). SVG ve raster eşzamansız çözülür; hazır olunca görünüm yeniden çizilir. Çizimden önce topluluğun istediği bütün görüntüler yerleştirilir (`prefetch`), sonra doku yüklenir. Sayfa dolunca baştan kurulur. WebGPU için mip düzeyleri atlasın kendisinde küçültülerek üretilir.
- **Eşitlik:** WebGL2 ve WebGPU aynı gölgelendirici mantığını taşır (`webgl2/styledShaders.ts`, `webgpu/styledShaders.ts`); birinde yapılan değişiklik ötekine de yapılır. Duman testi stilli sahnede iki motorun piksellerini karşılaştırır.
- **Performans:** katman yalnız kirlenince yeniden kurulur (öznitelik değişince yalnız işleyicisi olan katmanlar); çizim ölçeği ya da kitaplık değişince bütün katmanlar kurulur. Vurgu katmanları (`__sel`, `__hover`) eski ince çizgi hattını kullanır.
- **Bilinen sınırlar:**
  - Birleşimler hep yuvarlaktır (`join` yok sayılır); saydam kalın çizgide parça uçları üst üste binip koyulaşır.
  - Bulanık gölge ve parıltı gibi çizim efektleri yok.
  - Atlas çözünürlüğü sabittir (yazı 48 px, SVG 128 px, döşeme 128 px); çok büyütülen SVG işaret yumuşar.

## 7. Tasarımcılar

Pencereler ilk açılışta yüklenir (`ui/style/`); komutlar Araçlar menüsünde, katman bağlam menüsünde ve öznitelik panelindeki "Sembol" satırındadır.

- **Stil yöneticisi** (`style.manager`, `StyleManager.ts`):
  - Solda kaynak ağacı: Sistem (kilit simgesi, salt okunur), Kitaplığım, Proje; her düğümde altındaki öğe sayısı. Kategori seçmek onu açar ve altındaki bütün öğeleri gösterir.
  - Ortada önizlemeli ızgara: kartlar görünür oldukça çizilir (`Thumbs`, IntersectionObserver). Arama bütün kaynaklarda ad, kategori, etiket ve açıklamada yapılır (Türkçe harfler katlanır). Tür süzgeci: Tümü, Alan, Çizgi, İşaret, Çizim.
  - Sağda seçilenin büyük önizlemesi (alan, adalı alan; düz, kırık, alan kenarı), türü ve kaynağı, düzenlenebilir alanları (ad, kategori yolu "A / B", kaynak, açıklama, etiketler; sistem öğesinde salt okunur) ve eylemler: Düzenle ya da Kopyasını düzenle (Kitaplığım'a kopyalayıp açar), Seçili nesnelere uygula, Kopyala (Kitaplığıma / Projeye), Dışa aktar, Sil (satır içi onayla; bir çizimi kullanan sembol sayısı söylenir).
  - Üst çubukta Yeni sembol (alan/çizgi/işaret, SVG çizimi), İçe aktar (dosyadan ya da panodan; hedef kaynak ve çakışma kipi seçilir: kopya olarak al, üzerine yaz, atla) ve listedekileri Dışa aktar (`.kstil` dosyası ya da panoya metin: sunucu gelene kadar paylaşmanın en kısa yolu bir iletiye yapıştırmaktır). Kullanıcı ve proje kategorilerinde bağlam menüsü: yeni alt kategori, yeniden adlandır (satır içinde; F2), dışa aktar.
  - **Seçme kipi:** katman stilinden ya da "Seçili nesnelere sembol ver…" komutundan açılınca pencere üstte durur, geometri türü önden seçilidir ve alttaki "Seç" düğmesi (ya da çift tık) sembolü verir.
- **Sembol tasarımcısı** (`SymbolDesigner.ts`, `layerForms.ts`, `designerFields.ts`):
  - Solda katman yığını: satırda görünürlük kutusu, tür ve tek satırlık özet. İşaret yerleştiren katmanların (çizgi boyunca işaret, desen, iç noktada işaret) kendi işaret katmanları altında girintili satırdır. Katman ekle, yukarı/aşağı, çoğalt, sil. Üstteki katman önce çizilir.
  - Ortada canlı önizleme (örnek geometri seçici, yakınlaştırma, 1:1 = 96 dpi kâğıt boyu). Sağda seçili katmanın formu: türe göre alanlar ve "Genel" (birim, saydamlık, görünürlük). Her değişiklik önizlemeye hemen yansır, form yeniden kurulmaz (odak korunur).
  - **"ƒ" düğmesi:** veriye bağlanabilen değerleri (renk, kalınlık, boyut, döndürme, metin, görünürlük) ifadeye çevirir; sabit değer ifade boş kalırsa kullanılır.
  - Renk alanı: sistem renk seçici, hex yazımı (alfa korunur) ve tema jetonları (mürekkep, kâğıt, ön plan); "yok" seçilebilir. SVG çizimi dosyadan alınıp Kitaplığım'a eklenebilir (temizlenerek).
  - Kendi geri alması vardır (Ctrl+Z / Ctrl+Y; bir alana bir saniye içinde yazılanlar tek adım). Kaydedilmemiş değişiklikle kapatırken alt çubuk sorar. Sistem sembolü doğrudan açılmaz; kopyası açılır.
  - **Satır içi kip:** katman stilindeki bir sembol ("bu stilde") aynı tasarımcıda düzenlenir; "Uygula" sembolü stile geri verir, kitaplığa yazmaz.
- **Katman stili** (`style.layerStyle`, `LayerStyleDialog.ts`, `rulesEditor.ts`, `symbolSlot.ts`):
  - Üstte işleyici: Basit, Tek sembol, Kategorili, Aralıklı, Kurallar; her türün taslağı ayrı tutulur, türler arasında gidip gelmek iş kaybettirmez. Katmandaki alan, çizgi ve nokta sayıları yazar; semboller yalnızca katmanda bulunan geometri türleri için sorulur.
  - **Sembol yuvası:** küçük resim ve adı ("Basit görünüş", "Bu stilde" ya da kitaplıktaki adı); tıklayınca kitaplıktan seç, burada düzenle (kitaplık sembolünün kopyası), Kitaplığıma kaydet, basit görünüşe dön.
  - **Kategorili:** değer ifadesi (alan önerileriyle), "Değerlerden sınıfla" farklı değerleri nesne sayılarıyla getirir (var olan kategoriler korunur), düzenlenebilir değer ve etiket, "Diğer değerler".
  - **Aralıklı:** sayı veren ifade, eşit aralık ya da eşit sayı, sınıf sayısı, renk rampası; alt sınır dahil, son sınıf üst sınırı da içerir.
  - **Kurallar:** koşul (boş: hepsi), "değilse", ölçek aralığı (1:en yakın – 1:en uzak), alt kurallar; her kuralın koşulu sağlayan nesne sayısı ve ifade hatası canlıdır.
  - Sınıflama saf ve testlidir (`style/classify.ts`: değerler, benzersiz değerler, eşit aralık/sayı, renk rampaları, geometriye göre basit semboller). Uygula haritada gösterir, Tamam uygulayıp kapatır. Katman stili değişikliği tek geri alma adımıdır (`doc.setLayerStyle`); Katmanlar panelindeki renk, çizgi tipi ve kalınlık da öyle.
- **Nesne sembolü:** öznitelik panelinde "Sembol" satırı (Katman stiline göre / Kitaplıktan seç… / Katman stili…), `style.assign` ve `style.clearSymbol` komutları. Tek geri alma adımıdır; kilitli katmandaki nesneler atlanıp sayılır.
- **SVG çizim düzenleyicisi** (`style.svgEditor`, `ui/svgedit/`; model `style/svg/`): işaret ve desen çizimleri (piktogramlar) için kendi aracımız.
  - **Model:** tuval (viewBox) ve arkadan öne şekiller. Dikdörtgen, elips ve yazı türünü korur; gerisi Bézier düğümlü yoldur (çokgen, kırık çizgi, yıldız). Boyalar: sembol rengi (`currentColor`), ikinci renk (`param(stroke)`), sabit renk ya da yok. Yol verisi bütün komutlarıyla (göreli/mutlak, yumuşak eğriler, ikinci derece, elips yayı → kübik, bitişik yay bayrakları) düğümlere okunur ve geri yazılır; afin dönüşüm türü koruyabildiği sürece korur (öteleme, ölçek, düzgün döndürme), yoksa yola çevirir. Hesabın hepsi (aşağıdaki içe alma, dışa aktarma, izleme, yol ve düğüm işlemleri, kenet, hizalama dahil) SVG çekirdeğindedir (`crates/shared/svg-core`; kendi WASM paketi düzenleyici açılınca yüklenir, ADR 0008 “SVG düzenleyicisi”); `style/svg/*.ts` onun cepheleridir ve testlidir (`style/svg/svg.test.ts` …, dondurulmuş yanıtlar `fixtures/svg/v1`).
  - **İçe alma** (`style/svg/importSvg.ts`, değerler `svgValues.ts`; tarayıcı ayrıştırır, `ui/svgedit/readSvg.ts`): tarayıcının yaptığı gibi dolaşılır: `<style>` kuralları (sınıf, kimlik, etiket, öznitelik, torun ve çocuk seçicileri; özgüllük ve sıra, `!important`), sunum öznitelikleri ve `style` devralmayla; bütün dönüşümler (matrix, translate, scale, rotate(a cx cy), skewX/Y); iç içe `<svg>`; `<defs>`, `<symbol>` ve `<use>` (href ve xlink:href; kendine başvuru kırık sayılır); viewBox ve preserveAspectRatio; birimli genişlik/yükseklik (px, mm, cm, pt, in, pc): viewBox yoksa mm cinsinden bir dosya mm birimiyle açılır, mm boyu sembol boyu olur; bütün ilkeller (rx ≠ ry köşeli dikdörtgen yola döner), `<text>`/`<tspan>` (yeniden konumlanan her parça ayrı yazı); opacity, fill-opacity, stroke-opacity (sabit renkte alfa, sembol renginde şeklin saydamlığı); kesik, uç, köşe ve dolgu kuralı modelin alanlarına (SVG varsayılanları yolda açıkça yazılır). Degrade ortadaki (ikide ilk) durağın, desen ilk boyalı şeklinin düz rengine iner; kırpma yolu, maske, süzgeç, çizgi ucu işareti ve görüntü atlanır; hepsi sayılıp özetlenir ("2 degrade düz renge çevrildi, 1 kırpma yolu atlandı"). Katı XML bozuksa bildirimsiz ön ekler ve HTML varlıkları onarılır, en son HTML ayrıştırıcısı denenir. Temizlik `sanitizeSvg` ile (betik, olay, dış başvuru yok).
  - **Renk eşleme:** kendi rengini `currentColor`/`param()` ile veren dosyada renkler olduğu gibi kalır; öbürlerinde siyah ve siyaha yakın renkler (en büyük kanal ≤ 48) sembol rengi olur. İçe alma penceresi bulunan renkleri alan ağırlığıyla sıralar: sembol rengi olacak renk (Siyah, Baskın renk, Hiçbiri ya da bir renk) ve ikinci renk (`param(stroke)`) tıklanarak seçilir; önizleme hemen değişir (`mapColors`, `colorUsage`).
  - **Dosya** (`ui/svgedit/svgFile.ts`; çizimin üstündeki "Dosya ▾" menüsü ve düğmeleri): Yeni çizim; SVG dosyası aç (Ctrl+O) → içe alma penceresi: "Yeni çizim olarak aç" (tuval dosyanınki olur, geçmiş sıfırlanır, kaydedilmemiş iş varsa sorar) ya da "Bu çizime ekle" (tuvale sığdırılır, tek geri alma adımı); dosyayı tuvale sürükleyip bırakmak da açar. Panodaki SVG metni Ctrl+V ile doğrudan eklenir (özet durum satırında), "Panodan içe al" pencereyle alır. Kitaplıktan aç: stil yöneticisi çizim seçme kipinde açılır; sistem çiziminin önce Kitaplığım'a kopyası alınır. Farklı kaydet (Ctrl+Shift+S; alt çubukta Kaydet'in yanında): yeni ad, kategori ve Kitaplığım ya da Proje; açık çizim yeni kayda bağlanır.
  - **Dışa aktarma** (`svgExport.ts`, saf kısım `style/svg/exportSvg.ts`; Ctrl+Shift+E): SVG (sembol: `currentColor` ve `param(stroke)` kalır), SVG (düz renk: önizleme renkleri yazılır, alfa fill/stroke-opacity olur, sembol boyu varsa genişlik mm), PNG (piksel genişlik ya da DPI; DPI mm boyundan, yoksa 1 birim = 1 px; saydam ya da kâğıt zemin; DPI pHYs parçasına yazılır). Her biçimde "Yalnızca seçilenler" tuvali seçime (çizgi kalınlığıyla) kırpar. İndir ya da panoya kopyala.
  - **İzleme altlığı** (`svgReference.ts`): PNG, JPEG ya da SVG görüntü kâğıdın üstüne, ızgaranın ve çizimin altına konur (tuvalin `underlay` kancası); kilitli ve %50 saydam gelir. Altlık şeridinde göster/gizle, kilit (kilitsizken sürüklenerek taşınır), saydamlık, Konum… (X, Y, genişlik; oran korunur), Sığdır, "Dosyada sakla" (varsayılan kapalı; açıkken kaydedilen dosyada `<defs>` içinde `data-kentos="reference"` görüntüsü olarak durur, sembolde çizilmez, açılınca geri gelir), İzle…, kaldır. Geri alma geçmişine girmez (ızgara gibi çalışma yardımcısıdır).
  - **Bitmap izle** (`svgTrace.ts`, saf kısım `style/svg/trace.ts`): altlık ya da bir görüntü dosyası delikli yollara çevrilir. Parlaklık eşiği (alfa beyaz kâğıda yatırılır), ters çevir, benek temizliği (px²), köşe eşiği, yumuşatma, sadeleştirme, çözünürlük (uzun kenar 300/600/1000 px), her parça ayrı şekil; önizleme canlıdır. Yöntem: yürüyen kareler (mürekkep hep aynı yanda; dış halkalar artı, delikler eksi alanlı; çapraz pikseller bağlanır), en küçük saran dış halkaya delik yerleştirme, pikselin kestiği kare köşelerinin geri kazanılması, Douglas–Peucker ile köşe seçimi, köşeler ve çeyrek dönüşler arasında kübik uydurma (`fitCurve.ts`), birleşimlerde ortak teğet. Altlıktan izlenen yollar altlığın yerine, dosyadan izlenenler tuvale sığdırılarak konur; dolgu sembol rengi, evenodd.
  - **XML kaynağı** (`svgSource.ts`; "Kaynak" düğmesi, Ctrl+Shift+X): çizimin altında, satır numaralı, her öğe bir satırda ve kimliğiyle. Seçili şekillerin öğeleri vurgulanır, öğeye tıklamak şekli seçer. Metin düzenlenince Uygula (Ctrl+Enter) okuyup tek geri alma adımıyla uygular; XML hatası satır ve sütunuyla söylenir, imleç oraya gider; Geri al düzenlemeyi bırakır. Yüksekliği üst kenarından sürüklenir.
  - **Belge özellikleri** (`svgDocProps.ts`): görünüm kutusu (X, Y, genişlik, yükseklik: tuval o pencere olur; İçeriğe sığdır ve kenar payı), birim ölçeği (çizim, tuval ve çizgi kalınlıkları birlikte), sembol genişliği mm ("1 birim = … mm"; dosyada `data-size-mm`), önizleme zemini (temanın kâğıdı ya da bir renk; dosyada `data-background`). Tek geri alma adımı; altlık çizimle birlikte kayar.
  - **Araçlar:** Seç (V: tıkla, Shift ekle, pencereyle seç, sürükle taşı, sekiz tutamaçla boyutlandır, Shift oranı korur, üst düğmeyle döndür, Shift 15°), Düğüm (A), Dikdörtgen (R), Elips (E; Shift kare/daire, Alt merkezden), Çokgen (P; kenar sayısı ve yıldız), Kırık çizgi (L), Kalem (B; tık köşe, sürükle eğri düğümü, ilk düğüme tık kapatır, Enter bitirir), Yazı (T), Ölç (M). Tekerlekle yakınlaştırma, orta tuş ya da Boşluk+sürükle ile kaydırma. Tıklama bir şeyi taşımaz: taşıma, boyutlandırma, döndürme, düğüm ve kol sürükleme imleç 3 piksel yürüyünce başlar (kenet ve ızgara ancak o zaman çeker); seçili birkaç şekilden birine hareketsiz tık yalnız onu seçer. Çift tık (yola düğüm düzenleme; kırık çizgi ve kalemde bitir) düzenleyicinin kendi sayımıdır (iki basış 450 ms ve 4 piksel içinde): tuval imleci yakalayıp basışta yeniden çizdiği için tarayıcı çift tık olayı göndermez. Tuvalin kodu `svgCanvas.ts` (görünüm, seçim, çizim araçları), `svgNodeTool.ts`, `svgSnap.ts`, `svgRulers.ts`, `svgMeasure.ts`; ortak türler `svgView.ts`.
  - **Yol işlemleri** (`style/svg/pathOps.ts`; bindirme `pathBool.ts`, çizgi ve öteleme `pathStroke.ts`, uydurma `fitCurve.ts`, eğri yardımcıları `bezier.ts`; "Yol ▾" menüsü ve Özellikler'de Yol grubu): Birleşim (Ctrl++), Fark (Ctrl+-: alttakinden üsttekiler çıkar), Kesişim (Ctrl+*), Dışlama (Ctrl+^: tek sayıda şeklin örttüğü yer), Bölme (Ctrl+/: alttaki, üsttekilerin çizgileriyle parçalara), Yolu kes (Ctrl+Alt+/: alttakinin çizgisi kesişimlerde açık parçalara; dolgusuz); Tek yolda topla (Ctrl+K), Parçalara ayır (Ctrl+Shift+K; ayrıca delikleri çevrelerindekiyle bırakan biçimi); Nesneyi yola çevir (Ctrl+Shift+C; yazı çevrilmez, harf çizimi yok); Çizgiyi yola çevir (Ctrl+Alt+C: uçlar, köşeler, gönye sınırı ve kesik deseniyle; dolgusu olan şeklin dolgusu altta ayrı şekil kalır, ikisi gruplanır); İçe küçült / Dışa büyüt (Ctrl+( / Ctrl+): mesafe ve açılan köşeler yuvarlak, sivri ya da pah); Sadeleştir (Ctrl+L; tolerans seçimin boyunun yüzdesi, keskin köşeler kalır); Yönü çevir, Kapat, Aç (kapanış parçası kalır). Sonuç alttaki şeklin boyasını ve yerini alır; yazılar katılmaz ve söylenir.
    - **Eğriler eğri kalır.** Bezierler çizimin on binde biri toleransla kirişlere bölünür ve her kiriş hangi parçanın hangi parametre aralığından geldiğini bilir; uygulamanın düzlem bindirme motoru (`model/geom/overlay.ts`) girdi köşelerini bit bit koruduğu için sonuç halkasının her kenarı bir kirişin üstündedir ve aynı parça üstündeki kenar dizileri o Bézier'in tam alt parçasına (de Casteljau) döner. Yalnızca kesişim köşeleri tolerans içinde kayar. Kimsenin olmayan kenarlar (çizgi dış hattı, öteleme) kübiklerle uydurulur, yuvarlak birleşim ve uçlar tam yaydır.
    - **Dolgu kuralları:** nonzero şekil olduğu gibi bir kaynaktır (motorun iç testi sıfır olmayan sarım). Halkaları kesişmeyen even-odd şekil iç içe derinliğine göre yeniden yönlendirilir; halkaları kesişen (tek çizgiyle çizilmiş yıldız) çizgilerinin yüzlerine ayrılır, tek sarımlı yüzler tutulur.
    - **Çizginin dış hattı** tek kaynakta, hepsi aynı yönde dönen parçaların birleşimidir: kıvrılmayan her düzgün koşu tek şerit, iç yanı katlanan yerde yeni şerit ve arada disk; köşede birleşim (yuvarlak disk, gönye sınırı içinde sivri, yoksa pah), açık uçta uç. Kesik desen önce yolu böler. Öteleme, bölgenin dış hattının 2×mesafe kalınlıktaki çizgisini bölgeye ekler ya da çıkarır.
  - **Düğüm aracı** (A; `svgNodeTool.ts`, işlemler `style/svg/nodeOps.ts`, düğmeler sağda "Düğüm aracı" kutusunda): düğüme tık seçer (Shift ekler), boşlukta sürüklenen kutu içindekileri seçer, parçaya tık iki ucunu seçer, seçili düğümler birlikte sürüklenir (tutulan düğüm kenetlenir). Düğüm türleri: köşe (Shift+C, baklava), yumuşak (Shift+S, kare; kollar hizalı, boyları serbest), simetrik (Shift+Y; kollar eşit), otomatik (Shift+A, daire; kollar komşulardan, düğümler oynadıkça yeniden kurulur, kolu sürüklenince yumuşak olur); Alt ile sürüklenen kol serbestleşir. Parçaya çift tık oraya düğüm ekler, düğüme çift tık köşe/yumuşak değiştirir. Düğüm ekle (Insert: seçili parçaların ortasına), düğüm sil (Delete biçimi koruyarak, yani silinen aralığa uydurulan tek kübik; Ctrl+Delete korumadan), uçları birleştir (Shift+J: ortalarında tek düğüm; Shift+K: araya parça), düğümde kır (Shift+B), parçayı sil (Alt+Delete), parçaları düz (Shift+L) ya da eğri yap (Shift+U), ok tuşlarıyla kaydır, Ctrl+A tüm düğümler. Düğümleri hizala ve dağıt (sol, orta, sağ, üst, orta, alt; eşit aralık). Düğümün türü `PathNode.type` alanında durur (yalnız düzenleme; dosya yazmaz).
    - **Köşe yuvarla ve pah** (CAD'deki gibi): "Köşe yuvarla" ya da "Pah kır" basılıyken imlecin yaklaştığı köşe halkayla işaretlenir; köşeye basıp bir kenar boyunca çekince yarıçap (yuvarlamada teğet uzaklık × tan(açı/2)) ya da pah boyu canlı görünür (yakınlığa göre yuvarlanmış adım), bırakınca uygulanır; kip sonraki köşeyi bekler, Esc bitirir. Kesin değer: kutudaki alana yazılıp "Seçili köşeleri bu yarıçapla yuvarla" / "… pah kır". Kenarlar eğri olabilir: kesim noktası kenarın köşeden o uzaklıktaki yeridir, yay iki teğete oturur; komşu düğüme varan kesim onunla birleşir; sığmayan yarıçap söylenir.
  - **Kenetleme** (`style/svg/snapping.ts`; "Kenet ▾" düğmesi ve Özellikler'de tuval seçiliyken tür listesi; `%` açar/kapar): köşe ve yumuşak düğüm, parça ortası, kesişim (eğriler ve kılavuzlarla dahil), kutu köşesi, kutu kenar ortası, kutu merkezi, nesne merkezi (daire merkezi, dikdörtgen ortası, yolun ağırlık merkezi), dik ve teğet (aracın başladığı noktadan), kılavuz (ve kılavuz kesişimi), tuval kenarı ve ortası. Noktalar çizgilerden (kılavuz, tuval kenarı) önce gelir; hiçbiri yakın değilse ızgara. İşaret ana CAD'in biçimlerindedir, türün adı yanında yazar. Taşımada seçimin kutu köşeleri, merkezi ve yol düğümlerinin (en çok 64) hedefe en yakın olanı kenetlenir. Dizin sürükleme başına bir kez kurulur.
  - **Cetvel ve kılavuzlar** (`svgRulers.ts`; "Cetvel" düğmesi): üstte ve solda çizim birimli cetveller (1-2-5 aralık, tuvalin boyu amber çizgi). Üst cetvelden sürüklemek yatay, soldakinden dikey kılavuz çeker; kılavuz sürüklenerek taşınır, cetvele geri bırakılınca silinir, çift tık X, Y ve açıyı (eğik kılavuz) yazdırır, Sil. Kılavuzlar çizimle (`SvgDoc.guides`) geri almaya girer; SVG dosyasına yazılmaz.
  - **Ölç** (M; `svgMeasure.ts`): kenetlenen iki nokta arası (sürükleyerek ya da iki tıkla) uzaklık, açı (sağdan saat yönünün tersine) ve ΔX/ΔY; çizim biriminde ve belgenin sembol boyu varsa mm cinsinden. Ölçüm yokken imlecin altındaki yolun parça boyları ve toplam uzunluğu görünür.
  - **Sağ sütun sekmeleri:** Özellikler | Hizala (Ctrl+Shift+A) | Dönüştür (Ctrl+Shift+M) | Dizi.
    - **Özellikler:** seçimde dolgu ve çizgi boyası, kalınlık, saydamlık; çizgi biçimi (kesik hazırları kalınlığa göre ya da yazılı değerler, uçlar düz/yuvarlak/kare, köşeler sivri/yuvarlak/pah, dolgu kuralı tek-çift/sıfır olmayan); kutu (X, Y, genişlik, yükseklik, oran kilidi, yazılan açıyla ↺/↻ döndür); türe göre geometri; Yol düğmeleri; düzen (en öne Home, bir öne Page Up, bir arkaya Page Down, en arkaya End; yatay çevir H, dikey Shift+H, 90°, grupla/çöz, çoğalt, sil); seç (aynı dolgu, aynı çizgi, ikisi, seçimi ters çevir `!`). Seçim yokken tuval, ızgara, kenet türleri, cetvel, döşeme önizlemesi ve önizleme renkleri (sembol rengi temayı izler, bir renk seçilince sabitlenir).
    - **Hizala** (`svgAlign.ts`, saf `style/svg/arrange.ts`): neye göre (seçimin kutusu, ilk seçilen, son seçilen, en büyük, en küçük, tuval; tek şekil tuvale), kenarlar ve ortalar; dağıt: sol/orta/sağ, üst/orta/alt eşit aralık ya da eşit boşluk (en dıştakiler kalır). Grup tek şekil sayılır; "seçimi tek parça taşı" seçimi blok olarak hizalar.
    - **Dönüştür** (`svgTransform.ts`, Inkscape'in Dönüştür penceresi): taşı (kadar ya da konuma), ölçek (% ve oran kilidi, dokuz noktadan sabit nokta), döndür (açı, yön; merkez kutunun dokuz noktasından biri ya da "Tuvalde göster…" ile tıklanan, kenetlenen nokta), eğ (yatay/dikey açı), matris (a b c d e f); "her birine ayrı" her şekli kendi kutusuna göre dönüştürür, taşımada her şekli bir adım daha ileri götürür. Değerler pencere açık kaldıkça durur, Uygula aynı adımı yineler.
    - **Dizi** (`svgArray.ts`): satır-sütun (adet; aralık boşluk ya da adım olarak), dairesel (adet, dolum açısı; 360° tam tur eşit, azı yayın iki ucunu da kullanır; merkez seçim, tuval ya da tıklanan nokta; kopyalar dönsün ya da yalnızca yer değiştirsin; yön) ve aynalı kopya (dikey, yatay ya da açılı eksen; seçimden, tuvalden ya da tıklanan noktadan geçer). Kopyalar tuvalde soluk önizlenir; Uygula tek geri alma adımıdır, gruplar her kopyada kendi grubunu alır.
  - **Şekil listesi** (`svgObjects.ts`): öndeki üstte; göz gizler, kilit şekli tuvalde seçilmez ve taşınmaz yapar (tıklama alttakine geçer; kilitli seçimde tutamaç çıkmaz, taşıma ve hizalama söyleyip durur), ada çift tık yeniden adlandırır (Enter, Esc), satırı sürüklemek sırasını değiştirir (amber çizgi bırakılacak yeri gösterir).
  - Kendi geri alması (Ctrl+Z/Y), Ctrl+D, Ctrl+G, Ctrl+A (kilitli ve gizliler hariç), ok tuşlarıyla kaydırma. Kaydet çizimi Kitaplığım'a SVG varlığı olarak yazar (sistem çiziminde kopya; sembol boyu, zemin ve saklanan altlık dosyayla gider). Stil yöneticisinde "Yeni sembol → SVG çizimi" ve çizimlerde "Düzenle"; sembol tasarımcısında çizim seçicinin yanında "Yeni çizim…", "Düzenle…", "Dosya al…".
- **Lejant** (`style.legend`, `LegendDialog.ts`; saf kısım `style/legend.ts`): katman katman sembollerin anlamı. İşleyicisiz katman kendi görünüşüyle, tek sembol/kategorili/aralıklı/kurallı katmanlar sınıf başına bir satırla (yalnızca katmandaki geometri türleri; kurallarda üst kural adıyla), kendi sembolü olan nesneler o sembolün kitaplık adıyla girer. Katmanlar tek tek dışarıda bırakılabilir; PNG, ekran temasından bağımsız beyaz kâğıt ve siyah mürekkeple, 2× çözünürlükte kaydedilir.
- **Raster desen:** PNG ve JPEG dosyaları (stil yöneticisinde İçe aktar ya da tasarımcıda Dosya al) görüntü varlığı olur; görüntü dolgusunda döşeme, görüntü işaretinde işaret olarak kullanılır.

## 8. Aşamalar

1. **Çekirdek (yapıldı):** sembol ve işleyici türleri, kitaplık (kaynaklar, ağaç, kopyala-silinemez, dışa/içe aktar), çözümleme, derleme (vuruş, dolgu, işaret ilkelleri), testler.
2. **GPU (yapıldı):** kalın ve kesikli çizgiler, taramalar, işaret örnekleri (SDF), doku atlası (SVG, yazı, raster, desen); katmanlara ve nesnelere bağlama; ölçek aralıkları; iki arka uçta eşit çizim.
3. **Stil yöneticisi ve sembol tasarımcısı**, katman stili penceresi, nesneye sembol verme (yapıldı).
4. **SVG editörü** ve raster desenler (yapıldı).
5. **MPYY sistem kitaplığı (yapıldı):** yönetmelik eklerinin bütün gösterimleri, 770 sembol:

   | Bölüm | Kaynak | Öğe |
   |---|---|---|
   | Uygulama imar planı | EK-1d | 204 |
   | Nazım imar planı | EK-1ç | 118 |
   | Çevre düzeni planı | EK-1c | 46 |
   | Mekânsal strateji planı | EK-1e I | 39 |
   | Ortak gösterimler | EK-1a | UİP 89, NİP 89, ÇDP 88 |
   | Piktogramlar | Çizimlerden elle çizilmiş SVG | 79 |

   - Kaynaklar `style/system/mpyy/` altında, plan kademesine göre klasörlenir (`uip/`, `nip/`, `cdp/`, `msp/`, `ortak/`). Yardımcılar `dsl.ts`'te, piktogramlar `pictograms.ts` ve `pictogramDrawings.ts`'tedir.
   - Rakamlar EK-1e metninden alınır. Metnin vermediği ölçüler çizimlerden ölçülür. Metin ile çizim çeliştiğinde metin uygulanır. Her tahmin ve çelişki öğenin açıklamasında (`note`) yazar.
   - Değişken metinli semboller öznitelikten okur (`[Kat]`, `[TAKS]`, `[Genişlik]` …).
   - Yollar alanlardaki genişlikten gerçek boyunda çizilir (`$ölçek`).
   - Her sembol demo projenin gösterim kataloğunda örnek geometri üzerinde durur.
   - **Yaklaşık kalanlar**, motorda karşılığı olmadığı için:
     - Dişli şekli yok; dişliler şekil katmanlarından kurulur.
     - Desen daireye kırpılamaz; benekli daireler sabit nokta öbekleridir.
     - Şerit içi tarama (dik tarama çiftleri) işaret deseniyle çizilir.
     - Dalga evresi yol başına bağlanamaz; sulak alan dalgası işaretle yerleşir.
     - Sayfaya sabit gölge yok; MSP plan sınırının gölgesi kaydırılmış gri çizgidir.
     - Yay şekli yok; seyir terası kirişlerle çizilir.
     - Desendeki işaret kaydırması desenle dönmez, çizgi boyuncaki işaretinki döner.
   - Lejant penceresi kullanılan sembollerden lejant üretir.
6. **Paylaşma:** sunucu ile kitaplık paylaşımı ve kurum kitaplıkları.
