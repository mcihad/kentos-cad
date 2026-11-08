# 2820 — Saha kaydının kendi paketinin tasarımıyla denetlenmesi

Tarih: 2026-11-08. plan/map-cad.md MAP-19, dördüncü tur. Durum: taslak. ADR 2420'nin “Sonra”sındaki “paketin tasarım
noktalarıyla (eski tasarım) karşılaştırma seçeneği”; planın kabulü: “Eski tasarım revizyonuyla ölçülen kayıt daha sonra
yeni tasarıma sessiz bağlanmasın.”

İlgili:
- [ADR 1700](1700-asbuilt-check.md): uygulama kontrolü;
- [ADR 1920](1920-field-package.md): saha paketi (`noktalar.csv`, `cizgiler.csv`);
- [ADR 2420](2420-field-record-checked-with-its-package.md): dönen kaydın kendi paketiyle denetlenmesi.

## Dayanak

- **Trimble Access, As-staked point details**
  ([belge](https://help.fieldsystems.trimble.com/trimble-access/latest/en/stakeout-as-staked-point-details.htm),
  8 Kasım 2026'da okundu): aplikasyon farkları ölçülen noktadan, işteki tasarım noktasına göre verilir ve noktayla
  saklanır. Fark, ekibin elindeki tasarıma göredir.
- **KentOS bugün** (ADR 2420):
  - kayıt paketiyle verilince paketin tasarım özeti beklenen özet olur;
  - tasarım değiştiyse `designChanged` söylenir;
  - karşılaştırma ise çizimin bugünkü tasarımıyla yapılır. Ekibin uyguladığı noktanın değişen tasarıma göre farkı
    yapım hatası gibi görünebilir;
  - paket tasarımı zaten taşıyor (`noktalar.csv`: Ad, Y, X, Z, Kimlik; `cizgiler.csv`: Kimlik, Sıra, Y, X, Bulge). Her
    sayı, aynı f64'e geri okunan en kısa yazımdadır (ADR 1920).
- **Bulunan:** önceki ortak durumların paketlerinde `noktalar.csv` yükseksiz noktaya “None” yazıyordu, `cizgiler.csv`
  boştu. Dosyalar o durumlarda okunmuyordu. Üreteç artık paketin dosyalarını `cad.field.package`'ın yazdığı gibi yazar.
  Eski durumların cevapları aynı kaldı.

## Karar

1. **`packageDesign`** (`cad.asbuilt.check`, ek alan; komut sürümü değişmez; `package` ister):
   - karşılaştırılan tasarım paketin noktaları ve çizgileridir, ekibin elindeki tasarım;
   - noktalar ve çizgiler paketin kalıcı kimlikleriyle gelir. Nesne çizimden silinmiş olsa da kimlik söylenir;
   - yüzey verilmişse çizimin yüzeyidir. Paket yüzeyin yalnız kaynak özetini taşır.
2. **Paket kendini doğrular:** okunan noktaların ve çizgilerin özeti paketin `designDigest`'i olmalıdır. Değilse ret:
   paket bozulmuş ya da elle değiştirilmiştir.
3. **Değişim:**
   - `design` ve `lines` katmanları verilmişse yalnız çizimin bugünkü tasarımının özetini verir;
   - özet paketinkinden farklıysa `designChanged` doğrudur;
   - katman verilmezse değişim söylenmez, karşılaştırma yine paketin tasarımıyladır.
4. **Cevap:** `designDigest` paketin özeti; `package.design` doğru. Bir nokta adla, yerle ya da çizgiyle paketin
   tasarımına eşlenir; kurallar ADR 1700'ünkülerdir.
5. **Retler** (`invalid_asbuilt_check`):
   - `packageDesign` paketsiz (`packageDesign`);
   - paketin tasarım dosyaları yok ya da bir satırı okunmuyor (`package`);
   - dosyalar özetiyle tutmuyor (`package`).
6. **Masaüstü** (Uygulama kontrolü): Kaydın saha paketi satırında **paketin tasarımıyla** kutusu. Paket seçilince
   işaretlenebilir, paket kaldırılınca kalkar. Özet tasarımın paketten geldiğini söyler. Çizimdeki değişim bu kipte uyarı
   olur, kaydın hatası sayılmaz.
7. **Kapsam işareti:** ADR 1700'deki gibi **Harita + Yol + CBS**.

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/asbuilt_cases.py`, KentOS kodu yok):
  - üç yeni paket Python'un zipfile'ıyla:
    - eski tasarımın paketi: P1 3 cm güneyde, P7 yok, P8 yükseklikli, bordür 2 cm kuzeyde;
    - noktalar.csv'si özetiyle tutmayan bozuk paket;
    - eski paket de başka özetli olduğu için reddedilir;
  - durumlar:
    - değişmemiş tasarımın paketiyle cevap katmanınkinin aynısı;
    - eski tasarımla P1'in kuzey farkı 0,03 m, ölçülmeyenler P5 ve P6, `designChanged`;
    - katmansız denetimde değişim söylenmez;
    - üç ret;
  - 42 durum (36'dan). Eski durumlar aynı. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/asbuilt_package_design.rs`): eski tasarımın paketiyle cevap durumunki; bozuk paket
  reddedilir. ADR 2420'nin MCP testi aynen geçer.
- **Masaüstü** (`asbuilt_window_tests.rs`):
  - paketin tasarımıyla cevap durumunki;
  - Paketi kaldır kutuyu da kaldırır.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/paketin-tasarimi-2820/`.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("uygulama.kcad")
r = cad.asbuilt.check(doc, measured="olcum", design="tasarim", lines="bordur", search=0.5,
                      package="/saha/eski-tasarim-paketi.zip", package_design=True)
print(r.design_changed, [(p.name, p.dn) for p in r.points if p.design_name])
```

MCP: `{"tool": "cad.asbuilt.check", "arguments": {..., "package": "…zip", "packageDesign": true}}`.

## Sonra

- Paketin yüzeyinin kendisi (bugün yalnız kaynak özeti): yüzey farkının da paketin yüzeyiyle olması.
- İş türleri (as-built, aplikasyon, deformasyon) ve zaman serisi; yetkili inceleme kaydı.
- Web (TODOS §0.2).
