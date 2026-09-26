# ADR 0052: Masaüstünde çalışma modları

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.12; DESIGN.md §7.3.2; ADR 0049 (yeni proje ve proje ayarları), 0051 (şeridin pencereye sığması)
- **Sahibin yönü (26 Eylül):** masaüstü web ile aynı düzeye getirilir.

## Bağlam

- Web'de projenin çalışma modu (Hibrit, CAD, CBS; 3D Plan ve Afet Analizi “Yakında”) menüleri, şeridi ve araç kutusunu süzer (`app/workspaces.ts`). Mod proje ayarıdır; veriyi değiştirmez, gizlenen komut komut satırından ve kısayolundan çalışır.
- Web her modun şeridini o modun süzgeciyle kurar (`ribbonTabs` + `workspaceFilter`): gizlenen menüden gelen paneller düşer, boş sekme kalkar (CAD'de İşlemler), bazı paneller yalnız bir modda vardır (CBS'de Giriş'in Harita paneli), sekmeler modun adını alır (CAD'de Harita “Ölçme”).
- Masaüstü modu yeni projede ve Proje ayarları'nda saklıyor, gösteriyordu (ADR 0049) ama şeridi süzmüyordu; `workspace.*` komutları çalışmıyordu.

## Karar

- **Şerit envanterden, mod mod gelir.** Envanter Hibrit'in şeridinin yanında öbür hazır modların şeridini de yazar (`layout.ribbonByMode`, web'in kendi kurduğu gibi). Masaüstü çizimin modunun şeridini gösterir (`Catalog::tabs_in`); kuralları yeniden yazmaz. Mod hazır değilse ya da çizim yoksa Hibrit gösterilir (web'in `effectiveWorkspace`'i).
- **Açık sekme mod değişince kalırsa kalır;** mod onu gizliyorsa Giriş açılır (DESIGN.md §7.3.2).
- **`workspace.hybrid`, `workspace.cad`, `workspace.gis`** çizimin mod ayarını değiştirir: çizim kaydedilmemiş olur, geri alma adımı değildir (proje ayarı, ADR 0049); ileti web'inkidir: “Çalışma modu: CAD. Gizlenen komutlar komut satırından ve kısayoluyla yine çalışır.” Duyurulan modlar “Yakında” der, bir şey değiştirmez. Çizim yoksa bunu söyler.
- **Durum çubuğu:** koordinat sisteminin solunda vurgu renginde mod işareti ve modun adı; tıklayınca menü: başlık “Çalışma modu”, seçilebilen modlar (işaretli olan seçili), ayırıcı, duyurulanlar soluk ve sağda “Yakında”. İpucu modun ne olduğunu ve kaydedilmiş mod yakındaysa Hibrit gösterildiğini söyler. Görünüm → Görünüş → Çalışma modu menüsü de seçili modu işaretler.
- Pencereye sığma her modun şeridinde sınanır: üç modda her sekme 1100 px'e kaydırmadan sığar (ADR 0051).

## Bu dilimde olmayanlar

- Masaüstünde menü çubuğu ve araç kutusu yok; süzülen yalnız şerittir.
- Bağlamsal Seçim sekmesi (web'de seçim varken görünür) masaüstünde yok.

## Doğrulama (26 Eylül 2026, Linux)

- `modes::tests`:
  - mod proje ayarıdır: çizim kaydedilmemiş olur, geri alma adımı eklenmez, durum ve menü işareti seçili modu gösterir;
  - duyurulan mod bir şey değiştirmez; çizim yokken söylenir;
  - şerit modu izler: CAD'de İşlemler yok ve Harita “Ölçme”; CBS'de Çizim sekmesinde elips yok, Giriş'te Harita paneli var; gizlenen sekme Giriş'e döner, tutulan sekme kalır.
- `ribbon_tests`: üç modun her sekmesi 1100 px'e sığar.
- Görüntüler: `.run/shots/serit-cad-olcme-1440.png`, `serit-cbs-giris-1440.png`.
