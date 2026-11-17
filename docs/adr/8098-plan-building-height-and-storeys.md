# ADR 8098: Planın bina yüksekliği ve kat adedi — plan notu ve PAİY 5/11

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 ("plan notlarının öbür
  konuları"); ADR [8084](8084-plan-note-layer.md) §6 ("bina yüksekliği ve kat adedi").
- **Bağlam:** ADR [8084](8084-plan-note-layer.md) (plan notu katmanı, K87–K92), [8085](8085-zone-storey-heights.md)
  (zemin ve normal katlar, asma kat), [8072](8072-buildings-shared-basement-location.md) §5 (CBS'in plan kararları: `Hmax`,
  `KatAdedi`, salt okunur).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 5/11, 4/1-d (asma kat), 4/1-n (bina yüksekliği), 4/1-ppp (saçak seviyesi),
  4/1-yyy (subasman kotu), 9/1-c (kat adedi binanın kot aldığı noktaya göre), 69/2 (RG 3.7.2017/30113).
- **Web:** dokunulmadı (ADR 0228); TS türü `PlanNoteKey` yeniden üretildi. **`.kcad`:** belge şeması değişmedi; mimari
  verisi sürüm 16 (`settings.arch.planNotes[].key`: `buildingHeight`, `storeys`). **Sözleşme:** `PlanNoteKey`'e iki konu.

## §1. Araştırma (17 Kasım 2026)

Metinler derlenmiş PAİY'den (1.7.2026 derlemesi, `pdftotext`).

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 5/11 | Yeni yapılacak binalarda uygulama imar planında veya planda olmaması hâlinde bu Yönetmelikte gösterilen kat adedi veya bina yüksekliği aşılamaz | Planın değerleri plan notu; aşılırsa kaldı (K143, K145) |
| PAİY 69/2 | Birinci ve ikinci bölüm (5 dahil) planla değiştirilemez | 5/11 değişmez; not yalnız planın değerini verir |
| PAİY 4/1-n | Bina yüksekliği: binanın kot aldığı noktadan saçak seviyesine kadar olan yükseklik | ±0,00'dan saçak seviyesine (K144) |
| PAİY 4/1-ppp | Saçak seviyesi: son kat tavan döşemesi üst kotu | En üst katın kotu + kattan kata yüksekliği |
| PAİY 4/1-yyy | Subasman (zemin kat taban) kotu ±0,00 kotunun altına düşemez, +1,20'nin üstüne çıkamaz | PAİY'nin ±0,00'ı binanın kot aldığı noktadır; projenin ±0,00'ı o diye okunur |
| PAİY 9/1-c | Kat adetleri binanın kot aldığı noktaya göre hesaplanır | ±0,00 ve üstündeki katlar |
| PAİY 4/1-d; ADR 8085 K95 | Asma kat zemin katın parçasıdır | Kat adedine sayılmaz |
| PAİY 28/3 | Tesisat katının 2,00 m'si 60,50 m'yi aşan binada bina yüksekliğinden ve kat adedinden sayılmaz | Okunmaz (§6) |

## §2. Karar

- **K143 — İki yeni konu** (`PlanNoteKey::BuildingHeight`, `Storeys`): "Bina yüksekliği (en çok)" (m; 0'dan büyük, en
  çok 1000) ve "Kat adedi (en çok)" (1–200 tam sayı). Yükseklik notları gibi kaynakları yazılır, her konu bir kez
  (`invalid_arch`). Proje ayarları › Mimari › Plan notları'nda Konu'dan seçilir; değerin örneği konuya göre (9,50 · 3 ·
  4,00).
- **K144 — Ölçüler** (`bina_yuksekligi`, paket 0.21.0): bina yüksekliği ±0,00 ve üstündeki en üst katın (asma kat ve
  adında Çatı geçen çatı arası dışında) kotu artı kattan kata yüksekliği; en üst katın yüksekliği yoksa eksik veri. Kat
  adedi aynı katların sayısı.
- **K145 — Satırlar ve bulgular:** "Plan notu: bina yüksekliği", "Plan notu: kat adedi" (hâli yok, kaynağı notun yeri,
  K91); aşan `paiy_building_height_over` (en üst katın mahalleri), `paiy_storey_count_over` (sınırın üstündeki katların
  mahalleri); ileti "— Plan notu (…); PAİY 5/11". Notu olmayan ölçünün satırı yazılmaz: yönetmeliğin kendi kat adedi
  (plansız alanlar) bu kuralın dışında.
- **K146 — 34/1'in planın kat adedi:** asansör kuralı "uygulama imar planına göre kat adedi"ni önce parselin plan
  notundan (`storeys`), yoksa künyenin `kat_adedi`'sinden (dijital teslim kapsamının "bodrum dışındaki kat adedi"si),
  ikisi de yoksa çizimden okur; ileti nereden okunduğunu söyler ("kat adedi plan notundan"). Planın kat adedinin tek
  kaynağı plan notudur; künye kapsam kararının girdisidir.
- **Dosya:** mimari verisi 16 (`ARCH_WITH_PLAN_LIMITS`): yeni konulu notu taşıyan dosyayı eski KentOS sürüm 16 diye
  reddeder.

## §3. Uygulama

- **Sözleşme:** `PlanNoteKey::{BuildingHeight, Storeys, unit, is_plan_limit}`, `PLAN_BUILDING_HEIGHT_MAX`,
  `PLAN_STOREYS_MAX`, `ArchSettings::has_plan_limits`; `PlanNote::list_problem` konuya göre.
- **Dosya:** `kentos_kcad::{ARCH_WITH_PLAN_LIMITS, uses_plan_limits}`, okuyucunun reddi.
- **Masaüstü:** `plan_notes.rs` (sütun "Değer", konuya göre örnek ve birim, açıklama).
- **Paket:** `bina_yuksekligi.py`, Denetim'in `bina-yuksekligi` katkısı; `mimari.KURALLAR`; `asansor._plan` plan notunu
  önce okur.

## §4. Sınamalar

- **Sözleşme:** `the_plan_s_building_height_and_storey_count_are_notes_of_their_own`. **Dosya:**
  `the_plan_s_building_height_round_trips_under_arch_data_16`. **Komut durumları** (`cad.settings.set.json`, 2 yeni, 131).
- **Paket** (`fixtures/standards/v1/bina-yuksekligi.json`, 8 durum; metinden elle): 9,00 ≤ 9,50 geçer; 9,60 kalır; çatı
  arası sayılmaz; en üst katın yüksekliği yoksa eksik veri; bodrum ve asma kat sayılmaz; dört kat > 3 kalır; iki not
  birlikte; notsuz satır yok. `asansor.json`'a 1 yeni (26): künye 4, plan notu 3 kat → 11.7.2021 hâlinde asansör yeri
  gerekir, "kat adedi plan notundan".
- **Masaüstü** (`plan_limits_tests`, `dated_sizes_tests`'in evi; elle): kesirli kat adedi Kaydet'i bekletir; 7,00 m ve 1
  katta 7,60 m ve 2 kat kalır; 8,00 m ve 2 katta ikisi geçer. **Resimler:** `.run/gui-evidence/plan-yukseklik-8098/`.

## §5. Arayüz

- Plan notları: Konu'da "Bina yüksekliği (en çok)" ve "Kat adedi (en çok)"; sütun "Değer".
- Denetim: "Bina yüksekliği 7.60 m > 7.00 m (±0,00'dan “1. kat” katının tavan döşemesinin üstüne) — Plan notu (UİP
  12.3.2019, plan notu 3); PAİY 5/11".

## §6. Sıradaki adımlar

- **Planın değerleri CBS'ten:** ADR 8072 §5'in `Hmax`'ı ve `KatAdedi`'si bir onayla plan notuna alınsın.
- **Tahvil (9/1-c):** planın yalnız birini verdiği durumda öbürüne çevirme (subasman +1,20 ve 28/1'in kat yükseklikleri).
- **Tesisat katı** (28/3) ve **kademeli bina** (9/1, her kademenin kendi kotu) okunmaz.
