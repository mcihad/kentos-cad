# ADR 8151: Duvarların içte uygulanan yalıtımı — BYKHY 20/3

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15 ("mahal
  kaplamalarının içte uygulanan yalıtımı"); ADR [8138](8138-room-finish-coverings.md) §6 ("İçte uygulanan ısı ve ses
  yalıtımı (20/3): duvar stilinin iç yüzdeki yalıtım bileşeni katman katman okunabilir").
- **Bağlam:** ADR 8091 (malzemenin yanıcılık sınıfı, Ek-2/Ç), 8138 (mahallerin duvar kaplaması 20/3), 0785 ve 0791
  (proje malzemeleri, kategorileri), 8082 ve 8124 (dış duvar), 8081 (mahallerin ayrımı: oda ölçümü).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 20/3 (16.3.2015-2015/7401 ile değişik, RG 9.7.2015/29411),
  27 (cephe), Ek-2/Ç.
- **Sözleşme:** yeni komut yok. `kentos_domain::Document::inner_insulations` (`InnerInsulation`, `InsulationLayer`).
  `kentos.auditFacts` 1'in `building`'ine `insulations` (`InsulationFacts`). `tr.kentos.mimari-mevzuat` 0.39.0.
  **`.kcad`:** değişmedi. **Web:** dokunulmadı (Denetim'in kuralı pakettedir; web'in olgularına gerekenler TODOS.md
  §0.2'de).

## §1. Sorun

20/3 duvarlarda iç kaplamaları ve "içte uygulanacak ısı ve ses yalıtımları"nı birlikte sayar. 8138 mahallerin duvar
kaplamasını okudu; duvarın katmanlarındaki yalıtım hiç okunmuyordu: iç yüzde kolay alevlenici EPS'li bir duvar 20/3'ten
geçiyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 20/3 (8138'in okuması) | "Duvarlarda iç kaplamalar ile içte uygulanacak ısı ve ses yalıtımları; en az normal alevlenici, yüksek binalarda ve kapasitesi 100 kişiden fazla olan sinema, tiyatro, konferans ve düğün salonu gibi yerlerde ise en az zor alevlenici" | Yalıtım katmanı duvar kaplamasıyla aynı isteği taşır (K391) |
| BYKHY 27 | Dış cephe kaplamaları ve yalıtım ayrı maddededir | Dış duvarın taşıyıcısının dışındaki yalıtım cephenindir, 20/3'ün değil (K389) |
| Duvar sözleşmesi (`WallFields.components`) | Katmanlar sol yüzden sağa; malzemeye `materialId` ya da adla bağlanır | Katmanın malzemesi kimliğiyle, yoksa adıyla (K389) |
| Proje malzemeleri (`MaterialCategory`) | Taşıyıcı, Yalıtım (ısı ya da ses), Kaplama, Membran, Diğer | Yalıtım kategorili malzemenin katmanı yalıtımdır; Taşıyıcı ilk taşıyıcı katmandır (K389) |

## §3. Karar

- **K389 — İç yalıtım katmanları:** bir duvarın Yalıtım kategorili malzemeye bağlı katmanları (malzemesi kimliğiyle,
  yoksa adıyla ya da kimliğiyle, büyük-küçük harf bir). İç duvarda (dış duvar değil) hepsi içtedir. Dış duvarda iç yüz,
  katının bir mahalinin bulunduğu yandır: duvarın en uzun düz parçasının ortasından iki yana 2 cm adımlarla, genişliği
  artı 50 cm'ye kadar yürünür; iç yüzden ilk Taşıyıcı katmana kadarki yalıtımlar içtedir, dışındakiler cephenin (27).
  Taşıyıcı katmanı olmayan duvarın bütün yalıtımı içte sayılır (sıkı taraf).
- **K390 — İç yüz bulunamazsa:** iki yanında mahal yoksa (ya da ikisinde de varsa) bütün yalıtım katmanları yazılır,
  `unknownSide` ile; kural yetmeyen sınıfı kaldı değil uzman incelemesi sayar (katman cephenin olabilir).
- **K391 — Kural (`kaplamalar.duvar`):** iç yalıtım katmanı en az normal alevlenici; yüksek binada, 100 kişiyi aşan
  salona bakan katman en az zor alevlenici; 100 kişiyi aşan başka adlı mahale bakan normal alevlenici katman uzman
  incelemesi ("gibi yerler", 8138 gibi). Aynı malzemeli ve aynı isteği olan katmanlar tek bulguda
  (`bykhy_wall_insulation_reaction`, duvarlarıyla); sınıfı beyan edilmemişse eksik veri. Satır duvar kaplamalarıyla
  birlikte sayar. Paketin `kaplamalar` katkısı 20/3'ün iki bulgusunu da verir (8138'in duvar kaplaması bulgusu bu
  katkıda eksikti).

## §4. Uygulama

- `crates/native/domain/src/wall_insulation.rs` (yeni), `separations.rs` (`axis_rooms`: mahallerin eksenlere çizgisi,
  ayrımlar ve iç yalıtım birlikte okur).
- `crates/shared/extensions/src/audit_facts/building.rs` (`InsulationFacts`, `BuildingFacts.insulations`),
  `crates/native/processing/src/audit_facts/building.rs` (`insulations`).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/kaplamalar.py` (`duvar`), `denetim.py` (`kaplama_denetimi`).

## §5. Sınamalar

- **Kural** (`test_kaplamalar.py`; 20/3'ün metninden ve Ek-2/Ç'den elle): kaplamalar 39 durum, 5'i yeni: iki duvarın Taş
  yünü A1 → geçti; EPS F (kolay alevlenici) → kaldı, tek bulgu "(2 duvar)"; yüksek binada XPS D-s2,d0 → kaldı (zor
  alevlenici ister); sınıfsız Cam yünü → eksik veri; iç yüzü bulunamayan dış duvarın EPS F'si → uzman incelemesi. Eski 34
  durum değişmedi.
- **Masaüstü** (`wall_insulation_tests`, elle çözülen tek katlı konut): 6 × 4 m'lik dış duvarlar saat yönünün tersine,
  "Dış duvar" stiliyle (katmanları soldan Sıva, EPS, Tuğla, Taş yünü); "Bölme" stilinde Taş yünlü bölme duvar. Domain dış duvarların EPS'ini (iç yüzü sol, bir
  mahal) ve bölmenin Taş yününü (iki mahal) okur, cephenin Taş yününü okumaz; olgularda beş katman. Sınıfsız: 20/3 eksik
  veri, iki bulgu; EPS F, Taş yünü A1: kaldı, "“EPS” iç yalıtımı (4 duvar): malzemesi “EPS” F, kolay alevlenici"; EPS
  B-s1,d0: geçti.
- **Ortak süzgeçler:** kaplama, döşeme kaplaması, malzeme, malzeme zonu, kompartıman (ayrımlar ortak oda ölçümüyle),
  şaft, bitişik kenar, çatı, kural seti, modül, şerit, geçmiş, katalog ve ayar sınamaları yeşil (171); domain'in bütün
  sınamaları, olguların altı, uygulamanın `blocks_areas`, `arch_ifc`, `protocol` sınamaları ve iş çalıştırıcının ulusal
  kural sınamaları yeşil; SDK şeması yeniden üretildi.

## §6. Kalanlar

- Eğri duvarın iç yüzü en uzun düz parçasından okunur; düz parçası olmayan yay duvar okunmaz.
- Duvar kaplamasının (mahal) ve iç yalıtımın aynı duvarda birlikte okunması (kaplamanın arkasındaki yalıtım) ayrıca
  söylenmez.
