<script lang="ts">
	import { beatportStore } from '$shared/stores/beatport'
	import Icon from '$lib/components/common/Icon.svelte'
	import Spinner from '$lib/components/common/Spinner.svelte'
	import { focusTrap } from '$lib/components/common/focusTrap'
	import { openUrl } from '@tauri-apps/plugin-opener'
	import { translate } from '$shared/i18n'

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
		data-surface="beatport"
		use:focusTrap={{ onEscape: closeModal }}
		class="relative w-full max-w-lg overflow-hidden rounded-2xl border border-beatport/40 bg-surface-1 shadow-2xl shadow-beatport/10 focus-visible:outline-none"
	>
		<!-- Header -->
		<div class="flex items-center justify-between border-b border-stroke-subtle px-6 py-4">
			<div class="flex items-center gap-3">
				<Icon name="beatport" class="h-6 w-6 text-text-primary" />
				<div>
					<h2 id="beatport-login-title" class="text-base font-semibold text-text-primary">
						{$translate('beatport.login.title')}
					</h2>
					<p class="text-xs text-text-secondary">{$translate('beatport.login.subtitle')}</p>
				</div>
			</div>
			<button
				type="button"
				class="cursor-pointer rounded-lg p-1 text-text-secondary hover:bg-surface-3 hover:text-text-primary"
				onclick={closeModal}
			>
				<Icon name="x" class="h-5 w-5" />
			</button>
		</div>

		<!-- Tabs -->
		<div class="flex border-b border-stroke-subtle bg-surface-2 px-6 text-xs font-medium">
			<button
				type="button"
				class="border-b-2 px-4 py-3 transition-colors {mode === 'token'
					? 'border-beatport font-semibold text-beatport-text-strong'
					: 'border-transparent text-text-secondary hover:text-beatport-body-text-strong'}"
				onclick={() => (mode = 'token')}
			>
				{$translate('beatport.login.tabs.token')}
			</button>
			<button
				type="button"
				class="border-b-2 px-4 py-3 transition-colors {mode === 'pkce'
					? 'border-beatport font-semibold text-beatport-text-strong'
					: 'border-transparent text-text-secondary hover:text-beatport-body-text-strong'}"
				onclick={() => (mode = 'pkce')}
			>
				{$translate('beatport.login.tabs.code')}
			</button>
		</div>

		<!-- Content Area -->
		<div class="space-y-4 p-6 text-sm text-beatport-body-text">
			{#if mode === 'token'}
				<div class="space-y-3">
					<div class="space-y-2 rounded-xl border border-stroke bg-surface-0 p-3 text-xs">
						<div class="flex items-center justify-between">
							<span class="font-bold text-text-primary">{$translate('beatport.login.token.howTo')}</span>
							<button
								type="button"
								class="flex cursor-pointer items-center gap-1 font-semibold text-beatport-text-strong hover:underline"
								onclick={handleOpenDocs}
							>
								<span>{$translate('beatport.login.token.openDocs')}</span>
								<Icon name="external-link" class="h-3 w-3" />
							</button>
						</div>
						<ol class="list-inside list-decimal space-y-1 text-[11px] leading-relaxed text-text-secondary">
							<li>
								{$translate('beatport.login.token.step1Open')}
								<strong class="text-beatport-body-text-strong">api.beatport.com/v4/docs/</strong>
								{$translate('beatport.login.token.step1Click')}
								<strong class="text-beatport-text">Authorize</strong>
								{$translate('beatport.login.token.step1End')}
							</li>
							<li>{$translate('beatport.login.token.step2')}</li>
							<li>
								{$translate('beatport.login.token.step3Open')}<code class="text-beatport-text-strong">F12</code>
								{$translate('beatport.login.token.step3Or')}
								<code class="text-beatport-text-strong">Cmd+Opt+I</code>) &rarr; {$translate(
									'beatport.login.token.step3TabBefore'
								)}
								<strong class="text-beatport-body-text-strong">{$translate('beatport.login.token.step3Tab')}</strong>.
							</li>
							<li>
								{$translate('beatport.login.token.step4Filter')} <code class="text-beatport-text-strong">token</code>
								{$translate('beatport.login.token.step4Copy')}
								<code class="text-beatport-text">access_token</code>
								{$translate('beatport.login.token.step4And')} <code class="text-beatport-text">refresh_token</code>.
							</li>
						</ol>
					</div>

					<div>
						<label for="bp-token-input" class="mb-1 block text-xs font-medium text-text-secondary"
							>{$translate('beatport.login.token.label')}</label
						>
						<textarea
							id="bp-token-input"
							bind:value={directToken}
							rows="4"
							placeholder={'{\n  "access_token": "eyJ...",\n  "refresh_token": "1PBpi..."\n}\n' +
								$translate('beatport.login.token.placeholderHint')}
							class="w-full rounded-lg border border-stroke bg-surface-0 p-2.5 font-mono text-xs text-text-primary placeholder-text-disabled focus:border-beatport focus:outline-none"
						></textarea>
					</div>

					<button
						type="button"
						disabled={isSubmitting || !directToken.trim()}
						class="flex w-full cursor-pointer items-center justify-center gap-2 rounded-xl bg-beatport py-2.5 text-xs font-bold text-black shadow-lg shadow-beatport/20 hover:bg-beatport-hover disabled:opacity-50"
						onclick={handleTokenSubmit}
					>
						{#if isSubmitting}
							<Spinner class="h-3 w-3" />
						{:else}
							{$translate('beatport.login.token.submit')}
						{/if}
					</button>
				</div>
			{:else if mode === 'pkce'}
				<div class="space-y-4">
					<div class="space-y-2">
						<p class="text-xs leading-relaxed text-beatport-body-text">
							{$translate('beatport.login.code.step1')}
						</p>

						<button
							type="button"
							class="flex w-full cursor-pointer items-center justify-center gap-2 rounded-xl bg-beatport py-3 text-xs font-bold text-black transition-all hover:bg-beatport-hover hover:shadow-lg hover:shadow-beatport/20"
							onclick={handleOpenWeb}
						>
							<Icon name="external-link" class="h-4 w-4" />
							<span>{$translate('beatport.login.code.open')}</span>
						</button>
					</div>

					<div class="space-y-2 border-t border-stroke-subtle pt-3">
						<label for="bp-auth-code" class="block text-xs font-medium text-text-secondary">
							{$translate('beatport.login.code.step2')}
						</label>
						<div class="flex gap-2">
							<input
								id="bp-auth-code"
								type="text"
								bind:value={authCode}
								placeholder={$translate('beatport.login.code.placeholder')}
								class="flex-1 rounded-lg border border-stroke bg-surface-0 px-3 py-2 text-xs text-text-primary placeholder-text-tertiary focus:border-beatport focus:outline-none"
							/>
							<button
								type="button"
								disabled={isSubmitting || !authCode.trim()}
								class="flex cursor-pointer items-center gap-1 rounded-lg bg-beatport px-4 py-2 text-xs font-semibold text-black hover:bg-beatport-hover disabled:opacity-50"
								onclick={handlePkceSubmit}
							>
								{#if isSubmitting}
									<Spinner class="h-3 w-3" />
								{:else}
									{$translate('beatport.login.code.submit')}
								{/if}
							</button>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
