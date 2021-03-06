use console::Term;
use crate::errors::*;
use humansize::{FileSize, file_size_opts};
use serde::{Serialize, Deserialize};
use std::cmp;
use std::collections::BTreeMap;
use std::iter;
use std::io::prelude::*;
use std::time::{Instant, Duration};

const MAX_FILENAME_LEN: usize = 20;
const MINIMUM_WIDTH: u16 = 48;
const PROGRESS_BAR_OVERHEAD: u64 = 21 + MAX_FILENAME_LEN as u64;
const UPDATE_NOTIFY_RATELIMIT: Duration = Duration::from_millis(200);

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Status {
    pub idle_workers: usize,
    pub total_workers: usize,
    pub queue: usize,
    pub queue_size: u64,
    pub progress: BTreeMap<String, Progress>,
}

impl Status {
    pub fn update(&mut self, update: ProgressUpdate) {
        match update {
            ProgressUpdate::UploadStart(start) => {
                self.progress.insert(start.id, Progress {
                    label: start.label,
                    bytes_read: 0,
                    total: start.total,
                    speed: 0,
                });
            },
            ProgressUpdate::UploadProgress(progress) => {
                let p = self.progress.get_mut(&progress.id)
                    .unwrap_or_else(|| panic!("progress bar not found: {:?}", progress.id));
                p.bytes_read = progress.bytes_read;
                p.speed = progress.speed;
            },
            ProgressUpdate::UploadEnd(end) => {
                self.progress.remove(&end.id);
            },
        }
    }

    #[inline]
    fn is_idle(&self) -> bool {
        self.idle_workers == self.total_workers && self.queue == 0
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Progress {
    label: String,
    bytes_read: u64,
    total: u64,
    speed: u64,
}

#[derive(Debug)]
pub enum ProgressUpdate {
    UploadStart(UploadStart),
    UploadProgress(UploadProgress),
    UploadEnd(UploadEnd),
}

#[derive(Debug)]
pub struct UploadStart {
    pub id: String,
    pub label: String,
    pub total: u64,
}

#[derive(Debug)]
pub struct UploadProgress {
    pub id: String,
    pub bytes_read: u64,
    pub speed: u64,
}

#[derive(Debug)]
pub struct UploadEnd {
    pub id: String,
}

pub struct StatusWriter {
    term: Term,
    height: usize,
    last_update: Instant,
}

impl Default for StatusWriter {
    fn default() -> Self {
            Self::new()
    }
}

impl StatusWriter {
    pub fn new() -> StatusWriter {
        StatusWriter{
            term: Term::stderr(),
            height: 0,
            last_update: Instant::now() - UPDATE_NOTIFY_RATELIMIT,
        }
    }

    pub fn write_progress(&mut self, p: &Progress, width: u64) -> Result<()> {
        // TODO: show self.bytes_read/self.total

        let (progress, indicators) = if p.total > 0 {
            let progress = p.bytes_read * 100 / p.total;
            let indicators = p.bytes_read * width / p.total;
            (progress, indicators)
        } else {
            (0, 0)
        };
        let spaces = width - indicators;

        let speed = p.speed.file_size(file_size_opts::CONVENTIONAL)
            .map_err(|e| format_err!("{}", e))?;

        let indicators = iter::repeat('=').take(indicators as usize).collect::<String>();
        let spaces = if spaces > 0 {
            iter::once('>').chain(
                iter::repeat(' ').take(spaces as usize - 1)
            ).collect()
        } else {
            String::new()
        };

        let mut filename = p.label.as_str();
        if filename.len() > MAX_FILENAME_LEN {
            filename = &filename[..MAX_FILENAME_LEN];
        }

        writeln!(self.term, "{:filename_width$} [{}{}] {:>10}/s {:>3}%",
            filename,
            indicators,
            spaces,
            speed,
            progress,
            filename_width=MAX_FILENAME_LEN,
        )?;

        Ok(())
    }

    pub fn write(&mut self, status: &Status) -> Result<()> {
        // never skip updates that we became idle
        if !status.is_idle() {
            // check ratelimit
            let now = Instant::now();
            if now.duration_since(self.last_update) < UPDATE_NOTIFY_RATELIMIT {
                return Ok(())
            }
            self.last_update = now;
        }

        // update terminal
        let (_, width) = self.term.size();
        let progressbar_width = cmp::max(width, MINIMUM_WIDTH) as u64 - PROGRESS_BAR_OVERHEAD;

        // clear the lines we've written the last time
        self.term.clear_line()?;
        if  self.height > 0 {
            self.term.clear_last_lines(self.height)?;
        }
        self.height = 0;

        // print progress bars and print how many we wrote
        for p in status.progress.values() {
            self.write_progress(&p, progressbar_width)?;
            self.height += 1;
        }

        let queue_size = status.queue_size.file_size(file_size_opts::CONVENTIONAL)
            .map_err(|e| format_err!("{}", e))?;

        // print stats
        write!(self.term, " :: workers={}/{}, queue={} ({})\r",
            status.total_workers - status.idle_workers, // TODO: consider moving to busy_workers
            status.total_workers,
            status.queue,
            queue_size,
        )?;

        Ok(())
    }

    #[inline]
    pub fn finish(&self) -> Result<()> {
        self.term.clear_line()?;
        Ok(())
    }
}
