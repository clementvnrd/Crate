<script lang="ts">
	import { beatportStore } from '$shared/stores/beatport'
	import Icon from '$lib/components/common/Icon.svelte'
	import Spinner from '$lib/components/common/Spinner.svelte'
	import { focusTrap } from '$lib/components/common/focusTrap'
	import { openUrl } from '@tauri-apps/plugin-opener'

	let mode = $state<'token' | 'pkce'>('token')
	let authCode = $state('')
	let directToken = $state('')
	let isSubmitting = $state(false)

	function closeModal() {
		beatportStore.closeLoginModal()
	}

	async function handleOpenDocs() {
		await openUrl('https://api.beatport.com/v4/docs/')
	}

	async function handleOpenWeb() {
		await beatportStore.startPkceLogin()
	}

	async function handlePkceSubmit() {
		if (!authCode.trim()) return
		isSubmitting = true
		try {
			await beatportStore.loginWithPkce(authCode.trim())
		} finally {
			isSubmitting = false
		}
	}

	async function handleTokenSubmit() {
		if (!directToken.trim()) return
		isSubmitting = true
		try {
			await beatportStore.loginWithToken(directToken.trim())
		} finally {
			isSubmitting = false
		}
	}
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm">
	<div
		role="dialog"
		aria-modal="true"
		aria-labelledby="beatport-login-title"
		use:focusTrap={{ onEscape: closeModal }}
		class="relative w-full max-w-lg overflow-hidden rounded-2xl border border-[#00FF96]/40 bg-[#121418] shadow-2xl shadow-[#00FF96]/10 focus-visible:outline-none"
	>
		<!-- Header -->
		<div class="flex items-center justify-between border-b border-[#252830] px-6 py-4">
			<div class="flex items-center gap-3">
				<Icon name="beatport" class="h-6 w-6 text-text-primary" />
				<div>
					<h2 id="beatport-login-title" class="text-base font-semibold text-white">Connexion Beatport Streaming</h2>
					<p class="text-xs text-neutral-400">Associez votre compte Beatport à Crate</p>
				</div>
			</div>
			<button
				type="button"
				class="cursor-pointer rounded-lg p-1 text-neutral-400 hover:bg-neutral-800 hover:text-white"
				onclick={closeModal}
			>
				<Icon name="x" class="h-5 w-5" />
			</button>
		</div>

		<!-- Tabs -->
		<div class="flex border-b border-[#252830] bg-[#181a20] px-6 text-xs font-medium">
			<button
				type="button"
				class="border-b-2 px-4 py-3 transition-colors {mode === 'token'
					? 'border-[#00FF96] font-semibold text-[#00FF96]'
					: 'border-transparent text-neutral-400 hover:text-neutral-200'}"
				onclick={() => (mode = 'token')}
			>
				Token Permanent (Recommandé)
			</button>
			<button
				type="button"
				class="border-b-2 px-4 py-3 transition-colors {mode === 'pkce'
					? 'border-[#00FF96] font-semibold text-[#00FF96]'
					: 'border-transparent text-neutral-400 hover:text-neutral-200'}"
				onclick={() => (mode = 'pkce')}
			>
				Code d'autorisation
			</button>
		</div>

		<!-- Content Area -->
		<div class="space-y-4 p-6 text-sm text-neutral-300">
			{#if mode === 'token'}
				<div class="space-y-3">
					<div class="space-y-2 rounded-xl border border-[#2e323d] bg-[#0e1014] p-3 text-xs">
						<div class="flex items-center justify-between">
							<span class="font-bold text-white">Comment obtenir votre session permanente :</span>
							<button
								type="button"
								class="flex cursor-pointer items-center gap-1 font-semibold text-[#00FF96] hover:underline"
								onclick={handleOpenDocs}
							>
								<span>Ouvrir api.beatport.com</span>
								<Icon name="external-link" class="h-3 w-3" />
							</button>
						</div>
						<ol class="list-inside list-decimal space-y-1 text-[11px] leading-relaxed text-neutral-400">
							<li>
								Ouvrez <strong class="text-neutral-200">api.beatport.com/v4/docs/</strong> et cliquez sur le bouton vert
								<strong class="text-emerald-400">Authorize</strong> en haut à droite.
							</li>
							<li>Connectez-vous avec votre compte Beatport.</li>
							<li>
								Ouvrez l'inspecteur web (<code class="text-[#00FF96]">F12</code> ou
								<code class="text-[#00FF96]">Cmd+Opt+I</code>) &rarr; onglet
								<strong class="text-neutral-200">Réseau (Network)</strong>.
							</li>
							<li>
								Filtrez sur <code class="text-[#00FF96]">token</code> et copiez la réponse JSON contenant
								<code class="text-emerald-400">access_token</code>
								et <code class="text-emerald-400">refresh_token</code>.
							</li>
						</ol>
					</div>

					<div>
						<label for="bp-token-input" class="mb-1 block text-xs font-medium text-neutral-400"
							>Collez le JSON ou Token ici :</label
						>
						<textarea
							id="bp-token-input"
							bind:value={directToken}
							rows="4"
							placeholder={'{\n  "access_token": "eyJ...",\n  "refresh_token": "1PBpi..."\n}\nou collez le texte commençant par eyJ...'}
							class="w-full rounded-lg border border-[#2e323d] bg-[#0e1014] p-2.5 font-mono text-xs text-white placeholder-neutral-600 focus:border-[#00FF96] focus:outline-none"
						></textarea>
					</div>

					<button
						type="button"
						disabled={isSubmitting || !directToken.trim()}
						class="flex w-full cursor-pointer items-center justify-center gap-2 rounded-xl bg-[#00FF96] py-2.5 text-xs font-bold text-black shadow-lg shadow-[#00FF96]/20 hover:bg-[#00e687] disabled:opacity-50"
						onclick={handleTokenSubmit}
					>
						{#if isSubmitting}
							<Spinner class="h-3 w-3" />
						{:else}
							Valider et Activer la Connexion Permanente
						{/if}
					</button>
				</div>
			{:else if mode === 'pkce'}
				<div class="space-y-4">
					<div class="space-y-2">
						<p class="text-xs leading-relaxed text-neutral-300">
							1. Cliquez pour ouvrir la page officielle de connexion Beatport dans votre navigateur :
						</p>

						<button
							type="button"
							class="flex w-full cursor-pointer items-center justify-center gap-2 rounded-xl bg-[#00FF96] py-3 text-xs font-bold text-black transition-all hover:bg-[#00e687] hover:shadow-lg hover:shadow-[#00FF96]/20"
							onclick={handleOpenWeb}
						>
							<Icon name="external-link" class="h-4 w-4" />
							<span>Ouvrir la connexion Beatport</span>
						</button>
					</div>

					<div class="space-y-2 border-t border-[#252830] pt-3">
						<label for="bp-auth-code" class="block text-xs font-medium text-neutral-400">
							2. Après connexion, copiez l'URL de votre navigateur (ou le code) et collez-la ici :
						</label>
						<div class="flex gap-2">
							<input
								id="bp-auth-code"
								type="text"
								bind:value={authCode}
								placeholder="https://api.beatport.com/... ou code d'autorisation"
								class="flex-1 rounded-lg border border-[#2e323d] bg-[#0e1014] px-3 py-2 text-xs text-white placeholder-neutral-500 focus:border-[#00FF96] focus:outline-none"
							/>
							<button
								type="button"
								disabled={isSubmitting || !authCode.trim()}
								class="flex cursor-pointer items-center gap-1 rounded-lg bg-[#00FF96] px-4 py-2 text-xs font-semibold text-black hover:bg-[#00e687] disabled:opacity-50"
								onclick={handlePkceSubmit}
							>
								{#if isSubmitting}
									<Spinner class="h-3 w-3" />
								{:else}
									Valider
								{/if}
							</button>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
