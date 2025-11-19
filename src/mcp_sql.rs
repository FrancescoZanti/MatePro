// MCP SQL Server Module
// Gestione connessioni SQL Server con supporto autenticazione Windows/SQL
// IMPORTANTE: Solo operazioni READ-ONLY (SELECT)

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiberius::{AuthMethod, Client, Config, Query};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

// Type alias per semplificare le signature
pub type SqlClient = Client<Compat<TcpStream>>;

/// Rappresenta una connessione SQL Server attiva
#[derive(Clone)]
pub struct SqlConnection {
    pub connection_id: String,
    pub server: String,
    pub database: String,
    pub auth_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Gestore globale delle connessioni SQL
pub struct SqlConnectionManager {
    connections: Arc<Mutex<HashMap<String, SqlConnection>>>,
}

impl SqlConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Aggiunge una connessione al manager
    pub fn add_connection(&self, conn: SqlConnection) {
        let mut conns = self.connections.lock().unwrap();
        conns.insert(conn.connection_id.clone(), conn);
    }

    /// Rimuove una connessione
    pub fn remove_connection(&self, conn_id: &str) -> Option<SqlConnection> {
        let mut conns = self.connections.lock().unwrap();
        conns.remove(conn_id)
    }

    /// Ottiene info su una connessione
    pub fn get_connection(&self, conn_id: &str) -> Option<SqlConnection> {
        let conns = self.connections.lock().unwrap();
        conns.get(conn_id).cloned()
    }

    /// Lista tutte le connessioni attive
    pub fn list_connections(&self) -> Vec<SqlConnection> {
        let conns = self.connections.lock().unwrap();
        conns.values().cloned().collect()
    }
}

/// Valida che una query SQL sia di sola lettura (SELECT)
pub fn validate_readonly_query(query: &str) -> Result<()> {
    let query_upper = query.trim().to_uppercase();

    // Lista delle parole chiave vietate (operazioni di scrittura)
    let forbidden_keywords = [
        "UPDATE",
        "INSERT",
        "DELETE",
        "DROP",
        "CREATE",
        "ALTER",
        "TRUNCATE",
        "EXEC",
        "EXECUTE",
        "SP_",
        "XP_",
        "MERGE",
        "BULK",
        "WRITETEXT",
        "UPDATETEXT",
    ];

    // Rimuovi commenti SQL (-- e /* */)
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

    // Controlla se contiene parole chiave vietate
    for keyword in &forbidden_keywords {
        if without_block_comments.contains(keyword) {
            return Err(anyhow!(
                "Query non permessa: contiene operazione di scrittura '{}'. Solo SELECT è consentito.",
                keyword
            ));
        }
    }

    // Deve iniziare con SELECT, WITH (CTE), o essere una DECLARE per variabili
    let trimmed = without_block_comments.trim();
    if !trimmed.starts_with("SELECT")
        && !trimmed.starts_with("WITH")
        && !trimmed.starts_with("DECLARE")
    {
        return Err(anyhow!(
            "Query non permessa: deve iniziare con SELECT, WITH o DECLARE. Solo operazioni di lettura sono consentite."
        ));
    }

    Ok(())
}

/// Rimuove commenti a blocco /* */ da una stringa SQL
fn remove_block_comments(sql: &str) -> String {
    let mut result = String::new();
    let mut in_comment = false;
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        if !in_comment {
            if c == '/' && chars.peek() == Some(&'*') {
                in_comment = true;
                chars.next(); // Salta '*'
            } else {
                result.push(c);
            }
        } else {
            if c == '*' && chars.peek() == Some(&'/') {
                in_comment = false;
                chars.next(); // Salta '/'
            }
        }
    }

    result
}

/// Connette a SQL Server con autenticazione Windows (dominio)
/// NOTA: Autenticazione Windows richiede funzionalità specifiche del sistema operativo
/// Su Linux potrebbe richiedere configurazione Kerberos
pub async fn connect_windows_auth(server: &str, database: &str) -> Result<SqlClient> {
    #[cfg(windows)]
    {
        let mut config = Config::new();
        config.host(server);
        config.database(database);
        // Su Windows usa SSPI (Windows Integrated Auth)
        // Usa le credenziali correnti del processo per l'autenticazione integrata
        config.authentication(AuthMethod::Integrated);
        config.trust_cert();

        let tcp = TcpStream::connect(config.get_addr()).await?;
        let client = Client::connect(config, tcp.compat_write()).await?;
        Ok(client)
    }

    #[cfg(not(windows))]
    {
        // Su Linux/Mac l'autenticazione Windows non è supportata direttamente
        // Suggerisci di usare SQL Auth
        Err(anyhow!(
            "Autenticazione Windows non supportata su questo sistema operativo.\n\
            Su Linux/macOS usa autenticazione SQL (username/password).\n\
            Server: {}, Database: {}",
            server,
            database
        ))
    }
}

/// Connette a SQL Server con autenticazione SQL (username/password)
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

/// Esegue una query SELECT e ritorna risultati come JSON
pub async fn execute_query(client: &mut SqlClient, query: &str) -> Result<String> {
    // Valida che sia read-only
    validate_readonly_query(query)?;

    // Esegue la query
    let stream = Query::new(query).query(client).await?;

    // Raccogli tutti i risultati
    let rows = stream.into_first_result().await?;

    // Costruisci JSON semplificato
    let mut result_rows = Vec::new();
    for row in rows {
        // Per ora convertiamo tutto in stringhe
        // In produzione gestiresti i tipi specifici
        let mut row_data = Vec::new();
        for col in row.columns() {
            row_data.push(col.name().to_string());
        }
        result_rows.push(format!("{:?}", row));
    }

    let result = serde_json::json!({
        "success": true,
        "row_count": result_rows.len(),
        "rows": result_rows,
        "message": "Query eseguita con successo"
    });

    Ok(serde_json::to_string_pretty(&result)?)
}

/// Lista tutte le tabelle del database
pub async fn list_tables(client: &mut SqlClient) -> Result<String> {
    let query = r#"
        SELECT 
            TABLE_SCHEMA as [Schema],
            TABLE_NAME as [Table],
            TABLE_TYPE as [Type]
        FROM INFORMATION_SCHEMA.TABLES
        WHERE TABLE_TYPE IN ('BASE TABLE', 'VIEW')
        ORDER BY TABLE_SCHEMA, TABLE_NAME
    "#;

    execute_query(client, query).await
}

/// Descrive struttura di una tabella (colonne, tipi, nullable)
pub async fn describe_table(
    client: &mut SqlClient,
    schema: &str,
    table_name: &str,
) -> Result<String> {
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
        schema.replace("'", "''"), // Escape singole quote
        table_name.replace("'", "''")
    );

    execute_query(client, &query).await
}

/// Testa la connessione al database
#[allow(dead_code)]
pub async fn test_connection(
    server: &str,
    database: &str,
    auth_method: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<String> {
    let mut client = if auth_method == "windows" {
        connect_windows_auth(server, database).await?
    } else {
        let user = username.ok_or_else(|| anyhow!("Username richiesto per SQL Auth"))?;
        let pass = password.ok_or_else(|| anyhow!("Password richiesta per SQL Auth"))?;
        connect_sql_auth(server, database, user, pass).await?
    };

    // Query semplice per testare
    let test_query = "SELECT @@VERSION as SqlVersion, DB_NAME() as CurrentDatabase";
    execute_query(&mut client, test_query).await
}

/// Crea un client SQL a partire dalle informazioni memorizzate per una connessione
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
            "select * from products", // lowercase
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
            "CREATE TABLE Test (Id INT)",
            "ALTER TABLE Users ADD Column1 VARCHAR(50)",
            "TRUNCATE TABLE Users",
            "EXEC sp_executesql @sql",
            "SELECT * FROM Users; DROP TABLE Users; --",
        ];

        for query in invalid_queries {
            assert!(
                validate_readonly_query(query).is_err(),
                "Query dovrebbe essere invalida: {}",
                query
            );
        }
    }

    #[test]
    fn test_validate_query_with_comments() {
        let query = "SELECT * FROM Users -- UPDATE Users SET Name = 'hack'";
        assert!(
            validate_readonly_query(query).is_ok(),
            "Commenti dovrebbero essere ignorati"
        );

        let query2 = "SELECT * FROM Users /* UPDATE Users SET Name = 'hack' */";
        assert!(
            validate_readonly_query(query2).is_ok(),
            "Commenti blocco dovrebbero essere ignorati"
        );
    }
}
