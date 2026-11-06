# 1000 — Plan paftasının uçlarında gidilen yerin oku

Tarih: 2026-11-07. Durum: kabul (Y94). Kapsam ROAD-SEC-11, ADR 0998 §Açık ("KGM'nin yön okları") ve KCAD şeması.

## Sorun

KGM'nin plan paftaları yolun nereye gittiğini haritanın iki ucunda bir okla söyler. KentOS'ta aksın başının ve sonunun
ötesindeki yeri tutan bir alan yoktu; paftada da yazılmıyordu.

## Araştırma (7 Kasım 2026)

- **KGM Haziran 2024 örnek paftaları:**
  - s. 9: plan haritasının solunda kırmızı "← KAMAN", sağında "ANKARA-NİĞDE OTOYOLU →".
  - s. 8–12: oklar her paftada yineleniyor. Proje başı ya da sonu paftanın içindeyse ok o ucun yanında (s. 8 başta
    "KAMAN").
  - Ok yolun üstünde, yol boyunca; ad okun üstünde, kırmızı ve büyük harf.
- **KGM 2019 şartnamesi:** okları yazmaz; örneğin uygulamasıdır.
- **Civil 3D:** bu araştırmada hizalamanın uçlarına "gidilen yer" yazan hazır bir alan bulunamadı.
- **Türkçe büyük harf:** `to_uppercase` "Niğde"yi "NIĞDE" yapar (noktasız I). Ad kullanıcının yazdığı gibi yazılır;
  KGM gibi büyük harf isteyen kullanıcı öyle yazar.

## Karar

- **`AlignmentDef.startPlace` / `endPlace`** (KCAD **şema 194**): yolun başının ve sonunun ötesinde gidilen yer.
  - Boş olmayan, kenarında boşluk olmayan, en çok 80 harf, denetim karakteri yok.
  - Paralel aksın kendisininki olmaz.
  - Geometri değildir: aksın revizyonu artmaz, ona bağlı boykesit ve çizelgeler eskimez.
- **Komutlar:**
  - `road.alignments.create` iki alanı alır (kırpılır, boşsa yok).
  - `road.alignments.edit` `startPlace`/`endPlace` alır; "" kaldırır.
- **KCAD:**
  - Yazıcı bir aks yer taşıyınca 194'ü seçer.
  - 193 ve öncesi alanları `unknown_field` sayar (`broken/road-places-in-schema-193.kcad`).
  - Örnek `road-places.json/.kcad`; desteklenmeyen sürüm örneği `schema-version-195.kcad`.
- **Plan haritası** (`road_plan_axes::places`): yalnız bir haritada (`Room`).
  - Aksın haritanın 4 mm içinde kalan ilk ve son noktasında, 4 mm yanında, yol boyunca dışa bakan 15 mm'lik kırmızı ok
    (çizgi ve dolu baş).
  - Okun üstünde maskeli, okunur kırmızı ad.
- **Güzergâh penceresi:** "Başında gidilen yer" ve "Sonunda gidilen yer" alanları.

## Ölçüm

- **Ortak durumlar** (`road_alignment_command_cases.py`):
  - create: iki ad (kırpılarak), 80 harften uzun ad reddi, paralel aksın yeri reddi.
  - edit: yazılır ve revizyon artmaz; "" kaldırır; "değiştirilecek bir şey yok" iletisi iki alanı da sayar.
  - `road.alignments.*` durumlarının hepsi geçti.
- **KCAD:** referans yazıcı ve Python okuyucusu 1648 dosyada tutarlı; Rust `kentos-kcad` 242 test.
- **Test** `the_places_beyond_the_ends_are_named_on_the_map`:
  - Yolu gösteren haritada iki ad, kırmızı.
  - Başka yerdeki haritada ve yeri olmayan aksta ad yok.
- **Resimler** `.run/gui-evidence/yol-y94/`:
  - Güzergâh penceresi 1440 ve 1100 px, taşma yok.
  - İki A3 pafta. Y-101'de "KAMAN" proje başında, "ANKARA-NİĞDE OTOYOLU" sağ kenarda. Y-102'de "KAMAN" sol kenarda,
    "ANKARA-NİĞDE OTOYOLU" proje sonunda (`y94-oklar.png`).

## Açık

- **LandXML'de karşılığı** (yer adı) yok; içe ve dışa aktarmada taşınmıyor.
