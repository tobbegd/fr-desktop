<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import L from "leaflet";
  import "leaflet/dist/leaflet.css";

  type Bolag = {
    orgnr: string;
    orgnamn: string;
    lat: number | null;
    lon: number | null;
    postort_lat: number | null;
    postort_lon: number | null;
    postort: string | null;
    gatuadress: string | null;
  };

  type Props = { bolag: Bolag[]; onclose: () => void };
  let { bolag, onclose }: Props = $props();

  let mapEl: HTMLDivElement;
  let map: L.Map | null = null;

  function makePinSvg(count: number, exact: boolean): string {
    const bg = exact ? "#60a5fa" : "#fb923c";
    const label = count > 1 ? String(count) : "";
    return `<svg xmlns="http://www.w3.org/2000/svg" width="32" height="40" viewBox="0 0 32 40">
      <path d="M16 0C7.163 0 0 7.163 0 16c0 10 16 24 16 24S32 26 32 16C32 7.163 24.837 0 16 0z" fill="${bg}" stroke="#fff" stroke-width="1.5"/>
      <circle cx="16" cy="16" r="9" fill="white" fill-opacity="0.25"/>
      <text x="16" y="21" text-anchor="middle" font-size="${count > 99 ? 8 : count > 9 ? 10 : 12}" font-family="sans-serif" font-weight="bold" fill="white">${label}</text>
    </svg>`;
  }

  function makeIcon(count: number, exact: boolean): L.DivIcon {
    return L.divIcon({
      html: makePinSvg(count, exact),
      className: "",
      iconSize: [32, 40],
      iconAnchor: [16, 40],
      popupAnchor: [0, -40],
    });
  }

  function buildPopup(items: Bolag[]): string {
    return items.map(b => {
      const addr = b.gatuadress ? `<div class="text-xs text-gray-400">${b.gatuadress}${b.postort ? ", " + b.postort : ""}</div>` : (b.postort ? `<div class="text-xs text-gray-400">${b.postort}</div>` : "");
      return `<div class="mb-1 last:mb-0"><div class="font-semibold text-sm">${b.orgnamn}</div><div class="text-xs text-gray-500">${b.orgnr}</div>${addr}</div>`;
    }).join('<hr class="my-1 border-gray-200"/>');
  }

  onMount(() => {
    map = L.map(mapEl, { preferCanvas: true });
    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      attribution: "© OpenStreetMap",
      maxZoom: 19,
    }).addTo(map);

    // Exact pins
    const exactPins: Bolag[] = [];
    // Postort groups: key = "lat,lon"
    const postortGroups = new Map<string, Bolag[]>();

    for (const b of bolag) {
      if (b.lat !== null && b.lon !== null) {
        exactPins.push(b);
      } else if (b.postort_lat !== null && b.postort_lon !== null) {
        const key = `${b.postort_lat},${b.postort_lon}`;
        if (!postortGroups.has(key)) postortGroups.set(key, []);
        postortGroups.get(key)!.push(b);
      }
    }

    const bounds: [number, number][] = [];

    for (const b of exactPins) {
      const marker = L.marker([b.lat!, b.lon!], { icon: makeIcon(1, true) });
      marker.bindPopup(buildPopup([b]), { maxWidth: 280 });
      marker.addTo(map!);
      bounds.push([b.lat!, b.lon!]);
    }

    for (const [key, items] of postortGroups) {
      const [lat, lon] = key.split(",").map(Number);
      const marker = L.marker([lat, lon], { icon: makeIcon(items.length, false) });
      marker.bindPopup(buildPopup(items), { maxWidth: 280 });
      marker.addTo(map!);
      bounds.push([lat, lon]);
    }

    if (bounds.length > 0) {
      map!.fitBounds(L.latLngBounds(bounds), { padding: [40, 40], maxZoom: 14 });
    } else {
      map!.setView([62, 15], 5);
    }
  });

  onDestroy(() => { map?.remove(); });

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="fixed inset-0 z-40 bg-black/60" role="button" tabindex="-1" onclick={onclose} onkeydown={() => {}}></div>

<div class="fixed inset-4 z-50 flex flex-col rounded-xl border border-zinc-700 bg-zinc-950 shadow-2xl overflow-hidden">
  <div class="flex items-center justify-between px-5 py-3 border-b border-zinc-800 shrink-0">
    <span class="text-base font-semibold text-zinc-100">{bolag.length} bolag på karta</span>
    <div class="flex items-center gap-4 text-xs text-zinc-500">
      <span><span class="inline-block w-3 h-3 rounded-full bg-blue-400 mr-1"></span>Exakt adress</span>
      <span><span class="inline-block w-3 h-3 rounded-full bg-orange-400 mr-1"></span>Postortspin</span>
    </div>
    <button class="text-zinc-400 hover:text-zinc-100 text-xl leading-none px-1" onclick={onclose}>✕</button>
  </div>
  <div class="flex-1 min-h-0" bind:this={mapEl}></div>
</div>
