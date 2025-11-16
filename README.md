# MatePro

Applicazione Rust con interfaccia grafica per chattare con modelli LLM tramite Ollama.

## Prerequisiti

- Rust installato (https://rustup.rs/)
- Ollama installato e in esecuzione (https://ollama.ai/)
- Almeno un modello scaricato (es: `ollama pull llama2`)

## Installazione

```bash
cargo build --release
```

## Utilizzo

```bash
cargo run --release
```

Oppure dopo la compilazione:

```bash
./target/release/matepro
```

## Funzionalità

- 🔍 **Scansione automatica della rete** per trovare server Ollama disponibili
- 🎨 **Interfaccia grafica moderna** con design elegante in stile Apple
- 🌓 **Tema chiaro/scuro** adattivo alle preferenze di sistema
- 💬 **Chat conversazionale** con bolle messaggi stile iMessage
- 🔌 **Connessione a istanze Ollama** locali o remote
- 🤖 **Selezione interattiva** del modello tra quelli disponibili
- 📝 **Area di input spaziosa** con supporto multilinea
- ⌨️ **Scorciatoie da tastiera** (Ctrl+Enter per inviare)

## Esempio d'uso

1. Avvia Ollama: `ollama serve`
2. Esegui MatePro: `cargo run --release`
3. L'app scansionerà automaticamente la rete per trovare server Ollama
4. Seleziona un server dalla lista o inserisci un URL personalizzato
5. Scegli un modello dalla lista
6. Inizia a chattare!

## Dipendenze

- `eframe` / `egui` - Framework per interfaccia grafica
- `reqwest` - Client HTTP per comunicare con l'API Ollama
- `serde` / `serde_json` - Serializzazione/deserializzazione JSON
- `tokio` - Runtime asincrono
- `anyhow` - Gestione errori semplificata
- `poll-promise` - Gestione chiamate asincrone nell'UI
- `local-ip-address` - Rilevamento IP locale per scansione rete

## Release

Per creare una nuova release:

1. Aggiorna la versione in `Cargo.toml`
2. Committa le modifiche: `git commit -am "Release v0.1.0"`
3. Crea un tag: `git tag v0.1.0`
4. Pusha il tag: `git push origin v0.1.0`

GitHub Actions creerà automaticamente:
- 📦 Binario Windows (ZIP)
- 🍎 DMG universale per macOS (Intel + Apple Silicon)
- 🐧 Binario Linux (tar.gz)
- 📦 Pacchetto DEB per Debian/Ubuntu
- 📦 Pacchetto RPM per Fedora/RHEL/CentOS

## Licenza

MIT License - vedi file [LICENSE](LICENSE)
