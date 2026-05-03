<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { loadPrefs, savePrefs } from "$lib/store";
  import { buildPrompt } from "$lib/aiPrompt";

  type Props = { dbPath: string };
  let { dbPath }: Props = $props();

  const PRESET_MODELS = [
    { name: "qwen2.5-coder:7b",    label: "Qwen 2.5 Coder 7B", size: "4.7 GB", note: "Rekommenderas" },
    { name: "qwen2.5-coder:3b",    label: "Qwen 2.5 Coder 3B", size: "2.0 GB", note: "" },
    { name: "llama3.2:3b",         label: "Llama 3.2 3B",       size: "2.0 GB", note: "" },
  ];

  type OllamaStatus = "unknown" | "running" | "offline";
  type ModelInfo = { name: string; size: number };

  let ollamaStatus = $state<OllamaStatus>("unknown");
  let checking = $state(false);
  let lastCheckFailed = $state(false);
  let showUninstall = $state(false);

  let models = $state<ModelInfo[]>([]);
  let loadingModels = $state(false);
  let deletingModel = $state("");

  let activeModel = $state("");
  let customModelInput = $state("");

  let pulling = $state(false);
  let pullingModel = $state("");
  let pullStatus = $state("");
  let pullProgress = $state(0);

  let testPrompt = $state("Hämta de 5 senaste företagen från tabellen bolag, sorterat på organisationsnummer.");
  let testResult = $state("");
  let testing = $state(false);
  let testError = $state("");

  let activeTab = $state("gemini");

  // Claude
  type GeminiModel = { name: string; display_name: string };
  let geminiModels = $state<GeminiModel[]>([]);
  let geminiModelsLoading = $state(false);
  let geminiModelsError = $state("");
  let geminiApiKey = $state("");
  let geminiModel = $state("gemini-2.5-flash");

  async function loadGeminiModels() {
    if (!geminiApiKey) return;
    geminiModelsLoading = true;
    geminiModelsError = "";
    try {
      geminiModels = await invoke<GeminiModel[]>("list_gemini_models", { apiKey: geminiApiKey });
      if (geminiModels.length && !geminiModels.some(m => m.name === geminiModel)) {
        geminiModel = geminiModels[0].name;
      }
    } catch (e) {
      geminiModelsError = String(e);
    } finally {
      geminiModelsLoading = false;
    }
  }
  let geminiSaving = $state(false);
  let geminiSaved = $state(false);

  let currentOs = $state("linux");

  let installing = $state(false);
  let installStatus = $state("");
  let installError = $state("");

  async function installOllama() {
    installing = true;
    installStatus = "Förbereder...";
    installError = "";
    const unlisten = await listen<string>("ollama-install-status", (e) => {
      installStatus = e.payload;
    });
    try {
      await invoke("install_ollama");
      if (installStatus !== "terminal-opened") {
        installStatus = "Installeraren startad — klicka Kontrollera när den är klar.";
      } else {
        installStatus = "Terminal öppnad — klicka Kontrollera när installationen är klar.";
      }
    } catch (e) {
      const msg = String(e);
      if (msg === "unsupported") {
        installError = "";
      } else {
        installError = msg;
      }
    } finally {
      unlisten();
      installing = false;
    }
  }

  loadPrefs().then(p => {
    if (p.aiModel) activeModel = p.aiModel;
    if (p.geminiApiKey) { geminiApiKey = p.geminiApiKey; loadGeminiModels(); }
    if (p.geminiModel) geminiModel = p.geminiModel;
    if (p.groqApiKey) { groqApiKey = p.groqApiKey; loadOpenRouterModels(); }
    if (p.groqModel) groqModel = p.groqModel;
  });

  async function saveGeminiSettings() {
    geminiSaving = true;
    geminiSaved = false;
    await savePrefs({ geminiApiKey, geminiModel });
    geminiSaving = false;
    geminiSaved = true;
    setTimeout(() => { geminiSaved = false; }, 2000);
  }

  let geminiTestResult = $state("");
  let geminiTesting = $state(false);
  let geminiTestError = $state("");

  // Groq
  let groqApiKey = $state("");
  let groqModel = $state("");
  let groqSaving = $state(false);
  let groqSaved = $state(false);
  let groqTesting = $state(false);
  let groqTestResult = $state("");
  let groqTestError = $state("");
  type OpenRouterModel = { id: string; name: string };
  let openRouterModels = $state<OpenRouterModel[]>([]);
  let openRouterModelsLoading = $state(false);
  let openRouterModelsError = $state("");

  async function loadOpenRouterModels() {
    if (!groqApiKey) return;
    openRouterModelsLoading = true;
    openRouterModelsError = "";
    try {
      const raw = await invoke<{ id: string; name: string }[]>("list_openrouter_models", { apiKey: groqApiKey });
      openRouterModels = raw.map(m => ({ id: m.id, name: m.name ?? m.id }));
      if (openRouterModels.length && !openRouterModels.some(m => m.id === groqModel)) {
        groqModel = openRouterModels[0].id;
      }
    } catch (e) {
      openRouterModelsError = String(e);
    } finally {
      openRouterModelsLoading = false;
    }
  }

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

  async function saveGroqSettings() {
    groqSaving = true;
    groqSaved = false;
    groqApiKey = groqApiKey.trim();
    await savePrefs({ groqApiKey, groqModel });
    groqSaving = false;
    groqSaved = true;
    setTimeout(() => { groqSaved = false; }, 2000);
  }

  async function runGroqTest() {
    groqTesting = true;
    groqTestResult = "";
    groqTestError = "";
    try {
      const schema = await invoke<Record<string, string[]>>("get_schema", { dbPath });
      groqTestResult = await invoke<string>("query_groq", {
        apiKey: groqApiKey,
        model: groqModel,
        prompt: buildPrompt(schema, testPrompt),
      });
    } catch (e) {
      groqTestError = String(e);
    } finally {
      groqTesting = false;
    }
  }

  invoke<string>("get_os").then(os => { currentOs = os; });

  async function checkOllama() {
    checking = true;
    lastCheckFailed = false;
    ollamaStatus = "unknown";
    models = [];
    try {
      const running = await invoke<boolean>("check_ollama");
      ollamaStatus = running ? "running" : "offline";
      if (running) await loadModels();
      else lastCheckFailed = true;
    } finally {
      checking = false;
    }
  }

  async function loadModels() {
    loadingModels = true;
    try {
      models = await invoke<ModelInfo[]>("list_ollama_models");
      if (activeModel && !models.some(m => m.name === activeModel)) {
        activeModel = models[0]?.name ?? "";
        await savePrefs({ aiModel: activeModel });
      }
    } catch {
      models = [];
    } finally {
      loadingModels = false;
    }
  }

  async function setActiveModel(name: string) {
    activeModel = name;
    await savePrefs({ aiModel: name });
  }

  async function deleteModel(name: string) {
    deletingModel = name;
    try {
      await invoke("delete_ollama_model", { model: name });
      if (activeModel === name) {
        activeModel = "";
        await savePrefs({ aiModel: "" });
      }
      await loadModels();
    } catch (e) {
      alert(`Kunde inte ta bort: ${e}`);
    } finally {
      deletingModel = "";
    }
  }

  async function pullModel(model: string) {
    pulling = true;
    pullingModel = model;
    pullStatus = "Förbereder...";
    pullProgress = 0;

    const unlisten = await listen<{ status: string; completed?: number; total?: number }>(
      "ollama-pull-progress",
      (e) => {
        pullStatus = e.payload.status;
        if (e.payload.total && e.payload.total > 0) {
          pullProgress = Math.round(((e.payload.completed ?? 0) / e.payload.total) * 100);
        }
      }
    );

    try {
      await invoke("pull_ollama_model", { model });
      await loadModels();
      if (!activeModel) await setActiveModel(model);
    } catch (e) {
      pullStatus = `Fel: ${e}`;
    } finally {
      unlisten();
      pulling = false;
      pullingModel = "";
    }
  }

  function isInstalled(name: string) {
    return models.some(m => m.name === name);
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }

  async function runTest() {
    testing = true;
    testResult = "";
    testError = "";
    const model = activeModel || models[0]?.name;
    if (!model) { testError = "Ingen aktiv modell."; testing = false; return; }
    try {
      const schema = await invoke<Record<string, string[]>>("get_schema", { dbPath });
      const sql = await invoke<string>("query_ollama", {
        model,
        prompt: buildPrompt(schema, testPrompt),
      });
      testResult = sql.trim();
    } catch (e) {
      testError = String(e);
    } finally {
      testing = false;
    }
  }


</script>

<div class="flex flex-col max-w-xl">

  <!-- Flikar -->
  <div class="flex border-b border-zinc-800 mb-6">
    {#each [{ id: "groq", label: "OpenRouter (moln, gratis)" }, { id: "gemini", label: "Gemini (moln)" }, { id: "llama", label: "Llama (lokalt)" }] as tab}
      <button
        onclick={() => activeTab = tab.id}
        class="px-4 py-2 text-sm transition-colors cursor-pointer border-b-2 -mb-px
          {activeTab === tab.id ? 'border-zinc-300 text-zinc-100' : 'border-transparent text-zinc-500 hover:text-zinc-300'}"
      >{tab.label}</button>
    {/each}
  </div>

  {#if activeTab === "llama"}
    <p class="text-sm text-zinc-400 mb-6 leading-relaxed">
      Llama körs helt <strong class="text-zinc-200">lokalt på din dator</strong> via Ollama — inga data skickas till externa servrar.
      Det är <strong class="text-zinc-200">helt gratis</strong> att använda. Kräver att Ollama är installerat och att en modell är nedladdad.
    </p>

    <div class="flex flex-col gap-6">
      <!-- Status -->
      <div>
        <h2 class="text-sm font-medium text-zinc-200 mb-3">Status</h2>
        <div class="flex items-center gap-3">
          <div class="flex items-center gap-2 text-sm">
            {#if ollamaStatus === "running"}
              <span class="text-green-400">●</span><span class="text-zinc-300">Körs på localhost:11434</span>
            {:else if ollamaStatus === "offline"}
              <span class="text-red-400">●</span><span class="text-zinc-300">Ej hittad</span>
            {:else}
              <span class="text-zinc-600">●</span><span class="text-zinc-500">Okänd</span>
            {/if}
          </div>
          <button
            class="px-3 py-1 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-md transition-colors cursor-pointer disabled:opacity-50"
            onclick={checkOllama} disabled={checking}
          >{checking ? "Kontrollerar..." : "Kontrollera"}</button>
          {#if lastCheckFailed}
            <span class="text-xs text-yellow-500">
              Hittades inte —
              {#if currentOs === "linux"}starta med: <code class="bg-zinc-800 px-1 rounded">sudo systemctl start ollama</code>{:else}är Ollama igång?{/if}
            </span>
          {/if}
          {#if ollamaStatus === "running"}
            <button
              class="px-3 py-1 text-xs text-red-500 hover:text-red-400 transition-colors cursor-pointer"
              onclick={() => showUninstall = !showUninstall}
            >Avinstallera Ollama</button>
          {/if}
        </div>

        {#if ollamaStatus === "offline"}
          <div class="mt-3 bg-zinc-900 border border-zinc-800 rounded-lg p-4 text-sm text-zinc-400 flex flex-col gap-3">
            <p>Ollama behöver installeras för att AI-funktionen ska fungera.</p>
            {#if currentOs === "linux" || currentOs === "windows"}
              {#if installing}
                <p class="text-xs text-zinc-300">{installStatus}</p>
              {:else if installStatus && !installError}
                <p class="text-xs text-green-400">{installStatus}</p>
              {:else}
                <button
                  class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer"
                  onclick={installOllama}
                >Installera automatiskt</button>
              {/if}
              {#if installError}
                <p class="text-xs text-red-400">{installError}</p>
                <p class="text-xs text-zinc-600">Manuell installation:</p>
              {/if}
            {/if}
            {#if !installing && (currentOs === "mac" || installError)}
              {#if currentOs !== "mac"}
                <p class="text-xs text-zinc-600">
                  Windows: kör den nedladdade .exe-filen.<br>
                  Linux: <code class="bg-zinc-800 px-1 rounded">curl -fsSL https://ollama.com/install.sh | sh</code>
                </p>
              {:else}
                <button
                  class="w-fit px-3 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-md transition-colors cursor-pointer"
                  onclick={() => openUrl("https://ollama.com/download")}
                >Öppna ollama.com/download</button>
              {/if}
            {/if}
          </div>
        {/if}

        {#if showUninstall}
          <div class="mt-3 bg-zinc-900 border border-red-900/50 rounded-lg p-4 text-sm flex flex-col gap-3">
            <p class="text-zinc-300">Avinstallera Ollama:</p>
            {#if currentOs === "linux"}
              <p class="text-xs text-zinc-500">Kör i terminalen:</p>
              <pre class="bg-zinc-800 rounded px-3 py-2 text-xs text-zinc-300 font-mono whitespace-pre-wrap select-all">sudo systemctl stop ollama
sudo systemctl disable ollama
sudo rm /etc/systemd/system/ollama.service
sudo rm $(which ollama)
sudo rm -rf /usr/share/ollama</pre>
            {:else if currentOs === "windows"}
              <p class="text-xs text-zinc-400">Gå till <strong class="text-zinc-200">Inställningar → Appar → Installerade appar</strong>, sök efter <strong class="text-zinc-200">Ollama</strong> och klicka Avinstallera.</p>
              <p class="text-xs text-zinc-600">Modeller ligger kvar i <code class="bg-zinc-800 px-1 rounded">%USERPROFILE%\.ollama\models</code> och kan tas bort manuellt.</p>
            {:else}
              <p class="text-xs text-zinc-400">Kör <code class="bg-zinc-800 px-1 rounded">ollama</code> och följ avinstallationsguiden för ditt OS.</p>
            {/if}
          </div>
        {/if}
      </div>

      {#if ollamaStatus === "running"}
        <!-- Installerade modeller -->
        <div>
          <h2 class="text-sm font-medium text-zinc-200 mb-3">Installerade modeller</h2>
          {#if loadingModels}
            <p class="text-sm text-zinc-500">Laddar...</p>
          {:else if models.length === 0}
            <p class="text-sm text-zinc-500">Inga modeller installerade.</p>
          {:else}
            <div class="flex flex-col gap-1.5">
              {#each models as m}
                <div class="flex items-center justify-between bg-zinc-900 border rounded-md px-3 py-2 {activeModel === m.name ? 'border-zinc-500' : 'border-zinc-800'}">
                  <div class="flex items-center gap-2">
                    <button
                      class="w-4 h-4 rounded-full border-2 flex items-center justify-center cursor-pointer transition-colors {activeModel === m.name ? 'border-white bg-white' : 'border-zinc-600 hover:border-zinc-400'}"
                      onclick={() => setActiveModel(m.name)}
                    >{#if activeModel === m.name}<span class="w-1.5 h-1.5 rounded-full bg-zinc-900"></span>{/if}</button>
                    <span class="text-sm text-zinc-200 font-mono">{m.name}</span>
                    {#if activeModel === m.name}<span class="text-xs text-zinc-500">aktiv</span>{/if}
                  </div>
                  <div class="flex items-center gap-3">
                    <span class="text-xs text-zinc-600">{formatBytes(m.size)}</span>
                    <button
                      class="text-xs text-zinc-600 hover:text-red-400 transition-colors cursor-pointer disabled:opacity-30"
                      onclick={() => deleteModel(m.name)} disabled={deletingModel === m.name}
                    >{deletingModel === m.name ? "Tar bort..." : "Ta bort"}</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Hämta modell -->
        <div>
          <h2 class="text-sm font-medium text-zinc-200 mb-3">Hämta modell</h2>
          <div class="flex flex-col gap-2">
            {#each PRESET_MODELS as preset}
              <div class="flex items-center justify-between bg-zinc-900 border border-zinc-800 rounded-md px-3 py-2">
                <div>
                  <span class="text-sm text-zinc-200">{preset.label}</span>
                  <span class="text-xs text-zinc-500 ml-2">{preset.size}</span>
                  {#if preset.note}<span class="text-xs text-zinc-600 ml-1">— {preset.note}</span>{/if}
                </div>
                {#if isInstalled(preset.name)}
                  <span class="text-xs text-green-500">Installerad</span>
                {:else if pulling && pullingModel === preset.name}
                  <span class="text-xs text-zinc-400">{pullProgress > 0 ? `${pullProgress}%` : pullStatus}</span>
                {:else}
                  <button
                    class="px-3 py-1 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-md transition-colors cursor-pointer disabled:opacity-50"
                    onclick={() => pullModel(preset.name)} disabled={pulling}
                  >Ladda ner</button>
                {/if}
              </div>
            {/each}
            <div class="flex gap-2 mt-1">
              <input
                bind:value={customModelInput}
                placeholder="Annan modell, t.ex. mistral:7b"
                class="flex-1 bg-zinc-900 border border-zinc-800 rounded-md px-3 py-1.5 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
              />
              <button
                class="px-3 py-1.5 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-md transition-colors cursor-pointer disabled:opacity-50"
                onclick={() => customModelInput && pullModel(customModelInput)} disabled={pulling || !customModelInput}
              >Ladda ner</button>
            </div>
            {#if pulling}
              <div class="mt-1">
                <div class="flex justify-between text-xs text-zinc-400 mb-1">
                  <span>{pullingModel} — {pullStatus}</span>
                  {#if pullProgress > 0}<span>{pullProgress}%</span>{/if}
                </div>
                <div class="h-1 bg-zinc-800 rounded-full overflow-hidden">
                  <div class="h-full bg-white rounded-full transition-all duration-200" style="width: {pullProgress}%"></div>
                </div>
              </div>
            {/if}
          </div>
        </div>

        <!-- Testkörning -->
        {#if models.length > 0}
          <div>
            <h2 class="text-sm font-medium text-zinc-200 mb-3">Testkörning</h2>
            <div class="flex flex-col gap-3">
              <textarea
                bind:value={testPrompt}
                rows="2"
                class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 resize-none"
              ></textarea>
              <button
                class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
                onclick={runTest} disabled={testing || !activeModel}
              >{testing ? "Genererar..." : "Generera SQL"}</button>
              {#if testResult}
                <pre class="bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-xs text-green-300 font-mono whitespace-pre-wrap">{testResult}</pre>
              {/if}
              {#if testError}
                <p class="text-xs text-red-400">{testError}</p>
              {/if}
            </div>
          </div>
        {/if}
      {/if}
    </div>

  {:else if activeTab === "groq"}
    <p class="text-sm text-zinc-400 mb-6 leading-relaxed">
      OpenRouter ger tillgång till många AI-modeller i <strong class="text-zinc-200">molnet</strong> — gratis tier med generösa gränser.
      Skaffa gratis API-nyckel på
      <button
        onclick={() => openUrl("https://openrouter.ai/keys")}
        class="text-zinc-200 underline hover:text-white cursor-pointer transition-colors"
      >openrouter.ai</button>.
    </p>

    <div class="flex flex-col gap-4 max-w-md">
      <div>
        <p class="text-xs text-zinc-500 mb-1">API-nyckel</p>
        <input
          type="password"
          bind:value={groqApiKey}
          placeholder="sk-or-..."
          class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
        />
      </div>
      <div>
        <div class="flex items-center justify-between mb-1">
          <p class="text-xs text-zinc-500">Modell</p>
          <button
            onclick={loadOpenRouterModels}
            disabled={!groqApiKey || openRouterModelsLoading}
            class="text-xs text-zinc-600 hover:text-zinc-300 transition-colors cursor-pointer disabled:opacity-40"
          >{openRouterModelsLoading ? "Hämtar..." : "Hämta modeller"}</button>
        </div>
        {#if openRouterModelsError}
          <p class="text-xs text-red-400 mb-1">{openRouterModelsError}</p>
        {/if}
        {#if openRouterModels.length > 0}
          <select
            bind:value={groqModel}
            class="w-full appearance-none bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-zinc-600 cursor-pointer"
          >
            {#each openRouterModels as m}
              <option value={m.id}>{m.name} ({m.id})</option>
            {/each}
          </select>
        {:else}
          <input
            bind:value={groqModel}
            placeholder="t.ex. meta-llama/llama-3.3-70b-instruct:free"
            class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
          />
        {/if}
      </div>
      <button
        onclick={saveGroqSettings} disabled={groqSaving}
        class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
      >{groqSaving ? "Sparar..." : groqSaved ? "Sparat ✓" : "Spara"}</button>
    </div>

    {#if groqApiKey}
      <div class="mt-6">
        <h2 class="text-sm font-medium text-zinc-200 mb-3">Testkörning</h2>
        <div class="flex flex-col gap-3">
          <textarea
            bind:value={testPrompt}
            rows="2"
            class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 resize-none"
          ></textarea>
          <button
            class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
            onclick={runGroqTest} disabled={groqTesting}
          >{groqTesting ? "Genererar..." : "Generera SQL"}</button>
          {#if groqTestResult}
            <pre class="bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-xs text-green-300 font-mono whitespace-pre-wrap">{groqTestResult}</pre>
          {/if}
          {#if groqTestError}
            <p class="text-xs text-red-400">{groqTestError}</p>
          {/if}
        </div>
      </div>
    {/if}

  {:else if activeTab === "gemini"}
    <p class="text-sm text-zinc-400 mb-6 leading-relaxed">
      Gemini är Googles AI och körs i <strong class="text-zinc-200">molnet</strong> — anrop skickas till Googles servrar.
      Gemini 2.5 Flash ingår i en <strong class="text-zinc-200">gratis tier</strong> (1 500 anrop/dag, 15/minut) — mer än tillräckligt för normal användning.
      Ingen lokal installation krävs. Skaffa gratis API-nyckel på
      <button
        onclick={() => openUrl("https://aistudio.google.com/apikey")}
        class="text-zinc-200 underline hover:text-white cursor-pointer transition-colors"
      >aistudio.google.com/apikey</button>.
    </p>

    <div class="flex flex-col gap-4 max-w-md">
      <div>
        <p class="text-xs text-zinc-500 mb-1">API-nyckel</p>
        <input
          type="password"
          bind:value={geminiApiKey}
          placeholder="AIza..."
          class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
        />
      </div>
      <div>
        <div class="flex items-center justify-between mb-1">
          <p class="text-xs text-zinc-500">Modell</p>
          <button
            onclick={loadGeminiModels}
            disabled={!geminiApiKey || geminiModelsLoading}
            class="text-xs text-zinc-600 hover:text-zinc-300 transition-colors cursor-pointer disabled:opacity-40"
          >{geminiModelsLoading ? "Hämtar..." : "Hämta modeller"}</button>
        </div>
        {#if geminiModelsError}
          <p class="text-xs text-red-400 mb-1">{geminiModelsError}</p>
        {/if}
        {#if geminiModels.length > 0}
          <select
            bind:value={geminiModel}
            class="w-full appearance-none bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-zinc-600 cursor-pointer"
          >
            {#each geminiModels as m}
              <option value={m.name} class="bg-zinc-900 text-zinc-200">{m.display_name} ({m.name})</option>
            {/each}
          </select>
        {:else}
          <input
            bind:value={geminiModel}
            placeholder="t.ex. gemini-2.5-flash"
            class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600"
          />
        {/if}
      </div>
      <button
        onclick={saveGeminiSettings} disabled={geminiSaving}
        class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
      >{geminiSaving ? "Sparar..." : geminiSaved ? "Sparat ✓" : "Spara"}</button>
    </div>

    {#if geminiApiKey}
      <div class="mt-6">
        <h2 class="text-sm font-medium text-zinc-200 mb-3">Testkörning</h2>
        <div class="flex flex-col gap-3">
          <textarea
            bind:value={testPrompt}
            rows="2"
            class="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 resize-none"
          ></textarea>
          <button
            class="w-fit px-3 py-1.5 text-xs bg-white text-zinc-900 font-medium rounded-md hover:bg-zinc-200 transition-colors cursor-pointer disabled:opacity-50"
            onclick={runGeminiTest} disabled={geminiTesting || !geminiApiKey}
          >{geminiTesting ? "Genererar..." : "Generera SQL"}</button>
          {#if geminiTestResult}
            <pre class="bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-xs text-green-300 font-mono whitespace-pre-wrap">{geminiTestResult}</pre>
          {/if}
          {#if geminiTestError}
            <p class="text-xs text-red-400">{geminiTestError}</p>
          {/if}
        </div>
      </div>
    {/if}
  {/if}

</div>
