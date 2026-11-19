# ADR 8148: 5/8'in bodrum listesi — bodrumda %30 hesabının dışındakiler

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8079](8079-emsal-article-22-limits.md) §6 ("5/8'in bodrum listesi: bodrumda merdivenlerin, sığınağın ve tesisat
  alanlarının asgari alanı %30'un dışındadır"), [8147](8147-basement-common-rooms-22-1-j.md) §6 (5/8-d ve 22/1-j).
- **Bağlam:** ADR 8030 (Emsal ve TAKS tablosu, %30), 8079 (22/1'in sınırları, toprak altındaki kat), 8147 (bodrumdaki
  ortak mekânlar), [8036](8036-parking-and-shelter-provisions.md) (otopark ve sığınak hesabı), 8069 (emsal zonları).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-o, 5/8 (RG 30.9.2017/30196 ile değişik; ibareleri 25.7.2019/30842,
  31.12.2022/32060), 22/1; Otopark Yönetmeliği (RG 22.2.2018/30340) 3/1-d, 4/1-ç, Ek-1; Sığınak Yönetmeliği 7, 8/a.
- **Sözleşme:** yeni komut yok. `kentos_domain::zoning::BasementBent`, `BasementApart`, `ZoningReport.basement`,
  `ZoningRoom.apart` ve `bent`, `PARKING_TIMES`, `MASJID_DWELLINGS`, `MASJID_OTHERS`, `CHILDREN_AREA`;
  `kentos_ifc::arch::EmsalArea.apart`. `kentos.auditFacts` 1'in alanları aynı (`zoning.outside`'ın belgesi). **`.kcad`:**
  değişmedi. **Web:** dokunulmadı; TODOS.md §0.2'de.

## §1. Sorun

Emsal ve TAKS tablosu bodrumdaki emsal harici mahalleri %30'un içinde sayıyordu. 5/8 binaların bodrum katlarında
yapılan zorunlu otoparkın 2 katını, sığınak, merdiven, şaft, tesisat alanlarının asgari alanlarını, ortak mescidin
150/300 m²'sini, bütün cepheleri gömülü ortak alanları ve ortak çocuk oyun ve bakım alanlarının 100 m²'sini "bu hesaba
dâhil edilmeksizin" emsal harici sayar. Fazla sayılan emsal harici %30'u aşınca katlar alanına ekleniyor, emsal
gerektiğinden büyük çıkıyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 5/8 (1.7.2026'ya kadar işlenmiş birleştirilmiş metin, alomaliye.com) | "… binaların bodrum katlarında yapılan; a) Zorunlu otopark alanlarının 2 katı, b) Sığınak, asansör boşlukları, merdivenler, bacalar, şaftlar, ışıklıklar, ısı ve tesisat alanları, yakıt ve su depoları, jeneratör ve enerji odası, kömürlüklerin ilgili mevzuat, standart ya da bu Yönetmeliğe göre hesap edilen asgari alanları, c) (Mülga), ç) Ortak alan niteliğindeki mescit ve müştemilatın konutlarda 150 m²'si, konut dışı yapılarda 300 m²'si, d) Bütün cepheleri tamamen gömülü olmak ve ortak alan niteliğinde olmak kaydıyla; otopark alanları ve 22 nci maddede belirtilen tamamen gömülü ortak alanlar, e) Ticari amaç içermeyen, ortak alan niteliğindeki çocuk oyun alanlarının ve çocuk bakım ünitelerinin toplam 100 m²'si, bu hesaba dâhil edilmeksizin emsal haricidir." | K379–K383 |
| PAİY 4/1-o | "Bodrum kat: Zemin katın altındaki katları" | Bodrum, binasının zemin katından alçak kat (K379) |
| Otopark Yönetmeliği 3/1-d, 4/1-ç | Birim otopark alanı manevra alanları dahil toplam park alanı; "Binek otoları için birim park alanı, manevra alanı dâhil en az 20 m2'dir" | Zorunlu otopark alanı, Ek-1'in istediği yer sayısı × 20 m² (en azı; K381) |
| Sığınak Yönetmeliği 7/a–c, 8/a (ADR 8036) | Kişi başına 1 m² net; 10'dan az bağımsız bölümlü konut yapısı ve küçük yapılar muaf | Sığınağın asgari alanı gereken alandır; istenmiyorsa 0 (K382) |
| ADR 8079, 22/1-k | "Bütün cepheleri tamamen toprağın altında kalan" katı KentOS üstü binasının ±0,00'ı ya da altı olan kat diye okur | 5/8-d'nin "bütün cepheleri tamamen gömülü"sü aynı okuma (K379) |

Listenin ibareleri değişti (a: 31.12.2022, b'nin kömürlük ibaresi 25.7.2019, c 25.7.2019'da mülga); tablo bugünkü
metni sayar (8095'teki gibi); eski hâllerin metni kural setinde yoktur (§6).

## §3. Karar

- **K379 — Bodrum ve gömülü kat:** bodrum, binasının zemin katından (bina programının zemin katı, yoksa Zemin adlı ya
  da ±0,00'a en yakın kat) alçak kattır. Bütün cepheleri gömülü kat, üstü binasının ±0,00'ı ya da altında olan kattır
  (`buried_levels`; model tek tesviye kotu taşır).
- **K380 — Sıra ve paylaşım:** emsal harici sayılan bir mahalin %30 içindeki kısmı (22/1 sınırının aşanı düşülmüş)
  okuyan ilk bende girer: d, b, ç, e, a. Sınırlı bendin payı mahallerine alanlarıyla paylaşılır. d'nin %30 dışına
  aldığı otoparklar a'nın 2 katından düşülür (sıkı taraf).
- **K381 — a, otopark:** bodrumdaki otopark (adı ya da kullanımı Otopark, Garaj) zorunlu otopark alanının 2 katına
  kadar %30 dışında; zorunlu alan Ek-1'in istediği yer sayısı × 20 m² (4/1-ç "en az 20 m²": en azı).
- **K382 — b, sığınak:** bodrumdaki sığınak Sığınak Yönetmeliği 8/a'nın gerekli alanı kadar %30 dışında; yönetmelik
  sığınak istemiyorsa (7/a–c) hiç. b'nin öbür mahallerinin (merdiven, şaft, baca, ışıklık, ısı ve tesisat alanı, depo,
  jeneratör, enerji odası, kömürlük) asgari alanı KentOS'ta hesaplanmaz: %30 içinde kalırlar (sıkı taraf).
- **K383 — ç, d, e:** ç: ortak alan grubundaki mescit (adı, kullanımı ya da grubu Mescit) bir bağımsız bölümü konutsa
  150 m²'ye (karma yapı konut sayılır, sıkı taraf), değilse 300 m²'ye kadar. d: bütün cepheleri gömülü katta ortak alan
  grubunun emsal harici mahalinin %30 içindeki bütün kısmı (otopark, merdiven evi, tesisat, kapıcı dairesi, 22/1-j'nin
  sınır içindeki mekânları). e: ortak alan grubundaki çocuk oyun ve bakım alanları (Çocuk ve Oyun ya da Bakım) toplam
  100 m²'ye kadar. Emsal ve TAKS tablosunda "Emsal harici, %30 dışında" satırının altında her bendin satırı
  ("— gömülü bodrumun ortak alanları" …); IFC'nin emsal zonunun açıklamasında "5/8: bodrumda … m² emsal harici, %30
  dışında".

## §4. Uygulama

- `crates/native/domain/src/zoning/basement.rs` (yeni: `BasementBent`, `BasementApart`, `basement_levels`, `apply`),
  `zoning.rs` (mahalin `parking`, `shelter`, `masjid`, `children`'ı; `basement`, `apart`; tablo satırları).
- `crates/shared/ifc/src/arch.rs` (`EmsalArea.apart`), `crates/native/application/src/arch/arch_ifc.rs`.
- `crates/shared/extensions/src/audit_facts/architecture/zoning.rs` (`outside`'ın belgesi).

## §5. Sınamalar

- **Masaüstü** (`basement_list_tests`, `emsal_limits_tests`'in iki katlı planı; 5/8, 4/1-o, Otopark Yönetmeliği Ek-1 ve
  4/1-ç, Sığınak Yönetmeliği 7/a ve 8/a'dan elle):
  - d: toprak altındaki bodrumda ortak alanın otoparkı, merdiven evi ve kazan dairesi 90 m², hepsi %30 dışında
    (emsale dahil 60, %30 içinde 30, dışında 90); tablo satırı 90.00; zonun açıklaması; dairenin kendi otoparkı ortak
    değil, a'nın (40 m²'nin 30'u).
  - a: yarı gömülü bodrumda (−2,00) iki dairenin üçte birer yeri, 1 yer: 20 m², 2 katı 40 m²; 30 m²'lik otopark
    dışında; 60 m²'lik iki otoparkın 40'ı (her birinin 20'si); merdiven evi ve kazan dairesi içinde.
  - ç, e: ortak mescit 30 m² (konutlu yapı: 150'ye kadar) ve çocuk oyun alanı 30 m² (100'e kadar) dışında; sığınak
    içinde (7/a: 10'dan az bölümlü konut yapısı).
  - b: on daire (sekizi odasız, ikişer kişi): 20 kişi, 30 m²'lik sığınağın 20'si dışında.
- **Eski sınamalar:** `emsal_limits_tests`'in toprak altındaki bodrumunda ortak merdiven evi ve kazan dairesi (depo,
  kömürlük sahneleri), kapıcı dairesinin 70 m²'si ve 22/1-j'nin sınır içindeki 3 m²'si artık %30 dışında: beklenenler
  5/8-d ile güncellendi (katlar alanları değişmedi).
- **Ortak süzgeçler:** emsal, zon, alan, taş duvar, otopark ve sığınak, vaziyet planı, kural seti, modül, şerit,
  geçmiş, katalog ve ayar sınamalarıyla 208 masaüstü sınaması yeşil; uygulamanın `blocks_areas`, `arch_ifc`,
  `protocol` sınamaları (52) ve IFC sınamaları (70) yeşil; olguların altı değişmedi; SDK şemaları (`outside`'ın
  belgesi) yeniden üretildi.
- **Resimler:** `.run/gui-evidence/bodrum-listesi-8148/`: gömülü bodrumun ve yarı gömülü otoparkın Emsal ve TAKS
  tablosu, açık ve koyu.

## §6. Kalanlar

- b'nin öbür mahallerinin asgari alanları (merdiven, şaft, baca, ışıklık, tesisat, depo, jeneratör, enerji odası,
  kömürlük): standartlarına göre hesap.
- 22/1-g: "Bina için gerekli minimum sığınak alanı" emsal harici; gerekli alanı aşan sığınak emsale dahil olmalı
  (bugün bütün sığınak emsal harici sayılıyor) — yapıldı: ADR [8149](8149-shelter-needed-area-22-1-g.md); 5/8-b artık
  22/1-g'nin bıraktığı kısmın hepsini %30 dışına alır (bu ADR'nin b sahnesi 8149'da yeniden çözüldü).
- Listenin eski hâlleri (31.12.2022'den önceki a, 25.7.2019'dan önceki b ve c) kural setinde yok.
- Kısmen açığa çıkan cephe (eğimli arsa): cephe cephe tesviye kotu (8079 §6, 8147 §6).
