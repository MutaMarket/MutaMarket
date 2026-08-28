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

const intlPlain = new Intl.NumberFormat('en-US');

/** Whole grouped millions: "150M", "1,500M" — the donation amounts. */
export function toMillions(value: number): string {
	return `${intlPlain.format(Math.round(value / 1_000_000))}M`;
}

const intlThreeSignificant = new Intl.NumberFormat('en-US', {
	maximumSignificantDigits: 3
});

/** Whole millions at three significant digits: "1.5M" / "1.5" bare. */
export function toMillionsCompact(value: number, withUnit = true): string {
	const millions = Math.max(1, Math.round(value / 1_000_000));
	const formatted = intlThreeSignificant.format(millions);
	return withUnit ? `${formatted}M` : formatted;
}

/** "ISK 1,235" full currency form. */
export function toIsk(value: number, showCurrency = true): string {
	return showCurrency
		? intlCurrency.format(value)
		: intlCurrency.format(value).replace('ISK', '');
}
