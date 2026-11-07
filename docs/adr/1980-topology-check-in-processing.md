# 1980 — Topoloji denetimi İşlem araç kutusunda: “Topolojiyi kurallarla denetle”

Tarih: 2026-11-07. plan/map-cad.md MAP-07, ikinci tur. Durum: taslak. Plan satırı (CBS ortaklığı): “kural motoru,
bulgu/istisna kaydı, ortak sınır ve toplu onarım Harita/GIS'te aynıdır. GIS'te de Processing, veri formu ve MCP üzerinden
aynı denetim sonucu erişilir.” İlgili:

- [ADR 0202](0202-topology-rules.md): topoloji kuralları ve `cad.topology.check`;
- [ADR 1260](1260-topology-quality-review-and-repair-recipe.md): bulgunun yaşam döngüsü, istisna, kalite profili;
- [ADR 0201](0201-geometry-operations.md) §6: Geçerliliği denetle, bu aracın kalıbı.

## Dayanak

- **GRASS `v.clean`:** temizleme araçlarının sırası ve eşikleri açıktır (plan [S22], [S23]).
- **QGIS:** Check validity ve Topology Checker sonuçları İşlem araç kutusundan alınır, bir modelde ya da betikte
  çalıştırılır. Sonuç tablo ya da hata katmanıdır.
- **KentOS bugün:**
  - Topoloji denetimi Topoloji kuralları penceresinde ve `cad.topology.check` komutundaydı (MCP ve Python dahil).
  - İşlem araç kutusunda ise yalnız Geçerliliği denetle vardı: kuralları, istisnaları ve yaşam döngüsünü bilmez.
    GIS kullanıcısı kurallı denetimi bir modele ya da toplu çalıştırmaya koyamıyordu.

## Karar

1. **İşlem aracı `topology.check`, “Topolojiyi kurallarla denetle”** (Geometri kategorisi). Salt okunur, çizim değişmez.
   - **Parametreler:**
     - `rules`: kuralların kimlikleri, virgülle; boş: hepsi;
     - `exceptions`: İstisnaları da listele (ilk değer kapalı).
   - **Çıktılar:**
     - `findings` tablosu: Kural, Sorun, Durum, Nesneler (#kimlik), Doğu, Kuzey, Ölçü (alan m², uzunluk m, açı °),
       Gerekçe;
     - `objects`: bulgulu nesneler;
     - `count`, `open` sayıları.
   - Bulgulu nesneler seçilir. Özet kural ve nesne sayısını ve durumların dağılımını söyler.
2. **Tek denetim:** araç komutun denetimini çağırır. `topology_check::check_on`, komutun `check_in`'inin salt okunur
   ve iptal edilebilir kapısıdır (`feedback.canceled`). Bu yüzden aynı bulgular, anahtarlar, yerler, ölçüler,
   durumlar ve düşen istisnalar çıkar. Komutun uyarıları (eksik bant, düşen istisna) araç günlüğüne yazılır. Ret
   sözü komutunkidir: “Projede “r9” kimlikli topoloji kuralı yok.”
3. **Araç = komut:** araç İşlem araç kutusunda, modelde, toplu çalıştırmada ve geçmişte yer alır. Komut olarak
   `processing.run.topology.check` web durduğu için `catalog/v1/additions.json`'dadır. Takma adları komutun
   kendisininkiyle (`TOPOLOJIDENETLE`) çakışmasın diye `KURALDENETLE` ve `TOPOLOGYRULESCHECK`'tir.
4. **Kapsam:** Harita, Yol ve CBS (İşlem araç kutusunun olduğu her yer). Web aracı TODOS §0.2'dedir.

## Kanıt

- **Ortak İşlem durumları** (`fixtures/processing/v1/geometry.json`, belge `topology.kcad`). Bağımsız üretici
  `scripts/fixtures/geoprocess_cases.py`; KentOS kodu yok.
  - Belge `cad.topology.check` durumlarının beş parseli ve iki kuralıdır.
  - Bulgular elle hesaplanır: dikdörtgen çiftlerinin kesişim kutusu (alan, ağırlık merkezi) ve kuralın boyundan kısa
    kenar (boy, orta).
  - Üç durum: bütün kurallar (iki çakışma ve kısa kenar), yalnız r2, olmayan kuralın reddi.

  Doğrudan ve kopya üstünde çalışma (işçi hedefi) geçer; araçların varsayılanları belgeden okunur.
- **Üreticinin yeniden izdüşüm sayıları:** bu makinenin PROJ'uyla son basamakta farklı çıkıyor (benden önce de vardı).
  O satırlara dokunulmadı; dosyaya yalnız topoloji belgesi ve üç durum eklendi.
- **Masaüstü** (`processing::geometry_tests::the_topology_rules_from_the_toolbox_give_the_commands_findings`):
  - pencere komutun bulgularını tabloda gösterir;
  - beş nesne seçilir;
  - geri alınacak adım yoktur.

  Resimler `.run/shots/islem-geometri-topoloji-*`.

## Sonra

- **Veri formu yolu:** seçili nesnenin topoloji bulgularının Öznitelikler panelinde ve formda görünmesi.
- Bulguların hata katmanı olarak yazılması (QGIS Topology Checker gibi), istenirse.
- Taşınmaların sahnede işaretlenmesi (ADR 1262), üçlü birleşim ve çoklu köşe için açıklanabilir ret.
