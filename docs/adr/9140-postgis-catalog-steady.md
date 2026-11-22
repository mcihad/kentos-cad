# ADR 9140: PostGIS kataloğu aranırken yerinde durur; seçilen tablo kimliğini söyler

- **Durum:** kabul edildi (2026-11-22); adım VY-39. Plan: [plan/veri.md](../../plan/veri.md) VY-39.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  7. maddesi (P1):
  - “park” aranınca sonuç kümesi daraldı, pencere yeniden ortalandı; arama alanı ve düğmeler yer değiştirdi;
  - katmanın kimliğinde bağlantı, şema, tablo, geometri sütunu, birincil anahtar, SRID, yetki ve kapsam sunulmalı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- PostGIS'e bağlan penceresinde (`exchange/postgis.rs`) tablolar diyaloğun kaydırılan gövdesindeydi. Liste kısalınca
  diyalog kısalıyor ve ortalanıyordu.
- Katalog her tablo için şunları zaten okuyordu (`probe::Relation`): şema, ad, tür, geometri türü, SRID, satır
  tahmini, anahtar, okuma ve yazma hakkı. Seçilen tablonun bunları bir arada gösterilmiyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasının PostGIS kaynak seçimi tabloları sabit bir ağaçta şemalarına göre
  listeler, arama ağacı süzer, pencere yerinde kalır. Satırda geometri sütunu, türü, SRID ve birincil anahtar görünür.
  Kaynak: [PostGIS kaynak seçimi, 4.2 kaynak kodu](https://github.com/qgis/QGIS/blob/release-4_2/src/providers/postgres/qgspgsourceselect.cpp)
  (değerlendirmenin bağlantısı).

## §2 Karar

1. **Yerinde duran liste.** Tablolar kendi kaydırma alanında, sabit yükseklikte (280 mantıksal piksel, yazı ölçeğiyle)
   listelenir. Arama alanı ve Görünenleri seç / Hiçbiri bu alanın üstünde kalır. Arama listeyi daraltır; pencere ve
   arama yerinden oynamaz.
2. **Seçilen tablolar.** Listenin altında her seçilen tablo bağlanacağı gibi yazılır: “kent.park · anahtar fid · çok
   parçalı alan · ≈1.240 satır · EPSG:5256 · yazılabilir”. Yazma hakkı olmayan ya da görünüm olan tablo “salt okunur
   (yazma yetkisi yok ya da görünüm)” der.

## §3 Kanıt

- **Masaüstü** (`exchange/postgis_tests.rs`): seçilen tablonun kimlik satırı ve salt okunur durumu; PostGIS penceresi
  sınamaları (13) geçiyor.
- **Resimler** (`.run/gui-evidence/kullanim-9140/`, Git'e girmez): 35 tablolu katalog tümüyle ve “park” aramasıyla.
  Pencerenin, arama alanının ve düğmelerin yeri iki resimde aynı; liste kendi alanında kayıyor.

## §4 Kalan

- **Katalog.** Şema ağacı (aç/kapa), ilişkili öznitelik tabloları ve sık kullanılanlar.
- **Kimlik.** Geometri sütununun adı ve tablonun kapsamı: katalog sorgusu bunları okumuyor.
- **Canlı sınamalar.** Ayrı bir sınama veritabanında salt okunur rol, yetkili rol, anahtarsız görünüm, ağ kesintisi,
  ikinci istemci çakışması ve yeniden bağlanma. Bu makinede Docker yanıt vermediği için koşturulamadı.
- Web (TODOS.md §0.2).
