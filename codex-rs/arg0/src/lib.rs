use std::fs::File;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;

use codex_core::CODEX_APPLY_PATCH_ARG1;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use tempfile::TempDir;

const LINUX_SANDBOX_ARG0: &str = "codex-linux-sandbox";
const APPLY_PATCH_ARG0: &str = "apply_patch";
const MISSPELLED_APPLY_PATCH_ARG0: &str = "applypatch";
const LOCK_FILENAME: &str = ".lock";

/// Keeps the per-session PATH entry alive and locked for the process lifetime.
pub struct Arg0PathEntryGuard {
    _temp_dir: TempDir,
    _lock_file: File,
}

impl Arg0PathEntryGuard {
    fn new(temp_dir: TempDir, lock_file: File) -> Self {
        Self {
            _temp_dir: temp_dir,
            _lock_file: lock_file,
        }
    }
}

pub fn arg0_dispatch() -> Option<Arg0PathEntryGuard> {
    // Determine if we were invoked via the special alias.
    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap_or_default();
    let exe_name = Path::new(&argv0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if exe_name == LINUX_SANDBOX_ARG0 {
        // Safety: [`run_main`] never returns.
        codex_linux_sandbox::run_main();
    } else if exe_name == APPLY_PATCH_ARG0 || exe_name == MISSPELLED_APPLY_PATCH_ARG0 {
        codex_apply_patch::main();
    }

    let argv1 = args.next().unwrap_or_default();
    if argv1 == CODEX_APPLY_PATCH_ARG1 {
        let patch_arg = args.next().and_then(|s| s.to_str().map(str::to_owned));
        let exit_code = match patch_arg {
            Some(patch_arg) => {
                let mut stdout = std::io::stdout();
                let mut stderr = std::io::stderr();
                match codex_apply_patch::apply_patch(&patch_arg, &mut stdout, &mut stderr) {
                    Ok(()) => 0,
                    Err(_) => 1,
                }
            }
            None => {
                eprintln!("Error: {CODEX_APPLY_PATCH_ARG1} requires a UTF-8 PATCH argument.");
                1
            }
        };
        std::process::exit(exit_code);
    }

    // This modifies the environment, which is not thread-safe, so do this
    // before creating any threads/the Tokio runtime.
    load_dotenv();

    match prepend_path_entry_for_codex_aliases() {
        Ok(path_entry) => Some(path_entry),
        Err(err) => {
            // It is possible that Codex will proceed successfully even if
            // updating the PATH fails, so warn the user and move on.
            eprintln!("WARNING: proceeding, even though we could not update PATH: {err}");
            None
        }
    }
}

/// While we want to deploy the Codex CLI as a single executable for simplicity,
/// we also want to expose some of its functionality as distinct CLIs, so we use
/// the "arg0 trick" to determine which CLI to dispatch. This effectively allows
/// us to simulate deploying multiple executables as a single binary on Mac and
/// Linux (but not Windows).
///
/// When the current executable is invoked through the hard-link or alias named
/// `codex-linux-sandbox` we *directly* execute
/// [`codex_linux_sandbox::run_main`] (which never returns). Otherwise we:
///
/// 1.  Load `.env` values before creating any threads.
/// 2.  Construct a Tokio multi-thread runtime.
/// 3.  Derive the path to the current executable (so children can re-invoke the
///     sandbox) when running on Linux.
/// 4.  Execute the provided async `main_fn` inside that runtime, forwarding any
///     error. Note that `main_fn` receives `codex_linux_sandbox_exe:
///     Option<PathBuf>`, as an argument, which is generally needed as part of
///     constructing [`codex_core::config::Config`].
///
/// This function should be used to wrap any `main()` function in binary crates
/// in this workspace that depends on these helper CLIs.
pub fn arg0_dispatch_or_else<F, Fut>(main_fn: F) -> anyhow::Result<()>
where
    F: FnOnce(Option<PathBuf>) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    // Retain the TempDir so it exists for the lifetime of the invocation of
    // this executable. Admittedly, we could invoke `keep()` on it, but it
    // would be nice to avoid leaving temporary directories behind, if possible.
    let _path_entry = arg0_dispatch();

    // Regular invocation – create a Tokio runtime and execute the provided
    // async entry-point.
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let codex_linux_sandbox_exe: Option<PathBuf> = if cfg!(target_os = "linux") {
            std::env::current_exe().ok()
        } else {
            None
        };

        main_fn(codex_linux_sandbox_exe).await
    })
}

const ILLEGAL_ENV_VAR_PREFIX: &str = "CODEX_";

/// Load env vars from `.env` files.
///
/// Security: Do not allow `.env` files to create or modify any variables
/// with names starting with `CODEX_`.
fn load_dotenv() {
    let codex_home = codex_core::config::find_codex_home().ok();
    let cwd_env = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join(".codex").join(".env"));
    if let Some(cwd_env) = cwd_env
        && let Ok(iter) = dotenvy::from_path_iter(&cwd_env)
    {
        // Codez policy: if `cwd/.codex/.env` exists, ignore `$CODEX_HOME/.env`.
        set_filtered(iter);
        return;
    }

    if let Some(codex_home) = codex_home
        && let Ok(iter) = dotenvy::from_path_iter(codex_home.join(".env"))
    {
        set_filtered(iter);
    }
}

/// Helper to set vars from a dotenvy iterator while filtering out `CODEX_` keys.
fn set_filtered<I>(iter: I)
where
    I: IntoIterator<Item = Result<(String, String), dotenvy::Error>>,
{
    for (key, value) in iter.into_iter().flatten() {
        if !key.to_ascii_uppercase().starts_with(ILLEGAL_ENV_VAR_PREFIX) {
            // It is safe to call set_var() because our process is
            // single-threaded at this point in its execution.
            unsafe { std::env::set_var(&key, &value) };
        }
    }
}

/// Creates a temporary directory with either:
///
/// - UNIX: `apply_patch` symlink to the current executable
/// - WINDOWS: `apply_patch.bat` batch script to invoke the current executable
///   with the "secret" --codex-run-as-apply-patch flag.
///
/// This temporary directory is prepended to the PATH environment variable so
/// that `apply_patch` can be on the PATH without requiring the user to
/// install a separate `apply_patch` executable, simplifying the deployment of
/// Codex CLI.
/// Note: In debug builds the temp-dir guard is disabled to ease local testing.
///
/// IMPORTANT: This function modifies the PATH environment variable, so it MUST
/// be called before multiple threads are spawned.
pub fn prepend_path_entry_for_codex_aliases() -> std::io::Result<Arg0PathEntryGuard> {
    let codex_home = codex_core::config::find_codex_home()?;
    #[cfg(not(debug_assertions))]
    {
        // Guard against placing helpers in system temp directories outside debug builds.
        let temp_root = std::env::temp_dir();
        if codex_home.starts_with(&temp_root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Refusing to create helper binaries under temporary dir {temp_root:?} (codex_home: {codex_home:?})"
                ),
            ));
        }
    }

    std::fs::create_dir_all(&codex_home)?;
    // Use a CODEX_HOME-scoped temp root to avoid cluttering the top-level directory.
    let temp_root = codex_home.join("tmp").join("arg0");
    std::fs::create_dir_all(&temp_root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        // Ensure only the current user can access the temp directory.
        std::fs::set_permissions(&temp_root, std::fs::Permissions::from_mode(0o700))?;
    }

    // Best-effort cleanup of stale per-session dirs. Ignore failures so startup proceeds.
    if let Err(err) = janitor_cleanup(&temp_root) {
        eprintln!("WARNING: failed to clean up stale arg0 temp dirs: {err}");
    }

    let temp_dir = tempfile::Builder::new()
        .prefix("codex-arg0")
        .tempdir_in(&temp_root)?;
    let path = temp_dir.path();

    let lock_path = path.join(LOCK_FILENAME);
    let lock_file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    lock_file.try_lock()?;

    for filename in &[
        APPLY_PATCH_ARG0,
        MISSPELLED_APPLY_PATCH_ARG0,
        #[cfg(target_os = "linux")]
        LINUX_SANDBOX_ARG0,
    ] {
        let exe = std::env::current_exe()?;

        #[cfg(unix)]
        {
            let link = path.join(filename);
            symlink(&exe, &link)?;
        }

        #[cfg(windows)]
        {
            let batch_script = path.join(format!("{filename}.bat"));
            std::fs::write(
                &batch_script,
                format!(
                    r#"@echo off
"{}" {CODEX_APPLY_PATCH_ARG1} %*
"#,
                    exe.display()
                ),
            )?;
        }
    }

    #[cfg(unix)]
    const PATH_SEPARATOR: &str = ":";

    #[cfg(windows)]
    const PATH_SEPARATOR: &str = ";";

    let path_element = path.display();
    let updated_path_env_var = match std::env::var("PATH") {
        Ok(existing_path) => {
            format!("{path_element}{PATH_SEPARATOR}{existing_path}")
        }
        Err(_) => {
            format!("{path_element}")
        }
    };

    unsafe {
        std::env::set_var("PATH", updated_path_env_var);
    }

    Ok(Arg0PathEntryGuard::new(temp_dir, lock_file))
}

fn janitor_cleanup(temp_root: &Path) -> std::io::Result<()> {
    let entries = match std::fs::read_dir(temp_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip the directory if locking fails or the lock is currently held.
        let Some(_lock_file) = try_lock_dir(&path)? else {
            continue;
        };

        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            // Expected TOCTOU race: directory can disappear after read_dir/lock checks.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

fn try_lock_dir(dir: &Path) -> std::io::Result<Option<File>> {
    let lock_path = dir.join(LOCK_FILENAME);
    let lock_file = match File::options().read(true).write(true).open(&lock_path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };

    match lock_file.try_lock() {
        Ok(()) => Ok(Some(lock_file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::ILLEGAL_ENV_VAR_PREFIX;
    use super::LOCK_FILENAME;
    use super::janitor_cleanup;
    use super::load_dotenv;
    use std::env;
    use std::fs;
    use std::fs::File;
    use std::path::Path;
    use std::sync::Mutex;
    use std::sync::OnceLock;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: String,
        prev: Option<String>,
    }

    impl EnvVarGuard {
        fn new(key: &str) -> Self {
            Self {
                key: key.to_string(),
                prev: env::var(key).ok(),
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.prev.as_deref() {
                Some(value) => {
                    // SAFETY: tests in this module hold ENV_LOCK while mutating env.
                    unsafe { env::set_var(&self.key, value) }
                }
                None => {
                    // SAFETY: tests in this module hold ENV_LOCK while mutating env.
                    unsafe { env::remove_var(&self.key) }
                }
            }
        }
    }

    struct CwdGuard {
        prev: std::path::PathBuf,
    }

    impl CwdGuard {
        fn new() -> std::io::Result<Self> {
            Ok(Self {
                prev: env::current_dir()?,
            })
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.prev);
        }
    }

    fn create_lock(dir: &Path) -> std::io::Result<File> {
        let lock_path = dir.join(LOCK_FILENAME);
        File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
    }

    #[test]
    fn janitor_skips_dirs_without_lock_file() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        let dir = root.path().join("no-lock");
        fs::create_dir(&dir)?;

        janitor_cleanup(root.path())?;

        assert!(dir.exists());
        Ok(())
    }

    #[test]
    fn janitor_skips_dirs_with_held_lock() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        let dir = root.path().join("locked");
        fs::create_dir(&dir)?;
        let lock_file = create_lock(&dir)?;
        lock_file.try_lock()?;

        janitor_cleanup(root.path())?;

        assert!(dir.exists());
        Ok(())
    }

    #[test]
    fn janitor_removes_dirs_with_unlocked_lock() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        let dir = root.path().join("stale");
        fs::create_dir(&dir)?;
        create_lock(&dir)?;

        janitor_cleanup(root.path())?;

        assert!(!dir.exists());
        Ok(())
    }

    #[test]
    fn load_dotenv_prefers_cwd_codex_env_over_codex_home_env() -> std::io::Result<()> {
        let _lock = env_lock().lock().expect("lock env");
        let root = tempfile::tempdir()?;
        let codex_home = root.path().join("home");
        let cwd = root.path().join("cwd");
        fs::create_dir_all(codex_home.as_path())?;
        fs::create_dir_all(cwd.join(".codex"))?;

        fs::write(codex_home.join(".env"), "TEST_ENV_ORIGIN=home\n")?;
        fs::write(cwd.join(".codex").join(".env"), "TEST_ENV_ORIGIN=cwd\n")?;

        let _codex_home_guard = EnvVarGuard::new("CODEX_HOME");
        let _origin_guard = EnvVarGuard::new("TEST_ENV_ORIGIN");
        let _cwd_guard = CwdGuard::new()?;

        // SAFETY: tests in this module hold ENV_LOCK while mutating env.
        unsafe {
            env::set_var("CODEX_HOME", codex_home.as_os_str());
            env::remove_var("TEST_ENV_ORIGIN");
        }
        env::set_current_dir(cwd)?;

        load_dotenv();

        assert_eq!(env::var("TEST_ENV_ORIGIN").ok().as_deref(), Some("cwd"));
        Ok(())
    }

    #[test]
    fn load_dotenv_filters_codex_prefixed_keys() -> std::io::Result<()> {
        let _lock = env_lock().lock().expect("lock env");
        let root = tempfile::tempdir()?;
        let codex_home = root.path().join("home");
        fs::create_dir_all(codex_home.as_path())?;
        fs::write(
            codex_home.join(".env"),
            "SAFE_KEY=ok\nCODEX_FORBIDDEN=1\ncodex_lower=1\n",
        )?;

        let _codex_home_guard = EnvVarGuard::new("CODEX_HOME");
        let _safe_guard = EnvVarGuard::new("SAFE_KEY");
        let _forbidden_guard = EnvVarGuard::new("CODEX_FORBIDDEN");
        let _lower_guard = EnvVarGuard::new("codex_lower");

        // SAFETY: tests in this module hold ENV_LOCK while mutating env.
        unsafe {
            env::set_var("CODEX_HOME", codex_home.as_os_str());
            env::remove_var("SAFE_KEY");
            env::remove_var("CODEX_FORBIDDEN");
            env::remove_var("codex_lower");
        }

        load_dotenv();

        assert_eq!(env::var("SAFE_KEY").ok().as_deref(), Some("ok"));
        assert_eq!(env::var("CODEX_FORBIDDEN").ok().as_deref(), None);
        assert_eq!(env::var("codex_lower").ok().as_deref(), None);
        assert_eq!(ILLEGAL_ENV_VAR_PREFIX, "CODEX_");
        Ok(())
    }
}
