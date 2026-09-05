/** Invalidation key of the root layout's shared props, so a page can
 * refresh the navigation state in place (the offer thread clearing its
 * unread ping) without matching an API URL behind whatever proxy
 * serves it. Lives outside the server-only modules so pages can import
 * it. */
export const SHARED_PROPS_DEPENDENCY = 'app:shared-props';
