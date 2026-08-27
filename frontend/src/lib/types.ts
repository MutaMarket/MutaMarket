// Response shapes of the Axum JSON API (src/view in the Rust crate).
// Field names match the serialized snake_case keys exactly; the Rust
// integration tests pin the key sets.

export interface CurrentUser {
	name: string;
	active_character_id: number | null;
	/** Gates the admin navigation and the /api/admin endpoints. */
	is_admin: boolean;
	/** Any of the account's characters has active premium. */
	has_premium: boolean;
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
	public_asset: PublicAssetRef | null;
	slug: string;
	average_fraction: number | null;
	/** Only on similar-sold entries: the confirmed sale. */
	training_module?: TrainingModuleRef;
}

/** The legacy PublicAssetResource: the selling character; the price
 * column is unported schema, so always null for now. */
export interface PublicAssetRef {
	owner: { id: number; name: string };
	price: number | null;
}

export interface ModulesStats {
	total_count: number;
	/** Modules currently for sale (live latest contract). */
	listed_count: number;
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
	/** Bar-only statistic: hidden by the calculator's filters. */
	is_virtual: boolean;
	best: number;
	worst: number;
}

/** A published source type of the panel's abyssal type with its base
 * values: powers the slider pips and the type pickers. */
export interface FilterSourceType {
	id: number;
	name: string;
	meta_group_id: number | null;
	meta_level: number | null;
	attributes: { attribute_id: number; value: number }[];
}

export interface FilterPanelData {
	type_id: number;
	type_name: string;
	attributes: FilterAttribute[];
	source_types: FilterSourceType[];
}

export interface StationRef {
	id: number;
	name: string;
}

/** One row of a module display: the module plus, on the personal page,
 * the owner's asset location. */
export interface DisplayEntry {
	module: ModuleDetail;
	location?: AssetLocationView | null;
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
	/** Header stats over the account's whole owned set. */
	modules_count: number;
	estimated_value_total: number;
}

/** One container row of the sell page's select-modules dialog. */
export interface SellLocation {
	asset_id: number;
	type_id: number;
	name: string;
	type_name: string;
	location_flag: string;
	abyssal_count: number;
	public_asset_id: number | null;
	station_name: string | null;
}

export interface SellPageData {
	character_id: number;
	published_count: number;
	estimated_value_total: number;
}

export interface PersonalModuleEntry {
	module: ModuleDetail;
	location: AssetLocationView | null;
}

/** The type's estimator model sheet (`/api/module-page/{module}`). */
export interface EstimatorStatistic {
	r2: number | null;
	mae: number | null;
	nmae: number | null;
	data_count: number;
	/** Source-type name → training sample count. */
	data_statistics: Record<string, number> | null;
	last_trained_at: string | null;
}

export interface SourceTypeComparison {
	type: {
		id: number;
		name: string;
		meta_group_id: number | null;
		meta_level: number | null;
	};
	attributes: { id: number; value: number }[];
	average_price: number | null;
}

/** The sale a module's roll was confirmed at (legacy
 * TrainingModuleResource), attached to similar-sold entries. */
export interface TrainingModuleRef {
	contract_id: number;
	sold_for: number | null;
	sold_at: string | null;
}

/** An archived contract row of the history tab; the live contract is
 * appended client-side with status `outstanding`. */
export interface HistoricContract extends ContractRef {
	status: string;
	/** Present for admins only. */
	ignore_for_training?: boolean;
}

/** A type's roll extremes per attribute, feeding the search-menu
 * variance bounds (trimmed legacy abyssal_type_statistics row). */
export interface AbyssalTypeStatistic {
	attribute_id: number;
	best: number;
	worst: number;
	high_is_good: boolean;
	is_virtual: boolean;
}

export interface ModulePageData {
	module: ModuleDetail;
	estimator_statistic: EstimatorStatistic | null;
	source_type_comparisons: SourceTypeComparison[];
	historic_contracts: HistoricContract[];
	abyssal_type_statistics: AbyssalTypeStatistic[];
}

/** The per-location page payload (legacy ShowLocationPage props). */
export interface LocationShowData {
	location: {
		id: number;
		type: { id: number | null; name: string | null } | null;
		name: string | null;
		location?: { id: number; type: { name: string } | null; slug: string } | null;
	};
	modules: ModuleDetail[];
	available_types: number[];
	stats: {
		total_count: number;
		total_value: number;
		average_value: number;
		goldbars_count: number;
		brownbars_count: number;
		diamondbars_count: number;
	};
}
