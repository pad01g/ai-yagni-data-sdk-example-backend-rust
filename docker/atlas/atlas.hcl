# compose の schema（SE の作業）が使う Atlas の設定。root のパスワードはコマンドの引数に出さず、環境変数から読む。
env "compose" {
  src = "file:///schema/schema.sql"
  url = "mysql://root:${getenv("MYSQL_ROOT_PASSWORD")}@mysql:3306/shop"
  dev = "mysql://root:${getenv("MYSQL_ROOT_PASSWORD")}@mysql:3306/atlas_dev"
}
