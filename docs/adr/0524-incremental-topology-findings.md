# ADR 0524: Topoloji bulgularının kalıcı kimliği ve artımlı denetim

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-14'ün ilk dilimi (G1: denetle → bul → düzelt → yeniden
  denetle akışı; bulgunun kalıcı kimliği, değişenin ve komşularının yeniden denetlenmesi, tam denetimle aynı sonuç).
- **Bağlam belgesi:** ADR 0202 (topoloji kuralları, Topoloji sekmesi, Düzelt, İstisna), 0148 ve 0160 (topolojik
  temizlik ve düzenleme).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - bağlı katman ve dış sağlayıcı (GeoPackage, PostGIS): denetim kaynağın revizyonuna bağlanır, görünüm dışındaki
    komşular sağlayıcıdan istenir; sayfası eksik kaynak “hata yok” demez (CBS-12, G2);
  - istisnanın nesnelerin geometri özetine bağlanması (bugün kural, nesneler ve yer; yer değişince istisna düşer, aynı
    yerde değişen geometri istisnayı korur); `.kcad` şeması değişmeden yapılmadı;
  - düzeltmenin kaynak etkisinin ve ölçü farkının önizlemesi;
  - Boşluk olmamalı'nın küme içinde artımlı denetimi: bir blok tek kümedir, kümesi bütün olarak yeniden denetlenir.

## Bağlam

Topoloji denetimi (ADR 0202) her seferinde bütün kuralları bütün nesnelerde çalıştırıyordu; bulgular satır sırasıyla
tanınıyordu, denetimden sonra çizim değişince yalnız “sonuçlar eski olabilir” deniyordu. ArcGIS'in Error Inspector'ı gibi
bir akış (hatayı bul, gerekçeyle çöz, yeniden denetle) için bulgunun kendisi tanınmalı ve büyük çizimde yalnız değişenin
etkisi yeniden denetlenmeli; artımlı sonuç tam denetimden farklı olursa yanlış bir “temiz” sonuç çıkar.

## Karar

### 1. Bulgunun kalıcı kimliği

`topology_rules::incremental::finding_key`: kuralın kimliği, nesnelerin kalıcı kimlikleri bulgunun sırasıyla ve yerin
tolerans ızgarasındaki hücresi; 64 bitlik FNV-1a, 16 onaltılık rakam. Aynı bulgu yeniden bulununca aynı anahtarı alır;
nesnesi ya da yeri değişen başka bir bulgudur. Masaüstünde seçili satırlar yeniden denetimden sonra anahtarlarıyla
izlenir.

### 2. Denetim çizimin durumuna bağlıdır

Denetim çizimin oturumunu, kuşağını ve revizyonunu, kuralları, toleransı ve istisnaları tutar. Çizim değişince sekme
“eski” der ve **Değişenleri denetle** düğmesini gösterir. Kurallar, tolerans ya da istisnalar değiştiyse denetim bütün
yapılır.

### 3. Artımlı denetim

`incremental::recheck`; değişen nesnelerin kalıcı kimlikleri (eklenen, şekli ya da katmanı değişen, silinen) ve
eski ve yeni kutuları (`touched`) ile çalışır:

1. **Etkilenenler:** değişenler ve kutusu bir değişikliğin kutusuna tolerans içinde yaklaşan her nesne.
2. **En yakına uzaklık ölçen kurallar** (Sarkan uç, … çizgilerinin ucunda olmalı, … içinde kalmalı): ölçüsü en yakın
   bağlantıya, ne kadar uzaksa uzaklıktır. Eski bulgunun erimi (uzaklığı; uzaklığı yoksa her yer) bir değişikliğin kutusuna
   değiyorsa nesnesi etkilenir. Etkilenen bir ucun en yakın bağlantısı da denetime girer: kutusu en yakın kutunun en uzak
   köşesinden yakın her bağlantı (kutunun içindeki bir kenar onun en uzak köşesinden uzak olamaz).
3. **Boşluk olmamalı:** boşluk bir kümenin birleşiminin deliğidir, içindeki adalar çıkarılarak. Katmanın alanları tolerans
   içinde birleşen kümelere ayrılır (birleşim-bul, R-ağacıyla). Etkilenen alanı olan ya da kutusu bir değişikliğe değen
   küme, kutusu seçilen bir kümeninkine değen her küme (deliğindeki ada ya da çevresi) ile birlikte bütün olarak denetlenir.
4. Kurallar etkilenenler ve komşuları üzerinde çalışır. Etkilenen bir nesneyi anan yeni bulgular, etkilenen nesneyi anan
   eski bulguların yerini alır; öbür eski bulgular nesnelerinin şimdiki yerleriyle kalır. Kuralların baktığı nesne sayıları
   bütün çizimden sayılır.

Düzelt'ten sonraki yeniden denetim de artımlıdır.

## Doğrulama

- `cargo test -p kentos-geometry-core --test all topology_incremental`: 63 parsel (çakışma ve boşlukla), deliğinde bir ada, bir ince alan, yollar
  ve noktalar; on iki kural; üç tohumla 150'şer rastgele düzenleme (taşıma, büyütme, silme, ekleme). Her adımda artımlı
  sonuç tam denetimle aynı: kural, sorun, nesnelerin kimlikleri, yer, ölçü, istisna ve sayılar. Kalıcı anahtarın durumları.
- `cargo test -p kentos-desktop topology::tests`: yol taşınınca bulgular eski, Değişenleri denetle tam denetimle aynı
  satırları verir, seçili satır anahtarıyla izlenir; Düzelt'in yeniden denetimi.
- Süre (`topology_incremental::timing`, 10 000 parsel, biri taşınmış; release, bu makine): Çakışmamalı tam 30 ms ve
  artımlı 2 ms, Ortak sınırda köşe eksik olmamalı 28 ms ve 2 ms, Geçerli olmalı 6 ms ve 2 ms. Boşluk olmamalı tam 41 ms,
  artımlı 60 ms: blok tek küme olduğundan bütün olarak yeniden denetlenir, kümeleri bulmak da ek iştir.
- Resimler: `KENTOS_SHOTS_ONLY=degisti,yeniden cargo test -p kentos-desktop topology::tests::screens -- --ignored
  --nocapture` (`.run/shots/topoloji-degisti-*`, `topoloji-yeniden-*`).
