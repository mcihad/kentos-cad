# 0999 — Boykesitte proje uçlarının bayrağı

Tarih: 2026-11-07. Durum: kabul (Y93). Kapsam ROAD-SEC-11 ve ADR 0987 §Açık ("bayrak biçimi", "başlangıç noktasının
yanındaki ikinci KM = / KOT = yazısı").

## Sorun

Y81 proje uçlarını ince kırmızı bir bağ çizgisi ve alt alta üç yazıyla (PROJE BAŞI, KM, KOT) işaretliyordu. KGM'nin
2024 paftalarındaki bayrak ve noktanın yanındaki km–kot yazısı yoktu.

## Araştırma (7 Kasım 2026)

- **KGM Haziran 2024 örnek paftaları:**
  - s. 8: başlangıç noktasından eğik bir kırmızı bant (bu paftada yaklaşık 64 mm aşağı, 32 mm içeri; 1:2) yatay bir
    kırmızı şeride bağlanır. Şeridin ucu eğik kesiktir.
  - Şeridin üstünde "PROJE BAŞI", altında "KM : 0+000.00".
  - Noktanın hemen yanında, bandın iç tarafında "KM = 0+000.00 / KOT = 964.670".
  - s. 12 sonda aynısı: "PROJE SONU", "KM : 19+541.05", "KM = … / KOT = 886.461".
- **KGM 2019 şartnamesi** s. 5: "Proje başı ve sonunda kot ve koordinat değerleri yazılacaktır."
- **Matematik:** yok.

## Karar

- **Bağ çizgisi:** 10 mm içeri, 20 mm çerçevenin yakın kenarından uzağa. 2024'ün 1:2 eğimi korunur; 64 mm
  çoğu boykesit çerçevesine sığmaz.
- **Bayrak:** bağ çizgisinin ucundan içeri doğru 9 yazı yüksekliği uzunluğunda, yarım yazı yüksekliği kalınlığında,
  uzak ucu eğik kırmızı **dolu tarama**.
- **Yazılar:** bayrağın üstünde "PROJE BAŞI"/"PROJE SONU", altında "KM : 1+000.00". Noktanın yanında, bağ çizgisinin
  iç tarafında "KM = 1+000.00" ve "KOT = 102.000" (km iki, kot üç ondalık; 2024 gibi). Hepsi maskeli. Başlık artık
  siyah (2024'te siyah); kırmızı olan çizgi ve bayrak.
- **Nesne sayısı:** her görünen uç 6 nesnedir: çizgi, bayrak, dört yazı. Önceden 4'tü. Bağımsız başvuru aynı sayıyı
  ekler; boykesit, pafta, proje ve bant durumları yeniden üretildi.

## Ölçüm

- **Ortak durumlar:** bütün `road.` durumları geçti (`road.profiles.draw` 64, `road.sheets.plan` 39, `road.project.*`).
- **Testler:** uygulama 143 + 521, masaüstü 1858, plot 86.
- **Resim** `.run/gui-evidence/yol-y93/y93-uclar.png`: KGM A1, iki uç.
  - İlk resimde 6 × 12 mm'lik bağ çizgisiyle bayrağın "KM :" yazısı noktanın "KM = / KOT =" yazısına yapışıyordu.
  - 10 × 20 mm'de ikisi ayrı okunuyor.

## Açık

- **Şeridin rengi:** 2024'te şerit koyu kırmızıdan açığa geçişli; burada düz kırmızı tarama.
