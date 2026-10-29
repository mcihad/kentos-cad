#!/bin/sh
# The point-in-time recovery drill (docs/adr/0744 §2): a throwaway PostgreSQL/PostGIS in docker (never the developer's
# database, never port 5432) archives its WAL, takes a base backup, writes rows before and after a moment T, and a
# second server recovers the base backup with the archive up to T. The drill says how many rows came back (all before
# T, none after), the recovery's time (RTO) and the archive's bound on what could be lost (RPO: `archive_timeout`).
#
#   scripts/ops/pitr_drill.sh [ROWS]     # rows on each side of T, default 20000
set -eu
rows=${1:-20000}
image=postgis/postgis:18-3.6
tag="kentos-pitr-$$"
volume="$tag"
first="$tag-1"
second="$tag-2"
cleanup() {
	docker rm -f "$first" "$second" >/dev/null 2>&1 || true
	docker volume rm "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT
psql_in() { docker exec -u postgres "$1" psql -X -q -t -A -v ON_ERROR_STOP=1 -d postgres -c "$2"; }
ready() {
	n=0
	# Over TCP: the image's first-start server listens on its socket only, and restarts.
	until docker exec -u postgres "$1" pg_isready -q -h 127.0.0.1 -d postgres 2>/dev/null; do
		n=$((n + 1))
		[ "$n" -lt 600 ] || { echo "$1 hazır olmadı" >&2; docker logs "$1" >&2; exit 1; }
		sleep 0.1
	done
}

docker volume create "$volume" >/dev/null
# The archive and the base backup live on the volume; the first server's own data stays in its container.
docker run -d --name "$first" -e POSTGRES_PASSWORD=pitr -v "$volume:/pitr" "$image" \
	-c wal_level=replica -c archive_mode=on -c archive_timeout=1 \
	-c "archive_command=test ! -f /pitr/wal/%f && cp %p /pitr/wal/%f" >/dev/null
docker exec -u root "$first" sh -c 'mkdir -p /pitr/wal && chown -R postgres:postgres /pitr'
ready "$first"
psql_in "$first" "create table pitr (n bigint primary key, side text not null, at timestamptz not null default clock_timestamp())"
docker exec -u postgres "$first" pg_basebackup -D /pitr/base -X none -c fast
psql_in "$first" "insert into pitr (n, side) select g, 'önce' from generate_series(1, $rows) g"
psql_in "$first" "select pg_switch_wal()" >/dev/null
sleep 1.2
target=$(psql_in "$first" "select clock_timestamp()")
sleep 1.2
psql_in "$first" "insert into pitr (n, side) select g, 'sonra' from generate_series($rows + 1, 2 * $rows) g"
# The segment that holds the last write, switched out and waited for in the archive.
last=$(psql_in "$first" "select pg_walfile_name(pg_switch_wal())")
n=0
until docker exec "$first" test -f "/pitr/wal/$last"; do
	n=$((n + 1))
	[ "$n" -lt 300 ] || { echo "WAL arşivlenmedi: $last" >&2; exit 1; }
	sleep 0.1
done
lost_bound=$(psql_in "$first" "show archive_timeout")
docker stop "$first" >/dev/null

# The second server: the base backup, the archive, up to T.
docker run --rm -v "$volume:/pitr" "$image" sh -c "
	cat >> /pitr/base/postgresql.auto.conf <<EOF
restore_command = 'cp /pitr/wal/%f %p'
recovery_target_time = '$target'
recovery_target_action = 'promote'
archive_mode = off
EOF
	touch /pitr/base/recovery.signal
	chown -R postgres:postgres /pitr/base && chmod 700 /pitr/base"
start=$(date +%s.%N)
docker run -d --name "$second" -e PGDATA=/pitr/base -v "$volume:/pitr" "$image" >/dev/null
ready "$second"
n=0
until [ "$(psql_in "$second" "select not pg_is_in_recovery()")" = t ]; do
	n=$((n + 1))
	[ "$n" -lt 600 ] || { echo "kurtarma bitmedi" >&2; docker logs "$second" >&2; exit 1; }
	sleep 0.1
done
end=$(date +%s.%N)
before=$(psql_in "$second" "select count(*) from pitr where side = 'önce'")
after=$(psql_in "$second" "select count(*) from pitr where side = 'sonra'")
late=$(psql_in "$second" "select count(*) from pitr where at > '$target'")
rto=$(python3 -c "print(f'{$end - $start:.2f}')")
echo "hedef_an	$target"
echo "önce_yazılan	$rows	geri_gelen	$before"
echo "sonra_yazılan	$rows	geri_gelen	$after"
echo "hedeften_sonraki_satır	$late"
echo "kurtarma_süresi_s	$rto"
echo "arşivin_sınırı	$lost_bound"
if [ "$before" != "$rows" ] || [ "$after" != 0 ] || [ "$late" != 0 ]; then
	echo "PITR tatbikatı başarısız: hedefe kadarki satırlar dönmedi ya da sonrakiler döndü" >&2
	exit 1
fi
echo "PITR tatbikatı geçti"
