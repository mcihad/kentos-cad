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

## Modül ve kanal tablosu

| İş | Mimari | Harita | Yol | CBS | MCP | Python | Eklenti | Yapay zekâ ajanı |
|---|---|---|---|---|---|---|---|---|
| İmar takımı (BK-10) | var: Mimari › Proje ▾ › Ruhsat takımı (`arch.permitSet`) | yok: Mimari'nin işi | yok | yok | yok: katalog komutu değil (BK-19) | yok (BK-19) | yok (BK-19) | yok (BK-19) |

Düzeltme hiçbir hücreyi değiştirmedi.
