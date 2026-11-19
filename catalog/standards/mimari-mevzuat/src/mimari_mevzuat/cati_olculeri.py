"""Çatının ölçüleri: Planlı Alanlar İmar Yönetmeliği 40/4, 40/5, 42/1, Binaların Yangından Korunması Hakkında Yönetmelik
58/2 ve Sivas İmar Yönetmeliği 40/5-e, 40/5-g, 42/5 (docs/adr/8044 §9, 8159; Rust'tan taşındı, K10).

- **Mahya (PAİY 40/4; Sivas 40/5-g):** eğimli çatının mahyası saçak seviyesinden (son kat tavan döşemesinin üstü, kot
  farkı dahil) en çok 5,00 m; Sivas seçiliyse 4,00 m, iki katman ``katman.bulgu`` ile.
- **Saçak (PAİY 42/1):** duvarlarından çizilen çatının saçağı en çok 1,00 m (öbür çatının saçağı ölçülmez).
- **Saçağın parsel sınırı (PAİY 42/2; docs/adr/8161):** vaziyet planı varken saçak parsel sınırını aşamaz; satır
  yalnız vaziyet planıyla yazılır.
- **Çıkmadan saçak (PAİY 42/3; docs/adr/8164):** binada çıkma varsa saçak genişliği çıkmadan en çok 0,50 m: çatının
  altındaki çıkmaların ön yüzünden saçak çizgisine (``overProjection``); satır yalnız çatının altında çıkma varken
  yazılır. Planın saçak notu 42/1'in yerine geçer, 42/3'ü bağlamaz.
- **Hava bacası (PAİY 40/5):** çatı örtüsünü en yüksek köşesinde en çok 0,60 m aşar; düzeltme 0,60 m'ye indirir.
- **Duman bacası (BYKHY 58/2):** mahyanın en az 0,80 m üstüne çıkar; düzeltme 0,80 m'ye çıkarır.
- **Sivas seçiliyse** (yönetmeliğin günü 10.11.2022'den): eğim en çok %40 (40/5-g; düzeltme dik kenarları %40'a
  indirir), saçakta kar siperliği zorunlu (42/5; düzeltme Çatı aracının kar siperliğini, saçaktan 30 cm içeride, verir),
  çatı penceresi en çok 1,20 × 1,50 m (40/5-e-2), bina cephe hatlarına en az 1 m, birbirine en az 2 m (40/5-e-3).
- Düz çatının mahya, eğim, kar siperliği ve pencere kuralı yoktur.
- **Planın çatı notları** (docs/adr/8160): ``ridgeMost`` (mahya), ``roofSlopeMost`` (eğim, %; PAİY 40/2 eğimi idareye
  bırakır, Sivas'ın %40'ının yerine geçer, Sivas'sız da okunur) ve ``eavesMost`` (saçak; 42/1: "Uygulama imar planında
  belirlenmemiş ise") yönetmeliklerin değerlerinin önüne geçer (69/3): not bağlar, notun satırı sonucu söyler,
  yönetmeliklerin değerleri farklıysa iletide yanında yazılır; notun bağladığı ölçüde madde kapsam dışıdır.
- **Binanın kendi notları** (docs/adr/8163): her çatı katının binasının notlarıyla (yazmadığı ölçüde projeninkiyle)
  okunur; binanın notunun satırı binanın adıyla ayrıdır. Madde, maddeyle okunan çatı kalmadığında kapsam dışıdır.

Bulgunun düzeltmesi ``cad.arch.edit``'in çatı üstündeki işlemidir; parçalar olgulardaki çizilen değerleriyle yeniden
yazılır, yalnız düzeltilen değişir.

**Okunan olgular** (``facts.building``): çatılar (``form``: eğimli mi, mahya, saçak, eğimler, kar siperliği, ortası,
parçalar, pencere aralıkları, saçağın parselden ve çıkmadan taşması), yerel yönetmelik, plan notları.
"""

from __future__ import annotations

from datetime import date
from decimal import ROUND_HALF_UP, Decimal
from typing import Any

from . import katman
from . import yururluk as y

MADDELER = [("PAİY", "40/4"), ("PAİY", "40/5"), ("PAİY", "42/1"), ("BYKHY", "58/2")]
#: Vaziyet planının parseliyle okunan madde: satırı yalnız vaziyet planıyla.
PARSEL = ("PAİY", "42/2")
#: Çatının altındaki çıkmayla okunan madde (docs/adr/8164): satırı yalnız çatının altında çıkma varken.
CIKMA = ("PAİY", "42/3")
SIVAS = [("Sivas", "40/5-g"), ("Sivas", "40/5-e-2"), ("Sivas", "40/5-e-3"), ("Sivas", "42/5")]

#: Çatı aracının kar siperliği: saçaktan 30 cm içeride (kentos_commands::DEFAULT_SNOW_GUARD); yönetmeliğin değeri değil.
KAR_SIPERLIGI = 0.3

#: Planın çatı notları (olguların ``planNotes[].key``'i), satırdaki adları ve başlıkları.
MAHYA, EGIM, SACAK = "ridgeMost", "roofSlopeMost", "eavesMost"
NOT_SATIRI = {MAHYA: "Plan notu: mahya yüksekliği", EGIM: "Plan notu: çatı eğimi", SACAK: "Plan notu: saçak genişliği"}
NOT_BASLIGI = {MAHYA: "Mahya yüksekliği (en çok)", EGIM: "Çatı eğimi (en çok)", SACAK: "Saçak genişliği (en çok)"}


def _not_satiri(not_: dict[str, Any], durum: str, ne: str) -> dict[str, Any]:
    return {"rule": katman.not_satiri(NOT_SATIRI[not_["key"]], not_), "title": NOT_BASLIGI[not_["key"]],
            "version": None, "gazette": not_["source"], "state": durum, "note": ne}


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _iki(v: float) -> str:
    """İki basamakla, metresiz: “1.40”."""
    return f"{v:.2f}"


def egim(v: float) -> str:
    """Bir eğim, bulgunun yazdığı gibi: “%75”, “%42,5” (onda bire yuvarlanmış)."""
    r = Decimal(str(v)).quantize(Decimal("0.1"), rounding=ROUND_HALF_UP)
    return f"%{int(r)}" if r == r.to_integral_value() else f"%{r}".replace(".", ",")


def _parcalar(form: dict[str, Any]) -> list[dict[str, Any]]:
    """Çatının parçaları ``cad.arch.edit``'in yazdığı gibi: çizilen değerleriyle."""
    out = []
    for p in form.get("parts") or []:
        q: dict[str, Any] = {"kind": p["kind"], "at": {"x": p["at"][0], "y": p["at"][1]}, "width": p["width"],
                             "depth": p["depth"]}
        if p.get("rotation") is not None:
            q["rotation"] = p["rotation"]
        if p.get("rise") is not None:
            q["rise"] = p["rise"]
        out.append(q)
    return out


def _duzelt(etiket: str, cati: str, islem: dict[str, Any]) -> dict[str, Any]:
    return {"label": etiket, "command": "cad.arch.edit", "input": {"uids": [cati], "operation": islem}}


def _yukselt(form: dict[str, Any], i: int, deger: float) -> dict[str, Any]:
    parcalar = _parcalar(form)
    parcalar[i]["rise"] = deger
    return {"kind": "set", "parts": parcalar}


def _bulgu(kod: str, ileti: str, onarim: str, kural: str, cati: str, at: list[float], iz: str,
           fix: dict[str, Any] | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {"code": kod, "message": ileti, "repair": onarim, "uids": [cati], "rule": kural,
                           "at": list(at), "_iz": iz}
    if fix is not None:
        out["fix"] = fix
    return out


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin çatı bulguları ve maddelerin satırları."""
    hal = {m: y.hal(*m, gun) for m in MADDELER}
    sivas = bina.get("local") == "sivas"
    yerel = {m: y.hal(*m, gun) for m in SIVAS} if sivas else {}
    catilar = [r for r in bina.get("roofs") or [] if r.get("form")]
    proje = {n["key"]: n for n in bina.get("planNotes") or [] if n["key"] in (MAHYA, EGIM, SACAK)}
    kat_of = {k["level"]: k for k in bina.get("storeys") or []}

    def notlari(r: dict[str, Any]) -> dict[str, dict[str, Any]]:
        """Çatıyı bağlayan notlar: katının binasının kendi notları, yazmadığı ölçüde projeninkiler (docs/adr/8163)."""
        kat = kat_of.get(r.get("level") or "")
        return {n["key"]: n for n in katman.plan_notlari(bina, [kat] if kat else None)
                if n["key"] in (MAHYA, EGIM, SACAK)}

    # Notların satırları adlarıyla: projeninkiler sırasıyla, binalarınkiler ilk görüldükleri sırayla.
    satir_notu = {NOT_SATIRI[k]: n for k, n in proje.items()}
    cati_notlari: list[dict[str, dict[str, Any]]] = []
    bulgular: list[dict[str, Any]] = []
    kalan: dict[Any, int] = {}
    var: dict[Any, int] = {}

    def say(m: Any, kaldi: bool) -> None:
        var[m] = var.get(m, 0) + 1
        if kaldi:
            kalan[m] = kalan.get(m, 0) + 1

    h_mahya, h_baca, h_sacak, h_duman = (hal[m] for m in MADDELER)
    h_parsel = y.hal(*PARSEL, gun)
    h_cikma = y.hal(*CIKMA, gun)
    s_mahya, s_boyut, s_yer, s_kar = (yerel.get(m) for m in SIVAS)
    for r in catilar:
        f, uid = r["form"], r["uid"]
        orta = f["at"]
        egimli = bool(f.get("sloping"))
        notlar = notlari(r)
        cati_notlari.append(notlar)
        plan = {k: katman.plan_notu(n) for k, n in notlar.items()}
        ad = {k: katman.not_satiri(NOT_SATIRI[k], n) for k, n in notlar.items()}
        for k, n in notlar.items():
            satir_notu.setdefault(ad[k], n)
        # Mahya (40/4; Sivas 40/5-g).
        if egimli and f.get("ridge") is not None and h_mahya is not None:
            v = float(f["ridge"])
            ulusal = katman.Sinir(float(h_mahya.anahtar["most"]), h_mahya)
            yerel_ = katman.Sinir(float(s_mahya.anahtar["most"]), s_mahya) if s_mahya is not None else None
            b = katman.bulgu("paiy_ridge_high", "Mahya yüksekliği (saçak seviyesinden)", v, False, ulusal, yerel_,
                             "Çatının eğimini azaltın ya da çatıyı kot farkı olmadan oturtun.", [uid], f"mahya:{uid}",
                             plan=plan.get(MAHYA))
            if MAHYA in plan:
                say(ad[MAHYA], b is not None)
            else:
                u, s = katman.durumlar(v, False, ulusal, yerel_)
                say(MADDELER[0], u == y.KALDI)
                if s is not None:
                    say(SIVAS[0], s == y.KALDI)
            if b is not None:
                b["at"] = list(orta)
                bulgular.append(b)
        # Saçak (42/1): yalnız duvarlarından çizilen çatının.
        if f.get("overhang") is not None and h_sacak is not None:
            v = float(f["overhang"])
            sinir = katman.Sinir(float(h_sacak.anahtar["most"]), h_sacak)
            onarim = ("Saçağı planın notuna daraltın: çatıyı Çatı aracıyla yeniden çizin." if SACAK in plan
                      else "Saçağı daraltın: çatıyı Çatı aracıyla en çok 1,00 m saçakla yeniden çizin.")
            b = katman.bulgu("paiy_eaves_wide", "Saçak genişliği", v, False, sinir, None, onarim, [uid], f"saçak:{uid}",
                             plan=plan.get(SACAK))
            say(ad[SACAK] if SACAK in plan else MADDELER[2], b is not None)
            if b is not None:
                b["at"] = list(orta)
                bulgular.append(b)
        # Saçağın parsel sınırı (42/2): vaziyet planı varken.
        if f.get("outside") is not None and h_parsel is not None:
            asan = float(f["outside"])
            say(PARSEL, asan > 1e-3)
            if asan > 1e-3:
                bulgular.append(_bulgu(
                    "paiy_eaves_over_parcel", f"Saçak parsel sınırını {katman.metre(asan)} aşıyor — {h_parsel.kisa()}",
                    "Saçağı parsel sınırının içine çekin: çatıyı Çatı aracıyla yeniden çizin.", h_parsel.kural(), uid,
                    orta, f"parsel:{uid}"))
        # Çıkmadan saçak (42/3): çatının altındaki çıkmaların ön yüzünden saçak çizgisine.
        if f.get("overProjection") is not None and h_cikma is not None:
            v = float(f["overProjection"])
            b = katman.bulgu("paiy_eaves_past_projection", "Saçağın çıkmadan genişliği", v, False,
                             katman.Sinir(float(h_cikma.anahtar["most"]), h_cikma), None,
                             "Saçağı çıkmanın ön yüzünden en çok 0,50 m taşacak biçimde daraltın: çatıyı Çatı aracıyla "
                             "yeniden çizin.", [uid], f"çıkma:{uid}")
            say(CIKMA, b is not None)
            if b is not None:
                b["at"] = list(orta)
                bulgular.append(b)
        # Hava bacası (40/5) ve duman bacası (58/2): her parçanın tepesi.
        for i, p in enumerate(f.get("parts") or []):
            if p.get("top") is None:
                continue
            top = float(p["top"])
            if p["kind"] == "vent" and h_baca is not None:
                en_cok = float(h_baca.anahtar["most"])
                b = katman.bulgu("paiy_roof_vent_high", "Hava bacasının çatı örtüsünü aşan yüksekliği", top, False,
                                 katman.Sinir(en_cok, h_baca), None,
                                 "Hava bacasını çatı örtüsünün en çok 0,60 m üstünde bitirin.", [uid],
                                 f"baca:{uid}:{i}")
                say(MADDELER[1], b is not None)
                if b is not None:
                    b["at"] = list(p["at"])
                    b["fix"] = _duzelt("Hava bacasını 0,60 m'ye indir", uid, _yukselt(f, i, en_cok))
                    bulgular.append(b)
            elif p["kind"] == "chimney" and h_duman is not None:
                en_az = float(h_duman.anahtar["least"])
                b = katman.bulgu("bykhy_chimney_low", "Duman bacasının mahyayı aşan yüksekliği", top, True,
                                 katman.Sinir(en_az, h_duman), None,
                                 "Bacayı mahyanın en az 0,80 m üstüne çıkarın.", [uid], f"duman:{uid}:{i}")
                say(MADDELER[3], b is not None)
                if b is not None:
                    b["at"] = list(p["at"])
                    b["fix"] = _duzelt("Bacayı mahyanın 0,80 m üstüne çıkar", uid, _yukselt(f, i, en_az))
                    bulgular.append(b)
        # Planın çatı eğimi notu: Sivas'ın %40'ının yerine, Sivas'sız da.
        if egimli and EGIM in plan:
            sinir = plan[EGIM].deger
            egimler = [s for s in f.get("slopes") or [] if s is not None]
            dik = max(egimler, default=0.0)
            say(ad[EGIM], dik > sinir + 1e-9)
            if dik > sinir + 1e-9:
                yeni = [None if s is None else min(float(s), sinir) for s in f.get("slopes") or []]
                yan = (f"; {s_mahya.kisa()}: {egim(float(s_mahya.anahtar['slope']))}"
                       if s_mahya is not None and abs(float(s_mahya.anahtar["slope"]) - sinir) > 1e-9 else "")
                bulgular.append(_bulgu(
                    "paiy_roof_slope_steep", f"Çatının eğimi {egim(dik)} > {egim(sinir)} — {plan[EGIM].kisa()}{yan}",
                    "Çatının eğimini planın notuna indirin.", plan[EGIM].kural(), uid, orta, f"eğim:{uid}:plan",
                    _duzelt(f"Eğimi planın {egim(sinir)} sınırına indir", uid, {"kind": "set", "slopes": yeni})))
        if not sivas:
            continue
        # Sivas: eğim (planın notu yoksa) ve kar siperliği eğimli çatının, pencereler.
        if egimli and s_mahya is not None and EGIM not in plan:
            sinir = float(s_mahya.anahtar["slope"])
            egimler = [s for s in f.get("slopes") or [] if s is not None]
            dik = max(egimler, default=0.0)
            if dik > sinir + 1e-9:
                yeni = [None if s is None else min(float(s), sinir) for s in f.get("slopes") or []]
                bulgular.append(_bulgu(
                    "local_roof_slope_steep", f"Çatının eğimi {egim(dik)} > {egim(sinir)} — {s_mahya.kisa()}",
                    "Çatının eğimini %40'a indirin.", s_mahya.kural(), uid, orta, f"eğim:{uid}",
                    _duzelt("Eğimi %40'a indir", uid, {"kind": "set", "slopes": yeni})))
                say(SIVAS[0], True)
        if egimli and s_kar is not None:
            yok = not f.get("snowGuard")
            say(SIVAS[3], yok)
            if yok:
                bulgular.append(_bulgu(
                    "local_snow_guard_missing", f"Eğimli çatının saçağında kar siperliği yok — {s_kar.kisa()}",
                    "Çatıya kar siperliği verin (Çatı aracında Kar siperliği, saçaktan 30 cm içeride).", s_kar.kural(),
                    uid, orta, f"kar:{uid}",
                    _duzelt("Kar siperliği ver (saçaktan 30 cm)", uid, {"kind": "set", "snowGuard": KAR_SIPERLIGI})))
        pencereler = [(i, p) for i, p in enumerate(f.get("parts") or []) if p["kind"] == "window"]
        if s_boyut is not None:
            en, boy = float(s_boyut.anahtar["width"]), float(s_boyut.anahtar["length"])
            for i, p in pencereler:
                buyuk = float(p["width"]) > en + 1e-6 or float(p["depth"]) > boy + 1e-6
                say(SIVAS[1], buyuk)
                if buyuk:
                    bulgular.append(_bulgu(
                        "local_roof_window_large",
                        f"Çatı penceresi {_iki(float(p['width']))} × {_iki(float(p['depth']))} m, en çok {_iki(en)} × "
                        f"{_iki(boy)} m (en/boy) — {s_boyut.kisa()}",
                        "Pencereyi küçültün (Çatı penceresi aracında En ve Boy).", s_boyut.kural(), uid, p["at"],
                        f"pencere:{uid}:{i}"))
        if s_yer is not None:
            cephe, ara = float(s_yer.anahtar["facade"]), float(s_yer.anahtar["apart"])
            for i, p in pencereler:
                if p.get("facade") is None:
                    continue
                yakin = float(p["facade"]) < cephe - 1e-6
                say(SIVAS[2], yakin)
                if yakin:
                    bulgular.append(_bulgu(
                        "local_roof_window_close",
                        f"Çatı penceresinin bina cephe hattına uzaklığı {katman.metre(float(p['facade']))} < "
                        f"{katman.metre(cephe)} — {s_yer.kisa()}",
                        "Pencereyi cephe hattından en az 1 m içeri alın.", s_yer.kural(), uid, p["at"],
                        f"cephe:{uid}:{i}"))
            parts = f.get("parts") or []
            for g in f.get("windowGaps") or []:
                yakin = float(g["apart"]) < ara - 1e-6
                say(SIVAS[2], yakin)
                if yakin:
                    bulgular.append(_bulgu(
                        "local_roof_window_close",
                        f"İki çatı penceresi arası {katman.metre(float(g['apart']))} < {katman.metre(ara)} — "
                        f"{s_yer.kisa()}",
                        "Pencereleri çatının üstünde en az 2 m aralıkla yerleştirin.", s_yer.kural(), uid,
                        parts[int(g["a"])]["at"], f"aralık:{uid}:{g['a']}:{g['b']}"))
    satirlar = []
    # Binaların kendi notlarının ölçüleri: projenin notunu okuyan çatı kalmadıysa nedeni.
    kendi = {k for nt in cati_notlari for k, n in nt.items() if n.get("_bina")}
    for ad_, n in satir_notu.items():
        if not var.get(ad_):
            ne = ("çatı yok" if not catilar else "çatıların binalarının kendi notu var"
                  if not n.get("_bina") and n["key"] in kendi else "ölçülecek çatı yok")
            satirlar.append(_not_satiri(n, y.KAPSAM_DISI, ne))
        elif kalan.get(ad_):
            satirlar.append(_not_satiri(n, y.KALDI, f"{kalan[ad_]} ölçü planın notunu aşıyor"))
        else:
            satirlar.append(_not_satiri(n, y.GECTI, f"{var[ad_]} ölçü planın notunun içinde"))

    def bagli(m: Any, anahtarlar: tuple[str, ...]) -> bool:
        """Notun bağladığı ölçünün maddesi: maddeyle okunan çatı yok, not projenin ya da bir çatının."""
        return not var.get(m) and (all(a in proje for a in anahtarlar)
                                   or any(all(a in nt for a in anahtarlar) for nt in cati_notlari))

    # Notun bağladığı ölçünün maddesi kapsam dışı (Sivas 40/5-g mahyanın da eğimin de).
    baglanan = {m for m, anahtarlar in ((MADDELER[0], (MAHYA,)), (MADDELER[2], (SACAK,)), (SIVAS[0], (MAHYA, EGIM)))
                if bagli(m, anahtarlar)}
    yok_notu = {MADDELER[0]: "eğimli çatı yok", MADDELER[1]: "hava bacası yok", MADDELER[2]: "duvarlarından çizilmiş "
                "çatı yok (saçağı ölçülmez)", MADDELER[3]: "duman bacası yok", SIVAS[0]: "eğimli çatı yok",
                SIVAS[1]: "çatı penceresi yok", SIVAS[2]: "çatı penceresi yok", SIVAS[3]: "eğimli çatı yok"}
    if h_parsel is not None and any(r["form"].get("outside") is not None for r in catilar):
        if kalan.get(PARSEL):
            satirlar.append(_satir(h_parsel, y.KALDI, f"{kalan[PARSEL]} çatının saçağı parsel sınırını aşıyor"))
        else:
            satirlar.append(_satir(h_parsel, y.GECTI, f"{var.get(PARSEL, 0)} çatının saçağı parselin içinde"))
    if h_cikma is not None and var.get(CIKMA):
        if kalan.get(CIKMA):
            satirlar.append(_satir(h_cikma, y.KALDI, f"{kalan[CIKMA]} çatının saçağı çıkmadan 0,50 m'den geniş"))
        else:
            satirlar.append(_satir(h_cikma, y.GECTI, f"{var[CIKMA]} çatının saçağı çıkmadan en çok 0,50 m"))
    for m in MADDELER + (SIVAS if sivas else []):
        h = hal.get(m) or yerel.get(m)
        if h is None:
            continue
        if m in baglanan:
            satirlar.append(_satir(h, y.KAPSAM_DISI, "ölçü parselin plan notunda"))
        elif not var.get(m):
            satirlar.append(_satir(h, y.KAPSAM_DISI, yok_notu[m] if catilar else "çatı yok"))
        elif kalan.get(m):
            satirlar.append(_satir(h, y.KALDI, f"{kalan[m]} ölçü sınırı aşıyor"))
        else:
            satirlar.append(_satir(h, y.GECTI, f"{var[m]} ölçü sınırın içinde"))
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin çatı bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    maddeler = MADDELER + [PARSEL, CIKMA] + (SIVAS if bina.get("local") == "sivas" else [])
    return y.denetle(lambda g: kural(bina, g), maddeler, gun, neden, "çatı kuralı")
