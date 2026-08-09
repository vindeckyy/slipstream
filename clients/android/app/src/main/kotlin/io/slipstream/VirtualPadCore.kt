package io.slipstream

import android.content.Context
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import io.slipstream.kit.Gamepad
import io.slipstream.kit.NativeBridge
import java.util.concurrent.atomic.AtomicBoolean

// The virtual on-screen gamepad — a full controller the tablet itself becomes. Touch controls are
// declared on their OWN wire pad (a reserved high index the physical-pad router's lowest-free
// assignment never reaches short of sixteen plugged-in pads) and forwarded through the exact same
// NativeBridge seam a physical pad uses — Arrival before input, release-all + Remove on teardown —
// so the host presents one more genuine virtual pad and nothing about the stream changes.

/** The wire pad index the virtual pad owns. Physical pads take lowest-free from 0 upward. */
const val VIRTUAL_PAD_INDEX = 15

/**
 * The virtual pad's session controller: declares the pad, forwards button/axis transitions, and
 * guarantees a clean release-all + Remove on teardown so no held input sticks on the host.
 * Thread-safety: all calls land from the main (Compose) thread; the atomic is for the teardown
 * race (StreamScreen dispose vs. a late pointer-up).
 */
class VirtualPadController(private val handle: Long, private val pad: Int = VIRTUAL_PAD_INDEX) {

    private val attached = AtomicBoolean(false)
    private val held = mutableSetOf<Int>()
    private val axes = IntArray(6)

    /** Declare the pad on the host ([Gamepad.PREF_XBOX360] — the wire is XInput semantics). */
    fun attach() {
        if (handle == 0L) return
        if (attached.compareAndSet(false, true)) {
            NativeBridge.nativeSendGamepadArrival(handle, Gamepad.PREF_XBOX360, pad)
        }
    }

    fun button(bit: Int, down: Boolean) {
        if (!attached.get()) return
        if (down) held += bit else held -= bit
        NativeBridge.nativeSendGamepadButton(handle, bit, down, pad)
    }

    fun axis(id: Int, value: Int) {
        if (!attached.get()) return
        if (axes[id] == value) return
        axes[id] = value
        NativeBridge.nativeSendGamepadAxis(handle, id, value, pad)
    }

    /** Release every held button, zero every axis, and unplug the pad from the host. */
    fun release() {
        if (!attached.compareAndSet(true, false)) return
        held.forEach { NativeBridge.nativeSendGamepadButton(handle, it, false, pad) }
        held.clear()
        for (id in axes.indices) {
            if (axes[id] != 0) {
                axes[id] = 0
                NativeBridge.nativeSendGamepadAxis(handle, id, 0, pad)
            }
        }
        NativeBridge.nativeSendGamepadRemove(handle, pad)
    }
}

/** Touch haptics for the virtual pad — a short tick per press, guarded for devices without a motor. */
class VirtualPadHaptics(context: Context, private val enabled: () -> Boolean) {
    private val vibrator = context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator

    fun tick() = vibrate(10L, 45)
    fun press() = vibrate(18L, 90)

    private fun vibrate(ms: Long, amplitude: Int) {
        if (!enabled()) return
        val v = vibrator ?: return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            v.vibrate(VibrationEffect.createOneShot(ms, amplitude))
        } else {
            @Suppress("DEPRECATION")
            v.vibrate(ms)
        }
    }
}

/** Persisted virtual-pad preferences (`SharedPreferences`, like [SettingsStore]). */
data class VirtualPadConfig(
    /** Master switch: the overlay exists at all (the in-stream toggle still hides it per session). */
    val enabled: Boolean = true,
    /** 0.25..1.0 — how opaque the controls render. */
    val opacity: Float = 0.6f,
    /** 0.7..1.4 — uniform size multiplier for every control. */
    val scale: Float = 1.0f,
    /** Haptic tick on press. */
    val haptics: Boolean = true,
    /** Show the guide (home) button — some games map it to overlays users may not want triggered. */
    val showGuide: Boolean = true,
)

class VirtualPadStore(context: Context) {
    private val prefs =
        context.applicationContext.getSharedPreferences("virtual_pad", Context.MODE_PRIVATE)

    fun load(): VirtualPadConfig = VirtualPadConfig(
        enabled = prefs.getBoolean(K_ENABLED, true),
        opacity = prefs.getFloat(K_OPACITY, 0.6f).coerceIn(0.25f, 1f),
        scale = prefs.getFloat(K_SCALE, 1f).coerceIn(0.7f, 1.4f),
        haptics = prefs.getBoolean(K_HAPTICS, true),
        showGuide = prefs.getBoolean(K_GUIDE, true),
    )

    fun save(c: VirtualPadConfig) {
        prefs.edit()
            .putBoolean(K_ENABLED, c.enabled)
            .putFloat(K_OPACITY, c.opacity.coerceIn(0.25f, 1f))
            .putFloat(K_SCALE, c.scale.coerceIn(0.7f, 1.4f))
            .putBoolean(K_HAPTICS, c.haptics)
            .putBoolean(K_GUIDE, c.showGuide)
            .apply()
    }

    private companion object {
        const val K_ENABLED = "enabled"
        const val K_OPACITY = "opacity"
        const val K_SCALE = "scale"
        const val K_HAPTICS = "haptics"
        const val K_GUIDE = "show_guide"
    }
}

/** −1..1 float → ±32767 wire stick value (the same scale [Gamepad.AxisMapper] uses). */
fun virtualStickValue(v: Float): Int = (v.coerceIn(-1f, 1f) * 32767f).toInt()

/** 0..1 float → 0..255 wire trigger value. */
fun virtualTriggerValue(v: Float): Int = (v.coerceIn(0f, 1f) * 255f).toInt()
