# Gömülü yazı tipleri

kentos-ui bu yazı tiplerini `fonts` özelliğiyle (varsayılan açık) ikili dosyaya
gömer; uygulamanın çalıştığı makinede kurulu olmaları gerekmez. Hepsi
[SIL Open Font License 1.1](https://openfontlicense.org) ile dağıtılır; her
ailenin lisansı ve telif satırı `OFL-*.txt` dosyasındadır.

| Dosyalar                     | Kaynak                                                      | İşlem                          |
|------------------------------|-------------------------------------------------------------|--------------------------------|
| `IBMPlexSans-*.ttf`          | [IBM/plex](https://github.com/IBM/plex), `packages/plex-sans/fonts/complete/ttf` | Değiştirilmedi (sürüm 3.005) |
| `IBMPlexMono-*.ttf`          | [google/fonts](https://github.com/google/fonts), `ofl/ibmplexmono`               | Değiştirilmedi               |
| `Inter-*.ttf`                | google/fonts, `ofl/inter/Inter[opsz,wght].ttf`              | opsz 14, wght 400 ve 600       |
| `PlusJakartaSans-*.ttf`      | google/fonts, `ofl/plusjakartasans/PlusJakartaSans[wght].ttf` | wght 400 ve 600              |
| `JetBrainsMono-*.ttf`        | google/fonts, `ofl/jetbrainsmono/JetBrainsMono[wght].ttf`   | wght 400 ve 600                |

iced'in metin motoru değişken yazı tiplerinin eksenlerini seçemediği için
değişken dosyalardan fontTools ile (`varLib.instancer`) Regular ve SemiBold
statik örnekleri üretildi; ad tabloları ailenin kendi adıyla (ör. tipografik
aile "Inter", alt aile "SemiBold") yeniden yazıldı. Bu üç ailenin lisansında
ayrılmış yazı tipi adı (Reserved Font Name) yoktur.

IBM Plex'in lisansı "Plex" adını ayrılmış ad olarak tanımlar; değiştirilmiş
bir sürüm bu adı taşıyamaz. Bu yüzden IBM Plex dosyaları IBM'in yayımladığı
statik dosyalardır ve hiç değiştirilmemiştir.
