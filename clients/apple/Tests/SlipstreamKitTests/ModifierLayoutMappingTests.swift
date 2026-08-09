import XCTest

import SlipstreamShared
@testable import SlipstreamKit

/// Pins the location-based modifier remap (`InputCapture.applyModifierLayout`) — the wire-boundary
/// swap that relocates the Alt/Super role between the physical ⌥/⌘ keys per side, without touching
/// Control/Shift or the physical detection upstream.
final class ModifierLayoutMappingTests: XCTestCase {
    // The four physical modifier VKs the remap can touch, and everything else it must not.
    private let lSuper: UInt32 = 0x5B, rSuper: UInt32 = 0x5C
    private let lAlt: UInt32 = 0xA4, rAlt: UInt32 = 0xA5

    func testMacLayoutIsIdentity() {
        for vk: UInt32 in [lSuper, rSuper, lAlt, rAlt, 0xA0, 0xA2, 0x41, 0x1B] {
            XCTAssertEqual(InputCapture.applyModifierLayout(vk, .mac), vk)
        }
    }

    func testPCLayoutSwapsAltAndSuperPerSide() {
        XCTAssertEqual(InputCapture.applyModifierLayout(lSuper, .pc), lAlt)
        XCTAssertEqual(InputCapture.applyModifierLayout(rSuper, .pc), rAlt)
        XCTAssertEqual(InputCapture.applyModifierLayout(lAlt, .pc), lSuper)
        XCTAssertEqual(InputCapture.applyModifierLayout(rAlt, .pc), rSuper)
    }

    func testPCLayoutKeepsSideNeverCrossesLeftRight() {
        // A left key never becomes a right VK or vice-versa.
        XCTAssertNotEqual(InputCapture.applyModifierLayout(lSuper, .pc), rAlt)
        XCTAssertNotEqual(InputCapture.applyModifierLayout(rSuper, .pc), lAlt)
    }

    func testControlShiftAndRegularKeysNeverMove() {
        for vk: UInt32 in [
            0xA0, 0xA1, // L/R Shift
            0xA2, 0xA3, // L/R Control
            0x41, 0x5A, // A, Z
            0x1B, 0x0D, // Esc, Return
        ] {
            XCTAssertEqual(InputCapture.applyModifierLayout(vk, .pc), vk)
        }
    }

    func testPCRemapIsItsOwnInverse() {
        // Applying the remap on key-down and key-up prevents stuck modifiers.
        for vk: UInt32 in [lSuper, rSuper, lAlt, rAlt] {
            let once = InputCapture.applyModifierLayout(vk, .pc)
            XCTAssertEqual(InputCapture.applyModifierLayout(once, .pc), vk)
        }
    }
}
