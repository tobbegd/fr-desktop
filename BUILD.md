# Bygginstruktioner — fr-desktop

## Verktyg och beroenden

| Verktyg | Syfte | Installation |
|---------|-------|-------------|
| Rust + Cargo | Tauri backend + release-CLI | https://rustup.rs |
| bun | JS-pakethanterare och dev-server | https://bun.sh |
| Tauri CLI | Bygga och signera appen | `bun add -D @tauri-apps/cli` (i fr-app/) |

---

## Katalogstruktur

```
fr-desktop/
├── fr-app/          Svelte + Tauri-appen
│   ├── src/         Frontend (Svelte/TypeScript)
│   ├── src-tauri/   Rust backend
│   └── ...
├── fr-release/      Release-CLI (detta verktyg)
└── BUILD.md         Denna fil
```

---

## Utveckling (dev-läge)

```bash
cd fr-app
bun install
bunx tauri dev
```

Appen pekar mot `http://localhost:8081` (fr-web) i dev-läge.
För att peka mot annan server:

```bash
VITE_SERVER_URL=https://stage.foretagsdatabasen.se bunx tauri dev
```

---

## Signeringsnycklar (görs en gång)

Tauri kräver ett nyckelpar för att signera uppdateringar. Den privata nyckeln
används bara lokalt vid byggen och ska aldrig committas.

```bash
cd fr-app
bunx tauri signer generate -w ~/.tauri/fr-app.key --ci
```

Nyckelparet sparas i:
- `~/.tauri/fr-app.key`      — privat nyckel (håll hemlig)
- `~/.tauri/fr-app.key.pub`  — publik nyckel (läggs in i tauri.conf.json)

Kopiera innehållet i `.key.pub` till `fr-app/src-tauri/tauri.conf.json`:

```json
"plugins": {
  "updater": {
    "pubkey": "<innehållet i ~/.tauri/fr-app.key.pub>",
    "endpoints": ["https://foretagsdatabasen.se/desktop/latest.json"]
  }
}
```

---

## Nyckelhantering — använda på flera maskiner

Den privata signeringsnyckeln måste vara **samma** på alla maskiner du bygger från.
Om du genererar en ny nyckel på en ny maskin stämmer den inte mot den publika nyckel
som är inbakad i appen — uppdateringar slutar fungera för alla användare.

### Exportera nyckeln (en gång, från maskinen som har filen)

```bash
cat ~/.tauri/fr-app.key
```

Utskriften ser ut ungefär så här (två rader):
```
untrusted comment: minisign secret key: 2A38078FD49B8BB5
RWRTbxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Kopiera **hela utskriften** och spara den i din lösenordshanterare (t.ex. Bitwarden,
1Password) under namnet `FR_SIGNING_KEY`.

### Använda på en ny maskin (ingen fil behövs)

Hämta värdet ur lösenordshanteraren och exportera det i terminalen:

```bash
export FR_SIGNING_KEY="untrusted comment: minisign secret key: 2A38078FD49B8BB5
RWRTbxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
```

Sedan körs `fr-release` precis som vanligt — den ser miljövariabeln och behöver
ingen fil på disk:

```bash
FR_UPLOAD_KEY=<nyckel> fr-release --notes "Buggfixar"
```

### Prioritetsordning

`fr-release` väljer signeringsnyckel i denna ordning:
1. `FR_SIGNING_KEY` (miljövariabel med nyckelinnehållet)
2. `--key-path` / `~/.tauri/fr-app.key` (fil på disk)

---

## Bygga för release

Bygg görs normalt via `fr-release` (se nedan), men kan också köras manuellt:

```bash
cd fr-app
TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/fr-app.key bunx tauri build
```

Artefakter hamnar i `fr-app/src-tauri/target/release/bundle/appimage/`:
- `fr-app_<version>_amd64.AppImage`          — körbar AppImage
- `fr-app_<version>_amd64.AppImage.tar.gz`   — arkiv för uppdateraren
- `fr-app_<version>_amd64.AppImage.tar.gz.sig` — signatur (krävs av uppdateraren)

### Server-URL vid bygge

| Miljö | Kommando |
|-------|---------|
| Produktion (default) | `bunx tauri build` |
| Staging | `VITE_SERVER_URL=https://stage.foretagsdatabasen.se bunx tauri build` |
| Lokal | `VITE_SERVER_URL=http://localhost:8081 bunx tauri build` |

---

## Release-flöde (steg för steg)

### 1. Bumpa versionen

Redigera **båda** dessa filer med samma versionsnummer:

- `fr-app/src-tauri/tauri.conf.json` → `"version": "X.Y.Z"`
- `fr-app/src-tauri/Cargo.toml` → `version = "X.Y.Z"`

### 2. Bygg och publicera med fr-release

```bash
cd fr-release

# Sätt upload-nyckeln (finns i config.yaml på servern under upload_key)
export FR_UPLOAD_KEY=<upload-nyckel>

# Release mot produktion
cargo run -- --notes "Beskrivning av vad som är nytt"

# Release mot lokal server (för test)
cargo run -- --server http://localhost:8080 --notes "Test"
```

`fr-release` gör automatiskt:
1. Läser version ur `tauri.conf.json`
2. Kör `bunx tauri build` med signeringsnyckeln
3. Hittar `.AppImage.tar.gz` och `.sig` i bundle-katalogen
4. Laddar upp till servern
5. Servern uppdaterar `/desktop/latest.json`

Inloggade användare ser en uppdateringsbanner i appen vid nästa start.

### Alla flaggor

```
fr-release --help
```

---

## Server-endpoints (fr-web)

Starta fr-web med desktop-katalogen aktiverad (sker automatiskt med default-flaggor):

```bash
./fr-web --desktop desktop
```

| Endpoint | Beskrivning |
|----------|------------|
| `GET /desktop/latest.json` | Tauri updater läser denna vid varje appstart |
| `GET /desktop/download/<fil>` | Serverar AppImage-arkivet vid uppdatering |
| `POST /api/desktop/upload` | Skyddad, används av fr-release |

Upload-nyckeln är densamma som för databas-upload (`upload_key` i `config.yaml`).

---

## GitHub Actions — multi-platform byggen

Cross-kompilering Linux → Windows/Mac stöds inte av Tauri. Istället kör GitHub
Actions byggen parallellt på riktiga Windows- och Mac-maskiner i molnet.

Workflowen finns i `.github/workflows/build.yml` och triggas manuellt.

### Förutsättningar (görs en gång)

Lägg till signeringsnyckeln som GitHub Secret i repot:
`Settings → Secrets and variables → Actions → New repository secret`

| Secret | Värde |
|--------|-------|
| `TAURI_SIGNING_PRIVATE_KEY` | Hela innehållet i `~/.tauri/fr-app.key` (två rader) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Lämna tomt om inget lösenord sattes |

### Trigga ett bygge

Gå till `https://github.com/tobbegd/fr-desktop/actions/workflows/build.yml`
och klicka **Run workflow**.

Tre jobb körs parallellt (~30-40 min totalt):

| Plattform | Artefakt |
|-----------|----------|
| Linux (`ubuntu-22.04`) | `.AppImage` |
| Windows (`windows-latest`) | `.msi` |
| Mac (`macos-latest`) | `.dmg` (universal — Intel + M1) |

### Hämta artefakter

När bygget är klart finns artefakterna under **Artifacts** på körningens sida.
Ladda ner, extrahera och ladda upp till fr-web admin med `fr-release`.

### Mac-notering

Utan Apple Developer-konto ($99/år) visas ett Gatekeeper-varning för Mac-användare.
De kan kringgå det genom att högerklicka → Öppna. Lös med kodsignering när första
betalande Mac-kund dyker upp.

### Windows-notering

Utan kodsignering visas SmartScreen-varning vid installation — klicka "Mer info"
→ "Kör ändå". Acceptabelt i tidigt skede.

---

## Miljövariabler

| Variabel | Beskrivning |
|----------|------------|
| `FR_UPLOAD_KEY` | Upload-nyckel, alternativ till `--upload-key` |
| `VITE_SERVER_URL` | Override av server-URL vid bygge av fr-app |
| `TAURI_SIGNING_PRIVATE_KEY_PATH` | Sätts automatiskt av fr-release |
