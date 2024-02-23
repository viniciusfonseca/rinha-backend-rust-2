use tokio::{fs::File, io::{AsyncReadExt, AsyncSeekExt}};

pub struct AtomicFd {
    value: File,
    logs: File,
    log_size: usize
}

impl AtomicFd {
    pub async fn new(id: usize, log_size: usize) -> AtomicFd {
        AtomicFd {
            value: File::open(format!("/tmp/{id}.a")).await.unwrap(),
            logs: File::open(format!("/tmp/{id}.log")).await.unwrap(),
            log_size
        }
    }

    pub async fn get_value(&mut self) -> i32 {
        let r = self.value.read_i32().await.unwrap();
        self.value.seek(std::io::SeekFrom::Start(0));
        r
    }

    // pub async fn get_logs(&mut self) -> Vec<(i32, i32)> {
    //     self.value.seek(std::io::SeekFrom::Start(0));
    // }
}