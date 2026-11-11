plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "__PACKAGE__"
    compileSdk = 35
    defaultConfig {
        applicationId = "__PACKAGE__"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
    // The Rust library is built by ../build-android.sh into src/main/jniLibs.
    // No prefab: android-activity compiles its own GameActivity C glue.
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core-ktx:1.13.1")
    // The version android-activity 0.6 (winit 0.30) is built against.
    implementation("androidx.games:games-activity:4.4.0")
}
