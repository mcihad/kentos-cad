# 3004 — Komut geçmişi okunur Türkçe özet yazar; kimlik ve alanlar Ayrıntı'da

Tarih: 2026-11-07. Durum: kabul (M5, ADV-08; plan S14, araştırma A1 S15).

## Sorun

Komut geçmişi bir komutun girdisini alan alan döküyordu. Türkçe adı olmayan alan ve değer İngilizce kalıyor, satırın başında
komut kimliği duruyordu. Mimari komutlarının fikstür girdileri geçmiş yazıcısından geçirilince:

| Komut | Satır |
|---|---|
| Duvar oluştur | `cad.walls.create v1 · adım 1` · "Katman: duvar · walls: 1 öğe" |
| Dikdörtgen duvar | "adopt: evet · frame: corners (başlangıç 4.2, 0, bitiş 7.2, 3)" |
| Açıklık ekle | "Katman: kapi · Açıklıklar: 1 öğe" |
| Duvarı düzenle | "İşlem: shift (yöntem 0.3)" (`by` her komutta "Yöntem" sayılıyordu) |
| Ölçülü bölme | "Ölçü: clear · Yan: left · source: edge 0, wall $uid:source" |

Kimin neyi çizdiği okunmuyordu: "1 öğe" bir kapının hangi kapı olduğunu söylemiyor, `$uid` ve
`0192f5a1-…` kullanıcıya hiçbir şey anlatmıyor.

## Araştırma (7 Kasım 2026)

- **AutoCAD 2026 Activity Insights — What's Changed** (help.autodesk.com, AutoCAD 2026 What's New):
  - Panel "a high-level summary of changes made during each edit session" verir.
  - Olayları düz cümlelerle yazar: "Base point moved from 0,0,0 to 50,35,0", "Length units changed … Architectural to
    Decimal", "2 more text styles" (referans resim `.run/gui-evidence/mimari-m1/referans/autocad-activity-insights/`).
  - Seçilen etkinliğin ayrıntısını ayrıca gösterir: "provides detailed information for selected activities".
- **AutoCAD komut penceresi** (ADR 8003'ün kaynağı): girdiyi kullanıcının verdiği gibi yazar, iç kimlik göstermez.
- **Mevzuat:** bu işe doğrudan bir hüküm yok. 2026 Dijital Hazırlama Yönetmeliği EK-2 md. 2.11 yalnız revizyon numarasını
  dosya adına bağlar (000'dan başlar, idare istedikçe artar). Komut geçmişi değişikliğin ne olduğunu söyler, revizyon
  kaydının yerini tutmaz.

## Karar

- **Özet** (`apps/desktop/src/history/summary.rs`): 18 Mimari komutu girdisinin JSON'undan bir tasarımcının söyleyeceği
  gibi özetlenir. Örnekler:

  | Komut | Özet |
  |---|---|
  | Dikdörtgen duvar | 3.000 × 3.000 m · iç net · ortak duvar: var olan kullanılır |
  | Duvar | 9.000 m · kalınlık 0.200 m |
  | Duvarı düzenle | 0.300 m kaydırıldı · uzaklık 2.500 m · sol yan · boy 2.000 m · son uç sabit |
  | Ölçülü bölme | kaynaktan 1.500 m net · sol yan · kalınlık 0.100 m |
  | Açıklık | tipin adı (Pencere 120), tip yoksa biçimi (kapı) · uçtan 0.500 m |
  | Mahal | Z-01 Salon · net alan · duvarlardan bulundu |
  | Merdiven, rampa | genişlik · yükseklik · eğim |
  | Mutfak dizisi | 3.200 m · evye 2. modül · ocak 4. modül |
  | Hepsini etiketle | hepsi etiketlendi: mahaller, açıklıklar |

  - Kat, tip ve stil adları çizimden okunur; uzunluklar projenin biriminde yazılır.
  - Tanımadığı bir işlem İngilizce kalmaz, "değiştirildi" olur.
- **Satır:** başlık, özet (yoksa alanlar), sonuç ve uyarılar. Komut kimliği satırda yok.
- **Ayrıntı** (eski "Python" düğmesi): komut ve sürümü (`cad.walls.rectangle v1 · adım 1`), özetin dayandığı alanlar
  ("Girdi: …") ve Python kodu.
- **Alanların dökümü de Türkçe** (`history/text.rs`):
  - Mimari'nin gönderdiği 42 alan Türkçe adlandı: duvarlar, ortak duvar, çerçeve, hiza, boy, kaynak …
  - Seçim değerleri Türkçe yazılır: left → sol, clear → net, corners → köşeler, shift → kaydır …
  - Kimlik (`$uid…`, 36 karakterlik uuid) "1 nesne" olur.
  - Duvar komutlarında `width` Kalınlık'tır (ADR 8003).
  - Bir uzunluk olan `by` Miktar'dır, öbür komutlarda Yöntem kalır.

## Ölçüm

- **Testler** (`history/summary_tests.rs`, 6 durum; geçmiş testleri 17):
  - Komut fikstürlerindeki bütün Mimari girdileri (18 komut) özetlenir. Özette ve Ayrıntı'daki alanlarda ne kimlik ne
    İngilizce alan ya da seçim sözcüğü kalır.
  - Örnek özetler ve tip/biçim adları tutulur.
  - Testin bulduğu girdi biçimleri:
    - tipsiz cebe giren kapı;
    - yalnız tıklanan noktalı döşeme ve mahal;
    - sınırı çizilen mahal;
    - Hepsini etiketle;
    - tipe uygula, tipe döndür.
- **Gerçek pencere** (ADR 8000; `fixtures/interaction/real/komut-gecmisi.txt`, `.run/gui-evidence/mimari-m5/sonra/`):
  - Dikdörtgen duvar, kapı ve mahal çizildi.
  - Geçmiş "18.398 × 13.667 m · iç net · kalınlık 0.200 m · ortak duvar: var olan kullanılır", "kapı" ve "duvarlardan
    bulundu" yazdı.
  - Ayrıntı komutu, Türkçe alanları ve Python'u açtı.
- **Önce:** M4'ün gerçek pencere resmi `.run/gui-evidence/mimari-m4/sonra/m4-9-gecmis.png`: "Duvar oluştur
  cad.walls.create v1 · adım 1 / Katman: «Çizim» · walls: 1 öğe".
