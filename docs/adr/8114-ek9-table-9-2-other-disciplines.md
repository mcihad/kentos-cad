# ADR 8114: EK-9 Tablo 9.2 — öbür disiplinlerin modelleri için kalite kontrol formu, paket incelemesinde

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-08'in kalanı ("EK-9 Tablo 9.2"),
  [bim-plan.md](../../plan/bim-plan.md) B13; idare incelemesi B20 (ADR [8104](8104-administration-package-review.md)).
- **Bağlam:** ADR [8074](8074-ek9-from-the-written-file.md) (formun 21 sorusu yazılmış dosyanın olgularından; §8: "EK-9
  Tablo 9.2 … disiplin modelleri geldiğinde"), [8104](8104-administration-package-review.md) (gelen paketin incelemesi), [8101](8101-ifc-underlay-model.md)
  (statik altlık modeller).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Komut:** `file.review.package`'ın çıktısına `table`
  (formun tablosu: `9.1` ya da `9.2`). **Kural paketi:** dijital-proje 0.11.0.

## §1. Araştırma (18 Kasım 2026)

Dijital Proje Yönetmeliği EK-9 (`mevzuat/bim/ek-9-kalite-kontrol-formu.md`, PDF s. 238–241): **9.1** "Teslim edilen mimari
disipline ait BIM modeli/modelleri için Tablo 9.1'deki ve diğer disiplinlere ait BIM modelleri için Tablo 9.2'deki Model
Kalite Kontrol Formu düzenlenir." Tablo 9.2'nin 16 sorusu (PDF s. 240–241) Tablo 9.1'in sorularının mimariye özgü
olmayanlarıdır; metinleri birebir:

| 9.2 | Başlık | 9.1'deki aynı soru | Fark |
|---|---|---|---|
| 1–3 | Dosya boyutu, proje bilgileri, proje ve dosya isimlendirmesi | 1–3 | — |
| 4 | Varlıklar (isimlendirme) | 5 | 9.1'in 4'ü (mahal no ve ismi) yok |
| 5–8 | Koordinat sistemi, gelişim seviyesi, zorunlu sınıflar, zorunlu öznitelikler | 6–9 | 5: "öznitelikleri ve özellikleri" |
| 9 | Mahal ve mekânsal zon alanları emsal tablolarıyla | 10 | 9.1'in 11'i (Proje, Saha, Bina, Kat) yok |
| 10–11 | Varlıklar doğru katta, yinelenen yok | 15–16 | 9.1'in 12–14'ü (mahal) yok |
| 12–16 | IFC sınıfları, IFC sürümü, ön tanımlı tip, P_set'ler, PDF ve IFC uyumu | 17–21 | 12: "dönüştürüldü mü" (9.1: "oluşturuldu mu") |

Disiplin dosyanın adından okunur: EK-2 Tablo 2.1'in şablonunda üçüncü alan "Disiplin", Tablo 2.2'nin iki harfli kodu (MM
mimari, IM iç mimari, ST statik, EE elektrik, MK mekanik, HR harita, AL altyapı, PY peyzaj, ZE zemin, YN yangın, GS güneş
enerji sistemi, AK akustik).

## §2. Karar

- **K216 — Hangi tablo:** dosyanın adı EK-2 Tablo 2.1'in şablonundaysa ve disiplin alanı Tablo 2.2'de MM'den başkaysa
  Tablo 9.2; yoksa 9.1. Adı şablonda olmayan dosyanın disiplini bilinmez: mimari sayılır (3. soru adı söyler). İç mimari
  (IM) Tablo 9.2'dir: 9.1 "mimari disiplin" der, iç mimari Tablo 2.2'de ayrı disiplindir.
- **K217 — Kanıt:** 9.2'nin her sorusu 9.1'deki aynı sorunun kanıtını alır (`esas`). Onarımlar programdan bağımsız (öbür
  disiplinin modeli başka bir programın; KentOS'un künyesini, pencerelerini göstermez); proje bilgilerinin gerekçesi
  KentOS'un künye kutularını değil eksik alanları söyler. Mahali ve zonu olmayan disiplin modelinde 9. soru (emsal)
  inceleme: soru tabloda, ama emsal mimari modelin formunda (9.1/10) okunur; mahalli modelde 9.1'in kanıtı.
- **K218 — Paket incelemesi:** modelin tablosu (`tablo`, komutta `table`), konum katmanının sorusu `esas` 6 (9.2'de 5.
  satır); müellifin formu (`kentos.ek9Form`'un `table`'ı) modelin tablosundan başkaysa `review_form_table`, satırlar
  karşılaştırılmaz (numaraları başka sorulardır). Pencerenin özeti formun tablosunu söyler.

## §3. Uygulama

- `catalog/standards/dijital-proje/src/dijital_proje/form.py` (`SORULAR_92`, `ONARIM_92`, `tablosu`, `cevapla(ifc, tablo)`,
  `_disiplin`), `inceleme.py` (`tablo`, `esas`, `review_form_table`); 0.11.0, CHANGELOG.
- `crates/shared/contracts/src/arch/package_review.rs` (`PackageReviewed.table`), `crates/native/headless/src/package_review.rs`,
  `apps/desktop/src/arch/package_review_window.rs` (özet); TS, katalog ve Python SDK'sı.
- `crates/shared/ifc/src/read/facts_tests.rs`: başka bir programın statik modelinin olguları
  (`fixtures/ifc/v1/olgular/kesit-binasi-statik.json`, kaynağı `fixtures/ifc/v1/statik/kesit-binasi-statik-datum.ifc`).

## §4. Sınamalar

- `test_form.py` `OburDisiplinler`: disiplin adından (ST, MK, IM 9.2; MM ve şablonsuz ad 9.1); 16 satır ve 9.1
  karşılıkları; 12. sorunun metni; statik modelde kalanlar (proje bilgileri, dosya adı, varlık adları, sistem, zorunlu
  alanlar, katlar) ve emsal sorusu inceleme; onarımlarda künye ve pencere yok, kaynak "EK-9 Tablo 9.2 satır N";
  mahalli bir modelde 9. soru 9.1'in kanıtı.
- `test_inceleme.py`: statik model Tablo 9.2, 16 satır; Tablo 9.1'le gelen form `review_form_table`, karşılaştırma yok;
  sistem bulgusu konum katmanında; mimari paket 9.1.
- Başsız (`package_review::a_structural_model_is_answered_by_table_9_2`) ve masaüstü
  (`package_review_tests::a_structural_model_is_reviewed_by_table_9_2`): EK-2 adlı statik model 9.2 ve 16 satır, KentOS'un
  paketi 9.1 ve 21.
- Resim: `.run/gui-evidence/tablo-9-2-8114/` (statik modelin Form sekmesi, açık ve koyu).

## §5. Sıradaki adımlar

- Disiplin modelinin müellif formunun Tablo 9.2'ye göre yazımı (bugün KentOS yalnız mimari modelin formunu yazar).
- Bir pakette birden çok disiplin modeli (her birinin formu); IFC-08'in öbür kalanları: son yazılan dosyanın denetimi ve
  bulut işi, beş sonucun ayrı raporu, model eşlemesinin bütçesi.
