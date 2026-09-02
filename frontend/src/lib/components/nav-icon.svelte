<script lang="ts" module>
  // The navigation icon vocabulary, the legacy TNavigationIcon.
  export type NavigationIcon =
    | 'about'
    | 'api'
    | 'calculator'
    | 'chart'
    | 'cog'
    | 'collection'
    | 'contract'
    | 'cubes'
    | 'info'
    | 'location'
    | 'megaphone'
    | 'offer'
    | 'plus'
    | 'shop'
    | 'star'
    | 'users';
</script>

<script lang="ts">
  // The navigation icon map, the legacy Navigation/NavIcon.vue.
  import {
    Calculator,
    ChartNoAxesCombined,
    CircleQuestionMark,
    FileText,
    HandCoins,
    Info,
    Layers,
    MapPin,
    Megaphone,
    Plus,
    Settings,
    ShoppingBag,
    SquareTerminal,
    Star,
    Users,
  } from '@lucide/svelte';

  let { icon, class: className = '' }: { icon: NavigationIcon; class?: string } = $props();

  const iconMap = {
    shop: ShoppingBag,
    contract: FileText,
    plus: Plus,
    users: Users,
    collection: Layers,
    offer: HandCoins,
    cubes: Layers,
    calculator: Calculator,
    chart: ChartNoAxesCombined,
    cog: Settings,
    location: MapPin,
    megaphone: Megaphone,
    about: Info,
    info: CircleQuestionMark,
    api: SquareTerminal,
    star: Star,
  } as const;

  const Icon = $derived(iconMap[icon]);
</script>

<Icon class="size-4 {className}" stroke-width={1.5} />
