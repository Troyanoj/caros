<script lang="ts">
	import Dashboard from "$lib/components/Dashboard.svelte";
	import { onMount } from "svelte";

	onMount(() => {
		console.log("✅ DASHBOARD MONTADO - La app está funcionando");

		const debug = document.createElement("div");
		debug.id = "debug-marker";
		debug.textContent = "🟢 FRONTEND ACTIVO - Esperando respuesta del backend...";
		debug.style.cssText =
			"position: fixed; top: 10px; left: 10px; background: rgba(0,255,0,0.8); color: black; padding: 10px; border-radius: 8px; z-index: 9999; font-family: monospace; font-weight: bold;";
		document.body.appendChild(debug);

		return () => {
			const existing = document.getElementById("debug-marker");
			if (existing) existing.remove();
		};
	});
</script>

<Dashboard />
