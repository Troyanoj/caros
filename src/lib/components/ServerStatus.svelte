<script lang="ts">
	import { onMount } from "svelte";

	type HealthStatus = "loading" | "ok" | "error";

	interface HealthResponse {
		status: "ok" | "error";
		mbtiles_found: boolean;
		tiles_dir: string;
		mbtiles_path: string;
	}

	let healthStatus: HealthStatus = $state("loading");
	let errorMessage: string = $state("");
	let healthData: HealthResponse | null = $state(null);

	const TILE_SERVER_URL = "http://127.0.0.1:8765";

	async function checkHealth(): Promise<void> {
		healthStatus = "loading";
		errorMessage = "";

		try {
			const response = await fetch(`${TILE_SERVER_URL}/health`, {
				method: "GET",
				mode: "no-cors",
				signal: AbortSignal.timeout(5000),
			});

			if (!response.ok) {
				healthStatus = "error";
				errorMessage = `HTTP ${response.status} ${response.statusText}`;
				return;
			}

			try {
				const data: HealthResponse = await response.json();
				healthData = data;
				healthStatus = "ok";
			} catch (parseError) {
				healthStatus = "ok";
			}
		} catch (e) {
			healthStatus = "error";
			if (e instanceof DOMException && e.name === "AbortError") {
				errorMessage = "Timeout: El servidor no respondió en 5 segundos";
			} else if (e instanceof TypeError) {
				errorMessage = `Error de red: ${e.message}`;
			} else {
				errorMessage = e instanceof Error ? e.message : String(e);
			}
		}
	}

	onMount(() => {
		checkHealth();
		const interval = setInterval(checkHealth, 10000);

		return () => {
			clearInterval(interval);
		};
	});
</script>

<div class="server-status">
	{#if healthStatus === "loading"}
		<span class="status-loading">Verificando servidor de mapas...</span>
	{:else if healthStatus === "ok"}
		<span class="status-ok">
			✅ Servidor de mapas: OK
			{#if healthData?.mbtiles_found}
				(Archivo encontrado: Sí)
			{:else}
				(Archivo encontrado: No)
			{/if}
		</span>
	{:else if healthStatus === "error"}
		<span class="status-error" title={errorMessage}>
			❌ Error en servidor de mapas: {errorMessage}
		</span>
	{/if}
</div>

<style>
	.server-status {
		position: fixed;
		bottom: 16px;
		left: 16px;
		background-color: rgba(10, 10, 15, 0.85);
		backdrop-filter: blur(4px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		padding: 8px 12px;
		font-size: 12px;
		font-family: "SF Mono", "Fira Code", Consolas, monospace;
		max-width: 480px;
		z-index: 9999;
	}

	.status-loading {
		color: #ffaa00;
	}

	.status-ok {
		color: #00ff88;
	}

	.status-error {
		color: #ff4444;
		word-break: break-word;
	}
</style>
