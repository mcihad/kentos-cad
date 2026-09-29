# ADR 0141: Gezinme, sorgu ve seçim ekleri; Netcad adları

- **Durum:** kabul edildi (2026-09-29).
- **Tarih:** 2026-09-29
- **Bağlam belgesi:** ADR 0140 (yeni araçlar), ADR 0056 (gezinme ve pano), ADR 0067 (ölçme araçları), ADR 0029 (seçim ve depo), ADR 0083 (nokta hesapla); PiriCAD'in `netcad_plan.md`'si (N-01, N-02, N-03, N-04, U-01).

## Bağlam

PiriCAD'in Netcad planı, Netcad kullanıcısının elinin aradığı şeyleri sıralar. Planın "Bugün" satırları PiriCAD'i anlatır. KentOS'la karşılaştırıldı: alan işlemleri, içine tıklayarak alan, nokta hesaplayıcı ve paralel çizgi KentOS'ta zaten var.

Planın P0'larından eksik kalanlar dört başlıkta toplanır:

- **Gezinme:** önceki ve sonraki görünüm, katmana yakınlaşma, çizimin kapsamını bozan nesneleri bulma.
- **Sorgu:** ilk noktası sabit mesafe ölçme, bir noktanın bir hatta göre dik ayağı ve dik boyu (Netcad'in Prizma'sı), ölçülen alanı alan olarak çizme, içine tıklayarak alan ölçme.
- **Seçim:** çitle, daireyle ve noktayı içeren alanla seçme.
- **Adlar:** Netcad kullanıcısının yazdığı komut adları.

Sahibin kararı (29 Eylül): önce biçim değiştirmeyen bu hızlı işler yapılır, iki platformda.

## Karar

### Alınanlar

| Ne | Kimlik | Yer (sekme › panel) | Yazdığı |
|---|---|---|---|
| Önceki görünüm | `view.previous` | Görünüm › Yakınlaştır | yazmaz (oturum) |
| Sonraki görünüm | `view.next` | Görünüm › Yakınlaştır | yazmaz (oturum) |
| Kapsam denetimi | `view.extentCheck` | Görünüm › Yakınlaştır | yazmaz; seçer |
| Katmana yakınlaştır | katman ağacının sağ tık menüsü | Katmanlar paneli | yazmaz |
| Mesafe ölç: sabit ilk nokta | `tool.measure` seçeneği | Harita › Ölçme | yazmaz |
| Alan hesapla: içine tıkla, alan olarak çiz | `tool.area` seçenekleri | Harita › Ölçme | alan olarak çiz: `cad.polygon.create` |
| Dik ayak ölç | `tool.stationOffset` | Harita › Ölçme | yazmaz; iletiye |
| Çitle seç | `tool.selectFence` | Giriş › Seçim (Seç ▾) | yazmaz; seçer |
| Daireyle seç | `tool.selectCircle` | Giriş › Seçim (Seç ▾) | yazmaz; seçer |
| İçeren alanı seç | `tool.selectContaining` | Giriş › Seçim (Seç ▾) | yazmaz; seçer |
| Netcad adları | takma adlar | komut satırı ve Komut ara | — |

- Seç, Çitle seç, Daireyle seç ve İçeren alanı seç bir ailedir (`family: 'select'`): Seç ▾ bölünmüş düğmesi.
- Araçlar ve komutlar katalogdadır. Yapılmamış olan soluk durur ve "henüz yok" der; platformda yapıldıkça etkinleşir.

### Alınmayanlar

- **Biçim değiştirenler** (tepe noktası kotu, çok parçalı alan, blok, yazı ekleri): kendi ADR'leriyle, sahibin kararıyla ve özellik özellik gelecek.
- **Türe göre seç, dışındakileri seç, noktadan geçeni seç:** İfadeyle seç ve Seçimi ters çevir çoğunu karşılar; gerekirse sonra eklenir.
- **Nokta Girişi bağlam sekmesi:** Nokta hesapla (ADR 0083) komut satırının çipinden ve menüden açılır. Ayrı bir şerit sekmesi eklenmez.
- **KES ve KAYDIR'ın Netcad anlamı:** bu adlar KentOS'ta başka komutlara aittir (pano kesme, görünümü kaydırma). Ad çakışmasından kaçınmak için Netcad anlamları alınmaz.

### Mekanik (KentOS'un)

**Görünüm geçmişi.** Her pencerenin kendi geçmişi vardır, oturumun durumudur: belgeye, geri almaya ve dosyaya girmez.

- Bir görünüm, kullanıcı ondan ayrılırken kaydedilir. Bunlar:
  - gezinme komutları: Tümünü göster, Seçime yakınlaştır, Pencere yakınlaştır, Katmana yakınlaştır, Yakınlaştır, Uzaklaştır;
  - kaydırmanın başlangıcı (orta tuş ya da Kaydır aracı);
  - tekerleğin, en az 500 ms durduktan sonraki ilk adımı. Bir tekerlek dizisi tek geçiştir.
- Pencerenin boyu değişince kayıt olmaz. Son kaydın aynısı yeniden kaydedilmez.
- En çok 30 görünüm tutulur, en eskisi düşer.
- Önceki görünüm, bulunulan görünümü "sonraki"ler yığınına koyar ve son kayda döner. Sonraki görünüm bunun tersidir.
- Yeni bir gezinme "sonraki"leri siler.
- Başka bir çizim açılınca geçmiş boşalır.
- Komutlar geçmiş yoksa devre dışıdır.

**Kapsam denetimi.** Görünen nesnelerden çizimin geri kalanından çok uzakta olanları seçer ve kaç tane olduklarını söyler. Yanlış koordinat sistemiyle gelen ya da sıfıra düşen nesneler buna örnektir.

- Nesneleri taşımaz, silmez; karar kullanıcınındır.
- Ölçüt ortak çekirdeğin `Store::extent_outliers`'ıdır:
  - Nesne merkezlerinin eksen başına orta yarısı (çeyrekler arası kutu) çizimin çekirdeğidir.
  - Kendi kutusu çekirdekten, çekirdeğin uzun kenarının 10 katından (en az 10 m'nin 10 katından) daha uzakta kalan nesne uzaktır.
  - Nesnelerin dörtte birine kadarı uzak olsa da bulunur. Dörtten az nesnede çekirdek yoktur.
- Sonucu yazar:
  - bir şey bulunursa "3 nesne çizimin geri kalanından çok uzakta; seçildi.";
  - bulunmazsa "Çizimin kapsamını bozan nesne yok."

**Katmana yakınlaştır.** Katman satırının sağ tık menüsündedir; grup satırında adı "Gruba yakınlaştır"dır.

- Katmanın (grupta bütün katmanlarının) nesnelerine yakınlaştırır; görünüm geçmişine girer.
- Nesnesi olmayan katmanda devre dışıdır.

**Mesafe ölç, sabit ilk nokta.** "Sabit" seçeneği açılıp kapanır ve aracın oturum belleğinde kalır.

- Açıkken her yeni nokta ilk noktadan ölçülür.
- İmleç yanındaki etiket ilk noktadan mesafeyi ve semti gösterir.
- Önizleme zincir yerine ilk noktadan ışınlar çizer.
- Her tık iletiye "n: mesafe, semt" yazar. Toplam yazılmaz.

**Alan hesapla.**

- **İçine tıkla** seçeneği: tıklanan noktayı çevreleyen bölgeyi İçine tıklayarak alan'ın yoluyla bulur (kapalı nesne ya da çizgilerin yüzü, adaları delik). Alanını ve çevresini yazar.
- **Alan olarak çiz** çipi, bir ölçümden sonra görünür. Son ölçülen alanı, delikleriyle, etkin katmana kapalı alan olarak yazar: `cad.polygon.create`, tek geri alma adımı, adı "Alan olarak çiz". Bir sonraki ölçüme kadar geçerlidir.

**Dik ayak ölç (Prizma).**

- Adımlar:
  1. Hattın başına (A), sonra sonuna (B) tıklanır; kenet çalışır.
  2. Her tıklanan nokta için `side_offsets(A, B, P)` hesaplanır.
- Değerler Nokta hesapla'nın yan noktasıyla aynı kuraldadır:
  - **dik ayak:** A'dan B yönünde ölçülür;
  - **dik boy:** hatta dik ölçülür; sağı artı, solu eksidir.
- İmleç yanında "Ayak 12.400 m · Boy +3.100 m" görünür. Önizleme hattın uzantısını, dik ayağın yerini ve kesikli dikmeyi gösterir.
- Her tık iletiye yazılır; hiçbir şey çizime yazılmaz.
- "Başka hat" yeni hat seçtirir. Sağ tık ya da Enter bitirir.

**Seçim araçları.** Üçü de bitince Seç'e döner. Sonuç bulunan seçimin yerine geçer; bitirirken Shift basılıysa seçime eklenir.

- Yalnız görünen nesneler seçilir. Kilitli katman pencere seçimindeki gibidir.
- **Çitle seç:**
  - Çitin noktalarına tıklanır; sağ tık ya da Enter bitirir.
  - Çitin kestiği nesneler seçilir: bir kenarı, yazının gövdesi, çite imleç toleransından yakın nokta.
  - Sorgu `Store::in_fence`'tir.
- **Daireyle seç:**
  - Merkeze tıklanır, sonra yarıçap gösterilir ya da yazılır.
  - İçinde kalanlar seçilir. "Kesişen" seçeneği dokunanları ve daireyi içine alan kapalı nesneleri de seçer.
  - Sorgu `Store::in_circle`'dır.
- **İçeren alanı seç:**
  - Tıklanan noktayı içeren en küçük kapalı nesne seçilir: alan (deliği dışarıda), daire, tam elips ya da kapalı eğri.
  - Aynı yere yeniden tıklamak bir büyüğüne geçer (parsel → ada → mahalle), sonuncudan sonra başa döner. "Aynı yer" imleç toleransı içindedir.
  - Etiket "2/3 · 1 600 m²" gibi sırayı ve alanı gösterir.
  - Sorgu `Store::containing`'dir.

**Netcad adları.** Var olan komutlara takma ad eklenir. Takma adlar çakışma denetiminden geçer.

| Takma ad | Komut |
|---|---|
| `ALANSOR` | Alan hesapla |
| `CETVEL` | Mesafe ölç |
| `XYZSOR` | Koordinat oku |
| `LIMITBUL` | Tümünü göster |
| `PENCEREBUYUT` | Pencere yakınlaştır |
| `KUTU` | Dikdörtgen |
| `AYRISTIR` | Patlat |
| `KOSEYUVARLAT` | Köşe yuvarla |
| `COKLUDOGRU` | Çoklu çizgi |
| `BICIMBOYA` | Özellik kopyala |
| `OBJEBOL` | Parçala |
| `PRIZMA` | Dik ayak ölç |
| `ONCEKIPENCERE` | Önceki görünüm |

### Çekirdek işlevleri

`kentos-geometry-core`'da, `store/select.rs`. Web'e WASM deposundan açılırlar (`GeometryStore.containing`, `inFence`, `inCircle`, `extentOutliers`; web'de `PickIndex` yöntemleri), masaüstüne yerel depodan.

- `Store::containing(p) -> Vec<(id, alan)>`: noktayı içeren görünen kapalı şekiller, küçükten büyüğe; eşit alanlar belgenin sırasında.
- `Store::in_fence(çit, tol) -> Vec<id>`.
- `Store::in_circle(merkez, r, kesişen) -> Vec<id>`.
- `Store::extent_outliers() -> Vec<id>`.

Dik ayak `geom::survey::side_offsets`'tir (vardı). İçine tıkla, İçine tıklayarak alan'ın bölge yolunu kullanır.

### İş bölümü ve sıra

1. **Ortak zemin:**
   - çekirdek sorguları, WASM ve testleri;
   - katalog girişleri, ikonlar ve takma adlar;
   - envanter.
2. **Web ve masaüstü:** her platform dört fazı yapar: gezinme, sorgu, seçim, adların denenmesi. Her faz çalışır, sınanır ve resimle gösterilir.

## Sonuçlar

- Netcad kullanıcısı önceki pencereye döner, uzak nesneyi bulur, dik ayağı ölçer, çitle ve içeren alanla seçer. Adlarını yazarak komutları bulur.
- Seçim ve görünüm geçmişi oturumun durumudur. Dosya biçimi ve ürün komutları değişmez; yalnız "Alan olarak çiz" var olan `cad.polygon.create` ile yazar.
- Görünüm geçmişinin kuralı iki platformda aynıdır; her platform kendi kamerasında uygular.

## Doğrulama

- **Çekirdek:**
  - Rust birim testleri (`store::select`): el hesabıyla sahneler. İç içe üç kare, delikli alan, çit, daire ve TM koordinatlı 100 parselle iki uzak nesne. Elli km ara ile iki kasabada uzak nesne çıkmaz.
  - WASM testi (`apps/web/src/wasm/select.wasm.test.ts`) aynı sahnelerde.
- **Platformlar:**
  - her aracın ve komutun davranış testleri;
  - görünüm geçmişi: 31 gezinmeden sonra 30 adım geri, sonra ileri; yeni gezinme sonrakileri siler;
  - dik ayak: A(0,0), B(100,0), P(30,5) için ayak 30.000, boy −5.000 (sol);
  - resimler 1440×900 ve 1100×650'de, iki temada.
