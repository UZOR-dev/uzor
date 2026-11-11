package __PACKAGE__

import android.os.Bundle
import androidx.core.view.WindowCompat
import com.google.androidgamesdk.GameActivity
import dev.uzor.mobile.UzorBridge

/** __APP_TITLE__ Android host. The whole app is Rust (`android_main` in lib__APP__.so). */
class MainActivity : GameActivity() {
    companion object {
        init {
            System.loadLibrary("__APP__")
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        // Edge to edge: the app draws under the bars and pads itself by the insets.
        WindowCompat.setDecorFitsSystemWindows(window, false)
        super.onCreate(savedInstanceState)
        UzorBridge.attach(this)
    }
}
