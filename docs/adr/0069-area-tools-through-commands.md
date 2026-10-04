# ADR 0069: Alan işlemleri ürün komutlarıyla; yaylı iki köşeli halka; U beklerken Geri; tutamağa yazılan değer

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.7, §7, §18; TODOS.md `CMD-04`, `CMD-07`, `UX-01`; ADR 0047 (`cad.entities.edit`), 0057 (`cad.entities.create`), 0065 (alan işlemleri), 0067 (Uzunluk'un beklemesi), 0068 (tutamaçlar)
- **Web ajanının işi (27 Eylül):**
  - `61f98cd`: alan araçları ürün komutlarıyla, iki köşeli yaylı halka kuralı;
  - `7ffdfea`: U'nun istemi Geri'yi gösterir, bekleyen tutamaç yazılan değeri alır.
  Masaüstü tarafı bu ADR'yle geldi.

## Bağlam

- Alan işlemleri iki platformda belgeye doğrudan yazıyordu (ADR 0065). Python ve yapay zekâ yüzeyi yalnız ürün komutlarını kullanacak (CLAUDE.md §18).
- Komutların ortak geometri denetimi, halkası 3'ten az köşeli kapalı alanı reddediyordu. Ama çekirdek dairenin alanını iki köşeli yaylı halka yazar (`[c + r, c − r]`, bulge `[1, 1]`), iki dairenin kesişimi (mercek) ve doğruyla kesilen daire parçası da öyledir. Belgede bulunan bir şekli komut yazamıyordu.
- Uzunluk (U) beklerken istemde seçenek yoktu. G tuşu Kapalı alan'ın kısayolu olarak çalışıp çizimi bırakıyordu (ADR 0067'nin izinde görüldü).
- Web'de tutamaç sürerken istem "koordinat yazın" diyordu ama yazılan araca ulaşmıyordu (ADR 0068).

## Karar

### Alan işlemleri `cad.entities.edit` ve `cad.entities.create` ile

- **Yeni işlemler.** `EditOperation`'ın her biri adımı adlandırır:
  - `areaUnion` "Alan birleştir";
  - `areaIntersect` "Alan kesiştir";
  - `areaSubtract` "Alan çıkar";
  - `areaSplit` "Alan böl";
  - `toArea` "Alana çevir";
  - `toPolyline` "Çizgiye çevir".
  İçine tıklayarak alan `cad.entities.create`'in `boundary` işlemiyle yazar, adımı "Alan oluştur"dur.
- **Araçların değişiklikleri:**
  - Birleştir: kaynaklar `remove`, birleşimin parçaları ilk alandan `add` (keepData).
  - Kesiştir:
    - kaynaklar kalırsa ortak parçalar ilk alandan boş `add`;
    - Kaynakları sil ile kaynaklar `remove`, parçalar keepData ile.
  - Çıkar:
    - değişen her alan `remove`, kalanları ondan `add` (keepData);
    - Çıkarılanları sil ile kilitsiz kesiciler `remove`.
  - Böl: ilk parça `replace` (keepData), öbürleri alandan `add` (keepData).
  - Alana çevir: kapalı nesneler `replace` (keepData), çizgilerin yüzleri ilk çizgiden boş `add`.
  - Çizgiye çevir: dış halka `replace` (keepData), delikler alandan boş `add`.
- **Kullanıcının gördüğü değişmedi:** iletiler, adımlar, sonradan seçilenler (bölmede ve çizgiye çevirmede her alan kendi yeni parçalarıyla sırayla) ve komuttan önce kilitli katmanın ayıklanması.
- **Uç durumlar** yalnız İçine tıklayarak alan'da:
  - etkin katman yoksa `layer_not_found`;
  - etkin düğüm grupsa `not_a_layer`;
  - gizli katmanın uyarısı yazmadan sonra, başarı iletisinden önce söylenir.

### İki köşeli yaylı halka

- **Kural:** yalnız düzenleme ve eklemenin ortak geometri denetiminde (`cad.polygon.create` 3 köşede kalır). Kapalı alanın dış halkası ya da deliği geçerlidir, eğer:
  - en az 3 köşesi varsa;
  - ya da tam 2 köşesi varsa ve iki kenarından biri yaysa.
- Bulge verilmemişse düz sayılır. −0 düzdür; NaN yay sayılır, ardından sonluluk denetimi onu reddeder.
- **Yeni iletiler:**
  - "Kapalı alanın en az 3 köşesi olmalı (kenarlarından biri yaysa 2); 2 köşe verildi. Eksik köşeleri ekleyin."
  - "N. deliğin en az 3 köşesi olmalı (kenarlarından biri yaysa 2); … ya da deliği çıkarın."
- Taramanın halkası 3 köşede kalır.
- **Ortak durumlar:**
  - düzenleme 42: her işlem; halka kuralı; alana çevrilen daire, mercek ve yarım diskler;
  - ekleme 30: disk ve yarım disk adalı Alan oluştur, düz iki köşeli halka ve delik reddi, iki köşeli tarama reddi;
  - toplam 337. İki platform geçiyor.

### U beklerken Geri (G)

- İstem: "son doğrultuda devam edilecek uzunluğu yazın [Geri (G)]". G artık aracındır: noktayı geri alır, beklemeyi bitirir.
- `measure-parcel` ve `polyline-arc` izlerinde U adımının seçenekleri `["G"]` oldu.

### Bekleyen tutamaç yazılan değeri alır

- **Web** (`takesTypedInput`): tutamaç taşınırken komut satırı yazılanı seçim aracına verir ve komut önermez. İmleç yanındaki değer alanı da açılır. + ve − değeri başlatır, yakınlaştırmaz.
- **Masaüstü:** ADR 0068'de komut satırı zaten veriyordu. Artık değer alanı da açılır ve + ile − değeri başlatır.
- `grips` izine yazılan adım eklendi: sıcak tutamaca "@0,4" ve Enter.

## Doğrulama

- `cargo test -p kentos-native-application`: 337 ortak durum masaüstü işleyicilerinde.
- `crates/native/interaction/tests/all/area.rs`: alan araçlarının iletileri, adımları ve seçimleri komutlarla da aynı.
- İzler (`areas`, `measure-parcel`, `polyline-arc`, `grips`) iki platformda üç varyantta.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm e2e:interaction`, `pnpm inventory:check`.
