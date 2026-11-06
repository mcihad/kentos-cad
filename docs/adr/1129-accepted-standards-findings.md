# ADR 1129: Standart bulgusunun gerekçeyle kabulü

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12`'nin son maddesi “Geçerli istisna
  gerekçeyle saklanabilsin; eski projeyi açmak standardı uygulama izni sayılmasın”.
- **Bağlam belgesi:** ADR 0312 (standart denetimi ve uygulaması), 1002 ve 1100 (ofis paketi), 1128 (CAD-ADV-12 kabulü),
  0589 §3 ve 1260 §2 (topoloji istisnası; MAP-07, `origin/map-07-topoloji-kalite`).
- **Şema:** `.kcad` 190 (dalda geçici numara; birleşmede yeniden numaralanır).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.

## 1. Araştırma

Erişim: 6 Kasım 2026.

| Kaynak | Okunan | KentOS kararı |
|---|---|---|
| Autodesk, [Check Standards Dialog Box](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-Core/files/GUID-7D20B406-3DA3-4D47-971B-4AFD9A516CA9.htm), [To Audit a Drawing for Standards Violations](https://help.autodesk.com/cloudhelp/2020/ENG/AutoCAD-Core/files/GUID-3E786E2E-7F6A-4D89-8B2B-F749F1AC2B85.htm) | “Mark This Problem as Ignored” o anki sorunu yok sayılmış diye işaretler; bir sonraki denetimde gösterilmez. | Kabul, bulgunun kendisine bağlı ve çizimde saklanır. |
| Autodesk, [To Control the Display of Ignored Problems](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-Core/files/GUID-9309B488-C0D2-4C6C-837A-F7CC84BB5DCD.htm), [CAD Standards Settings](https://help.autodesk.com/cloudhelp/2019/ENU/AutoCAD-Core/files/GUID-E03CA0C6-C1EF-4C83-A0C7-B1EC9C484CD2.htm) | “Show Ignored Problems” kapalıysa yok sayılanlar gösterilmez. | KentOS gizlemez: kabul edilen bulgu listede `accepted` olarak kalır ve ayrı sayılır (kör nokta olmasın). |
| Autodesk, [AcStFlags](https://help.autodesk.com/cloudhelp/2021/ENU/AutoCAD-ActiveX-CSP/files/GUID-1F5FBFE0-F741-41A5-9210-B333FB94A81B.htm) | Sorunun uygulamaca mı kullanıcıca mı yok sayıldığı ayrılır. | Kim ve ne zaman (`by`, `at`) çağıran verirse kaydedilir; masaüstü bilgisayarın kullanıcısını ve şimdiki zamanı verir. |
| ArcGIS Pro SDK [MarkAsException](https://doc.esri.com/en/arcgis-pro/latest/sdk/api-reference/topic30040.html), ADR 1260 §2 | Kabul edilebilir hata istisnaya yükseltilir, kaldırılınca yeniden hatadır; işaretin geometrisi ya da kuralı değişince işaret düşer (`lapsed`). | Aynı yaşam döngüsü: açık, kabul edilmiş, düşmüş; gerekçe zorunlu, en çok 500 karakter. |

AutoCAD'in yok sayma işaretinde gerekçe alanı yok; KentOS gerekçeyi zorunlu tutar (madde “gerekçeyle” der).

## 2. Karar: kabulün kimliği ve düşmesi

- `ProjectSettings.standardExceptions: StandardException[]`: bulgunun türü (`kind`, denetimin adıyla), konusu
  (`subject`), katmanı (`layer`), standardın kaynağı (`standard`: bağlı paketin kimliği, yoksa standardın adı) ve
  revizyonu (`revision`), kabul anındaki `expected` ve `found`, gerekçe (`reason`), isteğe bağlı `by` ve `at`. Aynı
  tür, konu ve katman aynı kaynak için bir kez (`standard_exceptions_problem`).
- `cad.standards.check` yeni `origin` (kaynağın kimliği ve revizyonu) alır; her bulguyu kabullerle karşılaştırır: aynı
  bulgu ve kaynak için kabul varsa revizyon ve iki değer aynıysa `state: accepted`, değilse `lapsed`; gerekçe `reason`'da.
  Rapor `accepted` ve `lapsed` sayılarını ayrı verir; bulgular listeden çıkmaz, `fixable` değişmez. Kabulü olmayan bulgu
  eskisi gibidir (alanlar yazılmaz; eski ortak vakalar değişmeden geçer).
- Düşme: paketin yeni revizyonu (kaynak aynı, revizyon başka) ya da çizimin değeri (örn. katman rengi) değişince.
  Kabul kendiliğinden silinmez; yeniden kabul ya da geri alma kullanıcının işidir.

## 3. Karar: `cad.standards.exceptions` v1

- `operation` `accept` | `withdraw`; `findings` (tür, konu, katman); standart denetimdeki gibi (`name` ya da
  `standard`, `resources`, `origin`); `accept` için `reason` (zorunlu), `by`, `at`; `expectedRevision`.
- `accept`: bulgu bugünkü denetimde olmalı; aynı gerekçe, kim ve zamanla kabul edilmişse dokunulmaz; düşmüş ya da başka
  gerekçeli kabul bugünkü değer ve revizyonla yeniden yazılır. `withdraw`: kaynağa karşı kabul kaldırılır (bulgu artık
  yoksa da).
- Retler sırayla: `invalid_input` (bulgu yok ya da iki kez; geri almada `reason`/`by`/`at`; 500'den uzun gerekçe; boş ya
  da satır sonlu `by`/`at`), `reason_required`, standardın retleri, `invalid_revision`, `revision_conflict`,
  `finding_not_found` (`findings[i]`). Hepsi ya da hiçbiri.
- Kabuller proje ayarıdır: çizimi kirli yapar, geri alma adımı değildir (topoloji istisnaları gibi, ADR 0589 §3).
- Katalog, yetenekler, Python SDK, MCP (katalogdan), manifest, masaüstü TITLES.

## 4. Karar: masaüstü

Çizim standardı penceresinde bulgu seçilince altında **Gerekçe** alanı, **Gerekçeyle kabul et** (gerekçe yazılı ve
bulgu kabul edilmemişse) ve **Kabulü geri al** (kabul ya da düşmüş kabul varsa). Satır “Kabul edildi · …” ya da
“Kabul düştü · …” der, simgesi bilgi ya da uyarıdır; ayrıntı gerekçeyi ve düşmenin nedenini söyler; özet cümlesi kabul
edilen ve düşen sayılarını ekler. Bağlı paket seçiliyse kaynak paketin kimliği ve revizyonudur, birleşik standartta
`provenance.rules`.

## 5. “Eski projeyi açmak uygulama izni sayılmasın”

Açılış hiçbir şey uygulamaz: `office_profiles.rs`
`opening_a_project_bound_to_another_office_does_not_apply_it` (ADR 1128). Kabuller de açılışta değişmez; düşmüş kabul
yeniden değerlendirilene kadar `lapsed` kalır.

## 6. Kanıt

- `scripts/fixtures/standards_exceptions_cases.py` (KentOS kodu yok; bulgular `standards_cases.py`'nin):
  `cad.standards.exceptions.json` 32 durum, `cad.standards.check.exceptions.json` 21 durum.
- `.kcad` şema 190: `standard-exceptions.json/.kcad`, bozuklar `standard-exceptions-{in-schema-187,unknown-field,
  reason,duplicate}.kcad`; `kcad_v2_reference.py` ve `tools/kcad/kcad.py` birlikte.
- `office_profiles.rs` `an_accepted_finding_survives_a_reopen_and_lapses_on_the_next_revision`: kabul, kaydedip açınca
  `accepted`; ikinci ofisin revizyonunda `lapsed`.

## 7. Sınırlar

- Uygula (`cad.standards.apply`) kabul edilmiş bulguyu ayırt etmez: katmanın görünüşünü düzelt seçiliyse kabul
  edilen katman rengini de düzeltir, kabul o zaman düşer. Uygulamanın kabulleri atlaması ayrı iştir.
- Birleşik standartta kabul `provenance.rules`'un kaynağına bağlıdır (bir bulgunun kendi tanımının kaynağına değil).
