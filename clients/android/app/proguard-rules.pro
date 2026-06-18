# Slipstream ProGuard Rules

# Keep the Native Bridge and its methods for JNI
-keep class io.unom.slipstream.kit.NativeBridge { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep the models that might be serialized or accessed via JNI
-keep class io.unom.slipstream.models.** { *; }
-keep class io.unom.slipstream.kit.discovery.** { *; }
-keep class io.unom.slipstream.kit.security.** { *; }

# Compose rules are usually handled by the plugin, but we can add more if needed
-keepclassmembers class **.R$* {
    public static <fields>;
}
