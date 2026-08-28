// The personal contracts page logic, ported from the legacy
// ShowAllPersonalContractsPage.vue computeds: the same contract can
// arrive from up to three sources (public, historic, ESI personal), so
// entries sharing an id are merged into one row before the table.

import type { CharacterRef, ModuleDetail, TypeRef } from './types';

/** One entry of the merged /api/personal/contracts payload. Keys follow
 * the legacy ContractResource whenHas rules: public contracts carry no
 * status, only ESI personal contracts carry acceptor/privacy keys. */
export interface PersonalContractEntry {
	id: number;
	type: string;
	price: number | null;
	asking_for_items: boolean;
	plex_count: number;
	non_abyssal_modules_count: number;
	abyssal_modules_count: number;
	issuer: CharacterRef | null;
	status?: string;
	modules?: ModuleDetail[];
	types?: TypeRef[];
	is_private?: boolean;
	acceptor?: CharacterRef | null;
	acceptor_type?: string | null;
	date_issued: string | null;
	date_expired: string | null;
	date_accepted?: string | null;
	/** Present for admins only. */
	ignore_for_training?: boolean;
}

export interface PersonalContractsPage {
	contracts: PersonalContractEntry[];
	date_start: string;
	date_end: string;
}

export interface MergedContract extends Omit<PersonalContractEntry, 'modules' | 'types'> {
	accepted_by_user: boolean;
	/** Module cards where a source loaded them, bare types otherwise. */
	modules: (ModuleDetail | TypeRef)[];
	status: string;
}

/** A merged row's module list entry is a full card when it has a type. */
export function isModuleCard(entry: ModuleDetail | TypeRef): entry is ModuleDetail {
	return 'type' in entry;
}

/**
 * The legacy merged_contracts computed, quirks included: only the first
 * entry's acceptor decides accepted_by_user, the types fallback never
 * marks modules as found, and a missing status falls to outstanding.
 * Outstanding rows sort first, then newest id.
 */
export function mergeContracts(
	contracts: PersonalContractEntry[],
	characterIds: number[]
): MergedContract[] {
	const byId = new Map<number, PersonalContractEntry[]>();
	for (const contract of contracts) {
		const group = byId.get(contract.id);
		if (group) {
			group.push(contract);
		} else {
			byId.set(contract.id, [contract]);
		}
	}

	return [...byId.values()]
		.map((group) => {
			const first = group[0];
			const combined: MergedContract = {
				...first,
				accepted_by_user: characterIds.some((id) => id === first.acceptor?.id) || false,
				modules: [],
				status: first.status ?? 'outstanding'
			};
			let foundModules = false;
			for (const contract of group) {
				if (contract.is_private) {
					combined.is_private = true;
				}
				if (contract.acceptor) {
					combined.acceptor = contract.acceptor;
					combined.acceptor_type = contract.acceptor_type;
					combined.date_accepted = contract.date_accepted;
				}
				if (!foundModules) {
					if (contract.modules) {
						combined.modules = contract.modules;
						foundModules = true;
					} else if (contract.types) {
						combined.modules = contract.types;
					}
				}
				if (contract.issuer) {
					combined.issuer = contract.issuer;
				}
				if (contract.status) {
					combined.status = contract.status;
				}
				if (!contract.status) {
					combined.status = 'outstanding';
				}
			}
			return combined;
		})
		.toSorted((a, b) => {
			if (a.status === 'outstanding' && b.status !== 'outstanding') {
				return -1;
			}
			if (a.status !== 'outstanding' && b.status === 'outstanding') {
				return 1;
			}
			return b.id - a.id;
		});
}

export interface ContractTotals {
	earnings: number;
	spent: number;
	profit: number;
	outstandingValue: number;
	outstandingCount: number;
}

/** The legacy header computeds: completed sales earn, completed buys
 * spend, outstanding contracts count toward the open value. */
export function contractTotals(merged: MergedContract[]): ContractTotals {
	let earnings = 0;
	let spent = 0;
	let outstandingValue = 0;
	let outstandingCount = 0;
	for (const contract of merged) {
		const price = contract.price ?? 0;
		if (contract.status === 'completed') {
			if (contract.accepted_by_user) {
				spent += price;
			} else {
				earnings += price;
			}
		}
		if (contract.status === 'outstanding') {
			outstandingValue += price;
			outstandingCount += 1;
		}
	}
	return { earnings, spent, profit: earnings - spent, outstandingValue, outstandingCount };
}

/** The BaseTable global filter over the legacy accessor values: issuer
 * and acceptor names plus the module/type names. */
export function matchesSearch(contract: MergedContract, query: string): boolean {
	const needle = query.trim().toLowerCase();
	if (needle === '') {
		return true;
	}
	const haystack = [
		contract.issuer?.name ?? '',
		contract.acceptor?.name ?? '',
		...contract.modules.map((entry) => (isModuleCard(entry) ? entry.type.name : entry.name))
	];
	return haystack.some((value) => value.toLowerCase().includes(needle));
}
