# ADR 0701: Açıklığın geçici ölçüsü ve mesafeyle taşınması

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-EDIT-01'in ilk dilimi (açıklıklar).
- **Bağlam:** `plan/architecture.md` ARCH-EDIT-01 “Geçici ölçüyle doğrudan düzenleme”. ADR 0662 (hassas açıklık yerleştirme),
  ADR 0700 (duvar kenarının yüz doğrusu).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Mimar kapıyı ya da pencereyi çoğunlukla bir köşeden, komşu duvarın yüzünden ya da yandaki açıklıktan **net mesafeyle** yerleştirir
(“kapı iç köşeden 10 cm”, “iki pencere arası 1,20 m”). KentOS'ta yeni açıklık bunu duvar ucundan yapabiliyordu (ADR 0662), ama
çizilmiş açıklığın komşusuna olan mesafesini görmek için ölçü koymak, değiştirmek için `Taşı`'ya kaydırma miktarını hesaplamak
gerekiyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2023 — About Temporary Dimensions](https://help.autodesk.com/cloudhelp/2023/ENU/RevitLT-DocumentPresent/files/GUID-74FFB00B-FB29-44B7-8FEA-613DBE010AD4.htm):
  eleman seçilince en yakın dik elemana geçici ölçüler görünür; değer değiştirilince seçili eleman taşınır; kalıcı olması
  istenen ölçü kalıcı ölçüyle konur.
- [Revit 2023 — Move the Witness Line for Temporary Dimensions](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-DE82A2B9-0129-4D03-BC47-17A52BD4E168.htm):
  referans, uzatma çizgisinin denetimiyle başka bir elemana alınabilir; seçim oturumda hatırlanır.
- [Archicad — mesafe girme (Tracker)](https://helpcenter.graphisoft.com/?p=85299): taşıma sırasında değer, düzenleme başlangıcına
  göre yazılır; belirli bir noktaya göre ölçmek için kullanıcı başlangıç noktası konur.

KentOS kararı: Revit'in “seç, sayıya tıkla, yaz” akışı; referans görünür ve sabittir (sayının iki yanında hangi şeyin kaldığı
yazılıdır), değer yazılınca **yalnız seçili açıklık** taşınır. Ölçü değeri metni değiştirilerek sahte taşıma yapılmaz; ölçü
nesnesi yaratılmaz.

## Karar

### 1. Ortak hesap: yüz boyunca referanslar

Açıklığın bulunduğu duvar kenarının seçilen yüzünde (`left`/`right`, ADR 0700'ün yüz doğrusu: gönyeli yüz köşeleri arası)
yüz boyunca `s` konumları:

- açıklığın iki pervazı (yüz tarafındaki pervaz köşeleri, ADR 0694'ün yerleri), `s0 < s1`;
- **referans adayları:** yüz kenarının iki ucu (köşe ya da duvar ucu); aynı kenardaki öteki açıklıkların pervazları; başka
  duvarların yüz doğrularının bu yüz kenarını kestiği yerler (T birleşim, ayrı çizilmiş köşe duvarı). Yay yüzler ilk dilimde
  aday değildir.
- Başta (`start`) referans `s0`'dan önceki en yakın aday, sonda (`end`) `s1`'den sonraki en yakın aday. Mesafeler
  `s0 − r_start` ve `r_end − s1`, net ve yüz boyuncadır. Adayın türü ve varsa nesnesinin kalıcı kimliği söylenir.

Hesap ortak çekirdektedir (`ops::opening_distance`); masaüstü belgesi ve başsız sunucu aynı yolu kullanır. Yalnız düz kenar:
yay kenardaki açıklığın geçici ölçüsü yoktur ve komut açıklamalı reddeder.

### 2. Komutlar

- **`cad.openings.edit`'in yeni işlemi `distance`:** `{ kind: "distance", side: "start" | "end", face: "left" | "right",
  distance }`. Her açıklık, kendi referansı sabit kalarak, o yandaki net mesafesi `distance` olacak biçimde duvarı boyunca
  taşınır (`Move`'un denetimleriyle: duvar kenarında kalır, öteki açıklıklarla çakışmaz). Referansı olmayan yan, yay kenar ya
  da negatif/sonsuz mesafe `invalid_opening` ile reddedilir; hepsi ya da hiçbiri, tek geri alma adımı.
- **Salt okunur `cad.openings.distances` v1:** açıklıkların (kimlikleriyle) seçilen yüzdeki iki yan mesafesi ve referansları.
  Python ve MCP aracı katalogdan; ajan önce mesafeyi okur, sonra `distance` ile yazar.

### 3. Masaüstü

- Tek açıklık seçiliyken (komut çalışmazken) iki geçici ölçü, açıklığın yüzünde ve yüzün dışına küçük bir payla, ölçü
  önizlemesinin çizimiyle görünür; referans uçları işaretlidir. Yüz imlecin bulunduğu taraftır (kapalı duvarda başlangıçta
  iç yüz).
- Sayıya tıklamak, sayının yerinde değeri hazır bir yazı kutusu açar (ADR 0060'ın yazı kutusu). Enter yazar (tek adım), Esc
  hiçbir şey yazmaz. Uzunluk girişi projenin birimleriyle ve ek birimle okunur (`40 cm`, `0.4 m`).
- Ret iletileri komutun kendi sözleridir; yazılamayan değer yazı kutusu kapandıktan sonra durum satırında söylenir.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/opening_distance_cases.py` (KentOS kodu yok): yüz doğruları ve referanslar elle hesaplanır;
  köşe, duvar ucu, komşu açıklık, T birleşimdeki duvar yüzü, sağ ve sol yüz, büyük koordinat, dönük oda, yay kenar reddi,
  taşma ve çakışma retleri, tek geri al.
- Masaüstü: kapı seçilince iki geçici ölçü, sayıya tıklayıp yazma ve geri al (resimler).
