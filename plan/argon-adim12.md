# ADIM 12'nin özellikleri: büyük veri, modüller arası canlı altlık, 4'lü sahne

[plan/argon-tavsiye.md](argon-tavsiye.md)'nin ADIM 12'si (P2). **13 Kasım 2026, sahibin kararı:** yeniden düzenleme değil yeni ürün
özellikleri olduğu için Faz 1'den ayrıldı. Her biri kendi maddesi: önce araştırma (Türkiye mevzuatı, benzer yazılımlar,
kaynak gösterilerek), sonra kod, önce/sonra ekran görüntüleri, ölçümler ve ADR.

## [ ] 12.1 — Büyük veri bellek sınırı

**Büyük Veri Sınırı:** LiDAR/COPC nokta bulutları, milyonlarca üçgenlik TIN yüzeyleri ve büyük GeoPackage/PostGIS katmanları `Document` içindeki `Entity` yuvalarına kopyalanmayacak; `Document` yalnız kaynak kimliğini, CRS/datum bilgisini ve değişmez revizyon özetini (`revision_id`) tutacaktır.

## [ ] 12.2 — Disiplinlerarası canlı altlık (`Xref` ve `SourceLink`)

**Disiplinlerarası Canlı Altlık (`Xref` & `SourceLink`):** *Harita → Mimari* (aplikasyon/parsel altlığı), *Harita → Yol* (onaylı halihazır TIN ve kamulaştırma sınırı) ve *Mimari/Yol/Harita → GIS* bağlantı sözleşmesi standartlaştırılacak; kaynak dosya değiştiğinde bağlı modülde hangi hesapların eskidiği (`stale`) gösterilecektir.

## [ ] 12.3 — Yol ve harita için çoklu görünüm

**Yol ve Harita Çoklu Görünüm:** [apps/desktop/src/split_view.rs](file:///home/cihad/Projects/kentos-cad/apps/desktop/src/split_view.rs) genişletilerek **Plan + Boykesit + Enkesit + 3B Perspektif** sahnelerinin aynı pencerede 2×2 / 1+2 düzende ve ortak **İstasyon/Km İmleci (`StationCursor`)** ile eşzamanlı çalışması sağlanacaktır.
