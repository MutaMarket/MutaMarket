// The filter query path grammar, a faithful port of the Rust
// `modules::view` client helpers (themselves ports of the legacy
// `QueryBuilder`). The server-side `modules::search` stays the authority
// for resolution and validation; this mirror only renders and edits the
// URL. The emitted paths must match the Rust builder byte for byte.

export interface UiAttributeFilter {
	/** The attribute name as it appears in the URL (lowercased by the builder). */
	name: string;
	lower: number;
	upper: number | null;
}

export interface UiSearch {
	typeSlug: string | null;
	metaGroup: string | null;
	metaLevel: string | null;
	attributes: UiAttributeFilter[];
	/** [field, descending]; field is price, value, fraction or an attribute name. */
	sort: [string, boolean] | null;
	contractType: string | null;
	price: [number, number | null] | null;
	value: [number, number | null] | null;
	noMultiItemContracts: boolean;
	onlyContracts: boolean;
	withoutOtherItems: boolean;
	goldbar: boolean;
	brownbar: boolean;
	diamondbar: boolean;
	withPersonalModules: boolean;
	inJita: boolean;
}

export function defaultUiSearch(): UiSearch {
	return {
		typeSlug: null,
		metaGroup: null,
		metaLevel: null,
		attributes: [],
		sort: null,
		contractType: null,
		price: null,
		value: null,
		noMultiItemContracts: false,
		onlyContracts: false,
		withoutOtherItems: false,
		goldbar: false,
		brownbar: false,
		diamondbar: false,
		withPersonalModules: false,
		inJita: false
	};
}

/** The option keywords of the query path (mirror of the server-side list). */
const UI_OPTION_KEYWORDS = [
	'page',
	'type',
	'meta-group',
	'meta-level',
	'auction',
	'item-exchange',
	'contracts-only',
	'no-multi-item-contracts',
	'goldbar',
	'brownbar',
	'diamondbar',
	'attributes',
	'contract-price',
	'estimated-value',
	'with-personal-modules',
	'sort',
	'without-contracts',
	'without-fitted',
	'without-other-items',
	'without-assets',
	'created',
	'search',
	'needs-training',
	'in-jita'
];

/** Compact decimal: fixed precision with fractional zeros (and "-0") trimmed. */
export function toPrecision(value: number, precision: number): string {
	const formatted = value.toFixed(precision);
	const trimmed = formatted.includes('.')
		? formatted.replace(/0+$/, '').replace(/\.$/, '')
		: formatted;
	return trimmed === '-0' ? '0' : trimmed;
}

/** Significant digits kept in filter URLs. */
const URL_SIGNIFICANT_DIGITS = 6;

export function formatUrlNumber(value: number): string {
	if (value === 0) {
		return '0';
	}

	const magnitude = Math.floor(Math.log10(Math.abs(value)));
	const decimals = Math.min(Math.max(URL_SIGNIFICANT_DIGITS - 1 - magnitude, 0), 10);
	return toPrecision(value, decimals);
}

/** The trailing item id of a module slug (or a bare id); null for queries. */
export function moduleIdFromSlug(query: string): number | null {
	if (query === '' || query.includes('/') || !/^[a-zA-Z0-9-]+$/.test(query)) {
		return null;
	}

	const digits = /[0-9]+$/.exec(query)?.[0];
	return digits === undefined ? null : Number(digits);
}

export function slugify(text: string): string {
	let slug = '';
	for (const c of text) {
		if (/[a-zA-Z0-9]/.test(c)) {
			slug += c.toLowerCase();
		} else if (slug !== '' && !slug.endsWith('-')) {
			slug += '-';
		}
	}
	return slug.replace(/-+$/, '');
}

/** URL slug of a module: the slugified type name plus the item id. */
export function moduleSlug(typeName: string, itemId: number): string {
	return `${slugify(typeName)}-${itemId}`;
}

function takeLeadingNumber(text: string): [number, string] | null {
	const match = /^-?[0-9]+(\.[0-9]+)?/.exec(text);
	if (match === null) {
		return null;
	}
	return [Number(match[0]), text.slice(match[0].length)];
}

function parseBounds(text: string): [number, number | null] | null {
	const lower = takeLeadingNumber(text);
	if (lower === null) {
		return null;
	}
	const rest = lower[1];
	const upper = rest.startsWith('-') ? takeLeadingNumber(rest.slice(1)) : null;
	return [lower[0], upper === null ? null : upper[0]];
}

/** Parses a filter query path textually for the filter controls. */
export function parseQueryUi(query: string): UiSearch {
	const segments = query.split('/').filter((segment) => segment !== '');
	const search = defaultUiSearch();

	let index = 0;
	while (index < segments.length) {
		const segment = segments[index];
		const argsStart = index + 1;
		let argsEnd = segments.length;
		for (let i = argsStart; i < segments.length; i += 1) {
			if (UI_OPTION_KEYWORDS.includes(segments[i])) {
				argsEnd = i;
				break;
			}
		}
		const args = segments.slice(argsStart, argsEnd);

		switch (segment) {
			case 'type':
				search.typeSlug = args[0] ?? null;
				break;
			case 'meta-group':
				search.metaGroup = args[0] ?? null;
				break;
			case 'meta-level':
				search.metaLevel = args[0] ?? null;
				break;
			case 'auction':
				search.contractType = 'auction';
				break;
			case 'item-exchange':
				search.contractType = 'item_exchange';
				break;
			case 'contracts-only':
				search.onlyContracts = true;
				break;
			case 'no-multi-item-contracts':
				search.noMultiItemContracts = true;
				break;
			case 'without-other-items':
				search.withoutOtherItems = true;
				break;
			case 'goldbar':
				search.goldbar = true;
				break;
			case 'with-personal-modules':
				search.withPersonalModules = true;
				break;
			case 'in-jita':
				search.inJita = true;
				break;
			case 'brownbar':
				search.brownbar = true;
				break;
			case 'diamondbar':
				search.diamondbar = true;
				break;
			case 'contract-price':
				search.price = args[0] === undefined ? null : parseBounds(args[0]);
				break;
			case 'estimated-value':
				search.value = args[0] === undefined ? null : parseBounds(args[0]);
				break;
			case 'sort':
				if (args[0] !== undefined) {
					search.sort = [args[0], args[1] === 'desc'];
				}
				break;
			case 'attributes':
				for (let pair = 0; pair + 1 < args.length; pair += 2) {
					const bounds = parseBounds(args[pair + 1]);
					if (bounds !== null) {
						search.attributes.push({ name: args[pair], lower: bounds[0], upper: bounds[1] });
					}
				}
				break;
		}

		index = UI_OPTION_KEYWORDS.includes(segment) ? Math.max(argsEnd, index + 1) : index + 1;
	}

	return search;
}

/**
 * Builds the filter query path, mirroring the legacy `QueryBuilder.make`
 * segment order exactly.
 */
export function buildQueryPath(prefix: string, search: UiSearch): string {
	const parts: string[] = [];

	if (search.typeSlug !== null) {
		parts.push(`type/${search.typeSlug}`);
	}
	if (search.metaGroup !== null) {
		parts.push(`meta-group/${search.metaGroup}`);
	}
	if (search.metaLevel !== null) {
		parts.push(`meta-level/${search.metaLevel}`);
	}

	if (search.attributes.length > 0) {
		const attributeParts = search.attributes.map((filter) => {
			const name = filter.name.toLowerCase();
			return filter.upper === null
				? `${name}/${formatUrlNumber(filter.lower)}`
				: `${name}/${formatUrlNumber(filter.lower)}-${formatUrlNumber(filter.upper)}`;
		});
		parts.push(`attributes/${attributeParts.join('/')}`);
	}

	if (search.sort !== null) {
		const [field, descending] = search.sort;
		parts.push(`sort/${field.toLowerCase()}/${descending ? 'desc' : 'asc'}`);
	}

	if (search.contractType === 'item_exchange') {
		parts.push('item-exchange');
	} else if (search.contractType === 'auction') {
		parts.push('auction');
	}

	if (search.price !== null) {
		const [lower, upper] = search.price;
		parts.push(
			upper === null
				? `contract-price/${lower.toFixed(2)}`
				: `contract-price/${lower.toFixed(2)}-${upper.toFixed(2)}`
		);
	}
	if (search.value !== null) {
		const [lower, upper] = search.value;
		parts.push(
			upper === null
				? `estimated-value/${lower.toFixed(2)}`
				: `estimated-value/${lower.toFixed(2)}-${upper.toFixed(2)}`
		);
	}

	if (search.noMultiItemContracts) {
		parts.push('no-multi-item-contracts');
	}
	if (search.onlyContracts) {
		parts.push('contracts-only');
	}
	if (search.goldbar) {
		parts.push('goldbar');
	}
	if (search.brownbar) {
		parts.push('brownbar');
	}
	if (search.diamondbar) {
		parts.push('diamondbar');
	}
	if (search.withoutOtherItems) {
		parts.push('without-other-items');
	}
	if (search.withPersonalModules) {
		parts.push('with-personal-modules');
	}
	if (search.inJita) {
		parts.push('in-jita');
	}

	return parts.length === 0 ? `/${prefix}` : `/${prefix}/${parts.join('/')}`;
}
