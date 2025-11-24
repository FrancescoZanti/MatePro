// lib.rs - Library entry point for Android and other platforms
// Re-exports the main application functionality

mod agent;
mod mcp_sql;

pub use agent::*;
pub use mcp_sql::*;

// Android entry point
#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use eframe::NativeOptions;
    
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("MatePro"),
    );
    
    let options = NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    
    // For now, just log that the app started
    // Full implementation would require adapting the main app for mobile
    log::info!("MatePro Android started");
}
