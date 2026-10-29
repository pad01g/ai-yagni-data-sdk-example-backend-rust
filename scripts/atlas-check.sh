#!/bin/sh
# Atlas でスキーマの正本（schema.sql）を確かめる。
# `atlas schema lint` は有料版（Atlas Pro）だけなので、無料版（community）でできることで代える。
#
#   1. 空の DB に schema.sql が当たる。
#   2. 当てた後にもう一度差分を取ると、何も残らない（Atlas が schema.sql をそのとおりに理解している）。
#   3. BASE（既定 origin/main）の schema.sql から今の schema.sql への差分に、データを壊す変更
#      （DROP TABLE / DROP COLUMN）がない。壊すと決めたなら ALLOW_DESTRUCTIVE=1 で通す。
#
# 依存: docker, git
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
ATLAS=${ATLAS:-arigaio/atlas:1.3.4-community}
BASE=${BASE:-origin/main}
NAME=ai-yagni-data-atlas-$$
TMP=$(mktemp -d)
cleanup() {
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  docker network rm "$NAME" >/dev/null 2>&1 || true
  rm -rf "$TMP"
}
trap cleanup EXIT

docker network create "$NAME" >/dev/null
docker run -d --rm --name "$NAME" --network "$NAME" --tmpfs /var/lib/mysql \
  -e MYSQL_ROOT_PASSWORD=atlas -e MYSQL_DATABASE=shop mysql:8.4.6 >/dev/null
i=0
until docker exec "$NAME" mysql -uroot -patlas -h127.0.0.1 -e "CREATE DATABASE IF NOT EXISTS atlas_dev" >/dev/null 2>&1; do
  i=$((i + 1))
  [ "$i" -lt 180 ] || { echo "mysql did not become ready" >&2; exit 1; }
  sleep 1
done

URL="mysql://root:atlas@$NAME:3306/shop"
DEV="mysql://root:atlas@$NAME:3306/atlas_dev"
atlas() { docker run --rm --network "$NAME" -v "$ROOT":/s:ro -v "$TMP":/t:ro "$ATLAS" "$@"; }

echo "== 1. schema.sql が当たる"
atlas schema apply --url "$URL" --to file:///s/schema.sql --dev-url "$DEV" --auto-approve >/dev/null

echo "== 2. 当てた後に差分が残らない"
left=$(atlas schema diff --from "$URL" --to file:///s/schema.sql --dev-url "$DEV")
case "$left" in
  *"Schemas are synced"*) ;;
  *) echo "$left"; echo "FAIL: schema.sql を当てた後にも差分が残る" >&2; exit 1 ;;
esac

echo "== 3. $BASE から壊す変更がない"
if git -C "$ROOT" cat-file -e "$BASE:schema.sql" 2>/dev/null; then
  git -C "$ROOT" show "$BASE:schema.sql" > "$TMP/base.sql"
  chmod a+r "$TMP/base.sql"
  diff=$(atlas schema diff --from file:///t/base.sql --to file:///s/schema.sql --dev-url "$DEV")
  echo "$diff" | sed 's/^/   /'
  if echo "$diff" | grep -Eiq 'DROP TABLE|DROP COLUMN'; then
    if [ "${ALLOW_DESTRUCTIVE:-}" = 1 ]; then
      echo "   (ALLOW_DESTRUCTIVE=1 なので、壊す変更を通す)"
    else
      echo "FAIL: データを壊す変更（DROP TABLE / DROP COLUMN）がある。決めたなら ALLOW_DESTRUCTIVE=1" >&2
      exit 1
    fi
  fi
else
  echo "   ($BASE に schema.sql がないので飛ばす)"
fi
echo "ok"
