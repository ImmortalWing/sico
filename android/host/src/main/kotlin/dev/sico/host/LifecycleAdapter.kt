package dev.sico.host

import android.os.Bundle

internal data class GuestDescriptor(val appIdentity: String, val revisionDigest: String)

internal object LifecycleAdapter {
    private const val APP_IDENTITY = "sico.app_identity"
    private const val REVISION_DIGEST = "sico.revision_digest"

    fun save(descriptor: GuestDescriptor, state: Bundle) {
        state.putString(APP_IDENTITY, descriptor.appIdentity)
        state.putString(REVISION_DIGEST, descriptor.revisionDigest)
    }

    fun restore(state: Bundle?): GuestDescriptor? {
        val app = state?.getString(APP_IDENTITY) ?: return null
        val revision = state.getString(REVISION_DIGEST) ?: return null
        return GuestDescriptor(app, revision)
    }

    // Restored descriptors must be sent through shared-core open/reverify.
    fun canAssumeGuestRunningAfterRestore(): Boolean = false
}
