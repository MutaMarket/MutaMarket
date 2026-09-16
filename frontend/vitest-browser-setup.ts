// Browser tests render components on their own, without +layout.svelte, so
// nothing pulls in the stylesheet. Unstyled controls then collapse to a few
// pixels and a real click misses them entirely.
import './src/routes/layout.css';
