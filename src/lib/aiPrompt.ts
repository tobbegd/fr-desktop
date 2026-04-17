const COLUMN_DESCRIPTIONS: Record<string, string> = {
  orgnamn:        "företagets namn — använd LOWER(orgnamn) LIKE för sökning",
  orgnr:          "organisationsnummer",
  orgform:        "bolagsform t.ex. AB, HB, EF, KB — INTE bransch eller verksamhet",
  aktiv:          "1 om aktivt bolag, 0 om inaktivt",
  postort:        "ort/stad — använd LOWER(postort) LIKE för sökning",
  postnummer:     "postnummer",
  gatuadress:     "gatuadress",
  sateskommun:    "kommunnamn",
  "sateslän":     "lännamn",
  sni_1_namn:     "primär branschbeskrivning på svenska — sök här för verksamhetstyp",
  sni_2_namn:     "sekundär branschbeskrivning — sök här om sni_1_namn inte matchar",
  sni_3_namn:     "tertiär branschbeskrivning",
  sni_1:          "primär SNI-branschkod (numerisk)",
  sni_2:          "sekundär SNI-branschkod",
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
- Använd ALLTID LOWER(kolumn) LIKE LOWER('%term%') för textsökning — aldrig = för namn/orter/branscher
- För bransch/verksamhetstyp: sök i sni_1_namn, sni_2_namn och/eller verksamhet med LIKE
- orgform är bolagsform (AB, HB...) — använd ALDRIG orgform för att söka bransch
- Returnera rimlig mängd resultat, använd LIMIT om inget annat anges
`;

export function buildColumnGuide(schema: Record<string, string[]>): string {
  const lines: string[] = [];
  for (const [table, cols] of Object.entries(schema)) {
    const described = cols.filter(c => COLUMN_DESCRIPTIONS[c]);
    if (described.length === 0) continue;
    lines.push(`Kolumner i ${table}:`);
    for (const col of described) {
      lines.push(`  ${col.padEnd(28)} = ${COLUMN_DESCRIPTIONS[col]}`);
    }
  }
  return lines.join("\n");
}

export function buildPrompt(
  schema: Record<string, string[]>,
  question: string,
  model: string
): string {
  const isNsql = model.toLowerCase().includes("nsql");
  const columnGuide = buildColumnGuide(schema);

  const createStatements = Object.entries(schema)
    .map(([t, cols]) => `CREATE TABLE ${t} (${cols.join(", ")});`)
    .join("\n");
  const schemaText = Object.entries(schema)
    .map(([t, cols]) => `${t} (${cols.join(", ")})`)
    .join("\n");

  if (isNsql) {
    return `### Instruction:
Your task is to generate valid sqlite SQL to answer the following question.
Always use LOWER(column) LIKE LOWER('%term%') for text searches, never exact = for names or locations.
${columnGuide}

### Input:
Here is the database schema that the SQL query will run on:
${createStatements}

### Question:
${question}

### Response (use sqlite syntax):
`;
  }

  return `Du är en SQL-expert för en svensk företagsdatabas i SQLite. Generera ENDAST en giltig SQL-fråga — inga förklaringar, ingen markdown, inget kodblock.
${SEARCH_RULES}
${columnGuide}

Tabeller:
${schemaText}

Fråga: ${question}

SQL:`;
}
