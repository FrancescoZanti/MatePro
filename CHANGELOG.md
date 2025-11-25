# Changelog - MatePro

## [0.0.4] - Migrazione GUI a Tauri (2025)

### 🎉 Novità Principali

#### Migrazione a Tauri
- **BREAKING:** Migrazione completa dell'interfaccia grafica da egui/eframe a **Tauri v2**
- Frontend HTML/CSS/JavaScript moderno con design elegante
- Backend Rust con sistema IPC per comunicazione frontend-backend
- Stesse funzionalità agentiche e SQL Server della versione precedente
- Tema chiaro/scuro automatico basato sulle preferenze di sistema

#### Architettura
- `src-tauri/` - Backend Rust con Tauri v2
  - Sistema di comandi IPC per tutte le operazioni
  - Gestione stato thread-safe con tokio::sync::Mutex
  - Plugin opener per apertura URL
  - Plugin shell per operazioni di sistema
- `ui/` - Frontend web moderno
  - HTML5 semantico con accessibilità
  - CSS moderno con variabili per temi
  - JavaScript vanilla per massime performance
  - Design responsive per diverse risoluzioni

#### Vantaggi della Migrazione
- 🚀 **Performance migliorate** - Rendering web nativo più veloce
- 🎨 **UI più flessibile** - HTML/CSS permette styling più avanzato
- 📦 **Bundle più piccoli** - Tauri produce binari più leggeri
- 🔒 **Sicurezza** - Sandbox Tauri per isolamento processi
- 🌐 **Cross-platform migliorato** - Stesso codice su tutti i sistemi

#### Funzionalità Preservate
- ✅ Scansione automatica rete per server Ollama
- ✅ Chat conversazionale con rendering Markdown
- ✅ Modalità Agente con tutti i tool
- ✅ Connessione SQL Server (MCP)
- ✅ Caricamento file (PDF, Excel, TXT)
- ✅ Tema chiaro/scuro adattivo

### 📦 Struttura Progetto
```
matepro/
├── src-tauri/           # Tauri app (principale)
│   ├── src/
│   │   ├── main.rs      # Entry point e comandi IPC
│   │   ├── agent.rs     # Sistema tool agentico
│   │   └── mcp_sql.rs   # Connessione SQL Server
│   ├── icons/           # Icone app
│   └── tauri.conf.json  # Configurazione Tauri
├── ui/                  # Frontend web
│   ├── index.html       # Pagina principale
│   ├── styles.css       # Stili
│   └── app.js           # Logica frontend
└── src/                 # Codice legacy egui (preservato)
```

### 🔧 Dipendenze Principali
- `tauri` v2 - Framework desktop
- `tauri-plugin-shell` v2 - Operazioni shell
- `tauri-plugin-opener` v2 - Apertura URL/file
- Tutte le dipendenze backend dalla versione precedente

---

## [0.0.3-alpha] - Funzionalità Agentiche e SQL Server (2025)

### 🎉 Novità Principali

#### Modalità Agente
- Aggiunta **modalità agente** attivabile dall'interfaccia utente
- L'assistente AI può ora **prendere il controllo del computer** per eseguire operazioni
- Sistema di **tool calling** per interazione con il sistema operativo
- **Nuovo:** Comprensione azioni complesse che richiedono visualizzazione web

#### Tool Sistema (6)
1. **shell_execute** - Esecuzione comandi shell (con conferma)
2. **file_read** - Lettura file dal filesystem
3. **file_write** - Scrittura/creazione file (con conferma)
4. **file_list** - Navigazione directory con supporto ricorsivo
5. **process_list** - Lista processi attivi nel sistema
6. **system_info** - Informazioni hardware e sistema

#### Tool Web e Browser (5) 🆕
7. **browser_open** - Apre URL nel browser predefinito
8. **web_search** - Ricerca Google con query parametrizzata
9. **map_open** - Apre Google Maps per località o indicazioni
10. **youtube_search** - Cerca video su YouTube
11. **document_view** - Apre file locali con programma predefinito

#### Tool MCP SQL Server (5) 🆕🗄️
12. **sql_connect** - Connessione SQL Server (Windows/SQL Auth)
13. **sql_query** - Esecuzione query SELECT (read-only)
14. **sql_list_tables** - Lista tabelle e view database
15. **sql_describe_table** - Struttura tabella (colonne, tipi)
16. **sql_disconnect** - Chiusura connessione SQL

**Features MCP SQL:**
- ✅ Autenticazione Windows (Integrated Security) su PC a dominio
- ✅ Autenticazione SQL con username/password
- ✅ Validazione query READ-ONLY (solo SELECT)
- ✅ UI configurazione database con test connessione
- ✅ Support cross-platform (Windows full, Linux/macOS SQL Auth only)
- ⚠️ Sicurezza: UPDATE/INSERT/DELETE automaticamente bloccati

#### Sicurezza
- Sistema di **conferme esplicite** per operazioni pericolose
- Finestra modale per **approvazione/annullamento** operazioni
- Indicatori visivi per tool che richiedono conferma (⚠️)
- Limite iterazioni agentiche (max 5) per evitare loop infiniti

#### Interfaccia Utente
- Toggle **"🤖 Modalità Agente"** nell'header
- Contatore iterazioni correnti visibile
- Messaggi di sistema (🔧) per mostrare risultati tool
- Indicatore "Sto pensando..." durante elaborazione
- Log operazioni formattati in Markdown nella chat

#### Architettura
- Nuovo modulo `agent.rs` con sistema completo di gestione tool
- Parser JSON per estrarre tool calls dalle risposte LLM
- Executor asincrono per esecuzione tool in background
- Ciclo agentico autonomo con feedback automatico

### 📦 Dipendenze Aggiunte
- `regex` 1.10 - Pattern matching per parsing tool calls
- `sysinfo` 0.30 - Informazioni sistema e processi
- `walkdir` 2.4 - Navigazione filesystem ricorsiva
- **Nuovo:** `webbrowser` 1.0 - Apertura browser cross-platform
- **Nuovo:** `url` 2.5 - Parsing e validazione URL
- **Nuovo:** `urlencoding` 2.1 - Encoding parametri query
- **Nuovo:** `tiberius` 0.12.3 - Driver nativo SQL Server per Rust
- **Nuovo:** `tokio-util` 0.7 - Utilità async (compat layer)
- **Nuovo:** `lazy_static` 1.4 - Gestione stato globale connessioni
- **Nuovo:** `uuid` 1.0 - Generazione ID connessioni (feature v4)

### 📝 Documentazione
- `AGENT_FEATURES.md` - Documentazione completa funzionalità agentiche
- `AGENT_TEST_PROMPTS.md` - Esempi e test per modalità agente (21 scenari)
- **Nuovo:** `AGENT_WEB_TOOLS.md` - Guida completa tool web (400+ righe)
- **Nuovo:** `AGENT_WEB_TEST_PROMPTS.md` - 35 test prompts per tool web
- **Nuovo:** `MCP_SQL_GUIDE.md` - Guida completa SQL Server (600+ righe)
- **Nuovo:** `MCP_SQL_TEST_PROMPTS.md` - 33 test prompts SQL (8 categorie)
- README aggiornato con sezione dedicata

### 🔧 Miglioramenti Tecnici
- Gestione errori migliorata per operazioni tool
- Thread safety con clonazione AgentSystem
- Promise per gestione operazioni lunghe
- **Nuovo:** Prompt system enhancement con riconoscimento azioni complesse
- **Nuovo:** Validazione URL con schema HTTP/HTTPS
- **Nuovo:** Encoding sicuro query parametri
- **Nuovo:** Supporto apertura file locali con programma predefinito
- **Nuovo:** Modulo `mcp_sql.rs` (335 righe) per gestione SQL Server
- **Nuovo:** Gestione connessioni globali con lazy_static Arc<Mutex<HashMap>>
- **Nuovo:** Validazione query read-only con regex (blocca 14+ operazioni write)
- **Nuovo:** Supporto asincrono SQL con tokio-util compat layer
- **Nuovo:** UI Promise per test connessione SQL non-bloccante
- UI reattiva durante esecuzione tool

### 🐛 Bug Fix
- Risolti problemi di borrowing con modali di conferma
- Corretta compatibilità con nuova versione sysinfo
- Rimossi import duplicati

---

## [0.0.1] - Release Iniziale

### Funzionalità Base
- Interfaccia grafica moderna con egui/eframe
- Scansione automatica rete per trovare server Ollama
- Connessione a istanze Ollama locali e remote
- Selezione modelli con indicatore peso
- Chat conversazionale con rendering Markdown
- Supporto caricamento file (PDF, Excel, TXT)
- Tema chiaro/scuro adattivo
- Timestamp messaggi
- Formule matematiche con Unicode
- Scorciatoie tastiera

### Piattaforme Supportate
- Linux (testato su Ubuntu/Debian)
- macOS (Intel e Apple Silicon)
- Windows

---

## Formato Versioni

Il progetto segue [Semantic Versioning](https://semver.org/):
- **MAJOR**: Modifiche incompatibili all'API
- **MINOR**: Nuove funzionalità backward-compatible
- **PATCH**: Bug fix backward-compatible

## Link Utili

- [Repository GitHub](https://github.com/FrancescoZanti/MatePro)
- [Issues](https://github.com/FrancescoZanti/MatePro/issues)
- [Releases](https://github.com/FrancescoZanti/MatePro/releases)
