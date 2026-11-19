# ADR 8157: Bina yüksekliği ile kat adedinin tahvili — PAİY 9/1-c

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8098](8098-plan-building-height-and-storeys.md) §6, [8153](8153-plinth-level-4-1-yyy.md) §6,
  [8154](8154-rules-by-building-zero.md) §6 ("Tahvil (9/1-c): planın yalnız birini verdiği durumda bina yüksekliği ile
  kat adedinin birbirine çevrilmesi").
- **Bağlam:** ADR 8098 (planın bina yüksekliği ve kat adedi notları, 5/11), 8083–8085 ve 8094 (28/1'in bentleri, bölge,
  geleneksel kesme taş yığma bina), 8084 (plan notu katmanı, 69/3), 8153 (subasman), 8154 (binanın kendi ±0,00'ı, bina
  bina kat adedi).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 9/1-a, 9/1-c, 28/1, 28/2 (RG 3.7.2017/30113; 28/1-c ek ibare RG
  12.8.2023/32277; 28/1-d RG 9.10.2021/31623); Sivas Belediyesi İmar Yönetmeliği 9/1-c, 28/1 (RG 10.11.2022/32009).
- **Sözleşme:** yeni komut yok; olgular değişmedi. `tr.kentos.mimari-mevzuat` 0.44.0 (`tahvil`). **`.kcad`:**
  değişmedi. **Web:** dokunulmadı.

## §1. Sorun

Planın yalnız bina yüksekliğini verdiği parselde kat adedi, yalnız kat adedini verdiği parselde bina yüksekliği
okunmuyordu: 9,50 m'lik plana üç kat 3,00 m'lik bina 5/11'den geçiyor, 9/1-c'nin tahvilinin bu yükseklikte kaç kata
karşılık geldiği söylenmiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 9/1-c (1.7.2026'ya kadar işlenmiş birleştirilmiş metin; değişiklik işareti yok) | Kat adetleri binanın kot aldığı noktaya göre; planlardaki bina yüksekliklerinin ya da kat adetlerinin birbirlerine tahvilinde ya da neye karşılık geldiklerinin tespitinde "bu esaslar ile binanın kot aldığı noktaya en fazla 1.2 metre eklenmek suretiyle belirlenen subasman kotu dikkate alınır" | Tahvil 1,20 m subasmanla (K404) |
| PAİY 9/1-a (RG 25.7.2019 değişik) | Bina yükseklikleri, 28 inci maddede katın konumuna göre belirtilen en çok kat yüksekliklerinin kat adedi ile çarpımıyla bulunan toplamı aşmaz | "Bu esaslar": zemin ve normal katın 28/1'deki en çok yüksekliği (K404) |
| PAİY 28/1 | Bölgeye göre en çok kat yükseklikleri: ticaret zemin 4,50 (asma katlı 5,50), öbür 4,00; karma zemin 4,50 (5,50), konut katı 3,60, konut dışı 4,00; konut 3,60, 12.8.2023'ten bodrum hariç 3 katı geçmeyen binada 4,00; zemini ticaretli konut zemin 4,50, öbür 3,60; geleneksel kesme taş yığma zemin dahil 3 kata kadar 4,50 | Bölgenin bendi başvuru gününün hâliyle, n katlı binanın bendiyle (K404) |
| PAİY 28/2 | 28 inci maddenin kat yükseklikleri dikkate alınmadan bina yüksekliği verilen planlarda, plan yeniden değerlendirilinceye kadar 28/1'in kat yükseklikleri ile planın (yoksa yönetmeliğin) kat adedinin çarpımı | Okunuşu idarenin: aşan uzman incelemesi (K405) |
| Sivas İmar Yönetmeliği 9/1-c (Lexpera'nın metni) | Aynı tahvil, "en fazla 1.20 metre" | Sivas seçiliyse Sivas 28/1'iyle de, sıkı olanı (K404) |
| ADR 8084 (69/3) | Planın kat yüksekliği notları yönetmeliklerin önüne geçer | Tahvilde de önce (K404) |
| İkincil kaynaklar: imar hesabı paketi tr-imar'ın belgesi (PyPI, 0.1.2); kot ve subasman üzerine hukuk notu (hukukcularevi.com) | Hmax = kat yüksekliği × kat adedi (28/2), 9/1-c'nin 1,20 m'si ayrıca eklenebilir: "İdareler bu iki yaklaşımı farklı uygulayabilir"; subasman ±0,00 ile +1,20 arasında, kat sayımında kot alınan nokta | Tahvili aşmak kaldı değil uzman incelemesi (K405) |

## §3. Karar

- **K404 — Tahvil:** n katın yüksekliği H(n) = 1,20 m + zemin katın en çok yüksekliği + (n − 1) × normal katın en çok
  yüksekliği. Yükseklikler parselin (binanın kendi bölgesi varsa onun, ADR 8144) 28/1 bendinden, başvuru gününün
  hâliyle ve n katlı binanın bendiyle: 28/1-c'de n ≤ 3 ise 4,00 m (12.8.2023'ten); konut bölgesinde zemin dahil 3 katı
  geçmeyen geleneksel kesme taş yığma binada 28/1-d. Zemin katın asma katı varsa asma katlı zemin katın değeri; karma
  alanda konut projesinin katları konut katı. Planın kat yüksekliği notları önce (69/3). Bölgesi yazılmamış konut
  projesi konut alanıdır (ADR 8083). Planın bina yüksekliği H'nin kat adedi H(n) ≤ H olan en büyük n. Sivas seçiliyse
  Sivas 28/1'iyle de hesaplanır, sıkı olanı bağlar, öbürü iletide yazılır.
- **K405 — Kural (`tahvil`):** plan notlarında yalnız biri varsa: her binanın kat adedi (binasının ±0,00'ından, ADR
  8154) planın yüksekliğinin tahvilini, ya da yüksekliği planın kat adedinin tahvilini aşmıyorsa geçti (satırın notu
  tahvili söyler: "planın 9.50 m'si 2 kata karşılık gelir"); aşıyorsa uzman incelemesi (`paiy_height_storeys_conversion`,
  iletide subasman, kaynağı ve tahvilin iki komşu değeri). İkisi de yazılmışsa kapsam dışı (tahvil gerekmez); hiçbiri
  yoksa satır yok. Bölgesi yazılmamış konut dışı bina ve en üst katının yüksekliği yazılmamış bina satırda eksik veri,
  bulgusuz (nedenin bulgusu kat yüksekliği kuralınındır); 28/1'in saymadığı kullanım kapsam dışı. Satır 1.10.2017'den
  önceki başvuruda uzman incelemesi. `bina-yuksekligi` katkısı tahvilin bulgularını da verir.

## §4. Uygulama

- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/tahvil.py` (yeni: `yukseklik`, `kat_adedi`, `kural`), `mimari.py`,
  `denetim.py`, `manifest.json`, `data/surumler.json` (9/1-c'nin 1.10.2017 hâli, `subasman` 1,2).

## §5. Sınamalar

- **Kural** (`test_tahvil.py`; 9/1-c, 9/1-a, 28/1 ve Sivas 28/1-c'nin metninden elle): 20 bağımsız durum: konut alanında
  2.7.2026'da 9,50 m (1 kat 5,20, 2 kat 9,20, 3 kat 13,20) iki katlı bina geçer, üç katlı uzman incelemesi; 12,50 m'de
  üç kat 1.6.2023'te (3,60 m: 12,00) geçer, 2.7.2026'da (4,00 m: 13,20) uzman incelemesi, tarihsiz eksik veri; kat
  adedi 4 (15,60 m) 12,45 m'lik binayı geçirir, kat adedi 2 (9,20 m) 10,00 m'lik binada uzman incelemesi; en üst kat
  yüksekliksiz eksik veri; iki not birlikte kapsam dışı; notsuz satır yok; ticaret (5,70; 9,70), asma katlı zemin
  (6,70; 10,70), planın kat yüksekliği notu (7,20; 10,20), bölgesiz büro eksik veri, öbür kullanım kapsam dışı, iki
  binadan üç katlısı, kendi ±0,00'ı +2,00 olan binanın bodrumu sayılmaz, 2016 başvurusu, Sivas (PAİY 2 kat, Sivas 3
  kat) ve geleneksel kesme taş (5,70; 10,20). `Hesap`: H(1…4) = 5,20; 9,20; 13,20; 15,60 ve kat adetleri.
- **Paket:** bütün `mimari-mevzuat` sınamaları yeşil (asansör sınaması tahvilin eksik yükseklik bulgusunu yakaladı:
  kat yüksekliği kuralının bulgusuyla aynı neden; tahvil satırda söyler).
- **Masaüstü** (`plan_limits_tests`, 8098'in iki katlı evi, 3,80 m katlar, 7,60 m): yalnız bina yüksekliği 8 m:
  tahvil 1 kat (5,20; 9,20), evde 2 kat: uzman incelemesi, 5/11 geçti; 10 m: geçti; yalnız kat adedi 2 (9,20 m): geçti.
- **Resimler:** `.run/gui-evidence/tahvil-8157/`: Denetim'de tahvilin uzman incelemesi, açık ve koyu.

## §6. Kalanlar

- 28/2: planın 28 inci maddenin kat yükseklikleri dikkate alınmadan bina yüksekliği verip vermediği planda yazılı
  değildir; idarenin okunuşu uzman incelemesiyle.
- 9/1-a'nın yol genişliğine göre kat adedi (planda kat adedi ya da bina yüksekliği belirtilmemiş parsel): yolun genişliği
  çizimde yok.
- Kot alınan noktanın tespiti (10–14: yoldan, tabii zeminden kotlandırma, kademelendirme): binanın ±0,00'ı projenin
  yazdığıdır.
