// The mutation calculator page's pure logic, mirroring the legacy
// MutationProbabilitiyColumns: row search, column sorting (nulls last)
// and the probability/cost display strings.

export interface ProbabilityRow {
	mutaplasmid: { id: number; name: string };
	type: { id: number; name: string };
	probability: number;
	cost: number | null;
	cost_mutaplasmid: number;
	cost_type: number | null;
}

export type CalculatorSortKey = 'type' | 'mutaplasmid' | 'probability' | 'cost';

const intlPercent = new Intl.NumberFormat('en-US', { maximumFractionDigits: 2 });

/** "42.5%"; the legacy FormatNumber.toPercentage. */
export function toPercentage(value: number): string {
	return `${intlPercent.format(value * 100)}%`;
}

/** The legacy "1 in {count}" companion readout. */
export function oneIn(probability: number): string {
	return `1 in ${(1 / probability).toFixed(0)}`;
}

/** Case-insensitive match on the source type or mutaplasmid name, the
 * legacy BaseTable search over the two accessor columns. */
export function filterRows(rows: ProbabilityRow[], needle: string): ProbabilityRow[] {
	const query = needle.trim().toLowerCase();
	if (query === '') {
		return rows;
	}
	return rows.filter(
		(row) =>
			row.type.name.toLowerCase().includes(query) ||
			row.mutaplasmid.name.toLowerCase().includes(query),
	);
}

/** Sorts a copy by the column; unknown costs and impossible rolls sink
 * to the end regardless of direction, like the legacy sortFn. */
export function sortRows(
	rows: ProbabilityRow[],
	key: CalculatorSortKey,
	ascending: boolean,
): ProbabilityRow[] {
	const direction = ascending ? 1 : -1;
	const numeric = (value: number | null) => (value === null || value === 0 ? null : value);
	return [...rows].sort((a, b) => {
		if (key === 'type' || key === 'mutaplasmid') {
			return direction * a[key].name.localeCompare(b[key].name);
		}
		const left = key === 'cost' ? numeric(a.cost) : numeric(a.probability);
		const right = key === 'cost' ? numeric(b.cost) : numeric(b.probability);
		if (left === null && right === null) {
			return 0;
		}
		if (left === null) {
			return 1;
		}
		if (right === null) {
			return -1;
		}
		return direction * (left - right);
	});
}
