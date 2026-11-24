# KentOS CAD geliştirme komutları. `make` (ya da `make help`) hepsini gruplarıyla listeler.
#
# Servisler (API, masaüstü, arayüz vitrini) arka planda, kendi
# süreç gruplarında çalışır: PID'leri ve günlükleri .run/ altındadır; `make stop`
# hepsini (bütün alt süreçleriyle) kapatır, `make status` neyin çalıştığını gösterir.
#
# Ağır işler (cargo, Vitest, ölçüm, derleme) /tmp/kentos-heavy.lock kilidiyle
# sırayla çalışır: iki terminalden aynı anda başlatılırsa ikincisi bekler, makine
# kilitlenmez (CLAUDE.md §2). Kilitsiz çalıştırmak için: make test-rust HEAVY=
#
# Web uygulaması donduruldu (ADR 0228, F3): apps/web kodu durur, hedefleri (dev, build, e2e, envanter…) yoktur.
#
# Değiştirilebilir değerler (örnek: make dev-api API_PORT=8788):
#   PORTAL_PORT   bulut portalı geliştirme sunucusu (5173; pnpm portal)
#   API_PORT      kentosd (8787)
#   PG_CONTAINER  yerel PostGIS Docker kabı ve compose servisi (postgis)
#   DB_COMPOSE    PostGIS'i tanımlayan compose dosyası (~/Projects/database/compose.yml; yoksa docker start/stop)
#   TEST_DB       Rust veritabanı testleri: .env.local varsa "required" (atlanamaz), yoksa boş
#   LABEL         ölçüm raporunun etiketi (latest)
#   ARGS          kentosd yönetim komutuna geçen argümanlar
#   YES           1 ise db-stop onay sormaz
#   FILE          desktop-snapshot'ın çizeceği .kcad dosyası (boşsa açık çizim yok)

SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help
MAKEFLAGS += --no-print-directory

PORTAL_PORT ?= 5173
API_PORT ?= 8787
PG_CONTAINER ?= postgis
DB_COMPOSE ?= $(HOME)/Projects/database/compose.yml
TEST_DB ?= $(if $(wildcard .env.local),required,)
LABEL ?= latest
ARGS ?=
YES ?=
FILE ?=

SVC := $(CURDIR)/scripts/dev/svc.sh
HEAVY ?= $(SVC) heavy
API_URL := http://127.0.0.1:$(API_PORT)
PORTAL_URL := http://localhost:$(PORTAL_PORT)

.PHONY: help install setup doctor \
		up down dev-api desktop desktop-release showcase stop stop-api stop-desktop stop-showcase \
		restart status logs logs-api logs-desktop \
		db-status db-start db-stop db-setup db-migrate kentosd \
		build-portal build-api build-rust build-desktop catalog \
		check verify test-rust test-portal test-desktop test-module fixtures-changed fixtures-check fmt fmt-check arch ui-snapshots desktop-snapshot \
		clean clean-rust

##@ Yardım

help: ## Bu listeyi gösterir (varsayılan hedef)
	@awk 'BEGIN { FS = ":.*## " } \
	  /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } \
	  /^[a-zA-Z0-9_.-]+:.*## / { printf "  \033[36m%-17s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@printf "\nDeğişkenler: Makefile'ın başı. Servis günlükleri: .run/<servis>.log\n"

##@ Kurulum

install: ## pnpm bağımlılıklarını kilit dosyasına göre kurar
	pnpm install --frozen-lockfile

setup: install ## İlk kurulum: pnpm bağımlılıkları (portal)
doctor: ## Araç sürümlerini, servisleri ve veritabanını gösterir
	@echo "Araçlar:"
	@for c in "rustc --version" "cargo --version" "node --version" "pnpm --version" "docker --version"; do \
	  printf "  %-24s %s\n" "$${c%% *}" "$$($$c 2>/dev/null | head -1 || echo yok)"; done
	@echo "Servisler:"; $(SVC) status
	@$(MAKE) db-status

##@ Servisler (arka planda; durdurmak: make stop, günlükler: make logs)

up: dev-api desktop ## API + masaüstü uygulaması

down: stop ## Her şeyi durdurur (make stop ile aynı)

dev-api: ## Yalnız API (kentosd serve; .env.local ve PostGIS gerekir)
	@[ -f .env.local ] || { echo "• .env.local yok: önce yerel veritabanını kurun (make db-setup)"; exit 1; }
	@$(SVC) stop api
	@$(HEAVY) cargo build -q -p kentos-api
	@KENTOS_API_PORT=$(API_PORT) KENTOS_PUBLIC_URL=$(PORTAL_URL) $(SVC) start api target/debug/kentosd serve
	@$(SVC) wait api $(API_URL)/v1/health 30

desktop: ## Masaüstü programı kentos-cad (MODUL=mimari|harita|cbs|yol ile modülün programı, FILE=çizim.kcad)
	@cargo update -q -p acadrust
	@$(HEAVY) cargo build -q -p kentos-cad
	@$(SVC) start desktop target/debug/kentos-cad $(if $(MODUL),--modul $(MODUL)) $(FILE)

desktop-release: ## Masaüstü programını ve başlatıcıyı release derler (bu Mac'te ~8 dk), modül programlarının bağlantılarını koyar
	cargo update -q -p acadrust
	$(HEAVY) cargo build --release -p kentos-cad -p kentos-launcher -j 10
	python3 scripts/desktop/module_links.py target/release
	@echo "• target/release/kentos-cad (kentos-mimari, kentos-harita, kentos-gis, kentos-yol) ve target/release/kentos-launcher"

showcase: ## Arayüz bileşenleri vitrini (apps/ui-showcase)
	@$(HEAVY) cargo build -q -p kentos-ui-showcase
	@$(SVC) start showcase target/debug/kentos-ui-showcase

stop: ## Bütün servisleri durdurur (API, masaüstü, vitrin; veritabanına dokunmaz)
	@$(SVC) stop

stop-api: ## API'yi durdurur
	@$(SVC) stop api

stop-desktop: ## Masaüstü uygulamasını kapatır
	@$(SVC) stop desktop

stop-showcase: ## Arayüz vitrinini kapatır
	@$(SVC) stop showcase

restart: ## Durdurup API'yi ve masaüstünü yeniden başlatır
	@$(MAKE) stop
	@$(MAKE) up

status: ## Çalışan servisler, adresleri ve veritabanı
	@echo "Servisler:"; $(SVC) status
	@echo "Adresler:"
	@$(SVC) running api && echo "  API       $(API_URL)/v1/health" || true
	@echo "Durdurmak: make stop · Günlükler: make logs"

logs: ## Bütün servis günlüklerini izler (Ctrl+C çıkar)
	@ls .run/*.log >/dev/null 2>&1 || { echo "• henüz günlük yok"; exit 0; }
	@tail -n 40 -F .run/*.log

logs-api: ## API günlüğü
	@tail -n 60 -F .run/api.log

logs-desktop: ## Masaüstü uygulamasının günlüğü
	@tail -n 60 -F .run/desktop.log

##@ Veritabanı (yerel PostGIS; yalnız geliştirme makinesi)

db-status: ## PostGIS kabının ve bağlantının durumu
	@echo "Veritabanı:"
	@docker ps -a --filter "name=^$(PG_CONTAINER)$$" --format '  {{.Names}}: {{.Status}} ({{.Image}})' 2>/dev/null | grep . || echo "  $(PG_CONTAINER) kabı bulunamadı (PG_CONTAINER=...)"
	@docker exec $(PG_CONTAINER) pg_isready -q 2>/dev/null && echo "  bağlantı hazır" || echo "  bağlantı yok"
	@[ -f "$(DB_COMPOSE)" ] && echo "  compose: $(DB_COMPOSE)" || echo "  compose dosyası yok ($(DB_COMPOSE)); kap docker ile yönetiliyor"

# The compose file also holds other services (redis, mongodb, keycloak) and
# the PostGIS server hosts other applications' databases: only the one
# service is started, and a running container is never recreated.
db-start: ## PostGIS'i başlatır: compose varsa up --no-recreate (çalışan kabı yeniden kurmaz), yoksa docker start
	@if [ -f "$(DB_COMPOSE)" ]; then docker compose -f "$(DB_COMPOSE)" up -d --no-recreate $(PG_CONTAINER); else docker start $(PG_CONTAINER); fi

db-stop: ## PostGIS kabını durdurur; DİKKAT: kaptaki başka uygulamaların veritabanları da kapanır
	@[ "$(YES)" = 1 ] || { read -r -p "$(PG_CONTAINER) kabı durdurulsun mu? İçindeki başka veritabanları da kapanır [e/H] " a; [ "$$a" = e ] || { echo "vazgeçildi"; exit 0; }; }
	@if [ -f "$(DB_COMPOSE)" ]; then docker compose -f "$(DB_COMPOSE)" stop $(PG_CONTAINER); else docker stop $(PG_CONTAINER); fi

db-setup: ## Roller, kentos_cad veritabanı, migration ve geliştirme verisi (yalnız yerel!)
	$(HEAVY) pnpm db:setup

db-migrate: ## Bekleyen migration'ları uygular
	$(HEAVY) pnpm kentosd -- migrate

kentosd: ## Yönetim CLI'si: make kentosd ARGS="tenant list"
	$(HEAVY) pnpm kentosd -- $(ARGS)

##@ Derleme

build-portal: ## Bulut portalı üretim derlemesi: tsc + Vite (apps/portal/dist)
	$(HEAVY) pnpm portal:build

build-api: ## kentosd'yi derler
	$(HEAVY) cargo build -p kentos-api

build-rust: ## Sunucu ve ortak Rust crate'lerini derler (masaüstü hariç)
	$(HEAVY) cargo build

build-desktop: ## Masaüstü programını (kentos-cad), arayüz kütüphanesini ve vitrini derler
	cargo update -q -p acadrust
	$(HEAVY) cargo build -p kentos-ui -p kentos-ui-showcase -p kentos-cad

catalog: ## Ürün komutu katalogunu yeniden yazar (ADR 0013; farkı okuyup commit edin)
	$(HEAVY) env KENTOS_WRITE_CATALOG=1 cargo test -q -p kentos-commands catalog

##@ Test ve denetim (sırayla, ağır iş kilidiyle)

check: ## Hızlı takım: fmt-check + test-rust + test-desktop + test-portal
	@$(MAKE) fmt-check test-rust test-desktop test-portal

verify: ## Commit öncesi tam takım: check + portal ve masaüstü derlemesi
	@$(MAKE) check build-portal build-desktop

test-rust: ## cargo test + clippy + bağımlılık yönü (TEST_DB=required: DB testleri atlanamaz)
	$(HEAVY) env KENTOS_TEST_DB=$(TEST_DB) pnpm rust:test

test-portal: ## Bulut portalı: tip denetimi + Vitest
	$(HEAVY) pnpm -C apps/portal typecheck
	$(HEAVY) pnpm portal:test

test-desktop: ## Arayüz kütüphanesi, vitrin ve masaüstü testleri + clippy
	$(HEAVY) pnpm rust:test:desktop

test-module: ## Uygulama katmanı testleri tek modülle: make test-module MOD=arch|map|gis|road (ADR 8054 §11)
	$(if $(filter $(MOD),arch map gis road),,$(error MOD=arch, map, gis ya da road olmalı))
	$(HEAVY) cargo test -p kentos-native-application --no-default-features --features mod-$(MOD)

fixtures-changed: ## Yalnız değişenin ortak komut durumları (BASE=origin/main, FILES=a,b; eşlenemeyen değişiklikte hepsi)
	$(HEAVY) node scripts/dev/changed-fixtures.mjs

fixtures-check: ## Bütün üreteçlerin --check'i sabit Python ortamında; özet: yeşil, kırmızı, koşamayan (ADR 9303; ONLY=ad, JOBS=4)
	$(HEAVY) scripts/fixtures/py scripts/fixtures/check_all.py $(if $(JOBS),-j $(JOBS),) $(ONLY)

fmt: ## Rust kaynaklarını biçimler
	cargo fmt --all

fmt-check: ## Rust biçim denetimi
	cargo fmt --all -- --check

arch: ## Crate bağımlılık yönü (ADR 0010) ve kaynak boyu (2 000 satır; taban yalnız küçülür) denetimi
	pnpm arch:deps
	python3 scripts/dev/source_size.py

ui-snapshots: ## Arayüz vitrinini ekransız çizer, başvuru görüntüleriyle karşılaştırır
	$(HEAVY) cargo build -q -p kentos-ui-showcase
	$(HEAVY) node scripts/ui/snapshots.mjs

desktop-snapshot: ## Masaüstü penceresini görüntüye çizer: .run/desktop.png (FILE=çizim.kcad; çizim alanı wgpu ister)
	$(HEAVY) cargo build -q -p kentos-cad
	mkdir -p .run && KENTOS_SNAPSHOT_BACKEND=wgpu target/debug/kentos-cad snapshot .run/desktop.png $(FILE)
	@echo "• .run/desktop.png"

##@ Temizlik

clean: ## Servisleri durdurur; derleme çıktıları (portal dist) ve .run/ silinir
	@$(SVC) stop
	rm -rf apps/portal/dist dist .run

clean-rust: ## cargo clean: bütün Rust derleme çıktısı (sonraki derleme uzun sürer)
	cargo clean
