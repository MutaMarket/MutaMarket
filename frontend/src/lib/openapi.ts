// Reading the generated OpenAPI document into what the reference page
// renders. Pure, so the shaping rules are unit-testable and the page stays
// presentation only.

export interface OpenApiDocument {
  info: { title: string; version: string; description?: string };
  servers?: { url: string }[];
  tags?: { name: string; description?: string }[];
  paths: Record<string, Record<string, RawOperation>>;
  components?: { schemas?: Record<string, Schema> };
}

interface RawOperation {
  tags?: string[];
  summary?: string;
  description?: string;
  operationId?: string;
  parameters?: RawParameter[];
  requestBody?: { content?: Record<string, MediaType> };
  responses?: Record<string, RawResponse>;
}

interface RawParameter {
  name: string;
  in: string;
  required?: boolean;
  description?: string;
  schema?: Schema;
}

interface RawResponse {
  description?: string;
  content?: Record<string, MediaType>;
}

interface MediaType {
  schema?: Schema;
  example?: unknown;
}

export interface Schema {
  type?: string | string[];
  format?: string;
  description?: string;
  $ref?: string;
  items?: Schema;
  oneOf?: Schema[];
  properties?: Record<string, Schema>;
  required?: string[];
  additionalProperties?: Schema | boolean;
  nullable?: boolean;
}

export interface Parameter {
  name: string;
  location: string;
  required: boolean;
  description: string;
  type: string;
}

export interface ResponseEntry {
  status: string;
  description: string;
  /** What to show, e.g. `ModulePage` or `EstimatorStatistic[]`. */
  schema: string | null;
  /** The component to link to. An array response names its item type,
   * which is the schema a reader actually wants. */
  schemaRef: string | null;
  example: unknown;
}

export interface Operation {
  /** Stable anchor: `get-modules-query`. */
  id: string;
  method: string;
  path: string;
  tag: string;
  summary: string;
  description: string;
  parameters: Parameter[];
  requestBody: string | null;
  responses: ResponseEntry[];
}

/** The component a schema names, following an array to its items. */
export function componentName(schema: Schema | undefined): string | null {
  if (!schema) return null;
  return refName(schema) ?? refName(schema.items);
}

export interface TagGroup {
  name: string;
  description: string;
  operations: Operation[];
}

/** Methods in the order a reader expects them, not the map's order. */
const METHOD_ORDER = ['get', 'post', 'put', 'patch', 'delete'];

/** The component name behind a `$ref`, or null for an inline schema. */
export function refName(schema: Schema | undefined): string | null {
  const ref = schema?.$ref;
  if (!ref) return null;
  return ref.split('/').pop() ?? null;
}

/** A short type label for a parameter or property. */
export function typeLabel(schema: Schema | undefined): string {
  if (!schema) return 'any';
  const named = refName(schema);
  if (named) return named;
  if (schema.oneOf) {
    return schema.oneOf.map((entry) => typeLabel(entry)).join(' or ');
  }
  if (schema.items) return `${typeLabel(schema.items)}[]`;

  const type = Array.isArray(schema.type)
    ? schema.type.filter((entry) => entry !== 'null')
    : schema.type
      ? [schema.type]
      : [];
  const label = type.length > 0 ? type.join(' or ') : 'any';
  const nullable =
    schema.nullable === true || (Array.isArray(schema.type) && schema.type.includes('null'));
  return nullable ? `${label} or null` : label;
}

/** The anchor for an operation, stable across regenerations. */
export function operationId(method: string, path: string): string {
  const slug = path
    .replace(/[{}]/g, '')
    .replace(/[^a-zA-Z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  return `${method}-${slug}`.toLowerCase();
}

/** The document as tag groups of operations, ready to render. */
export function groupOperations(document: OpenApiDocument): TagGroup[] {
  const groups = new Map<string, TagGroup>();
  for (const tag of document.tags ?? []) {
    groups.set(tag.name, { name: tag.name, description: tag.description ?? '', operations: [] });
  }

  const paths = Object.entries(document.paths ?? {}).sort(([a], [b]) => a.localeCompare(b));
  for (const [path, item] of paths) {
    const methods = Object.entries(item).sort(
      ([a], [b]) => METHOD_ORDER.indexOf(a) - METHOD_ORDER.indexOf(b),
    );
    for (const [method, raw] of methods) {
      const tag = raw.tags?.[0] ?? 'Other';
      if (!groups.has(tag)) {
        groups.set(tag, { name: tag, description: '', operations: [] });
      }
      groups.get(tag)!.operations.push(toOperation(method, path, raw));
    }
  }

  return [...groups.values()].filter((group) => group.operations.length > 0);
}

function toOperation(method: string, path: string, raw: RawOperation): Operation {
  const json = (content: Record<string, MediaType> | undefined) => content?.['application/json'];

  return {
    id: operationId(method, path),
    method: method.toUpperCase(),
    path,
    tag: raw.tags?.[0] ?? 'Other',
    summary: raw.summary ?? '',
    description: raw.description ?? '',
    parameters: (raw.parameters ?? []).map((parameter) => ({
      name: parameter.name,
      location: parameter.in,
      required: parameter.required ?? false,
      description: parameter.description ?? '',
      type: typeLabel(parameter.schema),
    })),
    requestBody: refName(json(raw.requestBody?.content)?.schema),
    responses: Object.entries(raw.responses ?? {})
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([status, response]) => {
        const media = json(response.content);
        return {
          status,
          description: response.description ?? '',
          schema: refName(media?.schema) ?? typeLabelOrNull(media?.schema),
          schemaRef: componentName(media?.schema),
          example: media?.example ?? null,
        };
      }),
  };
}

function typeLabelOrNull(schema: Schema | undefined): string | null {
  return schema ? typeLabel(schema) : null;
}

/** Every schema an operation reaches, so the page can show only what is
 * actually referenced rather than the whole component list. */
export function reachableSchemas(
  document: OpenApiDocument,
  names: string[],
  seen = new Set<string>(),
): string[] {
  const schemas = document.components?.schemas ?? {};
  for (const name of names) {
    if (seen.has(name) || !schemas[name]) continue;
    seen.add(name);
    reachableSchemas(document, nestedRefs(schemas[name]), seen);
  }
  return [...seen];
}

function nestedRefs(schema: Schema): string[] {
  const found: string[] = [];
  const walk = (node: Schema | boolean | undefined) => {
    if (!node || typeof node === 'boolean') return;
    const named = refName(node);
    if (named) found.push(named);
    if (node.items) walk(node.items);
    node.oneOf?.forEach(walk);
    walk(node.additionalProperties);
    for (const property of Object.values(node.properties ?? {})) walk(property);
  };
  walk(schema);
  return found;
}
