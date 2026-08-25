// ISK/number formatting, ported from the legacy Helper/FormatNumber.ts.
// Deliberately pinned to en-US in every locale: en-US-grouped ISK is the
// convention across EVE tooling, and URL building elsewhere strips ","
// grouping. UI copy translates the prose around numbers, never the
// number format itself.

const intlCompactLong = new Intl.NumberFormat('en-US', {
	notation: 'compact',
	compactDisplay: 'long'
});

const intlCompactShort = new Intl.NumberFormat('en-US', {
	notation: 'compact',
	compactDisplay: 'short'
});

const intlCurrency = new Intl.NumberFormat('en-US', {
	style: 'currency',
	currency: 'ISK'
});

/** "142 million ISK"; null (no estimate) reads "N/A". */
export function toIskCompact(value: number | null, showCurrency = true): string {
	if (value === null) {
		return 'N/A';
	}
	return showCurrency ? `${intlCompactLong.format(value)} ISK` : intlCompactLong.format(value);
}

/** "1.2K" — the short compact form of inputs and badges. */
export function toVeryCompact(value: number): string {
	return intlCompactShort.format(value);
}

/** "142 million" without the currency suffix. */
export function toCompact(value: number): string {
	return intlCompactLong.format(value);
}

/** "ISK 1,235" full currency form. */
export function toIsk(value: number, showCurrency = true): string {
	return showCurrency
		? intlCurrency.format(value)
		: intlCurrency.format(value).replace('ISK', '');
}
