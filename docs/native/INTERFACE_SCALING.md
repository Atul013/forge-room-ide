# Native interface sizing

Atul's PR #87 adds saved whole-interface sizing in Settings and the command palette, separately from editor font sizing. Automatic preserves the existing startup behavior. A chosen 80–200% size represents physical pixels per layout point; the app derives egui zoom from the current Windows scale. An out-of-range saved value is clamped; nonfinite/missing values use Automatic.

The startup view caps a saved choice to the current monitor's 760 × 480 point layout minimum without rewriting the preference. Settings distinguishes the applied fitted size from the saved choice. Size adjustments use the pixels-per-point actually displayed, so Smaller cannot enlarge a capped interface and Bigger cannot shrink one already above the supported cap.

egui/winit 0.37 reports monitor dimensions in already-zoomed egui points. Recover physical dimensions using the complete rendered pixels-per-point, including app zoom; multiplying only by Windows' native scale makes the cap change as the user zooms. The integration corrects this conversion and tests its invariance across native scales and app zoom factors.

The current monitor is sampled at startup and when the user changes interface size. This is not automatic multi-monitor relocation acceptance. Source math, persistence, command registration, format/check/tests/Clippy and the Windows package/installer/headless gates pass at integrated runtime `b9cfa57`. Visible trials at 100/150/200%, screen-fit limits and real monitor/DPI changes remain open because native computer-use access is unavailable.

No trust, provider, Room, workflow or editor-content permissions are changed by interface sizing. Editor text commands retain their existing behavior.
