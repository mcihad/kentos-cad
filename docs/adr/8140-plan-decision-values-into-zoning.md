# ADR 8140: Plan kararının değerleri — bir onayla İmar durumu'na ve Plan notları'na

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-11 ve BIM-13, sıra B15; ADR
  [8072](8072-buildings-shared-basement-location.md) §5 ("imar durumuna bir kararın değerlerini kaynağıyla almak, kaynağı
  imar durumunda saklamak"), [8084](8084-plan-note-layer.md) §6 ("Plan kararlarının değerleri … bir onayla alınsın"),
  [8098](8098-plan-building-height-and-storeys.md) §6 ("Planın değerleri CBS'ten").
- **Bağlam:** ADR [8030](8030-emsal-taks-zoning-badge-and-table.md) (İmar durumu, emsal ve TAKS), 8072 §5
  (`cad.zoning.parcel`: parselin plan kararları, kaynağı ve notları; salt okunur), 8084 (Plan notları: değer ve kaynak),
  8098 (planın bina yüksekliği ve kat adedi, PAİY 5/11).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 4/1-ll, 4/1-tt, 4/1-cccc, 5/5, 5/11, 55/2, 55/4.
- **Sözleşme:** `Zoning`'e `farSource` ve `coverageSource` (isteğe bağlı; `cad.settings.set`'in `arch.zoning`'i; yeni
  komut yok). `kentos.auditFacts` 1'in `zoning`'ine `farSource`, `coverageSource`. `tr.kentos.mevzuat` 0.7.0.
  **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm 27 (`settings.arch.zoning.farSource`, `coverageSource`).
  **Web:** dokunulmadı (ADR 0228); TS türü yeniden üretildi (`Zoning`), web'in İmar durumu'na gerekenler TODOS.md §0.2'de.

## §1. Sorun

İmar durumu'nun emsali ve TAKS'ı elle yazılıyordu, nereden okunduğu projede yoktu. Belediyenin CBS'inden gelen plan
katmanlarında parselin kararları (8072 §5) gösteriliyor ama hiçbir alana yazılmıyordu: mimar KAKS'ı, TAKS'ı, Hmax'ı ve
kat adedini satırdan okuyup İmar durumu'na ve Plan notları'na yeniden yazıyordu, kaynağını da elle. Bir kesişim tek
başına hukuki karar olmadığı için değerlerin kendiliğinden yazılması da doğru değildi (8072 K19).

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY 4/1-ll, 55/2, 55/4 | İmar durum belgesi parselin plan kararlarını, plan notlarını ve yapılaşma şartlarını içerir; belgeyi idare verir | İmar durumu'nun değerleri belgeden ya da planın kararından; ikisinde de kaynağı yazılır (K349) |
| TKGM Parsel Sorgu kullanım koşulları m. 4 | Sorgu sonucu sunulan bilgi ve belgeler bilgilendirme amaçlıdır, resmî işlemlerde kullanılamaz | CBS'ten okunan karar belge değildir: değer yalnız mimarın düğmesiyle, kaynağıyla alınır; Kaydet onaylar (K352) |
| Belediyelerin e-imar sorguları (İzmir, Çorlu; basın haberleri) | Çevrimiçi imar durumu sorgusu bilgi amaçlıdır; resmî iş için belediyenin onaylı belgesi alınır | Aynı |
| Kent planlama yazılımlarının imar düzenlemesi belgesi (ArcGIS Urban 12.0, "Zoning regulation management") | Kat, yükseklik, FAR ve kapsama oranı imar türünde, örtüde ve parselde tanımlanır; parselde verilen öbürlerinin önüne geçer | İmar durumu parselin kendi değeridir; plan kararı yanında gösterilir, kendiliğinden yazılmaz; fark uyarı renginde (8072) |
| ADR 8072 K18 (`cad.zoning.parcel`) | Değerler yazıldığı gibi; sayılar sayı olarak karşılaştırılır ("1,20" ile "1.2" aynı) | Alınan değer aynı okuyuşla (`zoning_parcel::plan_number`): virgül ya da nokta; birimli, sözlü değer sayı değildir (K350) |
| ADR 8084, 8098 | Plan notu: ölçü, değer, kaynak (en çok 200 karakter); bina yüksekliği en çok 1000 m, kat adedi 1–200 tam sayı | Hmax ve kat adedi bu satırlara; emsalin ve TAKS'ın kaynağı da 200 karakter (K349, K350) |

## §3. Karar

- **K349 — Kaynağın yeri:** İmar durumu'nun emsali ve TAKS'ı nereden okunduğunu taşır (`zoning.farSource`,
  `zoning.coverageSource`): imar durum belgesi ya da alındığı plan kararı. Bir satır, en çok 200 karakter, değeri
  olmadan yazılmaz; boş kaynak yoktur (yazılmaz). Proje ayarları › Mimari › İmar durumu'nda "Emsalin kaynağı" ve "TAKS'ın
  kaynağı" satırları. Parsel alanının kaynağı yok: net parsel alanı plan kararından alınmaz (geometrik ve kayıtlı alan
  ayrıdır, CLAUDE.md §7).
- **K350 — Alınan değerler:** bir kararın KAKS'ı Emsal'e, TAKS'ı TAKS'a, Hmax'ı "Bina yüksekliği (en çok)", KatAdedi'si
  "Kat adedi (en çok)" plan notuna. Yalnız sayı olarak yazılmış ve alanın sınırındaki değer alınır (emsal 0–50, TAKS 0–1,
  yükseklik 0–1000 m, kat adedi 1–200 tam sayı); yüzde olarak yazılmış TAKS ("40"), "Z+3", "serbest" alınmaz, çevrilmez.
  Kullanım, nizam ve çekme mesafeleri alınmaz (İmar durumu'nda alanları yok, §6).
- **K351 — Kaynağın sözü:** planın kimliği, yürürlükte değilse durumu, onay günü ve kararın kimliği: "UIP-1, onay
  2024-05-14, karar K1"; askıdaki planın "UIP-2 (askida), karar K4". Planın adı ve dayanağı (meclis kararı) kararın
  "Kaynak:" satırında kalır: kaynak, plan notunun dar Kaynak sütununda bütün okunsun diye kısadır (planın adıyla
  kesiliyordu). 200 karakteri aşan kaynakla değer alınmaz. Planı çizilmemiş ya da yazılmamış kararın değeri alınmaz:
  kaynağı söylenemez.
- **K352 — Onay:** her kararın satırlarının altında "Değerlerini al" ve neyin nereye yazılacağı ("Emsal 1,20 ve TAKS 0,40
  İmar durumu'na, kat adedi 3 Plan notları'na."). Düğme formu doldurur, kaydetmez: Kaydet `cad.settings.set` ile yazar,
  Vazgeç (Esc) bırakır. Proje ayarı geri alma geçmişinde değildir (çağrı günlüğüne yazılır, bütün proje ayarları gibi):
  güvence Kaydet'ten önceki formdur. Plan notunun ölçüsünün satırı varsa değeri ve kaynağı değişir, yoksa satır eklenir;
  başka kararın değeri kaynağıyla kalır. Form o kararın değerlerini kaynağıyla tutuyorsa düğme kapanır, söz
  "…İmar durumu'nda, … Plan notları'nda." olur. Yeni komut yok: Python ve MCP `cad.zoning.parcel`'i okuyup
  `cad.settings.set`'e yazar.
- **K353 — Denetimde kaynak:** `tr.kentos.mevzuat` 0.7.0'ın `imar-durumu` kuralı aşan emsalin ve TAKS'ın bulgusunda
  kaynağı maddeyle anar: "— PAİY md. 4/1-tt ve 5/5; imar durumu (UIP-1, onay 2024-05-14, karar K1)". Kaynak yazılmamışsa söz eskisi
  gibi. Olgular `zoning.farSource`, `zoning.coverageSource`; eski SDK'da alan yoksa kural kaynaksız çalışır.
- **K354 — Okumaların saklanması:** İmar durumu'nun okumaları (emsal rozeti, vaziyet planı, plan kararları) ve eski kesit
  rozeti çizimin adresi ve revizyonuyla saklanıyordu. Çizim uygulamada hep aynı adreste durur: aynı revizyondaki başka
  bir çizim açılınca öncekinin okuması gösterilirdi; revizyon dışarıdan gelen değişiklikle de değişmez. Okumalar artık
  açılan çizimin oturumu (program çalıştıkça tek) ve kuşağıyla (her değişiklikle, dışarıdan gelenle de) saklanır.

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`Zoning.far_source`, `coverage_source`, `MAX_ZONING_SOURCE`,
  `sources_problem`; `Zoning` artık `Copy` değil: okuyanlar ödünç alır); `crates/shared/kcad/src/lib.rs`, `decode.rs`
  (`ARCH_WITH_ZONING_SOURCES`, `uses_zoning_sources`, eski sürümün reddi).
- `crates/native/application/src/arch/zoning_parcel.rs` (`plan_number` açık); `crates/native/domain/src/zoning.rs`
  (`zoning_measured` ödünç alır).
- `crates/shared/extensions/src/audit_facts/architecture/zoning.rs`, `crates/native/processing/src/audit_facts/
  architecture.rs` (olgular), `python/kentos/ext.py` (`ZoningFacts.far_source`, `coverage_source`).
- `catalog/standards/tr-mevzuat/src/tr_mevzuat/imar.py` (`kaynakli`), `audits/imar-durumu.examples.json` (iki örnek).
- `apps/desktop/src/map/zoning_badge.rs` (`PlanTake`, `plan_takes`; okumalar `kept` ile oturum ve kuşak başına bir kez),
  `apps/desktop/src/arch/section_badge.rs` (aynı anahtar), `apps/desktop/src/shell/project/arch.rs` (kaynak satırları,
  `take`, `holds`, düğmeler), `settings.rs` (`TakePlanValues`).

## §5. Sınamalar

- **Ortak durumlar:** `cad.settings.set` 135 (4'ü yeni: iki kaynak birlikte yazılır; kaynağı olup emsali olmayan, boş
  kaynak ve 201 karakterlik kaynak alanının yolunda reddedilir), `cad.zoning.parcel` değişmedi; ikisi de geçer.
- **`.kcad`:** `the_zoning_s_sources_round_trip_under_arch_data_27`: kaynaklar mimari verisi 27 ile yazılıp okunur;
  aynı dosya `modules.arch` 26 derse (bayt düzeltilip SHA-256 yeniden yazılarak) "sürüm 27'den önce yok" ile reddedilir
  (8135'ten beri ilk sürüm reddi sınaması). Bütün `arch_` sınamaları (28) yeşil.
- **Olgular:** processing'in `audit_facts`'i (20) kaynağın olgulara geçtiğini sınar; olguların altını
  (`fixtures/audit-facts/v1/mimari.json`) değişmedi. SDK'nın taslağı ve olgu şeması yeniden yazıldı (`sdk_pack`,
  `audit_facts` 24 yeşil).
- **Kural:** `apps/jobrun`'un `mevzuat_tests`'i paketi klasöründen paketleyip kuralı gerçek Python'da koşar: kaynaklı
  emsal ve TAKS bulgusu kaynağı anar; 14 yeşil (`KENTOS_AUDIT_CONFINE=report` ile: bulut kutusu Landlock ve seccomp ister,
  macOS'ta kurulamaz; bu ayar olmadan bulut işi sınaması kutuda durur, kuralla ilgisiz). Paketin dokuz örneği (ikisi
  yeni) depodaki Python'la kurala karşı tek tek denendi, hepsi tutar; `kentos kek test` KentOS'un kendi Python'unu ister
  (bu Mac'te kurulu değil, kurmak indirme ister): koşulmadı.
- **Masaüstü** (`zoning_parcel_tests`): K1'in düğmesi notlarından sonra; K1'in emsali 1,2, TAKS'ı 0,4 kaynağıyla, kat
  adedi 3 plan notu; K1 tutulunca düğmesi kapanır, K2 tutulmaz; K2 alınınca emsal 2, TAKS 0,5, bina yüksekliği 15,5
  ayrı satırda, K1'in kat adedi kaynağıyla kalır; Vazgeç projeyi değiştirmez; yeniden alınıp Kaydet `cad.settings.set`
  ile yazar, plan kararlarının farkı kaydedilen emsali okur. Sayı olmayan ya da sınırı aşan değer (TAKS "40", "Z+3",
  "serbest") alınmaz; askıdaki planın kaynağı "UIP-2 (askida), karar K2"; çizilmemiş planın kararı hiçbir şey vermez;
  art arda açılan üç çizim kendi okumalarını alır (K354). Ortak süzgeçlerle (modül, şerit, geçmiş, katalog, ayarlar,
  Öznitelikler, durum çubuğu, mevzuat, vaziyet planı, kesit rozeti, plan notları, emsal sınırları, taş duvarlar) 277
  sınama yeşil; kaynağın kısa sözünden sonra ilgili süzgeçler (166) yeniden yeşil.
- **Uygulama:** `blocks_areas`, `arch_ifc`, emsal ve vaziyet sınamaları 30 yeşil (`Zoning` ödünç alınıyor).
- **Resimler:** `.run/gui-evidence/plan-degerleri-8140/` (`zoning_parcel_tests::take_screens`): kararlar ve Değerlerini
  al; K1 alınınca İmar durumu'nda emsal, TAKS ve kaynakları; K1'in düğmesi kapalı ("…İmar durumu'nda"); iki karar alınınca
  Plan notları. Açık ve koyu.

## §6. Kalanlar

- **Nizam:** kararın Nizam'ı (A, BL, ayrık, bitişik) İmar durumu'nda yok. Yazılırsa 5/6'nın %40'ı (TAKS verilmeyen ayrık ve
  blok nizam), 26/4 (ayrık nizamda müstakil konut) ve 28/3 (bitişik nizam) uzman incelemesinden çıkar (ADR 8134, 8138,
  8139 §6); değer kararından aynı düğmeyle alınabilir — yapıldı: ADR [8141](8141-building-layout-nizam.md) (Yapı nizamı;
  Değerlerini al Nizam'ı da alır).
- **Çekme mesafeleri:** OnCekme, YanCekme, ArkaCekme vaziyet planının kenarlarına (ADR 8032) aynı onayla alınabilir.
- **Planın onay günü:** kaynağın onay günü başvuru gününden sonraysa uyarı (ADR 8084 §6).
- **Web:** İmar durumu'nun kaynak satırları ve Değerlerini al (TODOS.md §0.2).
