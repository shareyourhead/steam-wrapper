use anyhow::Result;
use futures_util::StreamExt;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::watch;

use crate::engine::Engine;
use crate::input::InputWatcher;

fn spawn_game(command: &[String]) -> Result<Child> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("--wrap requires a command"))?;
    Ok(Command::new(program).args(args).spawn()?)
}

fn is_descendant_of(mut pid: u32, ancestor: u32) -> bool {
    loop {
        if pid == ancestor {
            return true;
        }
        let status = match std::fs::read_to_string(format!("/proc/{}/status", pid)) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let ppid = status.lines()
            .find(|l| l.starts_with("PPid:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if ppid == 0 || ppid == pid {
            return false;
        }
        pid = ppid;
    }
}

// For native Wayland apps that implement the AT-SPI2 accessibility interface.
async fn monitor_atspi(child_pid: u32, tx: Arc<watch::Sender<bool>>) {
    let conn = match zbus::Connection::session().await {
        Ok(c) => c,
        Err(_) => return,
    };

    if let Ok(registry) = zbus::Proxy::new(
        &conn,
        "org.a11y.atspi.Registry",
        "/org/a11y/atspi/registry",
        "org.a11y.atspi.Registry",
    ).await {
        let _: zbus::Result<()> = registry.call("RegisterEvent", &("window:activate",)).await;
    }

    let rule = match zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.a11y.atspi.Event.Window")
    {
        Ok(b) => b.build(),
        Err(_) => return,
    };

    let mut stream = match zbus::MessageStream::for_match_rule(rule, &conn, None).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let Ok(dbus_proxy) = zbus::Proxy::new(
        &conn,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    ).await else { return };

    while let Some(Ok(msg)) = stream.next().await {
        let header = msg.header();
        let Some(member) = header.member() else { continue };
        if member.as_str() != "Activate" { continue }

        let Some(sender) = header.sender() else { continue };
        let sender_str = sender.to_string();

        let Ok(pid): Result<u32, _> = dbus_proxy
            .call("GetConnectionUnixProcessID", &(sender_str.as_str(),))
            .await
        else { continue };

        let _ = tx.send(is_descendant_of(pid, child_pid));
    }
}

// For XWayland/Proton apps: watches _NET_ACTIVE_WINDOW on the XWayland display
// via one persistent xprop process — event-driven, no polling.
// When a native Wayland window is focused, GNOME clears _NET_ACTIVE_WINDOW to 0.
async fn monitor_xwayland(child_pid: u32, tx: Arc<watch::Sender<bool>>) {
    let mut proc = match Command::new("xprop")
        .args(["-spy", "-root", "_NET_ACTIVE_WINDOW"])
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let stdout = match proc.stdout.take() {
        Some(s) => s,
        None => return,
    };

    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        // "_NET_ACTIVE_WINDOW(WINDOW): window id # 0x4e00027"
        let win_id = line.find("0x").and_then(|i| {
            u32::from_str_radix(
                line[i + 2..].split_whitespace().next().unwrap_or(""),
                16,
            ).ok()
        });

        let focused = match win_id {
            None | Some(0) => false,
            Some(id) => get_xwayland_pid(id).await
                .map(|p| is_descendant_of(p, child_pid))
                .unwrap_or(false),
        };

        let _ = tx.send(focused);
    }
}

async fn get_xwayland_pid(win_id: u32) -> Option<u32> {
    let out = Command::new("xprop")
        .args(["-id", &format!("0x{:x}", win_id), "_NET_WM_PID"])
        .output()
        .await
        .ok()?;
    // "_NET_WM_PID(CARDINAL) = 12345"
    let text = std::str::from_utf8(&out.stdout).ok()?;
    text.split('=').nth(1)?.trim().parse().ok()
}

pub async fn run_wrapped(
    command: Vec<String>,
    engine: Arc<Mutex<Engine>>,
    mut stream: evdev::EventStream,
    watchers: &mut Vec<Box<dyn InputWatcher>>,
) -> Result<()> {
    let mut child = spawn_game(&command)?;
    let child_pid = child
        .id()
        .ok_or_else(|| anyhow::anyhow!("Failed to get child PID"))?;

    println!("Launched game (PID {}). Waiting for focus...", child_pid);

    let (focus_tx, mut focus_rx) = watch::channel(false);
    let focus_tx = Arc::new(focus_tx);
    tokio::spawn(monitor_atspi(child_pid, Arc::clone(&focus_tx)));
    tokio::spawn(monitor_xwayland(child_pid, Arc::clone(&focus_tx)));
    drop(focus_tx);

    let mut grabbed = false;

    loop {
        tokio::select! {
            status = child.wait() => {
                println!("[wrapper] Game exited: {}", status?);
                if grabbed {
                    let _ = stream.device_mut().ungrab();
                    engine.lock().unwrap().release_all();
                }
                break;
            }
            result = focus_rx.changed() => {
                if result.is_err() {
                    eprintln!("[wrapper] Warning: focus monitoring stopped — input grab state will not change");
                    if grabbed {
                        let _ = stream.device_mut().ungrab();
                        engine.lock().unwrap().release_all();
                    }
                    let status = child.wait().await;
                    println!("[wrapper] Game exited: {}", status?);
                    break;
                }
                let focused = *focus_rx.borrow_and_update();
                if focused && !grabbed {
                    stream.device_mut().grab()?;
                    grabbed = true;
                    println!("[wrapper] Game focused — mappings active");
                } else if !focused && grabbed {
                    stream.device_mut().ungrab()?;
                    engine.lock().unwrap().release_all();
                    grabbed = false;
                    println!("[wrapper] Game unfocused — mappings suspended");
                }
            }
            event = stream.next_event() => {
                let event = event?;
                if grabbed {
                    for watcher in watchers.iter_mut() {
                        watcher.handle_event(&event);
                    }
                }
            }
        }
    }

    Ok(())
}
