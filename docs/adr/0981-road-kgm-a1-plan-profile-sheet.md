# 0981 — KGM'nin A1 plan–profil paftası

Tarih: 2026-11-06. Durum: kabul (Y75). Kapsam ROAD-SEC-11'in pafta yanı; ADR 0908 §10 ve 0980'in (Y74 bulgusu) devamı.

## Sorun

Y74'te KGM bantları paftalara geçti. Ama KentOS'un tek plan–profil şablonu A3 yataydı (`sys:yol-plan-boykesit`) ve
boykesit haritası kâğıtta 102 mm yüksekti. KGM'nin altı bandı 2,5 mm yazıyla 107,5 mm tutar, yani boykesitin kendisi
olmadan bile sığmaz: 1:1000'de Düşey Geometri ve Dever bantları haritanın dışında kalıyordu (`road_sheet_outside`).

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı

- **KGM 2019, Karayolları Kesin ve Ön Projeleri Mühendislik Hizmetleri Teknik Şartnamesi:**
  - **s. 20/64, Proje çizimleri (c):** "Proje belgeleri (paftalar); **594 x 841 mm (A1) normunda** parlak fotoğraf
    kağıdı (200-220 gr/m²) veya polyester". Yazılı belgeler A4'tür.
  - **s. 16–17/64, Plan-Profil Paftaları:**
    - madde 10: antette bölge, il ve kontrol kesim no bulunur, "veya idarenin uygun göreceği formda";
    - madde 17: paftalar baş paftadan (1) numaralanır; **pafta numarası ve toplam pafta sayısı sağ üst köşeye**
      yazılır;
    - madde 19: baş pafta dışındaki her paftanın **sağ üst köşesinde küçük bir klişe** bulunur "(KGM normlarına uygun
      olarak)"; imza ve onaylar yalnız baş paftanın büyük klişesindedir. Klişenin ölçüsü verilmez.
  - **s. 6/64:** profil paftada "ayrılan genişliğe" sığmazsa kot taşıma çizgisi kullanılır, yani boykesitin paftadaki
    yüksekliği sabittir.
  - **s. 35/64:** "Proje Sürekli Plan-Profil (Kırılmış ve Kırılmamış ayrı ayrı hazırlanmalıdır)". KGM hem paftalara
    bölünmüş hem bölünmemiş şeridi ister.
  - **s. 13–14/64:** "rulo halinde (… 10 veya 20 km'lik kesimler)" Brükner diyagramı içindir, plan–profil paftası için
    değil. ADR 0980'in "KGM rulo şablonu" adı bu yüzden yanlıştı; doğrusu A1 paftadır.
- **KGM'nin örnek paftaları** (şartnamenin EK-10–13'ü, s. 45–48; 16. Bölge, Sivas, K.Kes.No 58-07, 71 pafta), taranmış
  resimden ölçüldü (110 dpi; profil ızgarasının 10 m × 1 m hücresi 1 cm):
  - çerçeve eni ≈ 840 mm, yani A1'in uzun kenarı; bir pafta 700–800 m tutar (EK-13: 23+700 → 24+500);
  - yukarıdan aşağı: plan ≈ 235 mm, şev ve platform satırları ≈ 41 mm, profil ızgarası ≈ 191 mm, alttaki satırlar
    ≈ 70 mm;
  - klişe planın sağ üstünde, ≈ 157 × 35 mm: ≈ 20 mm'lik yol adı satırı, altında 7,5 mm'lik başlık ve değer satırları
    (BÖLGE · İL · K.KES.NO · DOSYA NO · PAF.NO · Σ PAF → 16 · SİVAS · 58-07 · – · 08/42 · 71).
  - Çözülmedi: çerçevenin yüksekliği ≈ 538 mm ölçüldü, 574 değil; tarama kırpılmış olabilir.
- **Büyük Ölçekli Harita ve Harita Bilgileri Üretim Yönetmeliği** (RG 26.06.2018, 30460 Mükerrer), md. 75: "Pafta
  boyutları tüm pafta ve kanavalar için 70 cm x 90 cm". Bu harita paftasıdır, yol plan–profil paftası değil. Resmî
  Gazete'de yol plan–profil paftasının boyutunu koyan bir metin bulunamadı; bağlayıcı olan KGM şartnamesi ve
  sözleşmedir.
- **TS EN ISO 5457** (TSE; T1, 18.12.2002) ve **TS EN ISO 7200**: Türkçe metinler ücretli, okunamadı.

### Standart

- **ISO 5457:1980**, s. 2–3 (okundu): A1 594 × 841; antet sağ altta, tanıtım kısmı en çok 170 mm; A0/A1'de kenar en az
  20 mm (10 mm'ye indirilebilir), solda isteğe bağlı en az 20 mm dosyalama payı. Uzatılmış boylar da tabloda: A2×3
  594 × 1261, A1×3 841 × 1783 …
- ISO 5457:1999'un kenar değerleri (sol 20 mm, öteki 10 mm, A1'de 811 × 574 çizim alanı) çokça anılır ama okunamadı.

### Yazılımlar

- **Netcad NETPRO:** plan–profil paftası kullanıcının şablonundan gelir, ölçüler kâğıt mm'sidir; örnek şablonlar
  `PPRO_A1` ve `PPRO_TCK`. Bindirme ve "Profili 100 metrenin katlarında kır" seçenekleri vardır. `PPRO_A1`'in ölçüleri
  alınamadı.
- **Civil 3D:** Create View Frames → Create Sheets, plan/profil görünüm pencereleri işaretli bir `.dwt` ile çalışır.
  Metrik şablonların adları doğrulanamadı.

## Karar

- **Yeni sistem şablonu `sys:yol-plan-boykesit-kgm`** (`crates/shared/sheet/templates/yol-plan-boykesit-kgm.json`):
  - A1 yatay 841 × 594; kenarlar sol 20, öteki 10 mm (ISO 5457); çerçeve 811 × 574.
  - **Plan haritası** üstte, 791 × 230 mm, 1:1000.
  - **Boykesit haritası** altında, 791 × 318 mm. KGM'nin örneğindeki profil (≈ 191) ve şev/platform satırları (≈ 41)
    ile KentOS'un KGM bantları (107,5) birlikte 339,5 mm eder; 318 mm bantları ve 210 mm'lik bir boykesiti taşır,
    şev/platform satırları henüz çizilmediği için onlara yer ayrılmadı.
  - **Klişe** sağ üstte, planın içinde, 160 × 35 mm (KGM s. 17, madde 17 ve 19; ölçü örnek paftadan): YOL ‹proje adı›;
    BÖLGE, İL, K.KES.NO; DOSYA NO, PAF.NO, TOP. PAF.
  - **Değişkenler:** `birim`, `il`, `kontrol_kesim`, `dosya_no`. KentOS bunlara değer koymaz; boşsa ön denetim
    "değerin değeri yok" hatası verir ve kâğıda `‹il?›` yazılır (öteki şablonlar gibi).
  - Σ yerine "TOP. PAF": şablonun yazı tipinde Σ yoktu, test yakaladı.
  - Kâğıtlar A1, A0, A2 yatay; küçük kâğıt için yatay/dikey A4 ve öteki türevler (şablon sınamasının kuralı).
- **`road.sheets.plan`:** `source` yoksa ve `bands: kgm` ise bu şablon kullanılır; yoksa eskisi gibi
  `sys:yol-plan-boykesit`. Açık bir `source` her zaman kazanır. KentOS bantlarıyla çizilen paftalar bayt bayt aynıdır.
- **Klişe neden sağ altta değil:** ISO 5457 anteti sağ alta koyar, KGM şartnamesi sağ üste. Yol paftası idareye
  teslim edilir; KGM'nin istediği yer seçildi.

## Ölçüm

- **Şablon sınaması** (`kentos-sheet`, 429 test): sistem şablonları 13; şablon üretilir, golden SVG yazıldı
  (`fixtures/sheet/v1/svg/yol-plan-boykesit-kgm.svg`), kip sıralamasında `sys:yol-plan-boykesit`'in ardından gelir.
- **Ortak durumlar:** `road.sheets.plan` 35 durum. İki KGM durumu artık A1'dedir: paftanın boyu 1:1000'de 370 m'den
  **791 m**'ye çıktı, 669 m'lik aks iki A3 yerine **tek A1** paftaya düştü. Boykesit görünümü 95 mm, bantlarla
  202,5 mm; 318 mm'lik haritaya sığar. Başvuru (`scripts/fixtures/road_sheets_cases.py`) A1 şablonun dosyasını okur.
  Rust ve Python tipli katman geçer.
- **Masaüstü testi:** KentOS bantlarıyla iki A3 (370 m), KGM bantlarıyla tek A1 (791 × 230 m); boykesit ve bantlar
  haritaya sığar; tek geri alma adımı.
- **Resimler** (`.run/gui-evidence/yol-y75/`): altı bandın hepsi boykesit haritasının içinde; klişe sağ üstte; ön
  denetim dört boş klişe değişkenini sayar.
- **Bulgu:** Örnek çizimin aksı 669 m'de çok döndüğü için planın 230 m'lik yüksekliğine sığmaz; `road_sheet_outside`
  bunu doğru söyler (boykesit değil, plan taşar).
- **Bulgu (resimle görüldü, Y10'dan beri):** Plan haritasında aksın kendisi çizilmiyor; yalnız noktalar ve km yazıları
  görünür. Bu şablonun değil, haritanın yol nesnelerini çizmemesinin sonucu; plana yazıldı.

## Açık

- **Plan haritasında aks çizgisi** (yukarıdaki bulgu).
- **Baş pafta ve büyük klişe:** KGM imza ve onayları yalnız baş paftada ister; takımın ilk paftası için ayrı şablon yok.
- **Şev/platform satırları** (≈ 41 mm) ve **kot taşıma çizgisi**.
- **Sürekli (kırılmamış) plan–profil şeridi** (KGM s. 35).
- **100 m'nin katlarında kırma:** KGM örnekleri 700–800 m'lik paftalardır; KentOS paftanın boyunu haritanın eninden
  alır (791 m). Netcad'in "100 metrenin katlarında kır" seçeneği gibi bir yuvarlama yok.
