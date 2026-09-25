import '@testing-library/jest-dom/vitest'
import { vi } from 'vitest'

// Browser API mocks
Object.defineProperty(window, 'matchMedia', {
	writable: true,
	value: vi.fn().mockImplementation((query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addListener: vi.fn(),
		removeListener: vi.fn(),
		addEventListener: vi.fn(),
		removeEventListener: vi.fn(),
		dispatchEvent: vi.fn(),
	})),
})

class ResizeObserverMock {
	observe = vi.fn()
	unobserve = vi.fn()
	disconnect = vi.fn()
}
window.ResizeObserver = ResizeObserverMock

class IntersectionObserverMock {
	observe = vi.fn()
	unobserve = vi.fn()
	disconnect = vi.fn()
}
window.IntersectionObserver = IntersectionObserverMock

window.scrollTo = vi.fn()
Element.prototype.scrollIntoView = vi.fn()

// Tauri API mocks
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn().mockResolvedValue(undefined),
	convertFileSrc: vi.fn((filePath: string) => `asset://${filePath}`),
}))

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn().mockResolvedValue(() => {}),
	emit: vi.fn().mockResolvedValue(undefined),
	once: vi.fn().mockResolvedValue(() => {}),
}))
