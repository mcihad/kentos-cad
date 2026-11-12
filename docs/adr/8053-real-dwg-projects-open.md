# ADR 8053: Gerçek DWG projeleri açılır

- **Durum:** uygulandı (12 Kasım 2026). Sahibin bildirimi: "downloads içerisine bugün yeni dwg dosyaları indirdim gerçek
  projeler ama hiç birini açamıyoruz, bu ciddi bir sorun … oradaki tüm dwg dosyaları sağlıklı şekilde açılabilir olmalı".
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

## Ölçüm

Aynı 13 dosya, Aç yolu (`cad_open::read_in`, cm), sürüm derlemesi, 20 çekirdek, 23 GB:

| Dosya | Önce | Sonra | En çok bellek |
|---|---|---|---|
| Proje B | ret | **14,7 s** | 3,9 GB |
| Proje A | ret | **15,0 s** | 3,8 GB |
| Proje D | ret | **12,9 s** | 3,4 GB |
| Proje C | ret | **3,4 s** | 1,1 GB |
| öbür 8 | 0,1–0,8 s | 0,1–0,8 s | |
| sample1.dwg (JPEG) | anlamsız ileti | "içinde bir JPEG resmi var" | |

Proje B'nin köprüsü 463 → 78 MB, yazımı 57 → 5,9 s. İlk görünüm (genişlik × yükseklik, çizim birimi): Proje C
693 076 × 225 007 → 160 120 × 9 676, Proje A 1 225 385 × 2 746 062 → 365 829 × 34 050; Proje B'ninki değişmedi.

## Açık kalanlar

- **Kodek deposu silindi.** `HakanSeven12/opencadcodec` GitHub'da yok ("Repository not found"); derleme yerel Cargo
  önbelleğinden yürüyor, temiz makinede kırılır. OpenCADStudio bugün `hakanaktt/acadrust` 0.6.8 @ `9344922` kullanıyor
  (595 commit ileride; 29 Ekim'de DGN "stroke pattern" okuma düzeltmesi). Geçiş bu ADR'de yapılmadı: yeni kodu
  derlemek sahibin onayını bekliyor (ADR 0586'nın güncelleme yolu).
- Açılış köprüyü yine iki kez okur (birim, sonra ölçeğiyle); büyük dosyada sürenin yaklaşık yarısı.
- Bellekte ASCII DXF köprüsü (bkz. ADR 0586) büyük çizimde 3,4–4,7 GB tutuyor; kodeğin belgesinden doğrudan KentOS
  nesnelerine geçiş köprüyü kaldırır.
- Gerçek dosyalar ve adları depoya girmez (kişisel projeler; depo herkese açık): burada Proje A–D. Kabul yerelde,
  İndirilenler'deki dosyalarla yapıldı.

## Doğrulama

- `cargo test -p kentos-dwg`: kurtarılan kayıtla açılış, yarım akışın reddi, yazma doğrulamasının katılığı; DGN çizgi
  stili kaydının sözlük girişiyle çıkması; kayıt türlerinin söylenmesi; DWG olmayan dosyaların iletileri.
- `cargo test -p kentos-native-application --lib cad_open import_apply`: Aç kayıplı çoklu çizgiyi açar ve söyler, İçe
  aktar'ın kuralı reddeder; 3 000 çizgi tipi bir kez ve tablo kurallarıyla alınır.
- `cargo test -p kentos-desktop --lib opening`: açılmayan dosya ekranda kalır, Tamam kapatır, çizim değişmez.
- `cargo test -p kentos-formats --lib import`: birkaç çizim uzaktaki nesneler ilk görünümün dışında kalır ve sayılır;
  düzgün dağılan çizim bütün açılır.
