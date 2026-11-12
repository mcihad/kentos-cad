# 8025 — Donatının kullanım alanı denetimde

Tarih: 2026-11-08. Durum: kabul (M25, ADV-12; plan S13).

## Sorun

- **Donatının önü denetlenmiyordu.** Klozetin, lavabonun, duşun önünde kullanıcının durduğu alan (kitaplığın "Boşluk
  zarfı", ADR 0702) yalnız bir görünüştü. Mimari denetim bu alana bir duvarın, başka bir donatının ya da bir kapı
  kanadının girdiğini söylemiyordu.
  - Bina programının `program_fixture_clearance` bulgusu (ADR 0773 §5) ancak projede program ve blok adıyla boşluk
    ölçüleri tanımlıysa çalışıyordu.
- **Sahte çarpışma.** Donatının izdüşümü (`Document::fixture_footprint`) bloğun bütün nesnelerinden hesaplanıyordu; görünüş
  durumunun gizlediği zarf da içindeydi.
  - Asma klozet duvardan 54 cm değil, 114 cm derin sayılıyordu.
  - Gerçek pencerede (M24 ikilisi): 2,0 × 1,6 m banyoda Denetim "K1 kapısının kanadı açılırken bir donatıya çarpıyor
    (fiziksel çarpışma)" dedi. Oysa kanat klozete değil, önündeki havaya değiyordu.
- **ADV-12:** "Kullanım alanı ve çakışma: kapı kanadı, klozet önü, tezgâh önü. Bulgular bugünkü mutfak çakışma
  denetimiyle aynı sistemdedir." İlk kabul: "Banyo örneğinde kullanım alanı çakışması işaretlenir."

## Araştırma (8 Kasım 2026)

- **Planlı Alanlar İmar Yönetmeliği, Madde 29** (mevzuat.gov.tr, 13.8.2025 değişikliğiyle):
  - Konutta banyo veya yıkanma yeri dar kenarı 1,50 m, net alanı 3,00 m²; tuvalet 1,00 m, 1,20 m².
  - (5): piyesler "engellilerin de kullanımını sağlayacak standartlara ve erişilebilirlik mevzuatına uygun olmak
    zorundadır".
  - Yönetmelik donatının önündeki alanın ölçüsünü vermiyor.
- **TS 9111** (erişilebilirlik) klozet ve lavabo çevresini düzenler; standardın metni açık değil, ölçüsü burada
  söylenmedi.
- **Kitaplığın zarfları** (ADR 0702, yaygın plan alışkanlığı): klozet önünde 80 × 60 cm, lavabo önünde 80 × 70 cm, duş
  girişinde 80 × 80 cm.
  - Zarfı yalnız sonradan eklenen dört blok çizer: Asma klozet, Pisuvar, Köşe duş, Dolaplı lavabo.
  - İlk 16 bloğun baytları eski projeler için korunmuş; Klozet, Lavabo ve Duş teknesi zarfsızdır.
- **KentOS'ta parçalar hazırdı:**
  - Kapı grubunun engelleri (duvar gövdeleri, kolonlar, donatılar) ve kapı kanadının süpürdüğü alanlar (ADR 0772 §5,
    0787 §1).
  - Çevir düzeltmesi.
  - Denetim panelinde bulguya tıklayınca nesnelerin seçilmesi ve görünümün onlara yakınlaşması.

## Karar

- **Donatının izdüşümü görünüş durumunun gizlediğini saymaz** (`fixture_footprint`, `hidden_by`). Durum, eklemenin
  seçtiği; yoksa ilki ("Sade"). Kapı grubunun sahte çarpışması bununla gitti.
- **Kullanım alanı** (`Document::fixture_zone`, `fixture_audit::fixtures`):
  - Bloğun "Boşluk zarfı" durumunun gösterip ilk durumunun gizlediği çizgilerin kutusu, eklemenin yerleştirdiği gibi.
  - Zarfı olmayan eski bloklarda ADR 0702'nin ölçüleri gövdenin önünde ortalanır: Klozet 0,80 × 0,60, Lavabo 0,80 × 0,70,
    Duş teknesi 0,80 × 0,80 m.
  - Bina programının boşluk verdiği blok programındır: bulgu iki kez söylenmez.
- **Yeni denetim grubu: Donatılar** (`ArchAuditCheck::Fixtures`, "fixtures"). Bulguların niteliği net geçiş:
  - `arch_fixture_zone_blocked`: alana bir duvar, kolon ya da başka bir donatı giriyor. İleti: "“Lavabo” donatısının
    önündeki kullanım alanına (0.80 × 0.70 m) başka bir donatı (“Asma klozet”) giriyor (net geçiş)."
  - `arch_fixture_zone_door`: bir kapının kanadı alana açılıyor. Düzeltmesi kapıyı çevirir (`cad.openings.edit` flip),
    çevrilmiş kanat alanın ve kapı grubunun engellerinden kurtulursa.
  - Değmek bulgu değildir: alanlar 1 mm küçültülerek sınanır.
- **Grup yalnız adıyla istenince çalışır** (kapılar gibi). Boş `checks` ile eski çağrıların cevabı değişmez; masaüstü
  bütün grupları ister.
- **Panelde işaret:** seçili ya da işaretli bir Donatılar bulgusunun kullanım alanı çizimde kesik kırmızıyla çizilir
  (`arch_audit_zones`). Düzeltmenin önizlemesiyle birlikte görünür.
- **Sözleşme:** grup ve iki kod eklendi. Web tipleri (`ArchAuditCheck.ts`), komut kataloğu ve Python SDK'nın tipleri
  yeniden üretildi.
- Kullanım alanı bir çizim yardımıdır; bir yönetmeliğe uyulduğu söylenmez.

## Ölçüm

- **Test** (`fixture_place_tests::a_bathroom_s_fixture_zones_are_found_taken_by_a_fixture_and_a_door`). Banyo 2,0 × 1,6 m:
  güney duvarında asma klozet, doğu duvarında ona dönük Lavabo, batı duvarında içe açılan K1.
  - Klozetin gövdesi duvardan 0,54 m, alanı 1,14 m'ye kadar.
  - Bulgular:
    - `arch_fixture_zone_blocked`: "“Lavabo” … (0.80 × 0.70 m) … (“Asma klozet”)";
    - `arch_fixture_zone_door`: "K1 kapısının kanadı “Asma klozet” …", niteliği passage, düzeltmesi `cad.openings.edit`.
  - Boş `checks` ile Donatılar bulgusu yok.
- **Gerçek pencere** (`fixtures/interaction/real/donati-denetim.txt`, testin sakladığı banyo, 1440×900):
  - ÖNCE (M24 ikilisi): 1 bulgu, Kapılar: "K1 kapısının kanadı açılırken bir donatıya çarpıyor (fiziksel çarpışma)".
  - SONRA: "2 bulgu · 1 düzeltilebilir", ikisi de Donatılar.
    - Kapı bulgusuna tıklayınca klozetin önü kırmızı çiziliyor; önizleme kapıyı dışa çeviriyor.
    - Lavabo bulgusunda lavabonun önü klozete biniyor.
    - "Düzelt: Açılışı öbür yüze çevir" ile "1 bulgu · 0 düzeltilebilir".
  - Resim: `.run/gui-evidence/mimari-m25/m25-once-sonra.png`.
- **Test takımları:** uygulama 672, masaüstü 2006 (önceden kırık `each_program_docks_its_own_panels` dışında), sözleşme
  1404, komutlar 1725 geçti.

## Sonraki adımlar

- Tezgâh önü (Mutfak dizisi) ve yatak, dolap önü; donatının Öznitelikler'de alanı açıp kapatması.
- Yerleştirirken canlı uyarı: Blok ekle'de alan bir şeye girerse imlecin yanında söylemek.
- Erişilebilir tuvalet profili (150 cm dönüş çemberi): erişilebilirlik profiliyle (ADR 0723) birlikte, TS 9111 okunduktan
  sonra.
