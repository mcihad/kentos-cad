# ADR 8093: Çift kanatlı bina girişi ve döner kapı — PAİY 39/1-b'nin bir kanadı, 39/1-ç

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8083](8083-dated-storey-height-and-doors.md) §6 ("39/1-b'nin bir kanadı … 39/1-ç döner kapının yanındaki normal kapı").
- **Bağlam:** ADR [8083](8083-dated-storey-height-and-doors.md) (kapı ölçüleri pakette, tarihli; `leaf: 1.0` anahtarı
  hâllerde vardı ama okunmuyordu), [0777](0777-revolving-pocket-folding-doors.md) (döner kapı).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 39/1-b, ç (1.10.2017 ve 13.8.2025 hâlleri); Sivas İmar Yönetmeliği 39/1.
- **Web:** dokunulmadı. **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in kapılarına `leaves` ve
  `clearFirst`.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 39/1-b (2017, 2025), Sivas 39/1-b | Kapı net (temiz) genişlikleri bina giriş kapılarında 1,50 m'den, kapıların çift kanatlı olması hâlinde bir kanat 1,00 m'den az olamaz | Bina girişinin iki kanatlı kapısında bir kanadın tek başına net geçişi en az 1,00 m |
| PAİY 39/1-ç, Sivas 39/1-ç | Döner kapılar, belirtilen ölçülerde yapılacak normal kapıların yanında ilave olarak bulunabilir | Döner bina giriş kapısı tek başına giriş olamaz: yanında (aynı mahale açılan) döner olmayan bir bina girişi aranır; döner kapının kendi net genişliği okunmaz |
| `OpeningSizes` (ADR 0772, 0777) | `leaves` her kanadın eni; `clear_first` iki kanatlı kapının yalnız ilk kanadı açıkken net geçişi | İlk kanat geniş ya da eşitse onun net geçişi okunur; ikinci kanat genişse tek başına net geçişi ölçülmez: uzman incelemesi (değer uydurulmaz) |

## §2. Karar

- **K127 — Bir kanat** (`kapi_olculeri`, paket 0.17.0): yeri bina girişi (ADR 8083 K84), iki kanatlı, döner olmayan
  kapıda `clearFirst` hâlin `leaf`'ine (1,00 m) karşı, iki katman (PAİY, Sivas) ADR 8083'ün yargısıyla
  (`paiy_door_leaf_below`). İkinci kanat genişse uzman incelemesi.
- **K128 — Döner kapı:** bina girişi olan döner kapının 39/1-b genişliği okunmaz; aynı mahale açılan döner olmayan bina
  girişi yoksa 39/1 kaldı (`paiy_revolving_door_alone`); yanındaki kapının ölçüsü kendi satırında okunur.
- **Olgular:** iki ve daha çok kanatlı kapının kanat enleri (`leaves`) ve ilk kanadının net geçişi (`clearFirst`).

## §3. Uygulama

- **Olgular:** `DoorFacts.leaves`, `clear_first` (`OpeningSizes`'tan); şema yeniden yazıldı.
- **Paket:** `kapi_olculeri._kapi` (bina girişinin kanadı, döner kapının yanı), `kontrol` kapıları `_kapi`'ye verir;
  Denetim'in `kapi-olculeri` katkısı `paiy_revolving_door_` kodunu da alır.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/kapi-olculeri.json`, 5 yeni, 30 durum, metinden elle): 1,10 + 0,60 m kanatlı kapı
  geçer; eşit 0,85 m kanatlar (bir kanat net 0,79 m) kalır; ikinci kanadı geniş kapı uzman incelemesi; döner kapının
  yanında 1,50 m normal giriş geçer; yalnız döner kapı kalır.
- **Masaüstü** (`double_door_tests`, iki daireli binanın ortak holü): 1,80 m'lik iki eşit kanatlı kapı olgularda iki
  kanat, ilk kanadın net geçişi 1,00 m'nin altında, ikisi açıkken 1,50 m'nin üstünde; 39/1-b kalır; 2,40 m'lik 60/40
  bölünmüş kapı geçer. **Resimler:** `.run/gui-evidence/cift-kanat-8093/`.

## §5. Arayüz

- Yeni pencere yok. "Çift kanatlı bina giriş kapısının bir kanadının net genişliği 0.81 m < 1.00 m — PAİY 39/1-b
  (13.8.2025)".

## §6. Sıradaki adımlar

- **İkinci kanadın tek başına net geçişi** `OpeningSizes`'a eklenirse uzman incelemesi kalkar.
- **39/1-d** (yaşlılar için cepheden ayırt edilen bina girişi) çizimde okunmaz; **39/5** (balkon kapısı) ayrı kural.
