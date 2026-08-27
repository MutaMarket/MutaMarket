// The legacy TypeDialog.vue catalog: three columns of sections, with the
// exact EVE type ids the legacy links (single-variant entries use the
// icon id as the type id, like getTypeLink(icon_id)). Icons are asset
// stems: a type id or a named image under /img/icons.

export interface CatalogEntry {
	icon: string;
	name: string;
	variants: [string, number][];
}

export interface CatalogSection {
	title: string;
	entries: CatalogEntry[];
}

function entry(icon: string, name: string): CatalogEntry {
	return { icon, name, variants: [] };
}

/** The icon stem of the catalog entry covering a type id (the entry's
 * own id or one of its variants), for showing the dialog's icon in the
 * trigger too. */
export function iconForType(typeId: number): string | null {
	for (const column of CATALOG) {
		for (const section of column) {
			for (const entry of section.entries) {
				if (Number(entry.icon) === typeId) return entry.icon;
				if (entry.variants.some(([, id]) => id === typeId)) return entry.icon;
			}
		}
	}
	return null;
}

export const CATALOG: CatalogSection[][] = [
	[
		{
			title: 'Electronic Warfare',
			entries: [
				entry('47702', 'Stasis Webifier'),
				entry('47732', 'Warp Scrambler'),
				entry('47736', 'Warp Disruptor'),
				entry('56303', 'Heavy Warp Scrambler'),
				entry('56304', 'Heavy Warp Disruptor')
			]
		},
		{
			title: 'Weapon Upgrades',
			entries: [
				entry('49722', 'Magnetic Field Stabilizer'),
				entry('49726', 'Heat Sink'),
				entry('49730', 'Gyrostabilizer'),
				entry('49734', 'Entropic Radiation Sink'),
				entry('49738', 'Ballistic Control System'),
				entry('60482', 'Drone Damage Amplifier'),
				entry('56313', 'Siege Module'),
				entry('78621', 'Vorton Tuning System'),
				entry('60483', 'Fighter Support Unit')
			]
		},
		{
			title: 'Mining Lasers',
			entries: [
				entry('90460', 'Mining Laser'),
				entry('90483', 'Deep Core Mining Laser'),
				entry('90474', 'Modulated Deep Core Miner')
			]
		},
		{
			title: 'Strip Miners',
			entries: [
				entry('90493', 'Strip Miner'),
				entry('90498', 'Deep Core Strip Miner'),
				entry('90467', 'Modulated Strip Miner'),
				entry('90487', 'Modulated Deep Core Strip Miner')
			]
		}
	],
	[
		{
			title: 'Shield',
			entries: [
				{
					icon: '47781',
					name: 'Shield Booster',
					variants: [
						['Small', 47781],
						['Medium', 47785],
						['Large', 47789],
						['X-Large', 47793],
						['Capital', 56309]
					]
				},
				{
					icon: '47836',
					name: 'Ancillary Shield Booster',
					variants: [
						['Medium', 47836],
						['Large', 47838],
						['X-Large', 47840],
						['Capital', 56310]
					]
				},
				{
					icon: '47800',
					name: 'Shield Extender',
					variants: [
						['Small', 47800],
						['Medium', 47804],
						['Large', 47808]
					]
				}
			]
		},
		{
			title: 'Armor',
			entries: [
				{
					icon: '47769',
					name: 'Armor Repairer',
					variants: [
						['Small', 47769],
						['Medium', 47773],
						['Large', 47777],
						['Capital', 56307]
					]
				},
				{
					icon: '47842',
					name: 'Ancillary Armor Repairer',
					variants: [
						['Small', 47842],
						['Medium', 47844],
						['Large', 47846],
						['Capital', 56308]
					]
				},
				{
					icon: '47812',
					name: 'Armor Plates',
					variants: [
						['Small', 47812],
						['Medium', 47817],
						['Large', 47820]
					]
				}
			]
		},
		{
			title: 'Propulsion',
			entries: [
				{
					icon: '47749',
					name: 'Afterburner',
					variants: [
						['1mn', 47749],
						['10mn', 47753],
						['100mn', 47757],
						['10000mn', 56305]
					]
				},
				{
					icon: '47408',
					name: 'Microwarpdrive',
					variants: [
						['5mn', 47740],
						['50mn', 47408],
						['500mn', 47745],
						['50000mn', 56306]
					]
				}
			]
		},
		{
			title: 'Ice Mining',
			entries: [entry('90502', 'Ice Mining Laser'), entry('90524', 'Ice Harvester')]
		},
		{
			title: 'Gas Harvesting',
			entries: [entry('90529', 'Gas Cloud Scoop'), entry('90593', 'Gas Cloud Harvester')]
		}
	],
	[
		{
			title: 'Engineering',
			entries: [
				{
					icon: '47824',
					name: 'Energy Neutralizer',
					variants: [
						['Small', 47824],
						['Medium', 47828],
						['Heavy', 47832],
						['Capital', 56312]
					]
				},
				{
					icon: '48419',
					name: 'Energy Nosferatu',
					variants: [
						['Small', 48419],
						['Medium', 48423],
						['Heavy', 48427],
						['Capital', 56311]
					]
				},
				{
					icon: '48431',
					name: 'Cap Battery',
					variants: [
						['Small', 48431],
						['Medium', 48435],
						['Large', 48439]
					]
				}
			]
		},
		{
			title: 'Miscellaneous',
			entries: [
				{
					icon: '52227',
					name: 'Damage Control',
					variants: [
						['Regular', 52227],
						['Assault', 52230]
					]
				},
				{
					icon: 'SmartbombEM',
					name: 'EMP Smartbombs',
					variants: [
						['Small', 84442],
						['Medium', 84438],
						['Large', 84434]
					]
				},
				{
					icon: 'SmartbombKin',
					name: 'Graviton Smartbombs',
					variants: [
						['Small', 84444],
						['Medium', 84440],
						['Large', 84436]
					]
				},
				{
					icon: 'SmartbombThermal',
					name: 'Plasma Smartbombs',
					variants: [
						['Small', 84443],
						['Medium', 84439],
						['Large', 84435]
					]
				},
				{
					icon: 'SmartbombExplo',
					name: 'Proton Smartbombs',
					variants: [
						['Small', 84445],
						['Medium', 84441],
						['Large', 84437]
					]
				},
				{
					icon: '60479',
					name: 'Drones',
					variants: [
						['Light', 60478],
						['Medium', 60479],
						['Heavy', 60480],
						['Sentry', 60481]
					]
				}
			]
		},
		{
			title: 'Mining Drones',
			entries: [
				entry('90614', 'Mining Drone'),
				entry('90618', 'Ice Harvesting Drone'),
				entry('90621', "'Excavator' Mining Drone"),
				entry('90622', "'Excavator' Ice Harvesting Drone")
			]
		}
	]
];
