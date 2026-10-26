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

  type ViaPoint = {
    lat: number;
    lon: number;
    marker: L.CircleMarker;
  };

  type CustomStart = {
    lat: number;
    lon: number;
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
  type RouteStep = { label: string; wpIdx: number | null; isVia: boolean; stopNum?: number };
  let routeSteps = $state<RouteStep[] | null>(null);
  let routeLegs = $state<{ duration: number; distance: number }[]>([]);
  let routeTotal = $state<{ duration: number; distance: number } | null>(null);
  let viaMode = $state(false);
  let viaPoints = $state<ViaPoint[]>([]);
  let customStart = $state<CustomStart | null>(null);

  function formatDuration(seconds: number): string {
    const totalMin = Math.round(seconds / 60);
    const h = Math.floor(totalMin / 60);
    const m = totalMin % 60;
    if (h === 0) return `${m} min`;
    return `${h} tim ${m} min`;
  }

  function formatDistance(meters: number): string {
    const km = meters / 1000;
    return km >= 10 ? `${Math.round(km)} km` : `${km.toFixed(1)} km`;
  }

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
    if (routeMode) {
      if (map) map.getContainer().style.cursor = "crosshair";
    } else {
      startIdx = null;
      routeOrder = null;
      routeSteps = null;
      routeLegs = [];
      routeTotal = null;
      routeError = "";
      viaMode = false;
      clearViaPoints();
      clearCustomStart();
      clearRoute();
      if (map) map.getContainer().style.cursor = "";
      refreshMarkerIcons();
    }
  }

  function clearRoute() {
    if (routeLayer && map) { map.removeLayer(routeLayer); routeLayer = null; }
  }

  function clearViaPoints() {
    viaPoints.forEach(v => map?.removeLayer(v.marker));
    viaPoints = [];
  }

  function clearCustomStart() {
    if (customStart) map?.removeLayer(customStart.marker);
    customStart = null;
  }

  function setCustomStart(latlng: L.LatLng) {
    startIdx = null;
    const marker = L.marker(latlng, { icon: makeIcon(1, true, undefined, true) });
    marker.bindPopup("Startpunkt", { maxWidth: 200 });
    marker.addTo(map!);
    customStart = { lat: latlng.lat, lon: latlng.lng, marker };
    if (map) map.getContainer().style.cursor = "";
    refreshMarkerIcons();
  }

  function toggleViaMode() {
    viaMode = !viaMode;
    if (map) map.getContainer().style.cursor = viaMode ? "crosshair" : "";
  }

  function addViaPoint(latlng: L.LatLng) {
    const marker = L.circleMarker(latlng, {
      radius: 7, color: "#fff", fillColor: "#a855f7",
      fillOpacity: 1, weight: 2,
    }).bindTooltip("Tvingande punkt", { permanent: false });
    marker.addTo(map!);
    viaPoints = [...viaPoints, { lat: latlng.lat, lon: latlng.lng, marker }];
  }

  function removeViaPoint(idx: number) {
    const v = viaPoints[idx];
    if (v) map?.removeLayer(v.marker);
    viaPoints = viaPoints.filter((_, i) => i !== idx);
  }

  function haversine(lat1: number, lon1: number, lat2: number, lon2: number): number {
    const R = 6371000;
    const toRad = (d: number) => (d * Math.PI) / 180;
    const dLat = toRad(lat2 - lat1);
    const dLon = toRad(lon2 - lon1);
    const a = Math.sin(dLat / 2) ** 2 + Math.cos(toRad(lat1)) * Math.cos(toRad(lat2)) * Math.sin(dLon / 2) ** 2;
    return 2 * R * Math.asin(Math.sqrt(a));
  }

  async function calcRoute() {
    const hasStart = startIdx !== null || customStart !== null;
    const minStops = customStart ? 1 : 2;
    if (!hasStart || waypoints.length < minStops) return;
    routeLoading = true;
    routeError = "";
    clearRoute();

    // Build the ordered coordinate list sent to OSRM: position 0 is always
    // the start (either a real bolag waypoint, or the free-standing custom
    // start point), followed by all remaining bolag waypoints. We track the
    // original `waypoints` array index for each entry so we can map OSRM's
    // visit order back correctly (null = the custom start, not a real stop).
    type TripPoint = { lat: number; lon: number; wpIndex: number | null };
    const points: TripPoint[] = [];
    if (customStart) {
      points.push({ lat: customStart.lat, lon: customStart.lon, wpIndex: null });
      waypoints.forEach((wp, i) => points.push({ lat: wp.lat, lon: wp.lon, wpIndex: i }));
    } else {
      points.push({ lat: waypoints[startIdx!].lat, lon: waypoints[startIdx!].lon, wpIndex: startIdx });
      waypoints.forEach((wp, i) => { if (i !== startIdx) points.push({ lat: wp.lat, lon: wp.lon, wpIndex: i }); });
    }
    const coords = points.map(p => `${p.lon},${p.lat}`).join(";");

    try {
      const res = await fetch(
        `https://router.project-osrm.org/trip/v1/driving/${coords}?roundtrip=false&source=first&overview=full&geometries=geojson`,
        { signal: AbortSignal.timeout(15000) }
      );
      if (!res.ok) throw new Error(`OSRM ${res.status}`);
      const data = await res.json();
      if (data.code !== "Ok") throw new Error(data.message ?? data.code);

      // data.waypoints is in INPUT order; each entry's waypoint_index is its
      // position in the visit order. Place each input point at that visit
      // position to get the trip sequence (wpIndex, null for custom start).
      const trip = data.trips[0];
      const visitSeq: (number | null)[] = new Array(points.length);
      data.waypoints.forEach((w: { waypoint_index: number }, inputIdx: number) => {
        visitSeq[w.waypoint_index] = points[inputIdx].wpIndex;
      });

      // routeOrder holds only the real bolag stops, in visit order (used by
      // the marker numbering and the "N. bolagsnamn" icons on the map).
      routeOrder = visitSeq.filter((v): v is number => v !== null);

      // Build the full step list (start + bolag stops, via-points inserted
      // at their best position) that both the geometry fetch and the UI
      // list / PDF export walk together, so every point — including
      // via-points — gets its own labeled leg and nothing is silently
      // folded into a neighboring leg's duration.
      type Step = { lat: number; lon: number; label: string; wpIdx: number | null; isVia: boolean };
      const steps: Step[] = visitSeq.map(v =>
        v === null
          ? { lat: customStart!.lat, lon: customStart!.lon, label: "Fri startpunkt", wpIdx: null, isVia: false }
          : { lat: waypoints[v].lat, lon: waypoints[v].lon, label: waypoints[v].items.map(b => b.orgnamn).join(", "), wpIdx: v, isVia: false }
      );

      for (const [viaIdx, via] of viaPoints.entries()) {
        let bestLeg = 0;
        let bestCost = Infinity;
        for (let i = 0; i < steps.length - 1; i++) {
          const a = steps[i], b = steps[i + 1];
          const cost = haversine(a.lat, a.lon, via.lat, via.lon) + haversine(via.lat, via.lon, b.lat, b.lon) - haversine(a.lat, a.lon, b.lat, b.lon);
          if (cost < bestCost) { bestCost = cost; bestLeg = i; }
        }
        // Insert right after position bestLeg, before the next stop — if
        // multiple vias land in the same leg this re-scans against the
        // already-inserted ones too, which is fine for the small N here.
        steps.splice(bestLeg + 1, 0, { lat: via.lat, lon: via.lon, label: `Tillagd punkt ${viaIdx + 1}`, wpIdx: null, isVia: true });
      }

      let legs: { duration: number; distance: number }[];
      let geometry: unknown;

      if (viaPoints.length === 0) {
        legs = trip.legs.map((l: { duration: number; distance: number }) => ({ duration: l.duration, distance: l.distance }));
        geometry = trip.geometry;
      } else {
        // Re-fetch via /route/ with every point as its own waypoint (no
        // `waypoints=` filter) so OSRM returns one leg per consecutive pair
        // — including the legs on either side of a via-point — instead of
        // folding a via-point's detour into one existing bolag-to-bolag leg.
        const fullCoords = steps.map(s => `${s.lon},${s.lat}`).join(";");
        const routeRes = await fetch(
          `https://router.project-osrm.org/route/v1/driving/${fullCoords}?overview=full&geometries=geojson`,
          { signal: AbortSignal.timeout(15000) }
        );
        if (!routeRes.ok) throw new Error(`OSRM ${routeRes.status}`);
        const routeData = await routeRes.json();
        if (routeData.code !== "Ok") throw new Error(routeData.message ?? routeData.code);
        const route = routeData.routes[0];
        legs = route.legs.map((l: { duration: number; distance: number }) => ({ duration: l.duration, distance: l.distance }));
        geometry = route.geometry;
      }

      routeSteps = steps.map((s, i) => ({
        label: s.label,
        wpIdx: s.wpIdx,
        isVia: s.isVia,
        stopNum: s.wpIdx !== null ? routeOrder!.indexOf(s.wpIdx) + 1 : undefined,
      }));
      routeLegs = legs;
      routeTotal = legs.reduce(
        (acc, l) => ({ duration: acc.duration + l.duration, distance: acc.distance + l.distance }),
        { duration: 0, distance: 0 }
      );

      const canvasRenderer = L.canvas();
      routeLayer = L.geoJSON(geometry as GeoJSON.GeoJsonObject, ({
        style: { color: "#22c55e", weight: 3, opacity: 0.85 },
        renderer: canvasRenderer,
      }) as unknown as L.GeoJSONOptions).addTo(map!);
      map!.fitBounds(routeLayer.getBounds(), { padding: [40, 40] });

      refreshMarkerIcons();
    } catch (e) {
      routeError = String(e);
      routeSteps = null;
      routeLegs = [];
      routeTotal = null;
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
      if (routeMode && startIdx === null && customStart === null) { setCustomStart(e.latlng); return; }
      if (viaMode) { addViaPoint(e.latlng); return; }
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
          if (routeMode && startIdx === null && customStart === null) { startIdx = idx; refreshMarkerIcons(); }
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
        if (routeMode) { clearCustomStart(); startIdx = idx; refreshMarkerIcons(); }
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
      if (routeSteps && routeSteps.length > 0) {
        pdf.addPage();
        let y = margin + 6;
        pdf.setFontSize(10);
        pdf.setTextColor(60, 60, 60);
        const totalLabel = routeTotal
          ? `Rutt i körordning — totalt ${formatDuration(routeTotal.duration)} (${formatDistance(routeTotal.distance)}):`
          : "Rutt i körordning:";
        pdf.text(totalLabel, margin, y);
        y += 7;
        pdf.setFontSize(8);
        pdf.setTextColor(40, 40, 40);
        const colW = (pageW - margin * 2) / 2;
        routeSteps.forEach((step, pos) => {
          const wp = step.wpIdx !== null ? waypoints[step.wpIdx] : null;
          const addr = wp ? (wp.items[0]?.gatuadress ?? wp.items[0]?.postort ?? "") : "";
          const leg = pos < routeLegs.length ? routeLegs[pos] : null;
          const legLabel = leg ? ` (${formatDuration(leg.duration)} till nästa)` : "";
          const prefix = step.isVia ? "" : `${step.stopNum}. `;
          const col = pos % 2 === 0 ? 0 : 1;
          const rowIdx = Math.floor(pos / 2);
          const x = margin + col * colW;
          const rowY = y + rowIdx * 5.5;
          if (rowY < pageH - margin) {
            pdf.text(`${prefix}${step.label}${addr ? " — " + addr : ""}${legLabel}`, x, rowY, { maxWidth: colW - 4 });
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
      {#if startIdx === null && !customStart}
        <span class="text-xs text-zinc-400">Klicka på ett bolag för att starta där, eller valfri plats på kartan för en fri startpunkt</span>
      {:else}
        <span class="text-xs text-zinc-400">
          {#if customStart}Startpunkt: fri plats{:else}Startpunkt: bolag{/if} — {waypoints.length} stopp
          {#if viaPoints.length > 0}· {viaPoints.length} tillagd{viaPoints.length !== 1 ? "a" : ""} punkt{viaPoints.length !== 1 ? "er" : ""}{/if}
        </span>
        <button
          class="px-3 py-1.5 text-xs rounded-md font-medium transition-colors cursor-pointer
            {viaMode ? 'bg-purple-700 hover:bg-purple-600 text-white' : 'bg-zinc-800 hover:bg-zinc-700 text-zinc-300'}"
          onclick={toggleViaMode}
        >{viaMode ? "Klicka på kartan..." : "Lägg till punkt"}</button>
        {#if viaPoints.length > 0}
          <button
            class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
            onclick={clearViaPoints}
          >Rensa punkter ({viaPoints.length})</button>
        {/if}
        <button
          class="px-3 py-1.5 text-xs rounded-md font-medium transition-colors cursor-pointer
            {!routeLoading ? 'bg-green-700 hover:bg-green-600 text-white' : 'bg-zinc-800 text-zinc-500 cursor-default'}"
          disabled={routeLoading}
          onclick={calcRoute}
        >{routeLoading ? "Beräknar..." : routeOrder ? "Beräkna om rutt" : "Beräkna rutt"}</button>
      {/if}
      <button
        class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
        onclick={toggleRouteMode}
      >Avbryt</button>
    {:else}
      {#if searchMode}
        <button
          class="px-3 py-1.5 text-xs rounded-md bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors cursor-pointer"
          onclick={startDraw}
        >Rita markering</button>
      {:else}
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

  <div class="relative flex-1 min-h-0">
    {#if routeSteps && routeTotal}
      <div class="absolute left-3 top-3 bottom-3 z-[1000] w-64 flex flex-col rounded-lg border border-zinc-700 bg-zinc-950/95 shadow-xl overflow-hidden">
        <div class="shrink-0 px-3 py-2.5 border-b border-zinc-800">
          <span class="text-xs font-semibold text-zinc-200">
            Total restid: {formatDuration(routeTotal.duration)}
          </span>
          <div class="text-[11px] text-zinc-500">{formatDistance(routeTotal.distance)}</div>
        </div>
        <div class="flex-1 overflow-y-auto px-3 py-2.5">
          {#each routeSteps as step, pos (pos)}
            <div class="text-xs leading-snug {step.isVia ? 'text-purple-300' : 'text-zinc-200'}">
              {step.isVia ? "• " : step.wpIdx === null ? "★ " : `${step.stopNum}. `}{step.label}
            </div>
            {#if pos < routeLegs.length}
              <div class="pl-3 py-1 text-[11px] text-zinc-500 border-l border-zinc-700 ml-1.5">
                ↓ {formatDuration(routeLegs[pos].duration)} ({formatDistance(routeLegs[pos].distance)})
              </div>
            {/if}
          {/each}
        </div>
      </div>
    {/if}

    <div class="absolute inset-0" bind:this={mapEl}></div>
  </div>
</div>
