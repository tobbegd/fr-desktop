<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { buildPrompt } from "$lib/aiPrompt";

  type Props = { dbPath: string };
  let { dbPath }: Props = $props();

  let testPrompt = $state("Hämta de 5 senaste företagen från tabellen bolag, sorterat på organisationsnummer.");



  // Claude — API-nyckel och modell hanteras server-sidan via Cloudflare-proxy
  let claudeTesting = $state(false);
  let claudeTestResult = $state("");
  let claudeTestError = $state("");

  async function runGeminiTest() {
    geminiTesting = true;
    geminiTestResult = "";
    geminiTestError = "";
    try {
      const schema = await invoke<Record<string, string[]>>("get_schema", { dbPath });
      geminiTestResult = await invoke<string>("query_gemini", {
        apiKey: geminiApiKey,
        model: geminiModel,
        prompt: buildPrompt(schema, testPrompt),
      });
    } catch (e) {
      geminiTestError = String(e);
    } finally {
      geminiTesting = false;
    }
  }

  async function runClaudeTest() {
    claudeTesting = true;
    claudeTestResult = "";
    claudeTestError = "";
    try {
      const schema = await invoke<Record<string, string[]>>("get_schema", { dbPath });
      claudeTestResult = await invoke<string>("query_claude", {
        apiKey: "",
        model: "",
        prompt: buildPrompt(schema, testPrompt),
      });
    } catch (e) {
      claudeTestError = String(e);
    } finally {
      claudeTesting = false;
    }
  }



</script>

<div class="flex flex-col max-w-xl">
  <p class="text-sm text-zinc-400 mb-6 leading-relaxed">
    Claude är Anthropics AI och körs i <strong class="text-zinc-200">molnet</strong>.
    API-nyckeln ingår i prenumerationen — ingen konfiguration behövs.
  </p>

  <h2 class="text-sm font-medium text-zinc-200 mb-3">Testkörning</h2>
  <div class="flex flex-col gap-3">
    <textarea
      bind:value={testPrompt}
      rows="2"
      class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 resize-none"
    ></textarea>
    <button
      class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
      onclick={runClaudeTest} disabled={claudeTesting}
    >{claudeTesting ? "Genererar..." : "Generera SQL"}</button>
    {#if claudeTestResult}
      <pre class="bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-xs text-green-300 font-mono whitespace-pre-wrap">{claudeTestResult}</pre>
    {/if}
    {#if claudeTestError}
      <p class="text-xs text-red-400">{claudeTestError}</p>
    {/if}
  </div>
</div>
