package io.slipstream

import android.content.Context
import android.net.wifi.WifiManager
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

/**
 * A compact device-specific tuning surface. The card is deliberately action-oriented: it tells a
 * Fire tablet owner what the stream will do, shows the current Wi-Fi link, and exposes one safe
 * reset instead of making them hunt through three settings categories.
 */
@Composable
internal fun FireHd10TuningCard(
    profile: DeviceProfiles.Profile,
    settings: Settings,
    onApply: () -> Unit,
    onOpenSettings: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val wifi = remember(context) { currentWifiSummary(context) }
    val optimized = DeviceProfiles.isOptimized(settings)

    OutlinedCard(modifier = modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(profile.name, fontWeight = FontWeight.SemiBold)
                    Text(
                        "Tablet tuning",
                        style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
                        color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Surface(
                    color = if (optimized) {
                        androidx.compose.material3.MaterialTheme.colorScheme.primaryContainer
                    } else {
                        androidx.compose.material3.MaterialTheme.colorScheme.surfaceVariant
                    },
                    shape = androidx.compose.material3.MaterialTheme.shapes.small,
                ) {
                    Text(
                        if (optimized) "READY" else "TUNE",
                        style = androidx.compose.material3.MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.Bold,
                        color = if (optimized) {
                            androidx.compose.material3.MaterialTheme.colorScheme.onPrimaryContainer
                        } else {
                            androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant
                        },
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                    )
                }
            }
            Text(
                "${profile.recommendedMode.first} x ${profile.recommendedMode.second} @ " +
                    "${profile.recommendedMode.third} Hz keeps the picture inside the tablet's " +
                    "1080p60 hardware decode envelope.",
                style = androidx.compose.material3.MaterialTheme.typography.bodyMedium,
            )
            Row(Modifier.fillMaxWidth()) {
                Text(
                    profile.decoderLabel,
                    style = androidx.compose.material3.MaterialTheme.typography.labelMedium,
                    modifier = Modifier.weight(1f),
                )
                Spacer(Modifier.width(12.dp))
                Text(
                    wifi,
                    style = androidx.compose.material3.MaterialTheme.typography.labelMedium,
                    color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Button(onClick = onApply, modifier = Modifier.weight(1f)) {
                    Text(if (optimized) "Reapply tuning" else "Tune for lower latency")
                }
                if (onOpenSettings != null) {
                    OutlinedButton(onClick = onOpenSettings) { Text("Settings") }
                }
            }
        }
    }
}

private fun currentWifiSummary(context: Context): String {
    val manager = context.applicationContext.getSystemService(WifiManager::class.java)
        ?: return "Network unavailable"
    if (!manager.isWifiEnabled) return "Wi-Fi off"
    val info = runCatching { manager.connectionInfo }.getOrNull()
        ?: return "Wi-Fi not connected"
    if (info.networkId == -1 && info.ssid == "<unknown ssid>") return "Wi-Fi not connected"
    val band = when {
        info.frequency >= 5925 -> "6 GHz"
        info.frequency >= 4900 -> "5 GHz"
        info.frequency >= 2400 -> "2.4 GHz"
        else -> "Wi-Fi"
    }
    return if (info.linkSpeed > 0) "$band - ${info.linkSpeed} Mbps" else "$band connected"
}
