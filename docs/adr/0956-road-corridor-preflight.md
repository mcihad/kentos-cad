# 0956 — Koridor ön denetimi: katlanma, kendi üstüne binme, bölge dikişi

Tarih: 2026-11-05. Durum: kabul (Y50). Kapsam ROAD-COR-13.

## Sorun

Koridorun üst yüzeyi 2,5B bir üçgen ağıdır: her yerde tek bir kot olduğu varsayılır. Bu varsayımı üç durum bozar:

1. **Katlanma.** Dar kurpta iç taraftaki şev, kurbun yarıçapından uzağa uzanır. Kesitler birbirini keser ve yüzey
   kendi üstüne katlanır.
2. **Uzak üst üste binme.** Serpantinde ya da dar dönüşte yolun iki ayrı parçasının şevleri planda üst üste biner.
3. **Bölge dikişi.** İki bölgenin farklı tip kesitleri aynı km'de buluşmaz: kotta ya da planda sıçrama olur, bir
   kod yalnız bir tarafta kalır.

Önceki durum ve neden yetmediği:

- Ardışık kesitlerin planda kesişmesi zaten yakalanıyordu: ters dönmüş üçgenli şerit atlanıyor ve `corridor_folded`
  uyarısı veriliyordu.
- Ama uzak parçaların üst üste binmesi ve bölge dikişleri hiç denetlenmiyordu.
- Hacim (ADR 0906) ve Brükner bu yüzeye güvendiği için bu durumlar sessizce iki kez sayılabiliyordu.

## Araştırma

- **Civil 3D:**
  - Bu duruma "corridor bowtie" denir.
  - Dar iç köşelerde şev izdüşüm çizgileri üst üste biner. "Clear Corridor Bowtie" komutu bunu temizler,
    "Restore Corridor Bowties" geri getirir.
  - Otomatik temizlik köşedeki kurplarda çalışmaz; kesitler köşede birbirini keserek kurulabilir.
  - Koridor kendi üstüne binince sınır (shrinkwrap) bozulur ve yüzey yanlış olur. Yüzeyin "Overhang correction"
    seçeneği de vardır.
  - Kaynaklar: Autodesk Civil 3D Best Practices ("slope projection lines may overlap"), Civil 3D forumları,
    Noble Desktop "Dealing with corridor bow ties".
- **OpenRoads:** Kırpma şekilleri (clipping) ve "key station"lar kullanır.
- **İkisinde de:** Sorunun yerini ve büyüklüğünü sayıyla söyleyen bir ön denetim bulunamadı.

## Karar

`kentos_road::corridor_check` modülü ve `road.corridors.preflight` komutu (salt okur) üç denetim yapar.

### 1. Katlanma payı

Kesit noktası P = C(s) + o·N(s)'dir. N sağ normaldir ve ofset sağa pozitiftir. dN/ds = κ·T olduğundan (κ sola dönüşte
pozitif):

dP/ds = (1 + κ·o)·T

Eşleme, 1 + κ·o ≤ 0 olduğu yerde, yani iç taraftaki nokta yarıçaptan uzaksa katlanır (Jacobian işaret değiştirir).

- **Kesitin payı:** Üst çizgi noktalarının en küçük 1 + κ·(d + o) değeri. d, aksın kendi ofsetidir.
- **Anlamı:** Pay aliymanda 1'dir, katlanmada 0.
- **Eklemler:** κ istasyonun hemen öncesinde ve sonrasında okunur (±10⁻⁹ m), küçük pay alınır. Kurbun aliymanla
  buluştuğu km iki elemana da aittir. Ölçüm: R = 4 m kurbun sonu (km 42,283) Rust'ta aliymanı, Python'da kurbu
  okuyordu; iki yanlı okumayla ikisi de kurbu okuyor.
- **Uyarı:** `margin` verilirse onun altındaki kesitler de `corridor_tight` olarak söylenir. Verilmezse yalnız
  katlananlar (pay ≤ 0) söylenir; KentOS öntanımlı bir pay vermez.
- **Örneklenmiş karşılığı:** Ardışık kesitlerin kesişmesi (`folded`) aynı olgunun ayrık biçimidir; ikisi birlikte
  raporlanır.

### 2. Kendi üstüne binme

Km paylaşmayan her iki şeridin üçgenleri planda birbirine kırpılır (ikisi de dışbükey, Sutherland–Hodgman). Her
parçanın ağırlık merkezinde iki düzlemin kotu okunur. Sonuç:

- örtüşme alanı;
- iki yüzey arasındaki en küçük ve en büyük kot farkı. Yolun yolun üstünden geçtiği yerde bu fark büyüktür,
  katlanmada sıfıra yakındır.

Hem ilk hem ikinci şeritleri birbirine değen çiftler tek bir bulguda birleştirilir. 10⁻⁶ m²'den küçük parça yuvarlama
sayılır.

### 3. Bölge dikişi

Bitişik iki bölgenin aynı km'deki (1 mm içinde) kesitleri kod kod karşılaştırılır:

- planda en büyük kayma ve onu veren kod;
- kotta en büyük sıçrama ve onu veren kod;
- yalnız bir tarafta olan kodlar.

Dikiş, kayma ve sıçrama `tolerance` (öntanımlı 1 mm) içindeyse ve tek taraflı kod yoksa buluşmuş sayılır; aksi hâlde
`corridor_seam` uyarısı verilir.

### Sonuç

- Çıktı `clear` alanıyla biter.
- İletilerde kodlar Türkçe adlarıyla yazılır ("sağ şev başı", "sol hendek ayağı", "sağ 3. bileşen"). Veri alanlarında
  kodlar olduğu gibi kalır.
- Koridor penceresine "Görünüş: Ön denetim" eklendi.
  - Planda katlanan kesitler kırmızı, dar olanlar turuncu, üst üste binen parçalar mor çizilir.
  - Dikiş çizgisi ve kot sıçraması, koridorun üstünde okunsun diye zemin renkli bir kutuyla yazılır.
  - Altta bir cümlelik özet ve bulgu tablosu durur.

## Ölçüm

Bağımsız referans `scripts/fixtures/road_corridor_preflight_command_cases.py`, KentOS kodu kullanmaz:

- kesitler `road_corridor_command_cases.py`'nin;
- eğrilik aksın mpmath referansından;
- örtüşmeler GEOS (shapely) ile.

24 durumun hepsi Rust'ta ve Python'da geçer. Ölçülenler:

| Koridor | Bulgu |
|---|---|
| Ana koridor (R 150) | En küçük pay 0,931: iç taraftaki en uzak nokta −10,29 m'de; 1 − 10,29/150 = 0,931. %95 payda km 1+234,713 – 1+300 dar. |
| Ana koridor, km 300'deki tk-1/tk-2 dikişi | Kotta 0,088 m (sağ şev başı), planda 3,5 m. Hendek kodları yalnız öncekinde, sağ 3. bileşen yalnız sonrakinde. Aynı tip kesitle 0 / 0, buluşur. |
| Dar kurp (R 4, 40 m şerit) | Pay −1,006 (−8,03 m ofsette). Katlanma km 0+036 – 0+042,283. Üst yüzey km 0+030 – 0+036 ile 0+042,283 – 0+050 arasında 13,9 m² kendi üstüne biniyor; aradaki kot 0,00–1,45 m, yani katlanma. |
| Dar kurp, 3 m şerit | Pay −0,125 (−4,5 m), örtüşme 0,2 m². |

## Açık

- **Malzeme şekillerinin kapanması** (ROAD-COR-13'ün kalanı): Üstyapı tabakaları koridorda değil kesit çiziminde
  kuruluyor (ADR 0930); tabaka tabanlarının kesişip kesişmediği orada denetlenmeli.
- **Kırpma:** Civil 3D'nin "Clear bowtie"ı gibi katlanan ve üst üste binen parçayı kırpıp yüzeyi düzeltmek, öneri
  olarak ayrı bir komut olmalı. Ön denetim yalnız söyler, değiştirmez.
- **Yol üstünden geçen yol:** Gerçek bir üst geçitte iki parça gerçekten üst üste olur; ayrımı şimdilik kot farkıyla
  kullanıcı yapar.
