# ADR 8154: Kurallarda binanın kendi ±0,00'ı ve bina bina kat adedi

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8153](8153-plinth-level-4-1-yyy.md) §6 ("Öbür kurallar katların kotunu bugün projenin ±0,00'ından okur").
- **Bağlam:** ADR 8072 (bina programının binası, kendi ±0,00'ı `zero`), 8153 (olguların katlarında `zero`), 8098
  (bina yüksekliği ve kat adedi), 8085 ve 8083 (kat yükseklikleri), 8078 ve 8092 (asansör), 8087 (kuyular).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-n (bina yüksekliği binanın kot aldığı noktadan), 4/1-o (bodrum),
  9/1-c (kat adedi binanın kot aldığı noktaya göre), 5/11.
- **Sözleşme:** değişmedi (olgular 8153'te `zero`'yu taşır). `tr.kentos.mimari-mevzuat` 0.42.0. **`.kcad`:**
  değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Kurallar katın kotunu projenin ±0,00'ıyla karşılaştırıyordu: bir bloğu öbüründen yüksek girilen (kendi ±0,00'ı +2,00)
binada bodrum katı zemin kat, zemin katı birinci kat sayılabiliyor; bina yüksekliği projenin sıfırından ölçülüp fazla
çıkıyordu. Planın kat adedi de bütün binaların katlarını birlikte sayıyordu: üçer katlı iki blok "6 kat > 3" diye
kalıyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-n, 9/1-c | Bina yüksekliği ve kat adedi binanın kot aldığı noktadan | Katın kotu binasının ±0,00'ından (K397) |
| PAİY 5/11 | Planın kat adedi ve bina yüksekliği aşılamaz | Bina bina: en çok katlı ve en yüksek bina okunur (K398) |
| ADR 8072 | Bir blok öbüründen yüksek ya da alçak girilir: binanın kendi ±0,00'ı | `zero` katın binasınınki (8153) |

## §3. Karar

- **K397 — Katın kotu binasının sıfırından:** `katman.kot(k)` = katın kotu − binasının `zero`'su (yoksa 0). ±0,00'la
  yapılan karşılaştırmalar onunla: ±0,00 üstündeki katlar (`kat_yuksekligi.ustteki`; 28/1, 22/1-m, 28/1-d ve bina
  yüksekliği bunu okur), asansörün üst katları ve bodrumları (34/1, 34/8), kaçış kapılarının bodrumu (47/3), yangın
  dayanımının, kuyuların, emsalin normal kat dışı katlarının ve kullanımın zemin katı (±0,00'a en yakın kat). Katlar
  arası karşılaştırmalar (bir katın zemin kattan alçak olması) aynı binada sıfırdan bağımsızdır, değişmedi.
- **K398 — Bina bina kat adedi ve yükseklik:** planın kat adedi en çok katlı binanın katlarıyla (binaların katları
  toplanmaz); bina yüksekliği kendi sıfırından en üst katın kotu artı yüksekliği, en yükseği.

## §4. Uygulama

- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/katman.py` (`kot`), `kat_yuksekligi.py` (`ustteki`),
  `bina_yuksekligi.py` (`katlar`, yükseklik, kat adedi), `asansor.py`, `kapilar.py`, `dayanim.py`, `kuyular.py`,
  `emsal.py`, `kullanim.py`.

## §5. Sınamalar

- **Kural:** bina yüksekliği 11 durum (3'ü yeni: üçer katlı iki bina, planın 3 katı → geçer; kendi sıfırı +2,00 olan
  binanın +8,00'deki üst katı, kendi sıfırından 9,00 m ≤ 9,50 → geçer; dört katlısı 12,00 m → kalır); kat yüksekliği 48
  durum (1'i yeni: kendi sıfırının altındaki 4,60 m'lik kat bodrumdur, zemin ve birinci kat 3,80 m → 28/1-c geçer).
  Bütün `mimari-mevzuat` sınamaları (eski durumlar sıfırsız: değişmedi) yeşil.
- **Masaüstü:** bina programı sahneleri (kendi sıfırlı iki blok), kural seti, kullanım, asansör, taş duvar, kesme taş,
  emsal, çıkma ve ortak süzgeçlerle sınamalar yeşil; iş çalıştırıcının ulusal kural sınamaları yeşil.

## §6. Kalanlar

- Tahvil (9/1-c): planın yalnız birini verdiği durumda bina yüksekliği ile kat adedinin birbirine çevrilmesi — yapıldı:
  ADR [8157](8157-height-storeys-conversion-9-1-c.md).
- Plan notunun binası: bir plan notu bütün projenin; blok blok farklı plan notu (kademe hattı) okunmaz — yapıldı: ADR
  [8163](8163-plan-notes-per-building.md).
