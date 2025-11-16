# Quick Start - MatePro Agent Mode

## Avvio Rapido

### 1. Prerequisiti
```bash
# Assicurati che Ollama sia in esecuzione
ollama serve

# Verifica che un modello sia scaricato
ollama list
# Se necessario, scarica un modello:
# ollama pull llama3.2
```

### 2. Avvio MatePro
```bash
cd /home/fzanti/RS-Agent
./target/release/matepro
```

### 3. Configurazione Base
1. L'app cerca automaticamente server Ollama sulla rete
2. Seleziona il server (default: localhost:11434)
3. Clicca "Connetti"
4. Scegli un modello dalla lista

### 4. Attiva Modalità Agente
- Trova il toggle **"🤖 Modalità Agente"** nell'header
- Clicca per attivarlo (diventa verde)
- L'assistente può ora usare tool per controllare il computer

## Esempi di Test Immediati

### Test 1: Info Sistema (Sicuro)
```
Dimmi le informazioni sul sistema: CPU, RAM e processi attivi
```

### Test 2: Lista File (Sicuro)
```
Mostrami tutti i file nella directory /tmp
```

### Test 3: Comando Shell (Richiede Conferma)
```
Esegui il comando 'ls -la /home' e mostrami il risultato
```
**Comportamento:** Apparirà finestra di conferma → Clicca "Consenti"

### Test 4: Creazione File (Richiede Conferma)
```
Crea un file /tmp/test_matepro.txt con il contenuto "Test successful!"
```
**Comportamento:** Apparirà finestra di conferma → Clicca "Consenti"

### Test 5: Task Multi-Step
```
Analizza il sistema, trova i 3 processi che usano più RAM, e salvali in /tmp/top_processes.txt
```
**Comportamento:** Multiple iterazioni automatiche, conferma per il file_write

## Cosa Osservare

### Indicatori UI
- 🟢 **Toggle verde**: Modalità agente attiva
- **(1/5)**: Contatore iterazioni agentic
- 🔧: Messaggi con risultati tool
- ⚡: "Sto pensando..." durante elaborazione

### Finestra Conferma
Appare per operazioni pericolose con:
- Nome del tool
- Parametri
- Pulsanti **Consenti** / **Annulla**

### Messaggi nella Chat
- **Messaggi utente**: Blu, a destra
- **Risposte AI**: Grigio, a sinistra
- **Risultati tool**: Con emoji 🔧
- **✅ Successo** / **❌ Errore**: Colorati

## Troubleshooting Rapido

### "L'agente non usa i tool"
✅ Verifica che il toggle sia **verde**
✅ Prova prompt più espliciti: "Usa i tool disponibili per..."
✅ Alcuni modelli funzionano meglio di altri (prova llama3.2, mixtral)

### "Tool non viene eseguito"
✅ Controlla se appare finestra di conferma
✅ Guarda eventuali messaggi di errore nella chat
✅ Verifica permessi sui file/directory

### "Troppo lento"
✅ Modelli più grandi sono più lenti
✅ Operazioni filesystem su directory grandi richiedono tempo
✅ Considera di usare modelli più piccoli per test

## Sicurezza

⚠️ **IMPORTANTE:**
- Modalità agente permette esecuzione codice
- **Conferma sempre** operazioni shell_execute
- **Conferma sempre** operazioni file_write
- Usa directory di test come `/tmp` per esperimenti
- Non testare comandi distruttivi

## Modelli Consigliati

### Per Test
- `llama3.2:latest` - Veloce, buono per test base
- `llama3.1:8b` - Bilanciato

### Per Produzione
- `mixtral:latest` - Eccellente comprensione tool
- `qwen2.5:latest` - Ottimo per tool calling

## Disattivare Modalità Agente

1. Clicca sul toggle **"🤖 Modalità Agente"** (diventa grigio)
2. L'assistente torna in modalità chat normale
3. Nessun tool verrà più chiamato

## Link Utili

- 📖 [Documentazione Completa](AGENT_FEATURES.md)
- 🧪 [Test Prompts Dettagliati](AGENT_TEST_PROMPTS.md)
- 📝 [Changelog](CHANGELOG.md)
- 🏠 [README Principale](README.md)

## Supporto

Problemi o domande?
- Issues: https://github.com/FrancescoZanti/MatePro/issues
- Email: me@francescozanti.dev

---

**Buon divertimento con MatePro Agent Mode! 🤖**
