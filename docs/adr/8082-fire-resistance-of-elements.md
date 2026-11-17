# ADR 8082: Yapı elemanlarının yangına dayanım süreleri — Ek-3/B ve Ek-3/C

- **Durum:** kısmen uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 ("beyan edilen
  dayanımın öbür elemanlarda karşılaştırılması (Ek-3/B, Ek-3/C)"). Çatılar, korunumlu şaftlar, asma tavan, yangın
  kesici, asansör kat kapıları ve taşıyıcı duvar kaldı (§6).
- **Bağlam:** ADR [8068](8068-fire-performance-material-declarations.md) (beyan edilen sınıflar), [8035](8035-fire-escape-routes-distances-load-exits.md)
  (bina ve yapı yüksekliği), [8047](8047-ifc-building-export.md) (dış duvar okuması), [8081](8081-fire-compartments.md)
  (ayrımlar, kompartımanlar), [8078](8078-regulation-rule-set-by-application-date.md) §8 (kapılar).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 4/h (bina yüksekliği), 23, 24/1, 26/1, 29/7, Ek-3/B,
  Ek-3/C (RG 19.12.2007/26735; Ek-3/C 2009/15316 sayılı BKK ile değişik, RG 9.9.2009/27344).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building` bölümüne
  isteğe bağlı `elements`, `height`, `basementDepth`.

## §1. Araştırma

Tablolar yönetmeliğin PDF'inin 91. (Ek-3/A, Ek-3/B) ve 93. (Ek-3/C) sayfalarından resim olarak okundu; düz metin
(pdftotext) yalnız yön için kullanıldı (ADR 8081'de Ek-4'ün düz metni yanlış okutuyordu).

- **29/7:** "Ek-3/B'de yapı elemanlarının yangına dayanım süreleri, Ek-3/C'de bina kullanım sınıflarına göre yangına
  dayanım süreleri."
- **Ek-3/B** (seçilenler):

  | Satır | Eleman | Süre |
  |---|---|---|
  | 1 | Taşıyıcı sistem (çerçeve, kiriş, kolon) | R, Ek-3/C |
  | 3-a | İki katlı konutun ikinci katı (garaj ve bodrum üstü hariç) | REI 30 |
  | 3-b | Dükkânla üstündeki kat arası | REI 60 ya da Ek-3/C |
  | 3-c | Öbür döşemeler, kompartıman döşemeleri dahil | REI, Ek-3/C |
  | 3-d | Bodrumla zemin kat arası | REI 90 ya da Ek-3/C (büyüğü) |
  | 5 | Dış duvarlar | REI, Ek-3/C |
  | 6 | Farklı kullanımları ayıran kompartıman duvarı | REI 60 ya da Ek-3/C (büyüğü) |
  | 7 | Öbür kompartıman duvarları | REI, Ek-3/C |
  | 8 | Korunumlu şaftlar | REI 120 |
  | 9-a | Korunumlu yangın merdiveni yuvasını binanın geri kalanından ayıran duvar | REI 120 |
  | 9-b | Yuva, acil durum asansör kuyusu ve yangın güvenlik holü arası | REI 60 |
  | 10, 11 | Yangın kesici, asma tavan | EI 30 |
  | 12 | Asansör kat kapıları (2015'te eklendi) | E 60 / E 30 |

- **Ek-3/C:** kullanım sınıfı (bir ve iki ailelik ev, apartman, konaklama, kurumsal, büro, ticaret, endüstri,
  toplanma, depo, otopark), yağmurlama, bodrumlarda derinlik (10 m'den fazla, az), giriş ve üst katlarda bina
  yüksekliği (5, 21,50, 30,50 m'den az; 30,50 m'den fazla). Değerler `data/surumler.json`'da, tablonun yazdığı gibi
  (`30(2)`, `120(3)`, `---`, `İzin verilmez`).
  - Not (1): bir bodrumun üstündeki döşeme, giriş ve üst katlarınki daha fazlaysa onu sağlar.
  - Not (2): binaları ayıran kompartıman duvarlarında en az 60.
  - Not (3): taşıyıcı sistemin parçası olmayan elemanlarda 120 yerine 90.
  - Not (4): acil kaçışı oluşturan elemanlarda 30.
- **Bina yüksekliği** (4/h): binanın kot aldığı noktadan saçak seviyesine. KentOS'un yangın kuralları onu ±0,00 ve
  üstündeki katların yüksekliklerinin toplamı olarak ölçüyordu (ADR 8035). Aynı ölçü kullanılır.
- **Bodrum derinliği** (Ek-3/C dipnotu): en alt bodrum döşemesiyle zemin kat döşemesi arası.

## §2. Karar

- **K79 — Elemanlar ölçülür** (`Document::fire_elements`):
  - Taşıyıcı sistemin kolonları (dikme değil) ve kirişleri (lento ve hatıl değil).
  - Duvarlar ve dış olup olmadıkları: katın duvar ve kolon gövdelerinin birleşiminin dış çizgisinde köşesi olan duvar,
    IFC'nin `IsExternal` kuralı (ADR 8047), duvarların kendi gövdeleriyle.
  - Döşeme rolündeki döşemeler (balkon, teras ve kaplama değil).
  - Olgular her birinin beyanını taşır (`fire_rating_of`). Bina yüksekliği `fire_audit::heights`'tan: yangın kuralları
    ve olgular aynı ölçüyü okur.
- **K80 — Kural paketin** (`tr.kentos.mimari-mevzuat` 0.6.0, `dayanim`; K10):
  - **Kolon ve kiriş:** R, elemanın katının Ek-3/C dakikası kadar.
  - **Döşeme:** bir katın döşemesi REI, o katın dakikası kadar. Zemin katın döşemesi bodrum varken en az 90, bodrumun ve
    üst katların büyüğü (3-d, not 1). İki katlı bir ve iki ailelik evin ikinci katı REI 30 (3-a). En alt katın
    döşemesi zemine oturur, okunmaz.
  - **Dış duvar:** E ve I, katın dakikası kadar. Taşıyıcılığı çizimde yok; not (3)'ün 120'den 90'a indirdiği yerde
    90–120 arası beyan uzman incelemesi.
  - **Kompartıman duvarı** (ADR 8081'in sınırları): E ve I, en az 60 ve katın dakikası.
  - **Korunumlu yangın merdiveni** (adı yangın merdiveni ya da korunumlu olan mahal): binanın geri kalanına REI 120,
    yangın güvenlik holüne REI 60.
  - **İzin verilmez:** yağmurlamasız konut dışı bina 30,50 m'yi aşarsa bulgu.
- **Okuma:** sınıfın harflerinden biri eksikse ya da dakika azsa kaldı. Beyan yoksa ya da okunmuyorsa eksik veri.
  - Bulgular kat, eleman türü, sorun ve istek başına birdir; nesneleri seçer.
  - Tablonun iki sütununun tam sınırında (bina yüksekliği 30,50 m, bodrum 10 m) uzman incelemesi.
- **Sınıf okuyucu:** `yangin_sinifi.okuma` her harfi okur (R 60 bir kolonun). `sinif` kapılar ve ayrımlar için E'li
  olanı verir.

## §3. Uygulama

- **Belge:** `fire_elements.rs` (`FireElement`, `Document::fire_elements`).
- **Uygulama:** `fire_audit::heights` (yangın kurallarının ölçüsü; kaçış kuralları da onu kullanır).
- **Olgular:** `ElementFacts` (kimlik, tür, kat, beyan, taşıyıcı sistem, dış); `height`, `basementDepth`. Fixture'ın
  planı: 20 duvar (8'i dış), 6,00 m, bodrumsuz.
- **Paket:** `dayanim.py`, `yangin_sinifi.okuma`, `data/surumler.json`'a BYKHY Ek-3 (Ek-3/C'nin satırları veri),
  manifestte `dayanim` denetimi.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/dayanim.json`, 18 durum, tablolardan elle):
  - Apartman 12 m: kolon R 60, döşemeler, dış duvar, zemin döşemesi REI 90 (geçer); kolon R 30 (kalır); beyansız
    kolonlar (eksik veri); 24 m'de R 60 (kalır, 90); zemin döşemesi REI 60 (kalır, 3-d).
  - Ev: iki katlı tek konut ve iki daireli "Konut" REI 30 ikinci kat (geçer).
  - Büro: yağmurlamasız 32 m izin verilmez; yağmurlamalı 32 m kolon R 120 ve dış duvar EI 90 (uzman); dış duvar EI 60
    (kalır); yağmurlamalı 12 m kolon R 30 (geçer); 24 m'de EI 60 kompartıman duvarı (kalır, 90).
  - Korunumlu yangın merdiveni: koridora REI 90 (kalır), güvenlik holüne REI 60 (geçer).
  - Kullanım yok (eksik veri); bodrum tam 10 m (uzman); eleman yok; tarihsiz; 2008 başvurusu.
- **Masaüstü** (`resistance_tests`, iki katlı ev, elle): olgular bir taşıyıcı kolon, sekiz dış duvar, bir döşeme,
  6,00 m. Denetim: kolon R 30 Ek-3/C'nin 60'ının altında (kaldı); üst katın beyansız duvarları eksik veri; döşeme REI 60
  ikinci katın REI 30'unun üstünde; kolon R 60 olunca yalnız eksik veri kalır.

## §5. Arayüz

- Yeni pencere yok; bulgular Denetim'in Mevzuat grubunda, madde kural setinin "BYKHY Ek-3" satırında.

## §6. Sıradaki adımlar

- **Çatılar:** 4-a kaçış yolu olan bölüm REI 30, 4-b döşeme görevi yapan REI Ek-3/C, 4-c dıştan yangına maruz EI.
- **Korunumlu şaftlar** (REI 120), **asma tavan** ve **yangın kesici** (EI 30), **asansör kat kapıları** (E 30/60, yapı
  yüksekliği 51,50 m).
- **Taşıyıcı duvar** (Ek-3/B 2): duvarın taşıyıcı olduğu beyanı modelde yok; not (3)'ün uzman incelemesini de kapatır.
- **Dükkânla üstündeki kat arası** (3-b): mahallerin kullanımından.
- **Binaları ayıran duvarlar** (24/3, not 2) birden çok binalı parselde.
