import { Store } from "@tauri-apps/plugin-store";

let _store: Store | null = null;

async function getStore(): Promise<Store> {
  if (!_store) _store = await Store.load("prefs.json");
  return _store;
}

export interface Prefs {
  serverUrl: string;
  apiKey: string;
  email: string;
  tier: string;
  launchCount: number;
  dbEtag: string;
  dbSha256: string;
  dbPath: string;
  offlineLogins: number;
}

const defaults: Prefs = {
  serverUrl: "",
  apiKey: "",
  email: "",
  tier: "",
  launchCount: 0,
  dbEtag: "",
  dbSha256: "",
  dbPath: "",
  offlineLogins: 0,
};

export async function loadPrefs(): Promise<Prefs> {
  const store = await getStore();
  const prefs: Prefs = { ...defaults };
  for (const key of Object.keys(defaults) as (keyof Prefs)[]) {
    const val = await store.get<Prefs[typeof key]>(key);
    if (val !== null && val !== undefined) (prefs as any)[key] = val;
  }
  return prefs;
}

export async function savePrefs(prefs: Partial<Prefs>): Promise<void> {
  const store = await getStore();
  for (const [key, val] of Object.entries(prefs)) {
    await store.set(key, val);
  }
  await store.save();
}
