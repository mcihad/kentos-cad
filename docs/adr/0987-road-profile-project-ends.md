# 0987 — Boykesitte proje başı ve sonu: km ve kot

Tarih: 2026-11-06. Durum: kabul (Y81). Kapsam ROAD-SEC-11 (boykesit görünüşü) ve ADR 0984 §Açık ("proje uçlarının
kotu").

## Sorun

KGM proje başı ve sonunda kot ister. Y78 plan haritasına proje uçlarının km'sini ve koordinatlarını yazıyor, kotu
yazmıyordu. Boykesit çiziminde ise uçlar hiç işaretlenmiyordu.

## Araştırma (6 Kasım 2026)

- **KGM 2019 şartnamesi** (2025 ve 2026 birim fiyatları hâlâ ona bağlı, ADR 0984), s. 5: "Proje başı ve sonunda kot ve
  koordinat değerleri yazılacaktır."
- **KGM 2019 EK-10:** "PROJE BAŞI, KM : 0+000.000", `Y = 461 740.648`, `X = 4 437 606.648`, `KOT=1202.685` (taramadan;
  son haneler okunamayabilir).
- **KGM Haziran 2024 örnek paftaları**, sayfa 8:
  - Planda "PROJE BAŞI / KM : 0+000.00 / X=… / Y=…"; kot yok.
  - Boykesitte kırmızı kotun başından eğik kırmızı bir bağ çizgisi, kırmızı bir bayrakta "PROJE BAŞI", altında
    "KM : 0+000.00" ve "KOT = 964.670"; başlangıç noktasının yanında da "KM = 0+000.00 / KOT = 964.670".
  - **Kot, boykesitin işidir;** plan koordinatı, boykesit kotu yazar.
- **Matematik:** kot, düşey güzergâhın uçtaki kotudur, yani ilk ya da son PVI'nın kotu. Hesap gerekmez.

## Karar

- **`road.profiles.draw`:** görünümün içine düşen her uca kırmızı, 12 mm'lik, 6 mm içeri eğik bir bağ çizgisi çizilir.
  Ucunda "PROJE BAŞI" ya da "PROJE SONU" (kırmızı), "KM : 1+000.00" (iki ondalık, 2024 gibi) ve "KOT = 102.000" (üç
  ondalık, 2024 ve 2019 gibi) yazılır.
  - Nokta çerçevenin üst yarısındaysa çizgi aşağı, alt yarısındaysa yukarı gider.
  - Yazılar yukarıdan aşağı okunur; başta sağa, sonda sola yaslıdır.
  - Yazılar maskelidir: ızgara ve arazi çizgisi üstlerinden geçiyordu.
- **Plandaki yazı kotsuz kalır** (2024 örneği gibi). Kot boykesittedir; bir aksın birden çok boykesiti olabilir.
- **Nesne sayısı:** her görünen uç 4 nesnedir. Bağımsız başvuru (`road_profile_command_cases.py`) aynı sayıyı ekler.
  Bundan türeyen proje (yeniden üretim, sorgu, düzenleme) ve pafta durumları yeniden üretildi.

## Ölçüm

- **Ortak durumlar:** `road.profiles.draw` 43, `road.project.*`, `road.sheets.plan` 35, `road.sheets.sections` 27.
  Bütün `road.` durumları geçti.
- **Testler:** uygulama 520 + 137, plot 86, masaüstü 1853 (`RUST_MIN_STACK=67108864`).
- **Resimler** `.run/gui-evidence/yol-y81/`: KGM A1 paftası, PDF'ten boykesitin iki ucu (`y81-uclar.png`):
  - başta "PROJE BAŞI / KM : 1+000.00 / KOT = 102.000", yukarı;
  - sonda "PROJE SONU / KM : 1+669.00 / KOT = 113.000", aşağı.
  - İlk resimde bağ çizgisi çerçevenin kenarına biniyordu, yazıları ızgara ve arazi çizgisi kesiyordu. Çizgi içeri
    eğildi, yazılar maskelendi.

## Açık

- **Bayrak biçimi:** 2024 örneğindeki kırmızı dolgulu ok bayrağı çizilmiyor; yalnız çizgi ve yazı var.
- **Başlangıç noktasının yanındaki ikinci "KM = / KOT =" yazısı** yazılmıyor.
- **Plandaki proje ucunda kot** (2019 EK-10'daki gibi).
