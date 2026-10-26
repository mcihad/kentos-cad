# Açıklık merkez tutamacı — kabul kaydı

25 Ekim 2026 · ADR 0663 · Bu dilimin merkez tutamacı kabulü tamamlandı; ana planın açık kapıları aşağıdadır.

## Gerçek pencere

Önceki derleme: ADR 0662'ün kayıtlı SHA'sı. Yeni derleme SHA-256:
`b656b6e488873a668c11014ee31e6ca234546e4e9012b9039c2c0f45fc85b365`.
İki derleme aynı sentetik dosyada, ayrılmış test ayar/veri dizinlerinde çalıştırıldı.

| Görüntü | Gözlenen sonuç |
|---|---|
| `27` | Eski derlemede seçili kapı #8 merkez tutamacı 1,503 m ötede önizlenir. |
| `28` | Eski derlemede sonraki tık taşıma yazmaz; kapının Konum alanı 5,000 m kalır, belge temizdir. |
| `29` | Yeni derlemede önizleme mevcut semantic komuttan gelir; kayıt hâlâ 5,000 m'dir. |
| `30` | Sonraki fare tıkı kapıyı 3,500 m'ye yerleştirir; duvar boşluğu beraber taşınır, en 1,100 m kalır. |
| `31` | Tek Undo konumu 5,000 m'ye ve duvar boşluğunu eski yerine döndürür. |
| `32` | Tek Redo 3,500 m'yi geri getirir; Kaydet sonrasında belge temizdir. |
| `33` | Uygulama kapatılıp aynı dosya yeniden açılır; kapı #8 Konum 3,500 m / Genişlik 1,100 m gösterir. |

Ekranlar gerçek native penceredir. Kamera büyüklüğü iki denemede farklıdır;
sonucun kanıtı yalnız piksel uzaklığı değil, Konum alanı ve bağımsız dosya okumasıdır.
Yapay görüntü veya performans ölçümü değildir.

## Bağımsız kayıt

`before.kcad` ve `after.kcad`, `tools/kcad/kcad.py` ile native modül çağrılmadan
okundu. Dokuz nesne korunur; yalnız #8 açıklığının konumu 5 → 3,5 m değişir.
Host UUID ve 1,1 m en, öteki sekiz nesnenin bütün alanları ve proje ayarları
aynıdır. Yeniden açılan dosyanın nesneleri, UUID sırası ve ayarları son kayıtla
birebir aynıdır. `saved-file-check.json` bu karşılaştırmayı ve son dosya hash'ini tutar.

## Otomatik doğrulama

- Yeni native birleşik kabul: iki yöntem × düz/ters duvar; sayısal en, yan/menteşe,
  kalınlık, bölme, başka host ve undo. Açıklık yerleştirme kümesi 4 test geçti.
  Bölünmüş hosttaki f64 yeniden parametreleme için 10 m işlem girdilerine dayanan
  sınırlı aritmetik hata denetimi var; üretim toleransı değiştirilmedi.
- Tam interaction 63 birim + 652 bütünleşme = **715** test geçti (6 atlanan).
  Tutamaç taşıma, kopya, uçtan taşma ve karışık sıcak seçimde kısmi yazmama
  yeni regresyonları içerir. Journal `cad.openings.edit` yazar.
- Tam desktop **1274** geçti, 220 atlandı. Ek GPU görüntü testi açıkça ayrıca
  çalıştırıldı ve geçti; atlanan test sayısı otomatik başarıya eklenmez.
- Native application/interaction/desktop clippy tüm hedeflerde `-D warnings`
  ile geçti; yeni GUI derlemesi tamamlandı.

Günlük sonuçları ve hash'leri `test-results.json` içindedir. Python/MCP için yeni
uç nokta veya tür eklenmedi; kullanılan mevcut `cad.openings.edit` Rehost/Copy
komutları ADR 0662'ün aynı oturumdaki native/Python/MCP kabulündedir.

## Ek GPU matrisi

`gpu/` altında sekiz durumun 18 görüntüsü vardır: 1440×900 ve 1100×650,
13/18 px, açık/koyu. `wgpu` renderer zorunludur; her görüntü gözle incelendi.
Bunlar offscreen native GPU renderlarıdır, işletim sistemi penceresi ekran
görüntüsü değildir. Ölçüden ve iki pervazdan yerleştirme ayarları incelendi;
dar/18 px durumda alt alanlar kaydırılarak erişilir (`door-lower-*`).
Ayrı Press/Move/Release olaylarıyla Kaydır kamerayı değiştirir, belgeyi
değiştirmez. Bu, gerçek penceredeki sürüklemenin geçtiği anlamına gelmez.
Gerçek pencere kayıtları 1440×828 iç alandır; 1440×900 olarak sunulmaz.

## Sınırlar

Bu değişiklik tek açıklığın merkez Esnet tutamacını bağlar. Farklı nesnelerin
çoklu sıcak seçiminde açıklığı atlayıp kısmi yazmak yerine açık ret vardır;
genel çoklu mimari tutamaç düzenleme tamamlanmış değildir.
ARCH-OPEN-01'in tam native pencere boyutu ve gerçek Kaydır kabulü hâlâ açık.
