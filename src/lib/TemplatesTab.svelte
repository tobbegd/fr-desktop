<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { loadPrefs } from "$lib/store";

  type Template = { id: number; namn: string; amne: string; brodtext: string; content_type: string; skapad: string };

  type Props = { navigateId?: number | null; onNavigated?: () => void };
  let { navigateId = null, onNavigated }: Props = $props();

  let templates = $state<Template[]>([]);
  let selected = $state<Template | null>(null);
  let sparar = $state(false);
  let sparat = $state(false);

  let testEmail = $state("");
  let testSkickar = $state(false);
  let testFel = $state("");
  let testOk = $state(false);

  async function skickaTest() {
    if (!selected || !testEmail.trim()) return;
    testFel = "";
    testOk = false;
    testSkickar = true;
    try {
      const p = await loadPrefs();
      if (!p.smtpHost) throw new Error("SMTP ej konfigurerat. Gå till Inställningar.");
      await invoke("send_utskick_test", {
        host: p.smtpHost,
        port: p.smtpPort ?? 587,
        encryption: p.smtpEncryption ?? "starttls",
        username: p.smtpUsername ?? "",
        password: p.smtpPassword ?? "",
        fromName: p.smtpFromName ?? "",
        fromEmail: p.smtpFromEmail ?? "",
        replyTo: p.smtpReplyTo ?? "",
        toEmail: testEmail.trim(),
        amne: selected.amne,
        brodtext: selected.brodtext,
        contentType: selected.content_type,
      });
      testOk = true;
      setTimeout(() => { testOk = false; }, 3000);
    } catch (e) {
      testFel = String(e);
    } finally {
      testSkickar = false;
    }
  }

  const PLATSHALLARE = ["{{orgnamn}}", "{{orgnr}}", "{{postort}}", "{{bransch}}"];
  const brodtextPlaceholder = "Skriv din text här. Använd platshållare som {{orgnamn}} för personalisering.";

  async function ladda() {
    templates = await invoke<Template[]>("list_templates");
  }

  function valj(t: Template) {
    selected = { ...t };
    sparat = false;
  }

  function nyTemplate() {
    selected = { id: 0, namn: "Ny template", amne: "", brodtext: "", content_type: "text", skapad: "" };
    sparat = false;
  }

  async function spara() {
    if (!selected) return;
    sparar = true;
    try {
      if (selected.id === 0) {
        const id = await invoke<number>("create_template", {
          namn: selected.namn,
          amne: selected.amne,
          brodtext: selected.brodtext,
          contentType: selected.content_type,
        });
        selected = { ...selected, id };
      } else {
        await invoke("update_template", {
          id: selected.id,
          namn: selected.namn,
          amne: selected.amne,
          brodtext: selected.brodtext,
          contentType: selected.content_type,
        });
      }
      await ladda();
      sparat = true;
      setTimeout(() => { sparat = false; }, 2000);
    } finally {
      sparar = false;
    }
  }

  async function taBort(t: Template) {
    await invoke("delete_template", { id: t.id });
    if (selected?.id === t.id) selected = null;
    await ladda();
  }

  function infogaPlatshallare(ph: string) {
    if (!selected) return;
    selected.brodtext += ph;
  }

  onMount(async () => {
    await ladda();
    if (navigateId != null) {
      const t = templates.find(t => t.id === navigateId);
      if (t) valj(t);
      onNavigated?.();
    }
  });
</script>

<div class="flex h-full gap-0">
  <!-- Vänster: templatelista -->
  <div class="w-56 shrink-0 border-r border-zinc-800 flex flex-col">
    <div class="p-3 border-b border-zinc-800">
      <button
        onclick={nyTemplate}
        class="w-full px-2 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded transition-colors cursor-pointer text-left"
      >+ Ny template</button>
    </div>
    <div class="flex-1 overflow-y-auto">
      {#each templates as t}
        <div
          role="button"
          tabindex="0"
          class="w-full text-left px-4 py-3 border-b border-zinc-800/60 flex items-center justify-between group transition-colors cursor-pointer
            {selected?.id === t.id ? 'bg-zinc-800' : 'bg-zinc-900/40 hover:bg-zinc-900'}"
          onclick={() => valj(t)}
          onkeydown={(e) => e.key === "Enter" && valj(t)}
        >
          <p class="text-sm text-zinc-200 truncate pr-2">{t.namn}</p>
          <button
            class="opacity-0 group-hover:opacity-100 text-zinc-600 hover:text-red-400 text-xs transition-all cursor-pointer px-1 shrink-0"
            onclick={(e) => { e.stopPropagation(); taBort(t); }}
          >✕</button>
        </div>
      {:else}
        <p class="text-xs text-zinc-600 px-4 py-3">Inga templates ännu.</p>
      {/each}
    </div>
  </div>

  <!-- Höger: redigeringsformulär -->
  <div class="flex-1 flex flex-col overflow-hidden">
    {#if selected}
      <div class="flex-1 overflow-y-auto p-5 flex flex-col gap-4">
        <div>
          <label class="text-xs text-zinc-500 block mb-1">Namn</label>
          <input
            bind:value={selected.namn}
            class="w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-zinc-600"
          />
        </div>
        <div>
          <label class="text-xs text-zinc-500 block mb-1">Ämnesrad</label>
          <input
            bind:value={selected.amne}
            placeholder="Ämne..."
            class="w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
          />
        </div>
        <div>
          <label class="text-xs text-zinc-500 block mb-1">Format</label>
          <div class="flex gap-1">
            {#each [{ value: "text", label: "Plaintext" }, { value: "html", label: "HTML" }] as opt}
              <button
                onclick={() => { if (selected) selected.content_type = opt.value; }}
                class="px-3 py-1 text-xs rounded transition-colors cursor-pointer
                  {selected?.content_type === opt.value
                    ? 'bg-zinc-200 text-zinc-900 font-medium'
                    : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200'}"
              >{opt.label}</button>
            {/each}
          </div>
        </div>
        <div class="flex-1 flex flex-col">
          <div class="flex items-center justify-between mb-1">
            <label class="text-xs text-zinc-500">Brödtext</label>
            <div class="flex gap-1">
              {#each PLATSHALLARE as ph}
                <button
                  onclick={() => infogaPlatshallare(ph)}
                  class="text-xs px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 rounded transition-colors cursor-pointer font-mono"
                >{ph}</button>
              {/each}
            </div>
          </div>
          <textarea
            bind:value={selected.brodtext}
            placeholder={brodtextPlaceholder}
            class="flex-1 min-h-64 w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 resize-none font-mono leading-relaxed"
          ></textarea>
        </div>
      </div>
      <div class="border-t border-zinc-800 px-5 py-3 flex items-center gap-3 flex-wrap">
        <button
          onclick={spara}
          disabled={sparar}
          class="px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
        >{sparar ? "Sparar..." : sparat ? "Sparat ✓" : "Spara"}</button>

        <div class="flex items-center gap-2">
          <input
            bind:value={testEmail}
            placeholder="Testadress..."
            type="email"
            onkeydown={(e) => e.key === "Enter" && skickaTest()}
            class="bg-zinc-900 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-500 w-48"
          />
          <button
            onclick={skickaTest}
            disabled={testSkickar || !testEmail.trim()}
            class="px-3 py-1.5 text-xs bg-zinc-800 text-zinc-200 rounded hover:bg-zinc-700 transition-colors cursor-pointer disabled:opacity-40"
          >{testSkickar ? "Skickar..." : "Skicka test"}</button>
          {#if testOk}
            <span class="text-xs text-green-400">Skickat ✓</span>
          {/if}
          {#if testFel}
            <span class="text-xs text-red-400">{testFel}</span>
          {/if}
        </div>
      </div>
    {:else}
      <div class="flex-1 flex items-center justify-center">
        <p class="text-sm text-zinc-600">Välj en template eller skapa ny</p>
      </div>
    {/if}
  </div>
</div>
