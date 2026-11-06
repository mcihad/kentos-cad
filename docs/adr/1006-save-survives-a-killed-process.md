# ADR 1006: Kaydın gerçek süreç ölümünde ve disk reddinde korunması

- **Durum:** kabul edildi (2026-11-05). Kapsam: plan/cad-advanced.md `CAD-ADV-18` kabulü “Gerçek test sürecini kayıt
  sırasında sonlandırıp kurtar; … Disk dolması ve iptalde önceki geçerli dosya ve Undo zinciri korunmalı.”
- **Bağlam belgesi:** ADR 0654 (masaüstü kaydı: geçici dosya, geri okuma, yeniden adlandırma, hata enjeksiyonu),
  `crates/native/headless/src/files.rs` (`write_bytes`).

## 1. Karar

Kayıt kuralı değişmedi: geçici dosyaya yaz, diske indir (`sync_all`), geri oku, ancak sonra adı ver. Bu ADR kuralın
**gerçek** süreç ölümüne dayandığını kanıtlar; hata enjeksiyonu (masaüstünün `Fault` testleri) bunun yerine geçmez.

## 2. Kanıt

`crates/native/headless/tests/headless/save_crash.rs`:

- Test kendini alt süreç olarak başlatır. Çocuk, paftalı `sheets.kcad`'a 30 000 nokta ekleyen üç sürümü önce ayrı
  dosyalara (kendi kalıcı kimlikleriyle) yazar, sonra hedef ada sırayla, durmadan kaydeder. Ebeveyn onu 3 ms ile 1,8 s
  arasında on farklı anda `SIGKILL` ile öldürür.
- Her ölümden sonra adın altındaki dosya, özgün dosya ya da çocuğun yazdığı sürümlerden biriyle **bayt bayt** aynıdır
  (yarım dosya yok), açılır ve paftaları korunur; geçici dosya hiçbir zaman çizimin adı değildir. Öldürmeler en az iki
  farklı sürüme denk geldi.
- İlk denemede bayt eşitliği tutmadı; tanı dosyanın bütün olduğunu (60 001 nesne) ve farkın ebeveyn ile çocuğun aynı
  noktalara ayrı rastgele kimlik vermesinden geldiğini gösterdi. Başvuru baytları çocuğun kendi kaydından alındı;
  beklenti gevşetilmedi.
- Yazılamayan klasöre kayıt `file_unwritable` ile reddedilir; çizimin özeti (revizyon, kirli bayrağı) değişmez, önceki
  dosya bayt bayt aynı kalır, geçici dosya kalmaz ve Undo zinciri çalışır.

## 3. Sınırlar

Gerçek disk dolması (ENOSPC) bu makinede güvenle üretilemediği için klasörün yazmayı reddetmesiyle sınandı; masaüstünün
kendi kayıt yolu ayrı uygulamadır ve kendi hata enjeksiyonu testleriyle kapsanır. Onarılmış sonucun ayrı kurtarma
kopyası, kısmi okunabilen dosyadan veri çıkarma ve açılışta bütçeli sağlık denetimi açıktır.

## Ek (6 Kasım 2026): gerçek dolu disk

`save_crash::a_save_onto_a_full_disk_keeps_the_previous_file_and_the_drawing` (macOS) `hdiutil` ile ayrıcalıksız 1 MB'lık
bir disk görüntüsü bağlar, çizimi oraya koyar ve alana sığmayacak bir sürümü kaydetmeye çalışır: kayıt diskin kendi
ENOSPC'siyle `file_unwritable` döner, önceki dosya bayt bayt aynıdır, birimde geçici dosya kalmaz, çizim kaydedilmemiş
ve aynı revizyonda kalır, geri alma zinciri sağlamdır. Linux/Windows'ta aynı test yok (orada küçük bir dosya sistemi
bağlamak ayrıcalık ister).

