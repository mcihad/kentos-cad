# ADR 9323: Veri katmanının tablosu kodlu alanı listesiyle düzenler; reddedilen değer hücrede nedeniyle kalır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  4, P2; ilk dilim). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 9103](9103-edit-session.md) §5 (veri tablosunda hücre düzenleme),
  [ADR 0418](0418-retired-values.md) (kullanımdan kalkan kodlar), [ADR 0422](0422-subtypes.md) (alt türler);
  [ADR 9324](9324-field-rules-in-the-source.md) (bu maddenin ikinci dilimi: kural kaynakta).

## Bağlam

- **Kusur:** listesi olan alanın hücresine çift tıklanınca seçenek listesi yerine düz metin kutusu açılıyordu.
  - Geçersiz değer reddediliyor, hücre NULL kalıyordu.
  - Hata kısaydı ve alt çubukta kesiliyordu; hangi hücrenin neden reddedildiği hücrede görünmüyordu.
- **Kod:**
  - Çizim katmanlarının Öznitelik tablosu kodlu alanı zaten listesiyle düzenliyordu. Kullanımdan kalkan kod yalnız onu
    tutan kayda sunuluyor, alt türler ve bağımlı değerler seçenekleri daraltıyordu (`choices_of`, `offered_on`).
  - Veri katmanının tablosu (`features/data.rs`) her alan için düz metin kutusu açıyordu.

## Karar

### 1. Aynı tipli düzenleyici

- Veri katmanının tablosunda listesi olan ya da evet/hayır alanının hücresi, çizim tablosunun liste düzenleyicisiyle
  açılır (`choices_of` ortak).
  - Kod etiketiyle seçilir, boş seçeneği vardır.
  - Kullanımdan kalkan kod yalnız onu tutan satıra sunulur.
  - Alt tür ve bağımlı değerler, satırın öbür alanlarına göre daraltır.
- Seçilen kod Enter'ın yolundan yazılır: satır gerekirse alınır, nesnesine yazılır, düzenleme oturumunda bekler.

### 2. Reddedilen değer hücrede kalır

- Alanın kuralı değeri reddederse hücre yazılanla açık kalır. Yanındaki uyarı simgesinin ipucu, denetimin kendi
  sözüyle (`write_attribute`) nedeni söyler; ileti Komut geçmişine de yazılır.
- Yeni bir yazı ya da Esc nedeni kaldırır. Düzeltilen değer Enter'la yazılır.

### 3. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Veri tablosunda tipli hücre, hücrede ret | yok: veri katmanı CBS'nindir | yok: aynı gerekçe (çizim tablosu zaten tipli) | yok: aynı gerekçe | var |

### 4. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Değer `cad.entities.set` (öznitelik) ile, aynı denetimden geçer; liste ve ret sözleri ortak çekirdekten. Hücre düzenleyicisi masaüstünün sunumudur. |
| MCP | var | Aynı komut ve denetim. |
| Yapay zekâ ajanı | var | Aynı komut. |
| Eklenti | var | `document.write` izniyle aynı komut. |

## Açık

- Kuralın kaynağa yazılması (GeoPackage şema uzantısı, PostgreSQL denetim kuralı), alan özelliklerinde “proje kuralı” ile
  “kaynakta kısıt” ayrımı ve listeye uymayan eski kodların listesi [ADR 9324](9324-field-rules-in-the-source.md)'tedir.
- Veri tablosunda toplu yapıştırma yok; çizim tablosunun yapıştırması reddedilenleri satırlarıyla söyler.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- typed_cell_tests closed_cell_tests null_tests`: 6 geçti.
  - `typed_cell_tests`:
    - `mahalle`'ye M1/M2/M3 listesi verildi; hücrede M3 seçilince alınan satıra M3 yazıldı.
    - `bedel`'e “yüz” yazılınca hücre yazılanla açık kaldı, neden `bedel`'i andı.
    - “120” yazılınca neden kalktı, Enter değeri yazdı.
- **Resim** (`.run/gui-evidence/kullanim-9323/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-listeli-hucre`: `mahalle` hücresi “Çınar” seçili listeyle;
  - `02-reddedilen-deger`: “yüz” hücrede, yanında uyarı simgesi. Beşinci satırdaki listede olmayan M9, ADR 9324'ün
    “eski kod” örneğidir.
