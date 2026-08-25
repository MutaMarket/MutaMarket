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
}

export interface CollectionCardData {
	id: number;
	slug: string;
	name: string;
	description: string | null;
	visibility: string;
	character_name: string;
	modules_count: number;
}

export interface CollectionPageData {
	collection: CollectionCardData;
	modules: ModuleDetail[];
}
