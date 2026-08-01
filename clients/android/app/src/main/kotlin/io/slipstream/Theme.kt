package io.slipstream

import android.os.Build
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

// Slipstream brand cyans (from the product logo: #0891B2 / #22D3EE / #A5F3FC on near-black).
// Used as the fallback dark scheme on pre-Android-12 devices; on 12+ we defer to Material You.
// `internal` (not private) so the CI screenshot tests can force the deterministic brand palette —
// Material You dynamic colour has no wallpaper to seed from under the Robolectric JVM renderer.
internal val BrandDark = darkColorScheme(
    primary = Color(0xFF22D3EE),
    onPrimary = Color(0xFF00313A),
    primaryContainer = Color(0xFF0E7490),
    onPrimaryContainer = Color(0xFFCFFAFE),
    secondary = Color(0xFFA5F3FC),
    onSecondary = Color(0xFF083344),
    tertiary = Color(0xFF67E8F9),
    onTertiary = Color(0xFF053543),
    background = Color(0xFF050A0F),
    onBackground = Color(0xFFE2E8F0),
    surface = Color(0xFF0B1218),
    onSurface = Color(0xFFE2E8F0),
    surfaceVariant = Color(0xFF1E2C38),
    onSurfaceVariant = Color(0xFFCBD5E1),
)

/**
 * App theme — always dark (a streaming client reads best on a dark canvas, and the immersive
 * stream view assumes it), but uses **Material You** dynamic colour on Android 12+ so the UI
 * harmonises with the user's wallpaper, falling back to the Slipstream brand cyans below that.
 */
@Composable
fun SlipstreamTheme(content: @Composable () -> Unit) {
    val scheme = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        dynamicDarkColorScheme(LocalContext.current)
    } else {
        BrandDark
    }
    // Geist Sans across the whole type scale — the brand typeface the website and the Apple client
    // already ship (see Type.kt).
    MaterialTheme(colorScheme = scheme, typography = SlipstreamTypography, content = content)
}
