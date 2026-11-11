// __APP_TITLE__ iOS host. The whole app is Rust: __APP___run() starts winit's
// UIApplicationMain and never returns while the app lives.
import UIKit

UzorBridge.install()
exit(__APP___run())
