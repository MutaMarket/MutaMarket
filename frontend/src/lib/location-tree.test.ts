import { describe, expect, it } from 'vitest';
import { buildTree, filterTree, type LocationsData } from './location-tree';

const data: LocationsData = {
	stations: [{ id: 100, type_id: null, name: 'Jita IV - Moon 4', slug: 'jita-iv-moon-4-100' }],
	structures: [{ id: 200, type_id: 35832, name: 'Home Fortizar', slug: 'home-fortizar-200' }],
	locations: [
		{
			id: 1,
			type: { id: 28514, name: 'Iteron Mark V' },
			name: 'My Hauler',
			location: { id: 100 },
			character_id: 7,
			corporation_id: null,
			slug: 'my-hauler-1'
		},
		{
			id: 2,
			type: { id: 3467, name: 'Small Standard Container' },
			name: null,
			location: { id: 1 },
			character_id: 7,
			corporation_id: null,
			slug: 'small-standard-container-2'
		}
	],
	location_modules_count: { '100': 1, '1': 2, '2': 3 }
};

describe('buildTree', () => {
	it('roots stations and structures alphabetically with rolled-up counts', () => {
		const tree = buildTree(data);
		expect(tree.map((node) => node.name)).toEqual(['Home Fortizar', 'Jita IV - Moon 4']);
		const jita = tree[1];
		expect(jita.count).toBe(6);
		expect(jita.children).toHaveLength(1);
		expect(jita.children[0].name).toBe('My Hauler');
		expect(jita.children[0].count).toBe(5);
		expect(jita.children[0].children[0].name).toBe('Small Standard Container');
		expect(jita.children[0].children[0].count).toBe(3);
	});
});

describe('filterTree', () => {
	it('keeps matches and their ancestors', () => {
		const tree = buildTree(data);
		const filtered = filterTree(tree, 'container');
		expect(filtered).toHaveLength(1);
		expect(filtered[0].name).toBe('Jita IV - Moon 4');
		expect(filtered[0].children[0].children[0].name).toBe('Small Standard Container');
	});

	it('drops stations without matches and returns all for a blank query', () => {
		const tree = buildTree(data);
		expect(filterTree(tree, 'fortizar')).toHaveLength(1);
		expect(filterTree(tree, '  ')).toHaveLength(2);
	});
});
