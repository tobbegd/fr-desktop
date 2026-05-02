<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import L from "leaflet";
  import "leaflet/dist/leaflet.css";
  import html2canvas from "html2canvas";
  import { jsPDF } from "jspdf";
  import { invoke } from "@tauri-apps/api/core";

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

  type Waypoint = {
    lat: number;
    lon: number;
    exact: boolean;
    items: Bolag[];
    marker: L.Marker;
  };

  type Props = {
    bolag: Bolag[];
    onclose: () => void;
    searchMode?: boolean;
    dbPath?: string;
    onsearchresult?: (orgnrs: string[]) => void;
  };
  let { bolag, onclose, searchMode = false, dbPath, onsearchresult }: Props = $props();

  let mapEl: HTMLDivElement;
  let map: L.Map | null = null;
  let waypoints: Waypoint[] = [];
  let routeLayer: L.GeoJSON | null = null;

  let routeMode = $state(false);
  let startIdx = $state<number | null>(null);
  let routeLoading = $state(false);
  let routeError = $state("");
  let routeOrder = $state<number[] | null>(null);

  let drawMode = $state(false);
  let drawPoints = $state<L.LatLng[]>([]);
  let drawPolyLayer: L.Polygon | null = null;
  let drawDotLayers: L.CircleMarker[] = [];
  let searchLoading = $state(false);
  let searchError = $state("");
  let searchTruncated = $state(false);

  // --- Icons ---

  function makePinSvg(label: string, bg: string, border: string, textColor = "white"): string {
    const fs = label.length > 2 ? 8 : label.length > 1 ? 10 : 12;
    return `<svg xmlns="http://www.w3.org/2000/svg" width="32" height="40" viewBox="0 0 32 40">
      <path d="M16 0C7.163 0 0 7.163 0 16c0 10 16 24 16 24S32 26 32 16C32 7.163 24.837 0 16 0z" fill="${bg}" stroke="${border}" stroke-width="2"/>
      <circle cx="16" cy="16" r="9" fill="white" fill-opacity="0.2"/>
      <text x="16" y="21" text-anchor="middle" font-size="${fs}" font-family="sans-serif" font-weight="bold" fill="${textColor}">${label}</text>
    </svg>`;
  }

  function makeIcon(count: number, exact: boolean, routeNum?: number, isStart?: boolean): L.DivIcon {
    let svg: string;
    if (isStart) {
      svg = makePinSvg("★", "#22c55e", "#fff");
    } else if (routeNum !== undefined) {
      svg = makePinSvg(String(routeNum), exact ? "#60a5fa" : "#fb923c", "#fff");
    } else {
      const label = count > 1 ? String(count) : "";
      svg = makePinSvg(label, exact ? "#60a5fa" : "#fb923c", "#fff");
    }
    return L.divIcon({ html: svg, className: "", iconSize: [32, 40], iconAnchor: [16, 40], popupAnchor: [0, -40] });
  }

  function buildPopup(items: Bolag[], routeNum?: number, isStart?: boolean): string {
    const badge = isStart
      ? `<div class="text-xs font-bold text-green-600 mb-1">★ Start</div>`
      : routeNum !== undefined
        ? `<div class="text-xs font-bold text-blue-500 mb-1">Stop ${routeNum}</div>`
        : "";
    return badge + items.map(b => {
      const addr = b.gatuadress
        ? `<div class="text-xs text-gray-400">${b.gatuadress}${b.postort ? ", " + b.postort : ""}</div>`
        : b.postort ? `<div class="text-xs text-gray-400">${b.postort}</div>` : "";
      return `<div class="mb-1 last:mb-0"><div class="font-semibold text-sm">${b.orgnamn}</div><div class="text-xs text-gray-500">${b.orgnr}</div>${addr}</div>`;
    }).join('<hr class="my-1 border-gray-200"/>');
  }

  // --- Route logic ---

  function refreshMarkerIcons() {
    waypoints.forEach((wp, idx) => {
      const isStart = startIdx === idx;
      let routeNum: number | undefined;
      if (routeOrder) {
        const pos = routeOrder.indexOf(idx);
        routeNum = pos >= 0 ? pos + 1 : undefined;
      }
      wp.marker.setIcon(makeIcon(wp.items.length, wp.exact, routeNum, isStart));
      wp.marker.setPopupContent(buildPopup(wp.items, routeNum, isStart));
    });
  }

  function toggleRouteMode() {
    routeMode = !routeMode;
    if (!routeMode) {
      startIdx = null;
      routeOrder = null;
      routeError = "";
      clearRoute();
      refreshMarkerIcons();
    }
  }

  function clearRoute() {
    if (routeLayer && map) { map.removeLayer(routeLayer); routeLayer = null; }
  }

  async function calcRoute() {
    if (startIdx === null || waypoints.length < 2) return;
    routeLoading = true;
    routeError = "";
    clearRoute();

    // Reorder: start first
    const ordered = [startIdx, ...waypoints.map((_, i) => i).filter(i => i !== startIdx)];
    const coords = ordered.map(i => `${waypoints[i].lon},${waypoints[i].lat}`).join(";");

    try {
      const res = await fetch(
        `https://router.project-osrm.org/trip/v1/driving/${coords}?roundtrip=false&source=first&overview=full&geometries=geojson`,
        { signal: AbortSignal.timeout(15000) }
      );
      if (!res.ok) throw new Error(`OSRM ${res.status}`);
      const data = await res.json();
      if (data.code !== "Ok") throw new Error(data.message ?? data.code);

      // Build route order from waypoints_index in trips
      const trip = data.trips[0];
      const waypointOrder: number[] = data.waypoints
        .slice()
        .sort((a: { waypoint_index: number }, b: { waypoint_index: number }) => a.waypoint_index - b.waypoint_index)
        .map((w: { trips_index: number; waypoint_index: number }, i: number) => ordered[i]);

      routeOrder = waypointOrder;
      refreshMarkerIcons();

      const canvasRenderer = L.canvas();
      routeLayer = L.geoJSON(trip.geometry, ({
        style: { color: "#22c55e", weight: 3, opacity: 0.85 },
        renderer: canvasRenderer,
      }) as unknown as L.GeoJSONOptions).addTo(map!);

      map!.fitBounds(routeLayer.getBounds(), { padding: [40, 40] });
    } catch (e) {
      routeError = String(e);
    } finally {
      routeLoading = false;
    }
  }

  // --- Draw / polygon search ---

  function clearDrawLayers() {
    drawDotLayers.forEach(m => map?.removeLayer(m));
    drawDotLayers = [];
    if (drawPolyLayer) { map?.removeLayer(drawPolyLayer); drawPolyLayer = null; }
  }

  function updateDrawLayers() {
    clearDrawLayers();
    if (drawPoints.length === 0) return;
    if (drawPoints.length >= 2) {
      drawPolyLayer = L.polygon(drawPoints, {
        color: "#f97316", fillColor: "#f97316", fillOpacity: 0.08,
        weight: 2, dashArray: "6 4",
        interactive: false,
      }).addTo(map!);
    }
    drawPoints.forEach((pt) => {
      const m = L.circleMarker(pt, {
        radius: 5, color: "#f97316", fillColor: "#fff",
        fillOpacity: 1, weight: 2, interactive: false,
      }).addTo(map!);
      drawDotLayers.push(m);
    });
  }

  function startDraw() {
    if (routeMode) return;
    drawMode = true;
    drawPoints = [];
    clearDrawLayers();
    searchError = "";
    searchTruncated = false;
    map!.getContainer().style.cursor = "crosshair";
  }

  function finishDraw() {
    if (drawPoints.length < 3) return;
    drawMode = false;
    map!.getContainer().style.cursor = "";
    updateDrawLayers();
  }

  function cancelDraw() {
    drawMode = false;
    drawPoints = [];
    clearDrawLayers();
    searchError = "";
    searchTruncated = false;
    map!.getContainer().style.cursor = "";
  }

  function pointInPolygon(lat: number, lon: number, poly: L.LatLng[]): boolean {
    let inside = false;
    const n = poly.length;
    let j = n - 1;
    for (let i = 0; i < n; i++) {
      const xi = poly[i].lng, yi = poly[i].lat;
      const xj = poly[j].lng, yj = poly[j].lat;
      if ((yi > lat) !== (yj > lat) && lon < ((xj - xi) * (lat - yi)) / (yj - yi) + xi) {
        inside = !inside;
      }
      j = i;
    }
    return inside;
  }

  async function searchWithinPolygon() {
    if (drawPoints.length < 3 || !dbPath) return;
    searchLoading = true;
    searchError = "";
    searchTruncated = false;
    if (drawMode) finishDraw();

    const lats = drawPoints.map(p => p.lat);
    const lons = drawPoints.map(p => p.lng);
    const minLat = Math.min(...lats), maxLat = Math.max(...lats);
    const minLon = Math.min(...lons), maxLon = Math.max(...lons);

    const sql = `SELECT orgnr, lat, lon, postort_lat, postort_lon FROM bolag WHERE (lat BETWEEN ${minLat} AND ${maxLat} AND lon BETWEEN ${minLon} AND ${maxLon}) OR (postort_lat BETWEEN ${minLat} AND ${maxLat} AND postort_lon BETWEEN ${minLon} AND ${maxLon})`;

    try {
      const res = await invoke<{ columns: string[]; rows: unknown[][]; truncated: boolean }>("query_db", { dbPath, sql });
      searchTruncated = res.truncated;
      const ci = (col: string) => res.columns.indexOf(col);
      const orgnrs = res.rows
        .filter(row => {
          const lat = row[ci("lat")] as number | null;
          const lon = row[ci("lon")] as number | null;
          const plat = row[ci("postort_lat")] as number | null;
          const plon = row[ci("postort_lon")] as number | null;
          if (lat !== null && lon !== null) return pointInPolygon(lat, lon, drawPoints);
          if (plat !== null && plon !== null) return pointInPolygon(plat, plon, drawPoints);
          return false;
        })
        .map(row => String(row[ci("orgnr")]));

      if (orgnrs.length === 0) {
        searchError = "Inga bolag hittades inom markeringen.";
        return;
      }
      onsearchresult?.(orgnrs);
    } catch (e) {
      searchError = String(e);
    } finally {
      searchLoading = false;
    }
  }

  // --- Mount ---

  onMount(() => {
    map = L.map(mapEl, { preferCanvas: true });
    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      attribution: "© OpenStreetMap",
      maxZoom: 19,
    }).addTo(map);

    map.on("click", (e: L.LeafletMouseEvent) => {
      if (!drawMode) return;
      drawPoints = [...drawPoints, e.latlng];
      updateDrawLayers();
    });

    map.on("dblclick", (e: L.LeafletMouseEvent) => {
      if (!drawMode) return;
      L.DomEvent.stopPropagation(e);
      // dblclick fires two click events first — remove the extra point
      drawPoints = drawPoints.slice(0, -1);
      if (drawPoints.length >= 3) finishDraw();
    });

    const postortGroups = new Map<string, Bolag[]>();
    const bounds: [number, number][] = [];

    for (const b of bolag) {
      if (b.lat !== null && b.lon !== null) {
        const wp: Waypoint = {
          lat: b.lat, lon: b.lon, exact: true, items: [b],
          marker: null as unknown as L.Marker,
        };
        const idx = waypoints.length;
        waypoints.push(wp);
        const marker = L.marker([b.lat, b.lon], { icon: makeIcon(1, true) });
        marker.bindPopup(buildPopup([b]), { maxWidth: 280 });
        marker.on("click", () => {
          if (routeMode) { startIdx = idx; refreshMarkerIcons(); }
        });
        marker.addTo(map!);
        wp.marker = marker;
        bounds.push([b.lat, b.lon]);
      } else if (b.postort_lat !== null && b.postort_lon !== null) {
        const key = `${b.postort_lat},${b.postort_lon}`;
        if (!postortGroups.has(key)) postortGroups.set(key, []);
        postortGroups.get(key)!.push(b);
      }
    }

    for (const [key, items] of postortGroups) {
      const [lat, lon] = key.split(",").map(Number);
      const wp: Waypoint = {
        lat, lon, exact: false, items,
        marker: null as unknown as L.Marker,
      };
      const idx = waypoints.length;
      waypoints.push(wp);
      const marker = L.marker([lat, lon], { icon: makeIcon(items.length, false) });
      marker.bindPopup(buildPopup(items), { maxWidth: 280 });
      marker.on("click", () => {
        if (routeMode) { startIdx = idx; refreshMarkerIcons(); }
      });
      marker.addTo(map!);
      wp.marker = marker;
      bounds.push([lat, lon]);
    }

    if (bounds.length > 0) {
      map!.fitBounds(L.latLngBounds(bounds), { padding: [40, 40], maxZoom: 14 });
    } else {
      map!.setView([62, 15], 5);
    }
  });

  let pdfLoading = $state(false);

  async function exportPdf() {
    if (!mapEl) return;
    pdfLoading = true;
    try {
      // Close all popups and reset Leaflet pan offset before capture
      map?.closePopup();
      map?.setView(map.getCenter(), map.getZoom(), { reset: true } as L.ZoomPanOptions);
      await new Promise(r => setTimeout(r, 300));

      const canvas = await html2canvas(mapEl, { useCORS: true, scale: 2, logging: false });
      const imgData = canvas.toDataURL("image/jpeg", 0.92);

      const pdf = new jsPDF({ orientation: "landscape", unit: "mm", format: "a4" });
      const pageW = pdf.internal.pageSize.getWidth();
      const pageH = pdf.internal.pageSize.getHeight();
      const margin = 10;
      const maxW = pageW - margin * 2;
      const maxH = pageH - margin * 2;
      const ratio = canvas.width / canvas.height;
      let mapW = maxW;
      let mapH = mapW / ratio;
      if (mapH > maxH) { mapH = maxH; mapW = mapH * ratio; }
      const offsetX = margin + (maxW - mapW) / 2;

      pdf.addImage(imgData, "JPEG", offsetX, margin, mapW, mapH);

      // Stop list on page 2 if route calculated
      if (routeOrder && routeOrder.length > 0) {
        pdf.addPage();
        let y = margin + 6;
        pdf.setFontSize(10);
        pdf.setTextColor(60, 60, 60);
        pdf.text("Rutt i körordning:", margin, y);
        y += 7;
        pdf.setFontSize(8);
        pdf.setTextColor(40, 40, 40);
        const colW = (pageW - margin * 2) / 2;
        routeOrder.forEach((wpIdx, pos) => {
          const wp = waypoints[wpIdx];
          const name = wp.items.map(b => b.orgnamn).join(", ");
          const addr = wp.items[0]?.gatuadress ?? wp.items[0]?.postort ?? "";
          const col = pos % 2 === 0 ? 0 : 1;
          const row = Math.floor(pos / 2);
          const x = margin + col * colW;
          const rowY = y + row * 5.5;
          if (rowY < pageH - margin) {
            pdf.text(`${pos + 1}. ${name}${addr ? " — " + addr : ""}`, x, rowY, { maxWidth: colW - 4 });
          }
        });
      }

      const b64 = pdf.output("datauristring").split(",")[1];
      await invoke("save_file_binary", { filename: "karta-rutt.pdf", data: b64 });
    } catch (e) {
      console.error(e);
    } finally {
      pdfLoading = false;
    }
  }

  onDestroy(() => { map?.remove(); });

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="fixed inset-0 z-40 bg-black/60" role="button" tabindex="-1" onclick={onclose} onkeydown={() => {}}></div>

<div class="fixed inset-4 z-50 flex flex-col rounded-xl border border-zinc-700 bg-zinc-950 shadow-2xl overflow-hidden">
  <div class="flex items-center gap-4 px-5 py-3 border-b border-zinc-800 shrink-0">
    {#if searchMode}
      <span class="text-base font-semibold text-zinc-100">
        {drawMode
          ? (drawPoints.length === 0
              ? "Klicka på kartan för att rita"
              : `Ritar — ${drawPoints.length} punkt${drawPoints.length !== 1 ? "er" : ""}`)
          : drawPoints.length >= 3
            ? `Markering klar — ${drawPoints.length} punkter`
            : "Rita en markering på kartan"}
      </span>
    {:else}
      <span class="text-base font-semibold text-zinc-100">{bolag.length} bolag på karta</span>
      <div class="flex items-center gap-4 text-xs text-zinc-500">
        <span><span class="inline-block w-3 h-3 rounded-full bg-blue-400 mr-1"></span>Exakt adress</span>
        <span><span class="inline-block w-3 h-3 rounded-full bg-orange-400 mr-1"></span>Postortspin</span>
      </div>
    {/if}
    <div class="flex-1"></div>

    {#if drawMode}
      {#if drawPoints.length >= 3}
        <span class="text-xs text-zinc-400">Fortsätt klicka för fler punkter · dubbelklicka för att avsluta</span>
        <button
          class="px-3 py-1.5 text-xs rounded-md bg-orange-700 hover:bg-orange-600 text-white font-medium transition-colors cursor-pointer disabled:opacity-40"
          disabled={searchLoading}
          onclick={searchWithinPolygon}
        >{searchLoading ? "Söker..." : "Hitta bolag inom markering"}</button>
      {:else}
        <span class="text-xs text-zinc-500">Rita minst 3 punkter</span>
      {/if}
      <button
        class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
        onclick={cancelDraw}
      >Avbryt</button>
    {:else if drawPoints.length >= 3}
      <button
        class="px-3 py-1.5 text-xs rounded-md bg-orange-700 hover:bg-orange-600 text-white font-medium transition-colors cursor-pointer disabled:opacity-40"
        disabled={searchLoading}
        onclick={searchWithinPolygon}
      >{searchLoading ? "Söker..." : "Hitta bolag inom markering"}</button>
      <button
        class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
        onclick={cancelDraw}
      >Rensa markering</button>
    {:else if routeMode}
      <span class="text-xs text-zinc-400">
        {#if startIdx === null}Klicka på en pin för att sätta startpunkt{:else}Startpunkt vald — {waypoints.length} stopp{/if}
      </span>
      <button
        class="px-3 py-1.5 text-xs rounded-md font-medium transition-colors cursor-pointer
          {startIdx !== null && !routeLoading ? 'bg-green-700 hover:bg-green-600 text-white' : 'bg-zinc-800 text-zinc-500 cursor-default'}"
        disabled={startIdx === null || routeLoading}
        onclick={calcRoute}
      >{routeLoading ? "Beräknar..." : "Beräkna rutt"}</button>
      <button
        class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
        onclick={toggleRouteMode}
      >Avbryt</button>
    {:else}
      <button
        class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
        onclick={startDraw}
      >Rita markering</button>
      {#if !searchMode}
        <button
          class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
          onclick={toggleRouteMode}
        >Planera rutt</button>
      {/if}
    {/if}

    {#if !searchMode}
      <button
        class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer disabled:opacity-40"
        disabled={pdfLoading}
        onclick={exportPdf}
      >{pdfLoading ? "Exporterar..." : "Exportera PDF"}</button>
    {/if}
    <button class="text-zinc-400 hover:text-zinc-100 text-xl leading-none px-1" onclick={onclose}>✕</button>
  </div>

  {#if routeError}
    <div class="shrink-0 px-5 py-2 text-xs text-red-400 border-b border-zinc-800">{routeError}</div>
  {/if}
  {#if searchError}
    <div class="shrink-0 px-5 py-2 text-xs text-red-400 border-b border-zinc-800">{searchError}</div>
  {/if}
  {#if searchTruncated}
    <div class="shrink-0 px-5 py-2 text-xs text-yellow-600 border-b border-zinc-800">Varning: området är för stort — resultatet är avkortat. Rita en mindre markering.</div>
  {/if}

  <div class="flex-1 min-h-0" bind:this={mapEl}></div>
</div>
