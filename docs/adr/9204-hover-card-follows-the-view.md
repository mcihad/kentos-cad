# ADR 9204: Görünüm kımıldayınca gezinme vurgusu ve kartı gider

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-04.
- **İstek:** sahibin sözü (23 Kasım 2026): “objeler ekranda sabit kalıyor yapışık kalıyor”. Bulgu:
  [ADR 9200](9200-render-engine-research.md) §1.2.
- **Sözleşmeler:** yok.
- **Bağımlılık:** [ADR 0068](0068-desktop-grips-and-hover-card.md) (gezinme kartı).

## §1 Sorun

Gezinme kartı, imleç bir nesnenin üstünde yarım saniye durunca açılır (0068). İmleç kıpırdamadan görünüm değişince
(tekerlek, orta tuşla kaydırma, “Tümünü göster”) kart da vurgu da yerinde kalıyordu.

Büyük NCZ'de on altı çentik yakınlaşınca imlecin altında artık boş bir yer vardı. Kart ise hâlâ
“Çoklu çizgi HAT_DOGALGAZ, Uzunluk 100,739 m” diyordu (`.run/gui-evidence/render/arastirma/ncz-gezinme-karti.png`). Kart
ve vurgu, ekranda sabit kalıp artık orada olmayan bir nesneyi gösteriyordu.

Sebep: vurgulanan nesne yalnız imleç hareket edince yeniden bulunuyordu (seçim aracının `pointer_move`'u).

## §2 Araştırma

CAD programlarında üstünde gezinme vurgusu (rollover) yakınlaştırma ve kaydırmada kalkar, imleç kıpırdayınca imlecin
altındaki nesneyle yeniden gelir. Harita motorlarında da açılır bilgi kartı (popup) bir nesneye bağlıysa onunla
birlikte kayar, bağlı değilse kapanır.

## §3 Karar

Görünüm imlecin altında kımıldayınca vurgu kalkar ve kart onunla gider:

- Hangi olaylar: `Zoomed`, `Panned`, `Extents`.
- Bir komut çalışmıyorken; komut çalışırken kart zaten gösterilmez.
- İmleç kıpırdayınca vurgu ve kart her zamanki kuralla (yarım saniye bekleme) geri gelir.

Vurgunun görünüm değişince imlecin altında yeniden aranması da düşünüldü. Ama kaydırmanın her adımında bir isabet
sorgusu demekti. Ayrıca tekerlek dönerken kartın sürekli değişmesi göz yorardı. Tuval motoru (RM-06) vurguyu kimlik
tamponundan okuyacak; o zaman ucuzdur ve yeniden düşünülür.

## §4 Kanıt

**Sınama:** `hover_card::tests::the_card_goes_when_the_view_moves_under_a_still_pointer`. Tekerlek, kaydırma ve “Tümünü
göster” için ayrı ayrı denetlenir: kart açıkken görünüm değişince vurgu ve kart gider.

**Gerçek pencere:** `.run/gui-evidence/render/rm04/{once,sonra}`.

## Uygulama

- `apps/desktop/src/shell/input.rs`: `App::pointer`.
- Sınama: `apps/desktop/src/cad/hover_card.rs`.
