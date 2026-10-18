<script lang="ts">
	import { onMount, onDestroy } from "svelte";

	type StatusType = "checking" | "alive" | "dead";

	let status: StatusType = $state("checking");
	let errorMessage: string = $state("");
	let lastCheck: Date | null = $state(null);

	let interval: ReturnType<typeof setInterval>;

	async function checkServer(): Promise<void> {
		try {
			const controller = new AbortController();
			const timeoutId = setTimeout(() => controller.abort(), 2000);

			const response = await fetch("http://127.0.0.1:8765/ping", {
				signal: controller.signal,
			});

			clearTimeout(timeoutId);

			if (response.ok) {
				const text = await response.text();
				status = "alive";
				errorMessage = "";
				console.log("[ServerAliveIndicator] Servidor vivo:", text);
			} else {
				status = "dead";
				errorMessage = `HTTP ${response.status} ${response.statusText}`;
				console.error("[ServerAliveIndicator] Servidor error:", response.status);
			}
		} catch (e) {
			status = "dead";
			if (e instanceof DOMException && e.name === "AbortError") {
				errorMessage = "Timeout (2s)";
			} else if (e instanceof TypeError) {
				errorMessage = `Error de conexión: ${e.message}`;
			} else {
				errorMessage = e instanceof Error ? e.message : String(e);
			}
			console.error("[ServerAliveIndicator] Error fetch:", errorMessage);
		}

		lastCheck = new Date();
	}

	onMount(() => {
		checkServer();
		interval = setInterval(checkServer, 2000);

		return () => {
			clearInterval(interval);
		};
	});

	onDestroy(() => {
		clearInterval(interval);
	});
</script>

<div class={`server-indicator ${status}`}>
	{#if status === "checking"}
		⏳ Verificando servidor...
	{:else if status === "alive"}
		✅ Servidor ACTIVO (puerto 8765)
		<div class="last-check">Última verificación: {lastCheck?.toLocaleTimeString()}</div>
	{:else}
		❌ Servidor CAÍDO: {errorMessage}
		<div class="hint">
			El backend de Rust no está corriendo.<br />
			Revisa el log: %TEMP%\caros_debug.log
		</div>
		<button class="retry-btn" onclick={checkServer}>🔄 Reintentar ahora</button>
	{/if}
</div>

<style>
	.server-indicator {
		position: fixed;
		top: 10px;
		right: 10px;
		padding: 12px 20px;
		border-radius: 12px;
		font-weight: bold;
		z-index: 10001;
		font-family: "Courier New", Courier, monospace;
		font-size: 14px;
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
	}

	.server-indicator.checking {
		background-color: #ffd93d;
		color: #000;
		animation: pulse-yellow 2s infinite;
	}

	.server-indicator.alive {
		background-color: #22c55e;
		color: #fff;
	}

	.server-indicator.dead {
		background-color: #ef4444;
		color: #fff;
		animation: pulse-red 2s infinite;
	}

	.last-check {
		font-size: 12px;
		opacity: 0.8;
		margin-top: 4px;
	}

	.hint {
		font-size: 12px;
		opacity: 0.9;
		margin-top: 8px;
		line-height: 1.4;
	}

	.retry-btn {
		margin-top: 8px;
		padding: 6px 12px;
		font-size: 13px;
		background-color: rgba(255, 255, 255, 0.3);
		color: inherit;
		border: 1px solid rgba(255, 255, 255, 0.5);
		border-radius: 6px;
		cursor: pointer;
		font-family: inherit;
		font-weight: bold;
	}

	.retry-btn:hover {
		background-color: rgba(255, 255, 255, 0.5);
	}

	@keyframes pulse-yellow {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.7; }
	}

	@keyframes pulse-red {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.7; }
	}
</style>
