use std::process::{Command, Stdio, Output};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::path::{Path, PathBuf};
use std::time::Duration;

use argh::FromArgs;

struct Video {
    path: PathBuf,
    duration: f64,
}

fn get_video_duration_ffprobe(file_path: &Path) -> Result<f64, &'static str> {
    let output: Output = Command::new("ffprobe")
        .args(&["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(file_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| "Failed to execute command")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse().map_err(|_| "Could not parse output")
}

fn process_videos_in_directory(num_threads: usize, directory: &Path) -> Duration {
    let (tx, rx): (Sender<Video>, Receiver<Video>) = channel();

    let mut total_duration: Duration = Duration::new(0, 0);

    directory.read_dir().expect("Failed to read directory").for_each(|entry: Result<std::fs::DirEntry, std::io::Error>| {
        let entry: std::fs::DirEntry = entry.expect("Failed to get directory entry");
        let path: PathBuf = entry.path();
        if path.is_dir() {
            let tx_clone: Sender<Video> = tx.clone();
            let dir_clone: PathBuf = path.clone();
            thread::spawn(move || {
                let duration: Duration = process_videos_in_directory(num_threads, &dir_clone);
                tx_clone.send(Video { path, duration: duration.as_secs_f64() }).unwrap();
            });
        } else if path.extension().and_then(|ext: &std::ffi::OsStr| ext.to_str()) == Some("mp4")
                || path.extension().and_then(|ext: &std::ffi::OsStr| ext.to_str()) == Some("avi")
                || path.extension().and_then(|ext: &std::ffi::OsStr| ext.to_str()) == Some("mov")
                || path.extension().and_then(|ext: &std::ffi::OsStr| ext.to_str()) == Some("mkv") {
            let tx_clone: Sender<Video> = tx.clone();
            thread::spawn(move || {
                let duration: f64 = get_video_duration_ffprobe(&path).unwrap_or(0.0);
                tx_clone.send(Video { path, duration }).unwrap();
            });
        }
    });

    drop(tx);

    let mut videos: Vec<Video> = rx.iter().collect::<Vec<_>>();
    videos.sort_by(|a: &Video, b| a.path.cmp(&b.path));

    for video in videos {
        total_duration += Duration::from_secs_f64(video.duration);
    }

    total_duration
}


#[derive(FromArgs)]
/// Reach new heights.
struct Cli {
    /// the number of threads to use
    #[argh(option,short = 'n')]
    num_threads: Option<usize>,

    /// an optional nickname for the pilot
    #[argh(option, short = 'd')]
    directory: Option<String>,
}

fn main() {
    let args: Cli = argh::from_env();
    let binding: String = args.directory.unwrap_or(".".to_string());
    let directory: &Path = Path::new(binding.as_str());
    let num_threads: usize = args.num_threads.unwrap_or(4);

    let total_duration: Duration = process_videos_in_directory(num_threads, &directory);

    let total_seconds: u64 = total_duration.as_secs();
    let hours: u64 = total_seconds / 3600;
    let minutes: u64 = (total_seconds % 3600) / 60;
    let seconds: u64 = total_seconds % 60;

    println!("Total duration: {} hours, {} minutes, {} seconds", hours, minutes, seconds);
}
