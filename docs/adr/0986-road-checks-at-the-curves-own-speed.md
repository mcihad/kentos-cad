# 0986 — Denetimler kurbun kendi proje hızıyla

Tarih: 2026-11-06. Durum: kabul (Y80). Kapsam ROAD-ALIGN-11 (kısmen): hızı kullanan denetim ve önerilerin hızı
projeden okuması. ADR 0985'in §Açık'ı.

## Sorun

Y79 aksın proje hızını ve kurbun kendi hızını saklıyor. Ama hızı kullanan dört komut onu hâlâ her çağrıda parametre
olarak istiyordu:

- `road.superelevations.check` (yanal sürtünme);
- `road.superelevations.transitions` (konforlu klotoid);
- `road.superelevations.widen` (genişletmenin hız terimi);
- `road.profiles.lengths` (durma görüşü ve konfor).

Ayrıca verilen tek hız bütün kurplara uygulanıyordu. Oysa KGM'nin örneğinde aynı yolda bir kurp 110, kavşak kurbu 50
km/sa'tir.

## Araştırma

- **KGM 2019 şartnamesi** s. 5: "Yatay kurblar proje hızına göre seçilmelidir"; "Maksimum deverler proje hızı, kurb
  yarıçapı, kavşak yerleri … dikkate alınarak". Hız kurbun bir özelliğidir.
- **AASHTO Green Book** (yanal sürtünme, nokta kütle dengesi): her kurp kendi tasarım hızında denetlenir.
- **Civil 3D:** "Design Speeds" tablosu km aralığıyla hız verir; tasarım denetimleri ve dever hesabı her kurbu
  bulunduğu yerdeki hızla okur.
- **OpenRoads:** aksın tasarım standardı ve hızı, kurp başına geçersiz kılınır.
- **Matematik değişmedi:** formüller ADR 0931, 0932 ve 0935'tekilerdir. Değişen yalnız her kurba hangi V'nin
  verildiğidir.

## Karar

- **Hızın kaynağı:** `speed` verilirse bütün kurplar için odur (bugünkü davranış, geriye uyumlu). Verilmezse her kurp
  kendi hızıyla okunur: PI'nın `speed`'i, yoksa aksın `designSpeed`'i (paralel aksta kaynağınınki;
  `road_alignments::curve_speeds`).
- **Dever denetimi:** sürtünme her ayrı hızda hesaplanır, satırlar o hızdaki kurplardan alınır, km sırasıyla dizilir.
  Bulgu iletisi kurbun kendi hızını yazar ("p1 kurbu 70 km/sa'te …").
- **Konforlu klotoid ve genişletme:** her kurp kendi hızıyla.
- **Boykesit uzunlukları:** `speed` isteğe bağlı oldu. Verilmezse boykesitin aksının `designSpeed`'i kullanılır.
  Düşey kurp bir PI'ya bağlı değildir, bu yüzden kurbun kendi hızı kullanılmaz.
- **Retler:** hız hiçbir yerde yoksa (ne `speed` ne kurbun ne aksın hızı) ve ölçüt hız istiyorsa ret verilir.
  İleti iki yolu da söyler: "… proje hızını (speed) verin ya da aksa proje hızı yazın."
- **Çıktı sözleşmeleri değişmedi:** satırlara hız alanı eklenmedi. Eklense bütün eski durumların çıktısı değişirdi.
  Hız ileti metninde yazar.
- **Masaüstü:** Dever penceresinin Denetim'inde hız alanı boşken, aksta hız varsa yer tutucu "kurpların kendi hızı"
  der. Boykesit uzunlukları penceresinde boş hız artık reddedilmez, aksınki kullanılır.

## Ölçüm

- **Ortak durumlar** (KentOS kodu yok; bağımsız başvurular aynı fizikle):
  - dever denetimi: hız verilmez, aksın 70'i; kurbun 50'si aksın 90'ını geçer; verilen 80 kurbun 50'sini geçer;
    hızsız ret;
  - konfor: hız verilmez, aksın 80'i;
  - genişletme: hız verilmez, aksın 80'i;
  - boykesit uzunlukları: hız verilmez, aksın 80'i; hızsız ret.
  - 24 + 29 + 28 + 23 durum geçti.
- **Uygulama testi** (`tests/all/design_speeds.rs`): üç kurplu g-5'te aks 80, p2 50. p1 ve p3'ün satırları 80 km/sa,
  p2'ninki 50 km/sa ile verilen denetimin satırlarıyla aynı. Bulgular hızlarını yazar. Verilen 80 hepsine uygulanır.
- **Testler:** masaüstü 1853 (`RUST_MIN_STACK=67108864`; boykesit uzunlukları penceresinin testi yeni sırayla: önce
  ölçüt, sonra hız), uygulama, plot, yol, KCAD, sözleşmeler ve komutlar 4106. Python SDK 545'ten 544: main'den gelen
  `file.package`'ın koşucusu yok (ADR 0985).
- **Resim** `.run/gui-evidence/yol-y80/`: Dever penceresi, hız boş ve aks 70 km/sa. Sürtünme 0,194: 70 km/sa'te
  v²/(gR) = 0,257, %6 deverle (0,257 − 0,06)/(1 + 0,257·0,06) = 0,194 (elle). Bulgu "p1 kurbu 70 km/sa'te".

## Açık

- **Km aralığıyla hız** (Civil 3D'nin Design Speeds tablosu, LandXML `DesignSpeed`).
- **`road.alignments.check`'in yarıçap, klotoid ve aliyman kuralları** kullanıcının değerleriyle çalışır, hızdan
  türetilmez: standardın tablosu okunamadı (ADR 0985).
- **Çıktı satırlarında hız alanı.**
