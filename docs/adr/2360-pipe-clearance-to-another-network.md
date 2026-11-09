# 2360 — Başka bir altyapı ağının borularına açıklık

Tarih: 2026-11-08. plan/map-cad.md MAP-16, üçüncü tur. Durum: taslak. Plan satırı: “Diğer disiplinlerle yatay/düşey
açıklık ve yüzey kaplaması denetlensin.”

İlgili:
- [ADR 1640](1640-gravity-pipe-check.md), boru hattı denetimi. “Sonra”sındaki “disiplinler arası yatay ve düşey
  açıklık” maddesini bu ADR kapatır;
- [ADR 1860](1860-pipe-run-profile.md), hat profili. “Sonra”sındaki “kesişen başka hatlar ve disiplinler arası düşey
  açıklık” maddesinin açıklık kısmını da bu ADR kapatır. Profilde kesişen borunun çizilmesi sonraya kalır.

Planın satırındaki “yüzey kaplaması” bu ADR'de yoktur.

## Dayanak

- **Autodesk Civil 3D, Interference Check** (8 Kasım 2026'da okundu):
  - kaynaklar: [About Checking for Pipe Network Interferences](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-B7947549-4D31-476F-BC1C-880397DF20C6.htm),
    [Criteria Tab](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-9E485F3F-538A-4131-9714-A5D31C95E0C9.htm);
  - denetim bir ağın kendi parçaları arasında ya da iki ağ arasında yapılır;
  - fiziksel çakışmayı bulur: birbirine giren, çarpışan ya da birbirini kesen parçalar;
  - 3B yakınlığı bulur:
    - “Use Distance”: verilen uzaklıktan yakın parçalar, her yönde;
    - “Scale factor”: parçanın boyutu × çarpan;
  - belgenin örneği: atıksu ile içme suyunun kesişiminde su hattı genellikle en az 18 inç üsttedir. Değer örnektir.
- **Utah Administrative Code R309-550-7**
  ([Cornell LII](https://www.law.cornell.edu/regulations/utah/Utah-Admin-Code-R309-550-7), 8 Kasım 2026'da okundu).
  Resmî bir kuralın biçimine örnektir:
  - paralel su ve atıksu hatları arasında en az 10 ft yatay uzaklık;
  - kesişimde su hattı atıksu hattının en az 18 inç üstünde;
  - uzaklıklar kenardan kenara ölçülür.
- **Türkiye:** İller Bankası'nın ve idarelerin şartnamelerinden bir değer okunamadı. Arama Türkçe bir şartname metni
  vermedi. ADR 1640'ta da İller Bankası şartnamesine erişilememişti. KentOS değer koymaz; sınırlar kullanıcınındır.
- **KentOS bugün** (ADR 1640): `cad.pipes.check` bir ağın borularını yalnız kendi içinde denetliyordu. Başka bir ağın
  borusuyla kesişim ya da yakınlık yoktu.

## Karar

1. **`clearance` girdisi** (`cad.pipes.check`, isteğe bağlı):
   - diğer ağın kimliği ya da adı;
   - akar kotlarının yeri (öznitelik ya da köşe kotları) ve çapın yeri (öznitelik ve birim). Kurallar denetlenen ağın
     kurallarıyla aynıdır;
   - `vertical`: en az düşey açıklık (m);
   - `horizontal`: en az yatay açıklık (m);
   - `above`: kesişimde hangi ağın borusu üstte olmalı, `pipe` (denetlenen) ya da `other`.

   Sınır verilmezse yalnız çakışmalar bulgu olur. Kesişimler her durumda listelenir.
2. **Kesişim:** denetlenen bir borunun diğer ağın bir borusuyla planda kesiştiği yer.
   - Geometri kesindir: çizgi, çoklu çizgi (yaylarıyla) ve yay. Ağ zaten yalnız bunları kenar alır. Kesişimi
     çekirdeğin kenar kesişimi verir (kesin yüklemler). İki kenarın paylaştığı yer tek kesişimdir.
   - İki borunun akarı o yerde, boydaki istasyonla doğrusal okunur (eğim ve örtüdeki gibi). Öznitelik istenmiş ama
     boruda yoksa köşe kotlarına geçilmez.
   - **Üstteki boru:** iç çapının ortası (akar + D/2) yüksek olandır. Eşitlikte denetlenen boru üstte sayılır.
   - **Düşey açıklık:** üstteki borunun akarı − alttakinin tepesi (akar + D). Sıfırdan küçükse borular çakışır.
   - Çap sorulmamış ağın boruları kalınlıksız sayılır, örtüde olduğu gibi akara ölçülür. Çap sorulmuş ama boruda
     yoksa açıklık bilinmez.
   - **Cevapta `crossings`:** iki boru, yer, iki akar, açıklık, üstteki. Akar ya da sorulan çap yoksa açıklık ve
     üstteki yazılmaz. Sıra: denetlenen boru, diğer boru, sonra boru boyunca.
3. **Yatay açıklık:** kesişmeyen her çift için eksenler arası en kısa uzaklık.
   - Kesişmeyen iki kenar ya bir kenarın ucunda en yakındır ya da ortak dikmenin ikisine de değdiği yerde. O dikme
     doğrunun yay merkezinden geçen dikmesi ya da iki yayın merkezlerinden geçen doğrudur. Böylece uzaklık kesindir.
   - Açıklık = uzaklık − iki yarıçap.
   - En azdan küçükse çift `near`'a yazılır: iki boru, iki en yakın yer, uzaklık, açıklık. Birçok yer aynı uzaklıktaysa
     (paralel borular) yerlerden biri yazılır.
   - Kesişen çift yatayda denetlenmez; onda düşey kural geçerlidir.
   - Sorulan çap yoksa açıklık bilinmez. Çift yalnız eksenleri en azdan yakınsa yazılır: o zaman çap ne olursa olsun
     açıklık en azdan küçüktür.
4. **Bulgular** (denetlenen borunun, değerleri açıklık):
   - `clash`: çakışma;
   - `closeCrossing`: düşey açıklık en azdan küçük;
   - `wrongSide`: üstte olması istenen boru altta;
   - `closeAlongside`: yatay açıklık en azdan küçük.
5. **Et kalınlığı bilinmez.** Çaplar iç çaptır (ADR 1640). Açıklıklar, dıştan dışa ölçülen açıklıktan et kalınlıkları
   kadar büyüktür. Katalogdaki not ve pencerenin özeti bunu söyler.
6. **Sınırlar:** kesişim ve yakın geçiş listeleri en çok 5000 satırdır (bulgu sınırı). Diğer ağda en çok 100 000 boru
   olur. Diğer ağın kutuları paketli bir R-ağacındadır; her boru yalnız kutusu değen borulara bakar.
7. **Retler** (sırasıyla):
   - önce denetlenen ağın sorusu;
   - sonra açıklığın sorusu (`invalid_pipe_check`, yerleri `clearance.` ile başlar): boş öznitelik adı, katalog, 0'dan
     büyük olmayan en az açıklık;
   - sonra denetlenen ağ ve yüzey;
   - en son diğer ağ, hepsi `clearance.network`'te: `unknown_network`, `not_a_pipe_network`, `invalid_pipe_check`
     (denetlenen ağın kendisi ya da 100 000'den çok boru), `invalid_network`.
8. **Masaüstü** (Boru hattı denetimi):
   - **Diğer ağ** satırı: Yok ya da öbür altyapı ağları ve akar (Öznitelik ya da Köşe kotları). Ağ seçilince akarın ve
     çapın yeri ağın borularından tahmin edilir, pencere açılırken denetlenen ağınki gibi;
   - **Diğer ağın çapı** satırı: öznitelik ve birim;
   - **Açıklık (m)** satırı: düşey en az, yatay en az, üstte (Fark etmez, Bu ağ, Diğer ağ);
   - özette Diğer ağ satırı; tabloda **Kesişim** ve **Yakın geçiş** görünümleri.

   Yeni şerit düğmesi yoktur. Yeri Arazi › Drenaj › Boru denetimi'dir (Harita, Yol), CBS'de Analiz › Ağ analizi.
9. **Kapsam işareti:** ADR 1640'taki gibi **Harita + Yol + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/pipes_check_cases.py` (mpmath 1.4.1, 50 basamak), KentOS kodu yok.
  - Çizime **İçme suyu** ağı eklendi: `su` katmanında W1–W7. Bunlar 3B çoklu çizgilerdir, Çap mm'dir; W7'nin kotu
    yoktur.
  - Kesişimler kapalı biçimde bulunur: doğru ile doğru, doğru ile çember (ikinci derece denklem). Yayın merkezi
    açısından hesaplanır: r = c / (2 sin(θ/2)), kirişin ortasından r·cos(θ/2) uzakta.
  - En yakın yerler aynı adaylarla elle aranır. Sık bir örneklemeyle de denetlenir.
  - Sonuçlar:

    | Kesişim | Akar (m) | Diğer akar (m) | Açıklık (m) | Üstte |
    |---|---|---|---|---|
    | P1 × W3 | 97,750 | 98,467 | 0,417 | su |
    | P2 × W1 | 97,550 | 97,667 | −0,183 (çakışma) | su |
    | P3 × W2 | 96,800 | 96,067 | 0,533 | atıksu |
    | P4 × W7 | 94,350 | — | — | — |
    | Q3 (yay) × W4 | 97,700 | 98,440 | 0,240 | su |

    Yatayda P3–W5: eksenler 3,000 m, açıklık 2,800 m. Q3–W6: 3,000 m, 2,700 m (yayın dibinde).
  - Su borularının çapı sorulmazsa P2 × W1'de atıksu üstte olur: −0,117 m, yine çakışma. Su borularında akar
    özniteliği istenince hiçbir kesişimin açıklığı bilinmez, köşe kotları yerine geçmez.
  - 44 durum (30'dan), 8'i ret. Eski 30 durumun cevabı aynıdır. Eklenen katman ve ağ yalnız çizimdedir.
  - Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/pipe_clearance.rs`): durumun çizimi `.kcad` olarak açılır.
  - Üç durumun kesişimleri, yakın geçişleri ve bulguları durumlarınkidir: en az düşey açıklık ve su üstte, en az yatay
    açıklık, yağmursuyunun yayı.
  - Denetlenen ağ diğer ağ olarak verilince `clearance.network`'te reddedilir.
- **Masaüstü** (`pipes_check_window_tests.rs`, `the_clearances_to_the_water_network_are_the_shared_cases`):
  - Diğer ağ'da İçme suyu seçilince köşe kotları ve Çap tahmin edilir;
  - düşey 0,5 ve “Diğer ağ üstte” ile kesişimler ve sayımlar durumunkidir;
  - yatay 3 ile yakın geçiş durumunkidir;
  - Yok seçilince açıklık kalkar, tablo borulara döner; çizime bir şey yazılmaz.

  Çapraz masaüstü denetimleri 139 geçer. Resimler `.run/gui-evidence/boru-acikligi-2360/`.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("altyapi.kcad")
r = cad.pipes.check(doc, network="atiksu", inverts={"from": "fields", "start": "Akar baş", "end": "Akar son"},
                    diameter={"field": "Çap", "unit": "mm"},
                    clearance={"network": "su", "inverts": {"from": "heights"},
                               "diameter": {"field": "Çap", "unit": "mm"}, "vertical": 0.5, "horizontal": 3.0,
                               "above": "other"})
print([(c.pipe, c.other, c.clearance, c.above) for c in r.crossings])
```

MCP: `{"tool": "cad.pipes.check", "arguments": {..., "clearance": {...}}}`.

## Sonra

- Yüzey kaplaması (planın satırının kalanı).
- 3B en kısa uzaklık. Eğimli borularda bu, kesişim yerindeki düşey açıklıktan biraz kısadır.
- Et kalınlığı: malzeme ve dış çap özniteliği. Et kalınlığı özniteliği yapıldı: [ADR 3960](3960-pipe-wall-thickness.md)
  (dış çap özniteliği Sonra).
- Diğer ağın yapılarına (baca, vana odası) açıklık.
- Kesişimlerin planda gösterilmesi. Yapıldı: [ADR 3160](3160-pipe-crossings-on-the-drawing.md).
- Hat profilinde kesişen borunun kesiti (Civil 3D'nin crossing pipe'ı). Yapıldı:
  [ADR 2760](2760-crossing-pipes-in-the-pipe-profile.md).
- EK-1'in ve idarenin değerleri sürümlü bir kural paketinde (ADR 1640 Sonra).
