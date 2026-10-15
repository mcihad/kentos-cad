# ADR 0310: Çizim sırası, Nesnenin üstüne ve altına, maske

- **Durum:** kabul edildi (2026-10-15). `plan/cad.md` §8 `CAD-EDIT-23`: “Drawing order, bring front/back, text above hatch, wipeout ve
  boundary görünürlüğünü tutarlı yap.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0260 (nesnenin görünüşü ve çizim sırası, `cad.entities.order`), ADR 0264 §3 (wipeout'un kaynak modeli), ADR
  0192 (yalnız kenarından seçilen resim), ADR 0278 (kilitli katman), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0260 katmanın içinde çizim sırasını (`order`) ve Öne getir ile Arkaya gönder'i getirdi. ADR 0264 maskeyi alanın görünüşü olarak
tanımladı ama uygulamayı bu maddeye bıraktı. Eksik kalanlar:

- **Sıra:** AutoCAD'in DRAWORDER'ının Üstüne ve Altına'sı (bir nesneye göre), TEXTTOFRONT (yazılar öne) ve HATCHTOBACK (taramalar
  arkaya).
- **Maske:** modeli, çizimi, seçimi, çerçevenin görünürlüğü, aracı ve komutları.
- **Bir sınırın yazılması:** çizimin yazıları ve etiketleri sahnenin üstündeki ayrı katmanda çizilir (ADR 0055).

## Karar

### 1. Maskenin modeli

- **Alan:** maske alanın (`polygon`) görünüşüdür: `ObjectLook.wipeout`, `{ frame: bool }`. Yeni tür değildir; alan işlemleri,
  tutamaçlar ve dönüşümler aynen geçerlidir.
- **Ad:** ADR 0264 alanın adını `mask` koymuştu. Görünüş nesnenin JSON'una düz yazılır (`flatten`); yazının ve ölçünün kendi
  `mask: bool` zemini (ADR 0145, 0147) ile çakışmasın diye ad `wipeout`'tur.
- **Yalnız alanda:** başka türde maske kuralı ihlal eder (`wipeout_problem`):
  - komutta `invalid_wipeout`;
  - dosyada `bad_value`;
  - sunucuda ret.
- **Blok tanımında da** olur (etiket zemini gibi).
- **Dosya (KCAD):** şema 54 (`SCHEMA_WITH_WIPEOUTS`).
  - Yazıcı 54'ü yalnız maske varken yazar.
  - Eski okuyucu `wipeout`'u bilinmeyen alan sayar ve dosyayı açmaz.
  - Tipli sütunlarda görünüşün iki biti vardır (32: maske, 64: çerçeve).
  - `FORMATS_VERSION` 64.
  - Spesifikasyon `docs/specs/kcad-v2.md`'dedir.
- **Sunucu:** maske `feature.look` jsonb'sinde durur; göç gerekmez.

### 2. Çizim

- **Dolgu:** maske çizim zemininin rengiyle opak dolar (`paper` jetonu, kullanıcının çizim zemini dahil). Renk kipleri (Tek renk,
  Gri) zemini değiştirmez.
- **Çerçeve:** `frame` açıksa sınır nesnenin renginde ve kalınlığında, sürekli çizilir; kapalıysa çizilmez.
- **Neyi örter:**
  - katmanın çizim sırasında kendinden önce çizilenleri;
  - kendi katmanının altındaki katmanları (katmanların sırası ağaçtadır).
- **Çizim sırası bantları:** bant içinde dolgular çizgilerden önce çizilir. Maskeli katmanda her maske kendi bandıdır (önceki ve
  sonraki nesneler ayrı bantlarda, `(order, maske sayacı)` sıralarından); bu yüzden maske kendinden önceki çizgileri de örter.
  - Maskeli katman, sıralı katman gibi parçalara bölünmeden bütün kurulur.
  - Maskesiz katmanın partileri değişmez.

### 3. Çizim sırası: Nesnenin üstüne ve altına

`cad.entities.order`'ın `to`'suna `above` ve `under` eklendi; başvurusu `reference` (kalıcı kimlik).

- **Yer:** nesneler, kendi çizim sıralarıyla, başvurunun hemen sonraki (önceki) sıralarını alır.
- **Kaydırma:** başvurunun öbür yanındaki nesnelerde yer yoksa hepsi aynı adımla kayar; öbür hiçbir çiftin sırası değişmez.
- **Değişmeyen:** zaten hemen üstünde (altında) olan nesneler değişmez.
- **Çıktı:** önce taşınanlar girdinin sırasıyla, sonra yer açmak için kayanlar.
- **Katman:** çizim sırası katmanın içindedir; başka katmandaki nesne `not_same_layer` ile reddedilir. Kilitli katmandaki nesne
  atlanır ve sayılır (ADR 0278).
- **Başvurunun kuralı (`invalid_reference`):**
  - üstüne ve altına başvuru ister;
  - öne ve arkaya istemez;
  - başvuru taşınan nesnelerden biri olamaz.

  Biçimi bozuk başvuru `invalid_uid`, çizimde olmayan `entity_not_found` alır; ikisinin de yeri `reference`'tır.
- **Ek düzeltme:** işleyici nesnenin belgedeki yerini her nesne için bütün çizimde arıyordu (n²). Yer tablosu artık bir kez kurulur.

Yazıları öne getir ve Taramaları arkaya gönder iki adımdır, iki komutla:

1. **Seçim:** görünen bütün yazılar, ölçüler, kılavuzlar ve tablolar (ya da taramalar) `cad.entities.select` ile seçilir.
2. **Sıra:** `cad.entities.order` ile öne ya da arkaya alınır. Python ve ajan aynı iki çağrıyı yapar.

Yazının taramanın üstünde kalması ayrıca güvencededir: çizimin yazıları zaten sahnenin üstündeki katmanda çizilir.

### 4. Seçim

- **Maske:** içinden değil, sınırından seçilir (resim gibi); böylece örttüğü nesneler tıklanabilir kalır.
- **Depo:** maskeleri işaretler (`Store::set_masks`, `is_mask`). İşareti masaüstünün eşitlemesi (`Spatial`) ve komutların deposu
  (`store_of`) yazar.

### 5. Araçlar ve komutlar

- **Maske** (`tool.wipeout`; WIPEOUT, MASKE): yol aracının bir biçimi; yaylar, yazılan noktalar ve kenet kendiliğinden gelir.
  - Halka `cad.polygon.create`'in yeni `wipeout`'uyla etkin katmana yazılır. Çakışma kırpmaz, Topoloji birleştirmez: maske örter.
  - **Çerçeve (Ç):** çerçeveyi açar ya da kapar (başta açık; AutoCAD'in WIPEOUTFRAME'i, maske başına).
  - **Nesne (N):** tıklanan kapalı alanı `cad.entities.set`'in `wipeout`'uyla maske yapar.
  - İki seçenek de oturumda kalır.
- **Öznitelikler'de Maske** (Yok, Çerçeveli, Çerçevesiz): tek alanda ve bütün seçim alanken. Yazma `cad.entities.set`, işlem
  `wipeout`, adımın adı “Maske”; `null` sıradan alana döndürür.
- **Nesnenin üstüne ve altına** (`tool.drawOrderAbove`, `tool.drawOrderUnder`): önce nesneler, sonra başvuruya tıklanır. Ret
  söylenir ve araç başka bir başvuru bekler.
- **Yazıları öne getir** (`edit.textsToFront`; TEXTTOFRONT) ve **Taramaları arkaya gönder** (`edit.hatchesToBack`; HATCHTOBACK).
- **Şeritteki yerleri:**
  - CAD'de Maske Giriş › Çizim'de; üstüne ve altına Değiştir'de, öbür ikisi onun ▾'inde.
  - CBS'de hepsi Düzenle › Nesne'de.
  - İkonlar `catalog/v1/additions.json`'dadır.
- **Python ve MCP:** `kentos.cad.entities.order`'ın `reference`'ı, `kentos.cad.polygon.create`'in ve `.entities.set`'in `wipeout`'u
  (SDK yeniden üretildi). MCP araçları katalogdan gelir.
- **Manifest:** beş komut `semantic`'tir.

### 6. Sınırlar ve sonraya kalanlar

- **Yazılar örtülmez:** maske altındaki yazıları ve etiketleri örtmez. Onlar sahnenin üstündeki katmanda çizilir; yazıyı maskeyle
  örtmek için o katmanın da çizim sırasını bilmesi gerekir.
- **DXF'in WIPEOUT'u:** okunmaz ve yazılmaz; okuyucu bugünkü gibi atlar ve söyler. Kırpma sınırının koordinat kuralı (görüntünün
  birim karesinde, ekseni ters) bağımsız denetlenemedi: elde AutoCAD'in yazdığı bir WIPEOUT örneği ve ezdxf yok. Tahminle yazılmadı;
  TODOS.md'de.
- **Katmanları aşan çizim sırası** ve SORTENTSTABLE ADR 0260 §8'deki gibi sonraya kalır.

## Performans

Süreler sürüm derlemesinde, 100 000 nesneli tek katmanda (her onuncusu yazı), bu makinede:

| İş | Değişen | Süre |
|---|---:|---:|
| Yazıları öne getir (10 000 yazı) | 10 000 | 37 ms |
| Nesnenin üstüne, başvuru katmanın başında | 99 998 | 171 ms |
| Nesnenin üstüne, başvuru sonda | 3 | 9 ms |
| Nesnenin altına, başvuru başta | 3 | 10 ms |
| Nesnenin altına, başvuru sonda | 99 997 | 172 ms |

En kötü durum, başvurunun öbür yanındaki bütün nesnelerin kaymasıdır. Daha az nesneli yanı kaydırmak sonraki bir iyileştirmedir.

```text
cargo test --release -p kentos-native-application --test all order_cmd::timing -- --ignored --nocapture
```

## Doğrulama

- **KCAD:** `fixtures/kcad/v2/wipeouts.kcad` (çerçeveli ve çerçevesiz maske, sırayla, blok tanımında maske) ve beş bozuk örnek.
  Örnekler bağımsız Python yazıcısıyla (`kcad_v2_reference.py`) yazıldı; bağımsız okuyucu (`tools/kcad/kcad.py`) ve Rust kodeği
  aynı sonucu verir (`tests/all/wipeouts.rs`: kodek, sütunlar, şema 54, retler). `schema-version-54.kcad` artık `-55`'tir.
- **`cad.entities.order`:** `fixtures/commands/v1/cad.entities.order.json`'a üstüne ve altına için on durum eklendi (34 durum). Kural
  `order_command_cases.py`'de ADR'den, KentOS kodu olmadan yazıldı. Kapsananlar:
  - kaydırma ve kaydırmasız yer, değişmeyenler, birden çok nesne;
  - kilitli nesne, plan;
  - sınır (`order_limit`), başvurunun kuralı, başka katman.
- **`cad.entities.set`:** `set_command_cases.py`'ye üç maske durumu eklendi (59 durum).
- **`cad.polygon.create`:** çerçeveli ve çerçevesiz maske eklendi (28 durum).
- **Yetenekler:** `capabilities.rs`'in testi yeni kodları durumlarda bulur.
- **Araç:** `crates/native/interaction/tests/all/wipeouts.rs`:
  - Maske'nin çerçeveli ve çerçevesiz yazması, Nesne;
  - maskenin içinden değil sınırından seçilmesi, maske kalkınca içinden seçilmesi;
  - Nesnenin üstüne ve başka katmanın reddi.
- **Resimler:** `apps/desktop/src/wipeout_scenes.rs` (`.run/shots/arac-maske-*`):
  - Maske aracı açıkken iki maske taramayı ve çizgileri örter;
  - Öznitelikler'in Maske satırı, Nesnenin üstüne;
  - CBS'nin Düzenle sekmesi;
  - 1440×900 ve 1100×650, açık ve koyu.
