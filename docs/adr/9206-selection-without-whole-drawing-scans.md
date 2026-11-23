# ADR 9206: Nesne seçmek arayüzü yeniden kurmanın bedelini taşımaz

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-06.
- **İstek:** sahibin sözü (23 Kasım 2026): “sahnede obje seçtiğimde bazen arayüzün de yenilendiği fark ediyorum”. Ölçü:
  [ADR 9201](9201-render-measurement.md) §3, DWG'de bir tık 106 ms UI, bunun 65 ms'i `App::view`.
- **Sözleşmeler:** yok.

## §1 Sorun

iced'de her ileti uygulamanın görünümünü baştan kurar; seçim de bir iletidir. Görünümün kendisi ucuzdur: imleç
karesinde 0,5–2 ms. Ama SİVAS AVM DWG'sinde (601 619 nesne) bir nesneye tıklamak 103 ms UI tutuyordu:

| Pay | Süre |
|---|---|
| Görünüm | 64,9 ms |
| Uygulama | 37,0 ms |

Kullanıcı bunu “arayüz yenileniyor” diye görüyordu: kare gecikiyor, şerit ve paneller bir an donuyordu.

## §2 Ölçüm: neresi

**Yöntem:**

- Simgeli release sınama ikilisi, 6 iş parçacığıyla derlendi. Satır tablolu derleme bellek yetmediği için iki kez
  SIGKILL ile düştü.
- `perf record --call-graph dwarf`, 150 tık turu (`perf::million`, `KENTOS_PROFILE=click`, `KENTOS_PROFILE_FILE`).
- Tıklamalar “Tümünü göster”den sonra alanın ortasına yapıldı.

**Sonuç:** örneklerin **%53'ü** tek bir döngüdeydi: `appearance_origin::explain`.

- Öznitelikler panelinin “Bu değer nereden geliyor?” bölümü (ADR 1100 §5), katmanında kendi rengini, çizgi tipini ve
  kalınlığını taşıyan nesneleri sayıyor.
- Bunu belgenin **bütün** nesnelerini üç kez katmana göre süzerek yapıyordu (`doc.entities().filter(...)`).
- Panel seçim değişince bir kez kurulup saklanıyor (`PanelCache`). Yani bedel tam olarak her seçim değişikliğinde
  ödeniyordu.

## §3 Karar

1. **Sayım tek geçişte ve yalnız nesnenin katmanında.** Belgenin katman dizini kullanılır (`Document::by_layer`), üç
   sayı aynı geçişte toplanır. Sonuç aynıdır; üç tam belge taraması, katmanın tek taramasına iner.
2. **Seçim vurgusu zaten GPU'da** (`viewport/highlight.rs`, ADR 0029). Seçim ve gezinme, seçimin sürümüne bağlı birer
   wgpu parçası olarak saklanıyor; değişiklik gerekmedi.
3. **Arayüzün geri kalanına `lazy` gerekmedi.** Düzeltmeden sonra seçim karesinde görünüm 2,3 ms. Planda “yalnız ilgili
   panel değişir (`lazy`)” yazıyordu. Ölçü, iced'in yeniden kurmasının değil, kurulurken yapılan hesabın pahalı
   olduğunu gösterdi.
4. **Kalan 37 ms, isabet sorgusu.** Alan çok uzakta olunca imlecin altındaki aday kutusu nesnelerin sekizde birinden
   fazlasını kapsıyor. Bu durumda `Store::candidates` bütün nesneleri belge sırasıyla geziyor, ardından her aday için
   görünürlük ve kenara uzaklık hesaplanıyor. En yakını önce arayan bir sorgu uzamsal dizinin işi: RM-08'e yazıldı.

## §4 Kanıt

**Profil** (UHD 770, release, SİVAS AVM DWG, 150 tık turu):

| Ölçü | Önce | Sonra |
|---|---|---|
| Tıklama karesi UI | 103,4 ms | **40,7 ms** |
| Görünüm payı | 64,9 ms | **2,3 ms** |
| Uygulama payı (isabet sorgusu) | 37,0 ms | 36,9 ms |

Sonra en sıcak işlevler: `Store::candidates` %18, `Store::shown` %12, R-ağacı araması %6.

**Sınamalar** (değişmedi, geçiyor):

- `properties::tests::objects::the_value_origin_section_names_the_level_and_the_reset_scope`;
- Öznitelikler'in 83 sınaması;
- `office_profiles` (7), öğe türleri ve pafta ayrıntısı sahneleri.

## Uygulama

- `crates/native/application/src/core/appearance_origin.rs`: `explain`'in sayımı.
- `apps/desktop/src/shell/perf/million.rs`: `KENTOS_PROFILE_FILE` ve `click`.
- **Profil almak:**

  ```text
  cargo test --release --config 'profile.release.package.kentos-desktop.strip="none"' -p kentos-desktop --lib --no-run
  ```

  Ardından ikili `perf record --call-graph dwarf` ile koşturulur. Satır tablolu derleme bu makinede bellek
  yetmediği için düşüyor.
