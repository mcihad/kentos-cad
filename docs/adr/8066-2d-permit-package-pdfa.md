# ADR 8066: 2B ruhsat paketi — paket kuralı Python'da, teslim listesi, bağımsız doğrulama

- **Durum:** uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-05; uygulama sırasında B5. İlk kabul
  sağlandı (§7). Eksik kalanlar §8'de.
- **Bağlam:** ADR [8037](8037-turkish-title-block-sheet-sets-pdfa.md) (Ruhsat takımı, PDF/A-2b, pafta başına dosya, EK-2
  adı, yazıcının kendi PDF/A denetimi), [1090](1090-general-2d-sheet-set-and-publication-flow.md) (yayın, `yayin.json`,
  atomik yazım), [8065](8065-sheets-from-the-model-gate.md) (ruhsat yayınının kapısı), [8062](8062-digital-project-rule-catalog.md)
  (K13: PDF/A'nın bölümü ve düzeyi).
- **Dayanak:** Hazırlama Yönetmeliği Md. 4/2 (2B çizim dosyaları PDF/A), Md. 6 ve EK-2 (dosya adları; Tablo 2.1, 2.1.1),
  Md. 8 (A4 ve katları; her çizim ayrı dosya); e-PYS Md. 6/4 (taranmış PDF kullanılmaz).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** yayının girdisi değişmedi; `yayin.json`'a
  isteğe bağlı `deliveryList` alanı eklendi. **Bağımlılık:** dış bağımlılık yok; bağımsız okuma qpdf ve Poppler'ı kullanır
  (yalnız betikte, ürünle dağıtılmaz). veraPDF bu bilgisayarda yok.

## §1. Karar

1. **Bugün var olan korunur:** PDF/A-2b, pafta başına dosya, EK-2 adı, künye; yazıcı her dosyayı teslimden önce kendi
   PDF/A-2 okumasıyla (`kentos_formats::pdf::pdfa::pdfa2_report`, madde madde) okur ve bozuk kural yayını durdurur; yayın
   geçici klasöre yazılıp tek adımda yerine taşınır (yarım paket kalmaz, önceki paket durur; ADR 1090).
2. **Paket kuralı Python'dadır** (`dijital_proje.paket`, paket 0.4.0; bim-plan K10): PDF/A seçilmemiş, bir dosyada birden
   çok pafta, A4 ve katları dışında kâğıt, EK-2'ye uymayan dosya adı. Bulguların hepsi hatadır. Olgular denetimin
   raporundandır: yazılacak dosyalar ve paftaları, paftaların kâğıdı, PDF/A ve dosya biçimi.
3. **Ruhsat yayınının kapısı iki kuralı birlikte sorar** (`dijital_proje.ruhsat` = `turetim` + `paket`): Denetle'nin raporu
   gelince sorulur, raporun dosya listesiyle.
4. **Teslim listesi:** PDF/A yayını klasörüne `teslim-listesi.csv` yazar (UTF-8, `;`): sıra, dosya, SHA-256, bayt, pafta
   numarası ve adı, paftanın revizyonu, üretildiği model revizyonu ve modelin özeti. `yayin.json` onu `deliveryList` ile
   adlandırır.
5. **Bağımsız doğrulama** üretilen baytlarda, KentOS kodu olmadan: `scripts/mevzuat/pdfa_bagimsiz.py` (qpdf ve Poppler).

## §2. Paket kuralı

| Kural | Kod | Dayanak | Ne |
|---|---|---|---|
| PDF/A | `digital_package_pdfa` | Md. 4/2; K13 | Yayında PDF/A seçilmemiş |
| Bir dosya, bir pafta | `digital_package_one_sheet` | Md. 8 | Bir dosyada birden çok pafta |
| Kâğıt | `digital_package_paper` | Md. 8; K39 | Kenarlardan biri 210 ya da 297 mm'nin katı değil (±1,5 mm) ya da kâğıt A4'ten küçük |
| Dosya adı | `digital_package_name` | Md. 6; EK-2 2.1, 2.1.1, Tablo 2.1 | Dokuz alan "_" ile değil; alanın uzunluğu ya da türü tablodaki gibi değil; disiplin 2.2'de, dosya türü 2.12'de, dosya tipi 2.13'te yok |

Kural verisi (`fileTemplate`, `fileKinds`, `fileTypes`) katalogdan `scripts/mevzuat/ek34_veri.py` ile üretilir.

## §3. KentOS'un kendi çıktısında bulunan

Paket kuralı ilk çalıştığında kesit paftasının adını reddetti: `000000_00_MM_XXXX_KS_MMC_B-B_01_000.pdf`. ADR 8042 tek
harfli kesit işaretinin konumunu "B-B" yazıyordu; Tablo 2.1 bu alanı üç harf ya da rakam ister, tire ikisi de değildir.
Konum artık "BBX"tir (`permit_set::section_place`; harfi iki kez, X ile üçe tamamlanır, iki harfli işaretteki kuralla).
ADR 8042 §6'ya ek yazıldı.

## §4. Bağımsız doğrulama

`pdfa_bagimsiz.py <teslim klasörü>` her PDF'i qpdf ve Poppler ile okur: yapı (`qpdf --check`), bir sayfa ve A4'ün katı
kâğıt (`pdfinfo`), XMP'de PDF/A-2b kimliği (`pdfinfo -meta`), gömülü yazı tipleri (`pdffonts`), taranmış sayfa (yazısız ve
yazı tipsiz, yalnız resim: `pdfimages`, `pdftotext`), EK-2 Tablo 2.1'in dokuz alanı; teslim listesinin her dosyayı bir kez
saydığı ve SHA-256'ların tuttuğu. Disiplin kodunu ve tabloları okumaz: o kural paketin testindedir.

| Paket | Dosya | Geçen | Liste |
|---|---|---|---|
| Kesit binası (`fixtures/arch/v1/section-building.kcad`) | 7 | 7 | tutuyor |
| 2+1 daire (fare ve klavye izi) | 5 | 5 | tutuyor |
| Bilerek bozulan | 3 | 0 | yok |

Bozulanlar: adı EK-2 olmayan dosya ("ad EK-2 Tablo 2.1'in dokuz alanı değil"); iki paftanın birleştirildiği dosya ("2 sayfa",
"XMP'de PDF/A-2b kimliği yok"); yazı tipi gömülmemiş, başka yerde yapılmış bir sayfa ("gömülmemiş yazı tipi: Helvetica",
"XMP'de PDF/A-2b kimliği yok"). Paketler `.run/gui-evidence/dijital-paket-8066/`'da.

## §5. Masaüstü

Kapı (`permit_gate.rs`) Denetle'de beklemeye geçer, raporun gelişiyle paket olgusunu kurup `dijital_proje.ruhsat`'ı sorar.
Pencerenin sonundaki bölüm "Ruhsat kapısı: paftalar modelden, paket yönetmelikte (Md. 4/2, 6, 8, 9/1)" adını taşır; açıkken
"dosyalar PDF/A, her pafta ayrı dosyada, kâğıtlar A4 ve katları, adlar EK-2'ye uygun" der.

## §6. Sınamalar ve kanıt

- **Paket** (`tests/test_paket.py`, 6): her kural için bir geçen, bir kalan durum; altı A kâğıdı geçer, üç başka kâğıt
  kalır; altı yanlış ad; `ruhsat`'ın iki kuralı birlikte vermesi.
- **`kentos-headless`:** yayın sınamaları (16) geçiyor; teslim listesi PDF/A yayınında yazılır.
- **`kentos-native-application`:** kesit paftasının konumu "BBX" (`section_places_follow_ek_2`, `sections`).
- **Masaüstü** (`permit_package_tests`): kesit binasının ruhsat yayınında kapı açık, denetim hazır; yayın yapılınca teslim
  listesi klasördeki her PDF'i bir kez sayar, SHA-256'lar dosyalarınki, `yayin.json` listeyi adlandırır. Adlandırma
  "{index}-{sheet}" olunca her dosya "EK-2 adı değil"; tek dosya seçilince "pafta var: her çizim ayrı dosyadır"; PDF/A
  kapatılınca "PDF/A seçilmemiş". Elle çalıştırılan bağımsız sınama §4'ün tablosunu verir.
- **Resimler:** `.run/gui-evidence/dijital-paket-8066/` (açık ve koyu): kapı açık, ad EK-2 değil, yayınlandı.

## §7. Kabul (plan: BIM-05 ilk kabul)

| Ölçüt | Durum |
|---|---|
| 2+1'in ve bölüm binasının paketi bağımsız doğrulayıcıdan hatasız geçer | geçer (5 ve 7 dosya; qpdf ve Poppler) |
| Bilerek bozulan dosya paketi durdurur ve sebebini söyler | yanlış ad ve tek dosya kapıyı kapatır; bağımsız okuma yanlış adı, iki sayfayı ve gömülmemiş yazı tipini sebebiyle reddeder |
| Teslim listesi | var: dosya, özeti, pafta numarası, revizyonu, model revizyonu |
| Atomik yayın | var (ADR 1090) |

## §8. Eksik kalanlar

- **Çizim dışı belgeler** (rapor, hesap) pakete henüz eklenmiyor: yayının girdisine bir belge listesi gerekir; bu sözleşme
  değişikliği web'in donmuş sözleşme dosyalarına ve Python SDK'sının üretilen katmanına dokunur, ayrı bir adımdır. Belgenin
  olguları KentOS'un PDF okuyucusundan (`pdfa2_report`, `read_page`) gelecek, kural `paket`'tedir.
- **veraPDF:** bağımsız PDF/A doğrulayıcı bu bilgisayarda yok; qpdf ve Poppler PDF/A'nın bütününü değil, kimliği, yazı
  tiplerini ve yapıyı okur.
- **Taranmış sayfa** kapıda değil, bağımsız okumada denetlenir: yazıcı vektör çizer; yalnız resim taşıyan bir pafta kapıda
  ayrıca söylenmiyor.
- **Teslim listesinin "Revizyon" sütunu** paftanın kendi revizyonudur; çoğu ruhsat paftasında boştur, revizyon dosya adındadır.
- **PDF/A'nın bölümü** kurum profilinden okunmuyor: kural PDF/A'nın seçildiğine bakar; yazıcı PDF/A-2b yazar (K13).
