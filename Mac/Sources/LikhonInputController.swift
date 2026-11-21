// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// One controller per text field (InputMethodKit creates it). MPL-2.0. See main.swift: not yet run on a Mac.

import Cocoa
import InputMethodKit

@objc(LikhonInputController)
class LikhonInputController: IMKInputController {
    /// The suggestion panel, shared by all fields (set in main.swift).
    static var sharedPanel: IMKCandidates?

    /// The Rust session for this field: one word at a time.
    private var session: OpaquePointer?
    /// The rows shown in the panel right now.
    private var rows: [String] = []

    private static let dictionaryPath: String? = {
        // The dictionary ships inside the app bundle (Contents/Resources/words.sqlite).
        Bundle.main.path(forResource: "words", ofType: "sqlite")
    }()

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        session = likhon_session_new()
        if let path = LikhonInputController.dictionaryPath {
            _ = likhon_session_load_dictionary(session, path)
        }
    }

    deinit {
        likhon_session_free(session)
    }

    // MARK: Keys

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event = event, event.type == .keyDown, let client = sender as? IMKTextInput else { return false }

        // The agreement comes first: until it is accepted nothing is typed and the key goes straight through.
        if !Agreement.accepted {
            Agreement.askOnce()
            return false
        }

        // Shortcuts belong to the application: end the word and let the key through.
        let shortcut: NSEvent.ModifierFlags = [.command, .control, .option]
        if !event.modifierFlags.intersection(shortcut).isEmpty {
            endWord(client)
            return false
        }

        switch Int(event.keyCode) {
        case 51:  // Delete (Backspace)
            return apply(client) { likhon_session_backspace(self.session, $0, $1) }
        case 53:  // Escape drops the word
            return apply(client) { likhon_session_cancel(self.session, $0, $1) }
        case 125, 126:  // Down, Up move in the suggestion list
            if rows.isEmpty { endWord(client); return false }
            let down: Int32 = event.keyCode == 125 ? 1 : 0
            return apply(client) { likhon_session_move(self.session, down, $0, $1) }
        case 36, 76:  // Return, keypad Enter: types the selected row when a list is open
            if rows.isEmpty { endWord(client); return false }
            return apply(client) { likhon_session_accept(self.session, $0, $1) }
        case 48, 117, 115, 119, 116, 121, 123, 124:  // Tab, forward delete, Home, End, PageUp/Down, Left, Right
            endWord(client)
            return false
        default:
            break
        }

        // Plain text: the shifted character the key makes in the current keyboard layout.
        guard let chars = event.characters, let scalar = chars.unicodeScalars.first,
              scalar.value >= 0x20, scalar.value != 0x7F, !(0xF700...0xF8FF).contains(scalar.value) else {
            endWord(client)  // function keys and the like
            return false
        }
        return apply(client) { likhon_session_key(self.session, scalar.value, $0, $1) }
    }

    /// Calls one of the session functions, then shows what it says: text to insert, the word being typed, the rows.
    private func apply(_ client: IMKTextInput,
                       _ call: (UnsafeMutablePointer<UnsafePointer<CChar>?>, UnsafeMutablePointer<UnsafePointer<CChar>?>) -> Int32) -> Bool {
        var commit: UnsafePointer<CChar>?
        var preedit: UnsafePointer<CChar>?
        let used = call(&commit, &preedit) != 0
        show(client, commit: commit.map { String(cString: $0) } ?? "", preedit: preedit.map { String(cString: $0) } ?? "")
        return used
    }

    private func endWord(_ client: IMKTextInput) {
        _ = apply(client) { likhon_session_reset(self.session, $0, $1) }
    }

    // MARK: Showing

    private let notFound = NSRange(location: NSNotFound, length: NSNotFound)

    private func show(_ client: IMKTextInput, commit: String, preedit: String) {
        if !commit.isEmpty {
            client.insertText(commit, replacementRange: notFound)
        }
        if preedit.isEmpty {
            client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0), replacementRange: notFound)
        } else {
            let marked = NSAttributedString(string: preedit, attributes: [
                .underlineStyle: NSUnderlineStyle.single.rawValue,
                .markedClauseSegment: 0,
            ])
            client.setMarkedText(marked, selectionRange: NSRange(location: preedit.utf16.count, length: 0), replacementRange: notFound)
        }
        showRows()
    }

    private func showRows() {
        let count = Int(likhon_session_row_count(session))
        rows = (0..<count).compactMap { likhon_session_row(session, Int32($0)).map { String(cString: $0) } }
        guard let panel = LikhonInputController.sharedPanel else { return }
        if rows.isEmpty {
            panel.hide()
        } else {
            panel.update()
            panel.show(kIMKLocateCandidatesBelowHint)
        }
    }

    // MARK: IMKCandidates (the panel asks for rows and tells us the choice)

    override func candidates(_ sender: Any!) -> [Any]! {
        return rows
    }

    override func candidateSelected(_ candidateString: NSAttributedString!) {
        // A click on a row: type that word with a space after it, as the number keys do.
        guard let client = client() as? IMKTextInput, let word = candidateString?.string,
              let index = rows.firstIndex(of: word) else { return }
        _ = apply(client) { likhon_session_pick(self.session, Int32(index), $0, $1) }
    }

    // MARK: Focus

    override func deactivateServer(_ sender: Any!) {
        if let client = sender as? IMKTextInput { endWord(client) }
        LikhonInputController.sharedPanel?.hide()
        super.deactivateServer(sender)
    }

    override func commitComposition(_ sender: Any!) {
        if let client = sender as? IMKTextInput { endWord(client) }
    }
}
