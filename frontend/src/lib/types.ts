// Response shapes of the Axum JSON API (src/view in the Rust crate).
// Field names match the serialized snake_case keys exactly; the Rust
// integration tests pin the key sets.

export interface CurrentUser {
	name: string;
	active_character_id: number | null;
	/** Gates the admin navigation and the /api/admin endpoints. */
	is_admin: boolean;
}

export interface AccountCharacter {
	id: number;
	name: string;
	corporation_id: number | null;
	has_asset_token: boolean;
	active: boolean;
}

export interface NavState {
	user: CurrentUser;
	characters: AccountCharacter[];
}

export interface TypeRef {
	id: number;
	name: string;
}

export interface MutaplasmidRef {
	id: number;
	name: string;
}

export interface SourceTypeRef {
	id: number;
	name: string;
	meta_group: string | null;
	meta_group_id: number | null;
	published: boolean;
}

export interface CharacterRef {
	id: number;
	slug: string;
	name: string;
	description: string | null;
	has_premium: boolean;
	corporation_id: number | null;
}

export interface UnitRef {
	id: number;
	name: string;
	display_name: string;
}

export interface ContractRef {
	id: number;
	type: string;
	price: number | null;
	asking_for_items: boolean;
	plex_count: number;
	non_abyssal_modules_count: number;
	abyssal_modules_count: number;
	issuer: CharacterRef | null;
	date_issued: string | null;
	date_expired: string | null;
}

export interface ModuleAttributeView {
	id: number;
	name: string;
	display_name: string;
	value: number;
	base_value: number;
	fraction: number;
	fraction_type: number;
	fraction_absolute: number;
	bar: number;
	is_derived: boolean;
	unit: UnitRef | null;
	is_virtual: boolean;
	/** (min, max) half-width fractions of the type-normalized bar band. */
	type_band: [number, number] | null;
}

export interface ModuleDetail {
	id: number;
	type: TypeRef;
	creator: CharacterRef | null;
	mutated_attributes: ModuleAttributeView[];
	source_type: SourceTypeRef | null;
	mutaplasmid: MutaplasmidRef | null;
	contract: ContractRef | null;
	estimated_value: number | null;
	estimated_value_updated_at: string | null;
	public_asset: unknown;
	slug: string;
	average_fraction: number | null;
}

export interface ModulesStats {
	total_count: number;
	contracts_count: number;
	auctions_count: number;
	item_exchanges_count: number;
	goldbars_count: number;
	diamondbars_count: number;
	brownbars_count: number;
	added_last_hour_count: number;
	added_last_day_count: number;
	added_last_week_count: number;
}

export interface FilterAttribute {
	attribute_id: number;
	name: string;
	display_name: string;
	unit_name: string | null;
	unit_display_name: string | null;
	high_is_good: boolean;
	best: number;
	worst: number;
}

export interface FilterPanelData {
	type_id: number;
	type_name: string;
	attributes: FilterAttribute[];
}

export interface StationRef {
	id: number;
	name: string;
}

export interface AssetLocationView {
	parent_name: string;
	parent_type_id: number | null;
	parent_slug: string;
	station: StationRef | null;
	location_id: number;
	location_type: string;
	location_flag: string;
	location_index: number;
	corporation_id: number | null;
}

export interface AssetImportView {
	id: number;
	character_id: number;
	status: string;
	step: string;
	assets_count: number;
	assets_corporation_count: number;
	abyssal_modules_count: number;
	abyssal_modules_imported_count: number;
	abyssal_modules_failed_count: number;
	/** Age of the last update; refreshes with every pushed update. */
	updated_seconds_ago: number;
}

export interface PersonalPageData {
	user_id: number;
	has_assets_scope: boolean;
	grant_scope_url: string;
	asset_import: AssetImportView | null;
}

export interface PersonalModuleEntry {
	module: ModuleDetail;
	location: AssetLocationView | null;
}
