package dev.uzor.mobile

import android.app.Activity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

/**
 * What winit does not tell the Rust side on Android: the safe area (system bars
 * and display cutout) and the soft keyboard height, in dp. The JNI names are fixed
 * in uzor-window-mobile (`Java_dev_uzor_mobile_UzorBridge_*`), so this class keeps
 * this package whatever the app's own package is.
 */
object UzorBridge {
    external fun setSafeArea(top: Float, right: Float, bottom: Float, left: Float)
    external fun setKeyboard(height: Float)

    fun attach(activity: Activity) {
        val root = activity.window.decorView
        val dp = activity.resources.displayMetrics.density
        ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
            val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
            setSafeArea(bars.top / dp, bars.right / dp, bars.bottom / dp, bars.left / dp)
            setKeyboard(ime.bottom / dp)
            ViewCompat.onApplyWindowInsets(view, insets)
        }
        ViewCompat.requestApplyInsets(root)
    }

    // Haptics: Rust -> Kotlin needs a JNI call back into the activity; until that
    // lands, uzor_window_mobile::haptics stays a no-op on Android.
}
