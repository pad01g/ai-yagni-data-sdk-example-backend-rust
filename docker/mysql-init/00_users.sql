-- 手元の compose の MySQL の初期化。
-- atlas_dev: Atlas が差分を計算するための空のデータベース（--dev-url）。
-- proxy: backend-proxy が使うユーザー。行の読み書きだけを許す（DELETE と DDL はできない）。
--        スキーマを変えるのは Atlas（SE の作業）だけ。
CREATE DATABASE atlas_dev;
CREATE USER 'proxy'@'%' IDENTIFIED BY 'proxy';
GRANT SELECT, INSERT, UPDATE ON shop.* TO 'proxy'@'%';
