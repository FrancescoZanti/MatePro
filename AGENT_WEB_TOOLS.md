# Tool Web e Browser - MatePro Agent Mode

## Panoramica

La modalità agente di MatePro ora include **potenti tool per interagire con il web e il browser**, permettendo all'assistente di aprire pagine, cercare informazioni, visualizzare mappe e molto altro.

## Nuovi Tool Disponibili

### 1. `browser_open` 🌐
Apre un URL specifico nel browser predefinito.

**Quando usarlo:**
- L'utente chiede di aprire un sito specifico
- Vuole visualizzare una pagina web
- Necessita di accedere a servizi online

**Parametri:**
- `url` (string, obbligatorio): URL completo (deve iniziare con http:// o https://)
- `description` (string, opzionale): Descrizione di cosa si sta aprendo

**Esempi d'uso:**
```
Utente: "Apri il sito di GitHub"
Tool: browser_open
Parameters: { "url": "https://github.com", "description": "sito GitHub" }

Utente: "Vai su Wikipedia"
Tool: browser_open
Parameters: { "url": "https://it.wikipedia.org", "description": "Wikipedia italiano" }

Utente: "Mostrami il mio repository"
Tool: browser_open
Parameters: { "url": "https://github.com/FrancescoZanti/MatePro" }
```

---

### 2. `web_search` 🔍
Esegue una ricerca su Google aprendo il browser con i risultati.

**Quando usarlo:**
- L'utente chiede informazioni che richiedono ricerca web
- Vuole trovare notizie, articoli, tutorial
- Necessita di informazioni in tempo reale (meteo, borsa, etc.)

**Parametri:**
- `query` (string, obbligatorio): La query di ricerca

**Esempi d'uso:**
```
Utente: "Cerca informazioni su Rust programming"
Tool: web_search
Parameters: { "query": "Rust programming language tutorial" }

Utente: "Che tempo fa a Milano?"
Tool: web_search
Parameters: { "query": "meteo Milano" }

Utente: "Ultime notizie sulla tecnologia"
Tool: web_search
Parameters: { "query": "ultime notizie tecnologia" }
```

---

### 3. `map_open` 🗺️
Apre Google Maps con una località o percorso.

**Quando usarlo:**
- L'utente chiede di vedere una mappa
- Vuole indicazioni stradali
- Cerca un luogo, ristorante, hotel
- Necessita di un percorso tra due località

**Parametri:**
- `location` (string, obbligatorio): Nome località, indirizzo o coordinate
- `mode` (string, opzionale): 'search' (default) o 'directions' per percorsi

**Esempi d'uso:**
```
Utente: "Mostrami Milano sulla mappa"
Tool: map_open
Parameters: { "location": "Milano, Italia" }

Utente: "Come arrivo a Roma da Milano?"
Tool: map_open
Parameters: { "location": "Roma, Italia", "mode": "directions" }

Utente: "Cerca pizzerie vicino a Piazza Duomo"
Tool: map_open
Parameters: { "location": "pizzeria Piazza Duomo Milano" }

Utente: "Dove si trova Via Roma 10, Torino?"
Tool: map_open
Parameters: { "location": "Via Roma 10, Torino" }
```

---

### 4. `youtube_search` 🎥
Cerca video su YouTube aprendo il browser.

**Quando usarlo:**
- L'utente chiede di vedere video
- Vuole tutorial, guide, corsi
- Cerca musica, film, trailer
- Vuole documentari o contenuti video

**Parametri:**
- `query` (string, obbligatorio): Query di ricerca su YouTube

**Esempi d'uso:**
```
Utente: "Mostrami tutorial Python per principianti"
Tool: youtube_search
Parameters: { "query": "Python tutorial principianti italiano" }

Utente: "Cerca video di ricette italiane"
Tool: youtube_search
Parameters: { "query": "ricette italiane tradizionali" }

Utente: "Voglio vedere trailer Dune 2"
Tool: youtube_search
Parameters: { "query": "Dune 2 trailer italiano" }
```

---

### 5. `document_view` 📄
Apre un documento/file locale nel programma predefinito.

**Quando usarlo:**
- L'utente chiede di aprire un file esistente
- Vuole visualizzare PDF, immagini, documenti
- Necessita di vedere il contenuto di un file

**Parametri:**
- `path` (string, obbligatorio): Percorso completo del file

**Esempi d'uso:**
```
Utente: "Apri il PDF che ho in Downloads"
# Prima trova il file con file_list
Tool: document_view
Parameters: { "path": "/home/user/Downloads/document.pdf" }

Utente: "Mostrami l'immagine screenshot.png"
Tool: document_view
Parameters: { "path": "/home/user/Pictures/screenshot.png" }

Utente: "Visualizza il file README.md"
Tool: document_view
Parameters: { "path": "/home/user/project/README.md" }
```

---

## Riconoscimento Azioni Complesse

L'assistente è stato addestrato a riconoscere automaticamente quando l'utente richiede azioni che necessitano del browser:

### Parole Chiave e Pattern

**Per `web_search`:**
- "cerca", "trova informazioni", "dimmi di"
- "che tempo fa", "meteo"
- "ultime notizie", "notizie su"
- "prezzo di", "quotazione"
- "come si fa", "tutorial"

**Per `map_open`:**
- "mostrami sulla mappa", "dove si trova"
- "come arrivo a", "indicazioni per"
- "cerca ristorante/hotel/negozio"
- "distanza tra", "percorso da X a Y"

**Per `youtube_search`:**
- "mostra video", "cerca video"
- "tutorial video", "guida video"
- "musica di", "canzone"
- "trailer", "clip", "documentario"

**Per `browser_open`:**
- "apri il sito", "vai su"
- "mostra la pagina", "visita"
- URL esplicito fornito dall'utente

**Per `document_view`:**
- "apri il file", "visualizza", "mostrami"
- Con percorso file specificato

---

## Task Multi-Step Complessi

L'assistente può combinare più tool per completare task articolati:

### Esempio 1: Pianificazione Viaggio
```
Utente: "Voglio andare a Roma questo weekend, trovami info meteo e indicazioni da Milano"

Step 1: web_search
Parameters: { "query": "meteo Roma weekend" }

Step 2: map_open
Parameters: { "location": "Roma, Italia", "mode": "directions" }

Output: "Ho aperto due finestre: una con il meteo di Roma e una con le indicazioni da Milano a Roma"
```

### Esempio 2: Ricerca e Apprendimento
```
Utente: "Voglio imparare Docker, cerca tutorial e aprimi la documentazione ufficiale"

Step 1: youtube_search
Parameters: { "query": "Docker tutorial italiano" }

Step 2: browser_open
Parameters: { "url": "https://docs.docker.com", "description": "documentazione Docker" }

Output: "Ho aperto YouTube con tutorial Docker e la documentazione ufficiale"
```

### Esempio 3: Analisi File + Web
```
Utente: "Leggi il file report.txt e poi cerca online informazioni su ciò che contiene"

Step 1: file_read
Parameters: { "path": "/home/user/report.txt" }

Step 2: web_search
Parameters: { "query": "[contenuto estratto dal file]" }

Output: "Ho letto il report e aperto una ricerca con le informazioni trovate"
```

---

## Linee Guida per l'Assistente

### Quando Usare Tool Web

1. **Priorità Browser vs Comandi Shell:**
   - ✅ Usa tool web per visualizzazioni, ricerche, mappe
   - ❌ Non usare `shell_execute` per aprire browser manualmente

2. **Informazioni in Tempo Reale:**
   - Meteo, borsa, notizie → Sempre `web_search`
   - Non cercare di generare info obsolete

3. **Multimodalità:**
   - Per video, immagini, grafici → Usa tool appropriati
   - Non descrivere testualmente quando puoi mostrare

4. **Esperienza Utente:**
   - Spiega cosa stai per aprire prima di farlo
   - Conferma dopo apertura
   - Offri alternative se qualcosa non funziona

---

## Sicurezza

### Validazione URL
- Solo `http://` e `https://` supportati
- URL validati prima dell'apertura
- Protezione contro URL malevoli

### Nessuna Conferma Richiesta
I tool web **NON** richiedono conferma perché:
- Non modificano file di sistema
- Non eseguono codice
- Aprono solo contenuti nel browser
- Operazioni reversibili (chiusura tab)

### Limitazioni
- Non può interagire con pagine aperte
- Non può leggere contenuto dopo apertura
- Non può compilare form automaticamente
- Solo apertura URL, non automazione browser

---

## Esempi Pratici Completi

### Esempio 1: Ricerca Ristorante
```
Utente: "Trova un ristorante italiano vicino a Piazza del Duomo a Milano e mostrami come arrivarci"

Assistente risponde: "Ti aiuto subito! Cercherò ristoranti italiani in zona Duomo."

Tool 1: map_open
{
  "location": "ristorante italiano Piazza Duomo Milano",
  "mode": "search"
}

Risultato: "✅ Google Maps aperto con ristoranti italiani vicino a Piazza Duomo"

Assistente: "Ho aperto Google Maps con i ristoranti italiani nella zona. Puoi vedere le recensioni, orari e cliccare su uno per le indicazioni stradali!"
```

### Esempio 2: Apprendimento Argomento
```
Utente: "Voglio imparare machine learning, serve un tutorial video e la documentazione"

Assistente: "Perfetto! Ti preparo risorse per imparare machine learning."

Tool 1: youtube_search
{
  "query": "machine learning tutorial italiano principianti"
}

Tool 2: browser_open
{
  "url": "https://scikit-learn.org/stable/tutorial/index.html",
  "description": "tutorial scikit-learn"
}

Assistente: "Ho aperto:
1. YouTube con tutorial ML in italiano
2. Documentazione ufficiale scikit-learn

Buono studio! Vuoi che ti cerchi anche risorse in inglese?"
```

### Esempio 3: Pianificazione Evento
```
Utente: "Devo organizzare un evento a Torino, mostrami dove si trova il luogo e cerca catering nella zona"

Assistente: "Ti aiuto con l'organizzazione dell'evento a Torino."

Tool 1: map_open
{
  "location": "Torino, Italia"
}

Tool 2: web_search
{
  "query": "servizi catering Torino eventi"
}

Assistente: "Ho aperto:
- Mappa di Torino per vedere la zona
- Ricerca di servizi catering locali

Hai già un indirizzo specifico per l'evento?"
```

---

## Troubleshooting

### "Browser non si apre"
- Verifica che ci sia un browser predefinito configurato
- Su Linux: potrebbe servire variabile BROWSER
- Prova tool alternativi se uno fallisce

### "URL non valido"
- Assicurati che inizi con http:// o https://
- Controlla spelling del dominio
- Usa `web_search` se non conosci URL esatto

### "File non trovato per document_view"
- Verifica percorso con `file_list` prima
- Usa percorsi assoluti
- Controlla permessi file

---

## Best Practices

1. **Combina Tool:** Usa più tool in sequenza per task complessi
2. **Spiega Intenzioni:** Dì all'utente cosa stai per fare
3. **Gestisci Errori:** Offri alternative se un tool fallisce
4. **Context Aware:** Ricorda cosa hai già aperto nella sessione
5. **User-Centric:** Chiedi conferma per azioni ambigue

---

## Prossimi Miglioramenti (Roadmap)

- [ ] Web scraping per estrarre contenuti
- [ ] Interazione con pagine aperte (via browser extension)
- [ ] Screenshot di pagine web
- [ ] Compilazione form automatica
- [ ] Download file da URL
- [ ] Bookmark e gestione cronologia

---

**I tool web trasformano MatePro in un vero assistente multimodale capace di navigare il web e aiutare con task complessi!** 🚀🌐
