// Shapes of the documentation payload (src/view/docs.rs).

export interface DocNavItem {
  slug: string;
  title: string;
}

export interface DocNavSection {
  title: string;
  pages: DocNavItem[];
}
