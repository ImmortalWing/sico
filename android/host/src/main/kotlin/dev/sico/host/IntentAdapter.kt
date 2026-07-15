package dev.sico.host

import android.content.ContentResolver
import android.content.Intent
import android.net.Uri

internal object IntentAdapter {
    const val SAPP_MIME: String = "application/vnd.sico.sapp"
    const val MAX_PACKAGE_BYTES: Int = 64 * 1024 * 1024

    fun copySinglePackage(intent: Intent, resolver: ContentResolver): ByteArray {
        require(intent.action == Intent.ACTION_VIEW || intent.action == Intent.ACTION_SEND)
        require(intent.type == SAPP_MIME)
        val uri: Uri = (intent.data ?: intent.getParcelableExtra(Intent.EXTRA_STREAM))
            ?: error("one content URI is required")
        require(uri.scheme == ContentResolver.SCHEME_CONTENT)
        return resolver.openInputStream(uri).use { input ->
            requireNotNull(input)
            val bytes = input.readNBytes(MAX_PACKAGE_BYTES + 1)
            require(bytes.size <= MAX_PACKAGE_BYTES)
            bytes
        }
    }

    fun isPickerDeepLink(intent: Intent): Boolean =
        intent.action == Intent.ACTION_VIEW && intent.data?.toString() == "sico://open"
}
