#!/usr/bin/env python3
"""STAC varlıklarına erişimin ortak durumları (docs/adr/0644): s3 adresi, AWS imzası (SigV4) ve Azure SAS belirteci.

    python3 scripts/fixtures/stac_access_cases.py           # fixtures/services/v1/stac-access.json'u yazar
    python3 scripts/fixtures/stac_access_cases.py --check   # hiçbir şey yazmaz; karşılaştırır
    uv run --no-project --with botocore python3 scripts/fixtures/stac_access_cases.py --check --botocore

KentOS kodu kullanılmaz. İmza AWS'nin “Signature Version 4” belgesinden Python'un hashlib ve hmac'ıyla yazılır; belgenin
kendi örneği (GET Object, Range bytes=0-9, imza f0e8bdb8…) birebir tutar. --botocore ile her durum botocore'un kendi
imzacısıyla da karşılaştırılır. s3 adresi sanal sunuculu HTTPS adresine çevrilir (adında nokta olan kova yol biçimine);
SAS belirteci adresin sorgusuna eklenir, adresin kendi sorgusu korunur.
"""

import datetime
import hashlib
import hmac
import json
import sys
from pathlib import Path
from urllib.parse import quote, unquote, urlsplit

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/services/v1/stac-access.json"

EMPTY = hashlib.sha256(b"").hexdigest()
UNRESERVED = "-_.~"


def enc(s, slash=False):
    """AWS's URI encoding: unreserved kept, every other byte %XX upper case; '/' kept in a path."""
    return quote(s, safe=UNRESERVED + ("/" if slash else ""))


def s3_url(href, region):
    """`s3://bucket/key` as the HTTPS address a range reader asks: virtual-hosted, else path style for a dotted bucket."""
    rest = href[len("s3://"):]
    bucket, _, key = rest.partition("/")
    region = region or "us-east-1"
    path = enc(key, slash=True)
    if "." in bucket:
        return f"https://s3.{region}.amazonaws.com/{bucket}/{path}"
    return f"https://{bucket}.s3.{region}.amazonaws.com/{path}"


def canonical_query(query):
    if not query:
        return ""
    pairs = []
    for part in query.split("&"):
        k, _, v = part.partition("=")
        pairs.append((enc(unquote(k)), enc(unquote(v))))
    pairs.sort()
    return "&".join(f"{k}={v}" for k, v in pairs)


def sign(c):
    """SigV4 of a GET (or HEAD) to S3: the canonical request, the string to sign, the signature, the Authorization header."""
    u = urlsplit(c["url"])
    headers = {k.lower(): " ".join(str(v).split()) for k, v in c["headers"].items()}
    headers["host"] = u.netloc
    headers["x-amz-content-sha256"] = EMPTY
    headers["x-amz-date"] = c["time"]
    if c.get("token"):
        headers["x-amz-security-token"] = c["token"]
    names = sorted(headers)
    canonical_headers = "".join(f"{n}:{headers[n]}\n" for n in names)
    signed = ";".join(names)
    canonical = "\n".join([c["method"], u.path or "/", canonical_query(u.query), canonical_headers, signed, EMPTY])
    day = c["time"][:8]
    scope = f"{day}/{c['region']}/s3/aws4_request"
    to_sign = "\n".join(["AWS4-HMAC-SHA256", c["time"], scope, hashlib.sha256(canonical.encode()).hexdigest()])

    def h(key, msg):
        return hmac.new(key, msg.encode(), hashlib.sha256).digest()

    k = h(("AWS4" + c["secret"]).encode(), day)
    k = h(k, c["region"])
    k = h(k, "s3")
    k = h(k, "aws4_request")
    signature = hmac.new(k, to_sign.encode(), hashlib.sha256).hexdigest()
    auth = f"AWS4-HMAC-SHA256 Credential={c['access']}/{scope}, SignedHeaders={signed}, Signature={signature}"
    return {"canonicalRequest": canonical, "stringToSign": to_sign, "signature": signature, "authorization": auth,
            "signedHeaders": signed}


def with_sas(url, token):
    """A blob address with a SAS token: the token after the address's own query."""
    token = token.lstrip("?")
    return f"{url}&{token}" if "?" in url else f"{url}?{token}"


def blob_parts(url):
    """The storage account and container of an Azure blob address; none for another address."""
    u = urlsplit(url)
    host = u.hostname or ""
    if u.scheme != "https" or not host.endswith(".blob.core.windows.net"):
        return None
    account = host[: -len(".blob.core.windows.net")]
    segments = [s for s in u.path.split("/") if s]
    if not account or len(segments) < 2:
        return None
    return {"account": account, "container": segments[0]}


AWS_KEY, AWS_SECRET = "AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"


def sigv4_cases():
    base = {"method": "GET", "access": AWS_KEY, "secret": AWS_SECRET, "time": "20130524T000000Z"}
    out = [
        {**base, "name": "AWS belgesinin örneği: GET Object, Range 0-9", "url": "https://examplebucket.s3.amazonaws.com/test.txt",
         "region": "us-east-1", "headers": {"Range": "bytes=0-9"},
         "expectSignature": "f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"},
        {**base, "name": "istek ödemeli kova: x-amz-request-payer", "url": s3_url(
            "s3://usgs-landsat/collection02/level-2/standard/oli-tirs/2024/175/032/LC08_L2SP_175032_20240725_20240801_02_T1/"
            "LC08_L2SP_175032_20240725_20240801_02_T1_SR_B4.TIF", "us-west-2"),
         "region": "us-west-2", "headers": {"Range": "bytes=0-16383", "x-amz-request-payer": "requester"}},
        {**base, "name": "geçici kimlik: oturum belirteci imzaya girer", "url": s3_url("s3://sentinel-cogs/a/b.tif", "us-west-2"),
         "region": "us-west-2", "headers": {"Range": "bytes=100-199"}, "token": "IQoJb3JpZ2luX2VjEXAMPLETOKEN//+=="},
        {**base, "name": "anahtarda boşluk ve Türkçe harf: yol bir kez kodlanır", "url": s3_url("s3://kova/şehir haritası/ğ.tif", "eu-central-1"),
         "region": "eu-central-1", "headers": {"Range": "bytes=0-0"}},
        {**base, "name": "adında nokta olan kova: yol biçimi", "url": s3_url("s3://my.bucket.tr/x/y.tif", "eu-west-1"),
         "region": "eu-west-1", "headers": {"Range": "bytes=0-1023"}},
        {**base, "name": "sorgulu adres: anahtarlar sıralı, değerler kodlu", "url": "https://kova.s3.us-east-1.amazonaws.com/a.tif?versionId=3%2B1&acl",
         "region": "us-east-1", "headers": {"Range": "bytes=0-9"}},
        {**base, "name": "HEAD isteği", "method": "HEAD", "url": "https://kova.s3.us-east-1.amazonaws.com/a.tif", "region": "us-east-1",
         "headers": {}},
        {**base, "name": "başlık değerindeki fazla boşluklar teke iner", "url": "https://kova.s3.us-east-1.amazonaws.com/a.tif",
         "region": "us-east-1", "headers": {"Range": "  bytes=0-9  ", "x-amz-request-payer": "requester"}},
        {**base, "name": "başka gün ve bölge", "time": "20261026T134501Z", "url": s3_url("s3://kova/k/l.tif", "ap-southeast-2"),
         "region": "ap-southeast-2", "headers": {"Range": "bytes=65536-131071"}},
        {**base, "name": "başka anahtar", "access": "AKIDEXAMPLE", "secret": "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
         "url": s3_url("s3://kova/k.tif", "us-east-1"), "region": "us-east-1", "headers": {"Range": "bytes=0-9"}},
        {**base, "name": "kök yol", "url": "https://kova.s3.us-east-1.amazonaws.com/", "region": "us-east-1", "headers": {}},
        {**base, "name": "yolda + ve = karakterleri", "url": s3_url("s3://kova/a+b=c.tif", "us-east-1"), "region": "us-east-1",
         "headers": {"Range": "bytes=0-9"}},
    ]
    for c in out:
        c.setdefault("token", None)
        got = sign(c)
        if "expectSignature" in c:
            assert got["signature"] == c.pop("expectSignature"), "AWS'nin kendi örneği tutmadı"
        c["expect"] = got
    return out


def s3_cases():
    raw = [
        ("sanal sunuculu adres", "s3://sentinel-cogs/sentinel-s2-l2a-cogs/36/S/VK/2024/7/B04.tif", "us-west-2"),
        ("Landsat'ın istek ödemeli kovası", "s3://usgs-landsat/collection02/level-2/x/SR_B4.TIF", "us-west-2"),
        ("bölge yok: us-east-1", "s3://kova/a.tif", None),
        ("adında nokta olan kova: yol biçimi", "s3://my.bucket/a.tif", "eu-west-1"),
        ("anahtarda boşluk", "s3://kova/klasör adı/a b.tif", "eu-central-1"),
        ("anahtarda + ve %", "s3://kova/a+b%c.tif", "us-east-1"),
        ("iç içe klasörler", "s3://kova/a/b/c/d.tif", "sa-east-1"),
        ("yalnız kova: kök", "s3://kova/", "us-east-1"),
    ]
    return [{"name": n, "href": h, "region": r, "expect": s3_url(h, r)} for n, h, r in raw]


def sas_cases():
    raw = [
        ("Planetary Computer'ın blob adresi", "https://sentinel2l2a01.blob.core.windows.net/sentinel2-l2/36/S/VK/B04.tif",
         "st=2026-10-26T10%3A00%3A00Z&se=2026-10-27T11%3A00%3A00Z&sp=rl&sv=2024-05-04&sr=c&sig=abc%2Bdef%3D"),
        ("belirteç başında ? ile", "https://landsateuwest.blob.core.windows.net/landsat-c2/l/a.TIF", "?se=2026&sig=x"),
        ("adresin kendi sorgusu korunur", "https://acct.blob.core.windows.net/cont/a.tif?versionid=7", "sig=y&se=z"),
        ("blob olmayan adres: belirteç yok", "https://example.com/cont/a.tif", "sig=y"),
        ("kapsayıcısız adres: belirteç yok", "https://acct.blob.core.windows.net/a.tif", "sig=y"),
        ("http blob adresi: belirteç yok", "http://acct.blob.core.windows.net/cont/a.tif", "sig=y"),
    ]
    out = []
    for n, url, token in raw:
        parts = blob_parts(url)
        out.append({"name": n, "url": url, "token": token, "expect": {"parts": parts, "signed": with_sas(url, token) if parts else None}})
    return out


def build():
    return {"format": "kentos.stac-access-cases", "version": 1, "sigv4": sigv4_cases(), "s3": s3_cases(), "sas": sas_cases()}


def botocore_check(doc):
    """Every signature again with botocore's own signer (an implementation that is not ours)."""
    from botocore.auth import S3SigV4Auth
    from botocore.awsrequest import AWSRequest
    from botocore.credentials import Credentials

    bad = []
    for c in doc["sigv4"]:
        req = AWSRequest(method=c["method"], url=c["url"], headers=dict(c["headers"]))
        req.context["timestamp"] = c["time"]
        auth = S3SigV4Auth(Credentials(c["access"], c["secret"], c["token"]), "s3", c["region"])
        auth._timestamp = c["time"]
        orig = datetime.datetime

        class Frozen(datetime.datetime):
            @classmethod
            def utcnow(cls):
                return orig.strptime(c["time"], "%Y%m%dT%H%M%SZ")

            @classmethod
            def now(cls, tz=None):
                return orig.strptime(c["time"], "%Y%m%dT%H%M%SZ").replace(tzinfo=tz)

        import botocore.auth as ba
        saved = ba.datetime.datetime
        ba.datetime.datetime = Frozen
        try:
            auth.add_auth(req)
        finally:
            ba.datetime.datetime = saved
        got = req.headers["Authorization"]
        if got != c["expect"]["authorization"]:
            bad.append(f"{c['name']}: botocore {got} \n  biz {c['expect']['authorization']}")
    return bad


def main():
    check = "--check" in sys.argv[1:]
    doc = build()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    problems = []
    if "--botocore" in sys.argv[1:]:
        problems += botocore_check(doc)
    if check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            problems.append(f"{OUT.name} güncel değil")
    else:
        OUT.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems))
        sys.exit(1)
    n = len(doc["sigv4"]) + len(doc["s3"]) + len(doc["sas"])
    print(f"{OUT.relative_to(ROOT)}: {n} durum" + ("; güncel." if check else " yazıldı."))


if __name__ == "__main__":
    main()
