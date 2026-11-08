# 2400 — Paftada ölçme kalite notu projenin GNSS arşivinden

Tarih: 2026-11-08. plan/map-cad.md MAP-18, üçüncü tur. Durum: taslak. Plan satırı: “Harita grid'i, gerekiyorsa ikinci
koordinat sistemi, kuzey türü, ölçek çubuğu, datum/epok ve kalite notları aynı proje bilgisinden gelsin.”

İlgili:
- [ADR 1680](1680-sheet-coordinate-facts.md), paftada koordinat bilgisi; [ADR 1900](1900-sheet-north-facts.md), kuzey
  bilgisi. İkisinin “Sonra”sındaki “ölçme kalite notu” maddesini bu ADR kapatır;
- [ADR 1340](1340-gnss-campaign-archive.md) ve [ADR 1740](1740-gnss-network-from-archive.md): GNSS arşivi, kabul edilen
  noktalar ve ağ dengelemesi kaydı (`da-N`);
- [ADR 0203](0203-network-adjustment.md): nokta konum hatası σP = √(σY² + σX²).

## Dayanak

- **KentOS'un kuralı** (ADR 0203): nokta konum hatası σP = √(σY² + σX²), 1σ. Türk ölçmeciliğinde “ortalama hata” ±
  ile yazılır.
- **KentOS bugün:**
  - harita projesinin GNSS arşivi her kabul edilen noktanın sonucunu tutar: σ kuzey, doğu, yukarı (m) ve noktanın
    yapıldığı ağ dengelemesi;
  - ağ dengelemesi kaydı sınıfı (C1/C2/C3), kural paketini ve sağlanıp sağlanmadığını, m0'ı ve serbestliği tutar;
  - pafta bunları okumuyordu. Antetteki kalite notu elle yazılıyordu; arşiv değişince eski not kalıyordu.
- **Yazı tipi:** paftanın yazı tipinde σ yoktur, “?” basılırdı. Test bunu yakaladı (ADR 1680'deki “k₀” gibi).

## Karar

1. **`kentos_project::survey_facts::survey_facts`:** projenin ölçme kalitesini tek yerden sözle verir.
   - **Konum doğruluğu:** σ'sı olan kabul noktalarının en büyük nokta konum hatası √(σK² + σD²), cm'nin ondalığıyla
     (mm), ör. “±0,5 cm”.
   - **Yükseklik doğruluğu:** en büyük σ yukarı, ör. “±0,9 cm”.
   - **Ağ dengelemesi:** noktaların dayandığı dengelemeler, arşivin sırasıyla. Ör. “da-1, C3 sınıfı, kurallar
     sağlandı (bohhbuy-2018-gnss@1), m0 0,98, serbestlik 3”. Kayıtta olmayan parça yazılmaz.
   - **Not:** üçü bir cümlede. Ör. “GNSS ile 2 nokta: en büyük nokta konum hatası ±0,5 cm, yükseklik hatası ±0,9 cm
     (ortalama hata); ağ dengelemesi da-1 (C3, kurallar sağlandı). 1 noktanın doğruluğu kayıtlı değil.”
   - Kabul noktası olmayan projede hiçbiri yoktur. Hiçbir değer tahmin edilmez.
   - Sözler gösterimdir; değerlerin kendisi arşivde kalır.
2. **Pafta yazıları için yeni `@` değerleri:** `@konum_dogrulugu`, `@yukseklik_dogrulugu`, `@ag_dengelemesi`,
   `@olcme_kalitesi`.
   - `RenderInputs.project` bu alanları taşır (web'in türü de). Baskı (`base_inputs`) ve masaüstünün pafta önizlemesi
     aynı işlevden doldurur.
   - **Ayrılmış ad değildir:** paftanın, kitabın ya da projenin aynı adlı değişkeni önce gelir (ADR 1680 gibi).
3. **Baskı öncesi bulgu `survey_quality_missing`** (uyarı): bir yazı bu değerlerden birini okuyor ama projenin
   arşivinde kabul edilmiş nokta yoksa, değer boş basılacaktır. Aynı adlı değişken varsa bulgu yoktur. Çözüm: noktaları
   arşivden kabul etmek ya da notu pafta değişkeni olarak yazmak.
4. **Kapsam:** proje düzeyindedir. Pafta bir projenin bir bölümünü gösterse de not projenin bütün kabul noktalarının en
   büyük hatasını söyler. Bu, paftadaki noktalarınkinden küçük değildir; doğruluğu abartmaz.
5. **Pafta tasarımcısı:** Bağla penceresinin hazır adlarına ve Değişkenler penceresinin açıklamasına yeni adlar
   eklendi.
6. **Kapsam işareti:** ADR 1680'deki gibi **CAD + CBS** (pafta); arşiv Harita projesindedir.

## Kanıt

- **Proje:** `fixtures/kcad/v2/map-survey-network.json`.
  - kn-1 ortalamayla kabul edilmiş, σ'sı yok;
  - kn-2, da-1 ağ dengelemesiyle kabul edilmiş: σ 4,0 / 3,5 / 9,0 mm; C3, kurallar sağlandı, m0 0,98, 3 serbestlik.
  - Elle: √(4,0² + 3,5²) = 5,315 mm → “±0,5 cm”; yükseklik 9,0 mm → “±0,9 cm”.
- **`kentos-project`** (`survey_facts::tests`): ağ projesinin dört değeri; σ'sız noktalar (“doğrulukları kayıtlı
  değil”) ve arşivsiz proje (hiçbiri).
- **`kentos-plot`** (`tests/all/survey_facts.rs`):
  - paftanın yazıları dört değeri basar, “?” yoktur;
  - projenin `konum_dogrulugu` değişkeni önce gelir;
  - baskı denetimi arşivsiz projede dört yazıyı söyler, değişkeni olanı söylemez, arşivli projede hiçbirini.
- **Bağımsız okuyucu:** basılan PDF'i `pdftotext` okur; dört satır yukarıdaki sözlerdir.
- **MCP** (`crates/native/mcp/tests/sheet_survey_quality.rs`): proje `.kcad` olarak açılır, pafta `cad.sheets.edit`
  ile eklenir. `file.preflight.sheets` arşivli projede bulgu vermez, arşivsizde iki yazıyı söyler.
- **Takımlar:** `kentos-sheet` (429) ve `kentos-sheet-ui` geçer. Masaüstünde `sheets` ve çapraz denetimler 154 geçer.
- **Resim:** `.run/gui-evidence/olcme-kalitesi-2400/` (`pafta.pdf`, `pafta-yakin-1.png`).

## Sonra

- Notun haritanın kapsadığı noktalarla sınırlanması (bağlı haritanın çerçevesi).
- Total station ve nivelman ölçmelerinin kalitesi (bugün yalnız GNSS arşivi).
- Kurum eksen adları ve resmî profil kapısı (sahibin ve kurumun kararı).
- Atlas sayfalarının kaynak dökümü (ADR 1320 Kalan).
