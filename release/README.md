# fr-release

Verktyg för att bygga och publicera fr-app till kunder.

---

## Snabbstart — vanlig release (GitHub Actions)

```bash
# 1. Bumpa patch-version (0.1.6 → 0.1.7)
./bump-version.sh

# 2. Committa och pusha
cd ../fr-app && git add -A && git commit -m "Bumpa till $(grep version src-tauri/tauri.conf.json | head -1 | grep -o '[0-9.]*')" && git push && cd ../fr-release

# 3. Bygg och ladda upp alla plattformar
./build.sh --github --notes "Beskriv vad som är nytt"

# 4. Aktivera i fr-web admin
# → https://foretagsdatabasen.se/admin/desktop
#    Klicka "Aktivera" på det nya bygget
```

Det tar 15–30 minuter medan GitHub Actions bygger Linux, Windows och Mac parallellt.
Alla tre plattformar hamnar automatiskt som staging-byggen och väntar på aktivering.

---

## Engångsinställning (första gången på en ny maskin)

### 1. Lägg in nycklar i build.sh

Kopiera `build.sh.example` till `build.sh` (gitignored) och fyll i nycklarna:

```bash
cp build.sh.example build.sh
```

Fyll i `FR_GITHUB_TOKEN` och `FR_UPLOAD_KEY` (se nedan).

### 2. GitHub PAT (FR_GITHUB_TOKEN)

Skapa ett Personal Access Token på:
→ https://github.com/settings/tokens → **Generate new token (classic)**

Scopes som krävs: `repo` + `workflow`

### 3. Upload-nyckel (FR_UPLOAD_KEY)

Finns i lösenordshanteraren som **FR_UPLOAD_KEY / fr-release**.

### 4. Signeringsnyckel (automatisk via GitHub Secrets)

I GitHub Actions-läget signerar GitHub Actions-workern med nyckeln som är lagrad i
repo-secretet `TAURI_SIGNING_PRIVATE_KEY`. Du behöver inte ha nyckeln lokalt.

Om du vill köra lokalt Linux-bygge behöver du nyckeln — se avsnittet nedan.

---

## Versionsbumpning

```bash
./bump-version.sh           # patch: 0.1.6 → 0.1.7
./bump-version.sh --manual 0.2.0   # valfri version
```

Uppdaterar `fr-app/src-tauri/tauri.conf.json` och `fr-app/src-tauri/Cargo.toml`.

---

## Lokalt Linux-bygge (utan GitHub Actions)

Används för snabb test mot lokal server, eller om GitHub Actions inte är tillgängligt.

Kräver signeringsnyckeln lokalt:
```bash
# Hämta ur lösenordshanteraren och exportera
export FR_SIGNING_KEY="$(cat ~/.tauri/fr-app.key)"   # om nyckeln finns på disk
# eller
export FR_SIGNING_KEY="<klistra in från lösenordshanteraren>"
```

Bygg och ladda upp:
```bash
FR_UPLOAD_KEY=abc123 cargo run -- --upload --notes "Buggfix"
```

Mot lokal server (test utan --upload):
```bash
./build.sh --notes "Test"   # kör mot http://localhost:8081, ingen uppladdning
./build.sh --upload --notes "Test"   # laddar upp till localhost
```

---

## Alla flaggor

| Flagga | Env-var | Beskrivning |
|--------|---------|-------------|
| `--github` | — | GitHub Actions-läge: bygg alla plattformar och ladda upp |
| `--github-token` | `FR_GITHUB_TOKEN` | GitHub PAT med repo+workflow-scope |
| `--upload` | — | Ladda upp efter lokalt bygge (Linux) |
| `--upload-key` | `FR_UPLOAD_KEY` | Bearer-token till `/api/output/desktop/upload` |
| `--notes` | — | Release-text som visas i admin |
| `--server` | — | Server-URL (default: https://foretagsdatabasen.se) |
| `--no-build` | — | Hoppa över bygget, använd senaste artifact |
| `--key-path` | — | Sökväg till signeringsnyckel (lokalt läge) |

---

## Vad händer i GitHub Actions-läget

1. `build.yml` triggas via GitHub API
2. Väntar tills nytt run dyker upp
3. Pollar var 30 s tills alla tre plattformar är klara
4. Laddar ner artefakt-zippar (linux / windows / mac)
5. Extraherar `.AppImage.tar.gz`, `.msi.zip`, `.app.tar.gz` + tillhörande `.sig`
6. Laddar upp varje plattform till fr-web som **staging**
7. Skriver ut länk till admin-sidan

Om en plattform misslyckas fortsätter de övriga och du får en varning.

---

## Filer

| Fil | Beskrivning |
|-----|-------------|
| `build.sh` | Lokalt testskript (gitignored, innehåller nycklar) |
| `build.sh.example` | Mall utan nycklar |
| `bump-version.sh` | Bumpar patch-version i tauri.conf.json + Cargo.toml |
| `src/main.rs` | Hela verktyget |
