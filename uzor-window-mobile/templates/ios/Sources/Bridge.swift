// What winit does not tell the Rust side: the soft keyboard's height, and the
// haptics performer. (The safe area comes from winit itself.)
import UIKit

enum UzorBridge {
    static func install() {
        let nc = NotificationCenter.default
        nc.addObserver(forName: UIResponder.keyboardWillChangeFrameNotification, object: nil, queue: .main) { note in
            guard let frame = note.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect else { return }
            let screen = UIScreen.main.bounds
            // Points of the keyboard over the bottom of the screen (0 when it is off screen).
            uzor_mobile_set_keyboard(Double(max(0, screen.maxY - frame.minY)))
        }
        nc.addObserver(forName: UIResponder.keyboardWillHideNotification, object: nil, queue: .main) { _ in
            uzor_mobile_set_keyboard(0)
        }
        uzor_mobile_set_haptics { kind in
            // Called on the main thread (the Rust event loop is UIKit's).
            switch kind {
            case 0: UISelectionFeedbackGenerator().selectionChanged()
            case 1: UIImpactFeedbackGenerator(style: .light).impactOccurred()
            case 2: UIImpactFeedbackGenerator(style: .medium).impactOccurred()
            case 3: UIImpactFeedbackGenerator(style: .heavy).impactOccurred()
            case 4: UINotificationFeedbackGenerator().notificationOccurred(.success)
            case 5: UINotificationFeedbackGenerator().notificationOccurred(.warning)
            case 6: UINotificationFeedbackGenerator().notificationOccurred(.error)
            default: break
            }
        }
    }
}
