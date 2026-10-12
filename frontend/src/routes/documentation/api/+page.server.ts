import type { PageServerLoad } from './$types';
import { apiGet } from '#lib/server/api.ts';
import type { DocNavSection } from '#lib/docs.ts';
import type { OpenApiDocument } from '#lib/openapi.ts';

interface DocumentationNav {
  sections: DocNavSection[];
}

export const load: PageServerLoad = async ({ fetch }) => {
  const [nav, spec] = await Promise.all([
    apiGet<DocumentationNav>(fetch, '/api/documentation'),
    apiGet<OpenApiDocument>(fetch, '/api/openapi.json'),
  ]);

  return { sections: nav.sections, spec };
};
