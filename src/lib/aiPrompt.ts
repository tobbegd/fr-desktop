const COLUMN_DESCRIPTIONS_FALLBACK: Record<string, string> = {
  orgnamn:        "företagets namn — använd LOWER(orgnamn) LIKE för sökning",
  orgnr:          "organisationsnummer",
  orgform:        "bolagsform t.ex. AB, HB, EF, KB — INTE bransch eller verksamhet",
  aktiv:          "1 om aktivt bolag, 0 om inaktivt",
  postort:        "ort/stad — sök med ulow(postort) LIKE '%stockholm%'",
  postnummer:     "postnummer",
  gatuadress:     "gatuadress",
  sateskommun:    "kommunnamn",
  "sateslän":     "lännamn",
  sni_1_namn:     "primär branschbeskrivning på svenska — sök här för verksamhetstyp",
  sni_2_namn:     "sekundär branschbeskrivning — sök här om sni_1_namn inte matchar",
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
- Använd LOWER(kolumn) LIKE LOWER('%term%') för textsökning på namn/branscher — aldrig =
- postort: använd ulow(postort) LIKE '%stockholm%' — ulow() är en inbyggd funktion som hanterar svenska tecken
- Regioner är inte postorter — använd sateskommun (numerisk kommunkod) med BETWEEN: Norrland = sateskommun BETWEEN 2200 AND 2599, Skåne = sateskommun BETWEEN 1200 AND 1299, Västra Götaland = sateskommun BETWEEN 1400 AND 1499, Värmland = sateskommun BETWEEN 1700 AND 1799, Dalarna = sateskommun BETWEEN 2000 AND 2099, Gävleborg = sateskommun BETWEEN 2100 AND 2199
- sni_1/sni_2/sni_3 är NUMERISKA koder — sök aldrig bransch med dessa; använd sni_1_namn, sni_2_namn med LIKE
- För verksamhetstyp: sök alltid i ALLA fyra kolumner med OR: sni_1_namn, sni_2_namn, verksamhet, orgnamn — många bolag har branschen bara i orgnamn eller verksamhet
- orgform är bolagsform (AB, HB...) — använd aldrig orgform för att söka bransch
- Använd alltid LIMIT — om antal efterfrågas, använd det antalet, annars LIMIT 200
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
SQL: SELECT * FROM bolag WHERE sateskommun BETWEEN 2200 AND 2599 AND (LOWER(sni_1_namn) LIKE '%bygg%' OR LOWER(sni_2_namn) LIKE '%bygg%' OR LOWER(verksamhet) LIKE '%bygg%');

Fråga: taxi i stockholm
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (LOWER(sni_1_namn) LIKE '%taxi%' OR LOWER(sni_2_namn) LIKE '%taxi%' OR LOWER(orgnamn) LIKE '%taxi%' OR LOWER(verksamhet) LIKE '%taxi%') LIMIT 200;

Fråga: ge mig 5 taxibolag från stockholm
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (LOWER(sni_1_namn) LIKE '%taxi%' OR LOWER(sni_2_namn) LIKE '%taxi%' OR LOWER(orgnamn) LIKE '%taxi%' OR LOWER(verksamhet) LIKE '%taxi%') LIMIT 5;

Fråga: visa 10 bolag från göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' LIMIT 10;

Fråga: bolag från göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' LIMIT 200;

Fråga: sushi restaurang göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' AND (LOWER(sni_1_namn) LIKE '%restaurang%' OR LOWER(sni_2_namn) LIKE '%restaurang%' OR LOWER(orgnamn) LIKE '%sushi%' OR LOWER(verksamhet) LIKE '%sushi%');

Fråga: bilverkstad i karlstad
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%karlstad%' AND (LOWER(sni_1_namn) LIKE '%bilverkstad%' OR LOWER(sni_2_namn) LIKE '%bilverkstad%' OR LOWER(verksamhet) LIKE '%bilverkstad%' OR LOWER(sni_1_namn) LIKE '%fordonsreparation%');

Fråga: ${question}

SQL:`;
}
