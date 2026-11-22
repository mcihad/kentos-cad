# ADR 9115: Veri katmanının çizgileri model ekranında kâğıt kalınlığını korur

- **Durum:** kabul edildi (2026-11-22). Plan: [plan/veri.md](../../plan/veri.md) VY-18 (ikinci tur).
- **İstek:** sahibin sözü (21 Kasım 2026): “Hepsini düzelt eksikleri tamamla.” Kapatılan kalan: ADR 9101 §10, “Küçük
  ölçekte ince çizgiler: kâğıt kalınlıkları yüzünden çizgiler soluk görünüyor; ölçeğe bağlı kalınlık stil işi.”
- **Sözleşmeler:**
  - Stil çekirdeğinin programına isteğe bağlı `screenWidths` eklendi (`Env::screen_widths`). Yazılmadığında her şey
    eskisi gibidir.
  - Yerelde `BuildOptions::screen_widths` eklendi.
  - Gölgelendirici, GPU düzen sözleşmesi (`styled.layout.json` 10), `.kcad` ve komutlar değişmedi.
- **Bağımlılık:** yok.

## §1 Sorun

KentOS'ta stilin `mm` birimi kâğıt milimetresidir: projenin çizim ölçeğinde 1 mm = ölçek/1000 metre (STYLE.md §3.4).
Model ekranı yakınlaşınca kâğıt kalınlığını tutar (ADR 0793 §1), ama uzaklaşınca çizgi dünyayla birlikte incelir.

- 1:1000'de 0,7 mm'lik bir ana yol 0,7 metredir.
- Bütün kente bakarken (1:200 000) bu 0,013 piksele iner. Çizici en az bir piksel çizer, ama piksel altı bir
  çizginin kenar yumuşatması onu iki piksele yarı yarıya dağıtır: çizgi soluk görünür.
- Bir milyon parselin renkleri arasında yollar seçilmiyordu (VY-03'ün ilçe ekranı).

CAD çiziminde uzaklaşınca kâğıdın küçülmüş hâlini görmek doğrudur. Bir CBS katmanında ise çizgi her ölçekte okunur
kalmalıdır.

## §2 Araştırma

- **QGIS:**
  - Sembol boyutlarının milimetresi ekran milimetresidir: 0,26 mm'lik bir çizgi her ölçekte aynı kalınlıktadır.
  - Harita birimi (metre) yakınlaşmayla büyür; en küçük ve en büyük boyu milimetreyle sınırlanabilir.
  - 3.22'den beri katmana bir “referans ölçek” verilebilir. O zaman milimetre boyları kâğıt gibi o ölçeğe göre büyüyüp
    küçülür. Bu, KentOS'un çizim katmanının davranışıdır.
- **MapLibre:** çizgi kalınlığı ekran pikselidir. Yakınlığa göre değişmesi istenirse bir ifadeyle (`interpolate`)
  açıkça yazılır.
- **Kâğıt:** harita işaretlerinin kalınlıkları kâğıt milimetresidir ve çıktıda öyle kalmalıdır. Pafta ve PDF bu
  kararla değişmez.

## §3 Karar

Model ekranında bir veri katmanının çizgileri kâğıt kalınlıklarını her ölçekte korur:

- **Nasıl:** stil çekirdeği bir çizginin kâğıt milimetrelerini (kalınlığını, kesiklerini, yumuşatmasını) gölgelendiriciye
  CSS pikseli olarak verir (`screenWidths`). 0,7 mm'lik çizgi her ölçekte 2,65 pikseldir.
- **Yakınlaşınca** sonuç değişmez: model ekranı orada kalınlığı zaten kâğıt kalınlığında tutuyordu (ADR 0793 §1).
- **Uzaklaşınca** çizgi artık dünyayla incelmez.
- **Kapsam:**
  - Yalnız veri katmanları ve yalnız model ekranı (`hold_weights` açıkken).
  - Çizim katmanlarının CAD davranışı aynı kalır.
  - Pafta ve PDF (`hold_weights` kapalı) kalınlıkları eskisi gibi dünyada bırakır.
- **Çizgi değil:** işaret boyları ve taramalar kâğıtta kalır. İstek çizgiler içindi. Bütün sembolleri ekranda sabit
  isteyen kullanıcı Uygulama ayarları › Sembol boyutu › Ekranda sabit'i seçer.
- **Metre:** metreyle verilmiş bir kalınlık (yolun gerçek genişliği gibi) yerde kalır.
- **Düzenlemedeki satırlar:** düzenleme oturumunda çizime alınan satırlar çizimin kendi sahnesinde çizilir. Onlar da
  aynı kuralı alır, yoksa tablodan okunanlardan ince görünürlerdi.

## §4 Planda olmayan kararlarım

1. **Gölgelendirici değil çekirdek.** İlk düşünce gölgelendiricideki kalınlık çarpanına (`weightK`) “uzaklaşınca da
   tut” bayrağı koymaktı. Bu, paylaşılan WGSL'yi, düzen sözleşmesinin sürümünü, CPU ikizini ve kırka yakın yapı
   yazımını değiştirirdi. Kalınlığı derlerken piksele çevirmek model ekranında aynı sonucu verir.
2. **Kesikler de piksele geçer.** Kesikler kalınlıkla aynı birimdedir. Ekranda sabit bir çizginin deseni de ekranda
   sabittir, QGIS'teki gibi.
3. **İşaretler ve taramalar kâğıtta kaldı** (§3).

## §5 Kanıt

**Sınamalar:**

- Stil çekirdeği:
  - bayrakla 0,7 mm'lik çizgi ve kesikleri pikseldir;
  - metre kalınlığı yerde kalır;
  - bayraksız kâğıt kalınlığı dünyadadır (1:1000'de 0,7 m).
- Masaüstü (gerçek okuma yolu, GeoPackage'taki 40 parsel):
  - model ekranında veri katmanının kenar çizgileri piksel ve katmanın kalemi kadardır (0,18 mm = 0,68 px);
  - kâğıtta dünyadadır.
- Stil ve çizim crate'lerinin bütün sınamaları ve masaüstünün sahne sınamaları geçti.

**Gerçek pencere:** bir milyon PostGIS parseli ve GeoPackage'tan yollar. Önce ve sonra aynı senaryo, aynı ölçekler.
Görüntüler `.run/gui-evidence/veri/vy18/{once,sonra}`.

- **Bütün kent (1:200 726):** önce yol ızgarası parsellerin içinde kayboluyordu. Sonra ana yollar belirgin kırmızı
  çizgiler, tali yollar daha ince turuncu çizgiler.
- **İlçe (1:18 209):** önce yollar neredeyse görünmüyordu. Sonra kırmızı ana yollar parsellerin üstünde net.

## Uygulama

- Stil çekirdeği:
  - `crates/shared/style-core/src/style/compile.rs`: `Env::screen_widths`, `stroke_style`;
  - `build.rs`: `screenWidths`;
  - sınama `style/tests.rs`.
- Yerel:
  - `crates/native/style/src/program.rs`: `BuildOptions::screen_widths`, program JSON'u;
  - `crates/native/plot/src/layers.rs`: veri katmanı ve model ekranı.
- Masaüstü:
  - `apps/desktop/src/cad/style/scene.rs`: düzenlemedeki satırlar;
  - sınama `gis/data_layers_tests.rs`.
- Öteki `BuildOptions` ve `Env` yazımları: bayrak kapalı.
