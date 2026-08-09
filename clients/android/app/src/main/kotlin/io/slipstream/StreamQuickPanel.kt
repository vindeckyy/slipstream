package io.slipstream

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Keyboard
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.MicOff
import androidx.compose.material.icons.filled.PowerSettingsNew
import androidx.compose.material.icons.filled.SportsEsports
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.slipstream.design.glassSurface

// The in-stream quick panel — one glass sheet for everything a streamer might want without leaving
// the game: mic mute, stats tier, the virtual gamepad, the soft keyboard, and disconnect. Replaces
// the single floating mic button as the primary in-stream control surface (the mic button stays as
// a quick glance indicator when muted). Slides down from the top over the video.

/**
 * The panel. [visible] drives the slide animation; content is inert while hidden. Every action
 * callback is wired by StreamScreen; booleans reflect live session state.
 */
@Composable
fun StreamQuickPanel(
    visible: Boolean,
    onDismiss: () -> Unit,
    hostName: String,
    sessionLine: String,
    micRunning: Boolean,
    micMuted: Boolean,
    onMicToggle: () -> Unit,
    statsVerbosity: StatsVerbosity,
    onCycleStats: () -> Unit,
    padAvailable: Boolean,
    padVisible: Boolean,
    onPadToggle: () -> Unit,
    onKeyboard: () -> Unit,
    onDisconnect: () -> Unit,
) {
    AnimatedVisibility(
        visible = visible,
        enter = slideInVertically(animationSpec = tween(260), initialOffsetY = { -it }),
        exit = slideOutVertically(animationSpec = tween(220), targetOffsetY = { -it }),
    ) {
        // Tap-outside-to-close scrim is owned by the caller (it knows the layout); the sheet itself
        // slides from the top edge.
        Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.TopCenter) {
            Column(
                Modifier
                    .padding(horizontal = 16.dp, vertical = 12.dp)
                    .widthIn(max = 460.dp)
                    .fillMaxWidth()
                    .glassSurface(shape = RoundedCornerShape(24.dp))
                    .padding(18.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(
                            hostName,
                            color = Color.White,
                            fontSize = 17.sp,
                            fontWeight = FontWeight.Bold,
                            maxLines = 1,
                            softWrap = false,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Text(
                            sessionLine,
                            color = Color.White.copy(alpha = 0.55f),
                            fontSize = 12.sp,
                            maxLines = 1,
                            softWrap = false,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                    Icon(
                        Icons.Filled.Close,
                        contentDescription = "Close",
                        tint = Color.White.copy(alpha = 0.7f),
                        modifier = Modifier
                            .clip(RoundedCornerShape(10.dp))
                            .clickable(onClick = onDismiss)
                            .padding(6.dp)
                            .size(22.dp),
                    )
                }
                Spacer(Modifier.height(16.dp))
                // Primary actions — the two rows of big glass tiles.
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    PanelTile(
                        icon = if (micMuted) Icons.Filled.MicOff else Icons.Filled.Mic,
                        label = if (!micRunning) "Mic off" else if (micMuted) "Unmute" else "Mute",
                        sub = if (!micRunning) "not enabled" else "microphone",
                        accent = if (micMuted) Color(0xFFFFB4AB) else SlipstreamViolet,
                        enabled = micRunning,
                        modifier = Modifier.weight(1f),
                        onClick = onMicToggle,
                    )
                    PanelTile(
                        icon = Icons.Filled.Tune,
                        label = statsVerbosity.label,
                        sub = "stats overlay",
                        accent = SlipstreamViolet,
                        enabled = true,
                        modifier = Modifier.weight(1f),
                        onClick = onCycleStats,
                    )
                }
                Spacer(Modifier.height(10.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    PanelTile(
                        icon = Icons.Filled.SportsEsports,
                        label = if (padVisible) "Hide pad" else "Show pad",
                        sub = if (padAvailable) "on-screen gamepad" else "disabled in settings",
                        accent = SlipstreamViolet,
                        enabled = padAvailable,
                        modifier = Modifier.weight(1f),
                        onClick = onPadToggle,
                    )
                    PanelTile(
                        icon = Icons.Filled.Keyboard,
                        label = "Keyboard",
                        sub = "type on the host",
                        accent = SlipstreamViolet,
                        enabled = true,
                        modifier = Modifier.weight(1f),
                        onClick = { onKeyboard(); onDismiss() },
                    )
                }
                Spacer(Modifier.height(16.dp))
                // Disconnect — full width, deliberately quiet until touched.
                Row(
                    Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(16.dp))
                        .background(Color(0xFFB3261E).copy(alpha = 0.16f))
                        .border(1.dp, Color(0xFFFFB4AB).copy(alpha = 0.25f), RoundedCornerShape(16.dp))
                        .clickable(onClick = onDisconnect)
                        .padding(vertical = 13.dp),
                    horizontalArrangement = Arrangement.Center,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        Icons.Filled.PowerSettingsNew,
                        contentDescription = null,
                        tint = Color(0xFFFFB4AB),
                        modifier = Modifier.size(18.dp),
                    )
                    Spacer(Modifier.width(10.dp))
                    Text("Disconnect", color = Color(0xFFFFB4AB), fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                }
            }
        }
    }
}

/** One action tile inside the quick panel. */
@Composable
private fun PanelTile(
    icon: ImageVector,
    label: String,
    sub: String,
    accent: Color,
    enabled: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    Column(
        modifier
            .height(84.dp)
            .clip(RoundedCornerShape(18.dp))
            .background(Color.White.copy(alpha = if (enabled) 0.08f else 0.03f))
            .border(1.dp, Color.White.copy(alpha = if (enabled) 0.14f else 0.06f), RoundedCornerShape(18.dp))
            .clickable(enabled = enabled) { onClick() }
            .padding(14.dp),
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = if (enabled) accent else Color.White.copy(alpha = 0.25f),
            modifier = Modifier.size(22.dp),
        )
        Column {
            Text(
                label,
                color = if (enabled) Color.White else Color.White.copy(alpha = 0.3f),
                fontSize = 14.sp,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                softWrap = false,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                sub,
                color = Color.White.copy(alpha = if (enabled) 0.45f else 0.2f),
                fontSize = 11.sp,
                maxLines = 1,
                softWrap = false,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}
