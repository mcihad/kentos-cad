# ADR 8147: Bodrumdaki ortak mekânlar — PAİY 22/1-j

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8079](8079-emsal-article-22-limits.md) §6 ("22/1-j: bodrumdaki yönetim, toplantı, spor ve sosyal mekânlar") ve
  [8094](8094-structural-system-traditional-cut-stone.md) §6 ("%5'in tabanı kaynak bekliyor").
- **Bağlam:** ADR 8079 (22/1'in sınırları: 22/1-l, -k, -ç, -f; toprak altındaki kat), [8069](8069-room-zone-far-model.md)
  (emsal zonları, sınırın aşanı zonun açıklamasında), [8030](8030-emsal-taks-zoning-badge-and-table.md) (Emsal ve TAKS
  tablosu), [8125](8125-room-emsal-state-type-and-storey.md) (mahalin kendi emsal durumu). Kurallar Python'dadır, Rust
  olguları verir (bim-plan K10).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 21/2, 21/3, 22/1-j (RG 25.7.2019/30842 ile değişik), 5/8.
- **Sözleşme:** yeni komut yok. `kentos_domain::zoning::ZoningLimit::Amenity`, `ZoningReport.amenity_over`,
  `amenity_allowed`, `AMENITY_SHARE`, `AMENITY_AREA`; `kentos_ifc::arch::EmsalArea.amenity`. `kentos.auditFacts` 1'in
  `limits[].limit`'ine `amenity` (alan yapısı aynı). `tr.kentos.mimari-mevzuat` 0.36.0. **`.kcad`:** değişmedi.
  **Web:** dokunulmadı; web'in emsal tablosuna gerekenler TODOS.md §0.2'de.

## §1. Sorun

22/1-j okunmuyordu: toprak altındaki bodrumda binada oturanların spor salonu, toplantı ve yönetim odası, sosyal
mekânı bütünüyle emsale dahil sayılıyordu. ADR 8079 ve 8094 "toplamda emsal hesabına konu alanın %5'i"nin tabanını
(çizimin katlar alanı mı, planın izin verdiği alan mı) kaynak bekleyen iş diye bırakmıştı.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 22/1-j (RG 25.7.2019/30842 ile değişik; 1.7.2026'ya kadar işlenmiş birleştirilmiş metin, alomaliye.com) | Bütün cepheleri tamamen toprağın altında kalan bodrum katlar ile kısmen açığa çıkan yol cephesi haricindeki bodrum katlarda; tek başına bağımsız bölüm oluşturmayan, bir bağımsız bölümün eklentisi olmayan, ticari amaç içermeyen, yalnız binada oturanlara yönelik; toplamda emsal hesabına konu alanın %5'ini ve 1000 m²'yi geçmeyen ortak alan niteliğindeki yönetim ve toplantı odaları ile spor ve sosyal mekânlar katlar alanına dahil edilmez | K376, K377, K378 |
| PAİY 21/2 | "22 nci madde uyarınca emsal hesabına konu edilmeyen alanlar" | Yönetmelik "emsal hesabına konu" sözünü emsale sayılan alan için kullanır (K376) |
| PAİY 21/3 | "Emsal hesabına dâhil edilmeyen alanlar … emsal hesabına konu alan haline getirilemez" | Aynı: emsal hesabına konu alan, katlar alanıdır (K376) |
| PAİY 5/8 | "parselin toplam emsale esas alanının % 30'unu aşamaz" | "Emsale esas alan" izin verilen alandır, ayrı bir terim; %5'in tabanı değil (K376) |
| ADR 8036 (8/3), 8030 (57/3) | Otopark hesabında ve onaylı projenin tablosunda "emsale konu alan" sayılan alandır | Aynı okuma (K376) |
| RG 1.7.2026/33297 | 22'ye ikinci fıkrayı ekledi (umumi binada ortak alan); birinci fıkranın (j) bendine dokunmadı | Bendin 25.7.2019 hâli bugün de yürürlükte |
| Web araması (17 ve 19 Kasım 2026) | %5'in tabanı için bakanlığın yorumu bulunmadı | Yönetmeliğin kendi kullanımı dayanaktır (K376) |

Bendin 25.7.2019'dan önceki hâlleri kural setinde yoktur: daha eski başvuruda metin bilinmez, bulgu uzman incelemesidir
(ADR 8078).

## §3. Karar

- **K376 — %5'in tabanı katlar alanıdır:** sınır, katlar alanının %5'i ile 1000 m²'nin küçüğüdür (`amenity_allowed`).
  Katlar alanı emsale dahil mahallerle öbür sınırların (22/1-l, -k, -ç, -f) aşanından oluşur; bu sınırın kendi aşanı ve
  5/8'in %30'u aşanı ona dayandığından tabana girmez (döngü olmaz). Taban nihai katlar alanından hiç büyük olmadığından
  sınır sıkı taraftadır.
- **K377 — Mahaller:** iç mahal; adı ya da kullanımı (adı emsale dahil sayılan mahalde grubunun adı ya da niteliği)
  Yönetim, Toplantı, Spor ya da Sosyal ile başlayan bir sözcük taşır; bir bağımsız bölümün kendisi ya da eklentisi
  değildir (grubu yok ya da ortak alan); bütün cepheleri toprak altındaki kattadır (üstü binasının ±0,00'ı ya da altı:
  22/1-k'nin okuması, ADR 8079). Böyle mahaller emsal haricidir, toplamları sınırı aşarsa aşan kısım alanlarına göre
  paylaşılıp emsale girer. Başka yerdeki ya da bölümün kendi mahali emsale dahildir. Bendin saymadığı adlar (fitness,
  hobi odası) okunmaz. Kısmen açığa çıkan yol cephesi okunmaz: model tek tesviye kotu taşır, böyle bodrumun mahalleri
  emsale dahil kalır (sıkı taraf).
- **K378 — Kural (`emsal`):** mahaller ortak alan değilse (bina programında ortak alan grubunda değilse) kaldı. Ortaksa
  ticari amaç içermediği ve yalnız binada oturanların kullanımına yönelik olduğu çizimden okunmaz: uzman incelemesi.
  Satır ve bulgu alanı, sınırı ve emsale eklenen kısmı söyler. Emsal ve TAKS tablosunda "Bodrumdaki ortak mekânların
  sınırı aşan kısmı, emsale eklenen" satırı; IFC'nin emsal zonunun açıklamasında "22/1-j: bodrumdaki ortak mekânların
  sınırını aşan … m²".

## §4. Uygulama

- `crates/native/domain/src/zoning/limits.rs` (`ZoningLimit::Amenity`, `AMENITY_SHARE`, `AMENITY_AREA`,
  `amenity_allowed`, `says_amenity`; `apply`'da sınır en sonda, tabanı o anki katlar alanı), `zoning.rs` (mahalin
  `amenity`'si, `amenity_over`, tablo satırı).
- `crates/shared/ifc/src/arch.rs` (`EmsalArea.amenity`, zonun açıklaması), `crates/native/application/src/arch/arch_ifc.rs`.
- `crates/shared/extensions/src/audit_facts/building.rs` (`EmsalLimit.limit`'in belgesi).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/emsal.py` (`ortak_mekan`), `denetim.py`, `data/surumler.json`
  (22/1-j'nin 25.7.2019 hâli).

## §5. Sınamalar

- **Kural** (`test_emsal.py`; 22/1-j'nin metninden elle): emsal 46 bağımsız durum, 5'i yeni: ortak alan grubunda spor
  salonu ve toplantı odası 130 m², sınır 120 m² (katlar alanı 2400 m²), 10 m² emsale eklendi → uzman incelemesi; 80 m²
  sınırın içinde → uzman incelemesi; katlar alanı 30000 m²'de sınır 1000 m² (1100 m², 100 m² eklendi); grupsuz toplantı
  odası → kaldı ("ortak alan değil"); 1.6.2018 başvurusu → metin bilinmiyor, uzman incelemesi. Eski 41 durumun satırlarına
  22/1-j eklendi: böyle mekân yok, kapsam dışı; 25.7.2019'dan önceki başvuruda satır yok; başvuru günü belirsiz ve sonuç
  güne bağlı durumda öbür satırlar gibi eksik veri. Bütün `mimari-mevzuat` sınamaları yeşil.
- **Domain:** `the_common_rooms_limit_is_5_percent_of_katlar_alani_and_at_most_1000_m2` (2400 → 120, 20000 → 1000,
  30000 → 1000 m²).
- **Masaüstü** (`emsal_limits_tests`; 8079'un iki katlı planı, bodrum toprak altında): ortak alanın spor salonu ve
  toplantı odası 60 m²; katlar alanı önce iki dairenin 60 m²'si, %5'i 3 m²: 3 m² dışarıda, 57 m² (her mahale 28,5 m²)
  emsale girer, katlar alanı 117 m²; tablo satırı 57.00; olgularda `amenity`; bodrumun emsal harici zonu 57 m²'yi emsale
  dahil diye adlandırır. Bodrumun üstü ±0,00'ın 1 m üstündeyse ya da mahaller dairenin kendisininse emsale dahil;
  dairenin depo eklentisindeki toplantı odası 22/1-j'nin değil, grubunun niteliğiyle 22/1-k'nin deposudur. Denetim:
  22/1-j uzman incelemesi (25.7.2019); mahaller grupsuzsa kaldı. Kural seti satırında kapsam dışı maddeler 34
  (`regulation_dates_tests`: 22/1-j, bodrumda ortak mekân yok). Olguların altı, IFC sınamaları (70) ve uygulamanın
  `blocks_areas`, `arch_ifc` sınamaları değişmeden yeşil.
- **Resimler:** `.run/gui-evidence/bodrum-ortak-8147/`: Emsal ve TAKS tablosu (57 m²'nin satırı) ve Denetim'in 22/1-j
  bulgusu, açık ve koyu.

## §6. Kalanlar

- Kısmen açığa çıkan yol cephesi (eğimli arsa): cephe cephe tesviye kotu; 22/1-k'nin "bütün cepheleri"yle birlikte (ADR
  8079 §6).
- 5/8'in bodrum listesi (ADR 8079 §6), bir sonraki dilim. Listenin (d) bendi bütün cepheleri gömülü ve ortak alan
  niteliğindeki otoparkları ve "22 nci maddede belirtilen tamamen gömülü ortak alanlar"ı %30 hesabına katmadan emsal
  harici sayar: 22/1-j'nin sınır içindeki ortak mekânları da onlardandır. Bu dilimde tablo onları %30'un içinde sayar
  (sıkı taraf); (d) ile %30'un dışına geçerler — yapıldı: ADR [8148](8148-basement-list-5-8.md).
