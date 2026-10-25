<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Modal from "$lib/Modal.svelte";
  import CrmList from "$lib/CrmList.svelte";

  // Enda CRM-ytan i appen. Två lägen:
  // - "list": fritextsök + statusfilter + lista över alla taggade bolag
  //   (CrmList.svelte) — nås via header-ikonen eller startup-påminnelsen.
  // - "edit": detaljvy för ETT bolag i taget (status, notis, återkomstdatum,
  //   scrapad bolagsinfo, egna kontakter) — nås via högerklicksmenyns
  //   "CRM: Hantera..."/"CRM: Hantera (N)", eller via "Redigera"/"Redigera
  //   alla" i listläget. Kön (flera bolag) hanteras HÄR internt, så alla
  //   anroparen behöver göra är att skicka en lista att börja på.
  type Bolag = { orgnr: string; orgnamn: string };

  type Props = {
    dbPath: string;
    // Om satt (icke-tom): öppnas direkt i redigeringsläge på denna kö.
    // Om tom/utelämnad: öppnas i listläge.
    initial?: Bolag[];
    onclose: () => void;
    onchanged: () => void;
  };
  let { dbPath, initial = [], onclose, onchanged }: Props = $props();

  let mode = $state<"list" | "edit">("list");
  let queue = $state<Bolag[]>([]);
  let queueIdx = $state(0);
  const current = $derived(queue[queueIdx] ?? null);

  // $props() värden är reaktiva — läs `initial` i en $effect (inte direkt i
  // state-initialiseraren) så prop-ändringar faktiskt tas med, inte bara
  // värdet vid första render.
  $effect(() => {
    if (initial.length > 0) {
      queue = initial;
      queueIdx = 0;
      mode = "edit";
    }
  });

  function editSingle(bolag: Bolag) {
    queue = [bolag];
    queueIdx = 0;
    mode = "edit";
  }

  function editAll(bolag: Bolag[]) {
    queue = bolag;
    queueIdx = 0;
    mode = "edit";
  }

  function backToList() {
    mode = "list";
  }

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
    if (!current) return;
    const orgnr = current.orgnr;
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
    if (!current) return;
    const orgnr = current.orgnr;
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
    if (!current) return;
    const rows = await invoke<{ id: number; orgnr: string; typ: string; varde: string }[]>("list_crm_kontakter", { orgnr: current.orgnr });
    egnaKontakter = rows.map(r => ({ id: r.id, typ: r.typ as "telefon" | "email", varde: r.varde }));
  }

  async function leggTillKontakt() {
    if (!current) return;
    const varde = nyttVarde.trim();
    if (!varde) return;
    kontaktBusy = true;
    try {
      await invoke("add_crm_kontakt", { orgnr: current.orgnr, typ: nyTyp, varde });
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

  async function laddaStatus() {
    if (!current) return;
    loading = true;
    try {
      const rows = await invoke<{ orgnr: string; status: string; notis: string; aterkom_datum: string | null }[]>("list_crm_bolag");
      const r = rows.find(x => x.orgnr === current!.orgnr);
      status = r?.status ?? null;
      notis = r?.notis ?? "";
      aterkomDatum = r?.aterkom_datum ?? "";
    } finally {
      loading = false;
    }
  }

  function laddaAllt() {
    kontakterExpanded = false;
    kontakterLoaded = false;
    kontakterRader = [];
    laddaStatus();
    laddaBolagInfo();
    laddaEgnaKontakter();
  }

  // Ladda om allt när man byter bolag (bläddring i kön) eller går in i
  // redigeringsläge på ett nytt bolag.
  let senastLaddadOrgnr = $state<string | null>(null);
  $effect(() => {
    if (mode === "edit" && current && current.orgnr !== senastLaddadOrgnr) {
      senastLaddadOrgnr = current.orgnr;
      laddaAllt();
    }
  });

  function flashSaved() {
    saved = true;
    setTimeout(() => saved = false, 1200);
  }

  async function save() {
    if (!status || !current) return;
    saving = true;
    try {
      await invoke("set_crm_status", {
        orgnr: current.orgnr,
        orgnamn: current.orgnamn,
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
    if (!current) return;
    saving = true;
    try {
      await invoke("remove_crm_bolag", { orgnr: current.orgnr });
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
    if (mode !== "edit") return;
    // Inte när man skriver i notis/kontakt-fält — bara global bläddring.
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "TEXTAREA" || tag === "INPUT") return;
    if (e.key === "ArrowLeft" && queueIdx > 0) queueIdx -= 1;
    if (e.key === "ArrowRight" && queueIdx < queue.length - 1) queueIdx += 1;
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<Modal title={mode === "edit" && current ? `CRM — ${current.orgnamn}` : "CRM"} subtitle={mode === "edit" ? current?.orgnr : undefined} {onclose}>
  {#snippet headerExtra()}
    {#if mode === "edit"}
      <div class="flex items-center gap-2 text-xs text-zinc-400 shrink-0">
        <button
          class="px-2.5 py-1.5 rounded hover:bg-zinc-800 hover:text-white transition-colors cursor-pointer"
          onclick={backToList}
        >← Lista</button>
        {#if queue.length > 1}
          <button
            class="px-3 py-1.5 rounded border border-zinc-700 hover:bg-zinc-800 hover:text-white hover:border-zinc-600 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-default disabled:hover:bg-transparent disabled:hover:border-zinc-700 font-medium"
            onclick={() => queueIdx -= 1}
            disabled={queueIdx === 0}
            title="Föregående (←)"
          >← Föregående</button>
          <span class="tabular-nums">{queueIdx + 1} / {queue.length}</span>
          <button
            class="px-3 py-1.5 rounded border border-zinc-700 hover:bg-zinc-800 hover:text-white hover:border-zinc-600 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-default disabled:hover:bg-transparent disabled:hover:border-zinc-700 font-medium"
            onclick={() => queueIdx += 1}
            disabled={queueIdx === queue.length - 1}
            title="Nästa (→)"
          >Nästa →</button>
        {/if}
      </div>
    {/if}
  {/snippet}

  {#if mode === "list"}
    <CrmList onEdit={editSingle} onEditAll={editAll} />
  {:else if current}
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
              onchange={(e) => (e.currentTarget as HTMLInputElement).blur()}
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
  {/if}
</Modal>
