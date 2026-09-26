/**
 * Human-readable message of a caught error. Tauri rejects `invoke` with the backend error
 * *string* (CrateError serializes to its message), so `instanceof Error` alone loses it.
 */
export function toErrorMessage(error: unknown, fallback: string): string {
	if (typeof error === 'string' && error.trim() !== '') return error
	if (error instanceof Error && error.message) return error.message
	if (error && typeof error === 'object' && 'message' in error) {
		const message = (error as { message: unknown }).message
		if (typeof message === 'string' && message.trim() !== '') return message
	}
	return fallback
}
