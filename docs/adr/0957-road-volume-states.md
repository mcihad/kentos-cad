# 0957 — Hacim halleri: yerinde, gevşek, sıkışık

Tarih: 2026-11-05. Durum: kabul (Y51). Kapsam ROAD-VOL-10.

## Sorun

Kübajın katsayı tablosu (ADR 0906 §4) kazıyı kabarmayla, dolguyu sıkışmayla çarpıyor ve kütle ordinatını bu
"düzeltilmiş" hacimlerle kuruyordu. İki hacmin hangi halde olduğu hiçbir yerde yazılı değildi:

- **Kazı:** Yerinde mi ölçülüyor, gevşek mi?
- **Dolgu:** Sıkışık hacmin istediği malzeme yerinde mi, gevşek mi?

Örnek projenin "sıkışma 0,9"u dolgunun istediğini, dolgunun kendi hacminden küçük gösteriyordu.

ROAD-VOL-10 şunu istiyor: "Bank/in-situ, loose ve compacted hacimler açık volume-state conversion ile taşınsın;
swell/shrink/compaction faktörleri kaynak ve onay taşısın."

## Araştırma

- **Civil 3D:**
  - Kazı katsayısı genellikle 1'den büyüktür; kabarmadır. 1,2, kazılan her 1 m³ için taşımada 1,2 m³ demektir.
  - Dolgu katsayısı da genellikle 1'den büyüktür; dolgunun her m³'ü için taşınması gerekeni söyler. %93'e sıkışan
    malzeme için 1/0,93 = 1,075 girilir.
  - Kullanıcılar aynı zemin için 1,30 mu yoksa 1/(1 − 0,30) ≈ 1,42 mi gireceklerini forumda soruyor. Uzlaşım belirsiz.
  - Kaynaklar: Autodesk Civil 3D kütle taşıma yardımı, Civil 3D API `FillFactor`, Autodesk forumu "dirt work
    quantities".
- **Brezilya:** "Fator de homogeneização" kabarmayı ve sıkışmayı tek katsayıda birleştirir.
- **Fizik:** Hiçbiri halleri yoğunlukla bağlamıyor. Oysa kütle korunur ve haller yoğunluklarla birbirine dönüşür.

## Karar

### Kütle korunumu

ρ_y yerinde, ρ_g gevşek, ρ_s sıkışık yoğunluktur (t/m³).

- **Yerinde V m³ kazı:**
  - kütlesi ρ_y·V t;
  - gevşek hacmi V·ρ_y/ρ_g;
  - sıkıştırılınca hacmi V·ρ_y/ρ_s.
- **Sıkışık V m³ dolgu:**
  - istediği yerinde hacim V·ρ_s/ρ_y;
  - istediği gevşek hacim V·ρ_s/ρ_g.

### Hal tanımı

Kütle ordinatı **gevşek m³** ile kurulur: kazının gevşek hacmi ile dolgunun istediği gevşek hacim karşılaştırılır.
Kamyon gevşek taşır. Bu yüzden:

- kabarma (kazı katsayısı) = ρ_y/ρ_g;
- dolgu katsayısı = ρ_s/ρ_g.

Mevcut katsayıların sayısal anlamı değişmedi (kazı × kabarma, dolgu × dolgu katsayısı). Değişen, bu tanımın açıkça
yazılması:

- grafikteki açıklama "Σ dolgu (sıkışık)" yerine "Σ dolgunun istediği (gevşek)" oldu;
- yardım metni tanımı söyler.

### Model (KCAD şema 174)

Katsayı satırına iki alan eklendi:

- `densities` {`bank`, `loose`, `compacted`} (t/m³, 0,5–4). Yoğunluklu satırda katsayılar yoğunluklardan çıkar
  (`VolumeFactorDef::effective`); satıra ayrıca katsayı yazılmaz.
- `source`: kaynağın sözlü adı (zemin etüdü, sondaj, tablo; en çok 200 karakter).

Katsayının onayı kübajın onayıyla taşınır (ADR 0906 §5, 0950). Onay kübajın sürümünü ve satırlarını tutar; katsayı
satırı değişince kübaj sürümü değişir ve onay eskir.

### `road.volumes.states` (salt okur)

Her katsayı satırı için:

- kaynağı ve yoğunlukları;
- etkin katsayılar;
- ölçülen kazı (yerinde) ve dolgu (sıkışık);
- gevşek halleri;
- yoğunluk varsa dolgunun istediği yerinde hacim, kazının sıkışık hacmi ve iki kütle.

Satıra düşmeyen aralıklar "katsayısız" olarak ayrı verilir. Toplamlar ve denge (gevşek) her zaman verilir; kütleler
yalnız her aralığın yoğunluğu varsa verilir.

### Zeminin yapamayacağı

`volume_factor_physics` uyarısı şunları söyler:

- **Kabarma < 1:** gevşek hacim yerindekinden az olamaz (ρ_g ≤ ρ_y).
- **Dolgu katsayısı < 1:** dolgunun istediği gevşek hacim sıkışık hacminden az olamaz (ρ_g ≤ ρ_s). Katsayı yerinde
  hacme göre yazıldıysa kütle ordinatı karışık hallerle kurulmuş olur; uyarı bunu söyler.
- **Yoğunluklarda:** ρ_g > ρ_y ya da ρ_s < ρ_g.

Kayada sıkışık yoğunluğun yerindekinden düşük olması (ρ_s < ρ_y, kaya dolgusu kabarır) geçerlidir; söylenmez.

### Pencere

Kübaj penceresinde iki ekleme var:

- **"Zemin yoğunlukları" tablosu:** Katsayı satırlarıyla aynı satırlarda yerinde, gevşek, sıkışık ve kaynak.
- **"Hacim halleri" görünüşü:**
  - Toplamlar bir cümle olarak yazılır, zeminin yapamayacağı uyarı şeridinde söylenir.
  - Tablonun sütunları: Km (⚠ işaretiyle), yoğunluklar ya da katsayılar, kazı → gevşek, dolgu → ister gevşek, ister
    yerinde.
- **Açıklama:** Grafik açıklaması yazı genişliğine göre dizildi. Önceden 140 px sabit aralıkla diziliyordu ve
  uzayan yazı bir sonrakinin üstüne biniyordu.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_volume_states_command_cases.py`, KentOS kodu kullanmaz: aralıklar
`road_volume_command_cases.py`'nin, haller kütle korunumundan. 22 durumun hepsi Rust'ta ve Python'da geçer.
Kübaj düzenleme komutuna 4 yeni durum eklendi.

Yoğunluklar örnektir: 1,80 / 1,45 / 1,95 ve 1,75 / 1,30 / 1,85 t/m³, el kitaplarının sıradan toprak ve kil için
verdiği büyüklükte. Projeninkini zemin raporu verir.

### Ana kübaj

Kazı yerinde 25 878 m³, dolgu sıkışık 3 507 m³.

| Katsayılar | Kazı gevşek | Dolgunun istediği gevşek | Fazla (gevşek) | Not |
|---|---|---|---|---|
| Örnek (1,25 / 0,9 ve 1,2) | 31 612 m³ | 3 495 m³ | 28 116 m³ | Dolgunun istediği dolgudan küçük: uyarılıyor |
| İki yoğunluk satırı | 33 667 m³ | 4 982 m³ | 28 685 m³ | Kütle kazı 45 845 t, dolgu 6 501 t |
| Kaya (2,6 / 1,6 / 2,1) | 42 052 m³ | — | 37 449 m³ | |

## Açık

- **Brükner'in hali:** Brükner ve taşıma planı (ADR 0907, 0924) hacimleri kübajdan gevşek m³ olarak alır. Taşıma
  maliyeti ve kamyon seferi (ADR 0925, 0936) gevşek hacimle hesaplanır. Yerinde hacimle ödenen birim fiyatlar için
  ayrıca dönüşüm gerekir.
- **Malzeme katmanları:** Bitkisel toprak ve diş kazısı (ADR 0906 §6) halleri ayrı yazılmıyor.
