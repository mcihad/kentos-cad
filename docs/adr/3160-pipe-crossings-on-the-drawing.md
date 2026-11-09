# 3160 — Boru kesişimlerinin planda gösterilmesi

Tarih: 2026-11-09. plan/map-cad.md MAP-16, beşinci tur. Durum: taslak. ADR 2360'ın “Sonra”sındaki “kesişimlerin planda
gösterilmesi”.

İlgili:
- [ADR 2360](2360-pipe-clearance-to-another-network.md): başka bir altyapı ağının borularına açıklık (kesişim, yakın
  geçiş, bulgular);
- [ADR 2760](2760-crossing-pipes-in-the-pipe-profile.md): hat profilinde kesişen borular;
- [ADR 3040](3040-road-width-overlaps-hatched.md), [ADR 3060](3060-infrastructure-nearness-on-the-drawing.md): İşareti
  kaldır'a dek kalan, çizim değişince yeniden hesaplanan işaret deseni.

## Dayanak

- **Autodesk Civil 3D:**
  - [About Checking for Pipe Network Interferences](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-B7947549-4D31-476F-BC1C-880397DF20C6.htm)
    (9 Kasım 2026'da okundu): girişimler stil tabanlı görsel işaretlerle ya da gerçek 3B gösterimle belirtilir;
  - [To Update a Pipe Network Interference Check](https://help.autodesk.com/cloudhelp/2016/ENG/Civil3D-UserGuide/files/GUID-75082346-754A-4108-8910-930881239FB4.htm)
    (9 Kasım 2026'da okundu): denetimdeki bir ağ parçası değişince denetim “eski” işaretlenir. Kullanıcı Prospector'dan
    yeniden çalıştırır.
- **KentOS bugün** (ADR 2360): Boru hattı denetimi kesişimleri ve yakın geçişleri tabloda yer koordinatlarıyla
  listeliyordu. Çizimde nerede olduklarını görmek için kullanıcı koordinatı kendisi arıyordu.

## Karar

1. **Kesişimleri göster** (Boru hattı denetimi, Diğer ağ ile; kesişim ya da yakın geçiş varken):
   - pencere kapanır; her kesişim yerinde çift halka ve düşey açıklığı (“0.42 m”, bilinmiyorsa “?”) sağ üstünde,
     zemin rengi haleli yazıyla;
   - her yakın geçiş iki borunun en yakın yerleri arasında kesikli çizgi ve açıklığı (çaplar bilinmiyorsa eksenler arası
     uzaklık);
   - ileti kesişim, sorunlu, açıklığı bilinmeyen ve yakın geçiş sayılarını söyler.
2. **Renk komutun kendi yargısıdır** (`cad.pipes.check`'in bulguları, o yerde):
   - çakışma (`clash`), en azdan yakın kesişim (`closeCrossing`) ya da yanlış taraf (`wrongSide`): tehlike rengi;
   - açıklığı bilinmeyen (akar ya da çap yok): uyarı rengi;
   - öbürleri başarı rengi;
   - yakın geçiş yalnız en azdan yakınsa listelendiği için hep tehlike rengidir.

   Pencere bulguyu yeniden hesaplamaz; komutun bulgusu kesişimle aynı yere yazılır, eşlik kesin eşitlikle bulunur.
3. **Kalıcılık:** işaret İşareti kaldır'a (`data.unmark`) ya da başka çizime dek kalır. Civil 3D'den farklı olarak
   eskimez: çizim değişince aynı soruyla yeniden hesaplanır. Su borusu kaldırılınca halkası kalkar. Hesap
   `pipes_check::preview`'dur (komutun salt okunur yolu).
4. Yazılan bir şey yoktur; komut ve sözleşme değişmez.
5. **Kapsam işareti:** ADR 2360'taki gibi **Harita + Yol + CBS**.

## Kanıt

- **Birim testi** (`pipes_marks.rs`): çakışma bulgusu olan kesişim tehlike, açıklığı bilinmeyen uyarı, öbürü başarı;
  aynı yerdeki başka türden bulgu kesişimin değildir; yazılar (“-0.20 m”, “?”, “0.25 m”).
- **Masaüstü** (`pipes_check_window_tests.rs`, `the_crossings_are_marked_on_the_drawing`):
  - ortak durumun soruyla (düşeyde en az 0,5 m, su üstte; yatayda 3 m) Kesişimleri göster pencereyi kapatır;
  - her işaret kesişimin yerinde ve komutun yargısında; durumda sorunlu kesişim vardır;
  - İşareti kaldır sunulur;
  - su borularının hepsi uzağa taşınınca işaret kalkar;
  - İşareti kaldır işareti siler.

  Boru hattı denetiminin öbür testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/kesisimler-3160/`:
  P1 × W3 0,42 m (en azdan yakın) ve P2 × W1 −0,18 m (çakışma) kırmızı, P3 × W2 0,53 m kırmızı (Bu ağ üstte, su üstte
  olmalı), W7 turuncu “?”, yakın geçiş 2,80 m.
- **MCP ve Python:** komut değişmedi.

## Sonra

- Kesişen borunun dış duvarı ve et kalınlığı (ADR 2760 Sonra).
- Seçili satırın çizimde öne çıkarılması (ADR 3020'deki gibi).
- Yüzey kaplaması (MAP-16'nın öbür maddesi); ~~profilin canlı güncellenmesi~~ [ADR 3560](3560-pipe-profile-kept-with-its-network.md).
- Web (TODOS §0.2).
