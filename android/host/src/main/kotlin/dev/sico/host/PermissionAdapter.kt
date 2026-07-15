package dev.sico.host

import android.content.Context
import android.content.pm.PackageManager

internal object PermissionAdapter {
    fun allGrantedNow(context: Context, runtimePermissions: List<String>): Boolean =
        runtimePermissions.all { permission ->
            context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED
        }

    fun requireKnownManifestPermission(permission: String) {
        require(permission == "android.permission.INTERNET")
    }
}
