# ADR 9203: “Tümünü göster” çizimin geri kalanından çok uzaktaki nesneleri saymaz ve söyler

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-03.
- **İstek:** sahibin sözü (23 Kasım 2026): “veri kaybolmaları var … sahne tam bir CAD gibi gözükmüyor”. Bulgu:
  [ADR 9200](9200-render-engine-research.md) §1.3, ölçü [ADR 9201](9201-render-measurement.md) §3.
- **Sözleşmeler:**
  - Ortak çekirdeğe `Store::extent_outliers_of` eklendi; `extent_outliers`'ın kuralı aynı.
  - Komut, ayar ve `.kcad` değişmedi.
- **Bağımlılık:** [ADR 0141](0141-netcad-hand-navigation-query-selection.md) (Kapsam denetimi),
  [ADR 0311](0311-layer-flags-states-find-translate.md) §1 (donmuş katman sayılmaz).

## §1 Sorun

“Tümünü göster” (orta tuşa çift tık) iki gerçek dosyada çizimi ekranda bir noktaya indiriyordu:

| Dosya | Açılışın kutusu | “Tümünü göster”in kutusu |
|---|---|---|
| Belediyenin imar planı NCZ'si (453 534 nesne) | 36 km'lik kent | x'te −4 104 135 … 4 702 431, y'de −374 748 … 4 785 838: 8 800 km |
| SİVAS AVM DWG'si | −242 … 350 | −3 515 … 3 841, 0,0 dahil |

Açılış dosyanın kendi saklı görünümünü kullandığı için doğru görünüyordu. “Tümünü göster” ise uzamsal dizindeki bütün
nesnelerin kutusunu alıyordu. Sıfıra düşmüş ya da başka bir sistemle gelmiş birkaç nesne, bütün çizimi bir
noktaya indiriyordu. Kullanıcının gözünde veri kaybolmuştu.

## §2 Araştırma

- **CAD programlarında** kapsama göre yakınlaştırma bütün nesneleri sayar, donmuş katmanı saymaz. Kullanıcı aykırı
  nesneyi ayrı bir araçla arar.
- **KentOS'ta bu araç var:** Kapsam denetimi (ADR 0141). Çekirdek kutuyu nesne merkezlerinin ortadaki yarısından
  kurar. Kutusu çekirdekten, çekirdeğin uzun kenarının 10 katından (en az 100 m) uzak olan nesne uzaktır. ADR 0141 bu
  aracın örneklerini zaten sayıyor: “yanlış koordinat sistemiyle gelen ya da sıfıra düşen nesneler”.
- **Yalnız sessizce dışarıda bırakmak kayıplı olurdu:** CLAUDE.md §7, atlanan nesnelerin açıklanmasını ister.

## §3 Karar

“Tümünü göster” ile saklı görünümü olmayan bir çizimin açılışı **tek kutuyu** kullanır (`selecting::extents_box`).

- **Kutuya girenler:**
  - Donmuş katmanda olmayan nesneler; gizli katmandakiler sayılır (0311 §1).
  - Gösterilen veri katmanlarının tabloları (9101 §5).
- **Uzak nesneler:** Kapsam denetiminin kuralıyla, aynı nesne kümesi içinde bulunur ve kutuya girmez. **Genişleme
  (25 Kasım):** kutusu çekirdekten erişimin ötesine uzanan (bir ucu 0,0'a düşmüş) ya da sonlu olmayan nesne de uzaktır
  ([ADR 9225](9225-golden-pictures-and-grid-bound.md) §2.3). Kural, gizli
  katmanlar da sayıldığı için kümeye uygulanır (`Store::extent_outliers_of`). Kapsam denetimi ise yalnız görünenlere
  bakar, değişmedi.
- **Söylenir (uyarı):** “3 nesne çizimin geri kalanından çok uzakta; görünüme alınmadı. Kapsam denetimi onları
  seçer.”
- **Uzak nesne yoksa** kutu eskisinin aynısıdır ve hiçbir şey söylenmez.
- **Saklı görünüm önce gelir.** Bir dosyanın kendi açılış görünümü değişmedi.

## §4 Planda olmayan kararlarım

1. **AutoCAD'in “her şeyi say” kuralından ayrıldım.** Uzak nesne sessizce dışarıda bırakılmıyor, sayısı söyleniyor ve
   onları seçmenin yolu adıyla veriliyor. Bir noktaya inmiş bir çizim, hem bu komutu hem de çizimin tamamını
   kullanılamaz kılıyordu.
2. **Açılışın iki kodu teke indi.** `show_document` kutuyu ayrıca kuruyordu; artık “Tümünü göster”in yardımcısını
   çağırıyor. Açılışta ileti verilmez: açılış raporu zaten açılıyor ve kullanıcı henüz bir şey istemedi.

## §5 Kanıt

**Sınamalar:**

- `navigation::tests::show_everything_leaves_out_an_object_fallen_to_zero_and_says_so`: sıfıra düşen bir nokta görünümü
  değiştirmez; uyarı söylenir; Kapsam denetimi onu seçer; silinince bir şey söylenmez ve görünüm aynıdır.
- `navigation::tests::extent_check_*` (Kapsam denetimi) değişmedi ve geçiyor.

**Gerçek pencere:** `.run/gui-evidence/render/rm03/{once,sonra}`.

## Uygulama

- `crates/shared/geometry-core/src/store/select.rs`: `extent_outliers_of`, `far_from_rest`.
- `apps/desktop/src/cad/selecting.rs`: `extents_box`, `zoom_extents`.
- `apps/desktop/src/shell/app/settings.rs`: `show_document`.
- Sınama: `apps/desktop/src/shell/navigation.rs`.
