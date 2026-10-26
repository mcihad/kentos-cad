# Mahal proje listeleri ve açık kat öneki — 26 Ekim 2026

[ADR0661](../../adr/0661-project-room-choices.md); ARCH-SPACE-02'nin liste/önek
dilimi. Bu liste/önek diliminin kabulü tamamlandı; genel mimari matris kapanmış değildir.

## Davranış ve uyumluluk

Proje ayarları → Mimari içindeki ad/kullanım listeleri sıralanabilir;
önekler kalıcı kat kimliğine bağlıdır. Mahal aracı ve kaynak özellikleri
listeleri mevcut serbest metnin yanında sunar. Kat değiştirmek taslak
numarayı değiştirmez; başlangıç açık seçimle alınır. Liste kaydı kaynak
mahalı yeniden adlandırmaz/numaralandırmaz. Mahal yazımı/bağlı etiket tek
undo'dur; ayarlar mevcut kuralla ayrı undo üretmez ve geometri undo'sunda kalır.

İki liste en çok 256 farklı tam değer, değerler 256 Unicode karakter,
önekler 253 karakterdir. Büyük/küçük harf korunur. API baş/son boşluğu
reddeder; form kaydederken kırpar. Bilinmeyen kat, kontrol karakteri,
tekrar ve sınır aşımı atomik rettir. arch bütün tercihin değiştirilmesidir,
iç içe yama değildir: Python/MCP eski kat/ayrıntı/yöntem/evreyi kopyalamalıdır.

KCAD 107 yalnız dolu yeni tercihte seçilir; boş varsayılan ve eski dosyalar
aynı bayt/şemada kalır. Eski okuyucu sessiz veri kaybı yerine sürümü reddeder.
1170 eski takipli KCAD baytı aynıdır; desteklenmeyen sürüm örneği 107→108,
üç geçerli/30 bozuk yeni örnek vardır. FORMATS_VERSION 96 değişmedi.

DXF proje ArchSettings'ini taşımaz. Gerçek round-trip bu sınırı ortaya
çıkardı; dolu yeni tercihte yazmadan önce görünür uyarı ve ortak sonuç
raporu KCAD olarak saklamayı söyler. Tercih eklemek DXF'in geometri
baytlarını değiştirmez. DXF import'un mevcut UUID yeniden üretmesi aynı;
KCAD kaynak kimliklerini ve tercihleri korur.

Yeni TS/TSX dosyası yoktur; yalnız mevcut ArchSettings.ts, SettingsSet.ts,
katalog ve tipli Python SDK yenilendi. Toplu TS çıktısı ignore edilen
.run/architecture-contours/contracts-ts içindedir. Web UI ADR0228 gereği
TODOS.md§0.2'de bekler.

## Otomatik kanıt

test-runs.json gerçek komut/çıkış/zaman ve ham günlük hash'lerini taşır.
Başarısız denemeler saklanır ve olumlu kabul sayılmaz:

- python-red eski SDK'nın alanları tanımadığını gösterir.
- kcad-native 206 testten sonra sabit geçerli örnek sayacında düştü; üç yeni
  dosyayla 119→122 düzeltildi. Son tam codec takımı 207/207 geçti.
- desktop-check ikinci mutable ödünçten derlenmedi; aynı d.arch taslağı
  kullanılarak düzeltildi.
- Üç audit betik denemesi gömülü proje şemaları/JSON düğümü/summary'yi eksik
  ele aldı; son bağımsız audit doğru kapsamı doğrular. Ürün hatası değildir.
- python-focused ilk DXF beklentisinde tercihleri kayıpsız sandı; gerçek
  çıktı varsayımı reddetti. Test mevcut format sınırını ve açık uyarıyı
  doğrular. Uyarıdan önceki red ve sonraki altı başarılı test saklanır.

| Son kapı | Gerçek sonuç |
| --- | --- |
| İlgili ortak durumlar | 385: 91 settings/28 space-create/36 edit/68 number/107 tag-create/26 edit/29 check; eski 62 settings aynıdır, 29 yeni |
| Native ana takımlar | 68 domain/699 interaction/219 application; diğer ikililer günlükte, mevcut 1/22 atlama başarı sayılmaz |
| Contracts/commands/KCAD | 1001/809/207; ek 32 document/10 settings/6 number; TS export testleri dahildir |
| Gerçek MCP Server | Dört test: yeni tercihler/ret/serbest edit/tek undo/gerçek KCAD kayıt-aç ve önceki numara/net etiket akışları |
| Clippy | On ilgili paket, bütün hedefler -D warnings geçti; mevcut ts-rs/block bildirimleri ham günlükte |
| Katalog denetimi | 268 kimlik aynı; 265 komut aynı, yalnız project.changes/project.create/settings gömülü ArchSettings ilavesi |
| Tam Python | 329 test geçti; mevcut sqlite test yardımcılarının ResourceWarning çıktıları saklandı |
| Diğer kapılar | WASM/no-default-features, SDK sapması, bağımlılık yönü 47 crate/61 hedef ve 1203 KCAD referansı geçti |
| Desktop taslak testleri | İki test: sıra/ret/stale indeks, kat kimliği ve açık önek |

## Gerçek arayüz ve bağımsız dosyalar

before.kcad önceki ADR0660 numaralı GUI kaydının bayt özdeş kopyasıdır:
beş 4×3 m mahal/beş etiket, iki kat, atanmamış ve gizli kaynak. Kullanıcı
çizimleri/asıl profil kullanılmaz. Ham JPEG'ler kırpılmadı, ölçeklenmedi,
yeniden boyanmadı. gui-events.json zaman/hash/değişmez QA ikilisini eşler;
binary manifestleri debug ikililerini ve ilgili kaynak hash'lerini tutar.
Son after-validation-binary.json yerel hata düzeltmesinin ayrı ikilisidir;
önceki ham görüntülerin değişmez paketleri korunmuştur. Toplam 42 ham ekran
ve 23 tam native kayıt doğrulandı.

- 01 eski sayfada yeni kontrol yoktur. 02–06 yeni sayfada ekle/değiştir/
  sırala/sil, yinelenende Kaydet reddi, geçerli Kaydet ve taslaktan Vazgeç.
- 07–11 listede bulunmayan Ana giriş durur; açık seçimle Mutfak ve Ofis,
  her biri ayrı undo/redo; bağlı etiket yalnız ad metnini izler.
- 12–16 kat değiştirilince K-009 taslağı durur; açık G-001, serbest Özel oda/
  Ofis, dört kesin köşe 0,5→4,5→4,8→0,8. Son onaydan önce Save dosyası
  aynı; tek Enter 12 m² yeni mahal oluşturur.
- 17 tanısal başarısız denemedir: aktif araç/komut girişi sırasında Cmd+Z
  belgeyi geri almadı. Başarılı undo kanıtı değildir. 18–19 araçtan çıktıktan
  sonraki tek belge Undo/Redo doğru bütün dosyaları verir.
- 20 açık 1. kat öneği 01-001; 21 seçim dışı G-001 çakışmasında tek Enter atomik
  ret. 22 iki oda G-001/G-002 önizleme, 23 tek onay, 24–25 tek undo/redo.
- 26–27 yeni süreçte KCAD açılınca iki önek/listeler korunur. 28–29 koyu 13
  dar, 30–34 açık 18 dar; kaydırma/dropdown, hatalı Kaydet ve iki köşeli
  taslağın Escape ile bırakılması. 35–37 açık 18 geniş ve aynı 11 nesneli sahne.
- 32 açıklamanın kullanım listesinin altında kaldığını yakaladı; Kaydet
  reddeder fakat hata o konumda görünmez. Hata ilgili listenin yanına
  taşındı ve yinelenen alan işaretlendi; 40–42 aynı açık 18 dar pencerede
  iki yerel hatayı, düzeltince temizlenmeyi ve Kaydet etkinleşmesini doğruladı; son Vazgeç/Save dosyası aynıdır.
- 38 gerçek DXF uyarısı yazmadan önce; 39 gerçek dosya yazıldıktan sonraki
  geçmişte aynı uyarı, 11 nesne/3 katman ve mahal/etiket aktarım açıklaması.

verify.py, 23 tam gerçek native kaydını bağımsız KCAD okuyucusuyla bütün
beklenen modele karşılaştırır; tüm UUID/geometri/kat/katman/ayar/etiket
yeri ve eşdeğer durumların dosya baytları aynıdır. seed.json yalnız rastgele
yeni UUID'yi sabitler; geometri ve alanlar açık beklentilerdir.
verify_dxf.py gerçek GUI çıktısını harici ezdxf 1.4.5 ile açar: 6 doğru kapalı
sınır, 5 etiket yazısı, 11 tam kaynak metadata/UUID ve 0 audit hatası; gizli
katman kapalıdır. DXF import'un aynı UUID'yi kullandığı iddiası değildir.

| Görünüş | Geniş ham Retina ekran | Dar ham Retina ekran |
| --- | --- | --- |
| Koyu 13 | 2880×1720 | 2200×1364 |
| Açık 18 | 2880×1720 | 2200×1364 |

Gözlenen 1440×860/1100×682 mantıksal çerçeve, istenen 1440×900/1100×650 ile
eşit sayılmaz. Dört temsilî görünüm bütün mimari araçların sekizli matrisini
kapatmaz. Eski mahal yazısı/ayrı etiket örtüşmesi başlangıçta da vardır;
bu dilimde renderer değişmedi.

## Yoğun belge ve kalan kapsam

measure.py, 10.000 mahal+20.000 çizgide iki 256×256 karakterlik sözlük ve iki
önekle herkese açık Python plan/yazımını ölçer. Her altı çağrıda bağımsız
tam model; tercih temizlenince eski KCAD baytları birebir dönmelidir.
GUI/Cargo kapalıyken alınan timing.json yerel debug CPU kontrolüdür;
release/p95/bellek/GPU/kare süresi kabulü değildir. Apple M5/32 GiB üzerinde
üç plan ortancası 12,730 ms, üç yazım ortancası 15,183 ms; kurulum/kayıt ve
bağımsız karşılaştırma süreleri ölçüme dahil değildir.

Eski etiketi açık Koru/Yenile, çoklu pafta ve bütün mimari iş akışlarının
tam matrisi açık kalır. ARCH-SPACE-02'nin tamamı bitmiş sayılmaz.
