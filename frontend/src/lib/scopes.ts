// ESI access per character: what the site was granted, what it still
// needs, and the re-authorization link that closes the gap.
import type { AccountCharacter } from '$lib/types';

export interface ScopeInfo {
	id: string;
	label: string;
	description: string;
	/** Granted through its own flow; never counted as missing. */
	optional: boolean;
}

/** The scopes a normal login asks for, so a character holding none of
 * them is missing all of them. */
export function requiredScopes(catalogue: ScopeInfo[]): ScopeInfo[] {
	return catalogue.filter((scope) => !scope.optional);
}

export function missingScopes(character: AccountCharacter, catalogue: ScopeInfo[]): ScopeInfo[] {
	return requiredScopes(catalogue).filter((scope) => !character.granted_scopes.includes(scope.id));
}

/** A character warns while it lacks required scopes and the user has not
 * silenced it. */
export function warnsAboutScopes(character: AccountCharacter, catalogue: ScopeInfo[]): boolean {
	return !character.scope_warnings_muted && missingScopes(character, catalogue).length > 0;
}

/** Characters whose warnings are still showing. */
export function charactersNeedingScopes(
	characters: AccountCharacter[],
	catalogue: ScopeInfo[],
): AccountCharacter[] {
	return characters.filter((character) => warnsAboutScopes(character, catalogue));
}

/** Re-authorization asks for everything the character already granted
 * plus what is missing, so the new token carries the full set rather
 * than replacing a broad token with a narrow one. */
export function grantUrl(character: AccountCharacter, catalogue: ScopeInfo[]): string {
	const wanted = new Set([
		...character.granted_scopes,
		...requiredScopes(catalogue).map((scope) => scope.id),
	]);
	const scopes = [...wanted].join(' ');
	return `/eve?add_to_account=true&scopes=${encodeURIComponent(scopes)}`;
}
