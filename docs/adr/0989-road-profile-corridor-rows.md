# 0989 — Boykesitin üstünde şev ve platform satırları

Tarih: 2026-11-06. Durum: kabul (Y83). Kapsam ROAD-SEC-11 (plan–profil paftası) ve ADR 0980 (KGM bantları).

## Sorun

KGM'nin plan–profil paftasında plan haritası ile boykesit arasında koridorun şevlerini, platform genişliğini ve üst
yapı kalınlığını km boyunca söyleyen altı satır vardır. KentOS bu satırları çizmiyordu: paftayı okuyan, bir km'de
yolun yarmada mı dolguda mı olduğunu ve hangi şevle açıldığını ancak enkesit paftalarından bulabiliyordu.

## Araştırma (6 Kasım 2026)

- **KGM Haziran 2024 örnek paftaları** (ADR 0984'te kaynak gösterilen PDF), plan–profil sayfaları:
  - Satırlar yukarıdan aşağı: SOL DOL.ŞEVİ, SOL YAR.ŞEVİ, PLATFORM GEN., ÜST YAPI KAL., SAĞ DOL.ŞEVİ, SAĞ YAR.ŞEVİ.
  - Şev yatay/düşey küçük tam sayılarla yazılır: "3/2", "2/1", "1/1". Tarafın o km'de dolgusu yoksa dolgu satırında
    "-", yarması yoksa yarma satırında "-".
  - Platform "12.00 m.", üst yapı "0.45 m." (iki ondalık ve "m.").
  - Bir değer değişene kadar tek hücredir; hücreler dik çizgiyle ayrılır.
- **KGM 2019 teknik şartnamesi** boykesit bantlarını sayar (ADR 0980); bu altı satırı yazmaz. 2025 ve 2026 birim
  fiyatları hâlâ 2019 şartnamesine bağlı (ADR 0984). Satırlar 2024 örneğinin uygulamasıdır.
- **Civil 3D / OpenRoads:** profil görünüşü bantları kesitten veri okuyabilir (Civil 3D "Section Data" bandı), bu
  araştırmada hazır bir "şev satırı" bulunamadı.
- **Matematik:** hesap yok, okuma var. Değerler koridorun kesitlerinden (ADR 0905) okunur: tarafın türü (yarma/dolgu),
  o km'deki tip kesitin (geçişi sayılarak) şevi ve şerit + banket genişlikleri, bölgenin tip kesitinin üst yapı
  katmanlarının toplamı.

## Karar

- **`road.profiles.draw` `corridor`** (kimlik ya da ad) ve **`road.sheets.plan` `corridor`**: yalnız `bands: kgm` ile.
  - KentOS bantlarıyla istenirse `invalid_label` ("yalnız KGM bantlarıyla").
  - Koridor boykesitin aksında değilse `invalid_label`; yoksa `unknown_corridor`.
- **Değerler** (`road_corridor_bands::rows`):
  - Dolgu satırı: taraf dolgudaysa dolgu şevi, değilse "-". Yarma satırı aynı.
  - Platform: iki tarafın bileşen genişliklerinin toplamı. Üst yapı: katman kalınlıkları toplamı; üst yapısız tip
    kesitte "-".
  - Şev yazımı: paydası 10'a kadar tam sayıya düşen oran "3/2"; düşmeyen "1.23/1".
- **Sınırlar:** iki komşu kesit farklı değer veriyorsa hücre sınırı ikisinin ortasıdır. Koridorun ilk kesitinden
  önce ve son kesitinden sonra hücre boştur.
- **Çizim:** satırlar boykesit çerçevesinin üstündedir, her biri yazı yüksekliğinin 2,4 katı (bantlarla aynı). Ad
  sütunu bantlarınkiyle aynı genişliktedir. Nesneler: ad sütununun iki dik kenarı; satır başına üst çizgi ve ad; dolu
  hücre başına yazı; iki hücre arasına dik çizgi.
- **Pafta:** `road.sheets.plan` satırların yüksekliğini görünümün üstüne ekler; boykesit haritasına sığmazsa
  `road_sheet_outside` uyarısı eskisi gibi.
- **Pencere:** Yol paftaları penceresinde "Şev ve platform" seçimi (varsayılan "Satır yok").

## Ölçüm

- **Bağımsız başvuru** `scripts/fixtures/road_corridor_band_cases.py` (KentOS kodu yok): koridor ve kesitleri koridor
  başvurusundan, çizim boykesit başvurusundan. `fixtures/road/v1/corridor-bands.json` (2 çizim) ve
  `road.profiles.draw.json` (48 durum, 5'i yeni: Tip 1 ve Tip 2 yarma/dolgu, 0.45 m. üst yapı, üç ret).
- **Satırlar** (başvuru ile KentOS, uçlar 1 µm içinde): SOL DOL "- / 3/2 / - / 2/1 / -", PLATFORM "9.00 m." 0+000–0+300,
  "12.50 m." 0+300–0+600, 0+600–0+669 boş (koridorun son kesitinden sonra); üstyapılı durumda ÜST YAPI "0.45 m."
  0+000–0+300.
- **Resim** `.run/gui-evidence/yol-y83/`: KGM A1 paftası 250 dpi (`y83-satirlar.png`); altı satır okunuyor, en dar
  hücre ("3/2", 0+162.50–0+184.86, ≈ 22 m) yazıyı taşıyor, çakışma yok.

## Açık

- **Hücre sınırı yaklaşımdır:** gerçek yarma/dolgu geçişi iki kesit arasında arazi ile platformun kesiştiği yerdir;
  sık kesit (ADR 0939) sınırı daraltır.
- **Genişletme** (ADR 0986) platform genişliğine eklenmez; tip kesitin genişliği yazılır.
- **Boykesit penceresinde koridor seçimi yok;** komut ve pafta penceresinde var.
