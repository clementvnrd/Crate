import { describe, it, expect } from 'vitest'
import {
	getFieldsForContext,
	getFieldDefinition,
	getOperatorsForType,
	operatorRequiresValue,
	operatorRequiresSecondValue,
	getSortFieldsForContext,
	createDefaultCondition,
	conditionHasValue,
	parseSmartRules,
	serializeSmartRules,
	findDeletedTagIds,
	type FieldDefinition,
} from './smartRules'
import type { SmartRules, TagCategory } from '../types'

describe('smartRules utils', () => {
	describe('getFieldsForContext', () => {
		it('returns discovery fields when context is "discovery"', () => {
			const fields = getFieldsForContext('discovery')
			expect(fields).toHaveLength(9)
			const fieldNames = fields.map((f) => f.field)
			expect(fieldNames).toContain('title')
			expect(fieldNames).toContain('artist')
			expect(fieldNames).toContain('source_type')
			expect(fieldNames).toContain('release_date')
			expect(fieldNames).toContain('notes')
			expect(fieldNames).toContain('tags')
			expect(fieldNames).not.toContain('bpm')
			expect(fieldNames).not.toContain('bitrate')
		})

		it('returns library fields for "library" or any other context', () => {
			const libraryFields = getFieldsForContext('library')
			expect(libraryFields).toHaveLength(20)
			const fieldNames = libraryFields.map((f) => f.field)
			expect(fieldNames).toContain('bpm')
			expect(fieldNames).toContain('rating')
			expect(fieldNames).toContain('play_count')
			expect(fieldNames).toContain('color')
			expect(fieldNames).toContain('format')
			expect(fieldNames).toContain('bitrate')
			expect(fieldNames).toContain('sample_rate')

			const defaultFields = getFieldsForContext('unknown-context')
			expect(defaultFields).toEqual(libraryFields)
		})
	})

	describe('getFieldDefinition', () => {
		it('finds existing field definitions in context', () => {
			const bpmDef = getFieldDefinition('bpm', 'library')
			expect(bpmDef).toBeDefined()
			expect(bpmDef?.field).toBe('bpm')
			expect(bpmDef?.type).toBe('numeric')

			const sourceTypeDef = getFieldDefinition('source_type', 'discovery')
			expect(sourceTypeDef).toBeDefined()
			expect(sourceTypeDef?.field).toBe('source_type')
			expect(sourceTypeDef?.type).toBe('enum')
			expect(sourceTypeDef?.enumValues).toBeDefined()
			expect(sourceTypeDef?.enumValues?.length).toBeGreaterThan(0)
		})

		it('returns undefined for non-existent field in context', () => {
			expect(getFieldDefinition('bpm', 'discovery')).toBeUndefined()
			expect(getFieldDefinition('source_type', 'library')).toBeUndefined()
			expect(getFieldDefinition('non_existent', 'library')).toBeUndefined()
		})
	})

	describe('getOperatorsForType', () => {
		it('returns text operators for text type', () => {
			const ops = getOperatorsForType('text')
			const opValues = ops.map((o) => o.value)
			expect(opValues).toEqual([
				'contains',
				'not_contains',
				'equals',
				'not_equals',
				'starts_with',
				'ends_with',
				'is_empty',
				'is_not_empty',
			])
		})

		it('returns numeric operators for numeric type', () => {
			const ops = getOperatorsForType('numeric')
			const opValues = ops.map((o) => o.value)
			expect(opValues).toEqual(['equals', 'not_equals', 'greater_than', 'less_than', 'in_range'])
		})

		it('returns date operators for date type', () => {
			const ops = getOperatorsForType('date')
			const opValues = ops.map((o) => o.value)
			expect(opValues).toEqual(['in_last_days', 'not_in_last_days', 'before', 'after', 'is_empty', 'is_not_empty'])
		})

		it('returns enum operators for enum type', () => {
			const ops = getOperatorsForType('enum')
			const opValues = ops.map((o) => o.value)
			expect(opValues).toEqual(['equals', 'not_equals', 'is_empty', 'is_not_empty'])
		})

		it('returns tag operators for tags type', () => {
			const ops = getOperatorsForType('tags')
			const opValues = ops.map((o) => o.value)
			expect(opValues).toEqual(['has_any', 'has_all', 'has_none'])
		})
	})

	describe('operatorRequiresValue', () => {
		it('returns false for is_empty and is_not_empty', () => {
			expect(operatorRequiresValue('is_empty')).toBe(false)
			expect(operatorRequiresValue('is_not_empty')).toBe(false)
		})

		it('returns true for other operators', () => {
			expect(operatorRequiresValue('contains')).toBe(true)
			expect(operatorRequiresValue('equals')).toBe(true)
			expect(operatorRequiresValue('in_range')).toBe(true)
			expect(operatorRequiresValue('has_any')).toBe(true)
			expect(operatorRequiresValue('in_last_days')).toBe(true)
		})
	})

	describe('operatorRequiresSecondValue', () => {
		it('returns true only for in_range', () => {
			expect(operatorRequiresSecondValue('in_range')).toBe(true)
			expect(operatorRequiresSecondValue('equals')).toBe(false)
			expect(operatorRequiresSecondValue('contains')).toBe(false)
			expect(operatorRequiresSecondValue('is_empty')).toBe(false)
		})
	})

	describe('getSortFieldsForContext', () => {
		it('returns discovery sort fields for "discovery"', () => {
			const sortFields = getSortFieldsForContext('discovery')
			const values = sortFields.map((s) => s.value)
			expect(values).toEqual(['date_added', 'title', 'artist', 'release_date', 'random'])
		})

		it('returns library sort fields for "library" or any other context', () => {
			const sortFields = getSortFieldsForContext('library')
			const values = sortFields.map((s) => s.value)
			expect(values).toEqual(['date_added', 'rating', 'play_count', 'bpm', 'title', 'artist', 'random'])

			const defaultFields = getSortFieldsForContext('custom')
			expect(defaultFields).toEqual(sortFields)
		})
	})

	describe('createDefaultCondition', () => {
		it('creates default text condition', () => {
			const fieldDef: FieldDefinition = { field: 'title', labelKey: 'title', type: 'text' }
			const condition = createDefaultCondition(fieldDef)
			expect(condition).toEqual({
				type: 'text',
				field: 'title',
				operator: 'contains',
				value: '',
			})
		})

		it('creates default numeric condition', () => {
			const fieldDef: FieldDefinition = { field: 'bpm', labelKey: 'bpm', type: 'numeric' }
			const condition = createDefaultCondition(fieldDef)
			expect(condition).toEqual({
				type: 'numeric',
				field: 'bpm',
				operator: 'equals',
				value: 0,
			})
		})

		it('creates default date condition', () => {
			const fieldDef: FieldDefinition = { field: 'date_added', labelKey: 'date_added', type: 'date' }
			const condition = createDefaultCondition(fieldDef)
			expect(condition).toEqual({
				type: 'date',
				field: 'date_added',
				operator: 'in_last_days',
				value: '30',
			})
		})

		it('creates default enum condition with first enum value or fallback', () => {
			const fieldDefWithEnums: FieldDefinition = {
				field: 'color',
				labelKey: 'color',
				type: 'enum',
				enumValues: [
					{ value: 'pink', labelKey: 'pink' },
					{ value: 'red', labelKey: 'red' },
				],
			}
			expect(createDefaultCondition(fieldDefWithEnums)).toEqual({
				type: 'enum',
				field: 'color',
				operator: 'equals',
				value: 'pink',
			})

			const fieldDefWithoutEnums: FieldDefinition = {
				field: 'color',
				labelKey: 'color',
				type: 'enum',
			}
			expect(createDefaultCondition(fieldDefWithoutEnums)).toEqual({
				type: 'enum',
				field: 'color',
				operator: 'equals',
				value: '',
			})
		})

		it('creates default tags condition', () => {
			const fieldDef: FieldDefinition = { field: 'tags', labelKey: 'tags', type: 'tags' }
			const condition = createDefaultCondition(fieldDef)
			expect(condition).toEqual({
				type: 'tags',
				operator: 'has_any',
				tag_ids: [],
			})
		})
	})

	describe('conditionHasValue', () => {
		it('returns true if operator does not require a value', () => {
			expect(conditionHasValue({ type: 'text', field: 'title', operator: 'is_empty' })).toBe(true)
			expect(conditionHasValue({ type: 'text', field: 'title', operator: 'is_not_empty' })).toBe(true)
			expect(conditionHasValue({ type: 'date', field: 'date_added', operator: 'is_empty' })).toBe(true)
		})

		it('evaluates text condition values', () => {
			expect(conditionHasValue({ type: 'text', field: 'title', operator: 'contains', value: 'House' })).toBe(true)
			expect(conditionHasValue({ type: 'text', field: 'title', operator: 'contains', value: '' })).toBe(false)
			expect(conditionHasValue({ type: 'text', field: 'title', operator: 'contains', value: undefined })).toBe(false)
		})

		it('evaluates date condition values', () => {
			expect(conditionHasValue({ type: 'date', field: 'date_added', operator: 'in_last_days', value: '30' })).toBe(true)
			expect(conditionHasValue({ type: 'date', field: 'date_added', operator: 'in_last_days', value: '' })).toBe(false)
		})

		it('evaluates numeric condition values including in_range', () => {
			expect(conditionHasValue({ type: 'numeric', field: 'bpm', operator: 'equals', value: 128 })).toBe(true)
			expect(conditionHasValue({ type: 'numeric', field: 'bpm', operator: 'equals', value: 0 })).toBe(true)
			expect(conditionHasValue({ type: 'numeric', field: 'bpm', operator: 'equals', value: undefined })).toBe(false)

			expect(conditionHasValue({ type: 'numeric', field: 'bpm', operator: 'in_range', value: 120, value2: 130 })).toBe(
				true
			)
			expect(conditionHasValue({ type: 'numeric', field: 'bpm', operator: 'in_range', value: 120, value2: 0 })).toBe(
				true
			)
			expect(conditionHasValue({ type: 'numeric', field: 'bpm', operator: 'in_range', value: 120 })).toBe(false)
			expect(
				// @ts-expect-error test null value
				conditionHasValue({ type: 'numeric', field: 'bpm', operator: 'in_range', value: null, value2: 130 })
			).toBe(false)
		})

		it('evaluates enum condition values', () => {
			expect(conditionHasValue({ type: 'enum', field: 'color', operator: 'equals', value: 'pink' })).toBe(true)
			expect(conditionHasValue({ type: 'enum', field: 'color', operator: 'equals', value: '' })).toBe(false)
			// @ts-expect-error test null value
			expect(conditionHasValue({ type: 'enum', field: 'color', operator: 'equals', value: null })).toBe(false)
		})

		it('evaluates tags condition tag_ids array', () => {
			expect(conditionHasValue({ type: 'tags', operator: 'has_any', tag_ids: ['tag-1'] })).toBe(true)
			expect(conditionHasValue({ type: 'tags', operator: 'has_all', tag_ids: ['tag-1', 'tag-2'] })).toBe(true)
			expect(conditionHasValue({ type: 'tags', operator: 'has_none', tag_ids: [] })).toBe(false)
		})
	})

	describe('parseSmartRules and serializeSmartRules', () => {
		const sampleRules: SmartRules = {
			match_mode: 'all',
			conditions: [
				{ type: 'text', field: 'genre', operator: 'contains', value: 'House' },
				{ type: 'numeric', field: 'bpm', operator: 'greater_than', value: 120 },
			],
			limit: {
				count: 50,
				sort_field: 'bpm',
				sort_direction: 'descending',
			},
		}

		it('returns null for null, empty or invalid JSON string', () => {
			expect(parseSmartRules(null)).toBeNull()
			expect(parseSmartRules('')).toBeNull()
			expect(parseSmartRules('invalid json')).toBeNull()
			expect(parseSmartRules('{ invalid }')).toBeNull()
		})

		it('parses valid JSON string into SmartRules object', () => {
			const json = JSON.stringify(sampleRules)
			const parsed = parseSmartRules(json)
			expect(parsed).toEqual(sampleRules)
		})

		it('serializes SmartRules object into JSON string', () => {
			const serialized = serializeSmartRules(sampleRules)
			expect(typeof serialized).toBe('string')
			expect(JSON.parse(serialized)).toEqual(sampleRules)
		})
	})

	describe('findDeletedTagIds', () => {
		const sampleCategories: TagCategory[] = [
			{
				id: 'cat-1',
				name: 'Genre',
				color: '#ef4444',
				sort_order: 1,
				tags: [
					{ id: 'tag-1', category_id: 'cat-1', name: 'House', color: null, sort_order: 1 },
					{ id: 'tag-2', category_id: 'cat-1', name: 'Techno', color: null, sort_order: 2 },
				],
			},
			{
				id: 'cat-2',
				name: 'Mood',
				color: '#3b82f6',
				sort_order: 2,
				tags: [{ id: 'tag-3', category_id: 'cat-2', name: 'Energetic', color: null, sort_order: 1 }],
			},
		]

		it('returns empty array when all tag IDs exist in categories', () => {
			const deleted = findDeletedTagIds(['tag-1', 'tag-2', 'tag-3'], sampleCategories)
			expect(deleted).toEqual([])
		})

		it('identifies tag IDs that no longer exist in any category', () => {
			const deleted = findDeletedTagIds(['tag-1', 'deleted-tag-1', 'tag-3', 'deleted-tag-2'], sampleCategories)
			expect(deleted).toEqual(['deleted-tag-1', 'deleted-tag-2'])
		})

		it('returns all IDs if categories is empty', () => {
			expect(findDeletedTagIds(['tag-1', 'tag-2'], [])).toEqual(['tag-1', 'tag-2'])
		})

		it('returns empty array if input tagIds is empty', () => {
			expect(findDeletedTagIds([], sampleCategories)).toEqual([])
		})
	})
})
