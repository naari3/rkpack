mod cli;
mod core;
mod gui;

/// 親プロセスがGUIシェル（explorer.exe / launchd）かどうかを判定する
fn launched_from_gui() -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    let pid = Pid::from_u32(std::process::id());

    let parent_name = sys
        .process(pid)
        .and_then(|p| p.parent())
        .and_then(|ppid| sys.process(ppid))
        .map(|parent| parent.name().to_string_lossy().to_lowercase());

    match parent_name.as_deref() {
        Some("explorer.exe") => true, // Windows: エクスプローラーから起動
        Some("launchd") => true,      // macOS: Finder/Dock/Spotlightから起動
        _ => false,
    }
}

fn main() -> anyhow::Result<()> {
    if std::env::args().len() <= 1 && launched_from_gui() {
        gui::run_gui()
    } else {
        cli::run_cli()
    }
}
