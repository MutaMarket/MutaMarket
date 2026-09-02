<!-- The login page: EVE SSO is the only way in. -->
<script lang="ts">
  import PageMeta from '$lib/components/page-meta.svelte';
</script>

<PageMeta
  title="Login"
  description="Login to MutaMarket and start trading abyssal modules today!"
/>

<section class="login">
  <h1>Log in</h1>
  <p>MutaMarket uses EVE Online's single sign-on. No separate account needed.</p>
  <a
    href="/eve"
    rel="external"
    class="mt-4 inline-block rounded-md border border-border bg-card-1 px-4 py-2 text-sm transition-colors hover:bg-card-2"
  >
    Log in with EVE Online
  </a>
  <p class="mt-3 text-xs text-muted-foreground">
    <a href="/eve?without_scopes=true" rel="external" class="underline">
      Log in without granting any scopes
    </a>
  </p>
</section>
