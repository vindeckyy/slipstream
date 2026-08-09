// Location-based modifier mapping, which GameStream virtual-key each PHYSICAL modifier position
// forwards to the host. A Mac keyboard's bottom row is `Control / Option / Command / space`; a PC
// keyboard's is `Ctrl / Super / Alt / space`. The key nearest the space bar is Command on a Mac but
// Alt on a PC, and the next one out is Option on a Mac but Super on a PC. This setting preserves
// that muscle memory without relabelling keycaps. It remaps by physical position, preserving
// Control/Shift and the left/right distinction.
//
// The model keeps physical detection (which side, which key) exactly as the OS reports it, then
// swaps only the Alt and Super roles between the Option and Command keys, per side. Under `.pc`,
// the Command position emits Alt (VK_L/RMENU) and the Option position emits Super (VK_L/RWIN), while
// left stays left and right stays right. Control and Shift are in the same place on both keyboards,
// so they never move. This lives in SlipstreamShared because both the Settings UI (SlipstreamClient)
// and the wire-boundary remap (`InputCapture.applyModifierLayout`, SlipstreamKit) resolve it.

import Foundation

/// How the physical ⌥ Option / ⌘ Command keys map to host modifiers. The raw values are stable on
/// disk. Keep the raw values stable after release.
public enum ModifierLayout: String, CaseIterable, Sendable {
    /// Apple positions (default): Option to Alt, Command to Super. The current behaviour.
    case mac
    /// PC positions: the key nearest the space bar (Command) to Alt, the next one (Option) to Super.
    /// Side (left/right) is preserved.
    case pc

    /// User-facing label (Settings picker).
    public var label: String {
        switch self {
        case .mac: return "Mac (⌥ Alt · ⌘ Super)"
        case .pc: return "PC (⌘ Alt · ⌥ Super)"
        }
    }

    /// A one-line explanation for the setting's footer.
    public var detail: String {
        switch self {
        case .mac:
            return "The Option key sends Alt and the Command key sends Super, the Apple layout."
        case .pc:
            return "The key nearest the space bar sends Alt and the next one sends Super, matching a PC keyboard. Client shortcuts still use the physical Command key."
        }
    }

    /// The persisted layout (default `.mac` when unset).
    public static var current: ModifierLayout {
        guard let raw = SessionSettings.active?.modifierLayout
            ?? UserDefaults.standard.string(forKey: DefaultsKey.modifierLayout) else {
            return .mac
        }
        return ModifierLayout(rawValue: raw) ?? .mac
    }
}
