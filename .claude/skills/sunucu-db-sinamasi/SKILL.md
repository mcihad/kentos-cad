---
name: sunucu-db-sinamasi
description: KentOS sunucusunun veritabanı sınamalarını (kentos-application, kentos-postgres, kentos-api) bu makinede gerçek PostGIS üstünde koşturmak — geçici Docker kabı, roller, ortam değişkenleri. Sunucu koduna ya da sunucu sınamalarına dokunulduğunda kullanın.
---

# Sunucunun veritabanı sınamaları

Depoda `.env.local` yoksa `kentos_postgres::testing::TestDb` veritabanı sınamalarını atlar ve bunu yalnız yazıyla
söyler. Atlanan sınama geçmiş sayılmaz: `KENTOS_TEST_DB=required` ile koşun.

Başka işlere ait kaplara ve portlara (5432, 5433, 5434, 55432) dokunmayın; kendi geçici kabınızı boş bir portta açın.

```sh
docker run -d --rm --name kentos-test-<ad> -e POSTGRES_PASSWORD=yerel-test -p 127.0.0.1:55440:5432 postgis/postgis:18-3.6
docker exec kentos-test-<ad> psql -U postgres \
  -c "create role kentos_cad_owner with login nosuperuser nocreatedb nocreaterole password 'sahip-test'" \
  -c "create role kentos_cad_app with login nosuperuser nocreatedb nocreaterole nobypassrls noinherit password 'uygulama-test'"

export KENTOS_TEST_DB=required \
  KENTOS_TEST_ADMIN_URL=postgres://postgres:yerel-test@127.0.0.1:55440/postgres \
  KENTOS_DATABASE_URL=postgres://kentos_cad_app:uygulama-test@127.0.0.1:55440/kentos_cad \
  KENTOS_DATABASE_OWNER_URL=postgres://kentos_cad_owner:sahip-test@127.0.0.1:55440/kentos_cad \
  KENTOS_PG_DUMP=/opt/homebrew/opt/libpq/bin/pg_dump KENTOS_PG_RESTORE=/opt/homebrew/opt/libpq/bin/pg_restore

scripts/dev/svc.sh heavy cargo test -p kentos-application --test all [<modül>::]
docker stop kentos-test-<ad>     # --rm: kap durunca silinir
```

- Her sınama kendi `kentos_cad_test_<zaman>_<rastgele>` veritabanını açar ve kapatır; modüller durum paylaşmaz.
- `pg_dump` ve `pg_restore` PATH'te değildir; yedek sınaması yukarıdaki iki değişkeni ister.
- Ölçüm (28 Kasım 2026, bu Mac): `--test all` 167 sınama, 63 sn.
- `disk_full` sınaması ayrı programdır; onu `scripts/ops/disk_full_drill.sh` koşar.
- Docker'ın API'si yanıt vermiyorsa (`docker ps` asılı kalır) Docker Desktop'ı yeniden başlatmak için kullanıcıya sorun.
