buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.3.1")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.2.10")
        // KSP for AppFunctions; its version must match the Kotlin version above.
        classpath("com.google.devtools.ksp:com.google.devtools.ksp.gradle.plugin:2.2.10-2.0.2")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

tasks.register("clean").configure {
    delete("build")
}

