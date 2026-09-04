//! Self-contained Herdr auto-update configuration, runner, and service adapters.
//!
//! The updater deliberately owns only a marker-delimited pair of Herdr TOML
//! settings. Binary replacement and service installation are transactional:
//! failures restore the prior bytes and modes before returning an error.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sha2::{Digest, Sha256};
use toml_edit::{value, DocumentMut, Item, Table};

pub const BLOCK_START: &str = "# cmux-herdr:herdr-auto-update:start";
pub const BLOCK_END: &str = "# cmux-herdr:herdr-auto-update:end";
pub const LOCK_NAME: &str = "herdr-auto-update.lock";
pub const BACKUP_LIMIT: usize = 3;
const LABEL: &str = "com.cmux-herdr.herdr-auto-update";
const RUNTIME_NAME: &str = "cmux-herdr-update-service";

pub type Result<T, E = UpdateError> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum UpdateError {
    Config(String),
    Io {
        context: String,
        source: io::Error,
    },
    Command {
        program: String,
        status: i32,
        stderr: String,
    },
    UpdateFailed {
        status: i32,
        restored: bool,
    },
    InvalidReplacement {
        reason: String,
        restored: bool,
    },
}

impl UpdateError {
    fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => f.write_str(message),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Command {
                program,
                status,
                stderr,
            } => {
                write!(f, "{program} exited with status {status}")?;
                if !stderr.trim().is_empty() {
                    write!(f, ": {}", stderr.trim())?;
                }
                Ok(())
            }
            Self::UpdateFailed { status, restored } => write!(
                f,
                "herdr update failed with status {status}{}",
                if *restored {
                    "; previous binary restored"
                } else {
                    ""
                },
            ),
            Self::InvalidReplacement { reason, restored } => write!(
                f,
                "invalid Herdr replacement: {reason}{}",
                if *restored {
                    "; previous binary restored"
                } else {
                    ""
                },
            ),
        }
    }
}

impl std::error::Error for UpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn parse_document(text: &str) -> Result<DocumentMut> {
    text.parse::<DocumentMut>()
        .map_err(|error| UpdateError::Config(format!("invalid Herdr config: {error}")))
}

struct ManagedBlock {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TomlStringState {
    Normal,
    MultilineBasic,
    MultilineLiteral,
}

fn toml_comment_line_mask(text: &str) -> Vec<bool> {
    let mut state = TomlStringState::Normal;
    let mut result = Vec::new();
    for line in text.split_inclusive('\n') {
        result.push(state == TomlStringState::Normal);
        let bytes = line.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match state {
                TomlStringState::Normal => match bytes[index] {
                    b'#' => break,
                    b'\'' if bytes.get(index..index + 3) == Some(b"'''") => {
                        state = TomlStringState::MultilineLiteral;
                        index += 3;
                    }
                    b'"' if bytes.get(index..index + 3) == Some(b"\"\"\"") => {
                        state = TomlStringState::MultilineBasic;
                        index += 3;
                    }
                    b'\'' => {
                        index += 1;
                        while index < bytes.len() && bytes[index] != b'\'' {
                            index += 1;
                        }
                        index += usize::from(index < bytes.len());
                    }
                    b'"' => {
                        index += 1;
                        while index < bytes.len() {
                            if bytes[index] == b'\\' {
                                index = (index + 2).min(bytes.len());
                            } else if bytes[index] == b'"' {
                                index += 1;
                                break;
                            } else {
                                index += 1;
                            }
                        }
                    }
                    _ => index += 1,
                },
                TomlStringState::MultilineLiteral => {
                    if bytes.get(index..index + 3) == Some(b"'''") {
                        state = TomlStringState::Normal;
                        index += 3;
                    } else {
                        index += 1;
                    }
                }
                TomlStringState::MultilineBasic => {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes.get(index..index + 3) == Some(b"\"\"\"") {
                        state = TomlStringState::Normal;
                        index += 3;
                    } else {
                        index += 1;
                    }
                }
            }
        }
    }
    result
}

fn managed_block(text: &str) -> Result<Option<ManagedBlock>> {
    parse_document(text)?;
    let comment_lines = toml_comment_line_mask(text);
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut offset = 0;
    let mut lines = Vec::new();
    for (line_index, segment) in text.split_inclusive('\n').enumerate() {
        let without_newline = segment.strip_suffix('\n').unwrap_or(segment);
        let content = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        let end = offset + segment.len();
        lines.push((offset, end, content));
        let is_comment = comment_lines.get(line_index).copied().unwrap_or(true);
        if is_comment && content == BLOCK_START {
            starts.push(lines.len() - 1);
        }
        if is_comment && content == BLOCK_END {
            ends.push(lines.len() - 1);
        }
        if is_comment
            && content.contains("cmux-herdr:herdr-auto-update:")
            && content != BLOCK_START
            && content != BLOCK_END
        {
            return Err(UpdateError::Config(
                "malformed cmux-herdr auto-update marker".into(),
            ));
        }
        offset = end;
    }
    if starts.is_empty() && ends.is_empty() {
        return Ok(None);
    }
    if starts.len() != 1 || ends.len() != 1 || starts[0] >= ends[0] {
        return Err(UpdateError::Config(
            "malformed cmux-herdr auto-update marker block".into(),
        ));
    }
    let start_line = starts[0];
    let end_line = ends[0];
    let active_header = lines[..start_line]
        .iter()
        .rev()
        .map(|(_, _, line)| line.trim())
        .find(|line| line.starts_with('['));
    if active_header
        .and_then(|line| line.split('#').next())
        .map(str::trim)
        != Some("[update]")
    {
        return Err(UpdateError::Config(
            "managed block is not inside [update]".into(),
        ));
    }
    let body = lines[start_line + 1..end_line]
        .iter()
        .map(|(_, _, line)| *line)
        .collect::<Vec<_>>();
    if body.is_empty() || body.iter().any(|line| line.trim().is_empty()) {
        return Err(UpdateError::Config("invalid managed block contents".into()));
    }
    let synthetic = format!("[update]\n{}\n", body.join("\n"));
    let parsed = parse_document(&synthetic)?;
    let table = parsed
        .get("update")
        .and_then(Item::as_table)
        .ok_or_else(|| UpdateError::Config("invalid managed block contents".into()))?;
    if table.len() != body.len() || table.is_empty() || table.len() > 2 {
        return Err(UpdateError::Config("invalid managed block contents".into()));
    }
    let allowed = HashSet::from(["channel", "manifest_url"]);
    for (key, item) in table.iter() {
        if !allowed.contains(key) || item.as_str().is_none() {
            return Err(UpdateError::Config("invalid managed block contents".into()));
        }
    }
    Ok(Some(ManagedBlock {
        start: lines[start_line].0,
        end: lines[end_line].1,
    }))
}

fn remove_managed_block(text: &str) -> Result<String> {
    let Some(block) = managed_block(text)? else {
        return Ok(text.to_owned());
    };
    let mut result = String::with_capacity(text.len() - (block.end - block.start));
    result.push_str(&text[..block.start]);
    result.push_str(&text[block.end..]);
    parse_document(&result)?;
    Ok(result)
}

/// Add the requested channel and HTTPS manifest without overwriting user-owned values.
pub fn install_settings(text: &str, channel: &str, manifest_url: &str) -> Result<String> {
    if !manifest_url.starts_with("https://") || manifest_url.len() == "https://".len() {
        return Err(UpdateError::Config("manifest URL must use https://".into()));
    }
    if channel.trim().is_empty() {
        return Err(UpdateError::Config(
            "update channel must not be empty".into(),
        ));
    }
    let clean = remove_managed_block(text)?;
    let mut document = parse_document(&clean)?;
    if let Some(item) = document.get("update") {
        if !item.is_table() {
            return Err(UpdateError::Config("[update] must be a TOML table".into()));
        }
    } else {
        document["update"] = Item::Table(Table::new());
    }
    let table = document["update"]
        .as_table_mut()
        .expect("table established above");
    if let Some(existing) = table.get("channel") {
        if existing.as_str() != Some(channel) {
            return Err(UpdateError::Config(format!(
                "update.channel is {:?}; expected {channel:?} or unset",
                existing
                    .as_str()
                    .map_or_else(|| existing.to_string(), str::to_owned),
            )));
        }
    }
    if let Some(existing) = table.get("manifest_url") {
        if existing.as_str() != Some(manifest_url) {
            return Err(UpdateError::Config(
                "update.manifest_url already points to a different manifest".into(),
            ));
        }
    }
    let add_channel = !table.contains_key("channel");
    let add_manifest = !table.contains_key("manifest_url");
    if !add_channel && !add_manifest {
        return Ok(clean);
    }

    let mut inserted = Vec::new();
    if add_channel {
        table.insert("channel", value(channel));
        inserted.push("channel");
    }
    if add_manifest {
        table.insert("manifest_url", value(manifest_url));
        inserted.push("manifest_url");
    }
    table
        .key_mut(inserted[0])
        .expect("inserted key")
        .leaf_decor_mut()
        .set_prefix(format!("{BLOCK_START}\n"));
    table[*inserted.last().expect("non-empty")]
        .as_value_mut()
        .expect("inserted value")
        .decor_mut()
        .set_suffix(format!("\n{BLOCK_END}"));
    let updated = document.to_string();
    managed_block(&updated)?;
    Ok(updated)
}

/// Remove exactly the settings inside cmux-herdr's validated managed block.
pub fn uninstall_settings(text: &str) -> Result<String> {
    remove_managed_block(text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChange {
    Changed,
    Unchanged,
}

/// Apply a config transform with a durable sibling-temp rename, retaining file mode.
pub fn update_file<F>(path: &Path, transform: F) -> Result<FileChange>
where
    F: FnOnce(&str) -> Result<String>,
{
    let original = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(UpdateError::io(format!("read {}", path.display()), error)),
    };
    let updated = transform(&original)?;
    if updated == original {
        return Ok(FileChange::Unchanged);
    }
    let mode = fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o7777)
        .unwrap_or(0o600);
    atomic_write(path, updated.as_bytes(), mode)?;
    Ok(FileChange::Changed)
}

fn ensure_parent(path: &Path) -> Result<&Path> {
    let parent = path.parent().ok_or_else(|| {
        UpdateError::Config(format!("{} has no parent directory", path.display()))
    })?;
    fs::DirBuilder::new()
        .recursive(true)
        .create(parent)
        .map_err(|error| UpdateError::io(format!("create {}", parent.display()), error))?;
    Ok(parent)
}

fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let parent = ensure_parent(path)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(&format!(
            ".{}.",
            path.file_name()
                .unwrap_or_else(|| OsStr::new("update"))
                .to_string_lossy()
        ))
        .tempfile_in(parent)
        .map_err(|error| {
            UpdateError::io(
                format!("create temporary file for {}", path.display()),
                error,
            )
        })?;
    temporary
        .write_all(bytes)
        .map_err(|error| UpdateError::io(format!("write {}", path.display()), error))?;
    temporary
        .flush()
        .map_err(|error| UpdateError::io(format!("flush {}", path.display()), error))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| UpdateError::io(format!("sync {}", path.display()), error))?;
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(mode))
        .map_err(|error| UpdateError::io(format!("chmod {}", path.display()), error))?;
    temporary
        .persist(path)
        .map_err(|error| UpdateError::io(format!("replace {}", path.display()), error.error))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| UpdateError::io(format!("sync {}", parent.display()), error))?;
    Ok(())
}

fn atomic_copy(source: &Path, destination: &Path) -> Result<()> {
    let bytes = fs::read(source)
        .map_err(|error| UpdateError::io(format!("read {}", source.display()), error))?;
    let mode = fs::metadata(source)
        .map_err(|error| UpdateError::io(format!("stat {}", source.display()), error))?
        .permissions()
        .mode()
        & 0o7777;
    atomic_write(destination, &bytes, mode)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run(&self, program: &Path, args: &[String]) -> io::Result<CommandOutput>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &Path, args: &[String]) -> io::Result<CommandOutput> {
        let output = Command::new(program).args(args).output()?;
        Ok(CommandOutput {
            status: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

pub trait ProcessLiveness {
    fn is_alive(&self, pid: u32) -> bool;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemLiveness;

impl ProcessLiveness for SystemLiveness {
    fn is_alive(&self, pid: u32) -> bool {
        let Some(pid) = i32::try_from(pid)
            .ok()
            .and_then(rustix::process::Pid::from_raw)
        else {
            return false;
        };
        match rustix::process::test_kill_process(pid) {
            Ok(()) => true,
            Err(error) => error == rustix::io::Errno::PERM,
        }
    }
}

#[derive(Debug)]
pub struct UpdateLock {
    path: PathBuf,
    pid: u32,
    _advisory: File,
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        let owned = fs::read_to_string(self.path.join("pid"))
            .ok()
            .and_then(|text| text.trim().parse::<u32>().ok())
            == Some(self.pid);
        if owned {
            let _ = fs::remove_file(self.path.join("pid"));
            let _ = fs::remove_dir(&self.path);
        }
    }
}

#[derive(Debug)]
pub enum LockAttempt {
    Acquired(UpdateLock),
    Busy { pid: u32 },
}

pub fn acquire_update_lock(
    state_root: &Path,
    liveness: &dyn ProcessLiveness,
) -> Result<LockAttempt> {
    fs::create_dir_all(state_root)
        .map_err(|error| UpdateError::io(format!("create {}", state_root.display()), error))?;
    let advisory_path = state_root.join("herdr-auto-update.guard");
    let advisory = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&advisory_path)
        .map_err(|error| UpdateError::io(format!("open {}", advisory_path.display()), error))?;
    fs::set_permissions(&advisory_path, fs::Permissions::from_mode(0o600))
        .map_err(|error| UpdateError::io(format!("chmod {}", advisory_path.display()), error))?;
    if let Err(error) = rustix::fs::flock(
        &advisory,
        rustix::fs::FlockOperation::NonBlockingLockExclusive,
    ) {
        if error == rustix::io::Errno::AGAIN || error == rustix::io::Errno::WOULDBLOCK {
            let path = state_root.join(LOCK_NAME);
            let owner = fs::symlink_metadata(&path)
                .ok()
                .filter(|metadata| metadata.file_type().is_dir())
                .and_then(|_| fs::read_to_string(path.join("pid")).ok())
                .and_then(|text| text.trim().parse().ok())
                .unwrap_or(0);
            return Ok(LockAttempt::Busy { pid: owner });
        }
        return Err(UpdateError::io("lock update guard", error.into()));
    }
    let path = state_root.join(LOCK_NAME);
    let pid = std::process::id();
    match fs::create_dir(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| UpdateError::io("inspect update lock", error))?;
            if !metadata.file_type().is_dir() {
                return Err(UpdateError::Config(format!(
                    "update lock is not a directory: {}",
                    path.display()
                )));
            }
            let owner = fs::read_to_string(path.join("pid"))
                .ok()
                .and_then(|text| text.trim().parse::<u32>().ok());
            if let Some(owner) = owner {
                if liveness.is_alive(owner) {
                    return Ok(LockAttempt::Busy { pid: owner });
                }
            }
            let stale = state_root.join(format!("{LOCK_NAME}.stale.{pid}"));
            fs::rename(&path, &stale)
                .map_err(|error| UpdateError::io("take over stale update lock", error))?;
            let entries = fs::read_dir(&stale)
                .map_err(|error| UpdateError::io("inspect stale update lock", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| UpdateError::io("inspect stale update lock", error))?;
            if entries
                .iter()
                .any(|entry| entry.file_name() != OsStr::new("pid"))
            {
                return Err(UpdateError::Config(format!(
                    "stale update lock contains unexpected files: {}",
                    stale.display()
                )));
            }
            if stale.join("pid").exists() {
                fs::remove_file(stale.join("pid"))
                    .map_err(|error| UpdateError::io("remove stale lock owner", error))?;
            }
            fs::remove_dir(&stale)
                .map_err(|error| UpdateError::io("remove stale update lock", error))?;
            match fs::create_dir(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    let owner = fs::read_to_string(path.join("pid"))
                        .ok()
                        .and_then(|text| text.trim().parse().ok())
                        .unwrap_or(0);
                    return Ok(LockAttempt::Busy { pid: owner });
                }
                Err(error) => return Err(UpdateError::io("acquire update lock", error)),
            }
        }
        Err(error) => return Err(UpdateError::io("acquire update lock", error)),
    }
    if let Err(error) = atomic_write(&path.join("pid"), format!("{pid}\n").as_bytes(), 0o600) {
        let _ = fs::remove_dir(&path);
        return Err(error);
    }
    Ok(LockAttempt::Acquired(UpdateLock {
        path,
        pid,
        _advisory: advisory,
    }))
}

#[derive(Debug, Clone)]
pub struct UpdateRequest {
    pub herdr_binary: PathBuf,
    pub state_root: PathBuf,
    pub backup_limit: usize,
}

impl UpdateRequest {
    pub fn new(herdr_binary: PathBuf, state_root: PathBuf) -> Self {
        Self {
            herdr_binary,
            state_root,
            backup_limit: BACKUP_LIMIT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    Busy {
        pid: u32,
    },
    Current {
        version: String,
        digest: String,
    },
    Updated {
        from_version: String,
        to_version: String,
        digest: String,
        backup: PathBuf,
    },
}

fn command_required(
    runner: &dyn CommandRunner,
    program: &Path,
    args: &[&str],
) -> Result<CommandOutput> {
    let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let output = runner
        .run(program, &args)
        .map_err(|error| UpdateError::io(format!("run {}", program.display()), error))?;
    if output.status != 0 {
        return Err(UpdateError::Command {
            program: program.display().to_string(),
            status: output.status,
            stderr: output.stderr,
        });
    }
    Ok(output)
}

fn sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .map_err(|error| UpdateError::io(format!("open {}", path.display()), error))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| UpdateError::io(format!("read {}", path.display()), error))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn restore_binary(backup: &Path, binary: &Path) -> Result<()> {
    atomic_copy(backup, binary)
}

fn backup_name(version: &str) -> String {
    let safe = version
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "._-".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("herdr-{}-{stamp}", safe.trim_matches('_'))
}

struct StagedPrune {
    _directory: tempfile::TempDir,
    moved: Vec<(PathBuf, PathBuf)>,
}

impl StagedPrune {
    fn rollback(self) -> Result<()> {
        let mut failures = Vec::new();
        for (original, staged) in self.moved.iter().rev() {
            if let Err(error) = fs::rename(staged, original) {
                failures.push(error.to_string());
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(UpdateError::Config(format!(
                "backup-prune rollback errors: {}",
                failures.join("; ")
            )))
        }
    }
}

fn stage_backup_prune(
    directory: &Path,
    retained: &Path,
    limit: usize,
) -> Result<Option<StagedPrune>> {
    let mut backups = fs::read_dir(directory)
        .map_err(|error| UpdateError::io(format!("read {}", directory.display()), error))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with("herdr-") && entry.path() != retained
        })
        .collect::<Vec<_>>();
    backups.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH)
    });
    let remove_count = (backups.len() + 1).saturating_sub(limit.max(1));
    if remove_count == 0 {
        return Ok(None);
    }
    let staging = tempfile::Builder::new()
        .prefix(".herdr-prune.")
        .tempdir_in(directory)
        .map_err(|error| UpdateError::io("create backup-prune staging directory", error))?;
    let mut moved = Vec::new();
    for entry in backups.into_iter().take(remove_count) {
        let original = entry.path();
        let staged = staging.path().join(entry.file_name());
        if let Err(error) = fs::rename(&original, &staged) {
            let original_error = UpdateError::io(
                format!("stage backup prune for {}", original.display()),
                error,
            );
            let mut restore_failures = Vec::new();
            for (old, temporary) in moved.iter().rev() {
                if let Err(error) = fs::rename(temporary, old) {
                    restore_failures.push(error.to_string());
                }
            }
            if restore_failures.is_empty() {
                return Err(original_error);
            }
            return Err(UpdateError::Config(format!(
                "{original_error}; backup-prune rollback errors: {}",
                restore_failures.join("; ")
            )));
        }
        moved.push((original, staged));
    }
    Ok(Some(StagedPrune {
        _directory: staging,
        moved,
    }))
}

#[derive(Serialize)]
struct InstalledRecord<'a> {
    version: &'a str,
    sha256: &'a str,
}

pub fn run_update(
    request: &UpdateRequest,
    runner: &dyn CommandRunner,
    liveness: &dyn ProcessLiveness,
) -> Result<UpdateOutcome> {
    let _lock = match acquire_update_lock(&request.state_root, liveness)? {
        LockAttempt::Busy { pid } => return Ok(UpdateOutcome::Busy { pid }),
        LockAttempt::Acquired(lock) => lock,
    };
    let binary = fs::canonicalize(&request.herdr_binary).map_err(|error| {
        UpdateError::io(format!("resolve {}", request.herdr_binary.display()), error)
    })?;
    let before_version = command_required(runner, &binary, &["--version"])?
        .stdout
        .trim()
        .to_owned();
    let before_digest = sha256(&binary)?;
    let backup_dir = request.state_root.join("herdr-backups");
    fs::create_dir_all(&backup_dir)
        .map_err(|error| UpdateError::io(format!("create {}", backup_dir.display()), error))?;
    let record_path = request.state_root.join("herdr-update.json");
    let record_snapshot = FileSnapshot::capture(record_path.clone())?;
    let temporary = tempfile::Builder::new()
        .prefix(".herdr-backup.")
        .tempfile_in(&backup_dir)
        .map_err(|error| UpdateError::io("create Herdr backup", error))?;
    atomic_copy(&binary, temporary.path())?;

    let update_args = vec!["update".to_owned(), "--handoff".to_owned()];
    let update = match runner.run(&binary, &update_args) {
        Ok(output) => output,
        Err(error) => {
            let changed = sha256(&binary).ok().as_deref() != Some(before_digest.as_str());
            if changed {
                restore_binary(temporary.path(), &binary)?;
            }
            return Err(UpdateError::io(
                if changed {
                    "run herdr update --handoff (previous binary restored)"
                } else {
                    "run herdr update --handoff"
                },
                error,
            ));
        }
    };
    let after_digest = sha256(&binary).ok();
    if update.status != 0 {
        let changed = after_digest.as_deref() != Some(before_digest.as_str());
        if changed {
            restore_binary(temporary.path(), &binary)?;
        }
        return Err(UpdateError::UpdateFailed {
            status: update.status,
            restored: changed,
        });
    }
    if after_digest.as_deref() == Some(before_digest.as_str()) {
        return Ok(UpdateOutcome::Current {
            version: before_version,
            digest: before_digest,
        });
    }
    let Some(after_digest) = after_digest else {
        restore_binary(temporary.path(), &binary)?;
        return Err(UpdateError::InvalidReplacement {
            reason: "updated binary is missing".into(),
            restored: true,
        });
    };
    let after_version = match command_required(runner, &binary, &["--version"]) {
        Ok(output) if !output.stdout.trim().is_empty() => output.stdout.trim().to_owned(),
        Ok(_) => {
            restore_binary(temporary.path(), &binary)?;
            return Err(UpdateError::InvalidReplacement {
                reason: "version check returned no version".into(),
                restored: true,
            });
        }
        Err(error) => {
            restore_binary(temporary.path(), &binary)?;
            return Err(UpdateError::InvalidReplacement {
                reason: error.to_string(),
                restored: true,
            });
        }
    };
    let record = serde_json::to_vec_pretty(&InstalledRecord {
        version: &after_version,
        sha256: &after_digest,
    })
    .map_err(|error| UpdateError::Config(format!("serialize installed update record: {error}")))?;
    let backup = backup_dir.join(backup_name(&before_version));
    if let Err(error) = temporary.persist(&backup) {
        restore_binary(error.file.path(), &binary)?;
        return Err(UpdateError::io(
            format!("retain {}", backup.display()),
            error.error,
        ));
    }
    let staged_prune = match stage_backup_prune(&backup_dir, &backup, request.backup_limit) {
        Ok(staged) => staged,
        Err(error) => {
            restore_binary(&backup, &binary)?;
            let _ = fs::remove_file(&backup);
            return Err(error);
        }
    };
    if let Err(error) = atomic_write(&record_path, &record, 0o600) {
        let mut recovery = Vec::new();
        if let Some(staged) = staged_prune {
            if let Err(restore_error) = staged.rollback() {
                recovery.push(restore_error.to_string());
            }
        }
        if let Err(restore_error) = record_snapshot.restore() {
            recovery.push(restore_error.to_string());
        }
        if let Err(restore_error) = restore_binary(&backup, &binary) {
            recovery.push(restore_error.to_string());
        }
        if let Err(remove_error) = fs::remove_file(&backup) {
            recovery.push(remove_error.to_string());
        }
        if recovery.is_empty() {
            return Err(error);
        }
        return Err(UpdateError::Config(format!(
            "{error}; update rollback errors: {}",
            recovery.join("; ")
        )));
    }
    drop(staged_prune);
    Ok(UpdateOutcome::Updated {
        from_version: before_version,
        to_version: after_version,
        digest: after_digest,
        backup,
    })
}

/// Resolve an override or PATH entry without invoking a shell.
pub fn resolve_executable(
    override_path: Option<&Path>,
    path: Option<&OsStr>,
    name: &str,
) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return fs::canonicalize(path)
            .map_err(|error| UpdateError::io(format!("resolve {}", path.display()), error));
    }
    let path = path.ok_or_else(|| UpdateError::Config(format!("{name} not found on PATH")))?;
    for directory in std::env::split_paths(path) {
        let candidate = directory.join(name);
        if fs::metadata(&candidate)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
        {
            return fs::canonicalize(&candidate).map_err(|error| {
                UpdateError::io(format!("resolve {}", candidate.display()), error)
            });
        }
    }
    Err(UpdateError::Config(format!("{name} not found on PATH")))
}
pub fn resolve_herdr_path(
    explicit: Option<&Path>,
    environment: Option<&Path>,
    path: Option<&OsStr>,
) -> Result<PathBuf> {
    resolve_executable(explicit.or(environment), path, "herdr")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceManager {
    Launchd { domain: String },
    Systemd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePaths {
    pub home: PathBuf,
    pub config_path: PathBuf,
    pub state_root: PathBuf,
    pub data_root: PathBuf,
    pub definition_dir: PathBuf,
    pub source_binary: PathBuf,
    pub herdr_binary: PathBuf,
}

fn absolute_xdg(name: &str, fallback: PathBuf) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or(fallback)
}

impl ServicePaths {
    pub fn discover(manager: &ServiceManager, herdr_override: Option<&Path>) -> Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| UpdateError::Config("HOME must be an absolute path".into()))?;
        let config_home = absolute_xdg("XDG_CONFIG_HOME", home.join(".config"));
        let state_home = absolute_xdg("XDG_STATE_HOME", home.join(".local/state"));
        let data_home = absolute_xdg("XDG_DATA_HOME", home.join(".local/share"));
        let definition_dir = match manager {
            ServiceManager::Launchd { .. } => home.join("Library/LaunchAgents"),
            ServiceManager::Systemd => config_home.join("systemd/user"),
        };
        let source_binary = std::env::current_exe()
            .and_then(fs::canonicalize)
            .map_err(|error| UpdateError::io("resolve cmux-herdr executable", error))?;
        let environment_herdr = std::env::var_os("HERDR_BIN")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let herdr_binary = resolve_herdr_path(
            herdr_override,
            environment_herdr.as_deref(),
            std::env::var_os("PATH").as_deref(),
        )?;
        Ok(Self {
            home,
            config_path: config_home.join("herdr/config.toml"),
            state_root: state_home.join("cmux-herdr"),
            data_root: data_home.join("cmux-herdr"),
            definition_dir,
            source_binary,
            herdr_binary,
        })
    }

    pub fn runtime_binary(&self) -> PathBuf {
        self.data_root.join("bin").join(RUNTIME_NAME)
    }
    pub fn launchd_plist(&self) -> PathBuf {
        self.definition_dir.join(format!("{LABEL}.plist"))
    }
    pub fn systemd_service(&self) -> PathBuf {
        self.definition_dir.join(format!("{LABEL}.service"))
    }
    pub fn systemd_timer(&self) -> PathBuf {
        self.definition_dir.join(format!("{LABEL}.timer"))
    }
}

#[derive(Debug, Clone)]
pub struct InstallRequest {
    pub manager: ServiceManager,
    pub paths: ServicePaths,
    pub channel: String,
    pub manifest_url: String,
}

impl InstallRequest {
    pub fn new(
        manager: ServiceManager,
        paths: ServicePaths,
        channel: String,
        manifest_url: String,
    ) -> Self {
        Self {
            manager,
            paths,
            channel,
            manifest_url,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallResult {
    pub manager: ServiceManager,
    pub config: FileChange,
    pub runtime_binary: PathBuf,
    pub definitions: Vec<PathBuf>,
    pub uninstall_command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallResult {
    pub config: FileChange,
    pub removed: Vec<PathBuf>,
    pub command_warnings: Vec<String>,
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn render_launchd(paths: &ServicePaths, _domain: &str) -> String {
    let runtime = xml_escape(&paths.runtime_binary().display().to_string());
    let home = xml_escape(&paths.home.display().to_string());
    let herdr = xml_escape(&paths.herdr_binary.display().to_string());
    let state_home = xml_escape(
        &paths
            .state_root
            .parent()
            .unwrap_or(&paths.state_root)
            .display()
            .to_string(),
    );
    let config_home = xml_escape(
        &paths
            .config_path
            .parent()
            .and_then(Path::parent)
            .unwrap_or(&paths.home)
            .display()
            .to_string(),
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key><array><string>{runtime}</string><string>update-service</string><string>run</string><string>--herdr</string><string>{herdr}</string></array>
  <key>RunAtLoad</key><true/>
  <key>StartInterval</key><integer>21600</integer>
  <key>ProcessType</key><string>Background</string>
  <key>StandardOutPath</key><string>{home}/Library/Logs/cmux-herdr-herdr-auto-update.out.log</string>
  <key>StandardErrorPath</key><string>{home}/Library/Logs/cmux-herdr-herdr-auto-update.err.log</string>
  <key>EnvironmentVariables</key><dict><key>HOME</key><string>{home}</string><key>XDG_CONFIG_HOME</key><string>{config_home}</string><key>XDG_STATE_HOME</key><string>{state_home}</string></dict>
</dict>
</plist>
"#
    )
}

fn systemd_quote_text(text: &str, command_argument: bool) -> String {
    let mut escaped = String::with_capacity(text.len() + 2);
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '%' => escaped.push_str("%%"),
            '$' if command_argument => escaped.push_str("$$"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() && u32::from(character) <= 0xff => {
                escaped.push_str(&format!("\\x{:02x}", u32::from(character)));
            }
            character => escaped.push(character),
        }
    }
    format!("\"{escaped}\"")
}

fn systemd_quote(path: &Path) -> String {
    systemd_quote_text(&path.display().to_string(), true)
}

pub fn render_systemd(paths: &ServicePaths) -> (String, String) {
    let home = systemd_quote_text(&format!("HOME={}", paths.home.display()), false);
    let state_home = systemd_quote_text(
        &format!(
            "XDG_STATE_HOME={}",
            paths
                .state_root
                .parent()
                .unwrap_or(&paths.state_root)
                .display()
        ),
        false,
    );
    let config_home = systemd_quote_text(
        &format!(
            "XDG_CONFIG_HOME={}",
            paths
                .config_path
                .parent()
                .and_then(Path::parent)
                .unwrap_or(&paths.home)
                .display()
        ),
        false,
    );
    let service = format!(
        "[Unit]\nDescription=Update Herdr from its configured release manifest\n\n[Service]\nType=oneshot\nEnvironment={home}\nEnvironment={config_home}\nEnvironment={state_home}\nExecStart={} update-service run --herdr {}\n",
        systemd_quote(&paths.runtime_binary()), systemd_quote(&paths.herdr_binary),
    );
    let timer = format!(
        "[Unit]\nDescription=Check for Herdr updates every six hours\n\n[Timer]\nOnBootSec=2min\nOnUnitActiveSec=6h\nRandomizedDelaySec=5min\nUnit={LABEL}.service\n\n[Install]\nWantedBy=timers.target\n",
    );
    (service, timer)
}

#[derive(Clone)]
struct FileSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    mode: u32,
}

impl FileSnapshot {
    fn capture(path: PathBuf) -> Result<Self> {
        match fs::read(&path) {
            Ok(bytes) => {
                let mode = fs::metadata(&path)
                    .map_err(|error| UpdateError::io(format!("stat {}", path.display()), error))?
                    .permissions()
                    .mode()
                    & 0o7777;
                Ok(Self {
                    path,
                    bytes: Some(bytes),
                    mode,
                })
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self {
                path,
                bytes: None,
                mode: 0o600,
            }),
            Err(error) => Err(UpdateError::io(format!("read {}", path.display()), error)),
        }
    }

    fn restore(&self) -> Result<()> {
        if let Some(bytes) = &self.bytes {
            atomic_write(&self.path, bytes, self.mode)
        } else {
            match fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(UpdateError::io(
                    format!("remove {}", self.path.display()),
                    error,
                )),
            }
        }
    }
}

fn run_status(
    runner: &dyn CommandRunner,
    program: &str,
    args: &[&str],
    required: bool,
) -> Result<Option<CommandOutput>> {
    let owned = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    match runner.run(Path::new(program), &owned) {
        Ok(output) if output.status == 0 => Ok(Some(output)),
        Ok(output) => Err(UpdateError::Command {
            program: program.into(),
            status: output.status,
            stderr: output.stderr,
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound && !required => Ok(None),
        Err(error) => Err(UpdateError::io(format!("run {program}"), error)),
    }
}

fn probe_status(
    runner: &dyn CommandRunner,
    program: &str,
    args: &[&str],
) -> Result<(bool, String)> {
    let owned = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let output = runner
        .run(Path::new(program), &owned)
        .map_err(|error| UpdateError::io(format!("run {program}"), error))?;
    Ok((output.status == 0, output.stdout))
}

#[derive(Clone, Copy)]
enum PriorServiceState {
    Launchd { loaded: bool, enabled: bool },
    Systemd { enabled: bool, active: bool },
}

fn rollback(snapshots: &[FileSnapshot]) -> Result<()> {
    let mut failures = Vec::new();
    for snapshot in snapshots.iter().rev() {
        if let Err(error) = snapshot.restore() {
            failures.push(error.to_string());
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(UpdateError::Config(format!(
            "rollback errors: {}",
            failures.join("; ")
        )))
    }
}

pub fn install_service(
    request: &InstallRequest,
    runner: &dyn CommandRunner,
) -> Result<InstallResult> {
    let paths = &request.paths;
    let defaults = command_required(runner, &paths.herdr_binary, &["--default-config"])?;
    if !defaults
        .stdout
        .lines()
        .any(|line| line.trim_start().starts_with("# manifest_url = "))
    {
        return Err(UpdateError::Config(
            "installed Herdr does not support update.manifest_url".into(),
        ));
    }
    command_required(runner, &paths.herdr_binary, &["config", "check"])?;
    let prior_state = match &request.manager {
        ServiceManager::Launchd { domain } => {
            let target = format!("{domain}/{LABEL}");
            let (loaded, _) = probe_status(runner, "launchctl", &["print", &target])?;
            if loaded && !paths.launchd_plist().exists() {
                return Err(UpdateError::Config(format!(
                    "launchd label {LABEL} is loaded from an unmanaged definition"
                )));
            }
            let disabled_output =
                run_status(runner, "launchctl", &["print-disabled", domain], true)?
                    .expect("required launchctl command returned output")
                    .stdout;
            let disabled = disabled_output
                .lines()
                .any(|line| line.contains(LABEL) && line.contains("=> true"));
            PriorServiceState::Launchd {
                loaded,
                enabled: !disabled,
            }
        }
        ServiceManager::Systemd => {
            run_status(runner, "systemctl", &["--user", "show-environment"], true)?;
            let timer = format!("{LABEL}.timer");
            let (enabled, _) =
                probe_status(runner, "systemctl", &["--user", "is-enabled", &timer])?;
            let (active, _) = probe_status(runner, "systemctl", &["--user", "is-active", &timer])?;
            PriorServiceState::Systemd { enabled, active }
        }
    };
    let definitions = match &request.manager {
        ServiceManager::Launchd { .. } => vec![paths.launchd_plist()],
        ServiceManager::Systemd => vec![paths.systemd_service(), paths.systemd_timer()],
    };
    let mut targets = vec![paths.config_path.clone(), paths.runtime_binary()];
    targets.extend(definitions.iter().cloned());
    let snapshots = targets
        .into_iter()
        .map(FileSnapshot::capture)
        .collect::<Result<Vec<_>>>()?;
    let attempt = (|| {
        let config = update_file(&paths.config_path, |text| {
            install_settings(text, &request.channel, &request.manifest_url)
        })?;
        if let Err(error) = command_required(runner, &paths.herdr_binary, &["config", "check"]) {
            return Err(UpdateError::Config(format!(
                "managed update settings failed Herdr validation: {error}"
            )));
        }
        atomic_copy(&paths.source_binary, &paths.runtime_binary())?;
        match &request.manager {
            ServiceManager::Launchd { domain } => {
                atomic_write(
                    &paths.launchd_plist(),
                    render_launchd(paths, domain).as_bytes(),
                    0o644,
                )?;
                let plist = paths.launchd_plist().display().to_string();
                run_status(runner, "plutil", &["-lint", &plist], false)?;
                let _ = run_status(runner, "launchctl", &["bootout", domain, &plist], false);
                run_status(runner, "launchctl", &["bootstrap", domain, &plist], true)?;
                let target = format!("{domain}/{LABEL}");
                run_status(runner, "launchctl", &["enable", &target], true)?;
            }
            ServiceManager::Systemd => {
                let (service, timer) = render_systemd(paths);
                atomic_write(&paths.systemd_service(), service.as_bytes(), 0o644)?;
                atomic_write(&paths.systemd_timer(), timer.as_bytes(), 0o644)?;
                let service_path = paths.systemd_service().display().to_string();
                let timer_path = paths.systemd_timer().display().to_string();
                run_status(
                    runner,
                    "systemd-analyze",
                    &["verify", &service_path, &timer_path],
                    false,
                )?;
                run_status(runner, "systemctl", &["--user", "daemon-reload"], true)?;
                let timer_name = format!("{LABEL}.timer");
                run_status(
                    runner,
                    "systemctl",
                    &["--user", "enable", "--now", &timer_name],
                    true,
                )?;
                let service_name = format!("{LABEL}.service");
                run_status(
                    runner,
                    "systemctl",
                    &["--user", "start", &service_name],
                    true,
                )?;
            }
        }
        Ok(config)
    })();
    let config = match attempt {
        Ok(config) => config,
        Err(error) => {
            match &request.manager {
                ServiceManager::Launchd { domain } => {
                    let target = format!("{domain}/{LABEL}");
                    let _ = run_status(runner, "launchctl", &["bootout", &target], false);
                }
                ServiceManager::Systemd => {
                    let timer = format!("{LABEL}.timer");
                    let _ = run_status(
                        runner,
                        "systemctl",
                        &["--user", "disable", "--now", &timer],
                        false,
                    );
                    let service = format!("{LABEL}.service");
                    let _ = run_status(runner, "systemctl", &["--user", "stop", &service], false);
                }
            }
            if let Err(rollback_error) = rollback(&snapshots) {
                return Err(UpdateError::Config(format!(
                    "{error}; rollback failed: {rollback_error}"
                )));
            }
            let manager_restore = (|| -> Result<()> {
                match (prior_state, &request.manager) {
                    (
                        PriorServiceState::Launchd { loaded, enabled },
                        ServiceManager::Launchd { domain },
                    ) => {
                        let plist = paths.launchd_plist().display().to_string();
                        if loaded {
                            run_status(runner, "launchctl", &["bootstrap", domain, &plist], true)?;
                        }
                        let target = format!("{domain}/{LABEL}");
                        let operation = if enabled { "enable" } else { "disable" };
                        run_status(runner, "launchctl", &[operation, &target], true)?;
                    }
                    (PriorServiceState::Systemd { enabled, active }, ServiceManager::Systemd) => {
                        run_status(runner, "systemctl", &["--user", "daemon-reload"], true)?;
                        let timer = format!("{LABEL}.timer");
                        let enable_operation = if enabled { "enable" } else { "disable" };
                        run_status(
                            runner,
                            "systemctl",
                            &["--user", enable_operation, &timer],
                            true,
                        )?;
                        let active_operation = if active { "start" } else { "stop" };
                        run_status(
                            runner,
                            "systemctl",
                            &["--user", active_operation, &timer],
                            true,
                        )?;
                    }
                    _ => {
                        return Err(UpdateError::Config(
                            "service manager changed during install".into(),
                        ))
                    }
                }
                Ok(())
            })();
            if let Err(restore_error) = manager_restore {
                return Err(UpdateError::Config(format!(
                    "{error}; service rollback failed: {restore_error}"
                )));
            }
            return Err(error);
        }
    };
    Ok(InstallResult {
        manager: request.manager.clone(),
        config,
        runtime_binary: paths.runtime_binary(),
        definitions,
        uninstall_command: vec![
            paths.runtime_binary().display().to_string(),
            "update-service".into(),
            "uninstall".into(),
        ],
    })
}

fn remove_existing(path: &Path, removed: &mut Vec<PathBuf>) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => {
            removed.push(path.to_path_buf());
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(UpdateError::io(format!("remove {}", path.display()), error)),
    }
}

fn best_effort(
    runner: &dyn CommandRunner,
    program: &str,
    args: &[&str],
    warnings: &mut Vec<String>,
) {
    if let Err(error) = run_status(runner, program, args, false) {
        warnings.push(error.to_string());
    }
}

pub fn uninstall_service(
    manager: &ServiceManager,
    paths: &ServicePaths,
    runner: &dyn CommandRunner,
) -> Result<UninstallResult> {
    let mut warnings = Vec::new();
    match manager {
        ServiceManager::Launchd { domain } => {
            let target = format!("{domain}/{LABEL}");
            best_effort(runner, "launchctl", &["bootout", &target], &mut warnings);
            let plist = paths.launchd_plist().display().to_string();
            best_effort(
                runner,
                "launchctl",
                &["bootout", domain, &plist],
                &mut warnings,
            );
        }
        ServiceManager::Systemd => {
            let timer = format!("{LABEL}.timer");
            let service = format!("{LABEL}.service");
            best_effort(
                runner,
                "systemctl",
                &["--user", "disable", "--now", &timer],
                &mut warnings,
            );
            best_effort(
                runner,
                "systemctl",
                &["--user", "stop", &service],
                &mut warnings,
            );
        }
    }
    let config = if paths.config_path.exists() {
        update_file(&paths.config_path, uninstall_settings)?
    } else {
        FileChange::Unchanged
    };
    let mut removed = Vec::new();
    match manager {
        ServiceManager::Launchd { .. } => remove_existing(&paths.launchd_plist(), &mut removed)?,
        ServiceManager::Systemd => {
            remove_existing(&paths.systemd_service(), &mut removed)?;
            remove_existing(&paths.systemd_timer(), &mut removed)?;
        }
    }
    remove_existing(&paths.runtime_binary(), &mut removed)?;
    if matches!(manager, ServiceManager::Systemd) {
        best_effort(
            runner,
            "systemctl",
            &["--user", "daemon-reload"],
            &mut warnings,
        );
        let service = format!("{LABEL}.service");
        best_effort(
            runner,
            "systemctl",
            &["--user", "reset-failed", &service],
            &mut warnings,
        );
    }
    Ok(UninstallResult {
        config,
        removed,
        command_warnings: warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};

    const MANIFEST: &str = "https://example.com/preview.json";

    #[derive(Default)]
    struct FakeRunner {
        calls: RefCell<Vec<(String, Vec<String>)>>,
        missing: HashSet<String>,
        failures: HashMap<String, i32>,
        argument_failures: HashMap<Vec<String>, i32>,
        default_config: String,
    }

    impl FakeRunner {
        fn supporting() -> Self {
            Self {
                default_config: "# manifest_url = \"https://example.com/releases.json\"\n".into(),
                ..Self::default()
            }
        }

        fn call_names(&self) -> Vec<String> {
            self.calls
                .borrow()
                .iter()
                .map(|(name, _)| name.clone())
                .collect()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, program: &Path, args: &[String]) -> std::io::Result<CommandOutput> {
            let name = program.file_name().unwrap().to_string_lossy().into_owned();
            self.calls.borrow_mut().push((name.clone(), args.to_vec()));
            if self.missing.contains(&name) {
                return Err(std::io::Error::from(std::io::ErrorKind::NotFound));
            }
            let probe_miss = args.first().map(String::as_str) == Some("print")
                || args
                    .iter()
                    .any(|arg| arg == "is-enabled" || arg == "is-active");
            let status = self
                .argument_failures
                .get(args)
                .or_else(|| self.failures.get(&name))
                .copied()
                .unwrap_or(i32::from(probe_miss));
            let stdout = if args == ["--default-config"] {
                self.default_config.clone()
            } else {
                String::new()
            };
            Ok(CommandOutput {
                status,
                stdout,
                stderr: String::new(),
            })
        }
    }

    struct FixedLiveness(bool);
    impl ProcessLiveness for FixedLiveness {
        fn is_alive(&self, _pid: u32) -> bool {
            self.0
        }
    }

    struct ScriptedUpdateRunner {
        binary: PathBuf,
        replacement: Option<Vec<u8>>,
        update_status: i32,
        after_version: Option<String>,
        before_version: String,
        version_calls: RefCell<usize>,
    }

    impl CommandRunner for ScriptedUpdateRunner {
        fn run(&self, _program: &Path, args: &[String]) -> std::io::Result<CommandOutput> {
            if args == ["--version"] {
                let mut calls = self.version_calls.borrow_mut();
                let stdout = if *calls == 0 {
                    self.before_version.clone()
                } else if let Some(version) = &self.after_version {
                    version.clone()
                } else {
                    *calls += 1;
                    return Ok(CommandOutput {
                        status: 1,
                        stdout: String::new(),
                        stderr: "bad replacement".into(),
                    });
                };
                *calls += 1;
                return Ok(CommandOutput {
                    status: 0,
                    stdout,
                    stderr: String::new(),
                });
            }
            assert_eq!(args, ["update", "--handoff"]);
            if let Some(bytes) = &self.replacement {
                fs::write(&self.binary, bytes).unwrap();
                fs::set_permissions(&self.binary, fs::Permissions::from_mode(0o755)).unwrap();
            }
            Ok(CommandOutput {
                status: self.update_status,
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }

    fn service_paths(root: &Path) -> ServicePaths {
        let home = root.join("home");
        ServicePaths {
            home: home.clone(),
            config_path: root.join("config/herdr/config.toml"),
            state_root: root.join("state/cmux-herdr"),
            data_root: root.join("data/cmux-herdr"),
            definition_dir: root.join("definitions"),
            source_binary: root.join("source/cmux-herdr"),
            herdr_binary: root.join("bin/herdr"),
        }
    }

    #[test]
    fn config_install_uninstall_preserves_comments_order_and_mode() {
        let original = "# owner comment\nonboarding = false\n\n[update] # keep\nversion_check = true\n\n[ui]\nshow_agent_panel = false\n";
        let installed = install_settings(original, "preview", MANIFEST).unwrap();
        assert!(installed.contains(BLOCK_START));
        assert!(installed.contains("channel = \"preview\""));
        assert!(installed.contains("manifest_url = \"https://example.com/preview.json\""));
        assert_eq!(uninstall_settings(&installed).unwrap(), original);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, original).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        assert_eq!(
            update_file(&path, |text| install_settings(text, "preview", MANIFEST)).unwrap(),
            FileChange::Changed
        );
        assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o640);
    }

    #[test]
    fn config_install_is_idempotent_and_matching_values_stay_user_owned() {
        let original = format!("[update]\nchannel = \"preview\"\nmanifest_url = \"{MANIFEST}\"\n");
        assert_eq!(
            install_settings(&original, "preview", MANIFEST).unwrap(),
            original
        );
        let managed =
            install_settings("[update]\nversion_check = true\n", "preview", MANIFEST).unwrap();
        assert_eq!(
            install_settings(&managed, "preview", MANIFEST).unwrap(),
            managed
        );
    }

    #[test]
    fn config_rejects_conflicts_non_https_and_malformed_markers() {
        assert!(
            install_settings("[update]\nchannel = \"stable\"\n", "preview", MANIFEST)
                .unwrap_err()
                .to_string()
                .contains("channel")
        );
        assert!(install_settings(
            "[update]\nmanifest_url = \"https://elsewhere.test/x\"\n",
            "preview",
            MANIFEST
        )
        .unwrap_err()
        .to_string()
        .contains("manifest"));
        assert!(install_settings("", "preview", "http://example.com/x")
            .unwrap_err()
            .to_string()
            .contains("https"));
        let malformed = format!("[update]\n{BLOCK_START}\nchannel = \"preview\"\n");
        assert!(uninstall_settings(&malformed)
            .unwrap_err()
            .to_string()
            .contains("marker"));
        let foreign = format!(
            "[update]\n{BLOCK_START}\nchannel = \"preview\"\nuser_key = true\n{BLOCK_END}\n"
        );
        assert!(uninstall_settings(&foreign)
            .unwrap_err()
            .to_string()
            .contains("managed block"));
    }

    #[test]
    fn marker_text_inside_multiline_string_is_user_content() {
        let original = format!(
            "[update]\nnotes = '''\n{BLOCK_START}\nchannel = \"preview\"\n{BLOCK_END}\n'''\n"
        );
        assert_eq!(uninstall_settings(&original).unwrap(), original);
    }

    #[test]
    fn herdr_environment_override_precedes_path_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let override_path = dir.path().join("custom-herdr");
        let path_binary = dir.path().join("herdr");
        fs::write(&override_path, b"custom").unwrap();
        fs::write(&path_binary, b"path").unwrap();
        fs::set_permissions(&override_path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&path_binary, fs::Permissions::from_mode(0o755)).unwrap();
        let resolved =
            resolve_herdr_path(None, Some(&override_path), Some(dir.path().as_os_str())).unwrap();
        assert_eq!(resolved, fs::canonicalize(override_path).unwrap());
    }

    #[test]
    fn update_file_is_atomic_and_noop_aware() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/config.toml");
        assert_eq!(
            update_file(&path, |_| Ok("[update]\n".into())).unwrap(),
            FileChange::Changed
        );
        assert_eq!(
            update_file(&path, |text| Ok(text.into())).unwrap(),
            FileChange::Unchanged
        );
        assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o600);
    }

    #[test]
    fn stale_lock_is_recovered_and_removed_by_owner() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let lock = root.join(LOCK_NAME);
        fs::create_dir_all(&lock).unwrap();
        fs::write(lock.join("pid"), "99999999\n").unwrap();
        let acquired = acquire_update_lock(root, &FixedLiveness(false)).unwrap();
        assert!(matches!(acquired, LockAttempt::Acquired(_)));
        drop(acquired);
        assert!(!lock.exists());
    }

    #[test]
    fn live_lock_is_busy_and_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join(LOCK_NAME);
        fs::create_dir_all(&lock).unwrap();
        fs::write(lock.join("pid"), "42\n").unwrap();
        assert!(matches!(
            acquire_update_lock(dir.path(), &FixedLiveness(true)).unwrap(),
            LockAttempt::Busy { pid: 42 }
        ));
        assert_eq!(fs::read_to_string(lock.join("pid")).unwrap(), "42\n");
    }

    #[test]
    fn advisory_lock_prevents_takeover_during_owner_liveness_race() {
        let dir = tempfile::tempdir().unwrap();
        let first = acquire_update_lock(dir.path(), &FixedLiveness(false)).unwrap();
        assert!(matches!(first, LockAttempt::Acquired(_)));
        assert!(matches!(
            acquire_update_lock(dir.path(), &FixedLiveness(false)).unwrap(),
            LockAttempt::Busy { .. }
        ));
    }

    #[test]
    fn lock_symlink_is_rejected_without_touching_target() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("foreign");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("pid"), "99999999\n").unwrap();
        symlink(&target, dir.path().join(LOCK_NAME)).unwrap();
        assert!(acquire_update_lock(dir.path(), &FixedLiveness(false)).is_err());
        assert_eq!(
            fs::read_to_string(target.join("pid")).unwrap(),
            "99999999\n"
        );
    }

    #[test]
    fn unchanged_update_is_noop_without_backup() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("herdr");
        fs::write(&binary, b"old binary").unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o750)).unwrap();
        let runner = ScriptedUpdateRunner {
            binary: binary.clone(),
            replacement: None,
            update_status: 0,
            after_version: None,
            before_version: "herdr 1.0\n".into(),
            version_calls: RefCell::new(0),
        };
        let outcome = run_update(
            &UpdateRequest::new(binary, dir.path().join("state")),
            &runner,
            &FixedLiveness(false),
        )
        .unwrap();
        assert!(matches!(outcome, UpdateOutcome::Current { .. }));
        assert_eq!(
            fs::read_dir(dir.path().join("state/herdr-backups"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn failed_or_invalid_replacement_restores_bytes_and_mode() {
        for (status, after_version) in [(7, Some("herdr bad\n".into())), (0, None)] {
            let dir = tempfile::tempdir().unwrap();
            let binary = dir.path().join("herdr");
            fs::write(&binary, b"old binary").unwrap();
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o750)).unwrap();
            let runner = ScriptedUpdateRunner {
                binary: binary.clone(),
                replacement: Some(b"replacement".to_vec()),
                update_status: status,
                after_version,
                before_version: "herdr 1.0\n".into(),
                version_calls: RefCell::new(0),
            };
            assert!(run_update(
                &UpdateRequest::new(binary.clone(), dir.path().join("state")),
                &runner,
                &FixedLiveness(false)
            )
            .is_err());
            assert_eq!(fs::read(&binary).unwrap(), b"old binary");
            assert_eq!(fs::metadata(binary).unwrap().mode() & 0o777, 0o750);
        }
    }

    #[test]
    fn successful_update_records_digest_and_keeps_three_backups() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("herdr");
        let state = dir.path().join("state");
        fs::create_dir_all(state.join("herdr-backups")).unwrap();
        for name in ["herdr-old-a", "herdr-old-b", "herdr-old-c"] {
            fs::write(state.join("herdr-backups").join(name), name).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        fs::write(&binary, b"old binary").unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        let runner = ScriptedUpdateRunner {
            binary: binary.clone(),
            replacement: Some(b"new binary".to_vec()),
            update_status: 0,
            after_version: Some("herdr 2.0\n".into()),
            before_version: "herdr 1.0\n".into(),
            version_calls: RefCell::new(0),
        };
        let outcome = run_update(
            &UpdateRequest::new(binary, state.clone()),
            &runner,
            &FixedLiveness(false),
        )
        .unwrap();
        let UpdateOutcome::Updated { digest, backup, .. } = outcome else {
            panic!("expected update")
        };
        assert_eq!(digest.len(), 64);
        assert!(backup.exists());
        assert_eq!(
            fs::read_dir(state.join("herdr-backups")).unwrap().count(),
            3
        );
        let record = fs::read_to_string(state.join("herdr-update.json")).unwrap();
        assert!(record.contains("herdr 2.0"));
        assert!(record.contains(&digest));
    }

    #[test]
    fn templates_render_real_paths_and_distinct_schedules() {
        let dir = tempfile::tempdir().unwrap();
        let paths = service_paths(dir.path());
        let launchd = render_launchd(&paths, "gui/501");
        assert!(launchd.contains("StartInterval"));
        assert!(launchd.contains("21600"));
        assert!(launchd.contains("update-service"));
        assert!(launchd.contains(&xml_escape(&paths.runtime_binary().display().to_string())));
        assert!(launchd.contains("XDG_STATE_HOME"));
        assert!(launchd.contains(&xml_escape(
            &paths.state_root.parent().unwrap().display().to_string()
        )));
        assert!(launchd.contains("XDG_CONFIG_HOME"));
        assert!(launchd.contains(&xml_escape(
            &paths
                .config_path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .display()
                .to_string()
        )));
        let (service, timer) = render_systemd(&paths);
        assert!(service.contains("update-service run"));
        assert!(service.contains("XDG_STATE_HOME="));
        assert!(service.contains(&paths.state_root.parent().unwrap().display().to_string()));
        assert!(service.contains("XDG_CONFIG_HOME="));
        assert!(service.contains(
            &paths
                .config_path
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .display()
                .to_string()
        ));
        assert!(timer.contains("OnBootSec=2min"));
        assert!(timer.contains("RandomizedDelaySec=5min"));
    }

    #[test]
    fn systemd_escaping_preserves_literal_special_characters() {
        let dir = tempfile::tempdir().unwrap();
        let mut paths = service_paths(dir.path());
        paths.home = PathBuf::from("/home/percent%/dollar$/tick`");
        paths.state_root = paths.home.join("state/cmux-herdr");
        paths.config_path = paths.home.join("config/herdr/config.toml");
        paths.data_root = paths.home.join("data/cmux-herdr");
        let (service, _) = render_systemd(&paths);
        assert!(service.contains("percent%%"));
        assert!(service.contains("dollar$$"));
        assert!(service.contains("tick`"));
        assert!(!service.contains("\\`"));
    }

    fn prepare_install(paths: &ServicePaths) {
        fs::create_dir_all(paths.config_path.parent().unwrap()).unwrap();
        fs::create_dir_all(paths.source_binary.parent().unwrap()).unwrap();
        fs::create_dir_all(paths.herdr_binary.parent().unwrap()).unwrap();
        fs::write(
            &paths.config_path,
            "[update]\nversion_check = true\n\n[ui]\nkeep = true\n",
        )
        .unwrap();
        fs::write(&paths.source_binary, b"cmux binary").unwrap();
        fs::set_permissions(&paths.source_binary, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(&paths.herdr_binary, b"herdr binary").unwrap();
    }

    #[test]
    fn launchd_install_and_uninstall_are_reversible() {
        let dir = tempfile::tempdir().unwrap();
        let paths = service_paths(dir.path());
        prepare_install(&paths);
        let original = fs::read_to_string(&paths.config_path).unwrap();
        let runner = FakeRunner::supporting();
        let result = install_service(
            &InstallRequest::new(
                ServiceManager::Launchd {
                    domain: "gui/501".into(),
                },
                paths.clone(),
                "preview".into(),
                MANIFEST.into(),
            ),
            &runner,
        )
        .unwrap();
        assert_eq!(result.config, FileChange::Changed);
        assert!(paths.runtime_binary().exists());
        assert!(paths.launchd_plist().exists());
        assert!(runner.call_names().contains(&"plutil".into()));
        assert!(runner.call_names().contains(&"launchctl".into()));
        let removed = uninstall_service(
            &ServiceManager::Launchd {
                domain: "gui/501".into(),
            },
            &paths,
            &runner,
        )
        .unwrap();
        assert!(removed.removed.contains(&paths.runtime_binary()));
        assert_eq!(fs::read_to_string(&paths.config_path).unwrap(), original);
        assert!(!paths.launchd_plist().exists());
        assert!(!paths.runtime_binary().exists());
    }

    #[test]
    fn launchd_enable_failure_rolls_back_install() {
        let dir = tempfile::tempdir().unwrap();
        let paths = service_paths(dir.path());
        prepare_install(&paths);
        let original = fs::read_to_string(&paths.config_path).unwrap();
        let mut runner = FakeRunner::supporting();
        runner
            .argument_failures
            .insert(vec!["enable".into(), format!("gui/501/{LABEL}")], 1);
        assert!(install_service(
            &InstallRequest::new(
                ServiceManager::Launchd {
                    domain: "gui/501".into()
                },
                paths.clone(),
                "preview".into(),
                MANIFEST.into()
            ),
            &runner
        )
        .is_err());
        assert_eq!(fs::read_to_string(&paths.config_path).unwrap(), original);
        assert!(!paths.launchd_plist().exists());
        assert!(!paths.runtime_binary().exists());
    }

    #[test]
    fn systemd_install_validates_then_uninstall_cleans_units() {
        let dir = tempfile::tempdir().unwrap();
        let paths = service_paths(dir.path());
        prepare_install(&paths);
        let runner = FakeRunner::supporting();
        install_service(
            &InstallRequest::new(
                ServiceManager::Systemd,
                paths.clone(),
                "preview".into(),
                MANIFEST.into(),
            ),
            &runner,
        )
        .unwrap();
        let names = runner.call_names();
        assert!(names.contains(&"systemd-analyze".into()));
        assert!(names.contains(&"systemctl".into()));
        assert!(paths.systemd_service().exists());
        assert!(paths.systemd_timer().exists());
        uninstall_service(&ServiceManager::Systemd, &paths, &runner).unwrap();
        assert!(!paths.systemd_service().exists());
        assert!(!paths.systemd_timer().exists());
        assert!(!paths.runtime_binary().exists());
    }

    #[test]
    fn failed_activation_rolls_back_config_runtime_and_units() {
        let dir = tempfile::tempdir().unwrap();
        let paths = service_paths(dir.path());
        prepare_install(&paths);
        let original = fs::read_to_string(&paths.config_path).unwrap();
        let mut runner = FakeRunner::supporting();
        runner.argument_failures.insert(
            vec![
                "--user".into(),
                "enable".into(),
                "--now".into(),
                format!("{LABEL}.timer"),
            ],
            5,
        );
        assert!(install_service(
            &InstallRequest::new(
                ServiceManager::Systemd,
                paths.clone(),
                "preview".into(),
                MANIFEST.into()
            ),
            &runner
        )
        .is_err());
        let reloads = runner
            .calls
            .borrow()
            .iter()
            .filter(|(_, args)| args == &["--user", "daemon-reload"])
            .count();
        assert_eq!(reloads, 2, "the restored unit set must be reloaded");
        assert_eq!(fs::read_to_string(&paths.config_path).unwrap(), original);
        assert!(!paths.runtime_binary().exists());
        assert!(!paths.systemd_service().exists());
        assert!(!paths.systemd_timer().exists());
    }
    #[test]
    fn failed_definition_validation_rolls_back_before_activation() {
        let dir = tempfile::tempdir().unwrap();
        let paths = service_paths(dir.path());
        prepare_install(&paths);
        let original = fs::read_to_string(&paths.config_path).unwrap();
        let mut runner = FakeRunner::supporting();
        runner.failures.insert("systemd-analyze".into(), 1);
        assert!(install_service(
            &InstallRequest::new(
                ServiceManager::Systemd,
                paths.clone(),
                "preview".into(),
                MANIFEST.into()
            ),
            &runner
        )
        .is_err());
        assert_eq!(fs::read_to_string(&paths.config_path).unwrap(), original);
        assert!(!paths.runtime_binary().exists());
        assert!(!paths.systemd_service().exists());
        assert!(!paths.systemd_timer().exists());
    }
}
