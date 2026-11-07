# 0997 — Enkesitlerin varsayılan yeri: çizimin altı

Tarih: 2026-11-07. Durum: kabul (Y91). Kapsam ROAD-SEC-11 ve ADR 0991 §Açık ("enkesit penceresi").

## Sorun

ADR 0991 boykesit ve plan–profil paftaları için boş Yer'i "çizimin altı" yaptı. Enkesit penceresi hâlâ "PI'ların
600 m altı" tahminini yazıyordu. Çizimin kutusu PI'lardan genişse ya da altında başka çizim varsa enkesitler onların
üstüne düşebiliyordu. İki pencere iki kurala göre davranıyordu.

## Araştırma

- **`road.sections.draw` ve `road.sheets.sections`:** `at` görünüm ızgarasının **sol üst** köşesidir. Satırlar ve
  pafta grupları aşağı doğru büyür (ADR 0903 §7, 0908 §11). İlk satırın çerçevelerinin üstünde km yazısı vardır.
- **Civil 3D:** kesit görünümü grubunun yerini kullanıcı seçer; ADR 0991'deki araştırma aynen geçerli.
- **Mevzuat ve matematik:** yeni bir şey yok.

## Karar

- **Yer boş başlar,** ipucu "Çizimin altı". Yazılırsa eskisi gibi kullanılır.
- **Boşsa:**
  1. Görünümler önce sıfırda planlanır (`road_sections_draw::plan`, hiçbir şey yazılmaz).
  2. En yüksek çerçeve tepesine bir yazı satırı (km) eklenir; bu, `at`'ın üstüne çıkan kısımdır.
  3. ADR 0991'in `under_drawing` kuralıyla sol üst köşe çizimin kutusunun 20 kâğıt mm altına, sol kenarına konur.
- **Aynı yer hem "Çizime yaz" hem "Paftalara diz" için kullanılır.** Pafta grupları ilk grubun altından iner.

## Ölçüm

- **Test** `an_empty_place_puts_the_sections_under_the_drawing`: çizimde de paftada da yazılanların kutusu çizimin
  kutusunun altında.
- **Masaüstü testleri** geçti.
- **Resim** `.run/gui-evidence/yol-y91/y91-model.png`: iki satır enkesit planın altında, sol kenardan başlıyor.
