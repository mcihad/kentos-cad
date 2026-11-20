# 1080 — Araç girdisi uygun katmanda açılır, girdi sorunları birlikte söylenir

Tarih: 2026-11-20. gis-usage-report.md GIS-014. Durum: kabul edildi.

İlgili: [ADR 0201](0201-geometry-operations.md) (işlem araçları), [ADR 0209](0209-network-analysis.md).

## Sorun

“Park koruma 10 m” alan katmanı etkin, üç noktalı “Ağaçlar” varken En yakın tesis iki nokta girdisini de etkin alan
katmanında açtı (uygun nokta yok). Çalıştır yalnız ilk girdi için hata verdi (“1 alanı düzeltin”); ikincisi
düzeltilmeden de iş yürümüyordu. Maliyet matrisi ve Hizmet alanları aynıydı.

## Karar

1. Masaüstü işlem penceresi açılırken (ve Varsayılanlar'da), varsayılanında kalan, belirli türleri alan her nesne
   girdisi etkin katmanda o türlerden hiçbir nesne yoksa ağaçta o türlerden nesne tutan ilk katmanda açılır. Her girdi
   ayrı önerilir ve ayrı değiştirilebilir; kullanıcının ya da geçmişin verdiği değer olduğu gibi kalır
   (`ToolDialog::propose_layers`). Herhangi türü alan araç (Tampon) etkin katmanda kalır.
2. Varsayılan değerlerin kendisi (`kentos_processing::parameters::default_values`) web ile paylaşılan sözleşmedir,
   değişmedi; öneri pencerenin işidir.
3. Çalıştırıcı, boş ya da hepsi kilitli girdileri ilkinde durmadan toplar ve hepsini tek değerlendirmede söyler
   (`Outcome::Invalid` bütün sorunlarla).

## Sınama

- `processing::input_layer_tests`: alan katmanı etkinken En yakın tesis'in iki girdisi Ağaçlar'da açılır; Tampon
  etkin katmanda kalır. Resim `.run/gui-evidence/gis-014/`.
- `crates/native/processing/tests/input_issues.rs`: Kırp'ın iki girdisi boş katmandayken iki sorun birlikte gelir.
