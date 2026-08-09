package io.slipstream.design

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp
import io.slipstream.AuroraBase
import io.slipstream.SlipstreamViolet
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.sin

// The Slipstream design system — the aurora/glass language the console UI invented, extracted so
// the WHOLE app (touch and console alike) shares one look. Keep this file dependency-light: it is
// imported by every rebuilt screen and by the CI screenshot scenes, so it must render under the
// Robolectric JVM renderer (no RenderEffect/haze here — those stay in the screens that can gate
// on API level).

/** One drifting colour blob of the aurora field. Integer [sx]/[sy] keep the loop seamless at wrap. */
private class AuroraBlob(
    val color: Color,
    val baseX: Float,
    val baseY: Float,
    val driftX: Float,
    val driftY: Float,
    val sx: Int,
    val sy: Int,
    val phase: Float,
    val radiusFrac: Float,
    val alpha: Float,
)

// The brand field: violet, deep indigo, plum, and one brand-cyan blob so the logo's colour is
// present in the ambience — not just the accents.
private val auroraBlobs = listOf(
    AuroraBlob(Color(0xFF877AF5), 0.30f, 0.26f, 0.16f, 0.10f, 1, 1, 0.0f, 0.62f, 0.55f),
    AuroraBlob(Color(0xFF3E33B8), 0.78f, 0.68f, 0.13f, 0.14f, 1, 2, 2.4f, 0.68f, 0.58f),
    AuroraBlob(Color(0xFF9E4CCC), 0.16f, 0.82f, 0.12f, 0.09f, 2, 1, 4.1f, 0.52f, 0.42f),
    AuroraBlob(Color(0xFF1F7FA8), 0.72f, 0.14f, 0.10f, 0.08f, 1, 3, 1.2f, 0.48f, 0.36f),
)

/**
 * The living Slipstream backdrop: soft brand-family blobs drifting over [AuroraBase] on slow,
 * seamless loops, finished with a centre-pooling vignette and top/bottom legibility scrims. This
 * is the SAME field the console UI uses (GamepadChrome), lifted here so the touch home and the
 * settings surface can wear it too — one ambience across the whole app.
 *
 * [animated] lets the CI screenshot renderer freeze the field (an infinite transition under the
 * JVM renderer never settles); interactive screens always use the animated field.
 */
@Composable
fun AuroraBackdrop(
    modifier: Modifier = Modifier,
    animated: Boolean = true,
    scrim: Boolean = true,
) {
    val sweep = if (animated) {
        val transition = rememberInfiniteTransition(label = "aurora")
        val angle by transition.animateFloat(
            initialValue = 0f,
            targetValue = (2 * PI).toFloat(),
            animationSpec = infiniteRepeatable(tween(96_000, easing = LinearEasing), RepeatMode.Restart),
            label = "angle",
        )
        angle
    } else {
        1.0f // a pleasing fixed phase for static renders
    }
    Canvas(modifier) {
        drawRect(AuroraBase)
        val span = max(size.width, size.height)
        for (b in auroraBlobs) {
            val cx = (b.baseX + b.driftX * sin(sweep * b.sx + b.phase)) * size.width
            val cy = (b.baseY + b.driftY * cos(sweep * b.sy + b.phase)) * size.height
            val r = span * b.radiusFrac
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(b.color.copy(alpha = b.alpha), Color.Transparent),
                    center = Offset(cx, cy),
                    radius = r,
                ),
                center = Offset(cx, cy),
                radius = r,
                blendMode = BlendMode.Plus,
            )
        }
        // Cinematic vignette: pool light centre, sink the corners.
        drawRect(
            Brush.radialGradient(
                colors = listOf(Color.Transparent, Color.Black.copy(alpha = 0.44f)),
                center = Offset(size.width / 2, size.height / 2),
                radius = span * 0.92f,
            ),
        )
        if (scrim) {
            drawRect(
                Brush.verticalGradient(
                    0.0f to Color.Black.copy(alpha = 0.40f),
                    0.30f to Color.Black.copy(alpha = 0.05f),
                    0.70f to Color.Black.copy(alpha = 0.06f),
                    1.0f to Color.Black.copy(alpha = 0.42f),
                ),
            )
        }
    }
}

/** The shared glass corner radius. */
val GlassShape = RoundedCornerShape(22.dp)
val GlassShapeSmall = RoundedCornerShape(14.dp)

/**
 * The standard glass fill every Slipstream surface wears: a translucent white wash with a hairline
 * highlight border. [tintAlpha] lets a selected/engaged surface lean violet. This is the material
 * of host cards, settings groups, dialogs and the in-stream panel. Applied to a Modifier AFTER any
 * layout/sizing, BEFORE padding.
 */
fun Modifier.glassSurface(
    shape: Shape = GlassShape,
    tint: Color = SlipstreamViolet,
    tintAlpha: Float = 0.0f,
    borderAlpha: Float = 0.14f,
): Modifier = this
    .clip(shape)
    .background(
        Brush.verticalGradient(
            listOf(
                Color.White.copy(alpha = 0.09f + tintAlpha * 0.4f),
                tint.copy(alpha = tintAlpha * 0.30f),
                Color.White.copy(alpha = 0.03f),
            ),
        ),
    )
    .border(1.dp, Color.White.copy(alpha = borderAlpha), shape)

/**
 * Animated press/focus scale for interactive glass. Returns the current scale to apply via
 * graphicsLayer — springs on press-release so taps feel physical.
 */
@Composable
fun animateGlassPress(pressed: Boolean): Float = animateFloatAsState(
    targetValue = if (pressed) 0.97f else 1f,
    animationSpec = spring(dampingRatio = 0.6f, stiffness = Spring.StiffnessMedium),
    label = "glassPress",
).value

/** Shared "breathing" progress phase for loading states (a soft pulsing ring). */
@Composable
fun rememberBreath(): Float {
    val t = rememberInfiniteTransition(label = "breath")
    val v by t.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(1800, easing = LinearEasing), RepeatMode.Reverse),
        label = "breathVal",
    )
    return v
}
