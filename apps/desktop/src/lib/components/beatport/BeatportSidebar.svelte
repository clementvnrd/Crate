<script lang="ts">
	import { beatportStore, type BeatportNavSection } from '$shared/stores/beatport'
	import { Text, Icon } from '$lib/components/common'
	import { translate } from '$shared/i18n'

	let showNewPlaylistInput = $state(false)
	let newPlaylistName = $state('')

	function handleNav(section: BeatportNavSection, playlistId: string | null = null, playlistName: string | null = null) {
		beatportStore.setNavSection(section, playlistId, playlistName)
	}

	function handleCreatePlaylist() {
		if (newPlaylistName.trim()) {
			beatportStore.createPlaylist(newPlaylistName.trim())
			newPlaylistName = ''
			showNewPlaylistInput = false
		}
	}
</script>

<div class="flex h-full flex-col justify-between overflow-y-auto bg-surface-0 p-3 select-none text-xs border-r border-stroke">
	<div class="space-y-4">
		<!-- Main Navigation -->
		<div class="space-y-1">
			<button
				type="button"
				class="flex w-full items-center justify-between rounded-md px-2.5 py-1.5 font-medium transition-colors {$beatportStore.navSection === 'home' && !$beatportStore.selectedPlaylistId
					? 'bg-emerald-500/20 text-emerald-400 font-semibold'
					: 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}"
				onclick={() => handleNav('home')}
			>
				<div class="flex items-center gap-2">
					<Icon name="beatport" class="h-3.5 w-3.5" />
					<span>Beatport</span>
				</div>
			</button>

			<button
				type="button"
				class="flex w-full items-center justify-between rounded-md px-2.5 py-1.5 font-medium transition-colors {$beatportStore.navSection === 'purchased'
					? 'bg-emerald-500/20 text-emerald-400 font-semibold'
					: 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}"
				onclick={() => handleNav('purchased')}
			>
				<div class="flex items-center gap-2">
					<Icon name="download" class="h-3.5 w-3.5" />
					<span>Purchased tracks</span>
				</div>
				{#if $beatportStore.purchases.length > 0}
					<span class="rounded bg-surface-2 border border-stroke px-1.5 py-0.2 text-[10px] font-mono text-emerald-400">
						{$beatportStore.purchases.length}
					</span>
				{/if}
			</button>

			<button
				type="button"
				class="flex w-full items-center justify-between rounded-md px-2.5 py-1.5 font-medium transition-colors {$beatportStore.navSection === 'offline'
					? 'bg-emerald-500/20 text-emerald-400 font-semibold'
					: 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}"
				onclick={() => handleNav('offline')}
			>
				<div class="flex items-center gap-2">
					<Icon name="database" class="h-3.5 w-3.5" />
					<span>Offline library</span>
				</div>
			</button>

			<button
				type="button"
				class="flex w-full items-center justify-between rounded-md px-2.5 py-1.5 font-medium transition-colors {$beatportStore.navSection === 'favorites'
					? 'bg-emerald-500/20 text-emerald-400 font-semibold'
					: 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}"
				onclick={() => handleNav('favorites')}
			>
				<div class="flex items-center gap-2">
					<Icon name="heart" class="h-3.5 w-3.5 text-red-400" />
					<span>Favorites</span>
				</div>
				<span class="rounded bg-surface-2 border border-stroke px-1.5 py-0.2 text-[10px] font-mono text-emerald-400">
					{$beatportStore.favorites.length}
				</span>
			</button>
		</div>

		<!-- Playlists Header -->
		<div class="pt-2">
			<div class="flex items-center justify-between px-2.5 pb-1 text-[11px] font-bold tracking-wider text-text-tertiary uppercase">
				<span>Playlists</span>
				<div class="flex items-center gap-1">
					<button
						type="button"
						class="rounded p-0.5 hover:bg-surface-2 hover:text-text-primary"
						title="Créer une Playlist Beatport"
						onclick={() => (showNewPlaylistInput = !showNewPlaylistInput)}
					>
						<Icon name="plus" class="h-3.5 w-3.5" />
					</button>
				</div>
			</div>

			{#if showNewPlaylistInput}
				<div class="my-1.5 flex items-center gap-1 px-1">
					<input
						type="text"
						placeholder="Nouvelle playlist..."
						bind:value={newPlaylistName}
						class="w-full rounded bg-surface-2 border border-stroke px-2 py-1 text-xs text-text-primary focus:border-emerald-500 focus:outline-none"
						onkeydown={(e) => e.key === 'Enter' && handleCreatePlaylist()}
					/>
				</div>
			{/if}

			<!-- Playlists List -->
			<div class="space-y-0.5">
				{#if $beatportStore.userPlaylists.length === 0}
					<div class="px-2.5 py-2 text-[11px] text-text-tertiary">
						{#if $beatportStore.auth.is_authenticated}
							Aucune playlist Beatport
						{:else}
							Connectez-vous pour voir vos playlists
						{/if}
					</div>
				{:else}
					{#each $beatportStore.userPlaylists as pl (pl.id)}
						<button
							type="button"
							class="flex w-full items-center justify-between rounded-md px-2.5 py-1.5 transition-colors {$beatportStore.selectedPlaylistId === String(pl.id)
								? 'bg-emerald-500/20 text-emerald-400 font-semibold'
								: 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}"
							onclick={() => handleNav('playlist', String(pl.id), pl.name)}
						>
							<span class="truncate">{pl.name}</span>
							<span class="rounded bg-surface-2 border border-stroke px-1.5 py-0.2 text-[10px] font-mono text-emerald-400">
								{pl.track_count}
							</span>
						</button>
					{/each}
				{/if}
			</div>
		</div>
	</div>

	<!-- Bottom Status Line -->
	{#if $beatportStore.auth.is_authenticated}
		<div class="flex items-center justify-between border-t border-stroke/60 pt-2.5 pb-1 px-1">
			<div
				class="h-3.5 w-16 bg-text-secondary"
				style="-webkit-mask-image: url('/beatport-full-logo.png'); -webkit-mask-size: contain; -webkit-mask-repeat: no-repeat; -webkit-mask-position: center left; mask-image: url('/beatport-full-logo.png'); mask-size: contain; mask-repeat: no-repeat; mask-position: center left;"
				title="Beatport Streaming"
			></div>
			<div class="flex items-center gap-1.5 text-emerald-500 text-[11px] font-medium">
				<span class="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
				<span>Connecté</span>
			</div>
		</div>
	{:else}
		<div class="border-t border-stroke/60 pt-2.5 px-1">
			<button
				type="button"
				class="flex w-full items-center justify-center gap-2 rounded-lg bg-surface-2 hover:bg-surface-3 border border-stroke py-1.5 text-xs font-semibold text-text-primary transition-all cursor-pointer"
				onclick={() => beatportStore.openLoginModal()}
			>
				<div
					class="h-3.5 w-14 bg-text-primary"
					style="-webkit-mask-image: url('/beatport-full-logo.png'); -webkit-mask-size: contain; -webkit-mask-repeat: no-repeat; -webkit-mask-position: center left; mask-image: url('/beatport-full-logo.png'); mask-size: contain; mask-repeat: no-repeat; mask-position: center left;"
				></div>
				<span>Se connecter</span>
			</button>
		</div>
	{/if}
</div>
