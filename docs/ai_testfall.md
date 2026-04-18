# AI-prompttest — exempelfrågor och förväntad SQL

Används för att utvärdera prompten. `ulow()` är en inbyggd SQLite-funktion som hanterar svenska tecken (å/ä/ö).

---

## Ort/geografi

```
visa 20 bolag från malmö
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%malmö%' LIMIT 20;
```

```
bolag i Uppsala eller Västerås
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%uppsala%' OR ulow(postort) LIKE '%västerås%';
```

```
jag vill ha 10 bolag från karlstad
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%karlstad%' LIMIT 10;
```

---

## Bransch

```
ge mig 5 taxibolag från stockholm
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (LOWER(sni_1_namn) LIKE '%taxi%' OR LOWER(sni_2_namn) LIKE '%taxi%') LIMIT 5;
```

```
sushi restaurang göteborg
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' AND (LOWER(sni_1_namn) LIKE '%restaurang%' OR LOWER(sni_2_namn) LIKE '%restaurang%' OR LOWER(verksamhet) LIKE '%sushi%' OR LOWER(sni_1_namn) LIKE '%sushi%');
```

```
alla restauranger i göteborg
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%göteborg%' AND (LOWER(sni_1_namn) LIKE '%restaurang%' OR LOWER(sni_2_namn) LIKE '%restaurang%' OR LOWER(verksamhet) LIKE '%restaurang%');
```

```
IT-konsulter i stockholm
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND (LOWER(sni_1_namn) LIKE '%it%' OR LOWER(sni_1_namn) LIKE '%konsult%' OR LOWER(sni_2_namn) LIKE '%it%' OR LOWER(verksamhet) LIKE '%it-konsult%');
```

```
byggföretag i norrland
→ SELECT * FROM bolag WHERE (ulow(postort) LIKE '%sundsvall%' OR ulow(postort) LIKE '%umeå%' OR ulow(postort) LIKE '%östersund%' OR ulow(postort) LIKE '%luleå%') AND (LOWER(sni_1_namn) LIKE '%bygg%' OR LOWER(sni_2_namn) LIKE '%bygg%');
```

```
pizzerior i sverige sorterade på omsättning
→ SELECT * FROM bolag WHERE LOWER(sni_1_namn) LIKE '%pizz%' OR LOWER(sni_2_namn) LIKE '%pizz%' OR LOWER(verksamhet) LIKE '%pizza%' ORDER BY nettoomsattning DESC;
```

---

## Storlek/ekonomi

```
bolag med mer än 50 anställda i stockholm
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND medelantal_anstallda > 50;
```

```
företag med omsättning över 10 miljoner
→ SELECT * FROM bolag WHERE nettoomsattning > 10000000;
```

```
de 10 största företagen i sverige mätt på omsättning
→ SELECT * FROM bolag ORDER BY nettoomsattning DESC LIMIT 10;
```

---

## Bolagsform

```
alla enskilda firmor i lund
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%lund%' AND orgform = 'EF';
```

```
aktiebolag inom vård i stockholm
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%stockholm%' AND orgform = 'AB' AND (LOWER(sni_1_namn) LIKE '%vård%' OR LOWER(sni_2_namn) LIKE '%vård%');
```

---

## Kombinationer

```
aktiva redovisningsbyråer i göteborg med fler än 5 anställda
→ SELECT * FROM bolag WHERE aktiv = 1 AND ulow(postort) LIKE '%göteborg%' AND (LOWER(sni_1_namn) LIKE '%redovisning%' OR LOWER(sni_2_namn) LIKE '%redovisning%') AND medelantal_anstallda > 5;
```

```
bilverkstad i karlstad
→ SELECT * FROM bolag WHERE ulow(postort) LIKE '%karlstad%' AND (LOWER(sni_1_namn) LIKE '%bilverkstad%' OR LOWER(sni_2_namn) LIKE '%bilverkstad%' OR LOWER(verksamhet) LIKE '%bilverkstad%' OR LOWER(sni_1_namn) LIKE '%fordonsreparation%');
```

---

## Knepiga/kantfall

```
finns det några bryggerier i dalarna
→ SELECT * FROM bolag WHERE (ulow(sateskommun) LIKE '%dalarna%' OR ulow(postort) LIKE '%falun%' OR ulow(postort) LIKE '%borlänge%') AND (LOWER(sni_1_namn) LIKE '%bryggeri%' OR LOWER(verksamhet) LIKE '%bryggeri%');
```

```
döda bolag inom mode
→ SELECT * FROM bolag WHERE aktiv = 0 AND (LOWER(sni_1_namn) LIKE '%mode%' OR LOWER(sni_1_namn) LIKE '%kläd%' OR LOWER(verksamhet) LIKE '%mode%');
```

```
sök på företaget acme ab
→ SELECT * FROM bolag WHERE LOWER(orgnamn) LIKE '%acme%';
```
