# Test Prompts - MCP SQL Server

Esempi di prompt per testare le funzionalità SQL Server di MatePro.

---

## Prerequisiti

1. SQL Server installato e raggiungibile
2. Database di test con dati (es: `Gestionale`, `Northwind`, `AdventureWorks`)
3. Credenziali appropriate (Windows Auth o SQL Auth)
4. MatePro avviato con **Modalità Agente attiva**

---

## Test Basici

### Test 1: Connessione Windows Auth

```
Connettiti al database Gestionale sul server localhost usando autenticazione Windows.
```

**Tool atteso:** `sql_connect` con `auth_method="windows"`

**Risultato:** Connessione stabilita, `connection_id` fornito

---

### Test 2: Connessione SQL Auth

```
Connettiti al server SQL 192.168.1.100, database Northwind, 
usando username 'sa' e password 'Test123!'.
```

**Tool atteso:** `sql_connect` con `auth_method="sql"`

**Risultato:** Connessione con credenziali SQL

---

### Test 3: Lista Tabelle

```
Dopo esserti connesso, mostrami tutte le tabelle del database.
```

**Tool atteso:** `sql_list_tables`

**Risultato:** Array JSON con schema, nome, tipo (TABLE/VIEW)

---

### Test 4: Descrivi Tabella

```
Descrivimi la struttura della tabella dbo.Clienti.
```

**Tool atteso:** `sql_describe_table` con schema="dbo", table="Clienti"

**Risultato:** Colonne, tipi, nullable, max_length

---

### Test 5: Query Semplice

```
Mostrami i primi 10 clienti dalla tabella Clienti.
```

**Tool atteso:** `sql_query` con query SELECT TOP 10

**Risultato:** Array JSON con dati clienti

---

## Test Intermedi

### Test 6: Query con Filtro

```
Trova tutti i clienti che vivono a Roma.
```

**Query generata:**
```sql
SELECT * FROM Clienti WHERE Citta = 'Roma'
```

---

### Test 7: Query Aggregate

```
Quanti ordini ci sono per ogni cliente? Mostrami i top 10.
```

**Query generata:**
```sql
SELECT TOP 10 ClienteID, COUNT(*) AS NumeroOrdini
FROM Ordini
GROUP BY ClienteID
ORDER BY NumeroOrdini DESC
```

---

### Test 8: Join Multi-Tabella

```
Mostrami gli ordini con nome cliente e prodotto ordinato.
```

**Query generata:**
```sql
SELECT o.OrdineID, c.RagioneSociale, p.NomeProdotto, o.Quantita
FROM Ordini o
JOIN Clienti c ON o.ClienteID = c.ClienteID
JOIN Prodotti p ON o.ProdottoID = p.ProdottoID
```

---

### Test 9: Analisi Temporale

```
Mostrami le vendite totali per ogni mese del 2024.
```

**Query generata:**
```sql
SELECT MONTH(DataOrdine) AS Mese, 
       SUM(Importo) AS TotaleVendite
FROM Ordini
WHERE YEAR(DataOrdine) = 2024
GROUP BY MONTH(DataOrdine)
ORDER BY Mese
```

---

### Test 10: Disconnessione

```
Chiudi la connessione SQL.
```

**Tool atteso:** `sql_disconnect`

**Risultato:** Connessione chiusa, risorse liberate

---

## Test Sicurezza (Read-Only)

### Test 11: Blocco UPDATE

```
Esegui: UPDATE Clienti SET Email = 'test@test.com' WHERE ClienteID = 1
```

**Risultato atteso:** ❌ Errore - Query non permessa: contiene 'UPDATE'

---

### Test 12: Blocco INSERT

```
Inserisci un nuovo cliente nella tabella Clienti.
```

**Risultato atteso:** ❌ Errore - Query non permessa: contiene 'INSERT'

---

### Test 13: Blocco DELETE

```
Elimina tutti i clienti di Roma.
```

**Risultato atteso:** ❌ Errore - Query non permessa: contiene 'DELETE'

---

### Test 14: Blocco DROP

```
Elimina la tabella Clienti.
```

**Risultato atteso:** ❌ Errore - Query non permessa: contiene 'DROP'

---

### Test 15: Blocco EXEC

```
Esegui la stored procedure sp_BackupDatabase.
```

**Risultato atteso:** ❌ Errore - Query non permessa: contiene 'EXEC'

---

## Test Avanzati

### Test 16: Workflow Completo

```
Connettiti a Gestionale su localhost (Windows Auth), 
lista le tabelle, 
mostrami la struttura di Ordini, 
fai una query per trovare ordini oltre 1000€, 
poi disconnetti.
```

**Tool sequence:**
1. `sql_connect`
2. `sql_list_tables`
3. `sql_describe_table`
4. `sql_query` con filtro importo
5. `sql_disconnect`

**Iterazioni:** 5

---

### Test 17: Analisi Multi-Dimensionale

```
Analizza le vendite per categoria prodotto, regione cliente e trimestre.
Mostrami i top 5 per fatturato.
```

**Query complessa con JOIN multipli e GROUP BY**

---

### Test 18: Report Dashboard

```
Crea un report con:
- Totale clienti
- Totale ordini
- Fatturato YTD
- Top 3 prodotti più venduti
- Clienti con ordini > 5000€
```

**Multiple query:**
```sql
-- Query 1: Totale clienti
SELECT COUNT(*) FROM Clienti

-- Query 2: Totale ordini
SELECT COUNT(*) FROM Ordini

-- Query 3: Fatturato YTD
SELECT SUM(Importo) FROM Ordini WHERE YEAR(DataOrdine) = YEAR(GETDATE())

-- Query 4: Top 3 prodotti
SELECT TOP 3 ProdottoID, COUNT(*) FROM Ordini GROUP BY ProdottoID ORDER BY COUNT(*) DESC

-- Query 5: Clienti high-value
SELECT c.* FROM Clienti c JOIN Ordini o ON c.ClienteID = o.ClienteID GROUP BY c.ClienteID HAVING SUM(o.Importo) > 5000
```

---

### Test 19: Subquery e CTE

```
Trova i clienti che hanno speso più della media usando una CTE.
```

**Query con CTE:**
```sql
WITH MediaSpesa AS (
    SELECT AVG(Importo) AS Media FROM Ordini
)
SELECT c.RagioneSociale, SUM(o.Importo) AS TotaleSpeso
FROM Clienti c
JOIN Ordini o ON c.ClienteID = o.ClienteID
CROSS JOIN MediaSpesa
GROUP BY c.RagioneSociale
HAVING SUM(o.Importo) > MediaSpesa.Media
```

---

### Test 20: Analisi Geografica

```
Mostrami la distribuzione dei clienti per regione con percentuale sul totale.
```

**Query con window functions:**
```sql
SELECT Regione, 
       COUNT(*) AS NumClienti,
       CAST(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER() AS DECIMAL(5,2)) AS Percentuale
FROM Clienti
GROUP BY Regione
ORDER BY NumClienti DESC
```

---

## Test Edge Cases

### Test 21: Database Inesistente

```
Connettiti al database 'DatabaseNonEsistente' su localhost.
```

**Risultato atteso:** ❌ Errore - Database non trovato

---

### Test 22: Server Offline

```
Connettiti al server 999.999.999.999.
```

**Risultato atteso:** ❌ Errore - Server non raggiungibile

---

### Test 23: Credenziali Errate

```
Connettiti con username 'wrong' e password 'invalid'.
```

**Risultato atteso:** ❌ Errore - Login failed

---

### Test 24: Query su Connessione Chiusa

```
Dopo aver disconnesso, prova a fare una query.
```

**Risultato atteso:** ❌ Errore - Connessione non trovata

---

### Test 25: Timeout Query Lenta

```
Esegui una query che impiega più di 30 secondi.
```

**Risultato atteso:** ❌ Timeout dopo 30s

---

## Test UI

### Test 26: Configurazione UI Windows Auth

1. Clicca 🗄️ SQL nell'header
2. Inserisci server: `localhost`
3. Inserisci database: `Gestionale`
4. Seleziona **Windows (Integrated)**
5. Clicca **Test Connessione**
6. Verifica status **✓ connected** (verde)

---

### Test 27: Configurazione UI SQL Auth

1. Clicca 🗄️ SQL
2. Inserisci server: `192.168.1.100`
3. Inserisci database: `Northwind`
4. Seleziona **SQL Authentication**
5. Inserisci username: `sa`
6. Inserisci password: `Test123!`
7. Clicca **Test Connessione**
8. Verifica status connesso

---

### Test 28: Test Connessione Fallita UI

1. Inserisci server inesistente
2. Clicca Test Connessione
3. Verifica status **✕ error: ...** (rosso)
4. Leggi messaggio errore

---

### Test 29: Switch Autenticazione

1. Seleziona Windows Auth → Username/Password nascosti
2. Seleziona SQL Auth → Username/Password visibili
3. Verifica tooltip info su Windows Auth

---

### Test 30: Connessioni Multiple UI

1. Connetti a Database1
2. Usa 🗄️ SQL per connetterti a Database2
3. Verifica che il prompt usi il nuovo connection_id

---

## Test Cross-Platform

### Test 31: Windows Domain (Solo Windows)

```
Su PC Windows joined a dominio, connettiti con Windows Auth.
Verifica che usi le credenziali DOMAIN\username.
```

**Prerequisiti:**
- PC Windows in dominio Active Directory
- Utente dominio con permessi DB

---

### Test 32: Linux SQL Auth (Solo Linux)

```
Su Linux, connettiti a SQL Server remoto con SQL Auth.
```

**Nota:** Windows Auth non disponibile su Linux

---

### Test 33: macOS SQL Auth (Solo macOS)

```
Su macOS, connettiti con SQL Auth.
```

**Nota:** Windows Auth non disponibile su macOS

---

## Metriche di Successo

✅ **Connessione OK:**
- Status verde in UI
- connection_id restituito
- Nessun errore

✅ **Query OK:**
- Risultati JSON validi
- Dati corretti
- Tempo < 5s per query semplici

✅ **Sicurezza OK:**
- UPDATE/INSERT/DELETE bloccati
- Errore chiaro e descrittivo
- Nessuna modifica dati

✅ **UI OK:**
- Form intuitivo
- Feedback visivo chiaro
- Nessun crash

---

## Troubleshooting

### ❌ "Cannot connect to server"

**Verifica:**
```bash
# Testa connettività
telnet server.domain.com 1433

# Ping server
ping 192.168.1.100
```

**Soluzioni:**
- Abilita TCP/IP in SQL Server Configuration Manager
- Apri porta 1433 nel firewall
- Verifica nome server corretto

---

### ❌ "Login failed"

**Verifica:**
- Credenziali corrette
- Utente ha permessi sul database
- Mixed Mode Authentication abilitata (per SQL Auth)

---

### ❌ "Database does not exist"

**Verifica:**
- Nome database corretto (case-sensitive su Linux SQL)
- Database online
- Utente ha accesso al database

---

### ⚠️ Query lenta

**Ottimizzazioni:**
- Usa `TOP N` per limitare risultati
- Aggiungi indici su colonne filtrate
- Evita `SELECT *`
- Usa `WHERE` per ridurre dataset

---

## Modelli LLM Consigliati

Per analisi SQL complesse:

| Modello | SQL Capability | Dimensione |
|---------|---------------|-----------|
| `llama3.1:8b` | ⭐⭐⭐⭐ Ottimo | 4.7GB |
| `mixtral:latest` | ⭐⭐⭐⭐⭐ Eccellente | 26GB |
| `qwen2.5:latest` | ⭐⭐⭐⭐ Ottimo | 4.4GB |
| `codellama:13b` | ⭐⭐⭐ Buono | 7.4GB |

**Raccomandazione:** `mixtral` per query complesse, `llama3.1:8b` per uso generale.

---

## Automazione Testing

### Script Bash Test Connessione

```bash
#!/bin/bash
# test_sql.sh - Test connessione SQL Server

SERVER="localhost"
DATABASE="Gestionale"

echo "Testing SQL Server connection..."
echo "Server: $SERVER"
echo "Database: $DATABASE"

# Usa sqlcmd (se disponibile)
sqlcmd -S "$SERVER" -d "$DATABASE" -E -Q "SELECT @@VERSION" > /dev/null 2>&1

if [ $? -eq 0 ]; then
    echo "✅ Connection successful!"
else
    echo "❌ Connection failed!"
    exit 1
fi
```

---

### Script PowerShell Test (Windows)

```powershell
# test_sql.ps1 - Test connessione SQL Server

$Server = "localhost"
$Database = "Gestionale"

try {
    $conn = New-Object System.Data.SqlClient.SqlConnection
    $conn.ConnectionString = "Server=$Server;Database=$Database;Integrated Security=True;"
    $conn.Open()
    Write-Host "✅ Connection successful!" -ForegroundColor Green
    $conn.Close()
} catch {
    Write-Host "❌ Connection failed: $_" -ForegroundColor Red
    exit 1
}
```

---

**Ultima modifica:** Novembre 2024  
**Tool MCP SQL:** 5 (connect, query, list_tables, describe_table, disconnect)  
**Sicurezza:** ✅ Read-Only Enforced  
**Platform:** Windows (full support) | Linux/macOS (SQL Auth only)
