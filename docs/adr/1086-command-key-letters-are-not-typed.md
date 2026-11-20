# 1086 — ⌘ kısayolunun harfi metin kutusuna yazılmaz

Tarih: 2026-11-20. gis-usage-report.md GIS-025. Durum: kabul edildi (gerçek klavyeyle deneme açık).

## Sorun

macOS'ta alt paneldeki formda bank sayısına 18 yazıp ⌘S basılınca alan “18s” oldu (tam sayı uyarısı); park koduna
“ANK-01-T” yazıp ⌘S basılınca “ANK-01-Ts” oldu ve geçerli metin sayılıp kaydedildi. iced'in metin girişi, tuş olayının
metnini Komut tuşuna bakmadan ekliyor ve olayı yakaladığı için ⌘S kaydetme kısayoluna da ulaşmıyordu.

## Dayanak

- Apple, Mac keyboard shortcuts ([belgesi](https://support.apple.com/en-us/102650), 20 Kasım 2026'da okundu): ⌘S
  belgeyi kaydeder; ⌘ ile basılan harf metin girmez.

## Karar

1. `kentos_ui::widget::command_keys` sarmalayıcısı, macOS'ta ⌘ (logo) basılıyken gelen tuş olayını içindekilere
   metinsiz verir: hiçbir metin kutusu ya da düzenleyici kısayolun harfini yazmaz. Option ile yazılan harfler (Türkçe
   Q'da @) ve düz harfler olduğu gibi kalır; başka işletim sistemlerinde Ctrl'li harf zaten denetim karakteridir.
2. Masaüstünün kök görünümü bununla sarılıdır. Olayın aslı uygulamanın klavye aboneliğine gider: alan odaktayken ⌘S
   artık projeyi kaydeder (alan dışındaki gibi); formun kendi Kaydet'i ayrı kalır, taslak alanda durur.

## Sınama

- `kentos-ui` `command_keys::tests::a_command_key_types_nothing_and_a_plain_s_types` (macOS, snapshot): odaktaki
  “18” kutusunda ⌘S hiçbir şey yazmaz, düz s “18s” yazar.
- Gerçek pencerede gerçek klavyeyle (metin, sayı, panel dışı odak) bu oturumda denenmedi.
