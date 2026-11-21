// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// The first-run software license agreement (the final legal documents of 2026-11-21). MPL-2.0.
//
// Until this copy has accepted the current agreement, Likhon types nothing and every key passes through. The first
// key of a session shows the agreement in a window with Accept and Decline; the accepted date is kept in this input
// method's own preferences (no other file), and asked again when the agreement's date changes. Decline leaves it off.
// NOT YET RUN ON A MAC: written from Apple's documentation, like the rest of this folder.

import Cocoa

enum Agreement {
    /// The date of the current agreement (Web/Laravel/resources/legal/en/license.md, `updated`).
    static let date = "2026-11-21"
    private static let key = "agreementAccepted"
    private static var asked = false

    static var accepted: Bool {
        UserDefaults.standard.string(forKey: key) == date
    }

    /// Shows the agreement once per session; Accept stores the date.
    static func askOnce() {
        if asked || accepted { return }
        asked = true
        DispatchQueue.main.async { show() }
    }

    private static func show() {
        let bangla = (Locale.preferredLanguages.first ?? "en").hasPrefix("bn")
        let name = bangla ? "agreement.bn" : "agreement.en"
        guard let path = Bundle.main.path(forResource: name, ofType: "txt"), let text = try? String(contentsOfFile: path, encoding: .utf8) else { return }

        let alert = NSAlert()
        alert.messageText = bangla ? "লিখন: সফটওয়্যার লাইসেন্স চুক্তি" : "Likhon: software license agreement"
        alert.informativeText = bangla ? "লিখন চালু হবে কেবল আপনি রাজি হলে।" : "Likhon starts only if you accept."
        alert.addButton(withTitle: bangla ? "আমি রাজি" : "I accept")
        alert.addButton(withTitle: bangla ? "রাজি নই" : "Decline")

        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 560, height: 320))
        scroll.hasVerticalScroller = true
        scroll.borderType = .bezelBorder
        let view = NSTextView(frame: scroll.bounds)
        view.isEditable = false
        view.string = text
        view.font = NSFont.systemFont(ofSize: 12)
        view.autoresizingMask = [.width]
        scroll.documentView = view
        alert.accessoryView = scroll

        NSApp.activate(ignoringOtherApps: true)
        if alert.runModal() == .alertFirstButtonReturn {
            UserDefaults.standard.set(date, forKey: key)
        }
    }
}
