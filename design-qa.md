# Modern Desktop Shell — Design QA

## Evidence

- Dark source visual truth: `/Users/ir/.codex/generated_images/019fdfff-8ce6-7281-8014-89a59d1a563e/exec-c87aebb1-057a-46c0-9d6a-4674e932c9fb.png` (1486 × 1058).
- Bright source visual truth: `/Users/ir/.codex/generated_images/019fdfff-8ce6-7281-8014-89a59d1a563e/exec-d7ba7619-1bca-4616-b0c7-be85fb0cf612.png` (1487 × 1058).
- Dark implementation: `/private/tmp/infekt-dark-implementation-final2.png` (2880 × 1910 at 2× density; 1440 × 955 logical pixels).
- Bright implementation: `/private/tmp/infekt-bright-implementation-final2.png` (2880 × 1910 at 2× density; 1440 × 955 logical pixels).
- Full comparisons: `/private/tmp/infekt-dark-final-comparison.png` and `/private/tmp/infekt-bright-final-comparison.png`.
- Focused comparisons: `/private/tmp/infekt-dark-toolbar-final-comparison.png`, `/private/tmp/infekt-dark-inspector-final-comparison.png`, `/private/tmp/infekt-bright-toolbar-final-comparison.png`, and `/private/tmp/infekt-bright-inspector-final-comparison.png`.

The implementation captures were downsampled to 1440 × 955. Each source was proportionally resized to 1440px wide and cropped at 955px high so the app-owned toolbar, canvas, and inspector align. Iced's capture excludes the native OS title bar; native traffic lights/window controls are therefore intentionally absent from the implementation evidence.

## State and Constraints

- Enhanced view, Presentation inspector open, dark Neon Pasture and bright Cobalt Paper states.
- A temporary UTF-8 NFO was used because the cow and radiation source files are not present in the repository.
- Renderer artwork, ANSI color, Glow, hyperlinks, wrapping, and encoding behavior were excluded from visual acceptance as required. The comparison covers the desktop shell, layout, density, colors, and interaction affordances.

## Findings and Comparison History

### Iteration 1

- **P2 — Toolbar balance and icon semantics.** The first implementation pushed the view selector too far right and used magnifying-glass/download symbols. Fixed with responsive metadata widths, centered presentation controls, minus/plus zoom icons, and the approved upload-style Export symbol.
- **P2 — Inspector density and material.** The first implementation used long color fields, a flat panel wash, and undersized typography. Fixed with two-column color wells, theme-derived glass gradients, larger control labels, and increased vertical rhythm.

### Final comparison

- **Fonts and typography:** hierarchy and optical scale now match the references closely. Cascadia Mono is the cross-platform default instead of SF Mono; the system-monospace selector remains available.
- **Spacing and layout rhythm:** toolbar height, full-height canvas, segmented modes, zoom group, and inspector alignment match. The inspector remains the planned 320px rather than the slightly wider generated reference.
- **Colors and visual tokens:** Neon Pasture and Cobalt Paper reproduce the dark/bright balance, colored selection states, translucent surfaces, borders, and theme-tinted glass wash.
- **Image quality and assets:** all shell icons use the repository's Tabler SVG set. No NFO artwork was recreated or substituted.
- **Copy and content:** toolbar, section labels, Glow terminology, Display Options, and available Properties match the approved product language. Unsupported controls remain session-bound only, and Export is visibly disabled.

No actionable P0, P1, or P2 visual differences remain within the agreed renderer boundary. Residual artwork, exact font, native-titlebar, and metadata differences are expected constraints, not implementation defects.

## Interaction and Verification Notes

- The native app launched successfully in both theme states and loaded a representative NFO.
- Open remains connected to the existing file picker; Enhanced, Classic, and Text Only use the existing views.
- Inspector, overflow, zoom, theme selection, and presentation-only state transitions are covered by focused unit tests.
- Export has no action and is rendered in the disabled state.
- No browser or browser console is involved in this native Iced application.

## About Menu Placement Follow-up

- Reported source capture: `/Users/ir/Desktop/Screenshot 2026-08-08 at 11.28.15.png` (2048 × 402 pixels), showing the empty Neon Pasture state with the inspector and overflow menu open.
- Revised implementation capture: `/private/tmp/infekt-about-anchor-final.png` (1920 × 1328 pixels from a 1200 × 832-point native window at 1.6× display density).
- Focused before/after comparison: `/private/tmp/infekt-about-anchor-comparison.png` (1920 × 762 pixels). The source was proportionally normalized to 1920 × 377; the implementation was cropped to the same top-window region.
- **P2 — Overflow menu detached from its trigger.** The menu was aligned to the full window's right edge, which placed it over the inspector at medium toolbar widths. Fixed by anchoring the interactive overlay to the More button's measured layout bounds, right-aligning their edges, placing the menu below the presentation bar, and clamping it to the viewport.
- The focused post-fix evidence confirms that typography, colors, glass material, copy, and icon treatment are unchanged; only the menu placement changed. No actionable P0, P1, or P2 differences remain in this state.

## Toolbar Metadata and About Glass Follow-up

- User-directed visual overrides: remove the in-toolbar `iNFekt` label, prevent filename metadata from painting over adjacent controls, and strengthen the About dialog surface.
- Toolbar implementation: `/private/tmp/infekt-toolbar-metadata-final.png` (2624 × 1888 capture of a 1200 × 832-point native window at 2× density, including the window shadow). State: Neon Pasture, inspector open, a deliberately long filename, dimensions `57×4`, and UTF-8 encoding.
- Dark About implementation: `/private/tmp/infekt-about-glass-dark-final.png` (2624 × 1888, same native window and density).
- Bright About implementation: `/private/tmp/infekt-about-glass-bright-final.png` (2624 × 1888, same native window and density).
- Full toolbar comparison: `/private/tmp/infekt-toolbar-request-final-comparison.png` (1440 × 2058). The approved dark visual and revised capture were each normalized to 1440 × 1025 and stacked with an 8px separator; the user's new copy/layout requests intentionally override the older toolbar label.
- Focused dark/bright dialog comparison: `/private/tmp/infekt-about-glass-dark-bright-comparison.png` (1928 × 664). Each implementation capture was normalized to 960 × 664 and combined with an 8px separator.
- **P2 — Redundant toolbar identity.** Removed the `iNFekt` label from every responsive toolbar branch while retaining the native window title.
- **P2 — Metadata escaped its layout slot.** The filename previously consumed the fixed-width region before dimensions and charset were reserved, allowing trailing text to paint over the view selector. Fixed by making the filename the sole flexible child, clipping it independently, reserving dimensions/charset as trailing content, and clipping the full metadata region. The widths were rebalanced to 245px and 407px so the view controls retain their visual position after brand removal.
- **P2 — About dialog lacked separation.** The dialog shared the menu's very translucent glass token. Fixed with a dedicated fully opaque, theme-composited gradient, a 1px theme-tinted border, and a deeper modal shadow. The dark state keeps a restrained cobalt-to-cyan glass tint; the bright state remains paper-light and readable.
- **Required fidelity surfaces:** toolbar and dialog typography are unchanged; spacing remains aligned at 1200px; both theme palettes retain their semantic colors; the existing icon and Iced assets remain sharp and unmodified; requested copy was removed only from the toolbar. No new or substituted raster assets were introduced.
- Post-fix captures show the complete view selector and zoom controls unobscured with the long filename, and readable dialog content in both themes. No actionable P0, P1, or P2 differences remain.

## Three-Zone Toolbar Follow-up

- User-directed visual target: left-aligned Open/file metadata, a geometrically centered View Mode + zoom group, and a right-aligned action group ordered Export, More, and inspector toggle.
- Source visual truth: `/Users/ir/.codex/generated_images/019fdfff-8ce6-7281-8014-89a59d1a563e/exec-c87aebb1-057a-46c0-9d6a-4674e932c9fb.png` (1486 × 1058). The user's latest ordering and removal requests intentionally override the older toolbar details in this source.
- Long-name implementation: `/private/tmp/infekt-toolbar-three-zone-1200.png` (2536 × 1800 capture of a 1200 × 832-point native window, including window shadow).
- Minimum-width implementation: `/private/tmp/infekt-toolbar-three-zone-900.png` (2024 × 1488 capture of a 900 × 632-point native window, including window shadow).
- Natural-width filename implementation: `/private/tmp/infekt-toolbar-short-filename-1200.png` (2652 × 1888 capture of a 1200 × 832-point native window, including window shadow).
- Full-view combined evidence: `/private/tmp/infekt-toolbar-three-zone-full-comparison.png` (1440 × 2012). The implementation window frame was cropped, both images were normalized to 1440px wide at 1× comparison density, and equal-height regions were stacked with an 8px separator.
- Focused combined evidence: `/private/tmp/infekt-toolbar-three-zone-focus-comparison.png` (1440 × 468), using matched 230px toolbar crops from the same normalized source and implementation images.
- State: Neon Pasture, Enhanced view, inspector open, disabled Export, first with a deliberately long filename and then `demo.nfo`. The 900-point capture exercises the application's declared minimum width.
- **P2 — Toolbar groups competed for sequential space.** Replaced the responsive sequence with equal left and right flex zones around an intrinsic center zone. The mode selector and zoom controls now remain centered while the requested action cluster stays flush right in the order Export, More, inspector.
- **P2 — Filename width was fixed rather than content-aware.** Added a trailing-first metadata layout: dimensions and charset are reserved, the filename keeps its intrinsic width whenever it fits, and only the filename is clipped when the remaining width is insufficient. The short-name capture confirms `demo.nfo` is followed immediately by `— 65×3 · UTF-8`; the long-name and 900-point captures confirm metadata and central controls remain visible under pressure.
- **Fonts and typography:** existing toolbar font sizes, weights, and no-wrap behavior remain unchanged; the long filename truncates without wrapping or painting into neighboring controls.
- **Spacing and layout rhythm:** the three zones are symmetric around the window center, action spacing is consistent, and both 1200- and 900-point captures retain clear separation between persistent controls.
- **Colors and visual tokens:** Neon Pasture glass, disabled, selected, separator, and text hierarchy tokens remain unchanged.
- **Image quality and assets:** Export, More, inspector, file, and Open continue using the repository's Tabler SVG assets with no raster or handcrafted substitutions.
- **Copy and content:** Export is always labeled; dimensions and charset remain directly adjacent to the filename; no toolbar identity label was reintroduced.
- Post-fix combined and responsive evidence shows no actionable P0, P1, or P2 difference. The source's older brand/action details are expected user-directed overrides.

## NFO-Derived Ambient Backdrop Follow-up

- User-provided validation file: `/Users/ir/Downloads/xrel-movie-3235328.nfo` (`80×220`, CP 437), used unchanged in every implementation capture.
- Dark implementation: `/private/tmp/infekt-backdrop-dark-v1.png` (3104 × 2198 capture; alpha-cropped native window 2880 × 1974, or 1440 × 987 logical pixels).
- Bright implementation: `/private/tmp/infekt-backdrop-bright-v1.png` (same dimensions and native state, using Cobalt Paper).
- Mode and inspector evidence: `/private/tmp/infekt-backdrop-classic-v1.png` and `/private/tmp/infekt-backdrop-text-collapsed-v1.png`.
- Minimum-width evidence: `/private/tmp/infekt-backdrop-dark-900-v1.png` (2024 × 1488 capture of the 900-point minimum-width window with the inspector open).
- Full combined comparisons: `/private/tmp/infekt-backdrop-qa-dark-combined.png` and `/private/tmp/infekt-backdrop-qa-bright-combined.png`. Each source and alpha-cropped implementation window was proportionally normalized to 1440px wide and stacked with an 8px separator.
- Focused combined comparisons: `/private/tmp/infekt-backdrop-qa-dark-focused.png` and `/private/tmp/infekt-backdrop-qa-bright-focused.png`, placing the approved and implemented ambient edge/inspector regions in the same image at equal width.
- The approved source artwork is not present in the repository, so backdrop QA compares the requested depth treatment, layer ordering, theme response, fixed positioning, and inspector/toolbar transmission—not identical blurred contours or colors. The real validation file supplies representative tall block art and ordinary text.
- **P2 — Shell had no NFO-derived depth layer.** Added one fixed 640 × 400 theme-colored NFO raster beneath the complete toolbar/content shell. It is cropped to visible content, fitted with padding, blurred at sigma 28, displayed with cover plus 1.08× overscan, and restrained by the specified dark/bright canvas, toolbar, and inspector scrims.
- **Renderer boundary:** the foreground geometry remains the existing renderer output. Only the Enhanced view's viewport-wide opaque fill was removed so the shell scrim can reveal the ambient image; decoding, grid creation, ANSI interpretation, links, wrapping, Glow, and export are unchanged.
- **Lifecycle and stability:** the cache key includes only grid identity, backdrop colors, and the rounded character ratio. New file/theme/ratio state clears stale pixels immediately; cancellation preserves the current image; failed loads clear it; stale asynchronous results are rejected. Zoom, Glow, view mode, scrolling, font size, and inspector state do not regenerate it.
- **Theme fidelity:** Neon Pasture exposes the deeper cyan ambient silhouette through the dark canvas and accent-tinted inspector. Cobalt Paper uses the intentionally quieter bright-mode opacity while retaining a visible blue haze. Empty state continues to use the original opaque shell gradient.
- **Responsive and interaction evidence:** Enhanced, Classic, and Text Only retain the same backdrop; collapsing the inspector expands the crisp viewer without replacing the image; the 900-point capture preserves toolbar controls, artwork, scrolling area, and inspector. The About layer remains above the complete backdrop/shell stack.
- **Required fidelity surfaces:** typography, spacing, toolbar grouping, theme colors, existing Tabler assets, metadata copy, and foreground NFO sharpness remain unchanged. No actionable P0, P1, or P2 shell/backdrop differences remain within the approved renderer boundary.

## Inspector-Local Ambient Backdrop Follow-up

- User-directed correction: keep the direct NFO canvas and toolbar free of ambient artwork while retaining an NFO-derived glass effect in the Presentation inspector.
- Bright source visual truth: `/Users/ir/.codex/generated_images/019fdfff-8ce6-7281-8014-89a59d1a563e/exec-d7ba7619-1bca-4616-b0c7-be85fb0cf612.png` (1487 × 1058).
- Dark source visual truth: `/Users/ir/.codex/generated_images/019fdfff-8ce6-7281-8014-89a59d1a563e/exec-c87aebb1-057a-46c0-9d6a-4674e932c9fb.png` (1486 × 1058).
- Bright implementation: `/private/tmp/infekt-inspector-local-bright-final.png` (2880 × 1974 at 2× density; 1440 × 987 logical pixels, vertically limited by the available desktop work area).
- Dark implementation: `/private/tmp/infekt-inspector-local-dark-final.png` (same dimensions and validation state, using Neon Pasture).
- Full combined comparisons: `/private/tmp/infekt-inspector-local-bright-comparison.png` and `/private/tmp/infekt-inspector-local-dark-comparison.png`. Both source and implementation captures were normalized to 1440px wide, top-aligned, and cropped to the shared 987px comparison height.
- Focused same-input comparisons: `/private/tmp/infekt-inspector-local-bright-focused.png` and `/private/tmp/infekt-inspector-local-dark-focused.png`, pairing the complete inspector regions at equal displayed width.
- State: Enhanced mode, inspector open, the user-supplied `/Users/ir/Downloads/xrel-movie-3235328.nfo` loaded unchanged, Cobalt Paper and Neon Pasture themes.
- **P2 — Ambient artwork competed with the foreground canvas.** Removed the root-level backdrop image and the translucent toolbar/canvas roles. The canvas and toolbar now use their normal opaque theme surfaces in every loaded state, so only the crisp renderer owns the viewer.
- **P2 — A centered full-window image could expose an empty sliver behind the inspector.** The generated 640 × 400 raster now computes the horizontal center of mass of eligible post-preamble block and text ink. Block geometry, shade opacity, text-mark area, and theme alpha contribute to the focal weight; the crop remains clamped inside its 24px padded bounds.
- **Inspector material:** one fixed image is aspect-filled with 1.10× overscan directly beneath the 320px inspector. It remains stationary while the inspector content scrolls. The overlay keeps the specified 0.64 dark and 0.82 bright scrims, with a neutral leading glass stop and a restrained theme tint at the far edge.
- **Tiling decision:** horizontal repeat and mirror-repeat were evaluated but not used. The reference reads as one broad, non-periodic material wash; repeating recognizable NFO structure would introduce seams and wallpaper rhythm. The focal aspect-fill crop provides complete inspector coverage without those artifacts.
- **Theme and visual fidelity:** Cobalt Paper retains a clean paper canvas and a subtle blue inspector wash; Neon Pasture retains a black canvas and a more visible cyan/teal glass atmosphere. Toolbar grouping, typography, controls, Tabler assets, metadata copy, and foreground NFO sharpness remain unchanged.
- **Renderer boundary and lifecycle:** no core, decoding, ANSI, hyperlink, wrapping, Glow, export, foreground geometry, zoom, or scrolling behavior changed. The existing cache inputs and stale-result protection remain intact; only the backdrop algorithm version changed for the new focal crop.

No actionable P0, P1, or P2 shell/backdrop differences remain for the corrected inspector-local direction.

## Seamless Gutters and Opaque NFO Paper Follow-up

- User-directed correction: reveal one continuous ambient backdrop under the toolbar, both viewer gutters, and the complete Presentation inspector, while keeping only the rendered NFO bounds plus 24px padding fully opaque.
- Validation file: `/Users/ir/Downloads/xrel-movie-3235328.nfo` (`80×220`, CP 437), loaded unchanged.
- Bright Enhanced implementation: `/private/tmp/infekt-seamless-paper-bright-final-v4.png` (1440 × 987 native window capture).
- Dark Enhanced implementation: `/private/tmp/infekt-seamless-paper-dark-final.png` (1440 × 987 native window capture).
- Bright Classic implementation: `/private/tmp/infekt-seamless-paper-classic-final.png` (1440 × 987 native window capture).
- Full same-input comparisons: `/private/tmp/infekt-seamless-paper-bright-comparison.png` and `/private/tmp/infekt-seamless-paper-dark-comparison.png`.
- Focused gutter/inspector comparisons: `/private/tmp/infekt-seamless-paper-bright-focused.png` and `/private/tmp/infekt-seamless-paper-dark-focused.png`.
- **P2 — Intrinsic NFO surface was initially left-aligned.** Replaced the ordinary container composition with a dedicated `NfoPaper` widget. It reports the renderer's intrinsic dimensions plus equal 24px padding, centers itself only when it fits, and preserves the left scroll origin when the paper is wider than the viewport. Enhanced, Classic, and Text Only share this behavior.
- **P2 — A single center-weighted cover image disappeared beneath the opaque paper.** The fixed 640×400 raster now uses two 320×400 cover-composed halves from the same eligible NFO content, with the second half mirrored. The complete field is blurred after composition, so both outer viewer gutters retain recognizable color energy without a hard center seam or wallpaper edge.
- **Layer continuity:** one root image sits below the toolbar and complete content row. Viewer and toolbar scrims, the inspector glass, and the opaque NFO paper are all foreground layers over that same image; there is no inspector-local copy, independent crop, or phase shift. The field remains fixed while the NFO and inspector scroll.
- **Paper isolation:** the exact selected NFO background color is drawn at alpha 1 only behind intrinsic foreground content plus padding. The surrounding viewer is translucent, so short and narrow files reveal both side gutters and the area below the paper; large files retain normal bidirectional scrolling.
- **Bright/dark calibration:** Neon Pasture keeps the specified stronger dark ambience. Cobalt Paper uses 0.70 image transmission and a 0.80 viewer scrim so the repeated wash remains visible in exposed gutters while the paper stays clean white; the toolbar remains quieter and the inspector retains its stronger glass material.
- **Fidelity and assets:** typography, toolbar zones, control copy, theme wells, Tabler icons, foreground geometry, and native window treatment remain unchanged. The source and implementation use different NFO artwork, so the comparison judges opacity boundaries, spatial continuity, atmosphere, and shell hierarchy rather than matching blurred contours.
- **Renderer boundary:** no decoding, grid generation, ANSI, hyperlinks, wrapping, Glow, export, or foreground rendering semantics changed. Backdrop generation remains fixed-size, asynchronous, cache-keyed, and stale-result safe.
- The final bright, dark, and Classic captures confirm equal side gutters, an opaque shrink-wrapped document surface, seamless continuation beneath the inspector to its bottom edge, and subtle backdrop transmission through the toolbar. No actionable P0, P1, or P2 differences remain for this corrected composition.

final result: passed
