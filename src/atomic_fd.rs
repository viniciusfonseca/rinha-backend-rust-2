use tokio::{fs::{File, OpenOptions}, io::{AsyncReadExt, AsyncSeekExt}};

// {tx_id},{updated_value},{datetime_rfc3339},{tipo},{descricao}
pub type AtomicLog = (i32, i32, String, String, String);
pub struct AtomicFd {
    value: File,
    logs: File,
    log_size: usize
}

impl AtomicFd {
    pub async fn new(id: usize, log_size: usize) -> AtomicFd {
        AtomicFd {
            value: OpenOptions::new()
                .create(true)
                .read(true)
                .write(false)
                .open(format!("/tmp/{id}.a")).await.unwrap(),
            logs: OpenOptions::new()
                .create(true)
                .read(true)
                .write(false)
                .open(format!("/tmp/{id}.a")).await.unwrap(),
            log_size
        }
    }

    pub async fn get_value(&mut self) -> i32 {
        let r = self.value.read_i32().await.unwrap();
        _ = self.value.seek(std::io::SeekFrom::Start(0)).await;
        r
    }

    pub async fn get_logs(&mut self, max: usize) -> Vec<AtomicLog> {
        let buffer_size = self.log_size * max;
        _ = self.logs.seek(std::io::SeekFrom::End(0)).await;
        if self.logs.seek(std::io::SeekFrom::Current(-(TryInto::<i64>::try_into(buffer_size + 1)).unwrap())).await.is_err() {
            _ = self.logs.seek(std::io::SeekFrom::Start(0)).await;
        }
        let mut buf = vec![0 as u8; buffer_size];
        self.logs.read_buf(&mut buf).await.unwrap();
        let lines = String::from_utf8(buf).unwrap();
        let lines = lines.split("\n");
        let mut r = Vec::new();
        for line in lines {
            let split = line.split(",").collect::<Vec<&str>>();
            r.push((
                split.get(0).unwrap().parse::<i32>().unwrap(),
                split.get(1).unwrap().parse::<i32>().unwrap(),
                split.get(2).unwrap().to_string(),
                split.get(3).unwrap().to_string(),
                split.get(4).unwrap().to_string(),
            ))
        }
        r
    }
}