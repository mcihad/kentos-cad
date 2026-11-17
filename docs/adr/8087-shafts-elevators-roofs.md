# ADR 8087: Şaftlar, asansörler ve çatılar — BYKHY 25/3, 41/9, 62/2, 62/4, 62/6, 62/7; Ek-3/B 4 ve 12

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8082](8082-fire-resistance-of-elements.md) §6 ("Çatılar: 4-a, 4-b, 4-c"; "Korunumlu şaftlar (REI 120) … asansör kat
  kapıları (E 30/60, yapı yüksekliği 51,50 m)").
- **Bağlam:** ADR [8081](8081-fire-compartments.md) (ayrımlar: duvar, açıklık ve açık yer), [8082](8082-fire-resistance-of-elements.md)
  (Ek-3, bina ve yapı yüksekliği), [8086](8086-compartments-across-storeys.md) (şaftın adıyla okunması, `room_names`),
  [8034](8034-regulation-stairs-shafts-elevators-local-rules.md) (asansör kuyusu ve şaftın ölçüleri, PAİY),
  [8068](8068-fire-performance-material-declarations.md) (kapının ve tipinin yangın sınıfı beyanı).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik (RG 19.12.2007/26735) 4/bbb (yapı yüksekliği), 4/ccc
  (yüksek bina; 2009/15316 sayılı BKK ile değişik, RG 9.9.2009/27344), 25/3, 41/9 (2009/15316 ile değişik), 62/2, 62/4,
  62/6, 62/7 (2015/7401 sayılı BKK ile değişik, RG 9.7.2015/29411), Ek-3/B 4, 8 ve 12 (12 nci satır 2015/7401'in 21 inci
  maddesiyle eklendi).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building` bölümüne
  isteğe bağlı `shafts` ve `structureHeight`, kapılara `elevator` ve `passage`; `ElementFacts.kind`'a `roof`, `terrace`.

## §1. Araştırma (17 Kasım 2026)

Metin yönetmeliğin derlenmiş PDF'inden (mevzuat.gov.tr, `pdftotext` ile; ADR 8082'deki kopya), Ek-3/B'nin 4, 8 ve 12 nci
satırları tablonun sayfasından, 2015 değişikliğinin yürürlüğü BKK 2015/7401'in taranmış metninden (ADR 8086 §1) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 4/ccc (2009) | Yüksek bina: bina yüksekliği 21,50 m'den, yapı yüksekliği 30,50 m'den fazla | İki koşuldan biri yeter; `height` ve `structureHeight` (`fire_audit::heights`) |
| BYKHY 25/3 | Yüksek binalarda çöp, haberleşme, evrak ve teknik donanım gibi düşey tesisat şaft ve baca duvarlarının yangına en az 120, kapaklarının en az 90 dakika dayanıklı ve duman sızdırmaz olması | Duvar E ve I ile 120; kapak (şaft duvarındaki kapı, pencere, boşluk) E ile 90 ve S; açık yer şaftı bozar |
| BYKHY 41/9 (2009) | Kaçış merdiveni yuvasına ve yangın güvenlik holüne elektrik ve mekanik tesisat şaftı kapakları açılamaz, kombi, dış ünite, sayaç konulamaz | Şaftın kapısının öbür yanı kaçış merdiveni mahali ya da yangın güvenlik holü olmamalı; çöp bacası sayılmaz; cihazlar modelde yok |
| BYKHY 62/2 | Asansör kuyusu ve makine dairesi yangına en az 60 dakika dayanıklı ve yanıcı olmayan malzemeden | Kuyunun duvarı ve duvarındaki pencere, boşluk E ve I ile 60; malzeme ve makine dairesi okunmaz |
| BYKHY 62/4 | Kuyuda havalandırma bacası ya da basınçlandırma; bodruma inen asansöre korunmuş koridor ya da güvenlik holünden ulaşma; asansör kapıları koridor, hol ve benzeri alanlar dışında doğrudan kullanım alanlarına açılamaz | Yalnız kapının önü okunur: bir yanı geçit (koridor, hol, antre, giriş, lobi, sahanlık, merdiven) |
| BYKHY 62/6 | Asansör kapısı yangın merdiven yuvasına açılamaz | Kapının bir yanı adında Yangın merdiveni ya da Korunumlu merdiven geçen mahal olmamalı (Ek-3/B 9'un okuması, ADR 8082) |
| BYKHY 62/7 (2015) | Asansör kat kapıları TS EN 81-58'in özelliklerinde ve Ek-3/B'deki dayanımda | Kat kapısı Ek-3/B 12'yle; standardın deneyi okunmaz |
| Ek-3/B 12 (2015, BKK 2015/7401 m. 21) | a) Yapı yüksekliği 51,50 m'den yüksek binada E 60; b) alçak binada E 30 | Tam 51,50 m'de iki satır da tutmaz: uzman incelemesi |
| BKK 2015/7401 m. 23, Geçici 1 | Yayımı günü (9.7.2015) yürürlüğe girer; önce ruhsat almış yapılara önceki hükümler | 62/7'nin 9.7.2015 hâli; önceki metin kural setinde yok: daha eski başvuru uzman incelemesi |
| Ek-3/B 4 | a) Kaçış yolu olan çatı bölümü REI 30; b) döşeme görevi yapan her türlü çatı REI, Ek-3/C; c) dıştan yangına maruz çatı (yük taşıyıcı değil) EI, Ek-3/C | Teras döşemesi (`SlabRole::Terrace`) 4-b, çatı (`Entity::Roof`) 4-c; 4-a çizimde okunmaz |
| Ek-3/B 8 | Korunumlu şaftlar (korunumlu yangın merdiveni yuvası ve acil durum asansör kuyusu hariç) REI 120 | "Korunumlu şaft" tanımlı değil, şaftın korunumlu olduğu beyanı modelde yok: okunmaz; yüksek binanın tesisat şaftını 25/3 aynı 120 dakikayla okur |

**Model denetimi ve modelleme kılavuzları** (karşılaştırma; KentOS kararı tabloda):

- Solibri'nin yangın kompartımanı kuralı tanımlanmış kompartımanları ayıran duvar, kapı ve pencerenin yangına dayanıklı
  yapı tipinden olduğunu, ayrı bir kural yangın duvarındaki doldurulmamış açıklığı denetler
  ([help.solibri.com](https://help.solibri.com/hc/en-us/articles/11926739699095)); şafta özgü bir şablon bulunmadı.
- FireBIM modelleme kılavuzu (ITEA, 2026): kompartımanlamaya giren şaft `IfcSpace` olarak modellenir, duvarın ve kapının
  dayanımı `Pset_WallCommon.FireRating`, `Pset_DoorCommon.FireRating`'te, kompartıman `IfcZone`
  ([itea4.org](https://itea4.org/project/workpackage/deliverable/document/download/1121/firebim-modelling-guideline-snapshot-2026-09-07.html)).
- Ahşap yüksek binada BIM tabanlı yangın denetimi (MDPI Buildings 2020): ayrımın sürekliliği ve dayanımı denetlenir;
  kompartıman modelde doğrudan yoktur, dayanım alanı mevcut projede çoğu kez boştur
  ([mdpi.com](https://mdpi.com/2075-5309/10/7/121)): KentOS'ta beyansız eleman eksik veridir.
- Uluslararası Yapı Yönetmeliği (IBC 713) dört ve daha çok katı bağlayan şaftı 2 saat, daha azını 1 saat ister
  ([structuremag.org](https://www.structuremag.org/article/shaft-wall-solutions-for-wood-frame-buildings/)); yalnız
  karşılaştırma, Türkiye'de BYKHY bağlar.

## §2. Karar

- **K102 — Şaftın çevresi ölçülür** (`Document::shaft_enclosures`, `crates/native/domain/src/separations.rs`): asansör
  kuyusu (boşluk mahal, adı Asansör) ve tesisat şaftı (Şaft, Tesisat, Baca, hava bacası; ışıklık gökyüzüne açıktır,
  sayılmaz). Çevresinin her 25 cm'sinden dışarı 2 cm adımlarla en çok 80 cm yürünür: ilk açıklık, yoksa ilk duvar gövdesi,
  yoksa açık yer; her birinin boyu. Köşeden ayrımlardaki 30 cm değil 10 cm (kısa kenarın dörtte biri) uzak durulur:
  şaftın kenarı kısadır, çevresindeki duvarlar köşesini tutar.
- **K103 — Asansör kapısı ve geçit** (olgular): kapının bir yanı asansör kuyusunun içindeyse `elevator`; bir yanındaki
  mahal geçitse (`room_names::passage`: Koridor, Hol, Antre, Giriş, Sahanlık, Merdiven, Geçit; `kind_of`'un hol okuması;
  Lobi; merdiven mahali) `passage`.
- **K104 — 25/3:** yüksek binada (4/ccc'nin iki koşulundan biri) her tesisat şaftının duvarı EI 120, kapağı E 90 S;
  açık yer kaldı, beyansızlık eksik veri (`bykhy_shaft_resistance`).
- **K105 — 41/9:** çöp bacası dışındaki tesisat şaftının kapısı (şaftın çevresindeki kapı) kaçış merdiveni mahaline ya
  da yangın güvenlik holüne açılıyorsa kaldı (`bykhy_shaft_cover_stair`).
- **K106 — 62/2:** asansör kuyusunun duvarı ve duvarındaki pencere, boşluk EI 60; açık yer kaldı
  (`bykhy_elevator_shaft`). Çevresinde hiç duvar ve açıklık yoksa (yalnız mahal çizilmiş) eksik veri: açık sayılmaz.
- **K107 — 62/4 ve 62/6:** asansör kapısının önü geçit değilse kaldı, önünde mahal yoksa eksik veri
  (`bykhy_elevator_door_room`); bir yanı yangın merdiveni yuvasıysa kaldı (`bykhy_elevator_door_stair`). Merdiven evi
  geçittir; adı yangın ya da korunumlu merdiven olmayan merdivene açılan asansör 62/6'yı bozmaz.
- **K108 — 62/7 ve Ek-3/B 12:** 9.7.2015'ten kat kapısı yapı yüksekliği 51,50 m'den yüksekse E 60, alçaksa E 30; tam
  51,50 m uzman incelemesi (`bykhy_elevator_door_resistance`). Satır Ek-3'ün değil 62/7'nin hâlidir: 9.7.2015'ten önceki
  metin bilinmez; Ek-3'e konsaydı tarihsiz projede kapının güne bağlılığı bütün elemanların bulgularını tek eksik veriye
  gömerdi.
- **K109 — Çatılar** (`dayanim`): teras döşemesi 4-b REI, çatı 4-c EI; ikisi de katının Ek-3/C süresiyle. Teras döşemesi
  önceden okunmuyordu (yalnız döşeme rolü); bodrum ve iki katlı evin döşeme kuralları ona uygulanmaz.
- **Kural seti:** 25/3 ve 41/9 9.9.2009 (4/ccc'nin ve 41/9'un 2009 hâli), 62/2, 62/4, 62/6 19.12.2007, 62/7 9.7.2015
  hâliyle. Konusu (şaft, asansör kapısı) olmayan madde kapsam dışı; metni bilinmeyen günde konusu varsa uzman
  incelemesi, yoksa satırı yok. Gün yokken adaylar maddelerin hepsinin bilindiği 9.7.2015'ten başlar (tek aday).

## §3. Uygulama

- **Alan:** `separations.rs` (`Enclosure`, `bodies`, `shaft_enclosures`); `fire_elements.rs` (`roof`, `terrace`).
- **Olgular:** `ShaftFacts` (kimlik, kat, ad, tür, duvarlar ve açıklıklar beyanlarıyla, açık boy),
  `BuildingFacts.shafts`, `structure_height`, `DoorFacts.elevator`, `passage`; `fixtures/audit-facts/v1/mimari.json`
  yeniden yazıldı (örneğin asansör kuyusu, dört duvarıyla).
- **Açıklığın halkası (düzeltme):** açıklığın `place`'i kasaların noktalarıdır (başlangıç sol, başlangıç sağ, bitiş sol,
  bitiş sağ); bu sırayla halka kendini keser ve duvarın yüzüne yakın yalnız kasaların dibini tutar. Ayrımlar (ADR 8081)
  duvarın ortasından baktığı için çoğu yerde tutuyordu; şaft yüzünden baktığı için 60 cm'lik kapıyı kaçırdı. Bitiş
  kasasının noktaları ters sırayla alınır (`bodies`). Şaftın yürüyüşü ilk adımdan başlar: çevresinin üstünde duvarın
  yüzü ile açıklığın kenarı buluşur.
- **Geçişin yeri (düzeltme):** ayrım ve şaft, kenarı 25 cm'de bir okuyup her okumaya bir adım boyu yazıyordu; 1,00 m'lik
  kapı adımların hizasına göre 4 ya da 5 adım (0,99 ya da 1,24 m) çıkıyordu. Ardışık iki okuma ayrıysa geçiş ikiye
  bölünerek 0,1 mm'ye bulunur (`spans`): kapının boyu genişliğidir. Birleşen duvarların üst üste bindiği yerde iki gövde
  tutuyorsa küçük kimlikli olan seçilir (`element`): çizimin sırası sonucu değiştirmez. Örnek projenin olgularında
  kapılar 0,747–0,996 m'den 0,90 ve 1,00 m'ye, duvarlar yuvarlak boylarına düzeldi (`fixtures/audit-facts/v1/mimari.json`).
  ADR 8081'in kuralı (ayrımın öğeleri ve açık yeri) değişmedi; yalnız boyları kesinleşti.
- **Paket** (`tr.kentos.mimari-mevzuat` 0.11.0): `kuyular.py` (`tesisat`, `kapaklar`, `kuyu`, `onu`, `merdivene`,
  `kat_kapilari`), Denetim'in `kuyular` katkısı; `dayanim.py`'ye çatılar; `data/surumler.json`'a BYKHY 25/3, 41/9, 62/2,
  62/4, 62/6, 62/7. Kural setinin metin sınaması fıkranın kendisini ister: en kısa fıkra 62/6 (54 harf), eşik 80'den 40'a
  indi (yer tutucuyu yine yakalar).
- **Komut:** `cad.arch.audit`'in belgesi ve kataloğun özeti yeni kodları sayar; Python taslakları yeniden üretildi.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/kuyular.json`, 24 durum, metinlerden elle): 33 m büroda şaft EI 120 ve kapak
  E 90-S200 geçer; bir duvar EI 60, dumansız kapak EI 90, beyansız duvar ve okunmayan "60 dk", 0,60 m açık; 12 m binada
  25/3 kapsam dışı; bina 20, yapı 31 m'de 4/ccc'nin ikinci koşulu; şaft kapağı merdiven evine ve güvenlik holüne açılır,
  çöp bacası sayılmaz; asansör kuyusu EI 60 geçer, EI 30 kalır, beyansız pencere ve duvarsız kuyu eksik veri; kapı ofise
  açılır, önü çizilmemiş, yangın merdivenine açılır, merdiven evine açılır; 54 m'de E 30 kalır, tam 51,50 uzman
  incelemesi, beyansız kapı eksik veri; 2014 başvurusunda 62/7, 2008'de 25/3 ve 41/9 uzman incelemesi; tarihsiz.
  `fixtures/standards/v1/dayanim.json`'a 2 durum: teras REI 30 kalır, çatı EI 60 geçer; beyansız çatı eksik veri.
- **Masaüstü** (`shaft_tests`, 8 katlı büro, 24 m, elle): olgularda kuyunun dört duvarı ve kat kapısı, şaftın dört
  duvarı ve kapısı, açık yer yok; kat kapısı asansörün ve hole açılıyor, şaft kapısı merdivene. Beyansızken 25/3, 62/2,
  62/7 eksik veri, 41/9 kaldı, 62/4 ve 62/6 geçer; EI 120, EI2 90-S200C5 ve E 30 beyanıyla yalnız 41/9 kalır.
  `regulation_dates_tests`'in satırı: duvarsız çizilmiş kuyular 62/2'de eksik veri, kapsam dışı 17 madde.
- **Resimler:** `.run/gui-evidence/saft-8087/` (Zemin kat ve Denetim'de 41/9 bulgusu; açık ve koyu).

## §5. Arayüz

- Yeni pencere yok. Bulgular Denetim'in Mevzuat grubunda; maddeler kural setinin "BYKHY 25/3", "BYKHY 41/9", "BYKHY
  62/2", "BYKHY 62/4", "BYKHY 62/6", "BYKHY 62/7" satırlarında.

## §6. Sıradaki adımlar

- **Ek-3/B 8 korunumlu şaft:** şaftın korunumlu olduğu beyanı (ya da 4/… tanımı) gelirse REI 120.
- **62/4'ün öbür cümleleri:** kuyunun havalandırma bacası (en az 0,1 m², kuyu alanının 0,025'i) ya da basınçlandırma;
  bodruma inen asansörün korunmuş koridor ya da yangın güvenlik holünden ulaşılması. **62/3:** bir kuyuda en çok üç
  kabin. **63:** acil durum asansörü ve güvenlik holüne açılması.
- **41/9'un cihazları** (kombi, iklimlendirme dış ünitesi, sayaç) ve **makine dairesi** (62/2) modelde yok.
- **Ek-3/B 4-a** (kaçış yolu olan çatı bölümü), **10, 11** (yangın kesici, asma tavan), **2** (taşıyıcı duvar): sınıf ve
  rol beyanı gelince.
- **62/7'nin 2015 öncesi metni** kural setine girerse 9.7.2015'ten önceki başvuru da okunur.
