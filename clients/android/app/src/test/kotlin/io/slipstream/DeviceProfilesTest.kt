package io.slipstream

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DeviceProfilesTest {
    @Test
    fun identifiesOnlyTheFireHd10ThirteenthGen() {
        assertTrue(DeviceProfiles.isFireHd10("KFTUWI"))
        assertTrue(DeviceProfiles.isFireHd10("kftuwi"))
        assertFalse(DeviceProfiles.isFireHd10("KFTRWI"))
        assertFalse(DeviceProfiles.isFireHd10(null))
    }

    @Test
    fun fireHd10UsesA1080pSafe16By10Default() {
        assertEquals(
            Triple(1680, 1050, 60),
            DeviceProfiles.streamDefaultMode("KFTUWI", Triple(1920, 1200, 60)),
        )
    }

    @Test
    fun fireHd10ProfileExposesTheSafeModeAndHardwareDecodeEnvelope() {
        val profile = DeviceProfiles.forModel("KFTUWI", Triple(1920, 1200, 60))
        assertEquals(DeviceProfiles.FIRE_HD_10_13_MODEL, profile?.model)
        assertEquals(Triple(1680, 1050, 60), profile?.recommendedMode)
        assertTrue(profile?.decoderLabel?.contains("1080p60") == true)
    }

    @Test
    fun fireHd10PresetResetsOnlyStreamPerformanceKnobs() {
        val current = Settings(
            width = 2560,
            height = 1440,
            hz = 120,
            renderScale = 1.5,
            lowLatencyMode = false,
            presentPriority = "smooth",
            smoothBuffer = 3,
            audioChannels = 8,
            micEnabled = true,
        )
        val tuned = DeviceProfiles.optimizedSettings(current)
        assertEquals(0, tuned.width)
        assertEquals(0, tuned.height)
        assertEquals(0, tuned.hz)
        assertEquals(1.0, tuned.renderScale, 0.0)
        assertTrue(tuned.lowLatencyMode)
        assertEquals("latency", tuned.presentPriority)
        assertEquals(0, tuned.smoothBuffer)
        assertEquals(2, tuned.audioChannels)
        assertTrue(tuned.micEnabled)
    }

    @Test
    fun fireHd10CapsAnUnexpectedHighRefreshPanelTo60() {
        assertEquals(
            Triple(1680, 1050, 60),
            DeviceProfiles.streamDefaultMode("KFTUWI", Triple(1920, 1200, 120)),
        )
    }

    @Test
    fun explicitSmallerOrNonPanelModesAreNotOverridden() {
        assertEquals(
            Triple(1280, 800, 60),
            DeviceProfiles.streamDefaultMode("KFTUWI", Triple(1280, 800, 60)),
        )
        assertEquals(
            Triple(1920, 1080, 60),
            DeviceProfiles.streamDefaultMode("KFTUWI", Triple(1920, 1080, 60)),
        )
    }

    @Test
    fun otherDevicesKeepTheirNativeMode() {
        val native = Triple(2560, 1440, 144)
        assertEquals(native, DeviceProfiles.streamDefaultMode("Pixel 9", native))
    }

    @Test
    fun explicitFireModeFallsBackOnlyWhenTheDecoderRejectsIt() {
        val requested = Triple(1920, 1200, 60)
        assertEquals(
            requested,
            DeviceProfiles.resolveModeForDecoder("KFTUWI", requested) { it == requested },
        )
        assertEquals(
            Triple(1680, 1050, 60),
            DeviceProfiles.resolveModeForDecoder("KFTUWI", requested) {
                it == Triple(1680, 1050, 60)
            },
        )
    }

    @Test
    fun decoderFallbackPreservesTheFire16By10Shape() {
        val requested = Triple(2560, 1440, 120)
        assertEquals(
            Triple(1920, 1080, 60),
            DeviceProfiles.resolveModeForDecoder("KFTUWI", requested) {
                it == Triple(1920, 1080, 60)
            },
        )
    }
}
