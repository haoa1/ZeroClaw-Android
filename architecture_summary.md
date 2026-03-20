# ZeroClaw-Android Architecture Overview

## Project Structure
```
ZeroClaw-Android/
├── app/                    # Android application module (Kotlin/Compose)
│   ├── src/main/java/com/zeroclaw/android/
│   │   ├── ui/            # Compose UI components and screens
│   │   ├── viewmodel/     # ViewModels for UI state management
│   │   ├── navigation/    # Navigation graph and routing
│   │   ├── service/       # Android services (foreground service)
│   │   ├── data/          # Data layer (repositories, database, remote)
│   │   ├── model/         # Data classes and entities
│   │   └── util/          # Utilities and extensions
│   └── src/main/res/      # Android resources
├── lib/                    # Library module for AAR publishing
├── zeroclaw/              # Rust core engine (git submodule)
│   ├── src/              # Rust source code
│   ├── crates/           # Internal Rust crates
│   └── Cargo.toml        # Rust dependencies
└── zeroclaw-android/     # FFI bridge and Android-specific Rust code
    ├── zeroclaw-ffi/     # UniFFI facade and bindings
    └── Cargo.toml        # Android-specific Rust dependencies
```

## Architecture Layers

### 1. Presentation Layer (Kotlin/Compose)
- **UI Components**: Composable functions for all screens
- **Navigation**: Jetpack Navigation with custom NavHost
- **ViewModels**: State management using Android Architecture Components
- **Theming**: Material 3 theming system

### 2. Domain Layer (Kotlin)
- **Use Cases**: Business logic orchestration
- **Repositories**: Data access abstraction
- **Models**: Domain entities and data classes

### 3. Data Layer (Kotlin)
- **Local Database**: Room with 7+ DAOs for persistent storage
- **Preferences**: DataStore for settings, EncryptedSharedPreferences for secrets
- **Remote Data**: WebSocket channels, OAuth, API integration
- **Security**: AES-256-GCM encryption via Android Keystore

### 4. FFI Bridge (UniFFI)
- **34 FFI Functions**: Bidirectional Kotlin↔Rust communication
- **Error Handling**: `catch_unwind` to prevent Rust panics from crashing JVM
- **Type Marshaling**: Automatic type conversion between Kotlin and Rust

### 5. Core Engine (Rust)
- **AI Agent Gateway**: Message processing and conversation management
- **Rhai Scripting Engine**: Terminal REPL with slash commands
- **Provider Integration**: 32+ AI providers (OpenAI, Claude, Gemini, etc.)
- **Plugin System**: Extensible tool system (web search, code execution, etc.)
- **Async Runtime**: Tokio multi-thread runtime for concurrent operations

### 6. External Integrations
- **AI Providers**: 32+ supported providers via unified API
- **Android System**: Foreground service, notifications, battery optimization
- **Security**: Hardware-backed encryption (StrongBox where available)

## Data Flow

```
User Interaction → UI Layer → ViewModels → Repositories → FFI Bridge → Rust Core → AI Providers
        ↑              ↑          ↑            ↑             ↑            ↑           ↑
    Rendering      State      Business      Data        Type        AI        API
                   Updates     Logic      Access     Conversion   Processing   Calls
```

## Key Architectural Decisions

1. **Rust Core with Kotlin UI**: Leverages Rust's performance and safety for AI processing while using Kotlin/Compose for native Android UI.

2. **UniFFI over JNI**: Uses Mozilla's UniFFI for safer and more maintainable FFI compared to raw JNI.

3. **Foreground Service**: Runs as Android foreground service for 24/7 operation with battery optimization.

4. **Hardware-backed Security**: Uses Android Keystore with StrongBox for secure API key storage.

5. **Plugin Architecture**: Extensible system for adding new capabilities without modifying core.

6. **REPL Interface**: Rhai scripting engine provides powerful terminal interface for advanced users.

## Dependencies

### Kotlin Dependencies
- Jetpack Compose (UI)
- Room (Database)
- DataStore (Preferences)
- WorkManager (Background tasks)
- Koin or Hilt (Dependency Injection)

### Rust Dependencies
- Tokio (Async runtime)
- Serde (Serialization)
- Reqwest (HTTP client)
- Rhai (Scripting)
- Various AI provider SDKs

## Build System
- **Gradle Kotlin DSL**: Modern build configuration
- **Gobley Plugin**: Automates Rust cross-compilation via cargo-ndk
- **UniFFI Kotlin**: Generates Kotlin bindings from Rust code
- **KSP**: Kotlin Symbol Processing for code generation

## Generated Architecture Diagrams

1. `architecture.svg` - High-level overview of the architecture layers
2. `architecture_detailed.svg` - Detailed component-level architecture
3. This document (`architecture_summary.md`) - Textual architecture description

## Viewing the Diagrams

Open the SVG files in any modern web browser or SVG viewer. For best results, use Chrome, Firefox, or Edge.

## Next Steps

1. Review the generated diagrams for accuracy
2. Update diagrams as the architecture evolves
3. Consider adding sequence diagrams for key workflows
4. Document API boundaries and contracts between layers