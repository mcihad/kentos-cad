# 0996 — Hafriyat satırlarında dolguda kullanılamayan yarma (DKY)

Tarih: 2026-11-07. Durum: kabul (Y90). Kapsam ROAD-SEC-11, ROAD-HAUL-04 (uygunsuz malzeme, ADR 0944) ve ADR 0995 §Açık
("SM ve DKY işaretleri").

## Sorun

ADR 0944'ten beri Brükner, kazının dolguya konamayan payını km aralıklarıyla tutuyor. Taşıma planı bu payı yalnız
döküm alanlarına gönderiyor. Paftada ise bu kesimler görünmüyordu.

## Araştırma (7 Kasım 2026)

- **KGM Haziran 2024 örnek paftaları** s. 9: hafriyat satırlarında yeşil dik çizgiler "DKY: 5+940" ve
  "DKY: 6+180". Aralarında yeşil not: "KM : 5+940 - 6+180 ARASINDA : TOPLAM 9805 m3 Dolguda Kullanılamayan Yarma
  KM = 0+300 'de 100 m Solda 1 Nolu Depoya".
- **KGM 2019 teknik şartnamesi:** ariyet ve depo gösteren kesimlerde "depo veya ariyet ocağının numarası ismi veya
  mahalli belirtilecektir."
- **Matematik:**
  - Bir aralığın uygunsuz kazısı = uygunsuz pay × ham kazı.
  - Ham kazı, kübaj satırlarının aralıkla örtüşen kısmıdır (satır içinde uzunlukla paylaşılır; ADR 0995 ile aynı).
  - Ham kazı kullanılır, çünkü not yerindeki kazı hacmini söyler (TOPLAM … m3 Dolguda Kullanılamayan Yarma).

## Karar

- **DKY satırı:** Brüknerde uygunsuz aralık varsa hafriyat satırlarının altına altıncı bir satır eklenir. Aralığı
  olmayan Brüknerin çizimi değişmez.
- **Aralık hücresi:** görünüme düşen her aralık için ortasına yeşil "KM : 1+100.00 – 1+250.00 : 1287 m³". Uçlarında
  yeşil dik çizgi.
- **Aralık kübajın dışına taşarsa** kübajın ucunda kesilir.
- **Pafta** görünüm derinliğine DKY satırını da ekler.
- **Görünümün başındaki DN** artık çerçevenin içinde, çizginin sağındadır. Önceden ad sütununa düşüyordu.

## Ölçüm

- **Bağımsız başvuru** `road_earthwork_band_cases.py`:
  - Ana Brükner'de %40 · 100–250 ve %30 · 450–650.
  - Beklenen 1287.279 m³ ve 4361.646 m³; ikincisi 600'de kesilir.
  - `sections_are_the_references` 1e-3 m³ içinde aynı.
- **`road.profiles.draw`** 64 durum, 2'si yeni (bütün görünüm, km 200–500 parçası).
- **Testler:** uygulama 143 + 521, masaüstü 1857, plot 86.
- **Resimler** `.run/gui-evidence/yol-y90/`: KGM A1, 1:1000, iki DKY hücresi okunuyor. İlk resimde baştaki DN ad
  sütunundaydı; düzeltildi (`y90-dn-bas.png`).

## Açık

- **Varış yeri** ("1 Nolu Depoya", km ve uzaklık) yazılmıyor. Taşıma planı akışları kesim kesim veriyor; aralığın
  hangi döküm alanına gittiği için akışın kaynak–hedef ayrıştırması gerekir.
- **Seçme malzeme (SM)** modeli yok.
