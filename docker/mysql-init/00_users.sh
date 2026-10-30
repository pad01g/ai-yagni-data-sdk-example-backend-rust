#!/bin/sh
# 手元の compose の MySQL の初期化（最初の起動のときだけ走る）。
# - atlas_dev: Atlas が差分を計算するための空のデータベース（--dev-url）。
# - proxy: backend-proxy が使うユーザー。パスワードは .env（make env が作る）から受け取る。
#   権限は表と列の単位で、表ができた後に seed が submodules/proxy/grants.sql を当てる（表がないと付けられない）。
set -eu
mysql -uroot -p"$MYSQL_ROOT_PASSWORD" <<SQL
CREATE DATABASE atlas_dev;
CREATE USER 'proxy'@'%' IDENTIFIED BY '$PROXY_DB_PASSWORD';
SQL
