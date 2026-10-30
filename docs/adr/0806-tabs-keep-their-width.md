# ADR 0806: Sekmeler daralmaz: oklar ve liste

- **Durum:** kabul edildi (2026-10-30), dal `launcher-zarif`.
- **İstek:** sahibin sözü: “sekme bileşenine el atmanı istiyorum mesela katmanların olduğu yerde yan yana sekmeler var ama aralarında geçiş çok zor neyin ne olduğu belli değil ikonlar hep üst üste
  binmiş sekmelerin boyutu daraldığından sanırım sekmelerin boyutları daralmasın sadece sağa sola kaydırma okları çıksın veya bir tane açılır menü kalsın oradaki aynı zamanda oklarda olsun”.
- **Bağlam belgesi:** DESIGN.md §7.5, §7.6; ADR 0804, 0805.
- **Web:** bir şey değişmez (ADR 0228); TODOS.md §0.2. **`.kcad`, yerleşim dosyası:** değişmez.
- **Bağımlılık:** yeni dış kitaplık yok.

## Bağlam

Yuvanın sekmeleri yer daralınca ikonlarına iniyordu (28 piksel); hemen sağlarında panelin eylem ikonları duruyordu: yan yana altı yedi ikon, hangisinin sekme hangisinin düğme olduğu belli değildi. Alt panelde
ve belge sekmelerinde başlıklar 110 piksele daralıp kesiliyordu (“Komut geç”, “Koordinat li”). Arkadaki sekme seçilince genişliyor, şerit oynuyordu. Taşan sekmeye yalnız menüden ulaşılıyordu.

## Karar

Üç şerit (`Tabs`, yuvanın yığını, alt panel) tek kurala uyar: `tabs::Scroll`.

1. **Sekmeler daralmaz.** Her sekme başlığı kadar geniştir (`MIN_WIDTH`–`MAX_WIDTH`), ikonuna inmez. Kapatma düğmesinin yeri her sekmede ayrılır: seçilince genişliği değişmez, şerit oynamaz.
2. **Sığmayan şerit kayar.** Sağda `‹ ›` okları çıkar, her basış tam bir sekme kaydırır; tekerlek de kaydırır. Kesilen kenar zemine doğru solar: o yönde sekme var. Kayacak yer yoksa ok sönüktür. Oklar sağda yan yanadır: şerit
   sola doğru hiç kaymaz, ilk sekme yerinde kalır, ard arda basmak için imleç oynamaz.
3. **Bir de liste.** `Tabs`'ta oklardan sonra `⌄`: bütün sekmeler, etkin olan işaretli. Yuvada aynı listeyi `⋯` taşır (“Paneller”), ikinci bir düğme eklenmez.
4. **Etkin sekme görünür.** Seçilen sekme kenardan içeri alınır; şeridin yeri değişince de. Okuyan kişinin kaydırması bunun dışında geri alınmaz: etkin sekme bir an dışarıda kalabilir.
5. **Eylemler sekmelerle karışmaz.** Yuvada panelin eylemleri sekmelerin yanına sığarsa başlıkta, sığmazsa altında kendi satırındadır (`Frame::toolbar`, panelin zemininde, ince çizgili); daraltılmış yığında görünmez. Kısa bilgi yazısı
   (`Pane::meta`: “3 nesne”) kendine satır açmaz, sığmazsa görünmez.
6. Şeridin yüksekliği satırların modülüdür (`metrics::row`, ADR 0804).

## Sonuçlar

- Bir bakışta hangi panelin açık olduğu ve hangi panellerin bulunduğu okunur.
- Varsayılan genişlikte (300 px) iki sekme yazısıyla görünür, kalanına oklar, tekerlek ya da menü götürür. Yuva genişletilince oklar kendiliğinden kalkar.
- Eylemi olan panel 28 piksel daha az gövdeyle açılır; karşılığında eylemleri sekme sanılmaz.
- `tabs::fit`, `tabs::window`, `SHRINK` ve yuvanın ikon sekmesi kalktı.

## Sınırlar

- Sürüklenen sekme kenara gelince şerit kendiliğinden kaymaz; önce kaydırıp sonra sürüklemek gerekir.
- Basılı tutunca ok sürekli kaydırmaz.
- Klavyeyle şerit kaydırma yok.
- Elle gerçek pencerede denenmedi; doğrulama gerçek olaylı testler ve ekran resimleriyledir.
