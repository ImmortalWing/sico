# Android native UI adapter v0 review

> - status: accepted-cross-check
> - date: 2026-07-16
> - phase: M6

## Result

The shared `UiModel` validator remains authoritative for node, depth, text, accessibility, event-rate and queue limits. The Android adapter maps only validated nodes to `LinearLayout`, `TextView`, `Button` and `EditText`; it never creates a WebView or interprets guest text as markup.

Native click and committed IME input return through the shared `EventGate`. UTF-8 input is capped at 4 KiB, accepted events at 120 per rolling second and queued events at 256. Interactive focus order follows the validated accessibility order and Kotlin assigns the same order for TalkBack traversal.

The Rust adapter and both Android target checks are verified. Android SDK compilation, TalkBack inspection and touch/IME execution remain unavailable without a licensed Android runner, so this is cross-check evidence rather than native runtime evidence.
