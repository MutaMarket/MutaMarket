// Bookmark route helpers, the legacy Sidebar/Bookmarks.vue priority
// table and Bookmark.vue icon mapping.
import {
  Archive,
  BookmarkIcon,
  Calculator,
  ChartNoAxesCombined,
  Clock,
  Crown,
  FileText,
  FlaskConical,
  FolderOpen,
  Heart,
  CircleQuestionMark,
  History,
  LayoutGrid,
  MapPin,
  MessageSquare,
  Package,
  Settings,
  Tag,
  User,
} from '@lucide/svelte';

/** The legacy route_priority table: bookmark list order by category. */
const ROUTE_PRIORITY: [string, number][] = [
  ['/modules', 0],
  ['/all-modules', 1],
  ['/characters', 2],
  ['/collections', 3],
  ['/locations', 4],
  ['/calculator', 5],
  ['/workbench', 6],
  ['/sell', 7],
  ['/personal/modules', 8],
  ['/personal/contracts', 9],
  ['/offers', 10],
  ['/historic-sales', 11],
  ['/statistics', 12],
  ['/donations', 13],
  ['/settings', 14],
  ['/premium', 15],
  ['/omega-calculator', 16],
  ['/documentation', 17],
];

export function routePriority(path: string): number {
  for (const [route, priority] of ROUTE_PRIORITY) {
    if (path.startsWith(route)) {
      return priority;
    }
  }
  return 99;
}

export interface BookmarkEntry {
  id: number;
  name: string;
  query: string;
  type_id: number | null;
}

/** The legacy sorted_bookmarks: category priority, then name. */
export function sortBookmarks(bookmarks: BookmarkEntry[]): BookmarkEntry[] {
  return [...bookmarks].sort((a, b) => {
    const priority = routePriority(a.query) - routePriority(b.query);
    return priority !== 0 ? priority : a.name.localeCompare(b.name);
  });
}

/** The legacy Bookmark.vue route icon. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function routeIcon(path: string): any {
  if (path.startsWith('/all-modules')) return LayoutGrid;
  if (path.startsWith('/modules') || path === '/') return Package;
  if (path.startsWith('/characters')) return User;
  if (path.startsWith('/collections')) return FolderOpen;
  if (path.startsWith('/calculator')) return Calculator;
  if (path.startsWith('/workbench')) return FlaskConical;
  if (path.startsWith('/sell')) return Tag;
  if (path.startsWith('/personal/modules')) return Archive;
  if (path.startsWith('/locations')) return MapPin;
  if (path.startsWith('/historic-sales')) return History;
  if (path.startsWith('/statistics')) return ChartNoAxesCombined;
  if (path.startsWith('/offers')) return MessageSquare;
  if (path.startsWith('/personal/contracts') || path.startsWith('/contracts')) return FileText;
  if (path.startsWith('/settings')) return Settings;
  if (path.startsWith('/donations')) return Heart;
  if (path.startsWith('/documentation')) return CircleQuestionMark;
  if (path.startsWith('/premium')) return Crown;
  if (path.startsWith('/omega-calculator')) return Clock;
  return BookmarkIcon;
}
