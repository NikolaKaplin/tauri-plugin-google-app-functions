import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.plugin.google_app_functions"
    compileSdk = 37

    defaultConfig {
        minSdk = 24 // androidx.appfunctions requires 24

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

dependencies {
    // Supplies the app_functions_schema.xsd asset and the matching manifest property.
    implementation("androidx.appfunctions:appfunctions:1.0.0-alpha12")
    implementation("androidx.core:core-ktx:1.9.0")
    implementation("androidx.appcompat:appcompat:1.6.0")
    implementation("com.google.android.material:material:1.7.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    implementation(project(":tauri-android"))
}

// tauri-plugin-google-app-functions-build writes the AppFunctions schema and app description
// into the app module from build.rs, which runs in the `rustBuild*` tasks of the Tauri Gradle
// plugin (id "rust"). The app must merge its assets and resources after them.
rootProject.allprojects {
    pluginManager.withPlugin("rust") {
        val rustBuild = tasks.matching { it.name.startsWith("rustBuild") }
        tasks.matching {
            it.name.startsWith("merge") &&
                (it.name.endsWith("Assets") || it.name.endsWith("Resources"))
        }.configureEach { mustRunAfter(rustBuild) }
    }
}
