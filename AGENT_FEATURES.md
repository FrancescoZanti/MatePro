# Funzionalità Agentiche di MatePro

## Panoramica

MatePro ora include funzionalità agentiche avanzate che permettono all'assistente AI di **prendere il controllo del computer** ed eseguire operazioni seguendo le istruzioni dell'utente.

## Modalità Agente

La modalità agente può essere attivata/disattivata tramite il toggle **"🤖 Modalità Agente"** nell'interfaccia.

Quando attiva, l'assistente può:
- Eseguire comandi shell
- Leggere e scrivere file
- Navigare il filesystem
- Monitorare processi e sistema
- Interagire autonomamente con il sistema operativo


## Tool Disponibili

### 1. `shell_execute` ⚠️ (Pericoloso)
Esegue comandi shell arbitrari.

**Parametri:**
- `command` (string, obbligatorio): Il comando bash da eseguire

**Esempio:**
```json
{
  "tool": "shell_execute",
  "parameters": {
    "command": "ls -la /home"
  }
}
```

### 2. `file_read`
Legge il contenuto di un file.

**Parametri:**
- `path` (string, obbligatorio): Percorso del file da leggere

**Esempio:**
```json
{
  "tool": "file_read",
  "parameters": {
    "path": "/home/user/documento.txt"
  }
}
```

### 3. `file_write` ⚠️ (Pericoloso)
Scrive o sovrascrive un file.

**Parametri:**
- `path` (string, obbligatorio): Percorso del file
- `content` (string, obbligatorio): Contenuto da scrivere

**Esempio:**
```json
{
  "tool": "file_write",
  "parameters": {
    "path": "/tmp/output.txt",
    "content": "Contenuto del file..."
  }
}
```

### 4. `file_list`
Lista file e directory in un percorso.

**Parametri:**
- `path` (string, obbligatorio): Directory da esplorare
- `recursive` (boolean, opzionale): Se true, cerca ricorsivamente

**Esempio:**
```json
{
  "tool": "file_list",
  "parameters": {
    "path": "/home/user/documenti",
    "recursive": true
  }
}
```

### 5. `process_list`
Lista i processi attivi nel sistema.

**Nessun parametro richiesto**

**Esempio:**
```json
{
  "tool": "process_list",
  "parameters": {}
}
```

### 6. `system_info`
Ottiene informazioni sul sistema (CPU, RAM, disco).

**Nessun parametro richiesto**

**Esempio:**
```json
{
  "tool": "system_info",
  "parameters": {}
}
```

## Sicurezza

### Tool Pericolosi
I tool contrassegnati con ⚠️ richiedono **conferma esplicita** dell'utente prima dell'esecuzione:
- `shell_execute`
- `file_write`

### Conferma Operazioni
Quando l'agente tenta di eseguire un'operazione pericolosa:
1. Appare una finestra di dialogo con i dettagli dell'operazione
2. L'utente può **Consentire** o **Annullare** l'operazione
3. Solo dopo conferma il tool viene eseguito

### Limiti
- **Massimo 5 iterazioni** per ciclo agentico (configurabile)
- Timeout e gestione errori per ogni tool
- Log completo di tutte le operazioni

## Ciclo Agentico

Il sistema implementa un ciclo agentico autonomo:

1. **Utente** invia una richiesta
2. **LLM** analizza e decide quali tool usare
3. **Sistema** esegue i tool richiesti (con conferme se necessario)
4. **Risultati** vengono aggiunti al contesto
5. **LLM** continua con i nuovi dati fino al completamento

Questo permette all'assistente di:
- Eseguire task complessi multi-step
- Raccogliere informazioni progressivamente
- Correggere errori autonomamente
- Completare obiettivi complessi senza intervento costante

## Esempi di Utilizzo

### Esempio 1: Analisi Sistema
**Utente:** "Analizza l'utilizzo della memoria del sistema e dimmi quali sono i 5 processi che usano più RAM"

**Agente:**
1. Chiama `system_info` per ottenere info generali
2. Chiama `process_list` per elencare tutti i processi
3. Analizza i dati e restituisce il risultato

### Esempio 2: Gestione File
**Utente:** "Crea un file di backup di tutti i file .txt nella mia home directory"

**Agente:**
1. Chiama `file_list` sulla home con `recursive: true`
2. Filtra i file .txt
3. Legge ogni file con `file_read`
4. Crea un archivio con `file_write` (con conferma)

### Esempio 3: Automazione
**Utente:** "Scarica l'ultima versione di Python e installala"

**Agente:**
1. Chiama `shell_execute` per scaricare (con conferma)
2. Verifica il download con `file_list`
3. Esegue l'installazione con `shell_execute` (con conferma)
4. Verifica l'installazione

## Interfaccia Utente

### Indicatori
- **Toggle verde**: Modalità agente attiva
- **Contatore iterazioni**: Mostra progresso (es. 2/5)
- **Messaggi sistema** (🔧): Risultati tool visibili nella chat
- **Spinner**: "Sto pensando..." durante elaborazione

### Log Operazioni
Ogni operazione eseguita viene:
- Mostrata nella chat con emoji 🔧
- Formattata in markdown per leggibilità
- Colorata (✅ successo, ❌ errore)

## Configurazione

Nel codice è possibile modificare:
- `max_agent_iterations`: Numero massimo iterazioni (default: 5)
- `allow_dangerous`: Permetti tool pericolosi senza conferma (sconsigliato)
- Tool disponibili nel sistema

## Note Tecniche

### Architettura
- **Modulo `agent.rs`**: Sistema tool e executor
- **Parser JSON**: Estrae tool calls dalle risposte LLM
- **Executor asincrono**: Esegue tool in background
- **Sistema conferme**: Gestisce sicurezza operazioni

### Dipendenze
- `regex`: Parsing tool calls
- `sysinfo`: Informazioni sistema/processi
- `walkdir`: Navigazione filesystem ricorsiva

### Thread Safety
- Clonazione `AgentSystem` per esecuzione asincrona
- Promise per gestione operazioni lunghe
- UI reattiva durante esecuzione

## Limitazioni Note

1. **Modelli LLM**: Non tutti i modelli Ollama comprendono bene il formato tool calling
2. **Contesto**: Tool calls devono essere in blocchi ```json```
3. **Iterazioni**: Limite per evitare loop infiniti
4. **Sicurezza**: Sempre verificare operazioni pericolose

## Roadmap Futura

- [ ] Tool per interazione browser/web scraping
- [ ] Tool per gestione processi (avvia/termina)
- [ ] Tool per manipolazione immagini
- [ ] Sandbox mode per testing sicuro
- [ ] History e rollback operazioni
- [ ] Tool personalizzati definiti dall'utente
- [ ] Integrazione con API esterne (GitHub, Jira, etc.)

## Contribuire

Per aggiungere nuovi tool:
1. Definire il tool in `AgentSystem::new()`
2. Implementare l'executor `execute_TOOLNAME()`
3. Aggiornare la documentazione
4. Testare con vari scenari

---

**⚠️ ATTENZIONE:** L'uso della modalità agente comporta rischi. L'assistente può eseguire comandi potenzialmente pericolosi. Usa sempre con cautela e conferma le operazioni critiche.
