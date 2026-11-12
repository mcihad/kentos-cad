# ADR 8053: Gerçek DWG projeleri açılır

- **Durum:** uygulandı (12 Kasım 2026). Sahibin bildirimi: "downloads içerisine bugün yeni dwg dosyaları indirdim gerçek
  projeler ama hiç birini açamıyoruz, bu ciddi bir sorun … oradaki tüm dwg dosyaları sağlıklı şekilde açılabilir olmalı";
  ardından "bu açtığımız dwg dosyaları da kcad olarak kaydedilemiyor bu durumları da düzeltmen gerek" (§12, §13).
- **Bağlam belgesi:** ADR 0586 (DWG kodeği, bellekte DXF köprüsü), ADR 0624 (DWG/DXF ayrı CAD çizimi olarak açılır),
  ADR 0726 §2 (birim sorusu), ADR 0258 §1 (kapalı çoklu çizgi), ADR 1001 §4 (çizgi tipleri).

## Bağlam

İndirilenler'deki 13 dosya masaüstünün Aç yoluyla (`cad_open::read_in`, birim cm verilerek) denendi. Kodek (acadrust,
OpenCADStudio'nun 22 Ekim kilidi) hepsini 0,0–3,1 s'de okuyor; reddeden KentOS'un kendi adaptörüydü:

| Dosya | DWG | Boyut | Önce |
|---|---|---|---|
| Proje A (mimari, 725 000 nesne) | AC1027 | 32,6 MB | ret: köprü DXF 270 MB, 256 MiB sınırı |
| Proje B (mimari, 7 346 çizgi tipi) | AC1021 | 32,6 MB | ret: 8 579 kurtarılan kayıt; köprü 463 MB |
| Proje C (mimari) | AC1021 | 8,5 MB | ret: eşit olmayan ölçekli blokta genişlikli yay |
| Proje D (mimari) | AC1027 | 28,2 MB | ret: 1 kurtarılan kayıt |
| sample1.dwg | — | 46 KB | "DWG imzası okunamadı desteklenmiyor" (dosya JPEG) |
| öbür 8 (R2004–R2018) | | 0,2–2 MB | açılıyordu |

Ret iletisi yalnız Geçmiş'e yazılıyordu: birim seçilip "Bu birimle aç" denince pencere kapanıyor, ekranda hiçbir şey
kalmıyordu.

Kurtarılan kayıtların hepsi çizilmeyen DGN çizgi stili bileşenleriydi (`LSSTROKEPATTERNCOMPONENT` …, MicroStation'dan
gelen çizgi tipleri). Proje B'nin 463 MB köprüsünün 380 MB'ı bunlar ve sözlükleriydi; köprünün yazılması 57 s sürüyordu.

## Karar

1. **Sonuna kadar okunan DWG açılır.** Kodeğin çözemediği kayıtlar ret değil, rapordaki satırdır: sayısı ve türleri
   ("Kodek bu kayıtları çözemedi; çizim onlarsız açıldı. Türleri (ilk 1000 tanı): LSSTROKEPATTERNCOMPONENT 999, DGN çizgi
   stili, çizilmez"). Sonuna kadar okunamayan akış yine reddedilir: yarım çizim açılmaz. KentOS'un yazdığı DWG'nin
   yeniden okunması (dışa aktarma doğrulaması) eskisi gibi katıdır.
2. **Okunmayan nesneler köprüye yazılmaz.** DGN çizgi stili kayıtları (çözülen ya da ham kalan) ve vekil (proxy)
   nesneler, sözlüklerdeki girişleriyle, geçici belgeden DXF yazılmadan çıkarılır; kaynak DWG değişmez. Raporda "DWG
   nesnesi" notu.
3. **Köprünün kendi sınırı 1 GiB.** DWG dosyasının 256 MiB sınırı kalır; köprü kullanıcının seçtiği bir dosya değildir ve
   DWG'nin 8–14 katı tutar. DXF okuyucusu sınırı parametre olarak alır (`read_checked_watched_within`); DXF dosyalarının
   256 MiB sınırı değişmedi.
4. **Aç, kayıpsız taşınamayan çoklu çizgiyi yaklaşık açar ve söyler** (`allowPolylineLoss`): eşit olmayan ölçekli
   bloktaki yay noktalara bölünür, genişlik eksenle gelir; notlar Geçmiş'e yazılır. İçe aktar'ın seçimi değişmedi:
   kaybı kabul edilmedikçe reddeder.
5. **Açılmayan dosya ekranda söylenir:** "Çizim açılamadı" penceresi dosyanın adını ve gerekçesini gösterir; Tamam ya
   da Esc kapatır; açık çizim olduğu gibi kalır.
6. **DWG olmayan dosya ne olduğunu söyler:** JPEG, PNG, GIF, PDF, ZIP, RAR, 7z ve DXF tanınır ("adı .dwg ile bitse de
   içinde bir JPEG resmi var"); desteklenmeyen sürüm imzasıyla adlandırılır ("Bu DWG'nin sürümü (AC1009) desteklenmiyor").
7. **Üç yavaşlık giderildi:**
   - DXF okuyucusu her nesnenin çizgi tipini tablonun hepsini büyük harfe çevirerek arıyordu: nesne başına 45 µs,
     25 s'nin 22'si. Ad dizini okumanın başında bir kez kurulur.
   - Açılışta çizgi tipleri projeye her biri için bütün tablo kopyalanıp yeniden doğrulanarak ekleniyordu (O(n³)):
     122 s'nin 99'u. Projenin tablosu bir kez doğrulanır, adlar kümede aranır, her tip kendi başına doğrulanır; sonuç
     aynıdır.
   - Açılış dosyayı iki kez okur (birim, sonra ölçeğiyle); DWG ikinci kez de çözülüyordu. Köprü bir kez üretilir
     (`kentos_dwg::bridge`), iki okuma onu kullanır (`read_bridge`): Proje B 24,2 → 14,7 s.
8. **Kodeğin teknik tanıları Geçmiş'e dökülmez:** DGN çizgi stilininkiler özet satırında sayılır, öbürlerinden en çok
   10'u yazılır (önce 100 satır).
9. **CAD çizimi yoğun kısmında açılır.** Üç gerçek projede birkaç nesne çizimden 3–5 çizim boyu uzaktaydı; bütün çizime
   sığan ilk görünümde 32 MB'lık plan birkaç piksellik bir şerit kalıyordu. Uzaktaki nesne kuralı (`strays`, 20 kat
   uzaklık; CBS'nin (0, 0)'a düşen nesneleri için) onları yakalamıyor. İlk görünüm, merkezi yüzde 2–98 aralığının
   (dörtte biri kadar genişletilmiş) içinde olan nesnelerin tam kutularıdır; çizim bunun 3 katından geniş ya da yüksek
   değilse bütün çizimdir. Uzakta kalan nesne sayısı "Açılış görünümü" notudur; Tümünü göster hepsini gösterir.
   Dosyanın kaydettiği etkin görünüm (VPORT *Active) okunmuyor: merkezinin dünya koordinatı görünüm hedefi ve dönüşü
   ister, ayrı iştir.

10. **Kodek, OpenCADStudio'nun bugünkü kilidi** (sahibin seçimi, 12 Kasım 2026). `HakanSeven12/opencadcodec` çatalı
    GitHub'dan silinmişti ("Repository not found"); derleme yalnız Cargo önbelleğinden yürüyordu, temiz makinede
    kırılırdı. Kodek `hakanaktt/acadrust` 0.6.8 @ `93449222904e9df1ba14e275b6b764df93227b33` (geliştiricisinin deposu;
    595 commit ileride; 29 Ekim'de DGN "stroke pattern" okuma düzeltmesi; MPL-2.0; derlemede kod çalıştıran build.rs ya
    da proc-macro yok; yeni bağımlılık yok, rayon ve crossbeam düştü). API değişmedi. Bu revizyonla kurtarılan kayıt
    kalmadı (Proje B: 8 579 → 0, Proje D: 1 → 0) ve bellek düştü; çözme ve köprü yazımı yavaşladı (yazıcı belgeyi
    kopyalayıp onarım geçişleri yapıyor, seçeneği yok).

11. **Emperyal birim bildiren dosyada Aç birimi sorar** (sahibin seçimi, 12 Kasım 2026). Proje C birimini inç
    bildiriyordu ve ADR 0624'ün kuralıyla (bildirilen birim geçerli) inç olarak açıldı; Türkiye'deki projelerde inç ve ft
    çoğunlukla bir şablondan kalır. Dosya inç, ft, yard, mil ya da ABD ft bildirirse "Çizimin birimini doğrula" penceresi
    açılır ("Dosya uzunluk birimini inç olarak bildiriyor …"), dosyanın birimi seçili gelir, önizleme her seçimin model
    boyutunu söyler. Dosyanın birimi onaylanırsa okunan çizim yeniden okunmadan açılır; başka birim seçilirse o birimle
    okunur ve bir daha sorulmaz. Metrik bildiren dosya ve İçe aktar değişmedi.

12. **Açılan DWG KCAD olarak kaydedilir.** 12 dosyanın 5'i (Proje A–D ve bir küçük proje) Kaydet'te "Renk nesnenin
    kendi değeri; katmana göre olamaz" ile reddediliyordu. Sebep okuyucudaydı: açılan bir bloğun (dinamik bloğun adsız
    tanımı her zaman açılır) bütün görünüşü nesnelerinin üstüne yazılıyordu; yerleştirme BYLAYER iken nesneye
    `byLayer` rengi işleniyor, nesnenin kendi rengi de duruyordu (BYBLOCK, yerleştirmenin rengi olarak yazılmıştı). KCAD
    sözleşmesi (ADR 0260 §1: değeri olan parça `byLayer`'da olmaz) bunu doğru olarak reddediyordu. Okuyucu artık kendi
    rengi ya da kalınlığı olan nesneye o parçayı katmana göre işlemez (§13 ile bu yalnız açılan yerleştirmenin
    öznitelikleri için kalır); kendi değer zaten önce çizildiğinden çizim değişmez. Ret iletisindeki çift nokta
    ("olamaz.. Değeri") tek noktaya indi.
13. **Açılan bloğun nesneleri yerleştirmeden yalnız bloğa göre parçalarını alır** (ADR 0315 §2'nin iç içe kuralı).
    Önce yerleştirmenin bütün görünüşü (çizgi tipi, desen, desen ölçeği, saydamlık, katmana göre parçalar) nesnelerin
    görünüşünün üstüne yazılıyordu: iç içe bloktaki kesikli çizgi yerleştirmenin çizgi tipini alıyor, BYLAYER nesne
    yerleştirmenin kendi rengi varken bloğa göre kalıyordu. Şimdi yerleştirmenin görünüşü okuma bağlamıyla iner
    (`Ctx::byblock_look`), her nesne kendi görünüşünü okunurken kurar:
    - kendi değeri: kendisinin; BYLAYER: kendi katmanının (0'daysa yerleştirmenin katmanının);
    - BYBLOCK: yerleştirmenin kendi değeri; yerleştirmeninki katmanına göreyse 0'daki nesne de katmana göre olur (o
      katmandadır), kendi katmanındaki nesne o katmanın değerini yazılı alır, tanımın 0'ındaki yerleştirmeninkini
      alamayınca bloğa göre kalır (en dıştaki yerleştirmenin değeri);
    - tanımın 0'ında BYLAYER ya da tanımın tepesinde BYBLOCK olan yerleştirmenin BYBLOCK nesnesi artık ön plan rengini
      (`ink`) değil, tanımı çizen yerleştirmenin (ya da katmanının) rengini alır;
    - desen ölçeği: nesnenin kendisi, yoksa yerleştirmeninki (önceki gibi).

    Yerleştirmenin bütün görünüşü yalnız onun açılmadan sonra çizdiklerine (öznitelikler) yazılır.

## Ölçüm

Aynı 13 dosya, Aç yolu (`cad_open::read_in`, cm), sürüm derlemesi, 20 çekirdek, 23 GB:

| Dosya | Önce | Sonra (eski kodek) | Sonra (yeni kodek) | En çok bellek (eski → yeni) |
|---|---|---|---|---|
| Proje B | ret | **14,7 s** | **21,3 s** | 3,9 → 3,6 GB |
| Proje A | ret | **15,0 s** | **18,5 s** | 3,8 → 3,1 GB |
| Proje D | ret | **12,9 s** | **16,2 s** | 3,4 → 2,7 GB |
| Proje C | ret | **3,4 s** | **3,6 s** | 1,1 → 0,9 GB |
| öbür 8 | 0,1–0,8 s | 0,1–0,8 s | 0,1–0,8 s | |
| sample1.dwg (JPEG) | anlamsız ileti | "içinde bir JPEG resmi var" | aynı | |

Proje B'nin köprüsü 463 → 78 MB, yazımı 57 → 5,9 s.

Kaydet (KCAD, `encode_verified`: yazılır, geri okunur, karşılaştırılır), aynı 12 DWG:

| | Önce | Sonra | Boyut (DWG → KCAD) | Yazma |
|---|---|---|---|---|
| Proje B | ret | kaydedildi | 32,6 → 118,5 MB | 1,6 s |
| Proje A | ret | kaydedildi | 32,6 → 108,3 MB | 1,4 s |
| Proje D | ret | kaydedildi | 28,2 → 95,5 MB | 1,2 s |
| Proje C | ret | kaydedildi | 8,5 → 30,5 MB | 0,4 s |
| küçük bir statik projesi | ret | kaydedildi | 2,1 MB | 0,0 s |
| öbür 7 | kaydediliyordu | kaydedildi | 0,6–8,0 MB | 0,0–0,1 s |

Üstüne yazılan görünüş (§13; bir okumada, iç içe her düzey ayrı sayıldı): Proje B'de nesnenin kendi çizgi tipi
528 040 kez yerleştirmeninkiyle değiştiriliyordu, BYLAYER renk 65 kez bloğa göreye düşüyordu; Proje C'de 513 ve 3.
Değişimlerin çoğu sürekli çizginin yerine yine sürekli çizgi koyuyordu; görünen fark kesikli, noktalı ya da adlı
desenli çizilen nesne sayısında:

| Dosya | Çizimde (önce → sonra) | Blok tanımlarında (önce → sonra) |
|---|---|---|
| Proje C | 5 046 → **4 673**: 373 BYLAYER nesne yerleştirmenin kesikli çizgisini alıyordu | 261 → 261 |
| Proje B | 3 647 → 3 647 | 11 739 → **11 779**: 40 nesnenin kendi deseni geri geldi |
| Proje A, D ve öbür 8 | aynı | aynı | İlk görünüm (genişlik × yükseklik, çizim birimi): Proje C
693 076 × 225 007 → 160 120 × 9 676, Proje A 1 225 385 × 2 746 062 → 365 829 × 34 050; Proje B'ninki değişmedi.

## Açık kalanlar

- Açılış köprüyü yine iki kez okur (birim, sonra ölçeğiyle); büyük dosyada sürenin yaklaşık yarısı.
- Bellekte ASCII DXF köprüsü (bkz. ADR 0586) büyük çizimde 3,4–4,7 GB tutuyor; kodeğin belgesinden doğrudan KentOS
  nesnelerine geçiş köprüyü kaldırır.
- Gerçek dosyalar ve adları depoya girmez (kişisel projeler; depo herkese açık): burada Proje A–D. Kabul yerelde,
  İndirilenler'deki dosyalarla yapıldı.
- KCAD sıkıştırılmıyor: 33 MB'lık DWG 95–119 MB KCAD oluyor (`FILE-10/11`, TODOS).
- Model alanında açılan yerleştirmenin çizgi tipi BYLAYER ise kendi katmanındaki BYBLOCK nesnesi o katmanın çizgi tipini
  alır, yerleştirmenin katmanınınkini almaz (tanımın içinde alır).

## Doğrulama

- `cargo test -p kentos-dwg`: kurtarılan kayıtla açılış, yarım akışın reddi, yazma doğrulamasının katılığı; DGN çizgi
  stili kaydının sözlük girişiyle çıkması; kayıt türlerinin söylenmesi; DWG olmayan dosyaların iletileri.
- `cargo test -p kentos-native-application --lib cad_open import_apply`: Aç kayıplı çoklu çizgiyi açar ve söyler, İçe
  aktar'ın kuralı reddeder; 3 000 çizgi tipi bir kez ve tablo kurallarıyla alınır.
- `cargo test -p kentos-desktop --lib opening`: açılmayan dosya ekranda kalır, Tamam kapatır, çizim değişmez; inç
  bildiren dosya inç seçili sorulur, onaylanınca okunduğu gibi, cm seçilince yeniden okunarak açılır.
- `cargo test -p kentos-formats --lib import`: birkaç çizim uzaktaki nesneler ilk görünümün dışında kalır ve sayılır;
  düzgün dağılan çizim bütün açılır.
- `cargo test -p kentos-formats --test all exploded_inserts`: tanımın içinde açılan adsız blokta kendi rengi olan
  hiçbir parça katmana göre değil, özniteliğin kendi rengi korunur (§12); çizimde ve tanımda açılan yerleştirmenin
  nesneleri kendi kesikli çizgisini ve BYLAYER'ını korur, yalnız BYBLOCK parçaları yerleştirmeden alır (§13). İkisi de
  önceki kodda kırmızıydı.
- `cargo test -p kentos-kcad --test all look`: ret iletisinde tek nokta.
- `KENTOS_SAVE_KCAD=1 cargo run --release -p kentos-native-application --example open_cad -- DOSYA.dwg`: Aç ve
  Kaydet'in kodlaması; 12 DWG'nin 12'si kaydedildi.
