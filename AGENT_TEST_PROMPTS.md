# Esempi di Prompt per Testare la Modalità Agente

## Test Basici

### Test 1: Informazioni Sistema
```
Dammi informazioni dettagliate sul sistema: CPU, RAM, processi attivi
```

**Tool attesi:** `system_info`, `process_list`

### Test 2: Esplorazione Filesystem
```
Mostrami tutti i file nella directory /tmp
```

**Tool attesi:** `file_list`

### Test 3: Lettura File
```
Leggi il contenuto del file /etc/hostname e dimmi qual è il nome del computer
```

**Tool attesi:** `file_read`

## Test Intermedi

### Test 4: Analisi Processi
```
Trova i 5 processi che stanno usando più memoria e dimmi quanto RAM stanno consumando
```

**Tool attesi:** `process_list` + analisi

### Test 5: Ricerca File
```
Cerca tutti i file .txt nella mia home directory (ricorsivamente) e dimmi quanti ne hai trovati
```

**Tool attesi:** `file_list` con recursive=true

### Test 6: Creazione File Semplice
```
Crea un file di testo in /tmp chiamato test.txt con il contenuto "Hello from MatePro Agent!"
```

**Tool attesi:** `file_write` (richiede conferma)

## Test Avanzati

### Test 7: Analisi Sistema Completa
```
Fai un'analisi completa del sistema: info hardware, uso memoria, processi top 10, e spazio disco disponibile
```

**Tool attesi:** `system_info`, `process_list`, `shell_execute df -h`

### Test 8: Backup Files
```
Crea un backup di tutti i file .md in questa directory concatenandoli in un unico file /tmp/backup.md
```

**Tool attesi:** `file_list`, `file_read` (multipli), `file_write`

### Test 9: Report Sistema
```
Genera un report testuale completo sullo stato del sistema e salvalo in /tmp/system_report.txt. 
Include: nome sistema, kernel, CPU, RAM, top 10 processi per memoria.
```

**Tool attesi:** `system_info`, `process_list`, `file_write`

### Test 10: Comandi Shell Multipli
```
Verifica se Python è installato, quale versione è, e mostrami dove si trova l'eseguibile
```

**Tool attesi:** `shell_execute` (multipli: python3 --version, which python3)

## Test di Sicurezza

### Test 11: Conferma Tool Pericoloso
```
Esegui il comando 'ls -la /home' e mostrami il risultato
```

**Comportamento atteso:** Richiesta di conferma prima dell'esecuzione

### Test 12: Scrittura File con Conferma
```
Scrivi un file /tmp/dangerous.txt con il contenuto "Testing security"
```

**Comportamento atteso:** Richiesta di conferma prima della scrittura

### Test 13: Operazioni Multiple con Conferme
```
Crea 3 file di test in /tmp (test1.txt, test2.txt, test3.txt) ognuno con contenuti diversi
```

**Comportamento atteso:** Richiesta conferma per ogni file_write

## Test Edge Cases

### Test 14: File Non Esistente
```
Leggi il contenuto del file /tmp/questo_file_non_esiste.txt
```

**Comportamento atteso:** Messaggio di errore gestito

### Test 15: Directory Non Esistente
```
Lista i file nella directory /questa/directory/non/esiste
```

**Comportamento atteso:** Messaggio di errore gestito

### Test 16: Comando Shell Invalido
```
Esegui il comando 'comandochenonesiste123'
```

**Comportamento atteso:** Errore command not found gestito

## Test Ciclo Agentico

### Test 17: Task Multi-Step
```
Trova il processo che usa più CPU, salvalo in un file /tmp/top_cpu.txt, 
poi leggimi il contenuto del file per confermarlo
```

**Iterazioni attese:** 3-4 (process_list → analisi → file_write → file_read)

### Test 18: Iterazioni Progressive
```
Analizza il sistema, poi crea un report, poi dimmi se il report è stato creato correttamente
```

**Iterazioni attese:** 3-5 (system_info → file_write → file_list o file_read)

### Test 19: Auto-Correzione
```
Crea un file in /tmp/test.txt, poi modificalo aggiungendo una riga, 
poi leggilo per verificare
```

**Iterazioni attese:** 4-5 (file_write → file_read → file_write → file_read)

## Test Performance

### Test 20: Grande Output
```
Lista tutti i file ricorsivamente dalla directory /usr (attenzione: molto output)
```

**Note:** Test limite output tool, gestione memoria

### Test 21: Molti Processi
```
Analizza tutti i processi in esecuzione e creami un report dettagliato
```

**Note:** Test parsing grande quantità dati

## Come Testare

1. **Avvia MatePro**
2. **Attiva Modalità Agente** (toggle nell'header)
3. **Copia e incolla** uno dei prompt sopra
4. **Osserva:**
   - Parsing dei tool calls
   - Richieste di conferma
   - Risultati visualizzati
   - Gestione errori

## Modelli Consigliati

Per testing ottimale, usa modelli con buona comprensione delle istruzioni:

- `llama3.2:latest` - Buono per task semplici
- `llama3.1:8b` - Bilanciato 
- `mixtral:latest` - Ottimo per task complessi
- `qwen2.5:latest` - Eccellente comprensione tool calling

## Troubleshooting

**LLM non usa i tool?**
- Verifica che la modalità agente sia attiva (toggle verde)
- Prova con un prompt più esplicito: "Usa i tool disponibili per..."
- Alcuni modelli richiedono esempi più chiari

**Tool non viene eseguito?**
- Controlla se richiede conferma (⚠️)
- Verifica il formato JSON nella risposta
- Guarda i messaggi di errore nella chat

**Troppe/Poche iterazioni?**
- Modifica `max_agent_iterations` nel codice se necessario
- Task complessi potrebbero richiedere più iterazioni
- Semplifica il task se raggiunge sempre il limite

## Note di Sicurezza per il Testing

⚠️ **ATTENZIONE durante il testing:**

1. Usa sempre directory di test come `/tmp`
2. Non testare comandi distruttivi (`rm -rf`, `dd`, etc.)
3. Monitora le conferme richieste
4. Fai backup prima di test su file importanti
5. Usa una VM o container per test avanzati

## Metriche di Successo

Un test è riuscito se:
- ✅ Tool corretti vengono chiamati
- ✅ Parametri JSON sono validi
- ✅ Risultati sono mostrati correttamente
- ✅ Errori sono gestiti senza crash
- ✅ Conferme appaiono per tool pericolosi
- ✅ Ciclo agentico completa entro iterazioni limite
