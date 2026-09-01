// The locations page tree, ported from the legacy useLocationTree:
// stations and structures root the tree, containers and ships nest by
// their location_id links, counts roll up, and the search filter keeps
// matching nodes together with their ancestors.

export interface LocationRow {
	id: number;
	type: { id: number; name: string } | null;
	name: string | null;
	location: { id: number | null } | null;
	character_id: number | null;
	corporation_id: number | null;
	slug: string;
}

export interface StationRow {
	id: number;
	type_id: number | null;
	/** Null for a structure the resolver has not named yet. */
	name: string | null;
	slug: string;
}

export interface LocationsData {
	locations: LocationRow[];
	stations: StationRow[];
	structures: StationRow[];
	location_modules_count: Record<string, number>;
}

export interface TreeNode {
	id: number;
	name: string;
	type: string;
	type_id: number | null;
	slug: string;
	count: number;
	character: number | null;
	corporation: number | null;
	children: TreeNode[];
}

export function buildTree(data: LocationsData): TreeNode[] {
	// Deliberate divergence from legacy (which labeled everything
	// "Station"): structures are labeled as such.
	return [
		...data.stations.map((station) => stationNode(station, 'Station', data)),
		...data.structures.map((structure) => stationNode(structure, 'Structure', data)),
	].sort((a, b) => a.name.localeCompare(b.name));
}

function stationNode(
	station: StationRow,
	kind: 'Station' | 'Structure',
	data: LocationsData,
): TreeNode {
	const children = childNodes(data, station.id);
	return {
		id: station.id,
		name: station.name || 'Unknown Structure',
		type: kind,
		type_id: station.type_id,
		slug: station.slug,
		count: nodeCount(station.id, children, data.location_modules_count),
		character: null,
		corporation: null,
		children,
	};
}

function childNodes(data: LocationsData, parentId: number): TreeNode[] {
	return data.locations
		.filter((location) => location.location?.id === parentId)
		.sort((a, b) => (a.type?.id ?? 0) - (b.type?.id ?? 0))
		.map((location) => {
			const children = childNodes(data, location.id);
			return {
				id: location.id,
				name: location.name || location.type?.name || 'Unknown',
				type: location.type?.name || 'Unknown',
				type_id: location.type?.id ?? null,
				slug: location.slug,
				count: nodeCount(location.id, children, data.location_modules_count),
				character: location.character_id,
				corporation: location.corporation_id,
				children,
			};
		});
}

function nodeCount(nodeId: number, children: TreeNode[], counts: Record<string, number>): number {
	const direct = counts[String(nodeId)] ?? 0;
	return direct + children.reduce((sum, child) => sum + child.count, 0);
}

/** Keeps nodes matching the query by name or type, and every ancestor
 * of a match (the legacy filterTreeRecursive). */
export function filterTree(nodes: TreeNode[], query: string): TreeNode[] {
	const needle = query.trim().toLowerCase();
	if (needle === '') {
		return nodes;
	}
	return nodes
		.map((node) => ({ ...node, children: filterTree(node.children, needle) }))
		.filter((node) => matches(node, needle) || node.children.length > 0);
}

function matches(node: TreeNode, needle: string): boolean {
	return node.name.toLowerCase().includes(needle) || node.type.toLowerCase().includes(needle);
}
