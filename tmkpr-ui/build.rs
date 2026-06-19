fn main() {
    let git_hash = jj_commit_hash()
        .or_else(git_commit_hash)
        .unwrap_or_else(|| "unknown".to_string());

    let build_date = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%d %H:%M UTC"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
}

fn jj_commit_hash() -> Option<String> {
    let out = std::process::Command::new("jj")
        .args(["log", "--no-graph", "-r", "@", "--template", "commit_id.short(7)"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn git_commit_hash() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}
