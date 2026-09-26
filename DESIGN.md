# KentOS CAD — Tasarım sistemi

KentOS'un görsel dilini ve etkileşim kurallarını tanımlar. Mimari ve kod
kuralları için [CLAUDE.md](CLAUDE.md) dosyasına bakın. Değerlerin tek kaynağı
`apps/web/src/styles/tokens.css` dosyasıdır; bu belge o değerlerin **neden** öyle
olduğunu ve nasıl kullanılacağını anlatır.

---

## 1. İlkeler

1. **Çizim alanı önce gelir.** Arayüz sakin, yoğun ve geri planda kalır. Göz, pafta üzerindeki veride durmalıdır.
2. **Tek vurgu, tek anlam.** Vurgu rengi yalnızca şunları işaretler: etkin araç, seçim, odak, birincil eylem ve "değişecek" durumu. Başka hiçbir şey vurgu renginde değildir. Vurgu rengini kullanıcı seçer (§3.5; varsayılan **lacivert**); bu belgede geçen “amber” sözü vurgu rengi demektir.
3. **Sahanın dili.** Terimler, birimler ve işaretler harita mühendisinin kullandığı gibidir: Y sağa ve X yukarı, semt (grad), ada ve parsel, pafta, kot, poligon noktası, "K" kuzey oku.
4. **Klavye birinci sınıftır.** Her araç ve komutun kısayolu vardır ve görünür durumdadır (araç kutusundaki tuş etiketi, ipucu, menü, F1 listesi).
5. **Dürüst arayüz.** Yapılmamış özellik "geliştirme aşamasında" yazar. Koordinat dönüştürülmüyorsa bunu açıkça söyler. Sessiz başarısızlık yoktur.
6. **Masaüstü iş istasyonu.** Uzun oturumlar için tasarlanır: yüksek bilgi yoğunluğu, düşük göz yorgunluğu, sabit yerleşim. Mobil hedeflenmez.

---

## 2. Marka

| Öğe | Kural |
|---|---|
| Ürün adı | **KentOS CAD** (sayfa başlığı, Hakkında penceresi) |
| Menü çubuğundaki yazı | **KentOS** (arayüz yazı tipi, 700, `--fs-md`, marka laciverti `--c-brand-text`; “OS” 500) ve ▾ |
| Logo | **Lacivert karo üstünde K.** 22×22 yuvarlak köşeli karo (r 6), açık maviden (`--c-brand-hi`) laciverte (`--c-brand`) çapraz geçişli, içte ince açık çizgi; üstünde beyaz (`--c-brand-ink`) K: dikey gövde ve kollar, birleştikleri yerde bir ölçme noktası (dolu daire) bulunur. 20×20 ızgara, 2,1 px çizgi, yuvarlak uçlar. |
| Favicon | `public/favicon.svg`: `#1E252E` zemin üstünde aynı K |

Logo ve yazısı **her zaman laciverttir** (`--c-brand*` jetonları; karo iki temada da lacivert, yazı koyu temada açık lacivert, açık temada lacivert); kullanıcının seçtiği vurgu rengini izlemez, çünkü markanın kendi rengidir. Logo döndürülmez, başka renge boyanmaz; altında hafif bir gölge vardır. Logo ile yazı bir düğmedir: tıklayınca uygulama menüsü açılır (§7.1.1).

---

## 3. Renk

Renkler **rol** adıyla kullanılır, değerle kullanılmaz. TypeScript içinde arayüz
rengi yazılmaz; çizim alanı renkleri `readCanvasPalette()` ile CSS
jetonlarından okunur.

### 3.1 Koyu tema: "Grafit" (varsayılan)

Mavi-grafit bir kabuk çizim alanını çevreler. Alan kabuktan bir ton daha
derindir. Yakın siyah değildir, belirgin biçimde mavi-gridir.

| Jeton | Değer | Rol |
|---|---|---|
| `--c-menubar` | `#181e26` | Menü çubuğu, durum çubuğu (en koyu kabuk) |
| `--c-toolbar` | `#1e252e` | Araç çubuğu |
| `--c-panel` | `#212932` | Paneller, araç kutusu, pencereler |
| `--c-panel-head` | `#252e38` | Panel başlıkları, alt çubuklar, ayar menüsü |
| `--c-popover` | `#27303b` | Açılır menüler, öneri listesi |
| `--c-field` | `#171c23` | Giriş alanları, komut satırı, liste zeminleri |
| `--c-tooltip` | `#2f3945` | İpucu |
| `--c-line` / `--c-line-strong` | `#2d3641` / `#3d4856` | Ayırıcılar / kenarlıklar |
| `--c-text` / `-2` / `-3` | `#d6dde5` / `#9ba7b5` / `#6d7988` | Birincil, ikincil ve üçüncül metin |
| `--c-accent` | `#f2b632` | **Prizma sarısı** |
| `--c-accent-ink` | `#1b1405` | Amber dolgu üstündeki metin ve simge |
| `--c-ok` / `--c-warn` / `--c-danger` / `--c-info` | `#6fd08c` / `#e9a23b` / `#ef6b61` / `#6db3f2` | Durum renkleri |
| `--canvas-bg` | `#141a21` | Çizim alanı |
| `--canvas-fg` / `--canvas-fg-dim` | `#e4eaf0` / `#a3afbc` | "Ana mürekkep" (`fg`) / "ikincil mürekkep" (`fg-dim`) |
| `--canvas-ink` | `#ffffff` | "Siyah" (`ink`, CAD renk 7): koyu zeminde beyaz çizilir |
| `--canvas-grid-minor` / `major` | `#ffffff0a` / `#ffffff17` | Izgara |
| `--canvas-accent` / `--canvas-snap` | `#f2b632` / `#6fd08c` | Seçim / kenet işareti |

### 3.2 Açık tema: "Pafta"

Serin, kâğıt paftayı andıran griler. Krem ya da sıcak kâğıt tonu kullanılmaz.

| Jeton | Değer | Not |
|---|---|---|
| `--c-menubar` / `--c-toolbar` / `--c-panel` | `#e3e7eb` / `#eceff2` / `#f4f6f8` | |
| `--c-field` | `#ffffff` | |
| `--c-text` / `-2` / `-3` | `#1b232c` / `#4d5966` / `#7a8591` | |
| `--c-accent` | `#f0b02a` | Dolgu olarak kullanılır |
| `--c-accent-text` | `#8f5f00` | Açık zeminde amber **metin** (kontrast için koyulaştırılmış) |
| `--canvas-bg` / `--canvas-fg` | `#f8f9fa` / `#1e2833` | |
| `--canvas-ink` | `#000000` | "Siyah" (`ink`): açık zeminde tam siyah |
| `--canvas-accent` / `--canvas-snap` | `#d08600` / `#1a9a48` | Açık zeminde okunur tonlar |

### 3.3 Vurgu kuralları

- **Dolu amber** (`--c-accent`) yalnızca üç yerde: etkin araç düğmesi, birincil düğme (Kaydet) ve açık switch.
- **Kırmızı** (`--c-danger`) arayüzde hata metni ve simgesi, çizim alanında yalnızca budama önizlemesinde "silinecek parça" içindir.
- **Yumuşak amber** (`--c-accent-soft`) seçili liste satırı, basılı araç çubuğu düğmesi ve menü vurgusudur.
- **Amber çizgi** (`--c-accent-line`) odak ve "değişecek" kenarlığıdır.
- **Amber metin** için her zaman `--c-accent-text` kullanılır, `--c-accent` değil; açık temada fark vardır.
- Uyarı rengi (`--c-warn`) amber'e yakındır. Bu yüzden uyarılar her zaman ⚠ simgesiyle birlikte gelir; renk tek başına anlam taşımaz.

### 3.4 Katman renkleri (çizim verisi)

- Katman rengi veridir (`LayerStyle.color`). Tema değişince yalnızca `fg`, `fg-dim` ve `ink` jetonları çözülür; diğer renkler sabittir.
- **"Siyah" (`ink`) CAD'deki renk 7'dir:** kâğıtta ve açık temada siyah, koyu temada beyaz çizilir. Saf siyah koyu zeminde görünmez olurdu. Taslak katmanı bu renkle başlar ve renk listelerinde ilk sıradadır.
- Varsayılan katman renkleri **orta doygunlukta** seçilir ki iki temada da okunsun. Sarı ve beyaz gibi tek temada kaybolan renklerden kaçının.
- Gelenek korunur:

  | Katman | Renk |
  |---|---|
  | Taslak | siyah (`ink`) |
  | Ada sınırı | ana mürekkep, kalın |
  | Parsel sınırı | ikincil mürekkep |
  | Yapı | açık mavi, hafif dolgulu |
  | Yol ekseni | kırmızı, noktalı kesik |
  | Eşyükselti | kahverengi (ana eşyükseltiler bir ton açık ve kalın) |
  | Kot noktası | yeşil artı |
  | Poligon noktası | turkuaz üçgen |

- Kullanıcı renk paleti (`DRAW_COLORS`): Kırmızı, Sarı, Yeşil, Camgöbeği, Mavi, Eflatun, Gri. İsimlerle sunulur.

### 3.5 Vurgu rengi seçenekleri

**Uygulama ayarları → Görünüm → Vurgu rengi** (`prefs.accent`, `app/appearance.ts`): **Lacivert** (varsayılan), Amber (§3.1–3.2'deki değerler), Petrol yeşili, Bordo. Her seçenek aynı jetonları yeniden tanımlar (`styles/accents.css`: `--c-accent`, `--c-accent-ink`, `--c-accent-text`, `--c-accent-soft`, `--c-accent-line`, `--c-tooltip-accent`, `--canvas-accent`) ve temaya göre ayarlanır: koyu temada okunabilsin diye açık bir ton (lacivertte `#4c7fe0` dolgu, `#8fb3f5` metin), açık temada koyu ton (lacivertte `#1f4a96`). Lacivert, petrol ve bordoda dolgu üstündeki mürekkep beyazdır. Çizimdeki seçim rengi (`--canvas-accent`) vurguyu izler. Uyarı turuncusu (`--c-warn`), kenet yeşili ve hata kırmızısı hiçbir seçenekte değişmez. Seçimde her renk, yarısı koyu temanın yarısı açık temanın tonu olan yuvarlak bir örnekle gösterilir.

### 3.6 Çizim kalitesi

**Uygulama ayarları → Çizim motoru** (tipli ayarlar `graphics.msaa`, `graphics.hiDpi`; bu cihaza özgü, [ADR 0023](docs/adr/0023-typed-settings.md)):

- **Hazır ayar:** **Hızlı** (kenar yumuşatma yok, mantıksal piksel başına bir piksel; Retina ve 4K ekranda dörtte bir piksel), **Dengeli** (tam çözünürlük, kenar yumuşatma yok), **Kaliteli** (varsayılan; 4× kenar yumuşatma ve tam çözünürlük). Hazır ayar yalnız iki değeri doldurur; değerler hiçbirine uymuyorsa seçili dilim **Özel**'dir (seçilemez).
- **Kenar yumuşatma (MSAA):** Kapalı, 2×, 4×, 8×, 16×. Aygıtın desteklemediği sayı istenebilir; desteklenen en yakın alt sayı kullanılır ve altındaki not “İstenen 8×, kullanılan 4×.” diye nedeniyle söyler (uyarı notu). Eşitse bilgi notu kullanılanı ve aygıtın sayılarını söyler.
- **Tam çözünürlük (HiDPI)** anahtarı ve çizim hedeflerinin yaklaşık ekran belleği.
- Değişiklik Kaydet ile hemen uygulanır: aynı tuval ve bağlamda yalnız çizim hedefleri yeniden kurulur. Kalın ve kesikli çizgilerin kenarı gölgelendiricide yumuşatıldığı için her kademede düzgündür; fark ince çizgilerde ve dolgu kenarlarında görünür.
- Masaüstünde aynı iki değer, Uygulama ayarları penceresinin Grafik grubundadır.

---

## 4. Tipografi

| Aile | Kullanım |
|---|---|
| **Plus Jakarta Sans** (varsayılan; değişken ağırlık) | Bütün arayüz metni. **Uygulama ayarları → Görünüm → Yazı tipi** (`prefs.uiFont`) ile Inter, IBM Plex Sans, Source Sans 3, Noto Sans, Roboto ya da sistemin yazı tipi seçilir. |
| **Barlow** (varsayılan çizim yazı tipi) | Çizimin kendi yazıları: yazı nesneleri, ölçü değerleri, etiketler (üst katman). **Proje ayarları → Genel → Çizim yazı tipi** (`doc.settings.drawingFont`, projeyle kaydedilir; yeni projelerin varsayılanı Uygulama ayarları → Yeni projeler) ile klasik teknik çizim yazıları seçilir: Arimo (Arial ölçülerinde), Overpass (DIN/ISO), Quicksand (ince, yuvarlak uçlu; plotter yazısına benzer), Architects Daughter (mimari el yazısı), Courier Prime (daktilo), IBM Plex Mono. Veridir; arayüz yazı tipi seçimini izlemez. |
| **IBM Plex Mono** (400, 500) | **Yalnızca** komut satırı girdisi, komut geçmişi, takma ad gösterimi (`PL`, `PARSEL`) ve işlem pencerelerindeki ifade alanı (komut gibi yazılır). Veri etiketlerinde mono kullanılmaz. |

**Yazı tipleri uygulamayla gelir, CDN'den ya da internetten yüklenmez** (`apps/web/src/assets/fonts/<ad>/`, her birinin yanında SIL OFL 1.1 lisansı; `styles/fonts.css`). Yalnız Latin ve Latin Extended alt kümeleri vardır (ğ, ş, İ ikincisindedir); tarayıcı yalnız kullanılan yazı tipini indirir. Seçim kartında her yazı tipi kendisiyle yazılır (“Ağ Şı İ 123”). Arayüz yazı tipi çizim alanındaki işaretlere (kenet adı, ölçek çubuğu, kuzey oku) de uygulanır; çizimdeki yazı nesneleri, ölçü değerleri ve etiketler veridir, yazı tipi değişmez. Masaüstü aynı yüzleri web'in dosyalarından üretilmiş TrueType olarak taşır (`apps/desktop/assets/fonts/drawing`, [ADR 0055](docs/adr/0055-desktop-drawing-text.md)).

Rakamlar her yerde **tabular** (`.num` sınıfı ya da `font-variant-numeric: tabular-nums`) yazılır, böylece koordinatlar imleç hareket ederken titremez.

### 4.1 Ölçek

Bütün boyutlar `--ui-scale` ile çarpılır. **Uygulama ayarları → Görünüm → Yazı boyutu** değerleri: Küçük 0,93, Standart 1, Büyük 1,08, Çok büyük 1,16, En büyük 1,25. En büyükte de şerit 1100 px'te her sekmeye sığar.

| Jeton | Standart | Kullanım |
|---|---|---|
| `--fs-2xs` | 11 px | Grup başlıkları, rozetler, küçük etiketler |
| `--fs-xs` | 12 px | Durum çubuğu, ikincil bilgiler, açıklama altı |
| `--fs-sm` | 13 px | **Gövde:** menüler, ağaç, özellik ızgarası, düğmeler |
| `--fs-md` | 14 px | Ürün adı, panelde öne çıkan başlıklar, ayar satırı etiketi |
| `--fs-lg` | 16 px | Pencere başlığı, seçili CRS adı |
| `--fs-xl` | 19 px | Ayar bölümü başlığı |

### 4.2 Kurallar

- **Cümle düzeni:** "Katman ve öznitelik paneli", "Yeni projeler için varsayılan". Yalnızca ilk kelime ve özel adlar büyük harfle başlar.
- **BÜYÜK HARF etiket yok,** harf aralığı açılmış üst başlık yok.
- Bir başlıkta tek kelimeyi renk ya da italikle vurgulamak yoktur.
- Kalınlıklar: 400 gövde, 500 etiket ve adlar, 600 başlıklar ve etkin öğeler. 700 kullanılmaz.
- Çizim alanı yazıları (etiketler) `--ui-scale`'den etkilenmez; boyutu `LabelStyle` belirler.

---

## 5. Ölçüler ve yerleşim

### 5.1 Kabuk

```
┌ Menü çubuğu (32) ─ K KentOS  Dosya Düzen Görünüm Çizim Değiştir Harita Koordinat Analiz İşlemler Araçlar Yardım ─ proje adı ─ TUREF / TM36 ┐
├ Araç çubuğu (44) ─ dosya │ geçmiş │ görünüm │ [■ Katman ▾] [Renk ▾] [Tip ▾] [Kalınlık ▾] ······ [Ölçek 1:1000 ▾] │ paneller ┤
│ ┌──┐                                                                           K │ Katmanlar (ağaç)                   │
│ │  │ kayan araç kutusu                                                        ↑  │ [Katmanlar | İşlemler] sekmeleri    │
│ │  │                    Çizim alanı (WebGL2 + 2B üst katman)                     ├──────────── ayırıcı ────────────────┤
│ └──┘                                                               0 ─── 50 m   │ Öznitelikler                       │
├ [alt panel: Komut geçmişi | Koordinat listesi | Uyarılar] (F2, isteğe bağlı)    │                                     │
├ Komut satırı (36) ─ İstem: [girdi……]                                          ⌃ │                                     │
├ Durum çubuğu (28) ─ Y … X … │ mesaj │ n seçili │ ▪Kenet ▪Izgara ▫Orto ▫Kutupsal │ Ekran 1:2.470 │ TUREF / TM36 │ ● Buluta kaydedildi │ ○ Sunucu: yok │ WebGL2  ┤
```

**Şerit düzeni** (Uygulama ayarları → Görünüm → Arayüz düzeni: Şerit): menü çubuğu, araç çubuğu ve araç kutusunun yerini tek bir şerit alır; gövde aynı kalır.

```
┌ Sekme satırı (32) ─ K KentOS [💾 ↶ ↷ ▾] │ Dosya  Giriş  Çizim  Değiştir  Harita  Görünüm  İşlemler  Araçlar  [Seçim 3] ─ proje adı ─ [⌕ Komut ara… Alt+Q] TUREF / TM36 ? ⌃ ┐
├ Paneller (96) ─ [Yapıştır│kes kopyala …] │ [seç, tümünü seç …] │ [Çizgi Çoklu çizgi│kapalı alan daire …] │ … │ [■ Katman ▾]│[Renk ▾ Tip ▾ Kalınlık ▾] ┤
│                                       Pano          Seçim                     Çizim ↘                         Katmanlar ↘   Özellikler ↘                  │
```

- En küçük kabuk boyutu 1100×600 px. Daha küçük pencerede sayfa kayar, yerleşim bozulmaz.
- Sağ dok 240–560 px arasında sürüklenir (varsayılan 312). Katmanlar ile öznitelikler arası oran sürüklenir (varsayılan %50).
- Alt panel 96 px ile ekran yüksekliğinin %60'ı arasında sürüklenir (varsayılan 190). Ayırıcıya çift tıklamak varsayılana döndürür.

### 5.2 Yükseklikler (standart ölçekte)

| Jeton | Değer | Öğe |
|---|---|---|
| `--menubar-h` | 32 | Menü çubuğu |
| `--toolbar-h` | 44 | Araç çubuğu |
| `--ribbon-h` | 96 | Şeridin panelleri (sekme satırı `--menubar-h`) |
| `--status-h` | 28 | Durum çubuğu |
| `--cmd-h` | 36 | Komut satırı |
| `--row-h` | 28 | Ağaç, özellik ızgarası ve liste satırı |
| `--ctl-h` | 30 | Kontrol yüksekliği |

### 5.3 Köşe yarıçapları (hiyerarşi)

| Jeton | Değer | Kullanım |
|---|---|---|
| `--r-xs` | 2 | Renk örneği |
| `--r-sm` | 4 | Düğme, alan, açılır liste |
| `--r-md` | 6 | Menü, not kutusu, kart |
| 8 px | | Kayan araç kutusu |
| `--r-lg` | 10 | Pencereler, tema kartı |

Dok içine yerleşik paneller köşesizdir (0).

### 5.4 Gölge

Yalnızca **yüzen** öğeler gölge alır: araç kutusu (`--shadow-float`); menü, ipucu ve pencere (`--shadow-pop`). Paneller ve kartlar gölgesizdir; ayrım kenarlıkla yapılır.

Tek istisna şeridin gölgesidir (§7.3.1): şeridin altında yalnız çizim alanına düşen hafif bir gölge (`--shadow-bar`) şeridi çizimin üstünde duran bir yüzey gibi ayırır. Yandaki paneller şeritle aynı düzlemdedir, gölge almaz. Koyu temada gölge daha yoğundur, çünkü koyu zeminde az görünür.

---

## 6. İkonografi

- 20×20 viewBox, `stroke="currentColor"`, 1,4 px çizgi, yuvarlak uç ve birleşim. Dolgu yoktur; istisna tutamaçlar ve kasıtlı %14–35 saydam alan dolguları.
- **Tutamaçlar:** çizim aracı simgelerinde tıklamaların düştüğü noktalar 3×3 px dolu karelerle gösterilir (çizgi: iki uç; dikdörtgen: iki karşı köşe). Bu, aracın nasıl kullanıldığını anlatır.
- Harita araçlarının kendi simgeleri vardır: parsel (köşe taşlı çokgen), ifraz (kesikli bölme), aplikasyon (jalon ve sehpa), kot (üçgen ve taban), mesafe (cetvel), alan (kesikli çerçeve ve dolgu).
- Boyutlar: araç kutusu 20, araç çubuğu 18, menü ve panel 16, ağaç satırı 15, küçük düğmeler 14. Şeritte büyük düğme 28 (çizgi kalınlığı büyümez: `vector-effect: non-scaling-stroke`, 1,55 px), küçük düğme 16.
- Yeni simge `ui/icons.ts`'e eklenir ve mevcut ağırlığa uyar. Dış simge kütüphanesi kullanılmaz.

---

## 7. Bileşenler

### 7.1 Menü çubuğu

- **Solda:** logo, KentOS ve menüler. **Ortada:** proje adı; kaydedilmemiş değişiklik varsa önünde amber nokta. **Sağda:** koordinat sistemi düğmesi (tıklayınca Proje ayarları → Koordinat sistemi) ve **Tam ekran** düğmesi (dört köşe dışa; tam ekrandayken içe, Esc de çıkar). Şeritte aynı düğme Yardım'ın solundadır; komut Görünüm → Paneller'dedir (`view.fullscreen`).
- Menü tıklayınca açılır. Açıkken fare başka bir menünün üstüne gelince o menüye geçer. ←/→ menüler arasında gezer, Esc kapatır.
- **Pencere daralınca** proje adı önce kendiliğinden kısalır. Menüler yine sığmazsa (büyük yazı boyutunda) çubuk adım adım yer açar: önce “KentOS” yazısı, sonra koordinat sisteminin adı (düğmesi ve ipucu kalır), en sonda menülerin iç boşluğu. 1100 px'te, En büyük yazıda da her menü görünür.

### 7.1.1 Uygulama menüsü

KentOS logosuna (klasik arayüzde menü çubuğunda, şeritte sekme satırında) tıklayınca açılır; AutoCAD'in uygulama menüsünün karşılığıdır (`ui/appmenu/AppMenu.ts`, ilk kullanımda yüklenir).

- **Panel:** logonun altında, ~780 px genişliğinde, `--c-popover` zeminli, `--r-lg` köşeli ve `--shadow-pop` gölgeli. **Başlık:** 28 px logo, lacivert “KentOS” ve ikincil “CAD”, altında çizimin adı (kaydedilmemişse sonunda •); başlığın zemini laciverten saydama hafif bir geçiştir.
- **Sol sütun:** Yeni, Aç, Kaydet, Farklı kaydet, İçe aktar ▸, Dışa aktar ▸, Bulut ▸, Yazdır ve pafta, Proje ayarları. Satırda 34 px zeminli simge, ad (600) ve altında durumu söyleyen üçüncül satır (Kaydet'te dosyanın adı ya da “İlk kayıtta yer sorulur”, Bulut'ta hesap ya da açık proje); sağda kısayol ya da ›.
- **Sağ bölme:** ▸ taşıyan satırın üzerine gelince ya da odaklanınca onun içeriği, başta “Bu çizim”: adı ve nerede durduğu, yongalar (koordinat sistemi, çalışma modu, ölçek, çizim yazı tipi), nesne ve katman sayısı, kayıt durumu (kaydedilmemişse vurgu noktası ve Kaydet düğmesi) ve dört hızlı karo (Yeni proje, Dosya aç, Bulut projesi, DXF içe aktar).
  - **İçe/Dışa aktar:** biçimler iki satırlı satırlardır; solda eş aralıklı yazılı biçim rozeti (DXF, NCN, NCZ …), ad ve ne aldığı; hazır olmayanın sağında “Yakında” hapı.
  - **Bulut:** sunucu yoksa gri resim, açıklama ve “Yeniden dene”; oturum yoksa vurgu parıltılı resim, “Projeleriniz her yerde” ve “Giriş yap”; oturum varsa baş harfli vurgu dairesiyle hesap kartı ve Çıkış, açık bulut projesi kayıt lambasıyla (yeşil kayıtlı, turuncu bekliyor, kırmızı çakışma ya da hata), hap düğmeler (Proje aç, Buluta yükle, açık projede Yeniden adlandır ve Sil) ve kurumun son beş projesi (tıklayınca Bulut projesi aç penceresi o proje seçili açılır; yüklenirken iskelet satırlar).
- **Alt şerit:** Uygulama ayarları, Kısayollar, Hakkında; sağda “KentOS CAD”.
- **Klavye:** logoda Enter, Boşluk ya da ↓ açar; ↑/↓ satırlar arasında, → sağ bölmeye, ← geri; Esc kapatır ve odağı logoya verir. Bir satır komutunu çalıştırınca, dışarı basınca ya da pencere boyutu değişince kapanır.
- **Masaüstü** (`apps/desktop/src/app_menu/`, ADR 0050): aynı satırlar ve bölmeler, KentOS UI'nin `AppMenu` bileşeniyle; şeridin marka düğmesi açar. Masaüstünde henüz olmayan komut şeritteki gibi soluk durur ve nedenini söyler (“Web'de var; masaüstüne henüz taşınmadı”). Kısayol başlığın satırındadır. Menü açıkken bütün tuşlar ona gider; odak kavramı olmadığından logodan klavyeyle açılmaz. Bulut bölmesindeki son proje tek tıkla açılır.

### 7.2 Açılır menü (PopupMenu)

- **Satır düzeni:** onay sütunu (✓ ya da radyo noktası), simge ya da renk örneği, etiket, ipucu (sayı, birim), kısayol ve alt menü oku.
- **İki satırlı öğe** (`MenuItem.detail`): adın altında ne yaptığını ve nasıl kullanıldığını söyleyen üçüncül renkte bir satır; simge 22 px'e büyür. Adından anlaşılmayan yöntemler için kullanılır (ör. Nokta hesabı).
- Açma ve kapama durumu olan komutlarda simge yerine onay sütunu gösterilir. Araç komutları eylem olarak simgeyle gösterilir.
- Devre dışı öğeler soluk görünür ama listede kalır; kullanıcı özelliğin varlığını görür.
- Ekran kenarına sığmazsa yukarıya ya da sola açılır.

### 7.3 Araç çubuğu

- **Gruplar ve aralarındaki ayırıcılar:** dosya │ geçmiş │ görünüm │ geçerli özellikler (katman, renk, tip, kalınlık) ··· çizim ölçeği │ panel düğmeleri.
- Düğme 30×30'dur; basılıyken yumuşak amber zemin ve amber simge gösterir.
- Açılır listelerin önünde küçük üçüncül bir etiket bulunur ("Renk", "Tip"). Etkin katman listesi renk örneğiyle başlar ve katmanları grup başlıklarıyla gösterir.
- **Pencere daralınca** araç çubuğu adım adım küçülür; sığan ilk adımda kalır, hiçbir şey kesilmez ya da görünmez kaydırmaya kalmaz:
  1. katman, renk, tip ve kalınlık listeleri daralır (değerleri üç noktayla kısalır; ölçek listesi daralmaz, "1:1000" tam okunur);
  2. Görünüm grubunda Yakınlaştır, Uzaklaştır ve Kaydır grubun sonundaki ⋯ düğmesinin menüsüne girer (tekerlek ve orta tuş aynı işi yapar);
  3. Renk, Tip ve Kalınlık tek bir “Özellikler” listesine katlanır; menüsünde üçü birer alt menüdür ve güncel değerlerini sağda gösterir; katman listesi genişliğine döner;
  4. katman listesi yeniden daralır (en büyük yazı boyutları).
  Standart yazıda 1280 px'te 1. adım, 1100 px'te 3. adım yeter.

### 7.3.1 Şerit

Menü çubuğu, araç çubuğu ve araç kutusunun sekmeli karşılığıdır; Uygulama ayarları → Görünüm → **Arayüz düzeni**'nden ya da Görünüm → Şerit arayüzü'nden seçilir ve hemen yerleşir. Sekmeleri menü modelinden ve araç kataloğundan türer (CLAUDE.md §4.5): **klasik arayüzde olan her araç ve komut şeritte de vardır**, yeni bir araç ikisinde birden kendiliğinden çıkar.

- **Sekme satırı** (`--menubar-h`, menü çubuğu zemini): logo ve KentOS; **hızlı erişim** (Kaydet, Geri al, Yinele her zaman; şeritteki bir düğmeye sağ tıklayarak eklenen komutlar ve ▾ özelleştir menüsü); sekmeler; ortada proje adı ve kaydedilmemiş noktası; sağda **Komut ara** kutusu (`Alt+Q`), koordinat sistemi, Yardım (?) ve daraltma düğmesi.
- **Sekmeler:** Dosya, Giriş, Çizim, Değiştir, Harita, Görünüm, İşlemler, Araçlar. Açık sekme ana metin rengi, 600 ağırlık ve 2 px amber alt çizgidir (dok sekmeleriyle aynı, §7.5). Giriş gündelik araçları, pano, seçim, katmanlar ve geçerli özellikleri bir arada tutar; diğerleri menülerin karşılığıdır (Koordinat ve Analiz menüleri Harita sekmesindedir, Düzen'in geçmişi hızlı erişimde).
- **Bağlamsal Seçim sekmesi:** yalnız seçili nesne varken sekmelerin sonunda görünür; seçimin rengindedir (amber metin, üstte amber çizgi, sayı yumuşak amber hapta). İçinde seçimin özeti (büyük amber sayı, “nesne seçili”, türlere göre sayılar), seçime uygulanan dönüştürme, dizi, nesne, alan, pano ve sembol komutları vardır. Seçim bitince kaybolur ve önceki sekmeye dönülür; kendiliğinden açılmaz.
- **Çalışan araç noktası:** çalışan araç açık olmayan bir sekmede de bulunuyorsa o sekmenin sağ üstünde 5 px amber nokta vardır; ipucu aracın adını söyler.
- **Alt kenar:** 1 px çizgi; altında yalnız çizim alanına düşen hafif gölge (`--shadow-bar`, §5.4), yan paneller gölgesizdir. Daraltılmışken sekme satırının altındadır.
- **Paneller:** sekme başına başlıklı gruplar; aralarında 1 px çizgi, altta küçük (`--fs-2xs`) üçüncül başlık; varsa sağında pencere açıcı (↘: katman stili, proje ayarları, uygulama ayarlarının ilgili bölümü; Giriş'in araç panellerinde o ailenin sekmesi).
- **Düğmeler:** büyük (28 px simge, altında en çok iki satır etiket), küçük (16 px simge ve tek satır etiket, sütunda üç tane), yalnız simge (dar pencere, hızlı erişim). Açılır düğmenin etiketi ▾ taşır. Etiket komutun kısa adıdır, sondaki “…” yazılmaz; tam ad ve kısayol ipucundadır.
- **Boyut anlamdan gelir, sıradan değil** (AutoCAD ve Netcad gibi): panelin **ana araçları** büyüktür (katalogda `primary`: Çizgi, Çoklu çizgi, Daire, Yay; Taşı, Kopyala, Döndür; Ötele, Buda; Köşe yuvarla; Yazı, Ölçü, Tarama; alan birleştir/kesiştir/çıkar/böl; Parsel, Kot noktası; araç olmayan komutlarda `PRIMARY_COMMANDS`: Yapıştır, Yeni, Aç, Kaydet, Tümünü göster, Stil yöneticisi …), geri kalanı küçüktür. Bir ana araç, sıkışık olmayan her panelde aynı boydadır. Panelde en çok dört büyük düğme olur. Ana aracı olmayan bir ya da iki öğeli panelin düğmeleri büyüktür. Giriş'in Değiştir paneli AutoCAD'in Modify paneli gibi **sıkışıktır** (`compact`): hepsi küçük.
- **Bölünmüş düğme (aile ve yöntemler):** bir araç ailesi (`family`: Dikdörtgen / Döndürülmüş dikdörtgen / Düzgün çokgen; Yardımcı çizgi / Işın; Dik in / Dik çık; Dizi / Kutupsal dizi; Köşe yuvarla / Pah) ve yöntemleri olan bir araç (`methods`: Daire ▾ merkez-yarıçap, 2 nokta, 3 nokta, TTY, TTT; Yay ▾ 3 nokta, merkez, devam) tek düğmedir. Üst parça (büyükte simge) son seçileni çalıştırır; alt parça (büyükte etiket ve ▾, küçükte ▾) listeyi açar. Yöntem aracı o seçenekle başlatır (istemde tuşa basılmış gibi). Son seçim oturum boyunca hatırlanır. İki parça üzerine gelince ayrı ayrı aydınlanır, çevresinde ince çizgi çıkar; ailenin aracı çalışırken üst parça dolu amberdir.
- **Panel ▾ (seyrek araçlar):** az kullanılan araçlar (`rare`: Halka, Revizyon bulutu, Uzat-kısalt, Köşe ekle/sil, Çizgiye çevir) panelde durmaz; panel başlığının yanındaki ▾ ile açılan listededir (AutoCAD'in panel genişletmesi). Arama onları bulur ve ▾'yi gösterir.
- **Durumlar:** çalışan araç **dolu amber** (araç kutusundaki gibi; hızlı erişimde yumuşak amber, çünkü dolu amber tek olmalı); açık anahtar komut (Kenetleme, Katman paneli) yumuşak amber; devre dışı %38; yapılmamış özelliğin simgesi %62 ve ipucunda “Geliştirme aşamasında”. Aramada bulunan düğme bir an 2 px amber çerçeve alır.
- **Pencere daralınca** paneller sağdan sola, her biri bir adım inerek küçülür: büyük → küçük etiketli → yalnız simge → panelin adını taşıyan tek düğme (tıklayınca panel altında açılır). Genişlik kazandırmayan adım atlanır. Giriş'te Çizim ve Değiştir büyük düğmelerini ve etiketlerini öbür paneller yalnız simgeye inene kadar korur (1600 px'te Çizim'in dört ana aracı büyüktür); yalnız bir paneli tek düğmeye katlamak onlardan önce gelir. Sekme satırı da adım adım yer açar: önce “KentOS” yazısı ve `Alt+Q` etiketi, sonra koordinat sisteminin adı, en sonda arama kutusu büyütece döner. 1100 px'te her sekme sığar; hiçbir düğme kesilmez ya da kaydırmaya kalmaz.
- **Daraltma:** `Ctrl+F1`, sekmeye çift tık ya da sağdaki ⌃ düğmesi şeridi sekme satırına indirir; çizim büyür. Daraltılmışken bir sekmeye tıklamak şeridi çizimin **üstünde** açar (gölgeli, `--shadow-pop`); bir komut çalışınca, dışarı tıklayınca ya da Esc ile kapanır.
- **Komut ara:** komutun adını ya da komut satırı takma adını (ör. `L`, `CIZGI`) arar; satırda simge, ad, şeritteki yeri (“Çizim › Eğri”) ya da “Geliştirme aşamasında”, kısayol ve “Şeritte göster” düğmesi vardır. Enter ilkini çalıştırır, Alt+Enter yerini gösterir, Esc temizler.
- **Klavye:** sekmelerde ←/→, Home/End; ↓ panellere iner; panellerde oklar düğmeler arasında gezer, Esc sekmeye döner. Düğmeye fareyle tıklamak odağı almaz: Enter son komutu yinelemeye devam eder.
- Hareket yoktur: açılma, daralma ve panel küçülmesi anlıktır (§12).
- Şeritle birlikte araç kutusu kapalıdır (Görünüm → Araç kutusu ya da F9 ile açılır ve ayrı hatırlanır).
- **Masaüstü** (KentOS UI, ADR 0051): aynı sığdırma kuralı ve seviyeler; genişlikler yazı ölçümünden hesaplanır. Katlanmış panel tıklayınca araçlarını menüde açar (aileler alt menü, seyrek araçlar “Diğer araçlar”). Düğmeler web'in ölçülerindedir ve web'in ikonlarını taşır (envanterden, [ADR 0054](docs/adr/0054-desktop-draws-the-web-icons.md)): satır 24 px, büyük ikon 28 px ve sabit 1,55 px çizgi; ikon dinlenirken ikincil, üzerine gelince ana renk; çalışan araç dolu vurgu, açık anahtar yumuşak vurgu. Görünüm sekmesinde web panellerinden sonra arayüzün kendi grupları durur: Tema (Koyu, Aydınlık, Gece, Yüksek karşıtlık; sekiz vurgu ve özel renk), Çizim zemini (temaya uy, arduvaz, siyah, kâğıt), Yazı tipi (IBM Plex Sans, Inter, Plus Jakarta Sans), Eş aralıklı (IBM Plex Mono, JetBrains Mono) ve Yazı boyutu (11–18 px). Web'in Tema ve Çizim motoru menüleri masaüstünde yoktur.

### 7.3.2 Çalışma modları

Bir projenin arayüzü **çalışma moduna** göre sadeleşir (`app/workspaces.ts`, CLAUDE.md §4.12): Hibrit (CAD + CBS, her şey), CAD (teknik çizim; Harita, Koordinat ve İşlemler menüleri, parsel ve arazi araçları gizli; Harita sekmesinin kalanı “Ölçme” adını alır), CBS (coğrafi bilgi sistemi; yardımcı çizgi, şekil, ölçü, tarama, dizi, köşe araçları gizli, Giriş'te Harita paneli). Mod veriyi değiştirmez ve gizlenen komutlar komut satırından ve kısayoluyla yine çalışır. Duyurulan modlar (3D Plan, Afet Analizi) “Yakında” yazar ve seçilemez.

- **Mod kartları** (`ui/settings/workspacePicker.ts`; Yeni proje, Proje ayarları → Genel, Uygulama ayarları → Yeni projeler): seçilebilen üç mod yan yana kartlardır. Kartta 40 px zeminli resim (mod simgesi 28 px), ad (`--fs-md`, 600), amber alt başlık (ne olduğu), açıklama ve üç maddelik liste; sağ üstte seçim halkası. Seçilen kart amber çerçeve, yumuşak amber zemin, amber resim zemini ve dolu onay halkasıdır. Ayar pencerelerinde kartlar kısadır (açıklama ve liste yok). Klavye: ←/→ seçilebilen modlar arasında gezer.
- **Yakında:** seçilebilen kartların altında, kesikli bir çizgiyle ayrılmış “Yakında” başlığı ve daha alçak, kesikli çerçeveli, saydam kartlar; sağda amber çizgili hap “Yakında”. Tıklanmaz; ipucu ne olacağını söyler.
- **Durum çubuğu:** koordinat sisteminin solunda amber mod simgesi ve modun adı (600). Tıklayınca menü: başlık “Çalışma modu”, seçilebilen modlar (radyo), ayırıcı, duyurulanlar soluk ve sağda “Yakında”. Aynı menü Görünüm → Çalışma modu'dadır.
- Mod değişince menü çubuğu, şerit ve araç kutusu yerinde yeniden kurulur; açık sekme kalıyorsa açık kalır. CAD'de sağ dokun İşlemler sekmesi gizlenir.
- **Masaüstü** (ADR 0052): her modun şeridi web'in kurduğu hâliyle envanterden gelir; durum çubuğunda mod işareti ve adı, tıklayınca aynı menü. Menü çubuğu ve araç kutusu masaüstünde yoktur.

### 7.4 Kayan araç kutusu

Arayüzün **tek cesur öğesi**dir.

- Varsayılan üç sütun (isteğe bağlı iki), 34×32 düğmeler. **Çalışma modunun gösterdiği tüm araçlar her zaman görünür** (§7.3.2); gizli alt menü (yığın) kullanılmaz, çünkü fareyle aracı arayan kullanıcı onu görmelidir.
- Gruplar kısa başlık taşır: Seçim, Çizim, Açıklama, Dönüştür, Düzenle, Alan, Harita. Başlık küçük (`--fs-xs`, 600, üçüncül renk) ve bir katlama düğmesidir: tıklamak grubu katlar, ok 90° döner. Katlanan gruplar çalışma alanı yerleşimiyle saklanır (`ui.toolboxFolded`).
- **Kaydırma çubuğu çıkmaz.** Seçilen sütun sayısı (2 ya da 3) yüksekliğe sığmıyorsa araç kutusu bir sütun daha genişler (en çok 6); araçlar gizlenmez ve kaydırılmaz. 900 px yüksekliğindeki pencerede 58 araç 5 sütunda sığar. Pencere büyüyünce seçilen sütun sayısına döner.
- **Etkin araç dolu amber zemin** ve koyu mürekkeple gösterilir.
- Her düğmenin sağ alt köşesinde **tuş etiketi** vardır: `L`, `⇧M`, `⌥P`, `Esc`, `Del`.
- İpucu: ad, kısayol, kısa açıklama ve **fareyle kullanım adımları** (numaralı liste, amber numaralar). Adımlar katalogdaki `steps` alanından gelir ve "tıklayın, sürükleyin, sağ tıklayın" diliyle yazılır.
- Hazır olmayan araçların simgesi %62 saydamdır; ipucunda "Geliştirme aşamasında" yazar.
- Tutamaçtan sürüklenir, kenara 14 px yaklaşınca yapışır ve sol sütuna sabitlenebilir. Sütun ve sabitleme düğmeleri üzerine gelince görünür.

### 7.4.1 Komut şeridi

Bir komut çalışırken çizim alanının üst ortasında yüzen şerittir (`ui/shell/CommandBar.ts`). Fare kullanıcısı alttaki komut satırına bakmadan ne yapacağını buradan okur.

- **Sol sütun:** araç simgesi (amber) ve adı (600), beklenen adım (ana metin), köşeli parantezdeki notlar (üçüncül) ve **seçenek düğmeleri**. Düğmede seçenek adı, varsa şu anki değeri (amber) ve aynı işi yapan tuş (küçük tuş etiketi) yazar: `Yay Y`, `Kopya açık K`, `Son yarıçap 3.000 m Enter`.
- **Sağ sütun (sabit):** ince ayırıcıdan sonra fare hatırlatması: `Sağ tık` onayla / bitir, `Esc` çık.
- Seçim aracı boştayken şerit gizlidir; tutamaç düzenlenirken görünür.
- Alttaki komut satırı da aynı seçenekleri düğme olarak gösterir.
- Şerit bir tercihtir: Uygulama ayarları → Görünüm → Fare yardımcıları → **Komut şeridi** (`drafting.commandBar`), **varsayılan kapalı**. Sahibin gözlemi (26 Eylül): aynı adım ve seçenekler alttaki komut satırında daha derli toplu duruyor. Kapalıyken komut satırı şeridin yalnız kendisinde olanları da gösterir: **Nokta hesabı** düğmesi ve tek seferlik kenet etiketi (“Sonraki tık: Orta nokta” ×).
- Masaüstünde aynı şerit çizim alanının üst ortasında, açılır pencere zemininde yüzer (`apps/desktop/src/command_bar.rs`). Aynı tercihe bağlıdır; ayarı Uygulama ayarları → Çizim yardımcıları'ndadır. Masaüstünde basılı sağ tık menüsü henüz olmadığı için sağ sütunda yalnız `Sağ tık` onayla, `Esc` çık yazar. Şeridin üstüne yapılan tıklama alttaki çizime geçmez.

### 7.4.2 Fare yardımcıları

- **İmleç yanında değer girişi:** imlecin sağ üstünde, amber çerçeveli küçük bir kart; içinde eş aralıklı yazıyla (`--font-mono`) değer alanı ve altında kabul edilen biçimler (`mesafe · Y,X · @dY,dX · @mesafe<açı`, `--fs-2xs`, üçüncül renk). Aracın kendi ölçü etiketi imlecin sağ altında kalır; ikisi çakışmaz.
- **Bilgi kartı:** imlecin sağ altında, panel zemininde; başlıkta tür ya da "Parsel 7" (600) ve sağda katman örneği ile adı; altında ince çizgiyle ayrılmış iki sütunlu değerler (etiket üçüncül, değer sağa yaslı). Fareyle etkileşmez (`pointer-events: none`).
- **Tek seferlik kenet etiketi:** komut şeridinde yumuşak amber zeminli "Sonraki tık: Orta nokta" ve × düğmesi.
- **Kenet simgeleri:** kenet türünün çizimdeki işareti (kare, üçgen, daire, eşkenar dörtgen, çarpı, dik açı, teğet, kum saati) düz çizgiyle, üzerinde durduğu geometri kesikli çizilir. Menülerde 16 px'tir.
- **Nesne izleme:** alınan izleme noktaları kenet renginde (`--canvas-snap`) 10 px'lik artıdır. Kilitlenilen hiza, noktadan başlayıp ekran kenarına kadar uzanan ince kesikli (3/4 px, %85) çizgidir. İmlecin sağ üstünde kenet etiketiyle aynı yazıda "İzleme 12.500 m < 90°" ya da "İzleme: kesişim" yazar.
- **Sağ tuş menüleri:** komut menüsünde başlık olarak aracın adı; önce Onayla/İptal, sonra aracın seçenekleri (sağda tuşları), sonra "Tek seferlik kenet" alt menüsü ve çizim yardımcıları. Tutamaç menüsü başlığı "Köşe 3" ya da "Kenar 2" biçimindedir.

### 7.4.3 İpuçları

İpucu zemini iki temada da koyudur (`--c-tooltip`). Mürekkebi tema metin jetonlarından değil, kendi jetonlarından alır: `--c-tooltip-text`, `--c-tooltip-text-2`, `--c-tooltip-line`, `--c-tooltip-accent`. İpucunun içine `--c-text*` ya da `--c-line` yazılmaz; açık temada koyu zeminde koyu yazıya dönüşür.

### 7.5 Dok panelleri

- **Üst yuva sekmeleri:** sağ dokun üst yuvası "Katmanlar | İşlemler" sekmelerini taşır. Sekme şeridi panel başlığının yerini alır (katlama oku solda kalır); etkin sekmede 2 px amber alt çizgi, amber simge ve 600 ağırlık vardır. Meta ("13 katman", "2 araç") sekmelerin sağında, dar dokta üç noktayla kısalır.
- **Başlık:** katlama oku, başlık (600), üçüncül meta ("13 katman", "#284") ve sağda simge düğmeleri.
- **Katman ağacı:**
  - Satır sırası: girinti, katlama oku, renk örneği ya da klasör, ad, nesne sayısı, göz ve kilit.
  - **Etkin katman** solda 2 px amber çubuk ve kalın adla gösterilir.
  - Gizli katmanın adı %45 saydamdır. Kilitli katmanın kilidi amberdir.
  - Çift tıklama etkin yapar, Boşluk gizler, F2 yeniden adlandırır, sağ tık bağlam menüsünü açar.
  - Göz, kilit ve renk düğmeleri tıklanınca klavye odağını almaz: Enter ve Boşluk ağaçta ya da çizimde kalır.
- **Öznitelikler:**
  - Üstte özet: tür, etiket (amber, ör. parsel no) ve katman yolu.
  - Altında katlanabilir bölümler: Genel, Geometri, Öznitelik bilgileri.
  - Düzenlenebilir hücre üzerine gelinene kadar düz metin gibi görünür. Enter kaydeder, Esc vazgeçer.
  - Sayılar tabular yazılır, birim üçüncül renktedir.
  - Seçim yokken "Seçili nesne yok" ve yönlendirme metni, altında çizim özeti gösterilir.

### 7.6 Komut satırı ve alt panel

- **Komut satırı her zaman görünür.** Solunda istem durur: kalın araç adı ve ardından ne beklendiği, ör. "**Çizgi**: sonraki noktayı belirtin [Kapat (K) / Bitir (Enter)]".
- Girdi mono yazıyla gösterilir. Odaklanınca solda 2 px amber çubuk belirir.
- Yazarken öneri listesi çıkar: simge, başlık, takma ad (mono) ve kısayol.
- **Alt panel isteğe bağlıdır** (F2). Sekmeleri: Komut geçmişi (zaman, simge, metin), Koordinat listesi (Köşe, Y (sağa), X (yukarı), Kenar, Semt; dipte alan ve çevre), Uyarılar (okunmamış sayısı rozetle).

### 7.7 Durum çubuğu

- Hücreler: Y/X imleç koordinatı (tabular) │ son mesaj (5–9 sn görünür) │ seçim sayısı (amber) │ çizim yardımcıları │ ekran ölçeği │ koordinat sistemi │ sunucu │ çizim motoru (en sağda; çip simgesi ve "WebGL2" / "WebGPU"; WebGPU'da simge amber; tıklayınca motor seçme menüsü).
- **Çizim yardımcısı düğmeleri** bir gösterge lambası taşır: kapalıyken boş kare, açıkken dolu amber kare. Metin kapalıyken üçüncül renktedir.
- **Pencere daralınca** hücreler, son mesaja kısa bir ileti sığacak yer (yazı boyunun 15 katı) kalana dek sırayla yer açar; her hücre ipucunu ve tıklamasını korur: çizim motorunun adı (çip simgesi kalır), koordinat sistemi hücresi (menü çubuğunda da vardır), ekran ölçeği, sunucu ve kayıt hücrelerinin yazısı (lambaları kalır), çalışma modunun adı (simgesi kalır), en sonda yardımcı düğmelerinin iç boşluğu. Koordinat, seçim sayısı ve yardımcıların adları her zaman görünür.
- **Sunucu hücresi** yuvarlak bir lamba taşır (yardımcıların kare lambasından ayrılsın diye): bağlıyken dolu yeşil (`--c-ok`), sunucu yokken boş halka ve üçüncül metin (Faz A'da olağan durumdur, hata rengi kullanılmaz), sözleşme sürümü uyuşmazken dolu amber (`--c-warn`). Tıklamak yeniden denetler; ipucu sürümü ya da nedeni yazar.
- **Kayıt hücresi** yalnızca bir bulut projesi açıkken görünür, sunucu hücresinin solunda durur ve aynı yuvarlak lambayı taşır. Yazısı ve lambası:
  - dolu yeşil “Buluta kaydedildi”: yalnızca sunucu yanıtladıktan sonra ve bekleyen bir şey yokken;
  - boş lamba, ikincil metin “Kaydedilecek: n” / “Kaydediliyor…”;
  - amber “Çevrimdışı: n bekliyor”;
  - kırmızı, dolu lamba “Çakışma: n” / “Kayıt hatası” / “Proje silindi” (başkası sildi) / “Erişim kaldırıldı” (paylaşım kaldırıldı); son ikisinde tıklamak ve `Ctrl+S` yerel dosyaya kaydettirir;
  - üçüncül metin “Salt okunur” (yalnız görüntüleme yetkisi; rol açıkken düşürülmüşse değişiklikler cihazda bekler, yetki dönünce gönderilir).

  Tıklamak işe yarayan sonraki adımı yapar: çakışmada çözüm penceresini açar, değilse hemen gönderir (`Ctrl+S` ile aynı). İpucu kurum › proje, son kayıt zamanı ve canlı bağlantının durumunu yazar. Sunucu hücresine tıklamak hesap menüsünü açar (giriş/çıkış, bulut projesi aç, buluta yükle, paylaş, bağlantıyı denetle).
- **Bulut pencereleri** (giriş, projeler, paylaşım, çakışma):
  - `dialog--cloud` sınıfını kullanır. Alan etiketleri üstte ve ikincil renktedir.
  - Hata satırı kırmızı ve `role="alert"`, ilerleme çubuğu amber ve 4 px'tir.
  - Proje listesinde satır seçimi ve çift tıklamayla açma vardır; seçili satır amber vurguludur.
  - Çakışma penceresinde birincil (amber) düğme güvenli seçenektir: “Sunucudakini al”. “Benimkini kaydet” ikincildir.
  - “Bulut projesi aç” penceresinin üstünde iki sekme (uygulamanın sekme biçimi): “Çalışma alanı” (seçilen kurumun ya da kişisel alanın projeleri) ve “Benimle paylaşılanlar” (başkalarının paylaştıkları; adın altında üçüncül renkte “Sahibi: … · alan”, sağda ikincil renkte rolünüz ve tarih).
  - Proje listesinin alt çubuğunda solda hayalet düğmeler: “Paylaş…”, “Yeniden adlandır…” ve “Sil…”. Seçili proje yoksa ya da yetki yoksa devre dışıdır ve nedenini ipucunda söyler (ör. “proje silme yetkiniz yok (project.delete)”). Üçü de listenin üstünde açılır ve bitince listeyi yeniler.
  - Silme penceresi ne olacağını madde madde söyler (listeden kalkar, açık tutanların kaydı durur, nesneler saklanır ve geri getirilebilir). Amber birincil düğmesi yoktur: odak “Vazgeç”tedir, silen düğme “Projeyi sil” kırmızı yazılı, çizgili düğmedir (`btn--danger`).
  - **Paylaşım penceresi** (`dialog--share`, 680 px): başlığın altında proje ve alanı; panel başlığı tonunda, mavi sunucu simgeli kutuda saklama biçimi (“Saklama: Yönetilen PostGIS veritabanı.”); “Kişi ekle” satırı (ad ya da e-posta alanı ve altında açılır menü gibi gölgeli öneri listesi, Rol, isteğe bağlı Bitiş, tek amber “Paylaş”), altında seçili rolün ne yaptığı ikincil renkte. “Erişimi olanlar” listesi: nötr yuvarlak baş harf rozeti, ad (500; kendinizse üçüncül “(siz)”), altında üçüncül renkte e-posta ve erişimin kaynağı (Proje sahibi, Paylaşım ve bitiş tarihi, Kurum politikası); sağda rol (değiştirilebilen paylaşımda açılır liste) ve “Kaldır” (onay penceresinde sorar) ya da değişmeyen erişimde kilit simgesi (ipucu nedenini söyler). Erişemeyen kişide rol yerine ⚠ ve “Erişemiyor”, kaynağın yerinde nedeni. Altta kurum politikası notu. Denetimler yalnız liste sunucudan geldikten sonra açılır.
    - İki sekme, her birinde tek amber düğme (ADR 0042): “Kişiler” yukarıdaki gibidir; “Davetler” e-posta, rol (Görüntüleyici varsayılan, en çok Düzenleyici) ve Geçerlilik (1–90 gün, 14 seçili) formu, altında “Kopyala” düğmeli tek gösterimlik bağlantı kutusu ve “yalnız şimdi gösterilir” uyarısı, en altta durum çipli davet listesi ve “Geri al” (onay penceresiyle). Kişi aramasında tam bir e-posta hiçbir üyeyle eşleşmezse öneri listesi “… adresine e-postayla davet gönder…” sunar.
  - **Projeye davet penceresi** (`?davet=` bağlantısıyla açılır, başlangıç ekranının yerine): bilgiler kutusunda proje, alan, rol ve misafir/üye; birincil düğme “Projeyi aç”; 403 sonrasında solda “Başka hesapla giriş yap”.
  - Açık projeye erişim kaldırılınca onay penceresi biçiminde “Projeye erişiminiz kaldırıldı” bildirimi çıkar: ne olduğunu, çizimin ve gönderilmemiş değişikliklerin cihazda kaldığını, yeniden erişim için kime başvurulacağını söyler; birincil yanıt “Yerel kopya kaydet…”, öbürü “Tamam”.

### 7.8 İpucu

- Başlık (600), kısayol tuşu ve açıklama (ikincil renk). Hazır değilse amber not eklenir.
- 450 ms gecikmeyle açılır; bir ipucu kapandıktan sonraki 600 ms içinde komşu öğelerde anında açılır (araç çubuğunda gezinirken).
- Tıklamada ve basılı tutmada kapanır; öğenin açılır menüsü açıkken görünmez.
- Masaüstünde KentOS UI'nin `tip`'i aynı kurallarla çalışır ([ADR 0054](docs/adr/0054-desktop-draws-the-web-icons.md)).

### 7.9 Pencereler

- Karartılmış arka plan, 10 px köşe ve pop gölgesi. Başlık 16 px. Esc ve arka plana tıklama kapatır; pencerenin içine (yazıya, boşluğa) tıklamak kapatmaz.
- **Boy:** pencere içeriği kadardır, en çok kendi sınırı ya da uygulama penceresi kadar; sığmayınca gövde kayar, başlık ve alt çubuk hep görünür. Kaydırma çubuğu gövdenin yanında durur, içeriğin üstüne binmez. Sekmeli pencereler (ayarlar, yeni proje) sekme değişince boy değiştirmesin diye sabit boydadır. Masaüstünde `Dialog::scroll` ve `max_height` (KentOS UI).
- **Pencere açıkken uygulama kısayolları çalışmaz.** Tab pencerenin denetimleri arasında döner; arka plandakilere geçmez.
- **Alt çubuk:** solda ikincil eylem (hayalet düğme), sağda "Vazgeç" ve birincil eylem. Birincil eylem, değişiklik yoksa devre dışıdır.
- **Tek birincil düğme** kuralı: bir yüzeyde yalnızca bir dolu amber düğme bulunur.
- **Form pencereleri** (ayarlar, katman stili, içe/dışa aktarma) × ve Esc ile sormadan vazgeçer: Vazgeç anlamındadır. **Kendi kaydı olan düzenleyiciler** (SVG çizim düzenleyicisi, sembol ve model tasarımcısı) yalnız kaydedilmemiş bir değişiklik varsa sorar; hiç değiştirilmemiş yeni bir çizim, sembol ya da model de sormadan kapanır. Kaydet'in yazamayacağı bir şey (görünür şekli olmayan çizim) için sorulmaz.

### 7.9.1 Onay penceresi

Uygulama bir şeyi yapmadan önce sorduğunda **tek yol budur** (`ui/widgets/confirm.ts`): soruyu soran pencerenin üstünde açılan küçük bir pencere. Durum satırında, menüde ya da satır içinde soru sorulmaz: orada gözden kaçar ve pencerenin kendi düğmeleriyle karışır.

- **Yerleşim:** başlık (sorunun konusu: “Kaydedilmemiş değişiklikler”, “Kitaplıktan sil”), bir iki cümle ne olacağını söyleyen metin, gerekirse madde madde sonuçlar (üçüncül renk). Genişlik en az 460 px × yazı ölçeğidir; yanıtlar sığmazsa pencere genişler, düğme hiç sıkışmaz, metin içeride kırılır.
- **Yanıtlar** alt çubukta, pencerelerin genel düzeninde: solda ayrı duran ikincil yanıt, sağda Vazgeç ve (varsa) tek amber birincil yanıt.
- **Kaydedilmemiş değişiklikler:** metin neyin değiştiğini adıyla söyler: “Yeni çizim” içinde kaydedilmemiş değişiklikler var. Pencere kapanırsa bu değişiklikler kaybolur. Solda “Kaydetmeden kapat”, sağda “Vazgeç” ve birincil “Kaydet ve kapat”. Odak birincildedir (Enter kaydeder); “Kaydetmeden …” hiçbir zaman varsayılan değildir. Başka bir çizime geçerken aynı soru “… devam et” der; satır içi sembolde “Uygula / Uygulamadan”.
- **Silme:** amber düğme yoktur; odak “Vazgeç”tedir, silen düğme kırmızı yazılı, çizgili düğmedir (`btn--danger`). Geri alınamıyorsa metin bunu söyler.
- **Esc, × ve arka plana tıklama** hiçbir şeyi değiştirmeyen yanıttır (Vazgeç) ve soran pencereye döner; oradaki çalışma olduğu gibi kalır. Soru açıkken ikinci bir tıklama arka plana düşer, yani yine Vazgeç'tir: hiçbir şey sormadan kapanmaz.
- Soru `alertdialog` olarak işaretlenir ve metni ekran okuyucuya başlıkla birlikte okunur.

### 7.10 Ayar pencereleri (Proje ayarları, Uygulama ayarları)

İki pencere aynı iskeleti (`SettingsShell`) kullanır ve kapsamlarını açıkça söyler:

| | Proje ayarları | Uygulama ayarları |
|---|---|---|
| Açılış | Dosya → Proje ayarları…, CRS düğmeleri | Araçlar → Uygulama ayarları…, `Ctrl+,` |
| Sol alttaki kapsam notu | kaydet simgesi + "Proje dosyasına kaydedilir" + dosya adı | ayar simgesi + "Bu tarayıcıda saklanır / Tüm projeler için geçerlidir" |
| Bölümler | Genel, Koordinat sistemi, Birimler ve hassasiyet | Görünüm, Kenetleme, Yeni projeler, Çizim motoru, Ayar dosyası (dışa/içe aktar, varsayılanlara döndür, kaydın yeri) |

- **Yerleşim:** 940 px genişlik ve sabit yükseklik; bölüm değişince pencere zıplamaz. Solda bölüm menüsü (etkin bölümde 3 px amber çubuk ve amber simge), sağda başlık (19 px), bir satırlık açıklama ve gruplar.
- **Satır (`settingRow`):** solda etiket (14 px, 500) ve açıklama (üçüncül renk, en fazla 52 karakter satır), sağda kontrol.
- **Taslak mantığı:** değişiklikler Kaydet'e kadar uygulanmaz. Kaydet yalnızca fark varsa etkindir. "Bu bölümü varsayılana döndür" yalnızca o bölümün alanlarını sıfırlar.
- **Koordinat sistemi seçici:**
  - Üstte etkin sistem kartı; değişecekse kenarlık amber olur ve etiket "Kaydedince … atanacak" der.
  - Solda aranabilir liste: datuma göre gruplu; her satırda SRID kodu, ad ve kapsam; varsayılan olan etiketli.
  - Sağda parametre kartı: tür, datum, elipsoid, projeksiyon, orta meridyen, ölçek faktörü, sağa öteleme, kapsam ve eksen sırası notu.
  - Tam bir SRID yazılınca odak kaybolmadan seçilir. Tanımsız SRID için açık bir mesaj gösterilir.
  - Proje sistemini değiştirmek **"Koordinatlar dönüştürülmez"** uyarı notunu gösterir.
- **Kontroller:** bölümlü seçici (segmented; seçili dilim kabarık), switch (açıkken amber), adımlayıcı (− değer + birim), tema kartları (etkin temadan bağımsız renkli mini çalışma alanı), arayüz düzeni kartları (etkin temanın renklerinde klasik ve şerit mini çalışma alanı, altında bir satır açıklama), motor kartları (radyo, rozet: Varsayılan / Deneysel / Desteklenmiyor).
- **Birimler bölümünde "Önizleme" kartı** vardır: kesikli kenarlıkla koordinat, kenar, alan ve semt örneklerini canlı gösterir.
- **Yeni proje** (Dosya → Yeni proje…, `Ctrl+Alt+N`) aynı satırları ve koordinat sistemi seçicisini düz bir pencerede kullanır (760 px, bölüm menüsü yok): Proje adı (açılışta seçili, yazmak yerine geçer), çizim ölçeği, koordinat sistemi. Seçicinin kartı “Yeni projenin koordinat sistemi” der; amber “değişecek” çerçevesi ve “Koordinatlar dönüştürülmez” notu yoktur, çünkü değişen bir şey yoktur. Altta bir bilgi notu ne açılacağını (katman grupları, varsayılan birimler), gerekirse ikinci not açık çizime ne olacağını söyler. Alt çubukta “Vazgeç” ve birincil “Oluştur”. Kaydedilmemiş değişiklik sorusu pencerenin üstünde açılır; oradaki Vazgeç Yeni proje penceresine döner.

### 7.11 İşlem araçları (pencere ve araç kutusu)

- **Araç kutusu (İşlemler sekmesi):** üstte "Araçlar | Geçmiş (n)" bölümlü seçicisi, altında arama ("İşlem ara: numara, kenar, parsel…"; Türkçe harfler katlanır). Kategoriler ağaç satırıdır (simge, ad, araç sayısı); araç satırında simge ve ad vardır, **tek tıkla** pencere açılır, üzerine gelince simge amber olur. İpucu açıklamayı ve komut satırı takma adlarını gösterir.
- **Geçmiş satırı:** durum simgesi (başarılı yeşil, hata kırmızı, iptal ⚠), araç adı ve saat (tabular), özet (ikincil renk), altta süre ve "n nesneyi seç" (hayalet) ile "Yeniden aç" (küçük) düğmeleri.
- **Pencere:** 940 px, sabit yükseklik; ayar pencereleriyle aynı aile.
  - Sol form grupları: **Girdi**, **Ayarlar**, **Çıktı** ve katlanır **Gelişmiş ayarlar** (sayı rozetli). Grup başlığı ayar grubu gibidir (12 px, 600, alt çizgi).
  - Satır: solda etiket (14 px, 500), isteğe bağlıysa yanında üçüncül "isteğe bağlı", altında açıklama (üçüncül, 12 px); sağda 300 px kontrol sütunu. Nesne girdisi tam genişlikte yığılır: kapsam seçicisi (Seçili, Görünen, Tümü, Katman), canlı sayı ("✓ 68 kapalı alan; seçili nesneler"; boşsa ⚠ ve "Seçili nesneler arasında uygun nesne yok") ve uygun türler.
  - Hata alanın altında kırmızı simge ve metinle, alan kenarlığı kırmızı. Dokunulmamış alan hata göstermez; Çalıştır'dan sonra hepsi gösterilir ve ilk hatalı alana odaklanılır.
  - Sağ panel (`--c-panel-head`): kategori yolu, simge kutusu ve tek cümlelik açıklama (500), yardım paragrafları, kesikli kenarlıklı **Önizleme** kartı, dipte "Nerede çalışır" radyo listesi (Otomatik ve sağında üçüncül "şimdi: bu tarayıcıda"; Bu tarayıcıda; Arka planda (worker); olmayan yerler kesikli halkayla, seçilemez ve "yakında"; seçili radyo amber halka, motor kartlarındaki gibi) ve komut satırı takma adları (mono).
  - Alt çubuk: solda "Varsayılanlar" (hayalet), ortada durum (ilerleme çubuğu, ✓ özet + "Sonuçları seç" + "Geri al", ya da ⚠ düzeltilecek alan sayısı; en çok iki satır), sağda "Kapat" (çalışırken "Durdur") ve tek birincil düğme "▷ Çalıştır".
- **Tür süzgeci:** nesne girdisinde iki ya da daha çok tür varsa sayının altında "Türler" ve her tür için hap düğme ("✓ Kapalı alan 118"). Açık hap yumuşak amber zemin ve amber çizgiyle, kapalı hap üstü çizili ve çizgisiz gösterilir.
- **Alan seçici:** yazılabilir alan ve sağında ok düğmesi; menü nesnelerdeki alanları sayılarıyla listeler. Altında üçüncül bir satır sonucu söyler: "16 nesnede var; değeri değişir." ya da "Yeni alan: nesnelere eklenir."
- **İfade alanı:** tam genişlikte tek satır, mono yazı. Altında solda "Alanlar" ve en çok altı alan hapı (fazlası "+n" menüsünde; tıklamak imlecin yerine ekler, gerekirse köşeli parantezle), sağda "Değişkenler" ve "İşlevler" menüleri (her öğede açıklama satırı). En altta canlı sonuç: ✓ "16 / 340 nesne koşulu sağlıyor." ya da "İlk nesnede (10): “472.26”."; eksik alan varsa ⓘ ile söylenir. Hata varsa önizleme gizlenir, hata alanın altında konumuyla yazar.
- **Arka planda çalışma:** durum satırı "Arka planda çalışıyor; sayfayı kullanmaya devam edebilirsiniz." der; Kapat düğmesi "Durdur" olur. Geçmişte süre satırına ", arka planda" eklenir.
- **Seçim sonucu:** seçim üreten araçta alt çubuk "Seçime yakınlaştır" sunar, "Geri al" sunmaz (belge değişmedi).
- **Nokta parametresi:** değer tabular koordinat olarak yazılır, yanında "Haritadan göster" düğmesi; pencere kapanır, komut satırı "…: haritada bir nokta gösterin ya da Y,X yazın" der, nokta alınınca pencere aynı değerlerle döner.

### 7.12 Model tasarımcısı

- **Pencere:** 1400 px genişlik, ekran yüksekliğinin %92'si. Üç sütun: solda parçalar (236), ortada diyagram, sağda ayarlar (340). Başlık modelin adını ve kaydedilmemişse "•" işaretini taşır.
- **Diyagram zemini** alan tonudur (`--c-field`), üzerinde yakınlaşmayla ölçeklenen nokta ızgarası. Boş alan sürüklenir (el imleci), tekerlek yakınlaştırır; sağ altta Uzaklaş, Yakınlaş, Tümünü göster.
- **Kutular:** panel zemini, güçlü çizgi, 6 px köşe. Girdi kutusu soldan 3 px mavi (`--c-info`) kenar ve mavi simgeyle "sorulan değer" olduğunu söyler; adım kutusu aracın simgesini taşır. İki satır: ad (600) ve üçüncül bilgi ("Girdi: Nesneler", "2 bağlantı"). Seçili kutu amber çerçeve alır; sorunlu adım kesik turuncu kenarla ve ⚠ ile ilk sorununu yazar. Bırakma hedefi amber halkayla gösterilir.
- **Portlar:** kutunun sağında içi boş halka (üzerine gelince amber dolar, artı imleç); adımın solunda küçük dolu nokta. Bağlantılar üçüncül renkte eğridir, ortalarında beslenen parametre yazar (zemin renginde halkalı yazı); seçili kutuya giren ya da çıkan bağlantılar amberdir, sürüklenen bağlantı kesikli amber.
- **Sol:** "Girdi ekle" iki sütunlu düğmeler (mavi simge), araçlar kategori başlıklarıyla; araç satırı sürüklenebilir (tutma imleci, sürüklerken imleci izleyen amber çerçeveli etiket). Dipte kısa bir kullanım notu.
- **Sağ:** üstte simge kutusu, tür başlığı ve açıklama; bölüm başlıkları ayar grupları gibidir. Satırlarda etiket üstte, denetim altta ve tam genişliktedir. Parametre kaynağı açılır listedir; girdi ya da çıktıya bağlıysa kenarı mavidir. Silme düğmeleri kırmızı yazılı hayalet düğmedir.
- **Alt çubuk:** solda "Düzenle" (hayalet), ortada durum, sağda Kapat, "Kaydet ve çalıştır…" ve tek birincil "Kaydet". Kaydedilmemiş değişiklikle kapatırken onay penceresi sorar (§7.9.1); model silme de onay penceresindedir.

### 7.14 Stil pencereleri (stil yöneticisi, sembol tasarımcısı, katman stili)

- **Aile:** model tasarımcısıyla aynı: yan sütunlar `--c-panel-head`, çalışma alanları `--c-field`; semboller çizim alanının "kâğıt" renginde (`--canvas-bg`) çizilir, haritada nasıl görünecekse öyle. Amber yalnızca seçili kart, seçili katman satırı ve odak içindir.
- **Stil yöneticisi:** 1240 px; üstte arama kutusu (simgeli), tür seçici ve sağda Yeni sembol, İçe aktar, Dışa aktar (küçük düğmeler). Sol ağaçta kaynaklar (Sistem kilit simgesiyle), sağda sayı (üçüncül, tabular). Kartlar 132 px'lik ızgara: resim ve en çok iki satır ad; seçili kart amber çerçeve. Aramada karta kaynak rozeti eklenir (Kitaplığım mavi, Proje yeşil). Sağ sütunda büyük önizleme ve altında geometri seçici, ad, tür ve kaynak rozeti ("Sistem · salt okunur"), alanlar (etiket üstte, üçüncül), eylem düğmeleri. Silme onay penceresinde sorulur (§7.9.1). Seçme kipinde tek birincil düğme "Seç"tir.
- **Sembol tasarımcısı:** 1320 px, üç sütun (270 / önizleme / 360). Katman satırında kutu, tür (500) ve özet (üçüncül, tek satır); alt katmanlar 18 px içeride. Önizleme alanı kesikli olmayan ince çerçeveli kâğıttır; üst çubukta örnek geometri seçici ve yakınlaştırma ("1 mm = 4.0 px"). Form satırlarında etiket üstte, denetim altta; iki kısa alan yan yana (`sdf__pair`). "ƒ" düğmesi basılıyken amber tonludur; ifade alanı mono yazılır. Alt çubukta Ad ve Kategori alanları, durum, Vazgeç ve birincil Kaydet (satır içi kipte Uygula).
- **Katman stili:** 980 px; üstte işleyici seçici ve nesne sayıları, altında seçilen türün paneli. Tablolar ince satır çizgili, sayılar sağa dayalı ve tabular. Sembol yuvası 64 × 36 px resim ve altında küçük ad; eksik kitaplık sembolünde kırmızı çerçeve. Kurallar kartlar halindedir, alt kurallar 22 px içeride; ifade hatası kuralın altında kırmızı yazılır. Alt çubukta durum (uygulanmamış değişiklik amber), Vazgeç, Uygula ve birincil Tamam.
- **SVG çizim düzenleyicisi:** 1320 px, üç sütun (210 / çizim / 330). Solda araç düğmeleri (simge, ad, sağda tuş; seçili araç amber tonlu) ve altında şekil listesi (öndeki üstte; göz, kilit (kilitliyken amber, değilken soluk), ad (çift tıkla yeniden adlandırılır), grupta ▣; sürüklenen satır soluk, bırakılacak yer 2 px amber çizgi). Ortada alan tonunda çalışma yüzeyi, üzerinde gölgeli kâğıt ve ince ızgara; üstte ve solda 18 px cetvel (panel başı zemini, üçüncül çentik ve rakam, tuvalin boyu amber çizgi). Kılavuzlar ince mavi (`--c-info`) çizgi, cetvele sürüklenirken kırmızı kesikli (silinecek); çift tıkla açılan küçük kutu açılır menü gibi gölgeli. Seçim kutusu kesikli amber, tutamaçlar 8 px kare, döndürme düğmesi üstte daire; düğümler türüne göre: köşe baklava, yumuşak ve simetrik kare, otomatik daire (seçili dolu); kollar amber daire. Kenet işareti yeşil (`--canvas-snap`) ve ana çizim alanının biçimlerinde (düğüm kare, yumuşak düğüm baklava, parça ortası üçgen, merkez daire, kesişim çarpı, dik açı, teğet üstü çizgili daire), türün adı sağ üstte zemin haleli küçük yazıyla. Önizlemeler (köşe yuvarlama, dizi kopyaları) amber kesikli hayalet ya da soluk kopya; yuvarlanacak köşe amber halka ve yanında "R 3". Ölçü çizgisi amber, uçlarda nokta, değer ortada amber yazı; yolun parça boyları ikincil yazı. Seçilen merkez noktası mavi artı ve halka. Sağda sekmeler (Özellikler | Hizala | Dönüştür | Dizi; uygulamanın sekme biçimi, seçili olanın altında amber çizgi); çokgen aracının ve düğüm aracının ayarları üstte amber tonlu kutuda; basılı düğmeler (düğüm türü, köşe kipi, kilit, sabit nokta ızgarası) amber tonlu. Dokuz noktalı sabit nokta ızgarası 20 px kareler. Çizimin üstündeki çubukta solda dosya grubu (küçük düğmeler: "Dosya ▾" menüsü, Altlık…, Bitmap izle…, Dışa aktar…, Kaynak; Kaynak açıkken amber tonlu), ince bir ayraçtan sonra düzenleme menüleri ("Yol ▾", "Nesne ▾", "Seç ▾"; öğelerde tuş ve açıklama satırı), sağda Kenet (açıkken amber tonlu; yanındaki ▾ türleri işaretli listeler) ve Cetvel anahtarları ile yakınlaştırma; dar pencerede ikinci satıra sarar. Yakınlaştırma etiketi tuvalin gerçek ölçeğini gösterir (sığdır, düğmeler, tekerlek). Tuvale dosya sürüklenince çerçeve amber kesikli olur; bırakılan görüntü için imleçte iki seçenekli menü (İzleme altlığı yap, Bitmap izle…) açılır.
  - **İzleme altlığı:** kâğıdın üstünde, ızgaranın ve şekillerin altında yarı saydam görüntü. Altlık varken çubuğun altında ince şerit (panel başlığı tonu): ad (üçüncül, 600), göz ve kilit simge düğmeleri (basılı durum amber), saydamlık kaydırıcısı ve yüzdesi (tabular), Konum…, Sığdır, "Dosyada sakla" kutusu, İzle…, çöp kutusu. Kilitsizken görüntünün üstünde imleç taşıma imlecidir.
  - **XML kaynağı:** tuvalin altında, yüksekliğin %38'i (üst kenarından sürüklenir). Başlıkta "SVG kaynağı", durum notu (üçüncül; hata kırmızı: "Satır 6, sütun 7: …"), Geri al, birincil küçük Uygula, kopyala ve kapat simgeleri. Solda panel başlığı tonunda satır numaraları, sağda mono yazı (`--fs-xs`, satır 1.55); seçili şekillerin öğeleri amber yumuşak zeminli ve amber çizgili vurgudur.
  - **Dosya pencereleri** (üst üste açılır, `dialog--svgfile`): içe alma 820 px ve dışa aktarma 760 px iki sütun (solda kâğıt renginde önizleme ve altında boyut/ad satırı, sağda form); içe almada renkler küçük renk kutulu "çip"lerdir (seçili amber çerçeve), özet kutusunda alınanlar yeşil onay, atlananlar amber uyarı simgesiyle; PNG saydam zemin önizlemesi dama desenlidir. Bitmap izle 1000 px: solda 480 px önizleme (görüntü %30, iz mürekkep renginde) ve altında sayılar, sağda 320 px kaydırıcılar (etiket üstte, değer sağda tabular, altında tek satır açıklama). Belge özellikleri 500 px, Farklı kaydet 440 px, altlığın konumu 360 px.
- **Üst üste pencere:** sembol seçici ve tasarımcı, açıldıkları pencerenin üstünde durur (`Dialog` `stack`); yalnızca en üstteki tuşları alır, kapanınca alttaki kaldığı yerden sürer.

### 7.15 Dosya alışverişi pencereleri (içe ve dışa aktarma)

- **Aile:** `dialog--io`, bulut pencereleriyle aynı dil: alan etiketi üstte ve ikincil renkte (`--fs-xs`), denetim altında, gerekiyorsa altında üçüncül bir ipucu satırı. Bir satırdaki alanlar üstten hizalanır. Stil dosyası (`styles/io.css`) pencerelerle birlikte yüklenir, başlangıçta değil.
- **Dosya satırı:** en üstte panel başlığı tonunda kutu; açma simgesi, dosya adı (600) ve altında üçüncül bilgi satırı ("3 veri satırı, UTF-8, ilk satır başlık").
- **Koordinat listesi içe aktar** (860 px):
  - Seçenek satırı: Ayırıcı (açılır liste; "Otomatik: boşluk" bulduğunu söyler), Ondalık ayırıcı (Nokta | Virgül), İlk satır (Başlık kutusu), Sütun sırası (Ad Y X Z | Ad X Y Z | Y X Z | X Y Z; eşleşmeyen özel sırada hiçbiri seçili değildir).
  - Önizleme tablosu: alan zemini, 260 px'e kadar, başlık yapışkan. Her sütunun başında rol seçici (Ad, Y (sağa), X (yukarı), Z (kot), Kod, Alınmaz); başlıklı dosyada seçicinin üstünde üçüncül renkte sütunun adı. Satır numarası üçüncül ve sağa dayalı, sayılar tabular; "Alınmaz" sütunu üçüncül. Durum sütununda nokta satırında yeşil onay, nokta olmayan satırda amber ⚠ ve nedeni; o satırın hücreleri üçüncül renkte.
  - Özet kutusu (panel başlığı tonu): yeşil onayla "n nokta alınacak.", amber ⚠ ile nokta olmayan satırlar (ilk beşi numarasıyla), ⓘ ile aynı adlı noktalar ve kapsam ("Kapsam: Y (sağa) … – …, X (yukarı) … – …"), amber ⚠ ile ipuçları (Y ve X yer değiştirmiş olabilir, değerler derece gibi).
  - "Bu koordinatlar hangi sistemde?": datuma göre gruplu açılır liste, projenin sistemi seçili ve ", projenin sistemi" ekli; altında üçüncül not "Koordinatlar olduğu gibi alınır; dönüştürülmez, yuvarlanmaz." Başka bir sistem seçilince uyarı kutusu (**Koordinatlar dönüştürülemez.** …) çıkar ve birincil düğme devre dışı kalır.
  - Hedef katman (Yeni katman ya da var olan katmanlar; kilitliler seçilemez ve "(kilitli)", gizliler "(gizli)" yazar) ve yeni katmanın adı (varsayılan dosya adı).
  - Alt çubuk: solda hayalet "Başka dosya…", ortada durum ("Dosya okunuyor…", hata kırmızı), sağda "Vazgeç" ve birincil "İçe aktar".
- **DXF içe aktar** (900 px):
  - Dosya satırının bilgi satırı dosyanın sürümünü, kodlamasını ve birimini söyler ("Sürüm: AutoCAD 2000 (AC1015), Karakter kodlaması: Windows-1254 (Türkçe), Birim ($INSUNITS): metre").
  - Katman tablosu (önizleme tablosu gibi): başta bütün katmanları seçen ve her satırda katmanı alan onay kutusu, katman rengi örneği ve adı, nesne sayısı (tabular, sağa dayalı) ve "Nereye" sütunu üçüncül renkte: "“Parsel” katmanına eklenir", "yeni katman, gizli" ya da (satır hata tonunda, kutusu devre dışı) "“Parsel” katmanı kilitli; alınmaz. Kilidini Katmanlar panelinden açın.".
  - Özet kutusu: yeşil onayla "n nesne alınacak: 12 çizgi, 3 yay …" ve kaç yeni katmanın dosya adlı grupta kurulacağı; ⓘ ile dönüştürülenler (bloklar patlatıldı, ölçüler çizgi ve yazıya patlatıldı, tarama yayları parçalandı …), amber ⚠ ile alınmayanlar (tür, sayı, neden ve ilk satır numaraları); kapsam satırı.
  - Altında aynı "Bu koordinatlar hangi sistemde?" sorusu; alt çubuk koordinat listesindekiyle aynı.
- **GeoJSON ve Shapefile içe aktar** (900 px): DXF'inki gibi; katman tablosunun başlığı "Katman", "Nereye" yeni katmanda yalnız "yeni katman" der. Koordinat sistemi sorusu özetten önce gelir: dosyanın dediği sistem seçili gelir ve listede ", dosyanın dediği" eklidir; altında dosyanın beyanı üçüncül satırda ("Dosyanın .prj'si: “TUREF_TM36” (EPSG:5256)."). Beyan okunamazsa liste "Sistemi seçin…" der, ⓘ notu seçmeyi ister. Dosyanın dediğinden başka sistem seçilince amber uyarı, koordinatlar seçilen sistemde olamayacak gibiyse (derece aralığı ile metre) ikinci amber uyarı çıkar. Shapefile'da dosya satırı bulunan parçaları söyler (".shp, .dbf, .shx, .prj"), özet kullanılmayan dosyaları; birden çok katmanlı `.zip`'te dosya satırının altında "Arşivdeki katman" açılır listesi (katmanın arşivdeki yolu) ve üçüncül ipucu.
- **GeoJSON dışa aktar** (820 px): DXF dışa aktarınınki gibi kapsam ve katman tablosu ("GeoJSON'da" sütunu: `kentos.layer: “Parsel”`); özet RFC 7946 olup olmayacağını yeşil onay ya da amber uyarıyla, örneklenecek eğrileri, taramaları, halka yönünü, yazılmayanları ve öznitelikleri ⓘ ile söyler.
- **Koordinat listesi dışa aktar** (720 px): Yazılacak noktalar (Seçili | Görünen katmanlar | Tümü, sayılarıyla; boş kapsam seçilemez), Biçim (NCN, TXT, CSV `;`, CSV `,`), Sütun sırası, İlk satır (başlık), Karakter kodlaması (UTF-8 | Windows-1254); özet kutusunda yazılacak nokta sayısı ve kotsuz ya da adsız noktalar. Birincil düğme "Dışa aktar…" kaydetme penceresini açar.

### 7.13 Kontroller (genel)

| Kontrol | Kural |
|---|---|
| Metin alanı | Alan zemini, 1 px çizgi; üzerine gelince güçlü çizgi, odakta amber çizgi. Metin kutularında odak halkası çizilmez; kenarlık yeterlidir. |
| Düğme | İkincil (çizgili), birincil (dolu amber, 600), hayalet (çizgisiz), küçük (28 px). |
| Not kutusu | Bilgi (mavi simge) ya da uyarı (amber zemin tonu, ⚠). İlk ifade kalın olabilir: "**Koordinatlar dönüştürülmez.**" |

---

## 8. Çizim alanı görsel dili

| Öğe | Görünüm |
|---|---|
| Seçim | Amber, **kesikli** çizgi (6/3 px); çokgenlerde %13 amber dolgu; köşelerde 6 px amber tutamaç (9 px'ten yakınları seyreltilir) |
| Üzerine gelme | Amber %85, **düz çizgi, dolgusuz**. Dolgu büyük alanlarda titreşim yarattığı için kullanılmaz. |
| Kenet işareti | Yeşil (`--canvas-snap`). Uç nokta: kare. Orta nokta: üçgen. Merkez ve nokta: daire. Çeyrek: eşkenar dörtgen. Kesişim: çarpı. Dik: dik açı işareti. Teğet: üstünde yatay çizgi olan daire. En yakın: kum saati. Adı işaretin sağ üstünde yazar, ölçü etiketiyle çakışmaz. |
| Sıcak tutamaç | Düzenlenen tutamaç diğerlerinden büyük (8 px), mürekkep renkli ve amber çerçevelidir. Hayalet şekil amber kesikli çizilir; eski yerden yeni yere ince kesikli bir bağ çizgisi uzanır. |
| Kenar ortası tutamacı | Çoklu çizgi ve alan kenarlarının ortasında 8 px, içi zemin renginde, amber çerçeveli baklava. Köşe tutamacından (dolu kare) ayırt edilsin diye biçimi farklıdır; ekranda 28 px'ten kısa kenarlarda gösterilmez. |
| Köşe ekle/sil önizlemesi | Eklenecek yerde amber artı, silinecek köşede kırmızı (`danger`) çarpı ve imleç yanında "Köşe ekle" / "Köşeyi sil" etiketi. |
| Değiştirme önizlemesi | Taşı, kopyala, döndür, ölçekle, aynala, dizi, ötele ve uzatma sonuçları **amber kesikli hayalet** olarak gösterilir; dizi için tüm kopyalar (en çok 400). |
| Budama önizlemesi | Hedef nesnenin tamamı **kırmızı kesikli** (`--c-danger`), kalacak parçalar üstünde **düz amber** çizilir: kırmızı kalan kısım silinecek olandır. Kırmızı çizim alanında yalnızca bu anlamda kullanılır. |
| Kutupsal izleme | Son noktadan ekran boyunca uzanan ince kesikli amber ışın; imlecin üstünde "Kutupsal 45°" etiketi. |
| Artı imleç | Seçimde kısa kollar ve 10 px seçim kutusu; çizimde uzun kollar. Boyutu ayarlardan seçilir (küçük, orta, tam ekran). İşletim sistemi imleci çizim alanında gizlenir. |
| Ölçü etiketi | İmlecin sağ altında, amber çerçeveli küçük kutu: uzunluk, semt, gerekirse alan ya da toplam |
| Pencere seçimi | Soldan sağa: mavi, düz kenarlı. Sağdan sola: yeşil, kesik kenarlı (kesişim). |
| Etiketler | Zemin rengi haleli (3 px) Barlow. Ada numarası "1244 ada" (600, yakınlaştıkça büyür, çok yakında gizlenir). Parsel numarası (500). Nokta adı ve kot sağ üstte. Eşyükselti değeri çizgi boyunca ve dik okunur. Sokak adları italik ve dünya biriminde yükseklikte. |
| Ölçü | İnce uzatma çizgileri (ölçülen noktadan küçük bir boşlukla başlar), ölçü çizgisi ve iki ucunda 45° eğik kısa çizgi. Değer çizginin üstünde, çizgiye paralel ve her zaman soldan sağa okunur; birim yazılmaz, hassasiyet proje ayarındandır. |
| Tarama | Çizgili (45° varsayılan), çapraz ya da dolu. Çizgi aralığı kâğıt mm cinsinden verilir (3 mm × çizim ölçeği). Dolu tarama katman renginin %45'i saydamlıktadır. |
| Kenar ölçüleri | Parsel kenarının ortasında, halkanın **dışında**, kenara paralel ve okunur; 2 mm kâğıt yüksekliği. Yazı yüksekliğinin üç katından kısa kenarlara yazılmaz. |
| Yerinde düzenleme | Çift tıklanan yazının üstünde, aynı açı ve boyutta, amber çerçeveli yarı saydam bir kutu. Enter kaydeder, Esc vazgeçer; görünüm kayarsa değişiklik kaydedilip kapanır. |
| Nokta simgeleri | Poligon noktası: üçgen. Kot noktası: artı. Genel nokta: halka ve merkez noktası. Ekran boyutu sabittir. |
| Çizgi tipleri | Sürekli; kesikli 9/5; noktalı kesik 14/4/2/4; noktalı 2/4 (ekran pikseli) |
| Izgara | Uyarlamalı 1-2-5 aralık, ana çizgiler yuvarlak TM değerlerinde |
| Ölçek çubuğu | **Sağ altta**, dört bölmeli siyah-beyaz çubuk, "0" ve "50 m". Araç kutusu solda durduğu için sağdadır. |
| Kuzey oku | Sağ üstte, yarısı dolu ok ve **"K"** (Kuzey) |

---

## 9. Etkileşim kalıpları

| Girdi | Davranış |
|---|---|
| Harf / Shift+harf / Alt+harf | Araç (tuş etiketi araç kutusunda) |
| Esc | Araçtan çık; seçim aracındaysa seçimi temizle; metin kutusunda önce metni temizle |
| Enter, sağ tık | Geçerli nesneyi bitir; seçim aracında son aracı tekrarla |
| Boşluk | Komut satırına git |
| Rakam, `@`, `.` | Komut satırına odaklanır ve koordinat yazılmaya başlanır |
| Orta tuşla sürükleme | Kaydır (her araçta). Orta tuşa çift tıklama: tümünü göster. |
| Tekerlek | İmleç etrafında yakınlaştır ya da uzaklaştır |
| Shift (çizimde) | Ortoyu geçici olarak tersine çevirir |
| Shift+tık (seçimde) | Seçime ekle ya da çıkar |
| Tutamaç sürükleme | Seçili nesnenin köşesini taşır. Tıklayıp bırakmak tutamacı sıcak yapar; sonraki tıklama yerleştirir, Esc vazgeçer. |
| Kenar ortası tutamacını sürükleme | Düz kenara yeni köşe ekler, yay kenarını sürüklenen noktadan geçecek biçimde büker. |
| Değiştirme araçlarında sayı | Aşamaya göre açı (Döndür), faktör (Ölçekle), mesafe (Ötele), yarıçap (Köşe yuvarla), "satır,sütun" ve "dY,dX" (Dizi) |
| Değiştirme araçlarında harf | `K` kopya (Döndür), `S` kaynağı sil (Aynala). Seçeneğin güncel durumu istem metninde yazar: "[Kopya (K): kapalı]" |
| F1 / F2 / F3 / F4 / F7 / F8 / F9 / F10 | Kısayollar / alt panel / kenet / sağ panel / ızgara / orto / araç kutusu / kutupsal |
| `Ctrl+F1`, sekmeye çift tık | Şeridi daralt ya da aç |
| `Alt+Q` | Komut ara (şeritte arama kutusu, klasik arayüzde komut satırı) |
| `Ctrl+,` | Uygulama ayarları |

- **Odak:** Görünür odak halkası 2 px amber'dir. Programla odaklanan kapsayıcılarda (pencere kartı, çizim alanı) halka çizilmez.
- **Yeniden çizimde odak korunur.** Ayar bölümü yeniden kurulduğunda aynı etiketli kontrol tekrar odak alır.
- **Seçim önceliği:** nokta ve kenar önce gelir, sonra içindeki en küçük çokgen. İçine tıklanınca seçilmemesi gereken çerçeveler (pafta) `pickInterior: false` taşır.

---

## 10. Metin ve dil

### 10.1 Üslup

- Yalın, etken çatı, günlük Türkçe. Düğme ne yapacağını söyler: "Kaydet", "Seçileni varsayılan yap", "Proje ayarlarını aç". Sonuç mesajı aynı fiili kullanır: "Ayarlar kaydedildi."
- **Hata ve uyarılar** ne olduğunu ve nasıl düzeltileceğini söyler, özür dilemez:
  - "“XYZ” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın."
  - "EPSG:1234 bu sürümde tanımlı değil. Listedeki sistemlerden birini seçin."
- **Boş durumlar** yönlendirir: "Seçili nesne yok. Özelliklerini görmek için çizimde bir nesneye tıklayın…"
- **Yapılmamış özellik:** "“DXF / DWG…” bu sürümde henüz kullanılamıyor." ya da "Geliştirme aşamasında".
- Tırnak olarak Türkçe tipografik tırnak “…” kullanılır.

### 10.2 Terimler

| Terim | Kullanım |
|---|---|
| Y (sağa) / X (yukarı) | Koordinat eksenleri. Her zaman Y önce yazılır. |
| Semt | Kuzeyden saat yönünde doğrultu açısı (grad) |
| Ada / parsel | "1244 ada 7 parsel", etiketlerde "1244 ada" ve "7" |
| Pafta | Harita paftası; çerçeve ve adı |
| Kot / eşyükselti / ana eşyükselti | Yükseklik noktası / kontur / her beşinci kontur |
| Poligon noktası | Ölçü ağının kontrol noktası |
| Aplikasyon, ifraz, tevhid | Arazide işaretleme, bölme, birleştirme |
| Kenetleme / kenet | Nesne yakalama (object snap) |
| Katmana göre | "ByLayer" karşılığı |
| Proje ayarları / Uygulama ayarları | Kapsamı ayrı iki pencere; asla yalnızca "Ayarlar" denmez |

### 10.3 Sayılar

- Ondalık ayırıcı **nokta**: `486512.340`, `12997.30 m²`. Komut satırı girdisiyle birebir aynıdır.
- Hassasiyet proje ayarındandır (varsayılan: uzunluk 3, alan 2 basamak, semt 4 basamak).
- Birim değerden bir boşlukla ayrılır ve üçüncül renktedir: `23.412 m`, `0.78 dönüm`, `132.4521 g`.
- Ölçek `1:1000` biçiminde yazılır. Ekran ölçeğinde binlik ayırıcı kullanılır: `Ekran 1:2.470`.

---

## 11. Erişilebilirlik

- Bütün kontroller klavyeyle ulaşılabilir ve rolleri doğrudur: `menubar`/`menu`/`menuitem`, `tree`/`treeitem`, `tablist`/`tab`, `radiogroup`/`radio`, `switch`, `listbox`/`option`, `dialog`.
- Yalnızca simgeden oluşan her düğmenin `aria-label`'ı ve ipucu vardır.
- Metin kontrastı her iki temada WCAG AA'yı hedefler. Üçüncül metin yalnızca yardımcı bilgi içindir.
- Durum hiçbir zaman yalnızca renkle anlatılmaz: gösterge lambası ve etiket, uyarı simgesi, onay işareti gibi ikinci bir işaret bulunur.
- `prefers-reduced-motion` açıksa bütün geçişler kapanır.

---

## 12. Hareket

- Hareket yalnızca kullanıcı eylemine cevap verir. 80–160 ms, `ease` varsayılan: ipucunda saydamlık, switch başparmağı, panel oku dönüşü, durum mesajının belirmesi.
- Giriş animasyonu, kayarak beliren bölüm ya da her kartta ayrı hover geçişi yoktur.

---

## 13. Yapılmayacaklar

- Tek bir krem, terrakota ya da neon yeşil vurgu; genel "SaaS kartı" görünümü; aynı köşe yarıçapı ve gölgenin her yerde kullanılması.
- BÜYÜK HARF etiketler, harf aralığı açılmış üst başlıklar, "A · B · C" biçimli meta satırları, bağlantı sonuna "→".
- Veri etiketlerinde mono yazı (mono yalnızca komut satırındadır).
- Arayüzde sabit renk ya da sabit px yazı boyutu; katman adına göre görsel özel durum.
- Birden fazla dolu amber düğme; amber'i uyarı ya da süsleme için kullanmak.
- Sessizce hiçbir şey yapmayan düğme; açıklamasız devre dışı öğe.
- Mobil kırılım noktaları ve çekmece desenleri.
