// Response shapes of the Axum JSON API (src/view in the Rust crate).
// Field names match the serialized snake_case keys exactly; the Rust
// integration tests pin the key sets.

export interface CurrentUser {
	name: string;
	active_character_id: number | null;
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
