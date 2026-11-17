# ADR 8086: Katlar arası açıklıklar — kompartıman katlar boyunca, BYKHY 24/4 ve 24/5

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8081](8081-fire-compartments.md) §6 ("Katlar arası açıklıklar: galeri, atrium (24/5) ve açık merdivenle birleşen
  katlar; 24/4"). Kompartımanın IFC grubu [ADR 8088](8088-fire-compartments-as-ifc-zones.md)'de yapıldı.
- **Bağlam:** ADR [8081](8081-fire-compartments.md) (kompartımanlar, ayrımlar), [8082](8082-fire-resistance-of-elements.md)
  (bina yüksekliği), [8045](8045-balanced-stairs-and-special-elements.md) §4 (döşeme boşluğu, merdivenin çıkışı),
  [8035](8035-fire-escape-routes-distances-load-exits.md) (kaçış merdiveni mahali).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 4/g (atrium; 2009/15316 sayılı BKK ile değişik), 24/4
  (2015/7401 sayılı BKK ile değişik, RG 9.7.2015/29411), 24/5 (2009/15316, RG 9.9.2009/27344), 26/1.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building`
  bölümüne isteğe bağlı `links`, mahallere `stair`.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 26/1 | Bütün döşemelerin yangın duvarı niteliğinde olması gerekir | İki katın mahalleri yalnız aralarındaki bir açıklıkla aynı kompartımana girer |
| BYKHY 4/g (2009) | Atrium: iki ya da daha çok katın içine açıldığı, merdiven yuvası, asansör kuyusu, yürüyen merdiven boşluğu ve tesisat bacaları ile şaftlar dışındaki üstü kapalı geniş ve yüksek hacim | Merdiven yuvası ve şaft olmayan katlar arası boşluk atrium |
| BYKHY 24/4 (BKK 2015/7401, 4 üncü madde; RG 9.7.2015, taranmış metin sayfa sayfa okundu) | Bina yüksekliği 21,50 m'den fazla olan konut harici binalarda ve 30,50 m'den fazla olan konut binalarında bu yüksekliklerin üstündeki katlarda en çok üç kat bir yangın kompartımanı | Eşiğin üstündeki katlarda bir kompartımanın kat sayısı |
| BKK 2015/7401, 23 üncü madde ve Geçici 1 | Yayımı tarihinde yürürlüğe girer; yayımından önce yapı ruhsatı verilmiş yapılara ve ilanı yapılmış kamu ihalelerine önceki hükümler | 24/4'ün 9.7.2015 hâli; önceki metin kural setinde yok, daha eski başvuru uzman incelemesi |
| BYKHY 24/5 (2009) | Atrium yalnız düşük ve orta tehlikeli kullanımlarda; alanı hiçbir noktada 90 m²'den küçük olmamalı, küçükse her katta en az 45 cm duman perdesi (yağmurlamalı binada başlıklar perdeden 15–30 cm, en çok 2 m arayla); doğal ya da mekanik duman kontrolü | Duman kontrolü ve perde modelde yok; "geniş ve yüksek hacim" yoruma bağlı: uzman incelemesi |

- **KentOS'ta parçalar hazırdı:** döşemenin delikleri (`SlabFields.parts[].holes`), boşluk mahal (`SpaceFields.void`),
  merdivenin yürüme çizgisi ve çıkışı (ADR 8045 §4'ün `arrivals`'ı), kaçış merdiveni mahali (`escape::stair_name`), bina
  yüksekliği (`fire_audit::heights`, ADR 8082).

## §2. Karar

- **K98 — Açıklıklar ölçülür** (`Document::vertical_links`, `crates/native/domain/src/vertical_links.rs`):
  - **Delik:** bir katın döşeme rolündeki döşemesinin, katın başka bir döşemesinin örtmediği (indirilmiş ıslak hacim) ve
    katın bir şaftının tutmadığı deliği. Alt kattaki mahaller: deliğin ortası ve kenarlarının 5 cm içindeki noktalar.
    Üst kattaki mahaller: ortasını tutan ve kenarlarından dışarı 2 cm adımlarla en çok 1 m yürünerek bulunan mahaller.
  - **Boşluk mahal:** şaft olmayan boşluk mahal (Galeri boşluğu, Atrium) delik gibi okunur. Şaft: asansör kuyusu,
    ışıklık, hava bacası ve adında Şaft, Baca, Tesisat geçen (`room_names::shaft`; okuma uygulamadan alana taşındı).
  - **Merdiven:** ayağını tutan mahal (katında) ve başını tutan mahal (üst katta), çizgisi boyunca en çok 1 m ileride.
  - **Merdiven yuvası:** alt kattan bir merdivenin başının ya da 15 cm gerisinin düştüğü delik ya da boşluk.
  - **Yığın:** ardışık katlarda üst üste duran (birinin ortası öbüründe) açıklıklar bir hacimdir, en alttakinin kimliğiyle.
- **K99 — Kompartıman katlar boyunca** (`kompartiman.kompartimanlar`): açıklığın birleştirdiği mahaller aynı
  kompartımandadır; Ek-4 alanı katlar boyunca toplanır (24/6). Yalnız kaçış merdiveni mahallerinden oluşan kompartıman
  merdivenin yuvasıdır.
- **K100 — 24/4:** konut binası (bütün bölümleri konut) için 30,50 m, öbürleri (karma dahil, sıkı olan) için 21,50 m;
  bina yüksekliği eşiği aşmıyorsa kapsam dışı. Kompartımanın kotu eşiğin üstündeki katları üçü aşıyorsa kaldı
  (`bykhy_compartment_storeys_over`); merdiven yuvası sayılmaz.
- **K101 — 24/5:** merdiven yuvası olmayan her yığın bir atriumdur: uzman incelemesi (`bykhy_atrium`): duman kontrolü,
  90 m²'den küçük yığında 45 cm duman perdesi (yağmurlamalı binada başlıkların yeri), binanın kullanımının tehlike sınıfı
  Ek-1'e bağlıysa o da iletide.
- **Kural seti:** 24/4 9.7.2015, 24/5 9.9.2009 hâliyle. Katlar arası açıklık yoksa 24/5 kapsam dışı, eski günlü
  başvuruda 24/4 ve 24/5 okunmaz (tek kat zaten kendi kompartımanıdır).

## §3. Uygulama

- **Alan:** `vertical_links.rs` (`VerticalLink`); `room_names::{ShaftKind, shaft_of, shaft}` (uygulamanın
  `regulation_audit` yeniden dışa verir).
- **Olgular:** `LinkFacts` (kimlik, tür, alt ve üst kat, mahaller, alan, merdiven yuvası, yığın), `SpaceFacts.stair`;
  `fixtures/audit-facts/v1/mimari.json` yeniden yazıldı (iki katı birleştiren ortak merdiven, iki "Merdiven evi").
- **Paket** (`tr.kentos.mimari-mevzuat` 0.10.0): `kompartiman.py` (`katlar_arasi`, `atrium`, `konut_binasi`);
  `data/surumler.json`'a BYKHY 24/4 ve 24/5.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/kompartiman.json`, 7 yeni durum, metinden elle): iki katlı büroda 24 m² galeri
  katları birleştirir, 9000 m² Ek-4'ün 8000'ini aşar, galeri atrium; merdiven yuvası atrium değil ama katları birleştirir;
  12 katlı büroda 21,50 m'nin üstünde 4 kat birleşik kalır, 3 kat tutar; konut binasında eşik 30,50; yalnız merdiven
  yuvasıyla birleşen katlar 24/4'ü bozmaz; 2014 başvurusunda 24/4 uzman incelemesi. Eski 22 durum tek katlı: 24/4 ve 24/5
  kapsam dışı satırları eklendi.
- **Masaüstü** (`storey_link_tests`, iki katlı büro, elle): döşeme yokken iki kompartıman; 4 × 4 m delikli döşemeyle tek
  kompartıman ve 16 m²'lik atrium (uzman incelemesi); delikten çıkan merdivenle merdiven yuvası, atrium yok, katlar yine
  birleşik. Olgular: bir delik, iki mahal, 16 m²; merdivenle bir delik (yuva) ve bir merdiven. `regulation_dates_tests`'in
  satırı: 12 madde kapsam dışı.
- **Resimler:** `.run/gui-evidence/atrium-8086/` (1. kat ve Denetim'de atrium bulgusu; açık ve koyu).

## §5. Arayüz

- Yeni pencere yok. Bulgular Denetim'in Mevzuat grubunda; maddeler kural setinin "BYKHY 24/4", "BYKHY 24/5" satırlarında.

## §6. Sıradaki adımlar

- **Kompartımanın IFC grubu** (bim-plan "Gruplar: … yangın kompartımanı"): paketin kompartımanları `IfcZone` olarak;
  hesap Python'da olduğu için dışa aktarma paketin sonucunu okumalı. [ADR 8088](8088-fire-compartments-as-ifc-zones.md)'de
  yapıldı.
- **Yürüyen merdiven boşluğu** (modelde yok), **duman kontrolü ve duman perdesi beyanı** (atriumun uzman incelemesini
  kapatır), **atrium alanının "hiçbir noktada"sı** (yığının en dar katı okunuyor; kesitte daralma okunmaz).
- **24/4 öncesi metin** (2007, 2009) kural setine girerse 9.7.2015'ten önceki başvuru da okunur.
