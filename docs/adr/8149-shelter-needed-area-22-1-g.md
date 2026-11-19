# ADR 8149: Sığınağın gerekli alanı — PAİY 22/1-g

- **Durum:** uygulandı (19 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13, sıra B15; ADR
  [8148](8148-basement-list-5-8.md) §6 ("22/1-g: gerekli alanı aşan sığınak emsale dahil olmalı").
- **Bağlam:** ADR 8079 (22/1'in sınırları), [8036](8036-parking-and-shelter-provisions.md) (sığınak hesabı: Sığınak
  Yönetmeliği 7, 8/a; `shelter_report`), 8148 (5/8-b: bodrumda sığınağın asgari alanı %30 dışında).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 22/1-g (RG 3.7.2017/30113), 5/8-b; Sığınak Yönetmeliği 7, 8/a (RG
  7.11.2025/33070 ile değişik).
- **Sözleşme:** yeni komut yok. `kentos_domain::zoning::ZoningLimit::Shelter`, `ZoningReport.shelter_over`;
  `kentos_ifc::arch::EmsalArea.shelter`. `kentos.auditFacts` 1'in `limits[].limit`'ine `shelter`.
  `tr.kentos.mimari-mevzuat` 0.37.0. **`.kcad`:** değişmedi. **Web:** dokunulmadı; TODOS.md §0.2'de.

## §1. Sorun

Tablo adı ya da kullanımı Sığınak olan her mahali bütünüyle emsal harici sayıyordu. 22/1-g yalnız "bina için gerekli
minimum sığınak alanı"nı katlar alanına katmaz: gerekli alanı aşan sığınak, sığınak istenmeyen yapıdaki sığınak da
emsale girer. 8148'in 5/8-b'si de bu yüzden gerekli alanı (net) mahalin brütüyle karşılaştırıyordu.

## §2. Araştırma (19 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 22/1-g (1.7.2026'ya kadar işlenmiş birleştirilmiş metin; bentte değişiklik işareti yok) | "Bina için gerekli minimum sığınak alanı" katlar alanına dahil edilmez | Sınır gerekli alandır; aşan emsale (K384) |
| Sığınak Yönetmeliği 8/a (ADR 8036) | Kişi başına en az net 1 m²; konutta bölüm başına 2, 3, 4 kişi; diğerlerinde emsale konu alan ÷ 20; tuvalet, lavabo, mutfak nişi ayrıca | Gerekli alan `shelter_report`'un net alanı; tuvalet ve niş hesaplanmaz (K385) |
| Sığınak Yönetmeliği 7 (ADR 8036) | 10'dan az bölümlü konut yapısı, 1500 m²'den az konut dışı ve karma yapı sığınak aramaz | Gerekli alan 0: sığınak bütünüyle emsale (K384) |
| RG 7.11.2025/33070 | Sığınak Yönetmeliği'nin 6–8. maddelerini değiştirdi | Daha önceki başvuruda önceki hâl bağlar: uzman incelemesi (K386) |
| PAİY 5/8-b | Bodrumda sığınağın "ilgili mevzuat … göre hesap edilen asgari alanları" %30 hesabına girmez | 22/1-g'nin bıraktığı alanın hepsi (K385) |

## §3. Karar

- **K384 — Sınır:** adı ya da kullanımı Sığınak olan emsal harici mahaller binanın gerekli sığınak alanına kadar
  katlar alanı dışında; aşan, alanlarıyla paylaşılıp emsale girer (tabloda "Sığınağın gerekli alanı aşan kısmı, emsale
  eklenen"; IFC zonunda "22/1-g: sığınağın gerekli alanını aşan … m²"). Sığınak Yönetmeliği 7 sığınak istemiyorsa
  gerekli alan 0. Bina programında bağımsız bölüm yoksa kişi sayısı bilinmez: sınır uygulanmaz.
- **K385 — Net ve brüt:** gerekli alan nettir (8/a), emsal mahalin brütünü (dış hat) sayar: sınır, gerekli net alanın
  sığınak mahallerinin brütü / neti oranıyla brüte çevrilmiş hâlidir (duvar payı gerekli alanla birlikte dışarıda).
  Tuvalet, lavabo ve mutfak nişinin alanı hesaplanmaz (sıkı taraf). 5/8-b bodrumda 22/1-g'nin bıraktığı kısmın
  hepsini %30 dışına alır; gerek bilinmiyorsa hiç.
- **K386 — Kural (`emsal`):** 22/1-g satırı sayımı söyler (sığınağın alanı, gerekli alanı, emsale eklenen), geçti.
  Başvuru 7.11.2025'ten önceyse gerekli alan Sığınak Yönetmeliği'nin önceki hâline bağlı: uzman incelemesi. Sınır yok
  ama bağımsız bölüm de yoksa ve Sığınak adlı mahal varsa eksik veri.

## §4. Uygulama

- `crates/native/domain/src/zoning/limits.rs` (`ZoningLimit::Shelter`, `apply`'ın `shelter_need`'i), `zoning.rs`
  (gerekli alan, `shelter_over`, tablo satırı), `zoning/basement.rs` (5/8-b `shelter_known`).
- `crates/shared/ifc/src/arch.rs` (`EmsalArea.shelter`), `crates/native/application/src/arch/arch_ifc.rs`.
- `crates/shared/extensions/src/audit_facts/building.rs` (`EmsalLimit.limit`'in belgesi).
- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/emsal.py` (`siginak`), `denetim.py`, `data/surumler.json`
  (22/1-g'nin 1.10.2017 hâli).

## §5. Sınamalar

- **Kural** (`test_emsal.py`; 22/1-g'nin ve Sığınak Yönetmeliği'nin metninden elle): emsal 50 durum, 4'ü yeni: gerekli
  32 m² (brütüyle), 40 m²'lik sığınağın 8 m²'si emsale → geçti; sığınak istenmiyor, 30 m² emsale → geçti; 1.6.2024
  başvurusu → uzman incelemesi; bağımsız bölümsüz projede Sığınak → eksik veri. Eski 46 durumun satırlarına 22/1-g
  eklendi: sığınak yok, kapsam dışı (başvuru günü belirsiz ve sonuç güne bağlı durumda öbür satırlar gibi eksik veri).
  Bütün `mimari-mevzuat` sınamaları yeşil.
- **Masaüstü** (`basement_list_tests`; 8148'in iki katlı planı, yarı gömülü bodrum):
  - on daire (sekizi odasız): 20 kişi, 20 m² net; sığınak uç bölmede, net 25,7125 m², brüt 30 m²: sınır
    20 × 30 / 25,7125 = 23,33495… m², 6,66504… m² emsale (satır 6.67); kalan 23,335 m² 5/8-b ile %30 dışında (satır
    23.33).
  - iki daire (7/a): sığınak bütünüyle emsale (satır 30.00), 5/8-b'nin satırı yok.
  - Denetim'in satırları: kapıcı dairesi sahnesinde 22/1-g kapsam dışı; kural seti satırında kapsam dışı maddeler 35.
- **Resimler:** `.run/gui-evidence/siginak-8149/`: on dairenin sığınağının Emsal ve TAKS tablosu, açık ve koyu.

## §6. Kalanlar

- Sığınağın tuvalet, lavabo ve mutfak nişi (8/a): gerekli alana katılmaları için ölçüleri.
- Sığınak Yönetmeliği'nin 7.11.2025'ten önceki hâli kural setinde yok.
- 5/8-b'nin öbür mahallerinin asgari alanları (8148 §6).
