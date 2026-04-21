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
  ar_year:                "senaste bokslutår som finns — IS NOT NULL betyder att nyckeltal finns för bolaget",
  nettoomsattning:        "senaste kända nettoomsättning i kr",
  medelantal_anstallda:   "medelantal anställda",
  arets_resultat:         "årets resultat i kr",
  eget_kapital:           "eget kapital i kr",
  lat:                    "exakt latitud för adressen — finns inte alltid",
  lon:                    "exakt longitud för adressen — finns inte alltid",
  postort_lat:            "latitud för postorten — finns nästan alltid",
  postort_lon:            "longitud för postorten — finns nästan alltid",
  registreringsdatum:     "datum då bolaget registrerades",
  storleksklass_anst:     "storleksklass baserad på antal anställda",
  storleksklass_oms:      "storleksklass baserad på omsättning",
};

const SEARCH_RULES = `
Regler:
- Använd ulow(kolumn) LIKE '%term%' för ALL textsökning — ulow() hanterar svenska tecken (å,ä,ö). Använd ALDRIG LOWER(), använd ALLTID ulow()
- Regioner är inte postorter — använd sateskommun (numerisk kommunkod) med BETWEEN: Norrland = sateskommun BETWEEN 2200 AND 2599, Skåne = sateskommun BETWEEN 1200 AND 1299, Västra Götaland = sateskommun BETWEEN 1400 AND 1499, Värmland = sateskommun BETWEEN 1700 AND 1799, Dalarna = sateskommun BETWEEN 2000 AND 2099, Gävleborg = sateskommun BETWEEN 2100 AND 2199
- sni_1/sni_2/sni_3 är NUMERISKA koder — sök aldrig bransch med dessa; använd sni_1_namn, sni_2_namn med ulow() LIKE
- För verksamhetstyp: sök ALLTID i ALLA fem kolumner med OR: sni_1_namn, sni_2_namn, sni_3_namn, verksamhet, orgnamn — detta är OBLIGATORISKT, hoppa aldrig över verksamhet eller orgnamn även om sni-termerna verkar täcka frågan. Många bolag har branschen BARA i orgnamn eller verksamhet och saknar rätt SNI-kod
- orgform är bolagsform (AB, HB...) — använd aldrig orgform för att söka bransch
- Använd alltid LIMIT — om antal efterfrågas, använd det antalet, annars LIMIT 200
- bolag har redan kolumnen webbadress — JOIN:a INTE webbplatser-tabellen för webbadress
- bolag har redan kolumnerna nettoomsattning, arets_resultat, eget_kapital — JOIN:a INTE arsredovisningar för dessa, använd direkt från bolag
- JOIN med arsredovisningar ENDAST om användaren specifikt ber om historik eller flera år — då: JOIN (SELECT orgnr, arets_resultat, rakenskapsar_slut FROM arsredovisningar GROUP BY orgnr HAVING rakenskapsar_slut = MAX(rakenskapsar_slut)) AS ar ON ar.orgnr = b.orgnr
- Om användaren frågar efter bolag med webbadress, e-post eller telefon — filtrera alltid bort NULL och tomma strängar: t.ex. webbadress IS NOT NULL AND webbadress <> ''
- Vid filtrering på numeriska fält (medelantal_anstallda, nettoomsattning, arets_resultat, eget_kapital) — lägg alltid till IS NOT NULL: t.ex. medelantal_anstallda IS NOT NULL AND medelantal_anstallda > 10
- "Webbyrå" är inte ett SNI-begrepp — sök alltid med BÅDE SNI-termer (dataprogrammering, it-konsult, webbdesign) OCH verksamhet/orgnamn med webb, digital
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

export function buildChatPrompt(
  schema: Record<string, string[]>,
  question: string,
  aiExpl: AiExpl = {},
  currentSql = ""
): string {
  const columnGuide = buildColumnGuide(schema, aiExpl);
  const schemaText = Object.entries(schema)
    .map(([t, cols]) => `${t} (${cols.join(", ")})`)
    .join("\n");

  const sqlContext = currentSql.trim()
    ? `\nAktiv SQL-fråga i editorn:\n${currentSql.trim()}\n`
    : "";

  return `Du är en assistent för en svensk företagsdatabas i SQLite. Svara på svenska med klartext — ingen SQL om det inte specifikt efterfrågas.

Tabeller:
${schemaText}

Kolumnbeskrivningar:
${columnGuide}
${sqlContext}
Fråga: ${question}`;
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
SQL: SELECT * FROM bolag WHERE sateskommun BETWEEN 2200 AND 2599 AND (ulow(sni_1_namn) LIKE '%bygg%' OR ulow(sni_2_namn) LIKE '%bygg%' OR ulow(sni_3_namn) LIKE '%bygg%' OR ulow(verksamhet) LIKE '%bygg%' OR ulow(orgnamn) LIKE '%bygg%') LIMIT 200;

Fråga: taxi i stockholm
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (ulow(sni_1_namn) LIKE '%taxi%' OR ulow(sni_2_namn) LIKE '%taxi%' OR ulow(sni_3_namn) LIKE '%taxi%' OR ulow(verksamhet) LIKE '%taxi%' OR ulow(orgnamn) LIKE '%taxi%') LIMIT 200;

Fråga: ge mig 5 taxibolag från stockholm
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (ulow(sni_1_namn) LIKE '%taxi%' OR ulow(sni_2_namn) LIKE '%taxi%' OR ulow(sni_3_namn) LIKE '%taxi%' OR ulow(verksamhet) LIKE '%taxi%' OR ulow(orgnamn) LIKE '%taxi%') LIMIT 5;

Fråga: visa 10 bolag från göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' LIMIT 10;

Fråga: bolag från göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' LIMIT 200;

Fråga: sushi restaurang göteborg
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' AND (ulow(sni_1_namn) LIKE '%restaurang%' OR ulow(sni_2_namn) LIKE '%restaurang%' OR ulow(sni_3_namn) LIKE '%restaurang%' OR ulow(verksamhet) LIKE '%sushi%' OR ulow(orgnamn) LIKE '%sushi%') LIMIT 200;

Fråga: bilverkstad i karlstad
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%karlstad%' AND (ulow(sni_1_namn) LIKE '%bilverkstad%' OR ulow(sni_1_namn) LIKE '%fordonsreparation%' OR ulow(sni_2_namn) LIKE '%bilverkstad%' OR ulow(sni_3_namn) LIKE '%bilverkstad%' OR ulow(verksamhet) LIKE '%bilverkstad%' OR ulow(orgnamn) LIKE '%bil%') LIMIT 200;

Fråga: webbyråer i karlstad med organisationsnummer, namn, webb och årsresultat
SQL: SELECT orgnr, orgnamn, webbadress, arets_resultat FROM bolag WHERE ulow(postort) LIKE '%karlstad%' AND (ulow(sni_1_namn) LIKE '%webbyrå%' OR ulow(sni_1_namn) LIKE '%dataprogrammering%' OR ulow(sni_1_namn) LIKE '%it-konsult%' OR ulow(sni_2_namn) LIKE '%webbyrå%' OR ulow(sni_2_namn) LIKE '%dataprogrammering%' OR ulow(verksamhet) LIKE '%webbyrå%' OR ulow(verksamhet) LIKE '%webb%' OR ulow(orgnamn) LIKE '%webbyrå%' OR ulow(orgnamn) LIKE '%webb%') LIMIT 200;

Fråga: 10 webbyråer från stockholm med fler än 10 anställda
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (ulow(sni_1_namn) LIKE '%webbyrå%' OR ulow(sni_1_namn) LIKE '%dataprogrammering%' OR ulow(sni_1_namn) LIKE '%it-konsult%' OR ulow(sni_2_namn) LIKE '%webbyrå%' OR ulow(sni_2_namn) LIKE '%dataprogrammering%' OR ulow(verksamhet) LIKE '%webbyrå%' OR ulow(verksamhet) LIKE '%webb%' OR ulow(orgnamn) LIKE '%webbyrå%' OR ulow(orgnamn) LIKE '%webb%') AND medelantal_anstallda IS NOT NULL AND medelantal_anstallda > 10 LIMIT 10;

Fråga: bolag i örebro med nyckeltal
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%örebro%' AND ar_year IS NOT NULL LIMIT 200;

Fråga: visa bolag i örebro på karta
SQL: SELECT orgnr, orgnamn, postort, gatuadress, lat, lon, postort_lat, postort_lon FROM bolag WHERE ulow(postort) LIKE '%örebro%' LIMIT 200;

Fråga: restauranger i stockholm på karta
SQL: SELECT orgnr, orgnamn, postort, gatuadress, lat, lon, postort_lat, postort_lon FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (ulow(sni_1_namn) LIKE '%restaurang%' OR ulow(sni_2_namn) LIKE '%restaurang%' OR ulow(verksamhet) LIKE '%restaurang%') LIMIT 200;

Fråga: 50 bolag från stockholm med webbadress
SQL: SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND webbadress IS NOT NULL AND webbadress <> '' LIMIT 50;

Fråga: restauranger i göteborg med telefon och webbadress
SQL: SELECT orgnamn, telefon, webbadress FROM bolag WHERE ulow(postort) LIKE '%göteborg%' AND telefon IS NOT NULL AND telefon <> '' AND webbadress IS NOT NULL AND webbadress <> '' AND (ulow(sni_1_namn) LIKE '%restaurang%' OR ulow(sni_2_namn) LIKE '%restaurang%') LIMIT 200;

Fråga: it-konsulter i stockholm med historisk omsättning per år
SQL: SELECT b.orgnr, b.orgnamn, ar.rakenskapsar_slut, ar.nettoomsattning FROM bolag b JOIN arsredovisningar ar ON ar.orgnr = b.orgnr WHERE ulow(b.postort) LIKE '%stockholm%' AND (ulow(b.sni_1_namn) LIKE '%it-konsult%' OR ulow(b.sni_2_namn) LIKE '%it-konsult%' OR ulow(b.verksamhet) LIKE '%it-konsult%') ORDER BY b.orgnr, ar.rakenskapsar_slut DESC LIMIT 200;

Fråga: vilka kolumner har bolag
SQL: SELECT name, type FROM pragma_table_info('bolag');

Fråga: vilka kolumner finns i arsredovisningar
SQL: SELECT name, type FROM pragma_table_info('arsredovisningar');

Fråga: vilka tabeller finns
SQL: SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;

Fråga: ${question}

SQL:`;
}
