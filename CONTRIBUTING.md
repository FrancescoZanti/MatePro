# Guida alla Contribuzione

Grazie per il tuo interesse nel contribuire a MatePro! 🎉

Questo documento fornisce le linee guida per contribuire al progetto in modo sicuro ed efficace.

## 📋 Indice

- [Codice di Condotta](#codice-di-condotta)
- [Come Iniziare](#come-iniziare)
- [Processo di Sviluppo](#processo-di-sviluppo)
- [Standard di Codice](#standard-di-codice)
- [Sicurezza](#sicurezza)
- [Comunicazione](#comunicazione)

## 🤝 Codice di Condotta

Partecipando a questo progetto, ti impegni a:

- Essere rispettoso e inclusivo verso tutti
- Accettare critiche costruttive
- Concentrarti su ciò che è meglio per la community
- Mostrare empatia verso gli altri membri della community

## 🚀 Come Iniziare

### Prerequisiti

Assicurati di avere installato:

- [Rust](https://rustup.rs/) 1.70 o superiore
- [Git](https://git-scm.com/)
- [Ollama](https://ollama.ai/) (per testare l'applicazione)

### Setup Ambiente di Sviluppo

```bash
# 1. Fork il repository su GitHub

# 2. Clona il tuo fork
git clone https://github.com/TUO_USERNAME/MatePro.git
cd MatePro

# 3. Aggiungi il repository upstream
git remote add upstream https://github.com/FrancescoZanti/MatePro.git

# 4. Installa le dipendenze e compila
cargo build

# 5. Esegui l'applicazione in modalità debug
cargo run

# 6. (Opzionale) Installa strumenti di sviluppo
rustup component add clippy rustfmt
```

## 🔄 Processo di Sviluppo

### 1. Trova o Crea un Issue

- Controlla gli [issue aperti](https://github.com/FrancescoZanti/MatePro/issues)
- Se vuoi lavorare su un issue esistente, commenta per far sapere agli altri
- Per nuove funzionalità, apri prima un issue per discuterne

### 2. Crea un Branch

Usa nomi descrittivi seguendo questa convenzione:

```bash
# Feature (nuova funzionalità)
git checkout -b feature/aggiungi-supporto-immagini

# Bugfix
git checkout -b fix/risolvi-crash-caricamento-pdf

# Documentazione
git checkout -b docs/aggiorna-guida-installazione

# Refactoring
git checkout -b refactor/ottimizza-client-ollama

# Performance
git checkout -b perf/migliora-velocita-rendering
```

### 3. Sviluppa

- Mantieni i cambiamenti focalizzati e atomici
- Testa frequentemente durante lo sviluppo
- Scrivi codice idiomatico Rust
- Commenta parti complesse del codice

### 4. Testa

```bash
# Compila e verifica errori
cargo build

# Esegui test
cargo test

# Verifica con clippy
cargo clippy -- -D warnings

# Formatta il codice
cargo fmt

# Test funzionale manuale
cargo run --release
```

### 5. Commit

Usa [Conventional Commits](https://www.conventionalcommits.org/):

```bash
# Formato: <tipo>(<scope opzionale>): <descrizione>

# Esempi
git commit -m "feat(ui): aggiungi dark mode automatico"
git commit -m "fix(pdf): gestisci correttamente file corrotti"
git commit -m "docs: aggiorna README con nuove funzionalità"
git commit -m "refactor(client): estrai logica di retry in funzione separata"
git commit -m "perf(network): ottimizza scansione con async/await"
git commit -m "test: aggiungi test per parsing Excel"
```

**Tipi di commit:**

- `feat`: Nuova funzionalità
- `fix`: Correzione bug
- `docs`: Modifiche alla documentazione
- `style`: Modifiche che non influenzano il significato (formattazione, spazi, ecc.)
- `refactor`: Modifica del codice che non aggiunge funzionalità né corregge bug
- `perf`: Miglioramento delle performance
- `test`: Aggiunta o correzione di test
- `build`: Modifiche al sistema di build o dipendenze
- `ci`: Modifiche ai file di CI
- `chore`: Altre modifiche che non cambiano src o test

### 6. Sincronizza

Prima di pushare, sincronizza con upstream:

```bash
# Scarica le ultime modifiche
git fetch upstream

# Rebase sul branch master upstream
git rebase upstream/master

# Se ci sono conflitti, risolvili e continua
git add .
git rebase --continue
```

### 7. Push e Pull Request

```bash
# Pusha sul tuo fork
git push origin nome-del-tuo-branch
```

Su GitHub:

1. Vai al repository principale
2. Clicca su **"New Pull Request"**
3. Seleziona il tuo branch
4. Compila il template:
   - Titolo descrittivo
   - Descrizione dettagliata delle modifiche
   - Link all'issue correlato (se presente)
   - Screenshot per modifiche UI
   - Note sui breaking changes
   - Checklist completata

## 📐 Standard di Codice

### Stile Rust

- Segui le [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Usa `cargo fmt` per formattare automaticamente
- Risolvi tutti i warning di `cargo clippy`
- Preferisci soluzioni idiomatiche Rust

### Naming Conventions

```rust
// Costanti: UPPER_SNAKE_CASE
const MAX_RETRY_ATTEMPTS: u32 = 3;


// Tipi: PascalCase
struct UserMessage { }
enum AppState { }

// Funzioni e variabili: snake_case
fn send_message(input_text: &str) { }
let available_models = vec![];

// Moduli: snake_case
mod network_scanner;
```

### Documentazione

Documenta le API pubbliche:

```rust
/// Invia un messaggio al modello LLM.
///
/// # Arguments
///
/// * `model` - Nome del modello Ollama da usare
/// * `messages` - Slice di messaggi della conversazione
///
/// # Returns
///
/// Ritorna la risposta del modello o un errore
///
/// # Errors
///
/// Può fallire se la connessione a Ollama non è disponibile
pub async fn chat(&self, model: &str, messages: &[Message]) -> Result<String> {
    // implementazione
}
```

### Gestione Errori

Usa `Result<T, E>` e `anyhow::Result` appropriatamente:

```rust
use anyhow::{Context, Result};

fn load_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .context("Impossibile leggere il file")?;
}
```

## 🔒 Sicurezza

### Cosa NON Fare

❌ **NON** committare:
- Credenziali, password, token
- API keys o secrets
- Dati personali o sensibili
- File di configurazione con informazioni private
- Indirizzi IP o hostname privati

### Best Practices

✅ **DA FARE**:
- Usa variabili d'ambiente per configurazioni sensibili
- Valida sempre l'input dell'utente
- Gestisci gli errori in modo sicuro senza esporre dettagli interni
- Usa dipendenze da fonti verificate
- Mantieni le dipendenze aggiornate
- Testa con dati di esempio realistici ma non sensibili

### Segnalare Vulnerabilità

**🚨 IMPORTANTE**: Non aprire issue pubblici per vulnerabilità di sicurezza.

**Procedura:**

1. Invia una email privata a: **me@francescozanti.dev**
2. Oggetto: `[SECURITY] Vulnerabilità in MatePro`
3. Includi:
   - Descrizione dettagliata
   - Passi per riprodurre
   - Impatto potenziale
   - Versione affetta
   - Suggerimenti per la fix (opzionale)

Riceverai una risposta entro **48 ore** e collaboreremo per risolvere il problema prima della divulgazione pubblica.

## 🧪 Testing

### Eseguire i Test

```bash
# Tutti i test
cargo test

# Test specifici
cargo test nome_test

# Con output verboso
cargo test -- --nocapture

# Test di integrazione
cargo test --test integration_tests
```

### Scrivere Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_info() {
        let json = r#"{"name":"llama2","size":4000000000}"#;
        let model = parse_model(json).unwrap();
        assert_eq!(model.name, "llama2");
        assert_eq!(model.size, 4000000000);
    }
}
```

## 📢 Comunicazione

### Dove Chiedere Aiuto

- **GitHub Issues**: Bug report, richieste di funzionalità
- **GitHub Discussions**: Domande generali, idee
- **Email**: Vulnerabilità di sicurezza, questioni private

### Tempi di Risposta

- Issue e PR: generalmente entro 1-3 giorni
- Security issues: entro 48 ore
- Discussions: quando possibile

## ✅ Checklist PR

Prima di aprire una Pull Request, verifica:

- [ ] Il codice compila senza errori (`cargo build`)
- [ ] Tutti i test passano (`cargo test`)
- [ ] Nessun warning da clippy (`cargo clippy -- -D warnings`)
- [ ] Codice formattato (`cargo fmt --check`)
- [ ] Documentazione aggiornata se necessario
- [ ] Commit seguono Conventional Commits
- [ ] Branch sincronizzato con `upstream/master`
- [ ] Screenshot inclusi per modifiche UI
- [ ] Nessuna informazione sensibile nel codice
- [ ] Issue correlato linkato nella descrizione PR

## 🎯 Aree di Contribuzione

### Priorità Alta

- 🐛 Risoluzione bug segnalati
- 📝 Miglioramento documentazione
- 🧪 Aggiunta test

### Contributi Benvenuti

- ✨ Nuove funzionalità (discuti prima nell'issue)
- 🌍 Internazionalizzazione (i18n)
- ⚡ Ottimizzazioni performance
- 🎨 Miglioramenti UI/UX
- 📦 Supporto nuovi formati file
- 🔌 Integrazione con altri servizi

### Idee Future

- Export conversazioni in PDF/HTML
- Supporto plugin
- Modalità offline con cache
- Sincronizzazione cloud (opzionale)
- Keyboard shortcuts personalizzabili

## 📚 Risorse Utili

- [Documentazione Rust](https://doc.rust-lang.org/)
- [egui Documentation](https://docs.rs/egui/)
- [Ollama API](https://github.com/ollama/ollama/blob/main/docs/api.md)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitHub Flow](https://guides.github.com/introduction/flow/)

## 🙏 Riconoscimenti

Grazie a tutti i contributori che aiutano a migliorare MatePro! Il tuo contributo, grande o piccolo, è prezioso per la community.

---

**Domande?** Non esitare a chiedere! Siamo qui per aiutarti. 💪
