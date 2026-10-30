# ホストには docker 以外何も入れない。CI（GitHub Actions）も同じターゲットを走らせる。
DEV      := ai-yagni-data-rust-dev
VERUS    := ai-yagni-data-verus
NAME     := ai-yagni-data-sdk-example-backend-rust
ATLAS    := arigaio/atlas:1.3.4-community
CACHE    := -v ai-yagni-data-cargo:/usr/local/cargo/registry -v ai-yagni-data-cargo-git:/usr/local/cargo/git -v $(NAME)-target:/work/target
# ビルドとテストのコンテナには docker のソケットを渡さない（依存の build.rs がホストを操作できないように）。
# 渡すのは、検査の中で MySQL のコンテナを立てるデータの未来の検査（FUTURES）だけ。
RUN      := docker run --rm $(CACHE) -v $(CURDIR):/work -w /work $(DEV)
FUTURES  := docker run --rm -v /var/run/docker.sock:/var/run/docker.sock $(CACHE) -v $(CURDIR):/work -w /work $(DEV)
BASE     ?= origin/main

.DEFAULT_GOAL := help
.PHONY: help image verus-image submodules build test lint fmt verify schema proxy-schema futures futures-report futures-pr ci env up down logs sh

help: ## このヘルプ
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

image: ## ビルドとテストのイメージを作る
	docker build -q -t $(DEV) docker/dev >/dev/null

verus-image: ## Verus のイメージを作る（amd64）
	docker build -q --platform linux/amd64 -t $(VERUS) docker/verus >/dev/null

submodules: ## submodule（frontend・proxy・payment）を取る
	git submodule update --init

build: image ## ビルド
	$(RUN) cargo build --all-targets --locked

test: image ## テスト
	$(RUN) cargo test --locked

lint: image ## clippy と rustfmt
	$(RUN) sh -c 'cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings'

fmt: image ## 整形する
	$(RUN) cargo fmt --all

verify: verus-image ## Verus で証明を確かめる（verified/）
	docker run --rm --platform linux/amd64 -v ai-yagni-data-verus-cargo:/opt/cargo/registry -v ai-yagni-data-verus-git:/opt/cargo/git \
	  -v $(CURDIR):/work -w /work/verified $(VERUS) cargo verus verify --target-dir /tmp/verus-target

schema: ## Atlas: schema.sql が当たり、当てた後に差分が残らず、BASE から壊す変更（DROP）がないこと
	ATLAS=$(ATLAS) BASE=$(BASE) scripts/atlas-check.sh

proxy-schema: submodules ## proxy のテスト用のスキーマ（testdata）が schema.sql の写しであること
	@sed '1,/^SET NAMES utf8mb4;$$/d' submodules/proxy/testdata/01_schema.sql | diff -u schema.sql - \
	  || { echo "submodules/proxy/testdata/01_schema.sql が schema.sql と違う。proxy の写しを直す" >&2; exit 1; }

futures: image submodules ## データの未来の検査（CI で走らせるもの）
	$(FUTURES) cargo run -q --locked -p datafutures -- check

futures-report: image submodules ## 語彙の対応表と、未来どうしの組み合わせの表
	$(FUTURES) cargo run -q --locked -p datafutures -- report

futures-pr: image submodules ## 共変更の検査も含める（BASE からの変更を見る）
	git diff --name-only --no-renames $(BASE)...HEAD > .changed-files
	$(FUTURES) cargo run -q --locked -p datafutures -- check --changed /work/.changed-files --repo /work

ci: lint test schema proxy-schema futures verify ## CI で走らせるもの全部

env: .env ## 手元の秘密（DB のパスワード、セッションの鍵）を乱数で作る（あれば何もしない）

.env:
	@umask 077; r() { od -An -N$$1 -tx1 /dev/urandom | tr -d ' \n'; }; { \
	  echo "MYSQL_ROOT_PASSWORD=$$(r 24)"; \
	  echo "PROXY_DB_PASSWORD=$$(r 24)"; \
	  echo "SESSION_SECRET=$$(r 32)"; \
	} > .env
	@echo ".env を作った（リポジトリには入れない）"

up: submodules .env ## 全体を起動する（http://localhost:8080）
	docker compose up --build -d
	@echo "http://localhost:8080 （hanako@example.com / password）"

down: ## 全体を止める（DB と決済の記録も消す。.env は残す）
	docker compose down -v

logs: ## ログを見る
	docker compose logs -f

sh: image ## 開発用のシェル
	docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock $(CACHE) -v $(CURDIR):/work -w /work $(DEV) bash
