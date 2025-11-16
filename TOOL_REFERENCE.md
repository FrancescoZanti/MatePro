# Riferimento Rapido Tool - MatePro Agent

## Tool Sistema (6)

### 1. shell_execute ⚠️
**Esegue comandi shell nel sistema operativo**

```json
{
  "tool": "shell_execute",
  "parameters": {
    "command": "ls -la /home/user"
  }
}
```

**Pericoloso:** ✅ Richiede conferma  
**Permessi:** Dipendono dall'utente che esegue MatePro  
**Output:** stdout + stderr del comando

---

### 2. file_read
**Legge contenuto di un file dal filesystem**

```json
{
  "tool": "file_read",
  "parameters": {
    "file_path": "/home/user/document.txt"
  }
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Limiti:** File di testo, encoding UTF-8  
**Output:** Contenuto file completo

---

### 3. file_write ⚠️
**Scrive o crea un file sul filesystem**

```json
{
  "tool": "file_write",
  "parameters": {
    "file_path": "/home/user/newfile.txt",
    "content": "Hello, World!"
  }
}
```

**Pericoloso:** ✅ Richiede conferma  
**Comportamento:** Sovrascrive se file esiste  
**Output:** Conferma creazione/modifica

---

### 4. file_list
**Lista contenuto di una directory**

```json
{
  "tool": "file_list",
  "parameters": {
    "directory_path": "/home/user/projects",
    "recursive": false
  }
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Parametri:**
- `recursive: true` - Scandisce sottodirectory
- `recursive: false` - Solo livello corrente

**Output:** Lista file e cartelle con path completi

---

### 5. process_list
**Mostra processi attivi nel sistema**

```json
{
  "tool": "process_list",
  "parameters": {}
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Informazioni:** PID, nome processo, CPU%, memoria  
**Output:** Tabella formattata processi

---

### 6. system_info
**Informazioni hardware e sistema operativo**

```json
{
  "tool": "system_info",
  "parameters": {}
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Informazioni:**
- Sistema operativo e versione
- Nome host
- CPU (modello, core, frequenza)
- RAM (totale, usata, disponibile)
- Kernel version

**Output:** Report formattato sistema

---

## Tool Web e Browser (5)

### 7. browser_open
**Apre URL nel browser predefinito**

```json
{
  "tool": "browser_open",
  "parameters": {
    "url": "https://github.com"
  }
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Validazione:** Solo HTTP/HTTPS  
**Cross-platform:** Linux, Windows, macOS  
**Output:** Conferma apertura browser

---

### 8. web_search
**Ricerca Google con query parametrizzata**

```json
{
  "tool": "web_search",
  "parameters": {
    "query": "Rust programming tutorial"
  }
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Encoding:** Automatico per caratteri speciali  
**Risultato:** Apre pagina risultati Google  
**Output:** Conferma ricerca avviata

---

### 9. map_open
**Apre Google Maps per località o indicazioni**

```json
{
  "tool": "map_open",
  "parameters": {
    "location": "Milano, Italia",
    "mode": "search"
  }
}
```

**Modalità disponibili:**
- `"search"` - Cerca località sulla mappa
- `"directions"` - Calcola percorso (usa `from:X to:Y`)

**Esempi:**
```json
// Cerca luogo
{"location": "Colosseo, Roma", "mode": "search"}

// Indicazioni stradali
{"location": "from:Milano to:Venezia", "mode": "directions"}
```

**Pericoloso:** ❌ Nessuna conferma  
**Output:** Conferma apertura Maps

---

### 10. youtube_search
**Cerca video su YouTube**

```json
{
  "tool": "youtube_search",
  "parameters": {
    "query": "Python tutorial italiano"
  }
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Encoding:** Automatico per query  
**Risultato:** Apre pagina risultati YouTube  
**Output:** Conferma ricerca avviata

---

### 11. document_view
**Apre file locale con programma predefinito**

```json
{
  "tool": "document_view",
  "parameters": {
    "file_path": "/home/user/report.pdf"
  }
}
```

**Pericoloso:** ❌ Nessuna conferma  
**Supporto:** PDF, Office, immagini, video, etc.  
**Comportamento:** Usa associazioni file di sistema  
**Output:** Conferma apertura documento

---

## Formato Tool Call

Tutti i tool devono essere chiamati in questo formato JSON racchiuso in code block markdown:

````markdown
```json
{
  "tool": "nome_tool",
  "parameters": {
    "param1": "valore1",
    "param2": "valore2"
  }
}
```
````

### Esempio Completo nella Risposta LLM

```
Certamente! Eseguirò il comando per te.

```json
{
  "tool": "shell_execute",
  "parameters": {
    "command": "ls -la"
  }
}
```

Ho eseguito il comando ls per mostrare i file.
```

---

## Quick Reference - Casi d'Uso

| Vuoi... | Tool da Usare | Esempio |
|---------|---------------|---------|
| Eseguire comando | `shell_execute` | `ls`, `ps aux`, `df -h` |
| Leggere file | `file_read` | Log, config, codice sorgente |
| Creare file | `file_write` | Script, documenti, config |
| Esplorare directory | `file_list` | Trovare file, analisi struttura |
| Monitorare processi | `process_list` | CPU/RAM usage, processo specifico |
| Info sistema | `system_info` | Hardware specs, versioni |
| Aprire sito web | `browser_open` | GitHub, documentazione |
| Cercare online | `web_search` | Info in tempo reale, tutorial |
| Visualizzare mappa | `map_open` | Località, indicazioni stradali |
| Cercare video | `youtube_search` | Tutorial, guide, recensioni |
| Aprire documento | `document_view` | PDF, Office, immagini |

---

## Pattern Matching - Riconoscimento Automatico

L'agente riconosce automaticamente questi pattern:

### Informazioni Tempo Reale → web_search
- "Che tempo fa a...?"
- "Ultime notizie..."
- "Quotazione borsa..."
- "Risultati partita..."

### Località/Mappe → map_open
- "Mostrami X sulla mappa"
- "Dove si trova...?"
- "Come arrivo a...?"
- "Indicazioni per..."

### Video/Tutorial → youtube_search
- "Cerca video su..."
- "Tutorial per..."
- "Mostrami come..."
- "Voglio imparare..." (+ menzione video)

### URL Diretti → browser_open
- "Apri il sito di..."
- "Vai su https://..."
- "Apri GitHub/Stack Overflow/etc."

### Comandi Sistema → shell_execute
- "Esegui ls/ps/grep..."
- "Controlla processi..."
- "Mostrami spazio disco..."

### Navigazione File → file_list
- "Quali file ci sono in...?"
- "Mostrami la directory..."
- "Cerca file nella cartella..."

---

## Sicurezza

### Tool Pericolosi ⚠️
Richiedono conferma esplicita utente:

1. **shell_execute** - Può eseguire codice arbitrario
2. **file_write** - Può sovrascrivere file importanti

**Modale conferma:** L'utente deve cliccare "✅ Conferma" o "❌ Annulla"

### Tool Sicuri ✅
Eseguiti automaticamente senza conferma:

3. file_read
4. file_list
5. process_list
6. system_info
7. browser_open
8. web_search
9. map_open
10. youtube_search
11. document_view

**Ragionamento:** Operazioni read-only o apertura browser (non modificano sistema)

---

## Limiti e Considerazioni

### Iterazioni
- **Max iterazioni:** 5 per richiesta
- **Ragione:** Evitare loop infiniti
- **Comportamento:** Dopo 5 cicli, agente si ferma e avvisa utente

### Dimensione Output
- Output tool troncato se > 2000 caratteri
- File molto grandi potrebbero essere parziali
- Usare filtri (grep, head, tail) per output grandi

### Permessi
- Tool operano con permessi utente che esegue MatePro
- Non è possibile eseguire operazioni privilegiate senza sudo
- Filesystem inaccessibile se permessi insufficienti

### Browser
- Richiede browser predefinito configurato
- Linux: Usa variabile `$BROWSER` o `xdg-open`
- Windows: Usa browser predefinito di sistema
- macOS: Usa `open` command

---

## Tips per Prompt Efficaci

### ✅ Buoni Prompts

```
"Lista i file Python nella directory src/"
→ Chiaro, specifico, tool ovvio (file_list)

"Cerca tutorial Rust su YouTube"
→ Esplicito, include piattaforma (youtube_search)

"Mostrami come arrivare a Roma da Napoli"
→ Indicazioni chiare, tool evidente (map_open directions)
```

### ❌ Prompts Ambigui

```
"Mostrami informazioni"
→ Troppo vago, cosa? dove?

"Cerca"
→ Cercare cosa? Dove? Web o filesystem?

"Apri quello"
→ Riferimento poco chiaro
```

### 🎯 Best Practices

1. **Sii specifico** - Indica cosa vuoi esattamente
2. **Menziona il contesto** - Web, filesystem, sistema
3. **Usa verbi chiari** - "Apri", "Cerca", "Mostra", "Esegui"
4. **Includi dettagli** - Path completi, query precise

---

## Esempi Multi-Step

### Analisi Progetto
```
"Analizza la struttura del progetto in /home/user/myapp,
leggi il README e cerca informazioni online sulla tecnologia usata"
```

**Tool chiamati:**
1. `file_list` → Struttura directory
2. `file_read` → Contenuto README
3. `web_search` → Info tecnologia

---

### Debug Errore
```
"Il mio script Python genera errore 'ModuleNotFoundError: numpy'.
Controlla se numpy è installato e cerca soluzione online"
```

**Tool chiamati:**
1. `shell_execute` → `pip list | grep numpy`
2. `web_search` → "Python ModuleNotFoundError numpy solution"

---

### Pianificazione Evento
```
"Devo organizzare meeting a Milano.
Mostrami la città sulla mappa, cerca hotel e controlla meteo"
```

**Tool chiamati:**
1. `map_open` → "Milano, Italia" search
2. `web_search` → "hotel Milano centro"
3. `web_search` → "meteo Milano"

---

## Documentazione Completa

Per guide dettagliate, consulta:

📖 **AGENT_FEATURES.md** - Tool sistema e funzionalità base  
📖 **AGENT_WEB_TOOLS.md** - Tool web e browser (400+ righe)  
📖 **AGENT_TEST_PROMPTS.md** - 21 test tool sistema  
📖 **AGENT_WEB_TEST_PROMPTS.md** - 35 test tool web  
📖 **QUICKSTART.md** - Quick start guide  

---

**Ultima modifica:** Dicembre 2024  
**Versione MatePro:** 0.0.2-beta  
**Tool disponibili:** 11 (6 sistema + 5 web)
