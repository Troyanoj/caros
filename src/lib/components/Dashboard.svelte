<script lang="ts">
	import { gpsStore } from "$lib/stores/gpsStore";
	import { onNavigate } from "$app/navigation";
	import ServerStatus from "./ServerStatus.svelte";
	import BackendDiagnostic from "./BackendDiagnostic.svelte";
	import ServerAliveIndicator from "./ServerAliveIndicator.svelte";

	function handleNavigate(path: string): void {
		if (path === "/navigate") {
			gpsStore.startPolling();
		}
		gpsStore.navigateTo(path);
	}

	onNavigate(() => {
		gpsStore.stopPolling();
	});
</script>

<main class="dashboard">
	<h1 class="dashboard-title">CarOS</h1>
	<div class="button-grid">
		<button
			class="nav-button nav-button--navigate"
			onclick={() => handleNavigate("/navigate")}
		>
			<span class="nav-button-icon">🛣️</span>
			<span class="nav-button-label">Navegar</span>
		</button>

		<button class="nav-button nav-button--music">
			<span class="nav-button-icon">🎵</span>
			<span class="nav-button-label">Música</span>
		</button>

		<button class="nav-button nav-button--gas">
			<span class="nav-button-icon">⛽</span>
			<span class="nav-button-label">Gasolinera</span>
		</button>
	</div>
	<ServerStatus />
	<ServerAliveIndicator />
	<BackendDiagnostic />
</main>

<style>
	.dashboard {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		gap: 3rem;
		background-color: #0a0a0f;
		color: #f0f0f0;
		padding: 1rem;
	}

	.dashboard-title {
		font-size: 3.5rem;
		font-weight: 700;
		color: #ffffff;
		margin: 0;
		text-shadow: 0 0 10px rgba(0, 200, 255, 0.5);
	}

	.button-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1.5rem;
		width: 100%;
		max-width: 700px;
	}

	.nav-button {
		min-width: 0;
		width: 100%;
		aspect-ratio: 1 / 1;
		min-height: 200px;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		gap: 0.75rem;
		border: none;
		border-radius: 1.5rem;
		cursor: pointer;
		transition: all 0.15s ease;
		user-select: none;
	}

	.nav-button:active {
		transform: scale(0.95);
		filter: brightness(1.3);
	}

	.nav-button-icon {
		font-size: 4rem;
	}

	.nav-button-label {
		font-size: 1.5rem;
		font-weight: 600;
	}

	.nav-button--navigate {
		background: linear-gradient(135deg, #0066cc 0%, #0099ff 100%);
		color: #ffffff;
	}

	.nav-button--navigate:active {
		background: linear-gradient(135deg, #0055b3 0%, #0088e6 100%);
	}

	.nav-button--music {
		background: linear-gradient(135deg, #7b2cbf 0%, #a855f7 100%);
		color: #ffffff;
	}

	.nav-button--music:active {
		background: linear-gradient(135deg, #6b1a9e 0%, #9333ea 100%);
	}

	.nav-button--gas {
		background: linear-gradient(135deg, #f59e0b 0%, #f9b31b 100%);
		color: #1a1a2e;
	}

	.nav-button--gas:active {
		background: linear-gradient(135deg, #d97706 0%, #eab308 100%);
	}
</style>
