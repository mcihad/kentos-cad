# 0658 — Mimari şerit ve dar CAD durum çubuğu

24 Ekim 2026 · `codex/architecture-2d-workflows` · ARCH-UX-02.
Durum: ARCH-UX-02 sunum dilimi kabul edildi; diğer mimari işler planda açık.

## Karar

- Mimari sekmesinin sekiz grubu **Çiz / Yerleştir / Düzenle / Belgele / Proje**
  sırasına toplanır. Eski komutlar korunur; sık duvar/bölme/kapı/mahal işleri
  yazılı düğmelerdedir. Seyrek işleri grubun adı yanındaki menü taşır.
  `scripts/ui/ribbon.py` yerleşimin tek kaynağıdır.
- UI `Group::labels(true)` yalnız bu sekme ve mimari bağlamsal sekmelerde
  kullanılır. Adlı küçük düğmeden yazısız ikon seviyesine inmez; sonraki adım
  adını taşıyan katlanmış grup menüsüdür. Ölçülen genişlik ve çizilen seviye
  aynı kurala uyar. Diğer sekmelerin yerleşim politikası değişmez.
- Katlanan grubun komutları, ek menüsü, arama vurgusu ve klavye harf ipuçları
  mevcut yollarını korur. Yeni bir çizim/düzenleme komutu kopyalanmaz.
- Görüntü incelemesi 1100 px / 18 px'de bağlamsal sekme adının sağ kenarda
  kaybolduğunu gösterdi. Dar mimari görünümde yalnız sekme adları yatay
  kaydırılabilir bir alana alınır; marka/hızlı erişim ve arama/yardım uçları
  sabit kalır. Sonlara eklenen Mimari/bağlam sekmesi başlangıçta görünürdür;
  etkin başlık veya pencere eni değişince gerçek yerleşim sınırlarından o
  başlık açığa çıkarılır. Ardından elle kaydırma serbest kalır. Diğer ana sekmeler ve
  CBS eski başlık düzenini kullanır. Yalnız grup genişliğinin sığması başlık
  satırının sığdığına kanıt sayılmaz.
- Aynı menüde belirsiz iki “Numarala” yerine “Aksları numarala” ve
  “Açıklıkları numarala” adları kullanılır.
- Duvar, Açıklık ve Mahal bağlamına **Özellikler** eklenir. `view.properties`
  mevcut Öznitelikler panelini gösterir; kapatıp açan F4'ten ayrıdır. Belgeyi,
  seçimi ve sürmekte olan aracın taslağını yazmaz. En/menteşe/ad/numara gibi
  alanlar mevcut özellik düzenleyicisini ve anlamsal komutlarını kullanır.
- CAD durum çubuğunun önceki yedi daralma adımı yetmediğinde sekizinci adım
  on bir yardımcının yerine **Yardımcılar** menüsünü koyar. Bütün anahtarlar,
  kısayollar, Kenet/Çakışma/Süzgeç alt seçenekleri ve topoloji nokta seçeneği
  erişilebilir kalır. Kontroller `RunKept` yoluyla etkin taslağı korur.
  Çizim ölçeği, koordinat ve kayıt/bağlantı hücrelerine yer kalır. CBS'nin
  durum çubuğunun adımları değişmez.
- Bu bir sunum dilimidir. SI/birim/ölçek hesabı, FP64 model koordinatları,
  pan/zoom/render motoru ve mevcut mimari geometri komutları değişmez.
  `view.properties` panel açan host eylemidir; Python/MCP'ye geometri yazması
  gibi sunulmaz. Ortak CAD manifesti bu ayrımı kaydeder.

## Kabul

Ana şerit 13/18 px ve bağlamlar 18 px, iki genişlik ve iki tema GPU
matrisinde üretildi. Gerçek pencerede grup/araç tıklaması, seçili açıklığın
kapalı panelini gösterme, elle kaydırılmış başlıktan Escape dönüşü ve
Yardımcılar menüsünün etkin taslağı koruması doğrulandı. Metin ölçümü ve
çizimi aynı shaping kullanır; dar menü adı satıra bölünmez.
1269 masaüstü ve 278 UI testi; son 7 hedef testi, clippy, ortak WASM ve
katalog/şerit/manifest/bağımlılık kontrolleri geçti. Web ADR 0228 gereği bekler.

İlerleme ve gerçek ekranların sürüm ayrımı [kabul kaydındadır](../baseline/2026-10-24-architecture-ribbon/README.md).
