# 1135 — Coğrafi projede metre isteyen işlemler çalıştırılmadan reddedilir

Tarih: 2026-11-20. gis-usage-report.md GIS-064, GIS-065, GIS-066. Durum: kabul edildi (kısmi).

İlgili: ADR 1097 (GIS-040: ölçü elipsoit üstünden), CLAUDE.md §5 (sessiz dönüşüm ya da tahmin yok), §23.3.

## Sorun

EPSG:4326 projede 10 m tampon 22°–42° D'ye yayılan, merkezden 898–1 111 km uzanan bir alan olarak kaydedildi; “En
yakını bul” daha uzak istasyonu seçip 10 km sınırının dışındakini yazdı ve uzaklık matrisi sırayı ters verdi; “10 metre
içinde seç” 10–11 km uzaktaki istasyonları seçti. Araçlar uzaklığı çizimin biriminde (derece) ölçüyordu.

## Karar

İşlem çalıştırıcısının doğrulaması (`Runner::validate`; pencere, `processing.run.*` komutu, model ve toplu çalıştırma
aynı yoldan geçer) projenin kendi sistemi coğrafiyse:
- birimi metre olan ve sıfırdan farklı bir değer verilmiş her sayı parametresini,
- yakınlık araçlarında (`proximity.*`) metre parametresini değeri ne olursa olsun (uzaklıkla ölçer ve sıralar)
çalıştırmadan reddeder: parametre adıyla, nedeniyle (“uzaklığı çizimin biriminde, derecede ölçer; sonuç yanlış yere
düşerdi”) ve çıkış yoluyla (Koordinat dönüştür ile TUREF/TM ya da UTM dilimine taşıyıp orada çalıştırma). Uzaklığı 0
olan Konuma göre seç (yalnız dokunma, kesişme) çalışır.

## Sınır

Elipsoit üstünde tampon (jeodezik halka) ve jeodezik en yakın/uzaklık seçimi yapılmadı; yerel bir projeksiyonda
hesaplayıp kaynağa dönmek de. Bu araçlar coğrafi projede şimdilik çalışmaz, nedeni söylenir. Çizgi, çoklu halka ve
tarih değiştirme çizgisi regresyon örnekleri bu yöntemle birlikte gelecek.

## Kanıt

`apps/desktop/src/gis/processing/tests.rs` `metres_in_a_geographic_project_are_refused_before_the_run`: WGS 84
projede Tampon 10 m, En yakını bul ve Konuma göre seç 10 m reddedilir, ileti parametreyi ve Koordinat dönüştür'ü anar;
TM36 projede Tampon reddedilmez. İşlem testleri ve paylaşılan örnekler (bilinen 41 dışında) değişmedi.
