# MatePro 🤖

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-blue.svg)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/FrancescoZanti/MatePro/releases)
[![Release](https://github.com/FrancescoZanti/MatePro/actions/workflows/release.yml/badge.svg)](https://github.com/FrancescoZanti/MatePro/actions/workflows/release.yml)

MatePro è un client desktop multipiattaforma per l'interazione con modelli LLM serviti da Ollama. L'applicazione combina un'interfaccia grafica curata con funzionalità agentiche complete, integrate da strumenti dedicati alla gestione del sistema, del web e dei dati aziendali.

> **Stato del progetto:** `v0.0.5-beta`. Il codice attivo risiede in `src-tauri/` (Tauri v2). Le directory `legacy-egui/` e `legacy-ui/` sono mantenute unicamente per archivio storico.

![MatePro Screenshot](.github/images/matepro-main.png)

## Panoramica

- Applicazione desktop per Windows, macOS e Linux basata su Tauri v2.
- Interfaccia conversazionale moderna con supporto Markdown, matematica e allegati.
- Modalità agente con strumenti specializzati per task locali e remoti.
- Controlli di sicurezza e tracciamento in tempo reale delle azioni.

## Architettura del Progetto

- `src-tauri/` – codice principale (frontend Tauri con backend Rust).
- `legacy-egui/` – implementazione precedente in egui (deprecata).
- `legacy-ui/` – asset HTML/CSS legacy (non più mantenuti).
- `packaging/` – materiale per la distribuzione dei pacchetti.

Le attività di sviluppo devono concentrarsi su `src-tauri/`.

## Funzionalità

- **Interfaccia utente**: layout ispirato a macOS, temi chiaro/scuro, chat a bolle, formattazione Markdown avanzata, anteprima allegati, timestamp e supporto multilinea.
- **Gestione conversazioni**: collegamento a istanze Ollama locali/remoto, selezione dinamica dei modelli con indicatore di carico, cronologia persistente e scorciatoie da tastiera.
- **Modalità agente di sistema**: esecuzione controllata di comandi shell, navigazione e modifica del filesystem, raccolta di metriche (CPU, RAM, processi), orchestrazione di task complessi.
- **Strumenti web e browser**: apertura di URL, ricerca Google, consultazione di Google Maps, ricerca YouTube, visualizzazione di documenti locali attraverso l'integrazione browser.
- **Tool MCP SQL Server**: connessione in sola lettura a SQL Server con autenticazione Windows/SQL, esecuzione di query, generazione report e supporto per credenziali di dominio.
- **Automazione avanzata**: loop agentico autonomo, riconoscimento di intenti complessi, gestione di più step operativi e richieste di conferma per azioni sensibili.
- **Sicurezza e osservabilità**: autorizzazioni granulari, log live, conferme esplicite per operazioni critiche e guida contestuale agli strumenti disponibili.

### Documentazione di dettaglio

- `AGENT_FEATURES.md` – panoramica completa della modalità agente.
- `AGENT_WEB_TOOLS.md` – guida agli strumenti web e browser.
- `MCP_SQL_GUIDE.md` – istruzioni per l'integrazione SQL Server.
- `AGENT_TEST_PROMPTS.md` e `AGENT_WEB_TEST_PROMPTS.md` – raccolte di prompt ed esempi di test.

## Requisiti di Sistema

- Toolchain Rust (consigliata l'installazione tramite <https://rustup.rs/>).
- Ollama attivo con almeno un modello scaricato (`ollama pull llama2`).
- Dipendenze GTK/WebKit su Linux. Per Debian/Ubuntu:

```bash
sudo apt-get install -y pkg-config build-essential \
   libgtk-3-dev libgdk-pixbuf2.0-dev libcairo2-dev libpango1.0-dev \
   libatk1.0-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
   libsoup-3.0-dev
```

## Installazione Rapida

### Download da GitHub Releases

1. Visitare la pagina delle release su [GitHub](https://github.com/FrancescoZanti/MatePro/releases).
2. Scaricare il pacchetto per la piattaforma desiderata (ZIP per Windows, DMG per macOS, tar.gz/DEB/RPM per Linux).
3. Verificare l'integrità del download (hash, firma digitale se disponibile).
4. Installare o estrarre il pacchetto rispettando le istruzioni del formato selezionato.
5. Su Linux, in caso di binario standalone, rendere eseguibile il file (`chmod +x matepro`) e avviare con `./matepro`.

### Installazione tramite Fedora COPR

1. Abilitare il repository dedicato:

```bash
sudo dnf copr enable frzzzzz19/MatePro
```

1. Installare il pacchetto:

```bash
sudo dnf install matepro
```

1. Avviare l'applicazione dal menu o tramite terminale (`matepro`).

Il repository COPR è mantenuto in allineamento con le release ufficiali e provvede automaticamente alle dipendenze necessarie in ambiente Fedora.

## Compilazione dai Sorgenti

```bash
git clone https://github.com/FrancescoZanti/MatePro.git
cd MatePro/src-tauri
cargo build --release
```

Il binario compilato è disponibile in `../target/release/matepro`.

## Esecuzione

```bash
# Avvio in modalità release
cargo run --release

# Con Tauri CLI (se installato)
cargo tauri dev

# Avvio diretto del binario compilato
./target/release/matepro
```

## Dipendenze Principali

- `tauri` v2 – framework applicativo multipiattaforma.
- `tauri-plugin-shell` – esecuzione di comandi shell.
- `tauri-plugin-opener` – apertura di URL e file locali.
- `reqwest` – client HTTP per interfacciarsi con Ollama.
- `serde`, `serde_json` – serializzazione e deserializzazione.
- `tokio` – runtime asincrono.
- `anyhow` – gestione avanzata degli errori.
- `local-ip-address` – rilevamento della rete locale.
- `tiberius` – driver SQL Server nativo.

## Procedura di Release

1. Aggiornare la versione in `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json`.
1. Committare le modifiche (esempio: `git commit -am "Release v0.1.0"`).
1. Creare il tag (`git tag v0.1.0`).
1. Pubblicare tag e commit (`git push origin v0.1.0`).

La pipeline GitHub Actions produce automaticamente i pacchetti per Windows, macOS (DMG universale), Linux (tar.gz, DEB, RPM) e l'APK Android (architetture arm64-v8a, armeabi-v7a, x86_64).

## Contribuire in Sicurezza

1. Effettuare il fork del repository e clonare la copia locale.
1. Creare un branch descrittivo (`feature/...`, `fix/...`, `docs/...`).
1. Installare e testare il progetto (`cargo build`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt`).
1. Stendere commit conformi a [Conventional Commits](https://www.conventionalcommits.org/).
1. Sincronizzarsi con upstream (`git fetch upstream && git rebase upstream/master`) e aprire una pull request completa di descrizione, motivazione, screenshot (per modifiche UI) e indicazione di eventuali breaking change.

### Linee guida di sicurezza

- Evitare il commit di credenziali, token o dati personali.
- Mantenere i commit compatti e ben focalizzati.
- Documentare le decisioni tecniche rilevanti.
- In caso di vulnerabilità, contattare privatamente [me@francescozanti.dev](mailto:me@francescozanti.dev) fornendo descrizione, passi di riproduzione, impatto e possibili mitigazioni. Il maintainer risponde entro 48 ore.

## Supporto e Domande

- Aprire una Discussion su GitHub per quesiti generali o proposte.
- Segnalare bug e richieste di funzionalità tramite issue.
- Scrivere a [me@francescozanti.dev](mailto:me@francescozanti.dev) per supporto dedicato o casi urgenti.

## Licenza

Il progetto è distribuito con licenza MIT. Il testo completo è disponibile nel file `LICENSE`.
