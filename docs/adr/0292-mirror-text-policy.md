# ADR 0292: Aynada yazıların politikası (MIRRTEXT): Okunur ya da Aynalı

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-06`: “Mirror text/attribute politikasını ayrı
  seçenek yap; kapı yönü, metin okunurluğu ve blok attribute değerleri rastlantısal değişmesin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0037 (Aynala), 0144 §3 ve §7 (blok, öznitelikler), 0145 §3 (Okunur yap), 0146 §4 (kılavuz), 0182 (çok satırlı
  yazı), 0184 §2 (tablo), 0196 §3 (eğri boyunca yazı), 0280 (tutamaç kipleri), 0291 (dönüşümlerin her türde doğrulanması).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Aynada yazının kuralı çekirdeğe gömülüydü (“okunur kalır”, MIRRTEXT = 0) ve seçilemiyordu; üstelik kural yalnız yazının doğrultusuna
yarım tur ekliyordu. Bu, düşey bir eksende doğru sonuç veriyordu ama yatay bir eksende yazı **baş aşağı** kalıyordu (doğrultusunun
görüntüsü doğuya bakarken bir yarım tur daha); hizası da değişmediği için yazı aynadaki yerini doldurmuyordu, kutusu kayıyordu. Aynı kural
blokların içindeki yazılara ve öznitelik yazılarına da uygulandığından, aynalanmış bir kapı bloğunun numarası eksene göre bazen baş aşağı
çıkıyordu: “rastlantısal”. Python ve ajanın da yazıların ne olacağını seçecek bir alanı yoktu.

## Karar

### 1. Politika

`cad.entities.transform`'un aynasına `text` alanı: `readable` (varsayılan) ya da `mirrored` (`MirrorText`). Çekirdekte `TextPolicy` ve
`transform_shape_with`; `transform_shape` okunurdur.

Aynada yazının çerçevesi (taban doğrultusu, yukarısı) sol elli çıkar; eksenlerinden biri geri döndürülür:

| Politika | Taban doğrultusunun görüntüsü okunur (≤ 90° ya da > 270°) | Taban doğrultusunun görüntüsü ters okunur (90°–270°) |
|---|---|---|
| **Okunur** (MIRRTEXT = 0) | yukarısı geri döner: doğrultu görüntüdür, hizanın düşey payı değişir (taban ve alt → üst, üst → taban, orta kalır) | taban geri döner: doğrultu bir yarım tur daha, hizanın yatay payı değişir (sol ↔ sağ, orta kalır) |
| **Aynalı** (MIRRTEXT = 1, harfler ters yazılmadan) | taban geri döner (doğrultu bir yarım tur daha, sol ↔ sağ) | aynı |

Okunur yazı aynadaki yerini doldurur ve hiçbir zaman baş aşağı değildir (ADR 0145 §3'ün sınırı). Aynalı'da yazı çizimle döner, ters
okunabilir; KentOS'ta harfleri ters yazan bayrak yoktur, bu yüzden AutoCAD'in MIRRTEXT = 1'inin ters harfleri yoktur (aşağıda).

### 2. Türler

| Tür | Okunur | Aynalı | Hiç değişmeyen |
|---|---|---|---|
| yazı, çok satırlı yazı | yukarıdaki kural; kutu genişliği ölçekle | taban geri | metin, biçimler, satır aralığı, genişlik çarpanı |
| eğri boyunca yazı | köşeler aynalanır, ters çevrilir; eğri ters okunuyorsa bir daha, düşey payıyla (ADR 0196 §3) | yalnız ilk ters çevirme | metin |
| kılavuzun notu | doğrultu yukarıdaki kuralla | taban geri | not, ok |
| tablo | yukarısı geri dönünce yeni sol üst eski sol altın görüntüsü, taban geri dönünce eski sağ üstün | taban geri | hücreler, birleşik alanlar, hizalar |
| ölçü değeri | ölçünün yerleşimi değeri her zaman okunur yazar | aynı | değer |
| blok yerleştirmesi | geometrisi aynalanır (`mirror` bayrağı: kapının açılışı döner); kendi yazıları ve öznitelik yazıları her zaman okunur | aynı | öznitelik değerleri |
| tarama, çizgi, yay, alan, resim | geometri aynalanır (ADR 0291) | aynı | — |

### 3. Araçlar

Aynala'da **Yazılar (Y)**: okunur ya da aynalı; tutamaç kipi Aynala'da (ADR 0280) aynı seçenek. Seçim oturum boyunca ikisinde ortaktır
(`Memory::mirror_text_mirrored`) ve komuta açıkça verilir.

### 4. Doğrulama yolu

`scripts/fixtures/affine_reference.py`'nin `text_frame`'i kuralı bağımsız yazar (hiza tabloları `ACROSS`, `UP`); `transform_kinds_cases.py`
yazıyı, çok satırlı yazıyı, eğri boyunca yazıyı (ADR 0196'nın başvurusu `text_along_cases.py` ile), kılavuzu ve tabloyu eğik ve yatay
eksende okunur, iki eksende aynalı planlar; `transform_command_cases.py`'nin simetrik kopyası artık sağ tabana hizalı yazıdır.
Blokta: `block.rs`'in `a_mirrored_door_keeps_its_number_readable` testi (kapının yayı döner, numara okunur, değeri yerleştirmenin).

## Yapılmadı

- Harfleri ters yazan yazı (AutoCAD'in MIRRTEXT = 1'i, DXF'in 71 kodunun ters ve baş aşağı bayrakları): `.kcad` şeması, üç çizim hattı ve
  DXF'le ayrı bir iş; Aynalı onun yerine yazıyı çizimle döndürür.
- Bloğun içindeki yazılar için ayrı politika: yerleştirme yazılarını hep okunur çizer; AutoCAD'in öznitelikleri ayrı nesne olarak aynalaması
  KentOS'ta yok.

## Sonuçlar

- Yatay eksende aynalanan yazı, kılavuz notu, tablo ve kapı bloğunun numarası artık baş aşağı çıkmaz; yazılar aynadaki yerlerini doldurur.
- Python ve ajan `transform={"kind": "mirror", ..., "text": "mirrored"}` ile seçer; vermezse okunurdur.

## Doğrulama

- `python3 scripts/fixtures/transform_kinds_cases.py --check`, `python3 scripts/fixtures/transform_command_cases.py --check` (mpmath yoksa
  ince plakanın 50 basamaklı öz denetimi atlanır ve söylenir; yazılan durumlar ona bağlı değil); `cargo test -p kentos-native-application
  --test all`; `cargo test -p kentos-geometry-core` (blokta kapı; web'in donmuş çağrı kayıtları); `cargo test -p kentos-interaction`; izler;
  `pnpm py:test`; `python3 scripts/cad/manifest.py --check`.
- Resimler: `KENTOS_SHOTS_ONLY=ayna-kapi,ayna-yazilar-okunur,ayna-yazilar-aynali,ayna-secenek cargo test -p kentos-desktop tools_screens --
  --ignored --nocapture` (`.run/shots/arac-ayna-*`).
