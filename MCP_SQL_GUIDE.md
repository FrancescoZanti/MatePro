# Tool MCP SQL Server - MatePro

## Panoramica

MatePro supporta la connessione a database SQL Server tramite **MCP (Model Context Protocol)** per analisi e report su dati gestionali. Le funzionalità SQL sono completamente **READ-ONLY** per garantire sicurezza.

---

## ⚠️ Limitazioni di Sicurezza

### Solo Lettura (Read-Only)
**IMPORTANTE:** Tutte le query SQL sono validate prima dell'esecuzione. Sono permesse **SOLO query SELECT**.

❌ **Operazioni VIETATE:**
- `UPDATE` - Modifica dati
- `INSERT` - Inserimento dati
- `DELETE` - Cancellazione dati
- `DROP` - Eliminazione oggetti
- `CREATE` - Creazione oggetti
- `ALTER` - Modifica struttura
- `TRUNCATE` - Cancellazione massiva
- `EXEC` / `EXECUTE` - Esecuzione stored procedure
- `MERGE` - Operazioni di merge
- `BULK INSERT` - Inserimenti bulk

✅ **Operazioni PERMESSE:**
- `SELECT` - Lettura dati
- `WITH` (CTE) - Common Table Expressions
- `JOIN` - Join tra tabelle
- Funzioni aggregate: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`
- Funzioni di stringa/data
- Subquery in SELECT

---

## 🛠️ Tool Disponibili

### 1. sql_connect

**Connette a database SQL Server**

#### Parametri:
- `server` (string, required) - Nome o IP del server (es: `localhost`, `192.168.1.10`, `server.domain.com`)
- `database` (string, required) - Nome del database
- `auth_method` (string, required) - Metodo autenticazione: `"windows"` o `"sql"`
- `username` (string, optional) - Username SQL (solo per `auth_method="sql"`)
- `password` (string, optional) - Password SQL (solo per `auth_method="sql"`)

#### Esempio JSON (Windows Auth):
```json
{
  "tool": "sql_connect",
  "parameters": {
    "server": "192.168.1.100",
    "database": "Gestionale",
    "auth_method": "windows"
  }
}
```

#### Esempio JSON (SQL Auth):
```json
{
  "tool": "sql_connect",
  "parameters": {
    "server": "localhost",
    "database": "Gestionale",
    "auth_method": "sql",
    "username": "sa",
    "password": "MyPassword123"
  }
}
```

#### Output:
```
✅ Connesso a SQL Server!
Connection ID: conn_a1b2c3d4
Server: 192.168.1.100
Database: Gestionale
Autenticazione: windows

Usa questo connection_id per le query successive.
```

**⚠️ Nota:** Salva il `connection_id` per usarlo negli altri tool.

---

### 2. sql_query

**Esegue query SELECT sul database connesso**

#### Parametri:
- `connection_id` (string, required) - ID connessione ottenuto da `sql_connect`
- `query` (string, required) - Query SQL SELECT

#### Esempio JSON:
```json
{
  "tool": "sql_query",
  "parameters": {
    "connection_id": "conn_a1b2c3d4",
    "query": "SELECT TOP 10 * FROM Clienti WHERE Citta = 'Roma'"
  }
}
```

#### Output:
```json
📊 Risultati query:
[
  {
    "ClienteID": 1,
    "RagioneSociale": "Acme Corp",
    "Citta": "Roma",
    "Email": "info@acme.com"
  },
  {
    "ClienteID": 5,
    "RagioneSociale": "Beta SRL",
    "Citta": "Roma",
    "Email": "contact@beta.it"
  }
]
```

#### Validazione Query
- Query viene analizzata prima dell'esecuzione
- Se contiene operazioni di scrittura → **ERRORE**
- Solo SELECT e CTE consentiti

---

### 3. sql_list_tables

**Lista tutte le tabelle e view del database**

#### Parametri:
- `connection_id` (string, required) - ID connessione

#### Esempio JSON:
```json
{
  "tool": "sql_list_tables",
  "parameters": {
    "connection_id": "conn_a1b2c3d4"
  }
}
```

#### Output:
```json
📋 Tabelle del database:
[
  {"schema": "dbo", "name": "Clienti", "type": "TABLE"},
  {"schema": "dbo", "name": "Ordini", "type": "TABLE"},
  {"schema": "dbo", "name": "Prodotti", "type": "TABLE"},
  {"schema": "dbo", "name": "Fatture", "type": "TABLE"},
  {"schema": "dbo", "name": "ViewOrdiniCompleti", "type": "VIEW"}
]
```

---

### 4. sql_describe_table

**Mostra struttura di una tabella (colonne, tipi, nullable)**

#### Parametri:
- `connection_id` (string, required) - ID connessione
- `schema` (string, required) - Schema della tabella (solitamente `"dbo"`)
- `table` (string, required) - Nome della tabella

#### Esempio JSON:
```json
{
  "tool": "sql_describe_table",
  "parameters": {
    "connection_id": "conn_a1b2c3d4",
    "schema": "dbo",
    "table": "Clienti"
  }
}
```

#### Output:
```json
🔍 Struttura tabella dbo.Clienti:
[
  {
    "column_name": "ClienteID",
    "data_type": "int",
    "is_nullable": false,
    "max_length": 4
  },
  {
    "column_name": "RagioneSociale",
    "data_type": "nvarchar",
    "is_nullable": false,
    "max_length": 200
  },
  {
    "column_name": "Email",
    "data_type": "nvarchar",
    "is_nullable": true,
    "max_length": 100
  }
]
```

---

### 5. sql_disconnect

**Chiude connessione SQL Server**

#### Parametri:
- `connection_id` (string, required) - ID connessione da chiudere

#### Esempio JSON:
```json
{
  "tool": "sql_disconnect",
  "parameters": {
    "connection_id": "conn_a1b2c3d4"
  }
}
```

#### Output:
```
✅ Connessione 'conn_a1b2c3d4' chiusa correttamente.
```

---

## 🖥️ Configurazione UI

MatePro include un'interfaccia grafica per configurare le connessioni SQL.

### Come Usare

1. **Avvia MatePro** e vai in modalità Chat
2. **Clicca pulsante 🗄️ SQL** nell'header
3. **Inserisci parametri connessione:**
   - **Server:** Hostname o IP (es: `localhost`, `192.168.1.100`)
   - **Database:** Nome database
   - **Autenticazione:**
     - **🪟 Windows (Integrated)** - Usa credenziali utente corrente (dominio su Windows)
     - **🔑 SQL Authentication** - Inserisci username/password
4. **Clicca "🔌 Test Connessione"**
5. **Verifica status:**
   - ✓ Verde = Connesso
   - ✕ Rosso = Errore (vedi messaggio)
   - ⟳ Blu = Connessione in corso

### Screenshot UI

```
┌─────────────────────────────────────────────────┐
│  🗄️ Configurazione SQL Server                   │
├─────────────────────────────────────────────────┤
│  Server:     [localhost________________]        │
│  (es: localhost, 192.168.1.10, ...)             │
│                                                  │
│  Database:   [Gestionale_______________]        │
│                                                  │
│  ─────────────────────────────────────────────  │
│  Autenticazione:                                 │
│  ○ 🪟 Windows (Integrated)                       │
│  ● 🔑 SQL Authentication                         │
│                                                  │
│  Username:   [sa____________________]           │
│  Password:   [••••••••••••••••••••••]           │
│                                                  │
│  ─────────────────────────────────────────────  │
│  ✓ connected                                     │
│                                                  │
│  ⚠️ SOLO LETTURA: Le query sono limitate        │
│     a SELECT. UPDATE, INSERT, DELETE             │
│     non sono permesse.                           │
│                                                  │
│  [ 🔌 Test Connessione ]    [ Chiudi ]          │
└─────────────────────────────────────────────────┘
```

---

## 💡 Esempi d'Uso

### Esempio 1: Analisi Vendite Mensili

**Prompt utente:**
```
Connettiti al database Gestionale su server 192.168.1.100 con autenticazione Windows, 
poi mostrami le vendite totali per ogni mese del 2024.
```

**Tool calls (automatici):**

1. Connessione:
```json
{"tool": "sql_connect", "parameters": {
  "server": "192.168.1.100",
  "database": "Gestionale",
  "auth_method": "windows"
}}
```

2. Query vendite:
```json
{"tool": "sql_query", "parameters": {
  "connection_id": "conn_xxx",
  "query": "SELECT MONTH(DataOrdine) AS Mese, SUM(Importo) AS TotaleVendite FROM Ordini WHERE YEAR(DataOrdine) = 2024 GROUP BY MONTH(DataOrdine) ORDER BY Mese"
}}
```

3. Disconnessione:
```json
{"tool": "sql_disconnect", "parameters": {"connection_id": "conn_xxx"}}
```

---

### Esempio 2: Esplorazione Database

**Prompt utente:**
```
Mostrami tutte le tabelle del database Gestionale e descrivimi la struttura della tabella Clienti.
```

**Tool calls:**

1. Connessione
2. Lista tabelle:
```json
{"tool": "sql_list_tables", "parameters": {"connection_id": "conn_xxx"}}
```

3. Descrivi tabella:
```json
{"tool": "sql_describe_table", "parameters": {
  "connection_id": "conn_xxx",
  "schema": "dbo",
  "table": "Clienti"
}}
```

---

### Esempio 3: Report Clienti per Città

**Prompt utente:**
```
Quanti clienti abbiamo in ogni città? Mostrami i primi 10.
```

**Query generata:**
```sql
SELECT TOP 10 Citta, COUNT(*) AS NumeroClienti
FROM Clienti
GROUP BY Citta
ORDER BY NumeroClienti DESC
```

---

## 🔐 Autenticazione Windows (Domain)

### Come Funziona

Quando usi `auth_method="windows"`:
- **Su PC Windows a Dominio:** Usa le credenziali dell'utente loggato (SSO)
- **Su PC Windows Locale:** Usa credenziali Windows locali
- **Su Linux/macOS:** ⚠️ Autenticazione Windows NON supportata (usa SQL Auth)

### Requisiti Windows Domain
- PC deve essere **joined al dominio**
- Utente deve avere **permessi di lettura** sul database
- **Integrated Security** deve essere abilitata su SQL Server
- Firewall deve permettere connessione a SQL Server (porta 1433)

### Configurazione SQL Server
```sql
-- Crea login per utente dominio
CREATE LOGIN [DOMAIN\username] FROM WINDOWS;

-- Assegna permessi read-only
USE Gestionale;
CREATE USER [DOMAIN\username] FOR LOGIN [DOMAIN\username];
ALTER ROLE db_datareader ADD MEMBER [DOMAIN\username];
```

---

## 🐧 Linux / macOS

Su sistemi non-Windows, usa **SQL Authentication**:

```json
{
  "auth_method": "sql",
  "username": "readonly_user",
  "password": "secure_password"
}
```

### Crea Utente Read-Only su SQL Server
```sql
-- Crea login SQL
CREATE LOGIN readonly_user WITH PASSWORD = 'SecurePassword123!';

-- Associa al database
USE Gestionale;
CREATE USER readonly_user FOR LOGIN readonly_user;

-- Solo permessi di lettura
ALTER ROLE db_datareader ADD MEMBER readonly_user;
```

---

## ⚙️ Configurazione SQL Server

### Abilita TCP/IP
1. Apri **SQL Server Configuration Manager**
2. Vai su **SQL Server Network Configuration** → **Protocols**
3. Abilita **TCP/IP**
4. Riavvia servizio SQL Server

### Abilita Mixed Mode Authentication
1. In **SQL Server Management Studio (SSMS)**
2. Right-click sul server → **Properties**
3. **Security** → Seleziona **SQL Server and Windows Authentication mode**
4. Riavvia SQL Server

### Firewall
```bash
# Windows - Apri porta 1433
netsh advfirewall firewall add rule name="SQL Server" dir=in action=allow protocol=TCP localport=1433
```

---

## 🧪 Testing

### Test Manuale UI
1. Apri MatePro
2. Clicca 🗄️ SQL
3. Inserisci dati connessione
4. Clicca "Test Connessione"
5. Verifica status verde

### Test Prompt con Agente
```
Attiva modalità agente, poi:
"Connettiti al database Gestionale su localhost con SQL auth (user: sa, password: Test123), 
lista le tabelle e mostrami i primi 5 record di ogni tabella."
```

### Verifica Sicurezza Read-Only
```
"Esegui query: UPDATE Clienti SET Email = 'test@test.com' WHERE ClienteID = 1"
```

**Risposta attesa:** ❌ Errore - Query non permessa: contiene operazione di scrittura 'UPDATE'.

---

## 🔧 Troubleshooting

### ❌ "Impossibile connettersi al server"

**Cause:**
- Server SQL non raggiungibile
- Firewall blocca porta 1433
- Nome server errato

**Soluzioni:**
```bash
# Testa connettività
telnet 192.168.1.100 1433

# Ping server
ping server.domain.com

# Verifica SQL Server in esecuzione (Windows)
services.msc → SQL Server (instance)
```

---

### ❌ "Login failed for user"

**Cause:**
- Credenziali errate
- Utente non ha permessi
- Mixed mode auth disabilitata

**Soluzioni:**
- Verifica username/password corretti
- Controlla permessi utente in SSMS
- Abilita Mixed Mode Authentication

---

### ❌ "Query non permessa: contiene operazione di scrittura"

**Causa:** Stai tentando una query che modifica dati.

**Soluzione:** Usa solo `SELECT`. MatePro blocca tutte le operazioni di scrittura per sicurezza.

---

### ⚠️ Autenticazione Windows non funziona su Linux

**Causa:** Windows Integrated Authentication richiede Active Directory / Kerberos.

**Soluzione:** Usa SQL Authentication su Linux/macOS.

---

## 📊 Casi d'Uso Reali

### 1. Analisi Fatturato Annuale
```
"Mostrami il fatturato totale per anno degli ultimi 3 anni, 
raggruppato per categoria prodotto."
```

### 2. Top 10 Clienti
```
"Quali sono i 10 clienti che hanno speso di più quest'anno?"
```

### 3. Inventario Prodotti
```
"Mostrami tutti i prodotti con giacenza inferiore a 10 unità."
```

### 4. Ordini Pendenti
```
"Lista tutti gli ordini in stato 'In Attesa' con data superiore a 7 giorni fa."
```

### 5. Report Geografico
```
"Quanti clienti abbiamo per ogni regione? Ordina per numero decrescente."
```

---

## 🚀 Best Practices

### 1. Usa Connection ID
Salva il `connection_id` dopo `sql_connect` per riusarlo in più query.

### 2. Chiudi Connessioni
Usa sempre `sql_disconnect` quando hai finito per liberare risorse.

### 3. Query Efficienti
- Usa `TOP N` per limitare risultati
- Evita `SELECT *` su tabelle grandi
- Usa indici appropriati

### 4. Sicurezza
- **Non condividere** password in prompt pubblici
- Usa **utenti read-only dedicati**
- Rivedi **permessi database** periodicamente

### 5. Testing
- Testa query complesse in SSMS prima
- Verifica performance su grandi dataset
- Usa `EXPLAIN` per ottimizzare query

---

## 📝 Note Tecniche

### Driver Utilizzato
- **tiberius** 0.12.3 - Native Rust SQL Server driver
- **tokio-util** - Async I/O
- **tokio** - Async runtime

### Limitazioni
- **Max risultati:** Non c'è limite hard, ma query grandi possono rallentare l'app
- **Timeout:** 30 secondi default per query
- **Connessioni simultanee:** Illimitate (limitato solo da memoria)

### Performance
- Prima connessione: ~500ms - 2s
- Query semplici: ~50-200ms
- Query complesse: variabile (dipende da DB)

---

## 📚 Documentazione Correlata

- **AGENT_FEATURES.md** - Funzionalità agentiche complete
- **TOOL_REFERENCE.md** - Reference rapido di tutti i tool
- **README.md** - Panoramica progetto MatePro

---

**Versione:** 0.0.12  
**Data:** Dicembre 2025  
**Funzionalità MCP SQL:** ✅ Completa e Testata
