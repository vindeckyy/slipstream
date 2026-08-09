package io.slipstream.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Key
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.LockOpen
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.slipstream.design.glassSurface
import io.slipstream.models.HostStatus

/**
 * Section header above each block of the connect screen — a brand-coloured small-caps label with a
 * soft hairline under it, the quiet divider between "Saved hosts" and "Discovered".
 */
@Composable
fun SectionLabel(text: String) {
    Column(Modifier.fillMaxWidth().padding(bottom = 10.dp)) {
        Text(
            text.uppercase(),
            style = MaterialTheme.typography.labelMedium,
            fontWeight = FontWeight.SemiBold,
            letterSpacing = 1.4.sp,
            color = MaterialTheme.colorScheme.primary.copy(alpha = 0.85f),
        )
    }
}

/**
 * One row of a host card's overflow menu. [startsSection] draws a divider above it, which is how
 * the profile actions ("Connect with: …", "Pin as card: …") stay legible next to the host actions
 * in one flat menu — Compose has no submenus, and the desktop client uses the same layout.
 */
data class HostMenuItem(
    val label: String,
    val startsSection: Boolean = false,
    val onClick: () -> Unit,
)

/** Live presence green — reads as "up" on any palette. */
private val PRESENCE_ONLINE = Color(0xFF4ADE80)

/**
 * A host as a glass card: a coloured avatar carrying the host's OS mark, name + address, a trust
 * badge, and (for saved hosts) an overflow menu with Wake / Edit / Forget plus whatever
 * [menuItems] adds. Tapping the card connects — with a spring-press scale so the tap feels
 * physical. The card sits on the aurora backdrop: translucent glass wash, hairline highlight.
 *
 * [profileLabel] names the settings profile this card connects with. On a host's own card that is
 * its default binding, drawn as a quiet chip — the card says what a tap will do. On a **pinned
 * card** ([profileProminent]) the host name is still the title, but the profile is the loud part,
 * because the pin exists to make that one combination a single tap.
 */
@Composable
fun HostCard(
    name: String,
    address: String,
    status: HostStatus,
    online: Boolean = false,
    /** OS-identity chain (mDNS `os` TXT / stored), drawn as the avatar's mark. "" = the initial. */
    os: String = "",
    enabled: Boolean,
    onConnect: () -> Unit,
    onForget: (() -> Unit)?,
    onEdit: (() -> Unit)? = null,
    onWake: (() -> Unit)? = null,
    profileLabel: String? = null,
    profileProminent: Boolean = false,
    accent: Color? = null,
    menuItems: List<HostMenuItem> = emptyList(),
    /**
     * Keep the profile chip's space even on a card that has no profile. `LazyVerticalGrid` sizes a
     * row to its tallest item but does NOT stretch the others, so a card that grew a chip would
     * leave its neighbour visibly short — a row of cards stepping up and down reads as broken
     * layout. The caller passes true when ANY card in that section carries a chip, so a user with
     * no profiles never pays for the slot.
     */
    reserveProfileSlot: Boolean = false,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val focused by interactionSource.collectIsFocusedAsState()
    // Spring the press so a tap feels physical; scale on focus (D-pad) too, with the violet ring.
    val scale by animateFloatAsState(
        targetValue = when {
            pressed -> 0.96f
            focused -> 1.02f
            else -> 1f
        },
        animationSpec = spring(dampingRatio = 0.6f, stiffness = Spring.StiffnessMedium),
        label = "cardPress",
    )
    val focusBorder by animateColorAsState(
        if (focused) MaterialTheme.colorScheme.primary.copy(alpha = 0.9f) else Color.White.copy(alpha = 0.14f),
        label = "cardFocusBorder",
    )
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(4.dp)
            .graphicsLayer {
                scaleX = scale
                scaleY = scale
            }
            .glassSurface(
                shape = RoundedCornerShape(22.dp),
                tint = if (profileProminent) (accent ?: MaterialTheme.colorScheme.primary) else MaterialTheme.colorScheme.primary,
                tintAlpha = if (profileProminent) 0.35f else 0.08f,
                borderAlpha = 0f, // the animated border below draws it instead
            )
            .border(
                width = if (focused) 2.dp else 1.dp,
                color = focusBorder,
                shape = RoundedCornerShape(22.dp),
            )
            .clickable(
                interactionSource = interactionSource,
                indication = null,
                enabled = enabled,
                onClick = onConnect,
            )
            .graphicsLayer { alpha = if (enabled) 1f else 0.45f },
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            HostAvatar(name, online, os)
            Spacer(Modifier.height(10.dp))
            Text(
                name,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                color = Color.White,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
            Text(
                address,
                style = MaterialTheme.typography.bodySmall,
                color = Color.White.copy(alpha = 0.5f),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
            if (profileLabel != null || reserveProfileSlot) {
                Spacer(Modifier.height(10.dp))
                Box(
                    Modifier.heightIn(min = PROFILE_CHIP_SLOT),
                    contentAlignment = Alignment.Center,
                ) {
                    if (profileLabel != null) {
                        ProfileChip(profileLabel, accent, prominent = profileProminent)
                    }
                }
            }
        }

        // Trust state lives in the free top-left corner, mirroring the overflow on the right —
        // it costs no height, and it is a state you glance at rather than read.
        TrustBadge(status, Modifier.align(Alignment.TopStart))

        if (onForget != null || onEdit != null || onWake != null || menuItems.isNotEmpty()) {
            var menu by remember { mutableStateOf(false) }
            Box(modifier = Modifier.align(Alignment.TopEnd)) {
                IconButton(enabled = enabled, onClick = { menu = true }) {
                    Icon(
                        Icons.Filled.MoreVert,
                        contentDescription = "More",
                        modifier = Modifier.size(20.dp),
                        tint = Color.White.copy(alpha = 0.55f),
                    )
                }
                DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                    if (onWake != null) {
                        DropdownMenuItem(
                            text = { Text("Wake host") },
                            onClick = {
                                menu = false
                                onWake()
                            },
                        )
                    }
                    if (onEdit != null) {
                        DropdownMenuItem(
                            text = { Text("Edit…") },
                            onClick = {
                                menu = false
                                onEdit()
                            },
                        )
                    }
                    if (onForget != null) {
                        DropdownMenuItem(
                            text = { Text("Forget") },
                            onClick = {
                                menu = false
                                onForget()
                            },
                        )
                    }
                    menuItems.forEach { item ->
                        if (item.startsSection) HorizontalDivider()
                        DropdownMenuItem(
                            text = { Text(item.label) },
                            onClick = {
                                menu = false
                                item.onClick()
                            },
                        )
                    }
                }
            }
        }
    }
}

/**
 * The profile a card connects with. Quiet on a bound host's own card (it is a note about what a tap
 * does); filled and tinted on a pinned card, where the profile IS the reason the card exists — the
 * accent field the schema reserves earns its keep here.
 */
@Composable
private fun ProfileChip(label: String, accent: Color?, prominent: Boolean) {
    val tint = accent ?: MaterialTheme.colorScheme.primary
    Row(
        modifier = Modifier
            .clip(RoundedCornerShape(50))
            .background(tint.copy(alpha = if (prominent) 0.28f else 0.14f))
            .padding(horizontal = 10.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(7.dp).clip(CircleShape).background(tint))
        Spacer(Modifier.width(6.dp))
        Text(
            label,
            style = if (prominent) {
                MaterialTheme.typography.labelLarge
            } else {
                MaterialTheme.typography.labelMedium
            },
            fontWeight = FontWeight.SemiBold,
            color = tint,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/**
 * Reserved height for the profile chip — the one part of a card that varies. `LazyVerticalGrid`
 * sizes a row to its tallest item and does NOT stretch the others, so a card that grew a chip its
 * neighbour lacks would leave the row stepping up and down.
 *
 * `heightIn(min =)`, not a fixed height: at a large accessibility font scale the chip must be
 * allowed to grow rather than clip, and the reservation is sized with room to spare because the
 * equal-height guarantee only holds while every card fits INSIDE it.
 */
private val PROFILE_CHIP_SLOT = 26.dp

/**
 * The host's avatar (Apple-contact style) with its presence as a dot on the corner — the idiom
 * every contact list already uses, and one fewer labelled badge on a small card. It carries the
 * host's OS mark when [os] resolves to one we ship, and the host's initial otherwise.
 *
 * [online] is true when the host advertises on mDNS OR answers the reachability probe, so a
 * routed/VPN host that never advertises still reads as up. Online is a FILLED green dot with a
 * soft glow, offline a hollow ring: the difference is a shape as well as a colour, so it survives
 * both a colour-blind reader and a screenshot in greyscale. TalkBack gets the word either way.
 */
@Composable
fun HostAvatar(name: String, online: Boolean = false, os: String = "") {
    val letter = name.trim().firstOrNull()?.uppercaseChar()?.toString() ?: "?"
    val osIcon = resolveOsIcon(os)
    Box {
        Box(
            modifier = Modifier
                .size(48.dp)
                .clip(CircleShape)
                .background(
                    Brush.verticalGradient(
                        listOf(
                            MaterialTheme.colorScheme.primary.copy(alpha = 0.55f),
                            MaterialTheme.colorScheme.primary.copy(alpha = 0.22f),
                        ),
                    ),
                )
                .border(1.dp, Color.White.copy(alpha = 0.18f), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            // The OS mark IS the avatar when we know the OS — it identifies the machine better than
            // the initial ever did, and it's the same circle, so a card whose host advertises no OS
            // (or one we ship no mark for) keeps the letter and the row still reads as one set.
            if (osIcon != null) {
                Icon(
                    osIcon,
                    contentDescription = os,
                    modifier = Modifier.size(26.dp),
                    tint = Color.White,
                )
            } else {
                Text(
                    letter,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = Color.White,
                )
            }
        }
        // Presence: filled green when online, a hollow grey ring offline. The outer box is a ring
        // in the card's own dark tone, which is what makes the dot read as sitting ON the avatar.
        Box(
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .size(14.dp)
                .clip(CircleShape)
                .background(Color(0xFF100E1D))
                .padding(2.dp)
                .clip(CircleShape)
                .then(
                    if (online) {
                        Modifier.background(PRESENCE_ONLINE)
                    } else {
                        Modifier
                            .background(Color(0xFF100E1D))
                            .border(1.5.dp, MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f), CircleShape)
                    },
                )
                .semantics { contentDescription = if (online) "Online" else "Offline" },
        )
    }
}

/**
 * The host's trust state as a corner glyph: locked (paired — nothing more to do), a key (this host
 * will ask for a PIN), or an open lock (trust-on-first-use, the weakest of the three). The full
 * label rides along as the content description, and the dialogs that actually make the decision
 * spell it out in sentences.
 */
@Composable
private fun TrustBadge(status: HostStatus, modifier: Modifier = Modifier) {
    val (icon, tint) = when (status) {
        HostStatus.PAIRED -> Icons.Filled.Lock to MaterialTheme.colorScheme.primary
        HostStatus.PAIRING -> Icons.Filled.Key to Color(0xFFE0B23C)
        HostStatus.TOFU -> Icons.Filled.LockOpen to Color.White.copy(alpha = 0.45f)
    }
    Icon(
        icon,
        contentDescription = status.label,
        tint = tint,
        modifier = modifier.padding(14.dp).size(18.dp),
    )
}

/** Shown when there are no saved or discovered hosts — an inviting empty state over the aurora. */
@Composable
fun EmptyHostsState() {
    Column(
        modifier = Modifier.fillMaxWidth().padding(vertical = 56.dp, horizontal = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            Modifier
                .size(72.dp)
                .clip(CircleShape)
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.14f))
                .border(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.3f), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                Icons.Filled.LockOpen,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(30.dp),
            )
        }
        Spacer(Modifier.height(18.dp))
        Text(
            "No hosts yet",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            color = Color.White,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            "Hosts on your network show up here automatically.\nTap “Add host” to enter one by address.",
            style = MaterialTheme.typography.bodyMedium,
            color = Color.White.copy(alpha = 0.55f),
            textAlign = TextAlign.Center,
        )
    }
}
