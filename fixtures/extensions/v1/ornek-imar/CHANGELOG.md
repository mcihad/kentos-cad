# Değişiklikler

## 1.1.0 — 14 Kasım 2026

- İşlemler: parsel TAKS hesabı (Planlı Alanlar İmar Yönetmeliği md. 4). Yapıların parselin içinde kalan kısımlarının
  alanlarını toplar, parsel alanına böler; plan notundaki sınırı aşanları işaretler, parselleri yeni katmana yazar ve
  sınırı aşanları çizimde seçer. İmar sekmesinde TAKS düğmesi aracın penceresini açar.
- Yaklaşma mesafesi denetimi katmanları kimlikleriyle de bulur: KentOS'un CBS şablonundaki "Parsel sınırı" (parsel) ve
  "Yapı" (yapi) katmanları.
- NumPy artık açıkça bağımlılık (shapely zaten getiriyordu).

## 1.0.0 — 14 Kasım 2026

- İmar sekmesi: yaklaşma mesafelerini denetle (shapely 2.0 serisi). Seçili yapı varsa yalnız onları denetler;
  sınırın içinde ya da parsel dışında kalanları raporlar ve çizimde seçer.
