//! Optional local Tesseract adapter. It runs an installed executable; installing goes only through `install_with_winget`.
use std::io::Cursor;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::{OcrDocument, MAX_IMAGE_PIXELS, MAX_LONG_EDGE, OCR_TIMEOUT};

const MAX_INPUT_BYTES: u64 = 100 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

/// Explicit paths and languages for a locally installed Tesseract instance.
#[derive(Debug, Clone)]
pub struct Tesseract {
    executable: PathBuf,
    tessdata: PathBuf,
    language: String,
}

impl Tesseract {
    /// True when a usable install exists: the executable plus its language data, found the
    /// same way the recognizer finds it (TESSERACT_PATH, the default install folder, then PATH).
    pub fn is_installed() -> bool {
        Self::default_installed().is_ok()
    }

    /// Returns the path to the Tesseract executable when installed, None otherwise.
    pub fn find_executable() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("TESSERACT_PATH") {
            let p = PathBuf::from(&path);
            if p.is_file() {
                return Some(p);
            }
        }
        // The default install folders come from the environment, not a fixed drive.
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = std::env::var_os(var) {
                let p = Path::new(&root).join("Tesseract-OCR").join("tesseract.exe");
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        if let Ok(output) = std::process::Command::new("where")
            .arg("tesseract")
            .creation_flags(0x08000000)
            .output()
        {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = path_str.lines().next() {
                    let p = PathBuf::from(first_line.trim());
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    /// Installs Tesseract silently with winget (no console window) and returns the executable
    /// found afterwards. winget can exit non-zero when the package is already present, so the
    /// re-detect decides the result and winget's output only feeds the error message.
    pub fn install_with_winget() -> anyhow::Result<PathBuf> {
        let output = Command::new("winget")
            .args([
                "install",
                "--id",
                "UB-Mannheim.TesseractOCR",
                "-e",
                "--silent",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .output()
            .map_err(|error| {
                anyhow!("winget could not start ({error}). Install App Installer from the Microsoft Store.")
            })?;
        if let Some(path) = Self::find_executable() {
            return Ok(path);
        }
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let last = text.lines().map(str::trim).rfind(|line| !line.is_empty()).unwrap_or("no output");
        let code = output
            .status
            .code()
            .map_or_else(|| "no exit code".to_string(), |code| format!("exit code {code}"));
        bail!("Tesseract is still not installed (winget {code}). Last message: {last}")
    }

    /// Automatically detects and initializes a local Tesseract installation using standard locations and PATH.
    pub fn default_installed() -> anyhow::Result<Self> {
        let exe = Self::find_executable().ok_or_else(|| anyhow!("Tesseract executable not found"))?;
        let tessdata = if let Ok(prefix) = std::env::var("TESSDATA_PREFIX") {
            let p = PathBuf::from(prefix);
            if p.is_dir() {
                p
            } else {
                exe.parent().unwrap_or(Path::new("")).join("tessdata")
            }
        } else if let Some(parent) = exe.parent() {
            parent.join("tessdata")
        } else {
            PathBuf::from("tessdata")
        };
        let lang = if tessdata.join("eng.traineddata").is_file() {
            "eng".to_string()
        } else if let Ok(entries) = std::fs::read_dir(&tessdata) {
            let mut found = None;
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(prefix) = name.strip_suffix(".traineddata") {
                    if !prefix.starts_with("osd") {
                        found = Some(prefix.to_string());
                        break;
                    }
                }
            }
            found.unwrap_or_else(|| "eng".to_string())
        } else {
            "eng".to_string()
        };
        Self::new(exe, tessdata, lang)
    }

    /// Create an adapter only when the configured executable and all requested
    /// language data files already exist locally.
    pub fn new(
        executable: impl Into<PathBuf>,
        tessdata: impl Into<PathBuf>,
        language: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let executable = executable.into();
        let tessdata = tessdata.into();
        let language = language.into();
        if !executable.is_file() {
            bail!(
                "configured Tesseract executable is not a file: {}",
                executable.display()
            );
        }
        if !tessdata.is_dir() {
            bail!(
                "configured Tesseract tessdata directory is missing: {}",
                tessdata.display()
            );
        }
        if language.is_empty()
            || language.split('+').any(|part| {
                part.is_empty()
                    || !part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
            })
        {
            bail!("invalid Tesseract language list");
        }
        for part in language.split('+') {
            let data = tessdata.join(format!("{part}.traineddata"));
            if !data.is_file() {
                bail!("Tesseract language data is missing: {}", data.display());
            }
        }
        Ok(Self {
            executable,
            tessdata,
            language,
        })
    }

    /// Recognize a source image directly by path. The executable is invoked
    /// without a shell; input pixels, process time, and captured output are capped.
    pub fn recognize(&self, path: &Path, max_pixels: u64) -> anyhow::Result<Option<OcrDocument>> {
        let file = std::fs::File::open(path)
            .map_err(|error| anyhow!("opening image {}: {error}", path.display()))?;
        let metadata = file
            .metadata()
            .map_err(|error| anyhow!("reading image metadata {}: {error}", path.display()))?;
        if !metadata.is_file() {
            bail!("image is not a file: {}", path.display());
        }
        if metadata.len() > MAX_INPUT_BYTES {
            bail!("image exceeds Tesseract input limit ({} bytes)", MAX_INPUT_BYTES);
        }
        // OCR and dimension checks use the same immutable bytes, so a changed
        // source path cannot replace the validated image before recognition.
        let mut input = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_INPUT_BYTES + 1).read_to_end(&mut input)?;
        if input.len() as u64 > MAX_INPUT_BYTES {
            bail!("image exceeds Tesseract input limit ({} bytes)", MAX_INPUT_BYTES);
        }
        let (width, height) = image_dimensions(&input, path)?;
        let pixel_cap = max_pixels.clamp(1, MAX_IMAGE_PIXELS);
        if width == 0
            || height == 0
            || width.max(height) > MAX_LONG_EDGE
            || width as u64 * height as u64 > pixel_cap
        {
            bail!(
                "image dimensions exceed Tesseract limit: {} ({}x{})",
                path.display(),
                width,
                height
            );
        }

        let mut child = Command::new(&self.executable)
            .arg("stdin")
            .arg("stdout")
            .arg("-l")
            .arg(&self.language)
            .arg("--tessdata-dir")
            .arg(&self.tessdata)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| anyhow!("starting configured Tesseract executable: {error}"))?;
        #[cfg(windows)]
        let job = match WindowsJob::assign(&child) {
            Ok(job) => job,
            Err(error) => {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let stdin = child.stdin.take().expect("piped stdin");
        let input_writer = thread::spawn(move || -> std::io::Result<()> {
            use std::io::Write;
            let mut stdin = stdin;
            stdin.write_all(&input)
        });
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let stdout_reader = thread::spawn(move || read_bounded(stdout, MAX_OUTPUT_BYTES));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, 16 * 1024));
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    #[cfg(windows)]
                    drop(job);
                    let _ = input_writer.join();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(anyhow!("waiting for Tesseract: {error}"));
                }
            }
            if started.elapsed() >= OCR_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                #[cfg(windows)]
                drop(job);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                let _ = input_writer.join();
                bail!("Tesseract timed out after {:?}", OCR_TIMEOUT);
            }
            thread::sleep(Duration::from_millis(10));
        };
        #[cfg(windows)]
        drop(job);
        input_writer
            .join()
            .map_err(|_| anyhow!("writing Tesseract input failed"))?
            .map_err(|error| anyhow!("sending image to Tesseract: {error}"))?;
        let (output, too_much_output) = stdout_reader
            .join()
            .map_err(|_| anyhow!("reading Tesseract output failed"))??;
        let (error_text, _) = stderr_reader
            .join()
            .map_err(|_| anyhow!("reading Tesseract diagnostics failed"))??;
        if too_much_output {
            bail!("Tesseract output exceeded {} bytes", MAX_OUTPUT_BYTES);
        }
        if !status.success() {
            bail!(
                "Tesseract failed ({status}): {}",
                String::from_utf8_lossy(&error_text).trim()
            );
        }
        let text = String::from_utf8_lossy(&output).trim().to_owned();
        if text.is_empty() {
            return Ok(None);
        }
        Ok(Some(OcrDocument {
            text,
            language: self.language.clone(),
            confidence: 1.0,
            width,
            height,
        }))
    }
}

#[cfg(windows)]
struct WindowsJob(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl WindowsJob {
    fn assign(child: &std::process::Child) -> anyhow::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        use windows::Win32::System::Threading::IO_COUNTERS;
        let job = unsafe { CreateJobObjectW(None, windows::core::PCWSTR::null()) }
            .map_err(|error| anyhow!("creating Tesseract process job: {error}"))?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: windows::Win32::System::JobObjects::JOBOBJECT_BASIC_LIMIT_INFORMATION {
                LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                ..Default::default()
            },
            IoInfo: IO_COUNTERS::default(),
            ..Default::default()
        };
        if let Err(error) = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } {
            unsafe {
                windows::Win32::Foundation::CloseHandle(job).ok();
            }
            return Err(anyhow!("configuring Tesseract process job: {error}"));
        }
        let process = windows::Win32::Foundation::HANDLE(child.as_raw_handle() as *mut _);
        if let Err(error) = unsafe { AssignProcessToJobObject(job, process) } {
            unsafe {
                windows::Win32::Foundation::CloseHandle(job).ok();
            }
            return Err(anyhow!("assigning Tesseract process job: {error}"));
        }
        Ok(Self(job))
    }
}

#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::Foundation::CloseHandle(self.0).ok();
        }
    }
}

fn image_dimensions(input: &[u8], path: &Path) -> anyhow::Result<(u32, u32)> {
    use image::io::Reader as ImageReader;
    let reader = ImageReader::new(Cursor::new(input))
        .with_guessed_format()
        .map_err(|error| anyhow!("opening image {}: {error}", path.display()))?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| anyhow!("reading image dimensions {}: {error}", path.display()))?;
    Ok((width, height))
}

fn read_bounded(mut input: impl Read, limit: usize) -> anyhow::Result<(Vec<u8>, bool)> {
    let mut output = Vec::with_capacity(limit.min(16 * 1024));
    let mut buffer = [0u8; 8192];
    let mut exceeded = false;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let keep = count.min(limit.saturating_sub(output.len()));
        output.extend_from_slice(&buffer[..keep]);
        exceeded |= keep < count;
    }
    Ok((output, exceeded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_names_cannot_escape_tessdata_directory() {
        let exe = std::env::current_exe().unwrap();
        let tessdata = std::env::temp_dir();
        assert!(Tesseract::new(&exe, &tessdata, "../secret").is_err());
        assert!(Tesseract::new(&exe, &tessdata, "eng").is_err());
    }

    #[test]
    fn bounded_reader_drains_beyond_limit_without_retaining_bytes() {
        let (bytes, exceeded) = read_bounded(&b"0123456789"[..], 4).unwrap();
        assert_eq!(bytes, b"0123");
        assert!(exceeded);
    }
}
