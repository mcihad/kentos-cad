# 4600 — Topoloji bulgusunun Öznitelikler'den işaretlenmesi ve çoklu seçimin bulgu sayıları

Tarih: 2026-11-10. plan/map-cad.md MAP-07, dokuzuncu tur. Durum: taslak. ADR 3800'ün “Sonra”sındaki “Bulgunun
Öznitelikler'den Hata işaretle ya da İstisna yap'ı (bugün sekmede)” ve “Çoklu seçimde nesnelerin bulgu sayıları”.

İlgili:
- [ADR 1260](1260-topology-quality-review-and-repair-recipe.md) §1: bulgunun yaşam döngüsü (Değerlendirilmedi, Hata, İstisna,
  Yeniden değerlendir) ve gerekçe;
- [ADR 0589](0589-topology-check-bound-to-sources.md) §3: `cad.topology.exceptions`;
- [ADR 3800](3800-topology-findings-of-the-selected-object.md): seçili nesnenin bulguları (Öznitelikler › Topoloji).

## Dayanak

- **ArcGIS Pro**, [Validate and fix geodatabase topology](https://doc.esri.com/en/arcgis-pro/latest/help/editing/validate-and-fix-geodatabase-topology.html)
  (10 Kasım 2026'da okundu):
  - Error Inspector sonuçları bir tablo görünümünde gözden geçirmeye, düzeltmeye ve yönetmeye yarar;
  - Fix sekmesinde seçili hata düzeltilir, istisna yapılır ya da istisnası kaldırılır;
  - Mark as Exception (X) vurgulanan hatayı topoloji katmanlarından kaldırır, doğrulamada yok sayar; kural yeniden
    gerekirse hata geri getirilir;
  - Clear Exception (E) istisnayı kaldırır ve kaydı hata olarak işaretler;
  - haritada hataya tıklanıp açılır menüyle de düzeltilir.
  - Sayfa nesnenin kendi panelinden (Attributes) hata yönetimini anmaz. İşaretin Öznitelikler'e taşınması KentOS'un
    kararıdır: ADR 3800 bulguları orada gösteriyordu, işaret için sekmeye gidiliyordu.
- **KentOS bugün** (ADR 3800): Öznitelikler › Topoloji bulguyu ve durumunu söyler, düğmesi sekmeyi bulguda açar.
  İşaret (İstisna yap, Hata işaretle, İşareti kaldır) ve gerekçe yalnız sekmededir. Birden çok nesne seçiliyken bölüm
  yoktur.

## Karar

1. **Öznitelikler › Topoloji** (tek nesne, Harita ve CBS):
   - bulgu varsa ilk satır **Gerekçe**: sekmenin Gerekçe'siyle aynı yazı; burada yazılan sekmede de görünür. Çizim
     değişmez. Altında sessiz satır: “Hata ve istisna bu gerekçeyle yazılır.”;
   - her bulgunun hücresi açılır liste: **Değerlendirilmedi**, **Hata**, **İstisna**; şimdiki durum işaretli (yeniden
     değerlendirilecek bulguda hiçbiri). Seçilen, sekmenin işaretiyle aynı yoldan yazılır: `cad.topology.exceptions`
     (İşareti kaldır, Hata, İstisna), gerekçeyle; sonra yeniden denetlenir;
   - sekmenin seçili satırları değişmez;
   - bulgunun altındaki sessiz satırlar: işaretin gerekçesi (varsa) ve ölçü ile kural;
   - satırın düğmesi eskisi gibi sekmeyi bulguda açar.
2. **Çoklu seçim:** kuralların andığı nesnelerin son denetimdeki bulguları, her biri bir kez:
   - **Bulgular** toplamı ve her durumun sayısı ayrı satırda;
   - sessiz satır: “Denetlenen N nesnenin; ortak bulgu bir kez sayılır.”;
   - denetim yoksa “Denetlenmedi”, çizim değiştiyse uyarı;
   - sayım bulguların üstünden bir kez geçer, seçimin büyüklüğünden bağımsızdır.
3. **Komut değişmedi:** `cad.topology.exceptions` aynı; proje kalite profili gerekçe istiyorsa ret iletisi uyarı
   satırına yazılır.
4. **Kapsam işareti:** ADR 3800 gibi **Harita + CBS**.

## Sınırlar

- Açılır liste tek bulguyu işaretler; birden çok bulgu sekmeden işaretlenir.
- İstisnanın kapsamı (`objects`: kuralın aynı nesneler arasındaki bütün bulguları) Öznitelikler'de seçilmez; bulgunun
  yeri (`finding`) yazılır.
- Çoklu seçimde bulgular listelenmez, sayılır.

## Kanıt

- **Masaüstü** (`topology::object_tests`):
  - A1'in bulgusunun açılır listesi üç seçenek sunar, Değerlendirilmedi işaretli;
  - Öznitelikler'de yazılan gerekçe sekmenin gerekçesidir;
  - İstisna bulguyu o gerekçeyle istisna yapar; satır İstisna, altında gerekçe; Hata, sonra Değerlendirilmedi;
  - sekmenin seçili satırları değişmez;
  - A1, A2 ve A5 seçiliyken iki bulgu (çakışma bir kez, kısa kenar) ve satırları;
  - ADR 3800'ün testi yeni sıraya göre (Gerekçe, bulgu, ölçü ve kural) güncellendi.

  Topoloji, Öznitelikler, form testleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/oznitelikten-isaret-4600/`.
- **Komut, Python ve MCP:** komut ve sözleşme değişmedi; `cad.topology.exceptions`'ın ortak durumları, Python ve MCP
  testleri (ADR 0589, 1260) bu dilimde yeniden çalıştırılmadı.

## Sonra

- Açılır listeden istisnanın kapsamı; birden çok bulgunun Öznitelikler'den işaretlenmesi.
- Köşenin gidişine ok; tutamaçla birden çok nesne; büyük veri ölçümü (MAP-07'nin kalanı).
- Web (TODOS §0.2).
