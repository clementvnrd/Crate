<script lang="ts">
	import { Text, ToggleSwitch } from '$lib/components/common'
	import { displaySettingsStore } from '$shared/stores/displaySettings'
	import { translate } from '$shared/i18n'
</script>

<div class="space-y-6 pb-6 select-none">
	<div>
		<Text variant="header-2" weight="bold">{$translate('settings.displayOptions.title')}</Text>
		<Text variant="body-2" color="secondary" class="mt-1">
			{$translate('settings.displayOptions.subtitle')}
		</Text>
	</div>

	<!-- Audio quality -->
	<section class="space-y-2 rounded-lg border border-stroke bg-surface-1 p-4">
		<Text variant="header-4" weight="semibold">{$translate('settings.displayOptions.audioQuality')}</Text>
		<div class="divide-y divide-stroke-subtle">
			<ToggleSwitch
				label={$translate('settings.displayOptions.audioImprovementMessage')}
				checked={$displaySettingsStore.showAudioImprovementMessage}
				onchange={(val) => displaySettingsStore.setShowAudioImprovementMessage(val)}
			/>
		</div>
	</section>

	<!-- Waveform -->
	<section class="space-y-2 rounded-lg border border-stroke bg-surface-1 p-4">
		<Text variant="header-4" weight="semibold">{$translate('settings.displayOptions.waveform')}</Text>
		<div class="divide-y divide-stroke-subtle">
			<ToggleSwitch
				label={$translate('settings.displayOptions.showDetectedCuePoints')}
				checked={$displaySettingsStore.showDetectedCuePoints}
				onchange={(val) => displaySettingsStore.setShowDetectedCuePoints(val)}
			/>
			<ToggleSwitch
				label={$translate('settings.displayOptions.showDetectedEnergyLevels')}
				checked={$displaySettingsStore.showDetectedEnergyLevels}
				onchange={(val) => displaySettingsStore.setShowDetectedEnergyLevels(val)}
			/>
		</div>
	</section>

	<!-- Camelot numbers -->
	<section class="space-y-2 rounded-lg border border-stroke bg-surface-1 p-4">
		<Text variant="header-4" weight="semibold">{$translate('settings.displayOptions.camelotNumbers')}</Text>
		<div class="divide-y divide-stroke-subtle">
			<ToggleSwitch
				label={$translate('settings.displayOptions.camelotZeroPadding')}
				description={$translate('settings.displayOptions.camelotZeroPaddingExample')}
				checked={$displaySettingsStore.camelotZeroPadding}
				onchange={(val) => displaySettingsStore.setCamelotZeroPadding(val)}
			/>
		</div>
	</section>

	<!-- Columns to display -->
	<section class="space-y-2 rounded-lg border border-stroke bg-surface-1 p-4">
		<Text variant="header-4" weight="semibold">{$translate('settings.displayOptions.columnsToDisplay')}</Text>
		<div class="grid grid-cols-1 md:grid-cols-2 gap-x-6 divide-y md:divide-y-0 divide-stroke-subtle">
			<div class="space-y-1 divide-y divide-stroke-subtle">
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.coverArt')}
					checked={$displaySettingsStore.columns.artwork}
					onchange={(val) => displaySettingsStore.setColumn('artwork', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.title')}
					checked={$displaySettingsStore.columns.title}
					onchange={(val) => displaySettingsStore.setColumn('title', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.artist')}
					checked={$displaySettingsStore.columns.artist}
					onchange={(val) => displaySettingsStore.setColumn('artist', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.album')}
					checked={$displaySettingsStore.columns.album}
					onchange={(val) => displaySettingsStore.setColumn('album', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.tempo')}
					checked={$displaySettingsStore.columns.bpm}
					onchange={(val) => displaySettingsStore.setColumn('bpm', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.key')}
					checked={$displaySettingsStore.columns.key}
					onchange={(val) => displaySettingsStore.setColumn('key', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.energy')}
					checked={$displaySettingsStore.columns.energy}
					onchange={(val) => displaySettingsStore.setColumn('energy', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.format')}
					checked={$displaySettingsStore.columns.format}
					onchange={(val) => displaySettingsStore.setColumn('format', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.bitrate')}
					checked={$displaySettingsStore.columns.bitrate}
					onchange={(val) => displaySettingsStore.setColumn('bitrate', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.time')}
					checked={$displaySettingsStore.columns.duration_ms}
					onchange={(val) => displaySettingsStore.setColumn('duration_ms', val)}
				/>
			</div>

			<div class="space-y-1 divide-y divide-stroke-subtle">
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.tags')}
					checked={$displaySettingsStore.columns.tags}
					onchange={(val) => displaySettingsStore.setColumn('tags', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.dateAdded')}
					checked={$displaySettingsStore.columns.date_added}
					onchange={(val) => displaySettingsStore.setColumn('date_added', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.rating')}
					checked={$displaySettingsStore.columns.rating}
					onchange={(val) => displaySettingsStore.setColumn('rating', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.filePath')}
					checked={$displaySettingsStore.columns.file_path}
					onchange={(val) => displaySettingsStore.setColumn('file_path', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.fileSize')}
					checked={$displaySettingsStore.columns.file_size}
					onchange={(val) => displaySettingsStore.setColumn('file_size', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.sampleRate')}
					checked={$displaySettingsStore.columns.sample_rate}
					onchange={(val) => displaySettingsStore.setColumn('sample_rate', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.genre')}
					checked={$displaySettingsStore.columns.genre}
					onchange={(val) => displaySettingsStore.setColumn('genre', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.label')}
					checked={$displaySettingsStore.columns.label}
					onchange={(val) => displaySettingsStore.setColumn('label', val)}
				/>
				<ToggleSwitch
					label={$translate('settings.displayOptions.columns.year')}
					checked={$displaySettingsStore.columns.year}
					onchange={(val) => displaySettingsStore.setColumn('year', val)}
				/>
			</div>
		</div>
	</section>
</div>
