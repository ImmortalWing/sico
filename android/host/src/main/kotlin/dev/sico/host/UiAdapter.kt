package dev.sico.host

import android.content.Context
import android.text.InputFilter
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView

internal enum class NativeWidgetKind {
    VERTICAL_LAYOUT,
    HORIZONTAL_LAYOUT,
    TEXT_VIEW,
    BUTTON,
    EDIT_TEXT,
}

internal data class NativeWidgetSpec(
    val nodeId: String,
    val kind: NativeWidgetKind,
    val text: String?,
    val contentDescription: String?,
    val maxInputBytes: Int?,
)

internal object UiAdapter {
    const val MAX_INPUT_BYTES: Int = 4 * 1024

    fun createView(context: Context, spec: NativeWidgetSpec): View {
        val view = when (spec.kind) {
            NativeWidgetKind.VERTICAL_LAYOUT -> LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
            }
            NativeWidgetKind.HORIZONTAL_LAYOUT -> LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
            }
            NativeWidgetKind.TEXT_VIEW -> TextView(context).apply { text = spec.text.orEmpty() }
            NativeWidgetKind.BUTTON -> Button(context).apply { text = spec.text.orEmpty() }
            NativeWidgetKind.EDIT_TEXT -> EditText(context).apply {
                filters = arrayOf(Utf8ByteLimitFilter(spec.maxInputBytes ?: MAX_INPUT_BYTES))
            }
        }
        view.id = View.generateViewId()
        view.tag = spec.nodeId
        view.contentDescription = spec.contentDescription
        return view
    }

    fun applyTalkBackOrder(orderedViews: List<View>) {
        orderedViews.zipWithNext().forEach { (previous, current) ->
            current.accessibilityTraversalAfter = previous.id
        }
    }

    fun requireBoundedInput(value: String) {
        require(value.toByteArray(Charsets.UTF_8).size <= MAX_INPUT_BYTES)
    }
}

private class Utf8ByteLimitFilter(private val maxBytes: Int) : InputFilter {
    override fun filter(
        source: CharSequence,
        start: Int,
        end: Int,
        dest: android.text.Spanned,
        dstart: Int,
        dend: Int,
    ): CharSequence? {
        val candidate = buildString {
            append(dest.subSequence(0, dstart))
            append(source.subSequence(start, end))
            append(dest.subSequence(dend, dest.length))
        }
        return if (candidate.toByteArray(Charsets.UTF_8).size <= maxBytes) null else ""
    }
}
