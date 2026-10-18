<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { gpsStore } from "$lib/stores/gpsStore";
	import { type Map as MapLibreMap, type Marker, type LngLatLike, type StyleSpecification } from "maplibre-gl";
	import type { GpsData } from "$lib/stores/gpsStore";
	import ServerStatus from "./ServerStatus.svelte";
	import ServerAliveIndicator from "./ServerAliveIndicator.svelte";

	import "maplibre-gl/dist/maplibre-gl.css";
	import "$lib/styles/global.css";
	import mapStyleData from "$lib/styles/map-style.json";

	const mapStyle: StyleSpecification = mapStyleData as unknown as StyleSpecification;

	let mapContainer = $state<HTMLDivElement | null>(null);
	let map: MapLibreMap | null = null;
	let marker: Marker | null = null;
	let mapReady = $state(false);
	let mapError: string | null = $state(null);

	const TILE_SERVER_URL = "http://127.0.0.1:8765";
	const defaultCenter: LngLatLike = [-77.0365, 38.8977];
	let lastFlyTo = 0;
	const FLY_TO_THROTTLE_MS = 5000;

	function watchGpsPosition(): void {
		const currentGps = gpsStore.gpsData;
		if (!currentGps || !currentGps.valid || !mapReady || !marker || !map) return;

		const now = Date.now();
		if (now - lastFlyTo < FLY_TO_THROTTLE_MS) {
			marker.setLngLat([currentGps.longitude, currentGps.latitude]);
			return;
		}

		lastFlyTo = now;
		const newPosition: LngLatLike = [currentGps.longitude, currentGps.latitude];
		marker.setLngLat(newPosition);
		map.flyTo({
			center: newPosition,
			essential: true,
			animate: true,
			duration: 1000,
		});
	}

	$effect(() => {
		const gps = gpsStore.gpsData;
		if (gps?.valid && mapReady && marker && map) {
			watchGpsPosition();
		}
	});

	async function initMap(): Promise<void> {
		if (!mapContainer) return;

		try {
			const { Map, Marker } = await import("maplibre-gl");

			map = new Map({
				container: mapContainer,
				style: {
					...mapStyle,
					glyphs: `${TILE_SERVER_URL}/glyphs/{fontstack}/{range}.pbf`,
					sources: {
						...mapStyle.sources,
					},
				},
				center: defaultCenter,
				zoom: 15,
				pitch: 0,
			});

			map.on("load", () => {
				mapReady = true;

				const markerEl = document.createElement("div");
				markerEl.className = "gps-marker";

				marker = new Marker(markerEl)
					.setLngLat(gpsStore.gpsData ? [gpsStore.gpsData.longitude, gpsStore.gpsData.latitude] : defaultCenter)
					.addTo(map!);

				watchGpsPosition();
			});

			map.on("error", (e) => {
				console.error("MapLibre error:", e);
				mapError = e.error?.message || "Error en el mapa";
			});
		} catch (e) {
			console.error("Failed to load maplibre-gl:", e);
			mapError = e instanceof Error ? e.message : "No se pudo cargar el mapa";
		}
	}

	onMount(() => {
		initMap();
		gpsStore.startPolling();
	});

	onDestroy(() => {
		gpsStore.stopPolling();
		marker?.remove();
		map?.remove();
		map = null;
		marker = null;
	});
</script>

<div class="map-page">
	<div class="map-header">
		<button class="back-button" onclick={() => gpsStore.navigateTo("/")}>← Volver</button>
		<div class="gps-status">
			{#if gpsStore.isLoading}
				<span class="status-indicator status-loading">Conectando GPS...</span>
			{:else if gpsStore.error}
				<span class="status-indicator status-error">Error: {gpsStore.error}</span>
			{:else if gpsStore.gpsData?.valid}
				<span class="status-indicator status-valid">
					Lat: {gpsStore.gpsData.latitude.toFixed(6)} |
					Lng: {gpsStore.gpsData.longitude.toFixed(6)} |
					Vel: {gpsStore.gpsData.speed_kmh.toFixed(1)} km/h
				</span>
			{:else}
				<span class="status-indicator status-waiting">Esperando señal GPS...</span>
			{/if}
		</div>
	</div>
	<div class="map-container">
		{#if mapError}
			<div class="map-error">
				<h3>❌ Error cargando el mapa</h3>
				<p>{mapError}</p>
				<p class="hint">Verifica que el servidor de tiles esté corriendo en http://127.0.0.1:8765</p>
			</div>
		{:else}
			<div bind:this={mapContainer} class="map-element"></div>
		{/if}
	</div>
	<ServerStatus />
	<ServerAliveIndicator />
</div>

<style>
	.map-page {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background-color: #0a0a0f;
		color: #f0f0f0;
	}

	.map-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1.5rem;
		background-color: #1a1a2e;
		border-bottom: 2px solid #0066cc;
	}

	.back-button {
		background-color: #16213e;
		color: #e94560;
		border: 2px solid #e94560;
		border-radius: 0.75rem;
		padding: 0.75rem 1.5rem;
		font-size: 1.25rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s ease;
	}

	.back-button:active {
		transform: scale(0.95);
		background-color: #e94560;
		color: #1a1a2e;
	}

	.gps-status {
		font-size: 0.875rem;
		color: #a0a0c0;
	}

	.status-indicator {
		padding: 0.25rem 0.75rem;
		border-radius: 0.5rem;
		font-weight: 500;
	}

	.status-valid {
		color: #00ff88;
	}

	.status-loading {
		color: #ffaa00;
	}

	.status-error {
		color: #ff4444;
	}

	.status-waiting {
		color: #666688;
	}

	.map-container {
		flex: 1;
		position: relative;
		width: 100%;
		height: calc(100vh - 60px);
	}

	.map-element {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
	}

	.map-error {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		background: rgba(20, 20, 30, 0.95);
		border: 2px solid #ff4444;
		border-radius: 12px;
		padding: 2rem;
		text-align: center;
		color: #ffaaaa;
		max-width: 400px;
		z-index: 1000;
	}

	.map-error h3 {
		margin: 0 0 1rem 0;
		color: #ff4444;
	}

	.map-error p {
		margin: 0.5rem 0;
	}

	.map-error .hint {
		font-size: 0.85rem;
		color: #aa8888;
		margin-top: 1rem !important;
	}
</style>