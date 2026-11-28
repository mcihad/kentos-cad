# ADR 9401: İncelemenin doğruluk düzeltmeleri

- **Durum:** kabul edildi (2026-11-28). [plan/bakim.md](../../plan/bakim.md) Aşama 1, BK-10…BK-19.
- **Kural:** CLAUDE.md §7 (belge değişikliği geri alınabilir; düzenleme bütün yazılır ya da hiç), §8 (hata iletisi),
  §20.2 (geç kalan sonuç başka belgenin üstüne uygulanmaz), §23 (sayısal doğruluk), §25 kural 6 (veri yalnız komutla
  değişir).
- **İlgili:** [ADR 9400](9400-maintenance-plan.md) (bakım planı), [ADR 0253](0253-command-protocol.md)
  (komut adımı ve kayıt).
- **Ürün:** hata düzeltmeleri; her biri aşağıda. **`.kcad`:** değişmedi. **Sözleşme:** madde madde yazılı.

## Bağlam

28 Kasım 2026 incelemesi ([plan/bakim.md](../../plan/bakim.md) “A. Doğruluk”) on bir doğruluk bulgusu çıkardı. Her
düzeltme ayrı commit'tir ve düzeltmeden önce kırmızı olan bir sınamayla gelir. Bu ADR her birinin kararını tutar.

## Kararlar

### 1. İmar takımı bütün yazılır ya da hiç (BK-10)

**Bulgu.** `kentos_native_application::permit_set::execute` önce künyeyi ayarlara yazıyor, sonra sayfaları
planlıyordu. Plan reddedilince (kat yok, bina bulunamadı, şablon okunamadı) künye yazılı kalıyordu. Ayarlar geri alma
adımı değildir; masaüstünün açtığı grup da onları geri almaz.

**Karar.**

- Takım önce planlanır: sayfalar, tadilat görünümlerinin denetimi ve pafta işlemleri yalnız okur. Hiçbiri künyeye
  bakmaz. Plan reddedilirse hiçbir şey yazılmamıştır.
- Sonra künye ve görünümler tek `set_settings` ile, ardından paftalar `cad.sheets.edit` ile yazılır. Paftalar
  reddedilirse ayarlar eski hâline konur.
- Bütünü bir `Document::run_step` çerçevesine almak seçilmedi: çerçevenin içindeki komut kendi kayıt satırını yazmaz
  (ADR 0253), Komut geçmişi takımın `cad.sheets.edit` satırını kaybederdi. Kalıcı çözüm takımın kendi komutu
  olmasıdır (BK-19).

**Sınama.** `permit_set::tests::a_refused_set_writes_no_particulars`: katsız çizimde takım reddedilir; ayarlar ve
revizyon değişmez. Düzeltmeden önce kırmızıydı (künyenin değişkenleri yazılıyordu).

### 2. İfade ve süzgeç ayrıştırıcılarında derinlik ve işlem sınırı (BK-14)

**Bulgu.** İfade dilinin ayrıştırıcısı (`crates/shared/expression/src/parser.rs`) ve stil süzgeci dilbilgisi
(`crates/shared/formats/src/sld.rs` `parse_qgis`, `parse_filter`) iç içe yapıları kendini çağırarak okur; ağacı gezen
öbür işlevler de (çözümleme, derleme, kaynağa itme) öyle. Derinliği sayan yoktu. İçe alınan bir stil dosyasının süzgeci
ya da bir proje dosyasındaki ifade programı çökertebilirdi; yığın taşması yakalanamaz.

Ölçüm (28 Kasım 2026, bu Mac; 2 MB yığınlı iş parçacığı, bir işçi iş parçacığının boyu):

| Girdi | Hata ayıklama derlemesi | Sürüm derlemesi |
|---|---|---|
| iç içe parantez `(((…1…)))` | 150 geçer, 200 taşar | 700 geçer, 1 000 taşar |
| işaret zinciri `----1` | 300 geçer, 400 taşar | 2 500 geçer, 3 000 taşar |
| işlem zinciri `1 + 1 + …` | 300 geçer, 400 taşar | 2 500 geçer, 3 000 taşar |

Üçüncü satır düşmanca girdi değildir: `a = 1 VEYA a = 2 …` biçiminde 3 000 terimli bir süzgeç sürüm programını
çökertiyordu. Zincir soldan derinleşen bir ağaçtır (`a + b + c`, `(a + b) + c`'dir): uzunluğu kadar derindir.

**Karar.**

- İfade dili: en çok **64** düzey iç içelik (`MAX_DEPTH`; parantez, işaret, argüman, dal) ve bir ifadede en çok
  **1 000** birleştirilen işlem (`MAX_OPERATIONS`). 64, JSON ve CBOR okuyucularının sınırıdır. Aşan ifade derlenmez;
  ileti yerini, nedenini ve çözümünü söyler (uzun değer listesi `içinde (…)` ile yazılır).
- Derinlik `unary`'de sayılır: dilin her iç içeliği oradan geçer. İşlemler `binary`'nin döngüsünde sayılır: ağacın
  özyinelemesiz derinleştiği tek yer orasıdır. Ağaç hiç kurulmadığı için reddin kendisi de yığını taşırmaz.
- Stil süzgeci: en çok 64 düzey; aşan süzgeç okunmaz (`None`, dilbilgisinin söyleyemediği süzgeç gibi). `VE` ve `VEYA`
  orada zaten düz listedir, zincir derinleşmez.
- Sınır derleme profiline göre değişmez: ortak durumlar her yerde aynı sonucu verir. 1 000 işlem sürümde 2 MB'ta 2,5 kat
  payla güvenlidir. Hata ayıklama derlemesi 2 MB'ta 300 işlemde taşar; cargo'nun koşturduğu her program 64 MB yığın alır
  (`.cargo/config.toml`, ADR 9400 §5), elle koşturulan hata ayıklama ikilisi almaz.
- Davranış değişikliği: 1 001–2 500 işlemli ifade sürümde derleniyordu, artık reddedilir.

**Sınama.** `crates/shared/expression/tests/language.rs` `an_expression_nested_past_the_limit_is_refused` ve
`an_expression_of_too_many_operations_is_refused` (100 000 parantez, işaret ve terim 2 MB'lık iş parçacığında
reddedilir; sınırın iki yanı); `sld::tests::a_filter_nested_past_the_limit_is_not_read`. Düzeltmeden önce aynı girdiler
süreci düşürüyordu (yukarıdaki ölçüm).

## Modül ve kanal tablosu

| İş | Mimari | Harita | Yol | CBS | MCP | Python | Eklenti | Yapay zekâ ajanı |
|---|---|---|---|---|---|---|---|---|
| İmar takımı (BK-10) | var: Mimari › Proje ▾ › Ruhsat takımı (`arch.permitSet`) | yok: Mimari'nin işi | yok | yok | yok: katalog komutu değil (BK-19) | yok (BK-19) | yok (BK-19) | yok (BK-19) |
| İfade ve süzgeç sınırları (BK-14) | var | var | var | var | var: aynı ayrıştırıcı | var: aynı ayrıştırıcı | var: aynı ayrıştırıcı | var: aynı ayrıştırıcı |

Düzeltmeler hiçbir hücreyi değiştirmedi. İfade dili her modülde ve her kanalda aynı crate'ten (`kentos-expression`)
derlenir; sınır hepsinde aynıdır.
