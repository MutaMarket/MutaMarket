// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
  namespace App {
    // interface Error {}
    interface Locals {
      /** The request's UI locale (see $lib/server/locale). */
      locale: import('#lib/i18n/messages.js').Locale;
    }
    interface PageData {
      locale?: import('#lib/i18n/messages.js').Locale;
      /** The shared props of every page (see $lib/server/shared-props). */
      nav?: import('#lib/types.js').NavState | null;
      sidebar?: import('#lib/sidebar.js').SidebarPayload | null;
      workbench?: import('#lib/workbench.js').WorkbenchEntry[] | null;
      sentOffers?: import('#lib/types-offers.js').SentOffer[] | null;
    }
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
