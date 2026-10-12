// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
  namespace App {
    // interface Error {}
    interface Locals {
      /** The request's UI locale (see $lib/server/locale). */
      locale: import('#lib/i18n/messages.ts').Locale;
    }
    interface PageData {
      locale?: import('#lib/i18n/messages.ts').Locale;
      /** The shared props of every page (see $lib/server/shared-props). */
      nav?: import('#lib/types.ts').NavState | null;
      sidebar?: import('#lib/sidebar.ts').SidebarPayload | null;
      workbench?: import('#lib/workbench.ts').WorkbenchEntry[] | null;
      sentOffers?: import('#lib/types-offers.ts').SentOffer[] | null;
    }
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
