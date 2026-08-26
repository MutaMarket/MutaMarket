// Response shapes of the character and collection page-data endpoints
// (src/view/social.rs in the Rust crate).

import type { ModuleDetail } from './types';

export interface CharacterCardData {
	id: number;
	slug: string;
	name: string;
	description: string | null;
	has_premium: boolean;
	corporation_id: number | null;
	modules_count: number | null;
}

export interface CharacterPageData {
	character: CharacterCardData;
	modules: ModuleDetail[];
	/** Header stats over the character's whole sets. */
	for_sale_count: number;
	created_count: number;
}

export interface CollectionCardData {
	id: number;
	slug: string;
	name: string;
	description: string | null;
	visibility: string;
	character_id: number;
	character_name: string;
	character_has_premium: boolean;
	modules_count: number;
	/** Distinct module types (most frequent first, capped) for the
	 * card's icon strip. */
	type_ids: number[];
	types_count: number;
}

export interface CollectionPageData {
	collection: CollectionCardData;
	modules: ModuleDetail[];
	/** Header stat: the whole collection's estimate sum. */
	estimated_value_total: number;
}
