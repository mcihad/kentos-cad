"""Bir ölçünün ulusal ve yerel yönetmelikteki iki sınırı (docs/adr/8034, 8083; Planlı Alanlar İmar Yönetmeliği 69/3).

Belediyenin imar yönetmeliği (Sivas) PAİY'nin üstüne okunur: plan aksini demiyorsa yerel yönetmelik uygulanır.

- **İkisine de uyan** ölçü tuttu.
- **Yalnız yerele uymayan** ölçüde yerel sınır bağlar; PAİY'ninki iletide yanında yazılır.
- **İkisine de uymayan** ölçüde sıkı olan bağlar, öbürü yanında; ikisi eşitse yalnız PAİY'ninki.
- **Yerele uyup PAİY'ye uymayan** ölçü çelişkidir (``regulation_conflict``): iki değer kaynaklarıyla yazılır, idare karar
  verir.
- **Parselin plan notu** (docs/adr/8084) ikisinin de önüne geçer: yerel yönetmelik plan aksini demiyorsa uygulanır
  (69/3), 28 ile 39 arasındaki maddeler planla değiştirilebilir (69/2 saymaz). Ölçü plan notunun değerini tutmuyorsa
  kaldı; yönetmeliklerin değerleri farklıysa iletide yanında yazılır.
- **Binanın plan notu** (docs/adr/8163; kademe hattı, PAİY 4/1-mm): bina programının binası kendi notlarını söylüyorsa
  (olguların katlarında ``planNotes``) binanın yazdığı ölçü projenin aynı ölçüdeki notunun yerine geçer; satırı ve
  bulgunun kaynağı binanın adıyladır (“Plan notu: bina yüksekliği (A Blok)”, “A Blok plan notu (…)”).

İletiler Mimari denetimin Rust kurallarınınkiyle aynı biçimdedir (“Ortak merdivenin kolu 1.25 m < 1.30 m — Sivas 31/1-a;
PAİY 31/1-a: 1.20 m”).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from . import yururluk as y

#: Ölçü ile sınırın karşılaştırılmasında kayan noktanın payı (Rust kurallarınınki gibi; olgular 1 mm'ye yuvarlanmış).
PAY = 1e-6

TUTTU, KALDI, CELISKI = "tuttu", "kaldı", "çelişki"


@dataclass(frozen=True)
class Sinir:
    """Bir hâlin bir ölçüye koyduğu sınır: değeri, hâli ve fıkranın bendi (“39/1-c”'nin ``c``'si)."""

    deger: float
    hal: y.Hal
    bent: str | None = None

    def kisa(self) -> str:
        return self.hal.kisa(self.bent)

    def kural(self) -> str:
        return self.hal.kural(self.bent)


@dataclass(frozen=True)
class PlanNotu:
    """Parselin plan notunun bir ölçüye koyduğu değer ve yazıldığı yer, mimarın yazdığı gibi (“UİP 12.3.2019, plan notu
    7”; docs/adr/8084); binanın kendi notuysa binanın adı (docs/adr/8163)."""

    deger: float
    kaynak: str
    bina: str | None = None

    def kisa(self) -> str:
        return f"{self.bina} plan notu ({self.kaynak})" if self.bina else f"Plan notu ({self.kaynak})"

    def kural(self) -> str:
        return f"{self.bina} plan notu · {self.kaynak}" if self.bina else f"Plan notu · {self.kaynak}"


def bina_adi(katlar: list[dict[str, Any]]) -> str | None:
    """Katların binasının iletideki adı (docs/adr/8163): bina programındaki adı (``buildingName``), yoksa kimliğiyle
    (“a” binası); binasız katlarda yok."""
    kat = next((k for k in katlar if k.get("building")), None)
    if kat is None:
        return None
    return kat.get("buildingName") or f"“{kat['building']}” binası"


def kendi_notu_var(bina: dict[str, Any]) -> bool:
    """Bir bina kendi plan notlarını söylüyor mu (docs/adr/8163; katların ``planNotes``'u)."""
    return any(k.get("planNotes") for k in bina.get("storeys") or [])


def plan_notlari(bina: dict[str, Any], katlar: list[dict[str, Any]] | None = None) -> list[dict[str, Any]]:
    """Okunan plan notları (docs/adr/8163): ``katlar``ın binasının kendi notları (``_bina``: binanın adı), binanın
    yazmadığı ölçülerde projeninkiler; ``katlar`` yoksa ya da binaları not söylemiyorsa projeninkiler."""
    proje = list(bina.get("planNotes") or [])
    kendi = next((k["planNotes"] for k in katlar or [] if k.get("planNotes")), None)
    if not kendi:
        return proje
    ad = bina_adi(katlar or []) or "Binanın"
    yazilan = {n["key"] for n in kendi}
    return [dict(n, _bina=ad) for n in kendi] + [n for n in proje if n["key"] not in yazilan]


def plan_notu(n: dict[str, Any]) -> PlanNotu:
    """Bir notun (``plan_notlari``'nın) değeri, yeri ve binası."""
    return PlanNotu(float(n["value"]), n["source"], n.get("_bina"))


def not_satiri(ad: str, n: dict[str, Any]) -> str:
    """Notun satırının adı: projeninki ``ad``, binanınki ardında binanın adıyla (“Plan notu: bina yüksekliği (A Blok)”;
    docs/adr/8163)."""
    return f"{ad} ({n['_bina']})" if n.get("_bina") else ad


def binalara(bina: dict[str, Any]) -> list[list[dict[str, Any]]]:
    """Katlar binalarına göre, binaların ilk görüldüğü sırayla; binasız katlar bir arada."""
    gruplar: dict[Any, list[dict[str, Any]]] = {}
    for k in bina.get("storeys") or []:
        gruplar.setdefault(k.get("building"), []).append(k)
    return list(gruplar.values())


def kot(kat: dict) -> float:
    """Katın kotu binasının ±0,00'ından (docs/adr/8154): olguların katının kotu eksi binasının kendi ±0,00'ı (``zero``,
    bina programından, docs/adr/8072); binası sıfırını söylemiyorsa projeninki."""
    return float(kat.get("elevation") or 0.0) - float(kat.get("zero") or 0.0)


def metre(v: float) -> str:
    """İki basamakla metre: “1.20 m”."""
    return f"{v:.2f} m"


def tutar(v: float, en_az: bool, s: Sinir) -> bool:
    """``v`` sınırı tutuyor mu (``en_az``: en az; değilse en çok)."""
    return v + PAY >= s.deger if en_az else v <= s.deger + PAY


def karar(v: float, en_az: bool, ulusal: Sinir, yerel: Sinir | None) -> tuple[str, Sinir | None, Sinir | None]:
    """Ölçünün iki katmandaki sonucu: (``TUTTU``, yok, yok), (``KALDI``, bağlayan, öbürü ya da yok), (``CELISKI``, yerel,
    ulusal)."""
    u = tutar(v, en_az, ulusal)
    if yerel is None:
        return (TUTTU, None, None) if u else (KALDI, ulusal, None)
    r = tutar(v, en_az, yerel)
    if u and r:
        return TUTTU, None, None
    if u:
        return KALDI, yerel, ulusal
    if r:
        return CELISKI, yerel, ulusal
    if abs(yerel.deger - ulusal.deger) < 1e-9:
        return KALDI, ulusal, None
    sik = yerel.deger > ulusal.deger if en_az else yerel.deger < ulusal.deger
    return (KALDI, yerel, ulusal) if sik else (KALDI, ulusal, yerel)


def bulgu(kod: str, ozne: str, v: float, en_az: bool, ulusal: Sinir | None, yerel: Sinir | None, onarim: str,
          uids: list[str], iz: str, nitelik: str = "", plan: PlanNotu | None = None) -> dict[str, Any] | None:
    """Ölçünün bulgusu; tuttuysa yok. ``ozne`` ölçünün adı (“Kapının net genişliği”), ``nitelik`` ölçünün ve sınırın
    ardından ayraç içinde (“ (konut bölgesi, 2 kat)”), ``iz`` gün denenirken parçanın kimliği. ``plan`` parselin plan
    notu: varsa o bağlar, yönetmeliklerin değeri (biri yoksa ``ulusal`` da yok olabilir) farklıysa yanında yazılır."""
    if plan is not None:
        if tutar(v, en_az, plan):
            return None
        isaret = "<" if en_az else ">"
        yan = [s for s in (ulusal, yerel) if s is not None and abs(s.deger - plan.deger) > 1e-9]
        if len(yan) == 2 and abs(yan[0].deger - yan[1].deger) < 1e-9:
            yan = yan[:1]
        obur = "".join(f"; {s.kisa()}: {metre(s.deger)}" for s in yan)
        return {
            "code": kod,
            "message": f"{ozne} {metre(v)} {isaret} {metre(plan.deger)} — {plan.kisa()}{obur}",
            "repair": onarim,
            "uids": uids,
            "rule": plan.kural(),
            "_iz": f"{iz}:plan:{plan.deger}",
        }
    if ulusal is None:
        return None
    sonuc, a, b = karar(v, en_az, ulusal, yerel)
    if sonuc == TUTTU or a is None:
        return None
    if sonuc == CELISKI and b is not None:
        soz = "en az" if en_az else "en çok"
        return {
            "code": "regulation_conflict",
            "message": f"{ozne} {metre(v)}{nitelik}: {a.kisa()} {soz} {metre(a.deger)} ister, uyuyor; {b.kisa()} {soz} "
                       f"{metre(b.deger)} ister, uymuyor — çelişkide idare karar verir",
            "repair": f"İdareye sorun (PAİY 69/3: plan aksini demiyorsa yerel yönetmelik uygulanır); {b.kisa()} "
                      f"değerine ({metre(b.deger)}) uymak çelişkiyi ortadan kaldırır.",
            "uids": uids,
            "rule": f"{a.kural()} ↔ {b.kural()}",
            "_iz": f"{iz}:çelişki:{a.deger}:{b.deger}",
        }
    isaret = "<" if en_az else ">"
    obur = f"; {b.kisa()}: {metre(b.deger)}" if b is not None else ""
    return {
        "code": kod,
        "message": f"{ozne} {metre(v)} {isaret} {metre(a.deger)}{nitelik} — {a.kisa()}{obur}",
        "repair": onarim,
        "uids": uids,
        "rule": a.kural(),
        "_iz": f"{iz}:kaldı:{a.deger}",
    }


def durumlar(v: float, en_az: bool, ulusal: Sinir, yerel: Sinir | None) -> tuple[str, str | None]:
    """Maddelerin satırları için ölçünün her katmandaki sonucu: (ulusal, yerel ya da yok), ``y.GECTI`` ya da
    ``y.KALDI``."""
    u = y.GECTI if tutar(v, en_az, ulusal) else y.KALDI
    r = None if yerel is None else (y.GECTI if tutar(v, en_az, yerel) else y.KALDI)
    return u, r
