# ADR 0526: Olay kaymasını denetle

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-27'nin dilimi (“doğrusal referansta olayların rotanın kaynağına
  bağı; kaynağın M'si ya da geometrisi değişince kaymanın raporu”). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0224 (doğrusal referanslama: rotalar, Olayları rotada göster, Rota boyunca konumla), 0508
  (İşlemler'in araçları komut olarak: `cad.processing.run`), 0525 (CBS-27'nin ilk dilimi).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** rota değişince olayların kendiliğinden izlemesi (belgenin adımında, ilişkili ölçü gibi); bugün kayma
  sorulunca raporlanır ve istenince düzeltilir. Terminaller, alt ağlar ve artımlı graf ADR 0525'in listesinde.

## Bağlam

Olayları rotada göster (ADR 0224 §5) bir tablonun satırlarını rotaların üstüne nesne olarak yazar; satırın dolu hücreleri
nesnenin öznitelikleri olur, yani olay nesnesi rota kimliğini ve ölçüsünü (Km, Başlangıç, Bitiş, Öteleme) kendi taşır.
Ama yazıldıktan sonra rota değişebilir: eksen düzeltilir (köşe taşınır), rota yeniden kalibre edilir (M'ler değişir).
Olay nesnesi eski yerinde kalır ve artık ölçüsünün söylediği yerde değildir. ArcGIS'in olay davranışları (Event Behavior)
bunu rota düzenlenince olayın kaydığı ya da kaldığı diye sınıflar; bizde önce bunun görünmesi gerekir.

## Karar

### 1. Bağ

Olayın rotasına bağı rota kimliğidir: rotaların kimlik alanının değeri, Olayları rotada göster'deki gibi (aynı kimliğin
ilk nesnesi). Bağ nesnenin kendi özniteliklerindedir; şema değişmez. Rota silinirse ya da kimliği değişirse olay
“rotası yok” olur.

### 2. Kayma

**Olay kaymasını denetle** (`linearRef.eventDrift`, İşlemler › Doğrusal referans) olay türündeki her nesneyi (Nokta ya da
Çizgi: çizgi ve çoklu çizgi; öbürleri sayılır, denetlenmez) kendi özniteliklerinden Olayları rotada göster'in kuralıyla
rotanın bugünkü hâline yeniden yerleştirir ve şimdiki yeriyle karşılaştırır:

- nokta olayının kayması iki noktanın uzaklığıdır;
- çizgi olayınınki iki yolun birinin köşelerinin ve yaylı kenarlarının ortasının öbür yola en büyük uzaklığıdır (iki
  yönde; `ops::lrs::line_drift`). Böylece altında taşınan köşe, rota boyunca yürüyen uç ve bükülen yay bulunur.

Toleranstan (varsayılan 1 mm) çok kayanlar tabloda (Nesne, Rota, Kayma, Durum) nesnelerin sırasıyla, gösterilemeyenlerle
(rota yok, ölçü sayı değil, ölçü rotanın dışında) birlikte. Kayan varsa uyarı düzeyinde söylenir.

### 3. Yerine taşı

Açıkken kayan olay rotanın verdiği yere taşınır: nokta noktaya, çoklu çizgi rotanın parçalarına (yayları ve M'leriyle),
çizgi nesnesi yalnız yol düz tek kenarsa (çizgi kırıklı yola taşınamaz, söylenir). Öznitelikler değişmez; tek geri alma
adımı “Olay kaymasını denetle”. Kilitli katmandaki olay İşlemler'in kuralıyla atlanır.

### 4. Arayüz, komut, Python

- CBS › Analiz › Ağ analizi ▾'de (Doğrusal referans'ın öbür araçlarının yanında), ikon `routeDrift`.
- Araç bir İşlemler aracıdır; Python ve MCP onu `cad.processing.run` ile çalıştırır (ADR 0508); manifestte `semantic`.
- Arayüz kaydının yeni kuralı: var olan panele (`into`) yalnız ▾ listesine komut eklenebilir (`items` boş, `overflow`
  dolu); yeni panel düğme ister.

## Doğrulama

- `scripts/fixtures/lrs_processing_cases.py --check`: `lrs.kcad`'e on bir olay nesnesi; üç durum (nokta: yerinde,
  toleransın içinde yayın üstünde, 5 m kaymış, olmayan rota, dışarıdaki ölçü, sayı olmayan ölçü; tolerans 6 m; çizgi ve
  Yerine taşı: köşesi kayan çoklu çizgi, 1 m kayan çizgi, kırıklı yola düşen çizgi, yaylı rotadaki çoklu çizgi). Başvuru
  KentOS kodu olmadan, `lrs_cases.py`'nin yerleştirmesi ve 50 basamaklı uzaklıklarıyla.
- `cargo test -p kentos-processing --test cases the_linear_referencing_cases`: aynı durumlar (değişen geometri
  `toleranceFields`'ın `geometry`'siyle toleransta).
- `cargo test -p kentos-desktop processing::lrs_tests`: durumlar pencereden; Yerine taşı tek adım, Ctrl+Z geri alır.
- `cargo test -p kentos-contracts ui_catalog`: ▾'e ek kuralı.
- Resim: `KENTOS_SHOTS_ONLY=kayma,kayma-tasindi cargo test -p kentos-desktop processing::lrs_tests::screens -- --ignored
  --nocapture --test-threads=1` (`.run/shots/islem-dr-kayma-*`): levhalar yazıldıktan sonra D-100'ün son köşesi 40 m
  kuzeye taşınır; araç kayan levhaları sayar, Yerine taşı onları yeni eksene oturtur.
