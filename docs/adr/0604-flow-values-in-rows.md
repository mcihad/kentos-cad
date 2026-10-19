# ADR 0604: Akışta değerler giriş satırında

- **Durum:** kabul edildi (2026-10-19). ADR 0600'ün akış dilimi; sahibin sözü: “expression editör ve flow yeterince iyi değil”.
- **Bağlam belgesi:** ADR 0101 (ifadenin akışı), 0600.
- **Web:** ertelendi (ADR 0228); çekirdek ortaktır, web'in çizicisi kutuları TODOS.md §0.2'deki satırla öğrenir.

## Bağlam

Akış her yaprağı ayrı düğüm çiziyordu: `'Arsa'`, `500`, `2` gibi sabitler, `Nitelik` ve `$alan` gibi alanlar ve değerler. Ağacın her
katı bir sütundu, yaprakları en solda ayrı bir sütun daha açıyordu. Sıradan bir koşul
(`durum eğer Nitelik = 'Arsa' ve $alan > 500 ise yuvarla($alan / 1000, 2) yoksa 0 son`) on beş düğüm ve beş sütundu. Pencereye
sığdırmak için ölçek okunurluk sınırına (0,72) düşüyordu; düğümlerin yazıları 8–9 piksele iniyor, şemanın solu kesiliyordu.

## Karar

1. **Yaprak girişin satırında kutudur.** Bir düğümün girişine bağlı sabit (sayı, metin, doğru/yanlış, boş), alan, `$` ya da `@`
   değeri ayrı düğüm değildir:
   - kendi sütunu ve kenarı yoktur;
   - o girişin satırının sağında, türünün renginde bir kutu olarak durur (`FlowNode.inline`: ebeveyninin kimliği);
   - kutunun yeri ve boyu çekirdektendir (`CHIP_X`, `CHIP_H`); iki platform aynı yerleri alır.
   - Ağacın kökü olan yaprak (sonuca doğrudan bağlı ya da hiçbir yere bağlı olmayan) düğüm kalır.
2. **Kutu düğüm gibi seçilir ve düzenlenir:** kimliği aynıdır (`0.0.0.1`), denetçi değerini, alanın ya da değerin adını değiştirir,
   Delete siler. Girişin noktasından çekilen kutu ayrılır ve düğüm olur. Kutudan tel çekilmez.
3. **Okunurluk:** düğüm 168 birim genişliğinde (140'tı), yazılar bir punto büyük (başlık 13, satırlar 12), açılıştaki en küçük ölçek
   0,8 (0,72'ydi).

Örnek koşul altı düğüm ve dört sütundur; 1440 × 900'de bütünüyle 0,85 ölçekte görünür.

## Doğrulama

- Çekirdek: `editor::flow::tests` (kutunun ebeveyni, yeri, satırla hizası, çizilen çocuğun ortalanması);
  `fixtures/expression/v2/flow.json` yeniden yazıldı (yalnız yerler, sınırlar ve `inline`); metin gidiş-dönüşü değişmedi.
- Masaüstü: `expression::tests::flow_screens` üç durum (akış, düğüm, seçili kutu ve denetçisi), iki boy, iki tema;
  `.run/shots/ifade-akisi-*`.
