# 0994 — Boykesit penceresinde şev ve platform satırları

Tarih: 2026-11-07. Durum: kabul (Y88). Kapsam ROAD-SEC-11 ve ADR 0989 §Açık ("boykesit penceresinde koridor seçimi
yok").

## Sorun

KGM'nin şev ve platform satırları (ADR 0989) yalnız Yol paftaları penceresinden çizilebiliyordu. Boykesiti tek başına
çizen kullanıcı satırları ancak Python ya da komut satırıyla alabiliyordu.

## Araştırma

- **Mevzuat ve matematik:** yeni bir şey yok; ADR 0989'daki KGM 2024 satırlarıdır. Bu bir pencere eksikliğidir.
- **Pencere düzeni:** Y84'ün 1100 px resminde Çizime satırı (Katman, Yer, Bantlar, Kot aralığı, Dever bandı, Çizime
  yaz) pencerenin kenarına dayanıyordu. Bir seçim daha sığmaz.

## Karar

- **"Şev ve platform" seçimi:** boykesitin aksındaki koridorlar, varsayılan "Satır yok". Yol paftaları penceresiyle
  aynı adlar.
- **KGM'ye özgü iki seçim ikinci satırda:** Bantlar KGM 2019 iken Dever bandı ve Şev ve platform. Çizime yaz düğmesi
  ilk satırın sağında kalır.
- **KentOS bantlarında seçim gönderilmez;** komut onu reddederdi (ADR 0989).
- **Yer boşken** (ADR 0991) yükseklik hesabına çerçevenin üstündeki satırlar da katılır. Yol paftaları penceresi
  bunu zaten yapıyordu.

## Ölçüm

- **Test** `the_window_draws_the_corridor_rows`:
  - KentOS bantlarında "PLATFORM GEN." yok; KGM'de 1.
  - Satırlar dahil çizilen her şey çizimin kutusunun altında.
- **Masaüstü** 1857 test geçti.
- **Resimler** `.run/gui-evidence/yol-y88/`:
  - 1100 px koyu pencerede iki satır, taşma yok.
  - İlk çizim resminde satırlar planın alt noktalarına değiyordu: Yer hesabı yalnız çerçevenin tepesini sayıyordu.
    Düzeltmeden sonra arada boşluk var (`y88-cizim-*.png`).
