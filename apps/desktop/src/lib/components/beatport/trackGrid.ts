// Column layout of the Beatport track table, shared by its header (BeatportView) and its rows (BeatportTrackRow).
// Columns: # · artwork · title & artist · genre · released · length · key · BPM · actions.
//
// The table is a size container (`@container` on its frame). The title column never goes under 160 px: when the
// table gets narrower, the release date goes first (under 52rem, 832 px), then the genre (under 45rem, 720 px), so
// the title keeps its room instead of shrinking to one character (register D7). At the default 1400 px window the
// table is 1110 px wide and no container rule applies: the layout is the historical one.

export const BEATPORT_TRACK_GRID =
	'grid-cols-[36px_40px_minmax(160px,1fr)_120px_100px_60px_64px_50px_100px] ' +
	'@max-[52rem]:grid-cols-[36px_40px_minmax(160px,1fr)_120px_60px_64px_50px_100px] ' +
	'@max-[45rem]:grid-cols-[36px_40px_minmax(160px,1fr)_60px_64px_50px_100px]'

/** The genre cell, hidden in the narrowest layout. */
export const BEATPORT_GENRE_CELL = '@max-[45rem]:hidden'

/** The release-date cell, the first one hidden when the table narrows. */
export const BEATPORT_DATE_CELL = '@max-[52rem]:hidden'

/** The mix name next to the title: in the narrow layouts it keeps 48 px instead of being squeezed by a long title. */
export const BEATPORT_MIX_NAME = '@max-[52rem]:min-w-12'
