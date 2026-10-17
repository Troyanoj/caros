<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { gpsStore } from "$lib/stores/gpsStore";
	import { type Map as MapLibreMap, type Marker, type LngLatLike, type StyleSpecification } from "maplibre-gl";
	import type { GpsData } from "$lib/stores/gpsStore";
	import ServerStatus from "./ServerStatus.svelte";

	import "maplibre-gl/dist/maplibre-gl.css";
	import "$lib/styles/global.css";
	import mapStyleData from "$lib/styles/map-style.json";

	const mapStyle: StyleSpecification = mapStyleData as unknown as StyleSpecification;

	let mapContainer: HTMLDivElement;
	let map: MapLibreMap | null = null;
	let marker: Marker | null = null;
	let mapReady = $state(false);

	const TILE_SERVER_URL = "http://127.0.0.1:8765";
	const defaultCenter: LngLatLike = [-77.0365, 38.8977];

	function watchGpsPosition(): void {
		const currentGps = gpsStore.gpsData;
		if (!currentGps || !currentGps.valid || !mapReady || !marker || !map) return;

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
		gpsStore.gpsData;
		if (gpsStore.gpsData?.valid && mapReady && marker && map) {
			watchGpsPosition();
		}
	});

	async function initMap(): Promise<void> {
		if (!mapContainer) return;

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
		});
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
		<div bind:this={mapContainer} class="map-element"></div>
	</div>
	<ServerStatus />
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
</style>
