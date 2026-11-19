# ADR 8084: Plan notu katmanı — ulusal, belediye, parsel

- **Durum:** ilk dilim uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 ("plan notu katmanı
  (ulusal → belediye → parsel)"); W4 §2 ve §4/4 ("Katmanlar: ulusal → belediye → parsel plan notu").
- **Bağlam:** ADR [8034](8034-regulation-stairs-shafts-elevators-local-rules.md) (belediye katmanı, çelişki),
  [8078](8078-regulation-rule-set-by-application-date.md) (kural seti), [8083](8083-dated-storey-height-and-doors.md)
  (kat yüksekliği pakette, iki katmanın yargısı `katman`), [8030](8030-emsal-taks-zoning-badge-and-table.md) (İmar durumu),
  [8072](8072-buildings-shared-basement-location.md) §5 (parselin plan kararları CBS'ten, salt okunur).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 28/1, 69/2, 69/3 (RG 3.7.2017/30113; 69/2'ye ek cümle 11.7.2021/31538);
  Mekânsal Planlar Yapım Yönetmeliği 7/1-b, 24/4, 32/5 (RG 14.6.2014/29030).
- **Web:** dokunulmadı (ADR 0228); TS sözleşme türleri yeniden üretildi (`ArchSettings`, `PlanNote`, `PlanNoteKey`).
  **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm 11 (`settings.arch.planNotes`). **Sözleşme:** `ArchSettings`'e
  `planNotes`; `kentos.auditFacts` 1'in `building` bölümüne isteğe bağlı `planNotes`.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 69/2 (derlenmiş metin) | Birinci, ikinci, altıncı, yedinci, sekizinci ve dokuzuncu bölümler, geçici maddeler, 19 ve 20 planlarla ve idarelerin yönetmelikleriyle değiştirilemez; planlarda bunlara aykırı hükümler uygulanamaz | 28–39 (beşinci bölüm, binalar) planla değiştirilebilir; plan notu bu maddelerde bağlar |
| PAİY 69/3 | Değiştirilemeyen hükümler saklı kalmak kaydıyla, idarelerin imar yönetmelikleri uygulama imar planında aksine bir hüküm bulunmadığı takdirde uygulanır | Sıra: parselin planı → belediye yönetmeliği → PAİY |
| PAİY 28/1 | "Kat yükseklikleri uygulama imar planında daha fazla belirlenmemiş ise … en fazla …" | Kat yüksekliği planın notuna açıkça bırakılmış: ilk tüketici |
| MPYY 7/1-b, 32/5 | "Planlar; pafta, gösterim, plan notları ve plan raporu ile bir bütündür"; planlar notları ve raporlarıyla birlikte onaylanır | Plan notu planın kararıdır, yorum değildir |
| MPYY 24/4 | Uygulama imar planında yapılanma koşulları (bina yüksekliği, yapı yaklaşma mesafeleri …) belirlenir | Sıradaki konular (§6) |
| Yalova 1/1000 UİP plan hükümleri | Zemin katında ticaret yapılabilen konut alanlarında ve Ticaret+Konut parsellerde zemin kat yüksekliği 4,00 m'yi geçemez | Zemin kat yüksekliği ayrı bir konu |
| Avcılar RNİP plan notları | Açıklanmayan hususlarda meri yönetmelik hükümleri ve meri plan notları geçerlidir | Notu olmayan ölçüde yönetmelik okunur |
| Otomatik ruhsat denetimi araçları (nomic.ai, InspectMind eğitim notu) | Belediyenin eklemeleri ve değişiklikleri kullanıcıdan alınır ya da araçta tutulur; eyaletinkini geçersiz kılar ya da tamamlar; imar koşulları ayrı katmandır | Plan notu projenin verisidir; KentOS metni yorumlamaz |

- **Plan notunun metni okunmaz:** CBS'ten gelen plan notları (ADR 8072 §5) serbest metindir; "Kat yüksekliği 3.50 m'dir"
  gibi bir cümleyi sayıya çevirmek tahmindir (CLAUDE.md "Değer uydurulmaz"). Değeri planı okuyan mimar yazar, notun yeri
  ("UİP 12.3.2019, plan notu 7") değerin yanında durur.

## §2. Karar

- **K87 — Plan notları projenin verisidir** (`ArchSettings.plan_notes`, `PlanNote { key, value, source }`): konu
  (`PlanNoteKey`), değer (m) ve kaynak. Denetim: değer 0'dan büyük, en çok 20 m; kaynak boş değil, en çok 200
  karakter; her konu bir kez; en çok 50 not (`PlanNote::list_problem`, `invalid_arch`).
- **K88 — Sıra parsel → belediye → ulusal:** plan notunun değeri bulunan ölçüde o bağlar (69/3; 28/1 açıkça plana
  bırakır). Yönetmeliklerin değeri farklıysa iletide yanında yazılır ("— Plan notu (UİP 12.3.2019, plan notu 7); PAİY
  28/1-c (1.10.2017): 3.60 m"). Çelişki yoktur: plan, değiştirebildiği maddede kararı verir. 69/2'nin saydığı
  değiştirilemeyen maddelere not açılmaz (ilk dilimin konuları beşinci bölümdedir).
- **K89 — İlk konular kat yüksekliği** (`groundStoreyHeight`, `storeyHeight`, en çok): zemin kat notu ±0,00'daki (en
  alttaki zemin ya da normal) katın, kat yüksekliği notu öbür zemin ve normal katların (zemin katın kendi notu yoksa
  onun da) sınırıdır. Notun bağladığı katta 28/1-c ve Sivas 28/1-c kapsam dışıdır.
- **K90 — Plan notu bölgeden bağımsızdır:** konut projesi olmayan binada da okunur (yalnız notu olan katlar).
- **K91 — Kural setinde notun satırı:** "Plan notu: zemin kat", "Plan notu: kat yüksekliği"; hâli yok, kaynağı
  `gazette`'te, sonucu (geçti, kaldı, eksik veri). Bulgunun `rule`'u "Plan notu · {kaynak}". Plan notu başvuru gününe
  bağlı değildir: bağladığı katın sonucu güne göre değişmez (K61'in denemesi onu eksik veriye çevirmez).
- **K92 — Dosya:** mimari verisi sürüm 11 (`ARCH_WITH_PLAN_NOTES`, ADR 8054 §10'un modül sürümü); belge şeması
  değişmez. Notu olmayan çizim eski KentOS'un okuduğu gibi kalır; notları sürüm 11'den eski diye işaretlenmiş dosya
  reddedilir.

## §3. Uygulama

- **Sözleşme:** `crates/shared/contracts/src/arch/arch.rs` (`PlanNote`, `PlanNoteKey`, `PLAN_NOTES_MAX`,
  `ArchSettings::has_plan_notes`, `problem`'de denetim). `cad.settings.set` belgesi ve kataloğu `planNotes`'u anlatır.
- **Dosya:** `kentos_kcad::ARCH_WITH_PLAN_NOTES`, `uses_plan_notes`, `arch_version`; okuyucunun reddi.
- **Olgular:** `BuildingFacts.plan_notes` (`PlanNoteFacts`: `key`, `value`, `source`), ölçü `measure`'da.
- **Paket** (`tr.kentos.mimari-mevzuat` 0.8.0): `katman.PlanNotu`, `katman.bulgu(..., plan=)`; `kat_yuksekligi`
  katın notunu bulur (`plan_notu`), satırlarını ve notlarını yazar.
- **Masaüstü:** Proje ayarları › Mimari'de "Plan notları" grubu (`shell/project/plan_notes.rs`): satır başına Konu,
  Değer (m), Kaynak ve sil; Plan notu ekle; tutmayan satır Kaydet'i bekletir ve altında söylenir.

## §4. Sınamalar

- **Sözleşme:** `plan_notes_are_checked_and_written_as_the_parcel_s` (geçerli iki not; değer 0, boş kaynak, aynı konu
  iki kez reddedilir; JSON `planNotes`; yoksa yazılmaz).
- **Dosya:** `arch_plan_notes::the_plan_notes_round_trip_under_arch_data_11`.
- **Paket** (`fixtures/standards/v1/kat-yuksekligi.json`'a 7 durum, 28/1 ve 69/3'ten elle): zemin kat notu 4,50 m
  PAİY'nin 4,00'ının üstünde geçer, 4,70 m kalır; katlarda 3,00 m (PAİY'den sıkı) iki katı da kaldırır; Sivas'ın 3,60'ının
  önüne geçen 3,80 m; büro binasında not okunur, yalnız zemin kat notu yalnız zemin katı okur; tarihsiz başvuruda notun
  bağladığı kat eksik veri olmaz.
- **Masaüstü** (`plan_notes_tests`, `dated_sizes_tests`'in iki katlı evi, 1.8.2023): kaynağı boş not Kaydet'i bekletir;
  iki not kaydedilir, olgulara geçer; zemin kat 3,80 m notun 3,50'sinden kalır (28/1-c'nin 3,60'ı yanında), 1. kat notun
  4,00'ı içinde; satırlar "Plan notu: zemin kat" kaldı, "PAİY 28/1-c" kapsam dışı, "Plan notu: kat yüksekliği" geçti;
  notlar silinince iki kat 28/1-c'den kalır.
- **Resimler:** `.run/gui-evidence/plan-notlari-8084/` (Plan notları grubu ve Denetim; açık ve koyu).

## §5. Arayüz

- Proje ayarları › Mimari, İmar durumu'nun altında "Plan notları": başlık satırı (Konu, Değer (m), Kaynak), satırlar,
  "Plan notu ekle", açıklama ("KentOS notun sözünü okumaz: değeri ve notun yerini planı okuyan yazar").
- Denetim'de bulgu Mevzuat grubunda; kural seti satırında "Plan notu: zemin kat kaldı".

## §6. Sıradaki adımlar

- **Başka konular:** bina yüksekliği ve kat adedi — yapıldı: ADR [8098](8098-plan-building-height-and-storeys.md) (5/11;
  CBS kararlarının `Hmax`'ı ve `KatAdedi`'yle bağ ve 34/1'in "uygulama imar planına göre kat adedi" orada §6); çatı
  eğimi, mahya ve saçak (40, 42; Rust'taki kurallar taşınınca — kurallar ADR [8159](8159-roof-rules-in-the-package.md)'da
  pakete taşındı, notlar ADR [8160](8160-plan-roof-notes.md)'da), bodrumun kullanımı, çıkmalar; subasman kotu (4/1-yyy'nin
  aksine hükmü) — yapıldı: ADR [8158](8158-plan-subasman-notes.md).
- **Plan kararlarının değerleri:** ADR 8072 §5'in kararları (KAKS, TAKS, KatAdedi, Hmax) kaynağıyla İmar durumu'na ya da
  plan notlarına bir onayla alınsın — yapıldı: ADR [8140](8140-plan-decision-values-into-zoning.md).
- **Planın onay günü:** notun planının onay günü başvuru gününden sonraysa uyarı; planın değişikliği sürümlensin.
- **Bölge türü:** 28/1-a, b, ç ve 28/1-c'nin konut bölgesi varsayımı [ADR 8085](8085-zone-storey-heights.md)'te yapıldı.
