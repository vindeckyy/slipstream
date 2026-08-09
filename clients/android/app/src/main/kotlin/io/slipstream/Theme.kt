package io.slipstream

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// The Slipstream brand palette, unified across the whole app. The console UI's aurora violet
// (the drifting field, the glass tiles, the animated focus rings) is promoted to the ONE look —
// touch and console alike — with the brand cyan kept as the secondary accent. Dark-only: a
// streaming client reads best on a dark canvas, and the immersive stream view assumes it.
//
// `internal` (not private) so the CI screenshot tests can force the deterministic brand palette.

/** Deep-space violet — the aurora's base colour, just off pure black. */
internal val AuroraBase = Color(0xFF08070F)

/** The brand violet the whole UI keys on (aurora blobs, focus rings, engaged switches). */
internal val SlipstreamViolet = Color(0xFF8678F5)

/** The deeper violet of filled/engaged surfaces (the console tile wash). */
internal val SlipstreamVioletDeep = Color(0xFF6656F2)

/** The brand cyan — the logo's colour, kept as the secondary accent. */
internal val SlipstreamCyan = Color(0xFF22D3EE)

/** Live presence green — reads as "up" on any palette. */
internal val SlipstreamPresence = Color(0xFF4ADE80)

internal val BrandDark = darkColorScheme(
    primary = SlipstreamViolet,
    onPrimary = Color(0xFF1B1440),
    primaryContainer = SlipstreamVioletDeep,
    onPrimaryContainer = Color(0xFFE9E6FF),
    secondary = SlipstreamCyan,
    onSecondary = Color(0xFF083344),
    secondaryContainer = Color(0xFF113A46),
    onSecondaryContainer = Color(0xFFCFFAFE),
    tertiary = Color(0xFF9E4CCC),
    onTertiary = Color(0xFF2A0E38),
    tertiaryContainer = Color(0xFF3D1A52),
    onTertiaryContainer = Color(0xFFF3DAFF),
    background = AuroraBase,
    onBackground = Color(0xFFE7E4F2),
    surface = Color(0xFF100E1D),
    onSurface = Color(0xFFE7E4F2),
    surfaceVariant = Color(0xFF1C1930),
    onSurfaceVariant = Color(0xFFBDB8D4),
    surfaceContainerLow = Color(0xFF0D0B18),
    surfaceContainer = Color(0xFF12101F),
    surfaceContainerHigh = Color(0xFF1A1729),
    surfaceContainerHighest = Color(0xFF221E36),
    outline = Color(0xFF3A3552),
    outlineVariant = Color(0xFF2A2640),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF4A1410),
    onErrorContainer = Color(0xFFFFDAD6),
)

/**
 * App theme — always dark, always the Slipstream brand palette. The aurora backdrop and the
 * glass surfaces carry the visual identity, so the scheme is fixed (no Material You): one look
 * on every device, exactly what the product branding promises.
 */
@Composable
fun SlipstreamTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = BrandDark, typography = SlipstreamTypography, content = content)
}
