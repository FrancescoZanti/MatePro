// MCP SQL Server Module
// Gestione connessioni SQL Server con supporto autenticazione Windows/SQL
// IMPORTANTE: Solo operazioni READ-ONLY (SELECT)

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::{Number, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiberius::{AuthMethod, Client, Config, Query};
use tiberius::{ColumnType, Row};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub type SqlClient = Client<Compat<TcpStream>>;

#[derive(Clone, Debug, Serialize)]
pub struct SqlColumnInfo {
    pub name: String,
    pub data_type: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<SqlColumnInfo>,
    pub rows: Vec<HashMap<String, Value>>,
}

#[derive(Clone)]
pub struct SqlConnection {
    pub connection_id: String,
    pub server: String,
    pub database: String,
    pub auth_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub struct SqlConnectionManager {
    connections: Arc<Mutex<HashMap<String, SqlConnection>>>,
}

impl SqlConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_connection(&self, conn: SqlConnection) {
        let mut conns = self.connections.lock().unwrap();
        conns.insert(conn.connection_id.clone(), conn);
    }

    pub fn remove_connection(&self, conn_id: &str) -> Option<SqlConnection> {
        let mut conns = self.connections.lock().unwrap();
        conns.remove(conn_id)
    }

    pub fn get_connection(&self, conn_id: &str) -> Option<SqlConnection> {
        let conns = self.connections.lock().unwrap();
        conns.get(conn_id).cloned()
    }

    pub fn list_connections(&self) -> Vec<SqlConnection> {
        let conns = self.connections.lock().unwrap();
        conns.values().cloned().collect()
    }
}

impl Default for SqlConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn validate_readonly_query(query: &str) -> Result<()> {
    let query_upper = query.trim().to_uppercase();

    let forbidden_keywords = [
        "UPDATE", "INSERT", "DELETE", "DROP", "CREATE", "ALTER", "TRUNCATE",
        "EXEC", "EXECUTE", "SP_", "XP_", "MERGE", "BULK", "WRITETEXT", "UPDATETEXT",
    ];

    let without_line_comments: String = query_upper
        .lines()
        .map(|line| {
            if let Some(pos) = line.find("--") {
                &line[..pos]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let without_block_comments = remove_block_comments(&without_line_comments);

    for keyword in &forbidden_keywords {
        if without_block_comments.contains(keyword) {
            return Err(anyhow!(
                "Query non permessa: contiene operazione di scrittura '{}'. Solo SELECT è consentito.",
                keyword
            ));
        }
    }

    let trimmed = without_block_comments.trim();
    if !trimmed.starts_with("SELECT")
        && !trimmed.starts_with("WITH")
        && !trimmed.starts_with("DECLARE")
    {
        return Err(anyhow!(
            "Query non permessa: deve iniziare con SELECT, WITH o DECLARE."
        ));
    }

    Ok(())
}

fn remove_block_comments(sql: &str) -> String {
    let mut result = String::new();
    let mut in_comment = false;
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        if !in_comment {
            if c == '/' && chars.peek() == Some(&'*') {
                in_comment = true;
                chars.next();
            } else {
                result.push(c);
            }
        } else if c == '*' && chars.peek() == Some(&'/') {
            in_comment = false;
            chars.next();
        }
    }

    result
}

fn column_type_label(column_type: ColumnType) -> &'static str {
    match column_type {
        ColumnType::Null => "null",
        ColumnType::Bit | ColumnType::Bitn => "bit",
        ColumnType::Int1 | ColumnType::Int2 | ColumnType::Int4 | ColumnType::Int8 | ColumnType::Intn => "int",
        ColumnType::Float4 | ColumnType::Float8 | ColumnType::Floatn => "float",
        ColumnType::Decimaln | ColumnType::Numericn => "decimal",
        ColumnType::Money | ColumnType::Money4 => "money",
        ColumnType::Datetime | ColumnType::Datetime4 => "datetime",
        ColumnType::Datetimen => "datetimen",
        ColumnType::Daten => "date",
        ColumnType::Timen => "time",
        ColumnType::Datetime2 => "datetime2",
        ColumnType::DatetimeOffsetn => "datetimeoffset",
        ColumnType::Guid => "guid",
        ColumnType::BigVarBin | ColumnType::BigBinary => "varbinary",
        ColumnType::BigVarChar | ColumnType::BigChar => "varchar",
        ColumnType::NVarchar | ColumnType::NChar => "nvarchar",
        ColumnType::Xml => "xml",
        ColumnType::Text | ColumnType::NText => "text",
        ColumnType::Image => "image",
        ColumnType::Udt => "udt",
        ColumnType::SSVariant => "sql_variant",
    }
}

fn try_number_from_decimal(decimal: Decimal) -> Option<Number> {
    decimal.to_f64().and_then(Number::from_f64)
}

fn bool_value(row: &Row, idx: usize) -> Result<Option<Value>> {
    Ok(row.try_get::<bool, _>(idx)?.map(Value::Bool))
}

fn int_value(row: &Row, idx: usize) -> Result<Option<Value>> {
    Ok(row.try_get::<i64, _>(idx)?.map(|v| Value::Number(Number::from(v))))
}

fn float_value(row: &Row, idx: usize) -> Result<Option<Value>> {
    Ok(row.try_get::<f64, _>(idx)?.and_then(Number::from_f64).map(Value::Number))
}

fn decimal_value(row: &Row, idx: usize) -> Result<Option<Value>> {
    Ok(row.try_get::<Decimal, _>(idx)?.map(|decimal| {
        try_number_from_decimal(decimal)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(decimal.to_string()))
    }))
}

fn string_value(row: &Row, idx: usize) -> Result<Option<Value>> {
    Ok(row.try_get::<&str, _>(idx)?.map(|text| Value::String(text.to_string())))
}

fn binary_value(row: &Row, idx: usize) -> Result<Option<Value>> {
    Ok(row.try_get::<&[u8], _>(idx)?.map(|bytes| Value::String(general_purpose::STANDARD.encode(bytes))))
}

fn datetime_value(row: &Row, idx: usize) -> Result<Option<Value>> {
    if let Some(dt) = row.try_get::<NaiveDateTime, _>(idx)? {
        return Ok(Some(Value::String(dt.to_string())));
    }
    if let Some(dt) = row.try_get::<NaiveDate, _>(idx)? {
        return Ok(Some(Value::String(dt.to_string())));
    }
    if let Some(dt) = row.try_get::<NaiveTime, _>(idx)? {
        return Ok(Some(Value::String(dt.to_string())));
    }
    if let Some(dt) = row.try_get::<DateTime<FixedOffset>, _>(idx)? {
        return Ok(Some(Value::String(dt.to_rfc3339())));
    }
    Ok(None)
}

fn column_value_to_json(row: &Row, idx: usize, column_type: ColumnType) -> Result<Value> {
    let value = match column_type {
        ColumnType::Null => Value::Null,
        ColumnType::Bit | ColumnType::Bitn => bool_value(row, idx)?.unwrap_or(Value::Null),
        ColumnType::Int1 | ColumnType::Int2 | ColumnType::Int4 | ColumnType::Int8 | ColumnType::Intn => {
            int_value(row, idx)?.unwrap_or(Value::Null)
        }
        ColumnType::Float4 | ColumnType::Float8 | ColumnType::Floatn => {
            float_value(row, idx)?.unwrap_or(Value::Null)
        }
        ColumnType::Decimaln | ColumnType::Numericn | ColumnType::Money | ColumnType::Money4 => {
            decimal_value(row, idx)?.unwrap_or(Value::Null)
        }
        ColumnType::Datetime | ColumnType::Datetime4 | ColumnType::Datetimen | ColumnType::Daten |
        ColumnType::Timen | ColumnType::Datetime2 | ColumnType::DatetimeOffsetn => {
            datetime_value(row, idx)?.unwrap_or(Value::Null)
        }
        ColumnType::Guid => string_value(row, idx)?.unwrap_or(Value::Null),
        ColumnType::BigVarBin | ColumnType::BigBinary | ColumnType::Image => {
            binary_value(row, idx)?.unwrap_or(Value::Null)
        }
        _ => string_value(row, idx)?.unwrap_or(Value::Null),
    };

    Ok(value)
}

#[cfg(windows)]
pub async fn connect_windows_auth(server: &str, database: &str) -> Result<SqlClient> {
    let mut config = Config::new();
    config.host(server);
    config.database(database);
    config.authentication(AuthMethod::Integrated);
    config.trust_cert();

    let tcp = TcpStream::connect(config.get_addr()).await?;
    let client = Client::connect(config, tcp.compat_write()).await?;
    Ok(client)
}

#[cfg(not(windows))]
pub async fn connect_windows_auth(server: &str, database: &str) -> Result<SqlClient> {
    Err(anyhow!(
        "Autenticazione Windows non supportata su questo sistema operativo.\n\
        Su Linux/macOS usa autenticazione SQL (username/password).\n\
        Server: {}, Database: {}",
        server,
        database
    ))
}

pub async fn connect_sql_auth(
    server: &str,
    database: &str,
    username: &str,
    password: &str,
) -> Result<SqlClient> {
    let mut config = Config::new();
    config.host(server);
    config.database(database);
    config.authentication(AuthMethod::sql_server(username, password));
    config.trust_cert();

    let tcp = TcpStream::connect(config.get_addr()).await?;
    let client = Client::connect(config, tcp.compat_write()).await?;
    Ok(client)
}

pub async fn run_query(client: &mut SqlClient, query: &str) -> Result<QueryResult> {
    validate_readonly_query(query)?;

    let mut stream = Query::new(query).query(client).await?;

    let schema: Vec<tiberius::Column> = stream
        .columns()
        .await?
        .map(|columns| columns.to_vec())
        .unwrap_or_default();

    let rows = stream.into_first_result().await?;

    let column_info: Vec<SqlColumnInfo> = schema
        .iter()
        .map(|column| SqlColumnInfo {
            name: column.name().to_string(),
            data_type: column_type_label(column.column_type()).to_string(),
        })
        .collect();

    let mut data_rows = Vec::new();
    for row in rows {
        let mut row_map = HashMap::new();
        for (idx, column) in schema.iter().enumerate() {
            let value = column_value_to_json(&row, idx, column.column_type())?;
            row_map.insert(column.name().to_string(), value);
        }
        data_rows.push(row_map);
    }

    Ok(QueryResult {
        columns: column_info,
        rows: data_rows,
    })
}

pub async fn list_tables(client: &mut SqlClient) -> Result<QueryResult> {
    let query = r#"
        SELECT 
            TABLE_SCHEMA as [Schema],
            TABLE_NAME as [Table],
            TABLE_TYPE as [Type]
        FROM INFORMATION_SCHEMA.TABLES
        WHERE TABLE_TYPE IN ('BASE TABLE', 'VIEW')
        ORDER BY TABLE_SCHEMA, TABLE_NAME
    "#;

    run_query(client, query).await
}

pub async fn describe_table(
    client: &mut SqlClient,
    schema: &str,
    table_name: &str,
) -> Result<QueryResult> {
    let query = format!(
        r#"
        SELECT 
            COLUMN_NAME as [Column],
            DATA_TYPE as [Type],
            CHARACTER_MAXIMUM_LENGTH as [MaxLength],
            IS_NULLABLE as [Nullable],
            COLUMN_DEFAULT as [Default]
        FROM INFORMATION_SCHEMA.COLUMNS
        WHERE TABLE_SCHEMA = '{}'
        AND TABLE_NAME = '{}'
        ORDER BY ORDINAL_POSITION
        "#,
        schema.replace('\'', "''"),
        table_name.replace('\'', "''")
    );

    run_query(client, &query).await
}

pub async fn connect_with_info(conn: &SqlConnection) -> Result<SqlClient> {
    if conn.auth_type == "windows" {
        connect_windows_auth(&conn.server, &conn.database).await
    } else {
        let username = conn
            .username
            .as_deref()
            .ok_or_else(|| anyhow!("Username mancante per connessione SQL"))?;
        let password = conn
            .password
            .as_deref()
            .ok_or_else(|| anyhow!("Password mancante per connessione SQL"))?;

        connect_sql_auth(&conn.server, &conn.database, username, password).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_select_query() {
        let valid_queries = vec![
            "SELECT * FROM Users",
            "SELECT TOP 10 Name, Email FROM Customers WHERE Active = 1",
            "WITH CTE AS (SELECT * FROM Orders) SELECT * FROM CTE",
            "select * from products",
        ];

        for query in valid_queries {
            assert!(
                validate_readonly_query(query).is_ok(),
                "Query dovrebbe essere valida: {}",
                query
            );
        }
    }

    #[test]
    fn test_validate_forbidden_queries() {
        let invalid_queries = vec![
            "UPDATE Users SET Name = 'Test'",
            "INSERT INTO Users VALUES ('test')",
            "DELETE FROM Users WHERE Id = 1",
            "DROP TABLE Users",
        ];

        for query in invalid_queries {
            assert!(
                validate_readonly_query(query).is_err(),
                "Query dovrebbe essere invalida: {}",
                query
            );
        }
    }
}
