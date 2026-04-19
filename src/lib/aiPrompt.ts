const COLUMN_DESCRIPTIONS_FALLBACK: Record<string, string> = {
  orgnamn:        "företagets namn — använd ulow(orgnamn) LIKE för sökning",
  orgnr:          "organisationsnummer",
  orgform:        "bolagsform t.ex. AB, HB, EF, KB — INTE bransch eller verksamhet",
  aktiv:          "1 om aktivt bolag, 0 om inaktivt",
  postort:        "ort/stad — sök med ulow(postort) LIKE '%stockholm%'",
  postnummer:     "postnummer",
  gatuadress:     "gatuadress",
  sateskommun:    "kommunnamn",
  "sateslän":     "lännamn",
  sni_1_namn:     "primär branschbeskrivning på svenska — sök med ulow(sni_1_namn) LIKE",
  sni_2_namn:     "sekundär branschbeskrivning — sök med ulow(sni_2_namn) LIKE",
  sni_3_namn:     "tertiär branschbeskrivning",
  sni_1:          "primär SNI-branschkod (numerisk) — använd INTE för textsökning, använd sni_1_namn",
  sni_2:          "sekundär SNI-branschkod (numerisk) — använd INTE för textsökning, använd sni_2_namn",
  verksamhet:     "fritext om vad företaget gör — använd LIKE för sökning",
  telefon:        "telefonnummer",
  email:          "e-postadress",
  webbadress:     "webbplatsadress",
  nettoomsattning:        "senaste kända nettoomsättning i kr",
  medelantal_anstallda:   "medelantal anställda",
  arets_resultat:         "årets resultat i kr",
  eget_kapital:           "eget kapital i kr",
  registreringsdatum:     "datum då bolaget registrerades",
  storleksklass_anst:     "storleksklass baserad på antal anställda",
  storleksklass_oms:      "storleksklass baserad på omsättning",
};

const SEARCH_RULES = `
Regler:
- Använd ulow(kolumn) LIKE '%term%' för ALL textsökning — ulow() hanterar svenska tecken (å,ä,ö). Använd ALDRIG LOWER(), använd ALLTID ulow()
- Regioner är inte postorter — använd sateskommun (numerisk kommunkod) med BETWEEN: Norrland = sateskommun BETWEEN 2200 AND 2599, Skåne = sateskommun BETWEEN 1200 AND 1299, Västra Götaland = sateskommun BETWEEN 1400 AND 1499, Värmland = sateskommun BETWEEN 1700 AND 1799, Dalarna = sateskommun BETWEEN 2000 AND 2099, Gävleborg = sateskommun BETWEEN 2100 AND 2199
- sni_1/sni_2/sni_3 är NUMERISKA koder — sök aldrig bransch med dessa; använd sni_1_namn, sni_2_namn med ulow() LIKE
- För verksamhetstyp: sök alltid i ALLA fyra kolumner med OR: sni_1_namn, sni_2_namn, verksamhet, orgnamn — många bolag har branschen bara i orgnamn eller verksamhet
- orgform är bolagsform (AB, HB...) — använd aldrig orgform för att söka bransch
- Använd alltid LIMIT — om antal efterfrågas, använd det antalet, annars LIMIT 200
- bolag har redan kolumnen webbadress — JOIN:a INTE webbplatser-tabellen för webbadress
- bolag har redan kolumnerna nettoomsattning, arets_resultat, eget_kapital — JOIN:a INTE arsredovisningar för dessa, använd direkt från bolag
- JOIN med arsredovisningar ENDAST om användaren specifikt ber om historik eller flera år — då: JOIN (SELECT orgnr, arets_resultat, rakenskapsar_slut FROM arsredovisningar GROUP BY orgnr HAVING rakenskapsar_slut = MAX(rakenskapsar_slut)) AS ar ON ar.orgnr = b.orgnr
`;

// aiExpl: table -> column -> description (loaded from ai_expl table in DB)
export type AiExpl = Record<string, Record<string, string>>;

export function buildColumnGuide(schema: Record<string, string[]>, aiExpl: AiExpl = {}): string {
  const lines: string[] = [];
  for (const [table, cols] of Object.entries(schema)) {
    const tableDescs = aiExpl[table] ?? {};
    const described = cols.filter(c => tableDescs[c] ?? COLUMN_DESCRIPTIONS_FALLBACK[c]);
    if (described.length === 0) continue;
    lines.push(`Kolumner i ${table}:`);
    for (const col of described) {
      const desc = tableDescs[col] ?? COLUMN_DESCRIPTIONS_FALLBACK[col];
      lines.push(`  ${col.padEnd(28)} = ${desc}`);
    }
  }
  return lines.join("\n");
}

export function buildPrompt(
  schema: Record<string, string[]>,
  question: string,
  aiExpl: AiExpl = {}
): string {
  const columnGuide = buildColumnGuide(schema, aiExpl);
  const schemaText = Object.entries(schema)
    .map(([t, cols]) => `${t} (${cols.join(", ")})`)
    .join("\n");

  return `Du är en SQL-expert för en svensk företagsdatabas i SQLite. Generera ENDAST en giltig SQL-fråga — inga förklaringar, ingen markdown, inget kodblock.
${SEARCH_RULES}
Använd ENDAST kolumnnamn som finns i schemat nedan.
${columnGuide}

Tabeller:
${schemaText}

Exempel:
Fråga: byggföretag från norrland
SQL: SELECT * FROM bolag WHERE sateskommun BETWEEN 2200 AND 2599 AND (ulow(sni_1_namn) LIKE '%bygg%' OR ulow(sni_2_namn) LIKE '%bygg%' OR ulow(verksamhet) LIKE '%bygg%') LIMIT 200;

Fråga: taxi i stockholm
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (ulow(sni_1_namn) LIKE '%taxi%' OR ulow(sni_2_namn) LIKE '%taxi%' OR ulow(orgnamn) LIKE '%taxi%' OR ulow(verksamhet) LIKE '%taxi%') LIMIT 200;

Fråga: ge mig 5 taxibolag från stockholm
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (ulow(sni_1_namn) LIKE '%taxi%' OR ulow(sni_2_namn) LIKE '%taxi%' OR ulow(orgnamn) LIKE '%taxi%' OR ulow(verksamhet) LIKE '%taxi%') LIMIT 5;

Fråga: visa 10 bolag från göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' LIMIT 10;

Fråga: bolag från göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' LIMIT 200;

Fråga: sushi restaurang göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' AND (ulow(sni_1_namn) LIKE '%restaurang%' OR ulow(sni_2_namn) LIKE '%restaurang%' OR ulow(orgnamn) LIKE '%sushi%' OR ulow(verksamhet) LIKE '%sushi%') LIMIT 200;

Fråga: bilverkstad i karlstad
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%karlstad%' AND (ulow(sni_1_namn) LIKE '%bilverkstad%' OR ulow(sni_2_namn) LIKE '%bilverkstad%' OR ulow(verksamhet) LIKE '%bilverkstad%' OR ulow(sni_1_namn) LIKE '%fordonsreparation%') LIMIT 200;

Fråga: webbyråer i karlstad med organisationsnummer, namn, webb och årsresultat
SQL: SELECT orgnr, orgnamn, webbadress, arets_resultat FROM bolag WHERE ulow(postort) LIKE '%karlstad%' AND (ulow(sni_1_namn) LIKE '%webbyrå%' OR ulow(sni_2_namn) LIKE '%webbyrå%' OR ulow(verksamhet) LIKE '%webbyrå%' OR ulow(orgnamn) LIKE '%webbyrå%') LIMIT 200;

Fråga: it-konsulter i stockholm med historisk omsättning per år
SQL: SELECT b.orgnr, b.orgnamn, ar.rakenskapsar_slut, ar.nettoomsattning FROM bolag b JOIN arsredovisningar ar ON ar.orgnr = b.orgnr WHERE ulow(b.postort) LIKE '%stockholm%' AND (ulow(b.sni_1_namn) LIKE '%it-konsult%' OR ulow(b.sni_2_namn) LIKE '%it-konsult%' OR ulow(b.verksamhet) LIKE '%it-konsult%') ORDER BY b.orgnr, ar.rakenskapsar_slut DESC LIMIT 200;

Fråga: ${question}

SQL:`;
}
