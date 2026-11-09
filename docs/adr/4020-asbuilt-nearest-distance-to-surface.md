# 4020 — Uygulama kontrolünde yüzeye en yakın uzaklık

Tarih: 2026-11-09. plan/map-cad.md MAP-19, yedinci tur. Durum: taslak. ADR 3220'nin “Sonra”sındaki “Yüzeye en yakın
uzaklık (Trimble Access'in Measure to surface'i); bugün düşey fark”, ADR 3620'nin “Yetkili inceleme kaydı ve yüzeye en
yakın uzaklık”ının ikinci yarısı.

İlgili:
- [ADR 1700](1700-asbuilt-check.md): Uygulama kontrolü (`cad.asbuilt.check`);
- [ADR 3220](3220-field-package-surface.md): saha paketinin yüzeyi.

## Dayanak

- **Trimble Access, “To measure to a surface”**
  ([belge](https://help.fieldsystems.trimble.com/trimble-access/latest/en/conventional-measure-to-surface.htm),
  9 Kasım 2026'da okundu):
  - yöntem “ölçülen noktadan seçilen yüzey modeline en yakın uzaklığı” hesaplar ve saklar. Birden çok yüzey seçiliyse en
    yakını kullanılır;
  - uzaklık ve yüzeydeki en yakın noktanın koordinatları noktayla saklanır;
  - işaret ölçen alete göredir: alet ile model arasındaki konum eksi, modelin öbür yanı artı;
  - uzaklık yalnız “Distance to surface limit” içindeyse gösterilir; yüzey bir DTM ya da BIM modeli olabilir.
- **Trimble'ın BIM yardımı** (aynı aramada): yöntem fiziksel yüzeyle tasarım arasındaki dik uzaklığı bulmak için
  kullanılır.
- **KentOS bugün** (ADR 1700, 3220): yüzey farkı yalnız düşeydi: noktanın kotu eksi yüzeyin onun altındaki kotu. Eğimli
  yüzeyde (şev, rampa) düşey fark yüzeyden dik uzaklıktan büyüktür; 1'e 2 şevde √1,25 katıdır.

## Karar

1. **En yakın uzaklık** (`cad.asbuilt.check`; ek alanlar, komut sürümü değişmez): yüzeyle karşılaştırılan her noktanın
   (yüzeyin içinde ve kotu olan):
   - `nearest`: yüzeyin üçgenlerine uzaydaki en kısa uzaklık;
   - `nearestAt`: yüzeydeki en yakın yer (doğu, kuzey, kot).
2. **Hesap** (`kentos_road::volume::closest_on_triangle`, `Ground::nearest`):
   - her üçgene en yakın yer köşe, kenar ve yüz bölgeleri sırayla sınanarak bulunur (Ericson, Real-Time Collision
     Detection §5.1.5). Koordinatlar noktadan alınır, harita koordinatlarında basamak kaybolmaz;
   - aday üçgenler, planda düşey fark kadar yakınındakilerdir. Düşey ayak yüzeyin üstündedir, en yakın uzaklık düşey
     farktan büyük olamaz;
   - paketin yüzeyi de çizimin yüzeyi de aynı yolla ölçülür.
3. **İşaret:** düşey farkınki (yüzeyin üstü artı, altı eksi). Trimble'ınki ölçen alete göredir; KentOS'ta noktanın
   aletinin yeri yoktur. Yüzeyin dışındaki noktaya (`offSurface`) uzaklık verilmez.
4. **Sınır:** `surfaceBy` (`vertical`, öntanımlı; `nearest`) yüzey sınırının (`vertical`) neye uygulanacağını seçer.
   `outsideSurface` seçilen ölçüyle verilir. Düşey fark eskisi gibi her zaman yazılır.
5. **Masaüstü** (Uygulama kontrolü):
   - **Yüzey farkı** satırı (Düşey fark | En yakın uzaklık), açıklamasıyla;
   - tabloda **En yakın** sütunu, CSV'de “Yüzeye en yakın (m)”.
6. **Kapsam işareti:** ADR 1700'deki gibi **Harita + Yol + CBS**.

## Kanıt

- **Çekirdek** (`volume::nearest_tests`):
  - üçgenin yüzü, kenarı ve köşesi bölgelerinde en yakın yer;
  - harita koordinatlarında mikrometreye;
  - 1'e 2 eğimli zeminde uzaklık 1/√1,25, ayak normal boyunca.
- **Bağımsız başvuru** (`scripts/fixtures/asbuilt_cases.py`, KentOS kodu yok):
  - iki yüzey de bir düzlemin dikdörtgenidir. En yakın yer, normal boyunca ayak dikdörtgenin içindeyse odur, değilse
    kenarlarından en yakını;
  - kesirlerle, karekök sonda;
  - yüzeyle karşılaştırılan her durumun her noktasına `nearest` ve `nearestAt` eklendi;
  - üç yeni durum (65): yalnız yüzeyle her noktanın uzaklığı; P3'ün iki ölçüsü arasındaki sınır düşeyde aşılır, en
    yakında aşılmaz;
  - Rust ve Python geçer.
- **Python** (`python/tests/test_asbuilt_nearest.py`), testin kendi geometrisiyle:
  - en yakın yer düzlemin üstündedir, nokta eksi o yer düzlemin normaline paraleldir, boyu uzaklıktır;
  - uzaklık düşey fark bölü √(1 + 0,02² + 0,01²);
  - sınır en yakına uygulanınca P3 içeride.
- **MCP** (`crates/native/mcp/tests/asbuilt_nearest.rs`): durumun her noktasının uzaklığı, yeri ve işaretleri. ADR
  1700, 2420, 2820, 3220 ve 3620'nin MCP testleri değişmeden geçer.
- **Masaüstü** (`asbuilt_window_tests::the_nearest_distance_to_the_surface_is_the_shared_case_s`):
  - uzaklıklar durumunki;
  - P3 Düşey fark'ta dışarıda, En yakın uzaklık'ta değil.

  Pencerenin öbür testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/yuzeye-en-yakin-4020/`.

## Sonra

- Ölçen aletin yerine göre işaret (Trimble'ınki): kaydın alet kurulumunu taşıması gerekir.
- Yüzeyin dışındaki noktanın yüzeyin kenarına uzaklığı.
- Yetkili inceleme kaydı; zaman serisi (MAP-19'un öbür maddeleri; zaman serisi [ADR 4420](4420-monitoring-series-of-epochs.md)'de).
- Web (TODOS §0.2).
