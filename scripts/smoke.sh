#!/bin/sh
# compose で立てた全体（make up）を、frontend の入口（http://localhost:8080）から HTTP で端から端まで試す。
# 依存: curl, jq
set -eu
URL=${URL:-http://localhost:8080}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
JAR="$TMP/jar"

i=0
until curl -fsS "$URL/api/products" >/dev/null 2>&1; do
  i=$((i + 1))
  [ "$i" -lt 180 ] || { echo "FAIL: $URL が上がらない" >&2; exit 1; }
  sleep 1
done

# call METHOD PATH [BODY] → 本文を $TMP/body に、状態を標準出力に
call() {
  curl -sS -o "$TMP/body" -w '%{http_code}' -b "$JAR" -c "$JAR" -X "$1" \
    -H 'content-type: application/json' ${3:+--data "$3"} "$URL$2"
}
expect() {
  [ "$1" = "$2" ] || { echo "FAIL: $3: $1（期待 $2）: $(cat "$TMP/body")" >&2; exit 1; }
  echo "ok   $3"
}
uuid() { od -An -N16 -tx1 /dev/urandom | tr -d ' \n' | sed -E 's/^(.{8})(.{4})(.{4})(.{4})(.{12})$/\1-\2-\3-\4-\5/'; }

stock() { curl -fsS "$URL/api/products" | jq ".[] | select(.id == $1) | .stock"; }

# 代表データの注文 5 は、決済サービスに届かないまま残った pending の注文（トートバッグを 1 つ引き当て済み）。
# proxy の突き合わせが declined に片付け、在庫を 2 から 3 に戻す（起動してすぐと、その後 1 分ごとに走る）
i=0
until [ "$(stock 5)" = 3 ]; do
  i=$((i + 1))
  [ "$i" -lt 90 ] || { echo "FAIL: 決着しない注文の在庫が戻らない（$(stock 5)）" >&2; exit 1; }
  sleep 1
done
echo "ok   決着しないまま古くなった注文を片付け、在庫を戻す"

expect "$(call POST /api/login '{"email":"Hanako@Example.com","password":"password"}')" 200 "大文字のメールでもログインできる"
expect "$(call GET /api/me)" 200 "自分を読める"

before=$(stock 3)
key=$(uuid)
body="{\"request_key\":\"$key\",\"lines\":[{\"product_id\":3,\"quantity\":2}],\"card_token\":\"tok_visa\"}"
expect "$(call POST /api/orders "$body")" 201 "注文できる"
id=$(jq .id "$TMP/body")
expect "$(jq -r .status "$TMP/body")" paid "注文は支払済み"
expect "$(call POST /api/orders "$body")" 201 "同じ依頼をもう一度送れる"
expect "$(jq .id "$TMP/body")" "$id" "同じ依頼には同じ注文"
expect "$(stock 3)" "$((before - 2))" "在庫は 1 度だけ減る"

changed="{\"request_key\":\"$key\",\"lines\":[{\"product_id\":3,\"quantity\":1}],\"card_token\":\"tok_visa\"}"
expect "$(call POST /api/orders "$changed")" 409 "同じキーで違う中身の注文は断る"

before=$(stock 1)
declined="{\"request_key\":\"$(uuid)\",\"lines\":[{\"product_id\":1,\"quantity\":1}],\"card_token\":\"tok_declined\"}"
expect "$(call POST /api/orders "$declined")" 409 "拒否されるカードは断る"
expect "$(stock 1)" "$before" "拒否された注文の在庫は戻る"

expect "$(curl -sS -o "$TMP/body" -w '%{http_code}' -b "$JAR" -X POST -H 'content-type: application/json' \
  -H 'origin: https://evil.example' --data "$body" "$URL/api/orders")" 403 "別のサイトからの注文は断る"

cp "$JAR" "$TMP/old"
expect "$(call POST /api/logout)" 204 "ログアウトできる"
expect "$(curl -sS -o "$TMP/body" -w '%{http_code}' -b "$TMP/old" "$URL/api/me")" 401 "ログアウトの前の cookie は使えない"

expect "$(curl -sS -o /dev/null -w '%{http_code}' "$URL/assets/missing.js")" 404 "ない静的ファイルは 404"
curl -sSI "$URL/" | grep -qi '^content-security-policy:' && echo "ok   CSP が付く" || { echo "FAIL: CSP がない" >&2; exit 1; }
echo "smoke ok"
