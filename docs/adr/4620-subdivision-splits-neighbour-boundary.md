# 4620 — İfrazda komşunun sınırının yeni köşede bölünmesi

Tarih: 2026-11-10. plan/map-cad.md MAP-08, dokuzuncu tur. Durum: taslak. ADR 4220'nin “Sonra”sındaki “İfrazda komşunun
sınırının bölünmesi”.

İlgili:
- [ADR 4220](4220-parcel-register-documents-and-boundaries.md): parsel kayıtları, kalıcı sınır kimliği;
- [ADR 2000](2000-parcel-boundary-graph.md): sınır grafiği, `missingVertex` (1 µm);
- [ADR 2600](2600-shared-edge-junctions-and-inner-vertices.md): kenarın içindeki köşe (1 µm).

## Dayanak

- **ArcGIS Pro 2.8**, [Set parcels current or historic](https://pro.arcgis.com/en/pro-app/2.8/help/data/parcel-editing/setcurrentorhistoric.htm)
  (10 Kasım 2026'da okundu): tarihî yapılan parsel güncel bir komşuyla sınır çizgisini paylaşıyorsa çizgi bölünür, güncel
  parsel çakışan güncel çizgileri alır; asıl çizgi tarihî olarak saklanır.
- **ArcGIS Pro**, [Split parcels](https://doc.esri.com/en/arcgis-pro/latest/help/data/parcel-editing/split-parcels.html)
  (10 Kasım 2026'da okundu):
  - bölünen parseller tarihî olur, Retired By Record yeni parselleri yapan kayıttır;
  - yeni parsellere yeni çizgiler yapılır;
  - komşuların kullanmadığı asıl çizgiler tarihî olur;
  - bölmeyle değişmeyen sınırlar onları yapan kayıtta kalır.
- **KentOS bugün** (ADR 4220): 101/7 ikiye bölününce yeni köşe (10, 10), 101/9'un da kullandığı (0, 10)–(20, 10)
  sınırının içine düşer. Yeni parsellerin kuzey kenarları iki yeni sınır olur; 101/9 eski sınırda kalır. İki yanın
  sınır kimliği ayrılır: 101/9'un yeni parsellerle komşuluğu kaybolur, aynı yerde iki güncel sınır kalır.

## Karar

1. **`cad.parcels.record`** (sürüm aynı). Belgenin yaptığı parsellerden sonra, önceki parselleri kapatmadan önce şu
   koşulları sağlayan sınır bölünür:
   - belgeden önce güncel ve düz bir sınırdır;
   - belgeden önce güncel olan, belgenin kapatmadığı bir parsel (komşu) onu kullanır;
   - içinde belgenin yeni parsellerinin köşeleri vardır: doğrusuna 1 µm içinde, iki ucundan 1 µm'den uzak (ADR 2000 ve
     2600'ün kuralı);
   - o köşelerle ayrılan parçalarının hepsi (küçük köşesinden başlayarak) güncel düz sınırlardır: belgenin yaptıkları
     ya da eşleştikleri.
2. **Bölme:**
   - her komşunun halkasında eski sınırın yerine parçalar sırayla girer; her parça kendi yönüyle işaretlenir;
   - halka eski sınırı ters yürüyorsa parçalar ters sırayla, eksi işaretle girer;
   - komşu kapanmaz: kimliği, belgesi ve belgedeki alanı kalır, halkası köşe kazanır;
   - eski sınırı artık hiçbir güncel parsel kullanmaz; ADR 4220'nin kuralıyla belgeyle kapanır.
   - Parçaları belgenin sınırı olmayan kenar bölünmez: yeni alan komşunun sınırının yalnız bir kısmına değiyorsa
     T birleşimi ADR 4220'deki gibi ayrı sınırdır.
3. **Cevap:** `boundariesSplit` (bölünen sınır sayısı) ve `neighbours` (halkası parça alan parseller); ikisi de
   yoksa yazılmaz. Plan aynısını verir.
4. **Alanlara dokunulmaz** (ADR 4220). Komşunun çizimdeki alanında yeni köşe yoksa `cad.parcels.records` onu
   “çizimde değişti”, parçaları “çizilmiyor” der; köşe çizilince kayıttaki gibidir.
5. **Masaüstü** (Parsel kayıtları): planın sözü “…, 1 komşu sınırı yeni köşede bölünür (101/9 parçaları alır).”
6. **Kapsam işareti:** ADR 4220 gibi **Harita + CBS**.

## Sınırlar

- Yaylı sınır bölünmez: yayın parçalarının kabarıklığı ayrıca hesap ister.
- Komşunun çizimdeki alanına köşe eklenmez; kayıt çizimden ayrıdır.
- Bölme, belgenin yaptığı parsellerin köşelerinedir; kapatılan parselin köşeleri sayılmaz.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/parcel_register_cases.py`; KentOS kodu yok):
  - `cad.parcels.record`, 38 durum (2 yeni):
    - komşu sınırı ters yürüyen ifraz (101/9 kapanır, 101/7 parçaları ters sırayla, eksi işaretle alır);
    - köşeleri komşunun sınırının içinde ama parçaları belgenin sınırı olmayan alan (bölünmez);
    - eski ifraz durumu bölmeyle değişti: 101/9'un halkası parçaları alır, kapanan sınır 2. Bu ADR'nin kuralıdır.
      O durumun kaydını kurulum olarak kullanan iki durum buna göre yeni kayıtla değişti.
  - `cad.parcels.records`, 23 durum (1 yeni): ifrazdan sonra 101/9'un alanına yeni köşe eklenince kayıttaki gibidir,
    komşuları 101/13 ve 101/14. Eski ifraz durumunda köşesiz alan çizimde değişti (adı söyler).

  Rust ve Python geçer. Bozma denemesi: ters yürünen sınırda parçalar çevrilmeyince koşucu yakalar.
- **Python** (`python/tests/test_subdivision_splits_neighbour.py`), testin kendi halka yürüyüşüyle: 101/9'un köşeleri
  (0, 10), (10, 10), (20, 10), …; eski sınırı ifraz kapatır; komşuları 101/13 ve 101/14; köşesiz alanı çizimde
  değişti, köşe eklenince kayıttaki gibi; tek geri alma halkayı geri getirir.
- **MCP** (`crates/native/mcp/tests/subdivision_splits_neighbour.rs`): ifraz bir sınırı böler, 101/9 parçaları alır;
  101/9'un komşuları 4 ve 5.
- **Masaüstü** (`parcel_register_window_tests`): ifraz planının sözü bölmeyi söyler; Belgeyi kaydet ortak durumun
  kaydını yazar. Parsel kayıtları testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/komsu-sinir-bolunmesi-4620/`.

## Sonra

- Yaylı sınırın bölünmesi.
- Kenarın belgedeki ölçüleri (COGO) ve ölçülerden hesaplanan alan; belge taslağı ile onaylı kayıt (MAP-08'in kalanı).
- Web (TODOS §0.2).
