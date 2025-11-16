# GitHub Actions Release Workflow

Questo workflow automatizza la creazione di release multi-piattaforma per MatePro.

## Come funziona

Quando viene pushato un tag che inizia con `v` (es: `v0.1.0`), il workflow:

1. **Crea una release** su GitHub
2. **Compila** per tutte le piattaforme in parallelo
3. **Genera pacchetti** per ogni sistema operativo
4. **Carica automaticamente** tutti gli asset nella release

## Come creare una release

```bash
# 1. Aggiorna la versione in Cargo.toml
# version = "0.1.0"

# 2. Committa le modifiche
git add Cargo.toml
git commit -m "Bump version to 0.1.0"

# 3. Crea il tag
git tag v0.1.0

# 4. Pusha tag e commit
git push origin main
git push origin v0.1.0
```

## Pacchetti generati

### Windows
- **matepro-windows-x64.zip** - Eseguibile per Windows 10/11

### macOS
- **matepro-macos-universal.dmg** - DMG universale (Intel + Apple Silicon)

### Linux
- **matepro-linux-x64.tar.gz** - Binario generico
- **matepro-linux-amd64.deb** - Debian/Ubuntu/Mint
- **matepro-linux-x86_64.rpm** - Fedora/RHEL/CentOS/openSUSE

## Personalizzazione

Prima di rilasciare, aggiorna in `Cargo.toml`:

```toml
[package]
authors = ["Il Tuo Nome <tua.email@esempio.com>"]
repository = "https://github.com/tuousername/matepro"
homepage = "https://github.com/tuousername/matepro"

[package.metadata.deb]
maintainer = "Il Tuo Nome <tua.email@esempio.com>"
copyright = "2025, Il Tuo Nome <tua.email@esempio.com>"
```

## Note

- Il workflow richiede che il repository sia pubblico o che tu abbia GitHub Actions abilitato
- I build richiedono circa 10-15 minuti in totale
- Puoi monitorare il progresso nella tab "Actions" del repository
