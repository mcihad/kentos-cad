# ADR 8095: Taşıyıcı taş duvarların kalın kısmı — PAİY 5/8 ve 20/8-j

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8094](8094-structural-system-traditional-cut-stone.md) §6 ("kalın taş duvarların payı", K133).
- **Bağlam:** ADR [8094](8094-structural-system-traditional-cut-stone.md) (taşıyıcı sistem beyanı, 22/1-m'nin iki hâli),
  [8090](8090-declared-load-bearing-and-escape-roof.md) (duvarın taşıyıcılık beyanı), [8082](8082-fire-resistance-of-elements.md)
  (dış duvar: katın gövdelerinin dış çizgisinde), [8030](8030-emsal-taks-zoning-badge-and-table.md) ve
  [8069](8069-room-zone-far-model.md) (mahal mahal brüt kontur, emsal zonları, K41), [8079](8079-emsal-article-22-limits.md).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 5/8 (RG 31.12.2022/32060 ek ibare), 20/8-j ve 22/1-m (RG 9.10.2021/31623),
  22/1-a (taban alanına dahil edilmeyenler katlar alanına da girmez).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building` bölümüne
  `masonry` (duvarların payı, taban alanı payı, kalın taş duvarlar, beyansız kalın duvarlar).

## §1. Araştırma (17 Kasım 2026)

Metinler ADR 8094 §1'dekilerdir (derlenmiş PAİY, 9.10.2021 ve 31.12.2022 değişiklik yönetmelikleri).

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 20/8-j (9.10.2021) | Taşıyıcı yığma taş dış duvarların tasarım gereği genişliği 25 cm'den fazla olan kısımları taban alanına dâhil edilmez | Dış duvarın payı taban alanından düşülür (K136) |
| PAİY 22/1-a | Taban alanına dâhil edilmeyen kullanımlar katlar alanına dâhil edilmez | Dış duvarın payı katlar alanından da düşülür |
| PAİY 22/1-m (9.10.2021–30.12.2022) | İç duvarların 12,50 cm'den, dış duvarların 25 cm'den fazla kısımları katlar alanına dâhil edilmez (%30 içinde) | O dönemin başvurusu uzman incelemesi (K138) |
| PAİY 5/8 (31.12.2022'den) | Aynı kısımlar "bu hesaba dâhil edilmeksizin emsal haricidir" (%30 dışında) | Tablo bugünkü hâliyle %30 dışında sayar (K135) |
| Belediyelerin hesap yöntemi (web araması, 17 Kasım 2026) | Kalın kısmın alanının nasıl ölçüldüğünü söyleyen yetkili bir kaynak bulunmadı | KentOS'un ölçüsü açıkça tanımlanır (K134), uzman onayına açık |

- **Ölçünün okuması:** "genişliği 25 cm'den fazla olan kısımları" duvarın 25 cm'yi aşan kalınlığıdır. Dış duvarda bu, iç
  yüzü yerinde kalan ve 25 cm'ye inceltilen duvarın dışta kalan şerididir: dikdörtgen planda 2e(W+H) − 4e² (e kalınlığın
  25 cm'yi aşanı). İç duvarda iki odanın ortak duvarıdır; ekseni yerinde kalan ve 12,50 cm'ye inceltilen duvarın iki
  yüzündeki şeritlerdir, her oda kendi yanınınkini sayar.
- **Taşıyıcı ve taş:** beyanlı geleneksel kesme taş yığma binanın (ADR 8094) taşıyıcı diye beyan edilen duvarı
  taşıyıcı yığma taş duvardır; taşıyıcılığı beyan edilmemiş kalın duvar okunmaz (değer uydurulmaz).

## §2. Karar

- **K134 — Payın geometrisi** (`kentos_domain::zoning::masonry`): katın duvar kütlesi mahallerin konturları eksi net
  alanlarıdır.
  - Dış duvarın (gövdesi katın dış çizgisinde, ADR 8082) payı dış yüzü boyunca (kalınlık − 25 cm) derinliğindeki
    şerittir; dış yüz, ötesinde mahal konturu olmayan yüzdür. Şerit köşede bütün olsun diye uçlarından duvarın kalınlığı
    kadar uzatılır, sonra kütleyle kesilir ve taş olmayan dış duvarların gövdesinden ayrılır.
  - İç duvarın payı iki yüzü boyunca (kalınlık − 12,50 cm)/2 derinliğindeki iki şerittir; kütleyle kesilir; dış
    duvarların gövdesinden, öbür iç taş duvarların ortadaki 12,50 cm'sinden ve dış duvarların payından ayrılır.
  - Her mahalin payı kendi konturunun içindeki kısımdır; iki konturun ortak cephe köşesi brüt alanda olduğu gibi yarı
    yarıya.
  - İki yanında mahal olmayan duvar okunmaz; iki yanında mahal olan dış duvar iç duvar gibi okunur.
- **K135 — Katlar alanı (5/8):** mahal konturu eksi payı ile sayılır (emsale dahil ya da harici); pay "Emsal harici, %30
  dışında"dır. 22/1'in sınırları payı düşülmüş alanı okur. Yapı inşaat alanı değişmez.
- **K136 — Taban alanı (20/8-j):** zemin katın sayılan mahallerinden dış duvarların payı düşülür; iç duvarınki kalır.
  Vaziyet planının oturum çizgisi binanın gerçek dış çizgisidir, değişmez.
- **K137 — Tablo ve zonlar:** Emsal ve TAKS tablosu "Taş duvarların kalın kısmı, %30 dışında" ve "Dış taş duvarların
  kalın kısmı, taban alanı dışında" satırlarını yazar (pay yoksa satır da yoktur). Emsal zonunun çokgeni mahallerin
  konturudur; payı açıklamasında yazılır ("5/8: … m² taşıyıcı yığma taş duvarların kalın kısmı emsal harici, %30
  dışında"); zon alanı eksi payı tablonun alanıdır (K41'in yolu).
- **K138 — Koşullar pakette** (`emsal.duvarlar`, paket 0.19.0): 12,50 cm'den kalın, taşıyıcılığı beyan edilmemiş duvar
  eksik veridir (`paiy_masonry_wall_undeclared`; payı emsale dahil sayıldı). Bina zemin dahil 3 kattan çoksa ya da başvuru
  9.10.2021'den önceyse kaldı; 9.10.2021–30.12.2022 başvurusunda duvarlar 22/1-m'deydi (%30 içinde): uzman incelemesi
  (`paiy_masonry_walls_not_exempt`). Satırlar PAİY 20/8-j ve PAİY 5/8; yalnız beyanlı binada.

## §3. Uygulama

- **Alan hesabı:** `zoning/masonry.rs` (`EXTERIOR_KEPT`, `INTERIOR_KEPT`, `walls`, `parts`); `zoning_measured` payı mahal
  mahal böler (`Room::net_shape`, cephe köşeleri); `ZoningReport::{masonry, masonry_footprint, masonry_walls,
  masonry_undeclared}`, `ZoningRoom::walls`; olgular sayımı `Document::zoning_counted` ile bir kez okur (ADR 8096); tablonun iki satırı.
- **IFC:** `EmsalArea::walls`, zonun açıklaması (`kentos_ifc::arch::emsal_state`); `arch_ifc::emsal_areas`.
- **Olgular:** `MasonryFacts` (`walls`, `footprint`, `thick`, `undeclared`); SDK'nın şeması yeniden yazıldı.
- **Paket:** `emsal.duvarlar`; `data/surumler.json`'a PAİY 20/8-j (yok, 9.10.2021) ve 5/8 (1.10.2017, 31.12.2022);
  Denetim'in `emsal` katkısı `paiy_masonry_` kodlarını da alır.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/emsal.json`, 6 yeni, 36 durum; metinden elle): 2026 başvurusunda iki satır geçer;
  beyansız kalın duvar eksik veri; dört katlı binada kaldı; 2022 başvurusunda 20/8-j geçer, 5/8 uzman incelemesi; 2020
  başvurusunda ikisi de kaldı; kalın taş duvarsız binada kapsam dışı.
- **Masaüstü** (`stone_wall_tests`; 10 × 6 m, 50 cm taş dış duvarlar, zemin katta 40 cm taş iç duvar; elle):
  - Beyansız dokuz duvar listede, katlar alanı 120 m², taban alanı 60 m².
  - Taşıyıcı beyan edilince: zemin katta dış şerit 7,75 m² (iki odaya 3,875), iç duvar 2 × 0,1375 × 5 = 1,375 m²
    (yarı yarıya); üst katta 7,75 m²; katlar alanı 103,125 m², %30 dışında 16,875 m²; taban alanı 52,25 m²; yapı inşaat
    alanı 120 m².
  - Olgular, tablonun iki satırı, iki emsal zonunun açıklaması; Denetim'de 20/8-j ve 5/8 geçti; betonarme seçilince pay
    kalmaz. **Resimler:** `.run/gui-evidence/tas-duvar-8095/`.

## §5. Arayüz

- Yeni pencere yok. Emsal ve TAKS tablosunun iki yeni satırı; Denetim'in kural setinde PAİY 20/8-j ve 5/8.

## §6. Sıradaki adımlar

- **Köşede taş olmayan dış duvar:** dış şerit köşeden uzatılınca taş olmayan duvarın köşe parçasına (iki gövdenin de
  dışında kalan kare) değebilir; karma duvarlı köşede pay biraz fazla çıkabilir.
- **İç taş duvarların T birleşimi:** öbür iç taş duvarın çekirdeği düşülür; çekirdeği olmayan (12,50 cm'den ince
  taşıyıcı) duvarla birleşimde tam gövde düşülür.
- **Sivas 22/k** (yerel yönetmeliğin kesme taş yığma bendi, toplamı %30'u geçmemek üzere) — yapıldı: ADR
  [8096](8096-sivas-22k-thick-walls-within-30.md).
