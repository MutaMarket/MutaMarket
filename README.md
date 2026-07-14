# MutaMarket Documentation

This repository holds the documentation shown at [mutamarket.com/documentation](https://mutamarket.com/documentation).
Everything under [`docs/`](docs) is rendered directly on the site — improvements are very welcome!

## Contributing

1. Find the page you want to improve in [`docs/`](docs) (or use the "Edit this page on GitHub" link at the
   bottom of any page on [mutamarket.com/documentation](https://mutamarket.com/documentation)).
2. Edit the markdown and open a pull request.
3. Once merged, the site picks up the change automatically within about 15 minutes.

### File conventions

- Files are named `NN-slug.md` — the numeric prefix controls the order in the sidebar, the slug becomes the
  URL (`docs/04-appraisal.md` → `mutamarket.com/documentation/appraisal`).
- Each file starts with a front matter block declaring the sidebar section it belongs to:

  ```
  ---
  section: Modules
  ---
  ```

- The first `# Heading` of each file is the page title shown in the sidebar and browser tab.
- GitHub-flavored markdown is supported (tables, task lists, strikethrough). Raw HTML is stripped when
  rendering, so stick to plain markdown.
- Links to MutaMarket pages should be absolute paths like `[login page](/login)`; external links get opened
  in a new tab automatically.
