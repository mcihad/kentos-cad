# 0923 — KentOS Yol: platformda su

Tarih: 2026-11-04. plan/road.md Y22 (ROAD-DEVER-09). Durum: kabul, açıklar aşağıda.

## Dayanak

- Dever geçişinde enine eğim sıfırdan geçer. Boy eğimi de azsa platformun bir yeri yatık kalır ve su orada durur.
  Tasarım kılavuzları bu yatık yeri, en küçük bileşke eğimi ve ızgaranın yerini denetletir (plan/road.md §11
  ROAD-DEVER-09; TxDOT ve AASHTO dever geçişi bölümleri, sayıları kuruma göre değişir).
- **Civil 3D:** koridor yüzeyinde eğim analizi ve su damlası (Water Drop) ile suyun yolu okunur (Y16'nın dayanağı,
  ADR 0914). Yüzey analizinin eğim aralıkları koridor yüzeyine de uygulanır.
- **OpenRoads Designer:** Analyze Trace Slope ve terrain analizi koridorun tasarım yüzeyinde çalışır (ADR 0914).
- **Netcad:** NETPRO için çevrimiçi resmî belge bulunamadı.
- En küçük eğim tasarım standardınındır (ROAD-DEC-01); KentOS sayı vermez.

## Karar

### §1 Hesap

`kentos_road::platform_water`. Koridorun üst yüzeyinin şerit ve banket üçgenleri okunur; şev ve hendek okunmaz. Bir
üçgenin eğimi düzleminin en dik eğimidir: kenarların vektörel çarpımının yatay boyu bölü düşey bileşeni. Eğimi
`least`'ten küçük üçgen yatıktır. Yatık üçgen içeren şeritler (ardışık iki kesit arası) birleştirilir: aynı bölgede
uç uca gelenler tek aralıktır. Her aralığın plan alanı ve en küçük eğimi verilir. Enine eğim, her yanın ilk şeridinde
eksen noktasından `c1` noktasına kot farkının plan uzaklığına oranıdır (dışa yükselen artı). Km'ler arasında doğrusal
alınır; işaret değiştirdiği yerde sıfırdır. Alçak nokta, platform kenarında (eksen ve bileşenlerin kenar çizgileri)
iki komşusundan alçak noktadır.

### §2 Komut

`road.corridors.water` v1 yalnız okur. Girdi: koridor ve `least` (oran). Cevap: yatık üçgen sayısı ve alanı, yatık
aralıklar, sıfır enine eğim km'leri, alçak noktalar. Ret: `not_a_road_project`, `unknown_corridor`,
`invalid_platform_water` (`least` 0'dan büyük değil); koridor kurulamazsa onun retleri. Uyarılar: `corridor_flat` ve
koridorun kendi uyarıları (eskimiş, çözülmeyen, katlanan).

### §3 Masaüstü

Koridor panelinde Platform suyu penceresi. Planda platform gri, yatık üçgenler kırmızı, şevler soluk gösterilir;
alçak noktalar halkayla işaretlenir. Yanda aralıklar, sıfır enine eğim km'leri ve alçak noktalar listelenir. Eğim
pencerede yüzde yazılır.

## Doğrulama

21 ortak durum (`scripts/fixtures/road_platform_water_cases.py`). Koridor, üçgenleri ve rolleri
road_corridor_cases.py'nin bağımsız hesabından alınır; eğim, km aralıkları, enine eğim ve alçak noktalar KentOS kodu
olmadan hesaplanır. Durumlar: deverli (geçişli) ve deversiz koridor, on beş eğim sınırı, ad ile, retler. Python
`kentos.road`, MCP, masaüstü penceresinin testi ve resimleri (`.run/gui-evidence/yol-y22-su-*`).

## Açıklar

- Suyun platform üstündeki yolu (koridor yüzeyinde su damlası) ve ızgara aralığı (yayılma genişliği, Rasyonel debi;
  HEC-22 yöntemi).
- Ters enine eğim (orta refüje akan şerit) ve bölünmüş yolda iç kenar.
- Yatık yerin planda çizime yazılması.
