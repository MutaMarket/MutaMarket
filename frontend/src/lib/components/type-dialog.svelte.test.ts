import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-svelte";

import TypeDialog from "./type-dialog.svelte";
import { abyssalSlug } from "$lib/abyssals";
import { parseQueryUi } from "$lib/query";

/** The Stasis Webifier entry (a single-variant catalog entry) and the
 * small 5MN Microwarpdrive (a variant of a grouped entry). */
const WEBIFIER = 47_702;
const SMALL_MWD = 47_781;

async function openDialog(availableTypes: number[] | null) {
  const screen = await render(TypeDialog, {
    prefix: "personal/modules",
    search: parseQueryUi(""),
    availableTypes,
  });
  await screen.getByRole("button").click();
  return screen;
}

function link(typeId: number): HTMLAnchorElement {
  const found = document.body.querySelector<HTMLAnchorElement>(
    `a[href="/personal/modules/type/${abyssalSlug(typeId)}"]`,
  );
  expect(found, `link for type ${typeId}`).not.toBeNull();
  return found as HTMLAnchorElement;
}

describe("type dialog availability", () => {
  it("dims the types the page has none of, in both entry shapes", async () => {
    await openDialog([WEBIFIER]);

    expect(link(WEBIFIER).dataset.missing).toBe("false");
    expect(link(SMALL_MWD).dataset.missing).toBe("true");
  });

  it("keeps a dimmed type selectable", async () => {
    await openDialog([WEBIFIER]);

    expect(link(SMALL_MWD).getAttribute("href")).toBe(
      `/personal/modules/type/${abyssalSlug(SMALL_MWD)}`,
    );
  });

  it("dims nothing when the page browses everything", async () => {
    await openDialog(null);

    expect(link(WEBIFIER).dataset.missing).toBe("false");
    expect(link(SMALL_MWD).dataset.missing).toBe("false");
    expect(
      document.body.querySelectorAll('[data-missing="true"]'),
    ).toHaveLength(0);
  });
});
