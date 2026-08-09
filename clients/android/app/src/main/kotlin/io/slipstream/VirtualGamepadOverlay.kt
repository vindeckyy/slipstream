package io.slipstream

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.slipstream.kit.Gamepad
import kotlin.math.hypot
import kotlin.math.min

// The virtual on-screen gamepad overlay — a touch-native controller for tablets and phones without
// a physical pad. Landscape-first layout (the streaming orientation): left cluster = D-pad + left
// stick, right cluster = face buttons + right stick, top corners = shoulders + triggers, centre =
// Back / Guide / Start. Every control consumes only ITS OWN touch area; the space between controls
// belongs to the stream's gesture layer underneath.

private val PadGlass = Color.White.copy(alpha = 0.10f)
private val PadGlassActive = Color.White.copy(alpha = 0.30f)
private val PadBorder = Color.White.copy(alpha = 0.22f)
private val PadText = Color.White.copy(alpha = 0.92f)

/**
 * The overlay. [controller] forwards wire events; [visible] animates the whole pad in/out (the
 * in-stream quick panel toggles it). [config] carries opacity/size/haptics. The overlay is inert
 * while hidden — no hit-testing at all.
 */
@Composable
fun VirtualGamepadOverlay(
    controller: VirtualPadController,
    haptics: VirtualPadHaptics,
    config: VirtualPadConfig,
    visible: Boolean,
) {
    AnimatedVisibility(
        visible = visible,
        enter = fadeIn() + scaleIn(initialScale = 0.96f),
        exit = fadeOut() + scaleOut(targetScale = 0.96f),
    ) {
        val alpha = config.opacity
        val s = config.scale
        Box(Modifier.fillMaxSize().graphicsLayer { this.alpha = alpha }) {
            // ---- Shoulders + triggers (top corners) ------------------------------------------
            Row(
                Modifier.fillMaxSize().padding(horizontal = (18 * s).dp, vertical = (10 * s).dp),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Column(horizontalAlignment = Alignment.Start, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    TriggerControl("LT", scale = s) { controller.axis(Gamepad.AXIS_LT, it) }
                    ShoulderButton("LB", scale = s, onDown = { controller.button(Gamepad.BTN_LB, true); haptics.press() },
                        onUp = { controller.button(Gamepad.BTN_LB, false) })
                }
                Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    TriggerControl("RT", scale = s) { controller.axis(Gamepad.AXIS_RT, it) }
                    ShoulderButton("RB", scale = s, onDown = { controller.button(Gamepad.BTN_RB, true); haptics.press() },
                        onUp = { controller.button(Gamepad.BTN_RB, false) })
                }
            }

            // ---- Left cluster: D-pad + left stick ---------------------------------------------
            Column(
                Modifier.align(Alignment.BottomStart)
                    .padding(start = (22 * s).dp, bottom = (20 * s).dp),
                verticalArrangement = Arrangement.spacedBy((16 * s).dp),
            ) {
                DpadControl(scale = s, onDirection = { bits, down ->
                    bits.forEach { controller.button(it, down) }
                    if (down) haptics.tick()
                })
                AnalogStick(
                    label = "LS", scale = s,
                    onMove = { x, y ->
                        controller.axis(Gamepad.AXIS_LS_X, virtualStickValue(x))
                        controller.axis(Gamepad.AXIS_LS_Y, virtualStickValue(-y)) // wire +y = up
                    },
                    onClick = { down -> controller.button(Gamepad.BTN_LS_CLICK, down) },
                )
            }

            // ---- Right cluster: right stick + face buttons ------------------------------------
            Column(
                Modifier.align(Alignment.BottomEnd)
                    .padding(end = (22 * s).dp, bottom = (20 * s).dp),
                verticalArrangement = Arrangement.spacedBy((16 * s).dp),
                horizontalAlignment = Alignment.End,
            ) {
                FaceButtons(scale = s, onPress = { bit, down ->
                    controller.button(bit, down)
                    if (down) haptics.press()
                })
                AnalogStick(
                    label = "RS", scale = s,
                    onMove = { x, y ->
                        controller.axis(Gamepad.AXIS_RS_X, virtualStickValue(x))
                        controller.axis(Gamepad.AXIS_RS_Y, virtualStickValue(-y))
                    },
                    onClick = { down -> controller.button(Gamepad.BTN_RS_CLICK, down) },
                )
            }

            // ---- Centre cluster: Back / Guide / Start -----------------------------------------
            Row(
                Modifier.align(Alignment.BottomCenter).padding(bottom = (14 * s).dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy((14 * s).dp),
            ) {
                SmallButton("⧉") { down -> controller.button(Gamepad.BTN_BACK, down) }
                if (config.showGuide) {
                    SmallButton("⌂", accent = true) { down -> controller.button(Gamepad.BTN_GUIDE, down) }
                }
                SmallButton("≡") { down -> controller.button(Gamepad.BTN_START, down) }
            }
        }
    }
}

// ---- Controls ---------------------------------------------------------------------------------

/**
 * An analog stick: a glass well with a draggable knob. The knob tracks the finger within the well
 * radius, emits −1..1 on both axes (screen +y = down; the caller negates Y for the wire), and
 * springs to centre on release. A quick tap-without-drag counts as a stick CLICK (L3/R3).
 */
@Composable
private fun AnalogStick(
    label: String,
    scale: Float,
    onMove: (x: Float, y: Float) -> Unit,
    onClick: (down: Boolean) -> Unit,
) {
    val well = (112 * scale).dp
    val knob = (52 * scale).dp
    var offset by remember { mutableStateOf(Offset.Zero) }
    var pressed by remember { mutableStateOf(false) }
    var moved by remember { mutableStateOf(false) }

    Box(
        modifier = Modifier
            .size(well)
            .clip(CircleShape)
            .background(if (pressed) PadGlassActive else PadGlass)
            .border(1.dp, PadBorder, CircleShape)
            .pointerInput(well) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = true)
                    down.consume()
                    pressed = true
                    moved = false
                    val centre = Offset(size.width / 2f, size.height / 2f)
                    val r = minOf(size.width, size.height).toFloat() / 2f
                    var last = Offset.Zero
                    drag(down.id) { change ->
                        val delta = change.position - centre
                        val len = hypot(delta.x, delta.y)
                        val clamped = if (len > r) delta * (r / len) else delta
                        if (change.positionChange() != Offset.Zero) moved = true
                        last = clamped
                        offset = clamped
                        onMove(clamped.x / r, clamped.y / r)
                        change.consume()
                    }
                    pressed = false
                    offset = Offset.Zero
                    onMove(0f, 0f)
                    if (!moved) {
                        // Quick tap = stick click (L3/R3): press + release so the host sees a tap.
                        onClick(true)
                        onClick(false)
                    }
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        // Faint cross guides
        Box(Modifier.size(well * 0.62f).clip(CircleShape).border(1.dp, Color.White.copy(alpha = 0.08f), CircleShape))
        Box(
            Modifier
                .size(knob)
                .graphicsLayer {
                    translationX = offset.x * density
                    translationY = offset.y * density
                    scaleX = if (pressed) 1.06f else 1f
                    scaleY = if (pressed) 1.06f else 1f
                }
                .clip(CircleShape)
                .background(
                    if (pressed) Color.White.copy(alpha = 0.34f) else Color.White.copy(alpha = 0.20f),
                )
                .border(1.dp, PadBorder, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Text(label, color = PadText, fontSize = (11 * scale).sp, fontWeight = FontWeight.Bold)
        }
    }
}

/** The four face buttons in the Xbox diamond: Y top, X left, B right, A bottom. */
@Composable
private fun FaceButtons(scale: Float, onPress: (bit: Int, down: Boolean) -> Unit) {
    val btn = (54 * scale).dp
    val gap = (4 * scale).dp
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(gap),
    ) {
        FaceButton("Y", Color(0xFFE0B23C), btn) { onPress(Gamepad.BTN_Y, it) }
        Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
            FaceButton("X", Color(0xFF4B7BD1), btn) { onPress(Gamepad.BTN_X, it) }
            FaceButton("B", Color(0xFFD14B4B), btn) { onPress(Gamepad.BTN_B, it) }
        }
        FaceButton("A", Color(0xFF6BBE45), btn) { onPress(Gamepad.BTN_A, it) }
    }
}

@Composable
private fun FaceButton(letter: String, accent: Color, size: androidx.compose.ui.unit.Dp, onDown: (Boolean) -> Unit) {
    var pressed by remember { mutableStateOf(false) }
    Box(
        modifier = Modifier
            .size(size)
            .graphicsLayer {
                scaleX = if (pressed) 0.92f else 1f
                scaleY = if (pressed) 0.92f else 1f
            }
            .clip(CircleShape)
            .background(if (pressed) accent.copy(alpha = 0.55f) else PadGlass)
            .border(1.5.dp, if (pressed) accent else PadBorder, CircleShape)
            .pointerInput(letter) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = true).consume()
                    pressed = true
                    onDown(true)
                    // Wait for the lift wherever it happens (a finger may drift off the button).
                    do { val ev = awaitPointerEvent() } while (ev.type != PointerEventType.Release)
                    pressed = false
                    onDown(false)
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        Text(letter, color = PadText, fontSize = (size.value * 0.4f).sp, fontWeight = FontWeight.Bold)
    }
}

/** A shoulder button (LB/RB): a wide glass pill that fires on touch-down, releases on lift. */
@Composable
private fun ShoulderButton(label: String, scale: Float, onDown: () -> Unit, onUp: () -> Unit) {
    var pressed by remember { mutableStateOf(false) }
    Box(
        modifier = Modifier
            .width((86 * scale).dp)
            .height((34 * scale).dp)
            .graphicsLayer { scaleX = if (pressed) 0.95f else 1f; scaleY = if (pressed) 0.95f else 1f }
            .clip(RoundedCornerShape(50))
            .background(if (pressed) PadGlassActive else PadGlass)
            .border(1.dp, PadBorder, RoundedCornerShape(50))
            .pointerInput(label) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = true).consume()
                    pressed = true
                    onDown()
                    do { val ev = awaitPointerEvent() } while (ev.type != PointerEventType.Release)
                    pressed = false
                    onUp()
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        Text(label, color = PadText, fontSize = (12 * scale).sp, fontWeight = FontWeight.Bold)
    }
}

/**
 * An analog trigger (LT/RT): a vertical glass slider — drag from the top (released) toward the
 * bottom (fully pressed); the value maps 0..255 live, so games get real partial-trigger travel.
 * Returns to 0 on release.
 */
@Composable
private fun TriggerControl(label: String, scale: Float, onValue: (Int) -> Unit) {
    val w = (86 * scale).dp
    val h = (64 * scale).dp
    var value by remember { mutableFloatStateOf(0f) }
    var pressed by remember { mutableStateOf(false) }
    Column(
        modifier = Modifier
            .width(w)
            .height(h)
            .clip(RoundedCornerShape(16.dp))
            .background(if (pressed) PadGlassActive else PadGlass)
            .border(1.dp, PadBorder, RoundedCornerShape(16.dp))
            .pointerInput(w, h) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = true)
                    down.consume()
                    pressed = true
                    fun setFrom(y: Float) {
                        value = (y / size.height).coerceIn(0f, 1f)
                        onValue(virtualTriggerValue(value))
                    }
                    setFrom(down.position.y)
                    drag(down.id) { change ->
                        setFrom(change.position.y)
                        change.consume()
                    }
                    pressed = false
                    value = 0f
                    onValue(0)
                }
            },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(label, color = PadText, fontSize = (12 * scale).sp, fontWeight = FontWeight.Bold)
        // Fill bar showing current travel
        Box(
            Modifier
                .padding(top = 6.dp)
                .width(w * 0.6f)
                .height(5.dp)
                .clip(RoundedCornerShape(50))
                .background(Color.White.copy(alpha = 0.12f)),
        ) {
            Box(
                Modifier
                    .fillMaxHeight()
                    .width((w * 0.6f) * value)
                    .clip(RoundedCornerShape(50))
                    .background(Color.White.copy(alpha = 0.8f)),
            )
        }
    }
}

/** The D-pad: a glass cross; each arm fires its direction button, diagonals allowed. */
@Composable
private fun DpadControl(scale: Float, onDirection: (List<Int>, Boolean) -> Unit) {
    val arm = (44 * scale).dp
    val total = arm * 3
    var heldDirs by remember { mutableStateOf(emptySet<Int>()) }

    fun bitsFor(x: Float, y: Float, size: Float): List<Int> {
        // Screen coords relative to the cross centre; each arm is one third of the total.
        val cx = size / 2f
        val cy = size / 2f
        val dx = x - cx
        val dy = y - cy
        val thresh = size / 6f
        val out = mutableListOf<Int>()
        if (dx < -thresh) out += Gamepad.BTN_DPAD_LEFT
        if (dx > thresh) out += Gamepad.BTN_DPAD_RIGHT
        if (dy < -thresh) out += Gamepad.BTN_DPAD_UP
        if (dy > thresh) out += Gamepad.BTN_DPAD_DOWN
        return out
    }

    Box(
        modifier = Modifier
            .size(total)
            .pointerInput(total) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = true)
                    down.consume()
                    fun apply(x: Float, y: Float) {
                        val next = bitsFor(x, y, minOf(size.width, size.height).toFloat()).toSet()
                        val old = heldDirs
                        // Emit only the transitions.
                        (next - old).forEach { onDirection(listOf(it), true) }
                        (old - next).forEach { onDirection(listOf(it), false) }
                        heldDirs = next
                    }
                    apply(down.position.x, down.position.y)
                    drag(down.id) { change ->
                        apply(change.position.x, change.position.y)
                        change.consume()
                    }
                    heldDirs.forEach { onDirection(listOf(it), false) }
                    heldDirs = emptySet()
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        // The cross visual: vertical + horizontal glass bars.
        Box(
            Modifier
                .width(arm)
                .height(total)
                .clip(RoundedCornerShape((arm.value * 0.36f).dp))
                .background(PadGlass)
                .border(1.dp, PadBorder, RoundedCornerShape((arm.value * 0.36f).dp)),
        )
        Box(
            Modifier
                .width(total)
                .height(arm)
                .clip(RoundedCornerShape((arm.value * 0.36f).dp))
                .background(PadGlass)
                .border(1.dp, PadBorder, RoundedCornerShape((arm.value * 0.36f).dp)),
        )
        // Direction pips
        Column(
            Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.SpaceBetween,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("▲", color = PadText.copy(alpha = 0.6f), fontSize = (10 * scale).sp, modifier = Modifier.padding(top = 6.dp))
            Spacer(Modifier.weight(1f))
            Text("▼", color = PadText.copy(alpha = 0.6f), fontSize = (10 * scale).sp, modifier = Modifier.padding(bottom = 6.dp))
        }
        Row(
            Modifier.fillMaxSize(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("◀", color = PadText.copy(alpha = 0.6f), fontSize = (10 * scale).sp, modifier = Modifier.padding(start = 6.dp))
            Spacer(Modifier.weight(1f))
            Text("▶", color = PadText.copy(alpha = 0.6f), fontSize = (10 * scale).sp, modifier = Modifier.padding(end = 6.dp))
        }
    }
}

/** A small centre-cluster button (Back / Guide / Start). */
@Composable
private fun SmallButton(glyph: String, accent: Boolean = false, onPress: (Boolean) -> Unit) {
    var pressed by remember { mutableStateOf(false) }
    Box(
        modifier = Modifier
            .size(42.dp)
            .graphicsLayer { scaleX = if (pressed) 0.9f else 1f; scaleY = if (pressed) 0.9f else 1f }
            .clip(CircleShape)
            .background(
                when {
                    pressed && accent -> SlipstreamViolet.copy(alpha = 0.5f)
                    pressed -> PadGlassActive
                    accent -> SlipstreamViolet.copy(alpha = 0.22f)
                    else -> PadGlass
                },
            )
            .border(1.dp, if (accent) SlipstreamViolet.copy(alpha = 0.6f) else PadBorder, CircleShape)
            .pointerInput(glyph) {
                awaitEachGesture {
                    awaitFirstDown(requireUnconsumed = true).consume()
                    pressed = true
                    onPress(true)
                    do { val ev = awaitPointerEvent() } while (ev.type != PointerEventType.Release)
                    pressed = false
                    onPress(false)
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        Text(glyph, color = PadText, fontSize = 15.sp, fontWeight = FontWeight.Bold)
    }
}
