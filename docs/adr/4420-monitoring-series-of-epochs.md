# 4420 — İzlemenin dönem dizisi: ikiden çok dönem

Tarih: 2026-11-10. plan/map-cad.md MAP-19, sekizinci tur. Durum: taslak. ADR 4020'nin “Sonra”sındaki “zaman serisi”
(MAP-19'un açığı: “zaman serisi (ikiden çok dönem)”).

İlgili:
- [ADR 3620](3620-asbuilt-job-kinds-and-monitoring.md): iş türleri, iki dönemli izleme, kontrol noktaları;
- [ADR 0633](0633-dataset-metadata-and-lineage.md): katmanın veri kümesi bilgisi (güncelleme günü).

## Dayanak

- **Trimble Business Center, Monitor Point Movement** (ADR 3620'nin okuması, 9 Kasım 2026):
  - ilk dönem (reference epoch) başlangıç koordinatlarını verir; sonraki her dönem onunla ve önceki dönemlerle
    karşılaştırılır;
  - noktalar adla eşlenir;
  - bir noktanın hareketi ancak en az iki dönemde ölçülmüşse görünür.
- **KentOS bugün** (ADR 3620): izleme bu dönemi tek bir önceki dönemle adla karşılaştırır; ikiden çok dönem birlikte
  okunamıyordu.

## Karar

1. **`cad.asbuilt.check`'in izleme işine `epochs`** (komut sürümü değişmez): önceki dönemler, eskiden yeniye. Her biri
   bir nokta katmanı ve günü (`YYYY-MM-DD`, isteğe bağlı). **`date`:** bu dönemin günü. İkisi de yalnız izleme işinin.
2. **Dizi:** önceki dönemler, sonra bu dönem. `design` (karşılaştırılan önceki dönem) ve sınırlar eskisi gibi kalır;
   dizi onların yanında okunur.
3. **İz** (her nokta için, TBC'nin kuralıyla):
   - bu dönemde ölçülen, adı bu dönemde bir kez geçen her nokta adla aranır. Bir dönemde iki kez geçen ad o dönemde
     eşlenmez;
   - başvurusu ölçüldüğü ilk dönemdir (`from`, dizideki sırası);
   - başvurudan sonraki her dönemde, başvurusuna göre hareketi: doğu, kuzey, kot (iki kot da varsa) ve yatay; sınırı
     aşan adım işaretlenir (`outside`);
   - ölçülmediği ya da başvurusundan önceki dönemde adım yoktur;
   - yalnız iki ya da daha çok dönemde ölçülen nokta izlenir (yalnız bu dönemdeki yeni nokta izlenmez).
4. **Hız:** başvurusunun ve bu dönemin günü varsa ortalama yatay ve düşey hız (m/yıl): son hareket bölü aradaki yıllar
   (yıl 365,25 gün). Bir doğrultu ya da eğri uydurulmaz.
5. **Gün:** gerçek bir gün olmalı (YYYY-AA-GG, artık yıl kuralıyla). Gün numarası Howard Hinnant'ın `days_from_civil`
   yöntemiyle bulunur.
6. **Retler** (`invalid_asbuilt_check`): izleme dışında dizi ya da gün (`epochs`, `date`); gün olmayan gün
   (`epochs[k].date`, `date`), “… bir gün değil; günü YYYY-AA-GG olarak yazın (ör. 2026-04-15).”. Çizimde olmayan dönem
   katmanı `layer_not_found` (`epochs[k].layer`).
7. **Masaüstü** (Uygulama kontrolü › İzleme):
   - **Önceki dönemler** satırı: katman adları eskiden yeniye, virgülle;
   - her dönemin günü katmanının **Veri kümesi bilgisi › Güncelleme günü**'nden (bu dönemin de). Gün yoksa hız yok;
   - sonucun altında **Dönem dizisi** tablosu: nokta, her dönemde yatay hareket (sınırı aşan “!”), son kot hareketi,
     yatay ve düşey hız;
   - olmayan katman adı söylenir.
8. **Kapsam işareti:** ADR 3620 gibi **Harita + Yol + CBS**.

## Sınırlar

- Dönemler dönüştürülmez, düzeltilmez (ADR 3620): kontrol noktaları yalnız karşılaştırılan iki dönem için denetlenir.
- Hız iki uç dönemden ortalamadır; dönemlerden doğru uydurma (en küçük kareler) ve ivme yok.
- Uyarı ve alarm diye iki eşik yok (tek sınır).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/asbuilt_series_cases.py`; KentOS kodu yok), 22 durum
  (`fixtures/commands/v1/cad.asbuilt.check.series.json`):
  - `asbuilt_cases.py`'nin çizimi ve üçüncü dönem (M1 ve M2 hareketi sürer, M3 oturur, M5 ikinci dönemde yok, M6
    ikinci dönemde yeni, M7 yeni);
  - izler ve hızlar elle (`datetime` günleri);
  - durumlar: günlerle ve günsüz; sınırsız, yalnız yatay, yalnız düşey sınır; yalnız birinci ya da ikinci dönem dizide;
    birinci dönemin günü yok; aynı dönem iki kez; bu dönem dizide de; birinci dönemle karşılaştırma; kalite sınırı;
    uygula; doğrulama; Yol ve CBS projeleri; altı ret.

  Eski 65 durum değişmedi. Rust ve Python geçer. Bozma denemesi: bir hız %1 değişince koşucu yakalar.
- **Çekirdek** (`asbuilt_series::tests`): gerçek gün, artık gün, aralıklar, biçimsiz günler.
- **Python** (`python/tests/test_asbuilt_series.py`), testin kendi okumasıyla:
  - izlenen noktalar ve başvuruları, her adım çizimin noktalarından;
  - işaretler sınırlardan, hızlar `datetime` ile.
- **MCP** (`crates/native/mcp/tests/asbuilt_series.rs`): iki durumun izleri, günün reddi. MCP'nin ana testleri geçer.
- **Masaüstü** (`asbuilt_series_tests`): günler `cad.datasets.edit` ile katmanlara yazılır; izler ve hızlar durumunki;
  tablo başlıkları; olmayan katman adı söylenir. Uygulama kontrolü testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/donem-dizisi-4420/`.

## Sonra

- Dönemlerden doğru uydurma (hız ve belirsizliği), uyarı ve alarm eşikleri (TBC'nin sarı ve kırmızısı).
- Yetkili inceleme kaydı (MAP-19'un öbür maddesi).
- Hareketin zaman grafiği (nokta başına).
- Web (TODOS §0.2).
