# ADR 0044: Olayları bekleyerek sorma (uzun sorgu)

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §21.1; ADR 0040 (masaüstü bulut istemcisi), 0043 (masaüstünde çevrimdışı çalışma)
- **Sahibin yönü:** proje kusursuz ve güçlü biçimde, kesintisiz eşitlenecek.

## Bağlam

- Web açık projenin olaylarını WebSocket'le alır. Masaüstü bugüne kadar `GET …/events`'i birkaç saniyede bir soruyordu. Başkasının değişikliği gecikmeli geliyor, boş sorular da trafik yapıyordu.
- Masaüstüne WebSocket de getirilebilirdi. Ama wss için TLS bağlayıcısını (rustls, sertifika doğrulayıcı, şifreleme sağlayıcısı) elle kurmak gerekirdi; HTTP isteklerinin kullandığı güvenilir yığının ikinci bir kopyası olurdu.

## Karar

- `GET …/events?after=&limit=&wait=<saniye>`:
  - `wait` verildiyse ve yeni olay yoksa istek, projenin bir sonraki commit'ini ya da sürenin dolmasını bekler;
  - commit gelir gelmez yanıt verir; süre dolarsa boş sayfa döner;
  - bekleme en çok 25 saniyedir, çünkü isteğin zaman aşımı 30 saniyedir.
- **Sunucu:**
  - İstek ilk okumadan önce süreçteki commit sinyalini (`Hub`) dinlemeye başlar. Okumayla bekleme arasında gelen commit kaçmaz.
  - Başka bir sürecin commit'i (başka sunucu, komut satırı) bu sürece sinyal vermez; bu yüzden 5 saniyede bir yeniden bakılır. WebSocket de aynı kuralla çalışır.
  - Kaçırılan sinyalde (`Lagged`) hemen yeniden okunur.
  - Erişim her okumada satır düzeyi güvenlikle yeniden denetlenir (ADR 0015). Saklanmayan ya da en yeniden ileride kalan imleç `resync_required` alır (ADR 0043).
- **Masaüstü** (`follow::wait`): yanıt gelir gelmez yeniden sorar. Başkasının değişikliği bir anda gelir. Bütün istekler aynı HTTP ve TLS yığınından geçer, vekil sunuculardan da.
- Web WebSocket'te kalır. Bu yol web'in davranışını değiştirmez.

## Doğrulama (26 Eylül 2026, Linux)

- Gerçek sunucu, `a_waiting_request_answers_as_soon_as_another_editor_commits`:
  - yeni bir şey yokken 1 saniyelik bekleme boş ve süresinin sonunda döner;
  - 25 saniyelik bekleme, yarım saniye sonra yapılan commit'le 5 saniyeden kısa sürede tek olayla döner (ölçülen yaklaşık 0,5 s);
  - yeni olay zaten varken hemen döner.
- Kasıtlı bozma: sunucu commit sinyalini dinlemeyince bekleyen istek ancak 5 saniyelik yeniden bakışta döndü (5,03 s) ve test düştü; geri alındı.
- Bütün masaüstü istemcisi testleri (12) gerçek sunucuyla geçti.
