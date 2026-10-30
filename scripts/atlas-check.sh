#!/bin/sh
# Atlas でスキーマの正本（schema.sql）を確かめる。
# `atlas schema lint` は有料版（Atlas Pro）だけなので、無料版（community）でできることと MySQL 自身で代える。
#
#   1. 空の DB に schema.sql が当たる。
#   2. 当てた後にもう一度差分を取ると、何も残らない（Atlas が schema.sql をそのとおりに理解している）。
#   3. BASE（既定 origin/main）の schema.sql と比べて、データを壊しうる変更がない。両方を別の DB に当て、
#      information_schema を比べる（Atlas の差分の文面を grep するより漏れがない）:
#        - 表・列を消す
#        - 列の型を変える、文字の長さや数の桁を減らす、符号を変える、照合順序を変える、NULL を許さなくする
#        - 一意キーを消す（冪等キーなどの約束が黙って消える）
#      壊すと決めたなら ALLOW_DESTRUCTIVE=1 で通す。
#   4. BASE のスキーマに BASE の代表データ（fixtures/）を入れた DB へ、今の schema.sql が当たる
#      （今あるデータの上で変更が通る）。
#
# BASE が見つからなければ失敗にする（打ち間違いで黙って飛ばさない）。BASE に schema.sql がなければ 3 と 4 を飛ばす。
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

if ! git -C "$ROOT" rev-parse --verify --quiet "$BASE^{commit}" >/dev/null; then
  echo "FAIL: BASE=$BASE が見つからない（git fetch するか、BASE を正しく渡す）" >&2
  exit 1
fi

# compose と同じ照合順序で立てる（CHECK 制約や一意キーの比べ方が照合順序で変わる）
docker network create "$NAME" >/dev/null
docker run -d --rm --name "$NAME" --network "$NAME" --tmpfs /var/lib/mysql \
  -e MYSQL_ROOT_PASSWORD=atlas -e MYSQL_DATABASE=shop mysql:8.4.6 \
  --character-set-server=utf8mb4 --collation-server=utf8mb4_0900_bin >/dev/null
i=0
until docker exec "$NAME" mysql -uroot -patlas -h127.0.0.1 -e "CREATE DATABASE IF NOT EXISTS atlas_dev" >/dev/null 2>&1; do
  i=$((i + 1))
  [ "$i" -lt 180 ] || { echo "mysql did not become ready" >&2; exit 1; }
  sleep 1
done

URL="mysql://root:atlas@$NAME:3306/shop"
BASE_URL="mysql://root:atlas@$NAME:3306/base"
DEV="mysql://root:atlas@$NAME:3306/atlas_dev"
atlas() { docker run --rm --network "$NAME" -v "$ROOT":/s:ro -v "$TMP":/t:ro "$ATLAS" "$@"; }
sql() { docker exec -i "$NAME" mysql -uroot -patlas -h127.0.0.1 --default-character-set=utf8mb4 -N "$@" 2>/dev/null; }

echo "== 1. schema.sql が当たる"
atlas schema apply --url "$URL" --to file:///s/schema.sql --dev-url "$DEV" --auto-approve >/dev/null

echo "== 2. 当てた後に差分が残らない"
left=$(atlas schema diff --from "$URL" --to file:///s/schema.sql --dev-url "$DEV")
case "$left" in
  *"Schemas are synced"*) ;;
  *) echo "$left"; echo "FAIL: schema.sql を当てた後にも差分が残る" >&2; exit 1 ;;
esac

if ! git -C "$ROOT" cat-file -e "$BASE:schema.sql" 2>/dev/null; then
  echo "   ($BASE に schema.sql がないので 3 と 4 を飛ばす)"
  echo "ok"
  exit 0
fi

git -C "$ROOT" show "$BASE:schema.sql" > "$TMP/base.sql"
chmod a+r "$TMP/base.sql"
echo "CREATE DATABASE base" | sql
atlas schema apply --url "$BASE_URL" --to file:///t/base.sql --dev-url "$DEV" --auto-approve >/dev/null

echo "== 3. $BASE から壊しうる変更がない"
atlas schema diff --from file:///t/base.sql --to file:///s/schema.sql --dev-url "$DEV" | sed 's/^/   /'
found=$(sql information_schema <<'SQL'
SELECT CONCAT('表を消す: ', b.TABLE_NAME)
FROM TABLES b
WHERE b.TABLE_SCHEMA = 'base'
  AND NOT EXISTS (SELECT 1 FROM TABLES n WHERE n.TABLE_SCHEMA = 'shop' AND n.TABLE_NAME = b.TABLE_NAME)
UNION ALL
SELECT CONCAT('列を消す: ', b.TABLE_NAME, '.', b.COLUMN_NAME)
FROM COLUMNS b
WHERE b.TABLE_SCHEMA = 'base'
  AND EXISTS (SELECT 1 FROM TABLES n WHERE n.TABLE_SCHEMA = 'shop' AND n.TABLE_NAME = b.TABLE_NAME)
  AND NOT EXISTS (SELECT 1 FROM COLUMNS n
                  WHERE n.TABLE_SCHEMA = 'shop' AND n.TABLE_NAME = b.TABLE_NAME AND n.COLUMN_NAME = b.COLUMN_NAME)
UNION ALL
SELECT CONCAT('列を狭める: ', b.TABLE_NAME, '.', b.COLUMN_NAME, ' ', b.COLUMN_TYPE,
              IF(b.IS_NULLABLE = 'YES', ' NULL', ' NOT NULL'), ' -> ', n.COLUMN_TYPE,
              IF(n.IS_NULLABLE = 'YES', ' NULL', ' NOT NULL'))
FROM COLUMNS b
JOIN COLUMNS n ON n.TABLE_SCHEMA = 'shop' AND n.TABLE_NAME = b.TABLE_NAME AND n.COLUMN_NAME = b.COLUMN_NAME
WHERE b.TABLE_SCHEMA = 'base'
  AND (b.DATA_TYPE <> n.DATA_TYPE
       OR COALESCE(n.CHARACTER_MAXIMUM_LENGTH, 0) < COALESCE(b.CHARACTER_MAXIMUM_LENGTH, 0)
       OR COALESCE(n.NUMERIC_PRECISION, 0) < COALESCE(b.NUMERIC_PRECISION, 0)
       OR COALESCE(n.NUMERIC_SCALE, 0) <> COALESCE(b.NUMERIC_SCALE, 0)
       OR COALESCE(n.DATETIME_PRECISION, 0) < COALESCE(b.DATETIME_PRECISION, 0)
       OR (b.COLUMN_TYPE LIKE '%unsigned') <> (n.COLUMN_TYPE LIKE '%unsigned')
       OR NOT (b.COLLATION_NAME <=> n.COLLATION_NAME)
       OR (b.IS_NULLABLE = 'YES' AND n.IS_NULLABLE = 'NO'))
UNION ALL
SELECT CONCAT('一意キーを消す: ', b.TABLE_NAME, '.', b.INDEX_NAME)
FROM (SELECT DISTINCT TABLE_NAME, INDEX_NAME FROM STATISTICS WHERE TABLE_SCHEMA = 'base' AND NON_UNIQUE = 0) b
WHERE NOT EXISTS (SELECT 1 FROM STATISTICS n
                  WHERE n.TABLE_SCHEMA = 'shop' AND n.TABLE_NAME = b.TABLE_NAME
                    AND n.INDEX_NAME = b.INDEX_NAME AND n.NON_UNIQUE = 0)
SQL
)
if [ -n "$found" ]; then
  echo "$found" | sed 's/^/   /'
  if [ "${ALLOW_DESTRUCTIVE:-}" = 1 ]; then
    echo "   (ALLOW_DESTRUCTIVE=1 なので、壊しうる変更を通す)"
  else
    echo "FAIL: データを壊しうる変更がある。決めたなら ALLOW_DESTRUCTIVE=1" >&2
    exit 1
  fi
fi

echo "== 4. $BASE の代表データの上で schema.sql が当たる"
for f in $(git -C "$ROOT" ls-tree --name-only "$BASE" fixtures/ | sort); do
  git -C "$ROOT" show "$BASE:$f" | sql base || { echo "FAIL: $BASE の $f が $BASE のスキーマに入らない" >&2; exit 1; }
done
if ! atlas schema apply --url "$BASE_URL" --to file:///s/schema.sql --dev-url "$DEV" --auto-approve >"$TMP/apply.log" 2>&1; then
  sed 's/^/   /' "$TMP/apply.log"
  echo "FAIL: $BASE のデータの上で schema.sql が当たらない" >&2
  exit 1
fi
echo "ok"
