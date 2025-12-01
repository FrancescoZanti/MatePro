# Test Prompts - Tool Web e Browser

## Panoramica

Questa guida contiene esempi di prompts per testare i nuovi tool web e browser della modalità agente.

---

## Test Basici - Singolo Tool

### Test 1: Apertura Browser Semplice
```
Apri il sito di GitHub
```
**Tool atteso:** `browser_open`  
**Risultato:** Browser aperto su https://github.com

### Test 2: Ricerca Web Base
```
Cerca informazioni su Rust programming
```
**Tool atteso:** `web_search`  
**Risultato:** Google search aperto con query

### Test 3: Mappa Località
```
Mostrami Milano sulla mappa
```
**Tool atteso:** `map_open`  
**Risultato:** Google Maps con Milano

### Test 4: YouTube Ricerca
```
Cerca tutorial Python per principianti
```
**Tool atteso:** `youtube_search`  
**Risultato:** YouTube search page aperta

### Test 5: Lettura File
```
Leggi il contenuto del file /tmp/test.txt
```
**Tool atteso:** `file_read`  
**Risultato:** Contenuto file mostrato

---

## Test Intermedi - Riconoscimento Contesto

### Test 6: Meteo (Comprensione Implicita)
```
Che tempo fa a Roma?
```
**Tool atteso:** `web_search` con query "meteo Roma"  
**Ragionamento:** Info in tempo reale richiede ricerca web

### Test 7: Indicazioni Stradali
```
Come arrivo a Firenze da Milano?
```
**Tool atteso:** `map_open` con mode="directions"  
**Ragionamento:** Richiesta di percorso

### Test 8: Ricerca Locale
```
Trova ristoranti giapponesi vicino a Piazza Duomo
```
**Tool atteso:** `map_open` con location specifica  
**Ragionamento:** Ricerca geografica locale

### Test 9: Tutorial Video
```
Voglio imparare Docker, mostrami dei video
```
**Tool atteso:** `youtube_search` con "Docker tutorial"  
**Ragionamento:** Richiesta video esplicita

### Test 10: Notizie Attuali
```
Quali sono le ultime notizie sulla tecnologia?
```
**Tool atteso:** `web_search` con "ultime notizie tecnologia"  
**Ragionamento:** Notizie = informazione in tempo reale

---

## Test Avanzati - Multi-Step e Azioni Complesse

### Test 11: Pianificazione Viaggio
```
Voglio visitare Venezia questo weekend. Mostrami il meteo, come arrivarci da Milano e cosa vedere
```
**Tool attesi:**
1. `web_search` → "meteo Venezia weekend"
2. `map_open` → Venezia con mode="directions"
3. `web_search` → "cosa vedere Venezia"

**Iterazioni:** 3-4

### Test 12: Ricerca Multi-Sorgente
```
Sto cercando informazioni su machine learning: serve documentazione ufficiale e tutorial video
```
**Tool attesi:**
1. `browser_open` → Documentazione scikit-learn o TensorFlow
2. `youtube_search` → "machine learning tutorial"

**Iterazioni:** 2-3

### Test 13: Analisi Locale + Web
```
Cerca tutti i file PDF nella mia home, poi leggine il contenuto e cercane informazioni online
```
**Tool attesi:**
1. `file_list` → Ricerca ricorsiva PDF
2. `file_read` → Lettura primo PDF
3. `web_search` → Ricerca contenuto trovato

**Iterazioni:** 3-4

### Test 14: Evento e Logistica
```
Devo organizzare una conferenza a Torino. Serve la mappa della città, catering locale e hotel nelle vicinanze
```
**Tool attesi:**
1. `map_open` → "Torino, Italia"
2. `web_search` → "catering Torino eventi"
3. `web_search` → "hotel Torino centro"

**Iterazioni:** 3-4

### Test 15: Apprendimento Argomento
```
Voglio studiare React JS. Mostrami il sito ufficiale, tutorial video in italiano e articoli recenti
```
**Tool attesi:**
1. `browser_open` → https://react.dev
2. `youtube_search` → "React tutorial italiano"
3. `web_search` → "React articoli 2024"

**Iterazioni:** 3

---

## Test Comprensione Linguaggio Naturale

### Test 16: Richiesta Ambigua
```
Ho fame
```
**Comportamento atteso:** Chiede località o usa geolocalizzazione  
**Tool possibile:** `map_open` con "ristoranti vicino a me"

### Test 17: Richiesta Implicita
```
Mostrami Parigi
```
**Interpretazioni valide:**
- `map_open` → Mappa di Parigi
- `web_search` → Informazioni su Parigi
- `youtube_search` → Video di Parigi

**Migliore:** Chiede chiarimento all'utente

### Test 18: Comando Colloquiale
```
Fammi vedere quel video divertente dei gatti
```
**Tool atteso:** `youtube_search` con "video divertenti gatti"  
**Ragionamento:** "Fammi vedere" + "video" = YouTube

### Test 19: Richiesta Formale vs Informale
```
A) "Si prega di aprire il browser con Wikipedia"
B) "Apri Wikipedia"
```
**Tool atteso:** `browser_open` per entrambi  
**Ragionamento:** Stesso intent, diversa formalità

### Test 20: Multi-Lingua
```
Show me the weather in London
```
**Tool atteso:** `web_search` con "weather London"  
**Ragionamento:** Funziona anche con prompt in inglese

---

## Test Edge Cases

### Test 21: URL Invalido
```
Apri il sito htp://esempio.it
```
**Comportamento atteso:** Errore "URL non valido"  
**Gestione:** Suggerisce correzione o usa web_search

### Test 22: Località Inesistente
```
Mostrami sulla mappa Atlantide
```
**Comportamento atteso:** Google Maps aperto, nessun risultato  
**Gestione:** Normale, Maps gestisce errore

### Test 23: Ricerca Vuota
```
Cerca su Google
```
**Comportamento atteso:** Chiede cosa cercare  
**Gestione:** Richiede parametro mancante

### Test 24: File Inesistente
```
Apri il file /this/does/not/exist.pdf
```
**Comportamento atteso:** Errore "File non trovato"  
**Gestione:** Suggerisce file_list per trovare il file

### Test 25: Troppi Tool in Sequenza
```
Cerca X, poi Y, poi Z, poi apri A, B, C, visualizza 1, 2, 3
```
**Comportamento atteso:** Limite iterazioni (5 max)  
**Gestione:** Esegue primi 5, avvisa del limite

---

## Test Sicurezza e Permessi

### Test 26: Nessuna Conferma per Web
```
Apri 10 finestre browser con siti diversi
```
**Comportamento atteso:** Apre tutti senza conferma  
**Ragionamento:** Tool web non sono pericolosi

### Test 27: Browser + Shell (Mix)
```
Apri GitHub e poi elimina tutti i file in /tmp con rm
```
**Comportamento atteso:**
- `browser_open` → Nessuna conferma
- `shell_execute` → **Richiede conferma**

### Test 28: Documenti Sensibili
```
Leggi il file /etc/passwd
```
**Comportamento atteso:** Legge se permessi OK  
**Nota:** file_read non è pericoloso, solo visualizza contenuto

---

## Test Performance e Limiti

### Test 29: Molte Ricerche Parallele
```
Cerca contemporaneamente: meteo, notizie, borsa, sport, tech
```
**Comportamento atteso:** Una ricerca alla volta (sequenziale)  
**Nota:** Tool eseguiti in sequenza, non parallelo

### Test 30: URL Molto Lungo
```
Apri https://example.com/path/very/long/url/with/many/parameters?a=1&b=2&c=3...
```
**Comportamento atteso:** Funziona se URL valido  
**Limite:** Lunghezza URL del browser

---

## Test Casi d'Uso Reali

### Test 31: Sviluppatore Cerca Errore
```
Ho un errore "NullPointerException in Java", aiutami a risolverlo
```
**Tool attesi:**
1. `web_search` → "NullPointerException Java come risolvere"
2. Opzionale: `browser_open` → StackOverflow

### Test 32: Studente Prepara Esame
```
Devo studiare la Rivoluzione Francese. Serve video documentario e articoli accademici
```
**Tool attesi:**
1. `youtube_search` → "Rivoluzione Francese documentario"
2. `web_search` → "Rivoluzione Francese articoli accademici"

### Test 33: Turista Esplora Città
```
Sono a Roma, dove posso mangiare bene vicino al Colosseo?
```
**Tool attesi:**
1. `map_open` → "ristoranti Colosseo Roma"

### Test 34: Designer Cerca Ispirazione
```
Mostrami esempi di UI design moderno e palettes colori
```
**Tool attesi:**
1. `web_search` → "modern UI design examples"
2. `web_search` → "color palettes 2024"

### Test 35: Musicista Cerca Accordi
```
Voglio imparare "Wonderwall" alla chitarra
```
**Tool attesi:**
1. `web_search` → "Wonderwall guitar chords"
2. `youtube_search` → "Wonderwall guitar tutorial"

---

## Metriche di Successo

Un test è riuscito se:

✅ **Tool Corretti:** Tool giusti chiamati per il contesto  
✅ **Parametri Validi:** JSON ben formato con valori corretti  
✅ **Browser Aperto:** Finestra/tab effettivamente aperte  
✅ **Contenuto Pertinente:** Pagine mostrano info richieste  
✅ **Gestione Errori:** Errori gestiti con messaggi chiari  
✅ **UX Fluida:** Utente non confuso, tutto chiaro  

---

## Come Testare

1. **Avvia MatePro** con Ollama in esecuzione
2. **Attiva Modalità Agente** (toggle verde)
3. **Copia prompt** da questa guida
4. **Osserva:**
   - Tool chiamati (visibili nei log)
   - Browser aperto
   - Contenuto mostrato
   - Errori o anomalie
5. **Valuta risultato** secondo metriche sopra

---

## Modelli Consigliati

Per testing ottimale dei tool web:

- **llama3.2:latest** - Buono per task semplici
- **llama3.1:8b** - Bilanciato, comprende contesto
- **mixtral:latest** - Eccellente per azioni complesse
- **qwen2.5:latest** - Ottimo riconoscimento intent
- **gemma2:latest** - Buone performance web tasks

---

## Troubleshooting

**Tool non chiamato?**
- Verifica modalità agente attiva
- Prova prompt più esplicito
- Controlla modello LLM usato

**Browser non si apre?**
- Verifica browser predefinito configurato
- Controlla permessi sistema
- Vedi log errori nella chat

**Contenuto sbagliato aperto?**
- Query troppo ambigua
- LLM ha interpretato male
- Riformula richiesta

**Troppe iterazioni?**
- Task troppo complesso
- Semplifica richiesta
- Suddividi in step separati

---

**Buon testing! 🧪🌐**
