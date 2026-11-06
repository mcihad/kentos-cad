# ADR 1013: Bozuk `.kcad`'in katı okuyucudan ayrı yolla, ayrı kopyaya kurtarılması

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-18` açıkları “Onarılmış sonuç ayrı
  kurtarma kopyası olsun; sağlam kaynak ezilmesin” ve “Kısmi okunabilen dosya için veri çıkarma yolu … normal
  okuyucunun katı doğrulamasından ayrı olsun; kurtarılan ve kaybolan kayıtlar açık raporlansın. Bozuk ilişkiyi
  görünmez silmek başarı sayılmasın.”
- **Bağlam belgesi:** docs/specs/kcad-v2.md (kap, CBOR profili, anahtar sırası), ADR 0025, ADR 0654 (kayıt yolu).

## 1. Kaynak

AutoCAD'in [RECOVER](https://help.autodesk.com/cloudhelp/2016/ENU/AutoCAD-LT/files/GUID-7AFE414B-BCC6-470C-B0E9-8228A252EE7C.htm)
komutu açılamayan çizimi okuyabildiği kadarıyla açar ve AUDIT'le düzeltir; sonuç kullanıcı kaydedene dek kaynağa
yazılmaz. KentOS kararı: kurtarma okuyucusu ayrıdır, katı okuyucuyu gevşetmez, sonucu kendiliğinden ayrı bir kopyaya
yazar ve kaynağı yalnız okur.

## 2. Karar

- `kentos_kcad::salvage` (yeni, `crates/shared/kcad/src/salvage.rs`):
  - Kap gevşek okunur: özeti tutmayan, kesik ya da sonunda fazla baytı olan dosya; her biri `container`'da söylenir.
    İmzanın ilk beş baytı, başlığın boyu ve kodek (yalnız sıkıştırmasız) tutmalı; tutmazsa kurtarılacak bir şey yok.
  - Yük genel bir CBOR ağacı olarak okunur ve baytlar bittiği ya da bozulduğu yerde **her düzeyde** durur; kesilen
    öğe kesildiği yere kadar tutulur (okuyucu neyin eksik olduğunu söyler), kesikten sonrası okunmaz.
  - Kalanı **katı okuyucunun kendisi** belirler: çizim, katı okuyucudan geçen parçalardan yeniden kurulur. Önce
    nesneler olmadan; çizimin okunmasını engelleyen anahtar (sondan başa) çıkarılır ve okuyucunun koduyla, sözüyle
    `document.<anahtar>` olarak kaydedilir. Sonra nesneler ikiye bölünerek geri konur: tek başına okunmayan nesne
    `entities[i]` olarak, okunabildiyse kalıcı kimliğiyle ve okuyucunun sözüyle kaydedilir. Kötü nesne sayısı k,
    nesne sayısı n iken yaklaşık k·log n katı okuma yapılır.
  - Kesik dosyada nesnelerden sonra gelen zorunlu anahtarlar (`settings`, `activeLayer`; anahtar sırası boy, sonra
    bayt) boş çizimin değeriyle konur ve **söylenir**: koordinat sistemi SRID 0 olur, kullanıcı yeniden atamalıdır;
    etkin katman ilk katmandır. Kesikten sonraki isteğe bağlı anahtarlar bilinemez; `container` yükün kesildiğini
    söyler.
  - Dosya katı okuyucuyla okunuyorsa `whole: true`.
- `file.recover` v1 (headless dosya komutu, masaüstünde Dosya › Kaydet › **Kurtar**): girdi `path` (mutlak `.kcad`)
  ve isteğe bağlı `target` (verilmezse kaynağın yanında `<ad>-kurtarilan.kcad`). Retler sırasıyla
  `invalid_file_path` (hedef kaynağın kendisi de), `file_not_found`, `file_exists` (hedef var; üzerine yazılmaz),
  `kcad_unrecoverable` (yeni), `file_unwritable`. Yürütme kurtarılan çizimi kaydın kendi yolundan yazar (geçici dosya,
  geri okuma, yeniden adlandırma); sağlam dosyada hiçbir şey yazmaz; plan yazmaz. Masaüstü sonucu, kaybolan her
  parçayı (en çok 12 satır) Komut geçmişine yazar ve kopyayı açar.

## 3. Kanıt

- `crates/shared/kcad/tests/all/salvage.rs` (5 test) hasarları geçerli dosyadan yapar, beklentiyi hasarın yapılışından
  alır: özet bozulunca bütün nesneler aynı; k. nesneye (k−1). nesnenin kimliği verilince yalnız o nesne
  `duplicate_uid` ile çıkar ve kalan çizim yeniden yazılıp okunur; k. nesnenin kimliğinin ortasında kesilince ilk k
  nesne ve katmanlar gelir, ayarlar ve etkin katman yerine konduğu söylenir.
- `scripts/fixtures/recover_cases.py` (KentOS kodu yok; kimlikleri bağımsız `tools/kcad/kcad.py` okur) 10 hasarlı
  dosya ve 22 ortak vaka yazar; `crates/native/headless/tests/headless/recover_cases.rs` hepsini oynatır, kaynağın
  bayt bayt değişmediğini, yazılan kopyanın katı okuyucuyla açılıp beklenen kimlikleri taşıdığını, yazılmayanın
  olmadığını denetler.
- İlk koşuda gerçek bir hata bulundu: kesik iç öğeden sonra üst diziler yanlış yerden okumayı sürdürüp sahte nesneler
  üretiyordu (2 nesne yerine 5 “okunmuş”, üçü anlamsız). Ayrıştırıcı kesikte her düzeyde durur oldu; kesilen yarım
  nesne sessizce düşmez, kayıp olarak söylenir. Başvurunun beklentisi buna göre (k + 1 okunan, `entities[k]` kayıp)
  yazıldı.
- Masaüstü: `recover_file.rs` testleri (kopya yazılır, açılır, kaynak değişmez; sağlam dosya kopyalanmaz) ve resim
  `kurtar`: kesik dosyanın kopyası açılır, Komut geçmişi kayıpları söyler; ayarlar kaybolduğu için çizimin türü
  (CAD/CBS) de bilinmez ve masaüstü bunu açılışta sorar.

## 4. Sınırlar

- Kurtarma blok tanımlarını ve sayfaları bölüm düzeyinde tutar ya da çıkarır; bir bloğun içindeki tek bozuk nesne
  bütün `blocks` bölümünü (ve onun yerleştirmelerini) götürür. Tanım düzeyinde bölme ayrı iştir.
- Sıkıştırılmış yük (bugün yazılmıyor) ve v1 JSON dosyaları kurtarılmaz.
- Gerçek disk dolu (ENOSPC) senaryosu hâlâ kanıtlanmadı (ADR 1006 yazma reddini sınar).
