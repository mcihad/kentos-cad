# ADR 9228: Dolgu, alanını boşluksuz doldurur: köprüler, değen delikler, kesişen halkalar

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-22, 2. adım.
- **İstek:**
  - planın RM-22 maddesi: “tarama sınırları … ele alınır”;
  - sahibin 23 Kasım isteği: “veri kaybolmasın, CAD gibi görünsün”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. `.kcad`, komut, ayar ve stil topluluklarının biçimi aynı. Geçerli bir çokgenin üçgenleri
  bayt bayt aynı. Yalnız bozuk çokgenlerin (halkası kesişen, deliği dış çizgiye ya da başka deliğe değen) üçgenleri
  değişti.
- **[ADR 0008](0008-shared-core-boundary.md):** üçgenleyici TypeScript'ten birebir taşınmıştı. Bu karar, o ADR'nin
  “sağlam kararlar ayrı dilimde, yalnız Rust'ta” dediği dilimdir.

## §1 Bulgu

### §1.1 Taramada üçgen boşluklar

SİVAS AVM DWG'sinin alan paftalarında ([ADR 9227](9227-dwg-regions-wipeouts-and-images.md) §4), RM-22'nin 1. adımından
önce de, çok delikli taramalarda büyük üçgen boşluklar vardı. Bölgelerin sınırı, taramanın neresinin eksik kaldığını
açıkça gösteriyordu.

Dolgu üçgenlemesi (`geometry-core` `triangulate.rs`) taramaları dışa alıp tek tek sınayınca:

- **Brüt alan taramaları** (A_ANNO_AREA_GROS, 11 tarama): çoğunun girdisi temiz, kendini kesen ya da birbirine değen
  halka yok. Yine de 10'unun alanının yalnız %57,9 ile %92'si doluyordu (#593664: 132,53 m²'nin 76,72'si).
- **Yapı taramalarının** (A_STR_PAT) altısının sınırı kendini kesiyor (3–40 kesişme): %19,6–76,7.
- **İz:** kulak kırpıcı 50 köşe ve 55,8 m² kala kulak bulamıyor, kalanı bırakıyordu (“degenerate ring; drop the
  remainder”).

### §1.2 Kök neden: köprü başka bir köprünün üstünden kuruluyordu

Delikler, sağdan sola, dış halkaya köprüyle bağlanır (Eberly). Önce bağlanan delikler halkanın parçası olur. Bir
köprünün iki kenarı aynı doğru üzerindedir ve uçları halkanın iki kez geçtiği noktalardır. Eski kodda iki eksik vardı:

- **Işın yön gözetmiyordu.** Bir köprünün iki kenarından deliğe bakmayanını da seçebiliyordu.
- **Aday noktanın kopyası denetlenmiyordu.** Halkanın birden çok geçtiği bir noktada, köşesi deliği tutmayan geçişe
  bağlanabiliyordu.

Sonuçta halka kendi üstüne biniyordu. #593664'te bir dış köşede üç geçişin açı dilimleri çakışıyordu.

**En küçük örnek:** bu tarama, üç köşeli bir dış halka ve üç deliğe indirildi; eski kod alanın %30,8'ini dolduruyor.
Hata sayıların son hanesine bağlı: 0,1 mm'ye yuvarlanınca kırpıcı dolduruyor.

### §1.3 Değen ve kesişen halkalar

- **Köşede değen delikler** (geçerli bir çokgende de olur) aynı yoldan bozuluyordu.
- **Kendini kesen bir halkanın** dolgusu kulak kırpmayla bulunamaz: kesişme noktaları üçgenin köşesi olmalı, girdide
  yoklar.

Donmuş çağrı fikstürlerinden 7'si (`calls-p8-triangulate.json`, TypeScript'in kaydı) bu türdendi. Eski çıktıları
nokta örneklemesiyle ölçüldü: fazla, eksik ya da çift kaplıyordu (14 400 örnekte 300–3 470).

## §2 Karar

### §2.1 Köprü, deliğe bakan kenara ve deliği tutan köşeye kurulur

`bridge` artık:

- yalnız iç tarafı deliğe bakan (yukarı giden) kenarları sayar;
- üçgendeki adaylardan yalnız köşesi deliği tutanı (`locally_inside`) alır;
- aynı noktanın kopyaları arasında içteki dilimi seçer (`holds_corner`).

İkisi de earcut'ın kuralıdır. Geçerli bir çokgende seçilen nokta değişmez: Eberly'nin en küçük açılı görünür noktası
yine odur.

### §2.2 Kırpıcı kaplamazsa earcut

Mapbox'un earcut'ı (v3.0, ISC lisansı, metni dosyanın başında) `triangulate/earcut.rs`'e taşındı:

- deliklerin köprüsü sektör seçimiyle;
- kulak bulunamazsa önce yinelenen ve doğrusal noktalar süzülür, sonra küçük kesişimler onarılır, sonra halka geçerli
  bir köşegenden bölünüp iki yarısı ayrı kırpılır.

Bağımlılık eklenmedi: çekirdek bağımlılıksız ve belirlenimci kalır. z-sıralı karma (hash) yok; yalnız bozuk
çokgenlerde çalışır.

**Sıra** (`triangulate_into`, indis döndürür):

1. eski kulak kırpıcı;
2. üçgenleri çokgenin alanını (dış halka eksi delikler, bir milyarda bir içinde) kaplamazsa earcut;
3. earcut kaplarsa onunki, kaplamazsa kırpıcınınki.

Alan hesabı halkanın kendi noktasına göre yapılır: TM koordinatında ham ayakkabı bağı formülü karşılaştırmanın
gerektirdiği haneleri yitiriyordu.

### §2.3 Kesişen halkalar örtüşme motoruyla çözülür

`fill_into` köşe döndürür (indis değil), bu yüzden yeni nokta ekleyebilir:

- **Kesişme varsa** (iki kenar ikisinin de ucu olmayan bir yerde kesişiyorsa; kesin yön testiyle, kutunun uzun kenarı
  boyunca süpürülür) doğrudan çözüme gider. Kendini kesen bir halkanın ayakkabı bağı alanı kendi alanı değildir;
  kaplama denetimi kandırılabiliyordu (bir örnekte %100,15).
- **Çözüm:** çokgen, dış halkası eksi deliklerinin birleşimi olarak çekirdeğin örtüşme motorundan geçer
  (`subtract_areas`; içerisi, halkanın sarım sayısının sıfır olmadığı yer). Çıkan basit alanlar üçgenlenir.
- **Kesişme yoksa ama kaplanmıyorsa** (dış çizgiye taşan ya da dışarıdaki delik) yine çözüme gider.

**Kullananlar:** stil çekirdeğinin dolguları (`batch.rs`, iki yerde: dolgular ve harita üstündeki resim) ve çağrı
tablosunun `triangulate`'i.

**İndis döndürenler** (`triangulate_into`, `triangulate_many`) yeni nokta ekleyemez: çizicinin ham sahnesi, yazı
glifleri, model görünümü kapakları, yol kesiti, IFC örgüsü, WASM. Bunlar §2.1 ve §2.2'den yararlanır; kesişen bir
halkayı yine eksik doldurabilirler.

### §2.4 Donmuş fikstürler

Web durdu ([ADR 0228](0228-web-pause-desktop-and-cloud-first.md)); iki donmuş dosya Rust'ın yazma kipleriyle
yeniden yazıldı. Yalnız değişen durumlar yazıldı, dosyanın geri kalanı bayt bayt aynı:

| Dosya | Yazma kipi | Değişen |
|---|---|---|
| `fixtures/geometry/v1/calls-p8-triangulate.json` | `KENTOS_WRITE_CALLS=1` (yeni) | 38 durumdan 11'i |
| `fixtures/style/v1/cases.json` | `KENTOS_WRITE_STYLE_CASES=1` (genişledi) | 48 katmandan 7'si, 804 sayı |

- **Çağrı fikstürü:** değişen 11 durumun hepsi bozuk girdi. Örneklemede 11'i de kusursuz (eksik 0, fazla 0, çift 0);
  eskiden 7'si değildi.
- **Stil fikstürü:** 7 katmanın her birinde dış çizgiye taşan bir tarama ya da alan deliği var. Yazma kipi artık bir
  topluluğun yalnız `from` ve `len`'inin değişmesine izin verir; başka her alan aynı kalmak zorunda.
- `fixtures/style/v1/batches.json` değişmedi.

## §3 Kanıt

**SİVAS AVM'nin beş ve daha çok delikli 23 taraması** (örtüşmeyle hesaplanan gerçek alana göre):

| Tarama | Önce | Sonra |
|---|---|---|
| Brüt alan, 11 tarama | %57,9–92 (biri %100) | %100 |
| Yapı, 6 tarama (halkası kesişen) | %19,6–76,7 | %100 |
| Döşeme, 4 tarama | %95,5–100 | %100 |

En pahalısı (#593258, 43 köşe ve 6 delik, kesişen; 2 890 üçgen) hata ayıklama derlemesinde 140 ms. Geçerli
taramalar eski yoldan geçer; onlara eklenen yalnız doğrusal süpürmedir.

**Sınamalar:**

- `kentos-geometry-core` `triangulate::tests`:
  - `holes_joined_across_a_bridge_are_filled_round`: AVM'nin en küçük örneği, sayıları son hanesine kadar; eski
    kodla %30,8;
  - `holes_touching_at_a_corner_are_filled_round`;
  - `a_ring_crossing_itself_is_filled_where_it_winds`: papyonun iki ilmeği, kesişme noktası köşe; papyon delik iki
    ilmeğini de çıkarır;
  - `a_hole_over_the_outline_takes_out_only_what_it_covers`;
  - `a_valid_polygon_is_filled_with_the_clippers_triangles`: köşe köşe aynı.

  9 sınama geçti, çağrı fikstürleri geçti, WASM derlemesi geçti.
- `kentos-style-core`: 85 + 9.
- `kentos-native-style`: 52 + 52 + 2 + 2.
- `kentos-render-wgpu`: GPU ile hepsi.
- Hedefli: `kentos-road` kesit, `kentos-model-view` kapaklar, `kentos-ifc` örgü.
- **Masaüstü:** 15 altın görüntünün hepsi piksel piksel aynı (0 fark); CAD uygunluk sınaması geçti.

**Gerçek pencere** (`.run/gui-evidence/render/rm22/`, sahibin ayarlarının kopyası):

- `once-sonra-tarama-ust.png`: alan paftalarının üst sırası, brüt ve net alan katmanları açık. Önce taramada büyük
  üçgen boşluklar; sonra bölgelerin sınırına kadar dolu, yalnız kolonlar ve boşluklar delik.
- `once-sonra-tarama-kat.png`: kat planları. Önce yeşil brüt alan dolgusunda beyaz kamalar; sonra düzgün.

## Uygulama

- `crates/shared/geometry-core/src/triangulate.rs`: `bridge` (yön, `locally_inside`, `holds_corner`),
  `triangulate_into` (kırpıcı, sonra earcut), `fill_into`, `crosses`, `covers`, `area_about`; `triangulate` köşelerden;
  sınamalar.
- `crates/shared/geometry-core/src/triangulate/earcut.rs`: yeni, earcut'ın taşıması.
- `crates/shared/style-core/src/style/batch.rs`: dolgular ve harita resmi `fill_into` ile.
- `crates/shared/geometry-core/tests/calls.rs`: `KENTOS_WRITE_CALLS=1`.
- `crates/shared/style-core/tests/all/style.rs`: yazma kipi toplulukların `from`/`len`'ini de yazar.
- `fixtures/geometry/v1/calls-p8-triangulate.json`, `fixtures/style/v1/cases.json`: yeniden yazıldı.
