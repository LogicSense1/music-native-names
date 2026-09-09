//! Tiny Objective-C++ bridge is JIT compiled in Music by a short-lived system LLDB.
//! All daemon, menu, HTTP, cache and log handling is Rust. No Python helper is used.
use crate::App;
use serde_json::json;
use std::{
    fs,
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
fn declarations(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut quote = false;
    let mut escaped = false;
    for (i, c) in content.char_indices() {
        if quote {
            if escaped {
                escaped = false
            } else if c == '\\' {
                escaped = true
            } else if c == '"' {
                quote = false
            }
        } else if c == '"' {
            quote = true
        } else if c == '{' {
            depth += 1
        } else if c == '}' {
            depth -= 1;
            if depth == 0 && !content[i + 1..].trim_start().starts_with(';') {
                out.push(content[start..i + 1].into());
                start = i + 1;
            }
        } else if c == ';' && depth == 0 {
            out.push(content[start..i + 1].into());
            start = i + 1;
        }
    }
    if !content[start..].trim().is_empty() {
        out.push(content[start..].into());
    }
    out
}
fn control_expr(name: &str, selector: &str) -> String {
    format!("((unsigned long(*)(id,SEL))method_getImplementation(class_getClassMethod((Class)objc_getClass({name:?}),sel_registerName({selector:?}))))((id)objc_getClass({name:?}),sel_registerName({selector:?}))")
}
pub fn control(app: &Arc<App>, action: &str, pid: i32) -> bool {
    if pid <= 0 || crate::platform::music_pid() != pid {
        return false;
    }
    if action == "install" {
        let sip = Command::new("/usr/bin/csrutil").arg("status").output();
        if !sip.is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("status: disabled")) {
            app.log("bridge_failed",json!({"reason":"current system integrity configuration does not permit runtime installation; no settings changed"}));
            return false;
        }
    }
    let sdk = Command::new("/usr/bin/xcrun")
        .arg("--show-sdk-path")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let Some(sdk) = sdk else { return false };
    let mut script=format!("settings set target.sdk-path {sdk}\nsettings set target.clang-module-search-paths {sdk}/System/Library/Frameworks\nexpression -l objc++ -- @import Foundation; @import ObjectiveC; @import Darwin;\n");
    if action == "install" {
        for source in [
            include_str!("../native/LiveNamesHook.mm"),
            include_str!("../native/DurationPolicy.h"),
            include_str!("../native/ModelNamesHook.mm"),
        ] {
            for chunk in declarations(source) {
                script.push_str("expression -p -l objc++ -- ");
                script.push_str(&chunk.replace('\n', " "));
                script.push('\n');
            }
        }
        // Retire 1.0.0/1.0.1 getters before installing V2 in the same Music process.
        let legacy = control_expr("ONMusicModelNames", "removeProbe");
        script.push_str(&format!("expression -l objc++ -- (unsigned long)(objc_getClass(\"ONMusicModelNames\") ? {legacy} : 0)\n"));
        let live = control_expr("ONMusicLiveNames", "enableProbe");
        let model = control_expr("ONMusicModelNamesV2", "enableProbe");
        let path = app
            .state
            .join("display-cache.json")
            .to_string_lossy()
            .to_string();
        script.push_str(&format!("expression -l objc++ -- (int)((objc_getClass(\"ONMusicLiveNames\") ? {live} : ONVInstall()) == 1 && (objc_getClass(\"ONMusicModelNamesV2\") ? {model} : ONMInstall({path:?})) == 1 ? 919081 : -919081)\n"));
    } else {
        for name in ["ONMusicLiveNames", "ONMusicModelNamesV2"] {
            let selector = match action {
                "remove" => "removeProbe",
                "enable" => "enableProbe",
                _ => "isInstalled",
            };
            let expr = control_expr(name, selector);
            script.push_str(&format!(
                "expression -l objc++ -- (unsigned long)(objc_getClass({name:?}) ? {expr} : 0)\n"
            ));
            if action == "status" {
                for sel in ["parseCount", "targetCount"] {
                    let expr = control_expr(name, sel);
                    script.push_str(&format!("expression -l objc++ -- (unsigned long)(objc_getClass({name:?}) ? {expr} : 0)\n"));
                }
            }
        }
        let checks = ["ONMusicLiveNames", "ONMusicModelNamesV2"]
            .iter()
            .map(|name| {
                if action == "enable" {
                    format!(
                        "(objc_getClass({name:?}) && {} == 1)",
                        control_expr(name, "isInstalled")
                    )
                } else if action == "remove" {
                    format!(
                        "(!objc_getClass({name:?}) || {} == 0)",
                        control_expr(name, "isInstalled")
                    )
                } else {
                    "true".into()
                }
            })
            .collect::<Vec<_>>()
            .join(" && ");
        script.push_str(&format!(
            "expression -l objc++ -- (int)({checks} ? 919081 : -919081)\n"
        ));
    }
    script.push_str("process detach\nquit\n");
    let commandfile = app.state.join("bridge.commands");
    let outputfile = app.state.join("bridge-output.tmp");
    if fs::write(&commandfile, script).is_err() {
        return false;
    }
    let Ok(output) = fs::File::create(&outputfile) else {
        return false;
    };
    let Ok(errors) = output.try_clone() else {
        return false;
    };
    let start = Instant::now();
    let spawned = Command::new("/usr/bin/xcrun")
        .args(["lldb", "-x", "--batch", "-p", &pid.to_string(), "-s"])
        .arg(&commandfile)
        .args(["-k", "process detach", "-k", "quit"])
        .stdin(Stdio::null())
        .stdout(output)
        .stderr(errors)
        .spawn();
    let mut timed_out = false;
    let okay = if let Ok(mut child) = spawned {
        loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.success(),
                Err(_) => break false,
                _ => {
                    if start.elapsed() > Duration::from_secs(45) {
                        let _ = child.kill();
                        let _ = child.wait();
                        timed_out = true;
                        break false;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    } else {
        false
    };
    let text = fs::read_to_string(&outputfile).unwrap_or_default();
    let _ = fs::remove_file(commandfile);
    let _ = fs::remove_file(outputfile);
    // Evaluate a numeric marker: target stdout (puts) is not captured by LLDB.
    let success =
        okay && text.lines().any(|l| {
            l.starts_with("(int)")
                && l.rsplit_once('=')
                    .is_some_and(|(_, v)| v.trim() == "919081")
        }) && text.contains("detached");
    app.log("bridge",json!({"action":action,"pid":pid,"success":success,"timedOut":timed_out,"ms":start.elapsed().as_millis()}));
    if action == "status" || !success {
        let lines: Vec<&str> = text
            .lines()
            .filter(|l| !l.starts_with("(lldb)") && !l.starts_with("expression "))
            .rev()
            .take(24)
            .collect();
        app.log("bridge_diagnostics", json!({"lines":lines}));
        if action == "status" {
            println!("{text}");
        }
    }
    success
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splits_without_breaking_nested_declarations() {
        let v = declarations("void f(){const char*s=\";}\";if(1){}}\nstruct X{int a;};\nint k;");
        assert_eq!(v.len(), 3);
    }
}
