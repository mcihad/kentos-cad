# 1092 — Var olan çıktı dosyası, ona bağlı nesnelerle çalıştırmadan önce söylenir

Tarih: 2026-11-20. gis-usage-report.md GIS-035. Durum: kabul edildi.

İlgili: [ADR 1078](1078-raster-result-files-of-their-own-with-band-units.md) (GIS-012, otomatik çıktı adı), ADR 3100 (Buluta uygula),
ADR 3900 (Son dönüşümü geri al).

## Sorun

Aynı taramanın ikinci dönüşümü (Tersini uygula) varsayılan çıktı yoluyla ilk sonucun `Tarama-2-donusmus.laz`
dosyasının üzerine yazdı; yeniden açılan projede iki kayıt aynı dosyaya bakıyordu, ilk sonuç sessizce kayboldu.

## Durum

Otomatik ad (çıktı alanı boş) GIS-012'de düzeldi: nokta bulutu yazıcıları da dosya yolunu `Files::output_path` ile
alır, o da `kentos_processing::files::free_output`'u çağırır. Diskte olan ya da süren bir işin ayırdığı ad atlanır:
ileri dönüşüm `Tarama-2-donusmus.laz`, ters dönüşüm `Tarama-2-donusmus-2.laz` olur. Kalan, kullanıcının açıkça var
olan bir dosyayı seçmesiydi.

## Karar

1. İşlem penceresindeki her “Çıktı dosyası” (`ParamKind::SaveFile`) seçilen yol diskte bir dosyaysa, açık çizimde o
   dosyaya bağlı nesneler (nokta bulutunun üye dosyası, rasterın dosyası; göreli bağ çizimin klasörüne göre, yollar
   diskin adıyla karşılaştırılır) aranır. Pencere yenilendikçe (`refresh_processing`) hesaplanır, çizimde gezinti
   yalnız seçilmiş ve var olan dosya için yapılır.
2. Yolun altında uyarı rengiyle söylenir: bağlı nesne yoksa “Bu dosya var: çalıştırınca üzerine yazılır.”; varsa
   sayısı ve ilk üç adı (etiket, yoksa katman adı) ile “… çalıştırınca üzerine yazılır, onlar da değişir. Ayrı bir
   sonuç için başka bir ad seçin ya da alanı boşaltın.”
3. Çalıştırma engellenmez: kullanıcı bilerek aynı dosyayı yenileyebilir; karar görünür kılınır.

## Sınır

- Kayıt, yeniden açma ve işlem geçmişinden yeniden çalıştırmada iki dosyanın özetinin korunması otomatik adla
  sağlanır (her çalıştırma kendi dosyası); geçmişten yeniden çalıştırmada değerlerde açık yol varsa yine bu uyarı
  çıkar.
- Diskte olmayan ama çizimde başvurulan bir dosyanın adı otomatik adda ayrıca dışlanmaz (dosya yoksa üzerine yazma
  da yoktur; kayıp bağ ayrı söylenir).

## Kanıt

`apps/desktop/src/gis/processing/overwrite_tests.rs`: iki otomatik çalıştırma iki ayrı dosya; seçilen var olan dosya
taşınmış taramayı söyler, yeni ad bir şey söylemez; satır ekranda (resim `.run/gui-evidence/cbs-kullanim/`).
