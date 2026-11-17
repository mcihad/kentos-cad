# ADR 9003: Harç ve ücret: veri olarak tarife, dosyaya tahakkuk

- **Durum:** kabul edildi (bulut-portal dalı)
- **Tarih:** 2026-11-18
- **Bağlam belgesi:** ADR 9000 (bulut portalı, §5 dosyanın akışı, §17 bildirimler); plan/bulut.md `BUL-22`;
  2464 sayılı Belediye Gelirleri Kanunu (Ek Madde 1, 2, 5, 6, 7; m.80, m.84, m.95, m.96, m.97)

## §1 Bağlam

Sahibin isteği (18 Kasım 2026): “Harç ve ücret parametrik olsun, gelişime açık olsun.” Belediyede imar işlerinin harcı
dosyadan ayrı tutuluyor: memur başvuruya bakıp tutarı elle hesaplıyor, makbuz numarasını bir deftere yazıyor. Ruhsat da
bu tutar ödenmeden verilmemeli.

Kanun bu işi şöyle kuruyor (metin: mevzuat.gov.tr, 2464 sayılı Kanun'un güncel hâli):

- **Bina inşaat harcı.** Ek Madde 1 konusu: ruhsat alınırken, ilave ve tadil dahil. Ek Madde 6 tarifesi: konut ve
  işyeri için ayrı, toplam inşaat alanının **girdiği dilime** göre m² başına bir tutar. Konutun dilimleri: 100 m²'ye
  kadar, 101–120, 121–150, 151–200, 200 m²'den yukarı. İşyerinin dilimleri: 25 m²'ye kadar, 26–50, 51–100, 100 m²'den
  yukarı. Her dilimin bir “en az – en çok” sınırı vardır.
- **Ödeme zamanı.** Ek Madde 5: harç ruhsat alınmadan önce, makbuz karşılığında ödenir. Ruhsat kısmen ya da tamamen
  iptal edilirse düzeltme ve iade yapılır.
- **İmarla ilgili harçlar.** m.80 konuları sayar (plan ve proje tasdik, yapı kullanma izni…); m.84 tarifeleri en az ve
  en çok tutarla verir. İstisnalar Ek Madde 2'de ve m.80'de (kamu bütçesinden yapılanlar, sağlık, sanayi…).
- **Tutarı kim belirler.** m.96/A: Kanun'da en az ve en çok tutarla verilen vergi ve harçların tarifesini, belediye
  grupları itibarıyla (m.95) Cumhurbaşkanı belirler. Ek Madde 6'nın dipnotu 2005/8730 sayılı Bakanlar Kurulu Kararı'nı
  gösterir. m.96/A bazı maktu tutarların her yıl yeniden değerleme oranında artırılacağını da söyler.
- **Ücretler.** m.97: harç konusu olmayan ve isteğe bağlı her hizmetin ücretini belediye meclisinin tarifesi belirler
  (örneğin kat irtifakı proje onayı).
- **Tek başvuru.** Ek Madde 7: başvuru tek seferde yapılırsa harçlar ve ücretler hesaplanıp tek seferde tahsil edilir.

Sonuç olarak tutarlar belediyeden belediyeye ve yıldan yıla değişir, kalemler de zamanla çoğalır. Bu yüzden program
tutar ya da kalem bilmemeli.

## §2 Karar

### Tarife veridir

- **Tarife bir JSON belgesidir.** Belgede üç bölüm vardır:
  - girdiler (`parameters`): inşaat alanı, yapının kullanımı, bölge, istisna…;
  - birim tutar tabloları (`tables`): bir seçeneğe ve bir miktar dilimine göre satır;
  - kalemler (`items`).

  Saklandığı yer `kentos.fee_schedule.definition` (migration 0048), sözleşmesi `FeeDefinition` (kentos-contracts).
  Yeni bir harç yeni bir kalemdir. Bunun için migration da program değişikliği de gerekmez.
- **Kalemin üç hesabı var:**
  - maktu (`fixed`);
  - birim başına (`per_unit`): miktar çarpı tablodan bulunan ya da sabit birim tutar;
  - formül (`formula`): KentOS ifade dilinde (ADR 0100); girdiler ve tablolar `@ad` ile okunur.

  Her kalemde ayrıca şunlar bulunur:
  - koşul (ör. `değil @kamu_istisnasi`);
  - en az / en çok tutar;
  - KDV oranı;
  - uygulandığı dosya türleri;
  - **karardan önce ödenir** işareti.

  Yeni bir hesap türü gerekirse sözleşmeye yeni bir türü olarak eklenir.
- **Dilimdeki kesir bir sonraki dilime düşer.** Kanun dilimleri tam sayıyla yazar (100'e kadar, 101–120). 100,5 m² ise
  100'ü aşmıştır. Tablo önce miktarın kendisiyle, uyan satır yoksa yukarı yuvarlanmış miktarla aranır. Dilim tutarı
  alanın **tamamına** uygulanır, artan oranlı değildir: alan hangi dilime giriyorsa tamamı o dilimin tutarıyla çarpılır.
- **Para kuruştur, tam sayıdır** (`i64`). Yuvarlama tarifenin kendi ayarıdır: kuruşa (yarım yukarı, varsayılan),
  liraya ya da liraya aşağı. Kayan noktanın `1,005`'i `1,00499…` diye tutması bir kuruş kaybettirmesin diye yarım,
  1e-7 payıyla yukarı alınır (sınama: `a_year_is_revalued_and_halves_round_up`).
- **Her satır neye dayandığını yazar:** “152,5 m² × 48 TL (Konut, 151–200 m²)”. Meslek mensubu tutarı tek tek kontrol
  edebilir, memur hesabı açıklamak zorunda kalmaz.
- **2464'e göre başlangıç.** `GET /v1/tenants/{t}/fees/template` Kanun'un yapısını verir:
  - bina inşaat harcının dokuz dilimli tablosu;
  - bölgeye göre plan ve proje tasdik harcı ile yapı kullanma izni harcı;
  - istisna girdisi.

  **Bütün tutarlar 0'dır.** Yılın tutarı belediyenin grubu için verilmiş karardır ve yeniden değerlenir. Program bir
  tutar uydurmaz. Örnek veri (seed) örnek tutarlar kullanır ve bunu kararın adında söyler.
- **Bir yıl bir kopyadır.** “Yeni yıla kopyala” bütün tutarları, dilim sınırlarını ve en az / en çok tutarları bir
  katsayıyla çarpar ve kuruşa yuvarlar (`revalued`).
  - **Formüllerin içindeki sayılar çarpılmaz.** Formüldeki bir sayı adet, oran ya da yüzde olabilir. Yeniden
    değerlenecek tutar tabloda ya da sabit birim tutarda durmalıdır. Kopyalama penceresi bunu söyler.
- **Taslak değişir, yürürlükteki değişmez.** Bir kurumun aynı anda en çok bir tarifesi yürürlükte olur (kısmi benzersiz
  indeks). Yürürlüğe giren taslak, öncekini geçmişe düşürür.

### Tahakkuk dosyadadır

- **Tahakkuk, yürürlükteki tarifenin bir dosyaya uygulanmış hâlidir** (`kentos.dossier_fee`). Kayıt şunları saklar:
  girdileri, satırları, toplamı, tarifenin adını ve ödeme notunu. Sonradan gelen bir tarife yapılmış tahakkuku
  değiştirmez (sınama: kopyalamadan sonra satır tutarı aynı).
- **Kim ne yapar:**
  - İki taraf da okur.
  - Belediyenin düzenleyicisi ve üstü tahakkuk eder, ödemeyi makbuz numarası ve günüyle kaydeder.
  - Proje yöneticisi ve üstü gerekçeyle iptal eder.
  - Satır güvenliği (RLS) yazmayı belediye tarafına bağlar; rolleri uygulama denetler.
  - Tahakkuk yalnız başvuruya yapılır (aktarım belediyenin kendi işidir) ve dosya kapanmamış olmalıdır.
  - Ödenmiş tahakkuk da iptal edilir. Bu durumda iade veznenin işidir (Ek Madde 5, ikinci fıkra) ve geçmişte öyle
    yazılır.
- **Bildirim ve geçmiş:**
  - Meslek mensubu tarafı tahakkuku ve ödemenin kaydını bildirimle öğrenir (`dossier.fee_issued`,
    `dossier.fee_paid`; ADR 9000 §17'nin e-posta ve SMS yolu dahil).
  - Dosyanın geçmişine `fee_issued`, `fee_paid` ve `fee_cancelled` düşer.
- **Ek Madde 5 kapısı:** başvuru şu iki durumda **onaylanmaz**:
  - yürürlükteki tarifede bu dosya türüne uyan “karardan önce ödenir” bir kalem varken hiç tahakkuk yapılmamışsa;
  - böyle bir kalem tahakkuk edilmiş ama ödenmemişse.

  Ret, düzeltme isteme ve öteki adımlar etkilenmez. Tarife hiç yoksa kapı da yoktur: harç işini henüz KentOS'ta
  tutmayan belediye eskisi gibi çalışır.
- **Tek tahakkuk.** Bir başvurunun harçları ve ücretleri tek tahakkukta, tek toplamla çıkar (Ek Madde 7). Sonradan
  çıkan bir kalem için “ek tahakkuk” yapılır.

### Uçlar

| Uç | Ne yapar |
|---|---|
| `GET/POST /v1/tenants/{t}/fees/schedules` | Tarifeler; yeni taslak (yönetici ve üstü) |
| `GET/PUT/DELETE …/schedules/{id}` | Okuma; yalnız taslak değişir ve silinir |
| `POST …/schedules/{id}/activate` | Yürürlüğe koyar |
| `POST …/schedules/{id}/copy` | Yeni yıla kopyalar (`factor`) |
| `GET /v1/tenants/{t}/fees/template` | 2464'e göre başlangıç |
| `POST /v1/tenants/{t}/fees/try` | Kaydedilmemiş tarifeyi bir dosya türünün girdileriyle dener |
| `GET/POST /v1/dossiers/{id}/fees` | Tahakkuklar; tahakkuk et |
| `POST /v1/dossiers/{id}/fees/preview` | Önizleme; boş girdiyle bu türün sorduğu girdileri verir |
| `POST /v1/dossiers/{id}/fees/{fee}/paid` · `/cancel` | Ödeme; iptal |

Dosyanın sayfası (`DossierDetail`) sunucunun kararını taşır: `canAssessFees`, `canCancelFees`, `feeCount`. Portal,
düğmeleri bunlardan açar. Masaüstü penceresi aynı alanları okuyabilir.

### Portal

- **Kurum › Harç ve ücretler:** tarifelerin listesi ve tek tarife sayfası. Sayfada şu bölümler var:
  - künye;
  - girdiler;
  - birim tutar tabloları (yerinde düzenlenir);
  - kalemler (pencerede düzenlenir);
  - **Dene:** sayfadaki hâliyle, kaydetmeden hesaplar;
  - gelişmiş kullanıcı için tarifenin JSON metni.
- **Dosya › Harç ve ücret sekmesi:**
  - Belediyede tahakkuk formu tarifenin bu türe sorduğu girdileri gösterir. Satırlar yazıldıkça hesaplanır.
  - Tahakkuk varken form “Ek tahakkuk” düğmesinin arkasında durur.
  - Meslek mensubu “Ödenecek” tutarı ve tarifenin ödeme notunu (“Nasıl ödenir”) görür.

## §3 Ölçülenler

- **Motor** (`portal::fees::tests`, 4 sınama):
  - Konut, 152,5 m², 151–200 dilimi, 48 TL → 7.320,00 TL.
  - 152,5 × 3,75 = 571,875 → 571,88 TL.
  - Formül, en az tutar ve %20 KDV: 350 → 500 TL + 100 TL KDV.
  - İstisnada yalnız ücret kalır.
  - Yanlış girdi ve eksik girdi alanıyla söylenir.
  - Yeniden değerleme: 12 × 1,4393 = 17,27; 3,75 × 1,4393 = 5,40.
- **Akış, gerçek PostgreSQL'de** (`tests/portal.rs` `fees_are_issued_by_the_tariff_and_paid_before_the_permit`):
  - Düzenleyici tarife yazamaz (403).
  - Tarife yokken önizleme anlaşılır bir hatayla döner.
  - Tahakkuk yokken onay reddedilir: “tahakkuk ettirilmeli”.
  - Meslek mensubu önizleyemez.
  - Boş önizleme dört girdiyi sorar.
  - Tahakkuk 7.510,63 TL'dir ve meslek mensubuna bildirim gider.
  - Ödenmemişken onay reddedilir: “ödenmedi”.
  - Ödeme kaydedilir, ikinci kayıt reddedilir, ardından onay geçer.
  - Geçmişte “7.510,63 TL” ve “makbuz MKB-2026-118” yazar.
  - Düzenleyici iptal edemez, proje yöneticisi edebilir.
  - Kopya ×1,5 ile 48 → 72 olur.
  - Yürürlükteki tarife değişmez.
  - Tahakkukun satırı kopyadan sonra da aynıdır.
- **Portal ekran görüntüleri** (`.run/gui-evidence/portal/87–95`, demo verisi, tutarlar örnektir):
  - tarifeler: geçmiş 2025, yürürlükte 2026, taslak 2027;
  - taslağın künyesi ve girdileri;
  - dokuz dilimli tablo;
  - kalemler ve “Dene”;
  - kalem penceresi;
  - belediyenin sekmesi;
  - canlı önizleme: 86,4 m² işyeri → 7.236,00 TL;
  - mimarın ödenecek tutarı (24.635,75 TL);
  - ödenmiş ve onaylanmış başvuru.

## §4 Açık kalanlar

- **Tahsilat.** Sanal POS, banka entegrasyonu ve e-tahsilat kurumla sözleşme ister; bu karar ödemeyi yalnız kayıt
  olarak tutar.
- **Grup tutarları kataloğu.** Kanun'un ve grup kararının yıllık tutarlarını yayımlamak ayrı bir iştir. Tutar
  uydurulmaz; her belediye kendi tutarını yazar.
- **Tahakkuk çıktısı.** Kâğıt tahakkuk ve makbuz örneği BUL-20 (raporlar) ile birlikte ele alınacak.
