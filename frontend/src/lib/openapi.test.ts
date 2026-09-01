import { describe, expect, it } from 'vitest';

import {
	groupOperations,
	operationId,
	reachableSchemas,
	refName,
	typeLabel,
	type OpenApiDocument,
} from './openapi';

const document: OpenApiDocument = {
	info: { title: 'Test', version: '1' },
	tags: [
		{ name: 'Modules', description: 'Module things.' },
		{ name: 'Reference', description: 'Reference things.' },
	],
	paths: {
		'/modules': {
			// Deliberately out of reading order: post before get.
			post: {
				tags: ['Modules'],
				summary: 'Import',
				requestBody: {
					content: { 'application/json': { schema: { $ref: '#/components/schemas/Import' } } },
				},
				responses: {
					'422': { description: 'Invalid' },
					'200': {
						description: 'Imported',
						content: { 'application/json': { schema: { $ref: '#/components/schemas/Envelope' } } },
					},
				},
			},
			get: {
				tags: ['Modules'],
				summary: 'Reject',
				responses: {
					'404': {
						description: 'No type',
						content: {
							'application/json': {
								schema: { $ref: '#/components/schemas/ApiError' },
								example: { message: 'Please provide a valid type.' },
							},
						},
					},
				},
			},
		},
		'/estimator-statistics': {
			get: { tags: ['Reference'], summary: 'Stats', responses: {} },
		},
	},
	components: {
		schemas: {
			Envelope: { type: 'object', properties: { data: { $ref: '#/components/schemas/Module' } } },
			Module: { type: 'object', properties: { id: { type: 'integer' } } },
			Import: { type: 'object', properties: { message: { type: ['string', 'null'] } } },
			ApiError: { type: 'object', properties: { message: { type: 'string' } } },
			Unused: { type: 'object', properties: {} },
		},
	},
};

describe('operationId', () => {
	it('is a stable anchor, templated segments included', () => {
		expect(operationId('get', '/modules/{query}')).toBe('get-modules-query');
		expect(operationId('POST', '/modules')).toBe('post-modules');
		expect(operationId('get', '/abyssal-type-statistics')).toBe('get-abyssal-type-statistics');
	});
});

describe('typeLabel', () => {
	it('names a referenced component rather than calling it an object', () => {
		expect(typeLabel({ $ref: '#/components/schemas/Module' })).toBe('Module');
		expect(refName({ $ref: '#/components/schemas/Module' })).toBe('Module');
		expect(refName({ type: 'string' })).toBeNull();
	});

	it('reads arrays, nullables and unions the way the spec writes them', () => {
		expect(typeLabel({ type: 'array', items: { $ref: '#/components/schemas/Module' } })).toBe(
			'Module[]',
		);
		expect(typeLabel({ type: ['string', 'null'] })).toBe('string or null');
		expect(typeLabel({ oneOf: [{ type: 'string' }, { type: 'integer' }] })).toBe(
			'string or integer',
		);
		expect(typeLabel(undefined)).toBe('any');
	});
});

describe('groupOperations', () => {
	const groups = groupOperations(document);

	it('groups by tag, in the order the document declares them', () => {
		expect(groups.map((group) => group.name)).toEqual(['Modules', 'Reference']);
		expect(groups[0].description).toBe('Module things.');
	});

	it('orders methods the way a reader expects, not the map order', () => {
		expect(groups[0].operations.map((operation) => operation.method)).toEqual(['GET', 'POST']);
	});

	it('carries the parts the page renders', () => {
		const post = groups[0].operations.find((operation) => operation.method === 'POST');
		expect(post?.requestBody).toBe('Import');
		expect(post?.responses.map((response) => response.status)).toEqual(['200', '422']);
		expect(post?.responses[0].schema).toBe('Envelope');
	});

	it('keeps a documented example so the page can show a real body', () => {
		const get = groups[0].operations.find((operation) => operation.method === 'GET');
		expect(get?.responses[0].example).toEqual({ message: 'Please provide a valid type.' });
	});

	it('drops a tag with no operations', () => {
		const empty = groupOperations({ ...document, paths: {} });
		expect(empty).toEqual([]);
	});
});

describe('reachableSchemas', () => {
	it('follows references so nested types are documented too', () => {
		expect(reachableSchemas(document, ['Envelope']).sort()).toEqual(['Envelope', 'Module']);
	});

	it('leaves out what no endpoint reaches', () => {
		expect(reachableSchemas(document, ['ApiError'])).not.toContain('Unused');
	});

	it('terminates on a cycle', () => {
		const cyclic: OpenApiDocument = {
			...document,
			components: {
				schemas: {
					A: { properties: { b: { $ref: '#/components/schemas/B' } } },
					B: { properties: { a: { $ref: '#/components/schemas/A' } } },
				},
			},
		};
		expect(reachableSchemas(cyclic, ['A']).sort()).toEqual(['A', 'B']);
	});
});

describe('componentName', () => {
	it('follows an array to the type a reader wants to read about', () => {
		const arrayResponse: OpenApiDocument = {
			...document,
			paths: {
				'/stats': {
					get: {
						tags: ['Reference'],
						responses: {
							'200': {
								description: 'Stats',
								content: {
									'application/json': {
										schema: { type: 'array', items: { $ref: '#/components/schemas/Module' } },
									},
								},
							},
						},
					},
				},
			},
		};
		const [group] = groupOperations(arrayResponse);
		const response = group.operations[0].responses[0];

		// The label keeps the array; the link points at the item type.
		expect(response.schema).toBe('Module[]');
		expect(response.schemaRef).toBe('Module');
	});
});
