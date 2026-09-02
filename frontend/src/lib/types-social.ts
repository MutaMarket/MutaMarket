// Response shapes of the character and collection page-data endpoints
// (src/view/social.rs in the Rust crate).

import type { ModuleDetail } from './types';

/** One index page, the legacy paginate() resource collection reduced
 * to the cards and the meta the pagination buttons read. */
export interface IndexPage<T> {
  data: T[];
  meta: IndexMeta;
}

export interface IndexMeta {
  current_page: number;
  per_page: number;
  total: number;
  last_page: number;
}

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

export interface StationRef {
  id: number;
  name: string;
  type_id: number | null;
  slug: string;
}

/** One asset-location row of the manage-modules dialog (the trimmed
 * legacy LocationResource; see CharacterLocationView in Rust). */
export interface CharacterLocationView {
  /** The assets.id the collection-location endpoints take. */
  asset_id: number;
  item_id: number;
  name: string | null;
  type_id: number;
  type_name: string | null;
  location_id: number | null;
  station: StationRef | null;
  modules_count: number;
  public_asset_id: number | null;
  corporation_id: number | null;
  slug: string;
}

export interface CollectionPageData {
  collection: CollectionCardData;
  modules: ModuleDetail[];
  /** Header stat: the whole collection's estimate sum. */
  estimated_value_total: number;
  auto_sync: boolean;
  last_synced_at: string | null;
  /** Owner-only (null for other viewers): the auto-sync tracked
   * locations. */
  tracked_locations: CharacterLocationView[] | null;
  /** Owner-only: the collection character's locations holding
   * abyssal modules. */
  locations: CharacterLocationView[] | null;
}
