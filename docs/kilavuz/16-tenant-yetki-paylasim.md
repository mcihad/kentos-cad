# KentOS CAD kılavuzu — §16 Tenant, proje yetkisi ve paylaşım

> [CLAUDE.md](../../CLAUDE.md)'nin 16. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §16…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 16. Tenant, proje yetkisi ve paylaşım

Mevcut tenant/rol altyapısını koruyup proje bazlı yetkiyle genişletin.
Proje erişimi proje düzeyindedir (ADR 0015): rolü `kentos.project_role`, izinleri
`application/src/access.rs` verir. Projeye dokunan her kullanım durumu, HTTP yolu, ürün komutu
ve WS aboneliği `access::project` ile başlar; yazanlar proje kilidi altında yeniden sorar.
Erişilemeyen proje var olmayan gibi 404'tür. Yaşam döngüsü komutları katalogdadır (ADR 0028):
arşiv `project.edit`, çöp kutusu/geri yükleme/kalıcı silme `project.delete` ister; kalıcı
silme yalnız çöpten ve projenin adıyla onaylıdır, denetim kaydı kalır; çöpteki proje
`KENTOS_TRASH_RETENTION_DAYS` (varsayılan 30) gün sonra kalıcı silinir. Projeye bağlı satırlar yalnız `app.project_id`
kapsamında görünür.
Kimlik/izin server'da doğrulanır; tenant üyeliği her projeyi görme hakkı değildir.
Paylaşılacak kişi yalnız arayanın görebildiği kişiler arasında aranır (projenin kurumu;
kişisel projede arayanın kurumları); hesabın varlığını sızdıran genel e-posta/giriş adı
araması yoktur (ADR 0024).
API, WS, sorgu, dosya/asset, job, Python ve AI aynı erişim modelini kullanır.
Künyenin kişisel bilgileri (yapı sahibi, müellif, sicil, büro, tapu kaydı, teslim eden) yalnız `feature.write` sahibine
gider; öbürleri okumada ve anlık görüntüde kesilmiş alır, künye taşıyan saklı kopyayı alamaz (ADR 8100,
`ProjectAccess::sees_particulars`). Projeye yeni bir okuma yolu eklerken bu kesme de eklenir.
Kurum dışına yol bağlantıyla davettir (ADR 0035): yalnız davetteki e-postanın doğrulanmış hesabı kabul eder;
kurum dışından biri yalnız o projeyi gören misafir olur, kurum misafiri kapatabilir (`tenant.allow_guests`).
Kişisel ve kurumsal sahiplik, davet, paylaşım ve izin iptali TODOS.md §12'dedir.
RLS/pool context ve runtime/admin rolleri test edilir. İndirilmiş kopyanın
geri alınabileceğini veya görüntüleme izninin DRM sağladığını iddia etmeyin.

**Bulut portalı (bulut-portal dalı, [ADR 9000](../adr/9000-cloud-portal.md)):** platform yöneticisi
(`kentosd platform-admin`), belediyeler ve meslek mensupları (mimar, harita ve inşaat mühendisi…) aynı sunucuda
çalışır. Belediye ile meslek mensubu arasındaki her iş bir dosyadır: meslek mensubunun başvurusu ya da belediyenin proje
aktarımı. Meslek mensubunun işlemleri hiçbir zaman kurum kapsamı taşımaz; yalnız aralarındaki dosyayı görür. Yönetim
işlemleri, çağıranı denetleyen `security definer` işlevlerinden geçer. Plan: [plan/bulut.md](../../plan/bulut.md).
