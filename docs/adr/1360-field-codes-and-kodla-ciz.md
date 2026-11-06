# 1360 — Saha kodları ve Kodla çiz (`map.features.code`)

Tarih: 2026-11-06. plan/map-cad.md MAP-04 (birinci dilim: sürümlü varlık kataloğunun saha koduyla bağı). Durum: taslak.
İlgili: [ADR 0541](0541-data-profiles.md) ve [0558](0558-institution-profile-import-migration-mapping.md) (veri profilleri),
[ADR 0622](0622-interface-arrangement.md) §5 (katman türü), [ADR 1340](1340-gnss-campaign-archive.md) (GNSS arşivi).

## Dayanak

- **Trimble Business Center:** Feature Definition Manager'da kurulan özellik kodu kütüphanesi (.FXL) bir kodun nokta, çizgi
  ya da kırık hat olarak ölçüldüğünü, özniteliklerini ve denetim kodlarını tanımlar; noktadaki koddan zengin nokta
  sembolü ve çizgiler kurulur; öznitelikler ofiste tanımlanır, sahada değiştirilemez
  ([Trimble Business Center feature libraries](https://help.fieldsystems.trimble.com/trimble-access/latest/en/feature-libraries-fdm.htm),
  [Measuring with feature codes](https://help.fieldsystems.trimble.com/siteworks/en/measurement-workflows-feature-codes.htm);
  6 Kasım 2026'da okundu).
- **Civil 3D:** noktanın ham açıklamasının ilk öğesi açıklama anahtarının koduyla karşılaştırılır, sonrası
  parametredir; anahtar katmanı, nokta stilini ve etiket stilini verir
  ([Description Key Format Reference](https://help.autodesk.com/cloudhelp/2020/ENG/Civil3D-UserGuide/files/GUID-3E7422F2-F6D9-4AAD-A997-04678401CE41.htm)).
- **OpenCities Map (XFM):** özelliğin geometri türü şemada tanımlıdır ve oturum boyunca uygulanır; yerleştirme araçları
  özelliğin türünden kurulur ([XFM Features](https://DOCS.BENTLEY.COM/LiveContent/web/OpenCities%20Map%20Help-v15/en/GUID-3C3DFB1B-D342-1DA2-F5C0-1F1985018D61.html)).
- **Netcad ve BÖHHBÜY:** Netcad'in kod tablosunu anlatan resmî belge ve BÖHHBÜY'nün obje kataloğunun metni bu oturumda
  bulunamadı (arama yalnız eğitim özetleri verdi). Yerleşik profilin kodları bu yüzden örnektir; mevzuat kodu iddiası yoktur.
- **KentOS bugün:** veri profili (ADR 0541) sınıfı katman yoluyla, geometri türüyle, alanlarıyla, değer listeleriyle ve
  kimliğiyle tanımlar; `cad.profiles.apply` katmanları kurar, `.check` uymayanı bulur. Plan “mevcut domain/form/profil
  altyapısını kullan” der; ayrı bir katalog kurulmadı. Eksik olan sahadaki kodla bağdı: kodlu noktalar elle katmana
  taşınıyor, bordür ve bina izi elle birleştiriliyordu.

## Karar

1. **Sınıfın saha kodları (`ProfileClass.codes`).** Bir ya da birkaç büyük harfli kod (A–Z, 1–12, rakamsız); profilde
   bir kod tek sınıfın; geometrisiz kayıt sınıfında kod olmaz. Profil biçimi sürüm 1 kalır (alan isteğe bağlı).
2. **Kodun okunuşu.** Noktanın kod alanı (`Kod`, ya da verilen `field`) kırpılır, ilk sözcüğü büyük harfe çevrilir: baştaki
   harfler kod, ardındaki rakamlar dizinin numarası (`BRD1`, `BRD2`). Sonraki sözcükler denetim kodu: **B** yeni dizi
   başlatır (açık olanı bitirip), **K** kapatır ve bitirir, **S** bitirir; başkası `control_unknown`. Harf–rakam biçiminde
   olmayan ya da profilde bulunmayan kod `unknown_code` (noktaları olduğu gibi kalır). Türkçe denetim harfleri (Başla,
   Kapat, Son) TBC'nin Start/Close/End denetim kodlarının karşılığıdır.
3. **Yerleşik profil `harita-varliklari`** (Harita varlıkları, saha kodlu): kontrol noktası (KN, PN), rögar (ROG), vana
   (VAN), direk (DRK), ağaç (AGC), kapı (KAP), bordür (BRD), yol kenarı (YK), duvar (DVR), bina izi (BNA), yeşil alan
   (YSL); şebeke, direk türü ve kontrol noktası türü değer listeleri. Kurum kendi profilini ve kodlarını (ADR 0558) getirir.
4. **`map.features.code` v1 — Kodla çiz.** Noktalar verilen sırayla (yoksa görünen ve kilitsiz katmanlardaki bütün
   noktalar çizimin sırasıyla). Nokta sınıfının noktası sınıfın katmanına taşınır. Çizgi ve alan sınıfında noktalar
   (sınıf, numara) dizilerine katılır; biten çizgi dizisi ≥2 noktaysa (K ile kapalıysa ≥3) çoklu çizgi, K ile kapatılmış
   ≥3 noktalık alan dizisi alan olur; bütün noktaların kotu varsa köşeler kotlu; yeni nesnenin kod alanı ilk noktasının
   kodudur; noktalar yerinde kalır. Kısa dizi `string_too_short`, kapatılmamış alan `area_not_closed`. Sınıfın katmanı
   yoksa `class_layer_missing` (önce profili uygula), kilitli katman `layer_locked`. Yazacak bir şey yoksa hiçbir şey
   yazılmaz; yoksa tek geri alma adımı “Kodla çiz”.
5. **Masaüstü penceresi Kodla çiz…** (`map.featuresCode`, ikon `featuresCode`: koddan çizgiye birleşen noktalar):
   profil seçimi (KentOS'un ve projenin profilleri), kod alanı, planın sınıf sınıf önizlemesi ve uyarıları; sınıfın
   katmanı yoksa **Profili uygula** (`cad.profiles.apply`); Uygula. Seçili noktalar varsa onlar seçim sırasıyla.
6. **CAD / CBS işareti: ikisi de.** Şeritte Harita'nın Ölçme › Noktalar paneli (`works: ["map"]`) ve GIS'in Ölçme ›
   Noktalar paneli; Mimari ve Yol sunmaz. Profil ve kodları ortaktır.

## Kanıt

- `map.features.code` 26 ortak durum, bağımsız `scripts/fixtures/map_features_cases.py` (kodun okunuşu, dizi birleştirme
  ve iletiler ADR'den elle): bütün kodlu noktalar ve geri alma; plan, doğrulama; verilen sıra; B, S, K; kotlu/kotsuz
  köşeler; kapatılmamış ve kısa alan; bilinmeyen kod ve denetim; rakamla başlayan kod; zaten katmanındaki nokta; başka
  alan; retler (kodlu nokta yok, katman yok, kilitli sınıf katmanı, kilitli nokta, profil yok, boş alan, nokta değil,
  çizimde yok, UUID değil, sürüm). Rust ve Python aynı durumları geçer. `cad.profiles.*`'ın liste cevapları yeni yerleşik
  profille güncellendi (`profile_files_cases.py`).
- Masaüstü `features_window_tests.rs`: önizlemenin sınıfları ve bilinmeyen kod uyarısı, Uygula (iki çizgi, bir alan) ve
  tek adımda geri alma; katmanı olmayan sınıfta Profili uygula; Harita ve GIS sunar, Mimari ve Yol sunmaz. Resimler
  `.run/gui-evidence/kodla-ciz-1360/`.

## Yol üstünde düzeltilen

`cad.profiles.apply`, sınıfın katmanı çizimde varken değer listesine bağlı alanı eklemeyi reddediyordu (“… değer listesi
projede yok”, `layer_refused`): listeler katmanlardan sonra yazılıyordu. Artık önce yazılır, ret olursa projenin kendi
listeleri geri konur. Ortak durum: “Katmanları olan, değer listesi olmayan proje” (eski kodda düşer). Python örneği
bunu ortaya çıkardı.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("/is/harita/alim.kcad")
cad.profiles.apply(doc, name="harita-varliklari")
out = cad.map.features.code(doc, name="harita-varliklari")
print([(c.title, c.moved, c.lines, c.areas) for c in out.classes])
```

MCP: `{"tool": "map.features.code", "arguments": {"name": "harita-varliklari"}}`.

## Sonra

- Sınıfın sembolü (blok) ve nokta yerine blok yerleştirme; yay ve eğri denetim kodları (TBC'nin smooth curve, arc).
- Aynı varlığın iki kaynakta (bulut ve ortofoto) eşlenmesi, aday çıkarımı ve inceleme kaydı (MAP-04'ün kalanı).
- İçe aktarmada (GNSS, nokta dosyası) kodun doğrudan bu komuta verilmesi; kurum kod listesinin BÖHHBÜY eşlemesi.
