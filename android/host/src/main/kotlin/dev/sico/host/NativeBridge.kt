package dev.sico.host

internal object NativeBridge {
    const val MAX_BRIDGE_BYTES: Int = 64 * 1024

    init {
        System.loadLibrary("sico_android_host")
    }

    external fun dispatch(envelope: ByteArray, packageBytes: ByteArray?): ByteArray

    fun checkedDispatch(envelope: ByteArray, packageBytes: ByteArray?): ByteArray {
        require(envelope.size <= MAX_BRIDGE_BYTES) { "bridge envelope exceeds 64 KiB" }
        return dispatch(envelope.copyOf(), packageBytes?.copyOf()).also {
            check(it.size <= MAX_BRIDGE_BYTES) { "bridge response exceeds 64 KiB" }
        }
    }
}
