import { invoke } from "@tauri-apps/api/core";
import { goto } from "$app/navigation";

export interface GpsData {
	latitude: number;
	longitude: number;
	speed_kmh: number;
	altitude_m: number;
	valid: boolean;
	timestamp: string;
	source: string;
}

let gpsData = $state<GpsData | null>(null);
let isLoading = $state(false);
let error = $state<string | null>(null);
let intervalId: ReturnType<typeof setInterval> | null = null;

const POLLING_INTERVAL_MS = 2000;

async function fetchGpsPosition(): Promise<void> {
	isLoading = true;
	error = null;
	try {
		const result = await invoke<GpsData>("get_gps_position");
		gpsData = result;
	} catch (e) {
		error = e instanceof Error ? e.message : String(e);
	} finally {
		isLoading = false;
	}
}

export function startGpsPolling(): void {
	if (intervalId !== null) return;

	fetchGpsPosition();
	intervalId = setInterval(fetchGpsPosition, POLLING_INTERVAL_MS);
}

export function stopGpsPolling(): void {
	if (intervalId !== null) {
		clearInterval(intervalId);
		intervalId = null;
	}
}

export function navigateTo(path: string): void {
	goto(path);
}

export const gpsStore = {
	get gpsData() {
		return gpsData;
	},
	get isLoading() {
		return isLoading;
	},
	get error() {
		return error;
	},
	startPolling: startGpsPolling,
	stopPolling: stopGpsPolling,
	navigateTo,
};