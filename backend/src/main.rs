fn main() {
    println!("Kamish OS v{}", env!("CARGO_PKG_VERSION"));
    println!("Available services:");
    println!("  - kamish-core: System hardware, audio, speech and scheduler daemon");
    println!("  - kamish-web:  Web panel backend (port 8080) and IPC client");
}
