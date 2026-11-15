# Değişiklikler

## 0.2.0

Olgular artık mahalin türünü ve dar kenarını, projenin yağmurlama ayarını ve konut kapısının kaçış merdiveni mahaline
açılıp açılmadığını veriyor (KentOS'un ADR 8057 §53'ü); kurallar bunlarla karar verir.

- **`piyesler`** (yeni; 0.1.0'ın `oturma-odasi` kuralının yerine): PAİY 29/1'in beş piyesi. Türü bilinen her piyesin
  net alanı ve dar kenarı tablonun en azına (hata, iki ölçüyü de söyler). Eksik piyes yönetmeliğin istisnalarıyla:
  3 veya daha az odalı konutta tuvaleti de karşılayan banyo (29/2, 29/4: en az 4.20 m²), mutfak nişi (29/4: odanın
  alanı ile 3.30 m²'nin toplamı), yatak nişi (5/25); istisna mümkünse "karar verilemedi" (bilgi), değilse hata.
  Piyeslerinin hiçbirinin türü okunamayan konutta 0.1.0'ın 12 m²'lik piyes denetimi sürer.
- **`koridor`** (yeni): hol ve koridor 1.20 m'den dar olamaz (PAİY 29/3).
- **`daire-ici`, `daire-kapisi`**: sınır projenin yağmurlama ayarınındır, aşılırsa hata; ayarı vermeyen eski bir
  KentOS'un olgularında iki sınır (uyarı ve hata) sürer.
- **`daire-kapisi`**: kaçış merdiveni mahaline açılan konut kapısı uygundur; hiçbir kat çıkışına ulaşmayan kapı her
  binada hata (BYKHY 48/3, 33/6-b).

## 0.1.0

- İlk sürüm, mimari çizimin ölçülmüş olgularıyla karar verilen dört kural:
  - Oturma odasının alanı: konut bağımsız bölümünde net alanı 12.00 m²'ye ulaşan piyes yoksa (PAİY 29/1, 5/25).
  - Daire içinde kapıya uzaklık: konutun içinde kapıya 20 m, yağmurlama sistemiyle 30 m (BYKHY 48/1, 48/2).
  - Daire kapısından kat çıkışına: Ek-5/B'nin apartmanlar satırı (BYKHY 32/3, 48/1, 48/4).
  - Kat çıkışına erişim: kat çıkışı olmayan kat, kendi bölümünden ve ortak alanlardan çıkışa ulaşmayan mahal
    (BYKHY 31/3, 33/6-b).
