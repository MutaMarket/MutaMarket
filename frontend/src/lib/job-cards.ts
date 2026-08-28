// Per-job presentation of the operations console bento grid: what each
// job's headline metric means, and how much room its card deserves.

export interface JobCardSeries {
	/** Key into a run's recorded `metrics`. */
	key: string;
	label: string;
	color: string;
}

export interface JobCardConfig {
	/** Card heading, the job's plain-language name. */
	title: string;
	/** What the recorded `items` metric counts. */
	itemsLabel: string;
	/** Bento footprint: wide cards span two columns. */
	size: 'wide' | 'standard';
	/** One line on what the job does (the title's tooltip). */
	description: string;
	/** Per-run sub-metric lines (same unit); without them the card
	 * shows the work-per-run columns. */
	series?: JobCardSeries[];
}

export const JOB_CARDS: Record<string, JobCardConfig> = {
	'region-contracts': {
		title: 'Region contracts',
		itemsLabel: 'new contracts',
		size: 'wide',
		description: 'Public contract sweep across all k-space regions',
		series: [
			{ key: 'new', label: 'new', color: '#a3e635' },
			{ key: 'invalidated', label: 'invalidated', color: '#d95926' }
		]
	},
	'character-assets': {
		title: 'Character assets',
		itemsLabel: 'modules imported',
		size: 'wide',
		description: 'Asset sync for characters with the read-assets scope',
		series: [
			{ key: 'found', label: 'found', color: '#22d3ee' },
			{ key: 'imported', label: 'imported', color: '#a3e635' },
			{ key: 'failed', label: 'failed', color: '#d03b3b' }
		]
	},
	'character-contracts': {
		title: 'Character contracts',
		itemsLabel: 'contracts',
		size: 'standard',
		description: 'Personal contract sync per linked character'
	},
	estimates: {
		title: 'Value estimates',
		itemsLabel: 'estimates refreshed',
		size: 'standard',
		description: 'AI price estimates for unvalued modules'
	},
	'auction-bids': {
		title: 'Auction bids',
		itemsLabel: 'auctions refreshed',
		size: 'standard',
		description: 'Bid refresh on live abyssal auctions'
	},
	'market-histories': {
		title: 'Market histories',
		itemsLabel: 'days stored',
		size: 'standard',
		description: 'Daily Forge price sweep over mutaplasmids, sources and PLEX'
	},
	'character-names': {
		title: 'Character names',
		itemsLabel: 'characters named',
		size: 'standard',
		description: 'Name resolution for stub character rows'
	},
	'stale-asset-imports': {
		title: 'Import sweeper',
		itemsLabel: 'imports failed',
		size: 'standard',
		description: 'Marks asset imports that stopped moving'
	},
	structures: {
		title: 'Structures',
		itemsLabel: 'structures resolved',
		size: 'standard',
		description: 'Daily public structure sweep'
	},
	alliances: {
		title: 'Alliances',
		itemsLabel: 'alliances upserted',
		size: 'standard',
		description: 'Daily alliance record sweep from ESI'
	},
	'training-modules': {
		title: 'Training modules',
		itemsLabel: 'modules qualified',
		size: 'standard',
		description: 'Sold-alone modules that qualify as training data'
	},
	'estimator-training': {
		title: 'Estimator training',
		itemsLabel: 'types trained',
		size: 'standard',
		description: 'Weekly random forest training per abyssal type'
	},
	'metric-samples': {
		title: 'Metric samples',
		itemsLabel: 'metrics sampled',
		size: 'standard',
		description: 'Dashboard time-series sampling every five minutes'
	}
};

/** The bento order: heavy movers first. */
export const JOB_CARD_ORDER = [
	'region-contracts',
	'character-assets',
	'character-contracts',
	'estimates',
	'auction-bids',
	'market-histories',
	'character-names',
	'stale-asset-imports',
	'structures',
	'alliances',
	'training-modules',
	'estimator-training',
	'metric-samples'
];

/** A "region 2/70" style progress line yields a live meter fraction. */
export function progressFraction(progress: string | null): number | null {
	if (progress === null) return null;
	const match = /(\d+)\/(\d+)/.exec(progress);
	if (!match) return null;
	const total = Number(match[2]);
	return total > 0 ? Math.min(Number(match[1]) / total, 1) : null;
}
