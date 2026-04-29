<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { loadPrefs } from "$lib/store";
  import PostaOverlay from "$lib/PostaOverlay.svelte";

  type Template = { id: number; namn: string; amne: string; brodtext: string };
  type Sack = { id: number; namn: string; antal: number };
  type Utskick = { id: number; sack_id: number; sack_namn: string; template_id: number; template_namn: string; skapad: string; status: string; fordrojning_sek: number };

  let templates = $state<Template[]>([]);
  let sackar = $state<Sack[]>([]);
  let utskick = $state<Utskick[]>([]);

  let valdTemplate = $state<Template | null>(null);
  let valdSack = $state<Sack | null>(null);
  let fordrojning = $state(30);
  let skapar = $state(false);

  let postaUtskick = $state<Utskick | null>(null);

  let testUtskickId = $state<number | null>(null);
  let testEmail = $state("");
  let testSkickar = $state(false);
  let testFel = $state("");
  let testOk = $state(false);

  async function ladda() {
    [templates, sackar, utskick] = await Promise.all([
      invoke<Template[]>("list_templates"),
      invoke<Sack[]>("list_sackar"),
      invoke<Utskick[]>("list_utskick"),
    ]);
  }

  async function skapaUtskick() {
    if (!valdTemplate || !valdSack) return;
    skapar = true;
    try {
      await invoke("create_utskick", {
        sackId: valdSack.id,
        templateId: valdTemplate.id,
        fordrojningSek: fordrojning,
      });
      valdTemplate = null;
      valdSack = null;
      await ladda();
    } finally {
      skapar = false;
    }
  }

  async function taBortUtskick(id: number) {
    await invoke("delete_utskick", { id });
    await ladda();
  }

  function oppnaTest(id: number) {
    if (testUtskickId === id) {
      testUtskickId = null;
      testEmail = "";
      testFel = "";
      testOk = false;
    } else {
      testUtskickId = id;
      testEmail = "";
      testFel = "";
      testOk = false;
    }
  }

  async function skickaTest(u: Utskick) {
    if (!testEmail.trim()) return;
    testFel = "";
    testOk = false;
    testSkickar = true;
    try {
      const p = await loadPrefs();
      if (!p.smtpHost) throw new Error("SMTP ej konfigurerat. Gå till Inställningar.");
      const tmpl = templates.find(t => t.id === u.template_id);
      if (!tmpl) throw new Error("Mallen hittades inte.");
      await invoke("send_utskick_test", {
        host: p.smtpHost,
        port: p.smtpPort ?? 587,
        encryption: p.smtpEncryption ?? "starttls",
        username: p.smtpUsername ?? "",
        password: p.smtpPassword ?? "",
        fromName: p.smtpFromName ?? "",
        fromEmail: p.smtpFromEmail ?? "",
        toEmail: testEmail.trim(),
        amne: tmpl.amne,
        brodtext: tmpl.brodtext,
      });
      testOk = true;
    } catch (e) {
      testFel = String(e);
    } finally {
      testSkickar = false;
    }
  }

  onMount(ladda);
</script>

<div class="flex flex-col h-full overflow-hidden">
  <!-- Kopplings-UI -->
  <div class="flex gap-0 border-b border-zinc-800" style="min-height: 260px">
    <!-- E-postmallar -->
    <div class="flex-1 border-r border-zinc-800 flex flex-col">
      <p class="text-xs text-zinc-500 px-4 py-2 border-b border-zinc-800">E-postmall</p>
      <div class="flex-1 overflow-y-auto">
        {#each templates as t}
          <button
            class="w-full text-left px-4 py-3 border-b border-zinc-800/50 transition-colors cursor-pointer
              {valdTemplate?.id === t.id ? 'bg-zinc-700 text-white' : 'hover:bg-zinc-900 text-zinc-300'}"
            onclick={() => valdTemplate = valdTemplate?.id === t.id ? null : t}
          >
            <p class="text-sm">{t.namn}</p>
            {#if t.amne}
              <p class="text-xs text-zinc-500 truncate">{t.amne}</p>
            {/if}
          </button>
        {:else}
          <p class="text-xs text-zinc-600 px-4 py-3">Inga e-postmallar. Skapa en under fliken Templates.</p>
        {/each}
      </div>
    </div>

    <!-- Pil + koppla-knapp i mitten -->
    <div class="w-40 shrink-0 flex flex-col items-center justify-center gap-3 px-3">
      {#if valdTemplate && valdSack}
        <div class="text-center w-full">
          <p class="text-xs text-zinc-400 truncate">{valdTemplate.namn}</p>
          <p class="text-2xl font-bold text-zinc-400 my-1 leading-none">↓</p>
          <p class="text-xs text-zinc-400 truncate">{valdSack.namn}</p>
        </div>
        <div class="flex flex-col gap-2 w-full">
          <div class="flex flex-col items-center gap-1">
            <label class="text-xs text-zinc-500">Fördröjning (s)</label>
            <div class="flex items-center gap-1">
              <button
                onclick={() => fordrojning = Math.max(5, fordrojning - 5)}
                class="w-8 h-8 flex items-center justify-center bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-lg font-bold cursor-pointer transition-colors leading-none"
              >−</button>
              <span class="w-10 text-center text-sm font-medium text-zinc-200">{fordrojning}</span>
              <button
                onclick={() => fordrojning = fordrojning + 5}
                class="w-8 h-8 flex items-center justify-center bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-lg font-bold cursor-pointer transition-colors leading-none"
              >+</button>
            </div>
          </div>
          <button
            onclick={skapaUtskick}
            disabled={skapar}
            class="w-full px-2 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
          >{skapar ? "..." : "Skapa utskick"}</button>
        </div>
      {:else}
        <p class="text-xs text-zinc-600 text-center leading-relaxed">Välj en e-postmall och en säck</p>
        <div class="text-3xl font-bold text-zinc-700">⇄</div>
      {/if}
    </div>

    <!-- Säckar -->
    <div class="flex-1 border-l border-zinc-800 flex flex-col">
      <p class="text-xs text-zinc-500 px-4 py-2 border-b border-zinc-800">Säck</p>
      <div class="flex-1 overflow-y-auto">
        {#each sackar as s}
          <button
            class="w-full text-left px-4 py-3 border-b border-zinc-800/50 transition-colors cursor-pointer
              {valdSack?.id === s.id ? 'bg-zinc-700 text-white' : 'hover:bg-zinc-900 text-zinc-300'}"
            onclick={() => valdSack = valdSack?.id === s.id ? null : s}
          >
            <p class="text-sm">{s.namn}</p>
            <p class="text-xs text-zinc-500">{s.antal} bolag</p>
          </button>
        {:else}
          <p class="text-xs text-zinc-600 px-4 py-3">Inga säckar. Skapa en under fliken Säckar.</p>
        {/each}
      </div>
    </div>
  </div>

  <!-- Skapade utskick -->
  <div class="flex-1 overflow-y-auto">
    <p class="text-xs text-zinc-500 px-4 py-2 border-b border-zinc-800 sticky top-0 bg-zinc-950">Skapade utskick</p>
    {#each utskick as u}
      <div class="border-b border-zinc-800/50">
        <div class="flex items-center justify-between px-4 py-3 group">
          <div>
            <div class="flex items-center gap-2">
              <span class="text-sm text-zinc-200">{u.template_namn}</span>
              <span class="text-zinc-500 font-bold">→</span>
              <span class="text-sm text-zinc-200">{u.sack_namn}</span>
            </div>
            <p class="text-xs text-zinc-600">{u.skapad.slice(0, 16)} · {u.fordrojning_sek}s fördröjning · {u.status}</p>
          </div>
          <div class="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-all">
            <button
              class="text-zinc-500 hover:text-zinc-200 text-xs cursor-pointer px-2 py-1 rounded hover:bg-zinc-800 transition-colors"
              onclick={() => oppnaTest(u.id)}
            >Test skicka</button>
            <button
              class="text-zinc-200 hover:text-white text-xs cursor-pointer px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 transition-colors font-medium"
              onclick={() => postaUtskick = u}
            >Posta säck</button>
            <button
              class="text-zinc-600 hover:text-red-400 text-xs cursor-pointer px-1"
              onclick={() => taBortUtskick(u.id)}
            >✕</button>
          </div>
        </div>

        {#if testUtskickId === u.id}
          <div class="px-4 pb-3 flex items-center gap-2 max-w-sm">
            <input
              type="email"
              bind:value={testEmail}
              placeholder="din@email.se"
              class="flex-1 bg-zinc-900 border border-zinc-700 rounded px-3 py-1.5 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-500"
              onkeydown={(e) => e.key === 'Enter' && skickaTest(u)}
            />
            <button
              onclick={() => skickaTest(u)}
              disabled={testSkickar || !testEmail.trim()}
              class="px-3 py-1.5 text-xs bg-zinc-700 hover:bg-zinc-600 text-zinc-200 rounded transition-colors cursor-pointer disabled:opacity-50"
            >{testSkickar ? "Skickar..." : "Skicka"}</button>
            {#if testOk}
              <span class="text-xs text-green-400">Skickat!</span>
            {/if}
            {#if testFel}
              <span class="text-xs text-red-400 truncate max-w-xs" title={testFel}>{testFel}</span>
            {/if}
          </div>
        {/if}
      </div>
    {:else}
      <p class="text-xs text-zinc-600 px-4 py-4">Inga utskick skapade ännu.</p>
    {/each}
  </div>
</div>

{#if postaUtskick}
  <PostaOverlay
    utskick={postaUtskick}
    onClose={() => postaUtskick = null}
    onDone={() => { ladda(); }}
  />
{/if}
