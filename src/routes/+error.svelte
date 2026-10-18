<script lang="ts">
	import { page } from "$app/stores";
	import { onMount } from "svelte";
	import { get } from "svelte/store";
	import BackendDiagnostic from "$lib/components/BackendDiagnostic.svelte";
	import ServerAliveIndicator from "$lib/components/ServerAliveIndicator.svelte";

	onMount(() => {
		const currentPage = get(page);
		console.log("[error] Página de error renderizada. Status:", currentPage.status);
		console.log("[error] Error:", currentPage.error);

		const debug = document.createElement("div");
		debug.id = "error-debug-marker";
		debug.textContent =
			"🛑 ERROR 500 - Página de error cargada. El backend falló. Mensaje: " +
			(currentPage.error?.message || currentPage.error?.toString() || "desconocido");
		debug.style.cssText =
			"position: fixed; bottom: 10px; left: 10px; background: rgba(255,0,0,0.9); color: white; padding: 12px 16px; border-radius: 8px; z-index: 99999; font-family: monospace; font-weight: bold; max-width: 600px; font-size: 14px;";
		document.body.appendChild(debug);

		return () => {
			const existing = document.getElementById("error-debug-marker");
			if (existing) existing.remove();
		};
	});
</script>

<BackendDiagnostic />

<div
	class="error-page"
	style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: #1a1a1a; color: white; display: flex; flex-direction: column; align-items: center; justify-content: center; font-family: monospace; z-index: 1;"
>
	<h1 style="color: #ff4444; font-size: 2rem; margin-bottom: 20px;">🛑 ERROR 500 DETECTADO</h1>
	<p style="font-size: 1.2rem; margin: 10px 0;">Estado HTTP: {$page.status}</p>
	<p style="margin-top: 20px; max-width: 600px; text-align: center; line-height: 1.6;">
		Esto significa que el backend de Rust no respondió correctamente.<br /><br />
		Posibles causas:
	</p>
	<ul style="text-align: left; max-width: 600px; margin: 20px 0;">
		<li>• El servidor de tiles no arrancó (puerto 8765 ocupado o sin permisos)</li>
		<li>• No se encontró el archivo thailand.mbtiles</li>
		<li>• Error en la inicialización de Tauri</li>
	</ul>
	<button
		onclick={() => window.location.reload()}
		style="margin-top: 30px; padding: 20px 40px; font-size: 20px; background: #2563eb; color: white; border: none; border-radius: 12px; cursor: pointer;">
		🔄 REINTENTAR
	</button>
</div>

<BackendDiagnostic />
<ServerAliveIndicator />
