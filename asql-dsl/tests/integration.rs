use asql_dsl::*;
use asql_types::{ColumnType, IntType, StringType};

fn mysql() -> MySql {
    MySql
}

fn pg() -> PostgreSql {
    PostgreSql
}

fn sqlite() -> Sqlite {
    Sqlite
}

// ── Dialect ─────────────────────────────────────────────────────────────

#[test]
fn test_dialect_name() {
    assert_eq!(mysql().name(), "MySQL");
    assert_eq!(pg().name(), "PostgreSQL");
    assert_eq!(sqlite().name(), "SQLite");
}

#[test]
fn test_quote_ident() {
    assert_eq!(mysql().quote_ident("user"), "`user`");
    assert_eq!(pg().quote_ident("user"), r#""user""#);
    assert_eq!(sqlite().quote_ident("user"), r#""user""#);
}

#[test]
fn test_quote_str() {
    assert_eq!(mysql().quote_str("hello"), "'hello'");
    assert_eq!(mysql().quote_str("it's"), "'it''s'");
}

#[test]
fn test_auto_increment() {
    assert_eq!(mysql().auto_increment(), "AUTO_INCREMENT");
    assert_eq!(pg().auto_increment(), "GENERATED ALWAYS AS IDENTITY");
    assert_eq!(sqlite().auto_increment(), "AUTOINCREMENT");
}

#[test]
fn test_limit() {
    assert_eq!(mysql().limit(10), "LIMIT 10");
    assert_eq!(mysql().limit_offset(10, 5), "LIMIT 10 OFFSET 5");
}

#[test]
fn test_supports_delete_limit() {
    assert!(mysql().supports_delete_limit());
    assert!(!pg().supports_delete_limit());
    assert!(sqlite().supports_delete_limit());
}

// ── DQL: SelectBuilder ──────────────────────────────────────────────────

#[test]
fn test_select_basic() {
    let sql = SelectBuilder::new()
        .column("id")
        .column("name")
        .from("users")
        .build(&mysql());
    assert_eq!(sql, "SELECT `id`, `name` FROM `users`");
}

#[test]
fn test_select_all() {
    let sql = SelectBuilder::new()
        .from("users")
        .build(&mysql());
    assert_eq!(sql, "SELECT * FROM `users`");
}

#[test]
fn test_select_with_where() {
    let sql = SelectBuilder::new()
        .from("users")
        .and_where("age", ">", "18")
        .build(&mysql());
    assert_eq!(sql, "SELECT * FROM `users` WHERE `age` > '18'");
}

#[test]
fn test_select_with_order_limit_offset() {
    let sql = SelectBuilder::new()
        .column("id")
        .from("users")
        .order_by("created_at", true)
        .limit(10)
        .offset(5)
        .build(&mysql());
    assert_eq!(
        sql,
        "SELECT `id` FROM `users` ORDER BY `created_at` DESC LIMIT 10 OFFSET 5"
    );
}

#[test]
fn test_select_build_count() {
    let sql = SelectBuilder::new()
        .from("users")
        .and_where("status", "=", "active")
        .build_count(&mysql());
    assert_eq!(sql, "SELECT COUNT(*) as cnt FROM `users` WHERE `status` = 'active'");
}

#[test]
fn test_select_column_with_func() {
    let sql = SelectBuilder::new()
        .column_with_func("COUNT", "id")
        .from("users")
        .build(&mysql());
    assert_eq!(sql, "SELECT COUNT(`id`) FROM `users`");
}

#[test]
fn test_select_complex_where() {
    let sql = SelectBuilder::new()
        .from("users")
        .and_where("status", "=", "active")
        .and_group(|w| w.and("age", ">", "18").and("age", "<", "65"))
        .build(&mysql());
    assert_eq!(
        sql,
        "SELECT * FROM `users` WHERE `status` = 'active' AND (`age` > '18' AND `age` < '65')"
    );
}

// ── DQL: WhereBuilder ───────────────────────────────────────────────────

#[test]
fn test_where_or_group() {
    let w = WhereBuilder::new()
        .and("role", "=", "admin")
        .or_group(|g| g.and("role", "=", "editor").and("status", "=", "active"));
    let sql = w.build(&mysql());
    assert_eq!(sql, "`role` = 'admin' OR (`role` = 'editor' AND `status` = 'active')");
}

#[test]
fn test_where_like() {
    let sql = WhereBuilder::new()
        .and("name", "LIKE", "john")
        .build(&mysql());
    assert_eq!(sql, "`name` LIKE '%john%'");
}

#[test]
fn test_where_is_null() {
    let sql = WhereBuilder::new()
        .and("deleted_at", "IS NULL", "NULL")
        .build(&mysql());
    assert_eq!(sql, "`deleted_at` IS NULL");
}

// ── DML: InsertBuilder ──────────────────────────────────────────────────

#[test]
fn test_insert_basic() {
    let sql = InsertBuilder::new()
        .into("users")
        .column("name")
        .column("email")
        .row(vec!["Alice", "alice@test.com"])
        .build(&mysql());
    assert_eq!(
        sql,
        "INSERT INTO `users` (`name`, `email`) VALUES ('Alice', 'alice@test.com')"
    );
}

#[test]
fn test_insert_multi_row() {
    let sql = InsertBuilder::new()
        .into("users")
        .column("name")
        .row(vec!["Alice"])
        .row(vec!["Bob"])
        .build(&pg());
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name") VALUES ('Alice'), ('Bob')"#
    );
}

#[test]
fn test_insert_with_null_and_now() {
    let sql = InsertBuilder::new()
        .into("logs")
        .column("event")
        .column("created_at")
        .row(vec!["start", "NOW()"])
        .row(vec!["NULL", "NOW()"])
        .build(&mysql());
    assert_eq!(
        sql,
        "INSERT INTO `logs` (`event`, `created_at`) VALUES ('start', NOW()), (NULL, NOW())"
    );
}

#[test]
fn test_insert_export() {
    let sql = InsertBuilder::new()
        .into("users")
        .column("name")
        .build_export(&mysql(), &[vec!["Alice".into()], vec!["Bob".into()]]);
    assert_eq!(
        sql,
        vec![
            "INSERT INTO `users` (`name`) VALUES ('Alice');",
            "INSERT INTO `users` (`name`) VALUES ('Bob');",
        ]
    );
}

// ── DML: UpdateBuilder ──────────────────────────────────────────────────

#[test]
fn test_update_basic() {
    let sql = UpdateBuilder::new()
        .table("users")
        .set("status", "active")
        .where_("id", "=", "1")
        .build(&mysql());
    assert_eq!(sql, "UPDATE `users` SET `status` = 'active' WHERE `id` = '1'");
}

#[test]
fn test_update_limit() {
    let sql = UpdateBuilder::new()
        .table("users")
        .set("status", "inactive")
        .limit(100)
        .build(&mysql());
    assert_eq!(sql, "UPDATE `users` SET `status` = 'inactive' LIMIT 100");
}

#[test]
fn test_update_pg_no_limit() {
    let sql = UpdateBuilder::new()
        .table("users")
        .set("status", "inactive")
        .limit(100)
        .build(&pg());
    assert_eq!(sql, r#"UPDATE "users" SET "status" = 'inactive'"#);
}

// ── DML: DeleteBuilder ──────────────────────────────────────────────────

#[test]
fn test_delete_basic() {
    let sql = DeleteBuilder::new()
        .from("users")
        .where_("id", "=", "1")
        .build(&mysql());
    assert_eq!(sql, "DELETE FROM `users` WHERE `id` = '1'");
}

#[test]
fn test_delete_all() {
    let sql = DeleteBuilder::new()
        .from("logs")
        .build(&mysql());
    assert_eq!(sql, "DELETE FROM `logs`");
}

#[test]
fn test_delete_with_group() {
    let sql = DeleteBuilder::new()
        .from("orders")
        .where_("status", "=", "cancelled")
        .or_group(|g| g.and("expires_at", "<", "NOW()"))
        .build(&mysql());
    assert_eq!(
        sql,
        "DELETE FROM `orders` WHERE `status` = 'cancelled' OR (`expires_at` < 'NOW()')"
    );
}

// ── DDL: CreateTableBuilder ─────────────────────────────────────────────

#[test]
fn test_create_table_basic() {
    let sql = CreateTableBuilder::new()
        .table("users")
        .column(ColumnDef {
            name: "id".into(),
            col_type: ColumnType::Int(IntType { display_width: None, unsigned: false, zerofill: false }),
            nullable: Some(false),
            default_value: None,
            comment: None,
            collation: None,
            extra: asql_types::ColumnExtra { auto_increment: true, on_update: false },
            key: None,
        })
        .column(ColumnDef {
            name: "name".into(),
            col_type: ColumnType::Varchar(StringType { length: Some(255) }),
            nullable: Some(false),
            default_value: None,
            comment: None,
            collation: None,
            extra: asql_types::ColumnExtra { auto_increment: false, on_update: false },
            key: None,
        })
        .build(&mysql());
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("`id` INT NOT NULL AUTO_INCREMENT"));
    assert!(sql.contains("`name` VARCHAR(255) NOT NULL"));
    assert!(sql.contains("PRIMARY KEY (`id`)"));
}

#[test]
fn test_create_table_with_engine() {
    let sql = CreateTableBuilder::new()
        .table("users")
        .column(ColumnDef {
            name: "id".into(),
            col_type: ColumnType::Int(IntType { display_width: None, unsigned: false, zerofill: false }),
            nullable: Some(false),
            default_value: None,
            comment: None,
            collation: None,
            extra: asql_types::ColumnExtra { auto_increment: false, on_update: false },
            key: None,
        })
        .engine("InnoDB")
        .collation("utf8mb4_general_ci")
        .build(&mysql());
    assert!(sql.ends_with(" ENGINE=InnoDB COLLATE=utf8mb4_general_ci"));
}

#[test]
fn test_create_table_pg() {
    let sql = CreateTableBuilder::new()
        .table("users")
        .column(ColumnDef {
            name: "id".into(),
            col_type: ColumnType::Int(IntType { display_width: None, unsigned: false, zerofill: false }),
            nullable: Some(false),
            default_value: None,
            comment: None,
            collation: None,
            extra: asql_types::ColumnExtra { auto_increment: true, on_update: false },
            key: None,
        })
        .column(ColumnDef {
            name: "name".into(),
            col_type: ColumnType::Varchar(StringType { length: Some(255) }),
            nullable: Some(false),
            default_value: None,
            comment: None,
            collation: None,
            extra: asql_types::ColumnExtra { auto_increment: false, on_update: false },
            key: None,
        })
        .build(&pg());
    assert!(sql.contains(r#""id" INT NOT NULL GENERATED ALWAYS AS IDENTITY"#));
}

// ── DDL: AlterTableBuilder ──────────────────────────────────────────────

#[test]
fn test_alter_add_column() {
    let sql = AlterTableBuilder::new()
        .table("users")
        .add_column(ColumnDef {
            name: "email".into(),
            col_type: ColumnType::Varchar(StringType { length: Some(255) }),
            nullable: Some(true),
            default_value: None,
            comment: None,
            collation: None,
            extra: asql_types::ColumnExtra { auto_increment: false, on_update: false },
            key: None,
        })
        .build(&mysql());
    assert_eq!(
        sql,
        vec!["ALTER TABLE `users` ADD COLUMN `email` VARCHAR(255)"]
    );
}

#[test]
fn test_alter_modify_column() {
    let sql = AlterTableBuilder::new()
        .table("users")
        .modify_column(ColumnDef {
            name: "name".into(),
            col_type: ColumnType::Varchar(StringType { length: Some(100) }),
            nullable: Some(false),
            default_value: None,
            comment: None,
            collation: None,
            extra: asql_types::ColumnExtra { auto_increment: false, on_update: false },
            key: None,
        })
        .build(&mysql());
    assert_eq!(
        sql,
        vec!["ALTER TABLE `users` MODIFY COLUMN `name` VARCHAR(100) NOT NULL"]
    );
}

#[test]
fn test_alter_add_drop_index() {
    let sql = AlterTableBuilder::new()
        .table("users")
        .add_index("idx_name", IndexType::Index, vec![("name", None)])
        .build(&mysql());
    assert_eq!(
        sql,
        vec!["ALTER TABLE `users` ADD INDEX `idx_name` (`name`)"]
    );
}

#[test]
fn test_alter_drop_index() {
    let sql = AlterTableBuilder::new()
        .table("users")
        .drop_index("idx_name")
        .build(&mysql());
    assert_eq!(sql, vec!["ALTER TABLE `users` DROP INDEX `idx_name`"]);
}

#[test]
fn test_alter_rename_table() {
    let sql = AlterTableBuilder::new()
        .table("users")
        .rename_table("customers")
        .build(&mysql());
    assert_eq!(sql, vec!["RENAME TABLE `users` TO `customers`"]);
}

// ── DDL: DropBuilder ────────────────────────────────────────────────────

#[test]
fn test_drop_table() {
    let sql = DropBuilder::new(DropTarget::Table)
        .name("users")
        .build(&mysql());
    assert_eq!(sql, "DROP TABLE `users`");
}

#[test]
fn test_drop_table_if_exists() {
    let sql = DropBuilder::new(DropTarget::Table)
        .name("users")
        .if_exists()
        .build(&mysql());
    assert_eq!(sql, "DROP TABLE IF EXISTS `users`");
}

#[test]
fn test_truncate() {
    let sql = DropBuilder::new(DropTarget::Table)
        .name("logs")
        .build_truncate(&mysql());
    assert_eq!(sql, "TRUNCATE TABLE `logs`");
}

#[test]
fn test_truncate_pg() {
    let sql = DropBuilder::new(DropTarget::Table).name("logs").build_truncate(&pg());
    assert_eq!(sql, "TRUNCATE TABLE \"logs\"");
}

#[test]
fn test_truncate_sqlite() {
    let sql = DropBuilder::new(DropTarget::Table).name("logs").build_truncate(&sqlite());
    assert_eq!(sql, "DELETE FROM \"logs\"");
}

#[test]
fn test_drop_database_mysql() {
    let sql = DropBuilder::new(DropTarget::Database).name("mydb").build(&mysql());
    assert_eq!(sql, "DROP DATABASE `mydb`");
}

#[test]
fn test_drop_database_pg() {
    let sql = DropBuilder::new(DropTarget::Database).name("mydb").if_exists().build(&pg());
    assert_eq!(sql, r#"DROP DATABASE IF EXISTS "mydb""#);
}

// ── DDL: DatabaseBuilder ────────────────────────────────────────────────

#[test]
fn test_create_database() {
    let sql = DatabaseBuilder::create("mydb")
        .if_not_exists()
        .character_set("utf8mb4")
        .collation("utf8mb4_general_ci")
        .build(&mysql());
    assert!(sql.starts_with("CREATE DATABASE IF NOT EXISTS `mydb`"));
    assert!(sql.contains("DEFAULT CHARACTER SET utf8mb4"));
    assert!(sql.contains("DEFAULT COLLATE utf8mb4_general_ci"));
}

#[test]
fn test_create_database_pg() {
    let sql = DatabaseBuilder::create("mydb")
        .character_set("UTF8")
        .build(&pg());
    assert!(sql.starts_with(r#"CREATE DATABASE "mydb""#));
    assert!(sql.contains("ENCODING 'UTF8'"));
}

// ── DDL: IndexBuilder ───────────────────────────────────────────────────

#[test]
fn test_create_index_basic() {
    let sql = IndexBuilder::new()
        .on("users")
        .name("idx_name")
        .column("name", None)
        .build(&mysql());
    assert_eq!(sql, "CREATE INDEX `idx_name` ON `users` (`name`)");
}

#[test]
fn test_create_index_with_method() {
    let sql = IndexBuilder::new()
        .on("users")
        .name("idx_email")
        .index_type(IndexType::Unique)
        .column("email", None)
        .using(IndexMethod::Hash)
        .build(&pg());
    assert_eq!(
        sql,
        r#"CREATE UNIQUE INDEX "idx_email" ON "users" ("email") USING HASH"#
    );
}

#[test]
fn test_drop_index() {
    let sql = IndexBuilder::new()
        .on("users")
        .name("idx_name")
        .build_drop(&mysql());
    assert_eq!(sql, "DROP INDEX `idx_name` ON `users`");
}

#[test]
fn test_drop_index_via_dropbuilder_mysql() {
    let sql = DropBuilder::new(DropTarget::Index)
        .name("idx_name")
        .on("users")
        .build(&mysql());
    assert_eq!(sql, "DROP INDEX `idx_name` ON `users`");
}

#[test]
fn test_drop_index_via_dropbuilder_pg() {
    let sql = DropBuilder::new(DropTarget::Index)
        .name("idx_name")
        .on("users")
        .if_exists()
        .build(&pg());
    assert_eq!(sql, r#"DROP INDEX IF EXISTS "idx_name""#);
}

// ── DDL: UserBuilder ────────────────────────────────────────────────────

#[test]
fn test_create_user_mysql() {
    let sql = UserBuilder::new()
        .create_user("app_user")
        .host("localhost")
        .identified_by("secret")
        .build(&mysql());
    assert_eq!(
        sql,
        "CREATE USER 'app_user'@'localhost' IDENTIFIED BY 'secret'"
    );
}

#[test]
fn test_create_user_pg() {
    let sql = UserBuilder::new()
        .create_user("app_user")
        .identified_by("secret")
        .build(&pg());
    assert_eq!(
        sql,
        r#"CREATE USER "app_user" WITH PASSWORD 'secret'"#
    );
}

#[test]
fn test_drop_user() {
    let sql = UserBuilder::new()
        .drop_user("app_user")
        .host("localhost")
        .build(&mysql());
    assert_eq!(sql, "DROP USER 'app_user'@'localhost'");
}

#[test]
fn test_rename_user_mysql() {
    let sql = UserBuilder::new()
        .rename_user("old_user", "new_user")
        .host("localhost")
        .build(&mysql());
    assert_eq!(
        sql,
        "RENAME USER 'old_user'@'localhost' TO 'new_user'@'%'"
    );
}

// ── DDL: GrantBuilder ───────────────────────────────────────────────────

#[test]
fn test_grant_basic() {
    let sql = GrantBuilder::new()
        .grant(vec!["SELECT", "INSERT"])
        .on("mydb.*")
        .to("app_user")
        .host("localhost")
        .build(&mysql());
    assert_eq!(
        sql,
        "GRANT SELECT, INSERT ON `mydb`.* TO 'app_user'@'localhost'"
    );
}

#[test]
fn test_grant_with_option() {
    let sql = GrantBuilder::new()
        .grant(vec!["ALL PRIVILEGES"])
        .on("*.*")
        .to("admin")
        .with_grant_option()
        .build(&mysql());
    assert_eq!(
        sql,
        "GRANT ALL PRIVILEGES ON *.* TO 'admin'@'%' WITH GRANT OPTION"
    );
}

#[test]
fn test_revoke() {
    let sql = GrantBuilder::new()
        .revoke(vec!["DELETE"])
        .on("mydb.*")
        .from("app_user")
        .build(&mysql());
    assert_eq!(
        sql,
        "REVOKE DELETE ON `mydb`.* FROM 'app_user'@'%'"
    );
}

#[test]
fn test_grant_role_super_admin() {
    let sql = GrantBuilder::new()
        .role(GrantRole::SuperAdmin)
        .to("root")
        .build(&mysql());
    assert_eq!(
        sql,
        "GRANT ALL PRIVILEGES ON *.* TO 'root'@'%' WITH GRANT OPTION"
    );
}

#[test]
fn test_grant_role_read_write() {
    let sql = GrantBuilder::new()
        .role(GrantRole::ReadWrite)
        .on("mydb.*")
        .to("app_user")
        .build(&mysql());
    assert_eq!(
        sql,
        "GRANT SELECT, INSERT, UPDATE, DELETE, EXECUTE ON `mydb`.* TO 'app_user'@'%'"
    );
}

#[test]
fn test_grant_role_read_only() {
    let sql = GrantBuilder::new()
        .role(GrantRole::ReadOnly)
        .on("mydb.*")
        .to("reader")
        .build(&mysql());
    assert_eq!(
        sql,
        "GRANT SELECT, SHOW VIEW ON `mydb`.* TO 'reader'@'%'"
    );
}

#[test]
fn test_grant_role_ddl() {
    let sql = GrantBuilder::new()
        .role(GrantRole::DDL)
        .on("mydb.*")
        .to("migrator")
        .build(&mysql());
    assert_eq!(
        sql,
        "GRANT CREATE, ALTER, DROP, INDEX ON `mydb`.* TO 'migrator'@'%'"
    );
}

#[test]
fn test_grant_role_dba() {
    let sql = GrantBuilder::new()
        .role(GrantRole::DBA)
        .on("mydb.*")
        .to("db_admin")
        .build(&mysql());
    assert_eq!(
        sql,
        "GRANT ALL PRIVILEGES ON `mydb`.* TO 'db_admin'@'%' WITH GRANT OPTION"
    );
}

#[test]
fn test_grant_pg() {
    let sql = GrantBuilder::new()
        .grant(vec!["SELECT", "INSERT"])
        .on("mytable")
        .to("app_user")
        .build(&pg());
    assert_eq!(
        sql,
        r#"GRANT SELECT, INSERT ON "mytable" TO "app_user""#
    );
}

// ── Introspection: Users ────────────────────────────────────────────────

#[test]
fn test_list_users_mysql() {
    let sql = UsersIntrospection::list_users(&mysql());
    assert_eq!(
        sql,
        "SELECT User, Host, account_locked, password_expired FROM mysql.user ORDER BY User"
    );
}

#[test]
fn test_list_users_pg() {
    let sql = UsersIntrospection::list_users(&pg());
    assert!(sql.contains("pg_roles"));
    assert!(sql.contains("rolcanlogin = true"));
}

#[test]
fn test_user_info_mysql() {
    let sql = UsersIntrospection::user_info(&mysql(), "app_user", Some("localhost"));
    assert_eq!(sql, "SHOW CREATE USER 'app_user'@'localhost'");
}

#[test]
fn test_user_info_pg() {
    let sql = UsersIntrospection::user_info(&pg(), "app_user", None);
    assert!(sql.contains("FROM pg_roles WHERE rolname = 'app_user'"));
}

#[test]
fn test_list_users_sqlite() {
    let sql = UsersIntrospection::list_users(&sqlite());
    assert_eq!(sql, "");
}

// ── Introspection: Server ───────────────────────────────────────────────

#[test]
fn test_process_list_mysql() {
    let sql = ServerIntrospection::process_list(&mysql());
    assert_eq!(sql, "SHOW FULL PROCESSLIST");
}

#[test]
fn test_process_list_pg() {
    let sql = ServerIntrospection::process_list(&pg());
    assert!(sql.contains("pg_stat_activity"));
}

#[test]
fn test_variables_mysql() {
    let sql = ServerIntrospection::variables(&mysql());
    assert_eq!(sql, "SHOW VARIABLES");
}

#[test]
fn test_variables_pg() {
    let sql = ServerIntrospection::variables(&pg());
    assert!(sql.contains("pg_settings"));
}

#[test]
fn test_status_mysql() {
    let sql = ServerIntrospection::status(&mysql());
    assert_eq!(sql, "SHOW GLOBAL STATUS");
}

#[test]
fn test_status_pg() {
    let sql = ServerIntrospection::status(&pg());
    assert!(sql.contains("pg_stat_database"));
}

#[test]
fn test_version_mysql() {
    let sql = ServerIntrospection::version(&mysql());
    assert_eq!(sql, "SELECT VERSION() as version");
}

#[test]
fn test_version_pg() {
    let sql = ServerIntrospection::version(&pg());
    assert_eq!(sql, "SELECT version() as version");
}

#[test]
fn test_version_sqlite() {
    let sql = ServerIntrospection::version(&sqlite());
    assert_eq!(sql, "SELECT sqlite_version() as version");
}

#[test]
fn test_kill_process_mysql() {
    let sql = ServerIntrospection::kill_process(&mysql(), &["123", "456"]);
    assert_eq!(sql, "KILL CONNECTION 123;\nKILL CONNECTION 456");
}

#[test]
fn test_kill_process_pg() {
    let sql = ServerIntrospection::kill_process(&pg(), &["123", "456"]);
    assert_eq!(
        sql,
        "SELECT count(pg_terminate_backend(pids.pid)) FROM unnest(ARRAY[123, 456]) AS pids(pid)"
    );
}

#[test]
fn test_kill_process_empty() {
    let sql = ServerIntrospection::kill_process(&mysql(), &[]);
    assert_eq!(sql, "");
}

// ── Introspection: Databases ────────────────────────────────────────────

#[test]
fn test_list_databases_mysql() {
    let sql = DatabasesIntrospection::list_databases(&mysql());
    assert!(sql.contains("information_schema.SCHEMATA"));
}

#[test]
fn test_list_databases_pg() {
    let sql = DatabasesIntrospection::list_databases(&pg());
    assert!(sql.contains("pg_database"));
    assert!(sql.contains("datistemplate = false"));
}

#[test]
fn test_use_database() {
    let sql = DatabasesIntrospection::use_database(&mysql(), "mydb");
    assert_eq!(sql, "USE `mydb`");
}

#[test]
fn test_current_database_mysql() {
    let sql = DatabasesIntrospection::current_database(&mysql());
    assert_eq!(sql, "SELECT DATABASE() as db");
}

#[test]
fn test_current_database_pg() {
    let sql = DatabasesIntrospection::current_database(&pg());
    assert_eq!(sql, "SELECT current_database() as db");
}

#[test]
fn test_current_database_sqlite() {
    let sql = DatabasesIntrospection::current_database(&sqlite());
    assert_eq!(sql, "SELECT '' as db");
}

// ── Introspection: Columns ──────────────────────────────────────────────

#[test]
fn test_show_columns_mysql() {
    let sql = ColumnsIntrospection::show_columns(&mysql(), "users", None);
    assert_eq!(sql, "SHOW FULL COLUMNS FROM `users`");
}

#[test]
fn test_show_columns_mysql_with_db() {
    let sql = ColumnsIntrospection::show_columns(&mysql(), "users", Some("mydb"));
    assert_eq!(sql, "SHOW FULL COLUMNS FROM `mydb`.`users`");
}

#[test]
fn test_show_columns_pg() {
    let sql = ColumnsIntrospection::show_columns(&pg(), "users", None);
    assert!(sql.contains("information_schema.columns"));
    assert!(sql.contains("c.table_schema = 'public'"));
    assert!(sql.contains("c.table_name = 'users'"));
}

#[test]
fn test_show_create_table_pg() {
    let sql = ColumnsIntrospection::show_create_table(&pg(), "users");
    assert!(sql.contains("information_schema.columns"));
    assert!(sql.contains("table_name = 'users'"));
}

// ── Introspection: Tables ───────────────────────────────────────────────

#[test]
fn test_list_tables_mysql() {
    let sql = TablesIntrospection::list_tables(&mysql(), None, None, TableNameMatch::Contains);
    assert!(sql.contains("information_schema.tables"));
    assert!(sql.contains("TABLE_NAME"));
    assert!(sql.contains("DATA_LENGTH"));
    assert!(sql.contains("INDEX_LENGTH"));
    assert!(sql.contains("DATA_FREE"));
    assert!(sql.contains("AUTO_INCREMENT"));
    assert!(sql.contains("table_schema = DATABASE()"));
}

#[test]
fn test_table_info_mysql() {
    let sql = TablesIntrospection::table_info(&mysql(), Some("mydb"), "users");
    assert!(sql.contains("TABLE_COMMENT"));
    assert!(sql.contains("DATA_LENGTH"));
    assert!(sql.contains("INDEX_LENGTH"));
    assert!(sql.contains("table_schema = 'mydb'"));
    assert!(sql.contains("TABLE_NAME = 'users'"));
}

#[test]
fn test_list_tables_postgres() {
    let sql = TablesIntrospection::list_tables(&pg(), None, None, TableNameMatch::Contains);
    assert!(sql.contains("DATA_LENGTH"));
    assert!(sql.contains("INDEX_LENGTH"));
    assert!(sql.contains("pg_relation_size"));
    assert!(sql.contains("pg_indexes_size"));
}

#[test]
fn test_list_tables_sqlite() {
    let sql = TablesIntrospection::list_tables(&sqlite(), None, None, TableNameMatch::Contains);
    assert!(sql.contains("DATA_LENGTH"));
    assert!(sql.contains("INDEX_LENGTH"));
    assert!(sql.contains("NULL AS AUTO_INCREMENT"));
}

#[test]
fn test_table_info_pg() {
    let sql = TablesIntrospection::table_info(&pg(), Some("public"), "users");
    assert!(sql.contains("TABLE_COMMENT"));
    assert!(sql.contains("tablename = 'users'"));
    assert!(sql.contains("pg_relation_size"));
    assert!(sql.contains("schemaname = 'public'"));
}

#[test]
fn test_table_count_mysql() {
    let sql = TablesIntrospection::table_count(&mysql(), None);
    assert!(sql.contains("COUNT(*)"));
    assert!(sql.contains("information_schema.tables"));
}

#[test]
fn test_table_count_pg() {
    let sql = TablesIntrospection::table_count(&pg(), None);
    assert!(sql.contains("COUNT(*)"));
    assert!(sql.contains("pg_tables"));
}

#[test]
fn test_table_sizes_mysql() {
    let sql = TablesIntrospection::table_sizes(&mysql(), None);
    assert!(sql.contains("TABLE_NAME"));
    assert!(sql.contains("size_bytes"));
    assert!(sql.contains("information_schema.tables"));
}

#[test]
fn test_table_sizes_pg() {
    let sql = TablesIntrospection::table_sizes(&pg(), None);
    assert!(sql.contains("TABLE_NAME"));
    assert!(sql.contains("size_bytes"));
    assert!(sql.contains("pg_statio_user_tables"));
}

// ── Introspection: Indexes ──────────────────────────────────────────────

#[test]
fn test_list_indexes_mysql() {
    let sql = IndexesIntrospection::list_indexes(&mysql(), "users");
    assert_eq!(sql, "SHOW INDEX FROM `users`");
}

#[test]
fn test_list_indexes_pg() {
    let sql = IndexesIntrospection::list_indexes(&pg(), "users");
    assert!(sql.contains("pg_indexes"));
    assert!(sql.contains("tablename = 'users'"));
}
