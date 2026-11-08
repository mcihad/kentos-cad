# 2540 — Uzlaştırmada alan kuralları ve pencerede alan girişi

Tarih: 2026-11-08. plan/map-cad.md MAP-04, dördüncü tur. Durum: taslak. Plan satırı: “… eksik zorunlu alan bulunur”;
ADR 2140'ın “Sonra”sındaki “pencerede eksik alan girişi (ADR 1760)” maddesi.

İlgili:
- [ADR 1760](1760-asset-candidate-reconciliation.md): adayların uzlaştırılması, kararlar (`fields` dahil);
- [ADR 2140](2140-area-candidates-by-overlap.md): alan adayları;
- [ADR 0199](0199-layer-fields-and-feature-table.md) §1: katman alanlarının değer kuralı (`check_value`);
- [ADR 1360](1360-field-codes-and-kodla-ciz.md): Harita varlıkları profili, alanların değer listeleri (domain).

## Dayanak

- **QGIS Kullanıcı Kılavuzu, Vector Properties › Attributes Form › Constraints**
  ([belge](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html), 8 Kasım 2026'da
  okundu):
  - kısıtlar alana girilebilecek değerleri sınırlar: boş olmaz, tek olur, ifadeye uyar;
  - form her değeri girilirken sınar. Uyan değerin yanında yeşil işaret çıkar, uymayanın yanında çarpı; üstüne gelince
    hangi kısıtın uymadığı görünür;
  - “Enforce” seçili kısıt (sert kısıt) uyulmadıkça değişikliğin kaydedilmesine izin vermez. Yumuşak kısıt yalnız uyarır.
- **KentOS'un alan kuralı** (ADR 0199 §1, `check_value`): katman alanının türü (tam sayı, ondalık, tarih …), uzunluğu,
  değer listesi ve aralığı. Uyan değer kanonik yazılışıyla saklanır (listenin etiketi kodu, ondalık virgül nokta, baştaki
  sıfırlar atılır). Katmana yazan komutlar bu kuralı uygular (`checks::field_values`).
- **Profilin uygulanması** (ADR 1360): Harita varlıkları profili sınıfların alanlarını katmanlarına koyar. Değer
  listesi olan alan (şebeke, direk türü, kontrol noktası türü) listesini katmanda taşır.
- **KentOS bugün:**
  - uzlaştırma varlığı sınıfın katmanına doğrudan yazıyordu. Adayların ve kararların değerleri katmanın alan
    kurallarından geçmiyordu: “Atıksu” şebeke listesinde olmadığı hâlde rögara yazılırdı, “Aydınlatma” etiketi kodu
    yerine kalırdı;
  - komut kararla alan vermeyi biliyordu (`fields`). Pencere ise yalnız eksik alanı gösteriyordu, değer girilemiyordu.

## Karar

1. **Alan kuralı** (`map.features.reconcile`): varlığın değerleri sınıf katmanının alanlarından geçer. Değerler
   adaylardan ve kararlardan gelir; kural, katmana yazan her komutunkiyle (`check_value`) aynıdır:
   - uyan değer kanonik yazılışıyla yazılır;
   - uymayan, grubun `invalid`'inde alanı ve kuralın cümlesiyle söylenir;
   - böyle grup yazılmaz. Bu QGIS'in sert kısıtı gibidir: uyan bir değer kararla verilene dek bekler;
   - boş değer sınanmaz (zorunlu alanın boşluğu `missing`'dir, ADR 1760);
   - katmanın alanı olmayan değer eskisi gibi yazılır;
   - sınıf katmanında alan yoksa (profil uygulanmamış) sınama yoktur. Var olan durumlar bu yüzden değişmedi.
2. **Cevapta** `ReconciledGroup.invalid`: `FieldTrouble` (alan, ileti), katmanın alan sırasıyla.
3. **Masaüstü** (Adayları uzlaştır):
   - tablonun **Alan** sütunu “… eksik” ve “… kurala uymaz” der;
   - özet her kurala uymayan değeri cümlesiyle söyler;
   - eksik ya da kurala uymayan alanı olan grubun satırında **Alan gir** vardır. Tablonun altında o alanların formu
     açılır: listesi olan alan listeden seçilir (kodu girer), öbürleri yazılır;
   - kurala uymayan alanın yanında kuralın cümlesi durur;
   - **Alanları ekle** değerleri grubun adaylarının kararlarına koyar (var olan kararın üstüne) ve plan yeniden okunur;
   - değeri pencere değil komut sınar. Kural Python'da ve MCP'de de aynıdır.
4. **Kapsam işareti:** ADR 1760'taki gibi **Harita** (pencere ve yazma); komut Python ve MCP'de.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_features_reconcile_cases.py`, KentOS kodu yok.
  - Alan kuralı ADR 0199'dan, durumların kullandığı türler için yeniden yazıldı: listenin etiketi Türkçe büyük-küçük harf
    katlanarak koduna, metnin uzunluğu, tam sayının ve ondalığın yazılışı.
  - Sınıf katmanlarına profilin alanları, listeli alanlara listeleri konur (profilin uygulanması gibi).
- **Yeni durumlar** (35, 30'dan):
  - direğin türü “Aydınlatma” `aydinlatma` olarak yazılır; kabul edilen rögarın “Atıksu”su listede yok, rögar yazılmaz;
    kapının numarası eksik;
  - kararla şebeke “Kanalizasyon” (`kanalizasyon`) ve kapı numarası verilir, ikisi de yazılır;
  - 25 karakterlik kapı numarası: “Kapı no” alanı en çok 20 karakter alır;
  - kat sayısı “üç”: tam sayı değil;
  - kat sayısı “04”: 4 yazılır.
- **Var olan durumlar** değişmedi (alansız katmanlar). Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/reconcile_field_rules.rs`): alanlı çizim `.kcad` olarak açılır. Plan rögarın
  kurala uymayan şebekesini söyler; kararla verilen değer yazılır, kodu listenin kodudur.
- **Masaüstü** (`features_reconcile_fill_tests.rs`):
  - durumun cümlesi pencerede de gelir;
  - Alan gir listeden Kanalizasyon'u seçer, kapı numarasını yazar; ikisi kuralın değeriyle yazılır;
  - boş Alanları ekle söylenir, uzun numara cümlesiyle kalır.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/alan-gir-2540/`.

## Sonra

- İnceleme kaydının özniteliğe değil ayrı kayda yazılması (MAP-04'ün kalanı).
- Kurala uymayan aday değerinin kaynağında düzeltilmesi (bugün karar onu ezer, aday olduğu gibi kalır).
- Büyük aday kümesinde ızgara dizini; yaylı aday; AI çıkarımı.
- Web (TODOS §0.2).
