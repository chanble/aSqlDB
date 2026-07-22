use asql_core::db_manager::DbManager;
use asql_core::persistence::ConnectionConfig;
use asql_core::result::DbSuccessResult;
use asql_core::test_utils;
use serial_test::serial;

fn get_test_sqlite_url() -> String {
    "sqlite::memory:".to_string()
}

#[serial]
#[tokio::test]
async fn test_add_and_list_connection() {
    let manager = DbManager::new();
    let url = get_test_sqlite_url();
    let conn_name = "test_mysql_db";

    let result = manager
        .add_connection(ConnectionConfig::from_url("test_db".into(), &url))
        .await;
    assert!(result.is_ok());
    let result = manager
        .add_connection(ConnectionConfig::from_url(conn_name.into(), &test_utils::mysql_base_url()))
        .await;
    assert!(result.is_ok());

    let connections = manager.list_connections().await;
    assert_eq!(connections.len(), 2);

    let connection_url = manager.get_connection_url(conn_name).await;
    assert!(connection_url.is_some());
    assert_eq!(connection_url, Some(test_utils::mysql_base_url()));
}

#[serial]
#[tokio::test]
async fn test_remove_connection() {
    let manager = DbManager::new();
    let url = get_test_sqlite_url();

    manager
        .add_connection(ConnectionConfig::from_url("temp_db".into(), &url))
        .await
        .unwrap();

    assert_eq!(manager.list_connections().await.len(), 1);

    let removed = manager.remove_connection("temp_db").await;
    assert!(removed);

    assert_eq!(manager.list_connections().await.len(), 0);

    let removed_again = manager.remove_connection("temp_db").await;
    assert!(!removed_again);
}

#[serial]
#[tokio::test]
async fn test_invalid_connection() {
    let manager = DbManager::new();
    let invalid_url = "postgres://user:pass@127.0.0.1:9999/nonexistent_db";

    manager
        .add_connection(ConnectionConfig::from_url("bad_db".into(), invalid_url))
        .await
        .unwrap();

    let result = manager.open_connection("bad_db").await;
    assert!(result.is_err());
}

#[serial]
#[tokio::test]
async fn test_mysql_use_db() {
    let manager = DbManager::new();
    let url = test_utils::mysql_base_url();
    let url_name = "test_db";
    let db_name = "asql_test_mysql_test1";
    let db_name2 = "asql_test_mysql_test2";
    let _ = manager
        .add_connection(ConnectionConfig::from_url(url_name.into(), &url))
        .await;
    let _ = manager
        .execute_sql(
            url_name,
            &format!("CREATE DATABASE IF NOT EXISTS `{db_name}` COLLATE utf8mb4_general_ci"),
        )
        .await;
    let _ = manager
        .execute_sql(
            url_name,
            &format!("CREATE DATABASE IF NOT EXISTS `{db_name2}` COLLATE utf8mb4_general_ci"),
        )
        .await;
    let _ = manager
        .execute_sql(url_name, &format!("USE `{db_name}`"))
        .await;
    let _ = manager
        .execute_sql(url_name, &format!("USE `{db_name2}`"))
        .await;
    let _ = manager
        .execute_sql(url_name, &format!("USE `{db_name}`"))
        .await;
    let select_db1 = manager.execute_sql(url_name, "SELECT DATABASE();").await;
    assert!(select_db1.is_ok());
    match select_db1.unwrap() {
        DbSuccessResult::Select(data) => {
            assert_eq!(data.data.rows.len(), 1);
            assert_eq!(
                data.data.rows[0].get("DATABASE()"),
                Some(&serde_json::json!(db_name))
            );
        }
        _ => panic!("Expected Select result"),
    }
    let _ = manager
        .execute_sql(
            url_name,
            &format!("drop database `{db_name}`;drop database `{db_name2}`;"),
        )
        .await;
}

#[serial]
#[tokio::test]
async fn test_mysql_use_db_batch() {
    let manager = DbManager::new();
    let url = test_utils::mysql_base_url();
    let url_name = "test_db_batch";
    let db_name = "asql_test_mysql_batch1";
    let _ = manager
        .add_connection(ConnectionConfig::from_url(url_name.into(), &url))
        .await;
    let _ = manager
        .execute_sql(
            url_name,
            &format!("CREATE DATABASE IF NOT EXISTS `{db_name}` COLLATE utf8mb4_general_ci"),
        )
        .await;
    let results = manager
        .execute_sql_batch(url_name, &format!("USE `{db_name}`;SELECT DATABASE();"), false)
        .await;
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    match &results[1] {
        Ok(DbSuccessResult::Select(data)) => {
            assert_eq!(data.data.rows.len(), 1);
            assert_eq!(
                data.data.rows[0].get("DATABASE()"),
                Some(&serde_json::json!(db_name))
            );
        }
        _ => panic!("Expected Select result for second statement"),
    }
    let _ = manager
        .execute_sql(url_name, &format!("DROP DATABASE IF EXISTS `{db_name}`"))
        .await;
}

#[serial]
#[tokio::test]
async fn test_pg_use_schema() {
    let manager = DbManager::new();
    let url = test_utils::pg_base_url();
    let url_name = "test_pg";
    let schema_name = "asql_test_pg_schema1";

    let _ = manager
        .add_connection(ConnectionConfig::from_url(url_name.into(), &url))
        .await;
    let _ = manager
        .execute_sql(
            url_name,
            &format!("DROP SCHEMA IF EXISTS \"{schema_name}\" CASCADE"),
        )
        .await;
    let _ = manager
        .execute_sql(
            url_name,
            &format!("CREATE SCHEMA \"{schema_name}\""),
        )
        .await;
    let _ = manager
        .execute_sql(
            url_name,
            &format!("SET search_path TO \"{schema_name}\""),
        )
        .await;
    let result = manager
        .execute_sql(url_name, "SELECT current_schema();")
        .await;
    assert!(result.is_ok());
    match result.unwrap() {
        DbSuccessResult::Select(data) => {
            assert_eq!(data.data.rows.len(), 1);
            assert_eq!(
                data.data.rows[0].get("current_schema"),
                Some(&serde_json::json!(schema_name))
            );
        }
        _ => panic!("Expected Select result"),
    }
    let _ = manager
        .execute_sql(
            url_name,
            &format!("DROP SCHEMA IF EXISTS \"{schema_name}\" CASCADE"),
        )
        .await;
}

#[serial]
#[tokio::test]
async fn test_pg_batch() {
    let manager = DbManager::new();
    let url = test_utils::pg_base_url();
    let url_name = "test_pg_batch";
    let table_name = "asql_test_pg_table";

    let _ = manager
        .add_connection(ConnectionConfig::from_url(url_name.into(), &url))
        .await;
    let _ = manager
        .execute_sql(
            url_name,
            &format!("DROP TABLE IF EXISTS \"{table_name}\""),
        )
        .await;
    let results = manager
        .execute_sql_batch(
            url_name,
            &format!(
                "CREATE TABLE \"{table_name}\" (id INT);\
                 INSERT INTO \"{table_name}\" VALUES (42);\
                 SELECT * FROM \"{table_name}\";"
            ),
            true,
        )
        .await;
    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok(), "CREATE TABLE failed");
    assert!(results[1].is_ok(), "INSERT failed");
    match &results[2] {
        Ok(DbSuccessResult::Select(data)) => {
            assert_eq!(data.data.rows.len(), 1);
            assert_eq!(
                data.data.rows[0].get("id"),
                Some(&serde_json::json!(42))
            );
        }
        _ => panic!("Expected Select result for third statement"),
    }
    let _ = manager
        .execute_sql(
            url_name,
            &format!("DROP TABLE IF EXISTS \"{table_name}\""),
        )
        .await;
}
