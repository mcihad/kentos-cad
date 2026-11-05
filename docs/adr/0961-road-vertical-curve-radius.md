# 0961 — Düşey kurbun K'si ve yarıçapı H: hangi birim

Tarih: 2026-11-05. Durum: kabul (Y55). Kapsam ROAD-PROFILE-04.

## Sorun

Boykesit raporu her düşey kurp için K = L/|A| veriyordu. A yüzdeydi, K metre/yüzde. Ama:

- **Denetim:** İletisi "tepe kurbunun K'si 10.000, en az 15." diyordu. Birim yazmıyordu; K'nin yüzde mi oran mı
  üstünden kurulduğu (10 mu 1000 mi) okunmuyordu.
- **Standartlar:** Hepsi K yazmaz. Alman RAA (FGSV 202) düşey kurbu "Kuppen-/Wannenhalbmesser" H ile, metre olarak
  verir. Böyle bir standardı uygulayan kişi her kuralı elle 100'e bölmek zorundaydı. Bu bölme yanlış yapılırsa sınır
  100 kat kayardı.

ROAD-PROFILE-04: "K=L/|g2−g1| raporunda grade yüzde mi oran mı açık olsun; standarttaki K birimiyle doğru çevir."

## Araştırma

| Kaynak | Ne diyor |
|---|---|
| AASHTO Green Book, TxDOT Roadway Design Manual 9.2.11 | K, eğimin %1 değiştiği yatay boydur: L = K·A, A yüzde. Metrik birimde m/%, ABD biriminde ft/%. |
| BC Forest Road Engineering Guidebook, düşey kurp ekleri | 100·K'ye kurbun "eşdeğer yarıçapı" denir. Bu eğriliği karşılaştırmak için bir uzlaşımdır, kurbun her yerindeki gerçek yarıçap değildir. |
| RAA 2008 (FGSV 202) | Düşey kurp (Ausrundung) parabolün parametresi H ile, metre olarak yazılır: z = x²/(2H). |
| Civil 3D | Düşey kurbun K değerini ve tasarım ölçütü dosyasında en küçük K'yi tutar. |

## Karar

### Matematik

Parabol z = z₀ + g₁·t + (g₂ − g₁)·t²/(2L):

- **İkinci türev sabittir:** z'' = Δg/L.
- **Yarıçap:** R(t) = (1 + z'²)^{3/2}/|z''|. En küçük değeri eğimin sıfır olduğu yerde alır: L/|Δg|.
- **K ile bağı:** Δg oran olduğundan H = L/|Δg| = 100·L/|A| = **100·K**. Standartların H'si budur.

**Asimetrik kurp** (kollar l₁ ve l₂, L = l₁ + l₂):

- İki parabol PVI'de, eğimi gm = (g₁l₁ + g₂l₂)/L olan ortak bir teğetle buluşur.
- Kolların ikinci türevleri z₁'' = Δg·l₂/(l₁L) ve z₂'' = Δg·l₁/(l₂L)'dir.
- Keskin kolun yarıçapı **H = L/|Δg|·min(l₁/l₂, l₂/l₁)** olur. Bu, 100·K'den küçüktür. Asimetrik kurbu yalnız K ile
  denetlemek keskin kolu gözden kaçırır.

**Daire kurp:** H kendi yarıçapıdır. K yine L/|A|'dır ve L yatay boy olduğundan 100·K'den biraz küçüktür.

### Rapor

- Boykesit raporunun her kurbuna `curveRadius` (m) eklendi: en küçük H.
- Çekirdek bunu kurbun parçalarından okur: parabol için 1/|r|, daire için yarıçap.
- Eğimi değişmeyen kurpta (kırık PVI) yoktur.
- Boykesit penceresinin özeti "en küçük K 10,0 m/% · en küçük yarıçap 1000 m" der.

### Denetim

`road.profiles.check` iki kural daha alır:

- `leastCrestRadius` ve `leastSagRadius` (m), bulgu türleri `crestRadius` ve `sagRadius`;
- K kurallarıyla birlikte de verilebilir, ikisi ayrı denetlenir.

İletiler birimleriyle:

- "v1 tepe kurbunun K'si 10.000 m/%, en az 15 m/%."
- "v1 tepe kurbunun yarıçapı 1000.000 m, en az 1500 m."

Tasarım denetimi penceresinde alanlar "Tepe K en az (m/%)", "Tepe yarıçapı en az (m)" diye adlandı. Altında K ile
H'nin bağını söyleyen bir yardım satırı var.

KentOS öntanımlı bir sınır vermez. ABD birimi (ft/%) ayak birimli çizimin işidir; burada her şey metredir.

## Ölçüm

### Bağımsız referanslar

`road_profile_command_cases.py`, H'yi KentOS kodu kullanmadan yukarıdaki kapalı formüllerden kurar. Rust çekirdeği
parçaların ikinci türevinden okur. Boykesit sorgusu, düzenlemesi ve çizimi bu alanla yeniden üretildi; hepsi Rust'ta
ve Python'da geçer.

`road_design_check_cases.py`'ye 6 yeni durum eklendi:

- tepe ve çukur yarıçapı;
- K ile birlikte;
- asimetrik kurbun keskin kolu;
- daire kurbun yarıçapı;
- tutan yarıçap;
- sıfır yarıçap (ret).

### Ölçülen değerler

| Kurp | K (m/%) | 100·K (m) | H (m) |
|---|---|---|---|
| Denetimin b-1 v1 (parabol, L = 80) | 10,000 | 1000 | 1000,0 |
| Denetimin b-1 v2 (parabol, L = 120) | 21,000 | 2100 | 2100,0 |
| Boykesit sorgusunun b-1 v3 (asimetrik, 50/35) | 11,370 | 1137 | 795,9 = 1137 × 35/50 |
| Boykesit sorgusunun b-1 v2 (daire, R = 1200) | 11,986 | 1198,6 | 1200,0 |

- **Asimetrik kurp:** Keskin kolu, simetrik eşdeğerinden %30 dar.
- **Daire kurp:** K'den bulunan yarıçap gerçek yarıçaptan 1,4 m küçük.

## Açık

- **Görüş mesafesi:** H'yi görüş mesafesinden türetmek (RAA'nın tepe kuralı, AASHTO'nun SSD formülleri) ayrı bir iştir.
  Görüş mesafesi ADR 0921'de, görüş ve konfordan kurp boyu ADR 0935'te kendi komutlarıyla var.
- **Çizimdeki etiket:** Boykesit çizimindeki kurp etiketi hâlâ "K=10,0" yazar, birimsiz ve H'siz.
