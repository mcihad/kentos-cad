# 2060 — Düzenleme kapsamı: sınıra giren parseller, ayırma çapının sayıları ve esas alan (`cad.readjustment.scope`)

Tarih: 2026-11-07. plan/map-cad.md MAP-11, ikinci tur. Durum: taslak. Plan satırı: “Düzenleme, kamulaştırma/irtifak ve
arazi toplulaştırması için ortak teknik çekirdek tasarla: giriş kayıtları, kapsam geometrisi, değer/hak ilişkisi …”
İlgili:

- [ADR 1460](1460-readjustment-distribution-draft.md): Düzenleme dağıtım taslağı. Bu ADR onun “Sonra”sındaki kapsam
  geometrisini ele alır;
- [ADR 1400](1400-parcel-area-check-and-identity.md): Parsel alan denetimi, yanılma sınırı çizelgesi;
- [ADR 0202](0202-topology-rules.md) §2: dar parçanın sayılmaması (ortalama genişlik 2A/P).

## Dayanak

- **Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik**, Resmî Gazete 22.02.2020, sayı 31047. Resmî Gazete'nin PDF'i
  okundu (TÜRMOB'un yayın arşivindeki kopyası, 7 Kasım 2026):
  - **m. 4(c), ayırma çapı:** bir kısmı düzenleme sahasında kalan parsel için düzenlenen kroki. Üzerinde tescilli
    yüzölçümü ile sahaya giren ve girmeyen kısımların yüzölçümü vardır.
  - **m. 4(f), düzenleme sınırı:** parselasyon planı yapılacak alanın dış sınırı.
  - **m. 13(2):** çapta şunlar gösterilir:
    - tescilli ve hesaplanan yüzölçümü;
    - yüzölçümü farkı ve yanılma sınırı;
    - giren ve girmeyen kısımların yüzölçümü, sınırları ve koordinatları.
  - **m. 13(3):** dışarıda kalan parçaların her biri parsel numarası alıp tescil edilir.
  - **m. 31(1):** yüzölçümleri koordinatlardan hesaplanıp tescilli yüzölçümle karşılaştırılır:
    - fark yanılma sınırı içindeyse tescilli yüzölçüm esas alınır;
    - dışındaysa ilgili mevzuatla düzeltilmiş yüzölçüm esas alınır.
  - **m. 31(2):** kısmen giren parselde giren kısmın hesaplanan yüzölçümü esas alınır. Yüzölçüm hatası parselin
    uygulama dışında kalan kısmında bırakılır.
  - **m. 31(3):**
    - 2020'deki ilk metin iki bağıntı veriyordu: “f = 0,013 √(M·F) + 0,0003 F” (yapılaşmanın olduğu köy içi ve
      mahalle) ve “f = 0,0004 M √F + 0,0003 F” (diğer yerler).
    - Lexpera'daki güncel metin ise yanılma sınırı için Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında
      Yönetmeliğe (RG 27.09.2022/31966) gönderir. Okuma tarihi 7 Kasım 2026; değişikliğin resmî metni ayrıca
      okunmadı.
- **2022 Yönetmeliği** (ADR 1400):
  - **m. 8(2):** arşiv onaylı geometrinin artan oranlı çizelgesi. KentOS bunu `cad.parcels.check`'te uyguluyor
    (`tkgm-2022-m8-arsiv`).
  - **m. 8(5):** geçici koordinatlı parselin iç ve dış yanılma sınırı. Tablosu resim olarak duruyor; KentOS bunu
    hesaplamıyor.
- **KentOS bugün:**
  - Dağıtım taslağı tapu alanını kullanıcının CSV'sinden alıyordu; çizimle bağı yoktu.
  - Parsel alan denetimi fark ve yanılma sınırını veriyordu, ama düzenleme sınırını bilmiyordu.

## Karar

1. **`cad.readjustment.scope` v1, salt okunur (Harita + CBS).**
   - **Girdi:**
     - `boundary`: düzenleme sınırı, kapalı bir alan;
     - `uids`: yoksa parsel katmanının, sınırın içinde kısmı olan alanları. Sınırın kendisi listelenmez;
     - `field`: tescilli yüzölçümün özniteliği, ilk değer `Tapu alanı (m²)`;
     - `scale` ve `rounding`: kural paketinin sayı politikası;
     - `tolerance`: sayılan en dar parça, ilk değer 0,01 m.
   - **Kısımlar:** giren ve girmeyen kısımlar çekirdeğin kesin alan cebiriyle bulunur (`intersect_area_sets`,
     `subtract_areas`; yaylar yay kalır).
     - Bir parça, ortalama genişliği 2A/P toleranstan büyükse sayılır (topoloji kurallarının ölçüsü).
     - Tamamı giren: içeride sayılan parça var, dışarıda yok. Kısmen giren: iki yanda da var. Dışarıda: içeride yok.
   - **Ayırma çapının sayıları** (m. 13/2):
     - tescilli yüzölçümü yazıldığı gibi; hesaplanan yüzölçümü;
     - fark ve yanılma sınırı: `cad.parcels.check`'inkiyle aynı, m. 8(2) çizelgesi;
     - giren ve girmeyen kısım; dışarıdaki parça sayısı (m. 13/3).
   - **Esas alan** (m. 31):
     - tamamı giren ve yanılma sınırı içindeki parselde tescilli yüzölçümü (virgül noktaya çevrilir);
     - kısmen girende giren kısmın hesaplanan yüzölçümü. Alanın kendi en kısa ondalık yazımından politikanın basamağına
       bir kez yuvarlanır;
     - girmeyen kısma kalan = tescilli − esas, kesin.
   - **Esas yok:**

     | Durum | Neden |
     |---|---|
     | `needsCorrection` | Yanılma sınırı dışında. Düzeltilmiş yüzölçümü kadastronun işidir; KentOS düzeltmez. |
     | `noRecord`, `invalidRecord` | Kayıt yok ya da okunmuyor. |
     | `finerThanPolicy` | Tescilli yüzölçüm politikanın basamağından ince. Kayıt yuvarlanmaz. |
     | `outsideTooSmall` | Girmeyen kısım hatayı taşıyamıyor (tescilli − esas < 0). |
     | `sliver` | Toleranstan dar parça var: sınır parselin kenarının yakınından geçiyor. |
     | `outside` | Parselin sınırın içinde kısmı yok. |
   - **Toplamlar:**
     - sınırın alanı ve parsellerin içerideki kısımları;
     - hiçbir parselin kaplamadığı alan (yollar, dere…, parseller çakışmıyorsa);
     - esasların kesin toplamı;
     - sayılar: tamamı, kısmen, esaslı, incelenecek.
   - Aynı Ada/Parsel iki parselde ise `duplicate` olur. Kayıtlar parseli bu adla bulur.
   - **Retler:**
     - `invalid_input`: `scale` 6'dan büyük, bilinmeyen `rounding`, `tolerance` sonlu değil ya da 1e-6'dan küçük;
     - `invalid_uid`, `entity_not_found`, `not_an_area` (`boundary`);
     - `invalid_uid`, `entity_not_found`, `not_a_parcel` (`uids[i]`).
2. **Hesaplanan alan tescilli alanın üstüne yazılmaz** (CLAUDE.md §7, §23). Belirsiz durumda esas verilmez: sonuç taslaktır,
   resmî karar değildir.
3. **Masaüstü** (Düzenleme dağıtım taslağı):
   - Pencere açılırken seçili alan düzenleme sınırıdır.
   - Kapsam, kural paketinin basamağı ve yuvarlamasıyla hesaplanır; ikisi ya da şerit toleransı değişince yeniden
     hesaplanır.
   - Özet satırı: sınırın alanı, parsel sayıları, esas alanların toplamı, incelenecekler, kaplanmayan alan.
   - **Kapsam** görünümü kitin veri tablosudur. Sütunlar:
     - Parsel, Giriş;
     - Tescilli, Hesaplanan, Fark / sınır;
     - Giren, Girmeyen, Esas alan;
     - Durum (örneğin “Giren kısım esas; dışarıda 265.07 kalır”).
   - **Esas alanları kayıtlara yaz:** esası olan her parselin kayıtlarına esas alanı koyar, sonra dağıtım yeniden
     hesaplanır. Ayrıca söyler:
     - değişenleri;
     - dosyadaki alanla kalanları (esası yok, “inceleyin”);
     - kapsamda olup kaydı olmayan parselleri;
     - kapsamda olmayan kayıt parsellerini.
   - Sınır seçili değilse özet bunu ve ne yapılacağını söyler.
   - Şerit yeri Düzenleme dağıtım taslağınınkidir: Harita'da Plan › Ada ve parsel, CBS'de Veri › Tablo.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/readjustment_scope_cases.py`, 31 durum; KentOS kodu yok.
  - **Başvurunun yöntemi:**
    - koordinatlar saklandığı gibi kesirle alınır;
    - giren kısım Sutherland–Hodgman ile kırpılır; girmeyen kısım kenar kenar ayrılır, ortak kenarı olan hücreler
      birleştirilir;
    - yanılma sınırı ADR 1400'ün çizelgesiyle, yuvarlama elle hesaplanır.
  - **Durumlar:**
    - karışık ada: her durum bir arada; dört esasın toplamı 1776.83 m², altı parsel incelenecek;
    - her durum tek başına;
    - 3 mm toleransla sayılan 4 mm'lik parça;
    - iki parçası dışarıda kalan parsel;
    - sıfırdan uzağa yuvarlama (1000/3 → 333.34);
    - sıfır basamak, başka öznitelik, seçim sırası;
    - aynı Ada/Parsel, parsel katmanındaki sınır, CBS, boş katman;
    - plan, doğrulama ve yedi ret.
  - **Kasıtlı bozulan beklenen** (333.33 → 333.34) durumu düşürdü.
  - Rust ve Python geçer.
- **Python** (`python/tests/test_readjustment_scope.py`), Python'un `decimal`'ıyla:
  - kısmen girende tescilli = esas + girmeyende kalan, kesin;
  - her parselde giren + girmeyen = hesaplanan;
  - esasların toplamı ve sayılar;
  - yuvarlama yalnız hesaplanan esasları değiştirir.
- **MCP** (`crates/native/mcp/tests/readjustment_scope.rs`): .kcad olarak açılan çizimde karışık adanın durumları,
  esasları ve sayıları; geçersiz sınırın reddi kodu, yeri ve sözüyle.
- **Masaüstü** (`readjustment_window_tests`):
  - seçili sınır ortak durumun durumlarını ve toplamını verir;
  - Esas alanları kayıtlara yaz 101/3'ü 333.33 yapar, 101/4'ün esası olmadığını, K9'un kapsamda olmadığını söyler;
  - dağıtım yeni alanla korunur;
  - basamak 0'da 101/1 politikadan ince olur; tolerans 3 mm'de 101/9 kısmen girer;
  - sınır seçili değilse kapsam yoktur;
  - çizim değişmez.

  Resimler `.run/gui-evidence/duzenleme-taslagi-1460/` altındadır (`3-kapsam-*`, `4-esas-alanla-parseller-*`).

## Otomasyon

```python
from kentos import cad
k = cad.readjustment.scope(doc, boundary=sinir, scale=2, rounding="half_even")
for p in k.parcels:
    print(p.parcel, p.kind, p.recorded_text, p.calculated, p.inside, p.basis, p.status)
```

MCP: `{"tool": "cad.readjustment.scope", "arguments": {"drawing": "…", "boundary": "…", "scale": 2, "rounding": "half_even"}}`.

## Sonra

- 2022 Yönetmeliği m. 8(5) (geçici koordinatlı parsellerin iç ve dış yanılma sınırı) ve m. 8(4)'ün istisnaları.
- Ayırma çapının çizimi: parçaların sınırları ve koordinatları, çıktısı.
- Birden çok dış parça arasında hatanın paylaşımı. KentOS bunu toplam olarak verir, paylaştırmaz.
- Tescil harici alanların (kadastro yolları, dere) düzenlemedeki yeri.
- ADR 1460'ın kalanı: değer esaslı dağıtım, kamulaştırma ve toplulaştırma, itiraz ve sürüm, yetkiler, ikinci kabul.
