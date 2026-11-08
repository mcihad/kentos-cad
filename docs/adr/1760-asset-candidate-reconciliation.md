# 1760 — Harita varlığı adaylarının iki kaynaktan uzlaştırılması (`map.features.reconcile`)

Tarih: 2026-11-07. plan/map-cad.md MAP-04, ikinci dilim. Durum: taslak. İlgili:
- [ADR 1360](1360-field-codes-and-kodla-ciz.md): saha kodları, Harita varlıkları profili, Kodla çiz. Bu ADR oradaki
  “Sonra: iki kaynaktan aday eşleme ve inceleme” maddesini açar.

## Dayanak (7 Kasım 2026'da okundu)

- **Esri Detect Feature Changes**
  ([ArcGIS Pro 3.5](https://pro.arcgis.com/en/pro-app/3.5/tool-reference/data-management/detect-feature-changes.htm)):
  - güncelleme ve taban nesnelerini arama mesafesiyle eşler;
  - eşleşmeyen güncellemeyi yeni (N), eşleşmeyen tabanı silinecek olabilir (D), eşleşip değişmeyeni NC diye işaretler;
  - belge sonucun hata içerebileceğini söyler ve özellikle N'lerin gözden geçirilmesini önerir.

  Çizgiler içindir; noktada aynı mantık uzaklıkla kurulur.
- **Esri Detect Objects Using Deep Learning**
  ([ArcGIS Pro](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/image-analyst/detect-objects-using-deep-learning.html)):
  - her tespit bir güven puanı ve sınıf taşır;
  - Non Maximum Suppression, örtüşen yinelenen tespitlerden güveni düşük olanı **siler**.
- **TBC** (ADR 1360): nokta bulutundan çıkarılan öğe kod ve öznitelik taşıyan nesneye dönüşür. Çıkarımın kendisi bu
  dilimde değildir: KentOS çıkarılmış adayları uzlaştırır.
- **Netcad:** iki kaynaktan aday uzlaştırmanın herkese açık belgesi bulunamadı.
- **Plan (MAP-04):** belirsiz aday sessizce kesin varlığa dönüşmesin. Model ya da katalog değişince kabul edilmiş insan
  düzeltmeleri korunsun. İşlem bir kez geri alınabilsin. Kabul satırı: aynı sokağın bulutundan ve ortofotosundan gelen
  direk ve bordür adaylarını uzlaştır; yinelenen varlığı önle, eksik zorunlu alanı bul, yanlış sınıfı düzelt.

## Karar

1. **Aday ve sınıfı:**
   - Aday, bir kaynağın katmanındaki nokta ya da açık çoklu çizgidir (en az iki köşe).
   - Sınıfı şöyle bulunur: önce kararın düzeltmesi; yoksa sınıf alanının (`field`, öntanımlı `Kod`) yazısı sınıf
     kimliği olarak; o da yoksa ilk sözcüğün baştaki büyük harflerini kodları arasında taşıyan sınıf (ADR 1360'ın
     kod kuralı).
   - Güven, verilen sayısal alandır; büyük olan iyidir, güvensiz aday her sayının altındadır.
   - Aday durumları:
     - `reviewed`: `Uzlaştırma` özniteliği olan aday daha önce uzlaştırılmıştır, okunmaz;
     - `unknown`: sınıfsız;
     - `wrongKind`: nokta sınıfı çizgide ya da tersi. Alan sınıfları bu dilimde uzlaştırılmaz.
2. **Gruplar:**
   - Aynı türden adaylar `radius` (m) içindeyse zincirle bir gruptur. Noktada düzlem uzaklığı kullanılır. Çoklu
     çizgide köşelerden öbürünün doğru parçalarına iki yönlü ayrık Hausdorff uzaklığının büyüğü kullanılır; yaylar
     bu dilimde kirişleriyle ölçülür.
   - Kaynağın temsilcisi, gruptaki güveni en yüksek adayıdır (eşitlikte ilki). Aynı kaynağın öbür adayları
     `duplicate` olur: Esri'nin NMS'i gibi güvenlisi kalır, ama KentOS yineleneni **silmez**, işaretler.
3. **Grubun durumu:**
   - `existing`: tek sınıfının katmanında (aday olmayan) bir varlık bir temsilcinin yarıçapında. Yeni varlık
     yazılmaz, adaylar o varlığa bağlanır. Böylece yinelenen varlık önlenir, insanın kabul ettiği sınıf ezilmez.
   - `conflict`: sınıflar farklı.
   - `agreed`: iki ya da daha çok kaynak bir sınıfta.
   - `single`: tek kaynak.
4. **Yeri ve alanları:**
   - Noktanın yeri temsilcilerin ortalamasıdır (hepsinde kot varsa kotun da). Yayılım, ortalamaya en uzak
     temsilcidir.
   - Çizginin geometrisi güveni en yüksek temsilcininkidir (eşitlikte önceki kaynak). Yayılım, temsilciler arasındaki
     en büyük Hausdorff'tur.
   - Alanlar temsilcilerden güven, sonra kaynak sırasıyla alınır; kararların alanları üstte. Sınıf alanı, güven
     alanı, `Uzlaştırma` ve köken öznitelikleri kopyalanmaz.
   - Profilin zorunlu alanlarından verilmeyenler `missing`'dir.
5. **Yazma:**
   - Yazılan gruplar: `agreed`, ya da kabul kararlı `single`; ikisinde de eksik alan olmamalı. Belirsiz aday
     kendiliğinden kesin varlık olmaz.
   - Varlık sınıfın katmanına yazılır. Sınıf alanı sınıfın ilk kodudur. Köken öznitelikleri:
     - `Kaynaklar` (kaynakların adları), `Adaylar` (kimlikleri), `Yayılım` (m, üç basamak);
     - kararla yazıldıysa `İnceleme` (kabul, sınıf …, alan …) ve `Gözden geçiren`.
   - Yazılan ve var olan grubun adaylarına `Uzlaştırma` = varlığın kimliği, reddedilen adaya `Reddedildi: neden`
     yazılır.
   - Tek geri alma adımıdır (“Adayları uzlaştır”). Yazılacak ya da işaretlenecek bir şey yoksa hiçbir şey yazılmaz.
   - `.kcad` değişmez: köken öznitelikle taşınır.
6. **Kararlar** (`decisions`, `reviewer` ile):
   - `class`: sınıf düzeltme; çelişki çözülür, grup yeniden kurulur;
   - `accept`: tek kaynaklı aday kabul;
   - `reject`: nedenle ret;
   - `fields`: eksik alanları verme.
7. **Retler:** `invalid_revision`, `revision_conflict`, `unknown_profile`, `invalid_profile`, `invalid_input`,
   `layer_not_found`, `invalid_uid`, `entity_not_found`, `class_layer_missing` (yazılacak sınıfın katmanı yok: önce
   profili uygulayın), `layer_locked` (sınıfın ya da işaretlenecek adayların katmanı).
8. **Masaüstü: Adayları uzlaştır** (`map.featuresReconcile`; yalnız KentOS Harita):
   - **Yer:** Ölçme › Noktalar, Kodla çiz'in yanı. Simgesi `featuresReconcile`: iki kaynağın kesişimi ve kabul işareti
     (seçenek sayfası `.run/gui-evidence/aday-uzlastirma-1760/secenekler.png`).
   - **Form:** iki kaynak (katman adlarından LiDAR/bulut ve ortofoto tahmin edilir), sınıf ve güven alanı, yarıçap,
     gözden geçiren.
   - **Özet:** durumlara göre sayılar, yinelenen, okunmayan ve daha önce uzlaştırılan adaylar, eksik alanlı gruplar.
   - **Tablo:** satır başına Kabul (tek), sınıf düğmeleri (çelişki, profilin başlıklarıyla), Reddet. Kararlar
     pencerede birikir, Uzlaştır hepsini yazar.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/map_features_reconcile_cases.py`, 25 durum; KentOS kodu yok. Uzaklıklar
  `math.hypot` ve köşe-doğru parçası Hausdorff'uyla hesaplanır.
  - Plan: uyuşan iki direk (biri yinelenenli); direk/ağaç çelişkisi; tek rögar; kabul edilmiş direğin yanındaki aday;
    bilinmeyen kod; KapiNo'suz kapı; bordür çizgileri; daha önce uzlaştırılan aday; çizgi olarak çizilmiş direk.
  - Yazma ve tek adımda geri alma.
  - Kararlar: kabul, sınıf düzeltme, KapiNo verme, ret.
  - Güvensiz temsilci; dar yarıçap; tek kaynak.
  - Yeniden çalıştırma: uzlaştırılmış adaylar okunmaz, hiçbir şey yazılmaz.
  - Doğrulama ve 15 ret.

  Rust ve Python geçer.
- **Masaüstü** (`features_reconcile_window_tests`, 3 test):
  - plan ortak durumla aynıdır; Kabul ve sınıf düzeltmeyle yazılan artar; Uzlaştır tek adımdır ve ikinci çalıştırma
    yazmaz;
  - Reddet;
  - Harita sunar, CBS ve Mimari sunmaz.

  Resimler `.run/gui-evidence/aday-uzlastirma-1760/` altındadır. Ortak masaüstü denetimleri 142 testle geçer.
- **MCP:** `kentos-mcp` testleri geçer.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("sokak.kcad")
sources = [{"layer": "aday-lidar", "name": "LiDAR"}, {"layer": "aday-orto", "name": "Ortofoto"}]
plan = cad.map.features.reconcile.plan(doc, name="harita-varliklari", sources=sources, confidence="Güven", radius=0.5)
tek = next(g.candidates[0] for g in plan.groups if g.status == "single")
cad.map.features.reconcile(doc, name="harita-varliklari", sources=sources, confidence="Güven", radius=0.5,
                           reviewer="Ayşe Demir", decisions=[{"candidate": tek, "accept": True}])
```

MCP: `{"tool": "map.features.reconcile", ...}` yazar; planı salt okunurdur.

## Sonra

- **Pencere:** eksik zorunlu alanın pencerede girilmesi.
- **Kapsam:** alan sınıflarının (bina izi) örtüşmeyle uzlaştırılması: yapıldı,
  [ADR 2140](2140-area-candidates-by-overlap.md); yaylı adayların yay uzaklığı.
- **Ölçek:** büyük aday kümesinde ızgara dizini (bugün tüm çiftler karşılaştırılır).
- **İnceleme kaydı:** kararların ayrı bir inceleme kaydı olarak (öznitelik değil) tutulması ve katalog değişikliğinin
  geçiş raporu.
- **Çıkarım:** AI/çıkarım modelinin indirilip çalıştırılması (MAP-04'ün kalanı).
