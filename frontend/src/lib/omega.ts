// The omega sale-stacking calculator's packages and math, mirroring the
// legacy OmegaCalculatorPage.vue script block: hard-coded USD PLEX
// packages and NES omega packages, the MarkeeDragon 3% code applied
// after the sale discount, and the five comparison scenarios.

/** A PLEX package with its regular EVE Store price in USD. */
export interface PlexPackage {
  plex: number;
  basePrice: number;
  label: string;
}

/** The legacy plexPackages list, largest first (the page's default). */
export const PLEX_PACKAGES: PlexPackage[] = [
  { plex: 20000, basePrice: 650, label: '20,000 PLEX' },
  { plex: 12000, basePrice: 420, label: '12,000 PLEX' },
  { plex: 6000, basePrice: 240, label: '6,000 PLEX' },
  { plex: 3000, basePrice: 125, label: '3,000 PLEX' },
  { plex: 1500, basePrice: 65, label: '1,500 PLEX' },
  { plex: 1000, basePrice: 45, label: '1,000 PLEX' },
];

/** A New Eden Store omega package priced in PLEX. */
export interface OmegaPackage {
  months: number;
  regularPlex: number;
  /** The NES discount slider's ceiling for this package. */
  maxDiscount: number;
}

/** The legacy omegaPackages list. */
export const OMEGA_PACKAGES: OmegaPackage[] = [
  { months: 24, regularPlex: 6600, maxDiscount: 25 },
  { months: 12, regularPlex: 3600, maxDiscount: 25 },
];

/** The affiliate code's price multiplier: 3% off, applied after the
 * sale discount. */
export const MARKEEDRAGON_MULTIPLIER = 0.97;

/** Sale discount first, then the MarkeeDragon 3% off the discounted
 * price. */
export function discountedPlexPrice(
  pkg: PlexPackage,
  plexDiscount: number,
  useMarkeedragon: boolean,
): number {
  let price = pkg.basePrice * (1 - plexDiscount / 100);
  if (useMarkeedragon) {
    price = price * MARKEEDRAGON_MULTIPLIER;
  }
  return price;
}

/** The combined discount readout, one decimal ("22.4"). */
export function effectiveTotalDiscount(
  pkg: PlexPackage,
  plexDiscount: number,
  useMarkeedragon: boolean,
): string {
  const finalPrice = discountedPlexPrice(pkg, plexDiscount, useMarkeedragon);
  return ((1 - finalPrice / pkg.basePrice) * 100).toFixed(1);
}

/** The omega package's PLEX price under the NES sale, rounded like the
 * legacy Math.round. */
export function discountedOmegaPlex(pkg: OmegaPackage, nesDiscount: number): number {
  return Math.round(pkg.regularPlex * (1 - nesDiscount / 100));
}

/** Whole months of omega the PLEX package redeems at the discounted
 * NES rate. */
export function omegaMonthsAffordable(
  plexPkg: PlexPackage,
  omegaPkg: OmegaPackage,
  nesDiscount: number,
): number {
  const discounted = discountedOmegaPlex(omegaPkg, nesDiscount);
  if (discounted === 0) return 0;
  const plexPerMonth = discounted / omegaPkg.months;
  return Math.floor(plexPkg.plex / plexPerMonth);
}

/** USD per month of omega under the full stack; 0 when nothing is
 * affordable (the legacy guard). */
export function costPerMonth(
  plexPkg: PlexPackage,
  omegaPkg: OmegaPackage,
  plexDiscount: number,
  useMarkeedragon: boolean,
  nesDiscount: number,
): number {
  const months = omegaMonthsAffordable(plexPkg, omegaPkg, nesDiscount);
  if (months === 0) return 0;
  return discountedPlexPrice(plexPkg, plexDiscount, useMarkeedragon) / months;
}

/** Months the same package buys at regular prices. */
export function regularOmegaMonths(plexPkg: PlexPackage, omegaPkg: OmegaPackage): number {
  const regularPlexPerMonth = omegaPkg.regularPlex / omegaPkg.months;
  return Math.floor(plexPkg.plex / regularPlexPerMonth);
}

/** USD per month at regular prices. */
export function regularCostPerMonth(plexPkg: PlexPackage, omegaPkg: OmegaPackage): number {
  const months = regularOmegaMonths(plexPkg, omegaPkg);
  if (months === 0) return 0;
  return plexPkg.basePrice / months;
}

/** One comparison-table row's inputs. */
export interface Scenario {
  name: string;
  plexDiscount: number;
  markeedragon: boolean;
  nesDiscount: number;
  omegaMonths: number;
  isFullStack?: boolean;
}

/** The five comparison scenarios for the current slider/checkbox state.
 * Legacy quirk, ported: "PLEX + MarkeeDragon" always applies the code,
 * only "Full Stack" honors the checkbox. */
export function scenarios(
  plexDiscount: number,
  useMarkeedragon: boolean,
  nesDiscount: number,
  omegaMonths: number,
): Scenario[] {
  return [
    {
      name: 'No Sales (Baseline)',
      plexDiscount: 0,
      markeedragon: false,
      nesDiscount: 0,
      omegaMonths,
    },
    {
      name: `PLEX Sale Only (${plexDiscount}%)`,
      plexDiscount,
      markeedragon: false,
      nesDiscount: 0,
      omegaMonths,
    },
    {
      name: 'PLEX + MarkeeDragon',
      plexDiscount,
      markeedragon: true,
      nesDiscount: 0,
      omegaMonths,
    },
    {
      name: `NES Sale Only (${nesDiscount}%)`,
      plexDiscount: 0,
      markeedragon: false,
      nesDiscount,
      omegaMonths,
    },
    {
      name: 'Full Stack',
      plexDiscount,
      markeedragon: useMarkeedragon,
      nesDiscount,
      omegaMonths,
      isFullStack: true,
    },
  ];
}

/** One computed comparison-table row, formatted like the legacy
 * calculateScenario (money as toFixed strings). */
export interface ScenarioResult {
  plexCost: string;
  months: number;
  costPerMonth: string;
  moneySaved: string;
  extraMonths: number;
  savingsPct: string;
}

export function calculateScenario(plexPkg: PlexPackage, scenario: Scenario): ScenarioResult {
  const omegaPkg = OMEGA_PACKAGES.find((pkg) => pkg.months === scenario.omegaMonths)!;

  const plexCost = discountedPlexPrice(plexPkg, scenario.plexDiscount, scenario.markeedragon);
  const months = omegaMonthsAffordable(plexPkg, omegaPkg, scenario.nesDiscount);
  const regularMonths = regularOmegaMonths(plexPkg, omegaPkg);

  const perMonth = plexCost / months;
  const moneySaved = plexPkg.basePrice - plexCost;
  const extraMonths = months - regularMonths;
  const savingsPct = ((moneySaved / plexPkg.basePrice) * 100).toFixed(1);

  return {
    plexCost: plexCost.toFixed(2),
    months,
    costPerMonth: perMonth.toFixed(2),
    moneySaved: moneySaved.toFixed(2),
    extraMonths,
    savingsPct,
  };
}
