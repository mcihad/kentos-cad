# 0998 — Plan paftalarında eşleşme çizgileri

Tarih: 2026-11-07. Durum: kabul (Y92). Kapsam ROAD-SEC-11 ("eşleşme çizgileri açık"; ADR 0908 §10).

## Sorun

ROAD-SEC-11 eşleşme çizgileri yüzünden [~] idi. Ardışık iki plan paftasının nerede birleştiği haritada
gösterilmiyordu; okuyucu bindirme bölgesinde hangi paftaya geçeceğini başlıktaki km'den çıkarıyordu.

## Araştırma (7 Kasım 2026)

- **Civil 3D plan üretimi**
  ([Plan Production Match Lines](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-6D055021-BD0E-4AA6-AB6D-3ECFD98F404B.htm),
  [To Create View Frames](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-879A407F-9F9F-49F3-842D-C6A1CA57AFD1.htm)):
  - Eşleşme çizgisi, iki görünüm çerçevesinin birleştiği yerde aksı kesen düz bir çizgidir.
  - "Insert Match Lines" ile her birleşmede oluşur.
  - Sol ve sağ yanında birer etiket olabilir.
- **KGM Haziran 2024 örnek paftaları:** eşleşme çizgisi yok. Plan haritasının uçlarında yön okları ve gidilen
  yerin adı var ("← KAMAN", "ANKARA-NİĞDE OTOYOLU →"); paftanın km aralığı bantlarda. Bu yüzden seçenek kapalı başlar.
- **Matematik:** çizgi aksın normali boyunca; yeri bindirmenin ortası. Her iki pafta da çizgiyi kendi haritasının
  içinde, kenara eşit uzaklıkta gösterir.

## Karar

- **`road.sheets.plan` `matchLines`** (yoksa kapalı):
  - Ardışık her iki pafta arasında, bindirmenin ortasındaki istasyonda aksı kesen bir çizgi. Plan haritasının
    yüksekliğinin %90'ı kadar, 0,5 kalınlıkta.
  - İki yanında çizgi boyunca, haritanın dönüşüne göre okunur, maskeli yazı: "EŞLEŞME ÇİZGİSİ KM : 1+360.00 · PAFTA
    Y-102". Önceki taraf sonraki paftayı, sonraki taraf öncekini anar.
  - Yazı yüksekliği paftanın ölçeğinde 2,5 mm.
- **Katman:** çizgiler çizim nesnesidir; boykesitin katmanına ya da `layerId`'ye çizilir. Katman yoksa
  `invalid_road_sheets`: "Eşleşme çizgileri bir katmana çizilir; layerId verin." Yalnız planlı takımda da çizilebilir.
- **Pencere:** Yol paftaları penceresinde "Eşleşme çizgisi: Yok / Var".
- **Saklama:** seçenek takımda saklanmaz. Proje yeniden üretimi pafta takımlarını yeniden çizmez, eskidiklerini
  söyler; kullanıcı yeniden yapar.

## Ölçüm

- **Bağımsız başvuru** (`road_sheets_cases.py`): eşleşme çizgisi başına 3 nesne.
  - Bindirmeli boykesitli ve yalnız planlı iki durum, bir ret.
  - `road.sheets.plan` 39 durum geçti.
- **Testler:** uygulama 143 + 521, masaüstü 1858.
- **Resim** `.run/gui-evidence/yol-y92/`: Ana aks 1:1000, 20 m bindirme, iki A3 pafta.
  - Y-101'in sağ ucunda ve Y-102'nin sol ucunda km 1+360.00'da çizgi (Y-101 1+000–1+370, Y-102 1+350–1+669).
  - Önceki yanda "PAFTA Y-102", sonrakinde "PAFTA Y-101" (`y92-yakin.png`).

## Açık

- **KGM'nin yön okları** ve gidilen yerin adı ("← KAMAN") çizilmiyor; yerin adı için aksa bir alan gerekir.
