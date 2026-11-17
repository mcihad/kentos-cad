# ADR 8089: Asma tavan — Ek-3/B 11, alt taraftan EI 30

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8082](8082-fire-resistance-of-elements.md) §6 ("asma tavan ve yangın kesici (EI 30)").
- **Bağlam:** ADR [8082](8082-fire-resistance-of-elements.md) (Ek-3, beyan edilen sınıf),
  [8087](8087-shafts-elevators-roofs.md) (çatılar), [8070](8070-missing-building-elements.md) §10 (kaplama: döşemenin
  `covering` rolü, IFC türü CEILING asma tavandır), [8068](8068-fire-performance-material-declarations.md) (beyan).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik Ek-3/B 11 (RG 19.12.2007/26735), 26/4, 29/7.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in
  `building.elements[].kind`'ına `ceiling`.

## §1. Araştırma (17 Kasım 2026)

Metin ADR 8082'nin yönetmelik kopyasından (Ek-3/B tablonun sayfası, 26 ncı madde) okundu.

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Ek-3/B 11 | Asma tavan: EI 30, deney yüzeyi alt taraftan | IFC türü CEILING olan kaplama (KentOS'un asma tavanı, ADR 8070 §10) en az E ve I ile 30 |
| Ek-3/B'nin sabit satırları (4-a, 8, 9, 10, 11, 12) | Süreyi kendisi verir, Ek-3/C'ye göndermez | Asma tavanın 30 dakikası Ek-3/C'nin sütunundan bağımsız: ev satırının süre vermediği (---) yükseklikte de istenir |
| BYKHY 26/4 | Ayrık nizamda müstakil konutlar dışındaki binaların tavan kaplamaları ve asma tavanlarının malzemesi en az zor alevlenici | Malzemenin yanıcılık sınıfı (TS EN 13501-1, Ek-2) modelde beyan edilmiyor: okunmaz (§6) |
| Ek-3/B 10 | Yangın kesici EI 30 | Modelde yangın kesici öğesi yok: okunmaz (§6) |
| ADR 8070 §10, `Covering` | Kaplamanın IFC 4.3 türü; `CEILING` asma tavan, odanın tavanı türetilmiş nesne | Tür adla değil IFC türüyle okunur: adı "Tavan kaplaması" olan CEILING da asma tavandır |

## §2. Karar

- **K114 — Asma tavan Ek-3'ün öğesidir** (`Document::fire_elements`, tür `ceiling`): IFC türü CEILING olan her kaplama;
  katı kendi katı. Döşeme kuralları ona uygulanmaz (döşeme kaplama değildir).
- **K115 — Süresi sabit** (`dayanim`, paket 0.13.0): alt taraftan EI 30 ("Ek-3/B 11 asma tavan (alt taraftan)"); Ek-3/C
  bakılmaz, binanın kullanımı Ek-3/C'nin sınıfına okunmuyorsa Ek-3 bütün olarak eksik veri kalır (öbür öğeler gibi).
  Bulgu kat, tür ve sorun başına birdir.

## §3. Uygulama

- **Alan:** `fire_elements.rs` (`ceiling`). **Olgular:** `ElementFacts`'ın belgesi; şema yeniden yazıldı (örnek projede
  asma tavan yok, olgu fikstürü değişmedi).
- **Paket:** `dayanim.py` (`_TUR`, döngünün başında sabit istek, kapsam dışı notları); `tr.kentos.mimari-mevzuat` 0.13.0.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/dayanim.json`, 4 yeni durum, metinden elle): apartmanda asma tavan EI 30 geçer, EI 15
  kalır, beyansız eksik veri; 24 m'lik tek konutlu evde Ek-3/C süre vermezken kolonun süresi okunmaz ama asma tavanın
  EI 15'i kalır.
- **Masaüstü** (`resistance_tests::a_suspended_ceiling_is_read_by_ek_3_b_11`, iki katlı ev, zemin katta 12,5 mm Alçıpan
  asma tavan 2,60 m'de): olgularda bir `ceiling`, döşeme sayısı değişmez; beyansız eksik veri, EI 15 kalır, EI 30 geçer.
- **Resimler:** `.run/gui-evidence/asma-tavan-8089/` (Zemin kat ve Denetim'de asma tavanın bulgusu; açık ve koyu).

## §5. Arayüz

- Yeni pencere yok. Bulgu Denetim'in Mevzuat grubunda: "Zemin kat: 1 asma tavan EI 15; Ek-3/B 11 asma tavan (alt
  taraftan) en az EI 30 ister — BYKHY Ek-3".

## §6. Sıradaki adımlar

- **26/4:** malzemenin yanıcılık sınıfı beyanı (TS EN 13501-1; Ek-2/A'nın sınıfları) gelince tavan kaplaması ve asma
  tavan en az zor alevlenici (müstakil konut dışında). [ADR 8091](8091-material-reaction-to-fire.md)'de yapıldı.
- **Ek-3/B 10 yangın kesici:** modelde öğesi yok.
- **Plenum (87/7):** asma tavan arasından geçen tesisat modelde yok.
