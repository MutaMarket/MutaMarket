import type { PageServerLoad } from './$types';
import { apiGet } from '$lib/server/api';

export interface DocNavItem {
  slug: string;
  title: string;
}

export interface DocNavSection {
  title: string;
  pages: DocNavItem[];
}

export interface DocumentationData {
  sections: DocNavSection[];
  slug: string;
  section: string;
  title: string;
  html: string;
  edit_url: string;
  previous: DocNavItem | null;
  next: DocNavItem | null;
}

// The index shows the first page, like the legacy controller default;
// unknown slugs 404 and a doc-load failure 503 through the API statuses.
export const load: PageServerLoad = async ({ fetch, params }) => {
  const path =
    params.page === undefined ? '/api/documentation' : `/api/documentation/${params.page}`;

  return { doc: await apiGet<DocumentationData>(fetch, path) };
};
