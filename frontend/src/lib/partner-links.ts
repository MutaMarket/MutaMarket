// The deployment's partner links, read through SvelteKit's public env
// so a fork points at its own affiliate accounts (or none). Empty means
// the corresponding card, button or coupon strip stays hidden.
import { env } from '$env/dynamic/public';

/** The Ko-fi page behind the sidebar card. */
export const KOFI_LINK = env.PUBLIC_KOFI_LINK ?? '';

/** The Patreon page behind the block-ads card. */
export const PATREON_LINK = env.PUBLIC_PATREON_LINK ?? '';

/** Where every Markee Dragon store button points; the plain store
 * without an affiliate id unless the deployment sets one. */
export const MARKEEDRAGON_URL = env.PUBLIC_MARKEEDRAGON_URL || 'https://store.markeedragon.com/';

/** The checkout code the coupon strip and copy button offer; empty
 * hides both. */
export const MARKEEDRAGON_CODE = env.PUBLIC_MARKEEDRAGON_CODE ?? '';
