# ADR 9118: Genel sembol kitaplığı

- **Durum:** kabul edildi (2026-11-22).
- **İstek:** sahibin sözü (22 Kasım 2026): “bizim stil ve semboloji motorumuz çok güzel senden istediğim şey sembol
  kütüphanesini zenginleştirmek, hem nokta hem çizgi hem de alan sembolleri kategorize ederek çok kaliteli bir sembol
  kütüpanesi oluşturmanı istiyorum, şu anda mekansal plan yapım yönetmeliği sembolleri var ama senden her türden
  kullanılabilecek genel semboller oluşturmanı istiyorum.”
- **Web:** ertelendi (ADR 0228). Masaüstü yeni dosyayı okur; web'in sistem kitaplığına eklenmesi TODOS.md §0.2'de.
- **`.kcad`:** biçim değişmedi. **Komut:** yok; kitaplık veridir. **Bağımlılık:** yeni dış kitaplık yok.

## §1 Bağlam ve araştırma

**Önce:** sistem kitaplığında iki dal vardı. Temel'de 22 öğe vardı: çizgi tipleri, birkaç işaret ve tarama. MPYY'de
754 öğe vardı: yönetmeliğin 695 sembolü ve 81 piktogram (docs/STYLE.md §8). Her türden harita için genel semboller,
örneğin durak, okul, orman, demiryolu, altyapı hattı ya da tarama deseni yoktu.

**Masaüstü CBS'nin kendi kitaplığı** (QGIS 4.2.2 kurulumundaki `symbology-style.xml`, 22 Kasım 2026'da okundu):

- 118 sembol: 52 alan, 36 çizgi, 30 nokta.
- Etiketler: Colorful (59), Grayscale (32), Topography (19: havalimanı, bina, orman, hastane, demiryolu, bataklık …),
  Showcase (15).
- Kılavuz, *The Style Manager* §10.1 (3.44): öğeler etiketlerle (bir öğe birden çok etiket alabilir), akıllı
  gruplarla ve sık kullanılanlarla düzenlenir.

**Öbür masaüstü CBS** (ArcGIS Pro, *Styles*, 22 Kasım 2026'da okundu):

- Kurulumla gelen sistem stilleri salt okunurdur. Öğeleri kişinin kendi stiline ya da sık kullanılanlarına
  kopyalanıp değiştirilir.
- ArcGIS 2D, ArcGIS 3D, ArcGIS Colors ve ColorBrewer her yeni projeye eklenir.
- Bütün sistem stillerinde arama yapılabilir.

**BÖHHBÜY Ek-3** (RG 26.06.2018/30460 Mük., yerel PDF'in 89. sayfasından görüldü): büyük ölçekli haritaların işaret
kataloğu. Her işaretin sıra numarası, tipi ve ölçeğe göre çizimi vardır. Bu, MPYY gibi kendi başına bir kitaplıktır
(§7).

**Renk:** temel şekiller için Okabe ve Ito'nun (2008) renk körlüğüne uygun sekiz renklik paleti alındı.

**Ölçüt:** KentOS'un kitaplığında ağaç (kategori yolu), etiket (arama), salt okunur sistem kaynağı ve kopyala-düzenle
zaten var. Eksik olan içerikti. Masaüstü CBS'nin kendi kitaplığından daha kapsamlı olmalı, temalara ayrılmalı ve
Türkçe adlarla aranabilmeliydi.

## §2 Karar: içerik ve düzen

**Genel** adlı yeni dal, Temel ve MPYY'nin yanında: 197 sembol ve 56 piktogram.

| Dal | Kategori (sembol) |
|---|---|
| Noktalar (87) | Temel şekiller 12, Ulaşım 11, Sağlık ve acil durum 6, Eğitim ve kültür 5, Kamu hizmetleri 5, İbadet ve anıt 6, Ticaret ve turizm 9, Spor ve rekreasyon 5, Altyapı 11, Ölçme 7, Doğa 7, Uyarı 3 |
| Çizgiler (59) | Yollar 13, Raylı sistem 5, Sınırlar 10, Su 7, Altyapı hatları 9, Topoğrafya ve yapı 8, Oklar ve genel 7 |
| Alanlar (51) | Arazi örtüsü 12, Su ve sulak alan 5, Yapılaşma 5, Kentsel kullanım 11, Tarama desenleri 12, Kısıt ve tehlike 6 |

**Kurallar:**

- **Kimlik:** `genel.<nokta|cizgi|alan>.<grup>.<ad>`. Piktogramlar `genel.svg.<ad>`'dır, “Genel › Piktogramlar”da durur.
- **Ölçü:** kâğıt milimetresidir, kitaplığın geri kalanı gibi. `ink` ve `paper` temanın çizim ve zemin rengidir.
- **Yer sembolleri:** temasının renkli dairesinde beyaz piktogramdır, beyaz kenarlıdır. Her zeminde okunur.
  - Renkler: ulaşım mavi, sağlık kırmızı, eğitim turuncu, kültür mor, kamu gri mavi, ticaret pembe, turizm yeşil mavi,
    spor yeşil.
  - Uluslararası alışkanlık harfle yazılır: H hastane, P otopark, M metro, i bilgi.
- **Teknik semboller kendi renklerindedir:** altyapı noktaları, ölçme noktaları ve doğa. Altyapının renkleri: elektrik
  kırmızı, doğal gaz turuncu, içme suyu mavi, atık su kahve, yağmur suyu camgöbeği, haberleşme yeşil, petrol koyu gri.
  Bu bir yönetmelik eşlemesi değil, KentOS'un varsayılanıdır.
- **Çizgiler:** yollar kademesine göre kenarlıdır (otoyol, devlet yolu, il yolu, cadde, sokak). Altyapı hatlarının
  üstünde harfleri yazar (G, S, Y, T, P). Şev uzun ve kısa tarakla, köprü dört kanatla çizilir.
- **Alanlar:** orman, çalı, çayır ve kayalık kendi piktogramlarının dağınık deseniyle doldurulur (`jitter`, sabit
  `seed`). Taramalar mürekkepledir. Kısıt alanlarının kenarı içe taraklıdır.
- **Etiketler:** Türkçe arama sözcükleri (“ulaşım”, “altyapı”, “orman” …).
- **Piktogramlar KentOS için elle çizildi:** viewBox 0 0 100 100, `currentColor`, delikler `evenodd`. Başka bir setten
  alınmadı. 3 mm'de okunacak kalınlıktadır.

## §3 Karar: kaynak ve yükleme

- **Kaynak:** `scripts/style/general_library.py`. Betik `crates/native/style/assets/general-library.kstil`'i yazar,
  biçimi masaüstünün stil dosyasıdır. `--check` dosya betikten geri kaldığında düşer. `--svg` piktogramları gözden
  geçirmek için döker.
- **Neden TypeScript değil:** öbür sistem kitaplığının (Temel, MPYY) kaynağı web'in TypeScript'idir. Web dururken yeni
  içerik oraya yazılmadı. Ayrı bir dosya, web'in mevcut denetimini (`system-library.test.ts`) bozmaz.
- **Yükleme:** `kentos_native_style::system` iki dosyayı da okur. Öğeleri ve kategorileri tek sistem kaynağında
  birleştirir: Stil yöneticisi, katman stili, nesneye sembol ve lejant hepsini görür. Öğeler salt okunurdur ve
  kopyalanabilir.

## §4 Karar: dayanak noktalı işaretin önizlemesi

- Konum işareti ucunun, bayrak direğinin dibinde durur (`anchor`). Önizlemenin sığdırma ölçüsü (`marker_reach`) dayanak
  noktasını saymıyordu. Böyle işaretler kartta kesiliyordu; MPYY'de dayanaklı işaret olmadığı için görünmemişti.
- Artık işaretin ortasının noktaya uzaklığı da sayılır. Sınaması: `preview::tests::an_anchored_point_symbol_is_fitted_whole`.

## §5 Motorun iki davranışı (yazarken görülen)

- **İşaretin kendi döndürmesi kaydırmasını da döndürür.** Çizgi boyunca dik duran bir tarağı yana kaydırmak için
  kaydırma döndürmeden önceki çerçevede verilir. Betikte bunu `beside()` hesaplar.
- **Alan kenarında olumlu kaydırma içe doğrudur,** Temel'in “İçe tırnaklı kenar”ında olduğu gibi.

## §6 Kanıt

- **Kitaplık denetimi** (`crates/native/style/tests/all/general_library.rs`):
  - her sembolün kimliği türünü söyler, kategorisi ve üst kategorileri listededir;
  - her katman o türün çizdiği bir katmandır;
  - her şekil gölgelendiricinin çizdiğidir (bilinmeyen şekil daire çizilirdi);
  - her yazı yüzü tanınır, her piktogram kitaplıktadır;
  - her sembol bir önizleme çizer, her kategoride sembol vardır;
  - her piktogram masaüstünün okuduğu bir vektör çizimidir.
- **Sistem kitaplığı:** `system::tests` yeni toplamı ve bir genel sembolü arar. Masaüstünün stil, lejant, modül, şerit,
  geçmiş, katalog ve ayar sınamaları geçer.
- **Resimler** (`.run/gui-evidence/genel-kitaplik`, `general_screens` sınamaları; açık ve koyu tema, 106 resim):
  - Stil yöneticisi her kategoride açık, ilk sembol seçili;
  - her kategori 1:1000'de bir katalog sayfası, her sembol örnek nesnesinde adıyla;
  - yalnız yeni sembollerle çizilmiş bir kasaba: göl, orman, tarla, bağ, kayalık tepe, yollar, demiryolu, enerji
    hattı, binalar, yerler.
- **Resimlerden düzeltilenler:**
  - top tekerleğe, çim oka benziyordu, dikilitaş incecikti;
  - kayanın çatlağı görünmüyordu;
  - demiryolunun beyaz aralığı koyu temada kayboluyordu, artık `paper`'dır;
  - şevin tarağı ortadaydı, köprünün kanatları tek yandaydı;
  - yaya yolunun noktaları birbirine karışıyordu;
  - koruma ve yasak alanının tarağı dışa düşüyordu;
  - konum işareti kartta kesiliyordu (§4).

## §7 Kalan

- **BÖHHBÜY Ek-3 kitaplığı:** büyük ölçekli harita işaretleri, numaralarıyla ve ölçeğe göre çizimleriyle, MPYY gibi ayrı
  bir dal olarak.
- **Web'de genel kitaplık** (TODOS.md §0.2).
- **Sahibin kullanımından gelecek istekler:** yeni piktogramlar (cemevi, tiyatro, ATM ayrı), renk çeşitleri.
