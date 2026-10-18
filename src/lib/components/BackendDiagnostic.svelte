<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	interface BackendStatus {
		tiles_dir: string;
		tiles_dir_exists: boolean;
		mbtiles_found: boolean;
		mbtiles_path: string;
		mbtiles_exists: boolean;
		appdata: string;
		port: string;
		all_mbtiles_files: string[];
	}

	let status: BackendStatus | null = $state(null);
	let isLoading: boolean = $state(true);
	let error: string | null = $state(null);

	async function fetchStatus(): Promise<void> {
		isLoading = true;
		error = null;
		try {
			const result = await invoke<BackendStatus>("get_backend_status");
			status = result;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			console.error("[BackendDiagnostic] Error fetching status:", error);
		} finally {
			isLoading = false;
		}
	}

	function openTilesDir(): void {
		if (status?.tiles_dir) {
			invoke("plugin:opener|open_path", { path: status.tiles_dir })
				.catch((e) => console.error("No se pudo abrir la carpeta:", e));
		}
	}

	onMount(() => {
		fetchStatus();
		const interval = setInterval(fetchStatus, 5000);

		return () => {
			clearInterval(interval);
		};
	});
</script>

<div class="diagnostic-panel">
	<h2 style="margin: 0 0 12px 0; font-size: 1.5rem;">🛠️ Diagnóstico del Backend</h2>

	{#if isLoading}
		<p style="margin: 0;">Verificando estado del backend...</p>
	{:else if error}
		<p style="color: #cc0000; margin: 0;">❌ Error: {error}</p>
	{:else if status}
		<div class="diagnostic-grid">
			<div class="diag-row">
				<span class="diag-label">APPDATA:</span>
				<span class="diag-value">{status.appdata}</span>
			</div>

			<div class="diag-row">
				<span class="diag-label">Tiles Dir:</span>
				<span class="diag-value code">{status.tiles_dir}</span>
			</div>

			<div class="diag-row">
				<span class="diag-label">Dir existe:</span>
				<span class="diag-value {status.tiles_dir_exists ? 'yes' : 'no'}">
					{status.tiles_dir_exists ? "✅ Sí" : "❌ No"}
				</span>
			</div>

			<div class="diag-row">
				<span class="diag-label">MBTiles:</span>
				<span class="diag-value {status.mbtiles_exists ? 'yes' : 'no'}">
					{status.mbtiles_exists ? "✅ Existe" : "❌ No existe"}
				</span>
			</div>

			{#if status.mbtiles_path}
				<div class="diag-row">
					<span class="diag-label">Ruta MBTiles:</span>
					<span class="diag-value code">{status.mbtiles_path}</span>
				</div>
			{/if}

			<div class="diag-row">
				<span class="diag-label">Puerto tiles:</span>
				<span class="diag-value code">{status.port}</span>
			</div>

			{#if status.all_mbtiles_files.length > 0}
				<div class="diag-row">
					<span class="diag-label">Archivos .mbtiles:</span>
					<div class="diag-files">
						{#each status.all_mbtiles_files as file}
							<span class="diag-file">{file}</span>
						{/each}
					</div>
				</div>
			{/if}
		</div>

		<button class="open-btn" onclick={openTilesDir} disabled={!status.tiles_dir_exists}>
			📂 Abrir carpeta de tiles
		</button>

		{#if !status.mbtiles_exists && status.tiles_dir_exists}
			<p class="hint">
				💡 Copie thailand.mbtiles a la carpeta mostrada arriba y reinicie la aplicación.
			</p>
		{:else if !status.mbtiles_exists && !status.tiles_dir_exists}
			<p class="hint">
				💡 La carpeta no existe. ¡Presione el botón para crearla y luego copie thailand.mbtiles!
			</p>
		{/if}
	{/if}
</div>

<style>
	.diagnostic-panel {
		position: fixed;
		top: 10px;
		left: 10px;
		right: 10px;
		max-width: 400px;
		background-color: #ffd93d;
		color: #000;
		border: 3px solid #000;
		border-radius: 16px;
		padding: 16px;
		font-family: "Courier New", Courier, monospace;
		font-size: 16px;
		font-weight: bold;
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
		z-index: 10000;
		max-height: 70vh;
		overflow-y: auto;
	}

	.diagnostic-grid {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin: 12px 0;
	}

	.diag-row {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.diag-label {
		color: #333;
		opacity: 0.8;
	}

	.diag-value {
		color: #000;
	}

	.diag-value.code {
		font-size: 13px;
		word-break: break-all;
	}

	.diag-value.yes {
		color: #006600;
	}

	.diag-value.no {
		color: #cc0000;
	}

	.diag-files {
		margin-top: 4px;
	}

	.diag-file {
		display: block;
		font-size: 12px;
		color: #333;
		word-break: break-all;
	}

	.open-btn {
		background-color: #2563eb;
		color: white;
		border: 2px solid #1d4ed8;
		border-radius: 8px;
		padding: 10px 16px;
		font-size: 16px;
		font-weight: bold;
		cursor: pointer;
		margin: 8px 0;
		transition: background-color 0.15s;
	}

	.open-btn:hover {
		background-color: #1d4ed8;
	}

	.open-btn:disabled {
		background-color: #6b7280;
		cursor: not-allowed;
	}

	.hint {
		color: #92400e;
		font-size: 14px;
		margin: 8px 0 0 0;
		padding: 8px;
		background-color: rgba(255, 255, 255, 0.5);
		border-radius: 6px;
		text-align: center;
	}
</style>
