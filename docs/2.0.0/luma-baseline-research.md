# Luma Bar baseline research

Checked 31 October 2026. This is a visual and behavior reference, not a source-code dependency or a native-app benchmark.

## Evidence and access

- [Official homepage](https://lumabar.xyz/): retrieved directly over HTTPS and inspected in the in-app browser, including its live dock preview.
- [Developer repository](https://github.com/Acadbek/lumabar): release information and older product description. It explicitly identifies the application as commercial and its source as private.
- Firecrawl CLI 1.26.24 was attempted through `npx firecrawl-cli`. Status reported unauthenticated; scraping was rejected because this connection requires an API key. No account or credential was created. Direct HTTPS and browser inspection were the fallback.
- No Luma installer was downloaded or run. No paid artwork or code was copied into Proton.

## Verified product claims

The current homepage advertises more than 100 widgets. The repository README still describes 85 widgets; retain both dated observations rather than treating one count as authoritative for every release.

The homepage lists a $5.99 one-time lifetime license with updates included, a crossed-out $15.99 comparison price, and a 14-day refund period. These are current marketing claims, not a purchase recommendation.

Documented behavior includes apps and widgets in one dock, top/bottom placement, configurable auto-hide or complete hiding of the Windows taskbar, restoration on close, and yielding to fullscreen content. The repository describes five themes and three sizes. These claims were not independently tested in the native paid app.

The homepage FAQ explicitly describes a Search widget that opens a folder path entered by the user. It also describes pinned/running apps and separate file-oriented widgets. Therefore **search is present**; indexed file-content search coverage, ranking quality, latency and efficiency remain unverified. Profile support was not established by the inspected sources. Do not label profiles absent.

## Visual observations

The live website preview uses one continuous dark glass surface with a quiet border. App artwork occupies compact equal-height slots. Data widgets gain horizontal room for values and short labels while keeping the same baseline. Narrow separators distinguish groups without turning each widget into a filled card.

In the 1,270-pixel browser screenshot, the preview bar was roughly 52 rendered pixels high and app artwork roughly 32 pixels high. These are website-preview observations, **not native Windows dimensions or a prescribed DPI scale**. AI-usage and calendar previews were wider than app slots and used concise data rows. Avoid captions under every app icon.

Official visual references:

- [Glass dock](https://raw.githubusercontent.com/Acadbek/lumabar/main/assets/dock-theme-liquid-glass.webp)
- [Widget examples](https://raw.githubusercontent.com/Acadbek/lumabar/main/assets/widgets-preview.webp)
- [Current Glass showcase](https://lumabar.xyz/assets/look-03-glass-green-ring-1200-B-JwXeOL.webp)

The homepage preview demonstrates visual composition, not native frame pacing, actual resource use or feature correctness. Marketing statements about idle rendering are targets to measure, not evidence of zero lag.

## Apply to Proton in stages

1. First prove one restrained glass dock and a small set of polished widgets. Use original assets, consistent icon sizes, stable geometry and truthful loading/unavailable states.
2. Test application activation and widget panel behavior before adding providers or widgets. Capture native screenshots at target DPI values.
3. Validate profile persistence and instant switching against our existing graph nodes. Treat richer profiles and content search as product goals, not proven competitive advantages.
4. Add real providers one at a time, then measure native performance and taskbar recovery. Keep the existing Windows taskbar until restoration tests pass.

Our knowledge graph already records implemented Work/Home/Presentation profiles and a local indexed search layer. Those are source-backed implementation records; the graph does not prove their native acceptance or superiority over Luma.

No application source changed in this research pass. Remaining unknowns: Luma native behavior, profiles, full search capabilities, performance, licensing details beyond the public claims, and exact native layout metrics.
