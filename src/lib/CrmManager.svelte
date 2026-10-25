<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Modal from "$lib/Modal.svelte";

  // CRM-hanterare för ETT bolag, öppnad från högerklicksmenyn i sökresultatet
  // ("CRM: Hantera..."). Samlar status, notis, återkomstdatum, egna
  // telefonnummer/mejladresser och borttagning på ett ställe, till skillnad
  // från de separata snabbvalen (Het/Kall/Återkomst) som redan finns direkt
  // i högerklicksmenyn.
  type Props = {
    dbPath: string;
    orgnr: string;
    orgnamn: string;
    queuePosition?: { index: number; total: number } | null;
    onprev?: () => void;
    onnext?: () => void;
    onclose: () => void;
    onchanged: () => void;
  };
  let { dbPath, orgnr, orgnamn, queuePosition = null, onprev, onnext, onclose, onchanged }: Props = $props();

  const STATUS_LABELS: Record<string, string> = {
    het: "Het",
    kall: "Kall",
    aterkom: "Återkomst senare",
  };
  const STATUS_COLORS: Record<string, { text: string; border: string }> = {
    het:     { text: "text-red-400",   border: "border-red-900/60" },
    kall:    { text: "text-sky-400",   border: "border-sky-900/60" },
    aterkom: { text: "text-amber-400", border: "border-amber-900/60" },
  };

  let status = $state<string | null>(null);
  let notis = $state("");
  let aterkomDatum = $state("");
  let loading = $state(true);
  let saving = $state(false);
  let saved = $state(false);

  // Scrapad bolagsinfo — hämtas read-only från den nedladdade bolagsdatabasen
  // (dbPath). Visas bara som referens så man slipper växla till sökresultatet
  // för att se kontaktvägar medan man taggar/noterar bolaget.
  let scrapTelefon = $state<string | null>(null);
  let scrapEmail = $state<string | null>(null);
  let adress = $state<string | null>(null);
  let harKontakter = $state(false);
  let kontakterAntal = $state(0);
  let infoLoading = $state(true);

  async function laddaBolagInfo() {
    infoLoading = true;
    try {
      const res = await invoke<{ columns: string[]; rows: unknown[][] }>("query_db", {
        dbPath,
        sql: `SELECT telefon, email, gatuadress, postnummer, postort, har_kontakter
              FROM bolag WHERE orgnr = '${orgnr.replace(/'/g, "''")}' LIMIT 1`,
      });
      const cols = res.columns;
      const row = res.rows[0];
      if (row) {
        const get = (c: string) => { const i = cols.indexOf(c); const v = i !== -1 ? row[i] : null; return (v !== null && v !== undefined && v !== "") ? String(v) : null; };
        scrapTelefon = get("telefon");
        scrapEmail = get("email");
        const gatuadress = get("gatuadress");
        const postnummer = get("postnummer");
        const postort = get("postort");
        adress = [gatuadress, [postnummer, postort].filter(Boolean).join(" ")].filter(Boolean).join(", ") || null;
        harKontakter = get("har_kontakter") === "1";
      }
      if (harKontakter) {
        const kRes = await invoke<{ columns: string[]; rows: unknown[][] }>("query_db", {
          dbPath,
          sql: `SELECT COUNT(*) FROM kontakter WHERE replace(orgnr, '-', '') = replace('${orgnr.replace(/'/g, "''")}', '-', '')`,
        });
        kontakterAntal = Number(kRes.rows[0]?.[0] ?? 0);
      }
    } catch {
      // Bolagsdatabasen kan sakna kolumner/tabeller i äldre versioner — visa bara det som finns.
    } finally {
      infoLoading = false;
    }
  }

  // Scrapad kontaktdata (namngivna personer/mailboxar/sociala länkar) —
  // samma källa som KontakterPanel, men fullständig text hämtas lat (bara
  // vid utfällning) eftersom bolag kan ha gott om rader här.
  type ScrapKontakt = {
    kategori: string;
    namn: string | null;
    titel: string | null;
    email: string | null;
    telefon: string | null;
    lank: string | null;
  };
  let kontakterExpanded = $state(false);
  let kontakterRader = $state<ScrapKontakt[]>([]);
  let kontakterLoading = $state(false);
  let kontakterLoaded = $state(false);

  async function toggleKontakter() {
    kontakterExpanded = !kontakterExpanded;
    if (kontakterExpanded && !kontakterLoaded) {
      kontakterLoading = true;
      try {
        const res = await invoke<{ columns: string[]; rows: unknown[][] }>("query_db", {
          dbPath,
          sql: `SELECT kategori, namn, titel, email, telefon, lank
                FROM kontakter
                WHERE replace(orgnr, '-', '') = replace('${orgnr.replace(/'/g, "''")}', '-', '')`,
        });
        const cols = res.columns;
        kontakterRader = res.rows.map(r => {
          const o: Record<string, unknown> = {};
          cols.forEach((c, i) => o[c] = r[i]);
          return o as unknown as ScrapKontakt;
        });
        kontakterLoaded = true;
      } finally {
        kontakterLoading = false;
      }
    }
  }

  // Egna tillagda telefonnummer/mejladresser — separat tabell i mailutskick.db
  // (crm_kontakt), överlever databas-nedladdningar. Flera per bolag tillåts.
  type EgenKontakt = { id: number; typ: "telefon" | "email"; varde: string };
  let egnaKontakter = $state<EgenKontakt[]>([]);
  let nyttVarde = $state("");
  let nyTyp = $state<"telefon" | "email">("telefon");
  let kontaktBusy = $state(false);

  async function laddaEgnaKontakter() {
    const rows = await invoke<{ id: number; orgnr: string; typ: string; varde: string }[]>("list_crm_kontakter", { orgnr });
    egnaKontakter = rows.map(r => ({ id: r.id, typ: r.typ as "telefon" | "email", varde: r.varde }));
  }

  async function leggTillKontakt() {
    const varde = nyttVarde.trim();
    if (!varde) return;
    kontaktBusy = true;
    try {
      await invoke("add_crm_kontakt", { orgnr, typ: nyTyp, varde });
      nyttVarde = "";
      await laddaEgnaKontakter();
    } finally {
      kontaktBusy = false;
    }
  }

  async function taBortKontakt(id: number) {
    kontaktBusy = true;
    try {
      await invoke("remove_crm_kontakt", { id });
      await laddaEgnaKontakter();
    } finally {
      kontaktBusy = false;
    }
  }

  async function ladda() {
    loading = true;
    try {
      const rows = await invoke<{ orgnr: string; status: string; notis: string; aterkom_datum: string | null }[]>("list_crm_bolag");
      const r = rows.find(x => x.orgnr === orgnr);
      status = r?.status ?? null;
      notis = r?.notis ?? "";
      aterkomDatum = r?.aterkom_datum ?? "";
    } finally {
      loading = false;
    }
  }
  ladda();
  laddaBolagInfo();
  laddaEgnaKontakter();

  // Vid bläddring i en kö (flera markerade rader) återanvänds samma
  // komponentinstans med ny orgnr-prop — ladda om allt state för det nya
  // bolaget och stäng ev. öppen "scrapad kontaktdata"-utfällning.
  let forraOrgnr = $state<string | null>(null);
  $effect(() => {
    if (forraOrgnr === null) {
      forraOrgnr = orgnr;
      return;
    }
    if (orgnr !== forraOrgnr) {
      forraOrgnr = orgnr;
      kontakterExpanded = false;
      kontakterLoaded = false;
      kontakterRader = [];
      ladda();
      laddaBolagInfo();
      laddaEgnaKontakter();
    }
  });

  function flashSaved() {
    saved = true;
    setTimeout(() => saved = false, 1200);
  }

  async function save() {
    if (!status) return;
    saving = true;
    try {
      await invoke("set_crm_status", {
        orgnr,
        orgnamn,
        status,
        notis,
        aterkomDatum: status === "aterkom" && aterkomDatum ? aterkomDatum : null,
      });
      onchanged();
      flashSaved();
    } finally {
      saving = false;
    }
  }

  async function removeAll() {
    saving = true;
    try {
      await invoke("remove_crm_bolag", { orgnr });
      status = null;
      notis = "";
      aterkomDatum = "";
      onchanged();
    } finally {
      saving = false;
    }
  }

  let copiedValue = $state("");
  function copy(v: string) {
    navigator.clipboard.writeText(v);
    copiedValue = v;
    setTimeout(() => { if (copiedValue === v) copiedValue = ""; }, 1200);
  }

  function onKeyDown(e: KeyboardEvent) {
    // Inte när man skriver i notis/kontakt-fält — bara global bläddring.
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "TEXTAREA" || tag === "INPUT") return;
    if (e.key === "ArrowLeft" && onprev && queuePosition && queuePosition.index > 0) onprev();
    if (e.key === "ArrowRight" && onnext && queuePosition && queuePosition.index < queuePosition.total - 1) onnext();
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<Modal title="CRM — {orgnamn}" subtitle={orgnr} {onclose}>
  {#snippet headerExtra()}
    {#if queuePosition}
      <div class="flex items-center gap-2 text-xs text-zinc-400 shrink-0">
        <button
          class="px-3 py-1.5 rounded border border-zinc-700 hover:bg-zinc-800 hover:text-white hover:border-zinc-600 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-default disabled:hover:bg-transparent disabled:hover:border-zinc-700 font-medium"
          onclick={onprev}
          disabled={queuePosition.index === 0}
          title="Föregående (←)"
        >← Föregående</button>
        <span class="tabular-nums">{queuePosition.index + 1} / {queuePosition.total}</span>
        <button
          class="px-3 py-1.5 rounded border border-zinc-700 hover:bg-zinc-800 hover:text-white hover:border-zinc-600 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-default disabled:hover:bg-transparent disabled:hover:border-zinc-700 font-medium"
          onclick={onnext}
          disabled={queuePosition.index === queuePosition.total - 1}
          title="Nästa (→)"
        >Nästa →</button>
      </div>
    {/if}
  {/snippet}

  {#if loading}
    <p class="text-xs text-zinc-600">Laddar...</p>
  {:else}
    <div class="flex flex-col gap-5 max-w-md">
      <!-- Scrapad bolagsinfo, read-only referens -->
      <div>
        <p class="text-xs text-zinc-500 mb-1.5">Bolagsinfo</p>
        {#if infoLoading}
          <p class="text-xs text-zinc-600">Laddar...</p>
        {:else}
          <div class="flex flex-col gap-1 text-xs">
            {#if scrapTelefon}
              <button
                class="text-left rounded px-2 py-1 transition-colors cursor-pointer flex items-center gap-2
                  {copiedValue === scrapTelefon ? 'bg-green-700 text-white' : 'bg-zinc-900 border border-zinc-800 hover:border-zinc-600 text-zinc-300'}"
                onclick={() => copy(scrapTelefon!)}
              ><span class="text-zinc-600">📞</span> {copiedValue === scrapTelefon ? "Kopierad!" : scrapTelefon}</button>
            {/if}
            {#if scrapEmail}
              <button
                class="text-left rounded px-2 py-1 transition-colors cursor-pointer flex items-center gap-2
                  {copiedValue === scrapEmail ? 'bg-green-700 text-white' : 'bg-zinc-900 border border-zinc-800 hover:border-zinc-600 text-zinc-300'}"
                onclick={() => copy(scrapEmail!)}
              ><span class="text-zinc-600">✉️</span> {copiedValue === scrapEmail ? "Kopierad!" : scrapEmail}</button>
            {/if}
            {#if adress}
              <div class="rounded px-2 py-1 bg-zinc-900 border border-zinc-800 text-zinc-300 flex items-center gap-2">
                <span class="text-zinc-600">📍</span> {adress}
              </div>
            {/if}
            {#if !scrapTelefon && !scrapEmail && !adress}
              <p class="text-zinc-600">Ingen bolagsinfo hittades.</p>
            {/if}
            <div class="rounded border border-dashed border-zinc-800">
              <button
                class="w-full text-left px-2 py-1 text-zinc-500 flex items-center gap-2 cursor-pointer hover:text-zinc-300 transition-colors disabled:cursor-default disabled:hover:text-zinc-500"
                onclick={toggleKontakter}
                disabled={!harKontakter}
              >
                <span class="text-zinc-600">📇</span>
                {harKontakter ? `Scrapad kontaktdata (${kontakterAntal})` : "Ingen scrapad kontaktdata"}
                {#if harKontakter}
                  <span class="ml-auto text-zinc-600">{kontakterExpanded ? "▲" : "▼"}</span>
                {/if}
              </button>
              {#if kontakterExpanded}
                <div class="px-2 pb-2 flex flex-col gap-1 border-t border-zinc-800/60 pt-1.5">
                  {#if kontakterLoading}
                    <p class="text-zinc-600">Laddar...</p>
                  {:else if kontakterRader.length === 0}
                    <p class="text-zinc-600">Ingen data hittades.</p>
                  {:else}
                    {#each kontakterRader as k}
                      <div class="rounded bg-zinc-950 border border-zinc-800 px-2 py-1 flex flex-col gap-0.5">
                        <div class="flex items-center gap-1.5">
                          <span class="text-zinc-600 text-[10px] uppercase shrink-0">{k.kategori}</span>
                          {#if k.namn}<span class="text-zinc-300 truncate">{k.namn}</span>{/if}
                          {#if k.titel}<span class="text-zinc-600 truncate">· {k.titel}</span>{/if}
                        </div>
                        {#if k.email || k.telefon || k.lank}
                          <div class="flex flex-wrap gap-x-3 text-zinc-500">
                            {#if k.email}<span>{k.email}</span>{/if}
                            {#if k.telefon}<span>{k.telefon}</span>{/if}
                            {#if k.lank}<span class="truncate">{k.lank}</span>{/if}
                          </div>
                        {/if}
                      </div>
                    {/each}
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <!-- Egna tillagda telefonnummer/mejladresser -->
      <div>
        <p class="text-xs text-zinc-500 mb-1.5">Egna kontaktuppgifter</p>
        {#if egnaKontakter.length > 0}
          <div class="flex flex-col gap-1 mb-2">
            {#each egnaKontakter as k}
              <div class="flex items-center gap-2 rounded px-2 py-1 bg-zinc-900 border border-zinc-800 text-xs text-zinc-300 group">
                <span class="text-zinc-600">{k.typ === "telefon" ? "📞" : "✉️"}</span>
                <span class="flex-1 truncate">{k.varde}</span>
                <button
                  class="text-zinc-600 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer px-1"
                  onclick={() => taBortKontakt(k.id)}
                  disabled={kontaktBusy}
                >✕</button>
              </div>
            {/each}
          </div>
        {/if}
        <div class="flex gap-1.5">
          <select
            bind:value={nyTyp}
            class="bg-zinc-950 border border-zinc-700 rounded px-1.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-zinc-500 cursor-pointer"
          >
            <option value="telefon">Tel</option>
            <option value="email">Email</option>
          </select>
          <input
            bind:value={nyttVarde}
            placeholder={nyTyp === "telefon" ? "Telefonnummer..." : "E-postadress..."}
            onkeydown={(e) => e.key === "Enter" && leggTillKontakt()}
            class="flex-1 bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-500"
          />
          <button
            class="px-2.5 py-1.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded transition-colors cursor-pointer disabled:opacity-40"
            onclick={leggTillKontakt}
            disabled={kontaktBusy || !nyttVarde.trim()}
          >Lägg till</button>
        </div>
      </div>

      <div>
        <p class="text-xs text-zinc-500 mb-1.5">Status</p>
        <div class="flex gap-1.5">
          {#each ["het", "kall", "aterkom"] as s}
            <button
              class="flex-1 text-xs rounded px-2 py-1.5 border transition-colors cursor-pointer
                {status === s ? (STATUS_COLORS[s].border + ' ' + STATUS_COLORS[s].text + ' bg-zinc-800') : 'border-zinc-700 text-zinc-400 hover:text-zinc-200'}"
              onclick={() => status = s}
            >{STATUS_LABELS[s]}</button>
          {/each}
        </div>
      </div>

      {#if status === "aterkom"}
        <div>
          <label class="text-xs text-zinc-500 mb-1 block" for="crmm-datum">Återkomstdatum</label>
          <input
            id="crmm-datum"
            type="date"
            bind:value={aterkomDatum}
            class="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-zinc-500"
          />
        </div>
      {/if}

      <div>
        <label class="text-xs text-zinc-500 mb-1 block" for="crmm-notis">Notis</label>
        <textarea
          id="crmm-notis"
          bind:value={notis}
          rows="5"
          placeholder="T.ex. Pratade med VD, återkom efter sommaren..."
          class="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-500 resize-none"
        ></textarea>
      </div>

      <div class="flex items-center gap-2 justify-between pt-1">
        <button
          class="px-3 py-1.5 text-xs text-zinc-500 hover:text-red-400 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-default"
          onclick={removeAll}
          disabled={saving || status === null}
        >Ta bort CRM-status</button>
        <button
          class="px-3 py-1.5 text-xs font-medium rounded cursor-pointer transition-colors disabled:opacity-40 disabled:cursor-default
            {saved ? 'bg-emerald-600 text-white' : 'bg-white text-zinc-900 hover:bg-zinc-200'}"
          onclick={save}
          disabled={saving || !status}
        >{saved ? "Sparad ✓" : "Spara"}</button>
      </div>
    </div>
  {/if}
</Modal>
