// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Likhon input method for macOS (InputMethodKit). MPL-2.0.
//
// A thin shell around the Rust typing engine (likhon-ffi): it shows the word being typed as underlined marked
// text and commits it when the word ends. No file is read except the dictionary, and nothing typed is stored or
// sent anywhere.
//
// NOT YET RUN ON A MAC. Written on Windows from Apple's documentation; the first build and test on a real Mac
// (Desktop/Mac/README.md) decides what needs fixing.

import Cocoa
import InputMethodKit

// The name must match InputMethodConnectionName in Info.plist.
let connectionName = Bundle.main.object(forInfoDictionaryKey: "InputMethodConnectionName") as? String
    ?? "app.likhon.inputmethod.Likhon_Connection"

// Kept alive for the life of the process: IMKServer answers the system's requests for new controllers.
let server = IMKServer(name: connectionName, bundleIdentifier: Bundle.main.bundleIdentifier)

// One shared suggestion panel (IMKCandidates), created once the server exists.
LikhonInputController.sharedPanel = IMKCandidates(server: server, panelType: kIMKSingleColumnScrollingCandidatePanel)

NSApplication.shared.run()
