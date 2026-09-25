import Foundation
import CoreServices

let audioUTIs = [
    "public.mp3",
    "com.apple.m4a-audio",
    "public.wave-format",
    "com.microsoft.waveform-audio",
    "public.flac-audio",
    "org.xiph.flac",
    "public.aiff-audio",
    "public.aac-audio",
    "org.xiph.ogg-audio",
    "public.audio"
]

let bundleIds = ["com.crate.app", "com.bbx-audio.crate"]

print("Setting Crate as default audio player for macOS UTIs...")

for bundleId in bundleIds {
    print("\nRegistering handlers for bundle ID: \(bundleId)")
    for uti in audioUTIs {
        let resAll = LSSetDefaultRoleHandlerForContentType(uti as CFString, .all, bundleId as CFString)
        let resViewer = LSSetDefaultRoleHandlerForContentType(uti as CFString, .viewer, bundleId as CFString)
        print("  - \(uti): all=\(resAll), viewer=\(resViewer)")
    }
}

print("\nDone setting default role handlers.")
